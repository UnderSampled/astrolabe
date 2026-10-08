# Manor death and fall: static implementation, 2026-10-08

Base: Astrolabe `d493ef24859d9b2a0b18ed68f5ebb845e7feeebb`, isolated branch
`codex/death-fall-static-sol-20261008`. This change connects source-backed fall
and death API calls. Camera reset and game-over presentation remain integration
dependencies; complete Manor acceptance is pending. No game, Godot, replay,
harness, compilation or test was run. Kip owns merging and runtime acceptance.
Main and peers' worktrees were read only; no generated assets changed.

## Evidence and search boundary

Read the existing actor/AI audit at
`output/integration-resume-20261008/source-audit/`, the older raw-script inventory
at `output/ai-research/`, Raymap's `AITypes_Hype` catalogue, existing Ghidra bodies,
the patched executable and reconstructed C. The audit predates some baseline
parser promotions: an audit `Unsupported` fragment is not proof that the current
exporter still lacks that operation. Scanning unsupported fragments alone missed
already typed `ApplyArmoredDamage` calls. The final trace includes all canonical
statements/expressions and independently scans raw Fix/Manor script allocations,
including normal behaviors and reflexes.

| Read-only fixture | SHA-256 |
| --- | --- |
| `reference/hype_patch/MaiDFXvr_bleu.exe` | `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d` |
| `disc/Gamedata/World/Levels/Fix.sna` | `36b5557a7d9d8d1f7636fcf88f945dc056ed1484f2b169322b13561c2099360e` |
| `disc/Gamedata/World/Levels/manoir/manoir.sna` | `9dae1c2612535eab0eb823f838bdb845d0d1dccba26f3c0814d81f3541c51af4` |

The raw allocation scan checked 462 Fix and 330 Manor script allocations. It
found armored damage calls in Fix Global, the circles cheat actor and the barrel
damage actor. Manor's explicit procedure 0 belongs to the checkpoint tester's
cheat-reset branch. No raw procedure 8/14/56 or function 37 was found in this
slice. This bounded scan does not establish behavior in other maps or scripts
created dynamically outside those allocations.

Reconstructed source authority:

- `/home/stephen/Projects/hype-decomp/src/ai/ai_hit_points_query.c` and
  `src/ai/continuations.c` were already complete.
- Translator commit `d24078a` in
  `/home/stephen/Documents/Codex/2026-10-08/task-18/hype-sol-manor` contains
  `src/ai/life_damage_procedures.c`, `economic_placement.c`,
  `src/gameplay/life_bytes.c`, `armor_damage.c`, `player_death.c`,
  `initial_placement.c`, and its source-call/evidence reports.
- Final translator checkpoint `dd59ab7` adds three sector/geometry query bodies
  after `d24078a`: 45 complete bodies, 75 matching PE spans and nine missing-raw
  recoveries in the final report. Its geometry queries do not change the traced
  death/fall API contracts or establish all upstream actor collision filters.
  Native contact/sector equivalence remains unverified. The final source handoff
  is `research/worker-deltas/sol-manor-handoff.md` in that source worktree.
- Interaction translator commit `f474a3a` in
  `/tmp/hype-sol-key-training-20261008` contains the complete Proc12 helper in
  `src/ai/training_receivers.c`. This branch reads it; it does not duplicate C.
- Proc307/308's complete recovered dispatcher was supplied read only by the
  source translator. Presentation authoring remains with its owner.

Direct PE call sites and complete caller bodies bound the engine search. Life
getter callers are display ratios, HP rendering and Func35..40 queries; life
writer callers are initial/options changes, actor initialization and byte API
wrappers. The armor damage helper has the shared Func224..227 dispatcher as its
direct caller. Literal phase-8 producers in the inspected contexts are the three
death helpers. No automatic zero-life observer appears in these inspected bodies.
Dynamic phase arguments and indirect calls are outside that conclusion. The
actual Manor death producers below are established script paths.

## Actual Manor trigger and recovery paths

Hype is `fix:/ai/models/28`, Manor main actor `level:/persos/world/0/0`; initial
life/current, maximum and limit are independent bytes, initially 50/50/50.
Hype reflex 3's floor-loss rule tests absence of ground/slope contact, records
fall flags/time and selects normal 3 or 23 with reflex 4 according to equipment.
Normal 3 redirects to 93; normal 23 redirects to 92. Empty normal 16 is different.
Contact and these already typed action/behavior operations remain native.

| Producer | Exact source condition and route |
| --- | --- |
| `iTombe_Falaise`, `level:/ai/models/759`, actor `level:/persos/world/0/144` | Normal 2 enters its watch within distance `< 7` of DSG waypoint 0 (`level:/waypoints/17-1/34`). Normal 0 samples MainActor position after 300 ms. Saved `z < 0` selects camera state 3, stores Hype's fall position, sets Global DSG201=1, selects Global normal 2 and own normal 1. Recovery waits 1000 ms, turns Hype toward (-1,0,0), then Proc12 places Hype at (-55,144,15.75) and camera at (-51,143,19), waits 3000 ms and resumes its watch. |
| `iManoirAll_Dummy_TombePont1`, `level:/ai/models/855`, actor `level:/persos/world/0/148` | Normal 0/1 samples position every 500 ms; the two branches test saved `z < -2` / `z < 0`. Both select camera state 3, set Global DSG201=1, select Global normal 2, wait 750 ms, then use Ultra Proc445 and Proc415 to place Hype and camera at the authored branch position. Normal 2 disables this watch while the cliff watcher owns it. |
| Fix Global, `fix:/ai/models/1`, actor `fix:/persos/15` | Normal 1 waits 1000 ms, tests MainActor `Position.z < -1000`, sets DSG201=0 and selects normal 2. This is a game-over safety threshold. It does not request the fall rescue branch. The level actor named Global without a Brain is different. |
| `iDummy_PourBarils`, `fix:/ai/models/671` | Normal 1 places itself at actor-valued DSG7's position and tests distance to Hype `< 3` with Hype DSG32=0. It assigns incoming damage 10, records front/back facing, then tests `ApplyArmoredDamage(10) < 1`. True selects Hype normal 24; otherwise normal 15. It is the only canonical Manor/Fix caller selecting Hype normal 24 in the inspected slice. |

Those positions and thresholds remain source AI operands. Native session code
contains no fixed-coordinate kill or hard-coded watcher/behavior-number check.

Global normal 2 disables Hype controls through source fields and switches the
microphone to camera. DSG201=1 selects recovery: fade 0 to 255 over 0.8 seconds,
wait for fade direction idle, clear DSG201, restore microphone, then select Hype
94 or, if already marked dead, 95. After waits and Camera_Reset it assigns Hype's
damage fields and applies armored 10 unless its source immunity field suppresses
damage. Remaining life `< 1` marks Hype dead and selects 95; surviving life clears
the temporary damage flag. It fades back and selects Hype 96, which restores
presentation/control fields and a playable behavior according to dialogue state.

Hype 24 and 95 disable reflexes, close an active dialogue, mark death/control
fields and perform equipment-dependent death animations. They activate the
`iHypeStatue` actor (`fix:/ai/models/688`, Manor actor `level:/persos/world/0/16`)
in normal 3, then end in empty Hype normal 55. Statue normal 3 takes Hype's source
position/sighting, shows and animates its configured statue; normal 0 waits
1000 ms, selects Global normal 2, then enters empty normal 1. With DSG201=0 this
selects game over: fade sound channels, fade to black and wait, stop music,
request bank 0/event 75, then Proc307 displays `multilanguage\\gameover.bmp` for
9000 ms and requests main-menu phase 4.

Thus reaching life zero is acted on by the source caller's explicit branch.
The life Resource must not automatically invoke death or reload a checkpoint.
Original cliff recovery uses authored AI placements. No automatic disk-checkpoint
restore has been established by these paths.

## API contracts carried into the native port

| Semantic API | Source contract / native implementation |
| --- | --- |
| `PlayerIsDead` (Proc0) | No operands: effects cleanup, then speech cleanup, then original engine phase 8. Model, walker, reader/writer and compiler call the native session death request; native terminal policy is described below. |
| `SubtractHitPoints` / `SubtractAndGetHitPoints` (Proc56 / Func37) | Evaluate actor then amount on the original caller; capture actor before reusing the result slot for amount. Real amount truncates, then narrows to its low byte; subtract with zero floor and query returns remaining byte. Existing actor-owned life component preserves independent maximum/limit. |
| `ApplyArmoredDamage` (Func224) | Source consumes the raw payload word even for a real operand. Compiler now preserves the float's IEEE payload low byte instead of converting it numerically. Native armor already narrows unsigned damage, consumes/wraps piece strike bytes, uses factor `[rank*(count+1)+broken]`, truncates and applies the minimum byte. Remaining life controls the source branch. Native RNG preserves selection distribution; exact original RNG/x87 parity is not claimed. |
| `WaitEndOfFade` (Meta61) | Tests shared fade direction, returning wait/complete. It does not place, reset or damage actors. Compiler uses the ordinary cancellable behavior coroutine and screen's component-owned direction/Tween. Equal fade endpoints preserve an existing direction. Out-of-byte alpha, nonfinite or negative duration is rejected; Manor uses ordinary endpoints. Zero-duration native completion is immediate, an unverified frame-timing difference from source draw-time completion. |
| `ChangeActorSighting` (Ultra Proc445) / sighting field | Captures selected receiver before evaluating direction on original caller. Calls existing native orientation API; dot Sighting reads existing source-coordinate sighting. This connects watcher turns and statue orientation. |
| `SetEconomicActorAbsolutePosition` (Proc415) | Captures selected receiver/matrix before evaluating one vector. Source writes translation, updates sector/relative pose and snapshots dynamics. It does not activate, reparent or clear velocity. Native global position uses Godot transforms, preserving velocity and activation. It shares existing native translation API while retaining a distinct typed procedure for source reconstruction. |
| `ActivateActorOnPosition` (Proc12) | Actor first, vector second, both on original caller; no Ultra consumed by dispatcher. Helper offsets z by +0.5, snapshots pose, restores z, performs its placement/contact step and activates regardless of placement results. Native uses bound CharacterBody's raised pose and half-unit `move_and_collide`; clears retained/native velocity only on contact. Missing dynamics skips placement but still activates. Original collision solver parity requires runtime comparison. |
| `DistanceToWaypoint` (Func11) | Ultra receiver captured first; waypoint evaluates on original caller. Canonical operand is now an expression, including a waypoint DSG slot. Native waypoint Resource resolves its current linked-actor/world position, then existing native Euclidean source-space distance is used. Literal-only parsing previously blocked the cliff watch. |

Research identities stay at the source boundary. Proc0's helper is 00437cc0;
Func11 dispatch 005378c0 and complete helper 0052e810 were read independently.
Proc415's recovered 0053db00 and Proc12's recovered 00540080 have different
snapshot/reset/contact contracts. Godot owns native transforms, physics and
parent hierarchy; it does not rebuild source matrices, dynamics buffers, sector
stream scheduling or a source AI engine. Dynamic parent-change and collision
solver equivalence are not statically established.

Proc8's recovered shared dispatcher evaluates/discards its first operand, then
evaluates the second, truncates a real and tests the complete word nonzero before
optional initial placement. Proc14 places explicit markers before cleanup/death.
Initial placement takes first marker1 and marker2 from own-sector character
order and fills missing markers from activity-sector order, without activation
filter/deduplication/early termination. Both markers are required. These variants
remain preserved unsupported fragments: nearest-checkpoint selection would not
reproduce their contract.

Proc307 evaluates image then duration with original caller. Signed raw DWORD
duration > 0 is required; positive calls display before requesting phase4.
Nonpositive performs neither. Proc308 requests phase2 after the same display.
Neither procedure nor Meta61 establishes placement or checkpoint restoration.

## Native terminal death and save policy

The original death phase byte differs from Proc_SetGameState's shared selector
mask. Its inspected lifecycle reparents/purges source actors and initializes
actors/Brains in memory. This branch does not equate selector 8 with phase 8 or
claim to implement that lifecycle.

The authored game session handles explicit `source_player_is_dead` using the
bounded terminal policy permitted by the gameplay plan: stop converted effects
and speech emitters (shared music stays separate), stop level Brains/body motion,
disable level processing, freeze authored follow camera, close pause/inventory,
release gameplay inputs and disable saving. Its death flag is transient. R
explicitly reloads current session scene; Escape returns to menu. Reload uses
existing New Game/saved-game route, without automatic life refill or save.
Death during initial loading/restoration reports the session API unavailable.

`godot/game/save_state.gd` and schema 4 are untouched. Independent life snapshots
and settled, alive checkpoint admission remain authoritative. Terminal session
is not ready to play; no admission exception or checkpoint policy is inferred
from original phase/reset code.

## Integration and static verification

Support interface 77 becomes 78; Kip must allocate a newer shared interface if
another worker uses the same value. Shared AI files, `character.gd`, `screen.gd`
and support version files can overlap others. Baseline authored session is
`godot/game/demo.gd`; Kip's rename worker owns `game_session.gd`. Apply session
hunks to renamed file, retaining UIDs. R uses `scene_file_path` and introduces
no old scene filename. This worker makes no scene/resource renames.

Remaining connected dependencies, established from the specific source path:

- **Camera owner:** Global recovery calls Proc136 Camera_Reset in both branches;
  Hype24's conditional camera branch also calls Proc166 followed by Proc136.
  These APIs remain unsupported at this baseline and need exact source contracts,
  not a guessed reset shortcut. Ordinary camera state3 is already typed.
  No camera peer file changed here.
- **Presentation owner:** game-over Proc307 must consume image/duration exactly
  and display before menu transition. Original contract is recovered; native
  presentation/export implementation is outside this worker's ownership.
- **Aggregate integration:** Fix Global must receive its actual music/resource
  bindings. An older audit-only script says a music binding was unavailable;
  that is evidence about that audit, not a current conversion failure assertion.
- **Placement variants/lifecycle:** optional/explicit death placement needs
  semantic marker/order data; original phase8 automatic reset stays separate from
  native checkpoint saves. Those paths are not used by the discovered ordinary
  Manor fall watchers and must not be faked through a checkpoint load.

Static checks: reviewed matching procedure/function IDs, reader/writer operand
order, expression/statement validation and traversal, receiver capture, guarded
compiler lowering, source input bytes and raw/direct-call scan boundaries.
Ran `rustfmt --edition 2024 --config skip_children=true` on five changed Rust
files and `git diff --check`; formatting parses syntax, not types or runtime
behavior. Existing two waypoint fixtures were adapted to canonical expression/
resource operand; no tests were added or run. No compiler, build, Godot syntax
check, source C test or execution was performed.

Kip still needs generated Brain syntax and conversion/roundtrip verification,
then original/native comparison of cliff and both bridge watchers, armored damage
at surviving/lethal values, Hype/statue death sequence, camera reset, game-over
image timing/menu transition, coroutine cancellation, control/camera lock,
explicit reload routes and unchanged checkpoint rejection. No runtime acceptance
is claimed here.
