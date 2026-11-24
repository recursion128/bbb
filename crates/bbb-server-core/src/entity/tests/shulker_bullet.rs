use super::native_level::{NativeLevel, DIMENSION};
use super::*;

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_datapack::damage::{DamageEffect, DamageScaling, DamageTypeFacts, DeathMessageType};
use uuid::Uuid;

use crate::entity::damage::{StoredEntityDamageRules, StoredEntityDamageSource};
use crate::entity::hurting_projectile::ProjectileState;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::shulker_bullet::spawn_shulker_bullet;
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::{
    tick_entities, tick_one_entity, EntityData, ShulkerBulletData, ShulkerBulletTarget,
};
use crate::level_event::{LevelEventSink, VanillaGameEvent};
use bbb_server_types::geometry::Axis;

fn position(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}

fn data(store: &EntityStore, id: i32) -> &ShulkerBulletData {
    match store.get(id).expect("the bullet is live").data() {
        EntityData::ShulkerBullet(data) => data,
        other => panic!("expected shulker bullet, found {other:?}"),
    }
}

fn restore_bullet(
    store: &mut EntityStore,
    uuid: Uuid,
    position: Vec3d,
    data: ShulkerBulletData,
    movement: Vec3d,
) -> i32 {
    store
        .restore_initialized(
            uuid,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            position,
            EntityData::ShulkerBullet(data),
            |entity| entity.set_delta_movement(movement),
        )
        .unwrap()
}

#[test]
fn shulker_bullet_spawns_at_the_owner_center() {
    let mut first = EntityStore::new(71, EntityIdCounter::new());
    let owner = first
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position(1.0, 65.0, 3.0))
        .unwrap();
    let target = first
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position(8.0, 65.0, 5.0))
        .unwrap();
    let target_entity = first.get(target).unwrap();
    let target = ShulkerBulletTarget {
        uuid: target_entity.uuid(),
        position: target_entity.position(),
        height: target_entity.bb_height(),
    };
    let expected_center = first.get(owner).unwrap().bounding_box().center();
    let bullet = {
        let mut native = NativeLevel::new();
        native.with_level(|level| spawn_shulker_bullet(&mut first, level, owner, target, None))
    }
    .unwrap();
    assert_eq!(first.get(bullet).unwrap().position(), expected_center);
    assert!(first.get(bullet).unwrap().no_physics());
    assert!((10..=50)
        .step_by(10)
        .any(|steps| steps == data(&first, bullet).flight_steps()));
}

#[test]
fn shulker_bullet_obstacle_reselects_away_from_the_current_axis() {
    let mut store = EntityStore::new(72, EntityIdCounter::new());
    let target = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position(0.5, 64.0, 8.5))
        .unwrap();
    let target_uuid = store.get(target).unwrap().uuid();
    let bullet = restore_bullet(
        &mut store,
        Uuid::from_u128(72),
        position(0.5, 64.0, 0.5),
        ShulkerBulletData::from_saved(
            ProjectileState::default(),
            Some(target_uuid),
            Some(Direction::East),
            5,
            position(0.15, 0.0, 0.0),
        ),
        position(0.0, 0.0, 0.0),
    );
    let mut level = NativeLevel::new();
    level.set(
        bbb_protocol::packets::BlockPos { x: 1, y: 64, z: 0 },
        state_id("minecraft:stone"),
    );
    let mut level = level;
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(
        data(&store, bullet).current_direction(),
        Some(Direction::South)
    );
    assert_eq!(
        crate::entity::shulker_bullet::direction_axis(
            data(&store, bullet).current_direction().unwrap()
        ),
        Axis::Z
    );
}

#[test]
fn shulker_bullet_save_keeps_an_unresolved_target_until_the_first_tick() {
    let mut store = EntityStore::new(73, EntityIdCounter::new());
    let target = Uuid::from_u128(0x1234_5678);
    let bullet = restore_bullet(
        &mut store,
        Uuid::from_u128(73),
        position(0.5, 10.0, 0.5),
        ShulkerBulletData::from_saved(
            ProjectileState::from_saved(Some(Uuid::from_u128(7)), true, true),
            Some(target),
            Some(Direction::North),
            30,
            position(0.1, -0.2, 0.3),
        ),
        position(0.0, 0.0, 0.0),
    );
    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut reloaded = EntityStore::new(74, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).unwrap();
    assert!(store.get(bullet).unwrap().no_physics());
    assert!(reloaded.get(ids[0]).unwrap().no_physics());
    let restored = data(&reloaded, ids[0]);
    assert_eq!(restored.target(), Some(target));
    assert_eq!(restored.current_direction(), Some(Direction::North));
    assert_eq!(restored.flight_steps(), 30);
    assert_eq!(restored.target_delta(), position(0.1, -0.2, 0.3));
    assert_eq!(
        store.get(bullet).unwrap().uuid(),
        reloaded.get(ids[0]).unwrap().uuid()
    );

    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut reloaded, level, true));
    assert_eq!(data(&reloaded, ids[0]).target(), None);
    assert_eq!(reloaded.get(ids[0]).unwrap().delta_movement().y, -0.04);
}

#[test]
fn shulker_bullet_living_hit_hurts_applies_levitation_and_destroys() {
    let mut level = NativeLevel::new();
    let mut cow = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap();
    cow.set_no_ai(true);
    let (target, target_uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_COW_ID,
        position(1.0, 64.0, 0.5),
        EntityData::GeneralLiving(cow),
    );
    let before = level
        .world
        .stored_entity_for_test(DIMENSION, target)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let bullet = level
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            position(0.0, 64.0, 0.5),
            EntityData::ShulkerBullet(ShulkerBulletData::from_saved(
                ProjectileState::from_saved(None, true, true),
                Some(target_uuid),
                None,
                0,
                position(1.0, 0.0, 0.0),
            )),
        )
        .0;
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, bullet)
        .unwrap()
        .set_delta_movement(position(1.0, 0.0, 0.0));
    level.world.tick(Vec::new());
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, bullet)
        .is_none());
    let living = level
        .world
        .stored_entity_for_test(DIMENSION, target)
        .unwrap()
        .living()
        .unwrap();
    assert_eq!(living.health(), before - 4.0);
    let effect = living
        .active_effect("minecraft:levitation")
        .expect("the actual hit applies levitation");
    assert_eq!(effect.amplifier(), 0);
    assert!((1..=200).contains(&effect.duration()));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::PROJECTILE_LAND
                && event.recorded.source_entity_id == Some(bullet)
        ));
}

#[test]
fn shulker_bullet_block_hit_and_hurt_destroy_emit_owned_effects() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
    level
        .world
        .set_player_position_for_test(crate::world::ConnectionId(1), position(4.5, 64.0, 4.5));
    let hit = BlockPos { x: 1, y: 64, z: 0 };
    level.set(hit, state_id("minecraft:stone"));
    let bullet = level
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            position(0.5, 64.0, 0.5),
            EntityData::ShulkerBullet(ShulkerBulletData::from_saved(
                ProjectileState::from_saved(None, true, true),
                None,
                None,
                0,
                position(0.0, 0.0, 0.0),
            )),
        )
        .0;
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, bullet)
        .unwrap()
        .set_delta_movement(position(1.0, 0.0, 0.0));
    let output = level.world.tick(receiver.drain());
    assert!(output.events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: crate::connection::ClientboundPlayPacket::Sound(_),
            ..
        }
    )));
    assert!(output.events.iter().any(|event| matches!(event, crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::LevelParticles(particles), .. } if particles.count == 2 && particles.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID)));
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, bullet)
        .is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::PROJECTILE_LAND
                && event.recorded.source_entity_id == Some(bullet)
        ));

    let mut hurt_store = EntityStore::new(77, EntityIdCounter::new());
    let hurt = restore_bullet(
        &mut hurt_store,
        Uuid::from_u128(77),
        position(0.5, 64.0, 0.5),
        ShulkerBulletData::from_saved(
            ProjectileState::default(),
            None,
            None,
            0,
            position(0.0, 0.0, 0.0),
        ),
        position(0.0, 0.0, 0.0),
    );
    hurt_store.drain_lifecycle_events();
    let mut sink = LevelEventSink::default();
    let damage = damage_type();
    let outcome = hurt_store
        .hurt_server(
            hurt,
            StoredEntityDamageSource {
                damage_type: &damage,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            1.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut sink,
        )
        .unwrap()
        .unwrap();
    assert!(outcome.succeeded);
    assert!(hurt_store.get(hurt).is_none());
    assert!(hurt_store
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    let event = sink.recent_game_events().next().unwrap();
    assert_eq!(event.game_event, VanillaGameEvent::ENTITY_DAMAGE);
    assert_eq!(event.source_entity_id, Some(hurt));
}

#[test]
fn shulker_bullet_despawns_in_peaceful() {
    let data = || {
        ShulkerBulletData::from_saved(
            ProjectileState::default(),
            None,
            None,
            0,
            position(0.0, 0.0, 0.0),
        )
    };
    let mut peaceful = EntityStore::new(78, EntityIdCounter::new());
    let bullet = restore_bullet(
        &mut peaceful,
        Uuid::from_u128(78),
        position(0.5, 64.0, 0.5),
        data(),
        position(0.0, 0.0, 0.0),
    );
    peaceful.set_difficulty(Difficulty::Peaceful);
    peaceful.drain_lifecycle_events();
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut peaceful, level, true))
    };
    assert!(peaceful.get(bullet).is_none());
    assert!(peaceful
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(
            event,
            EntityLifecycleEvent::Removed { entity, .. }
                if entity.id() == bullet
        )));
}

fn damage_type() -> DamageTypeFacts {
    DamageTypeFacts {
        resource_id: "minecraft:generic".to_string(),
        registry_id: 0,
        message_id: "generic".into(),
        scaling: DamageScaling::Never,
        exhaustion: 0.0,
        effects: DamageEffect::Hurt,
        death_message_type: DeathMessageType::Default,
        bypasses_invulnerability: false,
        bypasses_cooldown: false,
        is_drowning: false,
        is_fall: false,
        is_fire: false,
        is_freezing: false,
        is_explosion: false,
        ignites_armor_stands: false,
        burns_armor_stands: false,
        can_break_armor_stand: false,
        always_kills_armor_stands: false,
        always_most_significant_fall: false,
    }
}

#[test]
fn shulker_bullet_silent_hurt_keeps_particles_discard_and_damage_event() {
    let mut store = EntityStore::new(77, EntityIdCounter::new());
    let id = restore_bullet(
        &mut store,
        Uuid::from_u128(770),
        position(0.5, 64.0, 0.5),
        ShulkerBulletData::from_saved(ProjectileState::default(), None, None, 0, Vec3d::default()),
        Vec3d::default(),
    );
    store.get_mut(id).unwrap().set_silent(true);
    store.drain_lifecycle_events();
    let damage = damage_type();
    let mut sink = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            id,
            StoredEntityDamageSource {
                damage_type: &damage,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            0.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut sink,
        )
        .unwrap()
        .unwrap();
    assert!(outcome.succeeded);
    assert!(store.get(id).is_none());
    let events = store.drain_lifecycle_events();
    assert!(!events
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    let particles = events
        .iter()
        .find_map(|event| match event {
            EntityLifecycleEvent::Particles { particles } => Some(particles),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        particles.particle.particle_type_id,
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_CRIT_ID
    );
    assert_eq!(particles.count, 15);
    assert_eq!(particles.offset, position(0.2, 0.2, 0.2));
    assert_eq!(particles.max_speed, 0.0);
    assert!(sink
        .recent_game_events()
        .any(|event| event.game_event == VanillaGameEvent::ENTITY_DAMAGE
            && event.source_entity_id == Some(id)));
}

#[test]
fn shulker_bullet_silent_block_hit_keeps_explosion_and_destroy() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
    level
        .world
        .set_player_position_for_test(crate::world::ConnectionId(1), position(4.5, 64.0, 4.5));
    level.set(BlockPos { x: 1, y: 64, z: 0 }, state_id("minecraft:stone"));
    let bullet = level
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            position(0.5, 64.0, 0.5),
            EntityData::ShulkerBullet(ShulkerBulletData::from_saved(
                ProjectileState::from_saved(None, true, true),
                None,
                None,
                0,
                Vec3d::default(),
            )),
        )
        .0;
    let entity = level
        .world
        .stored_entity_mut_for_test(DIMENSION, bullet)
        .unwrap();
    entity.set_delta_movement(position(1.0, 0.0, 0.0));
    entity.set_silent(true);
    let result = level.world.tick(receiver.drain());
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, bullet)
        .is_none());
    assert!(!result.events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: crate::connection::ClientboundPlayPacket::Sound(_),
            ..
        }
    )));
    assert!(result.events.iter().any(|event| matches!(event, crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::LevelParticles(particles), .. } if particles.count == 2 && particles.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID)));
}

mod player_pose;

#[test]
fn shulker_bullet_turns_only_at_a_block_with_a_full_top_face() {
    let steer = |ahead: i32| {
        let mut store = EntityStore::new(72, EntityIdCounter::new());
        let target = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position(8.5, 64.0, 4.5))
            .unwrap();
        let target_uuid = store.get(target).unwrap().uuid();
        let bullet = restore_bullet(
            &mut store,
            Uuid::from_u128(72),
            position(0.5, 64.0, 0.5),
            ShulkerBulletData::from_saved(
                ProjectileState::default(),
                Some(target_uuid),
                Some(Direction::East),
                5,
                position(0.15, 0.0, 0.0),
            ),
            position(0.0, 0.0, 0.0),
        );
        let mut level = NativeLevel::new();
        level.set(BlockPos { x: 1, y: 64, z: 0 }, ahead);
        level.with_level(|level| tick_entities(&mut store, level, true));
        data(&store, bullet).current_direction()
    };
    let slab = super::state_with(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "false")],
    );
    assert_eq!(steer(slab), Some(Direction::East));
    assert_ne!(steer(state_id("minecraft:stone")), Some(Direction::East));
}
