# Camera sector lookup static handoff — 2026-10-09

Base: `b06d4c66ec54ea408d3dcf3ab093c5d2d101f01f`, verified as current
`main`. Isolated source-only worktree:
`output/worktrees/camera-sector-index-20261009`, branch
`codex/camera-sector-index-20261009`. No disc, generated asset or cache copies
were made. The parallel character-bank physics work owns `character.gd` and
`src/godot/character.rs`; this change does not edit either file.

The user reported seconds per frame and authorized both fixes concurrently,
using native Godot patterns. This handoff contains static reasoning and local
code changes only. No build, test, gameplay run, benchmark, performance check,
Godot launch, asset regeneration or push was performed. Rust source was
formatted with `rustfmt`. Kip owns integration and publication.

## Removed repeated work

Camera neighbor lookup previously recursively visited the entire scene for
each collision neighbor. Initial history binding made another full descendant
search. Mode-zero classification collected every Node3D, rescanned that list
for each eligible sector, discovered mesh descendants, copied Mesh faces and
ran a GDScript intersection loop for every triangle. Pose publication could
research the same selected third position a second time. The supplied Manor
figure of 316,122 distinct serialized paths includes inherited overrides; it
is not a measured count of live nodes or a new performance result.

`src/godot/scene.rs` now saves one `SourceSectorIndex` child of each level.
It emits an explicit `PackedStringArray` in native depth-first sector order,
sector ID-to-NodePath bindings, ordered physical-visual paths grouped by the
nearest owning sector, and mesh paths discovered during conversion. Dictionary
serialization order does not determine sector ties. Existing full member
bindings for Character/compiled-AI operands remain available.

`scene/sector_index.gd` uses those native paths for lookups and downward
classification. Sector metadata is parsed once per distinct value, including
invalid values. Mesh resources supply their cached native `TriangleMesh`
acceleration structures directly; runtime queries do not copy faces or scan
triangles in GDScript. `scene/sector_boundary.gd` builds and retains a native
BVH for its saved boundary vertices, invalidating it on vertex assignment.
Wireframe presentation and the directed portal-crossing contract retain their
existing geometry and behavior.

`camera/source_camera.gd` uses the index for collision neighbors, initial
history identity and destination bindings. It reuses the camera's reachable
set when composing the target-sector set, and reuses an already computed
position result during pose publication. A distinct selected second position
still gets its own research before holder publication and the matrix write.
Position results are not cached across updates.

## Semantics and ownership

The query retains active/nonvirtual gates, `unknown[0]` child gating, supported
single-LOD visual eligibility, unsupported type-1/11 and multi-LOD diagnostics,
strict nearest-distance comparison, ordered sector ties, final-sibling
fallback and membership-before-translation publication. Live node transforms
are read for every ray. Transforming a world-unit downward direction into
local space and restoring its ray parameter preserves distance comparisons
through scaled/sheared bases.

Bindings are ordinary saved NodePaths relative to the level-owned component.
Every access checks validity, pending deletion and level ancestry. The index
does not hold live nodes or a global level cache. Removed sector/visual paths
are skipped in point research; a missing required neighbor still fails binding.
The metadata cache is cleared on tree exit. Shared Mesh BVHs are owned and
invalidated by Godot; boundary BVHs belong to their boundary nodes.

## Integration and remaining uncertainty

Support interface is provisionally **84** in `support.rs`, `version.json` and
the addon README. The new script and UID are included in the packaged support
closure. Reconcile this version with the simultaneous physics contribution,
then update maintained support and regenerate selected level scenes with the
combined exporter. Support-only publication cannot supply the new paths.
Existing levels without `SourceSectorIndex` receive an explicit regeneration
diagnostic. Rete and source assets do not require reimport or schema changes.

Native [TriangleMesh](https://docs.godotengine.org/en/stable/classes/class_trianglemesh.html)
queries use Godot's BVH and numerical conventions. The inspected
[4.7 implementation](https://github.com/godotengine/godot/blob/4.7/core/math/triangle_mesh.cpp)
snaps triangle vertices to 0.0001 local units. Physical-visual `Mesh.get_faces()`
already used that native geometry; boundary research previously used its raw
vertices. Near boundary edges or nearly equal hits can therefore differ from
the removed script's epsilon conventions. A 0.0001 ray-parameter start margin
retains surface-origin hits; a face behind the original origin triggers a
second native forward query so it cannot hide a valid intersection.

First-use native BVH setup, syntax/import acceptance, observed camera behavior
and actual frame-time improvement remain unmeasured under the user's restriction.
Bindings describe the exported static hierarchy; authored additions or
reparenting of sector geometry must update the native bindings or regenerate.
Existing manual camera-projection fixtures construct unindexed scenes and need
corresponding native bindings when test work is authorized; tests were neither
edited nor run in this task.
