# Material transparency and pixel comparison

The Godot projection now distinguishes opaque textures, color-key cutouts,
alpha blending and additive effects. Previously every triangle material used
Godot's opaque mode. A palette's selected transparent entry also became opaque
RGB in the PNG, even when its material enabled the color key. This caused
Hype's black eye mask, the black wall behind the opening ladder and the brown
star ceiling reported during review.

## Source evidence and boundaries

Raymap's `TextureInfo` and `VisualMaterial` were consulted first. The Hype
Glide renderer `FUN_00499ed0` is decisive for this build: texture flags at +8
enable chromakey (`0x2`), alpha (`0x8`) and additive ONE+ONE (`0x20`). Its
material context gates texturing with bit 0 and shaded blending with `0x200`
and `0x20000000`. These are not Raymap's later-engine material flag positions.
`TEX_vSetTextureChromakey` (00482da0), the GF loader (00481310) and palette
upload (0049a2e0) connect the selected GF palette index to the color key.

Canonical texture samples, palette identity, GF key values and material flags
remain unchanged. Godot derives a separate PNG only when an enabled palette/RGB
key needs it; opaque uses keep the original PNG. Duplicate palette entries with
the same RGB stay independently meaningful. ARGB1555 and ARGB4444 already carry
alpha. Binary cutouts use alpha scissor with depth writing; alpha and additive
materials use their respective blending modes. Disabling shaded blending retains
the original alpha test (eight-bit alpha greater than 7). Sprites retain the material's
mode instead of forcing every sprite to use alpha scissor. No generic rule
removes black pixels.

The corpus includes 101 palette/other `0x2` uses, 15 `0x12` uses, 29 `0x8`
uses, one `0x1a` star material and 75 additive uses among 317 visual materials.
The 114 distinct textures used with chromakey comprise 98 RGB palettes and 16
component images; every selected palette key is in range.
The star texture `astrolabe/etoiletxynz.gf` has ARGB4444 coverage; the window
texture `astrolabe/2fentxynz.gf` has ARGB1555 coverage. The former contains 5,530
transparent and 2,662 opaque base-level samples. The complete engine render
context, mip selection, chrome, scrolling and runtime material changes remain
provisional; the exporter diagnoses that limit.

## Original-engine comparison

Hype MCP run `20260912-205121` used the established executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da` and English
save SHA-256 `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The save loaded into `manoir`; logged level navigation selected `astrolabe`.
The opening baseline and camera readings preceded any other intervention.
Its screenshot is in the external harness run as
`screen-1789261376042223412.png`; the read-only camera and actor observations
are in ignored `output/transparency-original-camera.json`.

The active viewport camera was resolved through 00475040/004853c0/00485300.
Inverting its view matrix gives Godot position
`(-22.94999589,-1.5000002,4.03999958)` and forward
`(-0.155865983,-0.195799770,0.968177783)`. Horizontal FoV is 1.5 radians;
vertical FoV is 1.219702363 radians (**69.88379766 degrees**). Both captures
use 1024×768 with near plane 0.25. Hype uses state 47/frame 0 in Godot and the
previously observed hidden channel IDs 28,71,80,136. Idle phase is not synchronized.

```sh
godot --path output/transparency-compare --audio-driver Dummy \
  --resolution 1024x768 -- \
  --character Hype --state 47 --frame 0 --hide-channels 28,71,80,136 \
  --camera -22.94999589,-1.5000002,4.03999958 \
  --look-at -23.105861873,-1.695799970,5.008177363 \
  --fov 69.88379766 --near 0.25 --no-overlay \
  --capture /absolute/path/opening.png
```

`tools/compare_images.py` uses ImageMagick 7 to produce pixel difference images,
reference/candidate/difference strips, hashes and normalized RMSE/MAE. It
requires equal image dimensions and does not silently resize or align them.

```sh
python3 tools/compare_images.py /absolute/path/original.png \
  /absolute/path/opening.png output/pixel-review \
  --region eyes:487,365,48,30 --region ladder-wall:416,343,146,109 \
  --region scene:0,0,1024,640
```

| Region | Before RMSE | After RMSE | Reduction |
| --- | ---: | ---: | ---: |
| Hype's eyes | 0.133101 | 0.0705411 | 47.0% |
| Ladder and wall | 0.153780 | 0.0529392 | 65.6% |
| Scene above HUD | 0.0373197 | 0.0296056 | 20.7% |
| Entire frame | 0.0496416 | 0.0451006 | 9.1% |

These measurements compare the same source screenshot with the same-camera
before/after Godot captures. Residual differences include filtering, lighting,
idle phase and the absent original HUD; they are not pixel-parity acceptance.
See ignored `output/pixels-before/report.json` and `output/pixels-after/report.json`.

The ceiling was also rendered before/after from interior Godot camera
`(-10,-5,4)` looking at `(-10,3,4.01)` with the same FoV. The brown fill disappears
and white stars become visible between the beams (`output/stars-before.png`,
`output/stars-after.png`, `output/pixels-stars/`). That comparison is between
two Godot renders, not an original-engine ceiling capture. The original-engine
walk reached the main-room doorway before the session lifetime expired. An
earlier paused local-transform positioning attempt was overwritten by the game
and was abandoned; the unchanged save was reloaded before the later walk.
No reconstructed asset was repaired in process memory.

## Regression checks

The synthetic palette test checks source row orientation, selected-entry identity
and an opaque duplicate black entry. The native Godot regression edits the key
through Rete, verifies separate opaque/keyed PNGs and sharing between keyed
materials, checks source-gated alpha/additive modes, and confirms canonical
texture values remain unchanged. Both traversal orders reject a derived PNG
filename that collides with another authored texture. The full local suite retains all GF byte
reconstruction checks and scene/Rete/Godot routes. Generated files and source
screenshots stay local.

Final full-suite evidence: **76 tests in 238.725 seconds**, including compilation
and cleanup, in `TestResults/20260913T012705.474229Z-2036182.log.json`. Formatting,
strict Clippy and diff whitespace checks passed. The comparison utility's
identical-image check produces zero RMSE and MAE for both full-frame and cropped
regions.

## Bilinear filtering follow-up

The source Glide initialization at 0049ba75 and 0049d4e2 calls
`grTexFilterMode(tmu, 1, 1)` for every texture unit. The Glide header defines
1 as `GR_TEXTUREFILTER_BILINEAR`; mode 2 passed separately to
`grTexMipMapMode` is `GR_MIPMAP_NEAREST_DITHER`
([upstream Glide header](https://sources.debian.org/src/glide/2002.04.10ds1-25/glide3x/h5/glide3/src/glide.h)).
Read-only Hype MCP code bytes at 0049ba63 corroborated the executable
disassembly. Godot materials now select linear filtering. The native material
test verifies that opaque, keyed, alpha, additive and source-disabled-blend
materials all retain that setting. Exact source mip selection remains open;
the Godot output still uses the base PNG rather than the original mip chain.

The follow-up baseline is Hype MCP run `20260912-213818`, screenshot
`screen-1789264195341146775.png`, with the same executable/save hashes above.
The fresh viewport read is in `output/fog-camera-evidence.json`. Both Godot
captures use its 69.88379766284831-degree vertical FoV and 1024×768 dimensions.
`output/fog-after.png` is the nearest-filter baseline;
`output/filter-after.png` adds only bilinear filtering.

| Region | Nearest RMSE | Bilinear RMSE | Reduction |
| --- | ---: | ---: | ---: |
| Hype's eyes | 0.0691787 | 0.0671771 | 2.89% |
| Ladder and wall | 0.0514060 | 0.0484031 | 5.84% |
| Scene above HUD | 0.0297184 | 0.0285306 | 4.00% |
| Entire frame | 0.0451630 | 0.0445032 | 1.46% |

The reports are `output/fog-pixels-after/report.json` and
`output/filter-pixels-after/report.json`. These are incremental improvements
after the transparency fix, measured against one unchanged original screenshot.
Lighting, unsynchronized idle phase and the missing HUD still contribute to
the differences. This view does not establish an original-engine star-ceiling
comparison; the ceiling check above retains that limitation.

The stopping-point regeneration is `output/scene-graph-compare`. Its matched
capture `output/scene-graph-final.png` has the same SHA-256 as
`output/filter-after.png`; all regional metrics are unchanged after the scene
record/bounds additions. The final report is
`output/scene-graph-final-pixels/report.json`.
