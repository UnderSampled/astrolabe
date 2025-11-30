"""Relocation table parsing for OpenSpace engine."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import BinaryIO, Optional
from io import BytesIO

from .encryption import EncryptedReader
from .lzo import decompress_lzo


@dataclass
class RelocationPointerInfo:
    """
    Information about a single pointer in a relocation table.

    Each entry describes where a pointer is located and which memory block
    it points to.
    """

    offset_in_memory: int
    """Offset where the pointer is located (in memory address space)."""

    module: int
    """Module ID of the target memory block."""

    block_id: int
    """Block ID within the module."""

    byte6: int = 0
    """Additional byte (usage varies by engine version)."""

    byte7: int = 0
    """Additional byte (usage varies by engine version)."""

    def __repr__(self) -> str:
        return f"RelocationPointerInfo(offset=0x{self.offset_in_memory:08X}, module={self.module}, block={self.block_id})"


@dataclass
class RelocationPointerList:
    """
    A list of pointers for a specific memory block.

    Groups all pointers that are located within a particular (module, block) pair.
    """

    module: int
    """Module ID for this block."""

    block_id: int
    """Block ID within the module."""

    count: int
    """Number of pointers in this block."""

    pointers: list[RelocationPointerInfo] = field(default_factory=list)
    """List of pointer information entries."""

    def __repr__(self) -> str:
        return f"RelocationPointerList(module={self.module}, block={self.block_id}, count={self.count})"


@dataclass
class RelocationTable:
    """
    Relocation table for OpenSpace engine files.

    Relocation tables (.rtb, .rtp, .rtt, .rtd) contain information about
    how to convert memory addresses in SNA files to actual file offsets.

    The Montreal engine (Hype) uses compressed relocation tables.
    """

    pointer_blocks: list[RelocationPointerList] = field(default_factory=list)
    """List of pointer blocks."""

    @classmethod
    def from_file(
        cls,
        path: str,
        encrypted: bool = True,
        compressed: bool = True,
    ) -> "RelocationTable":
        """
        Load a relocation table from a file.

        Args:
            path: Path to the relocation table file
            encrypted: Whether the file uses XOR encryption
            compressed: Whether pointer blocks are LZO compressed

        Returns:
            Parsed RelocationTable
        """
        with open(path, "rb") as f:
            return cls.from_stream(f, encrypted=encrypted, compressed=compressed)

    @classmethod
    def from_stream(
        cls,
        stream: BinaryIO,
        encrypted: bool = True,
        compressed: bool = True,
    ) -> "RelocationTable":
        """
        Load a relocation table from a stream.

        Args:
            stream: Binary stream to read from
            encrypted: Whether the stream uses XOR encryption
            compressed: Whether pointer blocks are LZO compressed

        Returns:
            Parsed RelocationTable
        """
        reader = EncryptedReader(stream, encrypted=encrypted)
        return cls._read(reader, compressed=compressed)

    @classmethod
    def from_bytes(
        cls,
        data: bytes,
        encrypted: bool = True,
        compressed: bool = True,
    ) -> "RelocationTable":
        """
        Load a relocation table from bytes.

        Args:
            data: Raw bytes
            encrypted: Whether the data uses XOR encryption
            compressed: Whether pointer blocks are LZO compressed

        Returns:
            Parsed RelocationTable
        """
        return cls.from_stream(BytesIO(data), encrypted=encrypted, compressed=compressed)

    @classmethod
    def _read(cls, reader: EncryptedReader, compressed: bool = True) -> "RelocationTable":
        """
        Read a relocation table from an encrypted reader.

        Args:
            reader: The reader to read from
            compressed: Whether pointer blocks are LZO compressed

        Returns:
            Parsed RelocationTable
        """
        table = cls()

        # Read number of pointer blocks
        count = reader.read_byte()

        # Montreal engine doesn't have the extra uint32 here
        # (Other versions like R2/R3 have: reader.read_uint32())

        table.pointer_blocks = []
        for _ in range(count):
            block = RelocationPointerList(
                module=reader.read_byte(),
                block_id=reader.read_byte(),
                count=reader.read_uint32(),
                pointers=[],
            )

            if block.count > 0:
                if compressed:
                    # Read compression header
                    is_compressed = reader.read_uint32()
                    compressed_size = reader.read_uint32()
                    compressed_checksum = reader.read_uint32()
                    decompressed_size = reader.read_uint32()
                    decompressed_checksum = reader.read_uint32()

                    compressed_data = reader.read(compressed_size)

                    if is_compressed != 0:
                        # Decompress the pointer data
                        decompressed_data = decompress_lzo(compressed_data, decompressed_size)
                        block_reader = EncryptedReader(BytesIO(decompressed_data), encrypted=False)
                    else:
                        block_reader = EncryptedReader(BytesIO(compressed_data), encrypted=False)

                    cls._read_pointer_block(block_reader, block)
                else:
                    cls._read_pointer_block(reader, block)

            table.pointer_blocks.append(block)

        return table

    @classmethod
    def _read_pointer_block(cls, reader: EncryptedReader, block: RelocationPointerList) -> None:
        """
        Read pointer entries for a block.

        Args:
            reader: Reader positioned at the pointer data
            block: Block to populate with pointer info
        """
        for _ in range(block.count):
            info = RelocationPointerInfo(
                offset_in_memory=reader.read_uint32(),
                module=reader.read_byte(),
                block_id=reader.read_byte(),
                # Montreal engine uses 6 bytes per pointer entry, not 8
                # byte6=reader.read_byte(),
                # byte7=reader.read_byte(),
            )
            block.pointers.append(info)

    def get_list_for_part(self, module: int, block_id: int) -> Optional[RelocationPointerList]:
        """
        Find the pointer list for a specific module/block.

        Args:
            module: Module ID
            block_id: Block ID

        Returns:
            The matching RelocationPointerList, or None if not found
        """
        for block in self.pointer_blocks:
            if block.module == module and block.block_id == block_id:
                return block
        return None


def get_relocation_key(module: int, block_id: int) -> int:
    """
    Generate a unique key for a module/block pair.

    Args:
        module: Module ID
        block_id: Block ID

    Returns:
        Combined key value
    """
    return (module * 0x100) + block_id
