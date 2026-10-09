# Manor ambient/channel composition and game handoff — 2026-10-09

Kip authorized integrating the two isolated corrections after clean main
`681b53ea74fbc44bec107be5fe5176e367a92753`, preserving the mana-pool correction
`dd8dd291561097320092c70dbd389979e3177f3c`. No builds, tests, lints, formatter
checks, regeneration, gameplay, original-game execution, reference changes or
remote pushes occurred during the initial static composition. Kip subsequently
approved the one converter build and Manor-only publication below, followed by
the recovered native camera fix. Worker files remain unchanged. No new gameplay,
test or lint suite was run for this handoff.

## Integrated source and reconciliation

| Contribution | Original commit | Main commit |
| --- | --- | --- |
| Absent logical-channel visibility | `b9c351f8f29051cfaa653f742232e71950cc50c3` | `54f5d16b6cf6e751b0190d05bc1b48d786bf99e4` |
| Actor-sector condition and effects volume | `01afe20f802a890135a307dca4f4ac1b34b045ce` | `2645698f72ab02ad96329ad79fa25e8becf4bf16` |
| Camera sector publication and movement maintenance | `f495b34f51f217eaee6fa07ccecd4b53877c92c4` | `7cf38563cb761ea45a8f7a0fca78c0ceed23b0e3` |
| Recovered final-child camera branch | `dd545dfcfc7ca51283b469aea570771782033b74` | `2bf67a565d3771c5c2261779424d66b62440e967` |

All four cherry-picks preserve original provenance. Their shared Character, AI,
model and support changes combined without conflicts. Static comparison retains
the mana subtraction's point-readiness guard and the channel component's
byte-narrowed actor-owned state and native animation tracks.

Rete62 is synchronized with both explicit schema assertions. Support83 is
synchronized across the exporter, version.json, README and staging assertion.
Both channel scripts and UID sidecars remain in the support inventory and
the existing staging assertion's file list. The sole complete non-default
`SceneNodeProperties` fixture in `tests/hud_quarantine.rs` now supplies
`actor_sector: None`; other fixtures use the updated Default implementation.
AI codec constructors use Default for the added sector binding maps.

The main compiler additionally limits native sector identity equality to
object operands. A captured null uses the separately computed raw-zero/null
payload condition; integers and float32 payload bits are never compared
directly to a native Node3D. Capture still precedes evaluation of the second
operand. The approved converter build and original-source publication exercise
this lowering; gameplay behavior remains unaccepted.

The original condition/channel contract was reviewed at independent original-C
commit `93d7d9efcf859469338bd95e35d3eb0ae26e919c`; Proc386 was reviewed at
`0147d41994738cac2f05fe3a8a8d980358f21729`. Existing original sound-request,
fade-admission and updater bodies corroborate receiver/operand order, full-word
overrides, effects-only termination and exact127 reset. Native timing and driver
gain fidelity remain unverified. See the two isolated notes for exact evidence
and boundaries: [channel](manor-channel-visibility-static-2026-10-09.md) and
[ambient](ambient-sector-volume-2026-10-09.md).

## Sector lifecycle and remaining source boundaries

The imported current-sector holder for the ambient operand `world/0/2`
(StdCamer, not Hype) is known null. The original ambient handoff's human label
was incorrect; the worker corrects it in follow-up
`f495b34f51f217eaee6fa07ccecd4b53877c92c4`. Hype is `world/0/0`.
Condition124's `source_current_sector()` is deliberately read-only. It must not
assign a containing sector while answering a query.

The approved follow-up connects camera0051c7b0 publication to the Character's
holder before the selected translation is written. It reuses the existing
ordered point resolver and source-bound last-sibling fallback. Camera history
publishes changed sectors when a known nonnull holder exists. Post-walk/mass
motion now runs existing directional contact-sector maintenance. The private
camera seed does not replace imported null, and ready-time activation does not
guess containment. Existing viewport-selection refresh also publishes the
camera owner's holder when its selection branch runs.

Read-only original source identifies the relevant boundaries:
`src/actor/actor_activation.c`'s00437740 invokes00447bf0 before activation
publication;00447bf0 researches the actor's world position and uses the last
ordered sector-root sibling when research returns no sector. Ordinary actor
update004499b0 invokes00448280, which applies004e03f0 directional history after
animation/motion. That history routine assumes a valid starting sector and
does not initialize a null holder. The query00535460 does neither operation.
Original-source commit `b94813c3d6b508aa53f7427b0f9844ecb89a539d` subsequently
recovered00447900 and00448350. The latter returns the full final sector-root
child pointer. Camera0051c7b0 captures it before researching the third position;
a no-hit or final-child result selects the second point, another nonnull hit
selects the third. The integrated follow-on compares native node identity,
removes the caller's prequery/third sector-ID argument and incidental entry
collision-history mutation, and refreshes membership before writing the chosen
pose. The actual saved Manor scene already binds `source_sector_fallback_path`
to `world/1/28` (Univers), so no generated contract changed. Original addresses
remain reader/research evidence; the game uses the named native reference.

The recovered startup callback's stable placement states are nonmember+mode2,
member+mode4, or mode0/1, with repeated calls and original short-circuit order.
The port does not yet map those startup snapshot modes and membership lists.
No unconditional ready-time seed is added. Known null passes membership
readiness and compares false against the four nonnull ambient literals until
an explicit publication updates it. Startup membership and ordering therefore
remain fidelity uncertainty; recovering the body does not accept the opening.

The camera's existing incomplete point-resolver, missing sector/fallback
binding and broader unmapped camera-motion branches can separately return
false and stop its compiled Brain. Static review does not establish that such
a guard is reached in this scene. Those guards are distinct from the two
startup fidelity boundary.
Generated-object inheritance of the new known-membership flag/path remains
outside this placed ambient-actor slice. No guessed startup assignment,
query-time containment or new shared resolver is added. The source composition
is synchronized. Compilation/publication succeeded below; the later native
follow-on and mechanically migrated camera fixture were not executed.

## Completed fresh-content publication

From clean `d1293abbbfaeb7f938f9b912f49f62a5ca270ee9`, the approved operations
were `cargo build --release --locked --offline --bin astrolabe` and
`target/release/astrolabe port disc godot --level manoir --locale en --content scene --godot /usr/bin/godot`.
The build exited0 in72.54s; the supported conversion/native import/publication
exited0 in433.46s. Both stayed within explicit300s/1200s budgets and ended with
no timeout, leaked children or remaining processes. Source stayed unchanged
through conversion. No additional suite, gameplay, original-game run, full
conversion or packaging-only rebuild followed.

The actual root manifest is schema2/support83 on pinned Godot
`4.7.2.stable.arch_linux.ed1daf0bf`, SHA256
`abd12f59e5b3e0983d38a2531f18ad6e8ef2c5171e4efba5a6ce4980d684cb26`.
Its previous support82 SHA256 was
`34db7a792f935976e859ea57a2477adc151f3ac0958e4a5595673d93846bc7a0`.
There are9,181 unique files: Manor8,855, inventory213, menu88, pause-menu27,
with3,369 preserved diagnostics. Memberships, file identities and generated UID
identities are preserved; no files were added or removed. The113 changed files
are11 AI scripts,41 native animation libraries,60 Perso/template scenes and
the saved native Manor root scene. All9,181 actual file hashes matched the
published manifest. All retained UI file hashes are unchanged.

The only Manor files shared with those UI scopes are
`textures/FixTex/Inventor/Objects/Usable/plastiks_nz.gf.mips.tres` and
`textures/FixTex/MOTORTXT/lettrageb.gf.mips.tres` under `generated/hype/`.
Both hashes are unchanged. Protected native saves, settings, imported original
slot and `project.godot` are unchanged. The current useful model is Rete62;
this direct original-source port did not serialize a new Rete snapshot.

The maintained camera follow-on was applied only after publication finished.
Actual root `godot/addons/astrolabe/camera/source_camera.gd` now has the corrected
branch; its script path/UID, existing fallback NodePath and support83 contract
are unchanged. The generated native scene references this maintained external
script. The converter binary remains the d1293ab build; its embedded support
predates this native-only follow-on, so it was not rebuilt merely for packaging.
The affected camera fixture now uses a native Character holder and final-child
binding, with real destination geometry and the two-argument method. Its source
migration is committed without a test execution.

Local evidence is under `output/ambient-channel-publication-20261009/`:
`result.json`, both manifest snapshots, `generated-ai.diff`, build/port logs,
and `remaining-checkout-status.json`. Generated assets and game-derived evidence
remain ignored and local.

## Normal launch and remaining limits

Launch the actual prepared game normally:

```sh
godot --path /home/stephen/Projects/astrolabe/godot
```

Its entry remains `res://game/bootstrap.tscn`; both New Game and Load Game use
the saved native Manor content. This handoff leaves no owned game/build process
running. Preparation succeeds; the full opening and subsequent gameplay have
not been accepted. The prior New Game acceptance failed before the completed
intro, physical handoff, Load Game, HUD, inventory or barrel checks. Those
observations are not superseded by publication. Startup membership mapping,
source sound scheduling/gain, broader camera geometry/motion and generated
actor membership inheritance retain their documented limits.

Read-only status audited61 registered/preserved checkout locations. Five old
archival paths are missing and remain unpruned. No current approved worker has
uncommitted changes. Preserved `new-game-load-game` staging is already integrated
or superseded, and the `session-recovery-20261008` note correction is already
reconciled on main; neither requires replay or a new game commit. Six other
dirty checkout directories contain retained temporary/validation/evidence
files. Worker state remains untouched; the final main handoff commit owns all
changes required here.

Local Git fetches transferred the worker commit objects but reported an existing
invalid Codex checkpoint ref during connectivity checking. Cherry-picks from
the available objects succeeded. No checkpoint refs or worker state were removed.
