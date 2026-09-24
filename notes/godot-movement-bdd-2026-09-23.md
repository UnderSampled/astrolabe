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
- A fresh `manoir` import under `/tmp/astrolabe-ai-check-20260923f` saved 3,465
  Rete assets. Reopening Hype at `level:/persos/world/0/0` finds a Brain bound
  to `fix:/ai/models/28` with 177 designer fields; normal/reflex selection is
  initially behavior 0. The import records 511 unsupported-script diagnostics
  and two remaining unsupported designer records. The source AI inventory
  probe still proves 55 source Brains, 33 models, 530 scripts and byte-exact
  reconstruction of its supported torch script; the partial importer now
  retains 32 models in that selected `astrolabe` level.
- Hype normal behavior 0 has a 1,397-node schedule and no rules. Its static
  inventory includes 43 unsupported items across gameplay, UI, combat, sound
  and particle paths. A whole-script failure stub would stop before testing
  movement even if unsupported operations are in unreachable branches. The
  next importer/compiler change preserves supported control-flow subtrees and
  fails at the first *reached* unsupported fragment; it must not silently
  implement a source operation as a no-op. In particular Raymap's
  `Operator_Dot` is member access, not a vector dot product.
  Recognized source calls whose arguments are understood should compile into
  maintained Godot API methods; unfinished methods fail visibly in Godot.
  Import-side fragments remain for source forms that cannot yet be decoded
  into a meaningful call. This keeps engine-side behavior work in Godot.
- The first full `manoir` Godot export reached validation of the generated
  native resources. Its first headless import took about 16 seconds, then the
  7.6 MB `build.gd` exceeded the prior 60-second builder deadline while Godot
  was reading animation resources; no generated scene was published. A bounded
  longer build deadline needs a measured full export before this path can be
  counted as working.
- Partial script decoding now retains supported `If` branches and statements
  around source fragments. Three parser tests pass, as does a native Godot
  test that runs an unselected branch and stops at a reached statement or
  condition fragment. A fresh import in
  `/tmp/astrolabe-ai-fresh-manoir-20260923` saved 3,475 assets and Hype's
  original Brain/model through Rete schema 27. Selected normal behavior 0
  currently has 208 top-level statements: 66 assignments, 12 conditionals,
  115 array assignments and 15 explicit unsupported statements. This is
  partial compilation evidence; the saved Godot scene and movement behavior
  have not yet passed.
- The fresh full export passed its first Godot import, then exposed a compiler
  error: assignments to unresolved source waypoint fields generated invalid
  `Variant(value)` expressions. The native builder continued after that script
  failed to load and exhausted its 180-second deadline. The compiler now emits
  a reached-path failure for those writes, and script validation runs before
  native scene construction so the next error can fail promptly.
- The first selected Hype schedule statement is executable procedure 408,
  `Proc_SND_FadeMusic(127, 1)`. Its two integer arguments are source-confirmed;
  their meaning and fade envelope are still being traced. It is now a typed
  `SoundFadeMusic` statement compiled to a maintained Godot
  `sound_fade_music` API stub, which reports an explicit failure if reached.
  Raymap labels its index 408 `FadeAll` because its table omits three earlier
  executable entries; the patched executable dispatches 408 to FadeMusic and
  411 to FadeAll. The build-specific table takes precedence. The renamed AST
  requires fresh Rete schema 29 and Godot support interface 20.
- The next full export exposed `float(Node3D)` in a compiled actor equality
  condition from `level:/ai/models/1106`. It compares a designer actor field
  with a preserved source perso reference. Equality/inequality now lower to
  native Godot object comparison for actor expressions; the unresolved source
  reference still fails only if that condition is reached.
- A fresh schema-29 `manoir` import saved 3,475 Rete assets and reopened Hype's
  Brain and model with `SoundFadeMusic(127, 1)` as the first typed statement.
  The full Godot export from those assets then passed in about 120 seconds:
  first import 12.6 seconds, native builder 80 seconds, validation reimport
  10.2 seconds and published project import 10.9 seconds. The saved 2 MB
  `generated/hype/levels/manoir/level.tscn` retains Hype at `world/0/0/Hype`
  with the compiled model-28 Brain, normal/reflex behavior 0, and a GDScript
  file that Godot loaded successfully. The disposable evidence is under
  `/tmp/astrolabe-godot-schema29-export-20260924`. Script loading does not yet
  establish actual gameplay: the reached FadeMusic API is a failing Godot stub,
  later source fragments remain, and the native character defaults to
  stationary inspection mode.

## Open work and debt

- The saved Hype Brain and model survive normal import, Rete reopening and
  completed Godot export. Real input execution remains required. Recognized
  but unfinished source calls need typed AST forms and maintained Godot API
  stubs as their reached paths are identified.
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
