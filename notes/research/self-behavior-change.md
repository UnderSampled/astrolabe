# Self behavior change (Proc_ChangeComport on the executing actor)

Static read-only research, 2026-11-03. Nothing built or run. Line numbers are
from the working copy and can drift. "V" = verified in code or decomp,
"I" = inferred.

## Headline

The "291 skipped self switches" in undocumented-adaptations.md is a count of
guard sites, not of self switches. The guard is a runtime test
(`change_actor == character()`), and almost none of the 291 can be true.
(V) Counting references that resolve to the perso running the model (Brain
scripts from each level.tscn against `source_actor_node("level:/persos/...")`),
only 3 sites can fire: manoir model 1290 lines 87, 471, 489 (target perso 163;
normal 6, 6, 4). The other 288 are:
- 89 `main_actor` sites. main_actor is Hype (`main_actor_path ../../../0/Hype`),
  and the only model Hype runs is fix/28, which has no main_actor change site.
- Reference sites aimed at another perso (I: not exhaustively mapped for models
  that run on several persos).
- 3 `dsg_*` variable sites (brigand 1203 x2 via dsg_34, 1445 via dsg_0). Unknown
  until run.

## 1. What ai.rs does today

(V) src/godot/ai.rs:4269-4291, `Statement::ChangeActorBehavior { actor, target }`.
Emitted per site, in order:
1. `var change_actor_N: Node3D = <actor_target_code>`. The actor expression is
   `character()` for SelfActor, `main_actor`, `dsg_N`, or
   `character().source_actor_node("<path>")` (ai.rs:910-920).
2. `is_instance_valid` check, else `fail_unsupported(...different target actor with a Brain using <model>)`, then exit.
3. `if change_actor_N == character(): fail_unsupported("<model> external <group> request cannot switch the currently executing actor because its source control-status semantics are unresolved")`, then exit.
4. Brain lookup: `source_model_path` must equal the target behavior's model and
   the Brain must have `change_<group>`, else the same `fail_unsupported`.
5. `change_brain_N.call("change_normal"|"change_reflex", <id>, false)`.

`exit` is `if strict_unsupported: return` in the game. `fail_unsupported` records
the failure, then execution falls through (continue_past_guards, ai.rs:343-358).
After the guard the generated code still runs steps 4-5 on `character()`'s own
Brain. So in the non-strict game a true self call is not skipped. It records the
failure and then calls `change_normal(N, false)` through the Brain lookup, if
the model path matches. I did not run it, so this is read from the text.
(I) Gap: with `strict_unsupported` it returns instead.
Operations covered: Proc 207 Proc_ChangeComport (normal) and 208
Proc_ChangeComportReflex (reflex); openspace/ai.rs:1002, 1255-1302. Group is
`id == 207 ? Normal : Reflex`. 209/210 (ChangeMyComport[Reflex]) are the
`Statement::ChangeBehavior` path at ai.rs:4234-4268.

Example: manoir 991.gd:1480-1498 targets `main_actor` (Hype, fix/28), changing
normal to 74 or 50. That is an ordinary cross-actor call, and the guard is
false for it.

behavior.gd (V):
- `change_normal(target, defer_entry)` (:367): returns false when
  `current_normal == target` (no restart). Otherwise it sets current,
  `normal_revision += 1` (cancels the old coroutine), `normal_resume_rule = 0`,
  `_normal_pending = true`, and `_normal_deferred = defer_entry`.
  `_step_normal` (:~380) enters pending behaviors in the same physics step
  unless deferred. `change_reflex` (:402) is symmetric.
- Hold policy state: `normal_rule_claimed`, `normal_resume_rule`,
  `normal_action_revision`. ChangeBehavior with `pause_after` sets
  `<group>_rule_claimed = true` and bumps the action revision, then awaits the
  tick.

## 2. Original semantics (decomp)

All four procedures end in `005426f0` (hype-decomp
src/dialogue/dialogue_lifecycle.c:245; astrolabe
reference/decompilation/functions/005426f0_FUN_005426f0.c):
```
if (intel[+8] != behavior) {            // same behavior requested again: no-op
    changed = FUN_005216e0(intel, behavior);   // prev=cur; cur=new; reset meta headers; schedule ptr = behavior[+4][0]
    if (changed && suppress == 0) FUN_005223b0(8);   // procedure control bit 8
}
```
- (V) Same behavior again: no restart, no control bit.
- (V) The new behavior is published immediately (005216e0, src/ai/training_receivers.c:8-22).
- (V) 208 (00542720, training_receivers.c:25-52): evaluates the actor and the
  behavior; if `target == caller`, `suppress = (mind[+0x10] != 0)`, else
  `suppress = 1`. Then `005426f0(mind[+8], behavior, suppress)`.
- (V from earlier disassembly notes, notes/research/unsupported-ai-entries.md:270-280)
  209 (005427c0) uses mind+4 with `suppress = (mind[+0x10] == 0)`. 210 uses
  mind+8 with `suppress = mind[+0x10]`. 207 (00542650) is the normal twin of 208.
  `mind[+0x10]` says which intelligence is running, so control 8 fires only
  when the actor changes the group that is executing. I checked 208 in the
  decomp source but did not re-read the 207 body. Its self branch is I.
- Control 8 is consumed by the rule loop `00522840` (reference/decompilation/functions/00522840_FUN_00522840.c).
  The loop reads `local_10[0]` from 00522650. Bit 1 holds and stores the resume
  rule at `storage+7`. Bit 8 breaks out and restarts the outer `do` loop in the
  same call, which re-reads the new behavior's rules from rule 0 (the same
  frame). Bit 0x10 continues to the next statement. Anything else returns 0.
  `005223c0` (ai_receiver_dispatch.c:~108) zeroes the control before each
  procedure and returns `control & ~4`.

Answers:
- Self switch of the running group (207 from a normal script, or 208 from a
  reflex script): (V for 208, I for 207) the switch is immediate. The rest of
  the current script is abandoned and the new behavior's rules start again in
  this frame.
- Self switch of the other group (normal script calling 208 on itself, or the
  mirror case): suppress = 1, so no control 8. The new behavior is published and
  the current script continues. The new one runs on its next evaluation.
- Cross-actor (target != caller): suppress is always 1, so the caller always
  continues. This is what the port already does.
- Dedicated self (209/210) is the same helper with the same suppress rule. The
  only difference is that the actor is implicit. So a self 207/208 should behave
  exactly like the already-emitted `ChangeBehavior`.
- With a preceding Proc_SetActionReturn 1 (P=1), the merged result holds. See
  docs/astrolabe/action-return-policies.md:232-240 and 253. That is where the
  "defer by one frame" comes from, and it is I per that doc (flagged "worth a
  probe").
- Reflex versus normal: only the intelligence slot (+4 or +8) and the mind[+0x10]
  polarity differ.

## 3. Recommended port

When the guard's `change_actor == character()` is true, emit the same code as
`ChangeBehavior` instead of failing:
```
if change_actor == character():          # model must still equal target model
    if change_<target_group>(N, <defer>):
        <exit if target_group == running group else pass>
```
- Same group: `change_normal(N, false)` then exit/return (control 8 restarts the
  new behavior this frame). `change_*` already returns false when the behavior
  is unchanged, giving the 005426f0 no-op.
- Other group: call `change_*` and do not exit (suppress = 1).
- Check that `target.model.path == model.path` at emit time, or compare
  `source_model_path` at runtime, and keep a `fail_unsupported` for a mismatch.
- If the call follows Proc_SetActionReturn 1 (hold_pending, openspace/ai.rs:1050),
  reuse the ChangeBehavior `pause_after` branch: `defer_entry = true`,
  `rule_claimed = true`, `action_revision += 1`, then await the tick.
  `ChangeActorBehavior` carries no `pause_after`, so the parser would need to
  add it.
Since the emitter already builds this for 209/210, the cheapest change is to
branch in the emitter when the actor resolves to SelfActor at compile time, and
when the actor is `ActorTarget::Reference` or a variable, add a runtime
`if change_actor == character()` branch around the above.

Uncertainty:
- The 207 self body and polarity are inferred from 208 and the 209 disassembly
  notes, not from a decomp body (00542650 has no raw entry in the Ghidra
  export).
- Whether Hype's own "previous comport" (intel+0xC) matters for 207 self.
- The P=1 plus self case (defer) is untested.
- Whether any 207 self call exists in shipped data beyond manoir 1290 (3 sites)
  and the 3 dsg-variable sites.

## 4. Recount (V)

Emitted marker: `fail_unsupported("<model> external normal|reflex request cannot switch the currently executing actor because its source control-status semantics are unresolved")`,
immediately before each `var change_brain_N`. It is emitted on every
ChangeActorBehavior site (the guard is the dynamic test), so it counts sites
that could be self, not sites that are.
- Total 291 in 70 files: brigand 147, manoir 144 (menu, inventory and
  pause-menu have no AI directory).
- Top models by site count: brigand 1800 (59), manoir 586 (29), manoir 154 (26),
  manoir 991 (19), brigand fix/28 (11), manoir fix/28 (11), brigand 599 (8),
  manoir 638 (8), manoir 939 (8), brigand 284 (7).
- Next: brigand 1203, fix/1 (both levels) and manoir 294, 868 (4 each).

## 5. Manoir 991 (Gogoud, perso 155) and fix/28 (Hype, perso 0)

(V) Gogoud is `level:/persos/world/0/155` (persos/i_Gogood_MasterYoda tscn).
Its Brain uses model 991 with `main_actor_path = ../../../0/Hype`.

991.gd guard sites (line = the `.call` line; target; to):
- normal 1 rule 0, :405; Hype; normal 50
- normal 2, :497; Hype; 74
- normal 7, :1145; perso 160 (arrows); 3
- normal 9, :1360; Hype; 74
- normal 10, :1499; Hype; 74
- normal 11, :1651 perso 149 (icons), normal 3; :1675 perso 149, reflex 0; :1784 Hype, 50
- normal 13, :2128 and :2359; Hype; 50
- normal 15 rule 1, :2851; perso 160; 3
- normal 16 rule 1, :3123; perso 160; 1
- normal 18, :3338 (74), :3442 (50), :3479 (74); Hype
- normal 20, :3881 (74), :3989 (50); Hype
- normal 26 rule 0, :4948 (50), :5012 (74); Hype

None targets perso 155, so the guard is false everywhere in 991. They are all
real cross-actor calls that go through. On the dialogue path
the only listed normal with a site is 16 (:3123); normals 5, 17, 19 and 23 have none. Training
normals 10, 11, 13, 15, 18 carry sites (10: :1499; 11: :1651, :1675, :1784;
13: :2128, :2359; 15: :2851; 18: :3338, :3442, :3479). Normals 4 and 14 have
none.

fix/28.gd (Hype, manoir): 11 sites, none self (targets 16 statue, 2 camera, 162
event; Hype is perso 0):
:1266 normal 0 -> perso 16 normal 4; :7256 normal 15 -> perso 2 normal 0;
:8478 normal 24 -> 2 normal 0; :8610 normal 24 -> 16 normal 3; :12672
normal 31 rule 0 -> 162 normal 1; :15061 normal 37 -> 2 normal 0; :15685
normal 38 -> 2 normal 0; :19067 normal 54 -> 2 normal 0; :19355 normal 58 ->
16 normal 3; :24192 normal 95 -> 16 normal 3; :28253 reflex 3 rule 10 -> 2
normal 0.

Conclusion: (V) none of these skips is a self switch, so the guard cannot be
the cause of a ~10 s longer dialogue or flaky training. (I) Candidates to
check instead, because these calls do run: the Hype switches to normals 50 and
74 on the key and training paths. Under the original, 207 on a different
actor sets no control 8 and the caller continues at once. In the port,
`change_normal(50|74, false)` on Hype's Brain runs in the physics step of the
caller's coroutine. Whether Hype enters it in the same frame depends on
Hype's Brain's `_step_normal` ordering, and a ~10 s excess probably comes from
some other wait, not from this guard. I did not trace which wait.
