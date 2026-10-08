# Manor presentation static follow-up, 2026-10-08

This source-only branch starts at `d493ef24859d9b2a0b18ed68f5ebb845e7feeebb`.
It lives in `output/worktrees/manor-camera-dialogue-static-20261008`, branch
`worker-manor-camera-dialogue-static-20261008`. Kip owns integration and
acceptance. No tests of any kind, compilation, lint/formatter runs, Godot,
original executable, harness, replay or gameplay experiment were run.
No original functions were rewritten here. Reference files, the main checkout,
generated game content and peer worktrees were read only.

## Connected changes

The existing compiled AI calls `source_run_dialog(name, mode, self)`, where
`self` is the actor's native Brain. Previously Screen ignored that argument and
bound the saved Dialogue owner. Complete original `00543c00` writes the
executing actor into the active dialogue's owner before InitDraw. Screen now
resolves the calling Brain's parent and uses that actor's emitter and Brain for
speech and the pending completion. It preserves the resource's saved owner as
conversion metadata and an authored default when there is no caller. A caller
without the ordinary actor/Brain relationship, or a completion referring to a
different model, remains an explicit unsupported binding.

`00543c00` also stops MainActor's speech through `0041e910` before RunDialog.
The maintained emitter now provides `stop_dialogue_speech`, which clears its
record and stops its voice-module players without stopping object effects or
shared music. Screen checks the selected presenter's `can_run` before that
side effect. Both presenters expose the same validation used by their `run`
methods. Invalid resource/mode requests therefore remain observable failures
and do not erase a valid active presentation.

Original `0041e8a0` dispatches on the voice module, while sound127 and413 use
the effects module (`src/sound/ai_requests.c` in the reconstruction). Previously
dialogue executed its program on the effects emitter's default bus and inherited
its effects volume. Native `play_dialogue_speech` now dispatches the already
compiled sound program on **Astrolabe Voice** and excludes those players from
object-effects volume changes. Existing Voice bus gain/fades continue to apply;
starting speech does not reset that bus. Ordinary effects and music retain their
existing route and object gain. A stale voice-finished closure is guarded by
the presentation generation before clearing its holder.

Source-presented ordinary dialogues (styles1/3) now use the same recovered
alpha-step semantics already used by the title presenter. Original
`0056d860` opens at most once per draw after elapsed exceeds10ms and increments
by float32 `0.10000000149011612`. It compares before the float32 store;
equality is not terminal. `0056d960` closes once per draw; the10ms condition
only gates timer reset/start. End clears text immediately; the stencil remains
until its close. `0056c8c0`/`0056da60` allow the over query to finish the close at
alpha at most double0.1. The UI keeps its input/continuation ownership during
that close. The previous configurable Tween remains available for authored
items without a source presentation; it no longer chooses the source fade
duration. No artificial fixed duration is assigned to source frames.

Cond120 still waits for both presentation completion and MainActor's distinct
speech holder before consuming the pending behavior. Cond121 remains a query.
The existing callback scheduling/consumption mechanism, title reveal timers,
AI-authored title waits and letterbox toggles are retained.

The translator's subsequent instruction review corroborated the outer drawing
and line-deadline branches in `0056c2d0` and `0056cb10`. The first current line
initializes the deadline. Each rendered line replaces it when its **first**
font record has a present text string; an allocated empty string qualifies,
while a missing string does not. The reader previously flattened both into
`""`. `DialogueSpan.text` now preserves `Option<String>` through the shared
model and Rete, and the Godot exporter emits semantic `text_present` on native
span Resources. No source pointer survives. Source deadline values are rounded
to float32 before use, matching their imported scalar precision.

Ordinary source presentation now selects using that line-order deadline and
strict `playback > deadline`; equality and unordered comparisons do not advance.
It compares playback only while the owner's speech record exists. Once a
voiced final page is exhausted and the holder clears, an ordered deadline
unequal to the exact float32 untimed sentinel selects and closes. The importer
already excludes successors and choice/module side effects, so that represented
subset has no nonnull successor for the source's unconditional EndDraw to
discard. Unvoiced source pages no longer advance on a guessed wall-clock
deadline. Active source drawing waits for MainActor's speech holder, with
timers continuing during the wait; closing after EndDraw remains independent.
The configurable authored presentation retains its prior deadline behavior.
On the first active source draw, speech starts and the zero-alpha panel is
initialized before returning. The next active draw begins alpha stepping;
title presentation also keeps this initial draw distinct from opening. Both
timers still accumulate from RunDialog. This uses the native component's
presentation lifecycle, without a second source script scheduler.

## Static evidence reviewed

- Reconstruction: `src/dialogue/dialogue_dispatch.c`, `dialogue_lifecycle.c`,
  `material_alpha.c`, `speech_record.c`; `src/sound/ai_requests.c`,
  `theme_request.c`, `theme_parts.c`, `resource_transition.c`,
  `request_playback_position.c`, `resource_playback_position.c`; and
  `src/render/snapshot_fade.c`, `fade_draw.c` in
  `/home/stephen/Projects/hype-decomp`.
- Original Ghidra exports for `0056c2d0`, `0056cb10`, `0056cc70`,
  `0056c690`, `0056e510`, `0056e840`, `004ecb60`, `004f45e0`, `004f4630`,
  `004f6620`, `004f6660`, `004f72b0` and `004f7380`.
- Existing complete Manor generated AI and the preserved source dialogue audit
  were inspected, without regenerating or executing them. The eight named
  voice samples support eight required first-visit dialogue calls; their saved
  owners remain Gogoud actor5 and statue actor28, matching their callers.
  Camera targets/travel and the final Gogoud12 handoff remain source decisions.
- The source translator independently inspected the copied-disc records for
  those eight first-visit names:37 lines and41 spans, each span text present
  and nonempty. This is exact subset evidence, not a claim about null or
  allocated-empty strings elsewhere in the67-item catalogue. The optional
  text promotion preserves the general source distinction without needing
  that subset assumption.
- The translator then traversed the entire loaded Fix+English Manor catalogue:
  67 items,591 lines,880 spans;161 text values are absent (160 first spans),
  719 are present and nonempty, and none are allocated empty strings. No first
  span is whitespace-only;18 later spans are. This confirms null first-span
  padding in the shipped catalogue while preserving allocated-empty behavior
  for authored/modified content. The existing semantic audit contains27
  supported resources and no unvoiced ordinary item; its flattened text alone
  could not supply this presence evidence.
- PE section-mapped static reads in both independent executable profiles:
  `005c4368..005c4373` contains float32 `-0.1,1,+0.1`; `005c0c48..005c0c4f`
  contains letterbox fractions `.875,.125`; both dialogue line-count tables
  contain `[2,6,10,2,2,6,10,0,10,10]`. Styles1/3/4 therefore retain6/2/2 lines.
  Read ranges `0056d860..0056d95f` and `0056d960..0056da4f` are identical
  between profiles. This establishes those inspected ranges, not whole-function
  or whole-executable equivalence. The ignored static-read report is
  `output/static-ui-source/pe-reads.json` inside this checkout.
- Additional section-mapped source-profile reads establish the untimed float32
  `3.3999999521443642e38` at `005c4310` and playback scale double
  `1.52587890625e-05` at `005c4308`. This proves the scalar operands, not the
  external driver's time units or the Godot playback clock.

Static review followed the changed call sites through the compiled AI API,
screen adoption, validation-before-replacement, close/input handoff, voice
completion and component teardown. This is review evidence only; GDScript
parsing and behavior are unverified by this branch.

## Retained camera, music and UI contracts

Mode0/3/6/-1 transitions remain in the existing source camera component.
Mode6 preserves orientation while the travel actor moves the eye; -1 restores
the imported initial mode. The World/travel/statue/Gogoud/Hype graph continues
to own eye writes, waypoint completion and3600/4500ms opening shot waits.
Neither a second camera controller nor an intro timeline was added.

`00421fa0` proves the letterbox flag toggles immediately. PE fractions confirm
the existing60px top/bottom bars at640x480 and360px inner fade area. Source AI
continues to remove letterbox in Gogoud12; dialogue completion does not invent
that toggle. The title's200/3000ms waits and per-draw reveal remain intact.

Music same-theme retention, null requests, native natural part order/looping
and the long PCM path are retained. Different active themes still fail closed.
`004faa60` selects CROSSFADE via `004f4630` or ATENDOFPART via `004f45e0`;
neither wrapper supplies a duration. The next layer dispatches through imported
driver callbacks. A zero CSB fade field does not prove an immediate theme cut.
No guessed crossfade or replacement policy was added.
The completed SXD/MIDI transition thunks confirm that both with-fade exports
receive only the handle and next resource. They forward the driver's complete
EAX result; neither a guessed Boolean result nor a native fade duration follows.

## Exact source follow-up for Kip's translator

| Priority | Bodies / dependency | Needed evidence before further native changes |
| --- | --- | --- |
| Dialogue reveal/draw | Completed `0056c2d0`, `0056cb10`; reconstructed selection bodies `0056c690`, `0056e510`, `0056e3b0`, `0056e840`; inner `0056cc70`, `0056db30` | The complete outer draw bodies agree with the represented native subset above. Inner renderer/reveal helpers and unvoiced ordinary reveal remain unfinished. Do not infer choice/module support, successor handling or a portable x87/driver clock from this patch. |
| Speech clock | `004ecb60` → `004f47d0` → selected driver callback; `0041e4c0` | Native playback seconds currently stand in for the driver's integer position scaled by65536. Exact driver units, missing-request handling, lip-animation selection and frame association remain unresolved. |
| Camera inputs | `00547620` (Proc153, missing raw export), `0051e620`, `0051c8d0`; failed-candidate path `0051ec70`, `0052c7f0`, `0051e350`, `0051e600` | Exact input angle writes and interpolation exits/consumers. The supported passive Manor modes must not be replaced by a guessed smoothing path. |
| Music replacement | Completed `004f6590`/`004f6620`/`004f6660` and `004f7220`/`004f72b0`/`004f7380`; imported SXD/MIDI driver targets | Recover PlayTransition, SetNextTransition and DoTransitionWithFade behavior, buffering, boundary selection and time units inside the external drivers. Complete main-executable thunks alone cannot establish a native fade duration. |

The parent messaging tool returned `thread not found` for Kip's delegated
source thread. Direct messaging to the source translator succeeded; that
handoff names the unresolved draw bodies and driver dependencies above.

The translator's twelve selection/timer/module/predicate bodies are committed
as `29f3aaa`, with the evidence and branch-specific limits in
`research/worker-deltas/sol-manor-presentation-dialogue-handoff.md`. They were
read as source evidence; this branch does not import or rewrite those C bodies.
Inner drawing/renderer and driver timing remain external.

Completed source follow-up was then read from the same reconstruction checkout:

- `78a33fc9ee2d8b23285f00fbd1aad749a55f2805`,
  `src/dialogue/dialogue_draw.c`, `include/hype/dialogue/draw.h` and
  `research/worker-deltas/sol-manor-presentation-draw-handoff.md`, with exact
  PE and copied-disc presence evidence. Static comparison confirms the native
  represented subset's first-span-presence deadline replacement, MainActor
  gate, first active draw, strict playback comparison and ordered non-sentinel
  final-holder-clear closure. No additional production edit was required.
- `cbbcf6f696f6c5a1db7edc4860340ad03eff2ff4`,
  `src/sound/transition_driver_thunks.c`,
  `include/hype/sound/transition_drivers.h` and
  `research/worker-deltas/sol-manor-presentation-driver-handoff.md`. Six SXD/MIDI
  forwarding bodies preserve arguments and full EAX results. The external
  transition policy, callback scheduling, resource layouts, null-resource
  handling, fade duration and clock units remain unresolved. Existing
  different-theme failure remains appropriate pending those contracts.

The source worker also reports child/channel attachment commit
`f389490c70f71fe8bb345e660a1be5b34adf082f` for the key worker's separate
dependency. That body is outside this presentation patch; MainActor/channel20
attachment is not evidence of key acquisition. Kip owns all source caller,
shared-declaration and coverage integration.

The source translator subsequently delivered camera reset commit `d9b1c9d` on
`reconstruction/sol-manor-presentation-task19`, checkout
`/home/stephen/Documents/Codex/2026-10-08/task-19/hype-presentation`.
`src/camera/camera_initial_reset.c` restores the41-word initial parameters,
clears ordered component fields and raw bounds, and **retains the target**.
This confirms the reset interpretation and is handed to the training worker,
which owns typed Proc136 and mapped native reset. This presentation branch
does not edit `camera/source_camera.gd`, `character/character.gd`, any typed
AI/model/import/compiler path, or the training worker's reset work.
Training subsequently committed its agreed scope as
`84a124449b4470194e4080a508ed190dcaa5a5bf` in
`/tmp/astrolabe-manor-training-static-oct08`, branch
`sol-manor-training-static-oct08`. Its native framing Resource and Proc136
remain that worker's deliverables; this note routes the verified Git hash to
Kip because the direct lead endpoint is unavailable. The training handoff
reports no tests/builds/runtime and matching provisional Rete59/support78.

## Integration and outstanding verification

The first commit, `333a728`, changes maintained support scripts, the support
interface stamp and this note. Its follow-up adds the useful nullable text
meaning to the model, OpenSpace reader, Rete and Godot resource export.
Support77→78 and Rete58→59 are provisional; Kip must reconcile both numbers
with sibling changes. `src/godot/support.rs`, `tests/ai.rs` and
`tests/camera_shift_getters.rs` update only their existing version/schema
assertions to match; no test was added or run. No new support file registration
or test-manifest entry is needed. Existing generated dialogue resources and
Rete packages need regeneration after integration to recover missing-string
presence; old flattened data cannot establish it.

This branch does not touch `demo.gd`, `demo.tscn` or `demo_camera.gd` and is
independent of the lead's `game_session`/`session_camera` rename. Potential
integration overlap is the maintained `audio/emitter.gd`, `ui/screen.gd`,
`ui/dialogue.gd`, `ui/dialogue_span.gd`, `ui/location_title.gd`, `version.json`,
`src/godot/support.rs`, `src/godot/dialogue.rs`, `src/model/dialogue.rs`,
`src/openspace/level/dialogue.rs`, `src/rete/mod.rs` and the two existing
schema assertions named above. Training was informed of this extended
ownership and the schema stamp; no camera/typed paths overlap.

Required later acceptance includes GDScript parsing, eight first-visit dialogue
calls and callback order, Voice-vs-Effects gain isolation, repeat/cancel/stale
completion, source panel draw cadence, final letterbox release and actual
player control through the complete authored New Game Manor. Also verify
missing versus allocated-empty span text through source→Rete→native Resources,
last-present-first-span deadlines, strict equality boundaries, MainActor draw
waiting and final-page owner speech completion. No syntax,
runtime, timing, lip-sync, pixel or full-Manor acceptance is claimed here.

## Proc307 GameOver follow-on and forced voice completion

The lead requested this small connected follow-on after the first three
presentation commits. It remains on the same isolated baseline branch. No
build, compiler, formatter, type/lint check, test, export, Godot, original-game
or other runtime command was run. Only original source, source asset metadata,
existing implementation and authored diffs were inspected. No original-game
or generated assets were written. The provisional branch stamps advance to Rete60/support79; the lead
must reconcile the unpublished union once before any later content conversion.

Original-engine source commit
`c208b411a2cd3905369d8515cdcc17358e6f1e2e`, read from the presentation source
worker, supplies six complete connected bodies in `src/presentation/game_over.c`
and its own header. Its `research/worker-deltas/sol-manor-presentation-game-over-handoff.md`
records exact PE/source corroboration. The shared Proc307/308 handler captures
the procedure, evaluates the string and integer operands in order, and only
queues a vignette for a strictly positive signed32 second operand. Proc307
then requests event4. Zero/negative values consume both operands and return
without either effect. Proc308/305/306 are outside this native follow-on.

The actual Global call is Fix AI model1, physical Normal member **2**, schedule
node166, string `multilanguage\gameover.bmp`, integer **9000**. This corrects
the earlier generated-script `Normal0` label; physical member0 has only15
schedule nodes. The original pending presenter constructs/draws the vignette,
then waits. Its nonzero argument reaches the untouched WinMM millisecond
counter through the original timer wait. Elapsed comparison uses wrapping
unsigned32 subtraction. Initialization is a WORD check, status is AX16, and an
uninitialized timer returns7fff without waiting; the presenter ignores it.
The zero-duration input loop exists for direct queue users, but Proc307's
positive guard cannot reach it. No source fade appears in this chain.

The raw original vignette loader corroborates case-insensitive
`Multilanguage\` insertion of the selected language and extension replacement.
Read-only CNT table/GF-header inspection found English, French and German
`gameover.gf` members, each640×480. The current scene importer already admits
English only. Its new dependency pass resolves English artwork through the
existing CNT codec and canonical alias reference. The typed statement retains
the original string and an optional useful texture reference. Source encoding
writes the same string/integer operands; Rete retains the semantic binding.
Native export saves that texture as PNG and preloads it from the compiled AI.
No source path, pointer, archive record or allocation table is interpreted by
Godot. Unbound standalone scripts fail only when a positive request is reached;
noninteger native operands also fail rather than guessing raw union semantics.

The session queues the request after operand capture and consumes it deferred
after the current AI call, preserving same-frame request replacement. It owns
the pending/menu route independently of the source game-state selector and
save state. The native presenter is an instanced CanvasLayer/TextureRect scene,
with a real millisecond hold starting after the first drawn frame. Gameplay,
camera and input are frozen during the hold; music remains governed by the
already executed source music requests. Presentation teardown precedes the
existing `leave_session(bootstrap)` menu route. Native timer availability is
assumed; original uninitialized-timer diagnostics and renderer/session driver
internals are not reconstructed. Exact frame-boundary, viewport stretching,
menu audio transition and real elapsed timing still require runtime acceptance.

Forced voice cleanup follows the source distinction supplied by the translator.
The explicit owner helper conditionally kills voice and always clears its
holder. Bulk request-kill wrappers do not directly clear that holder; source
speech animation later observes playback completion (-2/-1) and clears it.
The native emitter now stops players immediately, retaining completion
ownership until the next SceneTree process-frame. Natural and forced endings
then use the same guarded callback, with record/player/generation checks.
Explicit owner stop remains immediate; ordinary stop paths are separate.
This is a native adaptation for the missing source speech-update completion,
not a claim that Proc385 synchronously clears an owner record. It also works
when the death session disables level processing. Cross-worker death calls to
`kill_all()` can use this path without duplicating holder cleanup.

Read-only inspection of integration `f1afb15` found an added immediate
`emitter.stop_dialogue_speech()` after `kill_all()` in its retained terminal
death session (`game_session.gd`, near line74). That was compensating for the
old missing native finish callback. **Remove that compensating call and its
two-line comment when integrating this completion adaptation.** The retained
actor's holder then clears on next-frame completion even though its level is
disabled. The explicit dialogue API calls elsewhere still use immediate clear.
`character.gd`'s ordinary kill only calls `kill_all()` and needs no extra clear.
This branch does not edit the integration or death worker's files.

Rename-aware integration must map this branch's historical `game/demo.gd` and
`demo.tscn` hunks to accepted `game/game_session.gd` and `game_session.tscn`.
Do not restore old filenames or overwrite the death worker's independent
terminal/life logic. Shared typed-model, walker, codec/compiler, content import,
support/schema and dialogue/audio hunks may conflict with the unpublished
union. New support files are `ui/vignette.gd`, its UID and native scene; the
authored session instances that same maintained component. Existing schema
assertions were changed mechanically only and were not executed.

The separate follow-on below implements the real key resource `keysico_gogood`:
box5, five flags3 lines, two null-text padding lines, one completion behavior
to key703 Normal2, no voice and no item/answer icon. The prior importer/render
subset rejected it; the separate source review establishes the panel/text
geometry and selection-after-page-complete contract. No generic pickup popup
or timed hold replaces that resource.

## Independent style5 key-dialogue follow-on

This follow-on changes only the source decoder, useful dialogue validation,
native exporter diagnostic, and maintained dialogue/font/frame support. It
does not change item acquisition, key stock, attachment, training or death
logic. Rete60/support79 remain the unpublished branch contract from the prior
GameOver batch; no further fields are added to persistence or native Resources.
There are no generated assets or changes to main/peer worktrees. No checks,
tests, compiler/formatter/lint/type commands, exports or runtime experiments
were run. Review is source/data/diff inspection only.

The copied-disc resource `keysico_gogood` belongs to world key142/model703.
It has one answer, box5, five lines, no voice, no item/answer icon, no channel
lists, no stock/gold/action/module side effects, no answer text or branch/result
map, and a Normal2 completion reference. Both first lines contain a single
normal-font record with an absent text pointer; those are padding records.
Line2 has normal/emphasis/normal spans, line3 normal, line4 emphasis/normal.
Every line has flags3, no drawing layout, and exact untimed-sentinel bits
7f7fc99e. The semantic missing-text values already survive Rete/native export;
the new admission keeps those empty rows in the five-line page.

The original source translator independently corroborated the selector tables:
style5 draw page size and selection stride both6. Each flags3 line bypasses
the draw reveal-limit check through bit0, including the null padding records.
The wrapper consequently consumes the entire five-line page and marks it
complete/final. Bit1 blocks selection until page-complete becomes nonzero.
After an accepted selection the original scrolling helper advances the page,
enters mode0 and publishes the Normal2 continuation. There is no speech-started
flag or ordered non-sentinel deadline to trigger timed selection. Partial
letter reveal, choices, icons and next-page arrows are not needed for this item
and remain unsupported rather than simulated.

Source panel/calibration/text-chain helpers establish the geometry. Style5
draws only the center material, with columns8/rows4, signed integer texture
ratios and corner half-dimensions. Original Fix texture metadata and independently
decrypted CNT GF headers agree that the four stencil textures are32×32. At
calibrated origin0 the rectangle is left16/right304/top16/bottom176, or288×160.
The native frame derives those bounds from the converted textures, hides edge
and corner controls for style5, and restores the established frame layout when
an ordinary dialogue runs afterward. PositionDialog still owns the supplied
native source-pixel position; no pickup trigger/layout is authored into the
game session.

The native style5 font-record path maps the source's first-glyph half-width
addition and centered FON projection to native left bearings. Intervening
float32 stores do not prove bit-exact cancellation for arbitrary values.
Calibration uses the corroborated float32 constant
0.01406249962747097 and final float32 anchor stores; row pitch uses the
corroborated0.30000001192092896. The converted FontFile supplies the space
half-height and glyph bearings. Style5 now uses four ordered native emphasis
labels at the source diagonal offsets, followed by the normal foreground;
the admitted source font roles have identical glyph/atlas metrics. The
replacement source layout is zeroed after emphasis and remains equivalent
to the absent layouts admitted here. Existing style1/3/title geometry remains
on its previous paths.
Portable x87/renderer equivalence, exact emphasis pixels and full viewport
projection remain outside this native adaptation's static proof.

Importer admission is deliberately semantic rather than keyed to a resource
name: one unvoiced page, flags3, no layouts, exact untimed deadlines, and no
unrepresented answer effects. Answer fields are now a declarative80-byte
private wire record with explicit unknown prefix/runtime words. Stock lists,
actions, modules, answer speech, result mapping and signed selection word must
be absent/zero for this new subset. Unsupported style5 resources keep a clear
diagnostic. Native `can_run` applies the same representable page contract.

Read-only copied-disc SNA/LZO and relocation decoding additionally confirms
the existing importer admission fields: reserved header byte, item word+08
and item word+20 are all zero. The decoder's original guards therefore admit
this item; no meaning is invented for those unpromoted fields. No original
text, source pointers or data-derived assets are saved by this inspection.

The complete native page becomes eligible only after the shared source panel
opening/draw gate. Both padding rows count despite creating no glyph labels.
An accepted Button_Hit/SelectDialog then uses the existing Screen completion
owner and Proc330 continuation consumer; the UI does not invent item ownership,
modify MainActor DSG[56], or skip the key's later source behavior.

**Remaining key-control dependencies:** the preserved full key703 Normal1
path reaches unsupported camera Proc141 and Proc158 before RunDialog; its
Normal2 reset reaches Proc141 before MainActor DSG[56] is cleared. Additional
rotation/MTH calls on that path are owned by the key/mechanics workstream.
The source translator found Proc141's handler missing from raw/current clean
C and has not claimed its contract. Those callers still fail explicitly;
this UI follow-on alone does not establish item-pickup/control-release or full
Manor acceptance. The lead must compose their separate source/native camera
work with this resource, then authorize later validation of padding positions,
page-complete/early selection, emphasis, callback/cancel ordering and actual
player control. No popup, timed hold, camera substitute or gameplay bypass is
introduced here.

Source corroboration is isolated original-engine commit
`163b56c4fbf1282a79cead405541df4355a7521f`, following
`c208b411a2cd3905369d8515cdcc17358e6f1e2e` for the GameOver chain.
The five complete renderer/calibration helpers are in
`src/dialogue/dialogue_text_coordinates.c`, `dialogue_font_chain.c` and
`dialogue_panel.c`, with `include/hype/dialogue/renderer_internal.h`.
Their `research/worker-deltas/sol-manor-presentation-style5-handoff.md`,
`-evidence.json` and `-coverage.tsv` record the exact selected PE spans,
constants, table values and copied-disc semantic metadata. All full new C
bodies/header and the evidence were read for this follow-on. Font/quad driver,
arbitrary x87 precision and pixel parity remain external contracts.

Training sibling dependency: preserve both
`84a124449b4470194e4080a508ed190dcaa5a5bf` and its follow-on
`60a2aec4263ae41129fa706e97254be216047260`. That worker owns Proc136/reset,
immediate Proc166 groups0/1, direct camera-mode3 activation and actor cleanup
that calls `kill_all()` without synchronous holder clearing. Its settled
training stage5 snapshot requires item21 plus the inactive/suppressed key
after presentation/removal; it does not bypass this UI. Key703 Normal2's
Proc141 primary-viewport prerequisite must be recovered before claiming
the subsequent player-control release. Schema/support stamps60/79 are shared
provisional branch values, not proof that sibling Resources are interchangeable.
