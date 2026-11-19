//! Bounded aggregates of distinct valid publications seen by this client.
use crate::presentation::supported_unit;
use crate::protocol::{Identity, Sensor, SnapshotResponse};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RowKey {
    pub provider: Option<String>,
    pub device_path: String,
    pub attribute: String,
    pub measurement: Option<String>,
    pub unit: Option<String>,
}
impl RowKey {
    pub fn of(sensor: &Sensor) -> Self {
        Self {
            provider: sensor.identity.evidence.provider.clone(),
            device_path: sensor.device_path.clone(),
            attribute: sensor.attribute.clone(),
            measurement: sensor.identity.evidence.measurement.clone(),
            unit: sensor.unit.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aggregate {
    pub count: u64,
    pub minimum: f64,
    pub maximum: f64,
    pub average: f64,
}
impl Aggregate {
    pub fn first(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self {
            count: 1,
            minimum: value,
            maximum: value,
            average: value,
        })
    }
    pub fn add(&mut self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        let Some(count) = self.count.checked_add(1) else {
            return false;
        };
        let n = count as f64;
        // A convex combination avoids overflow in (value - average) for opposite extremes.
        let average = self.average * (1.0 - 1.0 / n) + value / n;
        if !average.is_finite() {
            return false;
        }
        self.count = count;
        self.average = average;
        self.minimum = self.minimum.min(value);
        self.maximum = self.maximum.max(value);
        true
    }
}
#[derive(Clone, Debug)]
struct Entry {
    identity: Identity,
    source_path: String,
    aggregate: Option<Aggregate>,
    failed: bool,
    ambiguous: bool,
}

#[derive(Default, Debug)]
pub struct SessionStatistics {
    instance: Option<String>,
    watermark: Option<u64>,
    entries: BTreeMap<RowKey, Entry>,
}
impl SessionStatistics {
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }
    pub fn aggregate(&self, sensor: &Sensor) -> Option<&Aggregate> {
        let entry = self.entries.get(&RowKey::of(sensor))?;
        (!entry.failed
            && !entry.ambiguous
            && entry.identity == sensor.identity
            && entry.source_path == sensor.source_path)
            .then_some(entry.aggregate.as_ref())
            .flatten()
    }
    pub fn unavailable(&self, sensor: &Sensor) -> bool {
        self.entries
            .get(&RowKey::of(sensor))
            .is_some_and(|e| e.failed || e.ambiguous)
    }
    pub fn observe(&mut self, response: &SnapshotResponse, eligible: bool) -> bool {
        let new_instance = self.instance.as_deref() != Some(&response.instance_id);
        if new_instance {
            self.instance = Some(response.instance_id.clone());
            self.watermark = None;
            self.entries.clear();
        }
        let Some(observation) = &response.assessment.observation else {
            return new_instance;
        };
        let eligible = eligible
            && response.assessment.service_current
            && response.assessment.lifecycle == "running"
            && response.assessment.producer == "active"
            && response.assessment.latest.state == "succeeded"
            && observation.role == "latest_attempt"
            && observation.freshness.state == "within_budget";
        let seq = observation.attempt.sequence;
        if self.watermark.is_some_and(|old| seq <= old) {
            return new_instance;
        }
        self.watermark = Some(seq);
        let sensors = &observation.snapshot.sensors;
        let mut counts = BTreeMap::new();
        for sensor in sensors {
            *counts.entry(RowKey::of(sensor)).or_insert(0_usize) += 1;
        }
        let live: BTreeSet<_> = counts.keys().cloned().collect();
        self.entries.retain(|key, _| live.contains(key));
        for sensor in sensors {
            let key = RowKey::of(sensor);
            let ambiguous = counts[&key] != 1
                || (key.provider.as_deref() == Some("nvml") && sensor.identity.candidate.is_none())
                || matches!(
                    sensor.identity.status.as_str(),
                    "collision" | "ambiguous_channel" | "conflicting_evidence"
                );
            let entry = self.entries.entry(key).or_insert_with(|| Entry {
                identity: sensor.identity.clone(),
                source_path: sensor.source_path.clone(),
                aggregate: None,
                failed: false,
                ambiguous: false,
            });
            if entry.identity != sensor.identity || entry.source_path != sensor.source_path {
                *entry = Entry {
                    identity: sensor.identity.clone(),
                    source_path: sensor.source_path.clone(),
                    aggregate: None,
                    failed: false,
                    ambiguous: false,
                };
            }
            if ambiguous {
                entry.aggregate = None;
                entry.ambiguous = true;
                continue;
            }
            if entry.ambiguous {
                entry.aggregate = None;
                entry.ambiguous = false;
            }
            if !eligible
                || sensor.status != "ok"
                || matches!(supported_unit(sensor), None | Some("P-state"))
            {
                continue;
            }
            if entry.failed {
                continue;
            }
            if let Some(value) = sensor.value.filter(|v| v.is_finite()) {
                if let Some(aggregate) = &mut entry.aggregate {
                    if !aggregate.add(value) {
                        entry.failed = true;
                        entry.aggregate = None;
                    }
                } else {
                    entry.aggregate = Aggregate::first(value);
                }
            }
        }
        new_instance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mean_min_max_include_valid_zero_and_negative() {
        let mut a = Aggregate::first(-3.0).unwrap();
        assert!(a.add(0.0));
        assert!(a.add(9.0));
        assert_eq!(a.count, 3);
        assert_eq!(a.minimum, -3.0);
        assert_eq!(a.maximum, 9.0);
        assert!((a.average - 2.0).abs() < 1e-12);
    }
    #[test]
    fn opposite_extremes_do_not_overflow_intermediate_difference() {
        let mut a = Aggregate::first(-f64::MAX).unwrap();
        assert!(a.add(f64::MAX));
        assert_eq!(a.average, 0.0);
    }
    #[test]
    fn arithmetic_failure_never_mutates_the_previous_count() {
        let mut a = Aggregate::first(1.0).unwrap();
        assert!(!a.add(f64::NAN));
        assert_eq!(a.count, 1);
        a.count = u64::MAX;
        assert!(!a.add(2.0));
        assert_eq!(a.count, u64::MAX);
    }
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    use crate::protocol::tests::{SECOND, frame, sample_value};
    use crate::protocol::{SnapshotResponse, parse_snapshot};
    fn snapshot(seq: u64, value: f64) -> SnapshotResponse {
        let mut s = parse_snapshot(&frame(&sample_value(seq))).unwrap();
        s.assessment.observation.as_mut().unwrap().snapshot.sensors[0].value = Some(value);
        s
    }
    fn sensor(s: &SnapshotResponse) -> &Sensor {
        &s.assessment.observation.as_ref().unwrap().snapshot.sensors[0]
    }
    #[test]
    fn nvml_memory_statistics_restart_when_query_semantics_change() {
        let mut stats = SessionStatistics::default();
        for (sequence, measurement, value, count) in [
            (1, "nvml_memory_used_v2_bytes", 100.0, 1),
            (2, "nvml_memory_used_v2_bytes", 100.0, 2),
            (3, "nvml_memory_used_legacy_bytes", 150.0, 1),
            (4, "nvml_memory_used_v2_bytes", 100.0, 1),
        ] {
            let mut publication = snapshot(sequence, value);
            let row = &mut publication
                .assessment
                .observation
                .as_mut()
                .unwrap()
                .snapshot
                .sensors[0];
            row.attribute = "memory_used".into();
            row.unit = Some("B".into());
            row.identity.evidence.provider = Some("nvml".into());
            row.identity.evidence.measurement = Some(measurement.into());
            row.identity.candidate = Some(format!("fixture-{measurement}"));

            stats.observe(&publication, true);
            let aggregate = stats.aggregate(sensor(&publication)).unwrap();
            assert_eq!(aggregate.count, count);
            assert_eq!(aggregate.average, value);
        }
    }

    #[test]
    fn publications_deduplicate_and_reset_keeps_the_watermark() {
        let mut stats = SessionStatistics::default();
        let first = snapshot(1, 10.0);
        stats.observe(&first, true);
        stats.observe(&first, true);
        assert_eq!(stats.aggregate(sensor(&first)).unwrap().count, 1);
        stats.clear();
        stats.observe(&first, true);
        assert!(stats.aggregate(sensor(&first)).is_none());
        let second = snapshot(2, 20.0);
        stats.observe(&second, true);
        assert_eq!(stats.aggregate(sensor(&second)).unwrap().count, 1);
        assert_eq!(stats.aggregate(sensor(&second)).unwrap().average, 20.0);
    }
    #[test]
    fn bad_rows_stale_and_historical_observations_never_contribute() {
        let mut stats = SessionStatistics::default();
        let first = snapshot(1, 10.0);
        stats.observe(&first, true);
        for (index, status) in [
            "fault",
            "unsupported",
            "permission_denied",
            "unavailable",
            "invalid",
            "error",
        ]
        .iter()
        .enumerate()
        {
            let mut bad = snapshot(index as u64 + 2, 500.0);
            bad.assessment
                .observation
                .as_mut()
                .unwrap()
                .snapshot
                .sensors[0]
                .status = (*status).into();
            stats.observe(&bad, true);
            assert_eq!(stats.aggregate(sensor(&first)).unwrap().count, 1);
        }
        let mut historical = snapshot(8, 500.0);
        historical.assessment.observation.as_mut().unwrap().role =
            "historical_after_failure".into();
        stats.observe(&historical, true);
        assert_eq!(stats.aggregate(sensor(&first)).unwrap().count, 1);
        let stale = snapshot(9, 500.0);
        stats.observe(&stale, false);
        stats.observe(&stale, true);
        assert_eq!(stats.aggregate(sensor(&first)).unwrap().count, 1);
        let valid = snapshot(10, 30.0);
        stats.observe(&valid, true);
        assert_eq!(stats.aggregate(sensor(&valid)).unwrap().count, 2);
        assert_eq!(stats.aggregate(sensor(&valid)).unwrap().average, 20.0);
    }
    #[test]
    fn restart_and_evidence_change_start_new_source_statistics() {
        let mut stats = SessionStatistics::default();
        let first = snapshot(1, 10.0);
        stats.observe(&first, true);
        let mut changed = snapshot(2, 20.0);
        changed
            .assessment
            .observation
            .as_mut()
            .unwrap()
            .snapshot
            .sensors[0]
            .identity
            .evidence
            .parent = Some("/devices/replacement".into());
        stats.observe(&changed, true);
        assert_eq!(stats.aggregate(sensor(&changed)).unwrap().count, 1);
        assert!(stats.aggregate(sensor(&first)).is_none());
        changed.instance_id = SECOND.into();
        assert!(stats.observe(&changed, true));
        assert_eq!(stats.aggregate(sensor(&changed)).unwrap().count, 1);
    }
    #[test]
    fn disappearance_and_resolved_duplicates_do_not_reuse_previous_aggregates() {
        let mut stats = SessionStatistics::default();
        let first = snapshot(1, 10.0);
        stats.observe(&first, true);
        let mut duplicate = snapshot(2, 20.0);
        let row = sensor(&duplicate).clone();
        duplicate
            .assessment
            .observation
            .as_mut()
            .unwrap()
            .snapshot
            .sensors
            .push(row);
        stats.observe(&duplicate, true);
        assert!(stats.unavailable(sensor(&first)));
        assert!(stats.aggregate(sensor(&first)).is_none());
        let resolved = snapshot(3, 30.0);
        stats.observe(&resolved, true);
        assert_eq!(stats.aggregate(sensor(&resolved)).unwrap().count, 1);
        assert!(!stats.unavailable(sensor(&resolved)));
        let mut gone = snapshot(4, 40.0);
        gone.assessment
            .observation
            .as_mut()
            .unwrap()
            .snapshot
            .sensors
            .clear();
        stats.observe(&gone, false);
        assert!(stats.aggregate(sensor(&resolved)).is_none());
        let returned = snapshot(5, 50.0);
        stats.observe(&returned, true);
        assert_eq!(stats.aggregate(sensor(&returned)).unwrap().count, 1);
    }
    #[test]
    fn arithmetic_failure_remains_unavailable_until_explicit_reset() {
        let mut stats = SessionStatistics::default();
        let first = snapshot(1, 10.0);
        stats.observe(&first, true);
        stats
            .entries
            .get_mut(&RowKey::of(sensor(&first)))
            .unwrap()
            .aggregate
            .as_mut()
            .unwrap()
            .count = u64::MAX;
        let next = snapshot(2, 20.0);
        stats.observe(&next, true);
        assert!(stats.unavailable(sensor(&next)));
        assert!(stats.aggregate(sensor(&next)).is_none());
        stats.observe(&snapshot(3, 30.0), true);
        assert!(stats.unavailable(sensor(&next)));
        stats.clear();
        let fresh = snapshot(4, 40.0);
        stats.observe(&fresh, true);
        assert_eq!(stats.aggregate(sensor(&fresh)).unwrap().count, 1);
    }
}
