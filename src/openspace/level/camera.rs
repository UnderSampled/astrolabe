//! Hype size getters 00440050/00440060 return 220/164 payload bytes.
//! The next allocator header is excluded from both declarations.
//! Hype camera parameter getters 00440ac0/af0, 00440b50/70,
//! 00440bd0/c10/f10 and camera setup 00527e00. Renderer publication
//! 0043dbd0 obtains horizontal FoV from parameter set 1 +0x8c.
use super::{
    container::{encoded, wire},
    records::*,
    session::SourceSession,
};
use crate::model::{
    ai::SourceObjectIdentity,
    camera::{Camera, CameraParameters},
};
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite};

/// Only declared parameter links are traversed. Inactive file targets may
/// point outside the selected level's rooted hierarchy; do not replay them.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct CameraLinksWire {
    pub(super) parameters: [u32; 4],
    pub(super) target: u32,
    pub(super) unknown_before_renderer: [u8; 8],
    pub(super) renderer: u32,
    pub(super) unknown: [u8; 188],
}

/// GLI_xCreateCamera 00485000 and near/far setter 004852b0. The
/// prefix contains renderer state, including a derived view matrix; it is
/// neither traversed nor used to cache the native camera pose.
#[derive(BinRead)]
#[br(little)]
struct CameraClipWire {
    _unknown_header: [u8; 8],
    mode: u32,
    _derived_view_and_projection: [u8; 96],
    clip_planes: [f32; 2],
}

#[derive(BinRead, BinWrite)]
#[brw(little)]
struct CameraParametersWire {
    unknown_00: [u8; 12],
    target_offset: [f32; 3],
    look_offset: [f32; 3],
    actor_offset: [f32; 3],
    distance: [f32; 2],
    unknown_38: [u8; 8],
    yaw: f32,
    unknown_44: [u8; 4],
    pitch: f32,
    unknown_4c: [u8; 64],
    horizontal_fov: f32,
    unknown_90: [u8; 8],
    mode: u32,
    unknown_9c: [u8; 8],
}

pub fn read_camera_parameters(
    bytes: &[u8],
    source: SourceObjectIdentity,
) -> Result<CameraParameters> {
    ensure!(bytes.len() == 164, "camera parameter extent differs");
    let w: CameraParametersWire = wire(bytes)?;
    let value = CameraParameters {
        source,
        target_offset: w.target_offset,
        look_offset: w.look_offset,
        actor_offset: w.actor_offset,
        distance: w.distance,
        yaw: w.yaw,
        pitch: w.pitch,
        horizontal_fov: w.horizontal_fov,
        mode: w.mode,
        unknown: [
            w.unknown_00.as_slice(),
            &w.unknown_38,
            &w.unknown_44,
            &w.unknown_4c,
            &w.unknown_90,
            &w.unknown_9c,
        ]
        .concat(),
    };
    value.validate()?;
    Ok(value)
}

pub fn write_camera_parameters(value: &CameraParameters) -> Result<Vec<u8>> {
    value.validate()?;
    let u = &value.unknown;
    encoded(&CameraParametersWire {
        unknown_00: u[0..12].try_into()?,
        target_offset: value.target_offset,
        look_offset: value.look_offset,
        actor_offset: value.actor_offset,
        distance: value.distance,
        unknown_38: u[12..20].try_into()?,
        yaw: value.yaw,
        unknown_44: u[20..24].try_into()?,
        pitch: value.pitch,
        unknown_4c: u[24..88].try_into()?,
        horizontal_fov: value.horizontal_fov,
        unknown_90: u[88..96].try_into()?,
        mode: value.mode,
        unknown_9c: u[96..104].try_into()?,
    })
}

pub(super) fn import(
    session: &SourceSession,
    root: Address,
    owner_motion_binding_present: bool,
) -> Result<Camera> {
    let links: CameraLinksWire = wire(session.bytes(root, 220)?)?;
    for offset in (0..220)
        .step_by(4)
        .filter(|offset| ![0, 4, 8, 12, 16, 28].contains(offset))
    {
        ensure!(
            !session.relocated(Address {
                offset: root.offset + offset,
                ..root
            })?,
            "camera component contains unpromoted pointer at {offset}"
        );
    }
    let mut parameters = Vec::with_capacity(4);
    for (index, pointer) in links.parameters.iter().enumerate() {
        ensure!(*pointer != 0, "missing source camera parameter set");
        let address = session.required(root, index * 4)?;
        for offset in (0..164).step_by(4) {
            ensure!(
                !session.relocated(Address {
                    offset: address.offset + offset,
                    ..address
                })?,
                "camera parameter set contains unpromoted pointer at {offset}"
            );
        }
        parameters.push(read_camera_parameters(
            session.bytes(address, 164)?,
            session.source_identity(address)?,
        )?);
    }
    let value = Camera {
        source: session.source_identity(root)?,
        owner_motion_binding_present,
        unknown: [links.unknown_before_renderer.as_slice(), &links.unknown].concat(),
        clip_planes: {
            ensure!(links.renderer != 0, "camera renderer link is empty");
            let renderer = session.required(root, 28)?;
            let clips: CameraClipWire = wire(session.bytes(renderer, 116)?)?;
            ensure!(
                clips.mode == 103,
                "unsupported camera projection mode {}",
                clips.mode
            );
            clips.clip_planes
        },
        parameters: parameters.try_into().unwrap(),
    };
    value.validate()?;
    Ok(value)
}
