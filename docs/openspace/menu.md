# English main menu presentation

The Hype PC main menu uses `MenuTextList`/`MenuTextureAndTextList` serialized
records, distinct from the general `MNU_fn_vDrawMenuItem` API. The importer
selects the Fix menu root whose background is `Menu\\menu_princ.bmp`, then follows
its text-list membership. It does not select similarly named pause-menu items.
The validated English profile contains new game, load game, configure, credits,
internet and quit. Other menus and the separate inventory `InvBox` structures
are not imported by this slice.

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
