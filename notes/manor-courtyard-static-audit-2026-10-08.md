# Manor courtyard approach: static collision audit — 2026-10-08

Audit baseline: `d493ef2`, in an isolated worktree. This investigation used
read-only source/archive inspection. No tests, compilation, formatter/lint
checks, Godot, original game, replay, or gameplay harnesses were run. Earlier
trace reports below are historical input, not new runtime acceptance.

**Disposition: no demonstrated import/controller defect causing the reported
invisible wall.** Keep the source forest collision. The supplied straight-Z
trace hits a finite, rendered source wall beside an opening; the supplied
angled trace crosses through that opening. The user's actual trajectory and
first blocking contact remain unknown. This does not establish that the user's
report is resolved, or that all sector/gate behavior is correct.

## Inspected content and source

- Original `disc/Gamedata/World/Levels/manoir/manoir.sna`, SHA-256
  `9dae1c2612535eab0eb823f838bdb845d0d1dccba26f3c0814d81f3541c51af4`.
- Research PE `reference/hype_patch/MaiDFXvr_bleu.exe`, SHA-256
  `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
- Ghidra functions `00464570`, `00464150`, `00463df0`, `00447f90`,
  `004b74f0`, `004e0020`, `004b7ab0`, `0049df90`, and `0049e4e0` under
  `reference/decompilation/functions/`; the material-exclusion branch was
  also read directly from the PE instructions.
- Raymap's `OpenSpace/Collide/CollideMaterial.cs` and
  `OpenSpace/General/GameMaterial.cs`, consulted for flags/layout before
  checking the original implementation.
- The independent clean-C project's
  `/home/stephen/Projects/hype-decomp/src/collision/actor_zone_query.c` and
  `actor_zone_lookup.c`, plus its coverage inventory. Those C bodies concern
  actor-zone queries, not physical-world queries.
- Existing local source Rete and trace reports under
  `output/worktrees/manor-courtyard-collision-20261008/output/courtyard-trace-20261008/`.
  The cached Rete envelope is schema 48. It was read as historical inspection
  data, not loaded into the current adapter or treated as a current port run.

The archive was decoded only to inspect its original records. In its geometry
block, the complete 51-triangle index sequence and complete face-normal
sequence for the candidate element each occur once. The three vertices of the
reported hit triangle each occur twice, consistent with separate visual and
collision meshes. Material allocation 5353 directly contains sound material 3
and the default collision sentinel. No pointer API or runtime state was added.

Ignored local evidence in this audit's worktree is
`output/courtyard-static-audit-20261008/source-read-summary.json` and
`source-material-branch.txt`. Raw disassembly and game-derived assets are not
part of the contribution.

## The traced face and the passage

All positions here use Godot world coordinates; conversion is
`[source_x, source_z, -source_y]`.

The historical straight-Z report starts at
`(-74.38131, 4.54829, -28.50581)` and commands `(0, 0, -1)`. Its first blocker
is `world/1/5/0/Collision`, shape 0, with material
`level:/materials/06-02/5353`. The hit point is approximately
`(-74.37908, 6.62500, -32.60405)`, with normal `(0, 0, 1)`.

That point lies on collision element 0, triangle 1, whose source indices are
`[56, 49, 48]`. Its projected vertices before Godot winding reversal are:

| Vertex | X | Y | Z |
| --- | ---: | ---: | ---: |
| 56 | -75.815163 | 5.035131 | -32.603958 |
| 49 | -73.071968 | 5.035138 | -32.604137 |
| 48 | -72.279953 | 10.117311 | -32.604183 |

The historical geometry report finds the same triangle on render surface 0,
triangle 1, about `0.000004` units from the hit. The render mesh is visible
and its material is opaque, using `texture:/manoir/za15txy.gf`. The traced
contact therefore does not demonstrate an extra collider across an empty
rendered opening. Whether the user could see this face from their camera still
needs a view associated with their actual contact.

The two sides of the opening have nearly vertical edges around X `-78.6428`
and `-75.8152`, at Z about `-32.604`; it continues through the wall's back face
near Z `-33.253`. The centerline is approximately X `-77.229`. These are mesh
edges, not promised movement clearance independent of actor shape, height,
orientation, or margin.

The historical angled report commands `(-0.5, 0, -0.8660254)` and ends at
`(-76.85920, 5.05606, -32.77584)` without a reported blocking contact. That
endpoint is between the opening's side edges. Straight-Z at X `-74.38` heads
into the wall east of the opening; the angled route moves west into it. Neither
artificial direction is known to reproduce the user's input.

## Material and sector eligibility

Material 5353 has `CollisionProperties::Default`, represented in the original
game material by `0xffffffff`, rather than an allocated material with an
identifier flag. `load_material` in `src/openspace/level/import.rs` preserves
that distinction. `src/godot/collision.rs` retains the shape and disables it
only for an actual `0x8000` identifier.

Original `00463df0` reads the returned game material's collide-material
reference using `0049df90` and explicitly skips the identifier test for the
default sentinel. For a concrete collide material, `0049e4e0` reads its
16-bit identifier and `00463df0` rejects contact when bit `0x8000` is set.
The PE shows the sentinel comparison followed by the high-byte `0x80` test.
There is no material exclusion flag here to restore by disabling this face.

The owning sector is `world/1/5`, `sctforet`: nonvirtual, activation
`0xffffffff`, with collision neighbors `world/1/25` and `world/1/4`.
`world/1/25`, `sctpont01`, also explicitly lists `world/1/5`. `00464570`
requires current-sector activation bit 2, traverses that sector, then traverses
explicit collision neighbors with bit 2. `00464150` visits physical-object
descendants and selects their collision set's ZDR through `004b74f0`, matching
this object's imported binding.

Supporting static geometry inspection finds a forest visual floor about
`0.01972` units below the supplied start and about `0.02678` below the straight
endpoint. Original `004e0020` considers active nonvirtual sector borders and
eligible visual GEOs with a downward semiaxis when finding an actor's sector.
This supports forest ownership at these points; it is not an observation of
the original game's live sector-history state.

The exporter currently gives static contact the union of physical sectors and
their collision neighbors. It excludes unreachable virtual display sectors;
exact per-actor static filtering and dynamic activation remain limitations.
Those limitations do not justify excluding this wall: the supplied approach
is over its own floor, and the bridge sector explicitly includes it. The
native actor-zone predicate's separate sector filter does not change
`CharacterBody3D.move_and_slide` world contact. Actor-zone filtering has not
completed physical-world filtering.

## Gate state

The source gate is `level:/persos/world/0/15`, `iMGrille_Manoir_Pont`, AI model
`level:/ai/models/272`. Its initial X is `-77.25`. Normal behavior 0 reads
Global `fix:/persos/15` DSG[149]; when true it assigns
`SelfPosition + (2.6, 0, 0)` and changes to Normal behavior 3. The false branch
changes to behavior 3 without translating. This is horizontal translation,
not a generic vertical gate lift or sector deactivation.

The prior Load Game trace records DSG[149] true and gate position
`(-74.65000, 7.53990, -32.58930)`, consistent with that branch. Existing
`src/godot/save_import.rs` admits this exact Global field; the actual game
session applies restored state before starting the gate Brain. Dummy DSG[8]
and secret lever DSG[1] must not substitute for DSG[149] to infer this gate's
passage state.

The reported collider is the forest object, not the gate. The trace's gate
root collision layer 0 alone cannot establish the gate's collision state:
the exporter gives its separately owned ZDR a `StaticBody3D` on layer 8.
Closed-gate contact, its interaction sequence, and New Game gate behavior
are not accepted by this investigation. They do not justify changing the
identified static face.

## Remaining observation and integration boundary

The smallest useful next evidence is one actual blocked approach: New/Load
route and save identity, actor position and requested motion immediately
before contact, collider/shape semantic path, hit normal, and a camera image
from that same contact. If it is this face, record current source sector,
movement shape/state and the gate's DSG[149], behavior, transform and enabled
ZDR shapes. If it is another face, inspect that source binding first. No
runtime work is authorized by this static audit.

No production files or support interfaces changed. The integration lead's
`demo.gd/.tscn` to `game_session.gd/.tscn` and `demo_camera.gd` to
`session_camera.gd` renames do not conflict with this documentation contribution.

At the first inspection, the clean-C inventory marked `00464570`, `00464150`,
`00463df0` and `004e0020` untouched. Descriptive translations would be useful
future source work; they do not block this disposition because Ghidra/PE
evidence is available. For exact actor-versus-gate physical filtering, also
inspect/translate `00464380`, `00464510`, `00430130`, `00430220`, and
`00430310`, preserving material/category filters and checking decompiler
return/global ambiguities against the PE. Do not recreate that machinery
inside Godot merely to obtain a clean-C counterpart.

## Subsequent sector translation evidence

The separate source translator subsequently supplied authored bodies in
`/home/stephen/Documents/Codex/2026-10-08/task-18/hype-sol-manor/src/sector/sector_research.c`
and `sector_traversal.c`. Both files and their private header were read only.
They are now committed in source checkpoint
`d24078a897ce98ba4f9b4e6725eab1fa0a84d73a`; no compiled or runtime acceptance
is implied. Kip owns integration.

The `004e0020` body corroborates the ordered downward `(0, 0, -1)` search,
initial closest-distance bits `0x7f7ffff8`, sector-data `+0x94` exclusion,
own boundary before child geometry, child search only when `+0x95 == 0`,
strictly smaller ordered query distance, and activation bit 4. Child kinds
13/21 select visual LOD 0. Kinds 1/11 retain the source inverse-matrix
initialization uncertainty. This strengthens the documented sector lookup
interpretation; it supplies no rule for enlarging bounds or accepting a
point outside a geometry query's supported domain.

The `004e03f0` body makes the dynamic-activation limitation more precise:
an inactive crossing advances the local query point and previous crossing,
but leaves the actor's sector reference unchanged. Its signed16 history
counter and unchecked stores belong to original-engine reconstruction,
not Godot gameplay storage. `004e04f0`/`004e0630` additionally retain ordered
face traversal, the PE-recovered equality/unordered entry gate, and signed16
face-link indices. These details do not exclude the identified forest wall:
the inspected forest and bridge activation words have all bits set.

At that checkpoint, external `004b7ab0`, `00551710`, and `00552b50` remain
unreconstructed dependencies. Kip assigned their recovery exclusively to the
same source translator; this consumer does not duplicate that work. Their
exact intersection/return contracts remain source-work gaps. The missing
actual user-contact observation and the no-defect-proven disposition above
are unchanged.
