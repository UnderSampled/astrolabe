#!/usr/bin/env python3
"""Generate the named key table for Hype save records from the semantic catalogue.

Every record gets a section and key. Records whose gameplay meaning is unresolved
go under unknown/ so a converted save never presents a guess as knowledge. The
catalogue comes from tools/save_semantics.py; see docs/openspace/save-games.md.
"""
import json
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parent.parent
catalogue = root/'output/savegame-semantic-map/semantic-catalogue.json'
table = root/'src/openspace/save_keys_manoir.tsv'
meanings_file = root/'tools/save_meanings_manoir.tsv'
tiers = {'runtime_corroborated': 'verified', 'source_verified': 'source', 'unknown': 'unknown'}


def slug(text):
    text = re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', text)
    return re.sub(r'[^a-z0-9+]+', '_', text.lower()).strip('_').replace('+', '_at_')


def section_for(owner, block_module):
    if not owner:
        return 'block_' + block_module.replace(':', '_')
    if owner.startswith(('fix:/', 'level:/')):
        return 'actor/' + owner
    for prefix, name in (('Fix inventory ', 'inventory/'), ('Hype dialogue list #', 'dialogue/'),
                         ('Hype map star entry #', 'map_star/')):
        if owner.startswith(prefix):
            return name + re.split(r' \(', owner[len(prefix):])[0].replace(' ', '_')
    return 'game/' + slug(owner)


def meaning_of(entry):
    semantic = entry.get('semantic') or {}
    text = semantic.get('meaning') or semantic.get('field') or ''
    if not text:
        partial = entry.get('partial_semantics') or {}
        text = partial.get('known_role') or partial.get('reference_role') or ''
    if not text:
        unresolved = entry.get('unresolved') or ['meaning unresolved']
        text = unresolved[0]
    return re.sub(r'\s+', ' ', str(text)).strip()


def load_meanings():
    """Hand-kept gameplay meanings from AI evidence: record id -> meaning."""
    meanings = {}
    for line in meanings_file.read_text().splitlines():
        if line and not line.startswith('#'):
            record, meaning, _evidence = line.split('\t')
            meanings[record] = meaning
    return meanings


def main():
    meanings = load_meanings()
    entries = json.loads(catalogue.read_text())['entries']
    rows, seen = [], set()
    for identity, entry in sorted(entries.items()):
        block, module, offset = identity.split(':')
        owner = entry.get('source_owner') or entry.get('owner')
        field = entry.get('source_field') or entry.get('field')
        key = slug(field) if field else f'offset_{offset}'
        tier = tiers[entry['meaning_status']]
        section = section_for(owner, f'{block}:{module}')
        text = meaning_of(entry)
        if tier == 'unknown' and identity in meanings:
            tier, text = 'source', meanings.pop(identity)
        if tier == 'unknown':
            section = 'unknown/' + section
        if (section, key) in seen:
            key += f'_{offset}'
        seen.add((section, key))
        rows.append('\t'.join((identity, section, key, tier, text)))
    if meanings:
        raise SystemExit(f'meanings name records that are not unknown: {sorted(meanings)}')
    table.write_text('\n'.join(rows)+'\n')
    print(f'{len(rows)} keys written to {table.relative_to(root)}')


if __name__ == '__main__':
    sys.exit(main())
