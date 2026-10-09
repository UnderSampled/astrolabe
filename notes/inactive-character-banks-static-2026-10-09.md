# Inactive character banks, 2026-10-09

Static implementation and review only. The user prohibited tests, Godot/game
launches, benchmarks, performance checks, builds and asset regeneration in this
worker. No such commands were run. Main was clean at the requested
`b06d4c66ec54ea408d3dcf3ab093c5d2d101f01f`. Work is isolated in
`codex/inactive-character-banks-20261009` at
`/tmp/astrolabe-inactive-character-banks-20261009`, without game asset copies.

## Native participation

The exporter retains saved native payloads for arbitrary shared configuration
permutations, normal/reset selection and editor inspection. Parking banks now
persist `PROCESS_MODE_DISABLED` and `top_level = true`. Inactive animation slot
anchors have the same defaults. A final conversion step applies the actor's
activation/suppression and reconciles slot participation with the saved initial
channel/actor visibility before packing the Perso scene. PhysicalContact areas
explicitly use `CollisionObject3D.DISABLE_MODE_REMOVE`; their source material
disabled flags, layers, masks and non-monitoring behavior are preserved.

This uses native Godot lifecycle rather than manually reconstructing a physics
space or deleting/recreating shapes. Godot propagates process-owner enabled and
disabled notifications to inheriting descendants. REMOVE unregisters areas
from their physics space and restores them when enabled. Node3D transform
propagation skips top-level children, while visibility still follows ancestors.
These rules were inspected in the official
[Node implementation](https://github.com/godotengine/godot/blob/master/scene/main/node.cpp),
[Node3D implementation](https://github.com/godotengine/godot/blob/master/scene/3d/node_3d.cpp)
and [CollisionObject3D implementation](https://github.com/godotengine/godot/blob/master/scene/3d/physics/collision_object_3d.cpp).
This is static API evidence, not a run on the project's pinned Godot 4.7.2.

The maintained configured-slot script runs in the editor as well as gameplay.
Native visibility signals reconcile slot process mode and transform inheritance
only when visibility changes. Suspension precedes transform changes; resumption
restores the local transform and publishes contained PhysicalContact transforms
before enabling physics. Frozen transforms retain local placement, including
authored anchor edits. No scene-wide per-frame participation scan was added.
Unchanged transparency keys no longer rescan every GeometryInstance descendant;
configuration reparenting still explicitly reapplies the destination opacity.

Inactive banks and unselected/hidden slots contain physical-object fallback,
which the existing contact API already rejects when not visible in the tree.
Independent ZDE/ZDR gameplay zones keep their own source state selection rules
and can remain eligible while rendering is hidden. Display suppression through
visual render layers or material opacity does not disable those zones or physical
fallback. Source object deactivation disables the actor's native physics through
inherited process mode; reactivation and save restoration use the same method.
State-selected contact updates now inspect only ZDE/ZDR descendants rather than
every CollisionShape3D in the character's payload banks.

Configuration selection, permutations and restoration still reparent saved
payloads without constructing assets. Reparenting transfers native process
inheritance to the destination bank/slot. Normal/reset library changes and native
seeking update slot visibility through the same signals. Shared configuration
signals disconnect when an actor leaves the tree; a re-entered actor reconnects
and applies current shared state after its descendants enter. Godot removes
collision objects when their nodes exit the tree.

## Integration handoff

Changed production files:

- `src/godot/character.rs`
- `godot/addons/astrolabe/character/character.gd`
- `godot/addons/astrolabe/character/configured_slot.gd`

No camera, sector lookup, reference, authored game, generated content or tests
were edited. The worker could not send progress directly to the delegated source
thread: its supplied ID returned `thread not found`. Final delivery carries the
handoff. Integration and publication belong to Kip.

Kip must compose the support-interface version with the simultaneous camera
change, update adopted support, and regenerate affected native Perso/level scenes
for the new exported defaults. Keep generated animations together with the scenes
as a normal scoped publication; native track paths and Rete meaning/schema are
unchanged. A support-only update does not set disabled/frozen properties on the
already saved parking banks. The final conversion step now calls the maintained
`Character.refresh_configured_participation()` method, so exporter and support
must be published together. No new script or UID is required.

Saved payload/node duplication is retained to preserve arbitrary Proc82 swaps and
complete native scenes. This fix removes dormant payloads from the active physics
space and stops their inherited transform work; it does not claim smaller saved
node counts, faster loading or measured frame-time improvement. Syntax/resource
validation, actor re-entry, shared normal/reset swaps, backward native seeking,
channel overrides, invisible ZDE/ZDR eligibility, activation/save restoration and
actual gameplay remain unexecuted under the worker constraint. Non-monitoring
exported PhysicalContact areas do not deliver physics callbacks; an authored
monitoring contact that changes visibility inside its callback must defer that
mutation as required by Godot.
