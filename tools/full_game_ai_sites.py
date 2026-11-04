#!/usr/bin/env python3
"""Static inventory of missing AI sites across converted levels.

Reads a converted project's generated GDScript (generated/hype/levels/<level>/ai)
and the per-level port logs, and counts by level and model:
  - stand-ins: stand_in("<id>", ...) sites, including generic-<cat>-<op> (calls
    with no table entry) and generic-untyped (scripts that could not be typed);
  - guards: fail_unsupported("...") sites, grouped by message template (actor
    paths and numbers replaced);
  - converter diagnostics: codes in the port log;
  - save-related sites: save procedures, Global (level-carried) Brain field
    writes, and inventory/progress calls.
Nothing is run. usage: full_game_ai_sites.py PROJECT LOG_DIR OUT_TSV
"""
import collections, json, re, sys
from pathlib import Path

project, logs, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
STAND = re.compile(r'stand_in\("([^"]+)"')
GUARD = re.compile(r'fail_unsupported\("((?:[^"\\]|\\.)*)"')
SAVE_PROCS = {"4-95": "Proc_LevelSaveRotationSector", "4-96": "Proc_PlayerSaveRotationSector",
              "4-99": "Proc_SaveGame", "4-100": "Proc_IncHistoricAndSaveGame", "4-101": "Proc_SaveAllGameValues",
              "4-125": "Proc_LevelSaveMovingSurface", "4-126": "Proc_PlayerSaveMovingSurface",
              "4-336": "Proc_ActivateSaveGameMotorMode"}
SAVE_NAMES = re.compile("|".join(SAVE_PROCS.values()))
GLOBAL_WRITE = re.compile(r'actor_brain_(\d+)\w*\.set\("(dsg_\d+)"')
EXT = re.compile(r'\[ext_resource type="Script" path="res://([^"]+)" id="([^"]+)"\]')
NODE = re.compile(r'^\[node ', re.M)

def global_brains(level_dir):
    """script path -> brain operand indices bound to the Global Brain in level.tscn."""
    scene = level_dir / "level.tscn"
    if not scene.exists(): return {}
    text = scene.read_text(errors="replace")
    scripts = {i: p for p, i in EXT.findall(text)}
    bound = collections.defaultdict(set)
    for block in NODE.split(text)[1:]:
        m = re.search(r'^script = ExtResource\("([^"]+)"\)', block, re.M)
        if not m or m.group(1) not in scripts: continue
        for k in re.findall(r'^actor_brain_(\d+)_path = NodePath\("[^"]*Globals/Global/Brain"\)', block, re.M):
            bound[scripts[m.group(1)]].add(k)
    return bound
INVENTORY = re.compile(r'source_inventory_\w+|source_magic_\w+|Inv_\w+')

def template(message):
    m = re.sub(r'(level|fix):/[\w/.\-]+', '<ref>', message)
    m = re.sub(r'\b\d+(\.\d+)?\b', 'N', m)
    return m[:140]

rows = collections.Counter()
for level_dir in sorted((project / "generated/hype/levels").iterdir()):
    level = level_dir.name
    to_global = global_brains(level_dir)
    for script in sorted((level_dir / "ai").rglob("*.gd")):
        model = str(script.relative_to(level_dir / "ai/models")).removesuffix(".gd")
        text = script.read_text(errors="replace")
        for m in STAND.finditer(text): rows[(level, model, "stand_in", m.group(1))] += 1
        for m in GUARD.finditer(text): rows[(level, model, "guard", template(m.group(1)))] += 1
        for m in SAVE_NAMES.finditer(text): rows[(level, model, "save_call", m.group(0))] += 1
        res = str(script.relative_to(project))
        for m in GLOBAL_WRITE.finditer(text):
            if m.group(1) in to_global.get(res, ()): rows[(level, model, "global_write", m.group(2))] += 1
        for m in INVENTORY.finditer(text): rows[(level, model, "inventory_call", m.group(0))] += 1
    log = logs / f"port-{level}.log"
    if log.exists():
        for code in re.findall(r'"code":"([^"]+)"', log.read_text(errors="replace")):
            rows[(level, "-", "diagnostic", code)] += 1
with out.open("w") as f:
    f.write("level\tmodel\tkind\tid\tsites\n")
    for (level, model, kind, ident), n in sorted(rows.items()):
        f.write(f"{level}\t{model}\t{kind}\t{ident}\t{n}\n")
print(len(rows), "rows")
