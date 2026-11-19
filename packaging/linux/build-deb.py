#!/usr/bin/env python3
"""Build the Debian review candidate from the previously verified payload."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

p=argparse.ArgumentParser()
p.add_argument('payload',type=Path)
p.add_argument('output',type=Path)
p.add_argument('--maintainer',required=True)
p.add_argument('--version', help='native version (defaults to the staged source provenance version)')
a=p.parse_args()
assert a.payload.is_dir() and not a.output.exists()
version = a.version
if version is None:
    record = json.loads((a.payload / 'usr/share/doc/siliconwatch/SOURCE-PROVENANCE.json').read_text())
    version = record['version'].replace('-', '~', 1) + '-1'
subprocess.run(['dpkg', '--validate-version', version], check=True)
control=a.payload/'DEBIAN';control.mkdir()
size=sum(x.stat().st_size for x in a.payload.rglob('*') if x.is_file())
dependencies='libc6 (>= 2.35), libgcc-s1, libfontconfig1, fonts-dejavu-core, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libwayland-cursor0, libwayland-egl1, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libegl1, libgl1'
(control/'control').write_text(f'''Package: siliconwatch
Version: {version}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: {a.maintainer}
Installed-Size: {(size+1023)//1024}
Depends: {dependencies}
Homepage: https://github.com/lgraak/Siliconwatch
Description: Read-only local hardware sensor monitor with native GUI
 Provides a sensor CLI, manually started foreground daemon, and current-user
 Slint GUI using a local read-only Unix-socket API. No hardware controls,
 service enablement, driver setup, remote access or automatic daemon startup.
''')
files=[f for f in sorted(a.payload.rglob('*')) if f.is_file() and control not in f.parents]
(control/'md5sums').write_text(''.join(hashlib.md5(f.read_bytes()).hexdigest()+'  '+str(f.relative_to(a.payload))+'\n' for f in files))
for f in control.iterdir():f.chmod(0o644)
control.chmod(0o755)
subprocess.run(['dpkg-deb','--root-owner-group','-Zxz','--build',str(a.payload),str(a.output)],check=True)
subprocess.run(['dpkg-deb','--info',str(a.output)],check=True)
