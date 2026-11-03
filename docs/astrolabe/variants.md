# Variants: parity, native, presentation and modernization

The converter produces a faithful, Godot-native base.

- **Gameplay** is one native implementation, matched behaviorally (section 1).
- **Presentation variants** are runtime modules.
- **Modernization** is an override layer.

Generated scripts never carry wrappers or if-parity branches, and the
hand-written core never references a variant.

## 1. Gameplay: one native implementation, matched behaviorally

Astrolabe aims for behavioral parity, not exact simulation parity. A player should see the
same rules, odds, timings and collision outcomes. They are verified against the original
through Orrery, recorded takes and resync. Exact-run reproduction is not a goal.

There are therefore no gameplay parity variants and no intrinsics table. Gameplay uses
Godot's native RNG with the original's distribution semantics, Godot physics, and GDScript's
native numbers.

If exact simulation parity is ever wanted, it would be a separate target, for example a C#
or GDExtension simulation core with 32-bit floats, not a toggle in this one.

### Randomness

Randomness uses Godot's native global RNG, emitted directly in the generated scripts, and
matches the original's distribution exactly. Func_RandomInt (005383e0) truncates real operands toward zero, and 004c58c0
returns `lower + table[i] * (upper - lower) / MAX`, with every table value below MAX and a
truncating division. The result is therefore:

- in [lower, upper) for an ordinary range;
- lower when the bounds are equal;
- in (upper, lower] for a reversed range.

The emitted form is `lower + int(randf() * float(upper - lower))`, with both operands
truncated to integers first. Hand-written choices (the armor piece) also call Godot's RNG
directly. For determinism, routes take `"seed": n` and the game reads `ASTROLABE_SEED`; both
call `seed()`.

## 2. Presentation modules (runtime)

These are eventually exposed in an options UI:

- **lighting**: the parity shader against Godot native, ideally via a global
  shader parameter; today the `astrolabe/rendering/parity_lighting` switch;
- **camera**;
- **sound mixing**;
- **animation blending**;
- **sprites**.

The data under them always ships: vertex colours, original frames.

## 3. Control and camera schemes (runtime modules, longer term)

Recorded, not planned:

- **A modern twin-analogue-stick camera**, with movement and dodging combat.
  Dodging touches gameplay, so it is a player-behavior module.
- **VR** via OpenXR.

## 4. Modernization overrides (future tools)

Upscaled textures, retimed, tweened or redefined animations, and high-res meshes.
They never edit converted output. They live in a separate folder or resource
pack at the same resource paths: resource remaps, alternate
AnimationLibraries, inherited scenes. Re-conversion therefore never clobbers
them, and "classic" is simply the base without the pack.

## 5. Unity

A future Unity target reuses the same split: the same decoded data, with its own
conversion profiles and modules. See the C# / Unity section of the README.
