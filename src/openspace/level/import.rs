//! Root selection and useful model/material decoding.
use super::records::*;
use super::{
    container::{u32_at, wire},
    session::{RootedNode, SourceSession},
};
use crate::model::{
    AssetPath, AssetRef, Selection,
    geometry::{Material, Model, ModelInstance, Submesh, TextureInfo, VisualMaterial},
};
use anyhow::{Context, Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelSummary {
    pub level: String,
    pub actual_world: String,
    pub dynamic_world: String,
    pub father_sector: String,
    pub rooted_nodes: usize,
    pub rooted_models: usize,
    pub texture_references: usize,
    pub populated_blocks: Vec<(String, usize)>,
}
#[derive(Debug, Clone)]
pub struct ModelImport {
    pub sprites: Vec<crate::model::sprite::Sprite>,
    pub model: Model,
    pub instance: ModelInstance,
    pub materials: Vec<Material>,
    pub source: LevelSummary,
    pub diagnostics: Vec<String>,
}
pub(super) fn summary(
    ctx: &SourceSession,
    selection: &Selection,
    tree: &[RootedNode],
) -> Result<LevelSummary> {
    let mut models = 0;
    for node in tree {
        if matches!(u32_at(ctx.bytes(node.address, 4)?, 0)?, 0xd | 0x15) {
            models += 1;
        }
    }
    let textures = ctx.texture_addresses()?;
    Ok(LevelSummary {
        level: selection.level.clone(),
        actual_world: ctx.label(ctx.root(0x10)?)?,
        dynamic_world: ctx.label(ctx.root(0x14)?)?,
        father_sector: ctx.label(ctx.root(0x18)?)?,
        rooted_nodes: tree.len(),
        rooted_models: models,
        texture_references: textures.len(),
        populated_blocks: ctx
            .actual
            .iter()
            .map(|(k, o)| {
                (
                    format!("{o:?}:{:02x}:{:02x}", k.0, k.1),
                    ctx.blocks[o][k].data.len(),
                )
            })
            .collect(),
    })
}
pub fn inspect_level(game: &Path, selection: &Selection) -> Result<LevelSummary> {
    let session = SourceSession::open(game, selection)?;
    summary(&session, selection, &session.tree()?)
}
/// Selects the first real IPO in rooted child order; unsupported geometry is an error, never a scanner fallback.
pub fn import_first_model(game: &Path, selection: &Selection) -> Result<ModelImport> {
    let session = SourceSession::open(game, selection)?;
    let tree = session.tree()?;
    let source = summary(&session, selection, &tree)?;
    let selected = tree
        .iter()
        .find(|node| {
            session
                .bytes(node.address, 4)
                .is_ok_and(|b| matches!(u32_at(b, 0), Ok(0xd | 0x15)))
        })
        .ok_or_else(|| anyhow!("rooted world has no IPO model"))?;
    let mut materials = Vec::new();
    let mut ids = BTreeMap::new();
    let mut imported = import_node_model(&session, selected, source, &mut materials, &mut ids)?;
    imported.materials = materials;
    Ok(imported)
}

pub(super) fn import_node_model(
    session: &SourceSession,
    selected: &RootedNode,
    source: LevelSummary,
    materials: &mut Vec<Material>,
    material_ids: &mut BTreeMap<Address, AssetPath>,
) -> Result<ModelImport> {
    let ipo = session.required(selected.address, SuperObjectWire::DATA)?;
    let po = session.required(ipo, InstantiatedObjectWire::PHYSICAL_OBJECT)?;
    let mut imported = import_physical_model(
        session,
        po,
        Some(ipo),
        &AssetPath::new(format!("level:/models/{}", selected.path))?,
        source,
        materials,
        material_ids,
    )?;
    imported.model.name = selected.path.clone();
    imported.instance.name = selected.path.clone();
    imported.instance.transforms = selected.transforms.clone();
    Ok(imported)
}
pub(super) fn import_physical_model(
    session: &SourceSession,
    physical_object: Address,
    instance_object: Option<Address>,
    path: &AssetPath,
    source: LevelSummary,
    materials: &mut Vec<Material>,
    material_ids: &mut BTreeMap<Address, AssetPath>,
) -> Result<ModelImport> {
    let visual = session.required(physical_object, PhysicalObjectWire::VISUAL_SET)?;
    let visual_header: VisualSetWire = wire(session.bytes(visual, 16)?)?;
    let lods = visual_header.lods;
    let kind = visual_header.kind;
    ensure!(
        lods > 0 && kind == 0,
        "first rooted model has unsupported visual set kind {kind}/LOD count {lods}"
    );
    let geometry = session.required(session.required(visual, VisualSetWire::OBJECTS)?, 0)?;
    let header: GeometryWire = wire(session.bytes(geometry, 48)?)?;
    ensure!(
        header.point_count > 0 && header.point_count <= i16::MAX as u16 && header.element_count > 0,
        "invalid geometry counts"
    );
    let lod_distance =
        session.floats::<1>(session.required(visual, VisualSetWire::DISTANCES)?, 1)?[0][0];
    let model_path = path.to_string();
    let mut diagnostics = Vec::new();
    diagnostics.push(format!(
        "Visual set has {lods} LOD(s), kind {kind}; selected LOD 0 at distance {lod_distance}."
    ));
    if header.spatial_data != 0 {
        diagnostics.push("Geometry has source spatial acceleration data; this subset does not decode/reconstruct it.".into());
    }
    let radiosity = if let Some(ipo) = instance_object {
        session.radiosity(ipo, 0, usize::from(lods), usize::from(header.point_count))?
    } else {
        Vec::new()
    };
    if !radiosity.is_empty() {
        diagnostics.push(format!("Retained {} exact signed per-position radiosity colors for IPO LOD 0; full scene/dynamic \
            lighting is outside this slice.", radiosity.len()));
    }
    if lods > 1 {
        diagnostics.push(format!(
            "Visual set has {lods} LODs; selected first LOD only."
        ));
    }
    let types = session.short_arrays::<1>(
        session.required(geometry, GeometryWire::ELEMENT_TYPES)?,
        header.element_count as usize,
    )?;
    let table = session.required(geometry, GeometryWire::ELEMENTS)?;
    let mut submeshes = Vec::new();
    let mut sprite_elements = Vec::new();
    let mut sprites = Vec::new();
    for (i, [kind]) in types.iter().enumerate() {
        if *kind == 3 {
            let (element, definitions) = super::sprite::read_element(
                session,
                session.required(table, i * 4)?,
                i,
                materials,
                material_ids,
                &mut diagnostics,
            )?;
            sprite_elements.push(element);
            sprites.extend(definitions);
            continue;
        }
        ensure!(
            *kind == 1,
            "unsupported geometric element {kind} in first model"
        );
        let element = session.required(table, i * 4)?;
        let triangle_header: TrianglesWire = wire(session.bytes(element, 32)?)?;
        let game_material = session.required(element, TrianglesWire::MATERIAL)?;
        let m = load_material(
            session,
            game_material,
            materials,
            material_ids,
            &mut diagnostics,
        )?;
        let normals = if let Some(a) = session.pointer(element, TrianglesWire::NORMALS)? {
            session.floats::<3>(a, triangle_header.triangle_count as usize)?
        } else {
            Vec::new()
        };
        let indices = if triangle_header.point_count > 0 {
            session
                .short_arrays::<1>(
                    session.required(element, TrianglesWire::POINTS)?,
                    triangle_header.point_count as usize,
                )?
                .into_iter()
                .map(|v| v[0])
                .collect()
        } else {
            Vec::new()
        };
        submeshes.push(Submesh {
            source_index: Some(i.try_into()?),
            material: m,
            // Zero-count arrays may have null pointers (tour); the source's
            // zero-count loops never read them.
            triangles: if triangle_header.triangle_count == 0 {
                Vec::new()
            } else {
                session.short_arrays::<3>(
                    session.required(element, TrianglesWire::TRIANGLES)?,
                    triangle_header.triangle_count as usize,
                )?
            },
            texcoords: if triangle_header.uv_count == 0 {
                Vec::new()
            } else {
                session.floats::<2>(
                    session.required(element, TrianglesWire::UVS)?,
                    triangle_header.uv_count as usize,
                )?
            },
            uv_indices: if triangle_header.triangle_count == 0 {
                Vec::new()
            } else {
                session.short_arrays::<3>(
                    session.required(element, TrianglesWire::UV_INDICES)?,
                    triangle_header.triangle_count as usize,
                )?
            },
            face_normals: normals,
            vertex_indices: indices,
        });
    }
    let mut vertex_materials = Vec::new();
    if let Some(table) = session.pointer(geometry, GeometryWire::POINT_MATERIALS)? {
        for i in 0..header.point_count as usize {
            let game_material = session.pointer(table, i * 4)?;
            vertex_materials.push(
                game_material
                    .map(|game_material| {
                        load_material(
                            session,
                            game_material,
                            materials,
                            material_ids,
                            &mut diagnostics,
                        )
                    })
                    .transpose()?,
            );
        }
    }
    let model = Model {
        path: AssetPath::new(model_path)?,
        name: path.file().to_owned(),
        positions: session.floats::<3>(
            session.required(geometry, GeometryWire::POINTS)?,
            header.point_count as usize,
        )?,
        normals: if let Some(a) = session.pointer(geometry, GeometryWire::NORMALS)? {
            session.floats::<3>(a, header.point_count as usize)?
        } else {
            Vec::new()
        },
        submeshes,
        sprite_elements,
        vertex_materials,
    };
    let instance = ModelInstance {
        path: AssetPath::new(format!("{}/instance", model.path))?,
        name: model.name.clone(),
        model: AssetRef::from_path(model.path.clone()),
        lod_distance,
        radiosity,
        transforms: Vec::new(),
    };
    instance.validate_model(&model)?;
    Ok(ModelImport {
        sprites,
        instance,
        model,
        materials: Vec::new(),
        source,
        diagnostics,
    })
}

pub(super) fn load_material(
    session: &SourceSession,
    game_material: Address,
    materials: &mut Vec<Material>,
    ids: &mut BTreeMap<Address, AssetPath>,
    diagnostics: &mut Vec<String>,
) -> Result<AssetRef<Material>> {
    if let Some(p) = ids.get(&game_material) {
        return Ok(AssetRef::from_path(p.clone()));
    }
    let identity = session.source_identity(game_material)?;
    let role = match identity.role {
        crate::model::ai::SourceRole::Fix => "fix",
        crate::model::ai::SourceRole::Level => "level",
    };
    let path = AssetPath::new(format!(
        "{role}:/materials/{:02x}-{:02x}/{}",
        identity.block[0], identity.block[1], identity.allocation
    ))?;
    let game: GameMaterialWire = wire(session.bytes(game_material, 16)?)?;
    let collision = match game.collision {
        0 => crate::model::collision::CollisionProperties::Absent,
        u32::MAX => crate::model::collision::CollisionProperties::Default,
        _ => {
            let h: CollisionMaterialWire =
                wire(session.bytes(session.required(game_material, 12)?, 4)?)?;
            crate::model::collision::CollisionProperties::Surface {
                kind: h.kind,
                identifier: h.identifier,
            }
        }
    };
    let visual = if let Some(visual_material) =
        session.pointer(game_material, GameMaterialWire::VISUAL)?
    {
        let header: VisualMaterialWire = wire(session.bytes(visual_material, 132)?)?;
        let mut texture_info = None;
        let texture =
            if let Some(t) = session.pointer(visual_material, VisualMaterialWire::TEXTURE)? {
                let texture: TextureInfoWire = wire(session.bytes(t, 188)?)?;
                let name = texture.name.split(|&byte| byte == 0).next().unwrap();
                let name = std::str::from_utf8(name).context("texture name is not UTF-8/ASCII")?;
                texture_info = Some(TextureInfo {
                    source_name: Some(name.to_owned()),
                    stored_size: [texture.stored_width, texture.stored_height],
                    logical_size: [texture.logical_width, texture.logical_height],
                    flags: texture.flags,
                    addressing: texture.addressing,
                });
                let name = name.replace('\\', "/");
                Some(AssetPath::new(format!("texture:/{name}"))?)
            } else {
                None
            };
        let count = header.animation_count;
        if count > 0 {
            diagnostics.push(format!(
                "{path}: {count} animated texture frames remain unsupported"
            ));
        }
        Some(VisualMaterial {
            texture,
            texture_info,
            ambient: header.ambient,
            diffuse: header.diffuse,
            specular: header.specular,
            color: header.color,
            flags: header.flags,
            scroll_position: header.scroll_position,
            scroll_speed: header.scroll_speed,
            scroll_mode: header.scroll_mode,
        })
    } else {
        None
    };
    materials.push(Material {
        path: path.clone(),
        visual,
        sound_material: game.sound,
        collision,
    });
    ids.insert(game_material, path.clone());
    Ok(AssetRef::from_path(path))
}
