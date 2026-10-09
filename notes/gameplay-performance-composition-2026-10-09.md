# Gameplay performance composition — 2026-10-09

Kip authorized composing and publishing two fixes for the user's reported
seconds-per-frame gameplay. The instruction prohibits tests, gameplay launches,
benchmarks and performance measurements. Necessary converter build, native
resource creation/import and Manor publication are delivery operations. No
remote push, save/settings change or unrelated work is authorized here.

## Source composition and static review

Clean main began at `b06d4c66ec54ea408d3dcf3ab093c5d2d101f01f`, support83.
Composition is isolated on `integration/gameplay-performance-20261009` in
`/tmp/astrolabe-gameplay-performance-20261009`.

| Contribution | Original commit | Composition commit |
| --- | --- | --- |
| Dormant character banks/slots | `321d0b17e19b80bbd21b29b4c265c1f42a5e827c` | `eaaa653` |
| Native indexed camera sector queries | `08520b77f42c38d152a2cbd9702d2b758007fd55` | `0156041` |

Both cherry-picks were conflict-free; worker branches and worktrees remain
unchanged. The exporter, maintained support version and README use interface84
once; Rete remains62. The existing support staging assertion is mechanically
migrated to84 and includes the new index script/UID. Existing hand-authored
camera fixtures now provide explicit sector/visual/mesh bindings. Their
assertions are retained and were not executed.

Static review found and repaired one native API mismatch: the camera index
called `generate_triangle_mesh()` through the Mesh base type. Godot4.7 binds
that method to ArrayMesh. Imported meshes are exported ArrayMeshes, so the
index now casts explicitly and marks unsupported authored mesh types incomplete.
It retains Godot's resource-owned BVH and geometry invalidation, rather than
extracting faces into a script cache. See the official
[Mesh implementation](https://github.com/godotengine/godot/blob/4.7/scene/resources/mesh.cpp)
and [TriangleMesh bindings](https://github.com/godotengine/godot/blob/4.7/core/math/triangle_mesh.cpp).

Sector bindings retain depth-first order and nearest-sector ownership.
NodePaths resolve under the current level, reject queued/departed nodes and
retain no global node cache. Boundary BVHs are owned by their boundary nodes
and invalidated on vertex assignment. Each query reads current global
transforms; affine conversion and the recovered ray parameter keep distance
ordering with scaled/sheared bases. Pose publication reuses only the exact
position result from that call. No cross-frame result cache is introduced.

The dormant-bank change retains saved payloads and native animation targets.
Hidden banks stay disabled/top-level; visible selected anchors restore local
placement before enabling inherited processing. Native visibility propagation
also covers channel/actor visibility, while transparency/render-layer
suppression retains its existing independent contact behavior. ZDE/ZDR remain
outside the visual-bank lifecycle, including intentionally invisible zones.
Normal/reset switches move saved payloads between banks and selected anchors;
seeking changes anchor visibility through the same signal. Exiting actors
disconnect shared configuration signals, and re-entry's deferred application
reads current state only while the actor is inside the tree.

Godot's [Node3D implementation](https://github.com/godotengine/godot/blob/4.7/scene/3d/node_3d.cpp)
skips inherited transform propagation for top-level children. The script keeps
the intended local transform across the native top-level toggle and forces
the restored contact transform before re-enabling it. Existing queued transform
notifications therefore cannot publish an old suspended pose. The
[collision-object implementation](https://github.com/godotengine/godot/blob/4.7/scene/3d/physics/collision_object_3d.cpp)
removes/re-adds physics-space participation through REMOVE and inherited
process enablement. Exported areas are non-monitoring and supported mutations
come from conversion, scene readiness, animation or AI, rather than area
callbacks. Authored monitoring callbacks retain Godot's normal requirement to
defer their own physics mutations; no blanket deferred visibility/physics
queue changes the supported same-tick query order.

Saved initial actor activation and final slot participation are reconciled
before PackedScene publication. Existing save restoration uses the same
activation method and retains the game's physics-readiness waits. These are
static lifecycle findings, not executed gameplay acceptance.

## Completed publication and import recovery

Main was fast-forwarded through the reviewed composition commit
`c98b96d94673fe4cd9429240e3a37562a5fd11d1`. The source checkout's adopted
support changed through Git, without a bundled support installer overwriting
it. From that clean source, one required
`cargo build --release --locked --offline --bin astrolabe` exited0 in98.71s.
The binary therefore embeds both fixes and interface84; no older converter
was reused and no second build occurred.

The only conversion was
`target/release/astrolabe port disc godot --level manoir --locale en --content
scene --godot /usr/bin/godot`. Native construction, staged imports and scoped
publication completed. Its destination import then exceeded the240s allowance
inherited from7,243 pre-build declarations; the complete destination owns9,181
files. The first port exited1 after649.47s. This remains a recorded failure,
even though its generated files had already been published.

Only destination import was resumed, using the documented300s allowance for
the complete destination closure. Logs showed restoration of the remembered
Manor editor scene. That owned retry was interrupted with exit130 and its
child was cleaned up. Godot's official
[editor source](https://github.com/godotengine/godot/blob/4.7/editor/editor_node.cpp)
confirms that editor startup restores saved scene tabs independently of resource
import. `interface/scene_tabs/restore_scenes_on_load` was then disabled only in
the publication-owned XDG_CONFIG_HOME. Project editor layout, metadata and
script-editor preferences were snapshotted and restored byte-for-byte.

Ordinary `/usr/bin/godot --path godot --headless --import` then exited0 in2.83s,
with no script/resource errors, timeout or remaining child. No recovery mode,
script suppression, gameplay, second conversion, additional build or separate
validation suite was used. The original timeout and interrupted retry remain
in the receipt. Delivery from the first manifest snapshot to the completed
receipt took1,067.61s, including the build, publication and import recovery.
Future CLI imports still use their existing allowance and
editor-session behavior; this recovery did not broaden production converter
changes. Editor reopening of the dense scene is not accepted by this import.

The final schema2/support84 manifest uses pinned Godot
`4.7.2.stable.arch_linux.ed1daf0bf`, SHA256
`2980c8f9c174c6137946a901d817b5bb8b197ca0abfad0546651fb10487de5cc`.
Its previous support83 hash was
`abd12f59e5b3e0983d38a2531f18ad6e8ef2c5171e4efba5a6ce4980d684cb26`.
Rete remains62; this direct source port did not serialize a new Rete snapshot.

| Generated scope | Files |
| --- | ---: |
| Manor | 8,855 |
| Inventory | 213 |
| Menu | 88 |
| Pause menu | 27 |
| Unique union | 9,181 |

There are59 changed generated files:58 native Perso/template scenes and the
saved Manor root. No generated file was added/removed; all344 existing generated
UID files and all AI/animation files retain their identities/bytes. The saved
`SourceSectorIndex` has29 ordered sector paths,37 physical-visual paths and43
mesh paths, persisted as native NodePath bindings.

All retained UI memberships and actual file hashes are unchanged. The two
Manor/UI shared textures remain
`textures/FixTex/Inventor/Objects/Usable/plastiks_nz.gf.mips.tres` and
`textures/FixTex/MOTORTXT/lettrageb.gf.mips.tres` under `generated/hype/`.
Native saves, game settings, imported original slot and `project.godot` hashes
are unchanged. Tracked source remained frozen throughout delivery, and no
uncommitted authored work remained on main before this receipt-only update.
The maintained `godot/game/` authored files were not changed by these fixes.

Local evidence is under `output/gameplay-performance-publication-20261009/`:
`result.json`, `receipt-summary.json`, before/after manifests, original build/port
logs, both resumed-import logs, editor-session snapshots and the saved index
fragment. These outputs and game-derived evidence remain ignored/local. Direct
progress messaging to the supplied parent thread returned `thread not found`;
`progress.json` carried the live phase and this final handoff carries the outcome.

The actual game is prepared at `godot/`, with its existing
`res://game/bootstrap.tscn` entry. No owned process remains and no delivery
blocker remains. Tests, gameplay launches, benchmarks and performance checks
were not run. Actual frame-time improvement and gameplay lifecycle acceptance
remain unmeasured by instruction; previous opening/fidelity limits remain.
