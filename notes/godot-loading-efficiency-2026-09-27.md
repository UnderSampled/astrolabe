# Godot manor loading measurements — 2026-09-27

## Scope and method

These results use the local English `manoir` export and Godot
`4.7.2.stable.arch_linux.ed1daf0bf` on the same workstation. Godot timings
start when **Load Game** is pressed in an already open menu and end when the
game reports `ready_to_play`. Each fresh-process run starts Godot again with
the project already imported; filesystem caches were not cleared. The headless
menu probe was repeated in two processes. A separate Xvfb/Mesa run timed the
first frame drawn after the playable signal. These are neither cold-disk nor
shipping-build benchmarks. The before and after projects were disposable
copies; local game data and the repository's existing generated project were
preserved. Evidence logs and a compact report are under ignored
`TestResults/loading-efficiency-20260927/`.

| Godot stage | Text animation baseline | Compressed native libraries |
| --- | ---: | ---: |
| Fresh process, Load Game to playable | 29.74, 29.71 s | 17.28, 19.74 s |
| Second Load Game in the same process | 28.14, 27.67 s | 0.291, 0.277 s |
| First rendered playable frame, Xvfb | 30.76 s | 21.14 s |
| First editor import on a copy without `.godot/` | 6.74 s | 7.05 s |
| Generated content files | 5,913 | 4,672 |
| Generated content bytes | 358.0 MB | 81.7 MB |
| Memory retained after returning to menu | about 150 MB | about 443 MB |
| Peak Godot static memory during menu loads | about 475–477 MB | about 477 MB |

The original menu path spent a fixed 5.1 seconds in startup waits. The revised
path prepares the native scene in a Godot loading thread, instantiates it,
applies the provisional saved state, waits for native physics contact, then
starts Hype's Brain. It temporarily caps rendering to 30 FPS while resources
load and restores the prior cap afterward. `loading_stage_changed` and
`load_timings_ms` expose the actual stages; a responsive loading message is not
treated as completion. In the two optimized fresh runs, `resource_ready` was
reached at 16.84 and 19.20 seconds. Instantiation then took about 0.13–0.18
seconds and tree entry plus restoration about 0.25–0.29 seconds. Native
dependency loading is still the dominant cost.

The current warm-cache policy holds one immutable `PackedScene` in the
`GameContent` autoload across reload and menu transitions. Every session gets
new nodes, physics, Brain, inventory and magick state and reapplies the saved
fixture. It accounts for the fast second load and for the roughly 293 MB more
memory retained at the menu. The cache has a `clear()` operation; it does not
grow with repeated loads. Existing independently owned projects need to opt
into the `GameContent` autoload in their own `project.godot`; without it they
still load but do not get this warm-cache benefit.

## Resource representation

The exporter now saves 45 compressed native `AnimationLibrary` `.res` files,
embedding 1,241 named native `Animation` subresources. AnimationPlayer still
exposes these states for inspection and seeking; mechanics and cue resources
retain their external sharing. Build-time text inputs are discarded and do not
enter the published generated-file manifest. The old 1,241 animation `.tres`
files occupied 316.8 MB; the 45 final libraries occupy 40.8 MB. A separate
compressed binary file for each state occupied 53.4 MB and loaded more slowly
than bundled libraries in the disposable native-resource probe. The exporter
keeps the stable path-derived UID and repeat export produces the same manifest
hashes in the focused native-animation test.

A synchronous headless `PackedScene` resource-load probe of the text baseline
took 81.96 and 82.17 seconds on this machine; the disposable bundled-library
experiment took 14.86 seconds. Its setup and scheduling differ from the actual
menu route above, so those numbers are diagnostic, not menu-load timings. The
earlier one-run 38.33-second resource probe remains a separate historical
measurement and is not used as the before value in this comparison.

The candidate binary mesh change converted 440 ArrayMesh files from 1.90 MB
text to 0.63 MB binary; synchronous load changed from 17.42/18.34 seconds
with compressed animation files to 17.36/17.52 seconds. Converting 538
AudioStreamWAV `.tres` files from 9.69 MB to 6.94 MB yielded 17.28 seconds
in the same experiment. Those differences are within run variation. Meshes
and scene files remain text: they are small enough here and their structure
can be reviewed or patched. AudioStreamWAV `.tres` mixes readable loop/rate
properties with opaque encoded samples; it is a reasonable future binary
candidate for patch ergonomics, but did not materially improve this load and
is left unchanged in this pass. Dense sampled animation data is binary because
its text form is very large and its individual numeric samples are poor patch
material. Rete assets remain independently serialized and no generated game
data is committed to Git.

The complete local `port` command took about 3 minutes 42 seconds by its
start/completion timestamps, including source conversion, native-resource
building and Godot import. The isolated first editor import above was measured
separately on a cache-free copy of the finished output. Neither belongs in
the subsequent menu-to-play numbers.

## Original Hype comparison

Three retained Orrery recordings of the supplied populated `SaveGame0/`
fixture reached `manoir` gameplay phase 9 after selecting the save:

| Recording | Save-selection Enter to phase 9 |
| --- | ---: |
| `orrery-save-name-commit-20260927` | 3.04 s |
| `orrery-save-name-commit-gated-v2-20260927` | 2.85 s |
| `orrery-save-name-commit-gated-v4-20260927` | 3.08 s |

In the last recording, gameplay imagery is present during the fade about
3.5 seconds after the selection and clearly visible about 4.5 seconds after
it. Orrery samples phase and frames independently; these are observed bounds,
not a measured exact first-render or input-consumption boundary. The original
game ran under Wine/Gamescope using its installed data and a staged copy of
the supplied save. Its title startup and deliberately timed menu navigation
preceded the measured save selection. The optimized Godot fresh Load Game is
therefore still roughly six times slower than the original's selected-save
load, even though it is substantially faster than the previous Godot output.

## Godot production resource export follow-up

A later experiment used Godot's own `--export-pack` command with its default
text-to-binary conversion. The input was the same optimized manor project,
including the 45 native animation libraries. The export preset included all
resources except the unrelated authored `luminous_hype.tscn`, which currently
references an ungenerated `astrolabe` level. The resulting PCK was 63,647,432
bytes. Its export log lists 742 binary scenes (`.scn`), 1,934 binary resources
(`.res`), 293 compiled scripts (`.gdc`), and 455 imported textures (`.ctex`);
it lists no text scenes or text resources. Both the PCK and a copy of its
source project were placed on the normal Btrfs drive for these measurements.

| Fresh-process run after import/export, menu to playable | Source project | Godot PCK |
| --- | ---: | ---: |
| Headless run 1 | 8.453 s | 7.924 s |
| Headless run 2 | 8.434 s | 7.621 s |
| First rendered playable frame, Xvfb/Mesa | 8.729 s | 8.139 s |
| Second Load Game in the same process | 0.148–0.152 s | 0.166–0.168 s |

The same Godot editor executable ran both inputs, with `--path` for the source
copy and `--main-pack` for the exported PCK. This isolates Godot's production
resource preparation and packaging, but does not measure a release-template
executable: `--export-release` could not run because this workstation has no
Godot 4.7.2 Linux export templates installed. The packaged game reached
playable state and drew a frame. The package saved roughly 0.5–0.8 seconds in
the paired headless runs, and 0.59 seconds to the first rendered frame. Most
of the remaining time was still native resource loading: 7.40–7.70 seconds
for the package. This is useful but not a direct-memory-image cache.

The earlier 17–20-second source-project runs and these later 8.4-second runs
were in different machine conditions. The earlier copies and later temporary
copies both lived on a RAM-backed `/tmp`, and the filesystem cache was not
cleared. The cause of that large variation is unverified, so the early source
numbers must not be used as the direct before value for this PCK experiment.
The paired runs above are the relevant package comparison. The PCK's
8.14-second first rendered frame remains above the original game's roughly
4.5-second clearly visible gameplay observation, subject to the different
runtime and timing endpoints. Evidence is under ignored
`TestResults/loading-production-20260927/`; the PCK contains local game data
and is not committed.

## Verification and limits

The maintained exporter completed a fresh full local manor port with 45
animation libraries and no published per-state animation files. The focused
native-animation test checked deterministic repeat output and Godot animation
seeking/cues. The full game smoke used the actual menu button and physical
input: Hype moved 1.926 units, reload produced new actor and inventory
identities, inventory/magick/velocity/Brain state reset, the temporary raised
gate did not lift twice, and Esc returned to the menu. A rendered Xvfb/Mesa
capture showed the playable scene after reload. The baseline and revised
editor imports both completed. Existing unsupported behavior errors from
other NPCs remain and this work does not establish whole-level gameplay
parity.

The selected scene/animation development supervisor passed 52 of 53 tests on
the 720-second bounded rerun. The one remaining test,
`source_scene_rete_routes_preserve_hierarchy_collision_and_model_sharing`,
timed out at its 360-second per-test limit while running three complete Godot
exports and editor checks; the suite therefore **failed**, despite the other
52 passes. The first 480-second run had passed 49 cases before its overall
budget stopped that same test and left three cases unrun. Both reports record
complete child cleanup and unchanged source. This is an unresolved validation
limit, not an assertion pass. The focused deterministic native-animation test
and the full local manor port/gameplay smoke passed separately.
