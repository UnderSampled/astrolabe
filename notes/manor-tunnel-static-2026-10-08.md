# Manor tunnel transition: static implementation, 2026-10-08

This work starts at `d493ef2` in the isolated branch
`codex/manor-tunnel-static-20261008`. It connects the existing imported tunnel
AI to the maintained `LevelTransition` service. It is source inspection and
static implementation only. No original game, Godot, replay harness, tests,
content export or `hype-decomp` build ran. Kip owns integration and acceptance.

## Actual source trigger

Manor's `iDummy_Load_Manoir_MAD` is `level:/ai/models/334`, attached to
`level:/persos/world/0/34`. The source contains two normal schedules:

1. Normal 0 checks local source-XY distance to MainActor `< 6.0` and requires
   World (`level:/persos/world/0/3`, model 154) to be in Normal 0. It writes
   Hype DSG[56]=1 and DSG[51]=0, runs `Action_Camera_GoToWP` with StdCamer and
   waypoint `level:/waypoints/17-1/55`, then changes the camera to state 3. It
   writes Hype DSG[62]=0, DSG[123]=3, DSG[157][0..2]=waypoints 56..58, and
   DSG[166][0..2]=0. It requests Hype's model-28 Normal 50 and changes itself
   to Normal 1. The World condition is part of this trigger, not a new area or
   teleport condition added by the port.
2. Normal 1 checks MainActor distance to waypoint 58 `< 5.0`. In order it
   fades 0→255 over two seconds, kills the entrance grille
   `level:/persos/world/0/159` (`i_Grille_Entree_Manoir1`), fades all sound to
   zero over 2000 ms, waits 2000 ms, kills music, fades sound to 127 over 1 ms,
   and requests `brigand` with entries `i_brigand_dummy_gogood_hype` and
   `i_brigand_dummy_gogood_camera`. This branch contains no `SetGameState`.

Read-only data evidence is model 334 in the schema-48 snapshot under
`output/worktrees/manor-courtyard-collision-20261008/output/courtyard-trace-20261008/source-rete/`.
Its unsupported fragments retain the camera and waypoint operands; they are
evidence of the script, not current-schema conversion acceptance. The existing
schema-42 transition snapshot was a second source-data reference. The existing
Brigand scene under
`output/level-transition-fixes-validation-20261008-final/two-map/` maps the two
entry names to `world/0/38/I_Brigand_Dummy_Gogood_Hype` and
`world/0/54/I_Brigand_Dummy_Gogood_Camera`, respectively. None of these local
assets was changed or committed.

## Engine and reconstructed-C evidence

Raymap's Hype catalogue identifies meta-action 49 as `Action_Camera_GoToWP`.
The original Ghidra callback `0054f2d0` captures camera slot 0 and waypoint
slot 1 through the generic `PW` capture path. It computes the waypoint's
current world location and reads the camera's current translation through
`00527dd0`. `00527d60` uses inclusive squared-distance arrival against the
squared radius. At arrival it returns 4; otherwise it supplies waypoint minus
camera translation through `00440fe0` and returns 1. The reconstructed C in
`hype-decomp/src/ai/waypoint_meta_parameter_capture.c`,
`src/camera/camera_vector_helpers.c`, `camera_motion_calculation.c`, and
`camera_fields.c` corroborates capture, geometry and the displacement consumer.
The native adaptation uses the existing `SourceCamera` displacement API and
ordinary generated coroutine cancellation. It adds no camera tween or source
scheduler. Negative radius is squared by the callback; the current useful
Waypoint validator still admits only nonnegative source geometry.

The PE's compressed table for `0053fc30` maps raw procedure 226 to dispatch
index 7 (`0053fda3`), not its default index 8. That branch calls `00437940`,
cleans the caller's pending action and cancels the caller only when the killed
Perso is itself. Ghidra `00437940`/`00437860` retain placed non-pool Perso
storage while removing its active/visual membership and sound. The port now
stops the selected native Brain and sound emitter and disables the retained
actor. A generated self-owned drop retains its existing node-removal API.
Pool reuse and source saved-reset fields are not established by this adaptation.

Proc5 strips one trailing `~` only when its signed string length is positive.
Proc13 evaluates all three operands and strips one trailing `~` from each;
unlike Proc5 it has no source empty-string guard. Both reset the listener to
MainActor, set the source media-selection flag, and call their map helper.
`00405a50` and the PE-only `00405c80` compare the map case-insensitively before
publishing new map/entry names or entering loading. Same-map requests therefore
continue the AI without changing placement. Native loading of an exported scene
replaces the original drive/media branch; the port does not emulate it.

### Proc13 operand correction for the source translator

The output pointer for the **first** evaluation is computed at `0053ff47`
before four callee-saved register pushes. Against the later stable stack pointer,
the evaluated values occupy first=`+0x20`, second=`+0x10`, third=`+0x30`.
At `00540028..00540030`, while the listener argument remains on the stack,
the loads `+0x34`, `+0x14`, `+0x24` recover third, second and first. The
subsequent pushes are third, second, first, so the cdecl helper receives
`00405c80(first, second, third)`. Its map argument is the first evaluated
operand. The first suffix scan likewise starts with first (`+0x2c` while
three evaluator arguments remain), followed by second and third.

The Task-18 handoff's `(second, first, third)` mapping confuses those two local
slots. This work does not change its isolated `death_map_procedures.c`; its
translator should correct the Proc13 local-slot mapping. Manor's actual literal
order and the connected existing map API both agree with first=map,
second=Hype marker, third=camera marker. Proc12/14 should be reviewed using the
same stable-stack accounting when changing local names.

`00405c80` publishes its second/third arguments through `00405210`.
`hype-decomp/src/gameplay/level_initialization.c` and Ghidra `00402910` show
those names resolved against the loaded World, followed by copying marker
matrices to MainActor and the first registered camera actor before actor startup.
`004425e0` is a matrix copy. The existing useful camera registration exports
the primary viewport as `SourceCamera.initially_current`; the port supports an
unambiguous primary camera and reports a missing/ambiguous named camera binding.
Multiple registered source viewports need their own future mapping.

The source-profile executable SHA-256 is
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`;
the installed-profile executable is
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
Static byte comparison found matching inspected ranges for the death/map
dispatchers, camera callback, arrival helper, displacement setter and positioned
map helper. This scoped agreement is not whole-executable or runtime equivalence.
Addresses in this note are reader/research evidence; native callers use named
assets, nodes and component properties.

## Connected changes and ownership

- Model/OpenSpace/Rete: `CameraGoToWaypoint` retains both expressions, traverses
  their dependencies and round-trip encoding, and uses Rete schema 59.
- Godot compiler/support: lower the camera meta-action through the existing
  displacement component; write MainActor/external actor designer-array
  elements instead of silently reading them; compile KillPerso against semantic
  native actor bindings; reset the listener before map requests. Source targets
  that remain unpromoted fail explicitly. Support interface is 78.
- `LevelTransition`: remove one literal suffix, preserve case-insensitive
  same-map behavior and free rejected fresh instances. Existing loading-cap,
  progress, cancellation/draining and terminal restoration functions are retained.
- Session: initialize both named native entry poses before child `_ready`,
  carry each marker's imported physical-sector binding when available, and use
  the registered source camera. Hold all destination Brains until placement and
  state are ready; restore the previous viewport camera on failure. The existing
  two physics-readiness frames remain. Outgoing actors are freed only at commit.
- State: inventory and armor use their component-owned complete snapshots and
  explicit catalogue clones for preflight; life uses its native snapshot API.
  Magick remains scalar component state. Fresh destination inventory/magick
  owners are resolved on the level before `_enter_tree` actor binding. Existing
  model-matched MainActor/Fix Brain primitive transfer excludes live actor and
  level-waypoint references, which are bound by the new level instance.

No save-envelope schema or general world persistence is added here. This is
in-session transfer through the existing transition framework. The production
schema-1 save path remains limited to its documented fields and Manor route;
saving/loading arbitrary destination levels remains a separate save-owner task.
Shared configuration permutations and arbitrary actor continuation are not added
to this bounded transfer.

## Static verification and integration boundary

`cargo check --lib --locked --offline -j 2` passed with a private worktree target
directory; the final incremental check reported 3.32 seconds. `cargo fmt --all
--check` and `git diff --check` passed. The existing
compiler fixture was aligned with the actual Normal-1 branch and its semantic
grille target; it was not run. No Godot parse/import, source reimport, Rete
round-trip or runtime acceptance is claimed.

Integrators must reimport source data with schema 59, regenerate both Manor and
Brigand in an isolated output and update support interface 78 before acceptance.
Runtime work still needs the actual guarded tunnel approach, camera arrival,
Hype's Normal-50 route/contact, grille and audio fade order, destination poses and
sector/contact, shared state, cancellation/failure recovery and loading-cap
restoration. Source camera conditional gaps elsewhere remain independent blockers.

`godot/game/demo.gd` is the baseline shared-session path. Kip's executor is
renaming it to `game_session.gd` (and its camera to `session_camera.gd`) while
preserving UIDs; apply these session hunks to the renamed file and current camera
property. Do not restore the old filenames. Concurrent AI/support changes may
overlap `src/model/ai.rs`, the AST walker, reader/compiler and schema/interface
constants; reconcile the aggregate versions once during integration.
