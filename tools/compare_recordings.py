#!/usr/bin/env python3
"""Compare two session frame streams on one declared elapsed-time grid.

No frame search, shift, crop, rescale, or gameplay-state repair is performed.
Unassociated pixels are diagnostic until both backends publish verified render
association. Requires Pillow and NumPy, available in the workspace runtime.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image


@dataclass(frozen=True)
class Frame:
    bucket: int
    start: int
    end: int
    path: Path
    width: int
    height: int
    association: str

    @property
    def midpoint(self) -> int:
        return (self.start + self.end) // 2


@dataclass(frozen=True)
class VideoFrame:
    ordinal: int
    path: Path
    elapsed_us: int
    width: int
    height: int
    association: str
    indexed_media_pts_micros: int | None


@dataclass(frozen=True)
class VideoSegment:
    path: Path
    relative_path: str
    frames: tuple[VideoFrame, ...]


def safe_recording_path(root: Path, value: object, expected_root: str) -> tuple[str, Path]:
    name = Path(str(value))
    if name.is_absolute() or ".." in name.parts or not name.parts or name.parts[0] != expected_root:
        raise ValueError(f"unsafe recording path: {name}")
    resolved = (root / name).resolve(strict=True)
    if not resolved.is_relative_to(root):
        raise ValueError(f"recording path escaped root: {name}")
    return name.as_posix(), resolved


def read_video_segments(root: Path, events: list[dict]) -> list[VideoSegment]:
    segments = []
    for event in events:
        if event.get("kind") != "video_stream_end":
            continue
        data = event.get("data", {})
        relative_video, video_path = safe_recording_path(root, data.get("path"), "video")
        relative_index, index_path = safe_recording_path(root, data.get("frame_index_path"), "video")
        if not relative_video.endswith(".mkv") or not relative_index.endswith(".frames.jsonl"):
            raise ValueError("unsupported video or frame-index path")
        frames = []
        previous_elapsed = -1
        with index_path.open(encoding="utf-8") as index_file:
            for line_number, line in enumerate(index_file, 1):
                try:
                    row = json.loads(line)
                    ordinal = int(row["index"])
                    elapsed = int(row["elapsed_micros"])
                    width, height = int(row["width"]), int(row["height"])
                    media_pts = row.get("pts_media_micros")
                    if media_pts is None:
                        media_pts = row.get("pts_unix_micros")
                    indexed_media_pts_micros = int(media_pts) if media_pts is not None else None
                    row_path = row["path"]
                    row_name, row_video_path = safe_recording_path(root, row_path, "video")
                except (json.JSONDecodeError, KeyError, TypeError, ValueError) as error:
                    raise ValueError(f"invalid video frame index at {index_path}:{line_number}: {error}") from error
                if ordinal != len(frames):
                    raise ValueError(f"video frame index order/count gap at {index_path}:{line_number}")
                if row_name != relative_video or row_video_path != video_path:
                    raise ValueError(f"video frame index points at a different segment: {index_path}:{line_number}")
                if elapsed <= previous_elapsed or width <= 0 or height <= 0:
                    raise ValueError(f"invalid or unordered video frame metadata at {index_path}:{line_number}")
                frames.append(VideoFrame(ordinal, video_path, elapsed, width, height,
                                         str(row.get("association", "unknown")), indexed_media_pts_micros))
                previous_elapsed = elapsed
        if not frames:
            raise ValueError(f"empty video frame index: {index_path}")
        if len(frames) != int(data.get("frames", -1)):
            raise ValueError(f"video frame index count differs from segment summary: {relative_video}")
        dimensions = {(frame.width, frame.height) for frame in frames}
        if len(dimensions) != 1:
            raise ValueError(f"video dimensions changed without a segment boundary: {relative_video}")
        segments.append(VideoSegment(video_path, relative_video, tuple(frames)))
    segments.sort(key=lambda segment: (segment.frames[0].elapsed_us, segment.relative_path))
    return segments


def media_pts_micros(timestamp: str) -> int:
    from decimal import Decimal, InvalidOperation, ROUND_HALF_UP
    try:
        return int((Decimal(timestamp) * 1_000_000).quantize(Decimal("1"), rounding=ROUND_HALF_UP))
    except InvalidOperation as error:
        raise ValueError(f"invalid media presentation timestamp: {timestamp}") from error


def validate_segment_media(segment: VideoSegment) -> None:
    """Stream ffprobe metadata and verify the sidecar maps each media frame."""
    process = subprocess.Popen(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "frame=best_effort_timestamp_time,width,height", "-of", "csv=p=0", str(segment.path)],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
    )
    assert process.stdout is not None
    try:
        count = 0
        for line in process.stdout:
            if not line.strip():
                continue
            fields = line.rstrip("\r\n").split(",")
            if len(fields) != 3 or count >= len(segment.frames):
                raise ValueError(f"media frame metadata count/format differs from index: {segment.relative_path}")
            expected = segment.frames[count]
            pts = media_pts_micros(fields[0])
            dimensions = (int(fields[1]), int(fields[2]))
            if dimensions != (expected.width, expected.height):
                raise ValueError(f"media dimensions differ from index at frame {count}: {segment.relative_path}")
            if expected.indexed_media_pts_micros is not None and abs(pts - expected.indexed_media_pts_micros) > 1:
                raise ValueError(f"media PTS differs from index at frame {count}: {segment.relative_path}")
            count += 1
        if process.wait() != 0:
            raise ValueError(f"ffprobe could not inspect video segment: {segment.relative_path}")
        if count != len(segment.frames):
            raise ValueError(f"media frame count differs from index: {segment.relative_path}")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        process.stdout.close()


def decode_segment(segment: VideoSegment):
    """Yield decoded RGBA frames using ffmpeg pipes; keep only one frame resident."""
    width, height = segment.frames[0].width, segment.frames[0].height
    process = subprocess.Popen(
        ["ffmpeg", "-nostdin", "-v", "error", "-i", str(segment.path), "-map", "0:v:0",
         "-fps_mode", "passthrough", "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
    )
    assert process.stdout is not None
    try:
        for metadata in segment.frames:
            size = metadata.width * metadata.height * 4
            chunks = bytearray()
            while len(chunks) < size:
                chunk = process.stdout.read(size - len(chunks))
                if not chunk:
                    raise ValueError(f"decoded video ended before indexed frame {metadata.ordinal}: {segment.relative_path}")
                chunks.extend(chunk)
            if metadata.width != width or metadata.height != height:
                raise ValueError(f"video dimensions changed within segment: {segment.relative_path}")
            yield metadata, np.frombuffer(chunks, dtype=np.uint8).reshape(height, width, 4)
        if process.stdout.read(1):
            raise ValueError(f"decoded video contains frames absent from its index: {segment.relative_path}")
        status = process.wait()
        if status:
            raise ValueError(f"ffmpeg could not decode {segment.relative_path} (exit {status})")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        if process.stdout:
            process.stdout.close()


def video_frames(segments: list[VideoSegment]):
    # Resize segments intentionally overlap. Keep each segment's complete raw
    # evidence, but use the newer segment from its first PTS onward for compare.
    for index, segment in enumerate(segments):
        cutoff = segments[index + 1].frames[0].elapsed_us if index + 1 < len(segments) else None
        validate_segment_media(segment)
        for frame in decode_segment(segment):
            if cutoff is None or frame[0].elapsed_us < cutoff:
                yield frame


def compare_video_streams(source: list[VideoSegment], candidate: list[VideoSegment],
                          tolerance_us: int, output: Path) -> dict:
    """Compare all frames on fixed 33.333 ms buckets, without run alignment."""
    bucket_us = 33_333
    left, right = iter(video_frames(source)), iter(video_frames(candidate))
    a = next(left, None)
    b = next(right, None)
    rows, divergences = [], []
    first_pixel = None
    while a is not None or b is not None:
        a_bucket = (a[0].elapsed_us + bucket_us // 2) // bucket_us if a else None
        b_bucket = (b[0].elapsed_us + bucket_us // 2) // bucket_us if b else None
        if b is None or (a is not None and a_bucket < b_bucket):
            pair_a, pair_b = a, None
            a = next(left, None)
        elif a is None or b_bucket < a_bucket:
            pair_a, pair_b = None, b
            b = next(right, None)
        else:
            pair_a, pair_b = a, b
            a, b = next(left, None), next(right, None)
        ma = pair_a[0] if pair_a else None
        mb = pair_b[0] if pair_b else None
        times = [frame.elapsed_us for frame in (ma, mb) if frame]
        at = min(times)
        row = {"elapsed_micros": at, "source_elapsed_micros": ma.elapsed_us if ma else None,
               "candidate_elapsed_micros": mb.elapsed_us if mb else None,
               "source_index": ma.ordinal if ma else None, "candidate_index": mb.ordinal if mb else None,
               "source_video": str(ma.path) if ma else None, "candidate_video": str(mb.path) if mb else None,
               "source_size": [ma.width, ma.height] if ma else None,
               "candidate_size": [mb.width, mb.height] if mb else None,
               "association": {"source": ma.association if ma else None,
                               "candidate": mb.association if mb else None}}
        if ma is None or mb is None:
            row["status"] = "missing_video_frame"
        elif abs(ma.elapsed_us - mb.elapsed_us) > tolerance_us:
            row["status"] = "video_timing_mismatch"
            row["time_delta_micros"] = abs(ma.elapsed_us - mb.elapsed_us)
        elif (ma.width, ma.height) != (mb.width, mb.height):
            row["status"] = "video_dimension_mismatch"
            row["time_delta_micros"] = abs(ma.elapsed_us - mb.elapsed_us)
        else:
            left_pixels, right_pixels = pair_a[1], pair_b[1]
            difference = np.abs(left_pixels.astype(np.int16) - right_pixels.astype(np.int16))
            row["status"] = "video_frame_compared"
            row["time_delta_micros"] = abs(ma.elapsed_us - mb.elapsed_us)
            row["pixel_metrics"] = {"mae_8bit": float(difference.mean()),
                                     "max_channel_error": int(difference.max()),
                                     "fraction_pixels_over_8": float(np.mean(np.max(difference, axis=2) > 8))}
            row["pixel_claim"] = (
                "qualified" if "game_render_update_verified" in ma.association
                and "game_render_update_verified" in mb.association
                else "diagnostic_unassociated"
            )
            if np.any(difference) and first_pixel is None:
                first_pixel = {"elapsed_micros": at, "source_index": ma.ordinal,
                               "candidate_index": mb.ordinal, "maximum_channel_error": int(difference.max()),
                               "source_video": ma.path.name, "candidate_video": mb.path.name,
                               "source_association": ma.association, "candidate_association": mb.association,
                               "claim": row["pixel_claim"]}
        row["fixed_time_bucket"] = (at + bucket_us // 2) // bucket_us
        rows.append(row)
        verified_pixel_difference = (
            row["status"] == "video_frame_compared"
            and row.get("pixel_metrics", {}).get("mae_8bit", 0) != 0
            and row.get("pixel_claim") == "qualified"
        )
        if row["status"] != "video_frame_compared" or verified_pixel_difference:
            divergences.append({"stream": "video", "elapsed_micros": at,
                                "status": "video_pixel_difference" if verified_pixel_difference else row["status"],
                                "source_index": row["source_index"], "candidate_index": row["candidate_index"]})
    def segments_report(segments):
        result = []
        gaps = []
        previous_size = None
        previous_frame = None
        for segment in segments:
            frame = segment.frames[0]
            size = [frame.width, frame.height]
            if previous_frame is not None and frame.elapsed_us - previous_frame.elapsed_us > 50_000:
                gaps.append({"start_elapsed_micros": previous_frame.elapsed_us,
                             "end_elapsed_micros": frame.elapsed_us,
                             "missing_time_micros": frame.elapsed_us - previous_frame.elapsed_us - 33_333,
                             "kind": "inter_segment_gap"})
            segment_gaps = []
            for before, after in zip(segment.frames, segment.frames[1:]):
                if after.elapsed_us - before.elapsed_us > 50_000:
                    gap = {"start_elapsed_micros": before.elapsed_us,
                           "end_elapsed_micros": after.elapsed_us,
                           "missing_time_micros": after.elapsed_us - before.elapsed_us - 33_333,
                           "kind": "timestamp_gap"}
                    segment_gaps.append(gap)
                    gaps.append(gap)
            cutoff = segments[len(result) + 1].frames[0].elapsed_us if len(result) + 1 < len(segments) else None
            superseded = sum(item.elapsed_us >= cutoff for item in segment.frames) if cutoff is not None else 0
            result.append({"path": segment.relative_path, "frames": len(segment.frames),
                           "start_elapsed_micros": frame.elapsed_us,
                           "end_elapsed_micros": segment.frames[-1].elapsed_us,
                           "resolution": size,
                           "resolution_change": previous_size is not None and previous_size != size,
                           "comparison_frames_superseded_by_next_segment": superseded,
                           "gaps": segment_gaps})
            previous_size = size
            previous_frame = segment.frames[-1]
        return result, gaps
    source_report, source_gaps = segments_report(source)
    candidate_report, candidate_gaps = segments_report(candidate)
    def overlap_report(segments):
        rows = []
        for older, newer in zip(segments, segments[1:]):
            start = newer.frames[0].elapsed_us
            end = min(older.frames[-1].elapsed_us, newer.frames[-1].elapsed_us)
            if start <= end:
                rows.append({"older_segment": older.relative_path, "newer_segment": newer.relative_path,
                             "overlap_elapsed_micros": [start, end],
                             "older_resolution": [older.frames[0].width, older.frames[0].height],
                             "newer_resolution": [newer.frames[0].width, newer.frames[0].height],
                             "older_frames_superseded": sum(frame.elapsed_us >= start for frame in older.frames),
                             "raw_frames_preserved": len(older.frames) + len(newer.frames),
                             "selection": "prefer newer segment from its first frame onward"})
        return rows
    return {"bucket_width_micros": bucket_us, "pairing_tolerance_micros": tolerance_us,
            "association_limit": "lossless presented-window samples; game render/update association is unverified",
            "source_segments": source_report, "candidate_segments": candidate_report,
            "source_overlaps": overlap_report(source), "candidate_overlaps": overlap_report(candidate),
            "frames": rows, "first_diagnostic_video_pixel_difference": first_pixel,
            "gaps": {"unpaired_frames": [row for row in rows if row["status"] == "missing_video_frame"],
                     "source_timing_gaps": source_gaps, "candidate_timing_gaps": candidate_gaps},
            "divergences": divergences}


def read_recording(root: Path, bucket_us: int) -> tuple[dict, list[dict], dict[int, list[Frame]]]:
    root = root.resolve(strict=True)
    manifest = json.loads((root / "manifest.json").read_text())
    if manifest.get("schema") != 1 or manifest.get("ending") == "in_progress":
        raise ValueError(f"incomplete or unsupported recording: {root}")
    events = []
    frames: dict[int, list[Frame]] = {}
    for line_number, line in enumerate((root / "events.jsonl").read_text().splitlines(), 1):
        try:
            event = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"torn event at {root}:{line_number}") from error
        if event.get("sequence") != len(events):
            raise ValueError(f"event sequence gap at {root}:{line_number}")
        events.append(event)
        if event.get("kind") != "frame":
            continue
        data = event["data"]
        name = Path(data["path"])
        if name.is_absolute() or ".." in name.parts or not name.parts or name.parts[0] != "frames":
            raise ValueError(f"unsafe frame path: {name}")
        path = (root / name).resolve(strict=True)
        if not path.is_relative_to(root):
            raise ValueError(f"frame escaped recording: {name}")
        start = int(data["request_start_micros"])
        end = int(data["reply_end_micros"])
        if end < start:
            raise ValueError(f"reversed frame acquisition interval: {name}")
        bucket = ((start + end) // 2) // bucket_us
        frames.setdefault(bucket, []).append(
            Frame(bucket, start, end, path, int(data["width"]), int(data["height"]), str(data.get("association", "unknown")))
        )
    return manifest, events, frames


def masks_for(size: tuple[int, int], masks: list[tuple[str, int, int, int, int, str]]) -> tuple[np.ndarray, list[dict]]:
    width, height = size
    include = np.ones((height, width), dtype=bool)
    descriptions = []
    for name, x, y, w, h, reason in masks:
        if min(x, y) < 0 or min(w, h) <= 0 or x + w > width or y + h > height:
            raise ValueError(f"mask {name} lies outside the frame")
        include[y : y + h, x : x + w] = False
        descriptions.append({"name": name, "xywh": [x, y, w, h], "reason": reason})
    if not include.any():
        raise ValueError("masks exclude the entire frame")
    return include, descriptions


def pixel_metrics(reference: Frame, candidate: Frame, masks: list[tuple[str, int, int, int, int, str]], threshold: int) -> tuple[dict, np.ndarray]:
    with Image.open(reference.path) as image:
        ref = np.asarray(image.convert("RGBA"), dtype=np.int16)
    with Image.open(candidate.path) as image:
        cand = np.asarray(image.convert("RGBA"), dtype=np.int16)
    if ref.shape != cand.shape or ref.shape[:2] != (reference.height, reference.width):
        raise ValueError("PNG content dimensions differ from frame metadata")
    include, descriptions = masks_for((reference.width, reference.height), masks)
    difference = np.abs(ref - cand)
    visible = difference[include]
    maximum = visible.max(axis=1)
    metrics = {
        "mae_8bit": float(visible.mean()),
        "rmse_8bit": float(np.sqrt(np.mean(np.square(visible.astype(np.float64))))),
        "max_channel_error": int(visible.max()),
        "fraction_pixels_over_threshold": float(np.mean(maximum > threshold)),
        "threshold_8bit": threshold,
        "included_pixels": int(include.sum()),
        "excluded_pixels": int((~include).sum()),
        "masks": descriptions,
    }
    heat = np.zeros((reference.height, reference.width, 3), dtype=np.uint8)
    heat[:, :, 0] = np.max(difference[:, :, :3], axis=2).astype(np.uint8)
    heat[~include] = 0
    return metrics, heat


def normalize_phase(value: object, godot: bool) -> str:
    if godot:
        return str(value)
    return {1: "startup", 38: "menu", 3: "loading", 9: "gameplay"}.get(value, f"source:{value}")


def state_samples(events: list[dict], godot: bool) -> list[dict]:
    result = []
    for event in events:
        if event.get("kind") != "state":
            continue
        state = event["data"].get("state", {})
        result.append({
            "phase": normalize_phase(state.get("phase"), godot),
            "level": state.get("level", ""),
            "elapsed_micros": int(event["elapsed_micros"]),
        })
    return result


def actor_samples(events: list[dict], godot: bool) -> list[dict]:
    result = []
    for event in events:
        data = event.get("data", {})
        if godot and event.get("kind") == "state":
            actor = data.get("state", {}).get("actor")
        elif not godot and event.get("kind") == "actor":
            actor = data.get("actor")
        else:
            continue
        if not isinstance(actor, dict):
            continue
        transform = actor.get("global_transform", {})
        affine = transform.get("Affine", {}) if isinstance(transform, dict) else {}
        result.append({
            "elapsed_micros": int(event["elapsed_micros"]),
            "translation": actor.get("translation") if godot else affine.get("translation"),
            "current_action": actor.get("current_action") if not godot else None,
            "secondary_action": actor.get("secondary_action") if not godot else None,
            "normal_behavior": data.get("normal_behavior") if not godot else None,
            "reflex_behavior": data.get("reflex_behavior") if not godot else None,
            "consistency": data.get("consistency", "process_frame_sample"),
        })
    return result


def input_samples(events: list[dict]) -> tuple[list[dict], str]:
    observed = [event for event in events if event.get("kind") in {"input", "input_observed"}]
    selected = observed or [event for event in events if event.get("kind") == "input_dispatch"]
    stage = "server_observed" if observed else "dispatch_only"
    result = []
    for event in selected:
        data = event.get("data", {})
        at = data.get("observed_micros") if stage == "server_observed" else None
        result.append({
            "elapsed_micros": int(at if at is not None else event["elapsed_micros"]),
            "code": data.get("code", data.get("source_code")),
            "keysym": data.get("keysym", data.get("source_keysym")),
            "edge": data.get("edge"),
            "repeat": data.get("repeat", False),
            "modifiers": data.get("modifiers"),
        })
    return result, stage


def pair_samples(source: list[dict], candidate: list[dict], tolerance_us: int):
    """Pair monotonic samples by observed time without shifting an entire run."""
    left = right = 0
    while left < len(source) or right < len(candidate):
        original = source[left] if left < len(source) else None
        target = candidate[right] if right < len(candidate) else None
        if original is None:
            yield None, target
            right += 1
        elif target is None:
            yield original, None
            left += 1
        elif abs(original["elapsed_micros"] - target["elapsed_micros"]) <= tolerance_us:
            yield original, target
            left += 1
            right += 1
        elif original["elapsed_micros"] < target["elapsed_micros"]:
            yield original, None
            left += 1
        else:
            yield None, target
            right += 1


def timeline_event(event: dict, backend: str, bucket_us: int) -> dict | None:
    kind = event.get("kind")
    if kind not in {"input", "input_observed", "input_dispatch", "input_gap", "epoch",
                    "state", "actor", "capture_gap", "frame_gap", "save_snapshot",
                    "session_failure", "session_end", "navigation_wait_start",
                    "navigation_gate_observation", "navigation_wait_end",
                    "navigation_wait_timeout"}:
        return None
    data = event.get("data", {})
    if kind.startswith("input"):
        summary = f"{data.get('edge', '')} key {data.get('code', data.get('source_code', '?'))}"
    elif kind == "state":
        state = data.get("state", {})
        summary = f"phase {state.get('phase', '?')} · {state.get('level', '')}"
    elif kind == "actor":
        actor = data.get("actor", {})
        normal = data.get("normal_behavior") or {}
        summary = f"action {actor.get('current_action', '?')} · normal {normal.get('current', '?')}"
    elif kind == "epoch":
        summary = f"phase {data.get('phase', '?')} · {data.get('level', '')}"
    elif kind == "save_snapshot":
        summary = f"stable save change · {len(data.get('saves', {}).get('files', []))} files"
    elif kind == "session_failure":
        summary = str(data.get("error", "capture failed"))
    elif kind == "navigation_wait_start":
        summary = f"wait for {data.get('gate', '?')}"
    elif kind == "navigation_gate_observation":
        summary = f"{data.get('gate', '?')} · {'matched' if data.get('matched') else 'unmatched'} · {data.get('frame', '?')}"
    elif kind == "navigation_wait_end":
        summary = f"{data.get('gate', '?')} ready · +{int(data.get('added_delay_micros', 0)) / 1000:.1f} ms"
    elif kind == "navigation_wait_timeout":
        summary = f"{data.get('gate', '?')} timed out · {data.get('last_frame', '?')}"
    else:
        summary = str(data.get("reason", data.get("ending", "")))
    elapsed = int(event["elapsed_micros"])
    return {"backend": backend, "elapsed_micros": elapsed, "kind": kind,
            "summary": summary, "frame_bucket": elapsed // bucket_us}


def write_viewer(output: Path, report: dict, source_events: list[dict], candidate_events: list[dict],
                 bucket_us: int, source_label: str, candidate_label: str) -> None:
    frames = []
    for row in report["frames"]:
        frames.append({**row,
                       "source": os.path.relpath(row["source"], output) if row.get("source") else None,
                       "candidate": os.path.relpath(row["candidate"], output) if row.get("candidate") else None})
    timeline = [item for backend, events in ((source_label, source_events), (candidate_label, candidate_events))
                for event in events if (item := timeline_event(event, backend, bucket_us)) is not None]
    timeline.sort(key=lambda item: (item["elapsed_micros"], item["backend"]))
    payload = json.dumps({"frames": frames, "timeline": timeline,
                          "first_divergence": report["first_divergence"],
                          "first_pixel": report["first_diagnostic_pixel_difference"],
                          "policy": report["policy"],
                          "source_label": source_label, "candidate_label": candidate_label},
                         ensure_ascii=False).replace("<", "\\u003c")
    page = """<!doctype html>
<html lang="en"><meta charset="utf-8"><title>Session comparison</title>
<style>
body{font:15px system-ui,sans-serif;background:#171b22;color:#eef1f5;margin:24px;line-height:1.4}
h1,h2{margin:0 0 12px}button,input{font:inherit}button{background:#303a49;color:white;border:1px solid #65738a;border-radius:6px;padding:6px 10px;cursor:pointer}
.controls{display:flex;align-items:center;gap:12px;margin:16px 0}.controls input{flex:1}.images{display:grid;grid-template-columns:1fr 1fr;gap:14px}
figure{margin:0;background:#242b36;padding:10px;border-radius:8px;min-width:0}figcaption{margin-bottom:8px;font-weight:600}
img{display:block;width:100%;height:auto;max-height:60vh;object-fit:contain;background:#0b0d12}.empty{min-height:220px;display:grid;place-items:center;color:#aab5c7}
pre{white-space:pre-wrap;background:#242b36;padding:12px;border-radius:8px;max-height:220px;overflow:auto}
.timeline{max-height:340px;overflow:auto;display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:4px}
.event{text-align:left;font-size:12px;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.Hype{border-left:3px solid #e5a85a}.candidate{border-left:3px solid #7fbfdb}
.note{color:#b9c3d1}a{color:#9bc7ff}
</style>
<h1>Session comparison</h1><p class="note">Frames stay at their recorded elapsed times. Pixel differences are diagnostic until both render associations are verified.</p>
<p id="first-summary"></p>
<div class="controls"><button id="previous">Previous</button><input id="frame" type="range" min="0" value="0"><button id="next">Next</button><button id="first">First divergence</button><span id="counter"></span></div>
<div class="images"><figure><figcaption id="source-label"></figcaption><div id="source"></div></figure><figure><figcaption id="candidate-label"></figcaption><div id="candidate"></div></figure></div>
<pre id="details"></pre><p><a id="heatmap" href="first-diagnostic-contact.png">First diagnostic contact sheet</a> · <a href="report.json">Full JSON report</a></p>
<h2>Input, state and capture timeline</h2><div id="timeline" class="timeline"></div>
<script>
const data = __PAYLOAD__;
document.getElementById('source-label').textContent=data.source_label;
document.getElementById('candidate-label').textContent=data.candidate_label;
document.getElementById('first-summary').textContent=data.first_divergence?`First divergence: ${(data.first_divergence.elapsed_micros/1e6).toFixed(3)}s · ${data.first_divergence.stream} · ${data.first_divergence.status}`:'No classified divergence in the selected streams';
const scrub = document.getElementById('frame'); scrub.max = Math.max(0,data.frames.length-1);
const frameIndex = new Map(data.frames.map((row,index)=>[row.bucket,index]));
function nearest(bucket){if(frameIndex.has(bucket))return frameIndex.get(bucket);let best=0,distance=Infinity;data.frames.forEach((row,index)=>{const gap=Math.abs(row.bucket-bucket);if(gap<distance){best=index;distance=gap;}});return best;}
function image(target,path){const box=document.getElementById(target);box.replaceChildren();if(!path){box.className='empty';box.textContent='No frame in this time bucket';return;}box.className='';const img=document.createElement('img');img.src=path;img.alt=target+' frame';box.appendChild(img);}
function show(index){index=Math.max(0,Math.min(data.frames.length-1,index));scrub.value=index;const row=data.frames[index];if(!row)return;image('source',row.source);image('candidate',row.candidate);document.getElementById('counter').textContent=`${index+1}/${data.frames.length} · ${(row.bucket_window_micros[0]/1e6).toFixed(3)}s · ${row.status}`;document.getElementById('details').textContent=JSON.stringify({status:row.status,window_micros:row.bucket_window_micros,source_interval_micros:row.source_interval_micros,candidate_interval_micros:row.candidate_interval_micros,midpoint_delta_micros:row.midpoint_delta_micros,maximum_possible_acquisition_delta_micros:row.maximum_possible_acquisition_delta_micros,source_size:row.source_size,candidate_size:row.candidate_size,pixel_metrics:row.pixel_metrics,pixel_claim:row.pixel_claim},null,2);}
scrub.oninput=()=>show(Number(scrub.value));document.getElementById('previous').onclick=()=>show(Number(scrub.value)-1);document.getElementById('next').onclick=()=>show(Number(scrub.value)+1);
document.getElementById('first').onclick=()=>{const first=data.first_divergence;if(first){const bucket=first.bucket??Math.floor(first.elapsed_micros/(data.policy.bucket_ms*1000));show(nearest(bucket));}};
for(const event of data.timeline){const button=document.createElement('button');button.className='event '+(event.backend===data.source_label?'Hype':'candidate');button.title=`${event.backend} ${event.kind} at ${(event.elapsed_micros/1e6).toFixed(3)}s`;button.textContent=`${(event.elapsed_micros/1e6).toFixed(3)}s ${event.backend}: ${event.kind} ${event.summary}`;button.onclick=()=>show(nearest(event.frame_bucket));document.getElementById('timeline').appendChild(button);}
if(data.first_pixel===null)document.getElementById('heatmap').hidden=true;
show(0);
</script></html>""".replace("__PAYLOAD__", payload)
    (output / "viewer.html").write_text(page)


def compare(
    source: Path,
    candidate: Path,
    output: Path,
    bucket_ms: int = 250,
    timing_tolerance_ms: int = 50,
    channel_threshold: int = 8,
    max_mae: float = 4.0,
    max_rmse: float = 12.0,
    max_fraction: float = 0.05,
    masks: list[tuple[str, int, int, int, int, str]] | None = None,
    state_tolerance_ms: int = 250,
    actor_position_tolerance: float = 0.05,
) -> dict:
    if (min(bucket_ms, timing_tolerance_ms, state_tolerance_ms) < 0
            or bucket_ms == 0 or actor_position_tolerance < 0
            or not 0 <= channel_threshold <= 255):
        raise ValueError("invalid timing or channel threshold")
    masks = masks or []
    bucket_us = bucket_ms * 1000
    source_manifest, source_events, source_frames = read_recording(source, bucket_us)
    candidate_manifest, candidate_events, candidate_frames = read_recording(candidate, bucket_us)
    source_video_segments = read_video_segments(source.resolve(), source_events)
    candidate_video_segments = read_video_segments(candidate.resolve(), candidate_events)
    candidate_godot = candidate_manifest.get("driver") == "godot_replay"
    source_godot = source_manifest.get("driver") == "godot_replay"
    source_label = "Godot source" if source_godot else "Hype source"
    candidate_label = "Godot replay" if candidate_godot else "Hype replay"
    source_states = state_samples(source_events, source_godot)
    candidate_states = state_samples(candidate_events, candidate_godot)
    source_actors = actor_samples(source_events, source_godot)
    candidate_actors = actor_samples(candidate_events, candidate_godot)
    source_inputs, source_input_stage = input_samples(source_events)
    candidate_inputs, candidate_input_stage = input_samples(candidate_events)
    output.mkdir(parents=True, exist_ok=False)
    frame_rows = []
    divergences = []
    first_diagnostic_pixel_difference = None
    input_rows = []
    for index in range(max(len(source_inputs), len(candidate_inputs))):
        original = source_inputs[index] if index < len(source_inputs) else None
        target = candidate_inputs[index] if index < len(candidate_inputs) else None
        if original is None or target is None:
            status = "missing_input_edge"
        elif any(original[field] != target[field] for field in ("code", "edge", "repeat")):
            status = "input_identity_mismatch"
        elif (original["keysym"] is not None and target["keysym"] is not None
              and original["keysym"] != target["keysym"]):
            status = "input_identity_mismatch"
        elif (original["modifiers"] is not None and target["modifiers"] is not None
              and original["modifiers"] != target["modifiers"]):
            status = "input_modifier_mismatch"
        elif (source_input_stage == candidate_input_stage == "server_observed"
              and abs(original["elapsed_micros"] - target["elapsed_micros"]) > timing_tolerance_ms * 1000):
            status = "input_timing_mismatch"
        elif source_input_stage == candidate_input_stage == "server_observed":
            status = "matched_server_edges"
        else:
            status = "matched_dispatch_only"
        times = [item["elapsed_micros"] for item in (original, target) if item is not None]
        row = {"index": index, "source": original, "candidate": target,
               "source_stage": source_input_stage, "candidate_stage": candidate_input_stage,
               "status": status, "observation_window_micros": [min(times), max(times)],
               "consumption_verified": False}
        input_rows.append(row)
        if status not in {"matched_server_edges", "matched_dispatch_only"}:
            divergences.append({"stream": "input", "elapsed_micros": min(times),
                                "observation_window_micros": row["observation_window_micros"],
                                "status": status, "index": index})
    for bucket in sorted(set(source_frames) | set(candidate_frames)):
        original = sorted(source_frames.get(bucket, []), key=lambda frame: frame.midpoint)
        target = sorted(candidate_frames.get(bucket, []), key=lambda frame: frame.midpoint)
        for pair_index in range(max(len(original), len(target))):
            a = original[pair_index] if pair_index < len(original) else None
            b = target[pair_index] if pair_index < len(target) else None
            row = {"bucket": bucket, "pair_index_in_bucket": pair_index,
                   "source_count": len(original), "candidate_count": len(target),
                   "bucket_window_micros": [bucket * bucket_us, (bucket + 1) * bucket_us],
                   "source_intervals_micros": [[frame.start, frame.end] for frame in original],
                   "candidate_intervals_micros": [[frame.start, frame.end] for frame in target],
                   "source": str(a.path) if a else None, "candidate": str(b.path) if b else None,
                   "source_interval_micros": [a.start, a.end] if a else None,
                   "candidate_interval_micros": [b.start, b.end] if b else None,
                   "source_midpoint_micros": a.midpoint if a else None,
                   "candidate_midpoint_micros": b.midpoint if b else None,
                   "source_size": [a.width, a.height] if a else None,
                   "candidate_size": [b.width, b.height] if b else None}
            at = min((frame.midpoint for frame in (a, b) if frame is not None),
                     default=bucket * bucket_us)
            if a is None or b is None:
                row["status"] = "missing_frame_checkpoint"
            else:
                minimum_gap = max(0, a.start - b.end, b.start - a.end)
                maximum_delta = max(abs(a.start - b.end), abs(a.end - b.start))
                row["acquisition_separation_micros"] = minimum_gap
                row["midpoint_delta_micros"] = abs(a.midpoint - b.midpoint)
                row["maximum_possible_acquisition_delta_micros"] = maximum_delta
                row["dimension_equal"] = (a.width, a.height) == (b.width, b.height)
                if minimum_gap > timing_tolerance_ms * 1000:
                    row["status"] = "timing_mismatch"
                elif maximum_delta > timing_tolerance_ms * 1000:
                    row["status"] = "timing_uncertain"
                elif not row["dimension_equal"]:
                    row["status"] = "dimension_mismatch"
                else:
                    row["status"] = "paired_diagnostic"
                    metrics, heat = pixel_metrics(a, b, masks, channel_threshold)
                    row["pixel_metrics"] = metrics
                    row["pixel_thresholds_met"] = (
                        metrics["mae_8bit"] <= max_mae
                        and metrics["rmse_8bit"] <= max_rmse
                        and metrics["fraction_pixels_over_threshold"] <= max_fraction
                    )
                    row["pixel_claim"] = "unqualified: render/update association is unverified"
                    if not row["pixel_thresholds_met"] and first_diagnostic_pixel_difference is None:
                        first_diagnostic_pixel_difference = bucket
                        Image.fromarray(heat).save(output / "first-diagnostic-heatmap.png")
                        with Image.open(a.path) as ref_image, Image.open(b.path) as cand_image:
                            contact = Image.new("RGB", (a.width * 3, a.height))
                            contact.paste(ref_image.convert("RGB"), (0, 0))
                            contact.paste(cand_image.convert("RGB"), (a.width, 0))
                            contact.paste(Image.fromarray(heat), (a.width * 2, 0))
                            contact.save(output / "first-diagnostic-contact.png")
            if row["status"] != "paired_diagnostic":
                divergences.append({"stream": "video", "bucket": bucket,
                                    "elapsed_micros": at, "status": row["status"],
                                    "pair_index_in_bucket": pair_index,
                                    "observation_window_micros": row["bucket_window_micros"]})
            frame_rows.append(row)
    state_rows = []
    for original, target in pair_samples(source_states, candidate_states, state_tolerance_ms * 1000):
        if original is None or target is None:
            status = "missing_state"
        elif original["phase"] != target["phase"] or original["level"] != target["level"]:
            status = "state_mismatch"
        else:
            status = "matched_selected_state"
        observed = [item["elapsed_micros"] for item in (original, target) if item is not None]
        row = {"source": original, "candidate": target, "status": status,
               "observation_window_micros": [min(observed), max(observed)],
               "sample_delta_micros": abs(observed[0] - observed[1]) if len(observed) == 2 else None}
        state_rows.append(row)
        if status != "matched_selected_state":
            divergences.append({"stream": "state", "elapsed_micros": min(observed),
                                "observation_window_micros": row["observation_window_micros"],
                                "status": status})
    actor_rows = []
    for original, target in pair_samples(source_actors, candidate_actors, state_tolerance_ms * 1000):
        compared = {}
        if original is None or target is None:
            status = "missing_actor_sample"
        else:
            left = original["translation"]
            right = target["translation"]
            if (isinstance(left, list) and isinstance(right, list)
                    and len(left) == len(right) == 3):
                compared["translation_distance"] = math.dist(left, right)
            if not source_godot and not candidate_godot:
                for field in ("current_action", "secondary_action", "normal_behavior", "reflex_behavior"):
                    if original[field] is not None and target[field] is not None:
                        compared[field] = {"source": original[field], "candidate": target[field],
                                           "equal": original[field] == target[field]}
            mismatch = (compared.get("translation_distance", 0) > actor_position_tolerance
                        or any(not item["equal"] for item in compared.values() if isinstance(item, dict)))
            status = "actor_sample_mismatch" if mismatch else "matched_selected_actor_sample"
        observed = [item["elapsed_micros"] for item in (original, target) if item is not None]
        row = {"source": original, "candidate": target, "status": status,
               "compared": compared, "observation_window_micros": [min(observed), max(observed)],
               "claim": "diagnostic: samples are not qualified completed-update observations"}
        actor_rows.append(row)
        if status != "matched_selected_actor_sample":
            divergences.append({"stream": "actor", "elapsed_micros": min(observed),
                                "observation_window_micros": row["observation_window_micros"],
                                "status": status})
    video_comparison = None
    if source_video_segments and candidate_video_segments:
        video_comparison = compare_video_streams(
            source_video_segments, candidate_video_segments, 15_000, output
        )
        divergences.extend(video_comparison["divergences"])
    elif source_video_segments or candidate_video_segments:
        missing_side = "candidate" if source_video_segments else "source"
        present = source_video_segments or candidate_video_segments
        video_comparison = {
            "status": f"continuous_video_missing_on_{missing_side}",
            "source_segments": [segment.relative_path for segment in source_video_segments],
            "candidate_segments": [segment.relative_path for segment in candidate_video_segments],
            "association_limit": "lossless presented-window samples; game render/update association is unverified",
            "gaps": {"unpaired_frames": [], "source_timing_gaps": [], "candidate_timing_gaps": []},
            "divergences": [{"stream": "video", "elapsed_micros": present[0].frames[0].elapsed_us,
                             "status": f"continuous_video_missing_on_{missing_side}"}],
        }
        divergences.extend(video_comparison["divergences"])
    first_divergence = min(divergences, key=lambda item: item["elapsed_micros"], default=None)
    report = {
        "schema": 1,
        "source": str(source.resolve()), "candidate": str(candidate.resolve()),
        "source_manifest_sha256": hashlib.sha256((source / "manifest.json").read_bytes()).hexdigest(),
        "candidate_manifest_sha256": hashlib.sha256((candidate / "manifest.json").read_bytes()).hexdigest(),
        "policy": {"bucket_ms": bucket_ms, "timing_tolerance_ms": timing_tolerance_ms,
                   "state_sample_tolerance_ms": state_tolerance_ms,
                   "actor_position_tolerance": actor_position_tolerance,
                   "channel_threshold": channel_threshold, "max_mae": max_mae,
                   "max_rmse": max_rmse, "max_fraction": max_fraction,
                   "frame_selection": "all checkpoints paired by acquisition midpoint order within the same fixed elapsed-time bucket; unmatched checkpoints retained; no cross-bucket or nearest-frame search",
                   "continuous_video": "decode every indexed frame; pair only within the same fixed 33.333 ms elapsed-time bucket and within 15 ms; no time shift, crop, or resize",
                   "frame_timing": "only pair as timed when every possible acquisition time within both recorded intervals differs by at most the tolerance",
                   "state_pairing": "chronological one-to-one samples within the declared elapsed-time tolerance; all samples retained",
                   "input_pairing": "ordered key edges; observed X-server times compared when both streams have them; dispatch-only matches do not prove delivery or consumption",
                   "pixel_claim": "diagnostic only while render/update association is unverified"},
        "first_divergence": first_divergence,
        "first_diagnostic_pixel_difference": first_diagnostic_pixel_difference,
        "frames": frame_rows, "video": video_comparison,
        "video_fallback": "sparse PNG acquisition checkpoints are retained above; continuous comparison requires indexed video segments on both runs",
        "inputs": input_rows, "states": state_rows, "actors": actor_rows,
    }
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    write_viewer(output, report, source_events, candidate_events, bucket_us,
                 source_label, candidate_label)
    return report


def parse_mask(value: str) -> tuple[str, int, int, int, int, str]:
    try:
        name, bounds, reason = value.split(":", 2)
        coordinates = [int(part) for part in bounds.split(",")]
    except ValueError as error:
        raise argparse.ArgumentTypeError("mask requires NAME:X,Y,W,H:REASON") from error
    if not name or not reason or len(coordinates) != 4:
        raise argparse.ArgumentTypeError("mask requires NAME:X,Y,W,H:REASON")
    return (name, *coordinates, reason)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--bucket-ms", type=int, default=250)
    parser.add_argument("--timing-tolerance-ms", type=int, default=50)
    parser.add_argument("--channel-threshold", type=int, default=8)
    parser.add_argument("--max-mae", type=float, default=4.0)
    parser.add_argument("--max-rmse", type=float, default=12.0)
    parser.add_argument("--max-fraction", type=float, default=0.05)
    parser.add_argument("--state-tolerance-ms", type=int, default=250)
    parser.add_argument("--actor-position-tolerance", type=float, default=0.05)
    parser.add_argument("--mask", action="append", type=parse_mask, default=[])
    args = parser.parse_args()
    report = compare(args.source, args.candidate, args.output, args.bucket_ms,
                     args.timing_tolerance_ms, args.channel_threshold,
                     args.max_mae, args.max_rmse, args.max_fraction, args.mask,
                     args.state_tolerance_ms, args.actor_position_tolerance)
    print(json.dumps({"first_divergence": report["first_divergence"],
                      "first_diagnostic_pixel_difference": report["first_diagnostic_pixel_difference"],
                      "frame_buckets": len(report["frames"]), "state_samples": len(report["states"]),
                      "actor_samples": len(report["actors"])}, indent=2))


if __name__ == "__main__":
    main()
