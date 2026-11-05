# Original Hype: what starts a character's AI (static read, 2026-11-04)

Method: objdump of `MaiDFXvr_bleu.exe` plus `reference/ghidra/functions` in hype-decomp. Nothing was run. The
five-actor table and `brigand-arrival-started-live.json` (136 rows with a brain) are the live evidence.
Sector membership and activity lists come from `godot/generated/hype/levels/brigand/level.tscn` (world/1/N).

## 1. The rule (high confidence on the code, medium on the data input)

There is no level-load "start" pass. The engine turn `0x44a1e0` (already in hype-decomp
`src/gameplay/frame_update.c`) re-derives the run set every frame, when not paused (`406c00` false):

1. Always-active list (`0x44a241..0x44a266`): head `FUN_00463c90` = `[0x756e7c]`; node = {+0 SO, +4 next, +8 prev,
   +12 header}. For each node, `perso = SO+4`, `sg = perso+4`, `FUN_00438fa0(sg, 1)` (StandardGame `+0x1d = 1`).
2. Sector wake (`0x44a2c7`, `call [0x5e9ee4]`): arg = current sector of the main actor
   (`443470` -> `443460` -> SO -> perso `+0x28` -> `447f90`, `[node+4]`). Target is `449cd0`. It walks that sector's
   character list and then, one level only (`449c60`), the character list of each sector in that sector's activity
   list. Per character (`0x449cf8`): `438f40(sg) == 1` -> `4430b0` (Data3D flags `+0x1a0 |= 0x200000`) and
   `438fa0(sg, 1)`; otherwise `443080` (clears that bit). `438f40` returns 0 if `sg+0x1b != 0`, else `sg+0x1c`.
3. Traversal (`0x44a316`, `call [0x5e9ee0]` = `449d70(root, 0)`; also `449f90` first pass and `43dd80` second pass).
   `449d70` runs `4499b0(SO, 1)` for a type-4 SO only if all hold (`0x449d70` body):
   - `43dbb0` false (perso `+0x14 == 0`), `sg != 0`, `439010` false (`sg+0x2a == 0`); otherwise the node is left to
     `449f90` (special `sg+0x2a`, same tests) or `43dd80`/`44a0d0` (perso `+0x14`, list `0x756e58` + 2 slots);
   - `438f40(sg) == 1` (`+0x1b == 0 && +0x1c == 1`);
   - `438f90(sg)` (`+0x1d`) `== 1` when the parent was not processed (`param_2 == 0`);
   - `438ea0(sg)` (`sg+0x10`, last-frame stamp) `!= DAT_00754aa8` (frame counter).
   A child of a processed perso (`param_2 != 0`) skips the `+0x1d` test. Not-active or not-flagged nodes get
   `434e60` (release), and an active, unflagged, dormant-state node can get `437940` (`43eb30` && Data3D `+0x14a`).
4. `4499b0` stamps `sg+0x10`, runs animation, and only inside `if (Data3D+4 != 0)` (`4424f0`) runs `4442e0` ->
   `444090` (father first) -> `5158e0` then `5158b0`. At the end, `param_2 != 0 && 43eb30(perso) == 0` ->
   `438fa0(sg, 0)`, so `+0x1d` is a consumed "run this frame" token. `43eb30` = `*(sg+8) > 0xffff`.
5. `mind+0x10`: `5158b0` writes 1 then runs `mind+4`; `5158e0` writes 0 then runs `mind+8`. `444090` runs `5158e0`
   (reflex, byte 0) and, only if it returns 0, `5158b0` (intelligence, byte 1). I found no reader of the byte. It is
   "the last executed slot was the intelligence one", initial 0. A reflex that returns nonzero every turn would also
   read 0. Treat 0 as "never ran" only with the extra evidence that 181's dsg did not change.

So: a perso's AI runs this frame iff it is active (`+0x1b == 0`, `+0x1c == 1`), flagged (`+0x1d`) by (a) the
always-active list or (b) belonging to the main actor's sector or to a sector in that sector's activity list, and
it has a current state (Data3D `+4`) and is not diverted to the `+0x14`/`+0x2a` paths. Script `ChangeComport`,
`Proc_ActivateObject` (`0x53fcf8` -> `443080`) only touch the `0x200000` bit or the behavior, not `+0x1d`.

## 2. Level load and the addresses in the earlier pointers

- Creation `0x438a60` (mode 0/1..6 switch, `438a77/438a99/438aeb/438af9`) sets `+0x1b = 0` and `+0x1c = 1` for mode 0
  and via `437740` (`actor_activate`); modes 1..6 clear `+0x1c` via `438f60(sg, 0)` (`0x438be1`). Astrolabe already
  decodes `+0x1b` (`never_back_latch`) and `+0x1c` (`active`) in `StandardSaveWire` (records.rs:879).
- `0x5e9f08` (0xc00) and `0x5e9f0c` (0x32) are NOT level-load counters. They belong to `44edc0`, a throttling wrapper of
  `449cd0` that `44ecc0` installs into the cell `0x5e9ee4` (`0x44ecca`), together with `44ed30` for `0x5e9ee0` (65535
  and 512 calls, then periodic skips) and `44ee40` for `0x5e9ee8`. `44ecc0` is selected at `0x405c39`, `0x405e7c`,
  `0x414d8d` only when the CD-drive scan (`44ef20` GetDriveType == 5, `44ef40` volume label/serial, `44f040`) leaves
  `ebp == 0`; otherwise `44ecb0` (ret) is stored. In this exe the `jz` after those checks is nop-patched
  (`0x405b14`, `0x405b25`, `0x405bea`, `0x405bfb`), so any CD-ROM-type drive gives `ebp = 1`. It is a disc-check
  degrade (eventually `404ac0(2)` at `0x44ee23`), dormant in a valid run. It is not an AI-start mechanism.
- The note's "00449cd0 stored at 0x5e9fe4" is a mis-address. `0x5e9fe4` is string data ("tGameMaterial..."). The
  cells are `0x5e9edc` (init, 44ecb0/44ecc0), `0x5e9ee0` = 449d70, `0x5e9ee4` = 449cd0, `0x5e9ee8` = 44ee30
  (read at 449f44, 44a2c7, 44a316, 52107c). `449cd0` has one caller, `44a2c7`.
- `0x43f120` iterates the "NumberOfAlways" pool (`DAT_0060ee48/58`, source `GAM\Always.c`), a different thing from
  `GAM\AlwAct.c` (the always-active list). `FUN_00463ad0` (AlwAct.c:0x37) is the only list-add, called from
  `0x43d673` (loader keyword `CharacterIsAlwaysActive`, LoadLvl.c) and `0x45c85c` (main-actor setup, also sets
  `DAT_00756e88`). `437b10` (sleep) is called only from `448280` on a main-actor sector change.

## 3. Check against the live rows (sg = [+1b, +1c, +1d, +20, +24, +28, +29])

Woken set for Hype in sector 7 = sector 7 + activity {9, 37, 10} (one level; sector 37's own list is not followed).
Cross-tab of the 136 rows (started, in woken sectors, +1b, +1c, +1d):

| started | woken sector | +1c | +1d | rows |
| --- | --- | --- | --- | --- |
| 1 | yes | 1 | 0 | 14 |
| 0 | yes | 1 | 0 | 0 |
| 0 | yes | 0 | 1 | 9 (118, 152, 154, 155, 157, 159, 161, 163, 169) |
| 0 | yes | 1 | 1 | 3 (17, 24, 183, all cameras) |
| 1 | no | 1 | 0 | 12 |
| 1 | no | 0 | 1 | 1 (22 iHypeStatue) |
| 0 | no | 1 | 0 | 58 |
| 0 | no | 0 | 0 | 39 |

(+1b is 0 for every row.) Reading:
- Every active, unflagged-now (+1d = 0, consumed) perso in a woken sector started (14 of 14), none idle. The 9
  inactive (+1c = 0) ones cannot run (`438f40` = 0). This matches the sector wake exactly.
- The 12 started persos outside woken sectors are Hype 0, Global 1, StdCamer 2 (no sector), drawbridge 14 (sector 20),
  42, 69, 123 (sector 11), 134 and 178 (sector 18), 193, 196 (34), 197 (17). They must come from the always-active list
  (or a sub-hierarchy node). 22 (+1c = 0, +1d = 1) fits "in the always list every frame, flagged, but inactive".
- 123 vs 48: both sector 11, not woken. 123 started by list membership, bat 48 has no flag (+1d = 0, never run).
- 178 vs 181: both sector 18. 181 (+1c = 1, +1d = 0) is neither woken nor listed: idle, as observed. 178 is listed.
- 119/120 wolves (sector 9), 23 Zatila (10), 3 IWorld (37), 43/127/179/192/195/198 (sector 7): woken sectors.
- This also explains why the earlier sector-only gate failed: the second input (always list) is not in sector data.
- The 58 idle active persos outside woken sectors (boars sector 28, logs 14/15, bats 11, thieves 13/14, checkpoint 36,
  181 ...) all have +1d = 0: never flagged.

## 4. Inputs and what Astrolabe lacks

1. Per-perso `sg+0x1b/+0x1c` (decoded already) and sector membership/activity lists (decoded).
2. The always-active LinkedList header `{first,last,count}` at VA `0x756e7c/80/84` and its 16-byte nodes whose word 0 is a
   SuperObject. Not decoded by Astrolabe (no hit for it in src/). It sits right after the 3-word header at
   `0x756e70` that Astrolabe reads as `GPT+0x2ae0` (`MainActorWire`, `ai.rs:972`, `live.rs:523`). If the GPT is a linear
   image the header is at `GPT+0x2aec/0x2af0/0x2af4`. Only one anchor (0x756e70 = 0x2ae0) supports that, so verify
   it before use. Raymap R2Loader.cs:697 reads `alwaysActiveCharacters` right after `families` for Hype/Alex.
3. Remaining unknowns on the data side: `sg+0x2a`, `sg+0x08`, perso `+0x14`, Data3D `+4` (state) for the three
   anomalous cameras 17, 24, 183.

## 5. Rejected hypotheses

- Sector-only gate: refuted by 123/178 (started outside woken sectors); the always list is the missing term.
- `0x5e9f08/0c` start counters or `0x5e9fe4` callback table: wrapper/disc-check state and a wrong address (section 2).
- `StandardGame +0x1b..+0x29` or `mind+0x14` as the discriminator: started and idle rows share the same tuples
  (e.g. (0,1,0,4,4) for 178 and 181). +1d is a consumed token, not an input.
- Script start (`Proc_ActivateObject`, ChangeComport): changes only the `0x200000` Data3D bit or the behavior.
- Initial `+0x1d` in the file as a start flag: inactive rows keep `+0x1d = 1`, active run rows reset to 0; it follows
  the run, it does not cause it.

## 6. Confidence and open items

- High: predicate in 1, the call order of `44a1e0`, the `0x5e9ee*` cell identities, the wake body.
- Medium: that the 12 outside-sector started persos are exactly the always-active list. Not checked against the
  file; the cross-tab fits but the list contents were not read.
- Open: (a) cameras 17/24/183 (+1c = 1, +1d = 1, idle) - candidates: `43dbb0`/`439010` diversion to `449f90`/`43dd80`
  (neither resets differently), `43eb30` (`sg+8 > 0xffff` suppresses the `+0x1d` reset), no Data3D state, or a reflex
  that always returns nonzero; (b) 22 iHypeStatue started yet inactive (ran earlier, then slept, or the list flag with
  a prior activation); (c) `437b10` event 5 effect on `+0x1c` (`437590`, `437940`) and the writer of mid-game `+0x1c`;
  (d) the GPT location of the list; (e) the real stay-asleep path for `0x200000` bit in the renderer (`4df0e0` family).
- Not done: no source rewrite committed to hype-decomp (the bodies read here are not yet verified clean-C
  candidates, and `0044a1e0` is already rewritten).
