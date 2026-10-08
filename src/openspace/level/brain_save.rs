//! Semantic projection of a bounded set of saved actor Brain fields.
//!
//! SAI addresses, allocation layout, and registration flags are consumed only
//! by this OpenSpace-side join. The result contains named actor/component/field
//! identities, typed values, and named appearance aliases; it is not a persisted
//! save envelope and carries no source-memory coordinates.
//!
//! `SaiRecord` and the separate save-join diagnostics retain wire block/module/
//! offset, record kind, flags, and raw value bytes at this reader boundary. SAI
//! kind `0x11` stores a source-relative reference; the source join may inspect it
//! for diagnostics, but it is not yet promoted for Brain fields. A supported
//! reference must become a named model link before leaving this module.
//! Unsupported records and `RecordTable::unknown_tail` stay in the original
//! decoded table.

use super::save_join::{
    SaveAllocationCandidate, SaveAllocationJoin, SaveRecordLocation, join_save_allocations,
};
use crate::{
    model::{AssetPath, PackageRole, Selection, ai::DesignerValue},
    openspace::save_game::SaiRecord,
};
use anyhow::{Result, bail, ensure};
use serde::{Serialize, Serializer};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActorSaveComponent {
    Brain,
}

/// Stable field identity used by the compiled Brain property, e.g. `dsg_11`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub(crate) struct ActorBrainFieldId(#[serde(serialize_with = "serialize_field_id")] u32);

impl ActorBrainFieldId {
    pub(crate) const fn dsg(index: u32) -> Self {
        Self(index)
    }

    fn source_label(self) -> String {
        format!("DSG[{}]", self.0)
    }

    fn property_name(self) -> String {
        format!("dsg_{}", self.0)
    }
}

fn serialize_field_id<S>(index: &u32, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("dsg_{index}"))
}

/// Canonical source actor plus its native component and stable field identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct ActorBrainFieldIdentity {
    pub(crate) owner: AssetPath,
    pub(crate) component: ActorSaveComponent,
    pub(crate) field: ActorBrainFieldId,
}

impl ActorBrainFieldIdentity {
    pub(crate) fn new(owner: impl Into<String>, field: ActorBrainFieldId) -> Result<Self> {
        let owner = AssetPath::new(owner)?;
        ensure!(
            matches!(owner.role(), PackageRole::Fix | PackageRole::Level)
                && owner.file().starts_with("persos/"),
            "Brain owner must be a named Fix or level Perso URI"
        );
        Ok(Self {
            owner,
            component: ActorSaveComponent::Brain,
            field,
        })
    }
}

/// Optional scene appearance is a named alias and never replaces `identity.owner`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ActorBrainFieldValue {
    pub(crate) value: DesignerValue,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) appearance_aliases: Vec<AssetPath>,
}

/// Address-free, deterministic projection keyed by owner/component/field.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ActorBrainSaveProjection {
    pub(crate) fields: BTreeMap<ActorBrainFieldIdentity, ActorBrainFieldValue>,
}

/// Resolve requested Brain fields from the active source package and SAI records.
///
/// Only one-byte Boolean designer values are currently promoted. Every other
/// save record remains in its original `RecordTable`; unsupported bytes are not
/// interpreted or removed here.
pub(crate) fn project_actor_brain_save_fields(
    game: &Path,
    selection: &Selection,
    records: &[SaiRecord],
    requested: &[ActorBrainFieldIdentity],
) -> Result<ActorBrainSaveProjection> {
    ensure!(
        requested
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == requested.len(),
        "duplicate requested actor Brain field identity"
    );
    if requested.is_empty() {
        return Ok(ActorBrainSaveProjection::default());
    }

    let locations = records
        .iter()
        .map(save_record_location)
        .collect::<Result<Vec<_>>>()?;
    let joined = join_save_allocations(game, selection, &locations)?;
    ensure!(
        joined.joins.len() == records.len(),
        "source Brain join did not preserve save record order"
    );

    let mut projection = BTreeMap::new();
    for (record, join) in records.iter().zip(&joined.joins) {
        for identity in requested {
            let Some(candidate) = select_candidate(join, identity)? else {
                continue;
            };
            ensure!(
                candidate.source_type.as_deref() == Some("designer type 0"),
                "save record {} has an unsupported source type for {}.brain.{}",
                join.id,
                identity.owner,
                identity.field.property_name()
            );
            ensure!(
                record.header.data_type == 0x10 && record.value_bytes.len() == 1,
                "save record {} is not a one-byte registered Boolean value",
                join.id
            );
            let value = match record.value_bytes[0] {
                0 => false,
                1 => true,
                other => bail!(
                    "save record {} has invalid Boolean value {other:#04x}",
                    join.id
                ),
            };

            let appearance_aliases = appearance_aliases(candidate, identity)?;
            let prior = projection.insert(
                identity.clone(),
                ActorBrainFieldValue {
                    value: DesignerValue::Boolean(value),
                    appearance_aliases,
                },
            );
            ensure!(
                prior.is_none(),
                "multiple save records resolve to {}.brain.{}",
                identity.owner,
                identity.field.property_name()
            );
        }
    }

    for identity in requested {
        ensure!(
            projection.contains_key(identity),
            "no unique supported saved source field resolves to {}.brain.{}",
            identity.owner,
            identity.field.property_name()
        );
    }

    Ok(ActorBrainSaveProjection { fields: projection })
}

fn candidate_matches(
    candidate: &SaveAllocationCandidate,
    identity: &ActorBrainFieldIdentity,
) -> bool {
    candidate.source_object.as_deref() == Some(identity.owner.as_str())
        && candidate.source_field.as_deref() == Some(identity.field.source_label().as_str())
}

fn select_candidate<'a>(
    join: &'a SaveAllocationJoin,
    identity: &ActorBrainFieldIdentity,
) -> Result<Option<&'a SaveAllocationCandidate>> {
    let source_field = identity.field.source_label();
    let exact_ambiguity = format!("{} {source_field}", identity.owner);
    if join
        .candidates
        .iter()
        .flat_map(|candidate| &candidate.field_match_ambiguity)
        .any(|ambiguity| ambiguity == &exact_ambiguity)
    {
        bail!(
            "save record {} has ambiguous source ownership for {}.brain.{}",
            join.id,
            identity.owner,
            identity.field.property_name()
        );
    }

    let matching = join
        .candidates
        .iter()
        .filter(|candidate| candidate_matches(candidate, identity))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return Ok(None);
    }
    ensure!(
        matching.len() == 1 && join.candidates.len() == 1,
        "save record {} resolves to multiple actor Brain owners for {}.brain.{}",
        join.id,
        identity.owner,
        identity.field.property_name()
    );
    ensure!(
        join.status == "single_bounded_allocation",
        "save record {} does not resolve to one bounded source owner for {}.brain.{}",
        join.id,
        identity.owner,
        identity.field.property_name()
    );
    Ok(Some(matching[0]))
}

fn appearance_aliases(
    candidate: &SaveAllocationCandidate,
    identity: &ActorBrainFieldIdentity,
) -> Result<Vec<AssetPath>> {
    let Some(appearance) = &candidate.source_appearance else {
        return Ok(Vec::new());
    };
    let appearance = AssetPath::new(appearance.clone())?;
    ensure!(
        appearance.role() == PackageRole::Level && appearance.file().starts_with("persos/"),
        "Brain appearance alias must be a rooted level Perso URI"
    );
    if appearance == identity.owner {
        Ok(Vec::new())
    } else {
        Ok(vec![appearance])
    }
}

fn save_record_location(record: &SaiRecord) -> Result<SaveRecordLocation> {
    let reference_target = if record.header.data_type == 0x11 {
        ensure!(
            record.value_bytes.len() == 6,
            "save record {} has an invalid source reference width",
            record.identity()
        );
        Some((
            record.value_bytes[0],
            record.value_bytes[1],
            u32::from_le_bytes(
                record.value_bytes[2..6]
                    .try_into()
                    .expect("reference width checked"),
            ),
        ))
    } else {
        None
    };
    let live_width = if record.header.data_type == 0x11 {
        4
    } else {
        record.value_bytes.len()
    };
    Ok(SaveRecordLocation {
        id: record.identity(),
        block: record.header.block,
        module: record.header.module,
        offset: record.header.offset,
        live_width,
        kind: format!("0x{:02x}", record.header.data_type),
        flags: format!("0x{:04x}", record.header.save_flags),
        reference_target,
    })
}

#[cfg(test)]
#[path = "brain_save_tests.rs"]
mod tests;
