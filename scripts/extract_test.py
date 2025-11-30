#!/usr/bin/env python3
"""Test script to extract geometry from Hype SNA files."""

import struct
import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file


def scan_for_floats(data: bytes, start: int, count: int) -> list[float]:
    """Read a sequence of floats from data."""
    floats = []
    for i in range(count):
        offset = start + i * 4
        if offset + 4 > len(data):
            break
        val = struct.unpack("<f", data[offset:offset + 4])[0]
        floats.append(val)
    return floats


def looks_like_vertex(f: float) -> bool:
    """Check if a float value looks like a reasonable vertex coordinate."""
    return -10000.0 < f < 10000.0


def find_vertex_arrays(data: bytes, min_vertices: int = 10) -> list[tuple[int, int]]:
    """
    Scan data for sequences that look like vertex arrays.

    Returns list of (offset, count) tuples.
    """
    results = []
    i = 0

    while i < len(data) - 12:
        # Try reading 3 floats as a potential vertex
        try:
            x, y, z = struct.unpack("<fff", data[i:i + 12])
        except struct.error:
            i += 4
            continue

        # Check if this looks like a valid vertex
        if looks_like_vertex(x) and looks_like_vertex(y) and looks_like_vertex(z):
            # Try to count how many consecutive valid vertices we have
            count = 0
            for j in range(500):  # Limit search
                offset = i + j * 12
                if offset + 12 > len(data):
                    break
                try:
                    vx, vy, vz = struct.unpack("<fff", data[offset:offset + 12])
                except struct.error:
                    break

                if looks_like_vertex(vx) and looks_like_vertex(vy) and looks_like_vertex(vz):
                    count += 1
                else:
                    break

            if count >= min_vertices:
                results.append((i, count))
                i += count * 12  # Skip past this array
            else:
                i += 4
        else:
            i += 4

    return results


def extract_vertices(data: bytes, offset: int, count: int) -> list[tuple[float, float, float]]:
    """Extract vertex data from a known location."""
    vertices = []
    for i in range(count):
        pos = offset + i * 12
        if pos + 12 > len(data):
            break
        x, y, z = struct.unpack("<fff", data[pos:pos + 12])
        vertices.append((x, z, y))  # Swap Y/Z for coordinate system conversion
    return vertices


def main():
    # Path to a level SNA file
    sna_path = Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/brigand/brigand.sna")

    print(f"Reading {sna_path}...")
    sna = read_sna_file(sna_path)

    print(f"\nFound {len(sna.blocks)} blocks:")
    for block in sna.blocks:
        print(f"  Module {block.module}, Block {block.block_id}: "
              f"size={block.size}, data_len={len(block.data)}")

    # Try to find vertex arrays in each block with data
    all_vertices = []

    for block in sna.blocks:
        if len(block.data) < 100:
            continue

        print(f"\n\nScanning block {block.module}/{block.block_id} "
              f"({len(block.data)} bytes)...")

        vertex_arrays = find_vertex_arrays(block.data)

        if vertex_arrays:
            print(f"  Found {len(vertex_arrays)} potential vertex arrays:")
            for offset, count in vertex_arrays[:10]:  # Show first 10
                print(f"    Offset 0x{offset:08X}: {count} vertices")

                if count >= 20 and count <= 1000:
                    vertices = extract_vertices(block.data, offset, count)
                    all_vertices.extend(vertices)

                    # Show first few vertices as sample
                    print(f"      Sample: {vertices[:3]}")

    if all_vertices:
        print(f"\n\nTotal vertices found: {len(all_vertices)}")

        # Write to simple OBJ file for quick testing
        obj_path = Path("/home/deck/code/astrolabe/output/test_vertices.obj")
        with open(obj_path, "w") as f:
            f.write("# Test vertex extraction\n")
            for x, y, z in all_vertices[:5000]:  # Limit to 5000
                f.write(f"v {x} {y} {z}\n")
        print(f"Wrote vertices to {obj_path}")
    else:
        print("\nNo vertex arrays found.")


if __name__ == "__main__":
    main()
