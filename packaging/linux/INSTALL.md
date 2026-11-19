# SiliconWatch native Linux packages

The examples use the proposed `v0.1.0-alpha.2` package names. Reviewers should use the packages and `SHA256SUMS` from the same candidate. Once a release is approved and published, obtain both from that version's GitHub release.

## Install

Run the package manager from the directory containing the downloaded candidate.
The Debian package is built on Debian 12. Ubuntu 24.04 is a separate installation
target. Test the installed distribution and version before relying on compatibility with Linux Mint, CachyOS or EndeavourOS.

```sh
# Debian 12 or Ubuntu 24.04
sudo apt install ./siliconwatch-0.1.0-alpha.2-linux-amd64.deb

# Fedora 44 candidate; see the report before trying other Fedora versions
sudo dnf install ./siliconwatch-0.1.0-alpha.2-linux-x86_64.rpm

# Arch Linux at a compatible rolling snapshot
sudo pacman -U ./siliconwatch-0.1.0-alpha.2-linux-x86_64.pkg.tar.zst
```

The candidates are unsigned. Package-manager trust policy may reject them;
do not disable global signature verification to make installation succeed.
In particular, this build does not establish Fedora 45 installation support.
Derivative compatibility is not implied by a successful base-distribution test.
Verify the package SHA-256 against the associated checksum manifest before installation. For the Debian package, run the following check.

```sh
file=siliconwatch-0.1.0-alpha.2-linux-amd64.deb
awk -v name="$file" '$2 == name' SHA256SUMS | sha256sum --strict --check -
```

Before upgrading, close the GUI, back up your configuration, and stop the foreground daemon. The updated GUI reads older profiles with defaults. Alpha 1 rejects profiles saved by the new version with a `[monitoring]` table. Restore your pre-upgrade profile before launching Alpha 1 after a downgrade. See the installed `GUI.md` for backup and import instructions.

Installation provides `/usr/bin/siliconwatch`, `/usr/bin/siliconwatch-gui`, a
desktop entry and the approved project icon. The first executable implements
both the CLI and foreground daemon. No service, autostart entry, driver setup,
socket permission changes or privileged background process is installed.

## Start and connect

Use your ordinary logged-in desktop account, without sudo. In a terminal:

```sh
siliconwatch sensors
siliconwatch sensors --json
siliconwatch daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

Leave that terminal open. In a second terminal, or from the application menu:

```sh
siliconwatch-gui
```

The daemon uses the login session's current-user, mode-0700 `XDG_RUNTIME_DIR`.
Its endpoint is `$XDG_RUNTIME_DIR/siliconwatch/api.sock`. The GUI checks the
endpoint and peer identity. Do not run either application as root, loosen the
runtime directory permissions or change socket ownership to fix a connection.
If the daemon is absent, the GUI shows a disconnected state. Start the daemon
manually, then choose **Reconnect**. Reconnect reads the daemon cache; it does
not trigger a new hardware acquisition. The connection/observation status and
Sensors table establish successful communication; stale or unavailable readings
remain visibly distinct from current observations. The local API contract is
installed beside this guide under `api/`.

Close the GUI with its window close control or **Quit SiliconWatch**. This leaves
the separate daemon running. Stop that foreground daemon with Ctrl-C in its
terminal; normal shutdown removes its own socket and empty child directory.
A crash may leave a stale endpoint. Inspect ownership and confirm the old
process is gone before handling that specific endpoint; never recursively
delete a runtime tree.

## Sensors and settings

The Sensors table groups observations by source and displays Current, Minimum,
Maximum and Average. These statistics cover the GUI session's observed valid
readings, not durable history. Expand a source and select a sensor for Device
Details. Unsupported values and diagnostics remain visible. Temperatures can
be displayed in Celsius or Fahrenheit; RPM, volts and raw values are preserved.

OLED Black is the default appearance. Settings also offers Dark and Light,
font/row/column controls and window geometry. **Minimize to Tray** defaults off:
closing the last window quits. Enable it only with a working desktop
StatusNotifierItem tray host. With a usable host, closing Sensors hides it and
the tray offers Open Sensors, Settings, Reconnect and Quit SiliconWatch. With no
usable host, the last-window close exits; losing the host while hidden restores
Sensors. Desktop focus and tray presentation remain compositor dependent.
Do not install desktop extensions solely to meet this candidate's expectations.

Under **Settings → Import, Export & Recovery**, enter a complete absolute local
file path, choose **Export Settings…** or **Import Settings…**, review the
confirmation/preview and confirm. Imports replace a complete validated profile.
Leave device-group preferences out of exports unless you intend to include
source identity selectors. Imported group matches require explicit review
against the current inventory. See `GUI.md` for profile/recovery details.

Preferences live at `$XDG_CONFIG_HOME/siliconwatch/config.toml`, or
`~/.config/siliconwatch/config.toml` when `XDG_CONFIG_HOME` is unset. The GUI
creates private preferences and retains a prior valid profile as
`config.toml.bak`. Package installation and removal do not own these files.

**Settings → Monitoring** shows both your requested daemon timings and the current daemon's effective timings. After applying a new request, stop the foreground daemon and restart it with the command displayed in Settings. The GUI’s saved API response timeout applies to its next requests.

## Missing sensors or a failed GUI launch

Available temperatures, voltages and fan RPM depend on Linux hwmon drivers and
what they expose on that machine. The optional NVIDIA NVML provider can supply GPU temperature, utilization, memory, clocks, performance state and supported fan or power readings. A missing motherboard sensor is not evidence
that the GUI needs elevated permissions. Review `siliconwatch sensors --json`
and the GUI diagnostics. Follow the distribution's ordinary supported-driver
documentation; do not force monitoring drivers, bypass ACPI protections, change
PWM controls or load an untrusted module for this test. No hardware controls are
provided by SiliconWatch.

A graphical login session, fonts and a working OpenGL/EGL stack are required.
The package declares Winit/FemtoVG runtime libraries, but a suitable Mesa or
vendor driver remains part of the desktop environment. Wayland or X11 display
access must be available to the current user. Starting the GUI over a headless
shell does not establish GUI compatibility. An absent DBus/tray host does not
authorize background daemon management. Consult the verification report for
tested backends, software rendering and unexecuted desktop/accessibility checks.

## Remove

First quit the GUI and stop any daemon you started manually. Removing files does
not supervise an already-running process.

```sh
sudo apt remove siliconwatch       # Debian family
sudo dnf remove siliconwatch       # Fedora
sudo pacman -R siliconwatch        # Arch family
```

Preferences and exported profiles remain. To remove preferences intentionally,
quit all SiliconWatch GUI instances, locate the exact configuration directory
described above and delete its `config.toml` and `config.toml.bak` files using
your file manager. Preserve exported profiles you still need. Do not remove
unrelated configuration or runtime directories.
