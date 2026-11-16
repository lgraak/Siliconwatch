mod common;

use common::Fixture;
use serde_json::Value;
use std::process::{Command, Stdio};

fn sample(f: &Fixture, count: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_siliconwatch"));
    command
        .args([
            "sample",
            "--count",
            count,
            "--interval-ms",
            "100",
            "--hwmon-root",
        ])
        .arg(&f.root);
    command
}

fn records(bytes: &[u8]) -> Vec<Value> {
    assert!(bytes.ends_with(b"\n"));
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn invalid_sample_arguments_return_usage_errors() {
    let f = Fixture::new();
    let bad_arguments = [
        vec!["sample"],
        vec!["sample", "--count", "1"],
        vec!["sample", "--interval-ms", "100"],
        vec!["sample", "--count"],
        vec!["sample", "--count", "1", "--interval-ms"],
        vec!["sample", "--count", "1", "--interval-ms", "100", "--json"],
        vec!["sensors", "--count", "1", "--interval-ms", "100"],
        vec!["sensors", "--json-lines"],
        vec![
            "sample",
            "--count",
            "1",
            "--count",
            "2",
            "--interval-ms",
            "100",
        ],
        vec![
            "sample",
            "--count",
            "1",
            "--interval-ms",
            "100",
            "--interval-ms",
            "200",
        ],
        vec![
            "sample",
            "--count",
            "1",
            "--interval-ms",
            "100",
            "--json-lines",
            "--json-lines",
        ],
        vec![
            "sample",
            "--count",
            "1",
            "--interval-ms",
            "100",
            "--hwmon-root",
            "a",
        ],
    ];
    for args in bad_arguments {
        let output = Command::new(env!("CARGO_BIN_EXE_siliconwatch"))
            .args(&args)
            .arg("--hwmon-root")
            .arg(&f.root)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
    }
    for (option, values) in [
        (
            "--count",
            vec![
                "0",
                "10001",
                "4294967296",
                "-1",
                "+1",
                "1.5",
                "",
                "one",
                "１",
                "18446744073709551616",
            ],
        ),
        (
            "--interval-ms",
            vec![
                "0",
                "99",
                "60001",
                "-100",
                "+100",
                "100.5",
                "",
                "1s",
                "18446744073709551616",
            ],
        ),
    ] {
        for value in values {
            let (count, interval) = if option == "--count" {
                (value, "100")
            } else {
                ("1", value)
            };
            let output = Command::new(env!("CARGO_BIN_EXE_siliconwatch"))
                .args([
                    "sample",
                    "--count",
                    count,
                    "--interval-ms",
                    interval,
                    "--hwmon-root",
                ])
                .arg(&f.root)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2), "{option} {value:?}");
            assert!(output.stdout.is_empty());
        }
    }
}

#[test]
fn json_lines_emits_exact_count_with_unchanged_nested_schemas() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    f.write("hwmon0/temp1_input", "42500");
    f.write("hwmon0/temp1_label", "CPU \"package\"\nline");
    f.write("hwmon0/temp2_input", "invalid");
    let output = sample(&f, "3").arg("--json-lines").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let lines = records(&output.stdout);
    assert_eq!(lines.len(), 3);
    for (i, record) in lines.iter().enumerate() {
        assert_eq!(record["record_type"], "siliconwatch.sample");
        assert_eq!(record["sampling_schema_version"], 1);
        assert_eq!(record["sequence"], i + 1);
        assert_eq!(record["requested_count"], 3);
        assert_eq!(record["interval_ms"], 100);
        assert!(record["error"].is_null());
        let snapshot = &record["snapshot"];
        assert_eq!(snapshot["schema_version"], 2);
        assert_eq!(snapshot["sensors"].as_array().unwrap().len(), 2);
        let good = &snapshot["sensors"][0];
        assert_eq!(good["raw_value"], 42500);
        assert_eq!(good["value"], 42.5);
        assert_eq!(good["label"], "CPU \"package\"\nline");
        assert_eq!(good["identity"]["schema_version"], 1);
        assert_eq!(snapshot["sensors"][1]["status"], "invalid");
        assert_eq!(record["freshness"]["physical_age_known"], false);
    }
}

#[test]
fn recoverable_root_failures_consume_attempts_and_complete_with_exit_zero() {
    let mut f = Fixture::new();
    f.root = f.root.join("missing");
    let output = sample(&f, "3").arg("--json-lines").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let lines = records(&output.stdout);
    assert_eq!(lines.len(), 3);
    for (i, record) in lines.iter().enumerate() {
        assert_eq!(record["sequence"], i + 1);
        assert!(record["snapshot"].is_null());
        assert!(record["error"]["message"].is_string());
        assert_eq!(record["freshness"]["basis"], "no_snapshot");
        assert!(record["freshness"]["lower_bound_ns"].is_null());
        assert!(record["freshness"]["upper_bound_ns"].is_null());
    }
}

#[test]
fn empty_snapshot_has_timing_but_no_value_freshness() {
    let f = Fixture::new();
    let output = sample(&f, "1").arg("--json-lines").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let record = &records(&output.stdout)[0];
    assert_eq!(record["snapshot"]["sensors"], serde_json::json!([]));
    assert!(record["timing"]["started_ns"].is_u64());
    assert_eq!(record["freshness"]["basis"], "no_sensor_observations");
    assert!(record["freshness"]["lower_bound_ns"].is_null());
    assert!(record["freshness"]["upper_bound_ns"].is_null());
    assert!(record["freshness"]["observation_window_exceeds_interval"].is_null());
}

#[test]
fn non_utf8_snapshot_serialization_errors_are_complete_recoverable_records() {
    use std::{ffi::OsString, fs, os::unix::ffi::OsStringExt};
    let mut f = Fixture::new();
    f.root = f.root.join(OsString::from_vec(vec![0xff]));
    fs::create_dir(&f.root).unwrap();
    let output = sample(&f, "2").arg("--json-lines").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let lines = records(&output.stdout);
    assert_eq!(lines.len(), 2);
    for record in lines {
        assert!(record["snapshot"].is_null());
        assert_eq!(record["error"]["kind"], "serialization_error");
        assert_eq!(record["freshness"]["basis"], "no_snapshot");
    }
}

#[test]
fn table_output_exposes_timing_and_escaped_observations_or_attempt_errors() {
    let mut f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    f.write("hwmon0/temp1_input", "42500");
    f.write("hwmon0/temp1_label", "CPU\u{1b}[31m");
    let output = sample(&f, "1").output().unwrap();
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Sample 1/1",
        "Monotonic offsets",
        "wall start",
        "Observation-age bounds at preparation",
        "Physical sensor age is unknown",
        "42.5",
    ] {
        assert!(text.contains(expected), "missing {expected:?}");
    }
    f.root = f.root.join("missing\u{1b}");
    let output = sample(&f, "1").output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(!output.stdout.contains(&0x1b));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Attempt error"));
    assert!(text.contains("no_snapshot"));
    assert!(!text.contains("42.5"));
}

#[test]
fn sample_output_failures_are_nonzero_in_both_formats() {
    use std::fs::OpenOptions;
    let f = Fixture::new();
    for json in [false, true] {
        let full = OpenOptions::new().write(true).open("/dev/full").unwrap();
        let mut command = sample(&f, "3");
        if json {
            command.arg("--json-lines");
        }
        let output = command.stdout(Stdio::from(full)).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn closed_output_pipe_fails_sampling_but_preserves_one_shot_convention() {
    use std::os::{fd::OwnedFd, unix::net::UnixStream};
    let f = Fixture::new();
    for command_name in ["sample", "sensors"] {
        // Close the consumer before launch, avoiding a race with real sampling time.
        let (writer, reader) = UnixStream::pair().unwrap();
        drop(reader);
        let mut command = if command_name == "sample" {
            let mut command = sample(&f, "3");
            command.arg("--json-lines");
            command
        } else {
            let mut command = Command::new(env!("CARGO_BIN_EXE_siliconwatch"));
            command
                .args(["sensors", "--json", "--hwmon-root"])
                .arg(&f.root);
            command
        };
        let output = command
            .stdout(Stdio::from(OwnedFd::from(writer)))
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if command_name == "sample" { 1 } else { 0 })
        );
    }
}
