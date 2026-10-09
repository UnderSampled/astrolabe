# Manor ambient/channel static composition — 2026-10-09

Kip authorized integrating the two isolated corrections after clean main
`681b53ea74fbc44bec107be5fe5176e367a92753`, preserving the mana-pool correction
`dd8dd291561097320092c70dbd389979e3177f3c`. No builds, tests, lints, formatter
checks, regeneration, gameplay, original-game execution, reference changes or
remote pushes occurred during this composition. Worker files remain unchanged.

## Integrated source and reconciliation

| Contribution | Original commit | Main commit |
| --- | --- | --- |
| Absent logical-channel visibility | `b9c351f8f29051cfaa653f742232e71950cc50c3` | `54f5d16b6cf6e751b0190d05bc1b48d786bf99e4` |
| Actor-sector condition and effects volume | `01afe20f802a890135a307dca4f4ac1b34b045ce` | `2645698f72ab02ad96329ad79fa25e8becf4bf16` |
| Camera sector publication and movement maintenance | `f495b34f51f217eaee6fa07ccecd4b53877c92c4` | `7cf38563cb761ea45a8f7a0fca78c0ceed23b0e3` |

All three cherry-picks preserve original provenance. Their shared Character, AI,
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
operand. This correction has not been executed or syntax-checked.

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
Two original-source boundaries remain. Startup holder event00447900 has no
reconstructed body, so its initial membership effects and ordering are unknown.
The incomplete decompiled00448350 return leaves the camera's second/third
position branch comparison unverified; this patch preserves the existing
native branch policy. Neither boundary is an explicit native unsupported call
or direct fail-stop. A known null passes membership readiness and compares
false against the four nonnull ambient literals until a supported publication
updates it. Missing startup effects or a different branch result could still
alter behavior silently; that is fidelity uncertainty, not an accepted opening.

The camera's existing incomplete point-resolver, missing sector/fallback
binding and broader unmapped camera-motion branches can separately return
false and stop its compiled Brain. Static review does not establish that such
a guard is reached in this scene. Those guards are distinct from the two
unreconstructed source boundaries. Kip owns the parallel source investigation.
Generated-object inheritance of the new known-membership flag/path remains
outside this placed ambient-actor slice. No guessed startup assignment,
query-time containment or new shared resolver is added. The source composition
is synchronized; compilation, publication and runtime behavior remain unverified.

## Smallest proposed fresh-content update

The smallest proposed update is one fresh release converter build and the
supported command
`astrolabe port disc godot --level manoir --locale en --content scene` suffice
for the currently published ownership graph. Import must start from original
source; existing Rete61 snapshots do not contain the new sector semantics.
The source checkout owns support through Git, so an addon installer must not
overwrite it. All source support literals now agree on83.

The unchanged published manifest remains schema2/support82, SHA256
`34db7a792f935976e859ea57a2477adc151f3ac0958e4a5595673d93846bc7a0`.
Manor owns8,855 files; inventory213, menu88 and pause-menu27 own no character or
AI resources. The only Manor files shared with those scopes are
`textures/FixTex/Inventor/Objects/Usable/plastiks_nz.gf.mips.tres` and
`textures/FixTex/MOTORTXT/lettrageb.gf.mips.tres` under `generated/hype/`.
Neither correction changes texture conversion. The ordinary publisher checks
those hashes and refuses changed shared resources while other scopes depend
on them. It also checks every owned replacement and preserves authored files,
settings and unrelated scopes. This is a static scope proposal, not a successful
build/publication or a new acceptance result.

Local Git fetches transferred the worker commit objects but reported an existing
invalid Codex checkpoint ref during connectivity checking. Cherry-picks from
the available objects succeeded. No checkpoint refs or worker state were removed.
