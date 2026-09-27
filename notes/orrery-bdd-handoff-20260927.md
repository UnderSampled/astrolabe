# Sol handoff: Orrery recording/replay and external BDD suite

## Assignment

Upgrade Orrery with complete session recording/replay and qualified timing/frame
precision, and restructure the behavior suite into an external consumer with
a separate Godot harness. Deliver a useful running path from recording through
replay to automatic state/API/video comparison, with extensible described
recordings. The purpose is to discover discrepancies without the user choosing
which frame or actor to inspect. This is implementation work, not another plan.

**Orrery remains an OpenSpace runtime analysis tool.** Preserve its inspection,
analysis, control, save research and MCP capabilities; recording/replay extends
them. The behavior suite has always belonged outside Orrery. Even OpenSpace-only
gameplay scenarios/assertions belong in the external suite. Orrery retains its
own tool/contract tests and source-specific analyses; do not move all analysis
out merely because a module happens to be called `comparison`.

This and the [loading-efficiency assignment](./loading-efficiency-handoff-20260927.md)
can proceed independently. No broader roadmap execution order has been chosen.

## Starting state and required reading

Repository: `/home/stephen/Projects/astrolabe`. Start from **the commit containing
this handoff**, which also checkpoints the required game code, fixture relocation,
Load Game menu and recording design. Its preparation-time parent was
`7caacd52bdb3d23c2250bf87ee367fd4d7754744`; that older commit alone is incomplete.
Preserve subsequent working-tree changes and include any needed source changes
in an isolated checkout. Ignored disc/generated assets and private
runtime evidence require explicit local access or regeneration; they are not in
Git. Inspect actual status before edits, since the loading task may have advanced.

Read root [AGENTS.md](../AGENTS.md), [architecture](../docs/astrolabe/rust-architecture.md),
[Rete design](../docs/rete/rete-design.md), [plan](../plan.md),
[session recording contract](../docs/astrolabe/session-recordings.md),
[comparison plan](./realtime-comparison-plan.md),
[Orrery guide](../docs/orrery/controller.md),
[runtime guide](../reference/README.md#runtime-validation),
[testing guide](../docs/astrolabe/testing.md), and latest
[implementation evidence](./implementation-progress.md). The September 27 scope
and user decisions supersede old ready-state-only and combined-package guidance.

The current Godot application is the game, including its basic actual menu.
**Load Game** currently applies a measured English-save fixture; **New Game**
will start with the intro when implemented. Use that game and its native
exported resources. Full save decoding, UI replacement and intro implementation
belong to other efforts; report their absence when limiting a replay claim.

## Existing capabilities and code map

| Area | Current files and limits |
| --- | --- |
| Native analysis/control | `crates/orrery/src/orrery.rs`, `mcp.rs`, `linux.rs`, `desktop.rs`, `config.rs`; private owned Wine/Gamescope/Xwayland, guarded live inspection and physical input |
| Original replay probe | `original.rs`, `instrumentation.rs`, `replay.rs`; fixed 18-update proof, narrow completed-update action evidence, not a general recorder |
| Mixed behavior suite | `behavior.rs`, `behavior/*.rs`, `tests/behavior_*.rs`, `tests/support/behavior_steps.rs`, `tests/features/*.feature`; currently misplaced inside Orrery |
| Godot control/observations | `crates/orrery/src/behavior/godot.rs`, `godot/tests/behavior_gameplay.gd`; existing functional backend to extract, not discard |
| CLI and source analysis | `main.rs` has `compare-behavior` alongside native commands; `comparison.rs` contains source/Rete/live analysis; preserve the latter's appropriate runtime-analysis role |
| Save research | `crates/orrery/src/save.rs`, CLI `save-inspect`/`save-diff`; partial decoder, not complete arbitrary save-set capture/restore |
| Shared typed readers | `src/openspace/level/live.rs` and related source/runtime modules; reuse declarations instead of duplicating decoders |
| AI tracing on Godot | `src/godot/ai.rs`, `godot/addons/astrolabe/behavior/behavior.gd` and focused native API components |
| Supervisor/inventory | `tools/test.py`, `suites.py`, `selection.py`, `supervisor.py`, `test-manifest.json`, `coverage.py`, `test-tools.json` |
| Existing image differences | `tools/compare_images.py`; does not establish frame association by itself |

Important limits: `replay::Recording` schema 1 starts from a neutral ready state,
has 4,096 updates/1 MiB limits, and aborts when it loses that epoch. It does not
represent program startup, multiple levels or a long recording. Native screenshots
explicitly report no completed-update/render association; Godot screenshot
requests also do not prove it. Stopped-byte memory inspection cannot establish
completed-update state. Hype's mechanics use variable deltas; equal configured
FPS does not align two engines. Current generic AI-call tracing and passive
human-input recording are not implemented.

Existing functional coverage includes movement/contact, jump, sword, initial
crossbow launch and Fire cases with deliberately bounded claims. Preserve all
scenario membership, assertions and negative controls when migrating. The last
reported September 25 movement selection was 87/90, with an AI count mismatch,
a native process-lifetime failure and original-game deadline. The September 27
Godot menu/player smoke succeeds but logs 30 unsupported other-actor behaviors.
These are baseline limits, not permission to suppress future failures.

## Ownership and collaboration

Own Orrery runtime/control/capture, a small engine-neutral evidence contract,
the external BDD/comparison runner, the separate Godot harness, their tests,
CLI/reporting and optional AI/API trace instrumentation. Choose a modest
workspace package layout that enforces these dependencies; do not proliferate
generic crates without a concrete use. The converter and ordinary game must
not depend on the controllers, suite or MCP. Orrery and Godot tooling must not
depend on each other; the suite composes both. Keep the converter the default
workspace member and update actual test discovery/coverage for moved packages.

The loading task owns `godot/game/**`, native resource serialization and the
loading/session lifecycle. Reuse current entry paths and `ready_to_play` where
needed; agree on a narrow observation seam if that task refines it. Do not
refactor its menu or implement a competing loader. Keep harness code outside
production game flow, with optional instrumentation hooks where needed.
Coordinate narrow edits in `src/godot/support.rs`, Cargo manifests/lockfile and
`tools/test-manifest.json`; preserve the other task's entries. Prefer dedicated
dated evidence notes to simultaneous rewrites of shared status documents.

Use separate checkouts with the supplied working-tree state, separate generated
outputs, evidence directories and build directories for simultaneous validation.
Never share one live original-game installation/session between jobs. Do not
let edits from another job invalidate a run unnoticed: include actual code,
authored game/harness files, fixture and generated-content hashes in provenance.
The current supervisor's source digest omits `godot/game`; fix relevant
provenance coverage while retaining its changed-during-run safeguards.

## Required delivery

1. **Restore the intended boundaries.** Move gameplay scenarios, shared Gherkin
   steps/assertions, cross-engine reports/commands and the Godot backend out of
   Orrery, preserving behavior and evidence. Keep native analysis/control and
   its tests in Orrery. Maintain one shared step registry, not engine-specific
   feature copies. Update the test manifest, runner, package discovery, coverage
   and documentation together; prove scenario counts and release membership
   survived. Do not stop after a directory move.
2. **Make a full session a reproducible artifact.** Implement the versioned
   manifest, human description/tags, immutable input/evidence streams, clocks,
   hashes, coverage/gap declarations and initial save-file fixture. Arm before
   launch and capture startup, menu, loads, gameplay and shutdown/interruption.
   The save fixture is the complete available file set with bytes and relative
   structure, not the chosen slot or a process dump. Represent empty/absent
   roots explicitly; unreadable roots fail. Restore into isolated per-run
   storage and preserve newly written saves as outputs. Adapt current hardcoded
   fixture checks through explicit verified profiles; do not weaken executable,
   prefix, ownership or process-identity guards to permit new fixtures.
3. **Record human or scripted play, then replay it on OpenSpace.** Capture
   physical input actually delivered to the owned game, including startup/menu
   events, holds, releases, repeats and supported devices. Offer a usable
   record/start/stop/replay workflow with CLI and appropriate existing MCP
   integration. User-driven recording must not capture unrelated desktop input.
   Use ordinary OS delivery for replay; do not write keyboard buffers or patch
   input callbacks to provide gameplay actions. Keep an immutable recording
   and separate replay-run outputs. An automated demonstration can establish
   the capture path without waiting for a person; label any unexercised human
   interaction explicitly rather than claiming it was driven by the user.
4. **Qualify timing and frames.** Record session monotonic time, engine clocks,
   phase/load epochs, intended/delivered/verified-consumed input, completed
   update state and render/capture association. Prove the actual fences in
   both engines, including stalls, catch-up updates, paused/menu phases and
   clock resets. Do not promote the existing action-only probe to all-state
   coherence. Preserve lossless output suitable for pixel comparisons and a
   frame index with gaps/duplicates/acquisition uncertainty. Keep independent
   millisecond latency measurements; never infer them from FPS. Unsupported
   association remains unqualified and fails a selected precision claim.
5. **Trace state and the AI–engine interface.** Record actor lifetimes, typed
   state snapshots/deltas, normal/reflex transitions as events, evaluated call
   arguments, result/out values, relevant side effects and deferred completion.
   Retain call/caller/receiver/source-location IDs and within-update order.
   Verify source dispatcher/handler conventions for the pinned executable;
   returns may be engine-managed expression values or flags. Add optional
   Godot generated-code/native support instrumentation with source mappings,
   preserving short circuiting and evaluating arguments once. Use bounded
   buffers and batched draining with overflow/coverage reporting. Unknown API
   calls and fields must be visible gaps, not silently dropped or recorded as
   empty successes. Expand by verified call families rather than promising
   complete engine interpretation from one Fire or movement probe.
6. **Connect the separate Godot harness and comparisons.** Consume the same
   declared physical recording and emit independently observed evidence from
   the actual game. Preserve focused initialized-state scenarios while adding
   full-session capability; label their different scopes. The current basic
   menu/fixed save route cannot pass original UI/save-loading parity by changing
   inputs until it matches. Declare literal-time, verified-update and any
   readiness-gated navigation policies before replay; report delays instead of
   silently realigning away slow loading. Do not force candidate state, API
   results or camera to the original's observations.
7. **Make differences reviewable and recordings extensible.** Produce an
   offline review artifact with linked video frames, input, state and API events,
   side-by-side/overlay differences and the earliest unexplained divergence.
   Compare the whole captured sequence, grouping related mismatch intervals
   instead of dumping every pixel as a separate failure. Distinguish raw
   per-backend evidence, normalized comparisons, independent behavior results
   and known debt. Descriptions, chapters and additional recordings should add
   coverage without writing a new timing script for every route. A chapter
   needs replay of its prefix or a separately verified start fixture to become
   independently replayable.
8. **Qualify bounded long-session operation.** Stream chunks/indexes instead
   of collecting the whole run in memory; preserve a readable completed prefix
   on interruption and mark incomplete tails. Test at least a load/epoch
   transition and storage/buffer limits. Full-game speed-run support is a design
   and endurance target, not a claim of a completed game playthrough. Do not
   require full-game Godot parity to deliver this recorder.

Keep a runnable path through these steps. A scaffold is an intermediate result,
not completion of this assignment. Deepen precision and trace coverage with
real evidence; unknown engine semantics may remain named gaps, but do not
advertise full precision or exhaustive AI tracing until the relevant claims
are demonstrated. Do not emulate the OpenSpace scheduler or memory in Godot.
Keep raw numeric precision; normalization/tolerances belong to comparison.
Memory captures remain local diagnostics, not portable saves or Rete assets.

## Acceptance evidence

Exercise the following with deterministic synthetic negative controls and real
engine runs where that claim requires them:

- Original-to-original recording/replay from launch through loading the supplied
  save, gameplay and stop; also an explicitly empty initial save set selecting
  New Game. Record the reached state and intro behavior rather than claiming
  complete intro coverage from merely pressing its menu item.
- Save creation/change followed by another replay restores the identical
  initial file set without overwriting user saves or inheriting prior results.
- Frame/update association catches a one-update-late input, a wrong/late image,
  stale/mid-update state, missed capture and buffer overflow. A slow assertion
  or observer does not silently alter the event schedule.
- An `A -> B -> A` AI transition within one update remains visible; altered
  argument/result, a deferred effect, nested/associated calls and actor address
  reuse are detected or explicitly reported outside verified coverage.
- A source/reference recording with a deliberate Godot discrepancy produces an
  actionable linked frame/state/API report; two equally broken backends do not
  pass because they agree. Same-engine repeatability informs tolerances before
  assessing Godot. Do not tune thresholds to accept the candidate.
- Cancellation, hook-install failure, exhausted storage, worker death and
  interrupted save/capture paths release keys and clean up owned processes and
  hooks safely. No live original-game installation or evidence is destroyed.
- Every existing behavior scenario is accounted for after extraction; previously
  passing assertions retain coverage. Separate baseline failures from new
  regressions. Unknown/pending/invalid evidence never becomes a green pass.

## Validation and local setup

Use `tools/test.py` and its exact manifest, mandatory safeguards, per-test and
selection deadlines. Available selectors include `runtime`, `comparison`,
`movement`, `jump`, `combat`, `magick`, `ai` and `animation`; there is no `bdd`
selector at preparation time. Start with selection/discovery and focused jobs:

```sh
cargo build --workspace --release --locked --offline
cargo fmt --all --check
cargo clippy --workspace --locked --all-targets -- -D warnings
python3 tools/test.py --area runtime --area comparison --offline --list
python3 tools/test.py --area runtime --area comparison --offline
python3 tools/test.py --area runtime --area comparison --local --offline --original-config /home/stephen/Projects/hype-harness/local/config.json
```

Run the migrated behavior areas affected by extraction too; prove the full
scenario inventory remains classified instead of executing only a new synthetic
case. Use explicit measured budgets, with no automatic retry-to-green or broad
exclusions. The known local config path is a starting reference: verify the
private installation, executable/profile and fixture root before launching.
Use native Orrery, not the available Python `hype` MCP harness. Preserve
`reference/Savefile_EN.zip`, all reference submodules and private saves/evidence.
A reusable recorder must support explicit fixture profiles; it must not always
inject the supplied manor save at launch.

Final delivery includes runnable recording/replay/comparison commands, a short
demonstrated recording, independent backend results, the timeline artifact,
precision/trace coverage and gap counts, instrumentation overhead, cleanup
evidence, migrated scenario inventory and a concise handoff for adding another
recording or letting the user drive. Keep game-derived files local. Coordinate
only the actual loading/readiness interface with the loading task; do not wait
for UI, complete save decoding, a cutscene port or all Rete coverage to make
the supported end-to-end path usable.
