# Native Linux packages

The candidate workflow builds both production executables on x86-64 Debian 12, Fedora 44 and Arch. A fourth installation test exercises the Debian package on Ubuntu 24.04. Package builds and installation tests run inside disposable containers.

## Run the candidate checks locally

Use Docker, Git, Python 3.11 or newer, GNU tar, zstd and Rust 1.85.0 through rustup on a Linux x86-64 host. Run from the repository root at the candidate commit. The helper extracts the committed source from Git. It builds a Cargo local registry for the supported Linux target. Every distributed crate has a locked checksum and an entry in SOURCE-NOTICES.json. The remaining lockfile entries supply resolver metadata only. The installed binary notices have their own Linux dependency inventory. Choose a temporary filesystem with space for unpacked sources, container images and builds.

```sh
tag=v0.1.0-alpha.2
sha=$(git rev-parse HEAD)
work=$(mktemp -d)
cargo +1.85.0 fetch --locked
python3 -B scripts/source_release/prepare.py --source . --commit "$sha" \
    --version "${tag#v}" --output "$work/source-inputs"
for distro in debian fedora arch; do
    bash packaging/linux/run-native.sh build "$distro" "$tag" "$sha" "$work/$distro" "$work/source-inputs"
    bash packaging/linux/run-native.sh verify "$distro" "$tag" "$sha" "$work/$distro"
done
bash packaging/linux/run-native.sh verify ubuntu "$tag" "$sha" "$work/debian"
```

The source helper retrieves the reviewed notice inputs from the checksum-pinned public Alpha 1 Arch package. With a populated Cargo cache, pass `--reviewed-package /path/to/alpha1.pkg.tar.zst` to use a previously downloaded copy offline. The generated `SOURCE-PROVENANCE.json` binds the source commit, lockfile, archive and notice hashes. The source archive includes the fontdue patch and a Cargo source configuration for offline builds. Changes to the covered dependencies, roles or fontdue patch require a corresponding update to the notice review.

Each build writes the native package and `payload-DISTRO.json`, which records size, mode and SHA-256 for every payload file. The runner verifies the source and notice manifest against the provenance file and compiles offline as an ordinary user. The container mounts the compiler sysroot and release inputs read-only. Its distribution package manager uses its usual repository signature verification. The base images are pinned by Linux amd64 manifest digest in `run-native.sh`. Distribution repository updates can still change native dependencies.

Native Debian and RPM versions and the Arch package recipe are derived from the candidate version. Arch regenerates the payload checksum and runs `makepkg --printsrcinfo` during each build. The build log records the generated metadata. The checked-in `arch/.SRCINFO` records the historical Alpha 1 payload.

Each verification runs in a freshly created installation root. It checks the exact version, architecture, runtime dependencies, package-owned paths, file modes, SHA-256 values and desktop entry, then exercises an ordinary-user fixture reading, cached API responses, socket permissions, an actual GUI window under virtual X11 and daemon shutdown. A second installation upgrades the digest-pinned public Alpha 1 package to the candidate and checks preservation of user preferences and removal of application files. Each runtime installs the same private copy of the native archive throughout its tests. After all checks succeed, it writes `install-DISTRO.json` with the distro, tag, source commit, package filename and SHA-256. Debian and Ubuntu issue separate receipts for the same Debian package. Sealing and promotion both verify all four receipts against the actual archives. The verification report archive admits only the expected regular files, up to 16 MiB per report, 4 KiB per receipt and 64 MiB for the expanded tar including its headers and padding. Native Arch checks do not qualify derivative distributions. Virtual displays and synthetic telemetry leave physical GPU and desktop qualification to separate tests.

The release workflow and approval controls are documented in the [automation guide](../../docs/releases/automation.md). See the [Alpha 1 preparation report](../../docs/milestones/010-multidistro-package-preparation.md) for historical qualification and the [installation guide](INSTALL.md) for user instructions.
