# Manor key and gate: static implementation

This isolated change starts at `d493ef2`. It implements the source-supported
key producer, ownership handoff, selection, spatial queries,
attachment APIs and the gate's blocking movement. Runtime acceptance remains
unverified; the original key popup is an explicit dependency on the separate
presentation worker. No tests,
compiler/lint checks, original-engine runs, Godot runs or conversion exports
were performed. Inspection was limited to source files, PE disassembly,
existing imported AST/data and previously generated native resources.

## Inventory and interaction

The physical `iKeys_Manoir_A` actor (`level:/persos/world/0/142`, model 703)
has a StockAction join to catalogue item 21, kind `Object`, quantity `-1`,
availability 0 and management 0. The empty inventory prop at `/world/0/14`
and cheat proxies at `/world/0/67` and `/world/0/122` share that item join.
This is a source identity join, not a grant inferred from an item label.

Catalogue quantity is not ownership. The original categories start vacant;
the native category admission already owns this distinction. Model 703's
Reflex0 eventually calls AddObject on **itself**, after player readiness and
handoff waits. The source AddObject handler reads the evaluated actor's stock
edge; it does not accept a numeric item ID. That existing native path is
reused. The newly typed RelativeZPositionOfPerso query preserves the signed
player height in the key actor's local frame, including the source pickup's
`< 0.2` guard. Other existing contact queries and waits stay in the compiled AI.

Func_Inv_GetObject reads the first selected catalogue Object, returning its
signed identity or `-1`. Func_Inv_SelectObject first resolves an admitted
item, requires exact kind `Object`, clears the other Object selections, and
returns integer 1 or 0. Its operand is an evaluated raw word: real values use
their float32 bit payload rather than numeric truncation. Both calls now have
typed Model/OpenSpace/Rete/Godot paths. The actual inventory screen uses the
same admission-aware selector and displays the selected Object. It accepts an
owned `-1` quantity sentinel; it never admits an item through the UI.

IWorld Manor model 154 requires Button_Hit just pressed and Hype DSG19,
DSG14 and DSG56 all zero. The lock path measures player-to-lock source-XY
distance `< 1.5`, absolute player facing angle `< 25` degrees and absolute
lock facing angle `< 45` degrees. It then compares the **selected** Object
with item 21, writes lock `/world/0/150` into Hype DSG133, requests Hype
Normal34 and waits on Hype DSG49. Normal34 turns toward that runtime actor
and executes action184. Meta23 now imports the actor-valued DSG operand;
the existing compiled turn implementation is reused. The traced lock/key-use
branch contains no inventory removal call.

Func_DistanceToPosition was traced separately: its operand is an absolute
point, not an actor handle. The new typed point-distance expression preserves
that distinction and an explicit Ultra receiver. The lock predicate itself
uses the already supported Func8 query, not Func13.

## Gate and persistence

IWorld writes existing Global `fix:/persos/15` DSG149 before waiting 1000 ms
and requesting gate model 272 Normal1. That normal behavior moves from the
current absolute point to `+2.3X` with factor 4.5, then to `+0.3X` with factor
0.3, writes DSG149 again and enters Normal3. Its separate Normal0 restore
path applies `+2.6X` when DSG149 is set. All these offsets remain source AST
operands; the native API has no Manor identity or scene-specific displacement.

MoveLift captures both points and its scalar once. Its source wrapper completes
when dynamics is absent; otherwise it starts movement, then waits for the
completion marker's magnitude to be zero or unordered. The marker is not
physical velocity and the wrapper has no position-distance convergence test.
The native component owns its request revision, points, scalar progress,
phase and completion. Stationary gate actors advance in native physics ticks
and write absolute translation, matching the source lift callback's direct
matrix placement. Coroutine cancellation uses the existing compiler pattern.

The motion callback resets progress on start while retaining the old phase.
Later updates advance an exponential scalar with divisor `factor / 2`.
Before old progress reaches `length * 0.6321`, it uses that scalar and resets
phase to half the factor. Afterward it uses
`((phase - half) / (half * 2.718) + 0.6321) * length` and increments phase by
the incoming step. Reaching/passing the length snaps exactly to the endpoint
and clears completion. Separate float32 scalar stores and native vector
placement retain the source's important rounding boundaries.

The source clock is now established statically: a millisecond timer feeds an
integer frame interval clamped to 16..80 ms; the event dispatcher multiplies
that interval by 0.001 and stores float32 seconds before calling the lift.
Godot physics seconds therefore have the correct units, and the native API
rounds the step to float32. Physics cadence versus source interval quantization
and x87 versus Godot numerical differences still need runtime verification.
Nonpositive/nonfinite factors remain unsupported requests; the two source gate
factors are positive. The component does not rebuild a timer or source scheduler.

Existing native inventory snapshots already save admission, per-member
selection and selected Object. Existing Global saves already own DSG149 and
the optional Gogoud-completed DSG147. No new save field or gate transform is
introduced. Baseline startup stops the gate Brain before applying saved Global
state, then starts it once. The English fixture has only checkpoint placement
in `scene_actors`, so it does not also restore an opened gate transform. The
fixture is the **post-key, gate-open** state and cannot validate acquisition.
The intro skip/optional DSG147 path and authored session are untouched.

## Established producer and pickup continuation

The key is awarded by **training Gogoud model 991**, not intro Gogoud model210.
Normal4 at training stage3 and IWorld154 mode4 selects Normal18. Normal18 writes
Global152 and local stage5, runs `gogood_master_finimannequin`, then enters empty
Normal5. The original dialogue's owner is training Gogoud `/world/0/155` and its
completion points to model991 Normal19. Normal19 executes its actions and pursuit,
sets local DSG5, then activates the physical key `/world/0/142` at
`MainActor.position + MainActor.sighting * 0.1`. A raw AST scan establishes this
reference; generated-script grep had missed the operands of the unsupported call.

The training worker owns typed Proc12 and its native placement/activation path
in `84a124449b4470194e4080a508ed190dcaa5a5bf`, including its null identity-card
follow-up. The actual key has a dynamics binding but state4 has no mechanics
identity card, so a blanket `current_mechanics == null` rejection is invalid.
This branch supplies the missing Ultra Func164 actor sighting query. The original
helper raises then lowers the placement point by half a source unit and activates
the target even if contact placement fails; it does not admit stock. Key703's own
Reflex0 performs admission through its StockAction after the existing Hype handoff.
The training worker separately owns checkpoint continuity and rejects a stage5
snapshot before item21 admission; this branch does not edit its save code.

The key's initial **custom mask is 7** (bits1/2/3). These StandardGame flags are
separate from activation/display. New HasCustomBit and SetCustomBit typed paths
support self, referenced actors and runtime actor DSGs, including Hype DSG133.
The bit index wraps one-based with the low five bits. SetCustomBit captures old
flags before either operand, and truth uses only the second raw word's low byte;
real operands retain float32 bit payloads. Hype33 reaches Hype61 through key bit3,
then bit10 starts the key's presentation continuation. Ownership is already
established before this bit10 handshake.

Key703 Normal1 attaches self to MainActor channel20; Normal2 removes that exact
child, preserving world pose. New native child-owned attachment uses real Godot
hierarchy and refreshes level actor/Brain/entry NodePaths. A synchronous hierarchy
move preserves active Brain coroutines through their tree notifications. The
original world-parent join was checked against the source GPT; Manor's dynamic
world is `/world/0`, the nearest source World ancestor retained on attachment.
Existing contact-sector ownership is retained. Broader source sector migration
on detach remains unverified; the source helper has more general cases.

Ultra actor sighting changes and Func171 vector angle now compile to native
geometry. Func171 retains source degrees, the raw low-byte sign selector
(0=Z,1=X,2=Y), the zero-vector denominator1 and only the upper cosine clamp.
Its float32 stores are represented; x87 equivalence is not claimed.

Proc141 and Proc158 now have paired typed reader/writer/compiler paths. Primary
viewport reselection restores the old target's opacity, including the same-camera
early return used by the key's `(1,1,0,camera)` call. The final viewport follow-on
below connects that owner independently of an authored rendered camera. Switching
to another already registered camera without an old-camera reset is supported;
source registration forces placement policy zero in that branch. Controller
reset, unregistered activation and secondary viewports remain explicit failures.
Focal changes skip selector/value evaluation when unregistered. The reached key
request overwrites source horizontal FOV with 1.3 radians. Source modes101/104/105
and relative-focal flag0x20 remain unsupported. Initial FOV is exported separately
for the -1 reset selector.

The physical key's `keysico_gogood` dialogue is box5, one answer, flagged blank
lines, no stock/gold payload, and a completion binding to key703 Normal2. It cannot
be replaced by a generic text popup. The presentation worker owns this exact
renderer/UI dependency by Kip's assignment. Unsupported style5 remains a reached
failure until that implementation is merged; acquisition and control release must
not be declared complete from admission alone.

Independent negative evidence excludes an invented intro award: all58 intro
Gogoud family states were traversed, yielding40 animation descriptors and eight
referenced action resources, all sound kind. State30's full five-entry cue array
is sound-only. The intro dialogue answers have zero stock/gold/action/module grant
fields. No source key grant at frame40, intro completion, DSG147 or scene startup
was added. Global219 is not written by training model991; its typed write belongs
to IWorld154's training setup.

Evidence includes retained original AI/StockAction/StandardGame/English dialogue
records, PE disassembly and the peer's complete C. The interaction translator's
`f474a3a`, `b32d68f`, `b0d5248` own inventory, Meta36/lift, custom-bit handlers and
activation bodies in `/tmp/hype-sol-key-training-20261008`. The presentation C
translator owns child-channel handler translation and dialogue callbacks. This
branch does not edit either decompilation tree or any peer worktree.

## Integration and verification

The first checkpoint advances Rete58→59/support77→78; this follow-up advances
Rete59→60/support78→79 for new expressions, statements and actor custom flags. Integration
must reconcile version changes from other workers and explicitly update
installed support before regenerating assets. Shared merge surfaces are the
AI model/walker/reader/compiler, Character and support manifest. No generated
asset is committed. No authored game session, scene or free-camera file is
changed, so the parent session-file rename has no overlapping hunk here.
The converter-owned source-camera support file is changed.

The training worker owns Proc12's null-card correction and actor activation
lifecycle. Its ActorActivation model/import/export changes overlap the adjacent
Perso custom-bit hunks here. Merge both actor-owned fields and initializer
updates. Its Brain pause/resume changes must also retain the synchronous
hierarchy-move guard added here; otherwise attaching the key cancels the
ongoing pickup behavior. Its settled stage5 save policy owns the removed key's
inactive, return-suppressed placed node and excludes an unfinished box5 pickup.

Static review covered operand capture, visitor/validation coverage, reader
and writer pairing, guarded query side effects, native support registration,
selection admission and existing save ownership/startup order. The existing
support-version assertion was updated only to match the version constant;
no test was added or executed. Compilation and generated GDScript parsing
remain unverified.

## Primary viewport follow-on

The retained Manor source has exactly two camera actors: `/world/0/2` has
registration availability 1 and viewport context 1 in both initial and active
parameter sets; `/world/0/147` has availability 0 and context 0. The original
startup registration iterates the camera list and assigns each available,
nonzero-context camera to its viewport. The existing generic exporter already
derived `initially_current` from this data. It now also exports registration
availability and viewport context separately; neither field is an actor address.

The native Viewport retains a weak reference to its semantic primary camera,
published by the source camera's ordinary initialization. An authored
presentation camera may occupy Godot's current render slot without replacing
that source owner. Proc141 resolves the semantic owner rather than treating the
rendered camera as an OpenSpace camera actor. All four operands are still
evaluated and captured before lookup. The old target's opacity is restored
before the same-camera comparison. Selecting that same source owner makes its
native Camera3D current and returns before controller reset or placement.
Thus key703 Normal2's `(1,1,0,world/0/2)` connects to the source camera and can
continue to the existing DSG56 release after the owned dialogue completes.
There is no key, Manor or actor-index conditional in the implementation.

Registration availability and viewport context are distinct. Switching away
without a reset clears only the old context; it does not clear availability.
Selecting an already registered destination forces placement policy zero after
operand evaluation, as the source does. Unregistered destinations and resetting
a different previous camera remain explicit unsupported branches; this follow-on
does not bypass them. Invalid/unloaded owner references still fail. Weak ownership
does not retain an unloaded level's camera.

This follow-on leaves Rete60 unchanged and advances the Godot support interface
79→80. It changes `src/model/camera.rs`, `src/godot/character.rs`,
`src/godot/support.rs`, the maintained `camera/source_camera.gd`, and this note.
The existing support-version assertion is updated only to the new constant.
No test, build, lint, conversion, Godot or original-game execution was performed.
The authored session/free-camera files remain untouched. Runtime verification
must still observe source camera ownership, pickup framing, control release and
level unloading together with the merged training/UI dependencies.

For the presentation importer follow-on, retained `keysico_gogood` item header
bytes are `[1,0,5,0]`; its u32 fields at +08 and +20 hex are both zero.

After the owned training/UI dependencies are integrated, runtime verification
still needs:
one real handoff/admission, selection and lock interaction; both sideways
segments and timing; no key consumption unless newly proven; native save and
fresh reload; repeat load without double admission or another gate displacement;
and source-compatible intro skipping. Kip owns merging and acceptance.
