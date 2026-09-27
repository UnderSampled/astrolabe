# Session recordings and independent replay harnesses

This is the agreed design for recording a complete play session and comparing
its replay. It is a contract for further implementation, not a claim that the
current tools already capture every stream. The [comparison plan](../../notes/realtime-comparison-plan.md#session-recording-direction--2026-09-27)
records the migration and first acceptance cases.

## Ownership

**Orrery is an OpenSpace runtime analysis tool, with recording/replay as an
additional capability alongside live inspection and control.** The behavior
test suite is a separate consumer of Orrery, as originally intended. Godot has
its own harness. Neither engine's tooling owns the other's runtime or the suite.

| Component | Responsibility |
| --- | --- |
| Orrery | OpenSpace runtime inspection, analysis and control; original-game launch, save-file staging, physical input recording/replay, state and AI/API tracing, output capture, and owned-process cleanup |
| Godot harness | Godot launch and initial persistence setup, native input delivery, scene/state and compiled-AI tracing, output capture, and cleanup, using the actual exported project and maintained support |
| Shared recording contract | Versioned descriptions, fixture and stream manifests, input/event identities, clocks, provenance and explicit coverage; independent of either engine controller |
| Behavior test suite, comparison runner and viewer | Own one set of shared scenarios and gameplay assertions. Run each selected scenario on OpenSpace, Godot, or both; retain each backend's result separately. Normalize and compare completed runs and present a synchronized timeline. |

Orrery remains useful by itself for interactive investigation as well as
recording and replay. The Godot harness can also run independently. The suite
depends on these tools through their public interfaces; neither depends on the
suite. Even OpenSpace-only gameplay scenarios and their pass/fail assertions
belong to the external suite. Orrery's own tool/contract tests remain with Orrery.
Orrery may verify that its own replay dispatched and observed the requested
physical edges. Judging whether two runs behaved alike, including two original
Hype runs, belongs to the suite. A still image captured for inspection or a
visual checkpoint is distinct from the continuous video stream required for a
session recording.
Shared formats do not require a Godot dependency on Orrery. Converter adapters
and ordinary gameplay do not depend on either tool.
Reuse Astrolabe's typed OpenSpace readers where applicable, without duplicating
the source decoder or recreating OpenSpace's execution machinery in Godot.

## A recording describes a whole session

Each recording has a stable identity, title, plain-language description,
behavior tags, provenance, initial environment, input stream and observed
output/state streams. Descriptions such as "new game through the first fight"
or "load the manor save, jump while changing direction, then cast Fire" make
recordings discoverable without a new test implementation for every route.
Assertions and narrowly scoped accepted discrepancies can be attached later;
annotations never rewrite the captured evidence.

The recorder is armed **before process launch**. The session timeline begins at
launch and includes startup, cinematics, menus, loading, gameplay, pauses, level
changes, saving, and exit or interruption. Record when a window and the first
capturable frame become available. A video beginning at the first visible frame
must retain its offset from launch; a lack of drawable output is explicit.
Human play and scripted input use the same recording contract, with the driver
identified. Capture the game-scoped physical events actually delivered and any
verified consumption evidence, not a reconstruction of intended player actions.

One recording can cover a short regression or a full-game speed-run. Use bounded,
incrementally written stream chunks and indexes rather than keeping an entire
session in memory. Preserve a readable completed prefix after interruption and
report the incomplete ending. Long runs have explicit duration/storage budgets
and separate endurance-suite selection; they do not enlarge every focused test.

## Initial save files are part of the fixture

The starting state is **the complete set of save files available before launch**,
not an already loaded level, a selected save slot, or a process-memory snapshot.

- Preserve the actual bytes and relative directory structure under the verified
  save root, including every slot and any catalogue/index files used by the
  game. Record content hashes, sizes and relevant filesystem metadata where it
  affects slot order or presentation. A path to mutable external files alone is
  not a reproducible fixture.
- Explicitly represent an absent save directory and an existing empty one.
  "No saves" is a valid captured condition; an unreadable or unknown save root
  is an error, not an empty fixture. A new game can also be started with existing
  saves present: the recorded menu inputs determine the choice.
- Capture from a quiescent source and verify the staged contents before launch.
  Restore into a fresh, isolated run directory/profile for every replay. Never
  merge in saves from a previous run or overwrite the user's originals.
- Record relevant configuration, controls, locale, executable/content hashes,
  engine/runtime versions, display settings and known randomness inputs
  separately. Save files alone do not establish deterministic execution;
  unresolved environmental dependencies remain explicit.
- Preserve newly created, changed and deleted saves as run outputs, with a final
  file manifest and stable snapshots at observed save boundaries where supported.
  Do not replace the immutable initial fixture with these outputs or present
  partially written files as completed saves.

The supplied English `Savefile_EN.zip` is one useful fixture, not an implicit
default for every recording. Loading it, choosing another slot or selecting New
Game is part of the captured menu sequence. Snapshotting files does not require
understanding or editing the whole save format.

The Godot harness stages equivalent available save slots through a declared,
versioned save-import/mapping path and records its provenance and unsupported
fields. It need not use the original file encoding. An unsupported menu or save
mapping is a reported gap in full-session replay. Existing tests that initialize
an actor from selected save values remain useful focused fixtures, but do not
count as replaying startup, menu selection or save loading.

## Synchronized streams

Keep a session-wide monotonic timeline plus engine-specific clocks and explicit
epochs for menus, loading and each gameplay/level lifetime. Record clock resets,
pauses, input-consumer identities and uncertainty. Gameplay update ordinals are
not meaningful throughout startup; do not invent them for unobserved phases.
Input events retain stable IDs, event order and held-state changes across phase
boundaries. Distinguish observed, dispatched and verified-consumed times.

| Stream | Evidence |
| --- | --- |
| Input | Physical device/key/button/axis events, repeats and releases, focus changes, actual delivery and known consumption; unsupported devices are explicit |
| Video and optional audio | Original captured output, frame/sample timestamps, render/update association where verified, acquisition uncertainty, gaps and duplicates; lossless images or video for pixel claims |
| Typed state | Initial observations, changes and periodic snapshots of supported actors, transforms, geometry/animation selections, cameras, contact, inventory and game variables, with units and identity provenance |
| AI/API events | Normal/reflex transitions, request/entry times, script location, caller/receiver, evaluated typed arguments, paired results/output parameters, observed mutations and deferred-effect lifecycle |
| Session events | Process/window lifecycle, menus and phase changes, level/load generations, save operations, errors, instrumentation changes and interventions |

AI transitions are events, not merely the final selection sampled each update.
Calls have correlation IDs and within-update ordering; synchronous duration is
distinct from delayed completion. Godot instrumentation uses compiled code and
native support with a source-location map, preserving short circuiting and
evaluating arguments only once. Orrery instruments build-verified script/engine
boundaries and their actual result conventions. Compare observable semantics
and causal order, without requiring identical internal scheduling or call stacks.

Preserve raw numeric precision. Coordinate/unit normalization and comparison
tolerances belong to a versioned comparison profile, not destructive rounding
during recording. Source addresses and optional selected memory snapshots/deltas
are OpenSpace diagnostic evidence; they are neither cross-engine identities nor
portable save states. Dynamic actor identities include their lifetime/load
generation so address reuse cannot join unrelated actors.

Record stream availability, trace coverage, overflow, missed hooks/frames and
measured overhead. Batch local events rather than pausing for an MCP call per
event. Missing evidence cannot silently become a matching result. A stream may
be useful before every state field or API is understood, provided the remaining
gaps are visible.

## Replay and comparison

Freeze a captured recording and initial fixture; each replay is a separate run
referencing their hashes and preserving its own observed outputs. Verify
original-to-original repeatability before assigning tight cross-engine
tolerances. Loading duration, input-to-effect latency and natural camera motion
are comparison results, alongside state and pixels.

Declare the delivery policy. Literal elapsed-time replay preserves the recorded
startup and loading delays, even if an input consequently arrives too early in
another run. Verified consumption-based replay may reproduce a gameplay epoch
at its qualified input boundary. A readiness-gated navigation profile can help
reach later behavior, but every wait must be explicit and its delay reported;
it is not evidence of identical startup timing. Never silently delay/rewrite
inputs or shift video to make a slower load or late animation look equivalent.
Do not force the candidate's state, camera or API results to match observations.

The viewer links video, input, AI calls, state and timing on one timeline and
offers side-by-side frames, overlays/differences and the earliest unexplained
divergence. Automated comparison examines the captured sequence, grouping
related mismatches into reviewable intervals; the operator need not choose the
instant to inspect. Preserve the [visual comparison rules](../../notes/realtime-comparison-plan.md#video-frames-and-pixel-comparison),
including explicit frame association, rendering profiles and visible masks.
Known debt names its behavior/field/interval, impact and revisit trigger; it
does not suppress unrelated new discrepancies.

Chapters and clips refer to intervals in the original recording. They are useful
for review and focused checks without inventing a new initial state. Independent
replay from a chapter requires a separately captured, verified start fixture
and input state, or replay of the preceding inputs. An arbitrary memory or typed
state snapshot is not automatically a restartable checkpoint.
