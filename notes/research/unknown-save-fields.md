# The 248 unknown Manoir save identities

Static research, 2026-11-03.

"Unknown" is the `unknown` tier in `src/openspace/save_keys_manoir.tsv`
(counted by `tools/manoir_checklist.py:15`). `tools/save_key_table.py`
generates it from `output/savegame-semantic-map/semantic-catalogue.json`:
- runtime_corroborated maps to verified;
- source_verified maps to source;
- a row is also promoted to source when `tools/save_meanings_manoir.tsv`
  has a hand meaning for it;
- everything else is unknown.

So unknown means "no hand-confirmed meaning". Many rows already have partial
evidence in `output/savegame-semantic-map/*-followup.md`.

The full row list, joined to the catalogue, is in my scratch directory, not
in the repo: `unk_table.txt` and `unk_full.json` under kip-zephyr
`cache/scratch`. Identities are `block:module:offset` in
`SaveGame0/Level000.sav`.

"Read" means seen in decomp, raw Ghidra or objdump of
`MaiDFXvr_bleu.exe`. "Inferred" means reasoned from AI usage or allocation
order.

## Groups (sum 248)

| | Group | n | Identities | Meaning (confidence) |
|---|---|---|---|---|
| A | Map icon table `+0x30` | 100 | `0:5:4986460 + 76n`, n=0..99, u32 (55 ones) | show this map icon on the current epoch's map (read, high) |
| B | Map label table `+0x38` | 60 | `1:10:1248 + 64i`, i=0..59, u32 (43 ones) | map text label revealed (read from draw code, medium-high) |
| C | Map destination slots | 10 | `0:5:4986348..4986384` step 4, u32, all −2 | index into table A to mark as destination, −2 = empty (read, medium-high) |
| D | Inventory SuperObject `data` links | 38 | `1:6:4437852..4450816`, 6-byte refs | 7 category list handles + 31 slot→item links (read registration; category labels inferred) |
| E | Inventory element words | 4 | `1:6:5322192 + 344k`, u32, all 0 | per-child 1-based item index, 0 = none (inferred, medium-low) |
| F | StandardGame.CustomBitsInit | 11 | see F | only bits 0 and 7 are named by the engine |
| G | Gauge threshold selector | 3 | `0:5:4985187, 4985591, 4985899`, u8, 0 | index into the root's 4 threshold bytes (read, high) |
| H | Engine singletons | 2 | `0:5:4986216`, `0:5:4986280`, u16 | selected magic slot + unknown high byte; ToolCam +0x98 state tag |
| I | POS prefixes | 2 | `2:6:1463692`, `2:6:1464196`, 36 bytes | Hype and StdCamer matrix headers + translation |
| J | Actor DSG variables | 18 | see J | from AI usage, mostly inferred |

## A. Map icon table (100 × 0x48)

- **Allocation and registration (read).** 00416800:50-56 allocates the
  table at `DAT_00750900`. 004164d0:14-18 registers `+0x30` of each entry.
- **Parser (read).** 00416560:34-38 parses rows with
  `"%d %s %d %d %s %d %d %s"`:

  | Record offset | Field |
  |---|---|
  | +0 | int |
  | +4 | name |
  | +0x10, +0x14 | x, y |
  | +0x18 | icon material (from the texture name) |
  | +0x30 | flag |
  | +0x34 | epoch |
  | +0x38 | string |

- **Reader (read).** The reader is in code Ghidra didn't split into
  functions (0x4172d8..0x417f30).
  - 0x417300 is the getter, `return DAT_00750900[idx]->+0x30` (checked).
  - The loops at 0x417855..0x4178cf, 0x417aab and 0x417b1a draw entry idx
    at (+0x10, +0x14) via 0x47c8e0, only if the getter is nonzero, +0x18
    is set, and the entry's epoch (+0x34, through 0x417310) is the current
    one.
- **Entry 0 (read).** 00416780 forces entry 0's flag to 1. It is the Hype
  marker, `MAP_PersoIcon`.
- **Not found.**
  - A static writer for the 55 saved ones: a .text scan found only the
    loader and reset.
  - Per-row labels: the authored `gamedata/vignette/map/<lang>/setting`
    file isn't on disk.

## B. Map label table (60 × 0x3c)

- **Parse and reset (read).** 00416800:25-37 allocates the table and
  004164d0 registers `+0x38`. The `*` rows are parsed by 00416560 case
  0x2a: `"%d %d %d %d %s"` fills +0x30, +0x28, +0x2c, +0x34 and the label
  at +0. `+0x38` is never parsed. 00416780 zeroes it.
- **Reader (read).** The loop at 0x417c52..0x417d61:
  - skips empty labels;
  - skips the entry if `[+0x38] == 0` (`cmp [edi+0x38],ebx` at 0x417ca0,
    checked);
  - checks the epoch +0x34 against the table at 0x750a90;
  - draws the label with `Playmo_Font2_1` at (+0x2c, +0x28) through
    0x5bc260 and 0x417520.
- **Meaning.** Label revealed. It is not a collected-star flag, which
  matches the existing note.
- **Not found:** the writer.

## C. Destination slots (`DAT_00750ac4`, 10 × u32)

- **Allocation and reset (read).** 00416800 allocates the slots; 00416780
  resets them to 0xfffffffe.
- **Readers (read).** 0x4178dd..0x4179a4 and 0x417b76..0x417c1a. For each
  slot that isn't −2, the reader draws `MAP_Destination`
  (`DAT_0061056c`) at table-A entry [value] when its epoch is current.
- **In the slot:** all ten are −2.
- **Not found:** the writer and the role of each slot.

## D. Inventory SuperObject links (38)

00417fe0:79-96 (read) walks the 7 children of the `Inventory.isp`
SuperObject `DAT_006107b4`. For every grandchild, it registers the `data`
word (+4) with `FUN_004c62e0(…, 0x200)`. 00419450:20-55 clears index > 0
nodes (`p[0] = p[1] = 0`).

Reconstruction from allocation order (inferred, after
`level6-reference-family-followup.md`):

| List | Handle | Slots (saved contents) | Guess |
|---|---|---|---|
| 1 | `0:10:40` | 2: Weapon 80, null | weapons |
| 2 | `0:10:8644` | 6: Object 21, 5 null | objects |
| 3 | `0:10:8324` | 4: NotFound 50 | magic (Ice?) |
| 4 | `0:10:8404` | 4: MagicFire 61, 3 NotFound | magic Fire |
| 5 | `0:10:8484` | 4: NotFound 50 | magic (Lightning?) |
| 6 | `0:10:360` | 5: RestoreMP 10, RestoreHP 12, 3 null | restore items |
| 7 | `0:10:8564` | 6: null | ? |

Each list has 1 handle and its slots: 7 + 31 = 38. `255:255:0` is the
null reference. The category labels are inferred, apart from Fire, which
is anchored by MagicFire 61.

## E. Inventory element words (4)

- **Registration (read).** 00417fe0:98-101 registers `child+0x28` for each
  child of `DAT_006106b8`, the second Inventory.isp SuperObject.
  00417fe0 and 00419450 zero it. `FUN_0041de20(index, value)` sets it.
- **Reader (read).** 0x41b194..0x41b1e3: if `v > 0` and
  `v-1 < parent[+0x10]`, it draws the child's v-th list item via
  0x419540.
- **Meaning (inferred).** A 1-based item index shown in the HUD. What the
  four children are (shortcut slots?) is unknown.

## F. StandardGame.CustomBitsInit (u32 at +0x2c, registered by 00438c30)

The engine names only two bits (read; `custom-bits-usage-followup.md`):
- **Bit 0:** target / nearby-character eligibility (00528fa0:44-46,
  69-71; 0053a900:58-61, 84-87, 133-136, 165-168).
- **Bit 7:** skips the collision tests in one action branch
  (0043bcd0:93-96).

Bits seen in AI, 1-based payloads as in `tools/save_actor_audit.py`:

| Id | Actor (model) | Saved | AI use |
|---|---|---|---|
| `0:5:24256` | Hype (fix/28) | 0 | tests 2,3,4,6,7; changes 10 |
| `0:5:4979180` | Object_template (fix/1196) | 0x11 | none |
| `1:5:1041160` | iIcons_Manoir (868) | 0x43 | tests and changes 10 |
| `1:5:1070916` | iSerrure_Manoir1 (915) | 0 | none |
| `1:5:1131128` | iFiole_Pont (1106) | 1 | changes 1, tests 4 |
| `1:5:1133884` | iGold_ManoirPont (1144) | 1 | changes 1, tests 4 |
| `1:5:1136640` | iHerbe_ManoirPont (1182) | 1 | changes 1, tests 4 |
| `1:5:1155136` | iLevierSecretManoir (1396) | 0 | none |
| `1:5:1202608` | iFioleBleue_NearBrigand (1106) | 1 | changes 1, tests 4 |
| `1:5:727144` | iCheckPoint_Manoir_MAD (348) | 0 | tests 14 |
| `1:5:842740` | iKeys_Manoir_A (703) | 0x207 | tests/changes 10, changes 3 |

- **Pickups (1106, 1144, 1182).** Payload 1 is cleared on pickup.
  Payload 4 is tested negated before the pickup (1106.gd:340-348), and no
  AI sets it. "Available / not handled" is a guess.
- **Other bits.** These have no engine names in this build. The fields stay
  unknown at bit level; the u32 can be saved and restored verbatim.

## G. Gauge threshold selector (read, high)

0043a8a0 registers `root+0x1f`, and 0043aa70 uses it as
`thresholds[root+0x1f]`, indexing bytes +0x18..+0x1b with no range check.
The saved selectors are 0 and the thresholds are [1,2,3,4]. The panels'
authored names are unknown (`gauge-selector-followup.md`).

## H. Engine singletons (read)

- **`0:5:4986216`, low byte (+0x28): selected magic slot 0..2.** It's set
  at 00439830:11-14, wrapped by 00439db0 and 00439df0, and read by
  00439e30 and 00439e50.
- **`0:5:4986216`, high byte (+0x29):** no known use.
- **`0:5:4986280`:** the low 16 bits of the dword at +0x98 of the object
  reached from ToolCam slot 1 (0043da70 → 0043da90 → +4 → +0x14 → +4).
  - Saved by 00431130 via 00440f40, and restored by 004390e0 via 00440f50.
  - The engine writes 0x6c (00439200:26-35) and tests 0x6a/0x6b
    (0043bcd0:455-465). This is the camera mode word that the 4-15x camera
    procedures test for 101/104/105 (see `unsupported-ai-entries.md`).
    That link is inferred: it is the same `+0x98` getter 00440f40.
  - It has no gameplay name (`post-load-word-followup.md`).

## I. POS prefixes (36 of 88 matrix bytes)

| Id | Owner | Translation (+4/+8/+12) |
|---|---|---|
| `2:6:1463692` | Hype | (−74.381, 28.506, 4.569) |
| `2:6:1464196` | StdCamer `world/0/2` (source-order match, not proven) | (−71.241, 28.581, 7.351) |

The matrix layout is in `byte-coverage-gaps.md`: u16 type, then
translation and the 3×3 matrices. The remaining 24 bytes of the prefix are
the start of the first 3×3. Mode flags are in the first word.

## J. Actor DSG variables (18; from generated AI under `godot/generated/hype/levels/manoir/ai/models`)

**Hype, `world/0/0`, fix/28**

| Id | Var | Saved | Meaning | Evidence | Confidence |
|---|---|---|---|---|---|
| `0:17:15419` | dsg_115 (u8) | 5 | base attack damage per stat tier | Set by tier in `fix/535.gd:191-488` and `154.gd:2027-2036`: 50/25, 40/20, 30/15, 20/10, 10/5, the higher value when Global dsg_16 == 1. Doubled by the cheat at `fix/28.gd:27356`. Copied into dsg_114 at `fix/28.gd:10986,12714`. | inferred, medium |
| `0:17:15540` | dsg_57 (bool) | 0 | alternate weapon channel latch | From Global dsg_45 at init and from Global dsg_73 == 4 in Reflex 7. 121 tests. | inferred, medium |
| `0:17:15586` | dsg_11 (bool) | 0 | Object 44 outfit latch | `fix/28.gd:790-823`: `select_object_payload(44)`, parts 6..11 → 341..346. | inferred, medium-low |

**Global, `fix:/persos/15`, fix/1**

| Id | Var | Saved | Meaning | Evidence | Confidence |
|---|---|---|---|---|---|
| `0:17:4384` | dsg_29 (vec) | (−74.379, 27.487, 4.689) | checkpoint position | Written by checkpoint 348 at `348.gd:137` with `source_position()`. Read at `fix/28.gd:1233,1280` to place iHypeStatue. | inferred, medium-high |
| `0:17:4408` | dsg_31 (vec) | (−71, 26, 7.5) | checkpoint camera position | Written at `348.gd:153`. Read at `fix/688.gd:577`, which positions StdCamer. | inferred, medium-high |
| `0:17:4557` | dsg_152 | 1 | mannequin slot-match gate | Written by 154 and 991, read by 939. | partial |
| `0:17:4560` | dsg_149 | 1 | grille / IWorld sequence latch | Models 154, 195, 272. | partial |
| `0:17:4563` | dsg_146 | 0 | fixed-camera "state 3" signal | Model 397 writes it 10× and reads it 2×; also 154 and 195. | partial |
| `0:17:4649` | dsg_66 | 0 | request for Gogood return behavior 10 | Read by 1251, 154, 195, 736. | partial |

The checkpoint values sit near the saved Hype and camera POS translations
in group I, which supports the dsg_29 and dsg_31 readings.

**Other actors**

| Id | Actor | Var | Meaning | Evidence | Confidence |
|---|---|---|---|---|---|
| `1:17:127660`, `131192`, `132112` | i_Mannequin_Arbalete/Epee/Magie (939) | dsg_2 | latch | Set at `939.gd:544` when Hype dsg_0 == 1 and own dsg_0 == 3. Read at 661. Reset and set by 154. All three are 1. | low |
| `1:17:171448` | i_Gogood_RetourManoir (1251) | dsg_0 | mode 1/3/2 → behavior 6/7/9 | TSV note | medium |
| `1:17:187070` | iLevierSecretManoir (1396) | dsg_3 | selects one of two action-state maps in Normal 2 | TSV note | medium |
| `1:17:73944`, `82360`, `83312`, `84264` | barrels 2/1/3/8 (488) | dsg_8 | contact zone overlaps a type-2 zone | `488.gd:824-829`: `dsg_8 = collide_perso_zde_no_with_type_zde(1,2)`. Normal 0 (`488.gd:59`) deactivates the barrel when dsg_8 == 1 and dsg_4 < 99. This contradicts the "unknown barrel-local" in `manor-four-dsg8-followup.md`. | inferred, medium-low |

## Still unknown, and why

- **A, B, C:** no writers found, so the 55/43 saved ones and the
  destinations can't be traced. Per-row labels need the absent `setting`
  file. Candidate: they may be set through the save-registry restore or
  through untraced dispatch code.
- **D:** category names (apart from Fire) and the type of the 0x16 handle
  node.
- **E:** what the four children are.
- **F:** every bit except 0 and 7.
- **G:** the panel names.
- **H:** the high byte of the magic word, and a gameplay name for the
  ToolCam tag.
- **I:** the rest of the matrices; the camera owner is not proven.
- **J:** all the rows are AI-usage inferences, with no engine-side names.

## Checklist lines

- [ ] Save fields: the 100 map icon flags (`DAT_00750900[n]+0x30`) show icons on the current epoch's map. Name them as map_icon_visible[n]; per-row labels need the authored map `setting` file.
- [ ] Save fields: the 60 map label flags (`+0x38` of the `*` rows) reveal text labels, and the 10 destination slots index the icon table (−2 = none).
- [ ] Save fields: 38 inventory SuperObject data links are 7 category handles plus 31 slots. Name the categories (weapons, objects, 3 magic, restore, ?).
- [ ] Save fields: the gauge selector, selected magic slot and ToolCam mode word are read from decomp. Promote them with the high byte and the gauge names left open.
- [ ] Save fields: Global dsg_29/dsg_31 are checkpoint Hype and camera positions. Hype dsg_115 is base attack damage. Barrel dsg_8 is the zone-overlap latch. Confirm with one checkpoint save.
- [ ] Save fields: CustomBitsInit bits other than 0 and 7 have no engine names. Keep them as a verbatim u32.
