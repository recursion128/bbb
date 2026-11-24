use super::*;

use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::fluid::block_state_is_air;
use std::sync::Arc;

fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn nest_fixture(occupants: &[&str], blocked: bool) -> (ServerWorld, BlockPos, i32) {
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let pos = BlockPos { x: 6, y: 300, z: 6 };
    let state = trigger_state(
        "minecraft:bee_nest",
        &[("facing", "east"), ("honey_level", "0")],
    );
    assert!(world.set_block(DIM, pos, state, SET_BLOCK_UPDATE_ALL));
    let entries = occupants
        .iter()
        .map(|id| {
            NbtSummaryValue::Compound(vec![
                field(
                    "entity_data",
                    NbtSummaryValue::Compound(vec![field(
                        "id",
                        NbtSummaryValue::String((*id).into()),
                    )]),
                ),
                field("ticks_in_hive", NbtSummaryValue::Int(37)),
                field("min_ticks_in_hive", NbtSummaryValue::Int(600)),
            ])
        })
        .collect();
    world
        .level_block_entity_for_test(DIM, pos)
        .and_then(crate::block_entity::BlockEntity::beehive_mut)
        .unwrap()
        .load_additional(&[field("bees", NbtSummaryValue::List(entries))]);
    if blocked {
        assert!(world.set_block(
            DIM,
            BlockPos {
                x: pos.x + 1,
                ..pos
            },
            block_state_id("minecraft:obsidian"),
            SET_BLOCK_UPDATE_ALL
        ));
    }
    (world, pos, state)
}

fn direct_source(pos: BlockPos, resource_id: &str) -> ExplosionSource {
    ExplosionSource {
        direct: ExplosionSourceEntity {
            custom_name: None,
            entity_id: 73,
            entity_resource_id: resource_id.into(),
            uuid: Uuid::from_u128(73),
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            kind: ExplosionSourceEntityKind::Other,
        },
        indirect: None,
        causing: None,
        in_water: false,
        wind_charge: false,
    }
}

fn bees(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_BEE_ID)
        })
        .collect()
}

#[test]
fn direct_explosives_release_ordered_occupants_before_base_removal() {
    for direct in [
        "minecraft:tnt",
        "minecraft:creeper",
        "minecraft:wither_skull",
        "minecraft:wither",
        "minecraft:tnt_minecart",
    ] {
        let (mut world, pos, _) = nest_fixture(&["minecraft:bee", "minecraft:bee"], false);
        heart_explode(
            &mut world,
            pos,
            Some(direct_source(pos, direct)),
            ExplosionInteraction::Block,
        );
        let released = bees(&world);
        assert_eq!(released.len(), 2, "{direct}");
        assert!(block_state_is_air(world.block_state(DIM, pos).unwrap()));
        assert!(released
            .iter()
            .all(|id| world.stored_entity_for_test(DIM, *id).unwrap().no_gravity()));
        let records = world.game_events(DIM).0;
        for id in released {
            assert!(records
                .iter()
                .any(|record| record.game_event == VanillaGameEvent::BLOCK_CHANGE
                    && record.source_entity_id == Some(id)));
        }
    }
}

#[test]
fn indirect_or_causing_explosive_and_keep_or_trigger_do_not_release() {
    for interaction in [ExplosionInteraction::None, ExplosionInteraction::Trigger] {
        let (mut world, pos, _) = nest_fixture(&["minecraft:bee"], false);
        heart_explode(
            &mut world,
            pos,
            Some(direct_source(pos, "minecraft:tnt")),
            interaction,
        );
        assert!(bees(&world).is_empty());
    }
    for direct in ["minecraft:player", "minecraft:fireball"] {
        let (mut world, pos, _) = nest_fixture(&["minecraft:bee"], false);
        let mut source = direct_source(pos, direct);
        source.causing = Some(direct_source(pos, "minecraft:tnt").direct);
        source.indirect = source.causing.clone();
        heart_explode(&mut world, pos, Some(source), ExplosionInteraction::Block);
        assert!(bees(&world).is_empty());
    }
}

#[test]
fn blocked_exit_still_releases_at_hive_center() {
    let (mut world, pos, _) = nest_fixture(&["minecraft:bee"], true);
    heart_explode(
        &mut world,
        pos,
        Some(direct_source(pos, "minecraft:tnt")),
        ExplosionInteraction::Block,
    );
    let id = bees(&world)[0];
    let bee = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(bee.position().x, f64::from(pos.x) + 0.5);
}

#[test]
fn datapack_tag_accepts_non_bee_without_bee_exit_snap() {
    let (mut world, pos, _) = nest_fixture(&["minecraft:silverfish"], false);
    let snapshot = RegistryTagSnapshot::new([(
        "minecraft:entity_type".to_string(),
        "minecraft:beehive_inhabitors".to_string(),
        vec!["minecraft:silverfish".to_string()],
    )]);
    world.install_level_registry_tags(Arc::new(snapshot));
    heart_explode(
        &mut world,
        pos,
        Some(direct_source(pos, "minecraft:tnt")),
        ExplosionInteraction::Block,
    );
    let silverfish = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find_map(|id| {
            let entity = world.stored_entity_for_test(DIM, id)?;
            (entity.entity_type_id() == VANILLA_ENTITY_TYPE_SILVERFISH_ID).then_some(entity)
        })
        .expect("tagged non-bee released");
    assert!(silverfish.no_gravity());
    assert_eq!(silverfish.position(), Vec3d::default());
}

#[test]
fn adjacent_fire_emergency_releases_saved_bees_without_anger_or_hive_removal() {
    for blocked in [false, true] {
        let (mut world, pos, state) = nest_fixture(&["minecraft:bee", "minecraft:bee"], blocked);
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 16.5,
                y: 300.0,
                z: 6.5,
            },
        );
        let fire = BlockPos {
            x: pos.x - 1,
            ..pos
        };
        assert!(world.set_block(
            DIM,
            BlockPos {
                y: fire.y - 1,
                ..fire
            },
            block_state_id("minecraft:netherrack"),
            SET_BLOCK_UPDATE_ALL,
        ));
        assert!(world.set_block(
            DIM,
            fire,
            trigger_state("minecraft:fire", &[]),
            SET_BLOCK_UPDATE_ALL
        ));
        let events = world.tick(receiver.drain()).events;
        assert_eq!(world.block_state(DIM, pos), Some(state));
        let saved_hive = world.block_entity_save_for_test(DIM, pos).unwrap();
        assert!(saved_hive
            .iter()
            .any(|entry| entry.name == "bees" && entry.value == NbtSummaryValue::List(Vec::new())));
        let released = bees(&world);
        assert_eq!(released.len(), 2);
        for id in &released {
            assert!(events.iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::AddEntity(packet) }
                    if packet.id == *id
            )));
            assert_eq!(world.mob_target_for_test(DIM, *id), None);
            let NbtSummaryValue::Compound(saved) =
                world.stored_entity_command_data_for_test(DIM, *id).unwrap()
            else {
                panic!("bee saved as compound");
            };
            assert!(saved.iter().any(|entry| entry.name == "hive_pos"));
            assert!(!saved.iter().any(|entry| entry.name == "angry_at"));
            assert!(saved
                .iter()
                .any(|entry| entry.name == "anger_end_time"
                    && entry.value == NbtSummaryValue::Long(-1)));
        }
        let request = world
            .save_request(
                1_786_000_000_999,
                200,
                bbb_server_persistence::RegionCompression::Zlib,
            )
            .unwrap();
        let mut candidate = restore_candidate(200, None, Vec::new());
        super::super::entities::restore_entity_writes(&mut candidate, &request.entity_writes);
        let restored = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
        for id in released {
            let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
            assert!(bees(&restored).iter().any(|id| restored
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .uuid()
                == uuid));
        }
    }
}

#[test]
fn soul_fire_and_nonadjacent_fire_leave_beehive_occupants_inside() {
    for (fire, distance, support) in [
        ("minecraft:soul_fire", 1, "minecraft:soul_soil"),
        ("minecraft:fire", 2, "minecraft:netherrack"),
    ] {
        let (mut world, pos, state) = nest_fixture(&["minecraft:bee"], false);
        let (_, receiver) = mailbox();
        let fire_pos = BlockPos {
            x: pos.x - distance,
            ..pos
        };
        assert!(world.set_block(
            DIM,
            BlockPos {
                y: pos.y - 1,
                ..fire_pos
            },
            block_state_id(support),
            SET_BLOCK_UPDATE_ALL
        ));
        assert!(world.set_block(
            DIM,
            fire_pos,
            trigger_state(fire, &[]),
            SET_BLOCK_UPDATE_ALL
        ));
        world.tick(receiver.drain());
        assert_eq!(world.block_state(DIM, pos), Some(state));
        assert!(bees(&world).is_empty());
        let hive = world
            .level_block_entity_for_test(DIM, pos)
            .and_then(crate::block_entity::BlockEntity::beehive_mut)
            .unwrap();
        assert_eq!(hive.bees().len(), 1);
    }
}

fn place_player(world: &mut ServerWorld, connection: u64, x: f64) {
    world.set_player_position_for_test(
        ConnectionId(connection),
        Vec3d {
            x,
            y: 300.0,
            z: 6.5,
        },
    );
}

fn join_survival(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    connection: u64,
    name: &str,
) {
    join_loaded(sender, receiver, world, connection, name, connection.into());
    let mode = bbb_protocol::game_types::GameType::Survival;
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(connection),
        mode,
        bbb_protocol::packets::PlayerAbilities::for_game_type(mode),
    );
}

fn nearby_bee(world: &mut ServerWorld) -> i32 {
    let (bee, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_BEE_ID,
        Vec3d {
            x: 12.5,
            y: 300.0,
            z: 6.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_BEE_ID).unwrap(),
        ),
    );
    bee
}

#[test]
fn hive_explosion_angers_released_and_idle_bees_without_replacing_existing_target() {
    let (mut world, pos, state) = nest_fixture(&["minecraft:bee"], false);
    let (sender, receiver) = mailbox();
    join_survival(&mut world, &sender, &receiver, 1, "first");
    join_survival(&mut world, &sender, &receiver, 2, "second");
    place_player(&mut world, 1, 12.5);
    place_player(&mut world, 2, 30.5);
    let busy = nearby_bee(&mut world);
    trigger_at(&mut world, pos, None, ExplosionInteraction::Trigger);
    assert_eq!(
        world.mob_target_for_test(DIM, busy),
        Some(Uuid::from_u128(1))
    );
    assert_eq!(world.block_state(DIM, pos), Some(state));
    assert_eq!(bees(&world), vec![busy]);
    assert_eq!(
        world
            .level_block_entity_for_test(DIM, pos)
            .unwrap()
            .beehive()
            .unwrap()
            .bees()
            .len(),
        1
    );

    place_player(&mut world, 1, 30.5);
    place_player(&mut world, 2, 12.5);
    let idle = nearby_bee(&mut world);
    trigger_at(
        &mut world,
        pos,
        Some("minecraft:tnt"),
        ExplosionInteraction::Block,
    );
    assert!(block_state_is_air(world.block_state(DIM, pos).unwrap()));
    assert_eq!(bees(&world).len(), 3);
    assert_eq!(
        world.mob_target_for_test(DIM, busy),
        Some(Uuid::from_u128(1))
    );
    for bee in bees(&world).into_iter().filter(|id| *id != busy) {
        assert_eq!(
            world.mob_target_for_test(DIM, bee),
            Some(Uuid::from_u128(2))
        );
    }
    assert!(bees(&world).contains(&idle));
}

#[test]
fn hive_explosion_anger_does_not_require_an_occupant_releasing_source() {
    for direct in [Some("minecraft:fireball"), None] {
        let (mut world, pos, _) = nest_fixture(&["minecraft:bee"], false);
        let (sender, receiver) = mailbox();
        join_survival(&mut world, &sender, &receiver, 1, "observer");
        place_player(&mut world, 1, 12.5);
        let bee = nearby_bee(&mut world);
        trigger_at(&mut world, pos, direct, ExplosionInteraction::Block);
        assert_eq!(bees(&world), vec![bee]);
        assert_eq!(
            world.mob_target_for_test(DIM, bee),
            Some(Uuid::from_u128(1))
        );
    }
}

#[test]
fn hive_explosion_anger_respects_keep_and_player_area_but_not_breeze_griefing_gate() {
    use bbb_protocol::game_types::GameType::{Creative, Spectator, Survival};
    for (interaction, mode, player_x, should_anger) in [
        (ExplosionInteraction::Trigger, Survival, 12.5, true),
        (ExplosionInteraction::None, Survival, 12.5, false),
        (ExplosionInteraction::Trigger, Creative, 12.5, false),
        (ExplosionInteraction::Trigger, Spectator, 12.5, false),
        (ExplosionInteraction::Trigger, Survival, 30.5, false),
    ] {
        let (mut world, pos, state) = nest_fixture(&["minecraft:bee"], false);
        let (sender, receiver) = mailbox();
        join_survival(&mut world, &sender, &receiver, 1, "observer");
        assert!(world.set_game_rule_for_test(
            "minecraft:mob_griefing",
            bbb_protocol::game_rules::GameRuleValue::Boolean(false),
        ));
        world.set_player_mode_and_abilities_for_test(
            ConnectionId(1),
            mode,
            bbb_protocol::packets::PlayerAbilities::for_game_type(mode),
        );
        place_player(&mut world, 1, player_x);
        let bee = nearby_bee(&mut world);
        trigger_at(
            &mut world,
            pos,
            Some("minecraft:breeze_wind_charge"),
            interaction,
        );
        assert_eq!(world.block_state(DIM, pos), Some(state));
        assert_eq!(bees(&world), vec![bee]);
        assert_eq!(
            world.mob_target_for_test(DIM, bee),
            should_anger.then_some(Uuid::from_u128(1)),
            "interaction={interaction:?}, mode={mode:?}, x={player_x}"
        );
        assert_eq!(
            world
                .level_block_entity_for_test(DIM, pos)
                .unwrap()
                .beehive()
                .unwrap()
                .bees()
                .len(),
            1
        );
    }
}
