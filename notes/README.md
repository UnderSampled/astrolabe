# Working notes and records

`notes/` holds information that changes as work proceeds: dated observations, implementation status, experiments, plans, reviews, handoffs, and historical designs. Use `docs/` for durable explanations of OpenSpace knowledge and Astrolabe, Rete, and Orrery design.

[`historical/`](historical/) preserves material from superseded or archived approaches. Historical instructions and completion claims do not define current Rust requirements.

## Common records

The [current roadmap](remaining-work.md#current-roadmap--2026-09-27) groups ongoing
and upcoming work: recording/replay, loading efficiency, manor gameplay parity,
graphics, savegames, UI and Rete coverage. Its dated scope takes precedence over
older priority statements in the historical checkpoints below.

Ready-to-use Sol implementation handoffs: [Godot loading efficiency](loading-efficiency-handoff-20260927.md),
[Orrery recording/replay with the external BDD suite](orrery-bdd-handoff-20260927.md),
and [save-game investigation and first persistence slice](savegame-investigation-handoff-20260927.md).
The follow-on [comprehensive save-game semantic mapping plan](savegame-semantic-map-handoff-20260928.md)
targets record meanings and corpus coverage beyond that first slice.
It also permits bounded improvements from those findings across Godot, its AI
API, Astrolabe and Orrery while keeping mapping primary; essential authored
research belongs in linked notes rather than only in ignored generated output.
Its [current mapping checkpoint](savegame-semantic-map-progress-20260928.md)
reconciles record occurrences, source allocation/actor joins and unknown families.
The [durable save evidence ledger](savegame-semantic-evidence-20260928.md)
records verified meanings, source anchors, negative results and local evidence hashes.
They are independent assignments, not a prescribed order for the roadmap.
The [dated Godot loading results](godot-loading-efficiency-2026-09-27.md)
record before/after menu, rendered-frame, import and original-game timings.

Start with the [[notes/implementation-progress|implementation report]] for current demonstrated coverage. Open work is tracked in the [[notes/godot-gameplay-plan|Godot gameplay plan]] and [[notes/realtime-comparison-plan|live comparison plan]]. The [[notes/orrery-live-session-2026-09-23|live Orrery session]], [[notes/behavior-suite-2026-09-23|movement BDD checkpoint]], [[notes/godot-movement-bdd-2026-09-23|Godot movement continuation]], [[notes/camera-research-2026-09-24|deferred camera research]], and [[notes/environment-fog-evidence|fog evidence]] preserve dated observations; [[notes/testing-history|testing checkpoints]] retain earlier suite results. Topic-specific [[notes/texture-validation-2026-09-12|texture timings]], [[notes/material-transparency-evidence|pixel comparisons]], and [[notes/scene-hierarchy-checkpoint|scene hierarchy status]] preserve detailed evidence that changes over time. Material inherited from retired approaches is indexed under [`historical/`](historical/), including the [historical-note index](historical/README.md).
Start with the [[notes/implementation-progress|implementation report]] for current demonstrated coverage. Open work is tracked in the [[notes/godot-gameplay-plan|Godot gameplay plan]] and [[notes/realtime-comparison-plan|live comparison plan]]. The [[notes/orrery-live-session-2026-09-23|live Orrery session]], [[notes/behavior-suite-2026-09-23|movement BDD checkpoint]], [[notes/godot-movement-bdd-2026-09-23|Godot movement continuation]], [[notes/camera-research-2026-09-24|deferred camera research]], and [[notes/environment-fog-evidence|fog evidence]] preserve dated observations; the [hidden-cheat investigation](hype-cheat-exploration-2026-09-12.md) and [cheat-save experiment](cheat-save-experiment-2026-09-24.md) record source-game tests. [[notes/testing-history|Testing checkpoints]] retain earlier suite results. Topic-specific [[notes/texture-validation-2026-09-12|texture timings]], [[notes/material-transparency-evidence|pixel comparisons]], and [[notes/scene-hierarchy-checkpoint|scene hierarchy status]] preserve detailed evidence that changes over time. Material inherited from retired approaches is indexed under [`historical/`](historical/), including the [historical-note index](historical/README.md).
