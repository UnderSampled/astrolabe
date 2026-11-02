#!/usr/bin/env python3
"""Capture the original's New Game intro and compare it with the port's.

The intro needs no input after the menu's Return. Run under a private Orrery:
  tools/intro_compare.py capture --port P --token-file T --work DIR
  tools/intro_compare.py compare --work DIR PORT_INTRO_REPORT.json
Capture samples Hype, Gogoud and the active camera against time since manoir
entry and takes a screenshot every ten seconds. Compare prints both timelines of
behavior changes and the camera distance over time.
"""
import argparse
import json
import math
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from save_live_check import Mcp  # noqa: E402

SECONDS = 225
GOGOUD = [27, 35, 80]
WORLD = [26, 33, 78]
CAMERA = [2, 2, 2]


def actor(mcp, types):
    try:
        found = mcp.call('find_scene_actor', level='manoir', object_types=types)['observation']
        return {'action': found['actor']['current_action'], 'normal': (found.get('normal') or {}).get('current', -1),
                'position': translation(found['actor'])}
    except RuntimeError as error:
        return {'error': str(error)[:80]}


def translation(row):
    transform = row['global_transform']
    return transform.get('translation') or transform.get('Affine', {}).get('translation')


def capture(args):
    mcp = Mcp(args.port, args.token_file.read_text().strip())
    args.work.mkdir(parents=True, exist_ok=True)
    mcp.call('start_game', duration_seconds=SECONDS+90)
    mcp.call('run_sequence', steps=[{'action': 'keys', 'keys': ['Return'], 'hold_ms': 55, 'gap_ms': 800},
                                    {'action': 'wait_state', 'phase': 9, 'level': 'manoir', 'timeout_ms': 40000}])
    started = time.monotonic()
    samples, next_shot = [], 0
    while (now := time.monotonic()-started) < SECONDS:
        row = {'s': round(now, 2)}
        try:
            data = mcp.call('inspect_main_actor', actions=[47], designer_values=[])['observation']
            row['hype'] = {'action': data['actor']['current_action'], 'normal': data['normal']['current'],
                           'reflex': data.get('reflex', {}).get('current', -1), 'position': translation(data['actor'])}
            row['gogoud'] = actor(mcp, GOGOUD)
            row['world'] = actor(mcp, WORLD)
            camera = mcp.call('inspect_scene_camera', level='manoir', object_types=CAMERA)['observation']['active_camera']
            row['camera'] = {'position': camera['position_source'], 'forward': camera['forward_source'], 'fov': camera['vertical_fov']}
        except RuntimeError as error:
            row['error'] = str(error)[:120]
        if now >= next_shot:
            next_shot += 10
            try:
                row['shot'] = str(mcp.call('screenshot'))[:160]
            except RuntimeError as error:
                row['shot_error'] = str(error)[:120]
        samples.append(row)
    mcp.call('stop_game')
    (args.work/'intro.original.json').write_text(json.dumps(samples))
    print(f'{len(samples)} samples')


def changes(samples, read):
    last, out = None, []
    for row in samples:
        value = read(row)
        if value is not None and value != last:
            out.append((row['s'], value))
            last = value
    return out


def compare(args):
    first = json.loads((args.work/'intro.original.json').read_text())
    report = json.loads(args.port_report.read_text())
    second = report['samples']
    for name, key in (('hype normal', ('hype', 'normal')), ('hype action', ('hype', 'action')),
                      ('gogoud normal', ('gogoud', 'normal')), ('world normal', ('world', 'normal'))):
        read = lambda row: row.get(key[0], {}).get(key[1])
        print(name)
        print('  original', changes(first, read))
        print('  port    ', changes(second, read))
    print('dialogues', changes(second, lambda row: row.get('dialogue') or None))
    print('s   camera original -> port, gap; hype original -> port, gap')
    for mark in range(0, SECONDS, 10):
        a = min(first, key=lambda row: abs(row['s']-mark))
        b = min(second, key=lambda row: abs(row['s']-mark))
        if 'camera' not in a or 'camera' not in b:
            continue
        ca, cb, ha, hb = a['camera']['position'], b['camera']['position'], a['hype']['position'], b['hype']['position']
        print(mark, [round(v, 1) for v in ca], [round(v, 1) for v in cb], round(math.dist(ca, cb), 2),
              [round(v, 1) for v in ha], [round(v, 1) for v in hb], round(math.dist(ha, hb), 2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    one = commands.add_parser('capture')
    one.add_argument('--port', type=int, required=True)
    one.add_argument('--token-file', type=Path, required=True)
    one.add_argument('--work', type=Path, required=True)
    two = commands.add_parser('compare')
    two.add_argument('--work', type=Path, required=True)
    two.add_argument('port_report', type=Path)
    args = parser.parse_args()
    (capture if args.command == 'capture' else compare)(args)


if __name__ == '__main__':
    main()
