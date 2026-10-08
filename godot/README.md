# Hype Godot game

Open `project.godot` with Godot **4.7.2**. The menu requires its installed Hype
artwork and native menu scene; play requires the generated manor. `addons/astrolabe/` is maintained converter support;
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
target/release/astrolabe port disc godot --level manoir --locale en --content menu
target/release/astrolabe port disc godot --level manoir --locale en --content inventory
target/release/astrolabe port disc godot --level manoir --locale en --content pause-menu
```

Open `project.godot` in the Godot editor, wait for texture and sound imports to
finish, press **F5**, and select **Load Game**. Running the project player
before that first editor import leaves generated textures unavailable. Load
Game first uses a native Godot slot if one exists; otherwise it reads the
imported original slot from `game/original_save_import.json`. If neither is
available, it uses the measured English manor seed. The normal generated
`manoir/level.tscn` and compiled Hype Brains make gameplay decisions. Authored
game code provides the camera, control guide, mana display, save operation and
menu. The gate runs its imported source AI after saved state is applied; there
is no raised-gate position workaround. The camera uses the measured initial
manor view and a simple actor follow; the source camera update remains under
investigation.

Controls: **Arrows** move and turn, **Shift** runs, **Ctrl** jumps, **X** draws
the sword, and **Space** uses it. To prepare the saved character for Fire, step
back with **Down**, press **X**, then press **Enter** to cast. **R** reloads the
provisional save start; **Esc** opens the separate pause menu. Resume or **Esc**
returns to the same session; "start a new adventure" uses the existing New Game
route, and "main menu" returns to the main menu. Configure remains unfinished.
**Tab** opens the inventory during gameplay; **Tab** or
**Esc** closes its modal view. Selecting an owned weapon updates the existing
inventory Resource; other item effects remain explicit unfinished actions.
[inventory ownership](game/inventory/README.md) explains source content and editor overrides. The supported slice includes
movement with contact, stationary/moving/wall jumps, sword selection and beam lifetime, and Fire cast
with mana cost. Combat hit/damage, crossbow play, spell contact/lifetime, cliff
death and complete camera parity are not established. Generated animation
libraries use compressed native `.res` resources; their named states remain
editable and seekable in the AnimationPlayer inspector. Load Game prepares the
saved native scene in the background and reports its loading stage. Reload and
return-to-menu retain one loaded scene resource while creating fresh gameplay
instances. See the [loading measurements](../notes/godot-loading-efficiency-2026-09-27.md)
for the remaining fresh-process cost and retained-memory tradeoff. The menu is imported independently from its source layout;
[menu ownership](game/menu/README.md) explains editor overrides and remaining submenus.

To import an extracted original Hype slot, run from the repository root:

```sh
target/release/astrolabe import-save /path/to/SaveGame0 \
  --profile hype-pc-english-supplied-save
```

The command writes `game/original_save_import.json`, which is ignored local
game data; the original files are read without modification. If no native
Godot slot exists, **Load Game** reads Hype's saved position and recoverable
orientation, mana, selected magic slot, supported inventory numbers, manor
lever state, and Global `DSG[149]`. It retains the raw source records in the
import report while runtime applies only canonical supported fields. The
supplied authentic SaveGame0 has Global `DSG[149] = true`. Gate AI model 272
reads that value, moves its own actor through the compiled source position
operation, and enters its next behavior. This replaces the former raised-gate
constant; the importer does not manufacture a gate pose or progression flag.
Current AI behavior and active-weapon selection are not imported. Native Godot
saves also capture Hype's transform and the Global gate-sequence Boolean, so
subsequent Load Game restores those supported fields. The status text labels
imported originals as supported-fields-only. See the [save importer evidence](../docs/openspace/save-games.md#import-an-original-slot-into-godot).

The current session still uses the historical `game/demo.*` and `demo_camera.gd`
filenames. They are game code; the names do not define a separate demo product.

## New Game

After generating the manor and importing its assets, run the project and select
**New Game**. This opens the complete generated manor at its original starting
state, with its compiled AI running. It does not restore a save, lift the bridge
gate or substitute the camera-test excerpt. Existing saves are preserved; saving
is disabled for New Game. **R** restarts New Game and
**Esc** pauses. **Load Game** keeps its existing save route.

The imported viewport registration selects the source camera at startup. Its
compiled AI and native camera component own the subsequent shots. The source
dialogue and letterbox calls share the application's presentation owner. The
session reports completion when Global147, Hype's gameplay behaviors, restored
camera preset/target and letterbox state reach the authored handoff together.
Unsupported actor behaviors retain exact console diagnostics. Full opening
acceptance is tracked in the dated Manor integration audit; a first dialogue or
successful conversion alone does not establish it.

The [current checkpoint](../notes/intro-integration-wrapup-2026-10-08.md)
rebuilt all four game scopes. Uncapped New Game startup stalled at50%; a temporary
30-FPS diagnostic reached readiness, then exposed missing selected armor in
World154 and an unmapped camera target mechanics callback. Full opening
acceptance remains blocked. The diagnostic cap is not installed in production.

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
Existing adopted projects can opt into warm Load Game caching by adding the
`GameContent` autoload at `res://game/content_cache.gd` to their owned project
settings; the game still loads without this autoload.

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
