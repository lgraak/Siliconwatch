# Native Fedora 44 candidate; common payload and fresh Rust build audited separately.
# Source0 must contain usr/ with only the reviewed application payload.
# Binaries must be the fresh, hash-verified Fedora 44 Rust 1.85 build.
# Default inputs preserve the original standalone Alpha 1 recipe.
%{!?sw_version:%global sw_version 0.1.0}
%{!?sw_release:%global sw_release 0.1.alpha.1}
%global debug_package %{nil}
# Keep the user's workstation identity out of distributable RPM metadata.
%global _buildhost siliconwatch-fedora44
# Preserve the exact audited native binaries, including embedded notes/symbols.
# Leave RPATH, shebang, archive, reproducibility and duplicate checks enabled.
%global __brp_strip %{nil}
%global __brp_strip_comment_note %{nil}
%global __brp_strip_lto %{nil}
# No separate debuginfo payload or automatically added build-id symlinks.
%global _build_id_links none

Name:           siliconwatch
Version:        %{sw_version}
Release:        %{sw_release}%{?dist}
Summary:        Read-only Linux sensor monitoring with a native GUI and CLI
License:        GPL-3.0-only
URL:            https://github.com/lgraak/Siliconwatch
Source0:        siliconwatch-fedora44-payload.tar.gz
BuildArch:      x86_64
BuildRequires:  desktop-file-utils
# ELF-generated dependencies remain enabled. These explicit requirements cover
# dynamically loaded windowing, keyboard, font and OpenGL interfaces.
Requires:       fontconfig
Requires:       libxkbcommon
Requires:       libxkbcommon-x11
Requires:       libwayland-client
Requires:       libwayland-cursor
Requires:       libwayland-egl
Requires:       libX11
Requires:       libX11-xcb
Requires:       libxcb
Requires:       libXcursor
Requires:       libXi
Requires:       libglvnd-egl
Requires:       libglvnd-glx
Recommends:     dejavu-sans-fonts

%description
SiliconWatch is an experimental Rust-based Linux hardware-monitoring
application with a native Slint GUI, CLI, foreground daemon and an opt-in
current-user read-only local API. Available readings depend on Linux drivers.
Installation does not start a daemon, install a service or change hardware.

%prep
%setup -q -c -T
%{__tar} -xf %{SOURCE0}

%build
# The separate native Fedora 44 build is recorded with the artifact provenance.
# This packaging step deliberately preserves those reviewed binary bytes.

%install
mkdir -p %{buildroot}
cp -a usr %{buildroot}/

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/siliconwatch.desktop

%files
%attr(0755,root,root) %{_bindir}/siliconwatch
%attr(0755,root,root) %{_bindir}/siliconwatch-gui
%attr(0644,root,root) %{_datadir}/applications/siliconwatch.desktop
%attr(0644,root,root) %{_datadir}/pixmaps/siliconwatch.png
%doc %{_datadir}/doc/siliconwatch/
%license %{_datadir}/licenses/siliconwatch/

# No %pre, %post, service, user-creation, module or hardware-control scriptlets.
