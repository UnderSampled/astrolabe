# Gameplay handoff — 2026-09-25

## Current checkpoint

The movement BDD cases pass independently in the original game and Godot. The
stationary, moving, and wall jump cases also pass independently; two sampled
action-timing differences remain in their cross-engine comparisons. Sword
selection and its generated beam, the default Fire cast, and **initial**
crossbow-arrow launch have passing focused behavior cases. The initial
crossbow case is deliberately bounded: it proves selection, spawning and
initial motion, not contact, lifetime, ammunition parity or visual weapon
presentation. See the [implementation report](implementation-progress.md),
[gameplay plan](godot-gameplay-plan.md), [combat record](combat-comparison-2026-09-24.md)
and [magick record](magick-comparison-2026-09-24.md) for exact scope.

The latest Godot crossbow BDD passed 1/1 tests and all five steps in headless
mode, with complete cleanup. Its diagnostic observations show Global Brain
field 14 `false` at baseline, selection, flight and all 15 retained arrow
samples. Weapon 80 quantity changes from three to two during flight. The
original initial-launch evidence shows Weapon 80 still at three after the
model-733 arrow appears and advances through stamps 294 and 295, but does
not yet observe Global field 14. Evidence:
`TestResults/weapon-channel-field14-godot-20260925/` and
`TestResults/original-crossbow-retry-20260925-evidence/`. The diagnostic-only
field in `godot/tests/behavior_gameplay.gd` and the latest comparison note
are committed with this handoff.

## First task for the next agent

Make a narrow, checked original-game capture of **live** Fix Global Brain
designer field 14 and Hype's selected weapon/Weapon 80 quantity in the same
paused state at baseline, selected and post-shot. The existing typed
`HypeBuild::scene_actor_with_ai("manoir", [0,1], [0,0,0], [], [14])`
path is a candidate. Verify its returned Perso pointer against the active
Fix module 5, block 0 base plus allocation-15 offset `0x314`, and verify
the linked AI model is canonical Fix model 1. Read the **current** typed
designer value, not the imported default. The static derivation is in
`TestResults/global-live-designer-field14-resolution-static-20260925.md`;
its runtime identity equality is still unverified. If the live pointer or
model link does not match, stop and resolve that identity before interpreting
the value. The previously assigned Luna was interrupted before any edit or
experiment began.

Use a fresh Luna agent for verbose commands or debugger experiments and give
the required permissions at the first launch. Do not take screenshots at
menu items or repeatedly during navigation; typed Orrery reads are preferred.
Astra agents may give design advice but must not run experiments. Keep Godot
gameplay/support code unchanged until a source observation identifies the
specific difference. The user prefers wiring compiled original AI into a
native Godot API; gameplay-specific support approximations are debt to flag.

## Remaining order and known gaps

1. Resolve the source field-14/ammunition branch and the arrow's contact and
   lifetime behavior. The `WaitFrames(2)` updater is understood statically,
   but its runtime action context has not been tied to model 733. The first
   scene-list write observed for the arrow was a same-root unlink/reinsert,
   not removal. `CollideWithNothing` still needs source-backed semantics.
2. Pair source and Godot weapon-channel observations through idle, walk, run,
   selection and attack to reproduce the user's crossbow visibility issue.
   Existing saved-source active bits and Godot channel 71 alone do not prove
   the rendered weapon mapping.
3. Finish sword contact/damage, crossbow contact/lifetime/ammo and the
   source-informed Fire effect behavior; then develop the real follow camera
   as its separate slice, cliff fall/death, and the playable combined demo.
   The current Fire BDD covers cast gate, action and mana; it does not accept
   effect motion/contact/lifetime.

The ignored `TestResults/` evidence is local and should stay local. The supplied
save, installed game and generated assets were not modified in this checkpoint.
No game, debugger or test process was running at handoff.
