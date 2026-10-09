# Save selection and startup restoration — 2026-10-09

Isolated implementation from `12ad58ce49d60b10cdcbb80a29c9118a5e9feffa`,
branch `codex/save-startup-sol-20261009`, in
`output/worktrees/save-startup-sol-20261009`. Main, generated content, source
disc, user saves and settings are unchanged. Kip owns composition/publication;
Luna owns all runtime execution. This record is static implementation evidence,
not acceptance of the loaded game.

## Two observed blockers

The real user slot
`user://manoir-save-v1.json` has SHA-256
`2974fc81b714ba11691b7bec03abf4df8de8191f7ec5fc3cccc2b18d7397e9eb`.
It is schema 1 with inventory quantities 21/61/80 equal to zero, weapon 80,
mana 30/50 and dummy DSG8 false. It has **no Hype transform, Global Brain,
opening/gate progression or gold**. The baseline bootstrap accepted it;
`open_level` gave it precedence over the original import and applied the measured
English fixture plus this sparse overlay. Scene Global147 therefore stayed false.
Generated Gogoud model210 Normal13 reads Global147 before choosing deactivation
or its introduction. This explains why the partial native file can hide the
authentic original import and replay the introduction. It does not prove the
reported barrel contact/position or the complete control failure in runtime.

Luna's independent actual-menu baseline used an isolated profile without a
native slot and selected the original import. It reached `tree_ready`, then
failed restoration with `Saved inventory item is outside the source catalogue`.
See `output/runtime-validation/luna-20261009/LUNA-BASELINE.md`, `load-report.json`
and `load-godot.log` in the canonical checkout. That run did not reach controls,
player position or camera acceptance. Timing labels differ by observer/session
origin; use the preserved receipt rather than treating one elapsed value as exact.

The source inventory join (`src/openspace/level/saved_inventory.rs`) resolves
item0 through the Fix HUD currency composition to its handle quantity and
definition capacity. `InventoryCatalog` and its exporter separate this resource
from ordinary items. In actual `shared/inventory/gameplay.tres`, item0 is the
`gold_definition`, outside `definitions`; `restore_legacy` correctly rejects it
as an ordinary item. The importer incorrectly supplied `quantities[0]=20`.
New imports carry `inventory.gold=20`. Existing imports resolve item0 against
the initialized native gold definition, reject ambiguous/incorrect bindings
and conflicting values, and restore the same balance in the gold component.
Unknown ordinary items still fail. The user file is never silently upgraded.

## Production changes

- Bootstrap now opens the native Load Game picker. Each existing native or
  original file retains its source, path, and original slot/display identity.
  Invalid candidates show their reason and cannot be selected. An original
  import remains selectable when an unusable native file exists.
- `validate` remains the component-overlay contract. `validate_load` requires
  position, basis, Global147/149, dummy world state, and the original balance or
  complete native inventory snapshot. Direct session entry preserves native
  precedence, with an explicit failure rather than an implicit alternate save.
- `open_level` reads the selected source and never starts a fixture-only game
  when both files are absent. The source remains explicit through an R reload.
  New Game clears the selected Load Game source.
- `save_startup.gd` holds every compiled Brain's automatic entry before the
  level enters the tree. After native character `_ready` establishes activation,
  it captures/holds character physics, applies fixture and saved component
  values, restores only still-active character physics/contact, and starts
  compiled Brains with their original automatic policy. Unbound pooled
  definitions remain unstarted; existing started continuations are preserved.
  Inactive banks and consumed/removed actors are not reactivated.
- Load readiness requires a running supported player Brain, active player
  physics and the source gameplay state, rather than setting success after
  an inventory failure or missing save. Gate lookup uses the level's semantic
  `level:/persos/world/0/15` Brain binding, preserving authored node renames.

Camera handoff remains owned by the camera worker. Its integration hooks belong
after contact preparation/before Brain startup and after the first Brain frame.
The baseline three DemoCamera calls are deliberately left for that worker's
registered-source-camera correction. This branch alone is not camera acceptance.
No dialogue suppression or raised-gate transform is added.

## Exact supplied fixture expectations

The unchanged archive `reference/Savefile_EN.zip` has SHA-256
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The existing import has SHA-256
`8d20ef3de198f53c0ca86ba11c3227145496a939e5ff2eaaeba1a2c2a7153c67`,
profile `hype-pc-english-supplied-save`, slot `SaveGame0`, display
`Gogoud's Manoir I`, level `manoir`, timer 249589ms.

The Level000 player POS record `2:6:1463692` has source XYZ
`(-74.3814620972, 28.5058479309, 4.5686554909)` and converts to Godot
`(-74.3814620972, 4.5686554909, -28.5058479309)`. Its recovered Godot basis
columns are approximately `(-0.99999577,0,-0.00291785)`, `(0,1,0)`,
`(0.00291785,0,-0.99999577)`. Restore Global147=true, Global149=true,
dummy8=false, gold20, mana30/50, magic61, and supported ordinary quantities
21/54/61/74=-1 and 80=8. The gate's compiled saved branch moves sideways in
source X; no vertical intervention belongs in loading.

The source reconstruction worker's `e7bf0a01ea6bb6061451d65d44d7507f668e7d96`
in the separate hype-priority checkout establishes that reader0043b490 loads
two 0x58-byte placement blocks from `*.ext` extras/preview files. Its scan and
selection callers use filenames from that enumeration. The supplied ZIP has
only General.sav, General000.sav, Level000.sav and manoir.sav, so this does not
provide another file-backed player-placement override. Source00402910 and
00402b60 give paired explicit player/camera object names priority over those
extras blocks, and restore before the world initialization event. The current
Load Game route provides no such paired placement names. Remaining source
position-copy/flag contracts are not promoted to guessed Godot behavior.

Read-only checks of both preserved installed SaveGame0 roots (current Wine
installation and its before-directshow snapshot) find no `.ext` companion.
Both Level000 files hash to
`6786fe4f781b04d1c674e33f04f40b95c7ec9f1f82e0cff5d56574b1cddb672f`
and decode to the same player POS as the archive. The direct load path is
source00402780 ->00431760 (selected `Level%03d.sav`) -> SAI mode4 ->004390e0
-> world event2. Source0040f4c0 registers MainChar's SuperObject+24 as
DAT00610534;004390e0 reads that restored POS, updates main-actor sector/placement
and copies its matrix. Generated `source_main_actor` and `source_actor_nodes`
both bind `level:/persos/world/0/0` to `world/0/0/Hype`, whose source metadata
matches. No wrong-player binding or missing transform-file requirement was
found. The old fixture seed differs from the serialized player translation by
about 1.0193 source-Y units; explicit save selection prevents inheriting that
fixture placement from the incomplete native overlay.

Prior unchanged-source original controls in
`output/savegame-investigation/source-no-action-control-proof-1790557697785110045/`
loaded this archive, physically walked to the lectern, and saved normally.
Both reports changed exactly timer, Hype POS and camera POS. The separate
[dated results](savegame-results-2026-09-27.md) describe that route; they do not
replace a new unchanged-original reached-state/collision comparison. Additional
read-only source receipts are preserved under the worker's ignored
`output/save-startup-sol-20261009/` directory.

The user's lectern expectation still needs an original-game reached-state
observation. The archive's actual player POS is not replaced by a guessed
lectern transform. Original active weapon, complete current Brain/action,
health, armor, inventory admission and camera POS ownership are still incompletely
mapped; the existing measured English fixture supplies the documented bounded
behavior/action seed. That limitation remains visible, and this work does not
claim full source-slot restoration.

## Validation and next single-slot observation

No Godot launch, native import, heavy build or runtime test ran in this worker.
`git diff --check` passes. Focused Rust formatting passes for edited Rust files;
the full `cargo fmt --all -- --check` reports pre-existing formatting differences
in unrelated baseline files, which were not rewritten. Synthetic assertions
are prepared in `godot/tests/save_load_selection.gd`, packaged by
`crates/godot-harness/tests/save_load_selection.rs` and declared in the manifest.
They cover completeness, explicit false, source identity, gold zero/balance,
unknown-item rejection, and all-Brain startup ordering. Two native invocations
are bounded at 10 seconds each, with a 30-second manifest test budget. They are
**not run**. Luna must perform syntax preflight before the actual-game run.

For Kip/Luna's next coordinated experiment:

1. Use a private profile with a byte-for-byte copy of the actual partial native
   file and the authentic original import. Click the actual menu Load Game
   control, record both candidates/reasons, and explicitly select SaveGame0.
   Record the selected source/path/hash and session `selected_save` receipt.
2. Sample Hype transform, Global147/149, dummy8, gold and ordinary quantities
   immediately after apply, after first/second contact frames, after Brain
   startup and at readiness. Confirm no earlier Brain entry saw scene defaults.
   Record Hype's Normal/Reflex/action, failure, activation/contact, game-state
   mask, world/Gogoud Brain selections, UI fade/widescreen and actual camera
   owner/current/target/mode. The camera worker must validate its handoff.
3. Observe the compiled gate's lateral movement, and the Gogoud branch chosen
   from restored Global147. Give one bounded forward input after real readiness;
   record displacement/contact and screenshots. A ready label is insufficient.
4. Write/reload a supported settled native checkpoint in the isolated profile,
   verifying its explicit source remains native. Missing/corrupt/incomplete
   files must show their reason and never start a new game or alternate save.
5. Independently load the unchanged supplied archive in the original engine,
   verify edition/save root/reached scene, and sample Hype's source transform
   and first subsequent decisions before resolving the lectern discrepancy.
   Record any named entry placement or save intervention. No teleport is a test.

The existing older `save_persistence.gd` scenarios include stale schema1 mock
writes and direct one-click loading expectations. They require reconciliation
with the actual picker/schema4 checkpoint contract before treating them as
current acceptance; no declared release coverage should be silently omitted.
