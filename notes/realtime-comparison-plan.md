# Live loading, comparison, and behavior testing

## Session-recording direction — 2026-09-27

This is the recording/tooling track of the [current roadmap](./remaining-work.md#current-roadmap--2026-09-27).
Loading efficiency can advance independently. Startup/menu, intro and save
support enter the Godot harness as those first-level features become available;
their absence is an explicit full-session comparison gap, not a reason to
block useful original recordings or focused gameplay work.

The agreed [session-recording design](../docs/astrolabe/session-recordings.md)
extends the earlier focused scenario plan and corrects its ownership boundary:
**Orrery is an OpenSpace runtime analysis tool, now gaining recording/replay
alongside inspection and control. The behavior test suite belongs outside it
and uses it, as originally intended; Godot has a separate harness.** Single-engine
gameplay assertions and cross-engine comparisons both belong to the suite.
Shared evidence does not put the suite or Godot inside Orrery. The older dated
checkpoints below describe what was demonstrated then; they do not establish
current whole-session coverage.

Record from process launch, with the full set of save files available before
launch as an immutable fixture. Empty/absent saves are valid explicit cases;
unreadable saves are not. Choosing a slot or New Game happens through recorded
menu input. Keep startup/loading time, level changes and subsequent save writes
in the chronology. Human-driven sessions and scripted routes use the same
described, tagged recording format, from a short case to a full-game speed-run.
Capture input, output, typed state and AI/API events continuously within declared
coverage so comparison can discover differences the user has not pointed out.

Implementation sequence for this extension:

1. Extract the engine-neutral recording/scenario/report contracts and comparison
   runner from Orrery; give Godot delivery/capture its own harness. Preserve the
   existing tests, scenario identities and evidence while restoring the intended
   ownership. Keep Orrery's own tool/contract tests and runtime analysis APIs.
   The current `crates/orrery/src/behavior/godot.rs` and shared BDD placement are
   boundary violations to fix, not a new responsibility for Orrery. No extraction
   is performed by this documentation update.
2. Establish the session manifest, immutable complete save-file fixture and
   isolated replay setup. Prove two startup recordings: an explicitly empty
   save root followed by New Game, and a populated root followed by menu-based
   slot selection. Include a replay after the preceding run has written saves
   to prove the initial fixture is restored without contamination. The supplied
   English archive is one selected fixture, not a mandatory launch injection.
3. Record a short human-driven original session from launch with input and
   video, supported state and timing evidence; replay it on OpenSpace and report
   repeatability and all capture gaps. Add event-based AI transitions and API
   arguments/results using verified boundaries. Prove transient transitions,
   a deferred effect, a missed event and a deliberately altered argument can be
   detected; declare untraced calls rather than presenting partial tracing as
   exhaustive. Keep the first end-to-end recording useful during expansion.
4. Have the separate Godot harness consume the recording and emit the same
   evidence contract through native gameplay. Missing startup/menu/save support
   remains visible; a focused actor-initialization fixture cannot pass full-session
   replay. Produce a timeline linking input, calls, state and frames, with
   automatic first-divergence reporting and narrowly scoped accepted debt.
5. Add recordings and chapters to cover more routes, then qualify streamed
   storage, level/clock epochs, interruption recovery and explicit endurance
   budgets for long runs. A chapter is not independently replayable without
   replaying its prefix or validating a separate starting fixture.

This extends discovery and repeatability without making complete tracing,
pixel equality or a full-game run a prerequisite for every feature. Report
recording coverage, functional results, precision, accepted debt and missing
capabilities separately. The current narrow `replay::Recording` schema starts
at a ready state and aborts on a lost epoch; it is not yet this session format.

## Earlier implementation checkpoint

Implementation plan, updated 2026-09-23 to put a runnable suite framework
before its first real comparison. The 2026-09-20 input-timing evidence and
technical contracts below remain in force.
The gameplay checkpoint is preserved. The actual Godot input-boundary probe,
focused test runner and initial Rust coverage collection are implemented; see
[testing evidence](../docs/astrolabe/testing.md). The first shared Cucumber
movement scenarios now pass on OpenSpace; Godot execution and broader typed
live loading remain in progress. See the [dated behavior-suite checkpoint](./behavior-suite-2026-09-23.md).
Resume the
[gameplay plan](./godot-gameplay-plan.md) through the first functional comparison
slice; do not wait for complete live loading, precision replay or level coverage.

2026-09-23 status audit: Orrery has native original-game inspection, a controlled
physical-input replay experiment, selected direct/reopened-Rete actor comparison
and an optional Godot recording consumer. Selected OpenSpace record readers are
shared between file and relocated-memory views. Actor action/transform and
normal/reflex behavior-selection reads have been exercised in the original game
through the registered Orrery MCP service. In the [2026-09-23 live session](./orrery-live-session-2026-09-23.md),
the supplied `manoir` save loaded, OS `Left` input changed Hype's transform,
screenshots were captured and cleanup completed. Selected designer-value reads
have synthetic relocated-memory coverage but no live selected-field result yet.
`inspect_main_actor` still returns stopped bytes, not completed-update evidence.
The native replay publishes a qualified action at completed updates for one
fixed Left/Shift+Left recording; that publication is not yet a general typed
AI or whole-level observation API.
There is now one `.feature` file and shared BDD step registry with OpenSpace
turn–release–idle and walk–release–stop results. The synthetic run demonstrates
selection, assertion and mismatch plumbing. These broad functional results use
stopped-byte observations and do not validate exact input-consumer timing or
contact. The [saved player Brain gap](./hype-control-progress.md#player-brain-export-gap--2026-09-23-audit)
also prevents the ordinary `manoir` export from supplying a runnable player AI.
The Godot slot and cross-backend result remain pending. Do not count the
component checks as a completed behavior comparison.

The Python `hype` MCP harness remains available for optional diagnosis and its
recorded observations remain useful baselines. The new suite must not depend on
it. Orrery is the planned original-game backend because its typed OpenSpace
reader can serve both suite code and MCP queries. Its selected live control path
has been verified. The framework must run with an explicit pending original slot
until that path is connected to a reusable scenario and its assertions. A scenario
can then pass on OpenSpace alone; a cross-engine result additionally requires
another live backend.

## Outcomes and boundaries

There are three independently reported results:

1. **Data:** OpenSpace files or reopened Rete assets against structured values
   read from the running original game. Establish asset identity, relationships,
   loaded values, and the reasons particular fields should differ.
2. **Single-backend behavior:** shared Gherkin steps produce one engine-neutral
   scenario and a literal physical-input recording. A selected engine adapter
   delivers the events and maps its observations to scenario terms. Its own
   assertions pass or fail without another engine running. The first real pass
   must be against OpenSpace; Godot and later ports use the same scenario where
   their capabilities apply.
3. **Cross-backend behavior:** when two or more backends run the same scenario,
   compare their outcomes, measured elapsed milliseconds and declared
   screenshot/pixel evidence. Report each backend's own pass/fail result even
   when another fails; the parity result is separate.

These results do not imply one another. Matching meshes or action definitions
does not prove movement; matching an observed movement does not prove complete
asset conversion. Acceptance of reconstructed files by the original game is a
separate later result requiring the generated files actually to be installed and
loaded in a test copy. Do not claim it from the results above.

Behavior scenarios declare an evidence level. A **functional** scenario uses
the declared ordered physical controls on each selected backend, records actual
dispatch and observed timing, and checks a visible/state outcome with bounds
wide enough for the known scheduling uncertainty. It proves the behavior path
on that backend, not
frame-exact replay or timing parity. A **precision** scenario additionally
requires verified consumption-update boundaries, frame-indexed delivery,
completed-update observations and predeclared millisecond tolerances. Keep a
missing precision capability explicit in the report instead of blocking an
unrelated functional demonstration. Promote a scenario when its claim needs
precision, or earlier if coarse timing could conceal the wrong AI choice,
input route, collision result or action order. Both levels use the same suite,
scenario meaning, backend interfaces and evidence format; the schedule and
assertions are declared rather than silently downgraded.

This is one application of the main plan's [sufficient-first-pass rule](../plan.md#development-rule-always-ready-to-demo). Apply it to the suite's other groups too: a selected-field live data check can validate the shared reader before every record is decoded; one useful evidence report can validate the result path before every metric is available. Each first pass must exercise its real owner and expose the unfilled cases. Finish details when they are needed to distinguish the claim, support a dependent scenario or satisfy a later acceptance gate.

Keep the OpenSpace, Rete, and Godot adapters around the existing useful model.
Do not implement OpenSpace execution, scheduling, or memory machinery in Godot.
The live loader is an additional OpenSpace source, and the native controller is
optional tooling distributed alongside the converter. Generated games and
ordinary conversion do not depend on the controller, MCP, or the original game.

## Checkpoint and resume contract

Gameplay checkpoint: **`c912db7`**, now included on `main` together with the
reviewed plan and implementation handoff. Start new implementation from `main`;
the named recovery tag preserves the pre-implementation state. The checkpoint
builds and passes formatting/Clippy, but has a known Godot rule-probe timeout and an incomplete full-suite run
at the old global cap. The [checkpoint evidence](./hype-control-progress.md#paused-implementation-checkpoint--2026-09-20)
records the exact limits. The initial planning/policy commit is **`4732ed7`**;
this update records the subsequent timing decision and shared-step design.

The implementation and tests are preserved separately from planning/policy. The
checkpoint changes to model types, codecs, Rete schema, exporter, native support,
and their tests are
interdependent; do not manufacture independently buildable feature commits by
arbitrarily splitting those files. Keep the validation-profile/runner change
with the tests it supports. Record exact versions, verification and open work in
[Hype control progress](./hype-control-progress.md).

The snapshot contains typed designer values and action arrays, source AI
arithmetic/behavior flow, native animation motion and contact, turn/walk rule
probes, and character collision profiles. Source-fragment and forced-action
tests remain component evidence. Full Hype initialization, the complete movement
and reflex path, and integrated input-driven walk/run acceptance remain open.
Rete schema is 25 and Godot support interface is 17 at this checkpoint.

Preserve authored settings, ignored generated projects, disc files, saves,
reference submodules, and the separate existing harness. Do not regenerate or
overwrite old local assets merely to make the checkpoint look clean. Validate
new outputs in disposable directories; old schemas require deliberate fresh
conversion. Store new evidence under ignored TestResults/output or the external
harness's local directory, with the associated source revision and fixture hashes.

The [implementation handoff](./realtime-comparison-handoff.md) records the named
recovery tag, preserved local evidence and a goal for the next agent. Start with
the runnable suite framework in the ordered delivery below. Once
the new movement scenarios exist, run them against this gameplay checkpoint and
use their failures while completing the missing gameplay path without
replacing the real converted AI with a test controller. Do not resume the older
archived runtime experiment.

## Native runtime and shared loading

Use `crates/orrery`, the user-facing **Orrery** runtime analysis tool depending on
the Astrolabe library, for native OpenSpace inspection, analysis, control,
recording/replay and CLI/MCP entry points. Keep Godot control/capture in a separate harness and
shared BDD/cross-engine comparisons in an independent runner, using shared
recording contracts outside Orrery. Existing combined code needs extraction as
described in the [2026-09-27 update](#session-recording-direction--2026-09-27).
Keep the root converter as the default workspace member; it must not acquire a
dependency back on either harness or the comparison runner.

The testkit process launches/owns the original game and links Astrolabe directly.
Prepared tests instantiate that same runtime session; the persistent MCP binary
owns a session for agent queries. No Python memory broker or internal per-record
RPC is needed. MCP is only the agent-facing surface. The available Python controller's
interface and recorded results are useful references; new suite control and
comparison work uses Orrery.

Suggested ownership:

| Concern | Owner |
| --- | --- |
| Hype record declarations, build-specific roots, field interpretation | `src/openspace`, sharing existing binrw declarations and codecs |
| Offline blocks/relocations and live pointer resolution | Separate source implementations inside OpenSpace |
| Original-game process handle, memory reads/maps, pause guard, launch/input/capture/cleanup | Orrery, with OS-specific modules |
| Assets, transforms, typed designer values and semantic references | Existing `model` types |
| Session handles and backend observations/traces | Owning engine harness; reuse model values without creating another asset model |
| Recording schema, scenario assertions, comparison policy and combined reports | Shared contracts and independent comparison runner |
| Saved native Godot resources and compiled AI | Existing Godot exporter and maintained support |
| Godot observation/control endpoint and replay | Separate Godot harness operating on those same exported resources |

Refactor `SourceSession` incrementally. Extract bounded byte access, pointer
resolution, roots, and source-identity lookup from domain interpretation. Replace
direct `block.data` access in the selected readers with bounded record/string/
array views. Reuse the binrw declarations and semantic mappings for both sources;
do not copy the struct catalogue into the controller. Source pointer integers
remain 32-bit Hype addresses, even in a 64-bit Rust process. Read bytes through
the OS and decode them; do not cast remote pointers into Rust references.

Byte access is not the only offline assumption to remove: current AI readers
derive asset paths from allocation ordinals and record/script lengths from
preceding allocator words. Keep bounds and source-local locators inside
OpenSpace. Prove their live equivalents rather than inventing allocation indices
to satisfy an offline API. Before freezing the shared reader, compare two
distinct equal-valued objects, a shared definition and a replaced dynamic
instance through offline, reopened-Rete and live paths. Source-local locators
and runtime generations belong to observation provenance; cross-source matching
evidence belongs to the comparison session, not a new canonical asset identity.
Unmatched dynamic objects must remain inspectable without pretending they had a
file allocation. This proof must accompany the narrow Hype slice before broader
reader migration makes an incorrect identity/bounds contract costly to change.

Offline reads retain GPT/RTP/RTB handling and active-level Fix bindings. Live
reads use verified globals and relocated pointers, including objects created or
patched after loading. Do not synthesize an SNA/relocation table to make live
memory fit the file reader. Keep unknown data explicit, bound lists/strings/
counts, and report unreadable or unsupported fields without silently supplying
zero. Structural AI inspection must work even if the full program cannot yet be
compiled to GDScript.

Start with the pinned Hype executable and the supplied English save. Validate
executable identity, process start identity, active level and source fixture
hashes. Resolve roots afresh after launch/reload. Bind actor handles to a session
and load generation; also detect removal/replacement within one level. An address
reused for another actor must not reuse the old handle. Persistent asset matching
uses verified source identity, role, family/object types, and instance relations;
names and offline allocation ordinals alone are insufficient. Ambiguous matches
are errors or unmatched results, never guessed joins. Runtime addresses remain
local evidence and are not Rete metadata.

### Snapshot and observation semantics

- A memory-stable inspection capture briefly pauses the owned process, checks that
  relevant threads are stopped, copies the bounded requested closure, and resumes
  before formatting/reporting. This can freeze the engine halfway through an
  update, with new position but previous action/contact values. Label it as a
  stopped-byte capture, not a completed-update gameplay observation. Structural
  validation still applies. A missing page cannot be fetched after resume and
  silently added to the same snapshot.
- Assertions about a coherent actor update or a particular input-consumption
  ordinal need a separately verified completion/publication boundary for their
  fields. An unchanged frame counter is insufficient if it increments at update
  start. Copy the bounded selected values at that boundary, or establish
  equivalent evidence that they represent a completed update and no writer can
  change them during observation. Writer quiescence alone is insufficient. A
  functional assertion may instead use a declared settled-state window or a
  coarse outcome with sampling uncertainty, when that evidence distinguishes
  success from failure. Preserve phase/consistency in the report; do not
  silently promote an inspection snapshot into a completed-update assertion.
  A fake producer stopped between related field writes must fail
  completed-update acceptance. Render association remains a separate contract.
- Cache/batch memory ranges within a capture; avoid a syscall for every scalar
  binrw read. Cache immutable definitions separately only after establishing their
  lifetime. Revalidate roots and invalidate mutable values on each capture.
- Live traces prioritize low disturbance: record capture start/end, game clock
  where verified, input dispatch/acknowledgment, sampling gaps and consistency
  mode. Detect/retry inconsistent observations under a bound or report them as
  unavailable. A running capture is not advertised as atomic.
- Pausing, holding input, or a failed assertion must not leave the game frozen or
  keys pressed. RAII handles normal failures; an outer supervisor/watchdog handles
  hangs and killed workers. Preserve pre-existing pause state and clean up only
  the owned installation/process tree. Test these failure paths with fake child
  processes before relying on the game.
- A snapshot is immutable local evidence with an ID and provenance. Replaying it
  tests decoding/comparison, not live behavior. Do not treat an old capture as a
  fresh original-engine run.

Linux/Wine is the first native target. Reuse Orrery's verified process ownership,
Gamescope/X11 input and capture, and bounded teardown with the prepared private
installation. Connect these capabilities to the suite and validate scenario
failure/timeout cleanup. Extend the controller where the selected scenario
requires more. Windows is a separate backend for process memory/control, not a restriction on the shared decoder.
Native Windows launch, input, capture and snapshot consistency need their own
validation; do not claim support from compiling a memory reader alone. Optional
platform dependencies must not prevent ordinary converter builds elsewhere.

## Replay decision: frames drive inputs, milliseconds judge timing

This section specifies the **precision** replay capability. Implement one
frame-indexed physical-input scheduler for that profile. There is no automatic
millisecond fallback within a precision run: a missing or unverified input
boundary fails that capability. The functional profile may use a separately
declared, controlled elapsed-time schedule with the same physical controls and
recorded actual delivery; it makes no frame-exact or tight timing claim.
Existing best-effort MCP actions remain useful for setup during migration, but
cannot qualify as precision replay evidence.

Here, a replay frame means one eligible gameplay input-consumption update,
identified by a verified engine boundary. It is not a rendered image, a polled
counter change, or an assumed interval derived from a configured FPS. Assign
relative replay ordinals from a verified ready state; retain engine counters,
phase and load/session epochs separately. Menus, focus loss, pause and loading
must have explicit policies. Initially, unexpected transitions abort a gameplay
tape; later transition scenarios need separately verified boundaries.

### Feasibility evidence and its limits

The 2026-09-20 experiment used the Hype MCP, the supplied English save in
`manoir`, and Godot `4.7.2.stable.arch_linux.ed1daf0bf`. The
[local experiment report](/home/stephen/Projects/hype-harness/local/frame-timing-research-20260920/README.md)
and adjacent raw evidence record fixture/build hashes, the probe and cleanup.

- Six original-engine replays supplied physical Left in the current keyboard
  device buffer for exactly five consecutive input updates. The original
  binding/history processing observed held counts 1–5 followed by release.
  No character transform, action counter or game clock was forced. Ordinary
  100 ms MCP pulses instead yielded eleven five-update holds and one four-update
  hold in twelve trials.
- Two real-time Godot probes supplied the equivalent five-update hold using
  physical key events, with correct pressed/held/released observations. Preparing
  the event between updates worked; injecting inside `physics_frame` put
  `just_pressed` on the following physics tick. The production ordering must be
  verified through the actual generated character's input/AI/movement path.
- The measured five-update intervals were **166, 161, 174, 147, 102 and 126 ms**
  in Hype and **104.177 and 103.919 ms** in Godot. These demonstrate scheduling
  feasibility, not timing parity or proposed tolerances. Both were configured
  for a 48 FPS cap in this experiment; the cap did not establish equal cadence.
- Hype uses elapsed real time, including a derived timestep clamped to 16–80 ms.
  Its update counter continues in the pause menu, and its game timer is restored
  on resume. Keep real elapsed time, raw game delta and clamped game delta
  distinct. The counters used by
  [HTTQPlayBack at the inspected revision](https://github.com/RKDGU/HTTQPlayBack/blob/c967c24278c1a4f6040234ecd4c7ea677a773cf9/HTTQ_PlayBack/HTTQPlayBack.cs)
  track animated textures and a water effect; polling them and sending OS keys
  is not a verified input-consumption boundary.

The experiment covered one key, level and executable. It did not prove behavior
parity, arbitrary input devices, transitions, multiple Godot physics ticks per
render, synchronized screenshots or deterministic physics. The running EXE
differed from the decompilation reference; the four relevant live code spans
were checked byte-for-byte before interpreting them. Production validates a
supported build profile, not merely a familiar filename or a few matching bytes.

The disposable Hype wrapper borrowed an empty pending-level buffer. One extra
state-report call consequently failed to decode that buffer; replay completed,
the callback self-restored, and final cleanup verified the restored buffer and
game state before stopping the game. This was timing-feasibility evidence, not a
production input design. Production gameplay input uses OS keyboard events on
the owned private display and leaves Hype's keyboard callback and device buffer
untouched. Any in-process callback wrapper is a separately labelled observation
or timing probe with dedicated owned scratch memory, validated hook ownership
and bounded cleanup; it never supplies gameplay input.

### Original mechanics cadence, input phase and observations

The [local follow-up mechanics investigation](../TestResults/physics-clock-investigation-20260920.md)
traced the main gameplay path and used read-only Hype MCP observations on a fresh
manor fixture. Twenty relevant live code spans matched the static reference;
the owned game was stopped afterward. No memory writes or hooks were used.

**No separate fixed-rate physics accumulator or independent physics thread was
found in that path.** The main gameplay update advances the global clock once,
then traverses actors. Eligible actors receive mechanics delta
`clamp(raw_elapsed_ms, 16, 80) * 0.001` seconds. Hype's own eight-sample mechanics
history contained both 0.020 and 0.021 seconds; separately read global raw/clamped
delta pairs included 20/20, 22/22 and 21/21 ms. These corroborate variable
integration steps, not a phase-pinned trace or an exact callback-count proof.

Actor activation and update guards determine whether mechanics runs; a global
update does not guarantee physics for every actor. The traced eligible-actor
path performs animation, AI and mechanics in per-actor order, not an all-AI then
all-physics world barrier. Mechanics command processing and the selected final
updater receive the same clamped delta. Inspected collision loops perform bounded
geometric resolution work; they do not establish independently scheduled physics
ticks. Exhaustive solver modes and internal subdivisions remain unproved.

The investigation also checked the running game's dispatch pointers: the
gameplay callback performs actor updates and graphics work, followed by the
graphics-end/buffer-swap callback, engine end-of-turn processing and keyboard
input polling. In
steady gameplay, a keyboard poll labelled with global update counter `k` prepares
input for update `k + 1`. It does not describe input already consumed by update
`k`. Preserve delivery and intended/confirmed consumption ordinals separately.

The actor's last-updated stamp is written before its animation/mechanics work.
Matching that stamp to the global counter cannot establish a completed actor
observation. Return from the gameplay callback, after actor updates and before
buffer swap, is a candidate completed-state boundary to validate. That callback
already does graphics work; it is not a proven pre-render boundary.
Presentation/image capture remains separate.
These findings strengthen the requirement to prove the actual consumer and
observation phases before interpreting the earlier keyboard probe as a precision
gameplay replay contract. A functional movement observation can proceed with
measured scheduling uncertainty while the compiled player AI is made playable.

Consequently, do not label the original input counter a fixed physics clock or
assume its timestep equals Godot's. For precision replay, use the original
consume-update boundary and a verified native Godot consumer boundary to index
the shared recording; qualify their real-time cadence and preserve both
engines' actual deltas.
Choosing Godot's native physics rate for a comparison profile is still a required
experiment, not permission to replay original deltas or recreate its scheduler.

### One shared recording and independent delivery adapters

The [whole-session contract](../docs/astrolabe/session-recordings.md) now owns the
outer timeline, prelaunch files and phase/level epochs. The ready-state recording
below remains a bounded gameplay profile within that design, not a restriction
that removes startup, menus or loading from a session. A captured human session
is frozen after recording; prepared scripted inputs can be frozen before it.

Shared steps/helpers build a versioned recording of literal physical events:
ordered press/release events, physical key/device identity, relevant modifiers,
and an explicit schedule. Functional recordings use declared elapsed-time
targets; precision recordings use relative input-update ordinals. Both retain
actual delivery times, and precision runs retain verified consumption ordinals.
Bind each fixture to the same controls; platform-specific key-code translation
preserves the physical event. Keep repeats, simultaneous events, initial held
state and final release explicit.
Start with keyboard support; do not claim mouse/controller equivalence yet.
Compose the steps into **one complete scenario recording**, with one ready-state
epoch, explicit neutral gaps and observation/settling windows. Compile and hash
that recording once before playback; each selected backend consumes those same bytes.
Assertions inspect declared recorded checkpoints without delaying later inputs.
Step dispatch, MCP latency, screenshots and assertion execution must not define
gaps in the recording. A two-action test with a deliberately slow intervening
assertion must still preserve the declared event schedule on each backend. A reset
inside a scenario is an explicit new setup epoch, not an unrecorded delay.
Do not change keys, hold length or release time in response to either engine's
progress. In a precision run, a missed boundary, duplicate event or replay
buffer underrun fails the run; never compress several events into one tick to
catch up. In a functional run, report actual timing and invalidate an outcome
whose delivery misses its declared useful window.

The original adapter queues the tape locally and delivers literal OS keyboard
events through Orrery's owned Gamescope Xwayland display. Hype performs keyboard
translation, callback order, edge/history processing, bindings and consumption.
The adapter never writes the keyboard buffer or patches a keyboard callback to
supply gameplay input. A separately labelled in-process observation/timing probe
may measure update boundaries; missed or late ordinal delivery fails rather than
switching input paths in a precision run. The Godot adapter supplies equivalent
physical `InputEvent`s through normal input dispatch; precision replay also
verifies delivery before the intended consumer tick. It does not call player
actions or set movement state.
Additional engine adapters must implement this same scenario contract without
making OpenSpace's memory layout, action IDs or scheduler part of the shared
assertions. Map engine-specific observations into named behavior facts such as
input accepted, motion, action category and contact, retaining raw values as
backend evidence. Unsupported observations are explicit per-backend capabilities.
Do not equate render and physics ticks or introduce an OpenSpace scheduler into
Godot to obtain matching counters. Prove the ordering of normal Godot input,
generated AI, movement and observation before fixing the adapter contract.

Use bounded local queues and batched trace reads instead of one MCP transaction
per frame. Functional runs record requested and actual dispatch/delivery times;
precision acknowledgments additionally record verified consumption updates.
Validate press/release edges and cleanup for the first functional path. Before
claiming precision replay, add one-tick pulses, chords, repeat policy, slow
renders and multiple physics ticks per render; also test pause/reload
invalidation, buffer exhaustion, failed hook installation and worker death.
Restore callbacks only while ownership still matches; do not free scratch
memory while a callback can execute. An outer owner must release inputs and
safely detach or stop its game after a failed worker. Bound instrumentation cost
and capacity; missing/overwritten observations invalidate the affected claim.

Keep the recording schema and backend tick contract provisional until real
cases pass. The schedule is explicit per scenario; a functional result cannot
be silently promoted to a precision result.

### Independent real-time comparison

For each run retain requested and delivered event order/times, available engine
simulation counters/deltas, and measured monotonic elapsed milliseconds.
Precision runs also retain verified consumption ordinals. Record dispatch,
verified consumption where available, and observable outcome times with sampling
uncertainty.
Relate any engine clock to the host monotonic clock with measured uncertainty;
handle resets/wrap explicitly. Never calculate measured milliseconds as
`frames / FPS`, use a pause-adjusted game timer as wall time, or count an
accelerated `--fixed-fps` run as real-time parity evidence.

Compare input hold duration and input-to-outcome latency (movement onset,
stopping, action transitions, contact), as well as the state trajectory and
visual milestones at the declared evidence level. Same event order and intended
durations do not guarantee equal simulation time, physics results or elapsed
time. Keep event IDs and elapsed time in every report, and verified consumption
ordinals in precision reports. Semantic labels may identify matching events,
but must not shift/stretch timelines to conceal a late result.

Before accepting tight tolerances, qualify baseline cadence with repeated
original runs and instrumented versus minimally observed controls. Record actual update/
render intervals, configured caps, stalls and observation overhead; do not
silently retry away slow runs. A documented environment/cadence failure is
inconclusive or failed, never a parity pass. Choose the first Godot test cadence
from this evidence and validate its ordinary native update behavior; the probe's
48 Hz is not a permanent gameplay requirement. Do not inject recorded Hype
deltas or modify clocks just to obtain a passing timing comparison.

Functional scenarios predeclare coarse outcome bounds and record timing gaps;
passing them does not accept timing parity. Precision scenarios predeclare
per-outcome millisecond targets/tolerances and uncertainty bounds from source
semantics and repeatable baselines. Freeze them before assessing Godot. A timing
mismatch then fails even when frame counts and final positions match. If
environment noise cannot be bounded enough for a precision target, report that
timing assertion as unavailable and keep its required parity job non-green.

## Comparing assets with a loaded game

Define named comparison profiles for a particular executable, level, loading
checkpoint, save and scope. Start with `manoir` and `astrolabe`, covering the
controlled actor, a shared family, one static model and a small dependency graph.
Use a verified loader checkpoint for load-state assertions. If only settled
gameplay can be reached, restrict assertions accordingly instead of pretending
the game still has its file-initial state.

Each selected field declares its comparison rule:

| Field class | Required comparison |
| --- | --- |
| Invariant content | Exact values/float bits where meaningful: topology, state/action definitions, designer definitions, shared references |
| Loader-derived | Explicit source-backed transformation: relocation, active-level Fix placements, resolved handles; compare semantic targets |
| Save/initialization dependent | Compare against a named fixture or established initialization contract; preserve file/default, initial and current values separately |
| Mutable runtime state | Compare only at a stated checkpoint or as a trace; not against arbitrary file defaults |
| Engine-specific derived values | Document coordinate/unit/tolerance rules; retain raw values beside normalized observations |
| Unknown/unavailable | Report coverage gaps and reasons; never turn omissions into matches |

Run both paths independently: OpenSpace -> useful model -> live comparison, and
OpenSpace -> saved Rete -> closed/reopened store -> live comparison. Also support
Rete-only comparison without consulting source files for the static side. Compare
relationships and sharing, including equal-valued distinct objects, not only
flattened values. Missing/extra entities and unmatched references appear in the
report even when the matching subset passes.

A deliberate Rete edit should produce an expected mismatch against the unchanged
original game. Agreement is expected only after a separately verified generated
package has been loaded, or when comparing the deliberately edited Godot port
against the specified changed behavior. Never patch the original's memory to
conceal a conversion mismatch.

Report per-field expected/actual values, policy, identity mapping and evidence,
plus counts of matched, different, unmatched, unavailable and unsupported items.
Shared decoder agreement can hide a shared bug: retain independent known-byte
fixtures, engine/decompilation facts, Raymap checks where applicable, and selected
raw-memory corroboration. Do not build a second production decoder as an oracle.

## Conversion tests and two kinds of coverage

Keep focused unit tests separate from expensive whole-project export checks.
Refactor existing large tests while preserving their assertions and traceability.
Use synthetic, distributable fixtures for routine tests and local corpus fixtures
for representative/complete data coverage. Round trips alone are insufficient:
assert known meaning, independent expected encodings, rejection paths and the
effect of deliberate edits.

| Domain | Required focused tests |
| --- | --- |
| Source access | Forward/shared/null references; Fix-to-level A/B/A rebinding; relocated live pointer equivalents; bounds, cycles, truncated records and stale captures |
| Geometry/material/media | Known vertices/indices/radiosity, texture palette/mips/alpha, sound precision/loops; equal values with distinct identity; resize/reference edits and supported reconstruction |
| Transforms/animation | Compact forms, signs/units/axis conversion, hierarchy/object changes, shared motion, state timing, loops and editor seeking; independent expected samples |
| Characters/collision | Family/state ownership, configuration reset tables, zone activation, primitive dimensions, edited selections and native contact |
| AI/value conversion | Typed expressions/values/arrays/references, defaults versus current state, malformed/unsupported nodes, known source bytes and edited AST effects |
| Rete | Real save/close/reopen, schema/kind/path validation, exact value preservation, dependency closure, sharing, rename collisions and source-free export |
| Godot | Native saved scene/resource inspection before gameplay, compiled script parsing/execution, direct versus Rete routes, authored-file preservation and edited output effects |
| Comparison engine | Independent expected mismatches, missing/extra items, ambiguous identity, units/tolerances, unsupported fields and changed fixture/build rejection |

Add generated/property cases for bounded counts, reference graphs and legal
edits where they can expose cases hand-picked fixtures miss. Persist minimized
regressions. Fuzzing is an optional extended job, not the daily feedback loop.
Small deliberate faults must prove the assertions catch removed geometry,
broken references, ignored edits, incorrect coordinate conversion and no-op
behavior. Expected values must not be computed with the same conversion under
test.

**Code coverage:** use `cargo-llvm-cov` for Rust line, region and function
coverage, reporting per adapter/domain and changed code. Pin the tool/toolchain
after verifying compatibility. Collect synthetic and corpus/runtime profiles
separately, then merge only compatible builds into an explicitly labelled full
report. Exclude dependencies/generated artifacts with a reviewed exclusion list,
not difficult handwritten code. Branch coverage is optional until supported by
the pinned toolchain; never label line coverage as branch coverage. Report covered
and total counts, executable/source revision, selected tests and exclusions.

Emit machine-readable JSON/LCOV plus a browsable HTML report under ignored
`TestResults/coverage/`, with source and instrumentation provenance. First
measure the baseline, then ratchet module/changed-code targets; do not
invent a current percentage. Every new conversion branch needs positive and
relevant negative/edited-value assertions. Missing instrumented coverage is
unknown, not zero or a pass. Rust coverage does not measure GDScript or the
proprietary executable: report generated-script/support execution and gameplay
scenario coverage separately. A high percentage is not proof of correct meaning.

**Level-data coverage:** produce an inventory and accounting report per level,
shared Fix package and media package, keyed by fixture hashes. Keep inventory
discovery independent of which assets successfully decode. Unknown roots,
allocations and unclassified regions stay in the denominator/accounting, with
incomplete discovery stated explicitly. Never call the successfully loaded
subset the whole level.

Record separate dimensions rather than one inflated percentage:

- Byte accounting: union of decompressed used source spans classified as
  semantically decoded, explicitly opaque, framing/allocator data, or unexplained.
  Record compressed physical sizes separately; overlapping reads count once.
  Reading bytes is not semantic coverage, and opaque preservation does not count
  as understood content.
- Content: discovered assets/records/fields and relationships by kind; decoded,
  unsupported and unresolved counts; root/dependency reachability. Separate
  unique shared assets from their instance counts and per-level Fix use.
- Pipeline: which inventoried items are retained through Rete, reconstructable,
  exported natively to Godot, matched to live instances, and actually compared.
  Track AI opcode/type occurrences and unique variants independently of compiled
  behavior support. Report media in their own units as well as dependency counts.
- Fidelity: semantic equality, supported byte reconstruction, native resource
  checks and behavioral evidence are distinct results. Mark non-applicable stages
  explicitly, with reasons.

Publish covered/total counts and ratios such as semantic bytes divided by total
decompressed used bytes, decoded objects divided by inventoried objects per
kind, and compared fields divided by eligible fields for a named comparison
profile. Every ratio includes its scope, exclusions and inventory-completeness
flag. A partial inventory yields a scoped metric, never an asserted whole-level
percentage. Store these in versioned local JSON with human-readable summaries;
coverage decreases identify the newly unsupported/missing items rather than
only changing an aggregate number.

Coverage events attach source spans/record identities to decoder facts in a
transient audit collector. Semantic support diagnostics remain portable through
Rete; original addresses/layout maps remain local reports. A Rete-only run can
report its asset closure coverage; it cannot reconstruct whole-source byte
coverage without a matching local audit manifest. Do not introduce layout caches
into Rete for metrics. Synthetic overlap/gap/alias tests verify the accounting.

## Gherkin with shared step definitions

Use Cucumber for Rust in the independent comparison runner, with one versioned set of
`.feature` files and **one shared step registry**. Step definitions or their shared
helpers own the literal physical-input recordings. Selecting a backend changes
setup, delivery, observation and cleanup, not the meaning or implementation of
the player action. The first functional path may use available original-game
state and screenshots with explicit observation limits. Structured live-memory
values are added when a claim needs them; Godot observations query actual
scene/physics state through test commands.
Shared assertions compare normalized observations with independently sourced
expectations. Do not copy expected results from the other backend at assertion
time or duplicate the step definitions under engine-specific registrations.

Feature files describe player intent and observable outcomes. Exact positions,
keys, input-update counts, mana amounts, state IDs, fixture mappings and
tolerances belong in reviewed step definitions or typed fixture data owned by
those definitions.
Those values have sources and are emitted into failure reports. Avoid a generic
"execute command/assert field" Gherkin vocabulary and backend-specific feature
copies. Product outcomes stay explicit even when their numeric realization is
hidden from the feature text.

Example specification for the first feature (not yet an executable test):

```gherkin
@movement @functional
Feature: Moving through the world
  Background:
    Given Hype is ready to move in a clear part of the manor

  Scenario: Walking and stopping
    When the player walks forward and releases the movement control
    Then Hype moves forward while remaining on the floor
    And Hype returns to standing without continuing to drift

  Scenario: Running instead of walking
    When the player walks and runs over comparable clear ground
    Then running moves Hype faster than walking

  Scenario: A wall blocks movement
    Given a solid wall is ahead of Hype
    When the player keeps walking toward the wall
    Then the wall stops Hype from passing through

  Scenario: Turning while standing
    When the player turns and releases the turning control
    Then Hype faces the new direction without walking away
```

The first walking step emits a declared press/hold/release recording, samples
through release and settling, and retains the trace. Its functional profile
uses controlled elapsed-time holds and broad outcome bounds. A later precision
profile uses verified input-update ordinals and independent millisecond targets;
it does not silently replace the functional schedule during a run.
Steps contribute to the scenario recording and its observation checkpoints;
they cannot run a tape, wait arbitrarily for an assertion, then start another
tape while calling the combined inputs identical. Report assertion success only
after its recorded evidence has been evaluated. The standing assertion checks
both action/visible pose and displacement within a fixed observation window.
The run comparison excludes start/stop transients and uses comparable reset conditions;
the wall check uses signed clearance/contact and a rendered corroboration, not
just an engine-reported flag. Chosen constants and tolerances must be established
from source evidence and repeated original runs, not tuned until Godot passes.

The original adapter uses Orrery with the supplied save and a declared input
profile. Its live control capabilities are verified; mark each scenario's
OpenSpace adapter integration pending until that scenario runs through the shared
registry, evaluates its assertions and reports cleanup. Orrery's narrow 18-update
replay is component evidence; it does not prove arbitrary scenario delivery.
Structured live loading is added for assertions that require it.
The Godot adapter uses the generated
project, native input dispatch and observed scene/physics state. A test-only
endpoint may deliver input and expose observations; it must not implement player
logic, force the expected action in a When step, or manufacture success events. Test setup may supply
explicit save-dependent initial state while full save import is unavailable;
verify equivalence and label that limitation. Initialization behavior itself must
be exercised without bypassing it when it is the subject of the scenario.
Any original-game positioning/navigation intervention is confined to documented
Given setup, logged and verified; it cannot repair converted data or pre-produce
the outcome asserted by Then. A fixture which disables required reflex or
initialization behavior is component evidence, not full gameplay acceptance.

Run each scenario's shared recording against the selected backend or backends.
Evaluate and report each backend independently. When cross-backend comparison
is selected, compare independently obtained normalized traces, real-time
measurements and selected images. Each backend must also satisfy the scenario's
own assertions;
two identically broken outcomes must not pass parity. Normalize coordinates,
units and semantic action identities in the comparison layer, preserving raw
observations. Pair tape events by their shared event IDs, retaining each
engine's actual delivery times and available counters. Precision runs also pair
verified relative consumption ordinals. Compare semantic outcomes against the
declared input origin; do not align away timing errors or assume equal absolute
frame numbers. Use predeclared bounds appropriate to the evidence level, record
sample gaps and clock uncertainty, and fail/invalidate insufficient evidence.
Sampling an unseen apex cannot prove an exact jump height. Repeated runs
characterize nondeterminism; automatic retries do not turn an intermittent
failure into an unqualified pass.

Keep each scenario's observations, assertions and report scoped to that scenario.
For a selected group, reuse one running game only when the next Given verifies
the compatible state left by the preceding passed scenario; otherwise start
from its declared fixture. The movement group runs turn then walk, with a
checked handoff and one final process cleanup. Cucumber Rust 0.23 has
scenario hooks but no feature-level hooks, so an outer runner owns this optional
session and invokes each Cucumber scenario in explicit order. Serialize scenarios
sharing one original-game installation; parallelize independent offline tests only.
Undefined/ambiguous steps, pending required scenarios, empty selections, failed
setup and missing required engines fail the selected job. Explicitly unselected
capabilities appear as not run. Lightweight hooks own technical lifecycle; Given
steps own meaningful game setup. Always release inputs and close owned processes
after failed steps. Save fixture/executable hashes, loaded Godot manifest/support
version, recording/hash and available consumption acknowledgments, event and
elapsed-time timelines,
intervention log, traces, mismatch report and linked screenshots; retain compact
summaries on success and richer evidence on failure.

Order later features after movement: jump/landing, sword/crossbow interaction,
magick resource cost and projectile lifecycle, then selected world interactions.
Add negative cases such as blocked movement, insufficient mana, interrupted
actions and invalid targets. Keep component fixtures for localization; they do
not replace end-to-end scenarios through the real exported character and AI.

## Screenshots and pixel comparison

Visual comparison is a planned assertion path with its own coverage and result,
not merely an optional image attached to a memory mismatch. Keep two uses clear:

- Asset/pose checks use an explicitly matched camera, pose, resolution and render
  profile. They help localize conversion/material/animation differences but do
  not establish gameplay or natural-camera parity.
- Behavior checks capture the scene reached by the shared inputs, using the
  game's normal camera. Camera movement and framing are part of the result;
  forcing Godot's camera to the original's observed pose would conceal errors.

Each visual checkpoint declares its input ordinal or observable milestone,
simulation observation point and completed-render/capture relationship. Record
the producing state/tick where observable, image acquisition interval and
uncertainty. Presentation can lag simulation; an MCP screenshot response time
does not identify its game state. Prove capture association on both engines
before claiming frame-matched pixels. If exact association is unavailable, use
an explicitly bounded settled-state checkpoint for that assertion or report it
unavailable. This is a declared visual profile, not an input-timing fallback.
Do not search neighboring frames for the best match and discard the timing error.

Pin viewport/content rectangle, scaling, aspect ratio, color space, renderer,
lighting/quality settings and relevant UI/locale for the profile. Preserve raw
images; document any color conversion or crop used for comparison. Compare at
the declared native/content resolution. Whole-image exact pixel equality is a
useful measured target, but not a universal gate across different renderers.
Profiles specify pixel error thresholds, allowed differing-pixel fraction and
regions, with any secondary perceptual/edge metric labelled separately. Freeze
thresholds from known intended equivalence and same-engine repeatability, not
from whatever makes the port pass. Every exclusion/mask has a reason and is
visible in the report; excluded pixels count toward reported visual gaps.
Never introduce geometric warping, adaptive alignment or broad masks to hide
camera, placement, missing-object or animation errors.

Retain both source images, difference/heat map, optional overlay and per-region
metrics, with tape/fixture/build identifiers and state/time metadata. Check
missing geometry, wrong pose/camera, a one-update-late frame and altered pixels
as negative controls; keep raw-image capture independent of the assertions it
supports. A stale, torn or unassociated required capture cannot pass. Measure
capture overhead as part of timing qualification; blocking memory snapshots or
slow readback must not silently distort the real-time run. Initially validate
one synchronized or bounded-settled movement checkpoint before expanding visual
coverage to every scenario.

## Test selection and timing policy

The former universal ten-minute whole-suite ceiling is removed. Individual tests
and active development areas must remain bounded and reasonably fast. A final
release suite can contain many tests and take longer; it still has explicit job
deadlines, interruption handling, process cleanup and complete reporting.

Extend the existing named suites, checked-in test manifest and pinned
`cargo-nextest` runner with scenario/backend prerequisites, fixture and budget.
Retain isolated Rust test execution, filtering and per-test deadlines. Supervise
custom Cucumber binaries and external engines explicitly too;
libtest/nextest timeouts alone do not guarantee descendant cleanup. The current
Python `tools/test.py` outer supervisor is independent of the older Python Hype
MCP harness and remains in use.
The implemented runner uses explicit per-selection budgets and pinned nextest
deadlines; it has replaced the historical 570-second limit.

Initial targets to measure and revise with evidence, not existing guarantees:

| Selection | Target and gate |
| --- | --- |
| Unit test | Normally well below one second; investigate above five seconds |
| Resource/integration test | Usually below 30 seconds; declared per-test hard deadline |
| Live BDD scenario | Bounded setup/action/observation/cleanup, normally below 90 seconds; shared startup measured separately |
| Mandatory safeguard suite | Aim for under one minute warm; hard deadline and no silent filtering |
| One development area plus safeguards | Aim for under three minutes warm; investigate above five minutes |
| Release suite | Explicit larger job budget based on inventory; no universal ten-minute cap and no unlimited individual tests |

Measure wall time including build/startup/teardown and report those stages
separately so warm-cache speed does not hide cold setup. Coverage collection can
have a separate budget; it must not silently replace the ordinary test results.
Split monolithic corpus tests into independently reported asset/level cases with
bounded execution and reusable immutable setup. Preserve all assertions and
release membership. Do not speed up by removing difficult fixtures.

Development runs select an area (source/Rete/media/scene/animation/AI/movement/
runtime/comparison) plus affected dependencies and mandatory safeguards. Release
runs enumerate all declared suites/corpus/scenarios on supported environments.
Print selected and excluded tests with reasons, record counts and reject new
unclassified tests. Selection changes and lowered coverage/budgets are reviewed
changes, never a shortcut to a green run. Missing local proprietary data is a
reported unavailable capability, not a release-parity pass.

Mandatory safeguards apply at every development validation checkpoint, rather
than running the whole suite after each edit. Build these from small existing
fixtures and explicit negative controls:

- Source-free Rete reopening/export and a deliberate edit that changes output.
- Asset identity/sharing, schema/support compatibility and unsupported diagnostics.
- Saved native scene/resources before preview/gameplay startup; production support
  used by test probes; no runtime source-table/AI interpreter substituted.
- Authored-content regeneration protection and manifest provenance.
- Test discovery/selection integrity, deadline cleanup and evidence status rules.
- When the gameplay path is affected, a small actual-input generated-code smoke
  check that catches forced-state/no-op shortcuts. This does not require booting
  the original game on every unrelated edit; fresh parity runs are required when
  claiming a behavior slice complete.

Architectural review complements these tests: textual checks cannot prove that
compiled GDScript has avoided recreating an engine. Testkit observations must
never become a production gameplay dependency.

## Ordered delivery and exit evidence

1. **Checkpoint and policy.** Preserve gameplay code/tests, record fresh results
   and open limitations, commit this plan separately. No gameplay completion or
   new coverage percentage is asserted by creating the checkpoint.
2. **Make the suite runnable.** Extend the existing `tools/test.py` supervision
   and companion Rust package with one suite entry point. Register the top-tier
   data and behavior comparison groups, scenario selection, backend contracts,
   recording, observation, per-backend assertions, optional cross-backend
   comparison, evidence reports and cleanup. Keep these contracts provisional
   until a real scenario tests them. Run a synthetic scenario through selection,
   one backend's pass/fail assertions, a second backend, a deliberate mismatch,
   comparison, reporting and cleanup. Classify the first development
   area's tests and mandatory safeguards; keep a full inventory of unclassified
   tests visible and required for release. Exit: one documented command produces
   a useful report, the suite can demonstrate a detected mismatch, timeout and
   cleanup, and unrelated expensive tests are excluded without omitting
   safeguards. This proves the suite plumbing, not Hype behavior.
3. **Place current-tier placeholders.** Inventory the existing controller and
   selected data-comparison capabilities, then represent their missing suite
   integration, Godot player delivery, generalized completed-update observation,
   timing and screenshot comparison as explicit pending capabilities. Preserve
   implemented component status separately from scenario acceptance.
   Selection and reporting show what is implemented, pending, failed and not run.
   A selected required case that is pending fails its acceptance job; the
   synthetic framework demonstration has its own result. Do not enumerate every
   Hype behavior or add success-shaped stubs for individual AI calls. Exit: the
   suite remains runnable and demoable while each missing top-tier path has an
   owner, dependency and observable completion check.
4. **Fill the first functional path.** Add one shared BDD movement step registry
   to the suite. Connect Orrery's verified live launch, supplied manor fixture,
   physical input, typed observation, screenshot and cleanup capabilities to the
   OpenSpace adapter, which remains pending until the scenario and assertions run.
   First make a turn–release–idle scenario pass on OpenSpace alone: assert a
   heading change after physical Left input and a settled idle state after
   release, with the existing completed-update action trace as supporting
   evidence where the scenario needs it. Then
   make a press–move–release–stop scenario pass there through the same backend.
   These are real behavior-suite milestones; Godot need not run for either
   OpenSpace result. Register Godot's missing player path as
   pending in the same scenario; do not build a separate rule probe. Then use
   that case to replace the current all-or-nothing AI import gate: ordinary
   `manoir` conversion must retain Hype's AI model and
   Brain with stable behavior/rule/designer-field identities, save its node/script,
   and represent unsupported siblings explicitly. The current source-rule probes
   add a test-created Brain after character instantiation. Compile the bounded
   player path and connect its action choice to native animation, movement and contact.
   Exercise direct and closed/reopened Rete conversion, then run the same scenario
   through normal Godot input. Record event delivery, elapsed time, actor/action
   outcomes and a screenshot for each selected backend.
   Intermediate exit: the OpenSpace scenario passes independently, the suite
   exposes the checkpoint's missing Godot player behavior, then the same case
   through compiled player AI produces a visible result in the saved scene.
   An explicit validated start
   fixture may supply state for this first case; it cannot count as full player
   initialization. The test must not insert a Brain or substitute its script.
   Report approximate timing and visual gaps. Mark the movement
   group demonstrated for this bounded case with its other children pending.
   One-tick precision replay and a complete live object reader are prerequisites
   only when this observation would otherwise be ambiguous.
5. **Prove and revise the framework.** Check broad displacement, action order,
   contact and stopping against independent original-game observations. Verify
   that a deliberate Rete AI edit changes the Godot result and that a no-op
   controller, late input or wrong image is detected at the claim it affects.
   OpenSpace and Godot runs must each satisfy their own functional assertions;
   two identically broken outcomes do not pass. Review scenario, backend,
   observation, ownership and report contracts against this real path, then fix
   them before copying the pattern. Exit: the first functional movement case has
   a repeatable demonstration with measured, named discrepancies. No setup or test endpoint
   substitutes for generated AI or the missing player controller.
6. **Tighten precision where required.** If event timing changes the decision,
   contact or action order, prove the necessary input-consumer and observation
   boundary before accepting that functional case. Continue to sibling first
   passes after the functional proof. Develop precision replay when a selected
   claim or dependency requires it: one-tick presses/releases, consecutive Godot
   catch-up ticks, render stalls, native frame-indexed replay, completed-update
   live observation and independent measured-millisecond assertions. Reuse
   selected source declarations against files and synthetic relocated memory;
   load the narrow Hype transform, action/behavior and designer values without
   caller addresses. Check a slow intervening assertion, stale handles,
   unmodified clocks, duplicate/shared/dynamic identity, cleanup and capture
   provenance. Exit: precise event acknowledgments and source/Godot timing
   reports support the specific timing claim. A failed boundary experiment
   changes that contract; it never silently downgrades a precision result.
   Completion of this capability does not gate unrelated functional work.
7. **Fill the validated groups.** This work may proceed while step 6 remains
   pending for other claims. Extend focused data profiles to a shared
   family, static model/dependency graph and `astrolabe`, with independently
   accounted code and level-data coverage. Complete affected-area selection and
   release membership reporting; show matched/gap counts, sharing and
   save-dependent differences. Demonstrate representative actual-input walk/run
   with contact before opening jump/landing, interaction, sword/crossbow and
   magick at the group tier. Keep the remaining movement and initialization cases
   visible with revisit triggers; the full movement-group exit is not a gate for
   every sibling first pass. For each, keep the suite runnable, choose a
   representative functional case, prove it and adjust shared contracts before
   filling its remaining cases. Tighten precision when that case needs it.
   Do not create a per-behavior stub catalogue in advance.
8. **Broaden and package.** Expand level/domain/scenario inventory, validate a
   Windows backend when available, publish optional inspection tooling, and run
   the complete release matrix. Grow Orrery's live mana, sequence, screenshot,
   failure/cleanup and precision evidence as selected scenarios require; keep
   installations/evidence local. Full source-package
   fidelity remains its own existing milestone.

The next implementation task is steps 2–3: a runnable, report-producing suite
with current-tier placeholders. Partial AI import, a saved player Brain and the
first functional movement case then fill and challenge that framework. Keep the
suite buildable and demoable after every step, with functional behavior, accepted
precision and pending work reported separately.
Complete that path before a general live object browser, broad engine
introspection or exhaustive metrics infrastructure. No human-effort estimates
are part of this plan; progress is judged by executable exit evidence.

### Design review: decisions that become expensive to reverse

Independent review on 2026-09-20 supported the adapter/dependency direction and
shared-input design, and identified four contracts to prove before accepting a
result that depends on them. They need not all precede the first functional
movement demonstration:

| Contract | Likely regret if fixed too early | Evidence before the dependent claim |
| --- | --- | --- |
| Input-consumer boundary and cadence | Fixtures depend on one-render/one-physics-tick behavior or an unjustified clock correspondence | Actual character/AI edge and catch-up tests; original mechanics cadence investigation; independent real-time controls |
| Complete scenario chronology | Identical step tapes acquire different gaps through assertion/controller latency | One scenario recording; a slow intervening assertion leaves declared event times or ordinals unchanged |
| Observation phase | Mid-update tuples become the behavior oracle and generate false failures | Verified completed-update publication; partial-write negative control |
| Source bounds and cross-source identity | Offline allocation assumptions leak into live handles or persisted assets | Distinct equal-valued objects, sharing and replacement across offline/Rete/live |

These gates reduce the cost of revising integration details; writing them down
does not establish their results. In particular, exact input counts do not yet
prove a useful real-time gameplay comparison profile.

## Tool references

- [Gherkin reference](https://cucumber.io/docs/gherkin/reference/): high-level
  scenarios with implementation details in step definitions.
- [Cucumber Rust](https://cucumber-rs.github.io/cucumber/current/) and
  [scenario hooks](https://cucumber-rs.github.io/cucumber/current/writing/hooks.html).
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov): Rust source coverage.
- [cargo-nextest](https://github.com/nextest-rs/nextest): the pinned isolated Rust
  test runner already used by the supervisor.
- [process_vm_readv](https://man7.org/linux/man-pages/man2/process_vm_readv.2.html),
  [Windows ReadProcessMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory)
  and [VirtualQueryEx](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex).

References describe the tools used or planned; they do not establish completed
scenario coverage. Existing commands and evidence remain in [testing.md](../docs/astrolabe/testing.md).
