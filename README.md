# Astrolabe

**Make a tool to repeatably port this OpenSpace game to Godot.**

Astrolabe is a Rust library and CLI that converts **Hype: The Time Quest**
OpenSpace data into a native Godot project using your local game copy. This is
still a partial game port; [coverage and evidence](./notes/implementation-progress.md)
distinguish implemented code from demonstrated gameplay.

## Open the native Godot project

Requires Rust/Cargo, Godot **4.7.2**, and a local Hype game directory containing
`Gamedata/` and `LangData/`. The example uses the ignored `disc/` directory;
any mounted or extracted game directory can be supplied instead.

From the repository root:

```sh
cargo build --release --locked
target/release/astrolabe prepare-game disc godot
godot --editor --path godot
```

Preparation converts the manor, main menu, inventory and pause menu, imports
native Godot resources, and reports each stage. It defaults to `manoir` and
English (`en`), the current authored game's supported selection; `--godot`
selects another Godot executable. Successful scopes retain their separate
ownership if a later scope fails. Resolve the reported failure and repeat the
command; authored files, edited generated files and unrelated levels retain
the existing publication protections.

Press **F5**, then choose **New Game** for the opening or **Load Game** for the
saved start. New Game plays the full converted opening with compiled AI and hands
control to Hype. The player can then walk, run, jump, draw and swing the sword,
fire the crossbow, cast Fire, collect pickups, save at the lectern or with F5, and
open the inventory. These match the original's action sequences. Fight outcomes,
progression and the rest of the manor are not finished. F1 shows a developer panel.

Load Game restores a real original slot after `astrolabe import-save` converts it
to a named-key Godot file, and reports how many records it understands. See
[save games](./docs/openspace/save-games.md).

Calls the port can't perform yet run as documented
[stand-ins](./docs/astrolabe/stand-ins.md), so the game keeps going. See the
[game guide](./godot/README.md) and the
[coverage notes](./notes/implementation-progress.md).

## How it works

The **OpenSpace** adapter decodes game files into one shared model of textures,
audio, geometry, scenes, characters and typed AI. **Rete** persists those same
assets as individual MessagePack files. The **Godot** adapter writes saved native
scenes, Resources and animations, and compiles supported AI into GDScript that
calls maintained gameplay components. Generated content opens in the editor;
the port does not reconstruct assets or interpret AI opcode tables at runtime.
See the [architecture](./docs/astrolabe/rust-architecture.md) and
[Rete design](./docs/rete/rete-design.md).

The committed `godot/game/` project owns authored application/session behavior;
`godot/addons/astrolabe/` is converter support; ignored `generated/` content comes
from the local game copy. Downstream remaster projects can own their authored
assets and depend on the same generated content. The archived C# implementations
remain research references, independent of the Rust implementation.

`port GAME PROJECT` remains the developer command for one selected asset scope:
it defaults to scene conversion, accepts explicit level/locale selection, and
supports `--content model`, `menu`, `inventory`, `pause-menu`, `textures` or `audio`.

To save and inspect Rete, then reconstruct supported source content:

```sh
cargo run --release --locked -- import-openspace disc output/rete --level astrolabe --content model
cargo run --release --locked -- inspect output/rete --asset level:/models/world/1/0/0 --kind model
cargo run --release --locked -- export-godot output/rete output/godot-from-rete
cargo run --release --locked -- export-openspace output/rete output/reconstructed
cargo run --release --locked -- formats
```

Use `--content textures` or `--content audio` on `port`/`import-openspace` for standalone media; `--audio-file Gamedata/World/Sound/Achat.apm` selects a focused audio input. Full model reconstruction currently emits a controlled fragment with generated relocations, not a complete loadable level. `formats` and conversion reports describe writer restrictions.

Before merging, run the gate. It takes about five minutes and asks one question: is this still a game?

```sh
python3 tools/gate.py
```

It needs a prepared project and `hype-harness/local`, and never touches the original install: the original runs from a disposable copy. Broader checks use the test supervisor, run when they answer a question. A full local run takes about 51 minutes:

```sh
python3 tools/test.py --area movement --local --offline
python3 tools/test.py --local --offline
```

See [testing](./docs/astrolabe/testing.md) for commands and policy.

## Direction

**Carve out the major systems and placeholders first, then fill them out, approaching game parity.** The big things must work: the frame rate holds, the AI scripts fire, and the game is actually a game. Things that merely don't quite match the original can wait. The [plan](./plan.md) sets out the milestones and what is deliberately waiting.

## C# / Unity

A C# AI backend spike lives on branch `spike/csharp-ai-backend`; its findings are in
`notes/csharp-ai-backend-spike-2026-11-03.md` on that branch. `port --ai-backend csharp`
converted all 150 Manoir and Brigand AI models (Fix models included) to C# classes that
compile (6.3 MB Release assembly). Loads were faster (Manoir from a save 5.5-6.6 s against
7.5-7.7 s; Brigand travel 8.5-9.6 s against 10.7-11.5 s), and per-tick AI was about 25%
cheaper. The costs: the .NET Godot build and mono export templates, a .NET SDK for anyone
building, about 86 MB of runtime per desktop export, no Web export, and C#-to-GDScript calls
going through `Call`/`Get`/`Set`. It is not merged. If anyone asks to add Unity support to
this project, start there: a C# emitter from the typed AI model could serve both Godot C#
and Unity.

## Variants

The converter's output is a faithful, Godot-native base. Gameplay is one native
implementation matched behaviorally to the original; presentation variants are runtime
modules and modernization is an override layer. See [docs/astrolabe/variants.md](docs/astrolabe/variants.md).

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

Read the [plan](./plan.md), the [Rust architecture](./docs/astrolabe/rust-architecture.md) and the [Rete asset design](./docs/rete/rete-design.md). The earlier plan and format matrix are in [notes/historical](./notes/historical/plan-2026-10.md). The historical C# plan is preserved in [`reference/astrolabe-csharp/plan.md`](./reference/astrolabe-csharp/plan.md); the old Rete JSON specification and implementation checklists are research material. Current evidence and remaining limitations are in the [implementation report](./notes/implementation-progress.md).

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
disc/          Local raw game data (gitignored; any source directory is accepted)
output/        Converted/exported assets (gitignored)
```

## Game data and license

Astrolabe expects a legally obtained copy of *Hype: The Time Quest*. Game files remain local and are not included in Git.

Astrolabe is released under CC0; see [`LICENSE`](./LICENSE). Third-party references and helper tools retain their own licenses.
