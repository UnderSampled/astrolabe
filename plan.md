# Astrolabe Plan

Astrolabe reads *Hype: The Time Quest* OpenSpace data into **Level** and **Fix** — the in-memory canonical models — and persists or exports them as **Rete** packages, OpenSpace level directories, or Godot projects. Rete is the on-disk encoding; **Level** and **Fix** are the hubs everything loads into and exports from (separate packages, linked by `fix:/` and `level:/` URIs). The C# core stays independent of Unity; raymap and other reference code are research sources only.

Users provide their own legally obtained game copy. Local testing uses `disc/`, especially `disc/Gamedata/World/Levels/astrolabe`.

## Architecture

**Level** and **Fix** are the in-memory hubs: canonical types from `FileFormats/` (`SceneGraph`, geometry, materials, perso/family data, …). **Rete** is how each hub is stored on disk (JSON structs, buffer descriptors, binary payloads, manifest). No exporter reads Rete JSON or OpenSpace bytes directly for its own logic — everything goes through **Level** or **Fix**. Cross-package work joins the two via `fix:/` and `level:/` URI resolution (not by embedding Fix inside `Level`).

```text
   OpenSpace Fix.* ──────► Fix ◄──── Rete package (fix/, packageRole: fix)
                              ▲ fix:/  level:/
   OpenSpace level dir ──► Level ◄──── Rete package (astrolabe/, packageRole: level)
                              │
                              └── export ──┬──► OpenSpace level dir
                                           ├──► Rete package(s)
                                           └──► Godot project
```

```text
(OpenSpace level | level Rete) ──► Level.Load(...) ──► export
(OpenSpace Fix | fix Rete)     ──► Fix.Load(...)   ──► export   (Fix type: target)
```

`import-openspace` is OpenSpace level → `Level` → level Rete (and extracts Fix → fix Rete when `Fix.*` is present). `export-openspace` is level Rete → `Level` → OpenSpace level files (fix Rete exported separately). `export-godot` is (OpenSpace level | level Rete) → `Level` → Godot. Level and Fix are separate hubs; serializers and URI resolution connect them where the game does.

OpenSpace export additionally uses a **VM layout** view (`MemoryContext`) for virtual addresses, relocation generation, and LZO encoding. That is an OpenSpace-specific slice used while serializing `Level` (and joined Fix layout for `fixlvl.rtb`) to disc bytes — not a parallel pipeline and not the hub itself.

Fix is a **separate Rete package** (`packageRole: fix`) and a **separate in-memory hub**. On import, the converter extracts the level and required Fix files together into one output parent (for example `output/astrolabe/` and `output/fix/`). Cross-package pointers use **`fix:/`** and **`level:/`** package-role URIs only (see [`docs/cross-package-uris.md`](docs/cross-package-uris.md)). Level export writes level files only; Fix export is independent.

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Level** | `Level.cs` + `FileFormats/` | In-memory level model (`SceneGraph`, level geometry, materials, …) |
| **Fix** | target: `Fix.cs` + `FileFormats/` | In-memory shared Fix model (perso, families, shared textures, …); today import/export via `OpenSpacePackageCodec` |
| Struct codecs | `Serialization/` | Binary ↔ type ↔ JSON; pointer field metadata |
| Rete package | `Rete/` | Serialize/deserialize `Level` or `Fix`; manifest; layout orchestration |
| OpenSpace exporter | `Rete/OpenSpace/` | `Level` / `Fix` → SNA layout, pointer resolution, relocation generation, encoding |
| Godot exporter | `FileFormats/Godot/` | `Level` → TSCN, ArrayMesh, materials |

Read [`docs/rete-format.md`](docs/rete-format.md) for the format specification. Implementation entrypoint: [`notes/rete-implementation.md`](notes/rete-implementation.md).

## Engineering rules

**One path, no cruft, no backward compatibility.** Each concern gets exactly one implementation. When a step replaces an approach, **delete the old code** in the same step — do not leave fallbacks, feature flags, CLI aliases, “just in case” bridges, parallel pipelines, or readers for superseded formats. **Compatibility with old CLI names, manifest schemas, URI shapes, or package layouts is an anti-requirement** — remove it, do not preserve it.

| Do | Don't |
|----|--------|
| `Level` / `Fix` → export | Separate Godot/OpenSpace/Rete readers that bypass the hubs |
| One CLI name per command (`import-openspace`, …) | Aliases (`extract-intermediate`, `compile-intermediate`, …) |
| `fix:/` and `level:/` URIs only | Legacy `../fix/…` paths, numeric pointer fallbacks, old manifest schemas |
| Remove relocation bridge once generation exists (Step 5 ✓) | Keep “preserved RT*” or encoding-cache shortcuts alongside generators |
| Delete superseded types (`Intermediate*`, overlay models) when migrated | Keep dead types “for reference” in production code |

Step 6 is a **consolidation step**: wiring `Level`, **and** deleting transition shims (old commands, dual loaders, legacy import paths). Step 8 **retires** the C# core after Rust passes the Step 7 gate — not an indefinite dual codebase.

## Canonical types and struct codecs

Every promoted OpenSpace struct has exactly one canonical type and one struct codec. The codec is the single source of layout truth:

- Fixed size and field order
- Read/write to bytes
- JSON serialization under a versioned schema
- Declared pointer fields for relocation generation

FileFormats readers become thin wrappers: bytes or `Level` slices → codec → canonical type, with an optional resolver pass for derived data (texture names from PTX, and so on).

Eliminate parallel `Intermediate*` DTOs. The canonical type is the JSON serialization type.

## Rete package

A Rete package is a directory of JSON structs, buffer descriptors, binary payloads, scene files, and a manifest (`astrolabe.rete.v1`). See [`docs/rete-format.md`](docs/rete-format.md).

Editorial cross-links use **reference URIs** (one line: package-relative path, `fix:/…`, or `level:/…`; optional `#` JSON Pointer fragment). No parallel id layer. Virtual addresses are computed at OpenSpace export.

Dense arrays use **descriptor JSON + `.bin` buffer`**, not inline float arrays in JSON.

### Fix package

- Level import extracts level + Fix into the same output parent in one pass; `fix/` is written once and reused on subsequent level imports into that output.
- New imports emit `fix:/…` for level→Fix pointers and `level:/…` for Fix→level pointers.
- Exporters resolve reference URIs from the referring package root (and sibling `fix/` for `fix:/`).
- `fixlvl.rtb` is generated during level export, not stored in either package.

## OpenSpace exporter

OpenSpace export serializes **`Level`** to disc bytes. It is a pipeline, not a byte copy:

1. **Hydrate** — Rete package → `Level` (Step 6 adds the shared loader and **removes** today’s direct Rete-layout export walk).
2. **Layout** — serialize canonical elements in `content.json` order; assign virtual bases.
3. **Pointer resolution** — resolve reference URIs; write `int32` values into struct pointer fields.
4. **Relocation generation** — walk pointer metadata; emit `.rtb`, `.rtp`, `.rtt`, and related files.
5. **Encoding** — OpenSpace checksums; LZO compression from canonical decompressed blocks.

Relocation tables are generated, not stored in Rete.

### Byte-identical validation

The OpenSpace exporter is correct when an unedited level Rete package exports to a level directory that `cmp`s equal to the original import source, file by file. Fix Rete is validated separately. Cross-package pointer resolution must reproduce `fixlvl.rtb` and level pointer values byte-identically. No engine runtime. This gate is **Step 7** (after CLI and Godot land in Step 6).

## Godot exporter

Godot export reads **`Level`** (not Fix directly, not Rete JSON, and not a dedicated OpenSpace scan path). `Level.Load` accepts an OpenSpace level directory or a level Rete package; cross-package `fix:/` pointers resolve against sibling `fix/` when needed. `GodotExporter` and `GodotMeshExporter` format TSCN and ArrayMesh from canonical scene and geometry types. Godot export has its own quality bar and does not require byte-identical output.

## CLI surface (target)

```bash
astrolabe import-openspace <level-dir> [rete-dir]
astrolabe export-openspace <rete-dir> [level-dir]
astrolabe export-godot <openspace-dir | rete-dir> [godot-dir]
```

Remove `extract-intermediate`, `compile-intermediate`, and any other transition command names in Step 6 — no aliases.

## Implementation steps

The refactor proceeds as **sequential steps** on one branch — not as separate pull requests. Code map and API contracts live in [`notes/rete-implementation.md`](notes/rete-implementation.md). The **byte-identical `cmp` gate** is Step 7 only — do not block Steps 6–8 on it unless noted.

**Progress:** Steps **1–6 complete**. Step **7** is next (OpenSpace export parity / `cmp` gate). Step 5 delivered: `RelocationGenerator` (RTB/RTP/RTT/fixlvl), export generates RT* from struct codecs and opaque LUTs with no relocation bridge in Rete, **67** struct codecs, **LZO done** (`OpenSpaceLzo` + `lzo1x` at `-O0`). Remaining export parity (RTB ~145 missing pointers, RTP/fixlvl plaintext, encoding gotchas) is **Step 7**. Steps 7–8 not started.

### Step 1 — Serialization scaffold (no behavior change)

- Add `Astrolabe.Core/Serialization/`: `IStructCodec<T>`, `BinaryPrimitives`, `StructCodecRegistry`.
- Move one codec (`VisualMaterialCodec`) out of `LevelIntermediateCodec`; register it; dispatch through the registry for `visualmaterial` only.
- Build + byte-identical round-trip on `astrolabe` must pass.

### Step 2 — Migrate promoted codecs

Migrate promoted types in dependency order (materials → geometry headers → scene → generics). For each type:

- Merge fields into FileFormats canonical type (including unknowns).
- One codec file per kind; delete `Read/WriteIntermediate*` for migrated kinds.
- Register in `StructCodecRegistry`.
- Pass byte-identical round-trip on `astrolabe` test level.

Track per-type progress in [`notes/intermediate-type-checklist.md`](notes/intermediate-type-checklist.md).

### Step 3 — Split orchestrator + Rete rename

- `Intermediate/` → `Rete/`; models → `RetePackageModels.cs`.
- Emit `astrolabe.rete.v1` on new imports; add `packageRole`. (`astrolabe.level-intermediate.v1` was transitional — **removed in Step 6**, not kept for read compatibility.)

### Step 4 — Fix import layout + reference URIs

- Import writes `output/fix/` + `output/{level}/` together in one pass.
- Pointer JSON fields become reference URIs; new imports emit `fix:/…` and `level:/…`.
- OpenSpace export resolves all reference URIs by package root → virtual addresses.
- `ReferenceUri` resolver in export/import.

### Step 5 — Relocation generator ✓

**Complete.** Build the generator and export encode path; parity tuning is Step 7.

- [x] `RelocationGenerator` for RTB from struct pointer metadata and layout.
- [x] RTP/RTT generators for GPT and PTX; URI-driven `fixlvl.rtb` from Fix opaque LUT.
- [x] Export generates RT* and re-encodes SNA/RT* via `OpenSpaceLzo` from Rete only (no disc or encoding-cache reuse).
- [x] Rete packages store no relocation inventory (no `*.reloc.json`, `*-sites.json`, or encoding cache).
- [x] LZO compression via vendored `lzo1x` (`-O0`); Layer A recompression matches disc on `astrolabe` `.rtp`, `.rtt`, and `fixlvl.rtb`.

**Not Step 5:** byte-identical full-level `cmp`, closing the last RTB pointer gaps, or encoding gotchas — those are Step 7 and can proceed in parallel with Step 6.

### Step 6 — CLI, Level, and Godot ✓

**Complete.** `Level` is the level hub; Fix remains a separate package/hub (import/export via `OpenSpacePackageCodec` today; dedicated `Fix` type is a later target).

**6a — CLI surface**

- [x] CLI commands are `import-openspace`, `export-openspace`, `export-godot` only.
- [x] **Delete** `extract-intermediate`, `compile-intermediate`, and all other transition names — no aliases, no compatibility shims.
- [x] **Delete** acceptance of `astrolabe.level-intermediate.v1`, `../fix/…` URIs, and any other legacy package/URI/command paths added during the refactor.

**6b — Level + Godot**

- [x] Add `Level.Load(openspaceDir | retePackageDir)` — hydrate level canonical types; resolve `fix:/` against sibling `fix/` for cross-package work.
- [x] Route `import-openspace`, `export-openspace`, and `export-godot` through `Level` only.
- [x] **Remove:** OpenSpace-only Godot pipeline (`ExportGodotCommand`’s direct `LevelLoader`/`MeshScanner` path), exporter-specific Rete JSON walkers, and other dead consolidation paths.
- [x] `export-godot` accepts **OpenSpace or Rete** input via `Level.Load`; Godot formatters read `Level` only.

### Step 7 — OpenSpace export parity (completion gate)

Finish what was deferred from the end of Step 5. LZO plumbing and RT* recompression are largely done (`lzo1x`); remaining work is **generated content parity** plus a few encoding gotchas.

**Primary work**

- Generated RT pointer **plaintext** `cmp` equal to originals on unedited `astrolabe` (and Fix validated separately).
- Close remaining RTB/RTP/fixlvl pointer gaps (~145 missing on `astrolabe.rtb`; RTP/fixlvl pointer-data diffs).
- Decide `Fix.rtv` support.

**Gotchas to look into**

- **`astrolabe.rtb` `05:01` LZO tail** — recompressing **disc plaintext** through `lzo1x` matches for 159,645/159,649 bytes; **4 bytes** diverge at offset `0x1D1A5` (alternate valid LZO1X encoding, not a decompress failure). Investigate whether the original Montreal encoder used different minilzo state or whether this block needs a special-case match. See `tools/LzoDiffProbe` and `OpenSpaceEncodingFidelityTests` Layer A.
- **SNA recompression** — some `astrolabe.sna` blocks (`05:01`, `06:02`, `11:01`) still differ from disc compressed blobs; separate from the `.rtb` tail above.

**Exit criterion:** unedited Rete → `export-openspace` → `cmp` every file in the source level directory.

### Step 8 — Rust + binrw port

Port the C# core to Rust with **binrw** for struct codecs, after Step 7 passes on `astrolabe` in C# (C# remains the oracle until Rust matches the same `cmp` suite).

Suggested order:

1. **binrw codecs** — 1:1 with `StructCodecRegistry`; golden tests against C# fixtures.
2. **Rete package I/O** — manifest, `content.json`, URI resolver (`fix:/`, `level:/`).
3. **`Level` loaders** — OpenSpace and Rete hydrate into the same Rust `Level` type.
4. **OpenSpace pipeline** — layout, relocation generation, LZO encode (port `OpenSpaceLzo` / `lzo1x` integration).
5. **CLI** — Rust binary becomes the only implementation; **remove** C# CLI/Core once Rust passes Step 7.

Godot export ports with Step 8 or immediately after — do not maintain two Godot exporters. No long-lived C#/Rust dual core.

### Coverage expansion (parallel)

Promote remaining documented leaves per checklist: `visualset`, element types, Perso/family, animation, AI/DSG, sectors/collision. Each promotion adds codec + pointer metadata; relocation generator coverage grows with it. Continues alongside Steps 5–8 as types are ready — does not block Step 6.

## Type promotion priorities

1. Geometry and materials — **mostly done** on `astrolabe`; collision-geometry leaves (`sectorcollide*`, `collidez*`) remain opaque
2. Perso, families, object lists — **partial** (`perso`, `perso3ddata`, `standardgame`, `objectlist`, `spawnableentry` promoted; `objecttype*` and `alwayssuperobjects` still opaque)
3. Animation and state machines — **partial** (`state`, `transition`, `animationmontreal`, `animchannel`, `animframes`, `animhierarchiesheader` promoted; `actiontable`, `actiontree`, `animhierarchies`, `compressedmatrix` still opaque)
4. AI, scripts, DSG — **partial** (`brain`, `mind`, `intelligence`, `aimodel` promoted; behavior lists, `script`, `dsgvar`/`dsgmem` still opaque)
5. Sectors and collision — **partial** (`sector`, `collideset`, `persosectorinfo` promoted; `sectorname`, `sectorcollide*`, `collidez*` still opaque)

## Documentation

| Document | Purpose |
|----------|---------|
| [`docs/rete-format.md`](docs/rete-format.md) | Rete format specification |
| [`notes/intermediate-type-checklist.md`](notes/intermediate-type-checklist.md) | Per-type promotion checklist |
| [`docs/cross-package-uris.md`](docs/cross-package-uris.md) | `fix:/` and `level:/` URI spec |
| [`docs/geometry-format.md`](docs/geometry-format.md), [`docs/lighting.md`](docs/lighting.md) | OpenSpace struct reference |
| [`docs/relocation-tables.md`](docs/relocation-tables.md) | RT binary layout for generator |

## Dependencies

- `lib/BinarySerializer.OpenSpace` — OpenSpace type definitions submodule
- `reference/raymap` — read-only reference implementation