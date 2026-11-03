#!/usr/bin/env python3
"""Compare a port replay of a recorded take with the take, in route order.

  tools/take_compare.py SOURCE_RECORDING REPLAY_DIR
Both clocks are rebased on the replay's anchors, so each segment is read from its
own start. Prints distance over time and every change of Hype's normal behavior
in both engines. Fine timing means little in a take the sampler kept pausing.
"""
import bisect
import json
from pathlib import Path
import sys


def load(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def source_rows(path):
    rows = []
    for event in load(path):
        if event['kind'] == 'actor':
            transform = event['data']['actor']['global_transform']
            transform = transform.get('Affine', transform)
            rows.append((event['elapsed_micros'], transform['translation'], event['data']['normal_behavior']['current'], event['data']['actor']['current_action']))
    return rows


def main():
    source = source_rows(Path(sys.argv[1])/'events.jsonl')
    replay = load(Path(sys.argv[2])/'events.jsonl')
    anchors = sorted((e['data']['source_micros'], e['data']['port_micros'], e['data']['anchor'], e['data'].get('level', ''), e['data'].get('reached', True))
                     for e in replay if e['kind'] == 'replay_anchor' and e['data']['anchor'] != 'menu_ready')
    print('anchors', [(round(a[0]/1e6, 1), round(a[1]/1e6, 1), a[2], a[3], a[4]) for a in anchors])
    port = []
    for event in replay:
        if event['kind'] == 'state':
            data = event['data'].get('state', event['data'])
            actor = data.get('actor')
            if actor:
                port.append((event.get('port_micros', event['elapsed_micros']), actor['translation'], (data.get('brain') or {}).get('normal_behavior'), actor['current_state'], data.get('level')))
    times = [row[0] for row in port]
    keys = [a[0] for a in anchors]
    last = None
    worst = 0
    for at, position, normal, action in source:
        index = bisect.bisect_right(keys, at)-1
        if index < 0:
            continue
        mapped = anchors[index][1]+at-anchors[index][0]
        near = min(bisect.bisect_left(times, mapped), len(port)-1)
        row = port[near]
        gap = sum((a-b)**2 for a, b in zip(position, row[1]))**.5
        worst = max(worst, gap)
        if last is None or at-last >= 10e6:
            print(f'{at/1e6:6.0f} gap {gap:6.1f} src {[round(v,1) for v in position]} n{normal} a{action} port {[round(v,1) for v in row[1]]} n{row[2]} a{row[3]} {row[4]}')
            last = at
    print('worst gap', round(worst, 1))


if __name__ == '__main__':
    sys.exit(main())
