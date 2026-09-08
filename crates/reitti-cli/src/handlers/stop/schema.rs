use serde_json::{json, Value};

fn coordinates() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["latitude","longitude"],
        "properties":{
            "latitude":{"type":"number","minimum":-90,"maximum":90},
            "longitude":{"type":"number","minimum":-180,"maximum":180}
        }
    })
}

fn stop() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["ref","id","name","code","platform","coordinates","distance_m","modes","wheelchair_boarding","service_area"],
        "properties":{
            "ref":{"type":"string","pattern":"^stop:HSL:[A-Za-z0-9_.-]+$"},
            "id":{"type":"string","pattern":"^HSL:[A-Za-z0-9_.-]+$"},
            "name":{"type":"string"},
            "code":{"type":["string","null"]},
            "platform":{"type":["string","null"]},
            "coordinates":{"anyOf":[coordinates(),{"type":"null"}]},
            "distance_m":{"type":["number","null"],"minimum":0},
            "modes":{"type":"array","uniqueItems":true,"items":{"enum":["bus","tram","rail","subway","ferry"]}},
            "wheelchair_boarding":{"enum":["accessible","not_accessible","unknown"]},
            "service_area":{"enum":["inside","outside","unknown"]}
        }
    })
}

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

fn route() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["id","short_name","long_name","mode"],
        "properties":{
            "id":{"type":"string","pattern":"^HSL:[A-Za-z0-9_.-]+$"},
            "short_name":{"type":["string","null"]},
            "long_name":{"type":["string","null"]},
            "mode":{"type":["string","null"],"enum":["bus","tram","rail","subway","ferry",null]}
        }
    })
}

fn alert_entity() -> Value {
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
            "entities":{"type":"array","items":alert_entity()},
            "source_feed":{"type":["string","null"]}
        }
    })
}

pub fn stop_schema() -> Value {
    json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "type":"object", "additionalProperties":false,
        "required":["search","count","complete","stops","request","source"],
        "properties":{
            "search":{
                "type":"object", "additionalProperties":false,
                "required":["kind","query","coordinates","radius_m","limit"],
                "properties":{
                    "kind":{"enum":["query","near"]},
                    "query":{"type":["string","null"]},
                    "coordinates":{"anyOf":[coordinates(),{"type":"null"}]},
                    "radius_m":{"type":["integer","null"],"minimum":1,"maximum":5000},
                    "limit":{"type":"integer","minimum":1,"maximum":10}
                },
                "allOf":[
                    {"if":{"properties":{"kind":{"const":"query"}}},"then":{"required":["query"],"properties":{"query":{"type":"string","minLength":1},"coordinates":{"type":"null"},"radius_m":{"type":"null"}}}},
                    {"if":{"properties":{"kind":{"const":"near"}}},"then":{"properties":{"query":{"type":"null"},"coordinates":coordinates(),"radius_m":{"type":"integer","minimum":1,"maximum":5000}}}}
                ]
            },
            "count":{"type":"integer","minimum":0},
            "complete":{"type":"boolean"},
            "stops":{"type":"array","maxItems":10,"items":stop()},
            "request":request(),
            "source":source()
        }
    })
}

pub fn departure_schema() -> Value {
    json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "type":"object", "additionalProperties":false,
        "required":["stop","at","time_source","window_seconds","count","complete","departures","request","source"],
        "properties":{
            "stop":stop(),
            "at":{"type":"string","format":"date-time"},
            "time_source":{"enum":["argument","clock"]},
            "window_seconds":{"type":"integer","minimum":60,"maximum":86400},
            "count":{"type":"integer","minimum":0},
            "complete":{"type":"boolean"},
            "departures":{"type":"array","maxItems":50,"items":{
                "type":"object", "additionalProperties":false,
                "required":["trip_id","route","headsign","platform","service_date","departure","cancelled","alerts"],
                "properties":{
                    "trip_id":{"type":"string","minLength":1},
                    "route":route(),
                    "headsign":{"type":["string","null"]},
                    "platform":{"type":["string","null"]},
                    "service_date":{"type":"string","format":"date"},
                    "departure":{
                        "type":"object", "additionalProperties":false,
                        "required":["state","scheduled_time","estimated_time","delay_seconds","observed_realtime"],
                        "properties":{
                            "state":{"enum":["scheduled","updated","cancelled","added","unknown"]},
                            "scheduled_time":{"type":"string","format":"date-time"},
                            "estimated_time":{"type":["string","null"],"format":"date-time"},
                            "delay_seconds":{"type":["integer","null"]},
                            "observed_realtime":{"type":"boolean"}
                        }
                    },
                    "cancelled":{"type":"boolean"},
                    "alerts":{"type":"array","items":alert()}
                }
            }},
            "request":request(),
            "source":source()
        }
    })
}
