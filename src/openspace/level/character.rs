//! Montreal families/configurations and sampled channels, with session-scoped ownership.
use super::{
    container::{u32_at, wire},
    records::*,
    scene::SceneImport,
    session::SourceSession,
};
use crate::model::{
    AssetPath, AssetRef,
    animation::*,
    character::*,
    geometry::*,
    io::{Content, Diagnostic, Severity},
};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(super) struct Importer<'a> {
    pub session: &'a SourceSession,
    pub result: &'a mut SceneImport,
    pub geometry_ids: &'a mut BTreeMap<Address, AssetPath>,
    sounds: super::sound::SoundSites,
    actions: BTreeMap<Address, AssetPath>,
    pub material_ids: &'a mut BTreeMap<Address, AssetPath>,
    families: BTreeMap<Address, AssetPath>,
    pub collision_profiles: super::character_collision::Profiles,
    bounds: BTreeMap<Address, AssetPath>,
    mechanics: BTreeMap<Address, AssetPath>,
    states: BTreeMap<Address, StateRef>,
    configs: BTreeMap<Address, AssetPath>,
    clips: BTreeMap<Address, AssetPath>,
    physical: BTreeMap<Address, Option<AssetPath>>,
    matrices: BTreeMap<Address, TransformValue>,
    allocations: BTreeMap<(Owner, BlockKey), BTreeMap<usize, u32>>,
    names: Vec<Vec<String>>,
}
impl<'a> Importer<'a> {
    pub fn new(
        session: &'a SourceSession,
        game: &std::path::Path,
        selection: &crate::model::Selection,
        result: &'a mut SceneImport,
        geometry_ids: &'a mut BTreeMap<Address, AssetPath>,
        material_ids: &'a mut BTreeMap<Address, AssetPath>,
    ) -> Result<Self> {
        let mut names = Vec::new();
        for kind in 0..3 {
            let mut values = Vec::new();
            let count = u32_at(&session.gpt, 0x40 + kind * 12)? as usize;
            ensure!(count <= 10000, "object name table too large");
            let mut node = if count > 0 {
                Some(session.root(0x3c + kind * 12)?)
            } else {
                None
            };
            for _ in 0..count {
                let n = node.context("short object-name list")?;
                let name = session.required(n, 12)?;
                let data = &session.block(name)?.data[name.offset..];
                let end = data
                    .iter()
                    .take(4096)
                    .position(|&b| b == 0)
                    .context("unterminated object name")?;
                values.push(data[..end].iter().map(|&b| char::from(b)).collect());
                node = session.pointer(n, 4)?;
            }
            values.reverse();
            names.push(values);
        }
        Ok(Self {
            sounds: super::sound::SoundSites::open(game, selection, session)?,
            actions: BTreeMap::new(),
            session,
            result,
            geometry_ids,
            material_ids,
            families: BTreeMap::new(),
            collision_profiles: Default::default(),
            bounds: BTreeMap::new(),
            mechanics: BTreeMap::new(),
            states: BTreeMap::new(),
            configs: BTreeMap::new(),
            clips: BTreeMap::new(),
            physical: BTreeMap::new(),
            matrices: BTreeMap::new(),
            allocations: BTreeMap::new(),
            names,
        })
    }
    fn note(&mut self, path: &AssetPath, message: String) {
        self.result.diagnostics.push(Diagnostic {
            code: "openspace.character.coverage".into(),
            asset: Some(path.clone()),
            severity: Severity::Warning,
            message,
        });
    }
    fn index(&mut self, a: Address) -> Result<u32> {
        let key = (a.owner, a.key);
        if !self.allocations.contains_key(&key) {
            self.allocations
                .insert(key, self.session.allocation_indices(a.owner, a.key)?);
        }
        self.allocations[&key]
            .get(&a.offset)
            .copied()
            .with_context(|| format!("reference is not an allocation start: {a:?}"))
    }
    pub(super) fn path(&mut self, a: Address, kind: &str) -> Result<AssetPath> {
        let index = self.index(a)?;
        AssetPath::new(format!("{}:/{kind}/{index}", role(a.owner)))
    }
    pub(super) fn list(&self, a: Address, field: usize) -> Result<Vec<Address>> {
        super::source_view::list(&mut super::source_view::FileView(self.session), a, field)
    }

    fn state_index(&self, site: Address, field: usize) -> Result<Option<u32>> {
        let value = u32_at(self.session.bytes(site, field + 4)?, field)?;
        if value == 0 || value == u32::MAX {
            return Ok(None);
        }
        let target = self.session.required(site, field)?;
        Ok(Some(
            self.states
                .get(&target)
                .context("state reference outside its imported family")?
                .state,
        ))
    }
    pub fn perso(&mut self, so: Address, node: &str) -> Result<AssetRef<Perso>> {
        let perso = self.session.required(so, 4)?;
        self.perso_at(
            perso,
            AssetPath::new(format!("level:/persos/{node}"))?,
            node,
        )
    }
    pub(super) fn perso_at(
        &mut self,
        perso: Address,
        path: AssetPath,
        fallback_name: &str,
    ) -> Result<AssetRef<Perso>> {
        let parts = super::source_view::perso_parts(
            &mut super::source_view::FileView(self.session),
            perso,
        )?;
        let data = parts.data3d;
        let types = parts.object_types;
        let family = self.family(parts.family)?;
        let config = self
            .session
            .pointer(data, 16)?
            .map(|a| self.configuration(a))
            .transpose()?;
        let initial = self
            .session
            .pointer(data, 12)?
            .map(|a| self.configuration(a))
            .transpose()?;
        let value = Perso {
            path: path.clone(),
            name: self.names[2]
                .get(types[2] as usize)
                .cloned()
                .unwrap_or_else(|| fallback_name.into()),
            family,
            initial_state: self.state_index(data, 0)?,
            current_state: self.state_index(data, 4)?,
            secondary_state: self.state_index(data, 8)?,
            configuration: config,
            initial_configuration: initial,
            object_types: types,
            camera: self
                .session
                .pointer(perso, 20)?
                .map(|a| super::camera::import(self.session, a))
                .transpose()?,
            hit_points: self
                .session
                .pointer(perso, 4)?
                .map(|a| -> Result<HitPoints> {
                    let standard: StandardSaveWire = wire(self.session.bytes(a, 56)?)?;
                    Ok(HitPoints {
                        current: standard.hit_points_init,
                        maximum: standard.hit_points_max_init,
                        limit: standard.hit_points_max_max,
                    })
                })
                .transpose()?,
            routes: super::way::import_bindings(self.session, perso, &path, types, self.result)?,
            stock: super::inventory_gameplay::stock(self.session, perso)?,
            brain: None,
            particles: super::particle::component(self.session, perso)?,
            collisions: self
                .session
                .pointer(perso, 24)?
                .map(|a| self.collisions(a))
                .transpose()?,
        };
        value.validate()?;
        self.result.persos.push(value);
        Ok(AssetRef::from_path(path))
    }
    fn family(&mut self, address: Address) -> Result<AssetRef<Family>> {
        if let Some(path) = self.families.get(&address) {
            return Ok(AssetRef::from_path(path.clone()));
        }
        let header: FamilyWire = wire(self.session.bytes(address, 56)?)?;
        let index = header.index;
        let path = AssetPath::new(format!("{}:/families/{index}", role(address.owner)))?;
        self.families.insert(address, path.clone());
        let state_addresses = self.list(address, 16)?;
        for (i, &a) in state_addresses.iter().enumerate() {
            self.states.insert(
                a,
                StateRef {
                    family: AssetRef::from_path(path.clone()),
                    state: i as u32,
                },
            );
        }
        let mut states = Vec::new();
        for (i, &st) in state_addresses.iter().enumerate() {
            let state: CharacterStateWire = wire(self.session.bytes(st, 64)?)?;
            let definition = super::source_view::state_definition(
                &mut super::source_view::FileView(self.session),
                st,
                &state_addresses,
            )?;
            let animation = self
                .session
                .pointer(st, 12)?
                .map(|a| self.animation(a))
                .transpose()?;
            let mut transitions = Vec::new();
            for tr in self.list(st, 16)? {
                let raw: StateTransitionWire = wire(self.session.bytes(tr, 24)?)?;
                transitions.push(StateTransition {
                    target: self
                        .state_index(tr, 12)?
                        .context("null transition target")?,
                    via: self.state_index(tr, 16)?.context("null transition state")?,
                    linking_type: raw.linking_type,
                    unknown: raw.unknown,
                });
            }
            let mut prohibited = Vec::new();
            for p in self.list(st, 28)? {
                prohibited.push(self.state_index(p, 12)?.context("null prohibited state")?);
            }
            states.push(CharacterState {
                id: i as u32,
                animation,
                mechanics: self
                    .session
                    .pointer(st, 48)?
                    .map(|a| self.mechanics(a))
                    .transpose()?,
                speed: definition.speed,
                repeats: definition.repeats,
                transition_lock: definition.transition_lock,
                next: definition.next,
                transitions,
                prohibited,
                unknown: state
                    .unknown
                    .into_iter()
                    .chain(state.unknown_tail)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            });
        }
        let mut configs = Vec::new();
        for c in self.list(address, 32)? {
            configs.push(self.configuration(c)?);
        }
        let default_configuration = self
            .session
            .pointer(address, 28)?
            .map(|a| self.configuration(a))
            .transpose()?;
        let family = Family {
            path: path.clone(),
            object_type: index,
            bounding_geometry: self
                .session
                .pointer(address, 44)?
                .map(|a| self.bounding_geometry(a, "family-bounds"))
                .transpose()?,
            name: self.names[0]
                .get(index as usize)
                .cloned()
                .unwrap_or_else(|| format!("Family {index}")),
            states,
            configurations: configs,
            default_configuration,
            animation_bank: header.animation_bank,
            unknown: header.unknown,
        };
        family.validate()?;
        self.result.families.push(family);
        Ok(AssetRef::from_path(path))
    }
    pub(super) fn bounding_geometry(
        &mut self,
        address: Address,
        prefix: &str,
    ) -> Result<AssetRef<crate::model::collision::Collision>> {
        if let Some(path) = self.bounds.get(&address) {
            return Ok(AssetRef::from_path(path.clone()));
        }
        let name = format!("{prefix}/{}", self.index(address)?);
        let mut notes = Vec::new();
        let mut importer = super::collision::CollisionImporter {
            session: self.session,
            geometries: Vec::new(),
            materials: &mut self.result.materials,
            diagnostics: &mut notes,
            visual_geometries: self.geometry_ids,
            geometry_ids: BTreeMap::new(),
            material_ids: self.material_ids,
        };
        let reference = importer.geometry(address, &name)?;
        self.result.collisions.extend(importer.geometries);
        self.bounds.insert(address, reference.path.clone());
        for note in notes {
            self.note(&reference.path, note);
        }
        Ok(reference)
    }
    fn mechanics(
        &mut self,
        address: Address,
    ) -> Result<AssetRef<crate::model::mechanics::Mechanics>> {
        if let Some(path) = self.mechanics.get(&address) {
            return Ok(AssetRef::from_path(path.clone()));
        }
        let kind = u32_at(self.session.bytes(address, 4)?, 0)?;
        let length = super::mechanics::size(kind)?;
        let block = self.session.block(address)?;
        let start = block
            .base
            .checked_add(address.offset.try_into()?)
            .context("mechanics address overflow")?;
        let end = start
            .checked_add(length as u32)
            .context("mechanics extent overflow")?;
        ensure!(
            self.session
                .pointers
                .range((address.owner, address.key, start)..(address.owner, address.key, end))
                .next()
                .is_none(),
            "mechanics card has an unrepresented pointer"
        );
        let path = self.path(address, "mechanics")?;
        let value = super::read_mechanics(self.session.bytes(address, length)?, path.clone())?;
        if matches!(
            value.data,
            crate::model::mechanics::MechanicsData::Unresolved { .. }
        ) {
            self.note(&path,format!("Mechanics kind {kind} has explicitly preserved, unpromoted fields; dynamics behavior is not implemented"));
        }
        self.mechanics.insert(address, path.clone());
        self.result.mechanics.push(value);
        Ok(AssetRef::from_path(path))
    }
    pub(super) fn configuration(&mut self, address: Address) -> Result<AssetRef<ObjectList>> {
        if let Some(path) = self.configs.get(&address) {
            return Ok(AssetRef::from_path(path.clone()));
        }
        let path = self.path(address, "configurations")?;
        self.configs.insert(address, path.clone());
        let header: ObjectListWire = wire(self.session.bytes(address, 24)?)?;
        let parts = self.parts(self.session.pointer(address, 12)?, header.count as usize)?;
        let reset_parts = self
            .session
            .pointer(address, 16)?
            .map(|data| self.parts(Some(data), header.count as usize))
            .transpose()?;
        let config = ObjectList {
            path: path.clone(),
            parts,
            reset_parts,
            unknown: header.unknown,
        };
        config.validate()?;
        self.result.configurations.push(config);
        Ok(AssetRef::from_path(path))
    }
    fn parts(&mut self, data: Option<Address>, count: usize) -> Result<Vec<ObjectPart>> {
        ensure!(
            count == 0 || data.is_some(),
            "missing configuration entries"
        );
        let mut parts = Vec::new();
        for i in 0..count {
            let table = data.unwrap();
            let entry = Address {
                offset: table.offset + i * 20,
                ..table
            };
            let raw: ObjectPartWire = wire(self.session.bytes(entry, 20)?)?;
            let kind = match raw.kind {
                -1 => ObjectPartKind::Absent,
                0 => ObjectPartKind::Model,
                5 => ObjectPartKind::Action,
                other => anyhow::bail!("unsupported configured object kind {other}"),
            };
            let instance = if kind == ObjectPartKind::Model {
                self.session
                    .pointer(entry, 4)?
                    .map(|po| self.physical(po))
                    .transpose()?
                    .flatten()
            } else {
                None
            };
            let scale = if kind == ObjectPartKind::Model {
                self.session
                    .pointer(entry, 0)?
                    .map(|a| self.session.floats::<3>(a, 1).map(|v| v[0]))
                    .transpose()?
            } else {
                ensure!(
                    raw.scale == 0,
                    "non-model configuration has an unrepresented scale"
                );
                None
            };
            let action = if kind == ObjectPartKind::Action {
                Some(self.action(self.session.required(entry, 4)?)?)
            } else {
                ensure!(
                    kind != ObjectPartKind::Absent || raw.content == 0,
                    "absent configuration contains unrepresented content"
                );
                None
            };
            parts.push(ObjectPart {
                instance: instance.map(AssetRef::from_path),
                action,
                kind,
                scale,
                unknown: raw
                    .unknown
                    .into_iter()
                    .chain(raw.tail)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            });
        }
        Ok(parts)
    }
    fn physical(&mut self, po: Address) -> Result<Option<AssetPath>> {
        if let Some(path) = self.physical.get(&po) {
            return Ok(path.clone());
        }
        let path = self.path(po, "objects")?;
        let Some(visual) = self.session.pointer(po, 0)? else {
            self.physical.insert(po, None);
            return Ok(None);
        };
        let geometry = self
            .session
            .required(self.session.required(visual, 12)?, 0)?;
        let h: GeometryWire = wire(self.session.bytes(geometry, 48)?)?;
        let kinds = self.session.short_arrays::<1>(
            self.session.required(geometry, 24)?,
            h.element_count as usize,
        )?;
        if kinds.iter().any(|&[kind]| !matches!(kind, 1 | 3)) {
            self.note(
                &path,
                format!("Configured object has unsupported geometric kinds {kinds:?}"),
            );
            self.physical.insert(po, None);
            return Ok(None);
        }
        let model_path = self.path(geometry, "models/geometry")?;
        let mut imported = super::import::import_physical_model(
            self.session,
            po,
            None,
            &model_path,
            self.result.source.clone(),
            &mut self.result.materials,
            self.material_ids,
        )?;
        imported.instance.path = path.clone();
        self.result.sprites.extend(imported.sprites);
        if let Some(model) = self.geometry_ids.get(&geometry) {
            imported.instance.model = AssetRef::from_path(model.clone());
        } else {
            self.geometry_ids
                .insert(geometry, imported.model.path.clone());
            self.result.models.push(imported.model);
        }
        self.result.instances.push(imported.instance);
        self.physical.insert(po, Some(path.clone()));
        Ok(Some(path))
    }
    fn matrix(&mut self, address: Address) -> Result<TransformLink> {
        let entry = self.index(address)?;
        if !self.matrices.contains_key(&address) {
            let value = super::source_view::transform(
                &mut super::source_view::FileView(self.session),
                address,
            )?;
            self.matrices.insert(address, value);
        }
        Ok(TransformLink {
            pool: AssetRef::from_path(pool_path(address)?),
            entry,
        })
    }
    fn action(&mut self, address: Address) -> Result<AssetRef<AnimationAction>> {
        if let Some(path) = self.actions.get(&address) {
            return Ok(AssetRef::from_path(path.clone()));
        }
        let path = self.path(address, "animation-actions")?;
        let raw: AnimationActionWire = wire(self.session.bytes(address, 12)?)?;
        let operation = match raw.kind {
            0 => AnimationOperation::Sound {
                event: match self.sounds.events.get(&address) {
                    Some(event) => event.clone(),
                    None if raw.payload == 0 || raw.payload == u32::MAX => None,
                    None => anyhow::bail!("sound action has no SND binding: {address:?}"),
                },
            },
            1 => AnimationOperation::Mechanics {
                command: raw.payload,
            },
            2 => AnimationOperation::Attachment { value: raw.payload },
            3 => AnimationOperation::AttachmentControl {
                command: raw.payload,
            },
            4 => AnimationOperation::Camera {
                command: raw.payload,
            },
            kind => AnimationOperation::Unknown {
                kind,
                payload: raw.payload.to_le_bytes(),
            },
        };
        let action = AnimationAction {
            path: path.clone(),
            operation,
            unknown: raw.unknown,
        };
        action.validate()?;
        self.result.actions.push(action);
        self.actions.insert(address, path.clone());
        Ok(AssetRef::from_path(path))
    }
    fn animation(&mut self, address: Address) -> Result<AssetRef<AnimationClip>> {
        if let Some(path) = self.clips.get(&address) {
            return Ok(AssetRef::from_path(path.clone()));
        }
        let path = self.path(address, "animations")?;
        self.clips.insert(address, path.clone());
        let h: AnimationWire = wire(self.session.bytes(address, 116)?)?;
        let table = self.session.required(address, 0)?;
        let mut frames = Vec::new();
        let mut samples = Vec::new();
        let mut sample_ids = BTreeMap::new();
        for i in 0..h.frame_count as usize {
            let frame = Address {
                offset: table.offset + i * 16,
                ..table
            };
            let f: AnimationFrameWire = wire(self.session.bytes(frame, 16)?)?;
            let channel_table = self.session.required(frame, 0)?;
            let mut channel_samples = Vec::new();
            for c in 0..h.channel_count as usize {
                let channel = self.session.required(channel_table, c * 4)?;
                let id = if let Some(&id) = sample_ids.get(&channel) {
                    id
                } else {
                    let ch: AnimationChannelWire = wire(self.session.bytes(channel, 16)?)?;
                    let transform = if ch.transform > 1 {
                        Some(self.matrix(self.session.required(channel, 0)?)?)
                    } else {
                        None
                    };
                    let id = samples.len() as u32;
                    samples.push(ChannelSample {
                        transform,
                        object: (ch.object != u16::MAX).then_some(ch.object),
                        channel: (ch.channel != 255).then_some(ch.channel),
                        visibility: ch.visibility,
                        opacity: ch.opacity,
                        kind: match ch.kind {
                            0 => ChannelKind::Object,
                            2 => ChannelKind::Hidden,
                            4 => ChannelKind::Light,
                            v => ChannelKind::Unknown(v),
                        },
                        unknown: ch
                            .unknown
                            .into_iter()
                            .chain(ch.tail)
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap(),
                    });
                    sample_ids.insert(channel, id);
                    id
                };
                channel_samples.push(id);
            }
            let mut hierarchy = Vec::new();
            if let Some(hierarchy_header) = self.session.pointer(frame, 12)? {
                let count = u32_at(self.session.bytes(hierarchy_header, 8)?, 0)? as usize;
                ensure!(count <= 255, "excessive animation hierarchy");
                if count > 0 {
                    for [child, parent] in self
                        .session
                        .short_arrays::<2>(self.session.required(hierarchy_header, 4)?, count)?
                    {
                        hierarchy.push([child.try_into()?, parent.try_into()?]);
                    }
                }
            }
            let motion_rotation = if f.rotation != 0 {
                Some(
                    self.session
                        .floats::<3>(self.session.required(frame, 4)?, 3)?
                        .try_into()
                        .unwrap(),
                )
            } else {
                None
            };
            let motion_translation = if f.translation != 0 {
                Some(
                    self.session
                        .floats::<3>(self.session.required(frame, 8)?, 1)?[0],
                )
            } else {
                None
            };
            frames.push(AnimationFrame {
                samples: channel_samples,
                hierarchy,
                motion_rotation,
                motion_translation,
            });
        }
        let mut cues = Vec::new();
        if h.event_count > 0 {
            let events = self.session.required(address, 8)?;
            for i in 0..usize::from(h.event_count) {
                let entry = Address {
                    offset: events.offset + i * 8,
                    ..events
                };
                let cue: AnimationCueWire = wire(self.session.bytes(entry, 8)?)?;
                cues.push(AnimationCue {
                    action: self.action(self.session.required(entry, 0)?)?,
                    frame: cue.frame,
                    channel: cue.channel,
                    configuration_slot: cue.configuration_slot,
                });
            }
        }
        if h.related != 0 || h.unknown_108 != 0 || h.morph_count != 0 {
            self.note(
                &path,
                "Animation morph/related data remain outside sampled playback/reconstruction"
                    .into(),
            );
        }
        let mut unknown = vec![h.unknown];
        unknown.extend(h.unknown_12.to_le_bytes());
        unknown.push(h.unknown_17);
        let clip = AnimationClip {
            path: path.clone(),
            channel_count: h.channel_count,
            frame_rate: h.speed,
            speed: SourceTransform {
                translation: h.speed_matrix.translation,
                rotation: h.speed_matrix.rotation,
                basis: h.speed_matrix.basis,
            },
            samples,
            frames,
            cues,
            morph_count: h.morph_count,
            unknown,
        };
        clip.validate()
            .with_context(|| format!("animation {path}"))?;
        self.result.animations.push(clip);
        Ok(AssetRef::from_path(path))
    }
    pub fn finish(mut self) -> Result<BTreeMap<Address, StateRef>> {
        self.result
            .sound_definitions
            .push(self.sounds.library.clone());
        let mut pools: BTreeMap<AssetPath, Vec<TransformEntry>> = BTreeMap::new();
        let matrices = std::mem::take(&mut self.matrices);
        for (address, value) in matrices {
            let index = self.index(address)?;
            pools
                .entry(pool_path(address)?)
                .or_default()
                .push(TransformEntry {
                    id: index,
                    source_index: Some(index),
                    value,
                });
        }
        for (path, entries) in pools {
            let pool = TransformPool { path, entries };
            pool.validate()?;
            self.result.transform_pools.push(pool);
        }
        Ok(self.states)
    }
}
fn role(owner: Owner) -> &'static str {
    match owner {
        Owner::Fix => "fix",
        Owner::Level => "level",
    }
}
fn pool_path(a: Address) -> Result<AssetPath> {
    AssetPath::new(format!(
        "{}:/transforms/{}-{}",
        role(a.owner),
        a.key.0,
        a.key.1
    ))
}
