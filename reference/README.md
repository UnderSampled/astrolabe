# References

This directory contains reference material used while building Astrolabe. Repository references include this README, `reference/.gitignore`, the supplied Lutris installer JSON, and the Git submodule entries. Large downloads, proprietary/game-derived files, and generated decompilation output are intentionally ignored.

The Astrolabe snapshots and edited serializer commits were created locally and have not been pushed. They are available in this checkout, including each archived implementation's exact `lib/` dependencies. A fresh clone from the hosted repositories cannot fetch unpublished commits until they are published to repositories that contain them and the relevant submodule URLs are updated as needed. No push was performed as part of this archive operation.

The submodule gitlinks pin the exact snapshots. The `gltf` branch setting records its provenance; ordinary `git submodule update` uses the pinned commit. Avoid `--remote` when restoring these historical snapshots.

After the pinned commits are available from their configured remotes, initialize the tracked references with:

```bash
git submodule update --init --recursive
```

## Tracked References

These paths are expected to exist once submodules are initialized.

| Path | Source | Use |
| --- | --- | --- |
| `astrolabe-csharp/` | This repository, `b1fba2ec614ffa79983fa0f54755b35f3de6269b` | Complete C# source snapshot before the Rust restart, including previously uncommitted source and notes. Exact edited dependencies are initialized under its `lib/`. |
| `astrolabe-gltf/` | This repository's local `gltf`, `aa688f478ea967df15cc72b5ed8191f0cea494ef` | Historical glTF implementation. This preserves the local branch tip, which differs from `origin/gltf`. |
| `BinarySerializer/` | <https://github.com/BinarySerializer/BinarySerializer>, local snapshot `bdec9d1f79c50803b672e9e8192427e76b2a9254` | Former C# dependency, moved from `lib/` and preserved with its local edits. Research only. |
| `BinarySerializer.OpenSpace/` | <https://github.com/BinarySerializer/BinarySerializer.OpenSpace>, local snapshot `ab4b9cc869e1ebe94e2a456e1dac6749ca2d24f3` | Former C# dependency, moved from `lib/` and preserved with its local edits. Research only. |
| `raymap/` | <https://github.com/byvar/raymap> | Unity map viewer/editor for OpenSpace games, including Hype: The Time Quest. Useful for comparing parser behavior, game/version constants, and data model assumptions. |
| `OpenSpaceToolbox/` | <https://github.com/raytools/OpenSpaceToolbox> | Toolbox for OpenSpace PC games. Useful for level/runtime behavior, supported game conventions, and reverse-engineering cross-checks. |
| `Rayman2Lib/` | <https://github.com/szymski/Rayman2Lib> | Rayman 2 reverse-engineering library and tools. Useful as an older independent reference for OpenSpace/Rayman 2 data structures. |
| `ray2get/` | <https://github.com/Synthesis/ray2get> | Rayman 2 APM audio conversion notes and tooling. Useful for audio format behavior shared with the OpenSpace family. |
| `OpenRayman/` | <https://github.com/imaginaryPineapple/OpenRayman> | Open source Rayman 2 engine reimplementation. Useful for runtime behavior and file-format decoding patterns. |
| `vgmstream/` | <https://github.com/vgmstream/vgmstream> | Video game audio decoder library. Useful as a broad audio codec/container reference. |

Generated C# `bin/`, `obj/`, test results (including hang dumps), and Python caches from the pre-rewrite working tree were moved to matching paths inside `astrolabe-csharp/`. They remain local and ignored; the commits preserve source, not generated artifacts. Nested community references inside the two Astrolabe snapshots remain uninitialized; their shared top-level copies are available alongside these archives.

## Reference authority and research workflow

Use this order of authority when interpreting Hype data and behavior:

1. The original OpenSpace engine and its decompilation are the source of truth. Match evidence to the relevant executable/build; investigate decompiler ambiguities against the executable when needed.
2. Raymap is the next authority and **usually the first reference to consult in practice**, before investigating the engine directly. Its parser, domain types, and web viewer are useful starting points; resolve disagreements against the original engine/decompilation.
3. The archived C# implementations and historical research notes remain valuable evidence and regression baselines. Preserve and consult them, but corroborate their interpretation rather than treating old code, layout notes, or completion claims as definitive.

Record source/build, code location or observed fixture, uncertainty, and any conflicting interpretation alongside format declarations and tests. A C# import comparison protects working behavior; it does not independently prove that every source field was understood or retained. Other community references can help resolve individual formats without changing this hierarchy.

Authority does not imply a required runtime harness: original-engine tests are opportunistic and already carry setup/coverage risk. Use Raymap as the practical backup within its capabilities, then source-informed structural/behavioral expectations and fixture checks. Keep unperformed runtime comparisons explicit. Godot pixel equality is a useful parity measurement, not an expected or universally required result.

## Runtime validation

2026-09-20 direction: preserve this working external installation/service while
developing the [native Rust live loader and comparison testkit](../docs/realtime-comparison-plan.md).
Reusable replacement tooling may be maintained alongside the converter and link
Astrolabe directly. The current Python controller remains the migration reference;
authentication material, game installations, saves and game-derived evidence stay
local. Retire the old controller only after native acceptance checks pass.

**For automated comparisons, use the verified standalone Hype MCP harness at `/home/stephen/Projects/hype-harness`.** It is separate from Astrolabe and has a private game copy at `local/prefix/drive_c/Games/Hype`. The persistent `hype-mcp.service` is registered as `hype` in the local Codex MCP configuration. Its 15 tools cover lifecycle, keyboard/cheat input, timed sequences and state waits, screenshots, state/mana reads, memory maps, guarded memory writes, and logged level navigation. Read the [harness README](/home/stephen/Projects/hype-harness/README.md) for tool arguments, save-loading recipe, service management, and authenticated client/CLI fallback when an existing Codex session has not refreshed tool discovery. Preserve the current external controller during the planned migration; do not copy its private files into the repository.

Start with `status` and `start_game`, then load the supplied save using the documented `run_sequence` recipe; wait for phase 9, level `manoir`, and fade-in completion. Save loading and gameplay are verified. All 36 hidden-lever teleport destinations loaded; `tele11` reaches `astrolabe`. The [hidden-lever report](/home/stephen/Projects/hype-harness/HIDDEN_BUTTON.md) documents the required unlock, temporary positioning intervention, Fire Trinity steering/pause tests, and input/camera/quest-state limitations. `navigate_level` is a memory intervention, not a cheat or proof of story progression. The [272-code catalog](/home/stephen/Projects/hype-harness/CHEATS.md) labels unverified bindings. Do not reuse captured heap addresses without resolving and validating the current process.

Use the harness for original-game baselines and comparisons with Godot as described in [the plan](../plan.md#available-original-engine-comparison-harness). No rebuilt Astrolabe package has yet been tested successfully in it. For generated-file acceptance, stage a suitable rebuilt asset only in the test copy, record the loaded file hashes and expected edit, and restore the test installation afterward. Keep navigation interventions separate from asset correctness. The current fast-test configuration disables AVI videos; it does not validate cinematics. Preserve saves and enforce the suite's overall deadline including engine cleanup. The earlier Lutris installation/setup reference follows; it need not be repeated to use the working harness.

The user supplied [`hype-the-time-quest-lutris.json`](hype-the-time-quest-lutris.json) for setting up **Hype: The Time Quest** under Lutris/Wine. The file is a Lutris API response: `results[0].script` contains the installer object and `results[0].content` contains its YAML form. Treat it as an installer reference, not as a shell script or a confirmed local installation. Inspect the actual installed prefix and runner before constructing launch commands.

The supplied configuration describes:

- A 32-bit Wine prefix at Lutris's `$GAMEDIR`.
- Game executable `drive_c/Games/Hype/MaiDFXvr_bleu.exe`, with working directory `$GAMEDIR/drive_c/Games/Hype`.
- User-provided disc data, the Zeus Hype patch, and nGlide; the installer also installs `mfc42` and `icodecs`, selects language data, configures nGlide's Vulkan backend, and disables DXVK.
- Creation of `$GAMEDIR/drive_c/Games/Hype/GameData/SaveGame`. This installer step does not establish the active save search path or where the supplied `SaveGame0/` belongs; verify those in the test installation.

**The intro is very slow. Use the supplied English save first.** The user added [`Savefile_EN.zip`](Savefile_EN.zip) as a local post-intro fixture. Archive inspection on 2026-09-12 found 16,912 bytes and a valid ZIP CRC check, with these members:

```text
SaveGame0/
  General.sav       (8 bytes)
  General000.sav    (46 bytes)
  Level000.sav      (13,420 bytes)
  manoir.sav        (2,847 bytes)
```

SHA-256: `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.

For runtime setup, preserve the archive and existing saves, confirm the installed English edition and actual save root, then install a copy retaining `SaveGame0/` and load it in the test installation. Record the executable/build, archive hash, selected save, and observed scene/state. Do not infer the reached level from the filenames. The 2026-09-12 Lutris setup copied this archive into the English installation's `GameData/SaveGame/`, matching `gamedsc.bin`. Gameplay was visually observed, and the user confirmed reaching it through the supplied save with working video/audio. Keep the game-derived fixture local.

Use another carefully chosen save or a targeted process-memory harness only when a test needs an additional state; playing the intro is not a prerequisite. Record interventions and expected behavior so another agent can reproduce the test. Memory interventions should navigate to the test, not repair exported content and hide defects. Test generated assets in a separate installation/copy. The current local installation is `/home/stephen/Games/hype-the-time-quest-win32`, pinned in Lutris to Wine-GE 8-26. It includes Hype-local video runtime dependencies and NVIDIA settings; use its saved Lutris entry when reproducing the successful desktop launch. Logs, hashes, and screenshots are under ignored `output/runtime-validation/lutris-20260912/`. The supplied save remains unchanged. A disposable Godot renderer/audio harness is implemented; see [runtime evidence](../docs/implementation-progress.md#runtime-reference-status).

**Raymap** provides another parser and visual reference. Its [web version](https://raym.app/) has been useful to the user, who expects local OpenSpace level loading may also be possible. An implementation-time probe opened the local WebGL shell but both the local-data and documented Hype route failed with `GAMEDATABIN folder doesn't exist`. Its FileSystem has a hardcoded remote data-server prefix; local symlinks alone did not route data loading locally. Verify support for loading the particular generated files before relying on it as an export test. The ignored `raymap-webgl/` snapshot described below is a possible local workflow. The user does not use the Unity Editor: use the web viewer or existing source references without making a Unity installation necessary. Raymap is the practical backup when original-engine tests cannot run. Use engine/decompilation-informed expectations for gameplay beyond viewer coverage and report the remaining verification gaps; do not make an engine run a prerequisite for every feature.

## Early format regression references

The user reports working LZO support and complete working texture/audio import in C#. Preserve those capabilities first in Rust; this priority protects existing functionality, not a choice to design all interfaces around media outside the level package. Use archived source and local fixtures as regression references, corroborating meaning through the hierarchy above:

- LZO and checksums: [`OpenSpaceLzo.cs`](astrolabe-csharp/src/Astrolabe.Core/Rete/OpenSpace/OpenSpaceLzo.cs), [`OpenSpaceChecksum.cs`](astrolabe-csharp/src/Astrolabe.Core/Rete/OpenSpace/OpenSpaceChecksum.cs), and the archived LZO 1.08 compressor integration. Working compression/decompression is an early milestone; the remaining difference from original compressed bytes is late research.
- Textures: [`CntReader.cs`](astrolabe-csharp/src/Astrolabe.Core/FileFormats/CntReader.cs) and [`GfReader.cs`](astrolabe-csharp/src/Astrolabe.Core/FileFormats/GfReader.cs). Compare decoded pixels, palette/alpha behavior, member order, and reconstruction.
- Audio: [`FileFormats/Audio/`](astrolabe-csharp/src/Astrolabe.Core/FileFormats/Audio/) contains BNM/APM readers, PCM/APM/Ubi-MPEG decoders, and WAV writing. Cross-check sample/channel/timing behavior with these and `vgmstream/`. The Ubisoft MPEG adaptation is needed in addition to any standard decoder.

The user identifies the game AVIs as Indeo-encoded. `disc/Gamedata/Videos/` contains `LOGO.AVI`, `HypeIntro.avi`, and `HypeConclusion.avi`; probing, FFmpeg conversion, and Godot playback checks are deferred to the late video milestone in [the plan](../plan.md).

## Ignored Local References

These paths are ignored by `reference/.gitignore` and will not exist in a clean checkout. Create them locally when needed.

| Path | How to obtain | Use |
| --- | --- | --- |
| `hype_patch/` | Download <https://www.zeus-software.com/files/nglide/hype_patch.zip> and extract it into `reference/hype_patch/`. | Zeus nGlide patch package for Hype. It contains `MaiDFXvr_bleu.exe` and `maidfxvr_bleu.sdb`. The patched EXE is useful for decompilation because it is a runnable/unwrapped counterpart to the protected Glide payload rather than the small SafeDisc CD-check loader. |
| `decompilation/` | Generate locally with Ghidra from `reference/hype_patch/MaiDFXvr_bleu.exe`. | Ghidra project, logs, function index, all-functions C-like output, and per-function output. |
| `raymap-webgl/` | Download a local snapshot of <https://raym.app/maps/> into `reference/raymap-webgl/`. | Raymap's Unity WebGL build for browser automation comparisons. Include `Build/raymap.loader.js`, `Build/raymap.data.unityweb`, `Build/raymap.framework.js.unityweb`, `Build/raymap.wasm.unityweb`, the page shell, shared CSS/JS assets, `json/content.json`, and `json/raymap/playmobil_hype/pc.json`. Serve this directory with a local static server before using Playwright, for example `python3 -m http.server 8899 --directory reference/raymap-webgl`. To test against local Hype disc data, create local symlinks such as `ln -s ../../disc/Gamedata reference/raymap-webgl/Gamedata` and `ln -s ../../disc/LangData reference/raymap-webgl/LangData`; these symlinks live inside the ignored snapshot and are not tracked. |

## Hype Decompilation Notes

The nGlide patch `.sdb` file is a Windows application compatibility database, not a debug-symbol database. Its useful context is:

- Target files: `MaiDFXvr_bleu.exe`, `MaiDFXvr_bleu.icd`
- Compatibility fixes: `EmulateDirectDrawSync`, `VirtualRegistry`

The patched EXE has useful reverse-engineering metadata:

- CodeView record: `NB10`
- Referenced PDB: `X:\CPA\EXE\MAIN\MaiDFXvr_bleu.pdb`
- PDB signature: `9c930b38`
- PDB age: `4`
- Other embedded debug records: `Misc`, `FPO`
- PE export table: about 2,740 named exports, which Ghidra can use for engine function names.

The matching PDB is not in the patch archive or this repository. Without it, Ghidra cannot load full debug symbols, but the export table still provides many function names.

Previous local Ghidra analysis of `hype_patch/MaiDFXvr_bleu.exe` found 6,663 functions. Ghidra decompiled 6,662 directly; `005012b0` caused the native decompiler process to die and was reconstructed manually from the listing by comparing it to the nearby allocator wrapper at `00409840`.

## CD Executable Layout

If you have the Hype CD files extracted under the repository `disc/` directory, the executable layout is SafeDisc-style:

- `disc/exe/Glide 3x/MAIDFXVR_BLEU.EXE` and `disc/exe/D3D/MAID3DVR_BLEU.EXE` are small loader/CD-check executables, about 280 KB each. They contain SafeDisc/CD-ROM/debugger-detection strings, have empty PE debug directories, and are not the actual engine payloads.
- `disc/exe/Glide 3x/MAIDFXVR_BLEU.ICD` is the larger Glide payload. It is a PE executable and carries the same `NB10` PDB reference to `X:\CPA\EXE\MAIN\MaiDFXvr_bleu.pdb`.
- `disc/exe/D3D/MAID3DVR_BLEU.ICD` is the larger D3D payload, but it has an empty PE debug directory.
- `reference/hype_patch/MaiDFXvr_bleu.exe` is not byte-identical to the CD Glide `.ICD`, but it has the same four-section PE shape and the same PDB reference, with SafeDisc/CD-check concerns removed for practical analysis.
