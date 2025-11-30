"""Relocation table (RTB/RTP/RTT) parser for OpenSpace.

Relocation tables map pointers between memory blocks in SNA files.
Note: Montreal engine (Hype) does NOT encrypt relocation files.
"""

from dataclasses import dataclass, field
from pathlib import Path

from .lzo import decompress


@dataclass
class RelocationPointerInfo:
    """Information about a single pointer relocation."""

    offset_in_memory: int  # Where the pointer is located
    module: int  # Target module
    block_id: int  # Target block

    @property
    def target_key(self) -> int:
        """Get the relocation key for the target block."""
        return (self.module << 8) | self.block_id


@dataclass
class RelocationPointerList:
    """List of pointers for a specific memory block."""

    module: int
    block_id: int
    pointers: list[RelocationPointerInfo] = field(default_factory=list)

    @property
    def source_key(self) -> int:
        """Get the relocation key for this block."""
        return (self.module << 8) | self.block_id


@dataclass
class RelocationTable:
    """Parsed relocation table containing pointer lists."""

    pointer_blocks: list[RelocationPointerList] = field(default_factory=list)

    def get_list_for_block(
        self, module: int, block_id: int
    ) -> RelocationPointerList | None:
        """Find pointer list for a specific block."""
        key = (module << 8) | block_id
        for plist in self.pointer_blocks:
            if plist.source_key == key:
                return plist
        return None


class RelocationTableReader:
    """Reader for relocation table files (RTB, RTP, RTT, etc.)."""

    def __init__(self, data: bytes, compressed: bool = True):
        """Initialize with raw file data.

        Args:
            data: Raw relocation table file bytes.
            compressed: Whether pointer blocks are LZO compressed.
        """
        # Montreal engine (Hype) does not encrypt relocation files
        self.data = data
        self.compressed = compressed
        self.pos = 0

    def _read_u8(self) -> int:
        """Read unsigned 8-bit integer."""
        val = self.data[self.pos]
        self.pos += 1
        return val

    def _read_u16(self) -> int:
        """Read unsigned 16-bit integer (little endian)."""
        val = int.from_bytes(self.data[self.pos : self.pos + 2], "little")
        self.pos += 2
        return val

    def _read_u32(self) -> int:
        """Read unsigned 32-bit integer (little endian)."""
        val = int.from_bytes(self.data[self.pos : self.pos + 4], "little")
        self.pos += 4
        return val

    def _read_bytes(self, count: int) -> bytes:
        """Read raw bytes."""
        val = self.data[self.pos : self.pos + count]
        self.pos += count
        return val

    def _read_pointer_block(
        self, data: bytes, count: int
    ) -> list[RelocationPointerInfo]:
        """Read pointer info entries from data."""
        pointers = []
        pos = 0
        for _ in range(count):
            offset_in_memory = int.from_bytes(data[pos : pos + 4], "little")
            module = data[pos + 4]
            block_id = data[pos + 5]
            # Montreal engine: 8 bytes per pointer (2 padding bytes)
            pos += 8
            pointers.append(
                RelocationPointerInfo(
                    offset_in_memory=offset_in_memory,
                    module=module,
                    block_id=block_id,
                )
            )
        return pointers

    def read(self) -> RelocationTable:
        """Parse the relocation table.

        Returns:
            RelocationTable containing all pointer lists.
        """
        table = RelocationTable()

        if len(self.data) == 0:
            return table

        num_blocks = self._read_u8()
        # Montreal engine: no extra u32 after count

        for _ in range(num_blocks):
            if self.pos >= len(self.data):
                break

            module = self._read_u8()
            block_id = self._read_u8()
            pointer_count = self._read_u32()

            plist = RelocationPointerList(module=module, block_id=block_id)

            if pointer_count > 0:
                if self.compressed:
                    # Read compression header
                    is_compressed = self._read_u32()
                    compressed_size = self._read_u32()
                    compressed_checksum = self._read_u32()
                    decompressed_size = self._read_u32()
                    decompressed_checksum = self._read_u32()

                    compressed_data = self._read_bytes(compressed_size)

                    if is_compressed:
                        pointer_data = decompress(compressed_data, decompressed_size)
                    else:
                        pointer_data = compressed_data
                else:
                    # Uncompressed: 6 bytes per pointer (Montreal)
                    pointer_data = self._read_bytes(pointer_count * 6)

                plist.pointers = self._read_pointer_block(pointer_data, pointer_count)

            table.pointer_blocks.append(plist)

        return table


def read_relocation_table(path: Path, compressed: bool = True) -> RelocationTable:
    """Read and parse a relocation table file.

    Args:
        path: Path to the relocation table file (.rtb, .rtp, .rtt).
        compressed: Whether pointer blocks are LZO compressed.

    Returns:
        Parsed RelocationTable structure.
    """
    with open(path, "rb") as f:
        data = f.read()
    reader = RelocationTableReader(data, compressed=compressed)
    return reader.read()
