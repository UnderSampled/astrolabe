# Main menu

`game/bootstrap.tscn` owns the editable menu controls and their layout;
`theme.tres` owns their appearance. `bootstrap.gd` fits the 640×480 canvas inside
the available viewport, checks content/save availability, and calls the existing
New Game and Load Game scene entries. The menu has no gameplay initialization or
save-writing responsibility. Its named `NewGame` and `LoadGame` Buttons remain
available to the existing session harnesses.

The installed original background is required at
`generated/hype/textures/MENU/menu_princ.gf.png`. An authored Texture2D override
can also be assigned to `background_texture` in the inspector. If neither is
available, the menu displays an explicit artwork error and disables game entry.
Game-derived
media and import caches remain ignored. The Theme uses an installed serif font;
the original blackletter glyphs, glow animation and menu music are not yet ported.
Configuration, credits and internet submenus are also outside this first slice.

Source evidence: the retained original main-menu capture and decoded
`MENU/menu_princ.gf` show a 4:3 heraldic background, title, descending menu
positions and golden labels. Original `MNU_fn_vDrawMenuItem` (`004b51a0`) draws
labels as text with distinct selected/unselected effects; `MNU_fn_vDrawMenu`
(`004affb0`) marks the selected item. This implementation uses ordinary Godot
Buttons, a Theme, TextureRect and focus navigation for those visible concerns.
