# Testing Astrolabe

Latest checkpoint: the environment/fog slice passed **78 tests in 239.345 seconds**,
including compilation, all local corpus/Godot checks and cleanup. Report:
`TestResults/20260913T014933.418118Z-2136654.log.json`. Child CPU time was
1,387.26 seconds, peak RSS 1,139,680 KiB, and no children leaked. The focused
environment checks also passed in 48.40 seconds including compilation. See
[environment/fog evidence](environment-fog.md) for the original MCP readings,
matched-FoV pixel comparison, successful distance rendering probe and separately
failed desktop capture attempt.

Run the suite through one supervisor from any working directory:

```sh
python3 /home/stephen/Projects/astrolabe/tools/test.py --offline
python3 /home/stephen/Projects/astrolabe/tools/test.py --local --offline
```

The routine command runs all non-ignored Rust tests. `--local` runs those tests **and every ignored integration/corpus test**, including the supplied game data and actual Godot checks. Each command is one complete suite invocation; it gets one budget rather than a fresh budget per format or child process. New Cargo tests are discovered automatically.

The supervisor invokes `cargo test --release --locked -- --nocapture` once, adding `--include-ignored` for `--local`. `--offline` requires dependencies already in the Cargo cache. Compilation, Cargo lock waits, every test, Python/Godot workers, process cleanup and temporary-file cleanup all count toward the run's elapsed time. A cold release build counts too.

The build/test deadline is **570 seconds**. The runner reserves cleanup/reporting time and treats 590 seconds as its internal maximum total, leaving room before the user-required ten-minute limit. `--timeout SECONDS` can shorten the deadline but cannot exceed 570 seconds. A timeout is a failed run, including when some tests have already passed. Do not divide one required suite into several commands to evade the limit.

To choose fixtures or Godot explicitly:

```sh
python3 /home/stephen/Projects/astrolabe/tools/test.py --local --offline \
  --game /home/stephen/Projects/astrolabe/disc \
  --godot /usr/bin/godot
```

`--game` sets `ASTROLABE_GAME_DIR` to the game/disc root containing `Gamedata`; otherwise tests use an existing environment setting or the repository's ignored `disc/`. `--godot` sets `ASTROLABE_GODOT`; otherwise Godot tests use that environment setting or `godot` on `PATH`. A relative executable path is resolved from the caller's working directory. Supplying a fixture/runtime option does not implicitly enable ignored tests: use `--local`.

No runtimes are installed by this runner. The development environment requires Linux with `/proc`, Rust/Cargo, Python 3 and the system `liblzo2` runtime. Local Godot tests require Godot 4; source corpus tests additionally require the user-supplied Hype files. Verified runtime versions during implementation were Cargo 1.98.1, Python 3.14.7 and Godot `4.7.2.stable.arch_linux.ed1daf0bf`. Missing prerequisites fail visibly rather than silently reducing coverage.

## Deadline supervision and cleanup

Cargo starts in a new process session. The supervisor tracks every process group in that session, including integration workers that create a separate process group. It sends TERM, then KILL when necessary, and uses Linux child-subreaper support to reap orphaned workers. SIGINT/SIGTERM also trigger cleanup. A Cargo process that exits successfully while leaving a renderer/worker alive is reported as a failed run and its children are terminated.

Each invocation gets its own temporary directory through `TMPDIR`. Rust/Python test fixtures and Godot staging projects use that directory, allowing cleanup even if a killed test cannot run destructors. The supervisor removes only its own temporary tree with a bounded cleanup process. It leaves the Cargo build cache, source data, reference snapshots and unrelated outputs intact. Failure to complete cleanup is reported with remaining process IDs or the retained temporary path.

Every run streams Cargo output to the console and keeps a timestamped `.log` plus `.log.json` report in the ignored `TestResults/` directory. `--log /absolute/new-file.log` chooses a different new log; existing logs/reports are never overwritten. The report includes the command, total elapsed time, status, child CPU time, peak child RSS, timeout/leak information and cleanup status. Peak child RSS is the largest reported child-process resident set, not a sum of concurrent processes.

Exit codes preserve test failures. Code 124 identifies the shared deadline, 125 identifies supervision/cleanup failure or leaked children, and interrupted runs use the usual 128-plus-signal convention. A log containing passing individual tests is not a passing suite unless the final report says `passed`.

## Coverage and evidence

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
