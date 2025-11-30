# DSB/DSC Format

**Extension:** `.dsb`, `.dsc`
**Purpose:** Game configuration file containing directory paths, level list, and startup settings
**Encryption:** XOR number masking (mask read from first 4 bytes)

## Overview

The DSB (Data Structure Binary) file is the entry point for loading game data. It defines:
- Directory paths for all game assets
- List of available levels
- Bigfile locations for textures and vignettes
- Initial level and vignette settings

Hype: The Time Quest uses `GAME.DSC` as its configuration file.

## Montreal Engine Format

The Montreal variant (used by Hype) has a simpler structure than Rayman 2/3.

### File Structure

```
Offset  Type      Description
------  ----      -----------
0x00    u32       XOR mask (for decryption)
--- After decryption ---
0x00    str8      DLL data path
0x??    str8      Game data path
0x??    str8      World data path
0x??    str8      Levels data path
0x??    str8      Sound data path
0x??    str8      Save game data path
0x??    str8      Texture data path (read twice)
0x??    str8      Texture data path (duplicate)
0x??    str8      Vignettes data path
0x??    str8      Options data path
0x??    str8      Bigfile vignettes path
0x??    str8      Bigfile textures path
0x??    u32       Unknown (typically 10000)
0x??    u16       Unknown (typically 0)
0x??    str8      Default config filename ("Default.cfg")
0x??    str8      Current config filename ("Current.cfg")
0x??    str8      First level name (e.g., "manoir")
```

### String Format (Montreal)

Montreal uses 1-byte length prefix for strings:
```
Offset  Type    Description
------  ----    -----------
0x00    u8      String length (N)
0x01    char[N] String data (not null-terminated)
```

The string content has `GameData\` prefix stripped during reading.

## Rayman 2/3 Format (Reference)

For comparison, Rayman 2/3 use a section-based format:

### Section Types

| ID | Name | Description |
|----|------|-------------|
| 0 | Memory | Memory allocation descriptors |
| 30 | LevelName | Level list |
| 40 | Directories | Asset directory paths |
| 32 | Random | Random table configuration |
| 64 | BigFile | Bigfile paths |
| 70 | Vignette | Loading screen settings |
| 90 | Level | Level-specific settings |
| 100 | GameOption | Game configuration options |
| 110 | Input | Input device settings |
| 120 | Unknown | INO device settings |

### Section Format

```
u32     Section type ID
[data]  Section-specific data
u32     End marker (0xFFFF)
```

### Directory Section (Type 40)

```
u32     Entry type (41-49, 58-63)
str16   Path string

Entry types:
41 = DLL path
42 = Game data path
43 = World data path
44 = Levels data path
45 = Sound data path
46 = Save game path
48 = Vignettes path
49 = Options path
```

### Level Name Section (Type 30)

```
u32     First level index
u32     Entry type (31 = level name)
str16   Level name
...     (repeat for each level)
u32     End marker (0xFFFF)
```

## Python Implementation

```python
from dataclasses import dataclass
from pathlib import Path

@dataclass
class DSBConfig:
    """Parsed DSB/DSC configuration."""
    dll_path: str
    game_data_path: str
    world_path: str
    levels_path: str
    sound_path: str
    save_path: str
    texture_path: str
    vignettes_path: str
    options_path: str
    bigfile_vignettes: str
    bigfile_textures: str
    first_level: str
    levels: list[str]

def parse_dsb_montreal(data: bytes) -> DSBConfig:
    """Parse Montreal-engine DSB file (already decrypted)."""
    pos = 0

    def read_str8() -> str:
        nonlocal pos
        length = data[pos]
        pos += 1
        value = data[pos:pos + length].decode('latin-1')
        pos += length
        # Strip GameData\ prefix if present
        if value.startswith('GameData\\'):
            value = value[9:]
        return value

    return DSBConfig(
        dll_path=read_str8(),
        game_data_path=read_str8(),
        world_path=read_str8(),
        levels_path=read_str8(),
        sound_path=read_str8(),
        save_path=read_str8(),
        texture_path=read_str8(),
        texture_path_dup=read_str8(),  # Read twice
        vignettes_path=read_str8(),
        options_path=read_str8(),
        bigfile_vignettes=read_str8(),
        bigfile_textures=read_str8(),
        # Skip: u32 (10000), u16 (0)
        # Skip: default.cfg, current.cfg strings
        first_level=...,  # Read after skipping
        levels=[]  # Montreal doesn't have level list in DSB
    )
```

## Typical Hype Directory Structure

Based on DSB configuration:
```
GameData/
├── GAME.DSC           # Configuration
├── Fix/               # Global/shared data
├── World/
│   ├── Levels/        # Level data
│   ├── Graphics/
│   │   └── Textures/  # Texture files
│   └── Sound/         # Audio files
├── Vignette/          # Loading screens
├── Options/           # Settings/input config
└── SaveGame/          # Save files
```

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/DSB.cs`
- Engine settings: `reference/raymap/Assets/Scripts/OpenSpace/General/Settings.cs`
