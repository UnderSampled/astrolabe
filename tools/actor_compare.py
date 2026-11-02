#!/usr/bin/env python3
"""Compare every Manoir actor between the running original and the port.

Both load the same save and wait. A difference here means AI that is not firing,
an actor in the wrong place, or a legitimate timing difference to look at.
"""
import argparse
import collections
import json
import math
import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from behavior_compare import LOAD  # noqa: E402
from save_live_check import Mcp  # noqa: E402

root = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--token-file', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--seconds', type=float, default=12)
    args = parser.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    census = args.work/'port.json'
    environment = dict(os.environ, XDG_DATA_HOME=str(args.work/'xdg'), XDG_CONFIG_HOME=str(args.work/'xdg'), XDG_CACHE_HOME=str(args.work/'xdg'))
    subprocess.run(['timeout', '-k', '5', '100', 'godot', '--headless', '--path', '.', '--audio-driver', 'Dummy',
                    '--script', 'res://tests/actor_census.gd', '--', str(census), str(args.seconds)],
                   cwd=root/'godot', env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    port = json.loads(census.read_text())
    counts = collections.Counter(tuple(row['object_types']) for row in port)
    mcp = Mcp(args.port, args.token_file.read_text().strip())
    mcp.call('start_game', duration_seconds=300)
    mcp.call('run_sequence', steps=LOAD+[{'action': 'wait', 'duration_ms': int(args.seconds*1000)}])
    rows = []
    for row in port:
        types = row['object_types']
        if counts[tuple(types)] != 1:
            rows.append({**row, 'result': 'ambiguous_types'})
            continue
        try:
            found = mcp.call('find_scene_actor', level='manoir', object_types=types)
        except RuntimeError as error:
            rows.append({**row, 'result': 'not_found_live', 'error': str(error)[:120]})
            continue
        live = found['observation']
        actor = live['actor']
        translation = actor['global_transform'].get('Affine', actor['global_transform'])['translation']
        rows.append({**row, 'result': 'compared', 'live_action': actor['current_action'], 'live_normal': (live.get('normal') or {}).get('current', -1),
                     'live_reflex': (live.get('reflex') or {}).get('current'), 'live_position': translation,
                     'distance': round(math.dist(translation, row['position']), 2)})
    mcp.call('stop_game')
    (args.work/'report.json').write_text(json.dumps(rows, indent=1)+'\n')
    compared = [row for row in rows if row['result'] == 'compared']
    same = [row for row in compared if row['normal'] == row['live_normal']]
    near = [row for row in compared if row['distance'] < 1]
    print(collections.Counter(row['result'] for row in rows))
    print(f'{len(same)} of {len(compared)} in the same behavior, {len(near)} within 1 unit')
    print('stopped in port:', [row['name'] for row in port if row['normal'] != -1 and not row['running']])
    for row in compared:
        if row['normal'] != row['live_normal'] or row['distance'] >= 1:
            print(row['name'][:30], 'normal', row['normal'], row['live_normal'], 'action', row['action'], row['live_action'], 'dist', row['distance'])


if __name__ == '__main__':
    sys.exit(main())
