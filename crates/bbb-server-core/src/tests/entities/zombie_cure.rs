//! A golden apple cures a weakened zombie villager: the countdown survives a
//! reload, and the villager it becomes keeps its trades and rewards the curer
//! (`ZombieVillager.java:168-275`).

use super::feeding::{interact_entity_packet, stack_of};
use super::*;
use crate::entity::item::plain_stack;
use crate::entity::living::ActiveMobEffect;
use crate::entity::{MerchantItemCost, MerchantOffer, MerchantOffers};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_VILLAGER_ID as VILLAGER,
    VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID as ZOMBIE_VILLAGER,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn record(world: &ServerWorld, entity: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(OVERWORLD, entity)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries
}

fn field(entries: &[NbtSummaryEntry], name: &str) -> Option<NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value.clone())
}

/// `/data merge entity` with `fields`.
fn merge(world: &mut ServerWorld, entity: i32, fields: Vec<NbtSummaryEntry>) {
    let mut entries = record(world, entity);
    for field in fields {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    world.load_stored_entity_record_for_test(OVERWORLD, entity, &entries);
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn spawn(world: &mut ServerWorld, entity_type: i32, dx: f64) -> i32 {
    let position = world.publication().players[0].position;
    world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            entity_type,
            Vec3d {
                x: position.x + dx,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(entity_type).unwrap()),
        )
        .0
}

fn live_ids(world: &ServerWorld, entity_type: i32) -> Vec<i32> {
    (1..256)
        .filter(|id| {
            world
                .stored_entity_for_test(OVERWORLD, *id)
                .is_some_and(|entity| {
                    entity.entity_type_id() == entity_type && entity.removal_reason().is_none()
                })
        })
        .collect()
}

fn entity_event_sent(events: &[WorldEvent], entity_id: i32, event_id: i8) -> bool {
    events.iter().any(|event| {
        matches!(event, WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityEvent(sent),
            ..
        } if sent.entity_id == entity_id && sent.event_id == event_id)
    })
}

/// The `Offers` record of a farmer's single wheat trade, as a villager saves it.
fn farmer_offers(world: &mut ServerWorld) -> NbtSummaryValue {
    let villager = spawn(world, VILLAGER, 6.0);
    let offer = MerchantOffer::new(
        MerchantItemCost::new("minecraft:wheat".into(), 20, Vec::new()).unwrap(),
        None,
        plain_stack("minecraft:emerald", 1).unwrap(),
        16,
        2,
        0.05,
    );
    assert!(world
        .stored_entity_mut_for_test(OVERWORLD, villager)
        .unwrap()
        .initialize_merchant_offers(MerchantOffers::new(vec![offer])));
    let offers = field(&record(world, villager), "Offers").expect("a villager saves its offers");
    world
        .stored_entity_mut_for_test(OVERWORLD, villager)
        .unwrap()
        .discard();
    offers
}

fn farmer_data() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("type", NbtSummaryValue::String("minecraft:plains".into())),
        entry(
            "profession",
            NbtSummaryValue::String("minecraft:farmer".into()),
        ),
        entry("level", NbtSummaryValue::Int(2)),
    ])
}

#[test]
fn a_golden_apple_cures_a_weakened_zombie_villager_into_a_villager_that_keeps_its_trades() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, zombie_variant, villager_variant) in [
        ("test:cure_plains", "minecraft:plains", "minecraft:plains"),
        (
            "test:cure_desert_zombie",
            "minecraft:desert",
            "minecraft:plains",
        ),
        (
            "test:cure_desert_villager",
            "minecraft:plains",
            "minecraft:desert",
        ),
    ] {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&serde_json::json!({
                    "criteria":{"cured":{
                        "trigger":"minecraft:cured_zombie_villager",
                        "conditions":{
                            "zombie":[{
                                "condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{
                                    "type":"minecraft:zombie_villager",
                                    "components":{"minecraft:villager/variant":zombie_variant}
                                }
                            }],
                            "villager":[{
                                "condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{
                                    "type":"minecraft:villager",
                                    "predicates":{"minecraft:villager/variant":villager_variant}
                                }
                            }]
                        }
                    }}
                }))
                .unwrap(),
            },
        );
    }
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "curer", 0xc0de);
    let player_uuid = Uuid::from_u128(0xc0de);
    let offers = farmer_offers(&mut world);
    let zombie_villager = spawn(&mut world, ZOMBIE_VILLAGER, 1.5);
    let pos = world
        .stored_entity_for_test(OVERWORLD, zombie_villager)
        .unwrap()
        .block_position();
    let filled = console_command(
        &sender,
        &receiver,
        &mut world,
        100,
        &format!(
            "fillbiome {} {} {} {} {} {} minecraft:plains",
            pos.x - 8,
            pos.y - 8,
            pos.z - 8,
            pos.x + 8,
            pos.y + 8,
            pos.z + 8,
        ),
    );
    assert!(console_command_answer(&filled).0.is_some());
    merge(
        &mut world,
        zombie_villager,
        vec![
            entry("VillagerData", farmer_data()),
            entry("Offers", offers.clone()),
            entry("Xp", NbtSummaryValue::Int(14)),
        ],
    );
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:golden_apple", 2));

    // Without weakness the apple is refused and kept.
    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(zombie_villager),
    );
    assert_eq!(
        refused.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:golden_apple", 2)
    );
    assert_eq!(
        field(&record(&world, zombie_villager), "ConversionTime"),
        Some(NbtSummaryValue::Int(-1))
    );

    world
        .stored_entity_mut_for_test(OVERWORLD, zombie_villager)
        .and_then(|entity| entity.living_mut())
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:weakness", 1200, 0).unwrap());
    let cured = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(zombie_villager),
    );
    assert_eq!(
        cured.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:golden_apple", 1)
    );
    assert!(entity_event_sent(&cured.events, zombie_villager, 16));
    let started = record(&world, zombie_villager);
    let Some(NbtSummaryValue::Int(time)) = field(&started, "ConversionTime") else {
        panic!("a converting zombie villager saves its countdown")
    };
    assert!((3590..=6000).contains(&time));
    assert!(field(&started, "ConversionPlayer").is_some());
    let living = world
        .stored_entity_for_test(OVERWORLD, zombie_villager)
        .and_then(|entity| entity.living())
        .unwrap();
    assert!(living.active_effect("minecraft:weakness").is_none());
    assert!(living.active_effect("minecraft:strength").is_some());

    let uuid = world
        .stored_entity_for_test(OVERWORLD, zombie_villager)
        .unwrap()
        .uuid();
    let cleared = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!("effect clear {uuid} minecraft:strength"),
    );
    assert_eq!(console_command_answer(&cleared).0, Some(1));
    let living = world
        .stored_entity_for_test(OVERWORLD, zombie_villager)
        .unwrap()
        .living()
        .unwrap();
    assert!(living.active_effect("minecraft:strength").is_none());
    let attack_damage =
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID);
    let cleared_record = record(&world, zombie_villager);
    assert!(
        matches!(field(&cleared_record, "ConversionTime"), Some(NbtSummaryValue::Int(time)) if time > 0)
    );
    assert_eq!(
        field(&cleared_record, "ConversionPlayer"),
        field(&started, "ConversionPlayer")
    );
    let loaded = console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        &format!("data merge entity {uuid} {{ConversionTime:200}}"),
    );
    assert_eq!(console_command_answer(&loaded).0, Some(1));
    let living = world
        .stored_entity_for_test(OVERWORLD, zombie_villager)
        .unwrap()
        .living()
        .unwrap();
    assert_eq!(
        living
            .active_effect("minecraft:strength")
            .unwrap()
            .amplifier(),
        0
    );
    assert_eq!(
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,),
        attack_damage + 3.0
    );
    let loaded_record = record(&world, zombie_villager);
    assert_eq!(
        field(&loaded_record, "ConversionPlayer"),
        field(&started, "ConversionPlayer")
    );
    let Some(NbtSummaryValue::List(effects)) = field(&loaded_record, "active_effects") else {
        panic!("the loaded cure saves its active effects");
    };
    assert!(effects.iter().any(|effect| matches!(effect,
        NbtSummaryValue::Compound(entries)
            if field(entries, "id") == Some(NbtSummaryValue::String("minecraft:strength".into()))
    )));

    // The countdown and its curer come back with a reloaded body.
    let reloaded = spawn(&mut world, ZOMBIE_VILLAGER, -1.5);
    world.load_stored_entity_record_for_test(OVERWORLD, reloaded, &started);
    let reloaded_record = record(&world, reloaded);
    assert_eq!(
        field(&reloaded_record, "ConversionTime"),
        Some(NbtSummaryValue::Int(time))
    );
    assert_eq!(
        field(&reloaded_record, "ConversionPlayer"),
        field(&started, "ConversionPlayer")
    );
    world
        .stored_entity_mut_for_test(OVERWORLD, reloaded)
        .unwrap()
        .discard();

    // Run the countdown out.
    merge(
        &mut world,
        zombie_villager,
        vec![entry("ConversionTime", NbtSummaryValue::Int(1))],
    );
    world.tick(receiver.drain());

    assert!(live_ids(&world, ZOMBIE_VILLAGER).is_empty());
    let villagers = live_ids(&world, VILLAGER);
    assert_eq!(villagers.len(), 1);
    let villager = record(&world, villagers[0]);
    let Some(NbtSummaryValue::Compound(data)) = field(&villager, "VillagerData") else {
        panic!("a villager saves its data")
    };
    assert_eq!(
        field(&data, "type"),
        Some(NbtSummaryValue::String("minecraft:plains".into()))
    );
    assert_eq!(
        field(&data, "profession"),
        Some(NbtSummaryValue::String("minecraft:farmer".into()))
    );
    assert_eq!(field(&data, "level"), Some(NbtSummaryValue::Int(2)));
    assert_eq!(field(&villager, "Offers"), Some(offers));
    assert_eq!(field(&villager, "Xp"), Some(NbtSummaryValue::Int(14)));
    let Some(NbtSummaryValue::List(gossips)) = field(&villager, "Gossips") else {
        panic!("a villager saves its gossips")
    };
    let curer = crate::entity::save::uuid_int_array(player_uuid);
    let gossip = |kind: &str| {
        gossips.iter().find_map(|gossip| match gossip {
            NbtSummaryValue::Compound(entries)
                if field(entries, "Target") == Some(curer.clone())
                    && field(entries, "Type") == Some(NbtSummaryValue::String(kind.into())) =>
            {
                field(entries, "Value")
            }
            _ => None,
        })
    };
    assert_eq!(gossip("major_positive"), Some(NbtSummaryValue::Int(20)));
    assert_eq!(gossip("minor_positive"), Some(NbtSummaryValue::Int(25)));
    assert!(world
        .stored_entity_for_test(OVERWORLD, villagers[0])
        .and_then(|entity| entity.living())
        .is_some_and(|living| living.active_effect("minecraft:nausea").is_some()));
    assert!(
        world.player_advancement_is_done(ConnectionId(1), "minecraft:story/cure_zombie_villager")
    );
    for (id, completed) in [
        ("test:cure_plains", true),
        ("test:cure_desert_zombie", false),
        ("test:cure_desert_villager", false),
    ] {
        assert_eq!(
            world.player_advancement_is_done(ConnectionId(1), id),
            completed,
            "{id}"
        );
    }
}

/// `ZombieVillager.finishConversion` keeps curse-of-binding gear and moves it
/// into the villager's inventory slot `index` (`ZombieVillager.java:239-247`).
#[test]
fn a_cured_zombie_villager_keeps_its_bound_helmet_in_the_villager_inventory() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "curer", 0xc0df);
    let zombie_villager = spawn(&mut world, ZOMBIE_VILLAGER, 1.5);
    let uuid = world
        .stored_entity_for_test(OVERWORLD, zombie_villager)
        .unwrap()
        .uuid();
    for (id, command) in [
        (
            10,
            format!(
                "item replace entity {uuid} armor.head with \
                 minecraft:iron_helmet[minecraft:enchantments={{\"minecraft:binding_curse\":1}}]"
            ),
        ),
        (11, format!("data merge entity {uuid} {{ConversionTime:1}}")),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, id, &command);
        assert_eq!(console_command_answer(&tick).0, Some(1), "{command}");
    }
    for _ in 0..3 {
        world.tick(Vec::new());
    }
    let villager = live_ids(&world, VILLAGER)[0];
    let inventory = format!("{:?}", field(&record(&world, villager), "Inventory"));
    assert!(inventory.contains("minecraft:iron_helmet"), "{inventory}");
}
