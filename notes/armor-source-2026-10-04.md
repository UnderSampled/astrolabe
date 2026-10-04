# Shared armor source contract, 2026-10-04

This slice uses static source inspection only. No build, test, conversion,
gameplay or experimental probe was run. The recovered Zeus payload
`reference/hype_patch/MaiDFXvr_bleu.exe` has SHA256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`,
matching the documented engine reference. Read-only `objdump` disassembly is
retained locally under ignored `output/static-port/armor/`. Reference submodule
content and gitlinks were left unchanged by this worker.

Raymap's pinned `AITypes_Hype.cs` names procedures 429–432 and functions
224–227; it does not decode armor definitions. The C# reference snapshot
`b1fba2ec614ffa79983fa0f54755b35f3de6269b` likewise provides names, not layout.
The executable supplies the missing definition, initialization and state API.

## Ownership and initialization

`0043a860` reads one Fix pointer into global `00610b9c` and one scalar byte into
`00610ba0`. These are the armor list and count. `0043a8a0` traverses definitions,
registers each definition's broken count/resistance rank and each piece's
broken/strike bytes for save restoration, then initializes selected armor
`00610ba4` to the first definition. Selection is global, not actor-local.
Fresh source initialization and save restoration are separate paths.

The importer follows source declaration order from the inventory anchor at
Fix.gpt18152/Fix.sda20. The catalogue pointer at GPT18152 is the seventh read
in `0041c290` (`0041c2f9` writes global `006107b0`), so the complete pointer
declaration starts at GPT18128. These offsets describe this Hype source profile;
they are not stored as portable identities:

| Reader | Fix GPT bytes | Fix SDA bytes |
| --- | ---: | ---: |
| `0041c290` inventory | 72 | 384 |
| `00416420` | 448 | 4 |
| `004e5150` | 96 + 24*n | 24 |
| `004161b0` | 8 | 0 |
| `0040f7b0` | 16 | 8 |
| `0040f300` | 136 | 2044 |
| `0040bcc0`, before armor | 140 | 1176 |

`n` is the signed short read by `004e5150`, at Fix.sda408. Its first pointer,
three pointers per `0..=n`, twenty fixed pointers, and three pointers per
`0..n` explain its variable GPT extent. `00416420` also reads sixty pointers
from the *level* stream via `004e9a60`; those do not advance Fix GPT.
The armor root is therefore Fix.gpt`19044 + 24*n`; count is Fix.sda3660.
Static reads of the supplied source declaration files show n=4, root slot19140,
root word`026d1164`, count3, GPT extent19344 and SDA extent3683. Root relocation
uses Fix.rtp and identifies block5:0. The three linked definitions have 40-byte
payloads, piece counts7/5/5 and damage matrices160/120/120 bytes. Their artwork
references identify the shield textures and their pieces have 24-byte payloads.
These are read-only source-byte observations, not converter execution.

The first implementation mistakenly counted all 72 inventory pointer bytes
from its seventh field, placing the armor root 24 bytes late at GPT19164. The
user's preparation run exposed this: that slot points to a 188-byte diamond
gauge TextureInfo at Fix6:1 offset469144; flags2 at +8 were incorrectly followed
as a pointer, producing the missing-relocation error at469152. The importer now
uses the complete declaration's start and checks allocation boundaries and
record extents before traversing definitions, artwork, pieces or damage tables.
The strict relocation requirement is unchanged. Re-execution remains deferred.

## Declarative records

`0043ad40` allocates and zeros a 40-byte definition, initializes minimum damage
to one, and allocates a five-rank damage matrix with `piece_count+1` columns.
`0043b120`/`0043b160` append the piece/definition singly linked lists.

| Definition offset | Meaning |
| --- | --- |
| 0 | next definition |
| 4,8,12 | intact, broken, optional background TextureInfo pointers |
| 16 | first piece |
| 20 | float damage matrix, rank-major |
| 24..28 | five strike thresholds |
| 29 | piece count |
| 30 | broken piece count |
| 31 | resistance rank |
| 32 | minimum damage byte |
| 33..35 | unresolved bytes, preserved explicitly |
| 36 | current damage piece pointer |

Pieces are 24-byte allocations: next pointer at0, four normalized rectangle
floats at4/8/12/16, broken byte20, strike byte21, unresolved bytes22/23.
`0043b071..0043b0fa` normalizes artwork coordinates and flips source V;
`0043ab40` reads those stored coordinates and indexes artwork by broken byte.
`00486400` resolves texture names to TextureInfo and `00482e60` reads its
logical width/height at52/48. Texture identity, dimensions and color key are
retained in the model; target RGBA/image conversion remains Godot-owned.

## Implemented direct native API

`0053dd60` dispatches procedures starting at429:

- 429 `Fight_ChangeArmor`: `0053ddc8` calls `0043a9b0`. Narrow the index to a
  byte, select that linked-list ordinal, or null when traversal leaves the list.
- 430 `Fight_ChangeArmorResistance`: `0053de08` narrows the rank to a byte;
  rank>=5 becomes zero. It updates the selected definition's rank.
- 431 `Fight_RepairArmor`: `0053ddf2` calls `0043aa00`. Clear selected broken
  count and every piece's broken/strike bytes. Preserve current damage piece.
- Function226 `Fight_ReadArmorResistance`: `00536430` dispatch reads
  selected definition+31.
- Function227 `Fight_ReadNumberOfArmors`: same dispatcher reads catalog count.

The shared canonical `ArmorCatalog` owns source-ordered definitions and their
private pieces. Source allocation identity accompanies definitions/pieces;
unknown bytes are retained. OpenSpace reads documented fields with binrw,
Rete serializes the useful model, and Godot exports a native Resource graph
with real Texture2D references. Character API methods delegate to that shared
Resource. Missing required catalog/selection fails explicitly; it does not
manufacture an empty/default gameplay catalog. Each level instance must own
one mutable catalog copy shared by its actors; catalog/definitions/pieces are
marked `resource_local_to_scene` for that lifecycle.

## Player damage continuation

`0043aa70` implements damage processing: choose a current unbroken piece using
`004c5860` and global RNG state`00610ba8`, increment its byte strike count,
break when strikes **exceed** the current rank's threshold, and select factor
`rank*(piece_count+1)+broken_count`. Incoming damage is byte-narrowed, scaled,
converted through `005137cc`, byte-narrowed, then minimum damage is applied.
The PE import table resolves `005137cc` through IAT `005c05d0` to MSVCRT
`_ftol`, the truncating float-to-integer conversion. Strike increment wraps as
a byte, and the break predicate is strictly greater than the rank threshold.

Function224 (`00536452`) evaluates its argument once and invokes `0043aa20`.
That helper narrows the input to a byte, uses the selected armor when present,
and otherwise passes raw byte damage to the main actor. `00460b10` reads the
actor's StandardGame life byte, subtracts the damage with a zero floor, writes
it back and returns **remaining life**, not damage dealt or a Boolean. Maximum
life and cap do not participate in subtraction. These operations are now
implemented in the shared native armor Resource and actor-local hit-point
Resource; the compiler guards both before mutating them.

Source RNG helper `004c5860` selects a bounded piece index from its engine
random table/channel; `0043aa70` retries broken pieces. The native catalog owns
a Godot RandomNumberGenerator and selects uniformly among intact pieces when
the current piece is absent/broken. It retains the current piece between hits
and clears it only when broken, preserving that gameplay rule without porting
the engine's random table/index machinery. Exact random sequence matching is
not claimed. A malformed authored matrix/state is rejected before armor or
life mutation. No death transition is invented: compiled AI observes zero
life and must select the source reactions/death behavior.

Function225 has a separate target-life presentation callback at `0040c4a0`;
it remains unsupported until that UI behavior is represented. Procedure432
and complete combat/contact/death remain open. This continuation was inspected
statically only, with no new runtime or regression acceptance.

## Remaining source-supported work

The existing armor ProgressBar HUD is not a source-faithful armor renderer.
`0043ab40` draws each piece using intact/broken artwork and a normalized
rectangle, with optional background in `0043ac60`. Source presentation globals
`00752690/94` and `007526a0/a4` supply its position/scale. Their declaration and
viewport semantics still need promotion before replacing the HUD placeholder.
The new native Resource graph provides those real artwork and piece inputs;
it does not seed a fictional armor total or percentage.

The cinematic Hype/statue overlap remains a separate issue. Actor-directed
display/opacity compilation and native rendering support can preserve those
source calls, but armor selection alone does not prove the placement/visibility
cause or repair the overlap.
