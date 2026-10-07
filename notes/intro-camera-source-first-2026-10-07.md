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

### Exact remaining Data3D blocker

`0044cf20` initializes eight runtime Data3D positions, auxiliary vectors, and
weights to zero. `004440e0` walks the target's active DNM identity-card list;
`00443f00` dispatches the callback indexed by the resolved card's actual
`DNM_fn_eIdentityCardGetType`, then copies live physical-object arrays at
`+0x2d8` and `+0x310` into the DNM Data3D arrays and assigns card mass as their
weights. The callback comes from the runtime identity-card type, which is not
one of the three `StandardIdentityWire.object_types`; type 0/8 uses `00585d51`,
and the callback table has distinct handlers for other card types. `00443bf0`
uses actor translation only when all eight runtime weights sum to zero; there
is no evidence that the manor target reaches that branch when the camera
updates.

The serialized `PersoData3dPrefixWire` is 24 bytes and contains only state,
configuration and family references. The current model retains those
references, the three StandardGame object-type IDs, and authored model and
animation assets; it does not contain the eight live DNM positions/weights,
resolved identity-card type, or current physics callback outputs. Constructing
the points from mesh bounds or choosing a callback from `object_types` would
be a guess. Porting DNM/physics internals wholesale into Godot would also
violate the repository's engine-boundary guidance. No original runtime or
debugger capture was allowed for this pass. Mode 0 therefore still reports
unsupported and does not claim collision success, evaluate the teleport gate,
or publish a pose until a source-equivalent Godot actor-state producer supplies
those eight points and weights.

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
A separate Godot 4.7.2 headless scan of the maintained `godot/` project also
completed with exit code 0 and no socket or script parse errors. It regenerated
five `.uid` files, which were removed afterward; this checked editor parsing
and class registration, not the generated camera scene or gameplay.
The first test attempt hit `/tmp` quota while creating its temporary project;
the successful rerun placed temporary files in the worktree's ignored Cargo
target directory. The camera model filter (`cargo test --offline --locked
--lib camera_`) passed 16 tests with 1 local-game-data test ignored. No
original-game run, debugger, new harness, or installation was used.
