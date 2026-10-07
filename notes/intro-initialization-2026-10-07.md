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

The AI codec retains procedure 429 as a typed armor-selection statement. The
compiled call finds a native Resource on the nearest generated level root; all
actors in that level therefore observe the same selected byte index, while
separate level instances get local state. The Resource emits
`selection_changed`/`changed`. This completes the observed valid index-0
operation without pretending that armor definitions, resistance or artwork
have been converted. It stores the byte ordinal; resolving it to the actual
definition pointer, including null for an out-of-range ordinal, remains
dependent on the Fix catalog.

The reviewed fork at `~/mnt/misc` commit
`294bd260fcfa6d4c53b99799e60bce14614f2984` corrects the catalog root to
`GPT[18128 + 72 + 448 + (96 + 24*n) + 8 + 16 + 136 + 140]`, where signed
`n = Fix.sda[408..410]`. For the supplied source bytes, `n=4`, root slot is
19140 and count at `Fix.sda[3660]` is 3. The earlier slot 19164 lands in a
diamond-gauge TextureInfo, not an armor root. I preserved this evidence for the
next Fix-owned data slice but did not add a standalone reader: integrating it
requires a canonical `ArmorCatalog` content type, allocation-bounded Fix
GPT/SDA/RTP traversal, Rete asset support, Godot Resource export and level
binding. Those dependencies are outside the node-987 initializer; the fork
itself records no build, test or runtime validation for its broader changes.
The fork's static evidence is `notes/armor-source-2026-10-04.md` and
`output/static-port/armor/engine-disassembly.txt`; it identifies the patched
executable by SHA-256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.

The earlier colored-Hype/statue overlap is not corrected by hiding Hype: model
28 explicitly requests display-visible during its initialization, and current
New Game preview holds the authored `DemoCamera` while camera model 437's first
source rule reaches procedure 135 (position update). The current native camera
implementation rejects mode 0; its camera movement/publication semantics are
still unverified. The V15 report also records the separate `iHypeStatue_Manoir_MAD`
actor (model 294) stopping at category 9 operation 0. Those are separate
source/runtime blockers. No original-engine run was made for this increment.

Raymap's pinned Hype type table identifies node kind 9 as `Field`, and field
0 as `Position` (`reference/raymap/.../AITypes_Hype.cs`); its translator emits
field nodes with their child operands (`TranslatedScript.cs`). The Rust AI
expression parser currently has no field-node arm. That names the statue stop
more precisely, but does not establish the original receiver, coordinate frame
or whether the value feeds the statue's pose change. Those semantics still need
to be traced through the executable's field-function dispatch before a native
API is added.

Evidence reviewed: `TestResults/hype-armor429-handler-static-proof-sol-20261004.json`,
`TestResults/actual-opening-v15-sol-20261004/new-game-opening/report.json`,
`reference/decompilation/functions/0043a9b0_FUN_0043a9b0.c`, and
`reference/decompilation/all_functions.c` around `0043aa20`–`0043acf0`.
