# Native action completion targets and Cond123

Source-only follow-up on `codex/new-game-dialogue-gate-sol-20261009`, from
main12ad58ce. No builds/imports/runtime tests were run. The combined pending
gate/dialogue/action changes use provisional Rete63/support85; integration owns
the final union stamp and actual-game publication.

Source commit `e7bf0a01ea6bb6061451d65d44d7507f668e7d96` in the separate
`/home/stephen/Documents/Codex/2026-10-09/task/hype-priority` checkout supplies
complete00436000/00436040 and0054a300/320/3d0/3f0 bodies. Its handoff also records
independent read-only ExecuteAction initializer/update, Cond123 and lifecycle
evidence. It does not claim body credit for those supporting entries.

00436000 requires action-completion byte14a and equality of current action+4
with requested action pointer+144.00436040 independently normalizes byte14b.
WaitEndOfAction0054a300 tests only00436000. ExecuteAction0054a110 completes on
rejected acceptance, numeric requested-id mismatch, or00436000. Its ID query
00435750 compares the separate Data3D+8 word. No branch waits for audio.

The maintained native character previously exposed only the completion flag
to these waits. It now retains a native completion target alongside its numeric
requested ID and exposes `source_action_completed()`. Source00433e90 realigns
its requested pointer to the installed current action after initial or automatic
next-state selection. Native selection/next-state playback does the same with
state identity while preserving the request ID and action-finished signal ID.
This adds ordinary native state properties, not a source scheduler or memory VM.

Cond123's handler00535410 evaluates an actor through00527250/00525920, then
calls00436000. The previous typed AST treated it as a leaf and native lowering
always queried the executing character. The AST/decoder/writer now retain its
actor expression; validation/walking retains dependencies, and guarded lowering
captures and checks that named native actor before testing its completion.
Missing actor state remains a reached diagnostic, never successful completion.

Vector subtraction now always selects guarded lowering, ensuring the captured
left operand precedes right-side evaluation even when this particular authored
endpoint has no guarded children. The native plain-expression helper still
supports direct vector arithmetic. This follow-up is required with gate commit
f6367e6's compiler regression, which expects the captured operands.

Prepared30-second unit checks cover Cond123 self/main source operand meaning,
reconstructed bytes, actor-bearing AI model/Rete parity and named native dispatch,
plus separate ExecuteAction and animation wait shapes. The existing real-export
native motion harness checks transition/automatic completion targets, numeric ID
retention, terminal completion and rejection of a mismatched target even with
the completion flag set. Its manifest now includes AI/animation selection.
The formatter parsed/formatted the new Rust changes and `git diff --check`
passed. The full `cargo fmt --all -- --check` returned1 with47 inherited
formatting differences in unrelated baseline sections; those unrelated edits
were restored. Detailed output is `/tmp/astrolabe-dialogue-gate-rustfmt-check-20261009.log`.
Tests, builds and lint compilation remain unrun under the source-first instruction.

This is independently source-proven missing completion/actor state. It is not
an attribution of the user's Hype reply stall: Luna's passive baseline did not
reach that stage, and its repeated snapshots made progression/timing inconclusive.
Hype's finite first reply needs no source arrow press. A lean coordinated retest
must capture item/holder/callback state and actor current/requested/target states
to determine which remaining contract blocks the reported transition.
