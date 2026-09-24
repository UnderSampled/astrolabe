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

## Effect identity probe

The cast report's bounded scene tail grows from 176 to 177 children. The new
source child at index 176 is AI-backed, has object types `[13, 15, 65536]`,
and holds the same sampled position from the late-cast through settled
checkpoints. These observations do not yet establish its source model or
motion contract. The imported Fire template `fix:/persos/5-0-25549` uses
AI model 817 but declares object types `[13, 15, 65596]`; mapping the live
child's AI locator to that template requires another source identity check.

Orrery now reads that bounded child's behavior selection and model shape in
the same stopped capture. A fresh original Fire BDD pass
(`TestResults/behavior-fire-cast-1790279417734372838.json`) observed child 176
at `spent` and `settled` with one normal behavior selected at 0, no reflex
behavior, and zero designer fields. These traits match imported model 817,
and the child stayed at `[-74.378, 27.584, 5.777]` in those two samples.
The live model locator remained `live:02890df4`; the traits are corroborating
evidence, not a unique pointer-to-Fix allocation resolution.

The Godot cast report counted one generated actor, but its detailed observation
was `{}`. A standalone Godot probe reproduced the test-harness error: model
817 has no designer variable 0, and converting its missing `dsg_0` value to
`int` aborts the dictionary-producing observation function. The harness now
records a missing start value as null and rejects incomplete actor snapshots.
This was an observation bug, not a change to the generated actor or its AI.

The focused Godot rerun passed the cast scenario and produced complete generated
actor snapshots (`TestResults/behavior-fire-cast-1790278778427649497.json`).
The same original report paired with this rerun passes all 26 functional checks
(`TestResults/fire-cast-cross-backend-observer-fix-20260924.json`); visual
render pairing remains pending.
At `cast_late`, the generated actor was at source coordinates about
`[-74.379, 26.520, 4.527]` with zero velocity and a running compiled Brain.
By `spent` it was at height 2.742 with vertical velocity −7.65; by `settled`
it had fallen to height −9.290 with velocity −21.53. The observed source
child remained at height 5.778 over those checkpoints. Because the live child
has not yet been mapped to the imported Fire template, this is a strong
placement/motion discrepancy to investigate, not proof that the two sampled
actors are identical or that the cast assertion should fail.

The verified generic source generation handler copies the caller's world
transform at creation (see the combat comparison); the first captured Fire
child position is later than that operation. Its apparent forward/upward
offset therefore cannot be assigned to the creation API without an earlier
sample or an execution trace. The arrow's imported and live third object-type
values also differ while its first two match, suggesting a generic runtime
type change, but neither case proves the Fire child's exact template identity.

An offline Fix allocation probe narrows the identity. Imported AI models 28
and 817 occupy payload offsets `0x1f58` and `0x50dd4` in source Fix block
`(17,0)`, whose SNA base is `0x02940020`. Their offset difference is
`0x4ee7c`, exactly the difference between the stopped Hype AI locator
`live:02841f78` and the spawned child's `live:02890df4`. Both live locators
imply the same block base, `0x02840020`. The source loader in
`reference/decompilation/functions/004e8ad0_FUN_004e8ad0.c` retrieves the
active module-block base and applies its difference from the SNA base to
addresses inside the block. The active block is allocated at runtime by
`004ae2a0_Mmg_fn_vInitBlock.c`. This loader rule explains the observed
one-megabyte base shift while preserving model offsets. The capture did not
independently read the active block-table entry, so this remains a strongly
corroborated model-817 mapping rather than a separately proven allocation
provenance. Its differing third object-type value still needs explanation.

An offline import of the Fire template shows family initial state 0 and two
states with no mechanics cards. The template has a character collision-zone
profile but no movement collision group. Raymap's state and mechanics readers
show that states may carry gravity-related mechanics settings; these Fire
states do not. Godot's generic spawn path currently enables motion for every
generated character, then supplies scene or project gravity in the support
component. That default is not derived from the Fire template. The missing
mechanics and movement profile warrant a source integration check before
changing shared spawn physics; they do not by themselves prove that the
original actor lacks gravity.

The first source integration trace rules out a simple no-mechanics/no-ZDM
gravity switch. Raymap identifies ZDM as a separate collision slot. The
original dynamics integrator in
`reference/decompilation/functions/005a1948_FUN_005a1948.c` calculates gravity
from the position-selected physical environment and a current mechanics ratio;
it does not test for a ZDM profile there. The ratio getter
`00595a02_DNM_fn_dfIdentityCardGetRatio.c` returns `1.0` for an invalid card,
although Fire's exact active-card path remains untraced. The integrator also
checks a transient actor byte at `+0x348` before applying gravity. The next
useful observation is that flag and active environment/card for the live Fire
child. A missing ZDM may affect contact, but disabling Fire gravity from the
template alone would be an unsupported support-side rule.

Static inspection of the imported scripts further narrows the movement
question. Model 28's Fire generation call passes the literal zero vector.
Model 817 has no position, velocity, gravity or mechanics operation: its
schedule selects family 13 states 0 and 1, updates other actors' designer
fields and mana, waits 1000 ms, then calls `KillSelf`. Its normal rule can
also call `KillSelf` under the original guarded condition. The state changes
do not select a mechanics card in the imported Fire family. Therefore any
Fire displacement between creation and death must come from the native
generation/action/dynamics path, not an explicit motion decision in its AI.

A bounded fresh Orrery run repeated the original Fire spawn. Its new child
briefly appeared at DynamicWorld index 176 with the same model locator and
type triple, then disappeared before a second tail capture roughly 1.5 seconds
later. The inspector previously exposed the model locator but not the child
SuperObject or Perso address, so reading a dynamics offset from that locator
would have targeted the wrong allocation. The scene-tail observation now
includes capture-local locators obtained directly from the validated dynamic
links.

The decompiled call chain now supplies that path for this build:
`00443f00_FUN_00443f00.c` follows SuperObject `+4` to Perso and Perso `+8`
to a dynamics wrapper; `0044d2c0_FUN_0044d2c0.c` reads the dynamics object at
wrapper `+0x10`; the mechanics callback passes that object unchanged through
`00585d51_FUN_00585d51.c` to the integrator. Orrery's scene-tail reader now
reports the wrapper and object locators and the raw integrator override byte
from object `+0x348`, with absent/unreadable results kept distinct. This byte
is cleared by the integrator after use, so a sampled zero does not prove it
was zero throughout the spell lifetime.

The exact original Fire BDD rerun passed with the new reader
(`TestResults/behavior-fire-cast-1790281145388216885.json`). Child 176 was
absent through `cast_late`, then present at both `spent` and `settled`. Its
node `live:03e0fe34`, Perso `live:04606c4c`, dynamics wrapper
`live:04606e98`, dynamics object `live:04606fc0` and AI model
`live:02890df4` were the same in both stopped captures. The dynamics reader
was ready and its raw override byte was `0` both times. Its source position
stayed `[-74.377876, 27.536543, 5.776759]` across the 800 ms sample gap.
The matching executable and save hashes, scenario cleanup and process-exit
check were verified. These samples do not show whether the one-update flag was
set between captures or whether this dynamics object was scheduled for
integration. The stationary result still needs a source-backed mechanism
before changing Godot's generated-actor motion.

The next source trace separates actor-update eligibility from gravity callback
selection. `004499b0_FUN_004499b0.c` gates per-actor work on the frame stamp;
`004440e0_FUN_004440e0.c` follows the Perso's dynamics wrapper and selects
the last nonnull card in its bounded list, then calls `00443f00_FUN_00443f00.c`
only when a card is present. The latter dispatches a mechanics callback by
card type; types 0 and 8 have the gravity-capable handler in the verified
build. A dynamics object and a zero transient override byte therefore do not
prove the callback ran. Orrery now reads the wrapper's cached card, list slots
and selected card type as typed, capture-local evidence. The imported model
can represent nullable state mechanics but not the original runtime update
stamp or card-list selection. Until Fire's effective per-update selection is
observed, an exported no-gravity rule would still be unsupported.

A second exact original Fire BDD rerun passed with the card reader
(`TestResults/behavior-fire-cast-1790281853278217524.json`). Child 176 stayed
at `[-74.377914, 27.395926, 5.777324]` between `spent` and `settled`. At
both stops its list was empty, inferred selected card was null, cached card
was `live:046073c4` with cached type 0, and override byte was 0. Hype's
sampled list was also empty, while its cached card was nonnull and its
position changed through the scenario. The source update path can populate
and consume a card list during an update, so these stopped captures cannot
establish that Fire never selected a gravity callback. The empty list is a
useful observation, not an inactivity assertion. The new run again verified
the executable/save hashes and full process cleanup.

A third exact original Fire BDD rerun passed with the effective-dynamics
observer (`TestResults/behavior-fire-cast-1790282456643205135.json`). Fire
appeared by `cast_late`; between `spent` and `settled` its StandardGame update
stamp advanced from 215 to 232, the same as Hype's. Its visible position and
sampled integrated velocity remained fixed and zero. At both stops the
candidate cached type-0 card had mass 0 and ratio 0, while the dynamics object
cached mass 72; its sole list slot (key 21) held no card, inferred selected
card was null, mode fields `e0/f0/f4` were all 0, and override/pending
velocities were 0. Hype's cached mass was 120 and its internal velocity
changed through the same interval. This demonstrates Fire received actor
updates without accumulating sampled motion. It does not prove the cached
candidate card was the callback's actual input: its mass disagrees with the
dynamics cached mass, and the list may change during an update. The source
rule for exporting the initial motion state still needs resolution. The
executable/save hashes and full process cleanup were verified again.

The static template has another relevant source link. Raymap identifies
Perso `+8` as a pointer to a serialized dynamics wrapper. Astrolabe declares
that pointer in `PersoHeaderWire` but does not yet project the linked dynamics
profile into the useful Perso model or Rete. The family state's nullable
mechanics references are retained and are a separate concern. A source-backed
initial motion configuration may belong on the converted Perso and exported
scene, rather than in a Fire-specific support rule. The Fire template's actual
serialized dynamics pointer and target have not yet been inspected, so
disabling gravity solely because its states have null mechanics would still
be a hypothesis. The existing unconditional `motion_enabled = true` at spawn
is native-API fidelity debt; any corrective rule must be shared and derived
from the converted source fields or explicit AI vector request.

An offline Fix relocation/allocation probe resolved that static link for Fire
and the arrow template. Each Perso has a nonnull dynamics wrapper, but the
wrapper's first, nested serialized Dynamics pointer is null, with no
relocation at that slot. Neither template therefore provides a serialized
dynamics mode or initial velocity. This does not rule out runtime allocation
of a dynamics object; Orrery has directly observed one for spawned Fire.
Because both templates lack the nested record, that field alone cannot
distinguish stationary Fire from the moving arrow. The Fire AI passes a zero
spawn vector, while the arrow's source AI supplies a nonzero vector. The
installed-build handler investigation recorded in
the combat comparison supports the vector as an initial motion seed written
separately from the copied world transform. It does not establish that a zero
seed disables later dynamics updates or gravity. The Fire AI's zero vector
therefore cannot by itself justify turning native physics off.

Godot's generic generation path enables native motion on every spawned
`CharacterBody3D`, and that component applies gravity each physics tick.
Whether the source Fire actor has gravity, stays attached, or is repositioned
by another source operation is unverified. Changing gravity for Fire or all
generated actors now would be a support-side gameplay guess. Resolve the
source template identity and creation/movement contract first; if a generic
API or exported initial-property mapping then proves wrong, fix that contract
and record its remaining fidelity limits here.
