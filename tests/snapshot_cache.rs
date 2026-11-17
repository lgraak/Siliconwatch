mod common;

use common::Fixture;
use siliconwatch::{
    hwmon::{self, Status},
    identity::IdentityStatus,
    snapshot_cache::{
        self, Acquisition, CacheReader, Freshness, LatestAttempt, ObservationRole, ProducerState,
    },
};
use std::{fs, io, os::unix::fs::symlink, path::PathBuf, time::Duration};

// Integration fixtures exercise real filesystem acquisition, never real sysfs.
// Exact timing and threshold tests belong to the cache's controlled-clock unit tests.
const AGE_BUDGET: Duration = Duration::from_secs(3600);

fn direct_json(root: &std::path::Path) -> serde_json::Value {
    serde_json::to_value(hwmon::scan(root).unwrap()).unwrap()
}

fn assert_latest(reader: &CacheReader, expected: &serde_json::Value, sequence: u64) {
    let mut view = reader.read().unwrap();
    let assessment = view.assess().unwrap();
    assert_eq!(assessment.producer, ProducerState::Active);
    assert!(matches!(
        &assessment.latest,
        LatestAttempt::Succeeded(attempt) if attempt.sequence == sequence
    ));
    let observation = assessment.observation.as_ref().unwrap();
    assert_eq!(observation.role, ObservationRole::LatestAttempt);
    assert_eq!(observation.attempt.sequence, sequence);
    assert!(observation.attempt.started <= observation.attempt.finished);
    assert!(observation.attempt.finished <= observation.attempt.published);
    assert!(observation.attempt.published <= assessment.assessed_at);
    assert_eq!(
        serde_json::to_value(observation.snapshot).unwrap(),
        *expected
    );
}

fn registration(fixture: &Fixture, number: u32, parent: &str, inputs: &[(&str, &str)]) -> PathBuf {
    let parent = fixture.root.parent().unwrap().join("devices").join(parent);
    let target = parent.join("hwmon").join(format!("hwmon{number}"));
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("name"), "fixture-chip\n").unwrap();
    for (attribute, value) in inputs {
        fs::write(target.join(attribute), value).unwrap();
        let channel = attribute.strip_suffix("_input").unwrap();
        fs::write(target.join(format!("{channel}_label")), "duplicate label").unwrap();
    }
    symlink(&parent, target.join("device")).unwrap();
    symlink(&target, fixture.root.join(format!("hwmon{number}"))).unwrap();
    target
}

#[test]
fn startup_is_distinct_from_a_successful_empty_inventory() {
    let fixture = Fixture::new();
    let (mut owner, reader) = snapshot_cache::new(fixture.root.clone(), AGE_BUDGET).unwrap();
    {
        let mut view = reader.read().unwrap();
        let assessment = view.assess().unwrap();
        assert!(matches!(assessment.latest, LatestAttempt::Empty));
        assert!(assessment.observation.is_none());
        assert_eq!(assessment.producer, ProducerState::Active);
    }
    let expected = direct_json(&fixture.root);
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    assert_latest(&reader, &expected, 1);
    let mut view = reader.read().unwrap();
    let assessment = view.assess().unwrap();
    let observation = assessment.observation.as_ref().unwrap();
    assert!(observation.snapshot.sensors.is_empty());
    assert_eq!(observation.freshness, Freshness::NoSensorObservations);
}

#[test]
fn partial_diagnostics_and_raw_fields_match_the_complete_direct_scan() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "fixture-chip\n");
    for (attribute, value) in [
        ("temp1_input", " 42000\n"),
        ("in0_input", "1152\n"),
        ("fan1_input", "0\n"),
        ("temp2_input", "malformed\n"),
        ("temp3_input", "33000\n"),
        ("power1_input", "17\n"),
    ] {
        fixture.write(&format!("hwmon0/{attribute}"), value);
    }
    fixture.write("hwmon0/temp1_label", " CPU package \n");
    fixture.write("hwmon0/temp2_enable", "malformed");
    fixture.write("hwmon0/temp2_fault", "1");
    fixture.write("hwmon0/temp3_enable", "0");
    fixture.write("hwmon0/temp3_fault", "malformed");
    symlink("missing-input", fixture.root.join("hwmon0/temp4_input")).unwrap();
    fixture.write("hwmon0/temp4_enable", "malformed");
    fs::create_dir(fixture.root.join("hwmon0/fan1_label")).unwrap();

    let expected = direct_json(&fixture.root);
    let (mut owner, reader) = snapshot_cache::new(fixture.root.clone(), AGE_BUDGET).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    assert_latest(&reader, &expected, 1);
    let mut view = reader.read().unwrap();
    let assessment = view.assess().unwrap();
    let snapshot = assessment.observation.as_ref().unwrap().snapshot;
    assert_eq!(snapshot.schema_version, 2);
    assert_eq!(snapshot.sensors.len(), 7);
    let sensor = |attribute| {
        snapshot
            .sensors
            .iter()
            .find(|s| s.attribute == attribute)
            .unwrap()
    };
    assert_eq!(sensor("temp1_input").raw_text.as_deref(), Some("42000"));
    assert_eq!(sensor("temp1_input").raw_value, Some(42000));
    assert_eq!(sensor("temp1_input").value, Some(42.0));
    assert_eq!(sensor("temp1_input").unit, Some("°C"));
    assert_eq!(sensor("temp1_input").label.as_deref(), Some("CPU package"));
    assert_eq!(sensor("in0_input").value, Some(1.152));
    assert_eq!(sensor("fan1_input").raw_value, Some(0));
    assert_eq!(sensor("fan1_input").value, Some(0.0));
    assert_eq!(sensor("power1_input").status, Status::Unsupported);
    assert_eq!(sensor("power1_input").raw_value, Some(17));
    assert_eq!(sensor("temp2_input").status, Status::Invalid);
    assert!(
        sensor("temp2_input")
            .error
            .as_deref()
            .unwrap()
            .starts_with("invalid integer:")
    );
    assert_eq!(sensor("temp3_input").status, Status::Unavailable);
    assert_eq!(sensor("temp3_input").raw_value, Some(33000));
    assert_eq!(
        sensor("temp3_input").error.as_deref(),
        Some("sensor is disabled")
    );
    assert_eq!(sensor("temp4_input").status, Status::Unavailable);
    assert!(
        !sensor("temp4_input")
            .error
            .as_deref()
            .unwrap()
            .contains("enable flag")
    );
    assert!(!snapshot.warnings.is_empty());
}

#[test]
fn an_all_failed_snapshot_and_then_empty_success_replace_healthy_data() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "fixture");
    fixture.write("hwmon0/temp1_input", "42000");
    let (mut owner, reader) = snapshot_cache::new(fixture.root.clone(), AGE_BUDGET).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    fixture.write("hwmon0/temp1_input", "invalid");
    fixture.write("hwmon0/temp1_fault", "1");
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(2));
    assert_latest(&reader, &direct_json(&fixture.root), 2);
    {
        let mut view = reader.read().unwrap();
        let assessment = view.assess().unwrap();
        let snapshot = assessment.observation.as_ref().unwrap().snapshot;
        assert_eq!(snapshot.sensors.len(), 1);
        assert_eq!(snapshot.sensors[0].status, Status::Invalid);
        assert_eq!(snapshot.sensors[0].raw_value, None);
        assert_eq!(snapshot.sensors[0].value, None);
    }
    fs::remove_dir_all(fixture.root.join("hwmon0")).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(3));
    assert_latest(&reader, &direct_json(&fixture.root), 3);
    let mut view = reader.read().unwrap();
    let assessment = view.assess().unwrap();
    let observation = assessment.observation.as_ref().unwrap();
    assert!(observation.snapshot.sensors.is_empty());
    assert!(observation.snapshot.warnings.is_empty());
    assert_eq!(observation.freshness, Freshness::NoSensorObservations);
}

#[test]
fn whole_scan_failures_retain_only_labeled_history_and_recovery_replaces_it() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "before");
    fixture.write("hwmon0/temp1_input", "42000");
    let expected = direct_json(&fixture.root);
    let (mut owner, reader) = snapshot_cache::new(fixture.root.clone(), AGE_BUDGET).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    let original_timing = {
        let mut view = reader.read().unwrap();
        let assessment = view.assess().unwrap();
        let attempt = assessment.observation.as_ref().unwrap().attempt;
        (attempt.started, attempt.finished, attempt.published)
    };
    fs::rename(
        &fixture.root,
        fixture.root.with_file_name("temporarily-missing"),
    )
    .unwrap();
    for sequence in 2..=3 {
        assert_eq!(owner.acquire_once().unwrap(), Acquisition::Failed(sequence));
        let mut view = reader.read().unwrap();
        let assessment = view.assess().unwrap();
        match &assessment.latest {
            LatestAttempt::Failed { attempt, error } => {
                assert_eq!(attempt.sequence, sequence);
                assert_eq!(error.kind, io::ErrorKind::NotFound);
                assert!(!error.message.is_empty());
                assert!(original_timing.2 <= attempt.started);
            }
            _ => panic!("failed acquisition must remain explicit"),
        }
        let historical = assessment.observation.as_ref().unwrap();
        assert_eq!(historical.role, ObservationRole::HistoricalAfterFailure);
        assert_eq!(historical.attempt.sequence, 1);
        assert_eq!(
            (
                historical.attempt.started,
                historical.attempt.finished,
                historical.attempt.published
            ),
            original_timing
        );
        assert_eq!(serde_json::to_value(historical.snapshot).unwrap(), expected);
    }
    fixture.write("hwmon7/name", "recovered");
    fixture.write("hwmon7/temp2_input", "48000");
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(4));
    assert_latest(&reader, &direct_json(&fixture.root), 4);
    let mut view = reader.read().unwrap();
    let assessment = view.assess().unwrap();
    let observation = assessment.observation.as_ref().unwrap();
    assert_eq!(observation.snapshot.sensors.len(), 1);
    assert_eq!(observation.snapshot.sensors[0].attribute, "temp2_input");
    assert_eq!(observation.snapshot.sensors[0].raw_value, Some(48000));
}

#[test]
fn first_failure_has_no_observation_and_can_recover_to_empty_success() {
    let fixture = Fixture::new();
    let missing = fixture.root.join("absent");
    let (mut owner, reader) = snapshot_cache::new(missing.clone(), AGE_BUDGET).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Failed(1));
    {
        let mut view = reader.read().unwrap();
        let assessment = view.assess().unwrap();
        assert!(
            matches!(&assessment.latest, LatestAttempt::Failed { attempt, error }
            if attempt.sequence == 1 && error.kind == io::ErrorKind::NotFound)
        );
        assert!(assessment.observation.is_none());
    }
    fs::create_dir(&missing).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(2));
    assert_latest(&reader, &direct_json(&missing), 2);
}

#[test]
fn collisions_null_candidates_labels_and_renumbering_remain_exact() {
    let fixture = Fixture::new();
    let first = registration(
        &fixture,
        0,
        "shared-controller",
        &[
            ("temp1_input", "1000"),
            ("in0_input", "1152"),
            ("power1_input", "17"),
        ],
    );
    registration(&fixture, 1, "shared-controller", &[("temp1_input", "2000")]);
    registration(&fixture, 2, "other-controller", &[("temp1_input", "3000")]);
    let (mut owner, reader) = snapshot_cache::new(fixture.root.clone(), AGE_BUDGET).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    assert_latest(&reader, &direct_json(&fixture.root), 1);
    let original_candidate = {
        let mut view = reader.read().unwrap();
        let assessment = view.assess().unwrap();
        let snapshot = assessment.observation.as_ref().unwrap().snapshot;
        assert_eq!(snapshot.sensors.len(), 5);
        let colliding: Vec<_> = snapshot
            .sensors
            .iter()
            .filter(|sensor| sensor.identity.status == IdentityStatus::Collision)
            .collect();
        assert_eq!(colliding.len(), 2);
        assert_eq!(
            colliding[0].identity.candidate,
            colliding[1].identity.candidate
        );
        assert!(colliding[0].identity.candidate.is_some());
        let unknown = snapshot
            .sensors
            .iter()
            .find(|sensor| sensor.attribute == "power1_input")
            .unwrap();
        assert_eq!(unknown.identity.candidate, None);
        assert_eq!(unknown.identity.status, IdentityStatus::AmbiguousChannel);
        assert_eq!(unknown.status, Status::Unsupported);
        assert!(
            snapshot
                .sensors
                .iter()
                .all(|sensor| sensor.label.as_deref() == Some("duplicate label"))
        );
        colliding[0].identity.candidate.clone()
    };
    let renamed = first.with_file_name("hwmon20");
    fs::rename(&first, &renamed).unwrap();
    fs::remove_file(fixture.root.join("hwmon0")).unwrap();
    symlink(&renamed, fixture.root.join("hwmon20")).unwrap();
    fs::write(renamed.join("temp1_label"), "renamed display label").unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(2));
    assert_latest(&reader, &direct_json(&fixture.root), 2);
    let mut view = reader.read().unwrap();
    let assessment = view.assess().unwrap();
    let snapshot = assessment.observation.as_ref().unwrap().snapshot;
    assert_eq!(snapshot.sensors.len(), 5);
    let changed = snapshot
        .sensors
        .iter()
        .find(|sensor| sensor.raw_value == Some(1000))
        .unwrap();
    assert_eq!(changed.identity.candidate, original_candidate);
    assert_eq!(changed.identity.status, IdentityStatus::Collision);
    assert_eq!(changed.label.as_deref(), Some("renamed display label"));
    assert_eq!(
        changed.source_path,
        fixture.root.join("hwmon20/temp1_input")
    );
    assert_eq!(changed.device_path, renamed);
}

#[test]
fn read_handles_share_the_published_payload_without_refreshing_missing_sources() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "fixture");
    fixture.write("hwmon0/temp1_input", "42000");
    let (mut owner, reader) = snapshot_cache::new(fixture.root.clone(), AGE_BUDGET).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    let second = reader.clone();
    fs::rename(&fixture.root, fixture.root.with_file_name("source-removed")).unwrap();
    {
        let mut first_view = reader.read().unwrap();
        let mut second_view = second.read().unwrap();
        let first = first_view.assess().unwrap();
        let second = second_view.assess().unwrap();
        let first = first.observation.as_ref().unwrap();
        let second = second.observation.as_ref().unwrap();
        assert!(std::ptr::eq(first.snapshot, second.snapshot));
        assert_eq!(first.attempt.sequence, 1);
        assert_eq!(second.attempt.sequence, 1);
        assert_eq!(first.role, ObservationRole::LatestAttempt);
        assert_eq!(first.snapshot.sensors[0].raw_value, Some(42000));
    }
    // Only the explicit owner call discovers the now-missing source root.
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Failed(2));
    for handle in [&reader, &second] {
        let mut view = handle.read().unwrap();
        let assessment = view.assess().unwrap();
        assert!(
            matches!(&assessment.latest, LatestAttempt::Failed { attempt, .. } if attempt.sequence == 2)
        );
        assert_eq!(
            assessment.observation.as_ref().unwrap().role,
            ObservationRole::HistoricalAfterFailure
        );
    }
}
