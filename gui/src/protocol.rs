//! Lossless, bounded client DTOs for local API 1 / snapshot 2 / identity 1.
//! This module never reads hardware, source paths, or daemon configuration.
use serde::de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;

pub const RESPONSE_BYTES: usize = 1_048_576;
pub const MAX_SENSORS: usize = 4096;
const MAX_TEXT: usize = 16_384;
const TOTAL_TEXT: usize = 262_144;

fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}
fn decimal<T: std::str::FromStr, E: de::Error>(s: &str) -> Result<T, E> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(E::custom("noncanonical decimal metadata"));
    }
    s.parse()
        .map_err(|_| E::custom("decimal metadata overflow"))
}
fn u64_text<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    decimal(&String::deserialize(d)?)
}
fn u128_text<'de, D: Deserializer<'de>>(d: D) -> Result<u128, D::Error> {
    decimal(&String::deserialize(d)?)
}
fn optional_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    Option::<String>::deserialize(d)?
        .map(|s| decimal(&s))
        .transpose()
}
fn optional_u128<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u128>, D::Error> {
    Option::<String>::deserialize(d)?
        .map(|s| decimal(&s))
        .transpose()
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct IdentityEvidence {
    #[serde(deserialize_with = "nullable")]
    pub provider: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub parent: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub device_link: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub canonical_parent: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub channel: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub measurement: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Identity {
    pub schema_version: u64,
    #[serde(deserialize_with = "nullable")]
    pub candidate: Option<String>,
    pub status: String,
    pub evidence: IdentityEvidence,
    pub issues: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Sensor {
    pub identity: Identity,
    pub device: String,
    pub device_path: String,
    pub attribute: String,
    #[serde(deserialize_with = "nullable")]
    pub label: Option<String>,
    pub source_path: String,
    #[serde(deserialize_with = "nullable")]
    pub raw_text: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub raw_value: Option<i64>,
    #[serde(deserialize_with = "nullable")]
    pub value: Option<f64>,
    #[serde(deserialize_with = "nullable")]
    pub unit: Option<String>,
    pub status: String,
    #[serde(deserialize_with = "nullable")]
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Warning {
    pub path: String,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Snapshot {
    pub schema_version: u64,
    pub root: String,
    pub sensors: Vec<Sensor>,
    pub warnings: Vec<Warning>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Attempt {
    #[serde(deserialize_with = "u64_text")]
    pub sequence: u64,
    #[serde(deserialize_with = "u128_text")]
    pub started_ns: u128,
    #[serde(deserialize_with = "u128_text")]
    pub finished_ns: u128,
    #[serde(deserialize_with = "u128_text")]
    pub published_ns: u128,
    #[serde(deserialize_with = "optional_u64")]
    pub wall_start_unix_ms: Option<u64>,
    #[serde(deserialize_with = "nullable")]
    pub wall_time_error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AcquisitionError {
    pub kind: String,
    #[serde(deserialize_with = "nullable")]
    pub raw_os_error: Option<i32>,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Latest {
    pub state: String,
    #[serde(deserialize_with = "nullable")]
    pub attempt: Option<Attempt>,
    #[serde(deserialize_with = "nullable")]
    pub error: Option<AcquisitionError>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Freshness {
    pub state: String,
    #[serde(deserialize_with = "optional_u128")]
    pub lower_ns: Option<u128>,
    #[serde(deserialize_with = "optional_u128")]
    pub upper_ns: Option<u128>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Observation {
    pub attempt: Attempt,
    pub role: String,
    pub freshness: Freshness,
    pub snapshot: Snapshot,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Assessment {
    pub lifecycle: String,
    #[serde(deserialize_with = "u128_text")]
    pub assessed_at_ns: u128,
    #[serde(deserialize_with = "u128_text")]
    pub stale_after_ns: u128,
    pub producer: String,
    pub service_current: bool,
    pub physical_age_known: bool,
    pub latest: Latest,
    #[serde(deserialize_with = "nullable")]
    pub observation: Option<Observation>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotResponse {
    pub instance_id: String,
    pub assessment: Assessment,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Limits {
    pub request_bytes: usize,
    pub response_bytes: usize,
    pub active_clients: usize,
    pub request_deadline_ms: u64,
    pub response_deadline_ms: u64,
    pub max_sensors: usize,
    pub max_warnings: usize,
    pub max_identity_issues_per_sensor: usize,
    pub max_text_bytes: usize,
    pub max_total_text_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Info {
    #[serde(default, deserialize_with = "monitoring")]
    pub monitoring: Option<EffectiveMonitoring>,
    pub supported_versions: Vec<u64>,
    pub methods: Vec<String>,
    pub snapshot_schema_version: u64,
    pub identity_schema_version: u64,
    pub read_only: bool,
    pub limits: Limits,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct EffectiveMonitoring {
    pub schema_version: u8,
    pub interval_ms: u64,
    pub stale_after_ms: u64,
}

fn monitoring<'de, D: Deserializer<'de>>(d: D) -> Result<Option<EffectiveMonitoring>, D::Error> {
    let value = Value::deserialize(d)?;
    match value.get("schema_version").and_then(Value::as_u64) {
        Some(1) => {
            let config: EffectiveMonitoring =
                serde_json::from_value(value).map_err(de::Error::custom)?;
            if !(100..=60_000).contains(&config.interval_ms) || config.stale_after_ms == 0 {
                return Err(de::Error::custom("invalid effective daemon timings"));
            }
            Ok(Some(config))
        }
        Some(2..) => Ok(None),
        _ => Err(de::Error::custom("invalid monitoring schema version")),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InfoResponse {
    pub instance_id: String,
    pub info: Info,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub supported_versions: Option<Vec<u64>>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ProtocolError {
    Invalid(String),
    Remote {
        instance_id: String,
        error: ApiError,
    },
}
impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "Invalid API response: {message}"),
            Self::Remote { error, .. } => write!(f, "API error: {}", error.code),
        }
    }
}
impl std::error::Error for ProtocolError {}
fn invalid(message: &str) -> ProtocolError {
    ProtocolError::Invalid(message.into())
}
fn ensure(condition: bool, message: &str) -> Result<(), ProtocolError> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
fn decode<T: DeserializeOwned>(v: Value) -> Result<T, ProtocolError> {
    serde_json::from_value(v).map_err(|_| invalid("missing, null, mistyped, or out-of-range field"))
}

// serde_json::Value normally accepts duplicate keys. Reject them recursively
// before typed decoding, including in ignored future additive fields. Retain
// signed/unsigned integers as Number, never through a floating-point round trip.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.to_owned())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
                let mut values = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                    let Unique(value) = map.next_value()?;
                    values.insert(key, value);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        d.deserialize_any(V)
    }
}
fn envelope(frame: &[u8]) -> Result<(String, Value), ProtocolError> {
    ensure(frame.len() <= RESPONSE_BYTES, "response exceeds 1 MiB")?;
    let content = frame
        .strip_suffix(b"\n")
        .ok_or_else(|| invalid("incomplete LF frame"))?;
    ensure(!content.contains(&b'\n'), "multiple frames or embedded LF")?;
    let Unique(mut v) = serde_json::from_slice(content)
        .map_err(|_| invalid("malformed JSON, UTF-8, duplicate key, or excessive nesting"))?;
    let object = v
        .as_object_mut()
        .ok_or_else(|| invalid("response must be an object"))?;
    ensure(
        object.remove("api_version").and_then(|v| v.as_u64()) == Some(1),
        "unsupported API version",
    )?;
    let instance: String = decode(
        object
            .remove("instance_id")
            .ok_or_else(|| invalid("missing instance"))?,
    )?;
    ensure(
        instance.len() == 32
            && instance
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid instance token",
    )?;
    let ok: bool = decode(
        object
            .remove("ok")
            .ok_or_else(|| invalid("missing success flag"))?,
    )?;
    if ok {
        ensure(!object.contains_key("error"), "success carries an error")?;
        Ok((
            instance,
            object
                .remove("result")
                .ok_or_else(|| invalid("missing result"))?,
        ))
    } else {
        ensure(!object.contains_key("result"), "failure carries a result")?;
        let error_value = object
            .remove("error")
            .ok_or_else(|| invalid("missing error"))?;
        let has_versions = error_value.get("supported_versions").is_some();
        let error: ApiError = decode(error_value)?;
        let retry = match error.code.as_str() {
            "request_timeout" | "cache_busy" => true,
            "invalid_request"
            | "unsupported_version"
            | "unknown_method"
            | "request_too_large"
            | "response_too_large"
            | "serialization_failed"
            | "service_unavailable" => false,
            _ => return Err(invalid("unknown API error code")),
        };
        ensure(
            error.retryable == retry && error.message.len() <= MAX_TEXT,
            "invalid API error semantics",
        )?;
        ensure(
            if error.code == "unsupported_version" {
                error.supported_versions.as_deref() == Some(&[1])
            } else {
                !has_versions
            },
            "invalid supported versions on error",
        )?;
        Err(ProtocolError::Remote {
            instance_id: instance,
            error,
        })
    }
}
pub fn parse_info(frame: &[u8]) -> Result<InfoResponse, ProtocolError> {
    let (instance_id, value) = envelope(frame)?;
    let info: Info = decode(value)?;
    let limits = Limits {
        request_bytes: 4096,
        response_bytes: RESPONSE_BYTES,
        active_clients: 1,
        request_deadline_ms: 1000,
        response_deadline_ms: 2000,
        max_sensors: MAX_SENSORS,
        max_warnings: 1024,
        max_identity_issues_per_sensor: 64,
        max_text_bytes: MAX_TEXT,
        max_total_text_bytes: TOTAL_TEXT,
    };
    ensure(
        info.supported_versions == [1]
            && info.methods == ["get_info", "get_snapshot"]
            && info.snapshot_schema_version == 2
            && info.identity_schema_version == 1
            && info.read_only
            && info.limits == limits,
        "incompatible versions, methods, or limits",
    )?;
    Ok(InfoResponse { instance_id, info })
}
pub fn parse_snapshot(frame: &[u8]) -> Result<SnapshotResponse, ProtocolError> {
    let (instance_id, value) = envelope(frame)?;
    let assessment: Assessment = decode(value)?;
    validate_assessment(&assessment)?;
    Ok(SnapshotResponse {
        instance_id,
        assessment,
    })
}
#[derive(Default)]
struct TextBudget(usize);
impl TextBudget {
    fn add(&mut self, s: &str) -> Result<(), ProtocolError> {
        ensure(s.len() <= MAX_TEXT, "text field exceeds limit")?;
        self.0 = self
            .0
            .checked_add(s.len())
            .ok_or_else(|| invalid("text size overflow"))?;
        ensure(self.0 <= TOTAL_TEXT, "aggregate text exceeds limit")
    }
    fn optional(&mut self, s: &Option<String>) -> Result<(), ProtocolError> {
        if let Some(s) = s {
            self.add(s)?;
        }
        Ok(())
    }
}
fn attempt(a: &Attempt, assessment: u128, text: &mut TextBudget) -> Result<(), ProtocolError> {
    ensure(
        a.started_ns <= a.finished_ns
            && a.finished_ns <= a.published_ns
            && a.published_ns <= assessment,
        "inconsistent attempt times",
    )?;
    ensure(
        a.wall_start_unix_ms.is_some() != a.wall_time_error.is_some(),
        "inconsistent wall time",
    )?;
    text.optional(&a.wall_time_error)
}
fn validate_assessment(a: &Assessment) -> Result<(), ProtocolError> {
    ensure(
        matches!(
            a.lifecycle.as_str(),
            "starting" | "running" | "stopping" | "stopped"
        ),
        "unknown lifecycle",
    )?;
    ensure(
        matches!(a.producer.as_str(), "active" | "stopped"),
        "unknown producer",
    )?;
    ensure(
        a.stale_after_ns > 0 && !a.physical_age_known,
        "unsupported freshness contract",
    )?;
    let mut text = TextBudget::default();
    if let Some(t) = &a.latest.attempt {
        attempt(t, a.assessed_at_ns, &mut text)?;
    }
    if let Some(e) = &a.latest.error {
        ensure(e.kind.len() <= MAX_TEXT, "I/O kind too long")?;
        text.add(&e.message)?;
    }
    match a.latest.state.as_str() {
        "empty" => ensure(
            a.latest.attempt.is_none() && a.latest.error.is_none() && a.observation.is_none(),
            "inconsistent empty cache",
        )?,
        "succeeded" => ensure(
            a.latest.attempt.is_some() && a.latest.error.is_none() && a.observation.is_some(),
            "inconsistent successful acquisition",
        )?,
        "failed" => ensure(
            a.latest.attempt.is_some() && a.latest.error.is_some(),
            "inconsistent failed acquisition",
        )?,
        _ => return Err(invalid("unknown latest state")),
    }
    if let Some(o) = &a.observation {
        attempt(&o.attempt, a.assessed_at_ns, &mut text)?;
        let expected_role = if a.producer == "stopped" {
            "historical_after_stop"
        } else if a.latest.state == "failed" {
            "historical_after_failure"
        } else {
            "latest_attempt"
        };
        ensure(o.role == expected_role, "inconsistent observation role")?;
        let latest = a
            .latest
            .attempt
            .as_ref()
            .ok_or_else(|| invalid("observation without acquisition"))?;
        if a.latest.state == "succeeded" {
            ensure(
                &o.attempt == latest,
                "observation differs from successful attempt",
            )?;
        } else {
            ensure(
                o.attempt.sequence < latest.sequence
                    && o.attempt.published_ns <= latest.published_ns,
                "historical observation is not older",
            )?;
        }
        let f = &o.freshness;
        if o.snapshot.sensors.is_empty() {
            ensure(
                f.state == "no_sensor_observations" && f.lower_ns.is_none() && f.upper_ns.is_none(),
                "invalid empty-observation freshness",
            )?;
        } else {
            let lower = a.assessed_at_ns - o.attempt.finished_ns;
            let upper = a.assessed_at_ns - o.attempt.started_ns;
            ensure(
                f.lower_ns == Some(lower) && f.upper_ns == Some(upper),
                "incorrect age bounds",
            )?;
            ensure(
                f.state
                    == if upper > a.stale_after_ns {
                        "stale"
                    } else {
                        "within_budget"
                    },
                "inconsistent freshness state",
            )?;
        }
        snapshot(&o.snapshot, &mut text)?;
    }
    let current = a.lifecycle == "running"
        && a.producer == "active"
        && a.latest.state == "succeeded"
        && a.observation
            .as_ref()
            .is_some_and(|o| o.role == "latest_attempt" && o.freshness.state == "within_budget");
    ensure(a.service_current == current, "inconsistent service_current")
}
fn snapshot(s: &Snapshot, text: &mut TextBudget) -> Result<(), ProtocolError> {
    ensure(
        s.schema_version == 2 && s.sensors.len() <= MAX_SENSORS && s.warnings.len() <= 1024,
        "unsupported snapshot version or count",
    )?;
    text.add(&s.root)?;
    for w in &s.warnings {
        text.add(&w.path)?;
        text.add(&w.message)?;
    }
    for r in &s.sensors {
        ensure(
            matches!(
                r.status.as_str(),
                "ok" | "unavailable"
                    | "permission_denied"
                    | "unsupported"
                    | "invalid"
                    | "error"
                    | "fault"
            ),
            "unknown sensor status",
        )?;
        ensure(
            r.value.is_none_or(f64::is_finite),
            "nonfinite canonical value",
        )?;
        for v in [&r.device, &r.device_path, &r.attribute, &r.source_path] {
            text.add(v)?;
        }
        for v in [&r.label, &r.raw_text, &r.unit, &r.error] {
            text.optional(v)?;
        }
        let i = &r.identity;
        ensure(
            i.schema_version == 1 && i.issues.len() <= 64,
            "unsupported identity version or count",
        )?;
        ensure(
            matches!(
                i.status.as_str(),
                "best_effort"
                    | "insufficient_evidence"
                    | "unreadable_evidence"
                    | "conflicting_evidence"
                    | "ambiguous_channel"
                    | "collision"
            ),
            "unknown identity status",
        )?;
        text.optional(&i.candidate)?;
        for v in [
            &i.evidence.provider,
            &i.evidence.parent,
            &i.evidence.device_link,
            &i.evidence.canonical_parent,
            &i.evidence.channel,
            &i.evidence.measurement,
        ] {
            text.optional(v)?;
        }
        for issue in &i.issues {
            text.add(issue)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "protocol/tests.rs"]
pub(crate) mod tests;
