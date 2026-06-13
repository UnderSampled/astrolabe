# Rete Implementation Guide

Cold-start entrypoint for the Rete refactor. Read in order:

1. [`plan.md`](../plan.md) — architecture and implementation steps
2. [`docs/rete-format.md`](../docs/rete-format.md) — format specification
3. This file — code map, APIs, step boundaries
4. [`intermediate-type-checklist.md`](intermediate-type-checklist.md) — per-type promotion

## Goal

Rete is the canonical level representation. OpenSpace level dirs and Godot projects are **export targets**. Canonical types + struct codecs are shared by import, both exporters, and JSON serialization.

**Acceptance test:** unedited Rete → OpenSpace export → `cmp` every file in the source level directory. No engine runtime.

## Current code (transition)

| Today | Role | Target |
|-------|------|--------|
| `src/Astrolabe.Core/Intermediate/LevelIntermediateCodec.cs` | Import/export monolith (~1.5k lines) | Split into `Rete/` + `Serialization/` |
| `src/Astrolabe.Core/Intermediate/LevelIntermediateModels.cs` | Parallel DTOs (`Intermediate*`) | Merge into `FileFormats/` canonical types |
| `src/Astrolabe.Core/Intermediate/OpenSpaceChecksum.cs` | Checksum helper | `Rete/OpenSpace/` or `FileFormats/` |
| `extract-intermediate` / `compile-intermediate` CLI | Works today | Aliases → `import-openspace` / `export-openspace` |
| `FileFormats/*Reader` | Read-only, drops unknowns | Thin wrappers over struct codecs |
| `FileFormats/Godot/*` | Export from memory scan | Also consume canonical types / Rete URIs |

Manifest schema today: `astrolabe.level-intermediate.v1`. Target: `astrolabe.rete.v1` (accept both during transition). Struct schemas (`astrolabe.visual-material.v1`, etc.) stay unchanged.

Relocation tables are still preserved on import/export today. Target: generated at OpenSpace export only; not stored in Rete.

## Target layout

```text
src/Astrolabe.Core/
  Serialization/
    IStructCodec.cs
    StructCodecRegistry.cs
    BinaryPrimitives.cs
    Codecs/
      VisualMaterialCodec.cs
      GameMaterialCodec.cs
      ...
  Rete/
    OpenSpaceImporter.cs      # was ExtractLevel
    OpenSpaceExporter.cs      # was CompileLevel
    SnaBlockSegmenter.cs
    ReferenceUri.cs           # parse/resolve relative URIs + #fragment
    RetePackageModels.cs      # manifest models
    OpenSpace/
      OpenSpaceChecksum.cs
      RelocationGenerator.cs  # phase 4
  FileFormats/
    Materials/VisualMaterial.cs   # lossless canonical type
    ...
```

## `IStructCodec<T>` contract

```csharp
public interface IStructCodec<T>
{
    string Kind { get; }           // "visualmaterial" — matches tracker NormalizeKind
    string Schema { get; }          // "astrolabe.visual-material.v1"
    int? FixedSize { get; }         // 0x78, or null for variable-length kinds

    T Read(ReadOnlySpan<byte> data, int offset, int length);
    byte[] Write(T value);

    T FromJson(JsonElement json);
    void ToJson(T value, Utf8JsonWriter writer);

    IReadOnlyList<PointerField> PointerFields { get; }
}

public readonly record struct PointerField(
    int Offset,           // byte offset in serialized struct
    string Name,          // JSON field name (holds a reference URI string)
    PointerTarget Target  // block-relative | fix | any — for relocation generator
);
```

Variable-length kinds (`uint32-record`, `float3-array`, opaque blobs) use `FixedSize = null`; `length` comes from the element span in `content.json`.

Registry drives import element extract and export element serialize — **no growing switch** in the orchestrator.

## Reference URIs

- One string per pointer field on canonical types.
- Resolved from the **referring package root** (relative path + optional `#` RFC 6901 fragment).
- Import emits `../fix/...` when the converter writes Fix at `output/fix/` and the level at `output/{level}/` in the same pass.
- Resolver: `Path.GetFullPath(Path.Combine(packageRoot, relativePath))` after splitting `#`.

```csharp
// ReferenceUri.cs
public static bool TryResolve(string packageRoot, string uri, out string filePath, out string? jsonPointer);
```

## Import behavior (Fix)

Single `import-openspace <level-dir> <output-parent>` pass:

1. Ensure `<output-parent>/fix/` exists; import Fix from `Gamedata/World/Levels/Fix.*` if missing or `--refresh-fix`.
2. Import level into `<output-parent>/{levelName}/`.
3. Merge Fix + level VM for pointer resolution during segmentation.
4. Write only level-owned elements into the level package.
5. Emit reference URIs to Fix targets as `../fix/...` relative to the level package.

Do not duplicate Fix elements into level packages. Subsequent level imports into the same output parent reuse `fix/`.

## OpenSpace export pipeline

1. Load level Rete package (+ resolve `../fix/` URIs against sibling `fix/`).
2. Serialize `content.json` elements in order → decompressed SNA blocks.
3. Resolve reference URIs → virtual addresses; write pointer `int32`s into structs.
4. **RelocationGenerator** → `.rtb`, `fixlvl.rtb`, `.rtp`, `.rtt`, … (phase 4).
5. Encode SNA; copy `files/` sidecars.

Until step 4 is done, keep reading preserved RT* from import as a bridge (current behavior).

## Promoted types (migrate to codecs first)

| Kind | Schema | Size | Canonical type home |
|------|--------|------|---------------------|
| `superObject` | `astrolabe.super-object.v1` | 0x38 | new or `SceneGraph` |
| `matrix` | `astrolabe.matrix.v1` | 88 | `SceneGraph` / matrices |
| `geometricobject` | `astrolabe.geometric-object.v1` | 0x40 | `FileFormats/Geometry/` |
| `physicalobject` | `astrolabe.physical-object.v1` | 0x10 | `FileFormats/Geometry/` |
| `ipo` | `astrolabe.ipo.v1` | 8 | `FileFormats/Geometry/` |
| `gamematerial` | `astrolabe.game-material.v1` | 0x10 | `FileFormats/Materials/GameMaterial.cs` |
| `visualmaterial` | `astrolabe.visual-material.v1` | 0x78 | `FileFormats/Materials/VisualMaterial.cs` |
| `boundingvolume` / `collidematerial` | `astrolabe.uint32-record.v1` | variable | generic codec |
| `vertices` / `normals` / `trianglenormals` | `astrolabe.float3-array.v1` | variable | → descriptor + `.bin` later |

Merge `Intermediate*` fields into canonical types (including unknown bytes). `VisualMaterialReader` delegates to `VisualMaterialCodec`.

## Implementation steps

Sequential work units are defined in [`plan.md`](../plan.md#implementation-steps). This file adds code-level detail per step:

| Step | Focus here |
|------|------------|
| 1 | `IStructCodec<T>`, `StructCodecRegistry`, first `VisualMaterialCodec` |
| 2 | Promoted types table below; delete `Intermediate*` DTOs as kinds migrate |
| 3 | Target layout (`Rete/`, `RetePackageModels.cs`); manifest schema transition |
| 4 | `ReferenceUri.cs`; Fix output layout; pointer fields as URI strings |
| 5 | `RelocationGenerator`; drop preserved RT* from packages |
| 6 | CLI aliases; Godot export from Rete |
| 7 | Checklist backlog — per-type codec + pointer metadata |

## Verification (every step)

```bash
dotnet build
dotnet run --project src/Astrolabe.Cli -- extract-intermediate \
  disc/Gamedata/World/Levels/astrolabe output/test-rete/astrolabe
dotnet run --project src/Astrolabe.Cli -- compile-intermediate \
  output/test-rete/astrolabe output/test-rete/rebuilt-astrolabe
for f in disc/Gamedata/World/Levels/astrolabe/*; do
  name=${f##*/}
  cmp -s "$f" "output/test-rete/rebuilt-astrolabe/$name" || printf 'DIFF %s\n' "$name"
done
```

## Out of scope until later

- Aggregate JSON docs (`geometry/meshes.json`) — per-file leaves OK for now
- S-expression AI promotion
- Full tracker label promotion (~60 opaque kinds)
- Godot material field completeness

## Key references

- RT binary layout: [`docs/relocation-tables.md`](../docs/relocation-tables.md)
- Raymap relocation: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/RelocationTable.cs`
- Segmentation: `LevelIntermediateCodec.BuildContentPlans` + `TrackingSuperObjectReader`
- Fix on disc: `Gamedata/World/Levels/Fix.sna`, `fixlvl.rtb`