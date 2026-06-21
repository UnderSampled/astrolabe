# Astrolabe Plan

Astrolabe reads *Hype: The Time Quest* OpenSpace data into **Level** and **Fix** — the in-memory canonical models — and persists or exports them as **Rete** packages, OpenSpace level directories, or Godot projects. Rete is the on-disk encoding; **Level** and **Fix** are the hubs everything loads into and exports from (separate packages, linked by `fix:/` and `level:/` URIs). The C# core stays independent of Unity; raymap and other reference code are research sources only.

Users provide their own legally obtained game copy. Local testing uses `disc/`, especially `disc/Gamedata/World/Levels/astrolabe`.

## Architecture

**Level** and **Fix** are the in-memory hubs: canonical types from `FileFormats/` (`SceneGraph`, geometry, materials, perso/family data, …). **Rete** is how each hub is stored on disk (JSON structs, buffer descriptors, binary payloads, manifest). No exporter reads Rete JSON or OpenSpace bytes directly for its own logic — everything goes through **Level** or **Fix**. Cross-package work joins the two via `fix:/` and `level:/` URI resolution (not by embedding Fix inside `Level`).

**Lazy hub, not a full preload.** `Level.Load` / `Fix.Load` build a **catalog index** (manifest + element metadata + `HubReference` URIs) and eagerly load only what the entry path needs immediately (e.g. scene graph roots). Promoted canonical records are **hydrated on demand** via `HubCatalog.TryHydrate` when a consumer follows a reference (mesh scan, Godot export, export-time codec walks). The hub is not “everything parsed into memory at load time”; do not reintroduce bulk preload of all `types/` elements as the default load path.

```text
   OpenSpace Fix.* ──────► Fix ◄──── Rete package (Gamedata/World/Levels/, packageRole: fix)
                              ▲ fix:/  level:/
   OpenSpace level dir ──► Level ◄──── Rete package (Gamedata/World/Levels/{level}/, packageRole: level)
                              │
                              └── export ──┬──► OpenSpace level dir (same mirrored paths)
                                           ├──► Rete package(s)
                                           └──► Godot project
```

```text
(OpenSpace level | level Rete) ──► Level.Load(...) ──► export
(OpenSpace Fix | fix Rete)     ──► Fix.Load(...)   ──► export   (Fix type: target)
```

`import-openspace` is OpenSpace level → `Level` → level Rete (and extracts Fix → fix Rete when `Fix.*` is present). `export-openspace` is level Rete → `Level` → OpenSpace level files (fix Rete exported separately). `export-godot` is (OpenSpace level | level Rete) → `Level` → Godot. Level and Fix are separate hubs; serializers and URI resolution connect them where the game does.

OpenSpace export additionally uses a **VM layout** view (`MemoryContext`) for virtual addresses, relocation generation, and LZO encoding. That is an OpenSpace-specific slice used **only** while serializing `Level` (and joined Fix layout for `fixlvl.rtb`) to disc bytes — not during `Level.Load`, not a parallel pipeline, and not the hub itself. Today `HydrateFromRetePackage` still rebuilds SNA + RT* for VM pointer chasing; **Step 8** removes that transitional path.

Fix is a **separate Rete package** (`packageRole: fix`) and a **separate in-memory hub**. **Conversion output mirrors the game layout** under one output root (same paths as the mounted disc: `Gamedata/World/Levels/{level}/`, `Gamedata/Textures/`, …). Fix Rete lives at `Gamedata/World/Levels/` (import collects uppercase `Fix.*` on disc); each level Rete lives in its level subdirectory. Cross-package pointers use **`fix:/`** and **`level:/`** package-role URIs (see [`docs/cross-package-uris.md`](docs/cross-package-uris.md)) — not a flat `output/fix/` + `output/{level}/` tree. Level export writes level files only; Fix export is independent.

Today's import still writes a transitional flat layout (`output/fix/`, `output/astrolabe/`); migrating to the mirrored layout is part of tightening import/output (Step 9 asset tree work or earlier resolver pass).

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Level** | `Level.cs` + `Hub/` + `FileFormats/` | In-memory level hub: **lazy** catalog + `HubReference` links; canonical records hydrate on demand (`SceneGraph`, geometry, materials, …) |
| **Fix** | `Fix.cs` + `Hub/` + `FileFormats/` | In-memory Fix hub (same lazy pattern: catalog index + on-demand hydration) |
| Struct codecs | `Serialization/` | Wire layout (bytes ↔ export); pointer field metadata for relocation generation at export only |
| Rete package | `Rete/` | Persist `Level` / `Fix` as JSON + buffers; cross-links as `fix:/` / `level:/` URIs |
| OpenSpace exporter | `Rete/OpenSpace/` | `Level` / `Fix` → VM layout, `int32` pointer values, relocation generation, encoding (**Step 8** consolidates here) |
| Godot exporter | `FileFormats/Godot/` | `Level` → TSCN, ArrayMesh, materials |

Read [`docs/rete-format.md`](docs/rete-format.md) for the format specification. Implementation entrypoint: [`notes/rete-implementation.md`](notes/rete-implementation.md).

## Engineering rules

**One path, no cruft, no backward compatibility.** Each concern gets exactly one implementation. When a step replaces an approach, **delete the old code** in the same step — do not leave fallbacks, feature flags, CLI aliases, “just in case” bridges, parallel pipelines, or readers for superseded formats. **Compatibility with old CLI names, manifest schemas, URI shapes, or package layouts is an anti-requirement** — remove it, do not preserve it.

| Do | Don't |
|----|--------|
| `Level` / `Fix` → export | Separate Godot/OpenSpace/Rete readers that bypass the hubs |
| One CLI name per command (`import-openspace`, …) | Aliases (`extract-intermediate`, `compile-intermediate`, …) |
| Role URIs on disk (`fix:/`, `level:/`, `texture:/`, `sound:/`, …); object references in hub | Legacy `../fix/…` paths, `int` VM pointers in canonical types, opaque `files/` pass-through, per-level/per-fix texture ownership |
| VM layout + RT* generation at export only (Step 8) | Regenerating RT* in `Level.Load` / hydration |
| Remove relocation bridge once generation exists (Step 5 ✓) | Keep “preserved RT*” or encoding-cache shortcuts alongside generators |
| Delete superseded types (`Intermediate*`, overlay models) when migrated | Keep dead types “for reference” in production code |
| Promote documented structs to canonical types + codecs when code must parse or traverse them | Use `raw` (`OpaqueBinaryRecord`) blobs for understood layout — `raw` is import/export preservation only |
| Hub readers, Godot export, mesh scan, relocation walks on **promoted** kinds with `HubReference` fields | Heuristic scans of `types/raw/` pointer LUTs to recover geometry, materials, or other known structs |

**`raw` is a placeholder, not a parser.** The `raw` codec kind exists for wire blobs whose layout is **not yet promoted** — byte-identical round-trip through import/export, optional inline pointer LUT for relocation. If any step needs to **understand** the data (field access, nested pointers, mesh/material resolution, scene semantics, Godot export, hub lazy-load), **promote the struct first** per [`notes/intermediate-type-checklist.md`](notes/intermediate-type-checklist.md). Do not leave known OpenSpace types in `types/raw/` and decode them with offset heuristics or fragment hacks; that duplicates the codec layer and blocks lazy `HubReference` resolution.

Step 6 is a **consolidation step**: wiring `Level`, **and** deleting transition shims (old commands, dual loaders, legacy import paths). The C# core remains the sole implementation.

## Canonical types and struct codecs

Every promoted OpenSpace struct has exactly one **hub canonical type** in `FileFormats/` and one struct codec for **wire layout**. They are not the same representation:

| Representation | Pointer fields | Where |
|----------------|----------------|-------|
| **Hub** (`Level` / `Fix`) | Object references — indices, handles, or direct links to owned elements | In memory after import or `Level.Load` |
| **Rete on disk** | Reference URIs (`types/…`, `fix:/…`, `level:/…`) | JSON in the package |
| **OpenSpace wire** | `int32` virtual addresses | Bytes at export only |

The codec is the single source of **wire** layout truth:

- Fixed size and field order
- Read/write to bytes (import from OpenSpace; export to OpenSpace)
- Declared pointer field metadata for relocation generation (**export only**)
- Rete JSON uses URIs for pointer slots, not VM integers

FileFormats readers become thin wrappers: hub types ↔ wire bytes at import/export boundaries, with an optional resolver pass for derived data (texture names from PTX, and so on).

Eliminate parallel `Intermediate*` DTOs. Hub canonical types replace records that still carry raw `int` pointer fields.

## Rete package

A Rete package is a directory of JSON structs, buffer descriptors, binary payloads, scene files, and a manifest (`astrolabe.rete.v1`). See [`docs/rete-format.md`](docs/rete-format.md).

Editorial cross-links use **reference URIs** (one line: package-relative path, `fix:/…`, or `level:/…`; optional `#` JSON Pointer fragment). No parallel id layer. Virtual addresses are computed at OpenSpace export.

Dense arrays use **descriptor JSON + `.bin` buffer`**, not inline float arrays in JSON.

### Fix package

- Level import extracts level + Fix into the same output parent in one pass; `fix/` is written once and reused on subsequent level imports into that output.
- New imports emit `fix:/…` for level→Fix pointers and `level:/…` for Fix→level pointers.
- Exporters resolve reference URIs from the referring package root (and sibling `fix/` for `fix:/`).
- `fixlvl.rtb` is generated during level export, not stored in either package.
- GPT, PTX, SDA, SND, and related sidecars are **not** long-term opaque `files/` leaves — **Step 9** promotes them to URI-backed Rete types with generated RT* (today they pass through unchanged; RTP/RTT use heuristic scans).

## OpenSpace exporter

OpenSpace export serializes **`Level`** to disc bytes. It is a pipeline, not a byte copy. **VM layout, `int32` pointer values, and relocation generation happen only here** (Step 8 removes them from `Level.Load` / hydration).

1. **Load hub** — `Level.Load(retePackageDir | openspaceDir)` yields live canonical types with object references (no RT* regeneration on load).
2. **Layout** — walk hub elements in export order; assign virtual bases.
3. **Pointer materialization** — resolve hub references → `int32` VM addresses; write into struct pointer fields.
4. **Relocation generation** — walk codec pointer metadata on laid-out bytes; emit `.rtb`, `.rtp`, `.rtt`, and related files.
5. **Encoding** — OpenSpace checksums; LZO compression from decompressed blocks.

Relocation tables are generated at export, not stored in Rete, and not rebuilt during hub load.

### Export validation

The OpenSpace exporter is correct when an unedited level Rete package round-trips to a level directory whose **decompressed content** matches the original import source. Fix Rete is validated separately. Cross-package pointer resolution must reproduce `fixlvl.rtb` and level pointer values in decompressed plaintext. No engine runtime. This **decompressed parity gate** is **Step 7** (complete).

**Compressed-byte `cmp`** (LZO container bytes matching Montreal's alternate encodings) is **Step 10** — not part of the Step 7 gate. Blocked on contacting the original LZO toolchain authors; always remains the final step.

## Godot exporter

Godot export reads **`Level`** (not Fix directly, not Rete JSON, and not a dedicated OpenSpace scan path). After **Step 8**, `Level` holds live scene and geometry via object references — Godot formatters walk the hub directly without VM pointer chasing. Today `Level.Load` still rehydrates SNA + RT* for transitional readers; Step 8 removes that. Godot export has its own quality bar and does not require byte-identical output.

## CLI surface (target)

```bash
astrolabe import-openspace <level-dir> [rete-dir]
astrolabe export-openspace <rete-dir> [level-dir]
astrolabe export-godot <openspace-dir | rete-dir> [godot-dir]
```

Remove `extract-intermediate`, `compile-intermediate`, and any other transition command names in Step 6 — no aliases.

## Implementation steps

The refactor proceeds as **sequential steps** on one branch — not as separate pull requests. Code map and API contracts live in [`notes/rete-implementation.md`](notes/rete-implementation.md). The **decompressed parity gate** is Step 7 only — do not block Steps 6–9 on compressed-byte `cmp`. Step 10 is the optional full compressed `cmp` gate (external dependency; always last).

**Progress:** Steps **1–8 complete** (hub refactor — live references, export-only VM layout). Step **9** (texture/sound sidecar promotion) is next. Step **10** (compressed LZO parity) deferred until LZO encoder parity is resolved with the original authors. Step 5 delivered: `RelocationGenerator` (RTB/RTP/RTT/fixlvl), export generates RT* from struct codecs and opaque LUTs with no relocation bridge in Rete, **67** struct codecs, **LZO done** (`OpenSpaceLzo` + `lzo1x` at `-O0`). Step 7 closed RTB/fixlvl/RTP pointer plaintext gaps, SNA decompressed plaintext parity, and `Fix.rtv` policy.

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

- Import writes Fix + level Rete together in one pass (target: mirrored `Gamedata/World/Levels/` layout; today transitional `output/fix/` + `output/{level}/`).
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

**Not Step 5:** decompressed export parity, closing the last RTB pointer gaps, or encoding gotchas — Step 7 (complete). Compressed LZO container matching is Step 10.

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

### Step 7 — OpenSpace export parity (decompressed gate) ✓

**Complete** on `astrolabe` for **decompressed/plaintext parity**. Finish what was deferred from the end of Step 5: generated content parity and export fidelity tests. Compressed LZO container matching is **Step 10**.

**Delivered**

- [x] SNA decompressed plaintext matches disc on export (null opaque LUT entries preserve imported `.bin` values instead of zeroing fringe RT* sites).
- [x] Generated RT pointer plaintext parity on `astrolabe`: RTB **69922/69922** matching, RTP/fixlvl pointer-data match on fresh import.
- [x] `fixlvl.rtb` empty blocks (`07:00`, `13:01`) via manifest `fixlvlBlockKeys`; migration warnings for stale packages missing the field.
- [x] `Fix.rtv` — explicit unsupported placeholder (pass-through `files/Fix.rtv` copy; no generator).
- [x] Fidelity gate refocused: `Category=Disc` tests Layer B (decompressed SNA/RT*) + Layer C (decompressed blobs + sidecar files). Layer A compressed checks moved to `CompressedDiscFidelityTests` (gated in Step 10).

**Exit criterion (met):** unedited Rete → `export-openspace` → decompressed SNA/RT* plaintext and generated relocation pointer data match disc; non-generated sidecar files byte-match.

### Step 8 — Hub refactor (live references, export-only VM layout) ✓

**Complete.** `Level` and `Fix` are real in-memory hubs with **object references**, not VM addresses. Rete stores URIs; OpenSpace export alone materializes `int32` pointers and generates RT*.

**8a — Hub canonical types**

- Replace `int` pointer fields on promoted records (`SpawnableEntryRecord.Perso`, `SceneNode.OffData`, …) with typed references that resolve within `Level` / `Fix` (element id, package-relative path, or direct object link — same identity the URI model names).
- `import-openspace`: OpenSpace bytes → hub types (resolve VM pointers to references during import; emit URIs when persisting to Rete).
- `Level.Load` from Rete: URIs → references into the hub catalog; **lazy hydration** of promoted records when followed (`HubCatalog.TryHydrate`), not a bulk preload of every element. **No** SNA/RT* rebuild, **no** `LevelLoader` VM map, **no** `HydrateFromRetePackage` relocation regeneration.

**8b — Export-only OpenSpace pipeline**

- `export-openspace`: `Level` / `Fix` → layout → references to VM addresses → struct codecs write bytes → `RelocationGenerator` → RT* → LZO.
- **Delete** the transitional path where `Level.Load` from Rete walks Rete JSON / regenerates RT* for readers.
- **Delete** `export-openspace` bypassing the hub (direct Rete JSON export walk). One path: hub → OpenSpace.
- `export-godot` and mesh/scene readers consume hub references directly (no `MemoryContext.GetPointerAt` for promoted data).

**8c — Fix hub**

- Add `Fix.cs` (or equivalent) as the Fix in-memory hub; mirror the level pattern (references in memory, URIs on disk, VM layout at export only).

**8d — Tests**

- Step 7 decompressed parity must still pass after the refactor.
- Add hub-round-trip tests: import → `Level` → export without relying on VM rehydration on load.

**Exit criterion:** `Level.Load(reteDir)` returns a lazy hub (catalog + references, hydrate on use) usable by Godot export without generating RTB; `export-openspace` is the only code path that assigns VM addresses and emits relocation tables.

### Step 9 — Texture and sound sidecars (PNG/WAV assets, URI pointers, generated RT*)

Promote level **sidecar pointer tables** currently stored as opaque `files/` pass-through copies. Today `{level}.gpt` / `{level}.ptx` sit in `files/` unchanged; RTP/RTT are regenerated by heuristically scanning `uint32`s in those blobs. `{level}.sda`, `{level}.snd`, and `{level}.rts` (and related RT*) are byte-copied with no URI conversion and no RTS generator.

**Canonical media is PNG and WAV** — not raw GF/CNT members, BNM banks, or APM blobs. Textures live in game-wide **CNT archives** on disc; `{level}.ptx` points at `TextureInfo` names in SNA that identify GF members inside those archives. On import, decode referenced GF/APM/BNM into **mirrored paths under the output root** — the same tree as the game (`Gamedata/Textures/**/*.png` from `Textures.cnt`, `Gamedata/Vignette/**/*.png`, `Gamedata/World/Sound/**/*.wav`). `fix.cnt` is not a texture source ([`docs/file-format-catalogue.md`](docs/file-format-catalogue.md)). Sidecar pointer JSON uses **`texture:/…`** and **`sound:/…`** URIs whose paths **match the mirrored game layout** (resolved against the output root, not the source disc). Godot export reads those paths directly; OpenSpace export **re-encodes** PNG→GF (rebuilding `.cnt` at the mirrored locations) and WAV→APM/BNM. Reuse existing decoders from `extract` / `textures` / `audio` CLI and `FileFormats/`.

**9a — Mirrored output layout + asset import**

- **Target output tree** mirrors the disc from one output root (see [`docs/cross-package-uris.md`](docs/cross-package-uris.md)); retire the flat `output/fix/` + `output/{level}/` convention.
- `import-openspace <disc-level-dir> <output-root>` writes Rete packages and decoded assets at mirrored paths; Fix Rete at `Gamedata/World/Levels/`, level Rete at `Gamedata/World/Levels/{level}/`.
- Walk PTX texture names + SDA/SND targets → decode from CNT/Sound → write PNG/WAV at mirrored paths; emit `texture:/Gamedata/Textures/….png` (etc.) on promoted pointer fields.
- Import pulls only textures/sounds **referenced** by the level batch (PTX names, sound sidecars), not entire CNT archives — but storage is global keyed by GF name / bank path, not duplicated per level.
- Optional provenance on asset manifest entries (source CNT, GF path, BNM bank, event id) for debugging.

**9b — Textures**

- Promote **GPT** and **PTX** to canonical Rete types with **reference URIs** for pointer slots — SNA `TextureInfo` via `types/…` or `level:/…`; GF payload via **`texture:/…`** pointing at the shared PNG corpus (keyed by the same name string PTX already carries).
- Reuse `GptReader`, `TextureTable`, and texture name resolution from `FileFormats/` — import parses sidecars into JSON; export materializes URIs → VM pointers, encodes PNG→GF, and writes GPT/PTX bytes.
- RTP/RTT generation walks **codec pointer metadata** on the promoted sidecar types, not a blind `uint32` scan of `files/{level}.ptx`.
- **Delete** opaque GPT/PTX pass-through in `looseFiles` once promoted paths own the data.

**9c — Sound**

- Promote **SDA** and **SND** similarly; add **RTS** relocation generation (today unsupported / pass-through).
- Sound pointer URIs target **`sound:/…`** paths in the shared WAV corpus under `Gamedata/World/Sound/` (one WAV per bank event or sample — naming convention TBD). Export re-encodes WAV→APM/BNM layout for OpenSpace parity.
- Dialog/language sidecars (`.dlg`, `.lng`, `.rtd`, `.rtg`) stay lower priority; non-audio/non-texture `Gamedata/` targets may still use **`game:/`** until promoted.

**9d — URI resolver + hub + export**

- `ReferenceUri` resolves `fix:/` to `Gamedata/World/Levels/`, `level:/` to `Gamedata/World/Levels/{level}/`, and `texture:/` / `sound:/` against the **mirrored output root**. No disc mount required for export.
- Sidecar records live in the **Level** hub (Step 8 references) or as first-class Rete `types/` elements — not resurrected as `files/*.ptx` opaque blobs on load.
- `export-openspace` regenerates sidecar bytes, texture CNT archives (`Textures.cnt`, `Vignette.cnt`), and RT* from hub/Rete URI records + shared PNG/WAV payloads; parity tests cover GPT/PTX/SDA/SND plaintext and RTS/RTP/RTT pointer data.

**Exit criterion:** fresh `import-openspace` → shared PNG/WAV tree populated for referenced assets; level Rete stores GPT/PTX/SDA/SND with `texture:/` / `sound:/` pointer fields; `export-openspace` regenerates OpenSpace CNT/sound wire layout and matching RTP/RTT/RTS **without** reading the source disc or relying on imported sidecar byte copies in `files/`.

### Step 10 — Compressed export parity (full `cmp` gate)

**Deferred — always last.** Match Montreal's **compressed** LZO container bytes on export, not just decompressed plaintext. Step 7 proved content correctness; Step 10 is bit-exact file `cmp` against the import source.

**Blocked:** alternate LZO1X encodings (e.g. `astrolabe.rtb` `05:01` tail — 4 bytes at `0x1D1A5`) likely require details from the original LZO / Montreal toolchain authors. Do not schedule this step ahead of external contact; it remains the final gate when/if encoder parity is achievable.

**Primary work**

- Re-enable and pass `CompressedDiscFidelityTests` (Layer A): every compressed SNA block and RT* block recompresses to disc-identical bytes.
- **`astrolabe.rtb` `05:01` LZO tail** — see `tools/LzoDiffProbe`.
- **SNA recompression** — `astrolabe.sna` blocks `05:01`, `06:02`, `11:01` differ from disc compressed blobs.
- Restore full-file Layer C `cmp` (or equivalent) once compressed blobs match.

**Exit criterion:** unedited Rete → `export-openspace` → `cmp` every file in the source level directory (Fix validated separately).

### Coverage expansion (parallel)

Promote remaining documented leaves per checklist: `visualset`, element types, Perso/family, animation, AI/DSG, sectors/collision. Each promotion adds codec + pointer metadata; relocation generator coverage grows with it. Continues alongside Steps 5–10 as types are ready — does not block Step 6. Texture/sound sidecar leaves are explicitly scheduled in **Step 9**.

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
| [`docs/cross-package-uris.md`](docs/cross-package-uris.md) | `fix:/`, `level:/`, `texture:/`, `sound:/` URI spec |
| [`docs/geometry-format.md`](docs/geometry-format.md), [`docs/lighting.md`](docs/lighting.md) | OpenSpace struct reference |
| [`docs/relocation-tables.md`](docs/relocation-tables.md) | RT binary layout for generator |

## Dependencies

- `lib/BinarySerializer.OpenSpace` — OpenSpace type definitions submodule
- `reference/raymap` — read-only reference implementation