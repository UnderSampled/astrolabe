# Godot project structure and ownership

Design direction agreed on 2026-09-13 and implemented for the supported native
scene/character slice. See [migration evidence](native-godot-migration.md) for
checks and remaining gaps; this is not a claim of complete game support. It complements the
[native conversion contract](rust-architecture.md#godot-adapter-native-project-conversion).

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
the disc and comparison reports. The implemented baseline is:

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
generated level entry points and current supported slice. Check availability
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
