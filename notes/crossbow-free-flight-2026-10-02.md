# Crossbow free-flight followup — 2026-10-02

This bounded followup starts after local commit `3a41b455`, whose
[paired checkpoint contact and ammunition evidence](crossbow-fixes-2026-10-02.md)
remains separate. It adds an exported-component regression for sustained
flight, expiry and representative target-directed aiming. It does not establish
enemy damage or original-game missed-bolt expiry parity. Magic is outside scope.

## Source-informed expectation and maintained regression

The actual exported `fix:/ai/models/733` normals 1 and 2 test designer field 0
against **150**, increment it once per scheduled tick, and on expiry decrement
the main actor's field 120. When the count reaches zero they clear fields 58
and 17, then kill the arrow. This is a tick counter, not a 150 ms clock test.
The native 60 Hz fixture reaches counter 151 at approximately 2.55 seconds.

`tests/crossbow.rs` imports and exports the full manor into a fresh temporary
project, locates the actual model-733 scene template and runs
`godot/tests/crossbow_free_flight.gd`. The fixture uses maintained support,
native movement, the source firing action 73 and animation attachment release,
then the compiled arrow Brain. It holds the owner AI and places the shooter at
source Z=200 to obtain clear space. The fixture does not change arrow velocity,
collision geometry, lifetime counter or cleanup code. Owner field 29 is zero
in this component fixture; ammunition remains covered by the earlier physical
gameplay and save-reload cases.

Straight and yaw-0.2 target cases require more than 100 source units of free
flight, no native slide/ZDE contact, no Brain failure, initial velocity equal
to released sighting times 45, sighting directed toward the controlled target,
and active owner guards `[1, true, true]`. The tree-exit capture must show counter
151 and guards `[0, false, false]`. Each Godot process has a 35-second deadline;
the inventoried local test has a 180-second deadline including fresh conversion.
No production behavior change was needed for this expiry branch.

## Original missed-shot fixture limit

`crates/behavior-suite/examples/crossbow_miss_probe.rs` is a bounded diagnostic,
not a BDD acceptance case. It loads the supplied English manor save through
Orrery, independently records inventory eight and declared setup three, then
attempts physical Z selection, keypad-0/Up aiming and Space firing. Partial
observations and the exact level/phase are retained if gameplay becomes
unavailable. The display bridge is explicitly `target/validation/orrery`; an
example executable must not recursively launch itself as that bridge.

The final candidate ran once in **21.976 seconds**, with no timeout, leaked
child or remaining process. It captured 34 stopped-byte observations in manor
phase 9, then reached phase **38** at index 34. Sol's screenshot inspection
identified the **Save UI**. The aiming capture showed main-actor action 31 and
no active owner guards. Captured scene-tail identities were Hype, not model
733; this candidate supplies no arrow trajectory, expiry or post-shot inventory
evidence. It must not be called a missed-bolt expiry pass.

Evidence is preserved under
`TestResults/crossbow-upward-source-probe-luna4-20261002/`, including
`aiming.json`, `report.json` and its `runs/` screenshot; sibling `.log` and
`.supervisor.json` retain execution and cleanup. The supplied archive SHA-256
remained `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The earlier default-sandbox worker attempt failed before gameplay on a private
PipeWire socket bind; the final candidate used the normal approved escalation
path. An earlier successful launch lost partial observations on phase exit;
the diagnostic now preserves them. None of those attempts establishes expiry.

The smallest next source step is a verified safe missed-shot start and input
sequence that remains in phase 9 and positively observes model 733 through its
expiry branch. The existing nearby checkpoint shot is already paired gameplay
evidence, but it cannot answer the no-contact expiry question.

## Enemy damage limit

The manor fixture includes `i_Mannequin_Arbalete`,
`level:/persos/world/0/152`, using `level:/ai/models/939`. Its compiled reflex
contains an unsupported actor-expression behavior query at source node 9,
function category 3 operation 1 at node 15, and later unsupported procedures
207/208. Training-target contact alone cannot demonstrate enemy hit-point
damage. The arrow's verified cleanup branch does not itself implement damage.

A damageable enemy start, target identity and independently readable original
hit-point baseline have not been established. Native hit-point/fight API
coverage and that target's compiled hit-response path also remain incomplete.
Raymap's Hype API catalog names `Proc_SubHitPoints`, `Func_GetHitPoints` and
`Fight_SubHitPointsEnemy`; these are leads for the target-specific slice, not
proof of implemented semantics. The smallest next acceptance step is a
reachable combat fixture with a positively identified enemy and before/after
hit-point observations, followed by the exact supported target hit-response
API. A dummy health decrement would conceal this gap.

## Validation and changed files

The bounded selection reuses the maintained `tools/suites.py` inventory,
supervisor, pinned nextest 0.9.145 and source-unchanged check through the saved
recipe `TestResults/run-crossbow-followup-selection-20261002.py`. It selects
27 cases: this new local test, five related Crossbow synthetic regressions,
all 20 always-required safeguards and the actual-input native movement/contact
safeguard. All four mandatory Python check modules run. The remaining inventory
is explicitly excluded from this followup; the earlier full combat result is
retained without relabeling its intermittent scratch failure. The job budget
is 330 seconds including cleanup.

```sh
python3 TestResults/run-crossbow-followup-selection-20261002.py
```

The selection **passed 27/27** in **180.743 seconds**, with 231 exclusions from
the exact 258-case inventory. All four mandatory Python check modules passed.
Build/discovery took 0.329 seconds; test execution took 174.287 seconds including
0.006 seconds of supervisor cleanup. Each stage reports no timeout, leaked
child or remaining process, and source was unchanged throughout. The earlier
intermittent scratch safeguard passed in 0.170 seconds in this selection.
These results do not erase its failure in the earlier full combat selection.

The new fresh-export test passed in **126.315 seconds**. Both native cases
released at physics frame 8 and exited at frame 160, clock 2550 ms and counter
151 in compiled normal 1. Straight flight captured 149 frames and traveled
111.000 source units between its first sample and exit; aimed flight captured
150 frames and traveled 111.750 units. Both had no native slide/ZDE contact or
Brain failure and guards `[1, true, true]` to `[0, false, false]`.
Straight velocity was `[-0.021290, 44.999943, -0.069677]`; aimed velocity was
`[-8.969084, 44.097061, -0.069669]`. Both satisfied the independently declared
released-sighting and controlled-target direction checks.

Evidence:

- `TestResults/crossbow-followup-selection-luna-20261002.log.json`: complete
  selection, exclusions, command stages, timing, cleanup and source digest.
- Sibling `.log.d/tests.xml` and `execute.log`: named per-test results.
- Sibling `.log.d/crossbow-native-free-flight-straight-1790984096489133762.json`
  and `crossbow-native-free-flight-aimed-1790984096489133762.json`: physics
  observations, compiled Brain state, contacts, guards and velocities.
- `TestResults/crossbow-worker-result-selection-20261002.md`: execution-only
  worker extraction; interpretation and acceptance scope remain with Sol.

Additional compile/style checks passed before the selection:

```sh
cargo build --workspace --all-targets --profile validation --locked --offline
cargo check --workspace --all-targets --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --check
```

Logs: `TestResults/crossbow-followup-build2-sol-20261002.log`,
`crossbow-followup-check-sol-20261002.log` and
`crossbow-followup-clippy2-sol-20261002.log`; the final format check is
`crossbow-followup-fmt-sol-20261002.log`.

Changed files:

- `tests/crossbow.rs`
- `godot/tests/crossbow_free_flight.gd`
- `crates/behavior-suite/examples/crossbow_miss_probe.rs`
- `tools/test-manifest.json`
- `docs/astrolabe/testing.md`
- `notes/README.md`
- `notes/implementation-progress.md`
- `notes/crossbow-fixes-2026-10-02.md`
- `notes/crossbow-free-flight-2026-10-02.md`

Original reference repositories, game assets and saves remain preserved.
