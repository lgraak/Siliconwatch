<p align="center">
  <img src="assets/branding/siliconwatch-icon.png" alt="SiliconWatch — a chip and heartbeat inside a shield" width="200">
</p>

# SiliconWatch

**Linux hardware monitoring, built in Rust.**

SiliconWatch is an experimental Linux hardware-monitoring application with a native desktop GUI and a CLI/headless monitoring tool. It reads the temperatures, voltages and fan speeds exposed by Linux drivers, keeps unavailable or questionable readings explicit, and leaves hardware settings unchanged.

The dense, HWiNFO64-inspired sensor view is built for users who want familiar, detailed monitoring on Linux. A separate foreground daemon owns acquisition; the GUI reads its cache through a local read-only API.

**Status: Alpha 1 candidate — Prototype 010.** Chris has conditionally authorized experimental external alpha testing after the verification and publication gates pass. Documented qualification gaps remain. This is read-only experimental monitoring, not production-ready software or safety-critical hardware protection.

**Public alpha download coming soon.** The intended version is `v0.1.0-alpha.1`. No public alpha download is available yet; follow the [GitHub repository](https://github.com/lgraak/Siliconwatch) and [project website](https://siliconwatch.cgillett.com/) for updates.

![Native SiliconWatch OLED Black sensor table with Current, Minimum, Maximum and Average columns](docs/milestones/010-native-gui/main-oled.png)

*Actual native OLED Black GUI with demonstration fixture readings. This is a running application capture, not a design mockup or a claim about a particular machine's sensors.*

## Implemented in the Alpha 1 candidate

- **Native Slint desktop GUI:** OLED Black by default, plus Dark and Light themes; compact hierarchical provider/device groups; keyboard navigation; configurable fonts, row density and columns.
- **Temperatures, voltages and fan RPM:** CPU/GPU temperatures where exposed by Linux, NVMe and other available hwmon temperatures, voltage inputs and fan-speed inputs. Coverage depends on the machine and its drivers.
- **Current, Minimum, Maximum and Average:** session statistics over distinct valid observations received by the GUI. They are not time-weighted or persistent history. Repeated cache reads, redraws and unit changes do not add samples.
- **Device Details:** a reusable native window shows the selected source's available readings, raw and canonical values, identity evidence, diagnostics and observation timing. Unsupported, failed, stale and historical states remain visible.
- **CLI and foreground daemon:** one-shot tables/JSON, bounded sampling, cooperative shutdown and an opt-in, current-user Unix-socket API with `get_info` and `get_snapshot`. GUI refresh does not trigger another hardware scan.
- **Desktop integration and portable preferences:** a StatusNotifier-compatible tray, versioned TOML appearance/layout settings with reviewed import/export, and Celsius/Fahrenheit display preferences. Conversion changes presentation only; raw readings remain intact.

Minimize to Tray is **off by default**. Closing Sensors exits the GUI; enabling the preference keeps the same session running while a usable tray host is available. Quit always exits the GUI and leaves the separately started daemon running.

<a id="try-the-prototype"></a>

## Build and run from source

The GUI commands below apply to a **Prototype 010 candidate checkout containing `gui/`**. Check the checkout and release status before using them; these source instructions do not imply an available alpha release or installer.

Use Linux and an installed Rust 1.85.0 toolchain. The GUI also needs a working desktop, OpenGL, and the platform graphics/font libraries used by its native backend. Distribution-specific prerequisites are not installed automatically. Slint and slint-build remain pinned to 1.13.1; Rust 1.85 compatibility uses the documented [fontdue 0.9.5 local patch](vendor/fontdue-0.9.5-PATCH.md).

From the candidate repository root, build the CLI/daemon and GUI:

```sh
cargo +1.85.0 build --locked --release --workspace
```

For a headless-only build:

```sh
cargo +1.85.0 build --locked --release -p siliconwatch
```

### CLI readings

List currently exposed sensors or emit a JSON snapshot:

```sh
cargo +1.85.0 run --locked --release -p siliconwatch -- sensors
cargo +1.85.0 run --locked --release -p siliconwatch -- sensors --json
```

Take five acquisition attempts with JSON-lines output:

```sh
cargo +1.85.0 run --locked --release -p siliconwatch -- sample --count 5 --interval-ms 1000 --json-lines
```

Run these as your normal user. CLI acquisition is independent of the daemon; use the daemon/GUI workflow below for shared monitoring. Sampling exit 0 means the requested attempts completed, even when individual attempts contain recoverable errors; inspect each record. See [sampling behavior and limitations](docs/milestones/003-bounded-foreground-sampling.md) and [best-effort source identities](docs/milestones/002-stable-sensor-identity.md).

### Foreground daemon and local API

For headless acquisition without an API listener:

```sh
cargo +1.85.0 run --locked --release -p siliconwatch -- daemon --interval-ms 1000 --stale-after-ms 3000
```

To connect the GUI, start the daemon **with `--local-api` instead**:

```sh
cargo +1.85.0 run --locked --release -p siliconwatch -- daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

Use one daemon instance. It stays in the foreground and writes lifecycle notices to stderr. Ctrl-C or SIGTERM requests cooperative shutdown; blocked drivers or output can delay completion. No service is installed. See the [daemon lifecycle report](docs/milestones/008-headless-daemon-lifecycle.md).

The API listens at `$XDG_RUNTIME_DIR/siliconwatch/api.sock`. The existing runtime directory must be private and owned by your non-root user. The daemon checks directory permissions and peer identity; neither it nor the GUI bypasses permissions, takes over another instance or automatically repairs stale endpoints. Do not run it through `sudo`.

Only `get_info` and `get_snapshot` are exposed. The latter reads the existing cache; it does not scan hardware. This is a versioned JSON protocol over a local Unix socket, not an HTTP service. See the [API contract and client example](docs/api/local-v1.md) and [Prototype 009 report](docs/milestones/009-local-readonly-api.md).

### Launch the GUI

With the API-enabled daemon running, use another terminal:

```sh
cargo +1.85.0 run --locked --release -p siliconwatch-gui -- --refresh-ms 1000
```

The GUI connects automatically to the current user's socket. **Reconnect** retries under the same permission checks and rate limits; it never requests a hardware scan. For a slower daemon acquisition interval, choose a GUI refresh interval at least as long. GUI refresh accepts 1000–60000 milliseconds and defaults to 1000.

Double-click a sensor, select it and press Enter, or choose **Open Details**. See the [GUI guide](docs/GUI.md) for keyboard controls, tray behavior, freshness labels, display preferences and recovery.

## Import and export appearance settings

These are GUI actions, not CLI subcommands:

1. Open **Settings → Import, Export & Recovery**.
2. Enter a full absolute file path. There is no native file picker in this candidate.
3. Choose **Export Settings…** or **Import Settings…**, review the preview, then confirm or cancel.

Leave **Include device group preferences in export** off for an appearance-only profile. Optional device-group selectors can reveal source topology and require review when imported. Profiles do not contain sensor history, session statistics, credentials, API access policy or hardware controls.

Settings are stored at `$XDG_CONFIG_HOME/siliconwatch/config.toml`, falling back to `~/.config/siliconwatch/config.toml`. Invalid files remain untouched; the GUI reports recovery/unsaved state. Importing appearance preferences does not reset session statistics or change daemon behavior. Full import, export, backup and recovery steps are in the [GUI guide](docs/GUI.md#portable-settings).

## Hardware compatibility

SiliconWatch consumes measurements exposed by Linux monitoring drivers. Available readings depend on hardware, firmware and kernel-driver support. A familiar source name is not proof of physical component identity, and missing fan RPM can indicate missing driver support rather than an application defect.

Some Super I/O monitoring chips need additional drivers. SiliconWatch **does not install or load kernel modules**. Third-party modules can introduce kernel-version compatibility, signature/trust and ongoing maintenance requirements; a configuration that works on one machine is not a universal recommendation.

For example, on a tested **GMKtec EVO-X1 with an IT8613E monitoring chip**, fan RPM became available after Linux exposed the channels through an already-installed `it87` driver. SiliconWatch reported `fan2_input` and `fan3_input` without scaling or invented CPU/System labels. This was a host-specific compatibility observation, not independent RPM calibration, a verified physical fan mapping, or a guarantee of future kernel/reboot compatibility.

No module-loading, forced device-ID or firmware changes are performed by SiliconWatch. Report missing or questionable readings through [GitHub Issues](https://github.com/lgraak/Siliconwatch/issues), omitting private machine identifiers and raw captures unless you have reviewed what they reveal.

## Native package candidates

Native `.deb`, `.rpm` and `.pkg.tar.zst` candidates have been prepared with Rust 1.85 in Debian 12, Fedora 44 and Arch build environments. Installation, fixture/API checks and removal were exercised on Debian 12, Ubuntu 24.04, Fedora 44 and the recorded Arch rolling image. The Debian GUI requires glibc 2.35; Fedora/Arch builds require 2.43. Linux Mint, CachyOS and EndeavourOS package installation are not established by the base-distribution tests.

See the [installation guide](packaging/linux/INSTALL.md) and [package preparation report](docs/milestones/010-multidistro-package-preparation.md) for exact versions, dependencies, hashes and desktop qualification limits. Installation does not start a daemon, enable a service or configure a driver. The packages remain unsigned candidates until separately approved release publication; no portable AppImage is claimed.

## Known limitations

- Alpha software for **read-only monitoring**: no hardware controls, protective shutdown, alarms, calibration or guaranteed sensor coverage.
- Statistics are session-only; stale/historical readings never become current merely because a response arrived recently. Physical measurement age and hardware health are not certified.
- Production launch/tray checks passed on the current KDE Wayland and XWayland host. The fresh instrumented Wayland full-workflow and lifecycle probes timed out waiting for initial snapshots; their cause remains unresolved. XWayland full workflow, profile restarts and lifecycle checks passed. These results do not establish other distributions' actual desktop sessions. A separate Xorg session, GNOME, mixed-monitor behavior, physical tray interaction and complete human keyboard/screen-reader qualification remain open.
- Numerical CPU/RSS/latency targets have not been fully qualified. There is no universal Linux binary-portability claim or publicly released AppImage.

The [Prototype 010 implementation report](docs/milestones/010-native-gui.md) records executed checks and limitations. The [approved proposal](docs/milestones/010-native-gui-proposal.md) remains a historical planning record; later implementation approvals govern the working candidate.

## Future direction

Graphs and persistent history, configurable dashboards, calculated sensors, broader hardware providers, portable diagnostic reports, remote monitoring, alarms and protective actions remain future work. Planned capabilities are described in [design decisions](docs/DESIGN_DECISIONS.md); they are not current Alpha 1 features. No subsequent prototype or deployment is implied by this documentation update.

## Help shape SiliconWatch

Interested in Rust, Linux hardware interfaces, native desktop applications, or testing across different systems? [Open an issue](https://github.com/lgraak/Siliconwatch/issues) to share a use case, discuss hardware support, report a bug, or propose an area you'd like to work on.

Contributions follow the [DCO signoff policy in Decision 41](docs/DESIGN_DECISIONS.md#41-contributor-provenance--accepted). Contributors retain their copyright; no CLA or copyright assignment is required.

## Acknowledgments

SiliconWatch is inspired in part by [HWiNFO64](https://www.hwinfo.com/), developed by **Martin Malík of REALiX**.

HWiNFO64 has set an exceptionally high standard for comprehensive hardware monitoring, sensor accuracy, and system diagnostics. Its depth of information and attention to detail have been a significant influence on the vision for SiliconWatch.

We sincerely appreciate the years of work Martin has invested in HWiNFO and the value his software brings to the hardware enthusiast and engineering communities.

SiliconWatch is an independently developed, open-source project intended to bring a similarly comprehensive monitoring experience to Linux.

**SiliconWatch is not affiliated with, endorsed by, or derived from HWiNFO64 or REALiX.**

## License

SiliconWatch is licensed under the [GNU General Public License v3.0](https://github.com/lgraak/Siliconwatch/blob/main/LICENSE).
