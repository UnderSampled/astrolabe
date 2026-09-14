# Hype gameplay in Godot

Direction after the 2026-09-14 rollback. This refines milestone 8 of
[the main plan](../plan.md), within the existing [adapter architecture](rust-architecture.md)
and [project ownership contract](godot-project.md). It is a plan, not completed
player control. The previous broad implementation goal is paused; this recovery
does not restart it.

## Starting point and recovery

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
See [implementation progress](implementation-progress.md) for the report.

Retained from the experiment:

- The [API assessment](ai-api-assessment.md), the verified
  [885-entry research catalogue](research/hype-ai-api.tsv), and concise
  [source-game observations](hype-ai-research.md). These guide conversion;
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
response against a clean Hype MCP sequence in a suitable unobstructed area.
Record units, timings and any intentional approximation. A forced state change
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

### 5. Repeatable conversion and Godot authoring

Apply these checks within every earlier slice, then for the combined controls.
Direct OpenSpace conversion and conversion from closed/reopened Rete must
produce equivalent native assets and behavior. A deliberate AI parameter/action
edit must survive Rete and change the expected visible behavior. Verify source
reconstruction for newly supported fields separately; full-package or
compression parity does not gate these controls.

**Exit:** the combined sequence runs through the normal exported project and
maintained support without a converter process, source tables or special test
controller. Inspect animation seeking and generated scripts in Godot. Make an
authored scene/resource variation through normal extension points, regenerate,
and show that it and unrelated levels/settings survive. Run the complete local
suite within the enforced ten-minute deadline.

## Working discipline

- Before adding a runtime class or model feature, name the Hype request it
  serves, the native Godot facility behind it and the observable check requiring
  it. Read the main plan and architecture when changing that boundary.
- Make the next implementation checkpoint **slice 1: input to visible walk/run
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
- Use the existing Hype MCP for relevant comparisons, following
  [the runtime guide](../reference/README.md#runtime-validation). Record executable
  and fixture hashes, save/scene, input sequence, interventions and observations.
  Use Raymap/decompilation evidence when a particular comparison cannot run and
  state the remaining gameplay uncertainty. Do not rebuild the harness.
- Use disposable projects with the consumer's real exporter/support for tests.
  Keep game-derived output local. Run focused checks as work changes and the
  supervised local suite at integration checkpoints; one suite including build
  and cleanup must finish below 600 seconds.
- Report what the player can do, what was compared and what remains blocked.
  Counts of types, API entries or passing service-only tests do not substitute
  for a completed gameplay slice.

Whole-game controller coverage, unrelated services, full save editing, exact
OpenSpace scheduling/storage emulation and broad numeric compatibility are not
prerequisites for this sequence. Later corpus expansion and reconstruction
remain in the main plan; they must not displace the next playable result.
