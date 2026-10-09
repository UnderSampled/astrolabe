# Shield opacity during unfreezing, 2026-10-09

This is a source-first correction based on clean main `12ad58ce49d60b10cdcbb80a29c9118a5e9feffa`
(support 84, Rete 62). The worker used `output/worktrees/shield-opacity-20261009`
and did not launch Godot, the original game, native import, a build or a runtime
test. Kip owns integration and Luna owns runtime checks. `git diff --check`
passed; the prepared native regression has not run.

## Concrete defect and correction

Both maintained diffuse shader templates write `ALPHA_SCISSOR_THRESHOLD`.
The actor renderer previously tried to fade them by adding `blend_mix` and
setting the threshold to zero. That does not disable cutout rendering.
Godot's [Compatibility surface classification](https://github.com/godotengine/godot/blob/4.7/drivers/gles3/rasterizer_scene_gles3.cpp#L211-L247)
excludes alpha-clipped shaders from ordinary alpha blending. Its
[fragment path](https://github.com/godotengine/godot/blob/4.7/drivers/gles3/shaders/scene.glsl#L2135-L2148)
discards alpha below the threshold and then makes surviving fragments fully
opaque. At threshold zero, even zero-alpha texture samples survive. This is a
concrete renderer defect consistent with the reported shield filling in, but
the exact observed frame still needs the canonical game capture below.

`character/rendering.gd` now removes that built-in assignment from temporary
fade shaders, keeping mix/additive selection and the independent opacity
uniform. Restoring ordinary display restores the saved source shader and
cutout threshold. `rendering/visual_material_state.gd` also removes the cutout
assignment when the source material is genuinely alpha-blended or additive.

Diffuse conversion can happen after actor fading has already duplicated a
StandardMaterial. That copy temporarily has alpha transparency even when its
base material is cutout. The copy now carries actor-local
`source_opacity_base_transparency` metadata so diffuse conversion preserves its
ordinary pipeline for later restoration. This is runtime material state, not
new Rete/source metadata or a generated asset format.

The prepared `visual_material_state_smoke.gd` regression covers conversion
during a fade, repeated diffuse updates, zero-opacity fading, restoration at
the source 254 boundary, and alpha/additive variants for both native lighting
modes. It asserts the shader output controlling renderer classification as
well as retained opacity and cutout threshold. It is structural native coverage,
not a rendered pixel comparison.

No bank, physics, animation, configuration, shared character interface or
sector-query code changed. Existing actor-local copies still cover saved bank
payloads; the correction neither activates hidden banks nor adds visible copies.

## Asset and sequence evidence

The current generated material `fix:/materials/06-01/18475` is a keyed cutout
with texture `CA/ca02_2a_nz.gf` and texture flags 2. Visual inspection identifies
its blue shield artwork. Its 128 by 128 keyed PNG has 2,352 zero-alpha and
14,032 opaque texels. Prior source import evidence joins it to submesh 0 of
geometry 18464, instance `fix:/objects/18460`.

The local `output/manoir-position-field-rete` cache (schema 18, historical,
not a fresh schema-62 import) places that instance in configuration 30 slot 2
in both normal and reset tables. Actions 323–327 reference clips
14165/14299/14368/14502/14531. Their object-2/channel-0 samples all have opacity
255 and visibility 1. Cache and texture hashes are recorded in the ignored
worker evidence `output/shield-opacity-static-20261009/receipt.json`.

Current compiled AI gives a separate actor-opacity path: Statue model 294
Normal 2 selects Hype Normal 69, which requests opacity 0 and transparent
display. Statue Normal 4 runs action 7, selects Hype Normal 67, then runs action
8. Hype Normal 67 requests opacity 255 before action 323/324. Statue then
requests opacity 0 before action 9. Hype Normal 70 later runs 325/326/327.
No animation sample edit is justified by the cached sample values.

The clean reconstruction at
`/home/stephen/Projects/hype-decomp/src/ai/ai_receiver_dispatch.c:179`
separates transparent/default display flags from clamped subtree opacity.
Raw source `reference/ghidra/functions/00499ed0_FUN_00499ed0.c` in that project
shows the material/context gate and opacity-below-254 blend branch. This
renderer function is still `untouched` in the decomp coverage ledger. Exact
effective context flags, including its bit-8 override and source alpha-test
threshold during fading, remain broader source-parity questions. If those
are needed for further correction, Kip should assign entry `00499ed0` and
its callers to the decomp project rather than expand this Godot fix into an
engine reconstruction.

## Integration and focused Luna request

Adopt the two maintained support scripts into the real `godot/` game through
Kip's composition. No native scene, AnimationLibrary, mesh, texture, Rete or
OpenSpace asset needs regeneration for this patch. Preserve the existing
generated Manor and bank physics. Kip should compose any support version bump
once with the other workers. A downstream installed support package needs an
explicit support update using the composed converter; an old binary embeds
old support bytes. This worker leaves that build/publication to Kip.

In Luna's single runtime slot, first parse/run the prepared existing regression
`source_visual_material::native_shader_variants_share_source_diffuse_updates`
(its Godot action deadline is 30 seconds). Then request a short rendered capture
window around Statue Normal 4 actions 7/8/9 through the existing canonical New
Game route, with all imported AI active. Record Hype/Statue behavior/action,
actor opacity/display mode, selected normal/reset slot-2 payload, shield
material URI, shader cutout assignment presence, threshold and opacity uniform.
Capture before Hype Normal 67, immediately after opacity 255 is restored, and
after Statue opacity 0. The shield's keyed holes must remain clear; a faded
actor must disappear instead of becoming an opaque duplicate. Check selected
bank participation remains unchanged. Do not alter source state to manufacture
this transition. These observations will distinguish this shader defect from
an additional configuration or overlapping-actor problem.

The task's source-thread coordination tool did not deliver: its initial message
was rejected because destination authorization was unclear; after the explicit
instruction to message Kip, the source ID returned `thread not found` with both
default and reported host routing. The precise experiment request is preserved
here and in the normal task commentary/final handoff for the parent to receive.
