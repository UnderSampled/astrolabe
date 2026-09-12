//! Bounded SNA framing, relocation tables, and binary record helpers.
use super::super::compression::{decode_envelope, encode_envelope};
use super::records::*;
use anyhow::{Context, Result, anyhow, ensure};
use binrw::{BinRead, BinWrite};
use std::{collections::BTreeMap, io::Cursor, path::Path};
pub(super) const MAX_FILE: usize = 64 * 1024 * 1024;
pub(super) const MAX_BLOCK: usize = 128 * 1024 * 1024;
const MAX_TOTAL: usize = 512 * 1024 * 1024;
pub(super) fn level_textures(bytes: &[u8]) -> Result<LevelTexturesWire> {
    let mut cursor = Cursor::new(bytes);
    let table = LevelTexturesWire::read_le(&mut cursor)?;
    ensure!(
        cursor.position() == bytes.len() as u64,
        "PTX trailing bytes"
    );
    Ok(table)
}
pub(super) fn wire<T: BinRead>(bytes: &[u8]) -> Result<T>
where
    for<'a> T::Args<'a>: Default,
{
    Ok(T::read_le(&mut Cursor::new(bytes))?)
}
pub(super) fn encoded<T: BinWrite>(value: &T) -> Result<Vec<u8>>
where
    for<'a> T::Args<'a>: Default,
{
    let mut c = Cursor::new(Vec::new());
    value.write_le(&mut c)?;
    Ok(c.into_inner())
}
pub(super) fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(o..o + 4)
            .ok_or_else(|| anyhow!("truncated u32"))?
            .try_into()?,
    ))
}
pub(super) fn file(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        path.metadata()?.len() <= MAX_FILE as u64,
        "file too large: {}",
        path.display()
    );
    std::fs::read(path).with_context(|| path.display().to_string())
}
pub(super) fn sna(bytes: &[u8]) -> Result<BTreeMap<BlockKey, Block>> {
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
pub(super) fn reloc(bytes: &[u8]) -> Result<BTreeMap<BlockKey, Vec<RelocRow>>> {
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
pub(super) type PointerMap = BTreeMap<(Owner, BlockKey, u32), BlockKey>;
pub(super) fn merge_relocations(
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
pub(super) fn pointer_file(bytes: &[u8]) -> Result<BTreeMap<u32, BlockKey>> {
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

/// RTB groups contain pointer sites; RTT groups contain pointer values.
/// The caller chooses the owning block and derives the rows from its current layout.
pub(super) fn emit_relocations(block: BlockKey, rows: &[RelocRow]) -> Result<Vec<u8>> {
    let mut bytes = vec![1];
    bytes.extend(encoded(&RelocGroup {
        module: block.0,
        id: block.1,
        count: rows.len().try_into()?,
    })?);
    if !rows.is_empty() {
        let mut data = Vec::new();
        for row in rows {
            data.extend(encoded(row)?);
        }
        bytes.extend(encode_envelope(&data)?);
    }
    Ok(bytes)
}
