# Inventory, menu and dialogue presentation checklist

Kept by hand. Built from the byte-coverage report of the Fix blocks these screens
read, plus what the port shows today. The two-layer version, which splits each item
into its data (source records, Rete, runtime memory map) and its UI behavior, and
adds the save, map and motor-mode flows and the manor cheat scripts with a priority
order, is [notes/research/inventory-menu-two-layer.md](../../notes/research/inventory-menu-two-layer.md)
(2026-11-04). Regenerate the coverage with:

```sh
for c in menu inventory pause-menu; do
  ASTROLABE_BYTE_COVERAGE=output/byte-coverage/$c \
    target/release/astrolabe import-openspace disc /tmp/rete-$c --level manoir --content $c
done
```

Numbers below are from 2026-11-03 (Manoir, English). "Untouched" means no parser
reads those bytes: they are what we do not understand yet, not what is broken.

## Coverage by block

| Fix block | Size | Read by | Interpreted | Untouched | Largest untouched gaps |
| --- | ---: | --- | ---: | ---: | --- |
| 18:0 (menu records) | 117,681 | menu (main) | 97.9% | 2.1% | 2,409 bytes at 0x1c248, after the credits key slots (menu.rs:713) |
| 18:0 | | pause menu | 31.0% | 69.0% | 11,832 at 0x19d79 and repeated 407-byte records from 0x9b1 (menu.rs:388): the pause scope reads one record kind only |
| 19:0 (menu text handles) | 24,269 | menu | 6.6% | 93.4% | 3,516 at 0x8, then 92-byte records from 0x5150 (menu.rs:768) |
| 19:1 (fonts, frames) | 61,673 | menu / pause / inventory | 84.8 / 83.2 / 74.7% | 15 to 25% | 1,620 at 0xa5d0, 520 at 0x2638 and 0x392c, 508 at 0x4c9c, 268-byte runs from 0x713c (all between dialogue_presentation.rs:88 reads) |
| 10:0 (inventory definitions) | 10,589 | inventory | 52.0% | 48.0% | 1,060 at 0x22c4 (between inventory.rs:254 and :331), 88 to 104-byte gaps inside each item record |
| 10:1 (map names) | 5,037 | scene | 76.2% | 23.8% | 1,188 at 0x0, before the first map name |

Blocks 5:0 and 6:1 are read in other scopes; the UI scopes touch them only
through bindings and sprite lookups.

## Inventory screen

- [x] Opens on Tab with the original artwork, categories and fonts; pauses play.
- [x] Weapon selection through the screen (`change_weapon`), with the source
  admission check and a message when an item is not admitted.
- [x] Object selection through the screen (`select_object`), e.g. the key (21).
- [x] Unequip: confirming the selected Object clears it, as the original's
  inventory confirm handler 0041d010 does; another Object replaces it (0041dd70);
  a selected Weapon or Magic stays selected (tools/routes/key_menu.json).
- [x] Key through the real UI end to end: Tab, the key's slot, Enter, Tab; the
  gate opens and Hype walks through (tools/routes/stage4_gate_ui.json).
- [ ] Item records: 88 to 104 untouched bytes in each block 10:0 record and a
  1,060-byte run at 0x22c4. Likely per-item presentation (description text,
  sound, sort order); not read.
- [x] Use item from the menu: RestoreHP, RestoreMP, RestoreHPMP and Poison as
  0041cd60 applies them (capped, refused when full), one taken off the count
  (tools/routes/use_flask.json).
- [ ] Menu motion (category slide) timing compared with the original.
- [x] The HUD is hidden while the inventory is open, as in the original (take frame 284).
- [ ] The inventory's own stone panel meters: in the original (take frame 284) it shows a
  gold sword life gauge, the gold count, a game timer (00:04), the selected armor in the
  right socket and a ring in the left socket. The port draws the panel empty.
  Source (read-only disassembly of the state-0x29 frame callback LAB_0041aa00,
  0x41aa00-0x41b3bc, set by state 0x28): it never calls the HUD (0040c1f0) but draws
  0040c300 (life), 0043a6a0 (magic arc), 0043ac60 (armor icon, row 3) and 0043ab40
  (armor pieces, row 2) unconditionally, after FUN_00419060 overrides the active
  rectangle rows 0, 1, 2, 4 and 5-7 with inventory values read by 0041c290
  (PresentationWire offsets 320-376 in inventory.rs). Gold (Inventory.isp Gold list,
  element +10) and the game clock (TMR DAT_007474d8, "%02d:%02d", minutes unwrapped,
  at text_position offset 76) are printed through 00419610 in InventoryFont. The left
  socket is either the magic arc or the Joyaux sprites (00419540); not settled.
- [ ] Armor in the inventory and on the HUD: geometry, artwork and damage arithmetic
  match the source (0043aa70, 0043ab40). Differences: piece choice uses Godot's global RNG,
  not the original's seeded handle DAT_00610ba8 (by decision, see variants.md); repair touches only the active armor; the port saves
  the current piece (+0x24), which the original does not.
- [ ] Map screen (map names in 10:1 are read; the 1,188 bytes before them are not).

## Dialogue presentation

- [x] Frames, fonts and two-line/six-line boxes from block 19:1; voice-timed pages.
- [x] Space (Button_Hit) advances a page through Hype's AI (Proc_SelectDialog).
- [x] Gogoud's letterbox dialogue (box 3) matches the original frame for frame at
  1024x768: bar heights, name and text lines, continuation arrow. Fixed: text
  started half a glyph left (x 8 to 10 instead of 22); box 3 now starts at its
  anchor like the style5 chain.
- [x] Framed boxes: box 1 frame geometry matches (rows 25 to 313, columns 497
  to 990 of 1024) and its text now starts at the anchor; box 5 (Focus Magicum,
  lock messages) panel is 461x256 with a 30 px inset in both. Fade: +0.1 alpha
  per drawn frame after 10 ms, text at full opacity, about 0.13 s in the video.
- [ ] Box 1 text after the anchor fix not re-captured; page timing not compared.
- [ ] The 520 and 508-byte gaps between font/frame records in 19:1: probably
  frame or box parameters we have not mapped.
- [ ] Branching answers, item icons and channel effects in dialogue items:
  coverage debt in the 67-item catalogue (see ui.md).

## Main and pause menus

- [x] Main menu, Load Game picker and credits from blocks 18:0 and 19:0.
- [x] Options page labels and artwork from the source Options root.
- [ ] Block 19:0: only 6.6% read. The 92-byte records from 0x5150 sit between
  credit-text handles; probably other menu pages' text handles.
- [ ] Pause menu reads 31% of 18:0: its 407-byte records repeat from 0x9b1 and
  are not interpreted.

## Controls rebinding

- [x] The original Controls page (`Menu/control_back.bmp` root, reached from the
  Configure "controls" item) with its title tiles, keyboard/gamepad pictures,
  tabs, 15 keyboard and 9 gamepad rows plus reset, source labels and key names.
  Compared with the original at 1024x768: RMSE 0.052 in every state (tab level,
  list, `???` capture, swap, reset, gamepad), against 0.280 for the old page.
  See [the research](../../notes/research/controls-menu-original.md) and
  [docs/openspace/menu.md](../openspace/menu.md).
- [x] Capture, swap, double Escape and reset as in the original (checked live on
  a disposable copy); the source defaults from Fix 7:0 (look is Numpad0).
- [x] Every script action takes its inputs from the bindings through the source
  composition, so shield (P), choose magic (Delete), side steps (A/S), map (End),
  look and the rest are bound; inventory goes to the authored host (4-334).
  `godot/tests/controls_rebind_probe.gd` rebinds through the menu, plays with the
  new keys, rebinds from the pause menu and checks the bindings after a restart.
- [x] Writes at the source's points (play start, Resume Game, shutdown); F1, F5
  and R bindable, port shortcuts yield; F1 help is a stub in the original.
- [x] Left/right Ctrl, Shift and Alt are distinct keys; real X11 events carry the
  side. The paused route bot still sends side-less Shift/Ctrl.
- [x] Gamepad: button capture and dispatch, D-pad, and the source stick ranges
  (walk, run, turn), checked with simulated joypad events.
- [x] The Configure page's stained-glass title.
- [x] Stick release band from the source's self-latching ranges (simulated
  transitions).
- [ ] A physical pad (button order is the source index).
