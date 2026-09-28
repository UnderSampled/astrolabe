#!/usr/bin/env python3
"""Inspect an imported Fix/manoir Rete package and rebuild Global DSG AI usage.

The tool follows typed Perso Brain references with Orrery's maintained
inspector, then joins those model AST sites to the save-semantic catalogue.
An existing ``actors``/``models`` export is optional comparison input. This
tool does not import game packages, launch the game, or decode save bytes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

PROFILE = "hype-pc-english-supplied-save"
GLOBAL_ACTOR = "fix:/persos/15"
GLOBAL_MODEL = "fix:/ai/models/1"
BYTES_BY_TYPE = {
    "designer type 0": 1,
    "designer type 2": 1,
    "designer type 5": 4,
    "designer type 10": 4,
    "designer type 12": 12,
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def inspect(binary: Path, rete_root: Path, asset: str, kind: str) -> dict[str, Any]:
    completed = subprocess.run(
        [str(binary), "inspect", str(rete_root), "--asset", asset, "--kind", kind],
        capture_output=True, text=True, timeout=20, check=False,
    )
    if completed.returncode:
        message = (completed.stderr or completed.stdout)[-2000:]
        raise RuntimeError(f"inspect {asset} ({kind}) failed: {message}")
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"inspect {asset} ({kind}) returned invalid JSON: {error}") from error


def asset_uri(path: Path, rete_root: Path) -> str:
    relative = path.relative_to(rete_root)
    parts = list(relative.parts)
    if parts[0] == "fix":
        prefix = "fix:"
        parts = parts[1:]
    elif parts[0] == "levels" and len(parts) >= 3:
        prefix = "level:"
        parts = parts[2:]  # Drop the package directory (manoir).
    else:
        raise ValueError(f"unsupported imported asset path: {relative}")
    stem = Path(*parts).with_suffix("").as_posix()
    return f"{prefix}/{stem}"


def export_from_rete(binary: Path, rete_root: Path, output_root: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    if not binary.is_file():
        raise ValueError(f"Astrolabe executable does not exist: {binary}")
    if not (rete_root / "fix").is_dir() or not (rete_root / "levels" / "manoir").is_dir():
        raise ValueError("Rete root must contain Fix and active levels/manoir packages")

    perso_files = sorted([
        *((rete_root / "fix" / "persos").rglob("*.rete")),
        *((rete_root / "levels" / "manoir" / "persos").rglob("*.rete")),
    ])
    actors = []
    model_paths: set[str] = set()
    failures = []
    for path in perso_files:
        actor = asset_uri(path, rete_root)
        try:
            perso = inspect(binary, rete_root, actor, "perso")
            brain = perso.get("brain")
            model = brain.get("model") if isinstance(brain, dict) else None
            if isinstance(model, str) and model:
                actors.append({"actor": actor, "model": model, "name": perso.get("name")})
                model_paths.add(model)
        except (RuntimeError, ValueError) as error:
            failures.append({"asset": actor, "kind": "perso", "error": str(error)})

    models = {}
    for model in sorted(model_paths):
        try:
            models[model] = inspect(binary, rete_root, model, "ai-model")
        except (RuntimeError, ValueError) as error:
            failures.append({"asset": model, "kind": "ai-model", "error": str(error)})

    # Hash every enumerated Rete source file in stable path order. This is a
    # compact identity for the imported package slice used by this export.
    inventory = []
    for directory in (rete_root / "fix" / "persos", rete_root / "fix" / "ai" / "models",
                      rete_root / "levels" / "manoir" / "persos",
                      rete_root / "levels" / "manoir" / "ai" / "models"):
        if directory.is_dir():
            inventory.extend(sorted(directory.rglob("*.rete")))
    inventory_hash = hashlib.sha256()
    for path in sorted(set(inventory)):
        relative = path.relative_to(rete_root).as_posix()
        inventory_hash.update(relative.encode("utf-8"))
        inventory_hash.update(b"\0")
        inventory_hash.update(bytes.fromhex(sha256(path)))

    output_root.mkdir(parents=True, exist_ok=True)
    if failures:
        failure_path = output_root / "typed-ai-export-failures.json"
        failure_path.write_text(json.dumps({"failures": failures}, indent=2) + "\n")
        raise RuntimeError(f"{len(failures)} inspect failures; details written to {failure_path}")
    actors.sort(key=lambda item: (item["actor"], item["model"], item["name"] or ""))
    export = {"actors": actors, "models": {key: models[key] for key in sorted(models)}}
    provenance = {
        "rete_root": str(rete_root),
        "asset_inventory_sha256": inventory_hash.hexdigest(),
        "perso_asset_count": len(perso_files),
        "linked_actor_count": len(actors),
        "unique_linked_model_count": len(models),
        "inspect_executable": str(binary),
        "inspect_executable_sha256": sha256(binary),
    }
    return export, provenance


def compare_json(left: Any, right: Any, tolerance: float = 1e-5) -> tuple[bool, int, float]:
    """Compare decoded exports while allowing JSON float-print precision noise."""
    float_differences = 0
    largest_float_difference = 0.0

    def walk(a: Any, b: Any) -> bool:
        nonlocal float_differences, largest_float_difference
        if isinstance(a, bool) or isinstance(b, bool):
            return type(a) is type(b) and a == b
        if isinstance(a, (int, float)) and isinstance(b, (int, float)):
            delta = abs(float(a) - float(b))
            if delta:
                float_differences += 1
                largest_float_difference = max(largest_float_difference, delta)
            return delta <= tolerance
        if type(a) is not type(b):
            return False
        if isinstance(a, dict):
            return a.keys() == b.keys() and all(walk(a[key], b[key]) for key in a)
        if isinstance(a, list):
            return len(a) == len(b) and all(walk(x, y) for x, y in zip(a, b))
        return a == b

    equal = walk(left, right)
    return equal, float_differences, largest_float_difference


def asset_file(model_path: str, rete_root: Path) -> str:
    if model_path.startswith("fix:"):
        base = rete_root / "fix"
    elif model_path.startswith("level:"):
        base = rete_root / "levels" / "manoir"
    else:
        base = rete_root / "unknown"
    return str(base / "ai" / "models" / f"{model_path.rsplit('/', 1)[-1]}.rete")


def global_saved_fields(catalogue: dict[str, Any]) -> dict[int, dict[str, Any]]:
    saved = {}
    for identity, entry in catalogue["entries"].items():
        field = entry.get("source_field")
        if entry.get("source_owner") != GLOBAL_ACTOR or not isinstance(field, str):
            continue
        if not (field.startswith("DSG[") and field.endswith("]") and field[4:-1].isdigit()):
            continue
        index = int(field[4:-1])
        occurrence = (entry.get("file_occurrences") or [{}])[0]
        raw = occurrence.get("value_hex")
        value: Any = None
        if raw is not None:
            try:
                value = bytes.fromhex(raw).hex()
            except ValueError:
                value = raw
        saved[index] = {
            "identity": identity,
            "source_object": GLOBAL_ACTOR,
            "source_field": field,
            "source_type": entry.get("source_type"),
            "fixture_value_hex": value,
            "save_evidence_file": occurrence.get("file"),
            "save_flags": occurrence.get("flags"),
        }
    return saved


def build_local_index(export: dict[str, Any], catalogue: dict[str, Any],
                      external_index: dict[str, Any], export_path: Path,
                      catalogue_path: Path) -> dict[str, Any]:
    """Project direct local Brain accesses from linked models onto each Perso.

    Model descriptors describe a reusable layout. Current fixture values are
    joined separately by the exact actor URI plus DSG index.
    """
    actors, models = export.get("actors"), export.get("models")
    if not isinstance(actors, list) or not isinstance(models, dict):
        raise ValueError("typed-AI export must contain actor list and model map")

    owners: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for actor in actors:
        owners[actor["model"]].append(actor)

    saved_by_actor_field: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for identity, entry in catalogue.get("entries", {}).items():
        actor = entry.get("source_owner")
        field = entry.get("source_field")
        if isinstance(actor, str) and isinstance(field, str):
            saved_by_actor_field[(actor, field)].append({"identity": identity, "entry": entry})

    model_sites: dict[str, list[dict[str, Any]]] = defaultdict(list)
    unclassified: list[dict[str, Any]] = []
    counts: Counter[str] = Counter()
    unsupported_fragments = 0
    dynamic_main_actor_sites = 0
    dynamic_main_actor_array_target_sites = 0
    explicit_actor_incdec_sites = 0
    dynamic_main_actor_incdec_sites = 0

    def add_site(model_uri: str, model: dict[str, Any], index: Any, access: str,
                 operand_form: str, group: str, behavior_id: Any, script: str,
                 path: str) -> None:
        if not isinstance(index, int) or isinstance(index, bool):
            unclassified.append({
                "source_model": model_uri, "behavior": {"group": group, "id": behavior_id},
                "script": script, "path": path, "access": access,
                "operand_form": operand_form, "reason": "local operand has no static integer index",
            })
            return
        if index < 0 or index >= len(model.get("variables", [])):
            unclassified.append({
                "source_model": model_uri, "dsg_index": index,
                "behavior": {"group": group, "id": behavior_id},
                "script": script, "path": path, "access": access,
                "operand_form": operand_form, "reason": "local index is outside model descriptor table",
            })
            return
        model_sites[model_uri].append({
            "dsg_index": index, "access": access, "operand_form": operand_form,
            "behavior": {"group": group, "id": behavior_id},
            "script": script, "path": path,
        })
        counts[access] += 1

    def walk(node: Any, model_uri: str, model: dict[str, Any], group: str,
             behavior_id: Any, script: str, path: str) -> None:
        nonlocal unsupported_fragments, dynamic_main_actor_sites, dynamic_main_actor_array_target_sites
        nonlocal explicit_actor_incdec_sites, dynamic_main_actor_incdec_sites
        if isinstance(node, dict):
            # Each typed AI node is represented by a one-key enum object. Handle
            # array operations at the parent so their base is not double-counted
            # as a scalar Variable read.
            if "Variable" in node and isinstance(node["Variable"], int):
                add_site(model_uri, model, node["Variable"], "read",
                         "Expression::Variable", group, behavior_id, script, path)
                return
            if "SelfFieldRead" in node:
                add_site(model_uri, model, node["SelfFieldRead"], "read",
                         "Expression::SelfFieldRead", group, behavior_id, script, path)
                return
            if "Unsupported" in node:
                # The preserved raw payload is not a typed operand tree.
                unsupported_fragments += 1
                return
            if "ActorListContains" in node and isinstance(node["ActorListContains"], dict):
                value = node["ActorListContains"]
                add_site(model_uri, model, value.get("list"), "list_read",
                         "Expression::ActorListContains", group, behavior_id, script, path)
                walk(value.get("actor"), model_uri, model, group, behavior_id,
                     script, f"{path}.actor")
                return
            if "ArrayElement" in node and isinstance(node["ArrayElement"], dict):
                value = node["ArrayElement"]
                base = value.get("array")
                direct = base.get("Variable") if isinstance(base, dict) else None
                if isinstance(direct, int) and not isinstance(direct, bool):
                    add_site(model_uri, model, direct, "array_read",
                             "Expression::ArrayElement(Variable base)", group,
                             behavior_id, script, path)
                else:
                    unclassified.append({
                        "source_model": model_uri, "behavior": {"group": group, "id": behavior_id},
                        "script": script, "path": path, "access": "array_read",
                        "operand_form": "Expression::ArrayElement",
                        "reason": "array base is not a direct Variable slot",
                    })
                    walk(base, model_uri, model, group, behavior_id, script, f"{path}.array")
                walk(value.get("index"), model_uri, model, group, behavior_id,
                     script, f"{path}.index")
                return
            if "ArrayAssign" in node and isinstance(node["ArrayAssign"], dict):
                value = node["ArrayAssign"]
                base = value.get("array")
                direct = base.get("Variable") if isinstance(base, dict) else None
                if isinstance(direct, int) and not isinstance(direct, bool):
                    add_site(model_uri, model, direct, "array_write",
                             "Statement::ArrayAssign(Variable base)", group,
                             behavior_id, script, path)
                else:
                    nested_dynamic_main_actor = count_main_actor_fields(base)
                    unclassified.append({
                        "source_model": model_uri, "behavior": {"group": group, "id": behavior_id},
                        "script": script, "path": path, "access": "array_write",
                        "operand_form": "Statement::ArrayAssign",
                        "reason": "compound or non-Variable array target is unresolved",
                        "nested_dynamic_main_actor_read_assign_sites": nested_dynamic_main_actor,
                    })
                    dynamic_main_actor_array_target_sites += nested_dynamic_main_actor
                    # Keep reads in the index/value expressions; the target is
                    # deliberately not searched for a guessed destination slot.
                walk(value.get("index"), model_uri, model, group, behavior_id,
                     script, f"{path}.index")
                walk(value.get("value"), model_uri, model, group, behavior_id,
                     script, f"{path}.value")
                return

            for kind, value in node.items():
                current = f"{path}.{kind}"
                if kind in ("Assign", "AddAssign", "SubtractAssign", "VectorComponentAssign") and isinstance(value, dict):
                    index = value.get("vector") if kind == "VectorComponentAssign" else value.get("variable")
                    add_site(model_uri, model, index, "write", f"Statement::{kind}",
                             group, behavior_id, script, current)
                elif kind in ("Increment", "Decrement") and isinstance(value, dict):
                    target = value.get("target")
                    if isinstance(target, dict) and isinstance(target.get("Variable"), int):
                        add_site(model_uri, model, target["Variable"], "write",
                                 f"Statement::{kind}(FieldTarget::Variable)", group,
                                 behavior_id, script, current)
                    elif isinstance(target, dict) and "ActorField" in target:
                        explicit_actor_incdec_sites += 1
                    elif isinstance(target, dict) and "MainActorField" in target:
                        dynamic_main_actor_incdec_sites += 1
                    # The target's Variable enum is an l-value, never a read.
                    continue
                elif kind in ("MainActorFieldRead", "MainActorFieldAssign"):
                    dynamic_main_actor_sites += 1
                walk(value, model_uri, model, group, behavior_id, script, current)
        elif isinstance(node, list):
            for index, value in enumerate(node):
                walk(value, model_uri, model, group, behavior_id, script,
                     f"{path}[{index}]")

    def count_main_actor_fields(node: Any) -> int:
        """Count dynamic target forms nested in an unresolved l-value only."""
        if isinstance(node, dict):
            return sum(
                (1 if kind in ("MainActorFieldRead", "MainActorFieldAssign") else 0)
                + count_main_actor_fields(value)
                for kind, value in node.items()
            )
        if isinstance(node, list):
            return sum(count_main_actor_fields(value) for value in node)
        return 0

    for model_uri, model in models.items():
        if not owners.get(model_uri):
            continue
        for group in ("normal", "reflex"):
            behavior_set = model.get(group) or {}
            for behavior in behavior_set.get("behaviors", []):
                behavior_id = behavior.get("id")
                schedule = behavior.get("schedule")
                if isinstance(schedule, dict):
                    for statement_index, statement in enumerate(schedule.get("statements", [])):
                        walk(statement, model_uri, model, group, behavior_id,
                             "schedule", f"{group}.behaviors[{behavior_id}].schedule.statements[{statement_index}]")
                for rule_index, rule in enumerate(behavior.get("rules") or []):
                    if isinstance(rule, dict):
                        for statement_index, statement in enumerate(rule.get("statements", [])):
                            walk(statement, model_uri, model, group, behavior_id,
                                 f"rule[{rule_index}]",
                                 f"{group}.behaviors[{behavior_id}].rules[{rule_index}].statements[{statement_index}]")

    actor_rows: list[dict[str, Any]] = []
    for actor in sorted(actors, key=lambda item: (item["actor"], item["model"])):
        actor_uri, model_uri = actor["actor"], actor["model"]
        model = models[model_uri]
        grouped: dict[int, list[dict[str, Any]]] = defaultdict(list)
        for site in model_sites.get(model_uri, []):
            grouped[site["dsg_index"]].append(site)
        saved_indices: set[int] = set()
        for owner_uri, source_field in saved_by_actor_field:
            if owner_uri == actor_uri and source_field.startswith("DSG[") and source_field.endswith("]"):
                raw_index = source_field[4:-1]
                if raw_index.isdigit():
                    saved_indices.add(int(raw_index))
        local_fields = []
        for index in sorted(set(grouped) | saved_indices):
            sites = grouped.get(index, [])
            source_field = f"DSG[{index}]"
            candidates = saved_by_actor_field.get((actor_uri, source_field), [])
            fixture = None
            if len(candidates) == 1:
                identity, entry = candidates[0]["identity"], candidates[0]["entry"]
                occurrence = (entry.get("file_occurrences") or [{}])[0]
                fixture = {
                    "identity": identity, "source_field": source_field,
                    "source_type": entry.get("source_type"),
                    "value_hex": occurrence.get("value_hex"),
                    "file": occurrence.get("file"), "flags": occurrence.get("flags"),
                }
                fixture_status = "single_exact_actor_field_match"
            elif not candidates:
                fixture_status = "no_exact_actor_field_match_in_catalogue"
            else:
                fixture_status = "ambiguous_exact_actor_field_matches"
            descriptors = model.get("variables", [])
            descriptor = descriptors[index] if index < len(descriptors) else None
            local_fields.append({
                "dsg_index": index,
                "model_descriptor": descriptor,
                "model_descriptor_status": "matched" if descriptor is not None else "no_model_descriptor_at_index",
                "fixture_saved_field_status": fixture_status,
                "fixture_saved_field": fixture,
                "fixture_candidates": [candidate["identity"] for candidate in candidates] if len(candidates) > 1 else [],
                "sites": [
                    {"actor": actor_uri, "source_model": model_uri, **site}
                    for site in sorted(sites, key=lambda item: (item["path"], item["access"], item["operand_form"]))
                ],
            })
        actor_rows.append({
            "actor": actor_uri, "actor_name": actor.get("name"),
            "source_model": model_uri, "local_fields": local_fields,
        })

    projected_site_count = sum(len(field["sites"]) for actor in actor_rows for field in actor["local_fields"])
    linked_models = {actor["model"] for actor in actors}
    return {
        "schema": "local-ai-variable-index-v1",
        "input": {
            "typed_ai_export": str(export_path), "typed_ai_export_sha256": sha256(export_path),
            "semantic_catalogue": str(catalogue_path), "semantic_catalogue_sha256": sha256(catalogue_path),
            "external_index": "global-inverse-ai-index.json",
            "external_index_schema": external_index.get("schema"),
        },
        "coverage": {
            "linked_actor_count": len(actors), "unique_linked_model_count": len(linked_models),
            "models_with_direct_local_sites": len({uri for uri, sites in model_sites.items() if sites}),
            "actors_with_direct_local_sites": sum(any(field["sites"] for field in row["local_fields"]) for row in actor_rows),
            "actor_qualified_field_count": sum(len(row["local_fields"]) for row in actor_rows),
            "actor_qualified_fields_with_sites": sum(bool(field["sites"]) for row in actor_rows for field in row["local_fields"]),
            "actor_qualified_saved_field_count": sum(field["fixture_saved_field"] is not None
                                                      for row in actor_rows for field in row["local_fields"]),
            "projected_local_site_count": projected_site_count,
            "model_local_site_count": sum(len(sites) for sites in model_sites.values()),
            "local_sites_by_access": dict(sorted(counts.items())),
            "unclassified_compound_or_invalid_targets": len(unclassified),
            "unsupported_ast_fragments": unsupported_fragments,
            "dynamic_main_actor_read_assign_sites_not_projected": dynamic_main_actor_sites,
            "dynamic_main_actor_targets_inside_unclassified_array_assign": dynamic_main_actor_array_target_sites,
            "legacy_external_index_dynamic_target_count": len(external_index.get("unresolved_dynamic_targets", [])),
            "explicit_actor_field_increment_decrement_sites_not_projected": explicit_actor_incdec_sites,
            "dynamic_main_actor_increment_decrement_sites_not_projected": dynamic_main_actor_incdec_sites,
        },
        "actors": actor_rows,
        "unclassified_local_targets": unclassified,
        "dynamic_main_actor_scope": {
            "typed_read_assign_sites_outside_unclassified_array_assign": dynamic_main_actor_sites,
            "read_assign_sites_inside_unclassified_array_assign": dynamic_main_actor_array_target_sites,
            "sum_matching_legacy_external_index": dynamic_main_actor_sites + dynamic_main_actor_array_target_sites,
            "legacy_external_index_dynamic_target_count": len(external_index.get("unresolved_dynamic_targets", [])),
            "preserved_unsupported_payloads_traversed_as_typed_sites": False,
        },
        "interpretation": (
            "Variable/local operands address the executing Perso Brain current buffer; "
            "model descriptors are shared layout/default metadata. Fixture identities "
            "are joined only by exact actor URI and DSG index. Explicit ActorField "
            "and dynamic MainActor targets are not projected into this local index. "
            "Dynamic MainActor read/assign coverage excludes preserved Unsupported payloads "
            "and compound ArrayAssign target expressions; their counts are reported separately."
        ),
    }


def assert_pinned_manoir_fixture(local_index: dict[str, Any],
                                external_index: dict[str, Any],
                                ranking: dict[str, Any],
                                catalogue: dict[str, Any]) -> None:
    """Guard the supplied-save actor identities and shared-model projection."""
    actors = {row["actor"]: row for row in local_index["actors"]}

    def field(actor_uri: str, model_uri: str, index: int) -> dict[str, Any]:
        actor = actors.get(actor_uri)
        if actor is None or actor.get("source_model") != model_uri:
            raise AssertionError(f"fixture actor/model mismatch: {actor_uri} -> {model_uri}")
        result = next((item for item in actor["local_fields"] if item["dsg_index"] == index), None)
        if result is None:
            raise AssertionError(f"missing fixture local slot: {actor_uri} DSG[{index}]")
        return result

    def identity(row: dict[str, Any], expected_id: str | None, expected_value: str | None) -> None:
        actual = row.get("fixture_saved_field")
        if expected_id is None:
            if actual is not None:
                raise AssertionError(f"unexpected saved identity: {actual}")
        elif not actual or actual.get("identity") != expected_id or actual.get("value_hex") != expected_value:
            raise AssertionError(f"fixture saved field mismatch: expected {expected_id}={expected_value}, got {actual}")

    def access_counts(row: dict[str, Any]) -> tuple[int, int]:
        sites = row["sites"]
        return (sum(site["access"] == "read" for site in sites),
                sum(site["access"] == "write" for site in sites))

    hype = field("level:/persos/world/0/0", "fix:/ai/models/28", 57)
    identity(hype, "0:17:15540", "00")
    if access_counts(hype) != (121, 4):
        raise AssertionError(f"Hype DSG[57] AST access counts changed: {access_counts(hype)}")
    hype_write_paths = {site["path"] for site in hype["sites"] if site["access"] == "write"}
    if hype_write_paths != {
        "normal.behaviors[0].schedule.statements[67].If.then[0].Assign",
        "normal.behaviors[0].schedule.statements[67].If.otherwise[0].Assign",
        "reflex.behaviors[7].rules[0].statements[35].If.then[0].Assign",
        "reflex.behaviors[7].rules[0].statements[35].If.otherwise[0].Assign",
    }:
        raise AssertionError(f"Hype DSG[57] write paths changed: {sorted(hype_write_paths)}")
    global_57 = next((entry for entry in catalogue["entries"].values()
                      if entry.get("source_owner") == GLOBAL_ACTOR and entry.get("source_field") == "DSG[57]"), None)
    if global_57 is None:
        raise AssertionError("missing separate saved Global DSG[57]")
    global_57_ids = [identity for identity, entry in catalogue["entries"].items()
                     if entry.get("source_owner") == GLOBAL_ACTOR and entry.get("source_field") == "DSG[57]"]
    if global_57_ids != ["0:17:4658"]:
        raise AssertionError(f"Global DSG[57] identity changed: {global_57_ids}")
    global_actor_57 = field(GLOBAL_ACTOR, "fix:/ai/models/1", 57)
    identity(global_actor_57, "0:17:4658", "00")
    if hype.get("fixture_saved_field", {}).get("identity") == global_actor_57.get("fixture_saved_field", {}).get("identity"):
        raise AssertionError("Hype and Global DSG[57] unexpectedly share one saved identity")

    lever = field("level:/persos/world/0/174", "level:/ai/models/1396", 3)
    identity(lever, "1:17:187070", "00")
    if access_counts(lever) != (1, 2):
        raise AssertionError(f"lever DSG[3] AST access counts changed: {access_counts(lever)}")
    lever_write_paths = {site["path"] for site in lever["sites"] if site["access"] == "write"}
    if lever_write_paths != {
        "normal.behaviors[3].schedule.statements[1].If.otherwise[0].Assign",
        "normal.behaviors[5].schedule.statements[1].Assign",
    }:
        raise AssertionError(f"lever DSG[3] write paths changed: {sorted(lever_write_paths)}")

    gogood = field("level:/persos/world/0/5", "level:/ai/models/210", 0)
    identity(gogood, None, None)
    if access_counts(gogood) != (1, 1):
        raise AssertionError(f"Gogood model-210 slot-0 access counts changed: {access_counts(gogood)}")
    return_actor = field("level:/persos/world/0/161", "level:/ai/models/1251", 0)
    identity(return_actor, "1:17:171448", "00")
    if access_counts(return_actor) != (3, 3):
        raise AssertionError(f"Gogood return slot-0 access counts changed: {access_counts(return_actor)}")

    mannequin_expected = {
        "level:/persos/world/0/152": "1:17:127662",
        "level:/persos/world/0/153": "1:17:131194",
        "level:/persos/world/0/154": "1:17:132114",
    }
    for actor_uri, record_id in mannequin_expected.items():
        mannequin = field(actor_uri, "level:/ai/models/939", 0)
        identity(mannequin, record_id, "04")
        if access_counts(mannequin) != (6, 3):
            raise AssertionError(f"mannequin {actor_uri} DSG[0] access counts changed: {access_counts(mannequin)}")

    linked_actor_uris = set(actors)
    expected_saved = {
        (entry.get("source_owner"), entry.get("source_field"), identity)
        for identity, entry in catalogue["entries"].items()
        if entry.get("source_owner") in linked_actor_uris
        and isinstance(entry.get("source_field"), str)
        and entry["source_field"].startswith("DSG[")
        and entry["source_field"].endswith("]")
        and entry["source_field"][4:-1].isdigit()
    }
    indexed_saved = [
        (row["actor"], field_row["fixture_saved_field"]["source_field"],
         field_row["fixture_saved_field"]["identity"])
        for row in local_index["actors"] for field_row in row["local_fields"]
        if field_row["fixture_saved_field"] is not None
    ]
    if set(indexed_saved) != expected_saved or len(indexed_saved) != len(expected_saved):
        raise AssertionError("actor-local index does not reconcile every exact saved actor DSG identity once")
    site_keys = [
        (site["actor"], site["source_model"], site["path"], site["access"], site["operand_form"])
        for row in local_index["actors"] for field_row in row["local_fields"] for site in field_row["sites"]
    ]
    if len(site_keys) != len(set(site_keys)):
        raise AssertionError("actor-local index contains duplicate projected site keys")

    coverage = external_index["coverage"]
    summary = ranking["summary"]
    if coverage["resolved_typed_external_sites"] != 1216:
        raise AssertionError("external typed-site total changed from the pinned 1,216")
    if summary["matched_explicit_external_read_sites"] != 335 or summary["matched_explicit_external_write_sites"] != 416:
        raise AssertionError("saved Global explicit-site totals changed from the pinned 335/416")
    dynamic = local_index["dynamic_main_actor_scope"]
    if dynamic["sum_matching_legacy_external_index"] != 757 or dynamic["legacy_external_index_dynamic_target_count"] != 757:
        raise AssertionError("typed/dynamic-target coverage no longer reconciles to the pinned 757 sites")


def build_index(export: dict[str, Any], catalogue: dict[str, Any], rete_root: Path,
                export_path: Path, catalogue_path: Path) -> dict[str, Any]:
    actors, models = export.get("actors"), export.get("models")
    if not isinstance(actors, list) or not isinstance(models, dict):
        raise ValueError("typed-AI export must contain actor list and model map")
    model_owners: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for actor in actors:
        model_owners[actor["model"]].append(actor)

    sites: list[dict[str, Any]] = []
    unresolved: list[dict[str, Any]] = []
    unsupported: list[dict[str, Any]] = []
    own_variable_indices: set[int] = set()
    global_local_counts: dict[str, dict[str, int]] = {}
    saved = global_saved_fields(catalogue)

    for model_path, model in models.items():
        sources = model_owners.get(model_path, [{
            "actor": None, "name": "unrooted-model", "model": model_path,
        }])

        def visit(node: Any, group: str, behavior_id: Any, script_name: str, path: str) -> None:
            if isinstance(node, dict):
                if "Unsupported" in node:
                    detail = node["Unsupported"]
                    unsupported.append({
                        "source_model": model_path,
                        "source_model_name": model.get("name"),
                        "source_asset_file": asset_file(model_path, rete_root),
                        "source_actors": sources,
                        "source_actor_link_status": "linked" if sources[0].get("actor") else "model asset imported without a linked Perso in this package",
                        "behavior": {"group": group, "id": behavior_id},
                        "path": path,
                        "reason": detail.get("reason"),
                        "preserved_nodes": len(detail.get("nodes", [])),
                    })
                for kind, value in node.items():
                    current = f"{path}.{kind}"
                    if kind in ("ActorFieldRead", "ActorFieldAssign") and isinstance(value, dict):
                        target, index = value.get("actor"), value.get("variable")
                        access = "read" if kind == "ActorFieldRead" else "write"
                        record = {
                            "target_actor": target, "dsg_index": index, "access": access,
                            "source_model": model_path, "source_model_name": model.get("name"),
                            "source_asset_file": asset_file(model_path, rete_root),
                            "source_actors": sources,
                            "source_actor_link_status": "linked" if sources[0].get("actor") else "model asset imported without a linked Perso in this package",
                            "behavior": {"group": group, "id": behavior_id},
                            "script": script_name, "path": current,
                        }
                        if target is None or index is None:
                            unresolved.append({**record, "reason": "typed external access lacks resolved actor or variable"})
                        else:
                            sites.append(record)
                    if kind in ("SelfFieldRead", "MainActorFieldRead", "MainActorFieldAssign"):
                        unresolved.append({
                            "target_actor": None, "dynamic_target": kind,
                            "dsg_index": value.get("variable") if isinstance(value, dict) else value,
                            "access": "write" if kind.endswith("Assign") else "read",
                            "source_model": model_path, "source_model_name": model.get("name"),
                            "source_asset_file": asset_file(model_path, rete_root),
                            "source_actors": sources,
                            "source_actor_link_status": "linked" if sources[0].get("actor") else "model asset imported without a linked Perso in this package",
                            "behavior": {"group": group, "id": behavior_id},
                            "script": script_name, "path": current,
                            "reason": "target is runtime self/main-actor keyword; no canonical actor identity",
                        })
                    visit(value, group, behavior_id, script_name, current)
            elif isinstance(node, list):
                for index, value in enumerate(node):
                    visit(value, group, behavior_id, script_name, f"{path}[{index}]")

        for group in ("normal", "reflex"):
            behavior_set = model.get(group) or {}
            for behavior in behavior_set.get("behaviors", []):
                behavior_id = behavior.get("id")
                schedule = behavior.get("schedule")
                if schedule is not None:
                    visit(schedule, group, behavior_id, "schedule", f"{group}.behaviors[{behavior_id}].schedule")
                for rule_index, rule in enumerate(behavior.get("rules") or []):
                    if rule is not None:
                        visit(rule, group, behavior_id, f"rule[{rule_index}]", f"{group}.behaviors[{behavior_id}].rule[{rule_index}]")

    for model in models.values():
        def collect_variables(node: Any) -> None:
            if isinstance(node, dict):
                for key, value in node.items():
                    if key == "Variable" and isinstance(value, int):
                        own_variable_indices.add(value)
                    collect_variables(value)
            elif isinstance(node, list):
                for value in node:
                    collect_variables(value)
        collect_variables(model)

    global_model = models.get(GLOBAL_MODEL, {})
    local = {index: {"read": 0, "write": 0} for index in saved}

    def collect_global_local(node: Any) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "Variable" and isinstance(value, int) and value in local:
                    local[value]["read"] += 1
                if key in ("Assign", "SubtractAssign", "AddAssign", "VectorComponentAssign") and isinstance(value, dict) and value.get("variable") in local:
                    local[value["variable"]]["write"] += 1
                collect_global_local(value)
        elif isinstance(node, list):
            for value in node:
                collect_global_local(value)
    collect_global_local(global_model)

    saved_rows = []
    for index, metadata in sorted(saved.items()):
        hits = [site for site in sites if site["target_actor"] == GLOBAL_ACTOR and site["dsg_index"] == index and site["source_model"] != GLOBAL_MODEL]
        saved_rows.append({
            **metadata, "dsg_index": index,
            "external_reader_count": sum(site["access"] == "read" for site in hits),
            "external_writer_count": sum(site["access"] == "write" for site in hits),
            "sites": hits,
        })
    return {
        "schema": "global-inverse-ai-index-v1",
        "input": {
            "typed_ai_export": str(export_path), "typed_ai_export_sha256": sha256(export_path),
            "semantic_catalogue": str(catalogue_path), "semantic_catalogue_sha256": sha256(catalogue_path),
            "rete_root": str(rete_root),
            "scope": "all exported Fix and active-level Perso assets with Brain model links and every exported model",
            "actor_count": len(actors), "unique_model_count": len(models),
            "global_actor": GLOBAL_ACTOR, "global_model": GLOBAL_MODEL,
        },
        "coverage": {
            "resolved_typed_external_sites": len(sites),
            "external_read_sites": sum(site["access"] == "read" for site in sites),
            "external_write_sites": sum(site["access"] == "write" for site in sites),
            "unresolved_dynamic_actor_field_sites": len(unresolved),
            "unsupported_ast_fragments": len(unsupported),
            "distinct_own_variable_indices_across_models": len(own_variable_indices),
            "models_with_brain_actor_link": len(model_owners),
        },
        "saved_global_fields": saved_rows,
        "resolved_external_sites_to_global_actor": sites,
        "unresolved_dynamic_targets": unresolved,
        "unsupported_ast_fragments": unsupported,
        "global_actor_brain_variable_occurrences": {str(index): local[index] for index in sorted(local)},
        "interpretation": "Only ActorFieldRead/ActorFieldAssign with a canonical actor URI are counted here as external reader/writer sites. Expression::Variable(n) and other local operands address slot n in the Brain of the Perso executing the linked model; see the sibling actor-qualified local index. Unsupported fragments are retained as coverage evidence but their operands are not classified as DSG accesses.",
    }


def rank_fields(catalogue: dict[str, Any], index: dict[str, Any]) -> dict[str, Any]:
    rows = []
    sites_by_index: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for site in index["resolved_external_sites_to_global_actor"]:
        if site.get("target_actor") == GLOBAL_ACTOR and isinstance(site.get("dsg_index"), int) and site.get("access") in ("read", "write"):
            sites_by_index[site["dsg_index"]].append(site)
    for identity, entry in catalogue["entries"].items():
        if entry.get("source_owner") != GLOBAL_ACTOR:
            continue
        field = entry.get("source_field")
        if not isinstance(field, str) or not (field.startswith("DSG[") and field.endswith("]") and field[4:-1].isdigit()):
            continue
        number = int(field[4:-1])
        sites = sorted(sites_by_index[number], key=lambda site: (
            site["access"], site.get("source_model_name", ""),
            site.get("behavior", {}).get("group", ""), site.get("behavior", {}).get("id", -1), site.get("path", ""),
        ))
        occurrence = (entry.get("file_occurrences") or [{}])[0]
        rows.append({
            "identity": identity, "source_owner": entry["source_owner"], "source_field": field,
            "dsg_index": number, "source_type": entry.get("source_type"),
            "payload_extent_bytes": BYTES_BY_TYPE.get(entry.get("source_type")),
            "fixture_value_hex": occurrence.get("value_hex"),
            "meaning_status": entry.get("meaning_status"),
            "meaning": (entry.get("semantic") or {}).get("meaning"),
            "external_reader_count": sum(site["access"] == "read" for site in sites),
            "external_writer_count": sum(site["access"] == "write" for site in sites),
            "external_site_count": len(sites),
            "distinct_source_model_count": len({site.get("source_model") for site in sites}),
            "sites": [{key: site.get(key) for key in (
                "target_actor", "dsg_index", "access", "source_model", "source_model_name",
                "source_asset_file", "source_actors", "source_actor_link_status", "behavior", "script", "path",
            )} for site in sites],
        })
    rows.sort(key=lambda row: (-row["external_site_count"], -row["distinct_source_model_count"], row["dsg_index"]))
    unknown = [row for row in rows if row["meaning_status"] != "source_verified"]
    counts = Counter(site["access"] for row in rows for site in row["sites"])
    return {
        "schema": "global-saved-field-ranking-v1",
        "inputs": {"semantic_catalogue_sha256": index["input"]["semantic_catalogue_sha256"], "inverse_index_sha256": "computed after index serialization"},
        "summary": {
            "saved_global_dsg_identities": len(rows), "unique_dsg_indices": len({row["dsg_index"] for row in rows}),
            "fields_with_explicit_external_readers": sum(row["external_reader_count"] > 0 for row in rows),
            "fields_with_explicit_external_writers": sum(row["external_writer_count"] > 0 for row in rows),
            "fields_with_any_explicit_external_site": sum(row["external_site_count"] > 0 for row in rows),
            "fields_without_explicit_external_site": sum(row["external_site_count"] == 0 for row in rows),
            "matched_explicit_external_read_sites": counts["read"],
            "matched_explicit_external_write_sites": counts["write"],
            "matched_explicit_external_sites_total": sum(counts.values()),
        },
        "ranking_method": "Unknown catalogue identities only, ordered by descending exact external ActorFieldRead/ActorFieldAssign sites, then descending distinct source models, then ascending DSG index. Counts are navigation evidence, not meaning evidence.",
        "all_saved_fields_ranked_by_use": rows,
        "high_value_unknowns_ranked_by_use": [dict(rank=rank, **row) for rank, row in enumerate(unknown, 1)],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--typed-ai-export", type=Path, help="Optional existing JSON export for comparison")
    parser.add_argument("--binary", type=Path, help="Astrolabe executable (defaults to target/release/astrolabe)")
    parser.add_argument("--catalogue", type=Path, required=True, help="save_semantics.py semantic-catalogue.json")
    parser.add_argument("--rete-root", type=Path, required=True, help="Root of the existing imported Rete package")
    parser.add_argument("--output-root", type=Path, required=True, help="Ignored output directory")
    args = parser.parse_args()
    root = args.repo.resolve()
    binary = (args.binary or (root / "target" / "release" / "astrolabe")).resolve()
    catalogue_path = args.catalogue.resolve()
    rete_root = args.rete_root.resolve()
    output_root = args.output_root.resolve()
    if not catalogue_path.is_file():
        raise SystemExit("semantic catalogue must be a readable file")
    catalogue = json.loads(catalogue_path.read_text())
    if catalogue.get("schema") != 1 or catalogue.get("profile") != PROFILE or not isinstance(catalogue.get("entries"), dict):
        raise SystemExit("catalogue schema/profile mismatch")
    try:
        export, provenance = export_from_rete(binary, rete_root, output_root)
    except (OSError, RuntimeError, ValueError, subprocess.TimeoutExpired) as error:
        raise SystemExit(str(error)) from error
    export_path = output_root / "typed-ai-export.json"
    export_path.write_text(json.dumps(export, indent=2, ensure_ascii=False) + "\n")
    if args.typed_ai_export:
        comparison_path = args.typed_ai_export.resolve()
        if not comparison_path.is_file():
            raise SystemExit(f"comparison export is missing: {comparison_path}")
        comparison = json.loads(comparison_path.read_text())
        same_actors = sorted(comparison.get("actors", []), key=lambda item: item.get("actor", "")) == sorted(export["actors"], key=lambda item: item.get("actor", ""))
        same_models, float_difference_count, max_float_difference = compare_json(
            comparison.get("models"), export["models"]
        )
        comparison_report = {
            "comparison_export": str(comparison_path),
            "comparison_export_sha256": sha256(comparison_path),
            "actors_match": same_actors, "models_match": same_models,
            "comparison_actor_count": len(comparison.get("actors", [])),
            "fresh_actor_count": len(export["actors"]),
            "comparison_model_count": len(comparison.get("models", {})),
            "fresh_model_count": len(export["models"]),
            "float_precision_differences_within_1e-5": float_difference_count,
            "maximum_float_difference": max_float_difference,
        }
        output_root.mkdir(parents=True, exist_ok=True)
        (output_root / "typed-ai-export-comparison.json").write_text(json.dumps(comparison_report, indent=2) + "\n")
        if not same_actors or not same_models:
            raise SystemExit(f"fresh inspector export differs from comparison export; see {output_root / 'typed-ai-export-comparison.json'}")
    index = build_index(export, catalogue, rete_root, export_path, catalogue_path)
    index["input"]["inspector_provenance"] = provenance
    index["input"]["inspect_executable"] = provenance["inspect_executable"]
    index["input"]["inspect_executable_sha256"] = provenance["inspect_executable_sha256"]
    index["input"]["rete_asset_inventory_sha256"] = provenance["asset_inventory_sha256"]
    local_index = build_local_index(export, catalogue, index, export_path, catalogue_path)
    local_index["input"]["inspector_provenance"] = provenance
    local_index["input"]["inspect_executable"] = provenance["inspect_executable"]
    local_index["input"]["inspect_executable_sha256"] = provenance["inspect_executable_sha256"]
    local_index["input"]["rete_asset_inventory_sha256"] = provenance["asset_inventory_sha256"]
    ranking = rank_fields(catalogue, index)
    if catalogue.get("profile") == PROFILE:
        assert_pinned_manoir_fixture(local_index, index, ranking, catalogue)
    output_root.mkdir(parents=True, exist_ok=True)
    index_path = output_root / "global-inverse-ai-index.json"
    local_index_path = output_root / "local-ai-variable-index.json"
    ranking_path = output_root / "global-saved-field-ranking.json"
    local_index_path.write_text(json.dumps(local_index, indent=2, ensure_ascii=False) + "\n")
    index["input"]["local_ai_variable_index"] = {
        "path": local_index_path.name,
        "sha256": sha256(local_index_path),
        "schema": local_index["schema"],
    }
    index_path.write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n")
    ranking["inputs"]["inverse_index_sha256"] = sha256(index_path)
    ranking_path.write_text(json.dumps(ranking, indent=2, ensure_ascii=False) + "\n")
    reads = ranking["summary"]["matched_explicit_external_read_sites"]
    writes = ranking["summary"]["matched_explicit_external_write_sites"]
    report = [
        "# Global saved DSG index and use ranking", "",
        f"Inputs: generated typed-AI export (`{len(export['actors'])}` linked actors, SHA-256 `{sha256(export_path)}`), semantic catalogue `{catalogue_path}` (SHA-256 `{sha256(catalogue_path)}`). Imported Rete inventory SHA-256 `{provenance['asset_inventory_sha256']}`; inspector binary SHA-256 `{provenance['inspect_executable_sha256']}`.",
        "",
        f"The typed-AI export contains {len(export['actors'])} Perso actors and {len(export['models'])} models. It yielded {len(index['resolved_external_sites_to_global_actor'])} resolved typed external field sites overall, including {reads} reads and {writes} writes to the saved Global DSG catalogue.",
        "",
        f"The catalogue contains {ranking['summary']['saved_global_dsg_identities']} saved Global DSG identities across {ranking['summary']['unique_dsg_indices']} indices. Exact external sites cover {ranking['summary']['fields_with_any_explicit_external_site']} identities; {ranking['summary']['fields_without_explicit_external_site']} have no supported explicit typed site. Ranking counts are navigation evidence, not semantic labels.",
        "",
        "## Highest-use unknowns", "",
        "| Rank | DSG | Identity | Type / bytes | Read / write | Fixture bytes |", "| ---: | ---: | --- | --- | ---: | --- |",
    ]
    for row in ranking["high_value_unknowns_ranked_by_use"][:25]:
        report.append(f"| {row['rank']} | {row['dsg_index']} | `{row['identity']}` | {row['source_type']} / {row['payload_extent_bytes']} | {row['external_reader_count']} / {row['external_writer_count']} | `{row['fixture_value_hex']}` |")
    report += ["", "## Scope limits", "", f"The export also contains {len(index['unresolved_dynamic_targets'])} unresolved runtime-target sites and {len(index['unsupported_ast_fragments'])} unsupported AST fragments. Neither is attributed to a canonical saved DSG without a typed canonical actor target. Actor-local `Variable(n)` accesses are projected per linked Perso in `local-ai-variable-index.json`.", "", "## Reproduction", "", "```sh", f"python3 tools/save_global_ai_index.py --catalogue {catalogue_path} --rete-root <imported-rete-root> --output-root output/savegame-semantic-map", "```", ""]
    (output_root / "global-saved-field-ranking.md").write_text("\n".join(report))
    coverage = index["coverage"]
    print(f"perso_assets={provenance['perso_asset_count']} actors={len(export['actors'])} models={len(export['models'])} typed_sites={coverage['resolved_typed_external_sites']} saved_fields={ranking['summary']['saved_global_dsg_identities']} matched_reads={reads} matched_writes={writes}")
    print(f"rete_inventory_sha256={provenance['asset_inventory_sha256']} inspector_sha256={provenance['inspect_executable_sha256']}")
    print(f"index_sha256={sha256(index_path)}")
    print(f"local_index_sha256={sha256(local_index_path)}")
    print(f"outputs={index_path} {local_index_path} {ranking_path} {output_root / 'global-saved-field-ranking.md'}")


if __name__ == "__main__":
    main()
