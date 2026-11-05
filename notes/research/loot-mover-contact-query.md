# Loot mover vs other characters' contact query (static, MaiDFXvr_bleu.exe)

Scope: static disassembly only (objdump -d -M intel; VA = file - 0x400 + 0x401000). No code run, no repo edits.

## 1. Type-4 table entries (filled by 00443540, base 0x00757320, 8 bytes/type)
- Type 4 = table offset 0x20/0x24 (PE 44357f..443595): +0 = 00585d51, +4 = 0058cc2d.
- 00585d51 is the SAME frame callback as type 0 (type 0: +0 00585d51, +4 00589bfa). Only the +4 handlers differ.
- 00443f00 calls table[type].+0 per frame (call at 443fd4, `call [ebx*8+0x757320]`, 7 args:
  dynamics, SO, parent, environment, workspace, identity card, float seconds).
- 00443e00 (event dispatch) calls +4. 0058cc2d is a small event switch (jump on event-3, 19 cases, only
  vector copies + 0058ff6a/004444d0 calls); it contains NO collision query. So per-frame movement = 00585d51.
- The "walk" kinds 0 and 4 therefore share one movement/contact routine; a type-4 loot object uses the
  character walk integrator, not a lighter prop path. (The type value 4 for loot is taken from the caller's brief;
  I did not re-derive 0043bcd0.)

## 2. Path from 00585d51 to the character contact query
- 00585d51 init: byte [ebp-0x20]=1 (585dc9) = "do collision". 005a070d integrates velocity (5866cb).
- Skip logic (586642..5866b5): only when the frame vector xyz==0 does it read 00430ea0(SO)
  (byte at +0xc of the CollSet ZDM-descriptor, 00430ea4..430ec3). bits: (b0 && b2 && b5) or (b0 && b1) clear
  [-0x20] (no query); b0 clear or neither clears the carried flag [-0x158] only. A fully motionless frame
  also exits at 58648d / 58663a before any query. A moving/falling object always reaches 5866d3.
- 5866d3: if [-0x20] != 0 -> 0059ae97 (586733, when [-0x3c]==1) else 0059a504 (586775). Result 4 then
  0059b83c (5867bb). Both 59a504 (loop up to 7 iterations, call 59a942) and 59ae97 (59b2e7) call
  00463df0(out=dynamics+0xfc, mover SO, local vec, local box). 59b83c calls it with fixed globals (59bc0f).
- No "kind" flags are passed by the mover path. 463df0 forwards (447f90([perso+0x28]) , mover SO, vec, box,
  -2) to 00464570 (463e6d). The kinds 1, 3 and mask -2 are literals inside 00464570 (see 3).
- 00463df0 early outs: [7543bc] set AND 0043dbb0(mover) -> return 0 (463e03..463e28); mover Perso+0x18
  CollSet == 0 -> return 0 (463e3b/463e40).

## 3. Exact tests
- 00463dd0(x) = `word [x] > 0` (46fe60 returns first word) = "list non-empty". It tests a LIST HANDLE,
  not a flag byte.
- Kinds (CollSet category): 0 = ZDD (+0), 2 = ZDE (+4), 1 = ZDM (+8), 3 = ZDR (+0xc) in the Perso CollSet
  (45f760 table; 00430930 called with slot 1 by Cond_CollideMovingZDM; ZDE uses category 2). PO CollSet (4b7510)
  has a different order but kind 3 is again +0xc.
- 004305xx list builders 00430130/00430220/00430310 (kind, SO, mask): walk the SO's descriptor list for that kind
  (42dad0/42e870/42e580/42e5f0); with mask -2 keep zones for which 42d8b0(zone, 8)==0 (no material matching 8);
  zones with a 0x10 material go to global 754404 instead. Output handles: 430130 -> 7543b8, 430220 -> 78ee60,
  430310 fills 78ee04 BUT RETURNS 7543b8 (43020c, 4303ec: `mov eax,ds:0x7543b8`).
  Consequence: in 00464570 both "list1" (ZDM of mover, 464586) and "list3" (464599) are the same ZDM handle 7543b8;
  the mover's ZDR list (78ee04) is built and never consumed.
- 00464570 (464570..4647ce):
  - 464590/4645a9 list1/list3 as above; sector gate `4dda60([ebp+4]) & 2` (4645b9) else skip everything.
  - needs 463dd0(list1) || 463dd0(list3) (4645c2/4645d3): mover must have a non-empty ZDM list.
  - **Mover's own CollSet +0x31 gate**: 4645e3..4645f4 `45f930(mover [perso+0x18])`, `cmp al,1 ; jne 464753`.
    Not == 1 -> the whole character loop is skipped; only the static-geometry path (464753 -> 00464150) runs.
  - Loop over chars in own sector (4e0830/4e0840) then each neighbour sector (4e0850/4e0860, also gated by its
    flag bit 2): skip when 4e0690(node)==mover (464612); additionally when 0043dbb0(mover) is true skip the node
    whose SO equals 00440fc0([perso+0x14]) (464616..46463f). Otherwise 00464380.
  - Mover's priority (+0x30) is NOT read here.
- Exclusion meaning: 0043dbb0(SO) = SO type word == 4 and [perso+0x14] != 0; 00440fc0(x)=[x+0x10]; same getter
  the camera code names camera_get_target. So [perso+0x14] is the camera object and +0x10 its TARGET SO.
  This is "the camera-bearing mover does not collide with its camera target"; it is NOT a generator/owner link.
  A loot/thief with [perso+0x14]==0 has no exclusion; the generator is not excluded anywhere.
- 00464380 (mover, list1, list3, other SO, vec, box, -2):
  1. 438f40([other perso+4]) must be 1: `0x1b`byte == 0 and byte `+0x1c` (438f20/438f40). Else return.
  2. other CollSet (Perso+0x18) non-null -> 45f930 (+0x31) must be non-zero (4643aa..4643b4); null CollSet passes.
  3. bounding test 55dde0 when both SO+0x34 present (4643dc).
  4. If other has CollSet and 463dd0(list3): 00464510 -> mover ZDM-list vs OTHER's ZDR list
     (430220(3,other,-2), non-empty), via 55dd50(list3, vec, box, otherZDR, other+0x24, 0x1ffffff, other, 1).
     **No priority check on this stage** (464429..464444).
  5. Priority gate `cmp cl,al ; jb` (464450..464456): mover CollSet +0x30 (45f910) >= other's (0 if no CollSet).
  6. If gate passes and 463dd0(list1): for each Data3D module node (442b10([other perso]); module=[node+0x10])
     with kind word 0xb and a PO CollSet: 004644b0 -> 55dd50(list1, vec, box, 4b7510(3, PO CollSet)=PO ZDR geometry,
     module+0x24, 0x1ffffff, module, 1). Again mover ZDM vs other ZDR (PO). Never ZDD/ZDE/ZDM of the other.
- A hit from 55dd50 yields 463df0 result kind 2 (463eea) for a character target -> 59a504/59ae97 call
  00599703 (object-contact response; not read).

## 4. Conclusions for the stated values
- Loot template priority 0 / flag 0; thief priority 0 / flag 1; Hype priority 127 / flag 1.
- Before release (loot +0x31 = 0): loot as MOVER skips every character (4645f2 jne) -> collides with static
  geometry only. As a TARGET it is also invisible to other movers (4643b4: +0x31 == 0 -> return).
- After release (+0x31 = 1): loot tests every active character in its sector/neighbours with non-null CollSet
  and +0x31 != 0, provided its own ZDM list (mask -2 filtered) is non-empty and the sector bit is set.
- Versus the dying thief (active, +0x31=1, priority 0): stage 4 runs (no priority); stage 5: mover 0 >= other 0 ->
  passes (equality is not `jb`), so stage 6 also runs. So the original DOES test loot vs the thief, against the
  thief's ZDR (CollSet kind 3 list and the ZDR of its Data3D PO modules) using the loot's ZDM zones. Not ZDM/ZDD/ZDE
  of the thief. Hype (127 > 0) would be tested only by stage 4 (CollSet ZDR), not stage 6.
- Rest-on-corpse: contact stays possible while 438f40(thief perso+4) is 1 (byte 0x1b clear, byte 0x1c set) and
  thief CollSet +0x31 != 0. Deactivating the corpse via +0x1c/+0x1b, or clearing its +0x31, drops it from the test.
- Generator/owner is NOT excluded (only camera target when the mover has a camera).
- Loot's own +0x31 matters: must be exactly 1 (cmp al,1) for any character test.

## 5. Rejected / unresolved
- Rejected: per-kind flags passed to 463df0 (none; kinds fixed inside 464570); +0x31 only on the other side
  (mover side gated at 4645f2); (perso+0x14)+0x10 as owner (it is the camera target); priority check on the
  first stage (absent); 58cc2d (+4 entry) doing collision (it does not).
- Unresolved statically: (a) data: whether loot has a non-empty mask-filtered ZDM list and the thief a ZDR with
  materials not flagged 8 (empty list -> no test at all); (b) 55dbf0 internal intersection and the hit list
  semantics; (c) 00599703 response (whether a contact stops/rests the loot); (d) sector flag bit2 at sector+0x98
  meaning; (e) the 430310 return of 7543b8 is as disassembled (looks like a copy slip in the original).
Confidence: high for call chain, arguments and gates; medium for ZDM/ZDR naming of kinds 1/3 (layout +
AI condition naming agree); low for runtime outcome (data dependent).
