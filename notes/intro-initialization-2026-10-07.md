# Intro initialization checkpoint — 2026-10-07

The integrated source checkpoint stopped Hype model 28 at source node 987,
category 4 operation 429. The installed Hype executable's procedure catalog
names 429 `Fight_ChangeArmor`; its handler calls `0043a9b0`, which resets the
shared selected-definition pointer to the first armor record and follows the
linked list by the byte index. The original Hype node supplies integer 0;
the alternative branch supplies index 2. Source call sites `0043aa20` and
`0043aa70` apply the selected definition's resistance, while `0043ab40` and
`0043ac60` draw its armor meter. The loader at `0043acf0` reads `armor.ini`'s
`Armor` section. Existing imported content has no promoted armor definitions.

The AI codec now retains procedure 429 as a typed armor-selection statement,
and the compiler calls a native `SourceArmorState` child on the character. That
component stores the selected byte index and emits `selection_changed`; the
generated manor actor can therefore complete this initialization call without
pretending that armor definitions, resistance or artwork have been converted.
The selector is scoped to the character as a native extension point for the
single manor player. General shared selection semantics await promoted armor
data and broader gameplay consumers.

The earlier colored-Hype/statue overlap is not corrected by hiding Hype: model
28 explicitly requests display-visible during its initialization, and current
New Game preview holds the authored `DemoCamera` while camera model 437's first
source rule reaches procedure 135 (position update). The current native camera
implementation rejects mode 0; its camera movement/publication semantics are
still unverified. The V15 report also records the separate `iHypeStatue_Manoir_MAD`
actor (model 294) stopping at category 9 operation 0. Those are separate
source/runtime blockers. No original-engine run was made for this increment.

Evidence reviewed: `TestResults/hype-armor429-handler-static-proof-sol-20261004.json`,
`TestResults/actual-opening-v15-sol-20261004/new-game-opening/report.json`,
`reference/decompilation/functions/0043a9b0_FUN_0043a9b0.c`, and
`reference/decompilation/all_functions.c` around `0043aa20`–`0043acf0`.
