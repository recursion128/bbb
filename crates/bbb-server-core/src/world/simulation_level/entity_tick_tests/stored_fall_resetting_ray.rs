use super::*;
use crate::entity::EntityLifecycleEvent;
use crate::level_event::VanillaGameEvent;
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::packets::LevelParticles;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use std::sync::Arc;

const START: Vec3d = Vec3d {
    x: 8.3125,
    y: 65.25,
    z: 8.6875,
};
const TRANSIT: BlockPos = BlockPos { x: 9, y: 65, z: 8 };

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

fn arena(level: &mut LevelCtx<'_, '_>) {
    for x in 6..=22 {
        for z in 6..=10 {
            for y in 63..=71 {
                level
                    .write_block(
                        BlockPos { x, y, z },
                        state(if y == 64 {
                            "minecraft:stone"
                        } else {
                            "minecraft:air"
                        }),
                    )
                    .unwrap();
            }
        }
    }
}

fn install_tag(level: &mut LevelCtx<'_, '_>, registry_id: &str, tag_id: &str, members: &[&str]) {
    let config = crate::tests::datapack_config();
    let registries = &config.datapacks.as_ref().unwrap().tags.registries;
    let mut replaced = false;
    let mut tags = Vec::new();
    for registry in registries {
        for (tag, existing) in &registry.tags {
            let members = if registry.registry_id == registry_id && tag == tag_id {
                replaced = true;
                members.iter().map(|member| (*member).to_string()).collect()
            } else {
                existing.clone()
            };
            tags.push((registry.registry_id.clone(), tag.clone(), members));
        }
    }
    assert!(replaced);
    level
        .chunks
        .install_registry_tags(Arc::new(RegistryTagSnapshot::new(tags)));
}

fn spawn(store: &mut EntityStore, kind: Body, start: Vec3d, delta: Vec3d) -> i32 {
    let id = match kind {
        Body::Cow => store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, start)
            .unwrap(),
        Body::ArmorStand => crate::entity::spawn_armor_stand(store, start).unwrap(),
    };
    let NbtSummaryValue::Compound(mut data) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    data.retain(|entry| entry.name != "NoAI" && entry.name != "fall_distance");
    data.extend([
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(0),
        },
        NbtSummaryEntry {
            name: "fall_distance".into(),
            value: NbtSummaryValue::Double(8.8_f64.to_bits()),
        },
    ]);
    store.load_command_data(id, &data).unwrap();
    store.get_mut(id).unwrap().set_delta_movement(delta);
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

fn landed(
    level: &LevelCtx<'_, '_>,
    store: &mut EntityStore,
    id: i32,
    kind: Body,
    health: f32,
    dx: f64,
    reset: bool,
) {
    let body = store.get(id).unwrap();
    assert_eq!(body.tick_count(), 1);
    assert_eq!(
        body.position(),
        Vec3d {
            x: START.x + dx,
            y: 65.0,
            ..START
        }
    );
    assert!(body.on_ground());
    assert!(!body.is_in_water());
    assert_eq!(body.fall_distance(), 0.0);
    if matches!(kind, Body::Cow) {
        assert_eq!(
            body.living().unwrap().health(),
            health - if reset { 0.0 } else { 6.0 }
        );
    }
    let packets = particles(store);
    if reset {
        assert!(packets.is_empty());
    } else {
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].count, 80);
        assert_eq!(
            packets[0].position,
            Vec3d {
                x: START.x + dx,
                y: 65.0,
                ..START
            }
        );
    }
    let hits: Vec<_> = level
        .event_sink
        .recent_game_events()
        .filter(|event| event.game_event == VanillaGameEvent::HIT_GROUND)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].source_entity_id, Some(id));
    assert_eq!(hits[0].affected_state, Some(state("minecraft:stone")));
}

#[test]
fn stored_fall_resetting_ray_transit_water_resets_before_dry_landing_with_hit_ground_retained() {
    // Entity.java:736-744; LivingEntity.java:344-375.
    for kind in [Body::Cow, Body::ArmorStand] {
        for (water, live_water_tag) in [(true, true), (false, true), (true, false)] {
            with_config(crate::tests::datapack_config(), |level| {
                arena(level);
                if water {
                    level
                        .write_block(TRANSIT, state("minecraft:water"))
                        .unwrap();
                }
                if !live_water_tag {
                    install_tag(level, "minecraft:fluid", "minecraft:water", &[]);
                    assert!(!LevelCtx::registry_tag_membership(
                        level,
                        "minecraft:fluid",
                        "minecraft:water",
                        "minecraft:water"
                    ));
                }
                let mut store = batch_store();
                let id = spawn(
                    &mut store,
                    kind,
                    START,
                    Vec3d {
                        x: 2.5,
                        y: -0.5,
                        z: 0.0,
                    },
                );
                assert!(store.get(id).unwrap().bounding_box().max_x < f64::from(TRANSIT.x));
                let health = store.get(id).unwrap().living().unwrap().health();
                tick(&mut store, level, id);
                assert!(store.get(id).unwrap().bounding_box().min_x > f64::from(TRANSIT.x + 1));
                landed(
                    level,
                    &mut store,
                    id,
                    kind,
                    health,
                    2.5,
                    water && live_water_tag,
                );
            });
        }
    }
}

#[test]
fn stored_fall_resetting_ray_live_block_tag_and_actual_clipped_length_gate() {
    for kind in [Body::Cow, Body::ArmorStand] {
        for (tagged, dx, requested_dy) in
            [(false, 2.5, -0.5), (true, 2.5, -0.5), (true, 0.75, -2.0)]
        {
            with_config(crate::tests::datapack_config(), |level| {
                arena(level);
                level
                    .write_block(TRANSIT, state("minecraft:structure_void"))
                    .unwrap();
                install_tag(
                    level,
                    "minecraft:block",
                    "minecraft:fall_damage_resetting",
                    if tagged {
                        &["minecraft:structure_void"]
                    } else {
                        &[]
                    },
                );
                let actual = Vec3d {
                    x: START.x + dx,
                    y: 65.0,
                    ..START
                };
                assert_eq!(
                    LevelCtx::clip_fall_resetting(level, START, actual),
                    Some(tagged)
                );
                let mut store = batch_store();
                let id = spawn(
                    &mut store,
                    kind,
                    START,
                    Vec3d {
                        x: dx,
                        y: requested_dy,
                        z: 0.0,
                    },
                );
                let health = store.get(id).unwrap().living().unwrap().health();
                tick(&mut store, level, id);
                landed(level, &mut store, id, kind, health, dx, tagged && dx >= 1.0);
            });
        }
    }
}

#[test]
fn stored_fall_resetting_ray_eight_block_cap_ignores_later_water_before_dry_endpoint() {
    for kind in [Body::Cow, Body::ArmorStand] {
        for water_x in [15, 17] {
            with_config(crate::tests::datapack_config(), |level| {
                arena(level);
                level
                    .write_block(
                        BlockPos {
                            x: water_x,
                            ..TRANSIT
                        },
                        state("minecraft:water"),
                    )
                    .unwrap();
                let mut store = batch_store();
                let id = spawn(
                    &mut store,
                    kind,
                    START,
                    Vec3d {
                        x: 11.0,
                        y: -0.5,
                        z: 0.0,
                    },
                );
                let health = store.get(id).unwrap().living().unwrap().health();
                tick(&mut store, level, id);
                assert!(store.get(id).unwrap().bounding_box().min_x > f64::from(water_x + 1));
                landed(level, &mut store, id, kind, health, 11.0, water_x == 15);
            });
        }
    }
}

#[test]
fn stored_fall_resetting_ray_real_level_portal_shapes_require_the_live_block_tag() {
    // ClipContext.java:60-82: Player portal extras do not apply to Stored context.
    for portal in [
        "minecraft:end_gateway",
        "minecraft:end_portal",
        "minecraft:nether_portal",
    ] {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level);
            level.write_block(TRANSIT, state(portal)).unwrap();
            let to = Vec3d {
                x: START.x + 2.5,
                y: 65.0,
                ..START
            };
            assert_eq!(LevelCtx::clip_fall_resetting(level, START, to), Some(false));
            install_tag(
                level,
                "minecraft:block",
                "minecraft:fall_damage_resetting",
                &[portal],
            );
            assert_eq!(LevelCtx::clip_fall_resetting(level, START, to), Some(true));
        });
    }
}
