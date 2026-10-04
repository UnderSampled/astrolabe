# Godot project structure and ownership

This guide defines ownership and regeneration rules for the Godot project. It complements the [native conversion contract](./rust-architecture.md#godot-adapter-native-project-conversion).

## Develop the Godot target alongside parity work

Converter-owned Godot assets may be introduced and maintained now when a
supported export needs them. Shaders, reusable components, resource classes,
and engine-side functions belong in a coherent, versioned support package.
Godot output is an ongoing semantic test of OpenSpace interpretation and
Model/Rete fidelity: a deliberate change should survive persistence and have
the expected effect in native Godot content. Full package reconstruction is
not a prerequisite for useful support assets or authored consumers.

The earlier blanket deferral guarded against committing to the wrong target
architecture. The native scene/resource/GDScript direction now supplies that
boundary. Add support for concrete needs and test its interfaces as it grows;
do not design the entire engine API in advance. Most AI engine functions can
wait until an AST-to-GDScript behavior test needs them. A shader required by a
current material slice need not wait for AI work.

## One project in two states

The repository's authored Hype project is the actual Godot game, and its menu
is the main menu. **New Game** is the canonical entry for intro integration and
acceptance: it loads the complete generated manor from the original starting
state and runs the full imported AI. The provisional **Load Game** route restores
the measured supplied-save state. Both routes share one game-session lifecycle.
The current `game/demo.*` filenames are historical session filenames.

New Game initialization, camera and presentation remain incomplete. The authored
follow camera provides a view until source camera initialization runs, and
saving during the unfinished opening is disabled. Those are fidelity gaps in
the game. Intro parity must compare this normal menu entry with the unskipped
[original reference](../../notes/manor-intro-reference-2026-10-03.md) through its
recorded return to gameplay. Isolated or excerpt fixtures are diagnostics and
cannot replace the full path. Preserve existing saves and keep source Brains
and imported behavior unchanged in that acceptance run.

Evolve one menu and game-session flow. Loading-efficiency work can establish
clear resource preparation, saved-state restoration, readiness and session
teardown boundaries. Savegame work replaces the fixed restoration with real
save/load, and UI parity work replaces the basic presentation while using the
same game operations. Keep gameplay in the generated AI and maintained native
support; test harnesses observe and drive the actual game rather than provide
another player implementation. Temporary gate/camera/initialization limitations
remain explicit fidelity debt in the game and its evidence.

### Authored session components

`game/demo.gd` coordinates both the saved-start and New Game routes. Its
historical path is retained; it is the shared game session. It owns start-state
selection, native level instantiation, actor binding, save application, readiness
stages, UI routes and generated-Brain startup. Its existing observable fields,
stage signal and methods remain the entry points for game observers.

Two focused native components keep infrastructure out of that coordinator:

- `game/level_preparation.gd` loads the saved PackedScene in one background
  dependency worker, uses the optional `GameContent` resource cache, and owns
  the temporary loading frame limit. Cancellation ends the departing session's
  wait and restores that limit; Godot retains ownership of an in-flight request.
  Every game session still instantiates fresh nodes from the prepared resource.
- `game/crossbow_input.gd` routes physical keys between movement and camera aim,
  projects the authored camera sight ray, and restores the pre-session input
  bindings on teardown. The generated Brain continues to own shooting,
  ammunition and projectile behavior. This presentation component supplies no
  alternate gameplay rules.

Leaving, failing or freeing a session tears down both components. The same
resource preparation and lifecycle serve New Game and Load Game; their distinct
initialization remains explicit. The saved-start route retains its measured
fixture and temporary raised gate. New Game preserves the complete imported AI,
without fixture application or gate intervention. These limitations are not
removed by extracting infrastructure into components.

### Native checkpoint save contract

The shared native inventory now separates complete item definitions from
admitted category slots. See the [source inventory contract](../openspace/inventory-gameplay.md)
for the supported AddObject/weapon-selection/capacity paths and source evidence.
Legacy sparse fixture values still load when admission cannot be established;
pickup and selection report that narrower state explicitly. Native schema 3
captures admitted slots, gold, capacities, represented selections and admission
status, together with the current armor damage state.

`game/save_state.gd` now writes native **schema 3** into the existing
`user://manoir-save-v1.json` slot. Its `manor-idle-v1` checkpoint includes the
player's native `global_transform` (origin and three Basis columns),
`source_hit_points.current`/`maximum`/`limit`, `source_magick` points and spell
selection, `source_inventory` snapshot, `source_armor` snapshot, and the dummy
Brain's `dsg_8`. The legacy player payload key remains readable; schema 2 and 3 explicitly
identify `level:/persos/world/0/0`; both session loading and persistence resolve
that asset through `source_actor_nodes` and verify its `source_perso` identity.
The world flag owner is resolved through `source_actor_brains`.

Capture is deliberately limited to the measured controllable idle: action 47,
Brain normal 4/reflex 3, a running supported Brain, native floor contact and
settled velocity/action transitions. Gameplay controls must be released and no
held or active generated projectiles may remain. These are authored checkpoint
admission rules, not a reconstruction of original save scheduling. A rejected
save explains the unmet condition and leaves the previous slot intact. Life
values are positive byte-sized integers with current <= maximum <= cap;
placement must remain finite in Godot's native vector precision and have a
nonsingular Basis. Unknown payload fields, identities and versions are rejected.

The supplied English fixture still seeds the player and supported manor state.
Schema 3 then restores placement/life and equipment before physics or Hype Brain
startup. Both equipment Resources expose capture, validation and application
contracts; shape checks, source identity/order checks and application on private
clones finish before changing any live owner. Inventory quantities/categories
and capacity-owning definitions are copied; armor definitions and pieces are
copied. Reordered or regenerated definitions/pieces with different source keys
cannot silently receive an old snapshot.

**Schema 1 and 2 remain loadable:** schema 1 retains seeded placement/life;
schema 2 restores those values. Both retain seeded armor/gold/other selections
and migrate their sparse inventory view, preserving unknown admission as an
explicit native limitation. Reading/loading does not rewrite an old slot; only
an accepted explicit save upgrades it to schema 3. Temporary-file replacement
remains in use. New Game saving stays disabled.

Source grounding is deliberately narrower than full original-save parity:
Raymap's `StandardGame` is the first structural reference, while the pinned-build
[save field evidence](../openspace/save-games.md#registration-and-restoration-findings)
corrects the Hype current/maximum/cap fields and identifies saved player position
prefixes. The current useful `HitPoints` model and native exporter project those
three source bytes into the Resource. The supplied fixture and
[recorded control observations](../../notes/hype-control-progress.md#original-game-observations)
identify idle normal/reflex 4/3 and settled action 47. Local decompilation is
absent in this checkout; exact original checkpoint restoration and arbitrary
actor-state continuation remain unverified.

This extension is implemented but has only source review in the current work.
Deferred acceptance must save after changed placement and life, reload in a
fresh session/process, and demonstrate restored placement, life, inventory
slots/quantities/gold/selections/capacities, armor rank/piece damage/current
piece, mana and the existing world flag without changing the source fixture. It must
also exercise schema-1/2 loading/upgrading, invalid placement/life/equipment rejection and
saving during movement, casting or active projectiles. Complete actor/world
progression, source-save decoding, arbitrary animation/AI continuation and
original checkpoint behavior remain outside this bounded native contract.
Armor snapshots retain selected definition, resistance rank, broken-piece
count, current damage piece, and each piece's broken/strikes values. Artwork,
damage coefficients and thresholds remain authored assets. Inventory Resource
snapshot version 2 retains ordered catalogue member quantities, selected flags
and capacities, admitted slots, gold and admission status. Distinct members
with repeated info IDs remain distinct; per-member flags retain exact-kind
Weapon/Magic replacement selection effects. The first-ID quantities and
Weapon/Object/Magic selection properties are derived views of those members.
Ordered replacement rules remain generated definitions rather than mutable
snapshot state. These branches are source-reviewed; native gameplay acceptance
remains unverified.

Inventory presentation uses `presentation_quantities()`, rather than treating
the full catalogue quantity dictionary as ownership. With known admission it
projects admitted slots; sparse legacy saves with unknown admission retain
their saved ownership in `legacy_owned_ids`. This keeps the saved-start UI
usable without exposing every imported sentinel or claiming a recovered slot
order. Admission-dependent pickup and selection still report the unresolved
membership. Schema 3 snapshots retain that fallback and its diagnostic.
Native RNG state and other unrepresented actor/world state are omitted;
schema 3 is not general continuation or source-save interoperability.

Design from a Godot developer's workflow: find a level or character, open its
scene, inspect its resources, make an authored variation, and regenerate source
content without losing that work. Use the same `res://` arrangement in both
states; rehydration fills in converted content without reorganizing the project.

| Area | Dehydrated: committed source | Rehydrated: local working project | Owner |
| --- | --- | --- | --- |
| Project settings, setup docs, bootstrap | Present and understandable without the disc | Same files and developer settings | Starter maintained by Astrolabe; consumer owns its adopted copy |
| Godot support package | Readable `.gd`, `.gdshader`, scenes/resources, docs and synthetic fixtures | The same versioned support files used by converted assets | Astrolabe |
| Converted Hype content | Absent; generation instructions and ignore rules are committed | Native level/perso scenes, meshes, materials, animations, media and generated GDScript | Exporter-managed, game-derived and local |
| Authored game content | Committed when independently authored | Same scenes/scripts/overrides, resolving generated dependencies after rehydration | Game/remaster developer |
| Import caches, staging and evidence | Absent | Derived local files | Godot/conversion tooling |

Use a Godot-facing project directory separate from Rust source, Rete packages,
the disc and comparison reports. A project can use this arrangement:

```text
godot/
  project.godot
  README.md
  .gitignore
  addons/astrolabe/
    README.md
    rendering/             # shaders and related support resources
    audio/                 # native voices and compiled-event support
    character/             # native animation controls and typed parameters
    scene/                 # native sector inspection
    behavior/              # engine functions for generated GDScript, as needed
  game/
    bootstrap.tscn         # source-free entry point and content availability
    bootstrap.gd
    levels/                # authored compositions/variations, as needed
    characters/            # authored compositions/variations, as needed
  tests/                   # synthetic support fixtures and optional inspection
  generated/hype/         # ignored; populated locally by conversion
    levels/astrolabe/
      level.tscn
      persos/
      characters/          # family/configuration-specialized native libraries
      ai/models/           # generated GDScript for supported behavior assets
    shared/                # Fix-owned families, reusable scenes and resources
    textures/
    sounds/
  generated/astrolabe-manifest.json
  .godot/                  # ignored; Godot-owned cache
```

Create only the directories needed by implemented slices. `godot/` is the
committed starter/reference project; independent consumers can adopt the same
arrangement in their own repository. The reusable support package has one
authoritative source there, also used when distributing it with the CLI.
Converter-only build scripts remain with the adapter; inspection harnesses
remain distinguishable from shipped support and consumer gameplay.

Within generated content, group a scene with its privately owned resources;
give independently shared resources a clear shared location. Preserve level
and source-package context, identity and meaningful tree relationships without
making developers navigate wire-record IDs or a global folder per struct kind.
Compile AI GDScript beside its selected level's Perso scenes when Fix-to-level
references are bound in that selection; the canonical Fix model remains shared
in the useful asset layer. Each level supplies its own live actor bindings.
Prefer meaningful file/node names, documented stable disambiguation for source
name collisions, and visible animation names. Exact filenames remain subject
to the first native scene/character probe; document changes to referenced paths.
This uses Godot's recommendations to group dependencies near their scenes and
place reusable third-party support in `addons/`, even without an editor plugin.
An EditorPlugin is optional tooling, not a requirement for loading the game.
See [Godot project organization](https://docs.godotengine.org/en/stable/tutorials/best_practices/project_organization.html).

## Support package boundaries

Support assets implement Godot behavior and rendering. They depend on Godot and
their own declared support resources; they must not depend on a developer's
checkout path, the Rust converter process, Rete serialization or the disc.
Generated resources/scripts depend on documented support interfaces. Authored
game scenes compose or extend those generated assets and support components.
Reusable support must not hard-code a particular generated Hype scene as its
global environment.

For example, a material conversion may write a ShaderMaterial referencing a
committed Astrolabe shader and supply texture/uniform values as native resource
properties. Document the shader's units, parameters, rendering assumptions and
known approximations beside its source. Test that shader with both a synthetic
material and the relevant imported/Rete-reloaded material; do not maintain a
second preview implementation that bypasses the shipped shader.

Expose typed properties, resource references, methods and signals for required
engine behavior. Supply level/instance context through scene composition and
explicit bindings. Keep components usable in standalone scenes; introduce an
autoload only for an evidenced project-wide service. Godot's
[scene organization guidance](https://docs.godotengine.org/en/stable/tutorials/best_practices/scene_organization.html)
supports independent scenes with dependencies supplied by their surrounding
context. Preserve the ban on runtime asset-table/AI interpreters: the support
package operates on converted native content and generated GDScript.

Version support interfaces together with the exporter that emits their callers
and resource properties. Record that version in conversion provenance and check
compatibility before publication. Ship static support files from their single
source; do not duplicate shader/GDScript bodies in Rust strings. A support
upgrade is an explicit package update with changed-file review, distinct from
ordinary content regeneration. In the Astrolabe source checkout, version control
owns the support files; the exporter must not overwrite them with bundled copies.

## Editing and regeneration

The committed starter should open to a source-free bootstrap and allow editing
support assets and running synthetic examples before game content is generated.
Document the source location, edition/locale selection, conversion command,
generated level entry points and supported content. Check availability
before loading a generated entry scene; ordinary loading of an already converted
PackedScene is sufficient. Keep missing content out of unconditional bootstrap
preloads and autoloads. Authored scenes referencing converted resources require
rehydration to open fully; identify those dependencies rather than supplying
fake converted content or claiming the whole game runs dehydrated.

Generated scenes expose their nested scene instances in the Scene dock by
default, including the level inside `preview.tscn`. Conversion writes Godot's
`editable` instance metadata at every nesting boundary; it preserves native
PackedScene references and sharing. Expand `Model0` in the preview to browse the
level tree, or open the neighboring `level.tscn` directly. Durable authored edits
still belong in `game/` compositions or inherited scenes.

In the rehydrated project, developers open generated level/perso scenes directly,
inspect native animation libraries, and build authored scenes by instancing,
composition or suitable scene inheritance. Offer useful exported properties,
signals and resource overrides so common edits need not patch generated files.
An independently owned duplicate can support deeper changes; document that it
then stops receiving changes from its generated original. Keep independently
authored changes in version control. Changes incorporating extracted game
content remain local under the same game-data policy as other generated assets.

Treat project initialization, support-package updates, and content regeneration
as separate ownership operations. Initialize `project.godot`, input settings,
autoloads, bootstrap and example files once; preserve an adopted project's
settings thereafter. Report any newly required settings explicitly instead of
silently replacing the consumer's configuration. Extend existing staged
publication and manifest checks to cover the new arrangement. Only previously
owned, unchanged generated files can be replaced or removed as stale. Preserve
authored files, local edits, unselected levels and unrelated generated packages;
report collisions or modified generated files before publication.

Ignore generated directories and caches by location. The repository's blanket `*.tscn`/`*.tres` ignore rules have been removed;
authored scenes and resources are tracked. Keep support
script/resource identity files and import settings required by the pinned Godot
version in version control with their owners; derived `.godot/` caches stay
local. Generated identity/import sidecars follow generated ownership. Confirm
the exact rules on the pinned version using Godot's
[version control guidance](https://docs.godotengine.org/en/stable/tutorials/best_practices/version_control_systems.html).

## Acceptance and migration

Start with one native level/character slice and any shader/component it needs.
Use it to establish the concrete paths, support interface and extension seams;
then migrate the existing preview and remove superseded production loaders.
Check the developer workflow as well as the converted values:

1. A clean dehydrated checkout exposes readable support/project files and opens
   its bootstrap and synthetic examples without requiring game-derived assets.
2. Rehydrate a selected slice through direct import and reopened Rete; both
   routes produce equivalent native assets using the same support package.
3. Open the level and character in the editor, scrub animations, inspect a
   shader/material, and make an authored scene/resource variation using ordinary
   Godot operations. Run that variation against source-informed expectations.
4. Regenerate after a deliberate model edit. Authored settings/variations and
   unrelated levels survive, references still resolve, and the intended native
   result changes. A conflicting local edit is reported before publication.
5. Exercise a support-version update and a clean import-cache rebuild. Diagnose
   incompatible generated/support versions; verify saved node/resource identity
   and supported authored extension points across the supported update.

Keep these checks within the existing whole-suite deadline. Record interface
coverage and known gaps as features land. Consumer dependencies may be developed
now; broad compatibility promises and full-game parity require their own
evidence, not a blanket waiting period for all adapters to finish.
