# Original Options > Controls page (static trace, 2026-11-04)

Static research only: decompilation (`reference/decompilation`), the patched exe's
data tables, Fix blocks 7:0 and 18:0 (dumped through `SourceSession` in a
throwaway worktree, level `manoir`, locale `en`), and the disc's `Default.cfg`.
Nothing was run. Tags: [read] = read in decomp/data, [inferred], [unknown].
Pixel positions use `menu.rs` `configuration()`'s depth-0 formula
`px = (4.65 + x) * 640 / 9.3`, `py = (3.49 - y) * 640 / 9.3`.

## 1. Entering the page

- On the Options root, the item named `Controls` (label "controls") goes to
  `FUN_00414b00`. It calls `FUN_00413ad0(DAT_0075240c, 0, 0)`, then
  `FUN_00413a80(0x25)` [read]. `DAT_0075240c` is a separate root, not the
  Options root.
- In Fix 18:0, that root is alloc 0x9a28: mode 1, background
  `Menu\control_back.bmp`, body 0x9cd4. Its hint string is inline at +0xc [read].
- The initial focus is body+8 = 0xbdf0, the `Keyboard` tab (`FUN_00413ad0`
  sets `DAT_00610474 = *(body+8)`) [read].

## 2. Layout (Fix 18:0 MenuText records under body 0x9cd4)

Each record holds the name at +0x2c, the label pointer at +0x4c, and its
position at +0x20. Links are at +0xc/+0x10/+0x14/+0x18 (up/down/left/right),
the child at +0x1c, and the fonts at +0x70 (selected) and +0x90 (normal). The
alignment is at +0xb0. +0xba holds the keyboard options-entry name and +0xda
the pad options-entry name; each is 32 bytes [read]. `menu.rs` reads only 177
bytes, so it never sees +0xba or +0xda.

Tabs [read]:

| alloc | name | label | px | fonts (normal/selected) | align | links | child |
|---|---|---|---|---|---|---|---|
| 0xbdf0 | Keyboard | keyboard | (165.2, 343.4) | MenuFontSmall / MenuFontSmall_H | 1 | left=right=Gamepad | 0xd9f0 (move forward) |
| 0xc038 | Gamepad | gamepad | (244.3, 343.4) | MenuFontSmall / MenuFontSmall_H | 1 | left=right=Keyboard | 0xc280 (run) |

The tabs have no up/down links.

Keyboard rows: name `KeyStrokes`, both fonts `MenuFontSmall_H`, align 0,
x = 1.5 (423.2 px). Rows are 0.25 apart, which is 17.2 px [read]:

| # | label | +0xba entry | py |
|---|---|---|---|
| 1 | move forward | Up_Key | 171.4 |
| 2 | move backward | Down_Key | 188.6 |
| 3 | go left | Left_Key | 205.8 |
| 4 | go right | Right_Key | 223.0 |
| 5 | run | SpeedUp_Key | 240.2 |
| 6 | jump | Jump_Key | 257.4 |
| 7 | sidestep right | SSRight_Key | 274.6 |
| 8 | sidestep left | SSLeft_Key | 291.8 |
| 9 | action or attack | Action_Key | 309.0 |
| 10 | unsheathe sword | GetSword_Key | 326.2 |
| 11 | unsheathe crossbow | GetCrossbow_Key | 343.4 |
| 12 | choose magic | ScrollMagic_Key | 360.6 |
| 13 | unleash magic | Magic_Key | 377.8 |
| 14 | inventory | Inventory_Key | 395.0 |
| 15 | look | Look_Key | 412.2 |
| 16 | reset to default values (`DefaultValues`, alloc 0xdc38, align 1) | none | 429.4 |

Gamepad rows: same fonts and x, spaced 28.7 px [read]:

| # | label | +0xda entry | py |
|---|---|---|---|
| 1 | run | SpeedUp_Pad | 171.4 |
| 2 | jump | Jump_Pad | 200.0 |
| 3 | sidestep | SideStep_Pad | 228.7 |
| 4 | action | Action_Pad | 257.4 |
| 5 | unsheathe weapon | Arme_Pad | 286.1 |
| 6 | choose magic | ScrollMagic_Pad | 314.7 |
| 7 | unleash magic | Magic_Pad | 343.4 |
| 8 | inventory | Inventory_Pad | 372.1 |
| 9 | look | ModeLook_Pad | 400.8 |
| 10 | `DefaultValues` with label "   " (three spaces), alloc 0xc4c8, align 1 | none | 429.4 |

- Up/down links form a wrapping ring in each list: the first row's up is the
  reset item, and the reset item's down is the first row [read]. The rows have
  no left/right links.
- Sprites in the root's sprite list (name at +0x24, x/y at +0x1c/+0x20,
  w/h at +0x74/+0x78, texture pointer at +0x64) [read]:
  - `DisplayWithKeyboard` and `DisplayWithGamepad`: 128x128 at (205, 255).
  - `SelectBox`: 310x20 at (442, 174).
  - Texture names: [unknown]. The pointers lead outside 18:0.
- Drawing, `FUN_004105b0` and `FUN_00410230` [read]:
  - On this root, `DAT_005e6be4` selects the list (1 = keyboard, 0 = pad).
  - A row whose entry name for the current mode is set draws the bound key's
    name, then its label.
  - `KeyStrokes` rows without an entry for the current mode are skipped.
  - `DefaultValues` items have no entry names, so both are drawn in both modes.
    The pad one is blank, so "reset to default values" shows in both modes
    [read; visual overlap inferred].
- The key-name text [read]:
  - It is `IPT_fn_szGetStringOf(handle)` passed through `_strlwr` (lowercase),
    in the row's +0x90 font (`MenuFontSmall_H`).
  - It is drawn at x - 0.4 (1.1, about 395.7 px) with alignment 2 at the
    row's y and z.
  - Alignment 2 is probably right-aligned, ending left of the label [inferred].
- During capture, the focused row's key text is replaced by `"???"`
  (`DAT_005e6ca4`) [read].
- Label fonts [read]:
  - Rows use `MenuFontSmall_H` for both states, so focus shows only through
    the `SelectBox`.
  - Tabs use the selected font when focused or when +0xb9 = 1 (the active tab
    while its list is open).
- `FUN_00411780` (refresh) on this root [read]:
  - If the focus is named `Keyboard`: `DAT_005e6be4 = 1`, SelectBox step
    `DAT_005e6c8c = 17.18` (0x418970a4), show `DisplayWithKeyboard`, hide
    `DisplayWithGamepad`.
  - If the focus is `Gamepad`: `DAT_005e6be4 = 0`, step 28.65 (0x41e53333), and
    the reverse visibility.
  - When no list is open (`DAT_00610498 == 0`), the other cases hide both
    pictures.
  - `SelectBox` is visible only while a list is open.
  - So moving left/right between tabs switches the shown list and picture at
    once [read].

## 3. Navigation and capture

- Per frame, `FUN_0040dca0` runs in mode '&'; `FUN_00413240` runs next and
  calls `FUN_004123b0` when `DAT_00610498` (list open) is set [read].
- Tab level, using the generic `FUN_00413240` path:
  - Left/right follow the links (Keyboard <-> Gamepad).
  - Confirm runs `FUN_00414b00("Keyboard" | "Gamepad")` [read], which:
    1. sets `DAT_00610498 = 1` and +0xb9 = 1 on that tab;
    2. sets `SelectBox` y to 173.0 (0x432d0000);
    3. sets `DAT_005e6be4` to 1 (Keyboard) or 0 (Gamepad);
    4. clears `DAT_0061049c`;
    5. moves the focus to the tab's child (+0x1c), the first row.
  - Gamepad entry does not check that a pad is present [read].
- In a list (`FUN_004123b0`) [read]:
  - Up/down follow +0xc/+0x10 and move `SelectBox` by the step.
  - The box wraps: going up below 167 sets y to 432, and going down above 438
    sets y to 173. This matches the 16-row and 10-row rings.
  - Left/right follow +0x14/+0x18 only if set (rows have none).
  - Confirm sets `DAT_006104b8 = 1`, plays the item sound (`FUN_0040b390`) and
    dispatches the item name.
- Capture start: `KeyStrokes` sets `DAT_0061049c = 1` [read]. There is no
  timeout [read: no timer in the capture branch].
- Each frame while `DAT_0061049c != 0`, at the top of `FUN_004123b0`, before
  any navigation [read]:
  - Gamepad mode: if `IPT_fn_bIsPaddleControlAvailable()` is false, capture
    ends at once (`DAT_0061049c = 0`).
  - It reads the entry name from the focused item at +0xba (kbd) or +0xda (pad).
  - It calls `IPT_fn_hGetEntryActionHandleForOptions(name)`, which matches the
    entry's options name at +0xc.
  - It then calls `IPT_fn_bChangeIPTValue(handle, 1)`. The second argument was
    confirmed in the disassembly at 0x412467..0x412482 (`push ebx`, ebx = 1).
  - On success it clears `DAT_0061049c` and `DAT_006104b8`.
- `IPT_fn_bChangeIPTValue` 004a4c50, keyboard entries (opcode 0xf006) [read]:
  - It scans the exe's key table at 0x5ee1d8 (`{char* name, short scancode}`,
    `--Last--` terminated) in table order.
  - It takes the first key that is down this frame and was up last frame.
  - The key must have a display name (`IPT_fn_p_szGetNameOfKeyName(code, 0)`
    non-null).
  - So the scan table holds Esc 0x01, 0x1a, PrintScreen 0xb7, Scroll 0x46,
    Pause 0x69 and NumLock 0xc5, but the name table does not, and they
    **cannot be bound**.
  - Enter, Space, the arrows, Tab and Backspace can be bound.
- Pad entries (opcode 0xf01a) [read]:
  - It scans the 32 button masks at 0x5ee7b0 on the entry's device and maps
    them to codes `PadButton0..31` (0x0a..0x29, table 0x5ee514 + i*8).
  - Only buttons can be captured. D-pad and axes cannot.
- Occupied key, mode 1: `FUN_004a4be0` finds another options-named entry of
  the same type with that key. That entry gets **this entry's old key** (a
  swap), then this entry gets the new key [read].
  - The search covers all 25 `_Key` options entries, including ones that have
    no row (Save F3, Load F2, Activate_Map End, SkipCinema Back, Raccroucis
    1/2/3, Shield P, AlwaysRun Caps, Lateral LAlt). Binding a row to F3 would
    move Save to the row's old key.
  - Fixed entries (Menu*, Inventory*) are never checked [read].
  - Mode 0 (overwrite) and mode 2 (refuse if used) exist, but this page does
    not use them [read].
- Escape during capture [read path; behaviour inferred]:
  - `MenuPrevious` (`DAT_00754354`) is handled in `FUN_0040dca0` before
    `FUN_00413240`.
  - The first Esc only clears `DAT_006104b8`, so capture stays armed and "???"
    stays.
  - A second Esc closes the list: `DAT_00610498 = 0`, and the focus goes back
    to the tab whose +0xb9 flag is set.
  - `DAT_0061049c` is left at 1 but is no longer processed, and it is cleared
    when a tab is confirmed again.
  - So Esc never binds and in effect abandons the capture. The old key is kept,
    because no write happens before a key is accepted.
  - The in-game hint (below) presents Esc as "confirm and go back".
- Back from the tab level: `FUN_00404ac0(0x27)` returns to the Options root.
  `FUN_00404700` does nothing for 0x27 [read]; the return itself is [inferred].
- Help: `FUN_00413240` and `FUN_00412ac0` show the root's +0xc text via
  `FUN_00414190`; the same key closes it. The handle is `DAT_00754364`
  [read]. That handle is `MenuHelpPopUp` = F1 [inferred from handle order;
  see section 5]. The Controls hint, read from 18:0 at 0x9a34:
  "Use the arrows to select the action that you want to change.  To release
  the old key press enter.  Then select the key that you would rather use.
  Press ESC to confirm this selection and to go back to the previous menu."

## 4. Defaults, saving and file format

- `DefaultValues` (either list) runs `FUN_00428c10`, then
  `IPT_fn_vActivePaddleControl()`, and sets `DAT_006104b8 = 0`. There is no
  confirmation [read].
  - `FUN_00428c10` descrambles `GameData\Options\` + `DefaultFile`
    (`DAT_00747080`).
  - It skips the 22-byte header and runs `IPT_fn_bLoadOptionsFromFile`.
  - So it restores **bindings only**, not volumes or graphics.
- File names come from `gamedsc.bin`: `DefaultFile` = `Default.cfg` and
  `CurrentFile` = `Current.cfg` (read in that order at all_functions.c:5850)
  [read].
- Startup (`FUN_00428a60`) loads `Current.cfg` and falls back to
  `Default.cfg` [read].
- Saving is `FUN_00428910`, which writes `Current.cfg` [read]. It runs:
  - on mode 0x12 (resume game) in `FUN_00404700`;
  - at shutdown (`FUN_004026b0`);
  - in `FUN_00402780`.
  It does not run on Back from Controls [inferred from 0x27 doing nothing].
- Scrambling (`FUN_00550550` mode 2 -> `FUN_00550a40`) [read]:
  - Byte 0 ^ 0x53 is a checksum.
  - u32 ^ 0x54555657 is the plain length.
  - Each following byte is `b ^ bitrev8(k) ^ 0xB9`, where k starts at 0x57
    and increments.
  - The decoded stream is 8-byte delta blocks `{row, mask, mask-selected
    bytes}` over a 256x8 table seeded with `row` in every byte. Output lags one
    block, and the last block is cut to `row` bytes.
- Plain layout, from `FUN_00428910` / `IPT_fn_bSaveOptionsInFile` 004a71f0
  [read]:
  - u32 at 0x0074738c, u32 at 0x00747390, u16 at 0x00747394, then three u32
    at 0x00747398/9c/a0.
  - Then u8 keyboard / paddle / joystick available. The keyboard byte is
    ignored on load.
  - Then, for every entry with an options name, in table order:
    `u8 len (including NUL), name, value`. The value is:
    - kbd (0xf006): u16 scancode;
    - pad (0xf01a): u16 device, u16 button;
    - anything else: nothing.
  - Then 0xFF, then an f32 texture-swapping ratio.
- `disc/Gamedata/Options/Default.cfg` decodes to 668 bytes; the checksum byte
  is 32 [read]:
  - Header: 0, 100, 4, 100, 100, 80. Kbd 1, paddle 0, joystick 0.
  - The bindings are exactly the Fix 7:0 defaults (table below).
  - `Look_Key` is 0x52 (Numpad0), not V.
  - Up/Down/Left/Right_Pad and `Button_ActivateMap/SideStep/SkipCinema` are
    stored name-only. Trailing f32 1.0.
  - This is the disc copy. Stephen's V binding would be in an installed
    `Current.cfg`, and none exists on this machine.

## 5. Entry actions (Fix 7:0, 497 entries of 0x1c at 0x68; block base 0x02ea0040)

Entry layout [read], matching `IPT_fn_vLoadBinaryFile`:
- +0 element count, +4 elements, +8 name, +0xc options name, +0x10 state
  (-2 at load), +0x18 active.
- Elements are 8 bytes. The value or entry pointer comes first, then flag
  bytes; byte 5 = 0x0a appears on the auto-repeat terms of Menu*.
- Opcodes: f001 AND, f002 OR, f00a entry, f00b NOT entry, f004 key just
  pressed, f006 key held, f007 key not held [inferred]. Also f00e pad
  direction, f010 axis range, f018 pad button just pressed, f01a pad button,
  f021..f023 typed cheat strings.

Options-named (user-facing, saved) entries, in table order, with defaults
[read]. Keyboard names are the English variant (see below):

| entry | default | row on page |
|---|---|---|
| Activate_Map_Key | End 0xcf | no |
| SkipCinema_Key | Back 0x0e | no |
| RaccroucisVie/Magie/VieMagie_Key | 1/2/3 (0x02/03/04) | no |
| AlwaysRun_Key | Caps 0x3a | no |
| SSRight_Key / SSLeft_Key | S 0x1f / A 0x1e | yes |
| Shield_Key | P 0x19 | no |
| ScrollMagic_Key / Magic_Key | Del 0xd3 / Enter 0x1c | yes |
| Load_Key / Save_Key | F2 0x3c / F3 0x3d | no |
| GetCrossbow_Key / GetSword_Key | Z 0x2c / X 0x2d | yes |
| Left/Right/Up/Down_Key | arrows 0xcb/0xcd/**0xc8**/0xd0 | yes |
| Look_Key | Pad0 0x52 | yes |
| SpeedUp_Key | Left Shift 0x2a | yes |
| Action_Key | Space 0x39 | yes |
| Inventory_Key | Tab 0x0f | yes |
| Lateral_Key | Left Alt 0x38 | no |
| Jump_Key | Left Ctrl 0x1d | yes |
| Up/Down/Left/Right_Pad | f00e direction 0..3 (not bindable) | no |
| SpeedUp/Jump/SideStep/ModeLook/Action/Magic/ScrollMagic/Arme/Inventory_Pad | PadButton 6/0/7/4/1/2/5/3/8 (names b7/b1/b8/b5/b2/b3/b6/b4/b9) | yes |
| Button_ActivateMap / Button_SideStep / Button_SkipCinema | -> Activate_Map_Key / Lateral_Key / SkipCinema_Key | no (no value) |

This resolves the open question about the Up default: Up_Key = 0xc8 [read].
There is exactly one key per `_Key` entry. Alternatives come only from the
composite `Button_*` entries, which OR keyboard and pad [read].

AI-facing composites (Hype's script names), all [read]:
- Movement:
  - `Button_Up/Down` = (Up/Down_Key | Up/Down_Pad | axis Y) ...
  - `Button_Left/Right` = (Left/Right_Key | (Left/Right_Pad & !SideStep_Pad) |
    axis X).
  - `Button_SpeedUp` = SpeedUp_Key | SpeedUp_Pad | axis.
- Combat:
  - `Button_Jump` = Jump_Key | Jump_Pad.
  - `Button_Hit` = Action_Key | Action_Pad.
  - `Button_GetSword` = GetSword_Key. `Button_GetCrossbow` = GetCrossbow_Key.
  - `Button_Arme_Pad` = Arme_Pad.
- Side steps:
  - `Button_SSLeft/Right` = SSLeft/SSRight_Key | (SideStep_Pad & Left/Right_Pad).
  - `Button_SideStep` = Lateral_Key.
- Magic and items:
  - `Button_Magic` = Magic_Key | Magic_Pad.
  - `Button_MagicScroll` = ScrollMagic_Key | ScrollMagic_Pad.
  - `Button_Shield` = Shield_Key.
  - `Button_ActivateMap` = Activate_Map_Key.
  - `Button_AlwaysRun` = AlwaysRun_Key.
  - `Button_RaccroucisVie/Magie/VieMagie` = keys 1/2/3.
- Inventory:
  - `TestInventoryMode` = (Inventory_Key & !LAlt) | Inventory_Pad.
  - `InventoryExit` = Inventory_Pad | Inventory_Key.
- Camera:
  - `BT_Camera_Look` = !InternalCameraAction & (Look_Key | ModeLook_Pad).
- So Shield, ActivateMap, AlwaysRun, SideStep (Lateral), SkipCinema, Save,
  Load and the Raccroucis shortcuts can be rebound only through the file,
  never on the page.

Menu navigation is fixed: these entries have no options names [read].
- `MenuUp/Down/Left/Right` = arrow just pressed, or arrow held with repeat,
  or pad direction.
- `MenuSelect` = (!LAlt & (Enter | PadEnter | Space)) | pad button 0.
- `MenuPrevious` = Esc & !LCtrl & !RCtrl. `TestMenu` uses the same term.
- `MenuHelpPopUp` = F1. `MenuBackSpace` = Backspace.
- `InventorySelect/UnSelect/Up/Down/Left/Right` use fixed keys and pad buttons.
- `FUN_0040e550` marks the nine handles at 0x754340..0x754394. The mapping
  0x40/44/48/4c/50/54 = MenuUp/Down/Left/Right/Select/Previous follows from
  `FUN_004123b0` [read behaviour]. The 0x364 = MenuHelpPopUp and 0x358 =
  MenuBackSpace names are [inferred]. The 38-name handle list is not in Fix
  7:0; it is filled at load from elsewhere [unknown].

Key display names: the Fix 7:0 table at 0x10898 has 170 records of 16 bytes
`{u16 code, French, English, German}`, terminated by -1 [read].
- `IPT_fn_szGetStringOf` picks the variant from `DAT_0087f25e`, which is set
  from the detected keyboard device type in `IPT_fn_vInitInput`:
  1 -> French, 0 -> English, other -> German [read].
- Pad codes are `button|0x8000`; the names run "B1".."B32" (PadButton0 = B1).
- English examples (shown lowercased): "up", "left shift", "left ctrl",
  "space", "enter", "tab", "del", "end", "pad0", "back", "f3".

## 6. Mouse

The input module can create a mouse device (`IPT_fn_vCreateMouseDevice`,
device type 5), and the exe names mouse axes and buttons at 0x5ee760 [read].
But no Fix 7:0 entry uses a mouse opcode, capture scans only keyboard or pad
buttons, and the menu functions traced here never read the mouse [read]. So
mouse buttons cannot be bound and the menu has no mouse support
[inferred, scoped to the functions traced].

## 7. Unknown

- Texture names and artwork for `SelectBox`, `DisplayWithKeyboard` and
  `DisplayWithGamepad` (pointers at sprite +0x64 lead outside 18:0).
- Exact `FON_fn_vDrawText3D` alignment semantics (0/1/2 assumed left/center/
  right). The help popup's look (`FUN_00414190`) and whether the hint ever
  shows without F1.
- The `DAT_005e9c5c` / `DAT_00610d3c` "handle == back handle" special cases in
  `FUN_004123b0` / `FUN_0040dca0` (they matter only if two menu handles alias).
- The source of the 38-entry handle list (`DAT_00754320`, loaded by name).
- What the real game shows on the Gamepad list with no pad: capture aborts
  immediately, and the tab is still enterable.
- Whether entering Gamepad mode activates the paddle device (only
  `DefaultValues` calls `IPT_fn_vActivePaddleControl`).
- Header fields 0x0074738c..0x007473a0 beyond "volumes/detail" [inferred].

## 8. Live check on a disposable original copy (2026-11-04)

One non-recording session: a reflink copy of the harness prefix with its own
Orrery MCP server (port 18767), English title menu → configure → controls,
physical keys through Orrery `press_keys`, screenshots only. The copy and
server were removed afterwards; the installation and saves were not touched.
Screenshots are kept outside the repository (Sandbox scratch has copies).
Confirmed [live]:

- Layout as in section 2; the tab level already shows the keyboard picture
  and list, keyboard tab in the glow font, gamepad dim, no SelectBox.
- Enter opens the list (SelectBox on "move forward"); Enter on a row shows
  "???"; pressing X binds move forward to "x" and unsheathe sword takes "up"
  (the swap).
- Escape during capture: the first keeps "???", the second closes the list
  and the old key stays.
- Right on the tabs switches to the gamepad list at once (b7 run, b1 jump,
  b8 sidestep, b2 action, b4 unsheathe weapon, b6 choose magic, b3 unleash
  magic, b9 inventory, b5 look), with "reset to default values" still shown.
- Reset to default values restores every keyboard binding at once.
- Escape at the tab level returns to Configure with "controls" focused.
- Not checked: F1 (not an Orrery key name), the pad capture itself (no pad),
  when Current.cfg is written.

Resolved from section 7:

- Texture names: `fixtex\menus\controlsmenu\kboard.tga`, `gamepad.tga` and
  `select.tga` (sprite +0x64). The root's sprite list also holds four
  unnamed sprites with their texture at +0x68: the English stained-glass
  title tiles `multilanguage\English\fixtex\menus\controlsmenu\contr_ta..td`
  centred at (158,86), (414,86), (574,86) and (158,214). The Configure root's
  `config_ta..td` tiles are presumably the same kind; the port's Configure
  page does not draw them yet.
- Alignment: matched against the capture, alignment 0 puts the first glyph's
  centre at x and alignment 2 the last glyph's centre, as the centred case
  works on glyph centres.
- Pad codes in both the entry elements and the key-name table are
  PadButton codes 0x0a + index; codes below 0x0a are directions ("padup").
