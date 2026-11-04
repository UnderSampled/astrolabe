# Func_DistanceToPerso, the Ultra operator, and designer actor operands

Static research, 2026-11-04. Read-only on hype-decomp and the repo.

Sources: `objdump -d -M intel` on
`hype-decomp/reference/source-profile/MaiDFXvr_bleu.exe` (the PE the research
TSVs are keyed to), plus raw Ghidra where it exists. Raw Ghidra has no body
for 00537c20 (`research/coverage.tsv:5605`: "raw export lacks this
callable/body"), so the handler itself is read from disassembly. Ghidra
exists for the helpers: 00522610, 005223c0, 00525920, 00525900, 00524fa0,
00523a50, 00523a90, 005247e0, 0052ea00, 00438e00, 004600c0.

## Answer

- **What it computes.** Func_DistanceToPerso (3, 4) is the plain 3D
  Euclidean distance, in world space, between the translation part of the
  global matrix (superobject +0x24) of the *receiver* and of the P operand's
  actor: `sqrt(dx^2+dy^2+dz^2)`. No local frame, no axis selection (the
  selector passed to 0052ea00 is 7). Confidence: high (read from
  disassembly: 00537c20 -> 0052ea00(recv, target, 7) -> 0052e910 default
  case).
- **Who is "self".** The receiver is the executing perso, unless an Ultra
  receiver is pending. 00537c20 calls 00525920 with the function's Ultra flag
  (1 for this entry), so under `receiver.Func_DistanceToPerso(P)` the receiver
  replaces the executing perso as the first point. Confidence: high.
- **Ultra (kind 2, parameter 24).** Operator row 24 of the operator table is
  "Operator_Ultra", handler 00525950. It evaluates its first child to an
  actor, stores it in the global pending-receiver DAT_00721bd8 (cleared when
  it equals the caller), then evaluates the second child (the function node).
  Both children are evaluated with the *script owner* as the caller context.
  Confidence: high.
- **Whose designer variables.** A node-11 inside an Ultra reads the
  **owner's** designer variables, whether it is the Ultra's receiver child or
  the function's operand. P is evaluated with the original caller, not the
  receiver; the receiver is swapped in only for the position read. So
  `MainActor.Func_DistanceToPerso(dsg 11)` is the distance from Hype to the
  actor stored in the owner's variable 11. Confidence: high.
- **Operand evaluation.** P goes through 00522610 -> node kind 11 handler
  00521e00 -> 00523a50 -> type reader for DSG type 10 (00523780), producing an
  AiValue with tag 0xC (perso) and payload = the actor's Perso pointer.
  Confidence: high.
- **Null perso.** The type reader returns payload 0 (tag still 0xC) for a null
  or non-actor stored value. 00537c20 then dereferences `[payload+4]` with no
  null check, so the original **reads address 0x4 and faults**. It does not
  return zero, set an error flag, or skip. Whether any SEH handler turns that
  into a recovered state is not determinable statically (no handler seen in
  this call path). Whether a given instance's variable is null at the time of
  the call depends on initialisation and is not decided here. The decoded
  model defaults for 1060 var 11 and 779 vars 7/8 are `Actor: null`.
- **Port.** The distance formula is already faithful: 3D Euclidean on
  Godot global positions, receiver = owner or Ultra receiver, target = owner's
  dsg. The failure is in the decoder: (3, 4) only accepts `Expression::Actor`,
  but a node-11 child decodes to `Expression::Variable`, so every
  `Func_DistanceToPerso(dsg N)` becomes Unsupported, and under Ultra it
  surfaces as "Ultra operation is unsupported for this receiver". The
  faithful change is to accept `Expression::Variable` in (3, 4) (and then the
  Ultra rewrite works unchanged).

## 1. Func_DistanceToPerso at 00537c20

### Identification

`research/hype-ai-api.tsv:135`: `function 4 Func_DistanceToPerso P 00537c20`.
The function table has stride 0x38 bytes per row: 44-byte name, then the
handler pointer (table base `0x6002d4` + 0x38*id), a signature dword (0x50 =
'P'), and a flags dword. For this row the flags dword is `00 58 00 01`, so the
byte read by `fn_ucGetFunctionUltraOperator` (`reference/ghidra/functions/
00527180_fn_ucGetFunctionUltraOperator.c`: `(&DAT_006002df)[id*0x38]`) is
**1**: the function honours an Ultra receiver.

### The handler (disassembly, cdecl `(caller, cursor, result)`)

```
537c2b  mov ecx,[ebx-8]          ; function id of this node
537c35  call 527180              ; ultra flag for the id
537c3e  call 525920              ; 525920(flag, caller, &recv)  -> recv
537c4d  call 522610              ; evaluate P with (edi = ORIGINAL caller, cursor, result)
537c59  mov eax,[edx+4]          ; edx = P payload (perso pointer)
537c5d  call 438e00              ; *(x+0xc)
537c6d  call 52ea00              ; 52ea00(recv, 438e00(P->+4), 7)
537c72  fcom 0.0 / fchs          ; value <= 0 is negated (0 becomes -0.0)
537c84  fstp [result]; [result+0xc] = 8   ; real
```

- 00525920 (`00525920_FUN_00525920.c`):
  ```c
  *param_3 = param_2;
  if (((DAT_00721bd8 != 0) && (DAT_00721bd8 != param_2)) && (param_1 != '\0')) {
    *param_3 = DAT_00721bd8;  DAT_00721bd8 = 0;
  }
  ```
  So the receiver is the caller, unless a pending Ultra receiver exists, is
  not the caller, and the flag is set.
- 00522610 is called with `edi`, which was loaded from the first stack
  argument at 537c27 *before* the receiver was written back. So P is evaluated
  in the original caller's context.
- 00438e00 (`00438e00_FUN_00438e00.c`): `return *(param_1 + 0xc)`. Applied to
  `[P+4]` (Perso -> standard-game struct) it yields the actor's SuperObject.
  The receiver (caller or pending value) is already a SuperObject: 0052ea00
  uses `param_1+0x24` directly, and `521e00` reaches the caller's Perso via
  `[caller+4]`.
- 0052ea00 (`0052ea00_FUN_0052ea00.c`):
  ```c
  if ((param_3 & 0x80) == 0) {
    POS_fn_vGetTranslationVector(*(param_1 + 0x24), &a);
    POS_fn_vGetTranslationVector(*(param_2 + 0x24), &b);
  }
  d = a - b;
  bVar1 = param_3 & 7;
  if (bVar1 != 7) { ... transform d into param_2's local frame ... }
  FUN_0052e910(&d, param_3);
  ```
  Selector 7: bit 0x80 clear, so the global matrices (SuperObject +0x24) are
  used; `&7 == 7`, so there is no local-frame transform.
- 0052e910 (disassembly 52e910..): `ecx = (sel & 0xff) - 1; if (ecx > 5)
  default`. Selectors 1..6 pick x, y, z, xy, xz, yz; the default (selector 7)
  computes `fsqrt(x^2 + y^2 + z^2)` (the 52e9aa block). So the result is the
  full 3D distance.
- The sign step (`fcom 0.0`, `test ah,0x41`, `fchs`) only flips values <= 0;
  a distance of exactly 0 becomes -0.0, otherwise no effect.

The sibling DistanceX/Y (e.g. 00537ca0, selector 0x81) are not this function;
the port already maps (3, 8) and (3, 7) separately.

### Null perso

The designer-variable Perso reader is the table row for DSG type 10
(`005247e0_FUN_005247e0.c`: `(&PTR_LAB_005fdcc4)[type*9]`, stride 36; the
type-to-reader association is shifted by one row relative to the name
column, so type 10 = reader 00523780, which writes tag 0xC):

```
523780  edx = *stored            ; stored word = SuperObject*
        if edx == 0            -> payload 0, tag 0xC, return false
        if *(edx) != 4         -> payload 0, tag 0xC, return false   ; not an actor superobject
        else                   -> payload = edx->+4 (Perso*), tag 0xC, return true
```

The boolean return is discarded: `00523a90_FUN_00523a90.c` is `void` and calls
the reader, and 00523a50 / 00521e00 ignore it. 00537c20 never tests the
payload. With payload 0 it executes `mov eax,[edx+4]` on null, an access
violation at address 0x4. No error flag, zero or skip exists on this path
(the 00537c20 body above has no branch before the dereference). Every other
callee on the path (00438e00, 0052ea00) also dereferences without a check.
Statically undeterminable: whether a SEH frame higher up (not in this call
chain as read) recovers it.

### Evaluator path and value tag for a designer-variable operand

- 00522610 (`00522610_FUN_00522610.c`) dispatches on the node's kind byte:
  `(&PTR_FUN_005fd088)[*(byte *)(param_2 + 7)](param_1, param_2, param_3)`.
  Table entries read from the PE: kind 0 -> 005222d0 (keyword), 1 -> 00522300
  (condition), 2 -> 00522380 (operator), 3 -> 00522350 (function), 4 ->
  005223c0 (procedure, `005223c0_FUN_005223c0.c`), 5 -> 005224f0 (meta
  action), 11 -> 00521e00 (designer variable).
- Kind 11 is 00521e00 (`00521e00_FUN_00521e00.c`; disassembly):
  `mind = *(*(caller+4)+0x10)`, i.e. `004600c0(...)`;
  `00523a50(dsg_id_byte, 0, mind, result)`; cursor += 8.
- 00523a50 (`00523a50_FUN_00523a50.c`) indexes the DSG info table at
  `**(mind+0xc)->+4` by id*0x10 and the DSG memory at `(mind+0xc)->+8`, then
  calls the type reader through `005247e0`.
- Type 10 (Perso/Actor in this model's DSG table) -> AiValue
  `{payload = Perso*, tag = 0xC}`. This matches the tag table in
  `unsupported-ai-entries.md` ("0xc: perso").
- The caller context used for the designer lookup is the first argument of
  00522610: the owner (see Q2).

## 2. The Ultra operator (kind 2, parameter 24)

- Kind 2 dispatches to 00522380 (`52238d: call 527110`;
  `00527110_FUN_00527110.c`: `(&PTR_LAB_005fff44)[param_1*8]`), which indexes
  the operator table (stride 32 bytes, base 0x5fff28, handler at +0x1c) by the
  node's parameter. Row 24's name is "Operator_Ultra" and its handler is
  **00525950**. Other rows seen: 0 Operator_Plus, 1 Minus, 2 Mul, 3 Div, 12
  Dot, 16..20 Vector ops, 25 Operator_ModelCast, 26 Operator_Array.
- 00525950 (disassembly, `(caller=esi, cursor, result)`):
  ```
  recv_ptr = 524fa0(caller, cursor, &recv)      ; evaluate child 1
  525900(caller, recv)                           ; DAT_00721bd8 = (caller != recv) ? recv : 0
  return 522610(caller, next_cursor, result)    ; evaluate child 2 with the same caller
  ```
- 00524fa0 (`00524fa0_FUN_00524fa0.c`):
  ```c
  uVar1 = FUN_00522610(param_1,param_2,local_10);        // child 1, caller = owner
  uVar2 = FUN_00438e00(*(undefined4 *)(local_10[0] + 4)); // Perso -> SuperObject
  *param_3 = uVar2;
  ```
- 00525900 (`00525900_FUN_00525900.c`):
  `DAT_00721bd8 = -(uint)(param_1 != param_2) & param_2;` (pending is 0 when
  the receiver equals the caller).
- Child 2 (kind 3 -> 00522350 -> `005271a0` handler) is Func_DistanceToPerso.
  Its 00525920 consumes DAT_00721bd8, because its Ultra flag is 1, and clears
  it.

Consequences:
- The receiver `MainActor` (kind 0, param 5) is evaluated by 00522610 with the
  owner as caller, giving the Hype perso; its SuperObject becomes the pending
  receiver.
- The function then measures from that SuperObject: `52ea00(recv = Hype's SO,
  target, 7)`.
- Both children of the Ultra are evaluated with the **owner** as the caller.
  The designer-variable handler 00521e00 derives the DSG memory from its
  caller argument (`caller->Perso->+0x10`), so a node 11 reads the owner's
  variable, both as an Ultra receiver and as the function's P operand. The
  receiver's own designer variables are never read by this path. If the
  receiver and caller are the same actor, pending stays 0 and the result is
  identical to the plain call.
- Caveat: the pending slot is a single global that is consumed only by a
  function whose Ultra flag is set. If the second child does not consume it
  (flag 0, or a non-function node), it stays set until the next consumer. That
  is not an issue for this function; it is noted because
  `unsupported-ai-entries.md` lists 3-153 as consuming the pending receiver
  without using it.

So for brigand 1060: `MainActor.Func_DistanceToPerso(dsg 11)` = distance from
Hype's global position to the position of the actor in the **thief's own**
designer variable 11.

## 3. Port comparison

| Link | Source | Port | Status |
|---|---|---|---|
| Distance points (global-matrix translation of both actors) | 0052ea00 selector 7 | `character().global_position.distance_to(target.global_position)` (`src/godot/ai.rs:1270`), `receiver.global_position.distance_to(target.global_position)` (`src/godot/ai.rs:1445`), `distance_to_main_actor()` (`godot/addons/astrolabe/behavior/behavior.gd:270`) | Implemented, equivalent: 3D Euclidean on world positions. Godot `global_position` is the source position with an axis remap that preserves length (`character.gd:925` `source_position()`: `(x, -z, y)`; no scale). |
| Self = executing perso | 00525920 (caller) | `character()` | Implemented |
| Ultra receiver replaces self | 00525950 + 00525920 | decode (2, 24) -> `ActorDistanceFrom { receiver, target }` (`src/openspace/ai.rs:1635`, `:1651`), `actor_target_code` | Implemented for `ActorDistance` and `MainActorDistance` children |
| Receiver / operand from the owner's designer variables | 00524fa0, 00521e00 (caller = owner) | `ActorTarget::Variable(n)` -> `dsg_n` of the script owner (`src/godot/ai.rs:914`) | Implemented for the receiver. The target operand: see below |
| (3, 4) operand = designer variable (node 11) | 00522610 -> 00521e00 | `src/openspace/ai.rs:1795-1808` requires `Expression::Actor`, but node 11 decodes to `Expression::Variable` (`:1366`) | **Lacks.** Produces "distance requires an unsupported actor expression"; under Ultra the child is Unsupported and the match falls to "Ultra operation is unsupported for this receiver" (`:1677-1684`) |
| Null target | unguarded dereference, fault | `typeof != TYPE_OBJECT or not is_instance_valid ... -> fail_unsupported` (`src/godot/ai.rs:1263`, `:1450`) | Stand-in: the port fails with a diagnostic instead of faulting. It is not a zero return. Mismatch only in outcome form; reaching it means the script or data diverged from the original |
| Sign step (-0.0 for exactly 0) | fchs when <= 0 | none | Immaterial |

Other (3, N) forms are separate source functions and are not covered here:
(3, 7) and (3, 8) use local-frame selectors, as already commented at
`src/openspace/ai.rs:1810-1827`.

### Port implications

1. **Decoder (the actual defect).** In `src/openspace/ai.rs` (3, 4) at
   `:1795-1808`, accept a kind-11 operand: when the child decodes to
   `Expression::Variable(n)`, produce
   `Expression::ActorDistance(Box::new(Expression::Variable(n)))` (and, as for
   `change_actor_behavior` at `:1255`, treat it as an actor-typed variable).
   Only `ActorTarget::Keyword(MainActor)` should keep the
   `MainActorDistance` shortcut. With that, the Ultra rewrite at `:1635`
   already yields `ActorDistanceFrom { receiver: MainActor, target:
   Variable(11) }`, and 779's `Func_DistanceToPerso(dsg 7/8)` becomes
   `ActorDistance(Variable(7/8))`.
2. **Codegen.** `Expression::ActorDistance` / `ActorDistanceFrom` already
   guard the null/invalid target and evaluate the target through
   `guarded_expression` in the owner's context, matching the original
   evaluation order (receiver first, then the operand, both against the
   owner). No change needed beyond verifying that `Expression::Variable`
   lowers to `dsg_N` there.
3. **Encoder side.** The `(3, 4)` re-encode (`src/openspace/ai.rs:3121`) takes
   the target through `self.expression`, so a Variable target round-trips as
   kind 11 with no change.
4. **Null handling.** Keep the fail-with-diagnostic guard. If a live run shows
   the guard firing for 779 or 1060, the cause is the designer variable being
   unset when the original would have faulted (or never reached the node), so
   the fix would be in whatever sets that variable (a different link), not in
   the distance.

### Not determined statically

- Whether the original has an SEH handler that would catch the null
  dereference (none seen on this path).
- The runtime value of dsg 7/8 (779) and dsg 11 (1060) when the node runs.
  The model defaults are `Actor: null`, and per-instance initial values were
  not examined.
- The port-side lowering of `Expression::Variable` to a typed Actor
  (confirm in codegen that DSG type-10 variables are `Node3D`/null-safe).
