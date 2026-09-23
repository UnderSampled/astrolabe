# Rust texture slice

The [[docs/openspace/cnt-format|CNT archive]] locates [[docs/openspace/gf-format|GF images]] by member path. The texture model retains useful image information: exact integer color components and their precision, palette entry identity (including unused/duplicate colors), per-pixel indices, every supplied mip level, image usage, and the engine-exposed color-key values. It does not retain GF headers, RLE tokens, escape-byte selections, archive offsets, XOR keys, or original encoded files. PNG is a presentation output for Godot, not the reconstruction source.

`Texture::rgba8(mip)` produces top-left RGBA8 without modifying the exact image values. Material images reverse source row order; screen images retain it. RGB565 screen images reverse red/blue interpretation compared with material images. The color-key values are retained separately because whether a material enables color-key transparency belongs to the material/renderer; the image projection does not infer that flag.

The public APIs are in `model::texture` and `openspace::texture`:

- `decode_gf(bytes, TextureUsage)` and `encode_gf(&Texture)` load/save one useful image.
- `decode_cnt(bytes, TextureUsage)` and `encode_cnt(&TextureArchive)` load/rebuild a texture archive.
- `Texture::{validate,width,height,rgba8}` serve inspection and target adapters.
- `TextureArchive::find_entry` resolves a complete TextureInfo `.tga` path to the `.gf` member using Windows separator/case rules. It never guesses using a basename.

Rete serializes the canonical `TextureSet` root and its independently shared `Texture` values. `TextureArchive` is the OpenSpace import result containing that root and image dependency map. It must retain the archive root when complete CNT membership/directory order/sharing is needed; individual texture files alone do not describe an archive. An index byte vector uses MessagePack binary values. Exact component arrays and palettes remain structured values inside their owning asset.

## Evidence and supported data

Research began with Raymap's `Assets/Scripts/OpenSpace/FileFormat/Texture/GF.cs` and `CNT.cs`. Engine decompilation confirms the 26-byte Montreal GF header and planar byte-channel RLE in `reference/decompilation/functions/005b5d60_FUN_005b5d60.c`. The header RGB and signed palette-index fields are returned by color-key queries in `005b5820_FUN_005b5820.c` and `005b5a10_FUN_005b5a10.c`; they are not disposable padding. Vignette loading and 24-to-16-bit display conversion are visible in `0050e310_FUN_0050e310.c` and `004773c0_FUN_004773c0.c`.

`005b7700_FUN_005b7700.c` establishes the CNT full-word XOR and checksum operations. Raymap follows this full-word XOR rule, while the archived Astrolabe C# reader also XORs the final 1–3 bytes. Actual checksums on every supplied member corroborate the engine/Raymap rule. The checksum sums decrypted little-endian 32-bit words with wrapping arithmetic; a short tail is neither XORed nor included. Directory checksums sum decoded directory bytes modulo 256.

The supplied archive inventory is:

| Archive | Directories | Members | Observed GF image types |
| --- | --- | --- | --- |
| `disc/Gamedata/Textures.cnt` | 363 | 3,555 | 2,176 indexed (256 RGB palette entries), 452 RGB565, 390 ARGB1555, 537 ARGB4444 |
| `disc/Gamedata/Vignette.cnt` | 17 | 23 | RGB565 screen images |

Their SHA-256 hashes are `bde3b7951b0809b18f556fc09eaa58d22f9cdbd1ae066c8684d72fb1674121da` and `8a221aca15583ca94763f5f03047951909309a4c7cdd114acb37f0eb541b30f4`, respectively. No game-derived fixture is added to Git.

The decoder also supports three- and four-component palettes with 1–256 entries, exercised by synthetic known-byte fixtures. It rejects unsupported GF versions/types, invalid precision, truncated/oversized channels, zero or overflowing RLE runs, incomplete mips, invalid palette indices, trailing GF bytes, invalid archive ranges/checksums, unsafe member paths, and duplicate case-insensitive member paths. Names use reversible CP1252 rather than replacement decoding. The parser applies bounded allocation limits, including 16M pixels per image level and 512 MiB of cumulative decoded pixel/palette storage per CNT. It preflights every decrypted GF header before allocating image arrays and counts every logical alias conservatively toward the archive budget. The supplied archives require 137,846,168 bytes (Textures, 131.46 MiB) and 27,279,360 bytes (Vignette, 26.02 MiB) against this conservative budget, leaving room below that bound.

## Reconstruction and sharing

The GF encoder derives the color type from exact component precision/palette values. It derives pixel counts from dimensions and supplied mips, chooses the lowest byte among those with minimum frequency across all decoded channels, and encodes runs of at least four bytes (or any escape byte). Empirical comparison recovered a historical detail: the final sample of a channel is excluded from its preceding run. This deterministic algorithm needs no retained encoding hints. The encoder does not invent missing mipmaps or requantize component samples.

CNT locations are regenerated from current member data, with complete-word checksums recomputed. The writer produces valid unencrypted archives, so original archive bytes are not claimed: historical obfuscation keys are not meaningful image data and are deliberately absent from Rete. Header/directory/member declarations use binrw; variable strings, XOR, checksums and RLE are bounded helpers owned by the OpenSpace adapter.

The supplied texture archive contains 214 additional logical members that share an existing member's physical image data. Each `TextureEntry` references the owning `Texture` by asset path, with one image value regardless of alias count. The writer emits one regenerated payload for each distinct reference. Editing that image updates all aliases; detach an entry by assigning a new image path and value. Equal images with distinct references remain distinct. Partial overlaps and incompatible declarations for a shared source range are rejected. The same sharing semantics apply before and after Rete persistence.

## Validation

Focused tests in `tests/texture.rs` use independently laid out bytes to verify useful colors, palette/index identity, mip ownership, exact GF reconstruction, declared component precision, row orientation, MessagePack value preservation, changed dimensions/archive member counts, directory lookup, explicit sharing/detachment and malformed input rejection.

The ignored local-corpus test requires the supplied disc data explicitly and compares every re-encoded GF byte to independently extracted source members. It also rebuilds each complete CNT and compares all decoded useful values, membership/order and sharing. An independent reference-copy normalization removes only XOR obfuscation, allowing complete generated CNT bytes (including order, locations and checksums) to be compared externally; the exporter never receives that normalized copy. Run it within the project's overall bounded test harness; the test is named `supplied_texture_corpus_decodes_and_reconstructs_every_gf`. Routine tests do not silently skip this fixture check: it is a separately named optional corpus invocation.

Godot resource writing, actual image display/contact-sheet checks, and application-level Rete-store tests belong to the parent adapter integration. Passing these codec tests alone does not establish rendered parity or complete milestone 2. Material color-key flags, complete original CNT obfuscation-byte matching, GF variants absent from the supported Hype corpus, and further games remain explicit follow-up work.

Measured local validation results are preserved in [the texture validation note](../../notes/texture-validation-2026-09-12.md).
