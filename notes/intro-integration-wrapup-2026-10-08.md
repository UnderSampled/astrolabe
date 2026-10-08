# Manor intro integration checkpoint — 2026-10-08

Follow-up: the [integration disposition](integration-disposition-2026-10-08.md)
records the resumed main-checkout audit, inventory/loading work, reconciled
evidence and separately assigned title/pickup/armor/camera/save contributions.
The parked state and “no production cap patch” below describe this checkpoint.

Root content is rebuilt, but the complete opening is not accepted. Work is
parked at the user's request; no next implementation batch is started here.
The real launch path is `godot --path /home/stephen/Projects/astrolabe/godot`,
whose entry is `res://game/bootstrap.tscn` and New Game uses the complete saved
Manor scene. There is no separate showcase or source-state intervention.

## Main and ownership

The production and observer checkpoint before this note is `ae4d8b1`; the last
runtime/export fix is `bac48e1`. Rete55 and support interface72 are coordinated.
All shared audio, Character, AI, model, parser and exporter ownership has returned
to the lead. Source-only worker checkouts are preserved and parked.

Subsequent Git housekeeping preserves and tracks the 21 `.uid` sidecars created
by destination Godot import: five maintained support assets, three authored game
scripts and thirteen test/observer scripts. Every owner is tracked, and the UIDs
are valid and unique across maintained sidecars. The eight packaged owners also
include their sidecars in the support/starter inventories; no interface changes
are needed. The original inventory remains in ignored
`output/integration-resume-20261008/native-uid-sidecars.json`. No game assets,
saves, reference submodules or user files were discarded. Generated root content
and all evidence remain local; no gameplay work resumed for this housekeeping.

## Completed and unmerged pieces

| Piece | State and commits |
| --- | --- |
| Camera semantics, target binding and mode6 | Integrated: worker `ffd1220` as main `ab7bacc` |
| Point-turn geometry, actor waypoint progress and native point/waypoint motion | Integrated: `c6e266d`/`342e80c`/`52e9278` as `aeffc28`/`93f9f39`/`e67e55e` |
| Initial music, receiver sound/fade, listener selection and long ordered themes | Integrated: `5b573fd`/`1340390`/`ed7a4f5`/`dc5fe646` as `1f5b7c9`/`dcf30f3`/`f2f3468`/`52cb522` |
| Typed finite intro calls, source/Rete fields, compiled motion, authored handoff and passive observer | Integrated lead batch, including `8d68004`, `9d93f17`, `6e04acb`, `46f1fdc` |
| Shared sound output ownership | Integrated `bac48e1`; independent actor traversals share native scripts and waveforms while retaining distinct event/voice dependencies |
| Optional pickup/barrel primitives and audit | **Unmerged**, `1aa775e67368980153c11fc502d36ee3c64dc219`, docs correction `ed961c7`, clean branch `codex/manor-pickup-barrel-20261008` at `output/worktrees/manor-pickup-barrel-20261008` |
| Optional native location title and audit | **Unmerged**, `e9296e88f92940d0e73ac206820e041cacd19181` and `f7dc29d915c57053392d156babf1fb6adf5d7795`, clean branch `codex/location-title-20261008` at `output/worktrees/location-title-20261008` |

The optional pieces are unregistered components, not acceptance of their whole
unchanged source AI. Pickup/barrel integration still needs source-qualified
collision/material queries, actor fields, waits, picking-center and persistence
work. Title integration still needs Func195, the catalogue/style4 join, shared
screen dispatch and package/test registration. They were kept outside the intro
batch. Their focused checks pass2/2 and3/3 respectively; exact limits are in the
worker audit notes. Native inverse/exceptional floating-point equivalence remains
explicitly unproved in the pickup candidate.

## Validation and actual root results

- The coherent release build passes in64.85s. Strict root all-target Clippy,
  formatting and diff checks pass. Integrated AI26, intro source/Rete1, native
  route/contact1 and native point/waypoint1 pass together in30.50s. The focused
  local source/camera/travel checks and all four mandatory safeguards also pass.
- All13 collected music bindings have actual waveform closure (40.60s); native
  initial music, long PCM theme and microphone/mixer checks pass. The shared
  output regression passes, as does the existing native mixer check (6.44s).
- First preparation failed before publication on duplicate shared sound output.
  The reviewed retry passed in410.53s: Manor8,785 outputs, menu88, inventory192,
  pause-menu27. It exited0 with no timeout or leaked/remaining child processes.
- Two uncapped actual bootstrap/New Game startup observations failed readiness
  at70s. The second records the native resource fraction stuck at0.5 from15s
  through65s. No full dialogue/camera/handoff stage was observed. Exit-only sound
  dependency errors differ between attempts; malformed audio is **not proved**.
- Fresh complete native resource-only loads pass headless in20.16s and rendered
  in24.93s, without instantiating actors or running AI. Both report43 cached
  ObjectDB instances at exit; their resource results succeed, not clean whole
  gameplay acceptance.
- The final controlled **temporary 30-FPS startup diagnostic** reaches resource
  completion20.42s and actual New Game readiness41.31s. It restores the prior
  cap0 before quitting, exits0 in45.97s and cleans every child. This is observed
  readiness improvement, not proof of a particular engine lock/starvation cause.
  No production loading-cap patch was authored or applied.

The last diagnostic then records16 error lines (14 unique). Two are critical
to the unchanged opening: World `level:/ai/models/154` requires native shared
armor with a selected definition, and Camera `fix:/ai/models/437` stops its
position update because the active mechanics identity-card callback is unmapped.
The remaining ambient/optional behavior errors are preserved individually;
they are not all claimed to gate the finite opening.

The full passive intro, mode6 shot, eight-dialogue chain, letterbox release and
physical player movement **remain unrun/unaccepted**. No original-engine run,
full release matrix, install, push or destructive cleanup was performed.
The earlier broad debug audio corpus timeout remains a failed check; focused
passes do not supersede it as a full-corpus result.

## Preserved evidence and next plan

Ignored evidence is under `output/integration-resume-20261008/`:
`prepare-game-summary.json`, `prepare-game-reviewed-result.json`,
`actual-intro-observed.json`/run/engine/supervisor logs, the inspected
`load_timeout.png`, and `startup-cap-evidence/diagnosis.json` with its native
logs, harness and exact post-load error list. The rendered resource comparison
is copied into `rendered-resource-evidence/`; worker originals are also retained.
The 30-FPS harness changes only the native startup frame cap and physical menu
input, never actor/source state. Full intro acceptance was not its scope.

Recommended next steps, for discussion before resuming:

1. Review a service-owned loading-phase cap for `LevelTransition`, preserving
   lower user caps and restoring on completion, cancellation, failure and exit;
   verify its lifecycle with small native checks. The fallback session path has
   a cap, but the initial service path currently bypasses it.
2. Trace the actual first-visit selected-armor binding and active target mechanics
   callback, repair those two critical consumers as a coherent source-backed
   batch, and check their source/Rete/export/native behavior. Do not infer the
   callback from a mechanics label or treat a missing binding as success.
3. Run one bounded unchanged actual-game intro acceptance after that batch.
   Review optional peer components separately once the opening is accepted.

Global null-Micro pose publication, speech driver-clock calibration/lip selection,
cross-active-theme switching, later curved routes and exceptional x87/contact
parity remain fidelity limits. They were not observed as causes of the current
readiness or critical armor/camera failures. Blinded font comparisons remain
untouched; no further original-source reservation is requested by this checkpoint.
