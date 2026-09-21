# Testing Astrolabe

## Focused suites and supervision

`tools/test.py` now uses a checked-in, exact test inventory and cargo-nextest
**0.9.145**. It retains the Python outer supervisor while the native runtime is
implemented. The manifest currently classifies all 106 Rust tests, their areas,
prerequisites, fixtures, release membership and individual hard deadlines. New,
removed, duplicate or newly ignored tests require a manifest update; discovery
mismatches fail before execution. The native original-game runtime, comparisons,
BDD scenarios and coverage collection remain in progress under the
[comparison plan](realtime-comparison-plan.md).

```sh
python3 tools/test.py --area runtime --offline
python3 tools/test.py --area movement --local --offline
python3 tools/test.py --area source --area rete --offline
python3 tools/test.py --local --offline
python3 tools/test.py --release --offline
python3 tools/test.py --area movement --local --offline --list
```

Area runs always include mandatory safeguards. Movement also selects AI and
animation dependencies and the actual-input native gameplay component safeguard.
`--local` enables ignored local tests within the selected areas; without an area,
it selects the complete existing converter inventory. `--release` selects every
declared test and cannot filter areas. The manifest is the current converter/test
foundation inventory, not evidence that planned live/parity scenarios exist.
`--list` builds and discovers tests but reports **not_run**, never passed.

Use `--game PATH` and `--godot EXECUTABLE` for explicit fixtures, or the existing
`ASTROLABE_GAME_DIR` and `ASTROLABE_GODOT` environment variables. Select the pinned
nextest executable with `--nextest`/`ASTROLABE_NEXTEST`; an executable on PATH or
`output/test-tools/nextest-0.9.145/cargo-nextest` is also detected. Tool versions
are checked. The runner does not install runtimes or proprietary data.

Native original-game checks use `--original-config PATH` or
`ASTROLABE_HYPE_CONFIG`; the existing sibling `hype-harness/local/config.json`
is detected when present. These ignored tests are selected by `--area runtime
--local` and the complete local/release inventory. They require the prepared
private Wine/Gamescope/KDE fixture and serialize with other engine tests. Missing
prerequisites are reported unavailable. Their retained JSON, raw snapshots and
screenshots live beneath the job's evidence directory.

The job budgets are explicit: development 300 seconds, routine 600, complete
local converter inventory 1,800, release inventory 2,400. `--timeout` can select a
finite job budget above the cleanup reserve. These are initial measured-budget
settings, not performance guarantees. There is no universal 570-second cap.
Per-test deadlines in `tools/test-manifest.json` are enforced by nextest, with no
retries. Engine tests serialize within one job. Two independent validation jobs
must not share an original-game installation.

Selection and exclusions are printed with reasons and retained in a JSON report.
Missing selected prerequisites are **unavailable** and fail the job; missing,
skipped or duplicate execution results cannot pass. Reports retain the source
revision, hash of tracked changes and all implementation files (including
untracked files), manifest hash, tool version, commands, individual results,
build/discovery time, execution and cleanup time, CPU time and peak child RSS.
Changing implementation files during validation invalidates the run. Nextest
JUnit output retains successful and failed test logs. New logs never overwrite
old evidence; `--log PATH` creates a sibling `.json` report and `.d` directory.
The optional native testkit is a companion workspace package; ordinary converter
builds still select the root package. Use `cargo fmt --all --check` and
`cargo clippy --workspace --locked --all-targets -- -D warnings` when changing it.

## Process ownership and mandatory controls

The Linux outer supervisor uses child subreaping and tracks descendant process
identities, including children that call `setsid` and leave the test process
group. Signals use pidfds after checking process start identity. An otherwise
successful test job with escaped workers fails and cleans them up. Existing
unrelated children are excluded. SIGINT/SIGTERM, failed setup and deadlines all
lead through bounded cleanup. Every run owns one temporary tree; only that tree
is removed. Source data, old generated assets and external game installations
are preserved.

The mandatory Python checks exercise real child processes, TERM resistance,
escaped sessions, parent exit, interruption, unrelated-process preservation,
and nextest's actual per-test timeout. A passing nextest job that leaks a detached
worker is explicitly a negative control. Selection tests reject hidden new or
missing cases and stale/skipped/duplicate results. Converter safeguards cover
source-free Rete export and deliberate edits, identity/sharing/schema checks,
native saved resources, production support, authored regeneration and diagnostics.
These tests complement architectural review; they do not establish full gameplay.
The five native process-owner safeguards additionally verify all-thread pauses,
pre-existing stop preservation, unwind restoration, immutable bounded snapshots,
mid-update rejection and stale processes. [Native runtime progress](native-runtime-progress.md)
records their scope and the remaining runtime acceptance gates.

## Checkpoint evidence

The initial focused implementation passed the movement selection: **28 Rust tests
in 210.554 seconds**, plus all mandatory supervisor/selection checks. Report:
`TestResults/focused-movement-foundation-20260920.log.json`. This includes the
actual compiled-input boundary proof, all four cadence/stall profiles, and both
deliberate late-input/no-op controls. After strengthening the mandatory contract
to export from a closed/reopened Rete store, the runtime selection passed **eight
Rust tests and 14 Python checks in 70.549 seconds**. Report:
`TestResults/runtime-foundation-20260920-final.log.json`. Both runs retained clean
process cleanup and unchanged implementation hashes. The earlier runtime attempt
failed its diagnostic-limit supervisor test despite passing all eight Rust tests;
its report remains at `TestResults/runtime-foundation-20260920.log.json`. The fix
bounds process-tree scan frequency independently of diagnostic throughput.

The 2026-09-20 paused gameplay checkpoint is **not fully validated**. The local
suite ended at the existing 570-second cap after **570.119 seconds**, exit 124,
with 84 logged passes and one explicit failure; remaining tests did not finish.
The new `source_hype_turn_and_walk_rules_drive_native_character` probe prints its
success marker but exceeds its own Godot 30-second deadline. Scene/direct/Rete/
editor checks passed before the suite cutoff. Cleanup completed with no leaked
or remaining processes. Report:
`TestResults/realtime-harness-checkpoint-20260920.log.json`.
Formatting, strict all-target Clippy, diff whitespace checks and the locked
offline release build pass. A mechanical Clippy correction in the new probe was
made after the suite compiled; no gameplay logic changed and the full suite was
not rerun. See [the checkpoint](hype-control-progress.md#paused-implementation-checkpoint--2026-09-20)
for the preserved work and limitations.

Earlier control-helper checkpoint: **94 tests passed in 451.614 seconds**
through `python3 tools/test.py --local --offline`, including ordered native
rule/reflex flow, repeated coroutine cancellation, Hype's complete idle rule 0,
the source action-array initializer, native input/action/contact checks, complete
corpus, Godot scene/editor checks and cleanup. There were no timeouts or leaked
children. Report: `TestResults/20260914T062627.071582Z-2088262.log.json`.
The locked offline release build passed in 37.62 seconds; formatting, strict
all-target Clippy and diff whitespace checks also passed.
See [Hype control progress](hype-control-progress.md) for original-game comparison
and the still-open playable-controller checkpoint.

The preceding checkpoint passed 91 tests in 469.441 seconds, before the native
rule/reflex and behavior-transition work:
`TestResults/20260914T055405.086137Z-1855899.log.json`.

Earlier checkpoint: native Godot migration, first AI-to-GDScript slice and
Scene dock visibility passed **86 tests in 497.572 seconds**, including 104
seconds of release compilation, all local corpus/Godot checks, actual sidebar
inspection and editor scrubbing, authored regeneration, support update/cache
rebuild, compiled AI behavior and cleanup. Report: `TestResults/20260913T185841.641467Z-25665.log.json`.
Child CPU time was 1,891.186 seconds, peak RSS 1,335,948 KiB, and no children
leaked. The scene route's three conversions and native/editor checks took
251.29 seconds. The sidebar exposes 2,632 rows in the preview and 2,631 in the
level, including all 150 source nodes, 108 persos and 899 parts.
See [migration evidence](native-godot-migration.md) and [AI evidence](ai-gdscript-slice.md)
for supported semantics, original-game comparisons and remaining limits.

The earlier native-only checkpoint passed 83 tests in 364.727 seconds, recorded
in `TestResults/20260913T081826.069430Z-4061873.log.json`.

The historical results above used the former one-invocation Cargo runner. Its
570-second cap and session-only cleanup have been replaced by the focused runner
described above; retain those logs as historical evidence, not current timings.

## Coverage and evidence

Install the optional, checksum-pinned Linux tools locally with
`python3 tools/install-test-tools.py nextest llvm-cov`. Existing tool directories
are preserved. `tools/test-tools.json` records the reviewed download URLs and
SHA-256 values; ordinary validation does not download tools.

```sh
python3 tools/coverage.py --area movement --local --offline
```

Coverage uses cargo-llvm-cov **0.9.1** and verifies the Rust/LLVM tool versions
match (baseline: Rust **1.98.1**, LLVM **22.1.8**). Each run builds in its own new
instrumented directory, exercises the same selected suites, and writes JSON,
LCOV and HTML plus provenance under `TestResults/coverage/`. Synthetic and
local-inclusive runs are labelled separately; profiles are never merged
implicitly. Ordinary test results remain separate. The reviewed initial
exclusion pattern removes `tests/`, `target/` and `reference/` trees. This raw
baseline **includes in-module test helpers under `src/`**; its totals must not be
presented as coverage of production implementation alone. Refining that scope
and changed-code targets remains part of the reporting work.

The first local-inclusive movement baseline passed in **334.071 seconds**,
including 329.658 seconds for build and selected tests. Evidence:
`TestResults/coverage/movement-baseline-20260920/summary.json` and
`html/html/index.html` beneath that directory. It measured **12,479/16,432 lines**,
**21,403/30,520 regions**, and **895/1,168 functions** for that selection and scope.
Per-domain counts are in the summary. These are Rust metrics, not branch,
GDScript, original-executable or level-data coverage. Level byte accounting,
changed-code reporting and broader live/scenario coverage remain unfinished.

The foundation tests exercise the same useful-content consumer against an in-memory collection and a reopened Rete store. They check shared versus equal-but-distinct assets, forward references, level A/B/A rebinding, cache invalidation, schema/kind/path/member validation, package roots and exact floating-point bits.

Compression tests use `tools/check_lzo.py`, a test-only independent `liblzo2` safe decoder with Python zlib Adler32. They verify newly generated streams/envelopes, including the empty-stream regression, and every populated envelope in the pinned Fix/astrolabe SNA and relocation fixtures. Original compressed-byte differences remain a separate fidelity result.

Archive allocation regressions use small explicit budgets with actual repeated CNT ranges and BNM PCM/APM descriptors; MPEG main/surround appends share the same budget and reject growth before reserve. Default CNT and BNM decoders cap cumulative pixel/palette or waveform allocations at 512 MiB per archive/bank, conservatively counting logical CNT aliases even though they share one image value; this is additional to per-member bounds. CNT preflights every GF header before image decoding. The supplied texture archives use 131.46 MiB and 26.02 MiB; all 199 supplied BNM banks fit the bank bound. MPEG counts actual decoded frames and retained vector capacity, including padding, rather than trusting declared duration.

Codec contract tests run the same typed consumer over GF image edits and Latin-1 sound-selector edits, and reject unsupported Unicode, mismatched image context, malformed labels, truncated headers and oversized declared lengths. Texture and audio tests check useful values and source reconstruction; local route tests exercise actual per-asset Rete persistence. OpenSpace export tests reject normalized texture-name and Fix/Level fragment collisions without replacing the first asset, and deduplicate repeated references in the report. Model tests use a rooted source model and deliberate edits. `tests/model_instances.rs` checks independently placed/lit instances referencing one geometry asset through Rete, controlled source fragments, and actual Godot resources. Godot contract tests validate actual generated resources and behavior with the selected engine. An unavailable original-engine harness is not a universal prerequisite; do not describe successful structural/import checks as complete gameplay or rendered parity.

The supervisor itself was checked with real short-lived process trees: a TERM-ignoring parent and separate-group worker timed out and were reaped in 2.48 seconds; an orphan worker after a successful parent was detected, terminated and marked failed; an ordinary exit code 7 was preserved. A full CLI trial with a controlled stand-in command confirmed argument/environment forwarding, timeout reporting and owned temporary cleanup in 0.28 seconds. These are supervisor checks, not claims that the complete Astrolabe suite or project milestones have passed. Current full-suite results live in `TestResults/` and the integration report.

The complete local-inclusive reviewed-refactor run on 2026-09-12 passed all **55 tests in 128.933 seconds**, including 50.36 seconds of release compilation and cleanup. It reported no leaked processes. Full report: `TestResults/20260912T162320.929180Z-235583.log.json`. Release build, strict Clippy, formatting, and diff whitespace checks passed. Godot checks ran outside the restricted sandbox to allow local sockets and editor settings.

The scene/sprite/animation/light checkpoint passed all **65 tests in 195.277 seconds**
on 2026-09-12, including release compilation and every local test. Report:
`TestResults/20260912T220040.714895Z-851647.log.json`. Cleanup completed without
leaked children; the single build/test deadline remained 570 seconds.

The sector checkpoint passed **67 tests in 225.55 seconds**, including release
compilation, all corpus/Godot checks, and cleanup. Report:
`TestResults/20260912T222959.258375Z-970703.log.json`. The earlier 226.958-second
run passed structural checks, but visual QA found that ImmediateMesh wireframes
did not persist. The accepted ArrayMesh implementation is checked after native
scene reload; direct/reopened-Rete routes also pass. The original-engine portal
comparison is separate manual MCP evidence in the scene/animation research note.

The shared animation-action checkpoint passed **68 tests in 225.69 seconds**,
including release compilation, every local corpus/Godot check and cleanup.
Report: `TestResults/20260912T225824.151502Z-1154481.log.json`. There were no
leaked children or timeouts. The native player checks cue ordering across state
entry, skipped frames and loops, pose visibility to listeners, and cancellation
of the old tick when a cue listener selects a new state. Source checks cover all
147 descriptors plus regenerated SND/RTS and edited cue/reference round trips.
Strict Clippy, formatting and diff whitespace checks passed. The earlier
233.113-second pass predates the callback/pose regression check.

The sound-program checkpoint passed **70 tests in 227.753 seconds**, including
release compilation, all local fixtures, both scene conversion routes, source
reconstruction and native Godot playback. Report:
`TestResults/20260912T234041.099463Z-1408481.log.json`. Child CPU time was
1,289.219 seconds, peak child RSS 1,323,036 KiB, and cleanup completed with no
leaked processes or timeout. The new audio model covers event-targeted stop,
quoted legacy source arguments, shared bank samples, Rete loop/volume edits,
material/default selection, no-repeat random choices, random pitch, stoppable
and nonabortable voices, and actual mixer output. The full scene route catches
localized CSB-owner/LCB-filename differences. New projects disable threaded
resource import after the separately reproduced Godot 4.7.2 WAV importer crash;
fixtures and coverage remain intact. Original walking events and SDA scalar
observations are separate MCP evidence in the scene/animation research note.

The configuration reconstruction checkpoint passed **71 tests in 224.780
seconds**, including 88 seconds of release compilation, all local/Godot
checks and cleanup. Report:
`TestResults/20260913T000049.705527Z-1554795.log.json`. Child CPU time was
1,223.686 seconds and peak child RSS 1,216,864 KiB; no timeout or leaked children,
and temporary cleanup completed. The source checks cover all 17 configurations
and 2,380 active/reset entries, reference targets, scales, unknown fields and RTB
receiving sites. Rete edit tests change a snapshot and table size. Native Godot
checks verify the displayed reset scale, frame/cue stability and snapshot
isolation. The full scene route exercises the CLI writer's nonoverlapping PO
fragment addresses and active/reset dependency traversal. Strict Clippy,
formatting and diff whitespace checks passed. Read-only original-engine MCP
observations independently confirm Hype's 56-byte family and 507-entry snapshot;
these are baselines rather than acceptance of generated game packages.


The family/state and mechanics checkpoint passed **73 tests in 259.117 seconds**,
including 120 seconds of release compilation, all local corpus/Godot tests and
cleanup. Report: `TestResults/20260913T003629.182307Z-1774655.log.json`.
Child CPU time was 1,539.672 seconds and peak child RSS 1,379,608 KiB; no timeout,
leaked children or cleanup failure occurred. The corpus verifies 21 family
records, 585 corrected 64-byte states, 19 transitions, 18 shared bounds and ten
mechanics cards. All ten cards reconstruct their original bytes; five are typed
and five explicitly retain unpromoted fields. Synthetic Rete edits exercise
family identity, shared configuration ownership, growing transition lists and
mechanics parameter changes. Native Godot checks verify state-selected mechanics,
bounds geometry/axis conversion and unchanged physics-body count. Both complete
scene routes and CLI family-group reconstruction pass. Formatting, strict Clippy
and diff whitespace checks pass. Original-engine MCP comparisons remain separate
from acceptance of rebuilt game packages.

The GPT Fix binding correction passed **74 tests in 259.578 seconds** in
`TestResults/20260913T010033.002086Z-1868962.log.json`, with complete cleanup and
no timeout or leaked children. It adds A→B→A active-level binding coverage and
checks all 108 corpus actor aliases. Strict Clippy passed. A fresh native Godot
probe also verified Hype's GPT placement and the existing MCP family/mechanics
expectations in 0.709 seconds; original read-only evidence is recorded in the
scene research document.

The final transparency checkpoint passed **76 tests in 238.725 seconds**,
including release compilation, all local/Godot tests and complete cleanup.
Report: `TestResults/20260913T012705.474229Z-2036182.log.json`. Child CPU was
1,427.693 seconds and peak child RSS 1,463,832 KiB, with no timeout or leaked
children. The material regression checks Rete key edits, opaque/keyed sharing,
duplicate black palette entries, source alpha-test/blend gates and derived PNG
filename collisions in both traversal orders. Formatting, strict Clippy and
diff whitespace checks passed. The existing 3,578 GF byte reconstruction checks
remain green. The separately measured original-engine screenshots and
ImageMagick comparison workflow are documented in
[material-transparency.md](material-transparency.md).

The scene hierarchy regression is documented in [scene-hierarchy.md](scene-hierarchy.md):
all 150 full SO records, source-order links, 145 picking-bound references and
embedded character matrices are checked under a derived layout. Synthetic and
native Godot cases cover parent edits, world placement and empty bounds.
The native material regression also checks source bilinear filtering; matched
original-engine pixel measurements are in [material-transparency.md](material-transparency.md#bilinear-filtering-follow-up).

The final scene-hierarchy/filtering checkpoint passed **81 tests in 216.322
seconds**, including release compilation, every local/Godot test and cleanup.
Report: `TestResults/scene-graph-full-retry-20260913.log.json`; no timeout, leaked
children or incomplete cleanup. Release build, formatting, strict Clippy and
diff whitespace checks passed. The preceding attempt received SIGTERM during
compilation after 116.842 seconds and is recorded as failed in
`TestResults/scene-graph-full-20260913.log.json`; it did not reach tests and left
no child processes. The successful retry used the same 570-second deadline.
