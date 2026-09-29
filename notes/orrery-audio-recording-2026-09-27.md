# Orrery private game audio, 2026-09-27

Orrery full `record-session` and full `replay-session` now arm a private
PipeWire/PulseAudio null sink and FFmpeg FLAC monitor capture before Wine
launch. Every owned Hype run gets the private route, including input-only and
ordinary controller runs, so its headless audio is not sent to the user's
desktop sink. Input-only recording omits the audio asset; full recapture arms
it through the same full-session path. The run owns the core, Pulse server,
WirePlumber, encoder, and temporary socket directory. Wine receives only the
private `PULSE_SERVER` and explicit null `PULSE_SINK`. The monitor cannot
include the desktop microphone or unrelated desktop streams. The host server
and its default sink are never changed.
The separate native `inspect-original` and `replay-original` supervisor also
owns a private route and passes it to its Gamescope worker. That route remains
owned through the worker-death cleanup path.

The short original-game proof is retained locally under ignored
`output/audio-recording/live-full-20260927-b/`. It ran against the pinned
Wine GE 8-26 Hype build with executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
The manifest and event stream are in `recording/`; `recording/audio.mka` is
1,082,707 bytes, playable FLAC, stereo, 48 kHz, 30.582 seconds. The
`audio_stream_end` event reports maximum level −16.7 dB and
`signal_detected=true`. The encoder was armed 616,202 microseconds before
Wine launch. The session ended `completed` after 30.432 seconds; video and
save evidence finalized, and `report.json` reports cleanup complete.

After the run, no `/tmp/orrery-audio-*` directory or owned Wine, Gamescope,
encoder, or private audio process remained. The desktop Pulse server stayed at
`/run/user/1000/pulse/native`; its default sink remained
`bluez_output.58_18_62_27_FF_61.1`. Existing host PipeWire services continued
normally. The source-installation window was released to the save task.

Focused synthetic checks routed a generated 440 Hz tone into the private sink,
decoded the recorded FLAC, verified signal and launch-relative timing, and
checked socket removal. A forced capture-start failure also removed temporary
resources. The Orrery library unit suite passed 29 tests with three expected
local/integration exclusions when run with required local socket/ptrace
permissions; `cargo clippy` and the release build passed. A prior sandboxed
library run failed three unrelated socket/ptrace tests because those operations
were denied inside the sandbox; the permitted rerun passed.

Limitations: the timing offset is based on the host monotonic launch clock and
does not calibrate Pulse buffering, FFmpeg acquisition latency, or an original
engine update boundary. A digital-silence recording is marked as a coverage
gap. This proof exercised the full record path; input-only omission and full
recapture use the shared mode branches and synthetic fixture checks, but were
not given another live Hype run after the exclusive window was released. The
separate native inspection/replay supervisor's new route was compiled and
linted but not given another live source run in this window. The
immutable live manifest predates a later wording correction and labels the
headless display `gamescope SDL`; newly created manifests say `Gamescope
headless ... private nested Xwayland`.
