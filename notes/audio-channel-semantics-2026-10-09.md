# Audio request scope and completion, 2026-10-09

Source-only work starts at `12ad58ce49d60b10cdcbb80a29c9118a5e9feffa` in
`output/worktrees/audio-channel-semantics-20261009`, branch
`codex/audio-channel-semantics-20261009`. Support interface is provisionally 85;
Rete stays 62. Kip must choose the merged support stamp with the sequence worker's
parallel support 85/Rete 63 changes. Main, original disc inputs, settings, saves,
generated game resources and reference repositories were not changed.

No runtime tests, Godot, game process, imports or builds were launched. Latest
delegation reserves runtime testing for Luna and this worker for source analysis
and implementation. The branch worktree was created through the permission
reviewer after ordinary creation could not write the main Git metadata.
Local messaging to source parent `01a116d7-0d91-75e3-8be8-9d923e96f03d` returned
`thread not found`; findings and experiment requests were posted in commentary
and shared with the active dialogue/gate worker. Runtime acceptance is pending.

## Production correction

`audio/emitter.gd` previously stored effects and speech in one native request
list, and `stop_event(identity)` matched identity alone. That could stop the
other source module when it used the same event. Stoppable removal freed its
player without publishing `voice_completed`, so the maintained dialogue
component could retain its owner record indefinitely. Nonabortable removal
waited for natural playback completion before notifying holders, although its
request had already been removed.

Requests now record their source module and native bus. Ordinary compiled
events select effects; the shared channel selects music; speech dispatch and
`execute_voice_event` select voice. STOP filters by module and event.
Request removal publishes one completion on the next native frame. Stoppable
players stop immediately; nonabortable players detach at their captured pose,
continue as native audio, and free on natural finish without a second request
completion. The effects-fade and bulk-kill paths share the existing guarded
completion publication. Proc386's raw word and Proc412's effects-only
fade/reset remain unchanged. The existing music fixture now waits for the
already-required next-frame bulk-kill completion before checking removal.
Per-object volume refresh now applies only to dynamic effects requests; static
requests retain their captured gain, matching clean004ec710's branch between
parameter recomputation and playback-status polling. New effects still capture
the current raw override.

No source memory tables, request scheduler or audio interpreter were added.
Native nodes, native playback signals and one-frame completion publication own
the lifecycle. Shared UI/character code is not modified by this worker.

Clean original-source evidence, read from `/home/stephen/Projects/hype-decomp`:
the inspected checkout is `e24540eec4546dc3ee70574ded56821dc72af1e7`.

- `src/sound/event_dispatch.c`,004ec880: request dispatch takes object, source
  module and event; STOP cases2/3 retain object/source when removing requests.
- `src/dialogue/speech_record.c`,0041e8a0: speech installs its owner record and
  dispatches the event on voice source0075ec64. Effects127/413 and music129
  select distinct sources in `src/sound/ai_requests.c`.
- `src/sound/request_playback_position.c`,004ecb60: an absent request returns
  -2. `src/dialogue/speech_animation.c`,0041e4c0 observes -2/-1 on the later
  speech update, kills voice-module requests and clears the holder.
- `src/sound/event_dispatch.c`,004ecc00: object kill removes requests and
  clears its source-module override. Existing Proc412 interpolation/admission
  evidence is recorded in `ambient-sector-volume-2026-10-09.md`.
- `src/sound/microphone_publication.c`,004ec710: only events with nonzero
  dynamic word+1c recompute parameters; static requests poll playback status.

The lower driver stop/detach implementation is still untranslated. This patch
retains the supported distinction between abortable and nonabortable playback;
it does not claim original driver timing, detach internals or byte-level gain
parity.

## Gate loop diagnosis

The sequence worker identified two different actors. The bridge gate is
`iMGrille_Manoir_Pont`, model272. The entrance gate is
`i_Grille_Entree_Manoir1`, `/world/0/159`, model1220. The entrance Normal0 sends
1071 then1069 and previously stopped on unsupported vector subtraction before
MoveLift. The sequence worker owns that arithmetic/movement correction and the
actor's original STOP cues. The entrance moves toward
`Me.position - Vector3(0,0,3)` with factor5. The bridge's sideways movement is a
separate sequence; no raised-gate workaround is introduced.

Direct unchanged-disc CSB evidence in `disc/Sound/S_DummyIA.csb`:

| Event | Name/resource | Authored behavior |
| --- | --- | --- |
| 1071 | Play_Ougm210d /250 | One-shot, volume90 |
| 1069 | Play_Ougm210c /251 | Loop, volume90, loop data0..36466 |
| 1070 | Stop_Ougm210c | Stops event1069 on the selected object/source |
| 1072 | Play_Ougm210f /252 | One-shot, volume100 |

Thus this gate's sustained sound is an authored loop awaiting its stop cue;
disabling loop playback would hide the stalled actor sequence. Separate
Portetro/Portetrf door resources inspected earlier are non-looping and do not
establish this gate's behavior. No runtime gate or audio result is claimed.

## Mixer and attenuation gaps for the decomp worker

These original functions still have no clean reconstruction in the inspected
coverage. They were reported to Kip; their raw evidence is read-only under
`reference/ghidra/functions/` in the independent decomp repository:

| Original function(s) | Needed contract |
| --- | --- |
| 004ebca0,004ebd60,004f3570,004f41b0 | Source-scoped removal, nonabortable release and driver kill |
| 004ebf50,004ec090,004ec300,004eb820,004eb840 | Common parameters, object/source gain, exact DWORD multiplication and byte narrowing |
| 00432bd0,004ecfb0,004ed0e0,004ed2f0,004eda20 | Registration, separate preference volume and source-channel fade volume |
| 004f5d50,004f7fe0; imported `_SND_fn_lPlaySxd_20` | Selected sample driver; spatial attenuation and gain/flag interpretation |

The native emitter currently treats the full retained Proc386 override word as
linear amplitude. Raw004ec090 instead calculates a bounded-width product and
publishes a byte parameter; exact arithmetic and driver interpretation need
translation before replacing that gain approximation. Actual model1290
Proc386 operands are bounded integer DSGs per the previous ambient checkpoint,
so a catastrophic raw-float gain is a general hazard, not an observed cause of
the New Game mix.

Raw registration00432bd0 and creation004ed0e0 show a preference-volume pointer
at source+28 and a separate source override word at+38 initialized127.
004eda20 reads the latter for source fade setup. Authored UserSettings and AI
fades currently write the same native bus, so a fade can overwrite the user's
mix preference. A native source bus sending to a preference bus is a plausible
Godot representation, pending the clean getter/setter and scaling contract.
No source/user bus separation or blanket gain reduction is claimed here.

Spatial players currently use Godot's native default attenuation. Source
distance curves, source flags (`SetVolable`, `SetPanable`, `SetSpacable`),
reverb and sector sound propagation remain unverified. Raymap's local source
does not expose a useful Montreal sound playback/attenuation implementation in
the inspected object/sector paths; it is not evidence for Godot defaults.

## Prepared validation and Luna requests

`native_sound_requests_keep_module_scope_and_complete_removed_holders` is an
ignored native test declared in the media/AI release inventory, with a 20-second
engine limit and 30-second manifest limit. It initializes only the maintained
support in its own temporary project and uses native PCM test resources. This
component regression is separate from actual New Game acceptance. It checks:

1. The same event starts distinct effects/voice players on their native buses.
2. Proc386 zero/exact127 effects overrides preserve independent speech gain,
   retain a full non-sentinel DWORD and leave existing static effects unchanged.
3. Effects STOP leaves voice active; voice STOP clears the actual maintained
   Dialogue holder on the next frame.
4. Removed nonabortable playback keeps its captured pose, completes once and
   frees at natural native finish.
5. Proc412 stops only effects and resets the override; bulk kill later clears
   the retained speech holder. Start/completion counts must both equal 6.

Luna should run parser preflight first, then the focused declared native test
and affected existing sound program/music, speech-holder and Proc386/412 cases
with their mandatory safeguards, in the shared runtime slot. No result exists
yet. Inspect `voice_started(info)` module/bus/identity/gain/dynamic metadata and
`voice_completed(info)` completion reason in the normal complete New Game path.

For gate acceptance, capture1071/1069 requests, native start count and loop mode,
actor Normal/position, then1070 and1072 after the sequence worker's movement
fix. The sound should remain active during movement and cease at its actual
STOP cue. Distinguish a repeated start from repeated PCM inside one loop.

For mix acceptance, record all three user preferences and source fade calls;
capture per-bus RMS/peak plus listener pose and each source player's native
attenuation settings at the same original/Godot scene state. Include a near
and far stationary effect, voiced dialogue and the current intro theme. Keep
original save/build/scene provenance with the recording and compare channels
independently. Do not use one global gain cut or mute to obtain a pass.
