# Route bot state (2026-11-04, paused for an architecture redesign)

The closed-loop follow bot (`godot/tests/route_bot.gd` `follow` step, routes from
`tools/take_route.py`) drives the port along Stephen's take
`stephen-linear-brigand-20261102` from New Game toward the Manoir exit
(about 322.6 s take time). It has not yet reached the exit transition cleanly in
any run; training completion is the blocker.

## What works

- Traversal and pacing: arc-length pursuit of the take's 2 Hz track, a gait
  ladder (run, walk, stand), turn hysteresis, a forward-only path cursor,
  re-anchoring at level loads, `from` for mid-take starts, the fixed physics
  clock under Movie Maker.
- Modal handling: dialogue/inventory/menu gating, a reverse gate for Space,
  and (fixed in e4e2a92) linger taps that advance a port dialogue the take has
  already left.
- Actions: presses wait one frame with no steering key held (Hype's movement
  rules precede his action rules, so a press with an arrow held was lost).
  Idle action spots (the take's last sample before the press, normal 4) are
  approached to 0.5 m and 0.05 rad, aiming through the take's aim point 3 m
  ahead.
- Inventory selection by item identity (`--select`); presses dropped once a
  training stage counted (`--skip-if`), since an extra hit after success sends
  Gogoud down the wrong-hit path and deadlocks the stage.
- Every clock hold is attributed (dialogue-space, modal, spot, action,
  steer-release, level-exit) and bounded: over 8 s it logs `IDLE` and is
  released; `HOLDS` prints totals. Logs: `WATCH` (Gogoud, all three
  mannequins, Hype stage flags, open dialogue), `NORMAL`, `KEY`, `SPOT`,
  `SKIP`, `SELECT`, `BOLT` (per-tick crossbow bolts), `MOTION`.
- Runs: `astrolabe-session-20261102/bot/run.sh` (headless gamescope) and
  `vrun.sh` (visible window on Stephen's session, real audio, ASTROLABE_PERF=1,
  persistent shader cache).

## What doesn't, and why

- **Training stages are flaky.** The zones leave 5 to 10 cm of margin. From the
  take's own spots, its first fire cast hits the mannequin body, and casts 2 and
  3 hit arms. Head and body zones are tested before the arms, so a line near the
  body is a wobble, not a count (notes/research/training-completion.md).
  - Copying the take's spots and facings (to 0.5 m and 0.05 rad) passed only some
    runs. Walking from a stand moves Hype about half a metre, so position
    cannot be refined further by walking.
  - The crossbow is frame-phase sensitive. Bolts move about 0.6 m per frame;
    identical aim sometimes enters the arm and the body zones on the same frame,
    a wobble (BOLT trace; notes/research/crossbow-bolt-race.md).
- **Aim-at inside follow is half-done (uncommitted, see below).** The aim targets
  are the spot/face pairs from `tools/routes/training_orb.json` and
  `stage2_magic.json`, walked to and turned with keys, not teleported or
  snapped.
  - Sword: passed first try.
  - Crossbow: from (-46.07, 136.95) at the arm centres, it did not count
    (pitch or spawn offset not checked).
  - Magic: counted, but out of order because the crossbow stage had not
    completed.
  - Aiming at the near sword arm's centre (instead of training_orb's
    face point 0.45 m outside it) swung across the body.
- **Barrels.** The take's swings at 133.1 and 134.1 s landed in a port dialogue.
  The linger bug let `gogood_texte2` run 9.9 s against the take's 7.0 s, and
  Gogoud then added `gogood_texte2a` (134.7 to 138.4 s). The bug is fixed but
  the fix is unverified on the barrels.
- **Saves wrote nothing.** `save_game()` (game_session.gd:972) returns silently
  unless the session is past `opening_finished` and Hype passes
  `checkpoint_error`. The 158.3 s attempt reported "Save after movement and
  action transitions have settled." (external velocity or a pending action
  state), though Hype was in normal 4 at 0.02 m/s. A native save does restore
  Hype's transform, inventory, magic and Gogoud's exercise stage. Not pursued
  further, by direction.
- **Idle waits.**
  - The teleport-past-the-bridge iteration (training_orb's opening) no
    longer triggers Gogoud's intro. Hype stood in normal 48 through
    `until_normal 28` (20 s) and `until_normal 4` (70 s), then aborted.
  - In the follow, the 6 to 12 s hold caps added up: full runs took 780 to
    1090 s wall for about 308 s of take. Holds are now attributed and
    bounded, but the totals have not been measured on a full run.
- **Motion.** Best run: 20 port starts and 19 stops against the take's 11 and 10;
  16 spurious starts or stops; 104 degrees per second RMS heading jitter going
  straight; 51 turn-key toggles per minute. The walk-not-stop goal is not met.

## Open findings

- The take records only Hype at 2 Hz: transform, action and frame, normal and
  reflex behavior. It has no sword beam or zone, no bolt or Fire contact, and
  no other actor's state or dsg flags. So hits can't be attributed from it.
  Per-frame candidates for a recording:
  - Hype's transform, action and frame;
  - the sword object's (fix/708) zone transform during swings;
  - bolt (733) and Fire (817) positions and their contact or kill frame;
  - each mannequin's dsg_0, dsg_1 and dsg_2;
  - barrel state;
  - Hype's dsg_17, dsg_55, dsg_58 and dsg_0.
- Fire's hold in place is the documented stand-in `card-less hold`; asset
  evidence agrees, but the decomp does not yet confirm it.
- Strict stand-in mode stops the gate at hud-count-offset, then
  load-placement, then squirrel model 816 (`generic-untyped`) within a second.

## Uncommitted

- `godot/tests/route_bot.gd`: the aim-at hand-off (`_aim_spot`, `aim_at` plan
  rows with [x, y, face_x, face_y] spots, a 0.3 m tolerance, and the approach
  that starts up to 3 s before a press), plus `AIM` logging.
- `tools/take_route.py`: `--aim TIME:PERSO:X,Y,FX,FY;...`.
- Session scripts: `bot/make_route.sh`, which builds `--aim` and `--skip-if`
  for the training stages.
