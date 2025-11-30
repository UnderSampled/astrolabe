#!/usr/bin/env python3
"""
Find vertex arrays by looking for sequences of reasonable 3D float coordinates.

This script scans SNA data blocks for patterns that look like vertex data:
- Sequences of floats in reasonable ranges (e.g., -5000 to 5000)
- Not denormalized (not extremely small non-zero values)
- Consistent patterns (X,Y,Z triplets)
"""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file


@dataclass
class VertexArray:
    offset: int
    count: int
    vertices: list[tuple[float, float, float]]
    bounds: tuple[tuple[float, float, float], tuple[float, float, float]]


def f32(data: bytes, off: int) -> float:
    return struct.unpack("<f", data[off:off + 4])[0]


def is_good_vertex_coord(v: float) -> bool:
    """Check if a float value looks like a valid vertex coordinate."""
    if v != v:  # NaN
        return False
    if abs(v) > 10000:  # Too large for typical game geometry
        return False
    # Check for denormalized numbers (very small but non-zero)
    if v != 0 and abs(v) < 1e-10:
        return False
    return True


def is_good_vertex(x: float, y: float, z: float) -> bool:
    """Check if XYZ triplet is a valid vertex."""
    return is_good_vertex_coord(x) and is_good_vertex_coord(y) and is_good_vertex_coord(z)


def find_vertex_sequences(
    data: bytes,
    min_vertices: int = 10,
    max_gap: int = 100,
) -> list[VertexArray]:
    """
    Find sequences of valid vertex-like float triplets.

    Returns list of vertex arrays found.
    """
    results = []
    data_len = len(data)
    i = 0

    while i < data_len - 12:
        # Try reading a vertex triplet
        try:
            x = f32(data, i)
            y = f32(data, i + 4)
            z = f32(data, i + 8)
        except struct.error:
            i += 4
            continue

        if not is_good_vertex(x, y, z):
            i += 4
            continue

        # Found a potentially valid vertex, try to count consecutive vertices
        start_offset = i
        vertices = []
        min_x = max_x = x
        min_y = max_y = y
        min_z = max_z = z

        while i < data_len - 12:
            try:
                x = f32(data, i)
                y = f32(data, i + 4)
                z = f32(data, i + 8)
            except struct.error:
                break

            if not is_good_vertex(x, y, z):
                break

            vertices.append((x, y, z))
            min_x, max_x = min(min_x, x), max(max_x, x)
            min_y, max_y = min(min_y, y), max(max_y, y)
            min_z, max_z = min(min_z, z), max(max_z, z)
            i += 12

            # Stop if we've found a very long sequence (probably not real vertex data)
            if len(vertices) > 10000:
                break

        # Check if this looks like real vertex data
        if len(vertices) >= min_vertices:
            # Check that there's some variation in the coordinates
            x_range = max_x - min_x
            y_range = max_y - min_y
            z_range = max_z - min_z

            # Should have meaningful variation in at least 2 dimensions
            meaningful_dims = sum([
                x_range > 0.1,
                y_range > 0.1,
                z_range > 0.1,
            ])

            if meaningful_dims >= 2:
                results.append(VertexArray(
                    offset=start_offset,
                    count=len(vertices),
                    vertices=vertices,
                    bounds=((min_x, min_y, min_z), (max_x, max_y, max_z)),
                ))

        # Move to next position (skip what we processed)
        if i == start_offset:
            i += 4

    return results


def write_obj(vertices: list[tuple[float, float, float]], path: Path) -> None:
    """Write vertices to OBJ file."""
    with open(path, "w") as f:
        f.write("# Extracted from Hype: The Time Quest\n")
        f.write(f"# {len(vertices)} vertices\n")
        for x, y, z in vertices:
            f.write(f"v {x} {y} {z}\n")


def main():
    files_to_scan = [
        Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/Fix.sna"),
        Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/brigand/brigand.sna"),
    ]

    all_arrays = []

    for sna_path in files_to_scan:
        print(f"\n{'='*60}")
        print(f"Loading {sna_path.name}...")
        print("=" * 60)

        sna = read_sna_file(sna_path)

        for block in sna.blocks:
            if len(block.data) < 1000:
                continue

            print(f"\nScanning block {block.module}/{block.block_id} "
                  f"(base=0x{block.base_in_memory:08X}, {len(block.data)} bytes)...")

            arrays = find_vertex_sequences(block.data, min_vertices=20)

            # Filter to interesting arrays (not just zeros)
            interesting = []
            for arr in arrays:
                # Check if most vertices are non-zero
                nonzero = sum(1 for v in arr.vertices if abs(v[0]) > 0.01 or abs(v[1]) > 0.01 or abs(v[2]) > 0.01)
                if nonzero > len(arr.vertices) * 0.5:
                    interesting.append(arr)

            if interesting:
                print(f"  Found {len(interesting)} potential vertex arrays:")
                for arr in interesting[:10]:
                    (min_x, min_y, min_z), (max_x, max_y, max_z) = arr.bounds
                    print(f"    Offset 0x{arr.offset:08X}: {arr.count} vertices")
                    print(f"      Bounds: ({min_x:.1f},{min_y:.1f},{min_z:.1f}) to ({max_x:.1f},{max_y:.1f},{max_z:.1f})")
                    if arr.vertices:
                        v = arr.vertices[0]
                        print(f"      First: ({v[0]:.2f}, {v[1]:.2f}, {v[2]:.2f})")
                    all_arrays.append((sna_path.stem, block.module, block.block_id, arr))

    if all_arrays:
        print(f"\n\n{'='*60}")
        print(f"Total vertex arrays found: {len(all_arrays)}")
        print("=" * 60)

        # Write the most promising arrays to OBJ files
        total_vertices = 0
        all_vertices = []

        for i, (sna_name, module, block_id, arr) in enumerate(all_arrays[:20]):
            # Convert from OpenSpace coordinates (X, Z, Y) to standard (X, Y, Z)
            converted = [(x, z, y) for x, y, z in arr.vertices]
            all_vertices.extend(converted)
            total_vertices += len(converted)

            obj_path = Path(f"/home/deck/code/astrolabe/output/verts_{sna_name}_{module}_{block_id}_{arr.offset:08X}.obj")
            write_obj(converted, obj_path)
            print(f"Wrote {len(arr.vertices)} vertices to {obj_path.name}")

        # Combined file
        if all_vertices:
            obj_path = Path("/home/deck/code/astrolabe/output/all_vertices.obj")
            write_obj(all_vertices, obj_path)
            print(f"\nWrote combined {total_vertices} vertices to {obj_path}")
    else:
        print("\nNo vertex arrays found.")


if __name__ == "__main__":
    main()
