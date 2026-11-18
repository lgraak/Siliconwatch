#!/usr/bin/env python3
"""Stage only distributable runtime files; never scan/copy a whole worktree."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

p = argparse.ArgumentParser()
for name in ['source', 'binary-dir', 'notices', 'destination', 'provenance']:
    p.add_argument('--'+name, type=Path, required=True)
a = p.parse_args()
assert not a.destination.exists(), 'Destination must be new'
src = a.source.resolve()
recipe = Path(__file__).resolve().parent
dst = a.destination

def copy(source, relative, executable=False):
    assert source.is_file() and not source.is_symlink(), source
    target = dst/relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    target.chmod(0o755 if executable else 0o644)

for binary in ['siliconwatch', 'siliconwatch-gui']:
    copy(a.binary_dir/binary, 'usr/bin/'+binary, True)
copy(src/'assets/branding/siliconwatch-icon.png', 'usr/share/pixmaps/siliconwatch.png')
copy(recipe/'siliconwatch.desktop', 'usr/share/applications/siliconwatch.desktop')
copy(recipe/'INSTALL.md', 'usr/share/doc/siliconwatch/INSTALL.md')
copy(src/'docs/GUI.md', 'usr/share/doc/siliconwatch/GUI.md')
for name in ['local-v1.md', 'local-v1.schema.json']:
    copy(src/'docs/api'/name, 'usr/share/doc/siliconwatch/api/'+name)
copy(src/'LICENSE', 'usr/share/doc/siliconwatch/copyright')
copy(a.provenance, 'usr/share/doc/siliconwatch/SOURCE-PROVENANCE.json')
for source in sorted(a.notices.rglob('*')):
    if source.is_file():
        copy(source, Path('usr/share/licenses/siliconwatch')/source.relative_to(a.notices))
for directory in dst.rglob('*'):
    if directory.is_dir():
        directory.chmod(0o755)
dst.chmod(0o755)
manifest = [{'path':'/'+str(f.relative_to(dst)), 'size':f.stat().st_size,
             'mode':oct(f.stat().st_mode & 0o777),
             'sha256':hashlib.file_digest(f.open('rb'),'sha256').hexdigest()}
            for f in sorted(dst.rglob('*')) if f.is_file()]
print(json.dumps(manifest, indent=2))
