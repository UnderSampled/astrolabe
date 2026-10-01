# Sword beam animation release

The generated beam previously began running at Hype's root transform. The
initial creation helper does copy that transform, but this is only the held
object's initial placement. The attack animation releases it later.

Hype family 1 action 135 (`fix:/animations/5195`) has an attachment operation
at frame 3, source frame column 22, configuration slot 70. The original event
dispatcher (`reference/decompilation/functions/0043bcd0_FUN_0043bcd0.c`, case 2)
resolves that frame column's channel, copies its world transform into the held
generated Perso, clears the held reference and activates the generated actor.
The operation's descriptor value is not the channel identifier.

A live Orrery observation of model 708 in the supplied English manoir save
confirmed the distinction. Initial placement was
`[-74.377998352, 26.216623306, 4.547268867]`; release placement was
`[-74.176826477, 28.509592056, 5.408596992]`. Movement then advanced source Y
while retaining release height. This is a rise of 0.861328125 source units,
not a manually selected hand offset. Evidence is retained locally in
`TestResults/sword-beam-source-updates-20260929.json`.

Support interface 39 exports each cue's resolved native channel node path and
marks animations which release a generated object. Such actors wait hidden
with their Brain and motion paused until the attachment cue copies the full
native channel transform and starts them. The same rule applies to the
crossbow. Animation paths resolve the triggering frame's column, including
duplicate channel occurrences, rather than treating the column as a fixed
channel ID.

The native animation regression exercises release at frame zero and after a
column-to-channel mapping change. The Godot sword scenario checks release at
the observed attachment position before checking compiled AI movement and
lifetime. Launch assertions for both weapons begin at the first running
generated frame, allowing the preceding held state.

Original recordings and Godot replays are retained under
`TestResults/sword-beam-recording-20260929/`. Their independent camera and
render timing prevent a claim of pixel parity. The original dispatcher's
intersection check along the attachment release segment remains unsupported;
this change implements the clear-space release placement and activation.
Hit/damage and full combat acceptance remain separate work.

The final `source-final/recording` and `godot-final/recording` sessions both
completed. Individual 30 Hz frames and contact sheets show the beam leaving
the animated attachment above the feet in both engines. The first full-state
capture delayed short input edges; a later attempt used an overlong title-menu
press and reached Configure. These attempts remain diagnostic evidence, not
successful attack comparisons. The final capture uses short menu presses,
sustained combat presses and 1000 ms state sampling. Godot capture used a
1920×1440 virtual display to contain the owned 1024×768 window. Its temporary
replay script allowed 120 seconds for video finalization instead of 30; the
maintained gameplay/support scripts were unchanged by that adjustment.

Final validation: release build, formatting and workspace Clippy passed.
The combat/animation selection completed in 598.2 seconds with unchanged
implementation sources: 94/96 passed, 87 unrelated cases excluded by the
declared selection. Both original and Godot sword scenarios, the native
animation regression and Godot crossbow scenario passed. Failures were the
existing AI inventory expectation (57 versus 58) and the original crossbow's
missing initial displacement observation, which passed in the preceding run.
The overall selection is therefore failed, not an all-green acceptance claim.
Evidence: `TestResults/sword-beam-final.log.json`.

Freshly converted assets were published to the local `godot/generated` tree.
Every file whose bytes differed from the previous ownership manifest was
preserved separately and restored, including the edited level scene, editor
mesh serialization and import settings. Backups and their path inventory are
in `TestResults/sword-beam-recording-20260929/preserved-generated-edits`.
