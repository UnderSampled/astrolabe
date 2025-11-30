"""Pointer and file abstraction for OpenSpace engine."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Optional, Callable, Any
from io import BytesIO

if TYPE_CHECKING:
    from .encryption import EncryptedReader


@dataclass
class Pointer:
    """
    A pointer in OpenSpace engine data.

    Pointers in OpenSpace are 32-bit values that reference locations within
    memory blocks. They need to be relocated based on relocation tables
    to convert from in-memory addresses to file offsets.
    """

    offset: int
    """Offset within the file after relocation."""

    file: Optional["FileWithPointers"] = None
    """The file this pointer belongs to."""

    def __hash__(self) -> int:
        return hash((self.offset, id(self.file)))

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Pointer):
            return False
        return self.offset == other.offset and self.file is other.file

    def __repr__(self) -> str:
        return f"Pointer(0x{self.offset:08X})"

    @classmethod
    def read(cls, reader: "EncryptedReader", file: Optional["FileWithPointers"] = None) -> Optional["Pointer"]:
        """
        Read a pointer from the stream.

        Args:
            reader: The reader to read from
            file: The file context for the pointer

        Returns:
            A Pointer instance, or None if the pointer is null
        """
        value = reader.read_uint32()
        if value == 0:
            return None
        return cls(offset=value, file=file)


@dataclass
class FileWithPointers:
    """
    Base class for files that contain relocatable pointers.

    OpenSpace engine files use a pointer relocation system where pointers
    in the file are stored as memory addresses and must be converted to
    file offsets using relocation tables.
    """

    name: str = ""
    """Name of the file."""

    base_offset: int = 0
    """Base offset for pointer calculations."""

    header_offset: int = 0
    """Offset to the header/start of actual data."""

    pointers: dict[int, Pointer] = field(default_factory=dict)
    """Map of file offsets to resolved pointers."""

    _data: bytes = field(default=b"", repr=False)
    """Raw file data."""

    _reader: Optional["EncryptedReader"] = field(default=None, repr=False)
    """Reader for the file data."""

    def get_pointer(self, offset: int) -> Optional[Pointer]:
        """
        Get a pointer at a specific offset.

        Args:
            offset: File offset where the pointer is stored

        Returns:
            The resolved pointer, or None if not found
        """
        return self.pointers.get(offset)

    def goto(self, pointer: Optional[Pointer]) -> bool:
        """
        Move the reader to a pointer's location.

        Args:
            pointer: The pointer to go to

        Returns:
            True if successful, False if pointer is None
        """
        if pointer is None or self._reader is None:
            return False

        self._reader.seek(pointer.offset)
        return True

    def goto_header(self) -> None:
        """Move the reader to the header offset."""
        if self._reader is not None:
            self._reader.seek(self.header_offset)

    @classmethod
    def do_at(
        cls,
        reader: "EncryptedReader",
        pointer: Optional[Pointer],
        action: Callable[[], Any],
    ) -> Optional[Any]:
        """
        Execute an action at a pointer's location, then return to the original position.

        Args:
            reader: The reader to use
            pointer: The pointer to go to
            action: The action to execute

        Returns:
            The result of the action, or None if pointer is None
        """
        if pointer is None:
            return None

        original_pos = reader.position
        reader.seek(pointer.offset)
        result = action()
        reader.seek(original_pos)
        return result
