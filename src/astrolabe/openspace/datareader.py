"""Data reader for navigating OpenSpace SNA data with pointer resolution."""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from io import BytesIO
from pathlib import Path
from typing import Optional, Callable, Any

from .sna import SNAFile, SNAMemoryBlock, read_sna_file
from .relocation import RelocationTable, RelocationPointerList


@dataclass
class Pointer:
    """A resolved pointer in the SNA data."""

    offset: int
    """Offset within the combined data."""

    block: Optional[SNAMemoryBlock] = None
    """The block this pointer points to."""

    @property
    def is_null(self) -> bool:
        """Check if this is a null pointer."""
        return self.offset == 0 and self.block is None


class SNADataReader:
    """Reader for navigating SNA data with pointer resolution."""

    def __init__(
        self,
        sna: SNAFile,
        rtb: Optional[RelocationTable] = None,
    ):
        """
        Initialize the data reader.

        Args:
            sna: Parsed SNA file
            rtb: Relocation table for pointer resolution
        """
        self.sna = sna
        self.rtb = rtb

        # Build lookup tables
        self._blocks_by_key: dict[int, SNAMemoryBlock] = {}
        for block in sna.blocks:
            if not block.is_terminator:
                self._blocks_by_key[block.relocation_key] = block

        # Current reading position
        self._current_block: Optional[SNAMemoryBlock] = None
        self._position = 0
        self._data: bytes = b""

        # Base address for the current block (for pointer math)
        self.base_address = 0

    def set_block(self, module: int, block_id: int) -> bool:
        """
        Set the current reading block.

        Args:
            module: Module ID
            block_id: Block ID

        Returns:
            True if block was found and set
        """
        key = (module << 8) | block_id
        block = self._blocks_by_key.get(key)
        if block is None:
            return False

        self._current_block = block
        self._data = block.data
        self._position = 0
        self.base_address = block.base_in_memory
        return True

    def seek(self, offset: int) -> None:
        """Seek to an absolute offset within current block data."""
        self._position = offset

    def tell(self) -> int:
        """Get current position."""
        return self._position

    def skip(self, count: int) -> None:
        """Skip bytes."""
        self._position += count

    def _read_bytes(self, count: int) -> bytes:
        """Read raw bytes from current position."""
        if self._position + count > len(self._data):
            raise EOFError(f"Read beyond end of data: pos={self._position}, count={count}, len={len(self._data)}")
        result = self._data[self._position:self._position + count]
        self._position += count
        return result

    def read_u8(self) -> int:
        """Read unsigned 8-bit integer."""
        return self._read_bytes(1)[0]

    def read_i8(self) -> int:
        """Read signed 8-bit integer."""
        return struct.unpack("<b", self._read_bytes(1))[0]

    def read_u16(self) -> int:
        """Read unsigned 16-bit integer."""
        return struct.unpack("<H", self._read_bytes(2))[0]

    def read_i16(self) -> int:
        """Read signed 16-bit integer."""
        return struct.unpack("<h", self._read_bytes(2))[0]

    def read_u32(self) -> int:
        """Read unsigned 32-bit integer."""
        return struct.unpack("<I", self._read_bytes(4))[0]

    def read_i32(self) -> int:
        """Read signed 32-bit integer."""
        return struct.unpack("<i", self._read_bytes(4))[0]

    def read_float(self) -> float:
        """Read 32-bit float."""
        return struct.unpack("<f", self._read_bytes(4))[0]

    def read_string(self, length: int) -> str:
        """Read a fixed-length string."""
        data = self._read_bytes(length)
        return data.rstrip(b"\x00").decode("latin-1")

    def read_pointer(self) -> Pointer:
        """
        Read a 32-bit pointer and resolve it using the relocation table.

        Returns:
            Resolved Pointer object
        """
        raw_value = self.read_u32()

        if raw_value == 0:
            return Pointer(offset=0, block=None)

        # For now, return a pointer with just the raw offset
        # Full pointer resolution requires the relocation table
        return Pointer(offset=raw_value, block=self._current_block)

    def goto_pointer(self, pointer: Pointer) -> bool:
        """
        Move to a pointer's location.

        Args:
            pointer: The pointer to navigate to

        Returns:
            True if successful
        """
        if pointer.is_null:
            return False

        if pointer.block is not None:
            self._current_block = pointer.block
            self._data = pointer.block.data
            self.base_address = pointer.block.base_in_memory

        # Convert memory address to data offset
        if pointer.block is not None:
            offset = pointer.offset - pointer.block.base_in_memory
            if 0 <= offset < len(self._data):
                self._position = offset
                return True

        return False


def load_level(level_path: Path) -> tuple[SNAFile, Optional[RelocationTable]]:
    """
    Load a level's SNA and RTB files.

    Args:
        level_path: Path to the level's .sna file

    Returns:
        Tuple of (SNAFile, RelocationTable or None)
    """
    sna = read_sna_file(level_path)

    rtb_path = level_path.with_suffix(".rtb")
    rtb = None
    if rtb_path.exists():
        try:
            rtb = RelocationTable.from_file(str(rtb_path))
        except Exception as e:
            print(f"Warning: Failed to load RTB: {e}")

    return sna, rtb
