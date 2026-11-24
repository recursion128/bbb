use super::*;
use crate::entity::{EntityData, EntityTickOutcome, FallingBlockData};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_ANVIL_ID, VANILLA_BLOCK_GOLD_BLOCK_ID};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
};

const FLOOR: BlockPos = BlockPos { x: 8, y: 63, z: 8 };
const FEET: Vec3d = Vec3d {
    x: 8.5,
    y: 64.0,
    z: 8.5,
};

fn state(block: i32) -> i32 {
    vanilla_block_default_state_id(block).unwrap()
}

fn falling_fixture(level: &mut LevelCtx<'_, '_>) -> (EntityStore, i32, i32) {
    level
        .write_block(FLOOR, state(VANILLA_BLOCK_GOLD_BLOCK_ID))
        .unwrap();
    let mut store = batch_store();
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, FEET)
        .unwrap();
    let start = Vec3d { y: 69.0, ..FEET };
    let falling = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        start,
        EntityData::FallingBlock(
            FallingBlockData::new(state(VANILLA_BLOCK_ANVIL_ID), BlockPos { y: 69, ..FLOOR })
                .with_saved_fields(0, true, false, true, 2.0, 40),
        ),
    );
    (store, falling, cow)
}

fn advance_before_landing(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    for _ in 0..40 {
        let body = store.get(id).unwrap();
        if body.position().y + body.delta_movement().y - 0.04 <= FEET.y {
            assert!(body.fall_distance() > 3.0);
            return;
        }
        assert!(matches!(
            crate::entity::tick_one_entity(store, level, id),
            EntityTickOutcome::Complete
        ));
    }
    panic!("falling block did not reach the last airborne tick");
}

#[test]
fn falling_contact_real_tick_hurts_before_hit_ground_and_places_anvil() {
    with_config(crate::tests::datapack_config(), |level| {
        let (mut store, falling, cow) = falling_fixture(level);
        advance_before_landing(&mut store, level, falling);
        let distance = store.get(falling).unwrap().fall_distance();
        let amount = (((distance - 1.0).ceil() as i32) as f32 * 2.0)
            .floor()
            .min(40.0);
        let health = store.get(cow).unwrap().living().unwrap().health();
        let before = level.event_sink.recent_game_events().count();
        assert!(matches!(
            crate::entity::tick_one_entity(&mut store, level, falling),
            EntityTickOutcome::Complete
        ));
        assert!(store.get(falling).is_none());
        assert_eq!(
            store.get(cow).unwrap().living().unwrap().health(),
            health - amount
        );
        let events: Vec<_> = level
            .event_sink
            .recent_game_events()
            .skip(before)
            .copied()
            .collect();
        let hit = events
            .iter()
            .position(|event| event.game_event == VanillaGameEvent::HIT_GROUND)
            .unwrap();
        assert_eq!(events[hit].source_entity_id, Some(falling));
        let broadcasts: Vec<_> = level.event_sink.drain_level_broadcasts().collect();
        assert!(broadcasts
            .iter()
            .any(|event| format!("{event:?}").contains("1031")));
        assert_eq!(
            bbb_protocol::block_types::vanilla_block_type(
                bbb_protocol::block_state_types::vanilla_block_state_block_type_id(
                    level.block_state(BlockPos { y: 64, ..FLOOR })
                )
                .unwrap()
            )
            .unwrap()
            .get_state_for_placement_override,
            Some(bbb_protocol::block_types::VanillaGetStateForPlacementOverride::AnvilBlock)
        );
    });
}

#[test]
fn falling_anvil_landing_hurts_player_and_cow_after_source_removal() {
    use crate::connection::{ClientboundPlayPacket, ConnectionId};
    use crate::tests::{join_loaded, mailbox};
    use crate::world::WorldEvent;

    let sources = crate::world::default_level_sources()
        .unwrap()
        .into_iter()
        .map(|(key, source)| (key, crate::world::ServerLevelSource::Flat(source)))
        .collect();
    let types = crate::world::default_level_dimension_types().unwrap();
    let mut world = ServerWorld::with_level_sources(
        crate::world::ServerWorldConfig {
            game_type: bbb_protocol::game_types::GameType::Survival,
            ..crate::tests::datapack_config()
        },
        &sources,
        &types,
        crate::world::CREATED_WORLD_SEED,
        false,
    )
    .unwrap();
    world.finish_debug_dimension_initialization(false).unwrap();
    world.refresh_all_sky_darkening();
    world.set_test_respawn_radius(0);
    let dimension = Java26BuiltinDimension::Overworld;
    let player = ConnectionId(1);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.ensure_entity_transfer_full_chunk_for_test(dimension, (0, 0));
    assert!(world.set_block(
        dimension,
        FLOOR,
        state(VANILLA_BLOCK_GOLD_BLOCK_ID),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_player_position_for_test(player, Vec3d { x: 8.05, ..FEET });
    world.set_player_health_for_test(player, 4.0);
    let player_id = world.levels[player].entity_id();

    let store = &mut world.level_mut(dimension).entities;
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d { x: 8.95, ..FEET })
        .unwrap();
    let bbb_protocol::NbtSummaryValue::Compound(mut cow_data) = store.command_data(cow).unwrap()
    else {
        unreachable!();
    };
    cow_data.retain(|entry| entry.name != "NoAI");
    cow_data.push(bbb_protocol::NbtSummaryEntry {
        name: "NoAI".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(1),
    });
    store.load_command_data(cow, &cow_data).unwrap();
    let cow_health = store.get(cow).unwrap().living().unwrap().health();
    let falling = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d { y: 69.0, ..FEET },
        EntityData::FallingBlock(
            FallingBlockData::new(state(VANILLA_BLOCK_ANVIL_ID), BlockPos { y: 69, ..FLOOR })
                .with_saved_fields(1, true, false, true, 2.0, 40),
        ),
    );
    let bbb_protocol::NbtSummaryValue::Compound(mut falling_data) =
        store.command_data(falling).unwrap()
    else {
        unreachable!();
    };
    falling_data.push(bbb_protocol::NbtSummaryEntry {
        name: "CustomName".into(),
        value: bbb_protocol::component_literal_nbt("Landing Anvil"),
    });
    store.load_command_data(falling, &falling_data).unwrap();
    let mut events = Vec::new();
    for _ in 0..40 {
        events.extend(world.tick(receiver.drain()).events);
        if world.level(dimension).entities.get(falling).is_none() {
            break;
        }
    }
    assert!(world.level(dimension).entities.get(falling).is_none());
    assert!(
        world
            .level(dimension)
            .entities
            .get(cow)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < cow_health
    );
    assert_eq!(world.player_health_for_test(player), 0.0);
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::DamageEvent(damage),
        } if *connection_id == player
            && damage.entity_id == player_id
            && damage.source_cause_id == falling
            && damage.source_direct_id == falling
    )));
    let message = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerCombatKill(kill),
            } if *connection_id == player => Some(kill.message.text()),
            _ => None,
        })
        .expect("the falling anvil kills the survival player");
    assert!(message.contains("death.attack.anvil"), "{message}");
    assert!(
        message.contains("alice") && message.contains("Landing Anvil"),
        "{message}"
    );
}

#[test]
fn falling_contact_breaks_frogspawn_without_drops_and_settles_the_block() {
    with_config(crate::tests::datapack_config(), |level| {
        let water = state(bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID);
        let frogspawn = state(bbb_protocol::block_types::VANILLA_BLOCK_FROGSPAWN_ID);
        let sand = state(bbb_protocol::block_types::VANILLA_BLOCK_SAND_ID);
        let spawn = BlockPos { y: 65, ..FLOOR };
        level
            .write_block(FLOOR, state(VANILLA_BLOCK_GOLD_BLOCK_ID))
            .unwrap();
        level
            .write_block(BlockPos { y: 64, ..FLOOR }, water)
            .unwrap();
        level.write_block(spawn, frogspawn).unwrap();
        let mut store = batch_store();
        let falling = store.spawn(
            VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            Vec3d { y: 68.0, ..FEET },
            EntityData::FallingBlock(
                FallingBlockData::new(sand, BlockPos { y: 68, ..FLOOR })
                    .with_saved_fields(1, true, false, false, 0.0, 0),
            ),
        );
        for _ in 0..40 {
            crate::entity::tick_one_entity(&mut store, level, falling);
            if level.block_state(spawn) != frogspawn {
                break;
            }
        }
        assert_eq!(
            level.block_state(spawn),
            state(bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID)
        );
        assert!(store.get(falling).unwrap().position().y > 64.0);
        assert_eq!(level.block_state(BlockPos { y: 64, ..FLOOR }), water);
        assert!(level.item_spawns.items.is_empty());
        for _ in 0..40 {
            crate::entity::tick_one_entity(&mut store, level, falling);
            if store.get(falling).is_none() {
                break;
            }
        }
        assert!(store.get(falling).is_none());
        assert_eq!(level.block_state(BlockPos { y: 64, ..FLOOR }), sand);
        assert!(level.item_spawns.items.is_empty());
    });
}

#[test]
fn living_contact_keeps_frogspawn_intact() {
    with_config(crate::tests::datapack_config(), |level| {
        let frogspawn = state(bbb_protocol::block_types::VANILLA_BLOCK_FROGSPAWN_ID);
        let spawn = BlockPos { y: 65, ..FLOOR };
        level
            .write_block(
                BlockPos { y: 64, ..FLOOR },
                state(bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID),
            )
            .unwrap();
        level.write_block(spawn, frogspawn).unwrap();
        let mut store = batch_store();
        let cow = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d { y: 65.0, ..FEET })
            .unwrap();
        crate::entity::tick_one_entity(&mut store, level, cow);
        assert!(store.get(cow).unwrap().is_alive());
        assert_eq!(level.block_state(spawn), frogspawn);
        assert!(level.item_spawns.items.is_empty());
    });
}
