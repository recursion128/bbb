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

    #[test]
    fn play_minecart_track_wire_closes_while_queue_semantics_stay_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_MOVE_MINECART_ALONG_TRACK"
            })
            .expect("move minecart along track coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_remove_entities_keeps_removed_vehicle_recovery_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_REMOVE_ENTITIES"
            })
            .expect("remove entities coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_entity_link_keeps_leashable_type_gate_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_ENTITY_LINK"
            })
            .expect("set entity link coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_passengers_keeps_local_mount_presentation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_PASSENGERS"
            })
            .expect("set passengers coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_take_item_entity_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_TAKE_ITEM_ENTITY"
            })
            .expect("take item entity coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_hurt_animation_keeps_living_entity_gate_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_HURT_ANIMATION"
            })
            .expect("hurt animation coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_animate_keeps_wake_up_and_type_gates_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_ANIMATE")
            .expect("animate coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_equipment_keeps_remaining_component_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_EQUIPMENT"
            })
            .expect("set equipment coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_attributes_keeps_registry_and_instance_resolution_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_ATTRIBUTES"
            })
            .expect("update attributes coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_projectile_power_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PROJECTILE_POWER"
            })
            .expect("projectile power coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_damage_event_keeps_living_presentation_and_registry_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_DAMAGE_EVENT"
            })
            .expect("damage event coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_item_cooldown_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_COOLDOWN")
            .expect("cooldown coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_change_difficulty_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CHANGE_DIFFICULTY"
            })
            .expect("change difficulty coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_health_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_HEALTH")
            .expect("set health coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_experience_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_EXPERIENCE"
            })
            .expect("set experience coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_held_slot_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_HELD_SLOT"
            })
            .expect("set held slot coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_simulation_distance_keeps_death_tick_gate_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_SET_SIMULATION_DISTANCE"
            })
            .expect("set simulation distance coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_set_camera_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_CAMERA")
            .expect("set camera coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_default_spawn_position_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION"
            })
            .expect("set default spawn position coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_player_abilities_keeps_walking_speed_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_ABILITIES"
            })
            .expect("player abilities coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_set_time_keeps_world_clock_resolution_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_TIME")
            .expect("set time coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_ticking_state_keeps_variable_tick_cadence_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_TICKING_STATE"
            })
            .expect("ticking state coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_ticking_step_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_TICKING_STEP"
            })
            .expect("ticking step coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_title_controls_close_while_text_components_stay_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for constant_name in [
            "CLIENTBOUND_CLEAR_TITLES",
            "CLIENTBOUND_SET_TITLES_ANIMATION",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == constant_name)
                .expect("title control coverage row");
            assert!(entry
                .coverage
                .values()
                .all(|cell| cell.status == PacketCoverageStatus::Complete));
        }
        for constant_name in [
            "CLIENTBOUND_SET_ACTION_BAR_TEXT",
            "CLIENTBOUND_SET_TITLE_TEXT",
            "CLIENTBOUND_SET_SUBTITLE_TEXT",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == constant_name)
                .expect("title text coverage row");
            let partial: BTreeSet<_> = entry
                .coverage
                .iter()
                .filter_map(|(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
                })
                .collect();
            assert_eq!(
                partial,
                BTreeSet::from(["canonical_apply", "runtime_side_effect"])
            );
        }
    }

    #[test]
    fn play_chunk_cache_view_keeps_residency_pruning_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for constant_name in [
            "CLIENTBOUND_SET_CHUNK_CACHE_CENTER",
            "CLIENTBOUND_SET_CHUNK_CACHE_RADIUS",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == constant_name)
                .expect("chunk cache view coverage row");
            let partial: BTreeSet<_> = entry
                .coverage
                .iter()
                .filter_map(|(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
                })
                .collect();
            assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
        }
    }

    #[test]
    fn play_award_stats_keeps_registry_and_screen_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_AWARD_STATS")
            .expect("award stats coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_tab_list_keeps_rich_component_and_overlay_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_TAB_LIST")
            .expect("tab list coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_server_data_keeps_rich_motd_and_server_list_persistence_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SERVER_DATA")
            .expect("server data coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_bundle_delimiter_keeps_bundle_framing_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BUNDLE_DELIMITER"
            })
            .expect("bundle delimiter coverage row");
        assert_eq!(
            entry.coverage["canonical_apply"].status,
            PacketCoverageStatus::NotApplicable
        );
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_pong_response_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PONG_RESPONSE"
            })
            .expect("pong response coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_low_disk_space_warning_keeps_system_toast_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_LOW_DISK_SPACE_WARNING"
            })
            .expect("low disk space warning coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_select_advancements_tab_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_SELECT_ADVANCEMENTS_TAB"
            })
            .expect("select advancements tab coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_custom_chat_completions_keeps_chat_suggestion_consumer_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS"
            })
            .expect("custom chat completions coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_command_suggestions_keeps_rich_tooltips_and_list_ui_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_COMMAND_SUGGESTIONS"
            })
            .expect("command suggestions coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_cookie_request_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_COOKIE_REQUEST"
            })
            .expect("play cookie request coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_start_configuration_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_START_CONFIGURATION"
            })
            .expect("play start configuration coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_store_cookie_keeps_transfer_lifecycle_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_STORE_COOKIE"
            })
            .expect("play store cookie coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_transfer_keeps_reconnect_lifecycle_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_TRANSFER")
            .expect("play transfer coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_tag_query_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_TAG_QUERY")
            .expect("play tag query coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_test_instance_status_keeps_rich_screen_update_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS"
            })
            .expect("play test instance block status coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_custom_report_details_is_closed_without_immediate_runtime_effect() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CUSTOM_REPORT_DETAILS"
            })
            .expect("play custom report details coverage row");
        assert_eq!(
            entry.coverage["runtime_side_effect"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["canonical_apply", "decode", "ownership", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_server_links_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SERVER_LINKS"
            })
            .expect("play server links coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_disconnect_is_closed_without_canonical_world_apply() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_DISCONNECT")
            .expect("play disconnect coverage row");
        assert_eq!(
            entry.coverage["canonical_apply"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["decode", "ownership", "runtime_side_effect", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_mount_screen_open_keeps_entity_preview_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_MOUNT_SCREEN_OPEN"
            })
            .expect("play mount screen open coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_open_book_keeps_rich_filtered_pages_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_OPEN_BOOK")
            .expect("play open book coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_open_sign_editor_keeps_filtered_text_and_editor_validity_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_OPEN_SIGN_EDITOR"
            })
            .expect("play open sign editor coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_place_ghost_recipe_keeps_remaining_display_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLACE_GHOST_RECIPE"
            })
            .expect("play place ghost recipe coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_open_screen_keeps_rich_title_and_menu_presentation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_OPEN_SCREEN")
            .expect("play open screen coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_container_close_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CONTAINER_CLOSE"
            })
            .expect("play container close coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_container_set_data_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CONTAINER_SET_DATA"
            })
            .expect("play container set data coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_container_content_keeps_remaining_component_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CONTAINER_SET_CONTENT"
            })
            .expect("play container set content coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_container_slot_keeps_remaining_component_and_creative_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CONTAINER_SET_SLOT"
            })
            .expect("play container set slot coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_set_cursor_item_keeps_creative_and_component_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_CURSOR_ITEM"
            })
            .expect("play set cursor item coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_set_player_inventory_keeps_component_and_tutorial_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_PLAYER_INVENTORY"
            })
            .expect("play set player inventory coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_player_combat_enter_is_closed_as_a_vanilla_noop() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_COMBAT_ENTER"
            })
            .expect("play player combat enter coverage row");
        assert_eq!(
            entry.coverage["canonical_apply"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert_eq!(
            entry.coverage["runtime_side_effect"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert!(["decode", "ownership", "tests"]
            .into_iter()
            .all(|dimension| entry.coverage[dimension].status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_player_combat_end_is_closed_as_a_vanilla_noop() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_COMBAT_END"
            })
            .expect("play player combat end coverage row");
        assert_eq!(
            entry.coverage["canonical_apply"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert_eq!(
            entry.coverage["runtime_side_effect"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert!(["decode", "ownership", "tests"]
            .into_iter()
            .all(|dimension| entry.coverage[dimension].status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_player_combat_kill_keeps_death_screen_and_component_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_COMBAT_KILL"
            })
            .expect("play player combat kill coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_reset_score_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_RESET_SCORE")
            .expect("play reset score coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_display_objective_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_DISPLAY_OBJECTIVE"
            })
            .expect("play set display objective coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_objective_keeps_unknown_method_and_rich_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_OBJECTIVE"
            })
            .expect("play set objective coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_set_score_keeps_rich_component_and_number_format_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_SCORE")
            .expect("play set score coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_set_player_team_keeps_unknown_method_and_rich_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_PLAYER_TEAM"
            })
            .expect("play set player team coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_sound_entity_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SOUND_ENTITY"
            })
            .expect("play sound entity coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_sound_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SOUND")
            .expect("play sound coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_stop_sound_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_STOP_SOUND")
            .expect("play stop sound coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_system_chat_keeps_rich_component_presentation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SYSTEM_CHAT")
            .expect("play system chat coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_update_advancements_keeps_rich_display_and_toasts_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_ADVANCEMENTS"
            })
            .expect("play update advancements coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_update_mob_effect_keeps_remaining_client_effect_behaviors_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_MOB_EFFECT"
            })
            .expect("play update mob effect coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_update_recipes_keeps_slot_display_and_presentation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_RECIPES"
            })
            .expect("play update recipes coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_update_tags_keeps_derived_registry_consumers_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_TAGS")
            .expect("play update tags coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_waypoint_keeps_locator_bar_presentation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_WAYPOINT")
            .expect("play waypoint coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_clear_dialog_keeps_screen_stack_restoration_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CLEAR_DIALOG"
            })
            .expect("play clear dialog coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_show_dialog_keeps_typed_dialog_and_screen_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SHOW_DIALOG")
            .expect("play show dialog coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_add_entity_keeps_registry_factory_and_runtime_parity_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_ADD_ENTITY")
            .expect("play add entity coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_block_changed_ack_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_CHANGED_ACK"
            })
            .expect("play block changed ack coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_block_destruction_keeps_full_crack_mesh_presentation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_DESTRUCTION"
            })
            .expect("play block destruction coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_block_entity_data_keeps_typed_codec_and_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_ENTITY_DATA"
            })
            .expect("play block entity data coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_block_event_keeps_registry_validation_and_trigger_families_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_EVENT")
            .expect("play block event coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_block_update_keeps_registry_and_full_level_invalidation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_UPDATE"
            })
            .expect("play block update coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_boss_event_keeps_rich_lerp_order_and_environment_effects_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_BOSS_EVENT")
            .expect("play boss event coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_chunks_biomes_keeps_registry_and_neighbor_rerender_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CHUNKS_BIOMES"
            })
            .expect("play chunks biomes coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_commands_keeps_registry_aware_dispatcher_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_COMMANDS")
            .expect("play commands coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_custom_payload_only_leaves_brand_telemetry_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CUSTOM_PAYLOAD"
            })
            .expect("play custom payload coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_debug_subscription_packets_do_not_treat_raw_payloads_as_typed_support() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "CLIENTBOUND_DEBUG_BLOCK_VALUE",
            "CLIENTBOUND_DEBUG_CHUNK_VALUE",
            "CLIENTBOUND_DEBUG_ENTITY_VALUE",
            "CLIENTBOUND_DEBUG_EVENT",
        ]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());

        for entry in entries {
            let partial: BTreeSet<_> = entry
                .coverage
                .iter()
                .filter_map(|(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
                })
                .collect();
            assert_eq!(
                partial,
                BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"]),
                "{}",
                entry.constant_name
            );
        }
    }

    #[test]
    fn play_debug_sample_closes_typed_tick_time_runtime() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_DEBUG_SAMPLE"
            })
            .expect("play debug sample coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_delete_chat_keeps_visible_deletion_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_DELETE_CHAT")
            .expect("play delete chat coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_disguised_chat_keeps_rich_chat_listener_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_DISGUISED_CHAT"
            })
            .expect("play disguised chat coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_entity_event_keeps_subclass_dispatch_audit_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_ENTITY_EVENT"
            })
            .expect("play entity event coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_explode_does_not_treat_raw_effect_payload_as_typed_support() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_EXPLODE")
            .expect("play explode coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_forget_level_chunk_keeps_runtime_cache_cleanup_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_FORGET_LEVEL_CHUNK"
            })
            .expect("play forget level chunk coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect", "tests"]));
    }

    #[test]
    fn play_game_event_keeps_four_client_presentation_types_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_GAME_EVENT")
            .expect("play game event coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_game_rule_values_keeps_typed_screen_apply_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_GAME_RULE_VALUES"
            })
            .expect("play game rule values coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_game_test_highlight_keeps_marker_gizmo_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS"
            })
            .expect("play game test highlight coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_initialize_border_keeps_warning_vignette_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_INITIALIZE_BORDER"
            })
            .expect("play initialize border coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect", "tests"]));
    }

    #[test]
    fn play_level_chunk_with_light_keeps_typed_terrain_parity_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT"
            })
            .expect("play level chunk with light coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_level_event_closes_handler_switch_coverage() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_LEVEL_EVENT")
            .expect("play level event coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_level_particles_closes_packet_dispatch_coverage() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_LEVEL_PARTICLES"
            })
            .expect("play level particles coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_light_update_keeps_vanilla_queue_and_neighbor_rebuild_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_LIGHT_UPDATE"
            })
            .expect("play light update coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect", "tests"]));
    }

    #[test]
    fn play_login_keeps_full_session_initialization_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_LOGIN")
            .expect("play login coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_map_item_data_closes_map_update_dispatch_coverage() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_MAP_ITEM_DATA"
            })
            .expect("play map item data coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_merchant_offers_keeps_component_aware_trade_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_MERCHANT_OFFERS"
            })
            .expect("play merchant offers coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_player_chat_keeps_secure_chat_validation_and_presentation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_CHAT")
            .expect("play player chat coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_player_info_remove_keeps_social_manager_cleanup_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_INFO_REMOVE"
            })
            .expect("play player info remove coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect", "tests"]));
    }

    #[test]
    fn play_player_info_update_keeps_chat_session_and_social_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_INFO_UPDATE"
            })
            .expect("play player info update coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "decode", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_player_look_at_closes_anchor_resolution_coverage() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_LOOK_AT"
            })
            .expect("play player look at coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_recipe_book_family_locks_add_remove_and_settings_boundaries() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let add = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_RECIPE_BOOK_ADD"
            })
            .expect("play recipe book add coverage row");
        let add_partial: BTreeSet<_> = add
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            add_partial,
            BTreeSet::from(["runtime_side_effect", "tests"])
        );

        for packet in [
            "CLIENTBOUND_RECIPE_BOOK_REMOVE",
            "CLIENTBOUND_RECIPE_BOOK_SETTINGS",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play recipe book coverage row");
            assert!(entry
                .coverage
                .values()
                .all(|cell| cell.status == PacketCoverageStatus::Complete));
        }
    }

    #[test]
    fn play_remove_mob_effect_keeps_broader_effect_consumers_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_REMOVE_MOB_EFFECT"
            })
            .expect("play remove mob effect coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_resource_pack_family_keeps_download_and_prompt_runtime_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for (packet, expected) in [
            (
                "CLIENTBOUND_RESOURCE_PACK_POP",
                BTreeSet::from(["runtime_side_effect"]),
            ),
            (
                "CLIENTBOUND_RESOURCE_PACK_PUSH",
                BTreeSet::from(["canonical_apply", "runtime_side_effect"]),
            ),
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play resource pack coverage row");
            let partial: BTreeSet<_> = entry
                .coverage
                .iter()
                .filter_map(|(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
                })
                .collect();
            assert_eq!(partial, expected, "{packet}");
        }
    }

    #[test]
    fn play_respawn_keeps_full_player_and_level_lifecycle_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_RESPAWN")
            .expect("play respawn coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect", "tests"])
        );
    }

    #[test]
    fn play_section_blocks_update_keeps_full_block_side_effects_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SECTION_BLOCKS_UPDATE"
            })
            .expect("play section blocks update coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["canonical_apply", "runtime_side_effect"])
        );
    }

    #[test]
    fn play_incremental_world_border_family_keeps_warning_vignette_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for packet in [
            "CLIENTBOUND_SET_BORDER_CENTER",
            "CLIENTBOUND_SET_BORDER_LERP_SIZE",
            "CLIENTBOUND_SET_BORDER_SIZE",
            "CLIENTBOUND_SET_BORDER_WARNING_DELAY",
            "CLIENTBOUND_SET_BORDER_WARNING_DISTANCE",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play incremental world border coverage row");
            let partial: BTreeSet<_> = entry
                .coverage
                .iter()
                .filter_map(|(dimension, cell)| {
                    (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
                })
                .collect();
            assert_eq!(
                partial,
                BTreeSet::from(["runtime_side_effect", "tests"]),
                "{packet}"
            );
        }
    }

    #[test]
    fn play_set_entity_data_keeps_schema_and_family_consumers_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_ENTITY_DATA"
            })
            .expect("play set entity data coverage row");
        let complete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(complete, BTreeSet::from(["ownership"]));
    }

    #[test]
    fn play_serverbound_attack_keeps_local_prediction_and_guards_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_ATTACK")
            .expect("play serverbound attack coverage row");
        let complete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(complete, BTreeSet::from(["command_surface", "encoder"]));
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
    }

    #[test]
    fn play_serverbound_tag_query_family_locks_authoritative_response_boundary() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for packet in [
            "SERVERBOUND_BLOCK_ENTITY_TAG_QUERY",
            "SERVERBOUND_ENTITY_TAG_QUERY",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play serverbound tag query coverage row");
            assert!(entry
                .coverage
                .values()
                .all(|cell| cell.status == PacketCoverageStatus::Complete));
        }
    }

    #[test]
    fn play_serverbound_bundle_item_selected_closes_without_a_dedicated_acknowledgement() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_BUNDLE_ITEM_SELECTED"
            })
            .expect("play serverbound bundle item selected coverage row");

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
    fn play_serverbound_difficulty_and_game_mode_commands_keep_server_authority() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for packet in [
            "SERVERBOUND_CHANGE_DIFFICULTY",
            "SERVERBOUND_CHANGE_GAME_MODE",
            "SERVERBOUND_LOCK_DIFFICULTY",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play serverbound difficulty command coverage row");
            assert!(entry
                .coverage
                .values()
                .all(|cell| cell.status == PacketCoverageStatus::Complete));
        }
    }

    #[test]
    fn play_serverbound_chat_ack_closes_without_a_server_response() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_CHAT_ACK")
            .expect("play serverbound chat ack coverage row");

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
    fn play_serverbound_unsigned_chat_command_keeps_signed_arguments_separate() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CHAT_COMMAND"
            })
            .expect("play serverbound unsigned chat command coverage row");

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
    fn play_serverbound_signed_chat_command_keeps_production_signing_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CHAT_COMMAND_SIGNED"
            })
            .expect("play serverbound signed chat command coverage row");
        let complete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();

        assert_eq!(complete, BTreeSet::from(["encoder"]));
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_chat_keeps_production_signature_and_echo_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_CHAT")
            .expect("play serverbound chat coverage row");
        let complete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();

        assert_eq!(complete, BTreeSet::from(["encoder"]));
        for dimension in ["ack_response", "command_surface", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_chat_session_update_keeps_key_lifecycle_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CHAT_SESSION_UPDATE"
            })
            .expect("play serverbound chat session update coverage row");
        let complete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();

        assert_eq!(complete, BTreeSet::from(["encoder"]));
        for dimension in ["ack_response", "command_surface", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_client_command_keeps_ui_and_response_lifecycles_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CLIENT_COMMAND"
            })
            .expect("play serverbound client command coverage row");
        let complete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();

        assert_eq!(complete, BTreeSet::from(["encoder"]));
        for dimension in ["ack_response", "command_surface", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_client_tick_end_keeps_tick_ordering_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CLIENT_TICK_END"
            })
            .expect("play serverbound client tick end coverage row");

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "encoder"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        for dimension in ["state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_client_information_keeps_change_dedup_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CLIENT_INFORMATION"
            })
            .expect("play serverbound client information coverage row");

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "encoder"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        for dimension in ["state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_command_suggestion_keeps_dispatcher_and_ui_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_COMMAND_SUGGESTION"
            })
            .expect("play serverbound command suggestion coverage row");

        for dimension in ["command_surface", "encoder"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        for dimension in ["ack_response", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_configuration_acknowledged_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "SERVERBOUND_CONFIGURATION_ACKNOWLEDGED"
            })
            .expect("play serverbound configuration acknowledged coverage row");

        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_serverbound_container_button_click_keeps_local_menu_mutation_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CONTAINER_BUTTON_CLICK"
            })
            .expect("play serverbound container button click coverage row");

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "encoder"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        for dimension in ["state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_container_click_keeps_specialized_prediction_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CONTAINER_CLICK"
            })
            .expect("play serverbound container click coverage row");

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "encoder"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        for dimension in ["state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_container_close_is_closed_without_a_server_response() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CONTAINER_CLOSE"
            })
            .expect("play serverbound container close coverage row");

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
    fn play_serverbound_container_slot_state_keeps_crafter_swap_and_local_state_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED"
            })
            .expect("play serverbound container slot state coverage row");

        assert_eq!(
            entry.coverage["encoder"].status,
            PacketCoverageStatus::Complete
        );
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_cookie_response_is_closed_without_a_second_response() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_COOKIE_RESPONSE"
            })
            .expect("play serverbound cookie response coverage row");

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
    fn play_serverbound_custom_payload_is_closed_without_a_server_response() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_CUSTOM_PAYLOAD"
            })
            .expect("play serverbound custom payload coverage row");

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
    fn play_serverbound_debug_subscription_keeps_registry_family_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST"
            })
            .expect("play serverbound debug subscription coverage row");

        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Partial));
    }

    #[test]
    fn play_serverbound_edit_book_keeps_native_editor_and_slot_guards_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_EDIT_BOOK")
            .expect("play serverbound edit book coverage row");

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert_eq!(
            entry.coverage["encoder"].status,
            PacketCoverageStatus::Complete
        );
        for dimension in ["command_surface", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_interact_keeps_local_interaction_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_INTERACT")
            .expect("play serverbound interact coverage row");

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "encoder"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        for dimension in ["state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_jigsaw_generate_keeps_game_master_screen_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_JIGSAW_GENERATE"
            })
            .expect("play serverbound jigsaw generate coverage row");

        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert_eq!(
            entry.coverage["encoder"].status,
            PacketCoverageStatus::Complete
        );
        for dimension in ["command_surface", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_pick_item_family_keeps_component_response_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for packet in [
            "SERVERBOUND_PICK_ITEM_FROM_BLOCK",
            "SERVERBOUND_PICK_ITEM_FROM_ENTITY",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play serverbound pick item coverage row");

            for dimension in ["command_surface", "encoder", "state_guard"] {
                assert_eq!(
                    entry.coverage[dimension].status,
                    PacketCoverageStatus::Complete,
                    "{packet} {dimension}"
                );
            }
            for dimension in ["ack_response", "tests"] {
                assert_eq!(
                    entry.coverage[dimension].status,
                    PacketCoverageStatus::Partial,
                    "{packet} {dimension}"
                );
            }
        }
    }

    #[test]
    fn play_serverbound_ping_request_closes_debug_monitor_round_trip() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for packet in ["SERVERBOUND_PING_REQUEST", "CLIENTBOUND_PONG_RESPONSE"] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play ping monitor coverage row");
            assert!(
                entry
                    .coverage
                    .values()
                    .all(|cell| cell.status == PacketCoverageStatus::Complete),
                "{packet}"
            );
        }
    }

    #[test]
    fn play_serverbound_place_recipe_keeps_response_semantics_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_PLACE_RECIPE"
            })
            .expect("play serverbound place recipe coverage row");
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::Partial
        );
    }

    #[test]
    fn play_serverbound_player_abilities_closes_flight_toggle_authority() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_PLAYER_ABILITIES"
            })
            .expect("play serverbound player abilities coverage row");
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
