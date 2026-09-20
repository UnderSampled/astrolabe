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
}
impl DesignerValueCodec {
    pub fn supports(kind: u32) -> bool {
        matches!(kind, 0..=7 | 10 | 12 | 14 | 20..=22 | 25)
    }
    pub fn kind(value: &DesignerValue) -> u32 {
        match value {
            DesignerValue::Actor(_) => 10,
            DesignerValue::Boolean(_) => 0,
            DesignerValue::Byte(_) => 1,
            DesignerValue::UnsignedByte(_) => 2,
            DesignerValue::Short(_) => 3,
            DesignerValue::UnsignedShort(_) => 4,
            DesignerValue::Integer(_) => 5,
            DesignerValue::UnsignedInteger(_) => 6,
            DesignerValue::Real(_) => 7,
            DesignerValue::Vector(_) => 12,
            DesignerValue::VectorArray(_) => 20,
            DesignerValue::RealArray(_) => 21,
            DesignerValue::IntegerArray(_) => 22,
            DesignerValue::Action(_) => 14,
            DesignerValue::ActionArray(_) => 25,
        }
    }
    /// Extent of one value, derived from its kind and array count, never cached.
    pub fn extent(&self, bytes: &[u8]) -> Result<usize> {
        Ok(match self.kind {
            0..=2 => 1,
            3..=4 => 2,
            5..=7 | 10 | 14 => 4,
            12 => 12,
            20..=22 | 25 => 8 + 12 * usize::from(ArrayHeader::read(&mut Cursor::new(bytes))?.count),
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
            notes: "Exact scalars, vectors, family-owned actions and Montreal arrays; unknown array fields preserved",
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
            12 => DesignerValue::Vector(<[f32; 3]>::read_le(&mut c)?),
            20 => DesignerValue::VectorArray(read_array(&mut c, 12, false)?),
            21 => DesignerValue::RealArray(read_array(&mut c, 7, true)?),
            22 => DesignerValue::IntegerArray(read_array(&mut c, 5, true)?),
            10 => DesignerValue::Actor(
                self.actors
                    .get(&0)
                    .cloned()
                    .context("unbound designer actor")?,
            ),
            14 => DesignerValue::Action(self.action(0)?),
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
