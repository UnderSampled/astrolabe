# Hype Godot game

Open `project.godot` with Godot **4.7.2**. The menu works without Hype data;
play requires the generated manor. `addons/astrolabe/` is maintained converter support;
`game/` is independently authored content. All extracted/generated content and
Godot caches are ignored by location.

This project is the Godot implementation of the game. Its basic menu is the
current game menu; loading, savegame and UI work evolve this same application.

## Load the manor save

With the supplied English Hype disc under `disc/`, generate the manor from the
repository root:

```sh
cargo build --release --locked
target/release/astrolabe port disc godot --level manoir --locale en
```

Open `project.godot` in the Godot editor, wait for texture and sound imports to
finish, press **F5**, and select **Load Game**. Running the project player
before that first editor import leaves generated textures unavailable. The
current load route restores the measured starting state of the supplied English `SaveGame0` using
`game/english_save_manoir.json`. This is a provisional saved-game restoration;
general savegame support will replace it. New Game is not implemented yet and
will start with the intro. The normal generated `manoir/level.tscn` and compiled
Hype Brain make the gameplay decisions. Authored game code provides the camera,
control guide, mana display, reload and menu. The session currently starts with
the bridge gate raised as a temporary workaround, separate from the restored
save values; its opening AI is not implemented yet. The camera uses the measured
initial manor view and a simple actor follow; the source camera update remains under
investigation.

Controls: **Arrows** move and turn, **Shift** runs, **Ctrl** jumps, **X** draws
the sword, and **Space** uses it. To prepare the saved character for Fire, step
back with **Down**, press **X**, then press **Enter** to cast. **R** reloads the
provisional save start; **Esc** returns to the menu. The supported slice includes
movement with contact, stationary/moving/wall jumps, sword selection and beam lifetime, and Fire cast
with mana cost. Combat hit/damage, crossbow play, spell contact/lifetime, cliff
death and complete camera parity are not established. Loading is currently slow;
the loading-efficiency work will improve resource preparation and readiness in
this Load Game flow. UI parity work will replace the current menu presentation.

The current session still uses the historical `game/demo.*` and `demo_camera.gd`
filenames. They are game code; the names do not define a separate demo product.

The project needs the locally generated assets to play. They are ignored by
Git and must be regenerated on another machine. Open
`generated/hype/levels/manoir/level.tscn` directly to inspect the saved scene,
characters, native animations and compiled scripts.

## Work in the editor

Open generated level and perso scenes directly to inspect their saved contents
and AnimationPlayer libraries. The project menu is the game's normal entry point.

To adopt this structure elsewhere, run `astrolabe init-godot PROJECT` once.
`port` also initializes a missing project. Project settings and authored files
belong to the adopter after initialization. Use `update-godot-support` for an
explicit support update. Ordinary content regeneration preserves these files
and unselected generated levels, and rejects modified generated files.

Build variations by instancing generated scenes in `game/levels/` or
`game/characters/`, adding your own nodes/scripts/resources there. Commit only
independently authored work. Such compositions need their documented generated
dependencies rehydrated before they can open fully. Editing a generated file
directly causes a regeneration conflict; move a deliberate independent copy
under your ownership if it should stop receiving converter updates.

`tests/example.tscn` is a development fixture. It uses the maintained character support with a native,
independently authored rotating cube animation. Select its AnimationPlayer to
inspect or scrub it without the disc. `game/characters/luminous_hype.tscn` is an
authored inherited scene: a scaled Hype with an original blue beacon material,
mesh and light. It requires a converted Astrolabe level to be generated. Its authored properties survive
regeneration while the generated character's native poses/resources update.

Perso filenames use the source name plus a stable source-identity suffix;
family/configuration-specialized libraries live beside the owning level's
`characters/` directory. State names remain `state_ID` because the source states
are numeric. The animation inspector exposes original frame count, state speed,
source animation identity and next-state metadata. Select `restored/state_ID`
to inspect the separately converted reset configuration. Shared models/materials
remain under their source role and keep independent resource identity.
