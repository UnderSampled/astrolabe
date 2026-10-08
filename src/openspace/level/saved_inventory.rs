//! Semantic projection of source-registered inventory values from an original save.
//!
//! SAI block/module/offset identities are joined to the active Fix composition
//! only inside this OpenSpace reader. The exported value is the stable item
//! identity and its typed quantity/capacity, never a source address.
use super::{
    container::{file, pointer_file},
    hud,
    records::{Address, Owner},
    session::SourceSession,
};
use crate::{
    model::Selection,
    openspace::save_game::{SaiRecord, SaveSlot},
};
use anyhow::{Context, Result, ensure};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SavedInventoryItem {
    pub(crate) item_id: u16,
    pub(crate) quantity: i16,
    pub(crate) capacity: i16,
}

fn record_table(save: &SaveSlot) -> Result<&[SaiRecord]> {
    Ok(save
        .members
        .get("Level000.sav")
        .context("save is missing its player record file")?
        .records
        .as_ref()
        .context("Level000.sav has no decoded SAI records")?
        .records
        .as_slice())
}

fn source_field(base: Address, displacement: usize) -> Result<Address> {
    Ok(Address {
        offset: base
            .offset
            .checked_add(displacement)
            .context("source inventory field offset overflow")?,
        ..base
    })
}

fn unique_registered_i16<'a>(
    records: &'a [SaiRecord],
    site: Address,
    field_name: &str,
) -> Result<&'a SaiRecord> {
    let offset = u32::try_from(site.offset).context("source field exceeds SAI address width")?;
    let mut matches = records.iter().filter(|record| {
        record.header.module == site.key.0
            && record.header.block == site.key.1
            && record.header.offset == offset
    });
    let record = matches
        .next()
        .with_context(|| format!("save is missing the source-composed {field_name} value"))?;
    ensure!(
        matches.next().is_none(),
        "save has ambiguous source-composed {field_name} values"
    );
    ensure!(
        record.header.save_flags == 0x0204,
        "source-composed {field_name} has unexpected save route/type flags"
    );
    ensure!(
        record.header.data_type == 0x02 && record.value_bytes.len() == 2,
        "source-composed {field_name} is not a signed 16-bit value"
    );
    Ok(record)
}

fn signed_i16_at(records: &[SaiRecord], site: Address, field_name: &str) -> Result<i16> {
    let record = unique_registered_i16(records, site, field_name)?;
    Ok(i16::from_le_bytes(
        record.value_bytes[..2]
            .try_into()
            .expect("width checked by unique_registered_i16"),
    ))
}

fn project_item_zero_from_sites(
    records: &[SaiRecord],
    item_id: i32,
    handle: Address,
    item_definition: Address,
) -> Result<SavedInventoryItem> {
    ensure!(
        item_id == 0,
        "source currency composition did not resolve to item 0"
    );
    let quantity_site = source_field(handle, 0x0a)?;
    let capacity_site = source_field(item_definition, 0x14)?;
    let quantity = signed_i16_at(records, quantity_site, "item-0 quantity")?;
    let capacity = signed_i16_at(records, capacity_site, "item-0 capacity")?;
    ensure!(
        quantity >= 0 && capacity >= 0 && quantity <= capacity,
        "source-composed item-0 quantity exceeds its saved capacity"
    );
    Ok(SavedInventoryItem {
        item_id: 0,
        quantity,
        capacity,
    })
}

/// Resolve the saved resource quantity by following the source Fix HUD root to
/// its inventory handle and item definition, then joining those live fields to
/// the save table. Addresses and relocation data never leave this function.
pub(crate) fn read_saved_inventory_item_zero(
    game: &Path,
    save: &SaveSlot,
) -> Result<SavedInventoryItem> {
    ensure!(
        save.current_level == "manoir",
        "item-0 save projection is only verified for the manoir profile"
    );
    let selection = Selection::hype("manoir", "en")?;
    let session = SourceSession::open(game, &selection)?;
    let fix_dir = game.join("Gamedata/World/Levels");
    let sda = file(&fix_dir.join("Fix.sda"))?;
    let fix_relocations = pointer_file(&file(&fix_dir.join("Fix.rtp"))?)?;
    ensure!(
        sda.len() == 3_683,
        "unsupported Fix.sda extent for item-0 save projection"
    );
    let (currency_root_offset, _) = hud::gpt_hud_offsets(&sda)?;
    let (child, handle, item_definition, item_id) = hud::currency_item(
        &session,
        &fix_relocations,
        &file(&fix_dir.join("Fix.gpt"))?,
        currency_root_offset,
    )?;
    ensure!(
        handle.owner == Owner::Fix && item_definition.owner == Owner::Fix,
        "item-0 source composition is not rooted in Fix"
    );
    ensure!(
        session.allocation_bytes(handle)?.len() == 24,
        "source-composed item-0 inventory handle has an unexpected extent"
    );
    ensure!(
        session.allocation_bytes(item_definition)?.len() == 32,
        "source-composed item-0 definition has an unexpected extent"
    );
    ensure!(
        child.owner == Owner::Fix,
        "source-composed item-0 child is not in Fix"
    );
    let value =
        project_item_zero_from_sites(record_table(save)?, item_id, handle, item_definition)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{SavedInventoryItem, project_item_zero_from_sites};
    use crate::{
        openspace::level::records::{Address, BlockKey, Owner},
        openspace::save_game::parse_records,
    };

    fn address(module: u8, block: u8, offset: usize) -> Address {
        Address {
            owner: Owner::Fix,
            key: BlockKey(module, block),
            offset,
        }
    }

    fn records(values: &[(u8, u8, u32, i16)]) -> Vec<crate::openspace::save_game::SaiRecord> {
        let mut bytes = Vec::new();
        for (module, block, offset, value) in values {
            bytes.extend_from_slice(&[*block, *module]);
            bytes.extend_from_slice(&offset.to_le_bytes());
            bytes.extend_from_slice(&0x0204u16.to_le_bytes());
            bytes.extend_from_slice(&[0, 0x02]);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        parse_records(&bytes)
            .expect("synthetic SAI records parse")
            .records
    }

    #[test]
    fn item_zero_semantics_survive_source_block_and_allocation_relocation() {
        let original = records(&[(10, 0, 5_790, 20), (10, 0, 5_828, 200)]);
        let relocated = records(&[(7, 3, 410, 20), (7, 3, 468, 200)]);

        let original_state = project_item_zero_from_sites(
            &original,
            0,
            address(10, 0, 5_780),
            address(10, 0, 5_808),
        )
        .expect("source composition projects original sites");
        let relocated_state =
            project_item_zero_from_sites(&relocated, 0, address(7, 3, 400), address(7, 3, 448))
                .expect("source composition projects relocated sites");

        let expected = SavedInventoryItem {
            item_id: 0,
            quantity: 20,
            capacity: 200,
        };
        assert_eq!(original_state, expected);
        assert_eq!(relocated_state, expected);
        let semantic_json = |item: SavedInventoryItem| {
            serde_json::json!({
                "item_id": item.item_id,
                "quantity": item.quantity,
                "capacity": item.capacity,
            })
        };
        let original_json = semantic_json(original_state);
        let relocated_json = semantic_json(relocated_state);
        assert_eq!(original_json, relocated_json);
        let serialized = serde_json::to_string(&relocated_json).unwrap();
        assert!(!serialized.contains("offset"));
        assert!(!serialized.contains("block"));
        assert!(!serialized.contains("module"));
        assert!(!serialized.contains("relocation"));
    }

    #[test]
    fn item_zero_semantic_join_rejects_unknown_and_ambiguous_sites() {
        let handle = address(10, 0, 5_780);
        let item = address(10, 0, 5_808);
        let missing = records(&[(10, 0, 5_828, 200)]);
        assert!(project_item_zero_from_sites(&missing, 0, handle, item).is_err());

        let duplicate = records(&[(10, 0, 5_790, 20), (10, 0, 5_790, 20), (10, 0, 5_828, 200)]);
        assert!(project_item_zero_from_sites(&duplicate, 0, handle, item).is_err());
    }

    #[test]
    fn item_zero_semantic_join_rejects_wrong_source_item() {
        let save = records(&[(10, 0, 5_790, 20), (10, 0, 5_828, 200)]);
        assert!(
            project_item_zero_from_sites(&save, 80, address(10, 0, 5_780), address(10, 0, 5_808))
                .is_err()
        );
    }
}
