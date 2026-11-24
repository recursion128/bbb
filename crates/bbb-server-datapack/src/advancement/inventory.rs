use bbb_protocol::content_registry::ContentRegistry;
use bbb_registry_codec::RegistryJsonValue;

use super::{decode_error, AdvancementDecodeError};
use crate::loot::codec::V;
use crate::loot::{parse_item_predicate_source_with_content_registry, IntBounds, ItemPredicate};

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct InventoryChangedSlots {
    pub occupied: IntBounds,
    pub full: IntBounds,
    pub empty: IntBounds,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct InventoryChangedCondition {
    pub player: Option<RegistryJsonValue>,
    pub slots: InventoryChangedSlots,
    pub items: Vec<ItemPredicate>,
}

fn item_predicate(
    value: &RegistryJsonValue,
    content_registry: &ContentRegistry,
) -> Result<ItemPredicate, AdvancementDecodeError> {
    parse_item_predicate_source_with_content_registry(V::Json(value), content_registry)
        .map_err(|error| decode_error(error.to_string()))
}

/// InventoryChangeTrigger.java:49-60; InventoryChangeTrigger.java:113-124.
pub fn parse_inventory_changed_condition(
    value: &RegistryJsonValue,
) -> Result<InventoryChangedCondition, AdvancementDecodeError> {
    parse_inventory_changed_condition_with_content_registry(value, ContentRegistry::vanilla())
}

/// Content-registry-aware form of [`parse_inventory_changed_condition`].
pub fn parse_inventory_changed_condition_with_content_registry(
    value: &RegistryJsonValue,
    content_registry: &ContentRegistry,
) -> Result<InventoryChangedCondition, AdvancementDecodeError> {
    value
        .as_object()
        .ok_or_else(|| decode_error("inventory_changed conditions must be an object"))?;
    let player = match value.get("player") {
        None => None,
        Some(value) if value.is_object() || value.as_array().is_some() => Some(value.clone()),
        Some(_) => {
            return Err(decode_error(
                "inventory_changed `player` must be an object or array",
            ))
        }
    };
    let slots = match V::Json(value).field("slots") {
        None => InventoryChangedSlots::default(),
        Some(value) => {
            if !value.is_compound() {
                return Err(decode_error("inventory_changed `slots` must be an object"));
            }
            let read = |name: &str| {
                value
                    .field(name)
                    .map(IntBounds::decode)
                    .transpose()
                    .map(Option::unwrap_or_default)
                    .map_err(|error| decode_error(error.to_string()))
            };
            InventoryChangedSlots {
                occupied: read("occupied")?,
                full: read("full")?,
                empty: read("empty")?,
            }
        }
    };
    let items = match value.get("items") {
        None => Vec::new(),
        Some(value) => value
            .as_array()
            .ok_or_else(|| decode_error("inventory_changed `items` must be an array"))?
            .iter()
            .map(|value| item_predicate(value, content_registry))
            .collect::<Result<_, _>>()?,
    };
    Ok(InventoryChangedCondition {
        player,
        slots,
        items,
    })
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

    #[test]
    fn missing_conditions_and_empty_objects_default_to_any_inventory() {
        assert_eq!(
            parse_inventory_changed_condition(&json!({})).unwrap(),
            InventoryChangedCondition::default()
        );
        let advancement = super::super::parse_advancement(&json!({
            "criteria": {"inventory": {"trigger": "inventory_changed"}}
        }))
        .unwrap();
        assert_eq!(
            parse_inventory_changed_condition(&advancement.criteria[0].conditions).unwrap(),
            InventoryChangedCondition::default()
        );
        assert!(IntBounds::default().matches(i32::MIN));
        assert!(IntBounds::default().matches(i32::MAX));
    }

    #[test]
    fn slot_bounds_are_inclusive_and_support_exact_and_open_ranges() {
        let data = bbb_registry_codec::decode_loaded_registry_json(
            br#"{"slots":{"occupied":{"min":-2,"max":3},"full":4294967297,"empty":{"max":9}}}"#,
            usize::MAX,
        )
        .unwrap();
        let condition = parse_inventory_changed_condition(data.json().unwrap()).unwrap();
        for (value, matches) in [(-3, false), (-2, true), (3, true), (4, false)] {
            assert_eq!(condition.slots.occupied.matches(value), matches);
        }
        assert!(condition.slots.full.matches(1));
        assert!(!condition.slots.full.matches(0));
        assert!(!condition.slots.full.matches(2));
        assert!(condition.slots.empty.matches(i32::MIN));
        assert!(!condition.slots.empty.matches(10));
        assert!(IntBounds::decode(V::Json(&json!({"min": 5})))
            .unwrap()
            .matches(i32::MAX));
    }

    #[test]
    fn items_decode_into_item_predicates() {
        use crate::loot::{ComponentPredicate, HolderSet, IntBounds};
        let components = json!({"minecraft:custom_data": {"marker": 1}});
        let predicates = json!({"minecraft:damage": {"damage": {"min": 2}}});
        let condition = parse_inventory_changed_condition(&json!({
            "player": [{"condition": "minecraft:random_chance", "chance": 0.5}],
            "items": [
                {"items": "#logs", "count": {"min": 2}, "components": components, "predicates": predicates},
                {"items": ["stone", "custom:ore", "stone"]},
                {"items": "apple", "ignored": true}
            ],
            "ignored": true
        })).unwrap();
        assert_eq!(
            condition.items[0].items,
            Some(HolderSet::Tag("minecraft:logs".into()))
        );
        assert_eq!(condition.items[0].count.min, Some(2));
        assert_eq!(condition.items[0].components.len(), 1);
        assert_eq!(
            condition.items[0].predicates,
            [ComponentPredicate::Damage {
                durability: IntBounds::default(),
                damage: IntBounds {
                    min: Some(2),
                    max: None,
                },
            }]
        );
        assert_eq!(
            condition.items[1].items,
            Some(HolderSet::Ids(vec![
                "minecraft:stone".into(),
                "custom:ore".into(),
                "minecraft:stone".into()
            ]))
        );
        assert_eq!(
            condition.items[2].items,
            Some(HolderSet::Ids(vec!["minecraft:apple".into()]))
        );
        assert!(condition.player.unwrap().as_array().is_some());
        assert!(parse_inventory_changed_condition(&json!({"player": {}})).is_ok());
    }

    #[test]
    fn builtin_inventory_advancement_definitions_are_accepted() {
        let snapshot = crate::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared().unwrap();
        let mut checked = 0;
        for path in snapshot
            .paths()
            .filter(|path| path.starts_with("advancement/") && path.ends_with(".json"))
        {
            let data = bbb_registry_codec::decode_loaded_registry_json(
                snapshot.read_path(path).unwrap(),
                usize::MAX,
            )
            .unwrap();
            let body = data.json().unwrap();
            let Some(criteria) = body.get("criteria").and_then(RegistryJsonValue::as_object) else {
                continue;
            };
            if criteria.iter().any(|(_, criterion)| {
                criterion
                    .get("trigger")
                    .and_then(RegistryJsonValue::utf8_string)
                    .as_deref()
                    == Some("minecraft:inventory_changed")
            }) {
                super::super::parse_advancement(body)
                    .unwrap_or_else(|error| panic!("{path}: {error}"));
                checked += 1;
            }
        }
        assert!(checked > 0);
    }
}
