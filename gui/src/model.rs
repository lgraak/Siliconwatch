//! UI-independent source grouping, freshness and navigation state.
use crate::client::TimedSnapshot;
use crate::preferences::{GroupOverride, Preferences};
use crate::presentation::{reading, supported_unit, visible};
use crate::protocol::{Identity, Sensor};
use crate::statistics::{RowKey, SessionStatistics};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
#[path = "model/clock.rs"]
mod clock;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct GroupKey {
    pub provider: Option<String>,
    pub path: String,
}
impl GroupKey {
    pub fn of(s: &Sensor) -> Self {
        Self {
            provider: s.identity.evidence.provider.clone(),
            path: s.device_path.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum VisibleRow {
    Group(GroupKey),
    Sensor(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct SourceStamp {
    pub instance: String,
    pub rows: Vec<(RowKey, Identity, String)>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum FreshnessState {
    Waiting,
    Empty,
    Recent,
    Stale,
    Unconfirmed,
    Historical,
    Failed,
    Stopped,
    Disconnected,
    InvalidClock,
}
impl FreshnessState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Waiting => "Waiting for first acquisition",
            Self::Empty => "Successful empty inventory · no sensor observations",
            Self::Recent => "Recent at last assessment · physical measurement age unknown",
            Self::Stale => "Stale observation · retained values and session statistics",
            Self::Unconfirmed => "Freshness unconfirmed · age budget exceeded",
            Self::Historical => "Historical observation · session statistics frozen",
            Self::Failed => "Acquisition failed · no successful observation",
            Self::Stopped => "Producer stopped · retained observation is noncurrent",
            Self::Disconnected => "Disconnected · last known observation only",
            Self::InvalidClock => "Freshness unknown after clock/suspend anomaly · restart daemon",
        }
    }
}

#[derive(Default)]
pub struct SensorModel {
    pub current: Option<TimedSnapshot>,
    pub connected: bool,
    pub connection_message: String,
    pub statistics: SessionStatistics,
    pub collapsed: BTreeSet<GroupKey>,
    pub selected: Option<VisibleRow>,
    pub details_sensor: Option<usize>,
    invalid_clock_instance: Option<String>,
    pending_clock_anomaly: bool,
    clock_guard: clock::ClockGuard,
    pub notice: String,
}
impl SensorModel {
    pub fn sensors(&self) -> &[Sensor] {
        self.current
            .as_ref()
            .and_then(|c| c.snapshot.assessment.observation.as_ref())
            .map(|o| o.snapshot.sensors.as_slice())
            .unwrap_or(&[])
    }
    pub fn source_stamp(&self) -> Option<SourceStamp> {
        let current = self.current.as_ref()?;
        let mut rows: Vec<_> = self
            .sensors()
            .iter()
            .map(|s| (RowKey::of(s), s.identity.clone(), s.source_path.clone()))
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
        Some(SourceStamp {
            instance: current.snapshot.instance_id.clone(),
            rows,
        })
    }
    pub fn mark_clock_anomaly(&mut self) {
        if let Some(c) = &self.current {
            self.invalid_clock_instance = Some(c.snapshot.instance_id.clone());
        } else {
            self.pending_clock_anomaly = true;
        }
    }
    /// Sample even when the API worker is blocked. This is conservative local
    /// anomaly detection, not a guarantee of detecting every suspend interval.
    pub fn check_clock(&mut self) -> bool {
        if self.clock_guard.check() {
            self.mark_clock_anomaly();
            true
        } else {
            false
        }
    }
    pub fn disconnect(&mut self, message: String) {
        self.connected = false;
        self.connection_message = message;
    }
    pub fn freshness(&self, now: Instant) -> FreshnessState {
        let Some(current) = &self.current else {
            return FreshnessState::Waiting;
        };
        if !self.connected {
            return FreshnessState::Disconnected;
        }
        if self.invalid_clock_instance.as_deref() == Some(&current.snapshot.instance_id) {
            return FreshnessState::InvalidClock;
        }
        let a = &current.snapshot.assessment;
        if a.producer != "active" || a.lifecycle != "running" {
            return FreshnessState::Stopped;
        }
        let Some(o) = &a.observation else {
            return if a.latest.state == "failed" {
                FreshnessState::Failed
            } else {
                FreshnessState::Waiting
            };
        };
        if o.role != "latest_attempt" || a.latest.state == "failed" {
            return FreshnessState::Historical;
        }
        if o.freshness.state == "no_sensor_observations" {
            return FreshnessState::Empty;
        }
        if o.freshness.state == "stale" {
            return FreshnessState::Stale;
        }
        let Some(elapsed) = now.checked_duration_since(current.request_started) else {
            return FreshnessState::InvalidClock;
        };
        let Some(upper) = o
            .freshness
            .upper_ns
            .and_then(|v| v.checked_add(elapsed.as_nanos()))
        else {
            return FreshnessState::InvalidClock;
        };
        if a.service_current && upper <= a.stale_after_ns {
            FreshnessState::Recent
        } else {
            FreshnessState::Unconfirmed
        }
    }
    pub fn next_expiry(&self, now: Instant) -> Option<Duration> {
        if self.freshness(now) != FreshnessState::Recent {
            return None;
        }
        let c = self.current.as_ref()?;
        let a = &c.snapshot.assessment;
        let upper = a
            .observation
            .as_ref()?
            .freshness
            .upper_ns?
            .checked_add(now.checked_duration_since(c.request_started)?.as_nanos())?;
        let remaining = a.stale_after_ns.checked_sub(upper)?.saturating_add(1);
        Some(Duration::from_nanos(remaining.min(u64::MAX as u128) as u64))
    }
    pub fn accept(&mut self, incoming: TimedSnapshot, now: Instant) -> Result<bool, String> {
        self.check_clock();
        if incoming.response_received < incoming.request_started || now < incoming.response_received
        {
            self.mark_clock_anomaly();
            return Err("Invalid local clock ordering".into());
        }
        let old_stamp = self.source_stamp();
        let selected_key = match self.selected {
            Some(VisibleRow::Sensor(i)) => self.selection_stamp(i),
            _ => None,
        };
        let details_key = self.details_sensor.and_then(|i| self.selection_stamp(i));
        let same_instance = self
            .current
            .as_ref()
            .is_some_and(|c| c.snapshot.instance_id == incoming.snapshot.instance_id);
        if same_instance {
            let old = &self.current.as_ref().unwrap().snapshot.assessment;
            let new = &incoming.snapshot.assessment;
            if new.assessed_at_ns < old.assessed_at_ns
                || new.latest.attempt.as_ref().map(|a| a.sequence)
                    < old.latest.attempt.as_ref().map(|a| a.sequence)
            {
                return Err("Protocol error: same-instance time or sequence regression".into());
            }
            if let (Some(a), Some(b)) = (&old.observation, &new.observation) {
                if a.attempt.sequence == b.attempt.sequence
                    && (a.attempt != b.attempt || a.snapshot != b.snapshot)
                {
                    return Err(
                        "Protocol error: cached publication changed without a new sequence".into(),
                    );
                }
            }
        } else {
            self.invalid_clock_instance = if self.pending_clock_anomaly {
                Some(incoming.snapshot.instance_id.clone())
            } else {
                None
            };
            self.pending_clock_anomaly = false;
            self.clock_guard.reset();
            self.collapsed.clear();
            self.selected = None;
            self.details_sensor = None;
        }
        self.current = Some(incoming);
        self.connected = true;
        self.connection_message.clear();
        let eligible = self.freshness(now) == FreshnessState::Recent;
        if let Some(current) = &self.current {
            if self.statistics.observe(&current.snapshot, eligible) {
                self.notice = "New daemon instance · session statistics reset".into();
            }
        }
        if same_instance {
            if let Some(key) = selected_key {
                self.selected = self.unique_matching_index(&key).map(VisibleRow::Sensor);
            }
            self.details_sensor = details_key.and_then(|key| self.unique_matching_index(&key));
        }
        if old_stamp != self.source_stamp() {
            self.collapsed.clear();
        }
        let groups = self.groups();
        self.collapsed.retain(|g| groups.contains_key(g));
        if let Some(VisibleRow::Group(g)) = &self.selected {
            if !groups.contains_key(g) {
                self.selected = None;
            }
        }
        if self.selected.is_none() {
            self.selected = self.visible_rows().first().cloned();
        }
        Ok(old_stamp != self.source_stamp())
    }
    fn selection_stamp(&self, index: usize) -> Option<(RowKey, Identity, String)> {
        self.sensors()
            .get(index)
            .map(|s| (RowKey::of(s), s.identity.clone(), s.source_path.clone()))
    }
    fn unique_matching_index(&self, stamp: &(RowKey, Identity, String)) -> Option<usize> {
        let index = self.unique_index(&stamp.0)?;
        let sensor = &self.sensors()[index];
        (sensor.identity == stamp.1 && sensor.source_path == stamp.2).then_some(index)
    }
    fn unique_index(&self, key: &RowKey) -> Option<usize> {
        let mut found = self
            .sensors()
            .iter()
            .enumerate()
            .filter(|(_, s)| RowKey::of(s) == *key)
            .map(|(i, _)| i);
        let one = found.next()?;
        found.next().is_none().then_some(one)
    }
    pub fn groups(&self) -> BTreeMap<GroupKey, Vec<usize>> {
        let mut groups: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (i, sensor) in self.sensors().iter().enumerate() {
            groups.entry(GroupKey::of(sensor)).or_default().push(i);
        }
        for rows in groups.values_mut() {
            rows.sort_by_key(|&i| RowKey::of(&self.sensors()[i]));
        }
        groups
    }
    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut result = Vec::new();
        for (group, rows) in self.groups() {
            result.push(VisibleRow::Group(group.clone()));
            if !self.collapsed.contains(&group) {
                result.extend(rows.into_iter().map(VisibleRow::Sensor));
            }
        }
        result
    }
    pub fn selected_index(&self) -> Option<usize> {
        let rows = self.visible_rows();
        rows.iter().position(|r| Some(r) == self.selected.as_ref())
    }
    pub fn select(&mut self, index: usize) {
        self.selected = self.visible_rows().get(index).cloned();
    }
    pub fn toggle(&mut self, group: &GroupKey) {
        if !self.collapsed.remove(group) {
            self.collapsed.insert(group.clone());
            self.selected = Some(VisibleRow::Group(group.clone()));
        }
    }
    pub fn navigate(&mut self, direction: &str) -> Option<GroupKey> {
        let rows = self.visible_rows();
        if rows.is_empty() {
            return None;
        }
        let index = self.selected_index().unwrap_or(0);
        match direction {
            "up" => self.select(index.saturating_sub(1)),
            "down" => self.select((index + 1).min(rows.len() - 1)),
            "home" => self.select(0),
            "end" => self.select(rows.len() - 1),
            "page-up" => self.select(index.saturating_sub(12)),
            "page-down" => self.select((index + 12).min(rows.len() - 1)),
            "left" | "right" => {
                let group = match &rows[index] {
                    VisibleRow::Group(g) => g.clone(),
                    VisibleRow::Sensor(i) => GroupKey::of(&self.sensors()[*i]),
                };
                self.selected = Some(VisibleRow::Group(group.clone()));
                let change = if direction == "left" {
                    self.collapsed.insert(group.clone())
                } else {
                    self.collapsed.remove(&group)
                };
                if change {
                    return Some(group);
                }
            }
            _ => {}
        }
        None
    }
    pub fn group_title(&self, group: &GroupKey) -> String {
        let label = self
            .groups()
            .get(group)
            .and_then(|v| v.first())
            .and_then(|&i| self.sensors().get(i))
            .map(|s| s.device.as_str())
            .unwrap_or("Unknown source");
        visible(&format!(
            "{} · {}",
            group.provider.as_deref().unwrap_or(label),
            group.path.rsplit('/').next().unwrap_or("source")
        ))
    }
    pub fn status(&self, now: Instant) -> String {
        if self.current.is_none() && !self.connection_message.is_empty() {
            return self.connection_message.clone();
        }
        let mut status = self.freshness(now).label().to_owned();
        if !self.connection_message.is_empty() {
            status.push_str(" · ");
            status.push_str(&self.connection_message);
        }
        if let Some(c) = &self.current {
            let a = &c.snapshot.assessment;
            if let Some(error) = &a.latest.error {
                status.push_str(&format!(" · {}", visible(&error.message)));
            }
            if self.freshness(now) != FreshnessState::Empty
                && a.observation
                    .as_ref()
                    .is_some_and(|o| o.snapshot.sensors.is_empty())
            {
                status.push_str(" · successful empty inventory");
            }
        }
        status
    }
    pub fn evidence(&self, index: usize, preferences: &Preferences, now: Instant) -> String {
        let Some(sensor) = self.sensors().get(index) else {
            return "Source is no longer confidently associated. Return to Sensors and select it again.".into();
        };
        let mut text = format!(
            "{}\n\nDisplay values are rounded; canonical/raw values below are retained.\nLabel: {:?}\nRow status: {}\nRaw text (escaped): {:?}\nExact raw integer: {:?}\nCanonical value: {:?} {:?}\nDisplayed value: {}\n\nDevice/source: {}\nDevice path: {}\nAttribute: {}\nSource path: {}\n\nIdentity (original API evidence):\n{:#?}\n\nRow diagnostic: {:?}\n",
            self.status(now),
            sensor.label,
            sensor.status,
            sensor.raw_text,
            sensor.raw_value,
            sensor.value,
            sensor.unit,
            reading(
                sensor.value,
                supported_unit(sensor),
                preferences.appearance.temperature_unit
            ),
            sensor.device,
            sensor.device_path,
            sensor.attribute,
            sensor.source_path,
            sensor.identity,
            sensor.error
        );
        if let Some(c) = &self.current {
            let a = &c.snapshot.assessment;
            text.push_str(&format!("\nLatest acquisition: {:#?}\n", a.latest));
            if let Some(o) = &a.observation {
                text.push_str(&format!("Observation role: {}\nObservation metadata: {:#?}\nFreshness at assessment: {:#?}\nSnapshot warnings: {:#?}\n",o.role,o.attempt,o.freshness,o.snapshot.warnings));
            }
        }
        text
    }
}

pub fn match_group(record: &GroupOverride, sensors: &[Sensor]) -> Result<GroupKey, String> {
    let mut group = None;
    for member in &record.members {
        let matches: Vec<_> = sensors
            .iter()
            .filter(|s| s.identity.candidate.as_deref() == Some(member.candidate_id.as_str()))
            .collect();
        if matches.len() != 1 {
            return Err(if matches.is_empty() {
                "Missing source anchor"
            } else {
                "Ambiguous source anchor"
            }
            .into());
        }
        let sensor = matches[0];
        let e = &sensor.identity.evidence;
        let tuple = serde_json::json!([
            "siliconwatch.hwmon-source",
            1,
            e.provider,
            e.parent,
            e.channel,
            e.measurement
        ])
        .to_string();
        if sensor.identity.schema_version != 1
            || member.identity_schema_version != 1
            || sensor.identity.status != "best_effort"
            || !sensor.identity.issues.is_empty()
            || e.provider.as_ref().is_none_or(String::is_empty)
            || e.parent.as_ref().is_none_or(String::is_empty)
            || e.channel.as_deref() != Some(&sensor.attribute)
            || e.device_link
                .as_ref()
                .is_some_and(|p| Some(p) != e.parent.as_ref())
            || e.canonical_parent
                .as_ref()
                .is_some_and(|p| Some(p) != e.parent.as_ref())
            || tuple != member.candidate_id
            || e.measurement.as_deref() != Some(&member.measurement)
            || supported_unit(sensor) != Some(member.unit.as_str())
        {
            return Err("Conflicting identity evidence or measurement semantics".into());
        }
        let next = GroupKey::of(sensor);
        if sensors.iter().filter(|s| GroupKey::of(s) == next).any(|s| {
            matches!(
                s.identity.status.as_str(),
                "collision" | "ambiguous_channel" | "conflicting_evidence"
            ) || s
                .identity
                .evidence
                .parent
                .as_ref()
                .is_some_and(|p| Some(p) != e.parent.as_ref())
        }) {
            return Err("Conflicting live source-group evidence".into());
        }
        if group.as_ref().is_some_and(|g| g != &next) {
            return Err("Anchors refer to different source registrations".into());
        }
        group = Some(next);
    }
    group.ok_or_else(|| "Missing source anchors".into())
}

#[cfg(test)]
#[path = "model/tests.rs"]
mod tests;
