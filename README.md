# Astrolabe

Astrolabe is a toolkit for extracting and converting data from **Hype: The Time Quest** (1999) into formats that are useful for inspection, reverse engineering, and Godot-based projects.

The game uses Ubisoft's OpenSpace Montreal engine, also used by titles such as *Rayman 2* and *Tonic Trouble*. Astrolabe focuses on the Hype data pipeline:

- Read files from a mounted disc directory or already extracted game directory.
- Convert texture containers to PNG and audio files to WAV.
- Unpack OpenSpace level files into their embedded binary objects, scene graphs, meshes, materials, families, and scripts.
- Convert levels and assets into Godot projects, scenes, meshes, textures, and scripts.


## Legal Notice

Astrolabe expects a legally obtained copy of *Hype: The Time Quest* as a mounted disc directory or extracted files.

## Requirements

- .NET 10 SDK/runtime
- Git submodules initialized
- A mounted or extracted Hype disc directory
- Optional: Godot 4 for opening generated projects

## Setup

Clone with submodules, or initialize them after cloning:

```bash
git submodule update --init --recursive
dotnet build
```

Run the CLI through the project:

```bash
dotnet run --project src/Astrolabe.Cli -- <command> [args]
```

For shorter examples below, `astrolabe` means the same command prefix.

## Workflow

Start from a mounted disc directory or a directory that already contains the game files.

Copy raw files into a local working directory if needed:

```bash
astrolabe extract /mnt/hype ./disc --raw
```

Convert supported loose assets for inspection:

```bash
astrolabe extract ./disc ./output
```

Inspect the OpenSpace scene graph:

```bash
astrolabe tree ./disc/Gamedata/World/Levels/castle_village --stats --depth 3
```

Inspect mesh candidates before export:

```bash
astrolabe meshes ./disc/Gamedata/World/Levels/castle_village
```

Extract a reversible intermediate level package:

```bash
astrolabe extract-intermediate ./disc/Gamedata/World/Levels/castle_village ./output/castle_village.intermediate
```

Compile that package back to OpenSpace level files:

```bash
astrolabe compile-intermediate ./output/castle_village.intermediate ./output/castle_village.rebuilt
```

The intermediate package keeps the scene as native folders with `node.json` files, SNA payloads as ordered content manifests with typed JSON elements and raw binary siblings, relocation tables as JSON plus preserved encoded leaves, and loose level files as exact binary leaves. If content is unchanged, compilation reuses the original encoded payloads for byte-identical output. If editable content or relocation data changes, Astrolabe writes an uncompressed replacement block with updated OpenSpace checksums.

Generate a Godot project with native scene and mesh resources:

```bash
astrolabe export-godot ./disc/Gamedata/World/Levels/castle_village ./output/castle_village
godot --editor --path ./output/castle_village
```

The Godot export writes:

- `project.godot`
- `<level>.tscn`
- `meshes/*.tres` as `ArrayMesh` resources
- `textures/*` copied from resolved texture references when available

## Commands

| Command | Purpose |
|---------|---------|
| `list <source>` | List files in a mounted or extracted directory. |
| `extract <source> [output]` | Convert supported assets to PNG/WAV. |
| `extract <source> [output] --raw` | Copy raw files from one directory source to another. |
| `extract-intermediate <level-dir> [output-dir]` | Extract a reversible intermediate level package. |
| `compile-intermediate <intermediate-dir> [output-dir]` | Compile an intermediate level package back to OpenSpace files. |
| `textures <cnt-path> [output-dir]` | Extract textures from a CNT container. |
| `cnt <cnt-path>` | Print CNT container metadata and sample entries. |
| `audio <apm-path\|bnm-path\|directory> [output]` | Convert APM files or extract BNM sound banks to WAV. |
| `tree <path> [options]` | Display scene graph roots from a level or game directory. |
| `byte-tree <level-dir> [options]` | Display scene graph with byte coverage analysis. |
| `scene <level-dir> [level-name]` | Print GPT and scene graph diagnostics. |
| `meshes <level-dir> [level-name]` | Scan a level for mesh candidates and print statistics. |
| `export-godot <level-dir> [output-dir]` | Export a Godot project with scene and `ArrayMesh` resources. |
| `scripts <level-dir> [--limit N] [--raw] [-o output-dir]` | Inspect or save AI scripts from Perso data. |
| `analyze <level-dir> [level-name]` | Print low-level SNA, RTB, and geometry diagnostics. |

Additional development commands include `textures-sna`, `debug-gf`, `debug-sna`, and `debug-names`.

## Level Layout

Most level commands expect a directory whose name matches its level files:

```text
disc/Gamedata/World/Levels/castle_village/
  castle_village.sna
  castle_village.gpt
  castle_village.rtb
  castle_village.ptx
```

Level loading can also use shared `Fix.sna`, `Fix.rtb`, `Fix.ptx`, and `fixlvl.rtb` files when they are present in the raw disc copy.

## Project Structure

```text
astrolabe/
├── src/
│   ├── Astrolabe.Cli/           # Command-line interface
│   └── Astrolabe.Core/          # OpenSpace readers, intermediate types, exporters
├── docs/                        # OpenSpace, Hype, and Astrolabe file format documentation
├── notes/                       # Development notes and implementation checklists
├── lib/                         # Build-time dependency submodules
├── reference/                   # Development-only reverse-engineering references
├── disc/                        # Local raw game data (gitignored)
└── output/                      # Converted/exported assets (gitignored)
```

## Core Components

- `Extraction/` handles directory-backed game sources.
- `FileFormats/` contains SNA, GPT, PTX, RTB, CNT, GF, scene graph, relocation, and memory readers.
- `FileFormats/Geometry/` scans OpenSpace geometry.
- `FileFormats/Animation/` reads family, object list, and animation structures.
- `FileFormats/Audio/` converts APM and BNM audio to WAV.
- `FileFormats/AI/` parses AI scripts and converts them to S-expressions.
- `FileFormats/Godot/` writes Godot project, scene, and `ArrayMesh` resource files.

## File Formats

| Extension | Description |
|-----------|-------------|
| `.sna` | Compressed level and fix data blocks. |
| `.gpt` | Global pointer table and scene graph roots. |
| `.ptx` | Texture pointer table. |
| `.rtb` | Relocation table for OpenSpace pointer fixups. |
| `.cnt` | Texture archive container. |
| `.gf` | Individual RLE-encoded texture inside CNT archives. |
| `.apm` | Streaming audio. |
| `.bnm` | Sound bank containing audio entries. |
| `.sda` | Sound data. |

See `docs/` for documentation on CNT, GF, SNA, geometry, relocation tables, lighting, Perso meshes/animation, AI scripts, the intermediate package format, and the broader file catalogue.

The reversible intermediate package format is documented in `docs/intermediate-format.md`. Current intermediate implementation status and type-promotion work are tracked in `notes/intermediate-type-checklist.md`.

## References

Astrolabe keeps several community reverse-engineering projects in `reference/` as development-only material for comparing format notes and implementation details while working on readers and exporters.

The most important references are:

- [Raymap](https://github.com/byvar/raymap)
- [BinarySerializer.OpenSpace](https://github.com/BinarySerializer/BinarySerializer.OpenSpace)
- [OpenSpaceToolbox](https://github.com/raytools/OpenSpaceToolbox)

Astrolabe itself keeps the core library independent of Unity and the reference projects.

## License

CC0 License. See `LICENSE` for details.
