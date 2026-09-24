# Hype cheat and save experiment — 2026-09-24

This is a paused checkpoint, not a completed survey of the 272 catalogued
bindings. The [complete code catalog](../docs/openspace/cheat-codes.md) and
[earlier lever investigation](hype-cheat-exploration-2026-09-12.md) provide the
source-informed expectations. The [save format notes](../docs/openspace/save-games.md)
describe the byte and record layout used for comparisons.

Luna runs the pinned English PC game through Orrery. The committed
`reference/Savefile_EN.zip` is the source fixture and must remain unchanged.
Each completed game's slot files are copied to a separate ignored directory
under `output/cheat-save-research/<case>/`. The same in-game slot is reused;
the installed fixture is restored before each case. `target/debug/orrery`
`save-diff` produces each case's decoded record comparison. These local game
files and raw runtime observations are not committed.

## Repeated game path

1. Start an Orrery owned session with enough lifetime for loading, cheat input,
   save-menu interaction and cleanup. Load the supplied save with Down,
   Return, Return (55 ms holds, 800 ms gaps); wait for phase 9 `manoir` and
   fade-in. Read state and mana before entering the code.
2. Enter the validated cheat through ordinary OS keyboard input. Record the
   live binding, state, mana and any relevant Global, DSG, inventory, health,
   armor, projectile or actor fields both before and after.
3. Reach the nearby save lectern with Up for 1,500 ms, then lowercase `space`
   for 150 ms. The current XTest parser rejects uppercase `Space`.
4. In phase 38, move Down twice to the reused test slot, press Return to open
   the overwrite confirmation, Right to select YES, then Return in a separate
   call. Verify changed save files before copying them out. Do not count a
   menu selection as a completed save.
5. Stop the owned game. Preserve the copied case directory, Rust diff, member
   hashes and run provenance. Restore the installed fixture before the next
   case. A 60-second session expired during the first `leben` attempt before
   confirmation; its slot still contained the preceding `druidik` save.

## Completed captures

| Case | Live result | Saved result | Local evidence |
| --- | --- | --- | --- |
| No-cheat control | Ordinary save from the loaded manor fixture | `Level000.sav` has three changes from the fixture: one time-like eight-byte value and two larger arrays. `manoir.sav` is unchanged. | `output/cheat-save-research/control/` |
| `thereyougo` | Recognized `Button_Cheat_ToutesMagies`; mana 30/50 → 250/250; level remained `manoir` | Current and maximum mana records `0:5:4986212` and `0:5:4986214` changed 30→250 and 50→250. Compared with the control, two one-byte records changed 0→1 and eleven pointer records shifted by 504 bytes. The time-like field and two arrays also differed; their gameplay meaning is not established. `manoir.sav` is unchanged. | `output/cheat-save-research/thereyougo/` |
| `druidik` | Recognized `Button_Cheat_FullMagic`; mana 30/50 → 50/50 | Current-mana record `0:5:4986212` changed 30→50; maximum mana stayed 50. The time-like field and two arrays changed as in the control. `manoir.sav` is unchanged. | `output/cheat-save-research/druidik/` |
| `leben` | Recognized `Button_Cheat_FullLife`; live mana stayed 30/50. The available typed read did not expose health. | Only the time-like field and two arrays differed from the source fixture, as in the control; no additional saved record change was observed. This does not establish whether health was already full or whether the effect persisted. `manoir.sav` is unchanged. | `output/cheat-save-research/leben/` |
| `protek` | Recognized `Button_Cheat_FullArmor`; live mana stayed 30/50. The available typed read did not expose armor. | Only the time-like field and two arrays differed from the source fixture, as in the control; no additional saved record change was observed. This does not establish whether armor was already full or whether the effect persisted. `manoir.sav` is unchanged. | `output/cheat-save-research/protek/` |

The two mana record identities are now matched to live values by controlled
input and independent in-game saves. The `thereyougo` flag and pointer changes
remain unnamed; inventory or magic linkage is a hypothesis until a live typed
observation identifies their owners. The control shows that time and two array
differences can arise without a cheat, so they must be analyzed separately.

## Checkpoint status

Five cases have game-written saves: the no-cheat control and four cheat codes.
No teleport save was captured in this run. The wider survey is paused because
collecting live state and saves individually is currently too costly. The
case files remain in ignored `output/cheat-save-research/`; the source archive
and prepared installed slot were restored to their original hashes. The source
archive's SHA-256 is
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The per-case observations cover binding, level and mana, but do not include a
complete typed Global, health or armor snapshot or a per-case executable hash.

After the captures, Orrery reported `controller.running=false`, and no game,
Wine, or gamescope process was found, but a subsequent start reported that the
prepared installation was already in use. That lease condition was not
resolved at this checkpoint. A resumed experiment should check ownership and
installation state before starting another game session.

## Remaining scope

The survey still needs an isolated run, live before/after observation and
game-written save for every catalogued code, including all 36 teleport loaders.
The hidden lever gates the teleport codes and two projectile controls; its
activation must be recorded without silently writing the gate flag. Per-level
save files may differ from the manor file. Save-record changes must be mapped
back to typed Global and other runtime state through source identities and
controlled comparisons before they become BDD or AI support assertions.
