# Testing Astrolabe

The test supervisor in `tools/test.py` selects declared Rust tests, prepares fixtures, owns child processes, enforces bounded deadlines, and records a JSON report with cleanup and provenance. The checked-in `tools/test-manifest.json` declares test areas, prerequisites, fixture needs, release membership, and per-test deadlines. Discovery mismatches, missing selected prerequisites, skipped results, leaked child processes, and cleanup failures make a run fail.

## The merge gate

`python3 tools/gate.py` runs at merge points, not continuously. It takes about
two minutes and asks one question: is this still a game?

| Check | What it shows |
| --- | --- |
| Controls probe | Load Game, then turn, walk, run, jump and sword through physical keys, with a median frame time floor on real hardware |
| Route | Load the save, walk to the level exit and load Brigand through the real game |
| Original comparison | Live original data against the parser, on a disposable copy of the prefix |

The game checks start by loading a save, which is far faster than replaying the
opening. The intro is an important behavior test of the full route instead:
`python3 tools/route_check.py --full HANDOFF_DIR` plays it before the routes. Stage
scenarios in `tools/behaviors/manoir.json` compare both engines from fast starts;
they are short, on demand and outside the gate.
A scenario's `place` starts both engines at a source position. In the original,
`behavior_compare.py` scans the heap for the position's 26 copies and rewrites
them inside one pause with Orrery's `write_memory_batch`. A far target's floor is
not active yet, so Hype falls, the game's own recovery (behavior 16) returns him
nearby, and a second placement holds. Placement moves position only: heading and
story flags stay as the save left them, so set them before trusting a stage.

The gate is capped at three checks and ten minutes. Adding a check means removing
one. It prints every stand-in reached. Godot renders on the discrete GPU through
a headless Gamescope, so no window opens. It doesn't replace `tools/test.py`,
which is run when it answers a question.

## Run checks

```sh
python3 tools/test.py --offline
python3 tools/test.py --area movement --local --offline
python3 tools/test.py --area inventory --area loading --local --offline
python3 tools/test.py --area source --area rete --offline
python3 tools/test.py --local --offline
python3 tools/test.py --release --offline
python3 tools/test.py --area movement --local --offline --list
python3 tools/test.py --area movement --local --offline --no-headless
```

Area runs include declared dependencies and mandatory safeguards. `--local` enables tests that need ignored local game data; without an area it selects the full existing converter inventory. `--release` selects the complete declared matrix. `--list` discovers the selected tests and reports `not_run`, never a pass. See `python3 tools/test.py --help` for fixture and budget options.

The focused `inventory` area imports only the inventory closure, reloads its
Rete assets, exports native animations and exercises the maintained UI against
admitted catalogue state. `loading` uses the production `LevelTransition` with
synthetic scenes to check cap restoration, lower limits, installation failure,
cancellation while the loader drains, and service exit. Both selections retain
mandatory safeguards; neither exports or accepts the complete game.

The manor camera checks distinguish unchanged camera-rule aiming from the
original World three-shot excerpt. The latter preserves source targets,
waypoints, mode requests and 3600/4500-ms waits, while excluding unsupported
earlier initialization/guards in its explicit temporary AI fixture. Its harness
supplies viewport activation and native clock stepping, not camera placement.
Neither component test establishes the complete opening or its final handoff.

For temporary native harnesses, follow [the probe workflow](./probe-workflow.md): syntax before conversion, explicit owned Godot logs, and Orrery's existing isolated-session and display-bridge APIs. It also records the contact shape-index lookup and the boundaries between compile checks and behavior evidence.

Godot runs headless by default. `--no-headless` runs rendered gameplay inside a
disposable authenticated Xvfb display; it requires `xvfb-run` and fails instead
of falling back to the desktop display. The suite owns the Xvfb and test process
lifetimes. Import, editor validation, and short script checks stay headless
because they do not render the game. The direct session replay binary also
requires an explicit `DISPLAY` and existing `XAUTHORITY`, so launch it under
`xvfb-run` when capturing rendered output.

The `movement` selection includes four Cucumber functional scenarios in the
separate `behavior-suite`: turn and stop, walk and stop, run versus walk, and
wall contact. `behavior-contract` owns their engine-neutral input, evidence and
assertions; Orrery owns the OpenSpace backend, and `godot-harness` owns the
Godot backend. The synthetic backend exercises their shared steps and mismatch
reporting offline. The original and Godot test binaries select a backend for
the same feature files and input schedules; they are not separate case sets.
With `--local` and the prepared original-game fixture, one ignored test runs
all four in one Hype process. A second ignored test uses one Godot process with
a normal `manoir` export, its saved Hype Brain, and a labeled English-save
state fixture. The suite evaluates the same scenario assertions separately for
each backend run and writes a `behavior-*.json` result; the suite file links all
four, records one launch, and verifies cleanup. Walk may inherit the successful
turn's compatible state;
run and wall restore the supplied save within the same process and establish
their start with physical input. Single-scenario execution remains available
through the same step runner. Backend reports are compared only after both
pass independently. See the [run comparison](../../notes/run-comparison-2026-09-24.md)
and [wall contact evidence](../../notes/wall-contact-2026-09-24.md) for bounds,
results, and remaining visual/timing limits.

The combat selection includes a Crossbow camera-mode shot from a declared
quantity-three start: physical Z selection, keypad-0/Left aiming and Space
firing. Both backends must identify model 733, observe movement, consume one
bolt and contact the saved checkpoint's reaction box. Source contact identity
uses object types and the checked result record; DynamicWorld sibling ordinals
can change during gameplay. Godot must release the arrow from its animation
channel, move along its released sighting, exit through compiled normal-1
cleanup and clear the firing guard. Its trace observes each native physics
tick, retaining wall-clock timestamps separately from physics lifetime.
The original backend separately checks that a later full-scene search finds
the arrow absent; it does not assert the original deletion caller.

`original_crossbow_quantity_survives_fresh_process` independently checks the
supplied save's eight bolts, the guarded quantity-three setup, one physical
shot leaving two, a real lectern save and a fresh process restoring two. It
does not edit save bytes. See the [Crossbow investigation and evidence](../../notes/crossbow-fixes-2026-10-02.md)
for fixture recovery, engine changes and remaining comparison limits.

`exported_crossbow_free_flight_expires_and_releases_guards` is a separate local
component regression. It freshly exports manor and releases the actual arrow
through the maintained animation channel in declared clear space. Straight
and yaw-0.2 target cases check sustained native flight, counter-151 expiry,
target-directed sighting and all owner guards. It holds owner AI and does not
establish original missed-shot expiry or enemy damage. See the
[bounded followup](../../notes/crossbow-free-flight-2026-10-02.md) for the fixture
and source comparison gaps.

Pair matching scenario reports with:

```sh
cargo run --locked --offline -p behavior-suite -- --openspace SOURCE.json --godot GODOT.json --output COMPARISON.json
```

The command checks cleanup, individual passes, scenario and
recording identity, then writes named action, relative path, heading, and input
delivery checks. It returns failure if any comparison check fails. The JSON
keeps timing and rendered-image pairing limits visible.

Each run owns a temporary workspace and tracks its process tree through cleanup. It does not install tools or proprietary game data. Independent jobs must not share one original-game installation. New reports do not overwrite older reports. The current selection, exclusions, tool versions, run times, and outcomes are captured in the generated JSON and supporting evidence directory.

Original-game comparisons run Gamescope with its explicit headless backend,
without inheriting the desktop's `DISPLAY` or `WAYLAND_DISPLAY`. Gamescope owns
the nested Xwayland display used by Wine, screenshots, and XTest input; active
game status reports this backend and its private nested display. Orrery also
routes game audio through its own temporary PipeWire/Pulse server and null
sink, then closes that route with the game. The save persistence test runs
rendered Godot stages in an authenticated private Xvfb display; it never opens
a game window on the desktop. Run the focused save selection with
`python3 tools/test.py --area save --local --offline`.

Use `cargo fmt --all --check` and `cargo clippy --workspace --locked --all-targets -- -D warnings` when changing the workspace. Rust test coverage is collected separately with `tools/coverage.py`; synthetic and local-inclusive reports remain separate. Install the pinned optional tools with `python3 tools/install-test-tools.py nextest llvm-cov` when needed.

Tests are evidence for declared behavior. Converter tests should exercise useful values through OpenSpace import, Rete persistence, edits, reconstruction, and Godot output as relevant. Process-supervisor safeguards establish ownership and cleanup behavior. Neither structural conversion checks nor a selected original-game observation alone establish complete gameplay, cross-engine parity, or acceptance of rebuilt packages. See the [comparison evidence model](../orrery/comparison-model.md) and [historical checkpoint ledger](../../notes/testing-history.md).

## Pose-paired stills

`tools/still_pairs.py capture` reads Gogoud's action and animation frame through
Orrery's `find_scene_actor` (`current_frame`, Perso3dData+0x78 low half) around each
original screenshot and saves `pairs.json`. The port captures at the same action and
frame with `ASTROLABE_INTRO_PAIRS=pairs.json` in `new_game_intro_acceptance.gd`;
`compare` prints fitted scale/shift, RMSE, colour ratio and sharpness.

## Route checks

`tools/route_check.py HANDOFF_DIR [route.json ...]` runs the saved routes in
`tools/routes/` through the real game with `godot/tests/route_bot.gd` (steps go,
plan, teleport, face, key, wait, shot, until_level, until_normal, until_item, watch, census, spell,
zones). It starts each run from a copy of an F5 handoff save directory, retries
a route whose `required` step fails, and reports any step marked FAILED or any
script error. A route takes up to a few minutes, so this is on demand. Routes cover
training, the key, the first gate and the exit; `stage*.json` start mid-game with
bot steps, and `save_exit.json` is the gate's route.

## Side-by-side videos

`tools/video/` makes an original-versus-port video from a take and a route-bot
run over it. All three steps cut by landmarks: frame or time, the first hard
cut or black run after or before another landmark, or a take's key press or
epoch (`videolib.py` lists them; `compose.py --timeline FILE START SECONDS`
prints the luma timeline to check them).

1. `original_span.py TAKE OUT.mkv --start LM --end LM` cuts the take's own FFV1
   capture by frame index (via `frames.jsonl`) to constant 30 fps FFV1. `--check`
   prints the capture's frame-interval statistics.
2. `port_capture.py ROUTE.json OUT_DIR` runs `route_bot.gd` under Movie Maker
   (`--write-movie`, fixed 30 fps, PNG and WAV) in a headless Gamescope, then
   converts the result to `port.mkv` (FFV1 and FLAC). The bot's clock is wall time,
   so the bot runs from a copy whose `Time.get_ticks_msec()` reads the fixed-step
   clock (`--clock wall` turns this off). `--gentle` runs at low priority on four
   cores, at no more than real time, for use beside another run.
3. `compose.py LEFT RIGHT OUT.mkv` stacks them with labels, starting both sides
   at their start landmarks and holding the shorter side's last frame. It keeps
   the right side's audio (`--loudnorm` brings it to -16 LUFS) and encodes
   hevc_nvenc main10 at CQ 20 (or `--encoder x265`).

Manoir, from the intro to the fade-out before Brigand, sync on the intro's
first frame after each side's loading screen, since the port's development
loading screen is longer and looks different:

    T=TestResults/stephen-linear-brigand-20261102 O=TestResults/side-by-side-manoir-$(date +%Y%m%d)
    tools/video/original_span.py $T $O/original.mkv --start 'scene-after:scene-after:key:65293@20' --end 'black-before:epoch:brigand:5'
    tools/take_route.py $T $O/route.json --until 330
    tools/video/port_capture.py $O/route.json $O/port
    tools/video/compose.py $O/original.mkv $O/port/port.mkv $O/manoir.mkv --right-start scene-after:start --right-end 'black-before:t:LOAD' --loudnorm

Here `LOAD` is a time just after the port's fade-out to Brigand; check it with
`--timeline`. Encoded against a lossless 4:4:4 reference, a 27 s test gave
luma PSNR 54.7 dB with NVENC (3.3 Mb/s) and 52.7 dB with libx265 slow CRF 18.
Under Movie Maker, game code that reads `Time.get_ticks_msec()` (the enemy
meter timeout, vignette holds) still runs on wall time.

## Byte coverage of the original files

A signal of which source bytes the parser understands, never a gate and never
used to rebuild anything:

```sh
ASTROLABE_BYTE_COVERAGE=output/byte-coverage \
  target/release/astrolabe import-openspace disc /tmp/rete-manoir --level manoir
```

It takes about the time of the import (7 s for Manoir, 10 s for Brigand) and writes
`output/byte-coverage/<level>.txt` and `.json`: for each Fix and level SNA block, the
share of bytes interpreted, read but kept opaque, and never touched, with the largest
untouched gaps and the parsing sites read just before and after each. Every read goes
through `SourceSession::bytes` and its helpers, which carry `#[track_caller]`, so a
range is tagged with the parser line that read it. A read counts as uninterpreted when
that line keeps bytes opaque (it mentions unknown, preserved, opaque or raw). Sites that
slice a whole block directly are listed per block rather than attributed per byte, so
those blocks report some read bytes as untouched. Without the variable the parser
records nothing.

## Exported-build smoke check

`tools/export_smoke.py [release|debug]` exports the Linux preset with the standard
4.7.2 templates and runs the built game with `ASTROLABE_SMOKE=new`: the bootstrap
starts a New Game and `game/smoke_check.gd` exits 0 once the Manoir is playable
(1 after a minute). Exported templates ignore `--script`, so route-bot checks
cannot drive an exported game; a game driven that way just sits at the main menu.
