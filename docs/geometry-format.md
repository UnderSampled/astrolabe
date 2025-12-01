# Geometry Format

This document describes the mesh and vertex data structures used in OpenSpace.

## Overview

Geometry in OpenSpace is organized hierarchically:
- **SuperObject**: Top-level scene graph node
- **GeometricObject**: Collection of mesh elements
- **GeometricObjectElement**: Individual submesh (triangles or sprites)

## GeometricObject Structure

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint32 | lookAtMode | Billboard mode (0 = none) |
| 0x04 | uint16 | num_vertices | Number of vertices |
| 0x06 | uint16 | num_elements | Number of elements |
| 0x08 | uint16 | num_parallelBoxes | Number of bounding boxes |
| 0x0A | Vector3 | sphereCenter | Bounding sphere center (X, Y, Z) |
| 0x16 | float32 | sphereRadius | Bounding sphere radius |
| 0x1A | Pointer | off_vertices | Pointer to vertex array |
| 0x1E | Pointer | off_normals | Pointer to normal array |
| 0x22 | Pointer | off_blendWeights | Pointer to blend weights (skinning) |
| 0x26 | Pointer | off_materials | Pointer to material data |
| 0x2A | Pointer | off_element_types | Pointer to element type array |
| 0x2E | Pointer | off_elements | Pointer to element array |
| 0x32 | Pointer | off_parallelBoxes | Pointer to bounding boxes |

## Vertex Data

### Vertex Array

Located at `off_vertices`, contains `num_vertices` entries:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | float32 | x | X coordinate |
| 0x04 | float32 | y | Y coordinate |
| 0x08 | float32 | z | Z coordinate |

### Normal Array

Located at `off_normals`, contains `num_vertices` entries:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | float32 | nx | Normal X component |
| 0x04 | float32 | ny | Normal Y component |
| 0x08 | float32 | nz | Normal Z component |

## GeometricObjectElementTriangles

Represents a submesh with triangle data.

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | Pointer | off_material | Pointer to material |
| 0x04 | uint16 | num_triangles | Number of triangles |
| 0x06 | uint16 | num_uvs | Number of UV coordinates |
| 0x08 | uint16 | num_uvMaps | Number of UV maps |
| 0x0A | uint16 | num_vertex_indices | Number of indexed vertices |
| 0x0C | Pointer | off_triangles | Pointer to triangle indices |
| 0x10 | Pointer | off_mapping_uvs | Pointer to UV mapping |
| 0x14 | Pointer | off_normals | Pointer to per-vertex normals |
| 0x18 | Pointer | off_uvs | Pointer to UV coordinates |
| 0x1C | Pointer | off_vertex_indices | Pointer to vertex indices |

### Optimization Fields (Revolution/later engines)

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| +0x00 | Pointer | OPT_off_mapping_vertices | Optimized vertex mapping |
| +0x04 | Pointer | OPT_off_mapping_uvs | Optimized UV mapping |
| +0x08 | Pointer | OPT_off_triangleStrip | Triangle strip data |
| +0x0C | Pointer | OPT_off_disconnectedTriangles | Disconnected triangles |

## Triangle Data

Located at `off_triangles`, contains `num_triangles` entries:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint32 | v0 | Vertex index 0 |
| 0x04 | uint32 | v1 | Vertex index 1 |
| 0x08 | uint32 | v2 | Vertex index 2 |

Or in compact format (uint16):

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint16 | v0 | Vertex index 0 |
| 0x02 | uint16 | v1 | Vertex index 1 |
| 0x04 | uint16 | v2 | Vertex index 2 |

## UV Coordinates

Located at `off_uvs`, contains `num_uvs` entries:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | float32 | u | U texture coordinate |
| 0x04 | float32 | v | V texture coordinate |

## Material Reference

Materials are referenced by pointer and contain:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | Pointer | off_visualMaterial | Visual material properties |
| 0x04 | Pointer | off_mechanicsMaterial | Physics material properties |
| 0x08 | Pointer | off_soundMaterial | Sound material properties |

### Visual Material

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint32 | flags | Material flags (transparency, etc.) |
| 0x04 | Color | ambientColor | Ambient color (RGBA) |
| 0x08 | Color | diffuseColor | Diffuse color (RGBA) |
| 0x0C | Color | specularColor | Specular color (RGBA) |
| 0x10 | Pointer | off_texture | Pointer to texture reference |

## Coordinate System

OpenSpace uses a right-handed coordinate system:
- X: Right
- Y: Up
- Z: Forward (into screen)

When exporting to glTF (which uses a different convention), you may need to transform:
- glTF Y = OpenSpace Y
- glTF Z = -OpenSpace Z
- glTF X = OpenSpace X

## LookAt Modes (Billboard)

| Value | Mode | Description |
|-------|------|-------------|
| 0 | None | No billboard |
| 1 | AxisY | Rotate around Y axis only |
| 2 | Camera | Always face camera |

## Raymap Code Reference

- `reference/raymap/Assets/Scripts/OpenSpace/Visual/GeometricObject.cs`
- `reference/raymap/Assets/Scripts/OpenSpace/Visual/GeometricObjectElementTriangles.cs`
- `reference/raymap/Assets/Scripts/OpenSpace/Visual/VisualMaterial.cs`
