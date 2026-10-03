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

World model 154's seven debug-input edge queries were preserved as unbound inputs.
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

V6 stopped at its early source-only corpus case before release build or
regeneration (32.269 seconds total). Its current source decode verifies Gogoud
node18 targets `world/0/28` (`iHypeStatue_Manoir_MAD`, model294) for five frames.
All absent-input checks passed before an incorrect final rule-count assertion:
there are eight total rules, comprising one startup rule and seven absent
JustPressed queries. The unchanged generated World code's rules 0 through 7 and
retained Rete independently corroborate that count. The test now requires all
eight slots populated, each of the seven conditions actually JustPressed, and
the exact turn target. The production implementation is unchanged.

The ten parsers and ordinary menu import passed in v6; menu assets are available
independently of intro repair. Evidence is
`TestResults/new-game-preview-setup-v6-sol-20261003/`. Source stayed unchanged,
with no timeout or remaining process; the native turn contract and later stages
were unrun. The fresh assignment reuses those parser/import passes and retains
the early source/native turn gate before expensive conversion.

The corrected read-only source case passed under
`cargo test --locked --offline -p astrolabe --lib openspace::level::ai::tests::source_opening_turn_and_absent_world_inputs_are_bound_deliberately -- --exact --ignored --nocapture`:
3.31 seconds of build and 1.27 seconds of test. No Godot or original-game process
was launched by Sol. Log:
`TestResults/opening-source-operands-correction-v7-sol-20261003.log`.

## V7 native turn pass and bounded font preflight recovery

The compiled/native incremental-turn contract passed in 0.61 seconds in v7.
The release build also passed, but ordinary port stopped before import/publication:
the new ten-second `--check-only res://build.gd` gate timed out. Total setup was
64.79 seconds. The verified 4,774-file interface45 backup was restored, failed
output archived, source unchanged and descendant cleanup complete. Evidence:
`TestResults/new-game-preview-setup-v7-sol-20261003/result.json`, `native-turn.log`
and `port.log`. No interface46 generation or later gameplay/save pass occurred.

The failing temporary project was removed by TempDir. A subsequent source-only
export capture used a deliberately failing launcher without invoking Godot:
the equivalent complete builder is 9,018,332 bytes / 64,487 lines, with 3,563
staged files. This establishes that the early gate parsed a whole level rather
than a small font fixture; the timeout alone does not establish the internal
parser cause. Retained static evidence is in
`TestResults/opening-builder-static-v8-sol-20261003/`.

The exporter now checks the exact emitted font bodies in a separate minimal
project, with an owned engine log and the same ten-second deadline. Ordinary
import/build still validates the complete native builder and resources. A second
static capture confirms the 89,258-byte / 1,216-line font script contains exactly
the actual font body, and the full builder is byte-for-byte identical to the
pre-fix capture (SHA256
`0ef7c0f201fde7294b5ad29a4cb2d777dada2e5630f99bfabe58e13149001c02`).
These intentional source-only captures are not native parser or gameplay passes.
Evidence: `TestResults/opening-font-static-v8-sol-20261003/static-capture.json`.

Current-source checks passed: `cargo build --release --locked --offline`
(56.88 seconds), `cargo fmt --all --check` (0.49 seconds), and
`cargo clippy --locked --offline -p astrolabe -p godot-harness --all-targets -- -D warnings`
(3.30 seconds). Logs/results:
`TestResults/opening-font-preflight-source-v8-sol-20261003/`.
V8 reuses those build checks, v7's native turn pass, earlier parser/menu/source
passes and the hash-verified backup. It performs one ordinary port, including
its own small font gate, before the unchanged 31-case selection. The full builder
retains its existing 360-second inner budget; the port stage is bounded at 360
seconds within the unchanged 600-second outer budget, with 15 seconds reserved
for cleanup. No duplicate native font probe or visual acceptance gate is added.
Native recovery, first dialogue, fresh save reload/rejection and full intro
acceptance remain pending.

## V8 generation/save pass and actual wrapped-level target diagnosis

V8's ordinary manor port succeeded in 108.21 seconds, with interface46 and
4,785 owned files. The isolated font syntax gate no longer blocked conversion.
The later selection passed 29/31 cases, including all 21 mandatory safeguards
and the full native save write/fresh-process reload/rejection case (30.93s).
Source stayed unchanged and process cleanup was complete. The two failures
were actual New Game after the first path and the separately owned UI font
Unicode mapping case (`has_e_acute=false`). Do not describe this as a clean
full selection. Evidence: `TestResults/new-game-preview-setup-v8-sol-20261003/`
and `TestResults/new-game-preview-selection-v8-sol-20261003.log.json`.

Actual Gogoud again completed actions0/26/24/23 with floor/source contact. At
clock12183.33ms he stopped because the incremental turn target resolved null.
The generated `level.tscn` already contains both the statue node and the exact
`level:/persos/world/0/28` entry, while unchanged compiled model210 calls
`character().source_actor_node` with that URI. The helper incorrectly climbed
past the exported manor to its authored `ManorNewGame` wrapper, which owns no
actor index. This is native scene ownership, not a missing source binding or
missing exported asset. Evidence:
`TestResults/new-game-preview-selection-v8-sol-20261003.log.d/new-game-opening/report.json`
and its retained model210/receipt, corroborated against current generated
`level.tscn` and the maintained character helper.

`source_actor_node` now resolves against the nearest ancestor owning
`source_actor_nodes`. A missing identity in that owner returns null rather than
escaping into another scene's index. The existing compiled/native turn contract
now adds a reference-target variant in a wrapped level, with a conflicting
outer target and an outer-only identity; all earlier keyword-target, frame,
snapshot and cancellation assertions remain. Actual New Game reports/asserts
the exact resolved statue and records its post-turn remaining angle. Original
AI, generated files and user saves remain unchanged. This API fix needs no
regeneration and keeps support interface46.

Current source-only checks passed: both AI and New Game test executables compile
(0.76s and 14.81s), formatting (0.50s), and scoped strict Clippy (3.00s), using
locked/offline commands in
`TestResults/opening-actor-owner-source-v9-r2-sol-20261003/result.json`.
The first source compilation caught a test constructor typo; its failed evidence
remains in the v9 directory, with the corrected pass separately in v9-r2.
The extended native contract and actual first-dialogue acceptance are unrun.
V9 selects the four affected turn/camera/UI/New Game cases plus all 21 mandatory
safeguards and four mandatory Python modules, without repeating conversion or
the unaffected save case. The separately owned glyph fix and full intro/Hype/
widescreen/camera acceptance remain pending.

## V9 referenced-turn pass, smoke parser failure and parser-first retry

V9 passed 24/25 selected cases, including all 21 mandatory safeguards, the
extended compiled/native reference-target turn contract (0.606s), camera lookup
and UI timing/adoption. Actual New Game was unrun: my two reporting declarations
used inferred types for dynamically attached character-script methods on a
`CharacterBody3D`, which cannot supply static return types. Its smoke parser
reported `resolved_target` at line137 and `remaining_turn` at line182. The setup
took 142.28 seconds; source/generated hashes stayed unchanged and cleanup was
complete. Evidence: `TestResults/new-game-preview-setup-v9-sol-20261003/` and
`TestResults/new-game-preview-selection-v9-sol-20261003.log.d/new-game-opening/parser-smoke.log`.

Only those declarations change: explicit `Node3D` and `Vector2`. No production
behavior, compiled original AI, generated assets, inventory or Rust test logic
changes. Replacing only the current smoke bytes with their committed v9 bytes
in a read-only source-digest calculation reproduces the complete prior digest
`bbb472139835957871c86cb0c46e42481f961d123ff573af325a6eb60a90da8c`.
This proves every other source input remains identical, supporting reuse of
the 24 individual passes and four clean mandatory Python module runs rather
than rerunning a broad selection before the failed opening.

V10 first stages the exact current smoke and its actual unchanged SAVE_STATE
preload in a fresh minimal project. It checks source/staged SHA256 hashes and
parses the changed script under a ten-second deadline with an owned engine log,
before any Cargo build or gameplay. Every changed GDScript must be included in
that hash-checked gate; prior parse evidence cannot stand in for changed bytes.
A failure stops immediately. The successful proof records all input hashes and
the parser-log hash, verifies the pinned Godot build, then builds only the New
Game test executable and runs only the actual opening under its existing
180-second deadline. The outer budget is 300 seconds, including a 15-second
cleanup reserve. Current native parsing and opening remain unrun.

Retained v9 safeguards, declared 281-case inventory, complete v8 save pass and
all 4,785 generated hashes remain checked. V10 reports one fresh native opening
case separately from 24 reused passes. No regeneration, release build, broad
selection, original-game run or additional visual gate is needed for this
harness-only correction. The UI glyph work remains separately owned.

Before V10 froze, the parent authorized integrating UI commit
`3b2ba700d6bcf8e91246c27fa4f24c512fc6ff63`; it is cherry-picked as `23d4afe`.
Its named-font A/B reload passed in 2.06s with all133 Unicode codes, source glyph
metrics and no empty16px cache. The exact changed resource-test script and its
entire emitter preload closure hash-match that passing parser evidence, so
that parse can be reused. The newly typed opening still requires its first
fresh native parse before any build. Current release inventory adds exactly
two pure font normalization tests, retaining all281 old rows for283 total.
Evidence: `/home/stephen/Documents/Codex/2026-10-03/task-2/ui-font-reload-20261003-r9/report.json`.

The current owned FontFile hash exactly matched the A/B probe's original. The
parent authorized narrow resource normalization rather than a full level port.
The validated ordered file differs only by moving fixed_size21 before font_name;
all native glyph/cache/image/settings bytes are retained. I copied those exact
validated bytes after checking every owned hash, archived the original font and
full receipt, and updated only that font's ownership hash in the existing
receipt. All4,785 owned files validate; interface46 and level ownership stay
unchanged. The old font hash is
`48cd6c3ea68a63daafebed609d924689ca76c48f66a778ab3ce5520509cb56cf`,
new hash `ab409595e5191d16a2b7a257e5884505971b647854277aee2ef1c72cda1591dc`;
new receipt hash
`27ced4a5135f12815c730110f81171dfca7539dcfd1cdd7bbc72b0018e0708b3`.
Backup: `output/archived-godot-manor-font-cache-order-20261003/`.
Repair evidence: `TestResults/manor-font-cache-order-repair-v10-sol-20261003/report.json`.
This reuses native resource validation, not actual New Game visual acceptance.
Interface47 remains reserved for the separately owned source-menu layout work.

The revised V10 keeps actual opening first after fresh hash-checked parsing and
its targeted test build. If that passes, it runs both added pure normalization
cases, the changed source/Rete é assertion, and the actual nine-dialogue native
resource case in its temporary project, then integrated format/scoped Clippy.
There are five fresh cases, with24 unchanged v9 passes retained separately.
The exact prior global source digest is reproduced by overriding only the five
scoped source inputs changed by smoke/font integration, and the current manifest
must equal the prior manifest with only the two declared cases added. Process,
input, movement and ownership safeguard evidence stays retained; font-specific
normalization/new UI assertions get fresh checks. The revised outer budget is
420 seconds with15 seconds cleanup reserved, original180-second opening/resource
deadlines, and no full level port, release build or extra visual acceptance run.
Fresh current-byte parsing and actual opening are still unrun.
