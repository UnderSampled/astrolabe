"""Tests for SNA file parsing."""

from pathlib import Path

import pytest

from astrolabe.openspace.sna import SNAReader, read_sna_file
from astrolabe.openspace.relocation import read_relocation_table


GAMEDATA_PATH = Path(__file__).parent.parent / "extracted" / "Gamedata"
LEVELS_PATH = GAMEDATA_PATH / "World" / "Levels"


@pytest.fixture
def tour_sna_path() -> Path:
    """Path to tour level SNA file."""
    return LEVELS_PATH / "tour" / "tour.sna"


@pytest.fixture
def tour_rtb_path() -> Path:
    """Path to tour level RTB file."""
    return LEVELS_PATH / "tour" / "tour.rtb"


@pytest.mark.skipif(
    not (LEVELS_PATH / "tour" / "tour.sna").exists(),
    reason="Game files not extracted",
)
class TestSNAReader:
    """Test SNA file reading."""

    def test_read_sna(self, tour_sna_path: Path) -> None:
        """Test reading an SNA file."""
        sna = read_sna_file(tour_sna_path)
        assert sna.name == "tour"
        assert len(sna.blocks) > 0
        # Last block should be terminator
        assert sna.blocks[-1].is_terminator

    def test_block_structure(self, tour_sna_path: Path) -> None:
        """Test that blocks have valid structure."""
        sna = read_sna_file(tour_sna_path)
        for block in sna.blocks:
            if not block.is_terminator:
                # Non-terminator blocks should have data
                assert block.size >= 0
                assert block.module >= 0
                assert block.block_id >= 0
                assert block.relocation_key == (block.module << 8) | block.block_id

    def test_read_rtb(self, tour_rtb_path: Path) -> None:
        """Test reading a relocation table."""
        rtb = read_relocation_table(tour_rtb_path)
        assert len(rtb.pointer_blocks) > 0

    def test_rtb_structure(self, tour_rtb_path: Path) -> None:
        """Test that RTB has valid structure."""
        rtb = read_relocation_table(tour_rtb_path)
        for plist in rtb.pointer_blocks:
            assert plist.module >= 0
            assert plist.block_id >= 0
            for ptr in plist.pointers:
                assert ptr.offset_in_memory >= 0
                assert ptr.module >= 0
                assert ptr.block_id >= 0


if __name__ == "__main__":
    # Quick test run
    if (LEVELS_PATH / "tour" / "tour.sna").exists():
        print("Reading tour.sna...")
        sna = read_sna_file(LEVELS_PATH / "tour" / "tour.sna")
        print(f"  Name: {sna.name}")
        print(f"  Blocks: {len(sna.blocks)}")
        print(f"  Total data size: {len(sna.data)} bytes")
        for block in sna.blocks[:5]:
            print(
                f"    Block ({block.module}, {block.block_id}): "
                f"base=0x{block.base_in_memory:08x}, size={block.size}"
            )
        if len(sna.blocks) > 5:
            print(f"    ... and {len(sna.blocks) - 5} more blocks")

        print("\nReading tour.rtb...")
        rtb = read_relocation_table(LEVELS_PATH / "tour" / "tour.rtb")
        print(f"  Pointer blocks: {len(rtb.pointer_blocks)}")
        total_pointers = sum(len(plist.pointers) for plist in rtb.pointer_blocks)
        print(f"  Total pointers: {total_pointers}")
    else:
        print("Game files not found. Run extraction first.")
