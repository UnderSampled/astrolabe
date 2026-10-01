# Crossbow use handoff — 2026-10-01

## Current state

Crossbow selection, camera aim, Space firing, initial arrow motion, and one-bolt
consumption are implemented and have passed in focused Godot and original-game
runs. The overall **Fix Crossbow use** goal remains open because source contact
classification and the arrow's removal path are not yet matched in Godot.

The sword-beam work is relevant at the shared projectile creation boundary:
both paths copy the caller transform and start the new Brain after assigning
that transform. Beam-specific mesh placement and animation findings do not
automatically apply to model 733.

## Committed changes

- `fa0b521 Support horizontal Crossbow aiming`
- `a585e7e Use live Crossbow inventory handle quantity`
- `297f9bc Trace Crossbow native contact paths`
- `5382213 Trace nearby Crossbow contact actors`
- `295fecd Record nearby Crossbow contact actors`

The inventory reader/setup now uses the active handle's `+0x0a` quantity, as
Hype's inventory functions do. The source and Godot Crossbow checks have both
observed quantity `3 → 2` after one firing action. Orrery also captures source
actors near the observed arrow contact point and includes them in the report.

## Working tree changes to commit

- Added the three Crossbow tests to `tools/test-manifest.json`; before this,
  the normal focused runner rejected the suite because they were unclassified.
- Updated inventory memory fixtures to model the 12-byte handle and its live
  quantity field.
- Updated the synthetic Crossbow trace to model the Godot release, flight,
  contact cleanup, and inventory evidence used by current assertions.

These changes passed `cargo fmt --check` and the focused three-test nextest
selection: both inventory tests and
`synthetic_crossbow_initial_launch_requires_two_positions` passed.

## Verification evidence

- Direct original-game Crossbow run passed all five BDD steps:
  `TestResults/behavior-crossbow-initial-launch-1790858766145048614.json`.
- Godot Crossbow case passed in the focused combat run in 129.338 seconds.
- Godot sword attack passed in 137.812 seconds; original sword attack passed
  in 24.161 seconds.
- The focused combat runner selected 104 tests and completed 102 successfully
  (87 were skipped). Report:
  `TestResults/crossbow-focused-combat-fixed-20261001.log.json`.
- `cargo fmt --check` passed. The combat-runner manifest discovery passed after
  adding the missing test entries.

The completed combat run had two failures:

1. `behavior-suite::behavior_original/original_crossbow_initial_launch`
   completed all five Gherkin steps, but its contact assertion failed because
   this run's bounded burst did not capture a publication-consistent nonzero
   contact word. Its quantity, aim, launch, displacement, and later-absence
   assertions passed. A previous original run did capture contact word 4, so
   the contact evidence varies between runs.
2. `behavior-suite::save_original/original_crossbow_quantity_resets_after_reload`
   failed with `source crossbow baseline changed`. This was not investigated
   further during handoff preparation.

## Unresolved work

- Identify the source target/contact event that produces model-733's published
  contact word and reproduce that result through native Godot collision data.
- Explain and match source arrow cleanup. Source reports show model 733 absent
  from a later full ActualWorld scan, but do not identify the removal caller.
- Godot's diagnostic contact is static node `world/1/5/0` near source-space
  `[-74.8635, 32.6040, 6.0810]`; an earlier source contact observation was
  around `[-74.034, 29.417, 6.073]`. These points do not establish the same
  contact event.
- The nearby-source-actor scan found the load marker (`world/0/34`, 1.54 units),
  bridge gate (`world/0/15`, 2.41 units), and blue potion pickup
  (`world/0/175`, 2.49 units). Their generated scenes have no ZDE areas; the
  scan does not identify them as the contact target.
- Resolve the save-reload test's baseline failure before treating the focused
  combat validation as green.

Detailed investigation history and source references remain in
[`combat-comparison-2026-09-24.md`](./combat-comparison-2026-09-24.md).
