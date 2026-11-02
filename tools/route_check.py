#!/usr/bin/env python3
"""Run saved route-bot routes (tools/routes/*.json) through the real game, on demand.

Not part of the gate: a route takes minutes. Usage:
  tools/route_check.py HANDOFF_DIR [route ...]
HANDOFF_DIR is a pristine XDG data directory holding the New Game handoff save
(user://manoir-save.cfg). Each run starts from a fresh copy. A route whose
"required" step fails is retried. Each route also needs a pass check: the final STATE
line is printed and any SCRIPT ERROR or ROUTE ABORT fails the run.
"""
import os, shutil, subprocess, sys, tempfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent

def run(handoff: Path, route: Path, attempts: int = 4) -> bool:
    for attempt in range(attempts):
        with tempfile.TemporaryDirectory() as scratch:
            data = Path(scratch) / 'x'
            shutil.copytree(handoff, data)
            env = dict(os.environ, XDG_DATA_HOME=str(data), XDG_CONFIG_HOME=str(data), XDG_CACHE_HOME=str(data))
            command = ['godot', '--headless', '--path', str(root / 'godot'), '--audio-driver', 'Dummy', '--script', 'res://tests/route_bot.gd', '--', str(route), str(Path(scratch) / 'out.json')]
            try:
                log = subprocess.run(command, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=600).stdout
            except subprocess.TimeoutExpired:
                print(f'{route.name}: timed out'); return False
        if 'ROUTE ABORT' in log:
            print(f'{route.name}: attempt {attempt + 1} aborted, retrying'); continue
        lines = [line for line in log.splitlines() if line.startswith(('STATE', 'ROUTE')) and 'FAILED' in line or 'SCRIPT ERROR' in line]
        for line in lines: print(f'  {line[:200]}')
        print(f'{route.name}: {"FAIL" if lines else "ok"}')
        return not lines
    return False

if __name__ == '__main__':
    if len(sys.argv) < 2: sys.exit(__doc__)
    names = sys.argv[2:] or sorted(p.name for p in (root / 'tools/routes').glob('*.json'))
    sys.exit(0 if all([run(Path(sys.argv[1]), root / 'tools/routes' / name) for name in names]) else 1)
