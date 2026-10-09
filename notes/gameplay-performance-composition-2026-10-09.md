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

## Publication

Publication is pending the single required current-source converter build and
Manor-only `port disc godot --level manoir --locale en --content scene --godot
/usr/bin/godot`. The source checkout owns adopted support through Git; no
bundled support installer may overwrite it. Menu, inventory, pause-menu and
shared textures must retain their existing closures and hashes.

The final receipt will be recorded under
`output/gameplay-performance-publication-20261009/`. Native syntax/import
acceptance belongs to this publication; gameplay and actual frame-time
improvement remain unmeasured by instruction.
