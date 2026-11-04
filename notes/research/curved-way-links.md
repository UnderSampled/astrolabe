# Curved way links (connection types 1-3) and the actor's motion along them

Static research, 2026-11-04. Read-only on hype-decomp and the repo. Sources: Ghidra
bodies under `hype-decomp/reference/ghidra/functions/` (and `ghidra-recovered/` for
the missing-raw 0054c800/0054c8d0), readable decomp in `hype-decomp/src/waypoint/*`
and `src/ai/waypoint_motion.c`, `src/ai/waypoint_meta_wrappers.c`, one objdump
(`reference/source-profile/MaiDFXvr_bleu.exe`, 0x531720..0x5318c7), the port
(`godot/addons/astrolabe/character/character.gd`, `src/godot/route.rs`,
`src/model/route.rs`, `src/openspace/level/way.rs`, `records.rs`), and the original
level data: `disc/Gamedata/World/Levels/{manoir,brigand}/*.sna` decoded with a
throwaway script outside the repo (SNA envelope + MMG allocation walk, the same
framing `session.rs` uses) to read the payload bytes and test the formulas below.
Nothing was run in Godot.

## Answer

### Link record (72 bytes, `WayLinkWire`)

`+0` payload pointer, `+4..+0x37` the 52-byte DynamicObject (speed/rotation
profile), `+0x38` connection type, `+0x3c` dynamic kind, `+0x40` current sample
(byte), `+0x41` used, `+0x44` length (float32). Constructor `WP_fnh_Link_Create`
00525ef0 (0x48 bytes, `+0x38 = 4`, `+0x3c = 4`). Types are named by the script
loader `WP_fnv_Link_BuildFromScriptParams` 005266b0 (`_strcmpi` chain):

| `+0x38` | Name | Payload (`link+0`) | Current point (`WP_fnv_Link_GetCurrentPoint` 00526cc0) |
|---|---|---|---|
| 0 | `StaticLine` | 8 bytes `{waypoint*, position*}` (00530e50 / 00530ec0) | `payload->position` (a cached world position of the endpoint waypoint; the port's `static_endpoint`) |
| 1 | `DynamicLine` | the target `WayPoint*` itself (AddRef'd) | `WP_fnv_WayPoint_ComputeLocation(waypoint)`: live world position of an actor-relative waypoint. Never seen in manoir or brigand. |
| 2 | `Bezier` | 12-byte `{curve*, samples*, lengths*}` (FUN_00530510 / 005304d0; `Allocate` 00526290 case 2) | `payload->samples[link.+0x40]` as three float32 (FUN_00530cc0: `*(payload+4) + (s & 0xff) * 0xc`) |
| 3 | `Circle` (circle arc) | same 12-byte `{curve*, samples*, lengths*}` (FUN_0052fbf0 / 0052fc90) | same table read (FUN_00530340, byte-identical body) |
| >=4 | invalid | none; script loader reports error 0x32 | `GetCurrentPoint` returns without writing |

`+0x3c` dynamic kind (0 none, 1 Constant, 2 Linear, 3 Sinus; script names `""`,
`Constant`, `Linear`, `Sinus`) is independent of geometry. Every link in manoir and
brigand has kind 0, so its speed profile never applies (see motion rules).

### Payload structs for types 2 and 3 (all baked into the level file)

The three payload pointers, the curve record, the sample table and the length
table are separate MMG allocations present in the level SNA. They are not rebuilt at
runtime: the only constructors/mutators (`BuildFromScriptParams` 005266b0,
`SecondPass` 00526b30, `ChangeConnectionTypeBezier/CircleArc` 00526380/00526450,
FUN_00530bf0/00530280/00530d10/00530390, FUN_00530950/00530010) are reached from the
script loader `WP_fne_Way_ScriptCallBackLoad` 005188c0 and FUN_00518b90 only. At
run time the original only reads the tables. I verified on real data that they are
present and equal to the formulas below.

Bezier curve record (`MTH3D_tdstBezierCurve_`, allocation 52 bytes, 0x31 used;
`MTH3D_fnv_BezierCurve_Init` 00531180, `SetParams` 00531670):

| offset | field |
|---|---|
| 0x00 | start point P0 (3 x f32) |
| 0x0c | end point P3 |
| 0x18 | start vector (tangent offset) |
| 0x24 | end vector |
| 0x30 | sampling rate `n` (u8) (`GetSamplingRate` 00531240: `return curve[0x30]`) |

Circle-arc curve record (`MTH3D_tdstCircleArcCurve_`, allocation 40 bytes, 0x25 used;
`Init` 00531b30, `SetParams` 00531c60):

| offset | field |
|---|---|
| 0x00 | start point |
| 0x0c | end point |
| 0x18 | centre |
| 0x24 | sampling rate `n` (u8) (`GetSamplingRate` 00531e10: `return curve[0x24]`) |

Sample table: `(n+1)` x 12 bytes (f32 x, y, z); length table: `(n+1)` x f32, cumulative
distance along the sample polyline, `lengths[0] = 0`, `lengths[i] = lengths[i-1] +
|S[i]-S[i-1]|` (FUN_00530ac0 / 00530180, same body). `link+0x44` (length) is
`lengths[n]` (FUN_00530e20 / 005304a0; `SecondPass` stores it as float32). Verified:
the file's `link.length` equals `lengths[n]` and every table (see Census).

Sample count / last sample: **`n + 1` samples, indices `0..n`; "last sample index" is
`n`.** `WP_fnuc_Link_GetLastSampleIndex` 00526c90 returns FUN_00530cb0/00530330 =
`GetSamplingRate`, i.e. `n` (types 0/1 return 0 in AL). `WP_fnb_Link_IsCurrentSampleIndex
LastSampleIndex` 00526c50 is **`n < current_sample`** (strict "past the last"), not
equality: for types 2/3 `(uint8)rate < link[0x40]`; for 0/1 it is always true.
(Quoted from the readable src: "Types2/3 compare the helper's rate AL with a fresh
current-sample byte after the call: rate < current, not equality.")

### Point formulas (sample index `s`, `0 <= s <= n`)

Both tables are `float32`; the original builds them with x87 (extended-precision
intermediates, float32 stores at the places shown in the 00531720 disassembly:
the two control points `P0+v0` and `P3+v1` are stored as float32, the four weights
are stored as float32, and the three-term sums are accumulated in extended
precision before the final float32 store). Baked file values match a double
recomputation to within 1.1e-5 absolute on coordinates around 160, which is one
float32 ulp at that magnitude. Prefer the baked tables over recomputation.

**Bezier** (`MTH3D_fnv_BezierCurve_CalculatePoint` 00531720; cubic Bezier in
Bernstein form, tangent *offsets* not absolute control points):

```
P0 = start            P1 = start + startvec
P3 = end              P2 = end   + endvec
t  = s / n            u  = 1 - t
S[s] = u^3 P0 + 3 u^2 t P1 + 3 u t^2 P2 + t^3 P3
```

Quoted (Ghidra): `fVar18 = fVar16*fVar16*fVar15*3.0; fVar17 = fVar15*fVar15*fVar16*3.0;
... = end*t^3 + (endvec+end)*fVar17 + start*u^3 + (startvec+start)*fVar18`. The loop
is `t = 0; do { ...; t += 1 } while (t <= n)` with `t/n` formed per iteration, so
`S[0] = P0` and `S[n] = P3` exactly (`t = n/n = 1`). `n = 0` would divide 0/0 and
produce NaN; no shipped link has `n = 0`.

**Circle arc** (`CalculatePoint` 00532810 -> `ComputeLocalInformation` 00531fb0,
`ComputeMainAngle` 00532560, `CalculatePointWithLocalInformation` 005326e0,
`ComputeVectorFromLocalToGlobalRepere` 00532470):

```
u   = normalize(start - centre)
w   = normalize(end   - centre)
nrm = normalize(u x w)            (normal of the arc plane)
v   = normalize(nrm x u)          (in-plane, towards w)
r   = |start - centre|            (radius is the START radius)
A   = acos(u . w)                 (main angle, 0..pi, always the short arc)
S[s] = centre + r*cos(a_s)*u + r*sin(a_s)*v,   a_0 = 0,  a_{s+1} = a_s + A/n
```

The angle accumulates in float32 (`param_1 = param_4/(float)rate + param_1`), the
radius is `sqrt` of the squared start offset, and the local vector `(r cos, r sin, 0)`
is mapped through rows `u`, `v`, `nrm` plus the centre. **The last sample is at
angle `A` and radius `r`, not the end waypoint**: the end point only fixes the
angle. In data one circle link ends 0.35 m short of its end waypoint (manoir route
1439, link 1); the others coincide with it. Sample 0 is the start point.

`SecondPass` 00526b30 (script load) fixes both ends from the neighbouring waypoints:
Bezier `SetParams(P0 = vertex(point i), P3 = vertex(point i+1), keep vectors, rate
unchanged 0xff)`, circle `SetParams(start, end, keep centre)`. In shipped data
`S[0]` equals waypoint i and, for Bezier, `S[n]` equals waypoint i+1 (distance 0).

Cached table, built at load: yes (cached in the level; recomputation is never done at
run time).

### Motion rules (GoOverWay 27 / GoToNextWP 30)

`0054c8d0` (GoOverWay) is `54c800(caller, continue=1, dir=0)`; `0054c910`
(GoToNextWP) is `54c800(caller, continue=0, dir=0)`; `0054c8f0` is the back variant
`(1, 1)` (disassembly, per `way-designer-operand.md`). Per update tick, 54c800
(`ai_update_waypoint_meta_0054c800`):

1. `way = slot 0` (FUN_0052e240); none -> return 4.
2. `WP_fnx_Way_GetCurrentSpeed` is called and its result **discarded**. Its only
   effect is `Link_SetCurrentSample`. Link speed (`link_speed.c`: kind 1 constant,
   2/3 `FUN_0052fa80` profile over `pos/length`) is therefore not applied by these
   two meta actions (other callers: camera actions 0054f390/0054f810). Walking speed
   comes from the actor's own dynamics (54a9f0 sets the unit direction vector).
3. `GetNextPoint` 005181e0 returns `(target, radius)`:
   - the way slot is `{+0x10 current point p, +0x11 current sample s, +0x12 previous
     point q (0xff none), +0x13 previous sample ps}` (FUN_00459b60..00459c80).
   - if `q == 0xff`, or `p == 0 && s == 0`: `target = location(waypoint p)`.
   - else `link = GetCurrentLinkForPosition` 00518e60 (`q==0xff` -> `links[p-1]`;
     `q < p` -> `links[q]`; going back: `ps == 0 ? links[p-1] : links[p]`);
     `Link_SetCurrentSample(link, s)`; `target = Link_GetCurrentPoint(link)`, which
     for type 2/3 is `samples[s]`.
   - **`radius = waypoint[p].radius` for every sample**, not a per-sample radius.
4. `54a960` -> `54a9f0`: `current` = actor world translation; displacement =
   `target - current`; arrival callback 0054a9a0 is `|displacement| <= radius`
   (3-D). Not arrived: if `radius == 0` and `|d| <= 0.25` (double at 0x5c41f0) treat
   as arrived; else set `motion_state+0x1c..0x24` to the unit displacement
   (`1.0 / sqrt`, constants 0x5c41e4 = 0.0f, 0x5c41f8 = 1.0f) and return control 1.
   Arrived (or the 0.25 zero-radius case): `radius == 0` -> snap the translation to
   `target` and recompute the matrix; return control 4. No dynamics -> return 4.
5. If control has bit 4 (arrived): `_WP_fnuc_Way_CurrentVertexReached_` 00518500
   (`dir=0`) or `...ReachedBack_` 00518600 (`dir=1`) with `continue_word`; if it
   returns 0 the updater returns `control` (4, action finished); otherwise (and in
   every non-arrived tick) it returns 1 (still running).

   `CurrentVertexReached_` (forward), quoted flow: refresh `p`, `q`, `s`; store
   `ps = s`; **if `p != 0 && q != 0xff`**: `link = GetCurrentLink` (`q < p ? links[q]
   : links[p]`), `s = s + 1` (byte wrap), `Link_SetCurrentSample(link, s)`,
   `if (!IsCurrentSampleIndexLastSampleIndex(link)) return 1;` (stay on this link,
   target the next sample on the next tick). Otherwise (`p == 0`, `q == 0xff`, or
   `s > n`): if `count == p + 1` (end of the way): `InitWay` and return 1 if the way
   is closed (`count > 1 && points[p] == points[0]`) else 0; else
   `WP_fnh_Way_NextWayPoint` and return `continue_word`.
   `NextWayPoint` 00517f10: `q = p; p = p + 1` (or wrap to `p = 0, q = 0xff` when
   `q == 0xff && p == count-1`), then if `q != 0xff && p != 0`: `s = 0` and
   `link[0x40] = 0`, then `Link_InitLinkForUsed` (00526d40: types 2/3 set
   `link[0x40] = 0`; type 0 FUN_00530ff0). `InitWay` 00518760: `p = 0, q = 0xff,
   s = 0, ps = 0xff`.
   `PrevWayPoint` 00518040 and the back variant decrement `s`, start from
   `GetLastSampleIndex - 1`, and do not call the "past last" test (00518600).

Consequences for a curved link `links[i]` (from `p = i`, `q = 0xff`/earlier):
arriving at waypoint i advances to `p = i+1, q = i, s = 0`; ticks then target
`S[0], S[1], ..., S[n]` of `links[i]` in order, one sample advance per tick at most
(the arrival test uses the radius of waypoint `i+1`, here 1.0 m, while samples are
0.2-0.5 m apart, so the actor tracks the first sample that is still farther than
1.0 m away); on arrival at `S[n]`, `s = n+1 > n` -> next waypoint (`p = i+2`,
`s = 0`) or end of way. Sample `n+1` is never read from the table because
`NextWayPoint` (or `InitWay`) resets `s` before the next `GetNextPoint`. For
GoOverWay (`continue = 1`) the action keeps running through every vertex of an open
way and returns control 4 only at its end; for GoToNextWP (`continue = 0`) it
finishes at the first waypoint reached, but keeps running (returns 1) while it
steps through the samples of a curved link.

Confidence: high for the layouts, the sample count rule, the point formulas and the
motion state machine (read directly from Ghidra/readable decomp, formulas confirmed
against 25 real links). Medium for exact float32 bit patterns of recomputed samples
(irrelevant when the baked tables are used). No evidence for DynamicLine (type 1)
runtime behaviour beyond the code read.

## Evidence

### 1. Payloads and tables

- `WP_fnv_Link_Allocate` 00526290: case 0 `FUN_00530e50`, case 2 `FUN_00530510`,
  case 3 `FUN_0052fbf0`, case 1 leaves payload 0 (filled by `ChangeConnectionType
  DynamicLine` 00526620, which stores the waypoint and calls the empty
  `UpdateDynamicInfo` 00526f40).
- `FUN_005304d0`: `FUN_00515690(0xc)` then `[0] = 0, [4] = 0` (12-byte payload);
  `FUN_00530950` (Bezier) and `FUN_00530010` (circle): `samples = alloc((rate+1)*12)`,
  `lengths = alloc((rate+1)*4)`, then fill (`FUN_00530ac0` / `FUN_00530180`).
- `FUN_00530cc0` / `FUN_00530340`:
  `puVar1 = (undefined4 *)(*(int *)(param_1 + 4) + (param_2 & 0xff) * 0xc);` three
  dword copies. `FUN_00530cb0` / `FUN_00530330`: `GetSamplingRate(*param_1)`.
- Length at sample: `FUN_00530cf0` (Bezier) / `FUN_00530370` (circle):
  `*(float *)(*(int *)(param_1 + 8) + (param_2 & 0xff) * 4)`; total length
  `FUN_00530e20` / `FUN_005304a0`: `lengths[rate]`. `WP_fn_Link_xGetCurrentPos`
  00526e30 uses them (`pos` along the link, forward `lengths[s] + d`, back
  `lengths[s] - d`) for the speed profile only.
- Script load (`BuildFromScriptParams`): Bezier params `[2]` rate (`atoi`), `[3..5]`
  start vector, `[6..8]` end vector (then `FUN_00530d10(payload, 0, 0, startvec,
  endvec, rate)`); Circle: `[2]` rate, `[3..5]` centre (`FUN_00530390(payload, 0, 0,
  centre, rate)`). Start/end points come from the waypoints in `SecondPass`.
- Verification (this note's decoder on manoir.sna/brigand.sna, block 17/1): the
  payload allocations are 12 bytes; Bezier curve allocation 52, circle 40; sample
  allocation `12*(n+1)`; length allocation `4*(n+1)`; `n` at curve+0x30 / +0x24;
  file samples equal the formulas above to <= 1.11e-5 (Bezier) / 7.6e-6 (circle) for
  all 25 curved links; `link.length` equals `lengths[n]` to 1e-4; `S[0]` equals the
  start waypoint; Bezier `S[n]` equals the end waypoint; the allocation stride is 5
  per curved link (link 72, payload 12, curve, samples, lengths).

### 2. Motion

All of: 0054c8d0/0054c910/0054c800 (`ghidra-recovered`, `src/ai/waypoint_meta_wrappers.c`),
0054a960/0054a9f0/0054a9a0 (`src/ai/waypoint_motion.c`), 00518500/00518600 and
`GetNextPoint`/`GetCurrentSpeed` (`src/waypoint/waypoint_progress.c`),
NextWayPoint/PrevWayPoint/InitWay/GetCurrentLink/ForPosition/IsBackSens (Ghidra only,
no readable src), slot accessors 0043e3a0..0043e480 -> 00459b60..00459c80 (+0x10..+0x13).
Quoted: `if (WP_fnb_Link_IsCurrentSampleIndexLastSampleIndex(link) == 0) return 1;`
(readable) and the Ghidra form `cVar2 = ...IsCurrentSampleIndexLastSampleIndex(uVar3);
if (cVar2 == '\0') { return 1; }`. Constants read from the exe: 0x5c41e4 = 0.0f,
0x5c41f0 = 0.25 (double), 0x5c41f8 = 1.0f, 0x5c3f70 = 0.0f, 0x5c3f74 = 1.0f.

### 3. Dynamics bytes

`DynamicObject` (13 dwords at `link+4`; `fn_vDynamicObject_Create` 0052f8b0): `+0` type
byte; type 1 constant speed at `+4`; type 2 linear speed from `+8` to `+0x14`; type 3
sinus `+0xc`, `+0x10`, amplitude `+0x18`; rotation start `+0x1c..0x24`, end
`+0x28..0x30`. `FUN_0052fa80`: linear `(f[+0x14]-f[+8])*t + f[+8]`, sinus
`(sin((f[+0x10]-f[+0xc])*t) + 1) * f[+0x18]`. All shipped dynamics here are
`00 a1 0a 01 00 ...` (type 0, remaining bytes padding); speed 0, rotation unused.

## Port status per link of the chain

`character.gd` and `route.rs` model the **StaticLine-only** subset:

| Chain link | Original | Port |
|---|---|---|
| link record `+0x38` dispatch (00526cc0) | 4 types | implemented for type 0 only (`route.rs`: `l.connection == 0 && l.dynamic_kind == 0 && static_endpoint == points[i+1]` -> `link_supported[i] = 1`); other links: fail |
| payload for types 1-3 | table above | **lacks** (importer reads the payload only for type 0, `way.rs:163`; `geometry` is an identity placeholder) |
| sample table `S[s]` / `GetCurrentPoint` | per sample | **lacks**; StaticLine stand-in: "selects its endpoint directly at every sample" (`_source_select_waypoint_target`, `selected = link+1`) |
| `IsCurrentSampleIndexLastSampleIndex` (00526c50) | `n < s` | implicit: type 0 is always past last, so the port advances the point after one step (`_source_advance_next_waypoint`) |
| per-actor slot `{p, s, q, ps}` (+0x10..+0x13) | per actor | implemented: `source_route_points/samples/previous_points/previous_samples` (initialized by `_source_reset_route_progress` = `InitWay`) |
| link byte `+0x40` and `InitLinkForUsed` | derived from the slot | not needed: always set from the slot before use; keep derived |
| `GetNextPoint` target selection (005181e0) | link sample point when `q != 0xff && !(p == 0 && s == 0)` | **implemented for GoToNextWP only** (`_source_select_waypoint_target`; same `ForPosition` rule), target limited to the endpoint |
| `radius = waypoint[p].radius` | all samples | implemented (`route.radii[point]`) |
| `54a9f0` arrival (3-D `<= radius`, 0.25 zero-radius, snap) | yes | implemented (`_source_point_control`, `_source_route_target`) |
| `CurrentVertexReached_` state machine (00518500) incl. `InitWay` at the end, closed-way test, `continue` word | yes | implemented for GoToNextWP (`_source_advance_next_waypoint`), minus the "not past last sample, return 1" branch; **GoOverWay does not use it**: `source_begin_route` / `_source_route_target` scan points directly with their own index and can pass several waypoints in one call (a stand-in that matches the source only for StaticLine routes; no source cite for the multi-point scan) |
| `GetCurrentSpeed` call (discarded) | no effect with kind 0 | not needed |
| `PrevWayPoint`, `ReachedBack_`, `InitWayBack`, `IsBackSens` | back variants | lacks (not reached by these actors) |
| types 2/3 in `source_begin_route` | | `route.supported` false -> returns -1; GoToNextWP -> `_source_fail_point_request("selected waypoint needs unsupported link geometry")` |

Failing boar message source: `_source_select_waypoint_target` at the
`route.link_supported[link] == 0` check.

## The importer

- Wire (`records.rs`): `WayLinkWire { geometry: u32, dynamics: [u8; 52], connection:
  u32, dynamic_kind: u32, sample: u8, used: u8, unknown: [u8; 2], length: f32 }`
  (72 bytes, matches the offsets above). `way.rs` `read_route` reads `geometry` as an
  identity (`s.pointer(a, 0)` -> `source_identity`) and, only for `connection == 0`,
  follows it (`StaticLineWire { waypoint, position }`).
- For types 2/3 the bytes are all reachable with the existing session API:
  `s.required(geometry, 0 | 4 | 8)` for curve/samples/lengths, `s.bytes`/`allocation_bytes`
  for the records, `s.pointer` handles the relocation sites (all pointers relocate
  within the level block 17/1). No new container work is needed. The importer
  currently records none of these bytes (they are uncovered in the byte-coverage
  report; `byte-coverage-gaps.md` does not list them separately).
- Model (`model/route.rs`): `RouteLink.geometry: Option<SourceObjectIdentity>`
  placeholder and `static_endpoint: Option<AssetRef<Waypoint>>`;
  `Route::validate` rejects `static_endpoint` on a non-zero connection.

## Census (manoir and brigand level routes; every link of every route read)

Connection types per link (rate `n` for types 2/3); every link has dynamic kind 0.
Waypoint radii are 1.0 on all checked curved routes; shipped current/previous/unknown
way bytes are 0.

| Level | Route | Used by (perso, model) | Points | Links by type | Closed |
|---|---|---|---|---|---|
| manoir | `routes/17-1/1434` | 45 `iDummy1_Dummy_Traveling_Manoir_MAD` (365) | 2 | 1 x type 0 | no |
| manoir | `1439` | 45 (365), second route | 6 | 1 x type 0, 4 x **type 3** (n = 17, 17, 15, 10) | no |
| manoir | `1464` | 5 `iGogood_manoir` (210) | 6 | 5 x type 0 | yes |
| manoir | `1477`, `1484`, `1491` | 5 (210) | 3, 3, 2 | all type 0 | no |
| manoir | `1496` | 146 `iEcureuil_Manoir1` (816) | 6 | 5 x **type 2** (n = 15) | no |
| brigand | `2176` | 179 `iEcureuil_Brigand1` (1862) | 5 | 4 x type 2 (n = 15) | no |
| brigand | `2199` | 180 `iEcureuil2_Birgand1` (1862) | 6 | 5 x type 2 (n = 15) | no |
| brigand | `2236` | 4 `I_Wild_Boar_A` (309), slot 0 | 8 | 7 x type 2 (n = 15) | **yes** |
| brigand | `2227` | 4 `I_Wild_Boar_A` (309), slot 1 | 4 | 3 x type 0 | yes |
| brigand | `2283` | 6 `I_Wild_Boar_B` (309), slot 0 | 5 | 4 x type 0 | yes |
| brigand | `2274` | 6 `I_Wild_Boar_B` (309), slot 1 | 4 | 3 x type 0 | yes |

Totals: manoir has 12 type 0, 5 type 2 and 4 type 3 links; brigand has 10 type 0 and
16 type 2 links (25 curved links in all). The boar failure is route `2236` (Boar A
slot 0, 7 type 2 links); Boar B's two routes and Boar A's slot 1 are StaticLine
only. Type 1 and types >= 4 do not occur in either level. (Rates:
type 2 always 15; type 3 17, 17, 15, 10.)

## Port implications

Importer (promote the payload):
- Replace `geometry: Option<SourceObjectIdentity>` + `static_endpoint` with a typed
  `RouteLinkGeometry` enum: `Static { endpoint: AssetRef<Waypoint> }`, `Dynamic {
  waypoint }` (type 1; keep unsupported), `Bezier { start, end, start_vector,
  end_vector, rate: u8, samples: Vec<[f32; 3]>, lengths: Vec<f32> }`, `CircleArc {
  start, end, centre, rate, samples, lengths }`. Keep the source identity for
  coverage. Add wire structs `CurvePayloadWire { curve, samples, lengths }`,
  `BezierWire { start[3], end[3], start_vector[3], end_vector[3], rate: u8 }`,
  `CircleArcWire { start[3], end[3], centre[3], rate: u8 }`; validate allocation sizes
  (12 / >=49 / >=37), `samples.len() == lengths.len() == rate + 1`, finiteness, and that
  `lengths` is the running polyline distance (tolerance 1e-4). A test should also
  recompute the samples from the formulas (tolerance ~2e-5) so a future layout
  misreading fails loudly.
- Import the baked tables as data: they are deterministic from the curve record,
  but the original does not recompute them, and x87 bit patterns cannot be
  reproduced exactly from Rust/GDScript. The recomputation test above is the
  consistency check.

Godot export:
- Keep the `Curve3D` for waypoints (do not use its baking: it would resample).
  Add per-link native data to `route.gd`: `link_kind: PackedByteArray`,
  `link_last_sample: PackedByteArray` (`n`; 0 for static), and the sample points as
  `Array[PackedVector3Array]` (one per link, empty for static), converted with the
  same source-to-Godot mapping used for waypoint positions (`vec3(vector(...))`).
  Set `link_supported[i] = 1` for types 0, 2 and 3. Leave type 1 and any
  `dynamic_kind != 0` link unsupported: their speed profile is not applied by 54c800,
  but only dynamic kind 0 is verified in data.

Runtime (`character.gd`):
- Reuse the existing slot arrays and `_source_advance_next_waypoint` as the single
  implementation of 00518500, adding `if sample > n for the selected link then
  advance, else return true (stay)` after the sample increment (the "not past last"
  branch). The test is `n < sample` using `link_last_sample`; static links have
  `n = 0`-equivalent "always past".
- In `_source_select_waypoint_target`: for a sampled link, `_source_point_target =
  link_samples[link][sample]` (no `link+1` substitution) and keep `radius =
  radii[point]`. The sample must be < n+1 whenever read; the original never reads
  sample n+1 because `NextWayPoint`/`InitWay` resets it first.
- Make GoOverWay (`source_begin_route`) drive the same state machine with
  `continue = true` (one `GetNextPoint` / arrival test / `CurrentVertexReached_` per
  tick, finishing only when `CurrentVertexReached_` returns 0), replacing the
  multi-point scan in `_source_route_target`. For StaticLine routes this is the
  same path as today except the scan no longer jumps several waypoints in one
  call; check the intro and Gogood routes that the change is neutral.
- Link speed/dynamics: not applied by these actions. `GetCurrentSpeed` is a
  discarded call with only a `SetCurrentSample` side effect that the port already
  derives from the slot. Do not invent a speed from `link.length` or the profile.

## Not determinable statically

- Bit-exact x87 results of the sample formulas (irrelevant if the baked tables are
  imported).
- Behaviour of `DynamicLine` (type 1) and of non-zero dynamic kinds: no shipped
  instance in manoir or brigand; I did not census the other levels (route files
  exist for them).
- What `FUN_0052eeb0(object, dynamics, 0x14)` (called by 54a9f0 after setting the
  direction) and `FUN_0044d700` (called each tick by 54c800) do to the actor's
  dynamics. The port's walking speed comes from animation root motion, not the
  source's force path.
- Whether an actor's `GetCurrentSpeed` call can pass 0xff as the sample
  (`GetCurrentSampleIndex` 00518f00 returns `max(s, ps)` and `ps` is 0xff after
  `InitWay`): only matters for dynamic kinds != 0 (out-of-range table read), not for
  the shipped data.
- Which boar slot (0 or 1) its script selects at run time (that is an AI question;
  slot 0 holds the type 2 route).

## hype-decomp functions read without a readable src/ body

All read from the raw Ghidra function files (no `src/` rewrite exists; coverage.tsv
status `untouched`): 00526290, 005266b0, 00526b30, 00526380, 00526450, 00526620,
00526d40, 00526e30, 00526ec0, 00525ef0, 00530510, 005304d0, 00530950, 00530010,
00530ac0, 00530180, 00530b60, 00530220, 00530bf0, 00530280, 00530d10, 00530390,
00530cf0, 00530370, 00530e20, 005304a0, 0052fbf0/0052fc90 (not opened beyond their
callers), 00531180, 00531670, 005315c0, 00531b30, 00531bd0, 00531c60, 00531720,
00532810, 005326e0, 00531fb0, 00532560, 00532470, 0052fa80, 0052f8b0, 0052fa70,
0052fae0, 0052fb40, 00517f10, 00518040, 00518760, 005187c0, 00518390, 00518400,
00518e00, 00518e60, 00518f00, 00459b60, 00459b90, 00459bc0, 00459c20, 00459c50,
00459c80, 0043e3a0..0043e480, 0054c8f0 (from the earlier note's disassembly) and
0054f390/0054f810 (callers of GetCurrentSpeed). Additionally disassembled with
objdump: 00531720 (0x531720..0x5318c7, float32 staging) and the rdata constants at
0x5c3f70, 0x5c41e4..0x5c4200.

## Addendum 2026-11-04: port on native Curve3D

Stephen's design decision: behavioral parity on Godot's Curve3D rather than
stepping baked samples.

- Importer: `RouteLink.curve` holds the shape (Bezier: start, end, start and
  end vectors; circle arc: start, end, centre) and the sampling rate. The
  baked sample and length tables are derived from the shape and not kept.
- Export: `route.gd` gains `link_curves`, one Curve3D per link (empty for
  StaticLine). A Bezier link is two points whose out and in handles are the
  start and end vectors, so it is the original's cubic exactly. A circle arc is
  split into pieces of at most a quarter turn with handles (4/3) tan(theta/4) r,
  ending at the main angle on the start radius as 00532810 does. Checked
  against the original's baked tables: every circle-arc sample of manoir route
  1439 lies within 1.4 mm of the exported curve, every Bezier sample of route
  1496 within 5.7 mm (Godot's bake tessellation).
- Runtime: on a curved link the actor steers toward the point one arrival
  radius past its nearest offset on the curve, clamped to the end. The arrival
  test is the current waypoint's radius at the link's end, where the original's
  last sample lies, and the vertex events follow as before (GoToNextWP's
  00518500 machine, GoOverWay's vertex scan). The original instead targets the
  first sample farther than the radius, one sample per tick; speeds may differ
  slightly, by decision.
- Behavior check (headless, 40 s): the Manoir squirrel walks 21.5 m with a
  median 0.38 m off its curve; Brigand boar A walks 21.6 m, median 0.31 m and
  at most 0.56 m off; boar B 28.2 m; the Brigand squirrel 21.8 m. No route
  guard reports.
