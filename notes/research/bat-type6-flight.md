# Brigand bats: identity-card type 6 (static research, 2026-11-04)

Nothing was run. Sources: hype-decomp reference/ghidra/functions, PE disassembly of
MaiDFXvr_bleu.exe, src/dynamics/*, src/ai/dynamics_event_procedures.c.

## 1. Which function is the type 6 entry

00443540 fills the table at 0x00757320 through its pointer argument with plain
`mov [eax+k], imm` stores. `param_1[2t]` is the +0 entry and `param_1[2t+1]` the +4 entry
of type t. Cross-checks: type 9 -> index 0x13 = 0058ab89 (the Mass kind9 control the
repo already names); type 3 -> indices 6/7 = 00578020/0057910c (matches type3_query.c).

Type 6 (indices 0xc/0xd):
- +0 entry (0x00757320+6*8): **00588198**. Per-frame callback, called by 00443f00 at
  00443fd4 with seven words (dynamics, superobject, parent, environment, workspace,
  identity, seconds). Type 6 reads only the first. It does 14 word copies
  (+2d8..+2f0 -> +2bc..+2d4, +310..+328 -> +2f4..+30c) and returns. It does not move
  anything. Rewritten: hype-decomp src/dynamics/type6_frame_snapshot.c (commit 42030ba).
- +4 entry (0x00757324+6*8): **0058ceea**, the same function the repo rewrote as the
  "gate-lift control" (src/dynamics/lift_control.c). 00443e00 calls it with six words
  (dynamics, superobject, workspace, identity, seconds, event).

Port mode == card type: the mechanics file starts with u32 kind (src/openspace/level/mechanics.rs
`Header.kind`; 00594093 DNM_fn_eIdentityCardGetType returns card+0, and the dispatchers
index the table with it). 00591464 case 6 allocates 0x5c bytes, zeroed, no defaults,
sets `*card = 6`. So the port's 92 byte record (4 + 88) is exactly this. Cards 141730
(brigand bats) and manoir 49183 are the only type-6 cards in the generated levels;
49183 is shared by Grille (gate) and PigeonPP persos, so type 6 = the shared
gate/lift/bird kinematic kind, not a character-style kind.

## 2. Behaviour of the type-6 update (0058ceea) for event 0x0c and for no event

Events: Proc_GoRelative (34) -> 00540220 -> event 0x0c -> 00443e00 -> 0058ceea.
Proc_SetVectorSpeed (29, ai_vector_speed.c) only zeroes dynamics+0x64..0x7c and writes
the receiver's "parameters" block (0052ec40/0044d2e0 block; norm speed +0x28 etc.).

0058ceea branches only on event 14 (start) and 15 (advance). Any other id (including 0x0c)
skips both branches and falls into the common tail, which (all with evidence in
lift_control.c and Ghidra 0058ceea):
- loads position P = dynamics+0xa4..0xac (it is its own "current position" input);
- republishes workspace +20c/+210/+218/+224 unchanged;
- stores velocity words dynamics+0x64..0x6c = -dt * (P - P) = zero (signed zero);
- writes P back to +0xa4..ac;
- POS_fn_vSetTranslationVector(HIE matrix of the superobject, P).

Consequences:
- No velocity, no local/world axis rotation, no gravity, no card field (speed, limits,
  mass), no collision/ZDR query is used for event 0x0c. The vector from SetVectorSpeed is
  not read by this function. Argument 4 (the card) is not read at all by either type 6
  entry, so the 88 byte body has no effect through these two functions. (The accessors
  for mass/speed/limits in src/dynamics/identity_card_*.c only handle types 0/4/0x12 and
  return defaults for 6.)
- Nothing persists: there is no integrated state. Frames with no event do not call +4
  at all (00443e00 is per queued event); 00443f00 then only runs the +0 snapshot.
- The only motion the entry can produce is the lift interpolation when events 14/15 are
  posted (Meta36 gate path): start->end along the line with exponential-ish easing,
  state from dynamics+0x368 (factor, start xyz, end xyz).

## 3. Open uncertainties

1. Source of dynamics+0xa4..0xac for a type-6 perso: not read here. If it were stale or
   zero the 0x0c event would snap the actor to it each time. Since shipped bats do not
   snap, something (placement/type-switch code, 005711f7 copies 0xa4 -> 0x310, or the
   animation/transform path) must keep it equal to the real position. Not traced.
2. Where the bats' visible flight/dive comes from in the original is not established
   by this trace. The type-6 dynamics do not integrate GoRelative. Candidates:
   animation root motion of the actions the AI requests (the port already takes wolf
   speed from root-motion), or another per-frame path before 004440e0. Not read.
3. Whether the engine drops the 0x0c event earlier than 00443e00 (e.g. 0052eeb0 /
   004adfe0 queue limits): not checked; the repo's event_queue.c only enqueues.
4. The 88 byte body of card 141730 was not decoded. Static evidence says it is unread
   by the type-6 entries; its meaning, if any, lives in other consumers (unfound).

## 4. Recommended Godot contract (character.gd, type-6 actor)

Do not invent a flight model. Faithful to the evidence:
- `source_set_vector_speed` on a type-6 actor: clear the stored velocity words and store the
  vector in the actor's parameters block only (no consumer for type 6). Do not apply it.
- `source_go_relative` on a type-6 actor: post event 0x0c; its type-6 handling is a no-op
  on position and velocity (re-publish of the cached position, zero velocity). Do not
  translate the actor, do not rotate the vector, no gravity, no collision query.
- Persistence: none. Nothing carries between passes.
- Motion that is genuinely type-6: events 14/15 lift interpolation (already the gate
  contract: see notes in src/dynamics/lift_control.c). Keep that as is.
- If the bats must still reach the player to match recorded play, the extra motion has to
  come from the action/animation root motion path (open item 2), and should be labelled
  as an adaptation in undocumented-adaptations.md until traced; do not attribute it to
  SetVectorSpeed/GoRelative.
- Test to write red-first: a type-6 actor receiving set_vector_speed(0,-5,dz-0.5)+go_relative
  for N passes keeps its position unchanged (apart from other drivers), and its velocity
  stays zero.

## Live result and port decision (owner, 2026-11-04)

Private disposable original. The take's SaveGame2 (forest save statue, 437 s) was copied into the
private copy's slot 1 and loaded from the menu. Hype was walked west with Down holds.
- Before: Hype at source (56.07, -26.47, -16.66), about 41 m from the bat dummy. Bat 1 in normal 2
  at its spawn (19.76, -30.29, -7.54), mind+0x10 = 0.
- Hype at (35.12, -21.60, -13.55), about 20 m from the dummy: bat 1 switched to normal 0 with action 4.
- It then flew (21.54, -32.33, -8.46), then (27.31, -27.77, -12.79), toward Hype and down to his
  height.
- Later, in patrol (normal 1, the same 5 m/s vector), it circled the dummy with z steady.

So in the original, SetVectorSpeed plus GoRelative moves a type-6 bat. The static reading that
0058ceea ignores event 0x0c does not account for this motion; where the original's translation
comes from is still not traced in the decomp. The motion is not animation root displacement:
state 4 alone carries 2.73 m per 0.3 s, and the bat circled at about the vector's speed.

Port (character.gd):
- `source_go_relative` enables motion for an actor that has a mechanics card. 00540220 posts nothing
  without dynamics.
- A type-6 actor takes only the rotated vector: no root displacement and no gravity.
- Probe `bats`, before: no bat leaves its spawn. After: bats 46, 47 and 48 fly at Hype and dive
  (normal 4), and Hype goes from 50 to 49.
- Manoir New Game smoke stays playable.
- Limits:
  - Speed was not timed against game time (each read pauses the game).
  - The other type-6 users (Grille gates, PigeonPP) take their lift path first and are unchanged.
