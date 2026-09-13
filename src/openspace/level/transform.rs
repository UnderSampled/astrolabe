//! POS compact transforms: engine 004b8e50, 004bed70, 004bf250 and 004c0990.
use super::{
    container::{encoded, wire},
    records::TransformWire,
};
use crate::model::{animation::TransformValue, geometry::SourceTransform};
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite};

#[derive(BinRead, BinWrite)]
#[brw(little)]
struct Compact {
    #[br(assert(matches!(flags, 0x11|0x22|0x33|0x77|0x7b|0x7f), "unsupported compact transform flags"))]
    flags: u16,
    #[brw(if(flags & 1 != 0))]
    translation: Option<[i16; 3]>,
    #[brw(if(flags & 2 != 0))]
    quaternion: Option<[i16; 4]>,
    // Engine reads zoom and axial scale as unsigned, symmetric stretch as signed.
    #[br(if(flags == 0x77))]
    #[bw(if(*flags == 0x77))]
    zoom: Option<u16>,
    #[br(if(flags == 0x7b))]
    #[bw(if(*flags == 0x7b))]
    axial: Option<[u16; 3]>,
    #[br(if(flags == 0x7f))]
    #[bw(if(*flags == 0x7f))]
    symmetric: Option<[i16; 6]>,
}
pub fn read_transform(bytes: &[u8]) -> Result<TransformValue> {
    ensure!(bytes.len() >= 2, "short transform");
    let flags = u16::from_le_bytes(bytes[..2].try_into()?);
    let value = if flags & 0x80 != 0 {
        let t: TransformWire = wire(bytes)?;
        TransformValue::Affine(SourceTransform {
            translation: t.translation,
            rotation: t.rotation,
            basis: t.basis,
        })
    } else {
        let c: Compact = wire(bytes)?;
        let stretch = if let Some(s) = c.symmetric {
            s.map(|x| x as f32 / 256.)
        } else if let Some([a, b, c]) = c.axial {
            [
                a as f32 / 256.,
                0.,
                0.,
                b as f32 / 256.,
                0.,
                c as f32 / 256.,
            ]
        } else {
            let x = c.zoom.unwrap_or(256) as f32 / 256.;
            [x, 0., 0., x, 0., x]
        };
        TransformValue::Components {
            translation: c.translation.unwrap_or([0; 3]).map(|x| x as f32 / 512.),
            quaternion: c
                .quaternion
                .unwrap_or([32767, 0, 0, 0])
                .map(|x| x as f32 / 32767.),
            stretch,
        }
    };
    value.validate()?;
    Ok(value)
}
/// Derive a compact form from current values; None means the engine's identity sentinel.
/// Out-of-range/less precise edits use a full affine record, never saturating silently.
pub fn write_transform(value: &TransformValue) -> Result<Option<Vec<u8>>> {
    value.validate()?;
    let TransformValue::Components {
        translation,
        quaternion,
        stretch,
    } = value
    else {
        return full(value).map(Some);
    };
    let Some(t) = quantize::<3>(*translation, 512., i16::MIN as i32, i16::MAX as i32) else {
        return full(value).map(Some);
    };
    let Some(q) = quantize::<4>(*quaternion, 32767., i16::MIN as i32, i16::MAX as i32) else {
        return full(value).map(Some);
    };
    let identity_scale = *stretch == [1., 0., 0., 1., 0., 1.];
    let identity_rotation = q == [32767, 0, 0, 0];
    let identity_translation = t == [0; 3];
    if identity_scale && identity_rotation && identity_translation {
        return Ok(None);
    }
    let mut c = Compact {
        flags: 0x33,
        translation: Some(t.map(|x| x as i16)),
        quaternion: Some(q.map(|x| x as i16)),
        zoom: None,
        axial: None,
        symmetric: None,
    };
    if identity_scale {
        if identity_rotation {
            c.flags = 0x11;
            c.quaternion = None;
        } else if identity_translation {
            c.flags = 0x22;
            c.translation = None;
        }
    } else if stretch[1] == 0.
        && stretch[2] == 0.
        && stretch[4] == 0.
        && stretch[0] >= 0.
        && stretch[3] >= 0.
        && stretch[5] >= 0.
    {
        let Some(s) = quantize(
            [stretch[0], stretch[3], stretch[5]],
            256.,
            0,
            u16::MAX as i32,
        ) else {
            return full(value).map(Some);
        };
        if s[0] == s[1] && s[1] == s[2] {
            c.flags = 0x77;
            c.zoom = Some(s[0] as u16);
        } else {
            c.flags = 0x7b;
            c.axial = Some(s.map(|x| x as u16));
        }
    } else {
        let Some(s) = quantize(*stretch, 256., i16::MIN as i32, i16::MAX as i32) else {
            return full(value).map(Some);
        };
        c.flags = 0x7f;
        c.symmetric = Some(s.map(|x| x as i16));
    }
    encoded(&c).map(Some)
}
fn quantize<const N: usize>(v: [f32; N], scale: f32, min: i32, max: i32) -> Option<[i32; N]> {
    let q = v.map(|x| (x * scale).round() as i32);
    (v.iter()
        .zip(q)
        .all(|(&x, q)| q >= min && q <= max && x == q as f32 / scale))
    .then_some(q)
}
fn full(value: &TransformValue) -> Result<Vec<u8>> {
    let t = value.affine();
    let identity = SourceTransform::default();
    let flags =
        0x80 | if t.translation != identity.translation {
            0x10
        } else {
            0
        } | if t.rotation != identity.rotation {
            0x20
        } else {
            0
        } | if t.basis != t.rotation { 0x40 } else { 0 };
    encoded(&TransformWire {
        flags,
        translation: t.translation,
        rotation: t.rotation,
        basis: t.basis,
    })
}
