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
