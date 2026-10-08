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
The shared production root is `game/game_session.gd` / `game_session.tscn`;
`session_camera.gd` owns the authored fallback/aim camera.

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

`game/game_session.gd` coordinates both the saved-start and New Game routes. It owns start-state
selection, native level instantiation, actor binding, save application, readiness
stages, UI routes and generated-Brain startup. Its existing observable fields,
stage signal and methods remain the entry points for game observers.

`game/level_transition.gd`, installed as the `LevelTransition` autoload, owns
threaded resource requests, progress, cancellation, and the loading presenter.
Both initial loading and map changes delegate installation back to the current
session. It temporarily limits loading/installation to 30 FPS when the prior
limit is zero or higher than 30, preserving lower limits. Completion, failure,
cancellation and service exit restore the prior limit idempotently. Cancellation
restores immediately while the uncancellable native loader request drains; that
request still occupies the service slot until drained. The cap is a measured
startup mitigation; a particular engine starvation or lock mechanism is unproved.
The session's older direct-loading fallback also has a temporary cap.

The session currently owns crossbow input and camera sight-ray routing directly.
`game/level_preparation.gd` and `game/crossbow_input.gd` remain preserved packaged
components from the earlier fork, but the actual session does not instantiate
them. Their existence does not establish a connected production refactor.
`GameContent` may cache prepared Resources; each session still instantiates fresh
nodes. Save restoration and actor/Brain/physics readiness remain session-owned.
The saved-start route restores the source-backed sideways gate through Global
DSG[149]. New Game preserves complete imported AI and does not apply that fixture.

### Native save contract

The production `game/save_state.gd` writes **schema 4** checkpoints and reads
schemas 1 and 4 in `user://manoir-save-v1.json`. Schema 1 remains the bounded
original-save import overlay. Schemas 2/3 from the archived fork are not admitted.
The legacy player key is `world/0/0/Hype`; schema 4 also records and verifies
`asset: level:/persos/world/0/0` and `checkpoint: manor-idle-v1`. The session resolves
that semantic player through the level's named node map and checks its ancestry,
actor metadata and manor context. Source addresses and relocation remain private
to the OpenSpace reader and diagnostic evidence.

Checkpoints capture native transform, magick, full inventory v2 snapshots,
actor-owned life v1 and level-owned armor v1. Inventory retains ordered member
identities, capacities, quantities, selection, admitted category slots and special
gold. Armor updates the shared level catalogue in place and synchronizes its
facade; another level's clone remains independent. Life's three byte values are
validated independently without clamping. Placement admits finite nonsingular
scaled/sheared Basis values. Integral JSON floats are accepted after range checks;
Boolean/fractional/nonfinite numbers and noncanonical item keys fail.

Supported Brain/world state includes Global `dsg_149`, optional `dsg_147` and
first-pickup flags `dsg_220` through `dsg_228`, the dummy's `dsg_8`, and four placed
barrels' consumed flags and removal tombstones. Barrels are actor instances,
independent of shared model 488 and transient spawned drops. Local named bindings
must be valid; a known removed actor can remain absent during repeated restoration.
Missing or incompatible live fields fail before any component mutation. Private
inventory/life/armor copies validate restoration before publication.

Schema 1 may omit complete inventory snapshots, life/armor, Basis and optional
Brain fields. Sparse legacy inventory restores its supported IDs and reports
unknown admission to pickup/selection callers; omitted life/armor and optional
Brain state retain their level defaults. Missing older world state uses the
measured dummy default. Reading does not upgrade files. Writing replaces the
slot atomically and admits a living, grounded, still player in action47,
Normal4/Reflex3 with a running supported Brain, no pending movement/action,
held/generated projectile or pressed gameplay control. New Game saving stays
disabled in the authored session. This is a bounded native checkpoint policy,
not arbitrary AI/coroutine, RNG or general world-progression restoration.

The useful broader fork capabilities are composed through this concrete native
slot and its fresh-process regression, rather than replaying its old schema3
session. The [integration disposition](../../notes/integration-disposition-2026-10-08.md)
and [checkpoint evidence](../../notes/native-checkpoint-restoration-2026-10-08.md)
record implementation, source distinctions and current acceptance limits.

Source-save import is also bounded: the compatibility import maps measured
semantic fields into the supported schema-1 payload. Optional Global DSG[147]
is not evidence for a granted key or later story progression. Existing frozen
route/persistence evidence predates some current source/support changes; consult
its recorded revision and regenerate before claiming current full-game acceptance.

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

### Application preparation publication

`prepare-game` builds the four application scopes in an isolated project and
publishes their owned closures together only after all scopes succeed. The
ordinary generated-file collision/sharing guards apply to this grouped
publication, preserving consumer settings, authored files and unrelated levels.
A failed scope leaves consumer generated content unchanged. Destination cache
import occurs after publication; its failure is reported separately.

Private raw mip PNGs have a `.gdignore` beneath `build_inputs/texture-mips/`;
the conversion builder reads them directly with `Image.load_from_file`. Native
Animation inputs and published resources retain ordinary resource visibility.
Editor import gets one minute per2,000 declared outputs, bounded to one through
ten minutes; the native resource builder has its separate360-second deadline.
