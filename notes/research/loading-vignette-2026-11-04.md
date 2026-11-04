# Level loading screen (Vignette module + `*pgb.bin`)

Static research, 2026-11-04. Nothing was run. Sources: `reference/decompilation/functions/`
(Ghidra C, `X:\cpa\tempgrp\VIG\Vignette.c` module), `objdump -d` of
`reference/hype_patch/MaiDFXvr_bleu.exe` where Ghidra output was lossy (addresses below are VAs),
and `disc/Gamedata/World/Levels/*/*pgb.bin`. Tags: **[P]** proven from code/data, **[I]** inferred.

## 1. Which image, and where it is encoded

- **[P] Per-level binary "progress bar" file, not the DSC.** Level init `00402b60` (lines ~40-70) builds
  `<level>.pgb` (text script, `SCR_fnp_st_RdL0_AnalyseSection` path) and falls back to
  `<level>pgb.bin` (format string `%s%s.bin` @005e430c); `FUN_0050ef30` loads the `.bin`.
  Only the `.bin` files exist on disc (e.g. `World/Levels/manoir/manoirpgb.bin`). Startup does the same
  for `GameData\Game.pgb` / `Gamepgb.bin` (`00407140`).
- **[P] The text form is the same data**: the exporter `0050f370` writes `LoadLevelVignette(name)`
  (or `LoadVignette` for mode 1), `SetBitmapBarMode(1,"bar.bmp")`, `SetBitmapBarPosition(bgY,fgY,minX,maxX)`,
  `InitBarOutlineColor/InsideColor/Color`, `CreateBar(x1,y1,x2,y2)`, `MaxValueBar(n)`, `AddBar`,
  `DisplayVignette`. Section parser callback = `LAB_0050e890` (registered by `0050d440` as section
  "Vignette"); no DSC text for it exists on disc (`grep` of disc finds none), so the bins are the source.
- **[P] `.bin` layout** (`0050ef30`, little endian): `i32 n1`, `n1` bytes image name with
  `plain = stored + 3*i - 7`; `i32 n2` (0 = default drawn bar), `n2` bytes bar name with
  `plain = stored + 6*i - 11`; then `i32` bgY, fgY, minX, maxX; 24 float32 (outline RGBA, inside RGBA,
  bar 4x RGBA corner colours); `i32` x1,y1,x2,y2 (640x480 space); `i32` MaxValueBar. Parsed with a throwaway
  script; no trailing bytes in any file.
- **[P] Name -> file**: `0050e310` replaces the extension of the stored name (`.bmp`) with a
  global suffix (`PTR_DAT_005ecdbc`, taken to be `.gf` [I]) and looks it up in `Vignette.cnt`.
  Image width/height are scaled to `640x480` basis (`/0x280`, `/0x1e0`).
- **[P] Mapping decoded from the bins** (image / max steps):

| level | image | MaxValueBar |
|---|---|---|
| manoir | epoque1 | 50 |
| brigand | epoque1 | 54 |
| toras | epoque1 | 57 |
| astrolabe | epoque2 | 46 |
| brigand2/3/4 | epoque2/3/4 | 49/53/46 |
| manoir2/3/4 | epoque2/3/4 | 51/49/48 |
| toras2/3/4 | epoque2/3/4 | 55/52/55 |
| cachots, cachots4 | epoque1, 4 | 50, 48 |
| casino, casino3 | epoque2, 3 | 48, 42 |
| ciel1..4 | epoque1..4 | 43 each |
| cite3, cite4 | epoque3, 4 | 46, 46 |
| fort, fort3, fort4 | epoque1, 3, 4 | 49, 54, 50 |
| labo, labo3 | epoque2, 3 | 54, 52 |
| monast1/2/3 | epoque1/2/3 | 50/47/53 |
| toptower, tour, tour3 | epoque4, 4, 3 | 46, 53, 53 |
| village2/3/4 | epoque2/3/4 | 45/46/46 |

  (All 38 levels with a bin; others on disc have none and were not checked further.) Rule of thumb: the
  digit suffix is the era (no suffix = era 1, except astrolabe/casino/labo = 2 and tour = 4).
- **[P] Startup** `Gamepgb.bin`: image `Ubisoft.bmp` (-> `Ubisoft.gf`, not verified present in the CNT
  because CNT names are obfuscated), no bitmap bar: default drawn bar rect (20,370)-(620,390), max 39, inside
  colour white-ish, bar colours blue/white ramps.

## 2. Bar geometry (640x480 basis)

All values identical in every level bin: bgY=0, fgY=60, minX=50, maxX=461, CreateBar =
(110,410)-(621,469), `Bar.gf` 512x120.
- **[P]** Bitmap mode subtracts minX from x1/x2 on load (`0050ef30`): frame rect x1'=60, x2'=571,
  y 410..469 inclusive => **frame destination (60,410) size 512x60**, source = `Bar.gf` rows 0..59 (bgY).
  Composited into the backdrop pixel buffer once at load by `0050dc70` (loop x1'..x2', y1..y2 inclusive).
- **[P]** Fill source = `Bar.gf` rows 60..119 (fgY), blitted straight to the screen by `0050d6b0` -> `00478f30`
  (args decoded from asm: dest x = x1'+minX+prev-1, dest y = y1, src x = minX+prev-1, src y = fgY,
  w = 2*(cur-prev)+2, h = y2-y1 = 59, colour key 1). So fill track on screen is x 110..521 (411 px =
  maxX-minX), bar-local x 50..461.
- **[P]** Non-640x480 windows: `0050dc70`/`0050f370` rescale by W/640, H/480 (`ceil`); original is native 640x480
  rendering (software/Glide LFB). Cross-check **[P]**: `orig_9505.png`/`orig_19346.png` (1024x768) show the
  bar from ~x 100..915, y ~657..750 = 62..572 x 410..469 at 640x480 scale (x1.6). Matches.

## 3. Fill rule

- **[P] Step driven**: `FUN_0050d500(1)` is called once per loaded chunk (sections in `0040a340` x9,
  `0040ac10` x23, `0043cea0` x4, `004e8ad0`, `00427b40`, `00429150`, `0042a8b0`, `00453bc0`, `004582e0`,
  `0049aef0`). `cur = min((maxX-minX)*n/Max, maxX-minX)` (integer), with `n` the running step count
  (`DAT_008dc070`) and `Max` = MaxValueBar (`DAT_008dc06c`). No time-based progress. Whether n really reaches
  Max at the end (the Max values are tuned counts, 42..57) cannot be proven statically; clamped if it exceeds.
- **[P] Grows left to right by cropping the filled-state source**: each step blits only the new column band
  [prev-1 .. prev-1+w) from the fill rows. **Quirk [P from asm, effect I]**: width is 2*(cur-prev)+2, so each
  step over-draws about (cur-prev)+2 px (~10 px) ahead of true progress; later steps redraw it. Equivalent
  Godot: crop fill region to x <= 50 + cur + ~dprev+2 (or just 50+cur; the difference is ~2% of the track).
  At 100% the over-draw reaches past maxX, so the right end cap of the filled art appears.
- **[P] Colour key**: during the fill blit, RGB565 value 0 (pure black) source pixels are skipped. For the
  frame, `0050dc70` sends key-0 pixels through `0050d870` with the inside colour (0,0,0,**a=1.0**);
  `0050d870` blends `existing*a + colour*(1-a)` **[I, from asm]**, so a=1.0 leaves the backdrop untouched. Capture
  confirms: the black corners of `Bar.gf` show the sky (transparent). Port: treat pure black pixels of `Bar.gf` as
  transparent, both for frame and fill. (Some levels carry non-zero alpha/colour inside values, e.g. astrolabe,
  casino, cachots4, cite3, labo, tour; those only matter for key-0 pixels and were not decoded further [I].)
- Redraw: `FUN_0050d5f0` (from window repaint `0040113c`) sets a flag; next step `0050d610` redraws backdrop (3x)
  and the whole fill up to the current value.

## 4. Fade, min time, text

- **[P]** The Vignette module (`0050d1e0`-`0050f370`, `0050e310`) contains no fade, timer, Sleep, or font call.
  Backdrop is drawn (3x, for buffer flips) by `0050d460`/`0050ef30` then only fill columns are added.
  Not found anywhere in the viewed functions: minimum display time. **[I]** none exists.
  Window captions per load stage (`0043c8e0`) are separate from the frame art.

## 5. Other vignettes

- **[P]** `Gamepgb.bin` startup Ubisoft vignette (above), same module/file format.
- **[P]** `random.vig` (`Gamedata\Vignette\random.vig`, section `randomVignette`, `0050d1e0`/`0050e230`):
  named-set picks a random vignette image; file absent on disc, so unused in this build.
- **[P]** AI statement `DisplayVignetteAndReturnToMainMenu` (game-over etc.; already bound in
  `src/openspace/content.rs:382`) goes through the same loader `0050e820`, but is not a loading screen.
- `map/epoque1..4` (`00416560`, `00417070`) are the inventory map textures, unrelated to loading.
