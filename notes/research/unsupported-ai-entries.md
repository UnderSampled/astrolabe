# Unsupported AI entries hit by Manoir and Brigand

Static research, 2026-11-03, read-only on hype-decomp and the repo.

Sources: decomp source where it exists. Elsewhere, `objdump -d -M intel` on
`hype-decomp/reference/source-profile/MaiDFXvr_bleu.exe` (sha256 ed105a42…,
the same exe as `reference/hype_patch/MaiDFXvr_bleu.exe`). "Inferred" marks
reasoning that isn't a direct read. Categories: 1 condition, 3 function,
4 procedure, 5 meta action. Control bits are explained in
`action-return-policies.md` in this directory.

## Where they occur (generated GDScript stand-in lines)

| Entry | Manoir | Brigand |
|---|---|---|
| 4-154 Procedure_Camera_Move | 12 (fix/437) | 12 (fix/437) |
| 4-150 Procedure_Camera_MoveTarget | 8 (fix/437) | 8 (fix/437) |
| 4-152 Procedure_Camera_MoveShift | 4 (fix/437) | 4 (fix/437) |
| 4-151 Procedure_Camera_MoveLookTarget | 2 (fix/437) | 2 (fix/437) |
| 5-25 Action_GoToWP | 1 (816) | 8 (1060:2, 927:2, 309:2, 1862:1, 544:1) |
| 3-220 ReduceIndexOfVectorArray | 0 | 18 (927:8, 1060:10) |
| 3-153 Func_GetNormalCollideVector | 0 | 18 (927:9, 1060:9) |
| 1-30 Cond_CollideMovingZDM | 0 | 8 (396:2, 1203:2, 374:2, 1060:1, 927:1) |
| 4-356 Proc_GPI_SetTextNumber | 0 | 5 (1308:4, 284:1) |
| 4-209 with a non-behavior operand | 0 | 30 (927:15, 1060:15) |

These counts are stand-in lines from
`grep -rhoE 'stand_in\("[^"]+"' godot/generated/hype/levels/<level>`.
Generated code can repeat a source node, so the counts may exceed the
number of source sites.

## Common conventions (seen in the decomp)

- An AiValue is 16 bytes, with the payload at +0/+4/+8 and the tag at
  +0xC. Tags:
  - 6: integer
  - 8: real
  - 0xC: perso
  - 0x15: control
  - 0x16: vector
  - 0x18: DSG variable id
  - 0x21: waypoint
  - 0x23: comport
- Operands are evaluated in node order by 00522610.
- Procedures go through evaluator 005223c0: control is reset to 0, and the
  result is `control & ~4`. None of the procedures below sets control
  except the comport changes.
- Ultra receiver: 00525920 swaps in a pending receiver only if the
  callable's Ultra flag is set.

## 3-220 ReduceIndexOfVectorArray (VDVL, 00536e60)

Raw Ghidra `reference/ghidra/functions/00536e60_FUN_00536e60.c`, checked
against the disassembly 0x536e60..0x537018.

```c
A   = eval(op1).xyz                         // vector
dsg = *(u8*)cursor; cursor += 8             // D is NOT evaluated: raw DSG-var id byte
base = dsg_storage(caller, dsg)             // elements at base+8, stride 12 (x,y,z)
B   = eval(op3).xyz
n   = (int16)eval(op4).word                 // raw low16, no real->int conversion
P   = A + B
sec = 00448010(caller)                      // actor's sector (perso+0x28 ->+4); 0 if not an actor
if (sec && n > 0)
  for i in 0..n-1:
    Q = base[i] + B
    if (clear_segment_00536da0(&P, sec, &Q)) return int(i)
return int(n)
```

- 00536da0 tests segment P→Q with HIE_bDetectIntersectSegmentWithSuperObject
  (004ff230) against the sector, then against each neighbour sector. Any hit
  means blocked.
- Result: the index of the first array point (offset by B) visible from A+B,
  or n if none is visible.
- No bounds check of n against the array length (storage+4 is inferred to be
  the length). No type check on the variable.
- Port: stateless and per-actor. Needs a line-of-sight raycast limited to
  the actor's sector plus its neighbours, and DSG vector arrays. In Godot,
  a physics ray query against level collision is the likely stand-in
  (inferred; it is broader than the source's sector set).

## 3-153 Func_GetNormalCollideVector (no operands, 0053b990)

No raw export; read from the disassembly. The entry is shared and dispatches
on id 0x99. Other ids report error 0x27.

```c
00525920(ultra_flag(153), caller, &recv)    // consumes a pending Ultra receiver, but result unused
rpt = 0052ec80(caller)                      // caller->perso[+8] -> [+0x10] + 0xD8 (dynamics collide report), 0 if absent
return vector(rpt ? rpt[0x24+0xC .. 0x24+0x14] : (0,0,0))
```

- Returns the obstacle normal from the actor's last dynamics collision.
  The sibling 3-174 Func_GetNormalGroundVector reads report+0x94 instead.
- The Ultra receiver being consumed but not used is read, not inferred;
  why it happens is unknown.
- Port: per-actor read of "last wall/obstacle contact normal", zero when
  none. CharacterBody3D `get_wall_normal()` or `get_last_slide_collision()`
  is the natural source (inferred mapping). The exact moment the source
  updates its report (every dynamics step?) was not traced.

## 1-30 Cond_CollideMovingZDM (UV, 00533c90)

No raw export; read from the disassembly.

```c
idx = eval(op1).word                        // 1-based ZDM index
V   = eval(op2).xyz                         // offset in actor-local space
return 00430930(caller, 1, idx, &V) == 0
```

00430930:
- `zdm = 0042e5f0(1, idx-1, actor)` walks the ZDM list in the actor's
  collide set (perso+0x18, slot [2]) idx-1 times. idx 0 wraps to 65535
  links; there is no guard.
- If zdm is null it returns 0, so the condition is TRUE.
- It resets the global collision result (0x733800 = 2.0f, hit flag
  0x733830 = 0).
- It builds a matrix that copies the actor's global matrix, with the
  translation replaced by `M * V`.
- It collides the moved ZDM (004309f0) against:
  - the actor's sector and active neighbour sectors;
  - static geometry;
  - collide sets of actor types 0xb/0x14 there (00430c60 was not analysed).
- It returns the hit flag from COL_fn_bGetResultCollision (00551590).

The flag sense, read from the disassembly:
- 005514e0 records a contact and sets 0x733830 = 1. 00551590 returns that
  flag.
- 00430930 ends `test al,al; sete al`, so it returns 1 for no hit and 0 for
  a hit. A missing ZDM returns 0 through `xor al,al` at 0x4309d8.
- The condition (0x533d03, `neg al; sbb eax,eax; inc eax`) yields
  `ret == 0`.

So Cond_CollideMovingZDM is TRUE when the moved ZDM hits something. It is
also true when the ZDM index or collide set is missing.

- Side effects: it overwrites the global collision result buffer. It does
  not move the actor.
- Port: per-actor, and needs the collision backend: the actor's ZDM shapes
  plus a shape cast at an actor-local offset. In Godot this is a
  `PhysicsDirectSpaceState3D.intersect_shape` with the ZDM sphere/box at
  `global_transform * V` (inferred mapping).

## 5-25 Action_GoToWP (W)

- Initializer: the generic 00522a50
  (`src/ai/waypoint_meta_parameter_capture.c`). It evaluates W and stores
  the raw payload (the waypoint pointer, tag 0x21) in scratch cell 0 of the
  meta slot, via 0052e1d0. There is no null or tag check.
- Updater: 0054b110 (disassembly).

```c
wp     = scratch0(caller)                   // 0052e240
radius = *(float*)(wp+0x10)                 // 005169e0
loc    = 005173f0(wp)                       // wp+4 vertex, times owner SO matrix if wp+0x14 != 0
return 0054a960(caller, &loc, radius)       // src/ai/waypoint_motion.c
```

0054a9f0 (decomp, constants checked in the exe):
- No dynamics on the caller: return 4.
- Otherwise it clears dynamics+4 bit 0, removes event 0x0C, and computes
  `disp = loc - pos`.
- If `|disp| > radius`, the move is not finished:
  - With radius 0 and `|disp| <= 0.25`, it completes.
  - Otherwise it writes the unit direction to motion state +0x1C..+0x24,
    queues dynamics event 0x14, and returns 1 (running).
- On completion with radius exactly 0, it snaps the actor's translation to
  `loc` and returns 4 (done).

Other points:
- The updater writes no scratch beyond cell 0, so re-entering the node
  restarts cleanly. A null waypoint crashes (no guard).
- 5-26 Action_GoToWPXY (0054b160) is the same, except the target z is set
  to `pos.z - 0.05` first (`src/ai/waypoint_meta_position.c:108-122`).
- Port: per-actor state is just the waypoint reference (plus the meta-slot
  resume from the policy note). It can reuse the existing GoToPosition path
  (5-32/33): the target is the waypoint's world position, the arrival radius
  is the waypoint's radius, and a zero radius snaps. Movement goes through
  dynamics' "move toward direction" (event 0x14), not a teleport.

## Camera procedures 4-150, 4-151, 4-152 (PLV) and 4-154 (PV)

- Raw Ghidra exists for 00547200, 00547360 and 005474c0. Ghidra's float
  compares against -NaN/0/1.4e-45 are really integer compares with -1/0/1,
  confirmed at 0x547200.
- 00547980 (4-154) was read from the disassembly.
- All the 4-15x stand-ins are in Fix model 437, the camera.

```c
// 150 MoveTarget (+0x0C), 151 MoveLookTarget (+0x18), 152 MoveShift (+0x24)
so  = 00438e00(eval(P).word + 4)            // P: perso; plain evaluate, no Ultra
cam = so->perso[+0x14]; blk = cam[+4]       // active camera parameter block
if (mode(blk)=blk[+0x98] in {101,104,105}) blk = cam[+0xC]   // alternate block
m = eval(L).word; v = eval(V).xyz
if (m == -1) new = (*cam[0])[same offset]   // base/initial block
else if (m == 0) new = v                    // absolute
else if (m == 1) new = blk[off] + v         // relative
else new = <uninitialised stack>            // no guard
blk[off] = new
```

```c
// 154 Camera_Move
so = 00438e00(eval(P).word + 4); v = eval(V).xyz
d  = POS_fn_vRotateVector(so+0x24 matrix, v)   // rotation only
cam_block(cam[+4])[+0] += d                    // camera position, no alternate-block switch
00441090(so->perso[+0x14], 0x40)               // camera flags +0xCE |= 0x40
```

- Field names are inferred from the procedure names. The source has no
  names for blk+0x0C/+0x18/+0x24. "-1 = restore from base block" is
  inferred from the base-block read.
- In the port, 12 stand-ins are "unbound camera actor": the P operand
  doesn't resolve. The binding has to be fixed before these procedures can
  run.
- Port: per-camera state, on the camera's active or alternate parameter
  block. The existing ChangeCameraAngle (4-153, `ai.rs` ~588) is the
  template. Flag 0x40 at +0xCE's consumer was not traced; it presumably
  marks the position as script-moved (inferred).

## 4-356 Proc_GPI_SetTextNumber (SSL, 0053ef50)

The entry is shared for ids 349..357 through the jump table at 0x53f300.
Case 356 is at 0x53f25e (disassembly).

```c
page = eval(op1) (char*); item = eval(op2) (char*); n = eval(op3).word   // raw word, no real->int
_itoa(n, buf, 10)
00447300(page, item, buf)
```

- 00447300 (raw Ghidra) finds the page by case-insensitive name in the
  list at 0x745710 (next +0xE8), then the item in page[+0xDC] (next
  +0x110). It strcpys the text into item+0xC8; NULL text writes "".
- There are no null checks, so an unknown name crashes. The field size is
  unknown.
- 4-355 Proc_GPI_SetText is the same without `_itoa`.
- Brigand sites: model 1308 (4), 284 IWorld_Brigand (1).
- Port: global HUD text registry keyed by (page, item), case-insensitive.
  Set the text to the decimal integer. No control effect.

## Comport requests with a non-behavior operand (4-209)

The operands, from a raw node scan of `brigand.sna` with my scratch script
`scanreq.py` (same method as the policy note):

| Request | Operand node | Brigand | Manoir |
|---|---|---|---|
| 4-209 | kind 32 comport literal | 334 | 285 |
| 4-209 | kind 11 DSG var 45, 46, 51 | 22 | 0 |
| 4-209 | kind 9 engine field 7 | 6 | 0 |
| 4-210 | kind 32 | 63 | 37 |
| 4-207 | (actor, kind 32); actor operand kind 0/11/24 | 121 | 111 |

That gives 28 non-literal 4-209 source sites against 30 stand-in lines. The
gap is presumably duplicated generated code; that's inferred, not checked.

### Semantics

The four procedures (207, 209 and 210 read from the disassembly; 208
already in `src/ai/training_receivers.c:25`) all end in 005426f0
(`src/dialogue/dialogue_lifecycle.c:245`):

```c
if (intel[+8] != behavior) {               // current comport
    intel[+0xC] = intel[+8]; intel[+8] = behavior;     // 005216e0: prev = cur; cur = new
    reset_meta_headers(intel); intel[+4] = behavior[+4] ? *behavior[+4] : 0   // schedule start
    if (!suppress) control = 8;            // restart rules this frame
}
```

- 209: `intel = mind+4`, normal intelligence, where
  `mind = 004600c0(caller->perso[+0x10])`. `suppress = (mind[+0x10] == 0)`.
- 210: `mind+8`, reflex. `suppress = mind[+0x10]`.
- 207/208: same as 209/210, but on the P actor. When the target is not
  the caller, suppress is always 1.
- mind[+0x10] selects which intelligence is running, so control 8 only
  fires when the actor changes the group that is currently executing.

The operand goes through plain 00522610. The tag is never checked, and the
payload word is used as the comport pointer. A null comport crashes at
behavior[+4] (inferred from the disassembly order).

Operand kinds seen:
- **Kind 11, DSG variable.** 00523a50 reads the actor's designer variable.
  For a comport-typed variable, the type reader 005238a0 writes tag 0x23
  with the stored pointer. Port: resolve the actor's comport-typed DSG
  variable (45, 46 or 51 in models 927/1060) at run time, then do the
  normal change. The variable's values come from earlier assignments of
  comport literals, so they're a finite set per model. Enumerating those
  assignments would let the converter compile a switch (inferred
  approach).
- **Kind 9, engine field 7.** 0052de60 → field getter 00536000:

  ```c
  mind = 004600c0(caller->perso[+0x10]); result = { (mind+4)[+0xC], tag 0x23 }
  ```

  This is the normal intelligence's previous comport. The getter's body was
  read. The id-7 mapping comes from the field table at 0x5ffd7c, stride
  0x20. Read as `{name[20], type, getter, setter}`, the
  "PrevComportIntell" name lines up with getter 00536000; that mapping is
  inferred. 00536030 is the reflex twin, PrevComportReflex.
- **Port:** "go back to the previous normal comport". The port needs a
  per-actor previous-comport record, updated on every change (005216e0
  writes intel+0xC before every switch, including changes made by other
  actors' 207).

## Checklist lines

- [ ] 4-154 Procedure_Camera_Move: move the camera position by a vector in the subject actor's frame and set camera flag 0x40 (Manoir and Brigand, 12 each, Fix 437; P operand currently unbound).
- [ ] 4-150/151/152 Camera_MoveTarget/MoveLookTarget/MoveShift: set, add or restore one vector of the active or alternate camera block (8/2/4 per level, Fix 437).
- [ ] 5-25 Action_GoToWP: move toward the waypoint's world position until within its radius, snapping when the radius is 0 (Manoir 1, Brigand 8).
- [ ] 3-220 ReduceIndexOfVectorArray: first visible point of a DSG vector array from A+B, else n (Brigand 18, guards and thieves).
- [ ] 3-153 Func_GetNormalCollideVector: actor's last obstacle-collision normal, or zero (Brigand 18).
- [ ] 1-30 Cond_CollideMovingZDM: would the actor's ZDM idx hit anything at a local offset V; true on a hit or a missing ZDM (Brigand 8).
- [ ] 4-356 Proc_GPI_SetTextNumber: set a HUD text item to a decimal integer by page and item name (Brigand 5).
- [ ] 4-209 with a DSG comport variable (22) or the PrevComportIntell field (6): change to a run-time comport (Brigand 927/1060).
