# fixlvl.rtb — Fix-to-Level Cross-Package Relocations

Montreal OpenSpace (Hype, Rayman 2, Tonic Trouble) loads **shared Fix data** and a **level SNA** into one virtual address space. `fixlvl.rtb` is the per-level relocation table that tells the loader how to fix pointers **stored in Fix** whose targets live in **level-owned memory**.

This document explains what `fixlvl.rtb` entries mean, how to resolve them to real structures, and how they relate to `Fix.rtb`. For RTB binary layout see [`relocation-tables.md`](relocation-tables.md). For `ObjectList` / `Family` semantics see [`perso-mesh-animation.md`](perso-mesh-animation.md).

## Files and roles

| File | Location | Applies to | Typical targets |
|------|----------|------------|-----------------|
| `Fix.rtb` | `Gamedata/World/Levels/Fix.rtb` | Pointers inside `Fix.sna` | Other Fix blocks |
| `{level}.rtb` | `Gamedata/World/Levels/{level}/{level}.rtb` | Pointers inside level SNA | Level blocks (and sometimes Fix, via URIs at export) |
| `fixlvl.rtb` | `Gamedata/World/Levels/{level}/fixlvl.rtb` | Pointers inside `Fix.sna` | **This level's** SNA blocks |

At load time the engine merges `fixlvl.rtb` into the Fix relocation set (raymap: `fixRtb.Add(fixLvlRtb)`), then applies fixups while mapping both SNAs.

`Fix.sna` is **shared** — the raw `int32` values at Fix pointer sites are identical for every level. `fixlvl.rtb` is **per-level** because it records which **level block** owns each escaping Fix pointer for the level currently being loaded.

In Rete, `fixlvl.rtb` is **generated at level export**, not stored in either package. See [`rete-format.md`](rete-format.md).

## What one entry says (and what it does not)

A `fixlvl.rtb` row is **relocation metadata**, not a semantic label:

```text
(Fix VM address of int32 slot) → (level target block module:id)
```

It does **not** say “root node”, “Hype”, or “spawn point”. To learn the actual target you must combine:

1. **`fixlvl.rtb`** — which level SNA block contains the target bytes.
2. **Raw pointer** — the `int32` at that Fix address in `Fix.sna` (same bytes on every level).
3. **Level memory** — interpret the target address inside the loaded level SNA (and applied relocations).

```text
fixlvl entry:  Fix 0x0262C4C0  →  level block 06:02
       +
raw pointer:   [Fix.sna + 0x0262C4C0]  ==  0x0944D21C
       +
level layout:  address 0x0944D21C in block 06:02
       ⇒
resolved:      ObjectList header / field near 0x0944D208 (51 PhysicalObject entries)
```

The relocation algorithm itself only needs block identity; semantic decoding is a separate step on the combined Fix+level map.

## Observed targets on Hype (`astrolabe`)

Analysis of all **111 mapped** (non-`FF:FF`) entries on the `astrolabe` test level (June 2026):

### Primary: level `ObjectList` tables (block `06:02`)

Roughly half of mapped pointers resolve to addresses on or immediately beside valid **`ObjectList`** headers in the level main data block (`06:02`). These are per-level mesh-configuration tables: collections of **`PhysicalObject`** entries (torso, head, limbs, etc.) used by **`Family`** animation templates in Fix.

| Fix region | Pattern | Resolved target |
|------------|---------|-----------------|
| `Fix` block `05:00`, early cluster (`0x02210xxx`) | Sparse pointer sites | `ObjectList` with 51–222 entries |
| `Fix` block `05:00`, large array (`0x0262Cxxx`…) | Repeating **0x15C** (348-byte) records; pointer often at **record+0xC4** | Nearby `ObjectList` (typically 51 or 102 entries) |

This matches the flyweight model in raymap: **`Family` and animation templates live in Fix**, but **`off_physical_list_default` and `ObjectList` bodies live in level memory**. `fixlvl` is the wiring that lets shared Fix families point at **this level's** object tables.

`ObjectList` is a stable cross-level *concept* (every level has them in block `06:02`), but the **bytes** at each target address are level-specific. The Fix pointer sites and target block IDs (`06:02`) are largely stable; the tables they aim into differ per level.

### Secondary: spawnable / object-index region (block `05:01`)

Four mapped entries target level block **`05:01`** near the GPT spawnable list (`OffSpawnableHead`). Raw values land in the spawnable object-index region (~`0x09D006xx`), wiring Fix character definitions to **which perso variants are spawnable in this level**.

### Unclassified / out-of-range

Remaining mapped entries fall into two buckets:

- **`0x094Bxxxx` inside `06:02`** — level-owned data, not near a valid `ObjectList` header; likely state/transition blobs or other family-adjacent structures not yet decoded in Astrolabe.
- **`0x094Dxxxx` above block `06:02` end** — raw values sit outside the loaded level VM window on `astrolabe`. May require full relocation before reading, or are unused/stale slots for this test level. Active area for Step 5 `fixlvl` generator parity.

### Sentinel targets (`FF:FF`)

~90% of all `fixlvl` rows (~1006 of 1117 on `astrolabe`) use target **`FF:FF`**. These mark Fix pointer slots the table tracks but **cannot map to a known level block** for this level (null, out-of-range, or not applicable). Per-level `fixlvl` files differ mostly in which sentinel slots are included, not in remapping shared entries to different block IDs.

## Cross-level stability

Comparing `fixlvl.rtb` across several Hype levels (`astrolabe`, `brigand`, `casino`, `ciel1`, `cite3`, `cachots`):

| Property | Behavior |
|----------|----------|
| Fix pointer **sites** (`offsetInMemory`) | ~99% overlap (~1108–1114 shared of ~1115–1120 total) |
| Target **block** (`module:id`) for shared sites | **Identical** across levels (including all mapped non-sentinel rows) |
| Level-exclusive sites | Small handful (~3–10), all observed as `FF:FF` sentinels |
| File size | ~4297–4312 bytes (varies with sentinel count) |

So `fixlvl.rtb` is **not** “the same Fix offsets pointing at different level blocks per level.” It is almost the same table of Fix-side escape pointers, with the same block keys, plus minor sentinel differences.

## Fix-side source layout

Mapped pointer sites observed in **`Fix.sna` block `05:00`**:

- **Early cluster** (`0x022103F4`, …) — sparse fields in the start of the block.
- **Large array** (`0x0262C364`, …) — repeating records spaced by **`0x15C`** bytes; the relocated `int32` often sits at **`+0xC4`** within each record (also `+0x6C`, `+0xAC` in some rows).

Astrolabe has not yet promoted this Fix record type to a named codec. Treat it as **Fix-side per-level variant metadata** that holds pointers into level `ObjectList` data until the struct is fully reversed.

## Relation to scene graph roots

GPT world roots (`OffActualWorld`, `OffDynamicSector`, `OffFatherSector`) also live in level block `06:02`, but **`fixlvl` mapped entries do not point at those addresses**. Scene graph traversal starts from GPT on the level side; `fixlvl` instead fixes **Fix→level** links for character/object configuration (chiefly `ObjectList`).

A useful rule of thumb:

- **GPT + level `.rtb`** — where is the world / scene hierarchy?
- **`fixlvl.rtb` + Fix bytes** — how do shared Fix families bind to **this level's** mesh tables and spawnables?

## Astrolabe / Rete implications

**Import:** Transient disc `fixlvl.rtb` is read during level import. Mapped rows annotate Fix opaque LUT entries with `level:/slots/0x{fixSite:X8}.json` and copy the resolved level record into `slots/0x{fixSite:X8}.json`. Sentinel (`FF:FF`) rows record `null` URI in the Fix opaque LUT. No fixlvl site inventory is persisted in the Fix package. Fix.rtb import (on the Fix package) is separate and does not substitute for fixlvl site discovery.

**Export:** `RelocationGenerator.GenerateFixLevelRtb` scans Fix opaque elements for LUT entries with a `level:/` URI or `null` URI (fixlvl sentinel from import). Pointer values are read from raw `.bin` (`record.Data`). Mapped rows resolve via `level:/` URI or value fallback against level layout; unresolved rows (including in-VM disc sentinels) emit `FF:FF`. Escaping Fix pointers without a `level:/` URI also emit `FF:FF` when the value leaves the Fix VM range. Struct `PointerFields` are **not** scanned for fixlvl (`fix:/` and package-relative LUT entries from Fix.rtb are ignored).

**Generator parity (`astrolabe` gate):** `fixlvl.rtb` — 1116 / 1117 row match (1 missing, 1 extra fringe). `astrolabe.rtb` — 69777 / 69922 (145 missing sentinels, 0 extra).

**Debugging workflow:**

1. Parse `fixlvl.rtb` → `(fixAddress → targetModule:targetId)`.
2. Read `int32` at `fixAddress` from `Fix.sna`.
3. Merge Fix + level SNA (and RTBs) into one memory map.
4. If target is `06:02`, try `FamilyReader.ReadObjectList` at the raw address and nearby offsets (±64 B) — headers are often a few bytes away from the stored pointer.
5. If target is `05:01`, inspect spawnable / object-index region near GPT `OffSpawnableHead`.
6. If target is `FF:FF`, treat as intentionally unmapped for this level.

Fix→level links in Rete use `level:/slots/{fixSite}.json` URIs; level→Fix uses `fix:/…`. VM block+offset stays in import-generated metadata, not in the URI. Full spec: [`cross-package-uris.md`](cross-package-uris.md).

## References

- [`relocation-tables.md`](relocation-tables.md) — RTB binary format and relocation algorithm
- [`perso-mesh-animation.md`](perso-mesh-animation.md) — `Family`, `ObjectList`, `PhysicalObject`
- [`rete-format.md`](rete-format.md) — Fix package layout, export of `fixlvl.rtb`
- `reference/raymap/Assets/Scripts/OpenSpace/Loader/R2Loader.cs` — load merge of `fixlvl.rtb`
- `reference/raymap/Assets/Scripts/OpenSpace/Object/Properties/Family.cs` — `off_physical_list_default`
- `src/Astrolabe.Core/Rete/OpenSpace/RelocationGenerator.cs` — `GenerateFixLevelRtb`