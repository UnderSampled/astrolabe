"""Shared helpers for the side-by-side video tools: probing, luma timelines and
cut-point landmarks.

A landmark names one frame of a constant-frame-rate video. Grammar:
  t:SECONDS          frame at that media time
  frame:N            frame N
  start / end        first / last frame
  scene-after:LM     first hard cut (mean-luma jump) after LM
  black-after:LM     first frame of the first black run after LM
  black-before:LM    first frame of the last black run starting before LM
  fade-before:LM     first frame of the fade-out that ends in that black run
Any landmark takes a +S or -S seconds suffix, e.g. black-before:end-0.5.
Take recordings add (original_span.py only; times are take elapsed seconds):
  elapsed:SECONDS    first captured frame at or after that take time
  key:KEYSYM@AFTER   first press of an X11 keysym after take time AFTER
  epoch:LEVEL:PHASE  first take epoch event entering that level and phase
"""
import json
import re
import subprocess
from pathlib import Path

BLACK_Y = 20.0      # mean 8-bit luma (limited range, black = 16) counted as black
SCENE_JUMP = 40.0   # mean-luma step between adjacent frames counted as a hard cut
WINDOW = 30.0       # seconds of video a search decodes


def probe(path):
    out = subprocess.run(['ffprobe', '-v', 'error', '-show_entries',
                          'format=duration:stream=codec_type,codec_name,width,height,r_frame_rate,sample_rate,channels',
                          '-of', 'json', str(path)], check=True, capture_output=True, text=True).stdout
    info = json.loads(out)
    video = next(s for s in info['streams'] if s['codec_type'] == 'video')
    num, den = video['r_frame_rate'].split('/')
    return {'duration': float(info['format']['duration']), 'fps': float(num) / float(den),
            'width': video['width'], 'height': video['height'],
            'audio': [s for s in info['streams'] if s['codec_type'] == 'audio']}


def luma(path, start, seconds, fps, index=None):
    """[(frame_index, mean_luma)] for frames in [start, start + seconds). Frame N
    sits at N / fps unless `index(pts_seconds)` maps times to frame indices."""
    cmd = ['nice', '-n', '19', 'ffmpeg', '-v', 'error', '-threads', '2', '-filter_threads', '2', '-copyts', '-ss', f'{max(start, 0.0):.3f}',
           '-t', f'{seconds:.3f}', '-i', str(path), '-an', '-vf',
           'scale=256:-2,signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-', '-f', 'null', '-']
    out = subprocess.run(cmd, check=True, capture_output=True, text=True).stdout
    rows, t = [], None
    for line in out.splitlines():
        m = re.search(r'pts_time:([-\d.]+)', line)
        if m:
            t = float(m.group(1))
        m = re.search(r'YAVG=([\d.]+)', line)
        if m and t is not None:
            rows.append((index(t) if index else round(t * fps), float(m.group(1))))
    return rows


def _black_runs(rows, min_frames=2):
    runs, begin = [], None
    for i, (_, y) in enumerate(rows):
        if y <= BLACK_Y and begin is None:
            begin = i
        if (y > BLACK_Y or i == len(rows) - 1) and begin is not None:
            end = i if y > BLACK_Y else i + 1
            if end - begin >= min_frames:
                runs.append((begin, end))
            begin = None
    return runs


def resolve(spec, path, info=None, base=None, index=None, time_of=None):
    """Frame index for a landmark. `base(spec)` may resolve extra prefixes (take
    landmarks) to a frame index or return None; `index`/`time_of` map between
    media seconds and frame indices on a variable-timestamp source."""
    info = info or probe(path)
    fps = info['fps']
    time_of = time_of or (lambda n: n / fps)
    last = int(round(info['duration'] * fps)) - 1
    m = re.match(r'^(.*?)([+-]\d+(?:\.\d+)?)$', spec)
    if m and (':' in m.group(1) or m.group(1) in ('start', 'end')):
        return max(0, min(last, resolve(m.group(1), path, info, base, index, time_of) + round(float(m.group(2)) * fps)))
    if spec == 'start':
        return 0
    if spec == 'end':
        return last
    kind, _, rest = spec.partition(':')
    if kind == 't':
        return round(float(rest) * fps)
    if kind == 'frame':
        return int(rest)
    if kind in ('scene-after', 'black-after', 'black-before', 'fade-before'):
        anchor = resolve(rest, path, info, base, index, time_of)
        at = time_of(anchor)
        if kind in ('scene-after', 'black-after'):
            rows = luma(path, at, WINDOW, fps, index)
        else:
            rows = luma(path, at - WINDOW, WINDOW + 1.0 / fps, fps, index)
        if not rows:
            raise SystemExit(f'{spec}: no frames decoded near {at:.2f} s in {path}')
        if kind == 'scene-after':
            for (_, a), (n, b) in zip(rows, rows[1:]):
                if n > anchor and abs(b - a) >= SCENE_JUMP:
                    return n
            raise SystemExit(f'{spec}: no hard cut within {WINDOW:.0f} s after {at:.2f} s')
        runs = _black_runs(rows)
        if kind == 'black-after':
            runs = [r for r in runs if rows[r[0]][0] > anchor]
            if not runs:
                raise SystemExit(f'{spec}: no black run within {WINDOW:.0f} s after {at:.2f} s')
            return rows[runs[0][0]][0]
        runs = [r for r in runs if rows[r[0]][0] <= anchor]
        if not runs:
            raise SystemExit(f'{spec}: no black run within {WINDOW:.0f} s before {at:.2f} s')
        begin = runs[-1][0]
        if kind == 'black-before':
            return rows[begin][0]
        # Walk back over the fade: luma keeps falling into the black run.
        i = begin
        while i > 0 and rows[i - 1][1] > rows[i][1] + 0.05:
            i -= 1
        return rows[i][0]
    if base is not None:
        found = base(spec)
        if found is not None:
            return found
    raise SystemExit(f'unknown landmark {spec!r}')


def timeline(path, start, seconds):
    """Print the luma timeline with black runs and hard cuts, for picking cuts."""
    info = probe(path)
    rows = luma(path, start, seconds, info['fps'])
    prev = None
    for n, y in rows:
        mark = ''
        if prev is not None and abs(y - prev) >= SCENE_JUMP:
            mark = ' cut'
        elif y <= BLACK_Y:
            mark = ' black'
        print(f'{n:7d} {n / info["fps"]:9.3f} s  Y={y:6.1f}{mark}')
        prev = y


def run(cmd, log=None):
    print('+', ' '.join(str(c) for c in cmd), flush=True)
    if log:
        with open(log, 'w') as handle:
            subprocess.run([str(c) for c in cmd], check=True, stdout=handle, stderr=subprocess.STDOUT)
    else:
        subprocess.run([str(c) for c in cmd], check=True)


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n')
