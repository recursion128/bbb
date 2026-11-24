use super::*;
use crate::entity::EntityLifecycleEvent;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::packets::LevelParticles;
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const FLOOR: BlockPos = BlockPos { x: 8, y: 64, z: 8 };
const START: Vec3d = Vec3d {
    x: 8.3125,
    y: 65.25,
    z: 8.6875,
};

#[derive(Clone, Copy)]
enum Body {
    Cow,
    ArmorStand,
}

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn arena(level: &mut LevelCtx<'_, '_>, floor: i32) {
    for x in 6..=11 {
        for z in 6..=11 {
            for y in 63..=69 {
                level
                    .write_block(BlockPos { x, y, z }, state("minecraft:air"))
                    .unwrap();
            }
            level.write_block(BlockPos { x, y: 64, z }, floor).unwrap();
        }
    }
}

fn spawn(store: &mut EntityStore, kind: Body, start: Vec3d, silent: bool) -> i32 {
    let id = match kind {
        Body::Cow => store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, start)
            .unwrap(),
        Body::ArmorStand => crate::entity::spawn_armor_stand(store, start).unwrap(),
    };
    let NbtSummaryValue::Compound(mut data) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    data.retain(|entry| {
        entry.name != "NoAI" && entry.name != "Silent" && entry.name != "fall_distance"
    });
    data.extend([
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(0),
        },
        NbtSummaryEntry {
            name: "Silent".into(),
            value: NbtSummaryValue::Byte(i8::from(silent)),
        },
        NbtSummaryEntry {
            name: "fall_distance".into(),
            value: NbtSummaryValue::Double(8.8_f64.to_bits()),
        },
    ]);
    store.load_command_data(id, &data).unwrap();
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    });
    assert_eq!(store.get(id).unwrap().fall_distance(), 8.8);
    store.drain_lifecycle_events();
    id
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        crate::entity::EntityTickOutcome::Complete
    ));
}

fn particles(store: &mut EntityStore) -> Vec<LevelParticles> {
    store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Particles { particles } => Some(particles),
            _ => None,
        })
        .collect()
}

fn assert_stone_packet(packet: &LevelParticles) {
    assert!(!packet.override_limiter);
    assert!(!packet.always_show);
    assert_eq!(packet.position, Vec3d { y: 65.0, ..START });
    assert_eq!(packet.offset, Vec3d::default());
    assert_eq!(packet.max_speed.to_bits(), 0.15_f32.to_bits());
    // LivingEntity.java:344-375: old 8.8 yields power 5/count 80; base sees 9.05.
    assert_eq!(packet.count, 80);
    assert_eq!(
        packet.particle.particle_type_id,
        VANILLA_PARTICLE_TYPE_BLOCK_ID
    );
    let mut raw = Vec::new();
    bbb_protocol::codec::write_var_i32_to(&mut raw, state("minecraft:stone"));
    assert_eq!(packet.particle.raw_options, raw);
}

#[test]
fn living_fall_particles_real_cow_and_armor_stand_use_old_distance_and_moved_origin() {
    for kind in [Body::Cow, Body::ArmorStand] {
        for silent in [false, true] {
            with_config(crate::tests::datapack_config(), |level| {
                arena(level, state("minecraft:stone"));
                let mut store = batch_store();
                let id = spawn(&mut store, kind, START, silent);
                let health = store.get(id).unwrap().living().unwrap().health();
                tick(&mut store, level, id);
                let body = store.get(id).unwrap();
                assert_eq!(body.position(), Vec3d { y: 65.0, ..START });
                assert!(body.on_ground());
                assert!(body.is_supported_by(FLOOR));
                assert_eq!(body.fall_distance(), 0.0);
                if matches!(kind, Body::Cow) {
                    assert_eq!(body.living().unwrap().health(), health - 6.0);
                }
                let packets = particles(&mut store);
                assert_eq!(packets.len(), 1);
                assert_stone_packet(&packets[0]);
                assert_eq!(
                    level
                        .event_sink
                        .recent_game_events()
                        .filter(|event| event.game_event == VanillaGameEvent::HIT_GROUND)
                        .count(),
                    1
                );
            });
        }
    }
}

#[test]
fn living_fall_particles_moved_water_reset_suppresses_landing_packet_and_hurt() {
    for kind in [Body::Cow, Body::ArmorStand] {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level, state("minecraft:stone"));
            level
                .write_block(BlockPos { x: 9, y: 65, z: 8 }, state("minecraft:water"))
                .unwrap();
            let mut store = batch_store();
            let id = spawn(&mut store, kind, START, false);
            store.get_mut(id).unwrap().set_delta_movement(Vec3d {
                x: 0.75,
                y: -0.5,
                z: 0.0,
            });
            let health = store.get(id).unwrap().living().unwrap().health();
            assert!(!store.get(id).unwrap().is_in_water());
            assert!(store.get(id).unwrap().bounding_box().max_x < 9.0);
            tick(&mut store, level, id);
            let body = store.get(id).unwrap();
            assert_eq!(
                body.position(),
                Vec3d {
                    x: START.x + 0.75,
                    y: 65.0,
                    ..START
                }
            );
            let moved = body.position();
            assert!((moved.x - START.x).powi(2) + (moved.y - START.y).powi(2) < 1.0);
            assert!(body.on_ground());
            assert!(body.is_in_water());
            assert_eq!(body.fall_distance(), 0.0);
            assert_eq!(body.living().unwrap().health(), health);
            assert!(particles(&mut store).is_empty());
            assert!(!level
                .event_sink
                .recent_game_events()
                .any(|event| event.game_event == VanillaGameEvent::HIT_GROUND));
        });
    }
}
