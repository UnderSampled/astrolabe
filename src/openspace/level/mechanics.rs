//! Hype DNM identity cards: sizes from DNM_fn_xIdentityCardCreate (00591464).
//! Character fields follow the named getters/setters at 005940b7..00595a9f.
use super::container::{encoded, wire};
use crate::model::{AssetPath, io::Content, mechanics::*};
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite};
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct CharacterWire {
    mass: f32,
    stairs_step_height: f32,
    min_linear_speed: f32,
    min_angular_speed: f32,
    max_linear_speed: f32,
    max_angular_speed: f32,
    fsl_available: u8,
    fsl_unknown: [u8; 3],
    ground_limit: f32,
    slope_limit: f32,
    wall_limit: f32,
    attic_limit: f32,
    step_front_limit: f32,
    step_high_limit: f32,
    orientation_free: u8,
    orientation_unknown: [u8; 3],
    adaptive_velocity: f32,
    dt_forced: u8,
    dt_unknown: [u8; 3],
    dt_limit: f32,
    against_wall: u8,
    wall_unknown: [u8; 3],
    adaptive_wall_velocity: f32,
    bring_down_height: f32,
    air_resisting_surface: f32,
    ratio: f32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct Header {
    kind: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct MassWire {
    mass: f32,
}
pub(super) fn size(kind: u32) -> Result<usize> {
    Ok(match kind {
        0 | 4 | 6 | 18 => 92,
        3 => 16,
        7 => 52,
        9 | 19 => 8,
        // Source case 0xb allocates a pointer-free 0x14-byte card. Its fields
        // are retained until their gameplay meaning is established.
        11 => 20,
        // 00591464 case 0x10 allocates 0x34 and seeds floats (1, 80, 2, 0.85,
        // 1, 5); case 0x11 allocates 100, zeroed. Both are pointer-free (the
        // copier memcpys 0x10 whole). Retained until their meaning is read.
        16 => 52,
        17 => 100,
        // The remaining 00591464 allocations; DNM_fn_vIdentityCardCopy
        // (00593eaf) copies every kind flat, so all are pointer-free.
        1 => 72,
        2 => 80,
        5 => 100,
        8 => 8,
        10 => 28,
        12 => 52,
        13 => 40,
        14 => 24,
        15 => 32,
        20 => 12,
        _ => anyhow::bail!("unsupported Hype mechanics kind {kind}"),
    })
}
pub fn read_mechanics(bytes: &[u8], path: AssetPath) -> Result<Mechanics> {
    let header: Header = wire(bytes)?;
    ensure!(
        bytes.len() == size(header.kind)?,
        "mechanics record length differs"
    );
    let data = match header.kind {
        0 | 4 | 18 => {
            let w: CharacterWire = wire(&bytes[4..])?;
            MechanicsData::Character {
                mode: header.kind,
                settings: CharacterMechanics {
                    mass: w.mass,
                    stairs_step_height: w.stairs_step_height,
                    min_linear_speed: w.min_linear_speed,
                    min_angular_speed: w.min_angular_speed,
                    max_linear_speed: w.max_linear_speed,
                    max_angular_speed: w.max_angular_speed,
                    fsl_available: w.fsl_available,
                    fsl_unknown: w.fsl_unknown,
                    ground_limit: w.ground_limit,
                    slope_limit: w.slope_limit,
                    wall_limit: w.wall_limit,
                    attic_limit: w.attic_limit,
                    step_front_limit: w.step_front_limit,
                    step_high_limit: w.step_high_limit,
                    orientation_free: w.orientation_free,
                    orientation_unknown: w.orientation_unknown,
                    adaptive_velocity: w.adaptive_velocity,
                    dt_forced: w.dt_forced,
                    dt_unknown: w.dt_unknown,
                    dt_limit: w.dt_limit,
                    against_wall: w.against_wall,
                    wall_unknown: w.wall_unknown,
                    adaptive_wall_velocity: w.adaptive_wall_velocity,
                    bring_down_height: w.bring_down_height,
                    air_resisting_surface: w.air_resisting_surface,
                    ratio: w.ratio,
                },
            }
        }
        9 => MechanicsData::Mass {
            mass: wire::<MassWire>(&bytes[4..])?.mass,
        },
        kind => MechanicsData::Unresolved {
            kind,
            body: bytes[4..].to_vec(),
        },
    };
    let result = Mechanics { path, data };
    result.validate()?;
    Ok(result)
}
pub fn write_mechanics(value: &Mechanics) -> Result<Vec<u8>> {
    value.validate()?;
    let (kind, body) = match &value.data {
        MechanicsData::Character { mode, settings: s } => (
            *mode,
            encoded(&CharacterWire {
                mass: s.mass,
                stairs_step_height: s.stairs_step_height,
                min_linear_speed: s.min_linear_speed,
                min_angular_speed: s.min_angular_speed,
                max_linear_speed: s.max_linear_speed,
                max_angular_speed: s.max_angular_speed,
                fsl_available: s.fsl_available,
                fsl_unknown: s.fsl_unknown,
                ground_limit: s.ground_limit,
                slope_limit: s.slope_limit,
                wall_limit: s.wall_limit,
                attic_limit: s.attic_limit,
                step_front_limit: s.step_front_limit,
                step_high_limit: s.step_high_limit,
                orientation_free: s.orientation_free,
                orientation_unknown: s.orientation_unknown,
                adaptive_velocity: s.adaptive_velocity,
                dt_forced: s.dt_forced,
                dt_unknown: s.dt_unknown,
                dt_limit: s.dt_limit,
                against_wall: s.against_wall,
                wall_unknown: s.wall_unknown,
                adaptive_wall_velocity: s.adaptive_wall_velocity,
                bring_down_height: s.bring_down_height,
                air_resisting_surface: s.air_resisting_surface,
                ratio: s.ratio,
            })?,
        ),
        MechanicsData::Mass { mass } => (9, encoded(&MassWire { mass: *mass })?),
        MechanicsData::Unresolved { kind, body } => (*kind, body.clone()),
    };
    let mut bytes = encoded(&Header { kind })?;
    bytes.extend(body);
    ensure!(
        bytes.len() == size(kind)?,
        "mechanics record length differs"
    );
    Ok(bytes)
}
