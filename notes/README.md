# Working notes and records

`notes/` holds information that changes as work proceeds: dated observations, implementation status, experiments, plans, reviews, handoffs, and historical designs. Use `docs/` for durable explanations of OpenSpace knowledge and Astrolabe, Rete, and Orrery design.

[`historical/`](historical/) preserves material from superseded or archived approaches. Historical instructions and completion claims do not define current Rust requirements.

## Common records

The [Manor intro integration wrap-up](intro-integration-wrapup-2026-10-08.md)
records the rebuilt actual root game, bounded startup results, critical armor
and camera blockers, parked peer commits and the proposed next steps.

The [fork-first integration ledger](fork-first-integration-2026-10-08.md)
records the source fork, checkpoint merge, accepted queue, reconciliations,
explicitly held work, and validation evidence.

The [armor catalog and AI port checkpoint](armor-ai-port-2026-10-07.md)
records the isolated Fix-owned armor implementation, its source boundaries,
and focused validation limits.

The [in-engine manor opening checkpoint](cinematic-opening-source-2026-10-03.md)
records typed/shared route conversion, the original opening actor/camera capture,
the validated native Gogoud component, explicitly deferred wind and remaining
startup blockers.

The [intro camera source-first checkpoint](intro-camera-source-first-2026-10-07.md)
records exact MainActor binding, viewport registration, target reset, mapped
mode-zero inputs, and the still-unsupported controller smoothing and sector
history.

The [Crossbow fixes and evidence](crossbow-fixes-2026-10-02.md) continue the
October 1 handoff with fixture recovery, typed original contact identity,
native reaction-zone contact and spent-ammunition reload checks.
The [free-flight followup](crossbow-free-flight-2026-10-02.md) adds native
counter-expiry/aiming regressions and records the original missed-shot and
enemy-damage fixture gaps.

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
The [pause checkpoint](savegame-pause-checkpoint-20260928.md) records the stopped
state; the [revised estimate](savegame-estimate-update-20260928.md) replaces the
initial timing assumptions using observed semantic progress and remaining gaps.
The [durable save evidence ledger](savegame-semantic-evidence-20260928.md)
records verified meanings, source anchors, negative results and local evidence hashes.
They are independent assignments, not a prescribed order for the roadmap.
The [dated save results](savegame-results-2026-09-27.md) record the structure
catalogue, source mana reload proof and native Godot persistence scope.
The [dated Godot loading results](godot-loading-efficiency-2026-09-27.md)
record before/after menu, rendered-frame, import and original-game timings.
The [FFV1 and Ut Video evaluation](video-codec-evaluation-2026-09-27.md)
benchmarks both codecs on a retained Orrery capture.
The [Orrery private audio record](orrery-audio-recording-2026-09-27.md)
documents the short full-session proof, routing isolation, and timing limits.

Start with the [[notes/implementation-progress|implementation report]] for current demonstrated coverage. Open work is tracked in the [[notes/godot-gameplay-plan|Godot gameplay plan]] and [[notes/realtime-comparison-plan|live comparison plan]]. The [[notes/orrery-live-session-2026-09-23|live Orrery session]], [[notes/behavior-suite-2026-09-23|movement BDD checkpoint]], [[notes/godot-movement-bdd-2026-09-23|Godot movement continuation]], [[notes/camera-research-2026-09-24|deferred camera research]], and [[notes/environment-fog-evidence|fog evidence]] preserve dated observations; [[notes/testing-history|testing checkpoints]] retain earlier suite results. Topic-specific [[notes/texture-validation-2026-09-12|texture timings]], [[notes/material-transparency-evidence|pixel comparisons]], and [[notes/scene-hierarchy-checkpoint|scene hierarchy status]] preserve detailed evidence that changes over time. Material inherited from retired approaches is indexed under [`historical/`](historical/), including the [historical-note index](historical/README.md).

The [[notes/intro-initialization-2026-10-07|intro initialization checkpoint]] records the source-backed Hype armor selection implementation and the distinct camera/statue blockers still open.

The [loading-presentation checkpoint](loading-presentation-2026-10-07.md)
records the decompiled window-caption stages, the missing dedicated loading art,
the observed threaded-resource progress contract, and native Control ownership.
The [AI audio fade trace](ai-audio-fade-static-trace-2026-10-08.md) records the
source-reached voice/effects procedures, matching executable ranges, and the
Godot channel and lifetime mapping.
Start with the [[notes/implementation-progress|implementation report]] for current demonstrated coverage. Open work is tracked in the [[notes/godot-gameplay-plan|Godot gameplay plan]] and [[notes/realtime-comparison-plan|live comparison plan]]. The [[notes/orrery-live-session-2026-09-23|live Orrery session]], [[notes/behavior-suite-2026-09-23|movement BDD checkpoint]], [[notes/godot-movement-bdd-2026-09-23|Godot movement continuation]], [[notes/camera-research-2026-09-24|deferred camera research]], and [[notes/environment-fog-evidence|fog evidence]] preserve dated observations; the [hidden-cheat investigation](hype-cheat-exploration-2026-09-12.md) and [cheat-save experiment](cheat-save-experiment-2026-09-24.md) record source-game tests. [[notes/testing-history|Testing checkpoints]] retain earlier suite results. Topic-specific [[notes/texture-validation-2026-09-12|texture timings]], [[notes/material-transparency-evidence|pixel comparisons]], and [[notes/scene-hierarchy-checkpoint|scene hierarchy status]] preserve detailed evidence that changes over time. Material inherited from retired approaches is indexed under [`historical/`](historical/), including the [historical-note index](historical/README.md).
