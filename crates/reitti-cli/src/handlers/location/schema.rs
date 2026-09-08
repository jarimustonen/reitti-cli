use serde_json::{json, Value};

pub fn schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["query", "kind", "limit", "count", "complete", "candidates", "request", "source"],
        "properties": {
            "query": {"type": "string", "minLength": 1},
            "kind": {"enum": ["any", "address", "venue", "stop"]},
            "limit": {"type": "integer", "minimum": 1, "maximum": 10},
            "count": {"type": "integer", "minimum": 0, "maximum": 10},
            "complete": {"const": false},
            "candidates": {"type": "array", "maxItems": 10, "items": {"$ref": "#/$defs/location"}},
            "request": {"$ref": "#/$defs/request"},
            "source": {"$ref": "#/$defs/source"}
        },
        "$defs": {
            "location": {
                "type": "object",
                "additionalProperties": false,
                "required": ["ref", "kind", "id", "label", "name", "locality", "neighbourhood", "postal_code", "coordinates", "source", "source_layer", "confidence", "service_area", "modes"],
                "properties": {
                    "ref": {"type": "string", "pattern": "^place:.+"},
                    "kind": {"enum": ["address", "venue", "stop", "locality", "other"]},
                    "id": {"type": ["string", "null"]},
                    "label": {"type": "string"},
                    "name": {"type": "string"},
                    "locality": {"type": ["string", "null"]},
                    "neighbourhood": {"type": ["string", "null"]},
                    "postal_code": {"type": ["string", "null"]},
                    "coordinates": {"$ref": "#/$defs/coordinates"},
                    "source": {"type": "string", "minLength": 1},
                    "source_layer": {"type": "string", "minLength": 1},
                    "confidence": {"type": ["number", "null"], "minimum": 0, "maximum": 1},
                    "service_area": {"enum": ["inside", "outside", "unknown"]},
                    "modes": {"type": "array", "items": {"enum": ["bus", "tram", "rail", "subway", "ferry"]}, "uniqueItems": true}
                }
            },
            "coordinates": {
                "type": "object",
                "additionalProperties": false,
                "required": ["latitude", "longitude"],
                "properties": {
                    "latitude": {"type": "number", "minimum": -90, "maximum": 90},
                    "longitude": {"type": "number", "minimum": -180, "maximum": 180}
                }
            },
            "request": {
                "type": "object",
                "additionalProperties": false,
                "required": ["request_id", "language", "timezone"],
                "properties": {
                    "request_id": {"type": "string", "minLength": 1},
                    "language": {"enum": ["en", "fi", "sv"]},
                    "timezone": {"type": "string", "minLength": 1}
                }
            },
            "source": {
                "type": "object",
                "additionalProperties": false,
                "required": ["provider", "dataset", "product", "retrieved_at", "attribution", "licenses", "realtime_included"],
                "properties": {
                    "provider": {"const": "digitransit"},
                    "dataset": {"const": "hsl"},
                    "product": {"const": "geocoding-v1"},
                    "retrieved_at": {"type": "string", "format": "date-time"},
                    "attribution": {"type": "string", "minLength": 1},
                    "licenses": {"type": "array", "items": {"type": "string"}, "minItems": 1, "uniqueItems": true},
                    "realtime_included": {"const": false}
                }
            }
        }
    })
}
