# Manor pickup/barrel integration, 2026-10-08

This implementation follows the [preserved audit](manor-pickup-barrel-audit-2026-10-08.md).
The helpers are now registered in maintained support and called by compiled
actor AI. The authored `godot/game/inventory_host.gd` owns pickup feedback;
`godot/game/save_state.gd` owns the native inventory snapshot and each placed
barrel's consumed/removal state. Main and the reference repositories were not
edited. No original game was launched and no game-derived input is committed.

## Source trace and implementation

Static decompilation and the matching installed executable corroborate these
dependencies. The decompilation checkout was read only, without builds/tests.

- `actor_distance_queries.c`, `relative_geometry.c` and
  `relative_position_condition.c`: picking centers come from the bound collision
  sphere; a missing bound uses the actor origin. DistanceZ and XY distance use
  the receiver's local frame. Cond98 retains signed local Z and its asymmetric
  threshold rules. The native inverse transform is explicitly deterministic;
  the original scratch header flag is undefined, so numeric parity is not
  asserted.
- `collision/actor_zone_query.c` and `actor_zone_lookup.c`: Cond28 operand3 is
  the all-bits low-16 material predicate, independent of the one-actor result
  limit. Resolve the requested ZDE, target eligibility, active sector and native
  collision neighbors, and prioritize target ZDE over physical models. Native
  intersections cap at 4096. An absent contact material fails the predicate,
  including for a zero mask. Native hit ordering remains an adaptation.
- `ai/continuations.c`, callbacks `0054dd00`/`0054ddb0`: Meta40 captures the
  actor/field reference, full DWORD selector and expected signed value once.
  Its continuation rereads only the observed signed field. Selectors 0..5
  complete at `==`, `!=`, `<`, `>`, `<=`, `>=`; larger selectors complete
  immediately. The prerequisite implementation was corrected to this contract.
  Invalid native field access fails closed rather than emulating undefined
  source scratch values. Meta38 is a cancellable Boolean property wait.
- Procedures420/421: count writes are signed16 and do nothing when an actor
  has no stock. HUD capture `0041df20`, draw `0041e020` and item lookup
  `00418f60` retain the entire 62-item catalogue, including unselectable IDs15
  and99. The exporter produces native Texture resources; the game host creates
  a pausable Control with source640x480 center coordinates and alpha tween.
- Procedure46 reaches mechanics event9 through the event queue, retaining
  accumulated velocity and rotating configured VectorSpeed. It does not add
  the stored impulse. The dummy's unchanged script supplies armored damage10;
  its native armor facade now resolves the canonical catalogue.
- Function195 is compiled as a checked call to the authored session's
  `source_get_game_state`. Its production getter belongs to the title worker's
  `a93fa279cba48191fdb46ce345d7d05d7e4c8e8d`; this branch does not duplicate
  that session work. Invalid/missing bindings fail explicitly.

The reader resolves source picking bounds and actor eligibility into shared
model fields. Export emits native named PickingCenter, ZDE/physical model
shapes, sector/neighbor paths and Brain paths. Original addresses and relocation
tables remain reader concerns. Generated actors find their containing level
under an authored session, retain its MainActor, bind external Brains before
starting, and duplicate actor stock. Source activation gates rendering,
processing and contact admission. No AI/opcode or sampled-animation interpreter
was introduced.

Native schema-1 slots gain optional canonical inventory snapshots and world
entries for the four placed model488 barrels (world/0/71..74). Their saved field8
is owned per actor, with a tombstone for a removed placed actor. Transient drop
field0 and dummy queue fields are not saved. Old slots retain the legacy
inventory path. Global[220..228] are source-confirmed Boolean first-pickup flags
with player-save mask0x200, retained as optional fields;
transient HUD bookkeeping19..23 is omitted. The lead must reconcile this shared
save file with the save worker's atomic Boolean preflight and life/armor work.

Rete schema is56 and support interface73 on this branch. The lead independently
advanced main to those same versions for inventory/loading work; version and
shared inventory changes need reconciliation rather than a duplicate increment.

## Changed files

- Shared semantics: `src/model/ai.rs`, `src/model/ai/walk.rs`,
  `src/model/character.rs`, `src/model/inventory.rs`.
- Reader/codecs: `src/openspace/ai.rs`, `src/openspace/level/character.rs`,
  `src/openspace/level/inventory.rs`.
- Export/persistence: `src/godot/ai.rs`, `src/godot/character.rs`,
  `src/godot/collision.rs`, `src/godot/inventory.rs`, `src/godot/scene.rs`,
  `src/godot/support.rs`, `src/rete/mod.rs`.
- Maintained runtime: `godot/addons/astrolabe/behavior/behavior.gd`,
  `character/character.gd`, `character/explosive_barrel.gd`,
  `scene/sector_boundary.gd`, new `ui/pickup_artwork.gd` and
  `ui/pickup_feedback.gd`, their UID sidecars, the two character-helper UID
  sidecars, support `README.md` and `version.json`.
- Actual game: `godot/game/inventory_host.gd`, `godot/game/save_state.gd`.
- Checks: `tests/manor_pickup_barrel.rs`, new `tests/check_manor_dispatch.gd`,
  `tests/ai.rs`, `tests/ai_wait_integer.rs`, `godot/tests/ai_wait_integer.gd`,
  fixture literals in `tests/animation.rs`, `tests/cameras.rs`,
  `tests/locomotion.rs`, `tests/routes.rs`, and `tools/test-manifest.json`.
- Records: this note and `notes/README.md`.

## Exact Git sequence

1. Started clean at `ed961c7`, retaining `1aa775e67368980153c11fc502d36ee3c64dc219`.
2. Fetched local origin main and merged the requested `edb7704` with
   `git merge --no-edit edb7704`. Merge commit:
   `8420af6573603bad6913c58e443e7bd82e2e00c8`. No conflicts.
3. Stashed in-progress integration, fetched and cherry-picked prerequisite
   `e79369362bc98ca6f2c2e991e6ac44efdce56e9b`. Resolved the sole cherry-pick
   conflict in `src/model/ai/walk.rs` by retaining both sound and wait traversal.
   Result: `08b14751ab56fec8404d7dd1c804b1ad6e82e61b`.
4. `git stash pop` conflicted in `src/openspace/ai.rs`; retained both Boolean38
   and Integer40 decoding, then corrected Integer40's semantics using the trace
   above. The task stash remains recoverable; it must not be reapplied.
5. The subsequent feature commit contains the integration and corrected wait
   contract. No later main commits were merged; sequencing/acceptance belongs
   to Kip. No remote push was performed.

## Bounded evidence and limits

The source test imports Manor in memory without a full level export. All eleven
barrel/dummy/drop models have no unresolved script fragments and pass native
Godot parsing after family-aware compilation. Tutorial model535 Normal6 is
checked for unresolved fragments. Typed stocks and actor ownership survive
Rete. A separate local Fix test retains native HUD artwork links for IDs12/15/99
through Rete, without copying textures into the repository.

The maintained-package fixture runs the unchanged ordinary pickup model1080's
settled normal/reflex path and dummy model671's damage path. It verifies actual
native Cond28 dispatch, exact all-bits material filtering, ZDE slot selection,
eligibility/sector rejection, session/level external bindings, independent
spawned stocks, the authored inventory host and pickup feedback, one-time grant,
damage10, queue advancement and the actual game save capture/apply methods.
Media/animation rig and sound cue resources in this bounded fixture are
synthetic. It is not an end-to-end visual playthrough or proof of every template's
animation release, tutorial dialogue completion or original trajectory parity.

Focused evidence lives only in ignored `output/evidence/`: `manor-final.log`,
`final/actual-dispatch.log`, `ai-final-isolated.log`, `first-pickup-flags.log`,
`codec-final.log`, `release-build-final.log` and `clippy-final.log`. The affected
AI/wait selection passed29 tests (14.43s +0.81s); Manor passed2 tests (11.24s,
plus4.74s build). AI source codecs passed49 tests; local artwork validation
passed1 test (1.41s). Release build, formatting,
all-target Clippy and whitespace checks are separate gates. Full exports, the
release matrix, original-game runs and long gameplay runs were excluded by the
assignment; production getter/save integration and acceptance remain with the
lead.

## Main reconciliation

Kip approved this exact branch tip after armor/camera, Brain-slot and title
integration. Main preserves the existing inventory animation lifecycle and
loading service, merges the authored InventoryHost feedback API, and keeps the
title's single typed Func195 model/import/write/compiler path and actual session
getter. Automatic merging placed duplicate GameState branches without textual
conflicts; those are explicitly removed. Sound and both wait traversals remain.

The previously integrated armor facade guard (`selected_definition` plus a
valid catalogue) is retained. Its existing damage/capacity methods consume that
same level-owned catalogue, so the branch's alternative catalog guard/helper
does not become a second production path. The newly mapped barrel fields use
main's live Bool preflight before mutation, alongside the existing dummy and
Global checks. The connected native dispatch regression additionally rejects an
integer-typed placed barrel Brain atomically and checks pausable timed feedback.

Combined contracts advance to support76/Rete58, including all title/pickup
registrations and the current schema assertions. The Orrery synthetic Perso
fixture explicitly defaults the new picking bounds/eligibility fields, preserving
whole-workspace compilation. `cargo check --workspace --all-targets --locked
--offline`, formatting and whitespace pass before the merge milestone. The
affected native/source combined selection is a separate subsequent gate; no
full level export or save follow-on capability is claimed by this milestone.

The local fetch imported this approved commit graph but its final check reported
an unrelated malformed Codex checkpoint ref in main. The requested commit and
common base are readable; that app-owned ref is left unchanged. No worker file,
recovery stash, original installation or remote was modified.
