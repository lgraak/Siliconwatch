//! Versioned, bounded JSON records. This module never owns acquisition or socket I/O.

use crate::hwmon::Snapshot;
use crate::snapshot_cache::{
    AcquisitionError, Assessment, AttemptMetadata, Freshness, LatestAttempt, Observation,
    ObservationRole, ProducerState,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(crate) const REQUEST_BYTES: usize = 4096;
pub(crate) const RESPONSE_BYTES: usize = 1_048_576;
pub(crate) const REQUEST_DEADLINE_MS: u64 = 1000;
pub(crate) const RESPONSE_DEADLINE_MS: u64 = 2000;
pub(crate) const MAX_SENSORS: usize = 4096;
pub(crate) const MAX_WARNINGS: usize = 1024;
pub(crate) const MAX_IDENTITY_ISSUES_PER_SENSOR: usize = 64;
pub(crate) const MAX_TEXT_BYTES: usize = 16_384;
pub(crate) const MAX_TOTAL_TEXT_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
    GetInfo,
    GetSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApiError {
    InvalidRequest,
    UnsupportedVersion,
    UnknownMethod,
    RequestTooLarge,
    RequestTimeout,
    CacheBusy,
    ResponseTooLarge,
    SerializationFailed,
    ServiceUnavailable,
    Cancelled,
    Deadline,
}

pub(crate) struct Budget<'a> {
    pub(crate) deadline: Instant,
    pub(crate) cancelled: &'a AtomicBool,
}

impl<'a> Budget<'a> {
    pub(crate) fn new(deadline: Instant, cancelled: &'a AtomicBool) -> Self {
        Self {
            deadline,
            cancelled,
        }
    }

    pub(crate) fn check(&self) -> Result<(), ApiError> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(ApiError::Cancelled)
        } else if Instant::now() >= self.deadline {
            Err(ApiError::Deadline)
        } else {
            Ok(())
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    api_version: u64,
    method: String,
}

/// Accept one complete frame including LF. Framing/transport enforces its own
/// accumulation limit before calling here; repeat the limit at this boundary.
pub(crate) fn parse_request(frame: &[u8]) -> Result<Method, ApiError> {
    if frame.len() > REQUEST_BYTES {
        return Err(ApiError::RequestTooLarge);
    }
    let content = frame.strip_suffix(b"\n").ok_or(ApiError::InvalidRequest)?;
    if content.contains(&b'\n') {
        return Err(ApiError::InvalidRequest);
    }
    let request: Request = serde_json::from_slice(content).map_err(|_| ApiError::InvalidRequest)?;
    if request.api_version != 1 {
        return Err(ApiError::UnsupportedVersion);
    }
    match request.method.as_str() {
        "get_info" => Ok(Method::GetInfo),
        "get_snapshot" => Ok(Method::GetSnapshot),
        _ => Err(ApiError::UnknownMethod),
    }
}

#[derive(Serialize)]
struct Response<'a, T> {
    api_version: u8,
    instance_id: &'a str,
    ok: bool,
    result: T,
}

#[derive(Serialize)]
struct Limits {
    request_bytes: usize,
    response_bytes: usize,
    active_clients: u8,
    request_deadline_ms: u64,
    response_deadline_ms: u64,
    max_sensors: usize,
    max_warnings: usize,
    max_identity_issues_per_sensor: usize,
    max_text_bytes: usize,
    max_total_text_bytes: usize,
}

#[derive(Serialize)]
struct Info {
    supported_versions: [u8; 1],
    methods: [&'static str; 2],
    snapshot_schema_version: u8,
    identity_schema_version: u8,
    read_only: bool,
    limits: Limits,
}

pub(crate) fn info(instance: &str, budget: &Budget<'_>) -> Result<Vec<u8>, ApiError> {
    validate_instance(instance)?;
    encode(
        &Response {
            api_version: 1,
            instance_id: instance,
            ok: true,
            result: Info {
                supported_versions: [1],
                methods: ["get_info", "get_snapshot"],
                snapshot_schema_version: 2,
                identity_schema_version: 1,
                read_only: true,
                limits: Limits {
                    request_bytes: REQUEST_BYTES,
                    response_bytes: RESPONSE_BYTES,
                    active_clients: 1,
                    request_deadline_ms: REQUEST_DEADLINE_MS,
                    response_deadline_ms: RESPONSE_DEADLINE_MS,
                    max_sensors: MAX_SENSORS,
                    max_warnings: MAX_WARNINGS,
                    max_identity_issues_per_sensor: MAX_IDENTITY_ISSUES_PER_SENSOR,
                    max_text_bytes: MAX_TEXT_BYTES,
                    max_total_text_bytes: MAX_TOTAL_TEXT_BYTES,
                },
            },
        },
        budget,
    )
}

impl ApiError {
    fn description(self) -> (&'static str, &'static str, bool) {
        match self {
            Self::InvalidRequest => ("invalid_request", "Invalid request", false),
            Self::UnsupportedVersion => ("unsupported_version", "Unsupported API version", false),
            Self::UnknownMethod => ("unknown_method", "Unknown method", false),
            Self::RequestTooLarge => ("request_too_large", "Request exceeds limit", false),
            Self::RequestTimeout => ("request_timeout", "Request deadline expired", true),
            Self::CacheBusy => ("cache_busy", "Snapshot temporarily busy", true),
            Self::ResponseTooLarge => ("response_too_large", "Response exceeds limit", false),
            Self::SerializationFailed => (
                "serialization_failed",
                "Snapshot cannot be represented",
                false,
            ),
            Self::ServiceUnavailable | Self::Cancelled | Self::Deadline => {
                ("service_unavailable", "Service unavailable", false)
            }
        }
    }
}

/// All interpolated text is fixed except the validated public instance token.
/// Cancelled/Deadline are internal: callers close without trying to transmit.
pub(crate) fn error(instance: &str, code: ApiError) -> Vec<u8> {
    assert!(
        validate_instance(instance).is_ok(),
        "invalid instance token"
    );
    let (name, message, retryable) = code.description();
    let versions = if code == ApiError::UnsupportedVersion {
        ",\"supported_versions\":[1]"
    } else {
        ""
    };
    format!(
        "{{\"api_version\":1,\"instance_id\":\"{instance}\",\"ok\":false,\"error\":{{\"code\":\"{name}\",\"message\":\"{message}\",\"retryable\":{retryable}{versions}}}}}\n"
    )
    .into_bytes()
}

fn validate_instance(instance: &str) -> Result<(), ApiError> {
    if instance.len() == 32
        && instance
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ApiError::ServiceUnavailable)
    }
}

#[derive(Serialize)]
struct Attempt<'a> {
    sequence: String,
    started_ns: String,
    finished_ns: String,
    published_ns: String,
    wall_start_unix_ms: Option<String>,
    wall_time_error: Option<&'a str>,
}

impl<'a> From<&'a AttemptMetadata> for Attempt<'a> {
    fn from(value: &'a AttemptMetadata) -> Self {
        Self {
            sequence: value.sequence.to_string(),
            started_ns: value.started.as_nanos().to_string(),
            finished_ns: value.finished.as_nanos().to_string(),
            published_ns: value.published.as_nanos().to_string(),
            wall_start_unix_ms: value.wall_start_unix_ms.map(|time| time.to_string()),
            wall_time_error: value.wall_time_error.as_deref(),
        }
    }
}

#[derive(Serialize)]
struct AcquisitionFault<'a> {
    kind: String,
    raw_os_error: Option<i32>,
    message: &'a str,
}

impl<'a> From<&'a AcquisitionError> for AcquisitionFault<'a> {
    fn from(value: &'a AcquisitionError) -> Self {
        Self {
            kind: format!("{:?}", value.kind),
            raw_os_error: value.raw_os_error,
            message: &value.message,
        }
    }
}

#[derive(Serialize)]
struct Latest<'a> {
    state: &'static str,
    attempt: Option<Attempt<'a>>,
    error: Option<AcquisitionFault<'a>>,
}

impl<'a> From<LatestAttempt<'a>> for Latest<'a> {
    fn from(value: LatestAttempt<'a>) -> Self {
        match value {
            LatestAttempt::Empty => Self {
                state: "empty",
                attempt: None,
                error: None,
            },
            LatestAttempt::Succeeded(attempt) => Self {
                state: "succeeded",
                attempt: Some(attempt.into()),
                error: None,
            },
            LatestAttempt::Failed { attempt, error } => Self {
                state: "failed",
                attempt: Some(attempt.into()),
                error: Some(error.into()),
            },
        }
    }
}

#[derive(Serialize)]
struct WireFreshness {
    state: &'static str,
    lower_ns: Option<String>,
    upper_ns: Option<String>,
}

impl From<Freshness> for WireFreshness {
    fn from(value: Freshness) -> Self {
        let (state, age) = match value {
            Freshness::NoSensorObservations => ("no_sensor_observations", None),
            Freshness::WithinBudget(age) => ("within_budget", Some(age)),
            Freshness::Stale(age) => ("stale", Some(age)),
        };
        Self {
            state,
            lower_ns: age.map(|age| age.lower.as_nanos().to_string()),
            upper_ns: age.map(|age| age.upper.as_nanos().to_string()),
        }
    }
}

#[derive(Serialize)]
struct WireObservation<'a> {
    attempt: Attempt<'a>,
    role: &'static str,
    freshness: WireFreshness,
    snapshot: &'a Snapshot,
}

impl<'a> From<Observation<'a>> for WireObservation<'a> {
    fn from(value: Observation<'a>) -> Self {
        Self {
            attempt: value.attempt.into(),
            role: match value.role {
                ObservationRole::LatestAttempt => "latest_attempt",
                ObservationRole::HistoricalAfterFailure => "historical_after_failure",
                ObservationRole::HistoricalAfterStop => "historical_after_stop",
            },
            freshness: value.freshness.into(),
            snapshot: value.snapshot,
        }
    }
}

#[derive(Serialize)]
struct WireAssessment<'a> {
    lifecycle: &'static str,
    assessed_at_ns: String,
    stale_after_ns: String,
    producer: &'static str,
    service_current: bool,
    physical_age_known: bool,
    latest: Latest<'a>,
    observation: Option<WireObservation<'a>>,
}

pub(crate) fn snapshot(
    instance: &str,
    lifecycle: &'static str,
    assessment: Assessment<'_>,
    stale_after: Duration,
    budget: &Budget<'_>,
) -> Result<Vec<u8>, ApiError> {
    validate_instance(instance)?;
    if !matches!(lifecycle, "starting" | "running" | "stopping" | "stopped") {
        return Err(ApiError::ServiceUnavailable);
    }
    preflight(&assessment, budget)?;
    let current = lifecycle == "running"
        && assessment.producer == ProducerState::Active
        && matches!(assessment.latest, LatestAttempt::Succeeded(_))
        && assessment.observation.as_ref().is_some_and(|observation| {
            observation.role == ObservationRole::LatestAttempt
                && matches!(observation.freshness, Freshness::WithinBudget(_))
        });
    let result = WireAssessment {
        lifecycle,
        assessed_at_ns: assessment.assessed_at.as_nanos().to_string(),
        stale_after_ns: stale_after.as_nanos().to_string(),
        producer: match assessment.producer {
            ProducerState::Active => "active",
            ProducerState::Stopped => "stopped",
        },
        service_current: current,
        physical_age_known: assessment.physical_age_known,
        latest: assessment.latest.into(),
        observation: assessment.observation.map(Into::into),
    };
    encode(
        &Response {
            api_version: 1,
            instance_id: instance,
            ok: true,
            result,
        },
        budget,
    )
}

struct Preflight<'a, 'b> {
    total: usize,
    budget: &'a Budget<'b>,
}

impl Preflight<'_, '_> {
    fn length(&mut self, length: usize) -> Result<(), ApiError> {
        self.budget.check()?;
        if length > MAX_TEXT_BYTES {
            return Err(ApiError::ResponseTooLarge);
        }
        self.total = self
            .total
            .checked_add(length)
            .filter(|total| *total <= MAX_TOTAL_TEXT_BYTES)
            .ok_or(ApiError::ResponseTooLarge)?;
        Ok(())
    }

    fn text(&mut self, value: &str) -> Result<(), ApiError> {
        self.length(value.len())
    }

    fn optional_text(&mut self, value: Option<&str>) -> Result<(), ApiError> {
        if let Some(value) = value {
            self.text(value)?;
        }
        Ok(())
    }

    fn path(&mut self, value: &Path) -> Result<(), ApiError> {
        self.length(value.as_os_str().as_bytes().len())?;
        if value.to_str().is_none() {
            return Err(ApiError::SerializationFailed);
        }
        Ok(())
    }

    fn optional_path(&mut self, value: Option<&Path>) -> Result<(), ApiError> {
        if let Some(value) = value {
            self.path(value)?;
        }
        Ok(())
    }

    fn attempt(&mut self, value: &AttemptMetadata) -> Result<(), ApiError> {
        self.optional_text(value.wall_time_error.as_deref())
    }

    fn snapshot(&mut self, value: &Snapshot) -> Result<(), ApiError> {
        self.budget.check()?;
        if value.sensors.len() > MAX_SENSORS || value.warnings.len() > MAX_WARNINGS {
            return Err(ApiError::ResponseTooLarge);
        }
        self.path(&value.root)?;
        for sensor in &value.sensors {
            self.budget.check()?;
            if sensor.identity.issues.len() > MAX_IDENTITY_ISSUES_PER_SENSOR {
                return Err(ApiError::ResponseTooLarge);
            }
            self.text(&sensor.device)?;
            self.path(&sensor.device_path)?;
            self.text(&sensor.attribute)?;
            self.optional_text(sensor.label.as_deref())?;
            self.path(&sensor.source_path)?;
            self.optional_text(sensor.raw_text.as_deref())?;
            self.optional_text(sensor.unit)?;
            self.optional_text(sensor.error.as_deref())?;
            if sensor.value.is_some_and(|value| !value.is_finite()) {
                return Err(ApiError::SerializationFailed);
            }
            let identity = &sensor.identity;
            self.optional_text(identity.candidate.as_deref())?;
            let evidence = &identity.evidence;
            self.optional_text(evidence.provider.as_deref())?;
            self.optional_path(evidence.parent.as_deref())?;
            self.optional_path(evidence.device_link.as_deref())?;
            self.optional_path(evidence.canonical_parent.as_deref())?;
            self.optional_text(evidence.channel.as_deref())?;
            self.optional_text(evidence.measurement)?;
            for issue in &identity.issues {
                self.text(issue)?;
            }
        }
        for warning in &value.warnings {
            self.path(&warning.path)?;
            self.text(&warning.message)?;
        }
        Ok(())
    }
}

fn preflight(assessment: &Assessment<'_>, budget: &Budget<'_>) -> Result<(), ApiError> {
    budget.check()?;
    let mut check = Preflight { total: 0, budget };
    match &assessment.latest {
        LatestAttempt::Empty => {}
        LatestAttempt::Succeeded(attempt) => check.attempt(attempt)?,
        LatestAttempt::Failed { attempt, error } => {
            check.attempt(attempt)?;
            check.text(&error.message)?;
        }
    }
    if let Some(observation) = &assessment.observation {
        check.attempt(observation.attempt)?;
        check.snapshot(observation.snapshot)?;
    }
    budget.check()
}

struct Capped<'a, 'b> {
    bytes: Vec<u8>,
    failure: Option<ApiError>,
    budget: &'a Budget<'b>,
}

impl Write for Capped<'_, '_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let checked = self.budget.check().and_then(|()| {
            self.bytes
                .len()
                .checked_add(bytes.len())
                .filter(|length| *length < RESPONSE_BYTES)
                .map(|_| ())
                .ok_or(ApiError::ResponseTooLarge)
        });
        if let Err(error) = checked {
            self.failure = Some(error);
            return Err(io::Error::other("API response encoding stopped"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.budget.check().map_err(|error| {
            self.failure = Some(error);
            io::Error::other("API response encoding stopped")
        })
    }
}

fn encode(value: &impl Serialize, budget: &Budget<'_>) -> Result<Vec<u8>, ApiError> {
    budget.check()?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(RESPONSE_BYTES)
        .map_err(|_| ApiError::ServiceUnavailable)?;
    let mut output = Capped {
        bytes,
        failure: None,
        budget,
    };
    if serde_json::to_writer(&mut output, value).is_err() {
        return Err(output.failure.unwrap_or(ApiError::SerializationFailed));
    }
    budget.check()?;
    output.bytes.push(b'\n');
    Ok(output.bytes)
}

#[cfg(test)]
mod tests;
