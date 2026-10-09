# Plain-wave source attenuation, 2026-10-09

Audio-only follow-on on `codex/audio-channel-semantics-20261009`, worktree
`output/worktrees/audio-channel-semantics-20261009`. No tests, builds, native
imports, Godot or game processes were launched here. Kip/Luna own runtime.
Support remains provisionally85, Rete62 in this unintegrated batch; the
integration lead chooses the combined interface stamp and regenerates content.

## Production change and source applicability

Converted sample voices opt into source parameter conversion. Ordinary object
requests use native `AudioStreamPlayer3D` with its distance attenuation and
distant low-pass disabled. Maintained `audio/spatial_gain.gd` computes the source
distance byte and its mixer-byte product; the player publishes that gain once.
Object-less music remains nonpositional. Explicitly authored voices keep their
native behavior. No new engine memory model, sound-driver interpreter or
source table loader is introduced.

The checked engine initializes ordinary-object parameter flags to0 and
object-less flags to3. The source mixer sets4 for its nonnull preference node.
Both Sxd converters receive parameters rather than a resource pointer and test
flag1 for direct2D,4 for input-volume multiplication and0x10 for a custom
rolloff pair. Resource `SetSpacable`, `SetVolable` and `SetPanable` descriptor
fields are not translated into flag1 by the checked plain-wave chain. Keeping
`controls.spatial` as the positional decision was therefore wrong for ordinary
converted events. The capability remains retained on the native voice for
inspection; its interpretation in other backends remains separate.

Engine004f9c20 reads `SXD_DIST_SATUR` and `SXD_DIST_BACK`. The original disc's
SATUR2 becomes131072; absent BACK becomes0x7ffffffe. `SXD_DIST_FADE=16` is not
read.004fbf00 derives the endpoint as8*SATUR. Distance is fixed16.16
`H(H(abs(sourceX),abs(sourceY)),abs(sourceZ))`, with H=max+SAR1(min), not a
Euclidean length. Native coordinates are mapped back to source X,-Z,Y, and
each position component is truncated before the wrapped subtraction. The
helper retains signed32 comparison/overflow in the pseudo norm. Finite rolloff
division uses native double/truncation; x87 exceptional and final-bit rounding
parity is not claimed. Default distance0/1/2 yields127,8 yields72,9 yields63,
16/17 yields0. With preference/source/object127, full distance then publishes
`127*127 >>7 =126`; at distance9 it publishes62.

Both checked type1 wave paths use the software converter before PCM playback.
DirectSound creation uses flags0xe8 without a3D-control flag and publishes
software volume/pan. SetMicroParam's backend notification is RET-only. Thus
moving only the listener does not refresh an existing static sample's compact
gain. Native dynamic requests refresh each physics frame, while static requests
refresh on source preference publication, which recaptures current coordinates.
Gain publication preserves the native playback instance, loop and completion.

Engine00431c30 initializes one publication node. Per-Perso Micro data selects
its position/orientation source rather than adding nodes. The maintained API
currently supports that one-node path. Missing selected pose warns and records
an `unbound_microphone` unsupported state with zero distance gain; it does not
assert source null-Perso behavior or substitute an invented camera pose.
Starts and completions still run. Multiple publication nodes and zero
registered-node compact defaults are not exposed by the current gameplay API.

## Source evidence and remaining limits

The dedicated source worker's clean driver follow-on is being committed in
`/home/stephen/Documents/Codex/2026-10-09/task-3/hype-audio-mixer`:
`audio_driver_rolloff.c`, `audio_driver_configuration.c`,
`audio_driver_wavx2.c`, `audio_driver_wavmw.c`. It independently checked the
engine functions and both unchanged original DLLs; the final handoff/evidence
will supply exact spans and hashes. The prior24 engine bodies are integrated
on hype-decomp main `b0439f5964c11d4a1563b45a024299360030e318`.

This patch fixes the distance curve and ordinary/static publication. Native pan
continues to be an approximation and can change with the listener even for a
static request. Sector eligibility is still unported. Resource gain remains
the earlier floating implementation, and final byte amplitude remains /127
until the checked DirectSound table/resource chain is applied. Stereo streamed
samples have an additional copied-byte half step; original descriptor+2c (the
SetStreaming byte) maps to driver+28. Bank ownership does not establish that
flag, and the earlier reconstruction label 'resident buffer' was corrected.
Theme transitions, event modulation and environment/material selectors also
remain incomplete. This is not audible New Game acceptance.

Baseline Luna evidence used Dummy audio and never reached gate1071/1069. Hype
event83 was Play_Hysaute, a jump: its Pas_Hype67 'Rien' result alone does not
prove a wrong footstep. The material callback's contact classification uses
literal global Z and threshold0.5, but upstream traversal selects one contact;
Godot slide-contact order is not established as equivalent. No guessed floor
priority or generic Air/material fallback was added.

## Prepared validation

`native_source_audio_attenuation_preserves_static_publication`, engine20s and
manifest30s, checks literal curve bytes and the non-Euclidean axis grouping,
the selected-Micro API, unbound pose, dynamic/static distinction, preference
recapture and preserved starts. Native Master PCM capture compares near1,
middle9 and far17, including absence of a second attenuation/filter and zero
far output. The existing converted choice/loop/stop fixture now selects a Micro
explicitly, as does the music/receiver fixture. The latter checks parameter88
and compact87 for an object90 near the listener.

Luna's first integration batch `58b0a25` could not run the previous two audio
tests because lib-test compilation failed at `src/godot/ai.rs:4491` with
unresolved `expression`. This was not an audio runtime failure or pass. Evidence:
`output/runtime-validation/luna-20261009/58b0a25e40e9679ec00b8f8488ed4e7bdc0ecd40/runtime-results.md`.
Once integration repairs that unrelated blocker, run the three prepared audio
tests, converted sound program, music/fade fixtures and mandatory safeguards.
Then exercise actual New Game gate movement with the sequence patch:1071 and
1069 start;1070 must remove only the1069 request and finish its logical holder;
1072 is a finite one-shot. Capture PCM and copied request metadata. Keep the
authored1069 loop enabled and never force a closing success without movement.

Godot's [native spatial-player API](https://docs.godotengine.org/en/stable/classes/class_audiostreamplayer3d.html)
documents `ATTENUATION_DISABLED`, the20500 filter bypass and default camera
listener. These were consulted statically; no engine was launched here.
