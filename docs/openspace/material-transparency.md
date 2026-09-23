# Material transparency and pixel comparison

Material flags determine whether a texture is opaque, color-keyed, alpha-blended,
or additive. Godot must apply the source material mode when projecting it; every
triangle cannot be treated as opaque. A palette's selected transparent entry also became opaque
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

## Comparison evidence

Matched-camera comparisons show that applying the source material modes improves
Hype's black eye mask, ladder wall, and star ceiling. The exact capture setup,
regional pixel measurements, and image reports are in [the dated transparency evidence](../../notes/material-transparency-evidence.md).

## Regression checks

The synthetic palette test checks source row orientation, selected-entry identity
and an opaque duplicate black entry. The native Godot regression edits the key
through Rete, verifies separate opaque/keyed PNGs and sharing between keyed
materials, checks source-gated alpha/additive modes, and confirms canonical
texture values remain unchanged. Both traversal orders reject a derived PNG
filename that collides with another authored texture. The full local suite retains all GF byte
reconstruction checks and scene/Rete/Godot routes. Generated files and source
screenshots stay local.

Dated suite and image-tool checks are in [the evidence note](../../notes/material-transparency-evidence.md).

## Bilinear filtering

The source Glide initialization calls `grTexFilterMode(tmu, 1, 1)` for every
texture unit; mode 1 is bilinear filtering. The separate mode 2 argument to
`grTexMipMapMode` selects nearest mipmap choice and must not be confused with
texture filtering. Godot's linear filtering is the corresponding texture
projection. Exact source mip selection remains distinct from filtering and is
not established by a base PNG. The original calls and measured image differences
are recorded in [the bilinear comparison note](../../notes/material-transparency-evidence.md#bilinear-filtering-follow-up).
