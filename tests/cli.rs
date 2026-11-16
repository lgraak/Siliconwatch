mod common;

use common::Fixture;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_siliconwatch"))
}

#[test]
fn table_shows_labels_and_attribute_fallback() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "42500\n");
    f.write("hwmon0/temp1_label", "CPU package\n");
    f.write("hwmon0/fan3_input", "0\n");
    let output = cli()
        .args(["sensors", "--hwmon-root"])
        .arg(&f.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in ["CPU package", "42.5", "fan3"] {
        assert!(text.contains(expected), "missing {expected:?} in {text:?}");
    }
}

#[test]
fn json_is_clean_and_partial_readings_still_succeed() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "42500\n");
    f.write("hwmon0/temp1_label", "CPU package\n");
    f.write("hwmon0/temp2_input", "invalid\n");
    let output = cli()
        .args(["sensors", "--json", "--hwmon-root"])
        .arg(&f.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], 1);
    let sensors = json["sensors"].as_array().unwrap();
    assert_eq!(sensors.len(), 2);
    let valid = sensors
        .iter()
        .find(|s| s["attribute"] == "temp1_input")
        .unwrap();
    assert_eq!(valid["label"], "CPU package");
    assert_eq!(valid["raw_value"], 42500);
    assert_eq!(valid["value"], 42.5);
    assert_eq!(valid["status"], "ok");
    let invalid = sensors
        .iter()
        .find(|s| s["attribute"] == "temp2_input")
        .unwrap();
    assert_eq!(invalid["status"], "invalid");
    assert!(invalid["value"].is_null());
}

#[test]
fn root_failure_uses_exit_one_and_json_error_object() {
    let f = Fixture::new();
    let missing = f.root.join("missing");
    let output = cli()
        .args(["sensors", "--json", "--hwmon-root"])
        .arg(missing)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_object());
    assert!(json.get("error").is_some());
}

#[test]
fn terminal_escapes_control_characters_but_json_preserves_labels() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "42500\n");
    let label = "CPU \"package\"\u{1b}[31m";
    f.write("hwmon0/temp1_label", label);
    let output = cli()
        .args(["sensors", "--hwmon-root"])
        .arg(&f.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
    let output = cli()
        .args(["sensors", "--json", "--hwmon-root"])
        .arg(&f.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["sensors"][0]["label"], label);
}

#[test]
fn usage_errors_are_distinct_from_help_and_version() {
    for args in [
        vec![],
        vec!["unknown"],
        vec!["sensors", "--unknown"],
        vec!["sensors", "--hwmon-root"],
    ] {
        assert_eq!(cli().args(args).output().unwrap().status.code(), Some(2));
    }
    for flag in ["--help", "--version"] {
        let output = cli().arg(flag).output().unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
    }
}

#[test]
fn non_utf8_paths_return_json_errors_without_panicking_or_partial_output() {
    use std::{ffi::OsString, fs, os::unix::ffi::OsStringExt};

    let f = Fixture::new();
    let path = f.root.join(OsString::from_vec(vec![0xff]));
    for exists in [false, true] {
        if exists {
            fs::create_dir(&path).unwrap();
        }
        let output = cli()
            .args(["sensors", "--json", "--hwmon-root"])
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(json["error"].is_object());
        assert!(output.stderr.is_empty());
        if exists {
            assert_eq!(json["error"]["kind"], "serialization_error");
        }
    }
}
