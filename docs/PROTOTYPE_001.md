# Alpha 1, Prototype 001

This document describes the accepted Prototype 001 checkpoint. For the current identity fields and experimental JSON schema version 2, see [Prototype 002](milestones/002-stable-sensor-identity.md). The commands below remain supported.

This prototype reads one snapshot from Linux `/sys/class/hwmon`. Its purpose is to prove discovery and parsing of real sensor data before building a daemon, API, or GUI. It does not implement those components or authorize protective actions.

## Build and run

Use Linux and Rust 1.85 or newer. Run as your normal user; inaccessible channels are reported rather than requiring root.

```bash
cargo run -- sensors
cargo run -- sensors --json
cargo run -- --help
```

For an isolated fixture or captured hwmon directory:

```bash
cargo run -- sensors --hwmon-root /path/to/fixture
cargo run -- sensors --json --hwmon-root /path/to/fixture
```

The optional root changes only the directory being read. The program never creates or writes sensor attributes. It performs a single scan and exits.

## Supported inputs

| Attribute | Raw unit | Display unit | Conversion |
| --- | --- | --- | --- |
| `tempN_input` | Millidegrees Celsius | °C | Divide by 1,000 |
| `fanN_input` | Revolutions per minute | RPM | Unchanged |
| `inN_input` | Millivolts | V | Divide by 1,000 |

Discovery follows existing filenames, including sparse channel numbering and voltage channel zero. Optional `*_label` attributes provide names; the input filename is the fallback. Zero RPM and negative temperatures are valid numeric readings.

These are the [standard Linux hwmon ABI units](https://docs.kernel.org/hwmon/sysfs-interface.html). Voltage is generally measured at the chip pin; board-specific resistor scaling and calibration are not applied. Some drivers expose exceptional thermistor inputs needing driver-specific conversion, which this prototype does not implement. A thermistor type alone cannot identify that exception. Do not use this output for protection or assume it reproduces libsensors' calibrated results.

Other `*_input` attributes remain visible as `unsupported`, with their raw values retained when readable. Average, limit, alarm, PWM, and configuration attributes are not treated as input measurements. Optional `*_fault` and `*_enable` flags qualify a channel's status; their absence does not establish sensor health.

## Output and errors

The terminal table includes device, sensor, converted value, and status. Device and attribute ordering is deterministic. Names and diagnostic messages have terminal control characters escaped.

JSON contains an experimental `schema_version: 1`, the scanned `root`, a flat `sensors` array, and discovery/metadata `warnings`. Each sensor retains:

- Device name and resolved device directory when available.
- Input attribute, optional label, and original source path.
- Trimmed raw text, parsed `raw_value`, converted `value`, and `unit`.
- A `status` and optional diagnostic `error`.

| Status | Meaning |
| --- | --- |
| `ok` | Read and parsed using the standard unit conversion; **not physically validated** |
| `unavailable` | Missing/disappeared input, unavailable device data, or disabled channel |
| `permission_denied` | The current user cannot read a required attribute |
| `unsupported` | Input family or filesystem operation is unsupported |
| `invalid` | Malformed, overflowing, or invalid numeric/flag content |
| `fault` | The driver reports a sensor fault |
| `error` | Another read error |

Failed reads are never replaced with zero. Raw data may remain present for a disabled or faulted channel; inspect `status` before consuming `value`. Device names, labels, and directory entries can fail independently without aborting other devices. A device with no `*_input` entries contributes no sensor rows.

An empty, readable root produces an empty snapshot. Failure to enumerate the root exits with status 1; JSON mode emits a JSON error object. Invalid CLI usage exits with status 2. Completed scans, including partial channel failures, exit with status 0, so consumers must inspect individual statuses and warnings. JSON is written to stdout without a table or progress messages; Cargo itself may print build messages to stderr.

This JSON format is a starting point for experimentation, not a stabilized service API. Paths and `hwmonN` indices are discovery evidence, not persistent identities. JSON includes local device paths and raw labels; it is not a privacy-redacted diagnostic report.

Snapshot paths must be representable as UTF-8. Otherwise JSON mode returns an error object and exit status 1 instead of silently changing source identity or emitting partial JSON. Error objects use a lossy display of the root path for diagnostics only.

## Example

Illustrative fixture output; available devices, labels, and readings vary by machine:

```text
SiliconWatch - Hardware Sensors

DEVICE    SENSOR          VALUE  STATUS
coretemp  Package id 0   42.5 °C  ok
nct6798   CPU Fan      1240 RPM   ok
nct6798   Vcore          1.152 V  ok

3 sensors discovered; 3 with status ok.
Values are unvalidated hardware reports; 'ok' means read and parsed.
```

## Development checks

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- sensors
cargo run -- sensors --json
```

Tests use temporary, isolated directories, including malformed inputs, missing attributes, flags, and symlinks. They do not write to `/sys` or depend on the development machine's sensors. Permission-denied filesystem checks apply when the test user actually lacks permission; error classification is also tested independently.

The only direct dependencies are `serde` and `serde_json`, for typed, correctly escaped JSON. CLI parsing and filesystem fixtures use the standard library. `Cargo.lock` is tracked.

## Limits and next steps

This is a sequential, best-effort snapshot, not an atomic hardware sample. It does not validate physical accuracy, infer safe thresholds, apply board calibration, preserve identity across boots, or continuously poll. In particular, an implausible numeric reading can still have status `ok` when the driver reports no fault.

No daemon, local or remote API, GUI, database, plugin system, hardware control, or shutdown path is included. Local API permissions remain pending in [Decision 43](DESIGN_DECISIONS.md#43-local-api-permissions--pending).

Prototype 001 is accepted for Alpha 1 development. See the [milestone report](milestones/001-hwmon-prototype.md) for final verification and the proposed identity-only Prototype 002 scope. The next milestone requires Chris's review before implementation; the [design decisions](DESIGN_DECISIONS.md) remain the broader project direction.
