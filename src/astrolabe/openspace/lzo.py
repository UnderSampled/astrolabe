"""LZO decompression for OpenSpace engine files."""

from __future__ import annotations

import lzo


def decompress_lzo(compressed_data: bytes, decompressed_size: int) -> bytes:
    """
    Decompress LZO-compressed data.

    The OpenSpace engine uses LZO1X compression for SNA blocks
    and relocation table data.

    Args:
        compressed_data: LZO-compressed bytes
        decompressed_size: Expected size of decompressed data

    Returns:
        Decompressed bytes

    Raises:
        lzo.error: If decompression fails
    """
    return lzo.decompress(compressed_data, False, decompressed_size)


def compress_lzo(data: bytes) -> bytes:
    """
    Compress data using LZO1X.

    Args:
        data: Bytes to compress

    Returns:
        LZO-compressed bytes
    """
    return lzo.compress(data, 1, False)
