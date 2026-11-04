# Missing AI sites across the whole game

Static inventory, refreshed 2026-11-04. All 37 levels convert. Each was
converted with `astrolabe port disc output/full-game/project3 --level L`
(isolated project, never `godot/`), and the generated GDScript and port logs
were counted with `tools/full_game_ai_sites.py`. Nothing was run in Godot.
Ids are `<category>-<op>`: 1 condition, 2 operator or field, 3 function,
4 procedure, 5 meta action. `generic-` marks a call with no table entry, and
`generic-untyped` a script the compiler could not type.

Fix models are compiled into every level, so their sites repeat per level.
"Unique sites" counts each fix model once.

"Original body" is the state of the original handler in hype-decomp:
- **rewritten**: a clean C rewrite exists;
- **raw**: the original Ghidra export had it;
- **raw recovered**: readable since the dispatch-table recovery
  (`docs/ai-dispatch-table-recovery.md`).

**One binary.** All 37 levels were converted fresh with a single pinned
binary, main e655c98. The run manifest `output/full-game/results3.tsv` has 37
rows, all exit 0, ending `DONE e655c98`. Every one of the 37 directories under
`disc/Gamedata/World/Levels` has a manifest row and a generated level folder.

## Conversion

Every level converts. The blockers of the first pass were fixed in the
converter or turned into documented stand-ins:

| Former blocker | Levels | Resolution |
| --- | --- | --- |
| Null triangle or UV array with a zero count (block (6,2)+8) | toras x4, tour x2, village x3 | Zero-count arrays skip their null pointer; empty submeshes get no Godot surface |
| Mechanics cards 16 and 17 (then 1, 2, 5, 8, 10, 12 to 15, 20) | casino3, ciel x4 | Sizes from 00591464; all are pointer-free (00593eaf copies them flat) and kept unpromoted |
| Theme loop groups | fort, monast2, monast3, toras4 | The source's natural part advance (004fa5c0) unrolled into clips, or one PCM stream above 63 clips |
| Waypoint-array element store ("dsg_N is not an array") | manoir2, manoir4, casino3 | Stores the waypoint Resource |
| Non-finite dialogue justification | manoir3 | Accepted: the source forwards the word unread to the FON draw calls |
| Action states of another family in a shared model | toras | Checked against their own family |
| Zone activation past the geometry's slots | tour3 | Stand-in `zone-past-slots` |
| Real stored into a vector variable | toptower | Stand-in `real-into-vector` |
| Unrelocated way-list word (0x12345678, a fill pattern) | casino | Stand-in `unrelocated-way-list` |

## Priority view

The largest remaining items are:

- **Untyped scripts:** `generic-untyped`, 688 unique sites in all 37 levels
  (967 before the camera Self, previous-behavior and Ultra transparency
  typing). In Manoir and Brigand: 60 sites, down from 180. The remaining
  typing failures there:
  - Ultra operations unsupported for a receiver (14);
  - unbound camera target actor (13);
  - actor-field receivers (13, plus 4 unbound field targets);
  - add-assignment targets that are not designer variables (9);
  - Ultra 387 on a designer-variable receiver (6);
  - Ultra 218 (4);
  - unbound source objects (4).
- **Turn procedures 4-36, 4-37, 4-38, 4-43** (Proc_TurnLeft, Right, Up, Down):
  about 380 sites in 4 levels; raw recovered.
- **Map destinations 4-271 and 4-272:** 160 sites in about 30 levels; their
  bodies are now rewritten in hype-decomp.
- **Camera procedures 4-150 to 4-176:** in all 37 levels.

Totals: 216 stand-in ids with 2752 unique sites;
203 guard templates with 97975 unique sites. Guards
are runtime validity checks that skip a call when a binding is missing.

## Stand-ins by sites (top 35)

Full lists: `output/full-game/final3.txt` and, per level and model,
`output/full-game/sites3.tsv` (regenerate with the tool).

| Id | Entry | Levels | Unique sites | Level-model sites | Fix-model sites | Original body |
| --- | --- | --- | --- | --- | --- | --- |
| `generic-untyped` | (script could not be typed) | 37 | 688 | 687 | 1 | - |
| `generic-4-38` | Proc_TurnUp | 4 | 108 | 108 | 0 | raw recovered |
| `generic-4-43` | Proc_TurnDown | 4 | 100 | 100 | 0 | raw recovered |
| `generic-4-37` | Proc_TurnRight | 4 | 92 | 92 | 0 | raw recovered |
| `4-271` | Map_vSetDestination | 31 | 87 | 87 | 0 | rewritten |
| `generic-4-36` | Proc_TurnLeft | 4 | 84 | 84 | 0 | raw recovered |
| `4-272` | Map_vResetDestination | 29 | 73 | 73 | 0 | rewritten |
| `generic-4-387` | Proc_SND_PersoChangeVolEffects | 4 | 70 | 70 | 0 | rewritten |
| `generic-4-216` | Proc_DeltaTurnPerso | 6 | 62 | 62 | 0 | rewritten |
| `generic-4-176` | Procedure_Camera_SetTargetPosition | 13 | 51 | 51 | 0 | raw recovered |
| `4-175` | Procedure_Camera_SetCameraPosition | 37 | 49 | 46 | 3 | raw recovered |
| `generic-4-218` | Proc_ChangeAction | 26 | 45 | 45 | 0 | raw recovered |
| `4-150` | Procedure_Camera_MoveTarget | 37 | 44 | 36 | 8 | raw |
| `generic-4-379` | Proc_FixNormReelSpeed | 4 | 40 | 40 | 0 | raw recovered |
| `generic-9-1` | - | 2 | 40 | 40 | 0 | - |
| `generic-4-270` | Map_vEnableObject | 6 | 38 | 38 | 0 | rewritten |
| `generic-4-32` | Proc_SetTarget | 20 | 37 | 37 | 0 | rewritten |
| `generic-4-51` | Proc_ReachTarget | 20 | 37 | 37 | 0 | raw recovered |
| `generic-3-28` | Func_RandomReal | 14 | 32 | 32 | 0 | raw recovered |
| `generic-4-356` | Proc_GPI_SetTextNumber | 8 | 32 | 32 | 0 | raw recovered |
| `generic-1-97` | Cond_IsInFrontOfPerso | 8 | 31 | 31 | 0 | rewritten |
| `generic-5-26` | Action_GoToWPXY | 2 | 31 | 31 | 0 | raw recovered |
| `generic-4-165` | Procedure_Camera_SetPosition | 12 | 30 | 30 | 0 | raw |
| `generic-3-83` | Func_GetNormSpeed | 5 | 27 | 27 | 0 | raw recovered |
| `generic-4-240` | Proc_SetVisualGMTTextureScrollingCoef | 6 | 26 | 26 | 0 | raw recovered |
| `generic-1-94` | Cond_IsAtLeftOfPerso | 7 | 25 | 25 | 0 | rewritten |
| `generic-4-350` | Proc_GPI_SetImage | 5 | 25 | 25 | 0 | raw recovered |
| `1-119` | Cond_Inv_FindObject | 37 | 24 | 23 | 1 | rewritten |
| `generic-1-95` | Cond_IsAtRightOfPerso | 7 | 24 | 24 | 0 | rewritten |
| `generic-5-7` | Action_WaitEndOfActionPerso | 23 | 23 | 23 | 0 | rewritten |
| `generic-4-419` | Inv_ChangeJewelState | 8 | 21 | 21 | 0 | rewritten |
| `generic-4-349` | Proc_GPI_SetDraw | 8 | 20 | 20 | 0 | raw recovered |
| `generic-4-357` | Proc_GPI_SlideGamePlayInterface | 8 | 20 | 20 | 0 | raw recovered |
| `generic-1-96` | Cond_IsBehindPerso | 7 | 19 | 19 | 0 | rewritten |
| `generic-4-34` | Proc_GoRelative | 6 | 19 | 19 | 0 | raw recovered |

## Guards (most common templates)

| Guard message (template) | Levels | Unique sites |
| --- | --- | --- |
| <ref> requires main actor Brain field N | 37 | 22064 |
| <ref> actor normal behavior request requires a different target actor with a Brain using <ref> | 37 | 10565 |
| <ref> requires actor <ref> Brain field N | 37 | 9971 |
| <ref> requires source object active state for <ref> | 37 | 2998 |
| <ref> requires camera target state for <ref> | 37 | 2202 |
| <ref> requires a native facing receiver | 37 | 2132 |
| <ref> requires a positioned facing target | 37 | 2132 |
| <ref> requires a live distance receiver and resolved waypoint | 33 | 1944 |
| Local distance needs positioned actors | 37 | 1862 |
| Presentation call requires scene UI | 37 | 1820 |

## Converter diagnostics

| Diagnostic | Levels | Sites |
| --- | --- | --- |
| `openspace.scene.model_coverage` | 37 | 9841 |
| `openspace.ai.unsupported_script` | 37 | 4732 |
| `openspace.character.coverage` | 37 | 2403 |
| `openspace.dialogue.unsupported_item` | 20 | 108 |
| `openspace.ai.opaque_designer_field` | 17 | 39 |
| `openspace.dialogue.coverage` | 37 | 37 |
| `openspace.sector.coverage` | 4 | 24 |
| `openspace.way.unrelocated_list` | 1 | 1 |
| `openspace.environment.coverage` | 1 | 1 |

## Saves and persistent state

| Item | Sites (37 levels) | Port support |
| --- | --- | --- |
| Proc_ActivateSaveGameMotorMode (4-336, lectern save menu) | 6 | Stand-in: saves through the authored save |
| Proc_SaveGame, Proc_IncHistoricAndSaveGame, Proc_SaveAllGameValues; save rotation sector and moving surface | 0 seen | Possibly inside untyped scripts |
| Global Brain (level-carried) field writes | 222 distinct fields, 2377 unique sites | Native manor save keeps a bounded set; elsewhere the `travel-save` stand-in |
| Inventory and magic calls | 819 unique sites, 13 kinds | Shared inventory; `1-118`, `1-119`, `3-197`, `3-199` are stand-ins |

Known save gaps:
- 248 unknown Manoir save identities (`notes/research/unknown-save-fields.md`).
- A port save can be refused or write nothing
  (`notes/research/route-bot-state.md`).
  - `save_game()` returns silently before `opening_finished`.
  - `checkpoint_error` refuses until Hype has settled.
