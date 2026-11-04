# Way operands: DSG type 9 and the squirrel scripts (Manoir 816, Brigand 1862)

Static research, 2026-11-04. Read-only on hype-decomp and the repo. Sources:
`hype-decomp/reference/source-profile/MaiDFXvr_bleu.exe` (objdump -d -M intel)
where there is no Ghidra body, Ghidra functions under `reference/ghidra/functions/`,
`src/ai/designer_variable_storage.c`, `src/ai/waypoint_meta_wrappers.c`,
`src/ai/ai_value_access.c`, and the port's decoded dumps
(`~/.hermes/cache/scratch/ai-dumps/816.json`, `1862.json`) plus `astrolabe inspect`
on the Rete roots `rete-manoir` and `rete-now`.

## Answer

- **DSG type 9 is `Way`.** It is one byte in memory: an index into the actor's own
  way list. The type table (26 entries, 36-byte stride at 0x5fdcc0) names type
  9 "Way"; `fn_ulSizeOfDsgVar` 005236a0 gives it size 1; its reader (523860) copies
  that one byte into an AiValue with tag 0xB. The port's other ids match the same
  table: 8 WayPoint (4-byte pointer, tag 0xA), 10 Perso (4 bytes, tag 0xC),
  11 List (count*4+4), 23 WayPointArray (count*12+8).
- **A `Y` operand is evaluated by the ordinary operand dispatcher (00522610).**
  Node kind 23 (handler 522040) is a literal: it stores the node's first byte and
  tag 0xB. Node kind 11 on a type 9 variable (handler 521e00 -> 523a50 -> reader
  523860) produces the identical value: one byte, tag 0xB. The consumer 00542ff0
  never looks at the tag or at which node kind produced the value. It passes the
  word to 0043e370, which indexes the **calling actor's own way list** (`perso+4`
  -> `+0x20` -> linked list; element n, node `+0xC` is the way). So kind 23 and a
  type 9 designer variable are interchangeable.
- **Designer byte 0 is "the actor's way 0", in the same index space as kind-23
  operands.** Each squirrel actor owns exactly one way, so index 0 is its only
  way. The decoder can resolve variable 10 as `ResetRoute(0)` / `GoOverRoute(0)`
  with no new runtime concept, provided it reads the variable's default and
  confirms the variable is never reassigned (no `Assign` to variable 10 in 816 or
  1862).
- Confidence: high for type 9 = Way, size 1, tag 0xB, and for the shared evaluation
  path (read from the disassembly and the type table). High that byte 0 resolves
  into the same list as kind 23 (same lookup function, same list the port already
  imports). Medium-high for "value stays 0 at run time": the importer holds 0 for
  both current and initial values, and no node in the squirrel scripts writes
  variable 10, but the original could still change it through a path not visible
  in these two models.

## Evidence

### 1. The DSG type table

`fn_eGetNbDsgVarType` 005247d0 returns 0x1a (26 types). Accessors
`005247e0/5247f0/524800` index `type*36 + 0x5fdcc4/0x5fdcc8/0x5fdcc0`
(`src/ai/ai_value_access.c`, `designer_variable_storage.c`: value-kind,
reader and writer slots). Each 36-byte entry is
`{name[24], tag, reader, writer}`: the name starts 24 bytes before the tag, so the
name that sits just before entry n's fields at `0x5fdcc0 + 36n` belongs to entry n
(`Boolean` is at 0x5fdca8, its tag/reader/writer at 0x5fdcc0). Reading the table
bytes from the exe:

| type | name | tag | size (005236a0) |
|---|---|---|---|
| 0 | Boolean | 1 | 1 |
| 1 | _128To127 | 2 | 1 |
| 2 | 0To255 | 3 | 1 |
| 3 | _32768To32767 | 4 | 2 |
| 4 | 0To65535 | 5 | 2 |
| 5 | Integer | 6 | 4 |
| 6 | PositiveInteger | 7 | 4 |
| 7 | Float | 8 | 4 |
| 8 | WayPoint | 0xA | 4 |
| **9** | **Way** | **0xB** | **1** |
| 10 | Perso | 0xC | 4 |
| 11 | List | 0xD | count*4+4 |
| 12 | Vector | 0x16 | 12 |
| 13 | Comport | 0x23 | 4 |
| 14 | Action | 0x19 | 4 |
| 15 | Text | 0x25 | 4 |
| 16 | GameMaterial | 0x27 | 4 |
| 17 | Capability | 0x2B | 4 |
| 18 | Graph | 0x2C | 4 |
| 19 | PersoArray | 0x2A | count*12+8 |
| 20 | VectorArray | 0x2A | count*12+8 |
| 21 | FloatArray | 0x2A | count*12+8 |
| 22 | IntegerArray | 0x2A | count*12+8 |
| 23 | WayPointArray | 0x2A | count*12+8 |
| 24 | TextArray | 0x2A | count*12+8 |
| 25 | ActionArray | 0x2A | count*12+8 |

The tag column is the entry's first field (tag 0x2B etc. are inferred from the
table bytes; I did not check them past the entries cited above). The cross-checks
that fix the alignment: tag 8 = Float matches "8: real" in
`unsupported-ai-entries.md`, tag 0xC = Perso matches "0xC: perso", and the size
switch gives pointers (4) to WayPoint/Perso and the array formula to ids 19-25.
Size switch quoted from `designer_variable_storage.c`:
`case 0: case 1: case 2: case 9: return 1;` (the 005236a0 disassembly agrees).

Reader for type 9, 523860:

```
mov cl, BYTE PTR [eax]          ; storage byte
mov BYTE PTR [result], cl       ; payload (only the low byte is written)
mov DWORD PTR [result+0xc], 0xb ; tag 0xB
```

Writer 523be0 copies one byte. The WayPoint reader (523840) writes a 4-byte
pointer and tag 0xA. So the importer's `Opaque{kind 9, bytes [0]}` has the exact
correct extent: one byte.

### 2. Evaluation of a way operand and its consumers

Evaluator table `0x5fd088` (indexed by the node's kind byte at +7):

- Kind 23 -> 522040: `mov dl,[node]; mov [res+0xc],0xB; mov [res],dl; add node,8`.
  A byte literal with tag 0xB. In `src/ai/ai_leaf_values.c` this body is
  "rewritten static" for the missing-raw entry.
- Kind 11 -> 521e00 (`ai_expression_dispatch.c`): `523a50(var, 0, context, result)`.
  523a50 loads the descriptor at `model+4 + var*16` (offset at +0, type at +4), adds
  `instance+8` (current value storage), and calls the per-type reader
  (`ai_read_typed_storage_00523a90`). For type 9 this is 523860.

Procedure `Proc_ReInitWay` etc. share entry 00542ff0 (no Ghidra body; disassembled):

```
call 0x522610            ; evaluate operand 1 -> local AiValue (this is the Y operand)
call 0x43e370(caller, value.word)  ; esi = way
switch (proc id):  0xDC (220) -> 518760(caller, way)    WP_fnv_Way_InitWay
                   0xDD (221) -> 5187c0(caller, way)    WP_fnv_Way_InitWayBack
                   0xDE (222) -> eval operand 2 (W), scan way's waypoints
                                  (518f70 size, 518fd0 element) for it,
                                  518840(caller, way, index)  InitWayWithWp
```

- 0043e370: `eax=[caller+4]; eax=[eax+0x20]; if 0 return 0; 43e340(eax, id)`.
- 0043e340: `if *list; node = 459a40(*list, id); return node ? 459250(node)` and
  459250 returns `node+0xC`.
- 00459a40 walks `*list` `id` times along the `next` pointer
  (`cx < dx` word compare, so only the low 16 bits of the argument matter). So the
  index is a **position in the actor's way list**; the object is the list node's
  `+0xC` pointer, which the port reads as `WayBindingWire.route`
  (`src/openspace/level/records.rs` 703; list walk in
  `level/way.rs::read_actor_ways`: perso+32 -> component -> `+0` header -> first
  entry at `+0`, route at `+12`, progress at `+16..19`). This is the same list. It
  is not a global graph list.
- InitWay (518760) clears per-binding progress: `43e3c0(...,0)` sets binding+0x10
  to 0, `43e440(...,0xff)` sets +0x12 to 0xFF, `43e400(...,0)` sets +0x11 to 0,
  `43e480(...,0xff)` sets +0x13 to 0xFF, then copies +0x10 to `way+9` and +0x12 to
  `way+10`. The port's `source_reset_route_progress` zeroes points/samples and sets
  previous to -1/-1, which is the same effect on the four progress bytes
  (`character.gd` 525-529). The `way+9`/`+10` mirror is not modelled in the port.
- InitWayBack (5187c0, 221) starts at the last point (size-1) and last sample of
  the last link; InitWayWithWp (518840, 222) starts at a given waypoint index. The
  decoder only handles 220 (`ai.rs` `(4, 220)`); 221 and 222 are not decoded.

`Action_GoOverWay` (27), `GoOverWayBack` and `GoToNextWPOfWay` (30) share the
initializer 0054b260 -> 0054b200 (`waypoint_meta_wrappers.c`):

```c
next = eval(operand, &v);
way_id = v.kind == REAL ? ftol(v.real) : v.word;   // tag-agnostic
way = FUN_0043e370(caller, way_id);                 // same actor-way-list lookup
if (way != 0 && way[8] != 0) FUN_0052e1d0(0, caller, way);  // way[8] = point count
```

0052e1d0 stores the way into slot 0 of the caller's continuation storage.
Updaters: 0054c8d0 (27) is `54c800(caller, continue=1, dir=0)`, 0054c910 (30) is
`54c800(caller, 0, 0)` (disassembled; 54c8f0 is 1,1 for the back variant). 54c800:
fetch the way with 52e240(0,caller); if none return 4; get next point and radius,
move with `54a960`, and when the control has bit 4 (arrived) test
`_WP_fnuc_Way_CurrentVertexReached_` (`dir=0`) or `...ReachedBack_` (`dir=1`) with
the continue word, returning 1 (finished) only when it succeeds. So 27 loops the
way and 30 steps to the next waypoint. Both use the identical way-index lookup, so
a type 9 designer operand and a kind-23 operand drive them the same way.

A quirk worth knowing, not a blocker: the consumer builds the AiValue on the stack
without zeroing and both producers write only the low byte, while 459a40 compares
16-bit values. Higher bytes are whatever the stack holds in the original; the
intended value is the byte. The port should use the byte.

### 3. Which ways the squirrel actors own, and variable 10

`astrolabe inspect <root> --kind perso --asset <perso>`:

- Manoir: `level:/persos/world/0/146` "iEcureuil_Manoir1", `family level:/families/36`,
  `brain.model level:/ai/models/816`, `routes` has **one** entry,
  `level:/routes/17-1/1496` (6 points, progress `[0,0,255,255]`). Its variable 10 is
  `Opaque{kind 9, bytes [0]}` in both `variables` and `initial_variables`.
- Brigand: `level:/persos/world/0/179` "iEcureuil_Brigand1" has one route,
  `level:/routes/17-1/2176`, variable 10 `Opaque{9,[0]}`. `…/180` (a second squirrel,
  "iEcureuil2_Birgand1") has one route `…/2199` and the same variable 10; I did not
  decode its model.

So byte 0 selects each squirrel's only way, and way slot 0 is already built by
`src/godot/character.rs` (~793) as `source_routes[0]` with matching progress arrays.
The scripts never write variable 10: no `Assign` with `variable: 10` in either
decoded model.

### 4. Every node in 816 and 1862 that the decoder cannot yet type

Both dumps decode identically (models differ only in family ids and model path).
Behaviors: 0..4, `reflex` null, one rule in behavior 1. Failures, by the dump's
`Unsupported` fragments (category = node kind column in the dump's JSON; I give
`category-id`):

1. **Behavior 0 schedule, node 0**: `4-220 ReInitWay` with `11 param 10`
   (`route call lacks local slot operand`). The rest of that script is fine
   (4-218 with an Action operand family 36/47 state 6, 4-209 -> behavior 1). The
   fragment spans nodes 0-5.
2. **Behavior 1 schedule, node 0**: `5-27 Action_GoOverWay` with `11 param 10`
   (same decode error). This is the same operand, the way variable.
3. **Behavior 1 rule condition, two sites**: `3-4 Func_DistanceToPerso` with
   `11 param 0` (start 13) and `11 param 1` (start 17). The decoder gives
   `distance requires an unsupported actor expression` because the `(3, 4)` arm
   (`src/openspace/ai.rs` ~1796) accepts only `Expression::Actor(..)`, while a
   kind-11 operand decodes to `Expression::Variable(n)` (~1366). Variables 0 and 1
   are type 10 Perso (`Actor`) in the model, so this needs the variable-to-actor
   conversion that other arms already do (e.g. ~403 `(11, variable) =>
   ActorTarget::Variable`).
4. **Behavior 1 rule, then-branch**: `5-25 Action_GoToWP` with `11 param 5`
   (waypoint variable 5, type 8): `unsupported AI statement category 5 operation 25 at
   node 23`. Already listed in `unsupported-ai-entries.md` (5-25, yielding a
   stand-in); the entry's initializer is 00522a50 with updater 0054b110, which I did
   not read here.

Nothing else in 816/1862 is untyped. Behaviors 2 and 4 already decode into
typed statements (`MoveLift`, `ChangeAction`, `DistanceToWaypoint`,
`SetSelfAbsolutePosition`, `VectorComponentAssign`, `ChangeBehavior`); behavior 3 has no
schedule. The remaining decoded items in behavior 1 (`MainActorDistance`,
`CollidePersoZdeNoWithTypeZde`, `IncrementalTurnPosition`, `WaypointValuePosition`)
are typed, so after the way operand and the two actor-distance sites are fixed,
5-25 is the only step left in these two models.

## Port implications

Links and status:

| Link | Source | Port status |
|---|---|---|
| Type 9 = Way, 1 byte, tag 0xB | 005236a0, 523860 | **Lacks**: `DesignerValueCodec::supports` rejects it, so it is kept as `Opaque{9,[0]}` (extent is correct) |
| Kind-23 way literal | 522040 | **Implements** (`ai.rs` ~450, encoder ~2319 `self.node(23, slot)`) |
| Kind-11 var of type 9 as a way operand | 521e00 + 523860 | **Lacks** |
| Way list lookup by index | 0043e370 / 43e340 / 459a40 | **Stands in**: `source_routes[slot]` built from the same list (`way.rs::read_actor_ways`, `character.rs` ~793); out-of-range yields `fail_unsupported` rather than an invalid way |
| InitWay progress reset (220) | 518760 | **Implements** (`source_reset_route`), without the `way+9/+10` mirror |
| 221 ReInitWayBack, 222 InitWayWithWp | 5187c0, 518840 | **Lacks** (not decoded) |
| GoOverWay (27) / GoToNextWP (30) lookup + movement | 0054b260, 54c8d0, 54c910, 54c800 | Lookup shares the slot model; the continue/arrival semantics are the port's native route stand-in (`source_begin_route`, `source_route_finished`); I did not compare the arrival rules against 54c800 |

Minimal faithful change:

1. In `DesignerValueCodec` (`src/openspace/ai_values.rs`), support kind 9 as a
   one-byte way index, e.g. a new `DesignerValue::Way(u8)`, kind 9, extent 1 (the
   importer already reads exactly one byte from the next-declaration bound;
   `level/ai.rs` ~725 opaque path would then not apply). Keep the Rete shape and
   Godot export (`godot/ai.rs` ~732, ~816) consistent with the new variant.
2. In the decoder (`ai.rs` ~450), accept for the `Y` operand either a kind-23 node
   (slot = parameter) or a kind-11 node whose model variable has type 9; treat the
   variable's value as a slot. Both resolve to the same `Statement::ResetRoute/GoOverRoute/GoToNextWaypoint(slot)`
   only if the variable is never written; otherwise add a variable-sourced slot
   (`ResetRoute(WaySlot::Variable(i))`) so codegen reads the variable at run time.
   Since the encoder must round-trip the source node, keeping the node kind is
   necessary: a constant-slot statement that was kind 11 on input would re-encode as
   kind 23 and change the script bytes. A variable-sourced slot avoids that.
3. Codegen (`godot/ai.rs` ~3281-3320) then passes the variable's value to
   `source_reset_route` / `source_begin_route`; slot 0 resolves to the existing
   single `source_routes[0]` for both squirrels.
4. Also needed for these two models to decode end to end: allow `Expression::Variable`
   (type 10 Actor) in the `(3, 4)` arm, and the 5-25 `Action_GoToWP` stand-in/implementation.
   These are separate from the way operand.

Still unknown or not determinable statically:

- Whether any other model writes a type 9 variable at run time (the `Assign`
  path for type 9 is the writer 523be0, one byte). In 816/1862 nothing does.
- The AiValue tag for a WayPoint-typed *node* (not a DSG read): the existing note
  lists tag 0x21 for waypoints, but the type-8 DSG reader writes 0xA. I did not
  trace where 0x21 comes from.
- Whether GoOverWay's arrival and speed logic (54c800 and callees 54a960, 44d700,
  WP_fnx_Way_GetCurrentSpeed) matches the port's native route movement; not
  examined here.

## Addendum 2026-11-04: ported, and what still blocks the squirrels

The importer now reads DSG type 9 as `DesignerValue::Way(u8)` and the decoder
accepts a kind-11 Way variable as a `RouteSlot::Variable` for 4-220, 5-27 and
5-30; codegen passes `int(dsg_N)` to the route calls. 816 and 1862 now type
whole. A headless probe (original save in Manoir, travel save in Brigand, 40 s
each) shows the squirrels still don't move: `source_begin_route` refuses the
route, so GoOverWay reports the guard "requires supported native route slot".

Both squirrel routes use link connection type 2 on every link (Manoir
`level:/routes/17-1/1496`, 5 links; Brigand `level:/routes/17-1/2176`, 4
links). The importer keeps that geometry as a placeholder
(`src/model/route.rs`, `RouteLink.geometry`), and the Godot route marks
non-StaticLine links unsupported (`src/godot/route.rs`). In the original,
00526cc0 WP_fnv_Link_GetCurrentPoint samples type 2 through FUN_00530cc0,
with the sample count from FUN_00530cb0 (hype-decomp
`src/waypoint/link_sampling.c`). Porting that is the curved-link work of
stand-in-priorities item 3, not part of the way operand.

After that, behavior 1's then-branch still has the documented 5-25
Action_GoToWP stand-in (node 23).
