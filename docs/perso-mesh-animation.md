# Perso Mesh and Animation Architecture

This document describes how character meshes and animations are stored in the OpenSpace Montreal engine (Hype: The Time Quest).

## Overview

Characters in OpenSpace use a **flyweight pattern**:
- **Family**: Shared graphics template (meshes + animations)
- **Perso**: Instance with unique gameplay data (AI, physics, position)

Multiple Perso instances can share the same Family, meaning they use identical meshes and animations but have independent AI, physics, and transforms.

## Data Hierarchy

```
Family (shared graphics template)
├── family_index: uint32              # Type ID
├── name: string                      # "Hype", "Guard", etc.
├── states: LinkedList<State>         # Animation states
│   └── State
│       ├── name: string              # "Idle", "Walk", "Attack"
│       ├── speed: byte               # Playback speed
│       ├── anim_refMontreal: AnimationMontreal
│       └── stateTransitions          # State machine transitions
├── objectLists: LinkedList<ObjectList>  # Mesh configurations
│   └── ObjectList
│       └── entries[]: ObjectListEntry
│           ├── off_po: Pointer       # -> PhysicalObject
│           └── scale: Vector3
├── animBank: byte                    # Animation bank index
├── off_bounding_volume: Pointer      # Collision bounds
└── properties: byte                  # Flags

Perso (instance)
├── p3dData: Perso3dData
│   ├── family: Family                # Shared family reference
│   ├── objectList: ObjectList        # Can override family default
│   └── stateCurrent: State           # Current animation state
├── stdGame: StandardGame             # Game flags, type IDs
├── dynam: Dynam                      # Physics data
├── brain: Brain                      # AI scripts, DSG variables
├── collset: CollSet                  # Collision geometry
├── msWay: MSWay                      # Pathfinding
└── sectInfo: PersoSectorInfo         # Sector membership
```

## ObjectList: Mesh Part Collections

An ObjectList is a collection of mesh parts that make up a character:

```
ObjectList
├── num_entries: uint32
└── entries[]: ObjectListEntry
    ├── entries[0] → PhysicalObject (e.g., torso)
    ├── entries[1] → PhysicalObject (e.g., head)
    ├── entries[2] → PhysicalObject (e.g., left arm)
    └── ...
```

Each PhysicalObject contains:
- `visualSet[]`: Array of LOD mesh variants
- `collideMesh`: Collision geometry
- `scaleMultiplier`: Optional scale

**Key insight**: A character is NOT one mesh - it's a collection of mesh parts that are individually transformed by animation channels.

## Animation System

### AnimationMontreal Structure

```
AnimationMontreal
├── off_frames: Pointer               # -> frame array
├── num_frames: byte                  # Total frames
├── speed: byte                       # Playback speed (typically 30)
├── num_channels: byte                # Number of bone/object channels
├── unkbyte: byte
├── off_unk: Pointer
├── speedMatrix: Matrix4x4
└── frames[]: AnimFrameMontreal
```

### AnimFrameMontreal Structure

```
AnimFrameMontreal
├── off_channels: Pointer
├── off_mat: Pointer
├── off_vec: Pointer
├── off_hierarchies: Pointer
├── channels[]: AnimChannelMontreal   # Per-channel transforms
└── hierarchies[]: AnimHierarchy      # Parent-child relationships
```

### AnimChannelMontreal Structure

```
AnimChannelMontreal
├── off_matrix: Pointer
├── isIdentity: uint32                # 1 = use identity matrix
├── objectIndex: byte                 # Index into ObjectList.entries[]
├── unk1: byte
├── unk2: short
├── unk3: short
├── unkByte1: byte
├── unkByte2: byte
├── unkUint: uint32
└── matrix: Matrix (compressed)       # Transform for this channel
```

### AnimHierarchy Structure

```
AnimHierarchy
├── childChannelID: short
└── parentChannelID: short
```

Hierarchies define parent-child relationships between channels and can change per frame.

## Compressed Matrix Format

Montreal engine uses compressed matrices to save space. The type byte determines format:

| Type | Components |
|------|------------|
| 1    | Translation only |
| 2    | Rotation only |
| 3    | Translation + Rotation |
| 7    | Translation + Rotation + Uniform Scale |
| 11   | Translation + Rotation + Per-axis Scale |
| 15   | Translation + Rotation + Matrix Scale |

### Encoding

- **Position**: 3 × Int16, divide by 512
- **Rotation**: 4 × Int16 (WXYZ quaternion), divide by 32767
- **Scale**: Varies by type, divide by 256

```csharp
// Position
float x = (float)reader.ReadInt16() / 512f;
float y = (float)reader.ReadInt16() / 512f;
float z = (float)reader.ReadInt16() / 512f;

// Rotation (quaternion)
float w = (float)reader.ReadInt16() / 32767f;
float qx = (float)reader.ReadInt16() / 32767f;
float qy = (float)reader.ReadInt16() / 32767f;
float qz = (float)reader.ReadInt16() / 32767f;

// Scale (type 7 - uniform)
float scale = (float)reader.ReadInt16() / 256f;

// Scale (type 11 - per-axis)
float sx = (float)reader.ReadInt16() / 256f;
float sy = (float)reader.ReadInt16() / 256f;
float sz = (float)reader.ReadInt16() / 256f;
```

## Animation Playback

At runtime, animation playback works as follows:

1. Get current State from Perso
2. Get AnimationMontreal from State
3. For current frame:
   - Read AnimFrameMontreal
   - Apply hierarchy (parent-child relationships)
   - For each channel:
     - Get objectIndex from AnimChannelMontreal
     - Look up mesh: `objectList.entries[objectIndex].po`
     - Apply transform matrix to that mesh part
4. Advance frame, interpolating between keyframes

### Object Switching (NTTO)

Animations can switch which mesh is visible per channel per frame. This is controlled by `objectIndex`:
- Different frames can reference different ObjectList entries
- `objectIndex = -1` means invisible (no mesh shown)
- This allows animations to swap body parts (e.g., open/closed hand)

## GLTF Export Strategy

### Node Hierarchy

Each channel becomes a GLTF node:

```
Family_Root
├── Channel_0 (torso)
│   └── mesh: entries[0].po
├── Channel_1 (head)
│   └── mesh: entries[1].po
├── Channel_2 (left_arm)
│   └── mesh: entries[2].po
└── ...
```

### Animations

Each State becomes a GLTF animation:

```
animations:
  - name: "Idle"
    channels:
      - target: Channel_0, path: translation
        keyframes: [...]
      - target: Channel_0, path: rotation
        keyframes: [...]
      - target: Channel_1, path: translation
        keyframes: [...]
      ...
  - name: "Walk"
    ...
```

### Limitations

1. **Dynamic hierarchies**: Montreal animations can change parent-child relationships per frame. GLTF requires a static skeleton. We'll use the first frame's hierarchy as the base.

2. **Object switching**: GLTF doesn't support swapping meshes per frame. Options:
   - Export all mesh variants, use visibility animations
   - Export separate GLTF per ObjectList variant
   - Use morph targets (limited)

3. **Mesh parts vs single mesh**: GLTF expects meshes under nodes. We'll create one node per channel with its mesh attached.

## File Locations

In the extracted game data:

- **SNA blocks**: Contain Family, State, Animation data inline
- **ObjectLists**: Referenced from Family, contain PhysicalObject pointers
- **PhysicalObjects**: Contain GeometricObject (actual mesh data)

The data is all in the level's SNA file, accessed via pointer resolution using RTB relocation tables.

## Coordinate System

OpenSpace uses:
- Y-up coordinate system (same as GLTF)
- But may need axis conversion for some data
- Raymap uses `convertAxes: true` which swaps Y and Z

When exporting to GLTF:
```csharp
// Position conversion
Vector3 gltfPos = new Vector3(osPos.X, osPos.Z, osPos.Y);

// Quaternion conversion
Quaternion gltfRot = new Quaternion(osRot.X, osRot.Z, osRot.Y, -osRot.W);
```

## Name Sources

This section documents where different types of names come from in the OpenSpace Montreal engine.

| Name Type | Source Location | Example |
|-----------|-----------------|---------|
| Level | Directory name | `casino/` → "casino" |
| Texture | Inline in TextureInfo struct (SNA) | "castle_wall01txy" |
| State | Inline 0x50-byte buffer in State struct | "Idle", "Walk" |
| Family | objectTypes[0] linked list entries | "senekal", "gladiateur" |
| Model | objectTypes[1] linked list entries | "MSenekal", "MGladiateur" |
| Perso/Instance | objectTypes[2] linked list entries | "ISenekal", "IGladiateur" |

### Level Names

Level names are derived from the **directory name** containing the level files:

```
./disc/Gamedata/World/Levels/casino/
                              ^^^^^^
                              Level name = "casino"
```

The level name determines which files to load:
- `{levelName}.sna` - Compressed level data
- `{levelName}.gpt` - Global pointer table
- `{levelName}.ptx` - Texture pointer table
- `{levelName}.rtb` - Relocation table

### Texture Names

Texture names are stored **inline in TextureInfo structures** within SNA memory. The PTX file contains an array of pointers to these structures.

```
PTX file:
├── count: uint32
└── pointers[]: int32[]  → TextureInfo addresses in SNA

TextureInfo (in SNA memory):
├── flags, dimensions, etc.
└── name: null-terminated string  (e.g., "castle_wall01txy")
```

Texture names typically end with suffixes like:
- `txy` - Standard texture
- `txynz` - Normal-mapped texture
- `.gf` - Raw GF format reference

The `TextureTable` class scans the structure to find these embedded names.

### State Names

State names are stored **inline at the start of State structures** in SNA memory:

```
State structure (Montreal engine):
├── +0x00: name[0x50]    # 80-byte inline string buffer
├── +0x50: off_next
├── +0x54: off_prev
├── +0x58: off_header
├── +0x5C: off_anim_ref
└── ...
```

State names describe animations like "Idle", "Walk", "Attack", etc.

### Object Type Names (Family, Model, Perso)

The OpenSpace Montreal engine stores names for Families, Models, and Persos in an **objectTypes** table. This is an array of 3 linked lists:

```
objectTypes[0] = LinkedList<ObjectType>  // Family names
objectTypes[1] = LinkedList<ObjectType>  // Model names
objectTypes[2] = LinkedList<ObjectType>  // Perso/Instance names
```

### StandardGame Reference

Each Perso has a StandardGame structure that references objectType indices:

```
StandardGame
├── objectType_Family: uint32    # Index into objectTypes[0]
├── objectType_Model: uint32     # Index into objectTypes[1]
├── objectType_Perso: uint32     # Index into objectTypes[2]
└── off_superobject: Pointer
```

### ObjectType Entry Structure

Each entry in the linked list has:

```
ObjectType (Montreal engine)
├── +0x00: off_next (4 bytes)     # Pointer to next entry
├── +0x04: off_prev (4 bytes)     # Pointer to prev entry
├── +0x08: off_header (4 bytes)   # Pointer to list header
├── +0x0C: off_name (4 bytes)     # Pointer to inline name
├── +0x10: marker (4 bytes)       # Always 0x00000001
├── +0x14: internal_id (4 bytes)  # Internal ID (not the objectType index!)
└── +0x18: name_string            # Null-terminated ASCII name
```

### Naming Convention

Names follow a prefix convention to indicate type:

| Prefix Pattern | Type | Example |
|----------------|------|---------|
| `I` + uppercase | Instance/Perso | `ISenekal`, `IHammer_Senekal` |
| `i` + `_` | Instance/Perso | `i_Casino_Dragon_Camera` |
| `M` + uppercase | Model | `MSenekal`, `MHammer_Senekal` |
| `m` + `_` | Model | `m_Casino_Dragon_Camera` |
| lowercase | Family | `senekal`, `gladiateur` |
| Other | Family | `World`, `Actor3`, `SANGLIER` |

### Index Mapping

The objectType index (from StandardGame) corresponds to the **position** in the linked list, not the internal_id field:

```
objectTypes[0]:  (Family linked list)
  [26] → "senek_hammer"
  [27] → "xxe"
  [28] → "spikey"
  [29] → "World"
  [30] → "qe"
  [31] → "senekal"      # stdGame.objectType_Family = 31
  [32] → "qf"
  [33] → "gladiateur"
  ...
```

The base index (26 in this example) is determined by reading the Family struct's FamilyIndex field, which is stored at offset +0x0C of the Family structure.

### Name Extraction Algorithm

To extract Family names:

1. Scan SNA blocks for the marker pattern `01 00 00 00` followed by a name string
2. Filter to keep only Family-type names (exclude I/M prefix patterns)
3. Build entry map with next/prev pointers
4. Find the longest chain without Instance/Model names
5. Determine base index from nearby Family struct
6. Map chain position + base index → name

### Usage in Export

The `ObjectTypeReader` class implements this extraction:

```csharp
var objectTypeReader = new ObjectTypeReader(memory, loader.Sna);
var familyNames = objectTypeReader.TryFindFamilyNames();
// Returns: { 26: "senek_hammer", 27: "xxe", ..., 31: "senekal", ... }

// Apply to Family structures
foreach (var family in families)
{
    if (familyNames.TryGetValue(family.ObjectTypeIndex, out var name))
    {
        family.Name = name;  // "senekal" instead of "Family_31"
    }
}
```

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/`
  - `Animation/AnimationMontreal.cs`
  - `Animation/ComponentMontreal/AnimFrameMontreal.cs`
  - `Animation/ComponentMontreal/AnimChannelMontreal.cs`
  - `Animation/Component/AnimHierarchy.cs`
  - `Object/Properties/Family.cs`
  - `Object/Properties/State.cs`
  - `Object/Properties/ObjectList.cs`
  - `Object/PhysicalObject.cs`
  - `General/Matrix.cs` (ReadCompressed method)
  - `Unity/Perso/PersoBehaviour.cs` (animation playback)
