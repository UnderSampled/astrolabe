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

A separate stopped Orrery capture after the same ten-second manor settle read
Hype at normal behavior 4, reflex behavior 3 and action 47. Its designer
fields 0, 10, 56 and 59 were all typed Boolean false. In that captured state,
the Enter transition cannot pass its required `dsg_59 == 1` gate. This
explains why a new magick scenario cannot assume the untouched supplied save
is ready to cast; it does not establish whether the earlier Enter pulse was
sampled. The capture verified executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`
and save archive SHA-256
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The game and service were stopped without changing the save.

Static model-28 AI shows a `Button_GetSword:Pressed` branch in normal behavior
4 that can set `dsg_59` true when its other designer and Global gates allow it.
A bounded original X pulse from the untouched save, with 55 ms hold and an
800 ms release gap, left action 47, normal behavior 4 and fields 0, 10, 56,
59 unchanged. Orrery acknowledged the physical pulse, but the capture did
not establish whether the AI sampled it. The proven sword behavior scenario
uses a clear-ground setup before X. That setup works in a fresh original-game
run: after the ten-second load settle, Down for 800 ms and a 700 ms settle
left normal behavior 4, action 47 and `dsg_59 == false`. One X pulse (120 ms
hold, 120 ms release gap) produced action 48 and `dsg_59 == true`, stable
after another 700 ms. Fields 0, 10 and 56 remained false, satisfying the
observed Enter designer-field gates. This is a source-backed route to a
cast-ready state; Enter was not sent in that run. Orrery verified the same
executable and save hashes and stopped the game/service without changing the
fixture.

A following original run repeated that setup, then sent Enter for 100 ms.
Action 290 was observed before mana changed; by about 0.9 seconds after the
input step, mana was 20/50, action 271 was active, and the scene child count
had risen from 176 to 177. The new child had object types `[13, 15, 65536]`
and source translation `[-74.37799, 27.02758, 5.77880]`; it remained in the
scene at the final 1.7–1.9-second sample. Its source identity and full
motion/contact/lifetime were not established by that count and type triple.
The stale Orrery service binary still reported null `source_code` while its
inventory listed selected `MagicFire` ID 61. The same executable and save
archive hashes were verified, and the game/service were stopped without save
edits. This is an original-game cast baseline, not a Godot scenario pass.

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
result to the current level's actor Brain field 2. In `manoir` that target is
`level:/persos/world/0/27`; in `astrolabe` it is `/29`. This is typed source AI
wiring; it has not yet demonstrated a complete cast in Godot.

Normal behavior 94 is a positive condition for an optional rule body, not a
prerequisite for model 817's schedule. That model has one normal behavior and
no reflex set. Its rule and schedule have no remaining unsupported AST
fragments, but the spawned actor also accesses manor actors `/27` and `/76`
and the Fix Global actor's Brain. The initial spawn path bound only
`MainActor`, so these actor field accesses could fail. The exported level now
carries a scene-local map of stable Perso references to Brain paths, and
generated Brains receive direct references before starting. The isolated
Godot binding test confirms the
resolved Brain identity and that a freed target remains invalid. This is
generic reference wiring; the full manor spell path has not yet run.

An escalated scene-content export of the local manor completed in a temporary
project. Its packed level root retained the `source_actor_brains` map, and
Godot's headless editor parse check completed without errors. The temporary
generated project was removed after inspection.

The apparent `/29` versus `/27` discrepancy came from importing the same Fix
model against different active levels. This is why the external actor map is
scene-local and regenerated for each selected level. The compiled GDScript
projection of a Fix AI model also lives beside the selected level's Perso
scenes: its source model identity stays `fix:/...`, while its resolved actor
bindings cannot overwrite a different level's script.

The boundary for the remaining work is the same: source script calls,
parameters and branch decisions stay in compiled AI, including the spell's
cost and object creation. Godot support implements the native operations
those calls require. Any spell-specific decision added to support would be a
fidelity debt that needs an explicit source rationale and a follow-up to move
it back to source AI compilation.

## Fire cast behavior scenario

The first Fire scenario uses the same supplied-save clear-ground setup as the
sword scenario, then sends one 120 ms X selection and one 100 ms Enter pulse.
The shared recording checks the selected Fire code 61, the script's cast-ready
designer field 59, action 290, the 30-to-20 mana cost, and action 271. Orrery
now reads actor AI, inventory-derived spell selection, mana, and a bounded
dynamic scene tail from one stopped capture at each checkpoint. The scene tail
is retained as evidence but is not an effect identity or contact assertion.

The named original-game BDD test passed on the verified executable and save:
`TestResults/behavior-fire-cast-1790277786792726218.json`. Its actions were
47 at baseline, 45 at the armed checkpoint, 290 and 271 during the cast, 271
after mana was spent, and 48 after settling. All nine checks passed; the
reported fixture cleanup completed. The armed checkpoint permits action 45 or
48 because field 59 was true during the observed selection action 45. This is
a source-observed transition, not a Godot-specific allowance. The test does
not establish the spawned effect's motion or contact.

The same recording passed independently in a fresh normal manor Godot export:
`TestResults/behavior-fire-cast-1790277843359059514.json`. The single named
scenario and all nine checks passed; the report records complete cleanup.
Both reports carry recording SHA-256
`0b76975d9857f6ea2e78055a35068daa3177a8a9a6151027bbcefeb43dd97b84`.
Godot observed the same action IDs at all six checkpoints, cast-ready field 59,
spell code 61, and mana 30 before and 20 after the cast. It spent mana at the
late-cast checkpoint while the source still read 30; both read 20 at the next
checkpoint. The source and Godot actor positions differed by a few tenths of
a source unit during the effect, so this independent pass does not imply timing
or trajectory parity. Orrery's `compare-behavior` command applies the declared
cross-engine bounds to these reports.

That pairing passed all 26 declared checks with no failures:
`TestResults/fire-cast-cross-backend-20260924.json`. It compares the functional
cast path and named checkpoints, not projectile damage, animation frames or
pixel parity. The mana deduction occurs within a different 200 ms sample
window, so a tighter timing claim needs completed-update observations.

This scenario adds no spell-specific decision to the Godot support package.
The converted model-28 and model-817 scripts still own the state transitions,
cost request and actor creation. The existing native API fidelity debt remains
in the generic support operations, especially the unverified spawned-motion
frame/timing, `SeePerso` visibility approximation and channel presentation
listed in the combat comparison note. A failing Godot cast must be traced to a
source call, binding or missing API operation before considering any support
side behavior rule; such a rule must be recorded here as new fidelity debt.
