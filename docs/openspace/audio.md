# Rust standalone audio

The audio slice restores the archived C# import coverage with native Rust decoding. It produces useful `Sound`, `SoundBank`, and `SoundDefinitions` values for the same Rete/Godot interfaces as other content. It is independent of level relocation state. All source data stays local.

## Interfaces and representation

`openspace::audio` exposes `read_apm`, `write_apm`, `read_bnm`, `write_bnm`, `read_ubi_mpeg`, `read_sound_definitions`, and `write_sound_definitions`. Binary readers take a bounded byte slice and an asset name; writers consume model values. The sound-definition reader takes decoded text (Hype sidecars are Latin-1). APM/BNM headers and descriptors use shared binrw declarations; the bitstream and adaptive quantization algorithms are explicit helpers.

A `Sound` owns a rate, channel count, interleaved integer or unclipped float samples, an optional frame-based loop, optional original bank resource identity, and fidelity notes. An optional `SurroundComponent` retains the independently decoded secondary stream and its 1RUS/2RUS relationship. It is not silently discarded or claimed to be a correctly mixed surround output. `Sound::validate()` is target-independent, and `Sound::interleaved_i16()` is an explicit waveform projection for playback targets. No original encoded stream, source pointer, or container offset is persisted.

`SoundBank` currently groups embedded sounds and named external references. Binary event/sequence records not yet interpreted are reported as unsupported; this is not a complete gameplay bank representation. `SoundDefinitions` exposes named CSB events and sample/theme resources, LCB event groups/bank selections/level selections and source identity bindings, and SIF environment/material enumerations. Scene/perso event binding remains a later domain. Detailed CSB flags, theme transitions, and binary-event compilation are still unsupported and reported.

Rete persists these exact values through its normal schema envelope. PCM quantization for WAV is a Godot operation; Rete keeps float values without silently reducing them to 16-bit.

## Evidence and choices

Raymap was consulted first, but no standalone APM/BNM decoder was found in its scripts. The main corroborating implementations are [`vgmstream` APM](../../reference/vgmstream/src/meta/ubi_apm.c), its [BNM handling](../../reference/vgmstream/src/meta/ubi_sb.c), and its [Ubisoft MPEG helper](../../reference/vgmstream/src/coding/libs/ubi_mpeg_helpers.c), which documents reverse engineering of MPGMXBVR.DLL. The archived [C# audio code](../../reference/astrolabe-csharp/src/Astrolabe.Core/FileFormats/Audio) and [Ray2Get APM description](../../reference/ray2get/apm_format_description.txt) are valuable additional evidence. The original engine/decompilation remains authoritative; an actual engine audio comparison has not been performed.

- APM is byte-interleaved, high-nibble-first IMA audio with reverse-ordered initial channel states. The meaningful frame count can exclude one terminal padding nibble. The C# decoder exposed that padding as an additional frame; Rust honors the declared duration when consistent with payload bounds.
- APM loop start and length agree with shipped CSB `SetLoopData` and BNM fields. All 69 embedded looping APM descriptors satisfy `start + length == APM frame count`, including a start of one frame. Standalone loop predictor-state copies corroborate this interpretation. Bank flags decide whether an authored APM loop is enabled. New APM encoding regenerates loop predictor state and nibble parity from current samples; it does not replay source header state.
- Ubisoft MPEG uses unaligned frames and a fixed allocation table. The adapter reconstructs a standard Layer-II envelope only in memory for Symphonia 0.5.5. Stereo uses a 256 kbps/48 kHz envelope; mono uses a standards-valid 192 kbps/48 kHz envelope because strict decoders reject the archived C# decoder's 256 kbps mono header. These envelope rates select the appropriate table; the useful sound retains the original bank sample rate, with no resampling.
- The initial 480-frame discard follows vgmstream's observed delay. BNM's MPDX prefix determines the meaningful duration, excluding final MPEG frame padding. This improves on the archived C# path, which ignored that count. Exact original-engine timing still needs runtime/decompilation confirmation.
- Both source implementations skipped surround's second stream. Rust independently decodes and retains it. The engine appears to combine components before synthesis, so simply mixing the two decoded waveforms is not asserted to be equivalent.

## Reconstruction coverage

APM writing is a working adaptive quantizer over current PCM values, including generated header sizes, channel states and loops. Exact historical ADPCM byte recovery is not claimed: decoding can lose quantization ambiguity and clipped predictor information, and the original encoder decisions are not cached. More useful codec-domain structure may be required for that later fidelity goal.

The BNM writer generates PCM-only resource banks, deriving every descriptor/payload offset and size. Synthetic bank edits and Rete reload have byte-exact reconstruction tests; PCM data itself is preserved exactly. It rejects unresolved event graphs, external references, loops whose PCM container rules are not yet verified, and float/MPEG content requiring an explicit encoding decision. It does not claim that a decoded real bank can already be reconstructed completely.

LCB and SIF writers regenerate supported named definitions from current semantic values. Tests compare re-imported meaning and deliberate edits; source whitespace/comments are not preserved. CSB writing refuses incomplete behavior semantics. Exact Ubi-MPEG re-encoding, historical compressed-media identity, full bank event reconstruction, surround mixing, and original-engine APM rounding remain explicit research gaps. None is hidden behind encoded-blob replay.

## Validation

From the repository root, these bounded commands exercise this slice:

```sh
timeout 540s cargo test --release --test audio -- --include-ignored --nocapture
timeout 540s cargo test --release --lib local_layer2_synthesis_matches_ffmpeg -- --ignored --nocapture
```

These are focused developer commands. Development validation selects affected audio checks plus mandatory safeguards; the full release matrix retains the complete audio corpus. Bound individual tests and account for child cleanup under the [current testing policy](../astrolabe/testing.md); there is no universal ten-minute release-suite ceiling. The optional local tests require the supplied `disc/`; the independent MPEG comparison also requires FFmpeg, solely as a test oracle. Production import has no subprocess decoder or C# dependency.

The local corpus passes:

| Content | Observed coverage |
| --- | --- |
| Standalone APM | 39 files, all decoded and persisted/reloaded through Rete |
| BNM | 199 banks, 2,573 embedded sounds, 224 external references |
| Embedded payload types | 1,515 MPEG, 1,030 APM, 28 PCM |
| Surround | 264 retained secondary channel groups |
| Sound sidecars | 87 CSB/LCB/SIF files; named content inspected; LCB/SIF semantic reconstruction |

Tests include changed samples/rates/durations, derived bank layout after resizing and inserting sounds, odd-frame APM loop state, malformed bounds, actual Rete envelope persistence, and explicit writer rejection of unsupported semantics. Three real APM fixtures have first-4,096-sample hashes independently produced by the archived C# decoder compiled in a temporary project. A real MPEG voice sample's 233,376 decoded samples were compared with FFmpeg: RMSE `0.0000124733`, maximum difference `0.0000318494` in normalized float amplitude. This is synthesis agreement, not a claim that the original engine or surround mixer was exercised.

The complete audio integration invocation, including all local corpus tests, takes approximately 7 seconds after compilation on this machine; recent rebuild-plus-test invocations took about 14 seconds. The independent Layer-II oracle test takes approximately 0.1 seconds after compilation. Godot resource validity, actual playback, and direct-versus-Rete route evidence are recorded by the application-level validation, not inferred from successful decoding here.
