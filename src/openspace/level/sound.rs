//! Session-local SND/RTS binding. Runtime sound handles never enter useful assets.
use super::{
    container::{file, pointer_file, wire},
    records::*,
    session::SourceSession,
};
use crate::{
    model::{
        AssetPath, AssetRef, Selection,
        audio::{SoundDefinitions, SoundEventLink},
    },
    openspace::codec::{FormatCodec, SoundScript},
};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, path::Path};

pub(super) struct SoundSites {
    pub events: BTreeMap<Address, Option<SoundEventLink>>,
    pub library: (AssetPath, SoundDefinitions),
}
impl SoundSites {
    pub fn open(game: &Path, selection: &Selection, session: &SourceSession) -> Result<Self> {
        ensure!(
            selection.locale == "en",
            "scene sound bindings currently require English source data"
        );
        let library_path = AssetPath::new("sound:/Sound/playmo_b.lcb")?;
        let library = SoundScript {
            name: "playmo_b.lcb".into(),
        }
        .read(&file(&game.join(library_path.file()))?)?;
        let language = game.join("LangData/English/world/levels");
        let mut events = BTreeMap::new();
        for (owner, prefix) in [
            (Owner::Fix, language.join("Fix")),
            (
                Owner::Level,
                language.join(&selection.level).join(&selection.level),
            ),
        ] {
            let sites = file(&prefix.with_extension("snd"))?;
            ensure!(
                sites.len().is_multiple_of(8),
                "SND has a partial event/site record"
            );
            let rts = pointer_file(&file(&prefix.with_extension("rts"))?)?;
            for record in sites.as_chunks::<8>().0 {
                let entry: SoundSiteWire = wire(record)?;
                let key = *rts
                    .get(&entry.site)
                    .context("SND site lacks RTS relocation")?;
                // Promoted animation descriptors and AI node operands consume these
                // bindings; unrelated domains are not traversed here.
                if !matches!(key.0, 5 | 17 | 20)
                    || (key.0 == 20
                        && !session
                            .blocks
                            .get(&owner)
                            .and_then(|b| b.get(&key))
                            .is_some_and(|b| !b.data.is_empty()))
                {
                    continue;
                }
                let site = session.resolve(owner, key, entry.site)?;
                let event = if entry.event == u32::MAX {
                    None
                } else {
                    let link = SoundEventLink {
                        library: AssetRef::from_path(library_path.clone()),
                        bank: (entry.event >> 16) as u16,
                        member: entry.event as u16,
                    };
                    link.binding(&library)?;
                    Some(link)
                };
                if let Some(old) = events.insert(site, event.clone()) {
                    ensure!(old == event, "conflicting SND events at one action site");
                }
            }
        }
        Ok(Self {
            events,
            library: (library_path, library),
        })
    }
}
