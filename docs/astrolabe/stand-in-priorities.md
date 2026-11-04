# Stand-in priorities

A to-do list for static work: porting the original's behavior in place of each
[stand-in](stand-ins.md) and runtime guard. It comes from
[strict stand-in collect mode](testing.md#strict-stand-in-mode) runs over the
Manoir to Brigand play paths (2026-11-03, at 8dc992b), merged with the
documented stand-ins, [unsupported AI entries](../../notes/research/unsupported-ai-entries.md),
[undocumented adaptations](../../notes/research/undocumented-adaptations.md) and
[byte-coverage gaps](../../notes/research/byte-coverage-gaps.md). Decomp
pointers are into `~/Projects/hype-decomp` (read-only):
`research/hype-ai-api.tsv` maps an AI call to its entry address,
`research/coverage.tsv` maps an address to its readable `src/` file, and raw
Ghidra is in `reference/ghidra/functions/<addr>_FUN_<addr>.c`.

## Runs

All runs were headless, with `ASTROLABE_STRICT_STAND_INS=collect`. Raw reports
are kept locally in `TestResults/stand-in-collect-20261103/`, which is ignored
by git.

| Run | Path | Result |
| --- | --- | --- |
| gate (`save_exit.json`) | original save, Manoir to the exit, Brigand loads | reached Brigand, 37 s |
| ctl (`controls_probe.gd`) | original save; turn, walk, run, jump, sword | finished, 25 s |
| tb (`to_brigand.json`) | original save to Brigand, entry cutscene, save | finished, 109 s |
| bl (`brigand_load.json`) | travel save from tb, 7 s in Brigand | finished, 20 s |
| tm (take `stephen-manoir-route-20261102`) | New Game, the take replayed by `follow` | stuck at take time 476 s, still in Manoir |
| tB (take `stephen-linear-brigand-20261102`) | New Game, the take replayed by `follow` | stuck at take time 339 s, still in Manoir |

Both take runs were held at the same spot, (-76.4, 33.1) in state 33, until
the 1200 s timeout. That is a `follow` failure in the route bot, not a slow
machine. **Brigand play past the first seconds was not exercised**: the
fights, beehives, monastery sign, the toras load and return, and the race.
Brigand sites below come only from the entry, its cutscene and the travel
reload. Every list here is a lower bound for Brigand.

## Totals

- 40 distinct sites reached, 18 distinct ids, 25 (id, model) pairs and 14150
  hits. Of these, 13 are guard sites with 54 hits.
- Three sites account for 13736 hits: the squirrels' untyped scripts (1862,
  816) and the thieves' Ultra call (1060).
- Of the documented stand-ins, 9 were reached: `load-placement`,
  `hud-count-offset`, `card-less hold`, `item point contact`, `travel-save`,
  `drop-shadow`, `speech-gain`, `kill-without-reset` and `4-271`. The other 21
  documented ids were never hit (see below).
- The New Game take runs never reached the squirrel (816) or pigeon (779)
  sites. The original-save runs reach them within a second of control. Why the
  New Game path doesn't is unexamined.

First hits are counted from process start and include about 8 to 12 s of
loading. In the original-save runs, the HUD, shadows and load-placement come
at load. The squirrel follows at about 8.6 s, then the pigeons, and the
Brigand-entry sites at about 36 s.

## Priority list

Ordered by what blocks or visibly changes play on the Manoir to Brigand path
first (how early, how often, how visible), then by how little static research
each needs and how many sites one fix covers.

| # | Item (id, models) | Seen | What the original does, or what is unknown | Where to look |
| --- | --- | --- | --- | --- |
| 1 | **Operand ported 2026-11-04; movement blocked by item 3.** Squirrel scripts (816 Manoir, 1862 Brigand) | were 12238 hits; now a guard at the same rate, squirrels still idle | Node 0's way operand is designer variable 10 of DSG type 9, Way: one byte indexing the actor's own way list, the same index a kind-23 operand gives. The importer, decoder and codegen now take it, and both scripts type whole. Both squirrel routes use link connection type 2 (a sampled curve), which the port doesn't sample, so `source_begin_route` refuses them. After that, 5-25 Action_GoToWP (node 23) remains | [way-designer-operand](../../notes/research/way-designer-operand.md): 005236a0, 00523860, 00542ff0, 0043e370; type-2 links 00526cc0 → FUN_00530cb0 / FUN_00530cc0 |
| 2 | **Ported 2026-11-04.** Thieves' Ultra call on MainActor (1060 nodes 118 and 9) | was 1498 hits in 7 s of Brigand | It was MainActor.Func_DistanceToPerso(dsg 11): the 3D distance from Hype to the actor in the thief's own designer variable 11, read with the owner as caller. The (3, 4) decoder now takes a designer actor operand, so the range test uses the real distance (probe: 145.7 m false, 4.0 m true). 6 of the 8 Ultra sites cleared; the two left (1398 node 60, 51 node 17) are another Ultra operation | [distance-to-perso-designer-actor](../../notes/research/distance-to-perso-designer-actor.md): 00537c20, 00525950, 00525920, 00521e00 |
| 3 | Boar patrol (now also blocks the squirrels, item 1), guard "selected waypoint needs unsupported link geometry" (309; the point-motion guard has 31 sites across 309, 1060, 927, 1203, 210, 365, 991, fix/28) | 29 hits in Brigand; boars don't walk their way | Only StaticLine links are sampled (`character.gd:647`). The original samples Bezier and CircleArc curves too | decomp `src/waypoint/curve_sampling_rate.c` (00531240, 00531e10), `link_sampling.c`, `waypoint_progress.c`. One fix covers every model that walks a curved way |
| 4 | `speech-gain` | every dialogue line, from the intro (23 s) | The speech event's own volume, not 0.7 times resource gain; the volume source is unknown | decomp `src/sound/event_dispatch.c`, `ai_object_volume.c`; UA-27 |
| 5 | `drop-shadow` (kinds 2, 3, 4) | every shadowed actor, at load | Fake_Sh textures; kind 2 projected, kinds 3 and 4 light-placed | No decomp pointer is recorded yet; find the Fake_Sh users |
| 6 | `hud-count-offset` | HUD, at load, always on screen | FON 3D projection of the weapon count | FUN_0040ca40 (`0040ca40_FUN_0040ca40.c`, untouched); BCG-19:0 FON text objects |
| 7 | Thief and guard behavior requests with DSG operands: guard "opaque source designer field 46/51/52" (17 sites) and `generic-untyped` "non-behavior operand" (30 sites); 1060, 927 | 18 hits in 7 s of Brigand, on enemies | Comport request from a DSG variable (kind 11: vars 45, 46, 51) or PrevComportIntell (kind 9) | [UAE 4-209](../../notes/research/unsupported-ai-entries.md): 005426f0, 005216e0, 00536000 / 00536030 |
| 8 | **Ported 2026-11-04**, with item 2. Pigeon distance, Func_DistanceToPerso of a designer actor variable (779 dsg 7 and 8; 816, 1862, fix/28) | was 80 hits in Manoir | Same fix: all 14 sites now compute the distance; the variable belongs to the script owner | same note |
| 9 | `kill-without-reset` | 33 hits: barrels in Manoir, Brigand entry | The revival component reset and the activation flags; matters when Brigand respawns its mobs on re-entry | stand-ins.md row; decomp actor reset (`src/actor/`) |
| 10 | `item point contact` and `card-less hold` | 64 and 15 hits: barrel drops; Fire in training | What DNM collides with when there's no ZDM; that card-less objects get no dynamics | [magic-mannequin-chain](../../notes/research/magic-mannequin-chain.md); decomp `src/dynamics/`, `src/collision/` |
| 11 | Camera position update guard (fix/437; 15 sites across 437, 535, 839, 1951) | 7 hits in Manoir, at about 4 min | The native camera position update that 437's rule 0 requires | `source_camera.gd`; UAE camera block (00547200 to 00547980) |
| 12 | Brigand HUD: `generic-4-356` Proc_GPI_SetTextNumber (284 IWorld) and `generic-4-357` Proc_GPI_SlideGamePlayInterface (2058 nest dummy) | at Brigand entry; HUD counters and beehive HUD | Set a HUD text item to a number; slide the gameplay interface | 0053ef50 (case table 0x53f300), 00447300; UAE 4-356 |
| 13 | Brigand AI one-offs: `generic-4-53` Proc_SetHitPointsToInit (309, 1203); `generic-4-296` Proc_CapsSetCapabilities (309); `generic-1-68` Cond_IsValidWay (1060, 927); `generic-5-25` Action_GoToWP and `generic-4-215` Proc_Turn2 (544 door lady) | 1 to 4 hits each, at Brigand entry | as named | 00540930 (`src/ai/life_damage_procedures.c`), 00545910, 005344a0, 0054b110 / 00522a50 (UAE 5-25), 00542bb0 |
| 14 | `travel-save` | Brigand save and reload | Per-level records and level history; Brigand re-entry respawns mobs in the original | stand-ins.md row; decomp `src/snapshot/` |
| 15 | `4-271` Map_vSetDestination (2026 Gogoud map) | once, in the entry cutscene | Records the map destination | 00545cc0 |
| 16 | `load-placement` | original-save loads only (tests, not New Game) | How the engine places Hype from the saved matrix | UA-26; `src/godot/save_import.rs` |

### Never hit on these paths

Documented stand-ins that none of the six runs reached. Most belong to
features the paths didn't use, or to Brigand play that wasn't reached:

- Inventory: `1-118`, `1-119`, `3-197`, `3-199`.
- Menus and map: `4-334`, `4-374`, `4-336`, `4-272`.
- Magic: `4-438`.
- Camera: `4-154`, `4-150`, `4-152`, `4-151`, `4-175`, `4-148`, `4-146`, `4-140`, `3-75`. Older
  stand-ins.md text called the 437 camera requests among the most reached. On
  these paths only 437's position-update guard (item 11) fired.
- Bindings: `unbound-waypoint`, which has no compiled site in either level.
- Training: `generator-contact`. The take runs stopped before the crossbow bolt
  hit anything, or never fired it; which is unchecked.
- Levels: `micro-basis`.
- UAE ids `generic-3-220`, `generic-3-153` and `generic-1-30`, all Brigand-only
  (models 927, 1060, 396, 1203, 374).

### Silent, so collect mode can't see them

These adaptations change play but don't report. From
[undocumented-adaptations.md](../../notes/research/undocumented-adaptations.md),
the ones visible on this path are:

- UA-22: particles never emit. Stephen noticed Gogoud's smoke.
- UA-4: death ends the run.
- UA-13: crossbow aiming.
- UA-14: dialogue box admission.
- UA-18: camera fail-closed paths.
- UA-8 to UA-11: floor, step and collision mapping.
- UA-23 and UA-24: lights and fog.
- UA-33: the wind no-op.

Making these report through `StandInReport` would put them in this list.

### Doc corrections found

- UA-1 quotes a guard text, "source control-status semantics are unresolved",
  that no longer appears in the generated code.
- UA-35 says `hud-count-offset` has no stand-ins.md row, but the row exists.

## Sites reached

Runs: gate = save_exit, ctl = controls, tb = to_brigand, bl = brigand_load,
tm and tB = the takes. First hit is the earliest across runs, from process
start.

| Hits | Id | Model | Site | Runs | First hit | Context |
| ---: | --- | --- | --- | --- | --- | --- |
| 9456 | `generic-untyped` | level:/1862 | `brigand/ai/models/1862.gd:45 normal_0_schedule` | bl,gate,tb | bl 12.3 s | source AI node 0: source AI script could not be typed: route call lacks local slot operand at node 0 |
| 2782 | `generic-untyped` | level:/816 | `manoir/ai/models/816.gd:45 normal_0_schedule` | ctl,gate,tb | gate 8.6 s | source AI node 0: source AI script could not be typed: route call lacks local slot operand at node 0 |
| 1498 | `generic-untyped` | level:/1060 | `brigand/ai/models/1060.gd:4761 normal_14_rule_1` | bl | bl 12.8 s | source AI node 118 (receiver MainActor): Ultra operation is unsupported for this receiver |
| 98 | `drop-shadow` | - | `rendering/drop_shadow.gd:31 _ready` | bl,ctl,gate,tB,tm,tb | gate 8.4 s | kind 2 drawn as a gradient blob, not Fake_Sh |
| 74 | `generic-untyped` | level:/779 | `manoir/ai/models/779.gd:224 normal_2_rule_0` | ctl,gate,tb | gate 11.6 s | source AI node 32: distance requires an unsupported actor expression |
| 64 | `item point contact` | - | `character/character.gd:1269 source_execute_action_and_generate_object` | ctl,tB,tm | ctl 18.2 s | 0.1 sphere added to an item without a movement zone |
| 33 | `kill-without-reset` | - | `character/character.gd:858 source_kill_perso` | gate,tB,tm,tb | gate 36.0 s | killed without the source component reset |
| 21 | `speech-gain` | - | `audio/emitter.gd:352 play_voice` | tB,tm,tb | tm 23.5 s | dialogue speech at 0.7 times resource gain |
| 15 | `card-less hold` | - | `character/character.gd:1262 source_execute_action_and_generate_object` | ctl,tB,tm | ctl 16.3 s | card-less generated object held at its spawn transform |
| 13 | `guard` | level:/309 | `brigand/ai/models/309.gd:273 normal_0_schedule` | bl,tb | bl 17.2 s | requires supported native point motion: selected waypoint needs unsupported link geometry |
| 13 | `guard` | level:/309 | `brigand/ai/models/309.gd:283 normal_0_schedule` | bl,tb | bl 17.2 s | selected waypoint needs unsupported link geometry |
| 8 | `drop-shadow` | - | `rendering/drop_shadow.gd:31 _ready` | bl,ctl,gate,tB,tm,tb | gate 8.3 s | kind 4 drawn as a gradient blob, not Fake_Sh |
| 8 | `hud-count-offset` | - | `ui/hud_slots.gd:50 _ready` | bl,ctl,gate,tB,tm,tb | gate 8.3 s | weapon count at a measured offset, FON 3D projection not ported |
| 7 | `guard` | fix:/437 | `manoir/ai/models/fix/437.gd:156 normal_0_rule_0` | tB,tm | tm 265.1 s | requires native camera position update |
| 6 | `generic-untyped` | level:/779 | `manoir/ai/models/779.gd:163 normal_1_rule_1` | ctl,gate,tb | tb 9.0 s | source AI node 10: distance requires an unsupported actor expression |
| 5 | `drop-shadow` | - | `rendering/drop_shadow.gd:31 _ready` | ctl,gate,tB,tm,tb | gate 8.4 s | kind 3 drawn as a gradient blob, not Fake_Sh |
| 4 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:226 normal_0_schedule` | bl | bl 12.4 s | accesses opaque source designer field 46 |
| 4 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:228 normal_0_schedule` | bl | bl 12.4 s | accesses opaque source designer field 46 |
| 4 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:230 normal_0_schedule` | bl | bl 12.4 s | accesses opaque source designer field 46 |
| 4 | `generic-4-53` | level:/309 | `brigand/ai/models/309.gd:529 normal_2_schedule` | gate,tb | gate 36.0 s | source AI node 10: unsupported AI statement category 4 operation 53 at node 10 |
| 4 | `generic-4-296` | level:/309 | `brigand/ai/models/309.gd:539 normal_2_schedule` | gate,tb | gate 36.0 s | source AI node 30: unsupported AI statement category 4 operation 296 at node 30 |
| 3 | `load-placement` | - | `game_session.gd:372 open_level` | ctl,gate,tb | gate 8.5 s | Hype placed at the live-measured manor spot after an original load |
| 3 | `generic-1-68` | level:/1060 | `brigand/ai/models/1060.gd:145 normal_0_schedule` | bl | bl 12.3 s | source AI node 39: unsupported AI expression category 1 operation 68 at node 39 |
| 2 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:1628 normal_4_schedule` | bl | bl 12.4 s | accesses opaque source designer field 51 |
| 2 | `generic-4-53` | level:/1203 | `brigand/ai/models/1203.gd:398 normal_4_schedule` | tb | tb 41.9 s | source AI node 12: unsupported AI statement category 4 operation 53 at node 12 |
| 2 | `generic-4-357` | level:/2058 | `brigand/ai/models/2058.gd:35 normal_0_schedule` | gate,tb | gate 36.1 s | source AI node 3: unsupported AI statement category 4 operation 357 at node 3 |
| 2 | `generic-4-356` | level:/284 | `brigand/ai/models/284.gd:383 normal_1_schedule` | gate,tb | gate 36.0 s | source AI node 10: unsupported AI statement category 4 operation 356 at node 10 |
| 2 | `guard` | level:/309 | `brigand/ai/models/309.gd:283 normal_0_schedule` | bl,tb | bl 12.3 s | next waypoint initialization needs unsupported link geometry |
| 2 | `generic-1-68` | level:/927 | `brigand/ai/models/927.gd:137 normal_0_schedule` | gate,tb | gate 36.0 s | source AI node 60: unsupported AI expression category 1 operation 68 at node 60 |
| 1 | `travel-save` | - | `game_session.gd:216 request_initial_level` | bl | bl 0.5 s | a travel save loads its level fresh, without per-level world records |
| 1 | `travel-save` | - | `save_state.gd:364 write_travel` | tb | tb 104.5 s | saving outside the manor writes one travel slot, not the source record table |
| 1 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:2922 normal_7_schedule` | bl | bl 12.4 s | accesses opaque source designer field 51 |
| 1 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:2924 normal_7_schedule` | bl | bl 12.4 s | accesses opaque source designer field 52 |
| 1 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:3401 normal_9_schedule` | bl | bl 12.5 s | accesses opaque source designer field 51 |
| 1 | `guard` | level:/1060 | `brigand/ai/models/1060.gd:3403 normal_9_schedule` | bl | bl 12.5 s | accesses opaque source designer field 52 |
| 1 | `4-271` | level:/2026 | `brigand/ai/models/2026.gd:267 normal_2_schedule` | tb | tb 64.5 s | source AI node 0: unsupported AI statement category 4 operation 271 at node 0 |
| 1 | `generic-4-357` | level:/2058 | `brigand/ai/models/2058.gd:49 normal_0_schedule` | tb | tb 39.9 s | source AI node 7: unsupported AI statement category 4 operation 357 at node 7 |
| 1 | `guard` | level:/309 | `brigand/ai/models/309.gd:273 normal_0_schedule` | bl | bl 12.3 s | requires supported native point motion: next waypoint initialization needs unsupported link geometry |
| 1 | `generic-4-215` | level:/544 | `brigand/ai/models/544.gd:104 normal_0_schedule` | bl | bl 13.0 s | source AI node 30: unsupported AI statement category 4 operation 215 at node 30 |
| 1 | `generic-5-25` | level:/544 | `brigand/ai/models/544.gd:67 normal_0_schedule` | bl | bl 13.0 s | source AI node 11: unsupported AI statement category 5 operation 25 at node 11 |
