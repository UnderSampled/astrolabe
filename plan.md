# Astrolabe implementation plan

**Make a tool to repeatably port this OpenSpace game to Godot.**

This is the current Rust plan, 2026-09-12. The initial Rust implementation targets milestones 0–5; see [the implementation report](docs/implementation-progress.md) for evidence, measured checks, and explicit remaining limitations. The milestone requirements below remain the acceptance criteria. The former C# plan and completion claims remain in [the archived snapshot](reference/astrolabe-csharp/plan.md); they are research evidence, not Rust progress. Read [the architecture](docs/rust-architecture.md) and [Rete design](docs/rete-design.md) before implementation.

## Priorities and completion

Godot is the primary deliverable. Work toward useful, repeatable conversion of Hype content and behavior, suitable for a modern fan-remaster and separately authored Godot extensions. OpenSpace reconstruction provides byte-level evidence and supports modifications of the original game. A full Godot-to-OpenSpace edit round trip is optional and deferred.

**Architectural priority: keep OpenSpace, Rete, and Godot as separate modular I/O adapters around one useful intermediate model.** OpenSpace reads/reconstructs game data, Rete loads/saves the model, and Godot exports it into a modern engine; Godot import remains optional future work. Application code composes these adapters through shared model and resolution contracts. Adapter separation is required; the number of Cargo crates is an implementation choice. See [the architectural overview](docs/rust-architecture.md#three-modular-io-targets-one-useful-model).

Minimal source metadata records facts such as identity/sharing and original order. Export derives encoding, alignment, sizes, placement, actual pointers, checksums, and relocation tables from current content and verified Hype rules. Do not add cached encoding decisions, offsets, or source-blob replay to make parity tests pass. Exact pre-compression package reconstruction is a required long-term target. The remaining historical LZO encoding difference is a separate final research gate.

**Defer long-lived converter-owned assets and durable game/remaster dependencies until OpenSpace import/export, Rete load/save, and Godot export all work for the supported game content.** Godot reverse import remains optional. Temporary development/test assets are allowed. Downstream game-parity and full-working-game projects depend on Astrolabe and own their durable assets, even when packaged together.

Before implementing a new domain, review asset boundaries, package/reference context, retained information, and all three adapters against representative cases. Record provisional choices and the evidence that would justify revising them. Do not let an early codec silently decide the architecture.

Every milestone delivers an observable result and records its limits. Rete schemas, OpenSpace readers/writers, Godot adapters, and tests grow together around useful assets. Full-package reconstruction must not block the first useful Godot result. Do not count unknown opaque data, an empty script, or an unanimated mesh as supported gameplay.

Restore **working LZO compression/decompression, textures, and audio first**, before the main OpenSpace level parser. The user reports complete working texture/audio import in C#, and preserving it is the reason for this priority. They are outside the level package; their simplicity must not determine the whole interface. Use C# source, local fixtures, and decoded results as regression references, preserving known working coverage while corroborating interpretation. The original engine/decompilation is the source of truth, then Raymap; Raymap is usually the first practical reference consulted. C# and historical notes remain useful but are not definitive. See [reference authority](reference/README.md#reference-authority-and-research-workflow). Prefer suitable existing crates for standard codecs, with Hype-specific adapters and measured compatibility checks. See [library reuse](docs/rust-architecture.md#library-reuse-and-early-delivery).

**One useful model, declarative codecs, Rete persistence.** Rete is the serialized form of the intermediate model. binrw declarations and reusable helpers document and implement how OpenSpace encodes its meaningful fields. The shared interface loads meaningful content from packages into memory and converts/saves it through the selected target, with common resolution, validation, diagnostics, and fixture tooling. A narrower OpenSpace `FormatCodec` pairs binary reads/writes of useful types. It does not impose OpenSpace parameters on Rete or Godot. Source reference resolution and semantic interpretation are part of content decoding. Temporary binary records support encoding where necessary; there is no required separate OpenSpace document model. Serde/MessagePack saves and reloads those same values. Apply [the declarative codec and unified interface contract](docs/rust-architecture.md#binary-codecs-with-binrw) throughout the milestones below.

## Ordered milestones

The order expresses priorities and real dependencies. Once the common format contract is established, independent formats can be documented and implemented alongside one another; completing every variant of an earlier format is not a prerequisite. Keep LZO, textures, and standalone audio first, and integrate their useful Godot slices as they become ready.

### 0. Establish interfaces and a repeatable harness

Establish the shared `model`, separate `openspace`/`rete`/`godot` adapters, and thin application/CLI orchestration described in the architecture. Choose a minimal Cargo package layout that preserves those module boundaries; a five-crate workspace is not required. Define small reference/root-context interfaces and diagnostics without scaffolding all domain types before the first asset. Pin dependencies, including binrw for OpenSpace codecs, and record the supported Godot version. Establish the package-content load/convert/save contract across the three adapters, with explicit supported directions and common diagnostics. Keep a typed OpenSpace binary codec contract and binrw conventions inside that adapter. Test the proposed interfaces with a small synthetic level-style graph containing shared/forward references, distinct equal-valued assets, Fix bindings to one active level, and an edit that changes generated sizes/pointers. This small architectural probe uses synthetic data; real level formats remain in their later milestones. Refine it against real texture/audio codecs in milestones 2 and 3; milestone 1 supplies stream/envelope primitives. Grow field meanings and encoding declarations together. Use disposable Godot fixtures and provisional naming; preserve unrelated/authored files on regeneration without promising a stable downstream API.

Serialize that same synthetic model asset through Serde/MessagePack with named fields, references, exact numeric values, and embedded dense arrays/byte buffers. Use MessagePack binary fields for raw byte payloads; there is no separate generic binary-content file or `.bin` sidecar. Verify save/load, useful inspection, and format version rejection. Exercise Rete and Godot APIs with the constructed model asset independently of any OpenSpace import; application code owns the composed conversion route. Measure per-asset serialization and file I/O without constructing a global pool or micro-file forest.

Discover the actual Lutris/Wine game installation using the supplied installer reference. Use the supplied `reference/Savefile_EN.zip` as the first-choice post-intro fixture in a separate runtime-test copy; preserve its `SaveGame0/` structure and confirm the installed edition/language and actual save root before installing it. Archive integrity has been checked, but loading the save in the engine has not. Record source edition, language, executable, fixture hash, selected save, and resulting scene/state. Add a narrow memory harness or another chosen save only when a test needs a state this fixture does not provide. Do not make playing through the slow intro a prerequisite. Verify Raymap's web/local-level workflow if available. Consult Raymap first for practical format/viewer work; resolve conflicting interpretations against the original engine/decompilation. Timebox runtime setup and record unavailable checks. Original-engine runs are opportunistic throughout the plan, not milestone prerequisites; use Raymap where supported and source-informed fixtures elsewhere without claiming an unperformed engine comparison.

**Exit:** the small synthetic content graph survives meaningful decoding, a size-changing edit, Rete save/load, and re-encoding with correct sharing/references. Its generated Godot resource imports, two clean conversions agree under the provisional mapping, and a disposable wrapper survives regeneration. Document runtime setup status and unresolved interface decisions. No real game-format support or durable downstream API is claimed.

### 1. LZO compression/decompression and envelope primitives

Restore LZO support as a small reusable codec before semantic level parsing. Use the archived `OpenSpaceLzo.cs`, `OpenSpaceChecksum.cs`, and LZO 1.08 compressor integration as references. Evaluate an existing crate against real compressed streams and decoded results. Use binrw for supported envelope/header records. Keep Hype checksums, compressed-stream framing, and declared-length handling in the `openspace` module; the LZO codec handles the compressed payload and need not understand a level or Rete asset.

Implement bounded decompression and working compression with focused stream/envelope fixtures. Compare original-stream decoding to the reference, and independently decode newly compressed output rather than relying only on one codec's self-round-trip. Test corrupt/truncated streams and output-length mismatches; establish any valid padding rule instead of copying the old decoder's truncation behavior without checking it.

**Exit:** real streams decompress to the expected bytes; newly compressed streams independently decode to the input; supported framing/checksums are regenerated and malformed data is diagnosed. Record compressed-size/byte differences. Check original-engine acceptance when the harness and a suitable rebuilt file are available; otherwise record it as unverified. Matching the historical encoder's exact byte choices is deferred to phase 13 and does not block textures or audio.

### 2. Texture assets: CNT/GF -> useful textures/Rete -> Godot

Restore the established C# `CntReader`/`GfReader` behavior: the real texture CNT directory/member reader, XOR/checksum handling, and GF decoding for Hype's observed palette/direct-color types. Use binrw for CNT entries and GF binary headers/records, with explicit XOR, pixel decoding, and encoding helpers. This slice reads texture archives directly and does not wait for SNA relocation or scene parsing. Build useful texture/material-image types, preserving exact image information needed by later encoders. Keep encoding/alignment rules in codecs. Distinguish `Textures.cnt` and `Vignette.cnt` from lowercase `World/Levels/fix.cnt`, which is a disc/catalog protection file rather than a texture archive.

Assess all required image information before choosing the Rete representation: include observed palette/index identity, mip data, and exact values where needed. Use structured MessagePack data when PNG (or TGA/TIFF/DDS) cannot preserve that information. Persist referenced texture assets and inspect them; generate PNG for Godot where suitable. Generate a Godot contact sheet or simple textured scene to check orientation, alpha, palette behavior, and pixel values. Implement GF encoding and archive writing for the supported subset as tests become available; derive member offsets/checksums and ordering from structured content plus minimal member-order metadata.

**Exit:** real textures display correctly in Godot; Rete reload retains all required image values and reconstruction facts; supported GF members reconstruct byte-for-byte before any unresolved compression distinction. Rebuilt CNT tests cover offsets, membership, and checksums. Whole-corpus/unsupported types remain enumerated.

### 3. Standalone audio: APM/BNM -> useful sounds/Rete -> Godot

Restore the working C# audio path before level parsing: standalone APM, BNM versions/entries and PCM/APM streams, then Ubi-MPEG/MPDX including observed surround wrappers. Use binrw for the APM/BNM container headers and entries, keeping sample decoding and custom bitstream adaptation in audio codec helpers. Use the archived audio readers/decoders and vgmstream as regression references. Generate WAV for Godot where suitable; choose the model/Rete representation to retain all required precision, channel, timing, loop, and reconstruction information. Compare sample rate, channel order, sample counts, loop boundaries, delay trimming, and decoded samples. Classify numeric decoder differences explicitly.

Reuse standard audio/WAV crates where compatible. Hype's Ubisoft MPEG framing needs an adapter before a standard MPEG decoder can consume it; standard MP2 support alone is insufficient. Preserve the established frame, channel, and timing behavior with representative fixtures. Add independently readable CSB event/resource scripts, LCB groups, and SIF material interactions as useful named events and sample references. SNA-dependent SND/SDA/RTS links and gameplay bindings belong to phase 6, so they do not block decoded media or isolated Godot playback.

Persist semantic sound/bank/event assets and references to sound content; keep any richer required sample or codec-domain values in the MessagePack assets. Exercise derived playback media in a disposable Godot audition scene. Pair supported readers with bank/sidecar writers that derive offsets, pointers, and checksums. Keep exact PCM/container tests distinct from lossy audio re-encoding research: no original encoded-blob replay, and no assumption that WAV alone establishes original MPDX bit identity. Unresolved encoded fidelity must not block restoring useful audio conversion.

**Exit:** representative music, effects, and voice samples play in Godot with tested channels, duration, and loop settings; Rete reload preserves their values and links. Decoder results and supported source-file reconstruction compare against the reference. Remaining codec cases, exact re-encoding gaps, and scene/event integration are recorded separately; a decoded bank is not a claim of complete gameplay audio support.

### 4. OpenSpace source access and a rooted model selection

Build SNA block access on phase-1 LZO/envelope/checksum primitives, then implement the RT entry format, RTB/RTP/RTT import, GPT roots, and PTX texture-name resolution. Load shared `Fix.*` alongside the one active level and resolve `fixlvl.rtb` bindings in that session. OpenSpace never requires simultaneously active levels. If batches reuse caches, test sequential level A/B/A loading for stale Fix bindings. Implement these wire records with binrw over bounded source views, with clear unsupported-field diagnostics. Keep relocation resolution in the importer and pointer/layout generation in the exporter.

Implement only the scene/physical/geometric headers needed to identify a meaningful model and its material dependencies. Address maps are transient importer machinery. Independently test relocation resolution with cross-block, Fix-to-level, null/sentinel, and invalid-reference cases; do not persist old address-indexed records as Rete assets.

**Exit:** select and inspect a real model's dependency closure from the rooted scene, with texture links resolving to phase-2 assets. Decompressed source blocks and supported RT entries match independent reference observations. SNA framing/RT emission primitives exist for focused reconstruction tests; a complete package writer is not required.

### 5. First complete model port

Define useful model, material, and submesh APIs. Implement vertices, normals, UV/index relationships, material flags, bounds, supported physical-object/LOD relationships, and relevant radiosity colors. Validate source layouts against the selected Hype assets rather than assuming all cross-game notes apply.

Complete the trip both directly and through Rete: OpenSpace -> model -> Godot and OpenSpace -> model -> Rete -> model -> Godot. Emit native scenes/resources with ArrayMesh geometry and actual textures. Keep coordinate conversion in the Godot adapter. Start with exact wire readers/writers for the selected model's structures, generating pointers and relocation entries from the current model.

**Exit:** a textured real model opens in Godot, agrees with OpenSpace/Raymap on geometry/materials, and survives a deliberate material or geometric edit. Both conversion routes agree. Controlled-layout reconstruction tests cover the model records/buffers without requiring the surrounding level to be complete.

This is the first complete 3D model milestone, following usable texture and audio exports. Do not postpone it until every file extension is parsed.

### 6. Static level scene, collision, lighting, and sound bindings

Expand GPT/root traversal into useful scene nodes, instances, sectors, IPO/physical-object relationships, collision meshes, `*pgb.bin` where applicable, lights, fog, and material/radiosity behavior. Preserve source semantics while implementing the Godot scene/collision/shader mapping.

Build deterministic scene and node paths with stable material/model references. Implement a small inspection/camera harness in Godot. Compare a fixed scene/camera with Raymap and, when available, the original engine; record the reference used and known differences. Pixel equality is a useful measurement, not a required Godot outcome. Separate prelit colors from dynamic lighting approximations.

Integrate the phase-3 sound assets and event definitions with SND/SDA and RTS relocations. Resolve scene/perso sound references, spatial settings, ambient emitters, and event routing into Godot playback. Test loop behavior and selected ambient/action events using documented source expectations and available references, adding original-engine comparison when feasible; defer AI-triggered behavior completion to phase 8. Generate supported sound-sidecar pointers from semantic links.

**Exit:** an inspectable level environment loads in Godot with tested transforms, model sharing, material assignment, selected collision behavior, and scene-bound audio. Screenshots and geometry assertions identify known visual differences. Supported scene records reconstruct under the local export layout.

### 7. Families, perso instances, and animation

Implement `Family`, states/transitions, `ObjectList` model configurations, `Perso3dData`, Montreal animation frames/channels/hierarchies, compact transforms, speed matrices, and mesh-switch/hide behavior. Preserve Fix-owned templates and level-specific configurations without freezing a shared family to the first imported level.

Associate transforms with useful animation content and retain minimal original pool indices for reconstruction. Derive compact encoding and alignment from values/profile rules; test matrices representing each observed form. Preserve shared entries and distinct entries with equal values. Regenerate matrix pools from scoped original order, not semantic traversal.

Exercise Godot animation export and temporary runtime support for dynamic channel parenting, model switches, sample timing, and state selection. Preserve these semantics instead of forcing all data into a static skeleton; durable game runtime assets remain downstream work.

**Exit:** a character plays representative idle/movement/action sequences in Godot; channel transforms, hierarchy changes, visible parts, and timing are checked against Raymap/source-informed expectations, supplemented by original-engine runs when available. Rete retains the same animation data; local matrix/animation reconstruction is byte-exact for the covered cases. Regenerating does not rename referenced states or nodes.

### 8. AI, interactions, and a playable behavior slice

Implement shared AIModel/Behavior/Macro types; Brain/Mind, designer-variable definitions and instance values; typed script trees; state transitions; perso physics/collision/sector membership; waypoints and relevant action bindings. Resolve text, sound, perso, state, and object references into semantic links. Pull the small localization subset required for the chosen behavior from phase 9 when needed.

Choose one real interaction/character behavior and exercise export of all operations it uses in a disposable Godot development runtime. Start with an interpreter over the typed IR and an explicit operation dispatcher. Define tick timing, blocking/meta actions, state changes, random-seed handling, and persistent instance state. Do not count S-expression output as execution support or silently stub unimplemented actions.

**Exit:** a reproducible behavior sequence runs in Godot with state/event traces checked against engine/decompilation-informed expectations and available comparison runs. Report gaps where no runtime reference could be exercised. An intentional parameter/script edit changes the expected behavior. Script serialization/reconstruction tests preserve supported values, reference targets, and tree structure. Unsupported operations are listed with affected assets.

### 9. Localization, remaining sidecars, menus, and configuration

Complete DLG/LNG and RTD/RTG, language-specific Fix/level bindings, text tables and TRN where confirmed, and all remaining SND/SDA/GPT/PTX fields. Implement required INI/MEM/CFG/VGM semantics for port configuration, options, menus, and credits. Classify each from actual source use; historical catalogue names are not a substitute for verification.

CFG game options are distinct from the supplied save-file fixture used to bypass the intro; a general original save editor is not required. AVI/cutscene conversion is deliberately deferred to phase 11.

**Exit:** selected language/dialogue and menu/option paths work in Godot with their dependencies. Source-file tests cover supported sidecars and remaining undecoded fields are visible. The converter handles explicitly selected locale deterministically.

### 10. Expand adapter coverage across the game corpus

Grow each domain's supported cases across levels, shared families, script operations, materials, sounds, and languages. Track read/write/export coverage and the structural/behavioral evidence for each. Use disposable Godot scenes/scripts/shaders and development harnesses to expose gaps. Do not introduce durable authored game/remaster assets or freeze their dependency contract here.

Run single-level, subset, and whole-game conversion as sequential level selections in different orders. Verify shared references, session binding isolation, collision detection, repeatable naming for the current schema, safe stale-output handling, and preservation of unrelated/authored files. Perform a clean rebuild with Godot import caches removed. Keep game-derived output local.

**Exit:** the supported corpus exercises the three adapters through development projects with an explicit coverage report, including pending OpenSpace writers/fidelity work and cinematics. This is converter integration evidence; a full working game or stable downstream asset API is not claimed. Complete the remaining adapter coverage before beginning durable downstream projects.

### 11. Late media work: Indeo AVI cutscenes

The user identifies the original AVIs as Indeo-encoded. Defer their conversion until the core asset/gameplay port is established. Inventory the real streams with `ffprobe`, then evaluate `ffmpeg` transcoding to a format supported by the pinned Godot build. Built-in [Ogg Theora playback](https://docs.godotengine.org/en/stable/classes/class_videostreamtheora.html) makes `.ogv` a candidate; confirm quality, audio, and playback with a real sample before fixing the profile. See the [FFmpeg transcoding documentation](https://ffmpeg.org/ffmpeg.html).

Use deterministic generated paths for `LOGO.AVI`, `HypeIntro.avi`, and `HypeConclusion.avi`, and record the tool version/options used. Validate frame timing, duration, aspect ratio, color, audio synchronization, and cinematic start/skip/completion behavior in Godot. Implement video reference/RTV or RTL structures only when discovered in the supported edition and needed. Keep this Godot transcode separate from any claim of recovering an original lossy encoded stream.

**Exit:** the selected edition's required cinematics play correctly in Godot and repeat conversion reproduces the development output arrangement for the same versions/options. No video probing, transcoding, or playback was performed during this planning update.

### 12. Complete OpenSpace reconstruction and pre-compression parity

Consolidate the domain writers already developed into package assembly. Finish verified block/base/layout rules, remaining understood fields, stream membership/order, and all required generated relocations, including sentinel/fringe cases. Resolve known data instead of adding a generic pointer/blob passthrough architecture.

Export from Rete in a process/context without source-disc access, then compare externally against original decompressed files, block headers, references, and relocations. Require unedited byte identity for the complete supported package before compression. Check small intentional edits with meaningful content and structural assertions alongside byte comparison. Try selected mods in the original engine when the harness supports them; use Raymap where suitable and document unverified runtime behavior when neither can exercise a case.

**Exit:** complete pre-compression parity for the supported corpus and verified structural/semantic effects of selected edits, with available runtime evidence and its gaps reported separately. Full package work consolidates prior tests; it does not retroactively define the canonical asset shape.

### 13. Close compression differences and assess optional reverse editing

Build on the working compression/decompression planned for phase 1. Investigate the historical LZO encoder difference with explicit fixtures and mismatch reports. Confirm decode correctness separately from matching the original compressed byte sequence; check engine acceptance when a suitable harness is available and report it separately. Research other unresolved encoded-media fidelity separately; do not classify every remaining difference as LZO.

**Exit for exact reconstruction:** matching compressed/container bytes for supported unedited source packages, with no replayed source streams. If an encoder discrepancy remains, document it precisely rather than claiming full bit identity.

After the repeatable Godot port works, a separate proposal may support a constrained Godot -> Rete -> OpenSpace editing workflow. Stable asset/member paths and retained minimal provenance keep that option open; arbitrary Godot content and a new OpenSpace game are outside this plan.

## Gate for durable downstream projects

Review readiness once OpenSpace import/export, Rete load/save, and Godot export work for the supported game content, including the relevant level domains rather than only standalone media. Keep coverage and remaining fidelity gaps explicit; exact historical compression matching is tracked separately. Godot reverse import is not required. Until this gate is met, only development/test assets may accompany adapter work.

Then plan game-parity, full-working-game, and fan-remaster work as downstream projects that depend on Astrolabe and own their durable assets/runtime integration. They may be packaged together. Establish the stable generated arrangement using those real consumer needs instead of committing the converter to long-lived assets while the model and adapters are still being designed.

## Format order and ownership matrix

This maps the local catalogue to milestones. OpenSpace format codecs live in domain/container modules of the `openspace` adapter and read/write the useful types owned by `model`, regardless of crate packaging. Rete persists those same types; Godot implements their target behavior. Reader and writer responsibilities are paired as coverage permits. Content formats contribute meaningful field definitions, documented encoding declarations, and fixtures through the common useful-value interface. Container/compression primitives compose within those codecs; text formats use suitable parsers. This shared structure lets independent formats advance together while keeping semantic and reconstruction coverage explicit.

| Source format/group | Initial milestone | Structured result / completion target |
| --- | --- | --- |
| LZO streams, compressed envelopes, checksums | 1 | Working bounded decompression/compression and regenerated envelopes; exact historical compressed-byte equality at 13. |
| Texture CNT (`Textures.cnt`, `Vignette.cnt`), GF | 2 | Information-preserving texture/Rete assets and PNG for Godot; tested member/archive reconstruction without level parsing. |
| APM; BNM PCM/APM; BNM Ubi-MPEG/MPDX | 3, in that order | Information-preserving sound/Rete assets and semantic banks, with WAV for Godot; isolated Godot playback and separate encoded-fidelity checks. |
| CSB, LCB, SIF | 3 | Independent sound events/groups/material responses and sample references; scene integration at 6. |
| SNA block access | 4, reusing 1 | Transient level access and generated containers; pre-compression closure at 12. |
| RTB, RTP, RTT, `fixlvl.rtb` | 4 | Context-aware reference resolution; emission from written pointer fields. Complete parity at 12. |
| GPT, PTX, TextureInfo | 4-6 | Scene roots and named texture dependencies; generated sidecar pointers, not stored wire copies. |
| SNA geometry/material/physical-object records and buffers | 5 | Models, submeshes, materials, LODs, bounds and references; actual Godot meshes. |
| SNA scene/sector/light/radiosity/collision records; `*pgb.bin` | 6 | Scene, collision, lighting and sector content; compare target rendering/physics. |
| SND, SDA, RTS and scene-dependent audio bindings | 6 | Link early sound assets/events to scenes and persos; Godot spatial/event playback. |
| SNA family/state/ObjectList/perso-animation/transform records | 7 | Animated characters and level bindings; regenerated shared matrix pools. |
| SNA AI/DSG/brain/mind/scripts, dynamics, waypoint/action data | 8 | Executable typed behavior and instance state; supported operation inventory. |
| DLG, LNG, RTD, RTG, `fixlvl.rtg`; TRN where confirmed | 9, required subset at 8 | Localized strings and level overrides; language-aware generated references. |
| INI, MEM, CFG, VGM | 9 | Verified configuration/options/resource semantics; classify unused/empty content explicitly. |
| Indeo AVI and any required RTV/RTL structures | 11 | Late FFmpeg transcode and actual Godot cinematic playback; research only variants actually present. |
| `Savefile_EN.zip` | 0, test fixture only | Supplied post-intro starting state; verify save loading in the test installation. No save editor required. |
| Lowercase `World/Levels/fix.cnt`, installers, original EXEs/DLLs | Outside asset conversion | Input/runtime/research material. Do not treat as textures or copy into a distributable generated-content bundle. |
| Unknown source fields/regions discovered during any milestone | Owning milestone, closure at 12 | Explicit tracked unknowns; promote when understood. No unsupported whole-format pass-through counted as completion. |

Shared text-token parsing for CSB/LCB/SIF and later configuration files can be factored when actual syntax overlap is demonstrated. PNG/WAV and media decoding libraries serve compatible target/codec operations. They do not prescribe the canonical media representation when more information is required in Rete; see [media representation](docs/rete-design.md#media-representation).

## Test and measurement policy

- **Hard runtime budget: every automated test suite must complete in under 10 minutes of wall-clock time, preferably much less.** Measure the entire suite invocation, including setup/teardown and external tools or engine runs, on the documented development/CI environment. The limit also covers optional integration and whole-corpus suites; it is not a per-test allowance. Enforce an overall deadline that leaves time for child-process cleanup and a failure report before 600 seconds. Timeout means failure, not a passing partial run.
- Aim for seconds to a few minutes for focused developer checks. Use small representative fixtures for routine feedback and optimize broader suites to fit the same limit. Record total elapsed time and fix runtime regressions without silently dropping required assertions or splitting one suite into sequential stages to evade the budget.
- The shared format harness exercises declarative content codecs through the useful-model interface. Known-byte fixtures assert meaningful fields/references and generated bytes after deliberate edits. Persist and reload the same model values through Rete and verify their export behavior. Primitive header/stream tests are useful supporting checks; a raw record round-trip alone does not establish content support. Keep field documentation, implementation, and support reporting consistent as formats evolve.
- Unit tests use small synthetic/known records for exact arithmetic, encoding, malformed bounds, and reference rules. Asset integration tests use local game fixtures and actual Rete serialization. Do not repeatedly run the whole game corpus while changing one small format.
- Validate adapter boundaries with shared model fixtures: Rete and Godot operations must work independently of an OpenSpace import, and OpenSpace reconstruction must consume model values without invoking another target adapter. Test composed routes through application orchestration.
- Each useful feature completes the trip to Godot and receives structural, visual, or behavioral validation appropriate to it. Use actual frame/state comparisons where relevant; headless import success is only a resource validity check.
- Original-engine tests are opportunistic and must not become universal prerequisites. Use Raymap as the practical backup where it supports the feature; use engine/decompilation-informed fixtures and structural/behavioral expectations elsewhere. Record unavailable checks without claiming runtime parity. Pixel-perfect Godot images are a useful parity target, not an expected universal result. When feasible, engine tests use the supplied [Lutris reference, English save fixture, and harness guidance](reference/README.md#runtime-validation). Start with `Savefile_EN.zip` and record its loaded state; use additional saves/memory interventions only as needed. Keep runtime navigation interventions separate from tested asset behavior.
- Record CPU time, filesystem operations/time, peak memory, and serialization size separately for import, Rete save/load, OpenSpace export, Godot writing, and Godot engine resource import (distinct from optional Godot-to-model import). Do not optimize by bypassing the model or persistence path that a test is intended to exercise.
- Determinism tests use two clean outputs and the same source/profile/toolchain. Compare generated bytes, provisional paths, dependencies, node IDs and animation names for the current converter/schema version. During adapter development this checks repeatability without promising durable downstream compatibility. Account explicitly for Godot-generated import/UID behavior; do not exclude all target resources merely because one engine cache differs.
- Reports distinguish semantic support, opaque preservation, decoded-data fidelity, encoded-byte fidelity, rendering, and gameplay. Source data and game-derived outputs/fixtures remain local; the reusable test harness and synthetic fixtures can be versioned.

## First implementation task

Keep milestone 0 narrow, then restore **1: LZO -> 2: textures -> 3: audio** before **4: level access -> 5: the first textured model port**. Reuse the working C# format knowledge and evaluate existing codec crates against local fixtures. Indeo AVI conversion waits until milestone 11; exact historical LZO bytes wait until 13. The first Rust implementation follows that sequence. Further scene, animation, and behavior work starts after reviewing the initial slice and its remaining interface decisions.
