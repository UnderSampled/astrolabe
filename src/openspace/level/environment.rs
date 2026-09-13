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
                visual,
                unresolved_mechanics: h.mechanics != 0,
                unresolved_sound: h.sound != 0,
            });
        }
        Ok(AssetRef::from_path(self.environments[&a].path.clone()))
    }
    pub(super) fn finish(self, out: &mut SceneImport) {
        for v in self.environments.values() {
            if v.unresolved_mechanics || v.unresolved_sound {
                out.diagnostics.push(Diagnostic {code:"openspace.environment.coverage".into(), asset:Some(v.path.clone()), severity:Severity::Warning,
                    message:format!("Environment components requiring codecs: mechanics={}, sound={}; visual/fog links are resolved",v.unresolved_mechanics,v.unresolved_sound)});
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
        out.visual_environments = self.visuals.into_values().collect();
        out.fogs = self.fogs.into_values().collect();
    }
}

#[cfg(test)]
pub(super) fn check(s: &SourceSession, imported: &SceneImport) -> Result<()> {
    assert_eq!(
        (
            imported.environments.len(),
            imported.visual_environments.len(),
            imported.fogs.len()
        ),
        (1, 1, 1)
    );
    let env = &imported.environments[0];
    let visual = &imported.visual_environments[0];
    let fog = &imported.fogs[0];
    assert!(env.unresolved_mechanics && !env.unresolved_sound && !visual.unresolved_water);
    assert_eq!(env.visual.as_ref().unwrap().path, visual.path);
    assert_eq!(visual.fog.as_ref().unwrap().path, fog.path);
    assert_eq!(
        (fog.near, fog.far, fog.infinite, fog.color, fog.unknown),
        (12.1, 3096., 5000., [0.744094; 3], 0)
    );
    for sector in imported.scene.nodes.iter().flat_map(|n| &n.sector) {
        assert_eq!(
            sector.environments,
            vec![AssetRef::from_path(env.path.clone())]
        );
    }
    let node = s
        .tree()?
        .into_iter()
        .find(|n| n.path == "world/1/0")
        .unwrap();
    let sector = s.required(node.address, 4)?;
    let e = s.required(super::sector::entries(s, sector, 4, 4)?[0], 0)?;
    let v = s.required(e, 8)?;
    let f = s.required(v, 4)?;
    assert_eq!(write_fog(fog)?, s.bytes(f, 28)?);
    let normalized = super::write_environments(
        &imported.environments,
        &imported.visual_environments,
        &imported.fogs,
        &super::EnvironmentDependencies {
            mechanics: BTreeMap::from([(
                env.path.clone(),
                super::ReferenceTarget {
                    address: 0x4000,
                    block: (6, 2),
                },
            )]),
            ..Default::default()
        },
        0x10000,
    )?;
    assert_eq!((normalized.ranges.len(), normalized.pointer_count), (3, 3));
    assert_eq!(&normalized.bytes[20..], s.bytes(f, 28)?);
    for (source, range, pointers) in [
        (e, &normalized.ranges[0], vec![0usize, 8]),
        (v, &normalized.ranges[1], vec![4]),
    ] {
        let mut expected = s.bytes(source, range.length)?.to_vec();
        let mut actual = normalized.bytes[range.offset..range.offset + range.length].to_vec();
        for field in pointers {
            expected[field..field + 4].fill(0);
            actual[field..field + 4].fill(0);
        }
        assert_eq!(actual, expected);
    }
    eprintln!(
        "Environment inventory: 17 sectors share one ENV, one VET and one 28-byte fog; fog byte parity and normalized ENV/VET pointers verified"
    );
    Ok(())
}
