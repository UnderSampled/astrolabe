# Original opening turn and absent debug input — 2026-10-03

The normal New Game regression in v5 now agrees with Stephen's observation:
Gogoud completes his first path and actions 0, 26, 24, 23. All 69 rooted actor
Brains are automatic; Global model 1 remains the separate inactive data holder.
Source-associated floor contact is present. Two seconds after the path he stops
at model 210, node 18, category 5 operation 23. PositionDialog already executed
at [0, 405], but RunDialog did not execute; nine loaded dialogue resources alone
do not demonstrate visible dialogue. Full intro acceptance remains pending.

V5 evidence is `TestResults/new-game-preview-selection-v5-sol-20261003.log.d/new-game-opening/`.
Its actual generated 154/210 code, JSON report and existing start/path-end PNGs
remain immutable. The combined selection passed 28/29 cases in 180.275 seconds;
setup, release build and interface-45 regeneration took 353.468 seconds total.
Every process was cleaned up. Godot reported 925 leaked ObjectDB instances on
the New Game exit; process cleanup is distinct from object teardown.

## Source interpretation and implementation

Raymap's Hype API table identifies meta-action 23 as
Action_IncrementalTurnPerso. The executable API catalogue identifies its PL
operands and initializer 0054ced0 / step 0054d030. The initializer's decompilation
captures current horizontal heading and the target actor's position, computes
the shortest signed angle with 0052ec00, then divides by the integer frame
count. A negative count becomes one. The actual opening operand is five in the
retained source asset; the new corpus check verifies current original nodes and
a rooted target before accepting that claim.

Ghidra did not discover step 0054d030 as a separate function. Static disassembly
of the supplied patch executable shows it tests remaining count, decrements it
once, adds the captured angular increment, updates heading and returns the
continuation status. Completion occurs on the following call with zero count.
The source angle helper 0052eb70 chooses PI for a zero horizontal direction.
Evidence: `TestResults/opening-turn-step-source-20261003.txt`, alongside the
existing initializer/angle helper decompilation. The inspected patch executable
SHA-256 is `ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`. No original-game session or
operand intervention was needed for this interpretation.

The useful AST now retains a typed actor target and frame expression. OpenSpace
reads/writes those nodes, Rete saves their meaning, and the Godot compiler emits
an ordinary cancellable coroutine calling native transform methods. The target
angle is captured once, equal steps preserve source frame counting, zero frames
do not move, and negative frames become one. A missing actor remains an explicit
error. This introduces no source opcode interpreter or replacement game AI.

World model 154's eight debug-input edge queries were preserved as unbound inputs.
The retained operands are absent; the new corpus check verifies actual current
zero/absent handles. Engine functions 00533520, 005335a0, 00533620 and 005336c0
explicitly handle null input: Pressed, JustPressed and JustReleased are false,
Released is true. See `TestResults/opening-null-input-source-20261003.txt`.
AbsentButton preserves that useful distinction. A nonzero unresolved handle
still fails rather than receiving a fabricated keyboard binding. Canonical
source output writes the absent handle as zero. World can reach its next genuine
unsupported operation; this does not claim to fix the later activation/music/
fade calls before widescreen or the remaining Hype/camera stops.

## Save regression and checks

V5's native write process exited successfully and printed its sentinel-preserving
proof. The Rust classifier expected the old write text and an old corrupt-stage
text; both are now synchronized with the actual assertions. Unknown engine
errors, script errors and missing proof markers still fail. The possible cyclic
animation-resource diagnostic was already recognized and was not the failing
condition. Fresh-process reload and corrupt/rejection stages were unrun in v5
and remain pending until the affected native save case runs.

Source-only checks passed: strict scoped Clippy (5.02 seconds), exact turn-node
round trip, absent-input meaning/nonzero failure, and all five save classifier
checks. Logs are `TestResults/opening-turn-source-clippy-v1-sol-20261003.log`,
`opening-turn-wire-v1-sol-20261003.log`,
`opening-absent-input-wire-v1-sol-20261003.log` and
`save-marker-classifier-v1-sol-20261003.log`.

The next necessary engineering run includes the synthetic compiled/native turn
contract, current source operands, ordinary regeneration and the existing normal
New Game case. That case now observes the first source dialogue for at most 20
seconds after the path and requires actions 23, 20, 31, 32 and
gogood_texte1_manoir. It keeps private save preservation, restart and menu checks.
This is an affected gameplay regression, not acceptance of the full intro.
Stephen knows how to run the game; notification of regenerated changes must not
wait for an additional visual acceptance run or launch instructions.

Completed UI font/frame commits were cherry-picked as `259ae71`, `3fb3cf7`,
`8e1382b`, `fa9005b`; root support includes both APIs at interface 46 and Rete
schema 39. All 281 declared tests and 21 safeguards remain. Combined format,
scoped strict Clippy and six selection checks passed in 7.111 seconds:
`TestResults/opening-turn-ui-source-preflight-r2-sol-20261003.json`. The first
preflight wrapper mistakenly called a nonexistent supervisor helper and stopped
before running a check; r2 uses the existing supervise API.
