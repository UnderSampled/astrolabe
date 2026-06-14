# Rete Format

Rete is Astrolabe's canonical, editable representation of an OpenSpace level. A **Rete package** is a directory of JSON documents and binary payloads that describes level structure, relationships, and data with enough fidelity to drive multiple exporters.

## Name

**Rete** (Italian and Latin: *net*) is the pierced reference plate on a historical astrolabe — the rotating star map laid over fixed coordinate plates. In Astrolabe, Rete is the reference network: JSON records, path links, and descriptors over binary ground.

| Astrolabe part | Rete |
|----------------|------|
| Rete plate | JSON graph — structs, paths, scene hierarchy, buffer descriptors |
| Fixed plates | Binary payloads — dense arrays, opaque spans |
| Alignment | Export — layout elements and resolve references into target formats |

## Architecture

Rete sits at the center of the pipeline. OpenSpace level directories and Godot projects are **export targets**, not the source of truth.

```text
OpenSpace level dir ──import──► Rete package ──export──► OpenSpace level dir
                                     │
                                     └──export──► Godot project
```

Both exporters consume the same **canonical types** (`VisualMaterial`, `GameMaterial`, scene nodes, geometry headers, and so on). Those types can be loaded by deserializing Rete JSON or built directly during import.

### Virtual memory and Fix data

OpenSpace loads **two SNA sources** into one virtual address space: the level and **Fix** — shared data used across levels (characters, common textures, and other global content). Fix lives alongside levels under `Gamedata/World/Levels/` as `Fix.sna`, `Fix.rtb`, `Fix.ptx`, and related files. Each level carries `fixlvl.rtb` (and related tables) linking **Fix pointers into that level's blocks** (chiefly level `ObjectList` mesh tables — see [`fixlvl-rtb.md`](fixlvl-rtb.md)).

Rete models this as **separate packages**:

| Package | Source | Contents |
|---------|--------|----------|
| **Fix Rete** | `Fix.sna` + Fix sidecars | Shared structs, buffers, and scene data imported once |
| **Level Rete** | `{level}.sna` + level sidecars | Level-owned data only; pointers into Fix use **external paths** |

Fix is a separate Rete package. Level packages reference Fix by URI rather than embedding duplicate copies of Fix elements.

**Import layout** is a conversion-time decision, not a format rule. When a level is imported, the converter extracts the level and the Fix files it needs into the **same output parent** in one pass:

```text
output/
  fix/                 ← Fix Rete (written once per output tree)
  astrolabe/           ← level Rete for this import
    types/...            (level-local)
```

Because the converter chooses that layout at emit time, level packages land beside `output/fix/`. Cross-package URIs use `fix:/` and `level:/` package roles (see [`cross-package-uris.md`](cross-package-uris.md)), not level directory names.

Re-importing another level into the same `output/` reuses existing `fix/` when present.

Import merges level and Fix into one virtual memory map for pointer resolution, but only **level-owned** elements are written into the level package. References into Fix memory are recorded as relative URIs pointing at the Fix paths the converter wrote (or will write).

Export of a level writes only that level's OpenSpace files. Fix OpenSpace files are produced by exporting the Fix Rete package separately. `fixlvl.rtb` is generated as part of level export and links level pointers to Fix block identities.

Godot export from a level Rete package loads the referenced Fix package (or resolves external paths against it). Direct OpenSpace-to-Godot export without Rete loads both SNA sources in memory, as the engine does.

### Canonical types

Each documented OpenSpace struct maps to one lossless canonical type in `Astrolabe.Core`:

- Known fields are typed and named.
- Unknown bytes are explicit scalars or small byte arrays, never silently dropped.
- Pointer fields use **reference URIs** — one string per link.
- Virtual addresses are derived at export time, not authoritative in the package.

A **struct codec** per type handles binary read/write and JSON serialization under a versioned schema ID (for example `astrolabe.visual-material.v1`).

### Package encoding

The Rete package is the on-disk encoding of canonical types plus layout metadata:

- **Structs** — fixed-size records as JSON.
- **Descriptors** — JSON pointing at binary leaves with `format`, `count`, `stride`, `path`, and `sha256`.
- **Buffers** — dense `.bin` payloads (vertices, indices, animation data, opaque spans).
- **Elements** — ordered serialization units that concatenate into SNA block payloads on OpenSpace export.

JSON is the reference layer. Binary files are the ground it sits on. This follows the same practical split as glTF (document graph + buffer files), adapted for OpenSpace reversibility and pointer-linked data.

## Package layout

```text
manifest.json
scene/
  <root-name>/
    <node-id>/
      node.json
      matrix.json
      static_matrix.json
      <child-node-id>/
sna/
  <sna-stem>/
    blocks/
      <block-key>/
        content.json
        elements/
          <order>_<kind>.bin
      <block-key>.encoded.bin
types/
  <kind>/
    <block-key>_<order>.<json|bin>
files/
  <loose-level-file>
semantic/
  scene-tree.json
  coverage.json
```

Paths in JSON are relative to the package root and use `/` separators.

## Vocabulary

| Term | Definition |
|------|------------|
| **Rete package** | A directory conforming to this specification |
| **Canonical type** | Lossless in-memory struct corresponding to one OpenSpace record |
| **Struct codec** | Binary and JSON serializer for one canonical type |
| **Struct** | Fixed-size engine record stored as a JSON document |
| **Descriptor** | JSON metadata for a binary buffer |
| **Buffer** | Dense binary payload file |
| **Element** | One ordered piece in a block's `content.json`; serializes to a byte span |
| **Reference** | One-line URI string identifying a record (file path, optional fragment, optional package prefix) |
| **Fix package** | Shared Rete package holding cross-level OpenSpace data |

## Manifest

`manifest.json` uses schema `astrolabe.rete.v1`.

Fields:

- `packageRole` — `level` or `fix`.
- `levelName` — logical level or Fix name (for example `castle_village` or `Fix`).
- `sourceDirectoryName` — basename of the imported OpenSpace directory, when applicable.
- `snaFiles` — SNA containers and ordered block records.
- `looseFiles` — non-SNA level files under `files/`.
- `semantic` — optional inspection artifact paths.

Each SNA block record stores block identity (`module`, `id`, `key`, `order`), virtual base address, OpenSpace header fields, content paths, content hashes, and optional original encoded storage metadata.

Relocation tables (`.rtb`, `.rtp`, `.rtt`, and related files) are **not** stored in the Rete package. They are produced by the OpenSpace exporter from pointer metadata and layout.

## SNA block content

Each payload-bearing block has `content.json` with schema `astrolabe.sna-block-content.v1`.

The document records the source SNA file, block identity, virtual base address, original decompressed hash (for validation), and an ordered `elements` array.

Each element:

| Field | Role |
|-------|------|
| `order` | Concatenation position in the decompressed block |
| `kind` | Struct codec kind (for example `visualmaterial`) |
| `dataPath` | Path to JSON struct, descriptor, or binary leaf |
| `sha256` | Hash of serialized element bytes at import time |
| `labels` | Parser coverage labels from import |

OpenSpace export rebuilds the decompressed block by serializing elements in `order` and concatenating the results. Element boundaries are defined by this list, not by original file offsets.

## Structs

Documented fixed-size OpenSpace records are JSON structs under `types/<kind>/` or referenced from `scene/`.

Current struct schemas:

- `astrolabe.scene-node.v1`
- `astrolabe.super-object.v1`
- `astrolabe.matrix.v1`
- `astrolabe.geometric-object.v1`
- `astrolabe.physical-object.v1`
- `astrolabe.ipo.v1`
- `astrolabe.visual-set.v1`
- `astrolabe.element-triangles.v1`
- `astrolabe.radiosity-header.v1`
- `astrolabe.game-material.v1`
- `astrolabe.visual-material.v1`
- `astrolabe.uint32-record.v1`
- `astrolabe.float3-array.v1` (transitioning to descriptor + buffer)

Every struct document includes a `schema` field. Struct codecs declare pointer fields so the OpenSpace exporter can write pointer values and generate relocation tables.

## References

Cross-record links are **reference URIs**: one-line strings that resolve to a filesystem record. No parallel id layer.

| Form | Meaning |
|------|---------|
| `types/foo.json` | Intra-package (relative to referring package root) |
| `fix:/types/foo.json` | Shared **Fix** package (`packageRole: fix`) |
| `level:/slots/….json` | **Level** package (`packageRole: level`) — not a level name |
| `../fix/…` | Legacy; still accepted |

**Full specification** (grammar, level slots, multi-level import union, export modes, implementation status): [`cross-package-uris.md`](cross-package-uris.md).

Quick examples:

```json
{ "visualMaterial": "fix:/types/visualmaterial/hype-body.json" }
{ "defaultObjectList": "level:/slots/0262C4C0.json" }
```

Import merges level and Fix virtual memory, resolves targets, and writes reference URIs. OpenSpace export walks URIs, computes pointer values, and emits relocation tables including `fixlvl.rtb` (see [`fixlvl-rtb.md`](fixlvl-rtb.md)).

`content.json` element `dataPath` values are reference URIs without a fragment when the whole file is the element payload, or with a fragment when serialized bytes come from a sub-record inside an aggregate JSON document.

## Descriptors and buffers

Dense numeric or byte data uses a descriptor JSON plus a `.bin` buffer:

```json
{
  "kind": "vertices",
  "schema": "astrolabe.buffer-descriptor.v1",
  "format": "float32x3",
  "count": 128,
  "stride": 12,
  "endianness": "little",
  "path": "geometry/buffers/mesh_012_vertices.bin",
  "sha256": "..."
}
```

Buffers are appropriate for vertices, normals, UVs, triangle indices, animation frames, collision vertices, and large opaque spans. AI script node arrays may remain as preservation buffers alongside S-expression source files.

Opaque preservation records also use a JSON descriptor plus sidecar `.bin`, even for small blobs. When an opaque blob contains pointer fields that must survive export, the JSON descriptor carries a `pointers` map keyed by byte offset (`"0x10": "types/..."`, `null` for zero), and the exporter patches those addresses into the binary payload before SNA serialization.

## Scene tree

`scene/` holds the editable scene hierarchy. Each node folder contains `node.json` (`astrolabe.scene-node.v1`) with SuperObject fields, stable node id, package path, display name, child paths, matrix paths, and semantic links.

When a scene file is referenced from SNA `content.json`, that scene file is authoritative for export.

Longer term, aggregate scene documents (for example `scene/actual_world.json`) may replace deep per-node folder trees where a single hierarchical JSON document is clearer.

## Loose files

GPT, PTX, SDA, and other level sidecar files are copied under `files/` and listed in the manifest with size and SHA-256. The OpenSpace exporter copies them to the output directory. Pointer tables inside these files are relocated by the exporter using the appropriate RT generator.

## Semantic inspection

`semantic/scene-tree.json` and `semantic/coverage.json` are analysis outputs from import. They are not export inputs.

## Import and export

### OpenSpace import

`import-openspace` reads an OpenSpace level directory, builds canonical types, segments SNA blocks into elements, resolves paths, and writes a Rete package.

### OpenSpace export

`export-openspace` reads a Rete package, lays out SNA blocks, resolves path references to virtual addresses, **generates** relocation tables, encodes payloads, and writes a complete OpenSpace level directory.

Export subsystems:

1. **Layout** — concatenate elements per block; compute block sizes and virtual bases.
2. **Pointer resolution** — reference URIs → target block and offset; write `int32` pointer values.
3. **Relocation generation** — struct pointer metadata → `.rtb`, `.rtp`, `.rtt`, and related tables.
4. **Encoding** — OpenSpace checksums; LZO compression where appropriate.

### Godot export

`export-godot` reads canonical types from a Rete package (or memory) and writes Godot scene and resource files. It does not produce relocation tables or SNA data.

### Validation

The acceptance test for the OpenSpace exporter is **byte-identical reproduction** on unedited Rete packages.

**Level export** — compare only the level's OpenSpace files (Fix is a separate package):

```bash
# rete/fix/ and rete/castle_village/ are siblings
import-openspace disc/.../castle_village rete/castle_village
export-openspace rete/castle_village output/rebuilt/castle_village
for f in disc/.../castle_village/*; do cmp -s "$f" "output/rebuilt/castle_village/$(basename "$f")"; done
```

**Fix export** — imported once into sibling `rete/fix/`, validated independently:

```bash
import-openspace disc/.../Levels rete/fix --role fix
export-openspace rete/fix output/rebuilt/fix
for f in <levels-dir>/Fix.* <levels-dir>/fixlvl.*; do
  cmp -s "$f" "<rebuilt-fix-dir>/$(basename "$f")"
done
```

No engine runtime is required. Matching bytes proves import, canonical types, layout, pointer resolution (including cross-package Fix references), relocation generation, and encoding are correct.

Edited packages have a separate smoke-test bar: intentional scalar edits produce predictable changes in rebuilt SNA data.

## AI scripts

Script-like structures with documented AST shape are stored as S-expression source files. JSON holds table metadata and references to script paths. Raw script node bytes remain as preservation buffers only where byte-perfect export requires them.

## Related documents

- Implementation guide: [`notes/rete-implementation.md`](../notes/rete-implementation.md)
- Type promotion checklist: [`notes/intermediate-type-checklist.md`](../notes/intermediate-type-checklist.md)
- OpenSpace geometry and materials: [`geometry-format.md`](geometry-format.md), [`lighting.md`](lighting.md)
- Relocation table binary layout: [`relocation-tables.md`](relocation-tables.md)
- Cross-package URIs (`fix:/`, `level:/`): [`cross-package-uris.md`](cross-package-uris.md)
- Implementation plan: [`plan.md`](../plan.md)
