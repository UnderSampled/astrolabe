# Shared diffuse-coefficient dependency, 2026-10-04

This is a static source/design survey, not an implemented rendering slice. No
tests, builds, conversion, Godot or original-engine execution were performed.
The executable is the documented patched `MaiDFXvr_bleu.exe`, SHA256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
Raymap's `VisualMaterial.cs` was consulted as the practical reference; the
matching engine disassembly supplies the Hype-specific evidence below.

## Source behavior and retained boundaries

Proc237's dispatcher case `00543386` resolves a category-36 GameMaterial through
`0049df30`. An absent GameMaterial or its visual component causes the remaining
operand to be skipped through `00522450`, without evaluating it. Otherwise one
packed RGBA integer is evaluated. `0052efc0` clamps each little-endian byte to
0..100 and scales by 0.01; `0048f1b0` copies all four floats into
VisualMaterial+0x14. See the corresponding AI survey in
[the static AI note](ai-static-port-2026-10-04.md#explicit-deferrals).

The mutation belongs to the resolved visual object. GameMaterial+4 points to
that object; retaining only GameMaterial asset identity cannot represent two
GameMaterials pointing to the same visual object. The current importer caches
GameMaterial addresses, then copies visual fields inline into each `Material`.
`Material.visual: Option<VisualMaterial>` has no identity. The fragment writer
allocates a fresh visual record for each material. The model therefore cannot
currently preserve this alias relationship, regardless of native shader choice.

The current static projection is explicitly approximate:
`clamp(2*(ambient.rgb + diffuse.rgb*RLI.rgb/256), 0, 1)`. It is baked into mesh
vertex colors by `godot/resources.rs`; only metadata retains exact signed RLI.
`scripts/build.gd::lighting_material` duplicates the material for unshaded or
per-vertex native lighting. `godot/sprite.rs` duplicates it for native billboard
properties. Actor opacity makes further `StandardMaterial3D` copies in
`character/rendering.gd`. Updating one exported albedo or base material would
leave these variants disconnected and would change the coefficient's meaning.

There is a concrete fourth-component dependency. `004948c0` loads ambient.w at
`00494922` and diffuse.w at `0049495e`, scales both with the same coefficient
constant as RGB, and calculates the fourth lighting result at
context+0x30d6c. `0049d150` combines the fourth per-vertex contribution with this
result, stores it into output vertex+0x10 at `0049d1bc`, and clamps it at
`0049d213..0049d223`. `00494a80..00494aa9` override the fourth diffuse/result
components for flags 0x04000000 and 0x8; `00494aad..00494ad3` may multiply the
result by TextureInfo+0xb8. This is insufficient evidence for an unconditional
`diffuse.w * albedo.a` formula. Dynamic lighting and effective renderer-context
flags remain incompletely mapped, as already recorded in
[the level research](level-rust-research.md#per-instance-radiosity).

## Proposed native design and ownership

First promote independently shared visual content into a canonical asset:
`Material.visual: Option<AssetRef<VisualMaterial>>`, with `VisualMaterial.path`
and minimal source identity. Import should deduplicate resolved visual pointers;
Rete should persist that shared content once; source reconstruction should
allocate it once and point every referring GameMaterial at it. TextureInfo and
its sampling metadata remain owned by that visual asset. Null visuals retain
their existing meaning. This is a separate, useful source-sharing prerequisite,
not a second mutable coefficient model copied alongside inline visual values.

The native mutable coefficient state should be one focused Resource per visual
asset in the active level. A saved ShaderMaterial variant should reference that
state, hold immutable texture/mode parameters, and update shader uniforms when
the state emits `changed`. Separate variants are needed for native prelit and
dynamic lighting, alpha scissor/blend/additive modes, and billboarding. The
converted files must contain complete meshes/materials and initial uniforms so
opening the scene in the editor presents the existing initial content.

Per-scene isolation must duplicate each coefficient Resource exactly once and
bind every material variant to the scene's copy. Mesh surface materials,
material overrides, sprite copies, hidden/precreated configuration payloads and
spawned actors must all adopt the same copy by canonical visual identity. A
scene-owned binding service can retain the identity-to-state map and material
variants. Actor opacity copies keep the coefficient reference shared while
owning their local opacity and transparency mode; they must be explicitly
rebound when produced. Deep-duplicating coefficient Resources independently
for each mesh, billboard or actor would break the source mutation semantics.
Textures and shader programs remain shared immutable content.

Prelit vertices need the unmodified signed four-component RLI in a native float
vertex attribute. Updating uniforms can then recalculate lighting without CPU
mesh reconstruction or interpreting source tables. Native material-specific
render-mode shaders must preserve the existing keyed PNG, alpha-test threshold,
additive blend, bilinear filtering, culling and billboard scale behavior. Actor
opacity and channel transparency remain separate inputs; they cannot overwrite
the source coefficient state. Exact float-attribute packing and billboard
vertex transforms need a focused Godot API/source review before coding.

## Bounded batches and blockers

1. Shared visual-asset promotion is a coherent static batch: model geometry;
   OpenSpace material import/fragment export; content/resolve/registry traversal;
   Rete schema; existing material/geometry fixture constructors and checks. It
   should retain current native rendering behavior while correcting identity.
2. Trace `004948c0` callers, `00492c50`, `0049d150`, and the sprite/dynamic-light
   consumers to name the effective context and fourth-component inputs. Review
   the pinned Godot implementation for float attributes, shader billboard
   transforms and material-copy lifecycle. This is source research, not a probe.
3. Once those inputs are established, implement the shader/resource slice in
   `godot/material.rs`, `resources.rs`, `sprite.rs`, `scripts/build.gd`, new native
   rendering Resources/shaders, `character/rendering.gd`, and scene/spawn shared
   binding. Parent-owned support/registry/version integration follows. Only
   then should the AI owner add a typed Proc237 contract that resolves the
   service before evaluating its packed operand.

A single shader-only Proc237 batch is not self-contained with the present model
and evidence. RGB-only prelighting, ignoring dynamic variants, silently dropping
the fourth coefficient, or changing only albedo would not complete this API.
