# World154 armor and Camera437 mechanics binding repair — 2026-10-08

This worker branch starts at `main` `edb7704`, with source, Cargo target,
temporary projects and evidence under
`output/worktrees/intro-armor-camera437-20261008/`. It addresses the two errors
in `output/integration-resume-20261008/startup-cap-evidence/rendered-engine.log`
(armor at line 879, mechanics callback at line 1008). It does not accept the
complete opening, change title/pickup implementations, run Hype, push, or
modify the separate `hype-decomp` repository.

## Armor: the selection exists; the compiled receiver contract was wrong

World154 normal2's first-visit schedule calls `SetArmorResistance(0)`.
`0043a8a0` initializes the Fix armor definitions and selects the catalogue
head. The existing importer already represents this as `selected = Some(0)`;
the preserved real game's exported `shared/gameplay/armor.tres` also has
`selected = 0` and three definitions. Source/Rete and native resource checks
retain that initialization.

The scene exporter creates one mutable catalogue graph and its `ArmorState`
facade per level. Each compiled Brain resolves that facade through the nearest
scene owner. The compiler's armor guard nevertheless required `active()`,
which belongs to `ArmorCatalog`, whereas the facade exposes
`selected_definition()`. Consequently the *method-presence* guard failed before
it inspected the selected definition. The diagnostic saying "with a selected
definition" was not evidence that the selection was absent.

The compiler now checks the facade's `selected_definition()` contract and bound
catalogue for all armor operations, and rejects a null definition for
resistance/repair/query calls that require selection. The catalogue remains the sole owner of mutable
resistance/piece state. No default selection or permissive missing-binding
fallback was added.

The new native regression reads a saved level through the production exporter,
with a Rete-reopened synthetic Fix catalogue. It executes compiled Proc430(0),
repair, resistance and count queries through two actors sharing the facade.
It checks initial selection, mutation of the selected graph, isolation from a
second level instance, and explicit null after an invalid selection. The
fixture is an identified armor excerpt; it does not run unchanged World154.

## Camera: explicit no-card state and incorrectly indexed callback table

The connected source chain is mechanics selection `004440e0` → frame dispatch
`00443f00` → position history `00443d50`/weight history `00443db0` → camera
weighted pivot `00444620`. Missing clean-C bodies were read directly from the
original static Ghidra evidence. No clean-C build/test or original executable
execution was performed.

The local unwrapped PE has SHA256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
Static disassembly corroborates initializer `00443540`: it writes update
callback `00585d51` at byte offsets 0 and `0x20` from the supplied table root.
Dispatch uses `kind * 8`, with the adjacent four-byte entry holding the
control callback. These entries therefore select **kinds 0 and 4**, not 0 and 8.
Kind 8's update at byte `0x40` is `00588941`. The earlier notes and native test
that mapped 0/8 and rejected 4 confused the initializer's word index with its
row index. This correction supersedes those mapping claims in the
[earlier camera pass](intro-camera-source-first-2026-10-07.md).

The existing native post-contact translation publisher already implements the
supported observable position result; this patch does not replace movement,
camera geometry, target/eye setters, travel or mode3/6 behavior. During
conversion, canonical mechanics kinds 0/4 bind the native Resource property
`camera_position_source = &"post_contact_position"`. Other kinds receive an
empty binding. Native state-animation tracks continue to select their saved
mechanics Resources. Actors and cameras exchange the named `position_source`
contract instead of a source callback address, and fail closed for absent or
unmapped bindings. The controls for kinds 0/4 differ (`00589bfa`/`0058cc2d`),
so `004440e0` can call the coordinate-mode reset `005711f7`. That reset copies
physical vectors but does not clear the position/weight history. Sharing the
position publisher consequently preserves that history; it is not a claim
that all dynamics/control behavior is identical.

The connected level trace also shows the MainActor starts at state 47 with a
kind-0 card, while Hype28 normal0 conditionally requests state 343 under Global
field 37's first-visit branch. State 343 explicitly has no mechanics card.
`00443690`/`0044a910` obtain that null state field; `004440e0` schedules no frame
dispatch when no nonnull card is selected. It leaves the actor's position/weight
history intact. Camera preparation `00443bf0` consumes those values and seeds
zero-weight history from the actor translation. No callback is required merely
to read the history. The previous native guard instead conflated an explicit
no-card state with an unknown/missing conversion binding.

Native state animations now also set `source_camera_retains_position_history`
from the state's actual optional mechanics reference. An explicit no-card
state skips sampling and supplies `position_source = &"retained_position_history"`
to the existing camera pivot. A null mechanics Resource without that exported
state binding still fails. No replacement card, source queue, scheduler or
address lookup was introduced. The saved support package and native animations
must both be regenerated to obtain this behavior.

The preserved error log does not include the target's selected state/card at
the failure. Thus the exact runtime choice between the no-card defect and the
kind-4 defect is not proved by that log. Both defects and their connected source
states are statically demonstrated and have targeted native regressions; this
worker does not substitute that evidence for unchanged intro acceptance.

The saved-camera regression reconstructs synthetic source cards 0/4/18,
reopens them through Rete and exports their native mechanics/animation
resources and an explicit no-card state. It checks a saved kind-4 target reaches the existing mode-0 pivot,
both supported cards supply the same publisher, history survives a switch
between those shared publishers, state 343 preserves history without sampling,
a fresh no-card state uses the existing zero-history translation fallback,
unsupported kinds 8/18 fail closed, and resuming a supported publisher after
failure starts fresh history.

Rete stays at schema 55: source mechanics meaning/serialization did not change.
Godot support advances 72→73 for the exported named contract. Integration must
update maintained support and regenerate affected content together; this
worker does not publish generated original-game content into the actual
`godot/` game on main.

The integration lead reports clean main `98a3d2e`, including inventory/loading
commit `c375693b5c8310ffc8450dd97ce3be0e910083a3`, using support 73 and Rete 56.
This worker remains based on `edb7704`, with its own Rete 55 tests. The numerical support
bump can therefore merge without a textual conflict while still describing
different contracts. The lead must assign the combined support interface and
regenerate matching content; this branch does not modify those lead-owned
changes or inventory animations. `src/godot/support.rs`, `version.json`, and `tools/test-manifest.json`
are the shared integration files.

## Validation and remaining limits

Owned ignored evidence is in this worktree's `output/evidence/`; Cargo builds
use its separate `output/target/`. Native fixtures use Godot 4.7.2 headless and
fresh temporary projects, maintained support assets, actual saved native
Resources/AnimationPlayer tracks, and owned parser/runtime logs. Pinned
nextest 0.9.145 runs the declared tests with `tools/supervisor.py` cleanup and
manifest per-test deadlines. The ignored runner/config/result files preserve
the exact commands and selections.

| Final selection | Result | Timing and evidence |
| --- | --- | --- |
| Compiled armor setup, saved camera projection/card/no-card states, armor AI operations, shared armor selection, native camera target/motion/cleanup, static connected Manor trace | 6/6 pass | 103.93s including 81s build, setup and cleanup; `final-focused-result.json`, `final-focused-nextest.log`, `final-source-bindings.json` |
| Every manifest `always`/`gameplay` Rust safeguard, including saved synthetic native conversion and actual CharacterBody contact | 21/21 pass | 134.72s including build/setup/cleanup; 61.43s test execution; `final2-safeguards-result.json`, `final2-safeguards-nextest.log` |
| Mandatory supervisor, selection, nextest deadline/escaped-child, and recording-comparison Python checks | 4/4 pass | 3.14s / 0.07s / 1.02s / 2.09s; `final2-test_*.log` |
| `cargo build --release --locked --offline` | Pass | 82.24s; `release-build.log`, `build-result.json` |
| `cargo fmt --all --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`, `git diff --check` | Pass | `final-format.log`, `final-clippy.log`, `final-diff-check.log`, `final-build-checks-result.json` |

Both final Rust selections exit 0, with no supervisor timeout, leaked child or
remaining process. The no-card test covers explicit state entry, retained
positions/weights/count, live pivot consumption, zero-history startup, and
resumption of the named publisher; a null Resource without the exported state
binding fails closed. The preserved source report includes selected armor 0,
Hype initialization model, no-card state 343, and source-card role URIs.

Earlier failed attempts remain preserved. Initial fixture errors (an empty
synthetic scene, missing Brain bindings/behavior selection, and a manually
created camera mechanics Resource without its converted binding) were repaired
and superseded by the exact focused passes. A safeguard retry needed the pinned
nextest path in its environment; the failed setup log is retained. Formatting
of one new source-trace assertion was corrected before the final style pass.

The first selection was mistakenly wider than intended: it included three
existing full-import camera checks. Two failed their 60-second Godot editor
import deadline (`imported_camera_rule_aims_at_manor_helpers_while_native_eye_moves`
and `imported_manor_opening_shot_excerpt_selects_eyes_targets_and_waits`); the
third failed on the existing harness's uninferred `action_changed` variable
(`imported_gogoud_opening_ai_drives_fourth_camera_cut`). That run passed 47/52
tests but **is a failed selection**, preserved in `focused-nextest.log`,
`focused-result.json` and `junit.xml`. No further full-level exports were run.
The final selections explicitly exclude those checks and all other unrelated
areas. They do not turn those failures into release coverage.

Full intro/dialogue/handoff acceptance, animation-command mechanics beyond the
selected state-card slice, other mechanics callbacks, native/source
floating-point equivalence, and the full release matrix remain outside this
worker's validation claim. Integration owns the combined version, support
update/content regeneration and a later unchanged actual-game acceptance run.
