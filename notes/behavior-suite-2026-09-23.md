# Functional movement BDD checkpoint — 2026-09-23

## Decisions and evidence

The first behavior suite uses Cucumber 0.23 with one `movement.feature` and one
shared Rust step registry for synthetic and OpenSpace runs. Scenario words express
player outcomes. The registry chooses a backend; it does not duplicate step
definitions. Each selected scenario writes a JSON result with the physical
input recording and SHA-256, requested and actual event times, typed actor
captures, source-normalized position/heading/action, independent assertions,
fixture hashes, screenshot, cleanup, and an explicit pending Godot slot.

`replay::Recording` counts verified eligible input consumers. Functional
movement currently uses a separate elapsed-millisecond recording. Its XTest
press/release calls traverse the game's normal keyboard path, but the actor
samples are stopped-byte captures and do not prove completed-update timing.
The input schedule and 250 ms delivery deadline therefore support only a broad
functional claim. Samples have bounded capture intervals, and the final
settled observations must be separated by at least 300 ms of actual elapsed
time so a late capture cannot collapse the idle window. No frame or pixel
parity is claimed. A comparison function
checks two independently passing backend results for the same scenario and
recording hash, paired events, canonical actions, and source-relative position
and heading, with named functional tolerances. It leaves visual pairing and
consumer timing pending.

The supplied English `SaveGame0` reaches phase 9 `manoir` with Hype at idle
action 47. A private Orrery probe on 2026-09-23 observed physical Left for
500 ms select action 35, turn about 80 degrees, and return to idle without
translation. Following that turn, Up for 1,800 ms selected action 32 and moved
about 4.1 source units with little vertical change; releasing returned to idle.
The executable hash was
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`,
and the supplied save archive hash was
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The private probe evidence is in
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790143630023557425/`.

The saved actor faces a barred gate. An initial walking BDD run moved only
1.03 units before stopping, so the walking scenario now has a declared Given
setup: a physical Left press/release turns Hype toward the measured clear route,
then the runner verifies stationary idle before starting the walk recording.
This setup is logged as an intervention with before/after typed captures. It
does not alter game memory or produce the walking result in advance.

The first clear-route walking run measured a 1.597 radian setup turn and
3.692 units of displacement, with action 32 while held and action 47 at both
settled samples. The user observed that Hype hit another, farther wall in that
run. The suite passed because its standing check could not distinguish a
natural stop from wall blockage. A 1,000 ms run moved 2.655 units and still
advanced 1.772 units after the held checkpoint, but its settled screenshot
remained close to the farther wall. The walking hold is now 700 ms, and an
assertion requires additional displacement between a held sample and release.
The turn BDD run changed heading by 1.366 radians with no translation and
returned to action 47. The source runs use the
supplied save, complete cleanup, and reverify the installed fixture hashes.
The ignored evidence files are under `TestResults/` and are not committed.

The user observed startup stutter after loading. The original backend now
waits another 2.5 seconds after first seeing idle, then rechecks the manor
phase and idle action before either recording begins. This wait is outside
the timed behavior tape and is paid once in the shared movement run.

The 700 ms OpenSpace run passed all ten assertions. It moved 1.957 source
units, including 1.608 units after the held checkpoint, then continued 0.185
units while settling and showed no displacement across the final 400 ms idle
window. The settled position was about 1.60 units short of the farther wall
location reached by the longer run. Actions were 47 before input, 32 while
held, 37 just after release and 47 at both settled samples. The settled
screenshot shows the nearby wall but cannot itself prove clearance; the
position and continued held motion support the bounded clear-route claim.

The earlier standalone supervised movement selection ran 45 tests in 141.164 seconds:
45 passed, including both original-game BDD scenarios; 33 unrelated cases
were skipped. The supervisor reported complete cleanup and unchanged source.
The final turn changed heading by 1.405 radians without translation; the
final walk moved 1.910 units, advanced 1.573 units after its held sample,
then continued 0.250 units while settling. The report is
`TestResults/20260923T063428.734769Z-development.log.json`; the matching
scenario JSON and screenshots are in its `.d/` evidence directory. Workspace
Clippy with warnings denied, formatting, the synthetic BDD test, and the
locked offline release build passed. This validates the broad OpenSpace cases,
not the pending Godot comparison.

After session reuse and the 2.5 second startup settle, the supervised movement
selection passed **44/44 selected tests** in 142.991 seconds, with 33 unrelated
tests skipped, complete cleanup and unchanged source. Both OpenSpace Cucumber
scenarios passed in one 23.860 second game run. The suite report is
`TestResults/20260923T065340.538587Z-development.log.d/behavior-suite-1790146539607326178.json`:
it records one launch, shared session identity, the walk's checked
`compatible_previous_scenario` handoff, and final cleanup. Turn changed
heading 1.388 radians without translation. Walk moved 1.969 units, including
1.575 units after the held checkpoint, then continued 0.217 units while
settling. Both settled samples showed idle action and no later drift. Each
scenario has its own JSON result with its recording identity and pending
Godot slot. Formatting, workspace Clippy with warnings denied, synthetic
scenarios, and the locked offline release build all passed after this change.

## Design debt and next checks

- The original functional capture is coherent while the process is stopped,
  but has no completed-update publication. Tight action/latency claims need the
  qualified update observation path generalized to these scenarios.
- The clear-route setup must be reproduced or replaced by an equivalent
  validated start fixture in Godot. It is part of the scenario precondition and
  must remain visible in each backend report.
- The current standing result uses idle action and bounded displacement in a
  400 ms settled window. It does not directly measure collision contact or
  visually inspect the pose at each sample. Walk/run with contact and wall
  penetration are separate acceptance cases.
- The Godot backend is pending normal saved `manoir` AI/Brain/player movement.
  Its result must use the same recording, expose native physics observations,
  normalize coordinates into source units and canonical action IDs, and pass
  its own assertions before cross-backend comparison runs.
- Functional parity bounds are provisional and deliberately broad. Repeated
  source runs and the first Godot run should challenge sample timing, coordinate
  conversion, image pairing, and tolerances before adding behavior groups.
- Cucumber Rust 0.23 creates scenario-scoped worlds and offers scenario hooks,
  but no feature-level hooks. The outer movement runner now invokes the two
  Cucumber selections in explicit order and owns one optional Hype session.
  Before/after hooks transfer backend ownership between fresh worlds; each
  scenario retains its own assertions and final JSON. The turn recording's
  500 ms Left hold provides the clear-route state for walking. The walking
  Given rereads phase, level, idle action, position and heading against the
  turn's settled observation and records a linked handoff; it does not turn
  Left again. Failure invalidates the handoff and leaves later scenarios
  `not_run` in the suite report. Process cleanup occurs after the group and
  final cleanup status is written into both scenario reports. Single-scenario
  execution still initializes from the supplied save and performs the physical
  clear-route setup when walking starts alone. This avoids repeated slow game
  startup while retaining independent scenario evidence.
