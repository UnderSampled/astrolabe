# Run versus walk comparison in manoir

## Original-game source observation

The 2026-09-24 Orrery probe used the supplied English save and executable
(archive SHA-256 `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`,
executable SHA-256 `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`).
It verified phase 9 in `manoir`, turned left for 500 ms to the short clear
route, and made two requested 300 ms physical input trials from the same
position. The game process stayed PID 490400 while the supplied save was
reloaded between trials. The reload used Escape, Down three times, Return,
Left+Return to return to title, then Down, Return, Return to load the save.
The actor returned to `(-74.37859, 27.48541, 4.55085)` and essentially the
same heading before each trial. No game-data or save bytes were changed.

| Trial | Held input | Held action | Settled action | Settled horizontal displacement |
| --- | --- | ---: | ---: | ---: |
| Walk | Up | 32 | 47 | 1.0953 source units |
| Run | Shift+Up | 33 | 47 | 2.5206 source units |

The converted Hype normal 4 rule requests action 79 when running starts;
normal 6 changes to action 33 after that start action completes. The first
visible Godot BDD run sampled action 79 during the 300 ms hold, action 38 on
release, and action 47 after settling. The active-run assertion therefore
accepts the source-script start and sustained run actions. The earlier trace,
`TestResults/behavior-run-vs-walk-1790232030365747905.json`, failed its
narrower action assertion while passing its spatial checks.

The settled displacement ratio was about 2.30. Source height changed only
0.0044 during walking and 0.0104 during running. These are stopped-byte
captures. The original probe's `run_sequence` operations included input and
focus overhead: 649 ms for the requested 300 ms walk hold and 510 ms for the
run hold. The BDD recording declares its own physical edge schedule and retains
actual delivery times; it should establish an independent passing result before
claiming a cross-engine comparison.

The original trial screenshots are
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790230986359322750/screen-1790231033021751467.png`
(walk) and
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790230986359322750/screen-1790231151239585774.png`
(run). The earlier same-process turn/walk BDD reports are under
`/home/stephen/Projects/hype-harness/local/orrery-runs/behavior-suite-1790230848382949917.json`.

## BDD implementation status

The new Gherkin scenario pairs equal-duration walk and run trials. Each has
five named samples around a 300 ms hold. A recorded same-process save reload
and clear-route turn restore an equivalent start. Assertions check source
actions, displacement ratio, route direction, start equivalence, standing
after release and bounded height change. The source probe sets broad functional
bounds; it does not prove exact input-consumption timing. Native Godot uses its
saved Hype actor, compiled Brain, physical input events and normal scene
reinstantiation in the same engine process.

The fresh visible Godot BDD passed all three scenarios in one shared process,
with complete cleanup (`TestResults/behavior-suite-1790232242102012196.json`).
The run report at the same session ID records walk 1.0398 units, run 2.1120
units, ratio 2.0312, and only 0.0013 units of run lateral deviation. The action
traces were walk 47→32→37→47 and run 47→79→38→47; both settled with zero
velocity. The first original-game BDD attempt
(`behavior-suite-1790232802559811533.json` under the local Orrery runs)
passed turn and walk, then stopped in run-comparison setup: the batched menu
sequence left Hype at the preceding walk endpoint. This produced no original
run-comparison samples. The setup now requires observed pause phase 16 and
title phase 38 before selecting the save, and checks the actor's restored pose.
The pause/title guard was exercised in the next live rerun.

The first live rerun of the state-gated version
(`TestResults/behavior-suite-1790233509873387293.json`) again passed turn and
walk in one process, but the exit sequence remained in pause phase 16 and
never reached title phase 38. Its new guard correctly failed setup before
collecting misleading run samples. A controlled menu run identified the cause:
the quit confirmation starts on NO, and `Left+Return` as one chord does not
select YES before confirming. Separate physical Left and Return presses reached
title phase 38 and reloaded the supplied save, restoring the original pose
after a 700 ms walk. The BDD now uses those separate presses and awaits a
live rerun. The test supervisor also rejected the old
synthetic test name in its inventory before execution; the manifest now names
the current synthetic test, and focused `--list` selection succeeds.

The next original BDD run (`TestResults/behavior-suite-1790233879725961556.json`)
passed turn and walk, reached title and reloaded the supplied position, then
sampled transitional action 322 before idle. The reload setup now waits for
idle action 47 and then allows the same 2.5-second streaming settle used on
initial load. The next live run exercised this wait.

In `TestResults/behavior-suite-1790233970905906990.json`, the idle wait let
the first reload and clear-route setup pass. The second reload, between the
paired walk and run trials, remained in pause phase 16 instead of reaching
title. Turn and walk again passed in the shared process. A second-cycle menu
diagnostic showed that the pause menu resets to Resume game on both visits;
the shorter batched key timing was unreliable. Sending separate keys with a
longer 700 ms gap resolved this in the next BDD run.

`TestResults/behavior-suite-1790234256118531807.json` is the first fully
passing original movement suite: turn, walk and run-versus-walk in one launch,
with complete cleanup. The run scenario restored equivalent starts within
0.004 units and 0.003 radians. Over the paired 300 ms Up holds, the settled
walk covered 1.040 units and Shift+Up covered 1.625, a ratio of 1.563. The
held run action was 79. The earlier manual trial's 2.30 ratio was measured
through a different input/focus schedule and is not the BDD result.

## Wall-contact candidate and remaining debt

The finite-face oracle, source and Godot corner probes, and first wall BDD
result are tracked in [the wall-contact note](./wall-contact-2026-09-24.md).

A disposable Godot geometry probe from the supplied-save pose followed the
measured clear-route direction, approximately `(-0.965, 0, -0.263)` in Godot
coordinates. Middle and upper rays hit the saved static body
`world/1/5/0/Collision` after about 4.9 units, with an outward normal near
`(0.927, 0.084, -0.366)`. An approximate capsule sweep allowed about 3.56
units of travel. The saved Hype actor actually uses three radius-0.55 sphere
shapes, so the capsule result is a route estimate, not an actor clearance.
The node path also does not prove which visible surface was hit. The probe log
is `/tmp/astrolabe_walkpath_probe.log`; it did not exercise player movement.

A subsequent gameplay probe used the exported scene, saved Hype and Brain,
the English-save fixture, a physical 500 ms Left turn and an eight-second Up
hold. At about 4.1 seconds the actor reported a slide contact with the same
surface normal, stayed on the floor, and remained in walk action 32. It moved
along the surface by about 0.26 units per second during sustained input, so
this route is evidence of wall collision and sliding, not a full stop. The
held-contact screenshot is `TestResults/behavior-wall-probe-20260924.png`. A
shorter 250 ms Left turn was tested separately. It brought Hype into contact
with `world/1/5/0/Collision` after about 3.1 seconds of Up, with about 1.61
units/second of approach velocity projected into the measured wall normal.
From 3.1 through 7.1 seconds, horizontal position varied less than 0.001
units while walk action 32 and floor contact persisted. This route also touched
a second surface; the plateau may be a corner constraint. It is a useful Godot
candidate. A later original-game probe used the same 250 ms Left turn and six
700 ms Up segments, with stopped-byte captures between segments. Hype reached
about `(-79.823, 32.034, 5.007)` in source coordinates by 2.8 seconds of
accumulated Up, then moved less than 0.004 units along X in the next 1.4
seconds of input while action 32 remained active. This corroborates the corner
and sustained blockage, but segmented input and stopped-byte capture do not
prove an uninterrupted contact interval. The original screenshot is
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790234371304234345/screen-1790234438957593297.png`.

The subsequent wall scenario identifies the two finite source faces and
Hype's movement spheres, checks sphere-to-face clearance and held-contact
stability, and corroborates contact through Godot's native collision report.
A fresh exported Godot wall run passed all 18 assertions after setting the
native character's minimum wall slide angle to zero
(`/tmp/astrolabe-wall-contact-bdd-fresh/behavior-wall-contact-1790236718344678055.json`).
The original fixed-duration Given once turned too far and missed the corner;
the [wall-contact note](./wall-contact-2026-09-24.md) records the physical
heading corrections now under original-game validation. The current camera
wrapper is a presentation aid for gameplay observation; camera FoV and
placement belong to the separate camera slice.

The shared recording still encodes paired trials as an optional preparatory
trial inside a functional recording, and the original menu reload uses
game-specific key navigation. These are local to this first behavior slice;
revisit the setup interface if additional scenarios need different saved
states. Original stopped-byte observations do not identify exact AI input
consumption times, so cross-engine timing remains a functional approximation.
Headless Godot provides no rendered viewport screenshot in this harness; its
report now records `not_captured_headless` instead of treating that as a
movement failure. A visible run is still needed for any visual or contact
screenshot claim.

After that headless handling change, a focused Godot rerun against the same
saved converted project passed all three scenarios in one 67.5-second
process with complete cleanup
(`TestResults/behavior-suite-1790234555331291284.json`). Walk-and-stop
covered 1.885 units; the paired walk/run trial covered 1.085/2.112 units, a
ratio of 1.946. Explicit cross-backend comparison against the passing
original suite (`1790234256118531807`) passed all checks: 17/17 turn,
17/17 walk and 36/36 run-versus-walk. The three
`TestResults/behavior-compare-*-1790234256118531807-1790234555331291284.json`
artifacts retain the exact checks and matching recording hashes. These are
functional elapsed-time comparisons; input-consumer timing and completed
render pairing remain pending.

A later four-scenario original run passed turn and walk but failed the paired
run setup's `equivalent_heading_rad` check: two separate physical 500 ms Left
turns from the same supplied-save pose settled at -1.73695 and -1.86262
radians, 0.12566 apart versus the 0.08 bound. A prior passing pair settled
at -1.77046 and -1.76767. The clear-route Given now targets -1.77 ±0.035
by reading the settled heading and applying bounded physical turn pulses when
needed. It records every correction. The full original four-scenario rerun
passed in 115.16 seconds with one shared game launch and complete cleanup
(`TestResults/original-four-movement-20260924/behavior-suite-1790237298354606540.json`).
The paired walk/run headings were -1.77325 and -1.77884, only about 0.0056
radians apart; run-versus-walk passed all 17 assertions. Turn, walk and wall
contact passed 7/7, 10/10 and 16/16 assertions respectively. The full
fresh-export Godot four-scenario suite also passed in one engine launch with
complete cleanup (`TestResults/fresh-four-movement-20260924`, session
`1790237492624866771`). Its test body took 256.21 seconds, including import,
export, startup, scenarios and cleanup; the manifest's Godot per-test timeout
was raised from 240 to 360 seconds to cover this observed run with reserve.

In the first comparison of these complete suites, turn, run-versus-walk and
wall contact passed 17/17, 36/36 and 23/23 checks. Walk-and-stop matched
positions and heading but its 450 ms held sample caught original walk-entry
action 78 and Godot sustained walk action 32. The source scripts use 78 in
normal 4 to enter walking, then 32 in normal 5; the independent walk scenario
already accepts both as active walking. The comparator now checks that same
source-backed walking phase at held-walk checkpoints in both walk-and-stop
and the preparatory walk of run-versus-walk. All other action checkpoints
remain exact and raw IDs remain in the reports.
The refreshed four-pair comparison passed with matching recording hashes:
17/17 turn, 17/17 walk, 36/36 run-versus-walk and 23/23 wall-contact
checks. Exact action IDs and both elapsed-time captures remain available in
their independent backend reports; this is functional comparison, not an
input-consumer frame or completed-render parity claim.
The four comparison JSON files are retained locally under
`TestResults/comparisons-20260924/`.

The supervised focused movement gate selected 56 checks and passed all 56
in 389.53 seconds, including both four-scenario engine BDD tests and mandatory
safeguards (`TestResults/20260924T082252.694345Z-development.log.json`). It
excluded 88 unrelated checks and reported 33 inventory skips. Its original
and Godot BDD tests took 113.048 and 160.001 seconds, each with one launch
and complete cleanup; the overall run stayed within its 480-second budget.
The supervised run's fresh comparison reports are also under
`TestResults/comparisons-20260924/`.
