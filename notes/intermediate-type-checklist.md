# Rete Type Promotion Checklist

Implementation checklist for promoting OpenSpace structs into Rete canonical types. Read [`notes/rete-implementation.md`](rete-implementation.md) first. Format: [`docs/rete-format.md`](../docs/rete-format.md). Plan: [`plan.md`](../plan.md).

Checked items are represented as Rete content elements and export through struct codecs. Some checked kinds are structured JSON; others are named binary leaves that still need field-level schemas.

## Next Agent Goal

Flesh out every type that is documented in `docs/` but still emitted as a named opaque binary leaf. The target state is that documented structures compile from structured JSON, with dense buffers described by JSON and stored as binary payloads. Unknown fields should be preserved explicitly as named scalars or small byte arrays. Opaque binary leaves should remain only for genuinely undocumented or unclassified data.

**Rule:** if implementation needs to parse or traverse a structure, it must be promoted — not left in `types/raw/`. See Engineering rules in [`plan.md`](../plan.md).

Definition of done for each promoted type:

- [ ] Add canonical type fields in `src/Astrolabe.Core/FileFormats/`.
- [ ] Add struct codec in `src/Astrolabe.Core/Serialization/` with pointer metadata.
- [ ] Preserve every byte needed for byte-perfect compilation, but do not depend on original file offsets or original element lengths for normal rebuilds.
- [ ] Use JSON as metadata/control structure, not as the default representation for dense numeric or byte buffers.
- [ ] For dense arrays, emit a JSON descriptor plus a binary payload leaf.
- [ ] For AI scripts and behavior ASTs, emit S-expression source as the meaningful editable representation.
- [ ] Represent pointers as one-line reference URIs (`file.json`, `doc.json#/key`, `../fix/types/...`); virtual addresses are derived at OpenSpace export.
- [ ] Preserve unknown fields as explicitly named `unknown*` fields, typed integers/floats where their width is known, or Base64 only when the internal shape is not known yet.
- [ ] Keep the unedited import -> export-openspace round trip byte-identical (`cmp` every level file).
- [ ] Add at least one changed-content smoke check for the promoted type where a safe scalar edit changes the rebuilt SNA.

## Package Shape Direction

The current package is too eager to create tiny files. Move toward larger hierarchical JSON documents that use explicit path references instead of relying on folder depth to encode relationships.

Preferred shape examples:

- `scene/actual_world.json` instead of one `node.json` per scene folder when the hierarchy can be represented as nested or referenced node records.
- `geometry/meshes.json` plus `geometry/buffers/*.bin` instead of one JSON file per vertex, normal, UV, or triangle island.
- `animation/families.json` plus `animation/buffers/*.bin` for frame/channel data.
- `ai/models.json` plus S-expression files for scripts, behaviors, and macros that are AST-shaped.

Use JSON path references for relationships and payloads:

```json
{
  "kind": "vertices",
  "format": "float32x3",
  "count": 128,
  "stride": 12,
  "endianness": "little",
  "path": "geometry/buffers/mesh_012_vertices.bin",
  "sha256": "..."
}
```

Binary payloads are appropriate for:

- dense numeric arrays: vertices, normals, UVs, triangle indices, animation frame/channel data, collision vertices.
- raw script node arrays only as preservation/support data for the S-expression source representation.
- preserved encoded SNA/RTB payloads needed for byte-identical unchanged output.
- loose source files and genuinely unknown large spans.

Documented fixed-size structs should usually be JSON fields, not binary blobs. Unknown fields inside those structs should be explicit scalars or small byte arrays. Large unknown spans can stay binary with a JSON descriptor that names their owning context and expected byte count/hash.

Avoid file-per-node or file-per-small-struct output by default. Use separate files when they are independently meaningful edit targets, large enough to deserve their own leaf, or shared by multiple parent records.

## AI Script Representation

AI scripts are already documented as serialized S-expression trees in `docs/ai-script-format.md`, and `src/Astrolabe.Core/FileFormats/AI/SExpressionConverter.cs` has the existing conversion path.

Represent script-like ASTs as S-expression files where possible:

- scripts under `script`.
- normal and reflex behavior scripts under `behaviors_normal` and `behaviors_reflex`.
- macro/action trees if they use the same indent-encoded AST shape.

Use JSON for AI model, brain, mind, intelligence, behavior-list, DSG, and script table metadata. The JSON should reference S-expression source paths and any raw preservation payloads needed for byte-perfect compilation.

If a behavior-related structure is not actually AST-shaped, keep it as JSON metadata plus binary buffer only until its shape is understood.

## Filesystem Shape

- [x] Level manifest
- [x] Scene folders
- [x] `scene/.../node.json`
- [x] `scene/.../matrix.json`
- [x] `scene/.../static_matrix.json`
- [x] SNA block `content.json`
- [x] `types/<kind>/...` grouped leaves
- [x] Relocation table JSON
- [x] Loose level files
- [x] Preserved encoded payloads for byte-identical unchanged output

## Structured JSON

- [x] `superObject`
- [x] `matrix`
- [x] `geometricobject`
- [x] `physicalobject`
- [x] `ipo`
- [x] `gamematerial`
- [x] `boundingvolume`
- [x] `collidematerial`
- [x] `vertices`
- [x] `normals`
- [x] `trianglenormals`
- [x] relocation pointer blocks

Note: `vertices`, `normals`, and `trianglenormals` are currently structured JSON. They should move to JSON descriptors plus binary payloads as part of the package-shape cleanup.

## Named Binary Leaves

These are promoted into `types/<kind>/` and compile through the content stream, but still need full field-level JSON schemas.

- [x] `actiontable`
- [x] `actiontree`
- [x] `aimodel`
- [x] `alwayssuperobjects`
- [x] `animationmontreal`
- [x] `animchannel`
- [x] `animchannelptrs`
- [x] `animframes`
- [x] `animhierarchies`
- [x] `animhierarchiesheader`
- [x] `behaviorlist_normal`
- [x] `behaviorlist_reflex`
- [x] `behaviors_normal`
- [x] `behaviors_reflex`
- [x] `brain`
- [x] `collideelementptrs`
- [x] `collideset`
- [x] `collidezddlist`
- [x] `collidezddzone`
- [x] `collidezdelist`
- [x] `collidezdezone`
- [x] `collidezdxlist`
- [x] `compressedmatrix`
- [x] `dsgmem`
- [x] `dsgvar`
- [x] `dsgvarptrindirect`
- [x] `dynam`
- [x] `elementptrs`
- [x] `elementsprites`
- [x] `elementtriangles`
- [x] `elementtypes`
- [x] `intelligence`
- [x] `loddataoffsets`
- [x] `loddistances`
- [x] `mind`
- [x] `objectlist`
- [x] `objecttypeentry`
- [x] `objecttypename`
- [x] `perso`
- [x] `perso3ddata`
- [x] `persosectorinfo`
- [x] `radiosityheader`
- [x] `script`
- [x] `scriptptrs`
- [x] `sector`
- [x] `sectorcollidegeo`
- [x] `sectorcollideverts`
- [x] `sectorname`
- [x] `spawnableentry`
- [x] `standardgame`
- [x] `state`
- [x] `transition`
- [x] `triangles`
- [x] `uvmapping`
- [x] `uvs`
- [x] `vertexindices`
- [x] `visualmaterial`
- [x] `visualset`

## Documented Type Promotion Backlog

Promote these named binary leaves first because the local docs, readers, tracker comments, or reference projects already describe enough structure to produce useful JSON now.

### Geometry and Materials

References: `docs/geometry-format.md`, `docs/lighting.md`, `src/Astrolabe.Core/FileFormats/Geometry/`, `src/Astrolabe.Core/FileFormats/Materials/`, `reference/raymap/Assets/Scripts/OpenSpace/Visual/`.

- [x] `geometricobject` structured JSON exists.
- [x] `physicalobject` structured JSON exists.
- [x] `visualset`: LOD count/type plus LOD distance/data pointer fields.
- [x] `gamematerial` structured JSON exists.
- [x] `visualmaterial`: flags, texture pointer/index data, material coefficients, transparency/lighting fields, unknown fields.
- [x] `elementtypes`: typed element table with sprite/triangle element kind values (`UInt16ArrayCodec`).
- [x] `elementtriangles`: triangle-element header, counts, pointer fields, material references.
- [x] `elementsprites`: sprite-element header and pointer/material fields.
- [x] `triangles`: triangle index/material records (`UInt16ArrayCodec`).
- [x] `vertexindices`: index arrays used by triangle elements (`UInt16ArrayCodec`).
- [x] `uvs`: UV coordinate arrays (`Float2ArrayCodec`).
- [x] `uvmapping`: UV mapping records (`UInt16ArrayCodec`).
- [x] `loddataoffsets`: LOD data pointer table (`PointerArrayCodec`).
- [x] `loddistances`: LOD distance values (`FloatArrayCodec`).
- [x] `radiosityheader`: radiosity/lit vertex-color metadata.

### Perso, Families, Object Lists, and Names

References: `docs/perso-mesh-animation.md`, `docs/file-format-catalogue.md`, `src/Astrolabe.Core/FileFormats/Animation/FamilyReader.cs`, `src/Astrolabe.Core/FileFormats/ObjectTypeReader.cs`, `src/Astrolabe.Core/FileFormats/TrackingSuperObjectReader.cs`, `reference/raymap/Assets/Scripts/OpenSpace/`.

- [x] `perso`: instance fields, links to 3D data, standard game, brain, mind/intelligence, collision, sector info.
- [x] `perso3ddata`: family/object-list/state links and graphics state.
- [x] `standardgame`: family/model/instance indices and object type references.
- [x] `objectlist`: object-list header and entries.
- [ ] `objecttypeentry`: object type table entries.
- [ ] `objecttypename`: decoded names as text JSON, preserving original bytes when needed.
- [x] `spawnableentry`: spawnable perso list entries.
- [ ] `alwayssuperobjects`: always-loaded SuperObject list entries.

### Animation and State Machines

References: `docs/perso-mesh-animation.md`, `docs/file-format-catalogue.md`, `src/Astrolabe.Core/FileFormats/Animation/`, `src/Astrolabe.Core/FileFormats/TrackingSuperObjectReader.cs`, `reference/raymap/Assets/Scripts/OpenSpace/Animation/`.

- [x] `state`: state name buffer, animation reference, transition list, mechanics/flags.
- [x] `transition`: transition target, condition/action references, unknown fields.
- [ ] `actiontable`: action table header and pointers.
- [ ] `actiontree`: action tree nodes or encoded action graph data.
- [x] `animationmontreal`: animation header, frame/channel counts, speed/timing data.
- [x] `animframes`: frame records.
- [x] `animchannel`: channel records, object index switching, hierarchy/compressed matrix links.
- [x] `animchannelptrs`: channel pointer table (`PointerArrayCodec`).
- [ ] `animhierarchies`: hierarchy records.
- [x] `animhierarchiesheader`: hierarchy header/counts.
- [x] `transform` (was `compressedmatrix`): Montreal compressed matrix wire + trailing stream gaps via `TransformCodec`; pooled in `animation/transforms.json` with channel URIs.

**Animation package shape:** nested `animation/families.json` (Family → State ownership) + transform pool; block `content.json` v2 segments/expand carry stream order. See dual-layer rules in [`docs/rete-format.md`](../docs/rete-format.md).

### AI, Scripts, DSG, and Behavior

References: `docs/ai-script-format.md`, `src/Astrolabe.Core/FileFormats/AI/`, `src/Astrolabe.Core/FileFormats/Animation/FamilyReader.cs`, `src/Astrolabe.Core/FileFormats/TrackingSuperObjectReader.cs`, `reference/raymap/Assets/Scripts/OpenSpace/AI/`.

- [x] `brain`: links to mind and AI model data.
- [x] `mind`: current AI state and AI model references.
- [x] `intelligence`: current behavior state and active behavior references.
- [x] `aimodel`: behavior lists, macro lists, DSG variable definitions.
- [ ] `behaviorlist_normal`: normal behavior list header and entries.
- [ ] `behaviorlist_reflex`: reflex behavior list header and entries.
- [ ] `behaviors_normal`: normal behavior records.
- [ ] `behaviors_reflex`: reflex behavior records.
- [x] `scriptptrs`: script pointer arrays (`PointerArrayCodec`).
- [ ] `script`: S-expression AST source, plus raw ScriptNode preservation only as needed for byte-perfect compilation.
- [ ] `dsgvar`: designer variable definitions with documented type ids.
- [ ] `dsgmem`: per-instance DSG variable values.
- [x] `dsgvarptrindirect`: indirect pointer/value table for DSG data (`PointerArrayCodec`).

### Sectors and Collision

References: `docs/file-format-catalogue.md`, `docs/lighting.md`, `src/Astrolabe.Core/FileFormats/SuperObjectReader.cs`, `src/Astrolabe.Core/FileFormats/TrackingSuperObjectReader.cs`, `reference/raymap/Assets/Scripts/OpenSpace/`.

- [x] `sector`: sector structure, linked lists, neighbor entries, activity/sound/graphics/collision sector references.
- [ ] `sectorname`: decoded sector names as text JSON, preserving original bytes when needed.
- [x] `persosectorinfo`: perso sector membership links.
- [ ] `sectorcollidegeo`: sector collision geometry header and references.
- [ ] `sectorcollideverts`: sector collision vertex arrays.
- [x] `collideset`: collide set header and pointers.
- [x] `collideelementptrs`: collide element pointer arrays (`PointerArrayCodec`).
- [ ] `collidezdxlist`: ZDX collision zone list entries and target zone references.
- [ ] `collidezddlist`: ZDD collision zone list entries and target zone references.
- [ ] `collidezdelist`: ZDE collision zone list entries and target zone references.
- [ ] `collidezddzone`: ZDD zone fields.
- [ ] `collidezdezone`: ZDE zone fields.

### Miscellaneous Documented or Partially Documented Leaves

References: `docs/file-format-catalogue.md`, `src/Astrolabe.Core/FileFormats/TrackingSuperObjectReader.cs`, `reference/raymap/`.

- [ ] `dynam`: dynamic object/physics state fields.
- [x] `elementptrs`: generic element pointer arrays where not covered by geometry-specific serializers (`PointerArrayCodec`).

## Current Verification

- [x] Build succeeds on .NET 10.
- [x] Unchanged intermediate extraction and compilation round-trips byte-identically on `astrolabe`.
- [x] `debug-relocations`: `astrolabe.rtb` 49,531 / 69,922 matching, 0 extra; RTP/RTT exact; `fixlvl.rtb` 1,060 / 1,117 matching.
- [x] Editing `scene/.../node.json` changes the rebuilt SNA.
- [x] Editing `scene/.../matrix.json` changes the rebuilt SNA.
- [x] Editing `types/vertices/*.json` changes the rebuilt SNA.
- [x] Editing `types/visualmaterial/*.json` changes the rebuilt SNA.

## Verification Commands

Use these commands after each promotion pass:

```bash
dotnet build
dotnet run --project src/Astrolabe.Cli -- extract-intermediate disc/Gamedata/World/Levels/astrolabe output/test-intermediate-astrolabe
dotnet run --project src/Astrolabe.Cli -- compile-intermediate output/test-intermediate-astrolabe output/test-compiled-astrolabe
for src in disc/Gamedata/World/Levels/astrolabe/*; do name=${src##*/}; cmp -s "$src" "output/test-compiled-astrolabe/$name" || printf 'DIFF %s\n' "$name"; done
```

The compare command should print nothing for an unedited package.

After testing, remove temporary output directories:

```bash
rm -rf output/test-intermediate-astrolabe output/test-compiled-astrolabe output/test-edited-astrolabe output/test-edited-compiled-astrolabe
```

## Godot Handoff Note

Do not wire the intermediate package into `export-godot` until the documented type promotion pass is stronger. The intermediate should become the source layer for Godot export after the core scene, geometry/material, Perso, animation, AI, and sector/collision structures are represented as meaningful JSON.
