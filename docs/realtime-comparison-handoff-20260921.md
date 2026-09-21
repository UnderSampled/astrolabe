# Realtime comparison implementation checkpoint — 2026-09-21

This is the next stopping point for transferring the goal to another agent. The
native wrapper path is now committed and has passed the focused comparison suite
under the required host privileges. A supervised Hype session has also completed
the native replay and direct/reopened comparison checks, but the live physical
consumer gate remains intentionally open. Preserve `disc/`, `output/`,
`TestResults/`, reference submodules and the external
`/home/stephen/Projects/hype-harness` installation.

## Resume here

The committed implementation baseline at this handoff is
`bcb58bb` on `main`. It contains the native
replay CLI, hand-emitted i386 wrappers, synthetic execution fixture, and complete
test inventory. The
[original handoff](realtime-comparison-handoff.md),
[comparison plan](realtime-comparison-plan.md), `AGENTS.md`, `plan.md`,
`docs/rust-architecture.md` and `docs/rete-design.md` remain the governing scope.
Complete ordered plan steps 2–6; gameplay recovery and wider platform/release
expansion follow later.

The next gate is **source-qualified physical-consumer evidence**. The wrapper
now publishes a verified-build Left action counter after the original keyboard
callback and completed-update hook, while explicitly reporting
`physical_consumption_verified: false`; the Shift/history consumer remains
unobserved. Do not skip ahead to a broad coverage pass or call the harness
complete. After that gate, run shared/duplicate/replaced/stale identity cases,
then the same physical-input recording in both engines, then broader BDD and
coverage accounting.

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
| `cb8e8d3` | Synthetic ELF32 execution now forwards both callbacks exactly once, survives hostile `timeGetTime` register clobbering, publishes 18 completed updates, rejects duplicate polls, checks actor identity/pointer chains, and records the exact shared input masks. The focused comparison suite passed 27 selected tests in 72.396 seconds under the required host privileges; release build, workspace Clippy and formatting also passed. |
| `bcb58bb` | Adds the verified-build `LEFT_ACTION_COUNTER` source observation after callback/update publication, keeps the overall physical-consumption result false, and makes the synthetic fixture map its own counter address. Native probe unit tests (3) and workspace Clippy/formatting pass. |

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

The stopped Sol agent's original changes remain backed up for provenance. Local,
ignored directory:

```text
TestResults/handoff-20260921-stop/
  native-replay-draft.patch
  manifest.json
  draft/crates/astrolabe-testkit/src/{main,original}.rs
  draft/crates/astrolabe-testkit/tests/original.rs
```

Patch SHA-256:
`8aed5cf3541ad7e3d3020b6855669b1149a19bb94f3b51c1a83b6579c8c936f6`.
It applies to the earlier implementation baseline and is retained only as
research evidence; the manifest records its original base and file hashes. Keep
this directory when transferring the workspace; it is not in Git. The concrete
wrapper blockers identified in review are covered by the committed synthetic
fixture, but the fixture still does not prove that the real Hype consumer reads
the injected buffer.

Publication must forward the original callback exactly once and observe only
after it returns. Read `docs/hype-publication-evidence.md`: current action and the
verified-build Left action counter are source-qualified observations. The native
report still keeps `physical_consumption_verified: false`; Shift/history
consumption and SO+36 global-transform writer closure remain unproved. Early
counters/actor stamps and stopped snapshots are not fences; screenshots remain
independent until their association is proved. No automatic millisecond replay
fallback is allowed. The report labels planned key masks and protocol ordinals
separately from source-qualified observations.

## Runtime operation and delegation

The latest user-directed operating rule makes **Luna the sole Hype MCP operator**.
Astra/Sol/Terra supply explicit bounded procedures: target, preconditions, bounds,
timeout and expected evidence. Luna returns raw results without improvising.
For precise memory/pointer operations, writes/patches or unexpected results,
Luna stops and escalates review; no guesses or retries. Use one Hype controller
at a time. Sol specifies and interprets native-runtime work instead of
independently operating Hype MCP. The existing Python Hype MCP remains the
behavioral physical-key baseline while the Rust wrapper is brought to equivalent
frame-indexed consumption evidence.

Current delegation ledger: Astra (gpt-6-astra) reviewed the wrapper ABI and
failure modes; Sol (gpt-5.6-sol) implemented the native wrappers and synthetic
ELF32 execution fixture; Luna (gpt-5.6-luna) supplied the bounded live-run
procedure, operated the supervised Hype sessions, and remains the sole Hype MCP
operator; Terra (gpt-5.6-terra) audited the ordered acceptance matrix and
identified the remaining identity/consumer gates. All implementation agents are
stopped after their evidence was incorporated. Coordinate file ownership and
freeze implementation during any supervised Hype validation.

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
- `TestResults/native-replay-20260921-luna-5/summary.json` has SHA-256
  `0d8ebcf2be311c601f2cbf387281f9adbbe827ed867b04f3e135500d0e187fc9` and
  `worker.json` has SHA-256
  `4968cc387615422af10a9276d7c2ef183d113928ef9a664bbfbbf15ac61b3699`. The
  rebuilt `target/release/astrolabe-testkit` was used (SHA-256
  `5969eee9f4b34e43be27ea59d392a2b4e6e230a236a16ccc139c6687c88f10b7`). The
  run reached phase 9 `manoir`, published 18 completed updates, observed the
  Left counter in every row, and cleaned up fully; physical consumption,
  frame replay and behavior parity remain false.
- `TestResults/comparison-original-20260921-luna/actor-comparison-1789967787416059917/comparison.json`
  has SHA-256 `a288661a1c4e5d410a8320145f417840ccdcb66419a5f725ed6838e0335ec72b`.
  Direct and reopened-Rete selected invariants match, a deliberate persisted
  edit is rejected, and the baseline is restored; behavior parity remains false.

Handoff verification passed:

```sh
python3 -B tools/test.py --area comparison --offline --log TestResults/handoff-20260921-bcb58bb-validation3.log
cargo fmt --all --check
cargo clippy --workspace --locked --offline --all-targets -- -D warnings
git diff --check
```

The latest focused run discovered the complete manifest, passed all 27 selected
Rust tests and 14 Python supervisor/selection checks in **37.344 seconds** under
the required host privileges, with no leaked/remaining child processes and
complete temporary cleanup. The Godot safeguard took 32.395 seconds. Evidence is
`TestResults/handoff-20260921-bcb58bb-validation3.log.json`; the committed code
hash is `bcb58bb43816cd08f024079e97da0f6861b56102`. `cargo build --release
--locked`, workspace Clippy and formatting also passed. An earlier parallel run
showed two transient process-lifecycle failures; both safeguards passed when
rerun serially and the final supervised run passed all 27.

Original-engine tests were excluded from the focused offline run. The supervised
native replay and one-actor direct/reopened comparison are recorded above; full
shared/duplicate/replaced/stale identity acceptance, BDD coverage accounting,
and gameplay parity remain open.
