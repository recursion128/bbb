use serde::{Deserialize, Serialize};

const VANILLA_REGISTRY_HOLDER_COVERAGE_26_1: &str =
    include_str!("../../../docs/evidence/vanilla-26.1/registry-holder-consumer-coverage.json");

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
    fn coverage_uses_narrow_families_and_closes_every_inventory_row() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let counts = ledger
            .entries
            .iter()
            .fold(BTreeMap::new(), |mut counts, entry| {
                *counts.entry(entry.family.as_str()).or_insert(0usize) += 1;
                counts
            });
        assert_eq!(counts["network-holder-or-inline-instrument"], 1);
        assert_eq!(
            counts["network-holder-or-inline-banner-pattern-synchronized"],
            1
        );
        assert_eq!(counts["network-holder-or-inline-chat-type-synchronized"], 1);
        assert_eq!(counts["network-holder-or-inline-dialog-synchronized"], 1);
        assert_eq!(
            counts["network-holder-or-inline-jukebox-song-synchronized"],
            1
        );
        assert_eq!(counts["network-holder-or-inline-sound-event"], 1);
        assert_eq!(counts["network-holder-or-inline-trims-synchronized"], 2);
        assert_eq!(
            counts["network-holder-or-inline-painting-variant-synchronized"],
            1
        );
        assert_eq!(counts["network-holder-reference-animal-sound-variants"], 5);
        assert_eq!(counts["network-holder-reference-attribute-built-in"], 1);
        assert_eq!(counts["network-holder-reference-cat-variant"], 1);
        assert_eq!(counts["network-holder-reference-damage-type"], 1);
        assert_eq!(counts["network-holder-reference-dimension-type"], 1);
        assert_eq!(
            counts["network-holder-reference-enchantment-synchronized"],
            1
        );
        assert_eq!(counts["network-holder-reference-farm-animal-variants"], 3);
        assert_eq!(counts["network-holder-reference-frog-variant"], 1);
        assert_eq!(counts["network-holder-reference-item-built-in"], 2);
        assert_eq!(counts["network-holder-reference-map-decoration-type"], 1);
        assert_eq!(counts["network-holder-reference-mob-effect-built-in"], 1);
        assert_eq!(counts["network-holder-reference-potion-built-in"], 1);
        assert_eq!(counts["network-holder-reference-villager-built-ins"], 3);
        assert_eq!(counts["network-holder-reference-world-clock"], 1);
        assert_eq!(counts["network-holder-reference-wolf-variant"], 1);
        assert_eq!(
            counts["network-holder-reference-zombie-nautilus-variant"],
            1
        );
        assert_eq!(counts["network-holder-reference-debug"], 2);
        assert_eq!(counts["network-holder-reference-helper"], 1);
        assert_eq!(counts["network-holder-set-or-tag"], 10);
        assert_eq!(counts["network-holder-set-or-tag-banner-pattern"], 1);
        assert_eq!(counts["network-tag-slot-display"], 1);
        assert_eq!(counts["persistent-hashed-tag-server-only"], 6);
        assert_eq!(
            counts["persistent-hashed-tag-dimension-type-synchronized"],
            1
        );
        assert_eq!(counts["persistent-holder-or-inline-dialog-synchronized"], 1);
        assert_eq!(counts["persistent-holder-or-inline-client"], 3);
        assert_eq!(counts["persistent-holder-or-inline-server-only"], 20);
        assert_eq!(counts["persistent-holder-or-inline-banner-pattern"], 1);
        assert_eq!(counts["persistent-holder-reference-client"], 25);
        assert_eq!(counts["persistent-holder-reference-server-only"], 13);
        assert_eq!(counts["persistent-holder-reference-painting-variant"], 1);
        assert_eq!(
            counts["persistent-holder-reference-timeline-synchronized"],
            1
        );
        assert_eq!(
            counts["persistent-holder-reference-world-clock-synchronized"],
            1
        );
        assert_eq!(counts["persistent-holder-or-inline-trims"], 2);
        assert_eq!(counts["persistent-holder-set-custom-server-only"], 3);
        assert_eq!(
            counts["persistent-holder-set-or-inline-dialog-synchronized"],
            1
        );
        assert_eq!(counts["persistent-holder-set-or-inline-server-only"], 5);
        assert_eq!(counts["persistent-holder-set-or-tag-client-components"], 14);
        assert_eq!(
            counts["persistent-holder-set-or-tag-enchantment-synchronized"],
            6
        );
        assert_eq!(counts["persistent-holder-set-or-tag-server-only"], 28);
        assert_eq!(counts["persistent-holder-set-or-tag-banner-pattern"], 1);
        assert_eq!(
            counts["persistent-holder-set-or-tag-timeline-synchronized"],
            1
        );
        assert_eq!(counts["persistent-tag-server-only"], 11);
        assert_eq!(
            ledger
                .entries
                .iter()
                .filter(|entry| entry.status == RegistryHolderCoverageStatus::Partial)
                .count(),
            0
        );
        assert_eq!(
            ledger
                .entries
                .iter()
                .filter(|entry| entry.status == RegistryHolderCoverageStatus::Deferred)
                .count(),
            0
        );
        assert_eq!(
            ledger
                .entries
                .iter()
                .filter(|entry| entry.status == RegistryHolderCoverageStatus::NotApplicable)
                .count(),
            87
        );
        let complete = ledger
            .entries
            .iter()
            .filter(|entry| entry.status == RegistryHolderCoverageStatus::Complete)
            .collect::<Vec<_>>();
        assert_eq!(complete.len(), 107);
        assert_eq!(
            complete
                .iter()
                .map(|entry| entry.resource_id.as_deref().unwrap())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "minecraft:attribute",
                "minecraft:banner_pattern",
                "minecraft:block",
                "minecraft:chicken_variant",
                "minecraft:chicken_sound_variant",
                "minecraft:cat_sound_variant",
                "minecraft:cat_variant",
                "minecraft:chat_type",
                "minecraft:cow_sound_variant",
                "minecraft:cow_variant",
                "minecraft:damage_type",
                "minecraft:dimension_type",
                "minecraft:dialog",
                "minecraft:enchantment",
                "minecraft:entity_type",
                "minecraft:frog_variant",
                "minecraft:game_event",
                "minecraft:item",
                "minecraft:instrument",
                "minecraft:jukebox_song",
                "minecraft:map_decoration_type",
                "minecraft:mob_effect",
                "minecraft:pig_variant",
                "minecraft:pig_sound_variant",
                "minecraft:painting_variant",
                "minecraft:potion",
                "minecraft:point_of_interest_type",
                "minecraft:sound_event",
                "minecraft:test_environment",
                "minecraft:trim_material",
                "minecraft:trim_pattern",
                "minecraft:timeline",
                "minecraft:villager_profession",
                "minecraft:villager_type",
                "minecraft:wolf_sound_variant",
                "minecraft:wolf_variant",
                "minecraft:world_clock",
                "minecraft:zombie_nautilus_variant",
            ])
        );
    }

    #[test]
    fn complete_network_holder_set_family_keeps_wire_registry_and_consumer_guards() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let entries = ledger
            .entries
            .iter()
            .filter(|entry| entry.family == "network-holder-set-or-tag")
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 10);
        assert!(entries
            .iter()
            .all(|entry| entry.status == RegistryHolderCoverageStatus::Complete));
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.resource_id.as_deref().unwrap())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "minecraft:block",
                "minecraft:damage_type",
                "minecraft:entity_type",
                "minecraft:item",
                "minecraft:mob_effect",
            ])
        );

        let components = include_str!("packets/data_components.rs");
        for required in [
            "enum NetworkHolderSetRegistry {",
            "fn decode_holder_set_summary(",
            "Self::EntityType => vanilla_entity_type(holder_id).is_some()",
            "Self::Item => crate::item_types::vanilla_item_type(holder_id).is_some()",
            "Self::MobEffect => vanilla_mob_effect_type(holder_id).is_some()",
        ] {
            assert!(
                components.contains(required),
                "network holder-set decode lost structural guard {required}"
            );
        }

        let recipes = include_str!("packets/client_features/recipes.rs");
        for required in [
            "fn decode_ingredient_summary(",
            "ingredient holder set must not be empty",
            "inventory::decode_non_empty_item_holder_id(decoder)?",
        ] {
            assert!(
                recipes.contains(required),
                "Ingredient decode lost structural guard {required}"
            );
        }

        let world_recipes = include_str!("../../bbb-world/src/recipes.rs");
        for required in [
            "pub(crate) fn ingredient_holder_set_is_resolved(",
            "access.holder_set_by_tag(\"minecraft:item\", tag)",
            ".holder_set_by_holder_ids(\"minecraft:item\", holder_ids)",
            "entry.resource_id != \"minecraft:air\"",
        ] {
            assert!(
                world_recipes.contains(required),
                "Ingredient consumer lost structural guard {required}"
            );
        }

        let persistent = include_str!("../../bbb-native/src/input/component_patch.rs");
        for required in [
            "pub(super) fn registry_holder_set_snbt(",
            "access.holder_set_by_tag(registry, tag)",
            ".holder_set_by_holder_ids(registry, holder_ids)",
            "fn block_holder_set_snbt(",
            "fn entity_type_holder_set_snbt(",
            "fn fixed_network_holder_set_encoders_follow_tag_revisions_and_reject_missing_tags(",
            "fn damage_type_encoders_use_session_order_or_fixed_catalog(",
        ] {
            assert!(
                persistent.contains(required),
                "network holder-set persistence lost structural guard {required}"
            );
        }
    }

    #[test]
    fn final_tag_and_custom_holder_set_families_keep_client_and_server_boundaries() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let network = ledger
            .entries
            .iter()
            .filter(|entry| entry.family == "network-tag-slot-display")
            .collect::<Vec<_>>();
        assert_eq!(network.len(), 1);
        assert_eq!(network[0].status, RegistryHolderCoverageStatus::Complete);
        assert_eq!(network[0].resource_id.as_deref(), Some("minecraft:item"));
        assert!(network[0]
            .consumer
            .as_deref()
            .is_some_and(|consumer| consumer.contains("resolve_slot_display_first_stack")));
        for required in [
            "decode_slot_display_value",
            "decodes_update_recipes_with_tag_slot_display",
            "push_slot_display_item_stack_candidates",
            "hud_inventory_screen_keeps_resolved_tag_ghost_recipe_snapshot",
            "stonecutter_buttons_and_tooltips_follow_current_tag_and_trim_context",
        ] {
            assert!(network[0]
                .evidence
                .iter()
                .any(|evidence| evidence.contains(required)));
        }

        for (family, count) in [
            ("persistent-hashed-tag-server-only", 6),
            ("persistent-holder-set-custom-server-only", 3),
            ("persistent-holder-set-or-inline-server-only", 5),
            ("persistent-tag-server-only", 11),
        ] {
            let entries = ledger
                .entries
                .iter()
                .filter(|entry| entry.family == family)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), count, "final server-only family {family}");
            assert!(entries.iter().all(|entry| {
                entry.status == RegistryHolderCoverageStatus::NotApplicable
                    && entry.consumer.is_none()
            }));
        }

        let custom = ledger
            .entries
            .iter()
            .find(|entry| entry.family == "persistent-holder-set-custom-server-only")
            .unwrap();
        assert!(custom
            .evidence
            .iter()
            .any(|evidence| evidence.contains("Ingredient.java")));

        let homogeneous = ledger
            .entries
            .iter()
            .find(|entry| entry.family == "persistent-holder-set-or-inline-server-only")
            .unwrap();
        assert!(homogeneous
            .evidence
            .iter()
            .any(|evidence| evidence.contains("Biome.NETWORK_CODEC")));

        let persistent_tags = ledger
            .entries
            .iter()
            .find(|entry| entry.family == "persistent-tag-server-only")
            .unwrap();
        assert!(persistent_tags
            .decision
            .contains("recipe displays reach the client through SlotDisplay.STREAM_CODEC"));
    }

    #[test]
    fn persistent_holder_set_families_keep_client_sync_and_server_boundaries() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let expected = [
            (
                "persistent-holder-set-or-tag-client-components",
                14,
                RegistryHolderCoverageStatus::Complete,
            ),
            (
                "persistent-holder-set-or-tag-enchantment-synchronized",
                6,
                RegistryHolderCoverageStatus::Complete,
            ),
            (
                "persistent-holder-set-or-tag-server-only",
                28,
                RegistryHolderCoverageStatus::NotApplicable,
            ),
        ];
        let mut total = 0;
        for (family, count, status) in expected {
            let entries = ledger
                .entries
                .iter()
                .filter(|entry| entry.family == family)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), count, "persistent family {family}");
            assert!(
                entries.iter().all(|entry| entry.status == status),
                "persistent family {family} status"
            );
            total += entries.len();
        }
        assert_eq!(total, 48);

        let state = include_str!("../../bbb-world/src/registries/state.rs");
        for required in [
            "pub enum RegistryHolderSetIdentity {",
            "pub fn persistent_holder_set_from_nbt(",
            "pub fn holder_set_by_identity(",
            "pub fn identity(&self) -> RegistryHolderSetIdentity",
        ] {
            assert!(
                state.contains(required),
                "persistent registry resolver lost structural guard {required}"
            );
        }

        let dialog =
            include_str!("../../bbb-world/src/client/dialog/body/component_value/holder_set.rs");
        assert!(dialog.contains(".persistent_holder_set_from_nbt(registry, value)"));
        let persistent = include_str!("../../bbb-native/src/input/component_patch.rs");
        assert!(persistent.contains("pub(super) fn registry_holder_set_snbt("));

        let enchantments = include_str!("../../bbb-world/src/registries/enchantments.rs");
        for required in [
            "pub fn synchronized_enchantment_holder_sets(",
            "fn validate_enchantment_effect_holder_sets(",
            ".persistent_holder_set_from_nbt(\"minecraft:item\", supported_items)",
            ".holder_set_by_identity(\"minecraft:item\"",
        ] {
            assert!(
                enchantments.contains(required),
                "synchronized enchantment HolderSet lost structural guard {required}"
            );
        }

        let server_only = ledger
            .entries
            .iter()
            .filter(|entry| entry.family == "persistent-holder-set-or-tag-server-only")
            .collect::<Vec<_>>();
        assert!(server_only.iter().all(|entry| entry.consumer.is_none()));
        assert!(server_only.iter().all(|entry| entry
            .evidence
            .iter()
            .any(|evidence| evidence.contains("RegistrySynchronization.java"))));
    }

    #[test]
    fn persistent_holder_reference_families_keep_named_identity_boundaries() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let client = ledger
            .entries
            .iter()
            .filter(|entry| entry.family == "persistent-holder-reference-client")
            .collect::<Vec<_>>();
        assert_eq!(client.len(), 25);
        assert!(client
            .iter()
            .all(|entry| entry.status == RegistryHolderCoverageStatus::Complete));
        assert_eq!(
            client
                .iter()
                .map(|entry| entry.source_path.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            24
        );
        for source in [
            "net/minecraft/world/damagesource/DamageType.java",
            "net/minecraft/world/entity/animal/wolf/WolfVariant.java",
            "net/minecraft/world/item/component/DebugStickState.java",
            "net/minecraft/world/level/gameevent/GameEvent.java",
            "net/minecraft/world/level/saveddata/maps/MapDecorationType.java",
        ] {
            assert!(client.iter().any(|entry| entry.source_path == source));
        }

        let server_only = ledger
            .entries
            .iter()
            .filter(|entry| entry.family == "persistent-holder-reference-server-only")
            .collect::<Vec<_>>();
        assert_eq!(server_only.len(), 13);
        assert!(server_only.iter().all(|entry| {
            entry.status == RegistryHolderCoverageStatus::NotApplicable && entry.consumer.is_none()
        }));
        assert_eq!(client.len() + server_only.len(), 38);

        let state = include_str!("../../bbb-world/src/registries/state.rs");
        for required in [
            "pub enum PersistentHolderDecodeError {",
            "pub fn persistent_holder_from_nbt(",
            ".persistent_holder_from_nbt(registry, value)",
            "vanilla_debug_game_event_type_by_id(holder_id)",
            "| \"minecraft:game_event\"",
        ] {
            assert!(
                state.contains(required),
                "persistent holder resolver lost structural guard {required}"
            );
        }

        let dialog =
            include_str!("../../bbb-world/src/client/dialog/body/component_value/holder_set.rs");
        assert!(dialog.contains(".persistent_holder_from_nbt(registry, value)"));
        let enchantments = include_str!("../../bbb-world/src/registries/enchantments.rs");
        for required in [
            "Some((\"damage_type\", \"minecraft:damage_type\", true))",
            "Some((\"trigger_game_event\", \"minecraft:game_event\", false))",
            ".persistent_holder_from_nbt(registry, value)",
        ] {
            assert!(
                enchantments.contains(required),
                "synchronized holder reference lost structural guard {required}"
            );
        }

        let persistent = include_str!("../../bbb-native/src/input/component_patch.rs");
        for required in [
            "pub(super) fn registry_resource_id(",
            "fn encode_damage_type(",
            "fn encode_enchantments(",
            "fn encode_named_variant(",
            "fn damage_type_encoders_use_session_order_or_fixed_catalog(",
            "fn named_variant_encoders_use_session_order_or_fixed_catalog(",
        ] {
            assert!(
                persistent.contains(required),
                "persistent holder reconstruction lost structural guard {required}"
            );
        }
    }

    #[test]
    fn holder_or_inline_closure_keeps_client_and_server_boundaries() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        for (family, count, status) in [
            (
                "network-holder-or-inline-instrument",
                1,
                RegistryHolderCoverageStatus::Complete,
            ),
            (
                "persistent-holder-or-inline-client",
                3,
                RegistryHolderCoverageStatus::Complete,
            ),
            (
                "persistent-holder-or-inline-server-only",
                20,
                RegistryHolderCoverageStatus::NotApplicable,
            ),
        ] {
            let entries = ledger
                .entries
                .iter()
                .filter(|entry| entry.family == family)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), count, "holder-or-inline family {family}");
            assert!(
                entries.iter().all(|entry| entry.status == status),
                "holder-or-inline family {family} status"
            );
        }

        let client = ledger
            .entries
            .iter()
            .filter(|entry| entry.family == "persistent-holder-or-inline-client")
            .map(|entry| entry.source_path.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            client,
            BTreeSet::from([
                "net/minecraft/gametest/framework/TestEnvironmentDefinition.java",
                "net/minecraft/sounds/SoundEvent.java",
                "net/minecraft/world/item/Instrument.java",
            ])
        );
        let server_only = ledger
            .entries
            .iter()
            .filter(|entry| entry.family == "persistent-holder-or-inline-server-only")
            .collect::<Vec<_>>();
        assert!(server_only.iter().all(|entry| entry.consumer.is_none()));
        assert_eq!(
            server_only
                .iter()
                .map(|entry| entry.source_path.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "net/minecraft/core/RegistryCodecs.java",
                "net/minecraft/world/level/biome/Biome.java",
                "net/minecraft/world/level/biome/MultiNoiseBiomeSourceParameterList.java",
                "net/minecraft/world/level/block/entity/trialspawner/TrialSpawnerConfig.java",
                "net/minecraft/world/level/dimension/DimensionType.java",
                "net/minecraft/world/level/levelgen/DensityFunction.java",
                "net/minecraft/world/level/levelgen/NoiseGeneratorSettings.java",
                "net/minecraft/world/level/levelgen/carver/ConfiguredWorldCarver.java",
                "net/minecraft/world/level/levelgen/feature/ConfiguredFeature.java",
                "net/minecraft/world/level/levelgen/flat/FlatLevelGeneratorPreset.java",
                "net/minecraft/world/level/levelgen/placement/PlacedFeature.java",
                "net/minecraft/world/level/levelgen/presets/WorldPreset.java",
                "net/minecraft/world/level/levelgen/structure/Structure.java",
                "net/minecraft/world/level/levelgen/structure/StructureSet.java",
                "net/minecraft/world/level/levelgen/structure/pools/StructureTemplatePool.java",
                "net/minecraft/world/level/levelgen/structure/templatesystem/StructureProcessorType.java",
                "net/minecraft/world/level/levelgen/synth/NormalNoise.java",
                "net/minecraft/world/level/storage/loot/LootTable.java",
                "net/minecraft/world/level/storage/loot/functions/LootItemFunctions.java",
                "net/minecraft/world/level/storage/loot/predicates/LootItemCondition.java",
            ])
        );

        let protocol = include_str!("packets/data_components.rs");
        for required in [
            "pub instrument_direct: Option<Box<InstrumentSummary>>",
            "pub struct InstrumentSummary {",
            "fn decode_direct_instrument_summary(",
            "fn decodes_inline_instrument_direct_sound_and_exact_component()",
        ] {
            assert!(
                protocol.contains(required),
                "Instrument protocol path lost structural guard {required}"
            );
        }

        let state = include_str!("../../bbb-world/src/registries/state.rs");
        assert!(state.contains("pub(crate) enum PersistentHolderOrInline<'a>"));
        assert!(state.contains("pub(crate) fn persistent_holder_or_inline_from_nbt("));
        let sounds = include_str!("../../bbb-world/src/registries/sound_events.rs");
        for required in [
            "pub(crate) fn decode_persistent_sound_event(",
            "vanilla_sound_event_registry_id(&resource_id)",
            "NbtSummaryValue::Compound(entries) => decode_direct(entries)",
        ] {
            assert!(
                sounds.contains(required),
                "SoundEvent persistent path lost structural guard {required}"
            );
        }
        let game_tests = include_str!("../../bbb-world/src/registries/game_tests.rs");
        assert!(game_tests.contains(".persistent_holder_or_inline_from_nbt("));
        let dialog =
            include_str!("../../bbb-world/src/client/dialog/body/component_value/instrument.rs");
        assert!(dialog.contains("PersistentHolderOrInline::Direct"));
        let revision = include_str!("../../bbb-world/src/registries/revision.rs");
        assert!(revision.contains("decode_instrument_with_access(&data.value"));
        assert!(revision.contains("decode_jukebox_song_with_access(&data.value"));
        let enchantments = include_str!("../../bbb-world/src/registries/enchantments.rs");
        assert!(enchantments.contains("validate_required_sound_event("));
        assert!(enchantments.contains("Some(\"minecraft:play_sound\" | \"play_sound\")"));

        let native = include_str!("../../bbb-native/src/input/component_patch.rs");
        for required in [
            "fn encode_direct_instrument(",
            "instrument.description_nbt.as_deref()?",
            "fn instrument_encoder_uses_session_order_or_fixed_catalog()",
        ] {
            assert!(
                native.contains(required),
                "Instrument persistence path lost structural guard {required}"
            );
        }
    }

    #[test]
    fn complete_timeline_holder_families_keep_session_resolution_guards() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        for family in [
            "persistent-holder-reference-world-clock-synchronized",
            "persistent-holder-reference-timeline-synchronized",
            "persistent-holder-set-or-tag-timeline-synchronized",
        ] {
            let entries = ledger
                .entries
                .iter()
                .filter(|entry| entry.family == family)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), 1, "timeline coverage family {family}");
            assert_eq!(
                entries[0].status,
                RegistryHolderCoverageStatus::Complete,
                "timeline coverage family {family}"
            );
        }

        let timelines = include_str!("../../bbb-world/src/registries/timelines.rs");
        for required in [
            "pub fn synchronized_timeline_definition(",
            ".persistent_holder_from_nbt(WORLD_CLOCK_REGISTRY, clock)",
        ] {
            assert!(
                timelines.contains(required),
                "timeline resolver lost structural guard {required}"
            );
        }

        let dimension_types = include_str!("../../bbb-world/src/registries/dimension_types.rs");
        for required in [
            "fn decode_timeline_holder_set(",
            ".persistent_holder_set_from_nbt(TIMELINE_REGISTRY, value)",
            ".persistent_holder_from_nbt(super::WORLD_CLOCK_REGISTRY, clock)",
        ] {
            assert!(
                dimension_types.contains(required),
                "dimension timeline holder set lost structural guard {required}"
            );
        }

        let game_tests = include_str!("../../bbb-world/src/registries/game_tests.rs");
        for required in [
            "\"minecraft:timeline_attributes\" =>",
            "TIMELINE_REGISTRY,",
            "decode_named_reference(",
        ] {
            assert!(
                game_tests.contains(required),
                "test environment timeline reference lost structural guard {required}"
            );
        }
    }

    #[test]
    fn complete_painting_variant_families_keep_typed_persistence_and_render_guards() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        for family in [
            "network-holder-or-inline-painting-variant-synchronized",
            "persistent-holder-reference-painting-variant",
        ] {
            let entries = ledger
                .entries
                .iter()
                .filter(|entry| entry.family == family)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), 1, "painting coverage family {family}");
            assert_eq!(
                entries[0].status,
                RegistryHolderCoverageStatus::Complete,
                "painting coverage family {family}"
            );
        }

        let protocol = include_str!("packets/data_components.rs");
        assert!(protocol.contains("fn decode_painting_variant_holder_summary("));

        let world = include_str!("../../bbb-world/src/registries/painting_variants.rs");
        for required in [
            "pub fn painting_variant_presentation(",
            "fn decode_painting_variant(",
            "vanilla_painting_variant_for_resource_id(&entry.id)",
        ] {
            assert!(
                world.contains(required),
                "painting world path lost structural guard {required}"
            );
        }

        let persistent = include_str!("../../bbb-native/src/input/component_patch.rs");
        for required in [
            "fn encode_painting_variant(",
            "if patch.painting_variant_direct.is_some()",
            "context.registry_resource_id(\"minecraft:painting_variant\"",
        ] {
            assert!(
                persistent.contains(required),
                "painting persistence path lost structural guard {required}"
            );
        }

        let native_textures = include_str!("../../bbb-native/src/entity_variant_textures.rs");
        assert!(native_textures.contains("synchronize_texture_atlas(\"paintings\""));
        assert!(native_textures.contains("ready_painting_texture("));
        let native_scene = include_str!("../../bbb-native/src/entity_scene/painting.rs");
        for required in [
            "pub(super) fn painting_model_instances(",
            "painting_light_probe_block_pos(",
            "EntityModelInstance::painting_tile(",
        ] {
            assert!(
                native_scene.contains(required),
                "painting scene path lost structural guard {required}"
            );
        }

        let renderer = include_str!("../../bbb-renderer/src/entity_models/textured.rs");
        for required in [
            "pub(in crate::entity_models) fn render_painting_tile(",
            "append_painting_tile_geometry(",
            "EntityModelLayerRenderType::EntitySolidZOffsetForward",
        ] {
            assert!(
                renderer.contains(required),
                "painting renderer path lost structural guard {required}"
            );
        }
    }

    #[test]
    fn complete_chat_type_family_keeps_session_resolution_and_typed_payload_guards() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let chat_type = ledger
            .entries
            .iter()
            .find(|entry| entry.resource_id.as_deref() == Some("minecraft:chat_type"))
            .unwrap();
        assert_eq!(chat_type.status, RegistryHolderCoverageStatus::Complete);
        assert_eq!(
            chat_type.family,
            "network-holder-or-inline-chat-type-synchronized"
        );

        let world_chat = include_str!("../../bbb-world/src/client/chat.rs");
        assert_eq!(
            world_chat
                .matches("resolve_chat_type(self.frozen_registry_access()")
                .count(),
            2
        );
        assert!(!world_chat.contains("fn chat_type_state("));

        let chat_types = include_str!("../../bbb-world/src/registries/chat_types.rs");
        for required in [
            "require_entry_by_holder_id(CHAT_TYPE_REGISTRY",
            "RegistryEntrySource::Server",
            "RegistryEntrySource::VanillaFallback",
            "entry.element_data",
            "fn decode_chat_type(",
        ] {
            assert!(
                chat_types.contains(required),
                "chat type resolver lost structural guard {required}"
            );
        }
    }

    #[test]
    fn complete_jukebox_song_family_keeps_session_resolution_and_consumer_guards() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        let jukebox_song = ledger
            .entries
            .iter()
            .find(|entry| {
                entry.resource_id.as_deref() == Some("minecraft:jukebox_song")
                    && entry.family == "network-holder-or-inline-jukebox-song-synchronized"
            })
            .unwrap();
        assert_eq!(jukebox_song.status, RegistryHolderCoverageStatus::Complete);
        assert_eq!(
            jukebox_song.family,
            "network-holder-or-inline-jukebox-song-synchronized"
        );

        let protocol = include_str!("packets/data_components.rs");
        for required in [
            "fn decode_jukebox_song_holder(",
            "pub description_nbt: Option<Box<NbtSummaryValue>>",
            "decode_styled_component_summary_with_nbt_from_decoder(decoder)",
        ] {
            assert!(
                protocol.contains(required),
                "jukebox protocol path lost structural guard {required}"
            );
        }

        let world = include_str!("../../bbb-world/src/registries/jukebox_songs.rs");
        for required in [
            "registry_entry_by_holder_id(JUKEBOX_SONG_REGISTRY",
            "fn decode_jukebox_song_with_access(",
            "pub fn synchronized_jukebox_song_presentation(",
        ] {
            assert!(
                world.contains(required),
                "jukebox world path lost structural guard {required}"
            );
        }

        let dispatcher = include_str!("../../bbb-native/src/runtime/events/dispatcher.rs");
        for required in [
            "fn jukebox_song_registry_from_world(",
            ".synchronized_jukebox_song_presentation(holder_id)",
            ".filter(|entry| !entry.has_data)",
            "JukeboxSongRegistry::vanilla_26_1_sound_event_id(&song_id)",
        ] {
            assert!(
                dispatcher.contains(required),
                "jukebox audio path lost structural guard {required}"
            );
        }

        let item_models = include_str!("../../bbb-native/src/item_models.rs");
        for required in [
            "fn world_jukebox_song_tooltip_context(",
            "world.synchronized_jukebox_song_presentation(holder_id)",
            "is_some_and(|entry| !entry.has_data)",
        ] {
            assert!(
                item_models.contains(required),
                "jukebox tooltip path lost structural guard {required}"
            );
        }
    }

    #[test]
    fn complete_trim_families_keep_typed_session_and_persistent_guards() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        for (family, expected_count) in [
            ("network-holder-or-inline-trims-synchronized", 2),
            ("persistent-holder-or-inline-trims", 2),
        ] {
            let entries = ledger
                .entries
                .iter()
                .filter(|entry| entry.family == family)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), expected_count);
            assert!(entries
                .iter()
                .all(|entry| entry.status == RegistryHolderCoverageStatus::Complete));
        }

        let protocol = include_str!("packets/data_components.rs");
        for required in [
            "fn decode_direct_trim_material_summary(",
            "fn decode_direct_trim_pattern_summary(",
            "description_nbt: description_nbt.map(Box::new)",
        ] {
            assert!(
                protocol.contains(required),
                "trim protocol path lost structural guard {required}"
            );
        }

        let world = include_str!("../../bbb-world/src/registries/armor_trims.rs");
        for required in [
            "pub fn synchronized_trim_material_presentation(",
            "pub fn synchronized_trim_pattern_presentation(",
            "fn synchronized_presentation<T>(",
            "VanillaTrimMaterialIdentity::for_resource_id(&entry.id)",
            "VanillaTrimPatternIdentity::for_resource_id(&entry.id)",
        ] {
            assert!(
                world.contains(required),
                "trim world path lost structural guard {required}"
            );
        }

        let tooltip = include_str!("../../bbb-item-model/src/item_runtime/tooltip.rs");
        for required in [
            "direct_trim_material_presentation(material)?",
            "direct_trim_pattern_presentation(pattern)?",
            "trim_material_presentations",
            "trim_pattern_presentations",
        ] {
            assert!(
                tooltip.contains(required),
                "trim tooltip path lost structural guard {required}"
            );
        }

        let persistent = include_str!("../../bbb-native/src/input/component_patch.rs");
        for required in [
            "fn encode_direct_trim_material(",
            "fn encode_direct_trim_pattern(",
            "material.description_nbt.as_ref()?",
            "pattern.description_nbt.as_ref()?",
        ] {
            assert!(
                persistent.contains(required),
                "trim persistent path lost structural guard {required}"
            );
        }
    }

    #[test]
    fn complete_banner_pattern_families_keep_holder_and_render_guards() {
        let ledger = vanilla_registry_holder_coverage_26_1().unwrap();
        for family in [
            "network-holder-or-inline-banner-pattern-synchronized",
            "network-holder-set-or-tag-banner-pattern",
            "persistent-holder-or-inline-banner-pattern",
            "persistent-holder-set-or-tag-banner-pattern",
        ] {
            let entries = ledger
                .entries
                .iter()
                .filter(|entry| entry.family == family)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), 1, "banner coverage family {family}");
            assert_eq!(
                entries[0].status,
                RegistryHolderCoverageStatus::Complete,
                "banner coverage family {family}"
            );
        }

        let protocol = include_str!("packets/data_components.rs");
        assert!(protocol.contains("fn decode_banner_pattern_holder("));
        let protocol_catalog = include_str!("packets/data_components/catalog.rs");
        assert!(protocol_catalog.contains("fn decode_provides_banner_patterns("));

        let world = include_str!("../../bbb-world/src/registries/banner_patterns.rs");
        for required in [
            "pub fn synchronized_banner_pattern_presentation(",
            "pub fn banner_pattern_presentation(",
            "pub fn direct_banner_pattern_presentation(",
            "BannerPatternKind::for_resource_id(&entry.id)",
        ] {
            assert!(
                world.contains(required),
                "banner world path lost structural guard {required}"
            );
        }

        let persistent = include_str!("../../bbb-native/src/input/component_patch.rs");
        for required in [
            "fn encode_provides_banner_patterns(",
            "fn encode_banner_patterns(",
            "direct_banner_pattern_presentation(",
        ] {
            assert!(
                persistent.contains(required),
                "banner persistence path lost structural guard {required}"
            );
        }

        let native_textures = include_str!("../../bbb-native/src/entity_variant_textures.rs");
        assert!(native_textures.contains("load_atlas_texture_images(atlas_name)"));
        assert!(native_textures.contains("synchronize_texture_atlas(\"banner_patterns\""));
        let native_scene = include_str!("../../bbb-native/src/banner_scene.rs");
        assert!(native_scene.contains("BannerPatternTexture::Runtime"));
        assert!(native_scene.contains("ready_banner_pattern_texture(sprite_id)"));

        let renderer = include_str!("../../bbb-renderer/src/entity_models/dispatch.rs");
        assert!(renderer.contains("BannerPatternTexture::Runtime(texture)"));
        assert!(renderer.contains("render_textured_layers_with_dynamic_texture("));
    }
}
