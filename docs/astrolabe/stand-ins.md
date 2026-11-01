# Stand-ins

A stand-in is a documented guess at what a source AI call should do, used until
game-accurate behavior replaces it. It keeps the game running without pretending
the call is implemented. Silent no-ops are not stand-ins.

Each one is findable and tracked:

- `src/godot/stand_ins.rs` holds the guess and any returned value.
- Compiled scripts call `stand_in("<id>", "<context>")` at the source location.
- `Behavior.stand_ins_reached` counts what a run actually reached, and the gate
  prints it.
- `grep -rn 'stand_in("' godot/generated` lists every compiled site.

Brains are strict by default, which every test relies on: a stand-in or runtime
guard stops the Brain with `unsupported_failure`. `game/bootstrap.gd` turns that
off for the game. `ASTROLABE_STRICT_BEHAVIOR=1` keeps it on.

Runtime guards (`fail_unsupported`) have one shared stand-in, recorded under the
id `guard`: abandon the current pass. A rule retries on the next tick, while a
schedule ends until its behavior is entered again. Calls with no table entry use
the `generic` stand-in, which skips the call and reads as zero.

To retire a stand-in, implement the call, delete its table row and delete its
row here. Replace them in order of what blocks play.

## Table

| Id | Source call | Current guess | Game-accurate follow-up |
| --- | --- | --- | --- |
| `3-189` | MTH_Modulus | Nonzero remainder, so periodic regeneration does not tick | Type both operands and compute the remainder |
| `1-65` | Cond_IsValidWayPoint | Waypoint is invalid | Check the waypoint reference |
| `1-118` | Cond_Inv_InventoryIsFull | Always has room | Ask the inventory Resource |
| `1-119` | Cond_Inv_FindObject | Object is not carried | Ask the inventory Resource |
| `1-32` | Cond_CollideWithWall | No wall contact reported | Read native contact |
| `3-197` | Func_Inv_RemoveObject | Nothing is removed | Inventory removal |
| `3-199` | Func_Inv_AddGold | No gold is added | Gold through inventory |
| `4-417` | Inv_UnselectSelectedObject | Selection is kept | Clear the selection |
| `4-438` | Magic_SelectNextSpellType | Spell is kept | Cycle the spell type |
| `4-334` | Proc_ActivateInventoryMotorMode | Authored Tab host owns the screen | Open it from AI |
| `4-374` | Proc_ActivateMapMotorMode | Map does not open | Native map screen |
| `4-336` | Proc_ActivateSaveGameMotorMode | Authored F5 save owns saving | Open the source save flow |
| `4-271` | Map_vSetDestination | Not recorded | Native map state |
| `4-272` | Map_vResetDestination | Not recorded | Native map state |
| `4-153` | Procedure_Camera_ChangeAngle | Keep native camera tuning | Drive source_camera.gd |
| `4-143` | Procedure_Camera_ChangeDistance | Keep native camera tuning | Drive source_camera.gd |
| `4-154` | Procedure_Camera_Move | Keep native camera tuning | Drive source_camera.gd |
| `4-150` | Procedure_Camera_MoveTarget | Keep native camera tuning | Drive source_camera.gd |
| `4-152` | Procedure_Camera_MoveShift | Keep native camera tuning | Drive source_camera.gd |
| `4-151` | Procedure_Camera_MoveLookTarget | Keep native camera tuning | Drive source_camera.gd |
| `4-175` | Procedure_Camera_SetCameraPosition | Keep native camera tuning | Drive source_camera.gd |
| `4-148` | Procedure_Camera_ChangeCamAngularInertia | Keep native camera tuning | Drive source_camera.gd |
| `4-146` | Procedure_Camera_ChangeCamLinearInertia | Keep native camera tuning | Drive source_camera.gd |
| `4-140` | Procedure_Camera_ShowInfo | Debug text is not shown | None needed if it stays debug-only |
| `3-60` | Function_Camera_GetTeta | Level camera | Read source_camera.gd |
| `3-58` | Function_Camera_GetAlpha | Level camera | Read source_camera.gd |
| `3-54` | Function_Camera_GetDistMin | No minimum distance | Read source_camera.gd |
| `3-55` | Function_Camera_GetDistMax | No maximum distance | Read source_camera.gd |
| `3-70` | Function_Camera_GetState | Default state | Read source_camera.gd |
| `3-75` | Function_Camera_GetLookVertex | Origin | Read source_camera.gd |
