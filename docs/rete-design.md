# Rete design for the Rust rewrite

Design decisions from the project discussion, updated 2026-09-12. No Rust implementation, fixed schema, or runtime test harness exists yet. This document defines the asset model; [rust-architecture.md](rust-architecture.md) defines module boundaries and the eventual deterministic Godot contract, and [../plan.md](../plan.md) defines the implementation order. These documents supersede the historical C# Rete JSON specification and semantic pool framework.

## Purpose and scope

**Make a tool to repeatably port this OpenSpace game to Godot.** Godot is the primary target, supporting a modern fan-remaster and independently authored Godot work that depends on locally converted game content.

The useful meanings are Astrolabe's intermediate model: textures, geometry, animation, behavior, and their meaningful relationships. **Rete is the serialized form of that same model.** Level and Fix remain distinct roots linked by file references within it. OpenSpace content codecs read and write those useful types; inspection, Rete persistence, and Godot export operate on the same values. Conversions can stay in memory without going through Rete on disk. A Godot-to-Rete/OpenSpace editing round trip may be useful later but is not required for the port.

OpenSpace, Rete, and Godot have separate modular I/O adapters around this model. The Rete adapter owns MessagePack persistence and asset-file loading/saving; generic serialization traits may live on the shared types. Application code connects adapters through the model and shared resolution contracts. The separation concerns target operations and remains the same whether implemented as modules or separate crates. See [the architectural overview](rust-architecture.md#three-modular-io-targets-one-useful-model).

Declarative binrw codecs document the mapping between source encoding and useful model fields. Temporary binary headers or records are implementation details where needed; they do not constitute a required second model or a public loading stage. A format is understood to the extent that its content has meaningful representation and behavior, with reconstruction evidence alongside it.

The OpenSpace target is understanding, reconstructing, and modifying the original Hype game. Developing a new OpenSpace game is outside the scope. Unedited full-package bitwise parity remains the final reconstruction goal, approached through smaller meaningful tests. Modified assets must produce valid game data; changes naturally alter bytes and may require new layout decisions.

## Files represent content

Choose file boundaries around useful assets: 3D models, materials, animations, AI scripts, scene graphs, perso families, and related content. Organize each asset around meaningful fields and relationships, as one would expect to inspect in a game engine editor. A material's texture link should expose a value such as `texture:/path/stone` (illustrative; file-extension conventions remain provisional).

- The filesystem locates assets. References use paths and, when necessary, stable member identifiers within an asset. Existing package roles such as `fix:/`, `level:/`, `texture:/`, and `sound:/` remain useful.
- Nest privately owned components. Give independently shared content references that preserve its identity and sharing. Equal values do not establish that two source objects were the same allocation.
- Asset file boundaries need not follow SNA blocks or engine struct boundaries. One useful asset may reconstruct many engine records and buffers spread across blocks.
- Content is loaded from package-scoped files into memory and saved through the selected target adapter. OpenSpace uses one active level with shared Fix/media packages; batch conversion can select levels sequentially. Level-specific Fix bindings belong to the session, not globally cached shared content.
- Loading a whole model, animation, or family into memory is appropriate. Resolve/cache referenced assets on demand when useful; there is no requirement for per-struct lazy hydration, a separately indexed binary container, a database, or one global document per engine struct kind.
- Preserve OpenSpace semantics, including animation hierarchy changes and mesh switching. Apply adaptations for Godot in the Godot exporter. Do not normalize away exact source values during import merely to fit a target engine's conventions.

Direct text editing and Git diffs of asset data are not requirements. Use Serde for Rete persistence. Each useful asset is one MessagePack file with named fields and embedded dense numeric data/byte buffers. Use MessagePack binary fields for raw bytes. Do not create a separate generic binary-content file or `.bin` sidecars; the filesystem still locates independently shared semantic assets. Keep inspection available through useful named keys and values. Validate schema/version conventions, file extensions, and performance with representative assets. PNG/WAV are primarily Godot outputs, not required Rete storage. Keep OpenSpace encoding choices and alignment in codec/profile rules; do not persist them as a shortcut for unresolved reconstruction behavior.

### Media representation

Choose the model/Rete representation for the information it must retain, before choosing a convenient playback or display format. Texture data may need palette entries and index identity, mip levels, or values beyond a flattened image; audio may need exact sample precision, channel relationships, timing/loop information, and meaningful codec-domain data beyond a playable waveform. Establish the actual requirements from the supported formats and evidence, rather than adding speculative fields.

Persist these useful values inside each MessagePack asset and attach only necessary source identity/order facts as metadata. If PNG or WAV can carry everything a particular asset needs, a standard media representation can be used; otherwise choose a richer lossless representation within the model/Rete design. Godot can receive PNG/WAV derived from those values. The target's media limitations must not discard information from the canonical model.

Minimal metadata follows the same rules as every other asset: no second original stream, raw encoded-blob replay, cached layout decisions, or hidden source reads. A lossy decode alone may not retain enough information for exact source re-encoding. Investigate useful codec-domain structure where required and keep any unresolved fidelity explicit; changing the file extension does not solve information loss. Test Rete save/load and re-encoding against that information, separately from testing Godot pixels or playback.

## Minimal reconstruction metadata is first-class

Useful content and exact source reconstruction are both goals. Store minimal source facts such as identity/sharing and original pool order as explicit OpenSpace metadata on the asset or component they describe. Package-wide source facts may have package-level metadata. Consumers can inspect and use the asset without interpreting that metadata. The exporter uses these facts and current content to regenerate the physical representation.

**Derive encoding, alignment, sizes, counts, placement, actual pointers, checksums, and relocation entries during export.** These are format/profile rules applied to the current asset values, not decisions cached from import. A matrix's original pool index is appropriate metadata; its original pointer, byte offset, alignment padding length, or encoding-selection flag is not the reconstruction strategy.

Meaningful values and explicitly unresolved data must still be represented faithfully. For example, an unknown nonzero source field must not be discarded by assuming it is padding. Treat unexplained bytes or multiple possible encodings as research gaps: establish a rule from source evidence and tests instead of automatically adding metadata to replay the original choice. Track unknown data separately from demonstrated semantic coverage. Exact pre-compression parity remains the objective; the historical near-match in LZO compression is a separate issue.

Do not store a second copy of understood source content, original encoded blobs to replay, or relocation tables to patch back. Export must rebuild from the structured assets and their metadata without consulting the original disc. VM pointer integers are generated at the OpenSpace boundary, not used as the canonical reference network. Persisted reconstruction metadata does not require constructing VM state when loading Rete or exporting Godot.

### Matrix pool example

The prior C# work struggled to reconstruct the order of transforms grouped in a block separate from the real content. Semantic traversal order need not reproduce that sequence. An imported transform can carry an ID/index describing its position in its original matrix pool while remaining accessible through its owning animation or other asset.

For example, an illustrative metadata object could be `openspace: { pool: "level-transforms", original_index: 137 }`. These are explanatory names, not an adopted schema. Omit the pool field if the owning context already identifies it unambiguously.

At export:

1. Collect the transforms required by the content, preserving source sharing. Several references to one imported transform identify one pool entry; separate original entries with equal values remain distinct.
2. Reconstruct each original pool's sequence using its scoped indices. Keep the index stable when editing a transform's value. Conflicting definitions for the same source slot require resolution rather than arbitrary overwrite.
3. Derive the compact encoding and alignment from the current transform values and verified Hype format rules. Compute sizes, placement, and resulting addresses, then materialize pointers and generate relocation tables.

An ordinal is not a byte offset. It recovers order; complete pool membership and the derived encoding/alignment rules determine placement. Investigate unexplained intervening data instead of saving original offsets to force a match. Shared components must have one authoritative value even when multiple asset files refer to them.

For inserted, removed, or resized content, retain applicable original ordering metadata and use a deterministic valid layout for the changed content. Do not blindly renumber surviving entries by traversal order. Define how each supported edit affects sharing and metadata rather than assuming every historical index remains applicable. New content here means additions to a modification of the original game.

Preserving order can help patches avoid unnecessary differences. It does not guarantee small binary patches: resizing allocations or recompressing a block can affect many bytes. Patch distribution format and baseline selection are separate, currently undecided concerns.

## Develop from useful assets toward package parity

Start with the useful API for a priority asset and a small dependency-complete example. Implement its OpenSpace interpretation, Rete persistence, and reconstruction together. Do not begin by translating every engine struct into a serialized record while requiring full-package identity.

Validation should establish progressively:

1. Meaningful inspection of the asset, its dependencies, and at least one deliberate edit with the expected effect.
2. Rete save/load fidelity for values, identity, references, and reconstruction metadata. Unit tests that do not test persistence can stay in memory; persistence tests must exercise actual serialization.
3. Byte parity for supported records, buffers, and assets under a controlled export layout, including correct reference targets. Assets spanning SNA allocations need these local tests even when there is no standalone original asset file.
4. Complete each useful asset's trip to Godot and compare its geometry, materials, animation, or behavior with the OpenSpace version. Add individual original-file and decompressed-block parity as coverage permits. Use Raymap for practical comparison and original-engine runs when available; engine availability is not a universal gate.
5. Full unedited package parity, including generated relocation data, followed by exact compressed/container-byte parity.

Keep semantic correctness, reconstruction coverage, and runtime behavior visible as separate evidence. An opaque byte copy can pass a byte comparison without demonstrating understanding. A render can look correct while dropping animation, AI, collision, or encoding information. Unknown regions and unsupported behaviors remain explicit work even when a smaller test passes. A runtime test must exercise the generated asset, not silently fall back to the original data.

## Downstream Godot dependencies

The final deterministic Godot arrangement should support separately authored content. Defer long-lived converter-owned assets and durable downstream dependencies until OpenSpace import/export, Rete load/save, and Godot export work for the supported game content. Godot import remains optional. Temporary development/test assets are permitted while exercising the adapters, with provisional naming and schemas.

Game-parity, full-working-game, and fan-remaster projects own their durable assets and depend on Astrolabe, even if packaged together. Establish their stable generated paths and runtime contracts after that gate. Regeneration always preserves unrelated/authored content. See [development outputs and downstream projects](rust-architecture.md#development-outputs-and-downstream-projects).

## Runtime validation references

The user supplied [`../reference/hype-the-time-quest-lutris.json`](../reference/hype-the-time-quest-lutris.json) to support running the original game under Lutris/Wine. The user also supplied `reference/Savefile_EN.zip` to bypass the very slow intro. Use it as the initial runtime fixture, preserving its `SaveGame0/` directory and verifying the installation's edition/language, actual save root, and loaded state. Archive integrity has been checked; loading it in the engine has not. Other chosen saves or targeted process-memory editing can reach additional test states when necessary. Raymap's web viewer is another useful reference; verify its local/generated-level loading workflow before depending on it. The user does not use the Unity Editor. Original-engine tests are opportunistic and must not become a prerequisite for every feature; Raymap is the practical backup, supplemented by source-informed structural/behavioral checks. Pixel-perfect Godot output is a useful comparison target, not a universal acceptance requirement. The original engine/decompilation is authoritative, followed by Raymap; consult Raymap first in ordinary research, and corroborate C# and historical notes. See [reference authority](../reference/README.md#reference-authority-and-research-workflow).

Read [`../reference/README.md#runtime-validation`](../reference/README.md#runtime-validation) and the root contributor guidance before setting up runtime tests. Record reproducible saves/interventions and distinguish setup assumptions from successful runs. Runtime setup has not been performed as part of documenting these decisions.
