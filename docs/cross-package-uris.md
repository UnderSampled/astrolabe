# Cross-Package Reference URIs

Astrolabe Rete packages are **shared Fix** plus **per-level** packages. Cross-package pointers must serialize as reference URIs that:

- Are **not empty**
- Do **not** name a specific level directory (`../astrolabe/…` in Fix JSON is invalid long-term)
- Do **not** embed VM block+offset (copyright-sensitive; discovered at import, not predetermined in the repo)
- **Do** name a filesystem record path under the target package role

This document specifies `fix:/` and `level:/` URIs and the **planned** import/export workflow. Resolver support exists in `ReferenceUri.cs`; import emission and level-slot assignment are **not implemented yet**.

See also: [`rete-format.md`](rete-format.md) (overview), [`fixlvl-rtb.md`](fixlvl-rtb.md) (Fix→level relocation context), [`perso-mesh-animation.md`](perso-mesh-animation.md) (`ObjectList` semantics).

## URI grammar

```text
reference-uri := path [ "#" fragment ]

path          := relative-path | fix-path | level-path
fix-path      := "fix:/" package-relative-path
level-path    := "level:/" package-relative-path

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
| `…#byteOffset=4` | any | Interior field in bin-backed record | `level:/slots/0x….json#byteOffset=0xC4` |

`fix:/` and `level:/` are **package roles**, not folder names on disk. Output layout (`output/fix/` beside `output/astrolabe/`) is a conversion-time choice; URIs must not depend on it.

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
fixSite := lower-hex Fix VM address of the int32 slot (no 0x prefix required in path)
```

Canonical level path (default convention):

```text
level:/slots/{fixSite}.json
```

Example: Fix pointer at `0x0262C4C0` → `level:/slots/0262C4C0.json` in Fix JSON; file `slots/0262C4C0.json` in each level package.

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

## Multi-level import

Import is modeled as a **batch of levels** (one, some, or all) into one output parent:

```text
output/
  fix/                 ← written once per output tree
  astrolabe/           ← per level in batch
  brigand/
  …
```

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

## Resolution (implemented)

`ReferenceUri.TryResolve(referringPackageRoot, uri, out filePath, out fragment, levelPackageRoot?)`:

1. Split `#` fragment.
2. `fix:/…` → Fix package root (self if `packageRole: fix`, else sibling `fix/` with manifest).
3. `level:/…` → Level package root (self if `packageRole: level`, else `levelPackageRoot` argument, else sibling level directory containing the target file).
4. Otherwise → relative path from referring root (legacy `../fix/…`).
5. Apply `#byteOffset=` or JSON Pointer at read time.

`ReferenceUri.MakeReference(referringPackageRoot, targetPath)` emits `fix:/`, `level:/`, or intra-package paths for import rewrite.

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
| Import rewrite → `fix:/` | **Not started** (still emits `../fix/…`) |
| Import Fix→level slot assignment | **Not started** |
| `level-slot-manifest.json` | **Not started** |
| Export via generated `fixlvl` | **Not started** (still compiles preserved RT) |

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