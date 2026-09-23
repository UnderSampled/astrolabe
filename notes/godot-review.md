# Godot adapter contract review

This review covers the first-model Godot projection and application composition in
`src/godot/mod.rs` and `src/app.rs`. The synthetic probe constructs useful model,
material, and texture values directly; it does not import OpenSpace or depend on
private game fixtures. It complements, rather than replaces, real-model geometry
assertions and visual comparison against the original engine or Raymap.

## Findings and corrections

1. **Clockwise target winding must be derived together with corner attributes.**
   The first rooted Hype triangle has positive cross-product/face-normal agreement
   (about 76.755), whereas [Godot ArrayMesh uses clockwise front faces](https://docs.godotengine.org/en/stable/classes/class_arraymesh.html).
   The conversion `C(x,y,z) = (x,z,-y)` has determinant +1; it does not reverse
   winding. The original adapter emitted corners 0,1,2. The correction emits
   0,2,1, moving independent UV indices and normals with their corners. Leaving
   backface culling to conceal this would fail on more complex models.
2. **Use the full affine basis exactly once.** Hype's full 3x3 basis already
   contains scale and rotation. The separate unscaled rotation field is not an
   extra transform to multiply into it. The adapter's conjugation `C M C^-1`
   and composition of the local parent chain are correct on inspection. The
   synthetic probe uses nonuniform scale, rotation, shear, and translation to
   distinguish this rule from a transpose, axis swap alone, or double rotation.
   Source evidence is recorded in [the rooted level research](./level-rust-research.md).
3. **Useful edits must affect emitted material values.** The first adapter
   hardcoded white albedo, silently discarding `Material.color` in the target.
   It now maps that value into `StandardMaterial3D.albedo_color`. The current
   unlit material remains an explicit first-model approximation: successful
   resource import does not establish engine lighting, blending, scrolling,
   texture-addressing, or scene behavior.
4. **Resource creation order is an engine contract.** A preview scene must not
   reference model scenes before the builder creates them. Moving preview
   creation after the builder was insufficient: the real initial `--import`
   still failed because `project.godot` named the nonexistent preview as its
   main scene. Keep the initial staging main scene valid or add that setting
   only once the preview exists. The adapter now supplies a valid placeholder
   preview and creates `generated/hype` before the initial import, including
   roots that produce diagnostics but no media.
5. **Save reusable mesh resources with an external identity.** Saving an
   `ArrayMesh` without changing its resource path can cause a PackedScene to
   embed another mesh instead of referencing the separately saved `.mesh.tres`.
   Although [ResourceSaver documents FLAG_CHANGE_PATH as changing the saved resource path](https://docs.godotengine.org/en/stable/classes/class_resourcesaver.html),
   a minimal runtime experiment with Godot 4.7.2 returned OK while leaving that
   path empty. Explicit `take_over_path` followed by loading the saved mesh
   fixes the external reference. The probe loads the scene and standalone mesh
   through Godot and compares their resource identity, not just filenames.
   It also caught random node `unique_id` values in two clean `.tscn` exports.
   The adapter removes provisional editor node IDs and normalizes file-local
   resource IDs; every generated resource participates in the byte comparison.
6. **Generated manifests require containment and conflict preflight.** The
   original publication code trusted old manifest paths and checked modified
   stale files only after writing new files. Corrected code validates the
   generated namespace, relative components, and symlink ancestors, then
   preflights stale conflicts before publishing. The probe verifies an edited
   stale resource and an outside-namespace entry cannot partially publish a
   changed mesh or remove the authored wrapper. Publication is atomic per file;
   recovery from disk failure midway through the whole publication is not yet
   a transaction guarantee.
7. **Keep whole-asset and member support explicit at the boundary.** Rete rejects
   direct member loads, while the original Godot filename mapper silently
   discarded `#member`. Until a member projection exists, Godot must likewise
   reject direct member saves. Those rejections are now implemented, as is
   standalone material root export with its texture dependency.
8. **Embedded asset identity must agree with its resolver key.** Model and
   material values carry an identity while Rete filenames and lookup keys also
   identify them. Previously a moved Rete file or incorrect resolver could
   silently emit resources under an unexpected path. The shared Content
   contract now checks agreement; Rete, the in-memory collection, application
   composition, and Godot material resolution use it. A fast synthetic
   regression supplies incorrectly named model and material values and verifies
   failure before the engine is launched or output is published.

## Synthetic engine probe

`tests/godot_contract.rs` adds the ignored `synthetic_godot_contract` test. It
uses `ASTROLABE_GODOT` or `godot`, with no game directory. It checks:

- a constructed quad, two surfaces sharing a material and texture, and actual
  headless `ArrayMesh`/`PackedScene` ResourceSaver and importer behavior;
- transformed vertex positions, clockwise fronts, independent per-corner UV
  mapping, texture row orientation, and nontrivial composed transforms;
- one externally referenced mesh and one shared material object;
- byte-for-byte deterministic generated files from two fresh staging projects,
  including resource paths and identifiers after UID normalization;
- geometry width and material color edits in reloaded native resources;
- unchanged canonical source values, authored project settings, and wrapper;
- preflight refusal of edited stale resources and invalid manifest ownership.

The whole standalone test has an eight-minute deadline, including its engine
subprocesses; each engine call has a shorter timeout. The repository suite
runner imposes its own shared deadline when this opt-in test is included.

Validation: `cargo test --test godot_contract -- --ignored --nocapture` passed
in **22.86 seconds** with Godot **4.7.2.stable.arch_linux.ed1daf0bf**, after
exposing and correcting the staging, external-resource, and node-ID failures.
The nonignored identity regression passed in under 0.01 seconds. The synthetic
fixture intentionally has no radiosity samples; first-model source radiosity
and its eventual Godot lighting projection require separate evidence.

No claim of visual or original-engine parity follows from this resource
contract probe. The current projection is provisional and the test protects
its useful behavior without freezing a durable remaster asset interface.

## Real-model persistence and reconstruction route

`tests/real_routes.rs` now exercises the real first rooted astrolabe model with
`ASTROLABE_GAME_DIR` (default `disc`) and `ASTROLABE_GODOT` (default `godot`). It
imports ten content assets (geometry, instance, four materials, four textures),
saves each through the Rete adapter,
drops the source import session, and opens a fresh Store. Every generated Godot
file hash agrees between direct import and this reopened Rete route. Repeated
material loads share the cached useful object, and the original model values
survive persistence unchanged.

The test then edits a source X coordinate by 1.25, saves it, closes/reopens the
Store again, and checks the edited vertex in an actual native Godot ArrayMesh.
Its source radiosity metadata retains negative and above-one components; the
fourth source component does not become rendered opacity. Unchanged textures
and materials keep their hashes. OpenSpace export receives only the reopened
Store, and independent byte assertions follow the generated PO/visual-set/LOD
chain, exact position array, 75 RTB pointer sites, full 8,208-byte PTX table, and
RTT pointer values. Original TextureInfo names survive canonical GF resolution.
This is controlled fragment reconstruction, not a complete loadable level.

The real route passed in **23.62 seconds** on Godot 4.7.2. Its standalone worker
has an eight-minute deadline, and `tools/test.py --local` includes it in the
shared suite deadline. A fast non-game regression also verifies restored source
diagnostics appear in both CLI output and the written reconstruction report.

## Architecture risks before expanding the converter

- **Source coverage must survive Rete.** Review found that import diagnostics
  were discarded when only useful assets and package roots were saved. That is
  now corrected: shared `Diagnostic` values carry a code, optional asset,
  severity, and message; Package persists them with a backward-compatible
  default. `Imported.source_summary` holds transient VM/address observations
  separately. The application exposes restored source coverage on both export
  commands. The first-model importer emits an explicit partial-import warning
  plus retained observations. Feature-specific codes at the producing codecs,
  and typed Godot/reconstruction diagnostics, remain useful follow-up work;
  this change does not pretend every existing prose diagnostic is now a
  machine-readable feature matrix.
- **Separate reusable model content from scene-instance values before
  milestone 6.** Current Model combines geometry with a root-to-instance
  transform chain and the selected IPO/LOD's radiosity. This is a workable
  first-instance slice. Extending it directly to two instances of one geometry
  will either duplicate the geometry or conflate independently changing
  transforms/lighting. The smallest next probe is two instances sharing one
  geometry and materials but having distinct transforms and radiosity, with an
  edit to only one instance. Use that evidence to place scene-instance data;
  no scene architecture redesign was made during this review.
- **The shared codec contract should grow from demonstrated differences.**
  The initially missing typed binary interface now exists as OpenSpace's
  `FormatCodec` and a format catalogue. GF/CNT/APM/BNM/text adapters pair useful
  read/write values through it; binrw still owns declarations in format
  modules. This is the appropriate boundary. The remaining issue is consistent
  limits, profile/subset capabilities, and diagnostic context as more formats
  are implemented independently. Add those with the next concrete format
  variation, without forcing a multi-file level conversion into a single
  buffer codec or making Rete/Godot inherit OpenSpace layout parameters.
- **Keep current generated paths provisional.** Internal model and MessagePack
  changes are inexpensive while all game content can be regenerated. Authored
  Godot dependencies become expensive once they rely on exact paths, node
  identities, or runtime interfaces. The current wrapper test proves safe
  regeneration under an unchanged topology; it does not prove that names based
  on rooted child ordinals survive scene insertion or model-sharing changes.
  Settle instance ownership and exercise topology changes before committing a
  durable downstream asset interface.


## Shared geometry and instance state

`tests/model_instances.rs` complements the original contract probe with four instances of one reusable model. It persists independent transforms and lighting, edits one instance and then shared geometry, and checks source fragments without disc access. The Godot probe inspects two plain instances sharing one native mesh and two differently lit variants retaining their own colors and transforms. Instance scenes and lighting variants derive paths from instance identity; plain meshes derive paths from model identity. These paths remain provisional.
