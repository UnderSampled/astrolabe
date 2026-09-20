# Live loading, comparison, and behavior testing

Implementation plan, 2026-09-20. This is not a claim that
the live loader, coverage collection, or BDD runners already exist. Checkpoint
the current gameplay work before implementing this plan. Resume the
[gameplay plan](godot-gameplay-plan.md) after the first useful comparison slice;
do not wait for complete engine or level coverage.

## Outcomes and boundaries

There are two independently reported comparisons:

1. **Data:** OpenSpace files or reopened Rete assets against structured values
   read from the running original game. Establish asset identity, relationships,
   loaded values, and the reasons particular fields should differ.
2. **Behavior:** the same high-level scenario against the original game and the
   generated Godot port. Establish player-visible outcomes, event order, timing,
   movement, and contact using engine-specific controls and observations.

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

Gameplay checkpoint: **`c912db7`**, on `codex/live-comparison-plan`; `main` remains
at the prior committed baseline. The checkpoint builds and passes formatting/
Clippy, but has a known Godot rule-probe timeout and an incomplete full-suite run
at the old global cap. The [checkpoint evidence](hype-control-progress.md#paused-implementation-checkpoint--2026-09-20)
records the exact limits. This planning/policy change is a separate commit.

Preserve current implementation and tests in a dedicated gameplay checkpoint
commit, followed by a separate planning/policy commit. The existing changes to
model types, codecs, Rete schema, exporter, native support, and their tests are
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

Resume from these commits, not from the older archived runtime experiment.
First run the new movement scenarios against the checkpoint and expose its
actual failures; then complete the missing gameplay path without replacing the
real converted AI with a test controller.

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

- A coherent inspection capture briefly pauses the owned process, checks that
  relevant threads are stopped, copies the bounded requested closure, and resumes
  before formatting/reporting. A stopped process may be between engine updates;
  structural validation still applies. A missing page cannot be fetched after
  resume and silently added to the same snapshot.
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

Linux/Wine is the first validated backend. Port process ownership/subreaping,
Gamescope/X11 input and capture, focus handling, and bounded teardown from the
existing harness without reinstalling its prepared game. Windows is a separate
backend for process memory/control, not a restriction on the shared decoder.
Native Windows launch, input, capture and snapshot consistency need their own
validation; do not claim support from compiling a memory reader alone. Optional
platform dependencies must not prevent ordinary converter builds elsewhere.

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

## Gherkin and two step implementations

Use Cucumber for Rust in the companion package, with one versioned set of
`.feature` files, one scenario vocabulary and engine-specific step implementations.
A selected backend supplies the implementation; avoid duplicate registrations
of identical step patterns. Keep shared outcome assertions and normalized
observation types, while the original and Godot modules independently implement
setup, logical input, state observation and cleanup. Do not copy expected results
from the other backend at assertion time.

Feature files describe player intent and observable outcomes. Exact positions,
keys, durations, mana amounts, state IDs, fixture mappings and tolerances belong
in reviewed step definitions or typed fixture data owned by those definitions.
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

The walking step dispatches real input for a documented interval, samples through
release and settling, and retains the trace. The standing assertion checks both
action/visible pose and displacement within a fixed observation window. The run
comparison excludes start/stop transients and uses comparable reset conditions;
the wall check uses signed clearance/contact and a rendered corroboration, not
just an engine-reported flag. Chosen constants and tolerances must be established
from source evidence and repeated original runs, not tuned until Godot passes.

Original-game steps use the native controller, supplied save, logical key mapping
and structured live loader. Godot steps use the generated project, native input
delivery and observed scene/physics state. A test-only endpoint may deliver input
and expose observations; it must not implement player logic, force the expected
action in a When step, or manufacture success events. Test setup may supply
explicit save-dependent initial state while full save import is unavailable;
verify equivalence and label that limitation. Initialization behavior itself must
be exercised without bypassing it when it is the subject of the scenario.
Any original-game positioning/navigation intervention is confined to documented
Given setup, logged and verified; it cannot repair converted data or pre-produce
the outcome asserted by Then. A fixture which disables required reflex or
initialization behavior is component evidence, not full gameplay acceptance.

Run each scenario against each backend, then compare independently obtained
normalized traces. Each backend must also satisfy the scenario's own assertions;
two identically broken outcomes must not pass parity. Normalize coordinates,
units and semantic action identities in the comparison layer, preserving raw
observations. Align by acknowledged input and semantic events, not assumed equal
frame numbers. Use predeclared timing/spatial envelopes, record sample gaps and
clock uncertainty, and fail/invalidate insufficient evidence. Sampling an unseen
apex cannot prove an exact jump height. Repeated runs characterize nondeterminism;
automatic retries do not turn an intermittent failure into an unqualified pass.

Use scenario-scoped state and reset between examples. Serialize scenarios sharing
one original-game installation; parallelize independent offline tests only.
Undefined/ambiguous steps, pending required scenarios, empty selections, failed
setup and missing required engines fail the selected job. Explicitly unselected
capabilities appear as not run. Lightweight hooks own technical lifecycle; Given
steps own meaningful game setup. Always release inputs and close owned processes
after failed steps. Save fixture/executable hashes, loaded Godot manifest/support
version, inputs, times, intervention log, traces, mismatch report and selected
screenshots; retain compact summaries on success and richer evidence on failure.

Order later features after movement: jump/landing, sword/crossbow interaction,
magick resource cost and projectile lifecycle, then selected world interactions.
Add negative cases such as blocked movement, insufficient mana, interrupted
actions and invalid targets. Keep component fixtures for localization; they do
not replace end-to-end scenarios through the real exported character and AI.

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
Python supervisor remains until a tested replacement is ready. Its present
570-second limit is an implementation constraint, not the new project policy.

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
2. **Focused test foundation.** Inventory/classify existing tests, implement area
   selection plus mandatory safeguards, per-test supervision and release mode.
   Establish Rust coverage and level-data inventory baselines. Exit: a selected
   area runs without unrelated expensive tests, safeguards cannot be omitted,
   coverage gaps remain visible, and timeout/cleanup negative tests pass.
3. **Shared source reader and native controller.** First prove file versus
   synthetic relocated-memory decoding, then port lifecycle/input/capture and
   implement bounded live snapshots. Exit: the supplied save loads, Hype's
   transform/action/behavior/designer data is returned without caller addresses,
   reload and process-exit tests reject stale state, and cleanup survives failure.
   Preserve existing mana, sequence serialization and screenshot acceptance.
4. **Static/live data comparison.** Join selected level/Fix assets, apply explicit
   field policies, compare both direct and reopened-Rete routes, and integrate
   coverage reports. Exit: manoir and astrolabe fixtures report real matched/gap
   counts; deliberate Rete edits and broken sharing produce precise mismatches;
   save-dependent differences are explained rather than ignored globally.
5. **First dual-backend BDD feature.** Implement the movement vocabulary and both
   engine step implementations, capture original traces, and run the checkpoint
   Godot port. Exit: real original runs pass their assertions, missing Godot
   behavior is reported as failing/incomplete, known injected faults are caught,
   and no setup or test endpoint substitutes for the missing player controller.
6. **Resume gameplay through red/green comparisons.** Complete initialization and
   actual idle/walk/run/contact until the same feature passes both backends and
   paired trace checks through direct and Rete conversions. Only then broaden
   jump, combat and magick. Add codec/coverage tests with each newly used field.
7. **Broaden and package.** Expand level/domain/scenario inventory, validate a
   Windows backend when available, publish optional testkit/inspection tooling,
   and run the complete release matrix. Retire Python control after equivalent
   native evidence and a documented migration; keep game installations/evidence
   local. Full source-package fidelity remains its own existing milestone.

The first implementation task is step 2, followed by a narrow actor-data slice
of step 3. Complete that useful vertical path before attempting a general live
object browser or broad engine introspection. No human-effort estimates are part
of this plan; progress is judged by the exit evidence above.

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
