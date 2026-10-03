"""Named, inventoried nextest jobs with mandatory safeguards and owned cleanup."""
import argparse
import ctypes.util
from dataclasses import asdict
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET

from selection import expression, identity, select, validate_inventory
from supervisor import Interrupted, interrupted, remove_temporary_tree, supervise


def source_digest(root):
    """Include untracked implementation files; a Git revision alone is insufficient."""
    paths = [root/'Cargo.toml', root/'Cargo.lock', root/'godot/project.godot']
    for directory in ('src', 'tests', 'tools', 'godot/addons', 'godot/game', 'godot/tests', 'crates'):
        paths.extend(p for p in (root/directory).rglob('*')
                     if p.is_file() and '__pycache__' not in p.parts)
    hasher = hashlib.sha256()
    for path in sorted(paths):
        hasher.update(str(path.relative_to(root)).encode()+b'\0')
        hasher.update(hashlib.sha256(path.read_bytes()).digest())
    return hasher.hexdigest()


def native_config(selected, report_path):
    lines = ['nextest-version = "0.9.145"', '[test-groups.engines]', 'max-threads = 1',
             '[profile.astrolabe]', 'retries = 0', 'test-threads = 2', 'fail-fast = false',
             'slow-timeout = { period = "5s", terminate-after = 1, grace-period = "2s" }',
             '[profile.astrolabe.junit]', 'path = ' + json.dumps(str(report_path)),
             'store-success-output = true', 'store-failure-output = true']
    for row in selected:
        lines += ['[[profile.astrolabe.overrides]]', 'filter = ' + json.dumps(expression([row])),
                  'slow-timeout = { period = "' + str(row['timeout_seconds']) +
                  's", terminate-after = 1, grace-period = "2s" }']
        if {'godot', 'original', 'xvfb', 'private_audio'} & set(row['prerequisites']):
            lines.append('test-group = "engines"')
    return '\n'.join(lines) + '\n'


def prerequisites(environment, root):
    engine = environment.get('ASTROLABE_GODOT', 'godot')
    game = Path(environment.get('ASTROLABE_GAME_DIR', root/'disc'))
    return {'godot': bool(shutil.which(engine)),
            'xvfb': bool(shutil.which(environment.get('ASTROLABE_XVFB_RUN', 'xvfb-run'))),
            'disc': (game/'Gamedata/World/Levels/Fix.sna').is_file(),
            'liblzo2': bool(ctypes.util.find_library('lzo2')),
            'ffmpeg': bool(shutil.which('ffmpeg')),
            'private_audio': all(shutil.which(tool) for tool in
                                 ('pipewire', 'pipewire-pulse', 'wireplumber', 'pactl')),
            'original': Path(environment.get('ASTROLABE_HYPE_CONFIG', '/nonexistent')).is_file()
                        and all(shutil.which(tool) for tool in
                                ('gamescope', 'pipewire', 'pipewire-pulse', 'wireplumber', 'pactl'))}


def junit_results(path, selected):
    wanted = {identity(r) for r in selected}
    found = {}
    for case in ET.parse(path).iter('testcase'):
        key = case.attrib['classname'] + '/' + case.attrib['name']
        if key in found or key not in wanted:
            raise ValueError(f'unexpected or duplicate executed test: {key}')
        status = ('failed' if case.find('failure') is not None or case.find('error') is not None
                  else 'not_run' if case.find('skipped') is not None else 'passed')
        found[key] = {'status': status, 'elapsed_seconds': float(case.get('time', '0'))}
    missing = wanted - found.keys()
    for key in missing:
        found[key] = {'status': 'not_run', 'reason': 'missing execution result'}
    return found


def main():
    started = time.monotonic()
    root = Path(__file__).resolve().parent.parent
    manifest_path = root/'tools/test-manifest.json'
    manifest = json.loads(manifest_path.read_text())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--area', action='append', choices=manifest['areas'], default=[])
    parser.add_argument('--release', action='store_true', help='require the entire declared inventory')
    parser.add_argument('--local', action='store_true', help='include local capabilities in the selected areas')
    parser.add_argument('--list', action='store_true', help='build/discover and report selection without executing tests')
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--game', type=Path)
    parser.add_argument('--godot')
    parser.add_argument('--headless', action=argparse.BooleanOptionalAction, default=True,
                        help='run Godot gameplay headless (use --no-headless for rendered gameplay in isolated Xvfb)')
    parser.add_argument('--original-config', type=Path, help='existing private Hype fixture configuration')
    parser.add_argument('--nextest', default=os.environ.get('ASTROLABE_NEXTEST'))
    parser.add_argument('--timeout', type=float, help='job budget including build and cleanup; no universal cap')
    parser.add_argument('--log', type=Path, help='new log path; sibling artifacts retain inventory and per-test evidence')
    args = parser.parse_args()
    if args.release and args.area:
        parser.error('--release cannot exclude areas')
    suite = 'release' if args.release else 'development' if args.area else 'local' if args.local else 'routine'
    budget = args.timeout if args.timeout is not None else manifest['budgets_seconds'][suite]
    if not math.isfinite(budget) or budget <= 15:
        parser.error('--timeout must be finite and greater than the 15-second cleanup reserve')
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    log = (args.log or root/'TestResults'/f'{stamp}-{suite}.log').resolve()
    evidence = log.with_suffix(log.suffix+'.d')
    report_path = log.with_suffix(log.suffix+'.json')
    if any(p.exists() for p in (log, evidence, report_path)):
        parser.error('log/report/evidence path already exists')
    log.parent.mkdir(parents=True, exist_ok=True)
    evidence.mkdir()
    environment = os.environ.copy()
    environment.update(PYTHONDONTWRITEBYTECODE='1', CARGO_TERM_COLOR='never')
    environment['ASTROLABE_GODOT_HEADLESS'] = '1' if args.headless else '0'
    environment.setdefault('RUST_BACKTRACE', '1')
    if args.game:
        environment['ASTROLABE_GAME_DIR'] = str(args.game.resolve())
    if args.godot:
        environment['ASTROLABE_GODOT'] = str(Path(args.godot).resolve()) if os.sep in args.godot else args.godot
    if args.original_config:
        environment['ASTROLABE_HYPE_CONFIG'] = str(args.original_config.resolve())
    else:
        prepared = root.parent/'hype-harness/local/config.json'
        if prepared.is_file():
            environment.setdefault('ASTROLABE_HYPE_CONFIG', str(prepared))
    environment['ASTROLABE_EVIDENCE_DIR'] = str(evidence)
    tool = args.nextest or shutil.which('cargo-nextest')
    local_tool = root/'output/test-tools'/f'nextest-{manifest["nextest_version"]}'/'cargo-nextest'
    if not tool and local_tool.is_file():
        tool = str(local_tool)
    if tool:
        tool = shutil.which(tool) or str(Path(tool).resolve())
    temporary = Path(tempfile.mkdtemp(prefix='astrolabe-suite-'))
    environment['TMPDIR'] = str(temporary)
    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    report = {'schema': 1, 'status': 'failed', 'suite': suite, 'areas': args.area,
              'godot_headless': args.headless,
              'job_budget_seconds': budget, 'source_revision': None,
              'manifest_sha256': hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
              'selected': [], 'excluded': [], 'stages': [], 'results': {}, 'error': None}
    report['source_sha256'] = source_digest(root)
    code = 0

    def stage(name, command, limit, echo=True):
        remaining = budget - (time.monotonic() - started) - 15
        if remaining <= 0:
            raise TimeoutError('job budget exhausted; pending checks have not passed')
        stage_log = evidence/(name+'.log')
        result = supervise(command, cwd=root, environment=environment, log_path=stage_log,
                           started=time.monotonic(), timeout=min(limit, remaining), echo=echo)
        report['stages'].append(dict(asdict(result), name=name, command=command, log=str(stage_log)))
        with log.open('ab') as output:
            output.write(stage_log.read_bytes())
        return result, stage_log

    try:
        print(f'{suite}: budget {budget:g}s including build and cleanup; '
              f'Godot gameplay {"headless" if args.headless else "rendered in isolated Xvfb"}; evidence {evidence}', flush=True)
        if not tool:
            raise RuntimeError('cargo-nextest 0.9.145 is required; set ASTROLABE_NEXTEST or --nextest')
        result, version_log = stage('tool-version', [tool, 'nextest', '--version'], 5, False)
        version = version_log.read_text().strip()
        if result.exit_code or not version.startswith('cargo-nextest '+manifest['nextest_version']+' '):
            raise RuntimeError(f'nextest version differs from the reviewed pin: {version}')
        report['nextest_version'] = version
        environment['ASTROLABE_NEXTEST'] = str(Path(tool).resolve())
        revision = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=root, capture_output=True, text=True, timeout=5)
        report['source_revision'] = revision.stdout.strip() or None
        dirty = subprocess.run(['git', 'diff', '--binary', 'HEAD'], cwd=root, capture_output=True, timeout=5)
        report['tracked_diff_sha256'] = hashlib.sha256(dirty.stdout).hexdigest()
        build = [tool, 'nextest', 'list', '--workspace', '--all-targets', '--cargo-profile', 'validation',
                 '--locked', '--message-format', 'json']
        if args.offline:
            build.append('--offline')
        result, build_log = stage('build-and-discover', build, manifest['budgets_seconds']['build'], False)
        if result.exit_code:
            code = result.exit_code
            raise RuntimeError(f'build/discovery failed; see {build_log}')
        # Cargo diagnostics share the capture; JSON is the complete final line.
        inventory = json.loads(next(line for line in reversed(build_log.read_text().splitlines())
                                    if line.startswith('{')))
        (evidence/'inventory.json').write_text(json.dumps(inventory, indent=2)+'\n')
        validate_inventory(manifest, inventory)
        declared_checks = [r['script'] for r in manifest['checks']]
        actual_checks = sorted(p.name for p in (root/'tools').glob('test_*.py'))
        if len(declared_checks) != len(set(declared_checks)) or sorted(declared_checks) != actual_checks:
            raise ValueError('unclassified, duplicate or missing supervisor check modules')
        selected, excluded = select(manifest, args.area, args.local, args.release)
        report.update(selected=selected, excluded=excluded, discovered_count=inventory['test-count'])
        for row in selected:
            print(f'SELECT {identity(row)}: {row["reason"]} ({row["timeout_seconds"]}s)', flush=True)
        for row in excluded:
            print(f'EXCLUDE {identity(row)}: {row["reason"]}', flush=True)
        if args.list:
            report['status'] = 'not_run'
        else:
            available = prerequisites(environment, root)
            report['capabilities'] = available
            executable = []
            for row in selected:
                missing = [p for p in row['prerequisites'] if not available[p]]
                if missing:
                    report['results'][identity(row)] = {'status': 'unavailable', 'reason': f'missing {missing}'}
                    code = 1
                else:
                    executable.append(row)
            for check in manifest['checks']:
                command = [sys.executable, '-B', '-m', 'unittest', 'discover', '-s', str(root/'tools'), '-p', check['script'], '-v']
                result, _ = stage(check['script'], command, check['timeout_seconds'])
                code = code or result.exit_code
            if executable:
                junit = evidence/'tests.xml'
                config = evidence/'nextest.toml'
                config.write_text(native_config(executable, junit))
                command = [tool, 'nextest', 'run', '--config-file', str(config), '--profile', 'astrolabe',
                           '--workspace', '--all-targets', '--cargo-profile', 'validation', '--locked',
                           '--run-ignored', 'all', '--no-tests', 'fail', '-E', expression(executable)]
                if args.offline:
                    command.append('--offline')
                if not args.headless:
                    xvfb_run = shutil.which('xvfb-run')
                    if not xvfb_run:
                        raise RuntimeError('--no-headless requires xvfb-run; refusing to use the desktop display')
                    command = [xvfb_run, '-a', '-s', '-screen 0 1024x768x24 -nolisten tcp',
                               'env', '-u', 'WAYLAND_DISPLAY', 'XDG_SESSION_TYPE=x11', *command]
                result, _ = stage('execute', command, budget)
                code = code or result.exit_code
                if junit.is_file():
                    report['results'].update(junit_results(junit, executable))
                else:
                    code = code or 1
            for row in selected:
                report['results'].setdefault(identity(row), {'status': 'not_run', 'reason': 'execution did not report this case'})
            if any(r['status'] != 'passed' for r in report['results'].values()):
                code = code or 1
            report['status'] = 'passed' if code == 0 else 'failed'
    except (OSError, ValueError, RuntimeError, TimeoutError, StopIteration, ET.ParseError, Interrupted) as exc:
        code = code or (124 if isinstance(exc, TimeoutError) else 128+exc.signum if isinstance(exc, Interrupted) else 125)
        report['error'] = str(exc) or type(exc).__name__
        print(report['error'], file=sys.stderr)
    finally:
        for sig in handlers:
            signal.signal(sig, signal.SIG_IGN)
        cleanup_start = time.monotonic()
        cleaned = remove_temporary_tree(temporary, cleanup_start+manifest['budgets_seconds']['cleanup'])
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
        elapsed = time.monotonic()-started
        report['source_unchanged'] = source_digest(root) == report['source_sha256']
        if not report['source_unchanged']:
            code = code or 125
            report['error'] = 'implementation changed during validation; results are stale'
        if not cleaned or elapsed > budget:
            code = code or 125
        if code:
            report['status'] = 'failed'
        for row in report['selected']:
            report['results'].setdefault(identity(row), {'status': 'not_run', 'reason': 'execution not completed'})
        report.update(exit_code=code, elapsed_seconds=elapsed,
                      temporary_cleanup_seconds=time.monotonic()-cleanup_start,
                      temporary_cleanup_complete=cleaned, temporary_directory=None if cleaned else str(temporary))
        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        report.update(child_cpu_seconds=usage.ru_utime+usage.ru_stime, peak_child_rss_kib=usage.ru_maxrss)
        report_path.write_text(json.dumps(report, indent=2)+'\n')
        print(f'{report["status"].upper()}: {elapsed:.3f}s; report: {report_path}', flush=True)
    return code
