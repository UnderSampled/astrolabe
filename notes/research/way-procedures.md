# Way procedures 220-222, the back direction, GoOverWay3D, Cond_IsValidWay

Static research, 2026-11-04. Read-only on hype-decomp and the repo (no build, no run).
Sources: `hype-decomp/reference/ghidra-recovered/functions/` (00542ff0, 0054b280,
0054c8d0/8f0/930/950/ca00, 005344a0), `reference/ghidra/functions/` (00517f10,
00518040, 00518760, 005187c0, 00518840, 00518e00, 00518e60, 00518f70, 00518fd0,
00526c90, 0043e3a0..0043e480, 00459b10..00459c80), readable src
`src/waypoint/waypoint_progress.c`, `src/ai/waypoint_meta_wrappers.c`, and an
`objdump -d` of the exe at 0x54c8d0..0x54ca10 for the wrapper pushes. New readable
rewrites for the previously unreadable bodies are in `src/ai/way_procedures.c`,
`src/ai/way_conditions.c` and `src/ai/waypoint_meta_wrappers.c`.

Notation. Way record: `+0` point table, `+4` link array, `+8` count `N` (byte), `+9/+a`
caches of p/q. Per-actor state keyed by the way pointer (`FUN_00459b10` finds the actor's
list node whose `+0xc == way`): `p` (+0x10), `s` (+0x11), `q` (+0x12, 0xff none), `ps`
(+0x13). Getters return 0xff when the actor has no state node; setters then do nothing.
All four are bytes (wrap mod 256). `links[k]` joins point k to point k+1 (so the forward
step from p=k reaches q=k, p=k+1 and uses `links[q]`). `n(L)` = `Link_GetLastSampleIndex`
(00526c90): 0 for link types 0/1, the sampling rate for curved types 2/3; samples are
`S[0..n]`. `loc(k)` = position of waypoint k, `rad(k)` = its radius.

## Answer

### 1. Procedures 220 / 221 / 222 (all 00542ff0)

The entry reads the procedure id (cursor-8: 0xdc=220, 0xdd=221, 0xde=222), evaluates
operand 1 and resolves it with `0043e370` (index into the calling actor's own way list;
same lookup the meta-action init uses), then tests the id in that order. No null check
on the way. None of them touches slot 0 of the actor (`FUN_0052e1d0`) and none affects a
running action's identity; they only rewrite the four state bytes (and the caches
`way+9 = p`, `way+a = q`).

| proc | p | s | q | ps |
|---|---|---|---|---|
| 220 ReInitWay (`InitWay` 00518760) | 0 | 0 | 0xff | 0xff |
| 221 ReInitWayBack (`InitWayBack` 005187c0) | `N-1` | `n(links[(N-2)&0xff])` | 0xff | 0xff |
| 222 InitWayWithWp(way, wp) (`InitWayWithWp` 00518840) at match index `i` | `i` | `i==0 ? 0 : n(links[i-1])` | 0xff | 0xff |

(220 is exactly what the port's `_source_reset_route_progress` stores: p=0, s=0, q=-1, ps=-1.)

222 selection: operand 2 is a WayPoint (full 32-bit pointer word). The entry scans
`i = 0, 1, ...` while `i < N-1` (loop bound re-read each pass) and takes the **first**
`i` with `points[i] == operand`. The last index `N-1` is never tried (for a closed way
the duplicate of point 0 is therefore harmless; for an open way its final point cannot
be selected). `N <= 1` skips the scan. **If no index matches, nothing is changed and no
error is raised**; the procedure returns normally. After a match the actor first targets
`loc(i)` itself (q=0xff means "go to the current point"), not point i+1. For a forward
run the `s` value is irrelevant (the first vertex-reached call sets `s=0` when it leaves
p); it is there to seed the back walk (see 2).
Edge cases that read out of bounds in the original: 221 on `N==1` (indexes `links[0xff]`),
and any of the three with a way that does not exist.

### 2. Back direction

Init actions: 29 and 31 use `0054b280`, which is `0054b200` with a pushed 1 that the
body never reads, i.e. identical to the forward init: it stores the way in actor slot 0
if the way exists and its count byte `+8 != 0`. **It does not reset p/s/q/ps.** The back
walk begins wherever the state is, normally right after 221 (p=N-1) or 222.

Per tick the updater is the same `0054c800` as forward with `direction = 1`:
29 GoOverWayBack = `(continue=1, dir=1)` (0054c8f0), 31 GoToNextWPOfWayBack =
`(continue=0, dir=1)` (0054c930), forward 27 = `(1,0)`, 30 = `(0,0)`.
Order: slot 0 way (none -> return 4); call GetCurrentSpeed and discard it (only touches
caches and a link's own current sample); `GetNextPoint` -> `(target, radius)`; move
toward target (`54a960`); if the move result has bit 4 (arrived) call
`CurrentVertexReachedBack` (00518600) with `continue`; if that returns 0 return the
move result (finished, 4), otherwise and on every non-arrived tick return 1 (running).

**Target selection** (same `GetNextPoint` as forward, nothing direction-specific):
- `q == 0xff`, or `p == 0 && s == 0`: `target = loc(p)`.
- else `L = LinkForPosition` (00518e60) with `q < p ? links[q] : (ps == 0 ? links[p-1] :
  links[p])` (`q == 0xff` -> `links[p-1]`, not reached here); `L.cur = s`;
  `target = L.GetCurrentPoint` (type 0: the link's endpoint waypoint = point k+1 of
  `links[k]`; type 1: its waypoint's live location; types 2/3: `samples[s]`).
- `radius = rad(p)` for every sample.
Going back `q > p`, so the link is `links[p]` (the link whose START is p) unless `ps == 0`.
So when walking back, the arrival radius is that of the waypoint at the **start** of the
walked link (forward uses the link's end waypoint).

**Vertex reached back** (00518600), all byte arithmetic:
```
(a) if p == 0 and ps == 0: InitWayBack(); return 0           // finished
(b) if q != 0xff and ps > 1:
        ps = s; s = s - 1; return 1                            // keep stepping this link
(c) ps = s; PrevWayPoint(); return continue
```
`PrevWayPoint` (00518040):
```
if q == 1 and p == 0:  q = 0xff; p = N-1                      // wrap
else:                   q = p;    p = p - 1
if q != 0xff:
    s = 0
    L = links[p]; m = n(L)
    s = (m == 0) ? 0 : m - 1                                    // start one sample before the link end
    if m == 0 and p != 0: s = n(links[p-1])                     // straight link: aim at end of previous link
```
(The `Link_SetCurrentSample` / `InitLinkForUsed` calls inside only set the links' own
cursor, which GetNextPoint overwrites.) A wrap leaves `s` and `ps` as they were, and
`ps` was just set to `s`.

**Return value / end of way.** There is no closed-way test in the back path (forward
tests `points[N-1] == points[0]` and loops). The only finish is rule (a): back at p == 0
with ps == 0, which also re-inits (p=N-1, q=0xff, s=n(links[N-2]), ps=0xff); the action
returns 4. For 31 (`continue = 0`) rule (c) also finishes the action at every waypoint
reached (the slot has already moved to the previous point); mid-link stepping (b)
returns 1. For 29 (`continue = 1`) (c) keeps running. A first call after 221 therefore
walks to `loc(N-1)` first (state q=0xff), exactly as the first forward call walks to
`loc(0)`.

**Curved links walked in reverse.** Idealised ("walk link L = links[p] from its end to its
start; arrival at its start with radius rad(p)"): after the vertex at the link's end,
targets `S[n-1], S[n-2], ..., S[0]` of L, one step per arrival, arrival radius `rad(p)`;
`S[n]` (the link end, where the actor already is) is not targeted again; the last target
`S[0]` is the start, then `PrevWayPoint` moves to the previous link. The original
implements this **only when the leg is entered with ps != 0 on a curved link and ps == n,
i.e. the first leg after 221, or after 222 with i > 0** (s was seeded to `n(links[i-1])`
for exactly this). Then each arrival at `S[k]` has `ps = k+1`, so (b) holds for `k >= 1`
and (c) fires at `k = 0`. Everything else in the original is not that clean walk:
- Leg entered with `ps == 0` (every leg after a fully walked or straight leg): the target
  uses `links[p-1]` (for p == 0 that reads the 4 bytes before the link array) with the
  index `s = n(L)-1` of the wrong link. A straight L (`n == 0`) is handled by the
  special `s = n(links[p-1])` and works: it aims at the end of the previous link, which is
  `loc(p)` for type 0 and for Bezier. A curved L (`n >= 1`) is **skipped**: one arrival
  at some sample of `links[p-1]`, then rule (c) with `ps = n(L)-1`; the next leg is then
  walked sample by sample only if that stale `ps > 1` at its first arrival (`n(L) >= 3`),
  otherwise skipped too.
- Curved `links[0]` reached at p == 0 ends in `PrevWayPoint`'s wrap (q = 0xff, p = N-1)
  instead of (a), because `ps = 1` on the last step; with `continue = 1` the action then
  runs another lap; (a) fires on the later lap.
So the original's reverse walk of several consecutive curved links is incoherent
(stale-`ps` indexing); it is coherent for straight links and for one leading curved
link. Whether any shipped script uses 29/31/221/222 was not checked here (the AI dumps
are not named); the loader has the actions, so a port decision is needed: implement the
exact machine above, or the idealised per-link reverse walk and accept the difference on
multi-curve back routes.

**GoOverWay3D (28, update 0054ca00 = `0054c950(caller, 1, 0)`).** Init is the forward
`0054b260`. `0054c950` is `0054c800` with exactly two steps removed: no
`way == null` test (the way is dereferenced; null would crash) and no
`GetCurrentSpeed` call (which only touched caches and one link's cursor, result already
discarded in 27). Target selection, arrival test (3-D sphere via `54a960`), vertex
reached call and return values are identical. So "3D" changes nothing observable for a
port beyond that missing speed call; it is a forward, continue=1 route like 27.

### 3. Cond_IsValidWay (condition 68, 005344a0; shared with 64-67)

Evaluates one operand (a way index; a real operand is truncated with `_ftol` first), looks
it up with `0043e370` in the calling actor's way list, and stores
`result = way == null ? 0 : (uint32)way->count_byte(+8)`. It tests existence in the
actor's list, not validity of anything: a present way with count 0 is false, any other
present way yields its point count (not normalised to 1). No other test (no connectivity,
no link check, no closed test). Conditions 64-67 in the same entry just store
`operand != 0`; unknown ids store 0.

## Evidence

- 00542ff0 (ghidra-recovered): `iVar6 = *(int *)(param_2 + -8);` ... `if (iVar6 == 0xdc)
  { WP_fnv_Way_InitWay(param_1,uVar4); } else { if (iVar6 == 0xdd) {
  WP_fnv_Way_InitWayBack(...); return uVar3; } if (iVar6 == 0xde) { uVar3 =
  FUN_00522610(param_1,uVar3,local_10); ... if ((uVar5 & 0xff) != 1 && -1 < (int)((uVar5 &
  0xff) - 1)) { do { iVar6 = WP_fnh_Way_GetElementFromWayPointArray(uVar4,param_1); if
  (iVar6 == local_10[0]) { WP_fnv_Way_InitWayWithWp(uVar1,uVar4,param_1); return uVar3; }
  ... } while ((int)(uint)bVar2 < (int)((uVar5 & 0xff) - 1));`. 00518fd0 returns
  `*(*way + (index & 0xff)*4)`; 00518f70 returns `*(byte*)(way+8)`.
- 00518760 InitWay: `FUN_0043e3c0(..,0); FUN_0043e440(..,0xff); ... FUN_0043e400(..,0);
  FUN_0043e480(..,0xff);` (setters 0043e3c0 p, 0043e400 s, 0043e440 q, 0043e480 ps ->
  00459b30/b90/bf0/c50 write `+0x10/+0x11/+0x12/+0x13`).
- 005187c0 InitWayBack: `FUN_0043e3c0(..,*(char*)(param_2+8) + -1); FUN_0043e440(..,0xff);
  ... uVar2 = WP_fnuc_Link_GetLastSampleIndex(*(link array + (byte)(p-1)*4));
  FUN_0043e400(..,uVar2); FUN_0043e480(..,0xff);`
- 00518840 InitWayWithWp: `FUN_0043e440(..,0xff); FUN_0043e480(..,0xff); FUN_0043e3c0(..,param_3);
  if ((char)param_3 == '\0') uVar2 = 0; else uVar2 = WP_fnuc_Link_GetLastSampleIndex(link
  array[(byte)(param_3-1)]); FUN_0043e400(..,uVar2);`
- 0054b280: `FUN_0054b200(param_1,param_2,1);` and 0054b200 (src) never reads the third arg;
  it only calls `FUN_0052e1d0(0, caller, way)` when `way != 0 && way[8] != 0`.
- Wrappers (exe): 54c8d0 pushes (0,1) -> `54c800(caller,1,0)`; 54c8f0 pushes (1,1);
  54c910 (0,0); 54c930 pushes (1,0) -> `(caller,0,1)`; 54ca00 `call 0x54c950` with
  (caller, 1, 0). All return the callee's AL unchanged (`add esp,0xc; ret`).
- 0054c950 (ghidra-recovered): `piVar5 = FUN_0052e240(0,param_1);
  WP_fnv_Way_GetNextPoint(iVar1,piVar5,local_c,&param_1); ...` - no `if (way == 0)
  return 4` and no `WP_fnx_Way_GetCurrentSpeed`; the exe shows the same shape (54c95e call
  0x52e240, then 0x5181e0 immediately).
- 00518600 (src quoted above): `if (WAYPOINT_BYTE(way, 9) == 0) { if (FUN_0043e460(object,
  way) == 0) { WP_fnv_Way_InitWayBack(object, way); return 0; } }` /
  `if (FUN_0043e460(object, way) > 1u) { ... FUN_0043e480(object, way, sample); ...
  FUN_0043e400(object, way, (uint8_t)(sample - 1u)); ... return 1; }` / `FUN_0043e480(
  object, way, sample); (void)WP_fnh_Way_PrevWayPoint(object, way); return continuation;`
  (`0043e460` = getter of `+0x13` = ps).
- 00518040 PrevWayPoint: `if ((*(char *)((int)param_2 + 10) == '\x01') && (*(char *)((int)
  param_2 + 9) == '\0')) { cVar1 = (char)param_2[2]; *(undefined1 *)((int)param_2 + 10) =
  0xff; } else { cVar1 = *(char *)((int)param_2 + 9); ... + 10) = cVar1; }
  *(char *)((int)param_2 + 9) = cVar1 + -1;` then, if q != 0xff: `FUN_0043e400(..,0)`,
  `uVar3 = *(param_2[1] + p*4)`, s = `LastSampleIndex == 0 ? 0 : LastSampleIndex - 1`,
  and `if ((cVar1 == '\0') && (p != '\0')) { uVar3 = *(param_2[1] + (byte)(p-1)*4);
  uVar2 = WP_fnuc_Link_GetLastSampleIndex(uVar3); FUN_0043e400(..,uVar2); ...}`.
- 00518e60: `if (bVar2 == 0xff) return links[bVar1-1]; if (bVar2 < bVar1) return links[bVar2];
  if (cVar3 == '\0') return links[bVar1-1]; return links[bVar1];` with `bVar1=p, bVar2=q,
  cVar3=ps`. 005181e0 (src): direct point when `q == 0xff` or `p == 0 && s == 0`.
- 00526c90: types 2/3 return the sampling-rate helper (`n`), other types AL 0.
- 005344a0 (ghidra-recovered): `case 0x44: if (local_4 == 8) { local_10[0] = ftol(); }
  uVar3 = FUN_0043e370(param_1,local_10[0]); *param_3 = uVar3; if (uVar3 != 0) { *param_3
  = (uint)*(byte *)(uVar3 + 8); return uVar2; } break;`; `0x40,0x43,0x41,0x42`:
  `*param_3 = (uint)(local_10[0] != 0);`; `default: *param_3 = 0;`. `FUN_0052e740(1/0)`
  just stores the byte into global 007238b8 around the operand evaluation.

## Not determinable statically

- What 007238b8 does (set to 1 only while evaluating the condition operand).
- How the condition consumer treats a result greater than 1 (count byte); if it compares
  equal to 1 instead of non-zero, ways with more than one point would read false.
- Whether any shipped script runs 29/31/221/222 on a way with curved links (the multi-curve
  back behaviour above is read from the code, not from data).
- Initial p/s/q/ps of a freshly loaded actor before any 220/221/222 (the state node is
  created elsewhere; not read here).

## Addendum 2026-11-04: what the port implements

- 220 ReInitWay: unchanged (`source_reset_route`). It also cancels a running
  route action on that slot, which the source does not do; this is older port
  behavior and is recorded here, not changed.
- 221 ReInitWayBack: `source_reset_route_back`, p = N-1, s = rate of
  links[N-2] (the route Resource now carries `link_rates`), q = ps = none.
- 222 InitWayWithWp: `source_init_route_at_waypoint`; first i < N-1 matching
  the waypoint, s = i == 0 ? 0 : rate of links[i-1]; no match changes nothing.
  The source compares waypoint pointers. The port matches a route-point
  waypoint (as returned by Func_CloserWP) by its index, and any other by its
  exact source position.
- 1-68 Cond_IsValidWay: `source_route_waypoint_count`, the point count or 0.
- Neither 221 nor 222 stops a running action, as in 00542ff0.
- Not ported: the back-walk metas 29 GoOverWayBack and 31
  GoToNextWPOfWayBack, and 28 GoOverWay3D. None of the 15 converted levels
  checked in output/full-game uses them (4-222 and 1-68 appear 10 times each),
  so nothing could check a port. The original's back walk over several curved
  links is incoherent (see Answer 2), so porting it needs a decision first:
  the exact machine, or a clean reverse walk of each link.
