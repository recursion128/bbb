use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const VANILLA_ITEM_COMPONENT_COVERAGE_26_1: &str =
    include_str!("../../../docs/item-component-coverage-26.1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemComponentCoverageStatus {
    Complete,
    Partial,
    Deferred,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemComponentCoverageCell {
    pub status: ItemComponentCoverageStatus,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VanillaItemComponentCodec {
    pub persistent: bool,
    pub persistent_codec: Option<String>,
    pub network_codec: String,
    pub network_codec_source: String,
    pub cache_encoding: bool,
    pub ignore_swap_animation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemComponentCoverageEntry {
    pub type_id: i32,
    pub constant_name: String,
    pub resource_id: String,
    pub value_type: String,
    pub vanilla_codec: VanillaItemComponentCodec,
    pub coverage: BTreeMap<String, ItemComponentCoverageCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemComponentCoverageLedger {
    pub schema_version: u32,
    pub vanilla_version: String,
    pub generated_by: String,
    pub update_command: String,
    pub check_command: String,
    pub coverage_overrides: String,
    pub allowed_statuses: Vec<String>,
    pub baseline_policy: String,
    pub entries: Vec<ItemComponentCoverageEntry>,
}

pub fn vanilla_item_component_coverage_26_1(
) -> Result<ItemComponentCoverageLedger, serde_json::Error> {
    serde_json::from_str(VANILLA_ITEM_COMPONENT_COVERAGE_26_1)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::{
        data_component_types::VANILLA_DATA_COMPONENT_TYPES,
        packet_coverage::{vanilla_packet_coverage_26_1, PacketCoverageStatus},
    };

    use super::*;

    const COVERAGE_DIMENSIONS: [&str; 6] = [
        "canonical_retention",
        "network_encode",
        "observable_consumers",
        "persistent_projection",
        "tests",
        "typed_decode",
    ];

    #[test]
    fn coverage_ledger_matches_every_component_catalog_entry_in_order() {
        let ledger = vanilla_item_component_coverage_26_1().unwrap();
        assert_eq!(ledger.schema_version, 1);
        assert_eq!(ledger.vanilla_version, crate::MC_VERSION);
        assert_eq!(ledger.entries.len(), VANILLA_DATA_COMPONENT_TYPES.len());
        assert_eq!(
            ledger
                .allowed_statuses
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "complete".to_string(),
                "deferred".to_string(),
                "not-applicable".to_string(),
                "partial".to_string(),
            ])
        );
        assert_eq!(
            ledger.coverage_overrides,
            "scripts/data/item-component-coverage-overrides-26.1.json"
        );

        let mut keys = BTreeSet::new();
        for (entry, catalog) in ledger.entries.iter().zip(VANILLA_DATA_COMPONENT_TYPES) {
            assert_eq!(entry.type_id, catalog.type_id);
            assert_eq!(entry.constant_name, catalog.constant_name);
            assert_eq!(entry.resource_id, catalog.resource_id);
            assert_eq!(entry.vanilla_codec.persistent, catalog.persistent);
            assert!(!entry.value_type.is_empty());
            assert!(!entry.vanilla_codec.network_codec.is_empty());
            assert!(matches!(
                entry.vanilla_codec.network_codec_source.as_str(),
                "explicit" | "persistent-fallback"
            ));
            assert_eq!(
                entry
                    .coverage
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                COVERAGE_DIMENSIONS
            );
            assert!(keys.insert(entry.constant_name.as_str()));
        }
    }

    #[test]
    fn coverage_cells_require_evidence_or_an_actionable_gap() {
        let ledger = vanilla_item_component_coverage_26_1().unwrap();
        for entry in &ledger.entries {
            for (dimension, cell) in &entry.coverage {
                let label = format!("{}/{dimension}", entry.constant_name);
                match cell.status {
                    ItemComponentCoverageStatus::Complete
                    | ItemComponentCoverageStatus::NotApplicable => {
                        assert!(!cell.evidence.is_empty(), "{label} lacks evidence");
                        assert!(cell.gap.is_none(), "{label} retains a completed gap");
                    }
                    ItemComponentCoverageStatus::Deferred => {
                        assert!(!cell.evidence.is_empty(), "{label} lacks evidence");
                        assert!(
                            cell.gap
                                .as_deref()
                                .is_some_and(|gap| gap.starts_with("docs/unsupported-features.md#")),
                            "{label} lacks an actionable gap"
                        );
                    }
                    ItemComponentCoverageStatus::Partial => {
                        assert!(
                            cell.gap
                                .as_deref()
                                .is_some_and(|gap| gap.starts_with("docs/unsupported-features.md#")),
                            "{label} lacks an actionable gap"
                        );
                    }
                }
            }
            assert_eq!(
                entry.vanilla_codec.persistent,
                entry.vanilla_codec.persistent_codec.is_some(),
                "{} persistent codec classification drifted",
                entry.constant_name
            );
            if !entry.vanilla_codec.persistent {
                assert_eq!(
                    entry.coverage["persistent_projection"].status,
                    ItemComponentCoverageStatus::NotApplicable,
                    "{} transient persistent projection must be not-applicable",
                    entry.constant_name
                );
            }
        }
    }

    #[test]
    fn component_dependent_packet_cells_cannot_close_before_the_component_ledger() {
        let components = vanilla_item_component_coverage_26_1().unwrap();
        let packets = vanilla_packet_coverage_26_1().unwrap();
        let clientbound_packets = [
            "CLIENTBOUND_CONTAINER_SET_CONTENT",
            "CLIENTBOUND_CONTAINER_SET_SLOT",
            "CLIENTBOUND_SET_CURSOR_ITEM",
            "CLIENTBOUND_SET_EQUIPMENT",
            "CLIENTBOUND_SET_PLAYER_INVENTORY",
        ];
        if has_partial_component_dimension(
            &components,
            &["typed_decode", "canonical_retention", "tests"],
        ) {
            for constant_name in clientbound_packets {
                assert_packet_dimension_partial(&packets, constant_name, "canonical_apply");
            }
        }
        if has_partial_component_dimension(&components, &["observable_consumers", "tests"]) {
            for constant_name in clientbound_packets {
                assert_packet_dimension_partial(&packets, constant_name, "runtime_side_effect");
            }
        }
        if has_partial_component_dimension(&components, &["network_encode", "tests"]) {
            assert_packet_dimension_partial(
                &packets,
                "SERVERBOUND_SET_CREATIVE_MODE_SLOT",
                "encoder",
            );
        }
    }

    fn has_partial_component_dimension(
        components: &ItemComponentCoverageLedger,
        dimensions: &[&str],
    ) -> bool {
        components.entries.iter().any(|entry| {
            dimensions.iter().any(|dimension| {
                entry.coverage[*dimension].status == ItemComponentCoverageStatus::Partial
            })
        })
    }

    fn assert_packet_dimension_partial(
        packets: &crate::packet_coverage::PacketCoverageLedger,
        constant_name: &str,
        dimension: &str,
    ) {
        let packet = packets
            .entries
            .iter()
            .find(|entry| entry.constant_name == constant_name)
            .unwrap_or_else(|| panic!("missing packet coverage row {constant_name}"));
        assert_eq!(
            packet.coverage[dimension].status,
            PacketCoverageStatus::Partial,
            "{constant_name}/{dimension} closed before item components"
        );
    }
}
