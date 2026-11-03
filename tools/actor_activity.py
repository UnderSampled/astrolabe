#!/usr/bin/env python3
"""Which Manoir actors run in the original, with Hype at each of several spots.

Under a private Orrery, loads the save, places Hype, and reads every actor twice.
An actor counts as running if its frame, behavior or position changed. Then
compares that with the sector lists in the generated level:
  tools/actor_activity.py --port P --token-file T --work DIR
"""
import argparse
import json
import math
from pathlib import Path
import re
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from behavior_compare import LOAD, observe, place  # noqa: E402
from save_live_check import Mcp  # noqa: E402

root = Path(__file__).resolve().parent.parent
GAP = 4
SPOTS = {'save': None, 'bridge': [-54.0, 125.0, 25.0], 'stairs': [-76.3, 41.9, 5.5], 'exit': [-75.0, 23.0, 4.5]}


def read(mcp, types):
    try:
        live = mcp.call('find_scene_actor', level='manoir', object_types=types)['observation']
    except RuntimeError:
        return None
    actor = live['actor']
    return {'frame': actor.get('current_frame'), 'action': actor['current_action'], 'normal': (live.get('normal') or {}).get('current'),
            'position': actor['global_transform'].get('Affine', actor['global_transform'])['translation']}


def sectors():
    """Actor path to its sector and each sector's activity list, from level.tscn."""
    owner, activity, name = {}, {}, None
    for line in (root/'godot/generated/hype/levels/manoir/level.tscn').read_text(errors='replace').splitlines():
        if line.startswith('sector_name = '):
            name = line.split('"')[1]
        elif name and line.startswith('persos = '):
            for path in re.findall(r'"([^"]+)"', line):
                owner[path] = name
        elif name and line.startswith('activity = '):
            activity[name] = re.findall(r'"([^"]+)"', line)
    return owner, activity


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--token-file', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    args = parser.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    rows = json.loads((root/'tools/behaviors/manoir-actors.json').read_text())
    unique = {}
    for row in rows:
        key = tuple(row['object_types'])
        unique[key] = None if key in unique else row
    actors = [row for row in unique.values() if row]
    mcp = Mcp(args.port, args.token_file.read_text().strip())
    mcp.call('start_game', duration_seconds=240)
    mcp.call('run_sequence', steps=LOAD)
    report = {}
    for spot, position in SPOTS.items():
        if position:
            place(mcp, position)
            time.sleep(6)
            if math.dist(observe(mcp)['position'], position) > 2:
                place(mcp, position)
                time.sleep(2)
        first = {row['asset']: read(mcp, row['object_types']) for row in actors}
        time.sleep(GAP)
        second = {row['asset']: read(mcp, row['object_types']) for row in actors}
        running = [asset for asset in first if first[asset] and second[asset] and first[asset] != second[asset]]
        idle = [asset for asset in first if first[asset] and second[asset] and first[asset] == second[asset]]
        report[spot] = {'hype': observe(mcp)['position'], 'running': running, 'idle': idle}
        print(spot, report[spot]['hype'], len(running), 'running', len(idle), 'idle', flush=True)
    mcp.call('stop_game')
    (args.work/'activity.json').write_text(json.dumps(report, indent=1)+'\n')
    owner, activity = sectors()
    for spot, row in report.items():
        hit = {}
        for key in ('running', 'idle'):
            for asset in row[key]:
                sector = owner.get(asset.removeprefix('level:/persos/'), '?')
                hit.setdefault(sector, [0, 0])[key == 'idle'] += 1
        print(spot, 'sector running/idle', dict(sorted(hit.items())))


if __name__ == '__main__':
    sys.exit(main())
