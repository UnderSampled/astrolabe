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
