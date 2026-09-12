# Testing Astrolabe

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
