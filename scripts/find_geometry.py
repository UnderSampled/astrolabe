#!/usr/bin/env python3
"""Find and extract GeometricObject structures from Hype SNA files."""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Optional

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file


@dataclass
class GeometryCandidate:
    """A potential GeometricObject found in the data."""
    offset: int
    num_vertices: int
    off_vertices: int
    off_normals: int
    off_materials: int
    num_elements: int
    off_element_types: int
    off_elements: int
    sphere_radius: float
    sphere_center: tuple[float, float, float]
    vertices: list[tuple[float, float, float]]


def read_u32(data: bytes, offset: int) -> int:
    if offset + 4 > len(data):
        return 0
    return struct.unpack("<I", data[offset:offset + 4])[0]


def read_float(data: bytes, offset: int) -> float:
    if offset + 4 > len(data):
        return 0.0
    return struct.unpack("<f", data[offset:offset + 4])[0]


def looks_like_pointer(val: int, base_addr: int, data_len: int) -> bool:
    """Check if a value looks like a valid pointer."""
    if val == 0:
        return True  # Null pointer is valid
    # Pointer should be in the data range
    offset = val - base_addr
    return 0 <= offset < data_len


def is_reasonable_vertex(x: float, y: float, z: float) -> bool:
    """Check if coordinates are reasonable vertex values."""
    # Reasonable game coordinates (not denormalized floats)
    for v in (x, y, z):
        if v != v:  # NaN check
            return False
        if abs(v) > 50000:  # Too large
            return False
        if 0 < abs(v) < 1e-20:  # Denormalized
            return False
    return True


def extract_vertices(data: bytes, offset: int, count: int) -> list[tuple[float, float, float]]:
    """Extract vertex data from a known location (swapping Y/Z for glTF)."""
    vertices = []
    for i in range(count):
        pos = offset + i * 12
        if pos + 12 > len(data):
            break
        x = read_float(data, pos)
        z = read_float(data, pos + 4)  # OpenSpace: Z is up, Y is forward
        y = read_float(data, pos + 8)
        if is_reasonable_vertex(x, y, z):
            vertices.append((x, y, z))
        else:
            break  # Stop at first invalid vertex
    return vertices


def find_geometric_objects(
    data: bytes,
    base_addr: int,
    min_vertices: int = 3,
) -> list[GeometryCandidate]:
    """
    Scan for GeometricObject structures using Montreal engine format.

    Montreal format:
    - u32 num_vertices
    - ptr off_vertices
    - ptr off_normals
    - ptr off_materials
    - u32 unk
    - u32 num_elements
    - ptr off_element_types
    - ptr off_elements
    - skip 16 bytes
    - float sphere_radius
    - float sphere_x, sphere_z, sphere_y
    """
    candidates = []
    data_len = len(data)

    # Scan through data looking for valid patterns
    for i in range(0, data_len - 64, 4):
        num_vertices = read_u32(data, i)

        # Check if num_vertices is reasonable
        if num_vertices < min_vertices or num_vertices > 100000:
            continue

        off_vertices = read_u32(data, i + 4)
        off_normals = read_u32(data, i + 8)
        off_materials = read_u32(data, i + 12)

        # Check if pointers look valid
        if not looks_like_pointer(off_vertices, base_addr, data_len):
            continue
        if not looks_like_pointer(off_normals, base_addr, data_len):
            continue

        # Skip to num_elements (after unk field)
        num_elements = read_u32(data, i + 20)

        # Check if num_elements is reasonable
        if num_elements > 1000:
            continue

        off_element_types = read_u32(data, i + 24)
        off_elements = read_u32(data, i + 28)

        # Read sphere data (after some skipped fields)
        sphere_offset = i + 32 + 16  # Skip additional unknown fields
        if sphere_offset + 16 > data_len:
            continue

        sphere_radius = read_float(data, sphere_offset)
        sphere_x = read_float(data, sphere_offset + 4)
        sphere_z = read_float(data, sphere_offset + 8)
        sphere_y = read_float(data, sphere_offset + 12)

        # Check if sphere values are reasonable
        if not is_reasonable_vertex(sphere_x, sphere_y, sphere_z):
            continue
        if sphere_radius < 0 or sphere_radius > 10000:
            continue

        # Try to read vertices
        if off_vertices != 0:
            vert_offset = off_vertices - base_addr
            if 0 <= vert_offset < data_len:
                vertices = extract_vertices(data, vert_offset, num_vertices)

                # Only accept if we got a reasonable number of vertices
                if len(vertices) >= min_vertices:
                    candidates.append(GeometryCandidate(
                        offset=i,
                        num_vertices=num_vertices,
                        off_vertices=off_vertices,
                        off_normals=off_normals,
                        off_materials=off_materials,
                        num_elements=num_elements,
                        off_element_types=off_element_types,
                        off_elements=off_elements,
                        sphere_radius=sphere_radius,
                        sphere_center=(sphere_x, sphere_y, sphere_z),
                        vertices=vertices,
                    ))

    return candidates


def write_obj(vertices: list[tuple[float, float, float]], path: Path) -> None:
    """Write vertices to an OBJ file."""
    with open(path, "w") as f:
        f.write("# Extracted from Hype: The Time Quest\n")
        for x, y, z in vertices:
            f.write(f"v {x} {y} {z}\n")


def main():
    sna_path = Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/brigand/brigand.sna")

    print(f"Reading {sna_path}...")
    sna = read_sna_file(sna_path)

    all_vertices = []

    for block in sna.blocks:
        if len(block.data) < 100:
            continue

        print(f"\nScanning block {block.module}/{block.block_id} "
              f"(base=0x{block.base_in_memory:08X}, {len(block.data)} bytes)...")

        candidates = find_geometric_objects(
            block.data,
            block.base_in_memory,
            min_vertices=10,
        )

        if candidates:
            print(f"  Found {len(candidates)} geometry candidates:")
            for c in candidates[:20]:  # Show first 20
                print(f"    Offset 0x{c.offset:08X}: {c.num_vertices} verts, "
                      f"{c.num_elements} elements, radius={c.sphere_radius:.2f}")
                print(f"      Center: ({c.sphere_center[0]:.2f}, "
                      f"{c.sphere_center[1]:.2f}, {c.sphere_center[2]:.2f})")
                if c.vertices:
                    print(f"      Sample vertices: {c.vertices[:3]}")
                    all_vertices.extend(c.vertices)

    if all_vertices:
        print(f"\n\nTotal vertices extracted: {len(all_vertices)}")
        obj_path = Path("/home/deck/code/astrolabe/output/geometry.obj")
        write_obj(all_vertices, obj_path)
        print(f"Wrote to {obj_path}")
    else:
        print("\nNo geometry found.")


if __name__ == "__main__":
    main()
