#!/usr/bin/env python3
"""Discover saved designer fields across the pinned local Hype level corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--per-level-timeout", type=int, default=45)
    args = parser.parse_args()
    root = args.repo.resolve()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"Orrery binary is missing: {binary}")
    manifest = json.loads(args.manifest.read_text())
    if manifest.get("schema") != 1 or manifest.get("profile") != "hype-pc-english-supplied-save":
        raise SystemExit("corpus manifest has the wrong schema or profile")
    levels = [item for item in manifest["source_packages"] if item["role"] == "level"]
    if len(levels) != manifest["available_level_count"]:
        raise SystemExit("level inventory differs from manifest count")
    output_root = args.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    results = []
    for package in levels:
        sna = root / package["sna"]
        sda = root / package["sda"] if package["sda"] else None
        if digest(sna) != package["sna_sha256"] or (sda and digest(sda) != package["sda_sha256"]):
            raise SystemExit(f"source package changed: {package['name']}")
        output = output_root / f"{package['name']}.json"
        command = [
            str(binary), "save-source-discover", "--game", str(root / "disc"),
            "--level", package["name"], "--output", str(output),
        ]
        try:
            completed = subprocess.run(command, capture_output=True, text=True,
                                       timeout=args.per_level_timeout, check=False)
            if completed.returncode == 0:
                report = json.loads(output.read_text())
                discovery = report["designer_discovery"]
                result = {
                    "level": package["name"], "status": "discovered",
                    "saved_field_occurrences": len(discovery["saved_fields"]),
                    "perso_nodes": discovery["perso_nodes"],
                    "persos_without_brain": discovery["persos_without_brain"],
                    "failed_actor_count": len(discovery["failed_actors"]),
                    "source_report": str(output.relative_to(root)),
                }
            else:
                result = {"level": package["name"], "status": "failed",
                          "error": (completed.stderr or completed.stdout)[-2500:]}
        except subprocess.TimeoutExpired:
            result = {"level": package["name"], "status": "timeout",
                      "seconds": args.per_level_timeout}
        results.append(result)
        print(f"{package['name']}: {result['status']}", flush=True)
    summary = {
        "schema": 1,
        "profile": manifest["profile"],
        "declared_level_count": len(levels),
        "discovered_level_count": sum(item["status"] == "discovered" for item in results),
        "failed_level_count": sum(item["status"] != "discovered" for item in results),
        "saved_field_occurrences": sum(item.get("saved_field_occurrences", 0) for item in results),
        "levels": results,
        "limit": "static descriptor discovery; no gameplay meaning or live registration implied",
    }
    (output_root / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(
        f"{summary['discovered_level_count']}/{len(levels)} levels; "
        f"{summary['saved_field_occurrences']} contextual saved-descriptor occurrences"
    )
    if summary["failed_level_count"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
