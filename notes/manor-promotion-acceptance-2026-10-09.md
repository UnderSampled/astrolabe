# Manor promotion and actual-game acceptance — 2026-10-09

The user approved promotion of the reviewed Manor batch to main and replacement
of local prepared content, then authorized one actual-game acceptance session.
Kip owns sequencing and acceptance. Promotion and all four preparation scopes
completed. **The single runtime session failed; the opening is not accepted.**
No second session, broad suite, lint, original-game run, reference build, or
remote push followed that failure.

## Source promotion and preserved state

Before promotion, main was clean at
`78a1c861a41c3ec5ba2780ad3eef5e41ee060fc9`. The named branch
`checkpoint/published-manor-78a1c86-20261009` preserves that exact revision.
The reviewed isolated union was clean at
`e6b3008ce1ddaba6b58c6311b93afc272001e48b`; main advanced to it by fast-forward.
The [static integration ledger](manor-static-integration-2026-10-08.md) preserves
the exact original/integrated contribution commits and bounded source findings.

The pre-promotion audit inventoried 55 registered worktrees, including 50 existing
checkouts and five missing archival paths. Existing worker files, uncommitted
changes, ignored evidence, source data, generated assets and reference submodules
were preserved. No worktree was cleaned or pruned.

Destination Godot import created two support-script UID sidecars. Both have
tracked script owners and unique valid identities across maintained sidecars.
Commit `172f1cb69a6c0eff85fd11fcb508230e427f9143` preserves them and registers
them beside their scripts in `src/godot/support.rs`:

- `godot/addons/astrolabe/character/channel_attachment.gd.uid`
- `godot/addons/astrolabe/character/lift_motion.gd.uid`

This metadata commit leaves Rete61 and support interface82 unchanged. It is the
exact clean source revision used for the actual-game acceptance session.

## Completed preparation

The supported `prepare-game disc godot --godot /usr/bin/godot` operation used
the release CLI built with `cargo build --release --locked --offline --bin
astrolabe`. Build and preparation used the existing subprocess supervisor with
owned logs, private XDG/TMP paths, explicit deadlines and descendant cleanup.

| Stage | Result | Elapsed |
| --- | --- | --- |
| Release build before preparation | Exit0 | 73.83s |
| Scene, menu, inventory, pause-menu conversion and grouped publication | Complete, exit0 | 465.51s |
| Release rebuild to embed the two preserved support UIDs | Exit0 | 70.81s |

All three stages report no outer timeout, leaked child process or remaining
process. The final metadata rebuild did not run conversion or replace content.
Preparation initially reported a metadata review requirement only because the
two UID files were added; no implementation source was modified or removed.

The published native manifest has schema2, support82 and engine version
`4.7.2.stable.arch_linux.ed1daf0bf`. It owns 9,181 unique files: Manor8,855,
menu88, inventory213 and pause-menu27; selection memberships can overlap.
There are no remaining preparation scopes. Its SHA-256 is
`883aebd291dafc2b7fb1774c813b194429d28aac663bc75982a1a8461e88b93b`.
The three retained manifest diagnostics concern two pause-menu audio ADPCM
re-encoding fidelity limits and the already documented menu settings/control
mapping. They are not preparation failures.

## Single actual-game session

The supervised command used the actual root `godot/` project and the maintained
`godot/tests/new_game_intro_acceptance.gd` observer with a private observation
subclass. It instantiated the real bootstrap and supplied native mouse input to
its New Game button. The subclass would use the real pause Main Menu action and
Load Game button after a successful passive opening, then inspect the actual
HUD and open inventory through native Tab input. No source state, actor pose,
camera, inventory grant, generated object or AI callback was synthesized.

The session used a private X11 display, software OpenGL compatibility rendering
and private user data. The Dummy audio driver limits any possible audio claim
to native playback state; audible output is unverified. No production FPS
setting was changed during this observation. The existing loading service cap
ran through the production path; the observed FPS limit was restored to0 by
the time failure cleanup executed.

| Native stage | Time from session start |
| --- | --- |
| Resource ready | 53.383s |
| PackedScene instantiated | 55.434s |
| Late opening stage | 101.284s |
| Observer failure report/capture completed | 120.356s |

The observer had already crossed its70-second New Game readiness bound and
returned failure: `actual menu did not reach ready New Game within 70 seconds`.
Long main-thread frames and the failure screenshot's awaited rendering delayed
the report; the late opening stage does not turn this into a readiness pass.
The outer supervisor exited1 after125.814s, with no outer timeout, leaked child
or remaining process. Godot separately warned of44 ObjectDB instances at exit;
process cleanup does not establish resource-lifetime correctness.

The first production error was:

`level:/ai/models/154 requires restored source magick state`

Its stack reaches World154 `normal_2_schedule` at generated script line2004,
through `normal_2`, `_enter_normal`, the maintained Brain's normal step,
`advance` and `start`. Later distinct errors report missing converted channel2
for Level1238 and Fix524, and unsupported condition124 at Level1290 node41.
These are retained errors, not accepted ambient behavior or a proven cause of
the resource-loading delay. The captured frame shows the late opening and its
unsupported-behavior status. No intro dialogue/camera/music chain or physical
handoff was accepted. **Load Game, HUD/inventory acceptance and optional
barrel/drop/pickup observation were not reached.**

## Static diagnosis and smallest proposed repair

The fresh exported scene owns shared magick with native defaults: current0,
maximum0, selected spell-1. Character `_enter_tree` resolves that scene-owned
Resource before child Brains start. New Game deliberately does not apply the
supplied saved-state fixture.

World154's initial Normal2 schedule first initializes inventory/life/armor,
then evaluates `Magic_RemoveMagicPoints(Magic_ReadMagicPoints())`, before
changing the mana maximum to50. The emitted point read correctly uses
`source_magic_points_ready()`. The subtraction guard in `src/godot/ai.rs`
instead uses `source_magic_ready()`, which also requires a positive maximum
and selected spell. The native character subtraction assertion repeats that
stronger guard. Those conditions are absent in this fresh state, so execution
stops before the maximum initialization. The earlier armor guard was passed
in this run; that fact is not full armor acceptance.

The smallest proposed repair is to use `source_magic_points_ready()` for the
subtraction compiler guard and the character method assertion, preserving the
nonnegative amount condition and the selected-spell query's separate guard.
The existing subtraction implementation itself already compares current mana,
returns0 if insufficient, otherwise subtracts/clamps and returns1. This agrees
with the verified handler contract recorded in
[the magick source boundary](magick-comparison-2026-09-24.md); it does not read
the selected spell. No invented starting spell or saved-state injection is
needed. **This proposal was not applied, rebuilt, regenerated or retested.**

The existing first-visit synthetic script tests maximum/addition with spell-1,
but omits subtraction. The native subtraction test sets spell61 and calls the
Resource directly, bypassing the compiler and character guards. Their earlier
passes therefore do not cover this actual fresh-source call chain. No test was
rerun during the static diagnosis.

## Evidence and preservation

Ignored evidence is under `output/integration-promotion-20261009/`:

- `before-promotion.json`, both preserved manifest records, preparation
  `result.json`, `prepare-summary.json`, `release-build.log`, `prepare-game.log`.
- `acceptance/result.json`, `observed.json`, `engine.log`,
  `actual-acceptance.log`, `support-metadata-build.log` and `load_timeout.png`.
- `promotion-outcome.json` joins the exact commits, stage results, timing,
  diagnostics, protected-file hashes, proposed repair and acceptance limits.

The failure image is also saved in Library as `load_timeout.png`, file identity
`libfile_2992a4c5924881918e86c2384e73208a`; its confirmation is retained in
`acceptance/library-capture.json`. Local Library identity metadata was applied
successfully. Capture SHA-256:
`b8e7204e2bda1bf448b4a99d8df1d91a0236799be421957df00703278d40bad7`.

The session's tracked source inventory, project settings, published manifest,
existing user save/settings paths and supplied original-save import were
hash-checked unchanged. The actual user's native slot was not copied into the
private session; the planned Load Game check would use the supplied imported
post-gate save. Its SHA-256 remained
`8d20ef3de198f53c0ca86ba11c3227145496a939e5ff2eaaeba1a2c2a7153c67`.
No game-derived content, screenshot, log or save is included in commits.

## Authorized mana-pool correction after the failed session

Kip subsequently authorized the proposed local fix and the smallest supported
affected regeneration, with no new test or gameplay run. The two subtraction
guards now use `source_magic_points_ready()`. Operand compilation, argument
evaluation placement, nonnegative assertion, subtraction/clamp/return behavior
and selected-spell guards remain as before. This is a compatible guard repair:
no model/schema, native method signature or support-file inventory changes, so
Rete61/support82 remain coordinated.

Before changing code, the complete original function dispatcher at00536550
through its return/jump table and procedure dispatcher at0053de60 through its
return/jump table were read statically from the independent project's original
source-profile PE, SHA-256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
Function230 evaluates its operand before resolving the pool, compares the
requested signed amount against unsigned current mana, subtracts/clamps the
low word on success and publishes a UInt0/1. It never reads selected spell.
The complete selected-spell wrapper00439e30 and inventory-name lookup0041d660
were also read; they form the distinct Function232 selection path.

The static sibling audit covers Function228 current read,229 addition,230
subtraction,231 maximum read,232 spell query and Procedure434 maximum change.
Only subtraction had the pool/spell guard confusion. Current/maximum reads,
addition and maximum change already use point readiness; the pool Resource
allows zero maximum and independent16-bit current/maximum values. Existing
selected-spell readiness requirements are preserved. The shared native pool
algorithms and OpenSpace operation mappings need no change.

The supported `port disc godot --level manoir --locale en --content scene`
route regenerates the complete Manor ownership closure while retaining menu,
inventory and pause-menu. Their only two shared files are non-AI resources;
publication refuses changed shared resources and checks every replacement
against its old hash. The existing scene preparation took326.74s, compared
with465.51s for full application preparation/publication. The fresh scope
still requires one release build to include the changed compiler and native
support; the already preserved UID inventory needs no separate rebuild.
Publication outcome is recorded separately after that operation completes.
This section supersedes the earlier unapplied-proposal status, without changing
the recorded failed acceptance or implying runtime verification.
