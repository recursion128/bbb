use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::world::WorldEvent;

use crate::random::GameRandom;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID;
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::tnt::{DEFAULT_EXPLOSION_POWER, DEFAULT_FUSE_TIME, DEFAULT_GRAVITY};
use crate::entity::{spawn_tnt, tick_entities, EntityData, TntData};

const OWNER: u128 = 0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10;

fn tnt_data(store: &EntityStore, id: i32) -> TntData {
    match store.get(id).expect("entity is live").data() {
        EntityData::Tnt(data) => data.clone(),
        other => panic!("expected primed tnt, found {other:?}"),
    }
}

fn set_tnt(store: &mut EntityStore, id: i32, data: TntData) {
    store.get_mut(id).expect("entity is live").data = EntityData::Tnt(data);
}

#[test]
fn a_spawned_tnt_scatters_and_carries_field_defaults() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let random = GameRandom::with_seed(0x51_a3_c0_de);
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = spawn_tnt(&mut store, &random, position, Some(Uuid::from_u128(OWNER)))
        .expect("a fresh UUID");

    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_TNT_ID);
    let motion = entity.delta_movement();
    assert!((motion.x.hypot(motion.z) - 0.02).abs() < 1.0e-9);
    assert_eq!(motion.y, f64::from(0.2_f32));
    assert!(entity.needs_sync());

    let data = tnt_data(&store, id);
    assert_eq!(data.fuse(), DEFAULT_FUSE_TIME);
    assert_eq!(data.explosion_power(), DEFAULT_EXPLOSION_POWER);
    assert_eq!(data.owner(), Some(Uuid::from_u128(OWNER)));
    assert_eq!(
        data.block_state_id(),
        crate::entity::tnt::default_block_state_id()
    );
    assert_eq!(entity.gravity(), DEFAULT_GRAVITY);
}

#[test]
fn a_primed_tnt_is_pickable() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let random = GameRandom::with_seed(1);
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = spawn_tnt(&mut store, &random, position, None).expect("a fresh UUID");
    let entity = store.get(id).unwrap();
    assert!(entity.is_pickable());
    assert!(entity.can_be_hit_by_projectile());
}

#[test]
fn a_primed_tnt_counts_down_discards_and_explodes_in_the_world() {
    for contact in ["minecraft:air", "minecraft:tripwire"] {
        let mut level = NativeLevel::new();
        if contact != "minecraft:air" {
            level.set(pos(0, 64, 0), state_id(contact));
        }
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
        level.world.set_player_position_for_test(
            crate::connection::ConnectionId(1),
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 6.5,
            },
        );
        let id = level
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                VANILLA_ENTITY_TYPE_TNT_ID,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
                EntityData::Tnt(TntData::with_saved_fields(
                    3,
                    crate::entity::tnt::default_block_state_id(),
                    Some(Uuid::from_u128(OWNER)),
                    DEFAULT_EXPLOSION_POWER,
                )),
            )
            .0;
        for _ in 0..2 {
            level.world.tick(receiver.drain());
            assert!(level.world.stored_entity_for_test(DIMENSION, id).is_some());
        }
        let output = level.world.tick(receiver.drain());
        assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
        assert_eq!(
            level.world.block_state(DIMENSION, pos(0, 63, 0)),
            Some(state_id("minecraft:air"))
        );
        assert!(output.events.iter().any(|event|matches!(event,WorldEvent::Send { packet:ClientboundPlayPacket::Explode(explosion),.. } if explosion.radius==DEFAULT_EXPLOSION_POWER)));
    }
}

#[test]
fn a_teleported_tnt_leaves_a_portal_frame_intact_when_it_explodes() {
    for used in [false, true] {
        let mut level = NativeLevel::new();
        let obsidian = state_id("minecraft:obsidian");
        for y in 63..=67 {
            for x in 1..=4 {
                if y == 63 || y == 67 || x == 1 || x == 4 {
                    level.set(pos(x, y, 0), obsidian);
                }
            }
        }
        for y in 64..=66 {
            for x in 2..=3 {
                level.set(pos(x, y, 0), state_id("minecraft:nether_portal"));
            }
        }
        let id = level
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                VANILLA_ENTITY_TYPE_TNT_ID,
                Vec3d {
                    x: 2.5,
                    y: 64.0,
                    z: 2.5,
                },
                EntityData::Tnt(TntData::with_saved_fields(
                    1,
                    crate::entity::tnt::default_block_state_id(),
                    None,
                    DEFAULT_EXPLOSION_POWER,
                )),
            )
            .0;
        if used {
            crate::entity::tnt::mark_used_portal_after_teleport(
                level
                    .world
                    .stored_entity_mut_for_test(DIMENSION, id)
                    .unwrap(),
            );
        }
        level.world.tick(Vec::new());
        assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
        assert_eq!(
            level.world.block_state(DIMENSION, pos(2, 64, 0))
                == Some(state_id("minecraft:nether_portal")),
            used
        );
    }
}

#[test]
fn used_portal_is_transient_across_chunk_save_and_reload() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let random = GameRandom::with_seed(2);
    let id = spawn_tnt(&mut store, &random, Vec3d::default(), None).unwrap();
    crate::entity::tnt::mark_used_portal_after_teleport(store.get_mut(id).unwrap());
    assert!(tnt_data(&store, id).used_portal());

    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).unwrap();

    assert!(!tnt_data(&reloaded, ids[0]).used_portal());
}

#[test]
fn a_primed_tnt_round_trips_through_the_entity_chunk_save_form() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let random = GameRandom::with_seed(2);
    let position = Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    };
    let owner = Uuid::from_u128(OWNER);
    let id = spawn_tnt(&mut store, &random, position, Some(owner)).expect("a fresh UUID");
    set_tnt(
        &mut store,
        id,
        TntData::with_saved_fields(
            37,
            crate::entity::tnt::default_block_state_id(),
            Some(owner),
            6.0,
        ),
    );
    let before = store.get(id).unwrap().clone();
    let before_data = tnt_data(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(document.entities().next().unwrap().id(), "minecraft:tnt");

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());

    let after_data = tnt_data(&reloaded, ids[0]);
    assert_eq!(after_data.fuse(), before_data.fuse());
    assert_eq!(after_data.block_state_id(), before_data.block_state_id());
    assert_eq!(after_data.explosion_power(), 6.0);
    assert_eq!(after_data.owner(), Some(owner));
}

#[test]
fn the_maximum_explosion_power_round_trips() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let random = GameRandom::with_seed(5);
    let position = Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    };
    let id = spawn_tnt(&mut store, &random, position, None).expect("a fresh UUID");
    set_tnt(
        &mut store,
        id,
        TntData::with_saved_fields(
            80,
            crate::entity::tnt::default_block_state_id(),
            None,
            crate::entity::tnt::MAX_EXPLOSION_POWER,
        ),
    );
    let document = write_entity_chunk(&store, (0, 0)).expect("well formed");
    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("reads back");
    assert_eq!(
        tnt_data(&reloaded, ids[0]).explosion_power(),
        crate::entity::tnt::MAX_EXPLOSION_POWER
    );
}

#[test]
fn primed_tnt_moves_into_water_without_a_splash() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let random = GameRandom::with_seed(1);
    let id = spawn_tnt(
        &mut store,
        &random,
        Vec3d {
            x: 2.4,
            y: 64.0,
            z: 0.5,
        },
        None,
    )
    .unwrap();
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.4,
        y: 0.0,
        z: 0.0,
    });
    store.drain_lifecycle_events();
    let mut wet = NativeLevel::new();
    wet.set(pos(3, 64, 0), state_id("minecraft:water"));
    wet.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).unwrap().is_in_water());
    assert_eq!(tnt_data(&store, id).fuse(), DEFAULT_FUSE_TIME - 1);
    assert!(!wet
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == crate::level_event::VanillaGameEvent::SPLASH));
    assert!(!store
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
}

/// `BlockAttachedEntity.ignoreExplosion` (BlockAttachedEntity.java:93-99): a
/// TNT that explodes in water leaves hanging entities alone.
#[test]
fn a_primed_tnt_exploding_in_water_spares_a_leash_knot() {
    for water in [false, true] {
        let mut level = NativeLevel::new();
        if water {
            level.set(pos(0, 64, 0), state_id("minecraft:water"));
        }
        let fence = pos(0, 64, 2);
        level.set(fence, state_id("minecraft:oak_fence"));
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
        level.world.set_player_position_for_test(
            crate::connection::ConnectionId(1),
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 8.5,
            },
        );
        let (knot, _) = level.world.spawn_stored_entity_for_test(
            DIMENSION,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: 2.5,
            },
            EntityData::LeashKnot(crate::entity::LeashKnotData::new(fence)),
        );
        crate::entity::hanging::apply_direction(
            level
                .world
                .stored_entity_mut_for_test(DIMENSION, knot)
                .unwrap(),
        );
        level.world.spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_TNT_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            EntityData::Tnt(TntData::with_saved_fields(
                3,
                crate::entity::tnt::default_block_state_id(),
                None,
                DEFAULT_EXPLOSION_POWER,
            )),
        );
        for _ in 0..4 {
            level.world.tick(receiver.drain());
        }
        assert_eq!(
            level
                .world
                .stored_entity_for_test(DIMENSION, knot)
                .is_some(),
            water,
            "water: {water}"
        );
    }
}
