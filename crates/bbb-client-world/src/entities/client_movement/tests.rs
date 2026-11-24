use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
    VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
    VANILLA_ENTITY_TYPE_FIREBALL_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
    VANILLA_ENTITY_TYPE_SNOWBALL_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, VANILLA_ENTITY_TYPE_TNT_ID,
    VANILLA_ENTITY_TYPE_TRIDENT_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, EntityDataValue, ItemStackSummary, RemoveEntities,
    SetBorderLerpSize as ProtocolSetBorderLerpSize, SetBorderSize as ProtocolSetBorderSize,
    SetEntityData as ProtocolSetEntityData, SetEntityMotion as ProtocolSetEntityMotion,
    Vec3d as ProtocolVec3d,
};
use uuid::Uuid;

use crate::{
    BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
    PaletteKind, PalettedContainerData, WorldDimension, WorldStore,
};

use super::*;
use crate::entities::{EntityClientAnimationState, ProjectileLifecycleState};

mod arrows;
mod fishing_hooks;
mod nonliving;

#[test]
fn direct_projectiles_integrate_synced_motion_and_shulker_rotation() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        10,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
        ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 1.0,
        },
    ));
    store.apply_add_entity(projectile(
        20,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        ProtocolVec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    assert_eq!(
        store
            .drain_entity_client_movement_effect_states()
            .collect::<Vec<_>>(),
        vec![
            particle_effect(EntityClientMovementParticleState::EyeOfEnder(
                EyeOfEnderTrailParticleState {
                    entity_id: 10,
                    origin: EntityVec3 {
                        x: 1.1875,
                        y: 63.625,
                        z: -1.25,
                    },
                    delta_movement: EntityVec3 {
                        x: 0.25,
                        y: -0.5,
                        z: 1.0,
                    },
                    in_water: false,
                },
            )),
            particle_effect(EntityClientMovementParticleState::ShulkerBullet(
                ShulkerBulletTrailParticleState {
                    entity_id: 20,
                    position: EntityVec3 {
                        x: 1.0,
                        y: 64.15,
                        z: -2.0,
                    },
                },
            )),
        ]
    );

    assert_eq!(
        store.probe_entity_transform(10).unwrap().position,
        EntityVec3 {
            x: 1.25,
            y: 63.5,
            z: -1.0,
        }
    );
    let bullet = store.probe_entity_transform(20).unwrap();
    assert_eq!(
        bullet.position,
        EntityVec3 {
            x: 2.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(bullet.y_rot, 45.0);
    assert_eq!(bullet.x_rot, 0.0);
    assert_eq!(
        generation_a.probe_entity_transform(20).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        "the retained generation remains isolated from client motion"
    );
}

#[test]
fn throwable_projectiles_apply_generated_gravity_and_float_inertia() {
    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (30, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
        (31, VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID),
        (32, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID),
        (33, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
    ] {
        store.apply_add_entity(projectile(
            id,
            entity_type_id,
            ProtocolVec3d {
                x: 0.25,
                y: 0.5,
                z: -0.75,
            },
        ));
    }
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 33,
        values: vec![EntityDataValue {
            data_id: entity_accessors.no_gravity_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));

    store.advance_entity_client_animations(1);

    let inertia = f64::from(0.99_f32);
    for (id, gravity) in [(30, 0.03), (31, 0.07), (32, 0.05)] {
        let transform = store.probe_entity_transform(id).unwrap();
        assert_vec3_near(
            transform.delta_movement,
            EntityVec3 {
                x: 0.25 * inertia,
                y: (0.5 - gravity) * inertia,
                z: -0.75 * inertia,
            },
        );
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 1.0 + 0.25 * inertia,
                y: 64.0 + (0.5 - gravity) * inertia,
                z: -2.0 - 0.75 * inertia,
            },
        );
    }
    let no_gravity = store.probe_entity_transform(33).unwrap();
    assert_vec3_near(
        no_gravity.delta_movement,
        EntityVec3 {
            x: 0.25 * inertia,
            y: 0.5 * inertia,
            z: -0.75 * inertia,
        },
    );
}

#[test]
fn hurting_projectile_families_apply_acceleration_inertia_and_typed_trails() {
    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (40, VANILLA_ENTITY_TYPE_FIREBALL_ID),
        (41, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID),
        (42, VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID),
        (43, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID),
    ] {
        store.apply_add_entity(projectile(
            id,
            entity_type_id,
            ProtocolVec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        ));
    }
    let (dangerous_data_id, _) =
        vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        )
        .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 43,
        values: vec![EntityDataValue {
            data_id: dangerous_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let fireball_delta = 1.1 * f64::from(0.95_f32);
    let wither_delta = 1.1 * f64::from(0.73_f32);
    assert_vec3_near(
        store.probe_entity_transform(40).unwrap().delta_movement,
        EntityVec3 {
            x: fireball_delta,
            y: 0.0,
            z: 0.0,
        },
    );
    assert_vec3_near(
        store.probe_entity_transform(41).unwrap().delta_movement,
        EntityVec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    );
    assert_vec3_near(
        store.probe_entity_transform(43).unwrap().delta_movement,
        EntityVec3 {
            x: wither_delta,
            y: 0.0,
            z: 0.0,
        },
    );
    assert_eq!(
        store
            .drain_entity_client_movement_effect_states()
            .collect::<Vec<_>>(),
        vec![
            simple_particle(
                40,
                EntityClientMovementSimpleParticleKind::Smoke,
                fireball_delta
            ),
            simple_particle(
                42,
                EntityClientMovementSimpleParticleKind::DragonBreath,
                fireball_delta,
            ),
            simple_particle(
                43,
                EntityClientMovementSimpleParticleKind::Smoke,
                wither_delta
            ),
        ]
    );
    assert_vec3_near(
        generation_a.probe_entity_transform(43).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
    );
}

#[test]
fn throwable_water_path_emits_four_pre_inertia_bubbles() {
    let metadata = EntityMetadata::default();
    let mut transform = test_transform(EntityVec3 {
        x: 0.4,
        y: -0.2,
        z: 0.6,
    });
    let mut effects = Vec::new();

    advance_throwable_projectile(
        50,
        &metadata,
        &mut transform,
        VanillaEntityClientMovementPolicy::ThrowableProjectile,
        true,
        &mut effects,
    );

    let gravity_movement = EntityVec3 {
        x: 0.4,
        y: -0.23,
        z: 0.6,
    };
    let bubble_position = EntityVec3 {
        x: 1.0 - gravity_movement.x * 0.25,
        y: 64.0 - gravity_movement.y * 0.25,
        z: -2.0 - gravity_movement.z * 0.25,
    };
    assert_eq!(effects.len(), 4);
    assert!(effects.iter().all(|state| *state
        == particle_effect(EntityClientMovementParticleState::Simple(
            EntityClientMovementSimpleParticleState {
                entity_id: 50,
                kind: EntityClientMovementSimpleParticleKind::Bubble,
                position: bubble_position,
                velocity: gravity_movement,
            }
        ))));
    assert_vec3_near(
        transform.delta_movement,
        scale(gravity_movement, f64::from(0.8_f32)),
    );
}

fn simple_particle(
    entity_id: i32,
    kind: EntityClientMovementSimpleParticleKind,
    moved_x: f64,
) -> EntityClientMovementEffectState {
    particle_effect(EntityClientMovementParticleState::Simple(
        EntityClientMovementSimpleParticleState {
            entity_id,
            kind,
            position: EntityVec3 {
                x: 1.0 + moved_x,
                y: 64.5,
                z: -2.0,
            },
            velocity: EntityVec3::default(),
        },
    ))
}

fn particle_effect(state: EntityClientMovementParticleState) -> EntityClientMovementEffectState {
    EntityClientMovementEffectState::Particle(state)
}

fn assert_vec3_near(actual: EntityVec3, expected: EntityVec3) {
    for (actual, expected) in [
        (actual.x, expected.x),
        (actual.y, expected.y),
        (actual.z, expected.z),
    ] {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "{actual} != {expected}"
        );
    }
}

fn test_transform(delta_movement: EntityVec3) -> EntityTransform {
    let position = EntityVec3 {
        x: 1.0,
        y: 64.0,
        z: -2.0,
    };
    EntityTransform {
        position,
        old_position: position,
        position_base: position,
        delta_movement,
        y_rot: 0.0,
        old_y_rot: 0.0,
        x_rot: 0.0,
        old_x_rot: 0.0,
        y_head_rot: 0.0,
        old_y_head_rot: 0.0,
        on_ground: None,
        interpolation: None,
    }
}

fn test_arrow_lifecycle() -> ProjectileLifecycleState {
    ProjectileLifecycleState {
        owner_uuid: None,
        left_owner: true,
        has_been_shot: true,
        trident_in_ground_time: 0,
        trident_dealt_damage: false,
        trident_return_ticks: 0,
        arrow_pickup_allowed: false,
        arrow_no_physics: false,
        arrow_last_block_state_id: None,
        arrow_hit_ground_sound_is_arrow: false,
    }
}

fn test_arrow_runtime(seed: i64) -> EntityAbstractArrowRuntime {
    EntityAbstractArrowRuntime {
        random_seed: java_random_seed(seed),
        last_deflected_by: None,
        pending_last_deflected_by: None,
        piercing_ignore_entity_ids: Vec::new(),
        in_ground_time: 0,
    }
}

fn projectile(id: i32, entity_type_id: i32, delta_movement: ProtocolVec3d) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement,
        y_rot: 0.0,
        x_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn set_nonempty_item_stack(store: &mut WorldStore, id: i32) {
    let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("generated ItemEntity stack accessor");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id,
        values: vec![EntityDataValue {
            data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count: 1,
                component_patch: Default::default(),
            }),
        }],
    }));
}

fn nonliving_collision_world() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: PalettedContainerData {
                domain: PaletteDomain::BlockStates,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![0],
                packed_data: Vec::new(),
                entry_count: 4096,
            },
            biomes: PalettedContainerData {
                domain: PaletteDomain::Biomes,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![0],
                packed_data: Vec::new(),
                entry_count: 64,
            },
        })],
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
    let grass = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(
            "minecraft:grass_block",
            &[("snowy".to_owned(), "false".to_owned())]
                .into_iter()
                .collect(),
        )
        .expect("vanilla grass block state")
        .id;
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
            block_state_id: grass,
        })
    );
    assert!(world.probe_block(BlockPos { x: 0, y: 0, z: 0 }).is_some());
    world
}
