# Inventory, menu and dialogue presentation checklist

Kept by hand. Built from the byte-coverage report of the Fix blocks these screens
read, plus what the port shows today. Regenerate the coverage with:

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
- [ ] Unequip: selecting the selected object again. The port keeps it selected;
  the original's menu behavior is not reconstructed (`Inv_UnselectSelectedObject`
  exists for the AI only). Needs a live check in the original.
- [ ] Key through the real UI end to end: Tab, Objects, key, select, close, then
  the gate opens (stage4_gate.json selects it with a route step instead).
- [ ] Item records: 88 to 104 untouched bytes in each block 10:0 record and a
  1,060-byte run at 0x22c4. Likely per-item presentation (description text,
  sound, sort order); not read.
- [ ] Use item from the menu (potions, herbs): "Using this item is not
  implemented yet."
- [ ] Menu motion (category slide) timing compared with the original.
- [ ] Map screen (map names in 10:1 are read; the 1,188 bytes before them are not).

## Dialogue presentation

- [x] Frames, fonts and two-line/six-line boxes from block 19:1; voice-timed pages.
- [x] Space (Button_Hit) advances a page through Hype's AI (Proc_SelectDialog).
- [ ] Gogoud's in-game dialogue matched to the original: position, frame,
  fade pacing and page timing. Not compared yet.
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

- [x] The port's Controls page rebinds five actions (move left, right,
  forward, back; use/attack) for keyboard and controller, as a Godot UI.
- [ ] Match the original's rebinding UI: layout, the action list and order,
  how a key is captured and shown, and its defaults. The original's controls page
  data is in the Options root (menu.rs:844); the port replaces its controls.
- [ ] The rest of the original's actions (jump, run, sword, crossbow, magic,
  object, freelook and the camera keys) are not rebindable in the port. Stephen
  bound freelook to V in the original during the linear take.
