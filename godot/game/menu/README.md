# Main menu ownership

The English source menu is exported to
`generated/hype/shared/menu/main/en/menu.tscn`. It is an editable native Control
scene with six Buttons, TextureRects, a Theme and AudioStreamPlayers. Positions,
alignment, initial selection, explicit navigation links, action identities and
normal/glow artwork come from the original serialized menu definitions.
`addons/astrolabe/ui/menu/` provides generic native behavior. No source records,
font table, menu interpreter or source-disc access are needed at runtime.

`game/bootstrap.tscn` composes that scene and owns application routes.
`NewGame` opens the actual manor opening through `game/new_game.tscn`;
`SavedGames` keeps the existing Load Game/save route through `game/demo.tscn`.
The named `NewGame` and `LoadGame` Buttons remain discoverable by session tests.
Configure, credits and internet remain visible and selectable; activation reports
that their submenu has not been implemented. It does not invent an application
backend or open an obsolete external link. Menu music and spoken idle prompts
remain outside this increment.

Generate the independent menu closure with the same converter used for gameplay:

```sh
target/release/astrolabe port disc godot --level manoir --locale en --content menu
```

The `menu` publication owner is separate from `manoir`, so menu regeneration
preserves generated gameplay. The complete background, item images and navigation
and confirmation streams are required. Missing assets produce an explicit error
and an exit button; no replacement menu artwork is synthesized. All extracted
media and import caches remain ignored.

To improve the menu in the editor, create an inherited scene from the generated
`menu.tscn`, save it under `game/menu/`, and assign it to the bootstrap's
`menu_scene` property. Save that bootstrap override under `game/` as well.
Adjust native node offsets, Theme overrides, textures, audio, or action handlers
there. Those authored files survive regeneration. Editing a generated base file
in place intentionally causes the converter's modified-output safeguard to
reject regeneration. Keep action identities `NewGame`, `SavedGames` and `Quit`
when extending the supplied bootstrap; other consumers can handle the generic
`action_selected` signal themselves.

Source details are documented in [the menu format note](../../../docs/openspace/menu.md).
