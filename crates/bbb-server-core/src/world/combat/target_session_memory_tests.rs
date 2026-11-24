use super::*;
use crate::entity::{ArmorStandData, EntityData, GeneralLivingData, MinecartData};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{ClientCommandAction, PlayServerbound};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const PLAYER: ConnectionId = ConnectionId(1);
const SOURCE: Vec3d = Vec3d {
    x: 8.5,
    y: 65.0,
    z: 8.5,
};
const TARGET: Vec3d = Vec3d { x: 9.5, ..SOURCE };

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn fixture() -> ServerWorld {
    fixture_with_players(false)
}

fn fixture_with_players(second: bool) -> ServerWorld {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: GameType::Survival,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    world.set_test_respawn_radius(0);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    if second {
        crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    }
    world.ensure_entity_transfer_full_chunk_for_test(DIM, (0, 0));
    for x in 7..=11 {
        for z in 7..=10 {
            for y in 64..=69 {
                let pos = BlockPos { x, y, z };
                let value = state(if y == 64 {
                    "minecraft:stone"
                } else {
                    "minecraft:air"
                });
                if world.block_state(DIM, pos) != Some(value) {
                    assert!(world.set_block(DIM, pos, value, SET_BLOCK_UPDATE_ALL));
                }
                assert_eq!(world.block_state(DIM, pos), Some(value));
            }
        }
    }
    world.set_player_position_for_test(PLAYER, SOURCE);
    assert!(world.levels[PLAYER].client_loaded());
    assert_eq!(memory(&world), (None, 0));
    world
}

fn memory(world: &ServerWorld) -> (Option<Uuid>, i32) {
    let player = world.levels.player_at(PLAYER);
    (
        player.cold.last_hurt_mob.map(|(_, uuid)| uuid),
        player.cold.last_hurt_mob_timestamp,
    )
}

fn dispatch(world: &mut ServerWorld, packet: PlayServerbound) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    world.apply_command(
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet,
        },
        &mut events,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, WorldEvent::Close { .. })));
    events
}

fn attack(world: &mut ServerWorld, id: i32, tick: i32) -> Vec<WorldEvent> {
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.tick_count = tick;
    player.attack_strength_ticker = 100;
    dispatch(
        world,
        PlayServerbound::Attack(AttackEntity { entity_id: id }),
    )
}

#[test]
fn a_mob_hit_exposes_live_knockback_in_command_loot_nbt_and_save() {
    let mut world = fixture();
    let (entity_id, uuid) = cow(&mut world);
    let body = world.stored_entity_for_test(DIM, entity_id).unwrap();
    let source = crate::commands::CommandDamageSourceEntity::Stored {
        entity_id,
        uuid: Some(uuid),
        position: Some(body.position()),
        entity_resource_id: "minecraft:cow".to_owned(),
        is_mob: true,
        is_living_non_player: true,
        display_name: world.stored_display_name_of(body),
    };
    let health = world.levels[PLAYER].damage.health;
    assert!(world.hurt_connected_player(
        PLAYER,
        "minecraft:mob_attack",
        1.0,
        Some(source.clone()),
        Some(source),
        Some([TARGET.x, TARGET.y, TARGET.z]),
        &mut Vec::new(),
    ));
    let player = world.levels.player_at(PLAYER);
    assert!(player.damage.health < health);
    let motion = player.delta_movement();
    assert_ne!(motion, Vec3d::default());
    let expected = NbtSummaryValue::List(
        [motion.x, motion.y, motion.z]
            .into_iter()
            .map(|value| NbtSummaryValue::Double(value.to_bits()))
            .collect(),
    );
    for data in [
        world
            .command_player_data(player, &world.item_stack_resolver())
            .unwrap(),
        world.player_loot_nbt(player.profile.uuid).unwrap(),
    ] {
        let NbtSummaryValue::Compound(entries) = data else {
            panic!("player NBT is a compound");
        };
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.name == "Motion")
                .map(|entry| &entry.value),
            Some(&expected)
        );
    }
    assert_eq!(
        world.player_save_patch(player).unwrap().motion,
        [motion.x, motion.y, motion.z]
    );
}

fn cow(world: &mut ServerWorld) -> (i32, Uuid) {
    world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_COW_ID,
        TARGET,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
    )
}

fn remember_cow(world: &mut ServerWorld) -> (i32, Uuid) {
    let (id, uuid) = cow(world);
    let before = world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .health();
    attack(world, id, 73);
    assert!(
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < before
    );
    assert_eq!(memory(world), (Some(uuid), 73));
    (id, uuid)
}

#[test]
fn target_session_memory_tests_normal_landed_hurt_records_removed_living_and_clears_for_nonliving()
{
    let mut world = fixture();
    remember_cow(&mut world);
    assert!(world.set_player_game_mode(PLAYER, GameType::Creative, &mut Vec::new()));
    let (stand, stand_uuid) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        TARGET,
        EntityData::ArmorStand(ArmorStandData::new().unwrap()),
    );
    attack(&mut world, stand, 81);
    assert!(world.stored_entity_for_test(DIM, stand).is_none());
    assert_eq!(memory(&world), (Some(stand_uuid), 81));
    let (cart, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        TARGET,
        EntityData::Minecart(MinecartData::new()),
    );
    attack(&mut world, cart, 82);
    assert!(world.stored_entity_for_test(DIM, cart).is_none());
    assert_eq!(memory(&world), (None, 82));
}

#[test]
fn target_session_memory_tests_real_heart_absorption_records_target_but_cooldown_refusal_does_not()
{
    let mut world = fixture();
    world.set_game_time_for_test(13_000);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13_000),
        &mut Vec::new(),
    );
    let heart = BlockPos { x: 0, y: 64, z: 1 };
    let props = |name: &str, values: &[(&str, &str)]| {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            name,
            &values
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        )
        .unwrap()
    };
    for y in [63, 65] {
        assert!(world.set_block(
            DIM,
            BlockPos { y, ..heart },
            props("minecraft:pale_oak_log", &[("axis", "y")]),
            SET_BLOCK_UPDATE_ALL
        ));
    }
    assert!(world.set_block(
        DIM,
        heart,
        props(
            "minecraft:creaking_heart",
            &[
                ("axis", "y"),
                ("creaking_heart_state", "dormant"),
                ("natural", "false")
            ]
        ),
        SET_BLOCK_UPDATE_ALL
    ));
    world.tick(Vec::new());
    world.tick(Vec::new());
    let bound = world
        .level_block_entity_for_test(DIM, heart)
        .unwrap()
        .creaking_heart()
        .unwrap()
        .protector_uuid()
        .unwrap();
    let id = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            let body = world.stored_entity_for_test(DIM, *id).unwrap();
            body.entity_type_id() == VANILLA_ENTITY_TYPE_CREAKING_ID && body.uuid() == bound
        })
        .unwrap();
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(body.creaking_home_pos(), Some(heart));
    let target = body.position();
    let health = body.living().unwrap().health();
    world.set_player_position_for_test(
        PLAYER,
        Vec3d {
            x: target.x + 1.5,
            ..target
        },
    );
    let before_events = world.game_events(DIM).0.len();
    attack(&mut world, id, 101);
    assert_eq!(
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        health
    );
    assert_eq!(memory(&world), (Some(bound), 101));
    assert!(world
        .game_events(DIM)
        .0
        .iter()
        .skip(before_events)
        .any(|event| {
            event.game_event == VanillaGameEvent::ENTITY_ACTION
                && event.source_entity_id == Some(id)
        }));
    attack(&mut world, id, 102);
    assert_eq!(memory(&world), (Some(bound), 101));
    assert_eq!(
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        health
    );
}

#[test]
fn target_session_memory_tests_actual_death_respawn_clears_alive_target_and_resets_stamp() {
    // PlayerList.java:390-394; ServerPlayer.java:1586-1625: a new body does not restore living hurt memories.
    let mut world = fixture();
    let (id, uuid) = remember_cow(&mut world);
    let entity_id = world.levels[PLAYER].entity_id();
    let source = world.stored_entity_for_test(DIM, id).unwrap();
    let causing = crate::commands::CommandDamageSourceEntity::Stored {
        entity_id: id,
        uuid: Some(source.uuid()),
        position: Some(source.position()),
        entity_resource_id: "minecraft:cow".to_owned(),
        is_mob: true,
        is_living_non_player: source.living().is_some(),
        display_name: world.stored_display_name_of(source),
    };
    world.levels[PLAYER].tick_count = 74;
    assert!(world.hurt_connected_player(
        PLAYER,
        "minecraft:generic",
        1.0,
        Some(causing),
        None,
        None,
        &mut Vec::new()
    ));
    assert_eq!(world.levels[PLAYER].cold.last_hurt_by_mob, Some(uuid));
    assert_eq!(world.levels[PLAYER].cold.last_hurt_by_mob_timestamp, 74);
    dispatch(
        &mut world,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert_eq!(memory(&world), (Some(uuid), 73));
    assert_eq!(world.levels[PLAYER].cold.last_hurt_by_mob, Some(uuid));
    assert_eq!(world.levels[PLAYER].cold.last_hurt_by_mob_timestamp, 74);
    assert!(world.hurt_connected_player(
        PLAYER,
        "minecraft:generic",
        100.0,
        None,
        None,
        None,
        &mut Vec::new()
    ));
    assert!(world.levels[PLAYER].damage.is_dead_or_dying());
    assert_eq!(world.levels[PLAYER].cold.last_hurt_by_mob, Some(uuid));
    assert_eq!(world.levels[PLAYER].cold.last_hurt_by_mob_timestamp, 74);
    let events = dispatch(
        &mut world,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Respawn(_),
            ..
        }
    )));
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.entity_id(), entity_id);
    assert_eq!(player.tick_count, 0);
    assert!(!player.damage.is_dead_or_dying());
    assert!(!player.avatar_removed);
    assert_eq!(memory(&world), (None, 0));
    assert_eq!(player.cold.last_hurt_by_mob, None);
    assert_eq!(player.cold.last_hurt_by_mob_timestamp, 0);
    assert!(
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .health()
            > 0.0
    );
}

#[test]
fn target_session_memory_tests_session_attack_without_hurt_does_not_record_target() {
    let mut world = fixture_with_players(true);
    world.set_player_position_for_test(PLAYER, SOURCE);
    world.set_player_position_for_test(ConnectionId(2), TARGET);
    remember_cow(&mut world);
    world.set_game_rule_for_test(
        "minecraft:pvp",
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
    );
    let expected = memory(&world);
    let target_id = world.levels[ConnectionId(2)].entity_id();
    let health = world.levels[ConnectionId(2)].damage.health;
    attack(&mut world, target_id, 95);
    assert_eq!(world.levels[ConnectionId(2)].damage.health, health);
    assert_eq!(memory(&world), expected);
}

#[test]
fn target_session_memory_tests_real_dimension_replacement_clears_raw_target_but_keeps_attacker_reference(
) {
    let mut world = fixture();
    let (id, uuid) = remember_cow(&mut world);
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    let causing = crate::commands::CommandDamageSourceEntity::Stored {
        entity_id: id,
        uuid: Some(uuid),
        position: Some(body.position()),
        entity_resource_id: "minecraft:cow".to_owned(),
        is_mob: true,
        is_living_non_player: true,
        display_name: world.stored_display_name_of(body),
    };
    assert!(world.hurt_connected_player(
        PLAYER,
        "minecraft:generic",
        1.0,
        Some(causing),
        None,
        None,
        &mut Vec::new()
    ));
    let stamp = world.levels[PLAYER].cold.last_hurt_by_mob_timestamp;
    world.ensure_entity_transfer_full_chunk_for_test(Java26BuiltinDimension::Nether, (0, 0));
    let outcome = world
        .transfer_stored_entity_tree_cross_level(
            DIM,
            Java26BuiltinDimension::Nether,
            id,
            crate::entity::StoredEntityTransition {
                position: TARGET,
                delta_movement: Vec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
        )
        .unwrap();
    assert_ne!(outcome.new_root_id, id);
    assert!(world.stored_entity_for_test(DIM, id).is_none());
    let replacement = world
        .stored_entity_for_test(Java26BuiltinDimension::Nether, outcome.new_root_id)
        .unwrap();
    assert_eq!(replacement.uuid(), uuid);
    assert!(replacement.living().unwrap().health() > 0.0);
    world.tick_players(&mut Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert!(player.body_tick_enabled);
    assert_eq!(memory(&world), (None, 73));
    assert_eq!(player.cold.last_hurt_by_mob, Some(uuid));
    assert_eq!(player.cold.last_hurt_by_mob_timestamp, stamp);
}

#[test]
fn a_wind_charge_hit_records_the_target_in_its_player_owner_memory() {
    let mut world = fixture();
    let (_, cow_uuid) = cow(&mut world);
    let owner = world.levels.player_at(PLAYER).uuid();
    let (charge, _) = world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        Vec3d {
            z: 7.2,
            y: 65.5,
            ..TARGET
        },
        EntityData::WindCharge(
            crate::entity::hurting_projectile::wind_charge::WindChargeData::new(
                crate::entity::hurting_projectile::HurtingProjectileState::new(Some(owner), 0.0),
                0,
            ),
        ),
    );
    world
        .stored_entity_mut_for_test(DIM, charge)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
    for _ in 0..3 {
        world.tick(Vec::new());
    }
    assert!(world.stored_entity_for_test(DIM, charge).is_none());
    assert_eq!(memory(&world).0, Some(cow_uuid));
}

#[test]
fn breaking_a_chest_minecart_angers_a_piglin_that_sees_the_player() {
    use crate::entity::ai::{MemoryModuleType, MemoryValue};
    let mut world = fixture();
    let player = world.levels.player_at(PLAYER).uuid();
    // A gold helmet keeps the piglin idle until the cart breaks.
    world.stock_menu_slot_for_test(
        PLAYER,
        5,
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_GOLDEN_HELMET_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let (piglin, _) = world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID,
        Vec3d {
            x: 10.5,
            z: 9.5,
            ..SOURCE
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID)
                .unwrap(),
        ),
    );
    world.tick(Vec::new());
    let (cart, _) = world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        TARGET,
        EntityData::ChestMinecart(crate::entity::MinecartContainerData::new_chest()),
    );
    for tick in 0..10 {
        if world.stored_entity_for_test(DIM, cart).is_none() {
            break;
        }
        attack(&mut world, cart, 100 + tick);
    }
    assert!(world.stored_entity_for_test(DIM, cart).is_none());
    world.tick(Vec::new());
    assert!(
        crate::entity::brain_packed(world.stored_entity_for_test(DIM, piglin).unwrap())
            .unwrap()
            .memories
            .iter()
            .any(|memory| memory.memory_type == MemoryModuleType::AngryAt
                && memory.value == MemoryValue::Uuid(player))
    );
}
