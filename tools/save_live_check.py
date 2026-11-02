#!/usr/bin/env python3
"""Compare every saved record with live memory after the original loads that slot.

Needs a disposable prefix copy (the gate's recipe) and a running private Orrery
MCP server. Writes a per-tier summary; a match shows where the bytes live, not
what they mean, so it strengthens the key table rather than replacing it.
"""
import argparse
import collections
import json
from pathlib import Path
import sys
import urllib.request

root = Path(__file__).resolve().parent.parent


class Mcp:
    def __init__(self, port, token):
        self.url, self.token, self.session, self.id = f'http://127.0.0.1:{port}/mcp', token, None, 0
        self.post('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {},
                                 'clientInfo': {'name': 'save-live-check', 'version': '1'}})
        self.post('notifications/initialized', note=True)

    def post(self, method, params=None, note=False):
        self.id += 1
        body = {'jsonrpc': '2.0', 'method': method}
        if params is not None:
            body['params'] = params
        if not note:
            body['id'] = self.id
        headers = {'Authorization': 'Bearer '+self.token, 'Content-Type': 'application/json',
                   'Accept': 'application/json, text/event-stream'}
        if self.session:
            headers['Mcp-Session-Id'] = self.session
        reply = urllib.request.urlopen(urllib.request.Request(self.url, json.dumps(body).encode(), headers), timeout=180)
        self.session = reply.headers.get('Mcp-Session-Id') or self.session
        text = reply.read().decode()
        if note or not text.strip():
            return None
        for line in text.splitlines():
            if line.startswith('data:') and line[5:].strip().startswith('{'):
                return json.loads(line[5:])
        return json.loads(text)

    def call(self, name, **arguments):
        result = self.post('tools/call', {'name': name, 'arguments': arguments})['result']
        text = ''.join(part.get('text', '') for part in result.get('content', []))
        if result.get('isError'):
            raise RuntimeError(text[:300])
        try:
            return json.loads(text)
        except ValueError:
            return text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--token-file', type=Path, required=True)
    parser.add_argument('--registry', type=Path, required=True, help='save_registry.json from orrery inspect-original')
    parser.add_argument('--catalogue', type=Path, default=root/'output/savegame-semantic-map/semantic-catalogue.json')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    bases = {}
    for node in json.loads(args.registry.read_text())['nodes']:
        source = node.get('mapped_source')
        if source:
            bases[(source['block'], source['module'])] = source['block_base']
    keys = {}
    for line in (root/'src/openspace/save_keys_manoir.tsv').read_text().splitlines():
        identity, section, key, tier = line.split('\t')[:4]
        keys[identity] = (section, key, tier)
    entries = json.loads(args.catalogue.read_text())['entries']
    mcp = Mcp(args.port, args.token_file.read_text().strip())
    mcp.call('start_game', duration_seconds=900)
    mcp.call('run_sequence', steps=[{'action': 'keys', 'keys': ['Down', 'Return', 'Return'], 'hold_ms': 55, 'gap_ms': 800},
                                    {'action': 'wait_state', 'phase': 9, 'level': 'manoir', 'timeout_ms': 25000},
                                    {'action': 'wait', 'duration_ms': 1500}])
    counts, mismatches = collections.Counter(), []
    for identity, entry in sorted(entries.items()):
        block, module, offset = (int(part) for part in identity.split(':'))
        tier = keys[identity][2]
        saved = next((row['value_hex'] for row in entry['file_occurrences'] if 'Level000' in row['file']), None)
        if saved is None or (block, module) not in bases:
            counts[(tier, 'no_live_address')] += 1
            continue
        reply = mcp.call('read_memory', address=hex(bases[(block, module)]+offset), size=len(saved)//2, type='hex')
        live = reply if isinstance(reply, str) else reply.get('value') or reply.get('hex') or reply.get('data') or ''
        live = ''.join(c for c in str(live).lower() if c in '0123456789abcdef')
        same = live == saved
        counts[(tier, 'match' if same else 'differs')] += 1
        if not same:
            mismatches.append({'identity': identity, 'section': keys[identity][0], 'key': keys[identity][1],
                               'tier': tier, 'saved': saved[:64], 'live': live[:64]})
    mcp.call('stop_game')
    summary = {f'{tier}/{state}': count for (tier, state), count in sorted(counts.items())}
    args.output.write_text(json.dumps({'summary': summary, 'mismatches': mismatches}, indent=1)+'\n')
    print(json.dumps(summary, indent=1))
    print(f'{len(mismatches)} mismatches, see {args.output}')


if __name__ == '__main__':
    sys.exit(main())
