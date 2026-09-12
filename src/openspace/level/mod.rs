//! Hype PC rooted model import. Transient virtual addresses never enter model assets.
//! Engine/Raymap evidence and reconstruction limits: docs/level-rust-research.md.
use super::compression::{decode_envelope, encode_envelope};
use crate::model::geometry::{Material, Model, SourceTransform, Submesh, TextureInfo};
use crate::model::{AssetPath, AssetRef, Selection};
use anyhow::{Context, Result, anyhow, ensure};
use binrw::{BinRead, BinWrite};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
    path::Path,
};
const MAX_FILE: usize = 64 * 1024 * 1024;
const MAX_BLOCK: usize = 128 * 1024 * 1024;
const MAX_TOTAL: usize = 512 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Owner {
    Fix,
    Level,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BlockKey(u8, u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Address {
    owner: Owner,
    key: BlockKey,
    offset: usize,
}
#[derive(Clone)]
struct Block {
    base: u32,
    data: Vec<u8>,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct SnaStart {
    module: u8,
    id: u8,
    base: i32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct SnaInfo {
    /// Mmg block information: capacity end is inclusive; the next two are allocator state.
    capacity_end: u32,
    free_list: u32,
    high_water: u32,
    size: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct RelocGroup {
    module: u8,
    id: u8,
    count: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct RelocRow {
    value: u32,
    module: u8,
    id: u8,
    unknown: [u8; 2],
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct SuperObjectWire {
    kind: u32,
    data: u32,
    first_child: u32,
    last_child: u32,
    child_count: u32,
    next: u32,
    previous: u32,
    parent: u32,
    local_transform: u32,
    global_transform: u32,
}
/// Hype engine allocates 0x30, not Raymap/C#'s 0x40 with spurious sphere fields.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct GeometryWire {
    point_count: u16,
    point_reserved: u16,
    points: u32,
    normals: u32,
    point_materials: u32,
    unknown_10: u32,
    element_count: u16,
    element_reserved: u16,
    element_types: u32,
    elements: u32,
    spatial_data: u32,
    unknown_tail: [u32; 3],
}
/// Engine GEO_vCreateElementIndexedTriangles allocates exactly 0x20 bytes.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct TrianglesWire {
    material: u32,
    triangle_count: u16,
    uv_count: u16,
    triangles: u32,
    uv_indices: u32,
    normals: u32,
    uvs: u32,
    point_count: u16,
    point_reserved: u16,
    points: u32,
}
/// IPO_fn_hAllocInstanciatedPhysicalObject allocates exactly two pointers (8 bytes).
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct InstantiatedObjectWire {
    physical_object: u32,
    radiosity: u32,
}
/// ISI_fn_hCreateISI / hCreateISILOD allocate 8-byte records. Their counts are signed
/// shorts, not the u32 used by Raymap; the engine initializes the second short to zero.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct RadiosityWire {
    #[br(assert(lod_count > 0))]
    lod_count: i16,
    #[br(assert(reserved == 0, "unsupported ISI reserved field"))]
    reserved: u16,
    lods: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct RadiosityLodWire {
    #[br(assert(vertex_count > 0))]
    vertex_count: i16,
    #[br(assert(reserved == 0, "unsupported ISI LOD reserved field"))]
    reserved: u16,
    colors: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct RadiosityColorWire {
    /// ISI_ColISIToGEO converts all four signed components with scale 1/256, unclamped.
    components: [i16; 4],
}
/// Level PTX, corroborated by engine FUN_00483860/FUN_00483930. The initial u32 is
/// texture table capacity, not a pointer; the channel table covers all 1024 slots.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
struct LevelTexturesWire {
    #[br(assert(capacity == 1024, "unsupported Hype texture table capacity"))]
    capacity: u32,
    #[br(assert(level_slots <= capacity))]
    level_slots: u32,
    #[br(count = level_slots)]
    pointers: Vec<u32>,
    #[br(assert(loaded_textures <= capacity))]
    loaded_textures: u32,
    #[br(count = capacity)]
    memory_channels: Vec<u32>,
    current_channel: u32,
}
fn level_textures(bytes: &[u8]) -> Result<LevelTexturesWire> {
    let mut cursor = Cursor::new(bytes);
    let table = LevelTexturesWire::read_le(&mut cursor)?;
    ensure!(
        cursor.position() == bytes.len() as u64,
        "PTX trailing bytes"
    );
    Ok(table)
}
fn wire<T: BinRead>(bytes: &[u8]) -> Result<T>
where
    for<'a> T::Args<'a>: Default,
{
    Ok(T::read_le(&mut Cursor::new(bytes))?)
}
fn encoded<T: BinWrite>(value: &T) -> Result<Vec<u8>>
where
    for<'a> T::Args<'a>: Default,
{
    let mut c = Cursor::new(Vec::new());
    value.write_le(&mut c)?;
    Ok(c.into_inner())
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(o..o + 4)
            .ok_or_else(|| anyhow!("truncated u32"))?
            .try_into()?,
    ))
}
fn file(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        path.metadata()?.len() <= MAX_FILE as u64,
        "file too large: {}",
        path.display()
    );
    std::fs::read(path).with_context(|| path.display().to_string())
}
fn sna(bytes: &[u8]) -> Result<BTreeMap<BlockKey, Block>> {
    let mut pos = 0;
    let mut blocks = BTreeMap::new();
    let mut total = 0;
    while pos < bytes.len() {
        let h: SnaStart = wire(&bytes[pos..])?;
        pos += 6;
        if h.base == -1 {
            continue;
        }
        ensure!(h.base >= 0, "unsupported negative block base");
        let meta: SnaInfo = wire(&bytes[pos..])?;
        pos += 16;
        let data = if meta.size > 0 {
            let (data, n) = decode_envelope(
                bytes
                    .get(pos..)
                    .ok_or_else(|| anyhow!("missing envelope"))?,
                MAX_BLOCK,
            )?;
            ensure!(
                data.len() == meta.size as usize,
                "SNA block size disagreement"
            );
            pos += n;
            data
        } else {
            Vec::new()
        };
        total += data.len();
        ensure!(total <= MAX_TOTAL, "SNA decompression budget exceeded");
        ensure!(
            blocks
                .insert(
                    BlockKey(h.module, h.id),
                    Block {
                        base: h.base as u32,
                        data
                    }
                )
                .is_none(),
            "duplicate SNA block identity"
        );
    }
    ensure!(pos == bytes.len(), "SNA trailing bytes");
    Ok(blocks)
}
/// Frame one nonempty controlled block, deriving allocator extent and envelope from current data.
/// Free-list state is empty: this is container validation for a compact fragment, not a complete
/// allocator-compatible/game-loadable SNA package. See FUN_004e86e0's no-free-list write branch.
pub fn write_sna_block(data: &[u8], block: (u8, u8), base: u32) -> Result<Vec<u8>> {
    ensure!(
        !data.is_empty() && data.len() <= MAX_BLOCK,
        "invalid SNA fragment size"
    );
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "SNA base must be nonzero/aligned"
    );
    let end = base
        .checked_add(data.len().try_into()?)
        .ok_or_else(|| anyhow!("SNA extent overflow"))?;
    ensure!(
        end <= i32::MAX as u32,
        "SNA extent exceeds supported signed address range"
    );
    ensure!(
        block.0 != 255 && block.1 != 255,
        "sentinel is not a populated SNA block"
    );
    let mut out = encoded(&SnaStart {
        module: block.0,
        id: block.1,
        base: base.try_into()?,
    })?;
    out.extend(encoded(&SnaInfo {
        capacity_end: end - 1,
        free_list: 0,
        high_water: end,
        size: data.len().try_into()?,
    })?);
    out.extend(encode_envelope(data)?);
    ensure!(out.len() <= MAX_FILE, "encoded SNA exceeds file budget");
    Ok(out)
}
fn reloc(bytes: &[u8]) -> Result<BTreeMap<BlockKey, Vec<RelocRow>>> {
    let count = *bytes
        .first()
        .ok_or_else(|| anyhow!("empty relocation table"))?;
    let mut p = 1;
    let mut groups = BTreeMap::new();
    for _ in 0..count {
        let h: RelocGroup = wire(
            bytes
                .get(p..)
                .ok_or_else(|| anyhow!("truncated RT header"))?,
        )?;
        p += 6;
        let mut rows = Vec::new();
        if h.count > 0 {
            let (data, n) = decode_envelope(
                bytes
                    .get(p..)
                    .ok_or_else(|| anyhow!("missing RT envelope"))?,
                MAX_BLOCK,
            )?;
            p += n;
            ensure!(data.len() == h.count as usize * 8, "RT row count mismatch");
            for chunk in data.as_chunks::<8>().0.iter() {
                rows.push(wire(chunk)?);
            }
        }
        ensure!(
            groups.insert(BlockKey(h.module, h.id), rows).is_none(),
            "duplicate RT group"
        );
    }
    ensure!(p == bytes.len(), "RT trailing bytes");
    Ok(groups)
}
type PointerMap = BTreeMap<(Owner, BlockKey, u32), BlockKey>;
fn merge_relocations(
    pointers: &mut PointerMap,
    owner: Owner,
    groups: BTreeMap<BlockKey, Vec<RelocRow>>,
) -> Result<()> {
    for (key, rows) in groups {
        for row in rows {
            let target = BlockKey(row.module, row.id);
            let site = (owner, key, row.value);
            if let Some(old) = pointers.get(&site).copied() {
                // fixlvl may fill unresolved sites; a sentinel must never erase an existing binding.
                if target == BlockKey(255, 255) {
                    continue;
                }
                ensure!(
                    old == target || old == BlockKey(255, 255),
                    "conflicting relocation site"
                );
            }
            pointers.insert(site, target);
        }
    }
    Ok(())
}
fn pointer_file(bytes: &[u8]) -> Result<BTreeMap<u32, BlockKey>> {
    let mut map = BTreeMap::new();
    for rows in reloc(bytes)?.into_values() {
        for row in rows {
            let k = BlockKey(row.module, row.id);
            let old = map.insert(row.value, k);
            ensure!(old.is_none_or(|v| v == k), "ambiguous pointer-file value");
        }
    }
    Ok(map)
}
struct Import {
    blocks: BTreeMap<Owner, BTreeMap<BlockKey, Block>>,
    actual: BTreeMap<BlockKey, Owner>,
    pointers: PointerMap,
    gpt: Vec<u8>,
    rtp: BTreeMap<u32, BlockKey>,
    ptx: Vec<u8>,
    rtt: BTreeMap<u32, BlockKey>,
}
impl Import {
    fn open(game: &Path, selection: &Selection) -> Result<Self> {
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
    fn block(&self, a: Address) -> Result<&Block> {
        self.blocks
            .get(&a.owner)
            .and_then(|b| b.get(&a.key))
            .ok_or_else(|| anyhow!("missing block {a:?}"))
    }
    fn bytes(&self, a: Address, n: usize) -> Result<&[u8]> {
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
    fn resolve(&self, owner: Owner, key: BlockKey, value: u32) -> Result<Address> {
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
    fn pointer(&self, a: Address, offset: usize) -> Result<Option<Address>> {
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
    fn required(&self, a: Address, offset: usize) -> Result<Address> {
        self.pointer(a, offset)?
            .ok_or_else(|| anyhow!("required null reference at {a:?}+{offset:x}"))
    }
    fn radiosity(
        &self,
        ipo: Address,
        lod: usize,
        lod_count: usize,
        vertex_count: usize,
    ) -> Result<Vec<[i16; 4]>> {
        let Some(isi) = self.pointer(ipo, 4)? else {
            return Ok(Vec::new());
        };
        let header: RadiosityWire = wire(self.bytes(isi, 8)?)?;
        ensure!(
            header.lod_count as usize == lod_count,
            "radiosity and visual LOD counts differ"
        );
        ensure!(lod < lod_count, "radiosity LOD out of range");
        let table = self.required(isi, 4)?;
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
        let bytes = self.bytes(self.required(entry, 4)?, vertex_count * 8)?;
        bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| Ok(wire::<RadiosityColorWire>(b)?.components))
            .collect()
    }
    fn texture_addresses(&self) -> Result<BTreeSet<Address>> {
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
    fn root(&self, offset: usize) -> Result<Address> {
        let value = u32_at(&self.gpt, offset)?;
        let key = *self
            .rtp
            .get(&value)
            .ok_or_else(|| anyhow!("GPT root lacks RTP entry"))?;
        self.resolve(Owner::Level, key, value)
    }
    fn label(&self, a: Address) -> Result<String> {
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
    fn floats<const N: usize>(&self, a: Address, count: usize) -> Result<Vec<[f32; N]>> {
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
    fn short_arrays<const N: usize>(&self, a: Address, count: usize) -> Result<Vec<[u16; N]>> {
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
    fn transform(&self, a: Address) -> Result<SourceTransform> {
        let flags = u32_at(self.bytes(a, 4)?, 0)?;
        ensure!(flags & 0x80 != 0, "compressed SO transform is unsupported");
        let b = self.bytes(a, 88)?;
        let value = |o| f32::from_bits(u32_at(b, o).expect("bounded matrix"));
        let mut t = SourceTransform::default();
        for i in 0..3 {
            t.translation[i] = value(4 + i * 4);
            for j in 0..3 {
                t.rotation[i][j] = value(16 + i * 12 + j * 4);
                t.basis[i][j] = value(52 + i * 12 + j * 4);
            }
        }
        Ok(t)
    }
    fn tree(&self) -> Result<Vec<(String, Address, Vec<SourceTransform>)>> {
        fn visit(
            ctx: &Import,
            a: Address,
            path: String,
            chain: &[SourceTransform],
            seen: &mut BTreeSet<Address>,
            out: &mut Vec<(String, Address, Vec<SourceTransform>)>,
            depth: usize,
        ) -> Result<()> {
            ensure!(depth < 256 && seen.len() < 100_000, "scene traversal limit");
            ensure!(seen.insert(a), "cycle or repeated child in rooted scene");
            let h: SuperObjectWire = wire(ctx.bytes(a, 40)?)?;
            ensure!(h.child_count <= 100_000, "excessive child count");
            let mut chain = chain.to_vec();
            if let Some(t) = ctx.pointer(a, 32)? {
                chain.push(ctx.transform(t)?);
            }
            out.push((path.clone(), a, chain.clone()));
            let mut child = ctx.pointer(a, 8)?;
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
                child = ctx.pointer(current, 20)?;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelSummary {
    pub level: String,
    pub actual_world: String,
    pub dynamic_world: String,
    pub father_sector: String,
    pub rooted_nodes: usize,
    pub rooted_models: usize,
    pub texture_references: usize,
    pub populated_blocks: Vec<(String, usize)>,
}
#[derive(Debug, Clone)]
pub struct ModelImport {
    pub model: Model,
    pub materials: Vec<Material>,
    pub source: LevelSummary,
    pub diagnostics: Vec<String>,
}
fn summary(
    ctx: &Import,
    selection: &Selection,
    tree: &[(String, Address, Vec<SourceTransform>)],
) -> Result<LevelSummary> {
    let mut models = 0;
    for (_, a, _) in tree {
        if matches!(u32_at(ctx.bytes(*a, 4)?, 0)?, 0xd | 0x15) {
            models += 1;
        }
    }
    let textures = ctx.texture_addresses()?;
    Ok(LevelSummary {
        level: selection.level.clone(),
        actual_world: ctx.label(ctx.root(0x10)?)?,
        dynamic_world: ctx.label(ctx.root(0x14)?)?,
        father_sector: ctx.label(ctx.root(0x18)?)?,
        rooted_nodes: tree.len(),
        rooted_models: models,
        texture_references: textures.len(),
        populated_blocks: ctx
            .actual
            .iter()
            .map(|(k, o)| {
                (
                    format!("{o:?}:{:02x}:{:02x}", k.0, k.1),
                    ctx.blocks[o][k].data.len(),
                )
            })
            .collect(),
    })
}
pub fn inspect_level(game: &Path, selection: &Selection) -> Result<LevelSummary> {
    let c = Import::open(game, selection)?;
    summary(&c, selection, &c.tree()?)
}
/// Selects the first real IPO in rooted child order; unsupported geometry is an error, never a scanner fallback.
pub fn import_first_model(game: &Path, selection: &Selection) -> Result<ModelImport> {
    let c = Import::open(game, selection)?;
    let tree = c.tree()?;
    let source = summary(&c, selection, &tree)?;
    let (path, so, transforms) = tree
        .iter()
        .find(|(_, a, _)| {
            c.bytes(*a, 4)
                .is_ok_and(|b| matches!(u32_at(b, 0), Ok(0xd | 0x15)))
        })
        .ok_or_else(|| anyhow!("rooted world has no IPO model"))?;
    let ipo = c.required(*so, 4)?;
    let po = c.required(ipo, 0)?;
    let visual = c.required(po, 0)?;
    let v = c.bytes(visual, 16)?;
    let lods = u16::from_le_bytes(v[4..6].try_into()?);
    let kind = u16::from_le_bytes(v[6..8].try_into()?);
    ensure!(
        lods > 0 && kind == 0,
        "first rooted model has unsupported visual set kind {kind}/LOD count {lods}"
    );
    let geo = c.required(c.required(visual, 12)?, 0)?;
    let header: GeometryWire = wire(c.bytes(geo, 48)?)?;
    ensure!(
        header.point_count > 0 && header.point_count <= i16::MAX as u16 && header.element_count > 0,
        "invalid geometry counts"
    );
    let lod_distance = c.floats::<1>(c.required(visual, 8)?, 1)?[0][0];
    let model_path = format!("level:/models/{path}");
    let mut materials = Vec::new();
    let mut material_ids = BTreeMap::new();
    let mut diagnostics=vec!["First LOD of one rooted model only; collision, scene lighting, animated materials, and spatial acceleration are not yet reconstructed.".into()];
    diagnostics.push(format!(
        "Visual set has {lods} LOD(s), kind {kind}; selected LOD 0 at distance {lod_distance}."
    ));
    if header.spatial_data != 0 {
        diagnostics.push("Geometry has source spatial acceleration data; this subset does not decode/reconstruct it.".into());
    }
    if c.pointer(po, 4)?.is_some() {
        diagnostics.push("PhysicalObject has collision data; collision import is outside this first-model slice.".into());
    }
    let radiosity = c.radiosity(ipo, 0, usize::from(lods), usize::from(header.point_count))?;
    if !radiosity.is_empty() {
        diagnostics.push(format!("Retained {} exact signed per-position radiosity colors for IPO LOD 0; full scene/dynamic lighting is outside this slice.", radiosity.len()));
    }
    if lods > 1 {
        diagnostics.push(format!(
            "Visual set has {lods} LODs; selected first LOD only."
        ));
    }
    fn material(
        c: &Import,
        gm: Address,
        base: &str,
        materials: &mut Vec<Material>,
        ids: &mut BTreeMap<Address, AssetPath>,
        diagnostics: &mut Vec<String>,
    ) -> Result<AssetRef<Material>> {
        if let Some(p) = ids.get(&gm) {
            return AssetRef::new(p.to_string());
        }
        let vm = c.required(gm, 0)?;
        let b = c.bytes(vm, 120)?;
        let path = AssetPath::new(format!("{base}/materials/{}", materials.len()))?;
        let float = |o| f32::from_bits(u32_at(b, o).expect("bounded material"));
        let vector = |o| [float(o), float(o + 4), float(o + 8), float(o + 12)];
        let mut texture_info = None;
        let texture = if let Some(t) = c.pointer(vm, 0x48)? {
            let tb = c.bytes(t, 188)?;
            let name = tb[100..180].split(|&b| b == 0).next().unwrap();
            let name = std::str::from_utf8(name).context("texture name is not UTF-8/ASCII")?;
            texture_info = Some(TextureInfo {
                source_name: Some(name.to_owned()),
                stored_size: [u32_at(tb, 16)?, u32_at(tb, 12)?],
                logical_size: [u32_at(tb, 52)?, u32_at(tb, 48)?],
                flags: u32_at(tb, 8)?,
                addressing: tb[181],
            });
            let name = name.replace('\\', "/");
            Some(AssetPath::new(format!("texture:/{name}"))?)
        } else {
            None
        };
        let count = u16::from_le_bytes(b[0x6c..0x6e].try_into()?);
        if count > 0 {
            diagnostics.push(format!(
                "{path}: {count} animated texture frames remain unsupported"
            ));
        }
        let sound_material = u32_at(c.bytes(gm, 16)?, 8)?;
        materials.push(Material {
            path: path.clone(),
            texture,
            texture_info,
            ambient: vector(4),
            diffuse: vector(20),
            specular: vector(36),
            color: vector(52),
            flags: u32_at(b, 0)?,
            scroll_position: [float(0x4c), float(0x50)],
            scroll_speed: [float(0x54), float(0x58)],
            scroll_mode: u32_at(b, 0x5c)?,
            sound_material,
        });
        ids.insert(gm, path.clone());
        AssetRef::new(path.to_string())
    }
    let types = c.short_arrays::<1>(c.required(geo, 24)?, header.element_count as usize)?;
    let table = c.required(geo, 28)?;
    let mut submeshes = Vec::new();
    for (i, [kind]) in types.iter().enumerate() {
        ensure!(
            *kind == 1,
            "unsupported geometric element {kind} in first model"
        );
        let el = c.required(table, i * 4)?;
        let h: TrianglesWire = wire(c.bytes(el, 32)?)?;
        let gm = c.required(el, 0)?;
        let m = material(
            &c,
            gm,
            &model_path,
            &mut materials,
            &mut material_ids,
            &mut diagnostics,
        )?;
        let normals = if let Some(a) = c.pointer(el, 16)? {
            c.floats::<3>(a, h.triangle_count as usize)?
        } else {
            Vec::new()
        };
        let indices = if h.point_count > 0 {
            c.short_arrays::<1>(c.required(el, 28)?, h.point_count as usize)?
                .into_iter()
                .map(|v| v[0])
                .collect()
        } else {
            Vec::new()
        };
        submeshes.push(Submesh {
            material: m,
            triangles: c.short_arrays::<3>(c.required(el, 8)?, h.triangle_count as usize)?,
            texcoords: c.floats::<2>(c.required(el, 20)?, h.uv_count as usize)?,
            uv_indices: c.short_arrays::<3>(c.required(el, 12)?, h.triangle_count as usize)?,
            face_normals: normals,
            vertex_indices: indices,
        });
    }
    let mut vertex_materials = Vec::new();
    if let Some(table) = c.pointer(geo, 12)? {
        for i in 0..header.point_count as usize {
            let gm = c.required(table, i * 4)?;
            vertex_materials.push(material(
                &c,
                gm,
                &model_path,
                &mut materials,
                &mut material_ids,
                &mut diagnostics,
            )?);
        }
    }
    let model = Model {
        path: AssetPath::new(model_path)?,
        name: path.clone(),
        lod_distance,
        positions: c.floats::<3>(c.required(geo, 4)?, header.point_count as usize)?,
        normals: if let Some(a) = c.pointer(geo, 8)? {
            c.floats::<3>(a, header.point_count as usize)?
        } else {
            Vec::new()
        },
        radiosity,
        submeshes,
        vertex_materials,
        instance_transforms: transforms.clone(),
    };
    model.validate()?;
    Ok(ModelImport {
        model,
        materials,
        source,
        diagnostics,
    })
}

/// Explicit layout inputs for references owned by other reconstructed assets.
#[derive(Debug, Clone, Copy)]
pub struct MaterialTarget {
    pub address: u32,
    pub block: (u8, u8),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmittedRange {
    pub name: String,
    pub offset: usize,
    pub length: usize,
}
#[derive(Debug, Clone)]
pub struct GeometryExport {
    /// Standalone controlled-layout geometry records/buffers, not a complete loadable SNA.
    pub bytes: Vec<u8>,
    pub rtb: Vec<u8>,
    pub ranges: Vec<EmittedRange>,
    pub pointer_count: usize,
    pub coverage: Vec<String>,
}
struct Layout {
    bytes: Vec<u8>,
    ranges: Vec<EmittedRange>,
    rows: Vec<RelocRow>,
    base: u32,
}
impl Layout {
    fn allocate(&mut self, name: impl Into<String>, bytes: &[u8]) -> Result<usize> {
        let aligned = self
            .bytes
            .len()
            .checked_add(3)
            .ok_or_else(|| anyhow!("layout overflow"))?
            & !3;
        ensure!(
            aligned + bytes.len() <= MAX_BLOCK,
            "geometry layout too large"
        );
        self.bytes.resize(aligned, 0);
        self.bytes.extend_from_slice(bytes);
        self.ranges.push(EmittedRange {
            name: name.into(),
            offset: aligned,
            length: bytes.len(),
        });
        Ok(aligned)
    }
    fn pointer(&mut self, site: usize, offset: usize) -> Result<()> {
        let address = self
            .base
            .checked_add(offset.try_into()?)
            .ok_or_else(|| anyhow!("pointer overflow"))?;
        self.external_pointer(
            site,
            MaterialTarget {
                address,
                block: (6, 2),
            },
        )
    }
    fn external_pointer(&mut self, site: usize, target: MaterialTarget) -> Result<()> {
        ensure!(target.address != 0, "non-null asset resolved to zero");
        self.bytes
            .get_mut(site..site + 4)
            .ok_or_else(|| anyhow!("invalid emission site"))?
            .copy_from_slice(&target.address.to_le_bytes());
        self.rows.push(RelocRow {
            value: self
                .base
                .checked_add(site.try_into()?)
                .ok_or_else(|| anyhow!("site overflow"))?,
            module: target.block.0,
            id: target.block.1,
            unknown: [0; 2],
        });
        Ok(())
    }
}
fn float_data<const N: usize>(values: &[[f32; N]]) -> Vec<u8> {
    values
        .iter()
        .flatten()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect()
}
fn short_data<const N: usize>(values: &[[u16; N]]) -> Vec<u8> {
    values
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
/// Reconstruct the verified geometry subset from useful values with caller-supplied material layout.
/// Counts, buffer placement and pointer sites are derived. Spatial acceleration and other domain
/// records are explicitly outside this fragment; it is never presented as whole-package parity.
pub fn write_geometry(
    model: &Model,
    materials: &BTreeMap<AssetPath, MaterialTarget>,
    base: u32,
) -> Result<GeometryExport> {
    model.validate()?;
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "geometry base must be nonzero/aligned"
    );
    let mut out = Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
    };
    let header = GeometryWire {
        point_count: model.positions.len().try_into()?,
        point_reserved: 0,
        points: 0,
        normals: 0,
        point_materials: 0,
        unknown_10: 0,
        element_count: model.submeshes.len().try_into()?,
        element_reserved: 0,
        element_types: 0,
        elements: 0,
        spatial_data: 0,
        unknown_tail: [0; 3],
    };
    out.allocate("geometry", &encoded(&header)?)?;
    let p = out.allocate("positions", &float_data(&model.positions))?;
    out.pointer(4, p)?;
    if !model.normals.is_empty() {
        let p = out.allocate("normals", &float_data(&model.normals))?;
        out.pointer(8, p)?;
    }
    if !model.submeshes.is_empty() {
        let types = vec![[1u16]; model.submeshes.len()];
        let p = out.allocate("element_types", &short_data(&types))?;
        out.pointer(24, p)?;
        let table = out.allocate("elements", &vec![0; model.submeshes.len() * 4])?;
        out.pointer(28, table)?;
        for (i, sm) in model.submeshes.iter().enumerate() {
            let wire = TrianglesWire {
                material: 0,
                triangle_count: sm.triangles.len().try_into()?,
                uv_count: sm.texcoords.len().try_into()?,
                triangles: 0,
                uv_indices: 0,
                normals: 0,
                uvs: 0,
                point_count: sm.vertex_indices.len().try_into()?,
                point_reserved: 0,
                points: 0,
            };
            let h = out.allocate(format!("submeshes/{i}"), &encoded(&wire)?)?;
            out.pointer(table + i * 4, h)?;
            let target = *materials
                .get(&sm.material.path)
                .ok_or_else(|| anyhow!("material has no export layout: {}", sm.material.path))?;
            out.external_pointer(h, target)?;
            for (name, field, data) in [
                ("triangles", 8, short_data(&sm.triangles)),
                ("uv_indices", 12, short_data(&sm.uv_indices)),
                ("face_normals", 16, float_data(&sm.face_normals)),
                ("texcoords", 20, float_data(&sm.texcoords)),
                (
                    "vertex_indices",
                    28,
                    sm.vertex_indices
                        .iter()
                        .flat_map(|v| v.to_le_bytes())
                        .collect(),
                ),
            ] {
                if !data.is_empty() {
                    let p = out.allocate(format!("submeshes/{i}/{name}"), &data)?;
                    out.pointer(h + field, p)?;
                }
            }
        }
    }
    if !model.vertex_materials.is_empty() {
        let p = out.allocate(
            "vertex_materials",
            &vec![0; model.vertex_materials.len() * 4],
        )?;
        out.pointer(12, p)?;
        for (i, m) in model.vertex_materials.iter().enumerate() {
            out.external_pointer(
                p + i * 4,
                *materials
                    .get(&m.path)
                    .ok_or_else(|| anyhow!("point material has no export layout: {}", m.path))?,
            )?;
        }
    }
    out.rows.sort_by_key(|r| r.value);
    let mut row_bytes = Vec::new();
    for row in &out.rows {
        row_bytes.extend(encoded(row)?);
    }
    let mut rtb = vec![1];
    rtb.extend(encoded(&RelocGroup {
        module: 6,
        id: 2,
        count: out.rows.len().try_into()?,
    })?);
    if !out.rows.is_empty() {
        rtb.extend(encode_envelope(&row_bytes)?);
    }
    Ok(GeometryExport{bytes:out.bytes,rtb,ranges:out.ranges,pointer_count:out.rows.len(),coverage:vec!["Geometry header (0x30), point/normal arrays, triangle groups (0x20), UV mapping, point subsets and material links regenerated in a controlled layout.".into(),"Source allocator bookkeeping, spatial acceleration, unknown geometry state, materials themselves, IPO/scene records and full SNA assembly are outside this fragment.".into()]})
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn model() -> Model {
        Model {
            path: AssetPath::new("level:/models/triangle").unwrap(),
            name: "triangle".into(),
            lod_distance: 0.,
            positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            normals: vec![[0., 0., 1.]; 3],
            radiosity: Vec::new(),
            submeshes: vec![Submesh {
                material: AssetRef::new("level:/materials/red").unwrap(),
                triangles: vec![[0, 1, 2]],
                texcoords: vec![[0., 0.], [1., 0.], [0., 1.]],
                uv_indices: vec![[2, 1, 0]],
                face_normals: vec![[0., 0., 1.]],
                vertex_indices: vec![0, 1, 2],
            }],
            vertex_materials: Vec::new(),
            instance_transforms: Vec::new(),
        }
    }
    #[test]
    fn controlled_layout_derives_pointers_and_preserves_distinct_uv_indices() -> Result<()> {
        let mut m = model();
        let targets = BTreeMap::from([(
            m.submeshes[0].material.path.clone(),
            MaterialTarget {
                address: 0x2000,
                block: (6, 1),
            },
        )]);
        let a = write_geometry(&m, &targets, 0x1000)?;
        let geom: GeometryWire = wire(&a.bytes)?;
        assert_eq!(geom.point_count, 3);
        assert_eq!(geom.points, 0x1030);
        assert_eq!(geom.spatial_data, 0);
        let groups = reloc(&a.rtb)?;
        let rows = &groups[&BlockKey(6, 2)];
        assert_eq!(rows.len(), a.pointer_count);
        for row in rows {
            let at = (row.value - 0x1000) as usize;
            let target = u32_at(&a.bytes, at)?;
            if row.id == 1 {
                assert_eq!(target, 0x2000);
            } else {
                assert!(target >= 0x1000 && target < 0x1000 + a.bytes.len() as u32);
            }
        }
        let uv = a
            .ranges
            .iter()
            .find(|r| r.name == "submeshes/0/uv_indices")
            .unwrap();
        assert_eq!(
            &a.bytes[uv.offset..uv.offset + uv.length],
            &[2, 0, 1, 0, 0, 0]
        );
        m.positions[1][0] = 2.;
        m.positions.push([3., 3., 3.]);
        m.normals.push([0., 0., 1.]);
        let b = write_geometry(&m, &targets, 0x1000)?;
        let changed: GeometryWire = wire(&b.bytes)?;
        assert_eq!(changed.point_count, 4);
        assert_ne!(changed.normals, geom.normals);
        assert_eq!(f32::from_bits(u32_at(&b.bytes, 0x30 + 12)?), 2.);
        Ok(())
    }
    #[test]
    fn malformed_framing_and_indices_fail() {
        assert!(sna(&[5, 1, 0]).is_err());
        assert!(reloc(&[1, 6, 2, 0xff, 0xff, 0xff, 0x7f]).is_err());
        let mut m = model();
        m.submeshes[0].uv_indices[0][0] = 42;
        assert!(m.validate().is_err());
    }
    #[test]
    fn verified_hype_records_do_not_include_adjacent_allocations() -> Result<()> {
        assert_eq!(
            encoded(&InstantiatedObjectWire {
                physical_object: 0,
                radiosity: 0
            })?
            .len(),
            8
        );
        assert_eq!(
            encoded(&RadiosityWire {
                lod_count: 1,
                reserved: 0,
                lods: 0
            })?
            .len(),
            8
        );
        assert_eq!(
            encoded(&RadiosityLodWire {
                vertex_count: 1,
                reserved: 0,
                colors: 0
            })?
            .len(),
            8
        );
        assert_eq!(
            encoded(&RadiosityColorWire {
                components: [-32768, 32767, -57, 0]
            })?
            .len(),
            8
        );
        assert_eq!(
            encoded(&GeometryWire {
                point_count: 1,
                point_reserved: 0,
                points: 0,
                normals: 0,
                point_materials: 0,
                unknown_10: 0,
                element_count: 0,
                element_reserved: 0,
                element_types: 0,
                elements: 0,
                spatial_data: 0,
                unknown_tail: [0; 3]
            })?
            .len(),
            0x30
        );
        assert_eq!(
            encoded(&TrianglesWire {
                material: 0,
                triangle_count: 0,
                uv_count: 0,
                triangles: 0,
                uv_indices: 0,
                normals: 0,
                uvs: 0,
                point_count: 0,
                point_reserved: 0,
                points: 0
            })?
            .len(),
            0x20
        );
        Ok(())
    }
}

#[derive(BinWrite)]
#[bw(little)]
struct PhysicalObjectWire {
    visual_set: u32,
    collision_set: u32,
    bounding_volume: u32,
}
#[derive(BinWrite)]
#[bw(little)]
struct VisualSetWire {
    flags: u32,
    lods: u16,
    kind: u16,
    distances: u32,
    objects: u32,
}
#[derive(BinWrite)]
#[bw(little)]
struct GameMaterialWire {
    visual: u32,
    mechanics: u32,
    sound: u32,
    collision: u32,
}
#[derive(BinWrite)]
#[bw(little)]
struct VisualMaterialWire {
    flags: u32,
    ambient: [f32; 4],
    diffuse: [f32; 4],
    specular: [f32; 4],
    color: [f32; 4],
    unknown_44: u32,
    texture: u32,
    scroll_position: [f32; 2],
    scroll_speed: [f32; 2],
    scroll_mode: u32,
    refresh: u32,
    animation_first: u32,
    animation_current: u32,
    animation_count: u16,
    reserved: u16,
    animation_time: u32,
    properties: [u8; 4],
    runtime_tail: [u32; 3],
}
#[derive(BinWrite)]
#[bw(little)]
struct TextureInfoWire {
    runtime_head: [u32; 2],
    flags: u32,
    stored_height: u32,
    stored_width: u32,
    unknown_14: u32,
    runtime_state: [u32; 6],
    logical_height: u32,
    logical_width: u32,
    remaining_state: [u32; 11],
    name: [u8; 80],
    sample_flags: u8,
    addressing: u8,
    reserved: u16,
    runtime_tail: u32,
}
#[derive(Debug, Clone)]
pub struct ModelFragment {
    pub bytes: Vec<u8>,
    pub rtb: Vec<u8>,
    pub ptx: Vec<u8>,
    pub rtt: Vec<u8>,
    pub ranges: Vec<EmittedRange>,
    pub pointer_count: usize,
    pub physical_object_offset: usize,
    /// Selected instance wrapper, including its optional per-LOD lighting pointer.
    pub instantiated_object_offset: usize,
    pub coverage: Vec<String>,
}
fn emit_rtb(rows: &[RelocRow]) -> Result<Vec<u8>> {
    let mut bytes = vec![1];
    bytes.extend(encoded(&RelocGroup {
        module: 6,
        id: 2,
        count: rows.len().try_into()?,
    })?);
    if !rows.is_empty() {
        let mut data = Vec::new();
        for r in rows {
            data.extend(encoded(r)?);
        }
        bytes.extend(encode_envelope(&data)?);
    }
    Ok(bytes)
}
/// Source-free model/material fragment for focused reconstruction and editing tests. This includes
/// generated material, TextureInfo/name, IPO/PO/LOD, radiosity and pointer-file relationships, but not a world SNA.
/// Runtime/unknown material state and collision/spatial data are explicitly unsupported.
pub fn write_model_fragment(
    model: &Model,
    materials: &[Material],
    base: u32,
) -> Result<ModelFragment> {
    model.validate()?;
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "fragment base must be aligned/nonzero"
    );
    let mut out = Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
    };
    let po = out.allocate(
        "physical_object",
        &encoded(&PhysicalObjectWire {
            visual_set: 0,
            collision_set: 0,
            bounding_volume: 0,
        })?,
    )?;
    let ipo = out.allocate(
        "instantiated_object",
        &encoded(&InstantiatedObjectWire {
            physical_object: 0,
            radiosity: 0,
        })?,
    )?;
    out.pointer(ipo, po)?;
    if !model.radiosity.is_empty() {
        let isi = out.allocate(
            "radiosity/header",
            &encoded(&RadiosityWire {
                lod_count: 1,
                reserved: 0,
                lods: 0,
            })?,
        )?;
        let lod = out.allocate(
            "radiosity/lod",
            &encoded(&RadiosityLodWire {
                vertex_count: model.radiosity.len().try_into()?,
                reserved: 0,
                colors: 0,
            })?,
        )?;
        let mut colors = Vec::with_capacity(model.radiosity.len() * 8);
        for &components in &model.radiosity {
            colors.extend(encoded(&RadiosityColorWire { components })?);
        }
        let colors = out.allocate("radiosity/colors", &colors)?;
        out.pointer(ipo + 4, isi)?;
        out.pointer(isi + 4, lod)?;
        out.pointer(lod + 4, colors)?;
    }
    let vs = out.allocate(
        "visual_set",
        &encoded(&VisualSetWire {
            flags: 0,
            lods: 1,
            kind: 0,
            distances: 0,
            objects: 0,
        })?,
    )?;
    out.pointer(po, vs)?;
    let distance = out.allocate("lod_distance", &model.lod_distance.to_bits().to_le_bytes())?;
    out.pointer(vs + 8, distance)?;
    let object = out.allocate("lod_object", &[0; 4])?;
    out.pointer(vs + 12, object)?;
    let mut targets = BTreeMap::new();
    let mut textures: BTreeMap<(AssetPath, String), (usize, TextureInfo)> = BTreeMap::new();
    for m in materials {
        m.validate()?;
        ensure!(
            !targets.contains_key(&m.path),
            "duplicate material definition: {}",
            m.path
        );
        let gm = out.allocate(
            format!("materials/{}/game", m.path.file()),
            &encoded(&GameMaterialWire {
                visual: 0,
                mechanics: 0,
                sound: m.sound_material,
                collision: 0,
            })?,
        )?;
        let vm = out.allocate(
            format!("materials/{}/visual", m.path.file()),
            &encoded(&VisualMaterialWire {
                flags: m.flags,
                ambient: m.ambient,
                diffuse: m.diffuse,
                specular: m.specular,
                color: m.color,
                unknown_44: 0,
                texture: 0,
                scroll_position: m.scroll_position,
                scroll_speed: m.scroll_speed,
                scroll_mode: m.scroll_mode,
                refresh: 0,
                animation_first: 0,
                animation_current: 0,
                animation_count: 0,
                reserved: 0,
                animation_time: 0,
                properties: [0; 4],
                runtime_tail: [0; 3],
            })?,
        )?;
        out.pointer(gm, vm)?;
        if let (Some(reference), Some(info)) = (&m.texture, &m.texture_info) {
            let name = info
                .source_name
                .clone()
                .unwrap_or_else(|| reference.file().replace('/', "\\"));
            let texture_key = (reference.clone(), name.clone());
            let t = if let Some((offset, previous)) = textures.get(&texture_key) {
                ensure!(
                    previous == info,
                    "shared texture has conflicting sampling metadata"
                );
                *offset
            } else {
                ensure!(name.len() < 80, "Hype TextureInfo name exceeds 79 bytes");
                let mut raw_name = [0; 80];
                raw_name[..name.len()].copy_from_slice(name.as_bytes());
                let t = out.allocate(
                    format!("textures/{}", reference.file()),
                    &encoded(&TextureInfoWire {
                        runtime_head: [0; 2],
                        flags: info.flags,
                        stored_height: info.stored_size[1],
                        stored_width: info.stored_size[0],
                        unknown_14: 0,
                        runtime_state: [0; 6],
                        logical_height: info.logical_size[1],
                        logical_width: info.logical_size[0],
                        remaining_state: [0; 11],
                        name: raw_name,
                        sample_flags: 0,
                        addressing: info.addressing,
                        reserved: 0,
                        runtime_tail: 0,
                    })?,
                )?;
                textures.insert(texture_key, (t, info.clone()));
                t
            };
            out.pointer(vm + 0x48, t)?;
        }
        targets.insert(
            m.path.clone(),
            MaterialTarget {
                address: base
                    .checked_add(gm.try_into()?)
                    .ok_or_else(|| anyhow!("fragment base overflow"))?,
                block: (6, 2),
            },
        );
    }
    let start = (out.bytes.len() + 3) & !3;
    out.bytes.resize(start, 0);
    let g = write_geometry(
        model,
        &targets,
        base.checked_add(start.try_into()?)
            .ok_or_else(|| anyhow!("geometry base overflow"))?,
    )?;
    out.bytes.extend_from_slice(&g.bytes);
    ensure!(out.bytes.len() <= MAX_BLOCK, "combined fragment too large");
    out.ranges
        .extend(g.ranges.into_iter().map(|r| EmittedRange {
            name: r.name,
            offset: r.offset + start,
            length: r.length,
        }));
    for rows in reloc(&g.rtb)?.into_values() {
        out.rows.extend(rows);
    }
    out.pointer(object, start)?;
    out.rows.sort_by_key(|r| r.value);
    let rtb = emit_rtb(&out.rows)?;
    let mut texture_pointers = Vec::new();
    let mut texture_rows = Vec::new();
    for (offset, _) in textures.values() {
        let address = base
            .checked_add((*offset).try_into()?)
            .ok_or_else(|| anyhow!("texture address overflow"))?;
        texture_pointers.push(address);
        texture_rows.push(RelocRow {
            value: address,
            module: 6,
            id: 2,
            unknown: [0; 2],
        });
    }
    ensure!(
        texture_pointers.len() <= 1024,
        "fragment exceeds Hype texture table capacity"
    );
    let loaded_textures = texture_pointers.len() as u32;
    texture_pointers.resize(1024, 0);
    let memory_channels = (0..1024)
        .map(|i| if i < loaded_textures { 1 } else { 0 })
        .collect();
    let ptx = encoded(&LevelTexturesWire {
        capacity: 1024,
        level_slots: 1024,
        pointers: texture_pointers,
        loaded_textures,
        memory_channels,
        current_channel: 1,
    })?;
    // Montreal RTT entries identify pointer VALUES, unlike RTB pointer-site addresses.
    let mut rtt = vec![1];
    rtt.extend(encoded(&RelocGroup {
        module: 5,
        id: 1,
        count: texture_rows.len().try_into()?,
    })?);
    if !texture_rows.is_empty() {
        let mut rows = Vec::new();
        for r in texture_rows {
            rows.extend(encoded(&r)?);
        }
        rtt.extend(encode_envelope(&rows)?);
    }
    Ok(ModelFragment{bytes:out.bytes,rtb,ptx,rtt,ranges:out.ranges,pointer_count:out.rows.len(),physical_object_offset:po,instantiated_object_offset:ipo,coverage:vec!["Rebuilt supported geometry arrays, material coefficients/texture names, source texture dimensions/addressing, point-material sharing, one IPO/PO/visual-set/LOD with exact per-position radiosity, and emitted RTB/RTT references.".into(),"Controlled fragment only: unknown material/texture runtime fields are initialized, not claimed byte-exact; collision, spatial acceleration, other LODs, scene lighting/matrices/world records, original Fix/level texture slot/channel placement, and source allocator bookkeeping are outside this export.".into()]})
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    #[test]
    fn generated_sna_block_preserves_values_and_resolves_generated_relocations() -> Result<()> {
        let m = super::tests::model();
        let targets = BTreeMap::from([(
            m.submeshes[0].material.path.clone(),
            MaterialTarget {
                address: 0x2000,
                block: (6, 1),
            },
        )]);
        let fragment = write_geometry(&m, &targets, 0x1000)?;
        let mut framed = write_sna_block(&fragment.bytes, (6, 2), 0x1000)?;
        let header: SnaInfo = wire(&framed[6..22])?;
        assert_eq!(
            header.capacity_end,
            0x1000 + fragment.bytes.len() as u32 - 1
        );
        assert_eq!(header.high_water, header.capacity_end + 1);
        assert_eq!(header.free_list, 0);
        framed.extend(write_sna_block(&[42; 16], (6, 1), 0x2000)?);
        let blocks = sna(&framed)?;
        assert_eq!(blocks[&BlockKey(6, 2)].data, fragment.bytes);
        let mut pointers = PointerMap::new();
        merge_relocations(&mut pointers, Owner::Level, reloc(&fragment.rtb)?)?;
        let ctx = Import {
            actual: blocks.keys().map(|&k| (k, Owner::Level)).collect(),
            blocks: BTreeMap::from([(Owner::Level, blocks)]),
            pointers,
            gpt: vec![],
            rtp: BTreeMap::new(),
            ptx: vec![],
            rtt: BTreeMap::new(),
        };
        let geo = Address {
            owner: Owner::Level,
            key: BlockKey(6, 2),
            offset: 0,
        };
        assert_eq!(
            ctx.floats::<3>(ctx.required(geo, 4)?, m.positions.len())?,
            m.positions
        );
        let table = ctx.required(geo, 28)?;
        let triangle = ctx.required(table, 0)?;
        assert_eq!(ctx.bytes(ctx.required(triangle, 0)?, 16)?, &[42; 16]);
        assert_eq!(
            ctx.short_arrays::<3>(ctx.required(triangle, 12)?, 1)?,
            m.submeshes[0].uv_indices
        );
        let mut changed = m.clone();
        changed.positions[0][0] = 7.;
        let edited = write_geometry(&changed, &targets, 0x5000)?;
        let roundtrip = sna(&write_sna_block(&edited.bytes, (6, 2), 0x5000)?)?;
        assert_eq!(
            f32::from_le_bytes(roundtrip[&BlockKey(6, 2)].data[48..52].try_into()?),
            7.
        );
        assert!(write_sna_block(&[], (6, 2), 0x1000).is_err());
        assert!(write_sna_block(&[0; 16], (6, 2), 0x7ffffffc).is_err());
        assert!(sna(&framed[..framed.len() - 1]).is_err());
        Ok(())
    }
    #[test]
    fn fix_to_level_binding_sentinels_conflicts_and_missing_sites_are_explicit() -> Result<()> {
        let fk = BlockKey(6, 1);
        let lk = BlockKey(6, 2);
        let mut data = vec![0; 16];
        data[0..4].copy_from_slice(&0x9004u32.to_le_bytes());
        data[4..8].copy_from_slice(&0x9004u32.to_le_bytes());
        data[12..16].copy_from_slice(&0x8000u32.to_le_bytes());
        let group = |site, target: BlockKey| {
            BTreeMap::from([(
                fk,
                vec![RelocRow {
                    value: site,
                    module: target.0,
                    id: target.1,
                    unknown: [0; 2],
                }],
            )])
        };
        let mut pointers = PointerMap::new();
        merge_relocations(&mut pointers, Owner::Fix, group(0x2000, BlockKey(255, 255)))?;
        merge_relocations(&mut pointers, Owner::Fix, group(0x2000, lk))?;
        merge_relocations(&mut pointers, Owner::Fix, group(0x2000, BlockKey(255, 255)))?;
        assert_eq!(pointers[&(Owner::Fix, fk, 0x2000)], lk);
        assert!(merge_relocations(&mut pointers, Owner::Fix, group(0x2000, fk)).is_err());
        merge_relocations(&mut pointers, Owner::Fix, group(0x200c, lk))?;
        let mut c = Import {
            blocks: BTreeMap::from([
                (
                    Owner::Fix,
                    BTreeMap::from([
                        (fk, Block { base: 0x2000, data }),
                        (
                            lk,
                            Block {
                                base: 0x9000,
                                data: vec![],
                            },
                        ),
                    ]),
                ),
                (
                    Owner::Level,
                    BTreeMap::from([(
                        lk,
                        Block {
                            base: 0x1000,
                            data: vec![42; 16],
                        },
                    )]),
                ),
            ]),
            actual: BTreeMap::from([(fk, Owner::Fix), (lk, Owner::Level)]),
            pointers,
            gpt: vec![],
            rtp: BTreeMap::new(),
            ptx: vec![],
            rtt: BTreeMap::new(),
        };
        let origin = Address {
            owner: Owner::Fix,
            key: fk,
            offset: 0,
        };
        assert_eq!(
            c.required(origin, 0)?,
            Address {
                owner: Owner::Level,
                key: lk,
                offset: 4
            }
        );
        assert!(c.pointer(origin, 4).is_err());
        assert!(c.pointer(origin, 8)?.is_none());
        assert!(c.pointer(origin, 12).is_err());
        merge_relocations(
            &mut c.pointers,
            Owner::Fix,
            group(0x2004, BlockKey(255, 255)),
        )?;
        assert!(c.pointer(origin, 4)?.is_none());
        let duplicates = [
            RelocRow {
                value: 0x1000,
                module: 6,
                id: 1,
                unknown: [0; 2],
            },
            RelocRow {
                value: 0x1000,
                module: 6,
                id: 2,
                unknown: [0; 2],
            },
        ];
        assert!(pointer_file(&emit_rtb(&duplicates)?).is_err());
        Ok(())
    }
    #[test]
    fn ptx_validates_real_pointer_slots_and_not_header_or_channel_words() -> Result<()> {
        let table = LevelTexturesWire {
            capacity: 1024,
            level_slots: 3,
            pointers: vec![0x1004, 0, 0xdeadbeef],
            loaded_textures: 1,
            memory_channels: vec![2; 1024],
            current_channel: 2,
        };
        let key = BlockKey(6, 2);
        let mut c = Import {
            blocks: BTreeMap::from([(
                Owner::Level,
                BTreeMap::from([(
                    key,
                    Block {
                        base: 0x1000,
                        data: vec![42; 16],
                    },
                )]),
            )]),
            actual: BTreeMap::from([(key, Owner::Level)]),
            pointers: PointerMap::new(),
            gpt: vec![],
            rtp: BTreeMap::new(),
            ptx: encoded(&table)?,
            rtt: BTreeMap::from([(0x1004, key), (0xdeadbeef, BlockKey(255, 255))]),
        };
        assert_eq!(c.texture_addresses()?.len(), 1);
        c.rtt.remove(&0x1004);
        assert!(c.texture_addresses().is_err());
        c.rtt.insert(0x1004, key);
        c.ptx.push(0);
        assert!(c.texture_addresses().is_err());
        c.ptx.pop();
        c.ptx[8..12].copy_from_slice(&0x1020u32.to_le_bytes());
        c.rtt.insert(0x1020, key);
        assert!(c.texture_addresses().is_err());
        c.ptx = encoded(&table)?;
        c.ptx[4..8].copy_from_slice(&1025u32.to_le_bytes());
        assert!(c.texture_addresses().is_err());
        assert!(level_textures(&[0; 7]).is_err());
        Ok(())
    }
    #[test]
    #[ignore = "requires local Hype source blocks; run under the overall suite deadline"]
    fn real_blocks_and_relocation_rows_match_independent_lib_lzo_observations() -> Result<()> {
        use sha2::{Digest, Sha256};
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
        let level = game.join("Gamedata/World/Levels/astrolabe");
        let blocks = sna(&file(&level.join("astrolabe.sna"))?)?;
        for (key, length, digest) in [
            (
                BlockKey(5, 1),
                904349,
                "53bbd8d87ce6e4a6c12af64028f13ea7c793489ac61d362ce78468c6b2ec8897",
            ),
            (
                BlockKey(6, 2),
                1360321,
                "7c856793e4bdbcf55db34f9f1eb0630112736327ba8769f52938ff2d68e87532",
            ),
            (
                BlockKey(17, 1),
                103461,
                "beb80a6b1765224016c80b03f5ac6621f91e07df845b87378c3259595f85b8aa",
            ),
        ] {
            assert_eq!(blocks[&key].data.len(), length);
            assert_eq!(format!("{:x}", Sha256::digest(&blocks[&key].data)), digest);
        }
        for (ext, key, count, first, digest) in [
            (
                "rtb",
                BlockKey(5, 1),
                51616,
                0x09d00160,
                "bda4b8873e7817b2ec27f782d8e0e91549dd2ae3036f648585d907b195697db4",
            ),
            (
                "rtb",
                BlockKey(6, 2),
                15926,
                0x09370048,
                "7b553e238c3f4f98153ed975f17d38cc1fb564a13b9fb8702358c1d2420a92ab",
            ),
            (
                "rtb",
                BlockKey(17, 1),
                2380,
                0x09000024,
                "550ac3138dbc5b16e84911683a65b1bb09d46119283abbd894fcaa4aa9c9a5bd",
            ),
            (
                "rtp",
                BlockKey(5, 1),
                86,
                0x01bded28,
                "d1aaab8e8e2b78237bfe432d5213a7860e6d0868d60e868d78cb590875c2d9c3",
            ),
            (
                "rtt",
                BlockKey(5, 1),
                125,
                0x093f7190,
                "c94047490d5fd46d11f4418e3f177c6500757951582cb566894316bda53f4e28",
            ),
        ] {
            let groups = reloc(&file(&level.join(format!("astrolabe.{ext}")))?)?;
            let rows = &groups[&key];
            assert_eq!(rows.len(), count);
            assert_eq!(rows[0].value, first);
            let mut data = Vec::new();
            for row in rows {
                data.extend(encoded(row)?);
            }
            assert_eq!(format!("{:x}", Sha256::digest(&data)), digest);
        }
        let table = level_textures(&file(&level.join("astrolabe.ptx"))?)?;
        assert_eq!(
            (
                table.capacity,
                table.level_slots,
                table.loaded_textures,
                table.current_channel
            ),
            (1024, 589, 560, 2)
        );
        assert_eq!(table.pointers.iter().filter(|&&p| p != 0).count(), 125);
        Ok(())
    }

    #[test]
    fn radiosity_uses_signed_values_and_checks_selected_lod_and_buffer_bounds() -> Result<()> {
        let key = BlockKey(6, 2);
        // IPO -> ISI -> LOD -> one color. No engine allocator padding is included.
        let mut bytes = Vec::new();
        bytes.extend(encoded(&InstantiatedObjectWire {
            physical_object: 0,
            radiosity: 0x1008,
        })?);
        bytes.extend(encoded(&RadiosityWire {
            lod_count: 1,
            reserved: 0,
            lods: 0x1010,
        })?);
        bytes.extend(encoded(&RadiosityLodWire {
            vertex_count: 1,
            reserved: 0,
            colors: 0x1018,
        })?);
        bytes.extend(encoded(&RadiosityColorWire {
            components: [396, -238, i16::MIN, i16::MAX],
        })?);
        let mut c = Import {
            blocks: BTreeMap::from([(
                Owner::Level,
                BTreeMap::from([(
                    key,
                    Block {
                        base: 0x1000,
                        data: bytes,
                    },
                )]),
            )]),
            actual: BTreeMap::from([(key, Owner::Level)]),
            pointers: [0x1004, 0x100c, 0x1014]
                .into_iter()
                .map(|site| ((Owner::Level, key, site), key))
                .collect(),
            gpt: vec![],
            rtp: BTreeMap::new(),
            ptx: vec![],
            rtt: BTreeMap::new(),
        };
        let ipo = Address {
            owner: Owner::Level,
            key,
            offset: 0,
        };
        assert_eq!(
            c.radiosity(ipo, 0, 1, 1)?,
            [[396, -238, i16::MIN, i16::MAX]]
        );
        assert!(c.radiosity(ipo, 1, 1, 1).is_err());
        assert!(c.radiosity(ipo, 0, 2, 1).is_err());
        assert!(c.radiosity(ipo, 0, 1, 2).is_err());
        c.blocks
            .get_mut(&Owner::Level)
            .unwrap()
            .get_mut(&key)
            .unwrap()
            .data
            .pop();
        assert!(c.radiosity(ipo, 0, 1, 1).is_err());
        assert!(wire::<RadiosityWire>(&[255, 255, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(wire::<RadiosityLodWire>(&[1, 0, 1, 0, 0, 0, 0, 0]).is_err());
        Ok(())
    }
    #[test]
    fn source_local_bases_resolve_shared_blocks_and_invalid_references_fail() -> Result<()> {
        let mut level = vec![0; 16];
        level[0..4].copy_from_slice(&0x8004u32.to_le_bytes());
        level[8..12].copy_from_slice(&0x9000u32.to_le_bytes());
        let mut fix = BTreeMap::new();
        fix.insert(
            BlockKey(6, 1),
            Block {
                base: 0x2000,
                data: vec![42; 16],
            },
        );
        let mut lvl = BTreeMap::new();
        lvl.insert(
            BlockKey(6, 1),
            Block {
                base: 0x8000,
                data: Vec::new(),
            },
        );
        lvl.insert(
            BlockKey(6, 2),
            Block {
                base: 0x1000,
                data: level,
            },
        );
        let c = Import {
            blocks: BTreeMap::from([(Owner::Fix, fix), (Owner::Level, lvl)]),
            actual: BTreeMap::from([(BlockKey(6, 1), Owner::Fix), (BlockKey(6, 2), Owner::Level)]),
            pointers: BTreeMap::from([
                ((Owner::Level, BlockKey(6, 2), 0x1000), BlockKey(6, 1)),
                ((Owner::Level, BlockKey(6, 2), 0x1008), BlockKey(6, 1)),
            ]),
            gpt: vec![],
            rtp: BTreeMap::new(),
            ptx: vec![],
            rtt: BTreeMap::new(),
        };
        let source = Address {
            owner: Owner::Level,
            key: BlockKey(6, 2),
            offset: 0,
        };
        let target = c.required(source, 0)?;
        assert_eq!(
            target,
            Address {
                owner: Owner::Fix,
                key: BlockKey(6, 1),
                offset: 4
            }
        );
        assert_eq!(c.bytes(target, 1)?, [42]);
        assert!(c.pointer(source, 4)?.is_none());
        assert!(c.pointer(source, 8).is_err());
        Ok(())
    }
}
