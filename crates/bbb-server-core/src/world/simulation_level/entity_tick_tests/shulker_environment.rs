use super::*;
use crate::entity::{EntityLifecycleEvent, ShulkerBulletTarget};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::packets::{SoundEventHolder, SoundSource};

const FEET: Vec3d = Vec3d {
    x: 8.5,
    y: 65.0,
    z: 8.5,
};

fn bullet(store: &mut EntityStore, level: &LevelCtx<'_, '_>) -> i32 {
    let owner = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d { x: 1.5, ..FEET })
        .unwrap();
    let target = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d { x: 12.5, ..FEET })
        .unwrap();
    let target = store.get(target).unwrap();
    let target = ShulkerBulletTarget {
        uuid: target.uuid(),
        position: target.position(),
        height: target.bb_height(),
    };
    let id = crate::entity::shulker_bullet::spawn_shulker_bullet(store, level, owner, target, None)
        .unwrap();
    let body = store.get_mut(id).unwrap();
    body.set_pos(FEET);
    body.set_invulnerable(true);
    id
}

fn block(level: &mut LevelCtx<'_, '_>, name: &str) {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    level
        .write_block(BlockPos { x: 8, y: 65, z: 8 }, state)
        .unwrap();
}

fn assert_effects(events: &[EntityLifecycleEvent], id: i32, position: Vec3d, lava: bool) {
    let mut kinds = Vec::new();
    for event in events {
        match event {
            EntityLifecycleEvent::Sound { sound } => {
                let SoundEventHolder::Reference { registry_id } = sound.sound else {
                    panic!("inline sound")
                };
                let hurt = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    "minecraft:entity.shulker_bullet.hurt",
                )
                .unwrap();
                let burn = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    "minecraft:entity.generic.burn",
                )
                .unwrap();
                if registry_id == hurt {
                    kinds.push("hurt");
                    assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
                } else {
                    assert_eq!(registry_id, burn);
                    kinds.push("burn");
                    assert_eq!(sound.volume, 0.4);
                    assert!((2.0..2.4).contains(&sound.pitch));
                }
                assert_eq!(sound.source, SoundSource::Hostile);
                assert_eq!(sound.position, position);
            }
            EntityLifecycleEvent::Particles { particles } => {
                kinds.push("crit");
                assert_eq!(particles.position, position);
                assert_eq!(
                    particles.particle.particle_type_id,
                    bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_CRIT_ID
                );
                assert_eq!(particles.count, 15);
                assert_eq!(
                    particles.offset,
                    Vec3d {
                        x: 0.2,
                        y: 0.2,
                        z: 0.2
                    }
                );
                assert_eq!(particles.max_speed, 0.0);
            }
            EntityLifecycleEvent::Removed {
                entity,
                tracked: true,
            } => {
                kinds.push("removed");
                assert_eq!(entity.id(), id);
            }
            other => panic!("unexpected effect: {other:?}"),
        }
    }
    assert_eq!(
        kinds,
        if lava {
            vec!["hurt", "crit", "burn", "removed"]
        } else {
            vec!["hurt", "crit", "removed"]
        }
    );
}

#[test]
fn shulker_environment_real_tick_fire_and_lava_destroy_invulnerable_bullet() {
    for lava in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let mut store = batch_store();
            let id = bullet(&mut store, level);
            if lava {
                block(level, "minecraft:lava");
            } else {
                store.get_mut(id).unwrap().set_remaining_fire_ticks(20);
            }
            store.drain_lifecycle_events();
            crate::entity::tick_one_entity(&mut store, level, id);
            assert!(store.get(id).is_none());
            let events = level.event_sink.drain_positioned_game_events();
            assert_eq!(
                events
                    .iter()
                    .map(|event| event.recorded.game_event)
                    .collect::<Vec<_>>(),
                vec![
                    VanillaGameEvent::PROJECTILE_SHOOT,
                    VanillaGameEvent::ENTITY_DAMAGE
                ]
            );
            assert_eq!(events[1].recorded.source_entity_id, Some(id));
            if !lava {
                assert_eq!(events[1].position, FEET);
            }
            assert_effects(
                &store.drain_lifecycle_events(),
                id,
                events[1].position,
                lava,
            );
            assert!(store.drain_commands().is_empty());
        });
    }
}
