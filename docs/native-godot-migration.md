# Native scene migration

Implementation decisions, 2026-09-13. This work projects the existing canonical
scene/character assets; it does not introduce a second model or change source
reconstruction. The supported local fixture remains English PC Hype/Astrolabe.

## Ownership checkpoint

`godot/` now contains the committed source-free starter and the single maintained
`addons/astrolabe/` support source. `init-godot` adopts settings/bootstrap once;
`update-godot-support` preflights a separate installed-file receipt and refuses
edited support. The source checkout uses Git to update support. Regeneration
validates the installed interface and actual support code without replacing it.
Support interface 1 targets Godot 4.7.2; script `.uid` files stay with their owners.

Generated manifest schema 2 tracks selected-level dependency closures. Shared
assets can have multiple owners. Regenerating A preserves B and its dependencies;
changing a shared resource still used by B reports a conflict. Only unchanged
orphaned generated files are deleted. `preview.tscn`, its script and diagnostic
`validation.json` are level-local. Old preview layouts require fresh generation.

Checks at this checkpoint: Rust support/update/ownership unit tests; actual
Godot 4.7.2 import and execution of the dehydrated bootstrap. Full scene migration
and complete local-suite acceptance are pending below.

## Native character projection being implemented

Raymap `PersoBehaviour.UpdateFrameMontreal` resolves per-frame parent columns and
local transforms; the engine evidence and semantic corrections are in
[scene-animation-research.md](scene-animation-research.md). Preserve those exact
values in Model/Rete. At Godot conversion, compose the source affine transforms
into character-relative transforms on stable logical channel nodes. Pre-instance
part alternatives and animate native transform, visibility and opacity properties.
Use discrete keys to retain full affine/shear values and the established sampled
semantics. Do not infer smooth interpolation, root motion, morphs or gameplay
visibility overrides. AnimationPlayer/AnimationLibrary own target playback and
seeking. Runtime support may choose native clips and implement cue/engine behavior;
it must not resolve source matrices, object tables or per-frame hierarchies.

Animation libraries are specialized to the owning family/configuration, while
geometry and materials retain their independent shared identity. Perso scenes
persist an initial pose and are instanced by the meaningful level hierarchy.
A reset configuration is converted ahead of time as another native library/part
selection. Bounds become saved debug geometry, not physical collision. Mechanics
cards become typed properties for inspection and future behavior, not a claimed
dynamics simulation. Sound-program consumers are audited with this migration.
