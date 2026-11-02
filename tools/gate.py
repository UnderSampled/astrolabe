#!/usr/bin/env python3
"""The merge gate: run at merge points, not continuously, and keep it small.

It answers one question: is this still a game? Adding a fourth check means
removing one; broader evidence belongs in tools/test.py. Every game check starts
from a loaded save; the intro runs with the full route in tools/route_check.py. See
docs/astrolabe/testing.md.
"""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time

MAX_CHECKS = 3
BUDGET_SECONDS = 600
root = Path(__file__).resolve().parent.parent
work = root/'output/gate'
harness = root.parent/'hype-harness/local'


def gpu_prefix():
    """Render on real hardware without opening a window on the desktop."""
    if shutil.which('gamescope') and shutil.which('nvidia-smi'):
        return ['gamescope', '--backend', 'headless', '-w', '1024', '-h', '768', '-W', '1024', '-H', '768', '--',
                'env', '__NV_PRIME_RENDER_OFFLOAD=1', '__GLX_VENDOR_LIBRARY_NAME=nvidia']
    return ['xvfb-run', '-a', '-s', '-screen 0 1024x768x24']


def prepare_save():
    """Load Game restores a real slot, so the gate converts one."""
    slot = work/'slot/SaveGame0'
    slot.mkdir(parents=True)
    subprocess.run(['unzip', '-qjo', str(root/'reference/Savefile_EN.zip'), '*.sav', '-d', str(slot)], check=True)
    subprocess.run([str(root/'target/release/astrolabe'), 'import-save', str(slot), '--game', str(root/'disc'),
                    '--profile', 'hype-pc-english-supplied-save'], cwd=root, check=True, stdout=subprocess.DEVNULL)


def run(name, command, cwd, environment, limit):
    log = work/f'{name}.log'
    with log.open('wb') as output:
        child = subprocess.Popen(command, cwd=cwd, env=environment, stdout=output, stderr=subprocess.STDOUT,
                                 start_new_session=True)
        try:
            child.wait(limit)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
            return None
        finally:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    return child.returncode


def godot_check(name, script, report, environment, limit):
    xdg = work/'xdg'/name
    xdg.mkdir(parents=True)
    private = dict(environment, XDG_DATA_HOME=str(xdg), XDG_CONFIG_HOME=str(xdg), XDG_CACHE_HOME=str(xdg))
    command = [*gpu_prefix(), 'godot', '--path', '.', '--audio-driver', 'Dummy', '--script', script, '--', str(report)]
    code = run(name, command, root/'godot', private, limit)
    if code is None or not report.is_file():
        return {'passed': False, 'detail': 'timed out' if code is None else f'no report, exit {code}'}
    data = json.loads(report.read_text())
    return {'passed': data['outcome'] == 'passed', 'detail': data.get('message', ''),
            'checks': data.get('checks', []), 'frames': data.get('frames'), 'stand_ins': data.get('stand_ins', [])}


def controls(environment):
    return godot_check('controls', 'res://tests/controls_probe.gd', work/'controls.json', environment, 120)


def route(environment):
    """Load the save, walk to the level exit and leave it, through the real game."""
    xdg = work/'xdg/route'
    xdg.mkdir(parents=True)
    private = dict(environment, XDG_DATA_HOME=str(xdg), XDG_CONFIG_HOME=str(xdg), XDG_CACHE_HOME=str(xdg))
    command = [*gpu_prefix(), 'godot', '--path', '.', '--audio-driver', 'Dummy', '--script', 'res://tests/route_bot.gd',
               '--', str(root/'tools/routes/save_exit.json'), str(work/'route.json')]
    code = run('route', command, root/'godot', private, 120)
    log = (work/'route.log').read_text(errors='replace')
    failed = [line for line in log.splitlines() if line.startswith('SCRIPT ERROR') or 'ROUTE ABORT' in line]
    return {'passed': 'ROUTE SESSION ENDED' in log and not failed, 'detail': f'exit {code}, {failed[:2]}'}


def original(environment):
    """Live original data against the parser, on a disposable copy of the prefix."""
    if not (harness/'config.json').is_file():
        return {'passed': False, 'detail': f'{harness} is not prepared'}
    local = work/'original/local'
    local.mkdir(parents=True)
    subprocess.run(['cp', '-a', '--reflink=auto', str(harness/'prefix'), str(local/'prefix')], check=True)
    for name in ('Savefile_EN.zip', 'cheat-bindings.json', 'input-actions.json', 'graphics.json', 'gamescope-env.json'):
        shutil.copy2(harness/name, local/name)
    (local/'lock').touch()
    text = (harness/'config.json').read_text().replace(str(harness/'prefix'), str(local/'prefix'))
    (local/'config.json').write_text(text)
    evidence = work/'original/evidence'
    evidence.mkdir()
    code = run('original', ['cargo', 'nextest', 'run', '--cargo-profile', 'validation', '--locked', '--offline',
                            '--run-ignored', 'all', '-p', 'orrery', '--test', 'comparison_original'],
               root, dict(environment, ASTROLABE_HYPE_CONFIG=str(local/'config.json'),
                          ASTROLABE_GAME_DIR=str(root/'disc'), ASTROLABE_EVIDENCE_DIR=str(evidence)), 240)
    reports = list(evidence.glob('*/comparison.json'))
    counts = json.loads(reports[0].read_text())['direct']['counts'] if reports else None
    return {'passed': code == 0, 'detail': f'exit {code}, fields {counts}'}


CHECKS = [controls, route, original]


def main():
    only = sys.argv[1:]
    if len(CHECKS) > MAX_CHECKS:
        sys.exit(f'the gate is capped at {MAX_CHECKS} checks; remove one before adding another')
    started = time.monotonic()
    nextest = root/'output/test-tools/nextest-0.9.145'
    environment = dict(os.environ, PATH=f'{nextest}:{os.environ["PATH"]}', TMPDIR=str(work/'tmp'))
    version = json.loads((root/'godot/addons/astrolabe/version.json').read_text())['interface']
    manifest = root/'godot/generated/astrolabe-manifest.json'
    if not manifest.is_file() or json.loads(manifest.read_text())['support_interface'] != version:
        sys.exit('generated content is missing or stale: run target/release/astrolabe prepare-game disc godot')
    shutil.rmtree(work, ignore_errors=True)
    (work/'tmp').mkdir(parents=True)
    prepare_save()
    results = {}
    for check in CHECKS:
        if only and check.__name__ not in only:
            continue
        begun = time.monotonic()
        results[check.__name__] = check(environment)
        results[check.__name__]['seconds'] = round(time.monotonic() - begun, 1)
        result = results[check.__name__]
        print(f'{"PASS" if result["passed"] else "FAIL"} {check.__name__} {result["seconds"]}s {result["detail"]}', flush=True)
        for row in result.get('checks', []):
            print(f'    {"ok" if row["passed"] else "FAIL"} {row["name"]}: {row["detail"]}')
        if result.get('frames'):
            print(f'    frames: {result["frames"]}')
    reached = {}
    for result in results.values():
        for row in result.get('stand_ins', []):
            reached[row['id']+' '+row['context'][:110]] = reached.get(row['id']+' '+row['context'][:110], 0)+row['count']
    print(f'stand-ins reached: {len(reached)}')
    for key, count in sorted(reached.items(), key=lambda item: -item[1]):
        print(f'    {count} {key}')
    elapsed = time.monotonic() - started
    passed = all(result['passed'] for result in results.values()) and elapsed <= BUDGET_SECONDS
    (work/'report.json').write_text(json.dumps({'passed': passed, 'seconds': elapsed, 'checks': results}, indent=2)+'\n')
    shutil.rmtree(work/'original/local', ignore_errors=True)
    print(f'{"GATE PASSED" if passed else "GATE FAILED"} in {elapsed:.0f}s (budget {BUDGET_SECONDS}s)')
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
