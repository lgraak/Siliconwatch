# Native Linux package recipes — 0.1.0-alpha.1

These recipes assemble local review candidates. They do not authorize a release,
signing, uploading, services or Prototype 011. Prototype 010 acceptance remains
Pending. The preparation report under `docs/milestones/` records exact final
artifacts, tested environments, failures and remaining qualification limits.

## Build inputs and isolation

Build the unchanged workspace with official Rust 1.85.0, Slint 1.13.1 and the
existing `vendor/fontdue-0.9.5` patch in a target-distribution environment.
Do not reuse the old CachyOS user-test binaries as portable package inputs.
Keep source read-only and build outputs, caches, test fixtures and package
artifacts outside every repository worktree. The reviewed preparation used
digest-pinned official Debian 12, Fedora 44 and Arch images with rootless user,
PID, mount, IPC and UTS isolation. Runtime installation roots were separate from
build roots. No host hwmon tree, desktop configuration or service manager was
mounted into those containers. Native host GUI checks used owned fixtures and
private settings separately.

Inside the build environment, with the Rust 1.85 toolchain first on PATH:

```sh
export CARGO_TARGET_DIR=/work/target
export CARGO_BUILD_JOBS=4
export RUSTFLAGS='-C target-cpu=x86-64'
cargo build --release --workspace --locked --offline
```

The offline Cargo registry cache must already contain the locked dependencies;
copy only registry data, not credential or user configuration files. Do not
update the lockfile to resolve a missing cache entry. The exact build sources
are recorded in the artifact's `SOURCE-PROVENANCE.json`. Its base commit alone
does not contain the uncommitted GUI. This is provenance, not a claim of
bit-for-bit reproducibility or published corresponding-source availability.

Build tools include a C/C++ compiler/linker, pkg-config, fontconfig, XKB, X11,
Wayland and EGL development libraries, plus the distribution's native package
tools. Resolve them with the distribution package manager and preserve its
normal repository signature checks. Exact image digests and package inventories
accompany this candidate. Runtime dependencies are distinct from build/test
tools. ELF DT_NEEDED alone is insufficient: Winit/FemtoVG also dynamically load
windowing, keyboard, font and OpenGL libraries.

## Common payload

`stage_payload.py` accepts explicit inputs and copies only the two production
executables, approved icon, desktop file, user/API documentation, source
provenance and reviewed third-party notices. It never scans a whole worktree.
The CLI already implements the foreground daemon; no duplicate daemon binary
is added. The GUI embeds its Slint UI/icon resources. The original PNG is also
installed under `/usr/share/pixmaps` for desktop icon lookup.

```sh
python3 packaging/linux/stage_payload.py \
  --source /source --binary-dir /work/target/release \
  --notices /inputs/licenses --destination /work/payload \
  --provenance /inputs/SOURCE-PROVENANCE.json > /work/payload-manifest.json
```

Supply the reviewed notices collection and exact source manifest from the
external preparation evidence; these are explicit inputs, not silently
downloaded files. The 383-package Cargo inventory includes build/procedural
macro dependencies conservatively and is not a linker-level SBOM. Review the
recorded `simd_helpers` notice gap and matching corresponding-source requirement
before any separately authorized public distribution.

## Native formats

- **Debian 12:** `python3 packaging/linux/build-deb.py PAYLOAD OUTPUT --maintainer 'Name <email>'` runs
  `dpkg-deb --root-owner-group`. Use the verified project contributor identity.
  Internal version is `0.1.0~alpha.1-1`, architecture `amd64`; the measured GUI
  floor is glibc 2.35. The exact runtime dependency list is in the builder.
- **Fedora 44:** put the staged `usr/` tree into
  `siliconwatch-fedora44-payload.tar.gz` under an external rpmbuild `SOURCES/`,
  then use `fedora/siliconwatch.spec` with `rpmbuild -bb`. The internal version is
  `0.1.0-0.1.alpha.1.fc44`. Keep RPM's generated ELF dependencies and explicit
  dlopen requirements. Default DNF weak-dependency handling installs the
  recommended DejaVu font; installations disabling weak dependencies must
  already supply suitable fonts. The native GUI requires glibc 2.43. The spec
  uses a generic Build Host, preserves verified binary bytes, and adds no
  scriptlets, service or build-id links.
- **Arch:** create `siliconwatch-0.1.0-alpha.1-arch-payload.tar` containing
  `payload/usr/`, verify/update its SHA-256 deliberately, and run the included
  `arch/PKGBUILD` with `makepkg` as a non-root build user. Internal version is
  `0.1.0alpha.1-1`; the GUI floor is glibc 2.43. The checked-in checksum matches
  this candidate's prepared payload and is not a remote source-code checksum.
  `.SRCINFO` is generated from that recipe. Do not use an underscore prerelease
  version, which sorts incorrectly relative to the stable version.

Preserve native metadata while copying final files to the requested delivery
names. The packages are unsigned review artifacts. Normal Fedora 44 local
installation was tested; Fedora 45's stricter signature policy is not qualified.
No trust keys or global signature overrides were introduced.

## Verification and release boundary

Use clean target roots for native package installation and dependency
resolution, then record installed runtime libraries before adding test tools.
Exercise the installed CLI/JSON, foreground daemon and read-only API against
synthetic hwmon files as a non-root user. Check socket modes, shutdown cleanup,
desktop metadata/icon and absence of application services. Remove using the
native manager and verify application files disappear while user preference
bytes and modes remain intact. Official slim images can filter documentation;
the test-only policy exception must be recorded separately from package behavior.

Virtual X11/Wayland tests establish behavior under their recorded software
graphics stacks; they do not establish an actual Fedora, Debian, Ubuntu or Arch
desktop session. Native host testing has its own evidence and limitations.
Human menu, tray, keyboard and accessibility checks remain separate. Do not
replace an unexecuted derivative test with an assumed compatibility claim.

See `INSTALL.md` for user instructions. No portable AppImage is included in this
preparation: the existing CachyOS-specific user-test AppImage is preserved,
and a new final portable AppImage was not qualified. Public release requires
Chris's separate acceptance/publication authorization and the unresolved source,
license and desktop qualification review.
