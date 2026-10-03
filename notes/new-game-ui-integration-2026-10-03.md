# Main New Game and maintained UI integration, 2026-10-03

Normal authored New Game is the actual game entry and canonical intro acceptance
start. Stephen observed Gogoud complete his first path in that game, then stall.
V4's early assertion stopped automated observation before the walk; it did not
establish that the actor failed to walk. Full acceptance remains the unskipped
[original sequence and gameplay return](./manor-intro-reference-2026-10-03.md),
about 210 seconds after manor entry, with no gameplay input during the cutscene.

## Integrated source and recovery

Only the four validated UI commits were cherry-picked from the isolated worker:

| Worker commit | Main commit | Change |
| --- | --- | --- |
| `9ffd70805ee6e50543a43ad1fd3dcf992f9bee07` | `fae35ea` | Maintained UI, typed dialogue bridge and life state |
| `3c92664e8d16a8b72bcd91fdbe33a15743cdce8c` | `b4cf144` | Callback completion and native fixtures |
| `92e43944e65380ad5c9bccc7ffb739b4629e6cd1` | `eeb5ec0` | Float serialization and complete UI fixture |
| `f2eb446c7fec15d2c1e3b09865eb28299131970b` | `72725d0` | All eight opening dialogues plus the ordinary item |

Local recovery branch `codex/pre-ui-integration-20261003-sol` retains the clean
pre-integration `9c58482` state. The test-manifest conflict was resolved as a
union of all 277 unique cases. Automatic merges preserve the route UID,
New Game starter and complete camera cases; the existing camera fixture now
supplies the new optional life field. No unfinished font/frame work was copied.
Support interface 45 and Rete schema 38 require one fresh manor conversion.
The root support package is maintained source: these reviewed Git changes are
its explicit update. `update-godot godot` is deliberately prohibited by the
source-checkout ownership guard and must not be used to install over it.
Ordinary content regeneration preserves authored settings and game files.

The live interface-44 output still matches every one of its 4,679 owned receipt
hashes. No edited file needs discarding for this conversion. The complete prior
interface-39 output remains recoverable in the existing archive. Original game
assets and user saves remain untouched.

## V4 regressions and corrected observation

All 21 mandatory safeguards and all 29 Python checks passed in v4; both actual
New Game and native save persistence failed. The Global data Brain is explicitly
nonautomatic in the exporter (`src/godot/scene.rs`), unlike rooted actor Brains.
The observer now records that exact role/model separately, asserts it remains
the one inactive Global model 1, and still requires every actor Brain automatic.
It preserves all stopped behavior diagnostics rather than letting one unrelated
actor prevent observation of Gogoud.

The measured English inventory has `61: -1`. Native quantity lookup also uses
-1 as its unavailable sentinel. Save capture retained it, but validation rejected
all negative quantities. Validation now accepts exactly that sentinel and still
rejects quantities below it. The existing process test checks actual F5 write,
fresh-process restoration of the sentinel, invalid-state slot preservation and
rejection of -2; no fixture quantity is zeroed to manufacture a pass.

Failed harness exits now use the existing scene-drain teardown rather than
quitting with the whole manor still attached. This targets v4's 900/907 ObjectDB
warnings; native retesting must distinguish object teardown from supervisor
process cleanup. Neither fix is a native regression pass yet.

The normal game reports an incomplete opening without calling it a preview.
The observer verifies all nine actual generated dialogue items, original Gogoud
route/actions and source-associated native contact, then observes two seconds
past the first path and records UI position and the actual actor's failure/state.
It retains the complete generated World/Gogoud code even on failure. Start/end
PNGs and private slot/restart/menu evidence remain required. A walk checkpoint
pass does not accept the full intro.

The old generated Gogoud code stops first at PositionDialog/node15 after its
1,200-ms post-path wait. Integrated source supports that UI call. The next
preserved unsupported statement is meta-action23/node18,
`Action_IncrementalTurnPerso`, before action20, action31 and RunDialog/node27.
This is the next source-backed blocker hypothesis; the normal-path observation
must establish which operation is actually reached. World unbound input, source
camera position updates and Hype particle initialization remain separate earlier
stops; UI integration does not claim to resolve them.

## Checks and next bounded experiment

A first formatting check stopped cheaply (0.528 seconds) on one long Rust print
statement; no later stage ran. After formatting that statement, the source-only
preflight passed in 19.244 seconds: `cargo fmt --all --check`, scoped strict
Clippy for Astrolabe/Godot-harness and all six selection checks. Source remained
unchanged and all stages cleaned up without timeout or remaining children.
Evidence: `TestResults/new-game-ui-integration-source-preflight-r2-sol-20261003.json`.
Sol ran no Godot or original-game session.

The next fixed-Luna assignment puts owned syntax and receipt checks before the
release build and required one-time regeneration. It runs one combined
integration selection with mandatory safeguards, both affected game/save cases
and the six UI cases, checking the complete discovery/manifest inventory.
Native execution, screenshots, regression passes and post-path behavior remain
pending. The concrete question is what the actual original Gogoud AI does after
its first path with the maintained UI now integrated. No source initialization,
World trigger or actor behavior is substituted by an isolated fixture.
