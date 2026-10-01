//! Selected Hype inspection through relocated memory. OS access and pausing are
//! supplied by optional tooling; declarations, roots and meaning stay in OpenSpace.
//! Observations are transient projections, not a second persistent asset model.
use super::{
    container::{u32_at, wire},
    records::{FamilyWire, MainActorWire, PersoHeaderWire, SuperObjectWire},
    source_view::{self, RecordSource},
};
use crate::model::{
    AssetRef,
    ai::{BehaviorSelection, DesignerArray, DesignerValue},
    animation::TransformValue,
    character::{Family, Perso, StateRef},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{borrow::Cow, collections::BTreeSet};

pub const HYPE_EXECUTABLE_SHA256: &str =
    "048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da";
/// The controller validates readable mappings, process identity and its chosen
/// capture consistency. Implementations must reject missing/truncated ranges.
/// A completed-update contract is separate; this trait cannot establish one.
pub trait ReadMemory {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>>;
}
struct MemoryView<'a, M>(&'a mut M);
impl<M: ReadMemory> RecordSource for MemoryView<'_, M> {
    type Location = u32;
    fn add(&self, address: u32, offset: usize) -> Result<u32> {
        address
            .checked_add(offset.try_into()?)
            .context("32-bit source address overflow")
    }
    fn bytes(&mut self, address: u32, size: usize) -> Result<Cow<'_, [u8]>> {
        ensure!(
            (1..=1048576).contains(&size),
            "invalid bounded memory read size"
        );
        self.add(address, size - 1)?;
        let bytes = self.0.read(address, size)?;
        ensure!(bytes.len() == size, "truncated memory source response");
        Ok(Cow::Owned(bytes))
    }
    fn pointer(&mut self, address: u32, offset: usize) -> Result<Option<u32>> {
        let site = self.add(address, offset)?;
        let pointer = u32_at(&self.bytes(site, 4)?, 0)?;
        Ok((pointer != 0).then_some(pointer))
    }
}

fn live_f32<M: ReadMemory>(
    source: &mut MemoryView<'_, M>,
    base: u32,
    offset: usize,
) -> Result<f32> {
    let address = source.add(base, offset)?;
    let value = f32::from_bits(u32_at(&source.bytes(address, 4)?, 0)?);
    ensure!(value.is_finite(), "non-finite live dynamics value");
    Ok(value)
}

/// The verified camera block stores a world-to-camera basis and a translation
/// vector after its two-byte pose flags. Applying the basis to the negated
/// translation yields the camera origin in OpenSpace axes; its third row is
/// the view-forward direction. The remaining nine pose floats duplicate the
/// basis in the source's 22-word transform representation.
fn decode_camera_pose(pose_words: &[u32; 22]) -> Result<([f32; 3], [f32; 3])> {
    let translation = [
        f32::from_bits(pose_words[1]),
        f32::from_bits(pose_words[2]),
        f32::from_bits(pose_words[3]),
    ];
    let mut basis = [[0.0f32; 3]; 3];
    for (index, value) in pose_words[4..13].iter().copied().enumerate() {
        basis[index / 3][index % 3] = f32::from_bits(value);
    }
    ensure!(
        translation.into_iter().all(f32::is_finite)
            && basis.iter().flatten().copied().all(f32::is_finite),
        "camera pose contains non-finite values"
    );
    let mut position = [0.0f32; 3];
    for row in 0..3 {
        position[row] = -(basis[row][0] * translation[0]
            + basis[row][1] * translation[1]
            + basis[row][2] * translation[2]);
    }
    let mut forward = basis[2];
    let norm = (forward[0] * forward[0] + forward[1] * forward[1] + forward[2] * forward[2]).sqrt();
    ensure!(
        norm.is_finite() && norm > 0.5,
        "camera forward basis is degenerate"
    );
    for component in &mut forward {
        *component /= norm;
    }
    Ok((position, forward))
}

/// Constructed only after a full executable-file hash matches. This guard does
/// not by itself prove the live module mapping; the controller must check that.
pub struct HypeBuild(());
impl HypeBuild {
    pub fn verified(executable_sha256: &str) -> Result<Self> {
        ensure!(
            executable_sha256 == HYPE_EXECUTABLE_SHA256,
            "unrecognized Hype executable build"
        );
        Ok(Self(()))
    }
    pub fn status(&self, memory: &mut impl ReadMemory) -> Result<Status> {
        let mut source = MemoryView(memory);
        let bytes = source.bytes(super::hype_runtime::STATUS, 261)?;
        let end = bytes[1..]
            .iter()
            .position(|b| *b == 0)
            .context("unterminated current level")?;
        ensure!(bytes[1..1 + end].is_ascii(), "non-ASCII current level");
        Ok(Status {
            phase: bytes[0],
            level: std::str::from_utf8(&bytes[1..1 + end])?.to_owned(),
        })
    }

    /// Read the active viewport camera selected by the verified build. The
    /// 22-word pose block is retained as capture provenance alongside the
    /// decoded position and forward direction in OpenSpace source axes.
    pub fn active_camera(&self, memory: &mut impl ReadMemory) -> Result<ActiveCameraObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9,
            "active camera requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let pointer = source.required(super::hype_runtime::ACTIVE_CAMERA_POINTER, 0)?;
        ensure!(pointer != 0, "active viewport has no camera");
        let identity = source.bytes(pointer, 12)?;
        let kind = u32_at(&identity, 8)?;
        ensure!(
            kind == 103,
            "active viewport camera has unexpected type {kind}"
        );
        let block = source.bytes(source.add(pointer, 0x0c)?, 0x58)?;
        let mut pose_words = [0u32; 22];
        for (index, word) in pose_words.iter_mut().enumerate() {
            *word = u32_at(&block, index * 4)?;
        }
        let (position_source, forward_source) = decode_camera_pose(&pose_words)?;
        let horizontal_fov = live_f32(&mut source, pointer, 0x64)?;
        let vertical_fov = live_f32(&mut source, pointer, 0x68)?;
        let near_plane = live_f32(&mut source, pointer, 0x6c)?;
        let far_plane = live_f32(&mut source, pointer, 0x70)?;
        let source_update_counter =
            u32_at(&source.bytes(super::hype_runtime::UPDATE_COUNTER, 4)?, 0)?;
        let source_clock_ms = u32_at(&source.bytes(super::hype_runtime::CLOCK_MS, 4)?, 0)?;
        ensure!(
            horizontal_fov > 0.0 && horizontal_fov < 3.2,
            "invalid horizontal camera FoV"
        );
        ensure!(
            vertical_fov > 0.0 && vertical_fov < 3.2,
            "invalid vertical camera FoV"
        );
        ensure!(
            near_plane > 0.0 && far_plane > near_plane,
            "invalid camera clipping planes"
        );
        Ok(ActiveCameraObservation {
            source_locator: pointer,
            pose_words,
            position_source,
            forward_source,
            horizontal_fov,
            vertical_fov,
            near_plane,
            far_plane,
            source_update_counter,
            source_clock_ms,
        })
    }
    /// Resolve active inventory handles, their item identities, and their
    /// transient quantity locations through the checked source list.
    fn inventory_members(
        &self,
        memory: &mut impl ReadMemory,
    ) -> Result<(usize, Vec<(InventoryItemObservation, u32)>)> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9,
            "inventory requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let mut entries = Vec::new();
        let mut visited = BTreeSet::new();
        let mut next = if let Some(root) = source.pointer(0x6107b0, 0)? {
            source.pointer(root, 8)?
        } else {
            None
        };
        while let Some(entry) = next {
            ensure!(visited.len() < 256, "inventory list exceeds 256 entries");
            ensure!(visited.insert(entry), "inventory list cycle");
            let handle = source.required(entry, 4)?;
            let handle_bytes = source.bytes(handle, 12)?;
            if handle_bytes[8] != 0 {
                let member = u32_at(&handle_bytes, 0)?;
                let quantity = i16::from_le_bytes(handle_bytes[10..12].try_into()?);
                drop(handle_bytes);
                ensure!(member != 0, "active inventory handle has no member");
                let (name_pointer, id) = {
                    let record = source.bytes(member, 30)?;
                    (
                        u32_at(&record, 12)?,
                        i16::from_le_bytes(record[28..30].try_into()?),
                    )
                };
                // Hype's functions 208/209 pass the inventory handle to the
                // quantity getter/setter (0x4a3ba0/0x4a3b90), which read/write
                // handle+0x0a. The referenced item record has a separate
                // +0x0a field and is not the live stack quantity.
                ensure!(name_pointer != 0, "active inventory member has no kind");
                let mut name = Vec::new();
                for offset in 0..64 {
                    let address = source.add(name_pointer, offset)?;
                    let byte = source.bytes(address, 1)?[0];
                    if byte == 0 {
                        break;
                    }
                    ensure!(byte.is_ascii(), "inventory kind is not ASCII");
                    name.push(byte);
                }
                ensure!(name.len() < 64, "unterminated inventory kind");
                entries.push((
                    InventoryItemObservation {
                        kind: String::from_utf8(name)?,
                        id,
                        quantity,
                    },
                    source.add(handle, 10)?,
                ));
            }
            next = source.pointer(entry, 20)?;
        }
        Ok((visited.len(), entries))
    }
    /// Read the active inventory list used by functions 201, 208 and 209 in
    /// this verified build. The list is capture-local; no source address is
    /// persisted as gameplay state.
    pub fn inventory(&self, memory: &mut impl ReadMemory) -> Result<InventoryObservation> {
        let (total_entries, members) = self.inventory_members(memory)?;
        let entries = members
            .into_iter()
            .map(|(item, _)| item)
            .collect::<Vec<_>>();
        let selected_weapon = entries
            .iter()
            .find(|item| item.kind == "Weapon")
            .map_or(-1, |item| item.id);
        Ok(InventoryObservation {
            total_entries,
            active: entries,
            selected_weapon,
        })
    }
    /// Return only a capture-local quantity address for a unique active item
    /// whose prior value is exactly the declared starting-state value.
    pub fn inventory_quantity_address(
        &self,
        memory: &mut impl ReadMemory,
        kind: &str,
        id: i16,
        expected_quantity: i16,
    ) -> Result<u32> {
        let (_, members) = self.inventory_members(memory)?;
        let mut matching = members
            .into_iter()
            .filter(|(item, _)| item.kind == kind && item.id == id);
        let (item, address) = matching
            .next()
            .context("active inventory item is missing")?;
        ensure!(
            matching.next().is_none(),
            "active inventory item is ambiguous"
        );
        ensure!(
            item.quantity == expected_quantity,
            "active inventory item has a different quantity"
        );
        Ok(address)
    }

    /// Resolve a live source pointer against one loaded Fix module/block. The
    /// module table and its entries are runtime state for this verified
    /// executable; only the block-relative offset may be compared with a
    /// source allocation.
    fn fix_block_offset(
        &self,
        source: &mut MemoryView<'_, impl ReadMemory>,
        module: u16,
        block: u16,
        address: u32,
    ) -> Result<Option<LiveBlockOffset>> {
        let Some((base, end)) = self.fix_block_bounds(source, module, block)? else {
            return Ok(None);
        };
        if address < base || address > end {
            return Ok(None);
        }
        Ok(Some(LiveBlockOffset {
            module,
            block,
            payload_offset: address - base,
            active_base_locator: format!("live:{base:08x}"),
            active_end_locator: format!("live:{end:08x}"),
        }))
    }

    fn fix_block_bounds(
        &self,
        source: &mut MemoryView<'_, impl ReadMemory>,
        module: u16,
        block: u16,
    ) -> Result<Option<(u32, u32)>> {
        // Mmg's module pointer array begins at this build-verified address;
        // module 17's slot is 0x00880f64 and module 5's is 0x00880f34.
        let module_slot = source.add(0x0088_0f20, usize::from(module) * 4)?;
        let Some(table) = source.pointer(module_slot, 0)? else {
            return Ok(None);
        };
        let block_record = source.add(table, usize::from(block) * 16)?;
        let record = source.bytes(block_record, 16)?;
        let base = u32_at(&record, 0)?;
        let end = u32_at(&record, 4)?;
        ensure!(
            base != 0 && end >= base && end - base < 64 * 1024 * 1024,
            "active Fix block has invalid bounds"
        );
        Ok(Some((base, end)))
    }

    /// Resolve one live AI model pointer against the loaded Fix AI block.
    fn fix_ai_block_offset(
        &self,
        source: &mut MemoryView<'_, impl ReadMemory>,
        model: u32,
    ) -> Result<Option<LiveBlockOffset>> {
        self.fix_block_offset(source, 17, 0, model)
    }
    /// Source function 232 selects one of three runtime magic-name rows, then
    /// looks up the selected inventory item named `Magic` plus that row and
    /// returns its signed info field.
    /// Preserve an unmatched row as None instead of inventing a spell code.
    pub fn magic_selection(
        &self,
        memory: &mut impl ReadMemory,
        inventory: &InventoryObservation,
    ) -> Result<MagicSelectionObservation> {
        ensure!(
            self.status(memory)?.phase == 9,
            "magic selection requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let table = source.required(0x610b98, 0)?;
        let values = source.bytes(source.add(table, 0x24)?, 5)?;
        let current_points = u16::from_le_bytes(values[0..2].try_into()?);
        let maximum_points = u16::from_le_bytes(values[2..4].try_into()?);
        let selected_index = values[4];
        ensure!(
            selected_index <= 2,
            "magic selection outside three name rows"
        );
        let row = source.bytes(source.add(table, usize::from(selected_index) * 10)?, 10)?;
        let end = row
            .iter()
            .position(|value| *value == 0)
            .context("unterminated magic name")?;
        ensure!(end > 0 && row[..end].is_ascii(), "invalid magic name");
        let selected_name = String::from_utf8(row[..end].to_vec())?;
        let inventory_kind = format!("Magic{selected_name}");
        let source_code = inventory
            .active
            .iter()
            .find(|item| item.kind == inventory_kind)
            .map(|item| item.id);
        Ok(MagicSelectionObservation {
            current_points,
            maximum_points,
            selected_index,
            selected_name,
            source_code,
        })
    }
    /// The verified build keeps the laser target point and presentation flag
    /// in one small global record. The target remains readable while hidden.
    pub fn laser_pointer(&self, memory: &mut impl ReadMemory) -> Result<LaserPointerObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9,
            "laser read requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let bytes = source.bytes(0x74b310, 22)?;
        let mut point = [0.0; 3];
        for (i, value) in point.iter_mut().enumerate() {
            *value = f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into()?);
            ensure!(value.is_finite(), "nonfinite laser target component");
        }
        let active = u16::from_le_bytes(bytes[20..22].try_into()?);
        ensure!(active <= 1, "laser presentation flag is not Boolean");
        Ok(LaserPointerObservation {
            point,
            active: active != 0,
        })
    }
    pub fn actor(&self, memory: &mut impl ReadMemory) -> Result<ActorObservation> {
        self.actor_with_actions(memory, &[47])
    }
    /// Inspect only the explicitly requested invariant action definitions. The
    /// family list is still traversed to preserve ordering and resolve links.
    pub fn actor_with_actions(
        &self,
        memory: &mut impl ReadMemory,
        actions: &[u32],
    ) -> Result<ActorObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && !status.level.is_empty(),
            "actor inspection requires a loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        // FUN_00443470 returns this global; FUN_00443460 reads its entry at +4.
        // File counterpart is GPT+0x2ae0. Do not infer other globals from a base.
        let entry = source.required(0x756e70, 0)?;
        actor(&mut source, entry, actions, |a| format!("live:{a:08x}"))
    }

    /// Inspect the selected actor plus its live behavior selections and only
    /// the requested designer values. All roots are resolved from the verified
    /// build; callers never supply process addresses. Returned labels are
    /// capture-local evidence and must not be persisted as asset identities.
    pub fn actor_with_ai(
        &self,
        memory: &mut impl ReadMemory,
        actions: &[u32],
        designer_values: &[u32],
    ) -> Result<ActorAiObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && !status.level.is_empty(),
            "actor inspection requires a loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let entry = source.required(0x756e70, 0)?;
        let node = source.required(entry, 4)?;
        actor_ai_at_node(&mut source, node, actions, designer_values)
    }

    /// Resolve a rooted actor by its source scene path within this capture.
    /// Paths and source type IDs are checked afresh; no live pointer is retained.
    pub fn scene_actor_with_ai(
        &self,
        memory: &mut impl ReadMemory,
        level: &str,
        child_path: &[u32],
        expected_object_types: [u32; 3],
        actions: &[u32],
        designer_values: &[u32],
    ) -> Result<ActorAiObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == level,
            "scene actor inspection requires the selected loaded level"
        );
        let mut source = MemoryView(memory);
        let node = scene_node(&mut source, child_path)?;
        let observation = actor_ai_at_node(&mut source, node, actions, designer_values)?;
        ensure!(
            observation.actor.object_types == expected_object_types,
            "scene actor source object types differ: expected {:?}, observed {:?}",
            expected_object_types,
            observation.actor.object_types
        );
        Ok(observation)
    }

    /// Capture the live Global control used by Hype's crossbow firing branch.
    /// Its scene path, Fix Perso allocation, allocation extent, linked model
    /// and typed designer field must all match the source-backed identity.
    pub fn crossbow_global(
        &self,
        memory: &mut impl ReadMemory,
    ) -> Result<CrossbowGlobalObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == "manoir",
            "crossbow Global observation requires loaded manor gameplay"
        );
        let mut source = MemoryView(memory);
        let (module_5_base, module_5_end) = self
            .fix_block_bounds(&mut source, 5, 0)?
            .context("active Fix module 5/block 0 is unavailable")?;
        let perso = module_5_base
            .checked_add(0x314)
            .context("manor Global Perso address overflow")?;
        let perso_fix_block_offset = self
            .fix_block_offset(&mut source, 5, 0, perso)?
            .context("manor Global Perso is outside active Fix module 5/block 0")?;
        ensure!(
            perso_fix_block_offset.payload_offset == 0x314,
            "manor Global Perso moved from Fix module 5/block 0 allocation 15"
        );
        ensure!(
            module_5_end >= perso && module_5_end - perso >= 79,
            "manor Global Perso allocation exceeds active Fix module 5/block 0"
        );
        let allocation_header_address = perso
            .checked_sub(4)
            .context("manor Global allocation header address underflow")?;
        let allocation_header_words = u32_at(&source.bytes(allocation_header_address, 4)?, 0)?;
        ensure!(
            allocation_header_words == 20,
            "manor Global allocation header differs from the expected 20-word Perso"
        );
        let found = find_live_actor_by_perso(&mut source, perso)?;
        let observation = actor_ai_at_node(&mut source, found.node, &[], &[14])?;
        ensure!(
            observation.actor.object_types == [0, 0, 0],
            "live Fix Global object types differ from source identity: {:?}",
            observation.actor.object_types
        );
        let ai = source_view::ai_parts(&mut source, perso)?;
        let ai_model_fix_block_offset = self
            .fix_ai_block_offset(&mut source, ai.model)?
            .context("manor Global AI model is outside active Fix module 17/block 0")?;
        ensure!(
            ai_model_fix_block_offset.payload_offset == 0x1c,
            "manor Global AI model differs from Fix module 17/block 0 allocation 1"
        );
        let field_14 = observation
            .designer_values
            .iter()
            .find(|value| value.index == 14)
            .context("manor Global AI field 14 was not captured")?;
        ensure!(
            field_14.kind == 0
                && matches!(
                    field_14.value,
                    ObservedDesignerValue::Value(DesignerValue::Boolean(_))
                ),
            "manor Global AI field 14 is not a typed Boolean"
        );
        Ok(CrossbowGlobalObservation {
            actor: observation,
            scene_path: found.path,
            scanned_nodes: found.scanned_nodes,
            perso_fix_block_offset,
            ai_model_fix_block_offset,
            allocation_header_words,
        })
    }

    /// Resolve a unique live actor by the source object-type triple when
    /// startup has changed its child ordinal. The checked live path is returned
    /// as evidence; its address is never reused outside this capture.
    pub fn scene_actor_by_types_with_ai(
        &self,
        memory: &mut impl ReadMemory,
        level: &str,
        expected_object_types: [u32; 3],
        actions: &[u32],
        designer_values: &[u32],
    ) -> Result<(Vec<u32>, ActorAiObservation)> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == level,
            "scene actor search requires the selected loaded level"
        );
        let mut source = MemoryView(memory);
        let found = find_live_actor(&mut source, expected_object_types, false)?;
        let observation = actor_ai_at_node(&mut source, found.node, actions, designer_values)?;
        ensure!(
            observation.actor.object_types == expected_object_types,
            "resolved live actor source identity changed during capture"
        );
        Ok((found.path, observation))
    }

    /// Find every live scene actor whose AI model resolves to one active Fix
    /// allocation. Unlike `scene_tail`, this follows the whole ActualWorld
    /// hierarchy, so reparenting cannot look like actor destruction.
    pub fn scene_ai_model_instances(
        &self,
        memory: &mut impl ReadMemory,
        level: &str,
        module: u16,
        block: u16,
        payload_offset: u32,
        designer_values: &[u32],
    ) -> Result<SceneAiModelSearchObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == level,
            "AI model search requires the selected loaded level"
        );
        let mut source = MemoryView(memory);
        let (base, end) = self
            .fix_block_bounds(&mut source, module, block)?
            .context("requested AI model Fix block is not active")?;
        ensure!(
            payload_offset <= end - base,
            "requested AI model offset is outside its active Fix block"
        );
        let model_address = base
            .checked_add(payload_offset)
            .context("AI model source address overflow")?;
        let model = self
            .fix_block_offset(&mut source, module, block, model_address)?
            .context("requested AI model identity did not resolve in the active Fix block")?;
        let (matches, scanned_nodes) = find_live_actors_by_ai_model(&mut source, model_address)?;
        let mut instances = Vec::with_capacity(matches.len());
        for found in matches {
            let observed = actor_ai_at_node(&mut source, found.node, &[], designer_values)?;
            let perso = source.required(found.node, SuperObjectWire::DATA)?;
            let ai = source_view::ai_parts(&mut source, perso)?;
            ensure!(
                ai.model == model_address,
                "scene AI model identity changed during the stopped capture"
            );
            let actor = observed.actor;
            ensure!(
                actor.perso == format!("live:{perso:08x}"),
                "scene AI match Perso changed during the stopped capture"
            );
            instances.push(SceneAiModelInstance {
                scene_path: found.path,
                actor,
                normal: observed.normal,
                reflex: observed.reflex,
                designer_values: observed.designer_values,
            });
        }
        Ok(SceneAiModelSearchObservation {
            model,
            scanned_nodes,
            instances,
        })
    }

    /// Capture source actors near a caller-observed world position while
    /// validating the complete ActualWorld hierarchy in this stopped view.
    pub fn scene_actors_near_position(
        &self,
        memory: &mut impl ReadMemory,
        level: &str,
        center: [f32; 3],
        radius: f32,
    ) -> Result<SceneNearbyActorsObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == level,
            "nearby actor search requires the selected loaded level"
        );
        ensure!(
            center.into_iter().all(f32::is_finite)
                && radius.is_finite()
                && (0.0..=20.0).contains(&radius),
            "nearby actor search bounds are invalid"
        );
        const MAX_NODES: usize = 4096;
        const MAX_DEPTH: usize = 32;
        let mut source = MemoryView(memory);
        let root = source.required(0x75ec6c, 0)?;
        let mut stack = vec![(root, Vec::<u32>::new())];
        let mut visited = BTreeSet::from([root]);
        let mut actors = Vec::new();
        while let Some((node, path)) = stack.pop() {
            let header: SuperObjectWire = wire(&source.bytes(node, 64)?)?;
            if path.is_empty() {
                ensure!(header.kind == 0, "ActualWorld root is not a world node");
            }
            if header.kind == 4 {
                let perso = source.required(node, SuperObjectWire::DATA)?;
                let object_types = source_view::perso_parts(&mut source, perso)?.object_types;
                let transform = source.required(node, 36)?;
                let global_transform = source_view::transform(&mut source, transform)?;
                let translation = global_transform.affine().translation;
                let distance = translation
                    .iter()
                    .zip(center)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f32>()
                    .sqrt();
                if distance <= radius {
                    actors.push(SceneNearbyActor {
                        scene_path: path.clone(),
                        object_types,
                        distance,
                        global_transform,
                    });
                }
            }
            let count = usize::try_from(header.child_count)?;
            ensure!(
                count <= MAX_NODES,
                "live scene child count exceeds search bound"
            );
            if count == 0 {
                ensure!(
                    header.first_child == 0 && header.last_child == 0,
                    "empty live scene node has child endpoints"
                );
                continue;
            }
            ensure!(path.len() < MAX_DEPTH, "live scene search depth exceeded");
            ensure!(
                visited.len().saturating_add(count) <= MAX_NODES,
                "live scene search node budget exceeded"
            );
            ensure!(
                header.first_child != 0 && header.last_child != 0,
                "live scene node lacks child endpoints"
            );
            let mut child = header.first_child;
            let mut previous = 0u32;
            let mut children = Vec::with_capacity(count);
            for index in 0..count {
                ensure!(child != 0, "live scene sibling list ended early");
                ensure!(visited.insert(child), "live scene cycle or repeated child");
                let member: SuperObjectWire = wire(&source.bytes(child, 64)?)?;
                ensure!(
                    member.parent == node,
                    "live scene child parent link differs"
                );
                ensure!(
                    member.previous == previous,
                    "live scene previous sibling link differs"
                );
                if index + 1 == count {
                    ensure!(
                        child == header.last_child && member.next == 0,
                        "live scene last child endpoint differs"
                    );
                } else {
                    ensure!(member.next != 0, "live scene sibling list ended early");
                }
                let mut child_path = path.clone();
                child_path.push(u32::try_from(index)?);
                children.push((child, child_path));
                previous = child;
                child = member.next;
            }
            stack.extend(children.into_iter().rev());
        }
        actors.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        Ok(SceneNearbyActorsObservation {
            center,
            radius,
            scanned_nodes: visited.len(),
            actors,
        })
    }

    /// Capture the four camera parameter records linked from a selected
    /// source Perso. These are capture-local source words for research, not
    /// reusable model fields or persisted virtual addresses.
    pub fn scene_camera_records(
        &self,
        memory: &mut impl ReadMemory,
        level: &str,
        child_path: &[u32],
        expected_object_types: [u32; 3],
    ) -> Result<SourceCameraRecordsObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == level,
            "source camera inspection requires the selected loaded level"
        );
        let mut source = MemoryView(memory);
        let node = scene_node(&mut source, child_path)?;
        let perso = source.required(node, 4)?;
        let parts = source_view::perso_parts(&mut source, perso)?;
        ensure!(
            parts.object_types == expected_object_types,
            "source camera actor object types differ: expected {:?}, observed {:?}",
            expected_object_types,
            parts.object_types
        );
        let camera = source.required(perso, 20)?;
        camera_records_at(&mut source, camera)
    }

    /// Find a unique camera-bearing Perso in the live ActualWorld hierarchy.
    /// Saved child ordinals may change after loading a game, so the match is a
    /// capture-local checked predicate rather than a persisted Perso identity.
    pub fn scene_camera(
        &self,
        memory: &mut impl ReadMemory,
        level: &str,
        expected_object_types: [u32; 3],
        designer_values: &[u32],
    ) -> Result<SceneCameraObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == level,
            "source camera search requires the selected loaded level"
        );
        let (found, actor, records) = {
            let mut source = MemoryView(memory);
            let found = find_live_camera(&mut source, expected_object_types)?;
            let actor = actor_ai_at_node(&mut source, found.node, &[], designer_values)?;
            let camera = source.required(found.perso, 20)?;
            let records = camera_records_at(&mut source, camera)?;
            (found, actor, records)
        };
        let active_camera = self.active_camera(memory)?;
        Ok(SceneCameraObservation {
            live_child_path: found.path,
            scanned_nodes: found.scanned_nodes,
            actor,
            source_camera_records: records,
            active_camera,
        })
    }

    /// Identify the newest DynamicWorld children, where source object creation
    /// links generated Persos. Ordinals are capture-local, not asset IDs.
    pub fn scene_tail(
        &self,
        memory: &mut impl ReadMemory,
        limit: usize,
    ) -> Result<SceneTailObservation> {
        self.scene_tail_with_ai_designer_values(memory, limit, &[])
    }

    /// Bounded tail capture with selected model-733 Brain fields. Other actors
    /// retain the ordinary lightweight AI identity/state observation.
    pub fn scene_tail_with_ai_designer_values(
        &self,
        memory: &mut impl ReadMemory,
        limit: usize,
        designer_values: &[u32],
    ) -> Result<SceneTailObservation> {
        ensure!((1..=16).contains(&limit), "scene tail limit outside 1..16");
        ensure!(
            designer_values.len() <= 64,
            "too many selected tail AI fields"
        );
        ensure!(
            designer_values
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                == designer_values.len(),
            "duplicate selected tail AI field"
        );
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9,
            "scene tail requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let root_address = source.required(0x75701c, 0)?;
        let root: SuperObjectWire = wire(&source.bytes(root_address, 64)?)?;
        ensure!(root.kind == 0, "DynamicWorld root is not a world node");
        ensure!(
            root.child_count <= 100_000,
            "scene child count exceeds bound"
        );
        let count = usize::try_from(root.child_count)?;
        if count == 0 {
            ensure!(
                root.first_child == 0 && root.last_child == 0,
                "empty scene root has child endpoints"
            );
        } else {
            ensure!(
                root.first_child != 0 && root.last_child != 0,
                "scene root lacks child endpoints"
            );
        }
        let mut address = root.last_child;
        let mut following = 0;
        let mut children = Vec::new();
        for index in (count.saturating_sub(limit)..count).rev() {
            ensure!(address != 0, "scene tail ended before child count");
            let child: SuperObjectWire = wire(&source.bytes(address, 64)?)?;
            ensure!(
                child.parent == root_address,
                "scene tail parent link differs"
            );
            ensure!(child.next == following, "scene tail next link differs");
            let (object_types, ai, perso_locator, update_stamp, dynamics) = if child.kind == 4 {
                let perso = source.required(address, SuperObjectWire::DATA)?;
                let object_types = source_view::perso_parts(&mut source, perso)?.object_types;
                let standard = source.required(perso, 4)?;
                let stamp_address = source.add(standard, 0x10)?;
                let update_stamp = u32_at(&source.bytes(stamp_address, 4)?, 0)?;
                let dynamics = match (|| {
                    let Some(wrapper) = source.pointer(perso, 8)? else {
                        return Ok::<_, anyhow::Error>(SceneTailDynamics::Absent);
                    };
                    let Some(object) = source.pointer(wrapper, 16)? else {
                        return Ok(SceneTailDynamics::Absent);
                    };
                    // The verified Hype dynamics integrator reads this byte at
                    // object +0x348, then clears it after its override step.
                    let flag_address = source.add(object, 0x348)?;
                    let override_flag = source.bytes(flag_address, 1)?[0];
                    let cached_card = source.pointer(wrapper, 0x14)?;
                    let cached_type_address = source.add(wrapper, 0x18)?;
                    let cached_card_type = u32_at(&source.bytes(cached_type_address, 4)?, 0)?;
                    let candidate_card_mass = if cached_card_type == 0 {
                        cached_card
                            .map(|card| live_f32(&mut source, card, 4))
                            .transpose()?
                    } else {
                        None
                    };
                    let candidate_card_ratio = if cached_card_type == 0 {
                        cached_card
                            .map(|card| live_f32(&mut source, card, 0x58))
                            .transpose()?
                    } else {
                        None
                    };
                    let cached_mass = live_f32(&mut source, object, 0xc0)?;
                    let integrated_velocity = [
                        live_f32(&mut source, object, 0x64)?,
                        live_f32(&mut source, object, 0x68)?,
                        live_f32(&mut source, object, 0x6c)?,
                    ];
                    let pending_override_velocity = [
                        live_f32(&mut source, object, 0x34c)?,
                        live_f32(&mut source, object, 0x350)?,
                        live_f32(&mut source, object, 0x354)?,
                    ];
                    let mode_e0_address = source.add(object, 0xe0)?;
                    let mode_e0 = u32_at(&source.bytes(mode_e0_address, 4)?, 0)?;
                    let contact_address = source.add(object, 0xf0)?;
                    let contact_words = source.bytes(contact_address, 12)?;
                    let previous = u32_at(&contact_words, 0)?;
                    let current = u32_at(&contact_words, 4)?;
                    let ai_visible = u32_at(&contact_words, 8)?;
                    let contact_classes = SceneTailContactClasses {
                        previous,
                        current,
                        ai_visible,
                        collide_with_nothing: ai_visible == 0,
                        publication_consistent: ai_visible == (previous | current),
                    };
                    let list = source.pointer(wrapper, 0xfc)?;
                    let mut card_slots = Vec::new();
                    if let Some(list_address) = list {
                        let count = usize::from(source.bytes(list_address, 1)?[0]);
                        ensure!(
                            count <= 8,
                            "dynamics card-selection list exceeds eight slots"
                        );
                        for index in 0..count {
                            let key_address = source.add(list_address, 4 + 4 * index)?;
                            let key = u32_at(&source.bytes(key_address, 4)?, 0)?;
                            let card = source.pointer(list_address, 0x24 + 4 * index)?;
                            let card_type = if let Some(card_address) = card {
                                Some(u32_at(&source.bytes(card_address, 4)?, 0)?)
                            } else {
                                None
                            };
                            card_slots.push(SceneTailCardSlot {
                                index: u8::try_from(index)?,
                                key,
                                card_locator: card.map(|address| format!("live:{address:08x}")),
                                card_type,
                            });
                        }
                    }
                    // The original selection walk passes its last nonnull
                    // candidate to the mechanics callback dispatcher.
                    let selected = card_slots
                        .iter()
                        .rev()
                        .find(|slot| slot.card_locator.is_some());
                    let selected_card_locator = selected.and_then(|slot| slot.card_locator.clone());
                    let selected_card_type = selected.and_then(|slot| slot.card_type);
                    Ok(SceneTailDynamics::Ready {
                        wrapper_locator: format!("live:{wrapper:08x}"),
                        object_locator: format!("live:{object:08x}"),
                        override_flag,
                        cached_card_locator: cached_card
                            .map(|address| format!("live:{address:08x}")),
                        cached_card_type,
                        candidate_card_mass,
                        candidate_card_ratio,
                        cached_mass,
                        integrated_velocity,
                        pending_override_velocity,
                        mode_e0,
                        contact_classes,
                        list_locator: list.map(|address| format!("live:{address:08x}")),
                        card_slots,
                        selected_card_locator,
                        selected_card_type,
                    })
                })() {
                    Ok(dynamics) => dynamics,
                    Err(error) => SceneTailDynamics::Unreadable {
                        reason: error.to_string(),
                    },
                };
                let ai = if source.pointer(perso, 16)?.is_none() {
                    SceneTailAi::BrainAbsent
                } else {
                    match (|| {
                        let parts = source_view::ai_parts(&mut source, perso)?;
                        let normal_set = source_view::behavior_set(&mut source, parts.model, 0)?;
                        let reflex_set = source_view::behavior_set(&mut source, parts.model, 4)?;
                        let normal = source_view::behavior_selection(
                            &mut source,
                            parts.mind,
                            4,
                            normal_set.as_ref(),
                        )?;
                        let reflex = source_view::behavior_selection(
                            &mut source,
                            parts.mind,
                            8,
                            reflex_set.as_ref(),
                        )?;
                        let designer_field_count =
                            source_view::designer_view(&mut source, parts.model, parts.mind)?
                                .map_or(0, |view| view.fields.len());
                        let fix_block_offset =
                            self.fix_ai_block_offset(&mut source, parts.model)?;
                        let designer_values = if fix_block_offset.as_ref().is_some_and(|offset| {
                            offset.module == 17
                                && offset.block == 0
                                && offset.payload_offset == 0x4e324
                        }) {
                            actor_ai_at_node(&mut source, address, &[], designer_values)?
                                .designer_values
                        } else {
                            Vec::new()
                        };
                        Ok::<_, anyhow::Error>(SceneTailAi::Ready {
                            model_locator: format!("live:{:08x}", parts.model),
                            fix_block_offset,
                            normal_count: normal_set.as_ref().map_or(0, |set| set.members.len()),
                            reflex_count: reflex_set.as_ref().map_or(0, |set| set.members.len()),
                            designer_field_count,
                            designer_values,
                            normal,
                            reflex,
                        })
                    })() {
                        Ok(ai) => ai,
                        Err(error) => SceneTailAi::Unreadable {
                            reason: error.to_string(),
                        },
                    }
                };
                (
                    Some(object_types),
                    Some(ai),
                    Some(format!("live:{perso:08x}")),
                    Some(update_stamp),
                    Some(dynamics),
                )
            } else {
                (None, None, None, None, None)
            };
            let global_transform = if child.kind == 4 {
                let transform = source.required(address, 36)?;
                Some(source_view::transform(&mut source, transform)?)
            } else {
                None
            };
            children.push(SceneTailChild {
                index: u32::try_from(index)?,
                kind: child.kind,
                node_locator: format!("live:{address:08x}"),
                perso_locator,
                update_stamp,
                dynamics,
                object_types,
                global_transform,
                ai,
            });
            following = address;
            address = child.previous;
        }
        if count <= limit {
            ensure!(
                address == 0 && following == root.first_child,
                "scene root first child link differs"
            );
        }
        children.reverse();
        Ok(SceneTailObservation {
            child_count: root.child_count,
            children,
        })
    }

    /// Capture only the newest DynamicWorld child. The bounded tail traversal
    /// still validates sibling links, but avoids parsing unrelated children.
    pub fn scene_tail_latest(&self, memory: &mut impl ReadMemory) -> Result<SceneTailObservation> {
        self.scene_tail(memory, 1)
    }

    pub fn scene_tail_latest_with_ai_designer_values(
        &self,
        memory: &mut impl ReadMemory,
        designer_values: &[u32],
    ) -> Result<SceneTailObservation> {
        self.scene_tail_with_ai_designer_values(memory, 1, designer_values)
    }
}

fn actor_ai_at_node(
    source: &mut MemoryView<'_, impl ReadMemory>,
    node: u32,
    actions: &[u32],
    designer_values: &[u32],
) -> Result<ActorAiObservation> {
    ensure!(
        designer_values
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            == designer_values.len(),
        "duplicate selected designer value"
    );
    let actor = actor_at_node(source, node, actions, |a| format!("live:{a:08x}"))?;
    let perso = source.required(node, 4)?;
    let parts = source_view::perso_parts(source, perso)?;
    let action_members = source_view::list(source, parts.family, 16)?;
    let ai = source_view::ai_parts(source, perso)?;
    let normal_set = source_view::behavior_set(source, ai.model, 0)?;
    let reflex_set = source_view::behavior_set(source, ai.model, 4)?;
    let normal = source_view::behavior_selection(source, ai.mind, 4, normal_set.as_ref())?;
    let reflex = source_view::behavior_selection(source, ai.mind, 8, reflex_set.as_ref())?;
    let designer = source_view::designer_view(source, ai.model, ai.mind)?;
    ensure!(
        designer.is_some() || designer_values.is_empty(),
        "actor AI has no designer values"
    );
    let mut selected_designer_values = Vec::with_capacity(designer_values.len());
    if let Some(designer) = designer {
        let family = AssetRef::new("level:/families/__live_observation__")?;
        let actor_ref = AssetRef::new("level:/persos/__live_observation__")?;
        for &index in designer_values {
            let info = *designer
                .fields
                .get(index as usize)
                .context("selected designer value outside definition")?;
            let selected_actor = std::cell::Cell::new(None);
            let (value, _) = source_view::designer_value(
                source,
                designer.values,
                info,
                designer.length,
                |target| {
                    target
                        .map(|target| {
                            action_members
                                .iter()
                                .position(|member| *member == target)
                                .map(|state| StateRef {
                                    family: family.clone(),
                                    state: state as u32,
                                })
                                .context("designer action outside selected actor family")
                        })
                        .transpose()
                },
                |target| {
                    selected_actor.set(target);
                    Ok(target.map(|_| actor_ref.clone()))
                },
            )?;
            let value = match value {
                DesignerValue::Action(value) => {
                    ObservedDesignerValue::Action(value.map(|value| value.state))
                }
                DesignerValue::ActionArray(value) => {
                    ObservedDesignerValue::ActionArray(DesignerArray {
                        values: value
                            .values
                            .into_iter()
                            .map(|value| value.map(|value| value.state))
                            .collect(),
                        unknown: value.unknown,
                        element_unknown: value.element_unknown,
                    })
                }
                DesignerValue::Actor(value) => {
                    let target = selected_actor.get();
                    ensure!(
                        value.is_some() == target.is_some(),
                        "designer actor binding differs"
                    );
                    if let Some(target) = target {
                        let _: PersoHeaderWire = wire(&source.bytes(target, 20)?)?;
                        source_view::perso_parts(source, target)
                            .context("designer actor is not a readable perso")?;
                    }
                    ObservedDesignerValue::Actor(target.map(|target| format!("live:{target:08x}")))
                }
                value => ObservedDesignerValue::Value(value),
            };
            selected_designer_values.push(SelectedDesignerValue {
                index,
                kind: info.kind,
                value,
            });
        }
    }
    Ok(ActorAiObservation {
        actor,
        normal,
        reflex,
        designer_values: selected_designer_values,
    })
}

#[cfg(test)]
mod active_camera_tests {
    use super::*;
    use std::collections::BTreeMap;

    struct Memory(BTreeMap<u32, u8>);
    impl Memory {
        fn put(&mut self, address: u32, bytes: &[u8]) {
            for (offset, byte) in bytes.iter().copied().enumerate() {
                self.0.insert(address + offset as u32, byte);
            }
        }
    }
    impl ReadMemory for Memory {
        fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
            (0..size)
                .map(|offset| {
                    self.0
                        .get(&(address + offset as u32))
                        .copied()
                        .context("unmapped synthetic camera byte")
                })
                .collect()
        }
    }

    #[test]
    fn inventory_quantity_locator_requires_the_active_expected_item() {
        let mut memory = Memory(BTreeMap::new());
        let mut status = vec![0; 261];
        status[0] = 9;
        status[1..7].copy_from_slice(b"manoir");
        memory.put(super::super::hype_runtime::STATUS, &status);
        memory.put(0x6107b0, &0x2000u32.to_le_bytes());
        let mut root = [0u8; 12];
        root[8..12].copy_from_slice(&0x3000u32.to_le_bytes());
        memory.put(0x2000, &root);
        let mut entry = [0u8; 24];
        entry[4..8].copy_from_slice(&0x4000u32.to_le_bytes());
        memory.put(0x3000, &entry);
        let mut handle = [0u8; 12];
        handle[0..4].copy_from_slice(&0x5000u32.to_le_bytes());
        handle[8] = 1;
        memory.put(0x4000, &handle);
        let mut member = [0u8; 30];
        member[12..16].copy_from_slice(&0x6000u32.to_le_bytes());
        member[28..30].copy_from_slice(&80_i16.to_le_bytes());
        memory.put(0x5000, &member);
        memory.put(0x6000, b"Weapon\0");
        let build = HypeBuild::verified(HYPE_EXECUTABLE_SHA256).unwrap();
        handle[0..4].copy_from_slice(&0x5000u32.to_le_bytes());
        handle[10..12].copy_from_slice(&0_i16.to_le_bytes());
        memory.put(0x4000, &handle);
        member[10..12].copy_from_slice(&8_i16.to_le_bytes());
        memory.put(0x5000, &member);
        assert_eq!(
            build
                .inventory_quantity_address(&mut memory, "Weapon", 80, 0)
                .unwrap(),
            0x400a
        );
        let inventory = build.inventory(&mut memory).unwrap();
        assert_eq!(inventory.selected_weapon, 80);
        assert_eq!(inventory.active[0].quantity, 0);
        assert!(
            build
                .inventory_quantity_address(&mut memory, "Weapon", 80, 3)
                .is_err()
        );
    }

    #[test]
    fn fix_block_offsets_resolve_the_active_module_and_block() {
        let mut memory = Memory(BTreeMap::new());
        let module_5_table = 0x1000u32;
        let module_17_table = 0x2000u32;
        memory.put(0x0088_0f34, &module_5_table.to_le_bytes());
        memory.put(0x0088_0f64, &module_17_table.to_le_bytes());
        for (record, base, end) in [
            (module_5_table, 0x0100_0000u32, 0x0101_0000u32),
            (module_17_table, 0x0200_0000u32, 0x0201_0000u32),
        ] {
            memory.put(record, &base.to_le_bytes());
            memory.put(record + 4, &end.to_le_bytes());
            memory.put(record + 8, &[0; 8]);
        }
        let build = HypeBuild::verified(HYPE_EXECUTABLE_SHA256).unwrap();
        let mut source = MemoryView(&mut memory);
        assert_eq!(
            build
                .fix_block_offset(&mut source, 5, 0, 0x0100_0314)
                .unwrap()
                .unwrap()
                .payload_offset,
            0x314
        );
        assert_eq!(
            build
                .fix_block_offset(&mut source, 17, 0, 0x0200_001c)
                .unwrap()
                .unwrap()
                .payload_offset,
            0x1c
        );
        assert!(
            build
                .fix_block_offset(&mut source, 5, 0, 0x0101_0001)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn live_ai_model_search_finds_instances_across_the_world_tree() {
        let mut memory = Memory(BTreeMap::new());
        let root = 0x1000u32;
        let child = 0x2000u32;
        let perso = 0x3000u32;
        let brain = 0x4000u32;
        let mind = 0x5000u32;
        let model = 0x6000u32;
        memory.put(0x75ec6c, &root.to_le_bytes());
        let mut root_node = [0u8; 64];
        root_node[8..12].copy_from_slice(&child.to_le_bytes());
        root_node[12..16].copy_from_slice(&child.to_le_bytes());
        root_node[16..20].copy_from_slice(&1u32.to_le_bytes());
        memory.put(root, &root_node);
        let mut child_node = [0u8; 64];
        child_node[0..4].copy_from_slice(&4u32.to_le_bytes());
        child_node[4..8].copy_from_slice(&perso.to_le_bytes());
        child_node[28..32].copy_from_slice(&root.to_le_bytes());
        memory.put(child, &child_node);
        let mut perso_header = [0u8; 20];
        perso_header[16..20].copy_from_slice(&brain.to_le_bytes());
        memory.put(perso, &perso_header);
        memory.put(brain, &[mind.to_le_bytes().as_slice(), &[0; 4]].concat());
        memory.put(mind, &[model.to_le_bytes().as_slice(), &[0; 16]].concat());
        memory.put(model, &[0; 16]);

        let (instances, scanned_nodes) =
            find_live_actors_by_ai_model(&mut MemoryView(&mut memory), model).unwrap();
        assert_eq!(scanned_nodes, 2);
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].path, [0]);
        assert_eq!(instances[0].node, child);
    }

    #[test]
    fn active_camera_decodes_checked_projection_from_one_source_pointer() {
        let mut memory = Memory(BTreeMap::new());
        let mut status = vec![0; 261];
        status[0] = 9;
        status[1..7].copy_from_slice(b"manoir");
        memory.put(super::super::hype_runtime::STATUS, &status);
        let pointer = 0x1234_0000u32;
        memory.put(
            super::super::hype_runtime::ACTIVE_CAMERA_POINTER,
            &pointer.to_le_bytes(),
        );
        memory.put(pointer, &[0; 12]);
        memory.put(pointer + 8, &103u32.to_le_bytes());
        let mut pose_words = [0u32; 22];
        pose_words[0] = 0x80;
        for (word, value) in pose_words[1..4].iter_mut().zip([2.0f32, 3.0, 4.0]) {
            *word = value.to_bits();
        }
        let basis = [0.0f32, -1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0];
        for (word, value) in pose_words[4..13].iter_mut().zip(basis) {
            *word = value.to_bits();
        }
        for (word, value) in pose_words[13..22].iter_mut().zip(basis) {
            *word = value.to_bits();
        }
        let pose_block: Vec<u8> = pose_words
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        memory.put(pointer + 0x0c, &pose_block);
        for (offset, value) in [
            (0x64, 1.5f32),
            (0x68, 1.2f32),
            (0x6c, 0.25f32),
            (0x70, 10_000.0f32),
        ] {
            memory.put(pointer + offset, &value.to_le_bytes());
        }
        memory.put(
            super::super::hype_runtime::UPDATE_COUNTER,
            &123u32.to_le_bytes(),
        );
        memory.put(super::super::hype_runtime::CLOCK_MS, &456u32.to_le_bytes());
        let build = HypeBuild::verified(HYPE_EXECUTABLE_SHA256).unwrap();
        let camera = build.active_camera(&mut memory).unwrap();
        assert_eq!(camera.source_locator, pointer);
        assert_eq!(camera.position_source, [3.0, 4.0, -2.0]);
        assert_eq!(camera.forward_source, [1.0, 0.0, 0.0]);
        assert_eq!(camera.horizontal_fov, 1.5);
        assert_eq!(camera.vertical_fov, 1.2);
        assert_eq!(camera.near_plane, 0.25);
        assert_eq!(camera.far_plane, 10_000.0);
        assert_eq!(camera.source_update_counter, 123);
        assert_eq!(camera.source_clock_ms, 456);
    }

    #[test]
    fn camera_records_follow_four_checked_links_in_one_capture() {
        let mut memory = Memory(BTreeMap::new());
        let camera = 0x1234_0000u32;
        let mut root = [0u8; 224];
        root[0xac..0xb0].copy_from_slice(&103u32.to_le_bytes());
        root[0xdc..0xe0].copy_from_slice(&42u32.to_le_bytes());
        for index in 0..4usize {
            let address = camera + 0x1000 + (index as u32) * 0x100;
            root[index * 4..index * 4 + 4].copy_from_slice(&address.to_le_bytes());
            let mut record = [0u8; 168];
            record[0xa4..0xa8]
                .copy_from_slice(&(if index == 3 { 14u32 } else { 42u32 }).to_le_bytes());
            record[0x30..0x34].copy_from_slice(&(index as f32 + 3.5).to_le_bytes());
            memory.put(address, &record);
        }
        memory.put(camera, &root);
        let mut source = MemoryView(&mut memory);
        let records = camera_records_at(&mut source, camera).unwrap();
        assert_eq!(records.root_words.len(), 56);
        assert_eq!(records.records.len(), 4);
        assert_eq!(records.records[0].words[0x30 / 4], 3.5f32.to_bits());
        assert_eq!(records.records[3].words[0xa4 / 4], 14);
        assert_eq!(records.records[3].locator, "live:12341300");
    }

    #[test]
    fn live_camera_search_uses_checked_live_hierarchy_and_rejects_ambiguity() {
        fn node(words: [u32; 8]) -> [u8; 64] {
            let mut bytes = [0u8; 64];
            for (index, word) in words.into_iter().enumerate() {
                bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
            }
            bytes
        }
        let mut memory = Memory(BTreeMap::new());
        let (root, other, group, camera, perso, data3d, standard): (
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
        ) = (0x1000, 0x1100, 0x1200, 0x1300, 0x2000, 0x2100, 0x2200);
        memory.put(0x75ec6c, &root.to_le_bytes());
        memory.put(root, &node([0, 0, other, group, 2, 0, 0, 0]));
        memory.put(other, &node([0, 0, 0, 0, 0, group, 0, root]));
        memory.put(group, &node([0, 0, camera, camera, 1, 0, other, root]));
        memory.put(camera, &node([4, perso, 0, 0, 0, 0, 0, group]));
        let mut perso_bytes = [0u8; 24];
        perso_bytes[0..4].copy_from_slice(&data3d.to_le_bytes());
        perso_bytes[4..8].copy_from_slice(&standard.to_le_bytes());
        perso_bytes[20..24].copy_from_slice(&0x2300u32.to_le_bytes());
        memory.put(perso, &perso_bytes);
        let mut data3d_bytes = [0u8; 24];
        data3d_bytes[20..24].copy_from_slice(&0x2400u32.to_le_bytes());
        memory.put(data3d, &data3d_bytes);
        memory.put(standard, &[2u32.to_le_bytes(); 3].concat());
        let mut source = MemoryView(&mut memory);
        let found = find_live_camera(&mut source, [2, 2, 2]).unwrap();
        assert_eq!(found.path, [1, 0]);
        assert_eq!(found.scanned_nodes, 4);
        let global = find_live_actor_by_perso(&mut source, perso).unwrap();
        assert_eq!(global.perso, perso);
        assert_eq!(global.path, [1, 0]);
        assert_eq!(global.scanned_nodes, 4);

        let (second_perso, second_data3d, second_standard): (u32, u32, u32) =
            (0x3000, 0x3100, 0x3200);
        memory.put(other, &node([4, second_perso, 0, 0, 0, group, 0, root]));
        let mut second_perso_bytes = [0u8; 24];
        second_perso_bytes[0..4].copy_from_slice(&second_data3d.to_le_bytes());
        second_perso_bytes[4..8].copy_from_slice(&second_standard.to_le_bytes());
        second_perso_bytes[20..24].copy_from_slice(&0x3300u32.to_le_bytes());
        memory.put(second_perso, &second_perso_bytes);
        memory.put(second_data3d, &data3d_bytes);
        memory.put(second_standard, &[2u32.to_le_bytes(); 3].concat());
        let mut source = MemoryView(&mut memory);
        assert!(
            find_live_camera(&mut source, [2, 2, 2])
                .unwrap_err()
                .to_string()
                .contains("found 2")
        );
    }
}

fn scene_node(source: &mut MemoryView<'_, impl ReadMemory>, child_path: &[u32]) -> Result<u32> {
    ensure!(
        !child_path.is_empty() && child_path.len() <= 32,
        "scene child path depth outside 1..32"
    );
    // The verified Hype build stores ActualWorld here. Resolve it anew for
    // each capture; source child ordinals are not runtime addresses.
    let mut parent = source.required(0x75ec6c, 0)?;
    let root: SuperObjectWire = wire(&source.bytes(parent, 64)?)?;
    ensure!(root.kind == 0, "ActualWorld root is not a world node");
    let mut traversed = 0_u32;
    for &index in child_path {
        let header: SuperObjectWire = wire(&source.bytes(parent, 64)?)?;
        ensure!(
            header.child_count <= 100_000 && index < header.child_count,
            "scene child index outside bounded list"
        );
        traversed = traversed
            .checked_add(index + 1)
            .context("scene path overflow")?;
        ensure!(traversed <= 100_000, "scene path traversal limit");
        let mut child = source.required(parent, 8)?;
        let mut previous = None;
        for step in 0..=index {
            let item: SuperObjectWire = wire(&source.bytes(child, 64)?)?;
            ensure!(item.parent == parent, "scene child parent link differs");
            if let Some(previous) = previous {
                ensure!(item.previous == previous, "scene sibling link differs");
            }
            previous = Some(child);
            // The next pointer is only needed before the requested ordinal.
            // Its absence is caught by `required` on the next iteration.
            if step < index {
                child = source.required(child, 20)?;
            }
        }
        parent = child;
    }
    let terminal: SuperObjectWire = wire(&source.bytes(parent, 64)?)?;
    ensure!(terminal.kind == 4, "scene child path is not a perso");
    Ok(parent)
}

#[derive(Debug)]
struct LiveActorMatch {
    node: u32,
    perso: u32,
    path: Vec<u32>,
    scanned_nodes: usize,
}

fn find_live_camera(
    source: &mut MemoryView<'_, impl ReadMemory>,
    expected_object_types: [u32; 3],
) -> Result<LiveActorMatch> {
    find_live_actor(source, expected_object_types, true)
}

fn find_live_actor(
    source: &mut MemoryView<'_, impl ReadMemory>,
    expected_object_types: [u32; 3],
    require_camera: bool,
) -> Result<LiveActorMatch> {
    find_live_actor_matching(source, require_camera, |source, perso| {
        Ok(source_view::perso_parts(source, perso)?.object_types == expected_object_types)
    })
}

fn find_live_actor_by_perso(
    source: &mut MemoryView<'_, impl ReadMemory>,
    expected_perso: u32,
) -> Result<LiveActorMatch> {
    find_live_actor_matching(source, false, |_, perso| Ok(perso == expected_perso))
}

fn find_live_actor_matching<M: ReadMemory>(
    source: &mut MemoryView<'_, M>,
    require_camera: bool,
    mut matches_actor: impl FnMut(&mut MemoryView<'_, M>, u32) -> Result<bool>,
) -> Result<LiveActorMatch> {
    const MAX_NODES: usize = 4096;
    const MAX_DEPTH: usize = 32;
    let root = source.required(0x75ec6c, 0)?;
    let mut stack = vec![(root, Vec::<u32>::new())];
    let mut visited = BTreeSet::from([root]);
    let mut found: Option<LiveActorMatch> = None;
    let mut matches = 0usize;
    while let Some((node, path)) = stack.pop() {
        let header: SuperObjectWire = wire(&source.bytes(node, 64)?)?;
        if path.is_empty() {
            ensure!(header.kind == 0, "ActualWorld root is not a world node");
        }
        if header.kind == 4 {
            let perso = source.required(node, SuperObjectWire::DATA)?;
            if matches_actor(source, perso)?
                && (!require_camera || source.pointer(perso, 20)?.is_some())
            {
                matches += 1;
                if found.is_none() {
                    found = Some(LiveActorMatch {
                        node,
                        perso,
                        path: path.clone(),
                        scanned_nodes: 0,
                    });
                }
            }
        }
        let count = usize::try_from(header.child_count)?;
        ensure!(
            count <= MAX_NODES,
            "live scene child count exceeds search bound"
        );
        if count == 0 {
            ensure!(
                header.first_child == 0 && header.last_child == 0,
                "empty live scene node has child endpoints"
            );
            continue;
        }
        ensure!(path.len() < MAX_DEPTH, "live scene search depth exceeded");
        ensure!(
            visited.len().saturating_add(count) <= MAX_NODES,
            "live scene search node budget exceeded"
        );
        ensure!(
            header.first_child != 0 && header.last_child != 0,
            "live scene node lacks child endpoints"
        );
        let mut child = header.first_child;
        let mut previous = 0u32;
        let mut children = Vec::with_capacity(count);
        for index in 0..count {
            ensure!(child != 0, "live scene sibling list ended early");
            ensure!(visited.insert(child), "live scene cycle or repeated child");
            let member: SuperObjectWire = wire(&source.bytes(child, 64)?)?;
            ensure!(
                member.parent == node,
                "live scene child parent link differs"
            );
            ensure!(
                member.previous == previous,
                "live scene previous sibling link differs"
            );
            if index + 1 == count {
                ensure!(
                    child == header.last_child && member.next == 0,
                    "live scene last child endpoint differs"
                );
            } else {
                ensure!(member.next != 0, "live scene sibling list ended early");
            }
            let mut child_path = path.clone();
            child_path.push(u32::try_from(index)?);
            children.push((child, child_path));
            previous = child;
            child = member.next;
        }
        stack.extend(children.into_iter().rev());
    }
    ensure!(
        matches == 1,
        "live actor search expected one matching actor (camera required: {require_camera}), found {matches} in {} nodes",
        visited.len()
    );
    let mut result = found.context("live actor search lost its unique match")?;
    result.scanned_nodes = visited.len();
    Ok(result)
}

struct LiveSceneActor {
    node: u32,
    path: Vec<u32>,
}

fn find_live_actors_by_ai_model<M: ReadMemory>(
    source: &mut MemoryView<'_, M>,
    expected_model: u32,
) -> Result<(Vec<LiveSceneActor>, usize)> {
    const MAX_NODES: usize = 4096;
    const MAX_DEPTH: usize = 32;
    let root = source.required(0x75ec6c, 0)?;
    let mut stack = vec![(root, Vec::<u32>::new())];
    let mut visited = BTreeSet::from([root]);
    let mut found = Vec::new();
    while let Some((node, path)) = stack.pop() {
        let header: SuperObjectWire = wire(&source.bytes(node, 64)?)?;
        if path.is_empty() {
            ensure!(header.kind == 0, "ActualWorld root is not a world node");
        }
        if header.kind == 4 {
            let perso = source.required(node, SuperObjectWire::DATA)?;
            if source.pointer(perso, 16)?.is_some()
                && source_view::ai_parts(source, perso)?.model == expected_model
            {
                found.push(LiveSceneActor {
                    node,
                    path: path.clone(),
                });
            }
        }
        let count = usize::try_from(header.child_count)?;
        ensure!(
            count <= MAX_NODES,
            "live scene child count exceeds search bound"
        );
        if count == 0 {
            ensure!(
                header.first_child == 0 && header.last_child == 0,
                "empty live scene node has child endpoints"
            );
            continue;
        }
        ensure!(path.len() < MAX_DEPTH, "live scene search depth exceeded");
        ensure!(
            visited.len().saturating_add(count) <= MAX_NODES,
            "live scene search node budget exceeded"
        );
        ensure!(
            header.first_child != 0 && header.last_child != 0,
            "live scene node lacks child endpoints"
        );
        let mut child = header.first_child;
        let mut previous = 0u32;
        let mut children = Vec::with_capacity(count);
        for index in 0..count {
            ensure!(child != 0, "live scene sibling list ended early");
            ensure!(visited.insert(child), "live scene cycle or repeated child");
            let member: SuperObjectWire = wire(&source.bytes(child, 64)?)?;
            ensure!(
                member.parent == node,
                "live scene child parent link differs"
            );
            ensure!(
                member.previous == previous,
                "live scene previous sibling link differs"
            );
            if index + 1 == count {
                ensure!(
                    child == header.last_child && member.next == 0,
                    "live scene last child endpoint differs"
                );
            } else {
                ensure!(member.next != 0, "live scene sibling list ended early");
            }
            let mut child_path = path.clone();
            child_path.push(u32::try_from(index)?);
            children.push((child, child_path));
            previous = child;
            child = member.next;
        }
        stack.extend(children.into_iter().rev());
    }
    Ok((found, visited.len()))
}

fn camera_records_at(
    source: &mut MemoryView<'_, impl ReadMemory>,
    camera: u32,
) -> Result<SourceCameraRecordsObservation> {
    let root = source.bytes(camera, 224)?.to_vec();
    ensure!(
        u32_at(&root, 0xac)? == 103 && u32_at(&root, 0xdc)? == 42,
        "source camera root layout differs"
    );
    let mut records = Vec::with_capacity(4);
    for index in 0..4 {
        let address = source.required(camera, index * 4)?;
        ensure!(address != camera, "source camera record aliases its root");
        let bytes = source.bytes(address, 168)?;
        let expected_tail = if index == 3 { 14 } else { 42 };
        ensure!(
            u32_at(&bytes, 0xa4)? == expected_tail,
            "source camera record {index} layout differs"
        );
        records.push(SourceCameraRecordObservation {
            locator: format!("live:{address:08x}"),
            words: bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|word| u32::from_le_bytes(*word))
                .collect(),
        });
    }
    Ok(SourceCameraRecordsObservation {
        root_locator: format!("live:{camera:08x}"),
        root_words: root
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| u32::from_le_bytes(*word))
            .collect(),
        records,
    })
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub phase: u8,
    pub level: String,
}

/// Immutable provenance for exactly the level source files consumed by the
/// selected `SourceSession`. This is report evidence, never a Rete asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFingerprint {
    pub profile: String,
    pub level: String,
    pub locale: String,
    pub files: Vec<SourceFileFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFileFingerprint {
    pub relative_path: String,
    pub sha256: String,
    pub byte_count: u64,
}

pub fn source_fingerprint(
    game: &std::path::Path,
    selection: &crate::model::Selection,
) -> Result<SourceFingerprint> {
    // Opening first proves these are the exact parseable inputs selected by the
    // source adapter, rather than a parallel list that merely happens to exist.
    let (session, files) =
        super::session::SourceSession::open_with_fingerprint_files(game, selection)?;
    drop(session);
    let files = files
        .entries()
        .map(|(relative, absolute)| {
            let bytes = std::fs::read(absolute)
                .with_context(|| format!("fingerprint {}", absolute.display()))?;
            Ok(SourceFileFingerprint {
                relative_path: relative
                    .to_str()
                    .context("non-UTF-8 source relative path")?
                    .replace(std::path::MAIN_SEPARATOR, "/"),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                byte_count: bytes.len().try_into()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SourceFingerprint {
        profile: selection.profile.clone(),
        level: selection.level.clone(),
        locale: selection.locale.clone(),
        files,
    })
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryItemObservation {
    pub kind: String,
    pub id: i16,
    pub quantity: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryObservation {
    pub total_entries: usize,
    pub active: Vec<InventoryItemObservation>,
    /// Function 201 returns -1 when the active Weapon entry is absent.
    pub selected_weapon: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicSelectionObservation {
    pub current_points: u16,
    pub maximum_points: u16,
    pub selected_index: u8,
    pub selected_name: String,
    pub source_code: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaserPointerObservation {
    pub point: [f32; 3],
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTailChild {
    pub index: u32,
    pub kind: u32,
    /// Capture-local addresses obtained from validated DynamicWorld links.
    /// They are observation evidence, not persistent asset identities.
    pub node_locator: String,
    pub perso_locator: Option<String>,
    /// Source StandardGame frame stamp, not a completed-update oracle by itself.
    pub update_stamp: Option<u32>,
    pub dynamics: Option<SceneTailDynamics>,
    pub object_types: Option<[u32; 3]>,
    pub global_transform: Option<TransformValue>,
    pub ai: Option<SceneTailAi>,
}

/// Source integrator state read from the validated SuperObject → Perso →
/// dynamics-wrapper → dynamics-object chain. The flag is a sampled raw value;
/// it does not describe whether a previous or later update applied gravity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SceneTailDynamics {
    Absent,
    Ready {
        wrapper_locator: String,
        object_locator: String,
        override_flag: u8,
        cached_card_locator: Option<String>,
        cached_card_type: u32,
        /// Source type-0 getters on the cached card; candidate callback inputs.
        candidate_card_mass: Option<f32>,
        candidate_card_ratio: Option<f32>,
        cached_mass: f32,
        integrated_velocity: [f32; 3],
        pending_override_velocity: [f32; 3],
        mode_e0: u32,
        contact_classes: SceneTailContactClasses,
        list_locator: Option<String>,
        card_slots: Vec<SceneTailCardSlot>,
        /// Last nonnull card in the source selection walk, if any.
        selected_card_locator: Option<String>,
        selected_card_type: Option<u32>,
    },
    Unreadable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTailContactClasses {
    /// Mechanics +0xf0 and +0xf4 are the carried and current class words.
    pub previous: u32,
    pub current: u32,
    /// Mechanics +0xf8 is the class word read by the Hype condition handler.
    pub ai_visible: u32,
    /// Raw condition operation 33 compares its mechanics contact word to zero.
    pub collide_with_nothing: bool,
    /// A false value can be a stopped capture between the publisher's stores.
    pub publication_consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTailCardSlot {
    pub index: u8,
    pub key: u32,
    pub card_locator: Option<String>,
    pub card_type: Option<u32>,
}

/// A validated live model link is capture-local evidence. Resolve it against
/// source allocations before claiming any imported `ai/models/*` identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SceneTailAi {
    BrainAbsent,
    Ready {
        model_locator: String,
        fix_block_offset: Option<LiveBlockOffset>,
        normal_count: usize,
        reflex_count: usize,
        designer_field_count: usize,
        designer_values: Vec<SelectedDesignerValue>,
        normal: Option<BehaviorSelection>,
        reflex: Option<BehaviorSelection>,
    },
    Unreadable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveBlockOffset {
    pub module: u16,
    pub block: u16,
    pub payload_offset: u32,
    pub active_base_locator: String,
    pub active_end_locator: String,
}

/// Checked source identity and live value for manor Global's crossbow control.
/// Locators are capture-local; block offsets identify the source allocations.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossbowGlobalObservation {
    pub actor: ActorAiObservation,
    pub scene_path: Vec<u32>,
    pub scanned_nodes: usize,
    pub perso_fix_block_offset: LiveBlockOffset,
    pub ai_model_fix_block_offset: LiveBlockOffset,
    pub allocation_header_words: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAiModelSearchObservation {
    pub model: LiveBlockOffset,
    pub scanned_nodes: usize,
    pub instances: Vec<SceneAiModelInstance>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAiModelInstance {
    pub scene_path: Vec<u32>,
    pub actor: ActorObservation,
    pub normal: Option<BehaviorSelection>,
    pub reflex: Option<BehaviorSelection>,
    pub designer_values: Vec<SelectedDesignerValue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneNearbyActorsObservation {
    pub center: [f32; 3],
    pub radius: f32,
    pub scanned_nodes: usize,
    pub actors: Vec<SceneNearbyActor>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneNearbyActor {
    pub scene_path: Vec<u32>,
    pub object_types: [u32; 3],
    pub distance: f32,
    pub global_transform: TransformValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTailObservation {
    pub child_count: u32,
    pub children: Vec<SceneTailChild>,
}

/// Source-local locators are evidence within this capture only. Equal values do
/// not merge locators; reuse of an address across captures requires generation
/// and replacement evidence from the controller, which is not supplied here.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorObservation {
    pub node: String,
    pub perso: String,
    pub data3d: String,
    pub family: String,
    pub object_types: [u32; 3],
    pub family_type: u32,
    pub action_members: Vec<String>,
    pub action_definitions: Vec<ActionDefinitionProjection>,
    pub initial_action: Option<u32>,
    pub current_action: Option<u32>,
    pub secondary_action: Option<u32>,
    pub global_transform: TransformValue,
}

/// Active source viewport camera values observed during the same stopped-byte
/// capture as a selected actor. `pose_words` includes the engine's packed
/// matrix header followed by its 21 transform words; it is not yet interpreted
/// as a reusable camera model transform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveCameraObservation {
    pub source_locator: u32,
    /// Camera origin in OpenSpace engine coordinates, decoded from its view
    /// basis and translation using `position = -basis * translation`.
    pub position_source: [f32; 3],
    /// Normalized third row of the source view basis, pointing from camera
    /// toward the scene in OpenSpace engine coordinates.
    pub forward_source: [f32; 3],
    pub pose_words: [u32; 22],
    pub horizontal_fov: f32,
    pub vertical_fov: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    /// Global source update counter, included as a checkpoint only; it does
    /// not prove a completed-update publication boundary.
    pub source_update_counter: u32,
    pub source_clock_ms: u32,
}

/// Stopped-byte source camera content for research. Addresses and raw pointer
/// words are capture-local provenance and must not enter Rete or Godot output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCameraRecordsObservation {
    pub root_locator: String,
    pub root_words: Vec<u32>,
    pub records: Vec<SourceCameraRecordObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCameraRecordObservation {
    pub locator: String,
    pub words: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCameraObservation {
    pub live_child_path: Vec<u32>,
    pub scanned_nodes: usize,
    pub actor: ActorAiObservation,
    pub source_camera_records: SourceCameraRecordsObservation,
    pub active_camera: ActiveCameraObservation,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorAiObservation {
    pub actor: ActorObservation,
    pub normal: Option<BehaviorSelection>,
    pub reflex: Option<BehaviorSelection>,
    pub designer_values: Vec<SelectedDesignerValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedDesignerValue {
    pub index: u32,
    pub kind: u32,
    pub value: ObservedDesignerValue,
}

/// Reference-bearing values use family member ids or capture-local actor labels;
/// every other kind is exactly the useful model value decoded by the shared codec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObservedDesignerValue {
    Value(DesignerValue),
    Action(Option<u32>),
    ActionArray(DesignerArray<Option<u32>>),
    Actor(Option<String>),
}

/// Cross-adapter meaning for the selected controlled actor and its resolved
/// family. Source locators and runtime generations remain observation evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorProjection {
    pub role: ActorRole,
    pub object_types: [u32; 3],
    pub family: FamilyProjection,
    /// File initialization or named-fixture state rather than definition data.
    pub selections: ActorSelections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    MainActor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyProjection {
    pub object_type: u32,
    pub action_count: u32,
    /// Requested definitions only; other family actions remain unselected.
    pub selected_actions: Vec<ActionDefinitionProjection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDefinitionProjection {
    pub id: u32,
    pub speed: u8,
    pub repeats: u8,
    pub transition_lock: u8,
    pub next: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSelections {
    pub initial: Option<u32>,
    pub current: Option<u32>,
    pub secondary: Option<u32>,
}

impl ActorObservation {
    pub fn projection(&self) -> ActorProjection {
        ActorProjection {
            role: ActorRole::MainActor,
            object_types: self.object_types,
            family: FamilyProjection {
                object_type: self.family_type,
                action_count: self.action_members.len() as u32,
                selected_actions: self.action_definitions.clone(),
            },
            selections: ActorSelections {
                initial: self.initial_action,
                current: self.current_action,
                secondary: self.secondary_action,
            },
        }
    }
}

impl ActorProjection {
    /// The caller supplies the family resolved by `actor.family`; this verifies
    /// that relationship instead of joining by an address, ordinal or value hash.
    pub fn from_model(actor: &Perso, family: &Family, actions: &[u32]) -> Result<Self> {
        ensure!(
            actions.iter().copied().collect::<BTreeSet<_>>().len() == actions.len(),
            "duplicate selected action"
        );
        ensure!(
            actor.family.path == family.path,
            "actor family reference does not resolve to supplied family"
        );
        ensure!(
            family.object_type == actor.object_types[0],
            "actor object type and family index differ"
        );
        ensure!(
            family
                .states
                .iter()
                .enumerate()
                .all(|(index, state)| state.id == index as u32),
            "noncanonical family state order"
        );
        let selected_actions = actions
            .iter()
            .map(|&id| {
                let state = family
                    .states
                    .get(id as usize)
                    .context("selected action outside family")?;
                Ok(ActionDefinitionProjection {
                    id: state.id,
                    speed: state.speed,
                    repeats: state.repeats,
                    transition_lock: state.transition_lock,
                    next: state.next,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            role: ActorRole::MainActor,
            object_types: actor.object_types,
            family: FamilyProjection {
                object_type: family.object_type,
                action_count: family.states.len().try_into()?,
                selected_actions,
            },
            selections: ActorSelections {
                initial: actor.initial_state,
                current: actor.current_state,
                secondary: actor.secondary_state,
            },
        })
    }
}
pub(super) fn actor<S: RecordSource>(
    source: &mut S,
    entry: S::Location,
    selected_actions: &[u32],
    label: impl Fn(S::Location) -> String,
) -> Result<ActorObservation> {
    let _: MainActorWire = wire(&source.bytes(entry, 20)?)?;
    let node = source.required(entry, 4)?;
    actor_at_node(source, node, selected_actions, label)
}

fn actor_at_node<S: RecordSource>(
    source: &mut S,
    node: S::Location,
    selected_actions: &[u32],
    label: impl Fn(S::Location) -> String,
) -> Result<ActorObservation> {
    ensure!(
        selected_actions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            == selected_actions.len(),
        "duplicate selected action"
    );
    let so: SuperObjectWire = wire(&source.bytes(node, 64)?)?;
    ensure!(so.kind == 4, "controlled node is not a perso");
    let perso = source.required(node, 4)?;
    let parts = source_view::perso_parts(source, perso)?;
    let data3d = parts.data3d;
    let object_types = parts.object_types;
    let family = parts.family;
    let header: FamilyWire = wire(&source.bytes(family, 56)?)?;
    ensure!(
        header.index == object_types[0],
        "actor object type and family index differ"
    );
    let members = source_view::list(source, family, 16)?;
    let action_definitions = selected_actions
        .iter()
        .map(|&id| {
            let member = *members
                .get(id as usize)
                .context("selected action outside family")?;
            let state = source_view::state_definition(source, member, &members)?;
            Ok(ActionDefinitionProjection {
                id,
                speed: state.speed,
                repeats: state.repeats,
                transition_lock: state.transition_lock,
                next: state.next,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let initial_action = source_view::optional_member(source, data3d, 0, &members)?;
    let current_action = source_view::optional_member(source, data3d, 4, &members)?;
    let secondary_action = source_view::optional_member(source, data3d, 8, &members)?;
    let transform = source.required(node, 36)?;
    let global_transform = source_view::transform(source, transform)?;
    Ok(ActorObservation {
        node: label(node),
        perso: label(perso),
        data3d: label(data3d),
        family: label(family),
        object_types,
        family_type: header.index,
        action_members: members.iter().copied().map(&label).collect(),
        action_definitions,
        initial_action,
        current_action,
        secondary_action,
        global_transform,
    })
}
/// File-side diagnostic projection uses the same readers and native file
/// relocations, without importing a second document model or constructing live pointers.
pub fn inspect_file_actor(
    game: &std::path::Path,
    selection: &crate::model::Selection,
) -> Result<ActorObservation> {
    let session = super::session::SourceSession::open(game, selection)?;
    let root = session.root(0x2ae0)?;
    actor(&mut source_view::FileView(&session), root, &[47], |a| {
        format!(
            "file:{:?}:{:02x}:{:02x}+{:x}",
            a.owner, a.key.0, a.key.1, a.offset
        )
    })
}
