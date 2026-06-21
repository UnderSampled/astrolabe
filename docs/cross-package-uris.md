# Cross-Package Reference URIs

Astrolabe Rete packages are **shared Fix** plus **per-level** packages. Cross-package pointers must serialize as reference URIs that:

- Are **not empty**
- Do **not** name a specific level directory (`../astrolabe/…` in Fix JSON is invalid long-term)
- Do **not** embed VM block+offset (copyright-sensitive; discovered at import, not predetermined in the repo)
- **Do** name a filesystem record path under the target package role

This document specifies `fix:/` and `level:/` URIs for Rete packages, plus **`texture:/` and `sound:/`** roles for canonical PNG/WAV in the shared game-data asset tree (**Step 9**). Resolver support for `fix:/` and `level:/` exists in `ReferenceUri.cs`; `texture:/` and `sound:/` are specified here for Step 9. A residual **`game:/`** role covers unpromoted `Gamedata/` paths (dialog, language) until those sidecars are promoted. **Transient import** annotates Fix opaque LUT entries from disc `fixlvl.rtb` (mapped rows → `level:/slots/0x{fixSite:X8}.json` plus per-level slot files; sentinel rows → `null` URI). Export generates `fixlvl.rtb` from Fix opaque LUT only (`level:/` mapped rows and `null`/escaping sentinel rows), not from walking Fix.rtb or any persisted site registry.

See also: [`rete-format.md`](rete-format.md) (overview), [`fixlvl-rtb.md`](fixlvl-rtb.md) (Fix→level relocation context), [`perso-mesh-animation.md`](perso-mesh-animation.md) (`ObjectList` semantics).

## URI grammar

```text
reference-uri := path [ "#" fragment ]

path          := relative-path | fix-path | level-path | texture-path | sound-path | game-path
fix-path      := "fix:/" package-relative-path
level-path    := "level:/" package-relative-path
texture-path  := "texture:/" png-relative-path
sound-path    := "sound:/" wav-relative-path
game-path     := "game:/" game-data-relative-path

fragment      := json-pointer | byte-offset-fragment
json-pointer  := RFC 6901 pointer (e.g. "#/perso")
byte-offset-fragment := "byteOffset=" int
```

### Forms

| Form | Referrer | Target package | Example |
|------|----------|----------------|---------|
| `types/foo.json` | level or fix | Same package | `types/objectlist/default.json` |
| `fix:/types/foo.json` | level | Shared Fix (`packageRole: fix`) | `fix:/types/perso/hype.json` |
| `level:/slots/0x….json` | fix | Level (`packageRole: level`) | `level:/slots/0x0262C4C0.json` |
| `texture:/Gamedata/Textures/foo.png` | level | Shared PNG from `Textures.cnt` GF | `texture:/Gamedata/Textures/torch.png` |

| `sound:/Bnk_ambient/Play_Foo.wav` | level | Shared WAV from `Gamedata/World/Sound/` | `sound:/Bnk_ambient/0001_Play_Foo.wav` |
| `game:/Gamedata/…` | any | Unpromoted path under mounted disc (dialog, lang) | `game:/Gamedata/LangData/EN/dialog.bin` |
| `…#byteOffset=4` | any | Interior field in bin-backed record | `level:/slots/0x….json#byteOffset=0xC4` |

`fix:/` and `level:/` are **Rete package roles**, not ad-hoc folder names. Conversion **output mirrors the game layout** from one output root; package roles map to fixed mirrored paths (below). URIs must not encode level directory names in Fix JSON (`../castle_village/…` remains invalid).

**PNG and WAV** are the canonical texture and sound payloads. Textures are **not** stored inside level or Fix Rete packages — they live in game-wide **CNT archives** on disc (`Gamedata/Textures.cnt`, `Gamedata/Vignette.cnt`). A level's `{level}.ptx` (and `Fix.ptx`) points at `TextureInfo` in SNA; the **name** field identifies a GF member inside those archives. `Gamedata/World/Levels/fix.cnt` is **not** a texture archive — copy-protection disc catalog only; see [`file-format-catalogue.md`](file-format-catalogue.md). Import decodes referenced GF/APM/BNM into a **shared asset tree** at the conversion output root; sidecar pointer JSON uses `texture:/…` and `sound:/…` URIs resolved against that tree (not the source disc). OpenSpace export re-encodes PNG→GF (rebuilding CNT) and WAV→APM/BNM. Godot export reads the same PNG/WAV files directly.

`game:/` is a **residual role** for unpromoted `Gamedata/` paths (dialog, language data, …).

### Legacy

Imports may contain `../fix/types/…` paths from earlier converters. Resolvers **must** keep accepting these. New imports **must** emit `fix:/` and `level:/`.

## Design rules

### 1. Pointer → filesystem reference

A reference URI is the pointer converted into a **stable filesystem address** under the target package — the same conceptual move as storing `types/visualmaterial/hype.json` for an intra-package link.

For Fix→level links the target is usually a level **`ObjectList`** (or spawnable-region record). The URI identifies **which slot** Fix points at, not the semantic name “Hype torso mesh”.

### 2. No VM in the URI

`fixlvl.rtb` plus Fix bytes plus imported layout yield `(module, id)` and byte offset. That metadata may be stored in **import-generated manifests** inside user output trees. It must **not** appear in committed repo templates and must **not** be encoded as `level:/block/06/02/offset/0x…` URIs.

### 3. Shared Fix, generic level slots

Fix Rete is exported **once** and shared by all levels. Fix JSON therefore holds `level:/…` URIs that are **identical for every level** — never `../castle_village/…`.

Each level package materializes the record at the resolved path (same relative path, different content). The game already behaves this way at the VM layer: shared Fix `int32` values, per-level bytes at the target offset (see cross-level `fixlvl` stability in [`fixlvl-rtb.md`](fixlvl-rtb.md)).

### 4. Empty URIs are invalid

Opaque or unmapped Fix→level pointer sites use codec sentinels or omit relocation-backed annotation — not `""` or `null` in promoted pointer JSON.

## Level slot paths (Fix → level)

### Slot key

Primary stable identity for a Fix→level mapped pointer:

```text
fixSite := 0x + 8-digit uppercase hex Fix VM address of the int32 slot
```

Canonical level path (default convention):

```text
level:/slots/{fixSite}.json
```

Example: Fix pointer at `0x0262C4C0` → `level:/slots/0x0262C4C0.json` in Fix JSON; file `slots/0x0262C4C0.json` in each level package.

Alternative layouts under `level:/` are allowed (e.g. `level:/types/objectlist/…`) once import assigns them; the prefix `level:/` is what matters for resolution.

### What gets stored where

| Location | Contents |
|----------|----------|
| **Fix package** JSON | `level:/slots/….json` on pointer fields |
| **Level package** `slots/….json` | Canonical record (`ObjectList`, etc.) for this level |
| **Fix package** `level-slot-manifest.json` (import-generated) | Union metadata: fix site → target block, optional kind, validation flags — **not** committed to the Astrolabe repo |

Fix does **not** duplicate level record bytes. Level does **not** embed Fix paths with level names.

### Mapped vs sentinel Fix sites

From `fixlvl.rtb` analysis on Hype (37 levels):

- **111 mapped** sites — same Fix offset set and same target block (`06:02` / `05:01`) on every level → **require** `level:/slots/…` URIs and level slot files.
- **~1000 sentinel** (`FF:FF`) sites — no level target → **no** `level:/` URI; optional import metadata only.
- **~50 sentinel** sites differ in presence per level → union policy (below).

## Conversion output layout (mirrors game)

One **output root** reproduces the mounted disc tree. Rete packages and decoded PNG/WAV assets live at the same relative paths as the original game files.

```text
{output-root}/                    ← mirrors disc root (e.g. output/ or a staging mount)
  Gamedata/
    Textures/                     ← PNGs decoded from Textures.cnt (export rebuilds Textures.cnt here)
    Vignette/                     ← PNGs from Vignette.cnt
    World/
      Levels/
        manifest.json             ← Fix Rete (packageRole: fix); types/, sna/, …
        types/…                   ←   co-located with where Fix.sna exports on disc
        astrolabe/
          manifest.json           ← Level Rete (packageRole: level)
          types/…
        brigand/
          manifest.json
      Sound/                      ← WAVs decoded from BNM/APM (export rebuilds banks)
```

| Package role | Mirrored root | On-disc analogue |
|--------------|---------------|------------------|
| `fix` | `{output-root}/Gamedata/World/Levels/` | `Fix.sna`, `Fix.rtb`, … (uppercase `Fix.*`) |
| `level` | `{output-root}/Gamedata/World/Levels/{level}/` | `{level}.sna`, `{level}.ptx`, … |

`fix:/` resolves to the **Levels** directory (parent of level subdirs). From `astrolabe/`, Fix is the parent package — matching how the engine loads `Fix.*` from `Gamedata/World/Levels/` alongside `{level}/`.

**Transitional:** today's import may still emit a flat `output/fix/` + `output/{level}/` tree. New work targets the mirrored layout above; `ReferenceUri` sibling-`fix/` lookup is legacy until migrated.

## Multi-level import

Import is modeled as a **batch of levels** (one, some, or all) into one mirrored output root:

### Pass outline (planned)

1. **Extract Fix once** — `packageRole: fix`; preserve `Fix.rtb`, SNA, codecs.
2. **Extract each level** — `packageRole: level`; preserve level RTB, `fixlvl.rtb`, SNA.
3. **Merge VM** per level (Fix + level) for pointer resolution during that level’s import.
4. **Rewrite level → Fix pointers** to `fix:/…` URIs.
5. **Build fixlvl union** across the batch (see below).
6. **For each mapped Fix site** in the union:
   - Read Fix `int32` at fix site.
   - Resolve level record in **this** level’s import (e.g. `ObjectList`).
   - Write `slots/{fixSite}.json` in **this** level package.
   - Write `level:/slots/{fixSite}.json` in **Fix** JSON (same URI for all levels; idempotent if Fix already imported).
7. **Write `level-slot-manifest.json`** into Fix package (import output only).

### fixlvl union policy

For each Fix pointer site `offsetInMemory` seen in any level’s `fixlvl.rtb`:

| Case | Union rule | Action |
|------|------------|--------|
| Target not `FF:FF` | Targets must agree across batch; else import warning/error | Emit `level:/slots/…`; include in manifest |
| Target `FF:FF` | Site is optional sentinel | Include in manifest as unmapped; no `level:/` URI |
| First import one level | Mapped set stable (~111 on Hype) | Sufficient for slot discovery |
| Full-game import | Sentinel presence union complete | Best `fixlvl` byte parity |

Empirical expectation on Hype: mapped rows are **identical** across all shipped levels; union adds confidence, not new mapped slots.

### fixlvl vs slot discovery

| Source | Provides |
|--------|----------|
| `fixlvl.rtb` | Fix site → **level block** `(module, id)` |
| Fix `int32` at site | **VM address** (shared across levels) |
| Level layout at import | Record boundary → **filesystem path** + slot file |
| **URI** | `level:/slots/{fixSite}.json` — **no block+offset** |

## Canonical assets — PNG and WAV (Step 9 — specified, not all implemented)

Texture and sound sidecar pointers (PTX, SDA, SND, …) target **decoded PNG/WAV in a shared asset tree**, not raw GF/CNT or BNM on disc and not files inside level/Fix Rete packages.

### Where textures actually live

On disc, GF payloads are archived — not per-level:

| Disc file | Role |
|-----------|------|
| `Gamedata/Textures.cnt` | Main texture archive (thousands of GF members) |
| `Gamedata/Vignette.cnt` | Full-screen / vignette images |

`{level}.ptx` and `Fix.ptx` are pointer tables into SNA `TextureInfo` records. Each record carries a **GF filename string**; the engine resolves that name against `Textures.cnt` / `Vignette.cnt`. Levels do not ship their own `.cnt` for ordinary textures.

`Gamedata/World/Levels/fix.cnt` is **not** a third texture archive — copy-protection catalog, outside uppercase `Fix.*` ([`file-format-catalogue.md`](file-format-catalogue.md)).

Import decodes only textures/sounds **referenced** by the imported level(s), but storage is keyed globally at **mirrored disc paths** — not duplicated per level package.

### CNT → folder rule

The game does not record which CNT a texture came from; `TextureInfo` only has a GF **name**. Astrolabe recovers provenance at import by scanning CNT archives, then stores PNGs in a **folder named after the CNT stem** (drop `.cnt`), at the same path as the archive on disc — matching `extract`:

| Disc archive | Decoded PNG root |
|--------------|------------------|
| `Gamedata/Textures.cnt` | `Gamedata/Textures/` |
| `Gamedata/Vignette.cnt` | `Gamedata/Vignette/` |

Internal CNT directory structure is preserved under that folder (`foo/bar.gf` → `foo/bar.png`). Export rebuilds each `.cnt` from its sibling folder. `texture:/Gamedata/Textures/…` URIs therefore encode both archive and member path.

### URI forms for assets

| Pointer target | URI in JSON | Resolved file |
|----------------|-------------|---------------|
| GF from `Textures.cnt` | `texture:/Gamedata/Textures/{path}.png` | `{outputRoot}/Gamedata/Textures/{path}.png` |
| GF from `Vignette.cnt` | `texture:/Gamedata/Vignette/{path}.png` | `{outputRoot}/Gamedata/Vignette/{path}.png` |
| Sound bank event | `sound:/Bnk_foo/{sample}.wav` | `{outputRoot}/Gamedata/World/Sound/…` |

The invariant is **paths under the output root that mirror the game tree** (`.png` / `.wav` where disc has `.cnt` / `.bnm`), referenced by `texture:/` and `sound:/`.

### Import vs export

| Phase | Behavior |
|-------|----------|
| **Import** | PTX name → locate GF in CNT → decode PNG → write shared tree → rewrite pointer fields to `texture:/…` |
| **Hub / Godot** | Resolve `texture:/…` → load PNG by GF name |
| **OpenSpace export** | PNG→GF, rebuild CNT archives; WAV→APM/BNM; URIs → VM pointers; generate RTP/RTT/RTS from codec metadata |

Optional **provenance** on asset manifest entries (source CNT, original GF path, BNM bank/event) aids debugging.

### Residual `game:/` role

Unpromoted sidecars (`.dlg`, `.lng`, `.rtd`, `.rtg`, …) may still use `game:/Gamedata/…` until promoted.

## Resolution (implemented)

`ReferenceUri.TryResolve(referringPackageRoot, uri, out filePath, out fragment, levelPackageRoot?)`:

1. Split `#` fragment.
2. `fix:/…` → Fix package root (self if `packageRole: fix`, else sibling `fix/` with manifest).
3. `level:/…` → Level package root (self if `packageRole: level`, else `levelPackageRoot` argument, else sibling level directory containing the target file).
4. `texture:/…` → `{outputRoot}/` + path (mirrored `Gamedata/Textures/…`, `Gamedata/World/Levels/fix/…`, etc.) (**Step 9**).
5. `sound:/…` → `{outputRoot}/Gamedata/World/Sound/` + path (**Step 9**).
6. `game:/…` → `{gameRoot}/` + path (unpromoted sidecars only; **Step 9+**).
7. Otherwise → relative path from referring root (legacy `../fix/…`).
8. Apply `#byteOffset=` or JSON Pointer at read time.

`ReferenceUri.MakeReference(referringPackageRoot, targetPath)` emits `fix:/`, `level:/`, `texture:/`, `sound:/`, or intra-package paths for import rewrite.

## Export (planned behavior)

### Level export (`export-openspace <level-rete>`)

- Referring package: level.
- Resolve `fix:/…` against sibling `fix/`.
- Resolve `level:/…` against self.
- Layout level SNA; generate `fixlvl.rtb` from Fix + level (not from preserved `fixlvl` once parity holds).
- Write **level directory only**.

### Fix export (`export-openspace <fix-rete>`)

- Referring package: fix.
- Resolve `fix:/…` against self.
- `level:/…` URIs in Fix JSON require **layout contract** from `level-slot-manifest.json` + last-known VM mapping to write Fix `int32`s — **or** Fix export is run only when Fix bytes are already resolved in the Rete package.
- Fix `int32` values are **shared** across levels; a correct Fix export should not depend on which level was imported last.
- Write **Fix directory only** (no `fixlvl`).

### Multi-level export session

```text
export-openspace output/fix           → Fix.* once
export-openspace output/astrolabe       → astrolabe.* + fixlvl.rtb
export-openspace output/brigand        → brigand.* + fixlvl.rtb
```

Same Fix bytes; different level SNA; per-level `fixlvl` (mostly identical sentinel fringe).

## Implementation status

| Component | Status |
|-----------|--------|
| `ReferenceUri` parse/emit `fix:/`, `level:/` | **Done** |
| Legacy `../fix/…` resolve | **Done** |
| Transient `fixlvl.rtb` import → `level:/slots/0x…` + slot files | **Done** (`AnnotateOpaquePointersFromFixLevelRelocations`) |
| Persisted fixlvl site registry on Fix package | **Removed** (URI-only opaque LUT; no `*-sites.json`) |
| Export `fixlvl.rtb` via `GenerateFixLevelRtb` | **Done** (opaque LUT only; not Fix.rtb walk) |
| Import rewrite → `fix:/` | **Partial** (legacy `../fix/…` still accepted) |
| Mirrored output layout (`Gamedata/World/Levels/…`) | **Partial** (flat `output/fix/` today; migration Step 9) |
| Shared PNG/WAV at mirrored paths | **Not started** (Step 9) |
| `texture:/` and `sound:/` parse/resolve | **Not started** (Step 9) |
| Sidecar pointers (PTX/SDA/SND) → `texture:/` / `sound:/` on import | **Not started** (Step 9) |
| OpenSpace export PNG→GF (CNT rebuild), WAV→APM/BNM encode | **Not started** (Step 9) |
| `game:/` parse/resolve (residual) | **Not started** (Step 9+) |
| `level-slot-manifest.json` union manifest | **Not started** |

## Verification (when implemented)

```bash
# Batch import
dotnet run --project src/Astrolabe.Cli -- extract-intermediate \
  disc/Gamedata/World/Levels/astrolabe output/rete/astrolabe
# (extend CLI for multi-level + union)

# Fix JSON uses fix:/ and level:/
grep -r '"fix:/' output/rete/fix/
grep -r '"level:/' output/rete/fix/

# Level packages have matching slots/
ls output/rete/astrolabe/slots/
ls output/rete/brigand/slots/   # same filenames, different content

# Round-trip
dotnet run --project src/Astrolabe.Cli -- compile-intermediate \
  output/rete/astrolabe disc-rebuild/astrolabe
cmp -r disc/Gamedata/World/Levels/astrolabe disc-rebuild/astrolabe
```

## References

- `src/Astrolabe.Core/Rete/ReferenceUri.cs` — resolver and emitter
- `src/Astrolabe.Core.Tests/ReferenceUriTests.cs` — scheme tests
- `src/Astrolabe.Core/Rete/OpenSpacePackageCodec.cs` — `RewritePointerReferences` (to be updated)
- [`fixlvl-rtb.md`](fixlvl-rtb.md) — mapped vs sentinel sites, `ObjectList` targets