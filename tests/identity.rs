mod common;

use common::Fixture;
use siliconwatch::{
    hwmon::{Status, scan},
    identity::IdentityStatus,
};
use std::{collections::BTreeSet, fs, os::unix::fs::symlink, path::PathBuf};

fn registration(
    f: &Fixture,
    index: u32,
    parent: &str,
    provider: &str,
    inputs: &[(&str, &str)],
) -> PathBuf {
    let parent = f.root.parent().unwrap().join("devices").join(parent);
    let directory = parent.join("hwmon").join(format!("hwmon{index}"));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("name"), provider).unwrap();
    for (attribute, value) in inputs {
        fs::write(directory.join(attribute), value).unwrap();
        let channel = attribute.strip_suffix("_input").unwrap();
        fs::write(
            directory.join(format!("{channel}_label")),
            "duplicate label",
        )
        .unwrap();
    }
    symlink(&parent, directory.join("device")).unwrap();
    symlink(&directory, f.root.join(format!("hwmon{index}"))).unwrap();
    directory
}

#[test]
fn renumbering_canonical_directory_and_alias_preserves_candidate() {
    let f = Fixture::new();
    let old = registration(
        &f,
        2,
        "controller-a",
        "fixture",
        &[("temp1_input", "42500")],
    );
    let before = scan(&f.root).unwrap();
    let identity = &before.sensors[0].identity;
    assert_eq!(before.schema_version, 2);
    assert_eq!(identity.schema_version, 1);
    assert_eq!(identity.status, IdentityStatus::BestEffort);
    assert!(identity.candidate.is_some());
    assert_eq!(identity.evidence.provider.as_deref(), Some("fixture"));
    assert_eq!(identity.evidence.channel.as_deref(), Some("temp1_input"));
    assert_eq!(
        identity.evidence.parent,
        Some(fs::canonicalize(old.parent().unwrap().parent().unwrap()).unwrap())
    );

    let renamed = old.with_file_name("hwmon19");
    fs::rename(&old, &renamed).unwrap();
    fs::remove_file(f.root.join("hwmon2")).unwrap();
    symlink(&renamed, f.root.join("hwmon19")).unwrap();
    fs::write(renamed.join("temp1_label"), "renamed label").unwrap();
    fs::write(renamed.join("temp1_input"), "43000").unwrap();
    let after = scan(&f.root).unwrap();
    assert_eq!(after.sensors[0].identity.candidate, identity.candidate);
    assert_eq!(after.sensors[0].identity.status, IdentityStatus::BestEffort);
    assert_ne!(after.sensors[0].device_path, before.sensors[0].device_path);
    assert_ne!(after.sensors[0].source_path, before.sensors[0].source_path);
    assert_eq!(after.sensors[0].label.as_deref(), Some("renamed label"));
    assert_eq!(after.sensors[0].value, Some(43.0));
    fs::remove_file(renamed.join("temp1_label")).unwrap();
    assert_eq!(
        scan(&f.root).unwrap().sensors[0].identity.candidate,
        identity.candidate
    );
}

#[test]
fn duplicate_labels_do_not_conflate_parents_providers_or_channels() {
    let f = Fixture::new();
    let parent = "controller:|[\"a\"]";
    let provider = "fixture:|[\"p\"]";
    registration(
        &f,
        0,
        parent,
        provider,
        &[
            ("temp1_input", "1000"),
            ("fan1_input", "0"),
            ("in0_input", "1200"),
        ],
    );
    registration(&f, 1, "controller-b", provider, &[("temp1_input", "1000")]);
    registration(&f, 2, parent, "other-provider", &[("temp1_input", "1000")]);
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 5);
    let candidates: BTreeSet<_> = snapshot
        .sensors
        .iter()
        .map(|sensor| {
            assert_eq!(sensor.identity.status, IdentityStatus::BestEffort);
            assert_eq!(sensor.label.as_deref(), Some("duplicate label"));
            assert_eq!(sensor.status, Status::Ok);
            let candidate = sensor.identity.candidate.as_ref().unwrap();
            let components: serde_json::Value = serde_json::from_str(candidate).unwrap();
            assert_eq!(components.as_array().unwrap().len(), 6);
            assert_eq!(
                components[2],
                sensor.identity.evidence.provider.as_deref().unwrap()
            );
            assert_eq!(
                components[3],
                sensor
                    .identity
                    .evidence
                    .parent
                    .as_ref()
                    .unwrap()
                    .to_str()
                    .unwrap()
            );
            candidate
        })
        .collect();
    assert_eq!(candidates.len(), 5);
}

#[test]
fn canonical_parent_is_evidence_only_outside_the_scan_root() {
    let f = Fixture::new();
    let directory = registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
    fs::remove_file(directory.join("device")).unwrap();
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(
        snapshot.sensors[0].identity.status,
        IdentityStatus::BestEffort
    );
    assert!(
        snapshot.sensors[0]
            .identity
            .evidence
            .canonical_parent
            .is_some()
    );

    f.write("hwmon1/name", "fixture");
    f.write("hwmon1/temp1_input", "1000");
    let snapshot = scan(&f.root).unwrap();
    let missing = snapshot
        .sensors
        .iter()
        .find(|s| s.source_path.starts_with(f.root.join("hwmon1")))
        .unwrap();
    assert_eq!(
        missing.identity.status,
        IdentityStatus::InsufficientEvidence
    );
    assert_eq!(missing.identity.candidate, None);
    assert_eq!(missing.identity.evidence.parent, None);
    assert_eq!(missing.status, Status::Ok);
}

#[test]
fn nested_alias_inside_scan_root_does_not_manufacture_parent_evidence() {
    let f = Fixture::new();
    f.write("nested/hwmon7/name", "fixture");
    f.write("nested/hwmon7/temp1_input", "1000");
    symlink(f.root.join("nested/hwmon7"), f.root.join("hwmon0")).unwrap();
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 1);
    let sensor = &snapshot.sensors[0];
    assert_eq!(sensor.identity.status, IdentityStatus::InsufficientEvidence);
    assert_eq!(sensor.identity.candidate, None);
    assert_eq!(sensor.identity.evidence.parent, None);
    assert_eq!(sensor.identity.evidence.canonical_parent, None);
    assert_eq!(sensor.status, Status::Ok);
}

#[test]
fn missing_or_unreadable_provider_does_not_use_hwmon_fallback_for_identity() {
    let f = Fixture::new();
    let directory = registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
    fs::remove_file(directory.join("name")).unwrap();
    let missing = scan(&f.root).unwrap();
    assert_eq!(
        missing.sensors[0].identity.status,
        IdentityStatus::InsufficientEvidence
    );
    assert_eq!(missing.sensors[0].identity.candidate, None);
    assert_eq!(missing.sensors[0].identity.evidence.provider, None);
    assert_eq!(missing.sensors[0].status, Status::Ok);
    fs::create_dir(directory.join("name")).unwrap();
    let unreadable = scan(&f.root).unwrap();
    assert_eq!(
        unreadable.sensors[0].identity.status,
        IdentityStatus::UnreadableEvidence
    );
    assert_eq!(unreadable.sensors[0].identity.candidate, None);
    assert!(!unreadable.sensors[0].identity.issues.is_empty());
}

#[test]
fn malformed_and_dangling_device_links_do_not_fall_back_to_canonical_parent() {
    for dangling in [false, true] {
        let f = Fixture::new();
        let directory = registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
        let link = directory.join("device");
        fs::remove_file(&link).unwrap();
        if dangling {
            symlink("missing-parent", &link).unwrap();
        } else {
            fs::create_dir(&link).unwrap();
        }
        let snapshot = scan(&f.root).unwrap();
        assert_eq!(
            snapshot.sensors[0].identity.status,
            IdentityStatus::UnreadableEvidence
        );
        assert_eq!(snapshot.sensors[0].identity.candidate, None);
        assert!(!snapshot.sensors[0].identity.issues.is_empty());
        assert_eq!(snapshot.sensors[0].raw_value, Some(1000));
    }
}

#[test]
fn contradictory_parent_evidence_is_not_resolved_by_preference() {
    let f = Fixture::new();
    let directory = registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
    let other = f.root.parent().unwrap().join("different-parent");
    fs::create_dir(&other).unwrap();
    fs::remove_file(directory.join("device")).unwrap();
    symlink(&other, directory.join("device")).unwrap();
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(
        snapshot.sensors[0].identity.status,
        IdentityStatus::ConflictingEvidence
    );
    assert_eq!(snapshot.sensors[0].identity.candidate, None);
    assert!(!snapshot.sensors[0].identity.issues.is_empty());
    assert_eq!(snapshot.sensors[0].status, Status::Ok);
}

#[test]
fn collisions_mark_every_member_and_ignore_alias_enumeration_order() {
    let f = Fixture::new();
    let first = registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
    let second = registration(&f, 1, "controller-a", "fixture", &[("temp1_input", "2000")]);
    let before = scan(&f.root).unwrap();
    assert_eq!(before.sensors.len(), 2);
    let candidate = before.sensors[0].identity.candidate.clone();
    assert!(candidate.is_some());
    for sensor in &before.sensors {
        assert_eq!(sensor.identity.status, IdentityStatus::Collision);
        assert_eq!(sensor.identity.candidate, candidate);
        assert_eq!(sensor.status, Status::Ok);
    }
    fs::remove_file(f.root.join("hwmon0")).unwrap();
    fs::remove_file(f.root.join("hwmon1")).unwrap();
    symlink(second, f.root.join("hwmon2")).unwrap();
    symlink(first, f.root.join("hwmon30")).unwrap();
    let after = scan(&f.root).unwrap();
    assert_eq!(after.sensors.len(), 2);
    assert_eq!(before.sensors[0].raw_value, Some(1000));
    assert_eq!(after.sensors[0].raw_value, Some(2000));
    for sensor in &after.sensors {
        assert_eq!(sensor.identity.status, IdentityStatus::Collision);
        assert_eq!(sensor.identity.candidate, candidate);
    }
    assert_eq!(
        after
            .sensors
            .iter()
            .map(|s| s.raw_value.unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([1000, 2000])
    );
}

#[test]
fn disjoint_channels_do_not_resolve_duplicate_source_groups() {
    let f = Fixture::new();
    registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
    registration(&f, 1, "controller-a", "fixture", &[("fan1_input", "2000")]);
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 2);
    for sensor in &snapshot.sensors {
        assert_eq!(sensor.identity.status, IdentityStatus::AmbiguousChannel);
        assert!(sensor.identity.candidate.is_some());
        assert!(!sensor.identity.issues.is_empty());
    }
    assert_ne!(
        snapshot.sensors[0].identity.candidate,
        snapshot.sensors[1].identity.candidate
    );
}

#[test]
fn unsupported_only_registration_still_makes_shared_channel_ownership_ambiguous() {
    let f = Fixture::new();
    registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
    registration(
        &f,
        1,
        "controller-a",
        "fixture",
        &[("power1_input", "2000")],
    );
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 2);
    let temperature = snapshot
        .sensors
        .iter()
        .find(|s| s.attribute == "temp1_input")
        .unwrap();
    assert_eq!(
        temperature.identity.status,
        IdentityStatus::AmbiguousChannel
    );
    assert!(temperature.identity.candidate.is_some());
    assert_eq!(temperature.status, Status::Ok);
    let power = snapshot
        .sensors
        .iter()
        .find(|s| s.attribute == "power1_input")
        .unwrap();
    assert_eq!(power.identity.status, IdentityStatus::AmbiguousChannel);
    assert_eq!(power.identity.candidate, None);
    assert_eq!(power.status, Status::Unsupported);
}

#[test]
fn noncanonical_and_unknown_channels_do_not_get_candidates() {
    let f = Fixture::new();
    registration(
        &f,
        0,
        "controller-a",
        "fixture",
        &[
            ("temp01_input", "1000"),
            ("temp0_input", "1000"),
            ("fan0_input", "0"),
            ("in00_input", "1200"),
            ("power1_input", "2000"),
        ],
    );
    let snapshot = scan(&f.root).unwrap();
    assert_eq!(snapshot.sensors.len(), 5);
    for sensor in &snapshot.sensors {
        assert_eq!(sensor.identity.status, IdentityStatus::AmbiguousChannel);
        assert_eq!(sensor.identity.candidate, None);
        assert!(sensor.raw_value.is_some());
    }
}

#[test]
fn denied_anchor_is_explicit_when_permissions_are_enforced() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let directory = registration(&f, 0, "controller-a", "fixture", &[("temp1_input", "1000")]);
    let blocked = f.root.parent().unwrap().join("blocked");
    fs::create_dir_all(blocked.join("anchor")).unwrap();
    let link = directory.join("device");
    fs::remove_file(&link).unwrap();
    symlink(blocked.join("anchor"), &link).unwrap();
    let permissions = fs::metadata(&blocked).unwrap().permissions();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0)).unwrap();
    if fs::canonicalize(&link).is_ok() {
        fs::set_permissions(&blocked, permissions).unwrap();
        eprintln!("Skipping denied-anchor assertion: this user can traverse mode-000 directories.");
        return;
    }
    let result = scan(&f.root);
    fs::set_permissions(&blocked, permissions).unwrap();
    let snapshot = result.unwrap();
    assert_eq!(
        snapshot.sensors[0].identity.status,
        IdentityStatus::UnreadableEvidence
    );
    assert_eq!(snapshot.sensors[0].identity.candidate, None);
    assert_eq!(snapshot.sensors[0].status, Status::Ok);
}
