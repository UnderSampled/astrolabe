# Astrolabe

A toolkit for extracting and converting game data from **Hype: The Time Quest** (1999) into Godot-compatible formats.

## About

Hype: The Time Quest was developed by Ubisoft (as Playmobil Interactive) for Brandstätter Group using the OpenSpace engine (shared with Tonic Trouble and Rayman 2). This project provides tools to:

1. Extract assets from the original game ISO
2. Parse OpenSpace engine file formats (SNA, GF, CNT, etc.)
3. Convert 3D models to glTF format
4. Generate Godot scene files for maps
5. Capture state graphs and interactions as GDScript (planned)

## Legal Notice

This project does not include any copyrighted game assets. Users must provide their own legally obtained copy of the original game ISO. This approach follows the precedent set by projects like OpenMW and Freespace Open.

## Requirements

- Python 3.10+
- A legally obtained copy of Hype: The Time Quest (ISO format)
- 7z (p7zip) for ISO extraction
- Blender 4.0+ (optional, for viewing extracted models)

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/astrolabe.git
cd astrolabe

# Install in development mode
pip install -e ".[dev]"
```

## Usage

### Quick Start

```bash
# Extract and convert assets from an ISO
astrolabe extract /path/to/hype.iso --output ./output

# Convert a specific level to glTF
astrolabe convert-level brigand --output ./output/levels
```

### Command Reference

```bash
# List available levels in an extracted game
astrolabe list-levels ./extracted

# Export a single mesh to glTF
astrolabe export-mesh ./extracted/World/Levels/brigand/brigand.sna --output ./mesh.gltf

# Extract textures from CNT archive
astrolabe extract-textures ./extracted/Textures.cnt --output ./textures
```

## Project Structure

```
astrolabe/
├── src/astrolabe/
│   ├── __init__.py
│   ├── cli.py                    # Command-line interface
│   ├── openspace/                # OpenSpace engine format parsers
│   │   ├── __init__.py
│   │   ├── sna.py                # SNA level file parser
│   │   ├── relocation.py         # RTB/RTP relocation table parser
│   │   ├── geometry.py           # Mesh/geometry parsing
│   │   ├── pointer.py            # Pointer relocation system
│   │   ├── encryption.py         # XOR encryption handling
│   │   └── lzo.py                # LZO decompression
│   ├── iso/                      # ISO extraction utilities
│   │   ├── __init__.py
│   │   └── extractor.py
│   ├── converters/               # Format converters
│   │   ├── __init__.py
│   │   └── gltf_export.py        # glTF 2.0 exporter
│   └── godot/                    # Godot scene generators (planned)
│       └── __init__.py
├── reference/
│   └── raymap/                   # Raymap Unity project (git submodule)
├── tests/
│   └── test_sna.py
├── docs/
│   └── formats/                  # File format documentation
│       ├── README.md
│       ├── SNA.md
│       ├── GF.md
│       └── ...
├── output/                       # Default output directory (gitignored)
├── pyproject.toml
└── README.md
```

## File Formats

The OpenSpace engine uses several proprietary file formats:

| Format | Extension | Description |
|--------|-----------|-------------|
| SNA | `.sna` | Main level data (geometry, objects, scripts) |
| RTB | `.rtb` | Relocation table for SNA pointers |
| GPT | `.gpt` | Global pointer table |
| PTX | `.ptx` | Texture pointer table |
| CNT | `.cnt` | Archive container (textures, vignettes) |
| GF | `.gf` | Texture image format (RLE compressed) |
| DSB | `.dsb` | Game configuration and level list |

See [docs/formats/](docs/formats/) for detailed format documentation.

## Development

### Running Tests

```bash
pytest tests/
```

### Architecture

The codebase is organized into three main layers:

1. **Parsers** (`openspace/`): Low-level binary format parsers that read game data
2. **Converters** (`converters/`): Transform parsed data into standard formats (glTF)
3. **Generators** (`godot/`): Create Godot-specific scene files and scripts

### Contributing

Contributions are welcome! Please see the format documentation in `docs/formats/` before working on parsers.

## Acknowledgments

This project heavily references [Raymap](https://github.com/byvar/raymap) by the Rayman community, which provides invaluable documentation of the OpenSpace engine formats through its Unity implementation.

## License

MIT License - See LICENSE file for details.
