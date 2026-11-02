#!/usr/bin/env python3
"""Run saved route-bot routes (tools/routes/*.json) through the real game, on demand.

Not part of the gate: a route takes minutes. Usage:
  tools/route_check.py [--full] HANDOFF_DIR [route ...]
--full plays the whole New Game intro first; the intro belongs to the full route.
HANDOFF_DIR is a pristine XDG data directory holding the New Game handoff save
(user://manoir-save.cfg). Each run starts from a fresh copy. A route whose
"required" step fails is retried. Each route also needs a pass check: the final STATE
line is printed and any SCRIPT ERROR or ROUTE ABORT fails the run.
"""
import json, os, shutil, subprocess, sys, tempfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root / 'tools'))
from gate import gpu_prefix

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

def intro() -> bool:
    with tempfile.TemporaryDirectory() as scratch:
        report = Path(scratch) / 'intro.json'
        env = dict(os.environ, XDG_DATA_HOME=scratch, XDG_CONFIG_HOME=scratch, XDG_CACHE_HOME=scratch)
        command = [*gpu_prefix(), 'godot', '--path', str(root / 'godot'), '--audio-driver', 'Dummy', '--script',
                   'res://tests/new_game_intro_acceptance.gd', '--', str(report)]
        subprocess.run(command, env=env, stdin=subprocess.DEVNULL, capture_output=True, timeout=300)
        passed = report.is_file() and json.loads(report.read_text())['outcome'] == 'passed'
    print(f'intro: {"ok" if passed else "FAIL"}')
    return passed

if __name__ == '__main__':
    args = sys.argv[1:]
    full = '--full' in args
    if full: args.remove('--full')
    if not args: sys.exit(__doc__)
    if full and not intro(): sys.exit(1)
    names = args[1:] or sorted(p.name for p in (root / 'tools/routes').glob('*.json'))
    sys.exit(0 if all([run(Path(args[0]), root / 'tools/routes' / name) for name in names]) else 1)
