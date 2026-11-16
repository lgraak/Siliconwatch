<p align="center">
  <img src="assets/branding/siliconwatch-icon.png" alt="SiliconWatch — a chip and heartbeat inside a shield" width="200">
</p>

# SiliconWatch

**Hardware intelligence and protection for Linux.**

SiliconWatch is an open-source project for native Linux hardware monitoring and protection, built around Rust. Its goal is to bring hardware inventory, live sensors, history, and configurable protective actions into one clear view of your machine.

Understand what your hardware is doing, spot problems early, and keep a record of what happened when a protection rule was triggered.

**Status: Alpha 1, Prototype 003.** A read-only Rust CLI discovers Linux `hwmon` sensors, reports best-effort source identities, and supports bounded foreground sampling with explicit timing and recoverable errors. This is experimental monitoring; the broader capabilities below remain planned.

## Try the prototype

On Linux with Rust 1.85 or newer:

```bash
cargo run -- sensors
cargo run -- sensors --json
cargo run -- sample --count 5 --interval-ms 1000
cargo run -- sample --count 5 --interval-ms 1000 --json-lines
```

Run as your normal user. See [Prototype 003 sampling, output, and limitations](docs/milestones/003-bounded-foreground-sampling.md) and [Prototype 002 identity rules](docs/milestones/002-stable-sensor-identity.md). Sampling exit 0 means all attempts completed, even if individual attempts report errors; inspect each output record. The prototype does not control hardware or perform protective actions.

## What we're building

- **A comprehensive hardware view:** inventory and telemetry for CPUs, GPUs, motherboards, memory, storage, fans, and supported cooling hardware, with NVIDIA, AMD, and Intel support planned.
- **Sensors that work together:** calculated sensors with arithmetic, functions, and sensor chaining, alongside live graphs and history.
- **Configurable protection:** alarms and protective shutdowns, with persistent event logs explaining the readings and rules behind each action.
- **Useful diagnostics:** portable reports and CSV/JSON exports for troubleshooting and analysis.
- **A native Linux experience:** a graphical interface and a separate monitoring daemon, so monitoring can continue when the window is closed.

The local API is foundational to the initial headless daemon and CLI. Authenticated remote access will come later. Available measurements will depend on the hardware, drivers, and permissions exposed by Linux.

## Built for Linux

Rust is the implementation language. Slint is the preferred GUI framework, pending compatibility testing. SiliconWatch draws inspiration in part from HWiNFO64 and is independently developed for Linux.

The [design decisions](docs/DESIGN_DECISIONS.md) capture accepted project direction and pending questions. Detailed architecture, roadmap, and security guidance will follow under `docs/`, with contributor guidance in `CONTRIBUTING.md` as development takes shape.

## Help shape SiliconWatch

Interested in Rust, Linux hardware interfaces, native desktop applications, or testing across different systems? [Open an issue](https://github.com/lgraak/Siliconwatch/issues) to share a use case, discuss hardware support, or propose an area you'd like to work on.

## Acknowledgments

SiliconWatch is inspired in part by [HWiNFO64](https://www.hwinfo.com/), developed by **Martin Malík of REALiX**.

HWiNFO64 has set an exceptionally high standard for comprehensive hardware monitoring, sensor accuracy, and system diagnostics. Its depth of information and attention to detail have been a significant influence on the vision for SiliconWatch.

We sincerely appreciate the years of work Martin has invested in HWiNFO and the value his software brings to the hardware enthusiast and engineering communities.

SiliconWatch is an independently developed, open-source project intended to bring a similarly comprehensive monitoring experience to Linux.

**SiliconWatch is not affiliated with, endorsed by, or derived from HWiNFO64 or REALiX.**

## License

SiliconWatch is licensed under the [GNU General Public License v3.0](https://github.com/lgraak/Siliconwatch/blob/main/LICENSE).
