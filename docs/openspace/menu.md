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
separate; controls remapping, image quality and credits remain outside this
slice. Inventory has its own `InvBox` import.

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
`0x25`. `FUN_00413bd0` loads three `DisplayWithSound` option values and maps their
source range `0..127` to vertical slider positions, with higher values toward
the top. The source configuration is retained inside the main menu asset and
exported as a separate native Godot scene. Its Music, Sound and Voices labels
remain original glyph artwork; native vertical sliders retain values on the
original scale through `UserSettings` and
Godot audio buses. A missing option defaults to 127. The image and controls pages
still need separate native mappings.

The same dispatcher sends `Credits` to mode `0x16`. `Internet` writes a temporary
HTML file and asks the installed browser to open the legacy Playmobil URL. Those
routes remain unimplemented in the Godot starter; it does not launch that
external destination.
