# Encryption and Compression

The OpenSpace Montreal engine uses XOR masking and LZO compression to protect game data.

## XOR Masking

### Number Masking (Primary Method)

Used for DSB files, relocation tables, and pointer files.

```
Initial mask: Read from first 4 bytes of file, OR use fixed value 0x6AB5CC79

For each byte:
    decoded_byte = encoded_byte XOR ((mask >> 8) & 0xFF)
    mask = next_mask(mask)

next_mask(current_mask):
    # Linear congruential generator
    # Magic constant: 0x075BD924 (123459876 decimal)
    return (16807 * (current_mask XOR 0x75BD924)) - (0x7FFFFFFF * ((current_mask XOR 0x75BD924) / 0x1F31D))
```

### Python Implementation

```python
def decode_masked_data(data: bytes, initial_mask: int | None = None) -> bytes:
    """Decode XOR-masked data using number masking."""
    if initial_mask is None:
        # Read mask from first 4 bytes
        initial_mask = int.from_bytes(data[:4], 'little')
        data = data[4:]

    mask = initial_mask
    result = bytearray(len(data))

    for i, byte in enumerate(data):
        result[i] = byte ^ ((mask >> 8) & 0xFF)
        mask = _next_mask(mask)

    return bytes(result)

def _next_mask(current_mask: int) -> int:
    """Calculate next mask value using LCG."""
    MAGIC = 0x075BD924  # 123459876
    xored = (current_mask ^ MAGIC) & 0xFFFFFFFF
    result = (16807 * xored) - (0x7FFFFFFF * (xored // 0x1F31D))
    return result & 0xFFFFFFFF
```

### Mask Initialization Modes

| Mode | Description | Used By |
|------|-------------|---------|
| `ReadInit` | Read 4-byte mask from file start | DSB, RTB, RTP, etc. |
| `FixedInit` | Use hardcoded `0x6AB5CC79` | Some file types |
| `Window` | Sliding window XOR (10-byte key) | Alternative method |

### Window Masking (Alternative)

Uses a 10-byte sliding window key:
```python
ORIGINAL_MASK = bytes([0x41, 0x59, 0xBE, 0xC7, 0x0D, 0x99, 0x1C, 0xA3, 0x75, 0x3F])

mask_bytes = bytearray(ORIGINAL_MASK)
current_pos = 0

for each byte:
    decoded = encoded XOR mask_bytes[current_pos]
    mask_bytes[current_pos] = (ORIGINAL_MASK[current_pos] + encoded) & 0xFF
    current_pos = (current_pos + 1) % 10
```

## LZO Compression

SNA blocks and relocation table entries use LZO1X compression.

### Compressed Block Header

```
Offset  Size  Description
------  ----  -----------
0x00    4     Is compressed (0 = no, non-zero = yes)
0x04    4     Compressed size
0x08    4     Compressed checksum
0x0C    4     Decompressed size
0x10    4     Decompressed checksum
0x14    N     Compressed data (N = compressed size)
```

### Checksum Algorithm

Custom Adler-like checksum used for verification:

```python
def calculate_checksum(data: bytes) -> int:
    """Calculate OpenSpace SNA checksum."""
    sum1 = 1
    sum2 = 0

    i = 0
    while i < len(data):
        # Process in blocks of up to 5552 bytes
        block_size = min(5552, len(data) - i)

        # Process 16 bytes at a time for efficiency
        while block_size >= 16:
            for j in range(16):
                sum1 += data[i + j]
                sum2 += sum1
            i += 16
            block_size -= 16

        # Process remaining bytes
        while block_size > 0:
            sum1 += data[i]
            sum2 += sum1
            i += 1
            block_size -= 1

        sum1 %= 0xFFF1
        sum2 %= 0xFFF1

    return sum1 | (sum2 << 16)
```

### Python LZO Decompression

```python
import lzo  # python-lzo package

def decompress_sna_block(data: bytes) -> bytes:
    """Decompress an SNA block if compressed."""
    is_compressed = int.from_bytes(data[0:4], 'little')
    compressed_size = int.from_bytes(data[4:8], 'little')
    # compressed_checksum = int.from_bytes(data[8:12], 'little')
    decompressed_size = int.from_bytes(data[12:16], 'little')
    # decompressed_checksum = int.from_bytes(data[16:20], 'little')

    compressed_data = data[20:20 + compressed_size]

    if is_compressed:
        return lzo.decompress(compressed_data, False, decompressed_size)
    else:
        return compressed_data
```

## File-Specific Encryption

| File Type | Masking | Compression |
|-----------|---------|-------------|
| DSB/DSC | Number (ReadInit) | None |
| SNA | Number (ReadInit) | LZO per block |
| RTB/RTP/RTT | Number (ReadInit) | LZO per entry |
| GPT | Number (ReadInit) | None |
| PTX | Number (ReadInit) | None |
| GF (textures) | None | RLE only |
| LVL | None | None |

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/IO/Reader.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/SNA.cs`
