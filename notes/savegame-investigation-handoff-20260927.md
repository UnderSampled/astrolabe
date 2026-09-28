# Sol handoff: save-game investigation and first persistence slice

## Assignment

Turn the partial Hype PC save research into a repeatable investigation that
explains what state the engine saves, who owns it, and how loading restores it.
Extend Orrery's existing analysis and fixture workflow, prove a complete
save/restart/load cycle, and carry the verified semantics into a bounded native
Godot persistence slice. This is research and implementation work with runnable
evidence, not a request for another plan.

First establish the overall structure and named gaps. Prove mana persistence
using the already identified records, then investigate one inventory change and
one persistent manor interaction. These exercise scalar values, relationships,
and level-owned state before expanding coverage. A mana result alone does not
complete the investigation or establish general save compatibility.

This assignment does not impose an order on the other roadmap efforts. Reuse
their current interfaces; complete recording precision, UI parity, and all manor
gameplay are not prerequisites for investigating source persistence. General
Godot persistence remains required by the roadmap. A converter for original
saves is a separately scoped interoperability feature to assess here.

## Starting state and required reading

Repository: `/home/stephen/Projects/astrolabe`. Inspect the current checkout and
working changes before editing. Preserve unrelated work, reference submodules,
the supplied archive, private installations and existing evidence. Do not assume
an older handoff describes current code: loading and recording work have already
advanced. Ignored game data and generated assets need explicit local access and
are not supplied by Git.

Read root [AGENTS.md](../AGENTS.md), the [architecture](../docs/astrolabe/rust-architecture.md),
[Rete design](../docs/rete/rete-design.md), [plan](../plan.md),
[roadmap](remaining-work.md), [implementation evidence](implementation-progress.md),
[save format notes](../docs/openspace/save-games.md),
[earlier experiments](cheat-save-experiment-2026-09-24.md),
[session recording contract](../docs/astrolabe/session-recordings.md),
[Orrery guide](../docs/orrery/controller.md),
[runtime guide](../reference/README.md#runtime-validation),
[Godot ownership](../docs/astrolabe/godot-project.md), and
[testing guide](../docs/astrolabe/testing.md).

### Existing evidence and limits

- `crates/orrery/src/save.rs` implements read-only `save-inspect` and `save-diff`.
  The supplied English fixture's envelope, checksum, compression and save-table
  record boundaries are understood. Decoding all records does not establish
  their gameplay meaning. Unknown record types currently fail explicitly.
- The fixture contains `General.sav`, `General000.sav`, `Level000.sav`, and
  `manoir.sav`. All 263 manor records also occur in its 1,271-record player file.
  Verify load precedence and later divergence; overlap in this fixture does not
  establish universal redundancy.
- Current and maximum mana are identified as two-byte records
  `0:5:4986212` and `0:5:4986214` for this source context. Earlier game-written
  saves and live observations support the mapping; fresh-process reload behavior
  still needs a deliberate persistence proof. Do not treat these as universal
  addresses across builds or levels.
- The no-cheat control changed a time-like value and two arrays. Additional
  `thereyougo` flag and pointer changes remain unnamed. Health and armor cases
  lacked adequate live observations; they do not establish those mappings.
- `session-recording` already captures, verifies and stages complete save roots.
  Orrery sessions use isolated game/prefix copies and retain output saves and
  stable snapshots. Stable file observations are not yet an instrumented engine
  save-completion boundary. Reuse these facilities.
- The Godot game still applies `godot/game/english_save_manoir.json` through
  `manoir_start.gd`. It has real native inventory and magick properties, but this
  measured fixture is not a general original-save importer. The bridge-gate lift
  in `demo.gd` is an explicit gameplay workaround, not decoded saved state.

## Ownership and implementation boundaries

| Concern | Owner and starting points |
| --- | --- |
| Source-save inspection, semantic mappings and live source observations | Orrery: `crates/orrery/src/save.rs`, `main.rs`, current typed readers and runtime control |
| Immutable initial save sets, isolated staging and output evidence | `crates/session-recording/src/lib.rs`, its fixture tests, and `crates/orrery/src/session.rs` |
| Gameplay scenarios and assertions, including original-only assertions | External `crates/behavior-suite` and `crates/behavior-contract`; reuse their actual current extension points |
| Godot observations and control | `crates/godot-harness` and its maintained Godot support; no second harness |
| Actual game persistence and restoration lifecycle | Existing `godot/game/bootstrap.gd`, `demo.gd`, `manoir_start.gd`, and maintained native components |
| Durable format findings and dated experimental evidence | `docs/openspace/save-games.md` and a new dated results note under `notes/` |

Keep source memory layout, save-record addresses and registration machinery at
the OpenSpace research/import boundary. Godot stores meaningful state using
ordinary properties, references and stable actor/content identities. It must
not implement the source save-table runtime or depend on Orrery. Raw recording
fixtures remain evidence, separate from useful content/Rete assets.

Keep one implementation of each codec and resolver. Use the existing typed
OpenSpace declarations for live interpretation. If a supported source-save
converter later needs production codecs, put source I/O in `openspace` using
declarative binrw where appropriate and let Orrery consume it; do not create a
second decoder or make the converter depend on the controller.

Make narrow edits at shared session, game lifecycle and test-inventory seams.
Preserve the loading work's resource reuse, fresh mutable scene instances and
readiness checks. Coordinate exclusive ownership before launching the original
game; never drive or stop a session owned by another task.

## Work sequence

### 1. Establish the structure catalogue and runnable baseline

Create a versioned report/catalogue covering slot metadata, player statistics,
inventory/equipment/magic, progression, actor/AI variables, level/world state,
references, and unresolved data. Each category needs its owner, current coverage,
evidence, open questions and next completion check. Categories are investigation
targets, not claims that corresponding fields have already been found.

Reuse `save-inspect`/`save-diff` for the baseline and extend them where needed.
Reports must account for the entire selected save set and preserve relative
paths so identically named files in different slots cannot collide. Keep encoded
changes, decoded record changes and verified semantic changes distinguishable.
Retain unknown values and unexplained flag bits. If a record's length cannot be
determined safely, report the undecoded remainder without guessing boundaries.

Demonstrate the existing fixture before adding mappings:

```sh
cargo run -p orrery --locked --offline -- save-inspect reference/Savefile_EN.zip \
  --output output/savegame-investigation/baseline.json
```

Use unique case/run directories thereafter. Add one documented command that
executes the first real scenario and produces its evidence and coverage report;
choose its final spelling through the existing runner rather than adding a
parallel scenario framework. Reports should distinguish placeholder, wired but
unproven, demonstrated for a stated scope, and accepted to that scope's criteria.

### 2. Trace registration, save selection and restoration

Consult Raymap first for practical actor/AI-variable structure leads. Verify
Hype-specific behavior against the pinned executable's decompilation. Related
Rayman/Tonic Trouble save flags are leads, not authoritative Hype definitions.
Start with the routines already identified in the format notes:
`00431180`–`004317d0` and `004c6d60`–`004c7300`; follow their actual callers,
registration paths and load paths rather than inferring semantics from names.

Answer these questions with a source reference and, where needed, a live check:

- Which objects/fields register for persistence, with what type and flags?
  When does the table capture a value, and when does saving use current memory?
- How do slot metadata, player files and named level files participate in save,
  load, level departure/return, and initialization? What is the application order
  when the same field appears in more than one file?
- How do saved block/module-relative identities resolve to current objects and
  referenced values? What context is required to resolve them after a restart?
- Which values are restored directly, reconstructed, reset, or subsequently
  overwritten by initialization/AI? What marks successful save and load?

For each supported mapping, report source profile/build, relative file path,
record identity, raw type/flags/value, semantic owner and field, interpreted
type/units, reference target where applicable, evidence and confidence. Do not
interpret every width-based scalar as an unsigned gameplay integer: signedness,
floating-point meaning and array element types require owner evidence.

Add narrowly scoped registration/load observations only where static analysis
cannot resolve a concrete question. Avoid building a general tracing overhaul.

### 3. Prove source persistence with controlled cases

Begin with an unchanged load-and-save control repeated from the same immutable
fixture. Preserve incidental differences in reports; classify them only when
evidence explains them. Repeated values, changed pointers and correlated arrays
do not by themselves establish a field's meaning.

For every case, use this same bounded sequence:

1. Verify the executable/edition, complete initial save root and staged bytes;
   launch a fresh isolated session. Load through ordinary menus and verify the
   reached level, gameplay phase and fade-in/readiness. Use the supplied English
   save first to avoid the long intro.
2. Capture typed baseline state, perform one controlled action, and observe its
   effect. Record physical input and any explicitly required intervention.
3. Make the game perform its normal save, including overwrite/name confirmation
   where applicable. Observe save completion as far as the instrumentation
   supports, preserve a stable complete output set, and produce the record diff.
4. Stop and clean up the owned process. Stage the resulting save set as a new
   immutable input fixture for a fresh process; do not merge it with old saves.
5. Load it normally, verify typed restored state and a relevant observable
   behavior, then save again where useful to detect initialization overrides.
   Preserve and explain differences rather than requiring incidental byte equality.

Run these cases in order:

| Case | Required question and evidence |
| --- | --- |
| Mana | Use the known `druidik` refill or an observed supported consumption action to produce a definite change. Check current and maximum mana before the action, in the game-written records, and after a fresh reload. Repeat from the original fixture to demonstrate reproducibility. |
| Inventory | Select an available action with a typed, observable item/quantity or equipment change. Establish the owning value and any references, then verify quantity/selection and usability after reload. Do not assume the broad `thereyougo` cheat isolates one inventory variable. |
| Manor world state | Select one reachable pickup, gate, switch or actor interaction that the source actually persists. Verify its owner, affected files, restored state and behavior. Investigate level departure/return if needed to distinguish player from level persistence. A teleport is a logged navigation intervention, not proof of quest progression. |

Choose the inventory/world candidates after the catalogue and baseline reads;
do not commit to an unsupported interaction merely because its name is familiar.
Keep failed attempts and reasons. A source-proven reset is a valid finding but
does not satisfy the case requiring persistent world state; select another
candidate or leave that acceptance gate explicitly unresolved.

Build the source scenarios in the external behavior suite. They can pass on
OpenSpace independently of Godot. Timing gates based on observed menus/state
are appropriate for these semantic experiments; label them as such rather than
claiming literal-time replay or frame-precise equivalence. A file change or
matching input sequence alone cannot pass a persistence assertion.

### 4. Implement the first native Godot persistence slice

Use the existing application and restoration lifecycle. Establish an explicit,
versioned save-state contract with supported fields, stable identities and
visible unsupported categories. Save mutable session/actor properties separately
from reusable converted content. Apply restoration at a verified lifecycle point
so initialization does not overwrite it and gameplay cannot observe half-applied
state. Keep mutable resources isolated between reloads.

Deliver a real Godot save/restart/load proof for mana first, using the ordinary
game's save/load operations and native magick component. The measured fixture
may remain an explicitly labeled seed for a new test slot while source import
is unsupported; loading that seed again cannot stand in for reading the newly
written Godot save. Retain other required starting state and report categories
that are still provisional. Remove a superseded fixture-restoration path only
once its supported responsibilities are covered by the replacement.

Extend the same contract to the selected inventory and world-state cases when
their actual converted gameplay is supported. If a dependency is missing, record
the concrete actor/API/field and failing or pending acceptance case. A synthetic
component check is useful infrastructure evidence, but cannot be promoted to
manor gameplay or cross-engine parity. Do not expand this assignment into broad
NPC repairs, an intro implementation or a full UI redesign.

Verify missing/corrupt/unsupported saves produce clear failures without
overwriting a valid slot or partially applying state. Use ordinary native file
persistence with validated data and safe replacement. Integrate minimal usable
save/load controls into the current menu/session operations; presentation parity
remains separate work. The Godot harness drives and observes these operations.

### 5. Assess original-save conversion and publish findings

Document whether the verified mappings can support a useful original-save
import slice, what converted content/identity context it requires, and which
unmapped categories prevent equivalent gameplay. Keep the decision explicit:
implemented for named fields, feasible but deferred, or blocked by named unknowns.
A complete original-save converter and an original-save writer/editor are not
required for this assignment; neither is exact compressed-byte reproduction.

If a small importer is necessary for a selected comparison claim, implement it
through the owning adapter boundaries and report field/behavior coverage. Do
not silently substitute the measured manor fixture for unsupported source data
or label a partial import as general compatibility.

## Evidence, tests and completion

Keep source saves, runtime observations, screenshots and other game-derived
artifacts in ignored `output/savegame-investigation/` or unique `TestResults/`
runs. Record code/build/content hashes, fixture/member hashes, relevant runtime
configuration, selected slot and level, observations, interventions, save/load
boundaries and cleanup. Preserve the supplied archive's SHA-256:
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
Recheck current session ownership; the old experiment's lease failure is historical
evidence, not proof that the current runtime is unavailable.

Add meaningful checks for new parsing/mapping behavior: malformed lengths and
references, unsupported records, duplicate identities, multiple slots with the
same filenames, and mappings that fail under a mismatched source profile. Keep
known/unknown coverage accounting honest. Retain the existing save-root isolation
tests rather than duplicating their implementation. Include a negative scenario
where saving or loading fails so the runner cannot pass merely by observing an
unchanged fixture or in-memory state.

Use the existing supervisor, declared prerequisites, per-test deadlines and
mandatory safeguards. Reserve cleanup time and report build/setup/test/cleanup
durations. Current areas include `runtime`, `comparison` and `magick`; no `save`
area exists at handoff time. Inspect selection before choosing the affected
areas or adding an explicit new area. These are starting commands, not a fixed
release matrix:

```sh
cargo build --release --locked --offline
cargo build --release --locked --offline -p orrery -p behavior-suite -p godot-harness
cargo fmt --all --check
cargo clippy --workspace --locked --all-targets -- -D warnings
python3 tools/test.py --area runtime --area comparison --local --offline --list
python3 tools/test.py --area runtime --area comparison --local --offline
```

Add the affected gameplay selections for native persistence changes. Update
test discovery and inventory together; do not omit safeguards or loosen assertions
to obtain a pass. Record unrelated baseline failures separately. Full-suite and
full-game compatibility claims require their own complete evidence.

The first checkpoint is complete when the catalogue/report framework runs and
the known mana mapping survives a demonstrated original-game save/restart/load.
Continue through the inventory/world investigation and the native Godot mana
slice; do not stop at that checkpoint when the remaining work is actionable.

Final delivery must contain:

- The structure catalogue with sourced findings, named unknowns and coverage.
- Maintained Orrery semantic inspection/diffs and one reproducible scenario path.
- Original-game mana, inventory and world-state results, with reload evidence;
  pending required cases remain visibly incomplete.
- A demonstrated Godot mana save/restart/load through the actual game, plus the
  supported inventory/world persistence scope and concrete remaining dependencies.
- Updated stable format documentation, a dated experiment/results note, exact
  reproduction commands, focused validation results and the source-import decision.

Report structural decoding, semantic mapping, original-game persistence, Godot
persistence and cross-engine agreement separately. Close each claim only with
its own evidence. If a live environment is unavailable, finish useful static,
synthetic and Godot work and state the missing source proof; do not count the
assignment's live acceptance gates as passed.
