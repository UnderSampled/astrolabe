# Hype voice and effects fade trace — 2026-10-08

This note records the source-backed implementation of `Proc_SND_FadeVoice`
(procedure 409) and `Proc_SND_FadeSound` (procedure 410). The scope is the two
calls reached by the Global AI script described below. This is a bounded static
trace of the Hype executable, not an observation of live audio playback.

## Reached calls

`notes/research/hype-ai-api.tsv` rows 781–784 identify procedures 408–411 as
`FadeMusic`, `FadeVoice`, `FadeSound`, and `FadeAll`. All use dispatcher
`0x0053ec70`, take two integer operands, and are global calls rather than actor
calls. `output/ai-research/inventory.json` records one call each to 409 and 410
in `Fix/17-0/21`, reached from `Fix/17-0/1`, Global normal behavior 2, on
`world/0/1`. Both calls pass target level `0` and duration `800` ms. They occur
after `FadeMusic(0, 800)` and before the wait and music restoration calls in
that schedule. The source model link is also visible in
`output/ai-research/scripts.txt` lines 125–126.

## Static executable evidence

The installed executable is
`/home/stephen/Games/hype-the-time-quest/drive_c/Games/Hype/MaiDFXvr_bleu.exe`
(SHA-256 `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`).
The reference executable is `reference/hype_patch/MaiDFXvr_bleu.exe` (SHA-256
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`, matching
the executable hash recorded in the API inventory). The whole-file hashes
differ. The relevant `.text` ranges compare byte-for-byte equal in both builds:

| Purpose | Address | Length |
| --- | ---: | ---: |
| Procedure dispatcher | `0x0053ec70` | `0x2b0` |
| Fade setup | `0x0040ba10` | `0xc0` |
| Fade update | `0x0040b770` | `0x190` |
| Mixer volume helpers | `0x004ed9c0` | `0x90` |

The dispatcher subtracts `0x196` from the procedure ID and indexes its table at
`0x0053ef20`. The 409 case calls setup at `0x0040ba50` with the voice channel
state (`0x0075ec64`, record `0x00754300`); the 410 case calls `0x0040ba90`
with the effects/sound channel state (`0x0075ec70`, record `0x007529a0`). The
neighboring 408 music case calls `0x0040ba10` (state `0x007543dc`, record
`0x007529c0`), and 411 calls all three setup paths. Each setup reads the current
channel level through `0x004eda20` → `0x004eb820`, marks that channel's record
active, and stores start level, target level, duration, and the shared engine
time at `0x00754ab0`.

The update path at `0x0040b770` processes separate per-channel records. The
voice and sound update branches are at `0x0040b820` and `0x0040b890`. They
interpolate from the recorded current level toward the target over the stored
duration, constrain the result to the source level range 0–127, apply it via
`0x004ed9c0` → `0x004eb710`, and clear the active flag when the shared clock
reaches the end time. A later fade setup replaces the record for that channel;
other channel records remain independent. The setup's use of the current level
also makes a replacement continue from the currently applied level.

## Godot mapping and limits

The typed AI variants preserve each source target and millisecond duration
through OpenSpace AI read/write, Rete serialization, and GDScript generation.
The runtime maps voice to `Astrolabe Voice` and sound effects to
`Astrolabe Effects`, the buses already used by the Godot settings and audio
emitter. Music remains on `Astrolabe Music`. The maintained `Behavior` support
keeps active fade tweens in metadata on the persistent SceneTree root and uses
tree-owned tweens. Thus the fader outlives the Brain that requested it, and a
call from another Brain replaces only that same channel's active fade. The
voice, effects, and music channels can fade concurrently. Levels are
interpolated linearly in the source 0–127 domain, then converted to Godot bus
volume; the Godot mixer transfer curve is not claimed to be perceptually or
waveform identical to Hype's mixer.

The disassembly establishes the timer and channel-record behavior, but this
trace does not establish zero-duration behavior from a reached source call,
does not establish every possible target/duration combination by an in-game
observation, and does not compare resulting audio waveforms. The source call
here uses `(0, 800)`. No Hype process or debugger was used. A synthetic Godot
test checks typed source/Rete/compiler round trips, separate channel state,
replacement, cleanup, and tween lifetime after Brain removal.
