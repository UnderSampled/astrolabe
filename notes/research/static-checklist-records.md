# Static checklist items: unnamed record types and other static data

Static research, 2026-11-03. Not covered here, because each has its own
note: action-return policies, the unsupported AI entries, byte-coverage
gaps and unknown save fields.

Method:
- Diagnostics come from `target/release/astrolabe import-openspace` runs
  into kip-zephyr scratch, read-only on `disc/`. The .rete outputs were
  decoded with a scratch msgpack reader.
- Raw .sna scans used the same helpers as the other notes.
- Decomp evidence is from `hype-decomp/reference/ghidra/functions/*.c`,
  and from objdump where bodies are missing.

Markers: [read] is traced in decomp or raw bytes. [inferred] is reasoning,
or OpenSpace/Raymap prior knowledge.

## Counts are stale

The current binary doesn't reproduce the checklist figures. With
`--content scene`, Manoir reports 203 diagnostics; menu, pause-menu,
inventory, hud, textures and audio report 0, and model reports 4. I didn't
find the command that produces 378.

| Gap | Manoir checklist | Manoir now | Brigand now |
|---|---|---|---|
| "animation morph data" (`openspace.character.coverage`) | 47 | 43 | 50 |
| "spatial acceleration data" (`openspace.scene.model_coverage`) | 30 | 20 | 80 |
| mechanics kinds with preserved fields | 6 | 6 | 7 |
| sector domains without codecs (`openspace.sector.coverage`) | 6 | 0 | 0 |

Brigand only:
- 10 "empty physical geometry omitted": objects 38337, 38388, 220886,
  220949, 221012, 221075, 221138, 222145, 222181, 222190.
- 7 `opaque_designer_field` warnings.
- 16 of 48 dialogue items unsupported. Manoir has 7 unsupported.

## 1. "Animation morph data": really the animation light table

The diagnostic is raised at `src/openspace/level/character.rs` (~767)
when `related != 0 || unknown_108 != 0 || morph_count != 0`.

[read] Every converted animation has `morph_count == 0`, the +0x0c morph
pointer 0 and +0x6c 0. Only `related` (+0x70) is nonzero:
- Manoir has 43 such animations, 42 Fix plus 1 level.
- Brigand has 50, the same 42 Fix plus 8 level.
- These are exactly the animations with a Light channel (channel kind byte
  +0xc == 4).

**+0x70 light table, used by GAM_PlayAnim_Play 004330f0:91-95 and
243-252 (checked).**
- Layout: 12 bytes, `{u32 lights_ptr, u16 count, u16 pad, u32 nodes_ptr}`.
  - lights_ptr is `count` light handles.
  - nodes_ptr is `count` pointers to 16-byte sector DNM-light list nodes
    `{handle, next, prev, header}`.
- Each frame the player:
  - calls `GLI_xSetLightState(lights[i], 0)` on every entry;
  - then, for each kind-4 channel, takes the channel's u16 at +4 as `idx`;
  - calls `GLI_xSetLightState(lights[idx], 1)`;
  - calls `SECT_fn_vSetHandleOfLightsInDNMLightsList(nodes[idx],
    lights[idx])` (004dea30, which does `*node = light`).
- Raw check: in the Manoir level animation at 5:1 +0x87728, count is 6 and
  channel 2 has kind 4.
- In short, the animation turns one of N lights on per frame and makes it
  the sector's dynamic light.

**+0x6c**, only test (004330f0:100):
```c
if (anim+0x6c && (frame & 1) && frame != frame_count-1) FUN_00443190(actor, 1)
else FUN_00443190(actor, 0)
```
This stores the result at actor state +0x1b0. Meaning unknown; it's zero
everywhere in both levels.

**+0x0c real morph list.** `morph_count` (u16 at +0x12) 6-byte entries,
`{u16 key, u8 channel, u8 frame, u16 value}`. FUN_00432eb0(anim, channel,
frame) looks up by channel and frame; field meanings are [inferred].
FUN_00432f60 is a stub returning 0. There are no instances in either
level.

## 2. "Spatial acceleration data": the collision octree

The diagnostic is raised at `src/openspace/level/import.rs:144` when
`GeometryWire.spatial_data` (+0x20) is nonzero. The sector boundary
reader bails on it.

[read] `COL_fn_bCreateOctree` (0055fd20) allocates 0x38 bytes into
geometry+0x20, but only above a face-count threshold. Below the threshold
it frees the block and builds the edge list instead.

Header (0x38):

| Off | Field |
|---|---|
| +00 | root node ptr |
| +04 | tagged-points table (point_count×4; zero in file, filled at run time) |
| +08 | i16 face count |
| +0c | tagged-faces table (zero in file) |
| +10 | element face bases, u16 each (cumulative: 0, 13, 18, 20 in the sample) |
| +14, +18 | cleared by COL_fn_vCreateTagsTables; +14 is sometimes 0x37 in the file (unexplained) |
| +1c | face list: u16 used count, then count×4 |
| +20 | bbox min, 3 floats |
| +2c | bbox max, 3 floats |

Node (32 bytes, `COL_fn_pstConstructRecursiveOctree` 0055f4d0):
- +00 min, +0c max;
- +18 pointer to 8 child pointers;
- +1c leaf face list: u16 count, then {u16 element, u16 face} pairs.

The consumers are the COL octree explorers (box, segment, semi-axis and
shadow: 00560e80, 005610e0, 005612e0, 00561500) and the dynamic-vs-static
octree collides (00565860, 00567030, 00569d00, 0056afa0). The octree only
speeds up collision and shadow queries, so skipping it changes no results
[inferred]. Godot builds its own acceleration structures.

**Unreported edge list in `unknown_tail` [read].**
- +0x24: u16 edge count (0046fe70).
- +0x28: u16 pairs.
- +0x2c: per-edge material pointers.

It is built by 00470040 and 00470710, and freed in 0046d380. Present in 7
small Manoir geometries (3–8 edges) and 5 in Brigand.

Also:
- Geometry +0x10 (`unknown_10`) is zero everywhere; meaning unknown.
- `GEO_xComputeObjectSphereBox` (0046c900) is a stub.
- The bounding sphere is computed on demand (00466bf0).

## 3. Mechanics identity cards with preserved fields

The diagnostic is raised at `character.rs:457`. `mechanics.rs` decodes
kinds 0/4/18 (character) and 9 (mass); the rest stay `Unresolved`.

[read]
- Layout and defaults come from `DNM_fn_xIdentityCardCreate` (00591464).
- The handler table is at `DAT_00757320`, built in FUN_00443540.
- The getters 00595599..00595e72 give the field offsets.

The "meaning" column is [inferred] from the handlers and the families that
use each card.

| Kind | Size | Fields (file values) | Update / control | Used by | Meaning |
|---|---|---|---|---|---|
| 3 | 0x10 | w1 16.001, w2 0.0183, w3 1.0 (f32) | 578020 / 57910c | Fix family Camera (card 196782) | camera dynamics |
| 6 | 0x5c | memset only, all zero | 588198 / 58ceea (`lift_control.c`, events 14/15) | Manoir "icon", Brigand "ph" | gate/lift mover; its endpoints are runtime state |
| 7 | 0x34 | defaults w1 100, w5 20, w6 1.746, w9 1.0; ratio +0x28; viscosity +0x2c (630); stiffness +0x30 (980). File: 100, 50, 1.0, 630, 981 | 58829c / 58d4d7; extra init in 0059dd97 | no family state refers to it | spring-damper |
| 11 | 0x14 | f32 +4 (default 0.5); ratio +8 (0); byte +0xc (1); "crazy path" f32 +0x10 (−1.0). Fix "xi": 0, 0, 0, −1; Brigand "dague": 1.0, 0.75, 0, −1 | 5898f3 / 58efd2 | Fix "xi", Brigand dagger (family 26) | thrown or bouncing body; 0.75 looks like restitution |
| 19 | 0x08 | one u32: 0 (card 135571) or 1 (card 27270) | 57e472 / 57e110 | NPC families (xw, villageoise1, astro) | flag, meaning unknown |

Manoir has 6 cards (5 Fix + level kind 6). Brigand has 7 (5 Fix + level
kinds 6 and 11). The per-kind reference-frame table `DAT_0060dd90` is zero
for every kind except kind 10, which has mode 2.

## 4. "Sector domains without codecs"

Raised in `lighting.rs` (~100-117), only for list 1 (surface) and list 5
(stream) when non-empty. The current binary reports 0 in both levels, so
the checklist wording is stale: environments are decoded now, and older
logs had surface: 1 and environment: 1.

Sector (160 bytes, `SECT_fn_hCreateSector` 004dd7c0; list getters
004e0690..0800) [read]:
- The border pointer is at +0.
- 12 list headers of 12 bytes follow:

  | Offset | List |
  |---|---|
  | +0x04 | environment |
  | +0x10 | surface |
  | +0x1c | character |
  | +0x28 | static lights |
  | +0x34 | DNM lights |
  | +0x40 | streams |
  | +0x4c | graphic |
  | +0x58 | optional graphic |
  | +0x64 | collision |
  | +0x70 | activity |
  | +0x7c | sound |
  | +0x88 | sound event |

- After the lists come the virtual flag, the activation word and the name
  pointer.

Surface lists [read]:
- List nodes are 16 bytes (004dee10).
- The surface object is 0x10c bytes (`SRF_fn_hCreatesSurface` 004ffde0):
  two 0x58 matrices at +4 and +0x5c, and an f32 offset at +0xb4.
- Height is the translation plus the offset (005001e0). `SCT_hGetSurface`
  (004e2170) walks the list.
- So these are horizontal planes, i.e. water [inferred].

The stream layout is by analogy only (004e2240) [inferred].

## 5. Designer variable types without codecs (Brigand, 7 fields)

The warning is raised at `level/ai.rs:1046`.

Fields:
- Types 9 and 13: models 927/1060, fields 45, 46, 51 and 52.
- Type 19: model 1060, field 46.
- Type 9: model 1862, field 10.
- Type 26: model 444, field 1.

From `fn_ulSizeOfDsgVar` (005236a0, `designer_variable_storage.c`) and
the init switch (`designer_variable_initialize.c` ~170-300) [read]:
- 9: one byte, atoi, default 0xff.
- 13: 4-byte link pointer into the list at 0x71d038.
- 19: array of type 10 (links), count×12+8 bytes. Types 20–25 are arrays
  of 12, 7, 5, 8, 15 and 14.
- 26: past the 26-entry size table, so size 0.

`DesignerValueCodec::supports` lacks 9, 13, 19 and 26.

Fields 45, 46 and 51 of 927/1060 are the comport variables used by the 22
dynamic 4-209 requests in `unsupported-ai-entries.md`. If those are type
13, decoding them would let the converter see the possible comports
[inferred link; check the field types].

## 6. Dialogue answers

Raised at `dialogue.rs:310`: items with `answers != 1` or a box style
outside 1/3/4/5 are unsupported.

Brigand:
- 6 `wellet_*` and 3 `brigand_zatila_speak_4..6` have 2 answers, style 1.
- 6 (potions, `herbe_jaune`, `fleches_rouges`, `trouvenoix`) fail only on
  style 5's single unvoiced untimed page.
- `brigand_monast_plaque_1` has an unsupported line layout (alignment
  [1,0], offset 1.8).

[read] Selection is `DLG_fn_vSelect` 0056c690 and FUN_0056e3b0
(`dialogue_selection_transitions.c`).

Dialog object:

| Off | Field |
|---|---|
| +0 | entry count |
| +1 | selected |
| +2 | selector byte |
| +4 | entries |
| +0x10 | continuation |
| +0x14 | mode (1 = scrolling, FUN_0056e510) |

Answer entry:

| Off | Field |
|---|---|
| +0x08 | flag 1 = one-shot: moved to the end and the count reduced after use |
| +0x18 | branch dialogue |
| +0x1c | continuation, copied into dialog+0x10 |
| +0x20 | visible/current |
| +0x28 | line list |
| +0x3c | speech |
| +0x40 | u8 mapping count |
| +0x44 | 8-byte entries `{u8 code, ptr next_dialog}` |

Result handling: FUN_0056e2c0 returns a code.
- −1: take the branch, apply its modules (FUN_0056e800) and start the
  main actor's speech.
- Otherwise the code is looked up in the mapping table. With a single
  mapping or no match, the first mapping is used. A null table restarts
  the same dialogue.

Drawing: FUN_0056dd50 draws the choice marker, outline and icon (via
FUN_00418f60(icon_id)).

Unknown:
- condition ids 120..122 (`dialogue_dispatch.c`);
- what produces the result code;
- the channel show/hide effects, which are read but not applied.

## 7. Controls: the original's action list and default keys

Not in the Options root. The list is in Fix block 7:0 (75,217 bytes), the
input module's entry-action table, which `menu.rs` doesn't read.

[read]
- User bindings live in the scrambled `Gamedata/Options/Default.cfg` (616
  bytes), handled by `IPT_fn_bLoadOptionsFromFile` 004a7330 and
  `IPT_fn_bSaveOptionsInFile` 004a71f0.
- Records come through `IPT_fn_vLoadBinaryFile` 004a5060 (IPT_Bin.c),
  0x1c bytes each.
- In Fix 7:0, each entry name is followed by a 16-byte record
  `06 f0 00 00 | 0 | scancode | 00 00 ff 00`.

Keyboard defaults (scancode names are standard DirectInput [inferred]):

| Action | Key | Action | Key |
|---|---|---|---|
| Left/Right/Down | ←/→/↓ (0xcb/0xcd/0xd0) | Up | not matched (presumably 0xc8) |
| SpeedUp | LShift 0x2a | Jump | LCtrl 0x1d |
| Action | Space 0x39 | Inventory | Tab 0x0f |
| Lateral | LAlt 0x38 | Look | Numpad0 0x52 |
| Magic | Enter 0x1c | ScrollMagic | Delete 0xd3 |
| Save / Load | F3 0x3d / F2 0x3c | GetSword / GetCrossbow | X 0x2d / Z 0x2c |
| AlwaysRun | CapsLock 0x3a | SSLeft / SSRight | A 0x1e / S 0x1f |
| Shield | P 0x19 | Activate_Map | End 0xcf |
| SkipCinema | Backspace 0x0e | RaccroucisVie / Magie / VieMagie | 1 / 2 / 3 |

Pad entries: Up/Down/Left/Right, SpeedUp, Jump, SideStep, ModeLook,
Action, Magic, ScrollMagic, Arme and Inventory (`_Pad`). There are also
analog-button, menu and inventory navigation, and debug/cheat entries.

## 8. Sector AI activation (static rule) [read]

FUN_00448280, run per actor:
- `SCT_fn_vTestChangeSectorWithHistory`;
- on a sector change, it updates the actor's sector;
- for the main actor it then calls FUN_00437b10.

FUN_00437b10 (sleep): for each sector S in the old sector's activity list
that is neither the new sector nor in the new sector's activity list, it
visits each character in S. It sends `FUN_00437590(char, 5)` (event 5 to
all components) when:
- StandardGame +0x24 is 0 or 1, and
- +0x1b != 0 or +0x20 == 0.

Wake: FUN_00449cd0(sector) and FUN_00449c60. For each character in the
sector and in its activity-list sectors:
- an inactive character is activated (FUN_00443080);
- otherwise the active byte is set (FUN_00438fa0, StandardGame +0x1c;
  +0x1b suppresses it, see `actor_activate_00437740`).

The caller of the wake isn't found.

This explains the earlier failure of "run only sectors in Hype's activity
list": the original only touches sectors that leave the set at a sector
change, and it exempts actors by those StandardGame fields.

## 9. Brigand stand-in breakdown

There are 344 `stand_in(` lines in the current generated Brigand output;
the checklist says 383.
- 182 are `generic-untyped`.
- 101 of those are action-return stand-ins, and 30 are dynamic comport
  requests.
- Named entries are from `hype-ai-api.tsv`:
  - 4-176 Procedure_Camera_SetTargetPosition: 5.
  - 4 each: 4-438 Magic_SelectNextSpellType, 4-334
    Proc_ActivateInventoryMotorMode, 4-374 Proc_ActivateMapMotorMode, 4-432
    Fight_SubHitPointsEnemy, 4-350 Proc_GPI_SetImage, 4-349
    Proc_GPI_SetDraw, 4-357 Proc_GPI_SlideGamePlayInterface.
  - 3 each: 4-175 Procedure_Camera_SetCameraPosition, 4-215 Proc_Turn2.
  - 1–2 each: inventory (InventoryIsFull, RemoveObject, AddGold,
    FindObject), waypoint network (IsValidWay, InitWayWithWp, Reseau*),
    camera (SetPosition, ShowInfo, WaitUntil), map (SetDestination,
    EnableObject), SetTarget, ReachTarget, ElapsedTime,
    CapsSetCapabilities, SetHitPointsToInit, ActivateSaveGameMotorMode,
    IncrementalTurn, GPI_MovingUV.

## Not identified

- animation +0x6c;
- geometry +0x10;
- octree +0x14 = 0x37;
- field names for mechanics kinds 6, 7 and 19, and FUN_0059dd97;
- the caller of the sector wake;
- DSG type 26;
- dialogue condition ids 120..122 and channel effects;
- the Up_Key default;
- the command behind the checklist's 378 / 47 / 30 / 6 / 6.

## Checklist lines

- [ ] Every record type is named: refresh the counts. The current import reports Manoir 43 / 20 / 6 / 0 and Brigand 50 / 80 / 7 / 0 for animation / geometry / mechanics / sector gaps.
- [ ] Animation light table: 43 Manoir / 50 Brigand animations (all with a kind-4 Light channel) carry a 12-byte table at header +0x70 (light handles, count, DNM-light node pointers). GAM_PlayAnim_Play 004330f0 switches the indexed light on each frame and installs it in the sector's DNM list. Rename the "morph" diagnostic; the real morph list (+0x0c/+0x12) and +0x6c are empty in both levels.
- [x] Geometry spatial data is the COL octree (COL_fn_bCreateOctree 0055fd20; 0x38 header + 32-byte nodes): acceleration only, safe to skip (20 Manoir / 80 Brigand). Also claim the edge list at +0x24/+0x28/+0x2c.
- [ ] Mechanics cards 3 (camera), 6 (lift), 7 (spring: ratio, viscosity, stiffness), 11 (thrown body: ratio, crazy path) and 19 (flag) have layouts from DNM_fn_xIdentityCardCreate 00591464. Decode them by name.
- [x] Sector surface and stream lists are empty in every Manoir and Brigand sector. Surfaces are height planes (SRF 004ffde0), if needed later.
- [ ] Designer types 9 (byte), 13 (link pointer), 19 (link array) and 26 (no size) need codecs: 7 Brigand fields, possibly including the comport variables behind the 22 dynamic behavior requests.
- [ ] Branching dialogue answers: entry layout and result mapping from 0056c690 / 0056e3b0. Brigand has 9 two-answer items (6 wellet_*, 3 zatila_speak_4..6).
- [x] Controls page: the action list, defaults and key names are read from Fix 7:0 (`controls()` in menu.rs, 2026-11-04; notes/research/controls-menu-original.md). Up_Key is 0xc8.
- [ ] Sector activation: implement FUN_00437b10's sleep rule at the main actor's sector change, plus the StandardGame +0x1b/+0x20/+0x24 exemptions. Wake via 00449cd0/00449c60.
