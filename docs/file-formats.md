# OpenSpace Montreal Engine File Formats

This document describes the file formats used by Hype: The Time Quest, which runs on the OpenSpace Montreal engine variant (shared with Tonic Trouble and Rayman 2).

## Overview

The game data is organized into several interconnected file types:

| Extension | Purpose |
|-----------|---------|
| `.sna` | Compressed level/fix data blocks |
| `.cnt` | Texture container archive |
| `.gf` | Individual texture files (inside CNT) |
| `.gpt` | Global pointer table |
| `.ptx` | Texture pointer table |
| `.rtb` | Relocation table for SNA blocks |
| `.rtp` | Relocation table for GPT pointers |
| `.rtt` | Relocation table for textures |
| `.rtv` | Relocation table for video data |
| `.sda` | Sound data |

## Data Flow

```
Game Loading Flow:
├── Load Main Level File
│   ├── .SNA (compressed level geometry/logic)
│   ├── .GPT (global pointer table)
│   ├── .PTX (texture pointer table)
│   └── .RTB, .RTP, .RTT (relocation tables)
│
├── Load Textures
│   └── .CNT (texture container)
│       └── .GF files (individual textures with RLE encoding)
│
└── Relocation & Pointer Fixup
    ├── Read relocation tables
    ├── Resolve all pointers using (module, block_id) pairs
    └── Update virtual offsets to file offsets
```

## Common Data Types

| Type | Size | Description |
|------|------|-------------|
| `byte` | 1 | Unsigned 8-bit integer |
| `int16` | 2 | Signed 16-bit integer (little-endian) |
| `uint16` | 2 | Unsigned 16-bit integer (little-endian) |
| `int32` | 4 | Signed 32-bit integer (little-endian) |
| `uint32` | 4 | Unsigned 32-bit integer (little-endian) |
| `float32` | 4 | IEEE 754 single-precision float |
| `Vector3` | 12 | Three float32 values (X, Y, Z) |
| `Pointer` | 4 | uint32 virtual memory offset |

## Detailed Format Specifications

- [SNA Format](sna-format.md) - Compressed level data
- [CNT Format](cnt-format.md) - Texture container
- [GF Format](gf-format.md) - Texture format
- [Relocation Tables](relocation-tables.md) - Pointer relocation
- [Geometry Format](geometry-format.md) - Mesh and vertex data

## References

- [Raymap](https://github.com/byvar/raymap) - Unity-based OpenSpace viewer
- [BinarySerializer.OpenSpace](https://github.com/BinarySerializer/BinarySerializer.OpenSpace) - Serialization library
