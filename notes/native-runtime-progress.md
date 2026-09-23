# Native runtime implementation evidence

Implementation follows steps 2–6 of the [comparison plan](./realtime-comparison-plan.md).
The validated first slice is commit `94b9476`: actual Godot consumer-boundary
checks, exact test selection, mandatory safeguards, process cleanup and a separate
Rust coverage baseline. [Testing](../docs/astrolabe/testing.md) records commands and measured results.

The optional `crates/orrery` workspace package, exposed as **Orrery**, depends on Astrolabe;
the root converter remains the default workspace member and has no testkit
runtime dependency. The Python Hype service remains available but is not a
dependency of the new suite. The prepared installation and earlier evidence remain local; Orrery
is the active native control path. Its live MCP launch, supplied-save loading,
physical input, selected AI inspection, screenshot and stop path was verified on
2026-09-23; integration as a behavior-suite backend remains open. See
[the live session record](./orrery-live-session-2026-09-23.md).

## Process and capture contract under implementation

The Linux controller owns child identities through pidfds and process start ticks,
adopts escaped descendants through subreaping, and excludes pre-existing processes.
One owner is allowed per controller process. All concurrent child launches in that
controller must go through the owner. Explicit teardown reports errors; Drop is a
bounded fallback and the outer test supervisor handles killed owners. Windows is
not implemented.

A pause guard verifies every process thread is stopped, preserves a pre-existing
whole-process stop, and rejects mixed/debugger-owned stops. New stops are resumed
on normal return or Rust unwind. Bounded memory reads check readable mappings and
reject partial transfers. A capture caches only requested ranges within a 16 MiB
budget; its immutable snapshot cannot read missing pages after resume. It is always
labelled `StoppedBytes`. A synthetic producer paused between related field writes
must fail completed-update acceptance even though those bytes are stable.

These interfaces remain provisional. No file allocation index or canonical asset
identity is inferred from a live address. Shared OpenSpace readers, live generations,
within-level replacement checks, native Wine/Gamescope launch/input/capture and
verified input/completion hooks are still required before a native gameplay run.
No native scenario, static/live parity or completed-update result is claimed here.

The first native ownership selection passes: **12 Rust tests and 14 Python
checks in 152.669 seconds**, including a 79-second cold workspace build. Report:
`TestResults/native-owner-foundation-20260920.log.json`. Native checks took
2.231 seconds for escaped TERM-resistant cleanup, 0.052 seconds for failed launch
and ownership exclusivity, 0.155 seconds for preserved-stop/unwind restoration,
and 0.134 seconds for bounded immutable captures, stale reads and mid-update
rejection. The real reopened-Rete/native Godot safeguard passed in 65.102 seconds.
All cleanup completed and the implementation hash remained unchanged.

## Shared source-reader decisions

The initial `RecordSource` boundary separates bounded bytes, offset arithmetic and
pointer resolution from interpretation. `FileView` retains normal block relocation
and active-level Fix binding. A relocated-memory view uses 32-bit pointers through
the caller's bounded capture. It never synthesizes source files or allocation indices.
The production character importer now shares intrusive-list traversal and exact
transform extents with the inspection path. Existing binrw declarations decode the
main-actor entry, superobject and family; the inspection projection contains canonical
`TransformValue` values and capture-local locators rather than new Rete assets.

A deliberately relocated known-byte fixture checks independent transform values,
null/sentinel and forward references, two distinct equal-valued members, and separate
actors sharing one family. Malformed counts, cycles, truncated records, unsupported
transform flags, foreign action references, build hashes and menu phases are rejected.
The named `manoir` and `astrolabe` file-side checks compare actor types, action members
and initial/current selections with the existing useful import. This is still **file
and synthetic-memory evidence**. Within-level replacement generations, real live
module/root validation, Rete/live identity proof, and completed-update hooks remain
acceptance gates; this reader contract is provisional until those pass.

The initial combined source/runtime/animation job passed 21 of 22 tests but failed
its new corpus fixture because the test supplied locale `EN` instead of the
importer's required `en`. That failure remains in
`TestResults/shared-source-views-20260920.log.json` (208.195 seconds). Correcting
only the fixture locale gave **15/15** runtime/actor tests plus the mandatory
Python checks in **124.278 seconds**;
`TestResults/shared-source-views-20260920-corrected.log.json`. Both source levels
completed their useful-import comparison in 5.994 seconds. A final run follows
promotion/reuse of the selected Perso/3dData/StandardGame prefixes; those changes
also correct the observation label from `dynamics` to `data3d` without changing
Rete or Godot interfaces.

The completed shared-reader extraction passed **22/22 Rust tests and 14 Python
checks in 206.235 seconds**, including build, both corpus projections, all
compact-matrix reconstruction checks, native animation/contact and mandatory
Rete/resource safeguards. Report:
`TestResults/shared-source-views-20260920-final.log.json`. Engine checks took
13.408 seconds (animation), 62.601 seconds (reopened-Rete resources) and 42.544
seconds (native motion/contact). Cleanup and implementation provenance passed.
Strict workspace Clippy, formatting and whitespace checks also pass.

## Native original-game launcher

The first native inspection completed in **19.795 seconds**, reached phase 9 in
`manoir`, decoded Hype's transform, object types `[1, 1, 1]`, current action 47 and
389-member family, and captured a verified 1024×768 image. The screenshot shows
the supplied save's character at the manor gate. The original service remained
running with no game session, the existing focus rule was restored, and owned
cleanup left no processes. Evidence:
`TestResults/native-inspection-20260921-first/` and the sibling
`native-inspection-20260921-first-supervisor.json`. The implementation hash stayed
unchanged during the run.

```sh
cargo build -p orrery --profile validation --locked --offline
target/validation/orrery inspect-original \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --output TestResults/native-inspection-new
python3 tools/test.py --area runtime --local --offline
```

The launcher reuses the existing installation and its advisory lock. It checks
the full executable hash, supplied save archive, all four installed SaveGame0
members, owned process identity/prefix and live image header. Gamescope supplies
the native worker's private display environment at exec. The worker links
Astrolabe directly and reads through the shared OpenSpace declarations. Only the
three documented menu keys are timed OS input; frame replay is not implemented.
The outer native owner restores focus and owns cleanup independently of the worker.

The first stopped-byte capture took **221.002 ms**, including pause/resume. Its
image request took **11.935 ms** independently after resume. Neither measurement
is a completed-update observation or real-time parity result. Page batching now
addresses the per-field syscall cost: cached neighbor bytes are discarded when
finishing, and only explicitly requested records remain in the immutable
snapshot. Cache and retained bytes share a 16 MiB budget and record counts are
bounded. A subsequent successful inspection measured 308 page reads (1,261,568
source bytes), 114.773 ms inside capture and 117.815 ms including pause/resume.
This whole-family inspection is not a qualified low-disturbance gameplay trace.

The first combined native acceptance run passed 17/18 tests but exhausted the
development budget while testing worker death: Gamescope stayed alive with the
stopped descendant, so watching Gamescope alone did not detect the dead worker.
The outer supervisor ultimately cleaned up; that is a failed native acceptance,
preserved at `TestResults/native-runtime-inspection-20260921.log.json` (287.322
seconds). The controller now publishes and watches the worker's own process/start
identity, independently of Gamescope. Evidence JSON is atomically published so
that lifecycle checks cannot observe a partially written marker.

The corrected runtime selection passed **18/18 Rust tests and 14 Python checks
in 107.862 seconds**, with unchanged implementation provenance and no leaked or
remaining processes. Report:
`TestResults/native-runtime-inspection-20260921-fixed.log.json`. The original-game
inspection took 20.130 seconds; deliberate worker death while all game threads
were stopped took 11.212 seconds and verified cleanup, unchanged saves and exact
restoration of the existing focus rule. The mandatory reopened-Rete/native Godot
resource check also passed (64.888 seconds). Workspace formatting, strict Clippy
and whitespace checks pass. Completed-update publication, frame replay, direct/
reopened-Rete/live comparison and runtime replacement identity remain open gates.
