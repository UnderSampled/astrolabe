# Unskipped manor intro reference — 2026-10-03

Stephen's second human take defines the current in-engine intro endpoint. He
selected New Game, skipped no dialogue, supplied no input during the cutscene,
and closed the game after the normal gameplay view returned. The proposed
fast-skip/training third take was cancelled; training is not part of this proof.
Input-only human recording remains a future workflow preference and is paused.

The immutable local source is
`TestResults/manor-intro-sol-visible-v1-20261003-session02/recording/`.
Its report says `failed`: closing the game caused the recorder to reject
FFmpeg's successful exit status 0. Cleanup completed with no remaining children
or supervisor timeout. This is a recorder-finalization gap, not evidence of a
completed Godot intro or an accepted original replay. Do not rewrite the failed
manifest or delete/relabel the take. A reusable scenario can derive its menu
start from the recorded edges without pretending this manifest passed.

Independent FFprobe inspection confirms the short 780×600 FFV1 menu segment
(2.596 seconds), the main 1024×768 FFV1 segment (241.763 seconds, 4,259,854,436
bytes) and FLAC audio (247.078 seconds, non-silent). The reported 78 video frames
count only the finalized first segment; the second segment lacks its normal
final index/summary after the error. Its retained video remains readable.
Luna reported all eight original save-file hashes unchanged. Full footage and
state evidence are sufficient to study the intro; another human take is not
required.

The four delivered key edges are Left Alt press/release around launch seconds
32.55/32.58, then Return press/release at observed seconds 36.474909/36.618560.
The menu was still New Game before Return. Manor phase 9 begins at launch
second 37.740961. There are **zero recorded keyboard edges after that entry**.
Consumption and focus changes remain unverified; the Alt edges must not be
invented as a cutscene requirement. Automated setup still needs the menu Return
press. The full take ends at 247.087827 seconds: about 209.35 seconds after
observed manor entry, including a short idle after the cutscene.

Sol inspected a 12-frame overview and the final gameplay view. The sequence
shows Gogoud's approach and conversation with the grey statue, the bright
transformation effect, Hype becoming animated, their continued dialogue and a
return to a camera behind Hype with the dialogue letterbox gone. Source actor
samples provide useful gates, in launch seconds:

| Time | Main normal/reflex | Action | Meaning of the observation |
| --- | --- | --- | --- |
| 37.806 | 48 / 2 | 47 | Initial stationary Hype, position [-2.052923,147.631805,33.953423] |
| 52.862–121.844 | 28 and 48 / 2 | 47 | Alternating sampled conversation selections |
| 122.842 | 16 / 2 | 47 | Scripted movement phase begins |
| 133.913 | 67 / 2 | 323 | Position has changed to [-13.501330,158.022568,32.932373] |
| 136.914–151.104 | 66,68,70 / 2 | 324–327 | Transformation/animation sequence selections |
| 154.508 | 0 / 3 | 47 | Short transition, then conversation resumes |
| 238.728 | 4 / 3 | 47 | Normal gameplay selection returns |
| 246.264 | 4 / 3 | 47 | User endpoint remains stationary at [-13.562554,159.065216,32.932972] |

These are sampled selections, not every AI transition or a calibrated render
timeline. Relative to manor entry, the observed gameplay return is about
201 seconds. A repeatable reference should perform verified New Game menu
navigation and then run without gameplay input for the observed ~210-second
window, checking the return to normal/reflex 4/3 and the final placement/view.
Do not assume menus require no input or substitute arbitrary displacement for
this endpoint.

Derived inspection artifacts are outside the original recording:
`TestResults/manor-intro-session02-reference-facts-sol.json`,
`TestResults/manor-intro-session02-contact-sheet-sol.png`, and
`TestResults/manor-intro-session02-endpoint-sol.png`. No original evidence was
modified. Camera parameters/movement, Hype initialization, transformation
presentation and dialogue remain native Godot implementation gaps. The previous
Gogoud route pass covers only its component. Systems untestable in manor may be
deferred/no-op under Stephen's general scope instruction, with explicit debt;
this does not exempt behavior demonstrated by this manor take.

The camera effort should eventually cover the intro-to-gameplay follow-camera
handoff at this endpoint. Framing and actor-view foundations are shared, but
the present native checkpoint accepts no motion mode. The source initial
parameter mode is 0 and the v3 opening observation has mode 3; player tracking
and obstruction remain separate behavior assertions. A fixed-boom presentation
camera is not follow-camera acceptance.

Dialogue is the next screen-space concern after the camera gate. Existing
authored demo assets already use CanvasLayer, containers, labels and buttons;
the native character exposes HUD visibility requests. They provide native
building blocks, but no dialogue presenter, source font/text mapping or shared
screen-scaling policy exists yet. Inspect the recorded original dialogue first,
then establish only the screen-space scaling/layers, text/font handling and
input ownership that manor needs. Keep its dialogue presenter separate from
compiled cinematic events and voice sequencing. Advance/skip must consume input
before it can reach gameplay. Menu, loading screen and HUD are later phases,
one at a time; no broad framework or disposable hardcoded overlay is required.
