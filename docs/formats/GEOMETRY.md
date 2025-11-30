# Geometry and Mesh Formats

**Location:** Embedded within SNA data blocks
**Purpose:** 3D mesh data including vertices, triangles, materials, and skeletal animation

## Overview

OpenSpace stores 3D geometry in a hierarchical structure:

```
Physical Object (PO/IPO)
├── Visual Set (LOD levels)
│   └── Geometric Object (mesh)
│       ├── Vertices, Normals
│       ├── Blend Weights (skeletal)
│       ├── Deform Set (bones)
│       └── Elements (submeshes)
│           ├── Triangles Element
│           │   ├── Visual Material
│           │   ├── Triangle indices
│           │   ├── UV coordinates
│           │   └── Vertex colors
│           └── Sprites Element
│               └── Billboard sprites
└── Collide Set (collision mesh)
```

## Geometric Object

The main mesh container storing vertex data and submesh elements.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset to data
0x04    2       u16         Number of vertices
0x06    2       u16         Number of elements (submeshes)
0x08    12      Vector3     Bounding sphere center
0x14    4       float       Bounding sphere radius
0x18    4       u32         Flags
0x1C    4       Pointer     Vertices array pointer
0x20    4       Pointer     Normals array pointer
0x24    4       Pointer     Elements array pointer
--- Optional (if has deformation) ---
0x28    4       Pointer     Blend weights pointer
0x2C    4       Pointer     Deform set (skeleton) pointer
```

### Vertex Data

Vertices are stored as arrays of Vector3 (12 bytes each):
```
struct Vector3 {
    float x;
    float y;
    float z;
}
```

### Blend Weights (Skeletal Animation)

For skinned meshes, each vertex has up to 4 bone weights:
```
For each vertex:
    float weight[4]  // Bone influence weights (sum to 1.0)
```

## Geometric Object Element - Triangles

Submesh containing triangles with a single material.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Visual material pointer
0x04    2       u16         Number of triangles
0x06    2       u16         Number of UVs
0x08    2       u16         Number of UV maps
0x0A    4       Pointer     Triangles array pointer
0x0E    4       Pointer     UVs array pointer
0x12    4       Pointer     Normals array pointer (optional)
0x16    4       Pointer     Vertex mapping pointer (optional)
--- Optional ---
0x1A    4       Pointer     Vertex colors pointer
0x1E    4       i32         Lightmap index (-1 if none)
```

### Triangle Data

```
struct Triangle {
    u16 v0;     // Vertex index 0
    u16 v1;     // Vertex index 1
    u16 v2;     // Vertex index 2
}
```

Note: Triangle winding may be reversed depending on engine version.

### UV Coordinates

```
struct Vector2 {
    float u;
    float v;
}
```

UVs are mapped to triangles via the UV mapping array, allowing shared/unique UVs per triangle vertex.

### Vertex Colors

When present, RGBA colors per vertex (4 bytes each):
```
struct Color {
    u8 r;
    u8 g;
    u8 b;
    u8 a;
}
```

## Geometric Object Element - Sprites

Billboard/sprite elements for particle effects, foliage, etc.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    2       u16         Number of sprites
0x02    4       Pointer     Sprites array pointer
```

### Indexed Sprite

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Visual material pointer
0x04    8       Vector2     Size (width, height)
0x0C    8       Vector2     UV min (u1, v1)
0x14    8       Vector2     UV max (u2, v2)
0x1C    2       u16         Center point vertex index
```

## Visual Material

Material definition for rendering.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       u32         Flags
0x08    16      Vector4     Ambient coefficient (RGBA)
0x18    16      Vector4     Diffuse coefficient (RGBA)
0x28    16      Vector4     Specular coefficient (RGBA)
0x38    16      Vector4     Base color (RGBA)
0x48    4       u32         Number of textures
0x4C    2       u16         Number of animated textures
0x4E    4       Pointer     Textures array pointer
```

### Material Flags

| Bit | Name | Description |
|-----|------|-------------|
| 0 | Gouraud | Use Gouraud shading |
| 1 | ? | Unknown |
| 2 | ? | Unknown |
| 3 | Backface | Enable backface culling |
| 4 | Transparent | Has transparency |
| 5 | ZBuffer | Use Z-buffer testing |
| 6 | ZWrite | Write to Z-buffer |
| 7 | ? | Unknown |
| 8 | Additive | Additive blending |
| 9 | Subtractive | Subtractive blending |

### Visual Material Texture

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Texture pointer
0x04    4       float       UV scroll speed X
0x08    4       float       UV scroll speed Y
0x0C    4       u32         Texture properties/flags
```

## Physical Object

Container linking visual and collision geometry.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     Visual set pointer
0x08    4       Pointer     Collide set pointer
0x0C    2       u16         Visual set type (LOD configuration)
```

### Visual Set LOD

Level-of-detail wrapper:
```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Geometric object pointer
0x04    4       float       LOD distance threshold
```

## IPO (Instanced Physical Object)

Instance of a Physical Object placed in the world.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     Physical object data pointer
0x08    4       Pointer     Radiosity/lightmap pointer
```

## Collision Geometry

### Geometric Object Collide

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    2       u16         Number of elements
0x06    4       Pointer     Elements array pointer
```

### Element Types

| Type | Name | Description |
|------|------|-------------|
| 1 | Triangles | Triangle mesh collision |
| 7 | Spheres | Sphere primitives |
| 8 | Boxes | Axis-aligned boxes |

## Deform Set (Skeleton)

Bone hierarchy for skeletal animation.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    2       u16         Number of bones
0x06    4       Pointer     Bones array pointer
```

### Deform Bone

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     Parent bone pointer (null for root)
0x08    4       Pointer     First child pointer
0x0C    4       Pointer     Next sibling pointer
0x10    12      Vector3     Position
0x1C    16      Quaternion  Rotation
0x2C    12      Vector3     Scale
```

## Python Data Classes

```python
from dataclasses import dataclass, field
from typing import Optional

@dataclass
class Vector2:
    x: float
    y: float

@dataclass
class Vector3:
    x: float
    y: float
    z: float

@dataclass
class Vector4:
    x: float
    y: float
    z: float
    w: float

@dataclass
class Triangle:
    v0: int
    v1: int
    v2: int

@dataclass
class VisualMaterialTexture:
    texture_ptr: int
    scroll_x: float
    scroll_y: float
    properties: int

@dataclass
class VisualMaterial:
    flags: int
    ambient: Vector4
    diffuse: Vector4
    specular: Vector4
    color: Vector4
    textures: list[VisualMaterialTexture]

@dataclass
class GeometricObjectElementTriangles:
    material: VisualMaterial
    triangles: list[Triangle]
    uvs: list[Vector2]
    normals: Optional[list[Vector3]]
    vertex_colors: Optional[list[tuple[int, int, int, int]]]
    lightmap_index: int

@dataclass
class GeometricObjectElementSprites:
    sprites: list['IndexedSprite']

@dataclass
class IndexedSprite:
    material: VisualMaterial
    size: Vector2
    uv_min: Vector2
    uv_max: Vector2
    center_vertex: int

@dataclass
class GeometricObject:
    vertices: list[Vector3]
    normals: list[Vector3]
    blend_weights: Optional[list[list[float]]]  # 4 weights per vertex
    elements: list[GeometricObjectElementTriangles | GeometricObjectElementSprites]
    bounding_center: Vector3
    bounding_radius: float

@dataclass
class PhysicalObject:
    visual_lods: list[tuple[GeometricObject, float]]  # (mesh, distance)
    collision: Optional['GeometricObjectCollide']
```

## Conversion to glTF

When converting to glTF:

1. **Vertices** → `accessor` with `VEC3` type
2. **Normals** → `accessor` with `VEC3` type
3. **UVs** → `accessor` with `VEC2` type (flip V coordinate: `1.0 - v`)
4. **Triangles** → `accessor` with `SCALAR` type for indices
5. **Materials** → PBR material with base color texture
6. **Blend Weights** → `WEIGHTS_0` attribute
7. **Bones** → glTF skeleton with `skin` and `joints`

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Visual/GeometricObject.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Visual/GeometricObjectElementTriangles.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Visual/VisualMaterial.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/Object/PhysicalObject.cs`
