#!/usr/bin/env python3
"""Classify unknown actor save rows (DSG variables, CustomBitsInit) from the compiled Manoir AI.

Actor -> model comes from each generated perso scene's Brain script (plus the Global
in level.tscn). For DSG fields every own-field access (bare dsg_K) and cross-actor
access (target from the generated 'requires actor URI Brain field' message; main
actor is Hype) is collected. A field with no gameplay reader or writer gets the
existing 'declared only' / 'debug cheats only' classes. CustomBitsInit rows are
resolved only when every bit the AI can change on the actor (own ops, or Hype's ops on
its dsg_133 target) and every saved bit is one the decompilation explains (bit 0, 7).

Dialogue list/entry references get their structural role only.

  python3 tools/save_actor_audit.py            # TSV rows (id, meaning, evidence) for new ids
  python3 tools/save_actor_audit.py --summary  # class counts and what stays unknown
"""
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

root = Path(__file__).resolve().parent.parent
manoir = root/'godot/generated/hype/levels/manoir'
models = manoir/'ai/models'
table = root/'src/openspace/save_keys_manoir.tsv'
meanings = root/'tools/save_meanings_manoir.tsv'
catalogue = root/'output/savegame-semantic-map/semantic-catalogue.json'
CHEAT = '195'  # iDummy_Load_Manoir_MAD: IWorld snapshot, restore and set-all-flags
HYPE = 'level:/persos/world/0/0'
NO_REF = 'Declared but never read or written by any compiled Manoir AI, including the debug cheats'
CHEAT_ONLY = 'No gameplay reader or writer in Manoir AI: only copied by the IWorld debug cheats (snapshot, restore and set-all-flags)'
EXPLAINED = {1, 8}  # AI bit numbers (one-based) whose engine meaning is source-verified: bit 0 and bit 7


def actor_models():
    """actor URI -> model stem ('195', 'fix/28') from Brain scripts in persos/*.tscn and level.tscn."""
    out = {}
    for path in [*sorted((manoir/'persos').glob('*.tscn')), manoir/'level.tscn']:
        text = path.read_text()
        res = {i: m for m, i in re.findall(r'path="[^"]*/ai/models/((?:fix/)?\d+)\.gd" id="(\w+)"', text)}
        perso = re.search(r'metadata/source_perso = "([^"]+)"', text)
        brain = re.search(r'\[node name="Brain"[^\]]*\]\n(?:[^\[\n]*\n)*?script = ExtResource\("(\w+)"\)', text)
        if path.name == 'level.tscn':
            g = re.search(r'\[node name="Brain" type="Node3D" parent="Globals/Global"[^\]]*\]\n(?:[^\[\n]*\n)*?script = ExtResource\("(\w+)"\)', text)
            out['fix:/persos/15'] = res.get(g[1]) if g else None
        elif perso and brain and brain[1] in res:
            out[perso[1]] = res[brain[1]]
    return out


def scan(stem, owner):
    """Yield (actor, field, 'r'|'w', line) for every dsg access in a model run by `owner`."""
    lines = (models/f'{stem}.gd').read_text().splitlines()
    uri, pending = {}, None
    for line in lines:
        m = re.search(r'actor_brain_(\d+)_(?:read|target|adjust)_\d+\b', line)
        if m and 'fail' not in line:
            pending = m[1]
        m = re.search(r'requires actor (\S+) Brain field', line)
        if m and pending is not None:
            uri[pending], pending = m[1], None
    for n, line in enumerate(lines, 1):
        if line.startswith('@export var dsg_'):
            continue
        for m in re.finditer(r'(\w+)\.(get|set)\("dsg_(\d+)"', line):
            var, op, k = m.groups()
            if var.startswith('main_actor_brain'):
                target = HYPE
            elif var.startswith('wait_brain'):
                target = None
                for back in range(n - 2, max(-1, n - 40), -1):
                    d = re.search(rf'var {var}: Node = (.*)', lines[back])
                    if d:
                        a = re.search(r'actor_brain_(\d+)', d[1])
                        target = HYPE if 'main_actor' in d[1] else owner if d[1].strip() == 'self' else uri.get(a[1]) if a else None
                        break
            else:
                a = re.match(r'actor_brain_(\d+)_', var)
                target = uri.get(a[1]) if a else None
            kinds = ['r' if op == 'get' else 'w'] + (['r' if op == 'set' else 'w'] if '_adjust_' in var else [])
            for kind in kinds:
                yield target or '?', f'dsg_{k}', kind, n
        for m in re.finditer(r'(?<![\w."])dsg_(\d+)\b(\s*[-+*/|&]?=(?!=))?', line):
            yield owner, f'dsg_{m[1]}', 'w' if m[2] else 'r', n
            if m[2] and m[2].strip() != '=':
                yield owner, f'dsg_{m[1]}', 'r', n


def bit_ops(stem):
    """[(one-based bit, 'self'|'hype_target', 'test'|'change', line)] for a model's custom-bit calls."""
    lines = (models/f'{stem}.gd').read_text().splitlines()
    out = []
    for n, line in enumerate(lines, 1):
        m = re.search(r'var bits_actor_(\d+): Node3D = (character\(\)|dsg_133)', line)
        if not m:
            continue
        block = '\n'.join(lines[n:n + 14])
        p = re.search(rf'var bit_payload_{m[1]} = (\d+)', block)
        kind = 'change' if f'bits_actor_{m[1]}.source_change_custom_bit' in block else 'test'
        if p:
            out.append((int(p[1]), 'self' if m[2] == 'character()' else 'hype_target', kind, n))
        else:
            out.append((0, 'self', kind, n))  # 0 = unparsed payload, never explained
    return out


def main():
    amodel = actor_models()
    refs = defaultdict(list)
    for stem in sorted({s for s in amodel.values() if s}):
        for owner in [a for a, s in amodel.items() if s == stem]:
            for tgt, field, kind, n in scan(stem, owner):
                refs[(tgt, field)].append((stem, kind, n))
    ops = {s: bit_ops(s) for s in {s for s in amodel.values() if s}}
    # actors Hype (fix/28) can hold in dsg_133: whoever assigns itself, plus the two named by model 154
    targets = {HYPE}
    for stem in ops:
        src = (models/f'{stem}.gd').read_text()
        if re.search(r'set\("dsg_133", character\(\)\)', src):
            targets |= {a for a, s in amodel.items() if s == stem}
        targets |= set(re.findall(r'set\("dsg_133", character\(\)\.source_actor_node\("([^"]+)"\)\)', src))
    hype_bits = ops.get('fix/28', [])
    values = {k: e['file_occurrences'][0]['decoded_value'] for k, e in json.loads(catalogue.read_text())['entries'].items()}
    existing = {l.split('\t')[0] for l in meanings.read_text().splitlines() if l and not l.startswith('#')}
    rows, stats = [], defaultdict(list)
    for line in table.read_text().splitlines():
        rid, section, key, tier, _ = line.split('\t')
        if tier == 'unknown' and section.startswith('unknown/dialogue/'):
            # structural role only (docs/openspace/save-games.md, DLG_fn_vLoadDialog); target content stays unresolved
            stats['dialogue_role'].append(rid)
            if key == 'list_reference':
                text = 'Reference from this dialogue list to its own one-byte member-count field (registered by DLG_fn_vLoadDialog); structural link, no gameplay value'
            else:
                text = ('Reference to a parsed DialogAnswer_<n> answer-section object in this list item\'s ItemAnswer array, dereferenced when the '
                        'player picks an answer; which authored answer it is stays unresolved (runtime heap pointer, shared by several lists)')
            if rid not in existing:
                rows.append(f'{rid}\t{text}\t0056ba50 DLG_fn_vLoadDialog:46-58; 0056f4e0:36-90; 0056e3b0:17-63; output/savegame-semantic-map/dialogue-reference-targets-followup.md')
            continue
        if tier != 'unknown' or not section.startswith('unknown/actor/'):
            continue
        actor = section[len('unknown/actor/'):]
        stem = amodel.get(actor)
        if key.startswith('dsg_'):
            if stem is None:
                stats['dsg_no_model'].append(rid); continue
            r = refs.get((actor, key), [])
            gameplay = [x for x in r if x[0] != CHEAT]
            if not r:
                stats['dsg_never'].append(rid); text, ev = NO_REF, f'none ({stem}.gd declaration only)'
            elif not gameplay:
                stats['dsg_cheat'].append(rid); text, ev = CHEAT_ONLY, f'{CHEAT}.gd:{min(x[2] for x in r)} ({len(r)} sites)'
            elif all(x[1] == 'w' for x in gameplay):
                stats['dsg_writeonly'].append(rid)
                text = 'Written by compiled AI but read by no compiled Manoir AI except the IWorld debug cheats: no gameplay effect'
                ev = ', '.join(sorted({f'{x[0]}.gd:{x[2]}' for x in gameplay}))
            else:
                stats['dsg_gameplay'].append(rid); continue
        elif key == 'standard_game_custom_bits_init':
            value = values[rid]
            own = [o for o in ops.get(stem, []) if o[1] == 'self'] if stem else []
            hyp = [o for o in hype_bits if o[1] == 'hype_target'] if actor in targets else []
            touched = {o[0] for o in own + hyp}
            if touched <= EXPLAINED and value & ~0x81 == 0:
                stats['bits_explained'].append(rid)
                how = 'no compiled Manoir AI tests or changes any bit of this actor'
                if touched:
                    how = 'compiled AI only tests/changes bit(s) ' + ','.join(str(t - 1) for t in sorted(touched))
                text = ('CustomBitsInit flag word (StandardGame+0x2c, the Cond107/Proc273 operand): bit 0 makes the actor a camera '
                        'and nearby-character target candidate, bit 7 skips segment-world collision in one action; ' + how)
                ev = 'decompilation 00528fa0:45, 0053a900:58, 0043bcd0:94, 00439660; ' + (f'{stem}.gd bit ops' if touched else 'no bit ops in ai/models')
            else:
                stats['bits_open'].append(rid); continue
        else:
            stats['other_actor'].append(rid); continue
        if rid not in existing:
            rows.append(f'{rid}\t{text}\t{ev}')
    if '--summary' in sys.argv:
        for k, v in sorted(stats.items()):
            print(k, len(v), v[:5] if k.endswith(('gameplay', 'open', 'no_model', 'actor')) else '')
    else:
        print('\n'.join(rows))


if __name__ == '__main__':
    sys.exit(main())
