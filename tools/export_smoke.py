#!/usr/bin/env python3
"""Exported-build smoke check: export the Linux preset and confirm the built game
reaches a playable Manoir from New Game.

The exported game ignores --script, so the check uses the bootstrap's
ASTROLABE_SMOKE=new hook (game/smoke_check.gd), which starts a New Game and
exits 0 once the Manoir is playable or 1 after a minute. Needs the standard
4.7.2 export templates in ~/.local/share/godot/export_templates/4.7.2.stable.

usage: export_smoke.py [release|debug]
"""
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else 'release'
    with tempfile.TemporaryDirectory(prefix='astrolabe-export-') as work:
        work = Path(work)
        binary = work / 'astrolabe.x86_64'
        export = subprocess.run(['godot', '--headless', f'--export-{mode}', 'Linux', str(binary)],
                                cwd=root / 'godot', capture_output=True, text=True, timeout=900)
        if export.returncode != 0 or not binary.is_file():
            print(f'FAIL export {mode}: exit {export.returncode}\n{export.stderr[-2000:]}')
            return 1
        home = work / 'xdg'
        home.mkdir()
        environment = dict(os.environ, ASTROLABE_SMOKE='new', XDG_DATA_HOME=str(home),
                           XDG_CONFIG_HOME=str(home), XDG_CACHE_HOME=str(home))
        prefix = []
        if shutil.which('gamescope'):
            prefix = ['gamescope', '--backend', 'headless', '-w', '1024', '-h', '768', '-W', '1024', '-H', '768', '--']
        try:
            run = subprocess.run([*prefix, str(binary), '--audio-driver', 'Dummy'], cwd=work, env=environment,
                                 capture_output=True, text=True, timeout=120)
            output = run.stdout + run.stderr
        except subprocess.TimeoutExpired as error:
            output = (error.stdout or b'').decode(errors='replace') if isinstance(error.stdout, bytes) else (error.stdout or '')
        lines = [line for line in output.splitlines() if line.startswith('SMOKE')]
        passed = any(line.startswith('SMOKE playable') for line in lines)
        print(f'{"PASS" if passed else "FAIL"} export {mode}: {lines[-1] if lines else "no smoke report"}')
        return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
