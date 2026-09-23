# Sector environments and fog

Hype Montreal sectors reference ordered environment lists. An environment can
reference mechanics, sound, and visual-environment data; a visual environment can
reference water and fog. Preserve these as distinct shared asset identities. Source
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

The active function pointer at 007fed30 resolves to **0049a460**, an entry absent
from the current decompiler function catalog. Its MCP-captured code was
disassembled locally. It disables fog when near is negative or near is not
less than far, converts RGB to the Glide color, multiplies near/far by four
for `guFogGenerateLinear`, and calls `grFogTable`. It caches infinite distance
but does not use it to generate this table. Live driver cache values are 48.4,
12384 and 5000. The separate 0049a5e0 driver has different ramp parameters;
its infinite-distance behavior must not be substituted for the active driver.

## Model and adapter boundaries

The useful model owns environment values and references. OpenSpace owns record
interpretation, list traversal, source addresses and reconstruction. Rete
persists independently shared values; Godot projects them into native resources.
Export derives references from current model links and verified layout rules.
A renderer must select the environment associated with the active sector or
surface; a single global fog value cannot represent mixed or clear sectors.

## Godot projection and limits

Godot's native depth fog can represent the source near/far/color controls, while
per-sector/surface selection belongs to scene rendering. The source infinite
source distance is retained even when it does not participate in the active
linear-fog table. Invalid source ranges disable fog.

Godot's depth curve is an approximation to the quantized linear Glide table.
Depth-fog properties were verified on local Godot 4.7.2 and against the official
[Environment reference](https://docs.godotengine.org/en/stable/classes/class_environment.html#class-environment-property-fog-density).
The large source range makes fog weak at Astrolabe's normal interior distances.
This projection does not model dynamic-light attenuation or sector light
selection.

## Evidence record

Dated live observations, implementation checkpoints, rendering comparisons, and test reports are retained in [the fog evidence note](../../notes/environment-fog-evidence.md).
