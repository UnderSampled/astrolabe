# Action-return policies (Proc_SetActionReturn, procedure 4-6)

Port status (2026-11-03): P=1 before a procedure other than a self comport
change is compiled as `Statement::HoldAfterPrevious`: in a rule it claims the
rule, records it as the next pass's first rule and returns; in a schedule it
waits one tick. That covers the 100 Proc_Turn sites (guards 927, thieves 1060)
and the three other-actor Proc_ChangeComport sites. P=0 before a meta action
still compiles as a plain sequence; the resume-at-action behavior below is
not ported.

Static research, 2026-11-03, from hype-decomp (read-only). Claims marked
"inferred" were reasoned from the decomp but not seen as a direct code path;
everything else is quoted from decomp source with its address.

## What the procedure is

Procedure 6 is `Proc_SetActionReturn`, signature `L` (one integer), handled by
the shared dispatcher 0053fc30 (`hype-decomp/research/hype-ai-api.tsv:379`,
`src/ai/death_map_procedures.c:55`). It does not act, and it does not set its
own control. It arms a one-shot override that replaces the control bits of the
next statement the interpreter runs.

```c
/* 0053fc30, case 6 */
ai_write_saved_flag_1b5c_00522a20((uint8_t)operand.value.word); /* pending = policy */
ai_write_saved_flag_1b58_00522a40(1);                           /* skip one merge  */
```

The procedure evaluator 005223c0 resets `procedure_control_00721b54` to 0
before every procedure and returns `control & ~4`, so the 4-6 node itself
returns control 0.

## Where it is stored

- `0x00721b5c`: pending policy byte. 0x20 means "none pending" (any value with
  bit 0x20 set is treated as none). Initialised to 0x20 with 1b58 = 1 by
  00515850 (the AI init, which also loads `Defines.ai`; raw
  `reference/ghidra/all_functions.c:184631`).
- `0x00721b58`: skip flag. 1 means "the next merge belongs to the 4-6 node
  itself; don't consume yet".
- Per meta-action slot: `+0x7e` holds 1b58 and `+0x7f` holds 1b5c
  (`src/ai/ai_meta_slots.c`, `SourceMetaSlot`). 00525bd0 (slot header write)
  copies both globals into the slot when a meta action starts (via
  00525b60). 00525ea0 copies them back into the globals whenever a rule or
  schedule resumes from that slot. So a pending policy is per-actor, per-slot
  state that lasts across frames while the meta action that follows it runs.

## Where it is consumed: 00522750 (merge), called only from 00522650

```c
/* 00522750: called after every statement evaluated by 00522650 */
control = result.low_byte;                 /* this statement's control */
if (pending_1b5c & 0x20) return control;   /* nothing pending */
if (skip_1b58 == 0) {
    if (!((control & 4) && next_node.depth == this_node.depth))
        control = (control & 0x1c) | pending_1b5c;
    pending_1b5c = 0x20; skip_1b58 = 1;    /* consumed */
    return control;
}
skip_1b58 = 0;                             /* this was the 4-6 node itself */
return control;
```

So, for `Proc_SetActionReturn(P); S`, where S is the next statement-level node
that 00522650 evaluates (whatever its kind):

    final_control(S) = (control(S) & 0x1c) | P

That is, P replaces S's bits 0x01, 0x02 (and 0x20..0x80, which nothing
reads), and S keeps bits 0x04, 0x08 and 0x10. One exception: if S returned
bit 0x04 ("meta action finished") and the node after S is at the same depth
as S, S's control is kept unchanged and the policy is simply dropped.

## What the control bits mean (00522650 / 00522840 / 00522930 / 005227c0)

Node loop 00522650 (`src/ai/interpreter_execution.c:42`), checked in order:

```c
if (c & 0x10 || c & 8) break;
if (c & 4) ai_clear_selected_meta_slot_00525c30(ctx);  /* meta done: free slot */
if (c & 1) break;
if (next.kind == 8 /* end */) { c = 0x10; break; }
if (c & 2) break;
/* else continue to the next statement */
```

Rule loop 00522840 (rules of the current comport):

- `& 1`: store this rule's mode (index+1) in `storage+7`, return 1. Next
  frame, rule evaluation starts at this rule (earlier rules are skipped), and
  if this rule has an active meta slot it resumes at the slot's node.
- `& 8`: restart rule evaluation from rule 0 of the (possibly new) comport,
  in the same frame.
- `& 0x10`: go on to the next rule (the normal "fell off the end of the rule"
  result).
- anything else (0, 2, 4 alone): return 0. Stop evaluating rules for this
  frame, with nothing saved.

Schedule (reflex/"saved" stream) 00522930: the cursor at `context+4` is
advanced to the next node unless the control has 8 or 2. So in the schedule,
2 means "re-run this statement next frame". 1 means "stop here this frame,
resume after it".

Intelligence 005227c0: rules first. If they returned 1, or a meta action ran,
the schedule cursor is restored from its mode-0 slot and the schedule doesn't
run this frame. Otherwise the schedule runs. The loop repeats while 8 is set
and 1 is not.

What following statements return (seen in the decomp):

- Procedures: 0 normally. 1 from Proc_Camera_UpdatePosition 00545d80
  (`src/camera/camera_update.c:149`) and from Proc_KillPerso (226) on self.
  2 from a dialogue wait (`src/dialogue/dialogue_dispatch.c:142`). 8 from a
  comport change of the caller's own comport (005426f0 /
  00535360: `if changed && !suppress → control 8`).
- Meta actions (kind 5): the updater's value. 1 = still running (hold),
  4 = finished. For example, the waypoint updater 0054c800 returns 4 when the
  way is gone or the target is reached, otherwise 1.

## Policy values

P is an arbitrary byte, but only bits 0x01 and 0x02 change anything, plus
0x04/0x08/0x10 when they're added to S's bits. These are the meanings:

| P | Effect on following statement S | Script-visible meaning |
|---|---|---|
| 0 | clears S's hold bits 1 and 2 | S does not block. Execution continues to the next statement in the same frame. A camera update (normally 1) proceeds. A running meta action (1) becomes non-blocking: the rest of the rule runs this frame, and the slot stays active, so next frame the rule resumes at the meta node (skipping the rule's earlier statements) and the policy is restored from the slot and re-applied, until the action returns 4. Before a comport change (8), no effect, because 8 survives the mask. |
| 1 | forces bit 1 | "Hold here." Stop the node loop. In rules, record this rule so next frame starts at it, and skip the schedule this frame. In the schedule, resume after S next frame. A meta action behaves as without a policy while running. When it finishes as the last statement of its block it gives 5: free the slot, then hold. Before a self comport change, 8\|1 = 9: rule loop sees bit 1 first and returns 1, so the new comport is not run until next frame (no same-frame restart). Inferred quirk: `storage+7` is written after 005216e0 reset it, so next frame the new comport's rules start at the old rule's index. |
| 2 | forces bit 2 | "Stop for this frame." Stop the node loop. In rules, stop evaluating rules with nothing saved; the schedule still runs unless a meta action ran. In the schedule, re-run S next frame. Not seen in the decomp as a literal; meaning derived from the bit (inferred). |
| 3 | 1 and 2 | bit 1 is tested first in both loops, so 3 behaves like 1 (inferred). |
| 4 | adds bit 4 | free the currently selected meta slot after S, then continue (inferred from the bit; the selected slot is whichever 005224f0 last selected). |
| 8 | adds bit 8 | restart rule evaluation from rule 0 in the same frame. In the schedule, don't advance. Inferred. |
| 0x10 | adds bit 0x10 | end this rule and go to the next. Inferred. |
| any with 0x20 | ignored | the merge treats it as nothing pending. |

The values that actually occur in Manoir and Brigand are listed below. Only
those need porting.

There's no animation-end coupling in this mechanism. The policy only edits the
interpreter's control byte. Waiting for an animation or action to end exists
only because the following meta action keeps returning 1 until its own updater
returns 4. The policy decides whether that 1 blocks the rule (P = 1, or no
policy) or not (P = 0).

## Sites per level

Policy values seen: only 0 and 1, in Manoir, Brigand and the shared Fix
scripts. The values 2, 3, 4, 8 and 0x10 never occur, so their inferred
meanings above don't need porting yet.

Method:
- All-site counts come from a raw byte scan of the decompressed .sna blocks
  for 8-byte AI nodes `(param u32, 2 zero bytes, indent, kind)`. A site is a
  (4,6) node followed by a (12, P) node at indent+1.
- The scan script is kip-zephyr scratch `scan46.py`. It reuses the `sna()`
  parser from `output/sound-sidecar-probe.py` and is not in the repo.
- Stand-in counts come from
  `grep -rhoE "unsupported action-return policy [0-9]+" godot/generated/hype/levels/<level>`.
- The scan matches the stand-ins exactly (2 and 101). It also matches the
  309 Fix uses cited in `notes/ai-api-assessment.md:87`.
- The scan is a pattern match, not the converter's decoder. Level variants
  (manoir2..4, brigand2..4) were not scanned.

| Level | P | Sites | Handled by converter | Stand-in |
|---|---|---|---|---|
| Manoir | 0 | 0 | 0 | 0 |
| Manoir | 1 | 6 | 4 | 2 |
| Brigand | 0 | 28 | 28 | 0 |
| Brigand | 1 | 108 | 7 | 101 |
| Fix (shared) | 0 | 4 | 4 | 0 |
| Fix (shared) | 1 | 309 | 309 | 0 |

The "handled" column is total minus stand-ins, inferred from the rules at
`src/openspace/ai.rs:1001-1050`.

Follower S of each site, taken as the next node at the same indent:

| Level | P | S | Count | Converter |
|---|---|---|---|---|
| Manoir | 1 | 4-209 Proc_ChangeMyComport | 3 | change_behavior, pause_after |
| Manoir | 1 | 4-207 Proc_ChangeComport | 2 | stand-in |
| Manoir | 1 | 5-19 Action_IncrementalTurnPosition | 1 | dropped (meta holds anyway) |
| Brigand | 1 | 4-214 Proc_Turn | 100 | stand-in |
| Brigand | 1 | 4-207 Proc_ChangeComport | 1 | stand-in |
| Brigand | 1 | 4-209 | 4 | change_behavior, pause_after |
| Brigand | 1 | 5-23 Action_IncrementalTurnPerso | 2 | dropped |
| Brigand | 1 | 5-19 | 1 | dropped |
| Brigand | 0 | 5-23 | 22 | dropped (plain sequence) |
| Brigand | 0 | 5-19 | 6 | dropped (plain sequence) |
| Fix | 1 | 4-209 | 305 | change_behavior, pause_after |
| Fix | 1 | 5-23 | 4 | dropped |
| Fix | 0 | 4-135 Proc_Camera_UpdatePosition | 2 | CameraUpdatePosition pair (model 437) |
| Fix | 0 | 5-23 | 2 | dropped |

Stand-in files:
- Manoir: `ai/models/154.gd`, 2 sites.
- Brigand: `927.gd`, 50 sites; `1060.gd`, 50 sites; `284.gd`, 1 site.

Example sites:
- P=1, then 4-214 Proc_Turn:
  - brigand model 927 (iGardeCommun_Brigand_01), `normal_2_rule_0`, nodes
    188 and 193 (927.gd:390, :394). `turn(±600)` inside a "close to target"
    branch. The other sites in this rule are nodes 241, 246 and 267.
  - brigand model 1060 (iThievesCommun_Brigand_01..04), `normal_6_rule_0`,
    nodes 188, 193 and 241.
  - These come four to a rule (two ± pairs).
- P=1, then 4-207 Proc_ChangeComport (target main_actor):
  - manoir model 154 (IWorld_Manoir), `normal_1_schedule`, nodes 17 and 41
    (154.gd:649, :690).
  - brigand model 284 (IWorld_Brigand), `normal_0_schedule`, node 94
    (284.gd:207).
- P=1, then 4-209: block (17,1) offsets 0x1e48, 0x1f08 and 0x1f6c in
  manoir; 0x11264 and 0x11524 in brigand. Models not resolved.
- P=1, then a meta action: brigand 0x5c48 and 0x5da4 (5-23).
- P=0, then a meta action: brigand 0x1c5d8 and 0x1c6d0 (5-23, indent 4);
  0x6060 and 0x3c010 (5-19, indent 1).

What the uncompiled pairs do:

- **P=1, then Proc_Turn (4-214, 00542bb0).** The body isn't decompiled. Its
  disassembly (`reference/installed-profile/MaiDFXvr_bleu.exe`,
  0x542c53..0x542c66) calls `0052ef40(receiver, angle)`. If that returns 0,
  the procedure sets control 1; otherwise control stays 0. With P=1 the
  merged result is always 1. So the actor applies the turn, then holds:
  - Rule evaluation stops for this frame.
  - Next frame starts at this rule. This rule has no meta slot, so it runs
    from the top, and earlier rules are skipped.
  - The schedule (reflex) does not run this frame.

  In practice it's "turn a step this frame and yield". Without the policy,
  a successful turn would continue to the next statement.
- **P=1, then Proc_ChangeComport on another actor (4-207, 00542650; read
  from the disassembly, see `unsupported-ai-entries.md`).** When the target
  isn't the caller, suppress_control = 1, so the comport change sets no
  control 8. S returns 0, and P=1 makes it 1. The caller holds after
  switching the main actor's comport. These sites are all in schedules
  (IWorld_*), so the schedule resumes after S next frame.

## Godot porting notes

- The Brigand stand-ins need a rule-level "hold" result: stop the rules this
  frame, remember this rule as the next frame's start rule (source
  `storage+7`), and skip the reflex/schedule. Camera update and running meta
  actions give the same signal, so if the Godot rule runner already holds for
  them, P=1 plus Proc_Turn can emit that hold after the turn.
- The policy needs per-actor state only when S is a meta action. The pending
  byte must live in the meta-action slot (the source copies it in at
  00525bd0 and back out at 00525ea0), because a non-blocking (P = 0) meta
  action re-applies it every frame it resumes. For procedure followers it is
  resolved in the same statement, so it compiles away.
- P = 0 plus a meta action is a "start the action, keep going" pattern, but
  it's not a fire-and-forget coroutine. Each later frame the rule resumes at
  the action node, not at the rule's top, so conditions before the action are
  not re-tested until the action ends. A Godot port needs a per-rule "resume
  at action N" cursor, plus "run the remainder after it every frame".
- P = 1 plus a self comport change defers the new comport by one frame
  instead of restarting in the same frame. The inferred rule-index carry-over
  is worth a probe before anyone relies on it.
- P = 0 plus a comport change is a no-op (bit 8 survives). It can compile as
  a plain change.
- The policy attaches to the next statement-level node whatever its kind:
  a keyword (If/Else) node, a procedure or a meta action. The converter
  should resolve the follower by evaluation order, not just the next sibling
  at the same indent. Inferred: nested blocks under an If are walked by the
  same 00522650 loop, so the follower of a 4-6 that ends a block is the next
  node the If's skip lands on.
- None of it depends on animation end events. Whether an action "ends with
  its animation" belongs to each meta action's updater.

## Checklist lines

- [ ] Proc_SetActionReturn 0 (4-6, P=0): the following statement's hold bits are cleared. Camera update proceeds (compiled pair, Fix model 437). Before a meta action it's non-blocking, and the rule resumes at the action until it returns done (Brigand 28 sites, all before 5-23/5-19, currently compiled as plain sequence: verify the resume-at-action behavior).
- [ ] Proc_SetActionReturn 1 (4-6, P=1) before a self comport change (4-209/210): defer the new comport to next frame instead of a same-frame restart (Manoir 3, Brigand 4, Fix 305; compiled as pause_after). Probe the inferred rule-index carry-over.
- [ ] Proc_SetActionReturn 1 before a meta action (5-19, 5-23): same as no policy while running. When it ends as the last statement of its block, free the slot and hold (Manoir 1, Brigand 3, Fix 4; dropped by the converter).
- [x] Proc_SetActionReturn 1 before Proc_Turn (4-214): turn, then hold. The rule loop stops this frame, the next frame starts at this rule, and the schedule is skipped (Brigand 100 stand-ins in 927.gd and 1060.gd).
- [x] Proc_SetActionReturn 1 before Proc_ChangeComport on another actor (4-207): change it, then hold the schedule until next frame (Manoir 2 in 154.gd, Brigand 1 in 284.gd; stand-ins).
- [x] Proc_SetActionReturn 2, 3, 4, 8, 0x10: no sites in Manoir, Brigand or Fix. Meanings are inferred from the control bits only.
