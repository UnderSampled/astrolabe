# English main and pause menu presentation

The Hype PC main menu uses `MenuTextList`/`MenuTextureAndTextList` serialized
records, distinct from the general `MNU_fn_vDrawMenuItem` API. The importer
selects the Fix menu root whose background is `Menu\\menu_princ.bmp`, then follows
its text-list membership. The Options route selects the sibling
`Menu\\config_back.bmp` root and retains its source labels, links, depth-projected
layout, backdrop and three `Slider1`/`Slider2`/`Slider3` positions. The pause
import selects the distinct `Resume Game` text-list parent and verifies that no
texture header owns that root.
The validated English profile contains new game, load game, configure, credits,
internet and quit. The installed pause profile contains `Resume Game`,
`NextGame`, `Options` and `Main Menu`, with localized labels "resume game",
"start a new adventure", "configure" and "main menu". These roots remain
separate; image quality and the legacy Internet launch remain outside this
slice. Credits retains a separate typed English text/font presentation nested
under the main menu root. Inventory has its own `InvBox` import.

`FUN_004105b0` draws each item using position words 8–10, localized text word
0x13, selected/normal font names at 0x1c/0x24, and horizontal alignment byte 0xb0.
`FUN_00413240` uses item links at 0x0c/0x10/0x14/0x18 for up/down/left/right and
passes the name at 0x2c to `FUN_00414b00` for action selection. The text-list body's
first item supplies initial selection. Typed records and relocation resolution
stay inside the OpenSpace adapter; useful content retains positions, alignment,
action identities, navigation links and media references without source addresses.

The original `MenuFont` and `MenuFont_H` contain 71 single-byte glyphs each.
`FON_fn_eLoadFontSprite` and `FON_fn_vDraw3DString` corroborate their quad UVs,
widths, vertical offsets and tracking. Normal and highlighted metrics differ.
The glyphs use multiple `FixTex/MOTORTXT/menus/*_ad.gf` atlases, white ambient
color and additive blending. These are the English blackletter sprites; the
French whole-word images under `FixTex/menus/mainmenu` are a separate asset set.
Conversion projects source quads into the original 640×480 UI canvas and bakes
normal/glow textures at twice that virtual pixel resolution. Native scene
properties preserve layout, while a native CanvasItemMaterial preserves additive
composition. The useful item position remains independently editable in Rete.
Pause text has source depth 0.3; `FON_fn_vDraw3DString` projects its glyphs with
`factor = 1 - depth / 6`, `half_x = 4.65 / factor`,
`half_y = 3.49 / factor`, and `pixel_scale = 640 / (2 * half_x)`.
The source text-only root becomes an ordinary transparent Control overlay over
the paused game. No screenshot, substitute background, depth interpreter or
source font machinery runs in Godot.

The supplied manual's printed page 14 describes "Load another adventure", but
the installed English menu says "start a new adventure" and dispatches
`NextGame`. `FUN_00414b00` sends that action to mode 0x0d;
`FUN_00404700` unloads the current level, resets designer defaults and selects
`HYPE`, then enters the normal fresh-game path. `SavedGames` uses a different
handler. The authored pause route therefore tears down the current session via
the existing `leave_session` API and opens `new_game.tscn`, without reading a
save fixture or overwriting a save. Resume keeps the current scene and actor.

Menu navigation and confirmation use the original custom sound array:
`FUN_0040b390` dispatches array element 2 for navigation and element 0 for
confirmation. English `Fix.snd`/`Fix.rts` bind these to library events 0:66 and
0:67; `playmo_b.lcb` maps those to `M_Menu.csb` events 2 and 4, selecting
`Select.apm` and `Valid.apm` in bank 0. Existing sound graph and ADPCM decoders
resolve these samples. Resource gains 70/127 and 75/127 become native player
volume properties. Music and idle voice events remain unimplemented here.

`--content menu` follows the usual OpenSpace → useful Model → Rete → Godot
pipeline and publishes under the independent `menu` ownership key. Godot output
contains editable native Controls, normal/glow PNGs and audio resources.
Regeneration protects authored inherited scenes and rejects modified generated
bases. See [game menu ownership](../../godot/game/menu/README.md).
`--content pause-menu` publishes the analogous independent `pause-menu` closure.
Rete schema 42 permits the useful presentation's explicitly absent backdrop;
support interface 54 exposes modal focus reset, confirmation cancellation and a
separate voice audio bus.
See [pause composition](../../godot/game/menu/README.md#pause-menu-composition).

`FUN_00414b00` routes `Options` into `DAT_007523f4` and enters configuration mode
`0x25`; the source initial item is `Images`. The separate Godot scene retains
all three original tabs and glyph labels. Its Music, Sound and Voices page uses
native vertical sliders on the source `0..127` scale, applies them to Godot audio
buses, and persists them through `UserSettings`. A missing source option defaults
to 127.

The Images page identifies the original settings without claiming an unsupported
Godot effect. `ImageSlider1` sets source shadow quality through
`SHW_fn_vSetLevelOfQualityOfShadows`; `ImageSlider2` passes its `60..100` value
to `FUN_004054c0`, which adjusts the active source camera's viewport rectangle;
`ImageSlider3` controls the OpenSpace texture-swapping ratio. Godot exposes
renderer shadow controls and camera viewport properties, but those APIs do not
provide a verified semantic mapping for the source's 0..4 shadow levels or
global camera adjustment. Texture swapping is a source streaming policy without
a corresponding Godot control for these locally loaded assets. None of the
three has a verified behavior-preserving equivalent in this project, so the
page says they are unsupported and provides no functional sliders. The
Controls page provides native remapping for the five source actions that the
current Godot session actually consumes: left, right, up, down and hit. It
persists physical keyboard keys and generic Godot joypad buttons/axis directions
in `user://hype-settings.cfg`; the current binding transfers to another action
when the same input is selected, matching the decompiled remapper's occupied-key
transfer path. Escape cancels an active capture before closing configuration.
Reset restores Astrolabe's current arrow/Space keyboard defaults and clears
generic joypad bindings. These are the port's present defaults, not values
recovered from the game's source options file.

`FUN_00414b00`, `FUN_004123b0` and `IPT_fn_bChangeIPTValue` show separate source
Keyboard and Paddle/Joystick paths. The generic controller tab deliberately
uses Godot JoypadButton/JoypadMotion events and does not claim support for the
source Paddle or Joystick device modes. The source KeyStrokes route displays
current bindings; the native action rows show those values directly. Other
source controls are not exposed until their gameplay actions exist in this
session.

The same dispatcher sends `Credits` to mode `0x16`; `FUN_00403fc0` enters credits
mode `0x17`. `FUN_0040f3f0` loads `credits.ini`, selects `Playmo_Font2_0`, and
looks up a text handle for each configured key. `FUN_0040ef40` appends every
active key into a 24-byte slot; it does not increment the display count. The
font-assignment loop in `FUN_0040f3f0` and draw loop in `FUN_0040eb10` both stop
at `DAT_0060fa00`, loaded by `CDTCreditsNumEntries("397")`. The installed INI
has 400 active directives, ending with `CDT2_B_PROD40`, `PROD41` and `PROD42`;
those last three registered keys are not assigned or drawn by the source loops.

The English `Fix.lng` Fix block 18 contains exactly 400 fixed-width credit key
slots. The importer compares every slot with the active INI entries in order.
English `Fix.dlg`/`Fix.rtd` contains 410 references to Fix block 19; its final
400 text handles map to that same key order, after ten unrelated Fix text
handles. Their pointer targets provide the localized strings. This matches the
decompiled `FON_fn_eLoadTextString` callback, which calls `Link_SetValue(key,
text_handle)`, and `FON_fn_hGetTextHandle`, which searches that key table. The
importer checks the first localized values and retains all 400 pairs in Rete;
Godot emits only the first 397 as native Label nodes. It uses the source font
metrics/UVs/atlas, black background, 40-line visible window and 0.35 world-unit
spacing. In `FUN_0040eb10`, `-` is left alignment, no prefix is centered and
`+` is right alignment; a right-aligned entry and the following left-aligned
entry share a baseline. Back/Escape return to the menu, and the source end
condition is represented by automatically returning when the final row leaves
the top of the viewport.

The decompilation snapshot does not include the numeric initializer for
`DAT_005e6c78`, the timer-to-world scroll multiplier used by `FUN_0040eb10`.
The authored Godot controller exposes a default speed of 1 world unit per
second for tuning; exact timed parity remains unverified. The credits background
image is commented out and there are no active `CDTImage` declarations, so the
source black background is used without extra artwork.

The dispatcher sends `Internet` through its legacy branch, which writes a
temporary HTML file and asks the installed browser to open the old Playmobil
destination. Godot reports that this route is obsolete and does not open an
external page.
