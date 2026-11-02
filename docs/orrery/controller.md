# Orrery OpenSpace runtime analysis

Orrery is Astrolabe's optional native Rust runtime analysis tool for OpenSpace,
initially the private original Hype installation. It provides inspection and
control through library, CLI and MCP interfaces. The Cargo package, library, CLI binary, MCP server identity,
and Gamescope application identity are all named `orrery`. The converter does
not depend on Orrery. The [[docs/orrery/comparison-model|comparison evidence model]] defines which claims its observations can support.

Recording/replay extends Orrery's runtime analysis capabilities. The behavior
test suite is a separate consumer of Orrery, including for OpenSpace-only
scenarios; Godot has a separate harness. The suite owns gameplay assertions and
run-to-run comparison. See [session recordings](../astrolabe/session-recordings.md)
for their shared evidence contract, including available save files captured
before process launch. The shared case definitions and BDD runner now live in
`crates/behavior-contract` and `crates/behavior-suite`; Orrery's own tool tests
remain with it. The CLI has a first launch-to-exit session recorder and literal
elapsed-time replay. General AI/API tracing remains implementation work.

The Python `hype` MCP bridge remains available for optional diagnosis. Orrery
implements its control tools and read-only typed-inspection tools.
The controller exposes game launch, save loading, physical input, selected actor
inspection, screenshots, and cleanup. A dated operation record is linked below.
`inspect_main_actor` pauses the owned game and invokes Astrolabe's shared
OpenSpace reader; Orrery does not maintain a second record decoder. Its result
is a `StoppedBytes` capture with executable/save/process/capture provenance and
capture-local locators. It is not a completed-update observation and must not be
used as frame-consumption evidence. With no arguments the tool preserves its
original action-only response. Supplying `designer_values` (zero to 64 unique
indices) opts into the shared typed AI view: normal/reflex behavior selections
plus only those current designer-memory values. An empty list requests behavior
selections without designer values.

The service binds only `127.0.0.1`, requires one exact bearer token from
an owner-owned mode-0600 file, checks `Host` and optional `Origin`, limits HTTP
bodies to 128 KiB, and serializes controller operations. It owns one private
Gamescope and Wine process tree. Every live-memory operation rechecks the pinned
PID/start-time identity and executable/private-prefix guards. Interactive input
uses XTest on Gamescope's private Xwayland display; Orrery does not write Hype's
keyboard buffer or patch its keyboard callback.

The MCP transport uses `rmcp` 3.4.0, the official Rust SDK maintained by the
Model Context Protocol project. Its Streamable HTTP service owns protocol
negotiation, the initialize/initialized lifecycle, session IDs, notification
responses, and content negotiation; Orrery layers its local security policy and
native controller behind that service.

## Build and run

The prepared installation remains at the sibling harness path;
its game data and token stay untracked:

```sh
cargo build -p orrery --release --locked --offline
install -m 600 /dev/null /home/stephen/Projects/hype-harness/local/orrery-mcp-token
openssl rand -hex 32 > /home/stephen/Projects/hype-harness/local/orrery-mcp-token
target/release/orrery mcp \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --token-file /home/stephen/Projects/hype-harness/local/orrery-mcp-token \
  --runs /home/stephen/Projects/hype-harness/local/orrery-runs \
  --port 18766
```

The endpoint is `http://127.0.0.1:18766/mcp`. The token file must belong to the
current user and have no group/other permission bits. The server starts without
launching the game; `start_game` creates the private window and `stop_game`
removes the full owned process tree. Dropping or terminating the server also
cleans up its game session.

Restart the MCP service after rebuilding Orrery. An older service can keep
serving the previous binary and attempt to launch its now-deleted executable.
For the supplied English save, allow the title menu to settle, select
**Load game**, open the highlighted **Gogoud's Manoir I** slot, then verify phase 9
and level `manoir` before inspecting gameplay. Send and verify each menu key
separately when diagnosing a failed load; a successful input-delivery result
alone does not prove that the save loaded. For a saved-state behavior probe,
allow at least ten seconds of resumed game time after phase 9 appears, then
verify the saved inventory and weapon before applying an intervention. One
unverified menu sequence reached `manoir` with `selected_weapon=-1` and no
active entries. A screenshot-guided sequence selected **Load Game**, then
the highlighted **Gogoud's Manoir I** slot; after the settle, typed inspection
confirmed selected Weapon 80 at quantity zero and Hype action 47. Phase 9 and
level name alone must not be treated as proof that this slot loaded.
The verified menu setup uses each key for 55 ms followed by an 800 ms release
gap. A shorter gap can leave the next screenshot before the menu visibly
updates. The menu `press_keys` tool uses X11 chord names `Down` and `Return`;
the separate `physical_key_sequence` tool uses typed names `ArrowDown` and
`Enter`. Check each tool response for an error before sending the next menu
key.

After verification, this is the Codex entry to add manually. Replace the sample
value with the exact contents of the token file. Do not include angle brackets.

```toml
[mcp_servers.orrery]
url = "http://127.0.0.1:18766/mcp"
http_headers = { Authorization = "Bearer <64-hex-character-token>" }
```

The implementation deliberately does not edit `~/.codex/config.toml`.

## Session recording and replay

`record-session` arms the evidence files before launching the game. It copies the
verified private Wine installation with copy-on-write reflinks into the output
run, stages a complete snapshot of the selected save root, and launches only
that copy. `--save-root` may identify an existing save directory, an explicitly
empty one, or an absent path. If omitted, the installed game's complete
`GameData/SaveGame` directory is captured. An unreadable root is an error.
Orrery gives every owned Hype run a private PipeWire/PulseAudio server and a
null sink, so headless game sound is not sent to the desktop speakers. Full
recording and full replay arm `recording/audio.mka` before Wine launches; it
contains lossless FLAC from that sink's monitor. The `audio_stream_end` event
states the sample format, duration, launch-relative capture start, provenance,
signal check, and timing uncertainty. The private server and encoder are owned
by the run and removed at teardown. A missing or failed stream is a recording
failure or explicit coverage gap. This requires local `pipewire`,
`pipewire-pulse`, `wireplumber`, `pactl`, `ffmpeg`, and `ffprobe` executables.
For the supplied English fixture, pass the directory containing `SaveGame0/`
as `--save-root`. Orrery rejects a path to the inner slot directory before
launch because it would flatten the save layout.

```sh
cargo build --release --locked -p orrery
target/release/orrery record-session \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --output TestResults/my-hype-recording \
  --title "Manor route" --description "Load the manor slot and move left" \
  --tag movement --seconds 90 --frame-ms 125
target/release/orrery replay-session \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --recording TestResults/my-hype-recording/recording \
  --output TestResults/my-hype-replay
```

For a small recording that can be shared or committed, use `--mode input-only`:

```sh
target/release/orrery record-session \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --output TestResults/my-input-run \
  --save-root /path/to/save-root-containing-SaveGame0 \
  --title "Manor route" --seconds 90 --mode input-only
target/release/orrery replay-session \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --recording TestResults/my-input-run/recording \
  --save-root /path/to/save-root-containing-SaveGame0 \
  --output TestResults/my-full-recapture
```

Commit or share only `my-input-run/recording/`. It contains the manifest with
the initial save files' paths, sizes, hashes and modification times, plus the
timed game-window input events. It contains no save bytes, screenshots, video, audio,
state samples or output saves. The game's copied installation and temporary
fixture are removed after a successful input-only run. If the starting save
root was populated, full recapture requires `--save-root` pointing to a local
copy with exactly matching bytes and structure; Orrery restores the recorded
modification times in the isolated run copy. A byte or layout mismatch fails
before launching the game. For an absent or empty starting save root,
`--save-root` is optional. The normal `replay-session` command captures full
video, checkpoints, state and save evidence in its new output run. Its manifest
references the source recording hash. The source recording remains unchanged.

`godot-harness` replays a recording in the port through
`godot/tests/session_replay.gd`. Loading takes different times in each engine, so
menu inputs run from the port's menu being ready and later inputs from manoir
entry, taken from the recording's first phase 9 state. That needs a full
recording: an input-only one has no state samples and is refused. The port's
menu must then land where the original's did; check the first state samples of
both runs before reading further differences as gameplay.

Input-only replay dispatches recorded press/release state changes. X11
auto-repeat presses marked while a key was already held remain in its input
timeline but are not injected as replay commands. Recapture checks the
nonrepeat edge sequence and reports source and replay repeat counts separately.
Those counts can differ because the X server generates repeats according to
hold duration and scheduling. A matching edge sequence establishes delivery to
the game window, not gameplay consumption or identical menu behavior. Full
source recordings retain the existing strict all-event validation.

For a captured route whose first three presses are Down, Enter, Enter at the
Hype title/load menus, `replay-session --profile hype-title-load-v1` enables a
separate visual navigation profile. Before Down it requires two recent title
frames with New Game selected; before the first Enter it requires two Load Game
selected frames; before the second Enter it requires two Existing games frames.
The detector also checks phase 38. Each wait, qualifying evidence frame, result
and added delay is recorded as a `navigation_*` timeline event and summarized in
`delivery.json`. A six-second gate timeout stops later input and seals the
failed recording with the last frame/phase evidence. The profile checks screen
readiness; it does not verify that Hype consumed a key or preserve literal
startup timing. The default `literal` profile retains the original event times.
The visual thresholds are versioned for the captured 780×600 and 1024×768
English Hype menus; a different edition needs recalibration.

For human play on the user's Wayland desktop, add `--interactive` to a full
`record-session` command. This opens a Gamescope window while keeping Hype,
keyboard observation and video capture on the run's private Xwayland display.
The private game-only audio monitor is also forwarded to the desktop playback
sink with `parec` and `pacat`. It captures no microphone or desktop audio and
does not change host audio settings. Other launches and replay remain headless.
Interactive mode requires `--mode full` and omits `--input-script`; its driver
is human. Start it from the desktop environment with `WAYLAND_DISPLAY` and
`XDG_RUNTIME_DIR` available. Click the Gamescope window and wait for the
`interactive_ready` event/message before playing. The recorder observes the
physical keyboard events delivered to Hype's window through XRecord.

Full recording has a separate continuous-video budget:
`--video-budget-mib` accepts 64 through 65536 MiB and defaults to 2048 MiB.
The PNG checkpoint budget stays at 2 GiB; `--frame-ms 1000` reduces checkpoint
frequency without changing the lossless 30 Hz video. Choose an explicit budget
appropriate to the duration and available disk space. Budget exhaustion seals
the retained prefix with ending `storage_budget`, even when the duration remains.
For interactive recording, `interactive_ready` now also requires FFmpeg's first
encoded video packet and a nonempty video artifact. Audio and the keyboard tap
are already attached before that cue; the game window can appear earlier.

To finish and save the recording, create an empty `stop-request` file directly
in that run's output directory, for example
`touch TestResults/my-hype-recording/stop-request`. The recorder seals an
`interrupted` recording through normal video/audio finalization, captures the
output saves and cleans its owned processes. Ctrl+C in the recorder's terminal
also requests graceful finalization. Wait for `report.json` with
`cleanup_complete: true` before starting another run. The duration limit is a
second stop boundary. Closing the game window can interrupt capture before
finalization and is not the preferred stop control.

A scripted headless route can instead pass
`--input-script` with JSON events such as
`[{"at_micros":15000000,"key":"Enter","edge":"press"},{"at_micros":15055000,"key":"Enter","edge":"release"}]`.
The script uses physical key names from Orrery's current `Key` enum; the saved
recording contains the delivered X11 keycodes, keysyms and modifier masks. XRecord observes
server-delivered key edges on the private game window. Its availability on the
active Gamescope Xwayland display is required for this profile. The recorder
reports actual observation time separately from scripted dispatch time.

`recording/manifest.json` describes the fixture, provenance and coverage;
`recording/initial-saves/` preserves the prelaunch bytes;
`recording/events.jsonl` is an appendable, ordered timeline;
`recording/video/` holds continuous lossless FFV1 Matroska segments and a
per-frame timestamp index; and `recording/frames/` holds sparse lossless PNG
checkpoints with acquisition intervals. FFmpeg and FFprobe must be available.
Each window-size change starts a new segment. The capture reports timing gaps,
clock drift, and possible clipping around a size change. The corrected live
12-second smoke recorded 78 frames at 780×600 and 227 at 1024×768. Both files
are playable with zero-based media time, and the indexed raw capture timestamps
differ from media timestamps by at most 0.5 ms. The segments overlap by 159 ms
around the size change; the comparison suite retains both as evidence and uses
the newer full-resolution segment during the overlap. Game render/update
association remains unverified, so pixel differences are diagnostic.
A corrected 30-second manor route and its literal replay each captured 845
indexed frames and all eight scripted keyboard edges; both reached gameplay
in `manoir`. The session duration now ends before video encoder finalization,
so a slow encoder does not lengthen replay. The suite detected a 206 ms
separation between two actor samples with different normal/reflex selections
near level entry, and most video frames fell outside its fixed 15 ms pairing
tolerance because replay capture began about 89 ms earlier. These are reported
differences, not a claim of visual or completed-update parity. See the ignored
local evidence in `TestResults/hype-input-video-state-corrected-20260927/`.
When save bytes change, the recorder waits for matching observations at least
500 ms apart and stores a verified copy under `recording/save-snapshots/` with
its first-observed and snapshot times. This locates an observed stable change;
the engine's save-completion boundary is not yet instrumented.
`output-saves/` and `report.json` describe the finished run. A replay verifies
and stages the captured initial fixture into another fresh private copy. It
checks file bytes and modification times, hashes the source recording before and
after staging, and uses its own staged copy for launch. Captured fixture and
completed recording files are sealed read-only; run-specific save copies remain
writable. The manifest's wall-clock
start is sampled at the Wine launch boundary; a failed launch leaves an
incomplete recording without a launch event. A capture error after launch
marks the manifest `failed`, seals the readable event/frame prefix, and reports
the error; failed sessions are not replay inputs. Replay dispatches the recorded
physical edges at their literal elapsed-time targets,
records actual lateness and observed delivery, and captures its own video,
sparse checkpoints and state. Video and state samples are evidence, not replay targets. Capture
stamps before the first visible window, dropped frame intervals, and incomplete
endings remain explicit. Separate 2 GiB video and checkpoint budgets plus a
100,000-checkpoint budget bound one run; reaching any budget produces a
`storage_budget` ending.

The manifest hashes the Hype executable, its content tree outside the save
directory, and the Wine executable. Replay requires matching hashes. The saved
keyboard map is checked for each replayed key. The game runs from a fresh
copy-on-write prefix, and its final save tree is retained as output evidence
without changing the installed save root. The report confirms whether the
observed input edge sequence matches; it does not treat matching edges as proof
that a gameplay update consumed them.
The report separates `ending` (whether capture reached its duration) from
`delivery_validation` (whether every source X-server edge was observed in
order). A repeat-count difference fails strict validation and the CLI while
retaining the sealed capture; `nonrepeat_edge_sequence_equal` reports whether
the remaining physical presses and releases still align.

A 50-second isolated source run reached the Save UI, selected an occupied slot,
confirmed replacement, and accepted its name. Its changed `SaveGame0` files
were retained in a stable snapshot at 37.84 seconds. Literal-time replay from
the frozen fixture observed the same 41 keyboard edges but selected a different
title-menu row after Down and loaded `manoir` four seconds earlier; it did not
repeat the save. The comparison report preserves this discrepancy. Use the
`delivery.input_sequence_equal` field only for X-server edge delivery, not menu
or gameplay repeatability. The fixture and private installed saves stayed
unchanged in both runs.

The independent Godot consumer and frame comparator can inspect the same
recording. Build the Godot harness package explicitly when invoking its release
binary directly; the root workspace's default build does not include it.

```sh
cargo run --release --locked -p godot-harness -- \
  --recording TestResults/my-hype-recording/recording \
  --project godot --output TestResults/my-godot-replay
python3 tools/compare_recordings.py \
  TestResults/my-hype-recording/recording \
  TestResults/my-hype-replay/recording \
  TestResults/my-frame-comparison
```

The suite comparator streams every indexed video frame through fixed elapsed-time
buckets without shifting or resizing. It retains both raw segments at a resize
and selects the newer full-resolution frames in their overlap. Sparse PNG
checkpoints remain separate diagnostics. The report also covers observed
input-edge identity/timing (or explicitly dispatch-only evidence), missing frames,
timing uncertainty, dimensions, and sampled state/actor differences. State samples
pair by observed time, retaining every sample. Pixel metrics remain diagnostic
until render and gameplay update association is verified. The output also contains a standalone
`viewer.html` with a shared input/state/capture timeline, side-by-side frames,
first-divergence navigation and links to the diagnostic contact sheet and full
report. Its image paths refer to the source and candidate recording directories,
so retain those alongside the report.

The Godot harness hashes the authored and generated project content while
excluding the derived `.godot/` cache. It forces viewport draws for script-run
capture and records stalls and late inputs. Threaded manor resource loading kept
all six inputs within 11 ms of their recorded times in the 2026-09-27 run and
reached playable `manoir` within the 30-second recording. Godot reached gameplay
at 29.05 seconds versus 16.51 seconds in Hype; the input at 16 seconds arrived
while Godot was loading. Original save import and intro/menu behavior remain
unsupported, so this is not a full-session gameplay match.

State sampling currently covers phase/current/pending level and a typed main
actor sample during gameplay. The actor sample includes current normal/reflex
behavior selections, but cannot reveal transitions between samples. Actor
sampling briefly stops the process and reports that pause; it does not claim a
completed-update boundary. The frame sequence has no verified update/render
association. Full Orrery sessions record lossless game audio with a declared
launch-relative start; sample-to-engine-update timing remains unverified.
Event-level AI/API traces, mouse/gamepad input, focus changes, verified save-operation boundaries, and automatic first-divergence
comparison inside Orrery are currently absent and named in coverage. The separate
comparison tool reports first timed frame/phase/level and selected actor-sample
divergence. The existing 18-update precision tape remains a separate focused
probe.

## Compatibility matrix

| Old Hype MCP tool | Orrery implementation and preserved guard |
| --- | --- |
| `status` | Service plus owned-session status; does not launch the game |
| `start_game` | Idempotent 60–43,200 second session; verified fixture and executable; private Gamescope display |
| `stop_game` | Idempotent bounded cleanup of the owned process tree |
| `press_keys` | 1–256 validated names/chords, 20–10,000 ms hold, 0–2,000 ms gap, 25-second action cap, unconditional key release |
| `physical_key_sequence` | 1–64 typed `{key, hold_ms, gap_ms}` steps, 25-second aggregate cap; direct XTest edges with unconditional release and no game-state reads or process pauses |
| `enter_cheat` | 1–64 lowercase letters/digits, unique live binding, ordinary OS input |
| `run_sequence` | 1–64 actions, complete prevalidation, 120-second worst-case budget, four screenshots, 1 MiB read budget, no request interleaving, stop and release on failure |
| `get_state` | Live phase/current/pending level from the exact owned process |
| `get_mana` | Live gauge pointer/current/maximum/spell read |
| `screenshot` | Owned X11 window only; path plus MCP `image/png` content, constrained to the run tree and 8 MiB |
| `list_bindings` | Live action table, keyboard-layout inversion, action/code/address output |
| `inspect_main_actor` | Bounded selected action definitions, state selections and transform decoded through Astrolabe from one stopped-byte capture; optional unique action IDs (1–64), default `[47]`; optional `designer_values` (0–64 unique indices) adds normal/reflex behavior selections and typed current values; `include_camera` adds the active viewport pose/projection; not a completed-update observation |
| `inspect_combat` | Hype's selected action/designer state, active inventory, selected magic name/index/mana and matched inventory code, and laser target/visibility decoded from one stopped-byte capture; reports the selected Weapon ID and each active item's ID/quantity, with the same completed-update limit |
| `inspect_scene_tail` | Last 1–16 DynamicWorld children with checked links, kinds, Perso object types and transforms; reads each Perso's Brain model link when ready. Ordinals and `live:*` model locators are capture-local evidence, not imported asset identities |
| `inspect_scene_actor` | Resolve a rooted scene actor by level/path and checked object types. Optional `include_camera_records` permits an empty action list for camera actors with no family actions, follows the camera link, and captures four bounded source records plus the active viewport camera during the same process pause. Raw words and `live:*` locators are transient research evidence, not Rete values or completed-update publication. |
| `inspect_scene_camera` | Search live ActualWorld for one camera-bearing Perso with the requested type triple. Reject missing, ambiguous or malformed matches. Capture its Brain, linked records and active viewport in one pause. The returned live path is a capture-local search result, not a proven imported asset identity. |
| `read_memory` | Numeric or hex read; MCP hex limit 64 KiB; readable mappings and 32-bit address validation |
| `write_memory` | 4 KiB maximum, expected-old-value comparison while all threads are stopped, read-back before resume |
| `memory_maps` | `/proc` maps for the pinned owned identity only |
| `memory_probe` | Write, deliberate guard rejection, and restoration under one pause before resume |
| `navigate_level` | Installed-level check, phase-9 check, guarded transition writes, 30-second completion deadline |

`run_sequence` supports `keys`, `cheat`, `wait`, `wait_state`, `state`, `mana`,
`screenshot`, `bindings`, `inspect_main_actor`, `inspect_combat`, `inspect_scene_tail`, `inspect_scene_actor`, `inspect_scene_camera`, `read`, `write`, and `level`. Runtime failure records
the completed prefix, stops later steps, reports the skipped count, marks the MCP
result as an error, and releases injected keys. Completed effects are retained.
A client timeout does not cancel a controller operation; inspect `status` or
state before retrying a mutation.

## Verification

### Operation record

A dated exercise of these controller operations, including observations and cleanup evidence, is recorded in [notes](../../notes/orrery-live-session-2026-09-23.md).
