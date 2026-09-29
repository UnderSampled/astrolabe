# Development process reviews

## Purpose and scope

The user now requests a check every hour to review development progress,
offer useful process improvements, and evaluate whether earlier suggestions
helped. Start with task `01a0e4b6-3072-79d0-87a1-5bbf80f4f7cb`, currently titled
**Develop the save effort**, and include subsequent relevant Astrolabe tasks.
Use current task titles when reporting them; IDs identify tasks across renames.
The active phase is the [comprehensive semantic map](savegame-semantic-map-handoff-20260928.md).
Earlier entries describe the completed first persistence slice and its former
30-minute review schedule; they are historical evidence, not the new exit gate.

This is a longitudinal evidence log, not a new development backlog. Read recent
changes and existing evidence before sending advice. Preserve active tasks'
scope and ownership. Avoid duplicate recommendations, repeated broad test runs,
and messages that merely ask for status. Stay quiet when nothing actionable has
changed. Never attribute an improvement that predates a suggestion to that
suggestion.

## Review method

The user's latest runtime requirement is that graphical game/test runs use
Xvfb so windows, focus changes and injected input do not interrupt their
desktop. Check that Sol and delegated agents apply the virtual display to
launch, input and capture consistently. Any incompatible backend must report
its limitation rather than silently falling back to the visible desktop.
Following the user's question about a Wayland equivalent, Sol is validating
Gamescope's headless backend with private Xwayland for Orrery, while rendered
Godot uses Xvfb. Assess actual desktop isolation and complete input/capture/
cleanup evidence for either route; a backend name alone is insufficient.
Multiple Luna agents are explicitly welcome for independent work. Evaluate
verified output and combined overhead; agent count alone is not a reason to
recommend reducing delegation. The monitor itself need not spawn workers.

For each meaningful review, record the UTC timestamp, task IDs and titles,
latest observed turn/report/commit, and a brief account of the work since the
last review. Assess only available evidence: iteration duration, repeated failed
runs, build/setup/test/cleanup cost, unnecessary rework, quality of findings and
completion of the intended behavior. Separate adoption of advice from an
observed benefit. Missing data means unknown, not no improvement.

Track each suggestion as pending, adopted but unmeasured, helped, did not help,
counterproductive, or superseded. Explain the comparison and confounders. Keep
effective advice, revise or withdraw ineffective advice, and distill general
lessons only when evidence supports transferring them to another task. Record
both the benefit and any added overhead. Keep raw logs elsewhere and link them.

Prefer one or two concrete improvements per review. State the observed cost,
the proposed change and how its effect can be checked. Keep reviews shorter
than the work they are intended to improve; no exhaustive history replay or
additional runtime experiments just to populate this log.

## Baseline — 2026-09-28 00:54 UTC

The initial review and feedback occurred in the preceding turns of task
`01a0e4a3-b70a-70d0-a3b7-fbd0b34194f1`. The target was active. Available task
history omitted its newest in-progress turn, so the review also inspected
current shared files and local reports. Do not mistake an empty retrieval for
an idle task.

Observed before the suggestions:

- Two original-game mana save/restart/load proofs passed, taking about 71
  seconds each, and a native Godot persistence slice was demonstrated.
- The focused save suite in
  `../TestResults/20260928T002309.779886Z-development.log.json` passed. Later
  changes require their own applicable validation.
- Repeated save-menu/name-confirmation failures consumed live runs. The task
  had already introduced screenshot-based menu recognition before feedback.
- A guarded crossbow quantity change reset after reload. This is a useful
  negative result; persistent inventory behavior remained unproven.
- A lever report in
  `../output/savegame-investigation/source-lever-proof-1790555647295407130/report.json`
  observed the unlock flag after a fresh reload. Its subsequent teleport was
  only at loading phase 5. Current code had already tightened the follow-up to
  require playable phase 9; that change also predates feedback.

Initial suggestions sent to the target; their subsequent effects are pending:

| ID | Suggestion | What would support a benefit |
| --- | --- | --- |
| P1 | Reuse established evidence; rerun affected cases after dependency changes and at the required validation checkpoint. | Fewer redundant live runs with the same required coverage and reliable final validation. |
| P2 | Use source registration, owner and script conditions to choose representative inventory cheat experiments. | More experiments produce the intended typed change or a useful falsifiable finding; fewer speculative failed launches. |
| P3 | Check menu recognition against retained screenshots offline, distinguish decoding/profile errors, and share the save/capture path. | Fewer menu-related live failures without accepting incomplete saves. |
| P4 | Use compile/parser/support preflight, focused checks, then the complete affected selection with safeguards. | Cheap failures caught before expensive runs, without omitted required checks. |
| P5 | Cache immutable baseline inspection and show compact semantic summaries with links to raw captures. | Less repeated decoding and oversized diagnostic output; evidence remains complete and accessible. |
| P6 | Update checkpoint notes and distinguish structural, source-runtime, Godot and cross-engine claims. | Later reviews can establish actual scope without rereading large raw histories or encountering stale completion statements. |

The user clarified after the review: cheats are the intended keyboard-facing
way to modify inventory. A representative selection is appropriate; one saved
experiment for every cheat is unnecessary. This clarification was sent to the
target. P2 must not be interpreted as avoiding cheats or replacing ordinary
input with direct memory changes.

## Subsequent reviews

Append concise dated entries here, updating the suggestion outcomes above or
linking to the entry that establishes them. Keep unproven hypotheses distinct
from demonstrated general lessons.

## Delegation evaluation requested by the user

After scheduling the recurring review, the user reported instructing Sol to
delegate to Luna. At the next check, verify what was actually delegated and
evaluate whether it improved token efficiency or harmed quality or overall
efficiency. This is pending; no benefit or harm is established yet.

- Identify the delegated work, actual model/agent identities, start/end points,
  expected result, and what useful work Sol completed alongside it.
- Use available task/agent usage evidence to count incremental tokens for Sol
  and Luna together, including handoff/context, supervision, review, integration
  and corrective work. Avoid double-counting cumulative usage. Separate input,
  cached input and output where reported. Account-wide usage percentages and
  message counts do not establish this task's token cost. State explicitly
  when reliable per-agent usage is unavailable; do not estimate numerical
  savings from model names or a smaller parent transcript.
- Compare useful verified results, elapsed time, duplicated investigation,
  integration effort, defects, failed checks and rework with the preceding
  workflow or suitably comparable subtasks. Explain complexity and scope
  differences; one before/after observation cannot establish causation.
- Treat token use, elapsed time and quality as separate outcomes. Faster
  parallel work may use more total tokens; lower token use may omit necessary
  work. Lower monetary cost is also distinct from token efficiency.
- Classify the result as helpful, mixed, harmful or inconclusive, with the
  supporting evidence. Recommend concrete changes to task boundaries, context,
  acceptance checks or review depth if justified. Preserve the user's chosen
  delegation arrangement while observing it; do not manufacture extra work
  solely to benchmark the agents.

## Review — 2026-09-28 01:25 UTC

Task: **Develop the save effort** (`01a0e4b6-3072-79d0-87a1-5bbf80f4f7cb`),
still active. The app snapshot again omitted the active turn; its unchanged
cursor is `720307bb-9c9c-4c51-8e15-39bdfc8cb983:1`. Current local artifacts and
only the relevant session logs supplied newer evidence. Other listed Astrolabe
tasks were idle; no additional task was woken for this review.

### Delegation: useful work, higher observed token traffic

Verified Sol as `gpt-6-sol` and seven children as `gpt-6-luna`: `offline_save_checks`,
`godot_world_gap`, `inventory_ui_investigation`, `magic_menu_trace`,
`save_validation`, `godot_save_flake`, and `source_save_flake`. The last two
had only just started at the review cutoff. Their work included the repeated
control-save case, native world-property persistence and its limits, inventory
menu research, focused validation and failure diagnosis. Sol continued source
inventory/mapping work, integration and documentation alongside them.

The comparison window is 00:58:42–01:25:03 UTC (about 26 minutes), beginning
with the first delegation. The preceding equal-duration window was Sol alone.
Available cumulative counters give:

| Counter | Preceding Sol-only window | Sol + Luna window |
| --- | ---: | ---: |
| Total input + output tokens | 21,267,679 | 37,148,219 |
| Cached input (included above) | 21,043,840 | 36,235,776 |
| Uncached input | 185,207 | 812,324 |
| Output (reasoning included) | 38,632 | 100,119 |

Sol's own second-window total fell to 14,223,834, but combined traffic rose
about 75%. Uncached input plus output rose from 223,839 to 912,443 (about
4.1 times). These are token counts, not monetary charges. Use cumulative
counter differences rather than summing repeated events; the child counters
were verified to begin at their first request. Automatic guardian-review usage
is outside this Sol/Luna subtotal. The interval expanded implemented coverage
and ran different work, so this is not a matched-task efficiency experiment or
proof that the same result would cost less with Sol alone.

Children started with roughly 33,000–45,000 input tokens. The parent made
38 coordination calls during the interval, including one unsuccessful spawn
at the concurrency limit. This establishes coordination/context traffic; it
does not establish that every such call was unnecessary. Exact agent IDs,
source log paths, counters and method limits are retained in
[`usage-summary.json`](../output/process-reviews/20260928T012503Z/usage-summary.json).

### Progress and quality

- The delegated two-run no-action control passed in 73.74 seconds and retained
  the same three changed record identities. Its independent initial fixtures
  add evidence rather than merely repeating the mana assertion.
- The source magic-inventory case mapped two selected-state bytes, observed
  game-written changes and fresh reload. Selecting and casting a newly enabled
  spell remains a separate acceptance gate. Manual/source research corrected
  confusion between inventory activation and current combat-spell selection.
- The stronger source lever proof reached playable `astrolabe` after reload.
  Native Godot persistence also gained the actual exported dummy's Boolean;
  converted lever gameplay remains explicitly unsupported.
- The combined focused suite
  [`20260928T011609.162030Z-development.log.json`](../TestResults/20260928T011609.162030Z-development.log.json)
  ran 33 tests: 30 passed and three failed. Failures were missing magic entries
  after cheat input, the lever-position guard, and the Godot reload stage.
  Build/discovery took 66.81 seconds and test execution 324.45 seconds. These
  differ from the earlier smaller passing selection; no causal attribution to
  Luna is justified. The failures remain acceptance work, not successful runs.

Assessment: **mixed; normalized token efficiency remains inconclusive**.
Delegation produced useful additional implementation and independent checks,
but has not demonstrated token savings or faster time to a fully accepted
result. The parent was already keeping scope limits visible, so that practice
cannot be credited entirely to delegation.

### Advice outcomes and next experiment

- P1: unmeasured. Repeated controls add required coverage; do not count them as
  redundant solely because they launch the game twice.
- P2: adopted with useful findings. Source ownership now maps inventory bits,
  and source/manual research has narrowed the next physical-input experiment.
  The effect on overall efficiency is not isolated.
- P3: partly adopted. Screenshot classification errors now propagate instead
  of becoming generic transitions. Offline captured-frame coverage and its
  effect on live failure rate are not established.
- P4: adopted, benefit unmeasured. Focused checks and the complete affected
  suite ran; the new failing gates show why the latter still matters.
- P5: partly adopted, benefit unmeasured. Semantic reports have improved, but
  the Godot failure still surfaced only trailing unrelated stack frames.
- P6: helped reviewability. Updated notes now distinguish successful source
  findings, native property persistence, and missing converted interactions.

Two suggestions sent to Sol at this review:

1. For the next suitable bounded assignment, try a self-contained brief with
   exact file/evidence pointers and acceptance criteria using no inherited
   conversation. Reuse a knowledgeable child for related follow-up, with one
   concise evidence-backed handoff. Preserve current workers and compare the
   result before further expanding the roster.
2. Have the existing diagnostic owners return the first relevant failure,
   exact stage, before/action/after values and log path, rather than the last
   eight lines of a noisy log. Distinguish input readiness, observation errors
   and actual persistence failures before changing the tests; retain the guards
   and final focused validation.

Next check: evaluate these two changes, the three failing cases and the completed
Luna work. Use the saved usage cutoff for incremental counters. Review overhead
was about four minutes; initial session output was too broad. Future checks
should reuse this compact snapshot and extract only new counter deltas, short
result summaries and the specific failing evidence.

## Review — 2026-09-28 01:55 UTC

Task: **Develop the save effort** (`01a0e4b6-3072-79d0-87a1-5bbf80f4f7cb`),
still active. The app snapshot still omits its active turn and has the same
cursor. Read narrowly extracted local session events and current artifacts;
empty retrieval is not inactivity. No other listed Astrolabe task had new
activity requiring advice. Usage cutoff is 01:55:03 UTC; code was inspected
shortly afterward. No game sessions or tests were launched by this review.

### Desktop isolation: useful progress, original-game proof pending

- The rendered Godot Xvfb smoke reports private display `:100`, an empty
  `WAYLAND_DISPLAY`, X11 session, and a captured 320×240 frame. Its owner also
  reports physical input and owned-process cleanup checks. Evidence:
  [`run.log`](../output/savegame-investigation/godot-xvfb-smoke/run.log) and
  [`frame.png`](../output/savegame-investigation/godot-xvfb-smoke/frame.png).
  Sol's review caught an earlier wrapper still inheriting `WAYLAND_DISPLAY`;
  it was corrected before this successful smoke. This is concrete review value.
- Orrery now has a shared Gamescope headless launcher, explicitly removes host
  `DISPLAY`/`WAYLAND_DISPLAY`, and connects with the private Xwayland authority.
  The MCP start schema only accepts the headless backend. A bridge-only launch
  and cleanup succeeded; complete Hype/MCP launch, input, nonblank capture and
  stop remained unverified at the cutoff. Source testing was paused for this
  requirement. Do not call that pause wasted time or the bridge smoke a full
  game pass. Inspect the completed proof at the next check.

### Delegation and usage

Verified `gpt-6-sol` parent and six `gpt-6-luna` children with new usage:
`magic_menu_trace`, `godot_save_flake`, `source_save_flake`, `xvfb_path`,
`xvfb_launcher`, and `godot_xvfb`. The last three were new independent
assignments and all explicitly used `fork_turns: none`; the earlier context
advice was adopted. Their initial inputs still contained about 33,000 tokens,
so omitting conversation inheritance does not remove the substantial base
context. Earlier children also sometimes started near that size.

Incremental cumulative-counter differences for 01:25:03–01:55:03 UTC:

| Counter | Sol + Luna, this 30-minute window |
| --- | ---: |
| Input | 34,238,810 |
| Cached input, included in input | 33,688,320 |
| Uncached input | 550,490 |
| Output, including reasoning | 100,675 |
| Total input + output | 34,339,485 |

Sol accounts for 14,570,182 of the total. These totals include the parent and
children's recorded handoff, supervision, review and corrective work, but
exclude automatic guardian reviews. Details and next-review cumulative
baselines are in
[`usage-summary.json`](../output/process-reviews/20260928T015503Z/usage-summary.json).
Cached input is a subset, not an additional charge; reasoning is likewise
included in output. These are token counts, not price estimates.

Observed traffic fell from about 1.410 million to 1.145 million tokens per
minute versus the previous delegation interval; uncached input plus output
fell from about 34,628 to 21,706 per minute. **This does not establish token
savings from the advice or delegation.** The work changed to user-requested
display isolation, with different waiting, implementation and testing demands.
The parent made 51 coordination calls (including 19 waits); their cost is
included, but the count alone does not establish unnecessary coordination.

### Results, quality and targeted feedback

- Source diagnostics now retain phase, inventory, mana and input response
  before the magic assertion. A serial source magic-inventory proof passed in
  72.68 seconds, with typed before/after observations at
  `output/savegame-investigation/source-magic-inventory-proof-1790559253551054978/first/cheat-observation.json`.
  This helps distinguish an intermittent input/readiness failure from save
  persistence. The stricter lever-position case remains unresolved; its run
  was stopped for the new isolation requirement.
- Manual/source research narrowed spell selection to the equipped spell and
  bounded Delete-key cycling. This is a useful P2 outcome, but casting and its
  persistence still need their own acceptance evidence.
- The Godot diagnostic owner reports two targeted persistence passes after
  allowing known unrelated scene diagnostics. Review found a concrete gap in
  [`save_persistence.rs`](../crates/godot-harness/tests/save_persistence.rs):
  unexpected `ERROR:` lines do not fail a marker-bearing exit-0 result, and
  `SCRIPT ERROR:` lines are ignored by the exit-1 allowlist. A success marker
  therefore does not guarantee the stated rejection of unexpected errors.
  This is a reachable false-pass condition, not evidence that a particular
  captured save was corrupt.
- Sent one actionable suggestion to the existing owner through Sol: reject
  unexpected engine/script errors regardless of exit code, constrain tolerated
  diagnostics to evidenced unrelated cases, and check the identified negative
  combinations before another expensive suite. Preserve the save assertions
  and required safeguards; do not broaden into unrelated AI repair.
- The latest complete affected selection is still the prior 30/33 result.
  Targeted passes and launcher preflight do not replace that validation.

Assessment: **mixed useful output; overall token/time efficiency remains
inconclusive**. Parallel owners contributed diagnostics, selection research
and isolation work. Supervision caught a real environment leak, while this
review found an error-classification gap in a delegated test change. That
supports focused integration review, not a general conclusion that Luna or
multiple workers harm quality.

P1 remains unmeasured. P2 has useful selection evidence, without measured
overall savings. P3's effect on live failure rates remains unmeasured. P4's
preflight approach was used for isolation, with final acceptance still pending.
P5 improved diagnostic specificity; its time/token benefit is unmeasured. P6
continues to make scope reviewable, but runtime notes should reflect the final
verified backend when available. Keep the concise no-history handoff practice;
do not claim its benefit from this changed workload alone.

Next review: inspect the result-classifier correction, full private-display
Orrery/MCP evidence and cleanup, lever outcome and required focused selection.
Use this cutoff for usage deltas and avoid re-reading completed investigations.
Review overhead was approximately five minutes, including a context compaction;
no independent workers or validation runs were created. Transferable lesson:
review test acceptance logic as carefully as production changes, especially
when an apparent fix tolerates existing errors.

## User-directed follow-up — 2026-09-28 02:12 UTC

The user reports that the headless game remains audible and explicitly requests
a separate Sol agent to add audio to Orrery's full-recording mode. Dispatched
`/root/audio_recording` with model `gpt-6-sol` and no inherited conversation,
under the monitoring task. This is user-requested implementation work, not an
extra review worker or delegation benchmark. Its usage should be accounted for
separately from the save task's Sol/Luna subtotal.

The audio owner is to implement game-scoped capture, session timing and coverage,
full replay/recapture support, error handling and owned-resource cleanup, while
preserving compact input-only recordings. Automated runs must be inaudible to
the user without removing the samples from the recording. No microphone or
unrelated desktop audio capture, global muting or default-device changes.

Informed **Develop the save effort** of the audible output and requested
game-scoped routing for current/next owned sessions. That task retains display
launcher/save ownership; the audio agent must agree minimal shared-file hooks
and an exclusive live-game validation window before integration. No competing
original-game sessions. Review the audio agent's evidence when available;
dispatch alone establishes neither implementation nor silent operation.

## Review — 2026-09-28 02:25 UTC

**Develop the save effort** remains active. The app snapshot again returned
the unchanged cursor and no active-turn text; narrowly extracted session events
and current evidence supplied progress. No other listed Astrolabe task had new
work needing feedback. The separately requested `/root/audio_recording` agent
is active; no new review workers or original-game sessions were started here.

### Verified progress and advice outcomes

- The full authenticated headless MCP smoke now passed: physical menu input
  loaded the pinned fixture into `manoir`, phase 9/action 47; independent title,
  save-list and gameplay screenshots were retained; MCP stop left no owned
  Hype/Gamescope process. Parent `DISPLAY` and `WAYLAND_DISPLAY` were absent.
  The captured interval lasted 164.904 seconds within a 180-second limit. See
  [`summary.json`](../output/savegame-investigation/headless-mcp-smoke/summary.json).
  This proves the display/input/capture/stop path, not the pending save or
  casting behavior. Unrelated Wine applications were correctly left alone.
- The first MCP attempt failed before game launch because a general isolated
  save-manifest helper did not satisfy MCP's pinned-archive contract. The owner
  corrected only the isolated fixture and verified its four members. P4's
  fail-before-launch guard worked; fixture preparation still caused rework.
  Reuse this corrected fixture preparation for subsequent MCP checks.
- The prior review's test-classifier advice was adopted and helped close the
  identified false-pass paths: unknown engine errors are rejected even on exit
  0, script errors are rejected, and the expected malformed-JSON diagnostic is
  restricted to the corrupt-slot stage. Five classifier checks and the
  three-process Godot proof passed; the latter took 40.32 seconds. Evidence:
  [`godot-save-persistence`](../output/savegame-investigation/save-classifier-review-final/godot-save-persistence/).
  Known scene diagnostics are still allowed explicitly; this does not repair
  those unrelated runtime issues or establish the original flake's cause.
- A separate Luna contract audit found that extra actor/state fields could be
  silently ignored. Sol tightened the schema and commissioned focused
  verification. Review also caught the prepared magic experiment cycling away
  from Ice before casting; it is being changed to stop on the typed selection
  and require gameplay evidence. These are concrete quality contributions
  from independent review and supervision, with final behavior tests pending.
- Static formatting/lint/build checks passed before subsequent concurrent
  edits. A fixture-test compile was interrupted by an in-progress audio edit;
  audio integration later compiled. Treat checks as applying to their observed
  tree, not automatically to later changes. The latest complete save selection
  remains 30/33; lever and spell-use acceptance are still pending.

### Audio coordination and user impact

The user heard audio during the otherwise isolated MCP run. Display isolation
alone is insufficient for nonintrusive tests. Sol correctly paused subsequent
source runs. The audio owner reports a passing isolated synthetic tone proof:
private PipeWire/Pulse routing, non-silent FLAC capture and temporary resource
cleanup. This is preflight, not yet a Hype audio proof at the cutoff.

The audio owner was waiting for a live-test window while the save owner was
waiting for private audio. This review resolved the handoff explicitly: the
save owner confirmed no active Hype/Gamescope, keeps its prepared source cases
paused, and granted the audio owner one short live validation. The audio owner
must return routing/capture/cleanup evidence and release the window before
source cases resume. Expected benefit: remove cross-task waiting without
overlapping game sessions; actual live outcome remains pending.

### Incremental delegation usage

Verified the parent as `gpt-6-sol`; six Luna children had new usage:
`xvfb_launcher`, `godot_xvfb`, `save_static_gate`, `save_contract_audit`,
`save_schema_gate`, and `magic_usability`. Four were new. Those four used full
history inheritance, so the earlier no-history handoff practice was not
maintained consistently; their initial inputs were about 37,900–38,200 tokens.
This alone does not show that their useful audits cost more overall.

For 01:55:03–02:25:03 UTC, Sol + Luna cumulative-counter deltas are:

| Counter | Tokens |
| --- | ---: |
| Input | 39,154,016 |
| Cached input, included above | 38,704,768 |
| Uncached input | 449,248 |
| Output, including reasoning | 79,544 |
| Total input + output | 39,233,560 |

The parent accounts for 13,793,626; the headless launcher/MCP owner accounts
for 18,794,402. Combined traffic rose 14.3% over the preceding equal interval,
while uncached input plus output fell 18.8% to 528,792. Thus the earlier lower
traffic rate did not continue across both measures. Different work, runtime
waits, review and integration prevent causal efficiency conclusions. The
parent's 66 coordination calls, including 37 waits, are included in its usage;
not every wait represents avoidable overhead.

The separately requested Sol audio implementation used 6,438,927 total tokens
within this interval: 6,420,168 input (6,323,200 cached) and 18,759 output. It is
reported separately because it is a newly requested feature, not added to the
save task's delegation comparison. Guardian-review usage remains excluded.
Full counters, paths and next-cutoff baselines:
[`usage-summary.json`](../output/process-reviews/20260928T022503Z/usage-summary.json).
Forked logs contain copied parent metadata/model contexts; identities use the
first session metadata and actual model context at token events. New child
counters start at their own first request; copied context was not counted as
an additional parent run.

Assessment: **helpful quality findings and completed isolation work; overall
token/time efficiency remains inconclusive**. Keep focused independent review
and explicit integration ownership. P1/P3 efficiency effects remain unmeasured;
P2 now narrows the casting experiment; P5/P6 make failure and scope reporting
more reviewable. Do not substitute these partial results for final validation.

Next check: audio live proof and resource cleanup, release of the game window,
corrected schema validation, spell/lever outcomes and the complete affected
selection. Review overhead was about four minutes including the necessary
cross-task handoff; no tests were rerun. Transferable lesson: when one task
waits for infrastructure and its owner waits for a test slot, explicitly assign
and release that slot rather than adding more workers or repeating status polls.

### Audio handoff update — 2026-09-28 02:32 UTC

The audio owner's coordinated live full-recording check completed. Inspected
[`report.json`](../output/audio-recording/live-full-20260927-b/report.json)
and its `audio_stream_end` event: completed recording, cleanup true, playable
FLAC metadata (48 kHz stereo, 30.582 seconds), detected signal at -16.7 dB,
and explicit launch-relative timing/uncertainty. The owner reports its private
servers, sink, encoder and owned game processes were removed, with the desktop
audio default unchanged. This proves a live recording path beyond the earlier
synthetic tone; sample-accurate engine/video synchronization is not established.

The source window was explicitly released. **Develop the save effort** has now
assigned one Luna to the targeted magic save/reload/cast test; no other Hype
launch by the audio owner is authorized concurrently. Audio documentation and
final independent checks are finishing. Review spotted a stale “audio absent”
documentation sentence and an old SDL display label in recording provenance;
asked the audio owner to correct maintained metadata/docs without rerunning a
game or rewriting the sealed evidence.

The save owner subsequently reports its single private-display/private-audio
magic save/reload test passed in 73.124 seconds and cleaned up all owned
Hype/Gamescope/audio processes. Ice54 and Lightning74 selected flags persisted;
three Delete taps left the active combat spell at Fire61, so no cast was
attempted. Treat this as a reported persistence pass under the new isolation,
not spell usability. The source game is stopped and the owner has assigned the
virtual graphics window to one Godot Xvfb save check. Inspect its retained
report at the next scheduled review rather than rerunning the source case here.

The audio implementation agent has now finished. Its
[evidence note](orrery-audio-recording-2026-09-27.md) records the game-scoped
private routing, full-record/replay FLAC capture, compact input-only behavior,
resource ownership and timing limits. Formatting, Orrery lint, release build,
29 library tests (three expected exclusions) and two private-audio synthetic
tests passed. The final monitor-attachment startup guard was verified
synthetically after the live recording; the separate worker route and live
input-only recapture were not independently exercised in that window.

The save owner's focused-list gate then found the new audio/isolation and
Godot classifier tests missing from `tools/test-manifest.json`. The save owner
has explicit ownership of adding those entries and the focused selection;
confirmed that the completed audio agent will not edit that manifest or
`tools/suites.py`. The implementation's focused checks do not count as a passed
integrated save selection. Next review should verify this classification gate
and its required safeguards, without restarting the completed audio agent.

## Review — 2026-09-28 02:55 UTC

**Develop the save effort** remains active. Compact retrieval still omits the
active turn; the cursor is unchanged. Used new local events, retained reports
and narrow code reads. No other listed Astrolabe task had new activity needing
intervention. The audio agent is complete and was not restarted. No tests or
game sessions were launched for this review.

### Completed behavior and corrected validation

- Source Ice usability after a fresh save reload is now demonstrated. The
  manual/screenshot investigation identified a sheathed weapon as the missing
  condition. Drawing with X and one Delete selected Ice 54; Return cast it,
  with typed mana falling 250 to 200 and an ice effect reported in the frame.
  Inspected the typed
  [`spell-cast-observation.json`](../output/savegame-investigation/targeted-magic-cast-20260927/source-magic-inventory-proof-1790563796143797301/reload/spell-cast-observation.json).
  The corrected run passed in 83.41 seconds using the private display/audio
  route, with owned-process cleanup reported. This establishes a representative
  cheat-enabled spell's save/reload/use behavior; other unknown reference bytes
  and broader inventory behavior remain outside that evidence.
- The preceding 73.124-second case proved persistence but no selection or cast.
  The subsequent weapon-draw probe proved selection, then failed because its
  cast used the unsupported key name `Enter`. The corrected `Return` run added
  another complete live cycle. The first rerun tested a supported hypothesis;
  the key-name failure was an avoidable interface error. Preserve both findings
  rather than describing every failed attempt as speculative or redundant.
- The Godot write-stage success marker was followed by SIGABRT during dummy
  renderer shutdown. The strengthened classifier correctly kept that result a
  failure. Orderly scene release alone still aborted in a 45.61-second attempt.
  Running the maintained test through authenticated Xvfb, with host Wayland
  removed, passed all three stages in 50.47 seconds with zero exits and the
  stronger schema rejection checks. The five classifier tests also passed.
  [`summary.md`](../output/savegame-investigation/godot-exit-crash-xvfb/summary.md)
  records these outcomes and cleanup. This is evidence for using the rendered
  backend for this test, not a general claim that Xvfb is faster or that all
  Godot headless tests are unsafe. One reproduction's temporary log was again
  lost, so durable failure evidence remains inconsistently applied.
- The exact test-inventory guard rejected nine new, unclassified tests twice.
  The first list attempt spent 68.58 seconds building/discovering (68.75 total);
  the second spent 2.34 seconds there (2.50 total). Neither ran behavior tests.
  After explicit ownership and classification, the list passed in 0.65 seconds,
  selecting 42 tests including 20 mandatory safeguards. Its `not_run` status is
  expected for listing and is not a suite pass. Evidence:
  [`025516 list report`](../TestResults/20260928T025516.886930Z-development.log.json).
  Sol reports starting the full focused selection immediately afterward; final
  integrated acceptance and the lever result remain pending.

### Advice and process outcomes

The exclusive audio-to-source handoff helped: the live recording completed,
released its window, and the subsequent source/Godot checks ran with explicit
ownership. No conflicting original-game launch was observed. Preserve that
handoff practice without attributing every successful test to coordination.

P2 helped produce a falsifiable precondition and a completed spell-use result;
its overall token savings remain unmeasured. P4 kept schema/runtime/inventory
failures visible before final acceptance, but key syntax was checked too late.
P5/P6 improve semantic reporting and scope distinctions; the lost reproduction
log and stale Godot timing text in the checkpoint show remaining documentation
and evidence-retention gaps. P1/P3's overall efficiency effects remain unmeasured.

Sent two concrete suggestions for subsequent edits, explicitly preserving the
currently running validation tree:

- **P7, pending:** validate all planned key names through the real input parser,
  or construct them with its typed keys, before launching Wine. Expected benefit:
  reject an unsupported name locally instead of after an approximately
  80-second save/reload cycle. No benchmark game run requested.
- **P8, pending as a delivery practice:** include exact manifest entries, or an
  explicit handoff to their owner, with delegated test changes. The current
  entries are already fixed; the next assignment should avoid repeating the
  same inventory failure. Keep the fail-closed inventory guard and safeguards.

### Incremental delegation usage

Verified the `gpt-6-sol` parent and four `gpt-6-luna` children with new usage:
`save_schema_gate`, `magic_usability`, `godot_exit_crash`, and
`magic_control_trace`. The last two children were new full-history assignments.
Agent count is not itself the efficiency measure.

For 02:25:03–02:55:03 UTC, combined save-task usage was 24,643,220 input,
including 24,329,472 cached input; 313,748 uncached input; and 57,408 output
(reasoning included). Total input + output: **24,700,628**. The parent accounts
for 16,676,280. Combined traffic fell 37.0% from the preceding equal interval;
uncached input plus output was 371,156. Different live-test waits, completed
infrastructure and work mix prevent attributing this decline to delegation or
the context advice. The parent made 49 coordination calls, including 35 waits;
these are included, without assuming all waits were unnecessary.

The separate Sol audio feature used another 11,940,505 tokens this interval
(11,923,939 input, 11,873,792 cached, 16,566 output). Its completed child total
is 18,379,432 (18,344,107 input, 18,196,992 cached, 35,325 output). This excludes
cross-task supervision, which remains in the respective parent usage, so it is
not an all-in feature cost. The audio child is kept outside the save-delegation
subtotal. Guardian-review usage remains excluded; counters are not prices.
Source paths, identities and next-cutoff baselines:
[`usage-summary.json`](../output/process-reviews/20260928T025503Z/usage-summary.json).

Assessment: **useful delegated diagnosis and acceptance evidence; overall
token/time advantage remains inconclusive**. The Godot owner corrected a real
backend failure without weakening the exit checks, and source research enabled
a successful cast. Integration still incurred an unsupported-key retry and
test-inventory omissions. Retain bounded ownership and review, and measure P7/P8
only on later naturally occurring work.

Next review: the 42-test focused selection, lever outcome, any new classifier
or integration failures, and adoption of P7/P8 if another edit occurs. Review
overhead was approximately four minutes; no new runtime experiments or workers.

## Review — 2026-09-28 03:25 UTC

**Develop the save effort** remains active. Compact retrieval still omits its
active-turn text, with the same cursor; recent local events and reports supply
the evidence. No other listed Astrolabe task had relevant new activity. The
audio agent remains completed. No monitor workers or runtime tests were started.

### Integrated results and remaining failures

| Full focused run | Result | Build/discovery | Test execution | Total |
| --- | --- | ---: | ---: | ---: |
| [025620 report](../TestResults/20260928T025620.551140Z-development.log.json) | 41/42 passed; lever position guard failed | 0.58 s | 413.59 s | 423.35 s |
| [030653 report](../TestResults/20260928T030653.879295Z-development.log.json) | 40/42 passed; mana cleanup and lever save approach failed | 22.52 s | 421.92 s | 452.96 s |

Both runs kept the implementation unchanged and completed supervisor cleanup
without leaked children. The native Godot Xvfb test, audio/display checks and
magic save/reload/cast case passed within the integrated selection. A pass count
of 40 versus 41 is not a quality trend by itself: correcting the lever guard
allowed that case to reach a later save-menu failure, while a separate process
exit race affected the mana case. Neither full run satisfies acceptance.

Sol now accepts two explicitly observed lever placements with the same actor
identity/hierarchy and restores the observed starting placement. Lever activation
sets the world flag, but the later physical approach misses the save lectern.
It is comparing retained successful/failed poses, about 0.35 units apart, and
running the lever case alone before another full selection. The owner reports
the disappearing-process cleanup race fixed and a targeted mana check passed;
the final integrated check of that change remains pending.

The latest [032422 list report](../TestResults/20260928T032422.256278Z-development.log.json)
spent 26.10 seconds building/discovering, then rejected the results because the
implementation changed during validation. All 42 cases are correctly `not_run`;
temporary cleanup succeeded. This is additional coordination cost, not a test
failure in one of the selected behaviors.

### Advice adoption and new feedback

- P7 is adopted in the magic test: all its planned key strings now pass through
  `Desktop::validate_chord` before configuration, staging or game launch. No new
  invalid key was caught this interval, so its prevention benefit is unmeasured.
- P8's exact classifications remained valid through both integrated runs. No
  new missing-test inventory failure appeared. Adoption by future delegated
  test owners remains unmeasured; do not credit an absence of new tests as proof.
- P1/P4: two full selections consumed about 14.6 minutes; the task has now moved
  back to isolated lever diagnosis. Retain that sequence and the eventual full
  gate. The second full run also found a cleanup race, so not all repeated
  coverage was wasted. P2/P5's typed state and saved pose evidence are narrowing
  the lectern approach; its successful outcome is still pending.

Sent one concrete coordination change: extend the stable-tree rule already used
for full runs to delegated list/static checks. Evidence reviewers can read while
implementation changes continue; signal a ready-to-validate checkpoint and hold
covered edits until the validator finishes. Expected benefit is avoiding another
discarded build result without dropping safeguards. This is pending. Supported
the already adopted isolated-lever diagnosis rather than prescribing new work.

### Usage and delegation outcome

For 02:55:03–03:25:03 UTC, verified `gpt-6-sol` parent plus new usage from
`gpt-6-luna` children `save_evidence_review` and `save_static_validation`:
16,705,572 input, including 16,590,464 cached; 115,108 uncached input; 23,730
output, including reasoning; **16,729,302 total input + output**. Sol accounts
for 15,963,757. The children began with about 45,400 input tokens despite only
three turns being inherited; narrower inheritance alone does not guarantee a
small prompt. A third `save_docs_audit` child was dispatched but had no usage
event by this cutoff, so no output or model use is credited yet. The completed
audio child added zero tokens in this interval.

Combined traffic is lower than the prior 24,700,628, but most of this interval
was Sol supervising serial tests, with new delegates only near its end. Their
deliveries are not yet complete. **This interval's delegation outcome is
inconclusive**, and does not establish token savings from using fewer/more
agents or shorter history. Previously demonstrated review benefits remain valid;
new integration costs and eventual corrective work must stay in later totals.
Counter differences, identities and next baselines:
[`usage-summary.json`](../output/process-reviews/20260928T032503Z/usage-summary.json).
Cached input is included in input, and guardian-review usage remains excluded.

Next check: lever save approach, integrated cleanup regression, final full
selection, new Luna deliveries and the stable-tree handoff. Review overhead was
about three minutes; no tests were rerun. Preserve explicit failure stages and
clean teardown evidence rather than interpreting partial pass totals as completion.

## Schedule ended at the user's request

The user requested ending the schedule after **Develop the save effort** reported
completion, and questioned whether the save attributes were fully mapped. Paused
`review-development-process`; no further periodic review is scheduled unless
the user resumes it. Do not restart the completed task implicitly.

The task's 03:44:53 UTC completion message explicitly claimed completion to the
plan's stated scope. Verified the final
[focused report](../TestResults/20260928T033518.226177Z-development.log.json):
42/42 passed in 483.361 seconds, with unchanged source and complete temporary
cleanup. This closes that selected validation milestone, not semantic coverage
of the whole save format.

The [results catalogue](savegame-results-2026-09-27.md#structure-catalogue-version-1)
still has only five verified gameplay record mappings: current and maximum mana,
Ice and Lightning inventory selected-state flags, and the manor dummy unlock
Boolean. It parses all four supplied save files and accounts structurally for
1,271 player records plus 263 named-level records; those counts are not 1,534
distinct understood attributes. Health/armor, broader inventory and progression,
other actor/world variables, array/reference meanings, metadata and complete
load precedence remain unresolved. Original-save import is deferred, and native
Godot persistence does not establish equivalent converted lever gameplay.

The handoff was framed as an investigation and first persistence slice with
representative proofs and named unknowns, not exhaustive semantic mapping.
Future mapping work needs an explicit completion contract for a chosen build
and save corpus: every encountered record/registration classified by owner,
meaning and evidence or an individually documented unresolved reason. Structural
parsing and a passing focused suite must not be presented as that completion.

## Hourly review resumed — 2026-09-28 04:17 UTC

At the user's explicit request, wrote and delivered
[the comprehensive semantic-map handoff](savegame-semantic-map-handoff-20260928.md)
to **Develop the save effort** (`01a0e4b6-3072-79d0-87a1-5bbf80f4f7cb`). Sol
acknowledged reading it and starting the corpus manifest and registration-call
index, then ownership joins. Its two existing Luna audits remain read-only
during integration. No new implementation workers were spawned by the monitor.

The new objective goes beyond the first slice: account for the supplied save,
resolve owners/types and actual meanings, then expand across the available
pinned English-build registration/content scope. A catalogue of unresolved
entries is a checkpoint, not completion. The first roughly two-hour checkpoint
should quantify coverage and unresolved families and revise the estimate;
implementation continues without a new approval gate. Progress belongs in
`notes/savegame-semantic-map-progress-20260928.md`.

Reactivated the existing `review-development-process` heartbeat at one-hour
intervals in this planning task. Notify only on meaningful, actionable changes;
verify actual semantic completion before ending the schedule. Keep reviews
focused on this mapping effort, not unrelated follow-on project work.

New-phase counter baseline:
[`usage-summary.json`](../output/process-reviews/semantic-map-start-20260928/usage-summary.json),
cutoff `2026-09-28T04:17:01Z`, with the parent and 20 existing child histories.
These are starting counters, not costs of the new phase. Use deltas and exclude
the completed audio child from the save-task subtotal. Latest app cursor is
`720307bb-9c9c-4c51-8e15-39bdfc8cb983:2`; active-turn text was still unavailable.
Sol's explicit acknowledgment supplied the first-step evidence.

Continue tracking P7 input preflight, P8 exact test-inventory handoffs and the
stable-tree validation suggestion, alongside coverage, integration overhead
and useful delegation results. Preserve private display/audio routing and
exclusive live ownership. Do not impose a live run for every source-proven
mapping or rerun the prior 42-case baseline merely to establish this phase.

### Incoming mapping checkpoint — 2026-09-28 05:03 UTC

Sol sent an early checkpoint; inspected the
[progress note](savegame-semantic-map-progress-20260928.md) and generated
[`coverage.json`](../output/savegame-semantic-map/coverage.json). The supplied
slot reconciles 1,534 file occurrences to 1,271 contextual identities. Of those,
523 have source owners/types; 123 have complete source-verified meanings and
five retain runtime corroboration, leaving 1,143 unresolved. The 177 partially
explained records correctly remain in that unresolved count. Only two of 222
array payloads have complete content meanings. These are coverage counts, not
fractions of development time remaining.

The manifest includes 37 level packages and 20 retained written-save roots.
Discovery covered all 37 levels and deduplicates repeated Fix descriptors;
it does not claim every descriptor registers in reachable gameplay. The index
now identifies nine wrappers and 92 literal calls, with indirect dispatch gaps
explicit. Family-based joins yielded many mappings without individual live
experiments, providing concrete evidence for the chosen bulk-mapping approach.

Verified the earlier stable checkpoint's
[43-test report](../TestResults/20260928T044912.510196Z-development.log.json):
43/43 passed in 424.445 seconds with unchanged source and complete temporary
cleanup. Later Global/StandardGame joins have targeted checks; final static
checks were still pending in this checkpoint. Do not transfer the earlier full
pass to those later edits automatically.

Sol now estimates several focused days for strong semantic coverage of the
whole supplied fixture, with broader progression coverage taking longer. The
earlier 8–16-hour estimate was optimistic for that goal. Remaining Global
arrays, dynamic blocks, partially understood actor bitfields and load precedence
have different research costs from the high-yield inventory family; extrapolate
neither the initial throughput nor the unresolved count linearly.

Next steps already target those gaps, so no duplicate process advice or new
workers were sent. This was a brief incoming-checkpoint review, not a new usage
interval: retain the 04:17 baseline for the scheduled hourly accounting. The
user has been informed of this coverage and changed estimate; do not repeat it
at the next heartbeat without a meaningful additional result.

### Hourly semantic mapping review — 2026-09-28 05:21 UTC

**Develop the save effort** remains active. The app snapshot still supplies no
active-turn text and retains cursor `720307bb-9c9c-4c51-8e15-39bdfc8cb983:2`;
this is a retrieval limit, not inactivity. Used recent local session events,
the progress note and existing generated reports. No monitor workers, game
sessions or tests were started.

Since the incoming 05:03 checkpoint, four map-era availability words became
source-verified: **132 complete meanings, 1,139 unresolved**, including 177
partially explained records, out of 1,271 supplied-save identities. Owned/type
coverage remains 523. The additional 170 map-group words have structural
grouping but no complete meaning. The existing corpus comparison decoded all
20 retained roots with unchanged identity sets and found three values varying
between no-action controls. This is useful evidence against assigning every
before/after difference to a cheat or interaction; it does not identify those
three meanings or establish causation for other correlated changes.

The bounded [registry capture](../output/savegame-semantic-map/registry-live-final/summary.json)
completed in 16.717 seconds wall time, with 2.023 seconds reported worker
cleanup. It traversed all 2,180 declared nodes, mapped 2,176, and matched every
supplied identity, including the 261 without static allocations. The 645
additional live identities and four unmapped addresses remain discovery work,
not explained attributes. The post-cutoff tightened-reader rerun also passed
in about 16.6 seconds and retained the same 1,916 mapped identities; see
[validated evidence](../output/savegame-semantic-map/registry-live-final-validated).
This repeat followed reader changes, so it was warranted validation.

Luna deliveries contributed the inventory trace, three previously missed
registration aliases, canonical Fix Global ownership correction, four epoch
meanings, 37-level refresh, retained-save comparison and the bounded registry
reader/capture. Sol reviewed and integrated them. This is useful parallel
output, with quality benefits from independent ownership/alias review. Some
source research overlapped: the stat audit and subsequent Global-family audit
both traced the same magic-selection low byte without resolving its high byte.
The runtime pilot also stopped for missing hash context, then hit a failed MCP
Gamescope launch before the later working inspection command. Multiple Luna
workers themselves are not the problem; narrow questions and reusable findings
remain the appropriate boundary. Overall delegation assessment: **mixed
process results, useful quality/output, net token and elapsed-time benefit
inconclusive**. There is no matched Sol-only run for this much broader scope.

New-phase [usage delta and next baseline](../output/process-reviews/20260928T052103Z/usage-summary.json)
covers 04:17:01–05:21:03.819 UTC. First session metadata identifies each child;
models are those active at usage events, not copied parent contexts. Parent
supervision/integration and child correction are included; completed audio work
is excluded. No nested delegated workers were observed in this interval.

| Actual model / scope | Input | Cached subset of input | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 53,078,243 | 52,603,648 | 125,873 | 53,204,116 |
| 12 active Luna workers | 31,391,643 | 30,413,312 | 128,834 | 31,520,477 |
| Combined incremental usage | 84,469,886 | 83,016,960 | 254,707 | 84,724,593 |

Uncached input is 1,452,926. Reasoning output is 105,089 and already included
in output. These are measured token counts, not cost estimates or demonstrated
savings. Existing cumulative usage was subtracted once; new children start at
zero. Compact delivery evidence is retained in
[recent-agent-evidence.json](../output/process-reviews/20260928T052103Z/recent-agent-evidence.json).

Validation/advice assessment: the earlier stable 43/43 selection remains the
full-pass evidence, not a pass for later edits. Its default-sandbox failure cost
12.388 seconds before the capability-adjusted 424.445-second successful run;
logs were retained and no desktop fallback occurred. The post-cutoff test-list
report at 05:22 took 53.005 seconds including 52.921 seconds build/discovery,
selected 46 tests, and reports `not_run`, unchanged source and complete cleanup.
Do not count this as 46 passing tests. The three new registry tests have exact
manifest entries: P8 adoption continues, though net time savings remain
unmeasured. P7 input validation has no new avoided-error evidence. Stable-tree
checks were adopted in both inspected reports, with no discarded validation
observed in this interval. P5/P6 evidence is useful but the progress note has
stale interpretation and runtime-status sentences needing reconciliation.

Private runtime routing is supported by the actual headless Gamescope/Xwayland
log and the current inspection launch path: host display variables are removed,
the worker validates its private display, and launch/input/capture use that
display. Its supervisor creates and applies private audio routing; the capture
and validation report owned cleanup. The failed pilot checked no game remained
before the successful owner ran. Rendered Godot's isolated Xvfb route was
already covered by the earlier complete selection; no new Godot run was needed
for monitoring. These observations do not constitute continuous monitoring of
all desktop/audio activity.

Sent two concrete refinements to Sol at 05:25 UTC; **adoption and benefit are
pending**:

- P9: include the proven runtime command, fixture/build hashes, capability
  requirements, exclusive ownership and cleanup evidence in runtime briefs.
  Reuse the working inspection route unless an MCP change itself needs testing.
  Expected effect: avoid the observed missing-context stop and incompatible
  setup retry while retaining safeguards.
- P10: give research briefs established/retired hypotheses and one unresolved
  question. Reconcile the old inventory interpretation of `1:10` with newer
  map-star evidence and the obsolete “no live experiment” sentence. Expected
  effect: reduce duplicate source walks and experiments driven by stale notes.

Review overhead was about five minutes. The
[partial monitor usage](../output/process-reviews/20260928T052103Z/monitor-usage.json)
records 620,646 tokens so far (616,256 input, including 546,944 cached; 4,390
output), excluding final log-writing and response. Two overly broad nested
report extracts added unnecessary output; future reviews should select scalar
fields before serialization. Next check should use this new cumulative
baseline, follow semantic gains in the unresolved families and discovery gaps,
and assess P9/P10 adoption separately from benefit. The hourly schedule remains
active because registration coverage is not comprehensive semantic completion.

### Incoming dialogue-family checkpoint — 2026-09-28

Sol reported a further semantic result after the hourly review. Checked the
updated [coverage](../output/savegame-semantic-map/coverage.json), catalogue
family rule and representative entries against the registration loop in
`reference/decompilation/functions/0056ba50_DLG_fn_vLoadDialog.c:46`.
The loop registers a list reference, its byte count and each member reference.
The existing live snapshot supplies the corresponding order and pointer joins.
This supports 67 dialogue-list member counts as complete structural meanings;
their authored dialogue identities and member contents remain unresolved.

Coverage now reports **199 complete meanings** (194 source-verified plus five
runtime-corroborated), **1,072 unknown**, and **724 owned/typed identities**.
All 201 dialogue records have ownership evidence; the other 134 are references
whose roles are partially known, correctly retained among the unknowns. Thus
311 unknown records now have partial explanations. The 67 repeated count fields
are contextual records explained by one family rule, not 67 independently
discovered gameplay concepts. Array-content coverage remains two of 222.

P10's documentation reconciliation is adopted: the old inventory hypothesis
for `1:10` is retired, the magic low-byte fact is retained, the next task targets
its unresolved high byte, and live validation is accurately reported. Adoption
in subsequent agent briefs and reduced duplicate work remain unmeasured. P9
has no new handoff evidence. Do not attribute this dialogue discovery to advice
sent only shortly beforehand. The existing registry/corpus results were already
reviewed; no duplicate runs or repeated feedback were requested.

This brief checkpoint does not reset token accounting: the next hourly review
still subtracts the saved 05:21:03.819 UTC baseline. Goal and hourly schedule
remain active. User notification is warranted by 67 additional explained
records, with the family-rule and unresolved-content limits preserved.

### Requested sector/reference review — 2026-09-28 05:47 UTC

Sol requested review of another checkpoint for inflated claims and a better
next discriminator. Read the changed progress, coverage, sector family rule,
join/test, cited engine routines and the compact
[11-reference investigation](../output/savegame-semantic-map/level6-reference-investigation.md).
No live session or validation was run. Reconciled **252 complete meanings**
(247 source-verified plus five runtime-corroborated), **1,019 unknown** (311
partial), and **837 owned/typed**. The 53 newly explained sector references and
60 additional map-star ownership joins affect separate coverage tiers.

The actor-to-list-to-rooted-sector join supports the sector field's purpose,
and the local test includes a valid target and an unresolved target. However,
`00447bf0` explicitly assigns the final sector-list element when the sector
search returns zero. “Containing sector” and “actors in sector” can therefore
overstate geometric containment. Sent the source-backed correction: describe
the **assigned/current sector-list element, including fallback assignment**,
retain the fresh-reload limitation, and preserve the 53 source-verified field
meanings once precisely described. No extra live experiment is needed for that
wording correction. This is direct review evidence, not a newly executed test.

The 11 varying references' 504-byte target shift remains correlation. Their
report calls the targets transform-pool context based on the export block and
pool path. That is insufficient type evidence: `animation_export.rs` selects a
block for known transforms; `character.rs` groups matrices already found by
typed traversal. Neither establishes that every allocation in that block is a
transform. Sent a targeted next step: reuse the existing imported pool and
allocation evidence to join each of the 22 exact target offsets to its source
allocation ordinal and `TransformEntry.source_index`, checking bounds and
interior offsets. Only successful typed joins warrant payload comparisons or
transform labels; unmatched addresses remain untyped references. Identifying
the source 64-byte allocation's owner/member is still necessary for a field
meaning. This should narrow the hypotheses before another magic experiment.

Both corrections were delivered to **Develop the save effort**; adoption and
benefit are pending. This review found overly strong wording/type inference,
not evidence that the 53 field-purpose mappings must be discarded. Maintain
the 05:21:03.819 usage baseline for the next hourly review; this incoming
checkpoint does not start another usage interval. Review took approximately
three minutes, with no implementation edits, new workers or expensive checks.

### Target-type recommendation outcome — 2026-09-28 06:04 UTC

Sol supplied the result of the requested exact-target discriminator. Reviewed
[item-reference-followup.md](../output/savegame-semantic-map/item-reference-followup.md),
the compact [pool check](../output/savegame-semantic-map/transform-target-pool-check.json),
the `instanciateditem` type assignment in `004fd370`, updated coverage, and
the sector wording in the maintained source/catalogue rule.

Both recommendations from 05:47 were adopted. Sector wording now explicitly
includes fallback assignment. None of the 22 target allocation ordinals occurs
in the existing imported transform pool; bounded source joins instead identify
64-byte SuperObjects with type `0x12`, whose data pointers reach typed inventory
handles. The 11 supplied targets reach NotFound 50, while one magic-labelled
save reaches named Ice/Fire/Lightning items; five additional fixed references
reach other named items. These 16 fields remain partially explained because
source owner/field, slot role, trigger and restoration are unresolved.

This is a demonstrated benefit of the recommendation: checking exact typed
membership rejected an unsupported block-level interpretation and narrowed the
next research question using existing offline evidence. It does not establish
a numerical token/time saving, magic causation or completed field semantics.
Coverage remains **252 complete**, **1,019 unresolved**, now **327 partial**.
Retain exact address/allocation/type checks as a transferable approach; an
adapter's output block or asset path does not type all source data in that block.

Sol reports a full 46-case selected validation running on a stable tree after
focused checks; this review has not observed its outcome and records no new
full-pass claim. No repeated advice, new runtime session, implementation edit or
test was needed. The brief incoming review retains the 05:21 usage baseline for
hourly accounting and leaves the schedule active.

### Hourly review — 2026-09-28 06:22 UTC

**Develop the save effort** is active. The compact snapshot still has cursor
`720307bb-9c9c-4c51-8e15-39bdfc8cb983:2` with no active-turn text. Used recent
local session events and existing artifacts, not that retrieval gap as evidence
of inactivity. No tests, games or monitor workers were launched.

At this checkpoint coverage remains **252 complete meanings**, **837
owned/typed**, **1,019 unknown**, including **327 partial**. Compared with the
last hourly baseline this adds 120 complete records: 67 dialogue counts and 53
assigned-sector references, already reviewed in incoming checkpoints. Only two
of 222 array contents are explained. Global DSG meanings, dialogue content,
map-star consumers, stat bytes, save precedence/initialization, metadata and
additional registration states remain unresolved scope. No completion claim was
made. Sol began a 52-byte/26-actor NeverBack investigation after the completed
validation snapshot; that work is not counted here as a verified new mapping.

**Validation:** the [06:13 report](../TestResults/20260928T061311.413026Z-development.log.json)
passes **46/46**, with 95 unrelated cases excluded, unchanged source and complete
cleanup. Total 440.819 seconds: 0.380 build/discovery, 434.105 execution and
0.039 final cleanup; remaining time is supervisor/setup checks. Existing private
display/audio/isolation and ownership safeguards passed, and the validator
reported no surviving Hype/Godot/Wine/private audio/Xvfb/Orrery processes. No
desktop fallback is reported. Keep the subsequent NeverBack changes outside
this pass's claim until their appropriate checks finish.

Two prior attempts cost **477.671 seconds**. The
[06:03 attempt](../TestResults/20260928T060338.426506Z-development.log.json)
used a 480-second budget, spending 60.153 seconds building/discovering and
465.139 seconds overall before termination after 42 passes. Two tests were
terminated and two lacked results; this is a failed run, not four proven product
defects. The next delegated [06:12 attempt](../TestResults/20260928T061206.255721Z-development.log.json)
used 720 seconds but repeated known sandbox ptrace/display/socket/audio failures:
33 passed, 13 failed in 12.532 seconds. The capability-adjusted retry passed.
All retained source stability and cleanup evidence. Three selections consumed
918.489 seconds total; an earlier list-only build/discovery cost another 53.005
seconds and is not a test pass. Focused checks before these runs were reported;
no additional failure diagnosis was needed for the budget/sandbox causes.

**Delegation and advice:** seven Luna workers contributed in this interval.
The reference investigation produced useful exact offsets but initially inferred
transform context too broadly; Sol and this review corrected it using exact
type/target joins. Dialogue/map-star audits properly left meanings unknown.
The validator completed the required checks. The Global audit, however, repeated
ownership findings and said the AI model was unavailable among its artifacts,
despite earlier identification of `fix:/ai/models/1`; both that model and
`fix:/persos/15` remain in `/tmp/manoir-scene-rete`. This is a concrete artifact
handoff gap. The magic-word investigation retained the known low-byte meaning
and failed to resolve the other bytes; negative research is not semantic gain.

P8 exact test classifications and stable-tree validation were adopted and held
through the completed runs. P9 capability-context transfer remains incomplete:
the new validator repeated an already understood restricted-environment failure.
P10 documentation corrections were adopted, but the Global brief did not supply
the reusable model needed to move beyond ownership. The 05:47 exact-target
recommendation demonstrated a quality benefit, as recorded at 06:04; no
numerical time/token saving follows. Overall delegation remains **mixed**:
useful parallel evidence and validation, plus source-type correction and missed
handoff context. Net time and token savings remain inconclusive, with no matched
Sol-only workload.

[Incremental usage and next baseline](../output/process-reviews/20260928T062233Z/usage-summary.json)
cover 05:21:03.819–06:22:33.844 UTC, subtracting the prior counters once. First
session metadata identifies workers and usage-event models verify Sol/Luna;
parent supervision/integration and corrections are included, audio is excluded.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 54,607,266 | 54,186,368 | 105,584 | 54,712,850 |
| Seven Luna workers | 17,913,069 | 17,516,032 | 55,071 | 17,968,140 |
| Combined | 72,520,335 | 71,702,400 | 160,655 | 72,680,990 |

Uncached input is 817,935; reasoning output is 61,099, already included in
output. Lower usage than the previous hour is not proof of improvement: scope,
build/validation waiting and family difficulty differ. Compact delivery and
validation evidence accompany the baseline in the same ignored directory.

Sent two refinements at 06:25; adoption/benefit pending: carry the successful
720-second budget and normal capability/approval requirements with the exact
test manifest, and give the Global investigator the existing actor/model assets
plus a concrete DSG read/write extraction task. Expected effects are fewer
budget/environment retries and less repeated ownership discovery, without
weaker safeguards or another import/game session just to find the model.

Review overhead was about four minutes. [Monitor usage](../output/process-reviews/20260928T062233Z/monitor-usage.json)
records 2,285,643 tokens for the preceding hour's scheduled/incoming reviews
(2,273,587 input, 2,120,448 cached subset, 12,056 output), separate from the
development subtotal. This review had used 795,049 through 06:25:17, excluding
final logging/response. Broad result dictionaries again caused excess output;
future extraction must reduce statuses to counts before printing. Next review
starts from this new cumulative baseline. Keep the hourly schedule active and
notify the user of the completed validation and concrete handoff corrections.

### Hourly review — 2026-09-28 07:22 UTC

**Develop the save effort** remains active. App cursor remains
`720307bb-9c9c-4c51-8e15-39bdfc8cb983:2` with no active-turn text; used recent
local session events and reports. No monitor workers, original-game sessions
or tests were launched. Current generated coverage is **310 complete meanings**
(305 source-verified plus five runtime-corroborated), **895 owned/typed**, and
**961 unknown**, including **327 partial**. The prose checkpoint still says
309/894 and needs routine reconciliation. This interval adds 56 NeverBack actor
state bytes, one sound-event game-state mask and one configured starting-magic
points word. The last came from bounded disassembly of the pinned executable's
MagicLevel callback omitted from the C export, not inference from fixture 20.
Array-content coverage remains two of 222; broader states, metadata, dialogue
content, map-star consumers and most designer meanings remain open.

The [first-slot precedence report](../output/savegame-investigation/source-player-named-precedence-proof-1790578160771347593/report.json)
passed two conflicting-value cases and identifies the player file as supplying
the tested actor field on that route. Subsequent source review distinguishes
startup state 3/player mode 4 from state 5/named-level mode 8; they must not be
assumed sequential. The named-level transition route remains untested. This is
a useful correction to the earlier general load-order hypothesis, not proof of
universal player-file precedence or all restoration semantics.

**Advice outcomes and validation:** P9's budget/capability recipe is now in the
handoff, including the instruction not to repeat known restricted preflight.
Two stable-tree selections passed without the preceding hour's repeated budget
or sandbox failures: [46 cases](../TestResults/20260928T063239.114464Z-development.log.json)
in 490.203 seconds (54.667 build/discovery, 429.018 execution, 0.039 final
cleanup), then [47 cases](../TestResults/20260928T065247.745818Z-development.log.json)
in 505.304 seconds (1.567 build/discovery, 497.126 execution, 0.155 cleanup).
The new case covers first-slot save conflict; both report unchanged source and
complete cleanup. The 47-case pass includes six original-game scenarios and
native Godot persistence, with no surviving children reported by supervised
stages. Later catalogue/source work must retain separate validation scope.
The two selections total 995.507 seconds; different revisions and the added
scenario justify not treating them as identical repeated work. The working
budget avoided the known under-budget condition; overall time savings remain
unmeasured and faster cached builds are not credited to advice.

The 06:25 Global model handoff was also adopted and **helped narrow research**:
the agent verified the existing actor/model link and produced a real typed
[AST site inventory](../output/savegame-semantic-map/global-ast-reader-sites.md),
with ten own-model indices but only two saved fields. Those two still lack a
proved gameplay role. Follow-up native callback work found executable descriptor
names conflicting with Raymap for IDs 402/415 and appropriately deferred to the
pinned build. This demonstrates why reference labels alone are not behavior
evidence. Negative dialogue/map/state/bit investigations retained unknowns
instead of inflating coverage. Inventory-capacity joins were still being
implemented at cutoff, not counted as completed meanings.

P7 did **not** generalize to ad hoc live work: the
[map probe](../output/savegame-semantic-map/map-state-live-probe.md) discovered
unsupported `End` input only after launching, then exhausted a 180-second
session before post-action evidence. A bounded
[retry](../output/savegame-semantic-map/map-state-live-probe-retry-01.md) used
private XTest input, but captured roughly 3 ms after release without establishing
that the map opened or a relevant update occurred. Unchanged words therefore
add no meanings. Both preserve evidence and report owned cleanup. The retry's
display metadata and Gamescope log show authenticated private Xwayland `:1`
and headless routing; private audio-server logs are present. The reported focus
matches the owned capture window. The successful full suite checks the established
private audio/Xvfb/cleanup safeguards; this review did not independently monitor
all live desktop/audio activity. No host fallback is reported.

Sent two targeted refinements at 07:25 (adoption and benefit pending): preflight
the exact key through the chosen private input route and prepare a bounded
action/wait/capture sequence with an observable prerequisite before another map
probe; and, when returning to Global, index explicit external actor references
across existing typed models by resolved target actor + DSG index, distinguishing
unsupported/dynamic sites and other actors' same-numbered local variables.
The latter is a proposed discriminator, not a claim that such references have
already been found. Preserve the independent inventory-capacity investigation.

**Usage/delegation:** [new baseline](../output/process-reviews/20260928T072233Z/usage-summary.json)
covers 06:22:33.844–07:22:33.944 UTC. First session identities and actual
usage-event models verify one Sol parent and 13 contributing Luna workers.
Parent supervision/integration/correction are included; prior counters and
completed audio scope are excluded. No nested worker spawns were observed.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 42,731,266 | 42,453,632 | 83,338 | 42,814,604 |
| 13 Luna workers | 24,659,148 | 23,694,208 | 127,698 | 24,786,846 |
| Combined | 67,390,414 | 66,147,840 | 211,036 | 67,601,450 |

Uncached input is 1,242,574; reasoning output is 80,364, included in output.
More workers and fewer total tokens than the preceding interval prove neither
efficiency nor inefficiency: output scope differs and uncached input increased.
Delegation assessment remains **mixed**, with demonstrated useful bounded source
and validation outputs, successful artifact reuse, and a failed/repeated map
probe. Net token/elapsed-time savings still have no matched-workload comparator.

Review overhead was about four minutes. [Monitor usage](../output/process-reviews/20260928T072233Z/monitor-usage.json)
records the preceding review interval at 1,171,139 tokens (1,163,719 input,
1,136,256 cached subset, 7,420 output); this review had used 962,523 through
07:25:18, before final logging/response. Several report extracts still exceeded
the desired output bound; prefer conclusions and exact cited lines next time.
Use this saved cumulative baseline at the next hourly check. Keep the schedule
active; 47 passing checks and 310 meanings do not complete the semantic scope.

### Hourly review — 2026-09-28 08:23 UTC

**Develop the save effort** remains active. The app snapshot still omits
active-turn text and retains cursor `720307bb-9c9c-4c51-8e15-39bdfc8cb983:2`;
used recent local delivery events and existing reports. No monitoring workers,
tests or game sessions were started.

Coverage is now **458 complete meanings** (453 source-verified and five
runtime-corroborated), **1,042 owned/typed**, and **813 unknown**, including
327 partial. The 148 additions are 63 inventory maximum-capacity fields,
84 actor hit-point fields and Global DSG[18]'s camera-look gate. The capacity
join rejects the nearby cursor-handle field; the hit-point join checks rooted
ownership, registration policy, allocation size, offsets, kind/flags/width and
near misses. These are repeated records explained through a few verified family
rules, not 148 distinct gameplay concepts. Arrays with explained contents rise
to three of 222, although an older prose sentence still lists only two.
Fresh restoration evidence remains separate from source-level meanings.

The 07:25 inverse-index recommendation was **adopted and helped**. Using the
existing import, Luna indexed 94 linked Persos/70 AI models and 1,216 resolved
typed actor-field sites. That exposed external readers/writers, and a focused
[control-flow follow-up](../output/savegame-semantic-map/global-dsg18-followup.md)
identified DSG[18]: zero permits camera-look entry with the other conditions;
one blocks entry and causes departure from the active look behavior. Unsupported
native call fragments were not used to infer exact camera flag effects. DSG[24]'s
snapshot/restore use in the test actor remains insufficient for a gameplay
meaning. This is a demonstrated research benefit without a corpus reimport or
live run, not a measured net token/time saving.

The map-probe prerequisite advice is adopted in the next-step note: map ownership,
exact key support and visible opening must be established before interpreting
values. No new map probe was observed, so a prevented failure is not measurable.
Gauge ownership/semantics progressed through delegated source work, with a
37-of-40 recommendation at cutoff, but the maintained join was only beginning;
those records are not included in the 458. Map-setting and inventory-object
reference audits correctly retained unresolved gameplay roles. Metadata,
named-level mode-5 behavior, unvisited states and broad designer/reference
meanings remain part of the incomplete objective.

**Validation/process:** the [48-case selection](../TestResults/20260928T074829.024819Z-development.log.json)
passed on unchanged source in **498.150 seconds**: 1.343 build/discovery,
488.875 execution, 0.038 final temporary cleanup, with remaining time in runner
checks/setup. No leaks or remaining children were reported; the selected original
game/Godot persistence and private-display/audio/ownership safeguards passed.
The 720-second budget and approved runtime capability recipe remain adopted,
with no repeat of the previous budget/sandbox failures in these reports. No new
standalone live mapping experiment was observed this interval.

P8 regressed for the hit-point test: the [first attempt](../TestResults/20260928T074603.742688Z-development.log.json)
stopped at discovery for one missing manifest entry, with zero cases executed.
It took 70.887 seconds, of which 70.079 was build/discovery; that compilation
was reused, so the whole 70.887 seconds must not be called wasted work. The
validator retained evidence and did not retry until the owner corrected the
manifest. The next gauge worker explicitly acknowledged delivering its focused
manifest row. This is renewed adoption; prevention benefit awaits the next
integration. Stable-tree ownership held through both attempts. No extra feedback
was sent: the owner had already corrected the omission and the next brief
addresses it, while the current bounded investigations preserve semantic scope.

[Incremental usage/new baseline](../output/process-reviews/20260928T082334Z/usage-summary.json)
covers 07:22:33.944–08:23:34.100 UTC. First session identities and usage-event
models verify Sol plus 12 contributing Luna workers; parent integration,
supervision and corrections are included, with prior counters subtracted and
completed audio scope excluded. No child worker spawns were observed.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 33,505,693 | 33,207,936 | 53,909 | 33,559,602 |
| 12 Luna workers | 33,004,456 | 32,104,448 | 162,149 | 33,166,605 |
| Combined | 66,510,149 | 65,312,384 | 216,058 | 66,726,207 |

Uncached input is 1,197,765; reasoning output is 89,805, included in output.
Delegation produced useful implemented joins, independent consumer evidence
and verified validation. Overall process assessment remains **mixed**, with a
clear inverse-index benefit and a corrected test-inventory handoff defect. More
explained records for similar total usage is encouraging but not causal evidence
of savings: capacity/HP family multiplication differs from prior difficult
single-field research. Net token and elapsed-time efficiency remain inconclusive.

Review overhead was about three minutes. [Monitor usage](../output/process-reviews/20260928T082334Z/monitor-usage.json)
records 1,713,866 tokens for the preceding review interval (1,705,915 input,
1,661,696 cached subset, 7,951 output), separate from development. The current
review had used 626,327 through 08:25:16, before final logging/response. Next
review starts from this saved cumulative baseline and should check gauge
integration/manifest delivery without replaying these completed audits. The
schedule remains active; no completion claim or user action is pending.

### Hourly review — 2026-09-28 09:25 UTC

**Develop the save effort** is active. The compact app snapshot still omits
active-turn text; cursor remains `720307bb-9c9c-4c51-8e15-39bdfc8cb983:2`.
Used recent local agent events, current coverage/progress and compact reports.
No tests, runtime sessions, implementation edits or monitor workers were started.

Coverage is **505 complete meanings** (500 source-verified plus five
runtime-corroborated), **1,084 owned/typed**, and **766 unresolved**, including
336 partial. The 47 additions comprise 37 gauge fields and ten array records:
Global DSG[17], DSG[30], DSG[145], the clock, and six mannequin fields. Arrays
with explained contents rise from three to 13 of 222. The old prose sentence
listing only dummy/lever arrays is stale; generated coverage and the individual
source rules provide the current evidence. None of these family counts measures
the fraction of investigation time remaining.

The relocated gauge join accounts for three roots and 17 children, proves 34
child phase/progress bytes and three completed-child counters, and keeps three
threshold selectors partial. Its focused checks reject wrong record shapes and
an unlinked allocation of similar size. The [clock trace](../output/savegame-semantic-map/save-timer-followup.md)
identifies the eight-byte timer and matches the supplied low/high words to two
General000 metadata lines outside the 1,271 SAI identities. It explains one
no-action difference; pause/menu inclusion remains open. Two other varying
records are partial 36-byte position snapshots from 88-byte matrices, with
Hype ownership stronger than the second actor candidate. Do not count their
unexplained bytes or candidate ownership as complete.

Inverse-index advice continues to yield useful leads: the expanded ranking
joins all 198 saved Global fields, with 335 explicit reads and 416 writes,
173 fields having both and 25 lacking supported explicit sites. The new narrow
meanings preserve important limits: DSG[30] supplies a checkpoint sighting vector
with sign-adjusted X/Y and discarded Z at its consumer, not a proved identical
restored orientation; DSG[145] selects two authored placement-vector sets, not
an identified story era. DSG[0]'s channel-72 toggle remains unknown because the
channel's purpose is unresolved. Review also withdrew an overbroad mannequin
DSG[2] promotion after finding another writer and no reader; only six of nine
mannequin candidates are counted. This is evidence that Sol's integration
review remains necessary and is catching unsupported interpretations.

**Validation/advice:** [49/49 selected cases passed](../TestResults/20260928T084019.272087Z-development.log.json)
after the gauge join, with unchanged source and complete cleanup. Total time
576.355 seconds: 73.713 build/discovery, 495.083 execution, 0.036 final temporary
cleanup; other time is runner checks/setup. No leaked/remaining children were
reported. Existing original-game/Godot and private display/audio/ownership
safeguards passed; there was no new standalone live mapping run in the observed
deliveries. No host fallback or competing session was reported.

P8's test-manifest delivery was adopted by the gauge implementer and discovery
accepted the new test on this run, with no repeat classification failure. The
720-second budget/capability recipe and stable-tree validation remained effective
for this larger build. Later source-only labels used Python compilation,
catalogue reconciliation and diff checks without repeating the live selection;
the progress note correctly separates those additions from the earlier full
pass. P1/focused validation is therefore visibly adopted, though avoided time
cannot be quantified against an unobserved alternative. P7 map prerequisites
remain in the plan, with no new map trial to measure their benefit. No new
feedback was needed: the owner already applied the relevant advice and retained
unknowns while advancing the next bounded source traces.

[Incremental usage/new baseline](../output/process-reviews/20260928T092504Z/usage-summary.json)
covers 08:23:34.100–09:25:04.102 UTC. First session identities and actual usage
models identify Sol and ten contributing Luna workers; parent supervision,
integration and corrective review are included. Existing counters were subtracted
once, audio is excluded, and no nested worker spawning was observed.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 43,178,152 | 42,832,384 | 88,550 | 43,266,702 |
| Ten Luna workers | 38,167,802 | 37,060,352 | 177,888 | 38,345,690 |
| Combined | 81,345,954 | 79,892,736 | 266,438 | 81,612,392 |

Uncached input is 1,453,218; reasoning output is 116,275, included in output.
Delegation provided useful implemented joins, source traces and validation;
quality review corrected proposed overreach. Net token/elapsed-time efficiency
remains **inconclusive**. Higher usage and fewer mapped records than the prior
hour do not establish a regression: the prior capacity/HP rules covered large
families, while this interval includes harder individual-field and owner traces.

Review overhead was approximately three minutes. [Monitor usage](../output/process-reviews/20260928T092504Z/monitor-usage.json)
records 1,127,206 tokens for the preceding review interval (1,121,173 input,
1,084,672 cached subset, 6,033 output), separate from development. This review
had used 691,156 through 09:26:34, excluding final logging/response. Use the new
saved baseline next hour. Broad designer, bitfield, dialogue/reference content,
named-level restoration and corpus-state discovery gaps remain; keep the hourly
schedule active and do not treat 505 meanings or 49 passing cases as completion.

### Hourly review — 2026-09-28 10:26 UTC

**Develop the save effort** remains active. The app cursor is unchanged at
`720307bb-9c9c-4c51-8e15-39bdfc8cb983:2`, still without active-turn text.
Used recent local deliveries and reports rather than assuming inactivity. No
monitor workers, tests, game sessions or implementation edits were performed.

Coverage is **508 complete meanings** (503 source-verified plus five
runtime-corroborated), **1,087 owned/typed**, and **763 unknown**, including
**366 partial**. The three additions are the separately rooted inventory
resource balance, full-list flag and exact first-gauge-root next link. The
resource's spending/reward/capacity behavior is identified, but its authored
name and fresh-load behavior remain unproved. Thirty more records have partial
evidence; arrays with complete contents remain 13/222. New Global guard/copy
annotations deliberately stop before unsupported downstream gameplay meanings.
The old broad corpus and named-level restoration gaps remain; no completion
claim was made.

**Validation:** [09:40](../TestResults/20260928T094024.061554Z-development.log.json)
passed 49/49 in 549.436 seconds (59.380 build/discovery, 483.334 execute stage,
0.295 final cleanup). The latest [10:13 run](../TestResults/20260928T101336.923585Z-development.log.json)
passed 49/49 in 500.521 seconds (0.318 build/discovery, 493.856 execute stage,
0.037 final cleanup), including all six original-game scenarios and native
Godot persistence. Both report unchanged source and complete cleanup, with no
leaked/remaining children. Existing private display/audio/ownership safeguards
passed. Later partial catalogue annotations received static/reconciliation
checks and were not presented as extra live scenarios.

P9 regressed despite the documented recipe: a new validator's
[10:11 attempt](../TestResults/20260928T101112.874990Z-development.log.json)
ran in the restricted environment, producing 35 passes and 14 failures from
known ptrace/display/socket/audio restrictions. It consumed 70.346 seconds,
including 57.914 seconds of compilation reused by the successful retry; do not
call all of that wasted work. Logs and cleanup were preserved. All three
selections used 720 seconds and unchanged source; total elapsed time was
1,120.303 seconds. No new test-manifest omission or budget overrun occurred.
P8/budget/stable-tree practices held, while capability-context transfer did not.

The named-level probe was only checking fixture/launch capabilities at cutoff:
its worker found no save-root override in the available MCP service and was
examining the existing isolated harness route. There is no new mode-5 result or
finished-session cleanup to claim. Do not interfere with its exclusive ownership
or run a competing session. The previous map prerequisite advice remains in
the plan; no new map-opening evidence was observed.

**Delegation/research:** nineteen Luna workers contributed, including reused
agents receiving additional bounded traces. Useful outputs include implemented
joins, partial actor-qualified data flow, and current-tree validation. Several
reports now stop at unsupported consumer behavior (for example Global 41/43
and 66), while another magic-word report repeats the established low-byte/high-
byte boundary. One worker reported an inspector `positions` error that Sol
could not reproduce with the current inspector; Sol recovered the narrow
copy relation and continued tracing its consumer. These are concrete context
and shared-blocker issues, not evidence that the worker count itself is harmful.

Sent two revised recommendations at 10:28, adoption/benefit pending:

- Since a memo alone has not prevented repeated restricted runs, keep repeat
  runtime validation with a continuing owner and its exact successful invocation,
  or require a new validator to acknowledge the budget/capability/normal approval
  requirements before execution. Include the fixture-input contract in live
  briefs; identify the proven isolated fixture path before launch, preserving
  the current probe and all safeguards.
- Group unresolved work by shared operation/callback or target actor/behavior,
  carrying the existing negative evidence and current inspector invocation.
  Resolve one common consumer in a bounded assignment and reuse it across fields,
  while preserving independent parallel work. Expected effect: less repeated
  source/context work and more discriminating follow-ups, not weaker meanings.

[Incremental usage/new baseline](../output/process-reviews/20260928T102604Z/usage-summary.json)
covers 09:25:04.102–10:26:04.168 UTC. First metadata identifies each worker;
usage-event models verify Sol and 19 contributing Lunas. Parent integration,
supervision and corrections are included. Prior totals and audio scope are
excluded; no nested worker spawning was observed.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 56,933,942 | 56,341,504 | 100,222 | 57,034,164 |
| 19 Luna workers | 35,782,703 | 34,513,664 | 178,796 | 35,961,499 |
| Combined | 92,716,645 | 90,855,168 | 279,018 | 92,995,663 |

Uncached input is 1,861,477; reasoning output is 87,697, already in output.
Overall process results are **mixed**: useful partial evidence and completed
validation, but renewed setup and context rework. Net token/time savings remain
inconclusive. Three promotions and higher usage cannot by themselves demonstrate
regression, since many fields require unresolved consumer/initialization work
and 30 gained partial evidence.

Review overhead was about four minutes. [Monitor usage](../output/process-reviews/20260928T102604Z/monitor-usage.json)
records 1,240,175 tokens for the prior review interval (1,233,903 input,
1,205,248 cached subset, 6,272 output). This review had used 1,353,374 through
10:28:46, excluding final logging/response; monitoring remains outside the
development subtotal. Use this saved cumulative baseline next hour. Schedule
remains active; assess the revised handoffs and named-level evidence next.

### 2026-09-28 10:52 UTC — documentation durability check

Stephen asked whether findings will survive for later parity/API work; mapping
priority is unchanged. The local documentation is substantial: field-level
meanings, source evidence, confidence/unknowns, pinned inputs and regeneration
commands are recorded in the catalogue, stable save guide and progress note.
However, `git ls-files`/status show the new progress/handoff notes, semantic and
discovery generators, join and registry modules are untracked; the tracked save
guide is modified but uncommitted. The catalogue and 68 research Markdown reports
are ignored. The generator cites some of these reports, and some reports depend
on `/tmp` artifacts/helpers, including `rank_global_saved_fields.py`. Catalogue
regeneration alone does not reproduce those investigations. No independent
backup was verified. Local documentation quality is not durable preservation.

Sent Sol two preservation suggestions: checkpoint owned source/docs in version
control and consolidate essential authored evidence/negative findings into
tracked documentation; retain necessary authored helpers in maintained tools
with commands, input hashes and a private-evidence manifest. Raw game data/saves
and runtime evidence remain private per repository policy. No scope change,
competing edits or new runtime tests requested. Adoption/benefit pending. The
compact task snapshot still reports active work but exposes no current message;
the assessment uses local artifacts. No development usage interval was sampled
for this bounded user-requested audit.

### 2026-09-28 11:00 UTC — authorized application of findings

Stephen explicitly confirmed use of `notes/` and authorized improvements to the
Godot port, its AI API, Astrolabe and Orrery based on discoveries, provided they
do not become a material distraction. Updated the semantic-map handoff's scope,
implementation guidance, preservation requirements and deliverables, and linked
the clarification in `notes/README.md`. Mapping remains primary; its completion
contract is unchanged. Bounded useful improvements may proceed with focused
validation and existing architectural/runtime safeguards. Broader opportunities
receive evidence-backed follow-ups in notes. Source understanding, implemented
behavior and runtime parity must remain distinct claims.

Sent the changed plan and authorization to Sol, including propagation into its
checkpoint and worker briefs. Essential authored research and necessary helpers
must be preserved in maintained/versioned locations; raw game-derived evidence
remains private. Adoption and benefits are not yet measured. The hourly review
guidance was updated to assess these authorized improvements and documentation
durability, with its cadence, mapping completion rule and quiet-notification
policy preserved. No implementation files were edited or validation runs started
by the monitor.

### Hourly review — 2026-09-28 11:27 UTC

**Develop the save effort** remains active. The compact app cursor still exposes
no active-turn text; recent local messages, usage events and saved artifacts
provide this review's evidence. No monitor workers, game sessions, tests or
implementation edits were performed.

Coverage remains **508 complete meanings** (503 source-only plus five
runtime-corroborated), **1,087 owned/typed**, **763 unknown**, now **369 partial**
(three more than last hour). Arrays remain 13/222 complete. The 37-level
structural survey and 20 retained save roots do not establish comprehensive
semantic or progression-state coverage. New shared-consumer investigations
clarified native time-selector values, ordered tier branches and dialogue
answer-object structure without promoting unsupported gameplay names. Grouping
by shared consumer was adopted and produced reusable partial evidence; net
time/token benefit remains unmeasured. Review corrected one worker's Hype
Variable[57] read count from 109 to 121 and retained its competing writer.

**Preservation improved materially.** Commit `1e6678f` preserves the source
joins/registry, generators, save guide, handoff/progress and the new
[evidence ledger](savegame-semantic-evidence-20260928.md). The maintained
`tools/save_global_ai_index.py` now reconstructs its input from imported Rete
and reproduced 1,216 typed sites, 198 saved Global fields, 335 reads and 416
writes. This follows the preservation advice and removes the temporary-export
dependency. The ledger records source anchors, negative findings, restoration
limits, local hashes and reproduction boundaries. Raw evidence remains private
and local; no independent backup was verified. Latest notes and runtime/test
changes remain uncommitted, with their dependencies under Sol's review. The
shared working-tree validation below is not proof that the commit alone
contains all runtime support. No new Godot gameplay or AI API implementation was
established this interval; the authorized bounded improvements remain available.

**Restoration evidence advanced.** The
[named-level case](../output/savegame-semantic-map/named-level-mode5-probe-20260928/final-report.json)
demonstrated state-5/mode-8 application for one NeverBack byte in two opposed
variants. Leaving manor normally overwrites its working named save, invalidating
the initial conflict assumption. The decisive test reinserted only the opposed
byte into an isolated file after departure and observed that value on return.
This is controlled named-file evidence, not proof of ordinary unmodified
precedence for every kind of record. A source-trace worker helped correct the
live worker's fixture timing. The focused test passed in 101.826 seconds with
28 safeguards; private display/audio and owned cleanup are documented.

The first full 50-case run
([11:02](../TestResults/20260928T110236.394288Z-development.log.json))
passed its tests but failed the outer integrity gate after implementation
changes, costing 598.137 seconds (2.171 build/discovery, 589.180 execute stage).
The frozen rerun
([11:14](../TestResults/20260928T111438.680887Z-development.log.json))
passed **50/50** in **587.276 seconds** (0.344 build/discovery, 580.355 execute
stage, 0.039 final temporary cleanup), unchanged source and no leaked children,
remaining processes or timeout. Both declared 960 seconds. Private-capability
and exact-manifest handoffs held; the source freeze initially failed, then
worked. The successful run includes seven original-save scenarios and native
Godot persistence. A separate staged-checkpoint compile passed in 18.89 seconds;
its broader test compilation then failed on `/tmp` quota after about 40 seconds,
not a demonstrated source defect.

Sent two refinements, adoption pending: explicitly include helper authors and
the exact `source_digest` paths in the freeze (including `tools/`), allowing
`notes/`/`docs/` research outside the fingerprint; preflight a disposable build
target's capacity/quota and select needed affected test binaries before another
isolated compile. Staging and note edits alone do not change this digest; the
helper was still changing during the rejected run. Preserve the integrity gate,
valid 50-case evidence and cleanup safeguards. No new benchmark or rerun was
requested.

[Usage/new baseline](../output/process-reviews/20260928T112704Z/usage-summary.json)
covers 10:26:04.168–11:27:04.211 UTC. First session metadata and models at usage
events identify Sol plus 17 contributing Luna workers; no child spawning was
observed. Parent supervision, integration and corrections are included. Older
counters and the separate audio feature are excluded.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 40,540,131 | 40,271,744 | 69,339 | 40,609,470 |
| 17 Luna workers | 39,711,526 | 38,423,808 | 203,091 | 39,914,617 |
| Combined | 80,251,657 | 78,695,552 | 272,430 | 80,524,087 |

Uncached input is 1,556,105; reasoning output 102,424 is already within output.
Delegation produced useful preservation tooling and verified restoration work,
with shared-consumer evidence, but also validation and environment rework.
Overall results remain **mixed**; lower usage than the previous interval does
not establish savings across different scopes or a Sol-only comparison.

Review overhead was about five minutes. [Monitor usage](../output/process-reviews/20260928T112704Z/monitor-usage.json)
records 3,798,899 tokens for the preceding planning-task interval, which also
included Stephen's durability questions and plan/automation amendments and must
not be labelled pure monitoring cost. This review used 692,552 through 11:30:02
(687,342 input, 644,096 cached subset, 5,210 output), excluding final logging.
An overbroad metadata print was truncated and a monitor-only extraction helper
needed one correction; neither touched development. Saved cumulative counters
now include read offsets to reduce future review overhead. Schedule stays active;
mapping is incomplete.

### Hourly review — 2026-09-28 12:28 UTC

**Develop the save effort** is active under the same title. App retrieval remains
limited to the old cursor without active-turn text; used narrowly extracted new
local messages and artifacts. No monitor workers, implementation edits, game
sessions or tests were started.

Ownership rose **1,087 → 1,222**: 25 StandardGame actor fields plus 100 map UI
slots and ten map-state array slots. **Complete meanings remain 508** (503
source-only plus five runtime-corroborated); **763 are unknown**, with partial
evidence increasing **369 → 397**. Arrays remain 13/222 complete. Map table
entry zero is separately initialized and entries 1–99 are parser candidates;
missing authored setting data and gameplay consumers prevent meaning claims.
These are useful ownership advances, not another 135 understood attributes.

The bounded Astrolabe change reuses typed AI template discovery, then an exact
reverse pointer/Perso validation for the last previously unowned actor field.
All 25 now have source owners; the validation worker compared canonical/probe
results and the final path changed only the last join. The existing save-join
regression includes representative template and reverse-pointer targets. A
further 201-owner ambiguity audit was running at cutoff; do not claim its result
yet. No new Godot AI API/gameplay implementation was established this interval.

**A material attribution correction:** local `Expression::Variable(n)` in the
lever, Gogood and Hype cases addresses the executing actor's Brain/DSG slot. The
previous Hype Variable[57]-versus-Hype DSG[57] distinction was wrong; Hype and
Global DSG[57] are distinct because they belong to different actors. The
[tracked evidence ledger](savegame-semantic-evidence-20260928.md) now records
that correction and the supported partial selectors, without inventing their
complete gameplay effects. Several worker reports needed corrective passes.
Sent one new suggestion: preserve this ownership contract in common worker
briefs/index documentation and consider a bounded local-use index extension
after the current refresh. Resolve uses per owning Perso/Brain, retain multiple
actors sharing a model, and audit only earlier conclusions dependent on the
incorrect separation. Adoption/benefit pending; no broad rerun requested.

**Validation/process:** the exact fingerprint freeze advice was adopted in the
handoff and followed during the
[12:11 selection](../TestResults/20260928T121150.841596Z-development.log.json).
It passed **50/50** in **643.233 seconds** of a 960-second budget, including
20 mandatory selected safeguards and seven original-save scenarios, with
unchanged source and complete cleanup. Build/discovery took 68.558 seconds,
execute stage 567.622 seconds, and final temporary cleanup 0.039 seconds.
Formatting, Clippy and the locked release build passed (0.419, 2.874 and
45.970 seconds respectively in the worker report). A focused join test passed
in 5.05 seconds; initial formatting differences in two assertions were fixed
before final validation. No invalidated full-run repeat, restricted runtime
attempt, quota failure, timeout or remaining owned processes was reported in
this interval. The prior private display/audio safeguards remained in the
passing selection; no new ad hoc map-game session was observed.

The freeze enabled notes/static investigation during validation and avoided the
previous repeat in this interval; this supports its practical benefit but not
a controlled numerical saving. Quota preflight is in the handoff; no new
disposable build occurred, so its benefit is unmeasured. The 110 map-slot
annotations followed the frozen run and received Python compilation and full
catalogue regeneration; they were not misrepresented as new runtime evidence.

Preservation continues: commit `5e66361` records the preceding validated
checkpoint; current owner/variable findings are in the tracked ledger and save
guide, with their latest edits still uncommitted. Private artifacts remain local.
Further original-runtime/test changes remain outside the committed checkpoint;
the working-tree pass is not a clean-checkout parity claim.

[Incremental usage/new baseline](../output/process-reviews/20260928T122804Z/usage-summary.json)
covers 11:27:04.211–12:28:04.259 UTC. Session identity and usage-event models
verify Sol plus 17 contributing Luna workers, including still-running audits.
Parent supervision/review/integration is included; prior totals and separate
audio work are excluded. No nested worker spawning was observed.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 42,498,315 | 42,194,688 | 81,053 | 42,579,368 |
| 17 Luna workers | 29,420,844 | 28,303,360 | 162,761 | 29,583,605 |
| Combined | 71,919,159 | 70,498,048 | 243,814 | 72,162,973 |

Uncached input is 1,421,111; reasoning output 81,200 is included in output.
Delegation again yielded useful verified ownership and quality corrections,
alongside attribution rework. Overall evidence remains **mixed**, with improved
validation coordination; net token/time savings remain inconclusive because
scope differs and no matched Sol-only comparison exists. The unchanged complete
meaning count must not be hidden by ownership growth.

Review overhead was about three minutes. [Monitor usage](../output/process-reviews/20260928T122804Z/monitor-usage.json)
records 1,116,756 tokens for the previous review interval (1,107,432 input,
1,055,232 cached subset, 9,324 output). This review used 482,469 through
12:29:31 (480,155 input, 370,304 cached subset, 2,314 output), excluding final
logging/response. Offset-based extraction worked without rereading old message
history. Save the new baseline for the next hour; comprehensive mapping remains
incomplete and the heartbeat remains active.

### Hourly review — 2026-09-28 13:29 UTC

**Develop the save effort** remains active under the same title. The app snapshot
still lacks active-turn text; saved cursor/offsets, new local deliveries and
artifacts supplied this review. No monitor workers, tests, live sessions or
implementation edits were made.

At the coverage snapshot, ownership remains **1,222/1,271**, complete meanings
**508** (503 source-only plus five runtime-corroborated), unknowns **763**, and
partial unknowns **398**, up one. Arrays remain 13/222 complete. Further star,
dialogue and reference-family reviews mostly confirmed existing structural
limits, although the last corrected an unsupported SuperObject owner claim and
the Level/Fix source context. The complete-meaning count has not advanced since
10:26; tool/ownership progress must not conceal the unresolved consumer work.

**The actor-local recommendation was adopted and helped evidence quality.**
The maintained index now projects 6,117 typed model sites to 6,662 actor-qualified
sites and reconciles all 221 saved actor DSG identities exactly once. It keeps
Hype/Global same-number fields and shared-model actors distinct. Its assertions
found lever writes that earlier manual reports missed, and an additional Hype
DSG[11] trace supports a partial latch. The implementation corrected double
counting of increment/decrement targets and a lever guard during review. Fifty-
one compound array targets and 2,769 opaque fragments remain explicit gaps; site
counts do not prove runtime behavior. Python compilation and generation passed;
generation was reported at 0.75 seconds, excluding development/review effort.
There is no measured net token saving or new Godot API/gameplay implementation.

The all-owner audit found one exact Perso parent for each of 201 StandardGame
targets, but identified a latent skip of reverse candidates already found by
rooted/template passes. Sol fixed the merge. The
[focused validation](../output/savegame-semantic-map/reverse-merge-validation.md)
passed one local test (5.78-second body, 9.77-second build), Orrery build
(8.71 seconds), and a fresh join (about 29.7 seconds) whose 1,271-identity JSON
was byte-identical. This fixture does not exercise multiple valid parents;
that limitation is recorded. A plain rustfmt invocation first rejected existing
Rust 2024 syntax; the edition-aware check passed. The previous 50-case pass was
reused with its earlier scope stated, rather than rerunning the full live suite.

**Fresh restoration evidence arrived just after the usage cutoff:** the
[default-magic probe](../output/savegame-semantic-map/magic-default-restore-84.md)
loaded opposed values 8 and 40 in separate isolated processes. Both restored
unchanged to the configured-default word while stable current/maximum mana
remained 30/50, matching the separate persisted fields. Four samples per run
agreed. This supports separate current-mana restoration at stable manor state;
it does not time the default callback or generalize to other fields. The focused
test took 59.93 seconds; both sessions closed with no remaining owned game,
display or audio processes, and original files/saves remained unchanged. The
runtime coverage counter had not yet incorporated this evidence when sampled.
Its post-cutoff usage belongs to the next interval.

The new runtime worker first stopped over its fixture/controller route, then
hit the already-known private-audio socket denial before retrying with normal
approved runtime access. The recommendation to hand off the proven private
launch path has therefore not held consistently across new workers. Its sole
temporary integration test was deleted after the probe; the result and machine
report survive locally, but a maintained runnable reproduction was not yet
established at review.

Sent two refinements: before another pass over exhausted structural families,
carry the exact missing bridge, prior negative evidence and a different
discriminating source/operation/experiment in the brief; prioritize a bounded
high-use unresolved consumer exposed by the new index. Preserve the successful
opposed-value probe or a runnable equivalent and its exact approved private
launch recipe in maintained locations, to avoid repeating fixture/capability
setup and losing executable evidence. No new broad suite or benchmark requested.
Adoption/benefit pending.

Preservation advanced through `0ed7bf6`, `23ccf29` and `bb1ec2c`: owners, index,
partial latch and source-owner corrections are in committed source/notes/docs.
The magic probe still needs the preservation step above. Raw game evidence stays
private; no independent backup was verified.

[Usage/new baseline](../output/process-reviews/20260928T132904Z/usage-summary.json)
covers 12:28:04.259–13:29:04.325 UTC. First metadata and actual usage-event
models identify Sol plus nine contributing Luna workers. Parent supervision,
integration and correction are included; prior usage/audio scope excluded.
No nested worker spawning was observed.

| Scope | Input | Cached subset | Output | Total |
| --- | ---: | ---: | ---: | ---: |
| Sol parent | 43,073,845 | 42,765,696 | 69,724 | 43,143,569 |
| Nine Luna workers | 19,309,892 | 18,760,704 | 127,323 | 19,437,215 |
| Combined | 62,383,737 | 61,526,400 | 197,047 | 62,580,784 |

Uncached input is 857,337; reasoning output 70,132 is included in output.
Delegation produced useful maintained indexing, corrected research and a focused
experiment, but repeated structural work and launch setup remain costs. Results
remain **mixed**; decreasing interval totals or fewer workers cannot establish
savings across different assignments without a comparable Sol-only baseline.

Review overhead was about four minutes. [Monitor usage](../output/process-reviews/20260928T132904Z/monitor-usage.json)
records 890,990 tokens for the previous review interval (885,168 input,
768,384 cached subset, 5,822 output); current review used 747,492 through
13:30:47, excluding final logging/response. An accidentally recursive output
file listing included private Wine-copy filenames and was truncated; future
artifact discovery must constrain depth before printing. No development process
was affected. Mapping remains incomplete and the hourly heartbeat stays active.

### 2026-09-28 14:04–14:10 UTC — user pause and revised estimate

Stephen requested stopping now and polishing a checkpoint. Sent the stop to
Sol and paused the existing hourly automation. Sol stopped delegated work,
closed owned sessions and committed
[the pause checkpoint](savegame-pause-checkpoint-20260928.md) as `57d4a09`.
The final app snapshot reports the task idle and its answer explicitly says
the goal is paused. No Hype/Wine/Gamescope children remain according to the
owner's cleanup report. Do not wake the task or resume the monitor without a
new user instruction. Latest app cursor: `1c8942d2-74b6-4ebd-973c-f4c0e06e5889:1`.

Final counts remain 1,222 owned/typed, 508 complete meanings, 763 unknown
(398 partial) of 1,271 distinct supplied-save identities. The selected-magic
low byte now has a three-trial fresh-load observation; its high-byte marker
persists without an established purpose. The last dialogue probe stopped after
an Orrery registry-reader overflow before observing dialogue data. Exact source
and the next corrective step are in the checkpoint. Original data and saves
were preserved; later annotations do not supersede the timing/scope of the
previous 50/50 full selection.

Stephen then requested revised timing. The
[new estimate](savegame-estimate-update-20260928.md) replaces the optimistic
8–16-hour fixture range: 40–80 additional active Sol-led hours for comprehensive
supplied-save meanings; 100–200+ total additional hours for the broader declared
English corpus, including fixture work. These are low-confidence judgment
ranges, not a throughput regression or additive worker-hours. Roughly ten hours
were spent in this phase, and complete meanings plateaued at 508 despite useful
ownership, tool and validation improvements. Remaining native consumers,
references/arrays and progression states dominate uncertainty. A 4–6-hour
reassessment after an eventual resumption should measure semantic gains and
resolved blockers. Whole-Godot-port timing is not established by this evidence.

Updated the handoff, progress estimate pointer and notes index. These are
documentation changes only; no task was resumed or new research/test run begun.
