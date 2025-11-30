#!/usr/bin/env python3
"""Analyze the actual structure of geometry data in Hype SNA files.

This script examines the raw data around known vertex arrays to understand
the actual GeometricObject structure format.
"""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file


def u16(data: bytes, off: int) -> int:
    if off + 2 > len(data):
        return 0
    return struct.unpack("<H", data[off:off + 2])[0]


def i16(data: bytes, off: int) -> int:
    if off + 2 > len(data):
        return 0
    return struct.unpack("<h", data[off:off + 2])[0]


def u32(data: bytes, off: int) -> int:
    if off + 4 > len(data):
        return 0
    return struct.unpack("<I", data[off:off + 4])[0]


def f32(data: bytes, off: int) -> float:
    if off + 4 > len(data):
        return 0.0
    return struct.unpack("<f", data[off:off + 4])[0]


def is_valid_float(v: float) -> bool:
    if v != v:  # NaN
        return False
    if abs(v) > 50000:
        return False
    return True


def hexdump(data: bytes, start: int = 0, length: int = 128) -> None:
    """Print hex dump."""
    for i in range(0, min(length, len(data) - start), 16):
        offset = start + i
        if offset >= len(data):
            break
        hex_bytes = ' '.join(f'{data[offset + j]:02x}' for j in range(min(16, len(data) - offset)))
        ascii_bytes = ''.join(
            chr(data[offset + j]) if 32 <= data[offset + j] < 127 else '.'
            for j in range(min(16, len(data) - offset))
        )
        print(f'{offset:08x}  {hex_bytes:<48}  {ascii_bytes}')


def find_vertex_arrays(data: bytes, min_count: int = 20) -> list[tuple[int, int]]:
    """Find sequences of valid 3D float coordinates."""
    results = []
    i = 0
    data_len = len(data)

    while i < data_len - 12:
        # Try reading vertex triplets
        count = 0
        for j in range(min(5000, (data_len - i) // 12)):
            pos = i + j * 12
            x = f32(data, pos)
            y = f32(data, pos + 4)
            z = f32(data, pos + 8)
            if is_valid_float(x) and is_valid_float(y) and is_valid_float(z):
                # Check it's not all zeros
                if abs(x) > 0.001 or abs(y) > 0.001 or abs(z) > 0.001:
                    count += 1
                else:
                    break
            else:
                break

        if count >= min_count:
            results.append((i, count))
            i += count * 12
        else:
            i += 4

    return results


def find_pointer_to_offset(data: bytes, base_addr: int, target_offset: int) -> list[int]:
    """Find all u32 values that point to the target offset."""
    target_addr = base_addr + target_offset
    results = []
    for i in range(0, len(data) - 4, 4):
        val = u32(data, i)
        if val == target_addr:
            results.append(i)
    return results


def find_u32_value(data: bytes, target_value: int) -> list[int]:
    """Find all occurrences of a specific u32 value."""
    results = []
    for i in range(0, len(data) - 4, 4):
        if u32(data, i) == target_value:
            results.append(i)
    return results


def main():
    sna_path = Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/brigand/brigand.sna")

    print(f"Loading {sna_path.name}...")
    sna = read_sna_file(sna_path)

    # Find the main data block (6/2 is typically geometry)
    for block in sna.blocks:
        if block.module == 6 and block.block_id == 2:
            data = block.data
            base_addr = block.base_in_memory
            print(f"\nAnalyzing block 6/2 (base=0x{base_addr:08X}, {len(data)} bytes)")

            # Find some vertex arrays
            print("\nFinding vertex arrays...")
            vertex_arrays = find_vertex_arrays(data, min_count=30)
            print(f"Found {len(vertex_arrays)} vertex arrays")

            # Analyze first few vertex arrays
            for va_offset, va_count in vertex_arrays[:5]:
                print(f"\n{'='*60}")
                print(f"Vertex array at offset 0x{va_offset:08X} ({va_count} vertices)")
                print(f"Memory address: 0x{base_addr + va_offset:08X}")

                # Show first few vertices
                print("\nFirst 3 vertices:")
                for i in range(min(3, va_count)):
                    pos = va_offset + i * 12
                    x, y, z = f32(data, pos), f32(data, pos+4), f32(data, pos+8)
                    print(f"  {i}: ({x:.2f}, {y:.2f}, {z:.2f})")

                # Find pointers to this vertex array
                print(f"\nSearching for pointers to 0x{base_addr + va_offset:08X}...")
                pointers = find_pointer_to_offset(data, base_addr, va_offset)
                print(f"Found {len(pointers)} pointers")

                for ptr_off in pointers[:3]:
                    print(f"\n  Pointer at offset 0x{ptr_off:08X}")
                    # Show context around the pointer (likely GeometricObject structure)
                    print("  Context before pointer:")
                    context_start = max(0, ptr_off - 16)
                    hexdump(data, context_start, 64)

                    # Try to interpret as GeometricObject
                    # If this is off_vertices at offset 4, then offset 0 should be num_vertices
                    struct_start = ptr_off - 4  # Assume ptr is at offset 4 (off_vertices)
                    if struct_start >= 0:
                        print(f"\n  If pointer is off_vertices, structure would start at 0x{struct_start:08X}:")
                        num_v = u32(data, struct_start)
                        print(f"    Offset 0 (num_vertices?): {num_v}")
                        if num_v == va_count or abs(num_v - va_count) < 10:
                            print(f"    *** MATCH! num_vertices = {num_v}, actual count = {va_count}")

                            # Continue analyzing the structure
                            for offset in range(8, 64, 4):
                                val = u32(data, struct_start + offset)
                                fval = f32(data, struct_start + offset)
                                is_ptr = base_addr <= val < base_addr + len(data)
                                if is_ptr:
                                    rel_off = val - base_addr
                                    print(f"    Offset {offset}: 0x{val:08X} (ptr to 0x{rel_off:08X})")
                                elif is_valid_float(fval) and 0 < abs(fval) < 10000:
                                    print(f"    Offset {offset}: {fval:.4f} (float) or {val}")
                                else:
                                    print(f"    Offset {offset}: {val} (0x{val:08X})")

            # Also look for triangle index patterns
            print(f"\n\n{'='*60}")
            print("Looking for triangle index arrays...")
            print("='*60}")

            # Look for sequences of small positive i16 values
            triangle_candidates = []
            i = 0
            while i < len(data) - 6:
                count = 0
                for j in range(min(5000, (len(data) - i) // 6)):
                    pos = i + j * 6
                    v0 = i16(data, pos)
                    v1 = i16(data, pos + 2)
                    v2 = i16(data, pos + 4)
                    # Triangle indices should be small positive values
                    if 0 <= v0 < 5000 and 0 <= v1 < 5000 and 0 <= v2 < 5000:
                        count += 1
                    else:
                        break
                if count >= 10:
                    # Check it's not just zeros
                    pos = i
                    non_zero = sum(1 for j in range(count) for k in [i16(data, i + j*6 + k*2) for k in range(3)] if k > 0)
                    if non_zero > count:
                        triangle_candidates.append((i, count))
                        i += count * 6
                    else:
                        i += 4
                else:
                    i += 2

            print(f"Found {len(triangle_candidates)} potential triangle arrays")
            for tri_off, tri_count in triangle_candidates[:10]:
                print(f"\n  Triangle array at 0x{tri_off:08X}: {tri_count} triangles")
                # Show first few
                for j in range(min(3, tri_count)):
                    pos = tri_off + j * 6
                    v0, v1, v2 = i16(data, pos), i16(data, pos+2), i16(data, pos+4)
                    print(f"    {j}: ({v0}, {v1}, {v2})")

                # Find pointers to this
                ptrs = find_pointer_to_offset(data, base_addr, tri_off)
                if ptrs:
                    print(f"    Found {len(ptrs)} pointers to this array")
                    for ptr_off in ptrs[:2]:
                        # Look at context
                        print(f"      Pointer at 0x{ptr_off:08X}")


if __name__ == "__main__":
    main()
