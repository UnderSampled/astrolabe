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

Both cherry-picks preserve original provenance. Their shared Character, AI,
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

## Remaining actor-sector lifecycle boundary

The imported current-sector holder for Hype is known null. Condition124's
`source_current_sector()` is deliberately read-only. It must not assign a
containing sector while answering a query.

The maintained `_ready()` eventually invokes `source_set_object_active_state`,
but that method currently changes activity without a sector placement query.
The movement paths call `move_and_slide` and camera-history maintenance without
an actor-sector update. `source_contact_sector()` only traverses portals when
there is already a bound current sector, so the saved null cannot be repaired
by that method. Camera selection's explicit refresh updates the camera owner,
not Hype. Generated actors inherit the caller's cached sector but do not yet
copy the new known-membership flag or bind its native path. These are connected
integration gaps, not accepted condition behavior after startup.

Read-only original source identifies the relevant boundaries:
`src/actor/actor_activation.c`'s00437740 invokes00447bf0 before activation
publication;00447bf0 researches the actor's world position and uses the last
ordered sector-root sibling when research returns no sector. Ordinary actor
update004499b0 invokes00448280, which applies004e03f0 directional history after
animation/motion. That history routine assumes a valid starting sector and
does not initialize a null holder. The query00535460 does neither operation.
The delegated ambient worker's follow-up lifecycle evidence remains with Kip;
no guessed sector, query-time containment, or new shared resolver is added here.
Static composition is therefore not yet sufficient to claim fresh gameplay
sector behavior, and regeneration remains unrun.

## Smallest proposed fresh-content update

After the lifecycle boundary is reconciled, one fresh release converter build
and the supported command
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
