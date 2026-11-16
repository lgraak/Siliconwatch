mod common;

use common::Fixture;
use siliconwatch::hwmon::{Status, scan};
use std::{fs, os::unix::fs::symlink};

#[test]
fn converts_sparse_channels_and_preserves_labels_and_zero() {
    let f = Fixture::new();
    f.write("hwmon2/name", "fixture-chip\n");
    f.write("hwmon2/temp1_label", " CPU package \n");
    let channels = [
        ("temp1_input", "42500\n", 42500, 42.5, "°C"),
        ("temp8_input", "-12500\n", -12500, -12.5, "°C"),
        ("in0_input", "1200\n", 1200, 1.2, "V"),
        ("fan9_input", "0\n", 0, 0.0, "RPM"),
    ];
    for (attribute, raw, ..) in channels {
        f.write(&format!("hwmon2/{attribute}"), raw);
    }
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.schema_version, 1);
    assert_eq!(snapshot.root, f.root);
    assert_eq!(snapshot.sensors.len(), channels.len());
    for (attribute, raw, raw_value, value, unit) in channels {
        let sensor = snapshot
            .sensors
            .iter()
            .find(|s| s.attribute == attribute)
            .unwrap();
        assert_eq!(sensor.device, "fixture-chip");
        assert_eq!(
            sensor.device_path,
            fs::canonicalize(f.root.join("hwmon2")).unwrap()
        );
        assert_eq!(sensor.source_path, f.root.join("hwmon2").join(attribute));
        assert_eq!(sensor.raw_text.as_deref(), Some(raw.trim()));
        assert_eq!(sensor.raw_value, Some(raw_value));
        assert_eq!(sensor.value, Some(value));
        assert_eq!(sensor.unit, Some(unit));
        assert_eq!(sensor.status, Status::Ok);
        assert_eq!(
            sensor.label.as_deref(),
            (attribute == "temp1_input").then_some("CPU package")
        );
    }
}

#[test]
fn retains_invalid_and_unsupported_inputs_without_reading_configuration() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "not-a-number\n");
    f.write("hwmon0/temp2_input", "9223372036854775808\n");
    f.write("hwmon0/mystery3_input", "17\n");
    f.write("hwmon0/temp1_max", "90000\n");
    f.write("hwmon0/pwm1", "255\n");
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 3);
    for sensor in &snapshot.sensors {
        assert_eq!(sensor.value, None);
        assert!(sensor.raw_text.is_some());
        if sensor.attribute == "mystery3_input" {
            assert_eq!(sensor.status, Status::Unsupported);
            assert_eq!(sensor.raw_value, Some(17));
            assert_eq!(sensor.unit, None);
        } else {
            assert_eq!(sensor.status, Status::Invalid);
            assert_eq!(sensor.raw_value, None);
            assert!(sensor.error.is_some());
        }
    }
}

#[test]
fn reports_missing_inputs_faults_and_disabled_channels_individually() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "42000\n");
    f.write("hwmon0/temp1_fault", "1\n");
    f.write("hwmon0/fan2_input", "0\n");
    f.write("hwmon0/fan2_enable", "0\n");
    symlink("missing", f.root.join("hwmon0/temp3_input")).unwrap();
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 3);
    for (attribute, status) in [
        ("temp1_input", Status::Fault),
        ("fan2_input", Status::Unavailable),
        ("temp3_input", Status::Unavailable),
    ] {
        assert_eq!(
            snapshot
                .sensors
                .iter()
                .find(|s| s.attribute == attribute)
                .unwrap()
                .status,
            status
        );
    }
}

#[test]
fn missing_root_fails_but_empty_root_succeeds() {
    let f = Fixture::new();
    assert!(scan(&f.root.join("missing")).is_err());
    assert!(scan(&f.root).unwrap().sensors.is_empty());
}

#[test]
fn malformed_and_dangling_flags_do_not_look_healthy() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "42000\n");
    f.write("hwmon0/temp1_fault", "2\n");
    f.write("hwmon0/temp2_input", "42000\n");
    symlink("missing", f.root.join("hwmon0/temp2_enable")).unwrap();
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors[0].status, Status::Invalid);
    assert_eq!(snapshot.sensors[1].status, Status::Unavailable);
    assert_eq!(snapshot.warnings.len(), 2);
    assert!(snapshot.sensors.iter().all(|s| s.raw_value == Some(42000)));
}

#[test]
fn flags_qualify_readable_inputs_but_preserve_parse_errors() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    let cases = [
        ("temp1", "invalid", "0", "1", Status::Invalid, None),
        ("temp2", "42000", "0", "1", Status::Unavailable, Some(42000)),
        ("power1", "17", "1", "1", Status::Fault, Some(17)),
        ("power2", "17", "0", "1", Status::Unavailable, Some(17)),
        ("temp3", "42000", "1", "0", Status::Ok, Some(42000)),
    ];
    for (channel, input, enable, fault, ..) in cases {
        for (suffix, text) in [("input", input), ("enable", enable), ("fault", fault)] {
            f.write(&format!("hwmon0/{channel}_{suffix}"), text);
        }
    }
    let snapshot = scan(&f.root).unwrap();
    assert!(snapshot.warnings.is_empty());
    assert_eq!(snapshot.sensors.len(), cases.len());
    for (channel, input, _, _, status, raw) in cases {
        let sensor = snapshot
            .sensors
            .iter()
            .find(|s| s.attribute == format!("{channel}_input"))
            .unwrap();
        assert_eq!(sensor.status, status);
        assert_eq!(sensor.raw_text.as_deref(), Some(input));
        assert_eq!(sensor.raw_value, raw);
        if channel.starts_with("power") {
            assert_eq!(sensor.value, None);
            assert_eq!(sensor.unit, None);
        } else {
            assert_eq!(sensor.value, raw.map(|v| v as f64 / 1000.0));
        }
        match status {
            Status::Invalid => assert!(
                sensor
                    .error
                    .as_deref()
                    .unwrap()
                    .starts_with("invalid integer:")
            ),
            Status::Unavailable => assert_eq!(sensor.error.as_deref(), Some("sensor is disabled")),
            Status::Fault => assert_eq!(sensor.error.as_deref(), Some("sensor reports a fault")),
            Status::Ok => assert_eq!(sensor.error, None),
            _ => unreachable!(),
        }
    }
}

#[test]
fn bad_siblings_and_unreadable_labels_do_not_hide_valid_readings() {
    let f = Fixture::new();
    f.write("hwmon0", "not a device directory");
    f.write("hwmon1/temp1_input", "42000\n");
    fs::create_dir(f.root.join("hwmon1/temp1_label")).unwrap();
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 1);
    assert_eq!(snapshot.sensors[0].device, "hwmon1");
    assert_eq!(snapshot.sensors[0].status, Status::Ok);
    assert_eq!(snapshot.sensors[0].label, None);
    assert!(
        snapshot
            .warnings
            .iter()
            .any(|w| w.path.ends_with("temp1_label"))
    );
}

#[test]
fn resolves_sysfs_style_device_symlink_identity() {
    let f = Fixture::new();
    let device = f.root.parent().unwrap().join("physical-device");
    fs::create_dir(&device).unwrap();
    fs::write(device.join("name"), "fixture\n").unwrap();
    fs::write(device.join("temp1_input"), "42000\n").unwrap();
    symlink(&device, f.root.join("hwmon7")).unwrap();
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 1);
    assert_eq!(
        snapshot.sensors[0].device_path,
        fs::canonicalize(device).unwrap()
    );
}

#[test]
fn permission_denied_is_distinct_when_enforced_for_this_user() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "42000\n");
    let input = f.root.join("hwmon0/temp1_input");
    fs::set_permissions(&input, fs::Permissions::from_mode(0)).unwrap();
    if fs::read_to_string(&input).is_ok() {
        eprintln!("Skipping permission assertion: this user can read mode-000 files.");
        return;
    }
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors[0].status, Status::PermissionDenied);
    assert_eq!(snapshot.sensors[0].value, None);
}
