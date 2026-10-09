# Native payload banks and actual loading — 2026-10-09

This is a bounded Godot ownership correction on branch
`sol-loading-native-20261009`, based on `12ad58ce`. Kip approved the shared
export/configuration/rendering edits. This work follows shape-sharing commit
`eb1bf63` and requires shield behavior from `699bfb3` (locally cherry-picked as
`67b8daf`). No Rust build, native import, content regeneration, Godot or game
launch ran here. Luna owns all native execution and timings through integration.

## Implemented ownership

`src/godot/character.rs` resolves each nonempty original ObjectList slot once,
then exports a complete native payload PackedScene containing its model instance,
action reference, scale, physical-contact area, transforms, material identity
and shape nodes. Templates are shared across actors, channels and family rigs
using the same source configuration within the selected level. Their paths use
the complete configuration-URI digest, normal/reset mode and original slot under
`characters/configuration_payloads/`. The existing staged manifest owns every
output. Shape geometry uses the independently reviewable sharing correction.

An ordinary immutable `configured_bank_catalog.gd` Resource references those
PackedScenes; null entries mean canonical Absent slots. It holds no source
geometry, sampled animation, opcodes or source memory/layout tables. The mutable
level-owned `configuration.gd` permutation remains unchanged. No Model or Rete
schema change is needed; proposed Godot support interface is85. Integration may
compose a later interface number while keeping source/receipt parity.

Each channel retains the existing named normal/reset banks and all stable
`part_<slot>` / `reset_part_<slot>` anchors. The saved actor retains every
initially selected normal payload whose slot is sampled in any family state,
including currently hidden anchors. Initial editor inspection and ordinary
native seeking therefore still use saved scene content. Other payloads are
native scene resources, without dormant node trees in the actor. Unique-template
and initially selected-instance budgets are independently bounded at65,536.

`configured_bank.gd` owns only channel-local warm instances. Configuration binding
validates source identity, mode, slot count, absent entries and cached identities.
It acquires a requested nonempty slot synchronously and rejects inconsistent
native content explicitly. Acquisition adds the full native scene to the hidden,
disabled, top-level bank. Root bank NodePaths are assigned per actor/channel;
scene resources never retain a particular actor's binding. Warm swaps and
selection changes reuse these instances. There is no eager-bank compatibility
fallback, asynchronous missing-payload window or runtime asset reconstruction.

Before attachment, native tree entry applies the current level's material variants
through the existing synchronous `node_added` service. Actor rendering then
applies cached display visibility and adds newly encountered meshes to its actor
opacity cache. This works after fading began with an empty selection, after
opacity returned to255, and for already warm payloads. Whole-mesh overrides are
registered once. The shield shader/cutout restoration and ordinary material
pipeline metadata are retained; no second opacity implementation is introduced.

`configured_slot.gd::attach_payload()` suspends participation, reparents with the
native local transform intact, reapplies anchor transparency, then publishes the
contact transform before restoring process inheritance. Hidden channels/actors
and parked banks keep REMOVE-mode contacts outside physics. Independent ZDE/ZDR
zones, sector indexes and native BVH caches are unchanged. Camera motion-history
hunks and save/startup selection/barriers were not edited.

## Preserved source behavior

Current selection and initial-table restoration remain distinct. A null current
selection empties visuals without resetting gameplay or changing the library.
Initial-table restore mutates that shared table without selecting it. Arbitrary
one-based, u16-narrowed swaps move the entire original payload identity, including
its native placement/scale/action. Matching actors adopt one mutable permutation
from the active level. A reset mode change uses the existing equivalent-time,
cue-suppressed native library seek. Payload acquisition itself never advances or
seeks an animation. Animation track addresses and channel identities are unchanged.
The existing detached-actor binding guard/re-entry behavior is retained.

The decompilation's `research/hype-ai-api.tsv` names procedures82/83, while
`research/coverage.tsv` marks dispatcher00541130 as missing from its raw export.
The source body is therefore not inferred from that inventory. Read-only
disassembly of the supplied patched executable rechecks00461860: selected table
getter00442530, u16 narrowing and five-word entry copies. Getters00442530/00442550
read Data3d+16/+12; setter00442540 writes+16. Executable SHA256 remains
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`.
Local disassembly receipts are `source-slot-swap.txt` and
`source-configuration-getters.txt` under the evidence directory. See the earlier
[source contract](configured-part-swap-source-2026-10-04.md) for dispatcher/reset
evidence and Raymap corroboration. Its earlier eager node strategy is superseded
by this ownership correction, not its source semantics.

## Structural loading target

The static inspector expands native scene instances and inherited overrides.
The projection removes only complete `bank_*/payload_*` subtrees, retaining banks,
anchors, initially selected content and unrelated hierarchy/components.

| Existing saved content | Expanded paths | Omitted parked payload paths | Projected initial paths | Retained banks |
| --- | ---: | ---: | ---: | ---: |
| Manor entry |344,932|333,339|11,593|625|
| Hype perso entry |109,784|108,210|1,574|124|

These are initial saved-structure projections, not engine node counts or regenerated
results. Startup scripts can acquire additional selected payloads, and warm
caches grow with actual configuration use. The projection identifies1,278
distinct parked bank/slot keys for Manor and572 for Hype; source URI/mode identity
is verified by export. This excludes templates whose payload was already selected
in every observed channel and is not a promised final template file count.

Existing parked node headers/properties account for86,712,240 bytes in Manor's
entry file and170,623,471 bytes across its instantiated scene dependencies.
Hype accounts for28,819,205 bytes. These exclude shape/resource sections and
include inherited overrides. Shared template/catalogue files add unmeasured
bytes, so these are removed serialized sections, not net output-size savings.
All referenced native templates are still loaded as Resources: lazy node
construction reduces repeated scene data and dormant instances, not the need to
load required gameplay assets. Measure that dependency cost after regeneration.

Receipts: `output/loading-static-20261009/manor-bank-projection.json` and
`hype-bank-projection.json`. Reproduce the Manor census without Godot:

```sh
python3 tools/scene_loading_census.py /home/stephen/Projects/astrolabe/godot res://generated/hype/levels/manoir/level.tscn --output output/loading-static-20261009/manor-bank-projection.json
```

Luna's current **pre-change** baseline is
`output/runtime-validation/luna-20261009/LUNA-BASELINE.md`: Load Game resource
progress reached99.999% at50.4s, then inventory restoration failed at56.95s
session time; the observer never reached ready. New Game became ready at78.977s,
with resource-ready at53.599s and instantiation at55.611s. Its later opening
observation ended at the outer watchdog and is not a completed opening/gameplay
result. These measurements precede shape sharing and this bank correction. No
elapsed-time improvement, frame-rate change or gameplay acceptance is claimed.

## Review and validation handoff

- Three synthetic Python census checks and six selection checks pass. They
  execute no engine. The projection checks preserve root/override accounting,
  bank roots, unique template keys, source section bytes and source nonmutation.
- Changed-file rustfmt and whitespace checks pass. Whole-repository formatting
  has the already reported untouched baseline failures. No compiler/lint,
  native syntax, engine test, full suite or runtime safeguard is claimed passed.
- The native exported-animation check now requires initial saved content before
  ready, empty dormant banks, shared catalogues, acquisition of an unsampled
  original slot, warm reuse, payload scale, fixed paths, reset timing and no cue
  replay. It updates stale anchor-scale expectations to the payload boundary.
- The new bounded `configured_bank_native` check uses the maintained production
  bank/slot/actor/material services. It covers late acquisition after fading an
  empty selection, per-actor materials with shared geometry, diffuse/cutouts,
  display hiding, channel/contact activation, null/Absent selection, shared
  permutation and initial/current reset distinction. It is written, not run.
- Integration should first compile the affected Rust targets and run support
  staging/native syntax and lifecycle checks in its single slot, then regenerate
  Manor only. Luna should inspect initial node counts/catalogue/resource identity
  and compare actual Load/New phase timings on that frozen composition. Include
  warm repeated swaps, reset, inactive channels, contact and shield transitions.
  Startup/save failure handling remains with its owner. Do not infer improved
  loading from the static projection or accept a skipped restoration barrier.
