# Manor first-visit integration audit, 2026-10-08

This audit starts at clean `209f2e1` and follows the unchanged imported English
Manor scripts. It is a dependency map, not a claim that the complete opening has
played successfully in Godot. The actual project remains `godot/project.godot`,
whose authored bootstrap opens New Game through `game/new_game.tscn`.

## Connected schedule

Actor references below are useful model identities. Their source allocations are
resolved by the OpenSpace reader; runtime code must use the named actor bindings.

| Actor | Model | Role |
| --- | --- | --- |
| `level:/persos/world/0/0` | `fix:/ai/models/28` | Hype, dialogue ownership, transformation and gameplay handoff |
| `level:/persos/world/0/2` | `fix:/ai/models/437` | Native source camera |
| `level:/persos/world/0/3` | `level:/ai/models/154` | World setup and timed shots |
| `level:/persos/world/0/5` | `level:/ai/models/210` | Gogoud's approach, dialogue and departure |
| `level:/persos/world/0/28` | `level:/ai/models/294` | Statue transformation and dialogue triggers |
| `level:/persos/world/0/45` | `level:/ai/models/365` | Traveling camera eye |
| `fix:/persos/15` | `fix:/ai/models/1` | Shared first-visit flags |

World normal2 takes the first-visit branch when Global147 is zero. It requests
`sound:/Sound/playmo_b.lcb` bank36/member0 (`Play_Intro2`), fades the screen,
toggles letterbox, initializes the first-visit inventory/life/magic/name state,
and places Gogoud, Hype and the statue. Opening targets18/19 and dummy33 use
waypoints86/84/82, camera mode3, and waits3600/4500ms before World normal3.

Gogoud13 enters0, follows its approach route and calls the first dialogue.
Completion callbacks advance Gogoud through normal4/5/6, with animation-frame
guards and World shot triggers in8/7. The required dialogue chain is
`gogood_texte1`, `1a`, `1b`, `1c`, `2`, `2a`, plus the Hype transformation lines.
The first dialogue alone does not establish the remainder of this route.

Gogoud normal1 sets its dsg4 flag. World normal3 then runs ordered waits
1100/1700/1400/4500/3000/2000/2100ms with targets21/43/21/25/21/31/29 and the
source eye waypoints. It reads traveling actor45's position, waits700ms, switches
travel365 to normal2, and returns World to normal0.

Travel2 copies its own position into the camera eye every tick and traverses
route0. At waypoint135 it selects camera mode6 and sets statue28 dsg0.
Mode6 preserves the prior camera orientation while translating the eye. The
statue's action sequence switches Hype through69/16,67/66 and70, with the
statue's normal5/9 coordinating further shots and travel365 normal4→3. Travel3
traverses route1; waypoint129 sets Gogoud dsg1 and releases its next dialogue.

The statue's initial normal0 and normal4 change the sound listener using the
source microphone procedures. Normal8 sends an ordinary sound through an Ultra
MainActor receiver before `hype_texte1`; normal9 triggers `hype_texte2`.

Gogoud normal12 sets Global147, toggles letterbox off, turns incrementally,
performs two GoToPosition calls and one GoToPositionXY, targets MainActor,
selects camera -1, clears Hype56, restores the microphone and deactivates itself.
Hype normal48 subsequently selects gameplay normal4/reflex3. Acceptance must
observe these source-owned transitions and input-driven movement afterward.

## Remaining dependency closure

The source audit identified these connected gaps before any new full export:

- Music event/theme decoding, dependencies, native initial playback and Ultra
  sound dispatch. Proc129 is nonblocking; null event status is discarded.
- Camera160/163 history/reset semantics, minimum-distance clamping, camera-owned
  displacement/probe state,135's conditional cleanup, and mode6.
- Func38 maximum hit points, evaluated from one original-caller actor operand.
- Ultra Func0 actor position in World/statue shots.
- Travel meta30 GoToNextWPOfWay and Gogoud final meta19/32/33.
- Microphone402/403 and keyword-Me object deactivation.
- Condition120's dialogue completion side effect, distinct from condition121.
- `gogood_texte1c` was rejected for a nonzero line layout scalar. The source
  loader computes intended justification, but exact FON disassembly confirms
  that the shipped renderer never consumes the forwarded eighth argument.
  Retaining `DialogueLine.justification` therefore permits import without
  changing glyph advances. The ordinary native FontFile remains the renderer.

Some unsupported calls lie in input/revisit guards that the passive first visit
does not execute. They remain explicit limitations, and must not be treated as
evidence that every compiled branch is implemented. The earlier reference run
took roughly201 seconds to reach player control without skipping dialogue;
this is a bounded-run guide, not a calibrated timing assertion.

## Evidence and ownership

Ignored source inspection evidence is under
`output/integration-resume-20261008/source-audit/`: canonical actor/model/text
values, diagnostics and readable schedule excerpts. Initial import returned201
actors,71 AI models and7 supported dialogues. That seven-dialogue count exposes
the missing1c import and cannot satisfy the existing nine-example localization
contract. The justification fix is checked against that existing contract.

Music and camera edits are isolated in source-only checkouts. The music worker
is the sole writer of the shared AI files until its audio commit is handed back.
The integration lead owns main, schema/support versions, manifest coordination,
remaining disjoint integration work and root-project acceptance. The original-C
worker supplies static engine evidence; no original game has been run here.

All blinded font comparison worktrees remain outside this audit. No assets,
original saves or reference submodules are modified by source inspection.
