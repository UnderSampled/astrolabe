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
raw AI code and mana state and is shared with generated actors. Function 230
remains an explicit stub. The available Raymap catalog identifies the names
and arguments but has no handler body or return contract. A static disassembly
of the verified original executable found a shared magick handler at
`0x536550` with a saturating subtraction arm that returns success as a Boolean;
the internal arm has not yet been proven to be source function 230. Assigning
that Boolean or a guessed remaining-point count to the level actor before
mapping the dispatch would silently alter the source rule. The Resource
returns no value and the compiled AI reports the unsupported call before using
it.

A fresh scene-scope Rete import yielded 3,534 assets, then direct compilation
of models 28 and 817 through the public AI compiler succeeded without starting
Godot. Model 28's complete normal-12 schedule now has no unsupported source AI
nodes: it emits guarded selected-spell and mana reads and the original action
and object-generation calls. Model 817 emits the literal ten-point deduction
and checks for the native stub's missing result before the designer-field
assignment. This is static conversion evidence, not a passing spell scenario.

The original game's selected spell *index* 0 and the source script's case-1
normalization initially suggested raw function-232 code 47, but 61 also maps
to case 1. A source trace shows function 232 searches the same inventory list
Orrery reads, selecting a handle whose name equals the current spell table
row, and returns that item's signed info field. Orrery already filters handles
by the source selection byte; the saved selected set includes `MagicFire` ID
61. Orrery now reads the selected table row, mana fields, and matching active
inventory code from one stopped capture. Its focused test checks the decoder
against synthetic `MagicFire` data; a fresh original-game read is still needed
to establish the actual row and returned code. The support Resource leaves the raw code unset until
it can be checked and restored through a behavior starting state. It also does
not invent a mana pool for normal exported content. The default spell cannot
yet pass the planned Godot behavior test. Next evidence should establish the
function-232 return for selected index 0, function-230 return/side effects,
and the spawned spell's contact and lifetime. Camera-derived laser target
acquisition remains in the later camera slice.
