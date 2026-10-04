# AI static port slice — 2026-10-04

This record covers source review and implementation only. The user deferred
tests, builds, converter runs, gameplay and experimental probes. Formatting
parsed the edited Rust source; no Rust type checking, generated GDScript parsing,
runtime behavior or new corpus coverage is claimed.

## References and callable identities

The recovered `reference/hype_patch/MaiDFXvr_bleu.exe` matches the recorded SHA256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
The virtual addresses below refer to static disassembly of that executable.
The pinned Raymap tree is restored and unchanged. Its
`Assets/Scripts/OpenSpace/AI/AITypes/AITypes_Hype.cs` and `TranslatedScript.cs`
corroborate keyword and tree forms, but its late callable indices differ from
this build. The build inventory in `notes/research/hype-ai-api.tsv` has 128
conditions, 242 functions, 446 procedures and 69 meta-actions; the older Raymap
tables have 126, 240 and 443 in the first three groups. Do not transplant those
late indices. A standalone decompilation source tree is unavailable.

The existing `TestResults/review-import.json` contains 2,075 unsupported-script
warnings across 54 models. It guided priorities only; it predates these edits
and was not regenerated. Repetition counts do not establish semantic support.

## Implemented dependency slice

`src/model/ai/walk.rs` provides one exhaustive, iterative preorder AST walk.
Model collectors, unsupported-fragment inspection and structural validation now
share it. The superseded traversals omitted children of `IntegerCast`,
`SetLaserPointerSniperMode`, `InventoryQuantity` and `VectorComponentAssign`
in different consumers. Nested content and unsupported expressions now reach
the same collectors. The explicit stack allows validation to reject excessive
depth before recursive compilation/encoding. The walker has no wildcard child
matches, so new variants require an explicit traversal decision.

`src/model/ai.rs`, `src/openspace/ai.rs` and `src/godot/ai.rs` contain the typed
model, source encoding and native compilation for the following additions:

| API/tree | Source evidence | Native meaning and scope |
| --- | --- | --- |
| Keyword Me/MainActor and bound actor-reference expressions | Raymap keyword 4/5 and category 24 trees | Ordinary actor node values; Me uses the Brain parent, references use the active scene resolver, including nonvisual Global holders. Unbound source references remain unsupported. |
| Cond64 IsValidObject | `005344a0` dispatch to `0053451d` tests a nonzero pointer word | Native object validity, without inventing an actor-active condition. |
| Func4 DistanceToPerso | Source distance helper `0052ea00`; full norm is unchanged by orthogonal frame conversion | Full native world distance to a bound Node3D; require positioned caller and target. Original MainActor tree form remains encodable. |
| Func25 Square | `00538350`: evaluate once, integer/real conversion, x87 multiplication, tag 8 and `fstps` | Capture input once and round the output to f32. |
| Func33 AbsoluteValue | `00538650`: integer/real conversion, compare with zero, negate nonpositive input, tag 8 and `fstps` | Capture once, conditional negation and f32 output retain the signed-zero branch instead of substituting a generic absolute-value helper. |
| Ultra display/opacity | Known receiver tree plus procedures 185/186/187/266/267 | Call the receiver's existing native transparent/default, opacity and visibility methods, with explicit receiver/support guards. This does not establish a complete Hype/statue visual fix. |
| Dot(Me, designer field) assignment | Preserved category 2 Dot tree and keyword 4 | Compile to the same native field property as local assignment while retaining its distinct source tree for reconstruction. |
| Proc207/208 external ChangeComport | `00542650`/`00542720`; normal helper `005426f0` preserves equal selection | Resolve the target Brain and exact AI model, then invoke native normal/reflex transition. A self transition respects caller cancellation; an external transition does not cancel the caller. No source scheduler is recreated. |
| Meta38 WaitBoolean | Init `0054db40`, update `0054dbd0`, indexed field reader `00523a50` | Capture expected integer once per action entry; reread the named designer field on native ticks. Restricted to identified local/self/main/bound-actor field forms; existing cancellation and normal/reflex callback boundaries remain. |
| Proc429/430/431 and Func226/227 armor | Shared armor source investigation recorded in `armor-source-2026-10-04.md` | Resolve shared native ArmorCatalog, select definition, set resistance, repair, query resistance/count. Selected-definition operations require a selection. Works for Global callers through the scene's shared service. |
| Func224 armored damage | Dispatcher `00536452`, damage `0043aa20`/`0043aa70`, HP subtraction `00460b10` | Evaluate damage once, guard catalog and main actor HP before mutation, apply catalog-owned native RNG armor damage, subtract effective damage with floor zero, return remaining byte HP. Reject invalid authored state before HP subtraction. |
| Proc82 configuration parts | `00541130`/`00541178` to `00461860` | Capture both integer indices in source order, call native shared configuration swap, and fail explicitly if the prepared slots cannot represent the request. |
| Func196 actor stock admission | `00539b60` dispatch, helper `0041c720` | Capture one actor; null returns zero. Pass its verified native stock binding to the shared inventory service, preserve signed return value, and exit on the service's explicit unsupported diagnostic. |
| Func202/210 inventory selection/capacity | Weapon selection `0041d570` with admitted-slot search `0041c610`; capacity `0041dab0` | Capture item once; shared native catalog preserves admitted-only weapon selection, catalog-wide capacity and special item -1. Native service diagnostics remain explicit. |
| Func198/199/200 gold | `0041d400`, `0041d420`, `0041d490`; word getter/setter `004a19d0/e0` | Shared signed16 gold read, wrapped add before signed capacity clamp, and insufficient-funds subtraction. Integer input is evaluated once; preserve integer/byte results. |
| Func205/206 selected Object | `0041d890` and `0041dc80` | Full-catalog selected exact-Object getter returns signed item ID/-1; selection requires an admitted exact-Object entry and returns 1/0. |
| Func201/208/209 queries/removal | `0041d4d0`, `0041dbb0`, `0041dc10` | Initialized shared service replaces Character-only wrappers; preserve full-catalog lookup, signed16 quantity and signed32 subtraction/return with narrowed word write. Admission uncertainty does not block known state operations. |
| Proc83 current configuration | `00541130..00541167`, setter `00442540`, category35 evaluator `00521f00` | Resolved literal ObjectTableRef/nullable Resource only. Replace selection without source reset/transition operations. Import typed dependencies before character import finishes and export each actor's reachable native banks. |

Armor selection/rank inputs narrow to a byte; invalid selection becomes null,
rank values at least five reset to zero. Repair retains the current piece while
resetting broken/strike state. The native catalog is shared by actor callers,
including its RNG state; the compiler does not use each Brain's ordinary random
integer generator for armor. Parent-owned native damage and HP methods provide
the implementation contract; this slice has not executed them.

Generic references depend on the active scene's `source_actor_nodes` including
rooted and Global actors. Shared armor depends on its `source_armor` metadata.
The parent integrated those bindings and bumped support/Rete versions. Actor
render calls still require visual-capable receivers; a data holder does not gain
invented rendering behavior.

## Explicit deferrals

**Partial distances, functions 7/8:** `00537a20`/`00537aa0` call `0052ea00` with
masks 4/3. The helper rotates the difference into the target actor's frame before
selecting components. Simply mapping source Z to native Y or source XY to native
XZ would be wrong for tilted actors. They remain unsupported pending typed
source-frame semantics.

**Proc237 SetDiffuseCoef:** handler `00543230`, case `00543386`, resolves its
category-36 game material through `0049df30`. If game material/visual is absent,
it skips the remaining operands through `00522450` without evaluating them.
Otherwise it evaluates one packed RGBA value. `0052efc0` clamps each little-endian
byte to 0..100 and multiplies by 0.01, not 1/255; `0048f1b0` stores all four
coefficients at visual material +0x14. The current exporter bakes diffuse into
`clamp(2*(ambient + diffuse*RLI),0,1)` prelit vertex lighting, and sprites/lighting
variants duplicate native materials. An albedo mutation would not preserve this
meaning. No AST/codec/compiler support was admitted: a future slice needs shared
material identity and dynamic native lighting/shader state for every variant,
while preserving opacity and skipped-operand evaluation.

**Proc82 SwapLinkTableObjects dependency:** gameplay review identifies `00541130`/`00541178`
to `00461860` as two integer operands, subtract one and narrow to u16, swapping
whole 20-byte entries in the actor's selected ObjectList parts. Proc83 separately
selects a whole configuration. Current native animation tracks bake channel
slots/action cues. Gameplay has added persistent shared native configuration
state and prepared complete payload variants for each slot/channel. The typed
compiler call is admitted against that focused native dependency; this pass
does not validate its saved animation/render behavior. Proc83 was subsequently
admitted for resolved literal references only, with the native bank dependency.

**Func196 AddObject bounded dependency:** workflow review identifies dispatcher `00539b60` and
`0041c720` as one actor operand accessing its StockAction, returning a signed
16-bit result (null actor returns zero). It is not an item-count increment.
Typed stock/catalogue bindings and admitted-slot semantics are required.
Function202 weapon selection also searches admitted slots; function210 maximum
capacity scans the catalog. The compiler now delegates to the native shared
inventory graph. Unsupported replacement chains, category cases and count
sentinels remain explicit service failures; this is not universal inventory
coverage. Stock-bearing Global actors require real stock-property bindings.

**Func225 and Proc432 damage:** their additional callback/life/RNG contracts are
not accepted by the function224 bridge. In particular, function225 has the
separate damage-target HUD callback `0040c4a0`; it remains deferred.

## Static integration audit

The AST additions are handled by the model walker, OpenSpace parser/writer and
Godot compiler. A source sweep of `src`, `crates`, `tools` and `tests` found no
other exhaustive Statement/Expression match requiring new arms. Existing tests
constructing the original MainActorDistance and field forms remain source
compatible. No tests were run. One existing sound-fade source assertion was
mechanically updated to check consecutive 127/1 operand captures and their
ordered call arguments after the compiler refactor. Historical assertions about specific
unsupported fragments and generated call text need deliberate review when
execution is authorized; they are not fresh acceptance evidence.

Native dynamic lookups use explicit Node/Node3D/Resource types where subsequent
Godot inference would otherwise depend on Variant method calls. Formatter and
whitespace review passed for the owned Rust files; this is not compilation.

ArmorCatalog's explicit `clone_for_scene()` clones the catalog, each definition
and each piece while sharing immutable Texture2D artwork. The parent binds one
copy through level metadata and actor properties. Current upstream Godot
`Resource::_duplicate_recursive` and `duplicate_for_local_scene` traverse
Array/Dictionary properties using a remap cache, supporting nested local graph
duplication and retained sharing on scene instantiation. The former 4.4
documentation exclusion for Array-contained subresources does not establish
behavior for the pinned 4.7.2; exact-pin runtime verification is still deferred.
See [current Resource docs](https://docs.godotengine.org/en/latest/classes/class_resource.html#class-resource-method-duplicate)
and [native Resource source](https://github.com/godotengine/godot/blob/master/core/io/resource.cpp).

Final static integration review corrected a compiler evaluation-order defect:
an inline left-side call could be delayed until after the right child's emitted
guard/mutation prelude. Multi-operand guarded expressions now capture each child
as a native Variant before lowering the next child. The same ordering applies
to array indices, dialog coordinates, sound-fade operands and action/object
generation inputs. And/Or retain their existing conditional native evaluation.
Proc82 explicitly checks the Brain parent's native swap API instead of assuming
a Character receiver; a nonvisual unsupported caller fails without an assertion.

Workflow disassembly confirms functions202/210 compare the full evaluated DWORD
against sign-extended source item IDs: `0041c684..0041c68e` and
`0041dab1`/`0041db00..0041db06`. There is no additional low16 argument narrowing;
the compiler passes the evaluated integer unchanged. Proc82 separately subtracts
one then narrows each slot index to u16 at the native resource boundary.

Behavior dependency closure is explicit: OpenSpace builds the behavior-member
index from every known rooted/template/Global actor Brain model before script
decode; model collectors traverse nested calls; Godot loads and validates every
external BehaviorRef model. Scene export emits receiver Brains for saved rooted
and Global actors and binds a shared identity map. Runtime transitions require
the resolved receiver Brain's exact model path. References outside the known
source actor graph remain unsupported instead of creating invented actors.

The subsequent inventory source audit established word-level arithmetic rather
than ordinary saturating counters. Gold add stores the low16 sum first, then
compares its signed16 interpretation against signed16 capacity; clamp returns
zero, otherwise one. Gold subtraction compares current signed16 against the
full evaluated integer and returns zero unchanged if insufficient; negative
inputs can increase/wrap the word. Quantity removal returns signed32
`quantity-count`, writes low16 only for a nonnegative result, and returns
signed32 `-count` for an absent item. Compiled calls now use the shared service's
source methods and explicit initialization diagnostics. The historical sparse
native fixture retains its documented absent=-1 quantity view; it does not
establish a missing source-catalog quantity. Admission uncertainty gates
selection/pickup, not the verified gold/selection getters/quantity/removal.

Proc83's dependency scanner accepts typed category35 literals only, with bounds
on actor/configuration counts and source script extents, allocation-start and
ObjectList record checks. Original evaluator `00522610` indexes the registry at
`005fd088`; entry35 at `005fd114` points to `00521f00`, which copies the pointer
literal, sets value tag30 and advances eight bytes. Entry28 at `005fd0f8` is
different and remains SurfaceRef. The scene imports all these typed lists while
the existing character importer can still follow geometry/material/scale links.
The AI codec registers their canonical role/asset identities and assigned
source handles; null literals roundtrip without a binding, while nonnull handles
must be assigned. Dynamic configuration expressions remain explicit unsupported
fragments because their reachable choices are not established.

Native rig banks include current/initial configuration and the actor's own
unique AI literal references. Configuration expression constants use the same
`native-configuration.tres` resource returned by the character exporter.
External behavior transitions cannot replace the actor's AI model: both model
validation and the runtime exact-model check retain that boundary, so these
banks do not need another model's scripts or a whole-family expansion. New
compiler binding data is grouped into one adapter-owned structure; public
compiler signatures and existing default codec constructors remain compatible.

The first newly authorized prepare-game retry imported the scene and failed at
export with 102414 configured payloads for `fix:/families/1`; evidence is
`TestResults/prepare-game-recovery-G7IFeh/prepare-01.log`. Parent inspection of
the freshly imported Rete found configuration30 has 507 entries in both normal
and reset tables, each 221 Absent/181 Model/105 Action. The old bound multiplied
all 507 entries by all 101 channels and both tables, including object-free
channels and empty native nodes.

The exporter now retains all transform/cue channels but emits banks only for
channels that actually sample objects. Every nonempty original entry remains
prebuilt in each such channel, preserving arbitrary Proc82 swaps. Absent
entries become explicit `empty_payload_slots` bank metadata; native selection
checks absent and payload identities are disjoint, unique and cover the complete
configuration before routing. Missing nodes are accepted only for an explicitly
absent original slot. Stable animation anchors/tracks cover sampled slots
independently of the initial payload, so swapping content into an initially
empty sampled slot remains possible. Unsampled content stays in saved hidden
banks. The unchanged 65536 bound counts actual nonempty normal/reset payloads,
with an assertion that the emitted record count matches the checked budget.
Formatting and whitespace checks only were run by this worker; parent owns
build/retry verification. Identical normal/reset payload deduplication and
shared cross-channel pools remain possible subsequent optimizations, not part
of this focused correction.
