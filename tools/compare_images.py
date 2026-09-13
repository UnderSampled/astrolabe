#!/usr/bin/env python3
"""Pixel comparisons for already matched camera captures; requires ImageMagick 7.

No resizing or alignment is performed. Use named regions to separate defects
from HUD, animation phase, lighting and filtering differences.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile


def magick(*args, comparison=False):
    result = subprocess.run(["magick", *map(str, args)], capture_output=True, text=True, timeout=30)
    if result.returncode not in ((0, 1) if comparison else (0,)):
        raise RuntimeError(result.stderr.strip())
    return result


def dimensions(path):
    return tuple(map(int, magick("identify", "-format", "%w %h", path).stdout.split()))


def metric(name, reference, candidate, destination):
    result = magick("compare", "-metric", name, reference, candidate, destination, comparison=True)
    match = re.search(r"\(([0-9.eE+-]+)\)", result.stderr)
    if not match:
        raise RuntimeError("Unexpected ImageMagick metric: " + result.stderr)
    return float(match[1])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--region", action="append", default=[], metavar="NAME:X,Y,W,H")
    args = parser.parse_args()
    size = dimensions(args.reference)
    if dimensions(args.candidate) != size:
        parser.error("Capture dimensions differ; match the camera and viewport before comparison")
    regions = {"full": (0, 0, *size)}
    for value in args.region:
        name, coordinates = value.split(":", 1)
        if not re.fullmatch(r"[A-Za-z0-9_-]+", name) or name in regions:
            parser.error("Region names must be unique simple filenames")
        box = tuple(map(int, coordinates.split(",")))
        if len(box) != 4:
            parser.error("Region needs x,y,width,height")
        x, y, w, h = box
        if min(x, y) < 0 or min(w, h) <= 0 or x + w > size[0] or y + h > size[1]:
            parser.error("Region lies outside the capture")
        regions[name] = box
    args.output.mkdir(parents=True, exist_ok=False)
    report = {"tool": magick("-version").stdout.splitlines()[0], "size": size, "images": {}, "regions": {}}
    for name, path in [("reference", args.reference), ("candidate", args.candidate)]:
        report["images"][name] = {"path": str(path.resolve()), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    with tempfile.TemporaryDirectory(prefix="astrolabe-pixels-") as temp:
        for name, box in regions.items():
            x, y, w, h = box
            crops = []
            for label, path in [("reference", args.reference), ("candidate", args.candidate)]:
                crop = Path(temp) / f"{label}.png"
                magick(path, "-alpha", "off", "-crop", f"{w}x{h}+{x}+{y}", "+repage", crop)
                crops.append(crop)
            difference = args.output / f"{name}-difference.png"
            rmse = metric("RMSE", *crops, difference)
            mae = metric("MAE", *crops, "null:")
            comparison = args.output / f"{name}-comparison.png"
            magick(*crops, difference, "+append", comparison)
            report["regions"][name] = {"xywh": box, "rmse": rmse, "mae": mae, "rmse_8bit": rmse * 255, "comparison": str(comparison.resolve()), "difference": str(difference.resolve())}
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: {m: v[m] for m in ("rmse", "mae")} for k, v in report["regions"].items()}, indent=2))


if __name__ == "__main__":
    main()
