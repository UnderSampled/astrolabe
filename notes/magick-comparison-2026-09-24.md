# Default spell: source AI boundary

The supplied English `manoir` save was previously observed at gameplay phase
9 with 30/50 mana and selected spell index 0. Pressing Enter produced a fire
effect and changed mana from 30 to 20. That run did not establish spell contact
or lifetime; see [the historical observation](./historical/hype-ai-research.md).

A fresh static compilation of imported Hype AI model 28 shows that its Enter
rule in normal behavior 4 transitions to normal behavior 12 only after its
designer-state and companion-actor checks. Behavior 12 first calls source
function 232, `Magic_ReadCurrentSpell`. It normalizes the result into a spell
case, checks source function 228, `Magic_ReadMagicPoints`, against ten for the
default fire case, hides the laser pointer, requests action 290, then requests
action 271 and generates the converted template `fix:/persos/5-0-25549`.
That template has compiled AI model 817. Its source node 33 calls function
230, `Magic_RemoveMagicPoints(10)`, and assigns the return value to a level
actor's designer field. The argument `10` was verified in the preserved raw
nodes of model 817, not inferred from the observed mana change.

The typed model, OpenSpace script codec and GDScript compiler now recognize
functions 232, 228 and 230. The native support Resource stores the selected
raw AI code and mana state and is shared with generated actors. Static
disassembly of the verified executable established the dispatch from function
230 to the subtraction arm at `0x536576` inside handler `0x536550`: the handler
subtracts 228 from the function ID and selects jump-table entry 2. The arm
reads current mana at the runtime magic-state pointer plus `0x24`. If the
requested amount exceeds current mana, it leaves mana unchanged and returns
`0`; otherwise it subtracts, clamps the result to maximum mana at `+0x26`,
writes current mana back and returns `1`. The result's source type tag is 6
(`UInt`). The Godot Resource now implements this called API and the compiler
uses its integer result directly. It adds no spell decision or casting rule.
The nonnegative ten-point call reached by model 817 is the verified path;
other argument cases and runtime execution of the compiled spell AI remain
unverified.

The isolated maintained-support Godot test now passes the `30 → 20` success
case, insufficient-points no-op, source maximum clamp, and zero-point call.
The test uses a temporary project without converted game assets. Godot support
interface 32 marks the implemented return contract. This checks the native API
in isolation; it does not claim that the default spell casts in the level.

A fresh scene-scope Rete import yielded 3,534 assets, then direct compilation
of models 28 and 817 through the public AI compiler succeeded without starting
Godot. Model 28's complete normal-12 schedule now has no unsupported source AI
nodes: it emits guarded selected-spell and mana reads and the original action
and object-generation calls. Model 817 emits the literal ten-point deduction
and assigns the returned success flag to the designer field. This is static
conversion evidence, not a passing spell scenario.

The original game's selected spell *index* 0 and the source script's case-1
normalization initially suggested raw function-232 code 47, but 61 also maps
to case 1. The verified executable's function-232 path takes the selected
10-byte spell row, prefixes it with `Magic`, scans selected inventory handles,
compares the resulting name with each item's Info2 string, and returns the
matched item's signed 16-bit info field at offset `+0x1c` (or `-1` if none
matches). The native inspector now implements that lookup. The supplied-save
Godot behavior fixture restores raw code 61 and 30/50 mana to the shared
support Resource; normal exported content still has no invented mana pool.
This restores a starting state, not a spell decision. Next evidence should
compare function-230's observed side effects and the spawned spell's contact
and lifetime. Camera-derived laser target acquisition remains in the later
camera slice.

A fresh Orrery capture from the supplied save reached phase 9 in `manoir`
after selecting **Load game** and **Gogoud's Manoir I** as separate menu steps
and waiting ten seconds after the level appeared. The current build read
selected index 0, selected name `Fire`, and 30/50 magic points in one stopped
capture. The active inventory included `MagicFire` with ID 61 and quantity 0.
The older Orrery reader reported a null `source_code` because it compared
`Fire` directly with `MagicFire`. The verified source lookup instead matches
this selected item and predicts raw function-232 code **61**. This is a
source-derived result from one coherent stopped capture, not a direct call to
the original function during that capture. The game and Orrery service were
stopped afterward, with the save archive intact.

A later bounded Orrery Enter attempt reached the same supplied manor save and
waited ten seconds after loading. The current reader still reported a null
`source_code` because the running Orrery binary predated the prefix fix. A
single 100 ms physical Enter pulse left action 47, mana 30/50, and the scene's
176-child count unchanged through roughly 1.5 seconds after the pulse; no
spell effect was visible in the captured screenshot. Enter is the verified
`Button_Magic` binding, but this run did not capture the source AI gate fields
or prove the game sampled that edge. Model 28's normal-4 cast transition needs
`dsg_10 == 0`, `dsg_59 == 1`, and either `dsg_0 == 0` or a companion Brain's
`dsg_2 == 1`; an earlier rule can leave normal 4 when `dsg_56 == 1`. The
failed pulse cannot yet be attributed to one of those gates or to input
delivery. It does not overturn the earlier observed 30-to-20 cast.

An in-memory import of the current spell template found a separate Godot
blocker. Model 817's normal-0 rule 0 queries whether `MainActor` is in normal
behavior 94 of `fix:/ai/models/28`. Its source operand points to an interior
behavior member in that other model. Import now indexes the active session's
Brain models before decoding scripts, retains the actor-qualified behavior
reference in Rete, and compiles the query against the main actor's Brain. The
compiled guard requires the referenced model identity before reading normal
behavior 94, so a missing or mismatched Brain fails visibly. The in-memory
import confirms this rule contains no unsupported source AI node, and the
generated model-817 script passes Godot's parse-only check. The spell's own
compiled schedule still calls `Magic_RemoveMagicPoints(10)` and writes its
result to `level:/persos/world/0/29` Brain field 2. This is typed source AI
wiring; it has not yet demonstrated a complete cast in Godot.

The boundary for the remaining work is the same: source script calls,
parameters and branch decisions stay in compiled AI, including the spell's
cost and object creation. Godot support implements the native operations
those calls require. Any spell-specific decision added to support would be a
fidelity debt that needs an explicit source rationale and a follow-up to move
it back to source AI compilation.
