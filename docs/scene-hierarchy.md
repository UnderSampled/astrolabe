# Scene hierarchy reconstruction

The useful scene now retains the complete supported Hype super-object (SO)
record: node kind, ordered hierarchy, placement, draw mask, light and picking
bounds, initial global transform and source opacity. The canonical
`SceneNodeProperties` groups node-owned values independently of reusable model
geometry and character data. Rete schema 9 persists these values and typed
bound references. Source addresses and matrix cache frame numbers stay outside
the model.

## Evidence and ownership

Raymap's Montreal `SuperObject` reader was the first layout reference. The
original Hype engine's `HIE_fn_hCreateSuperObject` (004fc660) allocates 64 bytes,
creates independent global and initial-global matrices, sets the draw mask to
`0xffffffff`, and initializes opacity to 255. The 150 source allocations are
all 64 bytes. The former importer read only the first 40 bytes plus the draw
mask separately.

| Byte offset | Meaning |
| ---: | --- |
| 0, 4 | Source kind and data component |
| 8, 12, 16 | First/last child and child count |
| 20, 24, 28 | Next/previous sibling and parent |
| 32, 36 | Local and computed global matrices |
| 40, 44 | Matrix cache frame and draw mask |
| 48, 52 | Light and picking-bound geometry |
| 56, 60 | Initial global matrix and opacity |

Both source kinds `0x0d` and `0x15` map to a useful object, but their identity
is retained for reconstruction. The +56 accessor is
`HIE_fn_hGetSuperObjectGlobalMatrixAtT0` (004661a0); the +60 accessor is
`HIE_fn_fGetSuperObjectTransparenceLevel` (004663d0).

`HIE_fn_vPushSuperObjectMatrix` (004fcc60) composes parent and local matrices
when the cache needs refreshing. A cache frame of `0xffffffff` selects a static
world matrix; `HIE_fn_vSetSuperObjectStaticMatrix` (00466390) establishes that
mode. The shared model represents the distinction as local or world placement.
The observed Astrolabe nodes all use local placement. Godot uses `top_level`
for world placement, including when the parent later moves.

The source's 145 node-bound references resolve to 58 shared collision-geometry
assets, including sharing with family bounds. Bounds are useful geometry for
visibility/picking, separate from physical collision bindings. The empty
sphere at `world/1/16` has center index 0 and radius 0.
`HIE_fn_vComputeBoundingVolumeOfSuperObjectNode` (004ff7c0) explicitly creates
zero-radius bounds for empty nodes. Such spheres survive Rete and source
reconstruction. Godot skips their physics/debug primitives and retains node
bound metadata without creating extra collision bodies. Negative radii remain
invalid.

## Reconstruction and checks

`openspace::level::write_scene_graph` accepts a `Scene` and explicit component
layout targets. It derives SO links, matrix locations and relocation entries
from current values. It preserves one global matrix per node, rather than
merging distinct equal matrices. Optional external local-matrix targets allow
the owning character writer to place matrices inside Perso3dData. The caller
must write those values at the supplied placements; the SO writer never reads
the original disc or substitutes saved addresses. Dynamic cache counters are
invalidated and static-world counters are derived from placement mode.

The corpus test closes the representation through Rete, emits a new controlled
layout, and checks all 150 records against independent source hierarchy links,
component targets, scalar words and initial matrices. It also checks all 108
character local matrices alias their respective embedded graphics matrices.
Original addresses supplied as external placements in this test are transient
test evidence only. Synthetic edits exercise changed parent translation,
rotation/scale composition, world placement, source-kind identity, opacity and
initial matrices. Native Godot checks move a parent, verify both placement
modes and ensure picking/empty bounds do not create physical shapes.

The focused run passed all 20 selected library/scene-graph tests in 46.45
seconds including compilation and cleanup, recorded in
`output/scene-graph-focused-tests.log`. See
[implementation progress](implementation-progress.md) for the latest complete
suite result.

This API is not yet composed into the CLI's scene assembly: complete Perso,
sector/environment and package layouts remain dependent work. The CLI emits
supported leaf fragments, now including node picking bounds, and explicitly
reports that gap. Per-node fading is retained and diagnosed but not projected
by the Godot compatibility renderer. Original-engine acceptance of a rebuilt
whole scene remains unverified. No reference submodule or original game asset
was modified.
