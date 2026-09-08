mod schema;
use super::{await_provider, HandlerContext};
use crate::{
    command::{self, AlertListArgs},
    config,
    digitransit::DigitransitRouter,
    error::AppError,
    output::{CommandOutput, Warning},
};
use chrono::{DateTime, FixedOffset, Utc};
use reitti_core::{Alert, AlertRequest, AlertSeverity, Language, Router};
use serde_json::{json, Value};
use std::{cmp::Ordering, collections::BTreeSet, time::Duration};

pub fn execute(
    context: HandlerContext<'_>,
    args: AlertListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let (active_at, time_source) = match &args.active_at {
        Some(value) => (
            DateTime::parse_from_rfc3339(value).map_err(|_| {
                AppError::invalid(
                    "invalid_datetime",
                    format!("Invalid --active-at value '{}'; expected RFC 3339 with an explicit offset.", command::escape_text(value)),
                    command::escape_text(value),
                    "RFC3339 with explicit offset",
                )
            })?,
            "argument",
        ),
        None => (now_in_timezone(&context)?, "clock"),
    };
    let routes = args.route.iter().cloned().collect::<BTreeSet<_>>();
    let stops = args.stop.iter().cloned().collect::<BTreeSet<_>>();
    let request_id = context.request_ids.next();
    let router = router(&context)?;
    let result = await_provider(router.alerts(AlertRequest { language }))?;

    let active_utc = active_at.with_timezone(&Utc);
    let mut alerts = result
        .value
        .into_iter()
        .filter(|alert| {
            let relevant = routes.is_empty() && stops.is_empty()
                || scope_unknown(alert, !routes.is_empty(), !stops.is_empty())
                || feed_wide(alert)
                || alert.entities.iter().any(|entity| {
                    entity
                        .route_id
                        .as_ref()
                        .is_some_and(|id| routes.contains(id))
                        || entity.stop_id.as_ref().is_some_and(|id| stops.contains(id))
                });
            relevant
                && alert.valid_from.is_none_or(|start| start <= active_utc)
                && alert.valid_until.is_none_or(|end| active_utc <= end)
        })
        .collect::<Vec<_>>();
    alerts.sort_by(alert_order);
    let available_matches = alerts.len();
    alerts.truncate(args.limit as usize);
    let count = alerts.len();
    let unknown_scope_ids = alerts
        .iter()
        .filter(|alert| scope_unknown(alert, !routes.is_empty(), !stops.is_empty()))
        .map(|alert| alert.id.clone())
        .collect::<BTreeSet<_>>();
    let unknown_validity_ids = alerts
        .iter()
        .filter(|alert| alert.valid_from.is_none() || alert.valid_until.is_none())
        .map(|alert| alert.id.clone())
        .collect::<BTreeSet<_>>();

    let mut warnings = Vec::new();
    if !unknown_scope_ids.is_empty() {
        warnings.push(Warning {
            code: "alert_scope_unknown".into(),
            message: "Alerts with absent or unknown entity scope were retained conservatively."
                .into(),
            details: json!({"alert_ids":unknown_scope_ids}),
        });
    }
    if !unknown_validity_ids.is_empty() {
        warnings.push(Warning {
            code: "alert_validity_unknown".into(),
            message: "Alerts with an unknown validity boundary were retained; activity could not be proven.".into(),
            details: json!({"alert_ids":unknown_validity_ids}),
        });
    }
    if available_matches > args.limit as usize {
        warnings.push(Warning {
            code: "results_truncated".into(),
            message: format!("{} relevant active alerts were available; only {} are included.", available_matches, args.limit),
            details: json!({"available_matches":available_matches,"returned":count,"complete":false}),
        });
    }
    if alerts.is_empty() {
        warnings.push(Warning {
            code: "normal_service_not_guaranteed".into(),
            message: "No matching alerts were returned; this does not guarantee normal service."
                .into(),
            details: json!({}),
        });
    }

    let text = render_alerts(&alerts, active_at, &result.source.attribution);
    CommandOutput::success(
        text,
        json!({
            "filters": {
                "routes": args.route,
                "stops": args.stop,
                "active_at": active_at,
                "time_source": time_source,
                "limit": args.limit,
            },
            "count": count,
            "complete": false,
            "alerts": alerts,
            "request": request_metadata(request_id, language, &context),
            "source": result.source,
        }),
    )?
    .with_warnings(warnings)
}

fn router<'a>(context: &'a HandlerContext<'a>) -> Result<DigitransitRouter<'a>, AppError> {
    let key = context
        .config
        .subscription_key
        .as_ref()
        .ok_or_else(AppError::credential_missing)?
        .0
        .expose()
        .to_owned();
    let connect =
        config::validate_duration("connect_timeout", &context.config.connect_timeout.0, None)?;
    let request =
        config::validate_duration("request_timeout", &context.config.request_timeout.0, None)?;
    await_provider(async {
        DigitransitRouter::new(
            &context.config.routing_url.0,
            key,
            context.transport,
            context.clock,
            Duration::from_millis(connect),
            Duration::from_millis(request),
        )
    })
}

fn now_in_timezone(context: &HandlerContext<'_>) -> Result<DateTime<FixedOffset>, AppError> {
    let timezone: chrono_tz::Tz = context.config.timezone.0.parse().map_err(|_| {
        AppError::caller(
            "invalid_timezone",
            "The configured timezone is not a valid IANA timezone.",
        )
    })?;
    Ok(context.clock.now().with_timezone(&timezone).fixed_offset())
}

fn scope_unknown(alert: &Alert, has_route_filters: bool, has_stop_filters: bool) -> bool {
    alert.entities.is_empty()
        || alert.entities.iter().any(|entity| {
            entity.kind == "unknown"
                || entity.kind == "route_type"
                || (!matches!(
                    entity.kind.as_str(),
                    "agency"
                        | "pattern"
                        | "route"
                        | "route_type"
                        | "stop"
                        | "stop_on_route"
                        | "stop_on_trip"
                        | "trip"
                ))
                || (entity.id.is_none()
                    && entity.route_id.is_none()
                    && entity.stop_id.is_none()
                    && entity.trip_id.is_none()
                    && !matches!(entity.kind.as_str(), "agency" | "route_type"))
                // Routing v2 supplies no route relationship for these shapes.
                // Under a route filter they cannot be safely declared irrelevant.
                || (has_route_filters
                    && matches!(entity.kind.as_str(), "trip" | "stop_on_trip")
                    && entity.route_id.is_none())
                || (has_stop_filters && entity.kind == "trip" && entity.stop_id.is_none())
        })
}

fn feed_wide(alert: &Alert) -> bool {
    alert.entities.iter().any(|entity| entity.kind == "agency")
}

fn alert_order(left: &Alert, right: &Alert) -> Ordering {
    severity_rank(left.severity)
        .cmp(&severity_rank(right.severity))
        .then_with(|| match (left.valid_from, right.valid_from) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
        .then_with(|| left.id.cmp(&right.id))
}

fn severity_rank(severity: AlertSeverity) -> u8 {
    match severity {
        AlertSeverity::Severe => 0,
        AlertSeverity::Warning => 1,
        AlertSeverity::Info => 2,
        AlertSeverity::Unknown => 3,
    }
}

fn severity_label(severity: AlertSeverity) -> &'static str {
    match severity {
        AlertSeverity::Severe => "SEVERE",
        AlertSeverity::Warning => "WARNING",
        AlertSeverity::Info => "INFO",
        AlertSeverity::Unknown => "UNKNOWN",
    }
}

fn request_metadata(id: String, language: Language, context: &HandlerContext<'_>) -> Value {
    json!({"request_id":id,"language":language.code(),"timezone":context.config.timezone.0})
}

fn render_alerts(alerts: &[Alert], active_at: DateTime<FixedOffset>, attribution: &str) -> String {
    let mut lines = vec![format!(
        "{} alerts at {}",
        alerts.len(),
        active_at.format("%Y-%m-%d %H:%M %:z")
    )];
    for alert in alerts {
        let title = alert
            .header
            .as_deref()
            .filter(|header| !header.is_empty())
            .unwrap_or(&alert.description);
        let range = match (alert.valid_from, alert.valid_until) {
            (Some(start), Some(end)) => format!(
                " · {}–{}",
                start.format("%Y-%m-%d %H:%MZ"),
                end.format("%Y-%m-%d %H:%MZ")
            ),
            _ => " · validity unknown".into(),
        };
        lines.push(format!(
            "{}  {}{}",
            severity_label(alert.severity),
            command::escape_text(title),
            range
        ));
        if alert.header.is_some() && !alert.description.is_empty() {
            lines.push(format!("  {}", command::escape_text(&alert.description)));
        }
    }
    lines.push(command::escape_text(attribution));
    lines.join("\n")
}

pub(crate) use schema::schema;
