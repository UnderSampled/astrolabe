# Astrolabe

**Make a tool to repeatably port this OpenSpace game to Godot.**

Astrolabe converts **Hype: The Time Quest** (1999), built on Ubisoft's OpenSpace Montreal engine, into useful, deterministic Godot content. Downstream fan-remaster and game-parity projects can depend on assets generated locally from the user's game copy. Those projects own their durable game assets and depend on Astrolabe, even if packaged together.

The target is native Godot project files: editor-visible level scenes and instanced props/persos, native resources and animations, and AI ASTs converted into GDScript. Conversion should make full use of Godot's scene tree and asset pipeline. The supported scene/character slice uses saved native scenes and animations. Compiled AI runs the torch sound controllers and the demonstrated Hype movement, jump, sword and Fire slices in `manoir`; see the [implementation evidence](./notes/implementation-progress.md), [conversion contract](./docs/astrolabe/rust-architecture.md#godot-adapter-native-project-conversion) and [next priorities](./notes/remaining-work.md).

The Rust library and CLI restore LZO, textures, and standalone audio, and convert supported levels and characters into a native Godot development project. Rete persists the same useful values. The previous C# implementation and the `gltf` branch remain pinned reference submodules. This remains a partial game port; see the [coverage and evidence](./notes/implementation-progress.md).

Astrolabe has **one useful intermediate model**: textures, models, animations, scenes, sounds, and behaviors with meaningful fields and relationships. **Rete is the serialized form of that model**, using Serde/MessagePack. The binrw codecs declaratively map OpenSpace encodings to and from these same useful types, through a unified interface so independent formats can be documented and implemented alongside one another. See the [binary codec boundary](./docs/astrolabe/rust-architecture.md#binary-codecs-with-binrw).

**Three modular I/O adapters organize the tool:** OpenSpace reads and reconstructs game data, Rete loads and saves the shared model, and Godot exports it into the target engine. Application code composes them through the model. These module boundaries matter independently of how many Rust crates are used; Godot import remains optional future work. See the [architectural overview](./docs/astrolabe/rust-architecture.md#three-modular-io-targets-one-useful-model).

## Open the native Godot project

Requires Rust/Cargo, Godot 4 (verified with `4.7.2.stable.arch_linux.ed1daf0bf`), and the local game files under `disc/`:

```sh
cargo build --release --locked
target/release/astrolabe port disc godot --level manoir --locale en
godot --editor --path godot
```

Let the editor finish importing the generated textures and sounds, then press
**F5** and select **Load Game**. The menu works without game data. This is the
Godot implementation of the game: its current load route uses the saved level,
compiled Hype scripts and a measured English-save fixture for the working
movement, jump, sword and Fire slices. General save loading will replace that
fixed restoration; New Game will begin with the intro when implemented.
Combat damage, crossbow play, spell contact/lifetime, death and source camera
parity remain open. Open
generated scenes and native animation libraries directly in the editor when
examining converted content. The default selects the supported scene slice;
`--content model` selects the first rooted model explicitly. See the
[native migration evidence](./notes/native-godot-migration.md).

To save and inspect Rete, then reconstruct supported source content:

```sh
cargo run --release --locked -- import-openspace disc output/rete --level astrolabe
cargo run --release --locked -- inspect output/rete --asset level:/models/world/1/0/0 --kind model
cargo run --release --locked -- export-godot output/rete output/godot-from-rete
cargo run --release --locked -- export-openspace output/rete output/reconstructed
cargo run --release --locked -- formats
```

Use `--content textures` or `--content audio` on `port`/`import-openspace` for standalone media; `--audio-file Gamedata/World/Sound/Achat.apm` selects a focused audio input. Full model reconstruction currently emits a controlled fragment with generated relocations, not a complete loadable level. `formats` and conversion reports describe writer restrictions.

Run selected development checks or the complete release matrix with the repository test supervisor:

```sh
python3 tools/test.py --local --offline
```

Keep individual tests and active development-area checks bounded; the full release matrix may take longer. Use `python3 tools/test.py --offline` for synthetic checks, `python3 tools/test.py --local --offline` for the complete local corpus and Godot suite, and `python3 tools/test.py --release --offline` for the declared release matrix. Focused selections use `--area`. See [testing](./docs/astrolabe/testing.md) for commands and policy; current coverage and runtime evidence are recorded in [notes](./notes/implementation-progress.md).

## Goals

- Repeatably convert a mounted or extracted game directory into a runnable Godot project, with stable resource paths, scene/node identities, and animation names that authored content can depend on.
- Preserve useful texture/audio information in the model and Rete, choosing richer representations when PNG/WAV are insufficient. Generate PNG/WAV for Godot where suitable; keep original-game assets local.
- Import OpenSpace levels and shared Fix data into distinct **Level** and **Fix** roots within the same useful intermediate model, resolving assets on demand where useful and binding shared packages to one active level at a time.
- Persist useful content using Serde as one **MessagePack Rete file per asset**, including its dense arrays and byte buffers. Use file references between independently shared models, materials, animations, AI scripts, scene graphs, and perso families; no generic binary-content sidecars.
- Preserve minimal source identity/sharing/order metadata, such as a matrix's original pool index. Derive OpenSpace encoding, alignment, sizes, placement, actual pointers, and relocations when exporting.
- Test useful assets through Godot using source-informed expectations and Raymap, with original-engine comparisons when feasible. Pixel-perfect output is a useful parity target, not a universal requirement.
- Rebuild OpenSpace files from canonical content without consulting the original disc. Prove unedited pre-compression package parity; close the remaining compression differences separately.
- Keep a possible Godot-to-OpenSpace mod-editing round trip as an optional later capability.

Develop needed converter-owned Godot shaders, components and engine functions alongside the exporter and OpenSpace/Model/Rete parity work, with clear ownership and tested interfaces. Design for a Godot developer in both the committed/dehydrated project and the rehydrated project containing locally converted assets; supported authored consumers can grow with those slices. Most AI engine functions can wait for the GDScript tests that need them, and Godot reverse import stays optional. See [Godot project structure and ownership](./docs/astrolabe/godot-project.md) for the maintained support package, native editing workflow and safe regeneration rules.

Read the [Rust architecture](./docs/astrolabe/rust-architecture.md), [Rete asset design](./docs/rete/rete-design.md), and [ordered implementation plan and format matrix](./plan.md). The historical C# plan is preserved in [`reference/astrolabe-csharp/plan.md`](./reference/astrolabe-csharp/plan.md); the old Rete JSON specification and implementation checklists are research material. Current evidence and remaining limitations are in the [implementation report](./notes/implementation-progress.md).

## References

| Path | Purpose |
| --- | --- |
| [`reference/astrolabe-csharp/`](./reference/astrolabe-csharp) | C# worktree snapshot, including the previously uncommitted source and notes, at `b1fba2ec614ffa79983fa0f54755b35f3de6269b`. |
| [`reference/astrolabe-gltf/`](./reference/astrolabe-gltf) | Local `gltf` branch snapshot at `aa688f478ea967df15cc72b5ed8191f0cea494ef`. |
| [`reference/BinarySerializer/`](./reference/BinarySerializer) | Former C# dependency, retained for research with its local edits committed. |
| [`reference/BinarySerializer.OpenSpace/`](./reference/BinarySerializer.OpenSpace) | Former OpenSpace C# dependency, retained for research with its local edits committed. |

The supplied [Lutris installer reference](./reference/hype-the-time-quest-lutris.json), runtime-testing guidance, Raymap web viewer, and other community references are documented in [`reference/README.md`](./reference/README.md). The archived C# README contains its original .NET build and CLI instructions. The original engine/decompilation is the source of truth, followed by Raymap, which is usually the first practical reference consulted. C# and old notes remain valuable but need corroboration; see [reference authority](./reference/README.md#reference-authority-and-research-workflow). Reference code is research material and is not part of the Rust implementation.

## Project layout

Start with the [documentation map](./docs/README.md) and [working-notes guide](./notes/README.md). The [Rust contributor guide](./docs/astrolabe/contributing-rust.md) gives a reading order, module map, and worked extension paths for formats, content types, and targets.

```text
src/model/     Useful values and shared typed references/load/save contracts
src/openspace/ Declarative binrw codecs, source resolution and reconstruction
src/rete/      Per-asset MessagePack persistence
src/godot/     Native conversion rules and disposable build/inspection harnesses
godot/         Committed starter, maintained support, authored scenes and ignored rehydrated assets
src/app.rs     CLI and adapter composition
tests/         Synthetic, corpus and actual Godot checks
docs/          Stable OpenSpace knowledge and Astrolabe, Rete, and Orrery design
notes/         Dated status, plans, experiments, handoffs, and historical research
reference/     Archived Astrolabe versions and third-party submodules
tools/lzo1x/   Existing standalone C compressor helper
tmp_probe/     Retained Python investigation scripts
disc/          Local raw game data (gitignored)
output/        Converted/exported assets (gitignored)
```

## Game data and license

Astrolabe expects a legally obtained copy of *Hype: The Time Quest*. Game files remain local and are not included in Git.

Astrolabe is released under CC0; see [`LICENSE`](./LICENSE). Third-party references and helper tools retain their own licenses.
