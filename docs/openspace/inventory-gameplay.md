# Hype inventory gameplay contracts

The item catalogue and admitted category slots are different source-owned
values. `InventoryCatalog` retains all 62 ordered main-list items, their useful
IDs/kinds, signed quantities and capacities, selected flags, availability and
management values, category slots, and the separate special-resource item.
Item source identity and catalogue order are distinct from the info ID: the
twelve separate `NotFound` placeholders all have info ID 50 and quantity -1.
Their identities must remain separate for replacement and admission. Full
catalogue ID queries find the first matching member in source order.
Source references use the existing `SourceObjectIdentity`; source addresses stay
inside the OpenSpace adapter. Presentation artwork and localized labels remain
the independently owned inventory-presentation asset.

`InventoryDefinition.kind` preserves an optional source kind string. Ordinary
catalogue items require a present, nonempty kind; the special gold resource's
definition has a null kind at `+12`. Its source operations use the special root
and never dispatch on that kind (`0041c75b`, `0041d400`). Godot projects that
absence to an empty native String on the special definition only. It does not
invent a `Gold` kind or relax ordinary-item validation.

Read-only traversal of the supplied Fix data confirms all 62 ordinary kind
strings resolve through `Fix.rtb` into populated `Fix.sna` block `10:0`.
The gameplay catalogue therefore needs no language hydration; localized
presentation labels and fonts retain their separate language dependencies.
A nonzero kind field without a relocation remains an error, and importer
diagnostics identify the ordinary item index/address or special resource.

Raymap's `AITypes_Hype.cs` names functions 196–210. The implementation below is
corroborated by static x86 disassembly of
`reference/hype_patch/MaiDFXvr_bleu.exe`, SHA-256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
This executable differs from the supplied English fixture's recorded runtime
executable. Cross-build equivalence and native gameplay acceptance remain
unverified. This source investigation used read-only disassembly and data
inspection; build and preparation results are reported separately.

| Source routine | Corroborated meaning |
| --- | --- |
| `00539b60` jump table, entry 196 at `00539b80` | Evaluate one actor expression. Null actor returns zero. Otherwise follow Perso `+0x38` stock and call `0041c720`; return its signed 16-bit value. |
| `0045c350`, `0045c370`, `0045c390` | Stock `+8` is category SuperObject, `+12` item SuperObject, `+16` signed pickup count. |
| `004a1920`, `004a1960`, `004a19d0`, `004a3ba0` | Instantiated-item handle selected byte `+8`, definition pointer `+0`, signed quantity `+10`. The durable save join identifies bounded 24-byte handles. |
| `004a3960`, `004a3a46`–`004a3a54` | Allocate a 24-byte handle; `+12/+16/+20` are one first/last/count replacement-list header, initially zero. `+20` is a scalar count, not a pointer. |
| `004a3a60`, `004a3bd0`, `004a3c70`, `004a3c90` | Allocate a 20-byte replacement, append/unlink it in the handle's list, and read its from/to item SuperObjects. |
| `004a19f0`, `004a3890`, `004a3940`, `004a3950` | Definition capacity `+20`, availability byte `+25`, signed info ID `+28`, kind string `+12`. |
| `0041c610` | Search admitted category slots by item info ID; skip artwork/header children and descend one group level when the second category child is a box. |
| `0041d570` / function 202 | Select an admitted item whose exact kind is `Weapon`, clearing other selected `Weapon` handles. Success is one; missing item or wrong kind is zero. |
| `0041dab0` / function 210 | Read capacity from the full catalogue; ID minus one selects the separate special-resource definition. Missing item returns minus one. |
| `0041d400`, `0041d420`, `0041d490` / functions 198–200 | Read signed special-resource balance; add with signed-16 wrap before capacity comparison (clamped result returns zero); subtract compares signed balance to full signed-32 request, writes a wrapped signed-16 result on success. Negative requests retain their source arithmetic. |
| `0041d4d0`, `0041d890` / functions 201/205 | Return the first selected exact-kind Weapon/Object from the catalogue, independent of admitted slots; missing selection returns minus one. |
| `0041dc80` / function 206 | Select an admitted exact-kind Object, clearing other selected Objects; return one or zero. |
| `0041dbb0`, `0041dc10` / functions 208/209 | Query catalogue quantity; removal computes signed-32 quantity minus request and writes its low signed-16 word only when the signed-32 remainder is nonnegative. Missing item returns the signed-32 negated request. |
| `0041bec0` | Pointer declaration order independently anchors main catalogue, category tree, display row, special resource in Fix.gpt. |

`0041c720` is an admission operation, not a generic increment. Its special
resource branch adds the signed pickup count with signed-16 wrap, clamps to the
linked capacity, and returns minus one. Availability exactly one inserts the
item into the first free category slot and returns its info ID without changing
quantity or selection (`0041c7ea`–`0041c84c`); this branch returns zero when full.
Other ordinary items search occupied slots for replacement or the same item,
or insert into the first empty slot. The empty-slot branch at `0041ca11` rejects
a zero count only when availability is nonzero and quantity is not minus one.
Insertion into slot zero selects an incoming Weapon or Magic kind. Quantity
minus one is a sentinel left unchanged; other quantities add the signed count
with signed-16 wrap before clamping to capacity. A quantity already at capacity
returns zero. If an ordinary category has no matching or empty slot, its
quantity update still runs without admitting the item.

The replacement-list element stores from/to item SuperObjects at `+0/+4`,
next/previous elements at `+8/+12`, and the owning handle at `+16`. The importer
checks exact 24-byte handles and 20-byte elements, bounded counts, cycles,
previous/owner links and the final tail. `InventoryReplacement` retains each
element's source identity and ordered nullable from/to item identities after
the complete catalogue is known. Source list links, addresses and the derived
count are not portable gameplay data. The supplied Fix has **21 replacements
on 15 ordinary items**: twelve Magic items upgrade distinct ID-50 placeholders,
and three Object items each have three combination rules. All observed
endpoints are ordinary catalogue members; the special resource has no rules.
The category hierarchy yields seven admission groups after retaining the three
nested Magic groups separately from their parent category.

`0041c869`–`0041ca10` searches admitted slots in order and each incoming item's
replacement rules in order. Rules with null endpoints do not match. A matched
from item is replaced by the rule's to item, with its selected flag transferred.
If the result is an exact Weapon or a Magic kind and no item of that exact kind
is selected in the full catalogue, the result is selected. The operation
returns the resulting item's info ID without applying the pickup quantity.
Replacement precedes the same-item comparison for each occupied slot.
Native replacement support uses these ordered identity relationships and
per-member quantity/selection/capacity state, rather than merging placeholders
by their repeated info IDs. Native runtime behavior remains untested.

The native component supports these paths using ordinary Resources and mutable
category arrays. Signed source counts and quantities, including negative values
and addition wrap, retain their source arithmetic. The source inventory-full
Boolean and gameplay effects beyond the represented admission, selection and
quantity APIs remain outside this slice. Item availability is not a generic
Boolean possession flag.

Legacy manor fixture/native schema-1/2 saves retain sparse quantities and one
weapon selection. When typed imported Persos supply unambiguous stock-category
membership, native migration can assign owned nonzero/sentinel items to those
category slots in catalogue order. This is an authored native migration;
original saved slot order is not claimed. Missing membership retains the old
saved-start quantities/weapon, marks admission unknown, and reports that gap
when pickup or selection is requested. Catalogue capacity reads remain usable.
Sparse migration updates saved quantities while other members retain their
imported seed quantities; it does not guess an absent member's handle sentinel
or balance from an ID count. A sparse save naming a repeated info ID is rejected
because it cannot identify which distinct member it owned.
The importer must not discover stocks by guessing from unrelated allocation
sizes. Reach stocks through the typed Perso field.

`presentation_quantities()` supplies the inventory UI's ownership view. With
known admission it projects category slots, using the first admitted member
for each info ID. With unknown legacy admission it projects only the saved IDs
retained in `legacy_owned_ids`; the UI does not expose the rest of the imported
catalogue merely because its quantity dictionary contains those IDs or minus-one
sentinels. A source-free authored inventory retains its sparse quantity view.
The full catalogue's `quantities` dictionary instead projects the first member
for each info ID for source queries. These views do not merge mutable member
state or resolve unknown admission for pickup/selection.

The generated shared inventory is cloned once per scene session. Definition
Resources are copied so mutable capacity values are isolated; quantities,
category arrays, gold and selections are also isolated mutable state shared by
that session's actors. The maintained
`inventory.gd` reports unsupported source operations through `last_failure`,
which compiled AI propagates through its ordinary unsupported-behavior path.
Failed unsupported pickups reject before native inventory mutation.

Inventory Resource snapshot version 2 retains ordered admitted slots,
per-member quantities, selected flags and capacities, the first-ID quantity
view, gold, derived Weapon/Object/Magic selections, and the admission-known
flag with its diagnostic and `legacy_owned_ids` fallback. Per-member selected
flags retain selections for distinct exact Magic kinds; the scalar
`selected_magic` exposes only the first selected Magic member in catalogue
order. Replacement rules remain generated definition relationships rather
than mutable snapshot fields. Ordered catalogue source identities and category
sizes bind the snapshot to its native graph. Resource shape/binding validation
and snapshot application support isolated clones.

The current production schema-1 game save restores sparse quantities and weapon
selection through `restore_legacy`; it does not compose full inventory and armor
snapshots. The fuller schema-3 checkpoint from the earlier fork was superseded
by integration. See the [current native save contract](../astrolabe/godot-project.md#native-save-contract)
for the active owner and omitted state. Broader unimplemented effects remain
unsupported rather than invented by persistence.
