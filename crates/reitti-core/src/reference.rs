use std::{fmt, str::FromStr};

use thiserror::Error;

use crate::Coordinates;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ReferenceError {
    #[error("value must contain a non-whitespace character")]
    Blank,
    #[error("invalid tagged location reference '{0}'; expected query:, place:, stop:, or coord:")]
    InvalidLocationRef(String),
    #[error("invalid coordinate '{0}'; expected LAT,LON without whitespace")]
    InvalidCoordinates(String),
    #[error("invalid stop ID '{0}'; expected a raw ID such as HSL:1020453")]
    InvalidStopId(String),
    #[error("invalid route ID '{0}'; expected a raw ID such as HSL:31M1")]
    InvalidRouteId(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LocationRef {
    Query(String),
    Place(String),
    Stop(StopId),
    Coordinate(Coordinates),
}

impl FromStr for LocationRef {
    type Err = ReferenceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.trim().is_empty() {
            return Err(ReferenceError::Blank);
        }
        if value.chars().any(char::is_control) {
            return Err(ReferenceError::InvalidLocationRef(escaped(value)));
        }
        let (tag, body) = value
            .split_once(':')
            .ok_or_else(|| ReferenceError::InvalidLocationRef(value.to_owned()))?;
        if body.trim().is_empty() {
            return Err(ReferenceError::InvalidLocationRef(value.to_owned()));
        }
        match tag {
            "query" => Ok(Self::Query(body.to_owned())),
            "place" => Ok(Self::Place(body.to_owned())),
            "stop" => Ok(Self::Stop(body.parse()?)),
            "coord" => Ok(Self::Coordinate(parse_coordinates(body)?)),
            _ => Err(ReferenceError::InvalidLocationRef(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopId(String);

impl StopId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StopId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for StopId {
    type Err = ReferenceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if is_hsl_id(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(ReferenceError::InvalidStopId(value.to_owned()))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteId(String);

impl RouteId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for RouteId {
    type Err = ReferenceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if is_hsl_id(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(ReferenceError::InvalidRouteId(value.to_owned()))
        }
    }
}

fn is_plain_decimal(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let mut dot_seen = false;
    let mut digit_seen = false;
    for byte in value.bytes() {
        if byte == b'.' && !dot_seen {
            dot_seen = true;
        } else if byte.is_ascii_digit() {
            digit_seen = true;
        } else {
            return false;
        }
    }
    digit_seen
}

fn escaped(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                character.escape_default().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}

fn is_hsl_id(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("HSL:") else {
        return false;
    };
    !suffix.is_empty()
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn parse_coordinates(value: &str) -> Result<Coordinates, ReferenceError> {
    if value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err(ReferenceError::InvalidCoordinates(value.to_owned()));
    }
    let mut fields = value.split(',');
    let (Some(latitude), Some(longitude), None) = (fields.next(), fields.next(), fields.next())
    else {
        return Err(ReferenceError::InvalidCoordinates(value.to_owned()));
    };
    if !is_plain_decimal(latitude) || !is_plain_decimal(longitude) {
        return Err(ReferenceError::InvalidCoordinates(escaped(value)));
    }
    let latitude = latitude
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && (-90.0..=90.0).contains(number))
        .ok_or_else(|| ReferenceError::InvalidCoordinates(value.to_owned()))?;
    let longitude = longitude
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && (-180.0..=180.0).contains(number))
        .ok_or_else(|| ReferenceError::InvalidCoordinates(value.to_owned()))?;
    Ok(Coordinates {
        latitude,
        longitude,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journey_references_require_tags() {
        assert!(matches!(
            "HSL:1020453".parse::<LocationRef>(),
            Err(ReferenceError::InvalidLocationRef(_))
        ));
        assert!(matches!(
            "stop:HSL:1020453".parse::<LocationRef>(),
            Ok(LocationRef::Stop(_))
        ));
        assert_eq!(
            "coord:60.1699,24.9384".parse::<LocationRef>(),
            Ok(LocationRef::Coordinate(Coordinates {
                latitude: 60.1699,
                longitude: 24.9384
            }))
        );
    }

    #[test]
    fn stop_positions_take_only_raw_ids() {
        assert!("HSL:1020453".parse::<StopId>().is_ok());
        assert!("stop:HSL:1020453".parse::<StopId>().is_err());
        assert!("HSL:".parse::<StopId>().is_err());
    }

    #[test]
    fn coordinates_are_strict_and_bounded() {
        for invalid in ["60, 24", "91,24", "60", "NaN,24", "60,24\n", "6e1,24"] {
            assert!(format!("coord:{invalid}").parse::<LocationRef>().is_err());
        }
    }
}
