# Hype PC save-game files

This describes the supplied English PC `SaveGame0` fixture and the relevant
Hype engine save path. It is a structural research baseline, not yet a mapping
from saved records to every gameplay variable. The fixture is local at
`reference/Savefile_EN.zip` and remains unchanged. Orrery's read-only Rust
`save-inspect` and `save-diff` commands are in
[`crates/orrery/src/save.rs`](../../crates/orrery/src/save.rs).
The current [investigation results](../../notes/savegame-results-2026-09-27.md)
separate structural decoding, field identification, reload behavior and Godot
persistence.
The [current semantic-map checkpoint](../../notes/savegame-semantic-map-progress-20260928.md)
reports coverage across the supplied slot and 37 available level packages.
The [durable evidence ledger](../../notes/savegame-semantic-evidence-20260928.md)
records source anchors, verified meanings and limits. Detailed report links in
this chapter point to ignored, game-derived local evidence; those reports are
not distributed with a clean checkout.

## File roles and observed contents

| File | Stored bytes | Decoded bytes | Observed content |
| --- | ---: | ---: | --- |
| `General.sav` | 8 | 8 | Two little-endian `u32` values, both zero in this fixture; written directly by `00431390` |
| `General000.sav` | 46 | 38 | Four CRLF-delimited lines: `manoir`, `Gogoud's Manoir I`, then game-clock low/high words `249589`, `0` |
| `Level000.sav` | 13,420 | 17,405 | 1,271 SAI save-table records |
| `manoir.sav` | 2,847 | 3,583 | 263 SAI save-table records |

The game writes `General000.sav` by formatting the current level identifier,
display text and the two 32-bit words of its millisecond game timer into a
temporary text file, then encoding it. The same timer is an eight-byte
player-file record at `0:5:4`, sampled before saving. It writes `Level000.sav`
through the player-save table route and the
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
changed. With the explicit semantic profile, it also reports
`verified_semantic_changes` separately from `encoded_changed` and
`record_changes`. That collection includes only mapped fields with
fresh-process reload evidence and a value interpretation verified for the
field; unknown records remain raw changes. A changed record is evidence of
saved bytes, not yet proof of a named gameplay variable; match it to a live,
typed Global/DSG observation and a controlled input to establish that mapping.
Orrery does not currently offer
an in-game save command. Its input and memory tools can support a logged
experiment, but the game must perform the save for the resulting file to count
as a save-format observation.

Two records in the fixture's `Level000.sav` are now identified by controlled
live reads and game-written saves: `0:5:4986212` is current mana, and
`0:5:4986214` is maximum mana. Both are two-byte values. See the
[dated experiment](../../notes/cheat-save-experiment-2026-09-24.md) for the
control and cheat cases that establish these identities. Other changed
records remain unnamed.

Two independent isolated Orrery runs on 2026-09-27 then proved the current
mana record across a normal game save, process stop, and fresh-process load:
live 30/50 → `druidik` 50/50 → game-written `Level000.sav` value 50 →
freshly loaded 50/50. See the [dated results](../../notes/savegame-results-2026-09-27.md#original-game-mana-proof).
The `thereyougo` magic-inventory case additionally wrote current/maximum mana
as 250/250, then loaded both values in a fresh process. Two Fix inventory
handle selected-state bytes have also been mapped: `0:10:1560` is `MagicIce`
54 and `0:10:2800` is `MagicLightning` 74. Both one-byte player records
changed `00` to `01` and both typed entries were active after a fresh reload.
Hype's `INV_fn_bIsSelected` reads the handle byte at `+8`; item availability
is a separate member byte at `+0x19` and is not mapped by these records. A
fresh-process load then drew the sword with X, selected Ice 54 with DEL, and
cast it with Return: typed mana fell from 250 to 200 and the frame showed an
ice effect. The [dated result](../../notes/savegame-results-2026-09-27.md#original-game-magic-inventory-reload-proof)
retains the source save and input evidence.

The hidden manor lever supplies one level-owned state mapping in this same
profile. `iDummy_PourCercles` (`level:/persos/world/0/27`) has Boolean
designer value `DSG[8]` at Fix block 0, module 17, offset 297732; its source
definition marks the field `0x0200` for player-file saving. The game-written
`SaveGame0/Level000.sav` record `0:17:297732`, kind `0x10`, flags `0x0204`,
changed `00` to `01` after a physical lever interaction. A separate process
loaded that file, observed the value as true, and reached playable
`astrolabe` after the unlocked `tele11` input. The named `manoir.sav` did not
change in this case. A second one-byte player record changed `01` to `00` but
still has no verified owner. See the [dated source proof](../../notes/savegame-results-2026-09-27.md#original-game-manor-world-state-proof).

## Source-backed field families

The [dated evidence ledger](../../notes/savegame-semantic-evidence-20260928.md)
preserves the source locations, family counts, restoration-order experiment,
negative findings and local-artifact hashes needed to interpret this map
without the ignored research files.

The maintained `tools/save_semantics.py` catalogue keeps record identities
separate from file occurrences and records both complete and partial meanings.
In the supplied manor slot, 1,534 record occurrences reduce to 1,271
contextual identities because all 263 named-level identities also occur in
the player file. Source and runtime joins identify 826 bounded/static fields;
live registration and source loops identify 201 dialogue fields and 60 map
star-entry fields. Five complete meanings have fresh-process runtime
corroboration, 503 more have source-backed explanations, and 763 remain
unexplained. Of those, 369 have a partial byte, bit, or reference-role
explanation. These counts concern this fixture and profile, not all reachable
game states.

The main inventory list registration loop at
[`00417fe0`](../../reference/decompilation/functions/00417fe0_FUN_00417fe0.c)
saves each item's handle `+8` selected Boolean and `+10` signed instantiated
number, plus a separate item-member `+0x14` field. A separately rooted
resource handle saves `+10` but does not register a paired `+8` there. This
supports a family rule for the 62 typed Fix handles where both `+8` and `+10`
appear with the same owner. `INV_fn_bIsSelected` reads `+8`, while
[`INV_fn_wInstanciatedItemGetNumber`](../../reference/decompilation/functions/004a3ba0_INV_fn_wInstanciatedItemGetNumber.c)
reads `+10`. Consumption paths decrement the latter, but negative numbers
receive special handling; they should not be simplified to an unlimited-item
claim. The live inventory inspector's item-member `+10` quantity is a
different address, and the relation between these two fields has not been
established. The 60 unbounded `1:10` four-byte saves follow the map
star-entry `+0x38` registration loop's count and stride. The live registry
confirms all 60 exact registrations; their individual map meanings remain
unverified.

The map registration routine at
[`004164d0`](../../reference/decompilation/functions/004164d0_FUN_004164d0.c)
saves 100 map-setting words at record `+0x30`, ten map-state words, four
epoch-availability words, and 60 star-entry words at record `+0x38`. Their
exact record groups are identified by the registration loops and saved
address strides; those structural matches do not establish individual field
meanings. The four epoch words at `0:5:4986392` through `0:5:4986404` are
source-verified: a nonzero word lets the map cycle to its corresponding epoch
slot, and a zero word skips it. The engine's selector maps era values 1, 2, 4,
and 8 to slots 0 through 3. The other 170 map words remain semantically
unexplained, including the ten words reset to `-2` and the 60 dynamic `1:10`
star-entry records. The latter have contextual star-entry ownership through
the source loop and live registry, but the word at `+0x38` still lacks an
explained gameplay purpose.

All 100 map-setting words match distinct bounded 72-byte Fix entries at
`+0x30`. The Hype loader reads that word as an integer from a
language-specific `map/<language>/setting` file before registration; the
supplied values are 55 ones and 45 zeros. The checked local Vignette archive
has no such setting member, and the available decompilation shows no direct
gameplay consumer. Their individual meanings remain unknown. See the
[source survey](../../output/savegame-semantic-map/map-setting-words-followup.md).

The dialogue loader
[`DLG_fn_vLoadDialog`](../../reference/decompilation/functions/0056ba50_DLG_fn_vLoadDialog.c)
registers a list reference, a one-byte member count, and one reference per
member. In the paused manor registry, 201 contiguous `0:21` registrations
form 67 exact reference/count/reference groups. Each saved list reference
points to its corresponding count byte, and every count is one. The 67 count
fields are source-verified as the number of member references. The other 134
records have identified link roles, but their referenced dialogue content and
authored identities remain unknown. The registration ordinal identifies a
list only within this post-load context.

The sector-character link provides a source-backed rule for 53 Fix `0:5`
references. Each rooted Perso's `+0x28` link reaches a bounded 24-byte
character-list allocation; the saved field at `+4` points to a sector-list
element. That element's `+4` pointer reaches one rooted level sector. The
supplied manor values assign 50 actors to `level:/scene/world/1/28` and
three to `level:/scene/world/1/5`. The engine's
[`00447bf0`](../../reference/decompilation/functions/00447bf0_FUN_00447bf0.c)
updates this link after finding the character's sector; when geometric
research finds none, it assigns a fallback list element. The
[`SECT_fn_hResearchInWhatSectorIAm`](../../reference/decompilation/functions/004e0020_SECT_fn_hResearchInWhatSectorIAm.c)
returns the list element. Thus the saved link shows assignment, not a proof of
physical containment. The 54th `0:5` reference has a different allocation
shape and remains unresolved. These scene paths describe the supplied manor
context; the save reference itself retains the source block/module/offset.

Sixteen other saved references reach bounded 64-byte type-`0x12`
instantiated-item SuperObjects. Their `+4` data pointers reach typed Fix
inventory handles, so the target item's source name and ID are known. Eleven
of these point to `NotFound 50` in the supplied slot and to named Ice, Fire,
and Lightning items in a retained magic-labelled save. The source fields,
slot roles and reason for the switch remain unresolved, so all 16 are partial
meanings. The exact target allocations are absent from the imported transform
pool even though that pool shares Fix block/module `6:1`; the
[`source-backed comparison`](../../output/savegame-semantic-map/item-reference-followup.md)
records the individual targets and bounds.

The manor lever `iLevierSecretManoir`, `level:/persos/world/0/174`, has a saved
one-byte Boolean `DSG[1]` at `1:17:187072`. Its source AI model
`level:/ai/models/1396` reads that value in Normal behaviors 2, 7 and 8;
behaviors 7/8 choose paired actions and steady poses, then flip the Boolean.
The observed game-written 01→00 change after physical interaction supports
this alternating phase interpretation. It does not establish which value is
the physical up/down position or whether either value means the passage is
open. `iDummy_PourCercles` `DSG[8]` remains the separately runtime-corroborated
unlock Boolean.

Three manor mannequin actors share AI model 939 and carry separate saved
designer buffers. Each actor's `DSG[0]` is a stage code: the world AI assigns
1/2/3 to sword, crossbow and magic actors; code 4 follows the actor's match
condition and is read by MasterYoda for a later action. Each `DSG[1]` is the
Boolean permission for that actor to advance to code 4 after the match.
These six one-byte records have source-verified operational meanings, with
all three stage values saved as 4 and all three permissions as 1. Their
`DSG[2]` fields have known writes but no verified reader and stay unknown.
See the [mannequin evidence](../../output/savegame-semantic-map/mannequin-dsg-followup.md).
The separate Global `DSG[152]` is a shared Boolean gate read as zero by
the mannequin model's Reflex slot-match path. IWorld branches clear it for
selector values 2–4 and set it for 5–6; a Yoda behavior also sets it.
Its complete authored purpose remains unknown, so this Global record has
only a partial meaning. See the [Global gate trace](../../output/savegame-semantic-map/global-dsg152-followup.md).

The source actor at `level:/persos/world/0/1` is a rooted appearance of the
canonical Fix Global actor `fix:/persos/15`. The join compares their actual
source Perso address and attributes its 198 saved designer fields to the Fix
actor, while retaining the level appearance as context. This is why the
rooted Rete appearance can have an empty brain while the Fix Global model has
238 typed designer variables. Saved Global Boolean `DSG[18]` at
`0:17:4701`, gates manual camera look in the typed `StdCamer` AI: zero permits
entry when the look button and companion state allow it, while one blocks
entry and triggers exit from the active look behavior. Hype's AI writes this
field during related state transitions. The source-supported gameplay role
does not establish fresh-process restoration or every native consumer; see
the [Global camera analysis](../../output/savegame-semantic-map/global-dsg18-followup.md).
Its companion `DSG[17]` at `0:17:4702` is the manual camera-look permission:
one allows the pressed look input to enter the mode when DSG[18] is zero;
zero prevents entry and is an exit condition from active look behavior. The
13 Hype AI writes set it to one or zero under model-local guards, while two
save-test actor writes restore a snapshot. These source-AI roles are narrow;
the full write lifecycle and fresh-process restoration remain unverified.
See the [companion-field evidence](../../output/savegame-semantic-map/global-dsg17-followup.md).
Global `DSG[0]` has a separate, partial relationship: Hype's initial
schedule activates channel 72 when it equals one and deactivates that
channel otherwise. The channel's authored target and gameplay effect are
unknown, so the field remains semantically unresolved. See the
[channel trace](../../output/savegame-semantic-map/global-dsg0-followup.md).
Global `DSG[1]` also synchronizes Hype channel 73 and gates several
weapon-pad and crossbow input branches; `DSG[2]` is set during a Hype
channel/action transition, cleared during a later transition, and gates
Armor HUD display when zero. Both affect statue channels as well. The
authored channel targets and complete gameplay states are unresolved, so
neither field has a complete meaning. See the [DSG[1]](../../output/savegame-semantic-map/global-dsg1-followup.md)
and [DSG[2]](../../output/savegame-semantic-map/global-dsg2-followup.md)
traces.
Global `DSG[30]` at `0:17:4396` stores a 12-byte vector copied from Hype's
`SelfSighting` by a manor checkpoint. Hype later negates its X and Y
components and passes them with zero Z to `ChangeSighting`. This is a
source-verified checkpoint sighting snapshot, while the native orientation
result remains unverified. See the [sighting trace](../../output/savegame-semantic-map/global-dsg30-followup.md).
The neighboring vectors are distinct: the checkpoint writes Hype
`SelfPosition` to Global `DSG[29]`, but no ordinary reader has been found;
it writes a local vector to `DSG[31]`, whose explicit producer reads a
Boolean field. Both remain partial unknowns, including their normal
restoration effects. See the [vector trace](../../output/savegame-semantic-map/global-dsg29-31-followup.md).
Global `DSG[37]` gates that later `ChangeSighting` call when it equals one.
Hype sets and clears this Boolean during an action sequence, but the source
does not connect every set to a checkpoint interaction. Global `DSG[4]`
gates Hype and statue object-table swap calls and Hype material-diffuse
calls when it equals one. The exact authored purpose of either flag remains
unresolved. See the [DSG[37]](../../output/savegame-semantic-map/global-dsg37-followup.md)
and [DSG[4]](../../output/savegame-semantic-map/global-dsg4-followup.md)
traces.
Global `DSG[39]` separates Boolean branches in the manor world script and
the time-travel actor, with IWorld selector value 6 setting it to one. The
guarded operations remain partly undecoded, so the field has only a partial
time-travel relation. See the [DSG[39] trace](../../output/savegame-semantic-map/global-dsg39-followup.md).
Nearby saved Global Booleans have narrower source-backed roles:
`DSG[41]` and `DSG[43]` separately gate object deactivation and two
time-travel behavior routes; `DSG[44]`, `[42]`, `[40]` and `[38]` select
configuration tiers 4, 3, 2 and 1 in that order, with an all-false tier 0.
The script writes the selected tier to unsigned-byte `DSG[73]`, which Hype
tests for 4 and IWorld can reset to 0. `DSG[45]` is copied into a Hype
Boolean that the tier-4 check can overwrite. `DSG[66]` selects several manor actor branches, `DSG[219]`
participates in an IWorld gate and a Yoda self-removal condition, and
`DSG[233]` gates Hype's ActivateMap input branch. The complete gameplay
meanings of these fields remain unknown because downstream effects or normal
writer lifecycles are incomplete. See the [dated source map](../../notes/savegame-semantic-map-progress-20260928.md)
for the individual evidence reports.
Global `DSG[145]` at `0:17:4564` selects one of two four-vector placement
presets in the manor time-travel script. Value one selects its variables
15–18, and zero selects 11–14; the travel camera also uses the switch for a
sound cue. The saved fixture contains one. The two locations' authored names
and restoration order remain open. See the [travel-preset trace](../../output/savegame-semantic-map/global-dsg145-followup.md).
Global `DSG[146]` is a shared Boolean signal used by two manor fixed-camera
actors. They set it on branches requesting camera state 3, clear it during
reset or transition branches, and select a different behavior when it is
one. The actual camera-state effect and actor arbitration are unresolved,
so the catalogue keeps this as partial. See the [camera trace](../../output/savegame-semantic-map/global-dsg146-followup.md).
Nine Global fields `DSG[220]` through `DSG[228]` are actor-owned Boolean
latches. Their object-template AI branches test zero and assign one; six
also have linked manor vial, herb or gold model sites. The surrounding AI
conditions and actions are not fully decoded, so none has a verified pickup
or quest-event meaning. The catalogue retains all nine as partial unknowns;
see the [object-template trace](../../output/savegame-semantic-map/global-object-cluster-followup.md).
The manor Global fields `DSG[147]` and `DSG[149]` are also only partly
resolved. Value one of DSG[147] triggers the statue actor's typed `KillSelf`
branch after an `iGogood` action sets it. DSG[149] gates a grille behavior
sequence and has IWorld set/clear paths. The save-test actor can snapshot,
restore or force both fields to one, so their saved bytes do not establish a
particular progression event. See the [manor gate trace](../../output/savegame-semantic-map/global-manoir-147-149-followup.md).
Global `DSG[32]` is a Perso reference assigned by the manor checkpoint
script. The supplied save targets the rooted checkpoint actor at
`level:/persos/world/0/35`; a source branch can instead assign the main
actor. No typed gameplay reader or restoration outcome has been verified,
so the reference remains partial. See the [checkpoint target trace](../../output/savegame-semantic-map/global-dsg32-checkpoint-target.md).
Separately, 176 saved four-byte words belong to
individual Perso StandardGame `CustomBitsInit` fields: 123 level records and
53 Fix records. The source parser writes the named field at `+0x2c`, and the
Hype save path registers that same offset. Camera targeting uses bit 0. One
action path uses bit 7 to bypass segment-versus-world collision checks before
moving a target, although that bit is clear in all supplied fixture values.
The other bits and full gameplay effect remain unknown; these are partial
meanings, not 176 completed mappings.

The same actor source join resolves 56 one-byte records at StandardGame
`+0x1b` and `+0x1c`, in 28 actor pairs. The parser's second `InitFlags`
option selects `NeverBack` when the word at `+0x24` is four, and the save
registrar includes exactly these two bytes for that policy. `+0x1b` is a
suppression latch: a nonzero value prevents the actor's return and forces
`+0x1c` to zero. `+0x1c` is the active state when that latch permits it;
the camera's target validation checks the combined result. The join checks
all 28 bounded actor allocations, attributes the Fix Global pair to its
canonical actor, and excludes a nearby record in a four-byte allocation.
This proves the saved state roles from source; individual gameplay triggers
and fresh-process restoration remain to be observed. See the
[working evidence](../../output/savegame-semantic-map/standard-activation-investigation.md).

For the same 28 `NeverBack` actor allocations, the Hype registration path
saves three more one-byte StandardGame fields at `+0x18`, `+0x19`, and
`+0x1a`: 84 records in all, with 78 Level and six Fix records. The authored
names `HitPointsInit`, `HitPointsMaxInit`, and `HitPointsMaxMax` correspond to
runtime current HP, its mutable maximum, and the cap on that maximum.
The join requires the rooted actor, policy four, exact 56-byte allocation,
and record kind/flags/width. Individual gameplay triggers and fresh-process
restoration remain unobserved. See the [hit-point source evidence](../../output/savegame-semantic-map/standard-hitpoints-followup.md).

The Fix inventory join separately resolves 63 two-byte records at item
allocation `+0x14` as `item.max_element_capacity`. Each item is a bounded
32-byte allocation reached through a rooted 24-byte handle. Hype's
`INV_fn_wGetMaxElement` reads that field, and its add-element path clamps
against it. Sixty-two target items have source names; one uniquely rooted
item has no readable member name. Its handle `+0x0a` record at `0:10:5790`
is the current amount of a separately rooted inventory resource, displayed
in the stats panel and spent or gained by inventory effects. The linked
unnamed item's maximum capacity is 200 in the supplied save. This gives the
handle word a source-verified behavior-level meaning, though the authored
resource name is unknown. See the [resource trace](../../output/savegame-semantic-map/special-inventory-word-followup.md).
Individual capacity changes and fresh-process restoration have not been
observed. See the [inventory evidence](../../output/savegame-semantic-map/inventory-max-capacity-evidence.md).

The one-byte `0:5:4986340` record is the inventory-list full flag. The
captured registration order matches the source sequence after the separate
resource handle, its item capacity and four list words. Hype sets the flag
when no free slot remains or the last slot is filled, and clears it after
item removal or consumption. The saved zero is source-verified as a
non-full state; a controlled fill and fresh reload have not been observed.
See the [flag join](../../output/savegame-semantic-map/inventory-full-flag-followup.md).

The Fix gauge join follows declared relocation pointers through three
40-byte root allocations and 17 bounded 24-byte child allocations in block
`0:5`. Each root's saved `+0x1e` byte counts completed children. Each child's
saved `+0x14` visual phase and `+0x15` progress tick participate in the
threshold update and rendering path. These 37 one-byte records have
source-verified roles. The three root `+0x1f` threshold selectors are owned
and typed, but their authored purpose and legal values remain unresolved;
the catalogue keeps them unknown with partial structural evidence. Root and
child source-list ordinals are diagnostic identities, not authored panel or
item names. See the [gauge pointer-chain evidence](../../output/savegame-semantic-map/gauge-pointer-chain-followup.md).
The first gauge root's saved reference `0:5:4985156` points to the next
root in that validated chain. The source join verifies the exact target and
rejects a mismatched root, giving this reference a source-verified list-link
meaning. Its authored panel names and fresh-process restoration remain
unverified.

The excluded `1:5:1205048` byte is instead the level's game-state mask
registered by `004483a0`. Its padded four-byte allocation begins at the
saved offset, but the registered value is one byte. The sector sound-event
path plays a matching event when this mask intersects the event's state
mask, or when both masks are zero. Map-era selectors also read the byte,
with cases for values 1, 2, 4 and 8. Its bounded sound-filter role is
source-verified; the individual bit names, writers and live restoration
remain open. The [independent source check](../../output/savegame-semantic-map/epoch-byte-check.md)
records the allocation, registry and call evidence.

The magick registration cluster at
[`00439830`](../../reference/decompilation/functions/00439830_FUN_00439830.c)
registers four adjacent two-byte player-file records. Current and maximum
mana at `0:5:4986212` and `0:5:4986214` are runtime-corroborated. In
`0:5:4986216`, the low byte is the selected magic slot/index (0–2), used by
the gauge and selection functions; the high byte has no verified meaning.
That record remains partly unexplained, and no nonzero selection has survived
a dedicated fresh-process reload check. The preceding `0:5:4986210` two-byte
value is the unsigned `DefaultMagicPoints` starting amount. Bounded
disassembly of the pinned executable's `MagicLevel` callback shows it caps
this value at configured maximum points and use it to initialize current
points; its fresh-process restoration order remains unverified. See the
[callback evidence](../../output/savegame-semantic-map/magick-config-followup.md).

The eight-byte record `0:5:4` is Hype's millisecond game-clock value.
`DAT_00610810` points to its exact bounded Fix allocation and is registered
for saving as eight bytes. The save-name confirmation path reads the running
timer into both that allocation and the two decimal clock lines of
`General000.sav`; the supplied value is 249,589 milliseconds. After a
player-mode load, startup resets and starts the timer, then advances it to
the saved value if the current low word is smaller. The timer library's
backend zero uses `timeGetTime()` and 1,000 ticks per second. This is a
source-verified clock role; exact pause/menu inclusion and nonzero high-word
wrap behavior remain unobserved. See the [clock evidence](../../output/savegame-semantic-map/save-timer-followup.md).

Two 36-byte player-file records at `2:6:1463692` and `2:6:1464196`
are consecutive saved prefixes of 88-byte POS matrices. The first is the
main actor Hype's position; source traversal makes `StdCamer` the strongest
candidate for the second, pending its collision-entry predicate. Their
uncompressed translation components are at byte offsets 4, 8 and 12.
The remaining saved bytes and full restoration behavior are unresolved, so
both records remain partial. See the [registration](../../output/savegame-semantic-map/position-snapshot-followup.md)
and [owner](../../output/savegame-semantic-map/position-owner-followup.md) evidence.
The following saved word `0:5:4986280` is separately registered from
`DAT_00752414`. It stores the low 16 bits of a nested ToolCam slot-1
object's `+0x98` dword, sampled before save and restored after player load.
Other code uses state-like values at that offset, but their gameplay meaning
is unresolved; see the [word trace](../../output/savegame-semantic-map/post-load-word-followup.md).

## Registration and restoration findings

Raymap's [Perso save-data view](../../reference/raymap/Assets/Scripts/OpenSpace/Object/Perso.cs)
is a practical lead for designer-variable ownership, but its `0x40 | 0x80`
test is for Rayman 2. Hype's pinned executable supplies the rules here.
`004c68e0` registers scalar, array and pointer records, captures their current
values into the table, and identifies entries by address and normalized flags.
Orrery's bounded private live capture in playable `manoir` traversed 2,180
registration nodes; 2,176 mapped into the loaded MMG block table and four did
not. All 1,271 identities in the supplied save had an exact live kind/flags
registration, including the 201 dynamic `0:21` records and 60 dynamic `1:10`
map-star candidates. The registry also had 645 identities absent from that
save fixture. This establishes presence for one post-load state. The dialogue
loader resolves the structure of the `0:21` family, while its target content
and the `1:10` entries' individual field meanings remain unresolved. The
[dated evidence](../../notes/savegame-semantic-map-progress-20260928.md)
records the hashes, phase, counts and limits.
`004c66c0` adds player/level eligibility bits `0x0004`/`0x0008` from the
selection flags. `004c6e30` confirms that bits `0x0100`/`0x0400` write the
captured table value and `0x0200`/`0x0800` read the current address when the
game writes a file. This means a record's saved value may be the registration
snapshot rather than its value at save time. See
[`004c68e0`](../../reference/decompilation/functions/004c68e0_FUN_004c68e0.c),
[`004c66c0`](../../reference/decompilation/functions/004c66c0_FUN_004c66c0.c), and
[`004c6e30`](../../reference/decompilation/functions/004c6e30_FUN_004c6e30.c).

`004c7250` decodes records into the table, and `004c7300` resolves each saved
block/module/relative address against the current allocation context. Its
pointer payload stays block/module/relative until an apply pass. The
`004c6430` → `004c6d00` → `004c6c20` path applies selected table values to
current memory, including arrays and resolved pointer targets. The observed
startup caller `00402780` applies mode 1. Its later player-file mode-4 apply
is guarded by `DAT_00754420 == 3`. The level startup path at
`00402b60`/`00402910` applies named-level mode 8 when
`DAT_00754420 == 5`. These are different guarded startup modes. In a controlled
first-slot manor load, two isolated copies set the same registered actor byte
to conflicting player/named values of 0/1 and 1/0. At playable phase 9 the
byte was 0 and 1 respectively, matching the player file in both runs. The
[retained report](../../output/savegame-investigation/source-player-named-precedence-proof-1790578160771347593/report.json)
proves the observed first-slot result for that field, while named-level mode-5
behavior needed a separate check. The state-6 level-transition handler first
calls `004312c0` to write the outgoing level's named save with mask `0x0c00`,
then changes the active name and enters state 5. On tele11 departure from
manor, the isolated working `HYPE/manoir.sav` was rewritten with the live
player-valued byte; the copied `SaveGame0/manoir.sav` retained the original
opposed byte. A controlled follow-up reinserted the opposing byte into only
the isolated `HYPE/manoir.sav` after arrival at astrolabe. Both tele05 returns
sampled mode 5 and reached playable manor with stable values matching that
named working file (1 and 0 in the two opposed cases). This demonstrates
named-file mode-8 application for this field under the documented disk-file
intervention. The ordinary, unmodified tele11/tele05 result is inconclusive
for source-file precedence because the departure save erased its conflict.
See the [transition result](../../output/savegame-semantic-map/named-level-mode5-probe-20260928/final-report.json)
and [`00404700`](../../reference/decompilation/functions/00404700_FUN_00404700.c),
[`004312c0`](../../reference/decompilation/functions/004312c0_FUN_004312c0.c), and
[`004c7250`](../../reference/decompilation/functions/004c7250_FUN_004c7250.c),
[`004c7300`](../../reference/decompilation/functions/004c7300_FUN_004c7300.c),
[`004c6c20`](../../reference/decompilation/functions/004c6c20_FUN_004c6c20.c), and
[`00402780`](../../reference/decompilation/functions/00402780_FUN_00402780.c).

Orrery's schema 2 inspection preserves relative slot paths and reports unknown
record-type tails without inventing a length. Its optional
`--profile hype-pc-english-supplied-save` annotates the two known mana fields,
two inventory selected-state flags, and the manor dummy unlock Boolean. This profile is selected by the caller;
save files do not prove the
executable build. For a game-written flat slot directory, use
`save-diff ... --after-slot SaveGame0` to compare it with the supplied ZIP.
An unsigned display value in a report is not a verified gameplay type.
