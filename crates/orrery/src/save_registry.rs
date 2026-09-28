//! Bounded read-only view of Hype's SAI save-registration list.
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::collections::BTreeSet;

pub const REGISTRY: u32 = 0x0089_ad50;
pub const MODULE_BLOCK_TABLE: u32 = 0x0088_0f20;
pub const BLOCK_COUNTS: u32 = 0x0088_1340;
pub const MODULE_COUNT: u32 = 0x0072_4310;
pub const NODE_BYTES: usize = 36;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RegistryNode {
    pub address: u32,
    pub next: u32,
    pub previous: u32,
    pub owner: u32,
    pub source_address: u32,
    pub flags: u32,
    pub reserved_word: u32,
    pub snapshot_or_target: u32,
    pub size_or_pointer_module: u32,
    pub trailing_word: u32,
    pub mapped_source: Option<SourceLocation>,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SourceLocation {
    pub module: u8,
    pub block: u8,
    pub offset: u32,
    pub block_base: u32,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RegistrySnapshot {
    pub root: u32,
    pub first: u32,
    pub tail: u32,
    pub declared_count: u32,
    pub nodes: Vec<RegistryNode>,
}

fn u32at(b: &[u8], n: usize) -> Result<u32> {
    let bytes = b.get(n..n + 4).context("short registry word")?;
    Ok(u32::from_le_bytes(bytes.try_into()?))
}
fn word(read: &mut impl FnMut(u32, usize) -> Result<Vec<u8>>, a: u32) -> Result<u32> {
    u32at(&read(a, 4)?, 0)
}

/// Traverse at most `limit` 36-byte records. Validates the doubly-linked list,
/// header count/tail, owner pointer and source address against active MMG blocks.
pub fn read_registry(
    read: &mut impl FnMut(u32, usize) -> Result<Vec<u8>>,
    limit: usize,
) -> Result<RegistrySnapshot> {
    ensure!(
        limit > 0 && limit <= 100_000,
        "registry limit outside 1..100000"
    );
    let head = word(read, REGISTRY)?;
    let tail = word(read, REGISTRY + 4)?;
    let count = word(read, REGISTRY + 8)?;
    ensure!(
        count as usize <= limit,
        "registry count {count} exceeds bound {limit}"
    );
    let modules = word(read, MODULE_COUNT)? as usize;
    ensure!(modules <= 256, "implausible MMG module count {modules}");
    let mut blocks = Vec::new();
    for module in 0..modules {
        let n = *read(BLOCK_COUNTS + module as u32, 1)?
            .first()
            .context("short MMG block count")? as usize;
        ensure!(n <= 256, "implausible block count {n} in module {module}");
        let table = word(read, MODULE_BLOCK_TABLE + module as u32 * 4)?;
        if n > 0 {
            ensure!(table != 0, "missing block table for module {module}");
        }
        for block in 0..n {
            let a = table
                .checked_add((block * 16) as u32)
                .ok_or_else(|| anyhow::anyhow!("block table overflow"))?;
            let base = word(read, a)?;
            let end = word(read, a + 4)?;
            if base != u32::MAX && base <= end {
                blocks.push((module as u8, block as u8, base, end));
            }
        }
    }
    let mut nodes = Vec::with_capacity(count as usize);
    let mut seen = BTreeSet::new();
    let mut cur = head;
    let mut prior = 0;
    while cur != 0 {
        ensure!(
            nodes.len() < limit,
            "registry traversal exceeded bound {limit}"
        );
        ensure!(seen.insert(cur), "registry cycle at 0x{cur:08x}");
        let b = read(cur, NODE_BYTES)?;
        ensure!(b.len() == NODE_BYTES, "short registry node");
        let next = u32at(&b, 0)?;
        let previous = u32at(&b, 4)?;
        let owner = u32at(&b, 8)?;
        let source_address = u32at(&b, 12)?;
        let flags = u32at(&b, 16)?;
        let reserved_word = u32at(&b, 20)?;
        let snapshot_or_target = u32at(&b, 24)?;
        let size_or_pointer_module = u32at(&b, 28)?;
        let trailing_word = u32at(&b, 32)?;
        ensure!(
            previous == prior,
            "node 0x{cur:08x} prev mismatch: 0x{previous:08x} != 0x{prior:08x}"
        );
        ensure!(
            owner == REGISTRY,
            "node 0x{cur:08x} owner mismatch 0x{owner:08x}"
        );
        let mut matches = blocks.iter().filter_map(|(m, k, base, end)| {
            (source_address >= *base && source_address <= *end).then_some(SourceLocation {
                module: *m,
                block: *k,
                offset: source_address - *base,
                block_base: *base,
            })
        });
        let mapped_source = matches.next();
        ensure!(
            matches.next().is_none(),
            "node 0x{cur:08x} source address has ambiguous MMG block ownership"
        );
        nodes.push(RegistryNode {
            address: cur,
            next,
            previous,
            owner,
            source_address,
            flags,
            reserved_word,
            snapshot_or_target,
            size_or_pointer_module,
            trailing_word,
            mapped_source,
        });
        prior = cur;
        cur = next;
    }
    ensure!(
        nodes.len() == count as usize,
        "header count {count} != traversed {}",
        nodes.len()
    );
    ensure!(
        nodes.last().map_or(0, |n| n.address) == tail,
        "header tail 0x{tail:08x} != traversed tail"
    );
    ensure!(
        head == 0 || nodes.first().is_some_and(|n| n.previous == 0),
        "bad head prev"
    );
    if head == 0 {
        ensure!(
            tail == 0 && count == 0,
            "empty head with nonempty tail/count"
        );
    }
    Ok(RegistrySnapshot {
        root: REGISTRY,
        first: head,
        tail,
        declared_count: count,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn put(m: &mut BTreeMap<u32, Vec<u8>>, a: u32, b: &[u8]) {
        m.insert(a, b.to_vec());
    }
    fn w(m: &mut BTreeMap<u32, Vec<u8>>, a: u32, v: u32) {
        put(m, a, &v.to_le_bytes())
    }
    fn fixture() -> BTreeMap<u32, Vec<u8>> {
        let mut m = BTreeMap::new();
        w(&mut m, REGISTRY, 0x1000);
        w(&mut m, REGISTRY + 4, 0x1000);
        w(&mut m, REGISTRY + 8, 1);
        let mut node = vec![0; 36];
        for (i, v) in [0, 0, REGISTRY, 0x2005, 0x100, 0, 0x1234, 4, 0]
            .iter()
            .enumerate()
        {
            node[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes())
        }
        put(&mut m, 0x1000, &node);
        w(&mut m, MODULE_COUNT, 1);
        put(&mut m, BLOCK_COUNTS, &[1]);
        w(&mut m, MODULE_BLOCK_TABLE, 0x3000);
        w(&mut m, 0x3000, 0x2000);
        w(&mut m, 0x3004, 0x2fff);
        m
    }
    #[test]
    fn maps_and_validates_node() {
        let m = fixture();
        let mut r = |a, n| {
            Ok(m.range(a..)
                .next()
                .filter(|(k, _v)| **k <= a && **k + n as u32 >= a + n as u32)
                .map(|(k, v)| {
                    let x = (a - *k) as usize;
                    v[x..x + n].to_vec()
                })
                .unwrap_or_else(|| vec![0; n]))
        };
        let s = read_registry(&mut r, 10).unwrap();
        assert_eq!(s.nodes[0].mapped_source.as_ref().unwrap().offset, 5);
    }
    #[test]
    fn rejects_cycle() {
        let mut m = fixture();
        let mut n = m[&0x1000].clone();
        n[0..4].copy_from_slice(&0x1000u32.to_le_bytes());
        m.insert(0x1000, n);
        let mut r = |a, n| {
            Ok(m.get(&a)
                .map(|v| v[..n].to_vec())
                .unwrap_or_else(|| vec![0; n]))
        };
        assert!(
            read_registry(&mut r, 10)
                .unwrap_err()
                .to_string()
                .contains("cycle")
        );
    }

    #[test]
    fn rejects_bad_header_links_owner_and_limit() {
        let mut m = fixture();
        m.insert(REGISTRY + 4, 0x1004u32.to_le_bytes().to_vec());
        let mut r = |a, n| {
            Ok(m.get(&a)
                .map(|v| v[..n].to_vec())
                .unwrap_or_else(|| vec![0; n]))
        };
        assert!(
            read_registry(&mut r, 10)
                .unwrap_err()
                .to_string()
                .contains("tail")
        );

        let mut m = fixture();
        let mut n = m[&0x1000].clone();
        n[8..12].copy_from_slice(&0u32.to_le_bytes());
        m.insert(0x1000, n);
        let mut r = |a, n| {
            Ok(m.get(&a)
                .map(|v| v[..n].to_vec())
                .unwrap_or_else(|| vec![0; n]))
        };
        assert!(
            read_registry(&mut r, 10)
                .unwrap_err()
                .to_string()
                .contains("owner")
        );

        let mut m = fixture();
        let mut n = m[&0x1000].clone();
        n[4..8].copy_from_slice(&0x1234u32.to_le_bytes());
        m.insert(0x1000, n);
        let mut r = |a, n| {
            Ok(m.get(&a)
                .map(|v| v[..n].to_vec())
                .unwrap_or_else(|| vec![0; n]))
        };
        assert!(
            read_registry(&mut r, 10)
                .unwrap_err()
                .to_string()
                .contains("prev")
        );

        let mut m = fixture();
        m.insert(REGISTRY + 8, 11u32.to_le_bytes().to_vec());
        let mut r = |a, n| {
            Ok(m.get(&a)
                .map(|v| v[..n].to_vec())
                .unwrap_or_else(|| vec![0; n]))
        };
        assert!(
            read_registry(&mut r, 10)
                .unwrap_err()
                .to_string()
                .contains("exceeds bound")
        );
    }
}
