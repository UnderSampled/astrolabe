# Inventory, map, save, menus and manor cheats: two-layer checklist

Static research, 2026-11-04. Nothing was run. It extends
[inventory-menu-checklist.md](../../docs/astrolabe/inventory-menu-checklist.md)
(every item there is kept, re-filed below), [stand-ins.md](../../docs/astrolabe/stand-ins.md)
and [static-checklist-records.md](static-checklist-records.md).

Layers, one line per row:
- **Data** = source records (Fix block, record kind, offsets) and whether Astrolabe decodes them
  into Rete (asset kind, src file), plus the runtime memory map (engine globals and structs,
  writers and readers, hype-decomp addresses). `[x]` = decoded or mapped and used; `[ ]` = not
  decoded, not mapped, or mapped here but unused by the port.
- **UI** = what the original does (motor-mode state machine, input, drawing, sound, timing)
  against what the port does now (godot/game, godot/addons/astrolabe/ui, `src/godot`). `[x]` = port
  matches the source with evidence; `[ ]` = stand-in, guard, absent or unverified.

Markers: [read] traced in a decomp body or the PE bytes; [gen] read in generated GDScript;
[doc] taken from an existing Astrolabe doc without re-checking; [inferred] reasoning only.
Addresses are PE virtual addresses (`ed105a42...` build). Raymap was not consulted.
Abbreviations: `inv.rs` = src/openspace/level/inventory.rs, `invg.rs` = inventory_gameplay.rs,
`F28` = godot/generated/hype/levels/manoir/ai/models/fix/28.gd (Hype), `F535` = .../fix/535.gd
(cheat controller), `M154` = .../ai/models/154.gd (IWorld_Manoir), `M348` = .../348.gd (checkpoint/lectern).

## Inventory

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| I1 | Catalogue and item handles (62 items) | [x] Fix 10:0 (10,589 B, 52.0% read): def +12 kind, +20 capacity, +25 availability, +28 info id; 24-byte handle +0 def, +8 selected, +10 qty i16, +12/+16/+20 replacement list; Rete `inventory-catalog` `fix:/inventory/gameplay` (invg.rs, src/model/inventory_gameplay.rs); runtime lists `DAT_006107b0` (items) and `DAT_006107b4` (categories), node +4 payload, +8 first child, +0x14 next [read 0041dab0, 0041d300] | [x] `character/inventory.gd` per-scene clone, signed-16 wrap and capacity clamp; AI reads 196/198/201/205/206/208/209/210 compile typed (src/openspace/ai.rs:1859-1981) |
| I2 | Category tree and admission (7 groups, 46 admitted items, 12 NotFound, 4 unbound) | [x] Fix 6:1 SuperObjects type 19 box, 18 item, 22 sprite set; stock pickup +8 category, +12 item, +16 count (0045bf00); admission 0041c720, search by id 0041c610 [doc inventory-gameplay.md] | [x] `inventory.gd` `add_object`, `select_*`; legacy sparse saves keep `legacy_owned_ids` |
| I3 | Replacement rules (21 rules on 15 items) | [x] 20-byte elements, 0041c869-0041ca10, Rete `InventoryReplacement` | [ ] ordered replacement implemented, native runtime untested [doc] |
| I4 | Presentation geometry, focus graph, colours, scales | [x] Fix.gpt slots 18152/18156/18160 + Fix.sda scalars after 20 B (0041bec0 writes, 0041c290 reads ~60 scalars into `DAT_00610598..006107e0`); Rete `inventory-presentation` `fix:/inventory/en` (inv.rs:20 `PresentationWire`) | [x] generated `inventory.tscn`, `ui/inventory/screen.gd` (five categories, three magic groups, six slots) |
| I5 | Fonts and frames (InventoryFont, 127 glyphs) | [x] Fix 19:1 (61,673 B; 74.7% read for inventory), reader dialogue_presentation.rs:88; Rete inside `inventory-presentation` | [x] native FontFile, fixed size set before name |
| I6 | Item labels (52) and counters | [x] chain from gpt slot 18124, Fix.dlg block 10:1 first region; counter 00419ef0 hides negative counts and availability 1 [doc] | [x] localized labels shown; counter rule in `screen.gd` |
| I7 | Per-item record gaps | [ ] 88-104 untouched bytes in each 10:0 record and 1,060 B at 0x22c4 (between inv.rs:254 and :331); likely description text, sound or sort order, unread | n/a |
| I8 | Open and close: motor mode 0x28 to 0x29 and back | [x] `DAT_00754420` is the pending event/mode (writer 00404ac0, dispatcher 00403fc0 case 0x28: 0044e800, 004055a0, 004ed500(`DAT_007543dc`), 0041b3e0 marks six handles (+0x18=1, +0x10=-2) `DAT_0075436c..00754380`, 00419060 initial boxes and timer `DAT_00610760`, callbacks `PTR_FUN_005e434c`=0041aa00, `PTR_FUN_005e4350`=00403f20); exit 0041b3c0 requests 0x2a, which returns to 9 [read] | [ ] original: AI rule `TestInventoryMode` in F28 reflex rules (4 sites) calls 4-334 (005437e0 requests 0x28). Port: Tab in `inventory_host.gd:_input`, pauses the tree, gate `inventory_available()` = ready and not widescreen; 4-334 is a stand-in |
| I9 | Entry guards before the screen opens | [x] Hype fields: `dsg_56==0`, `dsg_53==1`, `dsg_19==0`, `dsg_84==0`, life > 0, then `dsg_132=0`, `dsg_18=1` (F28:26985-27010 [gen]); else a refusal sound | [ ] port never evaluates them (`TestInventoryMode` is adopted with `KEY_NONE`, the AI rule cannot fire); Tab can open during other states |
| I10 | Exit handoff to the AI | [x] reflex rule `dsg_18==1` (F28:26797-26960): re-reads life, unselects Object 44 in forbidden behaviors, restores HUD indicators from `dsg_59`/`dsg_10`, fades in | [ ] never runs in the port (`dsg_18` stays 0), so the HUD and Object 44 restore do not happen after the screen |
| I11 | In-screen input and sounds | [ ] callback 0041aa00 is a `LAB_`, not a function in either export; key set, scroll sounds and `DAT_00610794/98` effects not read; refusal sound `*DAT_0061079c` in 0041d010 | [ ] port: arrows/Enter, Tab or Esc closes; no inventory sounds claimed |
| I12 | Weapon selection | [x] 0041d570/0041dc80 family, selected byte +8 | [x] `change_weapon` with admission message (tools/routes/key_menu.json) |
| I13 | Object selection and unequip | [x] 0041d890, 0041dd70, confirm handler 0041d010 | [x] `select_object` / `unselect_object` (key_menu.json) |
| I14 | Key end to end (Gogoud gate) | [x] M154 reads `selected_object_value()==21` and sets F28 normal 34 near `/150` | [x] stage4_gate_ui.json: Tab, slot, Enter, Tab, gate opens |
| I15 | Consumables (RestoreHP/MP/HPMP, Poison) | [x] 0041cd60, availability byte is the amount, count -1 unless `available` is 0xff | [x] `_use_consumable` (use_flask.json) |
| I16 | Magic spell selection | [x] 0041d010 `Magic*` branch, `selected_magic`, replacement upgrades | [ ] host has no `Magic` branch (`inventory_host.gd:_item_requested` falls to "not implemented yet"); `Magic_SelectNextSpellType` 4-438 is a stand-in, `Button_MagicScroll` unbound |
| I17 | Menu motion timing | [x] Fix.sda offset 272 `[96, 0.3, 50, 0.7]`, applied by 0041b500 [doc] | [ ] timing built from AnimationPlayer, not compared with the original |
| I18 | HUD hidden while open | [x] 0041aa00 never calls 0040c1f0 [read in checklist] | [x] `set_inventory_open` hides HUD (take frame 284) |
| I19 | Stone panel meters: sword gauge, gold, game timer, armor, ring | [x] drawn by 0041aa00 (0040c300, 0043a6a0, 0043ac60, 0043ab40) over rectangles from 00419060/0041c290 (`PresentationWire` 320-376); gold list element +10; clock `DAT_007474d8` | [ ] port draws the panel empty |
| I20 | Armor on screen and HUD | [x] Rete `armor-catalog` `fix:/gameplay/armor` (armor.rs:181); geometry 0043aa70, 0043ab40 | [ ] piece choice uses Godot RNG (variants.md), repair touches only the active armor, port saves piece +0x24 |
| I21 | Inventory-full flag | [x] byte at `*DAT_00610668` (GPT pointer read in 0041c290): set to 1 when no slot is free (0041c720:69, :79), cleared in 0041cb70:68/102; saved with the inventory | [ ] 1-118 is a stand-in (always false, F28 2 sites); Resource keeps no full flag |
| I22 | Find and remove object | [x] 1-119 helper 0041d300 (walks `DAT_006107b4`, kind 0x13 = box group), 3-197 0041cb70 | [ ] 1-119 and 3-197 stand-ins (F535: Galarneau ad cheat; Brigand 599.gd); `inventory.gd` already has `quantity`/`remove_source` |
| I23 | Change weapon/magic by AI (3-202, 3-203, 3-204) | [x] 0041d570, 0041d660, 00539b60 cases | [ ] no typed arm in src/openspace/ai.rs (no manor site seen) |
| I24 | Jewel state and stock list (4-419, 4-420) | [x] 0041de20, 0041de00 (recovered, rewritten) | [ ] 4-419 reaches `generic-4-419` (13 sites, 3 levels); 4-420 compiles as `SetStockCount` |
| I25 | Pickup display (4-421 Inv_DisplayAnObject) | [x] eight operands to 0041df20 | [x] `Statement::DisplayPickup`, `pickup_feedback.gd`, `pickup_artwork.tres` |
| I26 | Max capacity changes (4-418, 3-210) | [x] 0041db30 (-1 = gold), 0041dab0 | [x] `SetInventoryMaximum`, `maximum_capacity` |

## Gold

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| G1 | Balance | [x] special resource root `DAT_006107bc` +8 -> +4 element (i16), definition kind null (invg.rs, `InventoryDefinition`); 0041d400 read, 0041d420 add (clamps to max, returns 0 when clamped), 0041d490 sub (fails when request > balance) | [x] `gold`, `add_gold`, `subtract_gold`, `gold_value` in `inventory.gd`; pickup of the gold category adds with signed-16 wrap; 3-198 compiled |
| G2 | AI add and subtract (3-199, 3-200) | [x] both share 00539b60 (rewritten) | [ ] 3-199 stand-in (F535 `frik`; the random operand is dropped), 3-200 has no typed arm and no manor site |
| G3 | HUD cash counter | [x] flag `DAT_0060f5cc` (4-428, no panel refresh); draw 0040c950 slot 5 row 23, `gold_icone_nz.tga`, Rete `hud-presentation` `fix:/hud/en`; manor save has no item-0 quantity | [ ] counter uses the item-0 quantity; original-save currency restore not established [doc ui.md] |
| G4 | Gold on the inventory panel | [x] same list element, printed through 00419610 in InventoryFont | [ ] not drawn (I19) |

## HUD handoff to inventory

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| H1 | Indicator flags | [x] byte flags `DAT_0060f5bc` life, `f5c0` armor, `f5c4` magic, `f5cc` cash, `f5d0` weapon, `f5d4` selected object, `f5d8` dragon; written by 4-422..428 (0053dbb0, rewritten), refresh `0040c150(panel)` fills `DAT_0060f5ec+panel*0x10`; read by HUD frame 0040c1f0; cleared by 0040c0a0 | [x] `Statement::SetHudVisible` for 422-426 and 428 -> `source_set_hud_indicator_visible` -> `hud.gd`/`hud_slots.gd` |
| H2 | Dragon indicator (4-427) | [x] flag `DAT_0060f5d8`, panel 2, draw 004606a0/004608c0 | [ ] no typed arm: the SetHudVisible arm is `422..=426 | 428`, so 427 falls to the generic path |
| H3 | Selected Object slot | [x] row 22 (269,404, 48x60), 0040cb50, panel 3 frame | [x] `hud_slots.gd` Object rect and Panel3 |
| H4 | Selected Weapon slot and count | [x] row 10 (10,360, 48x48), 0040ca40, count via FON 3D projection | [ ] `hud-count-offset` stand-in |
| H5 | Armor icons and magic arc | [x] 0043ab40, 0043a450, 0043a6a0; magic words at `*DAT_00610b98` +0x22 init, +0x24 current, +0x26 max (0053de60, 00536550 rewritten) | [ ] drawn in `hud_slots.gd` but panels 0-2 gating untraced [doc ui.md] |
| H6 | Fix HUD layout (four panels, six textures) | [x] Fix.gpt 0x4a38 panels, textures 0x4a78, Fix 5:0/6:1 [doc ui.md]; Rete `hud-presentation` | [x] editable `Source/Panel0..3` |

## Map

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| M1 | Map-name table (60 names) | [x] Fix.dlg 60 words + Fix 10:1 (5,037 B, 76.2% read; 1,188 B before the first name); 60-byte record: label[40], position i32x2 @0x28, id @0x30, unknown @0x34, active @0x38 (map_names.rs:16); Rete `scene` field `map_names`; runtime 60 pointers `0x00750ae0..0x00750bd0` | [x] 4-376 Proc_ActivateName -> `ActivateMapName` -> `ui/map_names.gd:activate_name` (matches id, same as 00416310) |
| M2 | Drawing the names and the map | [ ] 10:1 bytes unread; the 60 positions are decoded, map artwork is not | [ ] no map screen exists; names are state only |
| M3 | Map objects (100) | [ ] pointer array `0x00750900`, flag word +0x30 (4-270 -> 00545c70 -> 00416260); the records and their source block are not read | [ ] 4-270 is untyped (`generic-4-270`, 1 manor site, 38 sites over 6 levels) |
| M4 | Destination slots (10) | [x] 10 signed words behind pointer `0x00750ac4`, -2 = empty; ids >= 100 ignored; duplicate ignored (4-271 -> 00416280, 4-272 -> 004162c0); not saved or decoded, runtime only | [ ] 4-271 and 4-272 are stand-ins (manor 1251.gd, Brigand 2026.gd and 599.gd; Brigand entry cutscene reaches 4-271 once) |
| M5 | Epoque (era) flags and maps | [x] four words behind `0x00750aa8` (4-375 -> 0053f780 -> 004162f0); era byte from 00448380 (1, 2, 4, 8) picks `map_epoque1..4.bmp` in 00416e70; archive holding the BMPs not found | [ ] 4-375 has no typed arm; no era-dependent map |
| M6 | Map motor mode 0x1c to 0x1d | [x] 4-374 -> 005437e0 -> request 0x1c; 00403fc0: 004ed500, 004055a0, 00416e70 (four controllers `DAT_00750a90`, load era BMP via 0050e820), 00416fd0 (flags `DAT_0075438c/90`), callbacks 004175d0 and 00403f20 [read] | [ ] stand-in "map does not open"; F28 rule (4 sites): `Button_ActivateMap` && `dsg_56==0` && `dsg_53==1` && Global `dsg_233==1` && `dsg_19==0`, then `dsg_132=0`, `dsg_77=1`; key unbound |
| M7 | Map screen drawing and input | [ ] callback 004175d0 is a `LAB_`; unread | [ ] none |
| M8 | Exit handoff | [x] reflex rule `dsg_77==1` (F28:~26830): actions 47/48 by `dsg_59`, ends dialogue, clears `dsg_18`/`dsg_77`, fade | [ ] never runs without M6 |

## Save

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| S1 | Save-file format | [x] `General.sav` (8 B), `General000.sav` (text: level, title, 64-bit ms clock), `Level000.sav`, `<level>.sav`: 5-byte header + XOR + 256-window codec, SAI records (block, module, offset, flags, type); masks 0x0100/0x0200 player, 0x0400/0x0800 level (src/openspace/save_game.rs, crates/orrery/src/save.rs) [doc] | [x] original slots import to `godot/game/saves/SaveGameN.cfg` (save_import.rs, save_file.gd) |
| S2 | Field coverage | [ ] 248 Manoir save identities have no meaning (unknown-save-fields.md); inventory chain via 004a2310/004a2340 | [ ] unmapped fields are kept under `unknown/` and not restored |
| S3 | Save lectern trigger | [x] actors `iCheckPoint_Manoir_MAD` run `level:/ai/models/348`: `Button_Hit`, Hype `dsg_19==0`, `dsg_14==0`, distance < 2 and z < 1, mutual facing < 45 degrees, then Hype `dsg_133` = lectern, Hype normal 81 [gen] | [x] compiled; guards for missing bindings only |
| S4 | Checkpoint pose | [x] M348 stores Hype `dsg_29` position, `dsg_30` sighting, `dsg_31` = `dsg_1`; `dsg_1` is set by a vector add the compiler cannot type | [ ] `generic-untyped` stand-in at M348 node 83, so the stored offset stays zero |
| S5 | Hype save sequence (normal 81) | [x] F28:21552-21790: reflex 2, HUD hidden, actions 91 then 65, fade out, 4-336, fade in, action 273, `dsg_37=0` | [x] compiled up to the stand-in |
| S6 | Save motor mode 0x25 to 0x26 | [x] 4-336 -> 005437e0 -> 00413a80(0x25) (frees `DAT_00610468/6c`) then 0040f840 = 0040e550 (marks nine input handles `DAT_00754340..`: +0x18=1, +0x10=-2), 00413ad0(`DAT_00752418`) pushes menu page `DAT_00752418` (24-byte node on `DAT_0061048c`, remembers `DAT_005e6bd8` and callbacks), 00406b30; 00403fc0 case 0x25: 0044e800, 004055a0, 00413bd0 (strings `ReverseSound`, `MenuSound`), callbacks 004148c0 and 00403f20 [read] | [ ] stand-in effect `source_game_session().call("save_game")` (stand_ins.rs:131): saves at once, no menu, no slot choice, no confirmation |
| S7 | Save page input and back | [x] per-frame 0040dca0 state 0x26: back action `DAT_00754354`, pops with 00413fe0 (restores mode and callbacks) or requests 0x27; selection 00414b00 (871 lines, action names `SavedGames`, `Resume_Game`, `NextGame`, `Options`) | [ ] n/a without S6 |
| S8 | Native save write | [x] `user://manoir-save.cfg` in the manor, `user://travel-save.cfg` elsewhere (save_state.gd:6, :37) | [ ] F5 key; `travel-save` stand-in outside the manor; a New Game session cannot save until the opening ends |
| S9 | Scripted saves (4-99, 4-100, 4-101) | [x] 005417d0 (text handle to 00431130; Proc100 first 00431090), 00541850 (writes "AI_SaveGame") | [ ] no typed arm; none seen in manor or Brigand [doc full-game-ai-sites.md] |
| S10 | Load Game | [x] main-menu route, picker lists native and imported slots | [x] `load_game_picker.gd`; original quick-load key F2 and quick-save F3 unbound |
| S11 | Original-save placement | [ ] saved matrix not read; `load-placement` stand-in | [ ] placement measured 2 units from the checkpoint |

## Main and pause menus

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| P1 | Main menu, Load Game picker, credits | [x] Fix 18:0 (117,681 B, 97.9% read, 2,409 B gap at 0x1c248), Rete `menu-presentation` `fix:/menu/main/en`, credits entries; page stack 00413ad0 (`DAT_00610470` current page, `DAT_00610474` selection) | [x] `ui/menu/menu.gd`, `credits_menu.gd`, `load_game_picker.gd` |
| P2 | Options page | [x] source Options root (`config_back.bmp`), sliders (menu.rs:844) | [x] labels and artwork from source; controls replaced (see C1) |
| P3 | Menu text handles | [ ] Fix 19:0 24,269 B, 6.6% read; 92-byte records from 0x5150 | n/a |
| P4 | Pause menu | [ ] 18:0 read 31%; 407-byte records repeat from 0x9b1 and 11,832 B at 0x19d79 unread; Rete `menu-presentation` `fix:/menu/pause/en` (Resume Game, NextGame, Options, Main Menu) | [x] `pause_host.gd` on Esc: Resume, NextGame to `new_game.tscn`, Options, Main Menu |
| P5 | Pause motor modes 0xe/0xf to 0x10/0x11 | [x] AI 4-189/4-190 -> 00543690 -> 0040d4c0/0040d4e0 request 0xe/0xf; 00403fc0: 0044e800, 004055a0, 0040e550 (0xe also 004ed500, 0040f930), state 0x10/0x11; Esc/back in 0040dca0 requests 0x12 | [ ] the manor AI does not call 4-189/4-190; what opens the original pause menu in play is unresolved (0xe is also requested at startup by 00405a50/00405c80); port opens on Esc only when `pause_available()` |
| P6 | Vignette and quit procedures | [x] 4-191 -> `vignette_queue_00426640`; 4-305..308 in src/presentation/game_over.c | [x] `DisplayVignetteAndReturnToMainMenu` typed; vignette host `vignette.gd` |

## Dialogue presentation

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| D1 | Frames, fonts, two-line and six-line boxes | [x] Fix 19:1 (84.8% read for menu), Rete `dialogue-presentation` + `dialogue` `level:/dialogue/en/<name>` (dialogue_presentation.rs, dialogue.rs) | [x] voice-timed pages, `ui/dialogue*.gd` |
| D2 | Space advances a page (Proc_SelectDialog 327) | [x] dialog object: +0 count, +1 selected, +0x10 continuation, +0x14 mode [doc static-checklist-records.md 6] | [x] `Button_Hit` -> `source_select_dialog` |
| D3 | Gogoud letterbox (box 3) | [x] box 3 anchor like the style-5 chain | [x] frame-for-frame at 1024x768 |
| D4 | Framed boxes 1 and 5 | [x] box 1 rows 25-313, columns 497-990; box 5 panel 461x256, 30 px inset | [x] fade +0.1 alpha per drawn frame after 10 ms |
| D5 | Box 1 text after the anchor fix, page timing | [x] anchor read | [ ] not re-captured or compared |
| D6 | Gaps between font and frame records | [ ] 520 and 508 B gaps in 19:1, probably frame or box parameters | n/a |
| D7 | Branching answers, item icons, channel effects | [ ] answer entry +0x08 one-shot, +0x18 branch, +0x28 lines, +0x3c speech, +0x44 mapping; condition ids 120-122 unknown (0056c690, 0056e3b0) | [ ] 16 of 48 Brigand items unsupported; coverage debt in the 67-item catalogue |
| D8 | GPI images (4-349..357, 3-216, 3-217) | [x] handlers 0053ef50, 005374c0 (3-216/217 rewritten) | [ ] 4-350, 4-349, 4-357 are stand-ins (4 Brigand sites each) |
| D9 | Fade texture and value (4-317..322) | [x] 00544180 (rewritten in src/render/snapshot_fade.c) | [x] `source_execute_fade`, `source_set_fade_texture` |

## Controls

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| C1 | Rebinding page | [x] Controls root in Fix 18:0 and entry actions in Fix 7:0 (`controls()` in menu.rs; [research](controls-menu-original.md)); user bindings in `user://hype-settings.cfg`, not the scrambled `Default.cfg` | [x] `game/controls_menu.gd` on the converted page; matched against the original (checklist) |
| C2 | Default key table | [x] read from Fix 7:0; Up_Key is 0xc8 | [x] source defaults for every options entry, including shield P, scroll Delete, side steps A/S, map End |
| C3 | Remaining actions rebindable | [x] the page lists 15 keyboard and 9 pad entries; Save, Load, Map, Shield, AlwaysRun, Lateral, SkipCinema and the 1/2/3 shortcuts have no row (file only) | [x] all composed into their script actions; TestInventoryMode stays with the inventory host (4-334) |
| C4 | Reserved keys | [x] nameless keys (Esc, PrintScreen, Scroll, Pause, NumLock) cannot be bound | [x] those plus F1, F5 and R (port session keys) |
| C5 | Entry-action reset on mode change | [x] 0040e550 marks nine handles (`DAT_00754340..` incl. back `DAT_00754354`) at every menu entry | [ ] port releases all `Button_*` on `leave_session` only |

## Cheats (manor AI)

The controller is actor `level:/persos/world/0/27` `iDummy_PourCercles`, model `fix:/ai/models/535`
(3,024 lines; 3 `stand_in(` and 526 `fail_unsupported(` lines; the guards test only that a binding
exists, they are not unconditional). DSG[7] = reveal flag, DSG[8] = lever flag. Status below is
"compiles to working code" unless a stand-in is named; none of it is reachable yet (CH1).

| # | Feature | Data layer | UI layer |
| --- | --- | --- | --- |
| CH1 | Typed-code recognizer | [ ] 272 code bindings in the 497-action table (docs/openspace/cheat-codes.md); table in Fix 7:0 unread | [ ] no recognizer; every cheat action is adopted with `KEY_NONE` (`behavior.gd:adopt_input` warns), so no cheat can fire |
| CH2 | `debug` reveal (`DebugTeleport`) | [x] F535 normal 3: sets own `dsg_7` when it is 0 (line 2674) | [x] working code |
| CH3 | Lever `iLevierSecretManoir` | [x] M154 (world actor) lines 505-600: `/27.dsg_7==1 && dsg_8==0` shows `/174`, waits 2000 ms (`source_time_elapsed`), `Button_Hit` within 1.5 units, sets lever model 1396 normal 1, Hype normal 43, `/27.dsg_7=0`, `dsg_8=1` | [x] working code; lever model 1396 normal 7 ends in `generic-5-7` (`Action_WaitEndOfActionPerso`) |
| CH4 | `tele01`-`tele36` (gate `dsg_8==1`) | [x] 36 sites: `source_switch_microphone(null)` + `LevelTransition.request_level_change(level, hype_spawn, cam_spawn)`; empty spawns for ciel1/2/3/4, tour, tour3, toptower; quirks kept: casino3 `~`, village3 camera spawn of labo3, `cachots4~` | [x] working code; unconverted levels end the session via `source_level_not_converted` |
| CH5 | `hunter` (HomingBalls) and `geritole` (PauseBall), gate `dsg_8==1` | [x] toggle Hype `dsg_7` / `dsg_8` with a sound | [x] toggles compile; the fireball consumer was not traced here |
| CH6 | `kutkut` (Hypehache) | [x] F28 reflex_3_rule_0 (line 27524), gate `/27.dsg_8==1` (not in the 38-binding list of the cheat report): sound, `source_swap_configured_parts(73, 74)`, toggles Hype `dsg_16` | [x] working code |
| CH7 | `protek` FullArmor, `druidik` FullMagic, `leben` FullLife | [x] `armor.repair()`; `magick.add_points(max)`; Hype `dsg_131=0` + life to max | [x] working code |
| CH8 | `littletroll` Regeneration, `littlegogoud` MagieRegen, `hermetik` InfiniMagie, `houdini` InfiniFleche | [x] toggle Hype `dsg_2`, `dsg_3`, `dsg_13`, `dsg_14`; normal 2 rules refill life and magic | [x] working code; consumers in F28 not traced |
| CH9 | `toutundefi` DetruireArmure | [x] repeated `Func_ApplyArmoredDamage(40)` and life refill (233 guard lines) | [x] working code |
| CH10 | `tunnel` CameraFocal | [x] toggles Hype `dsg_12`, `source_change_focal(0, 2.5)` / reset | [x] working code, needs a SourceCamera focal preset |
| CH11 | `youngelf`, `oldelf`, `grolot` | [x] add pickup actors `/39`, `/38`; seven `iDummy_CheatInv_*` objects (`/39 /38 /37 /40 /42 /41 /36`) | [x] `source_inventory_add_object` |
| CH12 | `thereyougo` ToutesMagies | [x] adds 12 magic pickups (`/77..88`), removes all points, max 250, refills | [x] working code |
| CH13 | `pouletfrit` (Galarneau ad, object `/107`) | [x] add or remove branch on 1-119 | [ ] stand-ins 1-119 (not carried) and 3-197 (nothing removed): always adds, never removes |
| CH14 | `frik` Plastycks | [x] `Func_Inv_AddGold(random 0..100)` | [ ] stand-in 3-199, the random operand is dropped, no gold |
| CH15 | `oint` TakePosResetMap and `revient` ResetMapButNotHype | [x] `level:/ai/models/195` on `iDummy_SaveTesteurs`: copy ~160 Hype fields, position (`dsg_9`), facing (`dsg_10`); restore, then the player-death call | [x] compiled (~150 guard lines per block); death through `source_player_is_dead` |
| CH16 | Left Shift + Left Ctrl + MagicScroll bulk quest flags | [x] same model 195, lines 2706-3774, sets ~150 Hype fields to 1 | [x] compiled; `Button_MagicScroll` unbound |
| CH17 | `ite00..26`, `gl000..176`, `epok*`, `supered`, `allpowers`, `p0..p6`, `joyo*`, `nrg*` | [ ] bindings only, no manor AI reference [doc cheat-codes.md] | n/a |

## Priorities

Order: what blocks or visibly changes play on the Manoir to Brigand path first, then research size
(S = under a day of reading plus a small change, M = several days, L = open-ended).

1. **Inventory entry and exit through the AI (I8-I10, M6, M8).** Today Tab bypasses the AI guards and the
   `dsg_18`/`dsg_77` restore rules never run, so the HUD indicators, Object 44 rule and fade after the screen
   differ from the original on every Tab. Bind `TestInventoryMode` and `Button_ActivateMap`, let 4-334 and 4-374 open
   the host, and run the exit rules. Size S. Evidence complete in this note.
2. **Retire 1-118, 1-119, 3-197, 3-199 (I21, I22, G2).** `inventory.gd` already has the methods; 1-118 needs a full flag
   set at admission failure and cleared on removal (0041c720, 0041cb70). F28 reaches 1-118 on both pickup branches. Size S.
3. **Bind the keys Hype's script adopts unbound (C2).** Done 2026-11-04 through the Controls page bindings (C1 to C3); TestInventoryMode and Button_ActivateMap now wait only on item 1.
   Size S.
4. **Save lectern as a motor mode (S4-S7).** Replace the instant save by the pushed menu page (`DAT_00752418`), its back path
   (00413fe0) and the checkpoint offset stand-in (M348 node 83); decide the slot model together with `travel-save`. Size M.
5. **Inventory panel meters, gold, timer, armor (I19, G4, I20).** Visible on every inventory visit; data and draw order already read. Size M.
6. **Map runtime state (M3-M5).** Destination slots, object flags and epoque words are tiny records already read; this removes 4-271, 4-272 and
   `generic-4-270` (Brigand entry cutscene, 38 sites in six levels). The map screen itself (M6-M7) needs callback 004175d0. Size S then L.
7. **Magic selection and 4-438, 4-419 (I16, I24).** Host lacks the Magic branch; jewel state and the stock list are untyped. Size M.
8. **Cheat reachability (CH1).** One recognizer for typed codes unlocks 36 teleports and all stat cheats as a test tool for the Brigand path; needs the
   Fix 7:0 table decoded (also C1, C2). Size M. Pause-menu trigger and 18:0 records (P4, P5), branching answers (D7), and byte gaps (I7, D6, P3) are
   last: unread data, no known block on the path.

## Evidence

Method: tables above come from `target/release/astrolabe inspect` on `/home/stephen/.hermes/cache/scratch/rete-menu`, `rete-pause-menu`,
`rete-inventory` (kinds `menu-presentation`, `inventory-presentation`), the `rete-manoir/fix/{inventory,hud,gameplay}` files (kinds
`inventory-catalog`, `hud-presentation`, `armor-catalog`), grep of the generated GDScript, and the recovered decomp bodies below.

Motor-mode events (00403fc0 switch on `DAT_00754420`, requested through `game_request_event_00404ac0` = 00404ac0):

| Request | Setup calls | Next state | Source of the request |
| --- | --- | --- | --- |
| 0x28 inventory | 0044e800, 004055a0, 004ed500, 0041b3e0, 00419060; callbacks 0041aa00, 00403f20 | 0x29, exit 0x2a -> 9 | 4-334 (005437e0), exit 0041b3c0 |
| 0x1c map | 004ed500, 004055a0, 00416e70, 00416fd0; callbacks 004175d0, 00403f20 | 0x1d | 4-374 |
| 0x25 save | 0044e800, 004055a0, 00413bd0; callbacks 004148c0, 00403f20 | 0x26 (input 0040dca0, selection 00414b00) | 4-336 after 00413a80(0x25) and 0040f840 |
| 0x16 credits | 0044e800, 004055a0, 0040e550, 0040e9a0 | 0x17 (back -> 0x18) | 4-333 after 00413a80(0x16) |
| 0x19 binocular | 0044e800, 004055a0, 0044dfa0, 0044e130, 004054c0(100), 0044e690 | 0x1a | 4-335 (string operand lowercased, copied to 200-byte buffer, 0044ddb0) |
| 0xe / 0xf pause | 0044e800, 004055a0, 0040e550, 0040f930 | 0x10 / 0x11 (back -> 0x12) | 4-189 / 4-190 (0040d4c0, 0040d4e0), startup code |
| 0x27, 0x24 | menu pop and return to saved mode (00413fe0, 0040fb10) | saved `DAT_005e6bd8` | menu back |

Recovered bodies read (all rewritten, see the decomp list in the report): 005437e0, 00545c70, 00545cc0, 00545d00, 0053f780, 0053f7d0, 0053d920,
0053dbb0, 00543690, 005417d0, 00541850, 0053ec30, 005374c0, 00536550, 0053de60, 00416260, 00416280, 004162c0, 004162f0, 00416310, 0041d300, 0041d3f0,
0041dab0. Also read, already rewritten: 00535390 (1-118/1-119), 00539b60 (196-210). Read from the original export (not rewritten here): 00403fc0,
0040dca0, 00413fe0, 00413ad0, 0040e550, 00416e70, 00416fd0, 0041b3e0, 0041b3c0, 0041c290, 00419060, 0041d010, 0041d400, 0041d420, 0041d490, 0040c0a0, 0040c150, 0040c1f0.
PE bytes checked with objdump: 005437e0, 00541850, 00416260-00416310.

Unresolved:
- What opens the original pause menu during play (P5), and the original key set inside state 0x29, 0x1d and 0x26 (callbacks 0041aa00, 004175d0, 004148c0 are `LAB_` addresses
  with no function body in either export).
- Where the `map_epoque*.bmp` files live, the 100 map-object records, and the meaning of Global `dsg_233`, `dsg_53`, `dsg_56`, `dsg_132` (named only by use).
- The fireball and regeneration consumers of the cheat flags, and whether routes inject the unbound `Button_*` actions by themselves (C2).
- The typed-code table (Fix 7:0): only its size and entry layout are known.
