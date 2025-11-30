# OpenSpace File Format Documentation

This directory contains detailed documentation of the OpenSpace engine file formats used by Hype: The Time Quest and related games. These formats are reverse-engineered primarily from the [Raymap](https://github.com/byvar/raymap) project.

## Overview

Hype: The Time Quest uses the **Montreal** variant of the OpenSpace engine (also used by Largo Winch). This variant has some differences from the Rayman 2/3 versions:

- LZO compression for data blocks
- XOR masking/encryption for certain files
- 1-byte string length prefixes (vs 2-byte in R2/R3)
- Different type codes for SuperObjects
- Palette-based and 16-bit textures (1555, 4444, 565 formats)

## File Types

### Configuration
- [DSB/DSC](./DSB.md) - Game configuration and directory paths

### Data Containers
- [SNA](./SNA.md) - Serialized data blocks (main game data)
- [LVL](./LVL.md) - Level container files
- [Relocation Tables](./RELOCATION.md) - Pointer relocation system (RTB, RTP, RTT, etc.)

### Textures
- [GF](./GF.md) - OpenSpace texture format

### Geometry
- [Geometry](./GEOMETRY.md) - 3D mesh and visual data structures

### Scene Hierarchy
- [SuperObject](./SUPEROBJECT.md) - Scene graph and world hierarchy

### AI/Behavior
- [AI System](./AI.md) - Brain, Mind, Behaviors, and Scripts

## Encryption/Compression

The Montreal engine uses several data protection mechanisms:

1. **XOR Masking** - Applied to DSB, relocation tables, and pointer files
2. **LZO Compression** - Applied to SNA data blocks and relocation table entries

See [Encryption](./ENCRYPTION.md) for implementation details.

## Data Flow

```
ISO
 └─ GameData/
     ├─ GAME.DSC          → Configuration (paths, levels list)
     ├─ Fix/
     │   ├─ fix.sna       → Global data (compressed + masked)
     │   ├─ fix.rtb       → Relocation table
     │   └─ fix.gpt       → Global pointer table
     └─ World/Levels/
         └─ <LevelName>/
             ├─ <level>.sna  → Level data
             ├─ <level>.rtb  → Level relocation
             ├─ <level>.gpt  → Level pointers
             └─ <level>.ptx  → Textures
```
