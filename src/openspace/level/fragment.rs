//! Controlled model/instance/material fragments, not complete SNA packages.
use super::records::*;
use super::{
    container::{emit_relocations, encoded},
    geometry::{EmittedRange, Layout, MaterialTarget, emit_geometry},
};
use crate::model::{
    AssetPath,
    geometry::{Material, Model, ModelInstance, TextureInfo},
};
use anyhow::{Result, anyhow, ensure};
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub struct ModelFragment {
    pub bytes: Vec<u8>,
    pub rtb: Vec<u8>,
    pub ptx: Vec<u8>,
    pub rtt: Vec<u8>,
    pub ranges: Vec<EmittedRange>,
    pub pointer_count: usize,
    pub physical_object_offset: usize,
    /// Selected instance wrapper, including its optional per-LOD lighting pointer.
    pub instantiated_object_offset: usize,
    pub coverage: Vec<String>,
}
/// Source-free model/material fragment for focused reconstruction and editing tests. This includes
/// generated material, TextureInfo/name, IPO/PO/LOD, radiosity and pointer-file relationships, but not a world SNA.
/// Runtime/unknown material state and collision/spatial data are explicitly unsupported.
pub fn write_model_fragment(
    model: &Model,
    instance: &ModelInstance,
    materials: &[Material],
    base: u32,
) -> Result<ModelFragment> {
    instance.validate_model(model)?;
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "fragment base must be aligned/nonzero"
    );
    let mut layout = Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
    };
    let physical_object = layout.allocate(
        "physical_object",
        &encoded(&PhysicalObjectWire {
            visual_set: 0,
            collision_set: 0,
            bounding_volume: 0,
        })?,
    )?;
    let instance_object = layout.allocate(
        "instantiated_object",
        &encoded(&InstantiatedObjectWire {
            physical_object: 0,
            radiosity: 0,
        })?,
    )?;
    layout.pointer(
        instance_object + InstantiatedObjectWire::PHYSICAL_OBJECT,
        physical_object,
    )?;
    if !instance.radiosity.is_empty() {
        let lighting = layout.allocate(
            "radiosity/header",
            &encoded(&RadiosityWire {
                lod_count: 1,
                reserved: 0,
                lods: 0,
            })?,
        )?;
        let lod = layout.allocate(
            "radiosity/lod",
            &encoded(&RadiosityLodWire {
                vertex_count: instance.radiosity.len().try_into()?,
                reserved: 0,
                colors: 0,
            })?,
        )?;
        let mut colors = Vec::with_capacity(instance.radiosity.len() * 8);
        for &components in &instance.radiosity {
            colors.extend(encoded(&RadiosityColorWire { components })?);
        }
        let colors = layout.allocate("radiosity/colors", &colors)?;
        layout.pointer(
            instance_object + InstantiatedObjectWire::RADIOSITY,
            lighting,
        )?;
        layout.pointer(lighting + RadiosityWire::LODS, lod)?;
        layout.pointer(lod + RadiosityLodWire::COLORS, colors)?;
    }
    let visual_set = layout.allocate(
        "visual_set",
        &encoded(&VisualSetWire {
            flags: 0,
            lods: 1,
            kind: 0,
            distances: 0,
            objects: 0,
        })?,
    )?;
    layout.pointer(physical_object + PhysicalObjectWire::VISUAL_SET, visual_set)?;
    let distance = layout.allocate(
        "lod_distance",
        &instance.lod_distance.to_bits().to_le_bytes(),
    )?;
    layout.pointer(visual_set + VisualSetWire::DISTANCES, distance)?;
    let object = layout.allocate("lod_object", &[0; 4])?;
    layout.pointer(visual_set + VisualSetWire::OBJECTS, object)?;
    let mut targets = BTreeMap::new();
    let mut textures: BTreeMap<(AssetPath, String), (usize, TextureInfo)> = BTreeMap::new();
    for m in materials {
        m.validate()?;
        ensure!(
            !targets.contains_key(&m.path),
            "duplicate material definition: {}",
            m.path
        );
        let game_material = layout.allocate(
            format!("materials/{}/game", m.path.file()),
            &encoded(&GameMaterialWire {
                visual: 0,
                mechanics: 0,
                sound: m.sound_material,
                collision: 0,
            })?,
        )?;
        let visual_material = layout.allocate(
            format!("materials/{}/visual", m.path.file()),
            &encoded(&VisualMaterialWire {
                flags: m.flags,
                ambient: m.ambient,
                diffuse: m.diffuse,
                specular: m.specular,
                color: m.color,
                unknown_44: 0,
                texture: 0,
                scroll_position: m.scroll_position,
                scroll_speed: m.scroll_speed,
                scroll_mode: m.scroll_mode,
                refresh: 0,
                animation_first: 0,
                animation_current: 0,
                animation_count: 0,
                reserved: 0,
                animation_time: 0,
                properties: [0; 4],
                runtime_tail: [0; 3],
            })?,
        )?;
        layout.pointer(game_material + GameMaterialWire::VISUAL, visual_material)?;
        if let (Some(reference), Some(info)) = (&m.texture, &m.texture_info) {
            let name = info
                .source_name
                .clone()
                .unwrap_or_else(|| reference.file().replace('/', "\\"));
            let texture_key = (reference.clone(), name.clone());
            let t = if let Some((offset, previous)) = textures.get(&texture_key) {
                ensure!(
                    previous == info,
                    "shared texture has conflicting sampling metadata"
                );
                *offset
            } else {
                ensure!(name.len() < 80, "Hype TextureInfo name exceeds 79 bytes");
                let mut raw_name = [0; 80];
                raw_name[..name.len()].copy_from_slice(name.as_bytes());
                let t = layout.allocate(
                    format!("textures/{}", reference.file()),
                    &encoded(&TextureInfoWire {
                        runtime_head: [0; 2],
                        flags: info.flags,
                        stored_height: info.stored_size[1],
                        stored_width: info.stored_size[0],
                        unknown_14: 0,
                        runtime_state: [0; 6],
                        logical_height: info.logical_size[1],
                        logical_width: info.logical_size[0],
                        remaining_state: [0; 11],
                        name: raw_name,
                        sample_flags: 0,
                        addressing: info.addressing,
                        reserved: 0,
                        runtime_tail: 0,
                    })?,
                )?;
                textures.insert(texture_key, (t, info.clone()));
                t
            };
            layout.pointer(visual_material + VisualMaterialWire::TEXTURE, t)?;
        }
        targets.insert(
            m.path.clone(),
            MaterialTarget {
                address: base
                    .checked_add(game_material.try_into()?)
                    .ok_or_else(|| anyhow!("fragment base overflow"))?,
                block: (6, 2),
            },
        );
    }
    let geometry = emit_geometry(&mut layout, model, &targets)?;
    layout.pointer(object, geometry)?;
    layout.rows.sort_by_key(|r| r.value);
    let rtb = emit_relocations(BlockKey(6, 2), &layout.rows)?;
    let mut texture_pointers = Vec::new();
    let mut texture_rows = Vec::new();
    for (offset, _) in textures.values() {
        let address = base
            .checked_add((*offset).try_into()?)
            .ok_or_else(|| anyhow!("texture address overflow"))?;
        texture_pointers.push(address);
        texture_rows.push(RelocRow {
            value: address,
            module: 6,
            id: 2,
            unknown: [0; 2],
        });
    }
    ensure!(
        texture_pointers.len() <= 1024,
        "fragment exceeds Hype texture table capacity"
    );
    let loaded_textures = texture_pointers.len() as u32;
    texture_pointers.resize(1024, 0);
    let memory_channels = (0..1024)
        .map(|i| if i < loaded_textures { 1 } else { 0 })
        .collect();
    let ptx = encoded(&LevelTexturesWire {
        capacity: 1024,
        level_slots: 1024,
        pointers: texture_pointers,
        loaded_textures,
        memory_channels,
        current_channel: 1,
    })?;
    // Montreal RTT entries identify pointer VALUES, unlike RTB pointer-site addresses.
    let rtt = emit_relocations(BlockKey(5, 1), &texture_rows)?;
    let coverage = vec![
        "Rebuilt supported geometry arrays, material coefficients/texture names, source texture \
            dimensions/addressing, point-material sharing, one IPO/PO/visual-set/LOD with exact \
            per-position radiosity, and emitted RTB/RTT references.".into(),
        "Controlled fragment only: unknown material/texture runtime fields are initialized, not claimed \
            byte-exact; collision, spatial acceleration, other LODs, scene lighting/matrices/world records, \
            original Fix/level texture slot/channel placement, and source allocator bookkeeping are outside \
            this export.".into(),
    ];
    Ok(ModelFragment {
        bytes: layout.bytes,
        rtb,
        ptx,
        rtt,
        ranges: layout.ranges,
        pointer_count: layout.rows.len(),
        physical_object_offset: physical_object,
        instantiated_object_offset: instance_object,
        coverage,
    })
}
