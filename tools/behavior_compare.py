#!/usr/bin/env python3
"""Send the same keys to the original and to the port, then compare what happened.

The original runs from a disposable prefix through a private Orrery server. Each
engine reports the sequence of actions and behaviors, without repeats, plus total
distance. Sampling differs between engines, so timing is not judged here.
Writes docs/astrolabe/manoir-behaviors.md.
"""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from save_live_check import Mcp  # noqa: E402

root = Path(__file__).resolve().parent.parent
scenarios = json.loads((root/'tools/behaviors/manoir.json').read_text())
LOAD = [{'action': 'keys', 'keys': ['Down', 'Return', 'Return'], 'hold_ms': 55, 'gap_ms': 800},
        {'action': 'wait_state', 'phase': 9, 'level': 'manoir', 'timeout_ms': 25000},
        {'action': 'wait', 'duration_ms': 1500}]
HELD = {'LeftShift+ArrowUp': ['LeftShift', 'ArrowUp']}
CAMERA_TYPES = [2, 2, 2]


def condense(values):
    out = []
    for value in values:
        if not out or out[-1] != value:
            out.append(value)
    return out


def distance(points):
    return sum(math.dist(a, b) for a, b in zip(points, points[1:]))


def busy_seconds(samples):
    """Time from the first sample until the last non-idle sample."""
    busy = [s['ms'] for s in samples if s['action'] != 47]
    return round((busy[-1]-samples[0]['ms'])/1000, 2) if busy else 0.0


def rise(samples, axis=2):
    heights = [s['position'][axis] for s in samples] or [0]
    return round(max(heights)-heights[0], 2)


def offset(camera, samples):
    """Camera distance from Hype and height above him, in source units."""
    if not camera or not samples:
        return None
    a, b = camera['position'], samples[-1]['position']
    return [round(math.dist(a, b), 2), round(a[2]-b[2], 2)]


def observe(mcp):
    data = mcp.call('inspect_main_actor', actions=[47], designer_values=[])['observation']
    transform = data['actor']['global_transform']
    matrix = json.dumps(transform)
    return {'ms': time.monotonic()*1000, 'action': data['actor']['current_action'], 'normal': data['normal']['current'],
            'position': transform.get('translation') or transform.get('Affine', {}).get('translation') or [0, 0, 0],
            'raw': matrix[:0]}


def original(mcp, scenario):
    mcp.call('start_game', duration_seconds=120)
    mcp.call('run_sequence', steps=LOAD)
    # The original plays an entry sequence after loading and ignores keys during it.
    entry = [observe(mcp)]
    deadline = time.monotonic()+20
    while entry[-1]['action'] != 47 and time.monotonic() < deadline:
        entry.append(observe(mcp))
    samples = [entry[-1]]
    started = time.monotonic()
    for step in scenario['steps']:
        if step['key'] == 'wait':
            end = time.monotonic()+step['hold_ms']/1000
            while time.monotonic() < end:
                samples.append(observe(mcp))
        else:
            keys = HELD.get(step['key'], [step['key']])
            if len(keys) == 1:
                mcp.call('physical_key_sequence', steps=[{'key': keys[0], 'hold_ms': step['hold_ms'], 'gap_ms': 0}])
            else:
                mcp.call('press_keys', keys=['Shift_L+Up'], hold_ms=step['hold_ms'], gap_ms=0)
            samples.append(observe(mcp))
    while time.monotonic()-started < scenario['seconds']:
        samples.append(observe(mcp))
    camera = mcp.call('inspect_scene_camera', level='manoir', object_types=CAMERA_TYPES)
    active = camera['observation']['active_camera']
    mcp.call('stop_game')
    return {'camera': {'position': active['position_source'], 'forward': active['forward_source'],
                       'horizontal_fov': active['horizontal_fov'], 'vertical_fov': active['vertical_fov']},
            'entry': condense(row['action'] for row in entry), 'samples': samples}


def port(scenario, directory):
    output = directory/f'{scenario["name"]}.godot.json'
    environment = dict(os.environ, XDG_DATA_HOME=str(directory/'xdg'), XDG_CONFIG_HOME=str(directory/'xdg'),
                       XDG_CACHE_HOME=str(directory/'xdg'))
    subprocess.run(['timeout', '-k', '5', '90', 'godot', '--headless', '--path', '.', '--audio-driver', 'Dummy',
                    '--script', 'res://tests/behavior_probe.gd', '--', str(root/'tools/behaviors/manoir.json'),
                    scenario['name'], str(output)], cwd=root/'godot', env=environment,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if not output.is_file():
        return [], None
    data = json.loads(output.read_text())
    return data['samples'], data.get('camera')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--token-file', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--only', action='append', default=[])
    args = parser.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    mcp = Mcp(args.port, args.token_file.read_text().strip())
    rows = []
    for scenario in scenarios:
        if args.only and scenario['name'] not in args.only:
            continue
        source = args.work/f'{scenario["name"]}.original.json'
        if source.is_file():
            first = json.loads(source.read_text())
        else:
            first = original(mcp, scenario)
            source.write_text(json.dumps(first))
        camera = first.get('camera')
        first = first['samples']
        second, port_camera = port(scenario, args.work)
        a, b = condense(s['action'] for s in first), condense(s['action'] for s in second)
        n, m = condense(s['normal'] for s in first), condense(s['normal'] for s in second)
        # Orrery cannot sample during a held key, so the original's sequence is
        # coarser. Every state it saw must appear in order in the port's.
        remaining = iter(b)
        state = 'match' if all(action in remaining for action in a) else 'partial' if set(a) & set(b) else 'differs'
        rows.append({'name': scenario['name'], 'state': state, 'original_actions': a, 'port_actions': b,
                     'original_behaviors': n, 'port_behaviors': m,
                     'original_moved': round(math.dist(first[0]['position'], first[-1]['position']), 2),
                     'port_moved': round(math.dist(second[0]['position'], second[-1]['position']), 2) if second else None,
                     'port_alive': bool(second) and second[-1]['running'],
                     'original_seconds': busy_seconds(first), 'port_seconds': busy_seconds(second),
                     'original_rise': rise(first), 'port_rise': rise(second),
                     'camera_offset_original': offset(camera, first), 'camera_offset_port': offset(port_camera, second),
                     'vertical_fov': [camera and camera['vertical_fov'], port_camera and port_camera['vertical_fov']]})
    (args.work/'report.json').write_text(json.dumps(rows, indent=1)+'\n')
    for row in rows:
        print(json.dumps(row))
    matched = sum(row['state'] == 'match' for row in rows)
    print(f'{matched} of {len(rows)} scenarios have the same action sequence')


if __name__ == '__main__':
    sys.exit(main())
