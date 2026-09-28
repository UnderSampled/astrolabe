# Save mapping pause checkpoint — 2026-09-28

The user requested a stop. Resume the comprehensive Hype save-semantic map
from this checkpoint; the goal is paused, not complete. This save task shares
the worktree with a separate source/runtime task. Stage only paths owned by
the task being committed. Preserve `disc/`, the private installation and
isolated runtime evidence.

## Coverage and completed work

The supplied English manor slot has 1,534 record occurrences but 1,271
contextual identities because the 263 named-manor records overlap the player
file. The current generated catalogue owns/types 1,222 identities. It has
508 complete meanings (503 source-verified, five runtime-corroborated) and
763 unknown meanings, of which 398 carry partial roles. These are semantic
counts, not a claim of whole-game coverage. Source descriptors were discovered
for 37 levels; new progression states remain largely unvisited.

The latest save commits before this pause are `5e66361`, `0ed7bf6`,
`23ccf29`, `bb1ec2c`, `c3e6750` and `dde45e0`. They retain the exact owner
joins, actor-local AI index, Hype DSG[11] partial trace and runnable authored
`DefaultMagicPoints` restore probe at
[`tools/probes/run_magic_default_restore.sh`](../tools/probes/run_magic_default_restore.sh).
The reverse owner merge has a focused passing Rust test and a byte-identical
1,271-identity CLI join. All 201 `CustomBitsInit` records have exact source
Perso owners (123 Level, 78 Fix). The actor-local index projects 6,662 typed
sites from 70 AI models onto 94 linked Persos and keeps shared-model actors
separate.

Since those commits, three isolated fresh-process magic-selection trials
restored `0000`, `0100` and `005a` in the live two-byte record
`0:5:4986216`; native selected-spell observations were 0, 1 and 0. The
source-verified low byte is corroborated and the upper marker survives this
load route, but the high byte's gameplay meaning remains unknown. The ignored
[report](../output/savegame-semantic-map/magic-slot-86.md) and
[JSON](../output/savegame-semantic-map/magic-slot-86.json) hold exact fixture,
executable, timing and cleanup evidence. The temporary test was removed.
Another bounded [native dispatch trace](../output/savegame-semantic-map/procedure82-native-85.md)
found no valid callback for procedure 82: a decompiled metadata-table address
lands in string data. It did not justify a new semantic promotion.

The save-specific working-tree updates at pause record those results in
`tools/save_semantics.py`, `docs/openspace/save-games.md`, the progress,
handoff and evidence notes, and this checkpoint. The catalogue and both AI
indexes were regenerated; the counts above are unchanged. Python compilation,
catalogue/index generation and whitespace checks passed. The last complete
save selection remains the earlier **50/50** frozen run in
`TestResults/20260928T121150.841596Z-development.log.json` (643.233 seconds,
20 safeguards, seven original-game scenarios); it predates the latest
annotation and was not rerun. No broad run should be inferred from the
focused checks.

## Unfinished semantic bridges

- **Global and actor DSG:** 194 unknown Global DSG fields remain among the
  largest Fix groups. The highest-use four fields and DSG[24] have narrow
  guarded evidence; a test actor's snapshot/restore path does not establish
  gameplay meaning. Hype DSG[11] is an object-info-code-44 guarded latch,
  but the authored identity and procedure-82 effect remain unresolved.
- **Dialogue and map:** 134 dialogue references have counted-list and
  member-target roles, not answer keys/text/effects. A source-informed live
  probe of one answer object's `+0x24` section-name pointer stopped at an
  Orrery registry traversal panic before reading dialogue bytes; its exact
  status is recorded below. The 60 `1:10`
  map-star words still lack individual gameplay meanings and a verified
  map-open transition in the private display.
- **Level links and other arrays:** 38 saved `1:6` links terminate in bounded
  Level allocations, Fix handles or sentinels, but their source owners and
  effects are not proven. The 123 `1:5` StandardGame four-byte words retain
  mostly unknown bit meanings. Dynamic records and progression states need
  typed consumers, not more counts.
- **Restoration:** first-slot and controlled named-level mode-5 behavior are
  proven for representative fields, not every record kind. Magic selection's
  high byte and other load modes remain open.

## Next discriminating steps after resumption

1. Repair the bounded registry mapper's eager `bool::then_some` subtraction
   at `crates/orrery/src/save_registry.rs:123` and add a focused negative
   address-range check, then repeat the isolated dialogue probe. Promote a
   saved dialogue target only after the live slot points to the bounded
   object and its section-name pointer yields a verified parser key. An
   authored text or effect still needs an independent backing-data join.
2. Obtain an independently verified procedure-ID-to-callback edge for native
   operation 82 before tracing writes to the 64-byte Level allocations or
   assigning Hype DSG[11] a gameplay noun. The decompiled accessor checked
   in `procedure82-native-85.md` is not that edge.
3. Choose a fresh progression state or one concrete unsupported AI consumer
   that can distinguish a high-use unknown field. Avoid another broad
   dialogue/star/link structural survey. Record fixture/build and selected
   behavior before any new live experiment.

The ignored `output/` evidence, generated Rete, private saves and runtime
configuration are local and unavailable on a clean checkout. Use the
[evidence ledger](savegame-semantic-evidence-20260928.md) for pinned hashes,
the [handoff](savegame-semantic-map-handoff-20260928.md) for exact private
runtime restrictions, and the [testing guide](../docs/astrolabe/testing.md)
for focused selection and cleanup budgets. Do not infer unavailable original
game behavior from a Raymap name or a saved value alone.

**Dialogue probe stop status:** the isolated session reached playable manor
phase 9, then Orrery's existing `save_registry::read_registry` panicked on
`attempt to subtract with overflow` before reading the target. Its
`bool::then_some(SourceLocation { offset: source_address - block_base })`
eagerly subtracts even for an out-of-range candidate block. This is an
instrumentation failure, not a negative dialogue result. The worker stopped
without a second trial; no Hype/Wine/Gamescope child remains, and its
temporary test was removed. Partial fixture/build/run evidence is preserved
in the ignored [report](../output/savegame-semantic-map/dialogue-live-bridge-87/dialogue-live-bridge-87.md)
and [JSON](../output/savegame-semantic-map/dialogue-live-bridge-87/dialogue-live-bridge-87.json).
