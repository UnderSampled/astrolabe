//! Ubisoft's bit-packed Layer-II framing. Tables and frame traversal are
//! corroborated by vgmstream's ubi_mpeg_helpers.c (reverse engineered from
//! MPGMXBVR.DLL). Symphonia handles standard Layer-II sample synthesis.
use crate::model::audio::{AudioSamples, Sound, SurroundComponent};
use anyhow::{Context, Result, bail, ensure};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{CODEC_TYPE_MP2, CodecParameters, DecoderOptions},
    formats::Packet,
};

const ALLOCATION_BITS: [usize; 27] = [
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2,
];
const Q_BITS: [i32; 17] = [-5, -7, 3, -10, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const Q_LOW: [usize; 15] = [0, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const Q_MIDDLE: [usize; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16];
const Q_HIGH: [usize; 7] = [0, 1, 2, 3, 4, 5, 16];
const Q_TOP: [usize; 3] = [0, 1, 16];

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}
impl Bits<'_> {
    fn remaining(&self) -> usize {
        self.data.len() * 8 - self.pos
    }
    fn read(&mut self, count: usize) -> Result<u32> {
        ensure!(
            count <= 32 && count <= self.remaining(),
            "truncated Ubi-MPEG frame at bit {}",
            self.pos
        );
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u32::from((self.data[self.pos / 8] >> (7 - self.pos % 8)) & 1);
            self.pos += 1;
        }
        Ok(value)
    }
    fn rest_is_zero(&self) -> bool {
        (self.pos..self.data.len() * 8).all(|p| (self.data[p / 8] >> (7 - p % 8)) & 1 == 0)
    }
}
struct FrameBits {
    bytes: Vec<u8>,
    pos: usize,
}
impl FrameBits {
    fn write(&mut self, count: usize, value: u32) -> Result<()> {
        ensure!(
            self.pos + count <= self.bytes.len() * 8,
            "Ubi-MPEG frame exceeds fixed Layer-II envelope"
        );
        for i in (0..count).rev() {
            self.bytes[self.pos / 8] |= (((value >> i) & 1) as u8) << (7 - self.pos % 8);
            self.pos += 1;
        }
        Ok(())
    }
    fn copy(&mut self, input: &mut Bits<'_>, count: usize) -> Result<u32> {
        let value = input.read(count)?;
        self.write(count, value)?;
        Ok(value)
    }
}
fn qbits(band: usize, allocation: u32) -> Result<i32> {
    let table: &[usize] = match band {
        0..=2 => &Q_LOW,
        3..=10 => &Q_MIDDLE,
        11..=22 => &Q_HIGH,
        _ => &Q_TOP,
    };
    let index = table
        .get(allocation as usize - 1)
        .context("invalid MPEG quantization allocation")?;
    Ok(Q_BITS[*index])
}
fn transform_frame(input: &mut Bits<'_>) -> Result<Option<(Vec<u8>, u16)>> {
    if input.remaining() < 16 {
        ensure!(
            input.rest_is_zero(),
            "nonzero incomplete Ubi-MPEG terminal frame"
        );
        return Ok(None);
    }
    let mut sync = 0;
    for _ in 0..32 {
        if input.remaining() < 12 {
            break;
        }
        sync = input.read(12)?;
        if sync != 0 {
            break;
        }
    }
    if sync == 0 && input.rest_is_zero() {
        input.pos = input.data.len() * 8;
        return Ok(None);
    }
    ensure!(
        sync == 0xfff,
        "invalid Ubi-MPEG sync {sync:03x} at bit {}",
        input.pos.saturating_sub(12)
    );
    let mode = input.read(4)?;
    let channel_mode = mode >> 2;
    let channels = if channel_mode == 3 { 1 } else { 2 };
    let joint_bound = if channel_mode == 1 {
        [4, 8, 12, 16][(mode & 3) as usize]
    } else {
        27
    };
    let mut output = FrameBits {
        bytes: vec![0; if channels == 1 { 0x240 } else { 0x300 }],
        pos: 0,
    };
    // MPEG-1 Layer-II, 256 kbps at nominal 48 kHz selects the same allocation table.
    // Nominal rate is solely a decoder envelope: Sound keeps the bank's real rate.
    output.write(
        32,
        (if channels == 1 {
            0xfffd_a400
        } else {
            0xfffd_c400
        }) | (mode << 4)
            | 12,
    )?;
    let mut allocations = [[0u32; 2]; 27];
    let mut selectors = [[0u32; 2]; 27];
    for band in 0..27 {
        let count = if band < joint_bound { channels } else { 1 };
        for allocation in allocations[band].iter_mut().take(count) {
            *allocation = output.copy(input, ALLOCATION_BITS[band])?;
        }
        if band >= joint_bound {
            allocations[band][1] = allocations[band][0];
        }
    }
    for band in 0..27 {
        for ch in 0..channels {
            if allocations[band][ch] != 0 {
                selectors[band][ch] = output.copy(input, 2)?;
            }
        }
    }
    for band in 0..27 {
        for ch in 0..channels {
            if allocations[band][ch] == 0 {
                continue;
            }
            let count = match selectors[band][ch] {
                0 => 3,
                1 | 3 => 2,
                2 => 1,
                _ => unreachable!(),
            };
            for _ in 0..count {
                output.copy(input, 6)?;
            }
        }
    }
    for _ in 0..12 {
        for (band, allocation) in allocations.iter().enumerate() {
            let count = if band < joint_bound { channels } else { 1 };
            for &value in allocation.iter().take(count) {
                if value == 0 {
                    continue;
                }
                let bits = qbits(band, value)?;
                for _ in 0..if bits < 0 { 1 } else { 3 } {
                    output.copy(input, bits.unsigned_abs() as usize)?;
                }
            }
        }
    }
    Ok(Some((output.bytes, channels as u16)))
}

fn append_samples(
    output: &mut Vec<f32>,
    other_capacity: usize,
    samples: &[f32],
    limit: usize,
) -> Result<()> {
    let total = output
        .len()
        .checked_add(samples.len())
        .and_then(|n| n.checked_add(other_capacity))
        .and_then(|n| n.checked_mul(std::mem::size_of::<f32>()))
        .context("Ubi-MPEG decoded allocation overflow")?;
    ensure!(
        total <= limit,
        "Ubi-MPEG cumulative decoded allocation exceeds {limit} bytes"
    );
    // Avoid geometric reserve growth beyond the budget; frame synthesis itself
    // uses a fixed, small Layer-II frame buffer.
    output
        .try_reserve_exact(samples.len())
        .context("allocate bounded Ubi-MPEG samples")?;
    output.extend_from_slice(samples);
    Ok(())
}

/// Input starts at the optional 1RUS/2RUS marker, after the MPDX sample-count word.
/// Surround's secondary component is retained; its engine-specific mixing is
/// explicitly unverified and must not be silently described as full surround.
pub fn read_ubi_mpeg(data: &[u8], sample_rate: u32, name: impl Into<String>) -> Result<Sound> {
    read_ubi_mpeg_with_limit(data, sample_rate, name, 512 * 1024 * 1024)
}

/// The bank's remaining budget includes main and surround waveform allocations.
/// Counts come from decoded frames, never from an untrusted declared duration.
pub(super) fn read_ubi_mpeg_with_limit(
    data: &[u8],
    sample_rate: u32,
    name: impl Into<String>,
    limit: usize,
) -> Result<Sound> {
    ensure!(
        data.len() <= 256 * 1024 * 1024,
        "Ubi-MPEG exceeds decode limit"
    );
    ensure!(
        (1..=192000).contains(&sample_rate),
        "invalid Ubi-MPEG sample rate"
    );
    let surround = data.starts_with(b"1RUS") || data.starts_with(b"2RUS");
    let mut input = Bits {
        data,
        pos: if surround { 32 } else { 0 },
    };
    let mut parameters = CodecParameters::new();
    parameters.for_codec(CODEC_TYPE_MP2);
    let registry = symphonia::default::get_codecs();
    let mut main_decoder = registry.make(&parameters, &DecoderOptions::default())?;
    let mut side_decoder = registry.make(&parameters, &DecoderOptions::default())?;
    let mut samples = Vec::new();
    let mut side_samples = Vec::new();
    let mut channels = 0;
    let mut side_channels = 0;
    let mut frame_count = 0;
    while let Some((frame, current_channels)) = transform_frame(&mut input)? {
        if channels == 0 {
            channels = current_channels;
        }
        ensure!(
            channels == current_channels,
            "channel count changes inside Ubi-MPEG sound"
        );
        let decoded = main_decoder.decode(&Packet::new_from_boxed_slice(
            0,
            frame_count * 1152,
            1152,
            frame.into_boxed_slice(),
        ))?;
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        append_samples(
            &mut samples,
            side_samples.capacity(),
            buffer.samples(),
            limit,
        )?;
        if surround {
            let Some((side_frame, current_side_channels)) = transform_frame(&mut input)? else {
                bail!("missing Ubi-MPEG surround component");
            };
            if side_channels == 0 {
                side_channels = current_side_channels;
            }
            ensure!(
                side_channels == current_side_channels,
                "surround component changes channel count"
            );
            let decoded = side_decoder.decode(&Packet::new_from_boxed_slice(
                0,
                frame_count * 1152,
                1152,
                side_frame.into_boxed_slice(),
            ))?;
            let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
            buffer.copy_interleaved_ref(decoded);
            append_samples(
                &mut side_samples,
                samples.capacity(),
                buffer.samples(),
                limit,
            )?;
        }
        frame_count += 1;
    }
    ensure!(channels != 0, "no Ubi-MPEG frames found");
    let delay = usize::from(channels) * 480;
    ensure!(
        samples.len() >= delay,
        "Ubi-MPEG stream shorter than encoder delay"
    );
    samples.drain(..delay);
    let mut notes = vec![
        "Ubi-MPEG retains unclipped decoded samples; exact source re-encoding is not implemented"
            .into(),
        "480-frame decoder-delay discard follows vgmstream; exact engine timing remains unverified"
            .into(),
    ];
    let component = if surround {
        let delay = usize::from(side_channels) * 480;
        ensure!(
            side_samples.len() >= delay,
            "surround component shorter than encoder delay"
        );
        side_samples.drain(..delay);
        notes.push("Surround component is retained independently; engine pre-synthesis mixing is unverified".into());
        Some(SurroundComponent {
            channels: side_channels,
            samples: AudioSamples::Float32(side_samples),
            relationship: String::from_utf8_lossy(&data[..4]).into_owned(),
        })
    } else {
        None
    };
    Ok(Sound {
        name: name.into(),
        sample_rate,
        channels,
        samples: AudioSamples::Float32(samples),
        loop_region: None,
        resource_id: None,
        surround: component,
        fidelity_notes: notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpeg_main_and_surround_appends_check_one_budget_before_reserve() {
        let mut main = Vec::new();
        let mut side = Vec::new();
        append_samples(&mut main, side.capacity(), &[0.25; 4], 32).unwrap();
        append_samples(&mut side, main.capacity(), &[0.5; 4], 32).unwrap();
        let before = (main.clone(), main.capacity(), side.clone(), side.capacity());
        let error = append_samples(&mut main, side.capacity(), &[0.75], 32).unwrap_err();
        assert!(error.to_string().contains("cumulative decoded allocation"));
        assert_eq!(main.capacity(), before.1);
        assert_eq!(side.capacity(), before.3);
        assert_eq!(main, before.0);
        assert_eq!(side, before.2);
    }

    #[test]
    #[ignore = "requires local disc and ffmpeg; run within the suite deadline"]
    fn local_layer2_synthesis_matches_ffmpeg() {
        use std::io::Write;
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
        let path = game.join("Gamedata/World/Sound/English/Bnk_155.bnm");
        let bank = std::fs::read(path).unwrap();
        let word = |at: usize| u32::from_le_bytes(bank[at..at + 4].try_into().unwrap()) as usize;
        let descriptor = word(12);
        let start = word(descriptor + 16);
        let size = word(descriptor + 12);
        let payload = &bank[start + 4..start + size];
        assert!(!payload.starts_with(b"1RUS") && !payload.starts_with(b"2RUS"));
        let sound = read_ubi_mpeg(payload, 44100, "reference").unwrap();
        let mut input = Bits {
            data: payload,
            pos: 0,
        };
        let mut mp2 = tempfile::NamedTempFile::new().unwrap();
        while let Some((frame, _)) = transform_frame(&mut input).unwrap() {
            mp2.write_all(&frame).unwrap();
        }
        mp2.flush().unwrap();
        let output = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-nostdin", "-i"])
            .arg(mp2.path())
            .args(["-f", "f32le", "-acodec", "pcm_f32le", "pipe:1"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected: Vec<f32> = output
            .stdout
            .as_chunks::<4>()
            .0
            .iter()
            .map(|v| f32::from_le_bytes(*v))
            .skip(480 * sound.channels as usize)
            .collect();
        let AudioSamples::Float32(actual) = sound.samples else {
            unreachable!()
        };
        assert_eq!(actual.len(), expected.len());
        let square_error: f64 = actual
            .iter()
            .zip(&expected)
            .map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2))
            .sum();
        let rmse = (square_error / actual.len() as f64).sqrt();
        let peak_difference = actual
            .iter()
            .zip(expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        eprintln!(
            "Layer-II independent decoder: {} samples, RMSE {rmse}, max difference {peak_difference}",
            actual.len()
        );
        assert!(rmse < 0.0001, "independent decoder mismatch: RMSE {rmse}");
    }
}
