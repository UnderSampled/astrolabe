# Hype gameplay in Godot

2026-09-27 scope update: the [current roadmap](./remaining-work.md#current-roadmap--2026-09-27)
targets full first-level (`manoir`) parity. The player slices below are useful
intermediate deliveries, followed by the intro/new-game handoff, actor
interactions and independent behavior/physics, and the required savegame and UI
flows. Graphical parity and loading efficiency advance alongside that work.
The intro required by this route is brought forward from the former late-media
milestone; unrelated cinematics remain later. Orrery supplies OpenSpace runtime
analysis and recording/replay to the external behavior suite; Godot has a
separate harness. This scope update does not assert new implementation coverage.

The authored Godot project is the actual game, with its current main menu.
New Game now runs the complete generated manor and full imported AI from its
original starting state. It is the canonical intro integration and acceptance
entry. The provisional Load Game route and New Game share the game-session
lifecycle; loading, savegame and UI work evolve this application.
See [game project ownership](../docs/astrolabe/godot-project.md#one-project-in-two-states).

2026-10-03 acceptance clarification: compare the normal authored New Game path
with the unskipped [original take](./manor-intro-reference-2026-10-03.md), from
menu entry through the recorded return to gameplay after the approach,
conversation, transformation and continued dialogue. Keep all imported AI
active and unmodified. Component and excerpt checks diagnose API behavior but
do not accept intro parity. The current Gogoud walk checkpoint ends before the
first dialogue; passing it advances this path without accepting the full intro.

Direction after the 2026-09-14 rollback. This refines milestone 8 of
[the main plan](../plan.md), within the existing [adapter architecture](../docs/astrolabe/rust-architecture.md)
and [project ownership contract](../docs/astrolabe/godot-project.md). The archived
whole-runtime implementation remains paused; the
main plan's broad, runnable scaffolding rule still applies.

2026-09-24 movement checkpoint: turn, walk, run-versus-walk and two-face wall
contact pass independently in the original game and a fresh normal Godot
export, each in one shared engine session. Their four functional cross-engine
comparisons pass; the source wall contact differs by about 0.02 units at the
held samples. The Godot gameplay camera is still a presentation wrapper, and
render pairing, exact input-consumer timing and later gameplay slices remain
open. See [run comparison](./run-comparison-2026-09-24.md) and
[wall evidence](./wall-contact-2026-09-24.md).

2026-09-22 planning update: use the preserved component/source-rule checkpoint,
then build the runnable framework for the
[live loader and comparison suite](./realtime-comparison-plan.md). That plan
defines data parity, engine-independent behavior scenarios with one shared
Gherkin step registry and declared physical-input recordings, per-backend
assertions, optional cross-backend comparison, measured elapsed time and
screenshots, coverage and focused testing. The first scenario must pass on
OpenSpace before requiring a Godot result. Precise frame-indexed replay and
tight timing/pixel comparisons are separate evidence gates. Resume slice 1
below through those comparisons; the playable checkpoint is still open.

## Starting point and recovery

The [2026-09-23 player Brain audit](./hype-control-progress.md#player-brain-export-gap--2026-09-23-audit)
found that ordinary `manoir` conversion currently omits Hype's Brain node:
unsupported content anywhere in his AI model prevents its import, while the
rule probes add a test-created Brain after loading the exported character. The
current all-or-nothing import gate conflicts with the main plan's rule that
supported children remain usable while siblings are unfinished. After the
comparison suite framework runs and a scenario passes independently on
OpenSpace, its first Godot gameplay case should require partial AI import and a
Brain with compiled script already saved in the normal Hype perso scene. Preserve unsupported behaviors and rules explicitly; if one
can affect that case, implement it or fail visibly rather than skip it. The
existing component probes remain useful evidence for the selected rules.

The complete unfinished experiment is committed on
`codex/archive-openspace-runtime-20260914` at
`9374ef5f860e36223a25f31d75d2c108e0a024e3`. It contains 69 changed files and an
incomplete `Statement::ActorUpdate` compiler change that does not build. It is
an archive for inspection and selective reuse, not an accepted milestone.

Active production code returns to `d8d0375`, **Expose generated scene hierarchies
in the Godot Scene dock**. That boundary already exports native level/perso
scenes, initial poses, animations, sound and two compiled torch AI controllers.
It does not implement player movement or combat. Its Rete schema is 10 and
Godot support interface is 2. No earlier committed implementation was removed.

Restoration validation: **86 tests passed in 535.787 seconds** through
`python3 tools/test.py --local --offline`, with native Godot runtime/editor
checks and complete cleanup. Release build, formatting and strict Clippy passed.
See [implementation progress](./implementation-progress.md) for the report.

Retained from the experiment:

- The [API assessment](./ai-api-assessment.md), the verified
  [885-entry research catalogue](research/hype-ai-api.tsv), and concise
  [source-game observations](./historical/hype-ai-research.md). These guide conversion;
  catalogue size is not the runtime implementation target.
- The pre-existing `godot/project.godot` changes, restored byte-for-byte: Godot's
  generated header, the 4.7 feature declaration and section ordering. These
  may be editor-generated; they are not known to be manual user edits. Local disc
  data, generated content, authored files and reference submodules are preserved.

Removed from active production: the generic behavior-task scheduler, generated
variable/foreign-actor accessor machinery, numeric compatibility helpers,
all-entry API stubs/operation enumeration and their unfinished compiler/model
expansion. Associated tests are preserved on the archive branch. These grew
source execution machinery before delivering a playable control sequence.

The experimental action, root-motion and character-collision work is also on
that branch. It contains useful research but is coupled to the removed runtime.
Recover individual native conversion algorithms only when the corresponding
slice below needs them; do not merge or cherry-pick the archive wholesale.

Ignored generated content may still reference the experiment's newer support
interfaces or Rete schemas. Keeping those files protects the user's edits; it
does not make them compatible with the restored code. Use a fresh disposable
project and fresh Rete output for validation. Reconcile local generated edits
before any later regeneration; do not force an update or edit a manifest to
pretend compatibility. The local recovery directory
`output/recovery-20260914/` contains settings/research copies, a file metadata
inventory and SHA-256 hashes of all 3,880 existing generated files.

## What we are building

**We MUST NOT reimplement the OpenSpace engine within Godot. Implement only the
AI script API using native Godot facilities, and compile the game's AI scripts
to call that API.** This is a hard architectural boundary, including when
OpenSpace internals would be written as compiled GDScript rather than interpreted.

Use the game's converted assets and AI scripts to reproduce Hype's behavior.
The scripts decide when to walk, run, jump, attack, switch equipment or cast a
spell. Compile those decisions into readable GDScript. Implement the requested
movement, animation, contact, spawning and game rules with Godot facilities.
Do not substitute a separately hand-authored approximation of Hype's controller.

For each remaining behavior, first establish the reached source AI call and
compile its decision path. Add the narrow native Godot method that the call
requires, with source-derived parameters and ordinary node/resource state.
If a temporary Godot-side rule is needed to demonstrate a behavior whose AI
path is still unsupported, identify it as fidelity debt in the slice report
and keep its test result separate from source-AI-backed acceptance. An explicit
unsupported call is preferable to a silent support-side decision that makes a
scenario pass for the wrong reason.

The script API is mixed: behavior requests, queries, game services and direct
property changes. Executing an action is a gameplay request; setting an impulse
followed by jumping is a procedural composition of gameplay requests. Neither
requires recreating the engine's memory layout or evaluator.

| Concern | Owner and representation |
| --- | --- |
| Binary layouts, numeric call IDs, source storage rules | `openspace`: declarative codecs and source resolution. Research addresses stay outside generated gameplay. |
| Behavior decisions, action identity, meaningful parameters and links | One shared useful model. Extend it for the selected behavior and dependencies; retain explicit unknowns and source metadata for reconstruction. |
| Persistence | `rete`: serialize those same assets and semantic links. Loading Rete never reconstructs OpenSpace memory. |
| References, expressions, branches and behavior transitions | `godot` conversion: emit direct GDScript, node/resource bindings and normal properties. Resolve source slots and tables ahead of runtime. |
| Movement and contact | `CharacterBody3D`, native collision shapes, physics queries and a small Hype movement component for the requested semantics. |
| Actions and presentation | Native animation resources/player, part nodes, signals and a small action component for transitions, interruption, motion and events that Hype requires. |
| Projectiles and other spawned actors | Converted `PackedScene` instances with ordinary node references, native collision and explicit lifetime. |
| Inventory, damage and magick | Focused game services/properties implementing Hype's rules. Godot does not supply these rules automatically. |
| Authored game changes | Scenes, scripts and resource overrides under `game/`, consuming the same maintained support and converted assets. |

Typed variables, arrays and actor references are normal game data. Emit native
GDScript types, arrays and references; do not wrap every read/write in a source
memory API. Preserve observable rounding or overflow with a narrow helper only
where a real behavior demonstrates the need. Source encoding fidelity remains
an OpenSpace/model concern and does not automatically define Godot execution.

Preserve rule priority, blocking, cancellation and re-entry through explicit
generated behavior flow and Godot callbacks/signals/coroutines. A small game
state machine is appropriate. Source instruction cursors, return flags, generic
task objects and general operation dispatch are not the target public API.
Merely generating GDScript instead of an interpreter is insufficient if that
GDScript recreates a source-engine runtime.

## Implementation sequence and acceptance

Each slice ends in a visible, repeatable result in the actual converted level.
Do the necessary codec/model/compiler work inside that slice. Do not make
importing all controllers or implementing all 885 calls its prerequisite.
Initialization, referenced actors and effects required by the selected path
must work; disabling them or silently stubbing them is not completion.
Apply the main plan's sufficient-first-pass rule to every gameplay dependency,
including imported data, compiled code, native components, presentation and
comparison tooling. State the bounded behavior each first pass proves, record
remaining cases or inconsistencies, and keep the owning component replaceable.
Tighten a detail before proceeding when it could conceal a wrong decision,
input route or contact outcome; otherwise continue building the surrounding
gameplay groups and revisit that detail at its dependent or fidelity gate.

### 1. Playable idle, walk and run in manoir

Use the supplied English SaveGame0 as the original-engine comparison fixture.
Record its actual initial scene, position, equipment and control state. Define
the corresponding meaningful Godot initialization explicitly. If a small
validated fixture supplies save-dependent state, label it as a fixture; do not
claim full save import or infer equivalent state from a loading-phase number.

Trace Hype's real initialization and idle/walk/run scripts, including required
reflex rules and referenced actors. List reached operations and unresolved
dependencies with their source behavior and reason. Compile that behavior
selection and action choice; import the additional useful values and references
it needs. Unrelated actors may retain explicit unsupported diagnostics without
pretending the whole level is playable.

Bind source logical controls through Godot InputMap, adopting physical defaults
without replacing user bindings. Arrows move and Shift selects running. Connect
the generated AI to native action playback and a CharacterBody3D. Convert action
motion into native animation data; consume its displacement in physics with a
single owner so animation and movement do not both translate the character.
Review the archive's root-motion/collision work for units, spaces, loop
boundaries and source evidence before reuse. Implement floor/wall contact and
the queried slope semantics for this path using native physics.

**Exit:** the saved scene exposes its character and animation resources; running
the exported level accepts arrows/Shift and visibly idles, walks, runs, turns
and stops. Compare displacement, action changes, animation timing and floor/wall
response against a clean Orrery original-game sequence in a suitable unobstructed
area, using its verified live control path.
Record units, timings and any intentional approximation. This functional exit
does not require exact input-update alignment or tight millisecond/pixel parity;
those claims receive separate checks. A forced state change
or animation-only motion probe is supporting evidence, not this exit.

### 2. Jump and landing

Extend the same generated behavior path for Ctrl. Implement the used impulse,
world/local speed and jump requests with their distinct effects on existing
velocity. Use physics contact and vertical motion to supply apex/landing queries.
Preserve action interruption and return to movement/idle from the actual script.

**Exit:** stationary and moving jumps leave the ground, follow the expected
motion, land and resume the correct behavior. Compare takeoff, sampled height,
airborne duration and landing against the original, including a wall/contact
case. The historical intermediate height sample is not a measured apex.

### 3. Sword and crossbow

Trace equipment selection and the actual attack/action calls. X selects sword,
Z selects crossbow, and Space performs the context-dependent action/attack.
Implement the needed inventory state, action timing, hit regions and damage.
Resolve attack attachments and object-producing actions to converted nodes and
PackedScenes, verifying spawn timing/placement and cleanup. Source templates
needed for projectiles are real dependencies even if absent from the rooted
initial scene.

**Exit:** the generated player selects each weapon and performs its real attack;
sword contact produces the expected target effect, and the crossbow spawns a
visible moving projectile with correct contact/lifetime and ammunition use.
Compare original-game effects and counts. Verify Space near an interaction does
not always attack. Scene instantiation by itself is not combat support.

### 4. Magick

Extend the same services for Enter and the spell available in the supplied
fixture. Preserve selection, mana checks/cost, action/effect timing, projectile
behavior, collision and lifetime. Implement additional steering or pause/resume
for a spell that actually requests it. The separate Fire Trinity fixture is a
later comparison case, not the default save's spell.

**Exit:** the spell visibly casts and causes its expected effect, consumes the
correct mana, rejects insufficient mana and cleans up its spawned effects.
Compare against the original save's observed 30-to-20 mana cast and establish
the missing collision/lifetime evidence. Record any additional save, cheat or
memory intervention used to reach a test state.

### 5. Real follow camera

The 2026-09-24 source-camera probe measured the active `manoir` view at idle,
after turning and after walking. Keep its transform, projection and screenshots
in the [movement BDD evidence](./camera-research-2026-09-24.md).
After the magick slice, implement a native Godot gameplay camera as its own
development slice, as the user requested. Determine where camera selection and
authored camera objects belong, then calibrate the view and its follow behavior
against the source. Camera-specific assertions belong in that slice. The
current movement BDD scenarios continue to test movement with their existing
presentation wrapper.

### 6. Cliff fall and death

Extend the same player path to a verified cliff in the converted level. Establish
the route and fall outcome in the original game before fixing BDD bounds.
Resolve loss of floor support, vertical motion, fall/death action selection and
the collision or death condition through the compiled player AI and native
Godot physics. For the first bounded death slice, death may end gameplay;
reload through the game's Load Game path is part of the first-level save/UI
work and must be checked when accepting the full route.

**Exit:** Hype can walk off the identified cliff, fall visibly, and enter the
source-informed death state. A behavior scenario checks the approach, loss of
floor contact, descent, and terminal game state independently in both engines;
it does not infer death merely from a position threshold or a missing actor.

### 7. Playable game and repeatable conversion

Apply these checks within every earlier slice, then for the combined controls.
Direct OpenSpace conversion and conversion from closed/reopened Rete must
produce equivalent native assets and behavior. A deliberate AI parameter/action
edit must survive Rete and change the expected visible behavior. Verify source
reconstruction for newly supported fields separately; full-package or
compression parity does not gate these controls.

**Exit:** a person can launch the normal exported Godot project and play the
combined movement, jump, weapon, magick, fall/death and follow-camera path
without the BDD controller, converter process or source tables. The maintained
support remains the same package exercised by the tests. Inspect animation
seeking and generated scripts in Godot. Make an
authored scene/resource variation through normal extension points, regenerate,
and show that it and unrelated levels/settings survive. Run affected development
suites plus mandatory safeguards; use the full declared matrix at release or
broad integration checkpoints under the [current test policy](../docs/astrolabe/testing.md).

## Working discipline

- Before adding a runtime class or model feature, name the Hype request it
  serves, the native Godot facility behind it and the observable check requiring
  it. Read the main plan and architecture when changing that boundary.
- Make the next gameplay checkpoint **slice 1: input to visible walk/run
  with contact**. Review infrastructure growth against working gameplay at each
  slice boundary; do not accumulate another catalogue-wide runtime first.
- Generated code should read like ordinary game code. Conditions and arithmetic
  become expressions; calls become specific component/service calls. Source API
  names may remain as thin adapters when useful, without requiring one runtime
  method per catalogue entry.
- Diagnose unsupported reached behavior at conversion with asset/call context.
  A development stub must fail visibly and cannot count as a supported service.
  Keep optional unsupported level content distinguishable from the playable
  path's required dependencies.
- Use Orrery for new original-game comparisons once its selected live controls
  are verified, following [the runtime guide](../reference/README.md#runtime-validation). Record executable
  and fixture hashes, save/scene, input sequence, interventions and observations.
  Use Raymap/decompilation evidence when a particular comparison cannot run and
  state the remaining gameplay uncertainty. The Python Hype MCP remains
  available for optional diagnosis, but the new suite must use Orrery.
- Use disposable projects with the consumer's real exporter/support for tests.
  Keep game-derived output local. Run affected checks and mandatory safeguards
  as work changes. Bound individual tests and active-area runs, and reserve the
  full, potentially longer matrix for release/broad integration validation.
- Report what the player can do, what was compared and what remains blocked.
  Counts of types, API entries or passing service-only tests do not substitute
  for a completed gameplay slice.

Whole-game controller coverage, unrelated services, full save editing, exact
OpenSpace scheduling/storage emulation and broad numeric compatibility are not
prerequisites for this sequence. Later corpus expansion and reconstruction
remain in the main plan; they must not displace the next playable result.
