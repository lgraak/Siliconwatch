use siliconwatch::hwmon::{self, Snapshot, Status};
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "Usage: siliconwatch sensors [--json] [--hwmon-root PATH]
       siliconwatch --help
       siliconwatch --version

Read one snapshot of Linux hwmon sensors without changing hardware settings.
--json             Emit an experimental JSON snapshot instead of a table.
--hwmon-root PATH  Read an alternate hwmon directory (default: /sys/class/hwmon).";

enum Command {
    Sensors { json: bool, root: PathBuf },
    Help,
    Version,
}

fn parse_args() -> Result<Command, String> {
    let mut args = env::args_os().skip(1);
    let first = args.next().ok_or("expected the sensors command")?;
    if first == "--help" || first == "-h" {
        return Ok(Command::Help);
    }
    if first == "--version" {
        return Ok(Command::Version);
    }
    if first != "sensors" {
        return Err(format!("unknown command: {}", first.to_string_lossy()));
    }
    let mut json = false;
    let mut root = PathBuf::from("/sys/class/hwmon");
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--json") => json = true,
            Some("--hwmon-root") => {
                let path = args.next().ok_or("--hwmon-root requires a path")?;
                if path.is_empty() || path.to_str().is_some_and(|s| s.starts_with("--")) {
                    return Err("--hwmon-root requires a path".into());
                }
                root = path.into();
            }
            Some("--help" | "-h") => return Ok(Command::Help),
            _ => return Err(format!("unknown option: {}", arg.to_string_lossy())),
        }
    }
    Ok(Command::Sensors { json, root })
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

fn write_table(out: &mut impl Write, snapshot: &Snapshot) -> io::Result<()> {
    let rows: Vec<_> = snapshot
        .sensors
        .iter()
        .map(|sensor| {
            let value = match (sensor.value, sensor.unit) {
                (Some(value), Some("°C")) => format!("{value:.1} °C"),
                (Some(value), Some("V")) => format!("{value:.3} V"),
                (Some(value), Some("RPM")) => format!("{value:.0} RPM"),
                _ => "—".to_owned(),
            };
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

fn run() -> io::Result<ExitCode> {
    let command = match parse_args() {
        Ok(command) => command,
        Err(error) => {
            eprintln!("siliconwatch: {}\n\n{USAGE}", terminal_text(&error));
            return Ok(ExitCode::from(2));
        }
    };
    let mut out = io::stdout().lock();
    match command {
        Command::Help => writeln!(out, "{USAGE}")?,
        Command::Version => writeln!(out, "siliconwatch {}", env!("CARGO_PKG_VERSION"))?,
        Command::Sensors { json, root } => match hwmon::scan(&root) {
            Ok(snapshot) if json => match serde_json::to_vec_pretty(&snapshot) {
                // Buffer serialization so an unencodable path cannot leave partial JSON.
                Ok(bytes) => {
                    out.write_all(&bytes)?;
                    writeln!(out)?;
                }
                Err(error) => {
                    write_json_error(&mut out, &root, "serialization_error", &error.to_string())?;
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
        },
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
