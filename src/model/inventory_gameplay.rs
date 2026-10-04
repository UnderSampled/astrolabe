//! The shared item catalogue, admitted category slots, and source pickup values.
use super::{
    AssetPath,
    ai::{SourceObjectIdentity, SourceRole},
    io::Content,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryCatalog {
    pub path: AssetPath,
    /// Source order also decides the first selected item returned by the API.
    pub items: Vec<InventoryDefinition>,
    pub gold: InventoryDefinition,
    pub gold_category: SourceObjectIdentity,
    pub categories: Vec<InventoryAdmission>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryDefinition {
    pub source: SourceObjectIdentity,
    pub id: i16,
    /// Ordinary items have a kind; the special resource has no source kind.
    pub kind: Option<String>,
    pub maximum: i16,
    pub quantity: i16,
    pub selected: bool,
    pub availability: u8,
    pub management: u16,
    /// Corroborated stock-action membership, used only for native legacy seeds.
    pub pickup_category: Option<SourceObjectIdentity>,
    /// Source-ordered replacement rules, with distinct item identity endpoints.
    pub replacements: Vec<InventoryReplacement>,
    pub unknown: [u8; 6],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryReplacement {
    pub source: SourceObjectIdentity,
    pub from: Option<SourceObjectIdentity>,
    pub to: Option<SourceObjectIdentity>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryAdmission {
    pub source: SourceObjectIdentity,
    /// Header/artwork children are excluded. Empty positions remain useful slots.
    pub slots: Vec<Option<SourceObjectIdentity>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryStock {
    pub category: Option<SourceObjectIdentity>,
    pub item: Option<SourceObjectIdentity>,
    pub count: i16,
    pub unknown: [u8; 2],
}
impl Content for InventoryCatalog {
    const KIND: &'static str = "inventory-catalog";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.path.as_str() == "fix:/inventory/gameplay",
            "inventory catalogue identity differs"
        );
        ensure!(
            !self.items.is_empty() && self.items.len() <= 256 && self.categories.len() <= 64,
            "inventory catalogue count bound"
        );
        let mut identities = Vec::new();
        for item in &self.items {
            ensure!(
                !identities.contains(&&item.source),
                "duplicate inventory item"
            );
            identities.push(&item.source);
            ensure!(
                item.id >= 0
                    && item.kind.as_ref().is_some_and(|kind| !kind.is_empty())
                    && item.source.role == SourceRole::Fix,
                "invalid inventory definition"
            );
        }
        ensure!(
            !identities.contains(&&self.gold.source)
                && self.gold.source.role == SourceRole::Fix
                && self.gold_category.role == SourceRole::Fix
                && self.gold.kind.as_ref().is_none_or(|kind| !kind.is_empty()),
            "invalid special inventory resource"
        );
        identities.push(&self.gold.source);
        let mut replacement_sources = Vec::new();
        for item in self.items.iter().chain(std::iter::once(&self.gold)) {
            ensure!(
                item.replacements.len() <= 256,
                "inventory replacement bound"
            );
            for replacement in &item.replacements {
                ensure!(
                    replacement.source.role == SourceRole::Fix
                        && !identities.contains(&&replacement.source)
                        && !replacement_sources.contains(&&replacement.source),
                    "invalid/repeated inventory replacement identity"
                );
                replacement_sources.push(&replacement.source);
                ensure!(
                    replacement
                        .from
                        .iter()
                        .chain(replacement.to.iter())
                        .all(|endpoint| identities.contains(&endpoint)),
                    "inventory replacement leaves catalogue"
                );
            }
        }
        let mut categories = Vec::new();
        for category in &self.categories {
            ensure!(
                !categories.contains(&&category.source)
                    && category.slots.len() <= 256
                    && category.source.role == SourceRole::Fix,
                "duplicate/unbounded inventory category"
            );
            categories.push(&category.source);
            ensure!(
                category
                    .slots
                    .iter()
                    .flatten()
                    .all(|item| identities.contains(&item) && item != &self.gold.source),
                "inventory admission leaves catalogue"
            );
        }
        for item in &self.items {
            ensure!(
                item.pickup_category
                    .as_ref()
                    .is_none_or(|category| category.role == SourceRole::Fix
                        && categories.contains(&category)),
                "inventory pickup category leaves catalogue"
            );
        }
        Ok(())
    }
}
