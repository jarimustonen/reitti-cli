use serde_json::{json, Value};

fn request() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["request_id","language","timezone"],
        "properties":{
            "request_id":{"type":"string","minLength":1},
            "language":{"enum":["en","fi","sv"]},
            "timezone":{"type":"string","minLength":1}
        }
    })
}

fn source() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["provider","dataset","product","retrieved_at","attribution","licenses","realtime_included"],
        "properties":{
            "provider":{"const":"digitransit"},
            "dataset":{"const":"hsl"},
            "product":{"const":"routing-v2-hsl-gtfs"},
            "retrieved_at":{"type":"string","format":"date-time"},
            "attribution":{"type":"string","minLength":1},
            "licenses":{"type":"array","items":{"type":"string"}},
            "realtime_included":{"const":true}
        }
    })
}

fn entity() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["kind","id","route_id","stop_id","trip_id"],
        "properties":{
            "kind":{"type":"string","minLength":1},
            "id":{"type":["string","null"]},
            "route_id":{"type":["string","null"]},
            "stop_id":{"type":["string","null"]},
            "trip_id":{"type":["string","null"]}
        }
    })
}

fn alert() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["id","header","description","severity","source_severity","effect","source_effect","valid_from","valid_until","entities","source_feed"],
        "properties":{
            "id":{"type":"string","minLength":1},
            "header":{"type":["string","null"]},
            "description":{"type":"string"},
            "severity":{"enum":["severe","warning","info","unknown"]},
            "source_severity":{"type":["string","null"]},
            "effect":{"enum":["detour","no_service","reduced_service","significant_delays","modified_service","stop_moved","other_effect","unknown"]},
            "source_effect":{"type":["string","null"]},
            "valid_from":{"type":["string","null"],"format":"date-time"},
            "valid_until":{"type":["string","null"],"format":"date-time"},
            "entities":{"type":"array","items":entity()},
            "source_feed":{"type":["string","null"]}
        }
    })
}

pub fn schema() -> Value {
    json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "type":"object", "additionalProperties":false,
        "required":["filters","count","complete","alerts","request","source"],
        "properties":{
            "filters":{
                "type":"object", "additionalProperties":false,
                "required":["routes","stops","active_at","time_source","limit"],
                "properties":{
                    "routes":{"type":"array","uniqueItems":true,"items":{"type":"string","pattern":"^HSL:[A-Za-z0-9_.-]+$"}},
                    "stops":{"type":"array","uniqueItems":true,"items":{"type":"string","pattern":"^HSL:[A-Za-z0-9_.-]+$"}},
                    "active_at":{"type":"string","format":"date-time"},
                    "time_source":{"enum":["argument","clock"]},
                    "limit":{"type":"integer","minimum":1,"maximum":100}
                }
            },
            "count":{"type":"integer","minimum":0},
            "complete":{"type":"boolean"},
            "alerts":{"type":"array","maxItems":100,"items":alert()},
            "request":request(),
            "source":source()
        }
    })
}
