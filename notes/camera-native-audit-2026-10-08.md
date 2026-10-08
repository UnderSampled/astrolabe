# Native camera audit — 2026-10-08

This focused batch starts from `209f2e1` and uses an isolated source checkout.
It fixes native camera state, authored modes 0/3/6/-1, and the owning motion
binding contract. It does **not** establish complete manor intro acceptance.
No original executable, original-engine tests, or comparison worktrees were
used. The lead owns root Godot publication, schema/support versions, and final
authored acceptance.

## Source authority and exact PE evidence

The clean reference was read from `/home/stephen/Projects/hype-decomp/src/camera/`
and its `evidence/camera_update.asm`, `camera_target_group.asm`,
`camera_target_reset.asm`, and `docs/findings.md`.
The reference checkout was `7ebe1a3` when this note was prepared. The relevant
camera reconstruction was already committed; this worker did not edit it.
Original exported functions were read from Astrolabe's
`reference/decompilation/functions/`. Disassembly and PE reads were static.

Both PE profiles have identical bytes for each span below. Whole-file SHA-256:

- Source profile `MaiDFXvr_bleu.exe`:
  `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
- Installed profile `MaiDFXvr_bleu.exe`:
  `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.

Inclusive function span | File offset | Bytes | SHA-256
--- | --- | ---: | ---
00548070–00548246 (Proc160) | 147470 | 471 | `08e25d8e04b3e54dff50eaff54ebaa9b942f806a60067d6bbaf20348914feb33`
00548250–005482a3 (Proc163) | 147650 | 84 | `63142872b1b953ed3fbf5f5a4d3a53d5302dff5eabfebb368e7c737735e64d55`
00545d80–00546073 (UpdatePosition135) | 145180 | 756 | `76005cef689b9ab36ebe7441fa3c046fdc3da547ead7f4825924be3c334e2574`
00527e00–00527ea4 (frame initialization) | 127200 | 165 | `06de6fd8907c536ba7f13c7bc1d6cf8f9bfe693d240cb549c42e70dbaf843cc6`
0051db70–0051de2d (motion calculation) | 11cf70 | 702 | `8be2df9fa78101521e08ccd6b8f1724a0f60e8db8a138de8cdecd47e5fd18f60`
0057b17f–0057bd62 (native frame consumer) | 17a57f | 3044 | `a8fdcedb6f2458f29019fe021a6610fb540ef904ce779c6c0db2ec90616df0ec`

The PE section table, rather than an assumed VA/file-offset subtraction, maps
these reads. Ignored `output/camera-native-evidence/pe-spans.json` retains both
profile paths and hashes. No binary bytes or game-derived assets are committed.

## Corrected component semantics

`camera_api.c` proves Proc160 restores the **previous** target's alpha before
new operand evaluation. The compiler already emits that preparation separately;
a failed new binding leaves target/progress unchanged after restoring alpha.
Successful assignment now resets the following component-owned progress:

Source camera field | Native property | Consumer evidence
--- | --- | ---
B8, 16-bit | `fallback_motion_frames` | 0052c2b0 recovery progress
BC, 16-bit | `collision_recovery_frames` | 0052c440 stationary/collision recovery
C0, 16-bit | `look_direction_hold_frames` | 0051cb10 look hold
C4, 16-bit | `static_probe_frames` | 0052bb50 probe cadence
74, float | `previous_look_radius` | 0051cfa0 look-radius limiter
78, float | `slope_pitch_adjustment` | 0051e960 pitch adjustment

CA's obstruction streak and session-wide controller pivot/teleport history
survive Proc160. Proc163 only assigns MainActor; it restores neither opacity nor
progress. The pending recovery/look consumers remain incomplete; these resets
do not claim those algorithms are implemented. Modes 3/6 reset the four mapped
progress values; 0051e960 also zeroes pitch adjustment outside modes 0/7/101.

0051db70 clamps a distance below the minimum **up** before clamping above the
maximum down. This fixes the prior omitted minimum. Its CE bit2 and vector50
are camera-owned supplied eye displacement, not SuperObject Position metadata.
The native API is now `source_set_eye_displacement`, `source_has_eye_displacement`
and `source_clear_eye_displacement`. No parent metadata stores that request.
0052bb50 increments word counters before signed comparisons. It returns one
for the **first** obstructed static probe; its >60 branch also returns one and
is not a grace period. Native cadence/streak properties narrow accordingly.

UpdatePosition135 uses its caller directly; there is no Ultra operand in that
wrapper. `camera_update.c` recovers the actual initializer sequence: get target
from camera, then call 005282c0(target, temporary vector, frame). Getter/frame
argument attribution in the raw export is wrong. 005282c0 returns signed AL
axes ±1..3; the target channel byte is signed and promoted to a 16-bit key.
The existing native -1 target-child path remains the supported path.

After frame application 00528030, **caller Perso+8 being null jumps directly to
finish**, skipping ordinary backup/flag cleanup. The lead's exact reviewed
patch adds `Camera.owner_motion_binding_present`, derives it from the resolved
Perso+8 edge in the source reader, retains it through Rete, and exports it to the
native camera. No mechanics kind or camera presence substitutes for this edge.
The actual manor camera's binding is present. Native ordinary cleanup clears
captured initial parameter bit8 and pending displacement; the null-binding path
keeps these after applying the pose. Other source backup/CE/D0/D2 flags are not
fully projected and are an explicit remaining gap.

Mode dispatch evidence is `camera_calculation.c` (005200a0), Proc157
00547be0, and frame consumer 0057b17f. Modes 0/3/6 can transition without extra
Proc157 reset branches; equal modes are no-ops, and -1 restores initial mode.
Ordinary mode3 has action flags0xc14 and uses the native actor eye with a
rotated target/look offset. Mode6 has flags0x514. At 0057b230 the consumer tests
0x100; the set branch at 0057b238 calls translation setter004bb350 and jumps
0057b253→0057bb19, skipping rotation construction. Thus **mode6 preserves the
previous basis**, while authored traveling-camera position updates move its
eye. Bit0x400 skips 0057ab7d smoothing in both modes.

Float-store rounding, x87 unordered comparisons, and source length intermediates
are documented in clean C. This native patch does not reproduce an x87 VM.
Finite authored transforms, semantic offsets, distance bounds, signed counter
widths, and the mode/cleanup branches are the tested contract. Exceptional
NaN/extended-precision parity and moving weighted Data3D target pivots in mode3
remain unverified.

## Connected authored camera slice

Paths below are canonical asset references, with actors under
`level:/persos/world/0/` and waypoints under `level:/waypoints/17-1/`.
The local check walks all normal/reflex rules and schedules of World154,
Camera437, travel365, Gogoud210 and statue294, retains 71 camera/eye calls,
reopens every model and camera Perso through Rete, and resolves shot waypoints.
It asserts source prerequisites; it does not execute the flow or bypass guards.

Authored source | Camera target / eye | Dependency or request
--- | --- | ---
World154 normal2 schedule | actor33/wp86 → actor18/wp84 → actor19/wp82 | mode3; waits3600/4500; then normal3
World154 normal3 rule0, dsg0=4/1/2/3/5/6 | 20/67, 29/69, 19/81, 31/79, 21/80, 26/47 | mode3; dsg0=1 branch waits1200
World154 normal3 rule1, Gogoud dsg4=1 | 21/80 → 43/78 → 21/80 → 25/81 → 21/80 → 31/77 → actor29 | waits1100/1700/1400/4500/3000/2000/2100; final eye is actor45's Ultra Func0 Position
World154 normal3 rule1 tail | actor29 / copied actor45 eye | wait700; actor45 switches to travel365 normal2; World returns normal0
Travel365 normal2 rule0 + schedule | camera eye copies SelfPosition each tick | speed(0,-1.4,0), GoOverWay dsg slot0; waypoint135 sphere → **mode6**, statue actor28 dsg0=1, travel normal1
Statue294 normal5 rule0 | actor46/wp48 | waits dsg0=1; MainActor normal70; mode3; statue normal9
Statue294 normal9 rule0 | MainActor / actor45 eye | MainActor dsg21=1; actor45 at wp134; Ultra Func0 eye copy; wait1200; travel normal4→normal3
Travel365 normal3 rule0 + schedule | camera eye copies SelfPosition each tick | speed(0,+0.8,0), GoOverWay slot1; waypoint129 sphere → Gogoud dsg1=1
Gogoud210 normal9 | no independent eye write | dsg3 and dsg1 gates; clear both, restore Gogoud normal2
Gogoud210 normal12 schedule | actor47/wp52 → MainActor | mode3, wide-screen toggle, route/metas19/32/33; Proc160 MainActor then **mode -1**
World154 normal5 schedule | camera wp55; MainActor wp58 plus wp57/53/54 route array | alternate setup/dialogue/control path; mode3 then -1
Camera437 normal0 rule0 | bound owner camera | dsg30=0: return-zero + UpdatePosition135; other guarded behavior selectors/angles are not unconditional intro updates

The finite graph therefore requires **World3 → travel365 → mode6 → statue and
return travel → Gogoud12 → -1**. Source behavior references preserve the
MainActor authored action/dialogue steps between those nodes. This camera
batch does not replace them with an artificial acceptance path.

Upstream failures must remain distinct: music/setup can stop before these shots;
Ultra Func0 cross-actor Position, GoOverWay30, Gogoud route/meta19/32/33,
procedure402, self-deactivation and several MainActor setup/dialogue branches
were unresolved at this source base. The lead and original-C worker own their
connected tracing and implementation. Mode0 failed-candidate interpolation
0051ec70/0052c7f0, additional hierarchy selectors, camera override/modes beyond
0/3/6, partial frame-write exits, full backups and source weighted target
behavior also remain incomplete. No full intro or player-control handoff is
claimed from parser/unit/component success.

## Focused validation

Evidence is ignored under `output/camera-native-evidence/` in the isolated
checkout. Builds, Godot projects, config/cache/temp files and reports are owned
there; source disc and clean-C references are read-only absolute paths.

- Native parser/component check: Proc160 ordering/resets, Proc163 non-resets,
  min/max bounds, signed cadence, mode3 target/eye binding, mode6 preserved basis,
  null/present motion cleanup and equal/-1 transitions.
- Actual saved-camera test: edited Rete FOV, coordinate/view axis, rotated offsets,
  exported motion-binding bool, collision source predicates and prior component
  checks. It uses maintained support and the saved scene, with unrelated actor
  processes disabled as declared by the existing fixture.
- Local connected import/Rete check: 71 calls, exact travel mode6 and ordered
  Gogoud MainActor→-1 prerequisites; 33.00 seconds after the lead bool patch.
- Synthetic `cargo test --lib camera_`: 20 pass; one explicitly local source
  camera probe excluded from this synthetic selection, then run separately.
- Mandatory supervisor, selection, nextest and recording safeguards pass.
  Nextest was given the existing pinned tool's absolute path; nothing installed.
- Release build and formatting/diff checks are recorded in their logs.
- Workspace Clippy fails six existing warnings: nested format in
  `src/godot/character.rs:491`, argument count in `src/godot/sound.rs:273`, two
  collapsible matches in `src/model/ai.rs:1480/1488`, camera items after tests,
  and clone-to-slice in `src/openspace/level/brain_save_tests.rs:303`.
  No unrelated source cleanup is included.

The saved-scene editor requires a local listener that fails in the ordinary
sandbox. Its narrowly scoped, user-authorized reviewed execution succeeded;
direct component and source checks ran in the ordinary sandbox. This is focused
validation, not the full declared release/gameplay matrix. New tests need the
lead-owned manifest entries and the bool persistence field needs the coordinated
Rete/support version bump before root publication.
