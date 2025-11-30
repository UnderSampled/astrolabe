"""Pure Python LZO1X decompression for OpenSpace engine files.

Ported from lzo.net by Bianco Veigel (MIT License)
https://github.com/bianco-veigel/lzo.net

This implementation is sufficient for reading OpenSpace/Montreal engine data.
"""

from __future__ import annotations

from io import BytesIO
from typing import BinaryIO


class LZOError(Exception):
    """LZO decompression error."""
    pass


class RingBuffer:
    """Ring buffer for LZO back-reference lookups."""

    def __init__(self, size: int):
        self._buffer = bytearray(size)
        self._size = size
        self._position = 0

    def write(self, data: bytes, offset: int, count: int) -> None:
        """Write data to the ring buffer."""
        for i in range(count):
            self._buffer[self._position] = data[offset + i]
            self._position = (self._position + 1) % self._size

    def copy(self, dest: bytearray, dest_offset: int, distance: int, count: int) -> None:
        """Copy data from the ring buffer at a given distance."""
        read_pos = (self._position - distance) % self._size
        for i in range(count):
            byte = self._buffer[read_pos]
            dest[dest_offset + i] = byte
            # Also write to buffer as it's read (for overlapping copies)
            self._buffer[self._position] = byte
            self._position = (self._position + 1) % self._size
            read_pos = (read_pos + 1) % self._size


class LzoState:
    """State of the LZO decompressor."""
    ZERO_COPY = 0
    SMALL_COPY_1 = 1
    SMALL_COPY_2 = 2
    SMALL_COPY_3 = 3
    LARGE_COPY = 4


class LzoDecompressor:
    """LZO1X decompressor."""

    MAX_WINDOW_SIZE = (1 << 14) + ((255 & 8) << 11) + (255 << 6) + (255 >> 2)

    def __init__(self, source: BinaryIO):
        self._source = source
        self._ring_buffer = RingBuffer(self.MAX_WINDOW_SIZE)
        self._state = LzoState.ZERO_COPY
        self._instruction = 0
        self._decoded_buffer: bytes | None = None
        self._output_position = 0
        self._finished = False

        # Decode first byte
        self._decode_first_byte()

    def _read_byte(self) -> int:
        """Read a single byte from source."""
        b = self._source.read(1)
        if not b:
            raise LZOError("Unexpected end of stream")
        return b[0]

    def _read_bytes(self, count: int) -> bytes:
        """Read multiple bytes from source."""
        data = self._source.read(count)
        if len(data) < count:
            raise LZOError("Unexpected end of stream")
        return data

    def _copy_from_source(self, dest: bytearray, offset: int, count: int) -> None:
        """Copy bytes from source to dest and ring buffer."""
        data = self._read_bytes(count)
        for i in range(count):
            dest[offset + i] = data[i]
        self._ring_buffer.write(data, 0, count)

    def _decode_first_byte(self) -> None:
        """Handle the first byte of the compressed stream."""
        self._instruction = self._read_byte()

        if 15 < self._instruction <= 17:
            raise LZOError("Invalid first instruction")

        if self._instruction >= 18:
            num_literals = self._instruction - 17
            self._decoded_buffer = bytearray(num_literals)
            self._copy_from_source(self._decoded_buffer, 0, num_literals)

            if self._instruction <= 21:
                self._state = num_literals  # SMALL_COPY_1-3
            else:
                self._state = LzoState.LARGE_COPY

            self._instruction = self._read_byte()

    def _read_length(self) -> int:
        """Read variable-length encoding."""
        length = 0
        while True:
            b = self._read_byte()
            if b != 0:
                return length + b
            length += 255

    def decompress(self, output_size: int) -> bytes:
        """Decompress to a fixed output size."""
        result = bytearray(output_size)
        pos = 0

        while pos < output_size and not self._finished:
            # First drain any buffered data
            if self._decoded_buffer:
                copy_len = min(len(self._decoded_buffer), output_size - pos)
                result[pos:pos + copy_len] = self._decoded_buffer[:copy_len]
                pos += copy_len
                if copy_len < len(self._decoded_buffer):
                    self._decoded_buffer = self._decoded_buffer[copy_len:]
                else:
                    self._decoded_buffer = None
                continue

            # Decode next instruction
            read = self._decode(result, pos, output_size - pos)
            if read < 0:
                self._finished = True
                break
            pos += read

        return bytes(result)

    def _decode(self, buffer: bytearray, offset: int, count: int) -> int:
        """Decode the next chunk of data."""
        if self._instruction <= 15:
            # Depends on the number of literals copied by the last instruction
            if self._state == LzoState.ZERO_COPY:
                # Copy long literal string
                length = 3
                if self._instruction != 0:
                    length += self._instruction
                else:
                    length += 15 + self._read_length()

                if length > count:
                    self._copy_from_source(buffer, offset, count)
                    self._decoded_buffer = bytearray(length - count)
                    self._copy_from_source(self._decoded_buffer, 0, length - count)
                    read = count
                else:
                    self._copy_from_source(buffer, offset, length)
                    read = length

                self._state = LzoState.LARGE_COPY

            elif self._state in (LzoState.SMALL_COPY_1, LzoState.SMALL_COPY_2, LzoState.SMALL_COPY_3):
                read = self._small_copy(buffer, offset, count)
            else:  # LARGE_COPY
                read = self._large_copy(buffer, offset, count)

        elif self._instruction < 32:
            # Copy of a block within 16..48kB distance
            length = (self._instruction & 0x7)
            if length == 0:
                length = 7 + self._read_length()
            length += 2

            s = self._read_byte()
            d = self._read_byte()
            d = ((d << 8) | s) >> 2
            distance = 16384 + ((self._instruction & 0x8) << 11) | d

            if distance == 16384:
                return -1  # End of stream

            read = self._copy_from_ring_buffer(buffer, offset, count, distance, length, s & 0x3)

        elif self._instruction < 64:
            # Copy of small block within 16kB distance
            length = (self._instruction & 0x1f)
            if length == 0:
                length = 31 + self._read_length()
            length += 2

            s = self._read_byte()
            d = self._read_byte()
            d = ((d << 8) | s) >> 2
            distance = d + 1

            read = self._copy_from_ring_buffer(buffer, offset, count, distance, length, s & 0x3)

        elif self._instruction < 128:
            # Copy 3-4 bytes from block within 2kB distance
            length = 3 + ((self._instruction >> 5) & 0x1)
            h = self._read_byte()
            distance = (h << 3) + ((self._instruction >> 2) & 0x7) + 1

            read = self._copy_from_ring_buffer(buffer, offset, count, distance, length, self._instruction & 0x3)

        else:
            # Copy 5-8 bytes from block within 2kB distance
            length = 5 + ((self._instruction >> 5) & 0x3)
            h = self._read_byte()
            distance = (h << 3) + ((self._instruction & 0x1c) >> 2) + 1

            read = self._copy_from_ring_buffer(buffer, offset, count, distance, length, self._instruction & 0x3)

        self._instruction = self._read_byte()
        self._output_position += read
        return read

    def _large_copy(self, buffer: bytearray, offset: int, count: int) -> int:
        """Copy 3 bytes from 2..3kB distance."""
        h = self._read_byte()
        distance = (h << 2) + ((self._instruction & 0xc) >> 2) + 2049
        return self._copy_from_ring_buffer(buffer, offset, count, distance, 3, self._instruction & 0x3)

    def _small_copy(self, buffer: bytearray, offset: int, count: int) -> int:
        """Copy 2 bytes from <= 1kB distance."""
        h = self._read_byte()
        distance = (h << 2) + ((self._instruction & 0xc) >> 2) + 1
        return self._copy_from_ring_buffer(buffer, offset, count, distance, 2, self._instruction & 0x3)

    def _copy_from_ring_buffer(
        self,
        buffer: bytearray,
        offset: int,
        count: int,
        distance: int,
        copy: int,
        state: int,
    ) -> int:
        """Copy from ring buffer and optionally append literals."""
        result = copy + state

        if count < result:
            # Need to buffer some output
            if count <= copy:
                self._do_ring_copy(buffer, offset, distance, count)
                self._decoded_buffer = bytearray(result - count)
                self._do_ring_copy(self._decoded_buffer, 0, distance, copy - count)
                if state > 0:
                    self._copy_from_source(self._decoded_buffer, copy - count, state)
                return count
            else:
                self._do_ring_copy(buffer, offset, distance, copy)
                remaining = count - copy
                self._decoded_buffer = bytearray(state - remaining)
                self._copy_from_source(buffer, offset + copy, remaining)
                self._copy_from_source(self._decoded_buffer, 0, state - remaining)
                self._state = state
                return count

        # Copy from ring buffer
        self._do_ring_copy(buffer, offset, distance, copy)

        # Copy trailing literals
        if state > 0:
            self._copy_from_source(buffer, offset + copy, state)

        self._state = state
        return result

    def _do_ring_copy(self, buffer: bytearray, offset: int, distance: int, count: int) -> None:
        """Perform the actual ring buffer copy."""
        self._ring_buffer.copy(buffer, offset, distance, count)


def decompress_lzo(compressed_data: bytes, decompressed_size: int) -> bytes:
    """
    Decompress LZO1X-compressed data.

    The OpenSpace engine uses LZO1X-1 compression for SNA blocks
    and relocation table data.

    Args:
        compressed_data: LZO-compressed bytes
        decompressed_size: Expected size of decompressed data

    Returns:
        Decompressed bytes

    Raises:
        LZOError: If decompression fails
    """
    if not compressed_data:
        return b""

    if decompressed_size == 0:
        return b""

    stream = BytesIO(compressed_data)
    decompressor = LzoDecompressor(stream)
    return decompressor.decompress(decompressed_size)


def compress_lzo(data: bytes) -> bytes:
    """
    Compress data using LZO1X.

    Note: This is a placeholder - full LZO compression is complex.

    Args:
        data: Bytes to compress

    Returns:
        LZO-compressed bytes
    """
    raise NotImplementedError("LZO compression not implemented")
