# Astrolabe

A toolkit for extracting and converting game data from **Hype: The Time Quest** (1999) into Godot-compatible formats.

## About

Hype: The Time Quest was developed by Ubisoft (as Playmobil Interactive) for Brandstätter Group using the OpenSpace engine (shared with Tonic Trouble and Rayman 2). This project provides tools to:

1. Extract assets from the original game ISO
2. Convert 3D models to glTF format
3. Generate Godot scene files for maps
4. Capture state graphs and interactions as GDScript

## Legal Notice

This project does not include any copyrighted game assets. Users must provide their own legally obtained copy of the original game ISO. This approach follows the precedent set by projects like OpenMW and Freespace Open.

## Requirements

- .NET 9.0 SDK or later
- A legally obtained copy of Hype: The Time Quest (ISO format)

## Building

```bash
dotnet build
```

## Usage

### List files in ISO

```bash
dotnet run --project src/Astrolabe.Cli -- list path/to/hype.iso
```

### Extract game data from ISO

```bash
# Extract all files
dotnet run --project src/Astrolabe.Cli -- extract path/to/hype.iso ./extracted

# Extract only game data
dotnet run --project src/Astrolabe.Cli -- extract path/to/hype.iso ./extracted --pattern "Gamedata/"

# Extract specific file types
dotnet run --project src/Astrolabe.Cli -- extract path/to/hype.iso ./extracted --pattern "*.sna"
```

## Project Structure

```
astrolabe/
├── src/
│   ├── Astrolabe.Core/          # Core library for file format parsing
│   │   ├── Extraction/          # ISO extraction utilities
│   │   ├── FileFormats/         # OpenSpace file format readers
│   │   └── Export/              # glTF and Godot scene exporters
│   └── Astrolabe.Cli/           # Command-line interface
├── reference/
│   └── raymap/                  # Raymap Unity project (git submodule)
├── docs/                        # File format documentation
└── extracted/                   # Extracted game data (gitignored)
```

## File Formats

Hype uses the OpenSpace engine's Montreal variant. Key file types:

| Extension | Description |
|-----------|-------------|
| `.sna`    | Compressed level/fix data (LZO compression) |
| `.cnt`    | Texture container (index of GF textures) |
| `.gpt`    | Game pointer table |
| `.ptx`    | Pointer table extension |
| `.rtb`    | Runtime binary data |
| `.rtp`    | Runtime pointer data |
| `.rtt`    | Runtime texture data |
| `.sda`    | Sound data |

See the `docs/` directory for detailed format specifications.

## Acknowledgments

This project heavily references [Raymap](https://github.com/byvar/raymap) by the Rayman community, which provides invaluable documentation of the OpenSpace engine formats.

## License

MIT License - See LICENSE file for details.
