# Combat comparison, 2026-09-24

The first combat recording starts from the supplied English `SaveGame0` in
`manoir`. Orrery physically holds Down for 800 ms to reach clear ground, waits
700 ms, then presses X for 120 ms and Space for 100 ms. The recording samples
Hype before selection, during drawing, twice during the swing, and twice after
recovery. It uses the same Cucumber steps and recorded physical input tape for
OpenSpace and Godot. The Godot run is headless by default.

The first original-game run reached action IDs **47, 45, 134, 134, 48, 48** at
0, 1100, 1700, 2000, 2700, and 3200 ms. A repeat reached **47, 45, 134, 48,
48, 48**: the swing had finished by the 2000 ms sample. Hype's source position
did not change in either run. Reports are
`TestResults/behavior-sword-attack-1790250237804405770.json` and
`TestResults/behavior-sword-attack-1790250359524316642.json`. The first test
failed because its provisional selection checkpoint required action 48; the
observed draw action was 45. The second failed because its late checkpoint
required action 134; the source had already recovered to 48. Acceptance now
requires action 45 during drawing, 134 at the early swing checkpoint, either
134 or 48 in the observed transition window, and 48 after recovery. The source
rerun passes all five Cucumber steps in
`TestResults/behavior-sword-attack-1790250567229954228.json`. The Godot
gameplay result is pending.

The sword test establishes selection, one swing and recovery on clear ground.
It does not yet establish a hit, target damage, weapon attachment visibility,
inventory state, or crossbow projectile behavior. Those need separately
observed effects and assertions. The current exporter preserves unsupported AI
operations with explicit failures, including channel presentation calls in
Hype's sword path. Implement the confirmed calls as typed model operations
compiled to Godot support methods; keep unknown gameplay effects explicit.

The first headless Godot run reached the manor Brain and stopped before the X
input on source `Proc_DeactivateChannel` (operation 105, model 28 node 885):
`TestResults/behavior-sword-attack-1790250602075227445.json`. Static source
decode identified its child as category 18, channel 72. Other reached channel
requests name channels 28, 80, 116 and 136; all five have corresponding
exported channel nodes with matching `source_channel` metadata. The source
script's HUD display calls 422–426 each have one integer Boolean operand.
They now decode as typed operations and call Godot support methods. Godot
toggles the matching channel parents and retains HUD visibility requests for
an authored native HUD. Neither the original handler's full channel side
effects nor visible HUD parity have been established.

The next headless Godot run reached the sword swing and recovery with action
sequence **47, 47, 134, 48, 48, 48**. Its only assertion failure was the draw
checkpoint, where the source reported 45 and Godot had returned to 47:
`TestResults/behavior-sword-attack-1790250994974308200.json`. No unsupported
call stopped the Brain. A source rerun using the jump suite's stopped-capture
pause compensation still reported action 45 at that draw checkpoint, ruling
out that timing-accounting difference. It reported action 135 rather than 134
at the early swing checkpoint; those phases need source action-family review
before the assertion is broadened. The pause-compensated source report is
`TestResults/behavior-sword-attack-1790251426006205933.json`.

Static action-family review showed normal behavior 29 executes actions 135 and
134 consecutively before armed idle 48. The early swing checkpoint may sample
either phase; raw IDs remain in the reports. The draw branch executes action
88 (0.45 s), then 45 (0.70 s), then requests 48. Godot action 47 at the draw
checkpoint is an actual discrepancy, so its draw assertion remains strict.
The native action request trace now records request, transition, selected
state and relevant designer fields to locate the interruption.

The trace in `TestResults/sword-debug-20260924/behavior-sword-attack-1790252367310834044.json`
showed the X path did run but requested action `-1` twice. The exported Hype
Brain's action arrays 162 and 164 defaulted to `-1` because the English-save
fixture resumed normal behavior 4 without restoring the arrays populated by
normal behavior 0. Orrery read both arrays from the original game's supplied
English save at startup and again after Down for 800 ms plus 700 ms settling;
both observations agreed. Array 162 was
`[88,45,200,201,192,194,216,220,235,236,218,221]`; array 164 was
`[48,48,47,31,31,32,92,92,33,27,27,36,63,111,82,62,112,85]`. The
original run was `run-1790252707092054741` under the local Orrery runtime
directory, using executable SHA-256 `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`
and save SHA-256 `877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The fixture now restores these values as typed Godot integer arrays and records
them in setup evidence. This is saved-state restoration, not an action-remapping
rule. The post-fix headless Godot run passed all nine assertions, with action
sequence **47, 45, 134, 48, 48, 48** and X/Space input edges at their planned
elapsed times. Its report is
`TestResults/behavior-sword-attack-1790252943191621702.json`.

The focused `python3 tools/test.py --area combat --local --offline` selection
passed **63/63** tests headless in 258.759 seconds, including its original and
Godot sword cases and mandatory safeguards. Its report is
`TestResults/20260924T123038.161616Z-development.log.json`. The fresh paired
backend reports are its `behavior-sword-attack-1790253271424542380.json`
(OpenSpace) and `behavior-sword-attack-1790253144917994468.json` (Godot).
Their recording hashes match and both sampled **47, 45, 134, 48, 48, 48**.
`TestResults/behavior-sword-attack-cross-backend-20260924.json` passes all 22
comparison checks: relative XY/heading errors and input-delivery deltas are
zero at the named checkpoints. Godot's starting height differs by roughly
0.068 source units. The comparison does not establish wall-clock or
input-consumption timing parity; Godot ran headless, so visual pairing remains
pending.
