use super::*;
use crate::protocol::tests::{SECOND, frame, sample_value};
use crate::protocol::{Sensor, parse_snapshot};
use serde_json::json;
fn timed(sequence: u64, now: Instant) -> TimedSnapshot {
    TimedSnapshot {
        snapshot: parse_snapshot(&frame(&sample_value(sequence))).unwrap(),
        request_started: now - Duration::from_millis(10),
        response_received: now - Duration::from_millis(5),
    }
}
fn sensors(s: &mut TimedSnapshot) -> &mut Vec<Sensor> {
    &mut s
        .snapshot
        .assessment
        .observation
        .as_mut()
        .unwrap()
        .snapshot
        .sensors
}
fn anchor(sensor: &mut Sensor) {
    let i = &mut sensor.identity;
    i.status = "best_effort".into();
    i.issues.clear();
    i.evidence.parent = Some("/devices/fixture".into());
    i.evidence.device_link = i.evidence.parent.clone();
    i.evidence.canonical_parent = i.evidence.parent.clone();
    i.evidence.channel = Some(sensor.attribute.clone());
    i.candidate = Some(
        json!([
            "siliconwatch.hwmon-source",
            1,
            i.evidence.provider,
            i.evidence.parent,
            i.evidence.channel,
            i.evidence.measurement
        ])
        .to_string(),
    );
}
fn override_for(sensors: &[Sensor]) -> GroupOverride {
    serde_json::from_value(json!({"expanded":false,"members":sensors.iter().map(|s|json!({"identity_schema_version":1,"candidate_id":s.identity.candidate,"measurement":s.identity.evidence.measurement,"unit":s.unit})).collect::<Vec<_>>()})).unwrap()
}
#[test]
fn request_duration_and_threshold_equality_control_freshness() {
    let now = Instant::now();
    let mut t = timed(1, now);
    t.request_started = now - Duration::from_nanos(999_999_800);
    t.response_received = now;
    let mut m = SensorModel::default();
    m.accept(t, now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::Recent);
    assert_eq!(m.next_expiry(now), Some(Duration::from_nanos(1)));
    assert_eq!(
        m.freshness(now + Duration::from_nanos(1)),
        FreshnessState::Unconfirmed
    );
    assert_eq!(m.next_expiry(now + Duration::from_nanos(1)), None);
    m.disconnect("fixture lost".into());
    assert_eq!(m.freshness(now), FreshnessState::Disconnected);
}
#[test]
fn empty_inventory_is_distinct_from_waiting_and_initial_failure() {
    let now = Instant::now();
    let mut m = SensorModel::default();
    assert_eq!(m.freshness(now), FreshnessState::Waiting);
    let mut t = timed(1, now);
    sensors(&mut t).clear();
    let a = &mut t.snapshot.assessment;
    a.service_current = false;
    let f = &mut a.observation.as_mut().unwrap().freshness;
    f.state = "no_sensor_observations".into();
    f.lower_ns = None;
    f.upper_ns = None;
    m.accept(t, now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::Empty);
    assert!(!m.status(now).contains("Waiting"));
    let mut failed = timed(2, now);
    failed.snapshot.instance_id = SECOND.into();
    failed.snapshot.assessment.latest.state = "failed".into();
    failed.snapshot.assessment.observation = None;
    failed.snapshot.assessment.service_current = false;
    m.accept(failed, now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::Failed);
}
#[test]
fn stale_and_historical_states_freeze_and_never_gain_a_recent_badge() {
    let now = Instant::now();
    let mut m = SensorModel::default();
    m.accept(timed(1, now), now).unwrap();
    let mut stale = timed(2, now);
    stale
        .snapshot
        .assessment
        .observation
        .as_mut()
        .unwrap()
        .freshness
        .state = "stale".into();
    stale.snapshot.assessment.service_current = false;
    m.accept(stale, now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::Stale);
    assert_eq!(m.statistics.aggregate(&m.sensors()[0]).unwrap().count, 1);
    let mut historical = timed(3, now);
    historical.snapshot.assessment.latest.state = "failed".into();
    historical.snapshot.assessment.service_current = false;
    historical
        .snapshot
        .assessment
        .observation
        .as_mut()
        .unwrap()
        .role = "historical_after_failure".into();
    m.accept(historical, now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::Historical);
    assert_eq!(m.statistics.aggregate(&m.sensors()[0]).unwrap().count, 1);
}
#[test]
fn known_anomaly_latches_across_polls_and_reconnect_until_new_instance() {
    let now = Instant::now();
    let mut m = SensorModel::default();
    m.accept(timed(1, now), now).unwrap();
    m.mark_clock_anomaly();
    m.accept(timed(2, now), now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::InvalidClock);
    m.disconnect("reconnect".into());
    m.accept(timed(3, now), now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::InvalidClock);
    assert_eq!(m.statistics.aggregate(&m.sensors()[0]).unwrap().count, 1);
    let mut new = timed(1, now);
    new.snapshot.instance_id = SECOND.into();
    m.accept(new, now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::Recent);
    assert_eq!(m.statistics.aggregate(&m.sensors()[0]).unwrap().count, 1);
}
#[test]
fn anomaly_before_first_response_does_not_trust_an_unknown_old_cache() {
    let now = Instant::now();
    let mut m = SensorModel::default();
    m.mark_clock_anomaly();
    m.accept(timed(1, now), now).unwrap();
    assert_eq!(m.freshness(now), FreshnessState::InvalidClock);
    assert!(m.statistics.aggregate(&m.sensors()[0]).is_none());
}
#[test]
fn same_sequence_content_change_and_regression_are_rejected() {
    let now = Instant::now();
    let mut m = SensorModel::default();
    m.accept(timed(2, now), now).unwrap();
    assert!(m.accept(timed(1, now), now).is_err());
    let mut changed = timed(2, now);
    sensors(&mut changed)[0].value = Some(999.0);
    assert!(m.accept(changed, now).is_err());
    assert_eq!(m.sensors()[0].value, Some(42.0));
}
#[test]
fn reordering_preserves_unique_selection_but_changed_identity_does_not() {
    let now = Instant::now();
    let mut first = timed(1, now);
    let mut second = sensors(&mut first)[0].clone();
    second.attribute = "temp2_input".into();
    second.identity.evidence.channel = Some(second.attribute.clone());
    second.source_path = "/fixture/hwmon0/temp2_input".into();
    sensors(&mut first).push(second);
    let mut m = SensorModel::default();
    m.accept(first.clone(), now).unwrap();
    m.selected = Some(VisibleRow::Sensor(1));
    m.details_sensor = Some(1);
    let mut next = timed(2, now);
    *sensors(&mut next) = sensors(&mut first).clone();
    sensors(&mut next).reverse();
    m.accept(next, now).unwrap();
    assert_eq!(m.selected, Some(VisibleRow::Sensor(0)));
    assert_eq!(m.details_sensor, Some(0));
    let group = m.groups().keys().next().unwrap().clone();
    m.toggle(&group);
    let mut changed = timed(3, now);
    *sensors(&mut changed) = m.sensors().to_vec();
    sensors(&mut changed)[0].identity.evidence.parent = Some("/devices/replacement".into());
    m.accept(changed, now).unwrap();
    assert!(m.details_sensor.is_none());
    assert!(m.collapsed.is_empty());
}
#[test]
fn duplicate_rows_do_not_retarget_details_and_provider_names_do_not_merge_sources() {
    let now = Instant::now();
    let mut m = SensorModel::default();
    m.accept(timed(1, now), now).unwrap();
    m.details_sensor = Some(0);
    let mut next = timed(2, now);
    let duplicate = sensors(&mut next)[0].clone();
    sensors(&mut next).push(duplicate);
    m.accept(next, now).unwrap();
    assert!(m.details_sensor.is_none());
    let mut next = timed(3, now);
    let mut separate = sensors(&mut next)[0].clone();
    separate.device_path = "/fixture/hwmon1".into();
    sensors(&mut next).push(separate);
    m.accept(next, now).unwrap();
    assert_eq!(m.groups().len(), 2);
}
#[test]
fn keyboard_navigation_preserves_group_focus_and_reaches_long_table_edges() {
    let now = Instant::now();
    let mut m = SensorModel::default();
    m.accept(timed(1, now), now).unwrap();
    assert_eq!(m.selected_index(), Some(0));
    m.navigate("down");
    assert_eq!(m.selected, Some(VisibleRow::Sensor(0)));
    let group = m.navigate("left").unwrap();
    assert!(m.collapsed.contains(&group));
    assert_eq!(m.visible_rows().len(), 1);
    m.navigate("right");
    assert!(m.collapsed.is_empty());
    assert_eq!(m.visible_rows().len(), 2);
    m.navigate("end");
    assert_eq!(m.selected, Some(VisibleRow::Sensor(0)));
    m.navigate("home");
    assert_eq!(m.selected_index(), Some(0));
    m.navigate("page-down");
    assert_eq!(m.selected_index(), Some(1));
    m.navigate("page-up");
    assert_eq!(m.selected_index(), Some(0));
}
#[test]
fn group_mapping_rejects_ambiguous_conflicting_and_mixed_registration_anchors() {
    let mut t = timed(1, Instant::now());
    let mut first = sensors(&mut t)[0].clone();
    anchor(&mut first);
    let record = override_for(&[first.clone()]);
    assert_eq!(
        match_group(&record, &[first.clone()]).unwrap(),
        GroupKey::of(&first)
    );
    assert!(match_group(&record, &[]).is_err());
    assert!(match_group(&record, &[first.clone(), first.clone()]).is_err());
    let mut conflicting = first.clone();
    conflicting.identity.evidence.device_link = Some("/devices/other".into());
    assert!(match_group(&record, &[conflicting]).is_err());
    let mut second = first.clone();
    second.attribute = "temp2_input".into();
    second.device_path = "/fixture/hwmon1".into();
    anchor(&mut second);
    let mixed = override_for(&[first.clone(), second.clone()]);
    assert!(match_group(&mixed, &[first.clone(), second]).is_err());
    let mut sibling = first.clone();
    sibling.attribute = "temp2_input".into();
    sibling.identity.candidate = None;
    sibling.identity.status = "ambiguous_channel".into();
    assert!(match_group(&record, &[first, sibling]).is_err());
}
