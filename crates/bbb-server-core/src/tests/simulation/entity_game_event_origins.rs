use super::*;

use crate::block_entity::vibration::VibrationData;
use crate::entity::shulker_bullet::spawn_shulker_bullet;
use crate::entity::{
    EntityData, EntityIdCounter, EntityStore, GeneralLivingData, HurtingProjectileState,
    ShulkerBulletTarget, WindChargeData,
};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
    VANILLA_ENTITY_TYPE_WIND_CHARGE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_server_types::geometry::Vec3Ops;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SENSOR: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const ORIGIN: Vec3d = Vec3d {
    x: -7.1,
    y: 64.5,
    z: 0.5,
};
const ORIGIN_BLOCK: BlockPos = BlockPos { x: -8, y: 64, z: 0 };

fn world_beside_sensor() -> (WorldCommandReceiver, ServerWorld) {
    let (_sender, receiver, mut world) = helpers::joined_stand_world(datapack_config(), ORIGIN);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    assert!(world.set_block(
        DIMENSION,
        SENSOR,
        helpers::sensor_state("inactive", 0, false),
        SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());
    let sensor = world
        .level_block_entity_for_test(DIMENSION, SENSOR)
        .and_then(|entity| entity.sculk_sensor_mut())
        .expect("the sensor's block entity");
    *sensor.vibration_data_mut_for_test() = VibrationData::default();
    (receiver, world)
}

fn event_origins(world: &ServerWorld, source: i32) -> Vec<(VanillaGameEvent, Vec3d, BlockPos)> {
    world
        .positioned_game_events_for_test(DIMENSION)
        .into_iter()
        .filter(|raised| {
            raised.recorded.source_entity_id == Some(source)
                && matches!(
                    raised.recorded.game_event,
                    VanillaGameEvent::ENTITY_DAMAGE | VanillaGameEvent::ENTITY_DIE
                )
        })
        .map(|raised| {
            (
                raised.recorded.game_event,
                raised.position,
                raised.recorded.pos,
            )
        })
        .collect()
}

#[test]
fn lethal_stored_damage_keeps_fractional_origin_through_listener_and_travel() {
    let (receiver, mut world) = world_beside_sensor();
    let zombie = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ORIGIN,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        )
        .0;
    crate::entity::living::set_health(
        world.stored_entity_mut_for_test(DIMENSION, zombie).unwrap(),
        0.1,
    );
    let outcome = world
        .hurt_stored_entity_by_command_for_test(DIMENSION, zombie, "minecraft:generic", 40.0, None)
        .expect("the stored hurt reached its world owner");
    assert!(outcome.applied && outcome.succeeded);
    assert_eq!(
        event_origins(&world, zombie),
        [
            (VanillaGameEvent::ENTITY_DAMAGE, ORIGIN, ORIGIN_BLOCK),
            (VanillaGameEvent::ENTITY_DIE, ORIGIN, ORIGIN_BLOCK),
        ]
    );

    let vibration = world
        .level_block_entity_for_test(DIMENSION, SENSOR)
        .and_then(|entity| entity.sculk_sensor())
        .unwrap()
        .vibration_data()
        .clone();
    let (candidate, _) = vibration
        .selection_strategy()
        .current()
        .expect("the sensor scheduled the damage vibration");
    assert_eq!(candidate.pos, ORIGIN);
    assert!((candidate.distance - 7.6).abs() < 0.01);
    world.tick(receiver.drain());
    assert_eq!(
        world
            .level_block_entity_for_test(DIMENSION, SENSOR)
            .and_then(|entity| entity.sculk_sensor())
            .unwrap()
            .vibration_data()
            .travel_time_in_ticks(),
        6
    );
}

#[test]
fn player_damage_and_death_raise_from_the_session_only_for_actual_damage() {
    let (_receiver, mut world) = world_beside_sensor();
    let player = world.player_entity_id_for_test(ConnectionId(1));
    let before = event_origins(&world, player).len();
    let _ = world.hurt_connected_player_by_command_for_test(
        ConnectionId(1),
        "minecraft:generic",
        0.0,
        None,
        None,
        None,
    );
    assert_eq!(event_origins(&world, player).len(), before);

    let (hurt, _) = world.hurt_connected_player_by_command_for_test(
        ConnectionId(1),
        "minecraft:generic",
        4.0,
        None,
        None,
        None,
    );
    assert!(hurt);
    assert_eq!(
        event_origins(&world, player)[before..],
        [(VanillaGameEvent::ENTITY_DAMAGE, ORIGIN, ORIGIN_BLOCK)]
    );
    let (killed, _) = world.hurt_connected_player_by_command_for_test(
        ConnectionId(1),
        "minecraft:generic_kill",
        40.0,
        None,
        None,
        None,
    );
    assert!(killed);
    assert_eq!(
        event_origins(&world, player)[before..],
        [
            (VanillaGameEvent::ENTITY_DAMAGE, ORIGIN, ORIGIN_BLOCK),
            (VanillaGameEvent::ENTITY_DAMAGE, ORIGIN, ORIGIN_BLOCK),
            (VanillaGameEvent::ENTITY_DIE, ORIGIN, ORIGIN_BLOCK),
        ]
    );
}

#[test]
fn shulker_bullet_hit_destroy_raises_its_events_at_fractional_origins() {
    let (receiver, mut world) = world_beside_sensor();
    let start = Vec3d {
        x: -7.1,
        y: 64.5,
        z: 2.5,
    };
    let movement = Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    let target_position = start.plus(movement);
    let target = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_COW_ID,
        target_position,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
    );
    let mut constructor_store = EntityStore::new(75, EntityIdCounter::new());
    let owner = constructor_store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, start)
        .unwrap();
    let target_height = world
        .stored_entity_for_test(DIMENSION, target.0)
        .unwrap()
        .bb_height();
    let constructed = world
        .with_level_for_test(DIMENSION, |level| {
            spawn_shulker_bullet(
                &mut constructor_store,
                level,
                owner,
                ShulkerBulletTarget {
                    uuid: target.1,
                    position: target_position,
                    height: target_height,
                },
                None,
            )
        })
        .unwrap();
    let constructed = constructor_store.get(constructed).unwrap();
    let start = constructed.position();
    let EntityData::ShulkerBullet(data) = constructed.data() else {
        panic!("the constructor returned a shulker bullet")
    };
    let target_delta = data.target_delta();
    let target_delta = Vec3d {
        x: (target_delta.x * 1.025).clamp(-1.0, 1.0),
        y: (target_delta.y * 1.025).clamp(-1.0, 1.0),
        z: (target_delta.z * 1.025).clamp(-1.0, 1.0),
    };
    let expected = start.plus(movement.plus(target_delta.plus(movement.scale(-1.0)).scale(0.2)));
    let bullet = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            start,
            EntityData::ShulkerBullet(data.clone()),
        )
        .0;
    world
        .stored_entity_mut_for_test(DIMENSION, bullet)
        .unwrap()
        .set_delta_movement(movement);

    world.tick(receiver.drain());

    assert!(world.stored_entity_for_test(DIMENSION, bullet).is_none());
    assert_eq!(
        event_origins(&world, bullet),
        [(
            VanillaGameEvent::ENTITY_DAMAGE,
            expected,
            BlockPos { x: -7, y: 65, z: 2 },
        )]
    );
    let raised = world.positioned_game_events_for_test(DIMENSION);
    let land_index = raised
        .iter()
        .position(|event| {
            event.recorded.game_event == VanillaGameEvent::PROJECTILE_LAND
                && event.recorded.source_entity_id == Some(bullet)
        })
        .expect("the bullet's real cow hit raised land");
    let land = raised[land_index];
    assert_eq!(land.recorded.affected_state, None);
    assert!(land.position.x < expected.x);
    assert_eq!(land.position.z, 2.5);
    assert_ne!(land.position.x, f64::from(land.recorded.pos.x) + 0.5);
}

#[test]
fn wind_charge_block_hit_shoots_at_hit_position_then_lands_at_block_center_with_current_state() {
    let (receiver, mut world) = world_beside_sensor();
    let air = block_state_id("minecraft:air");
    for x in -6..=-5 {
        for y in 64..=65 {
            let _ = world.set_block(
                DIMENSION,
                BlockPos { x, y, z: 2 },
                air,
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    let target = BlockPos { x: -4, y: 64, z: 2 };
    let stone = block_state_id("minecraft:stone");
    assert!(world.set_block(DIMENSION, target, stone, SET_BLOCK_UPDATE_ALL));
    let charge = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
            Vec3d {
                x: -6.1,
                y: 64.5,
                z: 2.5,
            },
            EntityData::WindCharge(WindChargeData::new(
                HurtingProjectileState::new(
                    None,
                    HurtingProjectileState::INITIAL_ACCELERATION_POWER,
                ),
                5,
            )),
        )
        .0;
    world
        .stored_entity_mut_for_test(DIMENSION, charge)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });

    world.tick(receiver.drain());

    let raised = world.positioned_game_events_for_test(DIMENSION);
    let shoot = raised
        .iter()
        .find(|event| event.recorded.game_event == VanillaGameEvent::PROJECTILE_SHOOT)
        .expect("the first tick emitted shoot after moving to the hit");
    let land = raised
        .iter()
        .find(|event| {
            event.recorded.game_event == VanillaGameEvent::PROJECTILE_LAND
                && event.recorded.source_entity_id == Some(charge)
        })
        .expect("the block hit emitted land");
    assert_eq!(
        shoot.position,
        Vec3d {
            x: -4.0,
            y: 64.5,
            z: 2.5
        }
    );
    assert_eq!(
        land.position,
        Vec3d {
            x: -3.5,
            y: 64.5,
            z: 2.5
        }
    );
    assert_eq!(land.recorded.pos, target);
    assert_eq!(land.recorded.affected_state, Some(stone));
}
