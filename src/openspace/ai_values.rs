//! Designer values use the same useful types as AI memory and Rete.
//! 005236a0 declares sizes; 00524730 initializes arrays with twelve-byte slots,
//! and 00523a20 indexes those slots. Raymap's compact-array reader is not correct
//! for these Montreal arrays. No pointer or storage offset enters numeric values.
use crate::model::{ai::*, character::StateRef, io::FormatSupport};
use crate::openspace::codec::FormatCodec;
use anyhow::{Context, Result, bail, ensure};
use binrw::{BinRead, BinWrite};
use std::io::Cursor;

#[derive(BinRead, BinWrite)]
#[brw(little)]
struct ArrayHeader {
    element_kind: u32,
    count: u8,
    unknown: [u8; 3],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct ListHeader {
    active_len: u8,
    capacity: u8,
    unknown: [u8; 2],
}

#[derive(Default)]
pub struct DesignerValueCodec {
    pub kind: u32,
    /// Relocated action operands, keyed by byte offset within this value.
    pub actions: std::collections::BTreeMap<usize, Option<StateRef>>,
    /// Output handles assigned by the OpenSpace writer, never persisted in Rete.
    pub action_handles: Vec<(Option<StateRef>, u32)>,
    pub actors: std::collections::BTreeMap<
        usize,
        Option<crate::model::AssetRef<crate::model::character::Perso>>,
    >,
    pub actor_handles: Vec<(
        Option<crate::model::AssetRef<crate::model::character::Perso>>,
        u32,
    )>,
    pub source_actors: std::collections::BTreeMap<usize, Option<SourceObjectIdentity>>,
    /// Resolved waypoint assets, keyed by byte offset in the value. A present
    /// `None` is a real null source pointer; an absent entry stays unresolved.
    pub waypoints: std::collections::BTreeMap<
        usize,
        Option<crate::model::AssetRef<crate::model::route::Waypoint>>,
    >,
    pub waypoint_handles: Vec<(
        Option<crate::model::AssetRef<crate::model::route::Waypoint>>,
        u32,
    )>,
    /// Resolved source waypoint references, keyed by byte offset in the value.
    pub source_objects: std::collections::BTreeMap<usize, Option<SourceObjectIdentity>>,
}
impl DesignerValueCodec {
    pub fn supports(kind: u32) -> bool {
        matches!(kind, 0..=8 | 10..=12 | 14 | 20..=23 | 25)
    }
    pub fn kind(value: &DesignerValue) -> u32 {
        match value {
            DesignerValue::Actor(_) => 10,
            DesignerValue::SourceActor(_) => 10,
            DesignerValue::Boolean(_) => 0,
            DesignerValue::Byte(_) => 1,
            DesignerValue::UnsignedByte(_) => 2,
            DesignerValue::Short(_) => 3,
            DesignerValue::UnsignedShort(_) => 4,
            DesignerValue::Integer(_) => 5,
            DesignerValue::UnsignedInteger(_) => 6,
            DesignerValue::Real(_) => 7,
            DesignerValue::Vector(_) => 12,
            DesignerValue::ActorList(_) => 11,
            DesignerValue::VectorArray(_) => 20,
            DesignerValue::RealArray(_) => 21,
            DesignerValue::IntegerArray(_) => 22,
            DesignerValue::Action(_) => 14,
            DesignerValue::ActionArray(_) => 25,
            DesignerValue::SourceWaypoint(_) => 8,
            DesignerValue::SourceWaypointArray(_) => 23,
            DesignerValue::Waypoint(_) => 8,
            DesignerValue::WaypointArray(_) => 23,
            DesignerValue::Opaque { kind, .. } => *kind,
        }
    }
    /// Extent of one value, derived from its kind and array count, never cached.
    pub fn extent(&self, bytes: &[u8]) -> Result<usize> {
        Ok(match self.kind {
            0..=2 => 1,
            3..=4 => 2,
            5..=8 | 10 | 14 => 4,
            11 => 4 + 4 * usize::from(ListHeader::read(&mut Cursor::new(bytes))?.capacity),
            12 => 12,
            20..=23 | 25 => 8 + 12 * usize::from(ArrayHeader::read(&mut Cursor::new(bytes))?.count),
            _ => bail!("unsupported designer type {}", self.kind),
        })
    }
}
impl FormatCodec for DesignerValueCodec {
    type Value = DesignerValue;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "Hype designer value",
            reads: true,
            writes: true,
            notes: "Exact scalars, vectors, resolved actor/waypoint links and Montreal arrays; unresolved source identities are retained but cannot yet be reconstructed",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<DesignerValue> {
        ensure!(
            bytes.len() == self.extent(bytes)?,
            "designer value extent differs"
        );
        let mut c = Cursor::new(bytes);
        let v = match self.kind {
            0 => {
                let b = u8::read_le(&mut c)?;
                ensure!(b <= 1, "noncanonical Boolean designer value");
                DesignerValue::Boolean(b != 0)
            }
            1 => DesignerValue::Byte(i8::read_le(&mut c)?),
            2 => DesignerValue::UnsignedByte(u8::read_le(&mut c)?),
            3 => DesignerValue::Short(i16::read_le(&mut c)?),
            4 => DesignerValue::UnsignedShort(u16::read_le(&mut c)?),
            5 => DesignerValue::Integer(i32::read_le(&mut c)?),
            6 => DesignerValue::UnsignedInteger(u32::read_le(&mut c)?),
            7 => DesignerValue::Real(f32::read_le(&mut c)?),
            11 => {
                let header = ListHeader::read(&mut c)?;
                ensure!(
                    header.active_len <= header.capacity,
                    "designer list length exceeds capacity"
                );
                let mut slots = Vec::with_capacity(usize::from(header.capacity));
                for index in 0..header.capacity {
                    let raw = u32::read_le(&mut c)?;
                    let offset = 4 + usize::from(index) * 4;
                    let entry = if raw == 0 {
                        None
                    } else if let Some(Some(actor)) = self.actors.get(&offset) {
                        Some(DesignerActorListEntry::Actor(actor.clone()))
                    } else if let Some(Some(source)) = self.source_actors.get(&offset) {
                        Some(DesignerActorListEntry::Source(source.clone()))
                    } else {
                        bail!("unbound designer list actor at +{offset}")
                    };
                    slots.push(entry);
                }
                DesignerValue::ActorList(DesignerActorList {
                    active_len: header.active_len,
                    header_unknown: header.unknown,
                    slots,
                })
            }
            12 => DesignerValue::Vector(<[f32; 3]>::read_le(&mut c)?),
            20 => DesignerValue::VectorArray(read_array(&mut c, 12, false)?),
            21 => DesignerValue::RealArray(read_array(&mut c, 7, true)?),
            22 => DesignerValue::IntegerArray(read_array(&mut c, 5, true)?),
            10 => {
                if let Some(actor) = self.actors.get(&0) {
                    DesignerValue::Actor(actor.clone())
                } else {
                    DesignerValue::SourceActor(
                        self.source_actors
                            .get(&0)
                            .cloned()
                            .context("unbound designer actor")?,
                    )
                }
            }
            14 => DesignerValue::Action(self.action(0)?),
            8 => match self.waypoints.get(&0) {
                Some(reference) => DesignerValue::Waypoint(reference.clone()),
                None => DesignerValue::SourceWaypoint(self.source_object(0)?),
            },
            25 => {
                let raw: DesignerArray<u32> = read_array(&mut c, 14, true)?;
                DesignerValue::ActionArray(DesignerArray {
                    values: (0..raw.values.len())
                        .map(|i| self.action(8 + i * 12))
                        .collect::<Result<_>>()?,
                    unknown: raw.unknown,
                    element_unknown: raw.element_unknown,
                })
            }
            23 => {
                let raw: DesignerArray<u32> = read_array(&mut c, 8, true)?;
                let mut values = Vec::with_capacity(raw.values.len());
                let mut resolved = true;
                for index in 0..raw.values.len() {
                    let offset = 8 + index * 12;
                    match self.waypoints.get(&offset) {
                        Some(reference) => values.push(reference.clone()),
                        None => resolved = false,
                    }
                }
                if resolved {
                    DesignerValue::WaypointArray(DesignerArray {
                        values,
                        unknown: raw.unknown,
                        element_unknown: raw.element_unknown,
                    })
                } else {
                    DesignerValue::SourceWaypointArray(DesignerArray {
                        values: (0..raw.values.len())
                            .map(|i| self.source_object(8 + i * 12))
                            .collect::<Result<_>>()?,
                        unknown: raw.unknown,
                        element_unknown: raw.element_unknown,
                    })
                }
            }
            _ => bail!("unsupported designer type {}", self.kind),
        };
        v.validate()?;
        Ok(v)
    }
    fn write(&self, value: &DesignerValue) -> Result<Vec<u8>> {
        value.validate()?;
        ensure!(
            self.kind == Self::kind(value),
            "designer value kind differs"
        );
        let mut c = Cursor::new(Vec::new());
        match value {
            DesignerValue::Boolean(v) => u8::from(*v).write_le(&mut c)?,
            DesignerValue::Byte(v) => v.write_le(&mut c)?,
            DesignerValue::UnsignedByte(v) => v.write_le(&mut c)?,
            DesignerValue::Short(v) => v.write_le(&mut c)?,
            DesignerValue::UnsignedShort(v) => v.write_le(&mut c)?,
            DesignerValue::Integer(v) => v.write_le(&mut c)?,
            DesignerValue::UnsignedInteger(v) => v.write_le(&mut c)?,
            DesignerValue::Real(v) => v.write_le(&mut c)?,
            DesignerValue::Vector(v) => v.write_le(&mut c)?,
            DesignerValue::ActorList(v) => {
                ListHeader {
                    active_len: v.active_len,
                    capacity: v.slots.len().try_into()?,
                    unknown: v.header_unknown,
                }
                .write(&mut c)?;
                for slot in &v.slots {
                    let handle = match slot {
                        None => 0,
                        Some(DesignerActorListEntry::Actor(actor)) => self
                            .actor_handles
                            .iter()
                            .find(|(reference, _)| reference.as_ref() == Some(actor))
                            .map(|(_, handle)| *handle)
                            .context("missing assigned designer list actor handle")?,
                        Some(DesignerActorListEntry::Source(_)) => {
                            bail!("external source list actor reconstruction is not implemented")
                        }
                    };
                    handle.write_le(&mut c)?;
                }
            }
            DesignerValue::VectorArray(v) => write_array(&mut c, v, 12, false)?,
            DesignerValue::RealArray(v) => write_array(&mut c, v, 7, true)?,
            DesignerValue::IntegerArray(v) => write_array(&mut c, v, 5, true)?,
            DesignerValue::Actor(v) => self
                .actor_handles
                .iter()
                .find(|(r, _)| r == v)
                .map(|(_, h)| *h)
                .context("missing assigned designer actor handle")?
                .write_le(&mut c)?,
            DesignerValue::SourceActor(_) => {
                bail!("external source actor reconstruction is not implemented")
            }
            DesignerValue::Action(v) => self.action_handle(v)?.write_le(&mut c)?,
            DesignerValue::ActionArray(v) => write_array(
                &mut c,
                &DesignerArray {
                    values: v
                        .values
                        .iter()
                        .map(|v| self.action_handle(v))
                        .collect::<Result<Vec<_>>>()?,
                    unknown: v.unknown,
                    element_unknown: v.element_unknown.clone(),
                },
                14,
                true,
            )?,
            DesignerValue::SourceWaypoint(_) | DesignerValue::SourceWaypointArray(_) => {
                bail!("source waypoint reconstruction is not implemented")
            }
            DesignerValue::Waypoint(value) => self.waypoint_handle(value)?.write_le(&mut c)?,
            DesignerValue::WaypointArray(value) => write_array(
                &mut c,
                &DesignerArray {
                    values: value
                        .values
                        .iter()
                        .map(|value| self.waypoint_handle(value))
                        .collect::<Result<Vec<_>>>()?,
                    unknown: value.unknown,
                    element_unknown: value.element_unknown.clone(),
                },
                8,
                true,
            )?,
            DesignerValue::Opaque { kind, .. } => {
                bail!("opaque source designer type {kind} cannot be reconstructed")
            }
        }
        Ok(c.into_inner())
    }
}
impl DesignerValueCodec {
    fn action(&self, offset: usize) -> Result<Option<StateRef>> {
        self.actions
            .get(&offset)
            .cloned()
            .with_context(|| format!("unbound designer action at +{offset}"))
    }
    fn action_handle(&self, value: &Option<StateRef>) -> Result<u32> {
        self.action_handles
            .iter()
            .find(|(r, _)| r == value)
            .map(|(_, h)| *h)
            .context("missing assigned designer action handle")
    }
    fn source_object(&self, offset: usize) -> Result<Option<SourceObjectIdentity>> {
        self.source_objects
            .get(&offset)
            .cloned()
            .with_context(|| format!("unbound source waypoint at +{offset}"))
    }
    fn waypoint_handle(
        &self,
        value: &Option<crate::model::AssetRef<crate::model::route::Waypoint>>,
    ) -> Result<u32> {
        self.waypoint_handles
            .iter()
            .find(|(reference, _)| reference == value)
            .map(|(_, handle)| *handle)
            .context("missing assigned designer waypoint handle")
    }
}
fn read_array<T>(c: &mut Cursor<&[u8]>, kind: u32, scalar: bool) -> Result<DesignerArray<T>>
where
    T: for<'a> BinRead<Args<'a> = ()>,
{
    let h = ArrayHeader::read(c)?;
    ensure!(
        h.element_kind == kind,
        "designer array element kind differs"
    );
    let mut result = DesignerArray::new(Vec::with_capacity(usize::from(h.count)));
    result.unknown = h.unknown;
    for index in 0..h.count {
        result.values.push(T::read_le(c)?);
        if scalar {
            let bytes = <[u8; 8]>::read_le(c)?;
            if bytes != [0; 8] {
                result.element_unknown.insert(index, bytes);
            }
        }
    }
    Ok(result)
}
fn write_array<T>(
    c: &mut Cursor<Vec<u8>>,
    array: &DesignerArray<T>,
    kind: u32,
    scalar: bool,
) -> Result<()>
where
    T: for<'a> BinWrite<Args<'a> = ()>,
{
    ArrayHeader {
        element_kind: kind,
        count: array.values.len().try_into()?,
        unknown: array.unknown,
    }
    .write(c)?;
    for (index, value) in array.values.iter().enumerate() {
        value.write_le(c)?;
        if scalar {
            array
                .element_unknown
                .get(&(index as u8))
                .unwrap_or(&[0; 8])
                .write_le(c)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod list_tests {
    use super::*;
    use crate::model::AssetRef;

    #[test]
    fn source_perso_list_round_trips_empty_and_edited_entries() -> Result<()> {
        let mut codec = DesignerValueCodec {
            kind: 11,
            ..Default::default()
        };
        let empty = [0u8, 1, 0, 0, 0, 0, 0, 0];
        let mut value = codec.read(&empty)?;
        assert_eq!(codec.write(&value)?, empty);
        let actor = AssetRef::new("level:/persos/world/0/2")?;
        let DesignerValue::ActorList(list) = &mut value else {
            panic!("type 11 did not decode as a list")
        };
        list.active_len = 1;
        list.slots[0] = Some(DesignerActorListEntry::Actor(actor.clone()));
        codec.actor_handles.push((Some(actor.clone()), 0x1234));
        let encoded = codec.write(&value)?;
        assert_eq!(encoded, [1, 1, 0, 0, 0x34, 0x12, 0, 0]);
        codec.actors.insert(4, Some(actor));
        assert_eq!(codec.read(&encoded)?, value);
        Ok(())
    }
}
