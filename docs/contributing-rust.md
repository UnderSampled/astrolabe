# Working on the Rust code

Start with the useful asset you want to add or change. Its meaning lives in `model`; each adapter owns the code that reads or writes its representation. The project uses one crate with ordinary Rust modules. There is no runtime plugin registry or separate OpenSpace document model to learn first.

Read [the architecture](rust-architecture.md), [asset ownership rules](rete-design.md), and [current coverage](implementation-progress.md) before adding a domain. The [plan](../plan.md) distinguishes implemented asset slices from future scene/game work.

## Where to start reading

Follow a model conversion in this order:

1. [`model/geometry.rs`](../src/model/geometry.rs) defines `Model`, `ModelInstance`, `Material`, and their invariants. A `Model` owns reusable geometry. A `ModelInstance` references it and owns its selected LOD distance, transforms, and radiosity.
2. [`openspace/content.rs`](../src/openspace/content.rs) selects source content and exposes an `Imported` session with roots, a dependency inventory, and a `Load<T>` resolver. The first-model path delegates source traversal to [`level/import.rs`](../src/openspace/level/import.rs).
3. [`godot/export.rs`](../src/godot/export.rs) walks those roots and their dependencies. [`resources.rs`](../src/godot/resources.rs) projects model values into Godot resources; [`project.rs`](../src/godot/project.rs) stages and validates the output. [`publication.rs`](../src/godot/publication.rs) protects authored files during regeneration.
4. [`rete/mod.rs`](../src/rete/mod.rs) can save the same dependency inventory and reopen it as a resolver. Godot receives the same types through either route.
5. [`openspace/export.rs`](../src/openspace/export.rs) reconstructs supported source content through that resolver. The current level writer emits controlled fragments and reports its coverage; it does not assemble a complete playable level.
6. [`app.rs`](../src/app.rs) selects and composes these entry points. Target naming, mesh construction, and binary layout belong to the adapters.

[`tests/model_instances.rs`](../tests/model_instances.rs) is a small complete example: construct one model with independently placed/lit instances, persist and edit it, reconstruct source fragments, and inspect actual Godot mesh sharing and lighting. [`tests/real_routes.rs`](../tests/real_routes.rs) applies the same route to local game data.

## Module map

| Location | Responsibility |
| --- | --- |
| `model/io.rs`, `model/reference.rs` | Typed loading/saving, identity validation, role URIs, selection, shared diagnostics |
| `model/content.rs` | In-memory collection and the built-in asset catalogue used for exhaustive target dispatch |
| `model/texture.rs`, `model/audio.rs`, `model/geometry.rs` | Domain values, semantic validation, Serde derives, and `Content` implementations |
| `model/ai.rs`, `openspace/ai.rs`, `openspace/level/ai.rs` | Typed behavior/memory values, script codec and session-bound source AI import |
| `godot/ai.rs`, repository `godot/addons/astrolabe/behavior/` | GDScript compilation and maintained timing/actor/audio functions |
| `openspace/codec.rs` | Narrow typed binary/text codec interface and supported-format catalogue |
| `openspace/texture/gf.rs`, `cnt.rs` | Image encoding versus archive membership, sharing, and placement |
| `openspace/audio/` | APM, BNM, MPEG adaptation, and sound-definition parsing/writing |
| `openspace/level/records.rs` | Shared binrw declarations and named pointer-field offsets |
| `openspace/level/container.rs` | Bounded SNA framing, relocation groups, binary read/write helpers |
| `openspace/level/session.rs` | Transient addresses, active Fix/level binding, rooted traversal |
| `openspace/level/import.rs` | Source records and dependencies decoded into useful model values |
| `openspace/level/geometry.rs`, `fragment.rs` | Derived geometry layout and parent model/material/instance reconstruction |
| `godot/resources.rs`, `export.rs` | Target value mapping and dependency traversal |
| `godot/project.rs`, `engine.rs`, `publication.rs` | Staging, bounded engine execution, deterministic normalization, safe publication |
| `godot/animation.rs`, `godot/character.rs`, `godot/sound.rs` | Native pose/resource projection and compiled sound behavior |
| `godot/scripts/build.gd`, `godot/preview.gd` | Disposable build and preview harnesses |
| repository `godot/addons/astrolabe/`, `src/godot/support.rs` | Authoritative support source, versioned installation/update rules |

Keep related record declarations together. A new file should own a recognizable responsibility, rather than one struct or one forwarding function. Public `mod.rs` files expose the useful entry points; internal helpers stay private to their adapter.

## Godot support and project work

The native exporter can introduce maintained Godot shaders, components and
engine-side functions as supported slices require them. Read
[Godot project structure and ownership](godot-project.md) before adding those
assets. The committed `godot/` project separates reusable support, authored game
work and ignored generated content. Its bootstrap and synthetic example work
dehydrated; the supported scene slice supplies native resources after conversion.

Keep static support source in ordinary Godot files with one authoritative copy.
Rust owns conversion rules and generated parameter/code values. Converter-only
build scripts and synthetic inspection harnesses remain distinct from support
used by exported projects. Test the same support package through direct export
and reopened Rete, and verify an authored extension survives regeneration.

## Add another source format

If an encoding represents an existing domain, reuse that domain type. Add its codec beside the related formats, with a `FormatCodec` implementation in the same module. [`texture/gf.rs`](../src/openspace/texture/gf.rs) is a compact example. Register its support description in `codec::catalogue` and connect source selection in `openspace/content.rs` when needed. Rete and Godot should need no changes for another encoding of an existing useful type.

Use binrw for declarations, counts, endianness, and record assertions. Use named helpers for RLE, reference resolution, and other algorithms that do not read naturally as fields. Both read and write paths should use the same declarations. Pointer-field constants describe packed source offsets, never Rust struct layout. The geometry and parent-fragment writers append to one `Layout`, which derives placement and emits relocations once. `openspace/export.rs` tracks generated filenames across all roots and dependencies: repeated references emit once, while distinct asset identities mapping to the same output are rejected before the conflicting write.

Record the source evidence and unsupported cases beside the codec or in a focused format note. Consult Raymap first, resolve conflicts against the relevant engine/decompilation, and retain regression evidence from the archived C# code without making it a runtime dependency.

## Add a useful content type

Define the type and its invariants in the owning model module. Implement `Content` there, including `identity()` when the value stores its own path. References and embedded identities must agree; renaming requires updating both. Separate locally checkable invariants from validation that needs a resolved dependency, as `ModelInstance::validate_model` does.

For a built-in package domain, add a variant and loading branch to `Asset` and a bound to `ContentSource` in `model/content.rs`. The compiler then identifies every exhaustive adapter match that must support or explicitly reject the new kind: Godot export, OpenSpace export, Rete saving, and inspection. This explicit list is intentional; adding a domain must not silently bypass a target. Custom library-only `Content` types can already use generic `Load<T>` and Rete `Save<T>` without joining the built-in catalogue.

Use one independently shared value and references to it. A `TextureSet` owns ordered entries, each pointing to a `Texture`; `TextureArchive` is only the OpenSpace import result containing that root and its image dependencies. Editing an image affects all aliases. Detaching an alias means creating an image under a new asset path and updating that entry's reference. Equal values at different paths remain distinct assets.

Keep explanatory notes informational. When a source feature cannot yet be reconstructed, retain a typed gap such as `AudioReconstructionGap`; writers test that state and current values. Rewording a note must not enable or disable export.

## Add another target

Create a peer adapter module that consumes model values through `Load<T>` or `ContentSource`. Put its naming, coordinate conversion, encoding, dependency traversal, and output validation there. Expose an export entry point and connect it from the application. Do not make the target parse Rete envelopes or rebuild OpenSpace address tables to obtain content.

Start with a synthetic asset and a small dependency graph. Exercise shared references, independent equal values, an identity mismatch, and a deliberate edit through saved/reloaded Rete. Validate actual target resources or behavior when available. For Godot, reserve every output path before either Rust or `ResourceSaver` writes it, and publish through the existing ownership checks.

## Readability and verification

Prefer names that explain source roles and operations over single-letter traversal variables. Keep diagnostic strings and generated code readable across multiple lines. Comments should explain evidence, units, ownership, or a non-obvious constraint. Avoid narrating an assignment or repeating a type name. Static GDScript lives in `.gd` files; generated expressions use readable multiline templates next to their value mapping.

Rete currently uses **schema 25**, including typed AI values/action arrays and character collision profiles; Godot support is interface 17. Regenerate older local packages from source; there is no compatibility reader. Generated paths and support interfaces remain provisional. The supported native scene/character and torch behavior slices are implemented; full scene/game semantics and package reconstruction remain partial. See [implementation progress](implementation-progress.md) for checkpoint results and limits.

Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo build --release --locked`. Existing commands are `python3 tools/test.py --offline` for synthetic checks and `python3 tools/test.py --local --offline` for the complete game-corpus/Godot suite. The current supervisor still caps its invocation at 570 build/test seconds. The new policy calls for fast bounded tests, affected-area selection plus mandatory safeguards, and a potentially longer full release matrix; implementation is planned in [live testing and comparison](realtime-comparison-plan.md). See [testing.md](testing.md) for available commands and logs.
