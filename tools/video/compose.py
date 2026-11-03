#!/usr/bin/env python3
"""Stack the original and the port side by side, labeled, with the port's audio,
as H.265 in Matroska.

usage: compose.py LEFT.mkv RIGHT.mkv OUT.mkv [--left-start LM] [--left-end LM]
           [--right-start LM] [--right-end LM] [--encoder auto|nvenc|x265] [--dry-run]
       compose.py --timeline FILE START SECONDS

Each side plays from its start landmark to its end landmark (inclusive; see
videolib.py; defaults start and end), so the two starts are the sync point. The
shorter side holds its last frame until the longer one ends. Audio is RIGHT's,
cut at the same frames and padded with silence to the end. For the Manoir video
the left is original_span.py's cut (already New Game to the Brigand fade, so
the defaults) and the right is port_capture.py's port.mkv with, e.g.,
  --right-start 'scene-after:start' --right-end 'black-before:end'
--timeline prints a file's luma per frame with black runs and hard cuts marked,
to check or choose landmarks. The resolved frames go to OUT.json.

Encoding: hevc_nvenc (RTX) main10, preset p7, VBR at constant quality --cq
(default 20), or libx265 slow at --crf (default 18) for --encoder x265; auto
picks nvenc when nvidia-smi sees a GPU. Audio is Opus 192 kb/s (--audio flac
keeps it lossless); --loudnorm brings it to -16 LUFS. BT.709 limited range.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from videolib import probe, resolve, run, timeline, write_json  # noqa: E402

FONTS = ['/usr/share/fonts/noto/NotoSans-Bold.ttf', '/usr/share/fonts/TTF/DejaVuSans-Bold.ttf']


def encoder_args(name, cq, crf):
    if name == 'nvenc':
        return ['-c:v', 'hevc_nvenc', '-preset', 'p7', '-tune', 'hq', '-profile:v', 'main10', '-pix_fmt', 'p010le',
                '-rc', 'vbr', '-cq', str(cq), '-b:v', '0', '-maxrate', '60M', '-bufsize', '120M',
                '-spatial-aq', '1', '-temporal-aq', '1', '-rc-lookahead', '32', '-bf', '3', '-g', '60']
    if name == 'ffv1':  # lossless 4:4:4 reference, for checking the lossy encodes
        return ['-c:v', 'ffv1', '-level', '3', '-g', '1', '-slices', '16', '-pix_fmt', 'yuv444p10le']
    return ['-c:v', 'libx265', '-preset', 'slow', '-crf', str(crf), '-pix_fmt', 'yuv420p10le',
            '-x265-params', 'aq-mode=3:keyint=60:log-level=warning']


def pick_encoder(choice):
    if choice != 'auto':
        return choice
    try:
        subprocess.run(['nvidia-smi', '-L'], check=True, capture_output=True)
        return 'nvenc'
    except (OSError, subprocess.CalledProcessError):
        return 'x265'


def main():
    if len(sys.argv) > 1 and sys.argv[1] == '--timeline':
        timeline(sys.argv[2], float(sys.argv[3]), float(sys.argv[4]))
        return
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('left')
    parser.add_argument('right')
    parser.add_argument('out')
    for side in ('left', 'right'):
        parser.add_argument(f'--{side}-start', default='start')
        parser.add_argument(f'--{side}-end', default='end')
    parser.add_argument('--left-label', default='Original (Hype: The Time Quest)')
    parser.add_argument('--right-label', default='Godot port (Astrolabe)')
    parser.add_argument('--size', default='1024x768', help='each panel')
    parser.add_argument('--fps', type=float, default=30.0)
    parser.add_argument('--header', type=int, default=56, help='label bar height in pixels')
    parser.add_argument('--encoder', choices=['auto', 'nvenc', 'x265', 'ffv1'], default='auto')
    parser.add_argument('--cq', type=int, default=20)
    parser.add_argument('--crf', type=int, default=18)
    parser.add_argument('--audio', choices=['opus', 'flac'], default='opus')
    parser.add_argument('--loudnorm', action='store_true', help='normalise the audio to -16 LUFS (EBU R128)')
    parser.add_argument('--seconds', type=float, help='encode only the first SECONDS (tests)')
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    width, height = (int(v) for v in args.size.split('x'))
    sides = {}
    for side in ('left', 'right'):
        path = getattr(args, side)
        info = probe(path)
        first = resolve(getattr(args, f'{side}_start'), path, info)
        last = resolve(getattr(args, f'{side}_end'), path, info)
        if last < first:
            raise SystemExit(f'{side}: end frame {last} is before start frame {first}')
        sides[side] = {'path': str(path), 'fps': info['fps'], 'first': first, 'last': last,
                       'start': first / info['fps'], 'seconds': (last - first + 1) / info['fps'],
                       'audio': bool(info['audio']), 'size': [info['width'], info['height']]}
    total = max(s['seconds'] for s in sides.values())
    if args.seconds:
        total = min(total, args.seconds)
    if not sides['right']['audio']:
        print('warning: the right side has no audio; the output will be silent', file=sys.stderr)

    encoder = pick_encoder(args.encoder)
    font = next((f for f in FONTS if Path(f).exists()), None)
    labels = out.with_suffix('.labels')
    labels.mkdir(exist_ok=True)
    chains = []
    for i, side in enumerate(('left', 'right')):
        s = sides[side]
        text = labels / f'{side}.txt'
        text.write_text(getattr(args, f'{side}_label'))
        draw = (f"drawtext=textfile='{text}':reload=0:fontcolor=white:fontsize={args.header // 2}:"
                f"x=(w-text_w)/2:y=({args.header}-text_h)/2" + (f":fontfile='{font}'" if font else ''))
        chains.append(
            f'[{i}:v]trim=start_frame={s["first"]}:end_frame={s["last"] + 1},setpts=PTS-STARTPTS,'
            f'fps={args.fps:g},scale={width}:{height}:flags=lanczos,'
            f'tpad=stop_mode=clone:stop_duration={max(0.0, total - s["seconds"]) + 1:.3f},'
            f'trim=duration={total:.4f},pad={width}:{height + args.header}:0:{args.header}:black,{draw}[{side}]')
    chains.append('[left][right]hstack=inputs=2,scale=out_color_matrix=bt709:out_range=tv,'
                  + ('format=yuv444p10le[v]' if encoder == 'ffv1' else 'format=yuv420p10le[v]'))
    r = sides['right']
    if r['audio']:
        chains.append(f'[1:a]atrim=start={r["start"]:.5f}:duration={min(r["seconds"], total):.5f},'
                      f'asetpts=PTS-STARTPTS,apad=whole_dur={total:.5f}'
                      + (',loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000' if args.loudnorm else '') + '[a]')
    audio = (['-map', '[a]'] + (['-c:a', 'libopus', '-b:a', '192k'] if args.audio == 'opus' else ['-c:a', 'flac'])
             if r['audio'] else [])
    cmd = ['ffmpeg', '-v', 'error', '-stats', '-y', '-i', args.left, '-i', args.right,
           '-filter_complex', ';'.join(chains), '-map', '[v]', *audio, *encoder_args(encoder, args.cq, args.crf),
           '-colorspace', 'bt709', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-color_range', 'tv',
           '-metadata', f'title={args.left_label} | {args.right_label}', '-t', f'{total:.4f}', out]
    summary = {'encoder': encoder, 'seconds': total, 'sides': sides, 'command': [str(c) for c in cmd]}
    if args.dry_run:
        print(json.dumps(summary, indent=2))
        return
    run(cmd)
    shutil.rmtree(labels, ignore_errors=True)
    summary['output'] = probe(out)
    write_json(out.with_suffix('.json'), summary)
    print(f'{out}: {total:.2f} s, {encoder}; left frames {sides["left"]["first"]}..{sides["left"]["last"]}, '
          f'right frames {r["first"]}..{r["last"]}')


if __name__ == '__main__':
    main()
