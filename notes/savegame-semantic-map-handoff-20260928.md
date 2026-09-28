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

One owner controls live-game sessions. Use Orrery's verified Gamescope headless/
private Xwayland path, authenticated Xvfb for rendered Godot checks, and private
game-only audio. Route launch, physical input and capture consistently; never
fall back to desktop display/audio. Preserve the private installation, saves and
unrelated processes. Use isolated copies, deadlines and owned cleanup. Before
launch, validate keys through the actual parser, fixture/profile hashes, renderer
and audio prerequisites. Keep failed-run evidence outside temporary cleanup.

Use fast synthetic/offline checks for joins, ambiguity, context isolation and
coverage accounting. Test meaningful negative cases and family generalization;
do not generate a redundant test for every labelled row. Register added/renamed
tests in `tools/test-manifest.json` in the same handoff, with prerequisites and
budgets. Build/preflight before a live experiment. Isolate a failing scenario
before another full affected selection, retaining mandatory safeguards.

Signal a stable-tree validation checkpoint to all workers, including list/static
validators; no covered source edits until it finishes. Read-only analysis can
continue. Measure build/setup/execution/cleanup separately. Set realistic declared
budgets from evidence, keep cleanup time, and count timeouts as failures. Reuse
the prior passing 42-case save baseline; rerun affected checks as changes require,
then the complete current affected selection on a stable tree. Do not repeatedly
run unrelated Godot/gameplay suites for static catalogue additions.

Current exact save-selection recipe: `python3 tools/test.py --area save --local
--offline --timeout 960` selects 50 cases and declares a 960-second save job
budget, including build/discovery and cleanup reserve. The default
development budget remains 720 seconds for other focused areas. The local selection
requires approved access to private ptrace, authenticated display sockets,
and private audio; request that access through the normal command approval
path before running. A 480-second run stopped after 42 passes and 465.139
seconds including build/discovery; its unfinished cases were not failures of
the implementation. The frozen 50-case elevated selection passed **50/50** in
587.276 seconds, with unchanged source, no timeout or leaked children and
complete cleanup (`TestResults/20260928T111438.680887Z-development.log.json`).
It includes rooted checkpoint and gauge joins, all seven original-game save
scenarios, the named-level mode-5 return case, native Godot persistence and
mandatory safeguards. The new named-level case also passed its focused two-variant run in 101.826 seconds with 28 mandatory safeguards. The outgoing state-6 transition overwrites `HYPE/manoir.sav`; the controlled case reinserted only the opposed byte into an isolated copy after departure, then observed mode-5 return applying the named file. See `output/savegame-semantic-map/named-level-mode5-probe-20260928/final-report.json`. The subsequent partial Global annotations passed Python compilation and catalogue reconciliation; they do not represent additional live tests. Do not infer a passing suite from a
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
while entries are merely classified as unknown. The estimated 8–16 hours for
strong fixture coverage and several days for broader coverage are planning
ranges, not guarantees or automatic stopping limits. Refine them using observed
mapping throughput and unresolved families at the first checkpoint.

This task's process review resumes hourly in the planning task. It should
inspect saved reports and offer concise evidence-based feedback, without
spawning review workers or competing for the game session.
