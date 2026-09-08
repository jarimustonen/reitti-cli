use serde_json::{json, Value};
fn stop() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["ref","id","name","code","coordinates","distance_m","modes","wheelchair_boarding","service_area"],"properties":{"ref":{"type":"string"},"id":{"type":"string"},"name":{"type":"string"},"code":{"type":["string","null"]},"coordinates":{"type":["object","null"]},"distance_m":{"type":["number","null"],"minimum":0},"modes":{"type":"array"},"wheelchair_boarding":{"enum":["accessible","not_accessible","unknown"]},"service_area":{"enum":["inside","outside","unknown"]}}})
}
pub fn stop_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["search","count","complete","stops","request","source"],"properties":{"search":{"type":"object"},"count":{"type":"integer","minimum":0},"complete":{"type":"boolean"},"stops":{"type":"array","items":stop()},"request":{"type":"object"},"source":{"type":"object"}}})
}
pub fn departure_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["stop","at","time_source","window_seconds","count","complete","departures","request","source"],"properties":{"stop":stop(),"at":{"type":"string","format":"date-time"},"time_source":{"enum":["argument","clock"]},"window_seconds":{"type":"integer","minimum":60,"maximum":86400},"count":{"type":"integer","minimum":0},"complete":{"type":"boolean"},"departures":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["trip_id","route","headsign","platform","service_date","departure","cancelled","alerts"],"properties":{"trip_id":{"type":"string"},"route":{"type":"object"},"headsign":{"type":["string","null"]},"platform":{"type":["string","null"]},"service_date":{"type":"string","format":"date"},"departure":{"type":"object"},"cancelled":{"type":"boolean"},"alerts":{"type":"array"}}}},"request":{"type":"object"},"source":{"type":"object"}}})
}
