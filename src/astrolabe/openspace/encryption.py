"""XOR masking and encryption utilities for OpenSpace files."""

from typing import Iterator


def _next_mask(current_mask: int) -> int:
    """Calculate next mask value using Linear Congruential Generator.

    The algorithm uses magic constant 0x075BD924 (123459876 decimal).
    """
    MAGIC = 0x075BD924
    xored = (current_mask ^ MAGIC) & 0xFFFFFFFF
    result = (16807 * xored - 0x7FFFFFFF * (xored // 0x1F31D)) & 0xFFFFFFFF
    return result


def _mask_generator(initial_mask: int) -> Iterator[int]:
    """Generate mask byte sequence from initial mask."""
    mask = initial_mask
    while True:
        yield (mask >> 8) & 0xFF
        mask = _next_mask(mask)


def decode_masked_data(data: bytes, read_mask_from_data: bool = True) -> bytes:
    """Decode XOR-masked data using number masking.

    Args:
        data: The encrypted data bytes.
        read_mask_from_data: If True, read initial mask from first 4 bytes.
                            If False, use fixed mask 0x6AB5CC79.

    Returns:
        Decoded data bytes.
    """
    if read_mask_from_data:
        if len(data) < 4:
            return data
        initial_mask = int.from_bytes(data[:4], 'little')
        data = data[4:]
    else:
        initial_mask = 0x6AB5CC79

    mask_gen = _mask_generator(initial_mask)
    result = bytearray(len(data))

    for i, byte in enumerate(data):
        result[i] = byte ^ next(mask_gen)

    return bytes(result)


def encode_masked_data(data: bytes, initial_mask: int | None = None) -> bytes:
    """Encode data with XOR masking.

    Args:
        data: The plaintext data bytes.
        initial_mask: Optional initial mask. If None, generates random one.

    Returns:
        Encoded data with 4-byte mask prefix.
    """
    import random

    if initial_mask is None:
        initial_mask = random.randint(0, 0xFFFFFFFF)

    mask_gen = _mask_generator(initial_mask)
    result = bytearray(len(data))

    for i, byte in enumerate(data):
        result[i] = byte ^ next(mask_gen)

    return initial_mask.to_bytes(4, 'little') + bytes(result)
