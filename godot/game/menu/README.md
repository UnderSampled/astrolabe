# Main menu

`game/bootstrap.tscn` owns the editable menu controls and their layout;
`theme.tres` owns their appearance. `bootstrap.gd` fits the 640×480 canvas inside
the available viewport, checks content/save availability, and calls the existing
New Game and Load Game scene entries. The menu has no gameplay initialization or
save-writing responsibility. Its named `NewGame` and `LoadGame` Buttons remain
available to the existing session harnesses.

The original background is optional at
`generated/hype/textures/MENU/menu_princ.gf.png`. An authored Texture2D override
can also be assigned to `background_texture` in the inspector. The committed
scene opens without either and displays its authored fallback. Game-derived
media and import caches remain ignored. The Theme uses an installed serif font;
the original blackletter glyphs, glow animation and menu music are not yet ported.
Configuration, credits and internet submenus are also outside this first slice.

To prepare the backdrop without regenerating or replacing any manor content,
use the existing texture converter in a separate output project, then copy its
one PNG to the existing generated-texture path:

```sh
target/release/astrolabe port disc output/menu-media --content textures
mkdir -p godot/generated/hype/textures/MENU
cp output/menu-media/generated/hype/textures/MENU/menu_princ.gf.png godot/generated/hype/textures/MENU/
```

This conversion invokes Godot; concurrent development assigns it to the shared
runtime runner. Open the actual game project in Godot 4.7.2 and let its texture
import finish before running. Manor generation and both game entries retain
their instructions in `godot/README.md`.

Source evidence: the retained original main-menu capture and decoded
`MENU/menu_princ.gf` show a 4:3 heraldic background, title, descending menu
positions and golden labels. Original `MNU_fn_vDrawMenuItem` (`004b51a0`) draws
labels as text with distinct selected/unselected effects; `MNU_fn_vDrawMenu`
(`004affb0`) marks the selected item. This implementation uses ordinary Godot
Buttons, a Theme, TextureRect and focus navigation for those visible concerns.
