#!/usr/bin/env bash
# Entry point is run-native.sh. Root operations occur only in its disposable container.
set -euo pipefail
[[ -d /release-tools && -d /inputs && $(id -u) == 0 ]] || exit 2
mode=$1 distro=$2 version=$3 commit=$4
prefix="siliconwatch-$version-linux"
case "$distro" in
    debian|ubuntu) format=debian; artifact="$prefix-amd64.deb" ;;
    fedora) format=fedora; artifact="$prefix-x86_64.rpm" ;;
    arch) format=arch; artifact="$prefix-x86_64.pkg.tar.zst" ;;
    *) exit 2 ;;
esac
export DEBIAN_FRONTEND=noninteractive
install_tools() {
    case "$distro" in
        debian|ubuntu) apt-get install -y --no-install-recommends "$@" ;;
        fedora) dnf -y --setopt=tsflags= install "$@" ;;
        arch) pacman -S --needed --noconfirm "$@" ;;
    esac
}
case "$distro" in
    debian|ubuntu)
        apt-get update
        # Slim images filter documentation. Include the application documentation in the test root.
        printf 'path-include=/usr/share/doc/siliconwatch\npath-include=/usr/share/doc/siliconwatch/*\n' > /etc/dpkg/dpkg.cfg.d/zz-siliconwatch-test
        install_tools python3 ca-certificates passwd util-linux
        ;;
    fedora) install_tools python3 shadow-utils util-linux-core util-linux ;;
    arch)
        # Bind the package repositories as well as the OCI base to the qualified rolling snapshot.
        # shellcheck disable=SC2016 # pacman expands the repository and architecture variables.
        echo 'Server = https://archive.archlinux.org/repos/2026/11/18/$repo/os/$arch' > /etc/pacman.d/mirrorlist
        if [[ $mode == verify ]]; then
            # Arch's slim image excludes documentation. Retain its exclusions outside the tested package.
            install -d -m 0755 /usr/share/doc
            printf '\n[options]\nNoExtract = !usr/share/doc/siliconwatch/ !usr/share/doc/siliconwatch/*\n' >> /etc/pacman.conf
        fi
        pacman -Syyuu --noconfirm
        install_tools python shadow util-linux
        ;;
esac
useradd --create-home --shell /bin/bash swbuild
as_user() { runuser -u swbuild -- "$@"; }
install -d -o swbuild -g "$(id -gn swbuild)" /work
cd /work

if [[ $mode == build ]]; then
    case "$distro" in
        debian) install_tools build-essential pkg-config cmake libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcb1-dev libxcursor-dev libxi-dev libegl1-mesa-dev libgl1-mesa-dev fonts-dejavu-core xz-utils ;;
        fedora) install_tools gcc gcc-c++ make pkgconf-pkg-config cmake fontconfig-devel libxkbcommon-devel libxkbcommon-x11-devel wayland-devel libX11-devel libxcb-devel libXcursor-devel libXi-devel mesa-libEGL-devel mesa-libGL-devel dejavu-sans-fonts rpm-build desktop-file-utils tar gzip ;;
        arch) install_tools base-devel cmake fontconfig libxkbcommon libxkbcommon-x11 wayland libx11 libxcb libxcursor libxi libglvnd ttf-dejavu desktop-file-utils ;;
    esac
    source_name="siliconwatch-$version-source"
    python3 /release-tools/verify-payload.py source "$version" "$commit" /inputs
    as_user tar --no-same-owner -xzf - < "/inputs/$source_name.tar.gz"
    cd "/work/$source_name"
    [[ $(< SOURCE-REVISION) == "$commit" ]]
    lock_digest=$(python3 -c 'import json; print(json.load(open("/inputs/SOURCE-PROVENANCE.json"))["cargo_lock_sha256"])')
    printf '%s  Cargo.lock\n' "$lock_digest" | sha256sum -c -
    export PATH="/opt/rust/bin:$PATH" CARGO_BUILD_JOBS=4 RUSTFLAGS='-C target-cpu=x86-64' CARGO_TARGET_DIR=/work/target
    rustc -Vv
    as_user cargo build --workspace --locked --offline --release
    test "$(/work/target/release/siliconwatch --version)" = "siliconwatch $version"
    python3 /release-tools/stage_payload.py --source "$PWD" --binary-dir /work/target/release --destination /work/payload --notices /inputs/licenses --provenance /inputs/SOURCE-PROVENANCE.json > "/out/payload-$format.json"
    case "$distro" in
        debian)
            python3 /release-tools/build-deb.py /work/payload "/out/$artifact" --maintainer 'lgraak <aeons80@gmail.com>' --version "${version/-/\~}-1"
            ;;
        fedora)
            base=${version%%-*}
            release=1
            [[ $version == "$base" ]] || release="0.1.${version#*-}"
            install -d -o swbuild -g "$(id -gn swbuild)" /work/rpmbuild/SOURCES
            tar -C /work/payload -czf /work/rpmbuild/SOURCES/siliconwatch-fedora44-payload.tar.gz usr
            chown swbuild /work/rpmbuild
            as_user rpmbuild -bb --define '_topdir /work/rpmbuild' --define "sw_version $base" --define "sw_release $release" /release-tools/fedora/siliconwatch.spec
            packages=(/work/rpmbuild/RPMS/x86_64/siliconwatch-*.rpm)
            [[ ${#packages[@]} == 1 ]]
            cp "${packages[0]}" "/out/$artifact"
            ;;
        arch)
            install -d -o swbuild -g "$(id -gn swbuild)" /work/arch
            cp /release-tools/arch/PKGBUILD /work/arch/
            cd /work/arch
            payload_name="siliconwatch-$version-arch-payload.tar"
            tar -C /work -cf "$payload_name" payload
            export SW_ARCH_VERSION="${version/-/}" SW_ARCH_PAYLOAD="$payload_name"
            SW_ARCH_SHA256=$(sha256sum "$payload_name" | cut -d ' ' -f 1)
            export SW_ARCH_SHA256
            as_user makepkg --printsrcinfo > /work/candidate.SRCINFO
            cat /work/candidate.SRCINFO
            as_user makepkg --cleanbuild --noconfirm
            packages=(siliconwatch-*-x86_64.pkg.tar.zst)
            [[ ${#packages[@]} == 1 ]]
            cp "${packages[0]}" "/out/$artifact"
            ;;
    esac
    sha256sum "/out/$artifact"
    exit 0
fi

[[ $mode == verify ]]
# Use the same private copy for every transaction even if the host replaces its input.
[[ -f /inputs/$artifact && ! -L /inputs/$artifact ]]
install -d -m 0755 /qualified
package="/qualified/$artifact"
install -m 0444 "/inputs/$artifact" "$package"
test -s "$package"
package_hash=$(sha256sum "$package" | cut -d ' ' -f 1)
case "$distro" in
    debian|ubuntu)
        test "$(dpkg-deb -f "$package" Package)" = siliconwatch
        test "$(dpkg-deb -f "$package" Architecture)" = amd64
        native_version="${version/-/\~}-1"
        test "$(dpkg-deb -f "$package" Version)" = "$native_version"
        dpkg --compare-versions '0.1.0~alpha.1-1' lt "$native_version"
        [[ $version != *-* ]] || dpkg --compare-versions "$native_version" lt "${version%%-*}-1"
        control=$(dpkg-deb --ctrl-tarfile "$package" | tar -tf -)
        if grep -Ev '^(\./|\./(md5sums|control))$' <<<"$control"; then exit 1; fi
        dpkg-deb --info "$package"
        install_pkg() { install_tools "$1"; }
        remove_pkg() { apt-get remove -y siliconwatch; }
        owner_files() { dpkg-query -L siliconwatch; }
        ;;
    fedora)
        base=${version%%-*}
        release=1
        [[ $version == "$base" ]] || release="0.1.${version#*-}"
        native_version="$base-$release.fc44"
        test "$(rpm -qp --qf '%{NAME} %{ARCH} %{VERSION}-%{RELEASE}' "$package")" = "siliconwatch x86_64 $native_version"
        rpm_older() { [[ $(rpm --eval "%{lua: print(rpm.ver('$1') < rpm.ver('$2'))}") == true ]]; }
        rpm_older '0.1.0-0.1.alpha.1.fc44' "$native_version"
        [[ $version != *-* ]] || rpm_older "$native_version" "$base-1.fc44"
        test -z "$(rpm -qp --scripts --triggers "$package")"
        rpm -qpR "$package"
        install_pkg() { install_tools "$1"; }
        remove_pkg() { dnf -y remove siliconwatch; }
        owner_files() { rpm -ql siliconwatch; }
        ;;
    arch)
        native_version="${version/-/}-1"
        test "$(pacman -Qp "$package")" = "siliconwatch $native_version"
        [[ $(vercmp '0.1.0alpha.1-1' "$native_version") -lt 0 ]]
        [[ $version != *-* || $(vercmp "$native_version" "${version%%-*}-1") -lt 0 ]]
        bsdtar -xOf "$package" .PKGINFO | grep -Fx 'arch = x86_64' >/dev/null
        contents=$(bsdtar -tf "$package")
        if grep -Fx '.INSTALL' <<<"$contents"; then exit 1; fi
        pacman -Qip "$package"
        install_pkg() { pacman -U --noconfirm "$1"; }
        remove_pkg() { pacman -R --noconfirm siliconwatch; }
        owner_files() { pacman -Qlq siliconwatch; }
        ;;
esac
# Install from a fresh runtime before adding graphical test tools, then inspect the actual installed payload.
install_pkg "$package"
owner_files > /work/owned-files
python3 /release-tools/verify-payload.py installed "$version" "$commit" "/inputs/payload-$format.json" /work/owned-files
for exe in /usr/bin/siliconwatch /usr/bin/siliconwatch-gui; do
    libs=$(ldd "$exe")
    echo "$libs"
    if grep -F 'not found' <<<"$libs"; then exit 1; fi
done
test "$(as_user siliconwatch --version)" = "siliconwatch $version"
as_user siliconwatch-gui --help
case "$distro" in
    debian|ubuntu) install_tools desktop-file-utils xvfb x11-utils libgl1-mesa-dri dbus-daemon binutils curl ;;
    fedora) install_tools desktop-file-utils xorg-x11-server-Xvfb /usr/bin/xwininfo mesa-dri-drivers dbus-daemon binutils curl-minimal ;;
    arch) install_tools desktop-file-utils xorg-server-xvfb xorg-xwininfo mesa dbus binutils curl ;;
esac
desktop-file-validate /usr/share/applications/siliconwatch.desktop
for exe in /usr/bin/siliconwatch /usr/bin/siliconwatch-gui; do
    header=$(readelf -d "$exe")
    if grep -E '\((RPATH|RUNPATH)\)' <<<"$header"; then exit 1; fi
done
as_user dbus-run-session -- python3 /release-tools/smoke-installed.py
sentinel=/home/swbuild/.config/siliconwatch/config.toml
as_user mkdir -p "$(dirname "$sentinel")"
# shellcheck disable=SC2016 # The ordinary-user shell receives the sentinel path as its first argument.
as_user sh -c 'umask 077; echo "schema_version = 1" > "$1"' sh "$sentinel"
before=$(sha256sum "$sentinel")
remove_pkg
python3 /release-tools/verify-payload.py removed "$version" "$commit" "/inputs/payload-$format.json"
# Exercise a real upgrade from the first public native release, pinned to the reviewed immutable bytes.
case "$format" in
    debian) previous_hash=89874755a3e5ed9b87c4d8c1fb65de90a97988af52a63516ddaf9a90082cc6a8 ;;
    fedora) previous_hash=cd0c35c0f6a9763bdd9aa0a843d7b12d5276b0521c980a219011b95b4537e767 ;;
    arch) previous_hash=8b4b606834079946aaaffe3a4ffc12067fa7448c40b26d2855fd2e8593f0c2b4 ;;
esac
previous=${artifact/"$version"/0.1.0-alpha.1}
curl --fail --silent --show-error --location --proto '=https' --max-time 180 --retry 2 -o "/work/$previous" "https://github.com/lgraak/Siliconwatch/releases/download/v0.1.0-alpha.1/$previous"
printf '%s  %s\n' "$previous_hash" "/work/$previous" | sha256sum -c -
install_pkg "/work/$previous"
test "$(as_user siliconwatch --version)" = 'siliconwatch 0.1.0-alpha.1'
install_pkg "$package"
test "$(as_user siliconwatch --version)" = "siliconwatch $version"
owner_files > /work/owned-files
python3 /release-tools/verify-payload.py installed "$version" "$commit" "/inputs/payload-$format.json" /work/owned-files
test "$(sha256sum "$sentinel")" = "$before"
test "$(stat -c '%a %U' "$sentinel")" = '600 swbuild'
remove_pkg
python3 /release-tools/verify-payload.py removed "$version" "$commit" "/inputs/payload-$format.json"
test "$(sha256sum "$sentinel")" = "$before"
test "$(stat -c '%a %U' "$sentinel")" = '600 swbuild'
printf '%s  %s\n' "$package_hash" "$package" | sha256sum -c -
# The version, commit and runtime are validated by run-native.sh; no receipt precedes the final checks.
printf '{"schema":"siliconwatch.install.v1","result":"passed","distro":"%s","tag":"v%s","source_commit":"%s","package":"%s","sha256":"%s"}\n' \
    "$distro" "$version" "$commit" "$artifact" "$package_hash" > "/receipts/install-$distro.json"
echo "Verified clean install, runtime, Alpha 1 upgrade and removal on $distro"
