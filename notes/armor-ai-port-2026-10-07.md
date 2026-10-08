# Armor catalog and AI port checkpoint, 2026-10-07

This isolated feature port adds a Fix-owned armor catalog model and importer,
Rete schema 43 persistence, native Godot catalog/definition/piece resources,
per-level mutable catalog copies, and typed AI operations for selection,
resistance, repair, armor queries, and armored damage. It also adds a shared
iterative AI AST walk used by dependency/fragment collection and structural
validation, plus source functions for Square, AbsoluteValue,
IsValidObject, and DistanceToPerso.

## Source basis and boundaries

The source layout and operation addresses are summarized in the stable
[armor reference](../docs/openspace/armor.md). The importer follows Fix.gpt
through Fix relocation data and validates MMG allocation boundaries. It does
not infer ownership from the selected level. The reconstructed-data test reads
the supplied English manor's Fix files and confirmed three definitions with
piece counts `[7, 5, 5]`; original game data was only read.

The existing shared level armor selector remains the gameplay entry point.
Field.Position, input remapping, saved-game conversion, and existing scene
selection state were retained. HUD presentation is still a separate worker's
scope. Armor-to-OpenSpace reconstruction, Function225's callback, procedure
432, and exact RNG sequence matching are not included.

The branch coordinates one schema increment (Rete 42 to 43) and one support
interface increment (54 to 55). Support 55 includes the armor Resources and
the hit-point API needed by Function224.

## Validation

- `cargo check --locked --offline`, `cargo fmt --all --check`, and
  `cargo clippy --locked --offline --all-targets -- -D warnings` passed.
- The narrow actual-data importer passed using
  `ASTROLABE_GAME_DIR=/home/stephen/Projects/astrolabe/disc`.
- The source AI node-decode/round-trip/compiler test and existing shared armor
  selection test passed, including Godot 4.7.2 `--check-only` parsing of
  generated GDScript. Synthetic Rete save/reload and resource/PNG writer tests
  also passed.
- The synthetic native armor resource graph and PNG writer have a focused
  Rust unit test. The full Godot resource import/export run could not be
  completed in this executor: Godot's first import attempts to open a local
  editor socket, which the sandbox denies. No resource-load or runtime armor
  behavior claim is based on that blocked run.
- Running the entire `--test ai` binary produced 13 passes and six Godot
  environment failures. Those failures reported a denied editor socket or
  denied user config/log writes; they do not pass as gameplay evidence.

The code is a local, unpushed feature-port commit for main integration; it has
not been cherry-picked into the project main worktree.
