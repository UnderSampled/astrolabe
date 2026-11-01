import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import importlib.util

if importlib.util.find_spec('numpy') is None:
    raise unittest.SkipTest('tools/compare_recordings.py needs numpy, which this Python lacks')

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from compare_recordings import compare, timeline_event  # noqa: E402


def recording(root: Path, frames: list[tuple[int, tuple[int, int, int], tuple[int, int] | None]], phase="menu") -> Path:
    root.mkdir()
    (root / "frames").mkdir()
    (root / "manifest.json").write_text(json.dumps({"schema": 1, "ending": "completed", "driver": "scripted"}))
    events = []
    for index, (time_us, color, size) in enumerate(frames):
        size = size or (4, 4)
        name = f"frames/{index:06}.png"
        Image.new("RGB", size, color).save(root / name)
        events.append({"sequence": len(events), "elapsed_micros": time_us, "kind": "frame", "data": {
            "path": name, "width": size[0], "height": size[1],
            "request_start_micros": time_us, "reply_end_micros": time_us + 1000,
            "association": "unverified",
        }})
    events.append({"sequence": len(events), "elapsed_micros": 1_000_000, "kind": "state", "data": {
        "state": {"phase": phase, "level": ""},
    }})
    (root / "events.jsonl").write_text("".join(json.dumps(row) + "\n" for row in events))
    return root


def append_event(root: Path, at_us: int, kind: str, data: dict) -> None:
    path = root / "events.jsonl"
    events = [json.loads(line) for line in path.read_text().splitlines()]
    events.append({"sequence": len(events), "elapsed_micros": at_us, "kind": kind, "data": data})
    path.write_text("".join(json.dumps(row) + "\n" for row in events))


def set_frame_interval(root: Path, start_us: int, end_us: int) -> None:
    path = root / "events.jsonl"
    events = [json.loads(line) for line in path.read_text().splitlines()]
    frame = next(event for event in events if event["kind"] == "frame")
    frame["data"]["request_start_micros"] = start_us
    frame["data"]["reply_end_micros"] = end_us
    path.write_text("".join(json.dumps(row) + "\n" for row in events))


def add_video(root: Path, frames: list[tuple[int, tuple[int, int, int, int], tuple[int, int]]],
              segment: int = 0) -> None:
    if not shutil.which("ffmpeg"):
        raise unittest.SkipTest("ffmpeg is required for continuous-video comparator tests")
    video = root / "video"
    video.mkdir(exist_ok=True)
    width, height = frames[0][2]
    raw = b"".join(bytes(color) * (width * height) for _, color, size in frames
                   if size == (width, height))
    if len(raw) != len(frames) * width * height * 4:
        raise ValueError("synthetic video segment must have one native resolution")
    segment_name = f"{segment:06}.mkv"
    path = video / segment_name
    subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-f", "rawvideo",
                    "-pixel_format", "rgba", "-video_size", f"{width}x{height}",
                    "-framerate", "30", "-i", "pipe:0", "-frames:v", str(len(frames)),
                    "-c:v", "ffv1", "-level", "3", str(path)], input=raw, check=True)
    probe = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
                            "frame=best_effort_timestamp_time", "-of", "csv=p=0", str(path)],
                           capture_output=True, text=True, check=True)
    pts = [int(round(float(line.strip()) * 1_000_000)) for line in probe.stdout.splitlines() if line.strip()]
    index_name = f"{segment:06}.frames.jsonl"
    index_path = video / index_name
    with index_path.open("w") as index:
        for ordinal, (elapsed, _, size) in enumerate(frames):
            index.write(json.dumps({"index": ordinal, "path": f"video/{segment_name}",
                                    "pts_media_micros": pts[ordinal],
                                    "pts_unix_micros": pts[ordinal] + 1_790_000_000_000_000,
                                    "elapsed_micros": elapsed, "width": size[0], "height": size[1],
                                    "association": "presented_window_sample; game_render_update_unverified"}) + "\n")
    append_event(root, frames[-1][0], "video_stream_end", {
        "path": f"video/{segment_name}", "frame_index_path": f"video/{index_name}",
        "frames": len(frames), "width": width, "height": height,
    })


class FrameComparisonTest(unittest.TestCase):
    def test_continuous_video_identical_and_one_differing_frame(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [])
            same = recording(root / "same", [])
            changed = recording(root / "changed", [])
            samples = [(1_000_000 + i * 33_333, (10, 20, 30, 255), (2, 2)) for i in range(3)]
            add_video(source, samples)
            add_video(same, samples)
            altered = samples.copy()
            altered[1] = (altered[1][0], (255, 0, 0, 255), altered[1][2])
            add_video(changed, altered)
            report = compare(source, same, root / "same-report")
            self.assertEqual(len(report["video"]["frames"]), 3)
            self.assertTrue(all(row["status"] == "video_frame_compared" for row in report["video"]["frames"]))
            self.assertIsNone(report["video"]["first_diagnostic_video_pixel_difference"])
            report = compare(source, changed, root / "changed-report")
            self.assertEqual(report["video"]["first_diagnostic_video_pixel_difference"]["source_index"], 1)
            self.assertEqual(report["first_divergence"], None)
            self.assertEqual(report["video"]["frames"][1]["pixel_claim"], "diagnostic_unassociated")
            self.assertIn("association is unverified", report["video"]["association_limit"])

    def test_continuous_video_rejects_corrupt_index_and_reports_missing_frame_gap(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [])
            candidate = recording(root / "candidate", [])
            samples = [(1_000_000 + i * 33_333, (0, 0, 0, 255), (2, 2)) for i in range(3)]
            add_video(source, samples)
            add_video(candidate, [samples[0], samples[2]])
            report = compare(source, candidate, root / "gap-report")
            self.assertEqual(report["video"]["gaps"]["unpaired_frames"][0]["source_index"], 1)
            self.assertEqual(report["video"]["gaps"]["unpaired_frames"][0]["status"], "missing_video_frame")
            self.assertTrue(report["video"]["gaps"]["candidate_timing_gaps"])
            broken = recording(root / "broken", [])
            add_video(broken, samples)
            index = broken / "video/000000.frames.jsonl"
            rows = index.read_text().splitlines()
            row = json.loads(rows[1])
            row["index"] = 9
            rows[1] = json.dumps(row)
            index.write_text("\n".join(rows) + "\n")
            with self.assertRaisesRegex(ValueError, "index order/count gap"):
                compare(source, broken, root / "broken-report")

    def test_continuous_video_keeps_native_resolution_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [])
            candidate = recording(root / "candidate", [])
            add_video(source, [(1_000_000, (0, 0, 0, 255), (2, 2))])
            add_video(candidate, [(1_000_000, (0, 0, 0, 255), (3, 2))])
            report = compare(source, candidate, root / "report")
            self.assertEqual(report["video"]["frames"][0]["status"], "video_dimension_mismatch")
            self.assertEqual(report["video"]["frames"][0]["source_size"], [2, 2])
            self.assertEqual(report["video"]["frames"][0]["candidate_size"], [3, 2])

    def test_overlapping_resize_segments_preserve_raw_evidence_and_prefer_newer_frames(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [])
            candidate = recording(root / "candidate", [])
            old_times = [1_000_000, 1_033_333, 1_066_666, 1_099_999]
            new_times = [1_066_666, 1_099_999, 1_133_332]
            add_video(source, [(t, color, (2, 2)) for t, color in zip(
                old_times, [(0, 255, 0, 255), (0, 255, 0, 255), (255, 0, 0, 255), (255, 0, 0, 255)])], 0)
            add_video(source, [(t, (0, 255, 0, 255), (3, 2)) for t in new_times], 1)
            add_video(candidate, [(t, color, (2, 2)) for t, color in zip(
                old_times, [(0, 255, 0, 255), (0, 255, 0, 255), (0, 0, 255, 255), (0, 0, 255, 255)])], 0)
            add_video(candidate, [(t, (0, 255, 0, 255), (3, 2)) for t in new_times], 1)
            report = compare(source, candidate, root / "report")
            self.assertEqual([segment["frames"] for segment in report["video"]["source_segments"]], [4, 3])
            self.assertEqual(report["video"]["source_overlaps"][0]["overlap_elapsed_micros"],
                             [1_066_666, 1_099_999])
            self.assertEqual(report["video"]["source_overlaps"][0]["older_resolution"], [2, 2])
            self.assertEqual(report["video"]["source_overlaps"][0]["newer_resolution"], [3, 2])
            self.assertEqual(report["video"]["source_overlaps"][0]["older_frames_superseded"], 2)
            self.assertEqual(report["video"]["source_segments"][0]["comparison_frames_superseded_by_next_segment"], 2)
            self.assertTrue(all(frame["status"] == "video_frame_compared" for frame in report["video"]["frames"]))
            self.assertIsNone(report["video"]["first_diagnostic_video_pixel_difference"])

    def test_media_pts_must_match_index_when_present(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [])
            candidate = recording(root / "candidate", [])
            add_video(source, [(1_000_000, (0, 0, 0, 255), (2, 2))])
            add_video(candidate, [(1_000_000, (0, 0, 0, 255), (2, 2))])
            index = candidate / "video/000000.frames.jsonl"
            row = json.loads(index.read_text())
            row["pts_media_micros"] += 10_000
            index.write_text(json.dumps(row) + "\n")
            with self.assertRaisesRegex(ValueError, "media PTS differs from index"):
                compare(source, candidate, root / "report")

    def test_input_edges_distinguish_observation_dispatch_and_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [(1_000_000, (0, 0, 0), None)])
            observed = recording(root / "observed", [(1_000_000, (0, 0, 0), None)])
            dispatched = recording(root / "dispatched", [(1_000_000, (0, 0, 0), None)])
            changed = recording(root / "changed", [(1_000_000, (0, 0, 0), None)])
            append_event(source, 1_100_000, "input", {"code": 116, "keysym": 65364,
                         "edge": "press", "repeat": False, "observed_micros": 1_100_000})
            append_event(observed, 1_110_000, "input_observed", {"code": 116,
                         "keysym": 65364, "edge": "press", "repeat": False,
                         "observed_micros": 1_110_000})
            append_event(dispatched, 1_110_000, "input_dispatch", {"source_code": 116,
                         "source_keysym": 65364, "edge": "press", "repeat": False})
            append_event(changed, 1_110_000, "input_observed", {"code": 36,
                         "keysym": 65293, "edge": "press", "repeat": False,
                         "observed_micros": 1_110_000})
            self.assertEqual(compare(source, observed, root / "observed-report")["inputs"][0]["status"],
                             "matched_server_edges")
            self.assertEqual(compare(source, dispatched, root / "dispatch-report")["inputs"][0]["status"],
                             "matched_dispatch_only")
            mismatch = compare(source, changed, root / "changed-report")
            self.assertEqual(mismatch["inputs"][0]["status"], "input_identity_mismatch")
            self.assertEqual(mismatch["first_divergence"]["stream"], "input")

    def test_timeline_shows_stable_save_changes_and_session_failure(self):
        saved = timeline_event({"kind": "save_snapshot", "elapsed_micros": 1_250_000,
                                "data": {"saves": {"files": [{"path": "slot.sav"}]}}},
                               "Hype", 250_000)
        self.assertEqual(saved["frame_bucket"], 5)
        self.assertIn("1 files", saved["summary"])
        failed = timeline_event({"kind": "session_failure", "elapsed_micros": 2_000_000,
                                 "data": {"error": "input tap stopped"}}, "Hype", 250_000)
        self.assertIn("input tap stopped", failed["summary"])
        gate = timeline_event({"kind": "navigation_wait_end", "elapsed_micros": 2_100_000,
                               "data": {"gate": "load_game_selected", "added_delay_micros": 67_000}},
                              "Hype", 250_000)
        self.assertIn("load_game_selected ready · +67.0 ms", gate["summary"])
        self.assertEqual(gate["frame_bucket"], 8)

    def test_same_bucket_exact_pixels_and_one_bucket_late_negative_control(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [(1_000_000, (10, 20, 30), None)])
            same = recording(root / "same", [(1_010_000, (10, 20, 30), None)])
            report = compare(source, same, root / "same-report")
            self.assertIsNone(report["first_divergence"])
            self.assertEqual(report["frames"][0]["pixel_metrics"]["mae_8bit"], 0)
            viewer = (root / "same-report/viewer.html").read_text()
            self.assertIn("Input, state and capture timeline", viewer)
            self.assertIn("../source/frames/000000.png", viewer)
            late = recording(root / "late", [(1_260_000, (10, 20, 30), None)])
            report = compare(source, late, root / "late-report")
            self.assertEqual(report["first_divergence"]["status"], "missing_frame_checkpoint")
            late_viewer = (root / "late-report/viewer.html").read_text()
            self.assertIn("../source/frames/000000.png", late_viewer)
            self.assertIn("../late/frames/000000.png", late_viewer)

    def test_same_bucket_checkpoint_duplicates_pair_in_order_and_retain_unmatched(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [
                (1_000_000, (10, 20, 30), None),
                (1_100_000, (40, 50, 60), None),
            ])
            identical = recording(root / "identical", [
                (1_010_000, (10, 20, 30), None),
                (1_110_000, (40, 50, 60), None),
            ])
            self_report = compare(source, identical, root / "self-report")
            self.assertEqual(len(self_report["frames"]), 2)
            self.assertTrue(all(row["status"] == "paired_diagnostic" for row in self_report["frames"]))
            self.assertIsNone(self_report["first_divergence"])
            missing = recording(root / "missing", [(1_010_000, (10, 20, 30), None)])
            report = compare(source, missing, root / "missing-report")
            self.assertEqual(len(report["frames"]), 2)
            self.assertEqual([row["pair_index_in_bucket"] for row in report["frames"]], [0, 1])
            self.assertEqual([row["status"] for row in report["frames"]],
                             ["paired_diagnostic", "missing_frame_checkpoint"])
            self.assertEqual(report["frames"][1]["source_count"], 2)
            self.assertEqual(report["frames"][1]["candidate_count"], 1)
            self.assertIsNotNone(report["frames"][1]["source"])
            self.assertIsNone(report["frames"][1]["candidate"])

    def test_pixel_difference_is_diagnostic_and_wrong_size_is_divergence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [(1_000_000, (0, 0, 0), None)])
            changed = recording(root / "changed", [(1_000_000, (255, 0, 0), None)])
            report = compare(source, changed, root / "changed-report")
            self.assertIsNone(report["first_divergence"])
            self.assertEqual(report["first_diagnostic_pixel_difference"], 4)
            self.assertTrue((root / "changed-report/first-diagnostic-heatmap.png").is_file())
            wrong_size = recording(root / "wrong", [(1_000_000, (0, 0, 0), (5, 4))])
            report = compare(source, wrong_size, root / "wrong-report")
            self.assertEqual(report["first_divergence"]["status"], "dimension_mismatch")

    def test_acquisition_timing_mismatch_even_in_same_bucket(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [(1_000_000, (0, 0, 0), None)])
            candidate = recording(root / "candidate", [(1_110_000, (0, 0, 0), None)])
            report = compare(source, candidate, root / "report", timing_tolerance_ms=50)
            self.assertEqual(report["first_divergence"]["status"], "timing_mismatch")

    def test_overlapping_broad_acquisition_intervals_cannot_pass_timing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [(1_000_000, (0, 0, 0), None)])
            candidate = recording(root / "candidate", [(1_000_000, (0, 0, 0), None)])
            set_frame_interval(source, 1_000_000, 1_200_000)
            set_frame_interval(candidate, 1_190_000, 1_390_000)
            report = compare(source, candidate, root / "report", bucket_ms=500,
                             timing_tolerance_ms=50)
            self.assertEqual(report["frames"][0]["status"], "timing_uncertain")
            self.assertEqual(report["frames"][0]["midpoint_delta_micros"], 190_000)

    def test_state_transition_inside_bucket_and_actor_change_are_not_hidden(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [(1_000_000, (0, 0, 0), None)])
            candidate = recording(root / "candidate", [(1_000_000, (0, 0, 0), None)])
            append_event(source, 1_100_000, "state", {"state": {"phase": "gameplay", "level": "manoir"}})
            append_event(candidate, 1_100_000, "state", {"state": {"phase": "menu", "level": ""}})
            append_event(source, 1_200_000, "actor", {"actor": {
                "global_transform": {"Affine": {"translation": [1.0, 0.0, 0.0]}},
                "current_action": 47, "secondary_action": 47}})
            append_event(candidate, 1_200_000, "actor", {"actor": {
                "global_transform": {"Affine": {"translation": [99.0, 0.0, 0.0]}},
                "current_action": 47, "secondary_action": 47}})
            report = compare(source, candidate, root / "report")
            self.assertEqual([row["status"] for row in report["states"]],
                             ["matched_selected_state", "state_mismatch"])
            self.assertEqual(report["actors"][0]["status"], "actor_sample_mismatch")
            self.assertEqual(report["first_divergence"]["elapsed_micros"], 1_100_000)

    def test_state_samples_across_bucket_edge_pair_by_observed_time(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = recording(root / "source", [(1_000_000, (0, 0, 0), None)])
            candidate = recording(root / "candidate", [(1_000_000, (0, 0, 0), None)])
            append_event(source, 1_490_000, "state", {"state": {"phase": "gameplay", "level": "manoir"}})
            append_event(candidate, 1_510_000, "state", {"state": {"phase": "gameplay", "level": "manoir"}})
            report = compare(source, candidate, root / "report")
            self.assertEqual([row["status"] for row in report["states"]],
                             ["matched_selected_state", "matched_selected_state"])
            self.assertEqual(report["states"][1]["sample_delta_micros"], 20_000)


if __name__ == "__main__":
    unittest.main()
