# Hype PC SNA framing

Verified for Hype's OpenSpace Montreal build on 2026-09-12. Other OpenSpace versions have different headers. See [rooted level research](../../notes/level-rust-research.md) for fixture hashes, pointer rules and implementation coverage.

SNA has no magic or file-wide header. It is a sequence of little-endian block entries. The Montreal prefix is **6 bytes**; it does not contain the additional byte used by later engines.

| Offset | Type | Field | Meaning |
| --- | --- | --- | --- |
| `0x00` | `u8` | module | Memory subsystem identity |
| `0x01` | `u8` | id | Block identity within that subsystem |
| `0x02` | `i32` | base | Source virtual base; `-1` ends this entry immediately |

If base is not `-1`, four more words follow. The complete ordinary header is **22 bytes**:

| Offset | Type | Field | Meaning |
| --- | --- | --- | --- |
| `0x06` | `u32` | capacity end | Inclusive final address of reserved block capacity |
| `0x0a` | `u32` | free-list head | First free allocator region; zero when none |
| `0x0e` | `u32` | high-water address | Furthest allocator position reached |
| `0x12` | `u32` | decoded size | Bytes saved from the source block base |

These meanings follow the engine's [block initializer](../../reference/decompilation/functions/004ae2a0_Mmg_fn_vInitBlock.c), [allocator](../../reference/decompilation/functions/004aeb10_FUN_004aeb10.c), [SNA writer](../../reference/decompilation/functions/004e86e0_FUN_004e86e0.c), and [loader](../../reference/decompilation/functions/004e8ad0_FUN_004e8ad0.c). The writer follows the free list and saves through the final free-region address plus nine bytes. Without a free list, it derives saved size as `capacity_end - base + 1`. Raymap calls the third word `maxPosMinus9`, but it is allocator state rather than an independently chosen file-size field. The two may coincide in a fixture, as they do for this level's populated blocks; that is not a universal rule.

A zero decoded size has no compression envelope. It can describe a source-local view of a populated shared Fix block. Interpret its pointers using this source file's base before selecting the actual populated owner; a flat virtual-address lookup is insufficient.

## Payload envelope

For positive decoded size, a 20-byte envelope follows the ordinary header:

| Relative offset | Type | Meaning |
| --- | --- | --- |
| `0x00` | `u32` | Compression mode: 0 stored, 1 LZO1X |
| `0x04` | `u32` | Stored payload size |
| `0x08` | `u32` | Stored payload Adler32 |
| `0x0c` | `u32` | Decoded payload size; must agree with SNA header |
| `0x10` | `u32` | Decoded payload Adler32 |
| `0x14` | bytes | Exactly the declared stored payload |

Adler32 uses initial accumulators 1 and 0 and modulus 65521. Stored mode requires equal stored/decoded sizes. The Rust compression adapter checks both checksums and bounded lengths. The next SNA entry follows the envelope's stored bytes. Historical compressed byte identity remains separate from correct decoded content and valid regenerated framing.

## Controlled Rust emission

`openspace::level::write_sna_block(data, (module, id), base)` emits one nonempty block around current reconstructed data. It derives capacity end, an empty free list, high-water address, sizes, checksums and compression choice. It rejects unaligned/zero bases, sentinel identities, oversized data and address overflow. Concatenating generated entries gives a container for focused block/relocation tests.

This primitive does not make a complete game-loadable level. Its compact payload has no reconstructed original allocator bookkeeping, and it does not assemble world roots, Fix bindings or all required packages. The engine's acceptance of this controlled layout is unverified. Full allocator-aware package reconstruction remains a later milestone. No original header values or encoded payload are cached in Rete.

Tests decode generated geometry blocks, resolve their emitted relocation rows across two blocks, and inspect current point/UV values after an edit. Separate local tests compare full decoded source blocks and RTB/RTP/RTT rows against independent Python/liblzo2 hashes.
