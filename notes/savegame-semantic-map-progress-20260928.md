# Hype save semantic map — working progress, 2026-09-28

This is the live checkpoint for the [comprehensive mapping handoff](savegame-semantic-map-handoff-20260928.md). The [durable evidence ledger](savegame-semantic-evidence-20260928.md) keeps key meanings, source locations, negative findings, and local-artifact hashes in versioned prose. The [first persistence slice](savegame-results-2026-09-27.md) remains the five-field runtime baseline. Current work maps the pinned English source corpus, and its counts are **not** a claim of complete save meaning or original-save import support.

## Repeatable corpus and ownership commands

```sh
mkdir -p output/savegame-semantic-map
cargo run -p orrery --locked --offline -- save-inspect reference/Savefile_EN.zip \
  --profile hype-pc-english-supplied-save \
  --output output/savegame-semantic-map/baseline-inspect.json
cargo run -p orrery --locked --offline -- save-source-join reference/Savefile_EN.zip \
  --game disc --level manoir \
  --output output/savegame-semantic-map/manoir-allocation-joins.json
python3 tools/save_semantics.py \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --inspection output/savegame-semantic-map/baseline-inspect.json \
  --allocation-joins output/savegame-semantic-map/manoir-allocation-joins.json \
  --live-registry output/savegame-semantic-map/registry-live-final-validated/save_registry.json \
  --output-root output/savegame-semantic-map
python3 tools/save_level_discovery.py \
  --manifest output/savegame-semantic-map/corpus-manifest.json \
  --binary target/debug/orrery \
  --output-root output/savegame-semantic-map/levels
```

These commands verify the supplied archive and executable hashes, leave their bytes untouched, and write raw game-derived catalogues only under ignored `output/`. The generated [corpus manifest](../output/savegame-semantic-map/corpus-manifest.json), [registration index](../output/savegame-semantic-map/registration-index.json), [ownership joins](../output/savegame-semantic-map/manoir-allocation-joins.json), [catalogue](../output/savegame-semantic-map/semantic-catalogue.json), and [coverage report](../output/savegame-semantic-map/coverage.json) are the current evidence. The maintained tool consumes Orrery's decoder rather than implementing another save parser.

The optional `--live-registry` input is the read-only paused snapshot captured
by Orrery's private original-game inspection. Its sibling `worker.json`
records the reached phase/level and pinned executable/fixture hashes, which
the catalogue generator checks before incorporating registration evidence.
Recreate that snapshot with `target/release/orrery inspect-original --config
/home/stephen/Projects/hype-harness/local/config.json --output
output/savegame-semantic-map/<new-evidence-directory>` using the approved
private Gamescope runtime path. Each output directory must be new.

## Reconciled first manor pass

The manifest identifies 37 level packages plus shared Fix source files and 20 retained, complete `written-saves` capture roots. The supplied slot contains four files: two metadata files outside the SAI table, 1,271 player-file records and 263 named-manor record occurrences. All 263 named-file identities overlap the player file, leaving **1,271 distinct contextual record identities** rather than 1,534 independent fields. The [level discovery summary](../output/savegame-semantic-map/levels/summary.json) covers **37/37** available level packages and finds **8,431 contextual saved-descriptor occurrences**: 7,585 Fix occurrences collapse to 205 distinct source location/field/type triples, and 846 Level occurrences are distinct when level context is retained. These are field occurrences, not save meanings or proof each field registers in a reachable state.

| Coverage tier | Distinct identities | What the evidence establishes |
| --- | ---: | --- |
| Bounded static allocation | 1,010 | The record's source block and width fit one Fix or level allocation in this manor context; this alone gives no field meaning. |
| No matching static source block | 261 | Block/module `0:21` (201) and `1:10` (60); live registration and source loops now establish contextual ownership for both. |
| Owned and typed | 1,087 | The 826 bounded/static or runtime fields plus 201 dialogue-list and 60 map-star fields joined through live registration and source loops. Dynamic ordinals are contextual. |
| Source-verified meaning only | 503 | Inventory handles, the separately rooted resource balance and full-list flag, 63 item capacities, 84 actor hit-point bytes, the manor lever phase, Global camera and checkpoint fields, four map epoch words, 67 dialogue member counts, 53 actor sector links, 56 NeverBack actor state bytes, one sound-event game-state mask, the game clock, gauge progress fields and first-root link, and configured default magic points. |
| Runtime-corroborated meaning | 5 | Two mana fields, Ice 54 and Lightning 74 selected flags, and the manor dummy unlock. |
| Meaning unresolved | 763 | An allocation, DSG index, type, or owner alone is not a gameplay explanation. Of these, 369 records have a partial byte/bit/reference-role explanation. |

The 62 inventory selected flags resolve through 24-byte Fix handle allocations, checked item member/name/ID references, the `00417fe0` registration loop, and Hype's `INV_fn_bIsSelected` accessor. The two previously tested magic flags are among them. Each of the 62 paired handle `+0x0a` two-byte records is the signed instantiated item number/quantity: Hype's same main-list registration loop saves `+8` and `+10` together, while a separately rooted resource handle saves `+10` without `+8`. The accessor and item-consumption paths read, decrement and set the main-list number. Its relation to the separate item-member `+10` quantity read by the typed live inspector remains unverified. Do not conflate these with the **60 unbounded `1:10` four-byte records**: the map loader and registration loop identify them as star-entry `+0x38` words by count and stride, and the live registry confirms all 60 exact registrations. Their gameplay meanings remain unknown.

A separate [bounded inventory item join](../output/savegame-semantic-map/inventory-max-capacity-evidence.md) explains **63** Fix `0:10` two-byte records at 32-byte item allocation `+0x14` as the maximum element capacity used to clamp additions. Each target is reached through a 24-byte handle. Sixty-two items have source names; one is uniquely rooted but unnamed. A [focused source trace](../output/savegame-semantic-map/special-inventory-word-followup.md) now explains that item's handle word `0:10:5790` at `+0x0a`: its current resource balance is displayed in the stats panel and added or spent by inventory effects, with the linked capacity of 200 limiting additions. Its authored name and fresh-process restoration remain unknown. The catalogue keeps the balance and capacity as distinct records and checks their exact allocation geometry and save shapes.

A [registry-order join](../output/savegame-semantic-map/inventory-full-flag-followup.md)
identifies the separate one-byte `0:5:4986340` as Hype's inventory-list full
flag. The paused registry's seven-entry tail matches the source registration
sequence exactly. The source sets it when no free slot remains or the last
slot is filled and clears it after removal or consumption. This is a complete
source-verified operational meaning; a controlled fill/reload remains open.

A typed Perso → Brain → Mind → designer descriptor/current-buffer join identifies 221 saved `DSG[index]` fields and their source actor, declared type and exact extent. The dummy's `DSG[8]` has a proved gameplay purpose. The manor lever's `DSG[1]` is now source-verified as the Boolean phase used by AI Normal behaviors 7/8 to alternate paired actions and poses; its saved 01→00 change after physical interaction corroborates the toggle, but does not identify physical up/down or passage state. The [lever AI model evidence](../output/savegame-semantic-map/manoir-lever-ai-model.json) comes from `level:/ai/models/1396`. Rebuild it by importing the manor scene with `astrolabe import-openspace disc <ignored-output> --level manoir`, then inspecting that asset. The join found 176 Perso nodes and 819 descriptors in manor; 105 Persos legitimately have no Brain pointer, while **zero** actors failed traversal and all 221 saved descriptors matched catalogue records.

Three manor mannequin actors share AI model 939 but have separate saved
designer buffers. The [source trace](../output/savegame-semantic-map/mannequin-dsg-followup.md)
resolves each actor's `DSG[0]` as a stage code and `DSG[1]` as the Boolean
permission for its matching branch to advance the code to 4: **six complete
source-verified meanings**. World and MasterYoda AI provide the code writes
and follow-on read; all six saved values are 4 or 1. Each actor's `DSG[2]`
has known write conditions but no verified reader, and a separate world
branch sets the same value, so those three fields remain partial unknowns.
The distinct [Global DSG[152] trace](../output/savegame-semantic-map/global-dsg152-followup.md)
shows a shared zero-gate on the mannequin Reflex slot-match path. IWorld
selector branches 2–4 clear it and 5–6 set it; Yoda also has a set-one
path. Its full authored event meaning remains unresolved.

The rooted `Global` appearance at `level:/persos/world/0/1` aliases the canonical Fix actor `fix:/persos/15`. The join now compares their typed source addresses and records the Fix owner plus the level appearance for its 198 saved designer fields. This resolves the apparent mismatch between the level appearance's empty Rete brain and the populated Fix Global AI model. The separate StandardGame source path maps **176** four-byte `CustomBitsInit` records (123 Level, 53 Fix) to individual actor owners in bounded 56-byte allocations. The engine's map/actor parser names and registers this field at `+0x2c`; camera targeting reads bit 0. A separate action path checks bit 7 to bypass segment collision tests before moving its target, but bit 7 is clear in every supplied fixture value. A [focused Hype usage audit](../output/savegame-semantic-map/custom-bits-usage-followup.md) found only two multi-bit fixture values, on `iIcons_Manoir` (`0x43`) and `iKeys_Manoir_A` (`0x207`); their extra bit purposes remain unproved. The field is registered from current memory and has a source setter, but ordinary post-setup mutation has not been observed. Other bit meanings and full restoration effects remain unknown, so these 176 are owned and partially explained, not counted as complete semantic mappings.

The [inverse AI index](../output/savegame-semantic-map/global-inverse-ai-followup.md) found exact external uses of canonical Global `DSG[17]`, `DSG[18]`, and `DSG[24]`. Focused [DSG[18]](../output/savegame-semantic-map/global-dsg18-followup.md) and [DSG[17]](../output/savegame-semantic-map/global-dsg17-followup.md) control-flow traces source-verify complementary Boolean gates on manual `BT_Camera_Look`: DSG[18] zero permits entry while one blocks/exits; DSG[17] one permits entry while zero blocks/exits. Entry also requires the look button pressed. Hype's AI writes both during related state transitions; the 13 indexed Hype writes to DSG[17] assign one when two model-local conditions hold, otherwise zero, plus a separate clear. `iDummy_SaveTesteurs` snapshots and restores these fields as a test action; this does not explain `DSG[24]`, which remains unknown. Neither gate has a fresh-process restoration observation.

The [nine-field object-template trace](../output/savegame-semantic-map/global-object-cluster-followup.md)
checks Global `DSG[220..228]` against exact source actors and model sites.
All nine are one-byte Boolean clear-to-set latches; six also appear in named
manor vial, herb or gold models. Unsupported enclosing actions and the
unresolved MainActor field 97 prevent specific pickup or quest-event labels,
so all nine remain partial unknowns rather than completed mappings.
The [Manoir DSG[147]/DSG[149] trace](../output/savegame-semantic-map/global-manoir-147-149-followup.md)
similarly identifies a statue-removal gate and grille-sequence latch, but
the enclosing actions are incomplete and `iDummy_SaveTesteurs` can restore
or force both fields to one. They remain partial unknowns.
The [DSG[1]](../output/savegame-semantic-map/global-dsg1-followup.md) and
[DSG[2]](../output/savegame-semantic-map/global-dsg2-followup.md) traces
identify Hype channel/input and transition/HUD effects, respectively, but
cannot name their complete gameplay states. A new rooted target join ties
Global [DSG[32]](../output/savegame-semantic-map/global-dsg32-checkpoint-target.md)
to the manor checkpoint actor in this save; the checkpoint AI can assign
itself or the main actor, and the consuming gameplay role remains unknown.
The [DSG[37] trace](../output/savegame-semantic-map/global-dsg37-followup.md)
shows a Boolean gate on Hype's use of the checkpoint-saved DSG[30] sighting
vector. The [DSG[4] trace](../output/savegame-semantic-map/global-dsg4-followup.md)
shows a Boolean gate on Hype/statue object-table swap calls and Hype
material-diffuse calls. Both stay partial because their complete authored
gameplay states and restoration behavior are unresolved.
The [neighboring vector trace](../output/savegame-semantic-map/global-dsg29-31-followup.md)
shows the checkpoint copying Hype `SelfPosition` to DSG[29] and its own
local vector to DSG[31]. No ordinary DSG[29] reader is indexed, and
DSG[31]'s producer has a vector/Boolean source mismatch. Both remain
partial unknowns rather than inferred respawn coordinates.
The [DSG[146] trace](../output/savegame-semantic-map/global-dsg146-followup.md)
identifies a shared manor fixed-camera signal. Two camera actors set it on
branches requesting camera state 3 and clear it during reset/transition
branches; its actual camera effect and arbitration remain unknown.
The [DSG[39] trace](../output/savegame-semantic-map/global-dsg39-followup.md)
shows a Boolean branch in the world and time-travel actors, set to one by
IWorld selector value 6. Unsupported operations prevent a complete state
label. A separate [DSG[73] trace](../output/savegame-semantic-map/global-dsg73-followup.md)
initially identified its unsigned-byte type and exact sites. The
[shared circle-configuration trace](../output/savegame-semantic-map/shared-pour-cercles-followup.md)
now shows Global DSG[44], [42], [40] and [38] selecting tiers 4, 3, 2 and 1,
with an all-false tier 0; the [DSG[73] follow-up](../output/savegame-semantic-map/global-dsg73-tier-followup.md)
shows that tier being stored as a mutable byte, checked for value 4 by Hype
and reset to zero by IWorld. Its complete authored purpose remains unknown.

Further actor-qualified traces document ten more Global Boolean fields as
partial unknowns. [DSG[41]/[43]](../output/savegame-semantic-map/global-dsg41-43-followup.md)
gate two separate object-deactivation and time-travel behavior routes;
[DSG[66]](../output/savegame-semantic-map/global-dsg66-followup.md) selects
Manoir world, mannequin-frame and Gogood-return branches.
[DSG[219]/[233]](../output/savegame-semantic-map/global-dsg219-233-followup.md)
gate a Yoda self-removal/world sequence and Hype's ActivateMap input branch,
respectively. The [DSG[38]/[40]/[42]/[44]/[45] trace](../output/savegame-semantic-map/global-dsg38-40-42-44-45-followup.md)
identifies ordered selector reads and a Hype schedule read; the selector
branches' terminal effects remain unavailable. A [fresh typed-AI inspection](../output/savegame-semantic-map/rete-ast-recovery-followup.md)
resolved Hype's DSG[45] branch as a Boolean copy into model Variable[57],
while that variable's gameplay purpose remains open. None of these source sites proves a complete
authored gameplay state or fresh-process restoration; the catalogue retains
all ten as unknown meanings.
The [shared-consumer blocker index](../output/savegame-semantic-map/shared-consumer-blockers.md)
groups opaque operations and target behaviors affecting multiple flags so
one source trace can be reused across them.
The [native time-selector trace](../output/savegame-semantic-map/shared-time-selector-followup.md)
resolved `Func_GetGameState` codes 3 and 4 and the two ordered routes,
while the [Manoir behavior-target trace](../output/savegame-semantic-map/shared-change-comport-followup.md)
identified Gogood's return actor as the target of several conditional
behavior changes. Those narrower effects strengthen partial meanings but
still do not identify named gameplay states. A corrected
[Hype Variable[57] audit](../output/savegame-semantic-map/rete-ast-recovery-followup.md)
found both a DSG[45] copy and a later DSG[73]-conditioned write; its
121 reads do not justify naming the saved DSG[45] flag.

The literal decompilation index covers 6,663 function files and **92** calls to nine known wrappers or the central registration routine, including three newly traced aliases (`004c6300`, `004c6320`, `004c6340`). Nine of those calls are between wrappers and should not be read as independent source fields. It stores each call's source line, address/size/flag expression and nearby control-flow hints. The hints do not prove loop ownership or exclude indirect calls. Registry removal (`004c6360`) and group clear (`004c6430`) are separate lifecycle actions, not registrations. In particular, actor designer-variable callback dispatch has not been reduced to a direct call list; the field ownership join does not establish that every designer descriptor is registered in all states.

The map registration loop at `004164d0` binds 174 saved records to four
structural groups: 100 map-setting words, ten map-state words, four
epoch-availability words, and 60 dynamic star-entry words. The catalogue
records that group membership without treating it as a complete gameplay
meaning. The four epoch words (`0:5:4986392`–`0:5:4986404`) are the exception:
`00417030` skips slots whose word is zero while cycling, and `00416e70` maps
era values 1, 2, 4 and 8 to the four slots. Their meanings are source-verified;
the other 170 map words remain unknown. Initialization sets the ten map-state
words to `-2`, but their consumers and value domain remain unverified. A
[map-setting source survey](../output/savegame-semantic-map/map-setting-words-followup.md)
matched all 100 four-byte `+0x30` entry fields to bounded Fix allocations:
55 saved ones and 45 zeros. Hype reads their initial integers from a
language-specific `map/English/setting` file and registers them, but the
checked local Vignette archive has no setting member and no gameplay
consumer was found. Their meanings remain unknown. A
[focused static follow-up](../output/savegame-semantic-map/map-state-word-followup.md)
found no gameplay read or non-initialization write in the checked C export;
it recommends a controlled map-menu memory comparison and executable xrefs.
The [first live probe](../output/savegame-semantic-map/map-state-live-probe.md)
confirmed ten `-2` words at playable manor but expired before post-input
capture. Its [single-key retry](../output/savegame-semantic-map/map-state-live-probe-retry-01.md)
sent the manual's `End` key on the private game display; no map appeared and
all ten words stayed `-2`. Key delivery into gameplay is unconfirmed, so the
result does not assign meanings to the words.

The [live registry capture](../output/savegame-semantic-map/registry-live-final-validated/save_registry.json)
reached playable `manoir` after loading the supplied save. Its bounded list
traversal matched header count and tail at **2,180 nodes**. MMG block tables
mapped 2,176 nodes to 1,916 distinct source identities; four node addresses
were outside the enumerated blocks. Every one of the 1,271 supplied-save
identities had a node with the exact kind and low flags. This includes all 201
dynamic `0:21` identities and 60 dynamic `1:10` map-star candidates. There
were 645 additional distinct live identities beyond the supplied fixture.
These facts prove registration presence in this one post-load state; they do
not by themselves identify those fields' gameplay meaning, prove later lifecycle, or show
which extra identities would be written to a selected save file.

The dialogue loader `DLG_fn_vLoadDialog` registers a list reference, then its
one-byte member count, then one reference for every member. All 201 live
`0:21` nodes are contiguous in that exact pattern, yielding 67 groups of
three. Each first saved reference targets its group's count byte; all 67
counts are `01` in both player and named manor files, and the 20 retained
game-written roots do not vary this family. This proves the middle field's
meaning as a dialogue-list member count and source-verifies **67** additional
records. The other 134 references are owned as list/member links. A
[target follow-up](../output/savegame-semantic-map/dialogue-reference-targets-followup.md)
shows that the 67 member links point to parsed `DialogAnswer_<n>` objects in
the source's `ItemAnswer` array; one target is shared by 12 saved links.
They remain partially explained because individual authored answer keys,
text and effects have not been resolved. Group ordinal is post-load context, not a portable
dialogue identity. The local English [`.dlg`/`.lng` source-file check](../output/savegame-semantic-map/dialogue-source-file-followup.md)
found relocated language-data pointer tables, but no bridge from saved dynamic
targets to authored keys. The canonical family rule and evidence are in the
catalogue.

The rooted Perso `+0x28` link and a bounded 24-byte sector-character list
resolve 53 saved Fix `0:5` references. Their `+4` fields point to level
sector-list elements, whose own `+4` pointers identify rooted sectors. The
saved targets assign 50 actors to `level:/scene/world/1/28` and three to
`level:/scene/world/1/5`; the source path through `00447bf0` and
`SECT_fn_hResearchInWhatSectorIAm` verifies the sector-assignment role.
`00447bf0` uses a fallback list element when geometric research finds no
sector, so assignment does not prove physical containment. The
canonical Fix Global actor retains its `fix:/persos/15` identity and level
appearance. A test covers a valid target and a source-shaped but unresolved
target. The remaining `0:5` pointer in a different 40-byte allocation does
not match this rule.

The StandardGame source join also resolves 56 one-byte fields in 28 actor
pairs, 26 Level and two Fix. Each actor's second `InitFlags` return policy is
`NeverBack` (`+0x24 == 4`), which makes the engine register bytes `+0x1b` and
`+0x1c` with player and named-level eligibility. The first is a suppression
latch that prevents a return when nonzero; the second is the active state,
forced to zero while suppressed. The bounded join excludes a nearby `1:5`
record in a separate four-byte allocation and retains the canonical Fix owner
for Global's pair. All 56 meanings are source-verified; their individual
gameplay triggers and fresh-process restoration remain unobserved. See the
[source investigation](../output/savegame-semantic-map/standard-activation-investigation.md).

Those same 28 rooted StandardGame allocations also contain 84 saved
hit-point bytes at `+0x18` through `+0x1a`: 26 Level and two Fix actors,
with three fields each. Hype's authored names and consumers establish
`HitPointsInit` as current HP, `HitPointsMaxInit` as its mutable maximum,
and `HitPointsMaxMax` as the cap on that maximum. The
[bounded join and source evidence](../output/savegame-semantic-map/standard-hitpoints-followup.md)
require policy four, exact allocation/offset, and record kind, flags and
width. Individual triggers and fresh-process restoration remain unverified.

The nearby `1:5:1205048` record is separate: one registered byte at the
start of a padded four-byte level allocation. `004483a0` registers the
global game-state mask with the same flags, `004480e0` uses it to admit
sector sound events whose masks intersect (or when both are zero), and map
era selectors read its values 1, 2, 4 and 8. The [independent check](../output/savegame-semantic-map/epoch-byte-check.md)
corroborates its exact allocation and live registry entry. The sound-filter
role is source-verified; bit labels, writers and live restoration remain
unknown.

A relocated Fix `0:5` pointer chain now resolves three 40-byte gauge roots
and their 17 linked 24-byte child items. The exact root counts (7/5/5),
allocation boundaries, and one-byte registration shapes explain 34 child
visual-phase/progress-tick saves and three root completed-child counters.
Those **37 fields have source-verified meanings**: ticks advance while a
child remains in phase zero, threshold comparison advances its visual phase,
and the root counts completed children. The first root's separate saved
reference `0:5:4985156` is its next-root link, whose exact target and
negative target guard are checked by the source join, making **38 complete
gauge meanings** including that link. The three root threshold-selector
bytes are owned and typed, with a known index into four threshold bytes, but
their authored selector purpose is unresolved. The join excludes a nearby
unlinked 24-byte allocation and invents no panel or item names. See the
[pointer-chain evidence](../output/savegame-semantic-map/gauge-pointer-chain-followup.md).

The first exact eight-byte Fix `0:5` allocation, saved as `0:5:4`, holds
Hype's game-clock value in milliseconds. The source registers the eight
bytes, samples the running timer immediately before a save, and advances the
reset timer to the saved value after player-mode load when greater. The
supplied 249,589-millisecond low word and zero high word match the third and
fourth `General000.sav` text lines exactly. Those metadata lines are outside
the 1,271 SAI record identities. Backend zero uses `timeGetTime()` at 1,000
ticks/second; precise pause/menu inclusion remains open. See the
[clock source trace](../output/savegame-semantic-map/save-timer-followup.md).

Orrery decodes kind `0x11` as a block/module/offset reference. The catalogue
now records each target without conflating it with a semantic owner: 228
distinct source identities produce 362 file occurrences, pointing to 120
distinct target identities. Of the 362 occurrences, 134 target an identity
that also has a saved record. The rest include unsaved targets, dynamic graph
locations, and 16 `255:255:0` encoded targets. The 53 sector links above are
resolved. A [focused Level SuperObject check](../output/savegame-semantic-map/inventory-superobject-links-followup.md)
reconciled the 38 `1:6` references at 64-byte allocation `+4`: 16 point to
kind `0x12` instantiated-item SuperObjects, seven to bounded `0:10`
allocations, and 15 are sentinels. That proves target structure but not the
source SuperObjects' parent paths or a full gameplay role, so none of the 38
is promoted to a complete meaning. The remaining pointer graph still needs
investigation.

Arrays need content-level work: 222 distinct length-prefixed array records hold 350 payload bytes in this fixture. Only the one-byte dummy Boolean and lever-phase payloads have a source-backed meaning. The saved two-byte magic record `0:5:4986216` has a source-verified **low byte**: selected magic slot/index 0–2, used by the magic gauge and changed by selection controls. Its high byte and nonzero fresh-reload behavior remain unknown, so the complete record stays in the unknown count. The preceding `0:5:4986210` word is now source-verified as the `DefaultMagicPoints` starting amount capped at maximum points by the pinned executable's [MagicLevel callback](../output/savegame-semantic-map/magick-config-followup.md). The fixture stores 20; restoration order relative to the callback is still untested. Pointer targets, other designer values, inventory fields beyond item capacity, player statistics beyond mana, progression and the 261 dynamic-block records remain substantial families.

The [retained-save comparison](../output/savegame-semantic-map/corpus-variation.md)
decoded all 20 complete game-written roots. Every player file has exactly the
same 1,271 identities as the supplied fixture, with zero added or missing
identities; 20 values vary. Two independent no-action controls differ in
exactly three records (`0:5:4`, `2:6:1463692`, `2:6:1464196`), so those
values cannot be assigned to a tested action from a single before/after
comparison. The first is now the game clock. The other two are
[source-backed 36-byte position snapshots](../output/savegame-semantic-map/position-snapshot-followup.md)
used after load. The [owner follow-up](../output/savegame-semantic-map/position-owner-followup.md)
joins the first to Hype's main-actor position and identifies `StdCamer` as
the strongest source-order candidate for the second. Each save contains only
the first 36 bytes of an 88-byte POS matrix; translation is readable, while
the remaining value and the second owner's collision predicate are unresolved.
The adjacent [ToolCam word trace](../output/savegame-semantic-map/post-load-word-followup.md)
joins `0:5:4986280` to a two-byte snapshot of the low half of a nested
`+0x98` state-like dword, read before save and restored after load; its
gameplay value meaning is still unknown.
Lever-labelled runs additionally vary in two known DSG fields;
magic-labelled runs vary in selected inventory, mana and eleven still-unknown
`1:6` reference fields. The labels and correlations alone do not establish
causation. `HYPE/` and `SaveGame0/` members mirror byte-for-byte in these
captures, while `General000.sav` has a changing third line.

The [focused `1:6` comparison](../output/savegame-semantic-map/level6-reference-investigation.md)
places those 11 source references in bounded 64-byte Fix allocations. Each
record has two targets, separated by exactly 504 bytes between the 14-root
and six-root groups. The [exact target follow-up](../output/savegame-semantic-map/item-reference-followup.md)
shows both sets are type-`0x12` instantiated-item SuperObjects whose `+4`
data pointers reach typed Fix inventory handles. The supplied targets all
resolve to `NotFound 50`; one magic-labelled retained save resolves the same
eleven slots to named Ice, Fire and Lightning items. Five more fixed
references target Weapon, Object, Fire and restorative items. All 16 are
partial meanings because the saving source field, slot role and trigger are
unproven. These 22 target allocations are absent from the imported
`fix:/transforms/6-1` pool, despite sharing its block/module `6:1`; the
common 504-byte shift is not transform evidence or proof of magic causation.

### Current checkpoint and effort estimate

The largest unknown groups are 194 Global `DSG` fields among the 200 still-unknown Fix `0:17` records, 134 `0:21` dialogue references whose target content is unknown, and 123 `1:5` four-byte StandardGame records whose bit meanings are mostly unknown; additional `0:5` scalar/pointer and `1:10` map-state families remain. Ownership is resolved for 1,087 identities, and **508 complete record meanings** are currently supported (5 runtime-corroborated and 503 source-verified). Inventory, hit-point, gauge, dialogue, sector-link and NeverBack actor rules produced most of these meanings; extrapolating that rate to the remaining 763 would be misleading. The earlier 8–16 hour range may suffice for a useful fixture map, but a strong semantic explanation of the whole supplied fixture now looks more like several focused days, with broader progression-state/level coverage taking longer. This is a working estimate, not a stopping condition.

## Next discriminating work

1. Continue with other high-use Global saved fields. The [DSG[30] trace](../output/savegame-semantic-map/global-dsg30-followup.md) now source-verifies its checkpoint sighting-vector role, and [DSG[145]](../output/savegame-semantic-map/global-dsg145-followup.md) selects two sets of manor time-travel placement vectors. The [DSG[0] channel trace](../output/savegame-semantic-map/global-dsg0-followup.md) proved conditional channel-72 activation but found no authored target or gameplay purpose, so it remains unknown. The [all-saved-field ranking](../output/savegame-semantic-map/global-saved-field-ranking.md) intersects 198 saved Global fields with 335 explicit external reads and 416 writes; 173 fields have both reader and writer sites, but those sites are evidence leads rather than complete meanings. The [inverse AI index](../output/savegame-semantic-map/global-inverse-ai-followup.md) scanned 94 linked Persos and 70 AI models. `DSG[24]` still has only the `iDummy_SaveTesteurs` snapshot/restore use outside Global's own initialization to 10. Determine its actual consumer or treat it as unknown; do not mistake the test copy for a gameplay role. The [Global own-model inventory](../output/savegame-semantic-map/global-ast-reader-sites.md), [two-field follow-up](../output/savegame-semantic-map/global-two-fields-followup.md), and [pinned-executable callback check](../output/savegame-semantic-map/global-callback-pointer-check.md) retain remaining uncertainty, including a Raymap callback-name conflict and dynamic/unsupported AST targets.
2. Resolve the authored content reached by the 134 dialogue references and the individual meanings of 60 `1:10` map-star entries. The [dialogue](../output/savegame-semantic-map/dialogue-target-investigation.md) and [star](../output/savegame-semantic-map/map-star-value-investigation.md) source surveys establish structural targets and values but no individual authored names or gameplay purpose. The supplied [manual](../disc/HypeManual.pdf) confirms `<End>` as the default map key and describes the map as something a character owns; the playable manor fixture's map ownership has not been established. Before another map live probe, preflight the exact private-display input path, map availability, and a visible map-open condition; the [first attempt](../output/savegame-semantic-map/map-state-live-probe.md) expired and the [retry](../output/savegame-semantic-map/map-state-live-probe-retry-01.md) did not visibly open the map. Do not interpret unchanged words until opening is confirmed in gameplay.
3. Check fresh-process restoration order for the now source-verified `0:5:4986210` default starting magic-points word, and resolve the high byte of `0:5:4986216`. The [selection-word follow-up](../output/savegame-semantic-map/magic-selection-word-followup.md) confirms the latter's low byte as the selected magic slot and its high byte as still unresolved, while [native callback disassembly](../output/savegame-semantic-map/magick-config-followup.md) resolved the former's configured starting amount. Neither proves the later load/apply result.
4. Extend the verified [player and named-level selection paths](../output/savegame-semantic-map/named-level-mode5-probe-20260928/final-report.json) to other representative record kinds and restoration timing. First-slot startup applied the player value for `1:5:833347`; tele11→tele05 sampled state 5 and, after a documented isolated-file intervention restored the opposed `HYPE/manoir.sav` value, applied that named value on return in both cases. The outgoing transition normally writes current manor state to `HYPE/manoir.sav` first, so the unmodified conflicting-file run is inconclusive for precedence. Source calls and masks are in the [transition-order trace](../output/savegame-semantic-map/mode5-transition-save-order-followup.md). Do not generalize one byte's stable post-return result to every array/reference kind or later initialization path.
5. Use the 37-level descriptor discovery to choose saves or progression states that expose new registrations, and track unvisited states separately from unknown records already seen.

The last complete affected save selection passed **49/49** before the new mode-5 test was registered; [the report](../TestResults/20260928T101336.923585Z-development.log.json) records unchanged source, 0.318 seconds build/discovery, 493.559 seconds test execution, 0.037 seconds temporary cleanup and 500.521 seconds total. It includes all six earlier original-game save scenarios and native Godot persistence, with no leaked child processes. The new named-level case passed its focused run in 101.826 seconds with 28 mandatory safeguard checks; the 50-case full selection is pending. The subsequent partial Global annotations passed Python compilation and catalogue reconciliation; they do not represent additional live scenarios. Formatting, Clippy and the locked offline release build passed on the prior Rust tree. The live registry was separately verified twice in playable `manoir`, with 2,180 nodes and the same 1,916 mapped source identities. The expanded save selection should use an explicit **960-second** job budget to fit its new 420-second case deadline plus cleanup reserve; the exact approved private-runtime command is in the [handoff](savegame-semantic-map-handoff-20260928.md).
