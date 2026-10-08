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
Configure opens `game/settings_menu.tscn`, an authored route alias for the
source-generated configuration scene. The scene root keeps the existing
`closed` signal, `focus_initial()` and `cancel_input_capture()` interface while
leaving its source artwork and native page layout under generated ownership.
The source-backed Images, Controls and Sound tabs use native persisted volume
controls. Controls remaps the five gameplay actions
currently consumed by the Godot session, with physical keyboard keys or generic
Godot joypad buttons and axes. Reassigning an occupied input swaps the two
actions. Escape cancels a capture before closing configuration. Reset restores
Astrolabe's current arrow/Space defaults and clears controller mappings; it does
not claim to recover the game's original options file defaults. The legacy
Paddle/Joystick device modes are not emulated. Images retains the original
labels and explains why source renderer settings have no verified Godot
equivalent. Credits exports all 400 English key/value records and the original
`Playmo_Font2_0` atlas into an editable native scene, while showing the first
397 entries as the source routine does. It retains the original black
background, paired left/right rows and source spacing, with Back/Escape return
navigation and automatic return when the final row scrolls offscreen. The
decompiled routine's timer multiplier initializer is absent from the available
decompilation, so the authored credits controller exposes an editable default
scroll rate; pixel-timed parity remains unverified. Internet explicitly reports
that the old route is obsolete and does not launch the external URL. Menu music
and spoken idle prompts remain outside this increment. The source Sound page
also names Stereo Test and Reverse Stereo; those entries are not exposed because
the current Godot mixer has no verified equivalent.

Generate the independent menu closure with the same converter used for gameplay:

```sh
target/release/astrolabe port disc godot --level manoir --locale en --content menu
```

After exporting menu content, check the authored Options route against the real
generated scene with:

```sh
godot --headless --path godot --script res://tests/settings_menu_route_smoke.gd
```

The smoke check verifies the forwarded route API, source pages, three audio
sliders on the original 0–127 scale, five implemented action bindings, the
explicitly unsupported image settings, and Back-to-menu signaling.

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
