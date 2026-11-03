#!/usr/bin/env python3
"""Cut a span of a take's own window capture, frame-accurately, for publishing.

usage: original_span.py TAKE_DIR OUT.mkv --start LANDMARK --end LANDMARK
           [--fps 30] [--audio] [--check]

The take's video/NNNNNN.mkv segments are lossless FFV1 at 30 Hz with wall-clock
timestamps (33/34 ms steps); video/NNNNNN.frames.jsonl indexes every frame
against take elapsed time. Landmarks (see videolib.py) resolve to frame indices
of the segment holding the start; the span keeps frames START..END inclusive and
is retimed to constant FPS, still FFV1, with a JSON sidecar of what was cut.
Manoir from New Game to the start of the Brigand transition:
  --start 'scene-after:key:65293@20' --end 'black-before:epoch:brigand:5'
(the first cut after the menu's Return is the loading screen; the end is the
first black frame of the fade-out before the Brigand load).
--audio also cuts the take's audio (FLAC) at the same times, for reference.
--check prints frame-interval statistics and the take's gap notes and exits.
"""
import argparse
import bisect
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from videolib import probe, resolve, run, write_json  # noqa: E402


def load_segments(recording):
    segments = []
    for index in sorted((recording / 'video').glob('*.frames.jsonl')):
        frames = [json.loads(line) for line in index.open()]
        segments.append({'path': recording / frames[0]['path'], 'frames': frames,
                         'elapsed': [f['elapsed_micros'] / 1e6 for f in frames],
                         'pts': [f['pts_media_micros'] / 1e6 for f in frames]})
    return segments


def load_events(recording):
    return [json.loads(line) for line in (recording / 'events.jsonl').open()]


def take_time(spec, events):
    """Take elapsed seconds for elapsed:/key:/epoch: landmarks, else None."""
    kind, _, rest = spec.partition(':')
    if kind == 'elapsed':
        return float(rest)
    if kind == 'key':
        keysym, _, after = rest.partition('@')
        for e in events:
            d = e['data']
            if (e['kind'] == 'input' and d.get('edge') == 'press' and d.get('keysym') == int(keysym)
                    and e['elapsed_micros'] / 1e6 >= float(after or 0)):
                return e['elapsed_micros'] / 1e6
        raise SystemExit(f'{spec}: no such key press')
    if kind == 'epoch':
        level, _, phase = rest.partition(':')
        for e in events:
            d = e['data']
            if e['kind'] == 'epoch' and d.get('level') == level and (not phase or d.get('phase') == int(phase)):
                return e['elapsed_micros'] / 1e6
        raise SystemExit(f'{spec}: no such epoch')
    return None


def innermost(spec):
    while True:
        kind, _, rest = spec.partition(':')
        if kind in ('scene-after', 'black-after', 'black-before', 'fade-before'):
            spec = rest
        else:
            return spec


def check(segments, recording):
    manifest = json.loads((recording / 'manifest.json').read_text())
    for seg in segments:
        steps = [round((b - a) * 1000) for a, b in zip(seg['pts'], seg['pts'][1:])]
        info = probe(seg['path'])
        span = seg['pts'][-1] - seg['pts'][0]
        print(f"{seg['path'].name}: {info['width']}x{info['height']} {info['fps']:g} fps nominal, "
              f"{len(seg['pts'])} frames over {span:.3f} s = {(len(seg['pts']) - 1) / span:.3f} fps, "
              f"take {seg['elapsed'][0]:.3f}..{seg['elapsed'][-1]:.3f} s, "
              f"steps {min(steps)}..{max(steps)} ms, >50 ms: {sum(s > 50 for s in steps)}, "
              f"index contiguous: {[f['index'] for f in seg['frames']] == list(range(len(seg['frames'])))}")
    for gap in manifest['coverage']['gaps']:
        print('note:', gap)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('take')
    parser.add_argument('out', nargs='?')
    parser.add_argument('--start')
    parser.add_argument('--end')
    parser.add_argument('--fps', type=float, default=30.0)
    parser.add_argument('--audio', action='store_true')
    parser.add_argument('--threads', type=int, default=4, help='ffmpeg threads (keep low beside a bot run)')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    recording = Path(args.take) / 'recording'
    segments = load_segments(recording)
    if args.check:
        check(segments, recording)
        return
    if not (args.out and args.start and args.end):
        parser.error('OUT, --start and --end are required')
    events = load_events(recording)

    # The segment holding the start's innermost take time (or the last segment).
    anchor = take_time(innermost(args.start), events)
    seg = segments[-1]
    if anchor is not None:
        seg = next(s for s in reversed(segments) if s['elapsed'][0] <= anchor)
    info = probe(seg['path'])
    pts, elapsed = seg['pts'], seg['elapsed']

    def index(t):
        i = bisect.bisect_left(pts, t - 0.0005)
        return min(i, len(pts) - 1)

    def base(spec):
        t = take_time(spec, events)
        if t is None:
            return None
        i = bisect.bisect_left(elapsed, t)
        if i >= len(elapsed):
            raise SystemExit(f'{spec}: take time {t:.3f} s is after {seg["path"].name} ends')
        return i

    def time_of(n):
        return pts[n]

    first = resolve(args.start, seg['path'], info, base, index, time_of)
    last = resolve(args.end, seg['path'], info, base, index, time_of)
    if last < first:
        raise SystemExit(f'end frame {last} is before start frame {first}')
    count = last - first + 1
    # FFV1 is intra-only: seek a little early, then trim by exact frame number.
    seek = max(0.0, pts[first] - 2.0)
    skip = first - index(seek)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    run(['nice', '-n', '10', 'ffmpeg', '-v', 'error', '-stats', '-y', '-threads', str(args.threads), '-ss', f'{seek:.3f}', '-i', seg['path'],
         '-map', '0:v:0', '-vf', f'trim=start_frame={skip}:end_frame={skip + count},setpts=N/({args.fps}*TB)',
         '-r', f'{args.fps:g}', '-c:v', 'ffv1', '-level', '3', '-g', '1', '-slices', '16', '-threads', str(args.threads), '-pix_fmt', 'bgr0',
         '-metadata', f'comment=frames {first}..{last} of {seg["path"].name}', out])
    sidecar = {'source': str(seg['path']), 'start': args.start, 'end': args.end, 'first_frame': first,
               'last_frame': last, 'frames': count, 'fps': args.fps,
               'take_elapsed': [elapsed[first], elapsed[last]], 'source_pts': [pts[first], pts[last]],
               'wall_seconds': elapsed[last] - elapsed[first], 'video_seconds': count / args.fps}
    if args.audio:
        # The audio capture starts at launch + capture_start_from_launch_micros.
        stream_end = next(e for e in events if e['kind'] == 'audio_stream_end')['data']
        offset = stream_end['capture_start_from_launch_micros'] / 1e6
        a0 = elapsed[first] - offset
        audio = out.with_suffix('.audio.mka')
        run(['ffmpeg', '-v', 'error', '-y', '-ss', f'{a0:.4f}', '-t', f'{count / args.fps:.4f}',
             '-i', recording / 'audio.mka', '-c:a', 'flac', audio])
        sidecar['audio'] = {'path': str(audio), 'take_start': a0, 'timing': stream_end['timing_uncertainty']}
    write_json(out.with_suffix('.json'), sidecar)
    print(f'frames {first}..{last} ({count}, {count / args.fps:.3f} s at {args.fps:g} fps; '
          f'take {elapsed[first]:.3f}..{elapsed[last]:.3f} s) -> {out}')


if __name__ == '__main__':
    main()
