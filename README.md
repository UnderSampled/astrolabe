# Astrolabe

Astrolabe is a .NET toolkit for extracting and converting data from **Hype: The Time Quest** (1999) into formats that are useful for inspection, reverse engineering, and Godot-based projects.

The game uses Ubisoft's OpenSpace Montreal engine, also used by titles such as *Rayman 2* and *Tonic Trouble*. Astrolabe focuses on the Hype data pipeline:

- Extract files from the original game ISO or an already extracted disc directory.
- Convert texture containers to PNG and audio files to WAV.
- Read OpenSpace level files, scene graphs, relocation tables, meshes, materials, families, and scripts.
- Export level meshes and character families to glTF/GLB.
- Generate a Godot project and `.tscn` scene from a level.

## Status

This is an active reverse-engineering tool. Extraction and many inspection commands are useful today, while mesh, family, script, and Godot scene exports are still best-effort and may need level-specific fixes as more OpenSpace structures are documented.

## Legal Notice

This repository does not include copyrighted game assets. You must provide your own legally obtained copy of *Hype: The Time Quest* in ISO or extracted disc form.

## Requirements

- .NET 9.0 SDK or later
- Git submodules initialized
- A legally obtained Hype ISO or extracted disc directory
- Optional: Blender for inspecting exported GLB files
- Optional: Godot for opening generated scenes

## Dependencies

Astrolabe uses the `lib/` submodules as build-time dependencies:

- [BinarySerializer.OpenSpace](https://github.com/BinarySerializer/BinarySerializer.OpenSpace), at `lib/BinarySerializer.OpenSpace`, is referenced by `Astrolabe.Core` for OpenSpace type definitions and serialization helpers.
- [BinarySerializer](https://github.com/BinarySerializer/BinarySerializer), at `lib/BinarySerializer`, is required by `BinarySerializer.OpenSpace`.

These are different from the `reference/` submodules, which are development-only references and are not loaded, called, or required by the CLI or core library at runtime.

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

## Typical Workflow

List the contents of an ISO or extracted disc directory:

```bash
astrolabe list hype.iso
```

Extract converted assets. This writes PNG textures and WAV audio where supported:

```bash
astrolabe extract hype.iso ./output
```

Extract raw game files for level loading and export commands:

```bash
astrolabe extract hype.iso ./disc --raw
```

Inspect a level scene graph:

```bash
astrolabe tree ./disc/Gamedata/World/Levels/castle_village --stats --depth 3
```

Export level meshes to GLB files:

```bash
astrolabe export-gltf ./disc/Gamedata/World/Levels/castle_village
```

Export character families from a level:

```bash
astrolabe export-families ./disc/Gamedata/World/Levels/castle_village
```

Generate a Godot project and scene:

```bash
astrolabe export-godot ./disc/Gamedata/World/Levels/castle_village ./output/castle_village
godot --editor --path ./output/castle_village
```

## Commands

| Command | Purpose |
|---------|---------|
| `list <source>` | List files in an ISO or extracted directory. |
| `extract <source> [output]` | Convert supported assets to PNG/WAV. |
| `extract <source> [output] --raw` | Copy raw game files for level parsing. |
| `textures <cnt-path> [output-dir]` | Extract textures from a CNT container. |
| `cnt <cnt-path>` | Print CNT container metadata and sample entries. |
| `audio <apm-path\|bnm-path\|directory> [output]` | Convert APM files or extract BNM sound banks to WAV. |
| `tree <path> [options]` | Display scene graph roots from a level, game directory, or ISO. |
| `byte-tree <level-dir> [options]` | Display scene graph with byte coverage analysis. |
| `scene <level-dir> [level-name]` | Print GPT and scene graph diagnostics. |
| `meshes <level-dir> [level-name]` | Scan a level for mesh candidates and print statistics. |
| `export-gltf <level-dir> [level-name] [output.glb]` | Export discovered level meshes as GLB files. |
| `export-families <level-dir> [output-dir]` | Export character family meshes and animation data as GLB files. |
| `export-godot <level-dir> [output-dir]` | Export a Godot project with scene and mesh assets. |
| `scripts <level-dir> [--limit N] [--raw] [-o output-dir]` | Inspect or save AI scripts from Perso data. |
| `analyze <level-dir> [level-name]` | Print low-level SNA, RTB, and geometry diagnostics. |

Additional development commands include `textures-sna`, `debug-gf`, `debug-sna`, and `debug-names`.

Most level commands expect a directory whose name matches its level files, for example:

```text
disc/Gamedata/World/Levels/castle_village/
  castle_village.sna
  castle_village.gpt
  castle_village.rtb
  castle_village.ptx
```

Family export can also use shared `Fix.sna`, `Fix.rtb`, `Fix.ptx`, and `fixlvl.rtb` files when they are present in the raw disc extraction.

## Project Structure

```text
astrolabe/
├── src/
│   ├── Astrolabe.Cli/           # Command-line interface
│   └── Astrolabe.Core/          # Extraction, format readers, exporters
├── docs/                        # OpenSpace and Hype file format notes
├── lib/                         # Build-time dependency submodules
├── reference/                   # Development-only reverse-engineering references
├── scripts/                     # Helper scripts
├── tools/                       # External inspection helpers
├── disc/                        # Raw extracted game data (gitignored)
└── output/                      # Converted/exported assets (gitignored)
```

## Core Components

- `Extraction/` handles ISO9660 and directory-backed game sources.
- `FileFormats/` contains SNA, GPT, PTX, RTB, CNT, GF, scene graph, relocation, and memory readers.
- `FileFormats/Geometry/` scans OpenSpace geometry and exports GLB files through SharpGLTF.
- `FileFormats/Animation/` reads family, object list, and animation structures.
- `FileFormats/Audio/` converts APM and BNM audio to WAV.
- `FileFormats/AI/` parses AI scripts and converts them to S-expressions.
- `FileFormats/Godot/` writes Godot project and scene files.

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

See `docs/` for current notes on CNT, GF, SNA, geometry, relocation tables, lighting, Perso meshes/animation, AI scripts, and the broader file catalogue.

## References

Astrolabe keeps several community reverse-engineering projects in `reference/` as development-only material. The CLI and core library do not load, call, or require these projects at runtime; they are only there so contributors can compare format notes and implementation details while working on readers and exporters.

The most important references are:

- [Raymap](https://github.com/byvar/raymap)
- [BinarySerializer.OpenSpace](https://github.com/BinarySerializer/BinarySerializer.OpenSpace)
- [OpenSpaceToolbox](https://github.com/raytools/OpenSpaceToolbox)

Astrolabe itself keeps the core library independent of Unity and the reference projects.

## License

CC0 License. See `LICENSE` for details.
