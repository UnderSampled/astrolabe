"""Pure Python LZO1X decompression.

This is a Python port of the lzo.net library used by Raymap,
sufficient for decompressing OpenSpace SNA blocks.

Based on lzo.net by Bianco Veigel (MIT License).

TODO: Replace with python-lzo package once a C compiler is available.
      Install with: sudo pacman -S base-devel && uv add python-lzo
      The native implementation will be significantly faster.
"""

from enum import IntEnum
from io import BytesIO


class LZOError(Exception):
    """LZO decompression error."""

    pass


class LzoState(IntEnum):
    """State of last copy operation."""

    ZERO_COPY = 0  # Last instruction did not copy any literal
    SMALL_COPY_1 = 1  # Last instruction copied 1 literal
    SMALL_COPY_2 = 2  # Last instruction copied 2 literals
    SMALL_COPY_3 = 3  # Last instruction copied 3 literals
    LARGE_COPY = 4  # Last instruction copied 4 or more literals


class LzoDecompressor:
    """LZO1X decompressor with ring buffer."""

    MAX_WINDOW_SIZE = (1 << 14) + ((255 & 8) << 11) + (255 << 6) + (255 >> 2)

    def __init__(self, data: bytes, output_size: int | None = None):
        """Initialize decompressor.

        Args:
            data: Compressed data bytes.
            output_size: Expected output size (optional).
        """
        self.source = BytesIO(data)
        self.output_size = output_size
        self.output = bytearray()
        self.ring_buffer: list[int] = []
        self.state = LzoState.ZERO_COPY
        self.instruction = 0

    def _read_byte(self) -> int:
        """Read a single byte from source."""
        b = self.source.read(1)
        if not b:
            raise LZOError("Unexpected end of input")
        return b[0]

    def _read_bytes(self, count: int) -> bytes:
        """Read multiple bytes from source."""
        data = self.source.read(count)
        if len(data) < count:
            raise LZOError("Unexpected end of input")
        return data

    def _copy_literal(self, count: int) -> None:
        """Copy literal bytes from source to output and ring buffer."""
        data = self._read_bytes(count)
        self.output.extend(data)
        self.ring_buffer.extend(data)
        # Keep ring buffer bounded
        if len(self.ring_buffer) > self.MAX_WINDOW_SIZE:
            self.ring_buffer = self.ring_buffer[-self.MAX_WINDOW_SIZE :]

    def _copy_from_ring_buffer(self, distance: int, length: int) -> None:
        """Copy bytes from ring buffer (back-reference)."""
        if distance > len(self.ring_buffer):
            raise LZOError(
                f"Invalid back-reference: distance {distance} > "
                f"buffer size {len(self.ring_buffer)}"
            )

        # Copy byte by byte to handle overlapping matches
        start_pos = len(self.ring_buffer) - distance
        for i in range(length):
            byte = self.ring_buffer[start_pos + (i % distance)]
            self.output.append(byte)
            self.ring_buffer.append(byte)

        # Keep ring buffer bounded
        if len(self.ring_buffer) > self.MAX_WINDOW_SIZE:
            self.ring_buffer = self.ring_buffer[-self.MAX_WINDOW_SIZE :]

    def _read_length(self) -> int:
        """Read variable-length encoded length."""
        length = 0
        while True:
            b = self._read_byte()
            if b != 0:
                return length + b
            length += 255
            if length > 0x1000000:  # Sanity check
                raise LZOError("Length too long")

    def _decode_first_byte(self) -> None:
        """Handle the first byte of compressed stream."""
        self.instruction = self._read_byte()

        if 15 < self.instruction <= 17:
            raise LZOError(f"Invalid first instruction: {self.instruction}")

        if self.instruction >= 18:
            # Initial literal run
            num_literals = self.instruction - 17
            self._copy_literal(num_literals)

            if self.instruction <= 21:
                self.state = LzoState(num_literals)
            else:
                self.state = LzoState.LARGE_COPY

            self.instruction = self._read_byte()

    def _decode_instruction(self) -> bool:
        """Decode one instruction. Returns False if end of stream."""
        if self.instruction <= 15:
            return self._decode_low_instruction()
        elif self.instruction < 32:
            return self._decode_long_match()
        elif self.instruction < 64:
            return self._decode_medium_match()
        elif self.instruction < 128:
            return self._decode_short_match_3_4()
        else:
            return self._decode_short_match_5_8()

    def _decode_low_instruction(self) -> bool:
        """Handle instructions 0-15 (depends on state)."""
        if self.state == LzoState.ZERO_COPY:
            # Copy long literal string
            # length = 3 + (L ?: 15 + (zero_bytes * 255) + non_zero_byte)
            if self.instruction != 0:
                length = 3 + self.instruction
            else:
                length = 3 + 15 + self._read_length()

            self._copy_literal(length)
            self.state = LzoState.LARGE_COPY

        elif self.state in (
            LzoState.SMALL_COPY_1,
            LzoState.SMALL_COPY_2,
            LzoState.SMALL_COPY_3,
        ):
            # Copy 2 bytes from <= 1kB distance
            # distance = (H << 2) + D + 1
            h = self._read_byte()
            distance = (h << 2) + ((self.instruction & 0x0C) >> 2) + 1
            self._copy_from_ring_buffer(distance, 2)

            # Copy S literals
            s = self.instruction & 0x03
            if s > 0:
                self._copy_literal(s)
            self.state = LzoState(s)

        else:  # LARGE_COPY
            # Copy 3 bytes from 2..3kB distance
            # distance = (H << 2) + D + 2049
            h = self._read_byte()
            distance = (h << 2) + ((self.instruction & 0x0C) >> 2) + 2049
            self._copy_from_ring_buffer(distance, 3)

            # Copy S literals
            s = self.instruction & 0x03
            if s > 0:
                self._copy_literal(s)
            self.state = LzoState(s)

        return True

    def _decode_long_match(self) -> bool:
        """Handle instructions 16-31 (long match 16-48kB)."""
        # length = 2 + (L ?: 7 + (zero_bytes * 255) + non_zero_byte)
        l = self.instruction & 0x07
        if l == 0:
            length = 2 + 7 + self._read_length()
        else:
            length = 2 + l

        # Read LE16
        s = self._read_byte()
        d = self._read_byte()
        d = ((d << 8) | s) >> 2

        # distance = 16384 + (H << 14) + D
        distance = 16384 + ((self.instruction & 0x08) << 11) + d

        # End of stream marker
        if distance == 16384:
            return False

        self._copy_from_ring_buffer(distance, length)

        # Copy S literals
        state = s & 0x03
        if state > 0:
            self._copy_literal(state)
        self.state = LzoState(state)

        return True

    def _decode_medium_match(self) -> bool:
        """Handle instructions 32-63 (medium match < 16kB)."""
        # length = 2 + (L ?: 31 + (zero_bytes * 255) + non_zero_byte)
        l = self.instruction & 0x1F
        if l == 0:
            length = 2 + 31 + self._read_length()
        else:
            length = 2 + l

        # Read LE16
        s = self._read_byte()
        d = self._read_byte()
        d = ((d << 8) | s) >> 2

        # distance = D + 1
        distance = d + 1

        self._copy_from_ring_buffer(distance, length)

        # Copy S literals
        state = s & 0x03
        if state > 0:
            self._copy_literal(state)
        self.state = LzoState(state)

        return True

    def _decode_short_match_3_4(self) -> bool:
        """Handle instructions 64-127 (3-4 bytes from < 2kB)."""
        # length = 3 + L
        length = 3 + ((self.instruction >> 5) & 0x01)

        # distance = (H << 3) + D + 1
        h = self._read_byte()
        distance = (h << 3) + ((self.instruction >> 2) & 0x07) + 1

        self._copy_from_ring_buffer(distance, length)

        # Copy S literals
        state = self.instruction & 0x03
        if state > 0:
            self._copy_literal(state)
        self.state = LzoState(state)

        return True

    def _decode_short_match_5_8(self) -> bool:
        """Handle instructions 128-255 (5-8 bytes from < 2kB)."""
        # length = 5 + L
        length = 5 + ((self.instruction >> 5) & 0x03)

        # distance = (H << 3) + D + 1
        h = self._read_byte()
        distance = (h << 3) + ((self.instruction & 0x1C) >> 2) + 1

        self._copy_from_ring_buffer(distance, length)

        # Copy S literals
        state = self.instruction & 0x03
        if state > 0:
            self._copy_literal(state)
        self.state = LzoState(state)

        return True

    def decompress(self) -> bytes:
        """Decompress the data.

        Returns:
            Decompressed data bytes.
        """
        if not self.source.read(1):
            return b""
        self.source.seek(0)

        self._decode_first_byte()

        while True:
            if self.output_size and len(self.output) >= self.output_size:
                break

            if not self._decode_instruction():
                break

            self.instruction = self._read_byte()

        if self.output_size:
            return bytes(self.output[: self.output_size])
        return bytes(self.output)


def decompress(data: bytes, output_size: int | None = None) -> bytes:
    """Decompress LZO1X compressed data.

    Args:
        data: Compressed data bytes.
        output_size: Expected output size (optional, for validation).

    Returns:
        Decompressed data bytes.

    Raises:
        LZOError: If decompression fails.
    """
    if not data:
        return b""

    decompressor = LzoDecompressor(data, output_size)
    return decompressor.decompress()


def decompress_block(data: bytes) -> tuple[bytes, bool]:
    """Decompress an SNA-style block with header.

    The header format is:
        u32 is_compressed (0=no, non-zero=yes)
        u32 compressed_size
        u32 compressed_checksum
        u32 decompressed_size
        u32 decompressed_checksum

    Args:
        data: Block data starting with compression header.

    Returns:
        Tuple of (decompressed_data, was_compressed).
    """
    if len(data) < 20:
        raise LZOError("Block header too short")

    is_compressed = int.from_bytes(data[0:4], "little")
    compressed_size = int.from_bytes(data[4:8], "little")
    # compressed_checksum = int.from_bytes(data[8:12], "little")
    decompressed_size = int.from_bytes(data[12:16], "little")
    # decompressed_checksum = int.from_bytes(data[16:20], "little")

    payload = data[20 : 20 + compressed_size]

    if is_compressed:
        return decompress(payload, decompressed_size), True
    else:
        return payload[:decompressed_size], False
