# Mechanics identity-card type 19: how walking NPCs move (static research)

Method: static reads of hype-decomp Ghidra exports (reference/ghidra/functions/*.c) and objdump of
MaiDFXvr_bleu.exe (443f00). Nothing was run. No hype-decomp commits made (all target bodies were
unrecovered; rewriting was out of scope for this question).

## 1. Table entries (00443540 fills 0x00757320, 8 bytes per type; word index = 2*type, +1 for update)
- word 0x26 = type 19 +0 (per-frame integrator) = 0057e472
- word 0x27 = type 19 +4 (event/update entry) = 0057e110
- word 0x28 = type 20 +0 = 0057d3b4 ; word 0x29 = type 20 +4 = 0057e110 (shared update)
- For comparison: type 0 +0 = 00585d51, +4 = 00589bfa (word 0x1d = type 14 +4 also 00589bfa).
- 0057e110 role (confirmed): event handler, NOT an integrator. Signature (block, superobject, state,
  card, dt_seconds, event_id); matches the 6 words pushed at 443ed5..443eeb. It only writes velocity
  requests into the per-Perso state (dynamics+0x14) and sets block+0xe0 (mode): events
  3/4 (yaw rate, mode 2), 5 (turn to angle, mode 2), 0xc (translation velocity, mode 1),
  0xd (position request into state+0x1b0, mode 1). Other event ids: no-op (leave state untouched).
- 0057e472 (type 19 +0) is the per-frame integrator, called by 443f00 at 00443fd4
  (`call [ebx*8+0x757320]`, 7 stack args: block, superobject, father-or-0, 447f90(superobject+0x28)
  collision set, state(44d2e0), card, dt seconds = ms 0x754af8 * 0.001 as float).

## 2. What a type-19 actor receives each tick (answer: animation root motion, yes)
Chain per engine tick (4499b0, once per engine frame guarded by DAT_00754aa8):
1. 00443960 -> 0058e782(state) zeroes state words 0x66..0x6b (0x198..0x1ac) and 0x73..0x79
   (0x1cc..0x1e4: translation vel xyz + yaw-rate angle + axis xyz). So velocity is NOT sticky; if no
   event arrives this tick the actor gets zero speed.
2. 00433e90 (animation step, only if 00442af0==0) ends with
   `if (DAT_00610b5c == 0) 004436c0(perso, prev_frame)` (DAT_00610b5c is set to 0 by 4499b0 at +0x48
   before; no other writer found, so effectively always true).
3. 004436c0 reads the animation displacement:
   - 00435930: translation channel (anim data +4 count, per-frame node +8 -> vec3 delta). Same integer
     frame: vec = fps(byte 0x17a of anim state, FUN_00442e60) * delta[frame]; crossing frames: partial
     first + full middle + partial last, then * 1/dt. Result is units/second (global DAT_00610a08).
   - Stores |v| at block+0x390 and unit v at block+0x384/0x388/0x38c; enqueues event 0xc (card =
     current state's card, 443690 -> 0044a910 -> state+0x30) via 0044d490/004adfe0, only if v != 0.
   - 00435db0: product of per-frame rotation matrices (anim node +4, 9 floats) for frames
     prev+1..current; 0059003e/0058f8b3 convert to angle-axis; block+0x39c = angle*(1/dt), axis in
     block+0x3a0..0x3a8; enqueues event 3 if angle != 0.
   (Queue = 8 slots: ids at q+4+i*4, cards at q+0x24+i*4; count byte at q+0. 4440e0 enqueues a default
   id-0 event if empty.)
4. 004442e0 -> 00444090 (AI) then 004440e0: per queued event -> 00443e00 -> type's +4 entry
   (0057e110): case 0xc: v = dir*mag (block+0x384.. * 0x390), rotated by ROTATION of the superobject
   global matrix (0059d14e) -> state+0x1cc..0x1d4. NO speed clamp, no card limits (type 0's 00589bfa
   clamps to card max linear speed and rotates by block yaw +0xb0).
   case 3/4: yaw rate = +/-(0x39c * 0x3a8) about z (005710b4 -> {rate,0,0,1}), composed into
   state+0x1d8 via 0058f5b4. case 5 (what Proc_TurnPerso 00542c80 enqueues: FUN_0052eeb0(..,5), angle
   at request+0x44 = block+0x3ac, axis z): yaw rate = wrap(target - yaw(block+0x31c)) / dt, i.e. the
   turn completes in ONE tick (0058f5b4/5a0980 wrap to +-pi).
   Then 443f00 calls 0057e472.
5. 0057e472 (the integrator) at instruction level:
   - copies prev pose (block 0x2d8..0x2f0 -> 0x2bc..0x2d4; 0x310..0x328 -> 0x2f4..0x30c);
   - speed S = state+0x1cc (7 floats). If state+0x1b0..0x1bc (position request, from event 0xd) is
     non-zero: block+0xa4..0xac := request, request cleared (teleport/set position);
   - if card[5] != 0: 0059cd2f ground probe along (0,0,-1) then 0057e080: S.xyz += n.yzw * (-n.x) * dt
     (ground-follow/slope adjust; record layout of ccd0/f0b0 not fully read);
   - 005a070d(block,S,out,dt): out.pos = block pos(0xa4) + S.xyz*dt (005a05c8/59d0e0);
     out.yaw = compose(yaw, S.angle*dt about S.axis) (005a0642/58f170);
   - if card[4] != 0: 0057e8d2 swept collision test (00463df0 vs block+0xfc contact struct) from old
     pose block+0x310 to new pose; on hit it pulls the new position back along the path
     (fraction at contact+8, 0059d6f1), zeroes the incoming speed, saves the 0x70-byte contact;
   - 0059f46d commits: writes superobject global rotation/translation (matrix mode by block+0x10:
     0 world, 1 father-local, 2), block+0x64.. speed and +0xa4.. pose, ground info (0x344), history.
   NOT used: gravity, mass (DNM_xIdentityCardGetMass), 005a1948 force accumulation, damping,
   accumulated velocity (0x64 only mirrors the commit), momentum.

## 3. Gating
- Perso must have dynamics (+8 of its Perso data) with block (44d2c0 != 0) and a card on the current
  state (state+0x30); `4499b0` must reach 4442e0/4440e0; the type changes via 005711f7 on type switch.
- Translation only exists if the CURRENT animation has non-zero translation channel deltas
  (root motion authored in the animation). Barrels (type 19) stay put because their states' animations
  carry no translation/rotation channel (event 0xc/3 never enqueued -> S = 0 every tick; 5a070d with
  S=0 re-commits the same pose). No dynamics flag is needed to explain this; the card is the same.
  (Barrel animation data not inspected here - inference from code, not from the asset.)
- card[4] (collide) and card[5] (ground follow) are per-card bytes; names unconfirmed. Anim step
  skipped when 00442af0() (frozen) -> no events -> no motion.
- Anything that rotates the actor (event 3/4/5) uses z axis only.

## 4. Type 0 comparison / port model
- Type 0 (00585d51 / 00589bfa) consumes the SAME block+0x384/0x390 anim velocity but as a thrust:
  clamps to card max speed, rotates by block yaw, adds external block+0x3f0, then integrates with
  mass, gravity (5a1948), damping, slide/ground/collision (59a504/59ae97, father change).
- Existing port "root-motion delta" model applies to type 19 with these corrections: it is pure
  kinematic anim velocity (no gravity/mass/clamp), rotated by the actor's global rotation matrix as of
  tick start, yaw changes (anim rotation channel and TurnPerso) applied the same tick.
- Rejected: (a) "meta-action/motion procedure translates Wellet" - not needed: TurnPerso only
  enqueues event 5 (rotation); translation comes from the run animation; (b) sticky velocity - 443960
  clears it every tick; (c) 0057e110 as integrator - it is only a request writer; (d) gravity for
  type 19 - no gravity code on 0057e472 path.

## Runtime contract for Godot port
- Each tick, for an actor whose current state's mechanics card type is 19 (or 20 via 0057d3b4, which
  differs from 19 in not calling 0057e080/ground-follow and passing the card to 0057d788; unread):
  vel_world = R_actor_global * (sum of anim per-frame translation deltas crossed this tick) / dt;
  pos += vel_world*dt (= interpolated sum, partial first/last frame); yaw += sum of anim
  rotation-channel angle (about z) this tick; TurnPerso request sets yaw to target (wrapped) in the
  same tick. No gravity, no mass, no speed clamp, no carry-over to next tick.
- If card[5]: adjust by ground probe (unverified magnitude); if card[4]: swept collision vs. world
  stopping at contact (position pulled back along path, speed zeroed).
- Actors with no translation channel in the playing animation do not move (barrels, idles).

## Confidence
High: table mapping, handler roles, per-tick clear, anim->event derivation, no gravity/mass on 19.
Medium: unit conventions (velocity units/s, yaw sign for events 3/4), meaning of card[4]/[5],
coordinate mode details (block+0x10 per-type table DAT_0060dd90). Unread: 0057d788 (type 20),
0059ccd0 record layout, 00463df0 internals, 005a0980.
