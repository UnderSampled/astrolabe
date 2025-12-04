# Hype: The Time Quest - Complete File Format Catalogue

This document catalogues all file formats found in the extracted game data.

## File Extension Summary

| Extension | Count | Category | Format | Purpose |
|-----------|-------|----------|--------|---------|
| `.bnm` | 199 | Audio | Binary | Sound bank (bundles APM samples) |
| `.csb` | 85 | Audio | Text script | Sound event definitions (Ubi Sound Studio) |
| `.rtg` | 75 | Relocation | Binary | Relocation table (language-specific SNA blocks) |
| `.rtb` | 75 | Relocation | Binary | Relocation table (main SNA block pointers) |
| `.bin` | 39 | Data | Binary | Polygon/collision data (pgb.bin) |
| `.apm` | 39 | Audio | Binary | Audio samples (IMA-ADPCM) |
| `.snd` | 38 | Audio | Binary | Sound reference pointers |
| `.sna` | 38 | Level | Binary+LZO | Compressed level data blocks |
| `.sda` | 38 | Audio | Binary | Sound data pointers |
| `.rtt` | 38 | Relocation | Binary | Relocation table (texture pointers) |
| `.rts` | 38 | Relocation | Binary | Relocation table (sound data) |
| `.rtp` | 38 | Relocation | Binary | Relocation table (GPT pointers) |
| `.rtd` | 38 | Relocation | Binary | Relocation table (dialog data) |
| `.ptx` | 38 | Texture | Binary | Texture pointer table |
| `.lng` | 38 | Data | Binary | Language-specific level data |
| `.gpt` | 38 | Level | Binary | Global pointer table (scene graph roots) |
| `.dlg` | 38 | Text | Binary | Dialog data pointers |
| `.cnt` | 3 | Texture | Binary | Texture archive container |
| `.avi` | 3 | Video | AVI | Video cutscenes |
| `.vgm` | 1 | Config | Text | Empty/minimal (video game music?) |
| `.TRN` | 1 | Data | Binary | Translation/lookup table |
| `.sif` | 1 | Audio | Text script | Sound interaction file (materials) |
| `.mem` | 1 | Config | Text script | Memory allocation configuration |
| `.lcb` | 1 | Audio | Text script | Sound event group list |
| `.ini` | 1 | Config | Text script | Game configuration (credits) |
| `.cfg` | 1 | Config | Binary (XOR) | Game options (encrypted) |

---

## Level Data Files

### `.sna` - Snapshot/Level Data
**Format:** Binary (LZO compressed blocks)
**Purpose:** Contains all level geometry, objects, AI, and logic data

```
SNA File
├── Block Header (per block)
│   ├── module: byte          # Module ID (subsystem)
│   ├── id: byte              # Block ID within module
│   ├── baseInMemory: int32   # Virtual memory address (-1 if not loaded)
│   ├── size: uint32          # Uncompressed size
│   └── Compressed Data
│       ├── isCompressed: uint32    # 1=LZO, 0=raw
│       ├── compressedSize: uint32
│       ├── compressedChecksum: uint32
│       ├── decompressedSize: uint32
│       ├── decompressedChecksum: uint32
│       └── data: byte[]      # Block data
└── (repeats for all blocks)
```

**Sub-structures within SNA blocks:**
- GeometricObject (mesh data)
- SuperObject (scene graph nodes)
- VisualMaterial / GameMaterial
- AI scripts and behaviors
- Collision data

### `.gpt` - Global Pointer Table
**Format:** Binary
**Purpose:** Root pointers for scene graph traversal

```
GPT File
├── actualWorldPointer: uint32    # Main world scene graph
├── dynamicWorldPointer: uint32   # Dynamic objects
├── fatherSectorPointer: uint32   # Sector hierarchy
└── (additional root pointers)
```

### `.ptx` - Texture Pointer Table
**Format:** Binary
**Purpose:** Maps texture indices to texture info structures

```
PTX File
└── Array of pointers to TextureInfo structures in SNA
    └── TextureInfo
        ├── textureAddress: uint32
        └── textureName: string (null-terminated)
```

---

## Relocation Tables

All relocation tables share a similar structure:

```
Relocation Table
├── blockCount: byte
├── reserved: uint32 (Montreal+)
└── For each block:
    ├── module: byte
    ├── id: byte
    ├── pointerCount: uint32
    └── Compressed Pointer Data (if pointerCount > 0)
        └── For each pointer:
            ├── offsetInMemory: uint32
            ├── targetModule: byte
            ├── targetId: byte
            ├── byte6: byte (unknown)
            └── byte7: byte (unknown)
```

| File | Type ID | Purpose |
|------|---------|---------|
| `.rtb` | 0 | Main SNA block relocations |
| `.rtp` | 1 | GPT pointer file relocations |
| `.rts` | 2 | Sound data relocations |
| `.rtt` | 3 | PTX/Texture file relocations |
| `.rtd` | 5 | Dialog/language data relocations |
| `.rtg` | 6 | Language-specific SNA blocks |
| `.rtv` | 7 | Video data relocations |

---

## Texture Files

### `.cnt` - Container Archive
**Format:** Binary archive
**Purpose:** Contains multiple GF texture files

```
CNT File
├── Header
│   ├── directoryCount: int32
│   ├── fileCount: int32
│   ├── isXor: byte
│   ├── isChecksum: byte
│   └── xorKey: byte
├── Directory List
│   └── For each directory:
│       ├── stringLength: int32
│       └── directoryPath: string (XOR encrypted)
└── File Entries
    └── For each file:
        ├── dirIndex: int32
        ├── filenameLength: int32
        ├── filename: string (XOR encrypted)
        ├── fileXorKey: byte[4]
        ├── fileChecksum: uint32
        ├── filePointer: int32
        └── fileSize: int32
```

**Contains:**
- `.gf` texture files (RLE-encoded images)

### `.gf` - Texture (inside CNT)
**Format:** Binary (RLE compressed)
**Purpose:** Individual texture with palette or direct color

See [gf-format.md](gf-format.md) for detailed structure.

---

## Audio Files

### `.apm` - Audio Sample
**Format:** Binary (Ubisoft IMA-ADPCM variant)
**Purpose:** Compressed audio data
**Decoder DLL:** `APMmxBVR.dll` (ACP Sound Engine M5.7.5)

```
APM File (100 bytes header + data)
├── Header
│   ├── 0x00: format_tag: uint16     # 0x2000 = Ubisoft ADPCM
│   ├── 0x02: channels: uint16       # 1 = mono, 2 = stereo
│   ├── 0x04: sample_rate: uint32    # typically 22050 Hz
│   ├── 0x08: byte_rate: uint32
│   ├── 0x0C: block_align: uint16
│   ├── 0x0E: bits_per_sample: uint16
│   ├── 0x10: header_size: uint32    # 0x50 (80 bytes)
│   ├── 0x14: magic: char[4]         # "vs12"
│   ├── 0x18: file_size: uint32
│   ├── 0x1C: nibble_size: uint32    # total nibbles of audio data
│   ├── 0x20: unknown: int32         # -1
│   ├── 0x24: unknown: uint32        # 0
│   ├── 0x28: nibble_flag: uint32    # high/low nibble state (runtime)
│   ├── 0x2C: ADPCM state per channel (last to first, 12 bytes each)
│   │   ├── history: int32           # initial predictor value
│   │   ├── step_index: int32        # initial step table index
│   │   └── copy: int32              # copy of first ADPCM byte
│   └── 0x60: data_magic: char[4]    # "DATA"
└── 0x64: ADPCM data (byte-interleaved for stereo)
```

#### Ubisoft ADPCM Decoder Quirk (Not Used by Hype)

The decoder DLL (`APMmxBVR.dll`) contains a **modified IMA-ADPCM algorithm** with a quirk:
after looking up the step value from the step table, it clears the lowest 3 bits:

```c
step = step_table[step_index];
step &= ~7;  // Ubisoft quirk: clear bits 0-2
```

**However, testing proves Hype's APM files were encoded with standard IMA-ADPCM, not the quirk variant.**

#### Encoding Analysis

To determine whether audio was encoded with the quirk, we compared decoder outputs:

1. **Control test with known encoding:**
   - Standard-encoded sine wave: quirk decoder drifts positive (+75 DC offset at 1s)
   - Quirk-encoded sine wave: standard decoder drifts negative (-230 DC offset at 1s)

2. **Hype APM files (RoomSm.apm, Portetro.apm):**
   - Quirk decoder drifts massively positive (+329 to +1840 DC offset)
   - Standard decoder stays near zero
   - **Pattern matches standard-encoded audio**

This proves Hype's audio was encoded with **standard IMA-ADPCM**. The quirk in the decoder DLL
either wasn't used for encoding, or was only relevant for Rayman 2.

#### Extraction Methods

| Method | Command | Recommended | Notes |
|--------|---------|-------------|-------|
| FFmpeg | `ffmpeg -i file.apm out.wav` | ✓ Yes | Standard IMA-ADPCM (correct for Hype) |
| vgmstream | `vgmstream-cli file.apm -o out.wav` | ✓ Yes | Standard IMA-ADPCM |
| ray2get (standard) | `python ray2get.py di file.apm out.wav` | ✓ Yes | Standard IMA-ADPCM |
| ray2get (quirk) | `python ray2get.py d file.apm out.wav` | ✗ No | Applies quirk (causes drift) |

**For Hype, use standard IMA-ADPCM decoders (FFmpeg, vgmstream, or ray2get `di` mode).**

#### Decoder DLL

Hype's ISO contains the decoder DLL at `\DLL\APMmxBVR.dll`:
- **Version:** Moteur sonore ACP - Version M5.7.5 b
- **Build date:** Oct 22 1999
- **MD5:** `09cc56d1de00895a2eeb2dc8dc4a30bc`

The `snd_cpa.ini` config confirms: `[DLL_Adpcm]` → `Default=APMMX`

This is the same DLL family used by Rayman 2, but the quirk only affects Rayman 2's audio encoding.

### `.csb` - Sound Script Bank
**Format:** Text script
**Purpose:** Sound event definitions and resource references

```
CSB File (text)
├── {CsbHeader:
│   ├── SetNextFreeResourceId[%lu](N)
│   ├── VersionNumber(N)
│   └── SNDLibraryVersion(string)
│   }
├── {SndEventE:N(...)
│   ├── SetName(event_name)
│   └── SetParam1(resource_ref)
│   }
└── {SndResourceE:N(...)
    ├── SetName(sample_name)
    └── LoadResource(refs...)
    }
```

### `.lcb` - Sound Event Group List
**Format:** Text script
**Purpose:** Lists sound groups to load

```
LCB File (text)
├── {LcbHeader:
│   ├── SetDefaultLanguage(English)
│   └── VersionNumber(N)
│   }
└── {SndEventGroupList:
    └── LoadEventGroup(name, id)
    }
```

### `.sif` - Sound Interaction File
**Format:** Text script
**Purpose:** Material-based sound mappings

```
SIF File (text)
├── {SIF_Type:Environment[%ld](0)
│   └── {SIF_Value:Air/Water/...}
│   }
└── {SIF_Type:Material[%ld](1)
    └── {SIF_Value:Generic/Mud/Metal/Stone/Wood/...}
    }
```

### `.snd` - Sound References
**Format:** Binary
**Purpose:** Pointers to sound data
**Structure:** Array of sound reference entries

### `.sda` - Sound Data
**Format:** Binary
**Purpose:** Sound data pointers
**Structure:** Small header + pointers to sound resources

---

### `.bnm` - Sound Bank (Bnk_*.bnm)
**Format:** Binary (Ubisoft SBx)
**Purpose:** Bundles multiple APM audio samples into a single bank
**Location:** `Gamedata/World/Sound/Bnk_*.bnm`

```
BNM File
├── Header (44 bytes)
│   ├── int32 unknown0, unknown1
│   ├── int32 event_count
│   ├── int32 unknown2
│   ├── int32 file_count
│   ├── int32 size1, size2
│   └── int32 unknown[4]
├── Event Table (32 bytes each)
│   └── Binary event definitions
├── File Entry Table (92 bytes each)
│   └── For each sound:
│       ├── byte id, unknown[3]
│       ├── int32 type (1=PCM ref, 0xA=ADPCM)
│       ├── byte unknown[4]
│       ├── int32 length
│       ├── [type 0xA: int32 a_length + 40 bytes]
│       ├── [else: 44 bytes padding]
│       ├── int32 sample_rate
│       ├── byte unknown[8]
│       └── char name[20]
└── Embedded Audio Data
    └── IMA-ADPCM audio (same format as APM)
```

#### Extraction

Use **vgmstream** to extract all subsongs from a BNM file:

```bash
# Build vgmstream (from reference/vgmstream)
cd reference/vgmstream && mkdir build && cd build
cmake .. -DBUILD_AUDACIOUS=OFF -DUSE_MPEG=OFF -DUSE_FFMPEG=OFF -DUSE_VORBIS=OFF
make -j4

# Extract all subsongs
./cli/vgmstream-cli Bnk_0.bnm -o 'output/?n.wav' -S 0
```

**Notes:**
- Named "BNK" for "Bank" (sound bank), not related to Bink
- vgmstream treats each sound as a "subsong" - use `-S 0` to extract all
- Audio is IMA-ADPCM encoded (same as standalone APM files)
- Bnk_0.bnm contains 195 subsongs

---

## Animation Data

**Animation data is embedded inside SNA files**, not stored in separate files.

In the Montreal engine (Hype), animations are stored as part of the **State** system within SNA blocks:

```
SNA Block (Level Data)
└── Family (character/object type)
    └── State (action state, e.g., "walk", "jump")
        └── AnimationMontreal
            ├── off_frames: pointer     # Pointer to frame data
            ├── num_frames: byte        # Number of animation frames
            ├── speed: byte             # Playback speed
            ├── num_channels: byte      # Number of bone channels
            ├── speedMatrix: Matrix     # Transform matrix
            └── frames[]                # Array of AnimFrameMontreal
                └── AnimChannelMontreal[]  # Per-bone transforms
```

Unlike later OpenSpace games (Rayman 3) which use separate `.lvl`/`.ptr` keyframe files, the Montreal engine stores all animation data inline with the state machine in the main SNA blocks.

---

## Text/Localization Files

### `.dlg` - Dialog Data
**Format:** Binary
**Purpose:** Pointers to dialog text strings

```
DLG File
└── Array of pointers to text tables in SNA
```

### `.lng` - Language Data
**Format:** Binary (compressed like SNA)
**Purpose:** Language-specific level data overrides

---

## Configuration Files

### `.mem` - Memory Description
**Format:** Text script
**Purpose:** Memory allocation settings for engine

```
MEM File (text)
├── {NewMemoryDescription:
│   ├── UseMemorySnapshot(1)
│   ├── ACPTextMemory(N)
│   ├── ACPFixMemory(N)
│   ├── GameFixMemorySize(N)
│   └── ...
│   }
```

### `.ini` - Configuration Script
**Format:** Text script (SCR format)
**Purpose:** Game configuration (credits, menus)

```
INI File (text)
├── ; SCR version header
├── $SetCurrentFileDouble(...)
└── {Section:
    └── Function(params)
    }
```

### `.cfg` - Options Configuration
**Format:** Binary (XOR encrypted)
**Purpose:** Saved game options

### `.TRN` - Translation Table
**Format:** Binary
**Purpose:** Lookup/translation table (array of uint32)

### `.vgm` - Video Game Music Reference
**Format:** Text (often empty)
**Purpose:** Unknown (possibly music references)

---

## Video Files

### `.avi` - Video
**Format:** Standard AVI container
**Purpose:** Cutscenes and cinematics

---

## Per-Level File Set

Each level directory contains:

```
Gamedata/World/Levels/{levelname}/
├── {levelname}.sna     # Main level data (includes embedded animations)
├── {levelname}.gpt     # Scene graph roots
├── {levelname}.ptx     # Texture table
├── {levelname}.rtb     # SNA relocations
├── {levelname}.rtp     # GPT relocations
├── {levelname}.rtt     # Texture relocations
├── {levelname}.rtd     # Dialog relocations
├── {levelname}.rts     # Sound relocations
├── {levelname}.rtg     # Language SNA relocations
├── {levelname}.snd     # Sound references
├── {levelname}.sda     # Sound data
├── {levelname}.dlg     # Dialog pointers
├── {levelname}.lng     # Language data
├── {levelname}pgb.bin  # Polygon/collision data
└── fixlvl.rtb/rtg      # Fix-level relocations

Gamedata/World/Sound/
├── Bnk_*.bnm           # Sound banks (bundled APM samples)
├── *.apm               # Individual audio samples
├── *.csb               # Sound event scripts
└── playmo_b.lcb/sif    # Sound configuration
```

---

## References

### Submodules (in `reference/`)

| Directory | Repository | Purpose |
|-----------|------------|---------|
| `raymap` | [byvar/raymap](https://github.com/byvar/raymap) | Unity-based OpenSpace level viewer/editor |
| `ray2get` | [Synthesis/ray2get](https://github.com/Synthesis/ray2get) | APM audio converter (Python) |
| `vgmstream` | [vgmstream/vgmstream](https://github.com/vgmstream/vgmstream) | Video game audio decoder (supports BNM, APM) |
| `Rayman2Lib` | [szymski/Rayman2Lib](https://github.com/szymski/Rayman2Lib) | Rayman 2 modding tools (C#, includes Sound Bank Extractor) |
| `OpenRayman` | [imaginaryPineapple/OpenRayman](https://github.com/imaginaryPineapple/OpenRayman) | Open source Rayman 2 engine reimplementation (C++) |

### External Tools (no source available)

| Tool | Description | Notes |
|------|-------------|-------|
| F1BNM | BNM extractor for CPA/OpenSpace games | Binary only, available on RaymanPC forums |
| tonictac | Tonic Trouble audio converter | Source at ctpax-x.org, requires game DLLs |

### Other Resources

- [BinarySerializer.OpenSpace](https://github.com/BinarySerializer/BinarySerializer.OpenSpace) - Serialization library for OpenSpace formats
- [RayCarrot.Rayman](https://github.com/RayCarrot/RayCarrot.Rayman) - General Rayman helper library
- [RaymanPC Forums](https://raymanpc.com/forum/) - Community modding resources
