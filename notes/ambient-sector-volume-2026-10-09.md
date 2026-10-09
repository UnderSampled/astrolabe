# Manor ambient actor sector and effects volume, static implementation

Integration outcome: the original ambient and lifecycle commits are on main as
`2645698f72ab02ad96329ad79fa25e8becf4bf16` and
`7cf38563cb761ea45a8f7a0fca78c0ceed23b0e3`. The recovered final-child follow-on
`dd545dfcfc7ca51283b469aea570771782033b74` is integrated as
`2bf67a565d3771c5c2261779424d66b62440e967`. Rete62/support83 assertions and
fixture migrations are reconciled; the camera fixture uses the maintained
Character holder, final-child reference, actual second-sector geometry and
two-argument publication API. Its test was not run. The approved Manor-only
publication completed before the native-only follow-on; its existing semantic
final-child binding remains valid. See the [composition/publication handoff](manor-ambient-channel-composition-2026-10-09.md).
The isolated-worker restrictions and pending-integration statements below are
historical; startup mode/list mapping and runtime acceptance remain uncertain.

This isolated change starts at clean main `681b53ea74fbc44bec107be5fe5176e367a92753`, on `work/ambient-condition124` in `/tmp/astrolabe-ambient-condition124`. Kip owns integration, regeneration and any subsequent validation. Only existing source, code and data were read. No tests, builds, lints, formatter checks, probes, gameplay, original-game runs or exports were performed. Test sources, generated assets, reference submodules, main, saves and settings were not changed. No workers or pushes were launched.

## Observed failure and limits of the observation

The existing read-only [acceptance log](../output/integration-promotion-20261009/acceptance/engine.log) records `level:/ai/models/1290`, Normal3 schedule, condition124 at node41 (line918; the delegated starting vicinity was line885). The trace does not identify the actor instance. `world/0/163`, `world/0/164` and `world/0/165` share this model. Their saved Brain normal slots are4,4,0 respectively; these saved slots do not identify which instance reached the observed Normal3 failure. Its effect on intro readiness remains unknown. This change carries no readiness, gameplay or runtime acceptance claim.

The older read-only `output/integration-resume-20261008/source-audit/ai.json` exposes connected unsupported Proc386 and Ultra Proc412 calls. Its unsupported MainActor position-field annotations are historical: current main already decodes and compiles those fields. This change does not duplicate that work.

## Original condition124 authority

The [Hype API catalogue](research/hype-ai-api.tsv) maps condition124 `SCT_ActorInSector`, signature `PR`, to entry00535460; Raymap's Hype condition catalogue agrees with the name. The independently reconstructed original body is committed as `93d7d9efcf859469338bd95e35d3eb0ae26e919c` in the translator's `reconstruction/channel-condition-task20` worktree:

- `/home/stephen/Documents/Codex/2026-10-08/task-20/hype-channel-condition/src/ai/actor_sector_condition.c`
- The accompanying `research/worker-deltas/task20-channel-sector-handoff.md` records original PE/static evidence and the exact three-stack-word cdecl ABI.

The handler reads the condition ID and clears the full result DWORD before evaluating anything. Unsupported IDs evaluate nothing and return the original cursor. For124 it evaluates the actor with the original caller, follows the resulting **Perso** through Perso+4 StandardGame, existing00438e00 to its SuperObject, SuperObject+4 Perso, Perso+0x28 holder, and00447f90(holder) to holder+4 current-sector **SuperObject**. It captures that value before evaluating the second operand with the same original caller. It compares the second raw payload DWORD to the captured pointer and stores full DWORD0/1. It consumes no Ultra state and writes no result kind; the outer condition evaluator supplies the integer kind. There is no geometric containment calculation or added actor/null validity guard. A stored sector0 can equal a second raw payload0, but a null actor/StandardGame/SuperObject/holder chain remains invalid.

The native port uses a guarded failure for an absent actor/holder binding, rather than manufacturing false or evaluating later operands after an invalid chain. A bound null current sector remains valid. Numeric raw-zero second operands are recognized without conflating float negative-zero with pointer0. The actual sector references resolve native named nodes, so Godot/Rete never reconstruct source virtual addresses.

## Actual source identities

A temporary read-only Python source reader decoded the existing SNA envelopes and relocation/allocation data, including the active GPT binding of Fix StandardGame back-references. It used installed LZO decoding for source bytes; no Astrolabe or original-game binary was executed, and no assets were produced. The retained allocation identities below are evidence, not runtime lookup keys.

All four condition124 sites in this shared model use the same actor operand: Fix block5/0 allocation23322, resolved by the source actor map to `level:/persos/world/0/2` (**StdCamer**, not Hype). Hype is `world/0/0`. The initial handoff incorrectly labeled world/0/2 as Hype; the follow-on actor-data review corrects that label. Its chain is StandardGame Fix5/0 allocation23324 -> active SuperObject Level6/2 allocation17618 -> the same Perso23322 -> holder Fix5/0 allocation23359. **The holder's saved current-sector pointer is null.** No sector character list in this source tree contains `world/0/2`. An original active-game initial-sector assignment has not been observed here; the saved null must not be replaced by a guessed containing sector.

| Site | Sector SuperObject allocation, Level6/2 | Semantic scene member | Name | Distinct Sector data allocation |
| --- | --- | --- | --- | --- |
| Normal1 rule0 node1 | 4218 | `level:/scene#world/1/13` | sctmanoir03 | 8934 |
| Normal2 rule1 node1 | 4154 | `level:/scene#world/1/5` | sctforet | 5382 |
| Normal3 schedule node41 | 4234 | `level:/scene#world/1/15` | sctmanoir05 | 9953 |
| Normal5 rule0 node1 | 4346 | `level:/scene#world/1/25` | sctpont01 | 16583 |

The actual category27 operands identify the **SuperObject**, despite Raymap's generic `SectorRef` label. Mapping them by the Sector data allocation would leave these operands unresolved.

| Ambient instance | Perso allocation, Level5/1 | Holder allocation, Level5/1 | Saved current sector |
| --- | --- | --- | --- |
| `world/0/163` i_DummyAmbSonManoir1_B | 9276 | 9284 | `world/1/25` |
| `world/0/164` i_DummyAmbSonManoir1_A | 9287 | 9293 | `world/1/25` |
| `world/0/165` i_DummyAmbSonManoir1 | 9295 | 9301 | `world/1/25` |

The sector's ordered Perso list (Sector component+28) also contains these three ambient actors. This agreement is independent evidence; the actor's holder owns the current-sector value used by condition124.

## Connected sound authority and implementation

The source translator's complete0053e7e0 handler is committed separately as **`0147d41994738cac2f05fe3a8a8d980358f21729`**, immediately after93d7d9e. The initially reported9b6ce041 hash was corrected by the translator and must not be used. Read `src/sound/ai_object_volume.c` and `research/worker-deltas/task20-object-volume-handoff.md` in that same translator worktree.

Proc386 evaluates one operand on the original caller, takes the full raw DWORD, reads the current effects-source global **after** evaluation, then calls existing004ed840 on the original caller. It does not select Ultra, coerce, narrow, clamp, stop playback or write procedure control. Existing004ed840 in `/home/stephen/Projects/hype-decomp/src/sound/event_dispatch.c` enters the sound lock, calls004eb750, refreshes microphone/request parameters with004ec710, and unlocks. Lower004eb750 removes the override only for exact DWORD127; other words are retained unchanged. Only387 selects Ultra in the shared handler. Other shared aliases and reset cases are outside this ambient consumer slice and remain explicit unsupported boundaries in Astrolabe.

Proc412's existing reconstructed0053ec70 handler in that original project's `src/sound/ai_requests.c` selects Ultra first, evaluates target then duration on the original caller, and invokes0040bad0(duration,target,receiver). It does not block or write procedure control. `src/sound/fade_requests.c` admits the first free one of ten global fade-out requests, captures004ed8b0's current override (127 if absent, per004eb840), and preserves existing fades when capacity is exhausted. The read-only raw0040b900 updater interpolates with signed32 division after DWORD multiplication, clamps the computed level0..127, writes the effects override, clears the active request before004ecc00 kills this receiver's effects and removes its override. It does not kill the separate voice module. The maintained native emitter now implements this bounded operation with native Tweens and an effects-only completion/reset. Speech completion publication follows the existing next-frame bulk-kill contract.

Compiler lowering captures the receiver before target/duration, evaluates each operand once in caller context, and preserves numeric scalar payloads: float values become their source float32 bits; integer/bool values retain the low32 bits. Actual Proc386 operands are the model's integer DSG0/DSG1 fields, assigned through existing explicit integer casts and bounds in the script. Proc412 targets are integer0 and durations read Global DSG234. Native duration0 and durations above the positive signed32 range fail explicitly; the original updater has no zero-duration guard and can divide by zero. Native Tween clock scheduling, timer wrap, driver gain limits and detailed original sound-request lifecycle parity remain unverified. No source interpreter, memory table or original scheduler was added.

## Ownership and integration

- Typed AI nodes, recursive validation and ordered traversal cover sector references, condition124, Proc386 and Proc412. The OpenSpace script writer emits the original categories/IDs and requires assigned sector handles at its boundary.
- Scene-local actor membership is attached to the actor's scene-node properties, separate from reusable Perso assets and sector character lists. Typed PersoSectorWire and SectorCharacterListWire declarations read it. The previously misleading holder field names are corrected: word0 is previous-sector, word4 is current-sector. Native mutable membership remains on the Character component.
- Godot conversion saves named sector NodePaths and known/null membership. Missing bindings remain failures. Existing directional `source_contact_sector`/SectorBoundary maintenance is reused; condition124 calls the read-only `source_current_sector` path and does not calculate a crossing.
- Existing authored initial `sector.persos` contact bindings are kept when no imported holder exists, without falsely marking a source holder known. `source_refresh_contact_sector` marks explicit membership updates known. This touches the same Character/contact initialization area as other sector work; Kip must reconcile that overlap. Initial placement/activation refresh of StdCamer's saved-null holder remains an explicit integration gap, rather than a hardcoded sector or query-time geometry fallback. Generated-drop inheritance also belongs to the existing sector-maintenance owner. The follow-on below distinguishes startup from proved camera publication and motion maintenance.

## Follow-on: actual camera ownership and sector lifecycle

Source actor data identifies world/0/2 as StdCamer (family2, camera Fix5/0 allocation23339); world/0/0 is Hype (family1, no camera). Condition124 therefore follows the camera actor's current sector, independently of Hype's position. This corrects the initial handoff's mistaken human label without changing the decoded identity or operand binding.

Original loader00402b60 calls00437a70 (event1 tree initialization), then00403980. That function calls00437d70/event4 tree initialization except engine modes3/5. Tree dispatcher00437dc0 selects the regular/special actor path using StandardGame+0x24 and+0x1b. Regular component dispatcher00437590 invokes event-table005e4bb8 in24-byte strides. A static read of both existing source/installed PE profiles confirms row1 at005e4bd0 points to004389b0 (StandardGame), row5 at005e4c30 to00440820 (camera), and row10 at005e4ca8 to00447900 (sector holder). The full component record for Perso+0x28 starts005e4ca0; its regular/alternate event slots are005e4ca8/005e4cac. The source profile SHA256 is `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`; installed profile is `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`. These were read as data, never executed. Raw00447900 is missing; the translator's independent PE recovery below supplies its callback behavior. No C reconstruction was duplicated.

Existing004389b0 calls00447bf0 during events2..6 only when the effective active byte is clear. Original00437740 activation independently calls00447bf0 before publishing the active byte. Original00402780's saved-state/mode3 path also calls004390e0, which researches the main actor's saved position and, on a hit, refreshes its sector. These gated paths establish that saved null alone is not evidence of the post-startup holder value. They do not justify an unconditional native ready-time reseed. Native Character ready sets position history and active state; camera ready publishes the native viewport owner and keeps private sector context. Neither proves a populated StdCamer holder. Its automatic Brain starts deferred after ready.

A concrete missing camera publication is implemented separately: original0051c7b0 calls00447bf0 with the selected third/second position in **both** branches before writing local/global translation. Native `source_publish_mode_zero_translation` previously wrote the pose and private `_source_active_sector` without updating the Character's holder. It now reuses the existing ordered004e0020 resolver and existing source-bound last-sibling fallback to publish the selected position's sector before the pose write. This is an explicit camera lifecycle call; it is not a condition124 fallback or a ready-time containment guess. The first follow-on retained the old branch policy while00448350's return was unresolved; the second follow-on below corrects that policy from the recovered pointer ABI.

Original00463df0 updates00448280 before collision reads; the latter writes holder+4 and membership lists when boundary history changes sector. Native camera history previously updated only its private sector. It now publishes changed history to the Character when a known nonnull holder already exists. The private initial point-resolver seed does not overwrite an imported null. The ordinary post-walk and mass-contact native paths also call the existing directional contact-sector maintenance after movement; original0059b83c/0059bc74 and reconstructed mass_contact.c call00448280 after their translation writes. No sector selection or containment algorithm was added, and null remains null in ordinary motion maintenance.

Actual ambient effect: before an explicit source-supported camera sector publication, all four comparisons against nonnull sector literals are false for the captured saved-null StdCamer holder. Existing viewport-selection refresh already publishes a camera holder when that selection branch runs. The new mode-zero publication and changed-history hookups prevent a private camera sector from diverging silently from the holder subsequently read by the ambient scripts. Exact startup sector, which publication occurs first in the accepted scene, ordering across Brains, and intro readiness remain unverified. No tests, builds, lints, formatter checks, runtime work or exports were performed for this follow-on.

## Follow-on: recovered final-child ABI and startup callback

The translator recovered original00448350 independently in `src/actor/sector_list_lookup.c` and00447900 in `src/actor/sector_component_events.c`, with declarations/dependency limits in `src/actor/sector_component_source.h`, in the same task20 worktree. Commit **`b94813c3d6b508aa53f7427b0f9844ecb89a539d`** contains exactly those two new bodies and private `research/worker-deltas/task20-sector-startup-{coverage.tsv,evidence.json,handoff.md}`. The C bodies and handoff were read as source; Astrolabe did not duplicate that recovery.

00448350 returns the final sector-root **child pointer in full EAX**, not a count or void result. It reads root00756ea0+8 first child and+0x10 child count, then follows child+0x14 sibling links until the signed low16 counter comparison reaches count-1. Consumer0051c7b0 captures that pointer before004e0020 research, and chooses its third position only for a nonnull research result unequal to that pointer. A no-hit or final-child hit selects the second position. Raw0051c7b0 lines14–24 retain the consumer and placement-before-matrix order even though raw00448350 lost its sibling reads/return.

Static source-data inspection resolves GPT field+0x18 to the sector root `world/1`, Level6/2 allocation4106. It has29 children, all kind8; the last is `world/1/28`, **Univers**, Level6/2 allocation4370. This differs from the ambient actors' saved `world/1/25` sctpont01. The existing exporter binding `source_sector_fallback_path` therefore supplies the precise named final-child identity for this Manor branch without persisting addresses or rebuilding source lists in Godot.

Native publication now captures that final-child node before researching the third position. It compares node identity, accepts a complete no-hit result, chooses the second/third branch, refreshes the chosen position's holder, then writes the pose. The caller's precomputed lookup and optional sector-ID argument are removed so research cannot precede the captured final-child reference. The old incidental collision-history refresh at publication entry is removed as well. Missing final-child bindings, unresolved nonnull research identities and incomplete geometry remain explicit failures before pose publication. Existing type1/11 child geometry and multiple source LOD limits are retained.

00447900 is a two-argument cdecl sector-component callback. Event1 clears holder+4, registers that field for snapshot handling through00447ba0, then falls through event6; event6 clears current/previous sectors and character/light/stream links. Events2..5 enter common placement directly. Common placement gets Data3D+0x18 matrix translation, calls00447bf0 on the StandardGame's SuperObject, then writes the position words into the reloaded holder+0x0c/+0x10/+0x14. Original00447bf0 writes holder+4 after research/default-final-child fallback, updates membership lists, and publishes holder+0. Its wider helper bodies remain external to this exact two-body translator assignment.

The callback and00447bf0 preserve repeated00447840 membership and004ea270 snapshot-mode calls in their original short-circuit order. For stable state they enter placement for **nonmember+mode2, member+mode4, or mode0/1**. An earlier message to the translator reversed the mode2/4 membership polarity; its PE correction supersedes that message. The port has no corresponding native startup snapshot-mode/membership-list contract yet. Consequently this isolated change does not apply unconditional ready-time placement or replace StdCamer's saved null with a guessed sector. That startup mapping, exact reached membership and its ordering before ambient Brains remain integration boundaries. Main integration is paused; no runtime/readiness acceptance is claimed.

- Rete schema62 and support interface83 are provisional increments from61/82 and require reconciliation with concurrent integration. No serialized or generated data was regenerated. Whole-level OpenSpace component export remains outside the existing scene-graph writer's supplied-component contract; no whole-package parity is claimed.

Under the strict no-tests constraint, test source was left unchanged. Integration must mechanically supply `actor_sector: None` in the explicit `SceneNodeProperties` literal at `tests/hud_quarantine.rs:142`, and reconcile version assertions currently61 in `tests/ai.rs` and `tests/camera_shift_getters.rs`, and82 in `src/godot/support.rs`'s test module. Existing `tests/check_camera_projection.gd:426/432` also retain the old same-active-sector branch policy and a three-argument publication call; they need the final-child binding, recovered branch expectations and two-argument API when test changes are authorized. These are known source migrations, not completed validation. The new source has only been read/reviewed, not compiled or syntax-checked.

Source input SHA256:

| File | SHA256 |
| --- | --- |
| Fix.sna | `36b5557a7d9d8d1f7636fcf88f945dc056ed1484f2b169322b13561c2099360e` |
| Fix.rtb | `7618b06dfda9339b296c74536a2acba11187e0de9f97790afd0681f493fb3484` |
| manoir.sna | `9dae1c2612535eab0eb823f838bdb845d0d1dccba26f3c0814d81f3541c51af4` |
| manoir.rtb | `09c1ee57dc91156253eafcbf3867755e6b13ccf3b7f2b74bfdf01e4d6e1feed7` |
| fixlvl.rtb | `05f42c6c9ad2be69d97357161659fea92c1cb8ae25c04bd45b21ff3910fee396` |
| manoir.gpt | `7ed883434058593ce4280c803ea80a81b1ecbe0bf5ded3c9784ac9656469cddc` |
| manoir.rtp | `8e553d7da2030a33fb3e20ef80bc18be46d2904794dc1edb584ce7e832f575b3` |
