//! Hype's GPT supplies active-level placements for Fix-owned persos.
//! The level loader copies each 88-byte transform into graphics+24 and calls
//! FUN_00438e10 to bind StandardGame+12 and graphics+112 to this level's SO.
use super::{
    container::{u32_at, wire},
    records::*,
    session::SourceSession,
};
use crate::model::geometry::SourceTransform;
use anyhow::{Result, ensure};
use binrw::{BinRead, BinWrite};
use std::collections::BTreeMap;

// Hype-PC GPT prefix through engine globals; other profile layouts are separate.
pub(super) const HEADER: usize = 0x2af0;
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct Header {
    #[br(assert(count <= 65536, "excessive Fix perso binding count"))]
    count: u32,
    persos: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct Placement {
    transform: TransformWire,
    node: u32,
}
#[derive(Default)]
pub(super) struct FixBindings {
    pub(super) transforms: BTreeMap<Address, SourceTransform>,
    pub(super) pointers: BTreeMap<Address, Address>,
}
impl SourceSession {
    pub(super) fn bind_fix_persos(&mut self) -> Result<()> {
        let header: Header = wire(
            self.gpt
                .get(HEADER..)
                .ok_or_else(|| anyhow::anyhow!("short Hype GPT binding header"))?,
        )?;
        if header.count == 0 {
            self.fix_bindings = FixBindings::default();
            return Ok(());
        }
        let table = self.root(HEADER + 4)?;
        ensure!(
            table.owner == Owner::Fix,
            "Fix perso table is not Fix-owned"
        );
        let mut bindings = FixBindings::default();
        for i in 0..header.count as usize {
            let at = HEADER + 8 + i * 92;
            let placement: Placement = wire(
                self.gpt
                    .get(at..at + 92)
                    .ok_or_else(|| anyhow::anyhow!("short Fix perso placement table"))?,
            )?;
            let perso = self.required(table, i * 4)?;
            ensure!(
                perso.owner == Owner::Fix,
                "Fix perso binding references a level-owned actor"
            );
            let node = self.root(at + 88)?;
            ensure!(
                u32_at(self.bytes(node, 4)?, 0)? == 4,
                "Fix perso binding targets a non-perso SO"
            );
            ensure!(
                self.required(node, 4)? == perso,
                "Fix perso binding and SO data disagree"
            );
            let graphics = self.required(perso, 0)?;
            let standard = self.required(perso, 4)?;
            let local = Address {
                offset: graphics.offset + 24,
                ..graphics
            };
            ensure!(
                self.required(node, 32)? == local,
                "Fix perso SO does not use its embedded local transform"
            );
            let global = self.required(node, 36)?;
            let transform = SourceTransform {
                translation: placement.transform.translation,
                rotation: placement.transform.rotation,
                basis: placement.transform.basis,
            };
            // Validate through the same useful transform contract as other source matrices.
            crate::model::animation::TransformValue::Affine(transform.clone()).validate()?;
            ensure!(
                bindings.transforms.insert(local, transform).is_none(),
                "duplicate Fix perso placement"
            );
            bindings.pointers.insert(
                Address {
                    offset: standard.offset + 12,
                    ..standard
                },
                node,
            );
            bindings.pointers.insert(
                Address {
                    offset: graphics.offset + 112,
                    ..graphics
                },
                global,
            );
        }
        self.fix_bindings = bindings;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::animation::TransformValue;

    #[test]
    fn fix_placements_rebind_a_b_a_without_mutating_shared_source_blocks() -> Result<()> {
        let fk = BlockKey(5, 0);
        let lk = BlockKey(6, 2);
        let mut fix = vec![0; 640];
        let mut level = vec![0; 512];
        let put = |bytes: &mut [u8], at, value: u32| {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes())
        };
        put(&mut fix, 4, 0x1020);
        put(&mut fix, 32, 0x1080);
        put(&mut fix, 36, 0x1240);
        put(&mut fix, 128 + 112, 0xdeadbeef);
        put(&mut fix, 576 + 12, 0xdeadbeef);
        let original = SourceTransform {
            translation: [8., 9., 10.],
            ..Default::default()
        };
        let matrix =
            crate::openspace::level::write_transform(&TransformValue::Affine(original.clone()))?
                .unwrap();
        fix[152..240].copy_from_slice(&matrix);
        for (at, global) in [(16, 0x3060), (160, 0x30f0)] {
            put(&mut level, at, 4);
            put(&mut level, at + 4, 0x1020);
            put(&mut level, at + 32, 0x1098);
            put(&mut level, at + 36, global);
        }
        let source_bytes = fix.clone();
        let mut pointers = BTreeMap::from([
            ((Owner::Fix, fk, 0x1004), fk),
            ((Owner::Fix, fk, 0x1020), fk),
            ((Owner::Fix, fk, 0x1024), fk),
        ]);
        for at in [0x3010, 0x30a0] {
            for (field, key) in [(4, fk), (32, fk), (36, lk)] {
                pointers.insert((Owner::Level, lk, at + field), key);
            }
        }
        let mut session = SourceSession {
            fix_bindings: Default::default(),
            allocation_cache: Default::default(),
            blocks: BTreeMap::from([
                (
                    Owner::Fix,
                    BTreeMap::from([
                        (
                            fk,
                            Block {
                                base: 0x1000,
                                data: fix,
                            },
                        ),
                        (
                            lk,
                            Block {
                                base: 0x3000,
                                data: vec![],
                            },
                        ),
                    ]),
                ),
                (
                    Owner::Level,
                    BTreeMap::from([
                        (
                            lk,
                            Block {
                                base: 0x3000,
                                data: level,
                            },
                        ),
                        (
                            fk,
                            Block {
                                base: 0x1000,
                                data: vec![],
                            },
                        ),
                    ]),
                ),
            ]),
            actual: BTreeMap::from([(fk, Owner::Fix), (lk, Owner::Level)]),
            pointers,
            gpt: vec![0; HEADER + 8 + 92],
            rtp: BTreeMap::from([(0x1004, fk), (0x3010, lk), (0x30a0, lk)]),
            ptx: vec![],
            rtt: BTreeMap::new(),
        };
        let standard = Address {
            owner: Owner::Fix,
            key: fk,
            offset: 576,
        };
        let graphics = Address {
            offset: 128,
            ..standard
        };
        let local = Address {
            offset: 152,
            ..standard
        };
        assert!(
            session.pointer(standard, 12).is_err(),
            "saved backlink is deliberately stale"
        );
        for (node, global, position) in [
            (0x3010, 96, [1., 2., 3.]),
            (0x30a0, 240, [4., 5., 6.]),
            (0x3010, 96, [1., 2., 3.]),
        ] {
            put(&mut session.gpt, HEADER, 1);
            put(&mut session.gpt, HEADER + 4, 0x1004);
            let t = SourceTransform {
                translation: position,
                ..Default::default()
            };
            let bytes =
                crate::openspace::level::write_transform(&TransformValue::Affine(t.clone()))?
                    .unwrap();
            session.gpt[HEADER + 8..HEADER + 96].copy_from_slice(&bytes);
            put(&mut session.gpt, HEADER + 96, node);
            session.bind_fix_persos()?;
            assert_eq!(session.transform(local)?, t);
            assert_eq!(
                session.required(standard, 12)?,
                Address {
                    owner: Owner::Level,
                    key: lk,
                    offset: (node - 0x3000) as usize
                }
            );
            assert_eq!(
                session.required(graphics, 112)?,
                Address {
                    owner: Owner::Level,
                    key: lk,
                    offset: global
                }
            );
            assert_eq!(session.blocks[&Owner::Fix][&fk].data, source_bytes);
        }
        put(&mut session.gpt, HEADER, 0);
        session.bind_fix_persos()?;
        assert!(session.fix_bindings.pointers.is_empty());
        assert_eq!(session.transform(local)?, original);
        session.gpt.truncate(HEADER + 7);
        assert!(session.bind_fix_persos().is_err());
        Ok(())
    }
}
