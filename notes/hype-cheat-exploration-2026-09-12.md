# Hype’s hidden cheat lever

Investigated and tested on 2026-09-12 in the separate Hype harness. This historical report preserves the original scope and limits of those observations; the maintained code catalog is in [the OpenSpace docs](../docs/openspace/cheat-codes.md).

Paths beginning with `local/` below refer to the external harness at `/home/stephen/Projects/hype-harness/`; those private game-derived observations are not part of this repository.

## Activation and what it unlocks

The English PC game contains `iLevierSecretManoir`, a normally invisible lever in Gogoud’s Manor I (`manoir`). Its original engine-space position is **(-38.073284, 152.783936, 17.162954)**. While playing, type **`debug`**, then press **Space** beside the lever. The script permits activation for **2 seconds**, and requires Hype to be **less than 1.5 game units** from it. Typing `debug` again retries an expired window.

The lever sets `iDummy_PourCercles.DSG[8]`. That flag gates **38 input bindings** in the cheat controller: **`tele01`–`tele36`, `hunter`, and `geritole`**. `debug` itself only raises the temporary reveal flag, `DSG[7]`; it does not directly enable the cheats. The two seconds limit activation, not the lifetime of the unlocked cheats. The unlock persisted across the tested level changes. Persistence across saving, restarting, or loading another save was not tested.

The keyboard-only MCP recipe in `/home/stephen/Projects/hype-harness/examples/activate-hidden-lever.json` assumes Hype is already beside the original lever. It does not move the player, bypass the gate, or verify proximity. The natural walking route to that location has not been reproduced in this investigation.

A first-person [2009 forum post](https://www.jeuxvideo.com/forums/1-2443-8625375-2-0-1-0-defi-hype-sans-epee.htm) mentions `debug` at Gogoud’s manor near the drawbridge. That was a research lead; the timing, distance, flags, and unlocked commands above come from the game’s own loaded AI and live tests.

## Live test method

The supplied save was loaded into the private harness installation. Before activation, `tele01`, `hunter`, and `geritole` registered as inputs, but the map and their flags remained unchanged. A distant `debug` attempt expired without enabling cheats.

Hype’s collision system rejected simple long-distance position edits: local/global render-matrix writes were overwritten; editing the mechanics position only nudged him against collision. To isolate the lever’s real activation logic, the test temporarily moved the lever’s local translation to Hype, typed `debug`, and pressed Space. The game itself changed the unlock flag **0 → 1**. No cheat gate or projectile flag was written by the harness. The lever’s original translation was restored and verified once gameplay resumed.

Space also opened the nearby save lectern, so the first follow-up test ran in the save menu and correctly failed to travel. Escape dismissed it without saving; subsequent gameplay tests worked. This is recorded as a test-fixture side effect, not a feature of the secret lever.

After activation, `hunter` changed its flag **0 → 1 → 0**, and `geritole` independently changed its flag **0 → 1 → 0**. `tele01` then performed an actual map transition to Torras. All 36 teleport destinations were subsequently reached in the same process, with state observations and screenshots; details and qualifications follow.

## Teleport capability and limits

These tests establish level loading and a rendered scene, not completion of each level’s quests or correct story progression. The starting fixture’s story/inventory state remains relevant. Thirty-five raw target names matched directly. `tele29` contains `cachots4~` in the script, while the engine reports the normalized name `cachots4`; the exact-string probe timed out, but its state and screenshot confirm arrival.

The first attempt to leave `ciel1` using `tele08` did nothing. The same code worked from the manor. The harness therefore used its explicitly logged **memory level transition** to recover after flight destinations; those recoveries are not counted as cheat successes. A later `tele05` attempt from `cachots4` was not recognized; the harness recovered to the manor again. These observations mean the codes are not a universal escape route from every runtime state.

`tele23` loads `casino3`, but its captured camera was clipped into Hype’s head. Its player and camera spawn strings contain trailing `~`. `tele24` loads `village3`, although its script names a camera spawn belonging to `labo3`. Successful map loading alone does not validate those authored spawn references.

The automated probe workloads took **49.739 s** and **183.274 s**, including the failed waits and flight recoveries. This was exploratory testing with a manual recovery between runs, not a single end-to-end regression suite. Each workload had a 480-second overall deadline.

| Code | Reported map | Result |
| --- | --- | --- |
| `tele01` | `toras` | Loaded |
| `tele02` | `fort` | Loaded |
| `tele03` | `cachots` | Loaded |
| `tele04` | `monast1` | Loaded |
| `tele05` | `manoir` | Loaded |
| `tele06` | `brigand` | Loaded |
| `tele07` | `ciel1` | Loaded; next teleport ignored in flight |
| `tele08` | `toras2` | Loaded |
| `tele09` | `monast2` | Loaded |
| `tele10` | `labo` | Loaded |
| `tele11` | `astrolabe` | Loaded Astrolabe; screenshot inspected |
| `tele12` | `casino` | Loaded |
| `tele13` | `manoir2` | Loaded |
| `tele14` | `brigand2` | Loaded |
| `tele15` | `village2` | Loaded |
| `tele16` | `ciel2` | Loaded; harness recovery after flight |
| `tele17` | `toras3` | Loaded |
| `tele18` | `fort3` | Loaded |
| `tele19` | `monast3` | Loaded |
| `tele20` | `labo3` | Loaded |
| `tele21` | `manoir3` | Loaded |
| `tele22` | `tour3` | Loaded |
| `tele23` | `casino3` | Loaded; camera clipped into Hype |
| `tele24` | `village3` | Loaded; questionable camera spawn name |
| `tele25` | `ciel3` | Loaded; harness recovery after flight |
| `tele26` | `cite3` | Loaded |
| `tele27` | `toras4` | Loaded |
| `tele28` | `fort4` | Loaded |
| `tele29` | `cachots4` | Loaded as cachots4; later tele05 not recognized |
| `tele30` | `manoir4` | Loaded |
| `tele31` | `tour` | Loaded |
| `tele32` | `toptower` | Loaded |
| `tele33` | `brigand4` | Loaded |
| `tele34` | `village4` | Loaded |
| `tele35` | `ciel4` | Loaded; harness recovery after flight |
| `tele36` | `cite4` | Loaded |


## Projectile controls

`hunter` is internally named `Button_Cheat_HomingBalls`, but its fireball script does more specifically describe **manual steering**: the middle Fire Trinity ball takes the camera, and Left/Right turn it. It is not evidence of automatic enemy homing. `geritole` enables **Space** to toggle that controlled ball’s pause state; the paused branch sets its vector speed to zero. The control script belongs to `fl03 / mMagieFeu_Boule`.

The first run’s cross-level state would not draw a weapon, so this test was repeated from a fresh load of the unchanged supplied save. The lever was activated again through keyboard input; after its interaction animation settled, its temporary position was restored. No projectile state was patched. Run: `local/runs/20260912-153301`, PID 253608, start identity 382815.

The inventory displayed **Focus Magicum: Trinity**. `thereyougo` supplied the spell and 250 mana; the spell was equipped with the inventory keyboard controls. With `hunter` and `geritole` enabled, casting cost **35 mana**: 250 → 215, then 215 → 180 on the next cast. A 600 ms key gap was too late to pause before nearby collision; a second cast using Return held 80 ms with a 300 ms gap, immediately followed by Space held 60 ms with an 80 ms gap, successfully left one ball suspended.

The source-identified active-ball count (Hype DSG[122]) fell from 2 to **1**, remained 1 across the next tool call and an additional 1.5-second wait, then became **0** after Space resumed the ball and it collided. Left and Right each held for 500 ms changed the camera/ball heading while that one ball remained alive. Screenshots show the suspended fiery ball and the changed view. This establishes steering and pause/resume, rather than merely accepting the cheat text. A fully independent per-ball position/velocity trace was not collected.

These timings are evidence from the private 48-FPS fixture, not a universal casting delay. Allow room ahead of Hype: a ball that has already collided cannot be paused. The flag toggles themselves were already verified in both directions.

## Other discoveries

The complete [cheat-code catalog](../docs/openspace/cheat-codes.md) and `/home/stephen/Projects/hype-harness/cheats.json` catalogs preserve all **272 text-code bindings** found in the engine’s 497-action input table, including internal inventory and quest codes. **57** had matching typed AI input references in the loaded manor/Fix snapshot. The remaining **215** are labeled binding-only: they may be native handlers, level-dependent, or abandoned development inputs. They must not be advertised as 215 working cheats.

Useful additional script findings:

- **`oint`** copies a large set of current global/quest values plus Hype’s position and facing into a checkpoint actor’s variables. **`revient`** restores those copied variables, then calls the player-death procedure. This suggests a developer checkpoint/reload pair, not an arbitrary level selector. Its complete round trip is not live-tested here.
- **Left Shift + Left Ctrl + Delete** is an additional non-text shortcut in that same script: hold SpeedUp and Jump, then press MagicScroll. It sets many global quest flags to 1. These keys were decoded from the current keyboard bindings. The bulk flag changes are script-confirmed, not live-tested.
- **`pouletfrit`** has an inventory add/remove branch for the Galarneau advertisement (object 44).
- **`frik`** calls the inventory gold adder with a random integer between 0 and 100; it is not a fixed 100-plastyks grant.
- **`grolot`** adds a specific set of seven inventory objects; its name must not be expanded into a claim that it grants every quest item.
- **`thereyougo`**, **`hermetik`**, **`houdini`**, **`tunnel`**, **`littletroll`**, **`littlegogoud`**, **`youngelf`**, **`oldelf`**, **`protek`**, **`druidik`**, **`leben`**, **`toutundefi`**, and **`kutkut`** also have AI references in this snapshot. The catalog preserves the action names. Existing harness research separately verifies `druidik` refilling mana and `thereyougo` raising maximum mana to 250.
- The large `ite…` and `gl…` families, `epok1`–`epok4`, `supered`, `allpowers`, `fps`, and several tempting debug names are present as bindings but have no AI reference in this snapshot. Their names alone are not enough to promise the advertised effect.

## Reproduction and evidence

- Executable SHA-256: `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
- Save archive SHA-256: `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
- Run: `local/runs/20260912-150332`, game PID 46196, process start identity 205973.
- Wine-GE 8-26, 32-bit prefix, dgVoodoo 2.79.3, DXVK 2.6.2, videos disabled. The load used the supplied English save, not New Game.
- MCP transport: persistent authenticated Streamable HTTP server. This existing Codex session used a Python MCP client because the newly installed tools were not added to its active tool catalog. All live input/memory/screenshots went through MCP and the owned controller.
- Raw observations: ignored `local/hidden-button-research/mcp-events.jsonl`, `teleport-probe.json`, `teleport-probe-continued.json`, `bindings.json`, memory snapshots and decoded script reports. Controller events are also recorded in the run directory.
- All four supplied `SaveGame0` files were byte-for-byte equal to the archive after the teleport and projectile tests. The engine does write its separate working `HYPE` cache during level changes; that is not a saved-slot overwrite.

Addresses below are **observations from this process**, not stable offsets for another session. Resolve and validate objects again before any future memory experiment.

| Item | Observed address |
| --- | --- |
| Lever Perso | `0x45f9a78` |
| Lever local translation | `0x45f9ae4` |
| Lever global translation | `0x3dbdad4` |
| Cheat controller current DSG buffer | `0x2888b20` |
| Lever-unlock flag | `0x2888b24` |
| Temporary debug/reveal flag | `0x2888b25` |
| Global projectile pause-enable flag | `0x2841287` |
| Global controlled-fireball flag | `0x2841288` |
| Active Fire Trinity ball count (Hype DSG[122], fresh run) | `0x2843c54` |

Primary evidence: manor activation script at `0x33a1b1c` (2,000 ms timeout and 1.5-unit distance), cheat gate at `0x288a4d4`, debug input at `0x288af2c`, and fireball control at `0x28921a8`. Raymap’s Hype node tables and Montreal object readers guided decoding, corroborated with the local engine decompilation. Some Raymap native-function labels differ from this build; the report avoids relying on ambiguous magic-function labels for effects. The engine matrix getter/setter functions at `0x466190`, `0x4661b0`, `0x4bb350`, and `0x4bb390` corroborate the position representation.
