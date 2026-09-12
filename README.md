# Astrolabe

**Make a tool to repeatably port this OpenSpace game to Godot.**

Astrolabe converts **Hype: The Time Quest** (1999), built on Ubisoft's OpenSpace Montreal engine, into useful, deterministic Godot content. Downstream fan-remaster and game-parity projects can depend on assets generated locally from the user's game copy. Those projects own their durable game assets and depend on Astrolabe, even if packaged together.

The first Rust library and CLI restore LZO, textures, and standalone audio, and convert a rooted textured model from the supplied Astrolabe level into a runnable Godot development project. Rete persists the same useful values. The previous C# implementation and the `gltf` branch remain pinned reference submodules. This is an initial asset converter, not a complete game port; see the [coverage and evidence](docs/implementation-progress.md).

Astrolabe has **one useful intermediate model**: textures, models, animations, scenes, sounds, and behaviors with meaningful fields and relationships. **Rete is the serialized form of that model**, using Serde/MessagePack. The planned binrw codecs declaratively map OpenSpace encodings to and from these same useful types, through a unified interface so independent formats can be documented and implemented alongside one another. See the [binary codec boundary](docs/rust-architecture.md#binary-codecs-with-binrw).

**Three modular I/O adapters organize the tool:** OpenSpace reads and reconstructs game data, Rete loads and saves the shared model, and Godot exports it into the target engine. Application code composes them through the model. These module boundaries matter independently of how many Rust crates are used; Godot import remains optional future work. See the [architectural overview](docs/rust-architecture.md#three-modular-io-targets-one-useful-model).

## Run the first model

Requires Rust/Cargo, Godot 4 (verified with `4.7.2.stable.arch_linux.ed1daf0bf`), and the local game files under `disc/`:

```sh
cargo run --release --locked -- port disc output/godot --level astrolabe
godot --path output/godot
```

The disposable viewer supports mouse drag to orbit and wheel to zoom. The default scope selects the first rooted model and its material/texture dependencies. It does not export the whole level.

To save and inspect Rete, then reconstruct supported source content:

```sh
cargo run --release --locked -- import-openspace disc output/rete --level astrolabe
cargo run --release --locked -- inspect output/rete --asset level:/models/world/1/0/0 --kind model
cargo run --release --locked -- export-godot output/rete output/godot-from-rete
cargo run --release --locked -- export-openspace output/rete output/reconstructed
cargo run --release --locked -- formats
```

Use `--content textures` or `--content audio` on `port`/`import-openspace` for standalone media; `--audio-file Gamedata/World/Sound/Achat.apm` selects a focused audio input. Full model reconstruction currently emits a controlled fragment with generated relocations, not a complete loadable level. `formats` and conversion reports describe writer restrictions.

Run the complete local corpus and engine suite with one enforced deadline:

```sh
python3 tools/test.py --local --offline
```

The entire invocation, including compilation and cleanup, must finish under ten minutes. Use `python3 tools/test.py --offline` for synthetic checks. See [testing](docs/testing.md) for dependencies, logs, and deadline enforcement.

## Goals

- Repeatably convert a mounted or extracted game directory into a runnable Godot project, with stable resource paths, scene/node identities, and animation names that authored content can depend on.
- Preserve useful texture/audio information in the model and Rete, choosing richer representations when PNG/WAV are insufficient. Generate PNG/WAV for Godot where suitable; keep original-game assets local.
- Import OpenSpace levels and shared Fix data into distinct **Level** and **Fix** roots within the same useful intermediate model, resolving assets on demand where useful and binding shared packages to one active level at a time.
- Persist useful content using Serde as one **MessagePack Rete file per asset**, including its dense arrays and byte buffers. Use file references between independently shared models, materials, animations, AI scripts, scene graphs, and perso families; no generic binary-content sidecars.
- Preserve minimal source identity/sharing/order metadata, such as a matrix's original pool index. Derive OpenSpace encoding, alignment, sizes, placement, actual pointers, and relocations when exporting.
- Test useful assets through Godot using source-informed expectations and Raymap, with original-engine comparisons when feasible. Pixel-perfect output is a useful parity target, not a universal requirement.
- Rebuild OpenSpace files from canonical content without consulting the original disc. Prove unedited pre-compression package parity; close the remaining compression differences separately.
- Keep a possible Godot-to-OpenSpace mod-editing round trip as an optional later capability.

During adapter development, use temporary test/development assets and provisional output contracts. Defer long-lived converter-owned assets and durable downstream dependencies until OpenSpace import/export, Rete load/save, and Godot export work for the supported game content. Godot reverse import stays optional. The unified interface organizes package content loaded into memory, converted, and saved through these targets.

Read the [Rust architecture](docs/rust-architecture.md), [Rete asset design](docs/rete-design.md), and [ordered implementation plan and format matrix](plan.md). The historical C# plan is preserved in [`reference/astrolabe-csharp/plan.md`](reference/astrolabe-csharp/plan.md); the old Rete JSON specification and implementation checklists are research material. Current evidence and remaining limitations are in the [implementation report](docs/implementation-progress.md).

## References

| Path | Purpose |
| --- | --- |
| [`reference/astrolabe-csharp/`](reference/astrolabe-csharp/) | C# worktree snapshot, including the previously uncommitted source and notes, at `b1fba2ec614ffa79983fa0f54755b35f3de6269b`. |
| [`reference/astrolabe-gltf/`](reference/astrolabe-gltf/) | Local `gltf` branch snapshot at `aa688f478ea967df15cc72b5ed8191f0cea494ef`. |
| [`reference/BinarySerializer/`](reference/BinarySerializer/) | Former C# dependency, retained for research with its local edits committed. |
| [`reference/BinarySerializer.OpenSpace/`](reference/BinarySerializer.OpenSpace/) | Former OpenSpace C# dependency, retained for research with its local edits committed. |

The supplied [Lutris installer reference](reference/hype-the-time-quest-lutris.json), runtime-testing guidance, Raymap web viewer, and other community references are documented in [`reference/README.md`](reference/README.md). The archived C# README contains its original .NET build and CLI instructions. The original engine/decompilation is the source of truth, followed by Raymap, which is usually the first practical reference consulted. C# and old notes remain valuable but need corroboration; see [reference authority](reference/README.md#reference-authority-and-research-workflow). Reference code is research material and is not part of the Rust implementation.

## Project layout

```text
src/model/     Useful values and shared typed references/load/save contracts
src/openspace/ Declarative binrw codecs, source resolution and reconstruction
src/rete/      Per-asset MessagePack persistence
src/godot/     Native target resources and disposable development viewer
src/app.rs     CLI and adapter composition
tests/         Synthetic, corpus and actual Godot checks
docs/          OpenSpace, Hype, and Rete format documentation
notes/         Research and historical C# implementation notes
reference/     Archived Astrolabe versions and third-party submodules
tools/lzo1x/   Existing standalone C compressor helper
tmp_probe/     Retained Python investigation scripts
disc/          Local raw game data (gitignored)
output/        Converted/exported assets (gitignored)
```

## Game data and license

Astrolabe expects a legally obtained copy of *Hype: The Time Quest*. Game files remain local and are not included in Git.

Astrolabe is released under CC0; see [`LICENSE`](LICENSE). Third-party references and helper tools retain their own licenses.
