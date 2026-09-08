mod schema;

use super::{location, HandlerContext};
use crate::{command::JourneyListArgs, error::AppError, output::CommandOutput};
use reitti_core::{Language, LocationRef, ProviderSource};
use serde_json::json;

pub fn execute(
    context: HandlerContext<'_>,
    args: JourneyListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    // The dispatcher has already validated both values. Parse again here to
    // retain the typed references for the reusable resolution seam.
    let from_ref = args
        .from
        .parse::<LocationRef>()
        .expect("dispatcher validated --from");
    let to_ref = args
        .to
        .parse::<LocationRef>()
        .expect("dispatcher validated --to");

    // Resolve in command order. Each resolver branch performs zero or one
    // provider call, and the future planning worker receives exact coordinates
    // without repeating either lookup.
    let from = location::resolve(&context, &args.from, from_ref, language)?;
    let to = location::resolve(&context, &args.to, to_ref, language)?;
    let sources = sources_in_request_order([from.source.as_ref(), to.source.as_ref()]);

    Err(AppError::feature_incomplete("journey planning")
        .with_detail("stage", "locations_resolved")
        .with_detail("request_id", context.request_ids.next())
        .with_detail("from", json!(from.selected))
        .with_detail("to", json!(to.selected))
        .with_detail("sources", json!(sources))
        .with_detail("plan_requests_sent", 0_u8))
}

fn sources_in_request_order<'a>(
    sources: impl IntoIterator<Item = Option<&'a ProviderSource>>,
) -> Vec<&'a ProviderSource> {
    // Each source carries its own retrieval time, so preserve one entry per
    // actual lookup rather than collapsing separately retrieved evidence.
    sources.into_iter().flatten().collect()
}

pub(crate) use schema::schema;
