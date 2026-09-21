# Realtime comparison implementation checkpoint — 2026-09-21

The user requested a stopping point to transfer the goal to another agent. The
goal is paused and incomplete. All delegated agents stopped; the final replay
draft was never executed in Hype. No game session was started during this
handoff. Preserve `disc/`, `output/`, `TestResults/`, reference submodules and the
external `/home/stephen/Projects/hype-harness` installation.

## Resume here

The committed implementation baseline before this handoff is
`0345d67c391998b7b0fa3aeae021ab5f4b8c81f4` on `main`. This handoff adds formatting
corrections to five shared-reader files and documentation. The
[original handoff](realtime-comparison-handoff.md),
[comparison plan](realtime-comparison-plan.md), `AGENTS.md`, `plan.md`,
`docs/rust-architecture.md` and `docs/rete-design.md` remain the governing scope.
Complete ordered plan steps 2–6; gameplay recovery and wider platform/release
expansion follow later.

The next gate is **native physical input and verified completed-update
publication**. Do not skip ahead to a broad coverage pass or call the harness
complete. After that gate, run the one-actor direct/reopened-Rete/live comparison,
then shared/duplicate/replaced/stale identity cases, then the same physical-input
recording in both engines, then broader BDD and coverage accounting.

## Committed work and its limits

| Checkpoint | Implementation and evidence |
| --- | --- |
| `94b9476` | Bounded inventoried suites, mandatory safeguards, area selection and actual Godot input-boundary checks. See `docs/testing.md`. |
| `34222f7`, `8060f0c` | Optional native process ownership, bounded immutable stopped-byte captures and shared file/memory readers. Stopped bytes do not qualify as completed updates. |
| `bbcc47a` | Native owned Hype launch, supplied English save into `manoir`, stopped actor inspection, independent screenshot and bounded crash cleanup. |
| `5575fa3`, `990a870` | Shared selected actor/family/action projection and exact nine-file source fingerprints. Direct and reopened-Rete source checks exist; live lifetime identity is unproved. |
| `07fc273` | Read-only verified-build callback/device profile in `src/openspace/level/hype_runtime.rs`. Synthetic profile checks are not a live replay proof. |
| `bde4541`, `90a4684` | Fail-closed owned-process instrumentation. Existing syscall instructions, all-thread stops, register restoration, callback ownership checks and exposed scratch retained until process exit. Native and real i386 synthetic tracees tested; no Hype injection acceptance yet. |
| `1625c4f`, `6ed75ca` | Shared exact-byte physical-input recording, narrow comparison reports, closed/reopened Rete edit detection, compiled ignored live comparison test and optional Godot recording-file consumption. |
| `c32a17d`, `0345d67` | Source evidence qualifies only current-action publication after the original gameplay callback returns. No global-transform publication proof. |

The former unsafe instrumentation prototype `89aec5a` was reverted by `4f76657`.
Do not resurrect it. The safe foundation is `bde4541` and `90a4684`.

`crates/astrolabe-testkit` depends on the converter; the converter does not depend
on the runtime controller. Ordinary converter builds still select the root
package. Do not duplicate OpenSpace record interpretations or recreate its
scheduler/VM inside Godot.

The recording has 18 explicit updates, including neutral gaps, single-update
Left edges, an ordered Left Shift/Left chord and explicit releases. Its 1,093
literal bytes hash to
`26ea660a2564bbf3006f74021f7a69bfa43024b26891dce1a660b784026a9c5a`.
Use `record-boundary --output NEW_PATH` to generate it. Its schema is provisional
until both adapters pass. The Godot script parses that file; the updated optional
path and Shift dispatch have parser/API checks, but still need the actual shared
recording functional run.

`tests/comparison_original.rs` is compiled and ignored, **not run**. It verifies
installed/disc source fingerprints, imports the useful model, saves a full Rete
package, drops the source resolver, reopens Rete, obtains a fresh native actor
inspection, and tests a deliberate persisted action-47 speed edit. Its report
claims only selected invariant matches; behavior parity remains false. The
fingerprint reader accepts unique case differences (`GameData`/`Gamedata`) and
rejects ambiguity. Matching values/addresses are not lifecycle identity proof.

## Preserved unfinished draft

The stopped Sol agent's changes were backed up, reread and restored out of the
working tree. Local, ignored directory:

```text
TestResults/handoff-20260921-stop/
  native-replay-draft.patch
  manifest.json
  draft/crates/astrolabe-testkit/src/{main,original}.rs
  draft/crates/astrolabe-testkit/tests/original.rs
```

Patch SHA-256:
`8aed5cf3541ad7e3d3020b6855669b1149a19bb94f3b51c1a83b6579c8c936f6`.
It applies to the implementation baseline above; `git apply --check` passed
after restoration. The manifest records the baseline and all three full-file
hashes. Keep this directory when transferring the workspace; it is not in Git.
Review or reimplement it rather than treating it as accepted code.

The draft adds a replay CLI, hand-emitted i386 keyboard/update wrappers and an
ignored 18-update test. Sol reported Clippy and test compilation passing, but no
wrapper execution or independent review occurred. Concrete blockers found by
root inspection, not yet fixed:

- `patch_rel8` uses unsigned `checked_sub`, so its backward error branch cannot
  be encoded. Add meaningful synthetic assembly/control-flow tests before any
  live installation.
- The wrappers call `timeGetTime` while ECX/EDX hold values used afterward;
  those are caller-saved registers. Verify and preserve the full callback ABI.
- A second poll before consumption silently returns. Prove input consumption
  and reject missed/duplicate boundaries; reporting queued masks is not an
  independent observation of consumption.
- The wrapper follows the actor pointer chain without per-update null/root/
  phase checks. A final phase check cannot prevent an invalid dereference or
  qualify an intervening epoch change.
- The independent monotonic time relationship and wrap handling are unproved.
  Keep timestamps, engine counters/deltas and consumer ordinals distinct.

Publication must forward the original callback exactly once and observe only
after it returns. Read `docs/hype-publication-evidence.md`: current action is the
only source-qualified outcome so far. SO+36 global-transform writer closure is
unproved. Early counters/actor stamps and stopped snapshots are not fences;
screenshots remain independent until their association is proved. No automatic
millisecond replay fallback is allowed.

## Runtime operation and delegation

The latest user-directed operating rule makes **Luna the sole Hype MCP operator**.
Astra/Sol/Terra supply explicit bounded procedures: target, preconditions, bounds,
timeout and expected evidence. Luna returns raw results without improvising.
For precise memory/pointer operations, writes/patches or unexpected results,
Luna stops and escalates review; no guesses or retries. Use one Hype controller
at a time. Sol specifies and interprets native-runtime work instead of
independently operating Hype MCP. No live operation is part of this handoff.

Prior division: Sol owned native runtime and shared readers, Astra reviewed and
implemented safe instrumentation, Terra audited checkpoint provenance, Luna
prepared checkpoint notes. All stopped; do not assume those agent sessions can
be reused. Multi-agent work was explicitly authorized for the implementation
goal. Coordinate file ownership and freeze implementation during supervised
validation.

The preserved native fixture config is
`/home/stephen/Projects/hype-harness/local/config.json`. Its EXE hash is
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`;
the supplied save archive hash is
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
Native setup reaches menu phase 38 then supplied-save phase 9, `manoir`, using
timed menu keys only. These setup actions are not replay timing evidence.
The launcher shares the external lock and preserves that installation/service.
Read `reference/README.md` and the external runtime instructions before use.

## Retained verification

- `TestResults/native-runtime-inspection-20260921-fixed.log.json`: 18 Rust tests
  and 14 Python checks passed in 107.862 seconds, unchanged implementation and
  complete cleanup. The previous 287.322-second failed run is preserved too.
  See `docs/native-comparison-checkpoints.md` for provenance reconstruction.
- `TestResults/native-instrumentation-foundation-20260921-revised.log`: native,
  real i386, clone-churn, prior-stop and callback-lifetime checks passed in
  0.26 seconds. `native-instrumentation-failures-20260921-revised.log`: nine
  injected failure stages and callback ownership rejection passed in 0.58
  seconds. These use synthetic owned children, not Hype.
- `TestResults/shared-recording-comparison-unit-20260921-final.log`: three
  synthetic comparison/recording tests passed; the original-engine test was
  ignored. This includes real closed/reopened-Rete edit detection.
- `TestResults/shared-recording-godot-parse-20260921-writable-log.log` is a
  parser check, not the shared recording's gameplay acceptance test.

Handoff verification passed:

```sh
python3 -B tools/test.py --area comparison --offline --log TestResults/handoff-20260921-stop/validation.log
cargo fmt --all --check
cargo clippy --workspace --locked --offline --all-targets -- -D warnings
git diff --check
```

The runner discovered all 116 declared tests, passed its 24 selected Rust tests
and all 14 Python supervisor/selection checks in **152.375 seconds**, with no
leaked/remaining child processes and complete temporary cleanup. Build/discovery
took 82.602 seconds; the Godot safeguard took 62.608 seconds. The report retains
selection/exclusion reasons and `source_unchanged: true`. Implementation digest:
`018bc6557e4b70c0b5bafe5170302894334d7a1ad9e9c0f128c6b22c55dd60c3`.
This was baseline `0345d67` plus the committed formatting corrections; docs are
outside the implementation digest. Evidence is `validation.log.json` and
`style.log` in the handoff directory. The patch backup is separate from accepted
implementation and is not included in that digest.

Original-engine tests were excluded. No new full release run, native replay,
live comparison or full gameplay acceptance is claimed by this handoff.
