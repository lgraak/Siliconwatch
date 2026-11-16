use crate::identity::{self, SourceIdentity};
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub schema_version: u8,
    pub root: PathBuf,
    pub sensors: Vec<Sensor>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Serialize)]
pub struct Warning {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Sensor {
    pub identity: SourceIdentity,
    pub device: String,
    pub device_path: PathBuf,
    pub attribute: String,
    pub label: Option<String>,
    pub source_path: PathBuf,
    pub raw_text: Option<String>,
    pub raw_value: Option<i64>,
    pub value: Option<f64>,
    pub unit: Option<&'static str>,
    pub status: Status,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Unavailable,
    PermissionDenied,
    Unsupported,
    Invalid,
    Error,
    Fault,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Unavailable => "unavailable",
            Self::PermissionDenied => "permission_denied",
            Self::Unsupported => "unsupported",
            Self::Invalid => "invalid",
            Self::Error => "error",
            Self::Fault => "fault",
        }
    }
}

/// Read existing hwmon attributes without changing hardware or configuration.
pub fn scan(root: &Path) -> io::Result<Snapshot> {
    let mut snapshot = Snapshot {
        schema_version: 2,
        root: root.to_path_buf(),
        sensors: Vec::new(),
        warnings: Vec::new(),
    };
    let root_canonical = fs::canonicalize(root).ok();
    for path in entries(root, &mut snapshot.warnings)? {
        let fallback = path.file_name().unwrap_or_default().to_string_lossy();
        if !numbered(&fallback, "hwmon") {
            continue;
        }
        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => continue,
            Err(error) => {
                warn(&mut snapshot.warnings, &path, error.to_string());
                continue;
            }
        }
        let device_path = fs::canonicalize(&path).unwrap_or_else(|error| {
            warn(&mut snapshot.warnings, &path, error.to_string());
            path.clone()
        });
        let name_path = path.join("name");
        let name = fs::read_to_string(&name_path);
        let device_identity = identity::for_device(&path, root_canonical.as_deref(), &name);
        let device = match name {
            Ok(name) if !name.trim().is_empty() => name.trim().to_owned(),
            result => {
                let message = match result {
                    Err(error) => error.to_string(),
                    Ok(_) => "empty device name".to_owned(),
                };
                warn(&mut snapshot.warnings, &name_path, message);
                fallback.into_owned()
            }
        };
        let attributes = match entries(&path, &mut snapshot.warnings) {
            Ok(attributes) => attributes,
            Err(error) => {
                warn(&mut snapshot.warnings, &path, error.to_string());
                continue;
            }
        };
        for source_path in attributes {
            let attribute = source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            if !attribute.ends_with("_input") {
                continue;
            }
            let mut sensor = read_sensor(
                &device,
                &device_path,
                &source_path,
                &mut snapshot.warnings,
                |path| fs::read_to_string(path),
            );
            sensor.identity = device_identity.for_channel(&sensor.attribute);
            snapshot.sensors.push(sensor);
        }
    }
    identity::mark_collisions(&mut snapshot.sensors);
    snapshot
        .warnings
        .sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.message.cmp(&b.message)));
    Ok(snapshot)
}

fn read_sensor(
    device: &str,
    device_path: &Path,
    source: &Path,
    warnings: &mut Vec<Warning>,
    read_text: impl Fn(&Path) -> io::Result<String>,
) -> Sensor {
    let attribute = source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let channel = attribute.strip_suffix("_input").unwrap_or(&attribute);
    let conversion = if numbered(channel, "temp") {
        Some((1000.0, "°C"))
    } else if numbered(channel, "fan") {
        Some((1.0, "RPM"))
    } else if numbered(channel, "in") {
        Some((1000.0, "V"))
    } else {
        None
    };
    let label_path = source.with_file_name(format!("{channel}_label"));
    let label = match optional_text(&label_path, &read_text) {
        Ok(label) => label.filter(|text| !text.is_empty()),
        Err(error) => {
            warn(warnings, &label_path, error.to_string());
            None
        }
    };
    let mut sensor = Sensor {
        identity: SourceIdentity::unresolved(),
        device: device.to_owned(),
        device_path: device_path.to_path_buf(),
        attribute: attribute.clone(),
        label,
        source_path: source.to_path_buf(),
        raw_text: None,
        raw_value: None,
        value: None,
        unit: conversion.map(|(_, unit)| unit),
        status: Status::Ok,
        error: None,
    };
    match read_text(source) {
        Ok(text) => {
            let text = text.trim().to_owned();
            match text.parse::<i64>() {
                Ok(raw) => {
                    sensor.raw_value = Some(raw);
                    if let Some((divisor, _)) = conversion {
                        sensor.value = Some(raw as f64 / divisor);
                    } else {
                        sensor.status = Status::Unsupported;
                        sensor.error = Some("unsupported hwmon input family".to_owned());
                    }
                }
                Err(error) => {
                    sensor.status = Status::Invalid;
                    sensor.error = Some(format!("invalid integer: {error}"));
                }
            }
            sensor.raw_text = Some(text);
        }
        Err(error) => {
            sensor.status = classify(&error);
            sensor.error = Some(error.to_string());
        }
    }
    // A parsed unknown family is still eligible for flags; a failed input is not.
    let mut primary_problem_recorded = sensor.raw_value.is_none();
    for (suffix, triggering_value, status, message) in [
        ("enable", 0, Status::Unavailable, "sensor is disabled"),
        ("fault", 1, Status::Fault, "sensor reports a fault"),
    ] {
        let flag_path = source.with_file_name(format!("{channel}_{suffix}"));
        let problem = match optional_text(&flag_path, &read_text) {
            Ok(None) => None,
            Ok(Some(text)) => match text.parse::<u8>() {
                Ok(value @ 0..=1) if value == triggering_value => {
                    Some((status, message.to_owned()))
                }
                Ok(0..=1) => None,
                _ => {
                    let message = format!("invalid {suffix} flag: expected 0 or 1, got {text:?}");
                    warn(warnings, &flag_path, message.clone());
                    Some((Status::Invalid, message))
                }
            },
            Err(error) => {
                warn(warnings, &flag_path, error.to_string());
                Some((classify(&error), format!("{suffix} flag: {error}")))
            }
        };
        if let Some((status, message)) = problem {
            // Preserve an input failure or an earlier flag failure as the primary cause.
            if !primary_problem_recorded {
                sensor.status = status;
                sensor.error = Some(message);
                primary_problem_recorded = true;
            }
        }
    }
    sensor
}

fn numbered(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix).is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn entries(path: &Path, warnings: &mut Vec<Warning>) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(path)? {
        match entry {
            Ok(entry) => paths.push(entry.path()),
            Err(error) => warn(warnings, path, error.to_string()),
        }
    }
    paths.sort();
    Ok(paths)
}

fn optional_text(
    path: &Path,
    read_text: &impl Fn(&Path) -> io::Result<String>,
) -> io::Result<Option<String>> {
    match read_text(path) {
        Ok(text) => Ok(Some(text.trim().to_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // A dangling symlink is a failed attribute, not absent optional metadata.
            match fs::symlink_metadata(path) {
                Err(missing) if missing.kind() == io::ErrorKind::NotFound => Ok(None),
                _ => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn classify(error: &io::Error) -> Status {
    // Linux ENODEV and ENODATA both mean no current measurement is available.
    if matches!(error.raw_os_error(), Some(19 | 61)) {
        return Status::Unavailable;
    }
    match error.kind() {
        io::ErrorKind::NotFound => Status::Unavailable,
        io::ErrorKind::PermissionDenied => Status::PermissionDenied,
        io::ErrorKind::Unsupported => Status::Unsupported,
        io::ErrorKind::InvalidData => Status::Invalid,
        _ => Status::Error,
    }
}

fn warn(warnings: &mut Vec<Warning>, path: &Path, message: String) {
    warnings.push(Warning {
        path: path.to_path_buf(),
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inject I/O errors that ordinary filesystem fixtures cannot reliably produce.
    fn read_with_flags(
        attribute: &str,
        input: Result<&str, io::ErrorKind>,
        enable: Result<&str, io::ErrorKind>,
        fault: Result<&str, io::ErrorKind>,
    ) -> (Sensor, Vec<Warning>) {
        let device_path = Path::new("/fixture/hwmon0");
        let mut warnings = Vec::new();
        let sensor = read_sensor(
            "fixture",
            device_path,
            &device_path.join(attribute),
            &mut warnings,
            |path| {
                let filename = path.file_name().unwrap().to_str().unwrap();
                let result = if filename.ends_with("_input") {
                    input
                } else if filename.ends_with("_enable") {
                    enable
                } else if filename.ends_with("_fault") {
                    fault
                } else {
                    assert!(filename.ends_with("_label"));
                    Ok("CPU package")
                };
                result
                    .map(str::to_owned)
                    .map_err(|kind| io::Error::new(kind, format!("injected {filename} read error")))
            },
        );
        (sensor, warnings)
    }

    #[test]
    fn input_read_errors_remain_primary_when_flags_report_problems() {
        for (kind, status) in [
            (io::ErrorKind::Unsupported, Status::Unsupported),
            (io::ErrorKind::PermissionDenied, Status::PermissionDenied),
            (io::ErrorKind::NotFound, Status::Unavailable),
            (io::ErrorKind::InvalidData, Status::Invalid),
            (io::ErrorKind::Other, Status::Error),
        ] {
            for (enable, fault, warning_count) in [
                (Ok("0"), Ok("1"), 0),
                (Ok("1"), Ok("1"), 0),
                (Ok("invalid"), Ok("2"), 2),
                (
                    Err(io::ErrorKind::Unsupported),
                    Err(io::ErrorKind::PermissionDenied),
                    2,
                ),
                (Ok("1"), Err(io::ErrorKind::Unsupported), 1),
            ] {
                let (sensor, warnings) = read_with_flags("temp1_input", Err(kind), enable, fault);
                assert_eq!(sensor.status, status, "{kind:?}, {enable:?}, {fault:?}");
                assert_eq!(
                    sensor.error.as_deref(),
                    Some("injected temp1_input read error")
                );
                assert_eq!(sensor.raw_text, None);
                assert_eq!(sensor.raw_value, None);
                assert_eq!(sensor.value, None);
                assert_eq!(sensor.unit, Some("°C"));
                assert_eq!(sensor.device, "fixture");
                assert_eq!(sensor.device_path, Path::new("/fixture/hwmon0"));
                assert_eq!(sensor.attribute, "temp1_input");
                assert_eq!(sensor.label.as_deref(), Some("CPU package"));
                assert_eq!(sensor.source_path, Path::new("/fixture/hwmon0/temp1_input"));
                assert_eq!(warnings.len(), warning_count);
            }
        }
    }

    #[test]
    fn first_unsupported_flag_error_remains_primary_after_successful_input() {
        for attribute in ["temp1_input", "power1_input"] {
            for (fault, warning_count) in [
                (Ok("1"), 1),
                (Ok("2"), 2),
                (Err(io::ErrorKind::PermissionDenied), 2),
            ] {
                let (sensor, warnings) = read_with_flags(
                    attribute,
                    Ok(" 42000\n"),
                    Err(io::ErrorKind::Unsupported),
                    fault,
                );
                let channel = attribute.strip_suffix("_input").unwrap();
                assert_eq!(sensor.status, Status::Unsupported);
                assert_eq!(
                    sensor.error,
                    Some(format!("enable flag: injected {channel}_enable read error"))
                );
                assert_eq!(sensor.raw_text.as_deref(), Some("42000"));
                assert_eq!(sensor.raw_value, Some(42000));
                assert_eq!(sensor.value, (attribute == "temp1_input").then_some(42.0));
                assert_eq!(sensor.unit, (attribute == "temp1_input").then_some("°C"));
                assert_eq!(warnings.len(), warning_count);
                assert_eq!(
                    warnings[0].path,
                    Path::new("/fixture/hwmon0").join(format!("{channel}_enable"))
                );
                assert_eq!(
                    warnings[0].message,
                    format!("injected {channel}_enable read error")
                );
            }
        }
    }

    #[test]
    fn classifies_io_failures_without_requiring_privileges() {
        for (error, expected) in [
            (
                io::Error::from(io::ErrorKind::PermissionDenied),
                Status::PermissionDenied,
            ),
            (
                io::Error::from(io::ErrorKind::NotFound),
                Status::Unavailable,
            ),
            (io::Error::from(io::ErrorKind::InvalidData), Status::Invalid),
            (
                io::Error::from(io::ErrorKind::Unsupported),
                Status::Unsupported,
            ),
            (io::Error::from_raw_os_error(19), Status::Unavailable),
            (io::Error::from_raw_os_error(61), Status::Unavailable),
            (io::Error::from_raw_os_error(5), Status::Error),
        ] {
            assert_eq!(classify(&error), expected);
        }
    }
}
