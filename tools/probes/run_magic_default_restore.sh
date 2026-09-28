#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_dir"

probe=crates/behavior-suite/tests/magic_default_probe_temp.rs
if [[ -e "$probe" ]]; then
    echo "Temporary magic probe already exists: $probe" >&2
    exit 1
fi

cleanup() { rm -f -- "$probe"; }
trap cleanup EXIT
cp tools/probes/magic_default_restore.rs "$probe"

cargo build --locked --offline -p orrery --bin orrery
cargo test --locked --offline -p behavior-suite --test magic_default_probe_temp --no-run

if [[ "${1:-}" == "--compile-only" && "$#" -eq 1 ]]; then
    exit 0
fi
if [[ "$#" -ne 0 ]]; then
    echo "Usage: $0 [--compile-only]" >&2
    exit 2
fi

# Live execution needs approved access to the private Gamescope and audio
# sockets. The test creates isolated installation/save copies and closes them.
cargo test --locked --offline -p behavior-suite --test magic_default_probe_temp -- --ignored --nocapture
