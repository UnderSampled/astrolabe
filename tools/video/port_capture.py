#!/usr/bin/env python3
"""Capture the Godot port while tests/route_bot.gd drives it, with Movie Maker.

usage: port_capture.py ROUTE.json OUT_DIR [--fps 30] [--limit SECONDS] [--clock movie|wall] [--gentle]

Godot's Movie Maker (--write-movie) renders every frame at a fixed 1/FPS step
and mixes audio in step with it, so the capture has no dropped or doubled
frames and no contention with the encoder, whatever the machine's load; it runs
slower or faster than real time as rendering allows. It writes a PNG sequence
plus WAV, converted here to OUT_DIR/port.mkv (FFV1 + FLAC, 1024x768) and then
deleted (--keep-png keeps them).

The bot times its follow track, key edges and timeouts with Time.get_ticks_msec(),
which is wall time and so wrong under Movie Maker. --clock movie (the default)
runs a copy of route_bot.gd with every Time.get_ticks_msec() replaced by the
fixed-step clock, Engine.get_physics_frames() in milliseconds; --clock wall runs
the bot unchanged. Make the route with tools/take_route.py, e.g.
  tools/take_route.py TestResults/stephen-linear-brigand-20261102 OUT/route.json --until 330
The run uses a headless Gamescope and private XDG directories like the bot's
own runs. OUT_DIR/bot.log is the bot's output; OUT_DIR/capture.json records the
frame count, wall time and the bot's LANDMARK lines.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from videolib import probe, run, write_json  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
GODOT = ROOT / 'godot'
CLOCK = '''

## Movie Maker clock (tools/video/port_capture.py): game time from the fixed
## step, since Movie Maker does not run in real time.
func _movie_ms() -> int:
	return Engine.get_physics_frames() * 1000 / Engine.physics_ticks_per_second
'''


def bot_script(out, clock):
    source = GODOT / 'tests/route_bot.gd'
    if clock == 'wall':
        return 'res://tests/route_bot.gd'
    text = source.read_text()
    count = text.count('Time.get_ticks_msec()')
    patched = out / 'route_bot_movie.gd'
    patched.write_text(text.replace('Time.get_ticks_msec()', '_movie_ms()') + CLOCK)
    print(f'{patched}: {count} wall-clock reads replaced with the fixed-step clock')
    return str(patched)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('route')
    parser.add_argument('out')
    parser.add_argument('--fps', type=int, default=30)
    parser.add_argument('--size', default='1024x768')
    parser.add_argument('--limit', type=float, default=1800.0, help='wall-clock timeout for the Godot run')
    parser.add_argument('--clock', choices=['movie', 'wall'], default='movie')
    parser.add_argument('--keep-png', action='store_true')
    parser.add_argument('--gentle', action='store_true', help='low priority, four cores, capped at real time')
    parser.add_argument('--threads', type=int, default=4, help='ffmpeg threads for the conversion')
    args = parser.parse_args()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    frames = out / 'frames'
    if frames.exists():
        shutil.rmtree(frames)
    frames.mkdir()
    xdg = out / 'xdg'
    if xdg.exists():
        shutil.rmtree(xdg)
    xdg.mkdir()
    width, height = args.size.split('x')
    script = bot_script(out, args.clock)
    env = {'XDG_DATA_HOME': xdg, 'XDG_CONFIG_HOME': xdg, 'XDG_CACHE_HOME': xdg}
    # --gentle: lowest CPU and I/O priority on the last four cores, and no more
    # than real-time frame rate, to share the machine with a timing-sensitive run.
    gentle = (['nice', '-n', '19', 'ionice', '-c', '3', 'taskset', '-c', f'{os.cpu_count() - 4}-{os.cpu_count() - 1}']
              if args.gentle else [])
    cmd = [*gentle, 'env', *[f'{k}={v}' for k, v in env.items()],
           'timeout', '-k', '5', str(int(args.limit)),
           'gamescope', '--backend', 'headless', '-w', width, '-h', height, '-W', width, '-H', height, '--',
           'env', '__NV_PRIME_RENDER_OFFLOAD=1', '__GLX_VENDOR_LIBRARY_NAME=nvidia',
           'godot', '--path', GODOT, '--write-movie', frames / 'port.png', '--fixed-fps', str(args.fps),
           '--disable-vsync', *(['--max-fps', str(args.fps)] if args.gentle else []), '--resolution', args.size, '--script', script, '--',
           Path(args.route).resolve(), out / 'bot.json']
    began = time.monotonic()
    try:
        run(cmd, log=out / 'bot.log')
        status = 0
    except subprocess.CalledProcessError as error:
        status = error.returncode
    wall = time.monotonic() - began
    pngs = sorted(frames.glob('port*.png'))
    wav = frames / 'port.wav'
    if not pngs:
        raise SystemExit(f'no frames written (exit {status}); see {out / "bot.log"}')
    first = re.search(r'(\d+)\.png$', pngs[0].name).group(1)
    mkv = out / 'port.mkv'
    audio = ['-i', wav] if wav.exists() and wav.stat().st_size > 44 else []
    run(['nice', '-n', '10', 'ffmpeg', '-v', 'error', '-stats', '-y', '-threads', str(args.threads), '-framerate', str(args.fps),
         '-start_number', str(int(first)), '-i', frames / f'port%0{len(first)}d.png', *audio,
         '-map', '0:v', *(['-map', '1:a', '-c:a', 'flac'] if audio else []),
         '-c:v', 'ffv1', '-level', '3', '-g', '1', '-slices', '16', '-threads', str(args.threads), '-pix_fmt', 'bgr0', mkv])
    info = probe(mkv)
    log = (out / 'bot.log').read_text(errors='replace')
    landmarks = [json.loads(line.split(' ', 1)[1]) for line in log.splitlines() if line.startswith('LANDMARK ')]
    summary = {'route': str(Path(args.route).resolve()), 'exit': status, 'clock': args.clock, 'fps': args.fps,
               'frames': len(pngs), 'video_seconds': len(pngs) / args.fps, 'wall_seconds': round(wall, 1),
               'realtime_factor': round(len(pngs) / args.fps / wall, 3), 'audio': bool(audio),
               'width': info['width'], 'height': info['height'], 'landmarks': landmarks,
               'script_errors': [line for line in log.splitlines() if 'SCRIPT ERROR' in line][:20]}
    write_json(out / 'capture.json', summary)
    if not args.keep_png:
        shutil.rmtree(frames)
    shutil.rmtree(xdg, ignore_errors=True)
    print(json.dumps({k: v for k, v in summary.items() if k != 'landmarks'}))


if __name__ == '__main__':
    main()
