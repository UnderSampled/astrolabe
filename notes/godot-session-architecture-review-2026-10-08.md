# Godot production session boundaries — review note

This is a source review, not a component refactor. The Load Game recovery fix is
being handled separately in the production game. No scene or script was renamed
for this note.

## Current ownership

| Production component | Current responsibility |
| --- | --- |
| `game/bootstrap.gd` and `bootstrap.tscn` | Main menu destinations, menu overlays, route button locking, and opening the New Game or Load Game session scene. |
| `game/demo.gd` and `demo.tscn` | Production session root despite the legacy `Demo` name. It starts initial loading, restores the imported/native state, installs and tears down levels, transfers supported actor/Brain state between levels, gates input/readiness, hosts pause/inventory UI, saves, and handles route failure. |
| `game/level_transition.gd` and the `LevelTransition` autoload | Threaded `PackedScene` loading, transition lifecycle signals, cancellation, named-entry requests, and the persistent loading/error presenter. It delegates session restoration and level installation back to the current session root. |
| `game/save_state.gd` | Native save envelope validation/capture/restore and compatibility-import reading. The owner/component contribution contract is still a separate architecture decision. |
| `addons/astrolabe/ui/loading/screen.gd` | Presents resource-loading progress and dismissible terminal errors; it does not own the current game scene or save restoration. |

The names `demo.gd`, `demo.tscn`, and the inherited `new_game.tscn` therefore
understate their production role. The responsibility overlap is more important
than the names: initial load is initiated by the session, asynchronous resource
loading is owned by the autoload, restoration returns to the session, and the
session currently schedules its own failed-route return to the bootstrap menu.

## Review boundary for a future split

Keep a small production `GameSession` facade as the scene root and move cohesive
responsibilities behind explicit child/component contracts in measured steps:

1. Keep route readiness, input locking, current-session teardown, and the active
   level reference at the session boundary. New Game and Load Game should enter
   the same session lifecycle with distinct restore policies.
2. Give a session-owned level controller the load/instantiate/restore/install
   transaction and named-entry placement. It should return a typed result to
   the session and keep the actor/Brain/physics ordering that makes restoration
   safe.
3. Keep `LevelTransition` limited to asynchronous resource loading, transition
   presentation, and terminal transition/cancel outcomes. It should not own
   save format decisions or duplicate level installation.
4. Keep `SaveState` responsible for envelope/version validation and restore
   ordering. Actor/component-owned values can be added only after the versioned
   owner contract is agreed; do not expose OpenSpace addresses or registration
   internals to Godot.
5. Leave pause, inventory, HUD, and authored menu screens as independently
   owned UI components that consume session availability/state through narrow
   signals or methods.

The first refactor should preserve the existing New Game, native-slot, original
import, cross-level entry, cancellation, and failure behavior. Each responsibility
move should have a focused Godot regression before the next move. This review does
not authorize a bulk rename or a production architecture migration.

## Recovery-specific observation

`Demo.fail()` schedules `_return_to_menu_after_initial_failure()` for an initial
route error. The first focused Godot 4.7.2 probe used the `SceneTree` harness's
default 64x64 root and sent the Dismiss center at `(32,70)`, outside its visible
height. The follow-up explicitly set the root Window and content scale to the
project's 1024x768 viewport. Both the legacy canvas point and the point from
`get_global_transform_with_canvas()` then hovered Dismiss at `(512,422)`. A
parsed mouse click hid the presenter and production recovery automatically
opened `bootstrap.tscn` with the route error status. This identifies the miss as
a test-harness viewport setup issue; no production presenter layout change is
needed. Enter mapped to `ui_accept` also remains a working recovery path.

The follow-up probe's attempt to report one CanvasLayer visibility diagnostic
called a method CanvasLayer does not provide, leaving that field empty. It did
not affect the captured root/viewport sizes, visible rect, hover, mouse click,
or recovery result. The later route acceptance used the configured viewport;
the diagnostic error remains limited to that earlier probe. The production
recovery coroutine captures the autoload-owned presenter before yielding, exits
if the session is canceled or no longer
current, and checks the scene-change result. The focused stage is
`output/frozen-manor-newgame-camera-20261008/failure-only-viewport-input-20261008/`;
the fixture hash remained unchanged. The later bounded route acceptance passed
New Game, original-import Load,
native schema-1 precedence and malformed-import dismissal/recovery with
`NEW_GAME_AND_LOAD_GAME_ROUTES_OK`. It used the frozen project under
`output/frozen-manor-newgame-camera-20261008/route-acceptance-clean-20261008/evidence/`,
which predates current exporter/support and optional Global DSG[147]. See the
[fork ledger](fork-first-integration-2026-10-08.md); this is not a fresh current
branch or complete intro acceptance.

## Safe naming cleanup after integration

`godot/project.godot` launches `game/bootstrap.tscn`. New Game inherits the
production `game/demo.tscn`; Load Game and restart also route to that scene.
`demo_camera.gd` is the authored production fallback/aim camera. These are not
optional demo-only dependencies. `godot/tests/demo_smoke.gd` is an observer of
that same production session, rather than another game implementation.

After title/pickup/armor/camera merges, rename filenames together:
`demo.gd` → `game_session.gd`, `demo.tscn` → `game_session.tscn`, and
`demo_camera.gd` → `session_camera.gd`. Move each `.gd.uid` unchanged. Keep the
existing scene/node names and observer fields during the first filename change
to avoid coupling it to an API migration. A later explicit node/field cleanup
may rename `ManorDemo`, `DemoCamera` and `demo_camera` with its own regression.

The filename change must update all of these current references:

- `godot/game/bootstrap.gd` Load Game and `godot/game/demo.gd` restart routes;
- `godot/game/new_game.tscn` inherited scene, and `demo.tscn` session/camera scripts;
- `src/godot/support.rs` starter paths and UID sidecar inventory;
- `crates/godot-harness/tests/new_game.rs` production session parser path;
- `godot/README.md`, game menu/inventory READMEs and the stable project guide.

If node/field names also change later, update `$DemoCamera` in the session,
`godot/tests/demo_smoke.gd`, and `godot/tests/level_transition_entry_smoke.gd`.
Search the final merged tree again before renaming, since workers may add actual
session references. Verify support initialization/update, scene parser checks,
New/Load/restart routes, failure/cancel recovery and named-entry camera handoff.
Regenerate local content only when the agreed aggregate validation is authorized.
No rename or component refactor is part of this reconciliation.

## Approved filename cleanup completed

The integration lead approved and the main executor completed the filename-only
change after the feature union. The actual routes, inherited New Game scene,
script preloads, starter/UID inventory and current documentation now reference
`game_session.gd`, `game_session.tscn` and `session_camera.gd`. The session UID
`uid://bljejs4l1sof2` and camera UID `uid://bel4kbbefd8qg` are unchanged. Existing
node names and the `demo_camera` observer property remain unchanged; there is no
new implementation or node/API migration. Older filenames in this dated review
describe the audited baseline. The rename is no longer a pending proposal.

Latest user direction prohibits tests, harnesses and gameplay acceptance. These
source references are statically reviewed; no parser/test/observer is run for the
rename. A production CLI build and source-content preparation are separately
authorized artifact creation, not game-behavior acceptance.
