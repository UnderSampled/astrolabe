"""Pointer and memory management for OpenSpace data structures.

This module handles pointer resolution within the SNA memory blocks.
"""

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .sna import SNAFile, SNAMemoryBlock


@dataclass
class Pointer:
    """A pointer to a location within the game data.

    Pointers in OpenSpace SNA files reference locations as:
    - module: Which SNA block type (5=level data, 6=world, etc.)
    - block_id: Which block within that module
    - offset: Offset within that block's data
    """

    offset: int
    module: int = 0
    block_id: int = 0

    @property
    def is_null(self) -> bool:
        """Check if this is a null pointer."""
        return self.offset == 0

    def __repr__(self) -> str:
        return f"Pointer(0x{self.offset:08x}, mod={self.module}, blk={self.block_id})"


class SNADataReader:
    """Binary reader for data within an SNA block."""

    def __init__(self, block: "SNAMemoryBlock", sna: "SNAFile"):
        """Initialize reader with a memory block.

        Args:
            block: The memory block to read from.
            sna: Parent SNA file for pointer resolution.
        """
        self.block = block
        self.sna = sna
        self.data = block.data
        self.pos = 0
        self.base_address = block.base_in_memory

    def seek(self, offset: int) -> None:
        """Seek to an offset within the block."""
        # Convert absolute address to relative offset
        if offset >= self.base_address:
            self.pos = offset - self.base_address
        else:
            self.pos = offset

    def tell(self) -> int:
        """Return current position."""
        return self.pos

    def remaining(self) -> int:
        """Return remaining bytes."""
        return len(self.data) - self.pos

    def read_u8(self) -> int:
        """Read unsigned 8-bit integer."""
        val = self.data[self.pos]
        self.pos += 1
        return val

    def read_u16(self) -> int:
        """Read unsigned 16-bit integer (little endian)."""
        val = int.from_bytes(self.data[self.pos : self.pos + 2], "little")
        self.pos += 2
        return val

    def read_i16(self) -> int:
        """Read signed 16-bit integer (little endian)."""
        val = int.from_bytes(self.data[self.pos : self.pos + 2], "little", signed=True)
        self.pos += 2
        return val

    def read_u32(self) -> int:
        """Read unsigned 32-bit integer (little endian)."""
        val = int.from_bytes(self.data[self.pos : self.pos + 4], "little")
        self.pos += 4
        return val

    def read_i32(self) -> int:
        """Read signed 32-bit integer (little endian)."""
        val = int.from_bytes(self.data[self.pos : self.pos + 4], "little", signed=True)
        self.pos += 4
        return val

    def read_float(self) -> float:
        """Read 32-bit float (little endian)."""
        import struct

        val = struct.unpack("<f", self.data[self.pos : self.pos + 4])[0]
        self.pos += 4
        return val

    def read_pointer(self) -> Pointer:
        """Read a 32-bit pointer value."""
        offset = self.read_u32()
        return Pointer(offset=offset, module=self.block.module, block_id=self.block.block_id)

    def read_bytes(self, count: int) -> bytes:
        """Read raw bytes."""
        val = self.data[self.pos : self.pos + count]
        self.pos += count
        return val

    def skip(self, count: int) -> None:
        """Skip bytes."""
        self.pos += count

    def align(self, boundary: int) -> None:
        """Align position to boundary."""
        if self.pos % boundary != 0:
            self.pos += boundary - (self.pos % boundary)
