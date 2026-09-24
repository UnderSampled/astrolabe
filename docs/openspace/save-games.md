# Hype PC save-game files

This describes the supplied English PC `SaveGame0` fixture and the relevant
Hype engine save path. It is a structural research baseline, not yet a mapping
from saved records to every gameplay variable. The fixture is local at
`reference/Savefile_EN.zip` and remains unchanged. Orrery's read-only Rust
`save-inspect` and `save-diff` commands are in
[`crates/orrery/src/save.rs`](../../crates/orrery/src/save.rs).

## File roles and observed contents

| File | Stored bytes | Decoded bytes | Observed content |
| --- | ---: | ---: | --- |
| `General.sav` | 8 | 8 | Two little-endian `u32` values, both zero in this fixture; written directly by `00431390` |
| `General000.sav` | 46 | 38 | Four CRLF-delimited lines: `manoir`, `Gogoud's Manoir I`, `249589`, `0` |
| `Level000.sav` | 13,420 | 17,405 | 1,271 SAI save-table records |
| `manoir.sav` | 2,847 | 3,583 | 263 SAI save-table records |

The game writes `General000.sav` by formatting the current level identifier,
display text and two decimal values into a temporary text file, then encoding
it. It writes `Level000.sav` through the player-save table route and the
current level's named `.sav` through the level-save route. See the locally
generated [Hype decompilation](../../reference/decompilation/all_functions.c),
especially `00431180`–`004317d0` and `004c6d60`–`004c7300`.

In this fixture, every `manoir.sav` record identity and value also occurs in
`Level000.sav`. The save writer selects table entries with flags masked by
`0x0300` for the player file and `0x0c00` for the level file. A record may
qualify for both; `0x0a0c` does. This explains the observed overlap without
implying that the two files will always remain identical after further saves.

## Encoded envelope

Except for `General.sav`, the observed files have a five-byte header followed
by an XOR-obfuscated compressed stream:

| Encoded offset | Decoded meaning |
| ---: | --- |
| 0 | Additive checksum of the deobfuscated compressed stream, XOR `0x53` |
| 1–4 | Decoded byte length as little-endian `u32`, XOR `0x54555657` |
| 5 onward | Compressed stream; byte *n* is XORed with `reverse_bits((0x57 + n) mod 256) ^ 0xb9` |

The checksum is the sum of the deobfuscated **compressed** bytes modulo 256.
The stream uses 256 eight-byte history windows initially filled with the
respective window index. Each group starts with a window index and a change
mask. Bit *i* of the mask means the next stream byte replaces byte *i* of that
window. The group emits the resulting eight bytes, except that a final count
byte can limit the final group. The [archived OpenSpace decoder](../../reference/BinarySerializer.OpenSpace/src/DataTypes/Save/R2/R2SaveEncoder.cs)
describes this codec; Orrery independently verifies checksum, decoded size,
and record boundaries against the Hype fixture. The codec name in that archived
code refers to Rayman 2, but these Hype files pass its byte-level checks.

## Decoded SAI records

Each table entry begins with this ten-byte header:

| Offset | Width | Meaning |
| ---: | ---: | --- |
| 0 | 1 | Source memory block ID |
| 1 | 1 | Source memory module ID |
| 2 | 4 | Offset within that block/module, little-endian |
| 6 | 2 | Save/init flags, little-endian |
| 8 | 1 | Middle byte of the combined flags/type word; zero in this fixture |
| 9 | 1 | Data type |

The engine writer locates the registered address's memory block and writes
its block/module pair, relative offset, and four-byte flags/type word before
the value. Type `0x01`, `0x02`, `0x04`, and `0x08` store that many value bytes;
`0x10` stores a little-endian `u32` length followed by the array bytes;
`0x11` stores a six-byte block/module/relative-offset reference. The fixture
contains types `0x01`, `0x02`, `0x04`, `0x10`, and `0x11`. All 1,534 decoded
entries parse to the exact end of their respective files, with no duplicate
block/module/offset identities.

Observed flags are `0x0204` (834 records), `0x0205` (174), and `0x0a0c`
(263) in `Level000.sav`; `manoir.sav` contains the `0x0a0c` records. The
engine's masks establish the player/level save selection bits. The archived
[Tonic Trouble flags](../../reference/BinarySerializer.OpenSpace/src/DataTypes/Save/TT/TTSAIFlags.cs)
suggest meanings for other bits, but those meanings still need Hype-specific
corroboration. Its pointer-type number differs from Hype's observed `0x11`.

The Hype writer checks `0x0100` or `0x0400` to write the registered table
value, and `0x0200` or `0x0800` to write the current memory value. Combined
with the player and level file masks, this supports the following selection
bits:

| Bit | Observed save role |
| ---: | --- |
| `0x0100` | Player file, table value |
| `0x0200` | Player file, current value |
| `0x0400` | Level file, table value |
| `0x0800` | Level file, current value |

## Measuring a cheat or teleport save

Use an unchanged copy of the supplied save in the private test installation.
For each case, capture live Global and other expected state before and after
the input, make the game itself save, then copy the resulting slot into an
ignored case directory outside `reference/`. Reuse the same in-game slot across
cases, restoring the source fixture before each run. Record executable hash,
fixture hash, code, level, live observation addresses and typed values, save
member hashes, and any test intervention.

```sh
mkdir -p output/cheat-save-research
cargo run -p orrery --locked --offline -- save-inspect reference/Savefile_EN.zip \
  --output output/cheat-save-research/baseline.json
cargo run -p orrery --locked --offline -- save-diff \
  reference/Savefile_EN.zip output/cheat-save-research/thereyougo \
  --output output/cheat-save-research/thereyougo/save-diff.json
```

The diff keys records by block/module/offset and reports type, flags, and
value changes. It omits record-position shifts when deciding whether a value
changed. A changed record is evidence of saved bytes, not yet proof of a named
gameplay variable; match it to a live, typed Global/DSG observation and a
controlled input to establish that mapping. Orrery does not currently offer
an in-game save command. Its input and memory tools can support a logged
experiment, but the game must perform the save for the resulting file to count
as a save-format observation.

Two records in the fixture's `Level000.sav` are now identified by controlled
live reads and game-written saves: `0:5:4986212` is current mana, and
`0:5:4986214` is maximum mana. Both are two-byte values. See the
[dated experiment](../../notes/cheat-save-experiment-2026-09-24.md) for the
control and cheat cases that establish these identities. Other changed
records remain unnamed.
