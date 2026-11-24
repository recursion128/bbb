//! Allay sample interaction, native item collection, delivery and saved memories.

use super::feeding::interact_entity_packet;
use super::horse_interaction::record;
use super::*;
use crate::entity::{EntityData, ItemEntityData};
use bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_ITEM_ID};
use bbb_protocol::packets::{BlockHitResult, EntityDataValueKind, UseItemOn};
use bbb_server_types::item_stack::RuntimeItemStack;
use serde_json::json;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const ALICE: ConnectionId = ConnectionId(1);
const ALLAY_AT: Vec3d = Vec3d {
    x: 4.5,
    y: 65.0,
    z: 8.5,
};
const PLAYER: Vec3d = Vec3d {
    x: 6.5,
    y: 65.0,
    z: 8.5,
};
const NOTE: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

fn config() -> ServerWorldConfig {
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.keep_alive_interval_ticks = 10_000;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, player, block, marker, context_entity, xp) in [
        (
            "bbb:allay_note",
            "minecraft:player",
            "minecraft:note_block",
            7,
            "minecraft:player",
            7,
        ),
        (
            "bbb:allay_wrong_player",
            "minecraft:cow",
            "minecraft:note_block",
            7,
            "minecraft:player",
            99,
        ),
        (
            "bbb:allay_wrong_location",
            "minecraft:player",
            "minecraft:note_block",
            7,
            "minecraft:cow",
            99,
        ),
        (
            "bbb:allay_wrong_tool",
            "minecraft:player",
            "minecraft:note_block",
            99,
            "minecraft:player",
            99,
        ),
    ] {
        startup.advancements.advancements.push(bbb_server_datapack::advancement::AdvancementElement {
            id: id.into(), source_pack: "test".into(),
            advancement: crate::tests::parse_advancement(&json!({
                "criteria":{"drop":{"trigger":"minecraft:allay_drop_item_on_block","conditions":{
                    "player":{"type":player},"location":[
                        {"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":context_entity}},
                        {"condition":"minecraft:location_check","predicate":{"block":{"blocks":block}}},
                        {"condition":"minecraft:match_tool","predicate":{"items":"minecraft:amethyst_shard","components":{"minecraft:custom_data":{"collected":marker}}}}
                    ]}}},"rewards":{"experience":xp}
            })).unwrap(),
        });
    }
    config
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, 100, text);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn prepare(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    world.tick_rate_mut().set_frozen(true);
    join_loaded(sender, receiver, world, 1, "alice", 1);
    for x in 0..16 {
        for z in 0..16 {
            world.set_block(
                DIM,
                BlockPos { x, y: 64, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    command(sender, receiver, world, "gamemode survival alice");
    command(sender, receiver, world, "gamerule spawn_mobs false");
    world.set_player_position_for_test(ALICE, PLAYER);
}

fn fixture() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world);
    command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:allay 4.5 65 8.5",
    );
    let allay = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_ALLAY_ID
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIM, allay).unwrap().uuid();
    (sender, receiver, world, allay, uuid)
}

fn memories(saved: &[NbtSummaryEntry]) -> &[NbtSummaryEntry] {
    let NbtSummaryValue::Compound(brain) = field(saved, "Brain") else {
        panic!("Brain compound")
    };
    let NbtSummaryValue::Compound(memories) = field(brain, "memories") else {
        panic!("memories compound")
    };
    memories
}

fn memory(world: &ServerWorld, allay: i32, name: &str) -> Option<NbtSummaryValue> {
    memories(&record(world, allay))
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value.clone())
}

fn memory_int(world: &ServerWorld, allay: i32, name: &str) -> i32 {
    let Some(NbtSummaryValue::Compound(fields)) = memory(world, allay, name) else {
        panic!("integer memory {name}")
    };
    match field(&fields, "value") {
        NbtSummaryValue::Int(value) => *value,
        other => panic!("integer value: {other:?}"),
    }
}

fn inventory(world: &ServerWorld, allay: i32) -> Vec<RuntimeItemStack> {
    let saved = record(world, allay);
    let NbtSummaryValue::List(items) = field(&saved, "Inventory") else {
        panic!("Allay Inventory list")
    };
    items
        .iter()
        .map(|item| RuntimeItemStack::try_read(item, &world.item_stack_resolver()).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect()
}

fn stack(world: &ServerWorld, text: &str) -> RuntimeItemStack {
    RuntimeItemStack::try_read(
        &bbb_protocol::parse_snbt_compound(text).unwrap(),
        &world.item_stack_resolver(),
    )
    .unwrap()
}

fn item(world: &mut ServerWorld, position: Vec3d, stack: RuntimeItemStack) -> i32 {
    world
        .spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            position,
            EntityData::Item(ItemEntityData::new(stack, 0.0)),
        )
        .0
}

fn dropped(world: &ServerWorld) -> Vec<RuntimeItemStack> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| match world.stored_entity_for_test(DIM, id)?.data() {
            EntityData::Item(item) => Some(item.stack().clone()),
            _ => None,
        })
        .collect()
}

fn give_sample(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    allay: i32,
    sample: &str,
) {
    command(
        sender,
        receiver,
        world,
        &format!("item replace entity alice weapon.mainhand with {sample}"),
    );
    play(sender, receiver, world, 1, interact_entity_packet(allay));
    let Some(NbtSummaryValue::Compound(fields)) = memory(world, allay, "minecraft:liked_player")
    else {
        panic!("giving a sample records the liked player")
    };
    assert_eq!(field(&fields, "value"), &nbt_uuid(Uuid::from_u128(1)));
    assert!(world.player_inventory_slot_for_test(ALICE, 0).count == 0);
}

fn player_has(world: &ServerWorld, expected: &RuntimeItemStack) -> bool {
    (0..36).any(|slot| {
        world
            .player_runtime_inventory_slot_for_test(ALICE, slot)
            .summary()
            == expected.summary()
    })
}

#[test]
fn a_sample_allay_rejects_a_different_potion_then_delivers_complete_matching_components() {
    let (sender, receiver, mut world, allay, uuid) = fixture();
    give_sample(
        &sender,
        &receiver,
        &mut world,
        allay,
        "potion[potion_contents={potion:\"minecraft:water\"},custom_data={sample:1}]",
    );
    let health = world
        .stored_entity_for_test(DIM, allay)
        .unwrap()
        .living()
        .unwrap()
        .health();
    console_command(
        &sender,
        &receiver,
        &mut world,
        101,
        &format!("damage {uuid} 4 minecraft:generic by alice"),
    );
    assert_eq!(
        world
            .stored_entity_for_test(DIM, allay)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        health,
        "the liked player cannot hurt its allay"
    );
    let wrong = stack(
        &world,
        r#"{id:"minecraft:potion",count:1,components:{"minecraft:potion_contents":{potion:"minecraft:healing"}}}"#,
    );
    let wrong = item(&mut world, ALLAY_AT, wrong);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    assert!(world.stored_entity_for_test(DIM, wrong).is_some());
    assert!(inventory(&world, allay).is_empty());
    world.set_player_position_for_test(ALICE, Vec3d { x: 12.5, ..PLAYER });
    let matching = stack(
        &world,
        r#"{id:"minecraft:potion",count:1,components:{"minecraft:potion_contents":{potion:"minecraft:water"},"minecraft:custom_data":{collected:7}}}"#,
    );
    let at = world.stored_entity_for_test(DIM, allay).unwrap().position();
    item(&mut world, at, matching.clone());
    let mut collected = false;
    for _ in 0..300 {
        world.tick(receiver.drain());
        collected |= inventory(&world, allay)
            .iter()
            .any(|stack| stack.summary() == matching.summary());
        if player_has(&world, &matching) {
            assert!(
                collected,
                "matching pickup preserves the complete component-bearing stack"
            );
            return;
        }
    }
    panic!("the allay never delivered the matching potion to its liked player");
}

fn tune(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            sequence: 1,
            hit: BlockHitResult {
                pos: NOTE,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
        }),
    )
}

#[test]
fn a_real_note_vibration_redirects_delivery_and_checks_player_location_and_tool_predicates() {
    let (sender, receiver, mut world, allay, _uuid) = fixture();
    give_sample(
        &sender,
        &receiver,
        &mut world,
        allay,
        "amethyst_shard[custom_data={sample:1}]",
    );
    let note = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID,
    )
    .unwrap();
    world.set_block(DIM, NOTE, note, SET_BLOCK_UPDATE_ALL);
    world.tick_rate_mut().set_frozen(false);
    tune(&sender, &receiver, &mut world);
    for _ in 0..30 {
        world.tick(receiver.drain());
        if memory(&world, allay, "minecraft:liked_noteblock").is_some() {
            break;
        }
    }
    assert!(
        memory(&world, allay, "minecraft:liked_noteblock").is_some(),
        "the real note reaches the Allay vibration listener"
    );
    assert!((570..=600).contains(&memory_int(
        &world,
        allay,
        "minecraft:liked_noteblock_cooldown_ticks"
    )));
    for _ in 0..80 {
        world.tick(receiver.drain());
    }
    let before = memory_int(&world, allay, "minecraft:liked_noteblock_cooldown_ticks");
    tune(&sender, &receiver, &mut world);
    for _ in 0..30 {
        world.tick(receiver.drain());
    }
    assert!(
        memory_int(&world, allay, "minecraft:liked_noteblock_cooldown_ticks") > before,
        "the same note renews its binding"
    );
    let carried = stack(
        &world,
        r#"{id:"minecraft:amethyst_shard",count:1,components:{"minecraft:custom_data":{collected:7b}}}"#,
    );
    let at = world.stored_entity_for_test(DIM, allay).unwrap().position();
    item(&mut world, at, carried);
    for _ in 0..240 {
        world.tick(receiver.drain());
        if world.player_advancement_is_done(ALICE, "bbb:allay_note") {
            break;
        }
    }
    assert!(world.player_advancement_is_done(ALICE, "bbb:allay_note"));
    for id in [
        "bbb:allay_wrong_player",
        "bbb:allay_wrong_location",
        "bbb:allay_wrong_tool",
    ] {
        assert!(!world.player_advancement_is_done(ALICE, id), "{id}");
    }
    let reward = command(
        &sender,
        &receiver,
        &mut world,
        "experience query alice levels",
    );
    assert_eq!(
        console_command_answer(&reward).0,
        Some(1),
        "the native reward grants exactly seven experience points"
    );
    assert!((1..=60).contains(&memory_int(
        &world,
        allay,
        "minecraft:item_pickup_cooldown_ticks"
    )));
    world.set_block(
        DIM,
        NOTE,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    );
    world.set_player_position_for_test(ALICE, Vec3d { x: 13.5, ..PLAYER });
    for _ in 0..80 {
        world.tick(receiver.drain());
    }
    assert!(memory(&world, allay, "minecraft:liked_noteblock").is_none());
    let fallback = stack(
        &world,
        r#"{id:"minecraft:amethyst_shard",count:1,components:{"minecraft:custom_data":{collected:9}}}"#,
    );
    let at = world.stored_entity_for_test(DIM, allay).unwrap().position();
    item(&mut world, at, fallback.clone());
    for _ in 0..300 {
        world.tick(receiver.drain());
        if player_has(&world, &fallback) {
            return;
        }
    }
    panic!("destroying the note never returned delivery to the liked player");
}

#[test]
fn sample_inventory_and_independent_memories_reload_and_empty_hand_retrieves_everything() {
    let (sender, receiver, mut world, allay, uuid) = fixture();
    give_sample(
        &sender,
        &receiver,
        &mut world,
        allay,
        "poppy[custom_data={sample:1}]",
    );
    let sample = world
        .stored_entity_for_test(DIM, allay)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        .unwrap()
        .clone();
    world.set_player_position_for_test(ALICE, Vec3d { x: 12.5, ..PLAYER });
    let carried = stack(
        &world,
        r#"{id:"minecraft:poppy",count:3,components:{"minecraft:custom_data":{kept:17}}}"#,
    );
    item(&mut world, ALLAY_AT, carried.clone());
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        world.tick(receiver.drain());
        if !inventory(&world, allay).is_empty() {
            break;
        }
    }
    assert_eq!(
        inventory(&world, allay).first().unwrap().summary(),
        carried.summary()
    );
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    for (name, value) in [
        ("minecraft:liked_player", "{value:[I;0,0,0,1],ttl:900L}"),
        (
            "minecraft:liked_noteblock",
            "{value:{dimension:\"minecraft:overworld\",pos:[I;8,65,8]}}",
        ),
        (
            "minecraft:liked_noteblock_cooldown_ticks",
            "{value:500,ttl:700L}",
        ),
        (
            "minecraft:item_pickup_cooldown_ticks",
            "{value:300,ttl:1100L}",
        ),
    ] {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{Brain:{{memories:{{\"{name}\":{value}}}}}}}"),
        );
    }
    let before = record(&world, allay);
    let saved_memories = memories(&before).to_vec();
    let health = world
        .stored_entity_for_test(DIM, allay)
        .unwrap()
        .living()
        .unwrap()
        .health();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 4 minecraft:generic"),
    );
    let wounded = world
        .stored_entity_for_test(DIM, allay)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(wounded < health);
    let request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(memories(&saved), saved_memories);
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let allay = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(memories(&record(&restored, allay)), saved_memories);
    assert_eq!(
        inventory(&restored, allay).first().unwrap().summary(),
        carried.summary()
    );
    assert_eq!(
        restored
            .stored_entity_for_test(DIM, allay)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
            .unwrap()
            .summary(),
        sample.summary()
    );
    prepare(&sender, &receiver, &mut restored);
    place_player(&sender, &receiver, &mut restored, 1, PLAYER, None);
    assert!(restored
        .player_tracking_viewers_for_test(DIM, allay)
        .contains(&ALICE));
    restored.tick_rate_mut().set_frozen(false);
    let mut published_healing = false;
    for _ in 0..180 {
        let tick = restored.tick(receiver.drain());
        published_healing |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                if data.id == allay && data.values.iter().any(|value|
                    value.data_id == VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
                    && matches!(value.value, EntityDataValueKind::Float(health) if health > wounded))
        ));
    }
    assert!(
        published_healing,
        "the client receives the allay's healing health metadata"
    );
    assert_eq!(
        memories(&record(&restored, allay)),
        saved_memories,
        "NoAI preserves values and their independent TTLs"
    );
    assert!(
        restored
            .stored_entity_for_test(DIM, allay)
            .unwrap()
            .living()
            .unwrap()
            .health()
            > wounded,
        "NoAI does not stop the allay's aiStep healing"
    );
    let at = restored
        .stored_entity_for_test(DIM, allay)
        .unwrap()
        .position();
    restored.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: at.x + 2.0,
            ..at
        },
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        "item replace entity alice weapon.mainhand with air",
    );
    play(
        &sender,
        &receiver,
        &mut restored,
        1,
        interact_entity_packet(allay),
    );
    assert!(inventory(&restored, allay).is_empty());
    assert!(memory(&restored, allay, "minecraft:liked_player").is_none());
    let hand = restored
        .stored_entity_for_test(DIM, allay)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand);
    assert!(hand.is_none_or(|stack| stack.is_empty()));
    assert!(
        dropped(&restored)
            .iter()
            .any(|stack| stack.summary() == carried.summary()),
        "empty-hand retrieval throws the complete internal stack"
    );
    assert!(
        player_has(&restored, &sample),
        "the complete sample returns to the player"
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("damage {uuid} 4 minecraft:generic"),
    );
    let wounded_again = restored
        .stored_entity_for_test(DIM, allay)
        .unwrap()
        .living()
        .unwrap()
        .health();
    for _ in 0..30 {
        restored.tick(receiver.drain());
    }
    assert!(
        restored
            .stored_entity_for_test(DIM, allay)
            .unwrap()
            .living()
            .unwrap()
            .health()
            > wounded_again,
        "the active allay heals itself"
    );
}

fn allay_boolean_metadata(tick: &TickResult, allay: i32, data_id: u8, expected: bool) -> bool {
    tick.events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::SetEntityData(data))
            if data.id == allay && data.values.iter().any(|value|
                value.data_id == data_id && value.value == EntityDataValueKind::Boolean(expected))
    ))
}

fn duplication_cooldown(world: &ServerWorld, allay: i32) -> i64 {
    match field(&record(world, allay), "DuplicationCooldown") {
        NbtSummaryValue::Long(value) => *value,
        other => panic!("the duplication cooldown is saved as a long: {other:?}"),
    }
}

#[test]
fn jukebox_dancing_duplicates_a_fresh_allay_and_saved_cooldowns_run_without_ai() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID, VANILLA_ALLAY_DANCING_DATA_ID,
    };

    let (sender, receiver, mut world, parent, parent_uuid) = fixture();
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    command(&sender, &receiver, &mut world, "gamemode spectator bob");
    place_player(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            z: PLAYER.z + 2.0,
            ..PLAYER
        },
        None,
    );
    give_sample(
        &sender,
        &receiver,
        &mut world,
        parent,
        "poppy[custom_data={sample:1}]",
    );
    let sample = world
        .stored_entity_for_test(DIM, parent)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        .unwrap()
        .clone();
    world.set_player_position_for_test(ALICE, Vec3d { x: 12.5, ..PLAYER });
    let cargo = stack(
        &world,
        r#"{id:"minecraft:poppy",count:3,components:{"minecraft:custom_data":{cargo:17}}}"#,
    );
    item(&mut world, ALLAY_AT, cargo.clone());
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        world.tick(receiver.drain());
        if !inventory(&world, parent).is_empty() {
            break;
        }
    }
    assert_eq!(inventory(&world, parent)[0].summary(), cargo.summary());
    world.tick_rate_mut().set_frozen(true);
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d { x: 10.5, ..PLAYER },
        None,
    );
    world.set_block(
        DIM,
        NOTE,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_JUKEBOX_ID,
        )
        .unwrap(),
        SET_BLOCK_UPDATE_ALL,
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:music_disc_cat",
    );
    let started = tune(&sender, &receiver, &mut world);
    let mut dancing = allay_boolean_metadata(&started, parent, VANILLA_ALLAY_DANCING_DATA_ID, true);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..30 {
        let tick = world.tick(receiver.drain());
        dancing |= allay_boolean_metadata(&tick, parent, VANILLA_ALLAY_DANCING_DATA_ID, true);
        if dancing {
            break;
        }
    }
    assert!(
        dancing,
        "a real playing jukebox publishes the dancing animation"
    );
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(inventory(&world, parent)[0].summary(), cargo.summary());
    let position = world
        .stored_entity_for_test(DIM, parent)
        .unwrap()
        .position();
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
        None,
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:amethyst_shard 2",
    );
    let duplicated = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(parent),
    );
    let children = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| {
            *id != parent
                && world
                    .stored_entity_for_test(DIM, *id)
                    .unwrap()
                    .entity_type_id()
                    == VANILLA_ENTITY_TYPE_ALLAY_ID
        })
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 1);
    let child = children[0];
    let child_uuid = world.stored_entity_for_test(DIM, child).unwrap().uuid();
    assert_ne!(parent_uuid, child_uuid);
    assert!(duplicated.events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::AddEntity(entity)) if entity.id == child
    )));
    assert!(duplicated.events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::EntityEvent(event))
            if event.entity_id == parent && event.event_id == 18
    )));
    let chime_listeners = duplicated
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| {
            let (sound, volume, pitch) = match packet {
                ClientboundPlayPacket::Sound(sound) => (&sound.sound, sound.volume, sound.pitch),
                ClientboundPlayPacket::SoundEntity(sound) => {
                    (&sound.sound, sound.volume, sound.pitch)
                }
                _ => return None,
            };
            if matches!(sound, SoundEventHolder::Reference { registry_id }
                if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:block.amethyst_block.chime")) {
                assert_eq!((volume, pitch), (2.0, 1.0));
                Some(connection_id)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert!(
        chime_listeners.contains(&ConnectionId(2)),
        "the observer hears the duplication chime"
    );
    assert!(
        !chime_listeners.contains(&ALICE),
        "the interacting player is excluded from the server sound"
    );
    assert_eq!(world.player_inventory_slot_for_test(ALICE, 0).count, 1);
    for id in [parent, child] {
        assert!((1..=6000).contains(&duplication_cooldown(&world, id)));
        assert!(
            allay_boolean_metadata(&duplicated, id, VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID, false),
            "parent and fresh child publish their duplication cooldown"
        );
    }
    assert!(inventory(&world, child).is_empty());
    assert!(memories(&record(&world, child)).is_empty());
    assert!(world
        .stored_entity_for_test(DIM, child)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        .is_none_or(|item| item.is_empty()));
    assert_eq!(
        field(&record(&world, child), "PersistenceRequired"),
        &NbtSummaryValue::Byte(1)
    );
    assert_eq!(inventory(&world, parent)[0].summary(), cargo.summary());
    assert_eq!(
        world
            .stored_entity_for_test(DIM, parent)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
            .unwrap()
            .summary(),
        sample.summary()
    );
    assert!(memory(&world, parent, "minecraft:liked_player").is_some());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(parent),
    );
    assert_eq!(
        world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .filter(|id| world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_ALLAY_ID)
            .count(),
        2,
        "the positive cooldown prevents another duplication"
    );
    assert_eq!(world.player_inventory_slot_for_test(ALICE, 0).count, 1);

    for uuid in [parent_uuid, child_uuid] {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{NoAI:1b}}"),
        );
    }
    let saved_cooldowns = [
        duplication_cooldown(&world, parent),
        duplication_cooldown(&world, child),
    ];
    let request = world
        .save_request(1_786_000_005_000, 205, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request);
    for (uuid, cooldown) in [parent_uuid, child_uuid].into_iter().zip(saved_cooldowns) {
        let saved = saved
            .iter()
            .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
            .unwrap();
        assert_eq!(
            field(saved, "DuplicationCooldown"),
            &NbtSummaryValue::Long(cooldown)
        );
    }
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let parent = restored
        .live_entity_id_for_uuid_for_test(DIM, parent_uuid)
        .unwrap();
    let child = restored
        .live_entity_id_for_uuid_for_test(DIM, child_uuid)
        .unwrap();
    assert_eq!(
        [
            duplication_cooldown(&restored, parent),
            duplication_cooldown(&restored, child)
        ],
        saved_cooldowns
    );
    prepare(&sender, &receiver, &mut restored);
    place_player(&sender, &receiver, &mut restored, 1, position, None);
    for id in [parent, child] {
        assert!(restored
            .player_tracking_viewers_for_test(DIM, id)
            .contains(&ALICE));
    }
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        restored.tick(receiver.drain());
    }
    for (id, previous) in [parent, child].into_iter().zip(saved_cooldowns) {
        assert!(
            duplication_cooldown(&restored, id) < previous,
            "NoAI does not stop the duplication cooldown"
        );
    }
    restored.tick_rate_mut().set_frozen(true);
    for uuid in [parent_uuid, child_uuid] {
        command(
            &sender,
            &receiver,
            &mut restored,
            &format!("data merge entity {uuid} {{DuplicationCooldown:3L}}"),
        );
    }
    restored.tick_rate_mut().set_frozen(false);
    let mut ready = [false; 2];
    for _ in 0..20 {
        let tick = restored.tick(receiver.drain());
        for (index, id) in [parent, child].into_iter().enumerate() {
            ready[index] |=
                allay_boolean_metadata(&tick, id, VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID, true);
        }
    }
    assert!(
        ready.into_iter().all(|ready| ready),
        "clients see both allays become ready again"
    );
    assert_eq!(
        [
            duplication_cooldown(&restored, parent),
            duplication_cooldown(&restored, child)
        ],
        [0, 0]
    );
}
