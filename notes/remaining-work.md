# Remaining work after the visual pass

Current priority (2026-09-20): follow the [live loader/comparison plan](./realtime-comparison-plan.md)
before resuming checkpointed gameplay work. This document retains the broader
remaining-work inventory and historical visual-pass evidence.

The native Godot scene/character migration passed on 2026-09-13. The saved
fixture contains supported parts and initial poses, native clips can be scrubbed
in the actual editor, and authored content survives Rete edits/regeneration.
The latest complete local suite, including the first AI slice and actual Scene dock checks, passed 86 tests
in 497.572 seconds. See
[native migration evidence](./native-godot-migration.md). Milestones 6–7 remain
partial because the source reconstruction and semantic gaps below are still open.

The reported helmet-eye, ladder-wall and star-ceiling transparency defects are
fixed. The opening scene is compared with the original Hype MCP screenshot at
the measured camera pose and 69.8838-degree vertical FoV. Bilinear filtering
adds a 4% reduction in opening-scene RMSE after the transparency correction.
The star-ceiling before/after check is Godot-only. See
[pixel evidence](../docs/openspace/material-transparency.md), [fog evidence](../docs/openspace/environment-fog.md)
and [scene reconstruction](../docs/openspace/scene-hierarchy.md). Current suite results and
coverage counts are in [implementation progress](./implementation-progress.md).

## Next pass and completed migration

The [committed/rehydrated project structure](../docs/astrolabe/godot-project.md) and native
scene/character slice are implemented. The first
[typed AI-to-GDScript slice](./ai-gdscript-slice.md) runs the two torch sound
controllers. Expand behavior support and the remaining reconstruction and
semantics below. The maintained support package, explicit authored/generated
ownership and source-free bootstrap with synthetic examples are established.
Extend these interfaces as new slices need them; broad OpenSpace reconstruction
is not a prerequisite.

1. **Completed for the supported fixture: saved native level.** Keep the
   meaningful OpenSpace tree as the level's root scene, with reusable prop/perso
   subscenes and shared geometry/resources. Convert initial configuration and
   pose into persisted part nodes, transforms, and visibility. Opening the level
   in the editor must show supported clutter and persos without `_ready()` or
   preview construction; explain any unsupported or initially hidden content.
   Verify node ownership, source identity, hierarchy, and independent instance
   placement. Make the level root the normal level-port result and retain the
   model selection as an explicit development mode. The camera/inspection
   wrapper should instance that root without supplying its contents.
2. **Completed for the supported fixture: native resources and animation.** Convert the
   currently exported animation JSON into Animation/AnimationLibrary resources
   and AnimationPlayer nodes. Resolve transform pools, channel parenting, and
   object tables during conversion; assess derived transform tracks and
   pre-instanced part alternatives with discrete property/visibility tracks.
   Preserve timing, state speed, loops and cues. Test editor scrubbing, backward
   seeking, initial/reset poses, and actual playback through the same resources.
   Audit family/configuration, bounds, mechanics, and sound JSON consumers too;
   use native nodes/resources and converted behavior, with typed custom
   Resources only for useful target semantics lacking a built-in representation.
   Wrapping interpreted source-shaped tables in `.tres` is insufficient. Remove
   superseded production players as replacements land; retain useful semantic
   fixtures and optional diagnostic/report JSON. See the
   [native Godot contract](../docs/astrolabe/rust-architecture.md#godot-adapter-native-project-conversion).
3. **Finish the supported scene/character reconstruction path.** Complete
   useful Perso, Perso3dData and StandardGame fields against this Hype build
   (76, 444 and 56 bytes respectively). Preserve unpromoted fields explicitly;
   derive runtime caches rather than replaying pointers. Compose the existing
   SO, sector/environment, family/configuration and geometry writers through
   explicit layouts. The SO writer already checks all 150 records and all 108
   embedded character-matrix aliases, but the CLI still emits leaf fragments
   instead of an assembled scene.
4. **Close remaining animation semantics.** Investigate related/morph data
   and the actual Hype interpolation path. Raymap's Montreal sampled player
   alone is not evidence that generic interpolation is correct. Extend
   source reconstruction and native Godot animation export together, then
   compare bounded idle, movement and action sequences with MCP observations.
   Runtime channel visibility overrides remain a provisional comparison aid;
   automatic gameplay-driven visibility, lip sync and root motion are incomplete.
5. **Finish scene audio coverage.** Resolve remaining ambient emitters,
   propagation/routing and source context beyond the tested animation sound
   programs. Use native audio resources and scene players with converted event
   behavior; add loop/stop and actual-mixer checks for representative cases.
   Keep AI/dialogue-triggered behavior completion with milestone 8. SDA was
   verified as static globals, not an ambient-sound table.

## Further scene and visual work

- Compare the star ceiling from a matching original-engine view. Keep the
  current pixel tool and explicit camera settings; separate idle-phase/HUD
  differences from material or geometry defects.
- Preserve/project original mip chains and verify source mip selection and
  LOD behavior. Bilinear sampling is implemented; exact mip filtering is not.
- Improve dynamic lighting and signed/prelit behavior, per-sector fog routing,
  full render-context flags, chrome, texture scrolling and node opacity.
  Uniform source fog is projected; mixed environments are diagnosed rather
  than applying one incorrect global fog to every sector.
- Expand advanced sprites, remaining LODs, sector activation/visibility and
  spatial structures. Keep picking bounds separate from physical collisions.
- Broaden deliberate-edit, collision and rendering checks beyond the current
  Astrolabe fixture, preserving level-local bindings for shared Fix assets.

## Later gates

- Demonstrate original-engine acceptance of generated files in a test copy.
  Current MCP evidence observes original game content; it does not establish
  acceptance of reconstructed packages. Complete whole-package relocation
  and pre-compression parity with milestone 12; exact historical LZO encoding
  remains a separate late gate.
- Finish unsupported audio writing/mixing, localization/sidecars, AI and
  interactions in their ordered milestones. Milestone 8 compiles the typed AI
  AST into GDScript and adds the needed Godot engine-side functions to the
  maintained support package; an AST interpreter is excluded. Convert and play the Indeo AVI
  cutscenes late in the port.
- Broaden support/consumer contracts as slices become usable. Test authored
  variations, support updates and regeneration in both project states; full-game
  coverage remains a later claim, not a gate for needed assets or supported
  consumers. Current Rete schema 10 and Godot paths remain provisional; regenerate
  older local Rete packages from source and document target-interface changes.

## Completed migration goal

The completed request was (its historical whole-suite timing limit was superseded
on 2026-09-20 by the [focused testing policy](./realtime-comparison-plan.md#test-selection-and-timing-policy)):

> Implement the native Godot scene/character migration for the currently
> supported Astrolabe fixture, following AGENTS.md, plan.md and
> docs/astrolabe/godot-project.md. Establish the committed/rehydrated project structure
> and maintain any needed support assets. Persist supported clutter, persos,
> parts and initial poses in the level scene; replace the corresponding
> interpreted asset paths with native resources and animations that can be
> inspected, scrubbed and played in Godot. Validate direct and reopened-Rete
> conversion, preserve existing reconstruction coverage, and demonstrate an
> authored Godot variation surviving regeneration. Use the available source-game
> comparison harness where relevant, keep the complete suite under ten minutes,
> and record any remaining semantic gaps. AI conversion remains the later
> milestone; do not introduce an interpreter as an interim target.

That migration is now implemented in the committed `godot/` project. Current
coverage and acceptance evidence are in [implementation progress](./implementation-progress.md)
and [the migration report](./native-godot-migration.md). The full port plan and
remaining semantic/source reconstruction work still apply.

## Resume without repeating setup

Current local handoff artifacts:

- Rehydrated project: `godot/project.godot`; saved level:
  `godot/generated/hype/levels/astrolabe/level.tscn`.
- Compiled torch behavior: `godot/generated/hype/levels/astrolabe/ai/models/824.gd`;
  [AI source/runtime evidence](./ai-gdscript-slice.md), Rete schema 10, support interface 2.
- Authored inherited variation: `godot/game/characters/luminous_hype.tscn`.
- Native opening capture: `output/native-opening.png`; comparison report:
  `output/native-opening-comparison/report.json`. The previous accepted preview
  remains in `output/scene-graph-compare/` as historical comparison evidence.
- Camera values: `output/fog-camera-evidence.json`.
- Complete validation: **86 tests in 497.572 seconds**,
  `TestResults/20260913T185841.641467Z-25665.log.json`, including the actual
  editor, direct/Rete routes, regeneration, compiled AI and cleanup. Strict Clippy and
  formatting pass. The private source-game session is stopped.

No reference-submodule update was made during this pass. Generated projects,
captures and game-derived evidence stay local and are excluded from the
implementation commit.

Use the existing external Hype MCP server and supplied English save. Keep
reference submodules, source game files and saves unchanged. Read
[the runtime guide](../reference/README.md#runtime-validation) before source
comparisons; no Unity Editor or new controller setup is needed.

```sh
cargo build --release --locked
python3 tools/test.py --local --offline
target/release/astrolabe port disc godot --level astrolabe --locale en
```

The complete suite has one 570-second build/test deadline plus bounded cleanup.
Do not treat timeouts, interrupted compilation or leaked processes as a pass.
Use the existing camera JSON and `tools/compare_images.py` recipe in the pixel
report for repeatable reviews. Generated projects, captures and game-derived
evidence remain under ignored local output directories.
