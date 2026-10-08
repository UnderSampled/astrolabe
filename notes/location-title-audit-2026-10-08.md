# Astrolabe location/era title: source cue and native presentation

2026-10-08. Optional, isolated title slice based on `bac48e1`, branch
`codex/location-title-20261008`. The intro integration remains lead-owned.
This slice adds a tested native presentation component; it does **not** yet make
the source title cue executable in the game. Only the two new UI files, the two
dedicated test files, and this note are changed.

## Source owner and cue

The imported Astrolabe actor is `iDummy_Astrolabe_Texte`,
`level:/persos/world/0/91`, with Brain model `level:/ai/models/659` (Level 17:1
allocation 659). Its normal current, previous, and initial selections are all 0.
The dedicated local test imports that actual AI, checks the following order,
and preserves the complete model through a Rete round trip:

| Normal behavior | Source statements, in order |
| --- | --- |
| 0 | Wait 200 ms; PositionDialog(160,70); if GetGameState()==2, RunDialog("astrolabe_txt2",1); change to normal 1 without pause |
| 1 | Wait 3000 ms; EndDialog; deactivate SelfActor |

The behavior change, later EndDialog, and deactivation are unconditional, even
when the opening predicate is false. The UI must not own these waits or replace
this actor with an every-load popup. Re-entry is governed by the source Brain and
session lifecycle. The name `iDummy_Manoir_Texte` alone does not establish a
second cue: its Brain refers to Fix model 688/HypeStatue.

The earlier title investigation joined `astrolabe_txt2` to English source text
"The Astrolabe" / "(Era of Taskan II)": style 4, one answer, two untimed lines,
no voice. That prior catalogue evidence is retained here, but this slice does
not retest the localized title join: at this base, catalogue import runs only
for Manor. Neither the component nor its committed fixture contains those
source strings or a guessed era.

Raw AI IDs are EndDialog 324, RunDialog 325 (`SL`), ScrollDialog 326,
SelectDialog 327, PositionDialog 328 (`LL`). Historical readable dumps were
off by one. The full original dispatcher 00543c00 confirms EndDialog 324 clears
the script-dialog gate and calls EndDraw(1). This remains shared dispatch work.

Full GetGameState 005398b0 in the independent `hype-decomp` reference returns
selectors 0,1,2 for masks 0,1,2 and selectors 3..8 for masks 4,8,16,32,64,128.
Other masks >=3 leave both the result kind and payload untouched. At this base
SetGameState and the shared mask exist, but expression 195 is still preserved as
an Unsupported fragment. The test explicitly reports this gap; it is not a
passing source-gameplay predicate test.

## Native resource and presentation contract

`location_title.tscn` contains one native TextureRect and native Label children
created from the maintained `dialogue_span_view.tscn`. `location_title.gd`
accepts the existing exported DialogueItem and DialoguePresentation resources:
their Translation, FontFile, colors, span offsets and four stencil textures.
It reuses `text_origin`, `span_origin` and `line_spacing`; there is no private
source decoder, fallback font, input dispatcher, speech gate or completion
callback. The supported item is mode 1/style 4, unvoiced, untimed, unflagged,
without a completion behavior, and at most two lines. Invalid input is rejected
without cancelling a live item. Nothing appears on `_ready()`.

Static 0056cf90 and the reviewed PE branch show styles 4..7 draw a single center
material (slot 3), then return before the border/corner sequence. Style 4 uses
width 8 and row factor 2. Corner dimensions are halved as integers before float
conversion; border/center width and side/center height ratios are integer
divisions. For the actual 32x32 stencils, the local rectangle is `(16,16)` with
size `(288,96)`. At source position `(160,70)`, its 640x480 screen rectangle is
`(176,86)` through `(464,182)`. Odd texture dimensions are checked separately.
The existing Screen canvas continues to own viewport scaling.

The full original alpha bodies 0056d860/0056d960 are independently reviewed in
`hype-decomp/src/dialogue/material_alpha.c`; their matching PE spans total
495 bytes. The original operands are float32 ±0.10000000149011612. Opening only
steps when elapsed exceeds 10; closing steps on every draw, with its threshold
gating only timer reset/start. A terminal comparison happens before the
triggering material setter, so that material retains its previous alpha.
Equality is not terminal. The native component uses that float32 step, compares
before storing a rounded float32 alpha, and advances at most once per draw.
It does not reproduce the engine's six-material globals or timer failure state.

The seconds mapping is supported by bounded static callees:
LoadDialog 0056ba50 creates both timers through 0056f090, which calls
CreateTimer(0); ReadTimer 00501ce0 reads the stored type through 00502680, whose
type 0 branch uses `timeGetTime`, and frequency helpers 005025c0/00502620 return
1000 for type 0. Thus this native component uses `.010` and `.002` seconds.
The alpha reconstruction itself does not claim portable x87 or clock/error
equivalence. No fixed fade duration is asserted: ordinary draw cadence matters.

For the unvoiced flags 0 page, 0056cc70 displays a prefix starting at one
character, then advances one character or span/line boundary after elapsed >2,
without catch-up. The text timer starts with RunDialog and accumulates during
the independent panel opening. Native Label `visible_characters` implements
this narrow presentation. Source 0056e510 ignores Select while opening; the
first reveal-time Select completes the page and a later one ends it. End clears
text immediately, then closes the panel. Cancellation and scene teardown clear
the component immediately; repeated run replaces the previous presentation.

## Focused evidence

All evidence is local and ignored under
`output/worktrees/location-title-20261008/output/location-title-evidence/`.
The worktree has its own Cargo target, temp projects, XDG state, logs and private
Xvfb display. No original-game or full game/intro run was performed.

| Check | Result and scope |
| --- | --- |
| Three dedicated tests, including ignored local cases | Passed; latest rendered run 19.09 s total, 15.44 s test execution, 0 leaked children (`focused-verified*`) |
| Native parser/runtime | Checks run only after syntax preflight; native resources save/reopen, strict alpha threshold/order, sequential text, Select, cancellation, repeated entry, position refresh and 640/1280 viewport scaling |
| Existing public conversion | Exports only the Manor presentation resource root, including shared Playmo_Font2_0 font/stencils; no level scene, voice or AI export |
| Actual Astrolabe import/Rete | Checks the real actor, Brain selection and 200/3000 ms source statement order; reports retained unsupported 195 and missing catalogue |
| Existing UI regression | `native_ui_timing_layers_callbacks_and_adoption` passed; 0 leaked children (`existing-ui-guard*`) |
| Focused Clippy | `--test location_title -- -D warnings` passed (`clippy*`) |

After tightening the source selection and behavior-transition assertions, the
exact local source test passed again in 1.46 s total (`source-cue-final*`).
Focused Clippy, `cargo fmt --all --check` and the staged diff whitespace check
also passed. The complete dedicated selection is
`cargo test --locked --offline --profile validation --test location_title -- --include-ignored --test-threads=1 --nocapture`;
its local cases need an explicit game directory. Rendered capture additionally
uses `ASTROLABE_GODOT_HEADLESS=0`, an owned display and an absolute
`ASTROLABE_LOCATION_TITLE_CAPTURE` path.

The rendered capture `converted-stencil-title-verified.png` was inspected: the
shared source font and single dark center stencil render at the expected
rectangle with synthetic literal two-line text. It is not an actual title-text
capture or original-engine pixel comparison. The renderer is Godot 4.7.2,
compatibility OpenGL/Mesa llvmpipe. Its V-Sync warning did not produce script or
runtime errors.

Failed preflight and earlier checks remain in the evidence directory. Initial
GDScript inferred-type errors were fixed before source conversion. Godot's
default log/settings paths were moved to owned XDG directories; the exporter
editor IPC and private rendered check used the reviewed execution route. A
frame-count-based playback expectation was replaced with a bounded 1000 ms
deadline. These failed attempts are not counted as passes.

## Minimal lead-owned integration proposal

1. Add typed Func 195 import/model/Rete/compiler support and a shared game-state
   getter using the source selector mapping. Preserve explicit diagnostics for
   the unrepresentable stale-result case; do not invent a raw-mask fallback.
2. Admit the actual Astrolabe language catalogue and narrowly support style 4
   through the existing model/import/export dialogue path. Preserve source
   translations and presentation references; do not manufacture era strings.
3. Add this component as a sibling presentation under the shared Screen canvas.
   Route source Position/Run/End/Select/IsOver through the active presentation
   owner. Keep the common script-dialog/speech/input/completion contracts with
   the lead-owned framework. In particular, the source's unconditional End must
   retain shared dispatch semantics. Cancel titles on adoption/release, map
   change and teardown; ordinary EndDialog uses animated close.
4. Register the maintained files and focused tests in the support package/test
   inventory, with the lead's interface/version update. The fixture's explicit
   staging is temporary until that registration lands.

The integrated acceptance check must execute the unchanged source cue with
selector 2 and other selectors, verify 200 ms before Run and the subsequent 3000 ms
before End/deactivation, and cover map cancellation/re-entry. This isolated
slice cannot establish those checks while the shared files remain lead-owned.
No full-area, release, original-engine, localization catalogue or intro
acceptance is claimed.
