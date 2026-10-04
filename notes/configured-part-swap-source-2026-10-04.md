# Configured part swaps, 2026-10-04

Static source and code review only; no tests, builds, converter execution or
gameplay. Engine identity and ignored disassembly provenance are documented
in [the armor source note](armor-source-2026-10-04.md).

Raymap and the corroborated API inventory call procedure82
`Proc_SwapLinkTableObjects`; it is not whole-configuration selection.
`00541130` evaluates its first integer operand, selects82 at`00541154`, then
evaluates its second operand at`00541186`. At`00541183` and`00541192` it
subtracts one from each. `00461860` checks the actor's current configuration
through`00442530`, narrows both indices to unsigned shorts, and swaps two
complete20-byte entries in that configuration's parts array (+12). It copies
five words in each direction, including content, scale, kind and unresolved
values. The configuration header is shared source data; actors referencing the
same canonical ObjectList must observe the same permutation.

Procedure83 follows the other branch in this dispatcher and calls`00442540`
to change the selected ObjectList reference. Its implementation and the corrected
current/initial distinction are recorded below. `00461a60` snapshots an ObjectList into its
private reset array; `00461ac0` copies the entire snapshot back into the current
parts array. Restoration therefore resets permutation as shared gameplay state.

The native implementation uses one editable configuration Resource per canonical
ObjectList. Its `payload_order` records the permutation of already converted
native child identities; `restored` selects the separately exported reset
payloads and native animation library. This state contains no mesh-building
instructions, source wire records or sampled-animation tables. A level owns one
mutable copy shared by matching actors; spawned actors adopt the current copy
from the level's `source_configurations` metadata map.

Conversion resolves every part into saved native slot anchors and payload nodes
for each animation channel, including currently unused/absent slots. Model
payloads contain instanced native model scenes, scale lives on the payload, and
action payloads retain ordinary Resource references. Native animation tracks
target stable anchors for visibility and opacity. A swap reparents existing
payload children between anchors and reapplies the destination opacity; no
runtime assets are reconstructed. Arbitrary repeated swaps and same-slot swaps
operate on current payload order. Restore resets that order, selects reset
payloads for all actors sharing the Resource, and seeks the equivalent native
animation position without firing cues.

`Character.source_swap_configured_parts(first,second)` accepts original one-based
integers and returns false on missing native configuration or invalid narrowed
indices. Compiled AI treats false as an explicit unsupported failure. The original
callback returns the script cursor; it has no Boolean gameplay return.

Animation event actions are independently shared descriptors, not interchangeable
with configured part action references. `0043b910` indexes the animation's
eight-byte cue list, then`0043bcd0` obtains the direct descriptor from cue+0 and
dispatches descriptor+4. These direct native cue references remain intact across
part swaps. The retained cue configuration slot is not used to invent a different
action descriptor. Source channel-control filtering is a separate incomplete
animation concern and has not been broadened by this slice.

The independent canonical limits are 65,535 ObjectList entries, up to255 frame
columns, 255 non-sentinel logical channel IDs, and at most two payload banks
(current and reset). Across all family clips, each logical ID can have255
occurrences, so the theoretical channel union is65,025. Their cross-product
would permit8,522,826,750 payloads and at least17,045,653,500 anchor/payload nodes,
before native model descendants. The exporter explicitly rejects a rig needing
more than65,536 payload budget units before constructing that graph. This is a documented
Godot capability budget, not a restriction on the canonical/source model.
With the parking banks added for Proc83, the upper structural node count at that
budget is196,608 plus channels, native
model descendants and existing character components. Family state count can
still reach65,536; actual animation/output memory needs measurement later.

Unused anchors remain hidden through saved properties and receive no animation
tracks. Anchors whose slots appear in any family sample get native visibility
and Godot `transparency` tracks; zero transparency is opaque, while source
opacity255 is opaque. This avoids two constant tracks per unused slot per state.
ObjectList validation already requires reset/current arrays to have equal
length; source import and Godot export both invoke it before preparing banks.

Swapping outside an animation tick changes only Resource order and existing child
parents/properties. It neither advances/seeks AnimationPlayer nor changes stable
track paths, so it does not fire cues, change playback position or accumulate
root motion. Restore is the distinct operation that switches native libraries;
it uses cue-suppressed update-only seeking at the existing position.

This conversion increases saved node counts because unused payloads must exist
before arbitrary runtime permutations. It has not been measured under the
user's static-only constraint. The API implements the current converted
Model/Absent/Action projection; unresolved ObjectPart bytes remain explicitly
preserved in the canonical model, with their broader rendering meaning still
outside existing coverage. No speculative visibility repair or Hype/statue
relocation was added.


## Whole configuration selection and initial-table correction

Proc83 `Proc_ChangeCurrentObjectTable` uses the same dispatcher `00541130`.
The first operand is evaluated by `00522610`; the branch at `0054115c` passes
its direct pointer value to `00442540`. That setter assigns only Data3d+0x10.
The Hype ObjectTableRef category is35, corroborated by Raymap's Hype inventory
and the matching engine category registry at `005fd114`/evaluator `00521f00`.
There is no integer ordinal conversion, table copy/reset, action transition or
animation restart in the source handler.

This exposed a source interpretation error. Raymap's `Perso3dData.Read` calls
Data3d+12 current and +16 initial, and the Rust importer followed it. Matching
engine getter `00442530` reads +16 for Proc82; setter `00442540` writes +16 for
Proc83. Reset consumer `004421f2` instead calls `00442550`, which reads +12,
checks its saved table through `00461b90`, and restores it with `00461ac0`.
The SuperObject+4 → Perso+0 → Data3d chain is the same source chain used by the
importer; family getter `00442570` reads +20, matching its family field. The
wire declaration and importer now correctly label +12 initial and +16 current.
The current source exporter gathers both dependencies but does not yet emit
Perso/Data3d records, so no record writer needed an offset correction. Existing
synthetic Perso constructors provide semantic fields, not those raw offsets.

The saved native rig now contains stable slot anchors over its reachable
configurations' slot/bank union. Each configuration has already converted native
payloads in per-channel hidden parking banks; the initial current bank is saved
under the stable anchors, so initial visuals exist before `_ready`. Selecting
another configuration parks existing children and reparents the selected bank's
payloads according to that shared Resource's current permutation. All model
instances, scales and action Resources were resolved during conversion.

`source_configuration_choices: Array[Resource]` contains exactly the unique
current and initial ObjectLists plus literals from the actor's AI model. The
supported behavior-change bridge cannot make this actor execute a different
AI model; cross-model requests fail explicitly. Thus the caller-model literal
closure is sufficient for the admitted compiler. The entire family pool and
implicit family default are deliberately not prepared. Unknown dynamic object
reference forms remain unsupported, and an authored request outside these saved
choices returns false rather than constructing a missing bank.

The native API `source_change_configuration(Resource or null) -> bool` changes
only actor-local selection. Null parks all payloads and leaves playback intact.
Normal/reset banks share stable native tracks; changing bank switches equivalent
libraries at the same position through cue-suppressed update-only seeking. A
same-bank selection or swap does not seek/advance playback. Direct cue/action
references, mechanics, root motion, current/initial state and repeat controls
remain intact. Source event channel-control filtering remains an existing gap.

Each active level owns one configuration Resource per canonical identity,
including unselected choices. Rooted actors receive the same map; spawned actors
adopt its current state. `source_initial_configuration` retains reset ownership
independently of `source_configuration`. `source_restore_initial_configuration`
restores the initial shared table without selecting it. The existing authored
`restore_configuration` inspection hook continues to restore the current table;
it is not represented as a complete source actor lifecycle reset.

The revised budget is the sum, over these reachable configurations, of
`max(1, slots) * max(1, channels) * (1 + has_reset_bank)`, capped at65,536. Charging
empty slots/channels also bounds empty banks and Resource count. Payloads,
stable anchors and parking banks each fit within that budget, giving the
196,608 structural-node upper bound above; actual native model descendants and
animation sizes remain unmeasured. No tests, conversion or runtime validation
were performed for this extension.
