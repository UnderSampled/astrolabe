"""XOR encryption/masking used by OpenSpace engine files."""

from __future__ import annotations

import struct
from typing import BinaryIO


class XORMask:
    """
    XOR masking used by OpenSpace Montreal engine for file encryption.

    The Montreal engine (used by Hype) uses a simple XOR mask that is initialized
    from the first 4 bytes of the file. The mask value changes as data is read.
    """

    def __init__(self):
        self.mask: int = 0
        self.initialized: bool = False

    def init_from_stream(self, stream: BinaryIO) -> int:
        """
        Initialize the mask from a stream.

        Reads 4 bytes from the stream and uses them to initialize the XOR mask.

        Args:
            stream: Binary stream to read from

        Returns:
            Number of bytes read (4)
        """
        mask_bytes = stream.read(4)
        if len(mask_bytes) < 4:
            raise ValueError("Not enough bytes to initialize mask")

        self.mask = struct.unpack("<I", mask_bytes)[0]
        self.initialized = True
        return 4

    def decode_byte(self, b: int) -> int:
        """
        Decode a single byte using the current mask.

        Args:
            b: Byte value to decode

        Returns:
            Decoded byte value
        """
        if not self.initialized:
            return b

        # XOR with the low byte of the mask
        result = b ^ (self.mask & 0xFF)

        # Update the mask
        self.mask = ((self.mask << 1) | ((self.mask >> 31) & 1)) & 0xFFFFFFFF

        return result

    def decode_bytes(self, data: bytes) -> bytes:
        """
        Decode a sequence of bytes.

        Args:
            data: Bytes to decode

        Returns:
            Decoded bytes
        """
        if not self.initialized:
            return data

        result = bytearray(len(data))
        for i, b in enumerate(data):
            result[i] = self.decode_byte(b)
        return bytes(result)


class EncryptedReader:
    """
    Binary reader with optional XOR decryption.

    Wraps a binary stream and provides methods to read common data types,
    with optional XOR decryption applied.
    """

    def __init__(
        self,
        stream: BinaryIO,
        little_endian: bool = True,
        encrypted: bool = False,
    ):
        """
        Initialize the encrypted reader.

        Args:
            stream: Binary stream to read from
            little_endian: Whether to use little-endian byte order
            encrypted: Whether the stream uses XOR encryption
        """
        self.stream = stream
        self.little_endian = little_endian
        self.encrypted = encrypted
        self.mask = XORMask()
        self._endian = "<" if little_endian else ">"

        if encrypted:
            self.mask.init_from_stream(stream)

    @property
    def position(self) -> int:
        """Current position in the stream."""
        return self.stream.tell()

    @position.setter
    def position(self, value: int):
        """Set the current position in the stream."""
        self.stream.seek(value)

    def seek(self, offset: int, whence: int = 0) -> int:
        """Seek to a position in the stream."""
        return self.stream.seek(offset, whence)

    def read(self, size: int) -> bytes:
        """
        Read bytes from the stream.

        Args:
            size: Number of bytes to read

        Returns:
            Read bytes (decrypted if encryption is enabled)
        """
        data = self.stream.read(size)
        if self.encrypted:
            data = self.mask.decode_bytes(data)
        return data

    def read_byte(self) -> int:
        """Read a single byte."""
        data = self.read(1)
        return data[0] if data else 0

    def read_sbyte(self) -> int:
        """Read a signed byte."""
        return struct.unpack("b", self.read(1))[0]

    def read_uint16(self) -> int:
        """Read an unsigned 16-bit integer."""
        return struct.unpack(f"{self._endian}H", self.read(2))[0]

    def read_int16(self) -> int:
        """Read a signed 16-bit integer."""
        return struct.unpack(f"{self._endian}h", self.read(2))[0]

    def read_uint32(self) -> int:
        """Read an unsigned 32-bit integer."""
        return struct.unpack(f"{self._endian}I", self.read(4))[0]

    def read_int32(self) -> int:
        """Read a signed 32-bit integer."""
        return struct.unpack(f"{self._endian}i", self.read(4))[0]

    def read_float(self) -> float:
        """Read a 32-bit floating point number."""
        return struct.unpack(f"{self._endian}f", self.read(4))[0]

    def read_string(self, length: int) -> str:
        """
        Read a fixed-length string.

        Args:
            length: Number of bytes to read

        Returns:
            Decoded string (null bytes stripped)
        """
        data = self.read(length)
        # Strip null bytes and decode
        return data.rstrip(b"\x00").decode("latin-1")

    def read_cstring(self) -> str:
        """Read a null-terminated string."""
        chars = []
        while True:
            b = self.read_byte()
            if b == 0:
                break
            chars.append(chr(b))
        return "".join(chars)
