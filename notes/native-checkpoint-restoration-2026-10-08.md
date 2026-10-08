# Native checkpoint restoration — 2026-10-08

Follow-on implementation to the [save fork capability audit](save-fork-capability-audit-2026-10-08.md),
on the isolated Boolean-save branch based on `edb7704`. The fuller fork
`294bd26` supplied useful checkpoint evidence; its old session, inventory,
gate-placement and source-data loading code was not replayed. The integration
lead explicitly assigned inventory/barrel adoption to another worker and life,
armor, checkpoint, identity and scalar validation to this branch.

## Implemented behavior and evidence boundaries

- Life remains owned by the actor's native `hit_points.gd` Resource. Its new
  version-1 snapshot captures, validates and restores independent `current`,
  `maximum` and `limit` bytes. It does not clamp or refill on restore.
- Armor uses the existing version-1 catalogue snapshot API. The slot validates
  the exact ordered definition/piece identities on the live catalogue and
  applies to a private clone before publishing any live change. Restore updates
  the level's shared catalogue in place; another level's owned copy is untouched.
  Selection, resistance, broken count, piece flags/strikes and the native current
  damage-piece ordinal survive a fresh process. Artwork, factors and thresholds
  remain generated native assets.
  The actual owner is the level's `source_armor_catalog` plus
  `source_armor_state` facade, as emitted by `src/godot/scene.rs`. Exported Hype's
  `source_armor` property is unbound at this baseline; copying the fuller fork's
  actor-property read would disconnect the save path. Restore therefore uses
  the level owner and a facade `restore_snapshot` method that synchronizes its
  selected index/selection observers. The fixture leaves the actor property
  unbound and rejects a facade linked to a different catalogue.
- `resolve_player` follows `source_actor_nodes` for
  `level:/persos/world/0/0`, verifies `source_perso`, and rejects external,
  malformed or non-character bindings. Save capture/apply also check the active
  `manoir` context. The actual session's initial player lookup calls this resolver;
  the rest of its loading and transition lifecycle is preserved.
- `write` admits only a living, grounded, still player in supported idle action
  47 / Normal 4 / Reflex 3, with a running Brain and no unsupported failure,
  pending action, external movement, held/generated projectile or held gameplay
  control. If a future native component exposes wind drift, it must also be
  settled. The current movement API explicitly defers wind and has no such
  property, so its absence is accepted. Capture/apply do not require floor
  contact: loading still occurs before physics and Brain startup.
- Shared scalar validation accepts finite integral JSON floats as well as native
  ints, after range checks and before conversion. Booleans, fractions, infinity,
  NaN and huge numbers fail. Item keys must be canonical, bounded strings; `-1`
  remains the separately admitted special-resource sentinel.
- Native placement preserves finite nonsingular scaled/sheared Basis values.
  Schema-1 imports retain optional position-only transforms. Both representations
  are checked in actual native Vector3 precision before live assignment.

Read-only Raymap `Object/Properties/StandardGame.cs` corroborates byte-valued
life. The independently reconstructed hype-decomp
`src/ai/ai_hit_points_query.c` implements the shared Func35..40 dispatch and
low-byte return convention. The checked source registrations at `00438c30`
register the three StandardGame bytes; the `00438ed0` / `00438ef0` setters write
one byte without a relational clamp. These prove field/storage/API behavior,
not a newly mapped Hype-owned original-save record. The original reader's health
and armor import scope is unchanged.

Armor initialization `0043a8a0` registers selection plus each definition's
broken count/rank and each piece's broken/strike bytes. `0043aa70` uses the
current damage-piece choice during damage; its pointer is not registered by
that initialization. Persisting the native **ordinal** is an explicit checkpoint
continuity choice inherited from the useful fork, not a claim of exact original
save encoding. No RNG state or source pointer is saved. The current-piece
ordinal and native Resources are sufficient for the tested next damage call.

The idle checkpoint admission is a declared port policy that bounds unsupported
transient state. It is not an original engine save restriction or arbitrary
coroutine/AI restoration. All byte-valid life combinations remain loadable,
including `current > maximum` and `maximum > limit`; only write admission
requires a living player. The helper's Hype `dsg_11` and lever `dsg_1` stay
unadmitted because their visible restoration effects remain unresolved. The
production regression continues to reject these fields explicitly.

## Exact schema and integration overlap

This isolated contribution writes **schema 4** to the existing native slot.
It reads schema 1 and schema 4. The number deliberately avoids silently reusing
the archived fork's schema 2/3 for a different independently owned inventory
payload. It does not claim compatibility with those fuller archived envelopes.
Inventory/barrel reconciliation belongs to the assigned worker/integration lead.

The schema-4 additions are top-level `checkpoint: "manor-idle-v1"` and player
`asset`, `life` and `armor`. The legacy player key remains `world/0/0/Hype`;
`asset` carries the verified semantic URI. Life is
`{version: 1, current, maximum, limit}`. Armor is the existing
`fix:/gameplay/armor` component snapshot. Player transform/magick/legacy
inventory and supported Global/dummy flags retain their existing payload shapes.
Schema 4 requires complete placement and its supported Global/dummy state;
schema-1 omission preserves seeded life/armor and optional Global behavior.

There are no edits to inventory Resources, placed barrels, the OpenSpace
save reader, shared model or Rete schema. Support interface is **74** in this
branch (main was reported as 73). Shared files to reconcile are
`game/save_state.gd`, one resolver line in `game/demo.gd`,
`character/snapshot_values.gd`, the new restore method in `character/armor_state.gd`,
support version files, the manifest fixture label
and notes index. This branch extends its own `actor_world_save_state` harness
and adds a source-free fixture; it does not overwrite inventory animations or
the other worker's persistence harness.

The pickup contribution `3ad4937074f08545d79e1a0e927b44f524d99cc5` extends
schema 1 with optional `inventory.snapshot`, Global first-pickup flags 220..228,
and per-placed-barrel world entries. Its own publication/restore validation
must precede this patch's live life/armor mutation. Integration must retain
those optional keys and their typed live-target/tombstone checks in both legacy
schema 1 and the combined schema-4 writer. This branch does not copy or merge
that peer code. Its independent harness retains legacy inventory values and
only Global 149/147 plus the supported dummy; the lead's combined tests own
the union. Pickup's new catalogue lookup for compiled damage remains separate
from this facade checkpoint synchronization.

## Validation

Godot 4.7.2 syntax preflight and the packaged Rust process harness use the
maintained Resources and production slot, with separate fresh write/reload
processes, private XDG directories, owned logs and a 25-second outer deadline.
The fixture uses actual native floor collision. It verifies:

1. Semantic identity resolution after renaming the player node, then durable
   life/armor selection and damage values across fresh processes.
2. Shared catalogue ownership, another level's unchanged clone, repeated restore
   and continued native armor damage followed by life subtraction.
3. Atomic rejection of malformed values, missing live components, mismatched
   ordered armor references, wrong actor/level bindings and invalid live Brains.
4. Rejected writes preserve the previous file for unsafe movement, stopped AI,
   pending actions, dead life, held controls and active/held generated objects.
5. Schema-1 omission and transform compatibility, independent byte values,
   finite scaled/sheared native placement, optional Global omission and strict
   rejection of unmapped demonstration fields.

Initial packaged execution passed in 0.62 seconds. Final timing and check
evidence are recorded in ignored
`TestResults/actor-checkpoint-level-owner-final/`: final package build/run
54.61 seconds (including 53.97-second Rust setup), isolated compiled-test
execution 0.615 seconds under its 25-second deadline, and scoped all-target
`godot-harness` Clippy 3.50 seconds. Each supervisor report records no timeout,
leaked child or remaining process. The native syntax/write/reload logs inside
`actor-world-save-state/` contain no engine errors; the synthetic saved slot is
retained there. `cargo fmt --all --check` and `git diff --check` passed.

Three support ownership/preload checks also passed during development (0.01
seconds of test execution, 71.70 seconds including build). Their follow-on
armor-owner correction was verified through the final actual packaged files.
The session script's resolver call passed a separate owned Godot syntax check.
The earlier missing-metadata diagnostics were fixed with explicit presence
checks and are absent from the final package run. No whole save/release matrix
or full original-data export was repeated; aggregate validation belongs to the
integration lead, including existing save harness/schema reconciliation.

No original
game process, original-save mutation, decompilation build/test, push or
copyrighted input commit occurred. These are native component/slot behavior
checks, not a new whole-scene or cross-engine gameplay acceptance claim.

## Main union verification

The approved checkpoint commit is composed after pickup merge `480ce152`. Main
writes schema 4 and reads 1/4 with Rete58/support77, retaining pickup inventory
v2 snapshots, Global220..228 and all four placed-barrel flags/tombstones. Every
live Boolean/model/actor binding is checked before life, armor, inventory or mana
publication. Named local tombstones restore idempotently after node deletion;
malformed/missing bindings remain errors. The armor facade stays bound to the
level-owned catalogue. Main also resolves all pickup artwork texture dependencies
during inventory import and uses the canonical armor-facade compiler assertion.

The maintained production-slot regression now combines admitted/unadmitted
inventory members, capacities, selection, gold, four barrel owners, all nine
first-pickup flags, life and armor through fresh write/reload processes. It tests
legacy sparse inventory and rejects incompatible live barrel fields before any
component mutation. Owned syntax/write/reload preflight passed in 0.708s with no
script errors, timeout or remaining child. Evidence: ignored
`output/integration-resume-20261008/final-feature-main/slot-result.json`. Full
combined selected validation follows this preflight and is reported separately.
