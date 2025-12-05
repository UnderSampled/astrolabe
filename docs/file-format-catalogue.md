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
**Purpose:** Bundles multiple audio samples into a single bank
**Location:** `Gamedata/World/Sound/Bnk_*.bnm`

```
BNM File
├── Header (0x2C bytes)
│   ├── 0x00: version: uint32         # 0x00000000 or 0x00000200
│   ├── 0x04: section1_offset: uint32
│   ├── 0x08: section1_count: uint32  # Event count
│   ├── 0x0C: section2_offset: uint32
│   ├── 0x10: section2_count: uint32  # Audio entry count
│   ├── 0x14: mpdx_block_offset: uint32
│   ├── 0x18: midi_block_offset: uint32
│   ├── 0x1C: pcm_block_offset: uint32
│   ├── 0x20: apm_block_offset: uint32
│   ├── 0x24: streamed_block_offset: uint32
│   └── 0x28: eof_offset: uint32
│
├── Section 1: Event Table
│   └── Binary event definitions (32 bytes each)
│
├── Section 2: Audio Entry Table (0x5C or 0x60 bytes each)
│   └── For each entry:
│       ├── 0x00: header_id: uint32
│       ├── 0x04: header_type: uint32   # 0x01 = audio entry
│       ├── 0x0C: stream_size: uint32   # Audio data size in bytes
│       ├── 0x10: stream_offset: uint32 # Offset (see notes below)
│       ├── 0x3C: sample_rate: uint32
│       ├── 0x42: channels: uint16
│       ├── 0x44: stream_type: uint32   # 0x01=PCM, 0x02=MPDX, 0x04=APM
│       └── 0x48: name: char[20]
│
└── Audio Data Blocks
    ├── MPDX Block (Ubi-MPEG compressed voice/dialogue)
    ├── PCM Block (raw 16-bit signed LE audio)
    └── APM Block (IMA-ADPCM compressed audio)
```

#### Stream Types

| Type | Value | Format | Typical Use |
|------|-------|--------|-------------|
| PCM | 0x01 | Raw 16-bit signed little-endian | Sound effects |
| MPDX | 0x02 | Ubi-MPEG (modified VBR MP2) | Voice/dialogue |
| APM | 0x04 | IMA-ADPCM with APM header | Music, ambient |

#### Extraction

Use Astrolabe CLI to extract all audio from a BNM file:

```bash
dotnet run --project src/Astrolabe.Cli -- audio Bnk_0.bnm ./output
```

Or use **vgmstream** for APM entries:

```bash
./cli/vgmstream-cli Bnk_0.bnm -o 'output/?n.wav' -S 0
```

**Notes:**
- Named "BNK" for "Bank" (sound bank), not related to Bink
- Version 0x200 has 4 extra bytes per entry (0x60 vs 0x5C)
- **Offset interpretation varies by stream type:**
  - PCM/APM: `stream_offset` is relative to their respective block start (pcm_block or apm_block)
  - MPDX: `stream_offset` is an **absolute file offset** (not relative to mpdx_block)
- APM uses IMA-ADPCM with **high nibble first** (non-standard nibble order)

---

### Ubi-MPEG Format (MPDX)

**Format:** Modified VBR MPEG Layer 2 (MP2)
**Purpose:** Compressed voice/dialogue audio in BNM banks
**Codec:** Custom Ubisoft variant of MP2

Ubi-MPEG is a proprietary audio format used by Ubisoft games from the late 1990s (Rayman 2,
Tonic Trouble, Hype: The Time Quest). It's a modified VBR MP2 format optimized for speech.

#### Differences from Standard MP2

| Feature | Standard MP2 | Ubi-MPEG |
|---------|-------------|----------|
| Sync word | 11 bits (0x7FF) | 12 bits (0xFFF) |
| Header size | 32 bits | 16 bits (sync + 4-bit mode) |
| Frame alignment | Byte-aligned | Bit-aligned (frames follow immediately) |
| Bitrate info | In header | Implicit (VBR ~128-160kbps) |
| Sample rate info | In header | Fixed (44100 Hz) |
| CRC/padding bits | In header | Omitted |

#### Header Structure

```
Standard MP2 Header (32 bits):
├── sync: 11 bits (0x7FF)
├── mpeg_version: 2 bits
├── layer: 2 bits
├── protection: 1 bit
├── bitrate_index: 4 bits
├── sample_rate_index: 2 bits
├── padding: 1 bit
├── private: 1 bit
├── channel_mode: 2 bits
├── mode_extension: 2 bits
├── copyright: 1 bit
├── original: 1 bit
└── emphasis: 2 bits

Ubi-MPEG Header (16 bits):
├── sync: 12 bits (0xFFF)
└── mode: 4 bits
    ├── mode_extension: 2 bits (joint stereo bounds)
    └── channel_mode: 2 bits (0=stereo, 1=joint, 3=mono)
```

#### MPDX Stream Structure in BNM Files

MPDX data in BNM files has a wrapper header before the Ubi-MPEG data:

```
MPDX Stream
├── 4 bytes: unknown (stream ID or size-related field)
├── Optional 4 bytes: surround marker ("2RUS" or "1RUS")
└── Ubi-MPEG data (frames starting with 0xFFF sync)
```

#### Surround Mode Headers

Some Ubi-MPEG streams include a 4-byte surround mode header after the 4-byte prefix:

| Header | Meaning | Structure |
|--------|---------|-----------|
| `2RUS` | Stereo surround | Pairs of stereo + mono frames |
| `1RUS` | Mono surround | Pairs of frames (rare) |
| (none) | No surround | Ubi-MPEG starts directly after 4-byte prefix |

In surround mode, each "logical frame" consists of:
1. A stereo frame (main audio)
2. A mono frame (surround/center channel data)

The mono frame's coefficients are meant to be mixed with the stereo frame during
synthesis to produce surround output. Current decoders typically ignore the mono frame.

#### Frame Data Structure

After the header, frame data follows standard MP2 structure:
1. Bit allocation (per subband, per channel)
2. Scalefactor selector information (SCFSI)
3. Scalefactors (6 bits each, 1-3 per subband based on SCFSI)
4. Quantized DCT coefficients (12 granules × subbands × channels)

Key differences:
- Only table 0 (27 subbands) is used
- Frames are **not** byte-aligned - next frame starts immediately after data
- No ancillary data or padding bytes

#### Decoding Process

To decode Ubi-MPEG:

1. **Find sync** - Search for 12-bit 0xFFF pattern
2. **Read mode** - 4 bits: extract channel_mode and mode_extension
3. **Transform to MP2** - Write standard 32-bit header with compatible settings:
   - Use 256kbps @ 48000Hz (allows sufficient frame size)
   - Copy bit allocation, scalefactors, and samples verbatim
   - Byte-align and pad output frame
4. **Decode MP2** - Use standard MP2 decoder (e.g., NLayer, minimp3)
5. **Repeat** - Process next frame (no byte alignment between frames)

#### Reference Implementation

The Astrolabe `UbiMpegDecoder` class transforms Ubi-MPEG to standard MP2 and uses
NLayer for decoding. See `src/Astrolabe.Core/FileFormats/Audio/UbiMpegDecoder.cs`.

vgmstream also implements Ubi-MPEG decoding in `src/coding/ubi_mpeg_decoder.c` and
`src/coding/libs/ubi_mpeg_helpers.c`, using minimp3 for the transformed frames.

#### Original Decoder DLLs

The games shipped with decoder DLLs:
- `MPGMXBVR.dll` - Regular version (no SIMD)
- `MPGMXSVR.dll` - XMM/SIMD optimized version

These DLLs implement the full Ubi-MPEG decoder natively without transformation

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
