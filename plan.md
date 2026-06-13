# Astrolabe Plan

Astrolabe reads *Hype: The Time Quest* OpenSpace data into **Rete** — a canonical, editable level representation — and exports to OpenSpace level directories or Godot projects. The C# core stays independent of Unity; raymap and other reference code are research sources only.

Users provide their own legally obtained game copy. Local testing uses `disc/`, especially `disc/Gamedata/World/Levels/astrolabe`.

## Architecture

```text
     fix/ ◄── relative URIs ── astrolabe/
                               │
              import    export │  export
                 ┌─────────────┼──────────────┐
                 ▼             ▼              ▼
        OpenSpace level dir  OpenSpace Fix  Godot project
```

**Rete** is the center. OpenSpace and Godot are parallel export targets consuming the same canonical types.

OpenSpace loads level and **Fix** into one virtual memory map. Fix is a **separate Rete package**. On import, the converter extracts the level and required Fix files together into one output parent (for example `output/astrolabe/` and `output/fix/`); emitted URIs such as `../fix/...` reflect that conversion-time layout, not a format constant. Level export writes level files only; Fix export is independent.

| Layer | Location | Responsibility |
|-------|----------|----------------|
| Canonical types | `FileFormats/` (enriched) | Lossless domain structs (`VisualMaterial`, …) |
| Struct codecs | `Serialization/` | Binary ↔ type ↔ JSON; pointer field metadata |
| Rete package | `Rete/` | Import/export orchestration, manifest, layout |
| OpenSpace exporter | `Rete/OpenSpace/` | SNA layout, pointer resolution, relocation generation, encoding |
| Godot exporter | `FileFormats/Godot/` | TSCN, ArrayMesh, materials from canonical types |

Read [`docs/rete-format.md`](docs/rete-format.md) for the format specification. Implementation entrypoint: [`notes/rete-implementation.md`](notes/rete-implementation.md).

## Canonical types and struct codecs

Every promoted OpenSpace struct has exactly one canonical type and one struct codec. The codec is the single source of layout truth:

- Fixed size and field order
- Read/write to bytes
- JSON serialization under a versioned schema
- Declared pointer fields for relocation generation

FileFormats readers become thin wrappers: memory or bytes → codec → canonical type, with an optional resolver pass for derived data (texture names from PTX, and so on).

Eliminate parallel `Intermediate*` DTOs. The canonical type is the JSON serialization type.

## Rete package

A Rete package is a directory of JSON structs, buffer descriptors, binary payloads, scene files, and a manifest (`astrolabe.rete.v1`). See [`docs/rete-format.md`](docs/rete-format.md).

Editorial cross-links use **reference URIs** (one line: relative path from the referring package, optional `#` JSON Pointer fragment). No parallel id layer. Virtual addresses are computed at OpenSpace export.

Dense arrays use **descriptor JSON + `.bin` buffer`**, not inline float arrays in JSON.

### Fix package

- Level import extracts level + Fix into the same output parent in one pass; `fix/` is written once and reused on subsequent level imports into that output.
- Level import merges Fix VM for resolution; emits relative URIs to Fix targets (typically `../fix/...` given default output layout).
- Exporters resolve reference URIs by relative path from the referring package root.
- `fixlvl.rtb` is generated during level export, not stored in either package.

## OpenSpace exporter

OpenSpace export is a pipeline, not a byte copy:

1. **Layout** — serialize elements in `content.json` order; assign virtual bases.
2. **Pointer resolution** — resolve path references; write `int32` values into struct pointer fields.
3. **Relocation generation** — walk pointer metadata; emit `.rtb`, `.rtp`, `.rtt`, and related files.
4. **Encoding** — OpenSpace checksums; LZO compression; reuse original encoded blobs when decompressed content is unchanged.

Relocation tables are generated, not stored in Rete.

### Byte-identical validation

The OpenSpace exporter is correct when an unedited level Rete package exports to a level directory that `cmp`s equal to the original import source, file by file. Fix Rete is validated separately. Cross-package pointer resolution must reproduce `fixlvl.rtb` and level pointer values byte-identically. No engine runtime.

Phasing for relocation generation:

1. Export with preserved relocations (bridge).
2. Export with generated relocations; `cmp` against original RT files.
3. Drop relocation storage from Rete once generation passes on test levels.

## Godot exporter

Godot export reads canonical types from a Rete package or memory. `GodotMaterialFormatter` takes `VisualMaterial`; mesh and scene exporters take geometry and scene canonical types. Godot export has its own quality bar and does not require byte-identical output.

## CLI surface (target)

```bash
astrolabe import-openspace <level-dir> [rete-dir]
astrolabe export-openspace <rete-dir> [level-dir]
astrolabe export-godot <rete-dir> [godot-dir]
```

Current commands `extract-intermediate` and `compile-intermediate` map to import/export-openspace during transition.

## Implementation phases

### Phase 1 — Serialization scaffold

- Add `Astrolabe.Core/Serialization/`: `IStructCodec<T>`, `BinaryPrimitives`, `StructCodecRegistry`.
- Add `Astrolabe.Core/Rete/`: split package orchestration out of `LevelIntermediateCodec`.
- Wire extract/compile dispatch through the registry.

### Phase 2 — Canonical type migration

Migrate promoted types in dependency order (materials → geometry headers → scene → generics). For each type:

- Merge fields into FileFormats canonical type (including unknowns).
- Implement per-type codec with pointer metadata.
- Register in `StructCodecRegistry`.
- Pass byte-identical round-trip on `astrolabe` test level.

Track progress in [`notes/intermediate-type-checklist.md`](notes/intermediate-type-checklist.md).

### Phase 3 — Paths primary

- Import resolves and writes path references alongside structs.
- Import emits relative URIs for Fix-owned records (typically `../fix/...`).
- OpenSpace export resolves all reference URIs by relative path → addresses.
- JSON pointer fields become path objects in new imports.

### Phase 4 — Relocation generator

- Implement `RelocationGenerator` for RTB from struct pointer metadata and layout.
- Add RTP/RTT generators for GPT and PTX.
- Validate: generated RT files `cmp` equal to originals on unedited imports.
- Remove relocation JSON and encoded RT leaves from Rete packages.

### Phase 5 — Exporter symmetry

- Rename CLI to `import-openspace` / `export-openspace`.
- `export-godot` accepts Rete package input.
- Godot formatters consume canonical types only.

### Phase 6 — Coverage expansion

Promote remaining documented leaves per checklist: `visualset`, element types, Perso/family, animation, AI/DSG, sectors/collision. Each promotion adds codec + pointer metadata; relocation generator coverage grows with it.

## Type promotion priorities

1. Geometry and materials (in progress)
2. Perso, families, object lists
3. Animation and state machines
4. AI, scripts, DSG
5. Sectors and collision

## Documentation

| Document | Purpose |
|----------|---------|
| [`docs/rete-format.md`](docs/rete-format.md) | Rete format specification |
| [`notes/intermediate-type-checklist.md`](notes/intermediate-type-checklist.md) | Per-type promotion checklist |
| [`docs/geometry-format.md`](docs/geometry-format.md), [`docs/lighting.md`](docs/lighting.md) | OpenSpace struct reference |
| [`docs/relocation-tables.md`](docs/relocation-tables.md) | RT binary layout for generator |

## Dependencies

- `lib/BinarySerializer.OpenSpace` — OpenSpace type definitions submodule
- `reference/raymap` — read-only reference implementation