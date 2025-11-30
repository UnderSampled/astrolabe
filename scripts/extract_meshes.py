#!/usr/bin/env python3
"""Extract complete meshes (vertices + triangles) from Hype SNA files.

This script combines vertex and triangle detection to extract complete meshes
that can be imported into Blender as proper 3D models with faces.
"""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file


@dataclass
class ExtractedMesh:
    """A mesh extracted from the SNA data."""

    name: str
    offset: int
    vertices: list[tuple[float, float, float]] = field(default_factory=list)
    triangles: list[tuple[int, int, int]] = field(default_factory=list)
    uvs: list[tuple[float, float]] = field(default_factory=list)
    uv_indices: list[tuple[int, int, int]] = field(default_factory=list)


def u16(data: bytes, off: int) -> int:
    return struct.unpack("<H", data[off:off + 2])[0]


def i16(data: bytes, off: int) -> int:
    return struct.unpack("<h", data[off:off + 2])[0]


def u32(data: bytes, off: int) -> int:
    return struct.unpack("<I", data[off:off + 4])[0]


def f32(data: bytes, off: int) -> float:
    return struct.unpack("<f", data[off:off + 4])[0]


def is_valid_float(v: float, max_val: float = 50000) -> bool:
    """Check if a float is a valid coordinate."""
    if v != v:  # NaN
        return False
    if abs(v) > max_val:
        return False
    return True


def is_valid_vertex(x: float, y: float, z: float) -> bool:
    """Check if XYZ triplet is a valid vertex."""
    return is_valid_float(x) and is_valid_float(y) and is_valid_float(z)


def read_vertices(data: bytes, offset: int, count: int) -> list[tuple[float, float, float]]:
    """Read vertex array from data, converting from OpenSpace (X,Z,Y) to standard (X,Y,Z)."""
    vertices = []
    for i in range(count):
        pos = offset + i * 12
        if pos + 12 > len(data):
            break
        x = f32(data, pos)
        z = f32(data, pos + 4)  # OpenSpace Z is up
        y = f32(data, pos + 8)  # OpenSpace Y is forward
        if is_valid_vertex(x, y, z):
            vertices.append((x, y, z))
        else:
            break
    return vertices


def read_triangles(data: bytes, offset: int, count: int, max_vertex_idx: int) -> list[tuple[int, int, int]]:
    """Read triangle index array from data."""
    triangles = []
    for i in range(count):
        pos = offset + i * 6  # 3 x i16
        if pos + 6 > len(data):
            break
        v0 = i16(data, pos)
        v1 = i16(data, pos + 2)
        v2 = i16(data, pos + 4)
        # Validate indices
        if v0 < 0 or v1 < 0 or v2 < 0:
            break
        if v0 >= max_vertex_idx or v1 >= max_vertex_idx or v2 >= max_vertex_idx:
            break
        triangles.append((v0, v1, v2))
    return triangles


def find_geometry_objects(data: bytes, base_addr: int) -> list[ExtractedMesh]:
    """
    Find GeometricObject structures in data and extract complete meshes.

    Montreal format GeometricObject:
    - u32 num_vertices (offset 0)
    - ptr off_vertices (offset 4)
    - ptr off_normals (offset 8)
    - ptr off_materials (offset 12)
    - u32 unk (offset 16)
    - u32 num_elements (offset 20)
    - ptr off_element_types (offset 24)
    - ptr off_elements (offset 28)
    - ... skip to offset 48 ...
    - float sphere_radius (offset 48)
    - float sphere_x, sphere_z, sphere_y (offset 52-64)
    """
    meshes = []
    data_len = len(data)

    for i in range(0, data_len - 64, 4):
        num_vertices = u32(data, i)

        # Quick filter: reasonable vertex count
        if num_vertices < 3 or num_vertices > 50000:
            continue

        off_vertices_raw = u32(data, i + 4)

        # Check if pointer looks valid (points into this block)
        if off_vertices_raw < base_addr:
            continue
        vertex_data_offset = off_vertices_raw - base_addr
        if vertex_data_offset >= data_len or vertex_data_offset < i + 64:
            continue

        # Check num_elements at offset 20
        num_elements = u32(data, i + 20)
        if num_elements > 500:
            continue

        # Read sphere data at offset 48
        if i + 64 > data_len:
            continue

        sphere_radius = f32(data, i + 48)
        sphere_x = f32(data, i + 52)
        sphere_z = f32(data, i + 56)
        sphere_y = f32(data, i + 60)

        # Validate sphere
        if not is_valid_float(sphere_radius) or sphere_radius < 0.1 or sphere_radius > 5000:
            continue
        if not is_valid_vertex(sphere_x, sphere_y, sphere_z):
            continue

        # Try to read vertices
        vertices = read_vertices(data, vertex_data_offset, num_vertices)
        if len(vertices) < 3:
            continue

        # Check vertices have variation
        xs = [v[0] for v in vertices]
        ys = [v[1] for v in vertices]
        zs = [v[2] for v in vertices]
        x_range = max(xs) - min(xs) if xs else 0
        y_range = max(ys) - min(ys) if ys else 0
        z_range = max(zs) - min(zs) if zs else 0

        if x_range < 0.01 and y_range < 0.01 and z_range < 0.01:
            continue

        # Found a valid geometry object - now try to find triangles
        mesh = ExtractedMesh(
            name=f"geo_{i:08X}",
            offset=i,
            vertices=vertices,
        )

        # Look for triangle elements
        off_elements_raw = u32(data, i + 28)
        if off_elements_raw >= base_addr:
            elements_offset = off_elements_raw - base_addr
            if elements_offset < data_len:
                # Read element pointers
                for elem_idx in range(min(num_elements, 50)):
                    elem_ptr_offset = elements_offset + elem_idx * 4
                    if elem_ptr_offset + 4 > data_len:
                        break
                    elem_ptr_raw = u32(data, elem_ptr_offset)
                    if elem_ptr_raw < base_addr:
                        continue
                    elem_offset = elem_ptr_raw - base_addr
                    if elem_offset >= data_len or elem_offset + 48 > data_len:
                        continue

                    # Try to read triangle element
                    # Format: ptr material, u16 num_tris, u16 num_uvs, ptr off_triangles...
                    num_tris = u16(data, elem_offset + 4)
                    num_uvs = u16(data, elem_offset + 6)

                    if num_tris == 0 or num_tris > 50000:
                        continue

                    off_tris_raw = u32(data, elem_offset + 8)
                    if off_tris_raw >= base_addr:
                        tris_offset = off_tris_raw - base_addr
                        if tris_offset < data_len:
                            triangles = read_triangles(data, tris_offset, num_tris, len(vertices))
                            if triangles:
                                mesh.triangles.extend(triangles)

        if mesh.triangles or len(vertices) >= 10:
            meshes.append(mesh)

    return meshes


def write_obj(mesh: ExtractedMesh, path: Path) -> None:
    """Write mesh to OBJ file with faces."""
    with open(path, "w") as f:
        f.write(f"# Extracted from Hype: The Time Quest\n")
        f.write(f"# Mesh: {mesh.name}\n")
        f.write(f"# Vertices: {len(mesh.vertices)}\n")
        f.write(f"# Triangles: {len(mesh.triangles)}\n\n")

        # Write vertices
        for x, y, z in mesh.vertices:
            f.write(f"v {x} {y} {z}\n")

        # Write faces (OBJ indices are 1-based)
        if mesh.triangles:
            f.write("\n# Faces\n")
            for v0, v1, v2 in mesh.triangles:
                f.write(f"f {v0 + 1} {v1 + 1} {v2 + 1}\n")


def write_gltf(meshes: list[ExtractedMesh], path: Path) -> None:
    """Write meshes to glTF 2.0 file."""
    import json
    import base64

    # Collect all binary data
    binary_data = bytearray()
    buffer_views = []
    accessors = []
    mesh_primitives = []

    for mesh_idx, mesh in enumerate(meshes):
        if not mesh.vertices:
            continue

        # Calculate bounds for vertices
        xs = [v[0] for v in mesh.vertices]
        ys = [v[1] for v in mesh.vertices]
        zs = [v[2] for v in mesh.vertices]

        # Write vertex positions
        vertex_start = len(binary_data)
        for x, y, z in mesh.vertices:
            binary_data.extend(struct.pack("<fff", x, y, z))
        vertex_size = len(binary_data) - vertex_start

        # Pad to 4-byte alignment
        while len(binary_data) % 4 != 0:
            binary_data.append(0)

        buffer_views.append({
            "buffer": 0,
            "byteOffset": vertex_start,
            "byteLength": vertex_size,
            "target": 34962  # ARRAY_BUFFER
        })

        position_accessor = len(accessors)
        accessors.append({
            "bufferView": len(buffer_views) - 1,
            "componentType": 5126,  # FLOAT
            "count": len(mesh.vertices),
            "type": "VEC3",
            "min": [min(xs), min(ys), min(zs)],
            "max": [max(xs), max(ys), max(zs)]
        })

        primitive = {
            "attributes": {
                "POSITION": position_accessor
            }
        }

        # Write indices if we have triangles
        if mesh.triangles:
            index_start = len(binary_data)
            for v0, v1, v2 in mesh.triangles:
                # Use unsigned short indices
                binary_data.extend(struct.pack("<HHH", v0, v1, v2))
            index_size = len(binary_data) - index_start

            # Pad to 4-byte alignment
            while len(binary_data) % 4 != 0:
                binary_data.append(0)

            buffer_views.append({
                "buffer": 0,
                "byteOffset": index_start,
                "byteLength": index_size,
                "target": 34963  # ELEMENT_ARRAY_BUFFER
            })

            index_accessor = len(accessors)
            accessors.append({
                "bufferView": len(buffer_views) - 1,
                "componentType": 5123,  # UNSIGNED_SHORT
                "count": len(mesh.triangles) * 3,
                "type": "SCALAR"
            })

            primitive["indices"] = index_accessor
            primitive["mode"] = 4  # TRIANGLES
        else:
            primitive["mode"] = 0  # POINTS

        mesh_primitives.append({
            "name": mesh.name,
            "primitives": [primitive]
        })

    # Create glTF JSON
    gltf = {
        "asset": {
            "version": "2.0",
            "generator": "Astrolabe - Hype: The Time Quest Extractor"
        },
        "scene": 0,
        "scenes": [{
            "nodes": list(range(len(mesh_primitives)))
        }],
        "nodes": [
            {"mesh": i, "name": mesh_primitives[i]["name"]}
            for i in range(len(mesh_primitives))
        ],
        "meshes": mesh_primitives,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{
            "uri": f"data:application/octet-stream;base64,{base64.b64encode(binary_data).decode()}",
            "byteLength": len(binary_data)
        }]
    }

    with open(path, "w") as f:
        json.dump(gltf, f, indent=2)


def main():
    output_dir = Path("/home/deck/code/astrolabe/output")

    files_to_scan = [
        output_dir / "Gamedata/World/Levels/Fix.sna",
        output_dir / "Gamedata/World/Levels/brigand/brigand.sna",
    ]

    all_meshes = []

    for sna_path in files_to_scan:
        if not sna_path.exists():
            print(f"Skipping {sna_path} (not found)")
            continue

        print(f"\n{'=' * 60}")
        print(f"Loading {sna_path.name}...")
        print("=" * 60)

        sna = read_sna_file(sna_path)

        for block in sna.blocks:
            if len(block.data) < 1000:
                continue

            print(f"\nScanning block {block.module}/{block.block_id} "
                  f"(base=0x{block.base_in_memory:08X}, {len(block.data)} bytes)...")

            meshes = find_geometry_objects(block.data, block.base_in_memory)

            # Filter to meshes with triangles
            meshes_with_tris = [m for m in meshes if m.triangles]
            meshes_points_only = [m for m in meshes if not m.triangles and len(m.vertices) >= 20]

            if meshes_with_tris:
                print(f"  Found {len(meshes_with_tris)} meshes with triangles:")
                for m in meshes_with_tris[:5]:
                    print(f"    {m.name}: {len(m.vertices)} vertices, {len(m.triangles)} triangles")
                all_meshes.extend(meshes_with_tris)

            if meshes_points_only:
                print(f"  Found {len(meshes_points_only)} vertex-only objects (point clouds)")

    if all_meshes:
        print(f"\n\n{'=' * 60}")
        print(f"Total meshes with triangles: {len(all_meshes)}")
        total_verts = sum(len(m.vertices) for m in all_meshes)
        total_tris = sum(len(m.triangles) for m in all_meshes)
        print(f"Total vertices: {total_verts}")
        print(f"Total triangles: {total_tris}")
        print("=" * 60)

        # Write individual OBJ files for first few meshes
        for i, mesh in enumerate(all_meshes[:20]):
            obj_path = output_dir / f"mesh_{i:03d}.obj"
            write_obj(mesh, obj_path)
            print(f"Wrote {mesh.name} ({len(mesh.vertices)} verts, {len(mesh.triangles)} tris) to {obj_path.name}")

        # Write combined glTF
        gltf_path = output_dir / "hype_meshes.gltf"
        write_gltf(all_meshes[:50], gltf_path)  # Limit to 50 meshes for reasonable file size
        print(f"\nWrote combined glTF to {gltf_path}")

        # Also write a combined OBJ with all meshes
        combined_mesh = ExtractedMesh(name="combined", offset=0)
        vertex_offset = 0
        for mesh in all_meshes:
            combined_mesh.vertices.extend(mesh.vertices)
            # Adjust triangle indices for combined mesh
            for v0, v1, v2 in mesh.triangles:
                combined_mesh.triangles.append((v0 + vertex_offset, v1 + vertex_offset, v2 + vertex_offset))
            vertex_offset += len(mesh.vertices)

        combined_obj_path = output_dir / "all_meshes.obj"
        write_obj(combined_mesh, combined_obj_path)
        print(f"Wrote combined OBJ ({len(combined_mesh.vertices)} verts, {len(combined_mesh.triangles)} tris) to {combined_obj_path}")
    else:
        print("\nNo meshes with triangles found.")
        print("The geometry structures may use a different format.")


if __name__ == "__main__":
    main()
