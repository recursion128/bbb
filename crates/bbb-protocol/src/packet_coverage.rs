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

    #[test]
    fn offline_login_coverage_keeps_online_encryption_and_raw_disconnect_gaps_explicit() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let login_entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "login")
            .collect();
        assert_eq!(login_entries.len(), 11);

        let partial: BTreeSet<_> = login_entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("SERVERBOUND_HELLO", "ack_response"),
                ("SERVERBOUND_HELLO", "state_guard"),
                ("SERVERBOUND_KEY", "ack_response"),
                ("SERVERBOUND_KEY", "command_surface"),
                ("SERVERBOUND_KEY", "encoder"),
                ("SERVERBOUND_KEY", "state_guard"),
                ("SERVERBOUND_KEY", "tests"),
                ("CLIENTBOUND_LOGIN_DISCONNECT", "decode"),
                ("CLIENTBOUND_HELLO", "decode"),
                ("CLIENTBOUND_HELLO", "runtime_side_effect"),
                ("CLIENTBOUND_HELLO", "tests"),
            ])
        );
    }

    #[test]
    fn configuration_core_control_keeps_frozen_keepalive_deferral_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let core = BTreeSet::from([
            "SERVERBOUND_FINISH_CONFIGURATION",
            "SERVERBOUND_KEEP_ALIVE",
            "SERVERBOUND_PONG",
            "CLIENTBOUND_DISCONNECT",
            "CLIENTBOUND_FINISH_CONFIGURATION",
            "CLIENTBOUND_KEEP_ALIVE",
            "CLIENTBOUND_PING",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && core.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), core.len());
        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([("CLIENTBOUND_KEEP_ALIVE", "runtime_side_effect")])
        );
    }

    #[test]
    fn configuration_cookie_family_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let cookie_packets = BTreeSet::from([
            "SERVERBOUND_COOKIE_RESPONSE",
            "CLIENTBOUND_COOKIE_REQUEST",
            "CLIENTBOUND_STORE_COOKIE",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration"
                    && cookie_packets.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), cookie_packets.len());
        for entry in entries {
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
    }

    #[test]
    fn configuration_resource_pack_wire_is_closed_without_claiming_runtime_downloads() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_RESOURCE_PACK",
            "CLIENTBOUND_RESOURCE_PACK_POP",
            "CLIENTBOUND_RESOURCE_PACK_PUSH",
        ]);
        let partial: BTreeSet<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("SERVERBOUND_RESOURCE_PACK", "command_surface"),
                ("CLIENTBOUND_RESOURCE_PACK_POP", "runtime_side_effect"),
                ("CLIENTBOUND_RESOURCE_PACK_PUSH", "runtime_side_effect"),
            ])
        );
    }

    #[test]
    fn configuration_known_packs_wire_does_not_claim_local_pack_selection() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_SELECT_KNOWN_PACKS",
            "CLIENTBOUND_SELECT_KNOWN_PACKS",
        ]);
        let partial: BTreeSet<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("SERVERBOUND_SELECT_KNOWN_PACKS", "command_surface"),
                ("SERVERBOUND_SELECT_KNOWN_PACKS", "encoder"),
                ("CLIENTBOUND_SELECT_KNOWN_PACKS", "runtime_side_effect"),
            ])
        );
    }

    #[test]
    fn configuration_code_of_conduct_family_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_ACCEPT_CODE_OF_CONDUCT",
            "CLIENTBOUND_CODE_OF_CONDUCT",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), packets.len());
        for entry in entries {
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
    }

    #[test]
    fn configuration_feature_and_chat_reset_packets_are_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "CLIENTBOUND_RESET_CHAT",
            "CLIENTBOUND_UPDATE_ENABLED_FEATURES",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), packets.len());
        for entry in entries {
            for (dimension, cell) in &entry.coverage {
                assert_eq!(
                    cell.status,
                    PacketCoverageStatus::Complete,
                    "{}/{}",
                    entry.constant_name,
                    dimension
                );
            }
        }
    }

    #[test]
    fn configuration_presentation_packets_only_leave_transfer_reconnect_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "CLIENTBOUND_CUSTOM_REPORT_DETAILS",
            "CLIENTBOUND_SERVER_LINKS",
            "CLIENTBOUND_TRANSFER",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), packets.len());

        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([("CLIENTBOUND_TRANSFER", "runtime_side_effect")])
        );
        assert!(entries.iter().all(|entry| {
            entry
                .coverage
                .values()
                .all(|cell| cell.status != PacketCoverageStatus::Deferred)
        }));
    }

    #[test]
    fn configuration_client_information_is_closed_without_an_acknowledgement() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "configuration"
                    && entry.direction == "serverbound"
                    && entry.constant_name == "SERVERBOUND_CLIENT_INFORMATION"
            })
            .unwrap();

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn configuration_dialog_coverage_keeps_codec_and_screen_gaps_explicit() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_CUSTOM_CLICK_ACTION",
            "CLIENTBOUND_CLEAR_DIALOG",
            "CLIENTBOUND_SHOW_DIALOG",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), packets.len());

        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("SERVERBOUND_CUSTOM_CLICK_ACTION", "command_surface"),
                ("SERVERBOUND_CUSTOM_CLICK_ACTION", "encoder"),
                ("SERVERBOUND_CUSTOM_CLICK_ACTION", "state_guard"),
                ("SERVERBOUND_CUSTOM_CLICK_ACTION", "tests"),
                ("CLIENTBOUND_CLEAR_DIALOG", "runtime_side_effect"),
                ("CLIENTBOUND_SHOW_DIALOG", "canonical_apply"),
                ("CLIENTBOUND_SHOW_DIALOG", "decode"),
                ("CLIENTBOUND_SHOW_DIALOG", "runtime_side_effect"),
            ])
        );
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.constant_name == "SERVERBOUND_CUSTOM_CLICK_ACTION")
                .unwrap()
                .coverage["ack_response"]
                .status,
            PacketCoverageStatus::NotApplicable
        );
    }

    #[test]
    fn configuration_custom_payload_only_leaves_brand_telemetry_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from(["SERVERBOUND_CUSTOM_PAYLOAD", "CLIENTBOUND_CUSTOM_PAYLOAD"]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), packets.len());

        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([("CLIENTBOUND_CUSTOM_PAYLOAD", "runtime_side_effect")])
        );
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.constant_name == "SERVERBOUND_CUSTOM_PAYLOAD")
                .unwrap()
                .coverage["ack_response"]
                .status,
            PacketCoverageStatus::NotApplicable
        );
    }

    #[test]
    fn configuration_registry_packets_keep_registry_access_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from(["CLIENTBOUND_REGISTRY_DATA", "CLIENTBOUND_UPDATE_TAGS"]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect();
        assert_eq!(entries.len(), packets.len());

        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("CLIENTBOUND_REGISTRY_DATA", "canonical_apply"),
                ("CLIENTBOUND_REGISTRY_DATA", "runtime_side_effect"),
                ("CLIENTBOUND_UPDATE_TAGS", "runtime_side_effect"),
            ])
        );
    }

    #[test]
    fn play_chunk_batch_feedback_family_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_CHUNK_BATCH_RECEIVED",
            "CLIENTBOUND_CHUNK_BATCH_START",
            "CLIENTBOUND_CHUNK_BATCH_FINISHED",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        for entry in entries {
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
    }

    #[test]
    fn play_common_ping_is_closed_and_keepalive_deferral_stays_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_KEEP_ALIVE",
            "SERVERBOUND_PONG",
            "CLIENTBOUND_KEEP_ALIVE",
            "CLIENTBOUND_PING",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([("CLIENTBOUND_KEEP_ALIVE", "runtime_side_effect")])
        );
    }

    #[test]
    fn play_movement_family_keeps_load_readiness_and_teleport_effects_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_ACCEPT_TELEPORTATION",
            "SERVERBOUND_MOVE_PLAYER_POS",
            "SERVERBOUND_MOVE_PLAYER_POS_ROT",
            "SERVERBOUND_MOVE_PLAYER_ROT",
            "SERVERBOUND_MOVE_PLAYER_STATUS_ONLY",
            "SERVERBOUND_PLAYER_LOADED",
            "CLIENTBOUND_PLAYER_POSITION",
            "CLIENTBOUND_PLAYER_ROTATION",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("CLIENTBOUND_PLAYER_POSITION", "runtime_side_effect"),
                ("SERVERBOUND_PLAYER_LOADED", "command_surface"),
                ("SERVERBOUND_PLAYER_LOADED", "state_guard"),
                ("SERVERBOUND_PLAYER_LOADED", "tests"),
            ])
        );
    }

    #[test]
    fn play_vehicle_movement_keeps_non_boat_prediction_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_MOVE_VEHICLE",
            "SERVERBOUND_PADDLE_BOAT",
            "CLIENTBOUND_MOVE_VEHICLE",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("CLIENTBOUND_MOVE_VEHICLE", "canonical_apply"),
                ("CLIENTBOUND_MOVE_VEHICLE", "runtime_side_effect"),
                ("SERVERBOUND_MOVE_VEHICLE", "ack_response"),
                ("SERVERBOUND_MOVE_VEHICLE", "command_surface"),
                ("SERVERBOUND_MOVE_VEHICLE", "state_guard"),
                ("SERVERBOUND_PADDLE_BOAT", "command_surface"),
                ("SERVERBOUND_PADDLE_BOAT", "state_guard"),
            ])
        );
    }

    #[test]
    fn play_relative_entity_moves_keep_authority_and_interpolation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "CLIENTBOUND_MOVE_ENTITY_POS",
            "CLIENTBOUND_MOVE_ENTITY_POS_ROT",
            "CLIENTBOUND_MOVE_ENTITY_ROT",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("CLIENTBOUND_MOVE_ENTITY_POS", "canonical_apply"),
                ("CLIENTBOUND_MOVE_ENTITY_POS", "runtime_side_effect"),
                ("CLIENTBOUND_MOVE_ENTITY_POS_ROT", "canonical_apply"),
                ("CLIENTBOUND_MOVE_ENTITY_POS_ROT", "runtime_side_effect"),
                ("CLIENTBOUND_MOVE_ENTITY_ROT", "canonical_apply"),
                ("CLIENTBOUND_MOVE_ENTITY_ROT", "runtime_side_effect"),
            ])
        );
    }

    #[test]
    fn play_absolute_entity_corrections_keep_authority_and_interpolation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "CLIENTBOUND_ENTITY_POSITION_SYNC",
            "CLIENTBOUND_TELEPORT_ENTITY",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("CLIENTBOUND_ENTITY_POSITION_SYNC", "canonical_apply"),
                ("CLIENTBOUND_ENTITY_POSITION_SYNC", "runtime_side_effect"),
                ("CLIENTBOUND_TELEPORT_ENTITY", "canonical_apply"),
                ("CLIENTBOUND_TELEPORT_ENTITY", "runtime_side_effect"),
            ])
        );
    }

    #[test]
    fn play_entity_motion_closes_while_living_head_lerp_stays_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from(["CLIENTBOUND_SET_ENTITY_MOTION", "CLIENTBOUND_ROTATE_HEAD"]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        let partial: BTreeSet<_> = entries
            .iter()
            .flat_map(|entry| {
                entry.coverage.iter().filter_map(move |(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial)
                        .then_some((entry.constant_name.as_str(), dimension.as_str()))
                })
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from([
                ("CLIENTBOUND_ROTATE_HEAD", "canonical_apply"),
                ("CLIENTBOUND_ROTATE_HEAD", "runtime_side_effect"),
            ])
        );
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
