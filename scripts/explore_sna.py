#!/usr/bin/env python3
"""Explore SNA structure to understand data layout."""

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file
from astrolabe.openspace.relocation import RelocationTable


def hexdump(data: bytes, start: int = 0, length: int = 256) -> None:
    """Print a hex dump of data."""
    for i in range(0, min(length, len(data) - start), 16):
        offset = start + i
        hex_bytes = ' '.join(f'{data[offset + j]:02x}' for j in range(min(16, len(data) - offset)))
        ascii_bytes = ''.join(
            chr(data[offset + j]) if 32 <= data[offset + j] < 127 else '.'
            for j in range(min(16, len(data) - offset))
        )
        print(f'{offset:08x}  {hex_bytes:<48}  {ascii_bytes}')


def find_float_sequences(data: bytes, min_count: int = 20) -> list[tuple[int, int]]:
    """Find sequences of reasonable float values."""
    results = []
    i = 0

    while i < len(data) - 12:
        count = 0
        for j in range(min(1000, (len(data) - i) // 4)):
            val = struct.unpack("<f", data[i + j * 4:i + j * 4 + 4])[0]
            # Check if it's a reasonable coordinate value
            if val != val:  # NaN
                break
            if abs(val) > 50000:
                break
            if 0 < abs(val) < 1e-10:  # Denormalized
                break
            count += 1

        if count >= min_count:
            results.append((i, count))
            i += count * 4
        else:
            i += 4

    return results


def main():
    # Try Fix.sna which should have shared assets
    fix_sna_path = Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/Fix.sna")
    fix_rtb_path = Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/Fix.rtb")

    print("=" * 60)
    print("Reading Fix.sna...")
    print("=" * 60)

    fix_sna = read_sna_file(fix_sna_path)
    print(f"\nBlocks in Fix.sna:")
    for block in fix_sna.blocks:
        print(f"  Module {block.module:2d}, Block {block.block_id:2d}: "
              f"base=0x{block.base_in_memory:08X}, size={block.size:8d}, "
              f"data_len={len(block.data)}")

    # Try to load RTB
    print("\n" + "=" * 60)
    print("Reading Fix.rtb...")
    print("=" * 60)

    try:
        rtb = RelocationTable.from_file(str(fix_rtb_path))
        print(f"\nPointer blocks in RTB: {len(rtb.pointer_blocks)}")
        for pb in rtb.pointer_blocks[:10]:
            print(f"  Module {pb.module}, Block {pb.block_id}: {pb.count} pointers")
            if pb.pointers:
                for ptr in pb.pointers[:5]:
                    print(f"    Ptr at 0x{ptr.offset_in_memory:08X} -> "
                          f"module {ptr.module}, block {ptr.block_id}")
    except Exception as e:
        print(f"Failed to read RTB: {e}")

    # Look at the largest blocks for float patterns
    print("\n" + "=" * 60)
    print("Searching for float sequences...")
    print("=" * 60)

    for block in fix_sna.blocks:
        if len(block.data) < 1000:
            continue

        sequences = find_float_sequences(block.data, min_count=30)
        if sequences:
            print(f"\nBlock {block.module}/{block.block_id}:")
            for offset, count in sequences[:10]:
                # Read first few values
                vals = []
                for i in range(min(6, count)):
                    val = struct.unpack("<f", block.data[offset + i * 4:offset + i * 4 + 4])[0]
                    vals.append(val)
                print(f"  Offset 0x{offset:08X}: {count} floats")
                print(f"    First values: {vals}")

    # Show start of each major block
    print("\n" + "=" * 60)
    print("Block data previews...")
    print("=" * 60)

    for block in fix_sna.blocks:
        if len(block.data) < 256:
            continue
        print(f"\nBlock {block.module}/{block.block_id} (first 128 bytes):")
        hexdump(block.data, 0, 128)


if __name__ == "__main__":
    main()
