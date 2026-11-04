# Crossbow reticle chain (static research, 2026-11-04)

Report: Manoir, no visible crossbow reticle; Stephen believes it projects on the aimed objects in world
space. Static only; nothing was run.

## Verdict

The reticle is the original engine's native **laser pointer** (AI procs 337-343, funcs 211-213), not a
Perso, Object, particle or AI-spawned sprite. The port compiles every call and tracks the state, but
`laser_pointer.gd` is a state-only Resource: **nothing in the port draws it**. First divergence: link 6
below. No stand-in documents it (stand-ins.md has no reticle/laser entry), so it is also an undocumented
omission.

## What the original does

Frames: `TestResults/stephen-linear-brigand-20261102/recording/frames/NNNNNN.png` (frame idx = events
elapsed seconds minus 5). events.jsonl: Look key is `v` (keysym 118, code 55; Stephen rebinding of the
Numpad0 default), aim windows 352.2-354.7 s (fire Space at 353.9) and 380.4-384.4 s. Z (122) selects the
crossbow.
- 000352: red ring-with-cross, about 10 px, on the far cliff wall just right of screen centre (520,388) on
  a distant surface. Frame 000353: same sprite about 25 px at (541,396) on the foliage/hive, much closer.
  Frame 000382: about 6 px, at (519,386) on a far cliff. Frame 000354 (after release): gone.
- Size scales with hit distance and the sprite sits on the surface the ray hits (frame 353 it overlaps
  foliage rather than floating at a fixed screen size). This supports world-space (quad at the hit
  point), not a fixed-size HUD crosshair. Whether it is oriented to the surface normal or camera-facing
  is NOT determined from frames (small sprite, no oblique view). Occlusion by geometry between camera
  and hit is natural (hit is the first surface), but depth-test vs overlay is unverified.
- It is not at exact screen centre (offset by up to ~30 px at 353), consistent with the aim ray coming
  from Hype/crossbow direction rather than the screen centre. The port's `update_target` uses the
  screen-centre ray (see link 4).

Texture: gen_viseur textures exist in the game data: `FixTex/gen_viseur/viseur1_alpha`, `viseur2_alpha`,
`viseur3_alpha`, `laserpointer`, `laserpointer_old`, `laserpointer_old2` (32x32 / 16x16 PNGs
converted at `output/rust-textures-godot/generated/hype/textures/FixTex/gen_viseur/`). viseur1_alpha is a
red cross inside a ring, matching the frames; laserpointer.gf is the same on a black/white disc. The
engine default texture name is the string `ViseurInitial` (.data VA 0x5e7f10).

Engine code (MaiDFXvr_bleu.exe, objdump):
- 0x4216a0 Activate = `word[0x74b324]=1`; 0x4216b0 Hide = 0. 0x4216c0 SetTexture(name): texture lookup
  (0x486400) -> material 0x7508b4 -> object 0x7508b8. 0x421870 SniperMode, 0x4218a0 FlyMode (scale
  `[0x74b334]` 1.0 or 3.0 = 0x40400000). Default init at 0x41ec7e: scale 2.5 (0x40200000) in `0x74b328`,
  max distance `0x74b330` = 100, detail `0x74b31e` = 2, active 0. Setters live in dispatch fn 00543920.
- 0x4218e0 is the per-frame draw, called from 0x4df406 (render pass). It returns at once when
  `word[0x74b324]==0` (0x42190f, 0x421afd). Otherwise it takes the camera (0x4bae60/0x4bb390), runs a
  ray query `0x420b90` with a miss default of -50000 on all axes (0xc7c35000), and sets `byte[0x6107f0]` =
  hit-found. Then, if detail == 2 and the beam flag `[0x6107ec]` is 0 and the hit distance is under
  `[0x74b330]` (100): `0x41fa00` + `0x420a90`; else `0x420eb0`, which builds vertices in the
  `0x74b3ac` dynamic mesh (distance scaling, `[0x74b334]` size), and the pass ends by drawing object
  `0x7508b8` with `0x47ccf0` (the shared world object draw) with a 0xfbfbffff flag mask. So it is
  geometry submitted into the world pass at the ray hit, i.e. world-space, consistent with the frames.
  The exact quad orientation (normal vs facing) and the 0x420eb0/0x41fa00 split were not decoded
  (detail-level semantics unverified; needs a decomp pass of 0x420a90/0x420eb0).
- Funcs 211/212/213 (GetLaserPointerDirection/Distance/DistanceWithMap) read the same hit data; the
  bolt (fix/733 line 98-101) aims along `Func_GetLaserPointerDirection`, so the reticle and the bolt
  trajectory share one target.
- Visibility is purely the AI-controlled active flag. Fix model 28 (Hype) sets it
  (`godot/generated/hype/levels/manoir/ai/models/fix/28.gd`: true at 1344, 2146, 2214, 2399, 2476, 3030,
  ...; false at 657, 2249, 2429, 2542, 3133...). It sets sniper mode at 1893-1894 and fly mode (0) at
  1387. Hype's AI never calls SetTexture/Distance/DetailLevel in Manoir (grep of all ai/models: no
  uses), so the defaults above apply. Other users of Activate in generated fix models: 768, 908, 828,
  882, 842, 893, 782, 797, 922, 757, 817, 864 (other weapon/aim-type Persos; 688 hides).

## Chain and port status

| # | Link | Status | Evidence |
|---|------|--------|----------|
| 1 | Input: look key held with crossbow selected switches to aim camera/inputs | IMPLEMENTED | `godot/game/crossbow_input.gd:50-60` (selected_weapon == 80), `game_session.gd:1022-1030` |
| 2 | Hype AI sets Activate/Hide, sniper, fly | IMPLEMENTED (compiled) | `ai.rs:3702-3738`; calls in 28.gd above; sniper/fly return true when the Resource exists |
| 3 | Native state | IMPLEMENTED | `godot/addons/astrolabe/character/laser_pointer.gd` (active, sniper_mode, fly_mode, scale, target) ; `character.gd:1124-1146,1215-1222` |
| 4 | Target acquisition | STAND-IN (undocumented): screen-centre physics ray, mask 1, 2000 m, excludes actor, miss = far point | `crossbow_input.gd:73-82`, `game_session.gd:1062-1074`. Original ray comes from the engine camera/pointer query (0x420b90) with 100 m range; not source-backed (README says "until a source-backed ray"). Non-aim fallback `position + z*30` is also undocumented (`game_session.gd:1033`) |
| 5 | Texture asset | MISSING in worktree generated output | `godot/generated/hype/textures/FixTex` has no `gen_viseur`; no consumer references it. Present in `output/rust-textures-godot/...` so the converter can produce it |
| 6 | **Draw the pointer** | **MISSING (first divergence)** | `active` and `target_*` are written but never read by any node: grep of godot/game, godot/addons shows no reader of `source_laser_pointer.active`; no `Sprite3D`/`MeshInstance3D`/`Decal` for it. No fail_unsupported is hit (the setters return true), so the omission is silent |
| 7 | Procs 339/340/341 (Distance, Texture, DetailLevel) | not reachable in Manoir (never emitted by 28.gd); parser maps only 337/338/342/343 (`src/openspace/ai.rs:578-581`) | would be MISSING parser coverage if another level uses them |
| 8 | Surface orientation / occlusion / range | UNKNOWN | see original notes |

Renderer: `godot/project.godot:35-36` `rendering_method="gl_compatibility"` (also mobile). Godot Decals
need Forward+ or Mobile (not supported in Compatibility), so a Decal is not usable without a renderer
change; use a native quad (Sprite3D/MeshInstance3D).

## Recommended fix

Native representation: a small authored presentation node (e.g. `character/laser_pointer_view.gd`
supporting asset, owned by astrolabe support like laser_pointer.gd) that observes the shared
`laser_pointer.gd` Resource each frame: visible iff `active and target_valid`; a billboarded or
normal-aligned `Sprite3D` with the converted `FixTex/gen_viseur` texture (default; viseur1_alpha is the
visual match; confirm which texture "ViseurInitial" resolves to), placed at `target_position` offset by
a small epsilon along the hit normal, size from the source scale (`2.5` units x `source_mode_scale`,
base unit unverified), `no_depth_test` false so geometry occludes, unshaded, alpha-blended. Keep the AI
untouched (it already drives `active`). Prefer extending the existing ray in
`crossbow_input.gd`/`game_session.gd` to also supply the hit normal (the two copies of the target code
duplicate each other; consolidate, since only one should run). Also emit the gen_viseur textures
(texture conversion) into the generated project. Document the screen-centre target ray as a stand-in
until the source ray origin (0x420b90) is decoded.

## New parser/decomp assumptions needing a live check (Orrery)

- Draw gating word at `0x74b324` is the only visibility gate (read `inspect_combat` laser visibility
  while aiming vs not).
- Detail level 2 / beam flag `0x6107ec` selects the dot path (0x420eb0) rather than the beam path; in
  the recording only a dot is visible, so confirm at runtime the path and the texture it binds.
- World-space depth behaviour (occluded by foreground geometry) and surface alignment: capture one
  frame aimed at a wall obliquely.

## Red-capable targeted test

Godot check (extend `tests/check_laser_pointer_fly_mode.gd` style, owned log path): instantiate Hype
(or a bare Character), call `source_set_laser_pointer_target(Vector3(3,1,4))` and
`source_set_laser_pointer_active(true)`, then assert the scene contains a visible node under the
character tree/level whose global position is within 0.1 of the target, and that after
`source_set_laser_pointer_active(false)` no node is visible while `target_position` is preserved. Red
today (no such node exists). Add a converter check that `generated/hype/textures/FixTex/gen_viseur`
exists. A second assertion: moving the target farther yields a smaller on-screen size from a fixed
Camera3D (world-space check).
