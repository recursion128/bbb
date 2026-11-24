//! `Advancements` surface (`ServerAdvancementManager` / `Advancement.CODEC`): the
//! `advancement/` tree decoded through the 26.1 criterion-trigger dispatch,
//! winner-take-all, warn-only per advancement
//! (`SimpleJsonResourceReloadListener.scanDirectory`), then tree-pruned the
//! `AdvancementTree.addAll` way — advancements whose parent is not in the loaded
//! set are dropped round by round with an error log.

use std::collections::HashSet;
use std::fmt;

use bbb_protocol::{content_registry::ContentRegistry, normalize_resource_location, JavaString};
use bbb_registry_codec::RegistryJsonValue;

pub mod block_contact;
mod criteria;
mod inventory;

pub use inventory::{
    parse_inventory_changed_condition, InventoryChangedCondition, InventoryChangedSlots,
};

use super::registry_data_loader::DatapackLoadLimits;
use super::resource_overlay::{MergedServerDataView, PackOverlayInput};

/// 26.1 `CriteriaTriggers` bootstrap set (`CriteriaTriggers.java`).
pub const ADVANCEMENT_TRIGGER_TYPES_26_1: &[&str] = &[
    "minecraft:allay_drop_item_on_block",
    "minecraft:any_block_use",
    "minecraft:avoid_vibration",
    "minecraft:bee_nest_destroyed",
    "minecraft:bred_animals",
    "minecraft:brewed_potion",
    "minecraft:changed_dimension",
    "minecraft:channeled_lightning",
    "minecraft:construct_beacon",
    "minecraft:consume_item",
    "minecraft:crafter_recipe_crafted",
    "minecraft:cured_zombie_villager",
    "minecraft:default_block_use",
    "minecraft:effects_changed",
    "minecraft:enchanted_item",
    "minecraft:enter_block",
    "minecraft:entity_hurt_player",
    "minecraft:entity_killed_player",
    "minecraft:fall_after_explosion",
    "minecraft:fall_from_height",
    "minecraft:filled_bucket",
    "minecraft:fishing_rod_hooked",
    "minecraft:hero_of_the_village",
    "minecraft:impossible",
    "minecraft:inventory_changed",
    "minecraft:item_durability_changed",
    "minecraft:item_used_on_block",
    "minecraft:kill_mob_near_sculk_catalyst",
    "minecraft:killed_by_arrow",
    "minecraft:levitation",
    "minecraft:lightning_strike",
    "minecraft:location",
    "minecraft:nether_travel",
    "minecraft:placed_block",
    "minecraft:player_generates_container_loot",
    "minecraft:player_hurt_entity",
    "minecraft:player_interacted_with_entity",
    "minecraft:player_killed_entity",
    "minecraft:player_sheared_equipment",
    "minecraft:recipe_crafted",
    "minecraft:recipe_unlocked",
    "minecraft:ride_entity_in_lava",
    "minecraft:shot_crossbow",
    "minecraft:slept_in_bed",
    "minecraft:slide_down_block",
    "minecraft:spear_mobs",
    "minecraft:started_riding",
    "minecraft:summoned_entity",
    "minecraft:tame_animal",
    "minecraft:target_hit",
    "minecraft:thrown_item_picked_up_by_entity",
    "minecraft:thrown_item_picked_up_by_player",
    "minecraft:tick",
    "minecraft:used_ender_eye",
    "minecraft:used_totem",
    "minecraft:using_item",
    "minecraft:villager_trade",
    "minecraft:voluntary_exile",
];

/// An advancement decode failure, attributed with the advancement id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementDecodeError {
    pub message: String,
}

impl fmt::Display for AdvancementDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AdvancementDecodeError {}

/// One named criterion: the whitelisted 26.1 trigger id and the raw conditions
/// object retained for the gameplay slice.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancementCriterion {
    pub name: JavaString,
    pub trigger: String,
    pub conditions: RegistryJsonValue,
    pub block_contact: Option<block_contact::BlockContactCondition>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct AdvancementRewards {
    pub experience: i32,
    pub loot: Vec<String>,
    pub recipes: Vec<String>,
    pub function: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeUnlockedCondition {
    pub recipe: String,
    pub player: Option<RegistryJsonValue>,
}

fn decode_error(message: impl Into<String>) -> AdvancementDecodeError {
    AdvancementDecodeError {
        message: message.into(),
    }
}

fn parse_resource_id(value: &RegistryJsonValue) -> Result<String, AdvancementDecodeError> {
    let raw = value
        .utf8_string()
        .ok_or_else(|| decode_error("resource id must be a string"))?;
    normalize_resource_location(raw).map_err(|error| decode_error(error.to_string()))
}

// `AdvancementRewards.java:27-36`.
fn parse_rewards(value: &RegistryJsonValue) -> Result<AdvancementRewards, AdvancementDecodeError> {
    value
        .as_object()
        .ok_or_else(|| decode_error("advancement `rewards` must be an object"))?;
    let experience = match value.get("experience") {
        None => 0,
        Some(value) => value
            .as_i64()
            .and_then(|number| i32::try_from(number).ok())
            .ok_or_else(|| decode_error("reward `experience` must be an i32"))?,
    };
    let ids = |name: &str| -> Result<Vec<String>, AdvancementDecodeError> {
        let Some(value) = value.get(name) else {
            return Ok(Vec::new());
        };
        value
            .as_array()
            .ok_or_else(|| decode_error(format!("reward `{name}` must be an array")))?
            .iter()
            .map(parse_resource_id)
            .collect()
    };
    Ok(AdvancementRewards {
        experience,
        loot: ids("loot")?,
        recipes: ids("recipes")?,
        function: value.get("function").map(parse_resource_id).transpose()?,
    })
}

/// `RecipeUnlockedTrigger.java:27-38`; player predicates remain with their owner.
pub fn parse_recipe_unlocked_condition(
    value: &RegistryJsonValue,
) -> Result<RecipeUnlockedCondition, AdvancementDecodeError> {
    value
        .as_object()
        .ok_or_else(|| decode_error("recipe_unlocked conditions must be an object"))?;
    let recipe = parse_resource_id(
        value
            .get("recipe")
            .ok_or_else(|| decode_error("recipe_unlocked requires `recipe`"))?,
    )?;
    let player = match value.get("player") {
        None => None,
        Some(value) if value.is_object() || value.as_array().is_some() => Some(value.clone()),
        Some(_) => {
            return Err(decode_error(
                "recipe_unlocked `player` must be an object or array",
            ))
        }
    };
    Ok(RecipeUnlockedCondition { recipe, player })
}

/// One decoded advancement with only gameplay-consumed source fields retained.
#[derive(Debug, Clone, PartialEq)]
pub struct Advancement {
    pub parent: Option<String>,
    pub rewards: AdvancementRewards,
    /// Criteria sorted by name.
    pub criteria: Vec<AdvancementCriterion>,
    /// Raw requirements when present; vanilla defaults to allOf(criteria).
    pub requirements: Option<RegistryJsonValue>,
    pub display: Option<RegistryJsonValue>,
    pub sends_telemetry_event: bool,
}

/// `Advancement.CODEC` envelope: an object with a non-empty `criteria` map of
/// named `{trigger, conditions?}` entries against the 26.1 trigger whitelist and
/// an optional `parent` id.
pub fn parse_advancement(value: &RegistryJsonValue) -> Result<Advancement, AdvancementDecodeError> {
    parse_advancement_with_content_registry(value, ContentRegistry::vanilla())
}

pub fn parse_advancement_with_content_registry(
    value: &RegistryJsonValue,
    content_registry: &ContentRegistry,
) -> Result<Advancement, AdvancementDecodeError> {
    parse_advancement_inner(value, None, content_registry)
}

pub fn parse_advancement_with_lookup(
    value: &RegistryJsonValue,
    lookup: Option<&crate::frozen_lookup::FrozenDatapackRegistryLookup<'_>>,
) -> Result<Advancement, AdvancementDecodeError> {
    let content_registry = lookup.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        crate::frozen_lookup::FrozenDatapackRegistryLookup::content_registry,
    );
    parse_advancement_inner(value, lookup, content_registry)
}

fn parse_advancement_inner(
    value: &RegistryJsonValue,
    lookup: Option<&crate::frozen_lookup::FrozenDatapackRegistryLookup<'_>>,
    content_registry: &ContentRegistry,
) -> Result<Advancement, AdvancementDecodeError> {
    value.as_object().ok_or_else(|| AdvancementDecodeError {
        message: "advancement must be an object".to_string(),
    })?;
    let parent = match value.get("parent") {
        Some(raw) => Some(parse_resource_id(raw)?),
        None => None,
    };
    let criteria_map = value
        .get("criteria")
        .and_then(RegistryJsonValue::as_object)
        .ok_or_else(|| AdvancementDecodeError {
            message: "advancement lacks a `criteria` object".to_string(),
        })?;
    if criteria_map.is_empty() {
        return Err(AdvancementDecodeError {
            message: "advancement requires at least one criterion".to_string(),
        });
    }
    let mut criteria = Vec::with_capacity(criteria_map.len());
    for (name, criterion) in criteria_map {
        criterion
            .as_object()
            .ok_or_else(|| AdvancementDecodeError {
                message: format!("criterion {name:?} must be an object"),
            })?;
        let raw_trigger = criterion
            .get("trigger")
            .and_then(RegistryJsonValue::utf8_string)
            .ok_or_else(|| AdvancementDecodeError {
                message: format!("criterion {name:?} lacks a string `trigger` field"),
            })?;
        let trigger = normalize_resource_location(raw_trigger)
            .map_err(|error| decode_error(error.to_string()))?;
        if !ADVANCEMENT_TRIGGER_TYPES_26_1.contains(&trigger.as_str()) {
            return Err(AdvancementDecodeError {
                message: format!("unknown criterion trigger {trigger}"),
            });
        }
        // Criterion.java:8-15; ExtraCodecs.java:592-598: missing `conditions`
        // dispatches an empty map to `CriterionTrigger.codec()`.
        let conditions = criterion
            .get("conditions")
            .filter(|value| {
                !matches!(
                    trigger.as_str(),
                    "minecraft:slide_down_block" | "minecraft:enter_block"
                ) || !value.is_null()
            })
            .cloned()
            .unwrap_or_else(|| RegistryJsonValue::Object(Vec::new()));
        let block_contact = matches!(
            trigger.as_str(),
            "minecraft:slide_down_block" | "minecraft:enter_block"
        )
        .then(|| {
            block_contact::parse_block_contact_condition_with_content_registry(
                &conditions,
                content_registry,
            )
        })
        .transpose()?;
        criteria::validate_criterion_conditions(
            &trigger,
            &conditions,
            block_contact.as_ref(),
            lookup,
            content_registry,
        )?;
        criteria.push(AdvancementCriterion {
            name: name.clone(),
            trigger,
            conditions,
            block_contact,
        });
    }
    criteria.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Advancement {
        parent,
        rewards: value
            .get("rewards")
            .map(parse_rewards)
            .transpose()?
            .unwrap_or_default(),
        criteria,
        requirements: value.get("requirements").cloned(),
        display: value.get("display").cloned(),
        sends_telemetry_event: value
            .get("sends_telemetry_event")
            .map(|value| {
                value
                    .as_bool()
                    .ok_or_else(|| decode_error("sends_telemetry_event must be a boolean"))
            })
            .transpose()?
            .unwrap_or(false),
    })
}

/// One loaded advancement, attributed to the pack that won the overlay merge.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancementElement {
    pub id: String,
    pub source_pack: String,
    pub advancement: Advancement,
}

/// One warn-only advancement loading event (`scanDirectory` / `AdvancementTree`
/// logs and skips the element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementLoadWarning {
    pub element_id: String,
    pub pack: String,
    pub message: String,
}

/// The advancement tree for one datapack view (`AdvancementTree` equivalent:
/// decoded, orphan-pruned, in sorted id order).
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancementSet {
    /// Advancements sorted by id (`TreeMap` iteration order of the merged listing).
    pub advancements: Vec<AdvancementElement>,
    pub warnings: Vec<AdvancementLoadWarning>,
}

impl AdvancementSet {
    pub fn advancement(&self, id: &str) -> Option<&AdvancementElement> {
        self.advancements
            .iter()
            .find(|advancement| advancement.id == id)
    }
}

/// `ServerAdvancementManager.prepare` + `AdvancementTree.addAll`: winner-take-all
/// scan of the `advancement/` tree with warn-only per-file decode, then round-based
/// orphan pruning — an advancement whose parent is absent from the loaded set is
/// dropped with an error log once a full round makes no progress.
pub fn load_advancements(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
    lookup: Option<&crate::frozen_lookup::FrozenDatapackRegistryLookup<'_>>,
) -> AdvancementSet {
    let mut decoded = Vec::new();
    let mut warnings = Vec::new();
    for namespace in view.namespaces() {
        for path in view.list(namespace, "advancement") {
            let Some(element_path) = path
                .strip_prefix("advancement/")
                .and_then(|path| path.strip_suffix(".json"))
            else {
                continue;
            };
            let element_id = format!("{namespace}:{element_path}");
            let Some((pack_id, resources)) = view.winner(packs, namespace, path) else {
                continue;
            };
            let Some(bytes) = resources.read(namespace, path) else {
                continue;
            };
            if bytes.len() > limits.max_element_bytes {
                warnings.push(AdvancementLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!(
                        "advancement file exceeds the budget ({} bytes)",
                        bytes.len()
                    ),
                });
                continue;
            }
            let parsed =
                bbb_registry_codec::decode_loaded_registry_json(&bytes, limits.max_element_bytes)
                    .map_err(|error| decode_error(format!("invalid JSON: {error}")))
                    .and_then(|data| {
                        parse_advancement_with_lookup(
                            data.json().expect("datapack JSON source"),
                            lookup,
                        )
                    });
            match parsed {
                Ok(advancement) => decoded.push(AdvancementElement {
                    id: element_id,
                    source_pack: pack_id.to_string(),
                    advancement,
                }),
                Err(decode) => warnings.push(AdvancementLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!("couldn't parse data file: {decode}"),
                }),
            }
        }
    }
    // `AdvancementTree.addAll`: removeIf(tryInsert) until a round inserts nothing,
    // then log and drop the remaining orphans.
    let mut known: HashSet<String> = HashSet::new();
    let mut accepted: Vec<AdvancementElement> = Vec::new();
    let mut remaining = decoded;
    loop {
        let mut progress = false;
        let mut still_orphan = Vec::new();
        for element in remaining {
            let parent_missing = element
                .advancement
                .parent
                .as_ref()
                .is_some_and(|parent| !known.contains(parent));
            if parent_missing {
                still_orphan.push(element);
            } else {
                known.insert(element.id.clone());
                accepted.push(element);
                progress = true;
            }
        }
        if still_orphan.is_empty() {
            break;
        }
        if !progress {
            for element in still_orphan {
                warnings.push(AdvancementLoadWarning {
                    element_id: element.id.clone(),
                    pack: element.source_pack.clone(),
                    message: format!(
                        "orphan advancement dropped: parent {:?} is not in the loaded set",
                        element.advancement.parent
                    ),
                });
            }
            break;
        }
        remaining = still_orphan;
    }
    accepted.sort_by(|a, b| a.id.cmp(&b.id));
    AdvancementSet {
        advancements: accepted,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! json {
        (null) => { RegistryJsonValue::Null };
        ($($token:tt)*) => {
            crate::loot::codec::registry_json(&serde_json::json!($($token)*)).unwrap()
        };
    }

    fn set_rewards(body: &mut RegistryJsonValue, rewards: RegistryJsonValue) {
        let RegistryJsonValue::Object(fields) = body else {
            unreachable!()
        };
        fields.retain(|(name, _)| name != "rewards");
        fields.push(("rewards".into(), rewards));
    }

    #[test]
    fn raw_pack_loader_preserves_criterion_identity_and_ignored_envelope_budget() {
        use crate::pack_repository::ServerPackRepository;
        use crate::vanilla_snapshot::VanillaDatapackSnapshot;
        use std::sync::Arc;

        let temp = std::env::temp_dir().join(format!(
            "bbb-advancement-source-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pack = temp.join("raw");
        std::fs::create_dir_all(pack.join("data/test/advancement")).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            br#"{"pack":{"description":"raw","min_format":101,"max_format":101}}"#,
        )
        .unwrap();
        let ignored = format!("{}0{}", "[".repeat(80), "]".repeat(80));
        let wide = vec!["0"; 4097].join(",");
        let raw = format!(
            r#"{{"criteria":{{"\ud800":{{"trigger":"tick","conditions":{{"player":{{"team":"\ud800"}}}}}},"\ud801":{{"trigger":"tick"}},"\ufffd":{{"trigger":"tick"}},"\ud83d\ude00":{{"trigger":"tick"}}}},"requirements":[["\ud800"],["\ud801"],["\ufffd"],["\ud83d\ude00"]],"ignored":{ignored},"wide":[{wide}],"sends_telemetry_event":true}}"#
        );
        std::fs::write(pack.join("data/test/advancement/raw.json"), raw).unwrap();
        let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let repository =
            ServerPackRepository::new(&temp, Box::new(snapshot.vanilla_pack()), Vec::new())
                .unwrap();
        let selected = repository.select_ids(&["file/raw".into()]);
        let opened = repository.open_selected(&selected);
        let inputs: Vec<_> = opened
            .iter()
            .map(|pack| PackOverlayInput {
                pack_id: &pack.id,
                resources: pack.resources.as_ref(),
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let loaded = load_advancements(
            &MergedServerDataView::merge(&inputs),
            &inputs,
            DatapackLoadLimits::default(),
            None,
        );
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        let advancement = &loaded.advancement("test:raw").unwrap().advancement;
        let names: Vec<_> = advancement
            .criteria
            .iter()
            .map(|entry| entry.name.units())
            .collect();
        assert_eq!(
            names,
            [
                vec![0xd800],
                vec![0xd801],
                vec![0xd83d, 0xde00],
                vec![0xfffd]
            ]
        );
        let first_condition = &advancement.criteria[0].conditions;
        assert_eq!(
            first_condition.get("player").unwrap().get("team"),
            Some(&RegistryJsonValue::String(JavaString::from(vec![0xd800])))
        );
        assert_eq!(
            advancement
                .requirements
                .as_ref()
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert!(advancement.sends_telemetry_event);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rewards_default_and_preserve_signed_experience_and_ordered_duplicates() {
        let mut body = json!({"criteria": {"x": {"trigger": "impossible"}}});
        assert_eq!(
            parse_advancement(&body).unwrap().rewards,
            AdvancementRewards::default()
        );
        set_rewards(&mut body, json!({}));
        assert_eq!(
            parse_advancement(&body).unwrap().rewards,
            AdvancementRewards::default()
        );
        set_rewards(
            &mut body,
            json!({
                "experience": -7,
                "loot": ["a", "custom:b", "a"],
                "recipes": ["custom:c", "d", "custom:c"],
                "function": "custom:reward"
            }),
        );
        let advancement = parse_advancement(&body).unwrap();
        assert_eq!(
            advancement.rewards,
            AdvancementRewards {
                experience: -7,
                loot: vec![
                    "minecraft:a".into(),
                    "custom:b".into(),
                    "minecraft:a".into()
                ],
                recipes: vec!["custom:c".into(), "minecraft:d".into(), "custom:c".into()],
                function: Some("custom:reward".into()),
            }
        );
        assert_eq!(advancement.display, None);
    }

    #[test]
    fn recipe_unlocked_requires_recipe_and_preserves_optional_player_shape() {
        let condition = parse_recipe_unlocked_condition(&json!({"recipe": "bread"})).unwrap();
        assert_eq!(condition.recipe, "minecraft:bread");
        assert_eq!(condition.player, None);
        for player in [
            json!({"flags": {"is_sneaking": true}}),
            json!([]),
            json!([{"condition": "custom:predicate"}]),
        ] {
            let condition =
                parse_recipe_unlocked_condition(&json!({"recipe": "custom:a", "player": player}))
                    .unwrap();
            assert_eq!(condition.player, Some(player));
        }
        for conditions in [
            json!(null),
            json!({}),
            json!({"recipe": 1}),
            json!({"recipe": "A"}),
            json!({"recipe": "a", "player": true}),
            json!({"recipe": "a", "player": null}),
        ] {
            assert!(
                parse_recipe_unlocked_condition(&conditions).is_err(),
                "{conditions:?}"
            );
            let body = json!({"criteria": {"x": {"trigger": "recipe_unlocked", "conditions": conditions}}});
            assert!(parse_advancement(&body).is_err(), "{body:?}");
        }
        assert!(
            parse_advancement(&json!({"criteria": {"x": {"trigger": "recipe_unlocked"}}})).is_err()
        );
        assert!(parse_advancement(&json!({"criteria": {"x": {"trigger": "recipe_unlocked", "conditions": {"recipe": "bread"}}}})).is_ok());
    }

    #[test]
    fn whitelist_matches_vanilla_26_1_count() {
        assert_eq!(ADVANCEMENT_TRIGGER_TYPES_26_1.len(), 58);
    }

    #[test]
    fn parses_criteria_and_defaults_trigger_namespace() {
        let advancement = parse_advancement(&json!({
            "criteria": {
                "tick": {"trigger": "tick"},
                "killed": {"trigger": "minecraft:player_killed_entity", "conditions": {}}
            }
        }))
        .unwrap();
        assert_eq!(advancement.criteria.len(), 2);
        assert_eq!(advancement.criteria[0].name, "killed");
        assert_eq!(
            advancement.criteria[0].trigger,
            "minecraft:player_killed_entity"
        );
        assert_eq!(advancement.criteria[1].trigger, "minecraft:tick");
    }

    #[test]
    fn parses_parent_id() {
        let advancement = parse_advancement(&json!({
            "parent": "adventure/root",
            "criteria": {"x": {"trigger": "impossible"}}
        }))
        .unwrap();
        assert_eq!(
            advancement.parent.as_deref(),
            Some("minecraft:adventure/root")
        );
    }

    #[test]
    fn malformed_criterion_conditions_reject_advancement_and_prune_descendants() {
        use crate::pack_repository::ServerPackRepository;
        use crate::vanilla_snapshot::VanillaDatapackSnapshot;
        use std::sync::Arc;

        for bad in [
            json!({"criteria": {"x": {"trigger": "impossible", "conditions": []}}}),
            json!({"criteria": {"x": {"trigger": "spear_mobs", "conditions": {"count": "bad"}}}}),
            json!({"criteria": {"x": {"trigger": "construct_beacon", "conditions": {"level": "bad"}}}}),
            json!({"criteria": {"x": {"trigger": "placed_block", "conditions": {"location": {}}}}}),
            json!({"criteria": {"x": {"trigger": "tick", "conditions": {"player": [{"condition": "minecraft:unknown"}]}}}}),
            json!({"criteria": {"x": {"trigger": "player_generates_container_loot", "conditions": {}}}}),
            json!({"criteria": {"x": {"trigger": "recipe_crafted", "conditions": {}}}}),
            json!({"criteria": {"x": {"trigger": "entity_hurt_player", "conditions": {"damage": 7}}}}),
        ] {
            assert!(parse_advancement(&bad).is_err(), "{bad:?}");
        }

        let temp = std::env::temp_dir().join(format!(
            "bbb-advancement-criteria-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pack = temp.join("bad_criteria");
        std::fs::create_dir_all(pack.join("data/test/advancement")).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            br#"{"pack":{"description":"bad","min_format":101,"max_format":101}}"#,
        )
        .unwrap();
        std::fs::write(
            pack.join("data/test/advancement/valid_root.json"),
            br#"{"criteria":{"ok":{"trigger":"minecraft:tick"}}}"#,
        )
        .unwrap();
        std::fs::write(
            pack.join("data/test/advancement/broken_parent.json"),
            br#"{"criteria":{"ok":{"trigger":"minecraft:tick"},"bad":{"trigger":"minecraft:spear_mobs","conditions":{"count":0}}}}"#,
        )
        .unwrap();
        std::fs::write(
            pack.join("data/test/advancement/orphan_child.json"),
            br#"{"parent":"test:broken_parent","criteria":{"ok":{"trigger":"minecraft:tick"}}}"#,
        )
        .unwrap();

        let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let repository =
            ServerPackRepository::new(&temp, Box::new(snapshot.vanilla_pack()), Vec::new())
                .unwrap();
        let selected = repository.select_ids(&["file/bad_criteria".into()]);
        let opened = repository.open_selected(&selected);
        let inputs: Vec<_> = opened
            .iter()
            .map(|pack| PackOverlayInput {
                pack_id: &pack.id,
                resources: pack.resources.as_ref(),
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let loaded = load_advancements(
            &MergedServerDataView::merge(&inputs),
            &inputs,
            DatapackLoadLimits::default(),
            None,
        );
        std::fs::remove_dir_all(&temp).unwrap();

        assert!(loaded.advancement("test:valid_root").is_some());
        assert!(loaded.advancement("test:broken_parent").is_none());
        assert!(loaded.advancement("test:orphan_child").is_none());
        assert_eq!(loaded.warnings.len(), 2, "{:?}", loaded.warnings);
        assert!(loaded
            .warnings
            .iter()
            .any(|warning| warning.element_id == "test:broken_parent"));
        assert!(loaded
            .warnings
            .iter()
            .any(|warning| warning.element_id == "test:orphan_child"));
    }
}
