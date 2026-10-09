# Camera primitive-mesh correction — 2026-10-09

Base: `0dbc47ce14f7f4efb68474874e98f1a04e026781`, support interface **84**.
Isolated branch: `codex/camera-primitive-bvh-20261009`; worktree:
`/tmp/astrolabe-camera-primitive-bvh-20261009`. Kip owns integration.

The assignment reports that the one completed authorized camera experiment
found identical old/current sector candidates (`world/1/13`) at all four points,
resolved bindings, matching ownership, and a forward-axis error of 0.00000185.
The previous resolver returned `complete=true`; the new resolver returned
`complete=false` because five mesh resources were not ArrayMeshes. Reported
runtime was 1.72 seconds, exit 0. The evidence paths supplied with the assignment
are `output/camera-angle-probe-20261009/report.json` and `run-result.json`.
Those two files are zero bytes in this worker's environment; the findings above
are the supplied verified handoff, not a fresh reading or repeated experiment.

`source_camera.gd` rejects incomplete results both when seeding boundary history
and when publishing a source camera position. The correction only changes mesh
eligibility and primitive acceleration-structure ownership in
`godot/addons/astrolabe/scene/sector_index.gd`.

## Implementation and static review

- ArrayMesh resources keep their existing native `generate_triangle_mesh()`
  path, including Godot's shared-resource cache and geometry invalidation.
- PrimitiveMesh resources use `get_faces()` on the first cache miss, followed by
  native `TriangleMesh.create_from_faces()`. The fallback BVH is built once per
  unchanged primitive resource, shared by that resource's indexed instances.
  Subsequent queries only look up the cached result and call native ray queries.
  There are no GDScript triangle loops or repeated face extraction.
- Empty faces cache a complete, null/no-hit geometry result. Invalid face counts
  or failed BVH construction cache an incomplete, null result. Other Mesh classes
  retain the existing unsupported/incomplete behavior.
- A cache entry strongly retains its PrimitiveMesh and nullable TriangleMesh;
  an instance ID cannot be reused while that entry exists. The changed callback
  binds only the integer ID, avoiding a resource-to-callable reference cycle.
- `Resource.changed` disconnects and discards that resource's entry. The next
  lookup rebuilds from the current primitive. Subscription happens after face
  extraction, which can itself generate a primitive and emit `changed`.
- Tree exit disconnects all retained resource callbacks and releases all entries.
  Live mesh replacements naturally use the new resource ID. Old unused entries
  are bounded by resources queried during that level's lifetime and released on
  change or tree exit; no global/node cache is introduced.

The inspected [Godot 4.7 Mesh source](https://github.com/godotengine/godot/blob/4.7/scene/resources/mesh.cpp)
binds `get_faces()` on Mesh and `generate_triangle_mesh()` on ArrayMesh. Its
`get_faces()` internally uses a native mesh cache; the primitive fallback owns
its own script-accessible TriangleMesh. The
[PrimitiveMesh implementation](https://github.com/godotengine/godot/blob/4.7/scene/resources/3d/primitive_meshes.cpp)
clears the mesh cache and emits `changed` on a geometry update. The
[TriangleMesh implementation](https://github.com/godotengine/godot/blob/4.7/core/math/triangle_mesh.cpp)
binds `create_from_faces(PackedVector3Array) -> bool` and
`intersect_ray(Vector3, Vector3) -> Dictionary`. The existing ray distance and
transform handling are unchanged. Godot's generic stable Mesh documentation
lists `generate_triangle_mesh()` on Mesh; this implementation follows the
actual script bindings in the source version used by the existing handoff.

## Installation and limits

Support interface stays **84**: no exported properties, NodePaths, caller
signatures, schema, or generated scene/resource representation changes. The
already published indexed Manor level can use this correction by installing the
updated native support script alone; **no content reconversion is needed**.
Source checkout consumers receive it through Git integration. Adopted external
projects can receive it through the normal reviewed support update with a CLI
that embeds this commit; rebuilding that CLI is an integrator action. Older
levels without `SourceSectorIndex` still need the original interface-84
regeneration described in the [earlier handoff](camera-sector-index-static-2026-10-09.md).

Static source/API review only. No tests, builds, parser/import checks, benchmarks,
gameplay launches, asset regeneration, pushes, save/settings changes, or process
interference were performed. Runtime syntax acceptance, changed-resource
behavior and restored camera publication remain unmeasured for this correction.
There were no denied actions. The freeze investigation remains with its existing
read-only log worker; this correction does not investigate or alter it.
