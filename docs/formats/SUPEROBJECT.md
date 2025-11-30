# SuperObject and Scene Hierarchy

**Location:** Embedded within SNA data blocks
**Purpose:** Scene graph structure organizing all game objects

## Overview

The OpenSpace engine uses a hierarchical scene graph based on **SuperObjects**. Every entity in the game world (meshes, characters, sectors, lights) is represented as a SuperObject node.

## SuperObject Types

| Type Code (Montreal) | Type Code (R2/R3) | Name | Description |
|---------------------|-------------------|------|-------------|
| 0x00 | 0x01 | World | Container/group node |
| 0x04 | 0x02 | Perso | Character/actor |
| 0x08 | 0x04 | Sector | World partition |
| 0x0D | 0x20 | IPO | Static interactive object |
| 0x15 | 0x40 | IPO_2 | Alternative IPO type |
| - | 0x08 | PhysicalObject | Dynamic physics object |
| - | 0x400 | GeometricObject | Visual mesh only |
| - | 0x80000 | GeometricShadowObject | Shadow geometry |

## SuperObject Structure

### Montreal Engine

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       u32         Type code
0x04    4       Pointer     Data pointer (type-specific)
0x08    4       Pointer     First child pointer
0x0C    4       Pointer     Next sibling pointer
0x10    4       Pointer     Previous sibling pointer
0x14    4       Pointer     Parent pointer
0x18    64      Matrix4x4   Local transform matrix
0x58    4       u32         Draw flags
0x5C    4       u32         Flags (SuperObjectFlags)
0x60    ?       BoundVol    Bounding volume
```

### Transform Matrix

4x4 transformation matrix stored in column-major order:
```
| m00 m01 m02 m03 |   | Right.x    Up.x    Forward.x  Pos.x |
| m10 m11 m12 m13 | = | Right.y    Up.y    Forward.y  Pos.y |
| m20 m21 m22 m23 |   | Right.z    Up.z    Forward.z  Pos.z |
| m30 m31 m32 m33 |   | 0          0       0          1     |
```

## SuperObject Flags

32-bit flags controlling behavior:

| Bit | Name | Description |
|-----|------|-------------|
| 0 | NoCollision | Disable collision detection |
| 1 | Invisible | Don't render this object |
| 2 | NoTransformMatrix | Ignore transform (identity) |
| 3 | ZoomInsteadOfScale | Use zoom instead of scale |
| 4 | BBoxInsteadOfSphere | Use box for bounds (not sphere) |
| 5 | DisplayOnTop | Render above everything else |
| 6 | NoRayTracing | Exclude from ray casts |
| 7 | NoShadow | Don't cast shadows |
| 8 | SemiLookat | Billboard on one axis |
| 9 | SpecialBoundingVolumes | Children bounds not in parent |

## Hierarchy Structure

```
World (Root)
├── Father Sector (World container)
│   ├── Sector 0
│   │   ├── IPO (static mesh)
│   │   ├── IPO (static mesh)
│   │   └── ...
│   ├── Sector 1
│   │   └── ...
│   └── Sector N
├── Dynamic World
│   ├── Perso (player)
│   ├── Perso (NPC)
│   └── ...
├── Inactive Dynamic World
│   └── (disabled objects)
└── Transit World
    └── (persistent objects between levels)
```

## Sector

World partitioning unit for culling and streaming.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     Perso list pointer
0x08    4       Pointer     Static lights list pointer
0x0C    4       Pointer     Graphic sectors list pointer
0x10    4       Pointer     Collision sectors list pointer
0x14    4       Pointer     Activity sectors list pointer
0x18    ?       BoundVol    Sector boundary
0x??    1       u8          Is virtual sector
0x??    1       u8          Sector priority
0x??    4       Pointer     Sky material pointer
0x??    4       Pointer     Collision mesh pointer (Montreal)
```

### Neighbor Sector

Links between adjacent sectors for visibility/collision:
```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Sector pointer
0x04    ?       ...         Additional neighbor data
```

## Perso (Character/Actor)

Game entity with AI, animation, and collision.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     3D data pointer (Perso3dData)
0x08    4       Pointer     Standard game pointer (metadata)
0x0C    4       Pointer     Dynamics pointer (physics)
0x10    4       Pointer     Brain pointer (AI)
0x14    4       Pointer     Collision set pointer
0x18    4       Pointer     Sector info pointer
```

### Perso3dData

3D model and animation data:
```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Family pointer
0x04    4       Pointer     Object list pointer
0x08    4       Pointer     Current state pointer
```

### StandardGame

Object classification:
```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       u32         Family type
0x04    4       u32         Model type
0x08    4       u32         Instance ID
0x0C    4       Pointer     Super object pointer
0x10    1       u8          Update flag
0x11    1       u8          Is main actor
```

## World

Virtual container node (no data, just hierarchy organization).

Common world names:
- `"actual world"` - Root of everything
- `"dynamic world"` - Dynamic objects and characters
- `"inactive dynamic world"` - Disabled dynamic objects
- `"father sector"` - Container for all sectors

## Bounding Volume

### Sphere (default)

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    12      Vector3     Center
0x0C    4       float       Radius
```

### Box (when flag set)

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    12      Vector3     Min corner
0x0C    12      Vector3     Max corner
```

## Python Data Classes

```python
from dataclasses import dataclass
from enum import IntEnum, IntFlag
from typing import Optional, Union

class SuperObjectType(IntEnum):
    """SuperObject type codes (Montreal engine)."""
    WORLD = 0x00
    PERSO = 0x04
    SECTOR = 0x08
    IPO = 0x0D
    IPO_2 = 0x15

class SuperObjectFlags(IntFlag):
    """SuperObject behavior flags."""
    NO_COLLISION = 1 << 0
    INVISIBLE = 1 << 1
    NO_TRANSFORM = 1 << 2
    ZOOM_INSTEAD_OF_SCALE = 1 << 3
    BBOX_INSTEAD_OF_SPHERE = 1 << 4
    DISPLAY_ON_TOP = 1 << 5
    NO_RAY_TRACING = 1 << 6
    NO_SHADOW = 1 << 7
    SEMI_LOOKAT = 1 << 8
    SPECIAL_BOUNDING_VOLUMES = 1 << 9

@dataclass
class Matrix4x4:
    """4x4 transformation matrix."""
    data: list[float]  # 16 floats, column-major

    @property
    def position(self) -> 'Vector3':
        return Vector3(self.data[12], self.data[13], self.data[14])

@dataclass
class BoundingSphere:
    center: 'Vector3'
    radius: float

@dataclass
class BoundingBox:
    min: 'Vector3'
    max: 'Vector3'

@dataclass
class SuperObject:
    """Scene graph node."""
    type: SuperObjectType
    data_ptr: int  # Pointer to type-specific data
    transform: Matrix4x4
    draw_flags: int
    flags: SuperObjectFlags
    bounds: Union[BoundingSphere, BoundingBox]

    # Hierarchy (resolved from pointers)
    children: list['SuperObject']
    parent: Optional['SuperObject'] = None

@dataclass
class Sector:
    """World partition."""
    boundary: Union[BoundingSphere, BoundingBox]
    persos: list['Perso']
    static_lights: list['LightInfo']
    neighbor_graphic: list['Sector']
    neighbor_collision: list['Sector']
    is_virtual: bool
    priority: int

@dataclass
class Perso:
    """Game character/actor."""
    perso3d: 'Perso3dData'
    standard_game: 'StandardGame'
    dynamics: Optional['Dynam']
    brain: Optional['Brain']
    collision: Optional['CollSet']
    sector_info: Optional['PersoSectorInfo']

@dataclass
class StandardGame:
    """Object classification metadata."""
    family_type: int
    model_type: int
    instance_id: int
    is_main_actor: bool
```

## Scene Traversal

```python
def traverse_scene(root: SuperObject, visitor: Callable[[SuperObject, int], None], depth: int = 0):
    """Traverse scene graph depth-first."""
    visitor(root, depth)
    for child in root.children:
        traverse_scene(child, visitor, depth + 1)

# Example: collect all IPOs
def collect_ipos(root: SuperObject) -> list[SuperObject]:
    ipos = []
    def visitor(obj: SuperObject, depth: int):
        if obj.type in (SuperObjectType.IPO, SuperObjectType.IPO_2):
            ipos.append(obj)
    traverse_scene(root, visitor)
    return ipos
```

## Conversion to Godot

When converting to Godot scene:

1. **World** → `Node3D` (empty container)
2. **Sector** → `Node3D` with visibility notifier
3. **IPO** → `MeshInstance3D` with collision shape
4. **Perso** → Custom scene with AnimationPlayer
5. **Transform** → `Node3D.transform` (convert matrix)
6. **Bounds** → `VisibilityNotifier3D` or `Area3D`

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Object/SuperObject.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Object/Sector.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Object/Perso.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Object/World.cs`
