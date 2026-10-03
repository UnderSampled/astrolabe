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
