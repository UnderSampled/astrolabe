# Demo release state (2026-11-05)

A local, unpublished demo of the Manoir and Brigand port. Nothing is tagged
or pushed; the Cargo version stays 0.1.0.

## Pinned state

- **Source:** main `4ce5e35`. The demo was regenerated from this revision.
- **Rete schema:** 75. **Godot support interface:** 101
  (`godot/addons/astrolabe/version.json`).
- **Generated selections:** `manoir`, `menu`, `pause-menu`, `inventory` and
  `brigand`, all fresh.
  - They were regenerated from empty with `target/release/astrolabe
    prepare-game disc godot` and `... port disc godot --level brigand`.
  - The manifest's 21,336 files all match on disk, and nothing is stale or
    unlisted.
- **Loading art:** the source vignette and bitmap bar are unchanged.

## Launch

```sh
cargo build --release --locked
target/release/astrolabe prepare-game disc godot   # only if regenerating
target/release/astrolabe port disc godot --level brigand
godot --path godot          # the game; add --editor for the editor
```

Choose **New Game** for the opening, or **Load Game** for the imported
original save in `godot/game/saves/`. For a pristine profile, run with fresh
user data, for example `XDG_DATA_HOME=$(mktemp -d) godot --path godot`.

## Verified on this build

- **New Game and Load Game** (`godot/tests/new_game_smoke.gd`, on a copy of
  the project with its own user data):
  - the real menu-to-New-Game route reaches play after the Gogoud opening;
  - the Load Game routes pass, including recovery from a corrupt save.
- **Controls** (`godot/tests/controls_probe.gd`, Load Game on the project):
  - loads in under 30 s;
  - turn, walk, run, jump and sword work;
  - an F5 save loads again;
  - the median frame is 16 ms on an RTX 5080 Laptop;
  - no stand-ins are reached.
- **Gameplay checks run before this build, not rerun on it:**
  - Brigand wolves, bats, Maliq and loot pickup;
  - the sector refresh after scripted placement (fe8c133);
  - loot items without a movement zone no longer collide with corpses
    (2c19dbf).
  - See `notes/research/brigand-*.md`.

## Material changes since the baf70b3 snapshot

- **Gameplay and runtime:**
  - controls remapping screen and source bindings;
  - dialogue answers, sounds and text rendering;
  - crossbow reticle;
  - actors placed as shared scene instances;
  - character AI gated by always-active or loaded sector;
  - Brigand wolves, bats, respawn and cutscene cameras;
  - race win through the last markers;
  - scripted-placement sector refresh;
  - loot contact.
- **AI typing (each from its source handler, with live checks where needed):**
  - field reads through ModelCast;
  - camera targets from designer variables and field reads;
  - Func_VectorLocalToGlobal and Func_GetAngleAroundZToPosition;
  - add-assignment into actor fields and vector components;
  - DSG type 19 PersoArray, including object-template identities;
  - Proc_SetHitPointsToInit;
  - capabilities (Proc296–298, Func143/144).

## Known gaps

- **Levels:** all 37 convert (inventory at `5392f91`), but gameplay is
  validated only on the Manoir-to-Brigand path.
  - [full-game-ai-sites.md](../docs/astrolabe/full-game-ai-sites.md) counts
    at `5392f91` and lists the later ports that make it stale.
  - It was not re-counted for this build.
- **Brigand race:** the full 37-torch course and water recovery are
  unverified (`notes/research/brigand-race-chain.md`).
- **Unported:**
  - Zat TurnLeft/TurnRight need the type-19 flight mechanics;
  - graph pathfinding (meta 35) is a stand-in, so capabilities are stored but
    don't steer it.
- **PersoArray limits:** a non-null default that names an unplaced template,
  and actors outside the imported graph, still stop conversion explicitly.
- **Gamepad:** a physical gamepad was not tested; button order follows the
  source index.
- **Other unsupported calls:** these run as documented stand-ins
  ([stand-ins.md](../docs/astrolabe/stand-ins.md)).
- **Checks not run on this build:** the route-bot gate (bot work is paused)
  and the original-engine comparison.

## Closeout housekeeping

- **Backups:** under `output/demo-closeout-20261105/`.
  - `tracked/stephen-project.godot.patch`: Stephen's editor re-save of
    `godot/project.godot`, now removed.
  - `tracked/worker-route-bot-wip.patch`: uncommitted route-bot
    work-in-progress (`godot/tests/route_bot.gd`, `tools/take_route.py`), now
    removed.
  - `generated-before/`: the previous mixed generated output (manifest
    interface 99).
  - `user-data/`: a safety copy of the saves and import fixture.
- **Kept:** saves, settings and other worktrees.

## Publication (user-only)

- Tag, push or publish.
- Decide on a version number.
- Build and ship any exported game. Game-derived generated content must stay
  local, per the README's game-data rules.
