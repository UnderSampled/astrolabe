# New Game entrance gate: source vector subtraction

Baseline `12ad58ce49d60b10cdcbb80a29c9118a5e9feffa`; isolated branch
`codex/new-game-dialogue-gate-sol-20261009`. Source-only work: no build,
conversion/import, Godot launch or runtime test performed by this worker.

The actual generated Manor entrance actor `level:/persos/world/0/159`, model
1220, starts Normal0's schedule with S_DummyIA events1071 and1069, then stops
at unsupported category2/operator17/node6. That prevents both MoveLift and
its subsequent event1070. Audio's independent source audit identifies1069 as
the authored resource251 loop and1070 as its stop cue; this failure explains
the otherwise unbounded closing sound without changing audio semantics.

The preserved source nodes specify `SelfPosition - Vector(0,0,3)` and motion
factor5. Normal0's rule sends1070 then1072 once sourceZ falls below33.25 and
its one-shot DSG0 was false. The schedule also sends1070 at motion completion
before entering Normal1. This is the entrance closure. Separate bridge model272
opens sideways along sourceX by2.3 then0.3 and sets global149; its saved-open
branch places it +2.6 sourceX. Those branches remain unchanged.

Source reconstruction commit `e9609f3` in the separate hype-priority checkout
provides `src/ai/vector_subtraction.c`: operator17's descriptor00600148 and
handler00600164 resolve00524d70. It evaluates/captures left before right,
subtracts vector components, writes tag22, and returns the right operand cursor.
The useful AI model, source codec/writer, validation/walker and native GDScript
compiler now represent vector subtraction. Guarded lowering captures the left
before right-side gameplay calls. Godot's native Vector3 owns arithmetic; this
does not reconstruct source VM or portable x87 exception behavior.

Rete schema63 records the typed AST change. Regenerate the actual AI/level
resources; a support-only update leaves the old unsupported node in place.
Prepared regressions cover the authored MoveLift source nodes, meaningful
decode/reconstructed bytes, native endpoint lowering and real motion completion.
Both are registered as bounded30-second unit checks in the release manifest.
Rust formatting parsed the changes and `git diff --check` passed; tests remain
unrun under the shared runtime/build coordination instruction.

Luna's baseline did not reach gate Normal0: at its final passive New Game
sample the gate remained Normal2 with no failure. See the independent ignored
receipt `output/runtime-validation/luna-20261009/LUNA-BASELINE.md`. Actual closure,
cue ordering and post-gate contact still require Kip's coordinated retest.
