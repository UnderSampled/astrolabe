//! Typed file-codec contract inside OpenSpace. Layout declarations live in each
//! format module; package composition and the other targets use model::io instead.
use crate::model::{
    audio::{Sound, SoundBank, SoundDefinitions},
    io::FormatSupport,
    texture::{Texture, TextureArchive, TextureUsage},
};
use anyhow::{Result, ensure};

pub trait FormatCodec {
    type Value;
    fn support(&self) -> FormatSupport;
    fn read(&self, bytes: &[u8]) -> Result<Self::Value>;
    fn write(&self, value: &Self::Value) -> Result<Vec<u8>>;
}

pub struct Gf {
    pub usage: TextureUsage,
}
impl FormatCodec for Gf {
    type Value = Texture;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "GF",
            reads: true,
            writes: true,
            notes: "Hype palette/components, mipmaps and exact observed GF reconstruction",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<Texture> {
        super::texture::decode_gf(bytes, self.usage)
    }
    fn write(&self, value: &Texture) -> Result<Vec<u8>> {
        ensure!(
            value.usage == self.usage,
            "GF usage differs from codec context"
        );
        super::texture::encode_gf(value)
    }
}
pub struct Cnt {
    pub usage: TextureUsage,
}
impl FormatCodec for Cnt {
    type Value = TextureArchive;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "CNT",
            reads: true,
            writes: true,
            notes: "Texture archives; membership/order/sharing retained, regenerated output is unencrypted",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<TextureArchive> {
        super::texture::decode_cnt(bytes, self.usage)
    }
    fn write(&self, value: &TextureArchive) -> Result<Vec<u8>> {
        ensure!(
            value.textures.iter().all(|t| t.texture.usage == self.usage),
            "CNT texture usage differs from codec context"
        );
        super::texture::encode_cnt(value)
    }
}
pub struct Apm {
    pub name: String,
}
impl FormatCodec for Apm {
    type Value = Sound;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "APM",
            reads: true,
            writes: true,
            notes: "PCM model; derived IMA ADPCM export is lossy, historical encoded bytes are unverified",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<Sound> {
        super::audio::read_apm(bytes, self.name.clone())
    }
    fn write(&self, value: &Sound) -> Result<Vec<u8>> {
        super::audio::write_apm(value)
    }
}
pub struct Bnm {
    pub name: String,
}
impl FormatCodec for Bnm {
    type Value = SoundBank;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "BNM",
            reads: true,
            writes: true,
            notes: "PCM/APM/Ubi-MPEG import; writer supports PCM-only banks without events/external references; unsupported values fail explicitly",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<SoundBank> {
        super::audio::read_bnm(bytes, self.name.clone())
    }
    fn write(&self, value: &SoundBank) -> Result<Vec<u8>> {
        super::audio::write_bnm(value)
    }
}
pub struct SoundScript {
    pub name: String,
}
impl FormatCodec for SoundScript {
    type Value = SoundDefinitions;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "CSB/LCB/SIF",
            reads: true,
            writes: true,
            notes: "Latin-1 text import; semantic LCB/SIF output, CSB event behavior export remains unsupported",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<SoundDefinitions> {
        let text = bytes.iter().map(|&b| char::from(b)).collect::<String>();
        super::audio::read_sound_definitions(&text, self.name.clone())
    }
    fn write(&self, value: &SoundDefinitions) -> Result<Vec<u8>> {
        let text = super::audio::write_sound_definitions(value)?;
        text.chars()
            .map(|c| {
                ensure!(
                    (c as u32) <= 255,
                    "sound script character {c:?} is outside source Latin-1 encoding"
                );
                Ok(c as u8)
            })
            .collect()
    }
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
            notes: "Rooted first-model import; controlled model fragment and framing primitives only, not complete level reconstruction",
        },
    ]
}
