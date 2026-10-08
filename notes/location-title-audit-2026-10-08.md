# Astrolabe location/era title: source cue and native presentation

2026-10-08. Initial isolated title slice based on `bac48e1`, branch
`codex/location-title-20261008`. The intro integration remains lead-owned.
The initial slice adds a tested native presentation component; it does **not** yet make
the source title cue executable in the game. Only the two new UI files, the two
dedicated test files, and this note are changed.

The integrated follow-up at the end of this note supersedes those initial
registration gaps and the prototype's no-completion restriction.

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

## Integrated follow-up on edb7704

The existing `e9296e88f92940d0e73ac206820e041cacd19181` and
`f7dc29d915c57053392d156babf1fb6adf5d7795` commits were preserved on
`preservation/location-title-before-edb7704`. Merge
`22cd788348928b63e99c2cede8b22beed92db3c8` brings the isolated branch onto
requested main base `edb7704b2597c26c78086acad2f58eaba1db7a78`, without conflicts.
No main checkout or remote was changed.

The title is now in the production paths: the OpenSpace scene importer admits
English Astrolabe's language package/catalogue; its real `astrolabe_txt2` becomes
a normal dialogue root; Rete persists it and typed `Expression::GameState`; the
public exporter writes its native Translation, font/stencils, line Resources
and complete compiled model659. Saved level Screen composition already adopts
all dialogue roots and source Brain bindings, so no new level-load trigger is
needed. Support interface 73 packages the title scene/script/UID and instances it
under Screen. Rete schema 56 includes the new typed query and useful line anchor.
Both registrations include exact release/development test-inventory entries.

The source catalogue corrected two prototype assumptions:

- Line+16 is a pointer to a 16-byte drawing anchor, not a letter effect. Static
  0056cc70 forwards it to 0056c8e0, which passes its first two words to FON
  horizontal/vertical alignment and adds its X/Y offsets. 005bb5a0 measures
  first-to-last glyph centers for left/center/right; 005bb650 uses unchanged,
  half or full space-glyph half-height. The useful model retains selectors and
  offsets, never the original pointer. Native font metrics provide placement,
  including prefix alignment during reveal. Actual title line anchors are
  `[1,1] / [1.8,-0.2]` and `[1,1] / [1.9,-0.4]`.
- The real item has a completion reference to model 659 normal 1. Both lines are
  untimed (`3.4e38`), flags0, and unvoiced; the owner is actor 91. The prototype's
  rejection of every completion reference would therefore omit the actual
  title. Title Select now emits the same pending-completion signal as ordinary
  dialogue. Shared Cond120/Proc330 consumes it through the bound compiled Brain;
  Cond121 leaves it pending. Source End clears it without invoking completion.

Screen dispatches Position/Run/Scroll/Select/End/IsOver to the presentation
owner, with Scroll a no-op for this single-answer title. The shared MainActor
speech gate remains in Screen. Ordinary End and title End animate closing;
level adoption/release and teardown cancel both immediately. Replacement Run
validates its target before cancelling another presentation. No UI-owned
200/3000ms timers or authored every-level popup were added.

Func195 uses the actual `game/demo.gd` session getter, with source selectors
0..8. An exhaustive byte-mask check validates its mapping and null for combined
masks; compiled callers diagnose the source's unrepresented stale-result case
before using it. The test never recreates an AI evaluator result cache.

### Verification and limits

Owned ignored evidence is under `output/location-title-evidence/integration/`.
The five dedicated title tests passed in `connected-callback.log`: 100.73s total,
32.75s test execution, 0 leaked descendants. They include source actor/Brain
selection and complete model Rete parity; actual catalogue/anchor/completion
Rete parity; public selected title+AI export; the unchanged compiled cue at
200/3000ms for selector 2 and selectors 0/1/3..8; unconditional End/deactivation;
shared Select/completion/speech gates; animated close; adoption, map cancellation
and fresh native physics startup/re-entry. The session test inherits the actual
game methods while suppressing level loading. No source cue is excerpted or
rewritten for that acceptance test. A separate synthetic model exercises the
same narrow calls and support components without game data.

The actual game's configured Button_Hit input smoke also passed syntax and
runtime checks in 0.66s with no leaked descendants (`input-*.log`). Its fixture
now enters through Screen's production Run dispatch. All temporary projects,
XDG state, logs and generated source assets remain owned and ignored. Earlier
failed attempts remain recorded: the initially assumed effect/callback
restrictions omitted the item; a strict floating-point boundary in a physics
observation was corrected; sandboxed editor loopback IPC failed before running
through the reviewed local execution route. Those attempts are not passes.

Full source-level exports, original-game sessions, and complete game/intro runs
were intentionally excluded. Original pixel/cadence equivalence and full game
route acceptance remain lead-owned validation, not claims of these focused
checks. The production implementation no longer awaits title registration or
catalogue/Func195/dispatch wiring.

The focused follow-up selected 33 checks, with 352 manifest entries explicitly
excluded from this bounded slice. All 33 passed across the recorded executions
and exact retries (`focused-outcome.json`): 21 Astrolabe checks and 12 Orrery
synthetic-process/contract safeguards. Every declared always/gameplay safeguard
was included. The four supervisor/selection/nextest/recording Python check groups
also passed. Root execution took 94.07s; the Orrery job took 72.44s including
rebuild, with 2.26s test execution. Five sandbox-blocked ptrace/ELF32 cases passed
on the reviewed retry (15.70s including setup, below 1s test execution). No
original-game backend was launched. No owned descendant leaked or test timed out.

The new Func195 byte-roundtrip test initially used the wrong synthetic Then
token. Correcting that fixture yielded an exact one-test pass; its 105.75s
supervised stage included build-lock wait/recompilation and near-zero execution.
No production behavior was changed for that retry. `cargo fmt --all --check` and
`git diff --check` passed. `cargo clippy --locked --offline --all-targets -- -D
warnings` passed in 13.87s, and `cargo build --release --locked --offline` passed
in 84.87s. These Cargo commands select the root default member, not all workspace
targets.

Whole-workspace discovery on this edb7704-based checkout failed on the unchanged
Orrery comparison Perso fixture, then target-specific discovery exposed 28 base
manifest gaps. This slice corrects the stale maintained-support test registration
and registers the existing setter test needed beside Func195; the other 26 gaps
are recorded without unrelated repairs. Exact selected inventory was validated.
Kip reports newer main separately repairs workspace discovery/lints; this worker
has neither merged those changes nor claimed release-matrix acceptance.

### Integration ownership

Merge the branch tip, preserving the original two title commits and the clean
edb7704 merge. Do not cherry-pick merge22cd788 as an ordinary implementation
commit. A linear alternative from edb7704 or its descendant is the original
e9296e8, then f7dc29d, then the final integration implementation commit.

At inspected main `98a3d2e1f0aa12a9b8b48f79e7e859be321a1ddb`, the common merge
base remains edb7704. Changed paths shared with that newer main are exactly
`godot/addons/astrolabe/version.json`, `src/godot/support.rs`,
`src/rete/mod.rs`, and `tools/test-manifest.json`. Both branches independently
choose schema56/interface73; Kip owns the combined versions and registration
reconciliation. Keep the newer inventory/loading assets and this title's three
support registrations together. This branch does not edit inventory animations.
All changed files are available in the committed diff; source game data,
converted resources, captures and test outputs remain ignored.
