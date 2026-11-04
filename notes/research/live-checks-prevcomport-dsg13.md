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

## DSG type 13: field types confirmed; pointer values not yet observed

- Live designer definitions (model +8, 16-byte rows of offset, type, save and
  initialize) give type 13 for model 927 fields 45 and 46 (the brigand guard)
  and model 1060 field 51 (all four thieves), matching the converter.
- All five actors' type-13 words were 0 (null). They stayed null after 20 s
  more.
  - Every actor was still in normal 0. The only assignment found, model
    927's normal 1 schedule, stores a behavior literal (a category-32 node,
    which the port does not yet accept in expression position) and had not
    run.
  - The original runs AI only near the player. Getting Hype there would mean
    driving him (bot work, out of scope).
- Still static for the non-null case:
  - the descriptor table 0x005fdcc0 gives type 13 value kind 0x23
    (comport);
  - the type's writer copies the word;
  - the assignment's value is a behavior literal.

  A live non-null value needs an assigning actor running near Hype.
