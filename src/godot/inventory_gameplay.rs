//! Native resources for the shared inventory API, not source-table execution.
use super::{Project, disk_path, quoted};
use crate::model::{
    ai::SourceObjectIdentity, content::ContentSource, inventory_gameplay::*, io::Content,
};
use anyhow::{Context, Result};
use std::{collections::BTreeSet, fmt::Write as _};

fn key(identity: &SourceObjectIdentity) -> Result<String> {
    // Keys only compare retained identities; Godot never interprets this text.
    Ok(serde_json::to_string(identity)?)
}
pub(super) fn stock_fields(stock: &InventoryStock) -> Result<String> {
    Ok(format!(
        "category_key = {}\nitem_key = {}\ncount = {}\nunknown = PackedByteArray({},{})\n",
        quoted(
            &stock
                .category
                .as_ref()
                .map(key)
                .transpose()?
                .unwrap_or_default()
        ),
        quoted(
            &stock
                .item
                .as_ref()
                .map(key)
                .transpose()?
                .unwrap_or_default()
        ),
        stock.count,
        stock.unknown[0],
        stock.unknown[1]
    ))
}
pub(super) fn export(
    project: &mut Project,
    _: &mut impl ContentSource,
    value: &InventoryCatalog,
) -> Result<()> {
    value.validate()?;
    // Info IDs are gameplay query values, not unique identities: the twelve
    // NotFound placeholders all use ID50. Native slots and replacement edges
    // identify the actual member in the ordered catalogue.
    let member_index = |identity: &SourceObjectIdentity| -> Result<usize> {
        value
            .items
            .iter()
            .position(|item| &item.source == identity)
            .context("inventory member leaves ordinary catalogue")
    };
    let mut text = format!(
        "[gd_resource type=\"Resource\" load_steps={} format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/character/inventory.gd\" id=\"inventory\"]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/character/inventory_definition.gd\" id=\"definition\"]\n",
        value.items.len() + 4
    );
    for (n, item) in value
        .items
        .iter()
        .chain(std::iter::once(&value.gold))
        .enumerate()
    {
        let unknown = item
            .unknown
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let replacements = item
            .replacements
            .iter()
            .map(|rule| -> Result<String> {
                let endpoint = |identity: &Option<SourceObjectIdentity>| -> Result<String> {
                    Ok(match identity {
                        Some(identity) => member_index(identity)?.to_string(),
                        None => "-1".into(),
                    })
                };
                Ok(format!(
                    "Vector2i({},{})",
                    endpoint(&rule.from)?,
                    endpoint(&rule.to)?
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .join(",");
        writeln!(
            text,
            "\n[sub_resource type=\"Resource\" id=\"item{n}\"]\nscript = ExtResource(\"definition\")\nsource_key = {}\nitem_id = {}\nkind = {}\nmaximum = {}\nquantity = {}\nselected = {}\navailability = {}\nmanagement = {}\npickup_category = {}\nreplacements = Array[Vector2i]([{replacements}])\nunknown = PackedByteArray({unknown})",
            quoted(&key(&item.source)?),
            item.id,
            quoted(item.kind.as_deref().unwrap_or_default()),
            item.maximum,
            item.quantity,
            item.selected,
            item.availability,
            item.management,
            quoted(
                &item
                    .pickup_category
                    .as_ref()
                    .map(key)
                    .transpose()?
                    .unwrap_or_default()
            )
        )?;
    }
    let mut categories = Vec::new();
    for category in &value.categories {
        let slots = category
            .slots
            .iter()
            .map(|item| -> Result<String> {
                Ok(match item {
                    None => "-1".into(),
                    Some(identity) => member_index(identity)?.to_string(),
                })
            })
            .collect::<Result<Vec<_>>>()?
            .join(",");
        categories.push(format!("{}: [{slots}]", quoted(&key(&category.source)?)));
    }
    let mut seen_ids = BTreeSet::new();
    let quantities = value
        .items
        .iter()
        .filter(|item| seen_ids.insert(item.id))
        .map(|i| format!("{}: {}", i.id, i.quantity))
        .collect::<Vec<_>>()
        .join(",");
    let definitions = (0..value.items.len())
        .map(|n| format!("SubResource(\"item{n}\")"))
        .collect::<Vec<_>>()
        .join(",");
    let selected = |kind: &str| {
        value
            .items
            .iter()
            .find(|i| i.selected && i.kind.as_deref() == Some(kind))
            .map_or(-1, |i| i32::from(i.id))
    };
    writeln!(
        text,
        "\n[resource]\nresource_local_to_scene = true\nscript = ExtResource(\"inventory\")\nsource_initialized = true\ndefinitions = Array[Resource]([{definitions}])\nquantities = {{{quantities}}}\ncategories = {{{}}}\ngold_definition = SubResource(\"item{}\")\ngold_category = {}\ngold = {}\nselected_weapon = {}\nselected_object = {}\nselected_magic = {}",
        categories.join(","),
        value.items.len(),
        quoted(&key(&value.gold_category)?),
        value.gold.quantity,
        selected("Weapon"),
        selected("Object"),
        value
            .items
            .iter()
            .find(|i| i.selected
                && i.kind
                    .as_ref()
                    .is_some_and(|kind| kind.starts_with("Magic")))
            .map_or(-1, |i| i32::from(i.id))
    )?;
    project.write(
        disk_path(&project.selection, &value.path, "tres"),
        text.as_bytes(),
    )
}
