//! ENV/VET accessors and constructors: 005a8f40, 005a9290, 0050c730,
//! 0050c200..0050c290 and 0050c2a0. Scene drawing uses these in 004df0e0.
use super::{
    container::{encoded, wire},
    records::*,
    scene::SceneImport,
    session::SourceSession,
};
use crate::model::{
    AssetPath, AssetRef,
    environment::*,
    io::{Content, Diagnostic, Severity},
};
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite};
use std::collections::BTreeMap;

#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct EnvironmentWire {
    pub mechanics: u32,
    pub sound: u32,
    pub visual: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct VisualEnvironmentWire {
    pub water: u32,
    pub fog: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct FogWire {
    near: f32,
    far: f32,
    infinite: f32,
    color: [f32; 3],
    unknown: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct PhysicalEnvironmentWire {
    gravity: f32,
    vertical: [f32; 3],
    retained: [u8; 32],
}

pub fn read_physical_environment(bytes: &[u8], path: AssetPath) -> Result<PhysicalEnvironment> {
    ensure!(bytes.len() == 48, "physical environment must have 48 bytes");
    let h: PhysicalEnvironmentWire = wire(bytes)?;
    let v = PhysicalEnvironment {
        path,
        gravity: h.gravity,
        vertical: h.vertical,
        retained: h.retained,
    };
    v.validate()?;
    Ok(v)
}
pub fn write_physical_environment(v: &PhysicalEnvironment) -> Result<Vec<u8>> {
    v.validate()?;
    encoded(&PhysicalEnvironmentWire {
        gravity: v.gravity,
        vertical: v.vertical,
        retained: v.retained,
    })
}

pub fn read_fog(bytes: &[u8], path: AssetPath) -> Result<Fog> {
    let h: FogWire = wire(bytes)?;
    let v = Fog {
        path,
        near: h.near,
        far: h.far,
        infinite: h.infinite,
        color: h.color,
        unknown: h.unknown,
    };
    v.validate()?;
    Ok(v)
}
pub fn write_fog(v: &Fog) -> Result<Vec<u8>> {
    v.validate()?;
    encoded(&FogWire {
        near: v.near,
        far: v.far,
        infinite: v.infinite,
        color: v.color,
        unknown: v.unknown,
    })
}
fn path(s: &SourceSession, a: Address, kind: &str) -> Result<AssetPath> {
    AssetPath::new(format!(
        "{}:/{kind}/{}",
        if a.owner == Owner::Fix {
            "fix"
        } else {
            "level"
        },
        s.allocation_index(a)?
    ))
}

#[derive(Default)]
pub(super) struct Import {
    environments: BTreeMap<Address, Environment>,
    physicals: BTreeMap<Address, PhysicalEnvironment>,
    visuals: BTreeMap<Address, VisualEnvironment>,
    fogs: BTreeMap<Address, Fog>,
}
impl Import {
    pub(super) fn environment(
        &mut self,
        s: &SourceSession,
        a: Address,
    ) -> Result<AssetRef<Environment>> {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.environments.entry(a) {
            let h: EnvironmentWire = wire(s.bytes(a, 12)?)?;
            ensure!(
                (h.mechanics != 0) == s.pointer(a, 0)?.is_some(),
                "physical environment relocation mismatch"
            );
            let physical = if let Some(p) = s.pointer(a, 0)? {
                if let std::collections::btree_map::Entry::Vacant(entry) = self.physicals.entry(p) {
                    entry.insert(read_physical_environment(
                        s.bytes(p, 48)?,
                        path(s, p, "physical-environments")?,
                    )?);
                }
                Some(AssetRef::from_path(self.physicals[&p].path.clone()))
            } else {
                None
            };
            ensure!(
                (h.visual != 0) == s.pointer(a, 8)?.is_some(),
                "visual environment relocation mismatch"
            );
            let visual = if let Some(v) = s.pointer(a, 8)? {
                if let std::collections::btree_map::Entry::Vacant(entry) = self.visuals.entry(v) {
                    let h: VisualEnvironmentWire = wire(s.bytes(v, 8)?)?;
                    ensure!(
                        (h.fog != 0) == s.pointer(v, 4)?.is_some(),
                        "fog relocation mismatch"
                    );
                    let fog = if let Some(f) = s.pointer(v, 4)? {
                        if let std::collections::btree_map::Entry::Vacant(entry) =
                            self.fogs.entry(f)
                        {
                            entry.insert(read_fog(s.bytes(f, 28)?, path(s, f, "fogs")?)?);
                        }
                        Some(AssetRef::from_path(self.fogs[&f].path.clone()))
                    } else {
                        None
                    };
                    entry.insert(VisualEnvironment {
                        path: path(s, v, "visual-environments")?,
                        fog,
                        unresolved_water: h.water != 0,
                    });
                }
                Some(AssetRef::from_path(self.visuals[&v].path.clone()))
            } else {
                None
            };
            entry.insert(Environment {
                path: path(s, a, "environments")?,
                physical,
                visual,
                unresolved_sound: h.sound != 0,
            });
        }
        Ok(AssetRef::from_path(self.environments[&a].path.clone()))
    }
    pub(super) fn finish(self, out: &mut SceneImport) {
        for v in self.environments.values() {
            if v.unresolved_sound {
                out.diagnostics.push(Diagnostic {code:"openspace.environment.coverage".into(), asset:Some(v.path.clone()), severity:Severity::Warning,
                    message:"Environment sound component requires a codec; physical/visual/fog links are resolved".into()});
            }
        }
        for v in self.visuals.values() {
            if v.unresolved_water {
                out.diagnostics.push(Diagnostic {
                    code: "openspace.environment.water".into(),
                    asset: Some(v.path.clone()),
                    severity: Severity::Warning,
                    message: "Water effect requires a codec; fog link is resolved".into(),
                });
            }
        }
        out.environments = self.environments.into_values().collect();
        out.physical_environments = self.physicals.into_values().collect();
        out.visual_environments = self.visuals.into_values().collect();
        out.fogs = self.fogs.into_values().collect();
    }
}

#[cfg(test)]
pub(super) fn check(s: &SourceSession, imported: &SceneImport) -> Result<()> {
    assert!(!imported.environments.is_empty());
    assert_eq!(
        imported.physical_environments.len(),
        imported.environments.len()
    );
    for sector in imported.scene.nodes.iter().flat_map(|n| &n.sector) {
        for reference in &sector.environments {
            assert!(
                imported
                    .environments
                    .iter()
                    .any(|env| env.path == reference.path)
            );
        }
    }
    let node = s
        .tree()?
        .into_iter()
        .find(|n| n.path == "world/1/0")
        .unwrap();
    let sector = s.required(node.address, 4)?;
    let e = s.required(super::sector::entries(s, sector, 4, 4)?[0], 0)?;
    let p = s.required(e, 0)?;
    let env = imported
        .environments
        .iter()
        .find(|v| v.path == path(s, e, "environments").unwrap())
        .unwrap();
    let physical = imported
        .physical_environments
        .iter()
        .find(|v| v.path == path(s, p, "physical-environments").unwrap())
        .unwrap();
    assert_eq!(env.physical.as_ref().unwrap().path, physical.path);
    assert_eq!(write_physical_environment(physical)?, s.bytes(p, 48)?);
    let v = s.required(e, 8)?;
    let f = s.required(v, 4)?;
    let visual = imported
        .visual_environments
        .iter()
        .find(|candidate| candidate.path == path(s, v, "visual-environments").unwrap())
        .unwrap();
    let fog = imported
        .fogs
        .iter()
        .find(|candidate| candidate.path == path(s, f, "fogs").unwrap())
        .unwrap();
    assert_eq!(env.visual.as_ref().unwrap().path, visual.path);
    assert_eq!(visual.fog.as_ref().unwrap().path, fog.path);
    assert_eq!(write_fog(fog)?, s.bytes(f, 28)?);
    let normalized = super::write_environments(
        &imported.environments,
        &imported.physical_environments,
        &imported.visual_environments,
        &imported.fogs,
        &super::EnvironmentDependencies::default(),
        0x10000,
    )?;
    assert_eq!(
        normalized.ranges.len(),
        imported.environments.len()
            + imported.physical_environments.len()
            + imported.visual_environments.len()
            + imported.fogs.len()
    );
    for (source, asset_path, pointers) in
        [(e, &env.path, vec![0usize, 8]), (v, &visual.path, vec![4])]
    {
        let range = normalized
            .ranges
            .iter()
            .find(|r| r.name == asset_path.as_str())
            .unwrap();
        let mut expected = s.bytes(source, range.length)?.to_vec();
        let mut actual = normalized.bytes[range.offset..range.offset + range.length].to_vec();
        for field in pointers {
            expected[field..field + 4].fill(0);
            actual[field..field + 4].fill(0);
        }
        assert_eq!(actual, expected);
    }
    eprintln!(
        "Environment inventory: {} sectors, {} ENV and {} physical records; physical and fog byte parity verified",
        imported
            .scene
            .nodes
            .iter()
            .filter(|n| n.sector.is_some())
            .count(),
        imported.environments.len(),
        imported.physical_environments.len()
    );
    Ok(())
}
