# Sol handoff: Godot loading efficiency

## Assignment

Implement and verify a substantial reduction in the Godot game's loading cost,
especially Load Game from the running menu and subsequent reloads. Improve the
actual game path and native generated resources; deliver measured before/after
results and maintain normal Godot editor inspection and editing. This is an
implementation assignment, not another planning pass.

This effort and the [Orrery/BDD upgrade](./orrery-bdd-handoff-20260927.md) are
independent assignments. No order between them, savegames, UI or the other
roadmap efforts has been decided. Coordinate only concrete shared interfaces.

## Starting state and required reading

Repository: `/home/stephen/Projects/astrolabe`. Start from **the commit containing
this handoff**, which also checkpoints the game code, fixtures, menu changes
and design documents prepared with it. Its preparation-time parent was
`7caacd52bdb3d23c2250bf87ee367fd4d7754744`; that older commit alone is incomplete.
If subsequent working-tree changes exist, preserve them and include any needed
source changes when creating an isolated checkout. Do not reset existing work.

Confirm that `godot/game/demo.gd`, `demo.tscn`, `manoir_start.gd`,
`english_save_manoir.json`, `godot/tests/demo_smoke.gd`, the **Load Game** menu,
and `docs/astrolabe/session-recordings.md` are present. Ignored generated assets,
disc data and reports are not supplied by Git; regenerate in an isolated output
or use an explicitly selected existing local project without overwriting it.
Inspect the actual starting status before applying changes; the other task may
already have advanced shared files.

Read root [AGENTS.md](../AGENTS.md), [architecture](../docs/astrolabe/rust-architecture.md),
[Rete design](../docs/rete/rete-design.md), [plan](../plan.md),
[Godot ownership](../docs/astrolabe/godot-project.md),
[roadmap](./remaining-work.md), [testing guide](../docs/astrolabe/testing.md),
and the latest [implementation evidence](./implementation-progress.md).

The former "demo" is the Godot implementation of the game. Its basic menu is
the actual current main menu. The current start is **Load Game**, provisionally
restoring the measured supplied English save. **New Game** will start with the
intro; neither New Game nor real savegame decoding is this assignment. Build on
this application. Savegame and UI work will replace the restoration and menu
presentation independently, using the same game operations.

## Evidence already available

Local probe: `TestResults/menu-load-profile-20260927/` contains `probe.gd`,
`probe.log` and `report.json`. It measured Godot 4.7.2 loading the current manor:

| Stage | One measured result |
| --- | ---: |
| Resource load | 38,328 ms |
| Instantiate loaded PackedScene | 52 ms |
| Instantiate with that resource retained | 21 ms |
| Enter tree and ready callbacks | 138 ms |
| Native scene nodes | 6,959 |

The static dependency inventory found 3,054 files, including 1,196 animation
resources with 314,289,375 bytes of text. `demo.gd` also waits 5.0 seconds, then
0.1 seconds, before starting the Brain. Treat these as investigation leads,
not proof that one serialization change will eliminate all delay.

This was one fresh **headless** process with filesystem caches **not cleared**;
it was not a cold-disk or GPU/first-frame benchmark. The probe did not apply the
full starting fixture. The latest headless menu/player smoke reached playable
state and moved Hype 1.926 units, but emitted 30 unsupported behavior errors
from other actors. See `TestResults/menu-flow-20260927/`. Preserve those limits;
do not use a player-only pass to claim the whole level is healthy.

## Code map and ownership

| Area | Starting points |
| --- | --- |
| Actual menu/session | `godot/game/bootstrap.gd`, `demo.gd`, `demo.tscn`, `manoir_start.gd`, `demo_camera.gd` |
| Native animation files/libraries | `src/godot/character.rs` (normal and restored libraries), `animation.rs`, `action.rs` |
| Mesh/resource generation | `src/godot/resources.rs`, `scripts/build.gd`, `scene.rs` |
| Export, references and safe publication | `src/godot/export.rs`, `project.rs`, `publication.rs`, `support.rs`, `engine.rs` |
| Current smoke and invariants | `godot/tests/demo_smoke.gd`, existing native asset/editor and source-free Rete tests, `tools/test-manifest.json` |

You own the game loading/session path, loading instrumentation, native resource
representation and their focused tests. The other task owns Orrery, BDD/shared
scenario code, the Godot recording/control harness, and optional AI tracing.
Do not restructure its suite or add a second runtime harness. In particular,
`crates/orrery/src/behavior/godot.rs` and `godot/tests/behavior_gameplay.gd`
belong to that migration.

Shared touchpoints are `src/godot/support.rs`, project/settings changes when
necessary, and the test inventory. Make narrow additive edits and preserve
the other task's entries. Keep game entry paths stable where practical; if a
rename is needed, update real references and communicate it. `ready_to_play`
is the current observation seam. Prefer a small ordinary readiness/status
interface the separate harness can observe, with no test dependency in the
game. Do not impose a speculative shared framework before either task needs it.

## Work to perform

1. Reproduce the bottleneck in the actual menu-to-load path and establish
   comparable fresh-process and warm/reload baselines. Separate conversion,
   first Godot import, native dependency loading, scripts, instantiation,
   state restoration, first rendered gameplay frame and controls becoming
   usable. Report cache conditions and instrumentation overhead. Measure the
   total wait as well as its components; moving cost before the menu is not
   automatically a user-visible improvement.
2. Investigate the expensive resource dependency graph. Experiment with native
   binary `.res`/`.scn` where justified, especially dense animation/mesh data.
   The user explicitly accepts Godot binary resources as long as they remain
   viewable/editable in the editor and follow Godot paradigms. Compare measured
   load/build/size costs before selecting compression or serialization flags.
   Retain meaningful resource sharing and inspect whether normal/restored
   variants duplicate content before changing their semantics.
3. Implement the winning change in the converter's maintained native output
   path, not only a one-off transformed generated folder. Update resource
   references, ownership/manifest checks and affected tests together. Preserve
   direct and reopened-Rete export equivalence and source-free use. Remove
   superseded owned output through the normal publication rules without
   touching unrelated levels or authored changes.
4. Organize the actual Load Game operation around resource preparation, applying
   the provisional saved state, native contact/readiness and enabling gameplay.
   Replace arbitrary warm-up sleeps only after verifying the conditions they
   were covering. Keep the menu/loading presentation responsive with a suitable
   native Godot approach; measure total duration independently of responsiveness.
   Prevent repeated requests, failed loads or returning to the menu from
   leaving duplicate actors, a running old Brain, or stuck inputs.
5. Reuse immutable loaded resources across reload/menu transitions where it
   produces a measured benefit; instantiate fresh mutable game state and
   reapply the provisional save. Check that inventory, transforms, AI state,
   effects and physics do not leak from the previous session. Bound retained
   memory and clean up old instances. Do not duplicate all content merely to
   obtain a clean reset.
6. Verify native editor opening, seeking/playing representative animations,
   scene contents, actual menu Load Game, reload and menu round trips. Inspect
   rendered gameplay as well as headless output. Keep known actor/gate/camera
   limitations explicit and distinguish new regressions from the baseline.

The level must still be a saved native scene before running it. Do not introduce
runtime asset reconstruction, an animation-table player or an AI interpreter.
Do not delete supported assets, reduce animation fidelity, omit actors or fake
readiness to make loading faster. Broad NPC fixes, a new menu design, save-format
research, cutscene implementation and Rete-wide completion are separate efforts.

## Validation and completion

Use the existing test supervisor with affected areas/dependencies and mandatory
safeguards. Existing area names include `scene`, `animation`, `ai`, `movement`,
`runtime` and `comparison`; there is no `loading` selector yet. Classify new
meaningful tests explicitly rather than bypassing discovery. Starting commands:

```sh
cargo build --release --locked --offline
cargo fmt --all --check
cargo clippy --workspace --locked --all-targets -- -D warnings
python3 tools/test.py --area scene --area animation --local --offline --list
python3 tools/test.py --area scene --area animation --local --offline
```

Adjust selection to the final change, preserving required gameplay safeguards.
The existing menu/player smoke can supplement these checks. Do not repeatedly
run unrelated full suites. Bound every engine run and reserve cleanup time.
If an original-game comparison is needed, coordinate use of the private
installation with the Orrery task; do not launch competing owners. Ordinary
Godot loading measurements need no original-game session.

Known prior suite failures include an AI model count (58 versus 57), a missing
native scratch-test process, and an original movement deadline (September 25
selection 87/90). Reproduce/classify failures in the selected scope; do not hide
them by weakening assertions or silently excluding required checks.

Finish with a before/after timing table from repeated comparable runs, resource
sizes and peak/retained memory, native editor/render evidence, focused test
results and named remaining costs. The actual Load Game wait must improve;
removing only the 5.1-second wait while leaving the measured resource bottleneck
untreated is not completion. Set a justified acceptance target after measuring
the baseline; report variability rather than one favorable sample. Do not claim
to beat the original game without a comparable original measurement.

Keep logs/assets local under ignored `TestResults/` or `output/`. Record decisions
and results in a dedicated dated loading note; make only concise shared-roadmap
updates to minimize collisions. State the readiness interface and any resource
path/support changes the Orrery/BDD task needs. Do not stop at profiling or a
proposal when the implementation can be completed.
