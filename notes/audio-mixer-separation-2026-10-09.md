# Source fades and user audio preferences, 2026-10-09

Follow-up to audio channel commit `32021599c90985f5a6ebfcc31f14dd42dd5a8933`
on branch `codex/audio-channel-semantics-20261009`, isolated worktree
`output/worktrees/audio-channel-semantics-20261009`. Source-only implementation;
no tests, builds, native imports, Godot or game processes launched. Runtime is
reserved by Kip for Luna. Support stays provisionally85, Rete62; integration
must choose the combined support version. The stale support test assertion84
from the first commit is corrected to85 here.

## Concrete bugs corrected

The previous AI fade implementation read the user bus gain as its starting
source level and wrote its target into that same bus. `FadeAll(127, duration)`
therefore raised the user's chosen preferences to full gain. A source starting
at127 with an Effects preference61 also incorrectly started its fade at61.
The original source stores these as separate values and initializes the source
fade word to127.

`audio/mixer.gd` is a tree-owned native component with three ordinary source
properties and three preference properties. The existing Astrolabe Music,
Effects and Voice buses hold only user gain. Existing settings values, slider
ranges and settings file are retained; applying them now publishes through the
component. Brain-owned fade requests still use native tree-owned Tweens and
survive their requesting Brain, but update the source properties.

A global source bus gain would affect static requests incorrectly. Source fade
and object changes refresh only dynamic players. A user preference publication
refreshes all matching active players, capturing their current source/object
gain and static spatial pose. This corresponds to two distinct original
publication paths; source+object gain is not permanently captured across a
preference change. Source updates never restart players.

Proc386 previously retained its raw DWORD correctly but then used that DWORD
as an unbounded floating amplitude. Native gain now uses the source parameter
byte: exact source127/object127 preserves the preference byte; other pairs use
low32(object*source*preference), signed SAR14, then low AL, without a clamp.
Splitting the low32 product also avoids Godot signed64 overflow for two full
DWORDs. For preference127/source127, object90 yields88, raw0x1000007f yields125
while retaining its override, and raw0xffffffff yields255. Only exact127 removes
the override. This does not introduce a blanket volume reduction.

The native player compensates for its existing user bus so the preference is
applied once: parameterByte/preference at the player and preference/127 at the
bus. An explicitly authored alternate bus receives parameterByte/127 directly.
The final byte-to-amplitude mapping remains the existing /127 approximation
pending the external driver's contract; the integer parameter itself is exact
for the supported plain-event path. Converted resource gain still uses its
existing floating representation. No source memory layout, scheduler, driver
interpreter or content loader is added.

Requests expose `parameter_volume`, `source_volume_word`, `preference_volume`,
`object_volume_word`, module and native bus for diagnostics in addition to
existing start/completion signals. The published metadata changes with the
corresponding gain publication; observers should copy it when recording an
instantaneous trace. Static values do not change on ordinary source updates.

## Original-source authority

Dedicated reconstruction worker:
`/home/stephen/Documents/Codex/2026-10-09/task-3/hype-audio-mixer`, branch
`reconstruction/audio-mixer-task3-20261009`, baseline `e24540e`.

- `ff74815ccd6dd45c4fe35f251dfb81d1b93e88bf`,
  `src/sound/audio_mixer.c`: clean004eb820 full source DWORD getter,
  004eb840 full first-match object getter,004ebf50 current object parameters,
  and004ec090 exact preference-byte/product/shift/narrowing order.
- `90fbd830932b4be44d89fdd5ddadc973ea229347`,
  `src/sound/audio_mixer_registration.c` and `audio_preferences.c`:
  004ecfb0 initial preference127;004ed0e0 independent source+28 preference
  pointer and+38 fade DWORD initialized127;00432bd0 separates gameplay
  effects/voice/music sources while menu sources share preferences;
  004ed2f0 publishes preferences;004eda20 reads source fade state;
  00428fa0/00428fd0/00429000 bound public preference values to127.
- `research/worker-deltas/audio-mixer-handoff.md` includes independently
  verified004ec300/004ec660/004f7c20 contracts.004ec660 freshly captures
  parameters and remixes all active requests sharing a preference pointer,
  without checking the dynamic word. This is distinct from already-clean
  `src/sound/microphone_publication.c`004ec710, whose ordinary frame path
  updates only dynamic events. No helper complete-body credit was claimed.
- Already-clean `src/sound/theme_request.c`004faa60 initially applies
  `(themeByte*parameterByte)>>7`. Active/same-theme and subsequent part paths
  differ. `resource_transition.c`004f3fd0 forwards sample resources and
  parameters to the external driver, so sample resource gain and final
  normalization cannot be inferred from this boundary alone.

The worker recorded matching complete spans/hashes from both original PE
profiles in `audio-mixer-evidence.json`. Reference files and original assets
remain unchanged by Astrolabe work.

## Prepared validation and acceptance gaps

New ignored native test
`godot::sound::tests::native_source_fades_preserve_preferences_and_publish_byte_gain`
uses maintained support/settings, native looped PCM and AudioEffectCapture.
Declared media/AI release inventory: engine deadline20s, manifest30s. It checks
literal source-byte cases, independent preferences through FadeAll, dynamic
versus static publication, preference recapture of current source values,
full DWORD retention, no gain-triggered restarts, and the native PCM RMS ratio
when preference changes127 to64. It applies the maintained settings route
without saving preferences. This checks mixer output, not audible original
driver parity. Existing source fade and music fixtures are updated for the
separate state and integer object gain. No execution result exists yet.

Luna should parser-check the maintained scripts first and then run the new
test, the first channel test, source fade/music fixtures and Proc386/412 checks
with required safeguards, once Kip releases the runtime slot. Copy bus/source
levels and request metadata at each transition. The actual New Game baseline
from `output/runtime-validation/luna-20261009/LUNA-BASELINE.md` used dummy audio;
it cannot establish audible mix correctness and did not reach later dialogue
or shield acceptance. The gate's authored event1069 loop must still follow its
real actor STOP1070 cue; this commit does not disable events or force closure.

Remaining loudness uncertainties: external driver byte-to-amplitude law,
sample resource volume/SetVolable/SetPanable/SetSpacable interpretation,
distance attenuation and units, sector sound propagation/reverb, exact event
modulation callback behavior in the converted API, and active theme parameter
transitions. Native spatial defaults remain unchanged because executable
004ebf50 supplies coordinates rather than a distance curve;004f5d50 and
004f7fe0 dispatch/import the external sound implementation. These must be
resolved with the dedicated source worker and controlled runtime evidence,
not a global gain cut. The full user-reported New Game mix remains unaccepted.

## Callback boundary and next source request

Source follow-up `7026a1980802d329222ceb953da54e163fee8792` completes the
reserved callback/driver group in `audio_driver_callbacks.c` and
`audio_driver_dispatch.c`.00432540..00432630 publish fixed65536-scaled position
and velocity.00432660 supplies byte0.00432670 supplies velocity magnitude for
its first output and fixed unity65536 for volume/pan.00432760 selects a sector
or dynamics-derived word;00432830 supplies Boolean sector eligibility using
the currently selected microphone position actor, with its source-defined
default1 when no matching link exists. These establish no distance curve.

The shared model already retains ordered `SectorSoundPropagation` links and
their signed volume words. The Godot scene/audio adapter does not yet publish
those links. Driver interpretation of the eligibility word must be established
before mapping it to native player gain; its Boolean callback does not justify
using the raw link magnitude as a floating amplitude.

Read-only disc evidence: `disc/DLL/snd_cpa.ini` sets
`SXD_DIST_SATUR=2`, `SXD_DIST_FADE=16`, `DLL_Sxd Default=WAVX2` and conditional
`File0=WAVMW`, `Test0=_SND_fn_bTestSnd_WinNT@0`. The actual original files are
`/home/stephen/Projects/hype-decomp/disc/DLL/WAVx2BVR.dll` and
`WAVMWBVR.DLL`. Both expose `_SND_fn_lPlaySxd@20`,
`_SND_fn_bSetParamSxd@8`, `_SND_fn_vSetMicroParamSxd@8`,
`_SND_fn_vParam3Dto2D@8`, `_SND_fn_vSetDefaultRollOffSxd@4` and
`_SND_fn_bSetResourceStaticVolumeSxd@8`. The dedicated source worker is
inspecting these and selection/loading statically. Which DLL the original
runtime selected remains unobserved. Config variable names alone are not a
verified law and no attenuation setting was changed from them.

The shared native mixer binding is published before its deferred insertion
under Root, so UserSettings can create it during autoload initialization without
adding a child to a parent still setting up children or creating one component
per preference group. Native buses are available on component construction.
No runtime validation of this initialization or the rest of the patch exists.
