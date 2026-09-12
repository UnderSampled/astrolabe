//! One active level and its Fix bindings; all addresses are transient.
use super::container::{
    PointerMap, file, level_textures, merge_relocations, pointer_file, reloc, sna, u32_at, wire,
};
use super::records::*;
use crate::model::{Selection, geometry::SourceTransform};
use anyhow::{Result, anyhow, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
pub(super) struct RootedNode {
    pub(super) path: String,
    pub(super) address: Address,
    pub(super) transforms: Vec<SourceTransform>,
}
pub(super) struct SourceSession {
    pub(super) blocks: BTreeMap<Owner, BTreeMap<BlockKey, Block>>,
    pub(super) actual: BTreeMap<BlockKey, Owner>,
    pub(super) pointers: PointerMap,
    pub(super) gpt: Vec<u8>,
    pub(super) rtp: BTreeMap<u32, BlockKey>,
    pub(super) ptx: Vec<u8>,
    pub(super) rtt: BTreeMap<u32, BlockKey>,
}
impl SourceSession {
    pub(super) fn open(game: &Path, selection: &Selection) -> Result<Self> {
        ensure!(
            selection.profile == "hype-pc-montreal",
            "unsupported source profile"
        );
        let levels = game.join("Gamedata/World/Levels");
        let level = levels.join(&selection.level);
        let fix = sna(&file(&levels.join("Fix.sna"))?)?;
        let lvl = sna(&file(&level.join(format!("{}.sna", selection.level)))?)?;
        let mut actual = BTreeMap::new();
        for (owner, blocks) in [(Owner::Fix, &fix), (Owner::Level, &lvl)] {
            for (key, b) in blocks {
                if !b.data.is_empty() {
                    actual.insert(*key, owner);
                }
            }
        }
        let mut pointers = BTreeMap::new();
        for (owner, path) in [
            (Owner::Fix, levels.join("Fix.rtb")),
            (Owner::Level, level.join(format!("{}.rtb", selection.level))),
            (Owner::Fix, level.join("fixlvl.rtb")),
        ] {
            merge_relocations(&mut pointers, owner, reloc(&file(&path)?)?)?;
        }
        let rtp = pointer_file(&file(&level.join(format!("{}.rtp", selection.level)))?)?;
        let rtt = pointer_file(&file(&level.join(format!("{}.rtt", selection.level)))?)?;
        Ok(Self {
            blocks: BTreeMap::from([(Owner::Fix, fix), (Owner::Level, lvl)]),
            actual,
            pointers,
            gpt: file(&level.join(format!("{}.gpt", selection.level)))?,
            rtp,
            ptx: file(&level.join(format!("{}.ptx", selection.level)))?,
            rtt,
        })
    }
    pub(super) fn block(&self, a: Address) -> Result<&Block> {
        self.blocks
            .get(&a.owner)
            .and_then(|b| b.get(&a.key))
            .ok_or_else(|| anyhow!("missing block {a:?}"))
    }
    pub(super) fn bytes(&self, a: Address, n: usize) -> Result<&[u8]> {
        self.block(a)?
            .data
            .get(
                a.offset
                    ..a.offset
                        .checked_add(n)
                        .ok_or_else(|| anyhow!("size overflow"))?,
            )
            .ok_or_else(|| anyhow!("read beyond block {a:?} length {n}"))
    }
    pub(super) fn resolve(&self, owner: Owner, key: BlockKey, value: u32) -> Result<Address> {
        let local = self.blocks[&owner]
            .get(&key)
            .ok_or_else(|| anyhow!("missing source base {key:?}"))?;
        let owner = *self
            .actual
            .get(&key)
            .ok_or_else(|| anyhow!("target block is not populated: {key:?}"))?;
        let offset = value
            .checked_sub(local.base)
            .ok_or_else(|| anyhow!("pointer below block base"))? as usize;
        let a = Address { owner, key, offset };
        self.bytes(a, 1)?;
        Ok(a)
    }
    pub(super) fn pointer(&self, a: Address, offset: usize) -> Result<Option<Address>> {
        let site = Address {
            offset: a
                .offset
                .checked_add(offset)
                .ok_or_else(|| anyhow!("offset overflow"))?,
            ..a
        };
        let value = u32_at(self.bytes(site, 4)?, 0)?;
        if value == 0 {
            return Ok(None);
        }
        let address = self
            .block(a)?
            .base
            .checked_add(site.offset.try_into()?)
            .ok_or_else(|| anyhow!("site overflow"))?;
        let key = *self
            .pointers
            .get(&(a.owner, a.key, address))
            .ok_or_else(|| anyhow!("nonzero pointer lacks relocation at {site:?}"))?;
        if key == BlockKey(255, 255) {
            return Ok(None);
        }
        Ok(Some(self.resolve(a.owner, key, value)?))
    }
    pub(super) fn required(&self, a: Address, offset: usize) -> Result<Address> {
        self.pointer(a, offset)?
            .ok_or_else(|| anyhow!("required null reference at {a:?}+{offset:x}"))
    }
    pub(super) fn radiosity(
        &self,
        ipo: Address,
        lod: usize,
        lod_count: usize,
        vertex_count: usize,
    ) -> Result<Vec<[i16; 4]>> {
        let Some(isi) = self.pointer(ipo, InstantiatedObjectWire::RADIOSITY)? else {
            return Ok(Vec::new());
        };
        let header: RadiosityWire = wire(self.bytes(isi, 8)?)?;
        ensure!(
            header.lod_count as usize == lod_count,
            "radiosity and visual LOD counts differ"
        );
        ensure!(lod < lod_count, "radiosity LOD out of range");
        let table = self.required(isi, RadiosityWire::LODS)?;
        self.bytes(table, lod_count * 8)?;
        let entry = Address {
            offset: table.offset + lod * 8,
            ..table
        };
        let header: RadiosityLodWire = wire(self.bytes(entry, 8)?)?;
        ensure!(
            header.vertex_count as usize == vertex_count,
            "radiosity and geometry vertex counts differ"
        );
        let bytes = self.bytes(
            self.required(entry, RadiosityLodWire::COLORS)?,
            vertex_count * 8,
        )?;
        bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| Ok(wire::<RadiosityColorWire>(b)?.components))
            .collect()
    }
    pub(super) fn texture_addresses(&self) -> Result<BTreeSet<Address>> {
        let table = level_textures(&self.ptx)?;
        let mut textures = BTreeSet::new();
        for value in table.pointers {
            if value == 0 {
                continue;
            }
            let key = *self
                .rtt
                .get(&value)
                .ok_or_else(|| anyhow!("nonzero PTX pointer lacks RTT entry"))?;
            if key != BlockKey(255, 255) {
                textures.insert(self.resolve(Owner::Level, key, value)?);
            }
        }
        Ok(textures)
    }
    pub(super) fn root(&self, offset: usize) -> Result<Address> {
        let value = u32_at(&self.gpt, offset)?;
        let key = *self
            .rtp
            .get(&value)
            .ok_or_else(|| anyhow!("GPT root lacks RTP entry"))?;
        self.resolve(Owner::Level, key, value)
    }
    pub(super) fn label(&self, a: Address) -> Result<String> {
        Ok(format!(
            "{:?}:{:02x}:{:02x}+0x{:x} (VM 0x{:08x})",
            a.owner,
            a.key.0,
            a.key.1,
            a.offset,
            self.block(a)?.base + a.offset as u32
        ))
    }
    // Stable Rust cannot use N * 4 as an as_chunks const argument.
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub(super) fn floats<const N: usize>(&self, a: Address, count: usize) -> Result<Vec<[f32; N]>> {
        let data = self.bytes(
            a,
            count
                .checked_mul(N * 4)
                .ok_or_else(|| anyhow!("array overflow"))?,
        )?;
        data.chunks_exact(N * 4)
            .map(|b| {
                let mut v = [0.; N];
                for (i, x) in v.iter_mut().enumerate() {
                    *x = f32::from_bits(u32_at(b, i * 4)?);
                }
                Ok(v)
            })
            .collect()
    }
    // Stable Rust cannot use N * 2 as an as_chunks const argument.
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub(super) fn short_arrays<const N: usize>(
        &self,
        a: Address,
        count: usize,
    ) -> Result<Vec<[u16; N]>> {
        let data = self.bytes(
            a,
            count
                .checked_mul(N * 2)
                .ok_or_else(|| anyhow!("array overflow"))?,
        )?;
        data.chunks_exact(N * 2)
            .map(|b| {
                let mut v = [0; N];
                for (i, x) in v.iter_mut().enumerate() {
                    *x = u16::from_le_bytes(b[i * 2..i * 2 + 2].try_into()?);
                }
                Ok(v)
            })
            .collect()
    }
    pub(super) fn transform(&self, address: Address) -> Result<SourceTransform> {
        let transform: TransformWire = wire(self.bytes(address, 88)?)?;
        Ok(SourceTransform {
            translation: transform.translation,
            rotation: transform.rotation,
            basis: transform.basis,
        })
    }
    pub(super) fn tree(&self) -> Result<Vec<RootedNode>> {
        fn visit(
            ctx: &SourceSession,
            a: Address,
            path: String,
            chain: &[SourceTransform],
            seen: &mut BTreeSet<Address>,
            out: &mut Vec<RootedNode>,
            depth: usize,
        ) -> Result<()> {
            ensure!(depth < 256 && seen.len() < 100_000, "scene traversal limit");
            ensure!(seen.insert(a), "cycle or repeated child in rooted scene");
            let h: SuperObjectWire = wire(ctx.bytes(a, 40)?)?;
            ensure!(h.child_count <= 100_000, "excessive child count");
            let mut chain = chain.to_vec();
            if let Some(t) = ctx.pointer(a, SuperObjectWire::LOCAL_TRANSFORM)? {
                chain.push(ctx.transform(t)?);
            }
            out.push(RootedNode {
                path: path.clone(),
                address: a,
                transforms: chain.clone(),
            });
            let mut child = ctx.pointer(a, SuperObjectWire::FIRST_CHILD)?;
            for i in 0..h.child_count {
                let current = child.ok_or_else(|| anyhow!("short scene child list"))?;
                visit(
                    ctx,
                    current,
                    format!("{path}/{i}"),
                    &chain,
                    seen,
                    out,
                    depth + 1,
                )?;
                child = ctx.pointer(current, SuperObjectWire::NEXT)?;
            }
            ensure!(
                h.child_count == 0 || child.is_none(),
                "scene child count/list mismatch"
            );
            Ok(())
        }
        let mut out = Vec::new();
        visit(
            self,
            self.root(0x10)?,
            "world".into(),
            &[],
            &mut BTreeSet::new(),
            &mut out,
            0,
        )?;
        Ok(out)
    }
}
