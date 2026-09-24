# Godot movement behavior continuation — 2026-09-23

## Goal and current evidence

The OpenSpace turn and short clear-route walk scenarios pass independently in
one Orrery game session. Godot is still pending in those scenario reports. The
next result must use the normally exported `manoir` scene and Hype's saved,
compiled Brain; a test-created Brain or forced action cannot establish it.
The [gameplay plan](./godot-gameplay-plan.md#1-playable-idle-walk-and-run-in-manoir)
also requires run and floor/wall contact before the full movement checkpoint.

## Decisions made during this continuation

- `tools/test.py` now accepts `--headless` (default) and `--no-headless`.
  The runner passes `ASTROLABE_GODOT_HEADLESS=1` or `0` to its children and
  records the choice in the report. A first visible run showed only brief
  import and script-check windows, not rendered gameplay; its project-contract
  test failed because a visible import changed the host project. The bounded
  `run_godot` validation helper therefore stays headless. Gameplay launches
  use the separate `run_godot_gameplay` path, which honors `--no-headless`.
  Both retain process deadlines and cleanup. The selected visible run reported
  43/44 passes, including AI, animation, behavior flow, native locomotion and
  contact checks, plus the original-game movement session. Its report is
  `TestResults/20260923T072153.124610Z-development.log.json`.
- The corrected focused selection passed 44/44 with `--no-headless` and full
  cleanup in `TestResults/20260924T031142.587446Z-development.log.json`.
  It still reports Godot gameplay as pending; the passing native component
  checks do not show a rendered movement scenario. A sandboxed attempt failed
  from Godot socket/settings and Gamescope access errors, so desktop/game
  validation must request the required access at delegation time. A stale
  generated `scene` test executable also failed nextest discovery with a Rust
  random-data error; rebuilding that executable resolved discovery.
- Partial AI import will use stable behavior/rule/field identities and an
  explicit unsupported script or value state. Source operands must be
  normalized to values or semantic references before Rete persistence; raw
  script bytes can contain virtual addresses. Malformed source records remain
  failures, and unsupported paths must fail if reached in Godot.

## Open work and debt

- Ordinary `manoir` AI import discards Hype's whole model if any script or
  designer type is unsupported. Partial import must retain stable model,
  behavior, rule and field identities; absent and unsupported scripts must
  stay distinguishable. Reached unsupported content must fail visibly.
- The saved Hype perso therefore lacks a Brain. The existing rule probes add
  one during a test and prove components only. Normal import, Rete reopening,
  saved Godot scene/script inspection and real input execution are required.
- The BDD runner has synthetic and OpenSpace backends; Godot remains a pending
  slot. Add a selected Godot backend, native observations and the equivalent
  declared clear-route setup. Compare only independently passing results.
- The optional shared-game chain passed its positive handoff case. A focused
  failure-path test should still prove later scenarios become `not_run` and
  final cleanup is reported correctly when the first scenario fails.
- Functional original observations are stopped-byte captures with broad
  elapsed-time bounds. Completed-update, exact timing and image pairing remain
  pending until a claim or ambiguous first Godot result requires them. The
  existing walk/idle check does not establish wall or floor contact.
