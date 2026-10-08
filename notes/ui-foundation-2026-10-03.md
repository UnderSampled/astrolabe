# Manor UI foundation checkpoint, 2026-10-03

Isolated branch `ui/manor-screen-space` starts at
`3c44391efc6e6d6c23aa9381e778e4fbed878602`. The source checkout was clean on
inspection. No dirty dependencies, original textures, credentials, generated
projects or test evidence were imported into the branch. Camera/intro mechanics
and menu work remain with their separate owner.

The maintained architecture and exact scope are in
[`docs/astrolabe/ui.md`](../docs/astrolabe/ui.md). This checkpoint supplies authored
Godot scenes/resources/scripts, typed source AI calls, native localized dialogue
resources and actor-owned life state. It is the first implementation increment,
not a pixel or complete-manor-UI acceptance claim.

Source-only tests resolved both selected items, all 24 line records, localized
voice bank/member links and stable owner/model/normal-behavior callbacks. Three
assumptions were corrected against actual data: DLG word zero is a name table,
word one is the item catalogue; manor RTG's header11 is not the .lng/.sna block
count; the untimed marker is 3.4e38 rather than IEEE FLT_MAX. No virtual addresses
are persisted in dialogue or translated text assets.

Initial static validation: all-target Rust compilation, formatting and Clippy;
70 passing nonignored library tests (20 existing local/native cases excluded),
the new compiled-bridge test, the real English manor dialogue/Rete test (7.23s),
and the release build (59.68s). Python supervisor/selection safeguards passed.
Native maintained-component and actual-resource export cases are declared in
`tests/ui.rs`, with bounded parser preflight and owned output. The parent alone
queues the shared experimenter. An additional existing regression was mistakenly
selected as source-only: `source_scene_rete_routes_preserve_hierarchy_collision_and_model_sharing`
actually attempted Godot editor import after its source/Rete phase. It failed
after 114.92s on sandbox socket/editor-settings access and exited; no original
game ran, no shared outputs were used and it is not validation evidence. This
ownership mistake was reported to the parent. Remaining native UI cases will
run only through the serialized experimenter; results will be recorded then.

Integration requires cherry-picking the isolated commit(s), not copying generated
output. Support interface45 adds the UI; Rete schema38 adds dialogue and Perso
life state, so regenerate old manor Rete/Godot content from user-owned inputs.
Review conflicts in AI source/compiler code, support interface/version, scene
composition and the test manifest against concurrent camera/menu changes. The
UI does not change camera scripts or menu scenes. The whole-catalogue dialogue
and presentation-fidelity backlog is explicit in the architecture document.


Static full-workspace discovery found one Orrery comparison fixture that also
constructs Perso; it now supplies the new optional life state. Native UI checks
print their success markers/resource facts into captured test output. Dialogue
completion uses the original alpha <= 0.1 predicate while retaining the pending
callback from text end. These corrections supersede the initial frozen commit
before the serialized native assignment.


Full-workspace Clippy also exposed an unchanged baseline lint in
`crates/orrery/src/orrery.rs:722` (`observe_magick_behavior` has eight arguments).
The normal package Clippy invocation passes; this unrelated full-workspace lint
is reported and left with its owner. Full-workspace compile/discovery passes with 275 tests, and the reviewed
manifest matches every discovered case.

The first serialized native assignment ran at frozen revision `3c92664` and
finished in 150.635s. Its four Python modules passed 28 checks; 20 of 23 Rust
checks passed. Every stage reported no timeout, leaked children or remaining
processes. Both UI cases emitted success markers but failed on engine errors,
so neither was accepted. The resource writer's Display formatting expanded
`3.4e38` into an integer-looking decimal, overflowing Godot's signed integer
parser. It now writes round-trippable float literals. The component fixture
omitted Character's required AnimationPlayer and disabled processing before
tree entry; it now supplies that node and disables processing after entry.
The resource check also asserts actual finite and untimed deadlines.

Native conversion independently resolved intro voice
`sound:/Sound/English/V_Manoir.csb#25` (4.12954648526077s), 3 lines and normal 4;
ordinary voice `#20` (20.2173242630385s), 21 lines and normal 17. The mandatory
Orrery scratch-lifetime safeguard failed with bare `No such process (os error
3)` after 0.085s. That unchanged test reports no operation context, so the
failing syscall/phase cannot be identified from this evidence. It remains a
separate unresolved safeguard failure, not a UI pass or proven baseline flake.
The parent queues the next bounded native assignment; this branch launches no
native sessions. Evidence is local and uncommitted under task workspace
`ui-native-20261003-r1/`.

The serialized r2 assignment at clean `92e43944e65380ad5c9bccc7ffb739b4629e6cd1`
passed all 23 Rust cases and 28 Python checks in 126.800s. Both UI cases reported
no engine errors; every stage reported no timeout, leaked children or remaining
processes. The unchanged scratch safeguard passed in 0.165s, so the earlier
bare errno3 was not reproduced; its underlying cause is still unidentified.
This validates maintained-component behavior and conversion of the two selected
actual dialogue resources. It does not validate full intro or visual parity.

After explicit freeze release, the next source-backed increment imports the
eight normal-opening items plus the ordinary item, totaling 58 lines. The
source/Rete check validates all nine names, voice bank/member and event IDs,
callback model/normal targets, and exact semantic round trips. Strict import
checks establish single answers without branches, answer text/icons, item
icons/channel effects, nonzero line flags or additional fonts. The architecture
document records the actual callback table. Native nine-item resource conversion
is queued separately; no session is launched by this task.

Read-only integration review against main `73e18e2fc9633b08697bc09e9895159bdeea3fe6`
found shared edited paths in `src/godot/support.rs`, `tests/cameras.rs` and
`tools/test-manifest.json`. Preserve Sol's New Game starter entry and camera
checks alongside the UI support list, new optional Perso life state in fixtures
and UI test entries. Main has not changed the support-interface or Rete-schema
constants from this branch's base. Integration still needs the normal manifest
inventory check; original inputs and generated output must stay uncommitted.


The serialized r3 assignment at clean `f2eb446c7fec15d2c1e3b09865eb28299131970b`
passed 24 Rust cases and 28 Python checks in 121.984s, with no engine errors,
timeouts, leaked children or remaining processes. All nine items total 58 lines,
23 finite and 35 untimed deadlines; voice/callback mappings passed. This is
component/resource evidence. The accepted end-to-end path is authored New Game
with imported source AI, compared against the actual original opening.

The subsequent isolated presentation increment promotes one shared source asset
and generates a native FontFile with 133 Unicode glyphs, fractional metrics,
source atlas UVs and a native red outline cache. Native Label spans adopt the
source offsets and first-glyph-center origin. The authored 320x192 frame uses
nine TextureRects with eight horizontal/four vertical edge tiles and a stretched
center. Actual Fix GPT/RTP dependencies and CNT samples are read separately from
maintained support; extracted media stays uncommitted. All five inputs are
ARGB4444, so their source alpha is retained; center alpha is 170/255. The atlas
uses no color key. Read-only research
corroborated Fix GPT stencil slots 18084/18088/18092/18096 against
DLG_fn_vSNAWriteInFix (0056bbe0). The importer uses explicit bounded profile roots
through RTP; a provisional PTX channel parser was removed as unnecessary. Fonts
normal/emphasis have matching geometry and white/red ambient colors. J's UV
reaches 256/255; preserving that one-texel extrapolation avoids rounding away
source coverage. Runtime/cache diagnostics and New Game visual comparison are
pending for this increment. Source reconstruction, letter effects and exact
fade pacing remain documented debt.

The foundation handoff includes only the four commits through f2eb446. At main
`e5460cfb38d8c5e4f37cd9e856fac9aa84e534ee`, the three shared paths are support.rs
(union New Game/route UID and UI support, interface45), cameras.rs (keep new
camera checks and add optional fixture life state), and test-manifest.json
(union entries and validate inventory). Sol owns main integration after its
current New Game session; no main checkout edits are made by this task. Reconcile
the presentation increment at a clean checkpoint after foundation integration.

The presentation increment bumps support interface to 46 and Rete to schema39;
regenerate selected data and update maintained support explicitly. Static package
Clippy/check and the exact source/Rete case pass before native assignment.


R4 at clean `8a296eaafa628f9b6488ff7a7098b409e9669bb1` passed 22/23 Rust
cases and all 28 Python checks in 138.724s; all stages reported no timeout,
leaked children or remaining processes. The new actual-resource case stopped
on a generated-builder parser error in 4.795s: the bitmap-scale constant was
incorrectly spelled and scoped to FontFile. The official TextServer enum is
`FIXED_SIZE_SCALE_DISABLE`; the exporter now uses
`TextServer.FIXED_SIZE_SCALE_DISABLE`. No presentation success marker or native
font/frame validation is claimed from this failed run. Existing source and
mandatory safeguards passed. The foundation checkpoint remains unaffected.

The correction also adds a five-second pinned native ClassDB audit of every new
FontFile method/argument count and property before source import. The exporter
checks generated build.gd syntax in its owned stage before media import/build
when dialogue resources are selected. The next assignment runs this affected
resource case first and aborts on its failure before broader mandatory checks.
Official FontFile/TextServer documentation and read-only installed-binary symbol
inspection support the API audit; actual pinned runtime audit remains pending.

R5 at clean `a4fb98888886e8e9c3a0a41974f40b20de228caa` ran the correct immutable
assignment after the parent replaced its experimenter. It passed the native API
and generated-builder syntax preflights and exported the shared 133-glyph font,
but the combined Unicode/fractional-advance assertion failed. Total assignment
time was 80.049s, including 62.007s workspace build/discovery; the resource case
took 11.525s. All 28 Python checks passed. Broader safeguards were intentionally
not started after the affected case failed; cleanup reported no leaks or remaining
processes. No font/frame parity or New Game acceptance is claimed.

Godot's documented automatic subpixel setting and backend source show that AUTO
rounds advances at the selected size21. The correction explicitly selects native
quarter subpixel positioning, separates Unicode and metric assertions, and prints
their values. A tiny synthetic FontFile save/reload and shaping check now runs in
the existing pre-import API preflight. It verifies fractional cache advances and
shaping within native precision before original media conversion. This is a native
font setting, not a custom glyph renderer. The corrected resource gate remains
pending the parent's serialized experiment queue.

The three presentation commits were reconciled in the isolated branch onto the
parent's integrated foundation `b9c5949d1c3bd4eb915fbe190334327f292cc1f6`.
The only conflict was a source-fixture description in test-manifest.json; the
combined description retains unchanged-source identity plus font/frame coverage.
New Game/camera files, tests, UID entries and inventory/save corrections remain
as integrated. The pre-integration branch is retained for earlier evidence;
main was inspected read-only and not edited. The next assignment validates this
reconciled branch at a fresh immutable checkpoint and output path.

R6 at clean `05cf773588aaacac28ff1248f45e3f572cead69c` failed the new tiny
pre-import metric gate in 0.665s, before any original-media conversion. The
serialized cache advance is correctly 16.2338, Unicode A maps to65 and é is
available. `Font.get_string_size("AA")` returned33.0; the test wrongly compared
that bounding size to a fractional sum. Official Godot source confirms
`TextServerAdvanced::_shaped_text_get_size` applies ceil to its bounds. This
does not establish a displayed-layout error. The revised assertion inspects
the actual native shaped glyph advances, then checks the integer bounds against
their ceil. It retains the fractional cache check.

Overall R6 took70.240s, including62.932s build/discovery. All28 Python checks
passed before the resource gate; the21 mandatory safeguards were unrun after
the early failure. Supervisor cleanup reported no leaked children or remaining
processes. The parent requested a minimal native metric-only diagnostic before
another build cycle. Isolated commits/tests are milestones; a user checkpoint
requires integration/regeneration and visible improvements in actual New Game.

## Integrated V8 named font reload diagnosis

At main revision `e0cf3d05b0c9762aaa154284940109ac1f372a91`, interface46
generation completed and the actual native dialogue/resource test reached its
Unicode gate. The cheap unnamed font passed; the exported named font reported
`has_e_acute=false`. Retained evidence is
`TestResults/new-game-preview-selection-v8-sol-20261003.log.d/tests.xml` and
`execute.log`. The source/Rete case and maintained UI timing test passed.

This is not a missing source glyph: the retained original font inspection
explicitly records code233, and generated
`godot/generated/hype/levels/manoir/dialogue/presentation/en.font.tres`
(SHA256 `48cd6c3ea68a63daafebed609d924689ca76c48f66a778ab3ce5520509cb56cf`)
contains all133 Unicode glyphs, including233, in both normal and outline caches.
The resource writes `font_name` before `fixed_size`. Godot4.7's
[`font_set_name`](https://github.com/godotengine/godot/blob/4.7-stable/modules/text_server_adv/text_server_adv.cpp#L2326)
initializes a size cache; changing fixed_size does not remove it. Bitmap
`font_has_char` consults the first cache. This explains an empty16px cache
shadowing the populated21px cache after named resource reload. The cheap
unnamed fixture did not cover that path.

The isolated `ui/font-cache-load-order` branch starts from committed main
`e0cf3d0`, with no live dirty dependencies. Export normalization now places the
native fixed_size property first, preserving all other property order and glyph
entries. Pure tests verify unchanged glyph content, normalization idempotence,
and unchanged dynamic/other resources. The source/Rete test explicitly checks
the é metrics and atlas binding. The native preflight adds a tiny authored named
font fixture, and the actual resource gate checks character support for every
cached code while printing size caches and any missing codes. No source text,
font repertoire, fallback policy or Unicode assertion is weakened.

A minimal parent-queued A/B reload assignment uses retained original and
property-ordered copies of that single native FontFile. It must confirm the
empty-cache diagnosis and all133 Unicode mappings without building or importing
original media. Native validation and visible New Game font/frame acceptance
remain pending; this worker launches no native sessions.

Before the native assignment: both pure font normalization tests passed;
the exact source/Rete case passed in5.18s with explicit é metrics; all29 Python
runner/selection checks passed using the existing pinned nextest executable
read-only; root-package offline Clippy for all targets and formatting passed.
The test manifest retains all281 existing cases and adds two pure scene cases
for283 total. The parent owns native syntax/reload and integrated validation.

## Reconciled retained evidence — 2026-10-08

The following R7/R9 and HUD evidence was preserved on the UI branch but missing
from the integrated note. It is retained here without replacing later production
facts. Font property normalization is integrated in `259ae71`; this closes the
minimal native reload assignment, while visible complete intro/font parity
remains governed by the actual-game acceptance. The original HUD survey below
predates integrated HUD assets and crops (`92ebebc`, `7be1b7b`, `99bb41b`,
`8ff522b`). Its “parked/incomplete” statements describe that survey, not current
implementation status. Full rendered HUD parity is still unproved.

The minimal R7 native metric-only diagnostic at clean
`d0e90efd187f469b37d34e71c0d5869bd778e744` passed in0.610s with no Cargo build,
level/media import or game session. The exact cached advance is16.2338; shaped
glyph advances are16.21875 each, summing32.4375 within native1/64px precision.
Native bounds are correctly33.0. Parser/API/serialization/shaping checks passed
on Godot4.7.2; both stages reported no errors, timeouts, leaked children or
remaining processes. Evidence stays outside Git at
`task-2/ui-font-metric-20261003-r7/report.json`. This resolves measurement
semantics and does not establish actual font/frame or New Game visual parity.

R9 at frozen `3b2ba700d6bcf8e91246c27fa4f24c512fc6ff63` passed all four
syntax/API/reload stages in2.055s on Godot4.7.2. The retained original resource
loaded caches `[(16,0),(21,0),(21,1)]` and reported all133 source characters
missing through has_char. The property-ordered copy loaded only
`[(21,0),(21,1)]`, exposed all133 characters with no missing codes, and retained
identical name and A/é advances. A, é and the Unicode curly quote were available.
The small named-font preflight also passed with only the21px cache. There were
no errors, warnings, timeouts, leaked children or remaining processes, and the
source remained unchanged. Evidence is
`task-2/ui-font-reload-20261003-r9/report.json` with immutable input hashes.
This confirms the cache-order diagnosis and native mapping fix; full integrated
dialogue/frame validation and visible New Game parity still require their
existing integration checks. The parent sent3b2ba70 to core for integration.
UI work is parked until an actual integration issue is observed; no additional
run or feature expansion is queued by this worker.

### Historical HUD layout investigation (Oct 3 snapshot)

The parent relayed the completed read-only investigation
`01a1042d-dacc-710c-93c3-218fc20490f4` ("Inspect HUD layout storage").
The Fix HUD [reader](../reference/decompilation/functions/0040bcc0_FUN_0040bcc0.c)
and [writer](../reference/decompilation/functions/0040bda0_FUN_0040bda0.c)
persist four panel descriptors, texture/font references, text offsets and two
24-entry rectangle tables. Installed `Fix.sda` scalars begin at `0x9b4`, with
tables at `0x9cc` and `0xc0c`; the inspected tables match. Each entry contains
six floats covering position, inclusive endpoints and size. The investigator
corroborated the following roles and concrete values against draw routines:

| Component | Entry | Position | Size | Maintained behavior |
| --- | --- | --- | --- | --- |
| Life | 0/1 | 0: (65,435) | 0: 38.4×38.4 | Hit-point ratio and full/empty texture split; entry 1 provides fill dimensions |
| Armor | 2 | (10,420) | 48×48 | Selected armor definition's linked texture/UV pieces |
| Magic | 5–7 | (450,425),(505,425),(560,425) | 48×48 each | Inventory icon selection and coded perimeter fill/quarter thresholds |
| Selected object | 10 | (10,360) | 48×48 | Selected inventory image, quantity and serialized counter offsets |
| Weapon | 22 | (269,404) | 48×60 | Current weapon's inventory image |

Panel descriptors also carry ordered texture layers, rectangles and UV bounds.
This evidence supports a useful shared `HUDPresentation` content asset, exported
to editable native Godot scenes/resources with inherited consumer overrides.
State remains in existing character/inventory/magick domains; maintained
components implement filling, selection, counters and visibility. No serialized
Godot anchors, containers or complete widget hierarchy were established, and
draw-code constants must not be presented as imported layout. Source addresses
stay at the OpenSpace boundary. Original artwork remains untracked.

Current authored HUD positions remain provisional, and armor/object/weapon
presentation is incomplete. Exact texture closure, every rectangle role and
rendered parity were not verified by that investigation. This is retained
architecture evidence for the existing HUD scope, not an implemented resource,
new schema or implementation authorization. The parent has parked this stage
until its priority is authorized. This worker made only these documentation
updates; no implementation, tests or native sessions were launched.
