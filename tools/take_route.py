#!/usr/bin/env python3
"""Build a closed-loop route for tests/route_bot.gd's `follow` step from a recorded take.

The route keeps the take's path and its action keys, not its movement keys:
Hype's sampled positions become a time-stamped track the bot steers along, and
every non-arrow key edge (attack, jump, sword, crossbow, magic, run, inventory,
menu and dialogue keys) is kept at its take time. Landmarks (level arrivals and
game-state phases) let the bot re-anchor its clock and report drift.

usage: take_route.py TAKE_DIR OUT.json [--until SECONDS]
"""
import argparse
import json
import math
from pathlib import Path

# X11 keysym -> Godot keycode. Arrows are steering, not replayed.
SPECIAL = {
    32: 32, 65293: 4194309, 65289: 4194306, 65307: 4194305, 65288: 4194308,
    65505: 4194325, 65506: 4194325, 65507: 4194326, 65508: 4194326,
}
ARROWS = {65361, 65362, 65363, 65364}


def godot_key(keysym):
    if keysym in SPECIAL:
        return SPECIAL[keysym]
    if 97 <= keysym <= 122:
        return keysym - 32
    if 48 <= keysym <= 57 or 65 <= keysym <= 90:
        return keysym
    if 65470 <= keysym <= 65481:  # F1..F12
        return 4194332 + keysym - 65470
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('take')
    parser.add_argument('out')
    parser.add_argument('--until', type=float, default=650.0)
    parser.add_argument('--select', action='append', default=[], metavar='TIME:ITEM',
                        help='inventory selection by item identity at the Enter press near TIME')
    args = parser.parse_args()
    events = Path(args.take) / 'recording/events.jsonl'
    track, keys, landmarks = [], [], []
    level = phase = None
    zero = None
    skipped = set()
    for line in events.open():
        event = json.loads(line)
        t = event['elapsed_micros'] / 1e6
        if t > args.until:
            break
        kind, data = event['kind'], event['data']
        if kind == 'state':
            state = data['state']
            if state.get('level') and state.get('level') != level:
                level = state['level']
                landmarks.append({'t': t, 'level': level})
                if zero is None and level == 'manoir':
                    zero = t
            if state.get('phase') != phase:
                phase = state.get('phase')
                landmarks.append({'t': t, 'phase': phase, 'level': level})
        elif kind == 'actor' and level:
            affine = data['actor']['global_transform']
            affine = affine.get('Affine', affine)
            x, y, z = affine['translation']
            basis = affine['basis']
            # Facing in the source ground plane, the angle the route bot's
            # heading() uses: the sampled yaw is 90 degrees ahead of it.
            facing = math.atan2(basis[0][1], basis[0][0]) - math.pi / 2
            track.append([round(t, 3), round(x, 3), round(y, 3), round(z, 3),
                          data['normal_behavior']['current'], level, round(facing, 4)])
        elif kind == 'input' and level:
            if data['keysym'] in ARROWS:
                continue
            code = godot_key(data['keysym'])
            if code is None:
                skipped.add(data['keysym'])
                continue
            keys.append([round(t, 3), code, data['edge'] == 'press'])
    route = {
        'start': 'new',
        'seed': 1,
        'steps': [{'follow': {'take_zero': zero, 'track': track, 'keys': keys, 'landmarks': landmarks,
                              'until': args.until,
                              'select': [[float(s.split(':')[0]), int(s.split(':')[1])] for s in args.select]}}],
    }
    Path(args.out).write_text(json.dumps(route))
    print(f'zero {zero:.1f} s, {len(track)} positions, {len(keys)} key edges, '
          f'{len(landmarks)} landmarks, skipped keysyms {sorted(skipped)}')


if __name__ == '__main__':
    main()
