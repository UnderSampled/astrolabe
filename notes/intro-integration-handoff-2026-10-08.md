# Manor integration checkpoint — 2026-10-08

The owner is parked at the user's request after the current bounded batch.
Production commit: `7bf7b08` on `main`.
This is a source-backed non-audio setup checkpoint, **not full intro acceptance**.
No original executable was run. No full project export, installation, push,
reset, source-data cleanup or reconstructed-C build was performed in this batch.

## Implemented slice

- Preserve the inherited typed Perso stock checkpoint. Stock is reached only
  through Perso +0x38, decoded with a binrw record and projected as a native
  actor-owned Resource with semantic category/item identity and signed quantity.
- Bind the Fix inventory catalogue to Scene/Rete/export. Restore the complete
  native admission/replacement component preserved in commit `294bd26`; it had
  been missing from main despite its exporter and authored callers. Inventory
  and magic are shared within an active level and isolated between instances.
  Saved actor NodePaths resolve their owning scene's resources before child
  Brains become ready; nested PackedScene local-resource overrides did not
  preserve the required shared identity.
- Compile AddObject196, ChangeObjectMaxCapacity418, SetHitPointsMax57,
  AddHitPoints55, Magic_ChangeMaxMagicPoints434, Magic_AddMagicPoints229 and
  Magic_ReadMagicPointsMax231 through the typed AST. **229 is not the maximum
  query.** Keep byte/word narrowing, full magic-add return value, first matching
  inventory ID, gold ID -1, and absent-item no-op behavior.
- Proc376 is map-label activation, not actor visibility. Import 60 ordered
  localized Fix records into a useful Scene component, preserve unknown label
  tail/field data in Rete, and export native map-name Resources. The source
  handler changes all matching IDs and interprets only the activation low byte.
- Preserve the source evaluation order for Proc160: restore the previous
  target's alpha before resolving the new target. Proc163 changes MainActor
  target independently without restoring opacity.
- Legacy save/transition inventory restoration uses the component API rather
  than overwriting its derived quantity view. Save application validates on a
  private catalogue clone before touching live state.
- Support interface is now 71. Generated local project content was **not**
  regenerated. Do not diagnose source/generated node indexes as though they
  already match this checkpoint.

## Source evidence

Life: `00540930`/`00460a90` and `00540b10`/`00460b90`/`00438ef0`.
Magic: `00536550`, dispatch table `005366b4`, and procedure handler `0053de60`.
Inventory capacity: `0053d920` → `0041db30` → `004a3840`.
Map activation: `0053f7d0` → `00416310`; construction `00416800` and parsing
`00416560`. The finite Fix loader `00401e00` consumes, in its separate DLG
stream, FON root plus SDA+8 text handles, the additional root in `0041ea40`,
52 inventory registrations reached from GPT18124 in this corpus, then 60 map
records in `00416420`. The map table starts at DLG byte 1852. The local source
test checks the actual 60-byte allocations, identities, decoded values and
initial activation. No allocation scan substitutes for this path.

Camera target handler `00548070` restores the old target before operand
evaluation; `00548250` is the separate reset-to-MainActor handler. The independent
static C project at `/home/stephen/Projects/hype-decomp` corroborates these
findings. Its files were not edited or run by this owner.

## Validation and retained evidence

Evidence is ignored under `output/integration-20261008/`.

- `workspace-check.log`: `cargo check --locked --workspace --all-targets
  --offline` passed (12.79 seconds).
- `ai-focused-final.log`: all 26 AI tests passed (15.27 seconds), including
  native target/reset behavior, source/Rete compilation, and first-visit setup.
  `setup-final.log` rechecks setup after the final single-evaluation emission
  adjustment.
- `saved-scene-focused-2.log`: saved-camera projection plus actual native
  catalogue/stock/map-name binding and cross-instance isolation passed
  (7.90 seconds). The preceding failed run caught incorrect resource sharing.
- `names-focused-4.log`: supplied English source map table passed (1.30 seconds).
- `cargo fmt --all --check` and `git diff --check` passed.

Two supervised AI/save attempts stopped during build/discovery, first on a new
test's missing ordering trait and then on Orrery fixture initializers. Both
compile failures were corrected. **The full declared selection and mandatory
safeguards were not completed**, and no release/clippy/full authored acceptance
is claimed. No new long run was started after the user requested wind-down.

## Remaining work

- Audio remains blocked/paused under the explicit execution denial. The known
  authored first stop is SendMusicRequest for `sound:/Sound/playmo_b.lcb`, bank36,
  member0, following music fade127,1. Receiver-wrapped413 remains an unsupported
  audio call. Do not silently bypass these to claim intro acceptance.
- Proc160's four 16-bit reset slots and two 32-bit stores are not projected as
  invented runtime memory. The latter have float-history consumers in
  `0051cfa0` and the angular helper reading `00441550`; their camera-motion paths need
  semantic mapping. The change here establishes opacity/target ordering only.
- Conditional IsDrawn217 remains unmapped. Existing mode-0 obstruction and
  interpolation gaps, dialogue fix `0d8a769` integration, letterbox sequence,
  complete authored camera shots and eventual player handoff need acceptance.
- `godot/project.godot` and the authored New Game route are the actual game.
  `game/demo.gd` still owns session orchestration and fallback `DemoCamera`;
  imported `SourceCamera` becomes current when its Brain runs with a target.
  That ownership needs a deliberate architecture pass, not another demo.
- Native save schema1 remains sparse and does not preserve the entire newly
  mutable inventory/map-name state. Full component snapshot save composition
  and the existing strict-int helper versus JSON fixture numbers need review;
  these were not broadened during wind-down.

## Quota/cleanup coordination

No files were removed and no quota recovery is claimed. This owner's active
scratch was project-backed (`output/integration-20261008/`, its `tmp/`, and
`target/`). Reconstruction lead, collision and particle owners each reported
no owned `/tmp` files, no disposable scratch and no background runtime process.
Their worktrees and ignored evidence remain needed. The failing `/tmp/.codex`
synthetic mount is tool-managed. Its filesystem inspection was explicitly
denied to the audio owner; this owner did not repeat that action through another
route. No supported quota recovery operation is exposed in the available tools.

The app's parent-thread message tool repeatedly returned `thread not found`;
those status-send attempts were not delivered. Use this handoff and the final
task outcome for coordination. No Astrolabe worktree was created by this owner.
All owned build/test sessions from this batch exited; no background process is
being left running. The earlier source-camera note was preserved without content
edits by this owner and is committed separately with this handoff, as requested
by the final clean-state instruction.
