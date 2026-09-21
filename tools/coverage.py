#!/usr/bin/env python3
"""Collect a separate, provenance-bound Rust coverage run through the same suites."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys
import time

from suites import source_digest
from supervisor import Interrupted, interrupted, supervise

# Tests, generated resources, research code, and dependencies are not converter
# implementation. Handwritten src/ and companion crate code remain in the report.
EXCLUSIONS = r'(^|/)(tests|target|reference)/'


def main():
    root = Path(__file__).resolve().parent.parent
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--area', action='append', default=[])
    p.add_argument('--local', action='store_true')
    p.add_argument('--offline', action='store_true')
    p.add_argument('--tool', type=Path, default=root/'output/test-tools/llvm-cov-0.9.1/cargo-llvm-cov')
    p.add_argument('--output', type=Path)
    p.add_argument('--timeout', type=float, default=900)
    args = p.parse_args()
    import math
    if not math.isfinite(args.timeout) or args.timeout < 60:
        p.error('--timeout must be finite and at least 60 seconds')
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    output = (args.output or root/'TestResults/coverage'/stamp).resolve()
    if output.exists():
        p.error('coverage output already exists; runs must not merge implicitly')
    output.mkdir(parents=True)
    environment = os.environ.copy()
    for key, name in [('LLVM_COV', 'llvm-cov'), ('LLVM_PROFDATA', 'llvm-profdata')]:
        environment.setdefault(key, shutil.which(name) or name)
    # New output path -> new instrumentation and raw profiles. Never mix old data.
    environment['CARGO_TARGET_DIR'] = str(output/'instrumented')
    report = {'schema': 1, 'status': 'failed', 'source_sha256': source_digest(root),
              'scope': 'local-inclusive' if args.local else 'synthetic', 'areas': args.area,
              'exclusions': EXCLUSIONS, 'coverage_dimensions': ['lines', 'regions', 'functions'],
              'measures_gdscript_or_original_executable': False, 'commands': [],
              'merged_with_other_runs': False, 'ordinary_test_results_replaced': False}
    started = time.monotonic()
    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    code = 0

    def run(name, command, limit):
        left = args.timeout-(time.monotonic()-started)-5
        if left <= 0:
            raise TimeoutError('coverage budget exhausted')
        log = output/(name+'.log')
        result = supervise(command, cwd=root, environment=environment, log_path=log,
                           started=time.monotonic(), timeout=min(left, limit), echo=False)
        report['commands'].append({'name': name, 'command': command, 'seconds': result.elapsed_seconds,
                                   'exit_code': result.exit_code, 'log': str(log)})
        if result.exit_code:
            raise RuntimeError(f'{name} failed with exit {result.exit_code}; see {log}')
        return log.read_text()

    tool = str(args.tool.resolve())
    try:
        version = run('version', [tool, 'llvm-cov', '--version'], 5).strip()
        if version != 'cargo-llvm-cov 0.9.1':
            raise ValueError('coverage tool differs from reviewed 0.9.1 pin')
        report['tool_version'] = version
        rust = run('rust', ['rustc', '-vV'], 5)
        llvm = run('llvm', [environment['LLVM_COV'], '--version'], 5)
        prof = run('profdata', [environment['LLVM_PROFDATA'], '--version'], 5)
        import re
        versions = [re.search(r'LLVM version:?\s+(\d+\.\d+\.\d+)', text).group(1)
                    for text in (rust, llvm, prof)]
        if len(set(versions)) != 1:
            raise ValueError(f'incompatible Rust/LLVM coverage tools: {versions}')
        report.update(rustc=rust.strip(), llvm_version=versions[0])
        env_text = run('environment', [tool, 'llvm-cov', 'show-env', '--sh', '--no-cfg-coverage'], 15)
        for line in env_text.splitlines():
            if not line.startswith('export '):
                continue
            words = shlex.split(line)
            if len(words) != 2:
                raise ValueError('unrecognized coverage environment output')
            key, value = words[1].split('=', 1)
            if key not in {'RUSTC_WRAPPER', 'LLVM_PROFILE_FILE', 'CARGO_LLVM_COV', 'CARGO_LLVM_COV_SHOW_ENV',
                           'CARGO_LLVM_COV_TARGET_DIR', 'CARGO_LLVM_COV_BUILD_DIR',
                           '__CARGO_LLVM_COV_RUSTC_WRAPPER', '__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS',
                           '__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES'}:
                raise ValueError(f'unreviewed coverage environment key: {key}')
            environment[key] = value
        command = [sys.executable, '-B', str(root/'tools/test.py'), '--log', str(output/'tests.log'),
                   '--timeout', str(max(16, args.timeout-(time.monotonic()-started)-60))]
        for area in args.area:
            command += ['--area', area]
        if args.local:
            command.append('--local')
        if args.offline:
            command.append('--offline')
        run('instrumented-tests', command, args.timeout)
        base = [tool, 'llvm-cov', 'report', '--profile', 'validation', '--locked',
                '--ignore-filename-regex', EXCLUSIONS]
        if args.offline:
            base.append('--offline')
        run('json', base+['--json', '--output-path', str(output/'coverage.json')], 45)
        run('lcov', base+['--lcov', '--output-path', str(output/'coverage.lcov')], 45)
        run('html', base+['--html', '--output-dir', str(output/'html')], 60)
        raw = json.loads((output/'coverage.json').read_text())
        domains = {}
        for item in raw['data'][0]['files']:
            path = Path(item['filename'])
            if root/'src' in path.parents:
                relative = path.relative_to(root/'src')
                domain = relative.parts[0] if len(relative.parts) > 1 else 'application'
            elif root/'crates' in path.parents:
                domain = 'testkit'
            else:
                continue
            summary = domains.setdefault(domain, {k: {'covered': 0, 'count': 0} for k in ['lines','regions','functions']})
            for dimension, counts in summary.items():
                for key in counts:
                    counts[key] += item['summary'][dimension][key]
        if not domains:
            raise ValueError('coverage report contains no handwritten project code')
        report['domains'] = domains
        report['totals'] = raw['data'][0]['totals']
        report['status'] = 'passed'
    except (OSError, ValueError, RuntimeError, TimeoutError, Interrupted, AttributeError) as exc:
        code = 1
        report['error'] = str(exc)
        print(str(exc), file=sys.stderr)
    finally:
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
        report['source_unchanged'] = source_digest(root) == report['source_sha256']
        if not report['source_unchanged']:
            report.update(status='failed', error='implementation changed during instrumented run')
            code = 1
        report['elapsed_seconds'] = time.monotonic()-started
        (output/'summary.json').write_text(json.dumps(report, indent=2)+'\n')
        print(f'Coverage {report["status"]}: {output}/summary.json', flush=True)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
