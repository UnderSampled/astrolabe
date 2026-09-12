//! Typed bounded content codec contract, private to OpenSpace's encoding concerns.
pub use super::{
    audio::{Apm, Bnm, SoundScript},
    texture::{Cnt, Gf},
};
use crate::model::{io::FormatSupport, texture::TextureUsage};
use anyhow::Result;

pub trait FormatCodec {
    type Value;
    fn support(&self) -> FormatSupport;
    fn read(&self, bytes: &[u8]) -> Result<Self::Value>;
    fn write(&self, value: &Self::Value) -> Result<Vec<u8>>;
}

pub fn catalogue() -> Vec<FormatSupport> {
    vec![
        Gf {
            usage: TextureUsage::Material,
        }
        .support(),
        Cnt {
            usage: TextureUsage::Material,
        }
        .support(),
        Apm {
            name: String::new(),
        }
        .support(),
        Bnm {
            name: String::new(),
        }
        .support(),
        SoundScript {
            name: String::new(),
        }
        .support(),
        FormatSupport {
            format: "SNA/RT/GPT/PTX",
            reads: true,
            writes: true,
            notes: "Rooted first-model import; controlled model fragment and framing primitives only, not complete \
            level reconstruction",
        },
    ]
}
