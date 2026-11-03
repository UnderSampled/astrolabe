//! One active level and its Fix bindings; all addresses are transient.
use super::container::{
    PointerMap, file, level_textures, merge_relocations, pointer_file, reloc, sna, u32_at, wire,
};
use super::records::*;
use crate::model::{
    Selection,
    ai::{SourceObjectIdentity, SourceRole},
    geometry::SourceTransform,
};
use anyhow::{Result, anyhow, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub(super) struct SourceFiles {
    relative: Vec<PathBuf>,
    resolved: Vec<PathBuf>,
}
impl SourceFiles {
    fn new(game: &Path, selection: &Selection) -> Result<Self> {
        ensure!(
            Selection::hype(selection.level.clone(), selection.locale.clone())? == *selection,
            "unsupported source profile"
        );
        let directory = PathBuf::from("Gamedata/World/Levels");
        let level = directory.join(&selection.level);
        let relative = vec![
            directory.join("Fix.sna"),
            directory.join("Fix.rtb"),
            level.join(format!("{}.sna", selection.level)),
            level.join(format!("{}.rtb", selection.level)),
            level.join("fixlvl.rtb"),
            level.join(format!("{}.rtp", selection.level)),
            level.join(format!("{}.rtt", selection.level)),
            level.join(format!("{}.gpt", selection.level)),
            level.join(format!("{}.ptx", selection.level)),
        ];
        let resolved = relative.iter().map(|path| game.join(path)).collect();
        Ok(Self { relative, resolved })
    }
    fn case_insensitive(game: &Path, selection: &Selection) -> Result<Self> {
        let mut files = Self::new(game, selection)?;
        files.resolved = files
            .relative
            .iter()
            .map(|relative| resolve_unique_case_insensitive(game, relative))
            .collect::<Result<Vec<_>>>()?;
        Ok(files)
    }
    pub(super) fn entries(&self) -> impl Iterator<Item = (&Path, &Path)> {
        self.relative
            .iter()
            .zip(&self.resolved)
            .map(|(relative, resolved)| (relative.as_path(), resolved.as_path()))
    }
    fn path(&self, index: usize) -> &Path {
        &self.resolved[index]
    }
}

pub(super) fn resolve_unique_case_insensitive(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut resolved = root.to_owned();
    for component in relative.components() {
        let wanted = component.as_os_str().to_string_lossy();
        let entries = std::fs::read_dir(&resolved)?.collect::<std::io::Result<Vec<_>>>()?;
        let matches = entries
            .into_iter()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&wanted)
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1,
            "source path component {wanted:?} has {} case-insensitive matches",
            matches.len()
        );
        resolved = matches.into_iter().next().unwrap();
    }
    Ok(resolved)
}
pub(super) struct RootedNode {
    pub(super) path: String,
    pub(super) address: Address,
    pub(super) transforms: Vec<SourceTransform>,
}
type AllocationIndices = BTreeMap<(Owner, BlockKey), BTreeMap<usize, u32>>;
pub(super) struct SourceSession {
    pub(super) fix_bindings: super::fix_bindings::FixBindings,
    pub(super) allocation_cache: std::cell::RefCell<AllocationIndices>,
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
        Self::open_files(SourceFiles::new(game, selection)?).map(|(session, _)| session)
    }
    pub(super) fn open_with_fingerprint_files(
        game: &Path,
        selection: &Selection,
    ) -> Result<(Self, SourceFiles)> {
        Self::open_files(SourceFiles::case_insensitive(game, selection)?)
    }
    fn open_files(files: SourceFiles) -> Result<(Self, SourceFiles)> {
        let fix = sna(&file(files.path(0))?)?;
        let lvl = sna(&file(files.path(2))?)?;
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
            (Owner::Fix, files.path(1)),
            (Owner::Level, files.path(3)),
            (Owner::Fix, files.path(4)),
        ] {
            merge_relocations(&mut pointers, owner, reloc(&file(path)?)?)?;
        }
        let rtp = pointer_file(&file(files.path(5))?)?;
        let rtt = pointer_file(&file(files.path(6))?)?;
        let mut session = Self {
            fix_bindings: Default::default(),
            allocation_cache: Default::default(),
            blocks: BTreeMap::from([(Owner::Fix, fix), (Owner::Level, lvl)]),
            actual,
            pointers,
            gpt: file(files.path(7))?,
            rtp,
            ptx: file(files.path(8))?,
            rtt,
        };
        for (&owner, blocks) in &session.blocks {
            for (&key, block) in blocks {
                super::coverage::register(owner, key, block.data.len());
            }
        }
        session.bind_fix_persos()?;
        Ok((session, files))
    }
    /// Direct block access; byte coverage lists these callers per block.
    #[track_caller]
    pub(super) fn block(&self, a: Address) -> Result<&Block> {
        super::coverage::record_block(a, std::panic::Location::caller());
        self.block_untracked(a)
    }
    /// A resolved target must exist; checking that is not reading it.
    fn in_bounds(&self, a: Address) -> Result<()> {
        anyhow::ensure!(
            a.offset < self.block_untracked(a)?.data.len(),
            "read beyond block {a:?} length 1"
        );
        Ok(())
    }
    fn block_untracked(&self, a: Address) -> Result<&Block> {
        self.blocks
            .get(&a.owner)
            .and_then(|b| b.get(&a.key))
            .ok_or_else(|| anyhow!("missing block {a:?}"))
    }
    pub(super) fn allocation_index(&self, a: Address) -> Result<u32> {
        let key = (a.owner, a.key);
        if !self.allocation_cache.borrow().contains_key(&key) {
            self.allocation_cache
                .borrow_mut()
                .insert(key, self.allocation_indices(a.owner, a.key)?);
        }
        self.allocation_cache.borrow()[&key]
            .get(&a.offset)
            .copied()
            .ok_or_else(|| anyhow!("reference is not an allocation start: {a:?}"))
    }
    #[track_caller]
    pub(super) fn bytes(&self, a: Address, n: usize) -> Result<&[u8]> {
        let block = self.block_untracked(a)?;
        super::coverage::record(a, n, block.data.len(), std::panic::Location::caller());
        block
            .data
            .get(
                a.offset
                    ..a.offset
                        .checked_add(n)
                        .ok_or_else(|| anyhow!("size overflow"))?,
            )
            .ok_or_else(|| anyhow!("read beyond block {a:?} length {n}"))
    }
    /// Read one whole MMG allocation, excluding its source-only size header.
    /// Typed consumers use this to validate record boundaries before traversal.
    #[track_caller]
    pub(super) fn allocation_bytes(&self, a: Address) -> Result<&[u8]> {
        self.allocation_index(a)?;
        let header = Address {
            offset: a
                .offset
                .checked_sub(4)
                .ok_or_else(|| anyhow!("allocation has no size header at {a:?}"))?,
            ..a
        };
        let size = (u32_at(self.bytes(header, 4)?, 0)? as usize)
            .checked_mul(4)
            .and_then(|size| size.checked_sub(4))
            .ok_or_else(|| anyhow!("invalid allocation size at {a:?}"))?;
        self.bytes(a, size)
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
        self.in_bounds(a)?;
        Ok(a)
    }
    /// Resolve a pointer against the explicitly named package's own SNA block.
    /// Fix.gpt/rtp and Fix.ptx/rtt identify Fix allocations even when an active
    /// level has a block with the same key; those references must not inherit
    /// the level-binding behavior of `resolve`.
    pub(super) fn resolve_local(&self, owner: Owner, key: BlockKey, value: u32) -> Result<Address> {
        let local = self.blocks[&owner]
            .get(&key)
            .ok_or_else(|| anyhow!("missing {owner:?} source base {key:?}"))?;
        let offset = value
            .checked_sub(local.base)
            .ok_or_else(|| anyhow!("pointer below {owner:?} block base"))?
            as usize;
        let address = Address { owner, key, offset };
        self.in_bounds(address)?;
        Ok(address)
    }
    #[track_caller]
    pub(super) fn pointer(&self, a: Address, offset: usize) -> Result<Option<Address>> {
        let site = Address {
            offset: a
                .offset
                .checked_add(offset)
                .ok_or_else(|| anyhow!("offset overflow"))?,
            ..a
        };
        if let Some(&target) = self.fix_bindings.pointers.get(&site) {
            return Ok(Some(target));
        }
        let value = u32_at(self.bytes(site, 4)?, 0)?;
        if value == 0 {
            return Ok(None);
        }
        let address = self
            .block_untracked(a)?
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
    /// Whether this word is declared as a pointer by source relocation data.
    /// Unknown AI operands use this before retaining any nonpointer bits.
    #[track_caller]
    pub(super) fn relocated(&self, site: Address) -> Result<bool> {
        if self.fix_bindings.pointers.contains_key(&site) {
            return Ok(true);
        }
        let address = self
            .block(site)?
            .base
            .checked_add(site.offset.try_into()?)
            .ok_or_else(|| anyhow!("source site overflow"))?;
        Ok(self.pointers.contains_key(&(site.owner, site.key, address)))
    }
    pub(super) fn resembles_source_address(&self, value: u32) -> bool {
        self.blocks
            .values()
            .flat_map(BTreeMap::values)
            .any(|block| {
                let start = u64::from(block.base);
                let end = start + block.data.len() as u64;
                let value = u64::from(value);
                value >= start && value < end
            })
    }
    #[track_caller]
    pub(super) fn source_identity(&self, a: Address) -> Result<SourceObjectIdentity> {
        Ok(SourceObjectIdentity {
            role: match a.owner {
                Owner::Fix => SourceRole::Fix,
                Owner::Level => SourceRole::Level,
            },
            block: [a.key.0, a.key.1],
            allocation: self.allocation_index(a)?,
        })
    }
    #[track_caller]
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
    #[track_caller]
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
    // This lint was added after Rust 1.96; keep its narrow exception compatible
    // with older Clippy while const-generic arithmetic remains unavailable.
    #[allow(unknown_lints, clippy::chunks_exact_to_as_chunks)]
    #[track_caller]
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
    #[allow(unknown_lints, clippy::chunks_exact_to_as_chunks)]
    #[track_caller]
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
    #[track_caller]
    pub(super) fn transform(&self, address: Address) -> Result<SourceTransform> {
        if let Some(value) = self.fix_bindings.transforms.get(&address) {
            return Ok(value.clone());
        }
        let bytes = &self.block(address)?.data[address.offset..];
        Ok(super::transform::read_transform(bytes)?.affine())
    }
    /// Engine FUN_004aeb10 prefixes pool allocations with their size in u32 words,
    /// including the header. The saved used prefix ends in a trimmed free-block header.
    pub(super) fn allocation_indices(
        &self,
        owner: Owner,
        key: BlockKey,
    ) -> Result<BTreeMap<usize, u32>> {
        let data = &self.blocks[&owner][&key].data;
        let mut out = BTreeMap::new();
        let mut cursor = 0;
        let mut index = 0;
        while cursor + 4 <= data.len() {
            let size = (u32_at(data, cursor)? as usize)
                .checked_mul(4)
                .ok_or_else(|| anyhow!("allocation size overflow"))?;
            if size > data.len() - cursor {
                ensure!(data.len() - cursor <= 9, "invalid trimmed allocation pool");
                break;
            }
            ensure!(size >= 4, "zero-size allocation");
            out.insert(cursor + 4, index);
            cursor += size;
            index += 1;
        }
        Ok(out)
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
            let h: SuperObjectWire = wire(ctx.bytes(a, 64)?)?;
            ensure!(h.child_count <= 100_000, "excessive child count");
            let mut chain = chain.to_vec();
            if h.last_compute_frame == u32::MAX {
                chain.clear();
            }
            if let Some(t) = ctx.pointer(
                a,
                if h.last_compute_frame == u32::MAX {
                    36
                } else {
                    SuperObjectWire::LOCAL_TRANSFORM
                },
            )? {
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
