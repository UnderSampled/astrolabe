# Rete Implementation Guide

Cold-start entrypoint for the Rete refactor. Read in order:

1. [`plan.md`](../plan.md) — architecture and implementation steps
2. [`docs/rete-format.md`](../docs/rete-format.md) — format specification
3. This file — code map, APIs, step boundaries
4. [`intermediate-type-checklist.md`](intermediate-type-checklist.md) — per-type promotion

## Goal

**Level** and **Fix** are the in-memory hubs; **Rete** is the on-disk encoding (separate level and fix packages). OpenSpace level dirs and Godot projects are **export targets**. Canonical types + struct codecs are shared by import, both exporters, and JSON serialization.

**Acceptance test:** unedited Rete → OpenSpace export → `cmp` every file in the source level directory. No engine runtime.

## Current code (transition, Step 5 in progress)

| Current | Role | Remaining target |
|---------|------|------------------|
| `src/Astrolabe.Core/Rete/OpenSpacePackageCodec.cs` | Import/export bridge for Rete packages, sibling Fix import, URI rewrite/export resolution, transient Fix `fixlvl` opaque LUT annotation (`level:/slots/…`, no persisted site registry) | Split further as pipeline pieces mature |
| `src/Astrolabe.Core/Rete/ReferenceUri.cs` | Relative URI parse/resolve helper | Keep as shared URI primitive |
| `src/Astrolabe.Core/Rete/ReferenceAddressResolver.cs` | Builds package address indexes from `content.json`; resolves URI ↔ virtual address | Feed relocation generation/layout |
| `src/Astrolabe.Core/Rete/ReferenceJson.cs` | Rewrites promoted JSON pointer fields to URI/null on import; `WriteElementBytesForExport` resolves URIs before serialization | Remove numeric fallback once all pointer fields are URI-backed |
| `src/Astrolabe.Core/Rete/RetePackageModels.cs` | Manifest/content models; content elements include offsets, lengths, virtual addresses, and imported Fix→level site metadata | Drop preserved relocation models after Step 5 |
| `src/Astrolabe.Core/Rete/OpenSpace/RelocationGenerator.cs` | Generates diagnostic RTB/fixlvl subsets from promoted struct pointer metadata, imported `fixlvl` site metadata, and GPT/PTX pointer-file tables; compares generated data with preserved RT data | Expand RTB coverage to remaining opaque promoted types; then replace preserved relocation export |
| `src/Astrolabe.Core/Serialization/Codecs/*` | 35 registered codecs (structs, pointer arrays, dense arrays) | Keep expanding per checklist; ~20k RTB pointers still opaque |
| `import-openspace` / `export-openspace` CLI | Level Rete import/export via `Level` | Done (Step 6) |
| `Fix` in-memory hub | Fix import/export still in `OpenSpacePackageCodec` | Add `Fix.cs` hub type (post–Step 6) |
| `debug-relocations` CLI | Compares generated relocation diagnostics against preserved relocation tables | Remove once generated RT output is the normal exporter path |
| `FileFormats/*Reader` | Read-only scanners/readers used for semantic discovery | Thin wrappers over struct codecs where practical |
| `FileFormats/Godot/*` | Export from memory scan | Also consume canonical types / Rete URIs |

New imports emit manifest schema `astrolabe.rete.v1` and accept both `astrolabe.rete.v1` and legacy `astrolabe.level-intermediate.v1` during transition. Struct schemas (`astrolabe.visual-material.v1`, etc.) stay unchanged.

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
    ReferenceAddressResolver.cs
    ReferenceJson.cs
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
- Cross-package URIs: `fix:/` (level→Fix) and `level:/` (Fix→level). Spec: [`docs/cross-package-uris.md`](../docs/cross-package-uris.md). Resolver done; import slot assignment not started. Legacy `../fix/...` still emitted.
- Resolver: `Path.GetFullPath(Path.Combine(packageRoot, relativePath))` after splitting `#`.
- Current Step 4 bridge rewrites promoted pointer fields to URI strings when the raw address maps to a package content element. `0` becomes `null`. Unresolved sentinel values and unpromoted pointer-like fields remain numeric to preserve byte-identical export.

```csharp
// ReferenceUri.cs
public static bool TryResolve(string packageRoot, string uri, out string filePath, out string? jsonPointer);
```

## Import behavior (Fix)

Current transition command: `extract-intermediate <level-dir> <level-package-dir>`.

When `Fix.*` exists in the parent `Gamedata/World/Levels/` directory:

1. Import/reuse sibling `<output-parent>/fix/`, where `<output-parent>` is the parent of `<level-package-dir>`.
2. Import the level into `<level-package-dir>`.
3. Build address indexes for the level and Fix packages from their `content.json` element offsets/virtual addresses.
4. Rewrite promoted pointer JSON fields to relative URIs (`types/...`, `scene/...`, or `../fix/...` when resolvable).

Do not duplicate Fix elements into level packages. Subsequent level imports into the same output parent reuse `fix/`. There is not yet a `--refresh-fix` option.

## OpenSpace export pipeline

1. Load level Rete package (+ resolve `../fix/` URIs against sibling `fix/`).
2. Serialize `content.json` elements in order → decompressed SNA blocks.
3. Resolve reference URIs → virtual addresses; write pointer `int32`s into structs.
4. **RelocationGenerator** → `.rtb`, `fixlvl.rtb`, `.rtp`, `.rtt`, … (phase 4).
5. Encode SNA; copy `files/` sidecars.

Until Step 5 is done, keep reading preserved RT* from import as a bridge (current behavior).

## Promoted types (migrate to codecs first)

| Kind | Schema | Size | Canonical type home |
|------|--------|------|---------------------|
| `superObject` | `astrolabe.super-object.v1` | 0x38 | new or `SceneGraph` |
| `matrix` | `astrolabe.matrix.v1` | 88 | `SceneGraph` / matrices |
| `geometricobject` | `astrolabe.geometric-object.v1` | 0x40 | `FileFormats/Geometry/` |
| `physicalobject` | `astrolabe.physical-object.v1` | 0x10 | `FileFormats/Geometry/` |
| `ipo` | `astrolabe.ipo.v1` | 8 | `FileFormats/Geometry/` |
| `visualset` | `astrolabe.visual-set.v1` | 0x10 | `FileFormats/Geometry/` |
| `elementtriangles` | `astrolabe.element-triangles.v1` | 0x28 | `FileFormats/Geometry/` |
| `radiosityheader` | `astrolabe.radiosity-header.v1` | 0x10 | `FileFormats/Geometry/` |
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
| 6 | CLI rename; `Level` hub; Godot from Rete; **Fix** remains separate hub |
| 7 | Checklist backlog — per-type codec + pointer metadata |

Progress as of 2026-06-14 (commits `5ca7ef4`, `137265d`):

- Steps 1–4 are complete on `astrolabe`, preserving byte-identical OpenSpace export.
- Level import creates/reuses sibling `fix/`; Fix export validates independently against `Fix.*` plus `fix.cnt`.
- Promoted structured pointer fields are URI/null on import and resolved back to virtual addresses on export via `ReferenceJson.WriteElementBytesForExport`.
- Relocation JSON and preserved encoded RT payloads remain in packages as the bridge for Step 5.
- Fresh level imports annotate Fix opaque LUT entries from transient disc `fixlvl.rtb`: mapped rows get `level:/slots/0x{fixSite}.json` plus per-level slot files; sentinel rows get `null` URI. No fixlvl site inventory is persisted on the Fix package.
- **35 struct codecs** registered: original ten structured kinds plus pointer arrays (`elementptrs`, `loddataoffsets`, `animchannelptrs`, `scriptptrs`, `dsgvarptrindirect`, `collideelementptrs`), dense arrays (`elementtypes`, `triangles`, `vertexindices`, `uvs`, `uvmapping`, `loddistances`), and promoted leaves (`animchannel`, `elementsprites`, `animationmontreal`, `animframes`, `animhierarchiesheader`, `perso`, `perso3ddata`, `brain`, `state`, `transition`, `standardgame`, `objectlist`, `spawnableentry`, `mind`, `intelligence`, `aimodel`, `sector`, `collideset`, `persosectorinfo`).
- `debug-relocations` on `astrolabe` (after fresh `extract-intermediate`):
  - `astrolabe.rtb`: **68,932** generated / **68,932** matching / **0** extra (69,922 preserved; **990 missing**, ~98.6% coverage)
  - `astrolabe.rtp`: 86/86 matching, **0 missing**, **0 extra**, `pointer data: diff`
  - `astrolabe.rtt`: 125/125 matching, `pointer data: match`
  - `fixlvl.rtb`: **1,117** generated / **1,117** matching / **0 missing** / **0 extra**, `pointer data: match`
- `debug-relocations` on sibling `fix/`:
  - `Fix.rtb`: **492,237** generated / **492,180** matching / **57 extra** (**493,305 preserved; 1,125 missing**), `pointer data: diff`
  - `Fix.rtp`: **790** generated / **789** matching / **1 extra**, `pointer data: diff`
  - `Fix.rtt`: 435/435 matching, `pointer data: match`
  - `Fix.rtv`: unsupported placeholder table
- RTP/RTT generation is complete only for the PTX sidecars at byte parity; GPT-side `.rtp` output still has byte/count drift on both level and Fix.
- On `astrolabe`, transient fixlvl import annotates **16 Fix opaque elements** with **1,117 LUT keys** (**111 mapped `level:/…`**, **1,006 `null` sentinel**).
- Cross-Fix `fix:/...` and Fix→level `level:/...` URIs now appear in imported JSON, but most remaining RTB gaps are still inside opaque leaves and not yet promoted into explicit pointer metadata.

`animchannel` pointer semantics (Montreal):

- `isIdentity` (bytes `0x00–0x03`): sentinel `0` / `1` or a compressed-matrix virtual address. Import/export accept legacy JSON key `matrixPointer` (numeric sentinels and URI strings via `IPointerFieldAliases`).
- `unknown10` (bytes `0x10–0x13`): polymorphic — small inline integers (e.g. `3`, `17`) or a virtual-address pointer rewritten to a URI. Relocation generation skips non-VM-range values via `IsLikelyVirtualAddress` (`0x08000000–0x0FFFFFFF`).

`PointerTarget` is honored in `RelocationGenerator.FindTargetBlock`: `BlockRelative` searches only the source package, `Fix` only Fix packages, `Any` searches all loaded layouts.

Next agent should continue Step 5 with the existing bridge intact:

- Keep the `fixlvl` path URI-driven. Do not reintroduce the broad Fix relocation scan or any `*-sites.json` registry. `GenerateFixLevelRtb` reads Fix opaque LUT only (`level:/` mapped rows; `null`/escaping sentinel rows).
- Close the remaining main-generator parity gaps:
  - `astrolabe.rtb`: resolve the last **990 missing** pointers without introducing extras.
  - `Fix.rtb`: eliminate **1,125 missing** / **57 extra**.
  - `astrolabe.rtp` / `Fix.rtp`: fix pointer-data/count parity.
  - decide whether `Fix.rtv` stays unsupported or needs an explicit generator/bridge rule.
- Promote remaining opaque leaves (`actiontable`, `actiontree`, `animhierarchies`, `compressedmatrix`, behavior lists, `script`, `dsgvar`/`dsgmem`, `objecttype*`, `sectorcollide*`, `collidez*`) in the order that best attacks the outstanding RTB mismatches.
- Re-run `extract-intermediate` before judging `fixlvl` on any package created before 2026-06-14; old imports may still carry legacy `semantic/fixlvl-sites.json` (pruned on re-import).
- Re-run `import-openspace` on level packages that list `fixlvl.rtb` but omit `manifest.FixlvlBlockKeys` (e.g. stale `output/test-rete/astrolabe`). Without re-import, export/compare skips disc empty blocks such as `07:00` / `13:01`.
- Re-run `extract-intermediate` on packages imported before 2026-06-14 if they still have inline `pointers`/`targets` on promoted JSON elements (for example scene `node.json`) without matching `*.reloc.json` sidecars. Export and relocation generation only read overlay data from sidecars now; pre-sidecar packages must be re-imported (or manually migrated) before Step 5 parity checks are meaningful.
- Phase out relocation storage only after generated RT files `cmp` against preserved originals. Step 6 (CLI aliases, Godot-from-Rete) still waits on RT parity.

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
