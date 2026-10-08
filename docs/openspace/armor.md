# Hype armor catalog

Hype keeps its armor catalog in the shared Fix package. The live selected
definition is global to an active level, so imported scenes reference the Fix
catalog while each instantiated level owns one mutable copy shared by its
actors. `fix:/gameplay/armor` identifies the catalog; definition and piece
identities retain their Fix `block 5:0` allocation indices. The index is source
identity only. It does not cache an address or allocation layout in Rete.

## Source layout

Static disassembly of the supplied Hype executable (`MaiDFXvr_bleu.exe`,
SHA-256 `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`)
and read-only inspection of its Fix data establish the catalog root. Fix
initialization `0043a860` reads the catalog pointer and count; `0043a8a0`
initializes each definition and selects the first one. The root comes from
Fix.gpt offset `19044 + 24*n`, where the prerequisite count `n` is the signed
short at Fix.sda offset 408. For the supplied corpus `n=4`, the root is Fix.gpt
offset 19140 and the count is Fix.sda offset 3660. The root pointer is resolved
through Fix relocation data, not the active level's relocations.

Each definition is a 40-byte MMG allocation. It contains three TextureInfo
references, a linked list of 24-byte pieces, a rank-major float matrix with
`piece_count + 1` columns, five break thresholds, the piece count and mutable
damage state. The imported model preserves unknown definition and piece bytes,
normalized piece rectangles, TextureInfo dimensions, color-key state, and
source allocation identities. CNT entries resolve artwork into ordinary
texture assets. Allocation boundaries are checked before typed fields or
pointers are traversed.

## Gameplay operations

The typed AI adapter supports the source operations covered by this slice:

- Procedure 429 selects a catalog ordinal after narrowing to a byte; an
  out-of-range ordinal clears the selection.
- Procedure 430 narrows resistance rank to a byte and maps ranks 5 or greater
  to rank 0. Procedure 431 clears piece damage while retaining the selected
  damage piece.
- Function 226 reads the selected definition's resistance rank. Function 227
  reads the Fix catalog count.
- Function 224 narrows input damage to a byte, applies the selected definition
  when present, subtracts the effective damage from the main actor's byte-valued
  hit points with a zero floor, and returns remaining life.

The Godot adapter writes native catalog, definition, and piece Resources plus
artwork PNGs. Runtime callers share the level-owned catalog copy; the resource
does not reproduce OpenSpace allocation, scheduler, or RNG machinery. The
native RNG preserves uniform selection among intact pieces, but exact source
random-sequence parity is not claimed. Function 225's damage-target UI callback
and procedure 432 remain unsupported. Armor artwork is available to native
consumers; the existing HUD importer remains a separate feature slice.

The implementation does not yet reconstruct edited armor data back into
OpenSpace files. OpenSpace armor writing requires coordinated Fix catalog and
active-gameplay-state export and is explicitly rejected by the current source
reconstructor.

See the [dated implementation checkpoint](../../notes/armor-ai-port-2026-10-07.md)
for current tests and remaining validation limits.
