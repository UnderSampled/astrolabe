# Manor key and gate: static implementation

This isolated change starts at `d493ef2`. It implements source-supported
selection, spatial queries and the gate's blocking movement. It does **not**
establish the complete Gogoud award chain or runtime acceptance. No tests,
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

The native adaptation supplies Godot physics seconds as the incoming step.
The source dispatcher step's units have **not** been established in this
static pass; timing/step scaling and x87 versus Godot numerical differences
still need verification. Nonpositive/nonfinite duration parameters are explicit
unsupported requests; the two traced gate factors are positive. The component
does not reproduce unrelated dynamics fields or execution machinery.

Existing native inventory snapshots already save admission, per-member
selection and selected Object. Existing Global saves already own DSG149 and
the optional Gogoud-completed DSG147. No new save field or gate transform is
introduced. Baseline startup stops the gate Brain before applying saved Global
state, then starts it once. The English fixture has only checkpoint placement
in `scene_actors`, so it does not also restore an opened gate transform. The
fixture is the **post-key, gate-open** state and cannot validate acquisition.
The intro skip/optional DSG147 path and authored session are untouched.

## Remaining award evidence

Model 210 (Gogoud) has no AddObject call. Its Normal1 sets DSG4, executes
action38, sets DSG2, executes action29, selects action30 and enters Normal3;
the latter uses frame40 to turn/leave. The inspected Gogoud animation library's
referenced actions are all sound cues. Neither those cues nor frame40 prove
an award. Gogoud's native stock binding is null. Its direct active-state calls
deactivate itself, while the only inspected direct reference to physical key
`/world/0/142` is IWorld's startup deactivation.

The other observer of Gogoud DSG2 is statue model294: it requests Hype
Normal69, which hides channels and enters Normal16, without granting stock.
The inspected dialogue completion binding enters Gogoud Normal1 and contains
no award payload. No direct references to the three item21 proxies establish
a transfer. A source-proven key activation, dialogue command or equivalent
handoff edge is still required. This change deliberately adds no grant at a
guessed frame, dialogue completion, DSG147 change or scene startup.

Evidence: imported Manor/Fix AI models and StockAction joins; the original
PE callable catalogue and disassembly; original selected-Object getter and
selector; lift initialization/update, registered lift motion callback and
scalar helper in `hype-decomp`. The clean-C peer owns those translations in
`/tmp/hype-sol-key-training-20261008`; this branch does not edit that tree.

## Integration and verification

Rete schema advances 58 to 59 and support interface 77 to 78. Integration
must reconcile version changes from other workers and explicitly update
installed support before regenerating assets. Shared merge surfaces are the
AI model/walker/reader/compiler, Character and support manifest. No generated
asset is committed. No authored `demo.gd`/scene or camera file is changed,
so the parent session-file rename has no overlapping hunk here.

Static review covered operand capture, visitor/validation coverage, reader
and writer pairing, guarded query side effects, native support registration,
selection admission and existing save ownership/startup order. The existing
support-version assertion was updated only to match the version constant;
no test was added or executed. Compilation and generated GDScript parsing
remain unverified.

After the missing award edge is resolved, runtime verification still needs:
one real handoff/admission, selection and lock interaction; both sideways
segments and timing; no key consumption unless newly proven; native save and
fresh reload; repeat load without double admission or another gate displacement;
and source-compatible intro skipping. Kip owns merging and acceptance.
