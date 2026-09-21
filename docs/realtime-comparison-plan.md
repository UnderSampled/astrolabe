# Live loading, comparison, and behavior testing

Implementation plan, updated 2026-09-20 after the live input-timing experiment.
The gameplay checkpoint is preserved. The actual Godot input-boundary probe,
focused test runner and initial Rust coverage collection are implemented; see
[testing evidence](testing.md). The native live loader and BDD runners remain
in progress. Resume the
[gameplay plan](godot-gameplay-plan.md) after the first useful comparison slice;
do not wait for complete engine or level coverage.

## Outcomes and boundaries

There are two independently reported comparisons:

1. **Data:** OpenSpace files or reopened Rete assets against structured values
   read from the running original game. Establish asset identity, relationships,
   loaded values, and the reasons particular fields should differ.
2. **Behavior:** shared Gherkin steps produce the same literal input recording
   for the original game and generated Godot port. Engine adapters deliver those
   events and independently observe their effects. Report gameplay/state,
   measured elapsed milliseconds and screenshots/pixel matching separately.

Neither comparison implies the other. Matching meshes or action definitions
does not prove movement; matching an observed movement does not prove complete
asset conversion. Acceptance of reconstructed files by the original game is a
third, later result requiring the generated files actually to be installed and
loaded in a test copy. Do not claim it from either comparison above.

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
at the old global cap. The [checkpoint evidence](hype-control-progress.md#paused-implementation-checkpoint--2026-09-20)
records the exact limits. The initial planning/policy commit is **`4732ed7`**;
this update records the subsequent timing decision and shared-step design.

The implementation and tests are preserved separately from planning/policy. The
checkpoint changes to model types, codecs, Rete schema, exporter, native support,
and their tests are
interdependent; do not manufacture independently buildable feature commits by
arbitrarily splitting those files. Keep the validation-profile/runner change
with the tests it supports. Record exact versions, verification and open work in
[Hype control progress](hype-control-progress.md).

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

The [implementation handoff](realtime-comparison-handoff.md) records the named
recovery tag, preserved local evidence and a goal for the next agent. Start with
the boundary/cadence proof and harness work in the ordered delivery below. Once
the new movement scenarios exist, run them against this gameplay checkpoint and
expose its actual failures; then complete the missing gameplay path without
replacing the real converted AI with a test controller. Do not resume the older
archived runtime experiment.

## Native runtime and shared loading

Use a companion Rust workspace package, provisionally
`crates/astrolabe-testkit`, depending on the Astrolabe library. It owns native
game control, comparison reports, CLI/MCP entry points and BDD tests. Keep the
root converter as the default workspace member. The converter must not acquire
a dependency back on the testkit; cross-engine tests live in the companion
package. Packaging names can change without changing this boundary.

The testkit process launches/owns the original game and links Astrolabe directly.
Prepared tests instantiate that same runtime session; the persistent MCP binary
owns a session for agent queries. No Python memory broker or internal per-record
RPC is needed. MCP is only the agent-facing surface. Retain the existing Python
controller as a migration reference until native control passes its acceptance
cases; then retire the replaced production path explicitly.

Suggested ownership:

| Concern | Owner |
| --- | --- |
| Hype record declarations, build-specific roots, field interpretation | `src/openspace`, sharing existing binrw declarations and codecs |
| Offline blocks/relocations and live pointer resolution | Separate source implementations inside OpenSpace |
| Process handle, memory reads/maps, pause guard, launch/input/capture/cleanup | Companion runtime, with OS-specific modules |
| Assets, transforms, typed designer values and semantic references | Existing `model` types |
| Session handles, snapshots, comparison policy, traces and evidence | Testkit observations/reports; reuse model values without creating another asset model |
| Saved native Godot resources and compiled AI | Existing Godot exporter and maintained support |
| Optional Godot observation/control endpoint | Test support operating on those same exported resources |

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
- Behavioral observations need a separately verified completion/publication
  boundary for their fields and an associated input ordinal. An unchanged frame
  counter is insufficient if it increments at update start. Copy the bounded
  selected values at that boundary, or establish equivalent evidence that they
  represent a completed update and no writer can change them during observation.
  Writer quiescence alone is insufficient. Preserve phase/consistency in
  the report; do not silently promote an inspection snapshot into a behavior
  assertion. A fake producer stopped between related field writes must fail
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

Linux/Wine is the first native target, using the validated existing harness.
Port process ownership/subreaping, Gamescope/X11 input and capture, focus
handling, and bounded teardown from the
existing harness without reinstalling its prepared game. Windows is a separate
backend for process memory/control, not a restriction on the shared decoder.
Native Windows launch, input, capture and snapshot consistency need their own
validation; do not claim support from compiling a memory reader alone. Optional
platform dependencies must not prevent ordinary converter builds elsewhere.

## Replay decision: frames drive inputs, milliseconds judge timing

Implement **one frame-indexed physical-input scheduler**. There is no automatic
millisecond scheduling fallback. A missing or unverified frame/input boundary
fails the selected replay capability; it must not silently become a timed OS
keypress. Existing best-effort MCP actions remain useful for setup during the
migration, but do not qualify as frame-exact behavior evidence.

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
game state before stopping the game. **Production uses dedicated owned scratch
memory**, validated hook ownership and bounded cleanup, never a game string or
an address copied from the experiment.

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
observation phases before interpreting the earlier keyboard probe as a full
gameplay replay contract.

Consequently, do not label the original input counter a fixed physics clock or
assume its timestep equals Godot's. Use the original consume-update boundary
and a verified native Godot consumer boundary to index the shared recording;
qualify their real-time cadence and preserve both engines' actual deltas.
Choosing Godot's native physics rate for a comparison profile is still a required
experiment, not permission to replay original deltas or recreate its scheduler.

### One shared recording and two delivery adapters

Shared steps/helpers build a versioned recording of literal physical events:
relative input-update ordinal, ordered press/release events, physical key/device
identity and relevant modifiers. Bind both fixtures to the same controls;
platform-specific key-code translation preserves the physical event. Keep
repeats, simultaneous events, initial held state and final release explicit.
Start with keyboard support; do not claim mouse/controller equivalence yet.
Compose the steps into **one complete scenario recording**, with one ready-state
epoch, explicit neutral gaps and observation/settling windows. Compile and hash
that recording once before playback; both backends consume those same bytes.
Assertions inspect declared recorded checkpoints without delaying later inputs.
Step dispatch, MCP latency, screenshots and assertion execution must not define
gaps in the recording. A two-action test with a deliberately slow intervening
assertion must still deliver identical event ordinals on both backends. A reset
inside a scenario is an explicit new setup epoch, not an unrecorded delay.
Do not change keys, hold length or release time in response to either engine's
progress. A missed boundary, duplicate event or replay buffer underrun is a
failure, not permission to catch up by compressing several events into one tick.

The original adapter queues the tape locally and applies it at the verified
physical keyboard sampling boundary, leaving original bindings, edge/history
processing and player AI responsible for the outcome. The Godot adapter supplies
equivalent physical `InputEvent`s through normal input dispatch before the
intended consumer tick; it does not call player actions or set movement state.
Do not equate render and physics ticks or introduce an OpenSpace scheduler into
Godot to obtain matching counters. Prove the ordering of normal Godot input,
generated AI, movement and observation before fixing the adapter contract.

Use bounded local queues and batched trace reads instead of one MCP transaction
per frame. Acknowledgments record where each event was actually consumed, not
when the controller requested it. Validate press/release edges, one-tick pulses,
chords, repeat policy, slow renders and multiple physics ticks per render; also
test pause/reload invalidation, buffer exhaustion, failed hook installation and
worker death. Restore callbacks only while ownership still matches; do not free
scratch memory while a callback can execute. An outer owner must release inputs
and safely detach or stop its game after a failed worker. Bound the cost and
capacity of instrumentation; missing/overwritten observations invalidate the
affected assertion.

Keep the recording schema and backend tick contract provisional until those
cases pass. The scheduling choice is settled; unproved boundary details are
implementation acceptance gates, not alternate millisecond execution modes.

### Independent real-time comparison

For each run retain three timelines: requested/consumed replay ordinals, engine
simulation counters/deltas, and measured monotonic elapsed milliseconds. Record
dispatch, consumption and observable outcome times with sampling uncertainty.
Relate any engine clock to the host monotonic clock with measured uncertainty;
handle resets/wrap explicitly. Never calculate measured milliseconds as
`frames / FPS`, use a pause-adjusted game timer as wall time, or count an
accelerated `--fixed-fps` run as real-time parity evidence.

Compare input hold duration and input-to-outcome latency (movement onset,
stopping, action transitions, contact), as well as the state trajectory and
visual milestones. Same input ordinals make the stimulus reproducible; they do
not guarantee equal simulation time, physics results or elapsed time. Keep both
event ordinal and elapsed time in reports. Semantic labels may identify matching
events, but must not shift/stretch timelines to conceal a late result.

Qualify baseline cadence with repeated original runs and instrumented versus
minimally observed controls before accepting tolerances. Record actual update/
render intervals, configured caps, stalls and observation overhead; do not
silently retry away slow runs. A documented environment/cadence failure is
inconclusive or failed, never a parity pass. Choose the first Godot test cadence
from this evidence and validate its ordinary native update behavior; the probe's
48 Hz is not a permanent gameplay requirement. Do not inject recorded Hype
deltas or modify clocks just to obtain a passing timing comparison.

Predeclare per-outcome millisecond targets/tolerances and uncertainty bounds
from source semantics and repeatable baselines. Freeze them before assessing
Godot. A timing mismatch fails even when frame counts and final positions match.
If environment noise cannot be bounded enough for a particular target, report
that timing assertion as unavailable and keep the required parity job non-green.

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

Use Cucumber for Rust in the companion package, with one versioned set of
`.feature` files and **one shared step registry**. Step definitions or their shared
helpers own the literal physical-input recordings. Selecting a backend changes
setup, delivery, observation and cleanup, not the meaning or implementation of
the player action. Original observations decode structured process memory;
Godot observations query actual scene/physics state through test commands.
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
@movement @parity
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

The walking step emits a fixed frame-indexed press/hold/release recording,
samples through release and settling, and retains the trace. Millisecond outcome
targets are separate fixture values; they do not schedule a second input path.
Steps contribute to the scenario recording and its observation checkpoints;
they cannot run a tape, wait arbitrarily for an assertion, then start another
tape while calling the combined inputs identical. Report assertion success only
after its recorded evidence has been evaluated. The standing assertion checks
both action/visible pose and displacement within a fixed observation window.
The run comparison excludes start/stop transients and uses comparable reset conditions;
the wall check uses signed clearance/contact and a rendered corroboration, not
just an engine-reported flag. Chosen constants and tolerances must be established
from source evidence and repeated original runs, not tuned until Godot passes.

The original adapter uses the native controller, supplied save, verified physical
input boundary and structured live loader. The Godot adapter uses the generated
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

Run each scenario's shared recording against each backend, then compare
independently obtained normalized traces, real-time measurements and selected
images. Each backend must also satisfy the scenario's own assertions;
two identically broken outcomes must not pass parity. Normalize coordinates,
units and semantic action identities in the comparison layer, preserving raw
observations. Pair acknowledged tape events using their relative input ordinals,
retaining each engine's absolute counters and elapsed times. Compare semantic
outcomes against that input origin; do not align away timing errors or assume
equal absolute frame numbers. Use predeclared timing/spatial envelopes, record
sample gaps and clock uncertainty, and fail/invalidate insufficient evidence.
Sampling an unseen apex cannot prove an exact jump height. Repeated runs
characterize nondeterminism; automatic retries do not turn an intermittent
failure into an unqualified pass.

Use scenario-scoped state and reset between examples. Serialize scenarios sharing
one original-game installation; parallelize independent offline tests only.
Undefined/ambiguous steps, pending required scenarios, empty selections, failed
setup and missing required engines fail the selected job. Explicitly unselected
capabilities appear as not run. Lightweight hooks own technical lifecycle; Given
steps own meaningful game setup. Always release inputs and close owned processes
after failed steps. Save fixture/executable hashes, loaded Godot manifest/support
version, recording/hash and consumption acknowledgments, all three timelines,
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

Implement named suites and a checked-in test manifest mapping area, kind,
prerequisites, fixture and budget. Prefer `cargo-nextest` for isolated Rust test
execution/filtering/per-test timeouts after evaluating it with existing child
cleanup. Supervise custom Cucumber binaries and external engines explicitly too;
libtest/nextest timeouts alone do not guarantee descendant cleanup. The current
Python outer supervisor remains while the native controller is implemented.
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
2. **Resolve consumer boundary and cadence before infrastructure.** Use the
   original mechanics/phase findings above and verify the selected fixture,
   then exercise the actual
   generated AI/native Godot character with one-tick presses/releases, consecutive
   catch-up ticks and render stalls. Distinguish delivery success from gameplay
   correctness, so the checkpoint's missing behavior need not block the probe.
   Exit: the native integration point and supported execution profile are
   evidenced, with independently measured elapsed time. A failed experiment
   requires an explicit integration decision, not a millisecond fallback or an
   unnoticed constraint that makes only the synthetic probe work.
3. **Minimum focused test foundation.** Classify the first development area's
   tests and mandatory safeguards, add bounded selection/supervision and a full
   inventory that keeps unclassified work visible and required for release.
   Establish code and level-data coverage baselines for the first actor slice.
   Exit: that area runs without unrelated expensive tests, safeguards cannot be
   omitted, and timeout/cleanup negative tests pass. Broad test decomposition
   and all-level inventory work must not delay the first live comparison.
4. **Native replay and narrow live loading.** Reuse selected source declarations
   against files and synthetic relocated memory, then load Hype's transform,
   action/behavior and needed designer values without caller addresses. Port
   enough lifecycle/input/capture to run the supplied manor fixture. Prove the
   scenario recording at both actual consumer boundaries, including a slow
   intervening assertion, cleanup, stale handles and unmodified clocks. Prove
   the duplicate/shared/dynamic identity cases before expanding the reader.
   Exit: replay acknowledgments establish identical inputs; completed-update
   observations reject mid-update tuples; real-time controls characterize
   overhead; one visual checkpoint has a verified capture relationship. Do not
   freeze the tape format or source-reader contract before these checks pass.
5. **First data comparison and shared BDD movement feature.** Compare the narrow
   manor actor/definition slice through direct and closed/reopened Rete routes;
   implement one shared movement step registry and engine observation adapters.
   Replay against the checkpoint Godot port immediately. Exit: original runs
   satisfy their own assertions; missing Godot behavior is reported as failing/
   incomplete; state, elapsed-ms and visual results are independent; deliberate
   Rete edits and no-op/late-input/wrong-image faults are caught. No setup or test
   endpoint substitutes for the missing player controller.
6. **Complete focused reporting around that path.** Extend profiles to a shared
   family, static model/dependency graph and `astrolabe`; integrate code and
   independently accounted level-data coverage. Complete affected-area selection
   and release membership reporting. Exit: real matched/gap counts, sharing and
   save-dependent differences are visible. This broadening is not a prerequisite
   for starting gameplay work once step 5 can reliably expose its failures.
7. **Resume gameplay through red/green comparisons.** Complete initialization and
   actual idle/walk/run/contact until the same feature passes both backends and
   paired state, measured-millisecond and declared visual checks through direct
   and Rete conversions. Only then broaden jump, combat and magick. Add codec/
   coverage tests with each newly used field.
8. **Broaden and package.** Expand level/domain/scenario inventory, validate a
   Windows backend when available, publish optional testkit/inspection tooling,
   and run the complete release matrix. Retire Python control after equivalent
   native mana, sequence serialization, screenshot and failure/cleanup evidence
   and a documented migration; keep installations/evidence local. Full source-
   package fidelity remains its own existing milestone.

The first implementation task is the boundary/cadence proof in step 2, followed
by the minimum foundation and narrow comparisons in steps 3–5. Complete that
path before a general live object browser, broad engine introspection or
exhaustive metrics infrastructure. No human-effort estimates are part of this
plan; progress is judged by the exit evidence above.

### Design review: decisions that become expensive to reverse

Independent review on 2026-09-20 supported the adapter/dependency direction and
shared-input design, and identified four contracts to prove early:

| Contract | Likely regret if fixed too early | Required early evidence |
| --- | --- | --- |
| Input-consumer boundary and cadence | Fixtures depend on one-render/one-physics-tick behavior or an unjustified clock correspondence | Actual character/AI edge and catch-up tests; original mechanics cadence investigation; independent real-time controls |
| Complete scenario chronology | Identical step tapes acquire different gaps through assertion/controller latency | One scenario recording; slow intervening assertion leaves input ordinals unchanged |
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
- [cargo-nextest](https://github.com/nextest-rs/nextest): candidate isolated test
  runner; verify features and pin a version during implementation.
- [process_vm_readv](https://man7.org/linux/man-pages/man2/process_vm_readv.2.html),
  [Windows ReadProcessMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory)
  and [VirtualQueryEx](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex).

These are tool choices to validate, not newly installed dependencies or completed
test infrastructure. Existing commands and evidence remain in [testing.md](testing.md).
