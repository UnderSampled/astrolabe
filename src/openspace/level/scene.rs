//! Rooted scene import. Address maps are transient identity caches, never Rete data.
use super::{
    container::{u32_at, wire},
    import::{LevelSummary, import_node_model, summary},
    records::*,
    session::SourceSession,
};
use crate::model::{
    AssetPath, AssetRef, Selection,
    ai::{SourceObjectIdentity, SourceRole},
    geometry::{Material, Model, ModelInstance},
    io::{Content, Diagnostic, Severity},
    scene::{NodeKind, Scene, SceneNode},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

const GLOBAL_PERSO_ALLOCATION: u32 = 15;

pub(super) fn global_perso_source(
    session: &SourceSession,
) -> Result<Option<(Address, SourceObjectIdentity)>> {
    let block = BlockKey(5, 0);
    if !session
        .blocks
        .get(&Owner::Fix)
        .is_some_and(|blocks| blocks.contains_key(&block))
    {
        return Ok(None);
    }
    let Some((offset, _)) = session
        .allocation_indices(Owner::Fix, block)?
        .into_iter()
        .find(|(_, allocation)| *allocation == GLOBAL_PERSO_ALLOCATION)
    else {
        return Ok(None);
    };
    let address = Address {
        owner: Owner::Fix,
        key: block,
        offset,
    };
    let identity = session.source_identity(address)?;
    ensure!(
        identity
            == SourceObjectIdentity {
                role: SourceRole::Fix,
                block: [5, 0],
                allocation: GLOBAL_PERSO_ALLOCATION,
            },
        "Fix Global Perso allocation identity differs"
    );
    let header: PersoHeaderWire = wire(session.bytes(address, 20)?)?;
    ensure!(header.brain != 0, "Fix Global Perso has no Brain");
    let parts =
        super::source_view::perso_parts(&mut super::source_view::FileView(session), address)?;
    let family: FamilyWire = wire(session.bytes(parts.family, 56)?)?;
    ensure!(family.index == 0, "Fix Global Perso family index differs");
    Ok(Some((address, identity)))
}

pub struct SceneImport {
    pub armor: Option<crate::model::armor::ArmorCatalog>,
    pub dialogue_presentation: Option<crate::model::dialogue::DialoguePresentation>,
    pub dialogues: Vec<crate::model::dialogue::Dialogue>,
    pub ai_models: Vec<crate::model::ai::AiModel>,
    pub environments: Vec<crate::model::environment::Environment>,
    pub physical_environments: Vec<crate::model::environment::PhysicalEnvironment>,
    pub visual_environments: Vec<crate::model::environment::VisualEnvironment>,
    pub fogs: Vec<crate::model::environment::Fog>,
    pub collision_profiles: Vec<crate::model::character::CollisionProfile>,
    pub mechanics: Vec<crate::model::mechanics::Mechanics>,
    pub actions: Vec<crate::model::animation::AnimationAction>,
    pub sound_definitions: Vec<(AssetPath, crate::model::audio::SoundDefinitions)>,
    pub lights: Vec<crate::model::light::Light>,
    pub sprites: Vec<crate::model::sprite::Sprite>,
    pub particle_generators: Vec<crate::model::particle::ParticleGenerator>,
    pub scene: Scene,
    pub families: Vec<crate::model::character::Family>,
    pub persos: Vec<crate::model::character::Perso>,
    pub routes: Vec<crate::model::route::Route>,
    pub waypoints: Vec<crate::model::route::Waypoint>,
    pub configurations: Vec<crate::model::character::ObjectList>,
    pub animations: Vec<crate::model::animation::AnimationClip>,
    pub transform_pools: Vec<crate::model::animation::TransformPool>,
    pub models: Vec<Model>,
    pub collisions: Vec<crate::model::collision::Collision>,
    pub instances: Vec<ModelInstance>,
    pub materials: Vec<Material>,
    pub diagnostics: Vec<Diagnostic>,
    pub source: LevelSummary,
}

pub fn import_scene(game: &Path, selection: &Selection) -> Result<SceneImport> {
    let mut session = SourceSession::open(game, selection)?;
    if selection.level == "manoir" {
        super::dialogue::load_language(&mut session, game, selection)?;
    }
    let tree = session.tree()?;
    let source = summary(&session, selection, &tree)?;
    let mut result = SceneImport {
        armor: None,
        dialogue_presentation: None,
        dialogues: Vec::new(),
        ai_models: Vec::new(),
        environments: Vec::new(),
        physical_environments: Vec::new(),
        visual_environments: Vec::new(),
        fogs: Vec::new(),
        collision_profiles: Vec::new(),
        mechanics: Vec::new(),
        actions: Vec::new(),
        sound_definitions: Vec::new(),
        lights: Vec::new(),
        sprites: Vec::new(),
        particle_generators: Vec::new(),
        families: Vec::new(),
        persos: Vec::new(),
        routes: Vec::new(),
        waypoints: Vec::new(),
        configurations: Vec::new(),
        animations: Vec::new(),
        transform_pools: Vec::new(),
        scene: Scene {
            path: AssetPath::new("level:/scene")?,
            name: selection.level.clone(),
            main_actor: None,
            armor: None,
            hud: None,
            global_actors: Vec::new(),
            nodes: Vec::new(),
        },
        collisions: Vec::new(),
        models: Vec::new(),
        instances: Vec::new(),
        materials: Vec::new(),
        diagnostics: Vec::new(),
        source: source.clone(),
    };
    let mut geometry_ids: BTreeMap<Address, AssetPath> = BTreeMap::new();
    let mut material_ids = BTreeMap::new();
    for node in &tree {
        let header: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
        let kind = match header.kind {
            0 => NodeKind::World,
            4 => NodeKind::Perso,
            8 => NodeKind::Sector,
            0xd | 0x15 => NodeKind::Object,
            n => NodeKind::Unknown(n),
        };
        let mut instance = None;
        let mut source_visual_lod_count = None;

        if kind == NodeKind::Object {
            let ipo = session.required(node.address, SuperObjectWire::DATA)?;
            let po = session.required(ipo, InstantiatedObjectWire::PHYSICAL_OBJECT)?;
            let visual = session.required(po, PhysicalObjectWire::VISUAL_SET)?;
            let visual_header: VisualSetWire = wire(session.bytes(visual, 16)?)?;
            source_visual_lod_count = Some(visual_header.lods);
            let geometry =
                session.required(session.required(visual, VisualSetWire::OBJECTS)?, 0)?;
            let geo: GeometryWire = wire(session.bytes(geometry, 48)?)?;
            let kinds = session.short_arrays::<1>(
                session.required(geometry, GeometryWire::ELEMENT_TYPES)?,
                geo.element_count as usize,
            )?;
            if kinds.iter().any(|[kind]| !matches!(kind, 1 | 3)) {
                result.diagnostics.push(Diagnostic {
                    code: "openspace.scene.unsupported_geometry".into(),
                    asset: Some(result.scene.path.clone()),
                    severity: Severity::Warning,
                    message: format!(
                        "{}: geometric element kinds {kinds:?} require additional codecs",
                        node.path
                    ),
                });
            } else {
                let mut imported = import_node_model(
                    &session,
                    node,
                    source.clone(),
                    &mut result.materials,
                    &mut material_ids,
                )
                .with_context(|| format!("scene model {}", node.path))?;
                if let Some(path) = geometry_ids.get(&geometry) {
                    imported.instance.model = AssetRef::from_path(path.clone());
                } else {
                    geometry_ids.insert(geometry, imported.model.path.clone());
                    result.models.push(imported.model);
                }
                imported.instance.transforms.clear();
                result.sprites.extend(imported.sprites);
                instance = Some(AssetRef::from_path(imported.instance.path.clone()));
                for message in imported.diagnostics {
                    result.diagnostics.push(Diagnostic {
                        code: "openspace.scene.model_coverage".into(),
                        asset: Some(imported.instance.path.clone()),
                        severity: Severity::Info,
                        message,
                    });
                }
                result.instances.push(imported.instance);
            }
        }
        result.scene.nodes.push(SceneNode {
            properties: crate::model::scene::SceneNodeProperties {
                source_kind: Some(header.kind),
                source_visual_lod_count,
                transform_space: if header.last_compute_frame == u32::MAX {
                    crate::model::scene::TransformSpace::World
                } else {
                    crate::model::scene::TransformSpace::Local
                },
                initial_global_transform: session
                    .pointer(node.address, 56)?
                    .map(|a| session.transform(a))
                    .transpose()?,
                bounding_geometry: None,
                opacity: header.opacity,
            },
            id: node.path.clone(),
            name: node.path.rsplit('/').next().unwrap().into(),
            parent: node.path.rsplit_once('/').map(|(p, _)| p.into()),
            kind,
            transform: session
                .pointer(
                    node.address,
                    if header.last_compute_frame == u32::MAX {
                        36
                    } else {
                        SuperObjectWire::LOCAL_TRANSFORM
                    },
                )?
                .map(|a| session.transform(a))
                .transpose()?,
            instance,
            collision: None,
            perso: None,
            draw_mask: header.draw_mask,
            sector: None,
            light: None,
        });
    }
    let mut collision_notes = Vec::new();
    let mut collisions = super::collision::CollisionImporter {
        session: &session,
        geometries: Vec::new(),
        materials: &mut result.materials,
        diagnostics: &mut collision_notes,
        visual_geometries: &geometry_ids,
        geometry_ids: BTreeMap::new(),
        material_ids: &mut material_ids,
    };
    for (source, node) in tree.iter().zip(&mut result.scene.nodes) {
        if node.instance.is_some() {
            let ipo = session.required(source.address, SuperObjectWire::DATA)?;
            let po = session.required(ipo, InstantiatedObjectWire::PHYSICAL_OBJECT)?;
            node.collision = collisions
                .physical(po, &node.id)
                .with_context(|| format!("collision {}", node.id))?;
        }
    }
    result.collisions = collisions.geometries;
    result
        .diagnostics
        .extend(collision_notes.into_iter().map(|message| Diagnostic {
            code: "openspace.scene.material_coverage".into(),
            asset: Some(result.scene.path.clone()),
            severity: Severity::Info,
            message,
        }));
    let mut characters = super::character::Importer::new(
        &session,
        game,
        selection,
        &mut result,
        &mut geometry_ids,
        &mut material_ids,
    )?;
    let mut perso_refs = Vec::new();
    let mut actor_refs = BTreeMap::new();
    for (i, node) in tree.iter().enumerate() {
        if u32_at(session.bytes(node.address, 4)?, 0)? == 4 {
            let perso = session.required(node.address, SuperObjectWire::DATA)?;
            let reference = characters
                .perso(node.address, &node.path)
                .with_context(|| format!("perso {}", node.path))?;
            ensure!(
                actor_refs.insert(perso, reference.clone()).is_none(),
                "rooted Perso occurs more than once"
            );
            perso_refs.push((i, reference));
        }
    }
    let mut global_actor = None;
    if let Some((address, source)) = global_perso_source(&session)? {
        let perso = characters.perso_at(
            address,
            AssetPath::new(format!("fix:/persos/{GLOBAL_PERSO_ALLOCATION}"))?,
            "Global",
        )?;
        actor_refs.insert(address, perso.clone());
        global_actor = Some(crate::model::scene::GlobalActor { source, perso });
    }
    let rooted_template_seeds = actor_refs.keys().copied().collect::<BTreeSet<_>>();
    for template in super::ai::object_templates(&session, &rooted_template_seeds)? {
        let identity = session.source_identity(template)?;
        let role = if identity.role == SourceRole::Fix {
            "fix"
        } else {
            "level"
        };
        let path = AssetPath::new(format!(
            "{role}:/persos/{}-{}-{}",
            identity.block[0], identity.block[1], identity.allocation
        ))?;
        let reference = characters.perso_at(template, path, "Object template")?;
        actor_refs.insert(template, reference);
    }
    let mut bounds = Vec::new();
    for (i, node) in tree.iter().enumerate() {
        if let Some(a) = session.pointer(node.address, 52)? {
            bounds.push((
                i,
                characters
                    .bounding_geometry(a, "node-bounds")
                    .with_context(|| {
                        format!(
                            "picking bounds for {} at {}",
                            node.path,
                            session.label(a).unwrap_or_default()
                        )
                    })?,
            ));
        }
    }
    let state_references = characters.finish()?;
    if let Some(actor) = global_actor {
        result.scene.global_actors.push(actor);
    }
    for (i, bound) in bounds {
        result.scene.nodes[i].properties.bounding_geometry = Some(bound);
    }

    for (i, reference) in perso_refs {
        result.scene.nodes[i].perso = Some(reference);
    }
    super::lighting::import(&session, &tree, &mut result)?;
    super::ai::import(
        &session,
        game,
        selection,
        &state_references,
        &actor_refs,
        &mut result,
        &mut material_ids,
    )?;
    if selection.level == "manoir" {
        super::dialogue::import(&session, game, selection, &actor_refs, &mut result)?;
    }
    let armor = super::armor::import(game, selection).context("import shared Fix armor catalog")?;
    result.scene.armor = Some(AssetRef::from_path(armor.path.clone()));
    result.armor = Some(armor);
    result.scene.validate()?;
    Ok(result)
}
