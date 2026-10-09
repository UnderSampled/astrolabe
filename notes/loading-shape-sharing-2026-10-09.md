# Native loading resource sharing — 2026-10-09

Static work starts from `12ad58ce49d60b10cdcbb80a29c9118a5e9feffa`,
support84/Rete62, in `output/worktrees/sol-loading-native-20261009`, branch
`sol-loading-native-20261009`. No generated asset tree was copied. No Godot,
game, native import, Rust build or content regeneration ran in this task.
Runtime validation and Load Game/New Game phase measurements belong to Luna,
coordinated through Kip. Historical timings are not reused as current results.

## Implemented correction

Commit `eb1bf63` changes `src/godot/collision.rs` to write one ordinary native
Shape3D resource for each distinct projected geometry body. Every use references
that resource through the conversion builder's existing `native_resource()`
cache. The finished scenes retain native external resource references; gameplay
does not run a shape loader or interpret source geometry. The generated resources
live under the selected level's `collision/shapes/` and participate in the normal
manifest, collision checks, staged publication and deterministic normalization.
Regenerating one level preserves other levels and authored files.

The shape key includes native type and complete geometry properties. Triangle
winding/coordinate conversion, sphere radii and box dimensions preserve the
existing projection. CollisionShape3D nodes retain their own placement, source
material identity, collision flags, sound response and disabled state. This is
target geometry reuse; canonical source identities and Rete are unchanged.
Support code only reads the shape resources. An authored per-instance geometry
edit can use an ordinary resource override/duplicate.

The change preserves inactive-bank process inheritance and REMOVE disable mode,
native sector indexes, cached BVHs, animation paths and scene node ownership.
It touches no `character.gd`, `src/godot/character.rs`, rendering/material files,
startup lifecycle or save state. No support API change is required.

Godot documents resource reuse through its
[resource cache](https://docs.godotengine.org/en/4.7/tutorials/scripting/resources.html).
[Shape3D](https://docs.godotengine.org/en/4.7/classes/class_shape3d.html) holds
geometry, while [CollisionObject3D shape owners](https://docs.godotengine.org/en/4.7/classes/class_collisionobject3d.html)
hold per-use ownership/activation. The native `data` serialization for concave
shapes was also checked against the existing generated scene files. This is
static evidence; loading the new resources in the pinned engine is pending.

## Actual loading path and saved structure

The real game enters `game_session.gd`, which requests Manor through the persistent
`LevelTransition` service. It requests the native level resource on one worker
with CACHE_MODE_REUSE. `open_level_from_transition()` retains that same PackedScene
in `GameContent` before deferring `open_level()`. The latter retrieves it from the
cache, instantiates it once, binds cameras, applies the chosen startup lifecycle,
attaches the tree and waits for physics before entering gameplay. The successful
service route does not issue a second source-scene load in `open_level()`.
`level_preparation.gd` is a separate component, not this startup route.

The dominant structural redundancy is in `export_rig()`: every nonempty normal
and reset ObjectList part is copied beneath every object-bearing channel. All
slots are retained for arbitrary source swaps. Each copied visual part contains
another physical fallback Area and collision geometry. The fix above removes
repeated geometry resource bodies, but deliberately leaves this saved node
duplication for a coordinated ownership change.

`tools/scene_loading_census.py` reads native text scenes without running Godot,
expands PackedScene instances and applies inherited overrides by node path. It
rejects unknown/binary scene representations, missing inherited targets and
cyclic scene references explicitly. It is inspection tooling, never a gameplay
loader. Its counts are distinct from engine-instantiated node counts and timings.

Current local static observations:

| Scope | Saved node paths | Paths under parked banks | Embedded shapes | Distinct shape bodies |
| --- | ---: | ---: | ---: | ---: |
| Hype perso scene | 105,320 | 103,924 | 24,311 | 134 |
| Manor, expanded native scene dependencies | 344,932 | 333,964 | 151,187 | 513 |

Manor's entry file has 316,123 distinct serialized node headers, including its
root. Expanding inherited instance descendants gives the larger count above.
The scene dependency closure contains 803 text scenes totaling 428,501,758 bytes.
Embedded-shape counts sum distinct scene files in that closure, including shape
data saved both in a perso and in level overrides; they are not counts of loaded
Resource instances. Hype alone embeds 24,304 concave shapes with 131 distinct
concave geometry bodies. These are existing-file measurements, not claims about
the as-yet unregenerated output or elapsed loading improvement.

The native animation inventory contains 45 libraries across 42 families, totaling
40,937,301 bytes. Hype's normal/reset pair accounts for 34,467,539 bytes. Current
files do not demonstrate duplicate libraries across separate configuration rigs;
the pairs have different normal/reset anchor targets. Do not remove a restored
library simply on its similar size. Consecutive identical value-track keys are
already collapsed by `NativeAnimation::value_track()`.

Reproduce static inspection from this worktree:

```sh
python3 tools/scene_loading_census.py /home/stephen/Projects/astrolabe/godot res://generated/hype/levels/manoir/level.tscn --output output/loading-static-20261009/manor-expanded-census.json
```

Local receipts are under `output/loading-static-20261009/`: `scene-census.json`,
`hype-shape-census.json`, `manor-expanded-census.json`, `library-census.json`,
`luna-load-failure-phases.json` and `static-checks.json`.
Game-derived scene contents remain local. Only aggregate findings and reusable
inspection code are committed.

## Coordinated next correction and native validation

The larger proposal sent to Kip preserves every native channel and normal/reset
slot anchor and keeps initially selected parts in the saved actor scene. Inactive
payloads become shared native PackedScene templates. Selecting a configuration
or swapping a previously unused original slot instantiates its converted scene
synchronously before attaching it to the stable animation target; no deferred
missing-state window or source-table reconstruction is acceptable. The templates
must retain original-slot identity and contact/material semantics. Selected
payloads remain actor/channel instances and return to disabled parking banks.

This needs coordinated ownership of `export_rig()` and
`_apply_source_configuration()` plus a bank component. It was not implemented in
this patch. Material opacity/display state applied before first selection must
also reach a new payload; the current rendering component caches already-seen
material instances, so naive lazy instantiation would miss that state. Review
this with the shield worker before changing payload activation. No generic
fallback should conceal a missing template, wrong slot identity or missing bank.

Kip's integration should schedule one focused Rust compile/unit run, then a
Manor-only regeneration/native import in the single runtime slot. Luna should
measure resource load, PackedScene instantiation, attachment/ready, save-state
application and physics readiness for the actual New/Load routes, keeping cold
and warm starts distinct. Observe transition-service resource loading directly:
session stage signals are cumulative, and the successful service route has
already loaded the resource before `open_level()` emits its restoration stage.
Check saved/native shape Resource identity across multiple channels/actors,
independent disabled/material state, selected-part contacts, source-sector query
behavior and initial editor-visible poses. No measured speedup or gameplay
acceptance is claimed here.

Luna's unchanged-content Load Game failure receipt is
`/home/stephen/Projects/astrolabe/output/runtime-validation/luna-20261009/load-report.json`,
on `12ad58ce`, Godot 4.7.2, manifest hash
`2980c8f9c174c6137946a901d817b5bb8b197ca0abfad0546651fb10487de5cc`.
It records these intervals:

| Recorded interval | Duration |
| --- | ---: |
| Transition request to restoration-entry stage, including resource loading | 53.513s |
| `resource_ready` to `instantiated`, including camera binding | 2.091294s |
| `instantiated` to `tree_ready`, including startup preparation/tree attachment | 0.983948s |
| `tree_ready` to `failed` | 0.361751s |

Restoration fails with `Saved inventory item is outside the source catalogue.`
at observer 57,293ms. The process continues until the observer's 100-second deadline;
receipt elapsed time is 100,260ms. This is not a successful 57-second load.
These stage intervals also do not isolate pure resource loading, pure
instantiation, pure `_ready()` cost or complete save restoration. They establish
that the interval containing resource loading dominates this failure attempt,
supporting resource-sharing work ahead of broader startup edits. No after-change
timing exists. The locally derived summary records the source receipt's SHA256;
the runtime receipt itself was only read and remains Luna-owned. New Game was
still running at inspection and is not treated as a result.

## Checks and limitations

- The focused Rust test is registered for the `scene` and `loading` areas and
  release inventory: shared triangles/spheres/boxes, source material/placement
  independence and radius edits. It is written but not built or executed.
- Three synthetic Python inspection checks pass in 0.002s; six existing selection
  checks pass in 0.015s. They execute no engine and read no game data.
- Changed-file rustfmt and `git diff --check` pass. Whole-repository
  `cargo fmt --check` exits 1 on untouched baseline formatting in Godot AI,
  model AI, OpenSpace AI/content/scene/dialogue files; the complete log is retained.
- No selected/full runtime suite ran, and no required safeguard is represented
  as passed. Build, native parse/import, regeneration and timings remain pending.
- Two app-tool attempts to message the supplied coordination source thread
  returned `thread not found`; plans/requests were shared through commentary.
  No unverified worker ID was messaged. Worktree/commit Git writes required the
  authorized sandbox escalation because `.git` is read-only in the default shell.
  The escalation succeeded; no permission rejection was bypassed.
