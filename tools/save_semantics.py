#!/usr/bin/env python3
"""Build a reproducible Hype save corpus and registration-call index.

This is research metadata. It consumes Orrery's maintained save inspection
instead of decoding save bytes independently, and never writes game data.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
from collections import Counter, defaultdict
from pathlib import Path


FIXTURE_SHA256 = "877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c"
EXE_SHA256 = "048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da"
PROFILE = "hype-pc-english-supplied-save"
WRAPPERS = {
    "FUN_004c6240": (1, "0x01"),
    "FUN_004c6260": (2, "0x02"),
    "FUN_004c6280": (4, "0x04"),
    "FUN_004c62a0": (4, "0x04"),
    "FUN_004c62e0": (8, "0x11"),
    "FUN_004c6400": (None, "0x10"),
    "FUN_004c6300": (4, "0x04"),
    "FUN_004c6320": (12, "0x10"),
    "FUN_004c6340": (36, "0x10"),
}
WRAPPER_CHAINS = {
    "FUN_004c6300": "FUN_004c62a0",
    "FUN_004c6320": "FUN_004c6400(size=0x0c)",
    "FUN_004c6340": "FUN_004c6400(size=0x24)",
}
TARGETS = (*WRAPPERS, "FUN_004c68e0")
CALL_RE = re.compile(r"\b(" + "|".join(TARGETS) + r")\s*\(")
DECLARATION_RE = re.compile(r"^\s*(?:void|bool|char|int|uint|undefined\d*)\s+FUN_")


def digest(path: Path) -> str:
    hash_value = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hash_value.update(chunk)
    return hash_value.hexdigest()


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def call_arguments(source: str, open_paren: int) -> list[str] | None:
    depth = 1
    part_start = open_paren + 1
    arguments = []
    for offset in range(part_start, len(source)):
        character = source[offset]
        if character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                arguments.append(source[part_start:offset].strip())
                return arguments
        elif character == "," and depth == 1:
            arguments.append(source[part_start:offset].strip())
            part_start = offset + 1
    return None


def registration_index(root: Path) -> dict:
    function_root = root / "reference/decompilation/functions"
    entries = []
    function_files = sorted(function_root.glob("*.c"))
    for path in function_files:
        source = path.read_text(errors="replace")
        lines = source.splitlines()
        for match in CALL_RE.finditer(source):
            line = source.count("\n", 0, match.start()) + 1
            if DECLARATION_RE.match(lines[line - 1]):
                continue
            arguments = call_arguments(source, match.end() - 1)
            if arguments is None:
                continue
            target = match.group(1)
            if target == "FUN_004c68e0":
                address = arguments[1] if len(arguments) >= 2 else None
                size = arguments[2] if len(arguments) >= 3 else None
                wire_kind = arguments[3] if len(arguments) >= 4 else None
                flags = arguments[4] if len(arguments) >= 5 else None
            else:
                fixed_size, wire_kind = WRAPPERS[target]
                address = arguments[0] if arguments else None
                size = str(fixed_size) if fixed_size is not None else (
                    arguments[1] if len(arguments) >= 2 else None
                )
                flags = arguments[-1] if len(arguments) >= 2 else None
            nearby = lines[max(0, line - 16) : line - 1]
            control = [
                {"line": line - len(nearby) + index, "text": text.strip()}
                for index, text in enumerate(nearby)
                if re.search(r"\b(?:for|while|if)\s*\(", text)
            ]
            entries.append(
                {
                    "caller": path.stem,
                    "source": str(path.relative_to(root)),
                    "line": line,
                    "target": target,
                    "wrapper_chain": WRAPPER_CHAINS.get(target),
                    "address_expression": address,
                    "size_expression": size,
                    "wire_kind": wire_kind,
                    "flags_expression": flags,
                    "arguments": arguments,
                    "nearby_control_hints": control[-4:],
                    "control_hints_are_proof": False,
                    "potential_owner": path.stem.split("_", 1)[-1],
                    "address_requires_runtime_resolution": not bool(
                        address and re.fullmatch(r"&?DAT_[0-9a-fA-F]+", address)
                    ),
                }
            )
    counts = Counter(entry["target"] for entry in entries)
    return {
        "schema": 1,
        "scope": "literal calls to nine known wrappers and central registration in pinned decompilation",
        "limits": [
            "This literal-call index does not establish absence of indirect registration calls.",
            "Nearby branch/loop lines are search hints, not verified control-flow ownership.",
            "Runtime address expressions are not portable save record identities.",
        ],
        "function_file_count": len(function_files),
        "call_count": len(entries),
        "delegating_wrapper_calls": sum(
            entry["caller"].split("_", 1)[-1] in WRAPPERS
            for entry in entries
        ),
        "calls_by_target": dict(sorted(counts.items())),
        "calls": entries,
    }


def corpus_manifest(root: Path, config_path: Path, inspection: dict) -> dict:
    fixture = root / "reference/Savefile_EN.zip"
    if digest(fixture) != FIXTURE_SHA256:
        raise ValueError("supplied save fixture hash differs")
    config = json.loads(config_path.read_text())
    executable = Path(config["exe"])
    if digest(executable) != EXE_SHA256 or config["exe_sha256"] != EXE_SHA256:
        raise ValueError("pinned executable hash differs")
    level_root = root / "disc/Gamedata/World/Levels"
    packages = []
    for package in sorted([level_root, *[p for p in level_root.iterdir() if p.is_dir()]]):
        sna_files = sorted(package.glob("*.sna"))
        for sna in sna_files:
            sda = sna.with_suffix(".sda")
            packages.append(
                {
                    "name": sna.stem,
                    "role": "fix" if sna.name.lower() == "fix.sna" else "level",
                    "sna": str(sna.relative_to(root)),
                    "sna_sha256": digest(sna),
                    "sda": str(sda.relative_to(root)) if sda.is_file() else None,
                    "sda_sha256": digest(sda) if sda.is_file() else None,
                }
            )
    captured = []
    evidence_root = root / "output/savegame-investigation"
    if evidence_root.is_dir():
        for directory in sorted(evidence_root.rglob("written-saves")):
            if not directory.is_dir():
                continue
            members = sorted(directory.rglob("*.sav"))
            if members:
                captured.append(
                    {
                        "root": str(directory.relative_to(root)),
                        "members": [
                            {
                                "path": str(path.relative_to(directory)),
                                "sha256": digest(path),
                                "size": path.stat().st_size,
                            }
                            for path in members
                        ],
                    }
                )
    return {
        "schema": 1,
        "profile": PROFILE,
        "fixture": {
            "path": str(fixture.relative_to(root)),
            "sha256": FIXTURE_SHA256,
            "members": {
                path: {"sha256": value["sha256"], "encoded_size": value["encoded_size"]}
                for path, value in sorted(inspection["files"].items())
            },
        },
        "executable": {"sha256": EXE_SHA256, "config": str(config_path)},
        "source_packages": packages,
        "available_level_count": sum(package["role"] == "level" for package in packages),
        "captured_game_written_save_roots": captured,
        "decompilation_function_root": "reference/decompilation/functions",
    }


def semantic_catalogue(inspection: dict, allocation_joins: dict | None) -> tuple[dict, dict]:
    if inspection.get("schema") != 2 or inspection.get("semantic_profile") != PROFILE:
        raise ValueError("expected a schema-2 Orrery inspection with the pinned English profile")
    records = defaultdict(list)
    non_records = {}
    for path, file in sorted(inspection["files"].items()):
        if "records" not in file:
            non_records[path] = {
                "sha256": file["sha256"],
                "decoded_size": file.get("decoded_size"),
                "text_lines": file.get("text_lines"),
                "u32_values": file.get("u32_values"),
                "status": "structurally_decoded_semantics_incomplete",
            }
            continue
        if file.get("undecoded_tail") is not None:
            raise ValueError(f"fixture has an undecoded record tail: {path}")
        for identity, record in file["records"].items():
            records[identity].append(
                {
                    "file": path,
                    "kind": record["kind"],
                    "flags": record["flags"],
                    "reserved": record["reserved"],
                    "value_hex": record["value_hex"],
                    "decoded_value": record.get("value"),
                    "value_size": len(record["value_hex"]) // 2,
                    "semantic": record.get("semantic"),
                }
            )
    entries = {}
    families = defaultdict(lambda: {"identities": 0, "occurrences": 0, "explained": 0, "samples": []})
    for identity, occurrences in sorted(records.items()):
        kinds = {occurrence["kind"] for occurrence in occurrences}
        flags = {occurrence["flags"] for occurrence in occurrences}
        semantics = [item["semantic"] for item in occurrences if item["semantic"]]
        semantic = semantics[0] if semantics and all(item == semantics[0] for item in semantics) else None
        status = "runtime_corroborated" if semantic else "unknown"
        block, module, _ = identity.split(":")
        family = f"{block}:{module}/{','.join(sorted(kinds))}/{','.join(sorted(flags))}"
        family_info = families[family]
        family_info["identities"] += 1
        family_info["occurrences"] += len(occurrences)
        family_info["explained"] += bool(semantic)
        if len(family_info["samples"]) < 8:
            family_info["samples"].append(identity)
        entries[identity] = {
            "profile": PROFILE,
            "context": "supplied English SaveGame0, manoir",
            "record_identity": identity,
            "file_occurrences": occurrences,
            "family": family,
            "owner": semantic.get("owner") if semantic else None,
            "field": semantic.get("field") if semantic else None,
            "semantic": semantic,
            "meaning_status": status,
            "persistence": {
                "player_file": all(int(flag, 16) & 0x0300 != 0 for flag in flags),
                "named_level_file": all(int(flag, 16) & 0x0c00 != 0 for flag in flags),
                "value_source": "current memory" if all(
                    int(flag, 16) & 0x0a00 != 0 for flag in flags
                ) else "registration snapshot or mixed; inspect flags",
                "load_apply": "player mode 4; named-level mode 8 when selected; later overwrites unverified",
            },
            "unresolved": [] if semantic else ["source owner, field meaning and restoration behavior not yet joined"],
        }
    allocation_statuses = Counter()
    if allocation_joins is not None:
        if allocation_joins.get("schema") != 1 or allocation_joins.get("profile") != PROFILE:
            raise ValueError("source allocation joins have the wrong schema or profile")
        joined = {item["id"]: item for item in allocation_joins["joins"]}
        if set(joined) != set(entries):
            raise ValueError("source allocation joins do not cover the exact catalogue identities")
        for identity, entry in entries.items():
            join = joined[identity]
            allocation_statuses[join["status"]] += 1
            entry["source_allocation_join"] = join
            if join["status"] != "single_bounded_allocation":
                entry["ownership_status"] = join["status"]
                continue
            candidate = join["candidates"][0]
            if candidate.get("source_object") and candidate.get("source_field"):
                entry["ownership_status"] = "source_object_and_field"
                entry["source_owner"] = candidate["source_object"]
                if candidate.get("source_appearance"):
                    entry["source_appearance"] = candidate["source_appearance"]
                entry["source_field"] = candidate["source_field"]
                entry["source_type"] = candidate.get("source_type")
                entry["source_rule"] = candidate.get("source_rule")
                if entry["meaning_status"] == "unknown" and candidate.get("source_meaning"):
                    entry["owner"] = candidate["source_object"]
                    entry["field"] = candidate["source_field"]
                    entry["semantic"] = {
                        "owner": candidate["source_object"],
                        "field": candidate["source_field"],
                        "meaning": candidate["source_meaning"],
                        "source_rule": candidate["source_rule"],
                        "confidence": "source_verified",
                    }
                    entry["meaning_status"] = "source_verified"
                    entry["unresolved"] = ["runtime behavior not separately corroborated for this item"]
            else:
                entry["ownership_status"] = "bounded_allocation_only"

    # The main inventory loop registers both handle+8 and handle+10 on the
    # same entry. A separate resource root registers +10 without +8.
    inventory_counts = 0
    for identity, entry in entries.items():
        if entry.get("source_field") != "handle+0x0a":
            continue
        block, module, offset = identity.split(":")
        selected = entries.get(f"{block}:{module}:{int(offset) - 2}")
        if not selected or selected.get("source_field") != "handle.selected":
            continue
        if selected.get("source_owner") != entry.get("source_owner"):
            continue
        if not all(item["kind"] == "0x02" and item["flags"] == "0x0204"
                   for item in entry["file_occurrences"]):
            continue
        entry["owner"] = entry["source_owner"]
        entry["field"] = "handle.instantiated_item_number"
        entry["semantic"] = {
            "owner": entry["owner"],
            "field": entry["field"],
            "meaning": "signed 16-bit instantiated item quantity/number on the main inventory-list handle; negative values have special handling",
            "source_rule": "hype-inventory-main-handle-number-v1",
            "confidence": "source_verified",
        }
        entry["meaning_status"] = "source_verified"
        entry["unresolved"] = [
            "runtime save/reload not separately corroborated for this item",
            "relationship to the separate item-member+10 quantity remains unverified",
        ]
        inventory_counts += 1
    if allocation_joins is not None and inventory_counts != 62:
        raise ValueError(f"expected 62 main-list inventory handle numbers, found {inventory_counts}")

    capacity_items = [
        (identity, entry) for identity, entry in entries.items()
        if entry.get("source_rule") == "hype-inventory-item-max-capacity-v1"
    ]
    if allocation_joins is not None:
        if len(capacity_items) != 63 or any(
            entry["meaning_status"] != "source_verified"
            or entry.get("source_field") != "item.max_element_capacity"
            or any(item["kind"] != "0x02" or item["flags"] != "0x0204"
                   or item["value_size"] != 2 for item in entry["file_occurrences"])
            for _, entry in capacity_items
        ):
            raise ValueError("the 63 rooted inventory item capacities were not fully resolved")
        if entries["0:10:5790"].get("source_rule") == "hype-inventory-item-max-capacity-v1":
            raise ValueError("the handle +0x0a word was classified as item capacity")

        resource = entries["0:10:5790"]
        resource_join = resource["source_allocation_join"]
        resource_allocation = resource_join["candidates"][0]
        resource_capacity = entries["0:10:5828"]
        if (resource_join["status"] != "single_bounded_allocation"
                or resource_allocation["owner"] != "fix"
                or resource_allocation["allocation_data_start"] != 5780
                or resource_allocation["allocation_end"] != 5804
                or resource_capacity.get("source_field") != "item.max_element_capacity"
                or resource_capacity["source_allocation_join"]["candidates"][0]["allocation_data_start"] != 5808
                or resource["meaning_status"] != "unknown"
                or any(item["kind"] != "0x02" or item["flags"] != "0x0204"
                       or item["value_size"] != 2 for item in resource["file_occurrences"])):
            raise ValueError("special inventory resource balance changed source path or save shape")
        resource["ownership_status"] = "source_object_and_field"
        resource["source_owner"] = "Fix inventory special resource handle"
        resource["source_field"] = "handle.current_element_balance"
        resource["source_type"] = "signed 16-bit amount"
        resource["source_rule"] = "hype-inventory-special-resource-balance-v1"
        resource["owner"] = resource["source_owner"]
        resource["field"] = resource["source_field"]
        resource["semantic"] = {
            "owner": resource["owner"],
            "field": resource["field"],
            "meaning": "current amount of a separately rooted inventory resource, displayed in the stats panel and added or spent by inventory effects; its linked item's maximum element capacity clamps additions",
            "source_rule": "hype-inventory-special-resource-balance-v1",
            "confidence": "source_verified",
            "source_evidence": [
                "reference/decompilation/functions/00417fe0_FUN_00417fe0.c:100-102",
                "reference/decompilation/functions/0041d400_FUN_0041d400.c",
                "reference/decompilation/functions/0041d420_FUN_0041d420.c",
                "reference/decompilation/functions/0041d490_FUN_0041d490.c",
                "reference/decompilation/functions/0040c950_FUN_0040c950.c",
                "reference/decompilation/functions/0056e0e0_FUN_0056e0e0.c",
                "output/savegame-semantic-map/special-inventory-word-followup.md",
            ],
        }
        resource["meaning_status"] = "source_verified"
        resource["unresolved"] = [
            "the authored resource name and fresh-process restoration behavior are unverified",
        ]

    actor_hitpoints = [
        (identity, entry) for identity, entry in entries.items()
        if entry.get("source_rule") == "hype-standard-hitpoints-v1"
    ]
    if allocation_joins is not None:
        expected_hitpoint_fields = Counter({
            "StandardGame.HitPointsInit": 28,
            "StandardGame.HitPointsMaxInit": 28,
            "StandardGame.HitPointsMaxMax": 28,
        })
        actual_hitpoint_fields = Counter(entry.get("source_field") for _, entry in actor_hitpoints)
        if actual_hitpoint_fields != expected_hitpoint_fields or any(
            entry["meaning_status"] != "source_verified"
            or entry.get("source_type") != "u8"
            or any(item["kind"] != "0x01" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 for item in entry["file_occurrences"])
            for _, entry in actor_hitpoints
        ):
            raise ValueError("the 84 rooted StandardGame hit-point bytes were not fully resolved")
        if Counter(identity.split(":", 1)[0] for identity, _ in actor_hitpoints) != Counter({"1": 78, "0": 6}):
            raise ValueError("StandardGame hit-point package ownership changed")

    gauge_fields = [
        (identity, entry) for identity, entry in entries.items()
        if entry.get("source_rule") == "hype-gauge-pointer-chain-v1"
    ]
    if allocation_joins is not None:
        expected_gauge_fields = Counter({
            "GaugeChild.visual_phase": 17,
            "GaugeChild.progress_tick": 17,
            "GaugeRoot.completed_children": 3,
            "GaugeRoot.threshold_selector": 3,
        })
        if Counter(entry.get("source_field") for _, entry in gauge_fields) != expected_gauge_fields or any(
            not identity.startswith("0:5:")
            or entry.get("source_type") != "u8"
            or any(item["kind"] != "0x01" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 for item in entry["file_occurrences"])
            for identity, entry in gauge_fields
        ):
            raise ValueError("the 40 rooted gauge fields changed shape or ownership")
        for _, entry in gauge_fields:
            if entry["source_field"] == "GaugeRoot.threshold_selector":
                if entry["meaning_status"] != "unknown":
                    raise ValueError("the gauge threshold selector was promoted without a complete meaning")
                entry["partial_semantics"] = {
                    "known_role": "indexes a threshold byte in the gauge root's +0x18..+0x1b range",
                    "unknown": "authored selector purpose, legal values, and restoration behavior",
                    "source_evidence": [
                        "reference/decompilation/functions/0043aa70_FUN_0043aa70.c",
                        "output/savegame-semantic-map/gauge-pointer-chain-followup.md",
                    ],
                }
                entry["unresolved"] = ["gauge threshold selector has a known structural role but no complete authored meaning"]
            elif entry["meaning_status"] != "source_verified":
                raise ValueError("the 37 rooted gauge fields were not fully resolved")

        gauge_root_links = [
            (identity, entry) for identity, entry in entries.items()
            if entry.get("source_rule") == "hype-gauge-root-link-v1"
        ]
        if len(gauge_root_links) != 1 or gauge_root_links[0][0] != "0:5:4985156":
            raise ValueError("expected exactly the first gauge root's saved next-root link")
        gauge_link = gauge_root_links[0][1]
        gauge_link_join = gauge_link["source_allocation_join"]
        gauge_link_allocation = gauge_link_join["candidates"][0]
        gauge_target = gauge_link_join["reference_target"]
        if (gauge_link.get("source_owner") != "Fix gauge root #0 (source list order)"
                or gauge_link.get("source_field") != "GaugeRoot.next_root"
                or gauge_link.get("meaning_status") != "source_verified"
                or gauge_link_allocation["allocation_data_start"] != 4985156
                or gauge_link_allocation["allocation_end"] != 4985196
                or gauge_target["identity"] != "0:5:4985560"
                or any(item["kind"] != "0x11" or item["flags"] != "0x0204"
                       or item["value_size"] != 6 for item in gauge_link["file_occurrences"])):
            raise ValueError("gauge root link changed target, owner or reference save shape")
        gauge_link["semantic"]["source_evidence"] = [
            "reference/decompilation/functions/0043a8a0_FUN_0043a8a0.c:13",
            "output/savegame-semantic-map/gauge-pointer-chain-followup.md",
            "src/openspace/level/save_join.rs",
        ]
        gauge_link["unresolved"] = [
            "authored panel names and fresh-process restoration are unverified",
        ]

    if allocation_joins is not None:
        for identity, ordinal, source_pointer in [
            ("2:6:1463692", 0, "DAT_00610534"),
            ("2:6:1464196", 1, "DAT_00610538"),
        ]:
            entry = entries[identity]
            join = entry["source_allocation_join"]
            candidate = join["candidates"][0]
            occurrence = entry["file_occurrences"][0]
            if (join["status"] != "single_bounded_allocation"
                    or candidate["owner"] != "level"
                    or candidate["allocation_data_start"] != int(identity.split(":")[2])
                    or candidate["allocation_end"] - candidate["allocation_data_start"] != 88
                    or occurrence["kind"] != "0x10"
                    or occurrence["flags"] != "0x0204"
                    or occurrence["value_size"] != 36
                    or not (int(occurrence["value_hex"][:2], 16) & 0x80)
                    or entry["meaning_status"] != "unknown"):
                raise ValueError("post-load position snapshots changed source shape")
            if ordinal == 0:
                entry["ownership_status"] = "source_object_and_field"
                entry["source_owner"] = "level:/persos/world/0/0"
                entry["source_field"] = "position_matrix_prefix"
                entry["source_type"] = "36-byte prefix of an 88-byte uncompressed POS matrix"
            entry["partial_semantics"] = {
                "known_role": (
                    "Hype main-actor POS prefix used for post-load repositioning"
                    if ordinal == 0 else
                    "POS prefix of the first qualifying scene Perso, source-order candidate StdCamer, used for post-load camera repositioning"
                ),
                "registration_pointer": source_pointer,
                "translation_xyz": list(struct.unpack_from("<fff", bytes.fromhex(occurrence["value_hex"]), 4)),
                "source_rule": "hype-post-load-position-snapshots-v1",
                "source_evidence": [
                    "reference/decompilation/functions/0040f4c0_FUN_0040f4c0.c:13-22",
                    "reference/decompilation/functions/004390e0_FUN_004390e0.c:17-39",
                    "output/savegame-semantic-map/position-snapshot-followup.md",
                    "output/savegame-semantic-map/position-owner-followup.md",
                ],
                "unknown": (
                    "the rest of the 36-byte prefix and source restoration order"
                    if ordinal == 0 else
                    "exact collision-entry predicate proving StdCamer ownership, the rest of the prefix, and source restoration order"
                ),
            }
            entry["unresolved"] = [
                "the saved 36 bytes are only a prefix of the POS matrix; full value meaning is unresolved",
            ]

    sector_links = [entry for entry in entries.values()
                    if entry.get("source_rule") == "hype-character-sector-link-v1"]
    if allocation_joins is not None and (
        len(sector_links) != 53
        or any(entry["meaning_status"] != "source_verified" for entry in sector_links)
    ):
        raise ValueError("the 53 rooted character-sector references were not fully resolved")

    instantiated_item_references = 0
    for entry in entries.values():
        target = entry.get("source_allocation_join", {}).get("reference_target")
        if not target or target.get("superobject_kind") != 18:
            continue
        label = target.get("instantiated_item_name")
        if not label or target.get("owner") != "fix" or target.get("offset_in_allocation") != 0:
            raise ValueError("instantiated-item SuperObject target lacks a typed Fix handle")
        entry["partial_semantics"] = {
            "known_target_role": "Hype type 0x12 instantiated-item SuperObject pointing to a typed Fix inventory handle",
            "current_target": label,
            "source_evidence": [
                "reference/decompilation/functions/004fd370_HIE_fn_xLoadSuperObject.c:536",
                "reference/decompilation/functions/004fe6f0_HIE_fn_vRecursiveRotationUpdate.c:17",
                "src/openspace/level/save_join.rs",
            ],
            "unknown": "source owner/field, slot role, and action/restoration behavior",
        }
        entry["unresolved"] = [
            "instantiated-item target is known, but source field and gameplay/restoration semantics are not"
        ]
        instantiated_item_references += 1
    if allocation_joins is not None and instantiated_item_references != 16:
        raise ValueError(
            f"expected 16 instantiated-item SuperObject targets, found {instantiated_item_references}"
        )

    lever = entries["1:17:187072"]
    if (lever.get("source_owner"), lever.get("source_field")) != (
        "level:/persos/world/0/174", "DSG[1]"
    ) or any(item["kind"] != "0x10" or item["value_size"] != 1
             for item in lever["file_occurrences"]):
        raise ValueError("manor lever phase no longer matches the source actor and one-byte Boolean")
    lever["owner"] = lever["source_owner"]
    lever["field"] = "DSG[1]"
    lever["semantic"] = {
        "owner": lever["owner"],
        "field": lever["field"],
        "meaning": "persistent Boolean phase for alternating lever actions and steady poses; the AI flips it after either action",
        "source_rule": "hype-manor-lever-phase-v1",
        "confidence": "source_verified",
        "source_evidence": [
            "level:/ai/models/1396 Normal behaviors 2, 7 and 8",
            "output/savegame-semantic-map/manoir-lever-ai-model.json",
        ],
        "runtime_evidence": "a game-written physical-interaction save changed this record 01 to 00; its live DSG value was not independently reloaded",
    }
    lever["meaning_status"] = "source_verified"
    lever["unresolved"] = ["which Boolean state corresponds to physical up/down or passage state is unverified"]

    lever_action_selector = entries["1:17:187070"]
    if (lever_action_selector.get("source_owner") != "level:/persos/world/0/174"
            or lever_action_selector.get("source_field") != "DSG[3]"
            or lever_action_selector.get("source_type") != "designer type 0"
            or lever_action_selector.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] != "00"
                   for item in lever_action_selector["file_occurrences"])):
        raise ValueError("manor lever action selector changed owner or Boolean save shape")
    lever_action_selector["partial_semantics"] = {
        "known_role": "the lever actor's own DSG[3] selects one of two AI action-state maps in Normal behavior 2 when DSG[2] is nonzero; DSG[4] selects within the map. Normal behavior 3 sets DSG[3] to one when DSG[5] differs from one, and Normal behavior 5 clears it after an action-state choice",
        "source_rule": "hype-manor-lever-action-selector-v1",
        "source_evidence": [
            "level:/ai/models/1396 Normal behavior 2 rule 0 statement 1",
            "level:/ai/models/1396 Normal behavior 3 schedule statement 1 otherwise",
            "level:/ai/models/1396 Normal behavior 5 schedule statement 1",
            "src/openspace/level/save_join.rs",
            "output/savegame-semantic-map/manoir-lever-dsg3-followup.md",
        ],
        "unknown": "the physical lever or passage meaning, full conditions entering behaviors 3 and 5, and visible results of the two action-state groups",
    }
    lever_action_selector["unresolved"] = [
        "source AI proves guarded action selection and two local writes, but not what this Boolean remembers or which physical state either value represents",
    ]

    gogood_choice = entries["1:17:171448"]
    if (gogood_choice.get("source_owner") != "level:/persos/world/0/161"
            or gogood_choice.get("source_field") != "DSG[0]"
            or gogood_choice.get("source_type") != "designer type 2"
            or gogood_choice.get("meaning_status") != "unknown"
            or len(gogood_choice["file_occurrences"]) != 2
            or any(item["kind"] != "0x10" or item["flags"] != "0x0a0c"
                   or item["value_size"] != 1 or item["value_hex"] != "00"
                   for item in gogood_choice["file_occurrences"])):
        raise ValueError("Gogood return selector changed owner or saved unsigned-byte shape")
    gogood_choice["partial_semantics"] = {
        "known_role": "Gogood's own DSG[0] receives 1, 3 or 2 from Button_Hit-guarded Normal behaviors 3, 5 or 8; behavior 1 reads 1/2/3 to select Normal behavior 6/7/9, and IWorld reads the same actor field in conditional behavior requests",
        "observed_authored_values": [1, 2, 3],
        "source_rule": "hype-manor-gogood-choice-selector-v1",
        "source_evidence": [
            "level:/ai/models/1251 Normal behaviors 1, 3, 5 and 8",
            "level:/ai/models/154 Normal behavior 2 schedule statement 15",
            "src/openspace/level/save_join.rs",
            "output/savegame-semantic-map/gogood-return-dsg0-followup.md",
        ],
        "unknown": "gameplay labels for the three choices, unsupported guards and effects, procedure 207's native callback, and full restoration lifecycle",
    }
    gogood_choice["unresolved"] = [
        "the authored three-value script selector is known, but its complete gameplay effects and native behavior-change callback are not",
    ]

    mannequin_fields = [
        ("level:/persos/world/0/152", {0: "1:17:127662", 1: "1:17:127661", 2: "1:17:127660"}),
        ("level:/persos/world/0/153", {0: "1:17:131194", 1: "1:17:131193", 2: "1:17:131192"}),
        ("level:/persos/world/0/154", {0: "1:17:132114", 1: "1:17:132113", 2: "1:17:132112"}),
    ]
    for actor, fields in mannequin_fields:
        for index, identity in fields.items():
            entry = entries[identity]
            expected_type = "designer type 2" if index == 0 else "designer type 0"
            expected_value = "04" if index == 0 else "01"
            if (entry.get("source_owner") != actor
                    or entry.get("source_field") != f"DSG[{index}]"
                    or entry.get("source_type") != expected_type
                    or entry.get("meaning_status") != "unknown"
                    or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                           or item["value_size"] != 1 or item["value_hex"] != expected_value
                           for item in entry["file_occurrences"])):
                raise ValueError(f"mannequin designer field {identity} changed shape or source owner")
            if index == 2:
                entry["partial_semantics"] = {
                    "known_writes": "set to 1 after the mannequin's three-way slot/MainActor match or by the world's separate Magie branch; cleared by world slot assignments 1/2/3",
                    "source_rule": "hype-mannequin-third-field-writes-v1",
                    "unknown": "reader, gameplay purpose, and which write path produced this save's value 1",
                    "source_evidence": [
                        "level:/ai/models/939 Reflex behavior 0 rule 0",
                        "level:/ai/models/154 Normal behavior 2 schedule statements 10-13",
                        "output/savegame-semantic-map/mannequin-dsg-followup.md",
                    ],
                }
                entry["unresolved"] = ["the field's writer conditions are known, but no consuming role is verified"]
                continue
            entry["owner"] = actor
            entry["field"] = f"DSG[{index}]"
            if index == 0:
                meaning = "per-mannequin stage code: world assigns 1/2/3 to sword/crossbow/magic mannequins, the actor sets 4 after its matching MainActor condition when DSG[1] is 1, and MasterYoda requires 4 for a follow-on action; Magie can also receive 5"
                rule = "hype-mannequin-stage-code-v1"
                evidence = ["level:/ai/models/154 Normal behavior 2 schedule statements 10-13", "level:/ai/models/939 Reflex behavior 0 rule 0", "level:/ai/models/991 Normal behavior 4 schedule statements 0-2"]
            else:
                meaning = "Boolean permission for this mannequin to change its stage code to 4 after the matching slot/MainActor condition; the world clears it on slot assignments and can set it for Magie in a later branch"
                rule = "hype-mannequin-stage-permission-v1"
                evidence = ["level:/ai/models/939 Reflex behavior 0 rule 0", "level:/ai/models/154 Normal behavior 2 schedule statements 10-13"]
            entry["semantic"] = {
                "owner": actor,
                "field": entry["field"],
                "meaning": meaning,
                "source_rule": rule,
                "confidence": "source_verified",
                "source_evidence": evidence + ["output/savegame-semantic-map/mannequin-dsg-followup.md"],
            }
            entry["meaning_status"] = "source_verified"
            entry["unresolved"] = ["broader authored sequence names and fresh-process restoration are unverified"]

    shared_mannequin_gate = entries["0:17:4557"]
    if (shared_mannequin_gate.get("source_owner") != "fix:/persos/15"
            or shared_mannequin_gate.get("source_field") != "DSG[152]"
            or shared_mannequin_gate.get("source_type") != "designer type 0"
            or shared_mannequin_gate.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                   for item in shared_mannequin_gate["file_occurrences"])):
        raise ValueError("Global mannequin gate changed source owner or Boolean save shape")
    shared_mannequin_gate["partial_semantics"] = {
        "known_role": "zero permits the shared manor mannequin Reflex slot-match path; IWorld selector branches 2-4 clear the flag and branches 5-6 set it, while Yoda behavior 18 also sets it",
        "source_rule": "hype-global-mannequin-match-gate-v1",
        "source_evidence": [
            "level:/ai/models/939 Reflex behavior 0 rule 0",
            "level:/ai/models/154 Normal behavior 2 schedule statements 10-14",
            "level:/ai/models/991 Normal behavior 18 schedule statement 2",
            "output/savegame-semantic-map/global-dsg152-followup.md",
        ],
        "unknown": "the full authored mannequin/world event meaning and fresh-process restoration",
    }
    shared_mannequin_gate["unresolved"] = [
        "the zero-gated mannequin behavior and set/clear paths are known, but the complete gameplay state is not",
    ]

    camera_look_gate = entries["0:17:4701"]
    if (camera_look_gate.get("source_owner") != "fix:/persos/15"
            or camera_look_gate.get("source_field") != "DSG[18]"
            or camera_look_gate.get("source_type") != "designer type 0"
            or camera_look_gate.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                   for item in camera_look_gate["file_occurrences"])):
        raise ValueError("Global camera-look gate source join or Boolean save shape changed")
    camera_look_gate["owner"] = camera_look_gate["source_owner"]
    camera_look_gate["field"] = camera_look_gate["source_field"]
    camera_look_gate["semantic"] = {
        "owner": camera_look_gate["owner"],
        "field": camera_look_gate["field"],
        "meaning": "Boolean gate for manual camera-look mode: 0 permits entry when the input and companion state allow it; 1 blocks entry and causes active camera-look behavior to exit",
        "source_rule": "hype-global-camera-look-gate-v1",
        "confidence": "source_verified",
        "source_evidence": [
            "fix:/ai/models/437 Normal behaviors 0, 3, and 8",
            "fix:/ai/models/28 Reflex behaviors 1, 2, 3, 4, and 6",
            "output/savegame-semantic-map/global-dsg18-followup.md",
        ],
    }
    camera_look_gate["meaning_status"] = "source_verified"
    camera_look_gate["unresolved"] = [
        "fresh-process restoration and any additional dynamic or native consumers are unverified",
    ]

    camera_look_allow = entries["0:17:4702"]
    if (camera_look_allow.get("source_owner") != "fix:/persos/15"
            or camera_look_allow.get("source_field") != "DSG[17]"
            or camera_look_allow.get("source_type") != "designer type 0"
            or camera_look_allow.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                   for item in camera_look_allow["file_occurrences"])):
        raise ValueError("Global camera-look allow field or Boolean save shape changed")
    camera_look_allow["owner"] = camera_look_allow["source_owner"]
    camera_look_allow["field"] = camera_look_allow["source_field"]
    camera_look_allow["semantic"] = {
        "owner": camera_look_allow["owner"],
        "field": camera_look_allow["field"],
        "meaning": "Boolean manual camera-look permission: 1 allows BT_Camera_Look to enter its look behavior when pressed and DSG[18] is 0; 0 prevents entry and is an exit condition from active look behavior",
        "source_rule": "hype-global-camera-look-allow-v1",
        "confidence": "source_verified",
        "source_evidence": [
            "fix:/ai/models/437 Normal behaviors 0, 3, and 8",
            "fix:/ai/models/28 Normal behavior 24 and Reflex behaviors 1, 2, 3, 4, and 6",
            "output/savegame-semantic-map/global-dsg17-followup.md",
        ],
    }
    camera_look_allow["meaning_status"] = "source_verified"
    camera_look_allow["unresolved"] = [
        "Hype-local write guards, fresh-process restoration and additional dynamic/native consumers are not fully resolved",
    ]

    sighting_snapshot = entries["0:17:4396"]
    if (sighting_snapshot.get("source_owner") != "fix:/persos/15"
            or sighting_snapshot.get("source_field") != "DSG[30]"
            or sighting_snapshot.get("source_type") != "designer type 12"
            or sighting_snapshot.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 12 for item in sighting_snapshot["file_occurrences"])):
        raise ValueError("Global checkpoint sighting field or vector save shape changed")
    sighting_snapshot["owner"] = sighting_snapshot["source_owner"]
    sighting_snapshot["field"] = sighting_snapshot["source_field"]
    sighting_snapshot["semantic"] = {
        "owner": sighting_snapshot["owner"],
        "field": sighting_snapshot["field"],
        "meaning": "checkpoint snapshot of Hype's sighting vector; Hype later uses its negated X/Y components with Z set to zero as input to ChangeSighting",
        "source_rule": "hype-global-checkpoint-sighting-v1",
        "confidence": "source_verified",
        "source_evidence": [
            "level:/ai/models/348 Normal behavior 0 schedule statement 0",
            "fix:/ai/models/28 Normal behavior 0 schedule statement 207",
            "output/savegame-semantic-map/global-dsg30-followup.md",
        ],
    }
    sighting_snapshot["meaning_status"] = "source_verified"
    sighting_snapshot["unresolved"] = [
        "native ChangeSighting orientation behavior, wider consumers and fresh-process restoration are unverified",
    ]

    travel_preset = entries["0:17:4564"]
    if (travel_preset.get("source_owner") != "fix:/persos/15"
            or travel_preset.get("source_field") != "DSG[145]"
            or travel_preset.get("source_type") != "designer type 0"
            or travel_preset.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                   for item in travel_preset["file_occurrences"])):
        raise ValueError("Manoir travel preset field or Boolean save shape changed")
    travel_preset["owner"] = travel_preset["source_owner"]
    travel_preset["field"] = travel_preset["source_field"]
    travel_preset["semantic"] = {
        "owner": travel_preset["owner"],
        "field": travel_preset["field"],
        "meaning": "Manoir time-travel placement preset selector: 1 copies the script's four position vectors 15-18 into active vectors 7-10; 0 selects vectors 11-14; the travel camera also branches on zero for a sound cue",
        "source_rule": "hype-manoir-travel-preset-v1",
        "confidence": "source_verified",
        "source_evidence": [
            "level:/ai/models/154 Normal behavior 2 schedule statement 6",
            "level:/ai/models/586 Normal behavior 0 schedule statement 1",
            "level:/ai/models/638 Normal behavior 4 schedule statement 4",
            "output/savegame-semantic-map/global-dsg145-followup.md",
        ],
    }
    travel_preset["meaning_status"] = "source_verified"
    travel_preset["unresolved"] = [
        "the in-world names for the two travel locations, any additional native consumers and fresh-process restoration are unverified",
    ]

    manor_camera_signal = entries["0:17:4563"]
    if (manor_camera_signal.get("source_owner") != "fix:/persos/15"
            or manor_camera_signal.get("source_field") != "DSG[146]"
            or manor_camera_signal.get("source_type") != "designer type 0"
            or manor_camera_signal.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                   for item in manor_camera_signal["file_occurrences"])):
        raise ValueError("Global manor camera signal changed source owner or Boolean save shape")
    manor_camera_signal["partial_semantics"] = {
        "known_role": "shared signal used by two manor fixed-camera actors: they set it to one on branches requesting camera state 3 and clear it on reset/transition branches; one schedules behavior 9 and zero behavior 0",
        "source_rule": "hype-global-manoir-camera-signal-v1",
        "source_evidence": [
            "level:/ai/models/397 Normal behaviors 0, 1, 19 and 21",
            "level:/ai/models/154 Normal behavior 2 schedule statement 20",
            "output/savegame-semantic-map/global-dsg146-followup.md",
        ],
        "unknown": "actual camera state 3 effects, actor arbitration, full lifecycle and fresh-process restoration",
    }
    manor_camera_signal["unresolved"] = [
        "the camera branch relation is source-backed but the complete gameplay meaning is not",
    ]

    manor_time_gate = entries["0:17:4676"]
    if (manor_time_gate.get("source_owner") != "fix:/persos/15"
            or manor_time_gate.get("source_field") != "DSG[39]"
            or manor_time_gate.get("source_type") != "designer type 0"
            or manor_time_gate.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                   for item in manor_time_gate["file_occurrences"])):
        raise ValueError("Global manor time gate changed source owner or Boolean save shape")
    manor_time_gate["partial_semantics"] = {
        "known_role": "IWorld selector value 6 sets one; later one/zero branches pair with Global DSG[0] values 3/2 and request Gogood-return Normal behaviors through procedure 207; iDummy_TimeTravel reads both polarities separately",
        "source_rule": "hype-global-manoir-time-gate-v1",
        "source_evidence": [
            "level:/ai/models/154 Normal behavior 2 schedule statements 14-15",
            "level:/ai/models/586 Normal behaviors 0 and 1",
            "output/savegame-semantic-map/global-dsg39-followup.md",
            "output/savegame-semantic-map/shared-change-comport-followup.md",
        ],
        "unknown": "the guarded unsupported effects, specific time-travel state and fresh-process restoration",
    }
    manor_time_gate["unresolved"] = [
        "the source proves Boolean branching in the world and time-travel actors, but not a complete gameplay-state meaning",
    ]

    channel_72_gate = entries["0:17:4721"]
    if (channel_72_gate.get("source_owner") != "fix:/persos/15"
            or channel_72_gate.get("source_field") != "DSG[0]"
            or channel_72_gate.get("source_type") != "designer type 0"
            or channel_72_gate.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 for item in channel_72_gate["file_occurrences"])):
        raise ValueError("Global channel-72 condition source join or save shape changed")
    channel_72_gate["partial_semantics"] = {
        "known_condition": "Hype Normal behavior 0 schedule statement 192 activates channel 72 when Global DSG[0] == 1 and deactivates it otherwise",
        "source_rule": "hype-global-channel72-condition-v1",
        "source_evidence": [
            "fix:/ai/models/28 Normal behavior 0 schedule statement 192",
            "output/savegame-semantic-map/global-dsg0-followup.md",
        ],
        "unknown": "authored channel-72 target and observable gameplay purpose",
    }
    channel_72_gate["unresolved"] = [
        "channel-72 activation is source-backed, but the channel's authored target and gameplay purpose are unknown",
    ]

    for index, identity, known_role, source_sites, report in [
        (
            4,
            "0:17:4717",
            "value one gates Hype and iHypeStatue branches with 35 Raymap-labeled SwapLinkTableObjects calls over matching pairs; Hype also issues material-diffuse calls under this gate",
            ["fix:/ai/models/28 Normal behaviors 0 and 80", "fix:/ai/models/688 Normal behaviors 3 and 4"],
            "output/savegame-semantic-map/global-dsg4-followup.md",
        ),
        (
            37,
            "0:17:4678",
            "value one gates Hype's ChangeSighting call using negated X/Y components of the checkpoint-saved Global DSG[30] vector; Hype sets and clears this latch around an action sequence",
            ["fix:/ai/models/28 Normal behaviors 0, 81 and 91", "level:/ai/models/348 Normal behavior 0"],
            "output/savegame-semantic-map/global-dsg37-followup.md",
        ),
    ]:
        entry = entries[identity]
        if (entry.get("source_owner") != "fix:/persos/15"
                or entry.get("source_field") != f"DSG[{index}]"
                or entry.get("source_type") != "designer type 0"
                or entry.get("meaning_status") != "unknown"
                or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                       or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                       for item in entry["file_occurrences"])):
            raise ValueError(f"Global authored effect gate {identity} changed owner or Boolean save shape")
        entry["partial_semantics"] = {
            "known_role": known_role,
            "source_rule": "hype-global-authored-effect-gates-v1",
            "source_evidence": source_sites + [report],
            "unknown": "the complete authored gameplay purpose, behavior-entry conditions and fresh-process restoration",
        }
        entry["unresolved"] = [
            "the exact conditional AI effects are known, but no complete gameplay-state label is supported",
        ]

    for index, identity, known_role, report in [
        (
            1,
            "0:17:4720",
            "Hype uses value 1 to activate channel 73 and to permit weapon-pad/GetCrossbow input branches; Hype transitions set/clear it with other Global fields, while iHypeStatue selects channels 19/14 from it",
            "output/savegame-semantic-map/global-dsg1-followup.md",
        ),
        (
            2,
            "0:17:4719",
            "Hype transition behavior 76 sets 1 while disabling channels, behavior 77 clears it while restoring channels; value 1 suppresses Hype channel 29 and statue channel 16, while value 0 permits several Armor HUD show branches",
            "output/savegame-semantic-map/global-dsg2-followup.md",
        ),
    ]:
        entry = entries[identity]
        if (entry.get("source_owner") != "fix:/persos/15"
                or entry.get("source_field") != f"DSG[{index}]"
                or entry.get("source_type") != "designer type 0"
                or entry.get("meaning_status") != "unknown"
                or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                       or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                       for item in entry["file_occurrences"])):
            raise ValueError(f"Global transition field {identity} changed owner or Boolean save shape")
        entry["partial_semantics"] = {
            "known_role": known_role,
            "source_rule": "hype-global-hype-transition-flags-v1",
            "source_evidence": [
                "fix:/ai/models/28 Normal behaviors 0, 76 and 77",
                "fix:/ai/models/688 Normal behaviors 3 and 4",
                report,
            ],
            "test_actor_limit": "iDummy_SaveTesteurs can snapshot, restore and force one; its writes do not identify normal gameplay state",
            "unknown": "authored channel/geometry identity, full gameplay concept and fresh-process restoration",
        }
        entry["unresolved"] = [
            "source AI proves channel and input effects but not a definitive gameplay state name",
        ]

    object_latches = {
        220: ("0:17:4486", "fix:/ai/models/1080", "level:/ai/models/1106"),
        221: ("0:17:4485", "fix:/ai/models/1051", "level:/ai/models/1106"),
        222: ("0:17:4484", "fix:/ai/models/1109", "level:/ai/models/1106"),
        223: ("0:17:4483", "fix:/ai/models/993", "level:/ai/models/1182"),
        224: ("0:17:4482", "fix:/ai/models/1022", "level:/ai/models/1182"),
        225: ("0:17:4481", "fix:/ai/models/964", None),
        226: ("0:17:4480", "fix:/ai/models/935", None),
        227: ("0:17:4479", "fix:/ai/models/1196", None),
        228: ("0:17:4478", "fix:/ai/models/1167", "level:/ai/models/1144"),
    }
    for index, (identity, template_model, level_model) in object_latches.items():
        entry = entries[identity]
        if (entry.get("source_owner") != "fix:/persos/15"
                or entry.get("source_field") != f"DSG[{index}]"
                or entry.get("source_type") != "designer type 0"
                or entry.get("meaning_status") != "unknown"
                or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                       or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                       for item in entry["file_occurrences"])):
            raise ValueError(f"Global object latch {identity} changed owner or Boolean save shape")
        entry["partial_semantics"] = {
            "known_role": "Boolean latch read as zero and explicitly assigned one by its authored object-template AI branch",
            "source_rule": "hype-global-object-template-latches-v1",
            "object_template_model": template_model,
            "additional_level_model": level_model,
            "source_evidence": [
                f"{template_model} Reflex behavior 0 rule 0 statement 0",
                *([f"{level_model} Reflex behavior"] if level_model else []),
                "output/savegame-semantic-map/global-object-cluster-followup.md",
            ],
            "unknown": "the enclosing unsupported conditions/actions and MainActor DSG[97] prevent a specific item or gameplay-event meaning",
        }
        entry["unresolved"] = [
            "source AI proves a clear-to-set latch, but the event it remembers and its later gameplay effect are unresolved",
        ]

    for index, identity, known_role, evidence in [
        (
            147,
            "0:17:4562",
            "Manoir event flag set after iGogood Action state 36; value 1 triggers iHypeStatue_Manoir_MAD's typed KillSelf branch",
            ["level:/ai/models/210 Normal behavior 12 schedule statements 0-1", "level:/ai/models/294 Normal behavior 10 rule 0"],
        ),
        (
            149,
            "0:17:4560",
            "Manoir grille/IWorld sequence latch: the grille tests value 1 before a behavior transition and writes 1 in its next behavior; IWorld has guarded set and clear paths",
            ["level:/ai/models/272 Normal behaviors 0-1", "level:/ai/models/154 Normal behaviors 0 and 2"],
        ),
    ]:
        entry = entries[identity]
        if (entry.get("source_owner") != "fix:/persos/15"
                or entry.get("source_field") != f"DSG[{index}]"
                or entry.get("source_type") != "designer type 0"
                or entry.get("meaning_status") != "unknown"
                or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                       or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                       for item in entry["file_occurrences"])):
            raise ValueError(f"Global Manoir field {identity} changed owner or Boolean save shape")
        entry["partial_semantics"] = {
            "known_role": known_role,
            "source_rule": "hype-global-manoir-event-gates-v1",
            "source_evidence": evidence + ["output/savegame-semantic-map/global-manoir-147-149-followup.md"],
            "test_actor_limit": "iDummy_SaveTesteurs can snapshot, restore and force this Global field to one; the fixture value alone does not prove normal progression",
            "unknown": "the full event and visual/quest effects of enclosing unsupported AI actions, plus fresh-process restoration",
        }
        entry["unresolved"] = [
            "specific Manoir effect and the cause of this save's value one remain unresolved",
        ]

    further_global_gates = [
        (38, "0:17:4677", "after higher flags fail, one selects configuration tier 1 and zero selects fallback tier 0 in iDummy_PourCercles; IWorld also reads it in conditional Gogood behavior selection", "shared-pour-cercles-followup.md"),
        (40, "0:17:4675", "after DSG[44] and DSG[42] fail, one selects configuration tier 2 in iDummy_PourCercles", "shared-pour-cercles-followup.md"),
        (41, "0:17:4674", "zero deactivates level allocation 5452; one can select iDummy_TimeTravel behavior 5 when Func_GetGameState is not 3 and other guards pass", "shared-time-selector-followup.md"),
        (42, "0:17:4673", "after DSG[44] fails, one selects configuration tier 3 in iDummy_PourCercles", "shared-pour-cercles-followup.md"),
        (43, "0:17:4672", "zero deactivates level allocation 5460; one can select iDummy_TimeTravel behavior 6 when Func_GetGameState is not 4 and earlier routes fail", "shared-time-selector-followup.md"),
        (44, "0:17:4671", "one selects configuration tier 4 in the first iDummy_PourCercles branch", "shared-pour-cercles-followup.md"),
        (45, "0:17:4670", "Hype copies this Boolean into its own DSG[57] in Normal behavior 0 statement 67; its Reflex behavior 7 also overwrites that actor field from Global DSG[73] == 4", "rete-ast-recovery-followup.md"),
        (66, "0:17:4649", "one selects a Gogood return actor Normal behavior 10 request under IWorld's outer gates, plus separate mannequin-frame and return-script branches", "shared-change-comport-followup.md"),
        (219, "0:17:4487", "IWorld selector 6 sets one; one gates later IWorld branches and i_Gogood_MasterYoda KillSelf", "global-dsg219-233-followup.md"),
        (233, "0:17:4473", "one selects Hype's ActivateMap input branch with unsupported operation 374; IWorld clears it under the DSG[147] zero branch", "global-dsg219-233-followup.md"),
    ]
    for index, identity, known_role, report in further_global_gates:
        entry = entries[identity]
        if (entry.get("source_owner") != "fix:/persos/15"
                or entry.get("source_field") != f"DSG[{index}]"
                or entry.get("source_type") != "designer type 0"
                or entry.get("meaning_status") != "unknown"
                or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                       or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                       for item in entry["file_occurrences"])):
            raise ValueError(f"Global branch field {identity} changed owner or Boolean save shape")
        entry["partial_semantics"] = {
            "known_role": known_role,
            "source_rule": "hype-global-conditional-branches-v1",
            "source_evidence": [f"output/savegame-semantic-map/{report}"],
            "unknown": "complete authored effect, normal writer lifecycle and fresh-process restoration",
        }
        entry["unresolved"] = [
            "the actor-qualified branch is known, but a complete gameplay-state meaning is not",
        ]

    hype_selector = entries["0:17:15540"]
    if (hype_selector.get("source_owner") != "level:/persos/world/0/0"
            or hype_selector.get("source_field") != "DSG[57]"
            or hype_selector.get("source_type") != "designer type 0"
            or hype_selector.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] != "00"
                   for item in hype_selector["file_occurrences"])):
        raise ValueError("Hype DSG[57] changed owner or saved Boolean shape")
    hype_selector["partial_semantics"] = {
        "known_role": "Hype's own Boolean DSG[57] is set from Global DSG[45] in Normal behavior 0 and independently from Global DSG[73] == 4 in Reflex behavior 7; 121 typed tests of this actor slot gate several behavior paths",
        "source_rule": "hype-hype-dsg57-branch-selector-v1",
        "source_evidence": [
            "fix:/ai/models/28 Normal behavior 0 schedule statement 67",
            "fix:/ai/models/28 Reflex behavior 7 rule 0 statement 35",
            "src/openspace/level/save_join.rs",
            "output/savegame-semantic-map/rete-ast-recovery-followup.md",
            "output/savegame-semantic-map/hype-dsg57-identity-followup.md",
        ],
        "unknown": "a single authored gameplay meaning for the broad downstream branches, update ordering and fresh-process restoration",
    }
    hype_selector["unresolved"] = [
        "the saved Hype actor slot and two script writers are joined, but the broad consumer fan-out does not establish a complete gameplay meaning",
    ]

    hype_link_swap_latch = entries["0:17:15586"]
    if (hype_link_swap_latch.get("source_owner") != "level:/persos/world/0/0"
            or hype_link_swap_latch.get("source_field") != "DSG[11]"
            or hype_link_swap_latch.get("source_type") != "designer type 0"
            or hype_link_swap_latch.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] != "00"
                   for item in hype_link_swap_latch["file_occurrences"])):
        raise ValueError("Hype DSG[11] changed owner or saved Boolean shape")
    hype_link_swap_latch["partial_semantics"] = {
        "known_role": "Hype's own Boolean DSG[11] gates six repeated procedure-82 calls and action/channel branches; four identical Reflex rules compare function 205's selected inventory Object item info word to 44, then set or clear this latch alongside Hype DSG[10] under further guards",
        "source_rule": "hype-hype-dsg11-link-swap-latch-v1",
        "source_evidence": [
            "fix:/ai/models/28 Normal behaviors 0, 89 and 103",
            "fix:/ai/models/28 Reflex behaviors 1, 3, 4 and 6",
            "reference/raymap/Assets/Scripts/OpenSpace/AI/AITypes/AITypes_Hype.cs procedure 82 and function 205",
            "reference/decompilation/functions/0041d890_FUN_0041d890.c:21-56",
            "reference/decompilation/functions/004a3940_INV_fn_wGetInfo.c:8-13",
            "output/savegame-semantic-map/hype-dsg11-actor-local-followup.md",
            "output/savegame-semantic-map/native-link-swap-83.md",
        ],
        "unknown": "meaning of Object info code 44, native procedure-82 effect, visible state, further trigger guards, and fresh-process restoration",
    }
    hype_link_swap_latch["unresolved"] = [
        "the actor-local latch and repeated script operations are proved, but no complete gameplay-state name is supported",
    ]

    configuration_tier = entries["0:17:4639"]
    if (configuration_tier.get("source_owner") != "fix:/persos/15"
            or configuration_tier.get("source_field") != "DSG[73]"
            or configuration_tier.get("source_type") != "designer type 2"
            or configuration_tier.get("meaning_status") != "unknown"
            or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 for item in configuration_tier["file_occurrences"])):
        raise ValueError("Global configuration tier changed owner or UByte save shape")
    configuration_tier["partial_semantics"] = {
        "known_role": "current configuration tier: iDummy_PourCercles writes 4, 3, 2, 1 or 0 from ordered Global DSG[44], [42], [40], [38] branches; Hype reads == 4 and IWorld can reset it to zero",
        "observed_authored_values": [0, 1, 2, 3, 4],
        "storage_range": "unsigned byte 0..255; do not constrain preserved values to the observed authored set",
        "source_rule": "hype-global-configuration-tier-v1",
        "source_evidence": [
            "fix:/ai/models/535 Normal behavior 2 rule 0",
            "fix:/ai/models/28 Reflex behavior 7 rule 0 statement 35",
            "level:/ai/models/154 Normal behavior 2 schedule statement 20",
            "output/savegame-semantic-map/global-dsg73-tier-followup.md",
        ],
        "unknown": "what the tier denotes in gameplay, effects of all values, ordinary writer lifecycle and fresh-process restoration",
    }
    configuration_tier["unresolved"] = [
        "the tier values and selected script effects are known, but its complete authored purpose is not",
    ]

    for index, identity, known_role, unknown in [
        (29, "0:17:4384", "the manor checkpoint copies Hype SelfPosition into this Global vector", "no ordinary gameplay reader or restoration effect has been verified"),
        (31, "0:17:4408", "the manor checkpoint copies its local Variable[1] into this Global vector", "that local vector's only explicit producer reads one-byte Boolean Global DSG[1]; the source type mismatch and gameplay purpose are unresolved"),
    ]:
        entry = entries[identity]
        if (entry.get("source_owner") != "fix:/persos/15"
                or entry.get("source_field") != f"DSG[{index}]"
                or entry.get("source_type") != "designer type 12"
                or entry.get("meaning_status") != "unknown"
                or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                       or item["value_size"] != 12 for item in entry["file_occurrences"])):
            raise ValueError(f"Global checkpoint vector {identity} changed owner or save shape")
        entry["partial_semantics"] = {
            "known_role": known_role,
            "source_rule": "hype-global-checkpoint-vector-writes-v1",
            "source_evidence": [
                "level:/ai/models/348 Normal behavior 0 schedule statement 0",
                "output/savegame-semantic-map/global-dsg29-31-followup.md",
            ],
            "unknown": unknown,
        }
        entry["unresolved"] = [unknown]

    checkpoint_ref = entries["0:17:4420"]
    checkpoint_target = checkpoint_ref["source_allocation_join"]["reference_target"] if allocation_joins is not None else None
    if allocation_joins is not None:
        if (checkpoint_ref.get("source_owner") != "fix:/persos/15"
                or checkpoint_ref.get("source_field") != "DSG[32]"
                or checkpoint_ref.get("source_type") != "designer type 10"
                or checkpoint_ref.get("meaning_status") != "unknown"
                or checkpoint_target["identity"] != "2:6:2170476"
                or checkpoint_target["superobject_kind"] != 4
                or checkpoint_target["rooted_perso"] != "level:/persos/world/0/35"
                or checkpoint_target["superobject_data_identity"] != "1:5:726380"
                or any(item["kind"] != "0x11" or item["flags"] != "0x0204"
                       or item["value_size"] != 6 for item in checkpoint_ref["file_occurrences"])):
            raise ValueError("Global checkpoint Perso reference changed target or save shape")
        checkpoint_ref["partial_semantics"] = {
            "known_role": "checkpoint-assigned Perso reference, currently targeting the rooted manor checkpoint actor; the checkpoint AI chooses MainActor or Me in opposite arms of an unresolved condition",
            "source_rule": "hype-global-checkpoint-perso-reference-v1",
            "source_evidence": [
                "level:/ai/models/348 Normal behavior 0 schedule statement 0",
                "output/savegame-semantic-map/global-dsg32-checkpoint-target.md",
            ],
            "unknown": "condition selecting MainActor versus checkpoint, gameplay consumer, and fresh-process restoration",
        }
        checkpoint_ref["unresolved"] = [
            "the reference target is typed and rooted, but its full gameplay role remains unknown",
        ]

    if allocation_joins is not None:
        toolcam_word = entries["0:5:4986280"]
        toolcam_join = toolcam_word["source_allocation_join"]
        toolcam_candidate = toolcam_join["candidates"][0]
        if (toolcam_join["status"] != "single_bounded_allocation"
                or toolcam_candidate["owner"] != "fix"
                or toolcam_candidate["allocation_data_start"] != 4986280
                or toolcam_candidate["allocation_end"] != 4986284
                or toolcam_word["meaning_status"] != "unknown"
                or any(item["kind"] != "0x02" or item["flags"] != "0x0204"
                       or item["value_size"] != 2 for item in toolcam_word["file_occurrences"])):
            raise ValueError("ToolCam post-load word changed source allocation or save shape")
        toolcam_word["partial_semantics"] = {
            "known_role": "low 16-bit snapshot of the +0x98 dword on the nested object reached through ToolCam slot 1; read before save and written after player load",
            "source_rule": "hype-toolcam-post-load-word-v1",
            "source_evidence": [
                "reference/decompilation/functions/0040f4c0_FUN_0040f4c0.c:21",
                "reference/decompilation/functions/00431130_FUN_00431130.c:15-21",
                "reference/decompilation/functions/004390e0_FUN_004390e0.c:30-36",
                "output/savegame-semantic-map/post-load-word-followup.md",
            ],
            "unknown": "declared member name, gameplay meaning of state-like values, and behavior of the 16-bit truncation",
        }
        toolcam_word["unresolved"] = [
            "the source pointer chain and low-half save are known, but the nested field's gameplay meaning is unresolved",
        ]

    if allocation_joins is not None:
        game_clock = entries["0:5:4"]
        clock_join = game_clock["source_allocation_join"]
        clock_candidate = clock_join["candidates"][0]
        if (clock_join["status"] != "single_bounded_allocation"
                or clock_candidate["owner"] != "fix"
                or clock_candidate["allocation_data_start"] != 4
                or clock_candidate["allocation_end"] != 12
                or game_clock["meaning_status"] != "unknown"
                or any(item["kind"] != "0x10" or item["flags"] != "0x0204"
                       or item["value_size"] != 8 for item in game_clock["file_occurrences"])):
            raise ValueError("Hype game clock source allocation or save shape changed")
        game_clock["ownership_status"] = "source_object_and_field"
        game_clock["source_owner"] = "Hype game clock"
        game_clock["source_field"] = "elapsed_time_ms"
        game_clock["source_type"] = "64-bit timer value in milliseconds"
        game_clock["source_rule"] = "hype-game-clock-v1"
        game_clock["owner"] = game_clock["source_owner"]
        game_clock["field"] = game_clock["source_field"]
        game_clock["semantic"] = {
            "owner": game_clock["owner"],
            "field": game_clock["field"],
            "meaning": "elapsed game-clock milliseconds sampled before saving; after player-mode load, the reset running timer is set forward to the saved value when lower",
            "source_rule": "hype-game-clock-v1",
            "confidence": "source_verified",
            "source_evidence": [
                "reference/decompilation/functions/00401e00_FUN_00401e00.c:32-33",
                "reference/decompilation/functions/00431130_FUN_00431130.c:20",
                "reference/decompilation/functions/004261a0_FUN_004261a0.c:13-16",
                "reference/decompilation/functions/00502680_FUN_00502680.c:13-16",
                "output/savegame-semantic-map/save-timer-followup.md",
            ],
        }
        game_clock["meaning_status"] = "source_verified"
        game_clock["unresolved"] = [
            "exact pause/menu inclusion and nonzero high-word behavior were not separately observed",
        ]
        clock_lines = non_records["SaveGame0/General000.sav"].get("text_lines")
        clock_bytes = bytes.fromhex(game_clock["file_occurrences"][0]["value_hex"])
        if (clock_lines is None or len(clock_lines) != 4
                or int(clock_lines[2]) != int.from_bytes(clock_bytes[:4], "little")
                or int(clock_lines[3]) != int.from_bytes(clock_bytes[4:], "little")):
            raise ValueError("General000 clock lines do not match the eight-byte game-clock save")
        non_records["SaveGame0/General000.sav"]["status"] = "source_verified_fields_restoration_partial"
        non_records["SaveGame0/General000.sav"]["field_meanings"] = [
            "current level identifier",
            "save display title",
            "game-clock low 32-bit milliseconds",
            "game-clock high 32-bit milliseconds",
        ]
        non_records["SaveGame0/General000.sav"]["source_evidence"] = [
            "reference/decompilation/functions/00412920_FUN_00412920.c:94-100",
            "reference/decompilation/functions/00431400_FUN_00431400.c:19-26",
            "output/savegame-semantic-map/save-timer-followup.md",
        ]

    magic_selection = entries["0:5:4986216"]
    if any(item["kind"] != "0x02" or item["value_size"] != 2
           for item in magic_selection["file_occurrences"]):
        raise ValueError("magic selection record changed width or type")
    magic_selection["partial_semantics"] = {
        "known_bytes": "low byte at +0x28 is magic selection row/index 0..2",
        "unknown_bytes": "high byte at +0x29",
        "source_evidence": [
            "reference/decompilation/functions/00439830_FUN_00439830.c:14",
            "reference/decompilation/functions/00439db0_FUN_00439db0.c:12",
            "reference/decompilation/functions/00439df0_FUN_00439df0.c:12",
        ],
        "runtime_evidence": [
            "output/savegame-semantic-map/magic-slot-86.json",
            "output/savegame-semantic-map/magic-slot-86.md",
        ],
        "runtime_observation": "isolated fresh loads restored 0000, 0100 and 005a into the live word; native selected_spell was 0, 1 and 0 respectively in stable playable manoir",
        "runtime_limit": "the high byte's gameplay meaning and selection in other load modes remain unresolved",
    }
    if allocation_joins is not None:
        magic_default = entries["0:5:4986210"]
        join = magic_default["source_allocation_join"]
        candidate = join["candidates"][0]
        if (join["status"] != "single_bounded_allocation"
                or candidate["owner"] != "fix"
                or candidate["allocation_data_start"] != 4986176
                or candidate["allocation_end"] != 4986220
                or any(item["kind"] != "0x02" or item["flags"] != "0x0204"
                       or item["value_size"] != 2 for item in magic_default["file_occurrences"])):
            raise ValueError("Magick default points source allocation or save shape changed")
        magic_default["ownership_status"] = "source_object_and_field"
        magic_default["source_owner"] = "Hype Magick state"
        magic_default["source_field"] = "default_starting_points"
        magic_default["source_type"] = "u16 magic points"
        magic_default["source_rule"] = "hype-magic-default-starting-points-v1"
        magic_default["owner"] = magic_default["source_owner"]
        magic_default["field"] = magic_default["source_field"]
        magic_default["semantic"] = {
            "owner": magic_default["owner"],
            "field": magic_default["field"],
            "meaning": "configured starting magic points, capped by configured maximum and used to initialize current points",
            "source_rule": magic_default["source_rule"],
            "confidence": "source_verified",
            "source_evidence": [
                "reference/decompilation/functions/004398f0_FUN_004398f0.c:27",
                "reference/decompilation/functions/00439830_FUN_00439830.c:11",
                "reference/decompilation/symbols.tsv:48579",
                "output/savegame-semantic-map/magick-config-followup.md",
            ],
            "restoration_scope": "opposed 8/40 values both survived fresh first-slot loads into stable playable manor as the live +0x22 word; settled current/max mana remained the separately saved 30/50",
            "restoration_evidence": "output/savegame-semantic-map/magic-default-restore-84.md",
        }
        magic_default["meaning_status"] = "source_verified"
        magic_default["unresolved"] = [
            "fresh-process restoration order relative to MagicLevel initialization is unverified",
            "authored configuration value producing the supplied save value is not located",
        ]
    standard_bits = 0
    for entry in entries.values():
        if entry.get("source_rule") != "hype-standard-custom-bits-init-v1":
            continue
        if any(item["kind"] != "0x04" or item["value_size"] != 4
               for item in entry["file_occurrences"]):
            raise ValueError("StandardGame CustomBitsInit record changed width or type")
        entry["partial_semantics"] = {
            "source_field_name": "StandardGame.CustomBitsInit",
            "known_bits": [
                {"mask": "0x00000001", "meaning": "candidate target can be selected by the camera's character-target scan"},
                {"mask": "0x00000080", "meaning": "when set, one action path bypasses its segment-versus-world collision checks before moving the target; this bit is absent from all 176 supplied fixture values"},
            ],
            "unknown_bits": "remaining bit purposes and full gameplay effect have not been established",
            "source_evidence": [
                "reference/decompilation/functions/00438100_FUN_00438100.c:138",
                "reference/decompilation/functions/00438c30_FUN_00438c30.c:21",
                "reference/decompilation/functions/00528fa0_CAM_fn_vGetNextTargettableCharacter.c:45",
                "reference/decompilation/functions/0043bcd0_FUN_0043bcd0.c:94",
            ],
        }
        entry["unresolved"] = ["remaining CustomBitsInit bit meanings and restoration effects are unverified"]
        standard_bits += 1
    if allocation_joins is not None and standard_bits != 201:
        raise ValueError(f"expected 201 typed StandardGame bitfields, found {standard_bits}")
    neverback = [entry for entry in entries.values()
                 if entry.get("source_rule") == "hype-standard-neverback-activation-v1"]
    if allocation_joins is not None:
        if len(neverback) != 56:
            raise ValueError(f"expected 56 paired NeverBack state records, found {len(neverback)}")
        field_counts = Counter(entry["source_field"] for entry in neverback)
        if field_counts != Counter({"StandardGame.never_back_latch": 28,
                                    "StandardGame.active": 28}):
            raise ValueError(f"NeverBack state fields are not paired: {field_counts}")
        if entries["1:5:1205048"].get("source_rule") == "hype-standard-neverback-activation-v1":
            raise ValueError("unrelated four-byte allocation was classified as NeverBack state")
    for entry in neverback:
        if any(item["kind"] != "0x01" or item["flags"] != "0x0a0c"
               or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
               for item in entry["file_occurrences"]):
            raise ValueError("NeverBack state record changed shape or value domain")
    if allocation_joins is not None:
        transition_probe = entries["1:5:833347"]
        if (transition_probe.get("source_owner") != "level:/persos/world/0/129"
                or transition_probe.get("source_field") != "StandardGame.never_back_latch"
                or transition_probe.get("meaning_status") != "source_verified"):
            raise ValueError("named-level transition probe no longer targets the rooted NeverBack latch")
        transition_probe["persistence"]["observed_apply"] = {
            "player_startup": "opposed player-file values 0 and 1 were observed after first-slot state-3 load",
            "named_level_return": "opposed HYPE/manoir.sav values reinserted in an isolated copy after tele11 departure were observed after state-5 tele05 return",
            "outgoing_save": "state-6 departure wrote the then-current manor byte into HYPE/manoir.sav before switching level names",
            "limit": "the named-file test uses an explicit disk-file intervention; the unmodified run cannot distinguish apply from outgoing autosave",
            "source_evidence": [
                "reference/decompilation/functions/00404700_FUN_00404700.c:28-36",
                "reference/decompilation/functions/004312c0_FUN_004312c0.c:20-45",
                "output/savegame-semantic-map/named-level-mode5-probe-20260928/final-report.json",
            ],
        }
    if allocation_joins is not None:
        game_state = entries["1:5:1205048"]
        candidate = game_state["source_allocation_join"]["candidates"][0]
        if (game_state["source_allocation_join"]["status"] != "single_bounded_allocation"
                or candidate["owner"] != "level"
                or candidate["allocation_data_start"] != 1205048
                or candidate["allocation_end"] != 1205052
                or candidate["allocation_index"] != 10426
                or any(item["kind"] != "0x01" or item["flags"] != "0x0a0c"
                       or item["value_size"] != 1 for item in game_state["file_occurrences"])):
            raise ValueError("game-state mask source allocation or save shape changed")
        game_state["ownership_status"] = "source_object_and_field"
        game_state["source_owner"] = "Hype level game state"
        game_state["source_field"] = "sound_event_game_state_filter_mask"
        game_state["source_type"] = "u8 bit mask"
        game_state["source_rule"] = "hype-game-state-sound-filter-v1"
        game_state["owner"] = game_state["source_owner"]
        game_state["field"] = game_state["source_field"]
        game_state["semantic"] = {
            "owner": game_state["owner"],
            "field": game_state["field"],
            "meaning": "current game-state mask used to filter sector sound events; a matching bit or both zero admits an event",
            "source_rule": game_state["source_rule"],
            "confidence": "source_verified",
        }
        game_state["meaning_status"] = "source_verified"
        game_state["unresolved"] = [
            "individual bit labels, state writers and fresh-process restoration remain unverified"
        ]

    # The map registrar's four loops account for 100 setting records, ten
    # words, four epoch-enable words, and 60 runtime marker records. The Fix
    # allocation shapes and ordered reset values distinguish the first three
    # groups. Runtime marker rows remain structural candidates until captured.
    map_groups = Counter()
    for identity, entry in entries.items():
        if entry["family"] == "0:5/0x04/0x0205":
            join = entry["source_allocation_join"]
            if join["status"] != "single_bounded_allocation" or len(join["candidates"]) != 1:
                raise ValueError(f"Fix map registration lost its unique allocation: {identity}")
            candidate = join["candidates"][0]
            start = candidate["allocation_data_start"]
            end = candidate["allocation_end"]
            index = candidate["allocation_index"]
            if (candidate["owner"] != "fix" or start is None or end is None or index is None
                    or any(item["kind"] != "0x04" or item["flags"] != "0x0205"
                           or item["value_size"] != 4 for item in entry["file_occurrences"])):
                raise ValueError("Fix map state lost its bounded allocation")
            block, module, offset_text = identity.split(":")
            if (block, module) != ("0", "5"):
                raise ValueError(f"Fix map registration changed source identity: {identity}")
            offset = int(offset_text)
            relative = offset - start
            size = end - start
            if (size == 72 and relative == 48 and 27451 <= index <= 27550
                    and start == 4986412 + 76 * (index - 27451)
                    and offset + 4 <= end):
                group = "map_setting_entry_plus_0x30"
                ordinal = index - 27451
            elif (size == 40 and index == 27449 and start == 4986348
                  and relative in range(0, 40, 4) and offset + 4 <= end):
                group = "map_ten_word_state"
                ordinal = relative // 4
            elif size == 16 and index == 27450 and relative in range(0, 16, 4):
                group = "map_epoch_selectable"
                ordinal = relative // 4
            else:
                raise ValueError(f"unmatched Fix map registration shape: {identity}")
            entry["registration_group"] = {
                "name": group,
                "ordinal": ordinal,
                "evidence": "004164d0 registration loops and 00416800 allocation/reset order",
                "binding": "source-informed structural match; no live registry address confirmation",
            }
            map_groups[group] += 1
            if group == "map_setting_entry_plus_0x30":
                entry["ownership_status"] = "source_object_and_field"
                entry["source_owner"] = "Hype map UI runtime entry table"
                entry["source_field"] = f"entry[{ordinal}]+0x30"
                entry["source_type"] = "four-byte integer parsed with %d"
                entry["source_rule"] = "hype-map-setting-slot-v1"
                entry["map_slot_role"] = (
                    "reserved_special_map_ui_record" if ordinal == 0
                    else "setting_parser_slot_candidate"
                )
                entry["unresolved"] = [
                    "the authored setting row, gameplay purpose, value domain and restoration behavior remain unknown"
                ]
            elif group == "map_ten_word_state":
                if any(item["value_hex"] != "feffffff" for item in entry["file_occurrences"]):
                    raise ValueError("baseline map state array differs from source reset pattern")
                entry["ownership_status"] = "source_object_and_field"
                entry["source_owner"] = "Hype map UI runtime state array DAT_00750ac4"
                entry["source_field"] = f"word[{ordinal}]"
                entry["source_type"] = "raw four-byte word; signedness unknown"
                entry["source_rule"] = "hype-map-state-array-slot-v1"
                entry["unresolved"] = [
                    "the slot's gameplay role, value domain and restoration behavior remain unknown"
                ]
            if group == "map_epoch_selectable":
                expected = "01000000" if ordinal == 0 else "00000000"
                if any(item["value_hex"] != expected for item in entry["file_occurrences"]):
                    raise ValueError("baseline map epoch flags differ from source reset pattern")
                entry["owner"] = "Hype map UI"
                entry["field"] = f"selectable_epoch_slot[{ordinal}]"
                entry["semantic"] = {
                    "owner": entry["owner"],
                    "field": entry["field"],
                    "meaning": "nonzero lets map navigation select this epoch slot; zero makes cycling skip it",
                    "source_rule": "hype-map-epoch-selectable-v1",
                    "confidence": "source_verified",
                    "source_evidence": [
                        "reference/decompilation/functions/004164d0_FUN_004164d0.c:24",
                        "reference/decompilation/functions/00416800_FUN_00416800.c:40",
                        "reference/decompilation/functions/00417030_FUN_00417030.c:14",
                        "reference/decompilation/functions/00416e70_FUN_00416e70.c:24",
                    ],
                }
                entry["meaning_status"] = "source_verified"
                entry["unresolved"] = [
                    "live restoration after later map initialization has not been separately checked"
                ]
        elif entry["family"] == "1:10/0x04/0x0205":
            offset = int(identity.split(":")[2])
            if offset < 1248 or (offset - 1248) % 64 != 0:
                raise ValueError(f"unexpected runtime map marker stride: {identity}")
            entry["registration_group"] = {
                "name": "map_star_entry_plus_0x38_candidate",
                "ordinal": (offset - 1248) // 64,
                "evidence": "004164d0 fourth loop and 00416800 sixty 0x3c-byte allocations",
                "binding": "inferred from count/stride; individual runtime addresses unverified",
            }
            map_groups["map_star_entry_plus_0x38_candidate"] += 1
    if allocation_joins is not None and map_groups != {
        "map_setting_entry_plus_0x30": 100,
        "map_ten_word_state": 10,
        "map_epoch_selectable": 4,
        "map_star_entry_plus_0x38_candidate": 60,
    }:
        raise ValueError(f"map registrar group counts differ: {dict(map_groups)}")
    star_ids = {
        identity for identity, entry in entries.items()
        if entry.get("registration_group", {}).get("name") == "map_star_entry_plus_0x38_candidate"
    }
    if allocation_joins is not None and star_ids != {
        f"1:10:{1248 + 64 * ordinal}" for ordinal in range(60)
    }:
        raise ValueError("map star-entry source identities differ from the 60-slot registrar")

    # Orrery decodes kind 0x11 as a block/module/offset reference. Preserve
    # each occurrence's target independently: player and named-level files can
    # hold different edges for the same registration identity. Membership in
    # another saved record is structural evidence, not a target's game meaning.
    reference_target_groups = Counter()
    reference_occurrences = 0
    reference_targets_with_saved_record = 0
    for entry in entries.values():
        targets = []
        for occurrence in entry["file_occurrences"]:
            if occurrence["kind"] != "0x11":
                continue
            target = occurrence["decoded_value"]
            if not isinstance(target, dict) or set(target) != {"block", "module", "offset"}:
                raise ValueError("Orrery reference target is not a decoded block/module/offset")
            target_id = f"{target['block']}:{target['module']}:{target['offset']}"
            exists = target_id in entries
            targets.append({
                "file": occurrence["file"],
                "target_identity": target_id,
                "target_has_saved_record": exists,
                "target_source_owner": entries[target_id].get("source_owner") if exists else None,
                "target_source_field": entries[target_id].get("source_field") if exists else None,
            })
            reference_occurrences += 1
            reference_targets_with_saved_record += exists
            reference_target_groups[f"{target['block']}:{target['module']}"] += 1
        if targets:
            entry["reference_targets"] = targets
    for entry in entries.values():
        if (entry["meaning_status"] == "unknown"
                and entry.get("ownership_status") == "source_object_and_field"
                and entry["unresolved"] == ["source owner, field meaning and restoration behavior not yet joined"]):
            entry["unresolved"] = [
                "source owner and field are joined; gameplay meaning, value domain and restoration behavior remain unresolved",
            ]

    for family_info in families.values():
        family_info["explained"] = 0
    for entry in entries.values():
        families[entry["family"]]["explained"] += entry["meaning_status"] != "unknown"
    player = inspection["files"]["SaveGame0/Level000.sav"]["records"]
    level = inspection["files"]["SaveGame0/manoir.sav"]["records"]
    if len(player) != 1271 or len(level) != 263 or not set(level) <= set(player):
        raise ValueError("fixture occurrence and overlap counts differ from measured baseline")
    arrays = {identity: entry for identity, entry in entries.items() if any(
        item["kind"] == "0x10" for item in entry["file_occurrences"]
    )}
    coverage = {
        "schema": 1,
        "profile": PROFILE,
        "save_file_occurrences": sum(len(items) for items in records.values()),
        "distinct_contextual_record_identities": len(entries),
        "player_record_occurrences": len(player),
        "named_level_record_occurrences": len(level),
        "named_level_identities_overlapping_player": len(set(level) & set(player)),
        "runtime_corroborated_identities": sum(entry["meaning_status"] == "runtime_corroborated" for entry in entries.values()),
        "source_verified_only_identities": sum(entry["meaning_status"] == "source_verified" for entry in entries.values()),
        "inferred_identities": 0,
        "unknown_identities": sum(entry["meaning_status"] == "unknown" for entry in entries.values()),
        "partially_explained_unknown_identities": sum(
            entry["meaning_status"] == "unknown" and "partial_semantics" in entry
            for entry in entries.values()
        ),
        "registration_group_counts": dict(sorted(map_groups.items())),
        "reference_occurrences": reference_occurrences,
        "reference_targets_with_saved_record": reference_targets_with_saved_record,
        "reference_target_groups": dict(sorted(reference_target_groups.items())),
        "source_object_and_field_identities": sum(entry.get("ownership_status") == "source_object_and_field" or entry["meaning_status"] == "runtime_corroborated" for entry in entries.values()),
        "allocation_join_statuses": dict(sorted(allocation_statuses.items())),
        "array_identity_count": len(arrays),
        "array_payload_bytes": sum(max(item["value_size"] for item in entry["file_occurrences"]) for entry in arrays.values()),
        "array_contents_explained_identities": sum(entry["meaning_status"] != "unknown" for entry in arrays.values()),
        "families": dict(sorted(families.items())),
        "metadata_files_outside_sai": non_records,
    }
    return {
        "schema": 1,
        "profile": PROFILE,
        "source_family_rules": {
            "hype-inventory-handle-selected-v1": {
                "applies_when": "one bounded 24-byte Fix block 0:10 handle allocation resolves a typed item and record +8 is kind 0x01, flags 0x0204",
                "meaning": "INV_fn_bIsSelected reads the handle's +8 Boolean; item availability is a separate +0x19 member byte",
                "source_evidence": [
                    "reference/decompilation/functions/00417fe0_FUN_00417fe0.c:93",
                    "reference/decompilation/functions/004a1920_INV_fn_bIsSelected.c:12",
                    "src/openspace/level/save_join.rs",
                ],
                "runtime_scope": "Ice 54 and Lightning 74 separately corroborated by normal save and fresh reload",
            },
            "hype-dsg-descriptor-v1": {
                "applies_when": "one saved Perso designer descriptor resolves its current buffer address and exact extent/flags to one record in this level context",
                "ownership": "actor source path, DSG index and declared designer type; descriptor definition may be shared but current memory belongs to actor instance",
                "meaning_limit": "DSG[index] and designer type establish ownership/storage, not the field's gameplay purpose or array element meaning",
                "source_evidence": [
                    "src/openspace/level/records.rs:693",
                    "src/openspace/level/ai.rs:533",
                    "src/openspace/level/save_join.rs",
                    "reference/raymap/Assets/Scripts/OpenSpace/Object/Perso.cs",
                ],
                "runtime_scope": "only iDummy_PourCercles DSG[8] separately corroborated by lever save/reload",
            },
            "hype-inventory-main-handle-number-v1": {
                "applies_when": "a typed Fix handle+0x0a two-byte record has a same-owner selected Boolean record at handle+8; only the main inventory list registers both fields",
                "meaning": "signed instantiated item number/quantity accessed through INV_fn_wInstanciatedItemGetNumber; a separate resource root registers +0x0a without +8",
                "source_evidence": [
                    "reference/decompilation/functions/00417fe0_FUN_00417fe0.c:93",
                    "reference/decompilation/functions/004a3ba0_INV_fn_wInstanciatedItemGetNumber.c:12",
                    "reference/decompilation/functions/0041d930_FUN_0041d930.c:37",
                ],
                "runtime_scope": "62 typed handle records in the supplied manor slot; individual number changes and fresh reload unverified",
            },
            "hype-inventory-special-resource-balance-v1": {
                "applies_when": "the separately rooted Fix inventory handle at block 0:10 allocation [5780,5804) has a two-byte +0x0a save, and its linked item at [5808,5840) has the +0x14 maximum-capacity save",
                "meaning": "current resource amount displayed in stats and added or spent by inventory effects, with additions clamped to the linked item's maximum",
                "meaning_limit": "the authored item/resource name and fresh-process restoration are not established",
                "source_evidence": [
                    "reference/decompilation/functions/00417fe0_FUN_00417fe0.c",
                    "reference/decompilation/functions/0041d400_FUN_0041d400.c",
                    "reference/decompilation/functions/0041d420_FUN_0041d420.c",
                    "reference/decompilation/functions/0041d490_FUN_0041d490.c",
                    "reference/decompilation/functions/0040c950_FUN_0040c950.c",
                    "output/savegame-semantic-map/special-inventory-word-followup.md",
                ],
                "runtime_scope": "one saved resource balance in the supplied manor slot; 20 in the fixture and a linked maximum of 200",
            },
            "hype-inventory-item-max-capacity-v1": {
                "applies_when": "one bounded 32-byte Fix block 0:10 item allocation is reached by a rooted 24-byte handle, and its +0x14 two-byte record has kind 0x02 and flags 0x0204",
                "meaning": "16-bit maximum element capacity used to clamp inventory item additions",
                "meaning_limit": "62 items have source names; one uniquely rooted item's authored resource name is unavailable",
                "source_evidence": [
                    "reference/decompilation/functions/00417fe0_FUN_00417fe0.c",
                    "reference/decompilation/functions/004a19f0_INV_fn_wGetMaxElement.c",
                    "reference/decompilation/functions/004a1e00_INV_fn_wAddElementToItem.c",
                    "src/openspace/level/save_join.rs",
                    "output/savegame-semantic-map/inventory-max-capacity-evidence.md",
                ],
                "runtime_scope": "63 rooted item allocations in the supplied manor slot; individual capacity changes and fresh reload unverified",
            },
            "hype-standard-hitpoints-v1": {
                "applies_when": "a rooted actor's 56-byte StandardGame allocation has return policy 4 and exact one-byte saves at +0x18/+0x19/+0x1a with kind 0x01 and flags 0x0204",
                "meaning": "HitPointsInit is current HP, HitPointsMaxInit is the mutable current maximum, and HitPointsMaxMax caps that maximum",
                "meaning_limit": "the supplied slot's individual gameplay triggers and fresh-process restoration are unverified",
                "source_evidence": [
                    "reference/decompilation/functions/00438100_FUN_00438100.c:89",
                    "reference/decompilation/functions/00438c30_FUN_00438c30.c:13",
                    "reference/decompilation/functions/00460bd0_FUN_00460bd0.c",
                    "src/openspace/level/records.rs",
                    "src/openspace/level/save_join.rs",
                    "output/savegame-semantic-map/standard-hitpoints-followup.md",
                ],
                "runtime_scope": "84 bytes in 28 rooted actor allocations (26 Level, two Fix), 28 records per field",
            },
            "hype-gauge-pointer-chain-v1": {
                "applies_when": "the unique Fix block 0:5 relocation chain contains three exact 40-byte gauge roots and 17 exact 24-byte child allocations, with 7/5/5 children, matching one-byte kind 0x01 flags 0x0204 saves",
                "meaning": "child visual phase and progress tick advance against root thresholds; each root counts completed children",
                "meaning_limit": "37 fields have complete source-backed roles; the three root threshold selectors are typed and structurally located but lack authored purpose and legal values",
                "source_evidence": [
                    "reference/decompilation/functions/0043a8a0_FUN_0043a8a0.c",
                    "reference/decompilation/functions/0043aa70_FUN_0043aa70.c",
                    "reference/decompilation/functions/0043ab40_FUN_0043ab40.c",
                    "src/openspace/level/records.rs",
                    "src/openspace/level/save_join.rs",
                    "output/savegame-semantic-map/gauge-pointer-chain-followup.md",
                ],
                "runtime_scope": "all 40 saved gauge fields in the supplied manor slot; authored panel/item names and independent reload behavior unverified",
            },
            "hype-gauge-root-link-v1": {
                "applies_when": "a kind-0x11, flags-0x0204 reference save is at the start of a bounded 40-byte gauge root in the unique relocated three-root chain, and its target equals the root's declared next pointer",
                "meaning": "link from the first saved gauge panel root to the next root in source list order",
                "meaning_limit": "the authored panel names and fresh-process restoration are unverified",
                "source_evidence": [
                    "reference/decompilation/functions/0043a8a0_FUN_0043a8a0.c:13",
                    "src/openspace/level/save_join.rs",
                    "output/savegame-semantic-map/gauge-pointer-chain-followup.md",
                ],
                "runtime_scope": "one first-root link is present in the supplied manor save; the join also checks a second-root source query and rejects wrong targets",
            },
            "hype-magic-default-starting-points-v1": {
                "applies_when": "the bounded Hype Magick state allocation in Fix block 0:5 has a two-byte current-value save at +0x22",
                "meaning": "DefaultMagicPoints configuration is capped at maximum and initializes current magic points",
                "meaning_limit": "the supplied first-slot load restored opposed 8/40 defaults to the live +0x22 word while current/max mana stayed 30/50; callback timing and other save routes remain unobserved",
                "source_evidence": [
                    "reference/decompilation/functions/004398f0_FUN_004398f0.c:27",
                    "reference/decompilation/functions/00439830_FUN_00439830.c:11",
                    "reference/decompilation/symbols.tsv:48579",
                    "output/savegame-semantic-map/magick-config-followup.md",
                ],
                "runtime_evidence": "output/savegame-semantic-map/magic-default-restore-84.md",
            },
            "hype-manor-lever-phase-v1": {
                "applies_when": "the named manor lever actor level:/persos/world/0/174 has saved Boolean DSG[1] at 1:17:187072",
                "meaning": "AI behavior 7/8 alternates paired actions and poses using this Boolean phase",
                "meaning_limit": "physical lever direction and passage open/closed relation not established",
                "source_evidence": [
                    "level:/ai/models/1396 Normal behaviors 2, 7 and 8",
                    "output/savegame-semantic-map/manoir-lever-ai-model.json",
                ],
            },
            "hype-mannequin-stage-code-v1": {
                "applies_when": "three named manor mannequins share AI model 939 and each has a one-byte saved DSG[0] joined to its own current designer buffer",
                "meaning": "stage code: world assigns 1/2/3 to sword/crossbow/magic actors; code 4 follows a matching MainActor condition plus DSG[1] permission and is read by MasterYoda; Magie can also receive code 5",
                "meaning_limit": "broader authored sequence or animation labels and fresh-process restoration are unverified",
                "source_evidence": [
                    "level:/ai/models/154 Normal behavior 2 schedule statements 10-13",
                    "level:/ai/models/939 Reflex behavior 0 rule 0",
                    "level:/ai/models/991 Normal behavior 4 schedule statements 0-2",
                    "output/savegame-semantic-map/mannequin-dsg-followup.md",
                ],
            },
            "hype-mannequin-stage-permission-v1": {
                "applies_when": "the same three mannequin actors each have a one-byte saved Boolean DSG[1] joined to their own current designer buffer",
                "meaning": "Boolean permission for changing that mannequin's stage code to 4 after its slot/MainActor match; world slot assignment clears it and the later Magie branch can set it",
                "meaning_limit": "broader authored purpose and fresh-process restoration are unverified",
                "source_evidence": [
                    "level:/ai/models/939 Reflex behavior 0 rule 0",
                    "level:/ai/models/154 Normal behavior 2 schedule statements 10-13",
                    "output/savegame-semantic-map/mannequin-dsg-followup.md",
                ],
            },
            "hype-mannequin-third-field-writes-v1": {
                "applies_when": "the same three mannequin actors each have a one-byte saved Boolean DSG[2] joined to their own current designer buffer",
                "known_writes": "mannequin model sets 1 after a three-way match, while the world also sets 1 in a separate Magie branch and clears it on slot assignments",
                "meaning_limit": "no verified reader or distinct gameplay purpose; all three complete-record meanings remain unknown",
                "source_evidence": [
                    "level:/ai/models/939 Reflex behavior 0 rule 0",
                    "level:/ai/models/154 Normal behavior 2 schedule statements 10-13",
                    "output/savegame-semantic-map/mannequin-dsg-followup.md",
                ],
            },
            "hype-global-mannequin-match-gate-v1": {
                "applies_when": "the canonical Fix Global DSG[152] is the exact one-byte Boolean record 0:17:4557 and the three rooted manor mannequins share model 939",
                "known_role": "zero admits the mannequin slot-match Reflex path; IWorld selector branches 2-4 clear it, branches 5-6 set it, and Yoda behavior 18 also sets it",
                "meaning_limit": "a complete mannequin stage/unlock meaning and fresh-process restoration are not established",
                "source_evidence": [
                    "level:/ai/models/939 Reflex behavior 0 rule 0",
                    "level:/ai/models/154 Normal behavior 2 schedule statements 10-14",
                    "level:/ai/models/991 Normal behavior 18 schedule statement 2",
                    "output/savegame-semantic-map/global-dsg152-followup.md",
                ],
            },
            "hype-global-camera-look-gate-v1": {
                "applies_when": "canonical Fix Global DSG[18] resolves to the one-byte Boolean record 0:17:4701",
                "meaning": "0 permits BT_Camera_Look entry if the button is pressed and Global DSG[17] is 1; 1 blocks entry and causes the active camera-look behavior to exit",
                "meaning_limit": "fresh-process restoration and any dynamic/native consumers beyond the typed AI model are unverified",
                "source_evidence": [
                    "fix:/ai/models/437 Normal behaviors 0, 3 and 8",
                    "fix:/ai/models/28 Reflex behaviors 1, 2, 3, 4 and 6",
                    "output/savegame-semantic-map/global-dsg18-followup.md",
                ],
            },
            "hype-global-camera-look-allow-v1": {
                "applies_when": "canonical Fix Global DSG[17] resolves to the one-byte Boolean record 0:17:4702",
                "meaning": "1 permits BT_Camera_Look entry if pressed and DSG[18] is 0; 0 prevents entry and causes active look behavior to exit",
                "meaning_limit": "Hype-local write guards, fresh-process restoration, and dynamic/native consumers are not fully resolved",
                "source_evidence": [
                    "fix:/ai/models/437 Normal behaviors 0, 3 and 8",
                    "fix:/ai/models/28 Normal behavior 24 and Reflex behaviors 1, 2, 3, 4 and 6",
                    "output/savegame-semantic-map/global-dsg17-followup.md",
                ],
            },
            "hype-global-checkpoint-sighting-v1": {
                "applies_when": "canonical Fix Global DSG[30] resolves to the 12-byte designer vector record 0:17:4396",
                "meaning": "checkpoint copies Hype SelfSighting into this field; Hype later feeds negated X/Y and zero Z into ChangeSighting",
                "meaning_limit": "the native orientation result, wider consumers and fresh-process restoration are not proven",
                "source_evidence": [
                    "level:/ai/models/348 Normal behavior 0 schedule statement 0",
                    "fix:/ai/models/28 Normal behavior 0 schedule statement 207",
                    "output/savegame-semantic-map/global-dsg30-followup.md",
                ],
            },
            "hype-manoir-travel-preset-v1": {
                "applies_when": "canonical Fix Global DSG[145] resolves to the one-byte Boolean record 0:17:4564",
                "meaning": "1 selects four manor time-travel position vectors 15-18; 0 selects vectors 11-14",
                "meaning_limit": "authored location names, further native consumers and fresh-process restoration are unverified",
                "source_evidence": [
                    "level:/ai/models/154 Normal behavior 2 schedule statement 6",
                    "level:/ai/models/586 Normal behavior 0 schedule statement 1",
                    "level:/ai/models/638 Normal behavior 4 schedule statement 4",
                    "output/savegame-semantic-map/global-dsg145-followup.md",
                ],
            },
            "hype-global-manoir-camera-signal-v1": {
                "applies_when": "canonical Fix Global DSG[146] is the one-byte Boolean record 0:17:4563 and both rooted manor fixed-camera actors use model 397",
                "known_role": "the camera model sets one on branches requesting camera state 3, clears on reset/transition branches, and schedules behavior 9 when the flag equals one",
                "meaning_limit": "the visual camera effect, arbitration between two actors and fresh-process restoration are unresolved",
                "source_evidence": [
                    "level:/ai/models/397 Normal behaviors 0, 1, 19 and 21",
                    "level:/ai/models/154 Normal behavior 2 schedule statement 20",
                    "output/savegame-semantic-map/global-dsg146-followup.md",
                ],
            },
            "hype-global-manoir-time-gate-v1": {
                "applies_when": "canonical Fix Global DSG[39] is the exact one-byte Boolean record 0:17:4676 and the IWorld/time-travel models have the actor-qualified sites",
                "known_role": "IWorld selector 6 sets one, later branches on one/zero paired with Global DSG[0] values 3/2 and requests Gogood-return behavior changes; iDummy_TimeTravel also reads both polarities",
                "meaning_limit": "unsupported guarded effects and lifecycle do not establish a specific time-travel permission or completion state",
                "source_evidence": [
                    "level:/ai/models/154 Normal behavior 2 schedule statements 14-15",
                    "level:/ai/models/586 Normal behaviors 0 and 1",
                    "output/savegame-semantic-map/global-dsg39-followup.md",
                    "output/savegame-semantic-map/shared-change-comport-followup.md",
                ],
            },
            "hype-global-channel72-condition-v1": {
                "applies_when": "canonical Fix Global DSG[0] resolves to the one-byte saved record 0:17:4721",
                "known_condition": "Hype sets channel 72 active when DSG[0] equals 1, otherwise inactive",
                "meaning_limit": "channel 72 has no verified authored target or observable gameplay purpose; the saved record remains unknown",
                "source_evidence": [
                    "fix:/ai/models/28 Normal behavior 0 schedule statement 192",
                    "output/savegame-semantic-map/global-dsg0-followup.md",
                ],
            },
            "hype-global-authored-effect-gates-v1": {
                "applies_when": "canonical Fix Global DSG[4] and DSG[37] retain their exact one-byte Boolean save identities and actor-qualified authored AI sites",
                "known_role": "DSG[4] gates Hype/statue object-table swap calls and Hype material-diffuse calls; DSG[37] gates Hype's ChangeSighting call using checkpoint-saved DSG[30]",
                "meaning_limit": "the full authored gameplay states and fresh-process restoration are unresolved, so both complete-record meanings remain unknown",
                "source_evidence": [
                    "output/savegame-semantic-map/global-dsg4-followup.md",
                    "output/savegame-semantic-map/global-dsg37-followup.md",
                ],
            },
            "hype-global-hype-transition-flags-v1": {
                "applies_when": "canonical Fix Global DSG[1] and DSG[2] retain exact one-byte Boolean record identities and typed Hype/statue AI sites",
                "known_role": "DSG[1] controls Hype channel 73 and gates weapon-input branches; DSG[2] is set/cleared in Hype transition behaviors, gates channels 29/16 and Armor HUD show branches",
                "meaning_limit": "authored channel targets, exact gameplay state and fresh-process restoration are unresolved; both complete meanings remain unknown",
                "source_evidence": [
                    "output/savegame-semantic-map/global-dsg1-followup.md",
                    "output/savegame-semantic-map/global-dsg2-followup.md",
                ],
            },
            "hype-global-object-template-latches-v1": {
                "applies_when": "each of nine exact Global DSG[220..228] IDs retains its individual one-byte Boolean source join and the corresponding object-template AI model reads it as zero and assigns one",
                "known_role": "per-object clear-to-set Boolean latch; six fields also have exact level-model read/write sites",
                "meaning_limit": "unsupported enclosing AI and unresolved MainActor DSG[97] prevent a specific pickup, activation or quest-event name; all nine complete meanings remain unknown",
                "source_evidence": [
                    "output/savegame-semantic-map/global-object-cluster-followup.md",
                    "output/savegame-semantic-map/global-inverse-ai-index.json",
                ],
            },
            "hype-global-manoir-event-gates-v1": {
                "applies_when": "canonical Fix Global DSG[147] and DSG[149] retain their exact one-byte Boolean save identities and typed Manoir AI sites",
                "known_role": "DSG[147] gates a statue KillSelf branch after an iGogood action; DSG[149] gates a grille behavior sequence and has IWorld set/clear paths",
                "meaning_limit": "unsupported enclosing effects and the save-test actor's snapshot/restore/force-one paths prevent a specific progression or bridge-state meaning",
                "source_evidence": [
                    "output/savegame-semantic-map/global-manoir-147-149-followup.md",
                ],
            },
            "hype-global-conditional-branches-v1": {
                "applies_when": "each exact Fix Global DSG[38,40,41,42,43,44,45,66,219,233] identity retains its individually checked one-byte Boolean save shape and actor-qualified source site",
                "known_role": "the individual field's guarded actor behavior, object deactivation, input branch or self-removal condition is recorded in its catalogue entry",
                "meaning_limit": "unsupported operations and incomplete writer/restoration lifecycles leave all ten complete gameplay-state meanings unknown",
                "source_evidence": [
                    "output/savegame-semantic-map/global-dsg38-40-42-44-45-followup.md",
                    "output/savegame-semantic-map/shared-pour-cercles-followup.md",
                    "output/savegame-semantic-map/rete-ast-recovery-followup.md",
                    "output/savegame-semantic-map/global-dsg41-43-followup.md",
                    "output/savegame-semantic-map/shared-time-selector-followup.md",
                    "output/savegame-semantic-map/global-dsg66-followup.md",
                    "output/savegame-semantic-map/shared-change-comport-followup.md",
                    "output/savegame-semantic-map/global-dsg219-233-followup.md",
                ],
            },
            "hype-global-configuration-tier-v1": {
                "applies_when": "canonical Fix Global DSG[73] is the exact one-byte designer type 2 record 0:17:4639 with the five ordered circle-dummy writes and separate Hype/IWorld sites",
                "known_role": "mutable unsigned configuration tier; source writes 0..4, Hype checks ==4 and IWorld can clear it",
                "meaning_limit": "the underlying authored configuration, full lifecycle and restored gameplay effect remain unknown",
                "source_evidence": [
                    "output/savegame-semantic-map/shared-pour-cercles-followup.md",
                    "output/savegame-semantic-map/global-dsg73-tier-followup.md",
                ],
            },
            "hype-global-checkpoint-vector-writes-v1": {
                "applies_when": "canonical Fix Global DSG[29] and DSG[31] are the exact 12-byte vector saves 0:17:4384 and 0:17:4408 and the rooted manor checkpoint's typed assignments target them",
                "known_role": "the checkpoint writes Hype SelfPosition to DSG[29] and its own Variable[1] to DSG[31]",
                "meaning_limit": "no ordinary gameplay reader uses DSG[29], while DSG[31]'s producer has a vector/Boolean source-type disagreement; both complete meanings and restoration effects remain unknown",
                "source_evidence": [
                    "level:/ai/models/348 Normal behavior 0 schedule statement 0",
                    "output/savegame-semantic-map/global-dsg29-31-followup.md",
                ],
            },
            "hype-global-checkpoint-perso-reference-v1": {
                "applies_when": "Global DSG[32] is the exact kind-0x11 Perso reference 0:17:4420 and its bounded kind-4 Level SuperObject target roots to the manor checkpoint actor",
                "known_role": "checkpoint AI assigns MainActor or Me to this field; the supplied reference targets the checkpoint actor",
                "meaning_limit": "the branch condition, gameplay consumer and fresh-process restoration are unresolved",
                "source_evidence": [
                    "level:/ai/models/348 Normal behavior 0 schedule statement 0",
                    "output/savegame-semantic-map/global-dsg32-checkpoint-target.md",
                ],
            },
            "hype-game-clock-v1": {
                "applies_when": "the first bounded eight-byte Fix module 5, block 0 allocation at offset 4 is saved as kind 0x10, flags 0x0204, length eight",
                "meaning": "millisecond game timer snapshot sampled before save and used to restore the running timer forward after player-mode load",
                "meaning_limit": "pause/menu inclusion, nonzero high-word wrap, and fresh-process display behavior are unobserved",
                "source_evidence": [
                    "reference/decompilation/functions/00401e00_FUN_00401e00.c:32-33",
                    "reference/decompilation/functions/00431130_FUN_00431130.c:20",
                    "reference/decompilation/functions/004261a0_FUN_004261a0.c:13-16",
                    "reference/decompilation/functions/00502680_FUN_00502680.c:13-16",
                    "output/savegame-semantic-map/save-timer-followup.md",
                ],
            },
            "hype-post-load-position-snapshots-v1": {
                "applies_when": "two consecutive exact 36-byte player-file saves at Level 2:6 offsets 1463692 and 1464196 match the two FUN_004c6340 registrations from DAT_00610534 and DAT_00610538",
                "known_role": "the first is Hype's main-actor position prefix; the second is the first qualifying scene Perso's position prefix, with StdCamer as the source-order candidate; bit 7 selects uncompressed coordinates at +4/+8/+12",
                "meaning_limit": "the second owner's collision-entry predicate and the remaining bytes of both truncated POS matrices are unresolved; both complete-record meanings remain unknown",
                "source_evidence": [
                    "reference/decompilation/functions/0040f4c0_FUN_0040f4c0.c:13-22",
                    "reference/decompilation/functions/004390e0_FUN_004390e0.c:17-39",
                    "output/savegame-semantic-map/position-snapshot-followup.md",
                    "output/savegame-semantic-map/position-owner-followup.md",
                ],
            },
            "hype-toolcam-post-load-word-v1": {
                "applies_when": "exact Fix record 0:5:4986280 is a two-byte kind-0x02 current-memory save at the start of a four-byte source allocation registered from DAT_00752414",
                "known_role": "low 16-bit snapshot of a ToolCam slot-1 nested object's +0x98 dword, read before save and written after player load",
                "meaning_limit": "declared member name and meanings of its state-like values remain unknown",
                "source_evidence": [
                    "reference/decompilation/functions/0040f4c0_FUN_0040f4c0.c:21",
                    "reference/decompilation/functions/00431130_FUN_00431130.c:15-21",
                    "reference/decompilation/functions/004390e0_FUN_004390e0.c:30-36",
                    "output/savegame-semantic-map/post-load-word-followup.md",
                ],
            },
            "hype-standard-custom-bits-init-v1": {
                "applies_when": "a rooted, AI-referenced, or reverse-pointer-validated source Perso has an exact StandardGame pointer to a bounded 56-byte source allocation with a four-byte current-value save at +0x2c",
                "ownership": "actor's StandardGame.CustomBitsInit field; 123 level and 78 Fix records in the manor fixture, including 24 AI-referenced Fix templates and one additional source-Perso reverse-pointer owner",
                "meaning_limit": "bit 0 participates in camera character target selection and bit 7 bypasses collision checks in one action path, but these consumers are not proved active for every template; other bit roles and restoration effects remain unresolved, so all 201 records remain semantically unknown",
                "source_evidence": [
                    "src/openspace/level/records.rs",
                    "reference/decompilation/functions/00438100_FUN_00438100.c:138",
                    "reference/decompilation/functions/00438c30_FUN_00438c30.c:21",
                    "reference/decompilation/functions/00528fa0_CAM_fn_vGetNextTargettableCharacter.c:45",
                    "reference/decompilation/functions/0043bcd0_FUN_0043bcd0.c:94",
                ],
            },
            "hype-standard-neverback-activation-v1": {
                "applies_when": "a rooted actor's bounded 56-byte StandardGame has second InitFlags return policy NeverBack (+0x24 == 4), with two one-byte current-value saves at +0x1b and +0x1c",
                "meaning": "+0x1b suppresses the actor's return when nonzero; +0x1c is its active state, forced to zero by that suppression latch",
                "meaning_limit": "the gameplay cause and fresh-process restoration for each individual actor have not been separately observed",
                "source_evidence": [
                    "reference/decompilation/functions/00438100_FUN_00438100.c",
                    "reference/decompilation/functions/00438c30_FUN_00438c30.c",
                    "reference/decompilation/functions/00438f20_FUN_00438f20.c",
                    "reference/decompilation/functions/00438f40_FUN_00438f40.c",
                    "reference/decompilation/functions/00438f60_FUN_00438f60.c",
                    "reference/decompilation/functions/00437dc0_FUN_00437dc0.c",
                    "reference/decompilation/functions/00528ed0_CAM_fn_vValidateIfTargetIsStillActive.c",
                    "src/openspace/level/save_join.rs",
                ],
            },
            "hype-game-state-sound-filter-v1": {
                "applies_when": "the single 1:5:1205048 one-byte 0x0a0c save is bounded at the start of level allocation 10426 (four padded bytes), matching the unique 004483a0 one-byte registration",
                "meaning": "current game-state bit mask used to filter sector sound events; a bitwise intersection or both values zero admits an event",
                "meaning_limit": "individual bit labels, state writers and fresh-process restoration remain unverified; map-era switches corroborate but do not exhaust the byte's role",
                "source_evidence": [
                    "reference/decompilation/functions/004483a0_FUN_004483a0.c:13",
                    "reference/decompilation/functions/00448380_FUN_00448380.c:11",
                    "reference/decompilation/functions/004480e0_FUN_004480e0.c:17",
                    "reference/decompilation/functions/00416e70_FUN_00416e70.c:24",
                    "output/savegame-semantic-map/epoch-byte-check.md",
                ],
            },
            "hype-character-sector-link-v1": {
                "applies_when": "a rooted Perso+0x28 points to a bounded 24-byte sector-character list; its +4 saved reference resolves through a level sector-list element+4 to one rooted 160-byte sector",
                "meaning": "the source actor's assigned sector-list element, identified through its rooted sector scene path; this may be a fallback element when geometric research finds no sector, and the saved pointer targets the list element rather than sector data directly",
                "source_evidence": [
                    "reference/decompilation/functions/004476b0_FUN_004476b0.c:24",
                    "reference/decompilation/functions/00447750_FUN_00447750.c:14",
                    "reference/decompilation/functions/00447ba0_FUN_00447ba0.c:11",
                    "reference/decompilation/functions/00447bf0_FUN_00447bf0.c:25",
                    "reference/decompilation/functions/00447bf0_FUN_00447bf0.c:27",
                    "reference/decompilation/functions/004e0020_SECT_fn_hResearchInWhatSectorIAm.c:48",
                    "src/openspace/level/save_join.rs",
                ],
                "runtime_scope": "53 actor links in the supplied manor player save resolve to two rooted manor sectors; fresh reload behavior is unverified",
            },
            "hype-instantiated-item-superobject-reference-v1": {
                "applies_when": "a kind 0x11 save target is a bounded 64-byte Hype SuperObject with type 0x12, whose +4 data pointer reaches a typed 24-byte Fix inventory handle",
                "known_target_role": "instantiated inventory item SuperObject; current target item name and ID are source-resolved",
                "meaning_limit": "the saving source field/owner, menu-slot role, and change/restoration trigger remain unproven, so records remain semantically unknown",
                "source_evidence": [
                    "reference/decompilation/functions/004fd370_HIE_fn_xLoadSuperObject.c:536",
                    "reference/decompilation/functions/004fe6f0_HIE_fn_vRecursiveRotationUpdate.c:17",
                    "src/openspace/level/save_join.rs",
                ],
                "runtime_scope": "16 such targets in the supplied manor save; 11 switch from NotFound 50 to named magic items across one retained magic-labelled save",
            },
            "hype-map-epoch-selectable-v1": {
                "applies_when": "one of four 4-byte Fix words in the 16-byte map allocation at 0:5:4986392..4986404, matching the 004164d0 third registration loop and reset values",
                "meaning": "nonzero makes the corresponding map epoch slot selectable when cycling; zero skips it",
                "meaning_limit": "fresh-process restoration and later initialization effects are unverified",
                "source_evidence": [
                    "reference/decompilation/functions/004164d0_FUN_004164d0.c:24",
                    "reference/decompilation/functions/00416800_FUN_00416800.c:40",
                    "reference/decompilation/functions/00417030_FUN_00417030.c:14",
                ],
            },
            "hype-map-setting-slot-v1": {
                "applies_when": "100 uniquely bounded 72-byte Fix map UI entries in allocation order, each with one four-byte 0:5/0x04/0x0205 save at +0x30",
                "ownership": "runtime table entry ordinal; entry 0 is separately initialized, while entries 1..99 are setting-parser slot candidates, not established authored rows",
                "meaning_limit": "the matching language-specific setting file, actual parsed row count, authored labels, gameplay consumers and restoration effects are unavailable",
                "source_evidence": [
                    "reference/decompilation/functions/00416800_FUN_00416800.c:50",
                    "reference/decompilation/functions/00416560_FUN_00416560.c:24",
                    "reference/decompilation/functions/004164d0_FUN_004164d0.c:14",
                    "reference/decompilation/functions/00416780_FUN_00416780.c:32",
                ],
            },
            "hype-map-state-array-slot-v1": {
                "applies_when": "ten consecutive four-byte 0:5/0x04/0x0205 saves exactly cover the bounded 40-byte Fix allocation for DAT_00750ac4",
                "ownership": "runtime array word ordinal 0..9; initialization and reset store the raw bit pattern 0xfffffffe",
                "meaning_limit": "per-word gameplay role, accepted value domain, signedness and restoration effects are unknown",
                "source_evidence": [
                    "reference/decompilation/functions/00416800_FUN_00416800.c:23",
                    "reference/decompilation/functions/004164d0_FUN_004164d0.c:19",
                    "reference/decompilation/functions/00416780_FUN_00416780.c:15",
                ],
            },
        },
        "entries": entries,
    }, coverage


def corroborate_live_registry(
    catalogue: dict, coverage: dict, registry_path: Path, root: Path
) -> None:
    registry = json.loads(registry_path.read_text())
    worker = json.loads((registry_path.parent / "worker.json").read_text())
    if worker["fixture"]["exe_sha256"] != EXE_SHA256 or worker["fixture"]["save_sha256"] != FIXTURE_SHA256:
        raise ValueError("live registry came from a different executable or save fixture")
    if worker["game"] != {"phase": 9, "level": "manoir"}:
        raise ValueError("live registry did not capture playable manoir")
    nodes = registry["nodes"]
    if registry["declared_count"] != len(nodes):
        raise ValueError("live registry header and traversal count differ")
    indexed = defaultdict(list)
    for node in nodes:
        location = node["mapped_source"]
        if location is None:
            continue
        identity = f"{location['block']}:{location['module']}:{location['offset']}"
        indexed[identity].append(node["flags"])
    evidence_path = str(registry_path.resolve().relative_to(root))
    for identity, entry in catalogue["entries"].items():
        expected = {
            (int(item["kind"], 16), int(item["flags"], 16))
            for item in entry["file_occurrences"]
        }
        exact = [
            value for value in indexed.get(identity, [])
            if ((value >> 24) & 0xff, value & 0xffff) in expected
        ]
        if not exact:
            raise ValueError(f"save identity lacks exact live kind/flags registration: {identity}")
        entry["live_registration"] = {
            "matching_node_count": len(exact),
            "observed_phase": "post-load playable manoir",
            "evidence": evidence_path,
            "limit": "presence in one paused runtime registry does not prove save selection or later lifecycle",
        }
    coverage["live_registry"] = {
        "evidence": evidence_path,
        "declared_node_count": len(nodes),
        "mapped_node_count": sum(node["mapped_source"] is not None for node in nodes),
        "unmapped_node_count": sum(node["mapped_source"] is None for node in nodes),
        "distinct_mapped_identities": len(indexed),
        "catalogue_identities_with_exact_kind_flags": len(catalogue["entries"]),
        "catalogue_matching_nodes": sum(
            entry["live_registration"]["matching_node_count"]
            for entry in catalogue["entries"].values()
        ),
        "nodes_at_catalogue_addresses": sum(
            len(indexed[identity]) for identity in catalogue["entries"]
        ),
        "additional_distinct_runtime_identities": len(set(indexed) - set(catalogue["entries"])),
    }

    # The end of 00417fe0's inventory registration order is distinctive:
    # special resource handle, item maximum, four list words, then its
    # separately allocated one-byte full-list flag.
    inventory_tail = [
        "0:10:5790", "0:10:5828", "1:6:5322192", "1:6:5322536",
        "1:6:5322880", "1:6:5323224", "0:5:4986340",
    ]
    inventory_positions = [
        index for index, node in enumerate(nodes)
        if node["mapped_source"] is not None
        and f"{node['mapped_source']['block']}:{node['mapped_source']['module']}:{node['mapped_source']['offset']}" == inventory_tail[0]
    ]
    if len(inventory_positions) != 1:
        raise ValueError("special inventory registration root is not unique")
    tail_start = inventory_positions[0]
    observed_tail = nodes[tail_start:tail_start + len(inventory_tail)]
    if len(observed_tail) != len(inventory_tail) or [
        f"{node['mapped_source']['block']}:{node['mapped_source']['module']}:{node['mapped_source']['offset']}"
        if node["mapped_source"] is not None else None
        for node in observed_tail
    ] != inventory_tail or [
        ((node["flags"] >> 24) & 0xff, node["flags"] & 0xffff)
        for node in observed_tail
    ] != [(2, 0x0204), (2, 0x0204), *([(4, 0x0204)] * 4), (1, 0x0204)]:
        raise ValueError("inventory registration tail no longer matches 00417fe0")
    full_flag = catalogue["entries"]["0:5:4986340"]
    flag_join = full_flag["source_allocation_join"]
    flag_allocation = flag_join["candidates"][0]
    if (flag_join["status"] != "single_bounded_allocation"
            or flag_allocation["owner"] != "fix"
            or flag_allocation["allocation_data_start"] != 4986340
            or flag_allocation["allocation_end"] != 4986344
            or full_flag["meaning_status"] != "unknown"
            or any(item["kind"] != "0x01" or item["flags"] != "0x0204"
                   or item["value_size"] != 1 or item["value_hex"] not in ("00", "01")
                   for item in full_flag["file_occurrences"])):
        raise ValueError("inventory full flag changed source allocation or Boolean save shape")
    full_flag["ownership_status"] = "source_object_and_field"
    full_flag["source_owner"] = "Hype inventory state"
    full_flag["source_field"] = "main_list_full"
    full_flag["source_type"] = "u8 Boolean"
    full_flag["source_rule"] = "hype-inventory-full-list-flag-v1"
    full_flag["owner"] = full_flag["source_owner"]
    full_flag["field"] = full_flag["source_field"]
    full_flag["semantic"] = {
        "owner": full_flag["owner"],
        "field": full_flag["field"],
        "meaning": "inventory list full flag, set when no free slot remains or the last slot is filled and cleared after item removal or consumption",
        "confidence": "source_verified",
        "source_rule": "hype-inventory-full-list-flag-v1",
        "source_evidence": [
            "reference/decompilation/functions/00417fe0_FUN_00417fe0.c:100-107",
            "reference/decompilation/functions/0041c720_FUN_0041c720.c:64-80",
            "reference/decompilation/functions/0041cb70_FUN_0041cb70.c:68,102",
            "output/savegame-semantic-map/inventory-full-flag-followup.md",
            evidence_path,
        ],
    }
    full_flag["meaning_status"] = "source_verified"
    full_flag["unresolved"] = [
        "normal fill/clear transitions and fresh-process restoration have not been independently observed",
    ]
    coverage["source_object_and_field_identities"] += 1
    coverage["source_verified_only_identities"] += 1
    coverage["unknown_identities"] -= 1
    coverage["families"]["0:5/0x01/0x0204"]["explained"] += 1
    catalogue["source_family_rules"]["hype-inventory-full-list-flag-v1"] = {
        "applies_when": "the pinned live registry's seven-node inventory registration tail matches 00417fe0 and ends in a bounded one-byte Fix allocation with kind 0x01 and flags 0x0204",
        "meaning": "Boolean inventory list full state set on no free slot or filling the last slot and cleared on item removal/consumption",
        "meaning_limit": "normal transition and fresh-process restoration have no separate live check",
        "source_evidence": [
            "reference/decompilation/functions/00417fe0_FUN_00417fe0.c:100-107",
            "reference/decompilation/functions/0041c720_FUN_0041c720.c:64-80",
            "output/savegame-semantic-map/inventory-full-flag-followup.md",
            evidence_path,
        ],
    }

    # DLG_fn_vLoadDialog registers three fields per dialog-list instance in
    # this order: its list reference, the list's one-byte count, and one
    # reference per member. The live registry gives 67 exact 0:21 triplets;
    # the saved reference targets and byte values check the interpretation.
    dialogue = [
        (index, node) for index, node in enumerate(nodes)
        if node["mapped_source"] is not None
        and (node["mapped_source"]["block"], node["mapped_source"]["module"]) == (0, 21)
        and node["flags"] & 0xffff == 0x0a0c
    ]
    if len(dialogue) != 201 or [index for index, _ in dialogue] != list(range(dialogue[0][0], dialogue[0][0] + 201)):
        raise ValueError("dialogue registry is not 67 contiguous three-field groups")
    for ordinal in range(67):
        triplet = [node for _, node in dialogue[ordinal * 3:ordinal * 3 + 3]]
        if [node["flags"] >> 24 for node in triplet] != [0x11, 0x01, 0x11]:
            raise ValueError(f"dialogue registry group {ordinal} changed kind/order")
        ids = [
            f"0:21:{node['mapped_source']['offset']}" for node in triplet
        ]
        try:
            root_ref, count, member_ref = [catalogue["entries"][identity] for identity in ids]
        except KeyError as error:
            raise ValueError(f"dialogue registry group {ordinal} is absent from save") from error
        if not all(target["target_identity"] == ids[1] for target in root_ref["reference_targets"]):
            raise ValueError(f"dialogue group {ordinal} root reference misses its count")
        if not all(item["value_hex"] == "01" for item in count["file_occurrences"]):
            raise ValueError(f"dialogue group {ordinal} count is not one")
        if len(member_ref["reference_targets"]) != len(member_ref["file_occurrences"]):
            raise ValueError(f"dialogue group {ordinal} member reference is missing")
        owner = f"Hype dialogue list #{ordinal} (post-load registration order)"
        for entry, field, source_type in [
            (root_ref, "list_reference", "relocated reference to counted list"),
            (count, "entry_count", "u8 count"),
            (member_ref, "entry_reference[0]", "relocated reference to parsed dialogue answer object"),
        ]:
            entry["source_owner"] = owner
            entry["source_field"] = field
            entry["source_type"] = source_type
            entry["source_rule"] = "hype-dialogue-list-registration-v1"
            entry["ownership_status"] = "live_registered_object_and_field"
            entry["dialogue_registration_ordinal"] = ordinal
        count["owner"] = owner
        count["field"] = "entry_count"
        count["semantic"] = {
            "owner": owner,
            "field": "entry_count",
            "meaning": "number of references in this dialogue list; one in the supplied manor save",
            "confidence": "source_verified",
            "source_rule": "hype-dialogue-list-registration-v1",
            "source_evidence": [
                "reference/decompilation/functions/0056ba50_DLG_fn_vLoadDialog.c:46",
                evidence_path,
            ],
        }
        count["meaning_status"] = "source_verified"
        count["unresolved"] = ["dialogue entry content and authored identity remain untyped"]
        for entry, role in [
            (root_ref, "counted dialogue list"),
            (member_ref, "parsed DialogAnswer_<n> object in the ItemAnswer array"),
        ]:
            entry["partial_semantics"] = {
                "reference_role": role,
                "source_evidence": [
                    "reference/decompilation/functions/0056ba50_DLG_fn_vLoadDialog.c:46",
                    "reference/decompilation/functions/0056f4e0_DLG_eCallbackAnalyseItemSection.c:36",
                    "reference/decompilation/functions/0056fec0_DLG_eCallbackAnalyseAnswerSection.c:96",
                    "reference/decompilation/functions/0056e3b0_FUN_0056e3b0.c:17",
                ],
                "unknown": "the target's authored answer key, text and effect have not been resolved",
            }
            entry["unresolved"] = ["the target's authored answer key, text and effect remain unknown"]
    coverage["dialogue_registration_groups"] = 67
    coverage["live_registered_object_and_field_identities"] = 201
    star_entries = [
        entry for entry in catalogue["entries"].values()
        if entry.get("registration_group", {}).get("name") == "map_star_entry_plus_0x38_candidate"
    ]
    if len(star_entries) != 60 or {entry["registration_group"]["ordinal"] for entry in star_entries} != set(range(60)):
        raise ValueError("map star-entry registration ordinals are incomplete")
    star_entries.sort(key=lambda entry: entry["registration_group"]["ordinal"])
    star_addresses = []
    for entry in star_entries:
        ordinal = entry["registration_group"]["ordinal"]
        if any(item["kind"] != "0x04" or item["flags"] != "0x0205" or item["value_size"] != 4
               for item in entry["file_occurrences"]):
            raise ValueError(f"map star entry {ordinal} changed kind, flags or width")
        identity = f"1:10:{1248 + 64 * ordinal}"
        matches = [
            node for node in nodes
            if node["mapped_source"] is not None
            and f"{node['mapped_source']['block']}:{node['mapped_source']['module']}:{node['mapped_source']['offset']}" == identity
        ]
        if (entry["record_identity"] != identity or len(matches) != 1
                or matches[0]["flags"] != 0x04000205
                or not isinstance(matches[0]["source_address"], int)
                or entry["source_allocation_join"]["status"] != "no_bounded_source_block"
                or entry["source_allocation_join"]["candidates"]):
            raise ValueError(f"map star entry {ordinal} lost its exact live registration")
        star_addresses.append(matches[0]["source_address"])
        entry["registration_group"]["binding"] = (
            "one exact post-load live registry node; ordinal and addresses are process-local"
        )
        entry["source_owner"] = f"Hype map star entry #{ordinal} (post-load allocation order)"
        entry["source_field"] = "record+0x38"
        entry["source_type"] = "raw four-byte word; signedness unknown"
        entry["source_rule"] = "hype-map-star-entry-registration-v1"
        entry["ownership_status"] = "live_registered_object_and_field"
        entry["unresolved"] = ["the gameplay purpose and value domain of this star-entry word remain unknown"]
    if any(address != star_addresses[0] + 64 * ordinal
           for ordinal, address in enumerate(star_addresses)):
        raise ValueError("map star-entry live addresses lost their 64-byte stride")
    coverage["live_registered_object_and_field_identities"] += 60
    coverage["owned_typed_identities"] = coverage["source_object_and_field_identities"] + 261
    coverage["source_verified_only_identities"] += 67
    coverage["unknown_identities"] -= 67
    coverage["partially_explained_unknown_identities"] += 134
    coverage["families"]["0:21/0x01/0x0a0c"]["explained"] = 67
    catalogue["source_family_rules"]["hype-dialogue-list-registration-v1"] = {
        "applies_when": "the post-load registry has contiguous 0:21 triplets of reference/count/reference with low flags 0x0a0c, matching DLG_fn_vLoadDialog registration order and saved pointer targets",
        "ownership": "one dialogue list instance by post-load registration ordinal; the ordinal is contextual, not a portable authored dialogue ID",
        "meaning": "the middle u8 is the number of dialogue member references, one in each of 67 supplied manor groups",
        "meaning_limit": "the list reference targets its count and the member reference targets a parsed answer-section object; individual authored answer keys, text and effects are not resolved",
        "source_evidence": [
            "reference/decompilation/functions/0056ba50_DLG_fn_vLoadDialog.c:46",
            "reference/decompilation/functions/0056f4e0_DLG_eCallbackAnalyseItemSection.c:36",
            "reference/decompilation/functions/0056fec0_DLG_eCallbackAnalyseAnswerSection.c:96",
            "reference/decompilation/functions/0056e3b0_FUN_0056e3b0.c:17",
            evidence_path,
        ],
    }
    catalogue["source_family_rules"]["hype-map-star-entry-registration-v1"] = {
        "applies_when": "the map registrar's fourth loop registers 60 four-byte words at star-entry +0x38, matched to all 60 dynamic 1:10 records by exact identity set, kind/flags, live node uniqueness and 64-byte stride",
        "ownership": "one map star entry by post-load allocation ordinal; the ordinal is contextual, not an authored map marker ID",
        "meaning_limit": "the saved word's individual gameplay purpose, value domain and later restoration behavior remain unknown",
        "source_evidence": [
            "reference/decompilation/functions/004164d0_FUN_004164d0.c:27",
            "reference/decompilation/functions/00416800_FUN_00416800.c:25",
            evidence_path,
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--inspection", type=Path, required=True)
    parser.add_argument("--allocation-joins", type=Path)
    parser.add_argument("--live-registry", type=Path)
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args()
    root = args.repo.resolve()
    inspection = json.loads(args.inspection.read_text())
    manifest = corpus_manifest(root, args.config, inspection)
    index = registration_index(root)
    joins = json.loads(args.allocation_joins.read_text()) if args.allocation_joins else None
    catalogue, coverage = semantic_catalogue(inspection, joins)
    if args.live_registry:
        corroborate_live_registry(catalogue, coverage, args.live_registry, root)
    write_json(args.output_root / "corpus-manifest.json", manifest)
    write_json(args.output_root / "registration-index.json", index)
    write_json(args.output_root / "semantic-catalogue.json", catalogue)
    write_json(args.output_root / "coverage.json", coverage)
    print(
        f"{manifest['available_level_count']} levels; {index['call_count']} literal registration calls; "
        f"{coverage['save_file_occurrences']} record occurrences / "
        f"{coverage['distinct_contextual_record_identities']} identities; "
        f"{coverage['runtime_corroborated_identities']} runtime-corroborated, "
        f"{coverage['source_verified_only_identities']} source-verified, "
        f"{coverage['unknown_identities']} unknown"
    )


if __name__ == "__main__":
    main()
