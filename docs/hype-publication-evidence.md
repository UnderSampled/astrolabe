# Hype completed-update publication evidence

This note qualifies the smallest original-game observation that can be copied
immediately after the pinned Hype gameplay callback returns. It is a static
source review, not proof that the runtime wrapper reaches the intended return or
that a captured observation came from a completed update.

## Result

For the first comparison, publish **the controlled actor's current action only**
from mutable gameplay state. Read the main actor's 3d data pointer and its
current-action pointer at `data3d+4`, resolve that pointer to the ordered family
member, and copy it inside the post-return wrapper before the main loop advances.
Tag the copy with the prepared input ordinal supplied by the controller.

Keep the following fields out of the completed-update claim:

- The global transform reached through the controlled superobject at `SO+36`
  remains unavailable. The pointer names one matrix, but animation, hierarchy,
  mechanics and helper paths can mutate the aliased matrix. This bounded review
  did not close the complete writer set or exclude every later/asynchronous alias.
- `data3d+0` is the initial/default action. It is useful static or named-fixture
  metadata, not an outcome of the just-completed update.
- Counter `0x00754aa8` and the actor update stamp are guards/provenance only. Both
  are written before actor work and cannot certify completion.
- Transform plus action must not be advertised as one completed tuple while the
  transform writer set is unresolved.

## Layout corroboration

Raymap is the first practical reference. Its `Perso3dData` reader names the first
three pointers initial, current and secondary state in that order, and its
`SuperObject` reader distinguishes the matrix pointers at offsets `0x20` and
`0x24`. This corroborates the selected offsets, but the Hype decompilation below
is authoritative for the pinned build and for runtime writes.

Hype's accessors make the action offsets explicit:

- `FUN_004424d0` reads `data3d+0`, and `FUN_004424e0` writes it.
- `FUN_004424f0` reads `data3d+4`, and `FUN_00442500` writes it.
- `FUN_00442510`/`FUN_00442520` read/write `data3d+8`.

Sources: `reference/raymap/Assets/Scripts/OpenSpace/Object/Properties/Perso3dData.cs:26`,
`reference/raymap/Assets/Scripts/OpenSpace/Object/SuperObject.cs:163`, and pinned
decompilation functions `004424d0`, `004424e0`, `004424f0`, `00442500`,
`00442510`, and `00442520`.

## Current-action writers and update order

The decompilation export has three direct call sites for the exact current-action
setter `FUN_00442500`:

1. `FUN_00433e90` advances animation/action state and writes the selected state at
   line 144. In the same transition branch it finishes matrix work first at
   lines 110–129.
2. `FUN_004355b0` is the forced/action-change path. Lines 19–23 reset action
   bookkeeping and write current and secondary action together.
3. `FUN_00441e50` initializes a newly created 3d-data record to null. It is a
   construction path, not the ordinary per-frame writer.

The callers of the forced/action-change path include reset and gameplay action
machinery (`FUN_00442040`, `FUN_004496d0`, and callers reached from AI). These
calls are ordinary synchronous calls; this review found no worker publication or
deferred action-write queue.

For an eligible perso, `FUN_004499b0` performs the ordinary update. It calls the
animation/action updater `FUN_00433e90` at lines 53–66, then `FUN_004442e0` at
line 67. `FUN_004442e0` synchronously invokes related-actor AI, current-actor AI,
mechanics and contact follow-up at lines 13–22. Action changes requested by those
paths complete before those calls return. The rest of `FUN_004499b0` performs
same-actor follow-up and returns at line 95. An actor skipped by its guards retains
the previous completed action; that is a valid completed observation, not proof
that it changed on every input poll.

The main loop `FUN_004049d0` calls, in order, the gameplay callback through
`PTR_FUN_005e434c`, the graphics-end callback through `PTR_FUN_005e4350`,
`GLI_vEndOfTurnEngine`, and the input callback through `PTR_FUN_005e4344`
(lines 25–30). The verified pointers for the pinned executable are respectively
`0x0044a1e0`, `0x0044a670`, and `0x0044dbd0` for gameplay, graphics end, and
input. The gameplay callback increments the update counter through
`FUN_00426110` and runs actor traversal before returning. The graphics-end
callback leads to buffer swap; the later input callback samples the controls for
the following gameplay update. No current-action setter is present in this
post-gameplay tail.

Therefore, a wrapper that copies the current action immediately after the real
`0x0044a1e0` invocation returns and before `0x0044a670` begins has a
source-qualified completed-update boundary for that field. The wrapper must not
invoke an extra `0x0044a1e0` merely to obtain an observation. It must forward the
intercepted main-loop invocation exactly once, then copy the field after that
original call returns. Gameplay already performs graphics work, so a second or
"pre-render" call would execute another update rather than observe one.

## Runtime acceptance still required

The runtime implementation must independently prove that it wraps the actual
main-loop call site at `0x00404a48`, runs after the original call exactly once,
copies the selected closure before the next main-loop stage, and associates the
copy with the correct prepared input ordinal. It must reject a changed executable
or hook profile. A stopped-byte snapshot remains `StoppedBytes`; this source
qualification cannot promote an arbitrary pause into a completed observation.

The first accepted record should expose current action as completed and report
global transform as unavailable with the reason above. A later transform claim
requires a closed writer/alias analysis plus runtime acceptance evidence. Source
review alone also does not prove absence of external modification, injected code,
or a wrapper bug.

## Evidence and limits

The executable identity is SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
The earlier byte-span comparison found the live installed bytes equal to the
static executable for the main loop, gameplay callback, actor updater, mechanics,
graphics-end and input spans. See
`TestResults/physics-clock-investigation-20260920.md` and its adjacent JSON.

This review did not control Hype, install a hook, or produce fresh behavioral
evidence. It did not exhaust all matrix writers, qualify screenshot presentation,
or claim that every actor updates on every poll. The selected action definition
and family relationship remain invariant comparison data; only the actor's
current selection is qualified here as a mutable completed-update field.
