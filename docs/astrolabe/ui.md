# Screen-space UI

The maintained UI lives in `godot/addons/astrolabe/ui/`. Its scenes, theme and
scripts are authored Godot support. Text, voices, fonts and original textures
are converted inputs under ignored `generated/`; content regeneration does not
rewrite the maintained scenes. The screen-space HUD/dialogue slice targets
manor; the main menu and level-loading presentation remain separate components.

## Level-loading presentation

`loading/screen.tscn` is a native `CanvasLayer` with editable `Control` nodes.
The persistent `LevelTransition` owner instances it once and emits lifecycle
signals; the presenter displays only the observed resource-loading fraction for
the requested destination `PackedScene` and its dependencies. It does not
smooth or extrapolate that fraction to cover scene replacement, state transfer,
entry placement or gameplay readiness. Completion and cancellation hide the
screen immediately. A load error remains visible with the supplied message
until dismissed; scene recovery stays with the transition owner.

The local decompilation source at `reference/decompilation/functions/` shows
`FUN_00401395` forwarding status text to `CWnd::SetWindowTextA`. The level-load
routine `FUN_0043c8e0` updates that caption for map, always, waypoints,
characters, dialogues, paths, stencils, game interfaces, highlight data and map
processing. This evidence shows no in-frame loading artwork, animation,
percentage bar or minimum presentation lifetime in that build. The whole
decompilation input and current private installed executable have different
SHA-256 hashes, but their machine code matches byte-for-byte across the relevant
loader ranges (`0x401395`, `0x402b60–0x4038af`, and `0x43c8e0–0x43cfff`). The
caption behavior is therefore confirmed for the installed executable. No
guessed background image is used. See the
[dated loading-presentation record](../../notes/loading-presentation-2026-10-07.md)
for paths, hashes and validation limits.

## Composition and layout

`screen.tscn` is a CanvasLayer named `AstrolabeUI`, at layer 20. Its Control tree
owns HUD, letterboxes, dialogue and a fade ColorRect in source draw order.
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
behavior completion target. Rete schema 39 persists those values and a shared presentation reference. Source
addresses and relocation decisions remain in OpenSpace import.

The manor adapter reuses SNA/LZO, pointer-file and relocation codecs for English
`.lng/.rtg/.dlg/.rtd/.snd/.rts` inputs. `.dlg` word zero is the name table; word
one is the circular item catalogue. Installed manor RTG has header byte 11 but
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
| Proc_EndDialog, 324 | Cancel active dialogue and its pending callback |
| Proc_RunDialog, 325(name, mode) | Select localized item and start its existing SoundEmitter event |
| Proc_ScrollDialog, 326(direction) | Cycle answers; no text-page advance in this selected non-branching slice |
| Proc_SelectDialog, 327 | Advance a page or finish |
| Proc_PositionDialog, 328(x, y) | Set source screen-space position |
| Proc_DLG_ChangeComport, 330 | Explicitly apply a pending owner/behavior callback |
| Cond_DLG_IsDialogOver, 121 | Query completion after closing presentation |

The dialogue node owns page index, elapsed time, voice-player reference and
native Tween fade. The voice event runs through the existing actor SoundEmitter
and compiled native waveform resources. Page deadlines are cumulative: scan
finite timestamps within the displayed page, since blank padding can be
untimed. The installed untimed sentinel is `3.4e38`, not IEEE FLT_MAX. An entirely
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
The source-derived Fix HUD reader and Godot projection remain quarantined while
the texture records are being joined to archive members and pixels. Legacy
`fix:/hud/en` data and generated scenes are not valid source layouts and are
not attached to the saved level screen. The maintained generic HUD support
remains separate from this reader.

### Fix HUD extraction status

The earlier Fix HUD reader used incorrect offsets and is quarantined. Corrected
reader/writer byte accounting places both `FUN_0040bcc0` and `FUN_0040bda0` at
Fix.gpt offset `0x4a38`. The 64-byte block there contains four disabled
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
`vie1.tga` and `gold_icone_nz.tga` under `fixtex\stencil\jauges\`. This
verifies the runtime-object-to-source-name join. The archive-member/pixel join,
the associated Fix.sda table roles, and the runtime layer-to-artwork roles
remain unresolved.

This corrects the former `0x4a24` cursor claim by `0x14`; `0x4904` is inside
the `FUN_004e5150` shadow arrays and the former `0x4944` values are shadow
fields, not HUD handles.

The descriptor/layer extents are now source-verified, but prior Rete roots at
`fix:/hud/en` and generated scenes still preserve the bad offsets and
associations. Do not use, regenerate, attach or merge those legacy assets.
Explicit `--content hud` continues to omit the component and emits the stable
`openspace.hud.layout_unverified` warning for caller compatibility; the
remaining blockers are archive pixel decoding and runtime layer-to-artwork
mapping, not the `0x4a38` descriptor cursor or GLI object names. Scene, menu
and save imports are independent. New Rete saves of the legacy type and Godot
exports containing it remain rejected;
legacy Rete files are inspect-only. The maintained generic HUD support script
is separate and is not evidence for the original rendering path.

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
vs Godot captures. HUD rectangle extents, layer records, descriptor cursor and
GLI object names are source-verified. Archive pixels, layer roles and dynamic
draw behavior remain unverified.
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
