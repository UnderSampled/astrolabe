//! Bounded source access, separate from record meaning. The selected readers use
//! this interface for file relocations and relocated 32-bit live pointers.
//! No allocator ordinal or persistent asset identity is part of this contract.
use super::{
    container::{u32_at, wire},
    records::{Address, ListWire},
    session::SourceSession,
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
