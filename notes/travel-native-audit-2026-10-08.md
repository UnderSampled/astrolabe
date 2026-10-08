# Native travel prerequisites — 2026-10-08

This follows camera checkpoint `ffd1220` (integrated on main as `ab7bacc`).
The completed native changes are the **pure meta19 point-turn geometry API**
and saved actor-local current/previous waypoint progress properties.
Meta30/32/33 movement is still gated on the original Mass consumer, point
movement and selected-waypoint/advance contracts below. No original executable
was run, and no comparison/font worktree was read or changed. The source-only
checkout is `/tmp/astrolabe-camera-native-20261008`; builds, test projects,
configuration and evidence are isolated there. Shared AI/model/export changes,
versions and actual-game acceptance remain lead-owned.

## Reference and exact evidence

Raymap's `AITypes_Hype.cs` supplies the practical catalogue names. Original
function exports and static PE instructions resolve behavior and ABI. The
meta30 clean-C checkpoint is `54d76b0c01d565a11cb871bedeeccf740fb1ed1b`, read from
`/home/stephen/Documents/Codex/2026-10-08/task-15/hype-sol-interpreter/`:
`src/ai/waypoint_meta_wrappers.c` and `include/hype/ai/waypoint_meta.h`.
The subsequent meta19/32/33 checkpoint is
`562aed54203db63d2a0295f7e67eb3656b18bd69`, in the same checkout,
`src/ai/waypoint_meta_parameter_capture.c` and `waypoint_meta_position.c`.
The complete point/way progress checkpoint is
`933b8f31ebc84b9d445248def7e80ddfd08b5479`, read from
`/home/stephen/Documents/Codex/2026-10-08/task-13/hype-sol-core/`:
`src/ai/waypoint_motion.c`, `src/waypoint/waypoint_progress.c` and
`movement_dependencies.c`. This worker did not edit or compile original-engine C.

Both PE profiles have identical bytes for all 42 recorded spans and the
meta33 height constant. Whole-file SHA-256 values are those in the
[camera audit](camera-native-audit-2026-10-08.md). The section table supplies
each file offset. Ignored `output/travel-native-evidence/pe-spans.json` retains
every span, profile path, file hash and constant value. Representative exact
function spans are:

Inclusive span | File offset | Bytes | SHA-256
--- | --- | ---: | ---
0054cbc0–0054cd17 (meta19 init) | 14bfc0 | 344 | `15a96f69ad680564b0c65c441b84ccf8ac76a88d0b5873b7a9a2405343fb64d5`
0054d030–0054d118 (meta19 update) | 14c430 | 233 | `aa27bac04ca89ab260c15f7d9c8e50ba5ac4201b313eb8dd8f5c146e9db540c2`
005181e0–00518286 (selected point) | 1175e0 | 167 | `c7ef649449b35e72d81f33d328640c5de3f41892f42dd7346a8dfd325f1e3b6f`
00518500–005185f5 (advance) | 117900 | 246 | `3fbbef6098f9e622189a3862b1694f0697b9e3638ef9fa7d61fa355123c14f7b`
00443540–00443681 (callback registration) | 42940 | 322 | `2358733031723ce2025642de24d83275fff11086d17bf22408dde4cad9124f4b`
0058ab89–0058b40e (Mass control) | 189f89 | 2182 | `50b7639e3b605f3dcd0a12c266ab17aed2ed59a88d08e409f07a147f06009089`

Hashes establish provenance; they do not establish host x87 or runtime parity.
The point/advance algorithms now have complete static clean-C contracts.
The Mass helper bodies are authored and remain pending their reviewed checkpoint.

## Closed point-turn API and compiler contract

`source_turn_angles_to_position(point: Vector3) -> Vector2` accepts a captured
absolute **source-coordinate** Position vector and returns `(start, shortest
delta)` in radians. Source `(x,y,z)` maps to native `(x,z,-y)`; source Z is
height and does not enter the horizontal angle. The method reads world-space
translation and the actual transformed forward column. Orthonormalizing a
composed/scaled basis before this query would alter that direction, unlike
the source matrix-vector multiplication. The existing actor-target query now
shares this geometry helper.

0054cbc0 uses 00522a50 to capture operands, subtracts caller translation,
and calls 0052eb70/0052ecb0/0052ec00 for heading, wrapping and shortest delta.
The zero horizontal vector selects PI. Exactly antipodal deltas retain their
sign. The native query changes no actor pose or motion state; nonfinite input
returns nonfinite angles. The existing `source_turn_to_heading(float)` remains
the native heading write used by compiled frame steps.

The compiler must preserve the **signed** count: step is delta divided by
abs(count), with a zero denominator replaced by 1e-6 during initialization.
0054d030 first returns AL4 for count<=0; negative counts do not become one.
It next calls 0052ef00; absent effective dynamics also returns AL4. Positive
counts decrement once, add/wrap one step, queue event5, write heading and
axis `(0,0,1)`, then return AL1. After the final positive update, completion
occurs on the following count-zero update. Coroutine frame counts/cancellation
belong to compiled scripts; no source scheduler is added to character support.

The declared native bool is `source_motion_binding_present`, default false.
It represents the resolved Perso+8 wrapper **and wrapper+10 dynamics** required
by 0052ef00, independently of mechanics kind. It is distinct from camera135's
owner-wrapper flag. The lead owns its model/import/export proof and population;
this prerequisite does not infer presence from a mechanics resource. The pure
geometry query precedes and does not perform the update's presence check.

## Connected authored consumers and preserved bindings

The finite camera handoff remains World154 normal3 → travel365 normal2 →
mode6/statue294 → return travel365 normal3 → Gogoud210 normal12 → mode-1.
The earlier camera audit called meta30 “GoOverWay”; its correct catalogue name
is **Action_GoToNextWPOfWay**. GoOverWay is meta27 and has a different continue
argument. Native movement must preserve this distinction.

Travel365 normal2 captures route slot0 after SetVectorSpeed `(0,-1.4,0)`;
normal3 captures slot1 after `(0,+0.8,0)`. Both camera rules copy SelfPosition.
The local source check proves actor45 is model365, initial/current state1,
without an animation, using **Mass mechanics kind9, mass1**. Its state0 has
Character kind0; that other card does not describe the traveling state.
Its Perso has no collision component. Source00463df0 returns signed zero when
callerSO+4→Perso+18 is null, before querying profiles or positive contact
responses. This selects Mass9's no-hit candidate path for actor45; the source
check asserts the absence rather than assuming it from an invisible dummy.

Binding | Initial actor progress | Fixed waypoints | Connection evidence
--- | --- | --- | ---
slot0, route1434 | `[0,0,ff,ff]` | 136→135, radius1 | StaticLine to135
slot1, route1439 | `[0,0,ff,ff]` | 129→134→133→132→131→130, radius1 | first StaticLine to134; four later connection3 geometries remain unresolved

005181e0 restores actor-local current and **previous vertex**. The previous
vertex sentinel ff selects the actual waypoint and its radius, bypassing link
sample geometry. The exact actor-binding getters are 00459b60 (+10 current
vertex), 00459bc0 (+11 current sample) and 00459c20 (+12 previous vertex).
00517f10 copies the old current vertex into the previous field before advancing;
00518760 initializes vertex0, sample0, previous vertexff and previous sampleff.
The ff branch must not be described as a current-sample sentinel. Thus the
initial slot1 destination129 is meaningful despite unsupported later curves.
Its current exporter marks the **whole** route unsupported; the new API needs
selected-point support and progress semantics, not a whole-route false bypass
or a straight-line substitute for later curves. 00518500 distinguishes actual
waypoints from sampled links and can advance one sample, loop/reset at the end,
or move one vertex. StaticLine semantics below close the finite fixed-link branch;
sampled curves remain a movement gate.

The saved native actor fields are `source_route_points`, `source_route_samples`,
`source_route_previous_points` and `source_route_previous_samples`, all
PackedInt32Array values. Previous fields preserve ff as semantic -1; current
sample starts at zero. Getter00459c80 proves that the fourth byte is +13, and
0043e460/0043e480 forward its query/write. Native scene save/reopen preserves
all four fields, and instance mutation does not alter another actor's progress.
The shared Route resource owns geometry and per-point/per-link support; actor
selection does not become a cache on that shared resource.

00518e60 chooses the position link independently of the ordinary current link;
00518f00 uses max(current sample, previous sample), with source ff treated as
unsigned255 during that query. The native -1 sentinel must be converted back
to that source meaning for this comparison, rather than ordered as a negative
sample. 005181e0 selects the actual point when previous vertex is ff, or when
current vertex=0 and current sample=0; otherwise it selects the position link's
point. Radius remains that of the current vertex. 00518500 first saves the
current sample to previous sample. Its linked branch increments sample modulo
256 before querying last-sample. It resets on a terminal vertex, returning
AL1 for a pointer-identical first/last waypoint loop and AL0 for an open route;
other advancement returns the caller's continuation byte, zero for meta30.

The exact StaticLine branches are now independently checked against both PEs:
00526cc0 type0 copies its saved endpoint vec3 directly; 00526c50 type0 returns
logical AL1 immediately, and 00526c90 returns logical AL0. Thus StaticLine
always reaches its last sample on advancement, with no sampled interpolation.
The last-sample return must not be promoted to a fabricated full EAX1. Exact
additional spans include00526c50–00526c8f (64 bytes, SHA-256
`c7d0b45ac0509e6a03f5ee5ad8fd13d352e415a7769af923cc26477fbbe31359`)
and00526cc0–00526d27 (104 bytes,
`d9e6445f435132324d483e1443d0f628ba9c2aacee2140b75feb951df7667dd4`).
Unsupported later curved links do not invalidate an earlier selected fixed
point, and are not replaced with straight lines.

Gogoud12's captured meta19 target is `(-12.80000019,143.5,33)` with count3.
The next meta32 targets are that same point/radius1 and
`(-15.5,138.5,33)`/radius1. Meta33 captures an Ultra Func0 Position plus
radius1; source reader/typed AI must bind its referenced actor canonically.
Gogoud action36 has kind19 mechanics; later actions28 and1 use Character kind0.
The check retains these distinct cards and their source state relationships.
All inspected Persos, families, brains, mechanics, routes and waypoints reopen
equal through Rete; this is a prerequisite check, not travel execution.

## Native movement gates and proposed signatures

Meta30 init0054b260→0054b200 evaluates one slot operand; real values use _ftol,
then low8 lookup. Nonnull/nonempty ways replace the captured way; failed lookup
leaves the old slot. Update0054c910→0054c800 returns AL4 for a null way, calls
CurrentSpeed for its side effects, then GetNextPoint. The latter overwrites a
reused caller stack argument with radius; **radius**, not caller bits, reaches
0054a960. The wrapper unconditionally sets dynamics-wrapper bit1. Completed
point movement calls 00518500 with continue0; zero returns the movement result,
otherwise AL1. Existing GoOverRoute scanning is not this one-waypoint API.

Meta32 update0054d3b0 reads captured point/radius and calls 0054a960.
Meta33 update0054d3e0 calls 0054b1b0, which rewrites target source Z to current
Z minus the double constant at005c4208, **-0.05**, on every update. Native target
Y is therefore current Y+0.05; this is not a flattened horizontal destination.
0054a9f0 uses inclusive 3D arrival, event14 direction requests, and a special
radius-zero near-point snap. Effective dynamics absence returns AL4. On success
it clears dynamics-wrapper bit0 and removes every event0c before checking
arrival. Callback false with zero radius also accepts distance<=0.25. Running
movement writes normalized full3D direction and queues event14, returning AL1.
Completion snaps only for zero radius, recomputes the relative transform and
refreshes the dynamics base from the caller's current wrapper, then returns AL4.
These are complete original-C contracts; exceptional unordered comparisons
remain source evidence rather than a claim of native NaN equivalence.

Proc_SetVectorSpeed29 reaches the missing-raw shared handler005404d0. The exact
catalogue handler word00603e90 points there; its dispatch table005408f8 selects
branch00540637 for procedure29. Parameter writes resolve the Ultra-selected
receiver, but existing linear/angular velocity is cleared through the original
caller's separate dynamics component. The finite authored travel calls use self
for both. The branch sets scalar speed to full3D vector length, normalizes a nonzero vector,
and stores direction. Source-zero uses float-zero at005c4160 and scalar zero.
The original behavior is more than storing a deferred raw vector. The native
point consumer can derive scalar1.4/0.8 from the captured source vector, while
immediate velocity clearing and direction storage need their own motion batch
and jump regression. No change to the existing speed setter is guessed here.
005404d0–005408f7 is1064 bytes, SHA-256
`c56a2a737fa5caf0812910a345f34ba474e7db23021afdf1cfe1521b71b1f3ed`;
the56-byte dispatch table hashes to
`a38bb2060f1f55dbda22ce2c9f4e9c48420acfeacb00bf5f1b8216f3fc37cf14`.

The Mass callback mapping is exact: 00443540 writes table+48=00587cc1 and
table+4c=0058ab89; 00443f00/00443e00 index paired callbacks by DNMkind*8.
0058ab89 event14 consumes full3D direction and scalar speed, plus heading
construction and 005a7a1d/0059ee70 force construction. Early original-C
instruction evidence proves fifth argument is a float time divisor and sixth
is the event ID. It clears dynamics+64..7c, derives heading from a normalized
temporary, then rereads the original direction/speed and scales the resulting
vector by rounded mass/divisor. 005a6dc8 sums force vectors, scales by rounded
inverse mass and time, then adds dynamics+64 velocity; the event has already
zeroed that velocity. 005a05c8/005a0642 apply another rounded velocity*time
step to translation/angular displacement. Algebraic mass/time cancellation
does not prove exact float behavior. Contact0059a504 and final application
0059f46d have now been authored in the Mass worker's twelve-body slice, pending
its static-reviewed commit. No-hit query signed<=0 leaves the integrated
candidate intact. Positive responses dispatch00599703/00597be6; zero response
does not make Mass update skip final application, while seven attempts or
response20 roll back candidate and zero velocity. Final application publishes
global/relative transforms and handles numeric motion spaces0/1/2 distinctly.
Positive-contact algorithms, application-space proof and exact helper evidence
remain required where those branches are reached; they do not block actor45's
proved absent-query branch. The Walk event14 handler is
00589bfa; animation-root speed, horizontal projection and ordinary Character
gravity cannot be assumed for Mass9. Clean C for the Mass pair, velocity/time
integration helpers is requested from the original-C coordinator. The latter's
point core/advance checkpoint is complete. No native motion implementation is
guessed here. Exact dispatcher instructions00443e00/00443f00 convert unsigned
millisecond DAT00754af8 with float0.001 and pass the rounded seconds to both
control and update callbacks; native physics delta therefore uses seconds.
The source identity-to-motion-space table0060dd90 selects zero for kinds0 and9.
005711f7 applies that table when initializing the selected mechanics. The lead's
read-only importer probe confirms actor45 Perso+8→wrapper+10→base+10 is zero;
its owned evidence is `output/integration-resume-20261008/source-motion-mode.log`
in the main checkout. Thus this actor reaches no-hit world-space application.

The proposed native API remains `source_begin_point_motion(point, radius,
horizontal=false) -> int` and `source_begin_next_waypoint(slot) -> int`, with
`source_point_motion_finished(ticket)` and `source_cancel_point_motion(ticket)`.
Named targets, actor-owned progress and native physics own runtime state;
raw pointers/addresses remain source-reader evidence. Selected destination,
radius, effective binding, speed and advancement contracts must close before
these methods are implemented.

## Focused checks and limits

`output/travel-native-evidence/` owns logs and the optional dependency report.
The native test has separate 15-second parser/run deadlines. It exercises
coordinate conversion, horizontal-only headings, wrapping, both antipodal
signs, transformed parents, actual forward columns, zero direction, pure
queries and invalid input. The existing compiled actor-target turn regression
also passes after sharing the geometry helper. The local source/Rete test has
a bounded selection and checks the concrete Mass, waypoint/radius/progress and
Gogoud mechanics prerequisites above. Mandatory supervisor/selection/nextest/
recording safeguards, release build and formatting are recorded separately.
The final combined native/source check passes in 36.51 seconds after 4.11
seconds of build time; the compiled actor-turn regression passes in 0.41
seconds. The release build passes in 67 seconds. The four safeguards pass
7/6/2/14 tests respectively. Workspace Clippy
remains blocked by the existing warnings listed in the camera audit; no lint
suppression or unrelated source repair is included.

Shared typed meta19 compilation, bool persistence/export, manifest entries and
version coordination are lead-owned. Native meta30/32/33, later route curves,
exceptional x87/NaN parity, authored full intro and final control handoff remain
unverified. This batch makes no claim of complete travel or game acceptance.

The saved-progress prerequisite additionally passes native scene save/reopen
and actor instance isolation in0.31 seconds. Its combined native/source check
passes in27.45 seconds after0.44 seconds of build time, including the assertion
that actor45 has no collision component. Logs and the refreshed canonical
dependency report are `progress-final.log` and `dependencies-progress.json`.
