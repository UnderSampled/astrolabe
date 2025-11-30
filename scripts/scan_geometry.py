#!/usr/bin/env python3
"""
Scan SNA data for actual GeometricObject structures by looking at pointer patterns.

Montreal PC engine GeometricObject format (from raymap GeometricObject.cs):
Offset 0:  u32 num_vertices
Offset 4:  ptr off_vertices
Offset 8:  ptr off_normals
Offset 12: ptr off_materials
Offset 16: skip 4 bytes
Offset 20: u32 num_elements
Offset 24: ptr off_element_types
Offset 28: ptr off_elements
Offset 32: skip 4 bytes
Offset 36: skip 4 bytes (for Montreal)
Offset 40: skip 4 bytes
Offset 44: skip 4 bytes
Offset 48: float sphereRadius
Offset 52: float sphereX
Offset 56: float sphereZ
Offset 60: float sphereY
"""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file
from astrolabe.openspace.relocation import RelocationTable


@dataclass
class GeoCandidate:
    block_offset: int
    num_vertices: int
    num_elements: int
    off_vertices: int
    vertex_data_offset: int
    sphere_radius: float
    sphere_center: tuple[float, float, float]
    vertices: list[tuple[float, float, float]]


def u32(data: bytes, off: int) -> int:
    return struct.unpack("<I", data[off:off + 4])[0]


def f32(data: bytes, off: int) -> float:
    return struct.unpack("<f", data[off:off + 4])[0]


def is_valid_float(v: float) -> bool:
    """Check if value is a valid, reasonable float."""
    if v != v:  # NaN
        return False
    if abs(v) > 50000:  # Too large for game geometry
        return False
    return True


def is_valid_coordinate(x: float, y: float, z: float) -> bool:
    """Check if all three coordinates are valid."""
    return is_valid_float(x) and is_valid_float(y) and is_valid_float(z)


def read_vertices(data: bytes, offset: int, count: int) -> list[tuple[float, float, float]]:
    """Read vertex array, converting from OpenSpace (X,Z,Y) to standard (X,Y,Z)."""
    vertices = []
    for i in range(count):
        pos = offset + i * 12
        if pos + 12 > len(data):
            break
        x = f32(data, pos)
        z = f32(data, pos + 4)  # OpenSpace Z is up
        y = f32(data, pos + 8)  # OpenSpace Y is forward
        if is_valid_coordinate(x, y, z):
            vertices.append((x, y, z))
        else:
            break
    return vertices


def find_geometry_objects(
    data: bytes,
    base_addr: int,
    rtb: RelocationTable | None = None,
) -> list[GeoCandidate]:
    """
    Search for GeometricObject structures in data.

    Uses pointer validation against base address to find valid structures.
    """
    candidates = []
    data_len = len(data)

    for i in range(0, data_len - 64, 4):
        num_vertices = u32(data, i)

        # Quick filter: num_vertices should be reasonable
        if num_vertices < 3 or num_vertices > 50000:
            continue

        off_vertices = u32(data, i + 4)
        off_normals = u32(data, i + 8)
        off_materials = u32(data, i + 12)

        # Check if off_vertices looks like a valid pointer into this block
        if off_vertices < base_addr:
            continue
        vertex_data_offset = off_vertices - base_addr
        if vertex_data_offset >= data_len:
            continue
        # Vertex offset should point to area after this structure
        if vertex_data_offset < i + 64:
            continue

        # off_normals should also be valid (or null)
        if off_normals != 0:
            if off_normals < base_addr:
                continue
            normal_offset = off_normals - base_addr
            if normal_offset >= data_len:
                continue

        # Read num_elements at offset 20
        num_elements = u32(data, i + 20)
        if num_elements > 500:  # Reasonable limit
            continue

        off_element_types = u32(data, i + 24)
        off_elements = u32(data, i + 28)

        # Validate element type pointer
        if off_element_types != 0:
            if off_element_types < base_addr:
                continue
            et_offset = off_element_types - base_addr
            if et_offset >= data_len:
                continue

        # Read sphere data at offset 48
        sphere_offset = i + 48
        if sphere_offset + 16 > data_len:
            continue

        sphere_radius = f32(data, sphere_offset)
        sphere_x = f32(data, sphere_offset + 4)
        sphere_z = f32(data, sphere_offset + 8)
        sphere_y = f32(data, sphere_offset + 12)

        # Filter: sphere values should be reasonable
        if not is_valid_float(sphere_radius):
            continue
        if sphere_radius < 0.1 or sphere_radius > 5000:
            continue
        if not is_valid_coordinate(sphere_x, sphere_y, sphere_z):
            continue

        # Try to read vertices at the pointer location
        vertices = read_vertices(data, vertex_data_offset, num_vertices)

        # Only accept if we got most of the expected vertices
        if len(vertices) < num_vertices * 0.5:
            continue
        if len(vertices) < 3:
            continue

        # Check that vertices have some variation (not all zeros)
        xs = [v[0] for v in vertices]
        ys = [v[1] for v in vertices]
        zs = [v[2] for v in vertices]
        x_range = max(xs) - min(xs) if xs else 0
        y_range = max(ys) - min(ys) if ys else 0
        z_range = max(zs) - min(zs) if zs else 0

        if x_range < 0.01 and y_range < 0.01 and z_range < 0.01:
            continue  # All vertices at same point

        candidates.append(GeoCandidate(
            block_offset=i,
            num_vertices=num_vertices,
            num_elements=num_elements,
            off_vertices=off_vertices,
            vertex_data_offset=vertex_data_offset,
            sphere_radius=sphere_radius,
            sphere_center=(sphere_x, sphere_y, sphere_z),
            vertices=vertices,
        ))

    return candidates


def write_obj(vertices: list[tuple[float, float, float]], path: Path) -> None:
    """Write vertices to OBJ file."""
    with open(path, "w") as f:
        f.write("# Extracted from Hype: The Time Quest\n")
        f.write(f"# {len(vertices)} vertices\n")
        for x, y, z in vertices:
            f.write(f"v {x} {y} {z}\n")


def main():
    # Try both Fix.sna (global data) and a level file
    files_to_scan = [
        Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/Fix.sna"),
        Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/brigand/brigand.sna"),
    ]

    all_candidates = []

    for sna_path in files_to_scan:
        print(f"\n{'='*60}")
        print(f"Loading {sna_path.name}...")
        print("=" * 60)

        sna = read_sna_file(sna_path)

        rtb_path = sna_path.with_suffix(".rtb")
        rtb = None
        try:
            rtb = RelocationTable.from_file(str(rtb_path), encrypted=False, compressed=True)
            print(f"Loaded RTB with {len(rtb.pointer_blocks)} blocks")
        except Exception as e:
            print(f"Warning: Failed to load RTB: {e}")

        for block in sna.blocks:
            if len(block.data) < 100:
                continue

            print(f"\nScanning block {block.module}/{block.block_id} "
                  f"(base=0x{block.base_in_memory:08X}, {len(block.data)} bytes)...")

            candidates = find_geometry_objects(block.data, block.base_in_memory, rtb)

            if candidates:
                print(f"  Found {len(candidates)} geometry objects:")
                for c in candidates[:10]:
                    print(f"    Offset 0x{c.block_offset:08X}: {c.num_vertices} verts "
                          f"(got {len(c.vertices)}), {c.num_elements} elements, "
                          f"radius={c.sphere_radius:.2f}")
                    print(f"      Center: ({c.sphere_center[0]:.1f}, {c.sphere_center[1]:.1f}, {c.sphere_center[2]:.1f})")
                    if c.vertices:
                        v = c.vertices[0]
                        print(f"      First vertex: ({v[0]:.2f}, {v[1]:.2f}, {v[2]:.2f})")
                    all_candidates.append(c)

    if all_candidates:
        # Combine all vertices for visualization
        all_vertices = []
        for c in all_candidates:
            all_vertices.extend(c.vertices)

        print(f"\n\n{'='*60}")
        print(f"Total geometry objects: {len(all_candidates)}")
        print(f"Total vertices: {len(all_vertices)}")
        print("=" * 60)

        obj_path = Path("/home/deck/code/astrolabe/output/all_geometry.obj")
        write_obj(all_vertices, obj_path)
        print(f"Wrote combined geometry to {obj_path}")

        # Also write first few objects separately
        for i, c in enumerate(all_candidates[:10]):
            obj_path = Path(f"/home/deck/code/astrolabe/output/geo_{i:03d}.obj")
            write_obj(c.vertices, obj_path)
            print(f"Wrote object {i} ({len(c.vertices)} verts, r={c.sphere_radius:.1f}) to {obj_path}")
    else:
        print("\nNo geometry found.")


if __name__ == "__main__":
    main()
