# Live checks: PrevComportIntell (field 7) and DSG type 13 (2026-11-04)

These are Orrery live-memory checks of two AI-parser assumptions added in
828d6bb. They ran on a private Orrery MCP server (port 18791) over the
disposable prefix copy `astrolabe-review-20261101/orig/local`. The installed
original and pid 1881 were not touched. The manor save was loaded through
the menu, then `navigate_level brigand` was used. The raw results are in
kip-zephyr `cache/scratch/live/` (`field7_live.json`, `type13_live.json`,
`type13_live2.json`).

## Field 7 PrevComportIntell: confirmed

Orrery's shared reader resolves intelligence +0x08 (current), +0x0c
(previous) and +0x14 (initial) as members of the actor's behavior set, so the
+0x0c word is a live behavior pointer. Field 7's reader 00536000 returns that
word.

Across a behavior change, Hype's normal behavior read as follows:

| Moment | Current | Previous |
| --- | --- | --- |
| After the load | 4 | 91 (the entry sequence is 16, 91, 4) |
| Up held for 1.5 s, then released | 5 | 4 |
| After 1.5 s more | 4 | 30 (5, then the 30 stop, then 4) |

So +0x0c holds the behavior left on each real change, as 005216e0 writes it.
The port's `previous_normal` and `previous_reflex` follow the same rule.

## DSG type 13: confirmed, including non-null values

- Live designer definitions (model +8, 16-byte rows of offset, type, save and
  initialize) give type 13 for model 927 fields 45 and 46 (the brigand guard)
  and model 1060 field 51 (all four thieves), matching the converter.
- In brigand all five words stayed null: no assigning AI ran.
  - The thieves are inactive (StandardGame +0x1c, the byte 00437740 sets, is
    0).
  - The guard is active but about 400 units from Hype, and its per-pass
    fields (50, 20) were never written.
  - Hop placement (`behavior_compare.py`'s method) failed there. The floor
    under a far target is not active, Hype fell, and the level restarted
    (phase 38). Pinning his position also failed: z kept sinking.
- A sweep of the 14 other levels with type-13 assigners entered each level
  from the manor save with `navigate_level` (no placement or driving). It
  waited 15 s, then read every assigner (raw result
  `cache/scratch/live/type13_sweep.log`).
  - 22 actors in 9 levels had run their AI.
  - Every non-null word was an entry of that actor's own normal behavior set.

| Level (model) | Actors with a value | Field: normal entry |
| --- | --- | --- |
| brigand4 (493) | 2 | 46: 8, 47: 10 |
| fort3 (804) | 4 | 46: 2 |
| fort4 (624) | 3 | 46: 2 |
| labo (1122) | 2 | 46: 5, 47: 6 |
| manoir4 (266) | 1 | 46: 2 |
| monast3 (379) | 2 | 46: 2; 46: 8, 47: 10 |
| toras4 (937) | 2 | 46: 5, 47: 6; 46: 2 |
| village4 (334) | 2 | 46: 2; 46: 8, 47: 10 |

- The guard models share one plan:
  - normal 1 and 4 store a category-32 literal into field 46;
  - normal 7 stores two literals into fields 46 and 47, 12 bytes apart;
  - normal 9 stores two literals into fields 46 and 47, 24 bytes apart.

  In model 493 the literals are 0x0900e8a8 (normal 1), 0x0900e8cc and
  0x0900e8d8 (normal 7), and 0x0900e8f0 and 0x0900e908 (normal 9). With
  12-byte entries these are entries 2, 5 and 6, and 8 and 10 of one array,
  the live values above. The brigand4 guards' previous behavior was 9.
- The category-32 leaf 005221c0 gives value tag 35 (0x23), the type's value
  kind in the descriptor table 0x005fdcc0. The writer copies the word.

So a type-13 value is a behavior pointer into the model's set. The port stores
it as the behavior's index.
