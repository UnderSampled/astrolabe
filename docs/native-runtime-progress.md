# Native runtime implementation evidence

Implementation follows steps 2–6 of the [comparison plan](realtime-comparison-plan.md).
The validated first slice is commit `94b9476`: actual Godot consumer-boundary
checks, exact test selection, mandatory safeguards, process cleanup and a separate
Rust coverage baseline. [Testing](testing.md) records commands and measured results.

The optional `crates/astrolabe-testkit` workspace package depends on Astrolabe;
the root converter remains the default workspace member and has no testkit
runtime dependency. The existing external Hype service and prepared installation
remain the working original-game baseline. This package does not yet replace them.

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
