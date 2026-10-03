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
