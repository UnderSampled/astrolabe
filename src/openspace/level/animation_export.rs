//! Source-free sampled animation fragments and independently rebuilt matrix pools.
use super::{
    container::{emit_relocations, encoded},
    geometry::{EmittedRange, GeometryExport, Layout, ReferenceTarget, float_data, short_data},
    records::*,
    transform::write_transform,
};
use crate::model::{AssetPath, PackageRole, animation::*, io::Content};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub struct TransformPoolExport {
    pub bytes: Vec<u8>,
    pub targets: BTreeMap<u32, ReferenceTarget>,
    pub ranges: Vec<EmittedRange>,
}
pub fn write_transform_pool(pool: &TransformPool, base: u32) -> Result<TransformPoolExport> {
    pool.validate()?;
    ensure!(
        base > 1 && base.is_multiple_of(4),
        "invalid transform pool base"
    );
    let block = match pool.path.role() {
        PackageRole::Fix => BlockKey(6, 1),
        PackageRole::Level => BlockKey(6, 2),
        _ => anyhow::bail!("invalid transform pool role"),
    };
    let mut out = Layout {
        bytes: Vec::new(),
        rows: Vec::new(),
        ranges: Vec::new(),
        base,
        block,
    };
    let mut targets = BTreeMap::new();
    for entry in &pool.entries {
        // Allocated identity retains its own identity rather than merging distinct entries into pointer 1.
        let bytes = match write_transform(&entry.value)? {
            Some(bytes) => bytes,
            None => write_transform(&TransformValue::Affine(Default::default()))?.unwrap(),
        };
        let allocated = (bytes.len() + 3) & !3;
        let mut record = Vec::with_capacity(allocated + 4);
        record.extend(((allocated / 4 + 1) as u32).to_le_bytes());
        record.extend(bytes);
        record.resize(allocated + 4, 0);
        let offset = out.allocate(format!("entries/{}", entry.id), &record)? + 4;
        let address = base
            .checked_add(offset.try_into()?)
            .context("transform pool address overflow")?;
        targets.insert(
            entry.id,
            ReferenceTarget {
                address,
                block: (block.0, block.1),
            },
        );
    }
    Ok(TransformPoolExport {
        bytes: out.bytes,
        targets,
        ranges: out.ranges,
    })
}
pub fn write_animation(
    clip: &AnimationClip,
    pools: &BTreeMap<AssetPath, BTreeMap<u32, ReferenceTarget>>,
    actions: &BTreeMap<AssetPath, ReferenceTarget>,
    base: u32,
) -> Result<GeometryExport> {
    clip.validate()?;
    ensure!(base > 1 && base.is_multiple_of(4), "invalid animation base");
    ensure!(
        clip.unknown.len() == 6,
        "animation header unknown field count"
    );
    ensure!(
        clip.morph_count == 0,
        "animation morph payload reconstruction is unsupported"
    );
    let block = animation_block(clip.path.role())?;
    let mut out = Layout {
        bytes: Vec::new(),
        rows: Vec::new(),
        ranges: Vec::new(),
        base,
        block,
    };
    let speed_bytes = write_transform(&TransformValue::Affine(clip.speed.clone()))?.unwrap();
    let header = AnimationWire {
        frames: 0,
        frame_count: clip.frames.len().try_into()?,
        speed: clip.frame_rate,
        channel_count: clip.channel_count,
        unknown: clip.unknown[0],
        events: 0,
        unknown_12: u32::from_le_bytes(clip.unknown[1..5].try_into()?),
        event_count: clip.cues.len().try_into()?,
        unknown_17: clip.unknown[5],
        morph_count: clip.morph_count,
        speed_matrix: super::container::wire(&speed_bytes)?,
        unknown_108: 0,
        related: 0,
    };
    let root = out.allocate("animation", &encoded(&header)?)?;
    if !clip.cues.is_empty() {
        let cues = out.allocate("cues", &vec![0; clip.cues.len() * 8])?;
        out.pointer(root + 8, cues)?;
        for (i, cue) in clip.cues.iter().enumerate() {
            let offset = cues + i * 8;
            out.bytes[offset..offset + 8].copy_from_slice(&encoded(&AnimationCueWire {
                action: 0,
                configuration_slot: cue.configuration_slot,
                frame: cue.frame,
                channel: cue.channel,
            })?);
            let target = *actions
                .get(&cue.action.path)
                .context("animation action dependency missing")?;
            out.external_pointer(offset, target)?;
        }
    }
    let frames = out.allocate("frames", &vec![0; clip.frames.len() * 16])?;
    out.pointer(root, frames)?;
    let mut samples = Vec::new();
    for (i, sample) in clip.samples.iter().enumerate() {
        let offset = out.allocate(
            format!("samples/{i}"),
            &encoded(&AnimationChannelWire {
                transform: if sample.transform.is_none() { 1 } else { 0 },
                object: sample.object.unwrap_or(u16::MAX),
                visibility: sample.visibility,
                unknown: sample.unknown[..3].try_into()?,
                channel: sample.channel.unwrap_or(255),
                opacity: sample.opacity,
                kind: match sample.kind {
                    crate::model::animation::ChannelKind::Object => 0,
                    crate::model::animation::ChannelKind::Hidden => 2,
                    crate::model::animation::ChannelKind::Light => 4,
                    crate::model::animation::ChannelKind::Unknown(v) => v,
                },
                tail: sample.unknown[3..].try_into()?,
            })?,
        )?;
        if let Some(link) = &sample.transform {
            let target = pools
                .get(&link.pool.path)
                .and_then(|p| p.get(&link.entry))
                .context("animation transform dependency missing")?;
            out.external_pointer(offset, *target)?;
        }
        samples.push(offset);
    }
    for (i, frame) in clip.frames.iter().enumerate() {
        let root = frames + i * 16;
        let channels = out.allocate(
            format!("frames/{i}/channels"),
            &vec![0; frame.samples.len() * 4],
        )?;
        out.pointer(root, channels)?;
        for (j, &sample) in frame.samples.iter().enumerate() {
            out.pointer(channels + j * 4, samples[sample as usize])?;
        }
        if !frame.hierarchy.is_empty() {
            let mut h = (frame.hierarchy.len() as u32).to_le_bytes().to_vec();
            h.extend([0; 4]);
            let header = out.allocate(format!("frames/{i}/hierarchy"), &h)?;
            let values: Vec<[u16; 2]> = frame.hierarchy.iter().map(|v| v.map(u16::from)).collect();
            let pairs = out.allocate(format!("frames/{i}/parents"), &short_data(&values))?;
            out.pointer(root + 12, header)?;
            out.pointer(header + 4, pairs)?;
        }
        if let Some(rotation) = &frame.motion_rotation {
            let p = out.allocate(format!("frames/{i}/rotation"), &float_data(rotation))?;
            out.pointer(root + 4, p)?;
        }
        if let Some(translation) = &frame.motion_translation {
            let p = out.allocate(
                format!("frames/{i}/translation"),
                &float_data(&[*translation]),
            )?;
            out.pointer(root + 8, p)?;
        }
    }
    out.rows.sort_by_key(|r| r.value);
    Ok(GeometryExport{bytes:out.bytes,rtb:emit_relocations(block,&out.rows)?,ranges:out.ranges,pointer_count:out.rows.len(),coverage:vec!["Sampled channel records, frame tables, dynamic hierarchies, frame motion and speed matrices rebuilt from values; transform links bind separately regenerated pools.".into(),"Ordered event cues reference separately rebuilt shared action records and sound bindings. Related/morph data and enclosing family/perso package reconstruction remain unsupported; these are controlled fragments.".into()]})
}

/// Shared action records plus regenerated SND/RTS bindings. These are controlled
/// fragments; the level writer must compose their target map with other records.
pub struct AnimationActionsExport {
    pub bytes: Vec<u8>,
    pub snd: Vec<u8>,
    pub rts: Vec<u8>,
    pub targets: BTreeMap<AssetPath, ReferenceTarget>,
    pub ranges: Vec<EmittedRange>,
}
pub fn write_animation_actions(
    actions: &[AnimationAction],
    base: u32,
) -> Result<AnimationActionsExport> {
    ensure!(!actions.is_empty(), "empty animation action collection");
    ensure!(base > 1 && base.is_multiple_of(4), "invalid action base");
    let role = actions[0].path.role();
    let block = animation_block(role.clone())?;
    let mut out = Layout {
        bytes: Vec::new(),
        rows: Vec::new(),
        ranges: Vec::new(),
        base,
        block,
    };
    let mut snd = Vec::new();
    let mut targets = BTreeMap::new();
    let mut rows = Vec::new();
    for action in actions {
        action.validate()?;
        ensure!(
            action.path.role() == role,
            "action collection must have one package owner"
        );
        let (kind, payload) = match &action.operation {
            AnimationOperation::Sound { .. } => (0, 0),
            AnimationOperation::Mechanics { command } => (1, *command),
            AnimationOperation::Attachment { value } => (2, *value),
            AnimationOperation::AttachmentControl { command } => (3, *command),
            AnimationOperation::Camera { command } => (4, *command),
            AnimationOperation::Unknown { kind, payload } => (*kind, u32::from_le_bytes(*payload)),
        };
        let offset = out.allocate(
            action.path.file(),
            &encoded(&AnimationActionWire {
                payload,
                kind,
                unknown: action.unknown,
            })?,
        )?;
        let address = base
            .checked_add(offset.try_into()?)
            .context("action address overflow")?;
        ensure!(
            targets
                .insert(
                    action.path.clone(),
                    ReferenceTarget {
                        address,
                        block: (block.0, block.1)
                    }
                )
                .is_none(),
            "duplicate action identity"
        );
        if let AnimationOperation::Sound { event: Some(event) } = &action.operation {
            snd.extend(encoded(&SoundSiteWire {
                event: (u32::from(event.bank) << 16) | u32::from(event.member),
                site: address,
            })?);
            rows.push(RelocRow {
                value: address,
                module: block.0,
                id: block.1,
                unknown: [0; 2],
            });
        }
    }
    Ok(AnimationActionsExport {
        bytes: out.bytes,
        snd,
        rts: emit_relocations(block, &rows)?,
        targets,
        ranges: out.ranges,
    })
}
fn animation_block(role: PackageRole) -> Result<BlockKey> {
    Ok(match role {
        PackageRole::Fix => BlockKey(5, 0),
        PackageRole::Level => BlockKey(5, 1),
        _ => anyhow::bail!("invalid animation role"),
    })
}
