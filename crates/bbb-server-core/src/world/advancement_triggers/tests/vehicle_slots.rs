use super::*;
use bbb_server_datapack::loot::{parse_loot_table, LootElement, LootElementValue};

fn put_loot_table(startup: &mut DatapackStartup, id: &str, body: Value) {
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    let element = LootElement {
        id: id.into(),
        source_pack: "test".into(),
        value: LootElementValue::Table(parse_loot_table(&body, None).unwrap()),
    };
    match registry
        .elements
        .binary_search_by(|existing| existing.id.as_str().cmp(id))
    {
        Ok(index) => registry.elements[index] = element,
        Err(index) => registry.elements.insert(index, element),
    }
}

#[test]
fn advancement_trigger_and_loot_mine_unpack_pending_vehicle_cargo_slots() {
    let mut world = world(&[]);
    let mut custom = (**built_in_startup()).clone();
    put_loot_table(
        &mut custom,
        "test:vehicle_cargo",
        json!({
            "type": "minecraft:chest",
            "pools": [{
                "rolls": 1,
                "entries": [{"type": "minecraft:item", "name": "minecraft:diamond"}]
            }]
        }),
    );
    put_loot_table(
        &mut custom,
        "minecraft:blocks/sponge",
        json!({
            "type": "minecraft:block",
            "pools": [{
                "rolls": 1,
                "entries": [{
                    "type": "minecraft:slots",
                    "slot_source": {
                        "type": "minecraft:slot_range",
                        "source": "this",
                        "slots": "container.*"
                    }
                }]
            }]
        }),
    );
    custom.advancements = AdvancementSet {
        advancements: vec![AdvancementElement {
            id: "test:riding_diamond_cart".into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&json!({
                "criteria": {
                    "got": {
                        "trigger": "minecraft:inventory_changed",
                        "conditions": {
                            "player": {
                                "vehicle": {
                                    "type": "minecraft:chest_minecart",
                                    "slots": {
                                        "container.*": {"items": "minecraft:diamond"}
                                    }
                                }
                            }
                        }
                    }
                }
            }))
            .unwrap(),
        }],
        warnings: Vec::new(),
    };
    world.cold.config.datapacks = Some(Arc::new(custom));
    world.reload_player_advancement_stores();

    command(
        &mut world,
        "summon minecraft:chest_minecart ~ ~ ~ {UUID:[I;0,0,0,7701],NoGravity:1b,LootTable:\"test:vehicle_cargo\",LootTableSeed:5L}",
    );
    command(
        &mut world,
        "summon minecraft:chest_minecart ~ ~ ~ {UUID:[I;0,0,0,7702],NoGravity:1b,LootTable:\"test:vehicle_cargo\",LootTableSeed:9L}",
    );
    let cart_a = Uuid::from_u128(7701);
    let cart_b = Uuid::from_u128(7702);

    command(&mut world, "setblock ~ ~-1 ~ minecraft:sponge");
    command(
        &mut world,
        &format!("execute as {cart_a} at @s run loot give inventory_player mine ~ ~-1 ~"),
    );
    let diamond_id = vanilla_item_type_for_resource_id("minecraft:diamond")
        .unwrap()
        .id;
    let has_diamond = (0..36).any(|slot| {
        world.levels[CONNECTION]
            .inventory
            .slot(slot)
            .is_some_and(|stack| stack.summary().item_id == Some(diamond_id) && stack.count() == 1)
    });
    assert!(has_diamond, "loot mine unpacked pending vehicle cargo");

    assert!(!world.player_advancement_is_done(CONNECTION, "test:riding_diamond_cart"));
    command(&mut world, &format!("ride inventory_player mount {cart_b}"));
    give(&mut world, "minecraft:stick", 1);
    assert!(
        world.player_advancement_is_done(CONNECTION, "test:riding_diamond_cart"),
        "inventory_changed unpacked pending vehicle cargo in player.vehicle.slots"
    );
}
