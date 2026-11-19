use super::*;
use crate::hwmon::{Sensor, Status, Warning};
use crate::identity::{Evidence, IdentityStatus, SourceIdentity};
use crate::snapshot_cache::AgeBounds;
use serde::ser::{SerializeSeq, Serializer};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

const INSTANCE: &str = "0123456789abcdef0123456789abcdef";

fn budget(cancelled: &AtomicBool) -> Budget<'_> {
    Budget::new(Instant::now() + Duration::from_secs(30), cancelled)
}

fn decode(bytes: &[u8]) -> Value {
    assert!(bytes.len() <= RESPONSE_BYTES);
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert!(!bytes[..bytes.len() - 1].contains(&b'\n'));
    serde_json::from_slice(bytes).unwrap()
}

fn attempt(sequence: u64) -> AttemptMetadata {
    AttemptMetadata {
        sequence,
        started: Duration::from_millis(100),
        finished: Duration::from_millis(110),
        published: Duration::from_millis(120),
        wall_start_unix_ms: Some(1_700_000_000_000),
        wall_time_error: None,
    }
}

fn sensor() -> Sensor {
    Sensor {
        identity: SourceIdentity {
            schema_version: 1,
            candidate: Some("[\"fixture\",1,\"temperature\"]".into()),
            status: IdentityStatus::Collision,
            evidence: Evidence {
                provider: Some("fixture".into()),
                parent: Some("/fixture/device".into()),
                device_link: Some("/fixture/device-link".into()),
                canonical_parent: Some("/fixture/canonical".into()),
                channel: Some("temp1_input".into()),
                measurement: Some("temperature"),
            },
            issues: vec!["preserve collision evidence".into(), "second issue".into()],
        },
        device: "fixture device".into(),
        device_path: "/fixture/hwmon0".into(),
        attribute: "temp1_input".into(),
        label: Some("unaltered label\n\t\"".into()),
        source_path: "/fixture/hwmon0/temp1_input".into(),
        raw_text: Some(format!(" {}\n", i64::MAX)),
        raw_value: Some(i64::MAX),
        value: Some(i64::MAX as f64 / 1000.0),
        unit: Some("°C"),
        status: Status::Fault,
        error: Some("unvalidated fixture fault".into()),
    }
}

fn sample_snapshot() -> Snapshot {
    let mut unsupported = sensor();
    unsupported.identity.candidate = None;
    unsupported.identity.status = IdentityStatus::AmbiguousChannel;
    unsupported.attribute = "power1_input".into();
    unsupported.raw_text = Some("17".into());
    unsupported.raw_value = Some(17);
    unsupported.value = None;
    unsupported.unit = None;
    unsupported.status = Status::Unsupported;
    unsupported.error = Some("unsupported fixture input".into());
    Snapshot {
        schema_version: 2,
        root: "/fixture".into(),
        sensors: vec![sensor(), unsupported],
        warnings: vec![Warning {
            path: "/fixture/warning".into(),
            message: "retain warning\nverbatim".into(),
        }],
    }
}

fn observed<'a>(snapshot: &'a Snapshot, attempt: &'a AttemptMetadata) -> Assessment<'a> {
    Assessment {
        assessed_at: Duration::from_secs(2),
        producer: ProducerState::Active,
        latest: LatestAttempt::Succeeded(attempt),
        observation: Some(Observation {
            snapshot,
            attempt,
            role: ObservationRole::LatestAttempt,
            freshness: Freshness::WithinBudget(AgeBounds {
                lower: Duration::from_millis(1890),
                upper: Duration::from_millis(1900),
            }),
        }),
        physical_age_known: false,
    }
}

fn record(assessment: Assessment<'_>, lifecycle: &'static str) -> Value {
    let cancelled = AtomicBool::new(false);
    decode(
        &snapshot(
            INSTANCE,
            lifecycle,
            assessment,
            Duration::from_secs(3),
            &budget(&cancelled),
        )
        .unwrap(),
    )
}

#[test]
fn request_parser_accepts_only_explicit_versioned_read_methods() {
    assert_eq!(
        parse_request(b"{\"api_version\":1,\"method\":\"get_info\"}\n"),
        Ok(Method::GetInfo)
    );
    assert_eq!(
        parse_request(b" {\"method\":\"get_snapshot\",\"api_version\":1} \n"),
        Ok(Method::GetSnapshot)
    );
    for version in [0, 2, 99, u64::MAX] {
        assert_eq!(
            parse_request(
                format!("{{\"api_version\":{version},\"method\":\"get_info\"}}\n").as_bytes()
            ),
            Err(ApiError::UnsupportedVersion)
        );
    }
    for method in ["refresh", "scan", "stop", "set", "GET_INFO", ""] {
        assert_eq!(
            parse_request(format!("{{\"api_version\":1,\"method\":\"{method}\"}}\n").as_bytes()),
            Err(ApiError::UnknownMethod)
        );
    }
}

#[test]
fn request_parser_rejects_ambiguous_fields_types_and_framing() {
    let invalid: &[&[u8]] = &[
        b"",
        b"\n",
        b"null\n",
        b"[]\n",
        b"{\"api_version\":1}\n",
        b"{\"method\":\"get_info\"}\n",
        b"{\"api_version\":1,\"method\":\"get_info\",\"root\":\"/sys\"}\n",
        b"{\"api_version\":1,\"api_version\":1,\"method\":\"get_info\"}\n",
        b"{\"api_version\":1,\"method\":\"get_info\",\"method\":\"get_snapshot\"}\n",
        b"{\"api_version\":1.0,\"method\":\"get_info\"}\n",
        b"{\"api_version\":-1,\"method\":\"get_info\"}\n",
        b"{\"api_version\":\"1\",\"method\":\"get_info\"}\n",
        b"{\"api_version\":1,\"method\":false}\n",
        b"{\"api_version\":1,\"method\":\"get_info\"}",
        b"{\"api_version\":1,\"method\":\"get_info\"} {}\n",
        b"{\"api_version\":1,\"method\":\"get_info\"}\n{}\n",
        b"{\n\"api_version\":1,\"method\":\"get_info\"}\n",
        b"{\"api_version\":1,\"method\":\"\xff\"}\n",
    ];
    for frame in invalid {
        assert_eq!(
            parse_request(frame),
            Err(ApiError::InvalidRequest),
            "{frame:?}"
        );
    }
    let mut nested = vec![b'['; 256];
    nested.extend(std::iter::repeat_n(b']', 256));
    nested.push(b'\n');
    assert_eq!(parse_request(&nested), Err(ApiError::InvalidRequest));
}

#[test]
fn request_limit_includes_final_lf_and_accepts_exact_boundary() {
    let mut frame = b"{\"api_version\":1,\"method\":\"get_info\"}".to_vec();
    frame.resize(REQUEST_BYTES - 1, b' ');
    frame.push(b'\n');
    assert_eq!(parse_request(&frame), Ok(Method::GetInfo));
    frame.insert(frame.len() - 1, b' ');
    assert_eq!(parse_request(&frame), Err(ApiError::RequestTooLarge));
}

#[test]
fn info_and_error_records_have_explicit_versions_limits_and_retry_policy() {
    let cancelled = AtomicBool::new(false);
    let value = decode(&info(INSTANCE, &budget(&cancelled), None).unwrap());
    assert_eq!(value["api_version"], 1);
    assert_eq!(value["instance_id"], INSTANCE);
    assert_eq!(value["ok"], true);
    assert!(value.get("error").is_none());
    assert_eq!(
        value["result"],
        json!({
            "supported_versions": [1],
            "methods": ["get_info", "get_snapshot"],
            "snapshot_schema_version": 2,
            "identity_schema_version": 1,
            "read_only": true,
            "limits": {
                "request_bytes": 4096,
                "response_bytes": 1048576,
                "active_clients": 1,
                "request_deadline_ms": 1000,
                "response_deadline_ms": 2000,
                "max_sensors": 4096,
                "max_warnings": 1024,
                "max_identity_issues_per_sensor": 64,
                "max_text_bytes": 16384,
                "max_total_text_bytes": 262144
            }
        })
    );
    for code in [
        ApiError::InvalidRequest,
        ApiError::UnsupportedVersion,
        ApiError::UnknownMethod,
        ApiError::RequestTooLarge,
        ApiError::RequestTimeout,
        ApiError::CacheBusy,
        ApiError::ResponseTooLarge,
        ApiError::SerializationFailed,
        ApiError::ServiceUnavailable,
    ] {
        let bytes = error(INSTANCE, code);
        assert!(bytes.len() < 256);
        let value = decode(&bytes);
        assert_eq!(value["ok"], false);
        assert!(value.get("result").is_none());
        assert_eq!(
            value["error"]["retryable"],
            matches!(code, ApiError::RequestTimeout | ApiError::CacheBusy)
        );
        assert_eq!(
            value["error"].get("supported_versions").is_some(),
            code == ApiError::UnsupportedVersion
        );
    }
}

#[test]
fn snapshot_preserves_every_raw_identity_path_and_diagnostic_field() {
    let sample = sample_snapshot();
    let attempt = attempt(1);
    let expected = serde_json::to_value(&sample).unwrap();
    let value = record(observed(&sample, &attempt), "running");
    assert_eq!(value["result"]["observation"]["snapshot"], expected);
    assert_eq!(
        value["result"]["observation"]["snapshot"]["sensors"][0]["raw_value"].as_i64(),
        Some(i64::MAX)
    );
    assert_eq!(value["result"]["service_current"], true);
    assert_eq!(value["result"]["physical_age_known"], false);
    assert_eq!(value["result"]["latest"]["state"], "succeeded");
    assert!(value["result"]["latest"]["error"].is_null());
    assert_eq!(
        value["result"]["observation"]["freshness"],
        json!({"state":"within_budget", "lower_ns":"1890000000", "upper_ns":"1900000000"})
    );
}

#[test]
fn latest_failure_keeps_its_own_metadata_and_historical_success() {
    let sample = sample_snapshot();
    let previous = attempt(1);
    let mut latest = attempt(2);
    latest.started = Duration::from_secs(1);
    latest.finished = Duration::from_millis(1010);
    latest.published = Duration::from_millis(1020);
    latest.wall_start_unix_ms = None;
    latest.wall_time_error = Some("wall clock unavailable".into());
    let failure = AcquisitionError {
        kind: io::ErrorKind::NotFound,
        raw_os_error: Some(2),
        message: "fixture root unavailable".into(),
    };
    let mut assessment = observed(&sample, &previous);
    assessment.latest = LatestAttempt::Failed {
        attempt: &latest,
        error: &failure,
    };
    assessment.observation.as_mut().unwrap().role = ObservationRole::HistoricalAfterFailure;
    let value = record(assessment, "running");
    assert_eq!(value["ok"], true);
    assert_eq!(value["result"]["service_current"], false);
    assert_eq!(value["result"]["latest"]["state"], "failed");
    assert_eq!(value["result"]["latest"]["attempt"]["sequence"], "2");
    assert_eq!(value["result"]["observation"]["attempt"]["sequence"], "1");
    assert_eq!(
        value["result"]["latest"]["attempt"]["started_ns"],
        "1000000000"
    );
    assert_eq!(
        value["result"]["observation"]["attempt"]["started_ns"],
        "100000000"
    );
    assert_eq!(
        value["result"]["latest"]["attempt"]["wall_time_error"],
        "wall clock unavailable"
    );
    assert_eq!(
        value["result"]["latest"]["error"],
        json!({
            "kind":"NotFound", "raw_os_error":2, "message":"fixture root unavailable"
        })
    );
    assert_eq!(
        value["result"]["observation"]["role"],
        "historical_after_failure"
    );
    assert_eq!(
        value["result"]["observation"]["snapshot"],
        serde_json::to_value(&sample).unwrap()
    );
}

#[test]
fn no_publication_first_failure_and_successful_empty_inventory_stay_distinct() {
    let empty = Assessment {
        assessed_at: Duration::ZERO,
        producer: ProducerState::Active,
        latest: LatestAttempt::Empty,
        observation: None,
        physical_age_known: false,
    };
    let value = record(empty, "starting");
    assert_eq!(
        value["result"]["latest"],
        json!({"state":"empty","attempt":null,"error":null})
    );
    assert!(value["result"]["observation"].is_null());

    let attempt = attempt(1);
    let failure = AcquisitionError {
        kind: io::ErrorKind::PermissionDenied,
        raw_os_error: Some(13),
        message: "fixture unavailable".into(),
    };
    let value = record(
        Assessment {
            assessed_at: Duration::from_secs(2),
            producer: ProducerState::Active,
            latest: LatestAttempt::Failed {
                attempt: &attempt,
                error: &failure,
            },
            observation: None,
            physical_age_known: false,
        },
        "running",
    );
    assert_eq!(value["result"]["latest"]["state"], "failed");
    assert!(value["result"]["observation"].is_null());
    assert_eq!(value["result"]["service_current"], false);

    let mut sample = sample_snapshot();
    sample.sensors.clear();
    let mut assessment = observed(&sample, &attempt);
    assessment.observation.as_mut().unwrap().freshness = Freshness::NoSensorObservations;
    let value = record(assessment, "running");
    assert_eq!(value["result"]["latest"]["state"], "succeeded");
    assert_eq!(value["result"]["service_current"], false);
    assert_eq!(
        value["result"]["observation"]["snapshot"]["sensors"],
        json!([])
    );
    assert_eq!(
        value["result"]["observation"]["freshness"],
        json!({
            "state":"no_sensor_observations", "lower_ns":null,"upper_ns":null
        })
    );
}

#[test]
fn lifecycle_producer_role_and_freshness_all_constrain_currentness() {
    let sample = sample_snapshot();
    let attempt = attempt(1);
    let ages = AgeBounds {
        lower: Duration::from_millis(2990),
        upper: Duration::from_secs(3),
    };
    for phase in ["starting", "running", "stopping", "stopped"] {
        for producer in [ProducerState::Active, ProducerState::Stopped] {
            for role in [
                ObservationRole::LatestAttempt,
                ObservationRole::HistoricalAfterFailure,
                ObservationRole::HistoricalAfterStop,
            ] {
                for freshness in [
                    Freshness::WithinBudget(ages),
                    Freshness::Stale(ages),
                    Freshness::NoSensorObservations,
                ] {
                    let mut assessment = observed(&sample, &attempt);
                    assessment.producer = producer;
                    let observation = assessment.observation.as_mut().unwrap();
                    observation.role = role;
                    observation.freshness = freshness;
                    let value = record(assessment, phase);
                    assert_eq!(
                        value["result"]["service_current"],
                        phase == "running"
                            && producer == ProducerState::Active
                            && role == ObservationRole::LatestAttempt
                            && matches!(freshness, Freshness::WithinBudget(_))
                    );
                    assert_eq!(value["result"]["lifecycle"], phase);
                    assert_eq!(
                        value["result"]["observation"]["role"],
                        match role {
                            ObservationRole::LatestAttempt => "latest_attempt",
                            ObservationRole::HistoricalAfterFailure => "historical_after_failure",
                            ObservationRole::HistoricalAfterStop => "historical_after_stop",
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn full_width_sequences_offsets_and_wall_time_serialize_without_narrowing() {
    let sample = sample_snapshot();
    let mut attempt = attempt(u64::MAX);
    attempt.started = Duration::new(u64::MAX - 2, 123);
    attempt.finished = Duration::new(u64::MAX - 1, 456);
    attempt.published = Duration::new(u64::MAX, 789);
    attempt.wall_start_unix_ms = Some(u64::MAX);
    let mut assessment = observed(&sample, &attempt);
    assessment.assessed_at = Duration::new(u64::MAX, 999_999_999);
    let value = record(assessment, "running");
    assert_eq!(
        value["result"]["latest"]["attempt"]["sequence"],
        u64::MAX.to_string()
    );
    assert_eq!(
        value["result"]["latest"]["attempt"]["started_ns"],
        attempt.started.as_nanos().to_string()
    );
    assert_eq!(
        value["result"]["latest"]["attempt"]["wall_start_unix_ms"],
        u64::MAX.to_string()
    );
    assert_eq!(
        value["result"]["assessed_at_ns"],
        Duration::new(u64::MAX, 999_999_999).as_nanos().to_string()
    );
}

#[test]
fn unrepresentable_values_and_paths_fail_without_a_partial_record() {
    let cancelled = AtomicBool::new(false);
    let budget = budget(&cancelled);
    let attempt = attempt(1);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut sample = sample_snapshot();
        sample.sensors[0].value = Some(value);
        assert_eq!(
            snapshot(
                INSTANCE,
                "running",
                observed(&sample, &attempt),
                Duration::from_secs(3),
                &budget
            ),
            Err(ApiError::SerializationFailed)
        );
    }
    let mut sample = sample_snapshot();
    sample.sensors[0].identity.evidence.canonical_parent =
        Some(PathBuf::from(OsString::from_vec(vec![0xff])));
    assert_eq!(
        snapshot(
            INSTANCE,
            "running",
            observed(&sample, &attempt),
            Duration::from_secs(3),
            &budget
        ),
        Err(ApiError::SerializationFailed)
    );
}

#[test]
fn collection_and_each_text_bounds_are_enforced_before_encoding() {
    let cancelled = AtomicBool::new(false);
    let budget = budget(&cancelled);
    let mut sample = sample_snapshot();
    sample.sensors = (0..=MAX_SENSORS).map(|_| sensor()).collect();
    let attempt = attempt(1);
    assert_eq!(
        preflight(&observed(&sample, &attempt), &budget),
        Err(ApiError::ResponseTooLarge)
    );
    sample.sensors.clear();
    sample.warnings = (0..=MAX_WARNINGS)
        .map(|_| Warning {
            path: "x".into(),
            message: String::new(),
        })
        .collect();
    assert_eq!(
        preflight(&observed(&sample, &attempt), &budget),
        Err(ApiError::ResponseTooLarge)
    );
    sample = sample_snapshot();
    sample.sensors[0].identity.issues = vec![String::new(); MAX_IDENTITY_ISSUES_PER_SENSOR];
    assert!(preflight(&observed(&sample, &attempt), &budget).is_ok());
    sample.sensors[0].identity.issues.push(String::new());
    assert_eq!(
        preflight(&observed(&sample, &attempt), &budget),
        Err(ApiError::ResponseTooLarge)
    );

    let mut check = Preflight {
        total: 0,
        budget: &budget,
    };
    assert!(check.text(&"x".repeat(MAX_TEXT_BYTES)).is_ok());
    assert_eq!(
        check.text(&"x".repeat(MAX_TEXT_BYTES + 1)),
        Err(ApiError::ResponseTooLarge)
    );
    let mut check = Preflight {
        total: 0,
        budget: &budget,
    };
    for _ in 0..MAX_TOTAL_TEXT_BYTES / MAX_TEXT_BYTES {
        check.text(&"x".repeat(MAX_TEXT_BYTES)).unwrap();
    }
    assert_eq!(check.total, MAX_TOTAL_TEXT_BYTES);
    assert_eq!(check.text("x"), Err(ApiError::ResponseTooLarge));
}

#[test]
fn oversized_raw_metadata_and_error_text_is_not_silently_dropped() {
    let cancelled = AtomicBool::new(false);
    let budget = budget(&cancelled);
    let sample = sample_snapshot();
    let mut attempt = attempt(1);
    attempt.wall_time_error = Some("x".repeat(MAX_TEXT_BYTES + 1));
    assert_eq!(
        preflight(&observed(&sample, &attempt), &budget),
        Err(ApiError::ResponseTooLarge)
    );
    attempt.wall_time_error = None;
    let failure = AcquisitionError {
        kind: io::ErrorKind::Other,
        raw_os_error: None,
        message: "x".repeat(MAX_TEXT_BYTES + 1),
    };
    let mut assessment = observed(&sample, &attempt);
    assessment.latest = LatestAttempt::Failed {
        attempt: &attempt,
        error: &failure,
    };
    assert_eq!(
        preflight(&assessment, &budget),
        Err(ApiError::ResponseTooLarge)
    );
    let mut sample = sample;
    sample.sensors[0].raw_text = Some("x".repeat(MAX_TEXT_BYTES + 1));
    assert_eq!(
        preflight(&observed(&sample, &attempt), &budget),
        Err(ApiError::ResponseTooLarge)
    );
}

#[test]
fn escaping_expansion_is_capped_and_exact_encoded_boundary_includes_lf() {
    let cancelled = AtomicBool::new(false);
    let budget = budget(&cancelled);
    let exact = "x".repeat(RESPONSE_BYTES - 3);
    assert_eq!(encode(&exact, &budget).unwrap().len(), RESPONSE_BYTES);
    assert_eq!(
        encode(&(exact + "x"), &budget),
        Err(ApiError::ResponseTooLarge)
    );
    let attempt = attempt(1);
    let mut sample = sample_snapshot();
    sample.sensors.clear();
    sample.warnings = (0..16)
        .map(|_| Warning {
            path: "x".into(),
            message: "\u{1}".repeat(16000),
        })
        .collect();
    assert!(preflight(&observed(&sample, &attempt), &budget).is_ok());
    assert_eq!(
        snapshot(
            INSTANCE,
            "running",
            observed(&sample, &attempt),
            Duration::from_secs(3),
            &budget
        ),
        Err(ApiError::ResponseTooLarge)
    );
}

#[test]
fn cancellation_and_expired_budgets_stop_preflight_and_encoding() {
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        info(INSTANCE, &budget(&cancelled), None),
        Err(ApiError::Cancelled)
    );
    cancelled.store(false, Ordering::Release);
    let expired = Budget::new(Instant::now() - Duration::from_secs(1), &cancelled);
    assert_eq!(info(INSTANCE, &expired, None), Err(ApiError::Deadline));
    let sample = sample_snapshot();
    let attempt = attempt(1);
    assert_eq!(
        preflight(&observed(&sample, &attempt), &expired),
        Err(ApiError::Deadline)
    );

    struct CancelWhileEncoding<'a>(&'a AtomicBool);
    impl Serialize for CancelWhileEncoding<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut sequence = serializer.serialize_seq(Some(2))?;
            sequence.serialize_element("first")?;
            self.0.store(true, Ordering::Release);
            sequence.serialize_element("second")?;
            sequence.end()
        }
    }
    assert_eq!(
        encode(&CancelWhileEncoding(&cancelled), &budget(&cancelled)),
        Err(ApiError::Cancelled)
    );
}

#[test]
fn invalid_internal_epoch_or_lifecycle_is_never_published_as_healthy() {
    let cancelled = AtomicBool::new(false);
    for instance in [
        "",
        "bad",
        "0123456789abcdef0123456789abcde\"",
        "ABCDEF0123456789abcdef0123456789ab",
    ] {
        assert_eq!(
            info(instance, &budget(&cancelled), None),
            Err(ApiError::ServiceUnavailable)
        );
    }
    let sample = sample_snapshot();
    let attempt = attempt(1);
    assert_eq!(
        snapshot(
            INSTANCE,
            "poisoned",
            observed(&sample, &attempt),
            Duration::from_secs(3),
            &budget(&cancelled)
        ),
        Err(ApiError::ServiceUnavailable)
    );
}
