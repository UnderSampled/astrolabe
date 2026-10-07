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
Configure opens the source-backed Images, Controls and Sound tabs. Sound uses
native persisted volume controls. Controls remaps the five gameplay actions
currently consumed by the Godot session, with physical keyboard keys or generic
Godot joypad buttons and axes. Reassigning an occupied input swaps the two
actions. Escape cancels a capture before closing configuration. Reset restores
Astrolabe's current arrow/Space defaults and clears controller mappings; it does
not claim to recover the game's original options file defaults. The legacy
Paddle/Joystick device modes are not emulated. Images retains the original
labels and explains why source renderer settings have no verified Godot
equivalent. Credits opens a black native screen using the confirmed source
background, notes that the config declares 397 entries but lists 400 active
keys, and returns to the main menu through Back or Escape. The English text map
and timed scroll are still unimplemented; the screen never presents internal
keys as credit names. Internet explicitly reports that the old route is
obsolete and does not launch the external URL. Menu music and spoken idle
prompts remain outside this increment.

Generate the independent menu closure with the same converter used for gameplay:

```sh
target/release/astrolabe port disc godot --level manoir --locale en --content menu
```

The `menu` publication owner is separate from `manoir`, so menu regeneration
preserves generated gameplay. It also exports
`generated/hype/shared/menu/configuration/en/menu.tscn` with the original
configuration backdrop and all three source tabs. The Sound page has native
vertical Music, Sound and Voices sliders. They store source-scale values from 0
to 127 in `user://hype-settings.cfg` and apply them to the Astrolabe Music,
Effects and Voice buses. The Images page identifies source shadow quality,
camera-to-viewport adjustment and OpenSpace texture swapping as unsupported.
The Controls page provides keyboard and generic controller capture, current
binding labels, swap-on-conflict behavior, Escape cancellation and an Astrolabe
defaults reset. Preferences persist alongside sound values in
`user://hype-settings.cfg`; only the five gameplay actions already consumed by
the session are exposed.
Missing menu or configuration media produces an explicit error; no replacement
artwork is synthesized. All extracted media and import caches remain ignored.

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

## Pause menu composition

Generate the installed game's separate pause presentation:

```sh
target/release/astrolabe port disc godot --level manoir --locale en --content pause-menu
```

Its independent `pause-menu` owner publishes
`generated/hype/shared/menu/pause/en/menu.tscn`. Four native Buttons preserve
the source labels, font/glow images, initial Resume selection, depth-projected
anchors and keyboard neighbors. The original root has no backdrop; the native
Control overlays the existing paused game. Original item images and feedback
streams are mandatory. Missing media reports an error; it does not supply
replacement artwork or route away from the session.

`game/pause_host.tscn` composes it in the real session. Esc opens and closes
pause; Esc in inventory closes inventory first and does not fall through to
pause or the main menu. Resume releases only the host's own pause, retains the
same gameplay objects and writes no save. The installed `NextGame` action says
"start a new adventure" and uses the existing New Game route after session
teardown. The manual's contradictory load description is documented in the
source note. Main menu uses the existing bootstrap route. Configure overlays the
same native sound settings scene while gameplay remains paused; Esc returns to
the pause menu without resuming the game.
Tab retains its manual inventory default. Closing a menu cancels pending audio
confirmation so an old timer cannot route after closing or reopening.

For layout/art changes, inherit the generated Control scene under `game/menu/`
and assign it to `PauseHost.pause_scene` in an authored inherited session scene.
Edit native offsets, Theme, textures or node properties there; generated asset
regeneration preserves the authored composition. Game routing and pause
ownership remain in the authored host. No gameplay, loading or HUD system is
introduced by this slice.
