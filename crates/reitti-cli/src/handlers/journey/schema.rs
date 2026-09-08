use serde_json::{json, Value};

pub fn schema() -> Value {
    json!({
        "type":"object",
        "additionalProperties":false,
        "required":["request","resolved","count","complete","filtering","alternatives","sources"],
        "properties":{
            "request": request(),
            "resolved": {"type":"object","additionalProperties":false,"required":["from","to"],"properties":{"from":{"$ref":"#/$defs/resolved_location"},"to":{"$ref":"#/$defs/resolved_location"}}},
            "count":{"type":"integer","minimum":1,"maximum":6},
            "complete":{"type":"boolean"},
            "filtering": filtering(),
            "alternatives":{"type":"array","minItems":1,"maxItems":6,"items":{"$ref":"#/$defs/itinerary"}},
            "sources":{"type":"array","minItems":1,"maxItems":3,"items":{"$ref":"#/$defs/source"}}
        },
        "$defs":{
            "coordinates": coordinates(),
            "resolved_location": resolved_location(),
            "source": source(),
            "itinerary": itinerary(),
            "accessibility": accessibility(),
            "realtime_summary": realtime_summary(),
            "comparison": comparison(),
            "leg": leg(),
            "place": place(),
            "route": route(),
            "evidence": evidence(),
            "stop_call": stop_call(),
            "stop": stop(),
            "step": step(),
            "geometry": geometry(),
            "alert": alert(),
            "alert_entity": alert_entity()
        }
    })
}

fn request() -> Value {
    json!({"type":"object","additionalProperties":false,
        "required":["request_id","language","timezone","from","to","time","modes","max_walk_m","wheelchair","include_geometry","limit"],
        "properties":{
            "request_id":{"type":"string","minLength":1},"language":{"enum":["en","fi","sv"]},"timezone":{"type":"string","minLength":1},
            "from":{"type":"string","minLength":1},"to":{"type":"string","minLength":1},
            "time":{"type":"object","additionalProperties":false,"required":["kind","value","time_source"],"properties":{"kind":{"enum":["depart_at","arrive_by"]},"value":{"type":"string","format":"date-time"},"time_source":{"enum":["argument","clock"]}}},
            "modes":{"type":"array","minItems":1,"uniqueItems":true,"items":{"enum":["bus","tram","rail","subway","ferry"]}},
            "max_walk_m":{"type":["integer","null"],"minimum":0,"maximum":20000},"wheelchair":{"type":"boolean"},"include_geometry":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":6}
        }
    })
}
fn filtering() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["provider_count","returned_count","max_walk_m","excluded_over_limit","excluded_over_cap","excluded_unknown_distance"],"properties":{
        "provider_count":{"type":"integer","minimum":1},"returned_count":{"type":"integer","minimum":1,"maximum":6},"max_walk_m":{"type":["integer","null"],"minimum":0,"maximum":20000},"excluded_over_limit":{"type":"integer","minimum":0},"excluded_over_cap":{"type":"integer","minimum":0},"excluded_unknown_distance":{"type":"integer","minimum":0}
    }})
}
fn coordinates() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["latitude","longitude"],"properties":{"latitude":{"type":"number","minimum":-90,"maximum":90},"longitude":{"type":"number","minimum":-180,"maximum":180}}})
}
fn resolved_location() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["input_ref","resolution","ref","kind","id","label","name","locality","neighbourhood","postal_code","coordinates","source","source_layer","confidence","service_area","modes"],"properties":{
        "input_ref":{"type":"string","minLength":1},"resolution":{"enum":["exact_coordinate","stable_place","stable_stop","unique_query"]},"ref":{"type":"string","minLength":1},"kind":{"enum":["address","venue","stop","locality","other"]},"id":{"type":["string","null"]},"label":{"type":"string"},"name":{"type":"string"},"locality":{"type":["string","null"]},"neighbourhood":{"type":["string","null"]},"postal_code":{"type":["string","null"]},"coordinates":{"$ref":"#/$defs/coordinates"},"source":{"type":"string"},"source_layer":{"type":"string"},"confidence":{"type":["number","null"],"minimum":0,"maximum":1},"service_area":{"enum":["inside","outside","unknown"]},"modes":{"type":"array","uniqueItems":true,"items":{"enum":["bus","tram","rail","subway","ferry"]}}
    }})
}
fn source() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["provider","dataset","product","retrieved_at","attribution","licenses","realtime_included"],"properties":{"provider":{"type":"string"},"dataset":{"type":"string"},"product":{"type":"string"},"retrieved_at":{"type":"string","format":"date-time"},"attribution":{"type":"string"},"licenses":{"type":"array","items":{"type":"string"}},"realtime_included":{"type":"boolean"}}})
}
fn itinerary() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["id","source_index","start_time","end_time","duration_seconds","transfers","walk_seconds","wait_seconds","transit_seconds","walk_distance_m","accessibility","realtime","comparison","alerts","fare","legs"],"properties":{
        "id":{"type":"string","pattern":"^alt-[1-6]$"},"source_index":{"type":"integer","minimum":0,"maximum":5},"start_time":{"type":"string","format":"date-time"},"end_time":{"type":"string","format":"date-time"},"duration_seconds":{"type":["integer","null"],"minimum":0},"transfers":{"type":"integer","minimum":0},"walk_seconds":{"type":["integer","null"],"minimum":0},"wait_seconds":{"type":["integer","null"],"minimum":0},"transit_seconds":{"type":["integer","null"],"minimum":0},"walk_distance_m":{"type":["number","null"],"minimum":0},"accessibility":{"$ref":"#/$defs/accessibility"},"realtime":{"$ref":"#/$defs/realtime_summary"},"comparison":{"type":"array","maxItems":5,"items":{"$ref":"#/$defs/comparison"}},"alerts":{"type":"array","items":{"$ref":"#/$defs/alert"}},"fare":{"type":"null"},"legs":{"type":"array","items":{"$ref":"#/$defs/leg"}}
    }})
}
fn accessibility() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["wheelchair_requested","status","evidence"],"properties":{"wheelchair_requested":{"type":"boolean"},"status":{"enum":["unknown"]},"evidence":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["leg_index","endpoint","wheelchair_boarding"],"properties":{"leg_index":{"type":"integer","minimum":0},"endpoint":{"enum":["from","to"]},"wheelchair_boarding":{"enum":["accessible","not_accessible","unknown"]}}}}}})
}
fn realtime_summary() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["status","updated_legs","scheduled_only_legs","cancelled_legs","unknown_legs"],"properties":{"status":{"enum":["updated","scheduled_only","mixed","cancelled","unknown"]},"updated_legs":{"type":"integer","minimum":0},"scheduled_only_legs":{"type":"integer","minimum":0},"cancelled_legs":{"type":"integer","minimum":0},"unknown_legs":{"type":"integer","minimum":0}}})
}
fn comparison() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["label","metric","value","tied"],"properties":{"label":{"enum":["fastest","earliest_arrival","latest_departure","fewest_transfers","least_walking"]},"metric":{"enum":["duration_seconds","end_time","start_time","transfers","walk_distance_m"]},"value":{},"tied":{"type":"integer","minimum":1,"maximum":6}}})
}
fn place() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["name","stop_ref","platform","coordinates","wheelchair_boarding"],"properties":{"name":{"type":"string"},"stop_ref":{"type":["string","null"]},"platform":{"type":["string","null"]},"coordinates":{"oneOf":[{"$ref":"#/$defs/coordinates"},{"type":"null"}]},"wheelchair_boarding":{"enum":["accessible","not_accessible","unknown"]}}})
}
fn route() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["id","short_name","long_name","mode"],"properties":{"id":{"type":"string"},"short_name":{"type":["string","null"]},"long_name":{"type":["string","null"]},"mode":{"type":["string","null"],"enum":["bus","tram","rail","subway","ferry",null]}}})
}
fn evidence() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["state","scheduled_time","estimated_time","delay_seconds","observed_realtime"],"properties":{"state":{"enum":["scheduled","updated","cancelled","added","unknown"]},"scheduled_time":{"type":"string","format":"date-time"},"estimated_time":{"type":["string","null"],"format":"date-time"},"delay_seconds":{"type":["integer","null"]},"observed_realtime":{"type":"boolean"}}})
}
fn leg() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["index","mode","from","to","route","trip_id","headsign","duration_seconds","distance_m","start","end","continues_previous_vehicle","intermediate_stops","steps","navigation_complete","geometry","alerts","cancelled"],"properties":{
        "index":{"type":"integer","minimum":0},"mode":{"type":"string"},"from":{"$ref":"#/$defs/place"},"to":{"$ref":"#/$defs/place"},"route":{"oneOf":[{"$ref":"#/$defs/route"},{"type":"null"}]},"trip_id":{"type":["string","null"]},"headsign":{"type":["string","null"]},"duration_seconds":{"type":["number","null"],"minimum":0},"distance_m":{"type":["number","null"],"minimum":0},"start":{"$ref":"#/$defs/evidence"},"end":{"$ref":"#/$defs/evidence"},"continues_previous_vehicle":{"type":["boolean","null"]},"intermediate_stops":{"type":"array","items":{"$ref":"#/$defs/stop_call"}},"steps":{"type":"array","maxItems":200,"items":{"$ref":"#/$defs/step"}},"navigation_complete":{"type":"boolean"},"geometry":{"oneOf":[{"$ref":"#/$defs/geometry"},{"type":"null"}]},"alerts":{"type":"array","items":{"$ref":"#/$defs/alert"}},"cancelled":{"type":"boolean"}
    }})
}
fn stop_call() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["kind","stop"],"properties":{"kind":{"type":"string"},"stop":{"oneOf":[{"$ref":"#/$defs/stop"},{"type":"null"}]}}})
}
fn stop() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["ref","id","name","code","platform","coordinates","distance_m","modes","wheelchair_boarding","service_area"],"properties":{"ref":{"type":"string"},"id":{"type":"string"},"name":{"type":"string"},"code":{"type":["string","null"]},"platform":{"type":["string","null"]},"coordinates":{"oneOf":[{"$ref":"#/$defs/coordinates"},{"type":"null"}]},"distance_m":{"type":["number","null"],"minimum":0},"modes":{"type":"array","items":{"enum":["bus","tram","rail","subway","ferry"]}},"wheelchair_boarding":{"enum":["accessible","not_accessible","unknown"]},"service_area":{"enum":["inside","outside","unknown"]}}})
}
fn step() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["instruction","distance_m","street_name","generated_name","relative_direction","absolute_direction","coordinates","area","stay_on","exit"],"properties":{"instruction":{"type":["string","null"]},"distance_m":{"type":["number","null"],"minimum":0},"street_name":{"type":["string","null"]},"generated_name":{"type":["boolean","null"]},"relative_direction":{"type":["string","null"]},"absolute_direction":{"type":["string","null"]},"coordinates":{"oneOf":[{"$ref":"#/$defs/coordinates"},{"type":"null"}]},"area":{"type":["boolean","null"]},"stay_on":{"type":["boolean","null"]},"exit":{"type":["string","null"]}}})
}
fn geometry() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["type","coordinates"],"properties":{"type":{"const":"LineString"},"coordinates":{"type":"array","maxItems":10000,"items":{"type":"array","minItems":2,"maxItems":2,"prefixItems":[{"type":"number","minimum":-180,"maximum":180},{"type":"number","minimum":-90,"maximum":90}]}}}})
}
fn alert() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["id","header","description","severity","source_severity","effect","source_effect","valid_from","valid_until","entities","source_feed"],"properties":{"id":{"type":"string"},"header":{"type":["string","null"]},"description":{"type":"string"},"severity":{"enum":["severe","warning","info","unknown"]},"source_severity":{"type":["string","null"]},"effect":{"enum":["detour","no_service","reduced_service","significant_delays","modified_service","stop_moved","other_effect","unknown"]},"source_effect":{"type":["string","null"]},"valid_from":{"type":["string","null"],"format":"date-time"},"valid_until":{"type":["string","null"],"format":"date-time"},"entities":{"type":"array","items":{"$ref":"#/$defs/alert_entity"}},"source_feed":{"type":["string","null"]}}})
}
fn alert_entity() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["kind","id","route_id","stop_id","trip_id"],"properties":{"kind":{"type":"string"},"id":{"type":["string","null"]},"route_id":{"type":["string","null"]},"stop_id":{"type":["string","null"]},"trip_id":{"type":["string","null"]}}})
}
