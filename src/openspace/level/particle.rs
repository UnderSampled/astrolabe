//! Promoted particle records. Pointers and runtime counters stay at this boundary.
use super::{
    container::{encoded, wire},
    records::*,
    session::SourceSession,
};
use crate::model::{AssetPath, AssetRef, io::Content, particle::*};
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite};

// 00450fd0 selects +0, 00451010 toggles +8, 00451310 derives +9 each update.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct ComponentWire {
    pub generator: u32,
    pub unknown_4: [u8; 4],
    pub enabled: u8,
    pub environment_valid: u8,
    pub unknown_10: [u8; 6],
}
// 0044fd30 binds the three definitions. 004512b0 derives bounds at +16;
// +40 is a lazy-initialization flag, not useful persisted content.
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct GeneratorWire {
    emission: u32,
    system: u32,
    environment: u32,
    medium_filter: u32,
    derived_bounds: [[f32; 3]; 2],
    initialized: u8,
    relative_environment: u8,
    unknown: [u8; 2],
}
// 004c1640/004c16b0 define emission/number; 004c2710 defines shape,
// coordinates, velocity and lifetime. 004c1480 derives the quad normal.
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct EmissionWire {
    mode: u8,
    unknown_1: [u8; 3],
    probability: f32,
    paused_updates: u16,
    period_updates: u16,
    update_counter: u16,
    number_mode: u8,
    unknown_15: u8,
    number: u16,
    number_range: [u16; 2],
    shape: u8,
    coordinate_mode: u8,
    first_corners: [[f32; 3]; 3],
    derived_normal: [f32; 3],
    last_corner: [f32; 3],
    velocity: [f32; 3],
    velocity_spread: [f32; 3],
    lifetime_mode: u8,
    unknown_109: [u8; 3],
    lifetime: [f32; 2],
    unknown_120: [u8; 12],
}
// 004c19c0 creates capacity-sized velocity/lifetime/geometry buffers and stores
// the selected Sprite at *(system+36). Their placements are not Rete settings.
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct SystemWire {
    capacity: u16,
    active: u16,
    velocities: u32,
    flags: [u8; 4],
    particle_flags: u32,
    unknown_16: [u8; 8],
    geometry: u32,
    kind: u16,
    unknown_30: [u8; 2],
    lifetimes: u32,
    sprite_definition: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct SpriteBindingWire {
    sprite: u32,
    unknown: [u8; 36],
}
// 004512b0 and 004c2990: box/sphere bounds, acceleration plus symmetric noise.
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct EnvironmentWire {
    bounds_kind: u32,
    box_bounds: [[f32; 3]; 2],
    sphere_center: [f32; 3],
    sphere_radius: f32,
    acceleration: [f32; 3],
    acceleration_spread: [f32; 3],
}

pub(super) fn reference(s: &SourceSession, a: Address) -> Result<AssetRef<ParticleGenerator>> {
    let id = s.source_identity(a)?;
    let role = if a.owner == Owner::Fix {
        "fix"
    } else {
        "level"
    };
    Ok(AssetRef::from_path(AssetPath::new(format!(
        "{role}:/particles/generators/{}-{}-{}",
        id.block[0], id.block[1], id.allocation
    ))?))
}

/// Current claimed slice is sprite-generator definition and selection, not
/// complete particle rendering or source package reconstruction.
pub(super) fn read(
    s: &SourceSession,
    a: Address,
    sprite: AssetRef<crate::model::sprite::Sprite>,
) -> Result<ParticleGenerator> {
    let g: GeneratorWire = wire(s.bytes(a, 44)?)?;
    let source = s.required(a, 0)?;
    let system = s.required(a, 4)?;
    let environment = s.required(a, 8)?;
    let e: EmissionWire = wire(s.bytes(source, 132)?)?;
    let sys: SystemWire = wire(s.bytes(system, 40)?)?;
    let v: EnvironmentWire = wire(s.bytes(environment, 68)?)?;
    ensure!(
        sys.kind == 4 && sys.active == 0,
        "particle system is not an initial sprite definition"
    );
    let value = ParticleGenerator {
        path: reference(s, a)?.path,
        source_identity: Some(ParticleIdentity {
            generator: s.source_identity(a)?,
            emission: s.source_identity(source)?,
            system: s.source_identity(system)?,
            environment: s.source_identity(environment)?,
        }),
        capacity: sys.capacity,
        sprite,
        emission: ParticleEmission {
            mode: e.mode,
            unknown_1: e.unknown_1,
            probability: e.probability,
            paused_updates: e.paused_updates,
            period_updates: e.period_updates,
            number_mode: e.number_mode,
            unknown_15: e.unknown_15,
            number: e.number,
            number_range: e.number_range,
            shape: e.shape,
            coordinate_mode: e.coordinate_mode,
            corners: [
                e.first_corners[0],
                e.first_corners[1],
                e.first_corners[2],
                e.last_corner,
            ],
            velocity: e.velocity,
            velocity_spread: e.velocity_spread,
            lifetime_mode: e.lifetime_mode,
            unknown_109: e.unknown_109,
            lifetime: e.lifetime,
            unknown_120: e.unknown_120,
        },
        environment: ParticleEnvironment {
            bounds_kind: v.bounds_kind,
            box_bounds: v.box_bounds,
            sphere_center: v.sphere_center,
            sphere_radius: v.sphere_radius,
            acceleration: v.acceleration,
            acceleration_spread: v.acceleration_spread,
        },
        relative_environment: g.relative_environment != 0,
        medium_filter: g.medium_filter,
        unknown: g.unknown,
        system_flags: sys.flags,
        system_unknown_16: sys.unknown_16,
        system_unknown_30: sys.unknown_30,
    };
    value.validate()?;
    Ok(value)
}

pub(super) fn sprite_address(s: &SourceSession, generator: Address) -> Result<Address> {
    let _: GeneratorWire = wire(s.bytes(generator, 44)?)?;
    let system = s.required(generator, 4)?;
    let h: SystemWire = wire(s.bytes(system, 40)?)?;
    ensure!(h.kind == 4, "unsupported particle system kind {}", h.kind);
    let binding = s.required(system, 36)?;
    let _: SpriteBindingWire = wire(s.bytes(binding, 40)?)?;
    s.required(binding, 0)
}

pub(super) fn component(s: &SourceSession, perso: Address) -> Result<Option<ParticleComponent>> {
    // 00542340 resolves the pointer at +64 of the acting Perso.
    #[derive(BinRead)]
    #[br(little)]
    struct ActorWire {
        _prefix: [u8; 64],
        particles: u32,
    }
    let actor: ActorWire = wire(s.bytes(perso, 68)?)?;
    if actor.particles == 0 {
        return Ok(None);
    }
    let a = s.required(perso, 64)?;
    let c: ComponentWire = wire(s.bytes(a, 16)?)?;
    ensure!(c.enabled <= 1, "invalid initial particle enable flag");
    Ok(Some(ParticleComponent {
        selected: s.pointer(a, 0)?.map(|g| reference(s, g)).transpose()?,
        enabled: c.enabled != 0,
        unknown_4: c.unknown_4,
        unknown_10: c.unknown_10,
    }))
}

pub(super) fn import(
    s: &SourceSession,
    a: Address,
    result: &mut super::scene::SceneImport,
    material_ids: &mut std::collections::BTreeMap<Address, AssetPath>,
) -> Result<AssetRef<ParticleGenerator>> {
    let reference = reference(s, a)?;
    if result
        .particle_generators
        .iter()
        .any(|g| g.path == reference.path)
    {
        return Ok(reference);
    }
    let sprite = sprite_address(s, a)?;
    let mut notes = Vec::new();
    let sprite =
        super::sprite::read_definition(s, sprite, &mut result.materials, material_ids, &mut notes)?;
    let value = read(s, a, AssetRef::from_path(sprite.path.clone()))?;
    if !result.sprites.iter().any(|v| v.path == sprite.path) {
        result.sprites.push(sprite);
    }
    for message in notes {
        result.diagnostics.push(crate::model::io::Diagnostic {
            code: "openspace.particle.material_coverage".into(),
            asset: Some(reference.path.clone()),
            severity: crate::model::io::Severity::Info,
            message,
        });
    }
    result.particle_generators.push(value);
    Ok(reference)
}

/// Reconstruct semantic source/environment records without a cached wire blob.
pub fn write_settings(value: &ParticleGenerator) -> Result<(Vec<u8>, Vec<u8>)> {
    value.validate()?;
    let e = &value.emission;
    ensure!(
        e.shape == 1,
        "source particle settings reconstruction currently covers parallelogram emission"
    );
    let subtract = |a: [f32; 3], b: [f32; 3]| std::array::from_fn::<_, 3, _>(|i| a[i] - b[i]);
    let a = subtract(e.corners[1], e.corners[0]);
    let b = subtract(e.corners[2], e.corners[0]);
    let mut normal = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let length = normal.iter().map(|n| n * n).sum::<f32>().sqrt();
    if length > 0. {
        for n in &mut normal {
            *n /= length;
        }
    }
    let source = encoded(&EmissionWire {
        mode: e.mode,
        unknown_1: e.unknown_1,
        probability: e.probability,
        paused_updates: e.paused_updates,
        period_updates: e.period_updates,
        update_counter: 0,
        number_mode: e.number_mode,
        unknown_15: e.unknown_15,
        number: e.number,
        number_range: e.number_range,
        shape: e.shape,
        coordinate_mode: e.coordinate_mode,
        first_corners: [e.corners[0], e.corners[1], e.corners[2]],
        derived_normal: normal,
        last_corner: e.corners[3],
        velocity: e.velocity,
        velocity_spread: e.velocity_spread,
        lifetime_mode: e.lifetime_mode,
        unknown_109: e.unknown_109,
        lifetime: e.lifetime,
        unknown_120: e.unknown_120,
    })?;
    let v = &value.environment;
    let environment = encoded(&EnvironmentWire {
        bounds_kind: v.bounds_kind,
        box_bounds: v.box_bounds,
        sphere_center: v.sphere_center,
        sphere_radius: v.sphere_radius,
        acceleration: v.acceleration,
        acceleration_spread: v.acceleration_spread,
    })?;
    Ok((source, environment))
}
