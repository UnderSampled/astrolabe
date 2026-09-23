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
```

Area runs include declared dependencies and mandatory safeguards. `--local` enables tests that need ignored local game data; without an area it selects the full existing converter inventory. `--release` selects the complete declared matrix. `--list` discovers the selected tests and reports `not_run`, never a pass. See `python3 tools/test.py --help` for fixture and budget options.

Each run owns a temporary workspace and tracks its process tree through cleanup. It does not install tools or proprietary game data. Independent jobs must not share one original-game installation. New reports do not overwrite older reports. The current selection, exclusions, tool versions, run times, and outcomes are captured in the generated JSON and supporting evidence directory.

Use `cargo fmt --all --check` and `cargo clippy --workspace --locked --all-targets -- -D warnings` when changing the workspace. Rust test coverage is collected separately with `tools/coverage.py`; synthetic and local-inclusive reports remain separate. Install the pinned optional tools with `python3 tools/install-test-tools.py nextest llvm-cov` when needed.

Tests are evidence for declared behavior. Converter tests should exercise useful values through OpenSpace import, Rete persistence, edits, reconstruction, and Godot output as relevant. Process-supervisor safeguards establish ownership and cleanup behavior. Neither structural conversion checks nor a selected original-game observation alone establish complete gameplay, cross-engine parity, or acceptance of rebuilt packages. See the [comparison evidence model](../orrery/comparison-model.md) and [historical checkpoint ledger](../../notes/testing-history.md).
