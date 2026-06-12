# References

This directory contains reference material used while building Astrolabe. A clean checkout only includes this README, `reference/.gitignore`, and the tracked Git submodule entries. Large downloads, proprietary/game-derived files, and generated decompilation output are intentionally ignored.

After cloning, initialize the tracked references with:

```bash
git submodule update --init --recursive
```

## Tracked References

These paths are expected to exist once submodules are initialized.

| Path | Source | Use |
| --- | --- | --- |
| `raymap/` | <https://github.com/byvar/raymap> | Unity map viewer/editor for OpenSpace games, including Hype: The Time Quest. Useful for comparing parser behavior, game/version constants, and data model assumptions. |
| `OpenSpaceToolbox/` | <https://github.com/raytools/OpenSpaceToolbox> | Toolbox for OpenSpace PC games. Useful for level/runtime behavior, supported game conventions, and reverse-engineering cross-checks. |
| `Rayman2Lib/` | <https://github.com/szymski/Rayman2Lib> | Rayman 2 reverse-engineering library and tools. Useful as an older independent reference for OpenSpace/Rayman 2 data structures. |
| `ray2get/` | <https://github.com/Synthesis/ray2get> | Rayman 2 APM audio conversion notes and tooling. Useful for audio format behavior shared with the OpenSpace family. |
| `OpenRayman/` | <https://github.com/imaginaryPineapple/OpenRayman> | Open source Rayman 2 engine reimplementation. Useful for runtime behavior and file-format decoding patterns. |
| `vgmstream/` | <https://github.com/vgmstream/vgmstream> | Video game audio decoder library. Useful as a broad audio codec/container reference. |

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
