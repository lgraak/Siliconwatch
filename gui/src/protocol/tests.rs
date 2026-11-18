use super::*;
use serde_json::json;

pub(crate) const INSTANCE: &str = "0123456789abcdef0123456789abcdef";
pub(crate) const SECOND: &str = "1123456789abcdef0123456789abcdef";
pub(crate) fn frame(v: &Value) -> Vec<u8> {
    let mut b = serde_json::to_vec(v).unwrap();
    b.push(b'\n');
    b
}
pub(crate) fn info_value() -> Value {
    json!({"api_version":1,"instance_id":INSTANCE,"ok":true,"result":{
        "supported_versions":[1],"methods":["get_info","get_snapshot"],"snapshot_schema_version":2,"identity_schema_version":1,"read_only":true,
        "limits":{"request_bytes":4096,"response_bytes":1048576,"active_clients":1,"request_deadline_ms":1000,"response_deadline_ms":2000,"max_sensors":4096,"max_warnings":1024,"max_identity_issues_per_sensor":64,"max_text_bytes":16384,"max_total_text_bytes":262144}
    }})
}
pub(crate) fn sample_value(sequence: u64) -> Value {
    let attempt = json!({"sequence":sequence.to_string(),"started_ns":"100","finished_ns":"200","published_ns":"250","wall_start_unix_ms":"0","wall_time_error":null});
    json!({"api_version":1,"instance_id":INSTANCE,"ok":true,"result":{
        "lifecycle":"running","assessed_at_ns":"300","stale_after_ns":"1000000000","producer":"active","service_current":true,"physical_age_known":false,
        "latest":{"state":"succeeded","attempt":attempt,"error":null},
        "observation":{"attempt":attempt,"role":"latest_attempt","freshness":{"state":"within_budget","lower_ns":"100","upper_ns":"200"},
        "snapshot":{"schema_version":2,"root":"/fixture","warnings":[],"sensors":[{
            "identity":{"schema_version":1,"candidate":null,"status":"insufficient_evidence","evidence":{"provider":"fixture","parent":null,"device_link":null,"canonical_parent":null,"channel":"temp1_input","measurement":"temperature_input_millidegree_celsius"},"issues":["fixture has no hardware anchor"]},
            "device":"fixture","device_path":"/fixture/hwmon0","attribute":"temp1_input","label":"Example","source_path":"/fixture/hwmon0/temp1_input","raw_text":"42000","raw_value":42000,"value":42.0,"unit":"°C","status":"ok","error":null
        }]}}
    }})
}
fn row(v: &mut Value) -> &mut Value {
    &mut v["result"]["observation"]["snapshot"]["sensors"][0]
}
#[test]
fn parses_lossless_limits_and_metadata() {
    assert_eq!(
        parse_info(&frame(&info_value()))
            .unwrap()
            .info
            .limits
            .max_sensors,
        4096
    );
    for raw in [i64::MIN, i64::MAX, 9_007_199_254_740_993] {
        let mut v = sample_value(u64::MAX);
        row(&mut v)["raw_value"] = json!(raw);
        let s = parse_snapshot(&frame(&v)).unwrap();
        let o = s.assessment.observation.unwrap();
        assert_eq!(o.snapshot.sensors[0].raw_value, Some(raw));
        assert_eq!(o.attempt.sequence, u64::MAX);
    }
    let mut v = sample_value(1);
    for p in ["latest", "observation"] {
        v["result"][p]["attempt"]["wall_start_unix_ms"] = json!(u64::MAX.to_string());
    }
    assert!(parse_snapshot(&frame(&v)).is_ok());
}
#[test]
fn nullable_is_required_and_wrong_types_fail() {
    for field in ["label", "raw_text", "raw_value", "value", "unit", "error"] {
        let mut v = sample_value(1);
        row(&mut v).as_object_mut().unwrap().remove(field);
        assert!(parse_snapshot(&frame(&v)).is_err(), "missing {field}");
    }
    for field in [
        "provider",
        "parent",
        "device_link",
        "canonical_parent",
        "channel",
        "measurement",
    ] {
        let mut v = sample_value(1);
        row(&mut v)["identity"]["evidence"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            parse_snapshot(&frame(&v)).is_err(),
            "missing evidence {field}"
        );
    }
    for value in [json!("42"), json!(42.0), json!(9223372036854775808_u64)] {
        let mut v = sample_value(1);
        row(&mut v)["raw_value"] = value;
        assert!(parse_snapshot(&frame(&v)).is_err());
    }
    let mut v = sample_value(1);
    v["result"]["observation"] = Value::Null;
    assert!(parse_snapshot(&frame(&v)).is_err());
}
#[test]
fn rejects_noncanonical_decimal_metadata() {
    for x in [
        "",
        "+1",
        "-1",
        "01",
        "1.0",
        "1e0",
        " 1",
        "١",
        "18446744073709551616",
    ] {
        let mut v = sample_value(1);
        v["result"]["latest"]["attempt"]["sequence"] = json!(x);
        assert!(parse_snapshot(&frame(&v)).is_err(), "{x}");
    }
    let mut v = sample_value(1);
    v["result"]["assessed_at_ns"] = json!("340282366920938463463374607431768211456");
    assert!(parse_snapshot(&frame(&v)).is_err());
    v["result"]["assessed_at_ns"] = json!(300);
    assert!(parse_snapshot(&frame(&v)).is_err());
}
#[test]
fn rejects_duplicates_even_in_additive_data_and_accepts_unique_additions() {
    let original = String::from_utf8(frame(&sample_value(1))).unwrap();
    let duplicate = original.replacen(
        "\"api_version\":1",
        "\"api_version\":1,\"api_version\":1",
        1,
    );
    assert!(parse_snapshot(duplicate.as_bytes()).is_err());
    let duplicate = original.replacen(
        "\"raw_value\":42000",
        "\"raw_value\":42000,\"raw_value\":42000",
        1,
    );
    assert!(parse_snapshot(duplicate.as_bytes()).is_err());
    let duplicate = original.replacen("{", "{\"future\":{\"x\":1,\"x\":2},", 1);
    assert!(parse_snapshot(duplicate.as_bytes()).is_err());
    let mut v = sample_value(1);
    v["future"] = json!({"safe":[1,2,3]});
    row(&mut v)["future"] = json!(true);
    assert!(parse_snapshot(&frame(&v)).is_ok());
}
#[test]
fn fails_closed_on_versions_unknown_enums_and_capabilities() {
    for path in ["lifecycle", "producer"] {
        let mut v = sample_value(1);
        v["result"][path] = json!("future");
        assert!(parse_snapshot(&frame(&v)).is_err());
    }
    {
        let path = "status";
        let mut v = sample_value(1);
        row(&mut v)[path] = json!("future");
        assert!(parse_snapshot(&frame(&v)).is_err());
    }
    let mut v = sample_value(1);
    row(&mut v)["identity"]["status"] = json!("future");
    assert!(parse_snapshot(&frame(&v)).is_err());
    for version in [json!(1.0), json!("1"), json!(2)] {
        let mut v = sample_value(1);
        v["api_version"] = version;
        assert!(parse_snapshot(&frame(&v)).is_err());
    }
    let mut v = info_value();
    v["result"]["read_only"] = json!(false);
    assert!(parse_info(&frame(&v)).is_err());
    v = info_value();
    v["result"]["limits"]["max_sensors"] = json!(8192);
    assert!(parse_info(&frame(&v)).is_err());
}
#[test]
fn verifies_age_arithmetic_equality_and_service_conjunction() {
    let mut v = sample_value(1);
    v["result"]["stale_after_ns"] = json!("200");
    assert!(parse_snapshot(&frame(&v)).is_ok());
    v["result"]["stale_after_ns"] = json!("199");
    assert!(parse_snapshot(&frame(&v)).is_err());
    v["result"]["observation"]["freshness"]["state"] = json!("stale");
    v["result"]["service_current"] = json!(false);
    assert!(parse_snapshot(&frame(&v)).is_ok());
    v["result"]["observation"]["freshness"]["lower_ns"] = json!("99");
    assert!(parse_snapshot(&frame(&v)).is_err());
    v = sample_value(1);
    v["result"]["latest"]["attempt"]["finished_ns"] = json!("99");
    assert!(parse_snapshot(&frame(&v)).is_err());
    v = sample_value(1);
    v["result"]["physical_age_known"] = json!(true);
    assert!(parse_snapshot(&frame(&v)).is_err());
}
#[test]
fn preserves_failed_historical_observation_and_fault_values() {
    let mut v = sample_value(1);
    v["result"]["latest"]["state"] = json!("failed");
    v["result"]["latest"]["attempt"]["sequence"] = json!("2");
    v["result"]["latest"]["error"] =
        json!({"kind":"Other","raw_os_error":null,"message":"fixture failure"});
    v["result"]["observation"]["role"] = json!("historical_after_failure");
    v["result"]["service_current"] = json!(false);
    row(&mut v)["status"] = json!("fault");
    row(&mut v)["error"] = json!("fixture fault");
    let a = parse_snapshot(&frame(&v)).unwrap().assessment;
    assert_eq!(a.latest.attempt.unwrap().sequence, 2);
    let o = a.observation.unwrap();
    assert_eq!(o.attempt.sequence, 1);
    assert_eq!(o.snapshot.sensors[0].value, Some(42.0));
    v["result"]["producer"] = json!("stopped");
    assert!(parse_snapshot(&frame(&v)).is_err());
    v["result"]["observation"]["role"] = json!("historical_after_stop");
    assert!(parse_snapshot(&frame(&v)).is_ok());
}
#[test]
fn distinguishes_empty_cache_empty_inventory_and_initial_failure() {
    let mut v = sample_value(1);
    v["result"]["observation"]["snapshot"]["sensors"] = json!([]);
    v["result"]["observation"]["freshness"] =
        json!({"state":"no_sensor_observations","lower_ns":null,"upper_ns":null});
    v["result"]["service_current"] = json!(false);
    assert!(parse_snapshot(&frame(&v)).is_ok());
    v["result"]["latest"] = json!({"state":"empty","attempt":null,"error":null});
    v["result"]["observation"] = Value::Null;
    assert!(parse_snapshot(&frame(&v)).is_ok());
    v = sample_value(1);
    v["result"]["latest"]["state"] = json!("failed");
    v["result"]["latest"]["error"] =
        json!({"kind":"Other","raw_os_error":5,"message":"failed first attempt"});
    v["result"]["observation"] = Value::Null;
    v["result"]["service_current"] = json!(false);
    assert!(parse_snapshot(&frame(&v)).is_ok());
}
#[test]
fn text_and_array_bounds_are_enforced_without_truncation() {
    let mut v = sample_value(1);
    row(&mut v)["label"] = json!("x".repeat(MAX_TEXT));
    assert!(parse_snapshot(&frame(&v)).is_ok());
    row(&mut v)["label"] = json!("é".repeat(MAX_TEXT / 2 + 1));
    assert!(parse_snapshot(&frame(&v)).is_err());
    v = sample_value(1);
    row(&mut v)["identity"]["issues"] = json!(vec!["issue"; 65]);
    assert!(parse_snapshot(&frame(&v)).is_err());
    v = sample_value(1);
    v["result"]["observation"]["snapshot"]["warnings"] =
        json!(vec![json!({"path":"","message":""}); 1025]);
    assert!(parse_snapshot(&frame(&v)).is_err());
    v = sample_value(1);
    v["result"]["observation"]["snapshot"]["warnings"] =
        json!(vec![json!({"path":"","message":"x".repeat(MAX_TEXT)}); 17]);
    assert!(parse_snapshot(&frame(&v)).is_err());
}
#[test]
fn framing_size_utf8_and_trailing_bytes_fail_closed() {
    let bytes = frame(&sample_value(1));
    assert!(parse_snapshot(&bytes[..bytes.len() - 1]).is_err());
    let mut two = bytes.clone();
    two.extend_from_slice(&bytes);
    assert!(parse_snapshot(&two).is_err());
    assert!(parse_snapshot(b"{\xff}\n").is_err());
    assert!(parse_snapshot(b"[]\n").is_err());
    let mut boundary = bytes.clone();
    boundary.pop();
    boundary.resize(RESPONSE_BYTES - 1, b' ');
    boundary.push(b'\n');
    assert!(parse_snapshot(&boundary).is_ok());
    boundary.insert(boundary.len() - 1, b' ');
    assert!(parse_snapshot(&boundary).is_err());
}
#[test]
fn remote_errors_validate_retryability_and_versions() {
    let mut v = json!({"api_version":1,"instance_id":INSTANCE,"ok":false,"error":{"code":"cache_busy","message":"busy","retryable":true}});
    assert!(matches!(
        parse_info(&frame(&v)),
        Err(ProtocolError::Remote { .. })
    ));
    v["error"]["retryable"] = json!(false);
    assert!(matches!(
        parse_info(&frame(&v)),
        Err(ProtocolError::Invalid(_))
    ));
    v["error"]["code"] = json!("unsupported_version");
    v["error"]["supported_versions"] = json!([1]);
    assert!(matches!(
        parse_info(&frame(&v)),
        Err(ProtocolError::Remote { .. })
    ));
    v["result"] = json!({});
    assert!(matches!(
        parse_info(&frame(&v)),
        Err(ProtocolError::Invalid(_))
    ));
}
