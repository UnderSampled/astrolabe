# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Astrolabe extracts and converts game data from **Hype: The Time Quest** (1999) into modern formats (glTF, Godot scenes). The game uses the OpenSpace Montreal engine (shared with Rayman 2 and Tonic Trouble).

## Build Commands

```bash
# Build solution
dotnet build

# Run CLI
dotnet run --project src/Astrolabe.Cli -- <command> [args]

# Common commands
dotnet run --project src/Astrolabe.Cli -- list path/to/hype.iso
dotnet run --project src/Astrolabe.Cli -- extract path/to/hype.iso ./output        # Convert to PNG/WAV
dotnet run --project src/Astrolabe.Cli -- extract path/to/hype.iso ./disc --raw    # Raw game files
dotnet run --project src/Astrolabe.Cli -- export-gltf ./disc/Gamedata/World/Levels/LEVELNAME
dotnet run --project src/Astrolabe.Cli -- export-families ./disc/Gamedata/World/Levels/LEVELNAME
dotnet run --project src/Astrolabe.Cli -- export-godot ./disc/Gamedata/World/Levels/LEVELNAME
dotnet run --project src/Astrolabe.Cli -- tree ./disc/Gamedata/World/Levels/LEVELNAME
dotnet run --project src/Astrolabe.Cli -- byte-tree ./disc/Gamedata/World/Levels/LEVELNAME
```

## Architecture

### Core Components

- **Astrolabe.Cli** - Command-line interface with commands for extraction and export
- **Astrolabe.Core** - Core library (no Unity dependencies)
  - `Extraction/` - ISO9660 extraction via DiscUtils
  - `FileFormats/` - OpenSpace format readers
  - `FileFormats/Animation/` - Family/character reading and animation types
  - `FileFormats/Geometry/` - Mesh scanning, glTF export, Family mesh export via SharpGLTF
  - `FileFormats/Godot/` - TSCN scene generation
  - `FileFormats/Materials/` - Visual/game material parsing
  - `FileFormats/AI/` - AI script parsing and S-expression conversion

### Data Pipeline

```
ISO/disc → Extract → PNG textures, WAV audio (./output)
        → Extract --raw → SNA/GPT/PTX/RTB files (./disc) → Load Level → Export glTF/TSCN
                                                          → Load Level → Export Families (characters)
                                                          → Load Level → tree / byte-tree (analysis)
```

### Key Classes

- **LevelLoader** - Loads SNA blocks + relocation tables, provides virtual memory access
- **MemoryContext** - Pointer resolution using RTB relocation data
- **MeshScanner** - Finds GeometricObject structures in SNA blocks by pattern matching
- **SuperObjectReader** - Parses scene graph hierarchy from GPT
- **GptReader** - Parses GPT header fields (scene roots, spawnable persos, families, object type tables)
- **FamilyReader** - Discovers character Families and their mesh/animation data
- **FamilyExporter** - Exports Family meshes with textures to glTF
- **ByteRangeTracker** - Tracks which byte ranges the parser accounts for in SNA data
- **TrackingSuperObjectReader** - SuperObjectReader wrapper that records byte coverage
- **GltfExporter** - Exports mesh data to GLB format
- **GodotExporter** - Generates Godot TSCN scene files

### OpenSpace File Formats

| File | Purpose |
|------|---------|
| `.sna` | LZO-compressed level data blocks |
| `.gpt` | Global pointer table (scene graph roots) |
| `.ptx` | Texture pointer table |
| `.rtb` | Relocation table (pointer fixups between SNA blocks) |
| `.cnt` | Texture archive container |
| `.gf` | Individual texture (RLE-encoded, inside CNT) |

See `docs/` for detailed format specifications.

### Pointer System

OpenSpace uses virtual memory addresses resolved via relocation tables. The RTB file maps (module, block_id) pairs to pointer targets. `MemoryContext.GetPointerAt()` resolves these at runtime.

### Fix Data

Shared data (characters like Hype, common textures) lives in `Fix.sna`/`Fix.rtb` at `Gamedata/World/Levels/`. The `fixlvl.rtb` relocation table links level data to Fix data. LevelLoader merges Fix blocks so that cross-references (e.g. a level's spawnable perso pointing into Fix for its mesh) resolve correctly.

## Dependencies

- **lib/BinarySerializer.OpenSpace** - Submodule for OpenSpace type definitions
- **reference/raymap** - Reference Unity implementation (read-only, for documentation)
- **reference/OpenSpaceToolbox** - OpenSpace reverse-engineering toolbox (read-only, for reference)

## Testing Exports

```bash
# Export meshes and view in Blender
dotnet run --project src/Astrolabe.Cli -- export-gltf ./disc/Gamedata/World/Levels/castle_village
flatpak run org.blender.Blender output/castle_village_meshes/mesh_*.glb

# Export character families (meshes + animations)
dotnet run --project src/Astrolabe.Cli -- export-families ./disc/Gamedata/World/Levels/castle_village

# Export full Godot scene
dotnet run --project src/Astrolabe.Cli -- export-godot ./disc/Gamedata/World/Levels/castle_village output/castle_village
godot --editor --path output/castle_village
```

- The original game disc ISO goes at ./hype.iso
- Raw game files (for level loading) go in ./disc
- Converted assets (PNG/WAV) go in ./output
