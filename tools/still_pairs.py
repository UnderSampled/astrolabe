#!/usr/bin/env python3
"""Pair original and port intro stills by Gogoud's animation state, not the clock.

Run under a private Orrery:
  tools/still_pairs.py capture --port P --token-file T --work DIR [--times 10,20,...]
  (port)  ASTROLABE_INTRO_PAIRS=DIR/pairs.json godot ... new_game_intro_acceptance.gd
  tools/still_pairs.py compare --work DIR --port-dir PORT_PNG_DIR   (needs numpy, pillow, scipy)
Capture reads Gogoud's action and animation frame (Perso3dData+0x78, exposed as
`current_frame` by find_scene_actor) right before and after each screenshot and
keeps only shots where the action is unchanged and the frame moved at most TOLERANCE
frames. Action ids are family state indices in both engines (character.current_state).
"""
import argparse
import json
import re
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from save_live_check import Mcp  # noqa: E402

GOGOUD = [27, 35, 80]
TOLERANCE = 3
TIMES = [8, 14, 20, 26, 32, 40, 46, 50, 56, 64, 72, 80, 90, 100, 115, 130]


def gogoud(mcp):
    actor = mcp.call('find_scene_actor', level='manoir', object_types=GOGOUD)['observation']['actor']
    return actor['current_action'], actor.get('current_frame')


def camera(mcp):
    return mcp.call('inspect_scene_camera', level='manoir', object_types=[2, 2, 2])['observation']['active_camera']['position_source']


def capture(args):
    mcp = Mcp(args.port, args.token_file.read_text().strip())
    args.work.mkdir(parents=True, exist_ok=True)
    times = [float(t) for t in args.times.split(',')]
    mcp.call('start_game', duration_seconds=int(times[-1]) + 90)
    mcp.call('run_sequence', steps=[{'action': 'keys', 'keys': ['Return'], 'hold_ms': 55, 'gap_ms': 800},
                                    {'action': 'wait_state', 'phase': 9, 'level': 'manoir', 'timeout_ms': 30000}])
    started, rows = time.monotonic(), []
    for want in times:
        while time.monotonic() - started < want:
            time.sleep(0.05)
        try:
            before = gogoud(mcp)
            t0 = time.monotonic()
            shot = str(mcp.call('screenshot'))
            seconds = time.monotonic() - t0
            after = gogoud(mcp)
            where = camera(mcp)
        except RuntimeError as error:
            rows.append({'t': want, 'error': str(error)[:100]})
            continue
        found = re.search(r'(/[^\s"\']+\.png)', shot)
        row = {'t': round(time.monotonic() - started, 1), 'shot': found.group(1) if found else shot[:160], 'shot_seconds': round(seconds, 2),
               'before': before, 'after': after, 'camera': where}
        row['accepted'] = (before[0] == after[0] and before[1] is not None and after[1] is not None
                           and 0 <= after[1] - before[1] <= TOLERANCE)
        if row['accepted']:
            row.update(action=before[0], frame_lo=before[1], frame_hi=after[1])
        rows.append(row)
        print(row, flush=True)
    mcp.call('stop_game')
    (args.work / 'pairs.json').write_text(json.dumps(rows, indent=1))
    print(sum(1 for r in rows if r.get('accepted')), 'accepted of', len(rows))


def compare(args):
    import numpy as np
    from PIL import Image
    from scipy import ndimage as nd
    rows = json.loads((args.work / 'pairs.json').read_text())
    ported = json.loads((args.port_dir / 'still_pairs.json').read_text()) if (args.port_dir / 'still_pairs.json').exists() else {}
    load = lambda p: np.asarray(Image.open(p).convert('RGB'), float) / 255
    grad = lambda g: (np.diff(g, axis=0)[:, :-1] ** 2 + np.diff(g, axis=1)[:-1, :] ** 2) ** .5
    for i, row in enumerate(rows):
        mine = ported.get(str(i))
        if not row.get('accepted') or not mine or mine['camera_gap'] > args.max_gap:
            continue
        b, a = load(row['shot']), load(mine['png'])
        if a.shape != b.shape:
            a = np.asarray(Image.fromarray((a * 255).astype('uint8')).resize((b.shape[1], b.shape[0])), float) / 255
        h, w = b.shape[:2]
        box = (w // 10, h // 8, w * 9 // 10, h * 3 // 4)
        x0, y0, x1, y1 = box
        raw = ((a[y0:y1, x0:x1] - b[y0:y1, x0:x1]) ** 2).mean() ** .5
        best, ga, gb = (9,), a.mean(2), b.mean(2)[y0:y1, x0:x1]
        for s in np.arange(0.97, 1.031, 0.004):
            z = nd.affine_transform(ga, [1 / s, 1 / s], offset=[h / 2 * (1 - 1 / s), w / 2 * (1 - 1 / s)], order=1)
            for dy in range(-8, 9, 2):
                for dx in range(-8, 9, 2):
                    e = ((z[y0 - dy:y1 - dy, x0 - dx:x1 - dx] - gb) ** 2).mean() ** .5
                    if e < best[0]:
                        best = (e, s, dx, dy)
        print(f"pair {i} t={row['t']} action {row['action']} frame {row['frame_lo']}-{row['frame_hi']} port frame {mine['frame']} camera gap {mine['camera_gap']:.2f}")
        print(f"  full-frame rmse raw {raw:.4f} fitted {best[0]:.4f} scale {best[1]:.3f} shift {best[2]},{best[3]}")
        gx0, gy0, gx1, gy1 = [int(v) for v in mine.get('actor_box', (0, 0, 0, 0))]
        gx0, gy0, gx1, gy1 = max(gx0, 0), max(gy0, 0), min(gx1, w), min(gy1, h)
        if gx1 - gx0 > 20 and gy1 - gy0 > 20:
            A, B = a[gy0:gy1, gx0:gx1], b[gy0:gy1, gx0:gx1]
            print(f"  gogoud box {gx0},{gy0},{gx1},{gy1} rmse {((A - B) ** 2).mean() ** .5:.4f} ratio {(A.mean((0, 1)) / B.mean((0, 1))).round(2)} "
                  f"sharp p90 {np.percentile(grad(A.mean(2)), 90):.3f}/{np.percentile(grad(B.mean(2)), 90):.3f}")
        for name, (u0, v0, u1, v1) in {'top': (x0, y0, x1, h // 2), 'bottom': (x0, h // 2, x1, y1)}.items():
            A, B = a[v0:v1, u0:u1], b[v0:v1, u0:u1]
            print(f"  {name}: ratio {(A.mean((0, 1)) / B.mean((0, 1))).round(2)} sharp p90 {np.percentile(grad(A.mean(2)), 90):.3f}/{np.percentile(grad(B.mean(2)), 90):.3f}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    one = commands.add_parser('capture')
    one.add_argument('--port', type=int, required=True)
    one.add_argument('--token-file', type=Path, required=True)
    one.add_argument('--work', type=Path, required=True)
    one.add_argument('--times', default=','.join(str(t) for t in TIMES))
    two = commands.add_parser('compare')
    two.add_argument('--work', type=Path, required=True)
    two.add_argument('--port-dir', type=Path, required=True)
    two.add_argument('--max-gap', type=float, default=1.0, help='largest camera distance to the original shot')
    args = parser.parse_args()
    (capture if args.command == 'capture' else compare)(args)


if __name__ == '__main__':
    main()
