# Camera model 437 source audit — 2026-10-08

This is a static follow-up to the model 437 intro camera checkpoint. It uses
the preserved `fix:/ai/models/437` source asset and generated New Game project;
it does not run Hype, modify the preserved stage, or modify the model 28
renderer path owned by the main integrator.

## Reached getter path

In Normal behavior 0, rule 5's no-button/timer branch reads:

| Source node | OpenSpace expression | Operands | Native accessor | Result |
| --- | --- | --- | --- | --- |
| 20, 25 | category 3/op 59, `Function_Camera_GetShiftAlpha` | selectors 1, 0; `SelfActor` | `0x0053d300` dispatch calls `0x00440bf0` | raw camera parameter `+0x44` |
| 77, 82 | category 3/op 61, `Function_Camera_GetShiftTeta` | selectors 1, 0; `SelfActor` | `0x0053d300` dispatch calls `0x00440c30` | raw camera parameter `+0x4c` |

The local API table at `notes/research/hype-ai-api.tsv` records both as `LP`
functions handled by `0x0053d300`. Static disassembly of the verified local
`reference/hype_patch/MaiDFXvr_bleu.exe` shows the getter arms at
`0x0053d493` and `0x0053d4c5` call the accessors and store their float result.
The decompilation bodies in
`reference/decompilation/functions/00440bf0_FUN_00440bf0.c` and
`reference/decompilation/functions/00440c30_FUN_00440c30.c` directly read
`param_1 + 0x44` and `param_1 + 0x4c`, respectively. These reads do not clamp
or reinterpret the floats.

The camera model's initial `CameraParameters` already retains these source
bytes. `CameraParameters::shift_alpha()` reads `unknown[20..24]`; the new
`shift_teta()` reads `unknown[24..28]`. Godot export already sends the latter
to the mode-zero pitch upper limit. The support getter returns that unchanged
source value. Both getter expressions preserve the integer selector as an
integer and retain the receiver; selector values 0 and 1 are the exact values
observed in model 437. Other selectors remain explicit unsupported cases.

The follow-up adds typed category 3/op 61 decoding, Rete persistence, guarded
GDScript lowering, and the `Character`/`SourceCamera` getter. Rete schema moves
once from 50 to 51; the Godot support interface moves once from 61 to 62.

## Finite follow-up list

1. **No-button convergence:** after each pair of getter reads, model 437 calls
   `Procedure_Camera_ChangeAngle` at nodes 53/70 or 110/127 only if the first
   value is nonzero and differs from the second. Each pair reads the same
   function, receiver, and source field without an intervening write, so the
   branch condition is false when these getters are mapped. Op 153 is not a
   blocker on the observed no-button intro path.
2. **Camera-button path:** the alternate rule-5 branch can reach op 153 at
   nodes 141/146/152/157. These are control-input angle changes, not part of
   the observed no-button startup path. The handler at `0x00547620` is not
   represented by a source-grounded component transition here, so these calls
   remain fail-closed until their writes and operands are traced.
3. **Camera-selection branch:** rule 1 can reach `Procedure_Camera_ChangeCamera`
   (category 4/op 141, nodes 13/19) when `dsg_24 == 1` and its actor/list
   condition selects a branch. The preserved New Game execution passed this
   rule before stopping later at node 20, so it was false at that checkpoint.
   This separate switch-camera path remains unsupported if its guard becomes
   true later.
4. **Other state-gated camera code:** model 437 rule 3 contains op 153 and op
   143; rule 4 contains getters 70/58/60/54/55 and ops 153/166/143. In the
   preserved generated behavior 0, `dsg_30` is assigned only zero (lines 151
   and 1194), making rule 4's `dsg_30 == 1` guard false; rule 3's
   `dsg_30 == 0 && dsg_31 == 1` guard is also false on the observed startup
   path because its only `dsg_31 = 1` assignment is inside rule 4. These are
   not sequential blockers after nodes 20/25/77/82.
5. **Adjacent actor blocker:** Hype model 28 category 4/op 237
   (`Proc_SetVisualGMTDiffuseCoef`, `0x00543230`) stopped the preserved
   authored New Game run independently. It belongs to the renderer workstream
   and was not changed here.

The preserved generated control flow is
`output/worktrees/fork-first-integration-20261008/output/fork-first-newgame-camera-20261008/newgame-camera-20261008/project/generated/hype/levels/manoir/ai/models/fix/437.gd`.
The authored-run evidence is under its sibling
`.../newgame-camera-20261008/evidence/`; that run predates this getter patch.
There is no post-patch authored intro/aperture acceptance result in this audit.

## Focused checks

- `cargo test --locked --lib camera_`: 20 passed, 1 local-game-data probe
  ignored.
- `cargo test --locked --lib snapshot_validation_helper_is_in_interface_62_and_staged`:
  1 passed.
- `cargo test --locked --test camera_shift_getters`: 1 passed; both typed
  expressions round-trip under Rete schema 51.
- `cargo test --locked --test cameras exported_camera_uses_edited_horizontal_fov_and_source_view_axis -- --exact --nocapture`:
  1 passed with Godot 4.7.2. The isolated exported-resource harness checks
  selectors 0/1 and raw zero/nonzero values for `+0x44` and `+0x4c`, while
  unobserved selector 2 fails closed.
- `cargo fmt --check` and `git diff --check`: passed.
