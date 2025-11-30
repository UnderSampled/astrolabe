#!/usr/bin/env python3
"""
Direct extraction of meshes by finding vertex arrays and nearby triangle arrays.

Instead of trying to parse the exact structure format, this script:
1. Finds vertex arrays (sequences of valid 3D float triplets)
2. Looks for nearby triangle index arrays that reference those vertices
3. Combines them into complete meshes
"""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass, field

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file


@dataclass
class VertexArray:
    offset: int
    count: int
    vertices: list[tuple[float, float, float]]


@dataclass
class TriangleArray:
    offset: int
    count: int
    triangles: list[tuple[int, int, int]]
    max_index: int


@dataclass
class ExtractedMesh:
    name: str
    vertices: list[tuple[float, float, float]]
    triangles: list[tuple[int, int, int]]


def u16(data: bytes, off: int) -> int:
    return struct.unpack("<H", data[off:off + 2])[0]


def i16(data: bytes, off: int) -> int:
    return struct.unpack("<h", data[off:off + 2])[0]


def u32(data: bytes, off: int) -> int:
    return struct.unpack("<I", data[off:off + 4])[0]


def f32(data: bytes, off: int) -> float:
    return struct.unpack("<f", data[off:off + 4])[0]


def is_valid_float(v: float, max_val: float = 50000) -> bool:
    if v != v:  # NaN
        return False
    if abs(v) > max_val:
        return False
    return True


def find_vertex_arrays(data: bytes, min_count: int = 10) -> list[VertexArray]:
    """Find sequences of valid 3D float coordinates."""
    results = []
    i = 0
    data_len = len(data)

    while i < data_len - 12:
        vertices = []
        start_offset = i

        while i < data_len - 12 and len(vertices) < 10000:
            x = f32(data, i)
            z = f32(data, i + 4)  # OpenSpace Z is up
            y = f32(data, i + 8)  # OpenSpace Y is forward

            if not (is_valid_float(x) and is_valid_float(y) and is_valid_float(z)):
                break

            vertices.append((x, y, z))
            i += 12

        if len(vertices) >= min_count:
            # Check for meaningful variation
            xs = [v[0] for v in vertices]
            ys = [v[1] for v in vertices]
            zs = [v[2] for v in vertices]

            x_range = max(xs) - min(xs)
            y_range = max(ys) - min(ys)
            z_range = max(zs) - min(zs)

            # Require variation in at least 2 dimensions
            dims_with_variation = sum([x_range > 0.1, y_range > 0.1, z_range > 0.1])

            # Also filter out arrays that are mostly zeros
            non_zero = sum(1 for v in vertices if abs(v[0]) > 0.01 or abs(v[1]) > 0.01 or abs(v[2]) > 0.01)

            if dims_with_variation >= 2 and non_zero > len(vertices) * 0.5:
                results.append(VertexArray(
                    offset=start_offset,
                    count=len(vertices),
                    vertices=vertices
                ))

        if i == start_offset:
            i += 4

    return results


def find_triangle_arrays(data: bytes, min_count: int = 5, max_index: int = 10000) -> list[TriangleArray]:
    """Find sequences of valid triangle indices (3 x i16 per triangle)."""
    results = []
    i = 0
    data_len = len(data)

    while i < data_len - 6:
        triangles = []
        start_offset = i
        actual_max_idx = 0

        while i < data_len - 6 and len(triangles) < 20000:
            v0 = i16(data, i)
            v1 = i16(data, i + 2)
            v2 = i16(data, i + 4)

            # Valid triangle indices: non-negative and below max_index
            if not (0 <= v0 < max_index and 0 <= v1 < max_index and 0 <= v2 < max_index):
                break

            triangles.append((v0, v1, v2))
            actual_max_idx = max(actual_max_idx, v0, v1, v2)
            i += 6

        if len(triangles) >= min_count:
            # Filter out arrays that are mostly zeros or have unreasonable patterns
            non_zero_tris = sum(1 for t in triangles if t[0] > 0 or t[1] > 0 or t[2] > 0)

            # Check for reasonable index distribution (not all same values)
            all_indices = [idx for tri in triangles for idx in tri]
            unique_indices = len(set(all_indices))

            if non_zero_tris >= len(triangles) * 0.3 and unique_indices >= 3:
                results.append(TriangleArray(
                    offset=start_offset,
                    count=len(triangles),
                    triangles=triangles,
                    max_index=actual_max_idx
                ))

        if i == start_offset:
            i += 2

    return results


def match_vertices_to_triangles(
    vertex_arrays: list[VertexArray],
    triangle_arrays: list[TriangleArray]
) -> list[ExtractedMesh]:
    """Match triangle arrays to compatible vertex arrays."""
    meshes = []

    # For each triangle array, find a compatible vertex array
    for tri_arr in triangle_arrays:
        # Find vertex arrays that could work with these triangles
        # (vertex count must be greater than max triangle index)
        compatible_verts = [
            va for va in vertex_arrays
            if va.count > tri_arr.max_index
        ]

        if not compatible_verts:
            continue

        # Find the closest compatible vertex array (by offset)
        # Prefer vertex arrays that appear before the triangle array
        best_va = None
        best_distance = float('inf')

        for va in compatible_verts:
            # Vertex data typically comes before triangle indices
            if va.offset < tri_arr.offset:
                distance = tri_arr.offset - va.offset
                # Prefer closer arrays, but within reasonable distance
                if distance < best_distance and distance < 500000:  # 500KB max distance
                    best_distance = distance
                    best_va = va

        if best_va is None:
            # Try vertex arrays after triangles
            for va in compatible_verts:
                distance = abs(va.offset - tri_arr.offset)
                if distance < best_distance and distance < 500000:
                    best_distance = distance
                    best_va = va

        if best_va:
            meshes.append(ExtractedMesh(
                name=f"mesh_{best_va.offset:08X}_{tri_arr.offset:08X}",
                vertices=best_va.vertices,
                triangles=tri_arr.triangles
            ))

    return meshes


def write_obj(mesh: ExtractedMesh, path: Path) -> None:
    """Write mesh to OBJ file."""
    with open(path, "w") as f:
        f.write(f"# Extracted from Hype: The Time Quest\n")
        f.write(f"# Mesh: {mesh.name}\n")
        f.write(f"# Vertices: {len(mesh.vertices)}\n")
        f.write(f"# Triangles: {len(mesh.triangles)}\n\n")

        for x, y, z in mesh.vertices:
            f.write(f"v {x} {y} {z}\n")

        f.write("\n# Faces\n")
        for v0, v1, v2 in mesh.triangles:
            f.write(f"f {v0 + 1} {v1 + 1} {v2 + 1}\n")


def write_gltf(meshes: list[ExtractedMesh], path: Path) -> None:
    """Write meshes to glTF 2.0 file."""
    import json
    import base64

    binary_data = bytearray()
    buffer_views = []
    accessors = []
    mesh_primitives = []

    for mesh in meshes:
        if not mesh.vertices or not mesh.triangles:
            continue

        xs = [v[0] for v in mesh.vertices]
        ys = [v[1] for v in mesh.vertices]
        zs = [v[2] for v in mesh.vertices]

        # Write vertices
        vertex_start = len(binary_data)
        for x, y, z in mesh.vertices:
            binary_data.extend(struct.pack("<fff", x, y, z))
        vertex_size = len(binary_data) - vertex_start

        while len(binary_data) % 4 != 0:
            binary_data.append(0)

        buffer_views.append({
            "buffer": 0,
            "byteOffset": vertex_start,
            "byteLength": vertex_size,
            "target": 34962
        })

        position_accessor = len(accessors)
        accessors.append({
            "bufferView": len(buffer_views) - 1,
            "componentType": 5126,
            "count": len(mesh.vertices),
            "type": "VEC3",
            "min": [min(xs), min(ys), min(zs)],
            "max": [max(xs), max(ys), max(zs)]
        })

        # Write indices
        index_start = len(binary_data)
        for v0, v1, v2 in mesh.triangles:
            binary_data.extend(struct.pack("<HHH", v0, v1, v2))
        index_size = len(binary_data) - index_start

        while len(binary_data) % 4 != 0:
            binary_data.append(0)

        buffer_views.append({
            "buffer": 0,
            "byteOffset": index_start,
            "byteLength": index_size,
            "target": 34963
        })

        index_accessor = len(accessors)
        accessors.append({
            "bufferView": len(buffer_views) - 1,
            "componentType": 5123,
            "count": len(mesh.triangles) * 3,
            "type": "SCALAR"
        })

        mesh_primitives.append({
            "name": mesh.name,
            "primitives": [{
                "attributes": {"POSITION": position_accessor},
                "indices": index_accessor,
                "mode": 4
            }]
        })

    gltf = {
        "asset": {
            "version": "2.0",
            "generator": "Astrolabe - Hype: The Time Quest Extractor"
        },
        "scene": 0,
        "scenes": [{"nodes": list(range(len(mesh_primitives)))}],
        "nodes": [{"mesh": i, "name": mesh_primitives[i]["name"]} for i in range(len(mesh_primitives))],
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
    sna_path = output_dir / "Gamedata/World/Levels/brigand/brigand.sna"

    print(f"Loading {sna_path.name}...")
    sna = read_sna_file(sna_path)

    all_meshes = []

    for block in sna.blocks:
        if len(block.data) < 1000:
            continue

        print(f"\nScanning block {block.module}/{block.block_id} "
              f"(base=0x{block.base_in_memory:08X}, {len(block.data)} bytes)...")

        # Find vertex arrays
        vertex_arrays = find_vertex_arrays(block.data, min_count=20)
        print(f"  Found {len(vertex_arrays)} vertex arrays")

        # Find triangle arrays
        triangle_arrays = find_triangle_arrays(block.data, min_count=10)
        print(f"  Found {len(triangle_arrays)} triangle arrays")

        # Match them
        meshes = match_vertices_to_triangles(vertex_arrays, triangle_arrays)
        print(f"  Matched {len(meshes)} meshes")

        all_meshes.extend(meshes)

    # Remove duplicates (same vertices paired with different triangles)
    # Keep only unique vertex arrays
    seen_vertices = set()
    unique_meshes = []
    for mesh in all_meshes:
        key = (len(mesh.vertices), mesh.vertices[0] if mesh.vertices else None)
        if key not in seen_vertices:
            seen_vertices.add(key)
            unique_meshes.append(mesh)

    all_meshes = unique_meshes

    if all_meshes:
        print(f"\n\n{'=' * 60}")
        print(f"Total unique meshes: {len(all_meshes)}")
        total_verts = sum(len(m.vertices) for m in all_meshes)
        total_tris = sum(len(m.triangles) for m in all_meshes)
        print(f"Total vertices: {total_verts}")
        print(f"Total triangles: {total_tris}")
        print("=" * 60)

        # Write individual OBJ files
        for i, mesh in enumerate(all_meshes[:30]):
            obj_path = output_dir / f"mesh_{i:03d}.obj"
            write_obj(mesh, obj_path)
            print(f"Wrote {mesh.name} ({len(mesh.vertices)} verts, {len(mesh.triangles)} tris) to {obj_path.name}")

        # Write combined glTF
        gltf_path = output_dir / "hype_meshes.gltf"
        write_gltf(all_meshes[:100], gltf_path)
        print(f"\nWrote combined glTF to {gltf_path}")

        # Write combined OBJ
        combined_vertices = []
        combined_triangles = []
        vertex_offset = 0
        for mesh in all_meshes:
            combined_vertices.extend(mesh.vertices)
            for v0, v1, v2 in mesh.triangles:
                combined_triangles.append((v0 + vertex_offset, v1 + vertex_offset, v2 + vertex_offset))
            vertex_offset += len(mesh.vertices)

        combined_mesh = ExtractedMesh(
            name="combined",
            vertices=combined_vertices,
            triangles=combined_triangles
        )
        combined_obj_path = output_dir / "all_meshes.obj"
        write_obj(combined_mesh, combined_obj_path)
        print(f"Wrote combined OBJ ({len(combined_vertices)} verts, {len(combined_triangles)} tris) to {combined_obj_path}")
    else:
        print("\nNo meshes found.")


if __name__ == "__main__":
    main()
