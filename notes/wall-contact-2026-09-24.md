# Manoir corner contact: source geometry and behavior

This note fixes the collision oracle for the first wall-contact BDD scenario.
The route starts from the supplied English `manoir` save, turns Hype left by
physical input for 250 ms, then holds Up. It is separate from camera work.

## Behavioral observations

The original-game probe in Orrery run
`run-1790234371304234345` reached source position about
`(-79.823, 32.034, 5.007)` by 2.8 seconds of accumulated Up input. Hype
remained in walk action 32 and barely moved for the next 1.4 seconds of Up.
The input was six 700 ms segments with stopped-byte captures between them;
the BDD must still test a continuous held input. The final screenshot in that
run shows Hype at the manor building corner. The executable and supplied-save
hashes are in the [run comparison](./run-comparison-2026-09-24.md).

The exported Godot scene, using its saved Hype actor and Brain, reached the
same corner after a 250 ms physical Left turn and about 3.1 seconds of Up.
It reported walk action 32, floor contact and slide collision with source node
`world/1/5/0`. From 3.1 through 7.1 seconds of the hold, horizontal position
varied less than 0.001 units. The observed corner position was near canonical
source `(-79.839, 32.053, 5.0)`. This is diagnostic evidence; the executable
BDD result is still pending.

## Source-owned finite faces

The generated `manoir/level.tscn` carries triangles from the imported source
model. Nodes `world/1`, `world/1/5` and `world/1/5/0` have identity initial
world transforms. Godot axes map source `(x,y,z)` to `(x,z,-y)`. The two nearby
vertical faces in source coordinates are:

| Face | Source material and exported group | Three source vertices | Actor-facing normal |
| --- | --- | --- | --- |
| Angled | `level:/models/world/1/5/0/materials/7`, shape 1, triangle 1 | `(-80.609024,32.603770,5.035125)`; `(-79.015520,28.686500,4.536334)`; `(-79.317210,28.002354,10.880944)` | `(0.926721,0.366340,0.083569)` |
| Cross | `level:/models/world/1/4/0/materials/6`, shape 0, triangle 6, instanced within `world/1/5/0` | `(-80.609024,32.603770,5.035125)`; `(-78.642820,32.603714,10.114957)`; `(-78.642810,32.603714,5.035129)` | approximately `(0,-1,0)` |

The source winding of the angled face points away from the actor; the normal
above is reversed to point toward the free approach region. At the original
contact endpoint, the lowest movement sphere's center is about 0.562 units
from the angled face plane and projects inside its triangle. Its clearance
after the 0.55-unit radius is about +0.012. The other two sphere centers
project outside that angled finite face. The cross face is about 0.570 units
from each sphere center, with all three projections inside its triangle;
clearance is about +0.020. These independently extracted faces match the two
normal directions seen in the Godot contact probe.

Hype's source movement collision assets are
`fix:/collision/character-zones/196716`, `196724`, and `196736`. The exported
active state uses three spheres of radius 0.55 at source vertical offsets
0.53, 1.10 and 1.65 from the actor origin. For an actor position `a`, sphere
center `c`, actor-facing unit normal `n` and a face point `p`, signed plane
clearance is `dot(n, c-p) - 0.55`. Plane clearance applies only where the
center's orthogonal projection lies in the finite triangle; otherwise use
sphere-to-triangle distance. The BDD must check both faces near the corner,
approach progress, held-contact stability, floor/lateral behavior, and
nonpenetration at its recorded checkpoints. A Godot collision report must
identify `world/1/5/0`; a stopped actor alone is insufficient.

The exported group triangle ordinals are not original OpenSpace face IDs.
The face-to-material relationship and vertices are source derived, but
per-face original IDs are not retained in the current model. Stopped-byte
samples also cannot prove that there was no sub-sample crossing; the BDD should
describe its claim as sampled contact plus native collision corroboration.

## First BDD result and contact divergence

The first headless Godot four-scenario run
(`TestResults/behavior-suite-1790235517933878224.json`) passed turn, walk
and run. Its wall scenario reached `world/1/5/0`, stayed on the floor, and
held within 0.001 units under Up, but met the cross face alone: cross-face
clearance was about +0.002, angled-face clearance about +1.452. The 250 ms
setup turn ended at heading -2.44346 radians, versus -2.1177 in the earlier
Godot diagnostic that reached both faces. The scenario correctly failed its
angled-face assertions.

A same-process Godot repeatability probe restored the same save pose three
times, waited for startup, and applied physical Left for 250 ms each time.
The settled headings were -2.30383, -2.25729 and -2.30383 radians with no
translation. A later short physical correction settled at -2.16421. Adjacent
results differ by about 0.04654 radians, consistent with a quantized turn per
physics update; the nominal hold duration is inadequate as a Given state.
Three same-process original-game reloads of the supplied save followed by the
same 250 ms physical Left turn settled at headings -2.45463, -2.39599 and
-2.35131 radians. Two continued Up probes reached approximately the same
corner by 3.5 seconds: source `(-79.82` to `-79.83`, `32.03` to `32.05`,
`5.007)`, still in walk action 32. This is run
`run-1790235755139852297`; the original-game session was stopped. The source
turn is also variable, yet its endpoint is stable across the measured range.
The failed Godot heading of -2.44346 lies inside that range. A backend-specific
heading correction would have concealed the contact difference, so the A/B
probe below compared native contact response at the same heading.

The first isolated original-game BDD wall scenario then ended its nominal
250 ms setup turn at -2.48535 radians, outside the measured range. At its
4.2-second held sample the actor was already at
`(-78.157,33.553,5.091)` and continued north of the cross face, rather than
stopping at the corner. Its two-face contact assertions correctly failed
(`TestResults/behavior-wall-contact-1790236346813686966.json`). Thus a fixed
duration turn is not a repeatable Given for either engine; source heading and
the reachable corner interval must be measured, then reached with physical
turn corrections. The wall scenario still requires the same source-owned two
faces on both backends.

A subsequent same-process original-game route probe restored the supplied
save three times and used only physical turn pulses before one continuous
5-second Up hold per trial. Headings -2.35689, -2.44625 and -2.40716 radians
all stopped at the corner, with endpoints spanning only about 0.014 source
units (`run-1790236502181136928`). The earlier -2.48535 route miss falls
outside that observed contact interval. The BDD Given therefore targets
-2.40 ±0.05 radians by reading the settled heading and, when necessary,
applying bounded physical Left/Right pulses. The probe found 20-35 ms pulses
could produce no turn, whereas a 50 ms pulse changed heading; correction
pulses use at least 50 ms and their actual edges/headings are recorded. This
establishes a reproducible observed interval, not a frame-perfect route.

The failed Godot position `(-78.317,32.157,5.002)` is about 0.55 units from
the cross face's right vertical edge at source horizontal coordinates
`(-78.643,32.604)`. Its slide normal in Godot coordinates
`(0.589,0,0.808)` points radially from that edge toward the actor. This is
consistent with sphere-to-finite-edge contact rather than evidence of an
unrelated face. The native `CharacterBody3D` wall sliding angle was tested as
the candidate cause.

A Godot contact probe at heading -2.44346 verified collider shape 0 is the
`ConcavePolygonShape3D` for cross-face material
`level:/models/world/1/4/0/materials/6`. Its reported contact point in source
coordinates was `(-78.642807,32.603714,5.533412)`, on the cross wall's
vertical end edge. This confirms the finite-edge interpretation. Godot 4.7.2
A/B probes at the same -2.44346 heading held Up for 4.25 seconds. Default
settings and `floor_block_on_wall = false` both stalled at approximately
`(-78.318,32.158,5.002)`. Setting only `wall_min_slide_angle = 0` let Hype
move about 1.52 source units farther along X, then stop near
`(-79.840,32.051,4.990)` with both the angled and cross face contacts.
The maintained character component now sets that native property when gameplay
motion is enabled. The fresh converted-project BDD check follows below.

## Passing independent behavior and comparison

The isolated original-game BDD wall scenario passed all 15 source assertions
in one clean game launch (`/tmp/original-wall-isolated-20260924b/behavior-wall-contact-1790237071015581218.json`).
Its 250 ms Left turn initially landed at -2.51048 radians; one physical 59 ms
Left correction and 1.8-second settle reached -2.39320. The 4.2- and
6.5-second held samples were approximately `(-79.821,32.049,5.009)` and
`(-79.826,32.034,5.007)`, with 0.016 source units between them. Action 32
remained active during the hold, then 47 after release and settling.

A fresh normal `manoir` import/export using maintained Godot support passed
all 18 Godot wall assertions in one clean engine launch
(`/tmp/astrolabe-wall-contact-bdd-fresh/behavior-wall-contact-1790236718344678055.json`).
At heading -2.44346 it reached about `(-79.840,32.050,4.990)` by 4.2 seconds,
stayed grounded and within 0.001 source units while Up was held, and returned
to idle after release. Both reports have the same recording SHA-256
`652c0acea2f361ef5a3aeeed6a684643326ae99c514a383ead4fe3ebf5b2f705`.
Orrery's cross-backend comparison passed 22/22 checks, with horizontal
position differences of 0.0185 and 0.0205 source units at held contact
(`/tmp/wall-contact-comparison-20260924.json`). The original screenshot was
captured while held; headless Godot correctly reports its render capture as
pending, so visual pairing remains open.

The later full original movement BDD suite passed all four scenarios in one
game session, including wall contact (16/16 assertions). Its wall setup
reached -2.44625 without a correction, and the contact endpoint was again
near `(-79.82,32.04,5.01)`. The suite report is
`TestResults/original-four-movement-20260924/behavior-suite-1790237298354606540.json`.
The matching fresh Godot four-scenario suite also passed in one headless engine
session (`TestResults/fresh-four-movement-20260924`, session
`1790237492624866771`). Their full-suite wall reports passed a fresh
cross-backend comparison of 23/23 checks. The independent wall-only 22/22
comparison above remains useful because it also exercised a physical heading
correction in the original setup.
