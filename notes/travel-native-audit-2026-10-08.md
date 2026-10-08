# Native travel prerequisites — 2026-10-08

This follows camera checkpoint `ffd1220` (integrated on main as `ab7bacc`).
The completed native changes are the **pure meta19 point-turn geometry API**,
saved actor-local current/previous waypoint progress, and **meta30/32/33 point
requests using native CharacterBody3D motion**. The finite traveling actor's
Mass consumer, selected fixed waypoint and no-hit application contracts are
closed by the source evidence below. No original executable
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
`movement_dependencies.c`. Its StaticLine dispatch checkpoint is
`288ec333edcfb4f7ea9376c92bebfd34eaf79fad`, `src/waypoint/link_sampling.c`.
Mass12 is `4826d81795b17d0a4fb0baa29ec25f4b3adb9215`, independently reviewed
and integrated into original-engine main as `21935f4`. The six source modules
are `src/dynamics/{force_record,mass_integration,mass_control,mass_update,
mass_contact,mass_apply}.c`. Their twelve complete code spans total8,837 bytes.
The full Proc19–32 handler and angle helpers are checkpoint
`f6f56c1bc00b3736ed1b2faf59b9fc91d3330db4`, `src/ai/ai_vector_speed.c` and
`angle_math.c`, integrated into original main as `742ee86`.
This worker did not edit or compile original-engine C.

Both PE profiles have identical bytes for all 53 recorded spans and the
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
All twelve Mass spans independently match the reviewed checkpoint's hashes in
both profiles. The additional Walk control span00589bfa–0058a40f is2070 bytes,
SHA-256 `253ad819326f1c3fcd1e2a39a6645d918e8c75489d5d9a0d383bf07ff8b0910b`.
Its two dispatch tables select event14 branch00589cd6, independently of Mass9.

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
speed setter now clears caller body/external velocity immediately, retains the
configured vector and pending impulse, and leaves the pose unchanged. The point
consumer derives positive scalar1.4/0.8 from its full3D length. Native jump checks
verify that old velocity is cleared without discarding the impulse or changing
the relative orientation used when the jump commits. Ultra calls with a different
receiver still require separate compiler treatment: parameter writes select the
receiver, but the original caller owns velocity clearing.
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
0059f46d are now complete and reviewed in the Mass twelve-body checkpoint.
No-hit query signed<=0 leaves the integrated
candidate intact. Positive responses dispatch00599703/00597be6; zero response
does not make Mass update skip final application, while seven attempts or
response20 roll back candidate and zero velocity. Final application publishes
global/relative transforms and handles numeric motion spaces0/1/2 distinctly.
Positive-contact algorithms remain unverified where those branches are reached;
they do not block actor45's proved absent-query branch. Null-query00463df0 calls
00448280 before its Perso+18 null gate and returns zero without clearing the
contact table; a separate debug branch clears that table. The Walk event14 handler is
00589bfa; animation-root speed, horizontal projection and ordinary Character
gravity cannot be assumed for Mass9. Exact dispatcher instructions00443e00/00443f00 convert unsigned
millisecond DAT00754af8 with float0.001 and pass the rounded seconds to both
control and update callbacks; native physics delta therefore uses seconds.
The source identity-to-motion-space table0060dd90 selects zero for kinds0 and9.
005711f7 applies that table when initializing the selected mechanics. The lead's
read-only importer probe confirms actor45 Perso+8→wrapper+10→base+10 is zero;
its owned evidence is `output/integration-resume-20261008/source-motion-mode.log`
in the main checkout. Thus this actor reaches no-hit world-space application.

The implemented native API is `source_begin_point_motion(point, radius,
horizontal=false) -> int` and `source_begin_next_waypoint(slot) -> int`, with
`source_point_motion_finished(ticket)`, `source_point_motion_failure(ticket)`
and `source_cancel_point_motion(ticket)`.
Named targets, actor-owned progress and native physics own runtime state;
raw pointers/addresses remain source-reader evidence. These methods consume
the saved Curve3D, radii and per-point/per-link support properties. They do not
rebuild event lists, dynamics layouts, force queues, or a source scheduler.

Begin returns -1 on a reached unsupported/malformed request, zero for immediate
completion, or a positive revision while active. A null/empty lookup retains
the previously captured route; no capture completes immediately. A failed active
request becomes finished and retains its reason under its positive ticket.
Compiled AI must check failure after polling and before cancelling, so reaching
a later curve cannot silently resume the schedule. New requests supersede older
tickets. Completion/cancellation holds animation steering until the next accepted
action or motion request; Character gravity/contact continues normally.

Mass9 event14 uses full3D normalized target displacement and the positive configured
speed magnitude. It sets the source +X heading, with the near-vertical 0.001
threshold, and applies native world velocity through CharacterBody3D. The actor45
no-shape/no-hit branch advances in world space without Character gravity or
animation speed. Source force/time/mass float stages are understood, but the native
API does not reproduce an original force-record stack tail or claim x87 bit parity.
Positive contacts use Godot body contact facilities and do not claim the original
external response algorithms' behavior.

Character0 uses native animation displacement magnitude, mechanics speed/turn
limits, and ordinary body contact/gravity. Walk event14's00589d2f–00589d77 computes
the planar length and **multiplies** the planar direction by it. Heading reads
that scaled direction in00589e5d–00589ed8 with signed acos; replacing it with a
fresh planar normalization or atan2 changes an inclined target's heading. The
native point API preserves this finite geometry and both signed PI endpoints.
Point and whole-route Character requests share this steering helper, avoiding
two different implementations of the same event14 velocity/heading geometry.
Inclined no-animation input turns without inventing translation. Detailed source
contact projection and tilted-environment parity remain outside this finite check.

Meta33 refreshes current native height+0.05 on each control update; its captured
height does not become a fixed vertical destination. Radius is first rounded to
source float32. Equality at radius0.05 therefore completes, while0.049 requests
motion. Missing effective dynamics completes before snapping. Nonzero radii never
snap; zero radius uses the separate0.25 fallback and snaps all three coordinates.
Nonfinite native inputs fail explicitly; source unordered/x87 exception behavior
is retained as evidence, not emulated as a gameplay policy.

Next-waypoint completion advances at most one vertex or sample per update. The
previous-vertex -1 branch selects the actual point even on a route with later
unsupported geometry. StaticLine selects its endpoint at every sample and reports
last-sample immediately. Current sample still wraps modulo256 and old sample is
saved as previous sample. Open terminals reset and complete; closed terminals
reset and yield one continuation. Unsupported next-link initialization preserves
the already advanced progress and reports failure. It never scans a whole route
or substitutes a line for a reached curve.

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
The earlier point-turn prerequisite's combined native/source check passed in36.51 seconds after4.11
seconds of build time; the compiled actor-turn regression passes in 0.41
seconds. The release build passes in 67 seconds. The four safeguards pass
7/6/2/14 tests respectively. That prerequisite's Clippy warnings were later
repaired in the lead-owned shared checkpoint.

Shared typed AI compilation, bool persistence/export, manifest entries and
version coordination are lead-owned. This branch incorporates reviewed shared
main checkpoint8d68004, including the audio/API dependencies, effective bool,
Mass9 resource, all four progress arrays and selected-point/link support flags.
The source-compiled meta30/32/33 integration and actual-game handoff remain the
lead's acceptance work. Later curves/relative routes, positive original Mass
contact algorithms, exceptional x87/NaN behavior and other mechanics modes
remain unverified. This batch makes no claim of full intro or game acceptance.

The saved-progress prerequisite additionally passes native scene save/reopen
and actor instance isolation in0.31 seconds. Its combined native/source check
passes in27.45 seconds after0.44 seconds of build time, including the assertion
that actor45 has no collision component. Logs and the refreshed canonical
dependency report are `progress-final.log` and `dependencies-progress.json`.

The motion batch's final native parser/runtime check passes in2.93 seconds
after4.22 seconds of build time (`shared-character-steering.log`). The unchanged test name is
`native_point_turn_geometry_uses_source_coordinates`; its scope now also covers
Proc29/jumps, full3D Mass/no gravity/source+X heading, radius equality/zero snap,
meta33 height refresh, Character root motion/wall/floor contact/cancellation,
inclined no-animation heading, both signed half-turn directions, selected
StaticLine on an otherwise unsupported route, low-byte/retained selection,
coincident vertices, sample wrapping, open/closed terminal reset and reached
unsupported point/link failures. Native scene save/reopen and independent actor
progress remain in this harness. Godot Euler extraction exposes a positive PI
pose as negative PI; point motion reads the wrapped world forward heading and
retains source float32 facing/half-turn boundaries instead of choosing the other
turn direction.

`manor_travel_dependencies_are_bound` passes in the combined source/native run
(50.63 seconds after0.46 seconds of build time), including effective dynamics
presence and the no-collision-query assertion. Canonical source/Rete evidence is
`dependencies-motion.json`. AI integration26 tests, intro API1, locomotion1
(21.12 seconds), source/model/compiler AI65 tests (11 local cases deliberately
excluded), and all four mandatory safeguards7/6/2/14 pass. The legacy route
fixture passes in9.41 seconds after its lead-approved test-only repair: its
duplicate was reparented directly under the level but retained the original
actor's deeper shared-service NodePath. Updating that duplicate's owner path
preserves real shared inventory/magick ownership and strict error checking.
No runtime ownership guard or diagnostic suppression was added.

The first native half-turn/contact fixture attempts exposed the native Euler
boundary and1.5mm vertical floor recovery. The final cancellation assertion
checks no horizontal steering, zero velocity and continued native floor contact.
The first sandboxed legacy AI checks failed because the Godot editor's loopback
listener was denied; narrowly permitted isolated reruns passed. These failures
and their corrections remain in the ignored evidence logs.

Formatting and root `cargo clippy --locked --offline --all-targets -- -D warnings`
pass (15.14 seconds for Clippy). Workspace-wide Clippy reports the pre-existing
eight-argument `observe_magick_behavior` in unowned `crates/orrery/src/orrery.rs:722`.
No suppression or unrelated controller change is included. Build/check process
ownership and cleanup are recorded by the existing supervisor in the evidence
directory; no original-game processes were launched.

After consolidating Character steering, the final release build passes in68.14
seconds, root all-target Clippy in3.73 seconds, and formatting in0.70 seconds.
Their supervised results report no timeout, leaked children or remaining owned
processes (`release-motion-shared.json`, `clippy-root-motion-shared.json`,
`fmt-motion-shared.json`). No root game publication or long acceptance run was
performed in this worker checkout.
