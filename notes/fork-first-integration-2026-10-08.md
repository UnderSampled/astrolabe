# Fork-first integration ledger — 2026-10-08

## Integration record

This review branch starts at original fork commit `294bd260fcfa6d4c53b99799e60bce14614f2984` (parent `00adc8a41860e295fee308cd730bac6421ff0b05`). It merges the agreed main checkpoint `cecf9ebf082d8804dfbee0086627c716c97ebed3` as merge commit `56a769f`. Fork and main worktrees remain unchanged; no push, reset, or delete was performed. The branch is `codex/fork-first-integration-20261008`.

The checkpoint is an ancestor of the final review branch. It supplies the newer menu, HUD, loading, level transition, camera, semantic save, and game/session ownership work. In particular, its saved sideways-gate restoration is retained; the fork's raised-gate workaround does not override it. OpenSpace continues to own source address resolution, while Rete and Godot use semantic assets and references.

## Keep, supersede, and remove ledger

| Disposition | Change | Reason and source evidence | Validation evidence |
| --- | --- | --- | --- |
| Keep | Fork's shared typed AI walker and source-backed AI/configuration import, including configuration expressions and native configuration resources | The walker is shared by the AST consumers; source allocation identity is mapped at the OpenSpace import boundary. Configuration discovery follows source registration/composition in `src/openspace/level/scene.rs`; Godot consumes the semantic reference. The checkpoint already has shared armor/AI work, so this branch reconciles rather than duplicates those paths. | AI and scene focused selections; `cargo check --locked --all-targets`. |
| Keep | Fork's source-backed armor catalog and native armor state | Source importer and native Resource behavior are distinct from the independently integrated checkpoint armor/AI code; retain one source path and one Godot implementation. `src/openspace/level/armor.rs`, `src/model/armor.rs`, `src/godot/armor.rs`; source routines and static evidence are recorded in `notes/armor-source-2026-10-04.md` and `notes/armor-ai-port-2026-10-07.md`. | Armor and AI selections; all-target compile. |
| Partial keep; session wiring superseded | Preserved level-preparation/input helpers and shared native inventory gameplay/catalog | The shared inventory/catalog is integrated. `level_preparation.gd` and `crossbow_input.gd` remain packaged, but the actual newer session uses `LevelTransition` and inlined crossbow routing; these disconnected helpers do not establish component-refactor completion. | Save, AI, scene selections and all-target compile establish the retained slices; current session wiring is documented in the project guide. |
| Keep with regeneration boundary | Native configuration payload-bank metadata contract | Current `src/godot/character.rs` emits each configured payload as a child of a configured-slot anchor and sets its `source_configuration_bank` NodePath meta before saving. `godot/addons/astrolabe/character/character.gd::_apply_source_configuration()` consumes that metadata. The preserved route project came from the separate main head recorded in its `frozen-manifest.json`, still had interface 58, and its Hype prefab instead stores direct `MeshInstance3D` `part_*`/`reset_part_*` nodes with no `source_configuration_bank` metadata. Updating only its support to interface 60 therefore produced 18 missing-meta runtime errors. This comparison points to stale generated content in the acceptance copy; it does not establish a current-branch exporter regression. Do not suppress the error or invent a fallback until a current-branch generated project is checked. | Static contract comparison: current exporter line 815, support line 1082, frozen Hype prefab `Hype__891c7341e3.tscn`, and preserved manifest. Route log records 18 occurrences. A current-branch re-export remains needed to close this check. |
| Keep | Main checkpoint's settings, menu/HUD/loading, level transitions, camera and semantic-save behavior | These are newer accepted main changes, including the real saved gate restoration and menu/session ownership. They remain ahead of older fork behavior. | Save, scene, and runtime safeguard selections; main branch status checked separately. |
| Keep | Native texture mip export (`bdc8113e155104a5d69f7a3127da60d07ba97fc4`, branch commit `eab0f77`) | Texture mips are preserved by the common `Project::save(Texture)` path. No separate baked menu-art or FontFile atlas call-site is warranted; those remain base-only. | `tests/texture_mips.rs`; media focused selection. |
| Keep | `AdjustPersoOnObject` operands then waypoint containment/category-29 values (`d0cd281baad4d022bb53f7d4ab19602dd982a3ba`, `365c4da33090eb1caaa80ba4be0ffca2af673fcc`; branch commits `f6d7d3b`, `5cdc172`) | Preserve typed operands, shared waypoint identity, positions and category-29 source values rather than reducing them to display-only data. | AI, movement and Rete focused selections; `tests/ai.rs`, `tests/routes.rs`, `tests/real_routes.rs`. |
| Keep | Manor model-334 source branch, raw-207 actor dispatch and actor-query harness fix (`bdcd659e8d094630d44d70b3af7fdd86dd07b353`, `90f4726b6507b3bc6075667b8d728f5584b02457`, `a301a336b847b44ce41f5a8b48316f919a7db132`; branch commits `e1cc0a0`, `d23c99c`, `61fa12d`) | Source dispatch remains source-backed and the focused harness no longer advances gameplay while querying. Patch-equivalent actor query `86b7992` was not replayed. | AI focused selection; `tests/ai_behavior_actor_query.rs`, `tests/ai_collision_wp.rs`. |
| Keep | Current-animation-frame query (`3e77cb0b8137532034e62d1210c2f527157a7239`; branch commit `8384dbc`) | The typed query is exposed through the existing native AnimationPlayer state. | Animation and AI focused selections; `tests/ai_animation_frame.rs`. |
| Keep | Reached voice/effects fades (`c4890a6e8020110a1e6a64b0fa6df1b6d18dbeae`; branch commit `54b1c2f`) | The source trace reaches the voice/effects fade procedures and maps them to native channels; source evidence is in `notes/ai-audio-fade-static-trace-2026-10-08.md`. | Media and AI focused selections; `tests/audio_godot.rs`, `tests/check_audio_fades.gd`. |
| Keep | Deferred settings node lifetime fix (`12db1a5492b40173a49ec1ac2b05cbd7e847d0b4`; branch commit `809addb`) | The deferred node can outlive its settings owner; resolve by instance identity before access. | Save/runtime focused selections and project smoke checks. |
| Absorbed; no replay | Brigand AI preservation (`ea0550422450a0ef121ed40ccbefaf49d3516e36`) | Three-way conflict resolution showed its source behavior was already present in the fork-first baseline. `git cherry-pick --continue` reported an empty result; it was skipped rather than creating a redundant patch. The resulting source retains the opaque/unsupported AI preservation behavior and actor-query diagnostics in `src/openspace/level/ai.rs` and `src/godot/ai.rs`. | AI focused selection; all-target compile. |
| Supersede current wiring; broader save capability comparison assigned | Fork's raised-gate workaround, older game/session flow and schema-3 save envelope | The main checkpoint restores the saved sideways gate and snapshots the actual `dsg_149` game variable. Keep that source-backed behavior and the checkpoint's newer menus, HUD, loading, transitions, camera and schema-1 save flow. The richer fork save capabilities were not composed into that envelope; current replacement is not proof all useful capability is obsolete. The assigned save worker owns semantic comparison and coherent integration. | Save and scene selections; static inspection of `godot/game/demo.gd` and `godot/game/save_state.gd`. |
| Remove | Optional `Perso.stock` ownership binding/count from serialized Rust model and Godot actor export | The explicit integration constraint keeps persisted Rust ownership metadata exploratory. Removed the field, OpenSpace stock join, Godot `source_stock` property/resource and associated test-fixture fields; shared inventory APIs/catalog remain. | Rete and AI selections; serialization/source model compile. |
| Integrated | Semantic Brain projection and optional Global intro latch (`57f9092`, `09db7bd`, `f7d01209`; production commits `2337318`, `8a344d5`, `1c5e5f5`, `be0b88e`) | Schema-1 optional Boolean `fix:/persos/15`.brain.dsg_147 captures/restores before deferred AI start. Older slots may omit it and preserve scene defaults; no address, registration flags, or key-grant inference enter gameplay state. | Six save-import unit tests, all-target cargo check, and authentic supplied-save import passed. Imported value was true alongside dsg_149; source details remain in the diagnostics sibling. Save-owner fresh-process capture/reload passed before integration; integrated Godot persistence and passive New/Load Brain/dialogue observations remain pending. |
| Partial keep; intro acceptance held | Selective SourceCamera sector-history seed hunk from candidate `540316614a120e7d0ad3bf9bbb55a24f3e5dd414`; the full candidate commit was not replayed. The registered camera Perso is outside the static sector hierarchy, so the existing source `004e0020` point resolver seeds its initial sector. Keep `Function_Camera_GetShiftAlpha` unsupported until source body/trace evidence establishes inputs, formula and return value. | Focused camera regression `exported_camera_uses_edited_horizontal_fov_and_source_view_axis`: 1/1 passed. Authored New Game-only run on Godot 4.7.2 reached the first dialogue and confirmed startup SourceCamera current/active, targeting Hype. At +2 s and post-dialogue, model 437 stops on node 20, category 3/op 59 (`Function_Camera_GetShiftAlpha`, `0x0053d300`, LP); camera `current` and active path persist. Hype model 28 independently stops at node 1078, category 4/op 237 (`Proc_SetVisualGMTDiffuseCoef`, `0x00543230`, CL), first observed at startup. No source widescreen toggle; widescreen, letterbox and HUD suppression remain false. The log also records 18 missing `source_configuration_bank` meta errors at `character.gd:1082`. Full report/log/screenshots: `output/fork-first-newgame-camera-20261008/newgame-camera-20261008/evidence/`. |
| Integrated; bounded route accepted | Lifecycle-safe recovery `d94f347` (production commit `11a3cbc`) plus route smoke fixes (`ecb6a1a`, `ee4341e`, `d61a6ae`, `a3fd944`, `0fcdd29`) | Recovery holds a persistent presenter reference across awaits, cancels when the session exits, and reports a failed scene change. Smoke waiters use stable instance IDs and guard detached-node snapshots; GUI input uses the configured viewport dimensions. | Godot 4.7.2 frozen-export acceptance exited 0 with `NEW_GAME_AND_LOAD_GAME_ROUTES_OK`: New Game, original-import Load, native schema-1 precedence, and malformed-import dismissal/recovery passed. Fixture was restored, native slot removed, and no script/get_path errors were reported. Evidence: `output/frozen-manor-newgame-camera-20261008/route-acceptance-clean-20261008/evidence/`. This frozen project predates the `dsg_147` integration and current exporter/support; do not treat it as validation of that save field or a fresh full-branch export. |

The patch-equivalent `git cherry` entries `96c3547`, `ab4e6c3`, `65d07c6`, `bdf0b1e`, `bc6754d`, `8a3fed0`, and `b53ebfe` were not replayed. The menu texture call-site was not added because the general texture-save path already registers mip sources and baked/font assets remain base-only.

## Deferred Manor acceptance backlog

These are follow-up acceptance items after the integrated runnable-game checkpoint; they do not block this finite fork-forward batch:

- Training-dummy hit and spin reactions.
- Player death damage and damage from falling off cliffs.
- Fresh Manor courtyard collision acceptance and the location-era title-window investigation. The current title-window candidate remains unresolved: the source opcode/name mapping and visible behavior labels disagree, so it is not yet evidence for a generic level-entry cue.

Inventory, animation, and save/title/courtyard owner work remains in isolated later increments. The focused source-camera startup check is positive, but camera/letterbox and New/Load success/error-recovery acceptance remain open; no unsupported camera formula was added from a name/address alone.

## Checks

- `cargo fmt --all -- --check`: passed on the final source diff.
- `cargo check --locked --all-targets`: passed on the final source diff (3.32 s).
- Movement focused selection: 131 passed, 119 skipped, 356 discovered (`TestResults/fork-first-movement-20261008-3.log`).
- Focused camera regression: 1 passed, 4 filtered. The first attempt hit `/tmp` quota during fixture setup; the successful rerun used a worktree-owned `TMPDIR`.
- New Game-only authored route: failed closed at camera/model support and did not observe the aperture toggle. It reached Gogoud actions `0 → 26 → 24 → 23` and first dialogue. Godot logged that X11 was unavailable and fell back to Wayland despite `xvfb-run`; the three viewport screenshots, JSON report and engine log are preserved under the evidence directory above. The command was interrupted during cleanup after the failed report had been written (shell exit 130); no rerun was performed.
- No original-game run, debugger harness, installation, or copyrighted asset commit was performed.

## Oct 8 authored New Game follow-up

The existing interface-65 Manor export copy was advanced with the maintained
support updater to interface 66, then exercised through its real
`bootstrap.tscn` menu and New Game route (headless Godot 4.7.2; evidence under
`output/worktrees/fork-first-integration-20261008/output/fork-first-newgame-camera-20261008/final-interface65-raw343-20261008/evidence/intro-attempt-8/`).
The corrected full harness reached Gogoud actions `0 → 26 → 24 → 23` and the
first item `gogood_texte1_manoir`. Two configured Space/`Button_Hit` selections
advanced through `gogood_texte1_manoir` and `gogood_texte1a_manoir` into model
210, normal 5/action 33, with no unsupported operation reported by Gogoud.
However, the run observed no SourceCamera position or angle change: the camera
remained current and targeted at Hype, with zero measured delta. The authored
widescreen state remained false and no letterbox/aperture cue appeared. Hype's
position was identical in the startup, two-second, first-dialogue, and
follow-up snapshots while Gogoud moved, so this route verifies camera
selection/targeting but does not exercise follow motion with a moving target.
Camera-motion and later-story acceptance therefore remain open; these
observations do not establish that model 154 caused the lack of motion.

The runtime log separately identifies an unsupported opening operation on
`level:/ai/models/154` (`IWorld_Manoir`): source node 3, category 4/op 312,
`Proc_SetGameState`. It remains fail-closed because this checkout has no
verified source implementation contract. Other unrelated Manor brains also
report unsupported operations. Preserve the exact model/node/op diagnostic;
do not infer a causal link to the missing camera movement from this run.

The maintained dialogue screen routes `Button_Hit` to `Proc_SelectDialog` only
while an item is active. `godot/tests/dialogue_input_smoke.gd` passed on the
updated project. The focused Rust AI, diffuse, camera-getter and HUD tests
passed (30 tests, one local-data HUD test ignored); `cargo fmt --all -- --check`
and `git diff --check` passed. Support interface is 66 and Rete schema
54. No original-game runtime, save, source asset, or generated asset in Git was
touched.
