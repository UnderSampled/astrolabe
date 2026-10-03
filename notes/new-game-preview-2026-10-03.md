# Current-state New Game menu preview, 2026-10-03

Stephen requested a way to see the current buggy opening without waiting for
complete cinematic/UI fidelity. The existing menu now adds **New Game**, entering
`game/new_game.tscn`, an inherited ordinary `game/demo.tscn` session. Load Game
keeps its default focus, cached resources, measured save fixture and supported
native slot restoration. A bad slot blocks Load Game while New Game remains
available. Both entries are disabled without generated manor content.

The New Game branch uses the complete normal generated manor scene, original
initial actor state and full compiled source AI. No test excerpt, World trigger,
camera pose edit, Brain hold or gate lift advances the opening. Save restoration
is skipped and preview saving is disabled. R restarts New Game using fresh nodes
from the retained resource; Esc returns to the menu. Original-game saves and
assets are not touched.

Source initialization is incomplete: World's reflex contains unbound input
expressions, Hype initialization reaches the particle-generator gate, and
later Gogoud dialogue/presentation also needs support. The existing authored
follow camera keeps the raw starting scene visible. The session activates the
generated camera only if its own AI is running and it has selected a target.
This viewport fallback supplies no camera state to the source API. The HUD labels
the opening incomplete, counts stopped actor behaviors and leaves exact compiled
AI errors in the console. This is a usable preview, not intro or visual parity.
No fade, letterbox, dialogue or other UI-owned renderer is implemented here.

The prepared declared check is
`godot-harness::new_game/menu_new_game_preserves_slot_and_restarts_original_opening`.
It uses a private XDG data/config/cache root and a test-owned unsupported-version
slot. The real menu launches New Game, checks raw starting pose and unlifted gate,
all Brains remaining automatic, disabled saving, F5 slot preservation, restart
with fresh actor nodes, retained content cache and Escape-to-menu preservation.
It records actual stopped behaviors and a rendered PNG. Only explicit unsupported
compiled-AI diagnostics are permitted for this rough preview; parser, renderer,
other engine errors and nonzero process exit remain failures. Owned syntax checks
precede expensive generation. The native case has a 180-second outer deadline;
its two session waits each have a 70-second bound.

Existing menu/player smoke and save-persistence harnesses now find named Load Game
and New Game buttons. Their original gameplay/restoration assertions are retained.
The separate original fourth-cut integration prepared in `0381048` is deferred
for this request. Native execution and screenshots are pending the fixed Luna
worker; Sol has run no Godot or original-game session for this preparation.

The source-only preflight passed in 94.280 seconds: formatting, strict all-target
Clippy for Astrolabe and Godot-harness, locked offline release build, the support
ownership safeguard and all six Python selection checks (including authored-game
and harness freeze coverage). Source stayed unchanged and all supervised stages
cleaned up without timeout or remaining children. Commands, timings and logs are
in `TestResults/new-game-preview-source-preflight-sol-20261003.json` and its
five companion logs. The broader workspace Clippy command separately fails on
the existing `too_many_arguments` lint at `crates/orrery/src/orrery.rs:722`;
that unchanged, unrelated API was not modified for the menu request.

The fixed Luna assignment is one invocation of
`python3 TestResults/run-new-game-preview-v1-sol-20261003.py`. Its 600-second
budget includes five early owned syntax checks, normal local manor regeneration,
then 23 Rust cases (all 21 mandatory safeguards, New Game preview and existing
native save/Load Game persistence) plus all four Python modules. The inner
selection has 420 seconds, with individual 180/100-second menu/save deadlines.
All other cases remain explicitly excluded; no fourth-cut or original-game
launch is assigned. Runtime results and rendered preview acceptance remain pending.

To preview after local generation/import: open `godot/project.godot`, run F5 and
select **New Game**. Load Game remains first/default; keyboard selection is Down
then Enter. Launch and exact current failures will be recorded after Luna's smoke.

## Why the local preview did not show Gogoud walking

Luna's v1 initialization and all five Godot parser checks passed. Regeneration
then stopped at the normal ownership guard after 100.865 seconds because the
local generated `manoir/level.tscn` hash differs from its receipt. Total setup
was 102.616 seconds, source unchanged and cleanup complete. No native New Game,
save/restart/menu or Load Game behavior case ran. The earlier prepared launch
instructions therefore remain unverified. Evidence:
`TestResults/new-game-preview-v1-sol-20261003-result.md` and
`new-game-preview-setup-v1-sol-20261003/result.json`.

The local generated receipt still declares support interface 39, while the source
exporter/support is 44. Its unchanged, receipt-matching Gogoud/model 210 GDScript
still emits a failure for procedure 220 (`ReInitWay(0)`) at node 12 in the original
first-visit normal 13 branch. It stops before selecting action 0/changing to
normal 0. The subsequent `GoOverWay(6)` at normal 0 node 0 is also still unsupported
in that old generated program, and its actor has no exported route bindings.
Current source compilation already lowers both operations to the maintained
native route API. The accepted original Gogoud component reaches actions
[0,26,24,23] with contact; that is supporting evidence, not a pass for this
full-scene preview. The earliest identified local walk blocker is stale generated
AI/assets, rather than a new missing movement implementation or later dialogue.

A read-only receipt audit found 395 changed generated files: one scene, 32 mesh
resources and 362 import sidecars (2,971,368 bytes), all in the sole manor selection.
The receipt alone does not reconstruct the intent of their edits. No authored
game/support file is part of that changed set. Exact hashes and paths are in
`TestResults/new-game-gogoud-stale-assets-static-sol-20261003.json`. Stephen said
he has no edits to keep and authorized replacement with a reversible backup.
All 395 changed files and the original receipt are copied and hash-verified under
`output/archived-godot-manor-edits-20261003-pre-gogoud/`. A complete generated-tree
snapshot also preserves 5,959 files/399,601,931 bytes, including unowned import/UID
sidecars, with a verified inventory. The live original output remains in place
until the owned regeneration stage; original game assets and saves are untouched.

The next Luna workflow moves that fully backed-up old generated tree into the
archive immediately before fresh normal generation. It preserves the ownership
guard and never changes its hashes to excuse an edit. On a generation failure,
the owned workflow archives any partial new output and restores the old tree.
Successful regeneration leaves the complete previous output recoverable.

The normal New Game smoke is strengthened to require the current support receipt
and actual compiled reset/route calls, observe Gogoud's full original route/action
sequence without holding any Brain, and assert sustained displacement,
source-associated native contact and the original observed end pose. It preserves
start/end rendered PNGs and source Brain failures. Native execution is still
pending; no initialization, World reflex, camera or dialogue gate is skipped.

The v2 source-only preflight passed in 0.662 seconds: `cargo fmt --all --check`,
`cargo clippy --locked --offline -p astrolabe -p godot-harness --all-targets --
-D warnings`, and `python3 -B tools/test_selection.py` (six selection checks).
The frozen source stayed unchanged and supervised cleanup completed without
timeouts or remaining children. Evidence:
`TestResults/new-game-gogoud-source-preflight-v2-sol-20261003.json` and its logs.

The v2 fixed-Luna assignment was one invocation of
`python3 TestResults/run-new-game-preview-v2-sol-20261003.py`, with a 600-second
total budget. It performs early syntax checks, verifies the full reversible
backup, archives the old generated tree, regenerates the manor, and runs the
same bounded 23 Rust cases and four Python modules. The walk observation ends
at original action 23, before the following 1,200-ms delay and dialogue call.
It does not depend on the UI worktree or omit any initial source behavior.

V2 stopped at the early smoke-script parser check after 1.028 seconds. Session
and menu parsed, but `new_game_smoke.gd:136` inferred a comparison against the
actor's dynamically resolved `current_state` without a declared Boolean type.
The local observer now declares `changed: bool`; other new inferred locals use
typed native methods/properties or a declared Dictionary return. This is a test
syntax fix, with no change to source AI or gameplay. No backup move, regeneration
or native behavior case ran; source stayed unchanged during the failed run and
cleanup completed without remaining children. Evidence:
`TestResults/new-game-preview-setup-v2-sol-20261003/result.json` and its owned
`parser-preview-smoke.engine.log`.

The corrected fixed-Luna assignment is
`python3 TestResults/run-new-game-preview-v3-sol-20261003.py`, using fresh evidence
paths and the same recovery, selection and budgets. Native parser and normal
opening acceptance remain pending. V1/v2 evidence is retained unchanged.

The v3 source-only preflight passed in 2.401 seconds using the same formatting,
scoped strict Clippy and six selection checks. Source stayed unchanged and all
stages cleaned up without timeout or remaining children. Evidence:
`TestResults/new-game-gogoud-source-preflight-v3-sol-20261003.json`. This does not
replace the pending native Godot parser check.
