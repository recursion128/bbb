use serde::{Deserialize, Serialize};

const VANILLA_REGISTRY_HOLDER_COVERAGE_26_1: &str =
    include_str!("../../../docs/registry-holder-consumer-coverage-26.1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryHolderCoverageStatus {
    Complete,
    Partial,
    Deferred,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryHolderCoverageEntry {
    pub call_site: String,
    pub source_path: String,
    pub line: u32,
    pub column: u32,
    pub owner: String,
    pub factory: String,
    pub kind: String,
    pub registry_expression: String,
    pub resource_id: Option<String>,
    pub family: String,
    pub status: RegistryHolderCoverageStatus,
    pub consumer: Option<String>,
    pub evidence: Vec<String>,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryHolderCoverageLedger {
    pub schema_version: u32,
    pub vanilla_version: String,
    pub generated_by: String,
    pub update_command: String,
    pub check_command: String,
    pub coverage_families: String,
    pub allowed_statuses: Vec<String>,
    pub scope: String,
    pub entries: Vec<RegistryHolderCoverageEntry>,
}

pub fn vanilla_registry_holder_coverage_26_1(
) -> Result<RegistryHolderCoverageLedger, serde_json::Error> {
    serde_json::from_str(VANILLA_REGISTRY_HOLDER_COVERAGE_26_1)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::registry_holder_consumers::VANILLA_REGISTRY_HOLDER_CONSUMERS;

    use super::*;

    #[test]
    fn coverage_ledger_maps_every_holder_consumer_once() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        assert_eq!(ledger.schema_version, 1);
        assert_eq!(ledger.vanilla_version, crate::MC_VERSION);
        assert_eq!(ledger.entries.len(), 194);
        assert_eq!(
            ledger
                .allowed_statuses
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["complete", "deferred", "not-applicable", "partial"])
        );

        let expected: BTreeSet<_> = VANILLA_REGISTRY_HOLDER_CONSUMERS
            .iter()
            .map(|entry| {
                format!(
                    "{}:{}:{}:{}",
                    entry.source_path, entry.line, entry.column, entry.factory
                )
            })
            .collect();
        let actual: BTreeSet<_> = ledger
            .entries
            .iter()
            .map(|entry| entry.call_site.clone())
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), ledger.entries.len());
    }

    #[test]
    fn coverage_rows_preserve_inventory_identity_and_actionable_decisions() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        for entry in &ledger.entries {
            let inventory = VANILLA_REGISTRY_HOLDER_CONSUMERS
                .iter()
                .find(|candidate| {
                    candidate.source_path == entry.source_path
                        && candidate.line == entry.line
                        && candidate.column == entry.column
                        && candidate.factory == entry.factory
                })
                .unwrap_or_else(|| panic!("missing inventory row {}", entry.call_site));
            assert_eq!(entry.owner, inventory.owner);
            assert_eq!(entry.kind, inventory.kind);
            assert_eq!(entry.registry_expression, inventory.registry_expression);
            assert_eq!(entry.resource_id.as_deref(), inventory.resource_id);
            assert!(!entry.evidence.is_empty());

            match entry.status {
                RegistryHolderCoverageStatus::Complete => {
                    assert!(entry.consumer.is_some());
                }
                RegistryHolderCoverageStatus::Partial => {
                    assert!(entry.consumer.is_some());
                    assert!(entry.decision.starts_with("docs/"));
                    assert!(entry.decision.ends_with(
                        "#protocol-coverage-for-remaining-required-261-packet-families"
                    ));
                }
                RegistryHolderCoverageStatus::Deferred => {
                    assert!(entry.decision.starts_with("restart: "));
                }
                RegistryHolderCoverageStatus::NotApplicable => {
                    assert!(entry.consumer.is_none());
                }
            }
        }
    }

    #[test]
    fn coverage_uses_narrow_families_without_premature_completion() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let counts = ledger
            .entries
            .iter()
            .fold(BTreeMap::new(), |mut counts, entry| {
                *counts.entry(entry.family.as_str()).or_insert(0usize) += 1;
                counts
            });
        assert_eq!(counts["network-holder-or-inline"], 8);
        assert_eq!(counts["network-holder-or-inline-sound-event"], 1);
        assert_eq!(counts["network-holder-reference"], 22);
        assert_eq!(counts["network-holder-reference-farm-animal-variants"], 3);
        assert_eq!(counts["network-holder-reference-debug"], 2);
        assert_eq!(counts["network-holder-reference-helper"], 1);
        assert_eq!(counts["network-holder-set-or-tag"], 11);
        assert_eq!(counts["network-tag"], 1);
        assert_eq!(counts["persistent-hashed-tag"], 7);
        assert_eq!(counts["persistent-holder-or-inline"], 27);
        assert_eq!(counts["persistent-holder-reference"], 41);
        assert_eq!(counts["persistent-holder-set-custom"], 3);
        assert_eq!(counts["persistent-holder-set-or-inline"], 6);
        assert_eq!(counts["persistent-holder-set-or-tag"], 50);
        assert_eq!(counts["persistent-tag"], 11);
        assert_eq!(
            ledger
                .entries
                .iter()
                .filter(|entry| entry.status == RegistryHolderCoverageStatus::Partial)
                .count(),
            187
        );
        assert_eq!(
            ledger
                .entries
                .iter()
                .filter(|entry| entry.status == RegistryHolderCoverageStatus::Deferred)
                .count(),
            2
        );
        assert_eq!(
            ledger
                .entries
                .iter()
                .filter(|entry| entry.status == RegistryHolderCoverageStatus::NotApplicable)
                .count(),
            1
        );
        let complete = ledger
            .entries
            .iter()
            .filter(|entry| entry.status == RegistryHolderCoverageStatus::Complete)
            .collect::<Vec<_>>();
        assert_eq!(complete.len(), 4);
        assert_eq!(
            complete
                .iter()
                .map(|entry| entry.resource_id.as_deref().unwrap())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "minecraft:chicken_variant",
                "minecraft:cow_variant",
                "minecraft:pig_variant",
                "minecraft:sound_event",
            ])
        );
    }
}
