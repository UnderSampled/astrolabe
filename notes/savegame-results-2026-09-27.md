# Save investigation — 2026-09-27 checkpoint

This is a bounded checkpoint against the [save handoff](savegame-investigation-handoff-20260927.md),
not acceptance of general save compatibility. The
source fixture is `reference/Savefile_EN.zip`, SHA-256
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The prior controlled captures remain under ignored `output/cheat-save-research/`.
Current reports are under ignored `output/savegame-investigation/`.

## Private display validation

Orrery's authenticated MCP `start_game` used Gamescope's headless backend with
no inherited desktop display. Its private Xwayland display accepted physical
menu input, loaded the supplied save into playable `manoir` (phase 9), and
returned Hype's typed idle action 47. Independent title, save-list and manor
screenshots are under `output/savegame-investigation/headless-mcp-smoke/` with
the request/response log and cleanup record. The pinned archive and executable
hashes matched; MCP stopped the session, and no Hype or Gamescope process
remained. This was a display/input smoke, not a source-save proof. Game audio
was still audible on the desktop during this check. Orrery subsequently gained
a private game-only audio route; a separate live full-record run captured 30.582
seconds of stereo 48 kHz FLAC from its null-sink monitor, armed 616 ms before
Wine launch, and removed the private server on cleanup. Later source-save tests
use that route without changing the desktop's audio output.

A rendered Godot input/screenshot smoke used an authenticated private Xvfb
display (`:100`) and captured a nonblank frame without showing a window or
changing desktop focus. Its log and frame are under
`output/savegame-investigation/godot-xvfb-smoke/`. The native save test uses
the same private Xvfb display to render its three stages.

## Structure catalogue, version 1

| Category | Owner, current coverage and evidence | Named gap and next check |
| --- | --- | --- |
| Slot metadata | `General.sav` contains two zero `u32` values and `General000.sav` has level ID, display text and two decimal lines. [Format notes](../docs/openspace/save-games.md) identify the game writer. Structural decoding demonstrated for one supplied slot. | Meaning of the two integer fields and decimal lines; change one slot property and verify the displayed load menu and reload. |
| Player statistics | `Level000.sav` has 1,271 records. Current/maximum mana at `0:5:4986212`/`0:5:4986214` are supported by controlled live observations and game-written saves. Current mana has independent `druidik` reloads; `thereyougo` now supplies a fresh-process reload of 250/250. Health and armor remain unmapped. | Health/armor live owner reads and save/reload checks. |
| Inventory, equipment and magic | `thereyougo` selects `MagicIce` 54 and `MagicLightning` 74 in the inventory. Their handle selected-state bytes map to player records `0:10:1560` and `0:10:2800`; both persisted through a normal save and fresh reload. After drawing the sword, DEL selected Ice 54 and a cast spent 50 mana. Eleven changed references remain unnamed. A guarded crossbow quantity 0 → 3 reset to 0 after save/reload. | Map separate availability flags, unnamed references and other item quantities. The native Godot inventory test still exercises only synthetic state. |
| Progression | No source record is assigned to quest or progression state. | Select one progress event with typed live observation and normal save. |
| Actor and AI state | Raymap's Perso save-data view led to Hype's designer-variable registration. `iDummy_PourCercles.DSG[8]` is now address-mapped to player record `0:17:297732` and restored after a fresh launch. | Map other actor variables and determine their player/level ownership. |
| Level/world state | `manoir.sav` has 263 records; all are present with the same values in the supplied player file. The hidden manor lever changes the dummy unlock Boolean in `Level000.sav`, and fresh reload restores it. The named `manoir.sav` did not change in that case. | Test other level-owned interactions and level-file precedence after departure/return. Overlap in the starting fixture does not prove identical later values. |
| References | 228 pointer records in `Level000.sav`; 134 in `manoir.sav`. The source reader resolves block/module/relative identities against current allocation context. | Map referenced objects and invalid-reference behavior. A pointer delta is not an inventory mapping. |
| Arrays and unresolved bytes | `Level000.sav` has 222 arrays, and a no-action game save changed two of them plus one time-like eight-byte record. The reader has complete boundaries for the supplied profile. Unknown record kinds now retain an undecoded tail. | Identify array element types/owners, changed flag bits, and time-like field. Unknown record lengths remain unresolved. |

In the supplied slot, `Level000.sav` types are 311 one-byte, 131 two-byte,
379 four-byte, 222 arrays and 228 references. `manoir.sav` has 124 one-byte,
five arrays and 134 references. This is structural coverage, not 1,534 named
fields. All four files parse; five record identities have supported gameplay
names. The semantic profile is caller selected because the save files
do not identify their executable build.

## Source-path findings

The [stable format notes](../docs/openspace/save-games.md#registration-and-restoration-findings)
link the pinned decompilation functions. Registration captures table values;
the writer may use either that snapshot or live memory according to selection
bits. Loading decodes table entries, resolves addresses against current memory
blocks, then applies selected modes. The observed startup code loads the player
file and applies a mode, while named level files have their own read path.
No complete precedence or post-initialization proof is claimed yet.

With explicit `--after-slot SaveGame0`, revised diffs against the earlier
game-written captures show three control changes in `Level000.sav`: `0:5:4`,
`2:6:1463692`, and `2:6:1464196`. `druidik` adds current mana
`0:5:4986212` (30 → 50) with maximum mana unchanged at 50. `thereyougo` has
18 changed records including both mana values (30/50 → 250/250), two mapped
inventory selected-state flags and eleven unnamed references. `manoir.sav` has no record changes in
these cases. These are prior saves with live before/after observations, not
fresh-process reload results.

Two fresh no-interaction controls then used independently staged copies of the
pinned archive. Each loaded playable `manoir`, observed mana 30/50, Ice and
Lightning unselected, and the dummy unlock false, saved through the normal
lectern flow, and retained those typed values. Hype physically walked to the
lectern; the control deliberately performed no item, magic or world-state
interaction. Both player-file diffs changed
exactly `0:5:4`, `2:6:1463692`, and `2:6:1464196`; their bytes differ with
timing/pose and remain semantically unnamed. The named `manoir.sav` had no
record changes. The two-run control passed in 73.74 seconds; complete raw
diffs and save roots are under
`output/savegame-investigation/source-no-action-control-proof-1790557697785110045/`.

## Original-game mana proof

The source-only behavior-suite case ran twice from independently staged copies
of the immutable archive, with the pinned executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
Each run reached playable `manoir`, observed current/maximum mana 30/50,
entered `druidik` through the ordinary keyboard path, observed 50/50, and
completed the normal lectern overwrite and name confirmation for the first
slot. The game wrote `SaveGame0/Level000.sav` current-mana record
`0:5:4986212` as 50; the full save root was stable and copied. A second,
separately owned game process loaded that copied root and observed 50/50 in
`manoir`. Reports are in
`output/savegame-investigation/source-mana-proof-1790545679202317650/` and
`output/savegame-investigation/source-mana-proof-1790545781189854329/`.
The runs took 71.48 and 71.78 seconds. They use an observed title/menu/fade
readiness wait, not literal-time or frame-precise replay. The game also creates
an `HYPE/` working set; both it and `SaveGame0/` contain the changed mana
record in the captured output. Exact precedence between those copies remains
open. The original installed save and archive still pass pinned hash checks.

## Inventory candidate findings

The crossbow candidate used the verified typed active-inventory reader and a
guarded, current-process quantity intervention; it did not edit save bytes.
The observed Weapon 80 quantity changed from 0 to 3 before the normal lectern
save. The game-written `SaveGame0/Level000.sav` differed from the supplied
fixture in the same three record identities as the no-action control, while
`manoir.sav` did not change. After staging that full output as the next fresh
process's input, the typed crossbow quantity was 0 again. The run evidence is
under `output/savegame-investigation/source-crossbow-proof-1790546570600008173/`.
This demonstrates a reset for this intervention, not the absence of all
inventory persistence. An earlier attempt did not complete the overwrite
dialog; its unchanged files are retained as a failed menu attempt.
The maintained expected-reset scenario then passed in 69.11 seconds with
`output/savegame-investigation/source-crossbow-proof-1790547059688092981/`.
A later 64.49-second run used screenshot-checked menu choices and reached the
same 3 → 0 result. Its menu observations show the list, No, Yes, then editable
name in order; evidence is under
`output/savegame-investigation/source-crossbow-proof-1790554897201826875/`.

The narrower `pouletfrit` input binding was present in the verified game but
did not produce an active Object 44 entry in the current manor context. The
failed probe is under
`output/savegame-investigation/source-object44-proof-1790546748814576657/`.
It was not promoted to a save/reload case. The scripted `grolot` group binding
also produced no typed active-inventory change in this manor load; that failed
probe is under
`output/savegame-investigation/source-grolot-proof-1790546923887247717/`.
Neither
binding alone supports a persistence claim, and their script conditions or
inventory targets still need to be identified.

Moving `iFioleBleue_NearBrigand` to the player in one guarded current-process
probe changed the actor's action/behavior, but neither touch nor Space added
an active inventory entry. The process was closed without saving; its evidence
is under `source-blue-flask-proof-1790556521148600554/`. The candidate was
not promoted to a persistence case.

## Original-game magic inventory reload proof

The physical `thereyougo` input changed the typed active inventory from
`MagicFire` 61, Weapon 80 and Object 21 to include `MagicIce` 54 and
`MagicLightning` 74. It also changed current/maximum mana 30/50 → 250/250;
the case does not attribute all broad-cheat record changes to inventory.
Source inventory-handle traversal identifies selected-state bytes at Fix block
0, module 10, offsets 1560 and 2800 as Ice 54 and Lightning 74 respectively.
The game-written `SaveGame0/Level000.sav` records at those identities (kind
`0x01`, flags `0x0204`) changed 0 → 1. After a separately staged fresh process
loaded the new save root, both entries were active again, with mana 250/250.
The direct case passed in 70.36 seconds; evidence is under
`source-magic-inventory-proof-1790557078561593569/`.

The first post-reload equipment input `Tab, Down, Right, Down, Space, Tab`
left the combat spell at Fire 61. Thus inventory selected-state persistence is
demonstrated, while choosing and casting Ice or Lightning remain
pending. A second isolated menu probe followed `Tab, Up, Down, Down,
Right×4`; its screenshot visibly highlighted `Magia Glacia: Dragon`.
`Space, Tab` still left the typed spell selection at Fire 61 with mana
250/250. The probe did not attempt a cast. Its screenshots and typed captures
are under `source-magic-ui-probe-20260927-ice/`. The local Hype manual
(`disc/HypeManual.pdf`, printed pages 16 and 18) says Space activates a focused
inventory item, while a separate DEL action chooses among three activated
spells in combat. The installed `Current.cfg` matches the disc default, so
Space was the expected inventory activation input. The observation reads the
combat spell, not the inventory selection bits; unchanged Fire therefore does
not show whether Ice activated. `0041b500` dispatches the inventory-selection
action to `0041d010`, which calls the inventory selected-state API. A typed
selection-bit observation or the active spell icons, followed by a cast, is
still needed for usability. The eleven changed reference records are still
unexplained.

A later targeted headless run repeated the normal save and fresh reload with
the private display/audio route. Ice 54 and Lightning 74 again appeared as
selected inventory handles, with 250/250 mana. Three isolated physical DEL
presses each left the typed current combat spell at Fire 61, the actor at idle
action 47, and mana unchanged. The screenshot showed Hype's sword sheathed;
the manual says DEL selects a spell while a weapon is in hand. The test did not
send Enter or claim a cast. Its 73.124-second result, screenshots and per-tap
observations are under `output/savegame-investigation/targeted-magic-run-20260927/`.

The next bounded probe drew the sword with physical X. Its screenshot showed
the blade in Hype's hand and the combat spell icons. One DEL press changed the
typed combat slot from Fire 61/index 0 to Ice 54/index 1, with mana still
250/250. That first run stopped before casting because the test used the
unsupported key name `Enter`; it left complete pre-cast evidence under
`output/savegame-investigation/targeted-magic-drawn-20260927/`. The corrected
test used the physical `Return` key for the manual's Enter action and passed
in 83.41 seconds. It again loaded the game-written save in a fresh process,
selected Ice 54 after drawing the sword, then observed mana fall 250 → 200
and a visible blue ice effect after one cast. The exact source record changes,
typed states, physical inputs and screenshots are under
`output/savegame-investigation/targeted-magic-cast-20260927/`. This establishes
source inventory selection and combat usability after reload for Ice 54. The
separate item availability bytes and eleven changed references remain unmapped.

## Original-game manor world-state proof

The hidden lever came from the earlier
[live investigation](hype-cheat-exploration-2026-09-12.md). The current
Orrery case resolves both actors by source object-type identity, moves
`iLevierSecretManoir` into reach with one guarded position intervention,
uses `debug` and Space, and restores its placement before a normal save. The
game sets `iDummy_PourCercles.DSG[8]` from false to true. The converted
source identities are `level:/persos/world/0/174` for the lever and
`level:/persos/world/0/27` for the dummy; live child ordinals can differ.

The source-layout test maps the dummy Boolean to Fix block 0, module 17,
offset 297732, with source save flag `0x0200`. The game-written player record
`0:17:297732` (kind `0x10`, flags `0x0204`) changed `00` → `01`. The named
`manoir.sav` did not change. Another player record `1:17:187072` changed
`01` → `00`, but its owner remains unknown. The earlier direct case
`source-lever-proof-1790555647295407130` established fresh reload of the
Boolean but checked the teleport while loading was at phase 5.

The stronger run `source-lever-proof-1790556429235091471` passed in 68.82
seconds. A fresh process loaded the game-written root, observed the Boolean
true, accepted `tele11`, and reached `astrolabe` in gameplay phase 9 with a
typed Hype actor at action 47 and a finite pose. The pending-level text still
reads `astrolabe` in this phase, so it is not used as a completion condition.
The teleport establishes behavior enabled by the restored unlock; general
quest progression remains untested. The current Godot bridge gate is a fixed
demo lift and does not yet offer a matching saveable interaction.

The live lever can begin at two observed local positions while retaining its
source object types, parent chain and interaction action. The guard accepts
only those measured positions and restores the one it found. The activation
also sometimes leaves Hype outside the save lectern's effective position:
one failed attempt remained idle at global Y 27.23 through two Space presses,
whereas a direct save passed at Y 27.58. A measured physical Up approach moved
Hype from Y 27.29 to 28.53 before Space; the targeted lever save and fresh
reload then passed in 79.88 seconds. The source trigger radius and cause of
the lever's two opening positions remain unproved. Evidence is under
`output/savegame-investigation/targeted-lever-approach-20260928/`.

## Native Godot persistence scope

The game now has a versioned JSON slot in `user://manoir-save-v1.json`. It
stores one stable player identity (`world/0/0/Hype`), native current/maximum
mana and selected spell, selected weapon, item quantities, and the exported
manor dummy Brain's `dsg_8` Boolean under stable source identity
`level:/persos/world/0/27`. The measured
English fixture remains the initial manor seed for other supported starting
state. The slot is read and validated before the level is opened; supported
mutable fields are restored after seed application and before Brain starts.
Invalid, missing and unsupported values cannot partly apply. Writes validate
the complete snapshot and replace the slot via a same-directory temporary
file. The menu surfaces a corrupt slot and disables loading it.

The `godot-harness` save test drives the actual menu and F5 save operation in
an isolated Godot user-data directory. It writes 23 mana after native
consumption from 30 and sets the real exported dummy Brain flag true. A fresh
Godot process loads 23 mana, inventory quantity 2 and that flag, then
consumes another five mana from the restored component.
It also checks that an invalid in-memory write keeps the prior slot and that a
corrupt or unsupported-version slot is rejected before scene launch. The
test's inventory quantity and world-flag changes are synthetic infrastructure
inputs applied to real native/exported objects; neither demonstrates the
matching converted manor pickup or lever interaction. The game slot does not
yet persist actor position, other actors, wider world changes, progression or
AI state. The direct harness rerun passed in 22.41 seconds; passing logs and
slot are under `output/savegame-investigation/godot-world-proof-20260928T010719Z/`.
The native `source_inventory` Resource currently exposes item quantities and
selected weapon, while `source_magick` exposes the current combat spell. It
does not yet represent the source inventory handle's selected state for Ice 54
or Lightning 74. A source-to-Godot magic-loadout comparison needs that native
field and the converted selection/casting behavior before these two source
records can be treated as supported Godot gameplay state.
The pending gameplay case is the converted manor lever at
`level:/persos/world/0/174` setting the converted dummy at
`level:/persos/world/0/27` `dsg_8` through native interaction, then restoring
that unlock and exercising its destination behavior after restart. The save
contract can retain the Boolean, but the lever action and destination behavior
are not yet supported in the converted game. The direct Brain mutation above
cannot close that case.
An earlier direct headless Godot run aborted at renderer shutdown. The
three-process save test now runs in an authenticated private Xvfb display,
retains stage logs and rejects unexpected engine or script errors even when a
success marker was printed. Its five classifier checks and the direct
persistence test passed with that display in 50.47 seconds. The earlier
headless abort is retained under
`output/savegame-investigation/godot-exit-crash-xvfb/`; the passing classifier
evidence is under
`output/savegame-investigation/save-classifier-review-final/godot-save-persistence/`.

## Exact reproduction

```sh
mkdir -p output/savegame-investigation
cargo run -p orrery --locked --offline -- save-inspect reference/Savefile_EN.zip \
  --profile hype-pc-english-supplied-save \
  --output output/savegame-investigation/baseline.json
cargo run -p orrery --locked --offline -- save-diff \
  reference/Savefile_EN.zip output/cheat-save-research/druidik \
  --after-slot SaveGame0 --profile hype-pc-english-supplied-save \
  --output output/savegame-investigation/druidik-prior-diff.json
cargo test --locked --offline -p orrery save::tests -- --include-ignored
cargo test --locked --offline -p godot-harness --test save_persistence -- --ignored
cargo test --locked --offline -p behavior-suite --test save_original -- --ignored --test-threads=1
ASTROLABE_HYPE_CONFIG=/home/stephen/Projects/hype-harness/local/config.json \
  python3 tools/test.py --area save --local --offline --timeout 600
```

The Godot test requires the ignored generated manor, Godot 4.7.2 in this run,
and the source game data used to produce it. The three process stages took
50.47 seconds in the latest direct check after Rust compilation. A failed stage
retains its log when `ASTROLABE_EVIDENCE_DIR` is set. Source-game runs require
the prepared private Hype installation and Gamescope's headless backend with
its private Xwayland display. Rendered Godot checks use authenticated Xvfb;
the save test also uses that private display. The native Godot harness has
separate generated-content and Godot runtime prerequisites. Use the focused
runner for the selected source and Godot cases with their mandatory safeguards.

The complete focused save selection passed on 2026-09-28: 42/42 selected tests,
95 other binary tests skipped, 483.361 seconds including build and cleanup,
with an explicit 600-second job budget. The report confirms an unchanged source
snapshot and complete temporary cleanup:
`TestResults/20260928T033518.226177Z-development.log.json`. The first full
attempt exposed a narrow source lever save position; after a physical approach
fix, its 480-second rerun reached 38 passes before the suite budget stopped
the Godot case. The final run retained every selected test and mandatory
safeguard, including the Godot test and the audio cleanup checks.

## Source-save import decision

A narrow importer for the two mana fields, two magic selected-state flags and
the manor dummy unlock is feasible for this pinned English fixture if the
source profile/build, active player identity, current converted inventory
handles and the dummy's stable actor identity are supplied. It is **deferred**:
the eleven shifted magic references, save-file precedence beyond these cases,
other inventory quantities, wider world state and equivalent gameplay remain
unknown. Even these verified fields cannot establish general original-save
compatibility. The measured English manor seed is explicitly a seed, not a
source-save converter. An original-save writer or general importer is outside
this checkpoint.
