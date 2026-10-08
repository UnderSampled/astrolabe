# Manor pickup/barrel audit, 2026-10-08

This optional slice is based on main `bac48e1` (support interface 72). It owns
only the two new character helpers, `tests/manor_pickup_barrel.rs`, its Godot
harness, and this note. It uses a source-only clone with independent Git/build/
evidence directories. Intro integration, shared code, support registration,
schema/version changes, generated content and main merges remain with the lead.

The delivered code implements spatial primitives and the actual-contact material
predicate. **The unchanged Manor pickup/barrel interaction is not yet runnable.**
The finite source map below identifies the remaining shared work before export
and gameplay acceptance. No original executable was launched, no original C was
edited, and no game assets were copied or committed.

## Exact actors and stock joins

Manor's placed barrels are `level:/persos/world/0/71` through `/74`, respectively
`iBaril2_Manoir_Mad`, `iBaril1_Manoir_Mad`, `iBaril3_Manoir_Mad`, and
`iBaril8_Manoir_Mad`. They share `level:/ai/models/488` and family 31. Their
imported current selector field 4 is zero; field 8 is false. The related dummy
is world actor 76, `iDummy_PourBarils`, using `fix:/ai/models/671`.

Historical model `Level/17-1/612` belongs to Astrolabe's barrel actors. Its frame
and template clues are not evidence of Manor's initial configuration. Even
within Manor, sharing model 488 does not make every barrel's mutable state or
configured drop identical. Initial selector zero takes the no-template path
unless gameplay changes it. Other actors' writes to that configuration have not
been exhaustively audited here.

The actual model-488 template list has these nine ordered entries. Catalogue
joins use the existing typed actor-owned `inventory_stock`, not new item tables.

| Selector | Fix actor suffix | AI model | Catalogue meaning | Stock count |
| --- | --- | --- | --- | --- |
| 1 | `5-0-27239` | 1080 | RestoreHP item 12 | 1 |
| 2 | `5-0-27177` | 1051 | RestoreHP item 13 | 1 |
| 3 | `5-0-27254` | 1109 | RestoreHPMP item 14 | 1 |
| 4 | `5-0-27000` | 993 | RestoreMP item 10 | 1 |
| 5 | `5-0-27162` | 1022 | RestoreMP item 11 | 1 |
| 6 | `5-0-26985` | 964 | Weapon/ammunition item 80 | 10 |
| 7 | `5-0-26867` | 935 | Weapon/ammunition item 81 | 5 |
| 8 | `5-0-27315` | 1167 | Special gold catalogue member (ID 0) | 50 initially |
| 9 | `5-0-27330` | 1196 | No stock; direct life/magic effect | — |

Gold's native AddObject return ID is **-1**, distinct from catalogue ID 0.
Gold normal 0 uses Proc420 to replace its stock count with Global field 74.
The barrel writes that field as `10 * Func195GameState` before object generation.
Thus the initial stock count 50 does not establish the effective spawned amount.

## Complete dropped-item path

All nine source models use the following finite interaction:

1. Spawn/release uses `ExecuteActionAndGenerateObject(template, action9, zero
   vector)`. Native generated-object release cues and animations already exist;
   this slice does not replace them with a timer or a runtime asset loader.
2. Normal 0 sets local field 0 to 1, sets upward impulse 10, jumps absolutely and
   changes to normal 1. Gold performs the count override first.
3. In normal 1, a positive field 0 plus contact doubles that field, sets upward
   impulse to `10 / field0`, and jumps. Above 7 it changes to action 0 and sets
   field 0 to zero. The schedule waits 25,000 ms, sets field 0 to 255 and enters
   normal 2. Normal 2 subtracts 14 each schedule tick and applies transparency;
   below zero it kills itself unless MainActor is in normal behavior 33.
4. Reflex 0 admits contact when `(MainActor56 == 0 OR MainActor28 == 0)` AND
   target-local absolute Z distance to MainActor `< 2.25` AND target-local XY
   distance `< 1.0` AND NOT `IsAbovePerso(MainActor.position, 0.5)` on this drop.
   This is not a generic body-touch event. Then Meta38 waits for Global19 == 0.
5. Seven ordinary stocks set MainActor97 to AddObject(this actor) and require
   their exact expected item ID before succeeding. Gold instead requires
   `Func210(-1) > Func198()` before AddObject and does not check an ordinary item
   return ID. The no-stock template succeeds unless both life and magic are
   already full; it adds integer `0.2 * maximum` life and magic directly.
6. Success sets opacity zero and transparent display, invokes Proc421 HUD
   feedback, advances Global20/21/22/23 bookkeeping, sets Global19 to one and
   plays MainActor bank-0 sound member 64 (`playmo_b`). The life/magic variant
   also updates life/magic HUD and player time/life fields, with the source armor
   HUD condition. Per-item first-pickup flags (gold 228, life/magic 227) can wait
   on the world-27 tutorial actor, set its tutorial item (gold 99, life/magic 15)
   and change its normal behavior to 6. KillSelf follows that tutorial path.

Rejected grants must not disappear. The suspend/grant/kill ordering matters:
visual disappearance precedes final actor removal, while an outstanding source
wait must not repeat the grant. The native check exercises that coroutine
ownership with a synthetic two-frame wait, not Manor's unresolved Meta38 or its
whole tutorial chain.

## Complete barrel/dummy path

Normal 0 deactivates the object if saved field 8 is true and configured field 4
is below 99, restores opacity 255, selects action 7, initializes grounding
tracking and enters normal 1/reflex 1. Object activation is a separate source
bit; current `source_set_object_active_state` does not itself hide or disable AI.
Grounded/airborne transitions track height and settled frames. A delayed armed
branch waits at least 1,000 ms; normal 4 alternates actions 7/1 with 500-ms waits.

Reflex 1 checks Cond28(this, 1, **2**) into field 8. The break/queue trigger is
player field 104 in {2,6,10} with the world-27 distance bound, OR field 8, OR
Cond28(this, 1, **4096**). It selects the closest barrel in dummy fields 3/4,
waits one frame and queues normal 2/reflex 0 with a ticket. Nearby dummy-radius
contact either queues a barrel with field 2 set, or selects action 6, records
time and applies the source angular impulse before returning to normal 1.

Normal 2 begins with Meta40 waiting for dummy field 5 to match the barrel's
ticket. Coordination temporarily locks player fields 0/17 and saves/restores
field 104. It chooses action 3 for airborne/field-0 barrels, otherwise action 2
or 0 according to field 1. It sets field 8, binds dummy field 7 to this barrel,
starts dummy normal 1, derives Global74 and copies configured selector 4 to 5.
Selectors >=100 use the source capacity/life/GameState/random fallback; the
probability tree remains in compiled AI. Selectors 1..9 generate the corresponding
template with action 9; other selectors clear field 8 and execute action 9
without a template.

The normal-2 rule's `CurrentAnimationFrame > 5` condition also requires dummy9
zero, action 0/2/3 and the final ticket. It releases/resets the coordination
queue. It is not a proved fuse or object-release time. After action completion,
an airborne barrel kills itself; a grounded one selects action 4/5, waits
2,000 ms, enters normal 3 and fades by 3 per tick from 255 before KillSelf.

Dummy normal 1 waits two frames, sets chain radius to 3 and takes the chosen
barrel's position. If MainActor is within distance 3 and player field 32 is zero,
it sets player field 119 to 10, computes the source facing flag and evaluates
**ApplyArmoredDamage(10)**. Remaining life below one selects player normal 24,
otherwise normal 15. This is explicit source damage evidence, not an inference
from explosion visuals. It then waits one frame, releases its player lock,
clears chain radius, increments the queue ticket and enters idle normal 2.

## Original spatial/contact proof and native contract

The original-C coordinator and independent workers supplied instruction proofs
under reservation `f504eab`. The subsequent six-body collision checkpoint
`b87f480e091492f6f740a32468cbea795cbf7f84` is complete in the independent
`hype-sol-core` clone, with contracts in
`research/worker-deltas/sol-core-priority5-notes.md` and the maintained
`collision_zone_condition.c`, `actor_zone_query.c`, and `actor_zone_lookup.c`
sources. It covers 2,144 profile-matched original code bytes. These are static
proofs, not engine runs or native equivalence evidence.

* `DistanceZToPerso` / `DistanceXYToPerso` (`00537a20` / `00537aa0`) evaluate the
  target with the original caller, retain the selected Ultra receiver, then
  use `0052ea00` with axis masks 4/3. Receiver-minus-target translation is
  transformed by the **target** inverse ISO basis. Source Z maps to Godot Y;
  source XY maps to Godot XZ. `contact_pickup.gd` implements native inverse-basis
  distances. The nonpositive-result negation is retained. Late instruction proof
  shows `POS_fn_vInvertIsoMatrix` (`004b9e30`) reads its destination header bit
  0x100 before writing output (`004b9e45/4e`, `004b9f0e`). Both distance helpers
  supply an uninitialized 88-byte stack local. Thus the original scale path
  depends on a residual-stack flag: call structure proves target-local projection,
  but not portable deterministic numeric equivalence. This candidate uses
  Godot's deterministic inverse as an explicit native adaptation for lead review;
  it must not be registered as fully faithful solely on these synthetic passes.
  Exact x87 rounding, NaN payload/sign, residual-stack scaling and degenerate/
  sheared ISO transforms are not accepted by these finite rigid-transform checks.
* `IsAbovePerso` (Cond98 in `00534fc0`) uses mask 0x84 through `0052ead0`:
  point-minus-receiver picking center, transformed by receiver inverse basis.
  `0052e7c0` resolves first indexed sphere element 0/sphere 0/point in its
  bounding geometry; only absent bounds use actor translation. Positive local
  Z is required. Nonnegative thresholds use >=; negative thresholds use strict
  `< -threshold`. The helper requires the resolved world picking center as a
  value. It does not parse retained JSON or guess the center from visible meshes.
* Cond28 (`005338a0`) forwards a full third operand via `00524420`, which seeds
  its independent output limit to **one**. `0042f210` narrows `(operand2 - 1)`
  through low8 for its zone index. `0042f2a0` requires source current sector
  activation bit 2; own-sector candidates exclude self and require StandardGame
  byte 1b == 0 and byte 1c == 1. Neighbor collision sectors require bit 2 and
  those activation bytes but do not have the same explicit self exclusion.
* `0042f4e0` tries candidate active category-2/ZDE geometry, then selected
  physical-object module kinds 0b/14. Both routes require the material prefilter
  `0042d8b0`, actual intersection, and the returned **target** game material
  test `0049e0b0`. Its game-material pointer is not null-guarded before the
  getter reads its +0c member. The obtained **collide-material address** rejects
  zero/ffffffff; this is not a safe missing-game-material fallback.
  `0049e580` requires
  `(identifier & uint16(mask)) == uint16(mask)`. Material prefilter supports
  triangles, points, spheres, boxes and cones. The external intersection body
  `00555cb0` is not closed; do not invent searching another shape/material after
  its selected contact fails.

`explosive_barrel.gd` implements that all-bit low16 predicate and resolves the
actual native contact shape's existing `source_collision_identifier`. It rejects
missing materials and invalid indices, including mask zero. It does not implement
Cond28 traversal/activation or assert that Godot selects the same intersection
pair as the original. Neither new helper autonomously grants items or explodes
barrels; compiled AI owns those decisions and the native support owns effects.
The helper's rejection of absent converted material metadata is its explicit
native contract; it does not reproduce an original null-pointer dereference.

## State ownership and shared patch proposal

Barrel model field 8 carries save flags **0x0a00**. Each barrel's Brain owns its
value; the test edits/reloads one Perso while leaving the imported actor unchanged.
All nine drop-model field-0 save flags are zero, as are dummy fields. Do not add a
global consumed-template flag or save the transient bounce/fade value by guess.
Rete round trips retain stocks, complete Persos, models and save metadata. Native
inventory snapshots reload quantities into independent Resources. This is not
acceptance of the game-slot persistence of saved barrel field 8 or spawned actors.

The lead can integrate these minimal source-backed changes after intro review:

1. Add typed DistanceZ and receiver-aware IsAbove expressions at the OpenSpace
   codec/shared-model/compiler boundary. Preserve original-caller operands and
   Ultra receiver. Export the actor's named picking-center reference/value from
   its SuperObject bounds, retaining an explicit absent-bounds origin fallback.
2. Rename Cond28's `maximum` field to a material/type mask throughout the typed
   contract. Replace Character's hard-coded identifier bit 1/counting/ZDM-body
   query with native active-ZDE/physical-object contact facilities. Preserve
   actor/sector eligibility using native scene/component state, not reconstructed
   OpenSpace lists. Apply the new predicate to the actual selected target shape.
   Any geometry-selection difference requires separate source/native validation.
3. Complete remaining source fragments: multiply-assign (operator 7), Proc420
   stock count setter, Proc421 HUD, Meta38 WaitBoolean, Meta40 WaitInteger,
   Func195 GameState, Func198/210 gold/capacity, Ultra receiver variable access,
   actor-qualified current-life query, and angular impulse Func23/24/Proc46.
   Existing typed inventory, armor damage, native animation and KillSelf are
   reused. Native coroutine waits should bind ordinary Brain fields, not emulate
   the original interpreter/scheduler. Add actor/component save ownership for
   field 8 only after the original save meaning is connected.
4. Register the two maintained helper files (and UIDs if required), version the
   support contract and add the dedicated tests to the supervisor inventory.
   These shared files were deliberately not edited in this branch.

The imported unsupported fragments are explicit and currently stop whole-source
execution. They must not be bypassed or treated as harmless no-ops to claim the
pickup/barrel backlog complete.

## Validation and limits

Focused release test command, with source data only for the ignored audit:

```sh
ASTROLABE_GAME_DIR=/home/stephen/Projects/astrolabe/disc CARGO_TARGET_DIR=output/target \
  timeout 70 cargo test --release --locked --offline --test manor_pickup_barrel \
  -- --include-ignored --nocapture
```

The Rust wrapper stages maintained support and these two candidate helpers into
an owned temporary project. It runs a 15-second Godot syntax preflight before
native execution (also bounded to 15 seconds), with explicit owned logs. Checks
cover oriented distances, center/threshold boundaries, actual native Area contact
and all-bit/low16/missing-material/selected-shape cases, repeated contact predicates,
compiled grant suspension without duplicate grants, rejected missing stock,
KillSelf removal, inventory snapshot file reload, independent resource ownership,
and native frame indices 5/6. The compiled lifecycle fixture uses a synthetic
two-frame wait; it is not exported unchanged Manor AI or a barrel-explosion test.

The local static check imports Manor in memory only: template/catalogue joins,
four exact initial actor identities/selectors, complete Perso/model Rete round
trips, transient drop flags and independent barrel field-8 edits. It does not
export/register content, run the root scene, or prove production save/reload.

The first static check exposed the ninth template's absent stock and failed;
that mistaken assumption was corrected against its direct-effect source AI.
The first native fixture exposed an invalid test inventory member and missing
gold snapshot binding; both fixtures were corrected without production changes.
No failed run is counted as passing. Focused final results are recorded in the
isolated ignored `output/evidence/focused-final.log`; raw targeted source evidence
is local `manor-source.json`. Whole-game and original-engine acceptance remain
pending. No release-wide coverage is claimed for this optional unregistered slice.

Final result: **2 passed**, 0 failed, 0 filtered, 0 ignored; 0.73 seconds test
execution after 18.61 seconds incremental release build. Godot was
4.7.2.stable.arch_linux.ed1daf0bf. `cargo fmt --check`, staged diff whitespace
checking, and targeted `cargo clippy --release --locked --offline --test
manor_pickup_barrel -- -D warnings` passed (Clippy 10.81 seconds). The initial
isolated dependency/release build took about 75 seconds. Temporary native
projects/processes were released by the Rust harness; evidence/build directories
remain local and ignored. Production test registration remains a lead-owned step.
