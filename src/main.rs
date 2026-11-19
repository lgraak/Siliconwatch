use siliconwatch::hwmon::{Snapshot, Status};
use siliconwatch::{daemon, sampling};
use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "Usage: siliconwatch sensors [--json] [--hwmon-root PATH]
       siliconwatch sample --count N --interval-ms M [--json-lines] [--hwmon-root PATH]
       siliconwatch daemon --interval-ms M --stale-after-ms M [--hwmon-root PATH] [--local-api]
       siliconwatch --help
       siliconwatch --version

Read Linux hwmon sensors without changing hardware settings.
--json             Emit an experimental JSON snapshot instead of a table.
--hwmon-root PATH  Read an alternate hwmon directory (default: /sys/class/hwmon).

sample options:
--count N          Required number of attempts, 1-10000 (failures count).
--interval-ms M    Required start-to-start interval, 100-60000 milliseconds.
--json-lines       Emit one experimental JSON record per attempt and flush it.

Sampling completes with exit 0 even when attempts report recoverable errors.
Output failures (including broken pipes) stop sampling with exit 1.
Interruptions can leave an incomplete stream; no hard runtime deadline is promised.

daemon options:
--interval-ms M    Required foreground acquisition interval, 100-60000 milliseconds.
--stale-after-ms M Required positive observation-age budget in milliseconds.
--local-api        Enable owner-only Linux JSON API in $XDG_RUNTIME_DIR/siliconwatch/api.sock.

Daemon runs in the foreground; SIGINT/SIGTERM request cooperative shutdown.
It writes lifecycle notices to stderr, uses no stdout telemetry, and requires no root.
Blocked drivers or output can delay shutdown; no forced shutdown is performed.";

enum Command {
    Sensors {
        json: bool,
        root: PathBuf,
    },
    Sample {
        json_lines: bool,
        root: PathBuf,
        options: sampling::Options,
    },
    Daemon {
        root: PathBuf,
        options: daemon::Options,
    },
    Help,
    Version,
}

fn parse_args(mut args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let first = args
        .next()
        .ok_or("expected the sensors, sample or daemon command")?;
    if first == "--help" || first == "-h" {
        return Ok(Command::Help);
    }
    if first == "--version" {
        return Ok(Command::Version);
    }
    if first == "daemon" {
        return parse_daemon_args(args);
    }
    let sampling = first == "sample";
    if first != "sensors" && !sampling {
        return Err(format!("unknown command: {}", first.to_string_lossy()));
    }
    let mut json = false;
    let mut count = None;
    let mut interval_ms = None;
    let mut root_seen = false;
    let mut root = PathBuf::from("/sys/class/hwmon");
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--json") if !sampling => json = true,
            Some("--json-lines") if sampling => {
                if json {
                    return Err("duplicate --json-lines".into());
                }
                json = true;
            }
            Some("--count") if sampling => {
                if count.is_some() {
                    return Err("duplicate --count".into());
                }
                count = Some(
                    u32::try_from(decimal_argument(args.next(), "--count")?)
                        .map_err(|_| "--count is too large")?,
                );
            }
            Some("--interval-ms") if sampling => {
                if interval_ms.is_some() {
                    return Err("duplicate --interval-ms".into());
                }
                interval_ms = Some(decimal_argument(args.next(), "--interval-ms")?);
            }
            Some("--hwmon-root") => {
                if sampling && root_seen {
                    return Err("duplicate --hwmon-root".into());
                }
                let path = args.next().ok_or("--hwmon-root requires a path")?;
                if path.is_empty() || path.to_str().is_some_and(|s| s.starts_with("--")) {
                    return Err("--hwmon-root requires a path".into());
                }
                root = path.into();
                root_seen = true;
            }
            Some("--help" | "-h") => return Ok(Command::Help),
            _ => return Err(format!("unknown option: {}", arg.to_string_lossy())),
        }
    }
    if sampling {
        let options = sampling::Options::new(
            count.ok_or("sample requires --count")?,
            interval_ms.ok_or("sample requires --interval-ms")?,
        )?;
        Ok(Command::Sample {
            json_lines: json,
            root,
            options,
        })
    } else {
        Ok(Command::Sensors { json, root })
    }
}

fn parse_daemon_args(mut args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut interval_ms = None;
    let mut stale_after_ms = None;
    let mut root = None;
    let mut local_api = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--interval-ms") => {
                if interval_ms.is_some() {
                    return Err("duplicate --interval-ms".into());
                }
                interval_ms = Some(decimal_argument(args.next(), "--interval-ms")?);
            }
            Some("--stale-after-ms") => {
                if stale_after_ms.is_some() {
                    return Err("duplicate --stale-after-ms".into());
                }
                stale_after_ms = Some(decimal_argument(args.next(), "--stale-after-ms")?);
            }
            Some("--local-api") => {
                if local_api {
                    return Err("duplicate --local-api".into());
                }
                local_api = true;
            }
            Some("--hwmon-root") => {
                if root.is_some() {
                    return Err("duplicate --hwmon-root".into());
                }
                let path = args.next().ok_or("--hwmon-root requires a path")?;
                if path.is_empty() || path.to_str().is_some_and(|s| s.starts_with("--")) {
                    return Err("--hwmon-root requires a path".into());
                }
                root = Some(PathBuf::from(path));
            }
            Some("--help" | "-h") => return Ok(Command::Help),
            _ => return Err(format!("unknown option: {}", arg.to_string_lossy())),
        }
    }
    Ok(Command::Daemon {
        root: root.unwrap_or_else(|| PathBuf::from("/sys/class/hwmon")),
        options: daemon::Options::new(
            interval_ms.ok_or("daemon requires --interval-ms")?,
            stale_after_ms.ok_or("daemon requires --stale-after-ms")?,
        )?
        .with_local_api(local_api),
    })
}

fn decimal_argument(value: Option<OsString>, option: &str) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("{option} requires a decimal integer"))?;
    let text = value
        .to_str()
        .filter(|s| !s.is_empty() && s.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| format!("{option} requires a decimal integer"))?;
    text.parse().map_err(|_| format!("{option} is too large"))
}

// Keep labels and filesystem error text from inserting control sequences in the terminal.
fn terminal_text(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

fn table_value(value: Option<f64>, unit: Option<&str>) -> String {
    match (value, unit) {
        (Some(value), Some("°C")) => format!("{value:.1} °C"),
        (Some(value), Some("V")) => format!("{value:.3} V"),
        (Some(value), Some(unit @ ("RPM" | "MHz" | "%"))) => format!("{value:.0} {unit}"),
        (Some(value), Some("B")) => format!("{:.2} MiB", value / 1_048_576.0),
        (Some(value), Some("W")) => format!("{value:.3} W"),
        (Some(value), Some("P-state")) => format!("P{value:.0}"),
        _ => "—".to_owned(),
    }
}

fn write_table(out: &mut impl Write, snapshot: &Snapshot) -> io::Result<()> {
    let rows: Vec<_> = snapshot
        .sensors
        .iter()
        .map(|sensor| {
            let value = table_value(sensor.value, sensor.unit);
            (
                terminal_text(&sensor.device),
                terminal_text(sensor.label.as_deref().unwrap_or(&sensor.attribute)),
                value,
                sensor.status.as_str(),
                sensor.identity.status.as_str(),
            )
        })
        .collect();
    let device_width = rows
        .iter()
        .map(|r| r.0.chars().count())
        .max()
        .unwrap_or(0)
        .max(6);
    let sensor_width = rows
        .iter()
        .map(|r| r.1.chars().count())
        .max()
        .unwrap_or(0)
        .max(6);
    let value_width = rows
        .iter()
        .map(|r| r.2.chars().count())
        .max()
        .unwrap_or(0)
        .max(5);
    let status_width = rows.iter().map(|r| r.3.len()).max().unwrap_or(0).max(6);
    let identity_width = rows.iter().map(|r| r.4.len()).max().unwrap_or(0).max(8);
    writeln!(out, "SiliconWatch - Hardware Sensors\n")?;
    writeln!(
        out,
        "{:<device_width$}  {:<sensor_width$}  {:>value_width$}  {:<status_width$}  IDENTITY",
        "DEVICE", "SENSOR", "VALUE", "STATUS"
    )?;
    writeln!(
        out,
        "{}",
        "-".repeat(device_width + sensor_width + value_width + status_width + identity_width + 8)
    )?;
    for (device, sensor, value, status, identity) in rows {
        writeln!(
            out,
            "{device:<device_width$}  {sensor:<sensor_width$}  {value:>value_width$}  {status:<status_width$}  {identity}"
        )?;
    }
    let ok = snapshot
        .sensors
        .iter()
        .filter(|s| s.status == Status::Ok)
        .count();
    writeln!(
        out,
        "\n{} sensors discovered; {ok} with status ok.",
        snapshot.sensors.len()
    )?;
    writeln!(
        out,
        "Values are unvalidated hardware reports; 'ok' means read and parsed."
    )?;
    for sensor in &snapshot.sensors {
        if let Some(error) = &sensor.error {
            writeln!(
                out,
                "{}: {}",
                terminal_text(&sensor.source_path.to_string_lossy()),
                terminal_text(error)
            )?;
        }
        writeln!(
            out,
            "Identity v{} {}: {}",
            sensor.identity.schema_version,
            terminal_text(&sensor.source_path.to_string_lossy()),
            sensor
                .identity
                .candidate
                .as_deref()
                .map(terminal_text)
                .unwrap_or_else(|| "no candidate".into())
        )?;
        for issue in &sensor.identity.issues {
            writeln!(out, "  Identity: {}", terminal_text(issue))?;
        }
    }
    for warning in &snapshot.warnings {
        writeln!(
            out,
            "Warning: {}: {}",
            terminal_text(&warning.path.to_string_lossy()),
            terminal_text(&warning.message)
        )?;
    }
    Ok(())
}

fn write_json_error(
    out: &mut impl Write,
    root: &Path,
    kind: &str,
    message: &str,
) -> io::Result<()> {
    serde_json::to_writer_pretty(
        &mut *out,
        &serde_json::json!({
            "schema_version": 2,
            "root": root.to_string_lossy(),
            "error": { "kind": kind, "message": message }
        }),
    )?;
    writeln!(out)
}

fn write_sample_table(out: &mut impl Write, record: sampling::Record) -> io::Result<()> {
    let timing = &record.timing;
    writeln!(
        out,
        "Sample {}/{}; interval {} ms",
        record.sequence, record.requested_count, record.interval_ms
    )?;
    writeln!(
        out,
        "Monotonic offsets (ns): target {}, start {}, finish {}, prepared {}",
        timing.target_start_ns, timing.started_ns, timing.finished_ns, timing.prepared_ns
    )?;
    writeln!(
        out,
        "Scan duration {} ns; start lateness {} ns; wall start {} Unix ms",
        timing.scan_duration_ns,
        timing.lateness_ns,
        timing
            .wall_start_unix_ms
            .map(|stamp| stamp.to_string())
            .unwrap_or_else(|| "unavailable".into())
    )?;
    if let Some(error) = &timing.wall_time_error {
        writeln!(out, "Wall clock: {}", terminal_text(error))?;
    }
    let freshness = &record.freshness;
    writeln!(out, "Observation basis: {}", freshness.basis)?;
    if let (Some(lower), Some(upper)) = (freshness.lower_bound_ns, freshness.upper_bound_ns) {
        writeln!(
            out,
            "Observation-age bounds at preparation: {lower}..{upper} ns; window exceeds interval: {}",
            freshness
                .observation_window_exceeds_interval
                .unwrap_or(false)
        )?;
    }
    writeln!(
        out,
        "Physical sensor age is unknown; preparation precedes serialization and output."
    )?;
    if let Some(snapshot) = &record.snapshot {
        write_table(out, snapshot)?;
    }
    if let Some(error) = &record.error {
        writeln!(
            out,
            "Attempt error ({}): {}",
            terminal_text(&error.kind),
            terminal_text(&error.message)
        )?;
    }
    writeln!(out)?;
    out.flush()
}

fn run() -> io::Result<ExitCode> {
    let mut args = env::args_os().skip(1).peekable();
    let daemon_requested = args.peek().is_some_and(|arg| arg == "daemon");
    let command = match parse_args(args) {
        Ok(command) => command,
        Err(error) => {
            if daemon_requested {
                let reported = writeln!(
                    io::stderr().lock(),
                    "siliconwatch: {}\n\n{USAGE}",
                    terminal_text(&error)
                );
                return Ok(if reported.is_ok() {
                    ExitCode::from(2)
                } else {
                    ExitCode::FAILURE
                });
            }
            eprintln!("siliconwatch: {}\n\n{USAGE}", terminal_text(&error));
            return Ok(ExitCode::from(2));
        }
    };
    match command {
        Command::Daemon { root, options } => {
            if let Err(error) = daemon::run(&root, options) {
                // Daemon reporting failures must never use the one-shot broken-pipe exception.
                let _ = writeln!(
                    io::stderr().lock(),
                    "siliconwatch: {}",
                    terminal_text(&error.to_string())
                );
                return Ok(ExitCode::FAILURE);
            }
        }
        Command::Help => writeln!(io::stdout().lock(), "{USAGE}")?,
        Command::Version => writeln!(
            io::stdout().lock(),
            "siliconwatch {}",
            env!("CARGO_PKG_VERSION")
        )?,
        Command::Sample {
            json_lines,
            root,
            options,
        } => {
            let mut out = io::stdout().lock();
            let result = sampling::run(&root, options, |record| {
                if json_lines {
                    sampling::write_json_line(record, &mut out)
                } else {
                    write_sample_table(&mut out, record)
                }
            });
            // Sampling failures must not enter the one-shot broken-pipe success path.
            if let Err(error) = result {
                eprintln!("siliconwatch: {}", terminal_text(&error.to_string()));
                return Ok(ExitCode::FAILURE);
            }
        }
        Command::Sensors { json, root } => {
            let mut out = io::stdout().lock();
            match siliconwatch::scan(&root) {
                Ok(snapshot) if json => match serde_json::to_vec_pretty(&snapshot) {
                    // Buffer serialization so an unencodable path cannot leave partial JSON.
                    Ok(bytes) => {
                        out.write_all(&bytes)?;
                        writeln!(out)?;
                    }
                    Err(error) => {
                        write_json_error(
                            &mut out,
                            &root,
                            "serialization_error",
                            &error.to_string(),
                        )?;
                        return Ok(ExitCode::FAILURE);
                    }
                },
                Ok(snapshot) => write_table(&mut out, &snapshot)?,
                Err(error) => {
                    if json {
                        write_json_error(
                            &mut out,
                            &root,
                            &format!("{:?}", error.kind()),
                            &error.to_string(),
                        )?;
                    } else {
                        eprintln!(
                            "siliconwatch: cannot scan {}: {}",
                            terminal_text(&root.to_string_lossy()),
                            terminal_text(&error.to_string())
                        );
                    }
                    return Ok(ExitCode::FAILURE);
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("siliconwatch: {}", terminal_text(&error.to_string()));
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn table_formats_gpu_metrics_and_keeps_unavailable_values_empty() {
        for (value, unit, expected) in [
            (7.0, "%", "7 %"),
            (1_572_864.0, "B", "1.50 MiB"),
            (734.0, "MHz", "734 MHz"),
            (32.75, "W", "32.750 W"),
            (3.0, "P-state", "P3"),
        ] {
            assert_eq!(super::table_value(Some(value), Some(unit)), expected);
            assert_eq!(super::table_value(None, Some(unit)), "—");
        }
    }

    use super::*;

    #[test]
    fn sample_accepts_numeric_bounds_without_executing_them() {
        for count in ["1", "10000"] {
            for interval in ["100", "60000"] {
                assert!(matches!(
                    parse_args(
                        ["sample", "--count", count, "--interval-ms", interval]
                            .into_iter()
                            .map(OsString::from)
                    ),
                    Ok(Command::Sample { .. })
                ));
            }
        }
    }
    #[test]
    fn daemon_accepts_numeric_bounds_without_executing_them() {
        for interval in ["100", "60000"] {
            for freshness in ["1", "18446744073709551615"] {
                assert!(matches!(
                    parse_args(
                        [
                            "daemon",
                            "--interval-ms",
                            interval,
                            "--stale-after-ms",
                            freshness
                        ]
                        .into_iter()
                        .map(OsString::from)
                    ),
                    Ok(Command::Daemon { .. })
                ));
            }
        }
    }
}
