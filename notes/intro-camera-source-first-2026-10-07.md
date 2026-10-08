# Intro camera source-first pass — 2026-10-07

Correction, 2026-10-08: the callback-table mapping below used four-byte word
indices as eight-byte rows. Walk position publication belongs to mechanics
kinds 0/4; kind 8 dispatches elsewhere. The earlier rejection of kind 4 is
superseded by the [source/PE-backed binding repair](intro-armor-camera-bindings-2026-10-08.md).
That repair also distinguishes an explicitly absent state card from an
unmapped callback and preserves its existing position history.

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
decompilation has a read site and no write site for this symbol. `0051f450`
likewise reads the `.35` displacement from `_DAT_005fcc78`; its `.data` file
offset is `0x1fae78`, and bytes `33 33 b3 3e` decode to `0.349999994f`.
When crossed,
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
preserves each node's raw `source_node_kind` and each type-13/21 VisualSet's
LOD count. Native support applies `Sector.unknown[0]` as the child traversal
gate, tests saved border triangles, and tests type-13/21 visual faces when the
source VisualSet has exactly one LOD. Type-1/11 GEO and multi-LOD physical
visuals are marked incomplete; pose publication refuses those partial results.

### Earlier Data3D blocker (superseded below)

The following records the initial checkpoint before the actor-owned producer
was implemented later in this pass.

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
computes the mode-0 controller candidate from the live target pivot, camera
owner, source +Y orientation and vertical limits. The owner Position bit-2 path
now consumes the exact `00440fe0` `+0x50` vector and forms `+0x24` from current
`+0x18`, as `0051db70` does. The ordinary path retains `0052ce20` range
classification, including the `0.001` epsilon and `+0.3` maximum padding.

The `005200a0` selector dispatch now runs both source paths. Selector 0 ports
`0051f670`: the first `+0x3c -> +0x30` ray retains collision point and normal;
`0051f450` applies the source `.35` normal offset and the controller reruns the
ray. It then checks `+0x18 -> +0x30`, updates the `DAT_00721988` and
`DAT_0072198c` counters and `DAT_007218fd`, and conditionally checks the
selected actor's translation plus source-Z 1.8. The source's candidate reset,
input-bit writes, and target-head branches update the controller candidate
without inventing an observed pose.

Selector 1 now ports `0052bb50`'s per-owner static-obstruction counters and
60-sample threshold, then `0051f290`'s filtered camera ray and
`0051f040`/`0051f250` safe-point recovery. Static geometry queries reuse the
exported source sector, activation, material and collision metadata through
the existing Godot collision adapter. `0051fb50` is still dispatched only
when the selected routine returns zero and `DAT_007218fc` is set; selector 1's
recovery path is now the only production path that sets that fallback request.
The initial parameter word at `+0x92` is now exported from source bytes 90–91;
`0051f520` uses the existing `Button_SSLeft`, `Button_SSRight`, and
`Button_Down` actions without assigning new physical keys.

The shared state Resource separates `DAT_007219d0` from the last-good
`DAT_007219ac` position, tracks the per-update `DAT_007218fc`, `DAT_007218fd`,
and `DAT_007219d1` flags, and follows the distinct success/failure distance
hysteresis rules. When the source gate sets d1, the native path reaches
`0051c7b0` and uses its active-sector decision to publish the selected pose.
The sector resolver follows `004e0020`'s border and child-GEO query for the
currently exported cases: borders and type-13/21 visuals with a single source
LOD. Active type-1/11 children and physical visuals with multiple LODs still
make the result incomplete rather than substituting bounds or an arbitrary LOD.

One behavior gap remains explicit. When a failed candidate sets
`DAT_007218fd` without reaching d1,
`005200a0` calls `0051ec70`, which updates controller offset interpolation
through `0052c7f0`, `0051e350`, and `0051e600`. That native camera-motion API is
not implemented, so the camera update returns an error on that path instead of
claiming success. `0051c7b0` translation writes and the supported candidate
branches are implemented independently of these cases.

The other startup inputs now have source-backed adapters: the target's source
+Y orientation selector, camera-owner Position sentinel bounds, Godot Y-up
conversion, and supported `004e0020` sector GEO inputs. The three collision
rays reuse exported source collision identifiers, reachable sectors, and
owner/target exclusions.

Letterboxing already has a maintained `Screen` component: compiled source UI
calls route to `source_toggle_widescreen()`, which shows the bars and suppresses
the HUD. `tests/check_ui.gd` covers that toggle. The remaining letterbox question
for the complete New Game demo is whether the converted intro invokes it at the
source timing; this camera change does not alter that UI path.

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
--lib camera_`) passed 17 tests with 1 local-game-data test ignored. No
original-game run, debugger, new harness, or installation was used.

After adding the actor-owned Data3D producer and collision-free mode-0 path,
`cargo test --offline --locked --test cameras` passed again (1 passed, 4
ignored) on Godot 4.7.2. The new assertions exercise callback type 0, the
eight-entry newest-first history and frame weights, unsupported type 4,
history reset on callback transition, live weighted pivot/focus, the
`0052ce20` distance classes, and a collision-free source update that correctly
does not invoke `0051c7b0`. `cargo fmt --all -- --check` and `git diff --check`
passed after these changes.

The current focused camera integration test passed on Godot 4.7.2. Its
projection script exercises the exported target binding, weighted Data3D
pivot, candidate math, clear-ray d0 state, structured collision hit point and
normal offset, the blocked target-head branch, selector 1's first static
obstruction sample, source material filtering, fallback classification, and
same/cross-sector conditional pose publication. `cargo fmt --all -- --check`
and `git diff --check` passed. The local original game was not launched and no
debugger, new harness or installation was used. The full New Game intro camera
and letterbox timing remain unaccepted.

The final focused reruns after correcting both `0051f670` calls to
`0051f450`'s source `.35` normal offset passed: camera projection integration
(`1 passed, 4 local-game cases ignored`) and camera model unit tests (`17
passed, 1 local-game-data case ignored`). Formatting and whitespace checks
also passed.

## 2026-10-08 source-first continuation

Camera target binding is concrete. Camera activation at `00440820` initializes
component `+0x10` from `MainActor` through `00440fd0`; procedure 160 (`00548070`)
writes its resolved Perso, and procedure 163 (`00548250`) restores `MainActor`.
Manor camera record 0 preserves target-child selector `-1` at parameter `+0xa0`,
so generated `SourceCamera.targeted_actor` binds the exact exported
`scene.main_actor` reference. No arbitrary scene child is selected.

The `+0x254` camera collision guard is now source-resolved for startup. The
camera callback context is created by `0044cde0`: it allocates a `0x400`-byte
payload through `00409810`, whose `00409840` implementation zeroes every
requested dword and the tail bytes, then stores the payload at context `+0x10`
through `0044d2d0`. `00527e00` obtains that payload with `0044d2c0`, stores its
base at camera-structure `+4`, and uses `base + 0xd8` as `*param1`. Therefore
`0051f670`'s `*param1 + 0x254` read is payload `+0x32c`, inside the zero-filled
allocation: the startup value is false. The native `SourceCamera` initializes
its camera-local guard to false and consumes that value in the guarded
target-head branch.

The dynamic writer remains conditional. `005796ea` clears payload `+0x32c` at
callback entry and sets it when its `00463df0` obstruction query hits, but
`00578020` invokes that writer only while the type-3 callback's `+0xe0` flag is
active. The identified zero-activation call at `00443eeb` reaches
`0057910c`, which explicitly clears `+0xe0`; that path skips the writer. The
native camera has a tested callback-result update method, but no manor runtime
producer currently activates and feeds this callback. Do not infer dynamic
changes on paths where its activation has not been established.

Selector 1 remains a player-input branch, not an authored manor intro call:
`005200a0` changes `DAT_00721b0c` from the `BT_CameraInOldMode` and
`BT_CameraInNewMode` actions. It is initialized to zero in BSS and the reviewed
manor intro AI has no call that synthesizes either action. The existing camera
port computes selector state and collision inputs, but reports failure when
`0051ec70`'s controller-offset interpolation is reached; this does not block
the source-authored mode-0 update path.

The completed source-sector query now matches the source filters for retained
content. It searches nonvirtual, activation-bit-4 sectors, checks border GEO,
and traverses child GEO only when `Sector.unknown[0] == 0`. For type-13/21
children it traces the imported face triangles only when the VisualSet has one
LOD, recorded from `VisualSetWire.lods` through Rete and emitted as
`source_visual_lod_count`. Missing type-1/11 GEO and multi-LOD visuals mark the
whole lookup incomplete, which prevents `0051c7b0` from publishing a pose from
a partial result. Focused Godot assertions cover the source child gate, nearer
physical-visual face selection over a border, and multi-LOD refusal.
