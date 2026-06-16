use std::collections::BTreeSet;

use bbb_protocol::packets::{
    KnownPack as ProtocolKnownPack, UpdateEnabledFeatures as ProtocolUpdateEnabledFeatures,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientFeatureState {
    #[serde(default)]
    pub enabled: BTreeSet<String>,
    #[serde(default)]
    pub known_packs: ClientKnownPacksState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientKnownPacksState {
    #[serde(default)]
    pub offered: Vec<KnownPackState>,
    #[serde(default)]
    pub selected: Vec<KnownPackState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownPackState {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

impl WorldStore {
    pub fn apply_update_enabled_features(&mut self, packet: ProtocolUpdateEnabledFeatures) {
        self.counters.update_enabled_features_packets += 1;
        let mut enabled = BTreeSet::new();
        for feature in packet.features {
            if is_known_vanilla_26_1_feature(&feature) {
                enabled.insert(feature);
            } else {
                self.counters.enabled_features_ignored += 1;
            }
        }
        self.features.enabled = enabled;
        self.counters.enabled_features_tracked = self.features.enabled.len();
    }

    pub fn enabled_features(&self) -> &BTreeSet<String> {
        &self.features.enabled
    }

    pub fn enabled_feature_list(&self) -> Vec<String> {
        self.features.enabled.iter().cloned().collect()
    }

    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.features.enabled.contains(feature)
    }

    pub fn apply_select_known_packs(
        &mut self,
        known_packs: Vec<ProtocolKnownPack>,
        selected_packs: Vec<ProtocolKnownPack>,
    ) {
        self.counters.select_known_packs_packets += 1;
        self.features.known_packs = ClientKnownPacksState {
            offered: known_packs.into_iter().map(KnownPackState::from).collect(),
            selected: selected_packs
                .into_iter()
                .map(KnownPackState::from)
                .collect(),
        };
        self.counters.known_packs_offered = self.features.known_packs.offered.len();
        self.counters.known_packs_selected = self.features.known_packs.selected.len();
    }

    pub fn known_packs(&self) -> &ClientKnownPacksState {
        &self.features.known_packs
    }
}

impl From<ProtocolKnownPack> for KnownPackState {
    fn from(pack: ProtocolKnownPack) -> Self {
        Self {
            namespace: pack.namespace,
            id: pack.id,
            version: pack.version,
        }
    }
}

fn is_known_vanilla_26_1_feature(feature: &str) -> bool {
    matches!(
        feature,
        "minecraft:vanilla"
            | "minecraft:trade_rebalance"
            | "minecraft:redstone_experiments"
            | "minecraft:minecart_improvements"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{KnownPack, UpdateEnabledFeatures};

    #[test]
    fn update_enabled_features_replaces_known_feature_set_and_ignores_unknowns() {
        let mut store = WorldStore::new();

        store.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec![
                "minecraft:minecart_improvements".to_string(),
                "minecraft:unknown".to_string(),
                "minecraft:vanilla".to_string(),
                "minecraft:vanilla".to_string(),
            ],
        });

        assert_eq!(
            store.enabled_feature_list(),
            vec![
                "minecraft:minecart_improvements".to_string(),
                "minecraft:vanilla".to_string(),
            ]
        );
        assert!(store.is_feature_enabled("minecraft:vanilla"));
        assert!(!store.is_feature_enabled("minecraft:unknown"));
        let counters = store.counters();
        assert_eq!(counters.update_enabled_features_packets, 1);
        assert_eq!(counters.enabled_features_tracked, 2);
        assert_eq!(counters.enabled_features_ignored, 1);

        store.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec!["minecraft:trade_rebalance".to_string()],
        });

        assert_eq!(
            store.enabled_feature_list(),
            vec!["minecraft:trade_rebalance".to_string()]
        );
        let counters = store.counters();
        assert_eq!(counters.update_enabled_features_packets, 2);
        assert_eq!(counters.enabled_features_tracked, 1);
        assert_eq!(counters.enabled_features_ignored, 1);
    }

    #[test]
    fn select_known_packs_tracks_offered_and_selected_packs() {
        let mut store = WorldStore::new();

        store.apply_select_known_packs(
            vec![
                KnownPack {
                    namespace: "minecraft".to_string(),
                    id: "core".to_string(),
                    version: "26.1".to_string(),
                },
                KnownPack {
                    namespace: "example".to_string(),
                    id: "server".to_string(),
                    version: "1".to_string(),
                },
            ],
            vec![KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "26.1".to_string(),
            }],
        );

        assert_eq!(
            store.known_packs(),
            &ClientKnownPacksState {
                offered: vec![
                    KnownPackState {
                        namespace: "minecraft".to_string(),
                        id: "core".to_string(),
                        version: "26.1".to_string(),
                    },
                    KnownPackState {
                        namespace: "example".to_string(),
                        id: "server".to_string(),
                        version: "1".to_string(),
                    },
                ],
                selected: vec![KnownPackState {
                    namespace: "minecraft".to_string(),
                    id: "core".to_string(),
                    version: "26.1".to_string(),
                }],
            }
        );
        let counters = store.counters();
        assert_eq!(counters.select_known_packs_packets, 1);
        assert_eq!(counters.known_packs_offered, 2);
        assert_eq!(counters.known_packs_selected, 1);
    }
}
