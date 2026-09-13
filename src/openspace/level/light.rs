//! GLI_xCreateLight (00490de0) and GLI light setters: 200 bytes in Hype.
use super::{
    container::{encoded, wire},
    records::TransformWire,
};
use crate::model::{
    AssetPath,
    geometry::SourceTransform,
    io::Content,
    light::{Light, LightUnknown},
};
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite};
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct LightWire {
    on: u8,
    cast_shadows: u8,
    rotating: u8,
    pulsing: u8,
    unknown_header: u32,
    kind: u32,
    far: f32,
    near: f32,
    angles: [f32; 2],
    rotation_step: f32,
    pulse_step: f32,
    transform: TransformWire,
    unknown_transform: [u32; 4],
    painting: u32,
    unknown_painting: u32,
    color: [f32; 4],
    creates_shadows: u32,
    name: [u8; 20],
    unknown_before_send: u8,
    send: u8,
    unknown_after_send: [u8; 2],
    intensity_min: f32,
    dimmer: u32,
}
pub fn read_light(bytes: &[u8], path: AssetPath) -> Result<Light> {
    let w: LightWire = wire(bytes)?;
    let end = w.name.iter().position(|&b| b == 0).unwrap_or(20);
    let v = Light {
        path,
        name: w.name[..end].iter().map(|&b| char::from(b)).collect(),
        kind: w.kind,
        on: w.on,
        cast_shadows: w.cast_shadows,
        rotating: w.rotating,
        pulsing: w.pulsing,
        far: w.far,
        near: w.near,
        angles: w.angles,
        rotation_step: w.rotation_step,
        pulse_step: w.pulse_step,
        transform: SourceTransform {
            translation: w.transform.translation,
            rotation: w.transform.rotation,
            basis: w.transform.basis,
        },
        color: w.color,
        painting: w.painting,
        creates_shadows: w.creates_shadows,
        send: w.send,
        intensity_min: w.intensity_min,
        unresolved_dimmer: w.dimmer != 0,
        unknown: LightUnknown {
            header: w.unknown_header,
            after_transform: w.unknown_transform,
            after_painting: w.unknown_painting,
            before_send: w.unknown_before_send,
            after_send: w.unknown_after_send,
            name_tail: w.name[end..]
                .iter()
                .rposition(|&b| b != 0)
                .map_or_else(Vec::new, |last| w.name[end..=end + last].to_vec()),
        },
    };
    v.validate()?;
    Ok(v)
}
pub fn write_light(v: &Light) -> Result<Vec<u8>> {
    v.validate()?;
    ensure!(
        !v.unresolved_dimmer,
        "light dimmer reconstruction is not implemented"
    );
    let mut name = [0; 20];
    let text: Vec<u8> = v.name.chars().map(|c| c as u8).collect();
    name[..text.len()].copy_from_slice(&text);
    name[text.len()..text.len() + v.unknown.name_tail.len()].copy_from_slice(&v.unknown.name_tail);
    encoded(&LightWire {
        on: v.on,
        cast_shadows: v.cast_shadows,
        rotating: v.rotating,
        pulsing: v.pulsing,
        unknown_header: v.unknown.header,
        kind: v.kind,
        far: v.far,
        near: v.near,
        angles: v.angles,
        rotation_step: v.rotation_step,
        pulse_step: v.pulse_step,
        transform: wire(
            &super::write_transform(&crate::model::animation::TransformValue::Affine(
                v.transform.clone(),
            ))?
            .expect("affine transforms have full records"),
        )?,
        unknown_transform: v.unknown.after_transform,
        painting: v.painting,
        unknown_painting: v.unknown.after_painting,
        color: v.color,
        creates_shadows: v.creates_shadows,
        name,
        unknown_before_send: v.unknown.before_send,
        send: v.send,
        unknown_after_send: v.unknown.after_send,
        intensity_min: v.intensity_min,
        dimmer: 0,
    })
}
