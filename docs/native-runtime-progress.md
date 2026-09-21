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
