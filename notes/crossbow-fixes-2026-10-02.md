# Crossbow contact and ammunition fixes — 2026-10-02

This continues the clean `5047f3c` handoff in the local Rust working tree.
The measured contact/lifetime values and validation results are recorded below.
Magic is outside this change.

## Independently established source facts

The pinned executable is
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
The supplied English save archive is
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
Orrery loads that fixture into `manoir` and reads the active inventory handle's
quantity at `+0x0a`. Its initial Weapon 80 quantity is **8**; Fire 61 and Sword
21 have unlimited quantity `-1`. The earlier zero baseline came from a
different record member and is superseded.

A read-only main-actor inspection captures action arrays 159–164. The saved
Godot start previously restored only 162 and 164. In the rendered baseline,
Z selection requested action `-1` twice, returned to unarmed idle and never
created an arrow. Restoring the six measured arrays repairs the fixture;
it is distinct from implementing projectile motion or contact.

The typed mechanics result reader follows the pinned query at `00463df0`:
normal `+0x0c`, contact point `+0x18`, hit SuperObject `+0x5c`, result length
`0x70`. Mechanics retains ground and wall/ceiling records at `+0x16c` and
`+0x1dc`. The class publisher's previous/current/AI-visible words remain
separate. Unknown result fields are opaque; these are transient live
diagnostics and are not stored as source addresses in Rete.

The model-733 bolt's wall record has class 4, result kind 2, normal
`[0, -1, 0]`, and contact near `[-73.97907, 29.08693, 6.09258]`.
Its target object types **29/42/108** identify `iCheckPoint_Manoir_MAD`, whose
canonical asset is `level:/persos/world/0/35`. The capture-local path is
`[0, 34]`: original gameplay reorders DynamicWorld. The preceding investigation
incorrectly joined that ordinal to the on-disk load-marker ordinal. Likewise,
the static `world/1/5/0` contact in an unfiltered report belongs to Hype,
not the bolt. Do not use runtime ordinals as persistent actor identities.

The checkpoint's preserved profile `level:/collision-profiles/4373` selects
zone `level:/collision/character-zones/49495` for state 0. Its source box spans
`[-0.5, -0.4, 0.05]` to `[0.5, 0.2, 1.85]`. The observed saved translation is
`[-74.378685, 29.486925, 4.589164]`; thus the independently decoded minimum-Y
plane is **29.086925**. The Godot saved-state fixture restores that actor pose
and checks its exported identity. No original asset is altered to establish it.

Initial source free-flight velocity was
`[-1.331904, 44.977348, -0.513735]`, equal to the observed arrow sighting times
45. The creation/release path `0043bcd0` / `005711f7` applies the animation
attachment transform before activating mechanics. Its collision path
`00464570` / `00463df0` excludes the generating Perso.

## Implementation

- The maintained native character component retains the generation vector
  until the first physics tick after release. It commits using the released
  actor's basis, rather than permanently seeding velocity from Hype's earlier
  spawning basis.
- The exporter saves state-selected reaction-zone geometry as native ZDR
  `StaticBody3D` children on layer 8. Type-11 generated bodies include that
  layer in their native contact mask and exclude the creator's owned bodies.
  ZDE query areas and ordinary character movement retain their roles.
- The authored English-save fixture restores the six verified action arrays,
  actual inventory and checkpoint pose. Fixture recovery is not proof of a
  general save decoder or full saved-world restoration.
- Support/exporter interface is 40. Contact metadata identifies the owning
  Perso without making a collision body appear to be another character.
- The test harness observes after native physics through a test-owned Node.
  Render stalls can no longer hide an entire short projectile lifetime.
  Physics clock and wall-clock timestamps are both retained.
- The shared Crossbow assertions reject the former later static hit, incorrect
  launch direction and excessive physics lifetime. The source contact must
  identify the checkpoint and its plane/normal, not merely set a nonzero word.
  Synthetic negative controls explicitly reject wrong contact and late cleanup.
- The isolated source reload test follows **8 → setup 3 → physical shot 2 →
  game-written save → fresh process 2**. Expected values are fixed independently
  of the returned reload value. User saves and the original installation are
  preserved by the existing Orrery staging/backup/cleanup paths.

## Validation record

Initial supervised baseline:
`TestResults/crossbow-baseline-sol-20261002.log.json` (101/104 passing,
448.703 seconds including setup). Godot Crossbow missed the shot; the old
zero-baseline reload assertion and a rendered sword trace also failed.

The first focused source diagnostic passed both tests in 82.904 seconds,
following a 29.75-second build. It independently demonstrated the actual
baseline, one-bolt consumption, typed checkpoint contact and persistence of
the guarded quantity three. Evidence:
`TestResults/crossbow-source-diagnostic-sol-20261002/`.

A direct rendered diagnostic inherited desktop `DISPLAY=:0` and stopped
before gameplay because no live Xvfb owned it. The supervised runner supplies
the private display. An intermediate combat run passed 103/104, with native
Crossbow hitting Hype's reaction body. Its report is explicitly stale because
implementation changed while it ran; it is diagnostic evidence, not the final
gate. A subsequent build-only attempt exposed the borrowed input-event error
in the strengthened save test; no gameplay ran in that attempt.

Final validation is recorded in
`TestResults/crossbow-acceptance-combat-sol-20261002.log.json`.
Command:

```sh
python3 tools/test.py --area combat --local --offline --no-headless --log TestResults/crossbow-acceptance-combat-sol-20261002.log
```

The selection ran **104 tests: 103 passed, one failed**, with 87 capability/area
exclusions listed in the report. Total time was 480.345 seconds, including
469.190 seconds of tests. Source was unchanged during execution; every stage
completed cleanup with no timeout, leaked child or remaining process. Both
rendered gameplay cases, both original-game cases and the strengthened save
case passed. The failure was the unchanged Orrery test
`native_and_i386_scratch_lifetimes_are_bounded`: thread-state inspection during
scratch close returned `ESRCH` ("No such process"). Its thread-churn fixture is
a possible cause; the exact race is not established. Three bounded isolated
reruns passed in 0.166, 0.161 and 0.165 seconds without leaks. Their exact
nextest command and supervisor results are retained in
`TestResults/crossbow-orrery-scratch-recheck-sol-20261002/report.json`.
The failed full selection is retained and is not relabeled as a clean gate.

The final rendered Godot Crossbow case passed in 125.079 seconds including
fresh conversion/setup. Its report is
`TestResults/crossbow-acceptance-combat-sol-20261002.log.d/behavior-crossbow-initial-launch-1790981528281390308.json`.
Release is physics frame 725; a no-contact moving sample is frame 726;
tree exit is frame 728 in compiled normal 1. The arrow clock advances
**50 ms**, while wall timestamps span **255 ms** because rendering stalled.
The firing guard is observed at 1 in flight and 0 at exit. The contact body is
the checkpoint's saved `ZDR`, with native point converted to source axes
`[-74.045174, 29.086926, 6.075004]` and source normal `[0, -1, 0]`.
The final paired original point is `[-73.946861, 29.086926, 6.092501]`; the
point difference is **0.099857 source units**. Both hit the same plane with
the same normal, but their lateral coordinates differ. This is contact
evidence, not pixel-perfect or full-trajectory parity.

The original Crossbow case passed in 20.983 seconds and its sword regression
in 22.348 seconds; rendered Godot sword passed in 118.932 seconds. The source
reload case passed in 65.644 seconds, independently demonstrating eight in
the supplied save, guarded setup three, physical-shot two and fresh reload
two. Its report is
`TestResults/crossbow-acceptance-combat-sol-20261002.log.d/source-crossbow-proof-1790981815624951242/report.json`.
The same directory retains baseline/intervention inventories, physical event
delivery, real lectern save differences and fresh-process inventory.

The maintained pairing CLI passed with:

```sh
target/validation/behavior-suite --openspace TestResults/crossbow-acceptance-combat-sol-20261002.log.d/behavior-crossbow-initial-launch-1790981772293315901.json --godot TestResults/crossbow-acceptance-combat-sol-20261002.log.d/behavior-crossbow-initial-launch-1790981528281390308.json --output TestResults/crossbow-acceptance-combat-sol-20261002.log.d/crossbow-backend-pair.json
```

Raw contact coordinates, release/exit timing and published original positions
are additionally summarized in the sibling `crossbow-contact-pair-diagnostics.json`.
Source samples before release show the newly created arrow at its owner's
pose; that held interval must not be counted as flight lifetime.

Build/style checks passed:

```sh
cargo check --workspace --all-targets --locked --offline
cargo build --release --locked --offline
cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
```

Their logs are `TestResults/crossbow-preflight-final2-sol-20261002.log`,
`crossbow-release-final-sol-20261002.log` and
`crossbow-root-clippy-acceptance-sol-20261002.log`. The release build took
46.57 seconds. An extra workspace-wide Clippy run stops on the pre-existing
`too_many_arguments` warning in `crates/orrery/src/orrery.rs:673`
(`observe_magick_behavior`); no unrelated Magic refactor was made.

## Changed files

- Native export/support: `src/godot/character.rs`, `src/godot/support.rs`,
  `godot/addons/astrolabe/character/character.gd`,
  `godot/addons/astrolabe/version.json`.
- Saved start and native observation: `godot/game/english_save_manoir.json`,
  `godot/game/manoir_start.gd`, `godot/tests/behavior_gameplay.gd`.
- Original observations: `src/openspace/level/live.rs`,
  `src/openspace/level/source_view_tests.rs`, `crates/orrery/src/behavior.rs`.
- Assertions and test inventory: `crates/behavior-contract/src/crossbow.rs`,
  `crates/behavior-suite/tests/behavior_synthetic.rs`,
  `crates/behavior-suite/tests/support/behavior_steps.rs`,
  `crates/behavior-suite/tests/save_original.rs`, `tools/test-manifest.json`.
- Documentation: `docs/astrolabe/testing.md`, `notes/README.md`,
  `notes/implementation-progress.md`, `notes/crossbow-handoff-2026-10-01.md`,
  this note.

## Acceptance limits

This scenario checks the supplied manor save, a camera-mode shot, a nearby
checkpoint reaction-box contact, native compiled cleanup and ammunition.
The source's exact deletion caller and tick phase are not established by
stopped snapshots. Godot physics lifetime is not its wall-clock duration
during render stalls. General type-11 mechanics, other collision eligibility
paths, enemy damage, original distant-flight expiry, full camera/laser behavior and
pixel-perfect trajectory/render matching require separate scenarios. The
existing paired-report CLI still describes its Crossbow comparison as an
initial-launch case; independently passing contact assertions are stronger
evidence than that old comparison label.

The subsequent [free-flight followup](crossbow-free-flight-2026-10-02.md)
records native counter-expiry/aiming regression coverage and the remaining
original missed-shot and enemy-damage fixture gaps.
