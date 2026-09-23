# Hype AI API: behavior requests and procedural operations

Assessment date: 2026-09-14. This is an implementation-direction review, not a
new runtime validation or a claim that the controller works.

**We MUST NOT reimplement the OpenSpace engine within Godot. Implement the AI
script API using native Godot facilities and compile the game's scripts to call
it.** The [gameplay plan](./godot-gameplay-plan.md) applies this boundary to the
next implementation slices.

## Finding

Hype's script API mostly addresses game concepts: actions, movement, contact,
weapons, inventory, spell state, dialogue, cameras and presentation. It includes
both commands requesting behavior and procedural setters. Neither class asks
Godot to reproduce OpenSpace's internal memory or instruction machinery.

A procedure can request a high-level behavior, and a function returning a value
can mutate inventory. The catalogue categories condition/function/procedure/
meta-action describe calling conventions and execution, not abstraction level.
The classifications below describe the visible contract and intended Godot
implementation; they are an architectural assessment, not an engine taxonomy.

## Scope and evidence

The audited set is the 122 distinct callable entries in Hype's entire shared
AI model, across 259 unique scripts. It includes initialization, movement,
combat, inventory, dialogue and other states, not just the supplied manoir save's
reachable controls. It is not the complete dependency closure: camera, Global,
projectile and other referenced actors can add calls. The complete executable
catalogue contains 885 entries; the archived all-entry stubs and this gameplay
assessment are different scopes. Those stubs are no longer active production code.

Names are rebound from raw node kinds/IDs to the verified current
[API catalogue](../notes/research/hype-ai-api.tsv), rather than the older Raymap name
list with shifted indices. Local input evidence is
`output/ai-research/inventory.json` and
`output/ai-research/hype-scripts-verified-api.txt`; neither ships to Godot.
[Retained research](./historical/hype-ai-research.md#verified-executable-catalogue) records
executable identities, table verification and source-game comparisons. Raymap's
`TranslatedScript.cs` independently presents assignments, references and arrays
as ordinary expressions. The original handler at `0054a5d0` reads actor/action/
vector arguments and delegates action/object work; `00540220` dispatches movement
requests to the mechanics service. These addresses are research evidence only.

Counts below are distinct names. Repeated call sites do not represent additional
engine features. This report does not infer implementation status from whether a
named stub has an override: Boolean/comparison calls, for example, lower to
expressions and do not require corresponding runtime API methods.

## What the scripts actually do

- **Walk/run:** normal behaviors 5 and 6 choose different entries in the action
  arrays, execute action sequences, change the current action, switch character
  parts and update HUD state. The action resources and their converted animation
  motion must drive the Godot character. These scripts do not describe a general
  physics solver or skeletal-animation evaluator.
- **Jump:** normal behavior 1 sets vector speed, issues `Proc_GoRelative`, sets an
  impulse vector and calls `Proc_JumpAbsolute`. Normal behavior 2 also pairs an
  impulse with `Proc_JumpWithoutAddingSpeed`. This is a procedural composition of
  a few movement requests and parameters. Godot should implement their local/
  world-space and existing-speed effects using its character movement facilities.
- **Object-producing attacks:** normal behavior 12 and later branches call
  `Action_ExecuteActionAndGenerateObject` with an actor template, action and vector.
  This is a compound gameplay request. Native scene instantiation and action
  playback belong behind that request; exact spawn/attachment timing and lifetime
  still need source-informed verification. It is not an instruction to port the
  source allocator or object tables.
- **Presentation:** initialization pairs `Fight_ChangeArmor` with repeated
  `Proc_SwapLinkTableObjects` calls. The script chooses the visible configuration;
  conversion should resolve those asset slots to native scene parts/resources.
  Channel activation and material/visibility changes operate on those converted
  objects. The names alone do not establish every channel side effect.

## Godot implementation boundary

| API contract | Godot responsibility |
| --- | --- |
| Execute/change an action; turn, jump, move to a target | A character/action component operating on converted animations, native transforms and physics. Preserve command ordering and completion conditions. |
| Execute an action and generate an object | Instantiate the converted PackedScene and coordinate its documented relationship to the action. |
| Query input, contact, position, action completion or game state | Native input/physics/animation observations and ordinary game properties. A slope/material query needs its actual source meaning, not merely an API-name substitution. |
| Inventory, health, armor, spells, dialogue, camera or audio request | Small game services with meaningful typed interfaces, nodes/resources and signals. Godot does not supply Hype's inventory or spell rules automatically. |
| Set speed, impulse, facing, opacity, parts or material values | Direct native property/component changes. Source-specific slot/reference resolution belongs in conversion. |
| Boolean logic, arithmetic, variable/array access, nulls | Compile ordinary GDScript expressions and properties; use narrowly scoped semantic helpers only when needed. Randomness uses a native generator with an explicit behavior contract. |
| Blocking actions and behavior transitions | Generated control flow and appropriate Godot callbacks/coroutines. Preserve rule priority, cancellation and re-entry, without treating source interpreter execution flags as the product API. |

`Proc_SetActionReturn` is the clearest execution-specific exception. Hype's 309
uses have previously been inspected: 303 precede behavior changes and six precede
incremental turning. Their timing matters, but these concrete call patterns are
candidates for conversion into explicit control flow. They do not establish a
requirement for a general OpenSpace interpreter/scheduler implementation.

An action is also more than an animation name: existing evidence shows state
transitions, configuration and motion effects. Using AnimationPlayer is the
foundation, while a small action component preserves the game's contract.

## Correction to implementation priorities

Keep the typed Rust model and source reconstruction at the adapter boundary.
Generated game state should be ordinary Godot properties and native references.
The generated accessor layer and generic execution plumbing were archived and
removed from active production during the rollback. Put the needed translation
in conversion or direct generated code. Do not turn each source
storage quirk into a runtime subsystem or a prerequisite for the first control
sequence.

The next integrated service work should follow Hype's real scripts: native input
and action selection, walk/run motion and contact queries, then jump impulse/
existing-speed behavior, followed by the action/object and inventory/magic calls
needed for sword, crossbow and spells. Initialization and effects reached by that
path still need meaningful implementations; unrelated catalogue coverage does
not come first. Keep the source scripts as the game's logic and implement their
requests in Godot.

The original assessment changed no production implementation. The subsequent
rollback preserves that incomplete experiment, including its failing ActorUpdate
build, at `9374ef5` on `codex/archive-openspace-runtime-20260914`. Active production
returns to `d8d0375`; current validation is recorded in
[implementation progress](./implementation-progress.md). This assessment establishes
direction, not gameplay parity.

## Complete reviewed call inventory

| Classification | Distinct calls |
| --- | ---: |
| Expression / utility | 12 |
| Query | 33 |
| Behavior request | 21 |
| Game service request | 21 |
| Direct state / asset change | 34 |
| Execution control | 1 |

Classification boundaries are judgment calls: an effect toggle is classified as
a direct change, whereas starting a fade is a service request. Both should have
native implementations without reproducing source internals.

### Expression / utility

| API call | Source occurrences |
| --- | ---: |
| `Cond_And` | 282 |
| `Cond_Or` | 172 |
| `Cond_Not` | 110 |
| `Cond_Equal` | 1445 |
| `Cond_Different` | 4 |
| `Cond_Lesser` | 40 |
| `Cond_Greater` | 61 |
| `Cond_GreaterOrEqual` | 9 |
| `Func_Int` | 4 |
| `Func_RandomInt` | 61 |
| `MTH_Modulus` | 2 |
| `Proc_None` | 16 |

### Query

| API call | Source occurrences |
| --- | ---: |
| `Cond_PressedBut` | 181 |
| `Cond_JustPressedBut` | 100 |
| `Cond_ReleasedBut` | 55 |
| `Cond_JustReleasedBut` | 26 |
| `Cond_CollideWithGround` | 26 |
| `Cond_CollideWithSlope` | 23 |
| `Cond_IsTimeElapsed` | 19 |
| `Cond_IsInComport` | 45 |
| `Cond_IsInReflex` | 58 |
| `Cond_IsInAction` | 41 |
| `Cond_IsTypeOfGMTCollide` | 12 |
| `Cond_IsValidObject` | 15 |
| `Cond_IsValidWayPoint` | 2 |
| `Cond_InTopOfJump` | 4 |
| `Cond_IsCustomBitSet` | 9 |
| `Cond_CollisionWP` | 1 |
| `Cond_Inv_InventoryIsFull` | 2 |
| `Cond_GetDialogStatus` | 3 |
| `Cond_DLG_IsDialogOver` | 16 |
| `Cond_ActionFinished` | 24 |
| `Func_GetPersoAbsolutePosition` | 13 |
| `Func_GetAngleAroundZToPerso` | 4 |
| `Func_GetWPAbsolutePosition` | 6 |
| `Func_DistanceToPerso` | 1 |
| `Func_GetHitPoints` | 15 |
| `Func_GetTime` | 40 |
| `Func_GetPersoSighting` | 1 |
| `Func_GetCurrectAnimFrame` | 3 |
| `Func_Inv_GetWeapon` | 18 |
| `Func_Inv_GetObject` | 6 |
| `Func_Inv_GetObjectQuantity` | 55 |
| `Magic_ReadMagicPoints` | 7 |
| `Magic_ReadCurrentSpell` | 1 |

### Behavior request

| API call | Source occurrences |
| --- | ---: |
| `Proc_ActivateObject` | 4 |
| `Proc_GoRelative` | 7 |
| `Proc_JumpAbsolute` | 11 |
| `Proc_JumpWithoutAddingSpeed` | 9 |
| `Proc_ChangeComport` | 11 |
| `Proc_ChangeMyComport` | 326 |
| `Proc_ChangeMyComportReflex` | 62 |
| `Proc_Turn` | 52 |
| `Proc_ChangeAction` | 583 |
| `Action_ExecuteAction` | 453 |
| `Action_WaitEndOfAction` | 1 |
| `Action_WaitEndOfAnim` | 7 |
| `Action_ExecuteActionAndGenerateObject` | 100 |
| `Action_AdjustPersoOnObject` | 5 |
| `Action_IncrementalTurnPosition` | 5 |
| `Action_IncrementalTurnPerso` | 8 |
| `Action_GoToPosition` | 2 |
| `Action_Wait` | 11 |
| `Action_WaitBoolean` | 2 |
| `Action_WaitEndOfFade` | 2 |
| `Action_WaitNbFrames` | 21 |

### Game service request

| API call | Source occurrences |
| --- | ---: |
| `Func_Inv_AddObject` | 2 |
| `Func_Inv_SelectObject` | 1 |
| `Func_Inv_RemoveObjectQuantity` | 11 |
| `Magic_AddMagicPoints` | 1 |
| `Proc_AddHitPoints` | 1 |
| `Proc_SendSoundRequest` | 88 |
| `Procedure_Camera_Reset` | 7 |
| `Procedure_Camera_ChangeState` | 3 |
| `Proc_ExecuteFade` | 11 |
| `Proc_EndDialog` | 15 |
| `Proc_ScrollDialog` | 2 |
| `Proc_SelectDialog` | 5 |
| `Proc_DLG_ChangeComport` | 1 |
| `Proc_ActivateInventoryMotorMode` | 4 |
| `Proc_ActivateSaveGameMotorMode` | 1 |
| `Proc_ActivateMapMotorMode` | 4 |
| `Proc_SND_FadeMusic` | 10 |
| `Inv_UnselectSelectedObject` | 5 |
| `Fight_ChangeArmor` | 6 |
| `Fight_RepairArmor` | 2 |
| `Magic_SelectNextSpellType` | 4 |

### Direct state / asset change

| API call | Source occurrences |
| --- | ---: |
| `Proc_SetImpulse` | 23 |
| `Proc_SetVectorSpeed` | 7 |
| `Proc_AddWind` | 1 |
| `Proc_SwapLinkTableObjects` | 185 |
| `Proc_ActivateChannel` | 422 |
| `Proc_DeactivateChannel` | 444 |
| `Procedure_Camera_ChangeDistance` | 1 |
| `Procedure_Camera_ChangeAngle` | 2 |
| `Procedure_Camera_ChangeFlag` | 6 |
| `Proc_TransparentDisplay` | 29 |
| `Proc_SetTransparency` | 29 |
| `Proc_SetParticleGeneratorOn` | 10 |
| `Proc_SetParticleGeneratorOff` | 35 |
| `Proc_SetParticleGenerator` | 1 |
| `Proc_SetVisualGMTDiffuseCoef` | 330 |
| `Proc_SetPersoAbsolutePosition` | 7 |
| `Proc_InvisibleDisplay` | 4 |
| `Proc_VisibleDisplay` | 29 |
| `Proc_ChangeOneCustomBit` | 8 |
| `Proc_setFadeValue` | 2 |
| `Proc_setFadeTexture` | 12 |
| `Proc_ActivateFade` | 2 |
| `Proc_DeactivateFade` | 5 |
| `Proc_ActivateLaserPointer` | 91 |
| `Proc_HideLaserPointer` | 81 |
| `Proc_SetLaserPointerSniperMode` | 9 |
| `Proc_SetLaserPointerFlyMode` | 1 |
| `Stats_DisplayMainActorLife` | 159 |
| `Stats_DisplayMainActorArmor` | 162 |
| `Stats_DisplayMainActorMagic` | 154 |
| `Stats_DisplayMainActorSelectedObject` | 153 |
| `Stats_DisplayMainActorWeapon` | 149 |
| `Stats_DisplayCash` | 1 |
| `Proc_ChangePersoSighting` | 2 |

### Execution control

| API call | Source occurrences |
| --- | ---: |
| `Proc_SetActionReturn` | 309 |
