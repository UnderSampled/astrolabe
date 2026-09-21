#!/usr/bin/env python3
"""Install checksum-pinned test tools locally; no system/user Cargo changes."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import platform
import tarfile
import tempfile
import urllib.request

root = Path(__file__).resolve().parent.parent
pins = json.loads(Path(__file__).with_name('test-tools.json').read_text())
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('tool', choices=sorted(pins), nargs='+')
args = p.parse_args()
if platform.system() != 'Linux' or platform.machine() != 'x86_64':
    p.error('this prebuilt pin is Linux x86_64; install the exact pinned versions for your platform')
base = root/'output/test-tools'
base.mkdir(parents=True, exist_ok=True)
for name in args.tool:
    pin = pins[name]
    target = base/(name+'-'+pin['version'])
    if target.exists():
        p.error(f'{target} exists; preserve it or choose a deliberate replacement')
    data = urllib.request.urlopen(pin['url'], timeout=30).read(100*1024*1024+1)
    if len(data) > 100*1024*1024 or hashlib.sha256(data).hexdigest() != pin['sha256']:
        raise SystemExit('download is oversized or checksum does not match reviewed release')
    with tempfile.TemporaryDirectory(dir=base, prefix='tool-install-') as directory:
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:
            archive.extractall(directory, filter='data')
        if not (Path(directory)/pin['executable']).is_file():
            raise SystemExit('expected executable missing from archive')
        Path(directory).rename(target)
    print(target/pin['executable'])
