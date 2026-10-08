# Inventory ownership

Generate the independent original inventory closure:

```sh
target/release/astrolabe port disc godot --level manoir --locale en --content inventory
```

`generated/hype/shared/inventory/en/inventory.tscn` contains native Control,
Button, TextureRect and Label nodes. Its item catalogue consists of editable
native Resources, original texture layers and English labels. The five category
centres, three magic groups, six-slot row, focus links, scale/colour values and
bitmap glyph metrics come from serialized source values. Media and Godot caches
are ignored; there is no replacement art.

`game/game_session.tscn` composes `inventory_host.gd`, shared by actual New Game and Load
Game. **Tab** opens it when gameplay is ready and the cinematic letterbox is closed.
**Tab** or **Esc** closes it. Tab is handled before Godot's focus navigation;
only a non-echo key press toggles the view. Opening pauses the tree; closing releases only the
pause owned by this host. The view binds the actor's existing inventory Resource.
It never inserts items into New Game, restores a fixture or changes save schema.
A missing generated scene/font/artwork reports an explicit required-assets error.

Selecting an owned `Weapon` calls the maintained selection API for the same
`selected_weapon` used by gameplay and saving. Item quantity queries/removal retain their existing semantics,
including present negative sentinels. Category up/down and six-slot row
left/right motion now use native `AnimationPlayer`/`AnimationLibrary` tracks
with Fix.sda's distance, rate and linear timing. New directional input is held
until a transition completes; closing/reopening resets interrupted motion. The
source does not establish an open/close tween, so that lifecycle remains
immediate.

Potion/magic/object effects, dynamic item permission changes, source pickup-order
preservation, status gauges/text and configurable controls remain unfinished.
The source names Armor, Magic, Sword and Pommeau gauges plus era/time and fading
settings, but the `Inventory.ini` layout values and its drawing callback are not
available in the installed loose data/decompilation. These HUD-adjacent elements
remain tracked as an evidence gap rather than guessed controls or values. The
initial catalogue includes 46 items with corroborated Stock category bindings;
16 source entries without those bindings are omitted rather than assigned to an
invented category. This is an inventory presentation and owned-weapon selection
increment, not complete original inventory parity.

The original manual (`disc/HypeManual.pdf`, printed pages 12–15 and 18) specifies
Tab for inventory and Esc for a separate pause menu, with Resume, Load another
adventure, Configuration and Return to main menu. The separate
[pause composition](../menu/README.md#pause-menu-composition) follows the installed
English source's "start a new adventure" action where it differs from that
manual. Esc closes inventory without opening pause or leaving the session.

For designer changes, inherit the generated scene and save it under
`game/inventory/`. Assign it to the `InventoryHost.inventory_scene` property in
an authored inherited session scene. Change native offsets, Theme, textures,
labels or item Resources there. The exported `inventory_key` changes the authored
keyboard shortcut. Regeneration preserves authored scenes and unrelated generated
packages; modifying a generated base in place triggers the existing safeguard.

[Source format and evidence](../../../docs/openspace/inventory.md) describe
conversion boundaries. The focused inventory runner checks source/Rete/native
conversion, admitted ownership and animation timing/reset. Its independent
presentation evidence is separate from actual session entry.
