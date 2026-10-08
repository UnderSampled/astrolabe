//! Per-instance activation and reusable collision profiles. Hype clones the
//! instance header while sharing purpose-specific geometry and activation lists.
use super::{character::Importer, container::wire, records::*};
use crate::model::{AssetRef, character::*, io::Content};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Profiles {
    profiles: BTreeMap<[Option<Address>; 2], usize>,
    selections: BTreeMap<(Address, usize), u32>,
}
impl Importer<'_> {
    pub(super) fn collisions(&mut self, address: Address) -> Result<CharacterCollisions> {
        let header: CharacterCollisionsWire = wire(self.session.bytes(address, 52)?)?;
        let mut result = CharacterCollisions {
            groups: Default::default(),
            unknown: header.unknown,
        };
        for group in 0..4 {
            let geometry = self.session.pointer(address, group * 4)?;
            let activations = self.session.pointer(address, 16 + group * 4)?;
            // 0045fb50/0045fbb0: active ZDM/ZDR fields are swapped.
            let active = self.session.pointer(address, [32, 36, 44, 40][group])?;
            if geometry.is_none() && activations.is_none() {
                ensure!(
                    active.is_none(),
                    "active collision list without profile definitions"
                );
                continue;
            }
            let key = [geometry, activations];
            let profile = if let Some(&index) = self.collision_profiles.profiles.get(&key) {
                index
            } else {
                let origin = geometry.or(activations).unwrap();
                // A profile can combine shared Fix geometry with Level-owned
                // activation/selection data. Keep that bound composite in the
                // active Level while its referenced collision geometry keeps
                // its original Fix identity.
                let identity = key
                    .into_iter()
                    .flatten()
                    .find(|part| part.owner == Owner::Level)
                    .unwrap_or(origin);
                let path = self.path(identity, "collision-profiles")?;
                let index = self.result.collision_profiles.len();
                self.result.collision_profiles.push(CollisionProfile {
                    path,
                    geometry: None,
                    activations: None,
                    selections: Vec::new(),
                });
                self.collision_profiles.profiles.insert(key, index);
                if let Some(list) = geometry {
                    let header: CollisionSlotsWire = wire(self.session.bytes(list, 16)?)?;
                    let mut slots = Vec::new();
                    for entry in self.list(list, 0)? {
                        let _: CollisionGeometryLinkWire = wire(self.session.bytes(entry, 16)?)?;
                        slots.push(
                            self.session
                                .pointer(entry, 12)?
                                .map(|a| self.bounding_geometry(a, "character-zones"))
                                .transpose()?,
                        );
                    }
                    self.result.collision_profiles[index].geometry = Some(CollisionGeometrySlots {
                        slots,
                        unknown: header.unknown,
                    });
                }
                if let Some(list) = activations {
                    let mut values = Vec::new();
                    for (state, entry) in self.list(list, 0)?.into_iter().enumerate() {
                        let header: CollisionActivationWire = wire(self.session.bytes(entry, 20)?)?;
                        ensure!(
                            usize::from(header.state) == state,
                            "collision activation state differs from list position"
                        );
                        let selection = self
                            .session
                            .pointer(entry, 12)?
                            .map(|a| self.collision_selection(a, index))
                            .transpose()?;
                        values.push(CollisionActivation {
                            state: header.state,
                            selection,
                            unknown: header.unknown,
                        });
                    }
                    self.result.collision_profiles[index].activations = Some(values);
                }
                index
            };
            let active = active
                .map(|a| self.collision_selection(a, profile))
                .transpose()?;
            let value = &self.result.collision_profiles[profile];
            value.validate()?;
            result.groups[group] = CharacterCollisionGroup {
                profile: Some(AssetRef::from_path(value.path.clone())),
                active,
            };
        }
        result.validate()?;
        Ok(result)
    }
    fn collision_selection(&mut self, address: Address, profile: usize) -> Result<u32> {
        let key = (address, profile);
        if let Some(&id) = self.collision_profiles.selections.get(&key) {
            return Ok(id);
        }
        let header: CollisionSlotsWire = wire(self.session.bytes(address, 16)?)?;
        let mut zones = Vec::new();
        for entry in self.list(address, 0)? {
            let value: CollisionZoneWire = wire(self.session.bytes(entry, 16)?)?;
            zones.push(CollisionZone {
                index: value.index,
                unknown: value.unknown,
            });
        }
        let selections = &mut self.result.collision_profiles[profile].selections;
        let id = selections.len().try_into()?;
        selections.push(CollisionSelection {
            id,
            zones,
            unknown: header.unknown,
        });
        self.collision_profiles.selections.insert(key, id);
        Ok(id)
    }
}
