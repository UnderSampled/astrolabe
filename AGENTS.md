# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Astrolabe reads and converts game data from **Hype: The Time Quest** (1999) into **Rete** (canonical level representation), OpenSpace level files, and Godot project assets. The game uses the OpenSpace Montreal engine (shared with Rayman 2 and Tonic Trouble).

**Rete refactor:** read [`plan.md`](plan.md), [`docs/rete-format.md`](docs/rete-format.md), [`notes/rete-implementation.md`](notes/rete-implementation.md) before changing import/export architecture.

## Build Commands

```bash
# Build solution
dotnet build

# Run CLI
dotnet run --project src/Astrolabe.Cli -- <command> [args]

# Common commands
dotnet run --project src/Astrolabe.Cli -- list ./disc
dotnet run --project src/Astrolabe.Cli -- extract ./disc ./output                 # Convert to PNG/WAV
dotnet run --project src/Astrolabe.Cli -- extract /mnt/hype ./disc --raw          # Copy raw game files
dotnet run --project src/Astrolabe.Cli -- export-godot ./disc/Gamedata/World/Levels/LEVELNAME
dotnet run --project src/Astrolabe.Cli -- tree ./disc/Gamedata/World/Levels/LEVELNAME
dotnet run --project src/Astrolabe.Cli -- byte-tree ./disc/Gamedata/World/Levels/LEVELNAME
```

## Architecture

### Core Components

- **Astrolabe.Cli** - Command-line interface with commands for extraction and export
- **Astrolabe.Core** - Core library (no Unity dependencies)
  - `Extraction/` - Directory-backed game sources
  - `FileFormats/` - OpenSpace format readers
  - `FileFormats/Animation/` - Family/character reading and animation types
  - `FileFormats/Geometry/` - Mesh scanning
  - `FileFormats/Godot/` - TSCN scene and ArrayMesh resource generation
  - `FileFormats/Materials/` - Visual/game material parsing
  - `FileFormats/AI/` - AI script parsing and S-expression conversion
  - `Intermediate/` (→ `Rete/`) - Rete package import/export; becoming `Serialization/` + `Rete/`

### Data Pipeline

```
disc files → import-openspace → Rete package (JSON + bin)
          → export-openspace → OpenSpace level dir (cmp-validated)
          → export-godot     → Godot project
          → extract          → PNG/WAV assets
```

CLI today: `extract-intermediate` / `compile-intermediate` (becoming `import-openspace` / `export-openspace`).

### Key Classes

- **LevelLoader** - Loads SNA blocks + relocation tables, provides virtual memory access
- **MemoryContext** - Pointer resolution using RTB relocation data
- **MeshScanner** - Finds GeometricObject structures in SNA blocks by pattern matching
- **SuperObjectReader** - Parses scene graph hierarchy from GPT
- **GptReader** - Parses GPT header fields (scene roots, spawnable persos, families, object type tables)
- **FamilyReader** - Discovers character Families and their mesh/animation data
- **ByteRangeTracker** - Tracks which byte ranges the parser accounts for in SNA data
- **TrackingSuperObjectReader** - SuperObjectReader wrapper that records byte coverage
- **GodotExporter** - Generates Godot TSCN scene files
- **GodotMeshExporter** - Writes Godot-native ArrayMesh `.tres` resources

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

Shared data (characters like Hype, common textures) lives in `Fix.sna`/`Fix.rtb` at `Gamedata/World/Levels/`. The `fixlvl.rtb` relocation table links level data to Fix data. On Rete import, Fix is extracted once to `output/fix/` alongside `output/{level}/`; level records reference Fix via relative URIs (`../fix/...`). OpenSpace export regenerates relocation tables (target); they are not stored in Rete packages.

## Dependencies

- **lib/BinarySerializer.OpenSpace** - Submodule for OpenSpace type definitions
- **reference/raymap** - Reference Unity implementation (read-only, for documentation)
- **reference/OpenSpaceToolbox** - OpenSpace reverse-engineering toolbox (read-only, for reference)

## Testing Exports

```bash
# Export full Godot scene and native mesh resources
dotnet run --project src/Astrolabe.Cli -- export-godot ./disc/Gamedata/World/Levels/castle_village output/castle_village
godot --editor --path output/castle_village
```

- Mount or pre-extract the original game disc before running Astrolabe
- Raw game files (for level loading) go in ./disc
- Converted assets (PNG/WAV) go in ./output
