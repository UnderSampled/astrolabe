//! BNM v0/v0x200 resource tables. Field positions corroborated by vgmstream
//! ubi_sb.c's PC configuration and local Hype banks. Non-audio events remain
//! explicitly unsupported; the reader does not claim a complete bank model.
use crate::model::audio::{AudioReconstructionGap, AudioSamples, ExternalSound, Sound, SoundBank};
use crate::{model::io::FormatSupport, openspace::codec::FormatCodec};
use anyhow::{Context, Result, bail, ensure};
use binrw::{BinRead, BinWrite, binrw};
use std::io::{Cursor, Seek, SeekFrom};

#[binrw]
#[brw(little)]
struct BankHeader {
    #[br(assert(version == 0 || version == 0x200))]
    version: u32,
    events_offset: u32,
    events_count: u32,
    sounds_offset: u32,
    sounds_count: u32,
    mpdx_offset: u32,
    midi_offset: u32,
    pcm_offset: u32,
    apm_offset: u32,
    streamed_offset: u32,
    file_size: u32,
}
#[binrw]
#[brw(little)]
struct AudioDescriptor {
    resource_id: u32,
    resource_type: u32,
    embedded: u32,
    payload_size: u32,
    payload_offset: u32,
    volume: u32,
    unknown_controls: [u32; 5],
    streamed: u32,
    loop_flag: u32,
    loop_start: u32,
    sample_extent: u32,
    sample_rate: u32,
    bits_per_sample: u16,
    channels: u16,
    stream_type: u32,
    name: [u8; 20],
}
fn bounded(data: &[u8], start: u32, len: u32) -> Result<&[u8]> {
    let start = start as usize;
    let end = start
        .checked_add(len as usize)
        .context("BNM range overflow")?;
    data.get(start..end)
        .context("BNM payload outside bounded bank")
}
fn mpdx_start(data: &[u8], offset: u32) -> bool {
    let Some(prefix) = data.get(offset as usize..).and_then(|s| s.get(4..8)) else {
        return false;
    };
    prefix == b"1RUS" || prefix == b"2RUS" || (prefix[0] == 0xff && prefix[1] & 0xf0 == 0xf0)
}
fn latin1(data: &[u8]) -> String {
    data.iter()
        .take_while(|&&b| b != 0)
        .map(|&b| char::from(b))
        .collect()
}

// Includes every owned waveform and surround component, even when descriptors
// share one encoded range. Separate from the encoded input/table bounds.
const MAX_BANK_DECODED_BYTES: usize = 512 * 1024 * 1024;

fn claim_decoded_bytes(remaining: &mut usize, bytes: usize, name: &str) -> Result<()> {
    *remaining = remaining
        .checked_sub(bytes)
        .with_context(|| format!("BNM cumulative decoded allocation limit exceeded at {name}"))?;
    Ok(())
}

pub fn read_bnm(data: &[u8], name: impl Into<String>) -> Result<SoundBank> {
    read_bnm_with_limit(data, name, MAX_BANK_DECODED_BYTES)
}

fn read_bnm_with_limit(data: &[u8], name: impl Into<String>, limit: usize) -> Result<SoundBank> {
    let mut remaining = limit;
    let mut cursor = Cursor::new(data);
    let header = BankHeader::read(&mut cursor)?;
    ensure!(
        header.file_size as usize == data.len(),
        "BNM size differs from header"
    );
    ensure!(
        header.sounds_count <= 100000,
        "BNM descriptor limit exceeded"
    );
    let descriptor_size = if header.version == 0x200 { 96 } else { 92 };
    bounded(
        data,
        header.sounds_offset,
        header
            .sounds_count
            .checked_mul(descriptor_size)
            .context("BNM descriptor table overflow")?,
    )?;
    bounded(
        data,
        header.events_offset,
        header
            .events_count
            .checked_mul(32)
            .context("BNM event table overflow")?,
    )?;
    let mut bank = SoundBank {
        name: name.into(),
        sounds: vec![],
        external_sounds: vec![],
        events: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    if header.events_count > 0 {
        bank.reconstruction_gaps
            .push(AudioReconstructionGap::BinaryEvents {
                count: header.events_count,
            });
        bank.fidelity_notes.push(format!("{} binary event records require semantic interpretation; full-bank reconstruction is unsupported", header.events_count));
    }
    for index in 0..header.sounds_count {
        cursor.seek(SeekFrom::Start(u64::from(
            header.sounds_offset + index * descriptor_size,
        )))?;
        let descriptor = AudioDescriptor::read(&mut cursor)?;
        if descriptor.resource_type != 1 {
            bank.reconstruction_gaps
                .push(AudioReconstructionGap::ResourceType {
                    resource_id: descriptor.resource_id,
                    kind: descriptor.resource_type,
                });
            bank.fidelity_notes.push(format!(
                "resource {} type {} has no semantic codec yet",
                descriptor.resource_id, descriptor.resource_type
            ));
            continue;
        }
        let name = latin1(&descriptor.name);
        ensure!(
            (1..=2).contains(&descriptor.channels),
            "invalid BNM channels for {name}"
        );
        ensure!(
            descriptor.sample_rate > 0 && descriptor.sample_rate <= 192000,
            "invalid BNM rate for {name}"
        );
        if descriptor.embedded == 0 {
            bank.external_sounds.push(ExternalSound {
                resource_id: descriptor.resource_id,
                name,
                sample_rate: descriptor.sample_rate,
                channels: descriptor.channels,
                loop_region: if descriptor.loop_flag != 0
                    && descriptor.stream_type == 4
                    && descriptor.sample_extent > 0
                {
                    Some(crate::model::audio::SampleLoop {
                        start_frame: u64::from(descriptor.loop_start),
                        end_frame: u64::from(descriptor.loop_start)
                            + u64::from(descriptor.sample_extent),
                    })
                } else {
                    None
                },
            });
            continue;
        }
        ensure!(
            descriptor.embedded == 1,
            "unsupported BNM storage flag for {name}"
        );
        let start = match descriptor.stream_type {
            1 => header
                .pcm_offset
                .checked_add(descriptor.payload_offset)
                .context("PCM offset overflow")?,
            4 => header
                .apm_offset
                .checked_add(descriptor.payload_offset)
                .context("APM offset overflow")?,
            2 => {
                // Shipped Hype banks use both absolute and relative MPDX offsets,
                // sometimes within one bank. A bounded codec signature establishes
                // the interpretation; no offset mode escapes into the model.
                if mpdx_start(data, descriptor.payload_offset) {
                    descriptor.payload_offset
                } else {
                    let offset = header
                        .mpdx_offset
                        .checked_add(descriptor.payload_offset)
                        .context("MPDX offset overflow")?;
                    ensure!(mpdx_start(data, offset), "no valid MPDX start for {name}");
                    offset
                }
            }
            other => bail!("unsupported BNM stream type {other} for {name}"),
        };
        let mut sound = match descriptor.stream_type {
            1 => {
                let payload = bounded(data, start, descriptor.payload_size)?;
                ensure!(descriptor.bits_per_sample == 16, "BNM PCM is not 16-bit");
                ensure!(
                    payload.len() % (2 * descriptor.channels as usize) == 0,
                    "incomplete BNM PCM frame"
                );
                claim_decoded_bytes(&mut remaining, payload.len(), &name)?;
                Sound {
                    name: name.clone(),
                    sample_rate: descriptor.sample_rate,
                    channels: descriptor.channels,
                    samples: AudioSamples::Pcm16(
                        payload
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|b| i16::from_le_bytes([b[0], b[1]]))
                            .collect(),
                    ),
                    loop_region: None,
                    resource_id: None,
                    surround: None,
                    fidelity_notes: vec![],
                }
            }
            4 => {
                let size_bytes = bounded(
                    data,
                    start
                        .checked_add(24)
                        .context("APM header offset overflow")?,
                    4,
                )?;
                let size = u32::from_le_bytes(size_bytes.try_into().unwrap());
                let payload = bounded(data, start, size)?;
                let decoded_bytes = payload
                    .len()
                    .checked_sub(100)
                    .context("BNM APM header is truncated")?
                    .checked_mul(4)
                    .context("BNM APM decoded allocation overflow")?;
                claim_decoded_bytes(&mut remaining, decoded_bytes, &name)?;
                super::read_apm(payload, name.clone()).with_context(|| {
                    format!("BNM APM resource {} ({name})", descriptor.resource_id)
                })?
            }
            2 => {
                let payload = bounded(data, start, descriptor.payload_size)?;
                ensure!(payload.len() > 4, "empty MPDX stream");
                let mut sound = super::mpeg::read_ubi_mpeg_with_limit(
                    &payload[4..],
                    descriptor.sample_rate,
                    name.clone(),
                    remaining,
                )
                .with_context(|| {
                    format!("BNM MPDX resource {} ({name})", descriptor.resource_id)
                })?;
                // Charge actual allocated vectors, including padded samples and
                // both surround components. Declared duration is not a memory bound.
                let allocated = |samples: &AudioSamples| -> usize {
                    match samples {
                        AudioSamples::Pcm16(values) => values.capacity() * 2,
                        AudioSamples::Float32(values) => values.capacity() * 4,
                    }
                };
                let decoded_bytes = allocated(&sound.samples)
                    .checked_add(
                        sound
                            .surround
                            .as_ref()
                            .map_or(0, |side| allocated(&side.samples)),
                    )
                    .context("BNM MPEG decoded allocation overflow")?;
                claim_decoded_bytes(&mut remaining, decoded_bytes, &name)?;
                // The MPDX prefix specifies the meaningful duration before decoder
                // delay, independently of its padded final MPEG frame. vgmstream's
                // FMT_MPDX handling corroborates this count and 480-frame discard.
                let declared_frames = u32::from_le_bytes(payload[..4].try_into().unwrap());
                ensure!(
                    declared_frames > 480,
                    "MPDX duration is shorter than decoder delay"
                );
                let wanted = (declared_frames - 480) as usize;
                ensure!(
                    wanted <= sound.frame_count(),
                    "MPDX declared duration exceeds decoded frames"
                );
                if let AudioSamples::Float32(samples) = &mut sound.samples {
                    samples.truncate(wanted * sound.channels as usize);
                }
                if let Some(component) = &mut sound.surround
                    && let AudioSamples::Float32(samples) = &mut component.samples
                {
                    samples.truncate(wanted * component.channels as usize);
                }
                sound
            }
            _ => unreachable!(),
        };
        sound.resource_id = Some(descriptor.resource_id);
        if descriptor.loop_flag != 0 {
            let start = u64::from(descriptor.loop_start);
            let end = start + u64::from(descriptor.sample_extent);
            if descriptor.stream_type == 4 && start < end && end <= sound.frame_count() as u64 {
                sound.loop_region = Some(crate::model::audio::SampleLoop {
                    start_frame: start,
                    end_frame: end,
                });
            } else {
                sound.fidelity_notes.push(format!(
                    "BNM loop flag={} start={} extent={} needs codec-specific verification",
                    descriptor.loop_flag, descriptor.loop_start, descriptor.sample_extent
                ));
            }
        } else {
            // Bank resource playback determines whether the APM's authored loop is enabled.
            sound.loop_region = None;
        }
        if sound.channels != descriptor.channels {
            sound.fidelity_notes.push(format!(
                "BNM declares {} channels; decoded stream has {}",
                descriptor.channels, sound.channels
            ));
        }
        bank.sounds.push(sound);
    }
    Ok(bank)
}

/// Build a PCM-only BNM from editable, integer waveform content. This explicitly
/// rejects banks containing unimplemented event/external/float/surround semantics;
/// it does not claim to regenerate the original compressed bank or event graph.
pub fn write_bnm(bank: &SoundBank) -> Result<Vec<u8>> {
    ensure!(
        bank.external_sounds.is_empty()
            && bank.events.is_empty()
            && bank.reconstruction_gaps.is_empty(),
        "BNM reconstruction currently supports standalone PCM resource banks without unresolved events \
            or external references"
    );
    let count = u32::try_from(bank.sounds.len())?;
    let data_start = 44u32
        .checked_add(
            count
                .checked_mul(92)
                .context("BNM descriptor size overflow")?,
        )
        .context("BNM header size overflow")?;
    let mut descriptors = Vec::with_capacity(bank.sounds.len());
    let mut payload = Vec::new();
    for (index, sound) in bank.sounds.iter().enumerate() {
        super::validate_sound(sound)?;
        ensure!(
            sound.loop_region.is_none() && sound.surround.is_none(),
            "BNM writer does not yet implement loops or surround"
        );
        let AudioSamples::Pcm16(samples) = &sound.samples else {
            bail!(
                "BNM PCM writer requires explicit integer waveform conversion; historical MPEG re-encoding is unimplemented"
            );
        };
        let mut name = [0u8; 20];
        ensure!(
            sound.name.chars().count() <= 20,
            "BNM sound name exceeds 20 Latin-1 characters"
        );
        for (destination, ch) in name.iter_mut().zip(sound.name.chars()) {
            *destination = u8::try_from(u32::from(ch)).context("BNM names require Latin-1")?;
        }
        let size = u32::try_from(samples.len().checked_mul(2).context("PCM size overflow")?)?;
        descriptors.push(AudioDescriptor {
            resource_id: sound.resource_id.unwrap_or(index as u32),
            resource_type: 1,
            embedded: 1,
            payload_size: size,
            payload_offset: u32::try_from(payload.len())?,
            volume: 100,
            unknown_controls: [0, 1, 1, 1, 1],
            streamed: 0,
            loop_flag: 0,
            loop_start: 0,
            sample_extent: size,
            sample_rate: sound.sample_rate,
            bits_per_sample: 16,
            channels: sound.channels,
            stream_type: 1,
            name,
        });
        for &sample in samples {
            payload.extend(sample.to_le_bytes());
        }
    }
    let end = data_start
        .checked_add(u32::try_from(payload.len())?)
        .context("BNM output overflow")?;
    let header = BankHeader {
        version: 0,
        events_offset: 44,
        events_count: 0,
        sounds_offset: 44,
        sounds_count: count,
        mpdx_offset: data_start,
        midi_offset: data_start,
        pcm_offset: data_start,
        apm_offset: end,
        streamed_offset: end,
        file_size: end,
    };
    let mut output = Cursor::new(Vec::new());
    header.write(&mut output)?;
    for descriptor in descriptors {
        descriptor.write(&mut output)?;
    }
    let mut output = output.into_inner();
    output.extend(payload);
    Ok(output)
}

pub struct Bnm {
    pub name: String,
}
impl FormatCodec for Bnm {
    type Value = SoundBank;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "BNM",
            reads: true,
            writes: true,
            notes: "PCM/APM/Ubi-MPEG import; writer supports PCM-only banks without events/external references; \
            unsupported values fail explicitly",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<SoundBank> {
        read_bnm(bytes, self.name.clone())
    }
    fn write(&self, value: &SoundBank) -> Result<Vec<u8>> {
        write_bnm(value)
    }
}

#[cfg(test)]
mod allocation_tests {
    use super::*;

    #[test]
    fn repeated_pcm_and_apm_descriptors_share_one_bank_decode_budget() {
        let sound = Sound {
            name: "shared.wav".into(),
            sample_rate: 22050,
            channels: 1,
            samples: AudioSamples::Pcm16(vec![0; 8]),
            loop_region: None,
            resource_id: Some(0),
            surround: None,
            fidelity_notes: vec![],
        };
        let original = write_bnm(&SoundBank {
            name: "shared".into(),
            sounds: vec![sound.clone()],
            external_sounds: vec![],
            events: vec![],
            fidelity_notes: vec![],
            reconstruction_gaps: vec![],
        })
        .unwrap();
        for stream_type in [1u32, 4] {
            let payload = if stream_type == 1 {
                original[136..].to_vec()
            } else {
                super::super::write_apm(&sound).unwrap()
            };
            // Three independent resource IDs reference the same encoded range;
            // neither encoded bank size nor one member's bound limits expansion.
            let mut bytes = original[..44].to_vec();
            let data_start = 44 + 3 * 92;
            bytes[16..20].copy_from_slice(&3u32.to_le_bytes());
            bytes[28..32].copy_from_slice(&(data_start as u32).to_le_bytes());
            bytes[32..36].copy_from_slice(&(data_start as u32).to_le_bytes());
            bytes[40..44].copy_from_slice(&((data_start + payload.len()) as u32).to_le_bytes());
            for id in 0..3u32 {
                let mut descriptor = original[44..136].to_vec();
                descriptor[..4].copy_from_slice(&id.to_le_bytes());
                descriptor[12..16].copy_from_slice(&(payload.len() as u32).to_le_bytes());
                descriptor[68..72].copy_from_slice(&stream_type.to_le_bytes());
                bytes.extend(descriptor);
            }
            bytes.extend(payload);
            let bank = read_bnm_with_limit(&bytes, "shared", 48).unwrap();
            assert_eq!(bank.sounds.len(), 3);
            assert!(bank.sounds.iter().all(|s| s.samples == sound.samples));
            let error = read_bnm_with_limit(&bytes, "shared", 32).unwrap_err();
            assert!(
                error.to_string().contains("cumulative decoded allocation"),
                "{error:#}"
            );
        }
    }
}
