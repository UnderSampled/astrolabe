# Testing Astrolabe

The test supervisor in `tools/test.py` selects declared Rust tests, prepares fixtures, owns child processes, enforces bounded deadlines, and records a JSON report with cleanup and provenance. The checked-in `tools/test-manifest.json` declares test areas, prerequisites, fixture needs, release membership, and per-test deadlines. Discovery mismatches, missing selected prerequisites, skipped results, leaked child processes, and cleanup failures make a run fail.

## Run checks

```sh
python3 tools/test.py --offline
python3 tools/test.py --area movement --local --offline
python3 tools/test.py --area source --area rete --offline
python3 tools/test.py --local --offline
python3 tools/test.py --release --offline
python3 tools/test.py --area movement --local --offline --list
python3 tools/test.py --area movement --local --offline --no-headless
```

Area runs include declared dependencies and mandatory safeguards. `--local` enables tests that need ignored local game data; without an area it selects the full existing converter inventory. `--release` selects the complete declared matrix. `--list` discovers the selected tests and reports `not_run`, never a pass. See `python3 tools/test.py --help` for fixture and budget options.

Godot runs headless by default. `--no-headless` shows rendered gameplay launches
on the current desktop and records that choice in the suite report. Import,
editor validation, and short script checks stay headless because they do not
render the game. Use visible mode for interactive movement development; the
test supervisor still owns process deadlines and cleanup. `--headless` restores
the default explicitly.

The `movement` selection includes four Cucumber functional scenarios in
Orrery: turn and stop, walk and stop, run versus walk, and wall contact. The
synthetic backend exercises their shared steps and mismatch reporting offline.
With `--local` and the prepared original-game fixture, one ignored test runs
all four in one Hype process. A second ignored test uses one Godot process with
a normal `manoir` export, its saved Hype Brain, and a labeled English-save
state fixture. Each scenario has independent assertions and a
`behavior-*.json` result; the suite file links all four, records one launch,
and verifies cleanup. Walk may inherit the successful turn's compatible state;
run and wall restore the supplied save within the same process and establish
their start with physical input. Single-scenario execution remains available
through the same step runner. Backend reports are compared only after both
pass independently. See the [run comparison](../../notes/run-comparison-2026-09-24.md)
and [wall contact evidence](../../notes/wall-contact-2026-09-24.md) for bounds,
results, and remaining visual/timing limits.

The combat selection also includes an initial crossbow launch scenario. It
uses a declared quantity-three start, physical weapon selection, an observed
armed checkpoint, and a physical attack. Each backend must identify model 733
and show the same arrow at two positions. The paired result covers initial
launch only; subsequent contact behavior, trajectory and ammunition parity
remain open. See the [combat comparison note](../../notes/combat-comparison-2026-09-24.md#initial-crossbow-launch-bdd-2026-09-25).

Pair matching scenario reports with:

```sh
cargo run --locked --offline -p orrery -- compare-behavior --openspace SOURCE.json --godot GODOT.json --output COMPARISON.json
```

The command checks cleanup, individual passes, scenario and
recording identity, then writes named action, relative path, heading, and input
delivery checks. It returns failure if any comparison check fails. The JSON
keeps timing and rendered-image pairing limits visible.

Each run owns a temporary workspace and tracks its process tree through cleanup. It does not install tools or proprietary game data. Independent jobs must not share one original-game installation. New reports do not overwrite older reports. The current selection, exclusions, tool versions, run times, and outcomes are captured in the generated JSON and supporting evidence directory.

Use `cargo fmt --all --check` and `cargo clippy --workspace --locked --all-targets -- -D warnings` when changing the workspace. Rust test coverage is collected separately with `tools/coverage.py`; synthetic and local-inclusive reports remain separate. Install the pinned optional tools with `python3 tools/install-test-tools.py nextest llvm-cov` when needed.

Tests are evidence for declared behavior. Converter tests should exercise useful values through OpenSpace import, Rete persistence, edits, reconstruction, and Godot output as relevant. Process-supervisor safeguards establish ownership and cleanup behavior. Neither structural conversion checks nor a selected original-game observation alone establish complete gameplay, cross-engine parity, or acceptance of rebuilt packages. See the [comparison evidence model](../orrery/comparison-model.md) and [historical checkpoint ledger](../../notes/testing-history.md).
