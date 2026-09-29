# Sol handoff: comprehensive Hype save-game semantic mapping

## Objective and completion contract

Implement the next investigation now. Build a reproducible map from Hype PC
save records to the game state they mean, across the pinned English build and
available game corpus. Start with the supplied manor save, then expand to the
other levels, registration families and progression states. The user wants the
meaning map itself, not another demonstration that a few values survive saving.

The previous [first-slice handoff](savegame-investigation-handoff-20260927.md)
and [results](savegame-results-2026-09-27.md) are the baseline. Their 42 passing
checks establish useful infrastructure and five gameplay mappings. They do not
complete this assignment. Comprehensive semantic mapping remains the priority.
Per the user's 2026-09-28 clarification, apply knowledge gained to bounded
improvements in the Godot port, its AI script API, Astrolabe and Orrery when
doing so does not materially distract from mapping. The implementation guidance
below supersedes the earlier blanket exclusion of additional Godot gameplay.
A full original-save importer/editor remains a separate deliverable; useful
shared helpers and focused fixes may advance here under the same guidance.

Keep three milestones separate:

1. **Accounted for:** all records and registration paths in the declared corpus
   appear in the catalogue, including individually identified unknowns.
2. **Owned and typed:** records resolve to a source object/field or justified
   engine bookkeeping role, with layout and persistence rules.
3. **Semantically explained:** their purpose, values/relationships and restoration
   behavior have supporting source evidence, with representative live checks
   where observation is needed to resolve uncertainty.

Milestones 1 and 2 are checkpoints. Do not mark the whole mapping goal complete
while semantic unknowns remain in its declared scope. An entry called
`DSG[8]`, an unnamed pointer, or a documented unresolved reason is not a meaning.
Source-backed family rules may establish many mappings without a separate live
experiment for each field. Distinguish that evidence from runtime confirmation.
Report genuine blockers and partial results honestly; continue independent
actionable mapping rather than shrinking the corpus to obtain completion.

## Starting evidence and boundaries

Read AGENTS.md and its architecture/reference instructions. Reuse the existing
OpenSpace model, source identity/resolution helpers, Orrery inspection and
external behavior suite. Preserve current working changes and reference modules.

- [Save format and registration notes](../docs/openspace/save-games.md), especially
  registration/restoration, establish the current engine evidence.
- `crates/orrery/src/save.rs` owns existing research inspection/diff/profile
  annotations. Extend the maintained path instead of creating another decoder.
- `reference/decompilation/functions/004c68e0_FUN_004c68e0.c` is the central
  registration routine. Six known wrappers are `004c6240`, `004c6260`,
  `004c6280`, `004c62a0`, `004c62e0`, and `004c6400`. Investigate their callers;
  the two four-byte wrappers need not imply identical semantic types.
- Relevant save/apply paths include `004c66c0`, `004c6e30`, `004c7250`,
  `004c7300`, `004c6430`, `004c6d00`, `004c6c20`, and startup `00402780`.
  Check indirect calls and wrappers beyond these initial leads; a text-search
  result alone does not establish complete registration coverage.
- Raymap's `reference/raymap/Assets/Scripts/OpenSpace/Object/Perso.cs` and its
  designer-variable types are practical ownership leads. Corroborate Hype
  meanings against this executable; related-game flags are not authoritative.
- Fixture: `reference/Savefile_EN.zip`, SHA-256
  `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
  Verify the hash against the existing results before use; never alter it.
- Existing runtime executable SHA-256:
  `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
  Match decompilation evidence and loaded data to the chosen profile explicitly.

Keep runtime addresses, registration tables and instrumentation in OpenSpace/
Orrery research boundaries. They must not become Rete's asset model or a Godot
runtime dependency. Diagnostic JSON is appropriate for this catalogue. Promote
needed source structures through existing typed readers, with binrw declarations
where appropriate; do not traverse opaque records by guessed offsets. Avoid a
broad parser migration or engine reimplementation as a prerequisite.

## Continuation after the 508-meaning plateau

This is the continuation strategy for the [paused checkpoint](savegame-pause-checkpoint-20260928.md).
It takes precedence over repeating the initial discovery/ownership phases below.
The task and monitor remain paused until the user requests resumption.

During approximately the final 3.5 hours, complete meanings stayed at 508 while
ownership rose by 135 records and partial explanations rose from 366 to 398.
Useful results included the actor-qualified AI index, corrected field attribution,
restoration experiments and committed evidence. Those supporting results did
not resolve the remaining fields' complete meanings. Three full selections
occupied about 30.5 minutes while research continued alongside them; one roughly
ten-minute run was invalidated by concurrent implementation edits. Several
dialogue, map-star and reference surveys returned to the same missing consumers.
Runtime setup and attribution corrections also caused rework. The planning
review should have pressed earlier for a concrete semantic result from the
accumulated tools. Treat this as a change in execution strategy, not a reason
to weaken evidence or discard useful groundwork.

### First resumed investigation

Sol should own one specific question through to an evidence-backed conclusion
before expanding the investigation portfolio. Start with the existing dialogue
experiment: fix the bounded Orrery registry-reader subtraction failure, verify
the out-of-range case with a focused check, then rerun the isolated observation
using the proven private runtime. Follow the saved reference to the verified
object and section key, then to authored answer data if that link is available.
A parser key alone is still not dialogue text or effect. Generalize to other
records only when the family rule and its applicability are established.

A genuinely independent Luna assignment can pursue the verified native dispatch
edge for procedure 82, which blocks Hype DSG[11] and Global DSG[4]. Reuse the
negative finding that the previously examined table resolves into strings;
verify the executable/build and callback edge before tracing its effect.
Do not launch another general survey of those fields. If the dialogue path
remains blocked, record exactly what new observation failed to resolve and
choose the next concrete consumer or progression-state question from existing
evidence rather than restarting its ownership research.

### Require a new question before repeating work

Each assignment must give the worker the current actor/field identity, relevant
index rows, established facts, previous negative findings, the precise missing
link and the new evidence that could distinguish competing interpretations.
Name the expected decision and where to preserve its evidence. Use the existing
actor-local ownership contract; shared AI models do not merge actors' saved
values. A useful negative result rules out a previously live possibility or
identifies an exact new prerequisite. Reconfirming the same count, layout or
unsupported consumer is not a new semantic result.

Keep the shared blocker summary and authored conclusions in `notes/`, with raw
captures/indexes local. Do not reassign dialogue, star or `1:6` work unless the
brief names a different source, a newly decoded operation, a verified new state
or an experiment that can close the missing link. Partial evidence and stronger
runtime corroboration remain valuable, but report them separately from new
complete meanings and implemented gameplay/API behavior.

### Make supporting work earn its place

Use the maintained joins, actor-local index, corpus reports and saved experiments
as the starting point. Add a tool or abstraction only to unblock the selected
question, make a necessary observation reproducible or correct a demonstrated
defect. Once that need is met, return to the field's meaning. Do not commission
another general index or framework as a substitute for resolving a consumer.
The authorized bounded Godot/API, Astrolabe and Orrery improvements remain in
scope when they advance that investigation or directly apply verified knowledge.

Delegate complementary work with explicit ownership and deliverables. Reuse
knowledgeable workers where useful; multiple Lunas remain welcome when their
tasks are independently actionable. Keep one live-session owner and provide
the exact successful fixture/config/launch path, normal approval requirements,
input/display/audio routing and cleanup recipe. Worker count itself is neither
a progress measure nor a reason to generate more assignments.

Use focused validation and mandatory safeguards for the actual changes, and
reuse the valid 50-case baseline within its recorded scope. Documentation or
static annotations alone do not require the whole live selection. When a full
affected selection is required, finish edits to its exact fingerprint inputs
before starting; independent notes and read-only work may continue. Diagnose a
failed setup or scenario before repeating it. Preserve successful authored
probes and failed-run evidence, so the next worker can reproduce the result
without recreating a temporary harness or repeating known permission failures.

### Reassess semantic progress explicitly

After the first 4–6 active hours of an eventual resumption, compare with the
pause baseline: 508 complete meanings, 398 partial unknowns, 1,222 owned/typed
records and 13/222 arrays with complete content meanings. Report which formerly
unknown fields now have verified meanings, which shared blockers were actually
removed, which conclusions gained runtime support, and what implementation
improvements resulted. Record setup/testing/rework costs separately where
observable; overlapping worker time cannot be summed into elapsed time.

If the count remains flat, explain any substantive new evidence and what it
enables next. If neither meanings nor an identified shared blocker have advanced,
revise the next investigation before sending more similar assignments. Do not
silently spend another interval on the same structural surveys or raise counts
by relabelling partial knowledge. This review point neither completes/shrinks
the comprehensive goal nor automatically pauses it after resumption.

## Work sequence

### 1. Establish the corpus and generate the registration index

Create a versioned corpus manifest: executable/data/fixture identities, save
members, available levels, existing captured saves and relevant registration
paths. Cache immutable baseline inspection. Initially account for all four
supplied files, including metadata outside SAI records. Preserve relative paths.

The supplied files contain 1,271 player records and 263 named-level records;
the latter overlap the former. Report file occurrences and distinct contextual
identities separately. Do not call these 1,534 independent gameplay attributes.
Arrays can contain multiple meaningful elements; one explained header does not
make an opaque payload understood.

Generate a searchable index of registration callers and wrapper chains, with
source location, address expression, type/size, flags, loop/branch context and
potential owner. Resolve common registration loops once so the result applies
to a family. Preserve dynamic arguments and indirect-call uncertainty explicitly.
Keep raw evidence linked; print compact counts and unresolved groups.

### 2. Build the automatic ownership join

Join save block/module/offset identities to the matching source allocation
context, typed object layouts, inventory definitions and actor/designer fields.
Validate bounds, field sizes, flags and context before accepting a match.
Do not reuse heap addresses across processes or bind shared Fix assets globally
to whichever level was inspected last. Ambiguous matches must remain ambiguous.

Exploit existing actor identities, field descriptors and typed AI access paths.
Resolve references to meaningful targets/roles and arrays to their actual
element layout; a pointer delta or byte-buffer length alone is insufficient.
Generalize proven family rules with clear applicability conditions instead of
accumulating hundreds of hand-written offset labels in the supplied-save profile.

If static/source joins leave a substantial gap, add the smallest bounded
read-only registry snapshot or targeted registration trace needed to resolve it.
Capture relevant phases, registered address, type/size/flags, runtime context
and caller where available. Detect dropped events and changed object lifetimes.
Prefer bulk local capture over one MCP request per field. Do not build a general
tracing framework before showing that this observation is necessary.

The first useful checkpoint, aimed for roughly two hours of work, is a generated
ownership map with reconciled counts, the largest unresolved families and a
revised effort estimate. This is an estimate/checkpoint, not a deadline, stopping
condition or permission gate. Continue useful work after publishing it.

### 3. Explain unresolved families and restoration behavior

Prioritize groups that explain many records and important gameplay state:
globals/player statistics, inventory/equipment/magic, actor designer variables,
quest/progression and world state, references/arrays, then metadata/bookkeeping.
Follow registration callers and code/AI scripts that read or mutate each field.
Use data flow and observed semantics rather than plausible names alone.

For each family, establish applicable value types, units/enums/bit meanings,
ownership and lifetime. Determine live-memory versus registration-snapshot
writing, player/level selection, load/apply modes, overwrite order and later
initialization. Trace player, named-level and `HYPE/` working-save precedence.
Padding, reserved, obsolete or derived-state classifications also need evidence.

Persist one catalogue entry/rule with: profile and context; file/record identity;
owner and field/member path; storage layout; semantic description; save/load
rules; source evidence; runtime evidence if any; confidence and unresolved
questions. Separate source-verified, runtime-corroborated, inferred and unknown.
Keep observed runtime addresses in transient evidence, not portable identities.

### 4. Validate selectively and expand across the game

Use retained saves, scripts and screenshots first. Choose the next live case
for the specific ambiguity it resolves, recording a hypothesis and expected
typed before/after observations. Cheats remain the intended keyboard-facing
inventory controls; use representative ones with verified conditions/targets.
A save-per-cheat or save-per-field survey is unnecessary.

For an uncertain meaning, use normal game input, inspect typed state, let the
game save, compare the records and reload when restoration is part of the claim.
Cluster compatible observations in one controlled run, keeping action boundaries
clear. Include controls where timing/initialization can change unrelated bytes.
Do not infer individual meanings solely because many records changed together.

Expand the index across all locally available levels and registration families.
Choose additional saves or progression states for newly exposed structures,
dynamic registrations, transitions and conflicting player/level values. Existing
teleports may reach levels but do not prove normal quest progression. Track
unvisited states and incomplete discovery separately from mapped records.
The manor map is a milestone; continue the broader English-build mapping.

## Apply findings while preserving mapping priority

An improvement is appropriate during this work when the current investigation
already supplies its evidence, the change has a clear bounded outcome, and
focused validation can establish that outcome. Examples include fixing a typed
Astrolabe reader/join, improving an Orrery observation or reproducible scenario,
or implementing/correcting a native Godot AI API operation whose behavior is now
understood. Favor changes that validate an interpretation, remove a recurring
investigation cost, or deliver a useful increment of game parity with the same
source evidence. These improvements are authorized; no separate approval is
needed for each bounded change.

Keep the investigation and implementation connected: record the source contract,
apply the smallest useful change, run the affected checks with mandatory
safeguards, and feed discrepancies back into the meaning map. Preserve the
architecture in AGENTS.md: native Godot facilities and compiled game AI scripts,
the shared useful model, separate adapter ownership, and no reconstruction of
OpenSpace's internal engine or an AI interpreter inside Godot. Respect the
existing gameplay milestone order when choosing improvements.

If an opportunity grows into a broad refactor, new subsystem, prolonged unrelated
debugging or a separate gameplay milestone, record its evidence, affected API/
component and concrete next step in `notes/`, then continue mapping. Do not turn
this permission into a second comprehensive parity goal or routinely request
permission to defer a distraction. Multiple Luna agents may handle independent,
bounded improvements with explicit file ownership and acceptance checks; keep
integration, validation and live-session ownership coordinated.

Report source understanding, implemented behavior and runtime corroboration
separately. A field meaning does not prove that the port implements it, and a
passing component test does not prove cross-engine gameplay parity. Record the
cost and useful result of these improvements separately where observable; do
not infer time or token savings without supporting comparisons. Mapping's
completion contract remains unchanged.

## Preserve findings in the repository

Use `notes/` throughout the investigation for dated family findings, experiments,
negative results, unresolved hypotheses and implementation follow-ups. Link
them from the current progress note and, where useful, `notes/README.md`. Move or
consolidate essential authored research currently kept only under ignored
`output/` into those notes, updating catalogue/source-evidence links. Preserve
the reasoning and limits needed by a future contributor without this chat;
there is no need to rewrite every report for presentation. Promote settled
format/API explanations into the appropriate stable `docs/` pages.

Retain necessary authored reproduction helpers in maintained tool locations,
not solely in `/tmp` or ignored output. Document commands, executable/fixture
identities, relevant source symbols/locations, expected observations and the
private evidence paths or manifest. Keep raw game data, original saves and
game-derived captures/catalogues private in their existing ignored locations;
notes should explain conclusions without requiring raw assets to be committed.
State which evidence can be regenerated and which retained observations still
need private preservation. Do not claim an independent backup unless verified.

At natural checkpoints, preserve coherent owned source, tooling and authored
notes in version control, excluding unrelated shared changes. A local untracked
note or a generated catalogue alone does not preserve the investigation in
repository history. Keep the current checkpoint concise enough to resume from
it without rereading all agent histories.

## Delegation and efficient validation

Sol owns corpus definitions, catalogue/schema integration and final claims.
Delegate independent registration families to Luna after their boundaries are
clear. Multiple Luna agents are welcome. Give each exact symbols/files, the
shared catalogue contract, evidence pointers, expected result and file ownership.
Prefer self-contained briefs without full conversation inheritance, or reuse
an existing knowledgeable agent. Do not duplicate whole-repository discovery.
Workers return mappings, evidence, unresolved cases and changed files compactly.
For every AI-field brief, apply the verified owner contract: `Expression::Variable(n)`
and statement assignments to variable `n` address the executing Perso's own
current Brain/DSG[n] buffer. An AI model supplies the descriptor type/default
template; it is not a separate current-value owner. If several Persos share a
model, qualify each local read/write by the executing actor before matching a
save identity. Keep Hype DSG[57] (`0:17:15540`) distinct from Global DSG[57]
(`0:17:4658`). Explicit `ActorFieldRead`/`ActorFieldAssign` targets are a
different cross-actor path; dynamic targets and unsupported AST fragments
remain unresolved without further evidence. Include this distinction in
worker results and reusable AI index output to avoid repeating local/external
attribution corrections.
`tools/save_global_ai_index.py` now writes a sibling
`local-ai-variable-index.json` from the same typed Rete export and semantic
catalogue as the explicit Global inverse index. It projects local sites by
linked Perso, includes saved DSG fields with no local site, and keeps compound
array targets unresolved. Use its actor-qualified rows in future worker briefs;
the external Global ranking remains a separate navigation measure.

The dialogue, star-entry and `1:6` reference passes already reconcile their
structure. A follow-up on those families must name the missing bridge and a
different source or experiment: dialogue target-object section key to authored
answer row/text; star `+0x38` gameplay consumer or verified map-open transition;
`1:6` source allocation to an exact rooted Level node plus a type-specific
caller. Do not assign another count or ownership survey as a meaning task.
For Hype DSG[11], native function 205 is now traced through the pinned patched
executable to the first selected `"Object"` inventory item's info word, tested
against 44. The next high-use discriminator is procedure 82: its native
dispatch/effect is still unresolved across Hype DSG[11] and Global DSG[4].
Use the [shared blocker index](../output/savegame-semantic-map/shared-consumer-blockers.md)
and [native trace](../output/savegame-semantic-map/native-link-swap-83.md);
require an actual procedure callback/effect before naming either gameplay state.
The [bounded dispatch follow-up](../output/savegame-semantic-map/procedure82-native-85.md)
ruled out one apparent table: the decompiled `00527250` accessor's ID-82
address lands in ordinary strings and does not identify a callback. Obtain an
independently verified procedure-ID-to-callback edge before another body trace.
For dialogue, the [next live bridge plan](../output/savegame-semantic-map/dialogue-live-bridge-plan.md)
starts from the validated module-21 registration for saved slot `0:21:392`
and checks its object `+0x24` section-name pointer. Treat any key as an
observed parser identity until the target and backing authored data agree.
The [first isolated attempt](../output/savegame-semantic-map/dialogue-live-bridge-87/dialogue-live-bridge-87.md)
reached playable manor but `save_registry::read_registry` panicked before
the slot read because its `bool::then_some` argument eagerly subtracts an
out-of-range block base. No dialogue meaning was gained. Fix that bounded
registry mapper and verify the negative range case before rerunning the probe.

One owner controls live-game sessions. Use Orrery's verified Gamescope headless/
private Xwayland path, authenticated Xvfb for rendered Godot checks, and private
game-only audio. Route launch, physical input and capture consistently; never
fall back to desktop display/audio. Preserve the private installation, saves and
unrelated processes. Use isolated copies, deadlines and owned cleanup. Before
launch, validate keys through the actual parser, fixture/profile hashes, renderer
and audio prerequisites. Keep failed-run evidence outside temporary cleanup.

The focused `DefaultMagicPoints` restore discriminator is retained as
[`tools/probes/run_magic_default_restore.sh`](../tools/probes/run_magic_default_restore.sh)
and its [Rust scenario](../tools/probes/magic_default_restore.rs). Run
`tools/probes/run_magic_default_restore.sh --compile-only` for a safe build, or
run the launcher without arguments for the isolated 8/40 original-game trials.
The launcher builds the Orrery bridge, copies its temporary integration test,
and removes that copy on exit. A Codex live run requires escalated runtime
access for Orrery's private PipeWire/PulseAudio Unix socket and Gamescope;
the first default-sandbox attempt failed at socket bind with `Operation not
permitted`, before Hype launched. The successful focused live command was
`cargo test --locked --offline -p behavior-suite --test magic_default_probe_temp -- --ignored --nocapture`
with that access; [the reproduction note](../output/savegame-semantic-map/magic-default-restore-template-run.md)
records the fixture/executable hashes, staged-save hashes, timing and cleanup.
The current template has stronger explicit assertions and passed compile-only
validation; it has not been rerun live since those assertions were added.
The separate [magic-selection probe](../output/savegame-semantic-map/magic-slot-86.md)
used three fresh isolated processes and restored `0000`, `0100` and `005a`
at `0:5:4986216`, with native selected-spell observations 0, 1 and 0.
The high byte persists on this route but its meaning remains unknown. The
temporary experiment test was removed after the successful run; the JSON
evidence remains under ignored `output/`.

Use fast synthetic/offline checks for joins, ambiguity, context isolation and
coverage accounting. Test meaningful negative cases and family generalization;
do not generate a redundant test for every labelled row. Register added/renamed
tests in `tools/test-manifest.json` in the same handoff, with prerequisites and
budgets. Build/preflight before a live experiment. Isolate a failing scenario
before another full affected selection, retaining mandatory safeguards.

Signal a validation checkpoint to all workers, including list/static validators.
Freeze the runner's implementation fingerprint inputs until it finishes:
`Cargo.toml`, `Cargo.lock`, `godot/project.godot`, and files under `src/`,
`tests/`, `tools/`, `godot/addons/`, and `crates/`. These include untracked
helpers; an agent editing a new `tools/` file can invalidate a long run.
Read-only analysis and writing `notes/`, `docs/`, or ignored `output/` may
continue, with evidence prose reconciled after validation. Measure
build/setup/execution/cleanup separately. Set realistic declared
budgets from evidence, keep cleanup time, and count timeouts as failures. Reuse
the passing 50-case save baseline; rerun affected checks as changes require,
then the complete current affected selection on a stable tree. Do not repeatedly
run unrelated Godot/gameplay suites for static catalogue additions.
Before a disposable staged-tree build, check free space and quota for its
separate Cargo target, compile only the affected binaries, and remove only
that build's owned temporary directory on failure. A broad `--no-run` attempt
on the first save-core checkpoint hit `/tmp` quota after a scoped offline
`cargo check -p astrolabe -p orrery` passed; that storage failure was not a
source or test failure.

Current exact save-selection recipe: `python3 tools/test.py --area save --local
--offline --timeout 960` selects 50 cases and declares a 960-second save job
budget, including build/discovery and cleanup reserve. The default
development budget remains 720 seconds for other focused areas. The local selection
requires approved access to private ptrace, authenticated display sockets,
and private audio; request that access through the normal command approval
path before running. A 480-second run stopped after 42 passes and 465.139
seconds including build/discovery; its unfinished cases were not failures of
the implementation. The latest frozen 50-case elevated selection passed **50/50** in
643.233 seconds, including 20 mandatory safeguards, with unchanged source, no timeout or leaked children and
complete cleanup (`TestResults/20260928T121150.841596Z-development.log.json`).
Formatting, Clippy and the locked release build passed before that run. The
earlier frozen checkpoint also passed 50/50 in 587.276 seconds
(`TestResults/20260928T111438.680887Z-development.log.json`).
It includes rooted checkpoint and gauge joins, all seven original-game save
scenarios, the named-level mode-5 return case, native Godot persistence and
mandatory safeguards. The new named-level case also passed its focused two-variant run in 101.826 seconds with 28 mandatory safeguards. The outgoing state-6 transition overwrites `HYPE/manoir.sav`; the controlled case reinserted only the opposed byte into an isolated copy after departure, then observed mode-5 return applying the named file. See `output/savegame-semantic-map/named-level-mode5-probe-20260928/final-report.json`. The later map-slot and lever-write annotations passed Python compilation and full catalogue regeneration but were added after the frozen run; they do not represent additional live tests. A later reverse-owner merge passed the exact ignored local join test and a fresh CLI join byte-identical to the canonical 1,271-record JSON, with all 201 StandardGame owners unchanged; see `output/savegame-semantic-map/reverse-merge-validation.md`. Do not infer a passing suite from a
default-sandbox preflight run that lacks these runtime capabilities.
That restricted run (`TestResults/20260928T101112.874990Z-development.log.json`)
finished 35 passes and 14 runtime-environment failures because private
process inspection and audio were denied; the subsequent approved run passed.
Give the next live-suite owner the exact `python3 tools/test.py --area save
--local --offline --timeout 960` command, 960-second budget and normal elevated approval
requirement before execution. For individual original-game probes, use the
existing `prepare_isolated_config` plus `GameSession::start_session_with_bridge`
fixture path; the Orrery MCP service does not accept a replacement save root.

## Deliverables and reporting

- Maintained repeatable commands producing the registration index, automatic
  ownership joins, semantic catalogue and coverage/unresolved reports.
- A source-backed meaning map for the declared corpus, extending beyond the
  supplied fixture to the available English-build registration/content scope.
- Reconciled coverage by file occurrence, distinct contextual identity, family
  and array contents: owned, semantically explained, runtime-corroborated,
  inferred and unknown. Do not inflate coverage through duplicated files or
  label an unknown array explained because its containing object is known.
- Focused evidence for ambiguous meanings, save/load selection and precedence;
  links to retained experiments and explicit unverified discovery branches.
- Stable findings in `docs/openspace/save-games.md`; dated progress and a compact
  current checkpoint in `notes/savegame-semantic-map-progress-20260928.md`.
  Essential authored family investigations and negative findings belong in
  linked, versioned `notes/`, with maintained reproduction helpers and explicit
  private-evidence dependencies. Keep raw game-derived catalogues/captures in
  ignored output directories.
- Any bounded Godot/AI API, Astrolabe or Orrery improvements made from these
  findings, with their affected behavior, validation and remaining limits
  recorded separately from semantic coverage. Record larger opportunities as
  follow-ups instead of expanding this assignment's completion contract.

Report progress as changes in explained meaning, not just new tooling, agents
or test counts. Maintain explicit remaining work and next discriminating checks.
Do not call the comprehensive goal complete at the first ownership report or
while entries are merely classified as unknown. The
[2026-09-28 revised estimate](savegame-estimate-update-20260928.md) supersedes
the earlier 8–16-hour fixture estimate: allow 40–80 additional active hours for
comprehensive supplied-save meanings and 100–200+ hours total from the pause
checkpoint for the broader declared corpus. These low-confidence planning
ranges are not additive, guarantees or automatic stopping limits. Reassess
after the first 4–6 active hours of an eventual resumption using new verified
meanings and resolved semantic blockers.

The task and its hourly review are paused at the user's request. If resumed,
the review should
inspect saved reports and offer concise evidence-based feedback, without
spawning review workers or competing for the game session.
