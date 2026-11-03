# Byte-coverage gaps: what the untouched ranges are

Static research, 2026-11-03.

Sources:
- Coverage reports from
  `ASTROLABE_BYTE_COVERAGE=output/byte-coverage target/release/astrolabe import-openspace disc <out> --level <level>`,
  plus `--content menu|inventory|pause-menu` (see `docs/astrolabe/testing.md`
  "Byte coverage").
- Allocation walks from my scratch scripts `lv.py` and `tail.py`, not in
  the repo.
- Ghidra bodies under `hype-decomp/reference/ghidra/functions/`.

Offsets are block-relative. Each allocation has a 4-byte header. "Scope
artefact" means a parser reads the structure in another `--content` scope,
or reads only part of it. "Inferred" marks byte-pattern identification that
wasn't traced to a loader.

## Summary

| Gap | What it is | Basis |
|---|---|---|
| Level 6:2 `0x0` +530444 (Manoir, Brigand) | Anim-playback pool: 2048 empty SuperObjects, 4096 matrices, 256-matrix buffer | decomp + exact byte match |
| Level 5:1 tail, Manoir `0x126334` +241661, Brigand `0x5d0a88` +250285 | 30 pooled "Always" actors: Perso component sets, saved-word array, 1500-byte buffers | decomp + bytes |
| Fix 19:0 `0x8` +3516 and 92-byte runs | FON text objects (blank buffers, 409-handle table); unread credits strings | decomp; strings from bytes |
| Fix 18:0 407-byte runs, `0x19d79` +11832, `0x1c248` +2409 | Rest of the 580-byte menu records; the unused part of the 12000-byte credits key table | arithmetic exact; record loader by size match only |
| Fix 19:1 small runs | Font glyph tables and the second 409-pointer table; probe artefacts | decomp + bytes |
| Fix 10:0 `0x22c4` +1060 and 88–136-byte runs | Inventory sprite lists and 600-byte string table; scope artefacts | decomp + bytes |
| Fix 10:1 `0x0` +1188 | Inventory item and spell label strings | bytes, targets of the 10:0 table |
| Fix 6:1 large runs | Mostly geometry/material/texture (font glyph quads, inferred); two unknown big allocations | low confidence |

## Level 6:2, first 0x8180c bytes (`before ai.rs:919`)

The same in Manoir (block 2,903,141 bytes) and Brigand (7,068,177).

### Loader

`FUN_00434940` runs when `FUN_004ea270() != 1`:
- it clears 0x400 dwords at 0x745d40;
- it sets `DAT_00745d20 = HIE_fn_p_stCreateSuperObjectTab(0x800)`;
- it calls `FUN_004365b0`.

`HIE_fn_p_stCreateSuperObjectTab` (004fca20, SPO_HieSpObj.c) allocates
`count<<6` bytes. For each record it calls `GEO_fn_hCreateMatrix` (00465910,
0x58 bytes) twice.

`FUN_004365b0` (GAM_PlayAnim_Play.c:0xb7a) allocates 0x5800 bytes into
`DAT_00746f48`. It fills it with `POS_fn_vSetIdentityMatrix` (004b93b0) at
a 0x58 stride.

The cleanup functions 00434e60, 00435020 and 00434a00 index `DAT_00746d40`
by `(matrix - DAT_00746f48)/0x58`.

### Layout

| Range | Allocations | Content |
|---|---|---|
| 0x0004–0x20004 | 1 × 131072 | 2048 SuperObject records × 64 |
| 0x20008–0x7c008 | 4096 × 88 (stride 92 with header) | local + global matrix per record, identity |
| 0x7c008–0x81808 | 1 × 22528 | 256 identity matrices × 88 |

The next allocation's header brings the end to 0x8180c, which matches the
report exactly. The pool is allocations 0–4097 of the block.

SuperObject record (64 bytes):

| Off | File value | Field |
|---|---|---|
| +00 | 0 | type |
| +04 | 0 | data |
| +08/+0c | 0 | first/last child |
| +10 | 0 | child count |
| +14/+18/+1c | 0 | next/prev/parent |
| +20 | 0 | ? |
| +24 | ptr | local matrix |
| +28 | 0xfffffffe | frame stamp (−2 when HIE current frame is 0) |
| +2c | 0xffffffff | ? |
| +30/+34 | 0 | ? |
| +38 | ptr | global matrix |
| +3c | 255.0f | transparency default |

Matrix (88 bytes):
- u16 type 0x80, u16 0;
- translation as 3 floats;
- two 3×3 matrices, both identity.

### Port

This is runtime scratch with no level content. The converter can claim it
as "anim-playback pool" and skip it. Which subsystem owns it ("PlayAnim")
comes from the source file string; medium confidence.

Other 6:2 gaps (`import.rs:235`, e.g. 0x294ca4 +44628, +37284, +36348)
were not investigated.

## Level 5:1 tail (after `way.rs:109`, to the block end)

### Loaders

- `FUN_0043cea0`: the level-script key `NumberOfAlways` sets `DAT_0060ee48`.
- `FUN_0043e7b0` (`src/actor/pooled_actor_creation.c`):
  - allocates `SuperObjectTab(count)` and a `count*0x4c` saved-Perso-word
    array (GAM_Always.c:0x6b);
  - per actor, calls `FUN_0043e730`, which allocates a 0x4c Perso (19
    words) and runs the 19 component constructors (table at VA 0x5e4bb0,
    stride 0x18);
  - copies the words into the saved array, then zeroes the Perso.
- `FUN_0043f060` (GAM_Always.c:0x27d): per actor, a 0x5dc (1500-byte)
  buffer filled with byte `index+1`, attached through saved word +0x10 via
  `FUN_004600d0`.
- `FUN_0043edd0`: release.

### Manoir layout

- 0x126338: a 4-byte `00 cd cd cd` allocation.
- 0x126340: the saved-word array, 2280 bytes = 30 × 76.
- 30 groups of 27 allocations, period 0x1948 (6472) bytes. Allocation sizes:
  `76, 444, 56, 292, 1024, 564, 68, 2488, 8, 220, 164×4, 52, 32, 12, 56, 24,
  8, 96, 24, 20, 76, 16, 16, 36`.
- 30 × 1500-byte buffers, filled with 0x01..0x1e.
- 4 × 16-byte allocations (Brigand has 7). Not identified.

The block has 11272 allocations, and the tail is the last 846 of them.

Perso slots, by the size of the allocation each points to:

| Slot | Off | Size | Name |
|---|---|---|---|
| 0 | +00 | 444 | Data3D |
| 1 | +04 | 56 | StandardGame |
| 2 | +08 | 292 | ? |
| 3 | +0c | 2488 | ? |
| 4 | +10 | 8 | gets the 1500-byte buffer |
| 5 | +14 | 220 | camera component (`notes/camera-research-2026-09-24.md:213`) |
| 6 | +18 | 52 | ? (inferred: collide set, per `unsupported-ai-entries.md` 1-30 using perso+0x18) |
| 7 | +1c | 32 | ? |
| 8 | +20 | 12 | ? |
| 9 | +24 | 56 | ? |
| 10 | +28 | 24 | sector-character list (`docs/openspace/save-games.md:350`) |
| 11 | +2c | 8 | ? |
| 12 | +30 | 96 | ? |
| 13 | +34 | 24 | ? |
| 14 | +38 | 20 | stock (`inventory-gameplay.md:40`) |
| 15 | +3c | 76 | ? |
| 16 | +40 | 16 | ? |
| 17 | +44 | 16 | ? |
| 18 | +48 | 36 | ? |

The 1024, 564, 68 and 164×4 allocations hang off the component allocations;
their parents aren't confirmed. Slot names for the remaining slots could
come from the 19 constructors in the 0x5e4bb0 table, which weren't named.

### Brigand differences

Brigand has the same 30-actor structure. About 10.9 KB earlier, starting at
0x5d0a8c, there are 244/232/240/16/284-byte records with names such as
"Sablier", "GlassUp", "StandByListGlassDown", "EmptyDown" and "FullDown".
These look like state or list name tables (inferred, no loader traced).

### Port

The pooled actors are templates for actors spawned at run time. No level
content is lost by skipping them, but `NumberOfAlways` and the 19-slot
order matter if spawned "Always" actors are ported.

## Fix 19:0, menu text (`menu.rs:768`)

**`0x8` +3516.** FON text objects created by `FON_fn_eLoadTextString`
(005bad30), which handles the `NewString`, `NewUpperString` and
`NewStringLenght` script commands, through `FUN_005bbda0`.

- Each object is 24 bytes: {char* buffer, +4 matrix array, +8, +0xc, +0x10,
  +0x14 per-character arrays or 0}.
- Each buffer is filled with 0x20 and NUL-terminated. The sizes here are
  256, 256, 104, 256, 256, 256, 4, 68 and 68.
- At 0x740 there is a 1636-byte table of 409 handle pointers. The tables are
  `DAT_0073f16c` and `DAT_0073f170`, sized `DAT_0073f174*4`.
- The port only follows the credits handles from `Fix.dlg`.

**92-byte runs from 0x5150 (0x520c, 0x526c, …).** Each is the tail of a
24-byte handle plus a credits string the port doesn't read. For example,
0x5168 holds "Special thanks to Serge Hascoet, …". The port reads
`keys.len()+10` handles. The cause is inferred from the code and bytes.

## Fix 18:0, menu records

Allocation census:

| Count | Size | Constructor (size match) |
|---|---|---|
| 61 | 0x244 (580) | `FUN_004b3a20` → `MNU_fn_p_vAlloc(0x244)`, the MenuText record (`MenuTextWire`, `menu.rs:48`) |
| 29 | 0x1c0 (448) | `FUN_004b2cd0` |
| 5 | 0x98 | `FUN_004b2cb0` |
| 8 | 0x1c | `FUN_004b3a00` |
| 104 | 524 | none found |
| 1 | 12000 | credits key table |

No callers of 004b3a20 or 004b2cd0 were found by pointer search, so the
record identity rests on size alone.

**Pause scope: 407-byte runs at `0x9b1`, `0xbf9`, `0xe41`, ….**
`menu.rs:388` reads 177 bytes of each 580-byte record:
- 8 links (32 bytes);
- position (3 floats);
- name[32];
- text ptr;
- 8 state words;
- selected font[32];
- normal font[32];
- 1 alignment byte.

That leaves 403 bytes, plus the next header, which makes 407. Nonzero bytes
past +0xb1 appear in at most 3 of the 61 records. They include the
CD-prompt strings "Place the original CD in the CDRom drive…", "Retry" and
"Cancel", and pointers near +0x23c. The layout beyond +0xb1 is not decoded.

**Pause `0x19d79` +11832.** The 12000-byte credits table, minus the 177
bytes read, plus a header.

**Main menu `0x1c248` +2409.** The credits key registry at 0x19cc8:
- 500 slots × 24 bytes, each a NUL-padded key with a `+` or `-` alignment
  prefix, e.g. `+CDT2_B_MGNT0`;
- the port expects exactly 400 keys (`menu.rs:690`);
- slots 400–499 are zero, 2400 bytes, plus 9 trailing bytes, which makes
  2409.

The arithmetic is exact.

## Fix 19:1, fonts and frames (`dialogue_presentation.rs:88`)

`named_font` probes 20 bytes of each allocation, and the gaps are what's
left. Probe artefacts; the content is identified.

Font record (32 bytes): {next, prev, ?, name ptr, 17, glyph count, glyph
table ptr, float tracking}.

| Font | Glyphs | Gap |
|---|---|---|
| Playmo_Font2_0 | 133 | |
| Playmo_Font2_1 | 133 | |
| Playmo_Font2_2 | 133 | `0x2638` +520, glyph table |
| Playmo_Font2_3 | 133 | `0x392c` +520, glyph table |
| FNT_SaveGame | 2 | |
| FixFont | 130 | `0x4c9c` +508 |
| InventoryFont | 127 | |
| MenuFont | 71 | `0x713c` +268 |
| MenuFont_H | 71 | |
| MenuFontSmall | 71 | |
| MenuFontSmall_H | 71 | |
| MenuFontMedium_H | 71 | |

- Glyph: 12 bytes {code, ?, shape ptr}. Shape: 12 bytes {geometry ptr,
  half_height, width}. There are 2294 12-byte allocations, which is 2 ×
  1146 + 2.
- `0xa5d0` +1620 is the second 409-pointer table (`DAT_0073f170`) at 0xa5bc.
  It points to 409 × 40-byte bloc-1 text companions
  (`FON_fn_p_vAllocInBloc(1,0x28)` in 005bbda0): a flags word, then a mode
  word (1 or 0x10), and so on. Mostly zero in the file.

## Fix 10:0 and 10:1, inventory

Decomp record sizes (`FUN_004ae910` in the creators):

| Record | Size | Creator |
|---|---|---|
| Item | 0x20 | 004a36f0 INV_fn_hCreateItem |
| InstanciatedItem | 0x18 | 004a3960 |
| Association | 0x14 | 004a3a60 |
| Box | 0x20 | 004a3ca0 INV_fn_hCreateBox |
| ListOfIit | 0xc | 004a3f60 |
| ElmtIit | 0x10 | 004a4040 |

Census of 10:0: 78×32, 146×24, 94×12, 81×8, 21×20, 5×16 and 1×600.

- **`0x22c4` +1060 (allocations 354–373 of 426).** This range holds the
  sprite layers of the last five boxes.
  - A ListOfIit {first, last, count}, then 24-byte sprite entries {texture,
    float w, float h, next, prev, list}, e.g. w = 245.0 or 42.0.
  - Then the 600-byte string table at 0x248c: 150 slots, 52 used. It comes
    from `INV_fn_xLoadTabOfStrings` (004a2ed0): `atol(arg)*4` on init, and
    `AddString index "text"` fills slots. The pointers go into 10:1 at
    offsets 4–1180.
- **88–136-byte runs** (default scope `0x9d4`, `0xa88`, `0xb3c` +92; `0x108`
  +136; inventory scope `0x1394` +88, `0x166c` +104).
  - Each covers an item name string (e.g. "MagicLight"), a ListOfIit and
    two 24-byte sprite entries: 16+4+12+4+24+4+24+4 = 92.
  - `inventory.rs` `images()` reads these in the inventory scope, so they
    are scope artefacts.
- **10:1 `0x0` +1188.** The inventory item and spell label strings,
  length-prefixed (u32 length, then text): "Green herbs", "Yellow herbs",
  "Blue potion", "Magia Glacia : Javelin", "Gold Bag", "Shield1", ….
  These are the targets of the 600-byte table. Map names such as
  "The_Field" start at 0x4a4.

## Fix 6:1 (5.5 MB), low confidence

| Gap | Contents | Notes |
|---|---|---|
| `0x0` +61212 | a 49152-byte allocation (16-bit repeating pattern, maybe texture data, unknown); 1536-byte float arrays; 132-byte VisualMaterials (`VisualMaterialWire`) and 188-byte TextureInfos (`TextureInfoWire`) | |
| `0x24f7c` +134916, `0x50650` +136164 | 1185 and 1053 48-byte allocations with 32/12/4-byte companions | 48 bytes is the `GeometryWire` size the font path uses. Inferred: font glyph quad geometry. The counts only roughly match the 1146 glyphs. |
| `0x500500` +61136 | 131 allocations of 1536/384/768/1024-byte arrays plus materials and textures | |
| `0x5127cc` +191813 (tail) | 121 allocations, mostly one 165692-byte allocation at 0x515518 | It starts with a pointer just past the block, then 0xCD fill (158792 bytes). Looks like an uninitialised pool. Not identified. |

## Not identified

- The 16-byte allocations at the end of the 5:1 tail.
- The Brigand pre-pool name records.
- Fix 6:1's 49152-byte first allocation and the 165692-byte tail
  allocation.
- 580-, 448- and 524-byte menu record fields past +0xb1, and the 448/524
  record types.
- The other 6:2 gaps at `import.rs:235`.

## Checklist lines

- [ ] Every record type is named: Level 6:2 `0x0`+530444 is the anim-playback pool (FUN_00434940: 2048 SuperObjects, 4096 + 256 identity matrices). Claim it as runtime scratch.
- [ ] Level 5:1 tail (Manoir +241661, Brigand +250285) is 30 pooled Always actors (FUN_0043e7b0/0043f060: 19-slot Persos, saved words, 1500-byte buffers). Claim it, and name the remaining Perso slots from the constructor table at 0x5e4bb0.
- [ ] Fix 18:0 menu records: decode the 580-byte MenuText past +0xb1 (CD-prompt text, pointers at +0x23c), and identify the 448- and 524-byte records.
- [ ] Fix 18:0 credits registry: 500 slots × 24, of which 400 are used. The unused 2400 bytes are padding.
- [ ] Fix 19:0/19:1 text: claim the FON blank text objects and the two 409-pointer tables (005bad30, 005bbda0), and read the rest of the credits strings.
- [x] Fix 10:1 `0x0`+1188 is the inventory label strings, indexed by the 600-byte AddString table in 10:0 (004a2ed0).
- [ ] Fix 6:1: confirm the 48-byte runs are font glyph geometry, and identify the 49152-byte and 165692-byte allocations.
