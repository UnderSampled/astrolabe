# Intro camera source-first pass — 2026-10-07

This pass starts from committed `main` and keeps camera work isolated in the
project-backed branch `codex/intro-camera-source-pass-storage-20261007-1`.

## Established bindings and fields

- Camera activation in `00440820` resolves the active `MainActor` and writes it
  through `00440fd0` into camera component `+0x10`. Procedure 160
  (`00548070`) writes its resolved actor to that field; procedure 163
  (`00548250`) restores the current `MainActor`. The scene exporter now gives
  each generated `SourceCamera` a `NodePath` to the exact `scene.main_actor`
  asset. Compiled procedure 163 retains its own typed AST opcode and calls the
  native reset API with the Brain's bound `main_actor`.
- `0043daf0` reads the active byte at parameter `+0x90` and viewport context
  at `+0x9c`. The model's source-order unknown tail puts these at indices 88
  and 96. Context 1 selects Godot's primary viewport; other contexts are not
  mapped. Parameter set 0 supplies this registration and the initial motion
  mode; set 1 still supplies the supported FOV/mode-3 framing.
- `00440f80` reads the signed target-child selector at parameter `+0xa0`, which
  is byte 100 in the preserved parameter tail. `-1` means the selected Perso
  itself. A nonnegative selector is retained and rejected explicitly until a
  source-child-to-Godot-node map exists; the exporter does not substitute an
  arbitrary Godot child. The manor camera record 0 has raw byte `0xff` at its
  `+0xa0` selector (the camera-record probe's allocation offset 160), so its
  startup target is the selected Perso itself.
- Camera activation (`00440820`) initializes component `+0x10` from the
  current MainActor; procedure 160 replaces it with its resolved Perso and
  procedure 163 resets it to the current MainActor. The exporter binds the
  manor camera's initial target to the exact `scene.main_actor` node, and the
  AI compiler retains procedure 163 as a typed reset call. Later procedure
  160 changes continue using the existing target API.
- `0051c8d0`, `0051d330`, and `0051d140` establish raw mode-0 target geometry
  from the selected actor's source basis and actor, target, and look offsets.
  The native camera exposes this unsmoothed geometry; it does not claim that
  it is the controller's current candidate eye.

## Candidate math and sector-history follow-up

`00443bf0` prepares eight `Data3D` points and weights. If their total weight is
zero, it places every point at the selected actor translation and sets each
weight to `0.03`. `00444620` accumulates the points in source order until
weight `0.4` (`DAT_00745a84`, initialized by `0043f180`) and returns that
weighted center. `0051c8d0` compares the center with raw actor translation;
its squared threshold is `_DAT_005fcc2c`, the static `9.0f` value at VA
`005fcc2c` in `reference/hype_patch/MaiDFXvr_bleu.exe` (the decompiled build,
SHA-256 recorded in `reference/decompilation/README.md`). Its `.data` VA maps
to file offset `0x1fae2c`; bytes `00 00 10 41` decode to `9.0f`. The checked
decompilation has a read site and no write site for this symbol. When crossed,
its shared fallback counter uses the actor translation for eight controller
updates (`DAT_00721af8`), incrementing `DAT_00721afc`.

`0051db70` derives the desired vector from controller `+0x18` minus pivot
`+0x3c`. `0051d410` supplies current orientation through `005282c0`. The DNM
orientation selectors initialize in `0044cf20` to `[0,-1,0]`, `[-1,0,0]`, and
`[0,0,-1]`; the decompilation has no other writer to those fields.
`005282c0` therefore selects the target's source Y basis, negates it, then
`0051d410` negates the result again, yielding source +Y. The scene exporter
maps source +Y to Godot local -Z, so the native adapter uses the selected
target's global basis times `Vector3.FORWARD`.

`0051d6e0` calls `0057c62f` for signed heading/elevation and clamps them with
parameter `+0x40/+0x44` and `+0x48/+0x4c`; `0051d7b0` rotates yaw about source
Z, then pitch about the resulting local-right axis. The scene exporter maps
source Z-up to Godot Y-up, so candidate height and both rotations now use
Godot Y. Distance is the current owner-to-pivot length; `0051db70` caps it to
the maximum only when it is already at least the minimum and exceeds the
maximum, then applies a `0.01` floor. `004406d0` initializes Position `+0xa4`
and `+0xa8` to finite sentinels `0xff7ffff8` / `0x7f7ffff8`; `00528590` clamps
candidate source Z to those values. The manor startup path retains those
defaults and maps them to Godot Y.

The yaw/pitch center angles are at source `+0x40/+0x48`; their upper bounds
are at `+0x44/+0x4c`, represented by parameter unknown-tail indices 20–24 and
24–28. The angular flag word is separately read by `00440ea0` from `+0x94`
(unknown-tail indices 92–94), not from the motion mode at `+0x98`. The manor
camera-record probe reports flags `0` for parameter set 0 and `8` for set 1.
These values and bounds are now read/exported explicitly from their source
offsets. Godot support has a weighted-pivot helper, shared eight-update smoothing state, and
a candidate calculator that requires the controller vectors and vertical
bounds as inputs. The projection script asserts the `0.4` pivot cutoff, `9.0`
squared movement threshold, eight-update hold and release, and a mode-0
candidate.

`00448280` carries the previous camera position and sector into `004e03f0`.
That routine repeatedly takes the first one-sided crossing of the current
sector's boundary, advances only into a neighbor with activation bit `4`, and
continues from the intersection. The exporter now preserves a face normal per
boundary triangle; native support performs the front-to-back plane test,
traverses the active neighbors, and updates sector context before camera
collision rays. Successful `0051c7b0`-style publication also resets the stored
history point to the selected owner translation so the next update does not
replay a crossing from the old pose.

`0051f670` uses three segments: controller `+0x3c` to `+0x30`, controller
`+0x18` to `+0x30`, and controller `+0x18` to the selected actor's world
translation plus source-Z 1.8 (Godot `Vector3.UP * 1.8`). The old Godot Z
offset was incorrect and is fixed. The existing collision helper still
requires all three computed controller points. `00463df0` updates sector
context, searches active-sector and collision-neighbor lists, ignores
material identifier bit `0x8000`, and excludes the camera owner/selected
target in the actor case.

`004e0020` resolves a point's sector by tracing one source unit down
(`[0,0,-1]`) through every nonvirtual sector with activation bit `4`, first
against that sector's border GEO and then against child physical visuals of
source kinds `1`, `0xd`, `0x15`, and `0xb`. It selects the closest hit;
`004b7ab0` mode 0 performs the face-geometry semi-axis query. The exporter now
preserves each node's raw `source_node_kind`, and native support runs the exact
downward border-GEO query over the saved SectorBoundary triangles. If active
child physical-visual nodes are present, the result is marked incomplete
because their visual-set triangles/LOD are not exported for this query; the
pose publisher refuses that partial result.

### Initial Data3D blocker (superseded below)

The first source pass identified that camera mode 0 needs eight runtime DNM
position/weight samples which are not present in the serialized
`PersoData3dPrefixWire`. At that checkpoint, the native actor did not yet
produce those inputs, so mode 0 was guarded. The supported callback-derived
producer and its remaining collision boundary are described in the next
section.

### Data3D producer and remaining mode-0 boundary

The live Data3D input is now produced by the maintained Godot character
component. `00443f00` dispatches by the active identity-card type. The typed
mechanics resource carries that source card kind (`MechanicsData::Character`
and `CharacterState.mechanics`), and the generated native animation assigns it
to `current_mechanics` before the character's per-frame sample. The decompiled
callback table `DAT_00757320` maps types 0 and 8 to Walk handler `00585d51`;
other types remain unsupported by this producer and fail closed.

For the supported Walk callback, the Godot actor records its world translation
after native `move_and_slide()` (or after the authored pose advance for
stationary actors). This is the corresponding native post-contact pose: the
Walk handler's integrated xyz output at physical object `+0x310` is also written
to the actor's global SuperObject translation. `00443d50` shifts the eight
`+0x310` xyz samples newest-first, and `00443db0` shifts weights while putting
the current frame delta in slot 0. The producer stores those eight positions
and delta weights on the actor and resets history when its active callback
changes. `source_mode_zero_live_pivot()` consumes them, uses the source
`00443bf0` actor-translation/`0.03` fallback when all weights are zero, applies
the `00444620` accumulated-weight cutoff of `0.4`, the shared eight-update
`9.0` squared-distance hold, then applies the target-rotated actor offset.
Candidate and focus offsets now use that same smoothed pivot. Focused Godot
assertions cover newest-first order, exact delta weights, callback selection,
callback transition reset, pivot weighting, actor offset and focus.

This removes the missing Data3D producer as a blocker. `source_update_position`
now computes the source candidate from the target pivot, camera-owner position,
orientation selector and vertical limits; it also classifies distance through
`0052ce20` using `+0x30/+0x34`, the verified `0.001` epsilon, and the static
`DAT_0060b23c=3` / `+0.3` padding. The clear-ray `0051f670` branch runs through
the shared gate state. It returns a completed no-translation update because
the clear rays reset the collision streak and that branch cannot set
`DAT_007219d1`; source `0051c7b0` is therefore not called for this result.

When either candidate ray is blocked, `0051f670` instead depends on the
fallback candidate search in `0051fb50` and its `0052d1b0`/`0052caa0`
classifications. That sweep's static angle/range arrays and source result
classification are not yet represented by the Godot provider. The camera-owner
flag used by a successful teleport gate is also not currently exported. The
blocked path fails explicitly before it can claim a pose update. No observed
runtime branch value is substituted for those inputs. The controller's
process-wide hysteresis state remains represented by the session-shared
`source_controller_state.gd` resource.

The other startup inputs now have source-backed adapters: the target's source
+Y orientation selector, camera-owner Position sentinel bounds, Godot Y-up
conversion, and the sector border-GEO lookup. The three collision rays reuse
exported source collision identifiers, reachable sectors, and owner/target
exclusions.

## Checks

The initial source-pass checks in commit `ab6e5a5` passed. In this follow-up,
`cargo fmt --all -- --check` and `git diff --check` passed. The focused
`cargo test --offline --locked --test cameras` passed (1 passed, 4 ignored);
its Godot 4.7.2 run parsed and executed the projection script, including the
`9.0f` threshold, eight-update hold, candidate math, and one-sided sector
crossing, source selector, and same/cross-sector pose-publication assertions.
The first test attempt hit `/tmp` quota while creating its temporary project;
the successful rerun placed temporary files in the worktree's ignored Cargo
target directory. The camera model filter (`cargo test --offline --locked
--lib camera_`) passed 16 tests with 1 local-game-data test ignored. No
original-game run, debugger, new harness, or installation was used.

After adding the actor-owned Data3D producer and collision-free mode-0 path,
`cargo test --offline --locked --test cameras` passed again (1 passed, 4
ignored) on Godot 4.7.2. The new assertions exercise callback type 0, the
eight-entry newest-first history and frame weights, unsupported type 4,
history reset on callback transition, live weighted pivot/focus, the
`0052ce20` distance classes, and a collision-free source update that correctly
does not invoke `0051c7b0`. `cargo fmt --all -- --check` and `git diff --check`
passed after these changes.

During integration into current `main`, the same focused camera test passed
again (1 passed, 4 ignored). Its first invocation failed before assertions when
`tempfile` used quota-constrained `/tmp`; the identical command passed with
`TMPDIR` set to an ignored checkout-owned directory. The temporary directory
was removed afterward. `cargo fmt --all -- --check` and `git diff --check`
passed on the integrated diff as well.
