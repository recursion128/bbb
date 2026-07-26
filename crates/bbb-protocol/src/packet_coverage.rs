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
    fn status_and_handshake_packet_families_are_closed() {
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
        for (dimension, cell) in &handshake.coverage {
            assert!(
                matches!(
                    cell.status,
                    PacketCoverageStatus::Complete | PacketCoverageStatus::NotApplicable
                ),
                "{dimension} remains {:?}",
                cell.status
            );
        }
    }

    #[test]
    fn authenticated_and_offline_login_coverage_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let login_entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "login")
            .collect();
        assert_eq!(login_entries.len(), 11);

        for entry in login_entries {
            for (dimension, cell) in &entry.coverage {
                assert!(
                    matches!(
                        cell.status,
                        PacketCoverageStatus::Complete | PacketCoverageStatus::NotApplicable
                    ),
                    "{} {dimension} remains {:?}",
                    entry.constant_name,
                    cell.status
                );
                assert!(
                    !cell.evidence.is_empty(),
                    "{} {dimension} must retain audit evidence",
                    entry.constant_name
                );
            }
        }
    }

    #[test]
    fn configuration_core_control_is_closed() {
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
        assert!(partial.is_empty());
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
    fn configuration_resource_pack_lifecycle_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_RESOURCE_PACK",
            "CLIENTBOUND_RESOURCE_PACK_POP",
            "CLIENTBOUND_RESOURCE_PACK_PUSH",
        ]);
        let entries = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), packets.len());
        for entry in entries {
            assert!(entry.coverage.values().all(|cell| matches!(
                cell.status,
                PacketCoverageStatus::Complete | PacketCoverageStatus::NotApplicable
            )));
        }
    }

    #[test]
    fn configuration_known_packs_negotiation_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from([
            "SERVERBOUND_SELECT_KNOWN_PACKS",
            "CLIENTBOUND_SELECT_KNOWN_PACKS",
        ]);
        let entries = ledger
            .entries
            .iter()
            .filter(|entry| {
                entry.state == "configuration" && packets.contains(entry.constant_name.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), packets.len());
        for entry in &entries {
            assert!(entry.coverage.values().all(|cell| matches!(
                cell.status,
                PacketCoverageStatus::Complete | PacketCoverageStatus::NotApplicable
            )));
        }

        let clientbound = entries
            .iter()
            .find(|entry| entry.constant_name == "CLIENTBOUND_SELECT_KNOWN_PACKS")
            .unwrap();
        let runtime = &clientbound.coverage["runtime_side_effect"];
        assert!(runtime
            .evidence
            .iter()
            .any(|item| item.contains("known_packs.rs#select_offered")));
        assert!(runtime
            .evidence
            .iter()
            .any(|item| item.contains("revision.rs#materialize_pending_known_pack_data")));

        let serverbound = entries
            .iter()
            .find(|entry| entry.constant_name == "SERVERBOUND_SELECT_KNOWN_PACKS")
            .unwrap();
        assert!(serverbound.coverage["command_surface"]
            .evidence
            .iter()
            .any(|item| item.contains("event_stream/configuration.rs")));
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
    fn configuration_presentation_packets_are_complete() {
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

        assert!(entries.iter().all(|entry| {
            entry.coverage.values().all(|cell| {
                matches!(
                    cell.status,
                    PacketCoverageStatus::Complete | PacketCoverageStatus::NotApplicable
                )
            })
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
    fn configuration_dialog_and_custom_click_runtime_is_complete() {
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
        assert!(partial.is_empty(), "partial cells: {partial:?}");
        for packet in ["CLIENTBOUND_CLEAR_DIALOG", "CLIENTBOUND_SHOW_DIALOG"] {
            let entry = entries
                .iter()
                .find(|entry| entry.constant_name == packet)
                .unwrap();
            assert!(entry
                .coverage
                .values()
                .all(|cell| cell.status == PacketCoverageStatus::Complete));
        }
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
    fn configuration_custom_payload_closes_without_external_telemetry() {
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
        assert!(partial.is_empty());
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.constant_name == "CLIENTBOUND_CUSTOM_PAYLOAD")
                .unwrap()
                .coverage["runtime_side_effect"]
                .status,
            PacketCoverageStatus::NotApplicable
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
    fn configuration_registry_packets_are_closed() {
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

        assert!(entries.iter().all(|entry| entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete)));

        for required in [
            "registries/revision.rs#fn build",
            "configuration_registry_and_tag_projections_commit_at_play_transition",
            "hud_inventory_screen_resolves_tag_ghost_recipe_ingredients",
        ] {
            assert!(entries.iter().any(|entry| entry
                .coverage
                .values()
                .flat_map(|cell| &cell.evidence)
                .any(|evidence| evidence.contains(required))));
        }
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
    fn play_common_keepalive_and_ping_are_closed() {
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
        assert!(partial.is_empty());
    }

    #[test]
    fn play_movement_family_is_closed() {
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
        assert!(partial.is_empty());
    }

    #[test]
    fn play_vehicle_movement_closes_local_authority_commands_and_corrections() {
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
        for entry in entries {
            for (dimension, cell) in &entry.coverage {
                let expected = if entry.constant_name == "SERVERBOUND_PADDLE_BOAT"
                    && dimension == "ack_response"
                {
                    PacketCoverageStatus::NotApplicable
                } else {
                    PacketCoverageStatus::Complete
                };
                assert_eq!(cell.status, expected, "{} {dimension}", entry.constant_name);
            }
        }
    }

    #[test]
    fn play_relative_entity_moves_close_authority_and_interpolation() {
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
        assert!(entries.iter().all(|entry| entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete)));
    }

    #[test]
    fn play_absolute_entity_corrections_close_authority_and_interpolation() {
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
        assert!(entries.iter().all(|entry| entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete)));
    }

    #[test]
    fn play_entity_motion_and_rotate_head_are_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let packets = BTreeSet::from(["CLIENTBOUND_SET_ENTITY_MOTION", "CLIENTBOUND_ROTATE_HEAD"]);
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| entry.state == "play" && packets.contains(entry.constant_name.as_str()))
            .collect();
        assert_eq!(entries.len(), packets.len());
        assert!(entries.iter().all(|entry| entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete)));
    }

    #[test]
    fn play_minecart_track_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_MOVE_MINECART_ALONG_TRACK"
            })
            .expect("move minecart along track coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_remove_entities_closes_debug_subscriber_drop() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_REMOVE_ENTITIES"
            })
            .expect("remove entities coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_entity_link_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_ENTITY_LINK"
            })
            .expect("set entity link coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_passengers_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_PASSENGERS"
            })
            .expect("set passengers coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_hurt_animation_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_HURT_ANIMATION"
            })
            .expect("hurt animation coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_animate_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_ANIMATE")
            .expect("animate coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_equipment_closes_component_rich_canonical_and_render_paths() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_EQUIPMENT"
            })
            .expect("set equipment coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_attributes_closes_session_registry_and_instance_resolution() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_ATTRIBUTES"
            })
            .expect("update attributes coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        for dimension in ["canonical_apply", "runtime_side_effect"] {
            assert!(entry.coverage[dimension].evidence.iter().any(|evidence| {
                evidence.contains(
                    "update_attributes_resolves_session_holders_and_validates_entity_supplier",
                )
            }));
        }
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
    fn play_damage_event_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_DAMAGE_EVENT"
            })
            .expect("damage event coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        for dimension in ["canonical_apply", "runtime_side_effect", "tests"] {
            assert!(entry.coverage[dimension].evidence.iter().any(|evidence| {
                evidence.contains("damage_event") || evidence.contains("DamageEvent")
            }));
        }
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
    fn play_set_simulation_distance_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_SET_SIMULATION_DISTANCE"
            })
            .expect("set simulation distance coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_player_abilities_is_complete_after_fov_consumption() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_ABILITIES"
            })
            .expect("player abilities coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_set_time_is_complete_after_world_clock_resolution() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_TIME")
            .expect("set time coverage row");
        let incomplete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status != PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();
        assert!(
            incomplete.is_empty(),
            "incomplete SetTime coverage: {incomplete:?}"
        );
    }

    #[test]
    fn play_ticking_state_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_TICKING_STATE"
            })
            .expect("ticking state coverage row");
        let incomplete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status != PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();
        assert!(
            incomplete.is_empty(),
            "incomplete TickingState coverage: {incomplete:?}"
        );
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
    fn play_title_controls_and_text_components_are_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for constant_name in [
            "CLIENTBOUND_CLEAR_TITLES",
            "CLIENTBOUND_SET_TITLES_ANIMATION",
            "CLIENTBOUND_SET_ACTION_BAR_TEXT",
            "CLIENTBOUND_SET_TITLE_TEXT",
            "CLIENTBOUND_SET_SUBTITLE_TEXT",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == constant_name)
                .expect("title coverage row");
            assert!(entry
                .coverage
                .values()
                .all(|cell| cell.status == PacketCoverageStatus::Complete));
        }
    }

    #[test]
    fn play_chunk_cache_view_is_closed() {
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
            let incomplete: BTreeSet<_> = entry
                .coverage
                .iter()
                .filter_map(|(dimension, cell)| {
                    (cell.status != PacketCoverageStatus::Complete).then_some(dimension.as_str())
                })
                .collect();
            assert!(
                incomplete.is_empty(),
                "incomplete {constant_name}: {incomplete:?}"
            );
        }
    }

    #[test]
    fn play_award_stats_keeps_stats_screen_runtime_open() {
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
        assert_eq!(partial, BTreeSet::from(["runtime_side_effect"]));
    }

    #[test]
    fn play_tab_list_keeps_only_rich_component_canonical_state_open() {
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
        assert_eq!(partial, BTreeSet::from(["canonical_apply"]));
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
    fn play_bundle_delimiter_closes_connection_owned_framing() {
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
        for dimension in ["decode", "ownership", "runtime_side_effect", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
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
    fn play_low_disk_space_warning_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_LOW_DISK_SPACE_WARNING"
            })
            .expect("low disk space warning coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_custom_chat_completions_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS"
            })
            .expect("custom chat completions coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_command_suggestions_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_COMMAND_SUGGESTIONS"
            })
            .expect("command suggestions coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_store_cookie_coverage_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_STORE_COOKIE"
            })
            .expect("play store cookie coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_transfer_closes_reconnect_lifecycle_coverage() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_TRANSFER")
            .expect("play transfer coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_test_instance_status_closes_rich_screen_update() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS"
            })
            .expect("play test instance block status coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_open_book_closes_rich_filtered_pages_and_interactions() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_OPEN_BOOK")
            .expect("play open book coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_open_sign_editor_closes_filtered_text_and_editor_validity() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_OPEN_SIGN_EDITOR"
            })
            .expect("play open sign editor coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("OpenSignEditorState")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("sync_sign_editor")));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("done_layout_and_preedit_projection")));
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
    fn play_open_screen_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_OPEN_SCREEN")
            .expect("play open screen coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("open_screen_exact_title_is_stored")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("all_generated_open_screen_menu_types")));
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
    fn play_container_content_closes_component_rich_canonical_and_screen_paths() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CONTAINER_SET_CONTENT"
            })
            .expect("play container set content coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_container_slot_keeps_creative_sync_and_pop_semantics_open() {
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
    fn play_set_cursor_item_keeps_creative_screen_suppression_open() {
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
    fn play_set_player_inventory_keeps_tutorial_side_effect_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_PLAYER_INVENTORY"
            })
            .expect("play set player inventory coverage row");
        assert_eq!(
            entry.coverage["canonical_apply"].status,
            PacketCoverageStatus::Complete
        );
        assert_eq!(
            entry.coverage["runtime_side_effect"].status,
            PacketCoverageStatus::Partial
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
    fn play_player_combat_kill_is_closed_through_native_death_screen_presentation() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_COMBAT_KILL"
            })
            .expect("play player combat kill coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_scoreboard_packet_family_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for constant_name in [
            "CLIENTBOUND_RESET_SCORE",
            "CLIENTBOUND_SET_DISPLAY_OBJECTIVE",
            "CLIENTBOUND_SET_OBJECTIVE",
            "CLIENTBOUND_SET_SCORE",
            "CLIENTBOUND_SET_PLAYER_TEAM",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == constant_name)
                .unwrap_or_else(|| panic!("play {constant_name} coverage row"));
            assert!(
                entry
                    .coverage
                    .values()
                    .all(|cell| cell.status == PacketCoverageStatus::Complete),
                "{constant_name} must have complete coverage"
            );
        }
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
    fn play_system_chat_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SYSTEM_CHAT")
            .expect("play system chat coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_update_advancements_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_ADVANCEMENTS"
            })
            .expect("play update advancements coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_update_mob_effect_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_MOB_EFFECT"
            })
            .expect("play update mob effect coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_update_recipes_is_complete_across_all_coverage_dimensions() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_RECIPES"
            })
            .expect("play update recipes coverage row");
        assert_eq!(entry.coverage.len(), 5);
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_update_tags_runtime_consumers_are_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_UPDATE_TAGS")
            .expect("play update tags coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_waypoint_locator_bar_presentation_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_WAYPOINT")
            .expect("play waypoint coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_clear_dialog_runtime_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CLEAR_DIALOG"
            })
            .expect("play clear dialog coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_show_dialog_runtime_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_SHOW_DIALOG")
            .expect("play show dialog coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_add_entity_lifecycle_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_ADD_ENTITY")
            .expect("play add entity coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(
            entry.coverage["canonical_apply"]
                .evidence
                .iter()
                .any(|evidence| evidence
                    .contains("packet_entity_type_uses_session_resource_identity"))
        );
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("llama_spit_recreation_batch")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("minecart_and_bee_tickable_sounds")));
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
    fn play_block_destruction_is_complete_with_model_and_block_entity_crumbling() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_DESTRUCTION"
            })
            .expect("play block destruction coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("preserves_arbitrary_baked_quad_geometry")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("block_entity_crumbling_uses_shared_highest_stage")));
    }

    #[test]
    fn play_block_entity_data_identity_and_presentation_are_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_ENTITY_DATA"
            })
            .expect("play block entity data coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["decode"]
            .evidence
            .iter()
            .any(|evidence| evidence
                .contains("block_entity_data_requires_known_type_and_terminal_compound_nbt")));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains(
                "block_entity_data_updates_every_known_matching_type_and_ignores_mismatches"
            )));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence
                .contains("block_entity_data_sign_text_updates_world_through_event_dispatcher")));
    }

    #[test]
    fn play_block_event_registry_dispatch_and_presentation_are_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_EVENT")
            .expect("play block event coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["decode"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("decode_block_event")));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("tunable_note_uses_current_state")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("interpolated_moving_piston_blocks")));
    }

    #[test]
    fn play_block_update_is_complete_through_native_section_reuse() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_BLOCK_UPDATE"
            })
            .expect("play block update coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["decode"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("block_state_registry_report_is_contiguous")));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("block_updates_follow_vanilla_block_entity")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("rebuild_dirty_sections_and_reuse_stable_layers")));
    }

    #[test]
    fn play_boss_event_closes_rich_ordered_lerp_environment_and_music_paths() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_BOSS_EVENT")
            .expect("play boss event coverage row");
        assert!(entry
            .coverage
            .iter()
            .all(|(_, cell)| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["decode"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("decodes_boss_event_operations")));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("linked_hash_map_order")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("interpolates_progress_for_exactly")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("restarts_after_backend_reports")));
    }

    #[test]
    fn play_chunks_biomes_environment_consumers_are_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_CHUNKS_BIOMES"
            })
            .expect("play chunks biomes coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("environment_audio.advance_for_ticks")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("environment_animate.advance_for_ticks")));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("ambient_particles_observe_chunks_biomes")));
    }

    #[test]
    fn play_commands_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_COMMANDS")
            .expect("play commands coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_custom_payload_closes_without_external_telemetry() {
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
        assert!(partial.is_empty());
        assert_eq!(
            entry.coverage["runtime_side_effect"].status,
            PacketCoverageStatus::NotApplicable
        );
    }

    #[test]
    fn play_debug_subscription_packets_close_typed_decode_and_runtime_lifecycle() {
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
            assert!(
                entry
                    .coverage
                    .values()
                    .all(|cell| cell.status == PacketCoverageStatus::Complete),
                "{}",
                entry.constant_name,
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
    fn play_delete_chat_closes_visible_deletion_runtime() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_DELETE_CHAT")
            .expect("play delete chat coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_disguised_chat_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_DISGUISED_CHAT"
            })
            .expect("play disguised chat coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_entity_event_closes_generated_subclass_dispatch_and_runtime_effects() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_ENTITY_EVENT"
            })
            .expect("play entity event coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_explode_closes_typed_audio_and_particle_pipeline() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_EXPLODE")
            .expect("play explode coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_forget_level_chunk_closes_queued_light_cleanup() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_FORGET_LEVEL_CHUNK"
            })
            .expect("play forget level chunk coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_game_event_closes_client_presentation_coverage() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_GAME_EVENT")
            .expect("play game event coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_game_rule_values_closes_typed_screen_apply() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_GAME_RULE_VALUES"
            })
            .expect("play game rule values coverage row");
        let incomplete: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status != PacketCoverageStatus::Complete).then_some(dimension.as_str())
            })
            .collect();
        assert!(
            incomplete.is_empty(),
            "incomplete game-rule values coverage: {incomplete:?}"
        );
    }

    #[test]
    fn play_game_test_highlight_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS"
            })
            .expect("play game test highlight coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|item| item.contains("expire_game_test_highlight_markers_at")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|item| item.contains("debug_always_on_top_text_pass")));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|item| item.contains("game_test_highlight_gizmos_match_vanilla")));
    }

    #[test]
    fn play_initialize_border_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_INITIALIZE_BORDER"
            })
            .expect("play initialize border coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("VignetteRuntime")));
    }

    #[test]
    fn play_level_chunk_with_light_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT"
            })
            .expect("play level chunk with light coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_light_update_closes_vanilla_queue_and_neighbor_rebuild() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_LIGHT_UPDATE"
            })
            .expect("play light update coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_login_closes_full_session_initialization() {
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
        assert!(partial.is_empty());
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
    fn play_merchant_offers_closes_exact_predicate_and_native_presentation() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_MERCHANT_OFFERS"
            })
            .expect("play merchant offers coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["decode"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("supported_component_predicate")));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("replacement_retains_exact_cost_components")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("hud_merchant_trade_tooltip")));
        assert!(entry.coverage["tests"].evidence.iter().any(|evidence| {
            evidence.contains("merchant_component_cost_decorations_and_tooltips")
        }));
    }

    #[test]
    fn play_player_chat_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_CHAT")
            .expect("play player chat coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_player_info_remove_closes_social_manager_and_native_presentation() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_INFO_REMOVE"
            })
            .expect("play player info remove coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("sync_social_interactions_screen")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("push_hud_social_interactions_screen")));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("player_info_remove_marks_retained_social_row")));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("offscreen_social_interactions_draws_shared_rows")));
    }

    #[test]
    fn play_player_info_update_closes_chat_session_and_social_semantics() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_PLAYER_INFO_UPDATE"
            })
            .expect("play player info update coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_remove_mob_effect_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_REMOVE_MOB_EFFECT"
            })
            .expect("play remove mob effect coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_resource_pack_family_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        for packet in [
            "CLIENTBOUND_RESOURCE_PACK_POP",
            "CLIENTBOUND_RESOURCE_PACK_PUSH",
        ] {
            let entry = ledger
                .entries
                .iter()
                .find(|entry| entry.state == "play" && entry.constant_name == packet)
                .expect("play resource pack coverage row");
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
    fn play_respawn_lifecycle_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "CLIENTBOUND_RESPAWN")
            .expect("play respawn coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_section_blocks_update_is_complete_through_ordered_section_reuse() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SECTION_BLOCKS_UPDATE"
            })
            .expect("play section blocks update coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["decode"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("decode_section_blocks_update")));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("apply_in_order_and_reject_impossible")));
        assert!(entry.coverage["runtime_side_effect"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("rebuild_dirty_sections_and_reuse_stable_layers")));
    }

    #[test]
    fn play_incremental_world_border_family_is_closed() {
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
            assert!(
                entry
                    .coverage
                    .values()
                    .all(|cell| cell.status == PacketCoverageStatus::Complete),
                "{packet}"
            );
            assert!(
                entry.coverage["runtime_side_effect"]
                    .evidence
                    .iter()
                    .any(|evidence| evidence.contains("vignette")),
                "{packet}"
            );
        }
    }

    #[test]
    fn play_set_entity_data_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "CLIENTBOUND_SET_ENTITY_DATA"
            })
            .expect("play set entity data coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["decode"].evidence.iter().any(|evidence| {
            evidence.contains("generated_serializer_catalog_has_one_decoder_route_each")
        }));
        assert!(entry.coverage["canonical_apply"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("bind_entity_data_registry_references")));
        for dimension in ["runtime_side_effect", "tests"] {
            assert!(entry.coverage[dimension].evidence.iter().any(|evidence| {
                evidence.contains("test_fixed_entity_matrix_has_no_partial_or_deferred_cells")
            }));
        }
    }

    #[test]
    fn play_serverbound_attack_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_ATTACK")
            .expect("play serverbound attack coverage row");
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert!(["command_surface", "encoder", "state_guard", "tests"]
            .into_iter()
            .all(|dimension| entry.coverage[dimension].status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["state_guard"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("local_player_cannot_attack_with_selected_item")));
        assert!(entry.coverage["tests"].evidence.iter().any(|evidence| {
            evidence.contains(
                "send_attack_entity_and_interact_commands_encode_with_carried_slot_repair",
            )
        }));
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
    fn play_serverbound_signed_chat_command_closes_live_signing() {
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

        assert_eq!(
            complete,
            BTreeSet::from(["command_surface", "encoder", "state_guard", "tests"])
        );
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
    }

    #[test]
    fn play_serverbound_chat_closes_signature_and_echo_state() {
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

        assert_eq!(
            complete,
            BTreeSet::from([
                "ack_response",
                "command_surface",
                "encoder",
                "state_guard",
                "tests"
            ])
        );
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::Complete
        );
    }

    #[test]
    fn play_serverbound_chat_session_update_closes_local_key_lifecycle() {
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

        assert_eq!(
            complete,
            BTreeSet::from([
                "ack_response",
                "command_surface",
                "encoder",
                "state_guard",
                "tests"
            ])
        );
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::Complete
        );
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
    fn play_serverbound_client_tick_end_is_closed() {
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
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_client_information_is_encoder_only_without_option_ui() {
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
        for dimension in ["command_surface", "state_guard"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::NotApplicable,
                "{dimension}"
            );
        }
        for dimension in ["encoder", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_command_suggestion_is_closed() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_COMMAND_SUGGESTION"
            })
            .expect("play serverbound command suggestion coverage row");

        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
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
    fn play_serverbound_container_button_click_is_complete_without_dedicated_ack() {
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
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
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
    fn play_serverbound_debug_subscription_closes_complete_client_lifecycle() {
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
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_serverbound_edit_book_closes_native_editor_and_slot_guards() {
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
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_interact_closes_local_interaction_prediction() {
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
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
        for reverse_guard in [
            "local_entity_interact_generated_owners_are_exhaustively_classified",
            "local_mob_interact_generated_owners_are_exhaustively_classified",
        ] {
            assert!(entry.coverage["tests"]
                .evidence
                .iter()
                .any(|evidence| evidence.contains(reverse_guard)));
        }
    }

    #[test]
    fn play_serverbound_jigsaw_generate_closes_all_applicable_cells() {
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
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_pick_item_family_closes_component_rich_response() {
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

            assert!(entry
                .coverage
                .values()
                .all(|cell| cell.status == PacketCoverageStatus::Complete));
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

    #[test]
    fn play_serverbound_player_action_closes_all_action_paths() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_PLAYER_ACTION"
            })
            .expect("play serverbound player action coverage row");
        for dimension in [
            "ack_response",
            "command_surface",
            "encoder",
            "state_guard",
            "tests",
        ] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_player_command_keeps_response_runtime_gaps_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_PLAYER_COMMAND"
            })
            .expect("play serverbound player command coverage row");
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
    fn play_serverbound_player_input_closes_changed_key_state_reporting() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_PLAYER_INPUT"
            })
            .expect("play serverbound player input coverage row");
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
    fn play_serverbound_recipe_book_commands_keep_seen_ui_lifecycle_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let settings = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS"
            })
            .expect("play serverbound recipe book settings coverage row");
        assert_eq!(
            settings.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
            assert_eq!(
                settings.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "settings {dimension}"
            );
        }

        let seen = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE"
            })
            .expect("play serverbound recipe book seen coverage row");
        assert_eq!(
            seen.coverage["ack_response"].status,
            PacketCoverageStatus::NotApplicable
        );
        assert_eq!(
            seen.coverage["encoder"].status,
            PacketCoverageStatus::Complete
        );
        for dimension in ["command_surface", "state_guard", "tests"] {
            assert_eq!(
                seen.coverage[dimension].status,
                PacketCoverageStatus::Partial,
                "seen {dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_rename_item_keeps_component_response_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_RENAME_ITEM")
            .expect("play serverbound rename item coverage row");
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
    fn play_serverbound_resource_pack_lifecycle_is_complete() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_RESOURCE_PACK"
            })
            .expect("play serverbound resource pack coverage row");
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
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_seen_advancements_is_complete_without_dedicated_ack() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SEEN_ADVANCEMENTS"
            })
            .expect("play serverbound seen advancements coverage row");
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
    fn play_serverbound_select_trade_keeps_component_response_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SELECT_TRADE"
            })
            .expect("play serverbound select trade coverage row");
        assert_eq!(
            entry.coverage["ack_response"].status,
            PacketCoverageStatus::Partial
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
    fn play_serverbound_set_beacon_is_complete_without_dedicated_ack() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_BEACON")
            .expect("play serverbound set beacon coverage row");
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
    fn play_serverbound_set_carried_item_is_complete_without_dedicated_ack() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_CARRIED_ITEM"
            })
            .expect("play serverbound set carried item coverage row");
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
    fn play_serverbound_set_command_block_closes_editor_and_authority_path() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_COMMAND_BLOCK"
            })
            .expect("play serverbound set command block coverage row");
        for dimension in [
            "ack_response",
            "command_surface",
            "encoder",
            "state_guard",
            "tests",
        ] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_set_command_minecart_closes_editor_and_authority_path() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_COMMAND_MINECART"
            })
            .expect("play serverbound set command minecart coverage row");
        for dimension in [
            "ack_response",
            "command_surface",
            "encoder",
            "state_guard",
            "tests",
        ] {
            assert_eq!(
                entry.coverage[dimension].status,
                PacketCoverageStatus::Complete,
                "{dimension}"
            );
        }
    }

    #[test]
    fn play_serverbound_set_creative_mode_slot_keeps_ui_and_authority_response_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_CREATIVE_MODE_SLOT"
            })
            .expect("play serverbound set creative mode slot coverage row");
        let partial: BTreeSet<_> = entry
            .coverage
            .iter()
            .filter_map(|(dimension, cell)| {
                (cell.status == PacketCoverageStatus::Partial).then_some(dimension.as_str())
            })
            .collect();
        assert_eq!(
            partial,
            BTreeSet::from(["ack_response", "command_surface", "state_guard"])
        );
    }

    #[test]
    fn play_serverbound_set_game_rule_closes_typed_screen_and_submit() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_GAME_RULE"
            })
            .expect("play serverbound set game rule coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_serverbound_set_jigsaw_block_closes_editor_and_authority() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_JIGSAW_BLOCK"
            })
            .expect("play serverbound set jigsaw block coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_serverbound_set_structure_block_closes_editor_and_codec() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_STRUCTURE_BLOCK"
            })
            .expect("play serverbound set structure block coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_serverbound_set_test_block_closes_native_editor_path() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SET_TEST_BLOCK"
            })
            .expect("play serverbound set test block coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_serverbound_sign_update_closes_native_editor_parity() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_SIGN_UPDATE")
            .expect("play serverbound sign update coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["state_guard"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("sanitize_sign_clipboard_text")));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("sign_editor_tick_removed_sign_submits_once")));
    }

    #[test]
    fn play_serverbound_spectate_entity_closes_crosshair_action() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_SPECTATE_ENTITY"
            })
            .expect("play serverbound spectate entity coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["command_surface"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("handle_mouse_input_at_partial_tick")));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence
                .contains("spectator_left_mouse_press_on_entity_queues_spectate_only")));
    }

    #[test]
    fn play_serverbound_swing_keeps_local_prediction_parity_open() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_SWING")
            .expect("play serverbound swing coverage row");
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
    fn play_serverbound_teleport_to_entity_closes_spectator_menu_action() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play" && entry.constant_name == "SERVERBOUND_TELEPORT_TO_ENTITY"
            })
            .expect("play serverbound teleport to entity coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["command_surface"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("handle_spectator_hotbar_selected")));
        assert!(entry.coverage["tests"].evidence.iter().any(|evidence| {
            evidence.contains("repeated_hotbar_selection_opens_selects_activates_and_queues_uuid")
        }));
    }

    #[test]
    fn play_serverbound_test_instance_action_closes_editor_and_codec() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.state == "play"
                    && entry.constant_name == "SERVERBOUND_TEST_INSTANCE_BLOCK_ACTION"
            })
            .expect("play serverbound test instance block action coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
    }

    #[test]
    fn play_serverbound_use_item_on_closes_targeted_prediction() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_USE_ITEM_ON")
            .expect("play serverbound use item on coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("generated_item_use_on_owners")));
    }

    #[test]
    fn play_serverbound_use_item_closes_item_semantic_prediction() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entry = ledger
            .entries
            .iter()
            .find(|entry| entry.state == "play" && entry.constant_name == "SERVERBOUND_USE_ITEM")
            .expect("play serverbound use item coverage row");
        assert!(entry
            .coverage
            .values()
            .all(|cell| cell.status == PacketCoverageStatus::Complete));
        assert!(entry.coverage["tests"]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("generated_owner_fluid_modes")));
    }

    #[test]
    fn custom_click_action_is_complete_in_both_states() {
        let ledger = vanilla_packet_coverage_26_1().unwrap();
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|entry| {
                matches!(entry.state.as_str(), "configuration" | "play")
                    && entry.direction == "serverbound"
                    && entry.constant_name == "SERVERBOUND_CUSTOM_CLICK_ACTION"
            })
            .collect();
        assert_eq!(entries.len(), 2);
        for entry in entries {
            assert_eq!(
                entry.coverage["ack_response"].status,
                PacketCoverageStatus::NotApplicable,
                "{}",
                entry.state
            );
            for dimension in ["command_surface", "encoder", "state_guard", "tests"] {
                assert_eq!(
                    entry.coverage[dimension].status,
                    PacketCoverageStatus::Complete,
                    "{} {dimension}",
                    entry.state
                );
            }
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
