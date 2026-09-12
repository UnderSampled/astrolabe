//! Hype standalone sound codecs. The canonical model contains decoded useful values;
//! these codecs never retain or replay an encoded source stream.
mod apm;
mod definitions;
pub use definitions::{read_sound_definitions, write_sound_definitions};
mod bnm;
mod mpeg;
pub use apm::{read_apm, write_apm};
pub use bnm::{read_bnm, write_bnm};
pub use mpeg::read_ubi_mpeg;

use crate::model::audio::Sound;
use anyhow::{Result, ensure};
pub fn validate_sound(sound: &Sound) -> Result<()> {
    sound.validate()?;
    ensure!(
        (1..=2).contains(&sound.channels),
        "OpenSpace media codec requires one or two primary channels"
    );
    Ok(())
}

pub use apm::Apm;
pub use bnm::Bnm;
pub use definitions::SoundScript;
