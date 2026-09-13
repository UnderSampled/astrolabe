# Rust implementation evidence

The Rust implementation covers milestones 0–5, the native scene/character slice of 6–7, and a first executable AI slice of 8. The complete local suite passed on 2026-09-13: **86 tests in 497.572 seconds**, including 104 seconds of release compilation, every corpus/Godot check, actual Scene dock inspection and editor scrubbing, authored regeneration, AI execution and cleanup. Report: `TestResults/20260913T185841.641467Z-25665.log.json`. No processes leaked. This is a converter for supported useful assets and native Godot project files; full level/game behavior remains incomplete. Game data, generated content and reference binaries remain local.

See [the Rust contributor guide](contributing-rust.md) for the current module map and extension workflow. The readability refactor moved root traversal/export rules into their adapters, split level and texture responsibilities, unified texture sharing, and separated geometry from instance state. Audio reconstruction gaps are typed data; explanatory notes no longer control writer support.

The native scene/character migration is accepted for the supported English PC
Astrolabe fixture. See [the migration report](native-godot-migration.md) for
native representation, ownership, actual editor and source-game evidence.
Milestones 6–7 remain partial; [remaining work](remaining-work.md) records the
open semantics and reconstruction work. The first [AI-to-GDScript slice](ai-gdscript-slice.md) executes the two torch
sound controllers, with independent state, native audio and source-game evidence.

## Native Godot scene/character migration

The supported fixture now exports saved level/perso scenes with initial parts,
poses, visibility and native Animation/AnimationLibrary/AnimationPlayer tracks.
The Scene dock exposes nested instances in both the preview and level; actual sidebar rows are checked alongside native animation seeking. The former character and sound JSON interpreters have been removed. Conversion
resolves transform pools, changing hierarchies and object configurations;
maintained Godot support controls native playback and compiled sound behavior.
The committed `godot/` starter separates authored content, support and ignored
regenerated content. Initialization, support updates and regeneration have
separate ownership rules. Generated resource/node IDs and sidecars are stable.

Focused tests and the actual Godot editor verify affine/shear values, duplicate
logical channels, native seeking/backward seeking, configuration restore, state
speed, fractional loop boundaries and cue-driven state changes. Direct and
reopened-Rete output agrees at the current scene checkpoint. The complete suite,
authored regeneration and visual inspection passed; see
[native migration evidence](native-godot-migration.md) for the exact status and
original-engine walking comparison. Milestones 6–7 remain partial: this projection
does not complete source reconstruction, morphs, interpolation or gameplay.

## Demonstrated coverage

| Milestone | Result and evidence |
| --- | --- |
| 0: interfaces/harness | One library/CLI, separate OpenSpace/Rete/Godot modules, shared typed references and Load/Save contracts, OpenSpace FormatCodec. Constructed shared/forward references, distinct equal values, session A/B/A binding, exact float bits, schema rejection and actual Godot synthetic scenes tested. Two clean outputs agree and authored wrappers survive edits/regeneration. |
| 1: LZO | All 26 populated Fix/astrolabe SNA/RT envelopes independently decoded: 17,922,573 bytes. New Rust-compressed streams also decode with system liblzo2. Checksums, bounds, corruption and empty-stream handling tested. All 26 historical compressed streams differ from the new encoding; exact historical compression remains late work. |
| 2: textures | All 3,578 GF members (3,555 main, 23 vignette) reconstruct exactly. Both CNTs reconstruct identically after normalizing original XOR obfuscation. Membership, aliases, order, offsets and checksums are tested. Closed/reopened Rete reconstruction uses 3,366 files: 3,364 independently stored images and two archive-root assets. Resizing one shared image changes both named aliases and regenerated placement without affecting the other archive. |
| 3: audio | Imports all 39 standalone APM files, 199 BNM banks and 87 CSB/LCB/SIF definitions. Banks expose 2,573 embedded and 224 external sound resources, including 264 separate surround components. Rete retains exact useful sample values and timing. Actual Godot mixer probes check effects, ambience/loop wrap, English voice and Astrolabe music; the four-fixture Rete/mixer test completed in 7.72 seconds before the final suite; supported APM/PCM-bank and semantic sidecar writing is tested. |
| 4: rooted access | SNA/RT resolution loads shared Fix with one level and follows GPT scene roots. The selected level has 150 rooted nodes and 22 rooted model candidates. Source block/RT hashes, null/missing/conflicting references, Fix/level binding and sentinel cases are tested. Verified SNA framing and PTX header/pointer/channel declarations generate controlled output with derived pointers. |
| 5: model/Godot | The first rooted model has 26 positions, 32 triangles, four materials/textures, one selected LOD and 26 signed radiosity colors. Native ArrayMesh/PackedScene resources render in Godot. Direct conversion and closed/reopened Rete produce identical generated bytes. A persisted position edit changes the actual Godot array and independently inspected source fragment. Source reconstruction covers 75 generated pointer sites, PTX/RTT names and selected model/material/geometry/IPO/ISI data, plus derived SNA fragment framing and four dependent GF files. |
| 6: scene, in progress | All 150 rooted nodes, 22 triangle/sprite IPOs and ZDR collision bindings survive Rete and Godot conversion. Twenty collision bindings reference their shared render model; two own independent meshes. Native raycasts check placement, hierarchy and disabled shapes. Canonical game materials contain optional visual data, collision properties and sound-material IDs. Basic sprites, 54 shared lights and 17 sectors import. The sectors share one environment, visual environment and fog asset; Rete retains all three identities, source fog bytes reconstruct exactly and the saved Godot level applies uniform fog. [MCP and pixel evidence](environment-fog.md) verifies the source values and records the rendering approximation. Thirteen sector boundaries retain triangle/UV/normal/subset data and neighbor links; ordered graphics, optional graphics, collision, activity, perso/light membership and sound-propagation links persist. Controlled scene and sector writer APIs regenerate supported records using caller-supplied component targets. All 150 full 64-byte SO records reconstruct hierarchy, source-kind identity, opacity and initial matrices; 145 picking-bound references retain 58 shared assets. [Scene hierarchy evidence](scene-hierarchy.md) covers Rete edits, embedded character matrices and native world/local placement. These APIs are not yet composed into CLI scene assembly. Godot portal queries match an original-engine sector crossing. Animation and supported torch-AI sound actions resolve typed programs and 179 shared waveforms; native Godot probes cover material/random selection, sample loops and event-targeted stop. Advanced sprites, exact lighting and per-sector fog routing, remaining ambient/AI-triggered scene audio, and full scene reconstruction remain incomplete. |
| 7: characters, in progress | 108 persos, 21 families, 17 configurations, 492 animation clips, 436 triangle/sprite models and two transform pools import. Native Godot libraries/players now project channel parenting changes, object switches/hide, state timing and selection into persisted parts and poses. Rete/direct exports agree. Of 204,927 imported compact matrices, 204,474 reconstruct byte-for-byte; 453 derive smaller equivalent forms, verified to preserve transforms. Original pool ordinals preserve sharing and distinct equal entries. 147 shared action assets retain 145 configuration action slots and 593 ordered cues in 304 clips. SND/RTS links resolve into library bank/member references and source fragments regenerate their receiving sites. Godot emits timed action signals and plays supported sound programs. The 56-byte Hype family layout is verified against the engine; configuration writers preserve 2,380 active/reset entries and regenerate model/action links. Godot exposes configuration restore without replaying animation cues. Family/state writers regenerate all 585 state records, 19 transitions and shared configuration links. Eighteen family bounds and ten mechanics cards persist; five cards have typed parameters and five retain explicit unpromoted fields. Godot exposes bounds and selected mechanics for inspection. Related/morph data, interpolation and full perso reconstruction remain incomplete. MCP observations verify selected states and a matched-camera comparison checks environment/character placement, with runtime visibility overrides and approximate lighting explicitly recorded. |
| 8: AI, in progress | One shared typed AI model and two independent torch-controller Brains compile to executable GDScript. Native bindings, 250 ms waits, distance/state branches and stop/play/stop sound behavior pass direct/reopened-Rete and intentional-threshold-edit tests. The 32-node source script reconstructs exactly through Rete with assigned sound handles. Original-engine guarded position changes verify state transitions and wait/restart policy. The other 32 AI models on 53 persos have explicit unsupported diagnostics; rule/reflex scheduling, broader gameplay and full AI/package assembly remain open. See [AI evidence](ai-gdscript-slice.md). |

See [scene/animation research](scene-animation-research.md) for source corrections, original-engine observations, matrix differences and current visual gaps. Scene conversion is the default; `--content model` selects the first-model fixture explicitly. The viewer adds character/state selection, pause, model focus and fly controls; `--show-sectors` displays boundary wireframes and `--show-bounds` displays family bounds without adding physics bodies. A repeatable pose capture accepts `--character Hype --state 47 --frame 0`. Generated fragments now keep `fix/` and `level/` under `fragments/` to prevent cross-role filename collisions.

Evidence lives in [testing.md](testing.md), [texture-rust.md](texture-rust.md), [audio-rust.md](audio-rust.md), [level-rust-research.md](level-rust-research.md), [godot-review.md](godot-review.md), and the corresponding executable tests. Full decoded source/fixture hashes are recorded in those tests. No proprietary bytes were embedded in the repository's fixtures.

## Reproduce

From the repository root with local game data in `disc/`:

```sh
cargo run --release --locked -- port disc output/godot --level astrolabe
cargo run --release --locked -- import-openspace disc output/rete --level astrolabe
cargo run --release --locked -- export-godot output/rete output/godot-from-rete
cargo run --release --locked -- export-openspace output/rete output/reconstructed
cargo run --release --locked -- inspect output/rete --asset level:/models/world/1/0/0 --kind model
cargo run --release --locked -- formats
godot --path output/godot
python3 tools/test.py --local --offline
```

Use `--content textures` for CNT/GF or `--content audio --audio-file Gamedata/World/Sound/Achat.apm` for a focused sound on the import/port commands. Empty audio selection reads the supported sound corpus. Conversion has no Unity dependency and uses no C# production code.

The generated development viewer supports drag/orbit, wheel/zoom, a texture contact sheet and audio audition buttons. For an offscreen software-rendered snapshot after conversion:

```sh
timeout --signal=TERM --kill-after=3 40 xvfb-run -a godot \
  --path output/godot --audio-driver Dummy res://generated/hype/levels/astrolabe/preview.tscn -- \
  --capture /absolute/path/model-preview.png
```

Audio projects accept `-- --audio-check /absolute/path/playback.json`. That probe uses AudioEffectCapture on the real Godot mixer, mutes the probe bus after capture, checks nonzero samples and tests loops across their end boundary. It never records a microphone. These are temporary development harnesses, not durable game runtime assets.

## Toolchain and validation

The complete direct texture export generated 3,367 files in **21.10 seconds**, including source import and both staging/destination Godot imports. Its rendered contact sheet was visually inspected for orientation, recognizable content and transparent regions. A selected 66-sound export generated 135 files in **6.52 seconds**; all 66 produced nonzero captured mixer output, and the source ambience loop wrapped. The model's static radiosity preview was rendered and inspected separately. These local previews are under ignored `output/rust-*-preview.png`; none are source-reference screenshots or claims of original-engine execution.

Verified development toolchain: Rust/Cargo 1.98.1; Godot `4.7.2.stable.arch_linux.ed1daf0bf`; Python 3.14.7. Rendering used Xvfb and Mesa llvmpipe. Runtime dependencies are pinned in Cargo.lock. System liblzo2 and FFmpeg supply independent test oracles; production LZO is pure Rust.

The shared test runner enforces 570 seconds for compilation/tests and reserves bounded cleanup/report time below the user's ten-minute whole-invocation limit. A timeout, lingering child, skipped required prerequisite, or cleanup failure is not a pass. The passing reviewed-refactor run is `TestResults/20260912T162320.929180Z-235583.log.json`: 128.933 seconds total, 684.514 child CPU seconds, 850,516 KiB peak child RSS, no timeout or leaked processes, and complete temporary cleanup. All 55 tests ran with zero ignored/filtered tests; release compilation took 50.36 seconds. Release build, formatting, strict Clippy across all targets, and diff whitespace checks passed. Godot checks ran outside the restricted sandbox because its local sockets and editor settings require that access.

An independent reviewer approved the refactor after a filename-collision fix. OpenSpace export now rejects distinct asset identities that map to the same output filename and deduplicates repeated references. Regression tests cover both texture-root orders and Fix/Level instance collisions.

The initial shared-instance probe completed in 5.86 seconds with actual Godot resources. The initial post-refactor source/direct/reopened-Rete route completed in 17.73 seconds. All 3,578 GF members remain byte-exact, complete CNTs still match after normalizing original XOR obfuscation, and the four-fixture Godot audio mixer check passes. These establish preserved supported behavior through the refactor; whole-level and original-engine acceptance of generated content remain outside current coverage.

## Stage measurements

Before the readability refactor and schema 2, a separate bounded measurement run completed in 16.46 seconds. These are historical measurements, not current file counts: the model closure now adds one independently persisted instance asset. Each stage used the built CLI under `strace -f -c` (file/read/write/seek/sync/rename calls), with per-process-tree CPU, peak RSS and kernel I/O accounting from `wait4`. Times include tracing overhead; these are pipeline stages, not isolated serializer microbenchmarks. Godot setup/import costs are included only in its row.

| Stage | Wall seconds | CPU seconds | Peak RSS MiB |
| --- | ---: | ---: | ---: |
| `textures_to_rete` | 4.780 | 1.568 | 307.0 |
| `textures_from_rete` | 1.227 | 1.070 | 362.3 |
| `model_to_rete` | 0.322 | 0.286 | 182.0 |
| `model_rete_to_godot` | 10.086 | 9.399 | 717.5 |
| `model_rete_to_openspace` | 0.041 | 0.009 | 13.0 |

`textures-rete` contains 3,367 MessagePack files including its package manifest, totaling 191,229,845 bytes. `model-rete` contains 10 MessagePack files including its package manifest, totaling 286,201 bytes. Full syscall-count/time summaries and kernel input/output block counts are local under `output/rust-measure/`. Zero kernel input blocks can mean cached reads; it does not mean no filesystem reads. The complete source-free reconstruction routes consume only Rete/model inputs.

## Runtime reference status

**Original-engine automation is available.** The standalone `/home/stephen/Projects/hype-harness` project now provides the persistent `hype` MCP server registered with local Codex. All 15 tools were discovered by a fresh Codex client and again through MCP after the game tests. The 28.2-second harness suite verified save loading, timed keyboard control, screenshots, mana consumption/refill (30 → 20 → 50), guarded memory write/restore, failure handling, and client reconnection. The private Gamescope/Wine window uses dgVoodoo 2.79.3 and DXVK 2.6.2 at 1024×768/48 FPS, with videos disabled and no desktop focus capture on launch. The controller and game copy remain outside Astrolabe; see the [comparison plan](../plan.md#available-original-engine-comparison-harness) and [harness README](/home/stephen/Projects/hype-harness/README.md).

Further source-game tests activated the hidden manor lever through keyboard input after temporarily moving the lever into reach, then restored its position without patching cheat flags. All 36 teleport destinations loaded, including `tele11` → `astrolabe`. A fresh-fixture Fire Trinity test measured a 35-mana cost, steering, pause/resume, and active-ball count changes. The supplied save files remained byte-identical to the archive. The [test report](/home/stephen/Projects/hype-harness/HIDDEN_BUTTON.md) records hashes, sequences, memory interventions, screenshots, and camera/input/quest-state limitations; its 272-entry catalog distinguishes bindings from proven effects. Evidence is local under the harness's `local/hidden-button-research/` and `local/runs/`. These are useful original-game baselines, **not acceptance of Astrolabe-generated packages or established Godot gameplay parity**. The test game was stopped; the MCP service remained available. Use this tool for relevant comparisons rather than treating original-engine setup as unavailable.

The 2026-09-12 Lutris setup installed English Hype in `/home/stephen/Games/hype-the-time-quest-win32`, using `wine-ge-8-26-x86_64`, a true `win32` prefix, `mfc42`, `icodecs`, and nGlide 2.10 with Vulkan. The default Proton runner had forced the supplied installer's requested 32-bit prefix to 64-bit, preventing `icodecs` installation. A Hype-local 32-bit `libvdpau.so.1` restores the Wine-GE GStreamer/libav decoder; the saved Lutris command also selects the runner's 32-bit GStreamer plugins and NVIDIA for OpenGL/Vulkan. Native DirectShow replacement trials were unsuccessful and were reverted. Original video imagery, the English menu, and gameplay at a portcullis/sign were visually observed on the real desktop. The supplied `SaveGame0/` was copied into `GameData/SaveGame/`, the path recorded in `gamedsc.bin`; the source archive remains unchanged. The user confirmed loading the supplied save and working video/audio. A full New Game intro/conclusion replay remains unverified. Setup details, executable/fixture hashes, launch configuration, logs, and screenshots are local under `output/runtime-validation/lutris-20260912/`. This demonstrates original-game execution, not acceptance of Astrolabe-generated packages or gameplay parity. See [the reference guide](../reference/README.md) for the save fixture and validation requirements.

The existing local Raymap WebGL snapshot opens in the browser. Both the local-data folder attempt and its documented Hype reference route failed with `GAMEDATABIN folder doesn't exist`. Source inspection shows this build's FileSystem uses a hardcoded remote data-server prefix; serving the shell and adding local data symlinks does not establish local-file loading. No generated-level viewing was established, and no Unity Editor installation was attempted. This setup failure did not block codec, source-informed geometry or actual Godot checks. Raymap source was the first practical format reference; conflicting layouts were resolved against the original engine/decompilation and actual fixtures.

## Limits and next architectural review

- OpenSpace export of a model emits a controlled fragment, not a game-loadable full SNA package. Supported SO hierarchy, collision, configuration, family/state, mechanics and animation fragments now reconstruct; full scene assembly, perso/spatial structures, allocator-compatible package assembly, remaining LODs and AI remain incomplete. Unknown/runtime state is described as unsupported rather than replayed to manufacture byte identity.
- Static Godot radiosity uses the verified material branch `clamp(2*(ambient + diffuse*RLI/256),0,1)`. Exact signed/unclamped lighting remains in Rete and mesh metadata; the fourth source component is not opacity. Source-gated color-key cutouts, alpha and additive blending are projected into native Godot materials. Source bilinear filtering reduces the matched opening-scene pixel RMSE by a further 4% after the transparency fix; exact mip selection remains open. Full context flags and exact dynamic lighting remain incomplete, and visual pixel parity is not claimed. See [the transparency and matched-camera pixel comparison](material-transparency.md).
- Original texture mip values remain in Rete; Godot currently imports base PNGs. CNT output deliberately derives an unencrypted representation; original per-entry XOR keys are not cached as metadata.
- APM writing is functional lossy ADPCM re-encoding. PCM-only BNM writing is limited; CSB binary event behavior, Ubi-MPEG encoding and full-bank reconstruction remain explicit gaps. Surround components are preserved separately but target mixing remains unverified. Animation event routing and selected original cue timing are checked; remaining ambient/AI/dialogue routing and original mixer parity remain incomplete; the torch AI sound events now execute natively.
- Reusable geometry and instance transforms/radiosity are now separate model assets. Synthetic shared instances exercise Rete, independent edits, controlled source fragments, and Godot mesh sharing/lighting variants. The explicit model route selects the first rooted instance; the scene route traverses all 150 nodes and preserves observed render/collision geometry sharing.
- Rete schema 10 and generated Godot paths are provisional. Regenerate older packages from source; scene, character, sprite, light, shared animation-action, typed sound-program, configuration reset-table, family bounds, state mechanics, shared environment/fog and full scene-node properties, typed AI/Brain values and the explicit main-actor binding changed the schema. Shared whole-asset loads work; generic direct member hydration is deliberately rejected until owner-specific resolution is defined. Embedded Model/ModelInstance/Material identities must agree with references. A rename must update both explicitly.
- Shared diagnostics have stable codes, asset context and severity; semantic coverage survives Rete persistence. Source address summaries remain transient debug observations. Broaden diagnostic codes and source profiles when new domains need them, not by creating a second wire-object database.
- Godot 4.7.2's ResourceSaver did not retain the mesh path with FLAG_CHANGE_PATH alone; explicit take_over_path is required. Generated node/resource IDs are normalized, and destination import caches are rebuilt. Every authored/stale-file conflict is checked before publishing generated content.
- Generated Godot paths/interfaces remain provisional, but needed converter-owned support and authored consumers may develop now. Establish tested extension points and versioned support incrementally; the committed/rehydrated project workflow is implemented and tested. Game/remaster projects own their authored game assets and depend on Astrolabe for supported conversion and applicable support.
