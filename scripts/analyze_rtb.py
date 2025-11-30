#!/usr/bin/env python3
"""Analyze RTB relocation table to understand cross-block pointer relationships."""

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from astrolabe.openspace.sna import read_sna_file
from astrolabe.openspace.relocation import RelocationTable


def main():
    sna_path = Path("/home/deck/code/astrolabe/output/Gamedata/World/Levels/brigand/brigand.sna")
    rtb_path = sna_path.with_suffix(".rtb")

    print(f"Loading {sna_path.name}...")
    sna = read_sna_file(sna_path)

    print(f"\nSNA Blocks:")
    for block in sna.blocks:
        print(f"  Module {block.module:2d}, Block {block.block_id:2d}: "
              f"base=0x{block.base_in_memory:08X}, size={len(block.data)} bytes")

    print(f"\nLoading {rtb_path.name}...")
    try:
        # Try with encrypted=False (Hype doesn't use XOR encryption on RTB)
        rtb = RelocationTable.from_file(str(rtb_path), encrypted=False, compressed=True)
        print(f"RTB loaded successfully with {len(rtb.pointer_blocks)} pointer blocks")

        # Analyze pointer relationships
        print("\nPointer Blocks:")
        for pb in rtb.pointer_blocks:
            print(f"\n  Module {pb.module}, Block {pb.block_id}: {pb.count} pointers")

            # Find the corresponding SNA block
            sna_block = None
            for block in sna.blocks:
                if block.module == pb.module and block.block_id == pb.block_id:
                    sna_block = block
                    break

            if sna_block and pb.pointers:
                print(f"    SNA block base: 0x{sna_block.base_in_memory:08X}")

                # Show first few pointers
                for i, ptr in enumerate(pb.pointers[:20]):
                    # Calculate offset within block
                    offset_in_block = ptr.offset_in_memory - sna_block.base_in_memory
                    if 0 <= offset_in_block < len(sna_block.data):
                        # Read the pointer value from the data
                        ptr_value = struct.unpack("<I", sna_block.data[offset_in_block:offset_in_block+4])[0]

                        # Find target block
                        target_block_info = f"-> module {ptr.module}, block {ptr.block_id}"
                        for target in sna.blocks:
                            if target.module == ptr.module and target.block_id == ptr.block_id:
                                target_offset = ptr_value - target.base_in_memory
                                if 0 <= target_offset < len(target.data):
                                    target_block_info += f" @ offset 0x{target_offset:08X}"
                                break

                        print(f"      [{i:4d}] Offset 0x{offset_in_block:08X}: "
                              f"value=0x{ptr_value:08X} {target_block_info}")

                if len(pb.pointers) > 20:
                    print(f"      ... and {len(pb.pointers) - 20} more pointers")

        # Look for patterns - which blocks point to which
        print("\n\n" + "=" * 60)
        print("Cross-block pointer summary:")
        print("=" * 60)

        cross_block = {}
        for pb in rtb.pointer_blocks:
            for ptr in pb.pointers:
                key = (pb.module, pb.block_id, ptr.module, ptr.block_id)
                cross_block[key] = cross_block.get(key, 0) + 1

        for (src_mod, src_blk, dst_mod, dst_blk), count in sorted(cross_block.items()):
            print(f"  Module {src_mod}/Block {src_blk} -> Module {dst_mod}/Block {dst_blk}: {count} pointers")

    except Exception as e:
        print(f"Failed to load RTB: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()
