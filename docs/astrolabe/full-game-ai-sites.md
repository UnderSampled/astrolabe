# Missing AI sites across the whole game

Static inventory, 2026-11-04. Every level was converted with
`astrolabe port disc output/full-game/project --level L` (HEAD 0a6c67a, one
isolated project, never `godot/`). The generated GDScript and the port logs
were then counted with `tools/full_game_ai_sites.py`. Nothing was run in Godot.
Ids are `<category>-<op>`: 1 condition, 2 operator or field, 3 function,
4 procedure, 5 meta action. `generic-` marks a call with no table entry, and
`generic-untyped` a script the compiler could not type.

Fix models are compiled into every level, so their sites repeat per level.
"Unique sites" counts each fix model once. "Original body" is the state of the
original handler in hype-decomp after the dispatch-table recovery
(hype-decomp `docs/ai-dispatch-table-recovery.md`):
- **rewritten**: a clean C rewrite exists;
- **raw**: the original Ghidra export had it;
- **raw recovered**: newly readable in that recovery;
- **none**: no handler.

## Conversion

Of 37 levels, 15 convert and 22 fail. Failures are recorded, not fixed:

| Blocker | Levels |
| --- | --- |
| Required null reference at Level block (6,2) +8 (collision or scene model) | toras, toras2, toras3, toras4, tour, tour3, village2, village3, village4 |
| Unsupported Hype mechanics kind 17 | ciel1, ciel2, ciel3, ciel4 |
| Sound theme loop groups or transition semantics (M_Toras#19, M_Monastere#36) | fort, monast2, monast3 |
| AI assignment target dsg_N is not an array | manoir2 (dsg_5), manoir4 (dsg_6), toptower (dsg_55) |
| Unsupported Hype mechanics kind 16 | casino3 |
| Nonzero pointer lacks relocation (perso world/0/8) | casino |
| Invalid dialogue justification | manoir3 |

Converted: astrolabe, brigand, brigand2, brigand3, brigand4, cachots,
cachots4, cite3, cite4, fort3, fort4, labo, labo3, manoir, monast1. The counts
below cover these 15 only. The failed levels' AI is not compiled, so their
sites are unknown.

## Priority view

By sites, which matter most once a level runs:

1. `generic-untyped`: 582 sites in all 15 levels, plus 2557
   `openspace.ai.unsupported_script` diagnostics. These are whole scripts the
   compiler could not type, and they hide whatever entries they contain.
2. Collision and vector functions: `generic-3-153` Func_GetNormalCollideVector
   (96), `generic-3-220` ReduceIndexOfVectorArray (95), `generic-1-30`
   Cond_CollideMovingZDM (33). Each is in 9 levels.
3. Movement meta actions and procedures: `generic-5-25` Action_GoToWP (55
   sites, 13 levels), `generic-4-217` Proc_TurnPerso (40, 10), `generic-4-215`
   Proc_Turn2 (33, 8), the way procedures 4-222 and 1-68, and
   `generic-4-32`/`4-51` target procedures.
4. Map destinations: `4-271`/`4-272`/`generic-4-270` (90 sites, 6 to 13 levels).
5. The camera procedures 4-150 to 4-175 reach all 15 levels through fix/437,
   but as few sites.

By levels blocked from converting, the null reference at block (6,2) +8 (9
levels) and mechanics kind 17 (4 levels) dominate.

Totals: 86 stand-in ids with 1566 unique sites; 183 guard templates with 47003
unique sites. Guards are runtime validity checks that skip a call when a
binding is missing, not unimplemented calls.

## Stand-ins by levels and sites

The top 45 of 86 ids. The full list is in `output/full-game/final.txt`, and
per level and model in `output/full-game/sites.tsv` (regenerate with the tool).

| Id | Entry | Levels | Unique sites | Level-model sites | Fix-model sites | Original body |
| --- | --- | --- | --- | --- | --- | --- |
| `generic-untyped` | (script could not be typed) | 15 | 582 | 569 | 13 | - |
| `4-175` | Procedure_Camera_SetCameraPosition | 15 | 25 | 22 | 3 | raw recovered |
| `4-154` | Procedure_Camera_Move | 15 | 12 | 0 | 12 | raw recovered |
| `4-150` | Procedure_Camera_MoveTarget | 15 | 8 | 0 | 8 | raw |
| `4-334` | Proc_ActivateInventoryMotorMode | 15 | 4 | 0 | 4 | raw recovered |
| `4-374` | Proc_ActivateMapMotorMode | 15 | 4 | 0 | 4 | raw recovered |
| `4-438` | Magic_SelectNextSpellType | 15 | 4 | 0 | 4 | raw recovered |
| `generic-2-8` | - | 15 | 4 | 3 | 1 | - |
| `4-152` | Procedure_Camera_MoveShift | 15 | 4 | 0 | 4 | raw |
| `1-118` | Cond_Inv_InventoryIsFull | 15 | 2 | 0 | 2 | rewritten |
| `4-140` | Procedure_Camera_ShowInfo | 15 | 2 | 0 | 2 | raw recovered |
| `4-151` | Procedure_Camera_MoveLookTarget | 15 | 2 | 0 | 2 | raw |
| `1-119` | Cond_Inv_FindObject | 15 | 2 | 1 | 1 | rewritten |
| `3-197` | Func_Inv_RemoveObject | 15 | 2 | 1 | 1 | rewritten |
| `4-336` | Proc_ActivateSaveGameMotorMode | 15 | 1 | 0 | 1 | raw recovered |
| `3-75` | Function_Camera_GetLookVertex | 15 | 1 | 0 | 1 | raw recovered |
| `4-146` | Procedure_Camera_ChangeCamLinearInertia | 15 | 1 | 0 | 1 | raw recovered |
| `4-148` | Procedure_Camera_ChangeCamAngularInertia | 15 | 1 | 0 | 1 | raw recovered |
| `generic-5-57` | Action_Camera_WaitUntil | 15 | 1 | 0 | 1 | raw recovered |
| `3-199` | Func_Inv_AddGold | 15 | 1 | 0 | 1 | rewritten |
| `generic-5-25` | Action_GoToWP | 13 | 55 | 55 | 0 | raw recovered |
| `4-271` | Map_vSetDestination | 13 | 29 | 29 | 0 | raw recovered |
| `4-272` | Map_vResetDestination | 11 | 23 | 23 | 0 | raw recovered |
| `generic-5-7` | Action_WaitEndOfActionPerso | 11 | 11 | 11 | 0 | rewritten |
| `generic-4-217` | Proc_TurnPerso | 10 | 40 | 40 | 0 | rewritten |
| `generic-3-153` | Func_GetNormalCollideVector | 9 | 96 | 96 | 0 | raw recovered |
| `generic-3-220` | ReduceIndexOfVectorArray | 9 | 95 | 95 | 0 | raw |
| `generic-1-30` | Cond_CollideMovingZDM | 9 | 33 | 33 | 0 | raw recovered |
| `generic-4-218` | Proc_ChangeAction | 9 | 13 | 13 | 0 | raw recovered |
| `generic-1-68` | Cond_IsValidWay | 9 | 10 | 10 | 0 | raw recovered |
| `generic-4-222` | Proc_InitWayWithWp | 9 | 10 | 10 | 0 | raw recovered |
| `generic-4-215` | Proc_Turn2 | 8 | 33 | 33 | 0 | raw recovered |
| `generic-3-28` | Func_RandomReal | 8 | 18 | 18 | 0 | raw recovered |
| `generic-4-32` | Proc_SetTarget | 8 | 16 | 16 | 0 | rewritten |
| `generic-4-51` | Proc_ReachTarget | 8 | 16 | 16 | 0 | raw recovered |
| `generic-4-270` | Map_vEnableObject | 6 | 38 | 38 | 0 | raw recovered |
| `generic-4-165` | Procedure_Camera_SetPosition | 6 | 13 | 13 | 0 | raw |
| `generic-4-176` | Procedure_Camera_SetTargetPosition | 4 | 20 | 20 | 0 | raw recovered |
| `generic-1-22` | Cond_CollidePersoZDEWithPersoTypeZDE | 4 | 5 | 5 | 0 | rewritten |
| `generic-4-387` | Proc_SND_PersoChangeVolEffects | 3 | 62 | 62 | 0 | rewritten |
| `generic-4-419` | Inv_ChangeJewelState | 3 | 13 | 13 | 0 | raw recovered |
| `generic-1-97` | Cond_IsInFrontOfPerso | 3 | 10 | 10 | 0 | rewritten |
| `generic-4-253` | Proc_SetPersoAbsolutePosition | 3 | 8 | 8 | 0 | raw |
| `generic-5-1` | Action_IncrementalTurn | 3 | 6 | 6 | 0 | raw recovered |
| `generic-4-53` | Proc_SetHitPointsToInit | 3 | 4 | 4 | 0 | rewritten |

Across all 86 stand-in ids:

- 53 handlers are newly readable after the table-dispatch recovery, among
  them Procedure_Camera_Move and SetCameraPosition, the three motor-mode
  procedures, Map_vSetDestination and Map_vResetDestination,
  Func_GetNormalCollideVector, Cond_CollideMovingZDM, Proc_InitWayWithWp,
  Action_GoToWP and Action_Camera_WaitUntil;
- 23 have a clean rewrite in hype-decomp;
- 7 already had a raw body;
- 3 have no handler: `generic-untyped` and two category-2 operator or field
  ids.

## Guards (most common templates)

| Guard message (template) | Levels | Unique sites |
| --- | --- | --- |
| <ref> requires main actor Brain field N | 15 | 10748 |
| <ref> requires actor <ref> Brain field N | 15 | 4947 |
| <ref> actor normal behavior request requires a different target actor with a Brain using <ref> | 15 | 4562 |
| <ref> requires camera target state for <ref> | 15 | 1167 |
| <ref> requires source object active state for <ref> | 15 | 1117 |
| <ref> requires a native facing receiver | 15 | 1078 |
| <ref> requires a positioned facing target | 15 | 1078 |
| <ref> Proc_SetVisualGMTDiffuseCoef requires native shared visual-material state for <ref> | 15 | 990 |
| <ref> requires a live distance receiver and resolved waypoint | 15 | 987 |
| <ref> requires native character channel visibility for operand N | 15 | 965 |
| Local distance needs positioned actors | 15 | 900 |
| Presentation call requires scene UI | 15 | 759 |

## Converter diagnostics

| Diagnostic | Levels | Sites |
| --- | --- | --- |
| `openspace.scene.model_coverage` | 15 | 3653 |
| `openspace.ai.unsupported_script` | 15 | 2557 |
| `openspace.character.coverage` | 15 | 885 |
| `openspace.dialogue.coverage` | 15 | 15 |
| `openspace.dialogue.unsupported_item` | 10 | 35 |
| `openspace.ai.opaque_designer_field` | 8 | 22 |
| `openspace.sector.coverage` | 1 | 6 |

## Saves and persistent state

| Item | Sites (15 converted levels) | Port support |
| --- | --- | --- |
| Proc_ActivateSaveGameMotorMode (4-336, lectern save menu) | 1 unique, in a fix model, all 15 levels | Stand-in: saves through the authored save, not the source save flow (stand-ins.md) |
| Proc_SaveGame, Proc_IncHistoricAndSaveGame, Proc_SaveAllGameValues (4-99 to 4-101) | 0 seen | Not compiled anywhere seen; they may sit inside untyped scripts |
| Save rotation sector or moving surface (4-95, 4-96, 4-125, 4-126) | 0 seen | Same |
| Global Brain (level-carried) field writes | 211 distinct fields, 1233 unique sites. Manoir writes 190 fields; other levels 35 to 51 | Native manor save keeps a bounded field set (save_state.gd capture); outside the manor the `travel-save` stand-in keeps one slot with no per-level records |
| Inventory and magic calls (quantity, add, remove, weapon, magic points) | 481 unique sites, 13 kinds | Implemented on the shared inventory; `1-118`, `1-119`, `3-197` and `3-199` are stand-ins |

Known save gaps:
- `notes/research/unknown-save-fields.md`: 248 Manoir save identities have no
  confirmed meaning.
- `notes/research/route-bot-state.md`: a port save can be refused or write
  nothing.
  - `save_game()` returns silently in a New Game session until
    `opening_finished`.
  - `checkpoint_error` refuses unless Hype has settled ("Save after movement
    and action transitions have settled").
  - Both refusals are silent to the caller.
- Original-save load uses the `load-placement` stand-in.
- Saves outside the manor use `travel-save`.
