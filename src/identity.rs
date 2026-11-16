//! Best-effort hwmon source identity, independent of reading quality.

use crate::hwmon::Sensor;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityStatus {
    BestEffort,
    InsufficientEvidence,
    UnreadableEvidence,
    ConflictingEvidence,
    AmbiguousChannel,
    Collision,
}

impl IdentityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BestEffort => "best_effort",
            Self::InsufficientEvidence => "insufficient_evidence",
            Self::UnreadableEvidence => "unreadable_evidence",
            Self::ConflictingEvidence => "conflicting_evidence",
            Self::AmbiguousChannel => "ambiguous_channel",
            Self::Collision => "collision",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Evidence {
    pub provider: Option<String>,
    pub parent: Option<PathBuf>,
    pub device_link: Option<PathBuf>,
    pub canonical_parent: Option<PathBuf>,
    pub channel: Option<String>,
    pub measurement: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceIdentity {
    pub schema_version: u8,
    /// A candidate is never proof of physical identity; always inspect status.
    pub candidate: Option<String>,
    pub status: IdentityStatus,
    pub evidence: Evidence,
    pub issues: Vec<String>,
}

impl SourceIdentity {
    pub(crate) fn unresolved() -> Self {
        Self {
            schema_version: 1,
            candidate: None,
            status: IdentityStatus::InsufficientEvidence,
            evidence: Evidence::default(),
            issues: Vec::new(),
        }
    }

    pub(crate) fn for_channel(&self, attribute: &str) -> Self {
        let mut identity = self.clone();
        let semantics = channel_semantics(attribute);
        identity.evidence.channel = Some(attribute.to_owned());
        identity.evidence.measurement = semantics;
        if semantics.is_none() {
            if identity.issues.is_empty() {
                identity.status = IdentityStatus::AmbiguousChannel;
            }
            identity
                .issues
                .push("unknown or noncanonical input channel semantics".into());
        }
        if identity.issues.is_empty() {
            if let (Some(provider), Some(parent), Some(measurement)) = (
                identity.evidence.provider.as_deref(),
                identity.evidence.parent.as_deref().and_then(Path::to_str),
                semantics,
            ) {
                // A fixed JSON tuple avoids delimiter collisions and unstable hashes.
                identity.candidate = Some(
                    serde_json::json!([
                        "siliconwatch.hwmon-source",
                        1,
                        provider,
                        parent,
                        attribute,
                        measurement
                    ])
                    .to_string(),
                );
                identity.status = IdentityStatus::BestEffort;
            }
        }
        identity
    }

    fn problem(&mut self, status: IdentityStatus, message: impl Into<String>) {
        if self.issues.is_empty() {
            self.status = status;
        }
        self.issues.push(message.into());
    }
}

fn numbered_hwmon(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        n.strip_prefix("hwmon")
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Recognize two bounded layouts, not arbitrary ancestor positions.
fn structural_parent(resolved: &Path, root: Option<&Path>) -> Option<PathBuf> {
    let parent = resolved.parent()?;
    if !numbered_hwmon(resolved) || root.is_none_or(|r| resolved.starts_with(r)) {
        return None;
    }
    if parent.file_name().is_some_and(|n| n == "hwmon") {
        parent.parent().map(Path::to_path_buf)
    } else {
        Some(parent.to_path_buf())
    }
}

pub(crate) fn for_device(
    path: &Path,
    root: Option<&Path>,
    name: &io::Result<String>,
) -> SourceIdentity {
    let mut identity = SourceIdentity::unresolved();
    match name {
        Ok(text) if !text.trim().is_empty() && !text.trim().chars().any(char::is_control) => {
            identity.evidence.provider = Some(text.trim().to_owned());
        }
        Ok(_) => identity.problem(
            IdentityStatus::InsufficientEvidence,
            "empty or invalid provider name",
        ),
        Err(error) => identity.problem(
            if error.kind() == io::ErrorKind::NotFound {
                IdentityStatus::InsufficientEvidence
            } else {
                IdentityStatus::UnreadableEvidence
            },
            format!("cannot read provider name: {error}"),
        ),
    }
    match fs::canonicalize(path) {
        Ok(resolved) => identity.evidence.canonical_parent = structural_parent(&resolved, root),
        Err(error) => identity.problem(
            IdentityStatus::UnreadableEvidence,
            format!("cannot resolve hwmon directory: {error}"),
        ),
    }
    let link = path.join("device");
    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            match fs::canonicalize(&link).and_then(|target| {
                if fs::metadata(&target)?.is_dir() {
                    Ok(target)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "device target is not a directory",
                    ))
                }
            }) {
                Ok(target) => identity.evidence.device_link = Some(target),
                Err(error) => identity.problem(
                    IdentityStatus::UnreadableEvidence,
                    format!("cannot resolve device link: {error}"),
                ),
            }
        }
        Ok(_) => identity.problem(
            IdentityStatus::UnreadableEvidence,
            "device attribute is not a symlink",
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => identity.problem(
            IdentityStatus::UnreadableEvidence,
            format!("cannot inspect device link: {error}"),
        ),
    }
    let conflict = matches!(
        (&identity.evidence.device_link, &identity.evidence.canonical_parent),
        (Some(link), Some(parent)) if link != parent
    );
    if conflict {
        identity.problem(
            IdentityStatus::ConflictingEvidence,
            "device link disagrees with canonical parent evidence",
        );
        return identity;
    }
    identity.evidence.parent = identity
        .evidence
        .device_link
        .clone()
        .or_else(|| identity.evidence.canonical_parent.clone());
    match identity.evidence.parent.as_deref() {
        None => identity.problem(
            IdentityStatus::InsufficientEvidence,
            "no supported parent-device evidence",
        ),
        Some(parent) if parent.to_str().is_none() => identity.problem(
            IdentityStatus::InsufficientEvidence,
            "parent path is not UTF-8; no lossy identity is generated",
        ),
        Some(parent) if root.is_some_and(|r| parent.starts_with(r)) => identity.problem(
            IdentityStatus::InsufficientEvidence,
            "scan root or its descendants are not parent-device evidence",
        ),
        Some(parent)
            if parent
                .components()
                .any(|c| numbered_hwmon(Path::new(c.as_os_str()))) =>
        {
            identity.problem(
                IdentityStatus::InsufficientEvidence,
                "parent evidence still contains a volatile hwmon index",
            );
        }
        Some(_) => {}
    }
    identity
}

fn channel_semantics(attribute: &str) -> Option<&'static str> {
    let channel = attribute.strip_suffix("_input")?;
    for (prefix, zero_allowed, semantics) in [
        ("temp", false, "temperature_input_millidegree_celsius"),
        ("fan", false, "fan_speed_input_rpm"),
        ("in", true, "voltage_input_millivolt"),
    ] {
        if let Some(digits) = channel.strip_prefix(prefix) {
            if !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (!digits.starts_with('0') || (zero_allowed && digits == "0"))
            {
                return Some(semantics);
            }
        }
    }
    None
}

/// Resolve ambiguity across the full snapshot without choosing an order-dependent winner.
pub(crate) fn mark_collisions(sensors: &mut [Sensor]) {
    let mut candidates = BTreeMap::<String, usize>::new();
    let mut registrations = BTreeMap::<(String, PathBuf), BTreeSet<PathBuf>>::new();
    for sensor in sensors.iter() {
        if let Some(candidate) = &sensor.identity.candidate {
            *candidates.entry(candidate.clone()).or_default() += 1;
        }
        let evidence = &sensor.identity.evidence;
        if let (Some(provider), Some(parent)) = (&evidence.provider, &evidence.parent) {
            registrations
                .entry((provider.clone(), parent.clone()))
                .or_default()
                .insert(sensor.device_path.clone());
        }
    }
    for sensor in sensors {
        let identity = &mut sensor.identity;
        if let Some(candidate) = &identity.candidate {
            if candidates.get(candidate).copied().unwrap_or(0) > 1 {
                identity.problem(
                    IdentityStatus::Collision,
                    "duplicate identity candidate; every source is retained",
                );
            }
            if let (Some(provider), Some(parent)) =
                (&identity.evidence.provider, &identity.evidence.parent)
            {
                if registrations
                    .get(&(provider.clone(), parent.clone()))
                    .is_some_and(|sources| sources.len() > 1)
                {
                    identity.problem(IdentityStatus::AmbiguousChannel, "multiple hwmon registrations share parent and provider; channel ownership is ambiguous");
                }
            }
        }
    }
}
