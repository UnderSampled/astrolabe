# Stopping point and remaining work

This pass is closed at the user's request on 2026-09-12. Milestones 6 and 7
have useful, tested coverage and remain partially complete. The current
visual result is accepted for this pass; further fidelity work is follow-up,
not a reason to keep this pass running.

The reported helmet-eye, ladder-wall and star-ceiling transparency defects are
fixed. The opening scene is compared with the original Hype MCP screenshot at
the measured camera pose and 69.8838-degree vertical FoV. Bilinear filtering
adds a 4% reduction in opening-scene RMSE after the transparency correction.
The star-ceiling before/after check is Godot-only. See
[pixel evidence](material-transparency.md), [fog evidence](environment-fog.md)
and [scene reconstruction](scene-hierarchy.md). Current suite results and
coverage counts are in [implementation progress](implementation-progress.md).

## Suggested next pass

1. **Finish the supported scene/character reconstruction path.** Complete
   useful Perso, Perso3dData and StandardGame fields against this Hype build
   (76, 444 and 56 bytes respectively). Preserve unpromoted fields explicitly;
   derive runtime caches rather than replaying pointers. Compose the existing
   SO, sector/environment, family/configuration and geometry writers through
   explicit layouts. The SO writer already checks all 150 records and all 108
   embedded character-matrix aliases, but the CLI still emits leaf fragments
   instead of an assembled scene.
2. **Close remaining animation semantics.** Investigate related/morph data
   and the actual Hype interpolation path. Raymap's Montreal sampled player
   alone is not evidence that generic interpolation is correct. Extend
   source reconstruction and Godot playback together, then compare bounded
   idle, movement and action sequences with MCP observations. Runtime channel
   visibility overrides remain a provisional comparison aid; automatic
   gameplay-driven visibility, lip sync and root motion are incomplete.
3. **Finish scene audio coverage.** Resolve remaining ambient emitters,
   propagation/routing and source context beyond the tested animation sound
   programs. Add loop/stop and actual-mixer checks for representative cases.
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
  interactions in their ordered milestones. Convert and play the Indeo AVI
  cutscenes late in the port.
- Establish durable downstream game/remaster asset contracts after the agreed
  adapter-coverage gate. Current Rete schema 9 and Godot runtime/paths remain
  provisional; regenerate older local Rete packages from source.

## Resume without repeating setup

Final local handoff artifacts:

- Godot project: `output/scene-graph-compare/project.godot` (generated with the
  final release build; port completed in 20.14 seconds).
- Matched opening capture: `output/scene-graph-final.png`; pixel report and
  regional comparison images: `output/scene-graph-final-pixels/`. The capture
  hash and every measured region match the preceding filtered preview exactly.
- Camera values: `output/fog-camera-evidence.json`; the pixel report identifies
  and hashes both the external original-game screenshot and final Godot image.
- Complete validation: **81 tests in 216.322 seconds**,
  `TestResults/scene-graph-full-retry-20260913.log.json`. Release build, strict
  Clippy, formatting and whitespace checks also passed. Test/render processes
  completed cleanly; the private original-game session is stopped.

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
target/release/astrolabe port disc output/next-scene --content scene --level astrolabe
```

The complete suite has one 570-second build/test deadline plus bounded cleanup.
Do not treat timeouts, interrupted compilation or leaked processes as a pass.
Use the existing camera JSON and `tools/compare_images.py` recipe in the pixel
report for repeatable reviews. Generated projects, captures and game-derived
evidence remain under ignored local output directories.
