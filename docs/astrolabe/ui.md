# Screen-space UI

The maintained UI lives in `godot/addons/astrolabe/ui/`. Its scenes, theme and
scripts are authored Godot support. Text, voices, fonts and original textures
are converted inputs under ignored `generated/`; content regeneration does not
rewrite the maintained scenes. The screen-space HUD/dialogue slice targets
manor and the source Astrolabe location title; the main menu and level-loading presentation remain separate components.

## Level-loading presentation

`loading/screen.tscn` is a native `CanvasLayer` with editable `Control` nodes.
The persistent `LevelTransition` owner instances it once and emits lifecycle
signals. On `transition_started` the presenter loads the destination's
converted vignette, `generated/hype/levels/<level>/loading.tres`
(`loading/vignette.gd`), before the level scene itself loads.

Each level's source loading screen is its `<level>pgb.bin` Vignette section
(read by 0050ef30; see the [loading-vignette research](../../notes/research/loading-vignette-2026-11-04.md)).
It names a full-screen era image from `Vignette.cnt` (`epoque1`–`epoque4`; manoir,
brigand and toras use `epoque1`) and the bitmap bar `Bar.gf`: a 512x60 frame
state above a fill state. All 37 installed levels place the bar at (60,410) on
the 640x480 screen with a fill track from bar x 50 to 461. Pixels equal to the
GF colour key (black) are transparent for both states, so the backdrop shows
through the bar's corners. The presenter draws the backdrop, the frame strip and
the fill strip clipped left to right, on a 640x480 canvas fitted uniformly and
centred like the main menu, with a black surround. There is no in-frame text,
fade or minimum display time in the source module, and none is added.

The source fill counts loader chunks against the level's `MaxValueBar`. The
port has no such chunk count, so the fill shows the observed fraction of the
requested `PackedScene` and its dependencies, which is the same observable
meaning: how much of the level is loaded. Each source step also over-draws
about one step ahead (asm of 0050d6b0); this is not reproduced, because
Godot's fraction arrives in a few large jumps and would draw far ahead.
`show_phase()` records the session's later restoration phase; with a vignette
it draws no text and leaves the bar as drawn.

Levels without a vignette resource use the editable status group: the
requested level name, phase text and the resource fraction. A load error
remains visible with the supplied message and a dismiss action until dismissed;
scene recovery stays with the transition owner. The window-caption stages of
0043c8e0 are separate from the in-frame art; the
[earlier loading record](../../notes/loading-presentation-2026-10-07.md) covers them.

## Composition and layout

`screen.tscn` is a CanvasLayer named `AstrolabeUI`, at layer 20. Its Control tree
owns HUD, letterboxes, dialogue, location title and a fade ColorRect in source draw order.
Dialogue is drawn above the bars; fade is drawn above dialogue. The camera,
projection and world viewport belong to the existing camera implementation.
ToggleWideScreen changes presentation only: top/bottom rectangles each cover
12.5%, the HUD is suppressed, and the fade rectangle covers the middle 75%.

Positions use the original 640x480 coordinate system. The root Control scales
independently in X and Y to the current viewport, preserving source screen
positions and the fractional bar coverage. Nodes use ordinary Godot layout and
mouse-filter settings; there is no renderer shim. `source_position_dialog`
retains the source position. ItemBox3 selects an unframed two-line page; ItemBox1
selects a framed six-line page regardless of widescreen state. Source items use a generated native bitmap font and the authored 320x192
frame scene. Precise rendered parity remains a New Game comparison gate.

An exported level owns one instance of the maintained PackedScene. The exporter
binds its main actor and converted dialogue resources before saving the level.
The existing support install/update receipt protects authored support edits;
regeneration publishes only generated content. A consumer can inherit the
maintained screen or instance a customized screen in an authored composition.
If an already-ready ancestor child named `AstrolabeUI` implements `adopt_level`,
the generated child delegates to it and hides its own canvas. The authored
screen accepts one active level, borrows its resources and actor bindings, then
releases translations, active dialogue and callbacks when the level exits.
Place that authored screen before attaching the generated level. No second HUD
or dialogue state machine should be introduced by bootstrap code.

## Content and text

`model::dialogue::Dialogue` stores stable name/locale, box style, ordered text
spans, line deadlines, actor owner, a compiled voice-event link and a normal
behavior completion target. Rete persists those values and a shared
presentation reference. Source addresses and relocation decisions remain in
OpenSpace import.

The Manor/Astrolabe adapter reuses SNA/LZO, pointer-file and relocation codecs for English
`.lng/.rtg/.dlg/.rtd/.snd/.rts` inputs. `.dlg` word zero is the name table; word
one is the circular item catalogue. Installed Manor/Astrolabe RTG has header byte 11 but
only two groups; its header meaning is unresolved. A narrowly scoped adapter
recognizes the evidenced Fix/level headers, checks every actual group and
requires all populated language blocks to be covered. The general relocation
reader remains strict. Source text is Windows-1252, decoded to Unicode with
undefined bytes rejected.

Export produces native Translation resources plus dialogue/line/span resources,
with stable keys `hype.dialogue.<name>.<locale>.line<N>.span<N>`. Native Label
instances from the maintained span scene render source items at recorded span
positions; an authored RichTextLabel fallback supports items without extracted
presentation. Both treat text literally, including brackets. TranslationServer registers the
active level's catalogues; an unsupported runtime locale selects the complete
English bundle, including voice, timing and pagination. Missing translations
fall back to that bundle's native Translation. Extraction rejects unevidenced
locales instead of relabeling English data.

This follows Godot's [Translation resources and TranslationServer](https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html).
One shared `DialoguePresentation` asset retains font geometry/UVs/tracking,
normal/emphasis colors and four frame dependencies. It is useful source data,
not an engine font allocation or renderer. Installed Fix GPT stencil roots resolve the
coin/top/middle/side texture inputs through RTP and verified relocations. The
profile checks GPT extent, explicit root slots, names and dimensions; other Fix
layouts require additional evidence. Canonical CNT samples and alpha
are preserved, with the existing semantic color-key conversion used only when
requested by TextureInfo. No extracted image or generated font is committed.

The exporter creates a native [FontFile](https://docs.godotengine.org/en/stable/classes/class_fontfile.html)
with all 133 Windows-1252 glyphs mapped to Unicode. Fractional advances, bearings,
sizes and UV rectangles are retained. Explicit native subpixel positioning avoids
Godot's size-dependent automatic rounding of advances; shaped runs may still
quantize within Godot's native precision. Normal/emphasis fonts have identical geometry
and differ in material color. Font texture UVs retain the source's one-texel
edge extrapolation. No system-font fallback substitutes missing glyphs. A
conversion-time coverage atlas provides the native outline cache for red
emphasis; native controls draw the white glyph over it. Original subpixel
blending and final pixels still need comparison on the pinned renderer.

Generated FontFile text sets `fixed_size` before `font_name`. Godot 4.7's
name setter initializes a size cache, and bitmap character support consults
the first cache. ResourceSaver's default property order otherwise creates an
empty 16px cache during reload before the populated 21px caches are restored.
Export normalization changes only that prerequisite order; glyph data remains
native and unchanged. Resource validation checks all 133 cached characters.

`dialogue_frame.tscn` uses nine TextureRects. The center stretches; top/bottom
repeat eight 32px tiles and sides repeat four, with mirrored corners. The authored
fallback panel and theme remain editable. `dialogue_presentation.gd` calibrates
source font units to 640x480 coordinates. Each span starts at the center of its
first glyph in the original draw API, so its native label receives a half-first-
glyph origin adjustment. Lines retain fractional 0.3-font-unit spacing. This
preserves the API's screen-space result using Godot font resources and controls.

## State and the AI bridge

The existing compiled Behavior resolves `AstrolabeUI` through scene ancestors.
Typed source opcodes map to the maintained presentation methods; missing UI,
unsupported modes or missing item/owner bindings stop the reached compiled flow
with a diagnostic. There is no runtime source opcode interpreter.

| Installed source API | Native method / effect |
| --- | --- |
| Proc_ToggleWideScreen, 323 | Toggle global presentation state |
| Proc_EndDialog, 324 | Close active presentation and clear its pending callback |
| Proc_RunDialog, 325(name, mode) | Select localized item and start its existing SoundEmitter event |
| Proc_ScrollDialog, 326(direction) | On a question page only: move the selected answer, wrapping (0056f3a0); plays SoundEffects:Dialog entry 2 (Play_InvDepl1), restored from the eighth DLG Fix GPT root |
| Proc_SelectDialog, 327 | Advance a page or finish |
| Proc_PositionDialog, 328(x, y) | Set source screen-space position |
| Proc_DLG_ChangeComport, 330 | Explicitly apply a pending owner/behavior callback |
| Cond_DLG_IsDialogOver, 121 | Query completion after closing presentation |

The native screen routes the configured `Button_Hit` action to
`Proc_SelectDialog` only while a dialogue item is active. This lets the starter
project's Space binding advance source dialogue pages and complete their
pending callbacks through the same presentation API.
The screen and compiled Hype rules share each physical input edge, so a press
advances one page even when both observe it. The source `red_arrow` stencil is
imported through its shared presentation asset and exported as a native
TextureButton. Completed nonfinal pages show it at the source style's pixel
anchor; clicking selects the page through the same dialogue API. Final finite
speech pages complete through their speech holder without requiring this arrow.

The dialogue node owns page index, elapsed time and voice-player reference.
Source fades retain their per-draw alpha steps; authored items may use a native
Tween. The voice event runs through the existing actor SoundEmitter
and compiled native waveform resources. Page deadlines are cumulative. Source
drawing stores the last drawable line's timestamp in source order, while authored
items use the earliest finite timestamp on the page. Blank padding can be untimed.
The installed untimed sentinel is `3.4e38`, not IEEE FLT_MAX. An entirely
untimed final page waits for selection. Cancellation invalidates deferred work.

Ending the text emits a native signal and stores a pending callback;
IsDialogOver waits until the closing presentation reaches alpha <= 0.1. The source
explicitly invokes ChangeComport; querying IsDialogOver alone must not change a
Brain. The screen validates owner/model identity, consumes the request and
applies the normal behavior through the existing Brain on a deferred turn.
It does not resume the old compiled coroutine from a drawing callback.

The HUD binds the existing Character visibility signal and state dictionary.
Widescreen suppresses drawing while retaining requested state. Life values come
from promoted StandardGame hit-point bytes in an actor-local native Resource;
magic uses the existing Magick Resource. It does not invent missing totals.
The source-derived Fix HUD now joins its six GLI textures to `Textures.cnt`,
exports the original image regions as editable native `TextureRect` nodes, and
binds the player-life presentation to the actor state. Its currency counter
uses the imported `InventoryFont` and the dedicated item-0 quantity from the
actor inventory Resource. Source initialization sets that dedicated count to
zero; absent ordinary inventory items retain the `-1` sentinel. The generated
level's saved screen scene references the generated HUD scene under the
maintained screen hierarchy. For locales without verified source HUD data, the
generic placeholder meters are hidden instead of presented as original art.

### Fix HUD extraction status

The earlier `0x4904/0x4944` layout hypothesis was rejected after byte-level
reader/writer accounting. Both `FUN_0040bcc0` and `FUN_0040bda0` read the
corrected layout at Fix.gpt offset `0x4a38`. The 64-byte block there contains four disabled
16-byte panel descriptors. Each layer-array pointer resolves to a Fix block
5:0 allocation whose payload extent is exactly `count * 0x24` (31, 7, 14 and
4 records). The `+4` and `+8` references resolve to Fix block 6:1 allocations
with the expected material-sized extents. `FUN_0040c270` confirms the 36-byte
layer record fields; `FUN_0047abb0` consumes coordinates as x0, x1, y0, y1,
then UVs as u0, u1, v0, v1.

The six texture references begin at `0x4a78`. RTP relocations resolve them to
Fix block 6:1 allocations of total size `0xc0` (payload `0xbc`), matching the
object size used by `GLI_xCreateTexture`. The corresponding Fix.ptx table
indexes, in slot order, are 22, 23, 13, 11, 12 and 30. Each resolves through
Fix.sna block 06:01 to a `0xbc` GLI texture record; its name at `+0x64` is,
respectively, `enemyfg_nz.tga`, `enemybg_nz.tga`, `poigne_nz.tga`, `vie0.tga`,
`vie1.tga` and `gold_icone_nz.tga` under `fixtex\stencil\jauges\`. The
Fix.rtt/Fix.ptx identities resolve to unique entries in `Textures.cnt`, and
the decoded first mip dimensions match each GLI logical size: 64x8, 128x16,
32x32, 64x16, 64x16 and 128x64. The formats are palette RGB, palette RGB,
ARGB1555, ARGB4444, ARGB4444 and ARGB4444, respectively. The join uses a
Fix-local resolver so overlapping block keys cannot bind these package
pointers into the active level. The cursor and archive/object joins have
focused source regressions.

The second Fix.sda table and its direct draw consumers now give slot roles.
`FUN_0040c300` reads rows 0 and 1 for the life handle and its two dynamically
split segments, drawing texture slots 2, 4 and 3. `FUN_0040c720` draws enemy
bar slot 1 as base/background and slot 0 as fill; its row selectors address the
base/fill pairs 11/12, 13/14 and 15/16. `FUN_0040c950` draws coin slot 5 with
row 23. Static `.data` from both the decompilation reference executable and
the exact installed `MaiDFXvr_bleu.exe` confirms selectors 11/12, 13/14 and
15/16 at `0x5e6839/0x5e683a` and the corresponding `+0x14` / `+0x28` fields.
The installed executable SHA-256 is
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`; the
alternate `disc/exe/Glide 3x/MAIDFXVR_BLEU.ICD` is a different build and is not
used for this selector comparison. These static joins do not yet establish
Godot's final keyed/upload behavior or pixel parity.

This corrects the former `0x4a24` cursor claim by `0x14`; `0x4904` is inside
the `FUN_004e5150` shadow arrays and the former `0x4944` values are shadow
fields, not HUD handles.

The descriptor/layer extents, six source textures, direct draw-slot roles,
currency font atlas, and dedicated inventory item identity are source-verified.
The native exporter writes editable image controls, a native `FontFile` and
currency `Label`, and composes the HUD into the persisted locale screen used by
the generated level. The actor supplies current life and currency values;
original-save currency restoration is not established because the manor save
fixture contains no item-0 quantity. The disabled panel descriptors do not own
the runtime draws. Explicit `--content hud` emits the stable
`openspace.hud.layout_unverified` warning and omits the optional component if
the source layout or texture join does not validate. Legacy Rete HUD values
with the former incorrect offsets remain inspection-only and cannot be saved
or exported. Scene, menu and save imports remain independent.

The selected Object and Weapon slots are drawn by `addons/astrolabe/ui/hud_slots.gd`,
which `screen.gd` adds under the HUD. `FUN_0040cb50` draws the selected Object's
inventory image (`FUN_0041d890`, kind "Object") at Fix.sda row 22, (269,404) 48x60,
and `FUN_0040ca40` the selected Weapon image at row 10, (10,360) 48x48, with its
count; each is gated by the AI's `Stats_DisplayMainActorSelectedObject` / `...Weapon`
request. The earlier table above had the two roles swapped. The Load Game
route requests only the Object slot (the original save selects the Gogoud key,
`Object_21`), so the key shows and nothing else, as in the original. The legacy
original-save restore now carries the selected Object flag (`restore_legacy`).
Documented gaps: the weapon count text offsets (Fix.gpt) are not extracted, so
the count sits under the image; life/armor/magic images are not wired to
`hud_slots`.

The stone frame at the bottom of the original Object slot is panel descriptor 3,
activated by `FUN_0040c150(3)` before the Object draw. The reader now decodes
every panel's layer records regardless of the descriptor's static enabled byte,
which is only the runtime draw gate. Layer UVs are ordered u0, u1, v0, v1
(`FUN_0040c270`; an earlier reader had v0/u1 swapped); the draw API puts v=1 at
the top row, so u0>u1 is a horizontal flip and v0<v1 a vertical one. Each layer
texture is a Fix GLI object resolved to Textures.cnt (`FixTex/STENCIL/Bordures`).
Panel 3 has four 31x31 layers (coina, bordb, coinb, coina) spanning x 255-379,
y 448-499, clipped by the 480 line. All four panels export as hidden
`Source/Panel0..3` Controls of native TextureRects; `hud_slots.gd` shows Panel3
with the Object slot. Panels 0-2 (life/enemy frames: 31, 7, 14 layers) are
exported but not yet shown, because which indicator gates each was not traced
and the bodies of the activation table at 0x5e6810 were not read.

Final Godot keyed/upload behavior and pixel parity remain unverified. The
current actor binding does not yet connect enemy bar values or the other
source HUD indicators (armor, magic, selected object, weapon). No gameplay or
original-game visual comparison has been run for this HUD slice.

## Current boundary and next increments

The import slice includes the eight dialogue items called by the normal manor
opening, plus one ordinary conversation. Opening items use ItemBox3 (two lines
per page); the ordinary item uses ItemBox1 (six). Voice links use bank 142.
The callback targets below come from actual answer behavior pointers, and are
independent of the normal behaviors that call RunDialog.

| Item | Lines | Voice member | Callback model / normal |
| --- | ---: | ---: | --- |
| gogood_texte1_manoir | 3 | 0 | 210 / 4 |
| gogood_texte1a_manoir | 3 | 1 | 210 / 5 |
| gogood_texte1b_manoir | 4 | 2 | 210 / 6 |
| gogood_texte1c_manoir | 4 | 3 | 210 / 1 |
| gogood_texte2_manoir | 4 | 4 | 210 / 11 |
| gogood_texte2a_manoir | 9 | 5 | 210 / 12 |
| hype_texte1_manoir | 2 | 6 | 294 / 5 |
| hype_texte2_manoir | 8 | 7 | 294 / 3 |
| gogood_master_whatif01 | 21 | 12 | 991 / 17 |

All nine selected items have a single nonbranching answer, zero line flags, no
item icon/channel effects and spans using the two recognized dialogue fonts.
The importer checks those constraints before promoting content. This does not
establish branching or whole-catalogue coverage. Other entries in the 67-item
catalogue emit coverage debt; reaching an unavailable item stops explicitly.
They remain subsequent manor implementation work.

Still required for manor fidelity: native font/frame pixel comparisons,
line reveal and fade pacing, additional dialogue
items and branching answers, channel/action/icon side effects, armor and
selected-inventory/weapon values/icons, saved life-state restoration, and original
vs Godot captures. HUD descriptor cursor, texture identities/dimensions, Fix.sda
rows and direct draw-slot roles are source-verified. Final native pixel parity,
keyed/upload behavior, enemy and other indicator values, and original-game
visual comparison remain unverified.
Fade geometry exists but unrelated source fade procedures are not promoted by
this slice. Native voice teardown follows the existing emitter's stoppable-event
policy; dialogue-specific interruption parity remains to be checked.

Dialogue-to-OpenSpace reconstruction rejects the newly promoted dialogue root
explicitly. Unknown item/line/font/channel fields and RTG header semantics must
be preserved/promoted before a source writer is added. This is a Godot semantic
slice and is not a claim of language-package byte reconstruction.

`tests/ui.rs` declares pure source/Rete checks, a native maintained-component
contract and a native actual-dialogue/resource export check. Native tests have
bounded parser preflight and owned temporary output. Original-game parity and
component checks are diagnostic gates. Canonical end-to-end acceptance is the
normal authored New Game entry with the original imported AI, compared against
the actual original opening. A demo or a synthetic UI fixture cannot establish
that integrated result.

## Source location title

English Astrolabe's `astrolabe_txt2` is imported from the actual language
catalogue as style 4, with two untimed lines, no voice, and completion to its
owner's model 659 normal 1.
The shared Screen instances the registered `location_title.tscn` sibling and
routes the source Position/Run/Select/End/IsOver APIs through the active owner.
The single center stencil, source bitmap font and span layout remain native
Godot resources. Opening alpha steps only after more than 10 ms; closing steps
once per draw, matching the source's asymmetric arithmetic.

Actor `level:/persos/world/0/91`, model `level:/ai/models/659`, owns the unchanged
cue: wait 200 ms; PositionDialog(160,70); Run only when GetGameState()==2; change
to normal 1; wait 3000 ms; EndDialog; deactivate SelfActor. Other selectors still
execute the unconditional transition, End and deactivation. There is no
authored every-level trigger. Func 195 compiles to the actual shared game-session
getter; combined masks fail with an explicit stale-result diagnostic rather
than receive a fabricated selector. Rete schema 58 retains the typed query alongside inventory motion and pickup semantics.

End animates the active presentation's close without invoking completion. Level
adoption/release and teardown cancel both presentations immediately. Ordinary
dialogue retains its pending-behavior and MainActor speech contracts; title
selection uses the same pending completion contract. Focused tests execute
synthetic and unchanged imported cue programs against the maintained components and session getter,
including native physics startup and map cancellation/reentry. Full level
conversion/game routes and original-game pixel/cadence parity are separate
validation scopes.

The optional line drawing anchor retains font alignment selectors and source
X/Y offsets. The importer declares that structure before reading it; Rete and
Godot export semantic values. Native bitmap-font metrics resolve first-to-last
glyph center width and the vertical space-glyph anchor during prefix reveal.
The title's first and second anchors are `[1,1] / [1.8,-0.2]` and
`[1,1] / [1.9,-0.4]`. Source addresses and relocation stay private to import.
