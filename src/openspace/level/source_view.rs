//! Bounded source access, separate from record meaning. The selected readers use
//! this interface for file relocations and relocated 32-bit live pointers.
//! No allocator ordinal or persistent asset identity is part of this contract.
use super::{
    container::{u32_at, wire},
    records::{
        Address, BrainWire, DesignerWire, InfoWire, IntelligenceWire, ListWire, MemoryWire,
        MindWire, ModelWire, PersoHeaderWire, SetWire,
    },
    session::SourceSession,
};
use crate::{
    model::{
        AssetRef,
        ai::{BehaviorSelection, DesignerValue},
        character::{Perso, StateRef},
    },
    openspace::{ai_values::DesignerValueCodec, codec::FormatCodec},
};
use anyhow::{Context, Result, ensure};
use std::{borrow::Cow, collections::BTreeSet, fmt::Debug};

pub(super) trait RecordSource {
    type Location: Copy + Ord + Debug;
    fn add(&self, address: Self::Location, offset: usize) -> Result<Self::Location>;
    fn bytes(&mut self, address: Self::Location, size: usize) -> Result<Cow<'_, [u8]>>;
    fn pointer(&mut self, address: Self::Location, offset: usize)
    -> Result<Option<Self::Location>>;
    fn required(&mut self, address: Self::Location, offset: usize) -> Result<Self::Location> {
        self.pointer(address, offset)?
            .with_context(|| format!("required null reference at {address:?}+{offset:x}"))
    }
}
pub(super) struct FileView<'a>(pub(super) &'a SourceSession);
impl RecordSource for FileView<'_> {
    type Location = Address;
    fn add(&self, a: Address, offset: usize) -> Result<Address> {
        Ok(Address {
            offset: a
                .offset
                .checked_add(offset)
                .context("source offset overflow")?,
            ..a
        })
    }
    fn bytes(&mut self, a: Address, size: usize) -> Result<Cow<'_, [u8]>> {
        Ok(Cow::Borrowed(self.0.bytes(a, size)?))
    }
    fn pointer(&mut self, a: Address, offset: usize) -> Result<Option<Address>> {
        self.0.pointer(a, offset)
    }
}

pub(super) struct PersoParts<L> {
    pub(super) data3d: L,
    pub(super) family: L,
    pub(super) object_types: [u32; 3],
}

pub(super) struct AiParts<L> {
    pub(super) mind: L,
    pub(super) model: L,
}

pub(super) struct BehaviorSetView<L> {
    pub(super) address: L,
    pub(super) members: Vec<L>,
}

pub(super) struct DesignerView<L> {
    pub(super) values: L,
    pub(super) length: usize,
    pub(super) fields: Vec<InfoWire>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StateDefinition {
    pub(super) speed: u8,
    pub(super) repeats: u8,
    pub(super) transition_lock: u8,
    pub(super) next: Option<u32>,
}

pub(super) fn state_definition<S: RecordSource>(
    source: &mut S,
    state: S::Location,
    members: &[S::Location],
) -> Result<StateDefinition> {
    use super::records::CharacterStateWire;
    let raw: CharacterStateWire = wire(&source.bytes(state, 64)?)?;
    Ok(StateDefinition {
        speed: raw.speed,
        repeats: raw.repeats,
        transition_lock: raw.transition_lock,
        next: optional_member(source, state, 40, members)?,
    })
}
pub(super) fn perso_parts<S: RecordSource>(
    source: &mut S,
    perso: S::Location,
) -> Result<PersoParts<S::Location>> {
    use super::records::{PersoData3dPrefixWire, PersoHeaderWire, StandardIdentityWire};
    let _: PersoHeaderWire = wire(&source.bytes(perso, 20)?)?;
    let data3d = source.required(perso, 0)?;
    let _: PersoData3dPrefixWire = wire(&source.bytes(data3d, 24)?)?;
    let standard = source.required(perso, 4)?;
    let identity: StandardIdentityWire = wire(&source.bytes(standard, 12)?)?;
    Ok(PersoParts {
        data3d,
        family: source.required(data3d, 20)?,
        object_types: identity.object_types,
    })
}

pub(super) fn ai_parts<S: RecordSource>(
    source: &mut S,
    perso: S::Location,
) -> Result<AiParts<S::Location>> {
    let header: PersoHeaderWire = wire(&source.bytes(perso, 20)?)?;
    ensure!(header.brain != 0, "perso has no AI brain");
    let brain = source.required(perso, 16)?;
    let _: BrainWire = wire(&source.bytes(brain, 8)?)?;
    let mind = source.required(brain, 0)?;
    let _: MindWire = wire(&source.bytes(mind, 20)?)?;
    let model = source.required(mind, 0)?;
    let _: ModelWire = wire(&source.bytes(model, 16)?)?;
    Ok(AiParts { mind, model })
}

pub(super) fn behavior_set<S: RecordSource>(
    source: &mut S,
    model: S::Location,
    field: usize,
) -> Result<Option<BehaviorSetView<S::Location>>> {
    let Some(address) = source.pointer(model, field)? else {
        return Ok(None);
    };
    let header: SetWire = wire(&source.bytes(address, 16)?)?;
    ensure!(header.count <= 65535, "too many AI behaviors");
    let mut members = Vec::with_capacity(header.count as usize);
    if header.count > 0 {
        let entries = source.required(address, 0)?;
        let size = (header.count as usize)
            .checked_mul(12)
            .context("behavior entry extent overflow")?;
        // Validate the whole declared contiguous entry table before exposing
        // any member location to a caller.
        source.bytes(entries, size)?;
        for index in 0..header.count as usize {
            members.push(source.add(entries, index * 12)?);
        }
    }
    Ok(Some(BehaviorSetView { address, members }))
}

pub(super) fn behavior_selection<S: RecordSource>(
    source: &mut S,
    mind: S::Location,
    field: usize,
    set: Option<&BehaviorSetView<S::Location>>,
) -> Result<Option<BehaviorSelection>> {
    let Some(address) = source.pointer(mind, field)? else {
        return Ok(None);
    };
    let _: IntelligenceWire = wire(&source.bytes(address, 24)?)?;
    let set = set.context("intelligence lacks behavior set")?;
    let slot = source.required(address, 0)?;
    ensure!(
        source.required(slot, 0)? == set.address,
        "intelligence binds another behavior set"
    );
    Ok(Some(BehaviorSelection {
        current: optional_member(source, address, 8, &set.members)?,
        previous: optional_member(source, address, 12, &set.members)?,
        initial: optional_member(source, address, 20, &set.members)?,
    }))
}

pub(super) fn designer_view<S: RecordSource>(
    source: &mut S,
    model: S::Location,
    mind: S::Location,
) -> Result<Option<DesignerView<S::Location>>> {
    let Some(definition) = source.pointer(model, 8)? else {
        return Ok(None);
    };
    let header: DesignerWire = wire(&source.bytes(definition, 16)?)?;
    let mut fields = Vec::with_capacity(usize::from(header.count));
    if header.count > 0 {
        let infos = source.required(definition, 4)?;
        let size = usize::from(header.count) * 16;
        source.bytes(infos, size)?;
        for index in 0..usize::from(header.count) {
            fields.push(wire(&source.bytes(source.add(infos, index * 16)?, 16)?)?);
        }
    }
    let mut values = source.required(definition, 0)?;
    if let Some(memory) = source.pointer(mind, 12)? {
        let _: MemoryWire = wire(&source.bytes(memory, 12)?)?;
        let slot = source.required(memory, 0)?;
        ensure!(
            source.required(slot, 0)? == definition,
            "designer memory refers to another definition"
        );
        if let Some(current) = source.pointer(memory, 8)? {
            values = current;
        }
    }
    Ok(Some(DesignerView {
        values,
        length: header.length as usize,
        fields,
    }))
}

/// Decode one declared designer value through the production value codec. The
/// caller supplies semantic bindings for relocated action and actor pointers;
/// this keeps capture-local locations out of the useful model.
pub(super) fn designer_value<S, A, P>(
    source: &mut S,
    values: S::Location,
    info: InfoWire,
    length: usize,
    mut action: A,
    mut actor: P,
) -> Result<(DesignerValue, usize)>
where
    S: RecordSource,
    A: FnMut(Option<S::Location>) -> Result<Option<StateRef>>,
    P: FnMut(Option<S::Location>) -> Result<Option<AssetRef<Perso>>>,
{
    let offset: usize = info.offset.try_into()?;
    let address = source.add(values, offset)?;
    let mut codec = DesignerValueCodec {
        kind: info.kind,
        ..Default::default()
    };
    let prefix = match info.kind {
        20..=22 | 25 => 8,
        0..=2 => 1,
        3..=4 => 2,
        5..=7 | 10 | 14 => 4,
        12 => 12,
        _ => anyhow::bail!("unsupported designer type {}", info.kind),
    };
    let remaining = length
        .checked_sub(offset)
        .context("designer offset outside memory")?;
    ensure!(
        prefix <= remaining,
        "designer value exceeds declared memory"
    );
    let extent = codec.extent(&source.bytes(address, prefix)?)?;
    ensure!(
        extent <= remaining,
        "designer value exceeds declared memory"
    );
    let bytes = source.bytes(address, extent)?.into_owned();
    let operands: Vec<usize> = match info.kind {
        14 => vec![0],
        25 => (0..(extent - 8) / 12).map(|index| 8 + 12 * index).collect(),
        _ => Vec::new(),
    };
    for operand in operands {
        let site = source.add(address, operand)?;
        let target = if matches!(u32_at(&source.bytes(site, 4)?, 0)?, 0 | u32::MAX) {
            None
        } else {
            Some(source.required(site, 0)?)
        };
        codec.actions.insert(operand, action(target)?);
    }
    if info.kind == 10 {
        let target = if matches!(u32_at(&source.bytes(address, 4)?, 0)?, 0 | u32::MAX) {
            None
        } else {
            Some(source.required(address, 0)?)
        };
        codec.actors.insert(0, actor(target)?);
    }
    Ok((codec.read(&bytes)?, extent))
}

/// Intrusive-list order is meaningful (family action members). The source count
/// bounds traversal; revisiting a member within that count is never a new object.
pub(super) fn list<S: RecordSource>(
    source: &mut S,
    address: S::Location,
    field: usize,
) -> Result<Vec<S::Location>> {
    let header = source.add(address, field)?;
    let h: ListWire = wire(&source.bytes(header, 12)?)?;
    ensure!(h.count <= 65536, "excessive linked-list count");
    let mut next = source.pointer(header, 0)?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for _ in 0..h.count {
        let entry = next.context("short linked list")?;
        ensure!(seen.insert(entry), "cyclic linked list");
        result.push(entry);
        next = source.pointer(entry, 0)?;
    }
    Ok(result)
}
/// Determine the declared transform's exact byte extent before decoding it.
/// Never read an arbitrary maximum that can cross an allocation/page boundary.
pub(super) fn transform<S: RecordSource>(
    source: &mut S,
    address: S::Location,
) -> Result<crate::model::animation::TransformValue> {
    let header = source.bytes(address, 2)?;
    let flags = u16::from_le_bytes(header.as_ref().try_into()?);
    let size = if flags & 0x80 != 0 {
        88
    } else {
        match flags {
            0x11 => 8,
            0x22 => 10,
            0x33 => 16,
            0x77 => 18,
            0x7b => 22,
            0x7f => 28,
            _ => anyhow::bail!("unsupported compact transform flags {flags:#x}"),
        }
    };
    super::transform::read_transform(&source.bytes(address, size)?)
}

pub(super) fn optional_member<S: RecordSource>(
    source: &mut S,
    address: S::Location,
    field: usize,
    members: &[S::Location],
) -> Result<Option<u32>> {
    let site = source.add(address, field)?;
    if matches!(u32_at(&source.bytes(site, 4)?, 0)?, 0 | u32::MAX) {
        return Ok(None);
    }
    let target = source.required(site, 0)?;
    Ok(Some(
        members
            .iter()
            .position(|a| *a == target)
            .context("state reference outside its family")?
            .try_into()?,
    ))
}

#[cfg(test)]
#[path = "source_view_tests.rs"]
mod tests;
