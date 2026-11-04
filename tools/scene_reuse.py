#!/usr/bin/env python3
"""Read-only Godot scene structure/reuse report for a generated level.

Reports level-owned nodes, PackedScene instances, overrides written inside
instances, `editable` flags, and groups the level's actor scenes by their
non-root content. Only established instance-specific fields are canonicalized
(the root node itself, `unique_id`/`uid`, and the per-actor source identity
metadata listed in INSTANCE_FIELDS); every other difference is reported.
"""
import argparse
import hashlib
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

HEADER = re.compile(r'^\[(\w+)(.*)\]$')
ATTR = re.compile(r'(\w+)=("(?:[^"\\]|\\.)*"|\w+\([^)]*\)|[^\s\]]+)')
REF = re.compile(r'(Ext|Sub)Resource\("([^"]+)"\)')
# Child fields that only restate the owning actor's source identity.
INSTANCE_FIELDS = {"metadata/source_collision_perso"}


def sections(text):
    current = None
    for line in text.splitlines():
        match = HEADER.match(line) if line.startswith("[") else None
        if match and (current is None or not current["open"]):
            if current:
                yield current
            attrs = {k: v.strip('"') for k, v in ATTR.findall(match.group(2))}
            current = {"kind": match.group(1), "attrs": attrs, "body": [], "open": False}
        elif current is not None and line.strip():
            current["body"].append(line)
            # Track open multi-line string/array values crudely: odd quotes.
            if line.count('"') % 2:
                current["open"] = not current["open"]
    if current:
        yield current


def parse(path):
    secs = list(sections(Path(path).read_text(encoding="utf-8")))
    ext = {s["attrs"]["id"]: s["attrs"].get("path", "") for s in secs if s["kind"] == "ext_resource"}
    sub_text = {s["attrs"]["id"]: s for s in secs if s["kind"] == "sub_resource"}
    cache = {}

    def canon(value):
        def repl(m):
            if m.group(1) == "Ext":
                return f'Ext({ext.get(m.group(2), m.group(2))})'
            return f'Sub({sub_hash(m.group(2))})'
        return REF.sub(repl, value)

    def sub_hash(ident):
        if ident not in cache:
            cache[ident] = "?"
            s = sub_text.get(ident)
            body = s["attrs"].get("type", "") + "\n" + "\n".join(canon(l) for l in s["body"]) if s else ident
            cache[ident] = hashlib.sha1(body.encode()).hexdigest()[:12]
        return cache[ident]

    nodes = []
    for s in secs:
        if s["kind"] != "node":
            continue
        a = s["attrs"]
        props = {}
        for line in s["body"]:
            key, _, value = line.partition(" = ")
            if value:
                props[key] = canon(value)
            elif props:
                last = next(reversed(props))
                props[last] += "\n" + canon(line)
        parent = a.get("parent")
        path = "." if parent is None else (a["name"] if parent == "." else f'{parent}/{a["name"]}')
        nodes.append({
            "path": path, "parent": parent, "type": a.get("type"),
            "instance": ext.get(REF.match(a["instance"]).group(2)) if "instance" in a else None,
            "props": props,
        })
    editable = [s["attrs"].get("path") for s in secs if s["kind"] == "editable"]
    return {"nodes": nodes, "editable": editable, "ext": ext, "subs": len(sub_text)}


def level_report(scene):
    nodes = scene["nodes"]
    instances = {n["path"]: n["instance"] for n in nodes if n["instance"]}
    inside = [n for n in nodes if n["parent"] and any(
        n["parent"] == p or n["parent"].startswith(p + "/") for p in instances)]
    refs = defaultdict(int)
    for target in instances.values():
        refs[target] += 1
    return {
        "nodes": len(nodes),
        "nodes_inside_instances": len(inside),
        "instances": len(instances),
        "distinct_instanced_scenes": len(refs),
        "editable_flags": len(scene["editable"]),
        "sub_resources": scene["subs"],
        "instanced_scenes": dict(refs),
    }


def template_key(scene, structure_only=False):
    parts = []
    for n in scene["nodes"]:
        if n["parent"] is None:
            parts.append(f'ROOT type={n["type"]}')
            continue
        props = {} if structure_only else {k: v for k, v in n["props"].items() if k not in INSTANCE_FIELDS}
        parts.append(json.dumps([n["path"], n["type"], n["instance"], props], sort_keys=True))
    parts.extend(f"editable {p}" for p in sorted(scene["editable"]))
    return hashlib.sha1("\n".join(parts).encode()).hexdigest()[:12]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("level_dir", type=Path)
    ap.add_argument("--actors", nargs="+", default=["persos", "actors"], help="actor scene subdirectories")
    ap.add_argument("--match", default="", help="only list groups containing this name")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    level = parse(args.level_dir / "level.tscn")
    report = {"level": level_report(level)}
    instanced = set(report["level"]["instanced_scenes"])
    actor_scenes = sorted(p for d in args.actors for p in (args.level_dir / d).glob("*.tscn"))
    groups = defaultdict(list)
    parsed = {}
    for path in actor_scenes:
        parsed[path] = parse(path)
        groups[template_key(parsed[path], structure_only=True)].append(path)
    group_rows = []
    for key, members in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        names = [m.stem for m in members]
        if args.match and not any(args.match in n for n in names):
            continue
        varying = set()
        if len(members) > 1:
            first = {n["path"]: n["props"] for n in parsed[members[0]]["nodes"] if n["parent"]}
            for m in members[1:]:
                for n in parsed[m]["nodes"]:
                    if n["parent"] and first.get(n["path"]) != n["props"]:
                        for k in set(first.get(n["path"], {})) | set(n["props"]):
                            if first.get(n["path"], {}).get(k) != n["props"].get(k):
                                varying.add(f'{n["path"]}:{k}')
        group_rows.append({"key": key, "count": len(members),
                           "content_variants": len({template_key(parsed[m]) for m in members}), "nodes_each": len(parsed[members[0]]["nodes"]),
                           "members": names, "canonicalized_child_fields": sorted(varying)})
    total_nodes = sum(len(p["nodes"]) for p in parsed.values())
    report["actors"] = {
        "scenes": len(actor_scenes),
        "structure_groups": len(groups),
        "content_groups": len({template_key(p) for p in parsed.values()}),
        "total_nodes": total_nodes,
        "bytes": sum(p.stat().st_size for p in actor_scenes),

    }
    report["groups"] = group_rows
    if args.json:
        json.dump(report, sys.stdout, indent=1)
        print()
        return
    print(json.dumps({k: v for k, v in report["level"].items() if k != "instanced_scenes"}))
    print(json.dumps(report["actors"]))
    for g in group_rows:
        if g["count"] > 1 or args.match:
            print(f'{g["key"]} x{g["count"]} variants={g["content_variants"]} nodes={g["nodes_each"]} {",".join(g["members"][:6])}'
                  + (f' child-diffs={g["canonicalized_child_fields"]}' if g["canonicalized_child_fields"] else ""))


if __name__ == "__main__":
    main()
