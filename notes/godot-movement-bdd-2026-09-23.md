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
  the saved AST keeps it as a typed `SoundFadeMusic` call to the maintained
  Godot API.
  Raymap labels its index 408 `FadeAll` because its table omits three earlier
  executable entries; the patched executable dispatches 408 to FadeMusic and
  411 to FadeAll. The build-specific table takes precedence. The renamed AST
  requires fresh Rete schema 29. Godot support interface 20 kept an explicit
  failing stub until the argument mapping could be measured.
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
  establish actual gameplay: interface 20 reached a failing FadeMusic stub,
  later source fragments remain, and the native character defaults to
  stationary inspection mode.
- A Luna original-game capture used the pinned executable and supplied English
  save, with a 15-second post-load wait. The call at `0x53edc4` entered the
  fade helper at `0x40ba10` with native cdecl arguments `(1, 127)`, reversing
  source argument order. The helper stored `127` as target and `1` as duration;
  its update routine at `0x40b7b0` interpolates against `timeGetTime`'s
  millisecond clock. Hype's startup call therefore requests level 127 over
  1 ms. Executable SHA-256 is
  `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`;
  save archive SHA-256 is
  `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
  Godot support interface 21 now implements that call with a separate native
  music bus, target range 0–127 and milliseconds. No converted music source is
  yet routed to that bus, so audible fade parity is unproven.
- A test-owned SceneTree wrapper around the normally exported `manoir` scene
  found saved `world/0/0/Hype`, action 47 and selected normal/reflex behavior 0.
  Instantiation took 36.3 seconds, followed by the requested 5-second settle;
  the interface-20 run stopped at the FadeMusic stub as expected. The wrapper
  sends physical InputEventKey edges and records scene-native state without
  creating a Brain. A persistent Orrery Godot BDD backend and focused test are
  being connected; they are not yet passing.

## 2026-09-24 continuation

- The persistent Godot backend now uses the same Cucumber steps and ordered
  turn/walk recordings as Orrery. It holds one saved `manoir` process across the
  compatible scenario handoff, sends physical key edges, captures action,
  source-normalized transform, native velocity/contact and a screenshot, and
  writes separate scenario and suite results. It checks a measured English-save
  fixture against the normally exported Hype actor and Brain; the fixture
  restores translation, basis, normal behavior 4, reflex behavior 3 and the 12
  designer values whose live values differ materially from imported defaults.
  Its provenance is the supplied save archive and pinned executable hashes
  above. The wrapper waits five seconds after loading for visible resource
  preparation, then enables saved gameplay motion and starts the compiled
  Brain. Import and validation Godot processes remain headless; selected
  gameplay honors `--no-headless`.
- The first fixture-backed runtime reached Hype action 47, on-floor contact and
  saved normal/reflex behaviors 4/3, then stopped at reflex 3 rule 9. Static
  source tracing found the target is a Fix-owned `Global` Perso at block
  `[5,0]` allocation 15, with its own Brain and model 1; designer field 17 is
  Boolean true by default. The current source/model change imports it as
  `fix:/persos/15`, records its source identity in `Scene.global_actors`, and
  normalizes the reached dot assignment to a typed actor-field write. Godot
  scenes create one nonvisual Global instance and wire referring compiled
  Brains to its saved native property. A fresh source-to-Rete guard confirms
  model 1, 238 typed fields and field 17 through persistence. Rete schema 30
  and Godot support interface 22 identify this revision.
- Hype normal 4 reads `Func_GetPersoSighting` into a source-coordinate vector,
  extracts its X/Y/Z components, and increments a designer counter. A Luna
  original-game turn probe kept Hype stationary, rotated 89.92 degrees, and
  observed the sighting vector match the negative source-Y basis column both
  before and after turning. In the saved Godot basis that is positive Z;
  the native character method converts it back to source coordinates for AI
  variables. Native floor/slope contact queries and the later random timer are
  typed through the same source AST. Slope classification remains provisional
  until a reached slope scenario validates it.
- A new original-game shared-session run passes all Cucumber steps and bounds
  after allowing both source-observed walking phase actions 78 and 32 at the
  held checkpoint. It launched once, turned 1.304 radians in place, walked
  1.870 source units with 1.539 units of late held progress, and cleaned up.
  The suite report is
  `TestResults/behavior-suite-1790226693035371324.json` (ignored local
  evidence).
- The first visible Godot run against the schema-30 scene exposed an omitted
  nonvisual Fix Global Brain in the packed level: its Brain was owned by the
  Global node rather than the scene root. Saving that Brain with the scene root
  owner retained Hype's actor-field NodePath and let his compiled Brain run.
  A subsequent turn still translated 1.281 source units with zero animation
  root motion. The native character accumulated a horizontal component from
  downward gravity projected along repeated floor contacts. The test wrapper
  also discovered that disabling physics before scene attachment did not keep
  it disabled through the load settle: Hype moved 1.366 units during that wait.
  The wrapper now pauses physics after attachment, and native contact clears
  the downward component without storing its slope projection. Godot support
  interface 23 identifies the movement correction.
- The visible Godot rerun passed both Cucumber scenarios in one saved scene
  process and cleaned up. The turn changed heading by 1.303 radians with zero
  translation or post-release drift. The walk advanced 1.885 source units,
  including 1.634 units after the held sample; after release it returned to
  zero velocity/root motion and stayed on the floor. The suite report is
  `TestResults/behavior-suite-1790228093808787414.json`, with separate turn
  and walk JSON reports and rendered screenshots under `TestResults/`.
- Orrery's `compare-behavior` command paired each passing Godot scenario with
  the independently passing OpenSpace report by recording hash. All declared
  action, relative path, relative heading, and functional input-delivery checks
  passed for both scenarios. The comparison artifacts are
  `TestResults/behavior-compare-turn-1790228093808787414.json` and
  `TestResults/behavior-compare-walk-1790228093808787414.json`. They explicitly
  label completed-frame image pairing and verified input-consumption timing as
  pending, so these are functional movement comparisons only.
- The focused `--area movement --local --no-headless --offline` supervisor run
  passed all 56 selected tests with 88 excluded, 33 nextest skips for excluded
  cases, one OpenSpace and one visible Godot BDD launch, complete cleanup and
  unchanged source. Its report is
  `TestResults/20260924T054744.542474Z-development.log.json`; build/discovery
  took 31 seconds, execution 262 seconds and cleanup under 0.1 second. This
  run includes the Fix Global count correction and the floorless input-boundary
  probe's explicit zero-gravity setup.
- Fresh OpenSpace and Godot reports from that focused run independently passed
  both scenarios; pairing them passed 17/17 functional comparison checks per
  scenario. The fresh pair artifacts are in the focused run's evidence directory
  as `behavior-compare-turn.json` and `behavior-compare-walk.json`; the largest
  relative walking path difference was 0.149 source units. Post-run formatting,
  workspace Clippy, release build and the synthetic Cucumber scenario test pass
  after two layout-only Rust lint fixes.

## Open work and debt

- The saved Hype Brain and model survive normal import, Rete reopening and
  completed Godot export and now execute the selected turn/walk input path.
  Recognized but unfinished source calls need typed AST forms and maintained
  Godot API stubs as later reached paths are identified.
- The fixture-backed rendered turn and walk and their named functional
  cross-backend comparisons pass. Verified input-consumption timing and
  completed-frame image pairing remain separate future claims.
- Global is instantiated under the current level for this first single-level
  path. The source level also has a rooted `Global` perso using the same model;
  the relationship between its runtime Brain state and the Fix allocation
  referenced by Hype's script still needs a targeted live-state check.
  Persistence of the Fix actor across future level transitions is open.
  The dedicated Godot music bus receives the fade value, but no converted
  music source is yet routed to it. Neither is a movement result.
- The optional shared-game chain passed its positive handoff case. A focused
  failure-path test should still prove later scenarios become `not_run` and
  final cleanup is reported correctly when the first scenario fails.
- Functional original observations are stopped-byte captures with broad
  elapsed-time bounds. Completed-update, exact timing and image pairing remain
  pending until a claim or ambiguous first Godot result requires them. The
  existing walk/idle check does not establish wall or floor contact.

## Deferred camera work

The separate [camera research](./camera-research-2026-09-24.md) records the 2026-09-24 source probe and temporary Godot calibration. Camera-specific implementation and assertions follow magick.
