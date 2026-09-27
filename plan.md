# Astrolabe implementation plan

**Make a tool to repeatably port this OpenSpace game to Godot.**

This is the Rust plan, updated 2026-09-27 with the [current roadmap](./notes/remaining-work.md#current-roadmap--2026-09-27). The Rust implementation covers initial milestones 0–5, partial scene/character slices of 6–7 and bounded player/AI slices of 8; see [the implementation report](./notes/implementation-progress.md) for evidence, measured checks, and explicit remaining limitations. The milestone requirements below remain the eventual acceptance criteria; bounded first passes can be useful and demoable before a whole milestone exits. The former C# plan and completion claims remain in [the archived snapshot](./reference/astrolabe-csharp/plan.md); they are research evidence, not Rust progress. Read [the architecture](./docs/astrolabe/rust-architecture.md) and [Rete design](./docs/rete/rete-design.md) before implementation.

## Development rule: always ready to demo

See the whole structure before implementing the whole structure. Apply this cycle to the port, its adapters, the Godot support project, the comparison suite, and each substantial child group:

1. **Set up the framework.** Establish the group's real entry point, ownership boundaries, inputs, outputs, lifecycle, and one command that runs it. Name the child groups and show how they connect. Keep the framework usable with a synthetic fixture or a previously supported slice while real coverage grows.
2. **Put placeholders at the current tier.** Represent the known child groups in code and in the runnable status/reporting path. Each placeholder identifies its owner, dependency and completion check; attempting unsupported work reports that gap explicitly. Outline behavior families as groups. Do not create a stub for every AI call, state or scenario before the relevant family is opened.
3. **Fill the first details.** Replace one representative chain of placeholders with enough real implementation to exercise the parent framework. Use that chain rather than building a standalone path around it.
4. **Prove the stated scope and fix the design.** Run the same command, inspect its output and compare with independent source evidence where relevant. State exactly what the first pass demonstrates and which details remain provisional. Correct contracts exposed by the case before spreading them to siblings.
5. **Fill the remaining placeholders in that group.** Bring siblings to useful first passes using the corrected pattern, repeating the cycle inside complex children. Return later to improve fidelity where a claim, dependency or release gate requires it. The first successful child does not mark the parent complete.

At every checkpoint, the repository must still build, the applicable suite must run, and a short demonstration must show both working behavior and named gaps. Progress is bounded by code and observed output: a prose inventory alone does not establish a scaffold; a synthetic suite demonstration establishes infrastructure, not Hype parity; a pending or unsupported required case cannot be counted as a pass. Keep framework health, implemented coverage and accepted game behavior distinct in reports. Explicit diagnostics are valid placeholders; silent no-ops, fake assets and an OpenSpace runtime recreated in Godot are not.

An unsupported child must not erase its supported siblings. Import and persistence should retain the parent and its stable child identities, represent unfinished children explicitly at the appropriate tier, and let completed children reach their real target path. A target may execute or claim only the supported dependency closure; reaching an unsupported child must produce a visible failure or pending result. Preserve unresolved source information for investigation without treating an opaque placeholder as decoded behavior or claiming reconstruction parity for it.

**A first pass is sufficient when it proves the contract at the current tier.** Complete detail can follow at later gates. Apply this rule to every group and every kind of detail: supported cases, semantic depth, fidelity, timing, visuals, diagnostics, performance and developer workflow. Before implementing a child, name the smallest real result that will test its parent architecture and the evidence needed to recognize that result. Implement that result, then record the unfilled cases and observed inconsistencies with their effects and a concrete revisit trigger. Keep these states visible in runnable capability reports: placeholder, wired but unproven, demonstrated for a stated scope, and accepted to the required fidelity. Do not promote a scoped first pass to whole-group completion.

Decide what must be exact **for the current claim**. A shortcoming that hides whether the intended path works must be resolved now; one that leaves the path observable can be tracked while its siblings take shape. Revisit a gap when it blocks a dependent feature, makes an observation ambiguous, or reaches a later acceptance gate. Design each first pass behind the owning module's interfaces so that deeper behavior, new cases or a more precise implementation can replace it locally. The filtering/mip discrepancy and frame timing are examples of this rule, not special exemptions. For the next gameplay proof, actual compiled player AI choosing native movement is essential; visual and timing refinements can follow when they do not obscure that choice.

Advance to sibling work when the common demo command runs the real path, its evidence can distinguish success from failure, the report states the supported scope and gaps, and the remaining work can be improved within its owning module. If any of those conditions fails, repair the first pass or its parent design before spreading it.

Work broadly at the **current tier**, then go deep enough to prove its shared design. At the project tier, the major groups are conversion, native Godot output, validation/comparison, and corpus/fidelity. Within validation, the first tier is scenario definition/selection, engine backend contracts, observations, per-backend assertions, optional cross-backend comparison, evidence/reporting, and cleanup. OpenSpace and Godot are the first backends, not a limit on the contract. Walking, jumping, combat and magick are child behavior groups, not prerequisites for building that suite. Within a behavior group, individual cases are another tier. The milestone details below describe what completion means; they are not a reason to postpone the runnable framework until the first case works by itself.

## Priorities and completion

The [current roadmap](./notes/remaining-work.md#current-roadmap--2026-09-27)
organizes seven efforts: Orrery recording/replay and external behavior-suite
restructuring; Godot loading efficiency; first-level (`manoir`) game parity;
graphical parity; savegame research/Orrery support and Godot persistence;
Godot UI parity; and complete Rete coverage. These are scope and dependency
statements, not a fixed execution order. Loading efficiency and Orrery/BDD have
independent [prepared handoffs](./notes/remaining-work.md#current-roadmap--2026-09-27).
Expand the manor's existing playable path with the required save, menu, intro
and actor behavior as their efforts proceed. Graphics
and required model/Rete coverage advance with those slices. Complete tracing,
whole-game Rete coverage and full-game replay are not universal prerequisites.

Orrery remains an OpenSpace runtime analysis tool. Recording/replay extends
inspection and control; the behavior suite owns scenarios/assertions outside
Orrery and uses a separate Godot harness. Follow the
[recording implementation update](./notes/realtime-comparison-plan.md#session-recording-direction--2026-09-27)
and [session contract](./docs/astrolabe/session-recordings.md). Preserve existing
scenario evidence while correcting code ownership.

The current Godot "demo" is the game implementation, and its basic menu is the
actual menu. The fixed manor start is a provisional Load Game route; New Game
will begin with the intro. Loading-efficiency work can organize the shared
game-session lifecycle, savegame work replaces fixed restoration, and UI work
replaces the current presentation. Develop this one application, following
[the project ownership contract](./docs/astrolabe/godot-project.md#one-project-in-two-states).

September 24–25 evidence includes a saved player Brain, input-driven native
movement/contact, bounded jump, sword and Fire comparisons, and a playable
manor demonstration. Those are scoped results, not complete player combat,
first-level, visual or precise timing acceptance. The latest reported movement
selection finished 87/90; retain and resolve its failures rather than citing an
older pass as the current whole-project result. See
[implementation progress](./notes/implementation-progress.md) for exact scope
and the [September 13 native migration](./notes/native-godot-migration.md) for
its historical acceptance. Do not restart completed media/model/player slices
or resume the archived engine-runtime experiment.

**We MUST NOT reimplement the OpenSpace engine within Godot. Implement the AI
script API using native Godot facilities, and compile the game's AI scripts to
call that API.** The script decisions and observable API behavior are the
contract; OpenSpace's internal execution and storage machinery are not.

The unfinished runtime experiment is preserved at `9374ef5` on
`codex/archive-openspace-runtime-20260914`; production development restarted
from `d8d0375` on September 14 and has since advanced. Follow
[the concrete Godot gameplay plan](./notes/godot-gameplay-plan.md)
for milestone 8. Build on Hype's real AI driving native movement/contact in
`manoir`; complete jump, sword/crossbow and magick alongside the other first-level
requirements in the current roadmap. The archived attempt to implement the whole
OpenSpace runtime remains paused; broad scaffolding of the port and suite is
the active development rule above.

Godot is the primary deliverable. Work toward useful, repeatable conversion of Hype content and behavior, suitable for a modern fan-remaster and separately authored Godot extensions. OpenSpace reconstruction provides byte-level evidence and supports modifications of the original game. A full Godot-to-OpenSpace edit round trip is optional and deferred.

**Astrolabe outputs Godot project files, not data and a parser for Godot to handle at runtime.** Use Godot's scene tree and native resource/animation pipeline wherever they represent the source semantics. Convert each level into a root scene whose meaningful subtree structure follows OpenSpace, with independently useful/reusable content split into instanced scenes and shared resources. Supported clutter, persos, parts, and their initial placement/pose must exist when that scene opens in the editor. Export native animations that can be inspected and scrubbed there, and compile AI ASTs into GDScript. Actual gameplay may run scripts and spawn converted scenes; ordinary asset conversion and initial scene assembly happen before delivery. See [the Godot adapter contract](./docs/astrolabe/rust-architecture.md#godot-adapter-native-project-conversion).

**Architectural priority: keep OpenSpace, Rete, and Godot as separate modular I/O adapters around one useful intermediate model.** OpenSpace reads/reconstructs game data, Rete loads/saves the model, and Godot exports it into a modern engine; Godot import remains optional future work. Application code composes these adapters through shared model and resolution contracts. Adapter separation is required; the number of Cargo crates is an implementation choice. See [the architectural overview](./docs/astrolabe/rust-architecture.md#three-modular-io-targets-one-useful-model).

Minimal source metadata records facts such as identity/sharing and original order. Export derives encoding, alignment, sizes, placement, actual pointers, checksums, and relocation tables from current content and verified Hype rules. Do not add cached encoding decisions, offsets, or source-blob replay to make parity tests pass. Exact pre-compression package reconstruction is a required long-term target. The remaining historical LZO encoding difference is a separate final research gate.

**Develop needed converter-owned Godot assets alongside the exporter and OpenSpace/Model/Rete parity work.** Shaders, components and engine-side functions may be committed and maintained now, with clear ownership, native Godot semantics and tested interfaces. The earlier deferral was protection against an architectural mismatch; the native conversion direction now provides that boundary. Full package reconstruction is not a prerequisite. Godot rendering, editing and behavior are ongoing checks that the canonical content is understood and used semantically. Godot reverse import remains optional.

Most AI engine-side functions can wait until testing converted GDScript requires them; support needed by current slices, including shaders, can start now. Maintain one reusable Godot support package, and grow its interfaces from actual supported uses. Game/remaster projects own their authored game content and may build on those slices while broader conversion coverage develops. A runtime AST interpreter or JSON asset player remains outside the target architecture.

Design for a Godot developer in both the **dehydrated** committed project and the **rehydrated** project containing locally converted content. Keep support code, authored game work and generated assets recognizable; provide a source-free entry point, clear setup, normal scene/resource extension points and safe regeneration. Apply [the project structure and ownership design](./docs/astrolabe/godot-project.md) as the first native scene/character slice establishes concrete paths and package interfaces.

Before implementing a new domain, review asset boundaries, package/reference context, retained information, and all three adapters against representative cases. Record provisional choices and the evidence that would justify revising them. Do not let an early codec silently decide the architecture.

Every milestone delivers an observable result and records its limits. Rete schemas, OpenSpace readers/writers, Godot adapters, and tests grow together around useful assets. Full-package reconstruction must not block the first useful Godot result. Do not count unknown opaque data, an empty script, or an unanimated mesh as supported gameplay.

Restore **working LZO compression/decompression, textures, and audio first**, before the main OpenSpace level parser. The user reports complete working texture/audio import in C#, and preserving it is the reason for this priority. They are outside the level package; their simplicity must not determine the whole interface. Use C# source, local fixtures, and decoded results as regression references, preserving known working coverage while corroborating interpretation. The original engine/decompilation is the source of truth, then Raymap; Raymap is usually the first practical reference consulted. C# and historical notes remain useful but are not definitive. See [reference authority](./reference/README.md#reference-authority-and-research-workflow). Prefer suitable existing crates for standard codecs, with Hype-specific adapters and measured compatibility checks. See [library reuse](./docs/astrolabe/rust-architecture.md#library-reuse-and-early-delivery).

**One useful model, declarative codecs, Rete persistence.** Rete is the serialized form of the intermediate model. binrw declarations and reusable helpers document and implement how OpenSpace encodes its meaningful fields. The shared interface loads meaningful content from packages into memory and converts/saves it through the selected target, with common resolution, validation, diagnostics, and fixture tooling. A narrower OpenSpace `FormatCodec` pairs binary reads/writes of useful types. It does not impose OpenSpace parameters on Rete or Godot. Source reference resolution and semantic interpretation are part of content decoding. Temporary binary records support encoding where necessary; there is no required separate OpenSpace document model. Serde/MessagePack saves and reloads those same values. Apply [the declarative codec and unified interface contract](./docs/astrolabe/rust-architecture.md#binary-codecs-with-binrw) throughout the milestones below.

## Original-engine comparison control

**2026-09-27 recording direction:** Orrery remains the OpenSpace runtime analysis
tool, adding recording/replay alongside inspection and control. The behavior
test suite belongs outside Orrery and uses it, as originally intended; Godot has
a separate harness. Their shared
[session-recording contract](./docs/astrolabe/session-recordings.md) captures the
available save-file set before launch and follows startup, menus and gameplay
on one timeline. This includes recordings that select New Game and can grow to
full-game runs. The [implementation update](./notes/realtime-comparison-plan.md#session-recording-direction--2026-09-27)
corrects the earlier plan/code placement of Godot/BDD work inside Orrery and
extends ready-state-only recording scope; existing bounded probes remain valid
only for their stated coverage. The misplaced suite requires extraction, not a
narrowing of Orrery to a recorder. These are design decisions, not completed
capture capabilities.

**The Python `hype` MCP harness is still available, but the new suite must not depend on it.** Its earlier runs remain useful evidence and it can serve optional diagnosis. Preserve the private installation, saves and evidence. The active suite control path is the native Rust [Orrery controller](./docs/orrery/controller.md), which links Astrolabe without adding a dependency from the converter adapters. Orrery shares selected OpenSpace record readers between file and relocated-memory views; it does not yet import a whole live level. Its narrow original-game replay has succeeded. A live Orrery MCP session has now verified launch, supplied-save loading, physical input, typed normal/reflex AI selection, screenshot and cleanup. Selected designer values and a reusable suite backend remain open; see [live evidence](./notes/orrery-live-session-2026-09-23.md).

Build the suite framework with independent backend results and an original-game slot that reports pending until Orrery’s now-verified live controls and observations are wired through a reusable scenario and its assertions. Make a turn–release–idle scenario pass on OpenSpace alone using the demonstrated physical Left input, heading change and settled idle state; then establish the first movement scenario there. Add Godot to those scenarios as its saved AI path becomes usable. Keep Orrery's broader tool surface and precision replay as separately demonstrated capabilities. Read [Orrery's guide](./docs/orrery/controller.md) and the [runtime guide](./reference/README.md#runtime-validation); the Python [harness README](/home/stephen/Projects/hype-harness/README.md), [hidden-lever report](/home/stephen/Projects/hype-harness/HIDDEN_BUTTON.md), and [cheat catalog](/home/stephen/Projects/hype-harness/CHEATS.md) remain useful references. The supplied English save bypasses the intro and has reached `manoir` in earlier runs; verify the current Orrery session and state.

| Historical original-game evidence | Use in Astrolabe validation |
| --- | --- |
| Isolated keyboard input, timed action chains, state waits, and screenshots | Reproduce a short source-game sequence and compare its scene, animation, interaction, and timing with the generated Godot slice. Launch does not steal desktop focus. |
| State/mana reads, memory maps, and guarded memory writes with executable/process identity checks | Corroborate field meanings and state transitions. Resolve addresses for the active process; retain VM addresses and interventions only in transient reports, never Rete assets. |
| All 36 teleport destinations reached after hidden-lever activation; `tele11` reaches `astrolabe` | Reach representative levels without a complete playthrough. `navigate_level` provides a separate, logged memory transition when a tested cheat cannot reach the required state. |
| Mana consumption/refill and Fire Trinity steering/pause/resume (`hunter`, `geritole`) | Candidate behavior fixtures for milestone 8: input, designer-variable changes, mana cost, projectile lifetime, and collision outcomes. |

For milestones 6–8, select a bounded source-game comparison alongside each useful Godot slice: scene placement/materials/collision, an animation/action, or a supported interaction. Record executable/fixture hashes, level/save, input sequence, interventions, relevant values, screenshots/viewpoint, elapsed time, and expected Godot behavior. Reload the clean supplied fixture between unrelated behavior tests when cross-level state interferes. Historical timed MCP key actions were best effort. The [2026-09-20 timing experiment and replay decision](./notes/realtime-comparison-plan.md#replay-decision-frames-drive-inputs-milliseconds-judge-timing) and later Orrery replay establish narrow input feasibility; a shared scenario still needs backend integration and its own pass/fail evidence. A declared functional comparison may use controlled physical input with recorded timing and coarse outcome checks while the playable AI path is established. It cannot silently count as frame-exact replay or timing parity. Tighten scheduling and measured-millisecond assertions when timing affects the behavior under test or when making a timing-parity claim.

Keep **original-game baseline observations**, **Godot behavior comparisons**, and **original-engine acceptance of rebuilt files** as separate evidence. No Astrolabe-generated package has yet been accepted by the original game. When a suitable rebuilt file is available, stage it only in a test copy, identify/hash the actual files loaded, and demonstrate the intended edited effect. Do not silently exercise the original asset or repair exported data through memory writes. Comparing a useful Godot slice against the original game need not wait for complete SNA reconstruction.

Limits: historical teleport success establishes loading, not correct quest state or complete gameplay; flight/cinematic states can ignore input, and some destinations have camera/spawn quirks. Lever activation was tested with a temporary lever-position intervention, not a demonstrated natural walking route. The catalog contains 272 bindings, not 272 proven working cheats. Videos were disabled for fast tests, so those runs do not validate cinematics. Original-engine checks remain opportunistic rather than universal milestone prerequisites: use Orrery when its selected capability is verified, report a concrete unavailable check otherwise, and use Raymap/source-informed expectations as fallback. Automated engine comparisons share the overall suite deadline below, including setup and cleanup.

## Capability milestones and acceptance

The numbering records priorities, dependencies and exit checks. Apply the [scaffold-first cycle](#development-rule-always-ready-to-demo) at each tier instead of completing one narrow path while its parent framework is absent. Once the common format contract is established, independent formats can be documented and implemented alongside one another; completing every variant of an earlier format is not a prerequisite. LZO, textures, and standalone audio were the first real Rust asset details; integrate their useful Godot slices as they become ready.

### 0. Establish interfaces and a repeatable harness

Establish the shared `model`, separate `openspace`/`rete`/`godot` adapters, and thin application/CLI orchestration described in the architecture. Choose a minimal Cargo package layout that preserves those module boundaries; a five-crate workspace is not required. Define small reference/root-context interfaces and diagnostics without scaffolding all domain types before the first asset. Pin dependencies, including binrw for OpenSpace codecs, and record the supported Godot version. Establish the package-content load/convert/save contract across the three adapters, with explicit supported directions and common diagnostics. Keep a typed OpenSpace binary codec contract and binrw conventions inside that adapter. Test the proposed interfaces with a small synthetic level-style graph containing shared/forward references, distinct equal-valued assets, Fix bindings to one active level, and an edit that changes generated sizes/pointers. This small architectural probe uses synthetic data; real level formats remain in their later milestones. Refine it against real texture/audio codecs in milestones 2 and 3; milestone 1 supplies stream/envelope primitives. Grow field meanings and encoding declarations together. Use disposable Godot fixtures and provisional naming; preserve unrelated/authored files on regeneration without promising a stable downstream API.

Serialize that same synthetic model asset through Serde/MessagePack with named fields, references, exact numeric values, and embedded dense arrays/byte buffers. Use MessagePack binary fields for raw byte payloads; there is no separate generic binary-content file or `.bin` sidecar. Verify save/load, useful inspection, and format version rejection. Exercise Rete and Godot APIs with the constructed model asset independently of any OpenSpace import; application code owns the composed conversion route. Measure per-asset serialization and file I/O without constructing a global pool or micro-file forest.

Use [Orrery](#original-engine-comparison-control) and the supplied `reference/Savefile_EN.zip` for new original-game checks using its verified live capabilities. The available Python harness's earlier save loading, keyboard, screenshot and guarded-memory evidence remains a useful reference. Preserve the save and test installation, record edition/build and fixture identity, and use additional saves or logged interventions only when needed. On a workstation without this local setup, use the supplied Lutris installer reference and timebox setup. Verify Raymap's web/local-level workflow before relying on generated-file viewing. Consult Raymap first for practical format research and resolve conflicting interpretations against the original engine/decompilation. Record unavailable comparisons without making runtime availability a universal prerequisite.

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

### 6. Native level scene, collision, lighting, and sound bindings

Expand GPT/root traversal into useful scene nodes, instances, sectors, IPO/physical-object relationships, collision meshes, `*pgb.bin` where applicable, lights, fog, and material/radiosity behavior. Export one root Godot level scene, mostly following meaningful source parent/child structure and transforms. Persist supported prop/clutter and perso subscenes with the appropriate node ownership and resource references. Milestone 7 supplies perso parts and initial poses; until then, diagnose incomplete instances rather than treating empty scripted nodes as completed scene content. Preserve source semantics while implementing the Godot scene/collision/shader mapping.

Build deterministic scene and node paths with stable material/model references, sharing reusable geometry while retaining independent placement and lighting. Split scenes/resources by ownership and reuse. The normal level port should expose this level root directly; the first-model selection remains an explicit development mode. Implement a small inspection/camera wrapper that instances the exported level without constructing its missing contents. Use Orrery to capture a matching level/viewpoint and selected collision observations. The historical harness reached `astrolabe` through `tele11` after unlocking cheats; verify that setup through Orrery before depending on it in a scenario. Compare with Godot and Raymap/source-informed expectations; record the reference used, interventions, and known differences. Pixel equality is a useful measurement, not a required Godot outcome. Separate prelit colors from dynamic lighting approximations.

Integrate the phase-3 sound assets and event definitions with SND and RTS relocations. Investigate SDA through its actual consumers: the verified Hype level file contains static globals, including a text handle, rather than an ambient-sound table (see [source/MCP evidence](./notes/scene-animation-research.md#sda-classification-correction-from-source-and-mcp)). Resolve scene/perso sound references, spatial settings, ambient emitters, and event routing into native audio resources/player nodes and converted Godot behavior where needed. Replace runtime sound-program JSON parsing as part of that projection. Test loop behavior and selected ambient/action events using documented source expectations and available references, adding original-engine comparison when feasible; defer AI-triggered behavior completion to phase 8. Generate supported sound-sidecar pointers from semantic links.

**Exit:** the saved level scene opens in the Godot editor with supported static contents already present, tested transforms, model sharing, material assignment, collision nodes, and audio bindings. With the milestone-7 slice, supported clutter/persos and initial poses are also present without executing a loader or preview. Tests inspect persisted descendants and references before gameplay, then exercise rendering, selected collision behavior, and scene-bound audio through the same assets. Initially hidden and unsupported contents are distinguished explicitly. Screenshots and geometry assertions identify known visual differences. Supported scene records reconstruct under the local export layout.

### 7. Families, perso instances, and animation

Implement `Family`, states/transitions, `ObjectList` model configurations, `Perso3dData`, Montreal animation frames/channels/hierarchies, compact transforms, speed matrices, and mesh-switch/hide behavior. Preserve Fix-owned templates and level-specific configurations without freezing a shared family to the first imported level.

Associate transforms with useful animation content and retain minimal original pool indices for reconstruction. Derive compact encoding and alignment from values/profile rules; test matrices representing each observed form. Preserve shared entries and distinct entries with equal values. Regenerate matrix pools from scoped original order, not semantic traversal.

Export reusable perso/configuration scenes with persisted part nodes and a source-informed initial pose. Produce native Animation and AnimationLibrary resources attached to AnimationPlayer nodes; use AnimationTree when its state/blending behavior fits. Resolve transform pools and sampled channel data at conversion time. Assess derived transform tracks on stable target nodes for changing parentage, pre-instanced part alternatives with discrete visibility/property tracks for object switches, and native audio/method tracks for cues. Validate source timing and interpolation instead of assuming a conventional static skeleton or smooth interpolation. A difficult source feature requires a documented mapping or an explicit gap; narrow engine-support code is allowed where native tracks cannot represent its behavior, but a general runtime frame-table player is not the target.

Replace the current animation/configuration/family/perso JSON loading path for the supported slice and remove superseded production players as the native path lands. Keep the source-faithful canonical animation and reconstruction metadata in the model/Rete. Native/custom Godot resources must expose useful target properties and references, not merely wrap the same interpreted tables in a new extension. Capture a corresponding short idle/movement/action sequence with Orrery where applicable, recording timing and state with the declared observation limits. Add and maintain required Godot support assets with this slice, using the documented support-package boundaries.

**Exit:** a character and its initial parts/pose are visible in the saved scene, its native clips appear in Godot's animation editor, and representative idle/movement/action sequences can be scrubbed and played through AnimationPlayer. Check forward/backward seeking across parent/object switches, reset/initial poses, loop boundaries, cues, state speed, and shared instances. Compare transforms, visible parts, and timing against Raymap/source-informed expectations, supplemented by original-engine runs when available. No custom asset parser or sampled-pose interpreter is needed for this slice. Rete retains the same animation data; local matrix/animation reconstruction is byte-exact for the covered cases. Regenerating does not rename referenced states or nodes.

### 8. AI, interactions, and a playable behavior slice

The torch and bounded player slices provide the starting point. Continue with
the controls and explicit exit checks in
[Hype gameplay in Godot](./notes/godot-gameplay-plan.md), extending them to the
intro, actor interactions and independent actor behavior/physics required by
the current manor-parity roadmap. Add engine services as those real converted
script paths require them; do not broaden the service catalogue speculatively.

Extend shared AIModel/Behavior/Macro types, designer-variable definitions and instance values, typed script trees, action bindings and semantic actor/resource links as that path requires. Preserve meaningful Brain/Mind source metadata for reconstruction without exporting their memory/execution organization as a Godot framework. Add physics/collision/sector and waypoint semantics when the selected gameplay uses them. Pull the small localization subset required for the chosen behavior from phase 9 when needed.

Choose one real interaction/character behavior and compile its typed AI AST into executable GDScript in the Godot adapter. Translate expressions, branches, loops, macros, state changes, and instance values into code; resolve content references into Godot nodes/resources. Preserve tick timing, blocking/meta actions, state changes, random-seed handling, and persistent instance state through generated control flow and appropriate Godot callbacks. Implement the needed engine-side functions in the maintained Godot support package using native facilities, with explicit supported semantics. Test the generated behavior and those same functions in the rehydrated project and focused fixtures. Do not ship an AST/opcode interpreter or generic operation dispatcher, count S-expression output as execution support, or silently stub unimplemented actions.

The [API assessment](./notes/ai-api-assessment.md) distinguishes behavior requests,
queries, game services and procedural property changes. Compile ordinary values,
arrays and references to ordinary GDScript properties and node/resource links.
Implement the API with InputMap, CharacterBody3D, native animation, PackedScenes
and focused Hype game services. Preserve observable ordering and completion with
generated game control flow; do not recreate generic source instruction cursors,
variable-access APIs or return-flag machinery. Required initialization and
referenced actors must work; unrelated controller coverage and whole-package
parity must not displace the playable checkpoint.

Use the previously measured hidden-lever or Fire Trinity sequence as a candidate source baseline once the slice supports its dependencies and Orrery can reproduce it. The lever has a two-second activation window and a distance condition of less than 1.5 units; the projectile test records a 35-mana cost, steering, pause/resume, and active-ball count changes. Reproduce the relevant observation and compare the exported Godot behavior, distinguishing input recognition, state changes, and visible effects. These are historical source-game fixtures, not already implemented Astrolabe AI support.

**Exit:** generated `.gd` files pass Godot parsing and run a reproducible behavior sequence through the exported scenes/resources and tested engine-side functions, with state/event traces checked against engine/decompilation-informed expectations and available comparison runs. Report gaps where no runtime reference could be exercised. An intentional AST/parameter edit changes regenerated GDScript and the expected behavior. Script serialization/reconstruction tests preserve supported values, reference targets, and tree structure. Unsupported operations are listed with affected assets. No runtime source-AST loading or interpretation is required.

### 9. Localization, remaining sidecars, menus, and configuration

Complete DLG/LNG and RTD/RTG, language-specific Fix/level bindings, text tables and TRN where confirmed, and all remaining SND/SDA/GPT/PTX fields. Implement required INI/MEM/CFG/VGM semantics for port configuration, options, menus, and credits. Classify each from actual source use; historical catalogue names are not a substitute for verification.

CFG game options are distinct from savegames. Catalogue source save structures
and extend Orrery's save analysis/fixture support; implement Godot persistence
and its save/load UI as required by first-level progression. Evaluate a source
save converter with explicit field/behavior coverage, without making Orrery a
runtime dependency of the port. A general original-save editor is not required.
Bring the intro needed for first-level parity forward from phase 11; unrelated
cinematics remain later.

**Exit:** selected language/dialogue and menu/option paths work in Godot with their dependencies. Source-file tests cover supported sidecars and remaining undecoded fields are visible. The converter handles explicitly selected locale deterministically.

### 10. Expand adapter coverage across the game corpus

Grow each domain's supported cases across levels, shared families, script operations, materials, sounds, and languages. Track read/write/export coverage and the structural/behavioral evidence for each. Use the maintained Godot support package, real authored consumers of supported slices, and development harnesses to expose gaps. Grow and version dependency contracts with demonstrated use; keep unknown behavior and broader compatibility claims explicit.

Run single-level, subset, and whole-game conversion as sequential level selections in different orders. Verify shared references, session binding isolation, collision detection, repeatable naming for the current schema, safe stale-output handling, and preservation of unrelated/authored files. Perform a clean rebuild with Godot import caches removed. Open representative level roots and native animation libraries in the editor, then exercise generated scripts and resources without a converter-specific runtime asset parser or preview bootstrap. Audit emitted gameplay dependencies to prevent JSON/table interpreters returning under new resource extensions. Keep game-derived output local.

**Exit:** the supported corpus exercises the three adapters through native Godot projects with an explicit coverage report, including pending OpenSpace writers/fidelity work and cinematics. Dehydrated/rehydrated workflows and authored extensions survive the supported regeneration/update paths. This is converter integration evidence; full-game parity and compatibility outside the tested interfaces remain separate claims. Continue developing supported consumers alongside remaining adapter coverage.

### 11. Late media work: Indeo AVI cutscenes

The 2026-09-27 roadmap brings the intro needed by the new-game/manor route
forward into first-level parity; this milestone retains the remaining
cinematics. Distinguish prerecorded video from scripted in-engine sequences
and validate the complete handoff into gameplay.

The user identifies the original AVIs as Indeo-encoded. Inventory the real streams with `ffprobe`, then evaluate `ffmpeg` transcoding to a format supported by the pinned Godot build. Built-in [Ogg Theora playback](https://docs.godotengine.org/en/stable/classes/class_videostreamtheora.html) makes `.ogv` a candidate; confirm quality, audio, and playback with a real sample before fixing the profile. See the [FFmpeg transcoding documentation](https://ffmpeg.org/ffmpeg.html).

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

## Incremental Godot support and consumer contracts

The all-adapter gate for converter-owned assets and downstream dependencies is removed. Add reusable support when a concrete slice requires it, including shaders before AI work and engine functions alongside AST-to-GDScript tests. Design and test the package's ownership, native interfaces, versioning and developer workflow as part of that slice. Speculative whole-engine implementation can wait for evidence of need.

Game-parity, full-working-game and fan-remaster projects own their authored game assets and may consume supported conversion slices now, even when packaged with Astrolabe. Establish dependable generated paths and extension points through real authored use, safe regeneration and explicit changes to evolving interfaces. Validate the committed project as well as the version containing generated files; see [the project design](./docs/astrolabe/godot-project.md).

Keep coverage and readiness claims proportional to demonstrated OpenSpace import/export, Rete persistence and native Godot behavior. A useful material/character slice does not establish whole-level or full-game parity. Complete the remaining reconstruction and fidelity milestones independently; exact historical compression and optional Godot reverse import do not gate needed support assets.

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
| SNA scene/sector/light/radiosity/collision records; `*pgb.bin` | 6 | Editor-visible level root and instanced subscenes, native collision/lighting resources; compare target rendering/physics. |
| SND, RTS and scene-dependent audio bindings | 6 | Link early sound assets/events to scenes and persos; Godot spatial/event playback. |
| SDA static globals | 6, 9 | Interpret through scene/UI consumers; Astrolabe's 12-byte SDA is not a sound-emitter table. |
| SNA family/state/ObjectList/perso-animation/transform records | 7 | Persisted perso parts/initial poses, native Animation/AnimationLibrary and AnimationPlayer playback, level bindings; regenerated shared matrix pools. |
| SNA AI/DSG/brain/mind/scripts, dynamics, waypoint/action data | 8 | Typed AST compiled to GDScript, Godot engine-function implementations for the tested slice and instance state; supported operation inventory. |
| DLG, LNG, RTD, RTG, `fixlvl.rtg`; TRN where confirmed | 9, required subset at 8 | Localized strings and level overrides; language-aware generated references. |
| INI, MEM, CFG, VGM | 9 | Verified configuration/options/resource semantics; classify unused/empty content explicitly. |
| Indeo AVI and any required RTV/RTL structures | Intro for first-level parity; remaining media at 11 | FFmpeg transcode where needed, actual Godot playback and gameplay handoff; research only variants actually present. |
| Source save files and supplied `Savefile_EN.zip` | Recording fixtures and first-level persistence/UI | Preserve available files at launch; catalogue structure, extend Orrery analysis and implement Godot save/load with an optional converter. Keep raw recording fixtures separate from useful content/Rete assets. No general save editor required. |
| Lowercase `World/Levels/fix.cnt`, installers, original EXEs/DLLs | Outside asset conversion | Input/runtime/research material. Do not treat as textures or copy into a distributable generated-content bundle. |
| Unknown source fields/regions discovered during any milestone | Owning milestone, closure at 12 | Explicit tracked unknowns; promote when understood. No unsupported whole-format pass-through counted as completion. |

Shared text-token parsing for CSB/LCB/SIF and later configuration files can be factored when actual syntax overlap is demonstrated. PNG/WAV and media decoding libraries serve compatible target/codec operations. They do not prescribe the canonical media representation when more information is required in Rete; see [media representation](./docs/rete/rete-design.md#media-representation).

## Test and measurement policy

- **Bound individual tests and active development-area validation; the full release matrix may take longer.** The universal ten-minute ceiling is removed. Select affected tests/dependencies plus mandatory safeguards, enforce per-test/scenario and explicit suite deadlines with cleanup reserved, and report selection and end-to-end duration. Timeout remains failure. See the [detailed policy](./notes/realtime-comparison-plan.md#test-selection-and-timing-policy).
- Aim for seconds to a few minutes for focused developer checks. Classify tests by area, prerequisites and cost; preserve all assertions and release membership when splitting monolithic tests. Record build/setup/test/cleanup time, selected/excluded counts and missing capabilities. Measure Rust code coverage and level-data coverage separately. Do not silently drop checks or let unsupported/pending behavior count as passing coverage.
- The shared format harness exercises declarative content codecs through the useful-model interface. Known-byte fixtures assert meaningful fields/references and generated bytes after deliberate edits. Persist and reload the same model values through Rete and verify their export behavior. Primitive header/stream tests are useful supporting checks; a raw record round-trip alone does not establish content support. Keep field documentation, implementation, and support reporting consistent as formats evolve.
- Unit tests use small synthetic/known records for exact arithmetic, encoding, malformed bounds, and reference rules. Asset integration tests use local game fixtures and actual Rete serialization. Do not repeatedly run the whole game corpus while changing one small format.
- Validate adapter boundaries with shared model fixtures: Rete and Godot operations must work independently of an OpenSpace import, and OpenSpace reconstruction must consume model values without invoking another target adapter. Test composed routes through application orchestration.
- Each useful feature completes the trip to Godot and receives structural, visual, or behavioral validation appropriate to it. Use actual frame/state comparisons where relevant; headless import success is only a resource validity check.
- Validate saved scene contents before gameplay/preview execution: level hierarchy, clutter/perso descendants, source-informed initial pose/visibility, resource sharing, and resolved native animation track paths. Inspect and scrub native clips in the editor, including backward seeks and discrete switches; test cue side effects separately during playback. Check generated GDScript parsing and behavior after deliberate AST edits. Runtime construction from JSON or renamed custom-resource tables cannot satisfy these acceptance checks; diagnostics and test-report JSON are permitted.
- Exercise the committed/dehydrated project with synthetic support fixtures and the rehydrated project with converted content using the same maintained shaders/components. Test an authored scene/resource variation, preserve it and project settings across regeneration, and check support-version compatibility and clean Godot cache rebuilds. Classify these checks into affected-area and release selections with explicit per-test and selected-suite budgets.
- Use [Orrery](#original-engine-comparison-control) for relevant original-game observations and generated-output comparisons through its verified live control path. The available Python harness may assist diagnosis but is not a suite dependency. Engine tests remain opportunistic, not universal prerequisites; report the specific unavailable check and use Raymap/source-informed expectations when necessary. Pixel-perfect Godot images are a useful parity target, not an expected universal result. Start with the supplied English save and record the actual loaded state; use additional saves/memory interventions only as needed. Keep navigation interventions, baseline observations, Godot parity, and acceptance of rebuilt OpenSpace files distinct.
- Record CPU time, filesystem operations/time, peak memory, and serialization size separately for import, Rete save/load, OpenSpace export, Godot writing, and Godot engine resource import (distinct from optional Godot-to-model import). Do not optimize by bypassing the model or persistence path that a test is intended to exercise.
- Determinism tests use two clean outputs and the same source/profile/toolchain. Compare generated bytes, provisional paths, dependencies, node IDs and animation names for the current converter/schema version. During adapter development this checks repeatability without promising durable downstream compatibility. Account explicitly for Godot-generated import/UID behavior; do not exclude all target resources merely because one engine cache differs.
- Reports distinguish semantic support, opaque preservation, decoded-data fidelity, encoded-byte fidelity, rendering, and gameplay. Source data and game-derived outputs/fixtures remain local; the reusable test harness and synthetic fixtures can be versioned.

## Next implementation task

Follow the [current roadmap](./notes/remaining-work.md#current-roadmap--2026-09-27)
and [recording sequence](./notes/realtime-comparison-plan.md#session-recording-direction--2026-09-27):
extract the external behavior suite and separate Godot harness, preserve the
existing assertions/evidence, and establish a complete short original session
with immutable prelaunch saves, actual inputs and synchronized output/state.
Develop precise timing/frame association and AI/API trace coverage with explicit
acceptance cases. Broader coverage remains visible while the first recording
and replay are useful.

Godot loading work can proceed independently from the measured resource-load
bottleneck. Establish rendered cold/warm menu-to-play measurements and evaluate
native resource representation/reuse and readiness waits. Continue manor
gameplay, intro, actors, saves/UI and graphics through the external suite as
their dependencies become available. Complete Rete coverage remains a broader
track; exact historical LZO fidelity and unrelated cinematics retain their late
milestones. Preserve native Godot editing and the
[project ownership contract](./docs/astrolabe/godot-project.md).
