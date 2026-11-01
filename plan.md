# Astrolabe plan

**Make a tool to repeatably port this OpenSpace game to Godot.**

This replaces the parallel roadmap of 2026-09-27. The earlier plan, including
its milestone detail and format matrix, is in
[notes/historical/plan-2026-10.md](./notes/historical/plan-2026-10.md).

## Direction

**Carve out the major systems and placeholders first, then fill them out,
approaching game parity.**

Every major system gets a real place in the running game early, even if what
fills it is a guess. Then the guesses are replaced in order of what they block.
The game runs throughout, so problems show up when they are made.

Not everything can match the original on a different engine, and things that
merely don't quite match can wait. The big things must work: the frame rate
holds, the AI scripts fire, and the game can be played from start to finish.

Four working rules follow:

1. **Run it.** Work is merged only after the [gate](./docs/astrolabe/testing.md#the-merge-gate)
   passes. Long stretches of static-only work are how Cond123 stopped the player
   without anyone seeing. A check of real source behavior was one memory scan
   away.
2. **Keep the gate cheap.** A full behavior suite once took so long and ran so
   constantly that progress stalled. The gate has three checks and a ten-minute
   cap, and runs at merge points only. A new check replaces an old one. Broader
   evidence lives in `tools/test.py`, which is run when it answers a question.
3. **Guess in the open.** An unsupported call gets a
   [stand-in](./docs/astrolabe/stand-ins.md): a documented guess at what it
   should do, tracked until game-accurate behavior replaces it. Silent no-ops
   are not allowed. Tests stay strict.
4. **Ask the original.** The game's data and live memory are very close to each
   other, and Orrery reads the running game through Astrolabe's own parser. When
   a new parser or behavior assumption matters, check it against live memory or
   a live run before relying on it. "If you don't know how a bit is used, you're
   just guessing."

## What stays and what waits

These stay: the Rust converter, its OpenSpace, Rete and Godot adapters around
one model, native Godot output, compiled AI (never an interpreter or a rebuilt
OpenSpace runtime), and honest scope in reports.

These wait, because none of them makes the game more of a game yet:

| Effort | Status |
| --- | --- |
| Byte parity | Keep as a very quick coverage signal, never a gate. It shows which bits we do not understand yet. |
| Rete coverage | Grow only as the game needs it. |
| OpenSpace reconstruction | Deprioritized. |
| Recording and replay infrastructure | Paused. Targeted Orrery state checks replace it for now. |
| The BDD gameplay suite | Parked. Its harness has its own startup path, which no longer matches the game. Port it only if it can stay cheap. |
| Exact visual and timing parity | Later, except where it hides whether something works. |

`hype-decomp` stays parked and is consulted only for a specific blocker.

## Milestones

### 1. Restore the signal: done

The suite builds and selects again, Cond123 is fixed, Load Game survives a jump,
the gate exists, and fatal unsupported calls have stand-ins.

### 2. The manor plays on the machine it runs on

The next milestone. The player can start, fight and move through the manor at a
usable frame rate with a default launch, and Load Game loads a real save.

- **Default launch performance.** A default launch uses the Intel GPU, which
  measured a 16.6 ms median on both routes on 2026-11-01, so the reported
  seconds per frame did not reproduce. Hitches remain: 60 ms at p95 and a worst
  frame of 847 ms. Find them, and retest in a real window under the editor.
- **Load actually loads.** Load Game must read the savegame file. Today it reads
  a fixture and a small JSON made ahead of time by `astrolabe import-save`. It
  should accept a real `SaveGameN` slot, with inventory, position, progression
  and world state, and report what it can't restore.
- **Save.** New Game cannot save yet, and the native slot records too little.
- **Controls.** Add crossbow and Fire to the controls probe.
- **Failing tests.** Triage the failures in `tools/test.py --local`: fix, rewrite
  or delete each one, with a reason. Most look like tests left behind by static
  changes.

### 3. The manor is a level

Gates and keys, Gogoud's progression, damage, enemies, death and the level exit,
through compiled AI and native systems. Replace stand-ins in order of what blocks
play, starting with camera, inventory and regeneration calls.

Compare with the original through targeted Orrery state checks, one per question:
the same key press in both engines, then the action and behavior sequence. Two
small Orrery additions would help most: a live AI-tree dump through the shared
codec, to diff against the disc parse, and a read of particle state.

### 4. A second level

This tests whether Astrolabe is a tool or a manor port. Convert another level,
list what breaks, and carve out the systems it needs. Level transitions belong
here.

### 5. The whole game

Remaining levels, cutscenes, UI flows and the remaining systems, then fidelity:
the discrepancies that matter, visual parity, and timing. Recording and replay
return here if they are worth their cost.

## Working loop

1. Pick the thing that most blocks play.
2. Give its system a real place in the game, with a stand-in if needed.
3. Check any new assumption against the original.
4. Run it, then run the gate at the merge.
5. Record what is guessed and what is known.
