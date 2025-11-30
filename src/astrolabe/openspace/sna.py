"""SNA file parser for OpenSpace Montreal engine (Hype: The Time Quest).

SNA files contain serialized game data as compressed memory blocks.
Note: Montreal engine (Hype) does NOT encrypt SNA files, unlike R2/R3.
"""

from dataclasses import dataclass, field
from pathlib import Path

from .lzo import decompress


@dataclass
class SNAMemoryBlock:
    """A single memory block from an SNA file."""

    module: int
    block_id: int
    base_in_memory: int
    size: int
    data: bytes
    data_position: int  # Position in decompressed data stream

    @property
    def relocation_key(self) -> int:
        """Get the relocation key for pointer resolution."""
        return (self.module << 8) | self.block_id

    @property
    def is_terminator(self) -> bool:
        """Check if this is a terminator block."""
        return self.base_in_memory == -1


@dataclass
class SNAFile:
    """Parsed SNA file containing memory blocks."""

    name: str
    blocks: list[SNAMemoryBlock] = field(default_factory=list)
    data: bytes = b""  # Combined decompressed data

    def get_block(self, module: int, block_id: int) -> SNAMemoryBlock | None:
        """Find a block by module and block ID."""
        key = (module << 8) | block_id
        for block in self.blocks:
            if block.relocation_key == key:
                return block
        return None


class SNAReader:
    """Reader for SNA files (Montreal engine variant)."""

    def __init__(self, data: bytes, name: str = ""):
        """Initialize with raw file data.

        Args:
            data: Raw SNA file bytes.
            name: Optional name for the SNA file.
        """
        self.name = name
        # Montreal engine (Hype) does not encrypt SNA files
        self.raw_data = data
        self.pos = 0

    def _read_u8(self) -> int:
        """Read unsigned 8-bit integer."""
        val = self.raw_data[self.pos]
        self.pos += 1
        return val

    def _read_u16(self) -> int:
        """Read unsigned 16-bit integer (little endian)."""
        val = int.from_bytes(self.raw_data[self.pos : self.pos + 2], "little")
        self.pos += 2
        return val

    def _read_i32(self) -> int:
        """Read signed 32-bit integer (little endian)."""
        val = int.from_bytes(
            self.raw_data[self.pos : self.pos + 4], "little", signed=True
        )
        self.pos += 4
        return val

    def _read_u32(self) -> int:
        """Read unsigned 32-bit integer (little endian)."""
        val = int.from_bytes(self.raw_data[self.pos : self.pos + 4], "little")
        self.pos += 4
        return val

    def _read_bytes(self, count: int) -> bytes:
        """Read raw bytes."""
        val = self.raw_data[self.pos : self.pos + count]
        self.pos += count
        return val

    def read(self) -> SNAFile:
        """Parse the SNA file and return the parsed structure.

        Returns:
            SNAFile containing all memory blocks.
        """
        sna = SNAFile(name=self.name)
        decompressed_parts: list[bytes] = []
        current_data_position = 0

        while self.pos < len(self.raw_data):
            module = self._read_u8()
            block_id = self._read_u8()

            # Montreal engine: baseInMemory is at offset 2 (no unk1 byte)
            base_in_memory = self._read_i32()

            if base_in_memory == -1:
                # Terminator block
                sna.blocks.append(
                    SNAMemoryBlock(
                        module=module,
                        block_id=block_id,
                        base_in_memory=-1,
                        size=0,
                        data=b"",
                        data_position=current_data_position,
                    )
                )
                break

            # Read block metadata
            unk2 = self._read_u32()
            unk3 = self._read_u32()
            max_pos_minus_9 = self._read_u32()
            block_size = self._read_u32()

            data_position = current_data_position

            if block_size > 0:
                # Read compression header
                is_compressed = self._read_u32()
                compressed_size = self._read_u32()
                compressed_checksum = self._read_u32()
                decompressed_size = self._read_u32()
                decompressed_checksum = self._read_u32()

                compressed_data = self._read_bytes(compressed_size)

                if is_compressed:
                    block_data = decompress(compressed_data, decompressed_size)
                else:
                    block_data = compressed_data[:block_size]

                decompressed_parts.append(block_data)
                current_data_position += len(block_data)
            else:
                block_data = b""

            sna.blocks.append(
                SNAMemoryBlock(
                    module=module,
                    block_id=block_id,
                    base_in_memory=base_in_memory,
                    size=block_size,
                    data=block_data,
                    data_position=data_position,
                )
            )

        # Combine all decompressed data
        sna.data = b"".join(decompressed_parts)

        return sna


def read_sna_file(path: Path) -> SNAFile:
    """Read and parse an SNA file.

    Args:
        path: Path to the SNA file.

    Returns:
        Parsed SNAFile structure.
    """
    with open(path, "rb") as f:
        data = f.read()
    reader = SNAReader(data, name=path.stem)
    return reader.read()
