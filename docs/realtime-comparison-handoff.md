# Realtime comparison implementation handoff

Prepared 2026-09-20 after planning, independent design review and two original-
engine timing investigations. The new work is implementation of the comparison
harness; full gameplay recovery follows once that harness exposes real failures.

## Recoverable starting point

The annotated tag **`codex/realtime-comparison-checkpoint-20260920`** identifies
the commit containing this handoff on `codex/live-comparison-plan`. Its history
includes these separate checkpoints:

| Commit | Preserved state |
| --- | --- |
| `c912db7` | Gameplay foundations, source-rule/component tests, Rete schema 25 and Godot support interface 17 |
| `4732ed7` | Initial native loader/comparison plan and revised testing policy |
| `0dda3f2` | Shared scenario recordings, timing/visual comparisons, review findings and mechanics investigation |

Use the tag to revisit this exact state without resetting ongoing work. From the
repository root, a separate checkout can be created with:

```sh
git worktree add --detach /tmp/astrolabe-realtime-checkpoint codex/realtime-comparison-checkpoint-20260920
```

Choose another unused destination if that directory exists. This recovers tracked
code and documentation; it does not populate ignored game data or initialize
reference submodules. Preserve the existing `disc/`, `output/`, `TestResults/`,
reference submodule checkouts, and external `/home/stephen/Projects/hype-harness`
installation. No game installation or generated asset snapshot is bundled in Git.

The local evidence archive is
`TestResults/checkpoints/realtime-comparison-20260920/evidence.tar.gz`, SHA-256:

```text
6f19c13ff060c1b17ab9a9849fe1eda30aeb4242dc75f269b05201900bc8f4a6
```

It contains 22 files: the original input probe/report/raw observations, both
harness run directories (including the screenshot), the mechanics investigation,
and the gameplay checkpoint's validation log/report. Its embedded `manifest.json`
records original locations and each member's size/hash. Every member was reread
and verified after archiving. This is ignored local evidence; keep it local and
extract into a fresh directory rather than over original runs. The source files
also remain in their original locations.

## Known state, not new work to repeat

- The working code is a preserved foundation, not a finished playable port.
  [Checkpoint validation](hype-control-progress.md#paused-implementation-checkpoint--2026-09-20)
  records 84 passes, one Godot shutdown timeout, and an incomplete suite at the
  old 570-second cap. Build, formatting and Clippy passed at that checkpoint.
  Do not present that as a complete passing suite or erase the failed test.
- Both engines accepted five-update physical-key recordings in narrow probes.
  The actual Godot character/AI consumer boundary under catch-up ticks and render
  stalls still needs proof. See the [timing evidence and limits](realtime-comparison-plan.md#feasibility-evidence-and-its-limits).
- The traced original mechanics path uses variable elapsed time clamped to
  16–80 ms; live actor history contained 20/21 ms steps. Input polled after update
  `k` prepares update `k + 1`. An actor update stamp precedes mechanics completion.
  See [mechanics and observation phases](realtime-comparison-plan.md#original-mechanics-cadence-input-phase-and-observations).
- Both investigation-owned game sessions were stopped. Original probe memory and
  callback changes were restored; the later mechanics investigation was read-only.
  Check current harness status before starting a new session, rather than assuming
  that this historical status is still current.
- Frame scheduling, shared literal inputs, independent real-time measurements and
  visual comparison are settled design choices. Tick integration/cadence, complete
  scenario chronology, observation phase and live identity are early validation
  gates. There is no automatic millisecond replay fallback.

## Goal for the implementation agent

Implement Astrolabe's native Rust realtime loader, live tester and comparison
harness, following `docs/realtime-comparison-plan.md` through ordered steps 2–6.
Start from `codex/realtime-comparison-checkpoint-20260920` in
`/home/stephen/Projects/astrolabe`; preserve that checkpoint and make new commits
on a development branch. Read `AGENTS.md`, this handoff, `plan.md`,
`docs/rust-architecture.md`, `docs/rete-design.md`, `docs/testing.md`, and the
external harness's runtime instructions before implementation.

Deliver working code and executable checks, not another plan:

1. Resolve the actual Godot character/compiled-AI input boundary first, including
   single-tick edges, consecutive catch-up ticks and render stalls. Use the
   established OpenSpace mechanics/input findings to qualify a comparison cadence.
   Preserve native Godot physics, frame-indexed input and independent measured
   milliseconds. Do not replay OpenSpace deltas or recreate its scheduler. If the
   assumed integration is disproved, document the evidence and resolve that
   integration explicitly before building dependent infrastructure; never silently
   change timing modes.
2. Build a native Rust runtime that links Astrolabe directly. Reuse OpenSpace
   declarations to expose structured original-game data to prepared tests and
   agent queries without caller-supplied memory addresses or a Python memory
   broker. Keep the controller optional and outside converter dependencies.
   Validate identity, bounds, stale handles, completed-update observations and
   cleanup before expanding readers. Preserve the working external harness
   until the native path has equivalent acceptance evidence.
3. Compare a narrow `manoir` actor/definition slice through both direct OpenSpace
   and closed/reopened Rete against the running original. Then extend to the
   planned shared/static dependency cases and `astrolabe`. Include deliberate
   edits, sharing/identity faults and unsupported/unmatched data in the reports.
4. Implement high-level Gherkin movement scenarios with one shared step registry
   generating a complete, identical physical-input recording for both engines.
   Engine adapters differ only in setup, delivery, observations and cleanup.
   Assertion/controller delays must not change inputs. Report state/behavior,
   actual elapsed milliseconds, and declared screenshot/pixel comparisons as
   separate results, using the real exported resources and compiled AI.
5. Add meaningful conversion unit tests, measured Rust code coverage, independently
   accounted level-data coverage, affected-area selection and mandatory safeguards.
   Bound individual tests and area runs; retain a complete release inventory that
   may take longer. Measure startup/cleanup and reject hidden omissions. Keep the
   first useful comparison ahead of broad metrics or runner refactoring.

Completion means documented, repeatable native commands exercise both data routes
and both engines, original scenarios pass their own assertions, deliberate
input/state/timing/image faults are caught, and the checkpoint port's real failures
are reported accurately. The Godot port need not already pass full movement parity
for the harness to be complete; its failures must remain failures, not forced
states, hidden exclusions or claimed parity. Record unavailable coverage honestly.

Run the appropriate focused checks and safeguards, preserve logs and measured
coverage/evidence, commit coherent implementation slices, and finish with exact
commands, results, remaining limitations and the next gameplay-resume point.
Full gameplay recovery (step 7), broad platform support and release expansion
(step 8) are subsequent goals, not prerequisites for completing this harness.
