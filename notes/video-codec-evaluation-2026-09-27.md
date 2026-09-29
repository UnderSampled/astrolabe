# FFV1 and Ut Video evaluation — 2026-09-27

## Decision

Keep FFV1 as the default codec for Orrery's continuous session recordings.
Ut Video encoded and decoded this capture faster, but its files were 42% larger
in the practical RGB-preserving path. The existing FFV1 path encoded at a median
of 177 frames/s for 30-fps footage (5.9× real time) and decoded at 386 frames/s.
That leaves useful measured headroom without paying the observed storage cost.

Reconsider Ut Video if live captures show FFV1 encoder backlog or dropped frames
under game load. This experiment re-encoded completed footage; it did not measure
capture while Hype and Orrery ran together.

## Measurements

The source was the 1024×768 segment of a corrected Hype manor recording:
767 indexed frames, 30 fps, 25.566 seconds, 406,343,051 bytes. Source SHA-256:
`03f706385005ad9cd1456d935131b45c7bcf92d686e47a7929e747749f66b838`.

Each row is three sequential full-file encode and decode runs. The table reports
the median elapsed-time throughput. Encode timing includes source decode,
pixel-format conversion, encoding, and Matroska muxing. Decode timing is a
separate decode-to-null pass. FFmpeg automatic threading was used; outputs were
written on the workspace NVMe filesystem.

| Evaluation | FFV1 | Ut Video | Frame check |
| --- | ---: | ---: | --- |
| Practical RGB-preserving path | BGR0, 387.52 MiB; encode 177 fps; decode 386 fps | GBRP, 550.70 MiB (**+42.1%**); encode 345 fps; decode 2,531 fps | All 767 visible BGR pixels matched exactly |
| Shared YUV 4:4:4 samples | 317.53 MiB; encode 210 fps; decode 436 fps | 500.13 MiB (**+57.5%**); encode 342 fps; decode 2,591 fps | All 767 YUV samples matched exactly |

In the practical path, Ut Video encoded about 1.9× faster and decoded about
6.6× faster, at an extra 171,111,348 bytes for this segment. Both codecs
preserved every visible source pixel. The Ut Video output's unused BGR0 padding
byte differs, so frame checks compare BGR24 color channels rather than that
non-image byte.

The second run controls for pixel format by converting each source frame from
RGB to YUV 4:4:4 through the same FFmpeg conversion path, then encoding those
samples with each codec. Ut Video remained about 1.6× faster to encode and 6.0×
faster to decode, while using 57.5% more space. This is a codec comparison on a
shared sample representation; RGB-to-YUV conversion itself is not lossless
relative to the original RGB capture. It is not a proposed recording mode.

The codec speeds are material-specific, not universal: these results describe
one gameplay/screen-capture segment, the tested FFmpeg build, and its automatic
thread settings. No audio stream was present, so audio handling was not tested.

## Compatibility and evidence

FFV1 has a public IETF specification describing it as lossless and intra-frame,
and its specification defines its Matroska mapping. It also defines slice CRC
support for corruption detection. The current Orrery invocation requests FFV1
level 3 but does not set the CRC option explicitly; set that choice explicitly
if CRC behavior is part of the recording contract. [RFC 9043](https://www.rfc-editor.org/rfc/rfc9043)

Ut Video's author describes it as a fast lossless codec. The current local
FFmpeg build can encode and decode it. The standalone suite lists Windows VCM
and DMO interfaces and is GPL-2-or-later, so check those integration and
distribution constraints before relying on it outside our FFmpeg-based tools.
[Ut Video Codec Suite](https://github.com/umezawatakeshi/utvideo)

The production recorder supplies BGR0 frames. This FFmpeg build's FFV1 encoder
does not accept 8-bit GBRP and falls back to BGR0; Ut Video accepts GBRP but not
BGR0. The practical Ut Video measurement therefore includes a BGR0-to-GBRP
conversion. That conversion preserved every visible RGB value, but it means the
practical comparison uses each codec's supported RGB representation rather than
the same packed bytes. FFmpeg documents the FFV1 encoder options, including its
coder, context, and slice-CRC settings. [FFmpeg codec documentation](https://ffmpeg.org/ffmpeg-codecs.html#ffv1)

## Reproduction records

Benchmark command families, FFmpeg version (`n9.0.2`), source/output hashes,
per-run timings, stream metadata, and full-frame comparison outcomes are saved
locally under the ignored directory
`TestResults/codec-evaluation-ffv1-utvideo-20260927/` in `evaluation.json` and
`repeat-timings.json`. The original source hash was checked again after the
benchmark. All four output files remain there for local playback or inspection.
