use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const VANILLA_PACKET_COVERAGE_26_1: &str = include_str!("../../../docs/packet-coverage-26.1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PacketCoverageStatus {
    Complete,
    Partial,
    Deferred,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketCoverageCell {
    pub status: PacketCoverageStatus,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketCoverageEntry {
    pub state: String,
    pub direction: String,
    pub id: i32,
    pub constant_name: String,
    pub resource_id: String,
    pub class_name: String,
    pub coverage: BTreeMap<String, PacketCoverageCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketCoverageLedger {
    pub schema_version: u32,
    pub vanilla_version: String,
    pub generated_by: String,
    pub update_command: String,
    pub check_command: String,
    pub coverage_overrides: String,
    pub allowed_statuses: Vec<String>,
    pub baseline_policy: String,
    pub entries: Vec<PacketCoverageEntry>,
}

pub fn vanilla_packet_coverage_26_1() -> Result<PacketCoverageLedger, serde_json::Error> {
    serde_json::from_str(VANILLA_PACKET_COVERAGE_26_1)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    use crate::packet_catalog::{PacketDirection, PacketState, VANILLA_PACKET_CATALOGS};

    use super::*;

    #[test]
    fn coverage_ledger_matches_every_packet_catalog_entry_in_order() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        assert_eq!(ledger.schema_version, 1);
        assert_eq!(ledger.vanilla_version, crate::MC_VERSION);
        assert_eq!(ledger.entries.len(), 256);
        assert_eq!(
            ledger
                .allowed_statuses
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "complete".to_string(),
                "partial".to_string(),
                "deferred".to_string(),
                "not-applicable".to_string(),
            ])
        );
        assert_eq!(
            ledger.coverage_overrides,
            "scripts/data/packet-coverage-overrides-26.1.json"
        );

        let mut entries = ledger.entries.iter();
        let mut keys = BTreeSet::new();
        for catalog in VANILLA_PACKET_CATALOGS {
            let state = state_name(catalog.state);
            let direction = direction_name(catalog.direction);
            for packet in catalog.packets {
                let entry = entries.next().expect("coverage row for catalog packet");
                assert_eq!(entry.state, state);
                assert_eq!(entry.direction, direction);
                assert_eq!(entry.id, packet.id);
                assert_eq!(entry.constant_name, packet.constant_name);
                assert_eq!(entry.resource_id, packet.resource_id);
                assert_eq!(entry.class_name, packet.class_name);
                assert!(keys.insert((entry.state.as_str(), entry.direction.as_str(), entry.id)));
            }
        }
        assert!(entries.next().is_none());
    }

    #[test]
    fn coverage_cells_have_direction_schema_and_actionable_partial_gaps() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let clientbound = BTreeSet::from([
            "decode",
            "ownership",
            "canonical_apply",
            "runtime_side_effect",
            "tests",
        ]);
        let serverbound = BTreeSet::from([
            "command_surface",
            "encoder",
            "state_guard",
            "ack_response",
            "tests",
        ]);

        for entry in &ledger.entries {
            let dimensions: BTreeSet<&str> = entry.coverage.keys().map(String::as_str).collect();
            assert_eq!(
                dimensions,
                if entry.direction == "clientbound" {
                    clientbound.clone()
                } else {
                    serverbound.clone()
                }
            );
            for cell in entry.coverage.values() {
                if cell.status == PacketCoverageStatus::Complete {
                    assert!(!cell.evidence.is_empty());
                } else {
                    let gap = cell.gap.as_deref().unwrap_or_default();
                    assert!(gap.starts_with("docs/unsupported-features.md#"));
                }
            }
        }
    }

    #[test]
    fn coverage_evidence_points_to_checked_in_code_or_vanilla_symbols() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

        for entry in &ledger.entries {
            for (dimension, cell) in &entry.coverage {
                for evidence in &cell.evidence {
                    let context = format!(
                        "{}/{}/{}/{}",
                        entry.state, entry.direction, entry.constant_name, dimension
                    );
                    if let Some(locator) = evidence.strip_prefix("vanilla:") {
                        let (path, symbol) = locator
                            .split_once('#')
                            .unwrap_or_else(|| panic!("{context}: malformed vanilla evidence"));
                        assert!(path.ends_with(".java"), "{context}: {evidence}");
                        assert!(!symbol.is_empty(), "{context}: {evidence}");
                        continue;
                    }

                    let (path, symbol) = evidence
                        .split_once('#')
                        .unwrap_or_else(|| panic!("{context}: malformed repo evidence"));
                    assert!(!path.starts_with('/'), "{context}: {evidence}");
                    assert!(
                        !path.split('/').any(|part| part == ".."),
                        "{context}: {evidence}"
                    );
                    let source = fs::read_to_string(repo_root.join(path))
                        .unwrap_or_else(|error| panic!("{context}: cannot read {path}: {error}"));
                    assert!(
                        source.contains(symbol),
                        "{context}: missing {symbol} in {path}"
                    );
                }
            }
        }
    }

    #[test]
    fn status_packet_family_is_closed_with_handshake_transfer_gaps_explicit() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let status_entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "status")
            .collect();
        assert_eq!(status_entries.len(), 4);
        for entry in status_entries {
            for (dimension, cell) in &entry.coverage {
                assert!(
                    matches!(
                        cell.status,
                        PacketCoverageStatus::Complete | PacketCoverageStatus::NotApplicable
                    ),
                    "{}/{dimension} remains {:?}",
                    entry.constant_name,
                    cell.status
                );
            }
        }

        let handshake = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "handshake")
            .unwrap();
        assert_eq!(
            handshake.coverage["encoder"].status,
            PacketCoverageStatus::Complete
        );
        assert_eq!(
            handshake.coverage["tests"].status,
            PacketCoverageStatus::Complete
        );
        assert_eq!(
            handshake.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "state_guard"] {
            assert_eq!(
                handshake.coverage[dimension].status,
                PacketCoverageStatus::Partial
            );
        }
    }

    fn state_name(state: PacketState) -> &'static str {
        match state {
            PacketState::Handshake => "handshake",
            PacketState::Status => "status",
            PacketState::Login => "login",
            PacketState::Configuration => "configuration",
            PacketState::Play => "play",
        }
    }

    fn direction_name(direction: PacketDirection) -> &'static str {
        match direction {
            PacketDirection::Serverbound => "serverbound",
            PacketDirection::Clientbound => "clientbound",
        }
    }
}
