# Astrolabe Architecture

## Overview

Astrolabe is a pipeline for converting Hype: The Time Quest assets into Godot-compatible formats. The process flows through several stages:

```
ISO Image → Extraction → Parsing → Conversion → Godot Assets
```

## Pipeline Stages

### 1. ISO Extraction (`astrolabe.iso`)

Extracts raw files from the game disc image using `pycdlib`. This produces the raw game data files in their original format.

**Input:** Game ISO file
**Output:** Raw game files (`.cnt`, `.sna`, `.gf`, etc.)

### 2. OpenSpace Parsing (`astrolabe.openspace`)

Parses the proprietary OpenSpace engine formats. This module is largely ported from the [Raymap](https://github.com/byvar/raymap) Unity project.

Key formats to parse:
- **SNA files** - Level/world data snapshots
- **CNT files** - Game data containers
- **GF files** - Texture data
- **PO/IPO files** - 3D geometry (physical/instanced physical objects)

**Input:** Raw game files
**Output:** Python data structures representing game assets

### 3. Converters (`astrolabe.converters`)

Converts parsed OpenSpace data into standard formats.

- **Mesh Converter** - Converts PO/IPO geometry to glTF
- **Texture Converter** - Converts GF textures to PNG
- **Animation Converter** - Converts animation data to glTF animations

**Input:** Parsed OpenSpace data
**Output:** glTF files, PNG textures

### 4. Godot Generation (`astrolabe.godot`)

Generates Godot-specific files for scenes and interactivity.

- **Scene Generator** - Creates `.tscn` files for maps/levels
- **Script Generator** - Creates `.gd` files for state machines and interactions

**Input:** Converted assets + parsed game logic
**Output:** Godot project files

## Key Data Structures

### From OpenSpace (to be implemented)

```python
@dataclass
class Mesh:
    vertices: list[Vector3]
    normals: list[Vector3]
    uvs: list[Vector2]
    triangles: list[tuple[int, int, int]]
    material_id: int

@dataclass
class Texture:
    width: int
    height: int
    data: bytes
    format: TextureFormat

@dataclass
class GameObject:
    name: str
    position: Vector3
    rotation: Quaternion
    scale: Vector3
    mesh: Mesh | None
    children: list[GameObject]
```

## Reference Material

The `reference/raymap` submodule contains the original Raymap Unity project. Key files for understanding OpenSpace formats:

- `Assets/Scripts/OpenSpace/` - Format parsers
- `Assets/Scripts/OpenSpace/Loader/` - Data loading logic
- `Assets/Scripts/OpenSpace/Object/` - Game object structures

## File Naming Conventions

- OpenSpace parser modules: Named after the format they parse (e.g., `sna.py`, `cnt.py`)
- Converters: Named with `_converter` suffix (e.g., `mesh_converter.py`)
- Godot generators: Named with `_generator` suffix (e.g., `scene_generator.py`)
