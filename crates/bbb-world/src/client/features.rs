use std::collections::BTreeSet;

use bbb_protocol::packets::UpdateEnabledFeatures as ProtocolUpdateEnabledFeatures;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientFeatureState {
    #[serde(default)]
    pub enabled: BTreeSet<String>,
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
    use bbb_protocol::packets::UpdateEnabledFeatures;

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
}
