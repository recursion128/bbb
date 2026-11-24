use bbb_protocol::{
    block_types::vanilla_block_type_for_resource_id, content_registry::ContentRegistry,
    normalize_resource_location, potion_types::vanilla_potion_for_resource_id, NbtSummaryValue,
};
use bbb_registry_codec::registry_codec::canonical_advancement_player_predicate_from_source;
use bbb_registry_codec::RegistryJsonValue;

use crate::frozen_lookup::FrozenDatapackRegistryLookup;
use crate::loot::codec::{id, list, source, V};
use crate::loot::{
    parse_item_predicate_source_with_content_registry,
    parse_location_predicate_source_with_content_registry, parse_loot_condition_nbt,
    parse_loot_condition_source, parse_mob_effects_predicate_source, DamageSourcePredicate,
    DistancePredicate, DoubleBounds, EntityPredicate, IntBounds,
};

use super::block_contact::BlockContactCondition;
use super::inventory::parse_inventory_changed_condition_with_content_registry;
use super::{decode_error, parse_recipe_unlocked_condition, AdvancementDecodeError};

// Criterion.java:8-15; ExtraCodecs.java:592-598.
pub(super) fn validate_criterion_conditions(
    trigger: &str,
    conditions: &RegistryJsonValue,
    block_contact: Option<&BlockContactCondition>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    content_registry: &ContentRegistry,
) -> Result<(), AdvancementDecodeError> {
    conditions
        .as_object()
        .ok_or_else(|| decode_error(format!("{trigger} conditions must be an object")))?;
    let value = V::Json(conditions);
    match trigger {
        "minecraft:impossible" => Ok(()),
        "minecraft:slide_down_block" | "minecraft:enter_block" => {
            if let Some(player) = block_contact.and_then(|contact| contact.player.as_ref()) {
                check_canonical_entity_predicate(player, lookup, content_registry)?;
            }
            Ok(())
        }
        "minecraft:recipe_unlocked" => {
            let parsed = parse_recipe_unlocked_condition(conditions)?;
            if let Some(player) = &parsed.player {
                advancement_entity_predicate(V::Json(player), lookup, content_registry)?;
            }
            Ok(())
        }
        "minecraft:inventory_changed" => {
            let parsed = parse_inventory_changed_condition_with_content_registry(
                conditions,
                content_registry,
            )?;
            if let Some(player) = &parsed.player {
                advancement_entity_predicate(V::Json(player), lookup, content_registry)?;
            }
            Ok(())
        }
        "minecraft:tick"
        | "minecraft:location"
        | "minecraft:hero_of_the_village"
        | "minecraft:voluntary_exile"
        | "minecraft:slept_in_bed"
        | "minecraft:avoid_vibration"
        | "minecraft:started_riding" => opt_entity(value, "player", lookup, content_registry),
        "minecraft:construct_beacon" => {
            opt_int_bounds(value, "level")?;
            opt_entity(value, "player", lookup, content_registry)
        }
        "minecraft:brewed_potion" => {
            if let Some(potion) = value.field("potion") {
                let name = id(potion).map_err(err)?;
                if vanilla_potion_for_resource_id(&name).is_none() {
                    return Err(decode_error(format!("unknown potion {name}")));
                }
            }
            opt_entity(value, "player", lookup, content_registry)
        }
        "minecraft:bee_nest_destroyed" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_item(value, "item", content_registry)?;
            if let Some(block) = value.field("block") {
                let name = id(block).map_err(err)?;
                if vanilla_block_type_for_resource_id(&name).is_none()
                    && content_registry.block_by_resource_id(&name).is_none()
                {
                    return Err(decode_error(format!("unknown block {name}")));
                }
            }
            opt_int_bounds(value, "num_bees_inside")
        }
        "minecraft:recipe_crafted" | "minecraft:crafter_recipe_crafted" => {
            let recipe_id = value
                .field("recipe_id")
                .ok_or_else(|| decode_error(format!("{trigger} recipe_id is missing")))?;
            id(recipe_id).map_err(err)?;
            if let Some(ingredients) = value.field("ingredients") {
                list(ingredients, |item| {
                    parse_item_predicate_source_with_content_registry(item, content_registry)
                })
                .map_err(err)?;
            }
            opt_entity(value, "player", lookup, content_registry)
        }
        "minecraft:placed_block"
        | "minecraft:item_used_on_block"
        | "minecraft:allay_drop_item_on_block"
        | "minecraft:default_block_use"
        | "minecraft:any_block_use" => {
            opt_entity(value, "player", lookup, content_registry)?;
            if let Some(location) = value.field("location") {
                context_aware_predicate(location, lookup, content_registry)?;
            }
            Ok(())
        }
        "minecraft:changed_dimension" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_dimension_key(value, "from")?;
            opt_dimension_key(value, "to")
        }
        "minecraft:nether_travel"
        | "minecraft:fall_from_height"
        | "minecraft:fall_after_explosion"
        | "minecraft:ride_entity_in_lava" => {
            if let Some(start) = value.field("start_position") {
                parse_location_predicate_source_with_content_registry(start, content_registry)
                    .map_err(err)?;
            }
            opt_entity(value, "player", lookup, content_registry)?;
            if let Some(distance) = value.field("distance") {
                DistancePredicate::decode(distance).map_err(err)?;
            }
            if trigger == "minecraft:fall_after_explosion" {
                opt_entity(value, "cause", lookup, content_registry)?;
            }
            Ok(())
        }
        "minecraft:effects_changed" => {
            opt_entity(value, "player", lookup, content_registry)?;
            if let Some(effects) = value.field("effects") {
                parse_mob_effects_predicate_source(effects).map_err(err)?;
            }
            opt_entity(value, "source", lookup, content_registry)
        }
        "minecraft:levitation" => {
            opt_entity(value, "player", lookup, content_registry)?;
            if let Some(distance) = value.field("distance") {
                DistancePredicate::decode(distance).map_err(err)?;
            }
            opt_int_bounds(value, "duration")
        }
        "minecraft:player_killed_entity"
        | "minecraft:entity_killed_player"
        | "minecraft:kill_mob_near_sculk_catalyst" => {
            if let Some(blow) = value.field("killing_blow") {
                for field in ["direct_entity", "source_entity"] {
                    if let Some(predicate) = blow.field(field) {
                        validate_source(predicate)?;
                    }
                }
                DamageSourcePredicate::decode_with_content_registry(blow, content_registry)
                    .map_err(err)?;
            }
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "entity", lookup, content_registry)
        }
        "minecraft:entity_hurt_player" | "minecraft:player_hurt_entity" => {
            if trigger == "minecraft:player_hurt_entity" {
                opt_entity(value, "entity", lookup, content_registry)?;
            }
            opt_entity(value, "player", lookup, content_registry)?;
            if let Some(damage) = value.field("damage") {
                if !damage.is_compound() {
                    return Err(decode_error("damage predicate must be an object"));
                }
                opt_double_bounds(damage, "dealt")?;
                opt_double_bounds(damage, "taken")?;
                if let Some(source_entity) = damage.field("source_entity") {
                    if !source_entity.is_compound() {
                        return Err(decode_error("entity predicate must be an object"));
                    }
                    validate_source(source_entity)?;
                    EntityPredicate::decode_with_content_registry(source_entity, content_registry)
                        .map_err(err)?;
                }
                if let Some(blocked) = damage.field("blocked") {
                    blocked
                        .as_bool()
                        .ok_or_else(|| decode_error("blocked is not a boolean"))?;
                }
                if let Some(damage_type) = damage.field("type") {
                    validate_source(damage_type)?;
                    DamageSourcePredicate::decode_with_content_registry(
                        damage_type,
                        content_registry,
                    )
                    .map_err(err)?;
                }
            }
            Ok(())
        }
        "minecraft:bred_animals" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "parent", lookup, content_registry)?;
            opt_entity(value, "partner", lookup, content_registry)?;
            opt_entity(value, "child", lookup, content_registry)
        }
        "minecraft:cured_zombie_villager" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "zombie", lookup, content_registry)?;
            opt_entity(value, "villager", lookup, content_registry)
        }
        "minecraft:lightning_strike" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "lightning", lookup, content_registry)?;
            opt_entity(value, "bystander", lookup, content_registry)
        }
        "minecraft:channeled_lightning" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity_list(value, "victims", lookup, content_registry)
        }
        "minecraft:killed_by_arrow" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity_list(value, "victims", lookup, content_registry)?;
            opt_int_bounds(value, "unique_entity_types")?;
            opt_item(value, "fired_from_weapon", content_registry)
        }
        "minecraft:target_hit" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_int_bounds(value, "signal_strength")?;
            opt_entity(value, "projectile", lookup, content_registry)
        }
        "minecraft:summoned_entity" | "minecraft:tame_animal" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "entity", lookup, content_registry)
        }
        "minecraft:player_interacted_with_entity" | "minecraft:player_sheared_equipment" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "entity", lookup, content_registry)?;
            opt_item(value, "item", content_registry)
        }
        "minecraft:villager_trade" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "villager", lookup, content_registry)?;
            opt_item(value, "item", content_registry)
        }
        "minecraft:fishing_rod_hooked" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_entity(value, "entity", lookup, content_registry)?;
            opt_item(value, "rod", content_registry)?;
            opt_item(value, "item", content_registry)
        }
        "minecraft:spear_mobs" => {
            opt_entity(value, "player", lookup, content_registry)?;
            if let Some(count) = value.field("count") {
                count
                    .codec_i32()
                    .filter(|count| *count > 0)
                    .ok_or_else(|| decode_error("spear_mobs count must be a positive int"))?;
            }
            Ok(())
        }
        "minecraft:used_ender_eye" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_double_bounds(value, "distance")
        }
        "minecraft:consume_item"
        | "minecraft:using_item"
        | "minecraft:used_totem"
        | "minecraft:filled_bucket"
        | "minecraft:shot_crossbow" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_item(value, "item", content_registry)
        }
        "minecraft:thrown_item_picked_up_by_entity"
        | "minecraft:thrown_item_picked_up_by_player" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_item(value, "item", content_registry)?;
            opt_entity(value, "entity", lookup, content_registry)
        }
        "minecraft:item_durability_changed" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_item(value, "item", content_registry)?;
            opt_int_bounds(value, "durability")?;
            opt_int_bounds(value, "delta")
        }
        "minecraft:enchanted_item" => {
            opt_entity(value, "player", lookup, content_registry)?;
            opt_item(value, "item", content_registry)?;
            opt_int_bounds(value, "levels")
        }
        "minecraft:player_generates_container_loot" => {
            opt_entity(value, "player", lookup, content_registry)?;
            let table = value
                .field("loot_table")
                .ok_or_else(|| decode_error("loot_table is missing"))?;
            id(table).map_err(err)?;
            Ok(())
        }
        _ => Err(decode_error(format!("unknown criterion trigger {trigger}"))),
    }
}

fn err(error: impl ToString) -> AdvancementDecodeError {
    decode_error(error.to_string())
}

fn validate_source(value: V<'_>) -> Result<(), AdvancementDecodeError> {
    bbb_registry_codec::validate_registry_source_input(
        value,
        bbb_registry_codec::PersistentNbtDecodeLimits {
            max_nodes: usize::MAX,
            ..bbb_registry_codec::PersistentNbtDecodeLimits::DEFAULT
        },
    )
    .map_err(err)
}

fn check_canonical_entity_predicate(
    canonical: &NbtSummaryValue,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    content_registry: &ContentRegistry,
) -> Result<(), AdvancementDecodeError> {
    match canonical {
        NbtSummaryValue::List(values) => {
            for value in values {
                parse_loot_condition_nbt(value, lookup).map_err(err)?;
            }
            Ok(())
        }
        predicate @ NbtSummaryValue::Compound(_) => {
            EntityPredicate::decode_with_content_registry(V::Nbt(predicate), content_registry)
                .map(|_| ())
                .map_err(err)
        }
        _ => Err(decode_error(
            "canonical entity predicate is not a compound or list",
        )),
    }
}

fn advancement_entity_predicate(
    value: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    content_registry: &ContentRegistry,
) -> Result<(), AdvancementDecodeError> {
    let canonical =
        canonical_advancement_player_predicate_from_source(value, source(value)).map_err(err)?;
    check_canonical_entity_predicate(&canonical, lookup, content_registry)
}

fn context_aware_predicate(
    value: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    content_registry: &ContentRegistry,
) -> Result<(), AdvancementDecodeError> {
    if value.list_len().is_none() {
        return Err(decode_error("context-aware predicate must be a list"));
    }
    let canonical =
        canonical_advancement_player_predicate_from_source(value, source(value)).map_err(err)?;
    check_canonical_entity_predicate(&canonical, lookup, content_registry)?;
    let len = value.list_len().unwrap();
    for index in 0..len {
        let item = value.list_element(index).unwrap();
        parse_loot_condition_source(item, lookup).map_err(err)?;
    }
    Ok(())
}

fn opt_entity(
    value: V<'_>,
    field: &str,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    content_registry: &ContentRegistry,
) -> Result<(), AdvancementDecodeError> {
    value
        .field(field)
        .map(|entity| advancement_entity_predicate(entity, lookup, content_registry))
        .transpose()
        .map(|_| ())
}

fn opt_entity_list(
    value: V<'_>,
    field: &str,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    content_registry: &ContentRegistry,
) -> Result<(), AdvancementDecodeError> {
    let Some(list) = value.field(field) else {
        return Ok(());
    };
    let len = list
        .list_len()
        .ok_or_else(|| decode_error(format!("{field} must be a list")))?;
    for index in 0..len {
        advancement_entity_predicate(list.list_element(index).unwrap(), lookup, content_registry)?;
    }
    Ok(())
}

fn opt_item(
    value: V<'_>,
    field: &str,
    content_registry: &ContentRegistry,
) -> Result<(), AdvancementDecodeError> {
    value
        .field(field)
        .map(|item| parse_item_predicate_source_with_content_registry(item, content_registry))
        .transpose()
        .map(|_| ())
        .map_err(err)
}

fn opt_int_bounds(value: V<'_>, field: &str) -> Result<(), AdvancementDecodeError> {
    value
        .field(field)
        .map(IntBounds::decode)
        .transpose()
        .map(|_| ())
        .map_err(err)
}

fn opt_double_bounds(value: V<'_>, field: &str) -> Result<(), AdvancementDecodeError> {
    value
        .field(field)
        .map(DoubleBounds::decode)
        .transpose()
        .map(|_| ())
        .map_err(err)
}

fn opt_dimension_key(value: V<'_>, field: &str) -> Result<(), AdvancementDecodeError> {
    let Some(raw) = value.field(field) else {
        return Ok(());
    };
    let raw = raw
        .utf8_string()
        .ok_or_else(|| decode_error(format!("{field} is not a dimension key")))?;
    normalize_resource_location(raw)
        .map(|_| ())
        .map_err(|error| decode_error(error.to_string()))
}
