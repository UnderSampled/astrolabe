//! APM WAVEFORMAT/ADPCM header: corroborated by vgmstream ubi_apm.c and
//! ray2get's format description. Hype's decoder rounding remains an engine check.
use crate::model::audio::{AudioSamples, SampleLoop, Sound};
use crate::{model::io::FormatSupport, openspace::codec::FormatCodec};
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite, binrw};
use std::io::Cursor;

#[binrw]
#[brw(little)]
struct ApmHeader {
    #[br(assert(format_tag == 0x2000))]
    format_tag: u16,
    #[br(assert((1..=2).contains(&channels)))]
    channels: u16,
    #[br(assert(sample_rate > 0 && sample_rate <= 192000))]
    sample_rate: u32,
    pcm_byte_rate: u32,
    block_align: u16,
    #[br(assert(bits_per_sample == 4))]
    bits_per_sample: u16,
    #[br(assert(header_size == 0x50))]
    header_size: u32,
    #[brw(magic = b"vs12")]
    file_size: u32,
    frame_count: u32,
    loop_start: i32,
    loop_frames: u32,
    runtime_nibble: u32,
    states: [ChannelHeader; 2],
    additional_state: [u32; 7],
    #[brw(magic = b"DATA")]
    _end: (),
}
#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Default)]
struct ChannelHeader {
    predictor: i32,
    step_index: i32,
    first_data_word: u32,
}

const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const INDICES: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];
fn advance(code: u8, state: &mut ChannelHeader) -> i16 {
    let step = STEPS[state.step_index as usize];
    let mut delta = step >> 3;
    if code & 1 != 0 {
        delta += step >> 2;
    }
    if code & 2 != 0 {
        delta += step >> 1;
    }
    if code & 4 != 0 {
        delta += step;
    }
    if code & 8 != 0 {
        delta = -delta;
    }
    state.predictor = (state.predictor + delta).clamp(-32768, 32767);
    state.step_index = (state.step_index + INDICES[(code & 7) as usize]).clamp(0, 88);
    state.predictor as i16
}

pub fn read_apm(data: &[u8], name: impl Into<String>) -> Result<Sound> {
    let header = ApmHeader::read(&mut Cursor::new(data))?;
    ensure!(
        header.file_size as usize == data.len(),
        "APM declared file size differs from bounded input"
    );
    let channels = header.channels as usize;
    let payload = &data[100..];
    ensure!(
        payload.len().is_multiple_of(channels),
        "APM has incomplete channel pair"
    );
    ensure!(
        payload.len() <= 256 * 1024 * 1024,
        "APM exceeds decode limit"
    );
    let mut states = [ChannelHeader::default(); 2];
    for (channel, state) in states.iter_mut().enumerate().take(channels) {
        *state = header.states[channels - 1 - channel];
        ensure!(
            (-32768..=32767).contains(&state.predictor) && (0..=88).contains(&state.step_index),
            "invalid APM predictor state"
        );
    }
    let mut samples = Vec::with_capacity(payload.len() * 2);
    for pair in payload.chunks_exact(channels) {
        for shift in [4, 0] {
            for (channel, byte) in pair.iter().enumerate() {
                samples.push(advance((byte >> shift) & 15, &mut states[channel]));
            }
        }
    }
    // The stored count may describe an odd last frame. Some shipped APM headers
    // disagree substantially, so decoded payload length is retained and reported.
    let mut notes = vec![
        "APM PCM decode retained; exact historical ADPCM re-encoding remains unverified".into(),
    ];
    let physical_frames = samples.len() / channels;
    if header.frame_count as usize <= physical_frames
        && physical_frames - header.frame_count as usize <= 1
    {
        samples.truncate(header.frame_count as usize * channels);
    } else {
        notes.push(format!(
            "APM declared {} frames; payload contains {physical_frames}",
            header.frame_count
        ));
    }
    let loop_region = if header.loop_start >= 0 && header.loop_frames > 0 {
        let start = header.loop_start as u64;
        let end = start + u64::from(header.loop_frames);
        ensure!(
            end <= (samples.len() / channels) as u64,
            "APM loop exceeds decoded duration"
        );
        Some(SampleLoop {
            start_frame: start,
            end_frame: end,
        })
    } else {
        ensure!(
            header.loop_start == -1 && header.loop_frames == 0,
            "unsupported APM loop header"
        );
        None
    };
    Ok(Sound {
        name: name.into(),
        sample_rate: header.sample_rate,
        channels: header.channels,
        samples: AudioSamples::Pcm16(samples),
        loop_region,
        resource_id: None,
        surround: None,
        fidelity_notes: notes,
    })
}

/// Reconstruct a valid APM from current waveform values. This is an ADPCM
/// quantizer, not an inverse of every original lossy/ambiguous encoding choice.
pub fn write_apm(sound: &Sound) -> Result<Vec<u8>> {
    super::validate_sound(sound)?;
    ensure!(
        sound.surround.is_none(),
        "APM cannot represent an unverified surround component"
    );
    let samples = sound.interleaved_i16();
    let channels = sound.channels as usize;
    let frames = samples.len() / channels;
    let mut states = [ChannelHeader::default(); 2];
    // Start each predictor at the first desired sample; a zero code initially
    // preserves it exactly. This avoids imposing a long attack on edited audio.
    for (ch, state) in states.iter_mut().enumerate().take(channels) {
        state.predictor = samples.get(ch).copied().unwrap_or(0) as i32;
    }
    let initial_states = states;
    let mut loop_states = [ChannelHeader::default(); 2];
    let mut payload = Vec::with_capacity(frames.div_ceil(2) * channels);
    for frame in (0..frames).step_by(2) {
        for ch in 0..channels {
            let mut byte = 0;
            for half in 0..2 {
                if frame + half >= frames {
                    break;
                }
                if sound
                    .loop_region
                    .as_ref()
                    .is_some_and(|region| region.start_frame == (frame + half) as u64)
                {
                    loop_states[ch] = states[ch];
                }
                let target = i32::from(samples[(frame + half) * channels + ch]);
                let code = (0..16)
                    .min_by_key(|&code| {
                        let mut candidate = states[ch];
                        (i32::from(advance(code, &mut candidate)) - target).abs()
                    })
                    .unwrap();
                advance(code, &mut states[ch]);
                byte |= code << (4 - half * 4);
            }
            payload.push(byte);
        }
    }
    let mut channel_headers = [ChannelHeader::default(); 2];
    for ch in 0..channels {
        let mut state = initial_states[ch];
        let mut bytes = [0; 4];
        for (i, out) in bytes.iter_mut().enumerate() {
            *out = payload.get(ch + i).copied().unwrap_or(0);
        }
        state.first_data_word = u32::from_le_bytes(bytes);
        channel_headers[channels - 1 - ch] = state;
    }
    let mut loop_state_words = [0u32; 7];
    if let Some(region) = &sound.loop_region {
        loop_state_words[0] = (region.start_frame % 2) as u32;
        for (ch, state) in loop_states.iter().enumerate().take(channels) {
            let mut bytes = [0; 4];
            let start = (region.start_frame as usize / 2) * channels + ch;
            for (i, out) in bytes.iter_mut().enumerate() {
                *out = payload.get(start + i).copied().unwrap_or(0);
            }
            let index = 1 + (channels - 1 - ch) * 3;
            loop_state_words[index] = state.predictor as u32;
            loop_state_words[index + 1] = state.step_index as u32;
            loop_state_words[index + 2] = u32::from_le_bytes(bytes);
        }
    }
    let header = ApmHeader {
        format_tag: 0x2000,
        channels: sound.channels,
        sample_rate: sound.sample_rate,
        pcm_byte_rate: sound.sample_rate * u32::from(sound.channels) * 2,
        block_align: 1,
        bits_per_sample: 4,
        header_size: 0x50,
        file_size: u32::try_from(100 + payload.len())?,
        frame_count: u32::try_from(frames)?,
        loop_start: sound
            .loop_region
            .as_ref()
            .map(|region| i32::try_from(region.start_frame))
            .transpose()?
            .unwrap_or(-1),
        loop_frames: sound
            .loop_region
            .as_ref()
            .map(|region| u32::try_from(region.end_frame - region.start_frame))
            .transpose()?
            .unwrap_or(0),
        runtime_nibble: 0,
        states: channel_headers,
        additional_state: loop_state_words,
        _end: (),
    };
    let mut output = Cursor::new(Vec::with_capacity(100 + payload.len()));
    header.write(&mut output)?;
    let mut output = output.into_inner();
    output.extend(payload);
    Ok(output)
}

pub struct Apm {
    pub name: String,
}
impl FormatCodec for Apm {
    type Value = Sound;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "APM",
            reads: true,
            writes: true,
            notes: "PCM model; derived IMA ADPCM export is lossy, historical encoded bytes are unverified",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<Sound> {
        read_apm(bytes, self.name.clone())
    }
    fn write(&self, value: &Sound) -> Result<Vec<u8>> {
        write_apm(value)
    }
}
