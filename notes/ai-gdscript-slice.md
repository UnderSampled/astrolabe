# First executable AI slice

The ownership decisions below were recorded before implementation on 2026-09-13.
The first executable slice is now implemented and tested. This is
phase 8 work after the accepted native scene/character migration, not a claim
that Hype's complete AI or gameplay is supported.

## Scope and ownership

The first target is the shared program of the two Astrolabe torch sound
controllers. Its schedule waits 250 milliseconds, tests three-dimensional
distance to the main actor against 9 source units, and sends start/stop sound
events on changes of a persistent Boolean designer variable. This supplies a
real dependency graph: two placed persos, one shared AI model, independent
instance memory, an explicit main-actor binding, and existing compiled sound
events/native audio streams.

An AI model owns its ordered normal/reflex behaviors and their typed rule and
schedule trees. The fixture inventory found 33 models and 530 distinct scripts
on 55 of the 108 rooted persos; no script is shared between these models.
Consequently separate global files for every wire record are unwarranted here.
Brain/mind selection and designer values belong to the perso instance. A later
independently shared macro/script may warrant a separate asset; none is needed
by this slice. Fix references remain role-relative and session scoped.

The model exposes statements, expressions, behavior identities and typed
designer values, without source pointers or a second OpenSpace document model.
Rete persists the same values in MessagePack. OpenSpace owns binrw records,
numeric opcode mappings, source identity, SND binding and reconstruction.
Godot emits normal GDScript branches and suspended execution over maintained
timing/distance/audio functions. It must not read or dispatch a source AST at
runtime. Unsupported programs receive asset-specific diagnostics and are not
partly executed with missing actions silently discarded.

The initial codec/compiler grammar is intentionally bounded to the real torch
program and useful synthetic variations: scalar values, Boolean/comparison
expressions, assignments, branches, waits, sound requests and explicit no-op.
Broader behavior transitions, macros, collision/input, cross-perso designer
writes and Hype's player controller remain separate work. Generated components
must be saved in the native perso scene; the level supplies actor references.
Standalone components require that binding before execution.

## Source evidence

Raymap's Montreal Perso reader accounts for the extra word before Brain at
Perso+16. Its AI readers are a starting point, corroborated against the patched
English Hype executable and allocation boundaries:

- `0045fce0` returns **8** for Brain size; its second word is explicitly retained.
  The archived 12-byte reader includes the next allocator header.
- `005154c0` allocates a 20-byte Mind; `005219e0` a 16-byte AIModel.
- `00521a30` allocates a 16-byte behavior-set descriptor with default behavior
  at +8 and action-table capacity at +12; Raymap reads only the first 8 bytes.
- `00521800` allocates 12-byte behaviors, `00521860` four-byte rule slots,
  `005218c0` eight-byte script nodes. `005217d0` initializes the two reserved
  node bytes to zero. All 43,681 inventoried nodes have zero reserved bytes.
- `00521780` terminates scripts on node kind 8; the corresponding fixture
  sentinels have zero indentation. Indentation is +6 and category is +7.
- `00521a80` allocates 24-byte per-instance intelligence. Current execution
  pointers/action-table cursors are transient runtime observations, not model
  values to serialize as source layout.
- The executable's procedure, condition and meta-action tables corroborate
  Raymap's indices. Meta-action 37 (`Action_Wait`) begins at `0054dad0` and
  updates at `0054db00`: it captures the engine millisecond clock and completes
  when the clock reaches the captured time plus the requested duration.
- Procedure 127 (`Proc_SendSoundRequest`, `00542230`) uses the calling perso;
  AI sound operands require the same localized SND/RTS binding discipline as
  animation cues, not persistence of source runtime sound handles.

The source-only inventory is local under `output/ai-research/`. Generated game
scripts, memory observations and media remain ignored. Raw inventory evidence remains diagnostic, outside Rete and generated runtime code.


## Implemented conversion

The rooted import loads one shared `level:/ai/models/824` asset for
`i_Dummy_Son_Torche_Astro_001` and `_002` (`world/0/103` and `world/0/102`).
Each perso retains its Brain selection and independent Boolean designer memory.
The scene's explicit main-actor reference resolves to Hype (`world/0/0`), using
GPT+0x2ae0 and the controlled-actor entry's SO at +4, corroborated by
`00443470`, `00443460` and keyword-5 handler `00549a00`. This role is not inferred
from a name or traversal order. Function 4 at `00537c20` calls `0052ea00` with
mask 7: subtract the world positions and take the three-axis distance.

Rete schema **10** persists typed AI models, Brain memory and the scene actor
binding. Regenerate older local packages. Godot support interface **2** adds the
maintained `behavior/behavior.gd`; adopted projects update support explicitly
before regenerating content. Export produces an ordinary `.gd` file with branches,
assignments and suspended waits, preloading compiled native sound-event resources.
Each perso scene saves a Brain node with that script; the level saves the relative
main-actor NodePath as an instance override. There is no AI AST/opcode dispatcher,
source script loader or runtime scene construction.

The two controllers start outside their 9-unit threshold. After each 250 ms wait,
a near controller sends stop event `sound:/Sound/SP_SonsPonctuels.csb#103`, then
play event `#102`, and sets its Boolean state. Further near cycles do not resend.
Moving outside sends stop and clears the state. Generated sound code uses the
existing native emitter and AudioStreamWAV voice path. The other 32 inventoried
models, affecting 53 persos, produce structured, asset-specific unsupported
operation diagnostics. Source reference operand values/VM addresses are excluded
from those persisted diagnostics. No incomplete program is attached to a perso.

Supported syntax includes scalar values, typed designer assignment, Boolean and
comparison expressions, branches, blocking waits, main-actor distance, localized
sound requests and explicit no-op. The Godot compiler currently accepts normal
schedules; it rejects rule/reflex scheduling until their execution policy is
validated. Broader model values retain ordered behavior/rule identities (including
null rule slots), default/current/previous selections and explicit unknown fields.

## Reconstruction and deliberate edits

`AiScriptCodec` uses declarative eight-byte binrw nodes and maps them directly to
the canonical typed tree. The local source test reconstructs the torch's **32
nodes byte-for-byte after Rete reload** using caller-assigned sound handles.
Those handles belong to the source export boundary, not the persisted asset.
Reassigning handles changes encoding while preserving semantic sound links.
Changing the wait from 250 to 100 changes only the expected four operand bytes
and decodes back to the edited tree. Malformed nodes, reserved bytes, indentation,
missing variables and source variable indices that would truncate are rejected.

Full AIModel/Brain/designer records and GPT actor bindings are **not yet assembled
by the scene CLI writer**; its reconstruction report now states that limit.
No original-package acceptance is claimed. Existing media, matrices, animation,
family/configuration, mechanics, geometry and scene-fragment coverage is retained.

`tests/ai.rs` exports a source-free canonical AI model directly and through a
reopened Rete store, compares generated files and runs the actual generated code
in Godot. It checks strict distance boundaries, wait deadlines, typed state,
cancellation/restart, automatic physics processing and a changed AST threshold.
The real native scene test also moves the main actor across the threshold,
checks the stop/play/stop trace and native voice, and verifies independent state
on the second controller. A Rete edit changes **9 to 3** before regeneration;
the regenerated behavior uses the new radius while the authored luminous Hype
variation and project settings survive alongside the existing pose/geometry edits.
Large world coordinates use samples 0.1 units either side of the boundary to avoid
float-position quantization; the source-free test checks the exact boundary.

## Original-engine comparison

The persistent external Hype MCP harness was used on 2026-09-13. Evidence stays
in `/home/stephen/Projects/hype-harness/local/runs/20260913-050347/`.
The patched English executable SHA-256 is
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`;
the supplied English save archive SHA-256 is
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The supplied SaveGame0 loaded into `manoir`, then the existing navigation tool
selected `astrolabe`. The process was PID 4122687, start time 5247494.

Fresh guarded reads resolved the main actor, torch Brain, current Boolean memory
and active action table. The original torch-to-actor distance was 11.85175 units
and both controller state flags were zero. Only controller 001's local/global
position was temporarily moved, first to 8 units from the actor and then to 10.
Its script, timers, designer values and sound data were left to the engine.

- Near: state changed 0 to 1, observed 70.741 ms into the sequence. A wait was
  already active, so this is not a newly measured 250 ms activation latency.
- Far: state changed 1 to 0, observed 369.005 ms into that sequence.
- Both original positions were restored with expected-byte guards; subsequent
  reads matched the originals exactly and the state remained zero. The game
  was stopped after the comparison.

Read-only action-table/clock sampling found an active duration of 250 ms.
One wait started at clock 386599 and completed at 386849; its next wait began at
386900. The following observed completion was 387150, then restart at 387199.
This corroborates the 250 ms deadline plus a following-tick restart. The observed
engine tick was about 50 ms under that run's load, giving roughly 300 ms cycles.
Godot uses its configured native physics cadence (normally 60 Hz), so automatic
wall-clock cycle spacing is not claimed identical. Deterministic `advance` permits
explicit cadence tests. The original audio waveform was not recorded; the engine
procedure and state changes support the branch interpretation, while Godot tests
check the actual native event trace and voice creation/stopping.

## Remaining gaps

This is one real ambient interaction, not a player controller or full phase-8
completion. Rule/reflex ordering, behavior transitions, macros, collision/input,
cross-perso variables, random-seed policy, localization, the hidden lever and Fire
Trinity remain unsupported. Mixed numeric precision edge cases and frame-for-frame
original timing are unverified. Original mixer/PCM parity, full AI record/package
reconstruction and acceptance of generated packages in Hype remain separate work.
The scene/character semantic gaps in the native migration report still apply.


## Accepted checkpoint

The complete local suite passed **86 tests in 467.576 seconds** (7 minutes
48 seconds), including 97 seconds of release compilation, every local corpus/
Godot test, native/editor checks, direct/reopened-Rete conversion, deliberate
regeneration and cleanup. Report: `TestResults/20260913T092504.324983Z-4253.log.json`.
No timeout or leaked children occurred; temporary cleanup completed. Child CPU
was 1,636.253 seconds and peak child RSS was 1,710,720 KiB. The full scene test
took 231.71 seconds, including its three conversions and actual editor probe.
Strict Clippy, formatting, diff whitespace and the locked release build passed.
The earlier native-only checkpoint remains documented in the migration report.

The committed starter was rehydrated again with the accepted exporter in
45.747 seconds: **3,815 generated files, 399 scenes and 290,892,234
bytes**. The native script is at
`godot/generated/hype/levels/astrolabe/ai/models/824.gd`. SHA-256 checks confirmed
that `godot/game/characters/luminous_hype.tscn` and `godot/project.godot` were
unchanged. The local CLI report is `output/ai-research/rehydrate.log`.
