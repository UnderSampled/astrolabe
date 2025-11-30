#!/usr/bin/env python3
"""Extract meshes using RTB pointer information to locate structures."""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass, field

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file
from astrolabe.openspace.relocation import RelocationTable


@dataclass
class ExtractedMesh:
    """A mesh extracted from the SNA data."""
    name: str
    offset: int
    vertices: list[tuple[float, float, float]] = field(default_factory=list)
    triangles: list[tuple[int, int, int]] = field(default_factory=list)


def u16(data: bytes, off: int) -> int:
    return struct.unpack("<H", data[off:off + 2])[0]


def i16(data: bytes, off: int) -> int:
    return struct.unpack("<h", data[off:off + 2])[0]


def u32(data: bytes, off: int) -> int:
    return struct.unpack("<I", data[off:off + 4])[0]


def f32(data: bytes, off: int) -> float:
    return struct.unpack("<f", data[off:off + 4])[0]


def is_valid_float(v: float) -> bool:
    if v != v:  # NaN
        return False
    if abs(v) > 50000:
        return False
    return True


def is_valid_vertex(x: float, y: float, z: float) -> bool:
    return is_valid_float(x) and is_valid_float(y) and is_valid_float(z)


def read_vertices_at(data: bytes, offset: int, count: int) -> list[tuple[float, float, float]]:
    """Read vertex array, converting from OpenSpace (X,Z,Y) to standard (X,Y,Z)."""
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


def read_triangles_at(data: bytes, offset: int, count: int, max_idx: int) -> list[tuple[int, int, int]]:
    """Read triangle indices."""
    triangles = []
    for i in range(count):
        pos = offset + i * 6
        if pos + 6 > len(data):
            break
        v0 = i16(data, pos)
        v1 = i16(data, pos + 2)
        v2 = i16(data, pos + 4)
        if 0 <= v0 < max_idx and 0 <= v1 < max_idx and 0 <= v2 < max_idx:
            triangles.append((v0, v1, v2))
        else:
            break
    return triangles


def find_geometry_with_rtb(data: bytes, base_addr: int, pointer_offsets: set[int]) -> list[ExtractedMesh]:
    """
    Find GeometricObject structures using RTB pointer locations.

    Montreal format GeometricObject (64+ bytes):
    - 0:  u32 num_vertices
    - 4:  ptr off_vertices  <- RTB marks this as a pointer
    - 8:  ptr off_normals   <- RTB marks this as a pointer
    - 12: ptr off_materials <- RTB marks this as a pointer
    - 16: u32 unk
    - 20: u32 num_elements
    - 24: ptr off_element_types <- RTB marks this as a pointer
    - 28: ptr off_elements <- RTB marks this as a pointer
    - 32-44: skip
    - 48: float sphere_radius
    - 52-60: float sphere_x, sphere_z, sphere_y
    """
    meshes = []
    data_len = len(data)

    # Look for locations where there's a pointer at offset 4 (off_vertices)
    # and we can validate the structure
    for ptr_offset in sorted(pointer_offsets):
        # Check if this could be off_vertices (at offset 4 of GeometricObject)
        struct_start = ptr_offset - 4
        if struct_start < 0:
            continue
        if struct_start + 64 > data_len:
            continue

        # Read what would be num_vertices at offset 0
        num_vertices = u32(data, struct_start)
        if num_vertices < 3 or num_vertices > 50000:
            continue

        # Verify off_vertices pointer is valid
        off_vertices_raw = u32(data, struct_start + 4)
        if off_vertices_raw < base_addr:
            continue
        vertex_offset = off_vertices_raw - base_addr
        if vertex_offset >= data_len:
            continue

        # Verify there are consecutive pointers (off_normals at +8, off_materials at +12)
        # These should also be in the pointer_offsets set
        if (struct_start + 8) not in pointer_offsets:
            continue

        # Check num_elements at offset 20
        num_elements = u32(data, struct_start + 20)
        if num_elements > 500:
            continue

        # Check sphere radius at offset 48
        sphere_radius = f32(data, struct_start + 48)
        if not is_valid_float(sphere_radius) or sphere_radius < 0 or sphere_radius > 10000:
            continue

        # Try to read vertices
        vertices = read_vertices_at(data, vertex_offset, num_vertices)
        if len(vertices) < 3:
            continue

        # Check vertices have variation
        xs = [v[0] for v in vertices]
        ys = [v[1] for v in vertices]
        zs = [v[2] for v in vertices]
        x_range = max(xs) - min(xs) if xs else 0
        y_range = max(ys) - min(ys) if ys else 0
        z_range = max(zs) - min(zs) if zs else 0

        if x_range < 0.1 and y_range < 0.1 and z_range < 0.1:
            continue

        mesh = ExtractedMesh(
            name=f"geo_{struct_start:08X}",
            offset=struct_start,
            vertices=vertices,
        )

        # Try to find triangle elements
        off_elements_raw = u32(data, struct_start + 28)
        if off_elements_raw >= base_addr:
            elements_offset = off_elements_raw - base_addr
            if elements_offset < data_len:
                # Read element pointers
                for elem_idx in range(min(num_elements, 20)):
                    elem_ptr_off = elements_offset + elem_idx * 4
                    if elem_ptr_off + 4 > data_len:
                        break
                    if elem_ptr_off not in pointer_offsets:
                        # Not a valid pointer location
                        continue
                    elem_ptr_raw = u32(data, elem_ptr_off)
                    if elem_ptr_raw < base_addr:
                        continue
                    elem_offset = elem_ptr_raw - base_addr
                    if elem_offset + 48 > data_len:
                        continue

                    # Try to read triangle element
                    # Montreal format: ptr material, u16 num_tris, u16 num_uvs, ptr off_triangles...
                    num_tris = u16(data, elem_offset + 4)
                    if num_tris == 0 or num_tris > 50000:
                        continue

                    # Check if off_triangles location is a pointer
                    off_tris_loc = elem_offset + 8
                    if off_tris_loc not in pointer_offsets:
                        continue

                    off_tris_raw = u32(data, off_tris_loc)
                    if off_tris_raw >= base_addr:
                        tris_offset = off_tris_raw - base_addr
                        if tris_offset < data_len:
                            triangles = read_triangles_at(data, tris_offset, num_tris, len(vertices))
                            if triangles:
                                mesh.triangles.extend(triangles)

        if mesh.triangles or len(vertices) >= 10:
            meshes.append(mesh)
            print(f"  Found mesh at 0x{struct_start:08X}: {len(mesh.vertices)} verts, {len(mesh.triangles)} tris")

    return meshes


def write_obj(mesh: ExtractedMesh, path: Path) -> None:
    """Write mesh to OBJ file with faces."""
    with open(path, "w") as f:
        f.write(f"# Extracted from Hype: The Time Quest\n")
        f.write(f"# Mesh: {mesh.name}\n")
        f.write(f"# Vertices: {len(mesh.vertices)}\n")
        f.write(f"# Triangles: {len(mesh.triangles)}\n\n")

        for x, y, z in mesh.vertices:
            f.write(f"v {x} {y} {z}\n")

        if mesh.triangles:
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

    for mesh_idx, mesh in enumerate(meshes):
        if not mesh.vertices:
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

        primitive = {
            "attributes": {"POSITION": position_accessor}
        }

        if mesh.triangles:
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

            primitive["indices"] = index_accessor
            primitive["mode"] = 4
        else:
            primitive["mode"] = 0

        mesh_primitives.append({
            "name": mesh.name,
            "primitives": [primitive]
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
    rtb_path = sna_path.with_suffix(".rtb")

    print(f"Loading {sna_path.name}...")
    sna = read_sna_file(sna_path)

    print(f"Loading {rtb_path.name}...")
    rtb = RelocationTable.from_file(str(rtb_path), encrypted=False, compressed=True)

    all_meshes = []

    # Process each block with its RTB pointer information
    for block in sna.blocks:
        if len(block.data) < 1000:
            continue

        # Find RTB pointer block for this SNA block
        rtb_block = None
        for pb in rtb.pointer_blocks:
            if pb.module == block.module and pb.block_id == block.block_id:
                rtb_block = pb
                break

        if not rtb_block:
            continue

        # Build set of pointer offsets within this block
        pointer_offsets = set()
        for ptr in rtb_block.pointers:
            offset = ptr.offset_in_memory - block.base_in_memory
            if 0 <= offset < len(block.data):
                pointer_offsets.add(offset)

        print(f"\nScanning block {block.module}/{block.block_id} "
              f"(base=0x{block.base_in_memory:08X}, {len(block.data)} bytes, "
              f"{len(pointer_offsets)} pointer locations)...")

        meshes = find_geometry_with_rtb(block.data, block.base_in_memory, pointer_offsets)

        meshes_with_tris = [m for m in meshes if m.triangles]
        if meshes_with_tris:
            all_meshes.extend(meshes_with_tris)

    if all_meshes:
        print(f"\n\n{'=' * 60}")
        print(f"Total meshes with triangles: {len(all_meshes)}")
        total_verts = sum(len(m.vertices) for m in all_meshes)
        total_tris = sum(len(m.triangles) for m in all_meshes)
        print(f"Total vertices: {total_verts}")
        print(f"Total triangles: {total_tris}")
        print("=" * 60)

        # Write individual OBJ files
        for i, mesh in enumerate(all_meshes[:20]):
            obj_path = output_dir / f"mesh_{i:03d}.obj"
            write_obj(mesh, obj_path)
            print(f"Wrote {mesh.name} to {obj_path.name}")

        # Write combined glTF
        gltf_path = output_dir / "hype_meshes.gltf"
        write_gltf(all_meshes[:50], gltf_path)
        print(f"\nWrote combined glTF to {gltf_path}")

        # Write combined OBJ
        combined_mesh = ExtractedMesh(name="combined", offset=0)
        vertex_offset = 0
        for mesh in all_meshes:
            combined_mesh.vertices.extend(mesh.vertices)
            for v0, v1, v2 in mesh.triangles:
                combined_mesh.triangles.append((v0 + vertex_offset, v1 + vertex_offset, v2 + vertex_offset))
            vertex_offset += len(mesh.vertices)

        combined_obj_path = output_dir / "all_meshes.obj"
        write_obj(combined_mesh, combined_obj_path)
        print(f"Wrote combined OBJ to {combined_obj_path}")
    else:
        print("\nNo meshes with triangles found.")


if __name__ == "__main__":
    main()
