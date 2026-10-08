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
- PE section-mapped static reads in both independent executable profiles:
  `005c4368..005c4373` contains float32 `-0.1,1,+0.1`; `005c0c48..005c0c4f`
  contains letterbox fractions `.875,.125`; both dialogue line-count tables
  contain `[2,6,10,2,2,6,10,0,10,10]`. Styles1/3/4 therefore retain6/2/2 lines.
  Read ranges `0056d860..0056d95f` and `0056d960..0056da4f` are identical
  between profiles. This establishes those inspected ranges, not whole-function
  or whole-executable equivalence. The ignored static-read report is
  `output/static-ui-source/pe-reads.json` inside this checkout.

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

## Exact source follow-up for Kip's translator

| Priority | Bodies / dependency | Needed evidence before further native changes |
| --- | --- | --- |
| Dialogue page/finish | `0056c2d0`, `0056cb10`; reconstructed selection bodies `0056c690`, `0056e510`, `0056e3b0`, `0056e840` | Complete draw branch order, owner speech ending, reveal state, pending continuation and final-page automatic close. The translator's selection/module bodies preserve the MainActor speech wait and byte stores. Raw `0056cb10` appears to select the last rendered nonempty line's deadline; current ordinary UI still takes the minimum finite deadline. No deadline/automatic-close change is made here pending complete draw corroboration. |
| Speech clock | `004ecb60` → `004f47d0` → selected driver callback; `0041e4c0` | Native playback seconds currently stand in for the driver's integer position scaled by65536. Exact driver units, missing-request handling, lip-animation selection and frame association remain unresolved. |
| Camera inputs | `00547620` (Proc153, missing raw export), `0051e620`, `0051c8d0`; failed-candidate path `0051ec70`, `0052c7f0`, `0051e350`, `0051e600` | Exact input angle writes and interpolation exits/consumers. The supported passive Manor modes must not be replaced by a guessed smoothing path. |
| Music replacement | `004f6620`/`004f6660` and `004f72b0`/`004f7380`, their imported driver targets | Recover SetNextTransition/SetTransitionWithFade behavior, buffering, boundary selection and time units. Main executable wrappers alone cannot establish a native fade duration. |

The parent messaging tool returned `thread not found` for Kip's delegated
source thread. Direct messaging to the source translator succeeded; that
handoff names the unresolved draw bodies and driver dependencies above.

The translator's twelve selection/timer/module/predicate bodies are committed
as `29f3aaa`, with the evidence and branch-specific limits in
`research/worker-deltas/sol-manor-presentation-dialogue-handoff.md`. They were
read as source evidence; this branch does not import or rewrite those C bodies.
Broader drawing/renderer and driver timing remain external.

The source translator subsequently delivered camera reset commit `d9b1c9d` on
`reconstruction/sol-manor-presentation-task19`, checkout
`/home/stephen/Documents/Codex/2026-10-08/task-19/hype-presentation`.
`src/camera/camera_initial_reset.c` restores the41-word initial parameters,
clears ordered component fields and raw bounds, and **retains the target**.
This confirms the reset interpretation and is handed to the training worker,
which owns typed Proc136 and mapped native reset. This presentation branch
does not edit `camera/source_camera.gd`, `character/character.gd`, any typed
AI/model/import/compiler path, or the training worker's reset work.

## Integration and outstanding verification

Only maintained support scripts, the support interface stamp and this note
are changed. Rete/model shapes and source readers are unchanged. The branch
advances support77→78 for the coordinated emitter/presenter contract; Kip must
reconcile that number with sibling support changes. `src/godot/support.rs`
updates its existing interface assertion to match; no test was added or run.
No new support file registration or test-manifest entry is needed.

This branch does not touch `demo.gd`, `demo.tscn` or `demo_camera.gd` and is
independent of the lead's `game_session`/`session_camera` rename. Potential
integration overlap is the maintained `audio/emitter.gd`, `ui/screen.gd`,
`ui/dialogue.gd`, `ui/location_title.gd`, `version.json` and
`src/godot/support.rs`.

Required later acceptance includes GDScript parsing, eight first-visit dialogue
calls and callback order, Voice-vs-Effects gain isolation, repeat/cancel/stale
completion, source panel draw cadence, final letterbox release and actual
player control through the complete authored New Game Manor. No syntax,
runtime, timing, lip-sync, pixel or full-Manor acceptance is claimed here.
