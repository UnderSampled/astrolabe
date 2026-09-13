# Sector environments and fog

Astrolabe now follows sector environment lists into shared environment, visual
environment and fog assets. These are separate useful content identities because
the source stores independent references at both boundaries. Seventeen Astrolabe
sectors share one of each; neither sector traversal nor Rete persistence copies
the shared values. This extends the provisional Rete schema to **8**. Source
addresses and placement remain private to OpenSpace.

## Source evidence

Raymap's Montreal `Object/Sector.cs` identifies the environment list but does not
decode its entries. Hype supplies the remaining evidence:

- `SECT_GetEnvironmentInList` (004e0770) returns the element's first pointer.
  The sector list at +4 uses the existing 16-byte doubly linked element layout.
- `ENV_fn_hCreateEnvironment` (005a8f40) allocates 12 bytes. Accessors at
  005a9250, 005a9270 and 005a9290 identify mechanics, sound and visual pointers.
- `VET_fn_hCreateVisualEnvironment` (0050c730) allocates eight bytes for water
  and fog pointers. Fog creation at 0050c2a0 allocates 28 bytes; setters/getters
  at 0050c200..0050c290 identify near, far, infinite distance and three RGB floats.
  The final word remains explicitly unknown and reconstructs unchanged.
- `SCT_fn_hGetEnvironmentFromPosition` (004e21c0) uses ordered surface crossings
  to select the corresponding environment. Drawing at 004df0e0 configures fog
  from that visual environment before submitting sector geometry and characters.
  This is distinct from later-engine kind-6 light fog.

The local corpus probe decodes `(near, far, infinite) = (12.1, 3096, 5000)` and
RGB `(0.744094, 0.744094, 0.744094)`, with a zero final word. There is one unresolved
mechanics environment, no sound environment and no water effect. The importer
reports those component limits instead of inventing their content.

Hype MCP run `20260912-213818`, executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`, loaded the
English save SHA-256
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c` into `manoir`
and used logged level navigation to reach `astrolabe`. Read-only MCP observations
resolved the active visual environment via 008c208c and confirmed all 28 fog
bytes. Evidence is local in `output/environment-mcp-evidence.json`; no source
asset was changed in memory.

The active function pointer at 007fed30 resolves to **0049a460**, an entry absent
from the current decompiler function catalog. Its MCP-captured code was
disassembled locally. It disables fog when near is negative or near is not
less than far, converts RGB to the Glide color, multiplies near/far by four
for `guFogGenerateLinear`, and calls `grFogTable`. It caches infinite distance
but does not use it to generate this table. Live driver cache values are 48.4,
12384 and 5000. The separate 0049a5e0 driver has different ramp parameters;
its infinite-distance behavior must not be substituted for the active driver.

## Adapters and reconstruction

`model::environment` owns useful values and role references. OpenSpace owns the
declarative binrw ENV/VET/fog records, list traversal and reconstruction.
Rete saves each independently shared asset as MessagePack; Godot consumes these
same values without reopening source blocks or regenerating relocations.

`write_environments` reconstructs shared records and derives their references.
Unresolved mechanics/sound/water links require explicit caller-supplied component
layouts; omitting one fails. `write_sectors` also requires environment targets
and regenerates the ordered environment list. Corpus checks compare fog bytes
exactly and normalize relocated pointer fields when comparing ENV/VET records.
These controlled fragment APIs do not yet assemble a full scene or SNA. The CLI
scene exporter emits shared fog leaf records and reports the uncomposed ENV/VET
and sector layout work explicitly.

## Godot projection and limits

The development preview applies fog when every sector selects the same effective
fog. Mixed fog or fog/clear sectors produce a diagnostic and disable global fog;
correct per-sector/surface draw routing remains future work. Scene metadata keeps
all shared environment values for inspection. Native preview settings use the
source near/far/color and a depth-fog density of one. Invalid source ranges remain
disabled, matching the active Hype driver. The stored infinite distance remains
in the model but does not alter this projection.

Godot's depth curve is an approximation to the quantized linear Glide table.
Depth-fog properties were verified on local Godot 4.7.2 and against the official
[Environment reference](https://docs.godotengine.org/en/stable/classes/class_environment.html#class-environment-property-fog-density).
The large source range makes fog weak at Astrolabe's normal interior distances.
This change does not address the separate dynamic-light attenuation and sector
light selection gaps.

Focused tests cover Rete edits, sharing at both reference boundaries, unknown-word
preservation, regenerated sector/asset pointers, missing-layout rejection, native
Godot properties and the mixed-environment guard. Generated assets and screenshots
remain local.

## Rendered comparison and validation

The fresh original screenshot is
`local/runs/20260912-213818/screen-1789264195341146775.png` in the external harness.
The camera chain was resolved again in this process, rather than reusing heap
addresses from a previous run. Vertical FoV is **69.88379766284831 degrees**, near
plane 0.25 and viewport 1024×768. The source camera matrix and derived Godot pose
are in `output/fog-camera-evidence.json`.

Before/after captures use the same llvmpipe renderer, pose, FoV and actor settings
as the transparency comparison. ImageMagick reports **zero changed pixels** in
this opening view. Against the fresh original, both have scene-above-HUD RMSE
0.0297184, ladder-region RMSE 0.051406 and eye-region RMSE 0.0691787. Reports are
`output/fog-pixels-before/`, `output/fog-pixels-after/` and `output/fog-pixels-change/`.
This supplies no evidence of a visible fog improvement at this viewpoint; remaining
filtering, lighting, idle-phase and HUD differences are still present.

A separate native rendering probe places an unshaded white surface at source
distances 5, 1554 and 4000, using the generated fog resource. Its center pixels are
respectively gray levels **1.0, 0.886275 and 0.745098**, demonstrating actual
near/middle/far behavior rather than merely saved properties. The distant value
matches the source fog gray at eight-bit precision. Evidence and captures are in
`output/fog-compare/fog-distance-report.json` and `fog-distance-*.png`.

The initial desktop screenshot attempt timed out after 90.07 seconds and cleaned
its child processes. Offscreen X11 captures then passed in 4.77 and 4.38 seconds;
the distance rendering probe passed in 3.84 seconds. This timeout was a failed
capture, separate from the successful complete suite.

The full local suite passed **78 tests in 239.345 seconds**, including release
compilation, corpus/native Godot checks and cleanup. Report:
`TestResults/20260913T014933.418118Z-2136654.log.json`. No leaked children or suite
timeout were reported; formatting, strict Clippy and diff whitespace checks passed.
