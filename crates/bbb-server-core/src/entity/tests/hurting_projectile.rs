use super::native_level::{NativeLevel, DIMENSION};
use super::*;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
    VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID, VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
    VANILLA_ENTITY_TYPE_FIREBALL_ID, VANILLA_ENTITY_TYPE_INTERACTION_ID,
    VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
    VANILLA_ENTITY_TYPE_TNT_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
};
use bbb_protocol::packets::Direction;

use crate::entity::hurting_projectile::{self, HurtingProjectileState, FIREBALL_SPEC};
use crate::entity::item::{plain_stack, ItemEntityData};
use crate::entity::projectile_action::{
    hurt_projectile_to_stored, hurt_stored_entity, PendingProjectileHurt,
    PendingProjectileMobEffect, StoredHurtRequest, StoredHurtResult,
};
use crate::entity::projectile_hit::{
    hit_result_on_move_vector, HitResult, ProjectileCandidate, ProjectileTarget,
};
use crate::entity::{tick_one_entity, Entity, EntityTickOutcome, LargeFireballData};
use crate::level_event::PendingHurtEntity;

fn damage_type(resource_id: &str) -> &'static bbb_server_datapack::damage::DamageTypeFacts {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in damage type"))
}

fn framed_map_stack(map_id: i32) -> bbb_server_types::item_stack::RuntimeItemStack {
    let raw = bbb_registry_codec::item_stack_summary_to_persistent_nbt(&ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
            ],
            map_id: Some(map_id),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    })
    .unwrap();
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &raw,
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

fn source(position: Vec3d, movement: Vec3d) -> Entity {
    let mut entity = Entity::new(
        99,
        Uuid::from_u128(99),
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
        position,
        EntityData::LargeFireball(LargeFireballData::new(
            HurtingProjectileState::new(None, 0.1),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
            1,
        )),
    );
    entity.set_delta_movement(movement);
    entity
}

#[test]
fn hurting_projectile_contact_reaches_the_following_base_tick_before_burning() {
    let mut store = EntityStore::new(100, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        EntityData::SmallFireball(crate::entity::SmallFireballData::new(
            HurtingProjectileState::new(None, 0.0),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
        )),
    );
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    });
    let entry = BlockPos { x: 1, y: 68, z: 0 };
    let mut level = NativeLevel::new();
    level.set(entry, state_id("minecraft:nether_portal"));

    let EntityTickOutcome::Portal(pending) =
        level.with_level(|level| tick_one_entity(&mut store, level, id))
    else {
        panic!("AbstractHurtingProjectile.applyEffectsFromBlocks must feed super.tick");
    };

    assert_eq!(pending.requester_entity_id(), id);
    assert_eq!(pending.request().entry, entry);
    assert_eq!(store.get(id).unwrap().remaining_fire_ticks(), 0);
}

#[test]
fn prepared_source_position_applies_dynamic_no_knockback() {
    for (no_knockback, expected_x) in [(false, 0.4), (true, 0.0)] {
        let mut store = EntityStore::new(0xCA79, EntityIdCounter::new());
        let target = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
            )
            .unwrap();
        let rows = no_knockback.then_some((
            "minecraft:damage_type",
            "minecraft:no_knockback",
            &["minecraft:generic"] as &[_],
        ));
        let registry_tags = crate::entity::EntityRegistryTagView::Frozen(Arc::new(
            registry_tag_snapshot(&rows.into_iter().collect::<Vec<_>>()),
        ));
        let damage_type = damage_type("minecraft:generic");
        let mut level = NativeLevel::new();
        let result = level.with_level(|level| {
            hurt_stored_entity(
                StoredHurtRequest {
                    direct_entity_snapshot: None,
                    target_entity_id: target,
                    damage_type_resource_id: "minecraft:generic",
                    amount: 1.0,
                    direct_resource_id: None,
                    registry_tags: &registry_tags,
                    causing_entity_snapshot: None,
                    source_position: Some(Vec3d::default()),
                    explicit_source_position: None,
                },
                None,
                crate::entity::StoredEntityDamageSource {
                    damage_type,
                    direct_entity_id: None,
                    causing_entity: None,
                    attacker_views: Default::default(),
                },
                Default::default(),
                Default::default(),
                &mut store,
                level,
            )
        });
        assert!(matches!(result, StoredHurtResult::Applied { outcome, .. } if outcome.succeeded));
        assert!((store.get(target).unwrap().delta_movement().x - expected_x).abs() < 1.0e-9);
    }
}

#[test]
fn prepared_coupled_effect_preserves_live_rejection() {
    let mut store = EntityStore::new(0xCA78, EntityIdCounter::new());
    store.set_difficulty(bbb_protocol::difficulties::Difficulty::Normal);
    let target = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d::default(),
        )
        .unwrap();
    let registry_tags =
        crate::entity::EntityRegistryTagView::Frozen(Arc::new(registry_tag_snapshot(&[(
            "minecraft:entity_type",
            "minecraft:ignores_poison_and_regen",
            &["minecraft:cow"],
        )])));
    let hurt = PendingProjectileHurt {
        target: PendingHurtEntity::Stored { entity_id: target },
        damage_type: "minecraft:mob_projectile",
        amount: 4.0,
        causing: None,
        direct: None,
        direct_resource_id: "minecraft:shulker_bullet",
        registry_tags,
    };
    let effect = PendingProjectileMobEffect {
        resource_id: "minecraft:poison",
        amplifier: 0,
        easy_duration_ticks: 200,
        normal_duration_ticks: 200,
        hard_duration_ticks: 200,
        source_owner: None,
        projectile_entity_id: 99,
    };
    let damage_type = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:mob_projectile")
        .unwrap();
    let mut level = NativeLevel::new();
    let result = level.with_level(|level| {
        hurt_projectile_to_stored(
            &hurt,
            Some(&effect),
            crate::entity::damage::StoredEntityDamageSource {
                damage_type,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            Default::default(),
            Default::default(),
            &mut store,
            level,
        )
    });

    assert!(matches!(
        result,
        StoredHurtResult::Applied {
            outcome,
            effect_applied: false,
            ..
        } if outcome.succeeded
    ));
    assert!(store
        .get(target)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:poison")
        .is_none());
}

#[test]
fn prepared_falling_block_marks_hurt_but_keeps_the_false_return() {
    let mut store = EntityStore::new(0xfa11, EntityIdCounter::new());
    let target = store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d::default(),
        EntityData::FallingBlock(crate::entity::FallingBlockData::new(
            state_id("minecraft:sand"),
            BlockPos { x: 0, y: 0, z: 0 },
        )),
    );
    let hurt = PendingProjectileHurt {
        target: PendingHurtEntity::Stored { entity_id: target },
        damage_type: "minecraft:generic",
        amount: 1.0,
        causing: None,
        direct: None,
        direct_resource_id: "minecraft:snowball",
        registry_tags: crate::entity::EntityRegistryTagView::Frozen(Arc::new(
            registry_tag_snapshot(&[]),
        )),
    };
    let mut level = NativeLevel::new();
    let result = level.with_level(|level| {
        hurt_projectile_to_stored(
            &hurt,
            None,
            crate::entity::StoredEntityDamageSource {
                damage_type: damage_type("minecraft:generic"),
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            Default::default(),
            Default::default(),
            &mut store,
            level,
        )
    });
    assert!(matches!(
        result,
        StoredHurtResult::Applied { outcome, .. }
            if outcome.applied && !outcome.succeeded
    ));
    assert!(store.get(target).unwrap().hurt_marked());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
}

#[test]
fn prepared_fixed_frame_refusal_stops_after_the_bypass_tag() {
    let mut store = EntityStore::new(0xf1ced, EntityIdCounter::new());
    let target = store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        Vec3d::default(),
        EntityData::ItemFrame(
            crate::entity::ItemFrameData::new(BlockPos { x: 0, y: 0, z: 0 }, Direction::North)
                .with_saved_fields(None, 0, 1.0, true, false),
        ),
    );
    let hurt = PendingProjectileHurt {
        target: PendingHurtEntity::Stored { entity_id: target },
        damage_type: "minecraft:generic",
        amount: 1.0,
        causing: None,
        direct: None,
        direct_resource_id: "minecraft:arrow",
        registry_tags: crate::entity::EntityRegistryTagView::Frozen(Arc::new(
            registry_tag_snapshot(&[(
                "minecraft:damage_type",
                "minecraft:is_fall",
                &["minecraft:generic"],
            )]),
        )),
    };

    let mut level = NativeLevel::new();
    let result = level.with_level(|level| {
        hurt_projectile_to_stored(
            &hurt,
            None,
            crate::entity::StoredEntityDamageSource {
                damage_type: damage_type("minecraft:generic"),
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            Default::default(),
            Default::default(),
            &mut store,
            level,
        )
    });

    assert!(matches!(
        result,
        StoredHurtResult::Applied { outcome, .. }
            if !outcome.applied && !outcome.succeeded
    ));
    assert!(store.get(target).is_some());
    assert_eq!(
        level.world.positioned_game_events_for_test(DIMENSION).len(),
        0
    );
}

#[test]
fn framed_map_projectile_clear_removes_the_displayed_item() {
    let mut store = EntityStore::new(0x4b0, EntityIdCounter::new());
    let target = store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        Vec3d::default(),
        EntityData::ItemFrame(
            crate::entity::ItemFrameData::new(BlockPos { x: 7, y: 65, z: -2 }, Direction::West)
                .with_saved_fields(Some(framed_map_stack(51)), 3, 1.0, false, false),
        ),
    );
    let hurt = PendingProjectileHurt {
        target: PendingHurtEntity::Stored { entity_id: target },
        damage_type: "minecraft:generic",
        amount: 1.0,
        causing: None,
        direct: None,
        direct_resource_id: "minecraft:arrow",
        registry_tags: crate::entity::EntityRegistryTagView::Frozen(Arc::new(
            registry_tag_snapshot(&[]),
        )),
    };
    let mut level = NativeLevel::new();
    let result = level.with_level(|level| {
        hurt_projectile_to_stored(
            &hurt,
            None,
            crate::entity::StoredEntityDamageSource {
                damage_type: damage_type("minecraft:generic"),
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            crate::entity::StoredEntityDamageRules {
                entity_drops: false,
                ..Default::default()
            },
            Default::default(),
            &mut store,
            level,
        )
    });
    assert!(matches!(
        result,
        StoredHurtResult::Applied { outcome, .. }
            if outcome.applied && outcome.succeeded
    ));
    assert!(matches!(
        store.get(target).unwrap().data(),
        EntityData::ItemFrame(frame) if frame.item().is_none()
    ));
}

#[test]
fn framed_map_projectile_owner_with_entity_drops_false_keeps_marker() {
    let mut store = EntityStore::new(0x4b4, EntityIdCounter::new());
    let causing = crate::entity::spawn_item_entity(
        &mut store,
        Vec3d::default(),
        plain_stack("minecraft:stone", 1).unwrap(),
    )
    .unwrap();
    let target = store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        Vec3d::default(),
        EntityData::ItemFrame(
            crate::entity::ItemFrameData::new(BlockPos { x: 8, y: 65, z: -2 }, Direction::West)
                .with_saved_fields(Some(framed_map_stack(52)), 0, 1.0, false, false),
        ),
    );
    let hurt = PendingProjectileHurt {
        target: PendingHurtEntity::Stored { entity_id: target },
        damage_type: "minecraft:generic",
        amount: 1.0,
        causing: Some(PendingHurtEntity::Stored { entity_id: causing }),
        direct: None,
        direct_resource_id: "minecraft:arrow",
        registry_tags: crate::entity::EntityRegistryTagView::Frozen(Arc::new(
            registry_tag_snapshot(&[]),
        )),
    };
    let mut level = NativeLevel::new();
    let result = level.with_level(|level| {
        hurt_projectile_to_stored(
            &hurt,
            None,
            crate::entity::StoredEntityDamageSource {
                damage_type: damage_type("minecraft:generic"),
                direct_entity_id: None,
                causing_entity: Some(crate::entity::StoredDamageCausingEntity::Other {
                    entity_id: causing,
                }),
                attacker_views: Default::default(),
            },
            crate::entity::StoredEntityDamageRules {
                entity_drops: false,
                ..Default::default()
            },
            Default::default(),
            &mut store,
            level,
        )
    });
    assert!(matches!(
        result,
        StoredHurtResult::Applied { outcome, .. }
            if outcome.applied && outcome.succeeded
    ));
    assert!(matches!(
        store.get(target).unwrap().data(),
        EntityData::ItemFrame(frame) if frame.item().is_none()
    ));
}

#[test]
fn entity_sweep_wins_before_the_clipped_block_and_preserves_block_face() {
    let start = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let movement = Vec3d {
        x: 4.0,
        y: 0.0,
        z: 0.0,
    };
    let source = source(start, movement);
    let stone = state_id("minecraft:stone");
    let mut level = NativeLevel::new();
    level.set(pos(3, 68, 0), stone);
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let target_id = store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 1.5,
            y: 68.0,
            z: 0.5,
        },
        EntityData::Item(ItemEntityData::new(
            plain_stack("minecraft:stone", 1).unwrap(),
            0.0,
        )),
    );
    assert!(matches!(
        level.with_level(|level| hit_result_on_move_vector(&source, &store, level, |_, _, _| true)),
        Some(HitResult::Entity {
            target: ProjectileTarget::Stored { entity_id },
            ..
        }) if entity_id == target_id
    ));

    let empty = EntityStore::new(8, EntityIdCounter::new());
    assert!(matches!(
        level.with_level(|level| hit_result_on_move_vector(&source, &empty, level, |_, _, _| true)),
        Some(HitResult::Block {
            pos: hit_pos,
            direction: Direction::West,
            ..
        }) if hit_pos == pos(3, 68, 0)
    ));
}

#[test]
fn block_clip_uses_the_nearer_interaction_shape_face() {
    let source = source(
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        Vec3d {
            x: 0.7,
            y: -2.0,
            z: 0.0,
        },
    );
    let mut level = NativeLevel::new();
    level.set(pos(0, 63, 0), state_id("minecraft:composter"));
    let store = EntityStore::new(90, EntityIdCounter::new());
    let Some(HitResult::Block {
        location,
        direction,
        ..
    }) =
        level.with_level(|level| hit_result_on_move_vector(&source, &store, level, |_, _, _| true))
    else {
        panic!("the diagonal ray crosses the composter wall");
    };
    assert_eq!(direction, Direction::Up);
    assert!(location.y < 64.0, "the location remains the collider hit");
}

#[test]
fn hurting_projectile_applies_acceleration_before_inertia_and_position() {
    let mut entity = source(
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    );
    let mut store = EntityStore::new(9, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let mut state = HurtingProjectileState::new(None, 0.1);
    assert_eq!(
        level.with_level(|level| hurting_projectile::tick(
            &mut entity,
            &mut store,
            level,
            &mut state,
            FIREBALL_SPEC,
            |_, _| true,
        )),
        Ok(None)
    );
    assert!((entity.delta_movement().x - 1.045).abs() < 1e-7);
    assert!((entity.position().x - 1.545).abs() < 1e-7);
    assert!((entity.y_rot() - 18.0).abs() < 0.01);
    assert!(entity.x_rot().abs() < 0.01);
    assert!(state.projectile().has_been_shot());
    assert!(state.projectile().left_owner());
}

#[test]
fn hurting_projectile_leaves_apply_their_visible_hit_effects() {
    for (kind, armoured) in [
        (VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, true),
        (VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, false),
        (VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, true),
        (VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, false),
    ] {
        let mut level = NativeLevel::new();
        let (sender, receiver) = crate::tests::mailbox();
        sender
            .try_send(crate::world::WorldCommand::Admin {
                envelope: crate::admin::AdminEnvelope {
                    operation_id: 1,
                    request_id: "difficulty".into(),
                    principal: "fixture".into(),
                    scope: crate::admin::AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 0,
                },
                request: crate::admin::AdminRequest::ExecuteCommand {
                    command: "difficulty normal".into(),
                },
            })
            .unwrap();
        level.world.tick(receiver.drain());
        let target_kind = if armoured {
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        } else {
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
        };
        let target_data = if armoured {
            let mut store = EntityStore::new(120, EntityIdCounter::new());
            let stand = crate::entity::spawn_armor_stand(
                &mut store,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 2.0,
                },
            )
            .unwrap();
            store.get(stand).unwrap().data().clone()
        } else {
            let mut data = crate::entity::GeneralLivingData::new(target_kind).unwrap();
            data.set_no_ai(true);
            EntityData::GeneralLiving(data)
        };
        let target = level
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                target_kind,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 2.0,
                },
                target_data,
            )
            .0;
        let before = level
            .world
            .stored_entity_for_test(DIMENSION, target)
            .unwrap()
            .living()
            .unwrap()
            .health();
        let data = if kind == VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID {
            EntityData::SmallFireball(crate::entity::SmallFireballData::new(
                HurtingProjectileState::new(None, 0.1),
                plain_stack("minecraft:fire_charge", 1).unwrap(),
            ))
        } else {
            EntityData::WitherSkull(crate::entity::WitherSkullData::new(
                HurtingProjectileState::new(None, 0.1),
                true,
            ))
        };
        let shot = level
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                kind,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                data,
            )
            .0;
        level
            .world
            .stored_entity_mut_for_test(DIMENSION, shot)
            .unwrap()
            .set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            });
        level.world.tick(receiver.drain());
        assert!(level
            .world
            .stored_entity_for_test(DIMENSION, shot)
            .is_none());
        assert!(level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(|event| event.recorded.game_event
                == crate::level_event::VanillaGameEvent::PROJECTILE_LAND
                && event.recorded.source_entity_id == Some(shot)));
        if let Some(target) = level.world.stored_entity_for_test(DIMENSION, target) {
            if armoured {
                assert_eq!(kind, VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID);
                assert_eq!(target.living().unwrap().health(), before);
                assert!(target.remaining_fire_ticks() <= 0);
            } else {
                assert!(target.living().unwrap().health() < before);
                if kind == VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID {
                    assert!(target.remaining_fire_ticks() > 0);
                } else {
                    let effect = target
                        .living()
                        .unwrap()
                        .active_effect("minecraft:wither")
                        .expect("the actual skull hit applies wither");
                    assert!((1..=200).contains(&effect.duration()));
                }
            }
        } else {
            assert!(armoured);
            assert_eq!(kind, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID);
        }
    }
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
    level.world.set_player_position_for_test(
        crate::world::ConnectionId(1),
        Vec3d {
            x: 4.5,
            y: 68.0,
            z: 4.5,
        },
    );
    level.set(pos(0, 68, 2), state_id("minecraft:stone"));
    let shot = level
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
            Vec3d {
                x: 0.5,
                y: 68.0,
                z: 0.5,
            },
            EntityData::DragonFireball(crate::entity::DragonFireballData::new(
                HurtingProjectileState::new(None, 0.1),
            )),
        )
        .0;
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, shot)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        });
    let output = level.world.tick(receiver.drain());
    assert!(output.events.iter().any(|event| matches!(event, crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::LevelEvent(event), .. } if event.event_type == 2006)));
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, shot)
        .is_none());
    assert!(level.world.stored_entity_ids_for_test(DIMENSION).into_iter().any(|id|matches!(level.world.stored_entity_for_test(DIMENSION,id).unwrap().data(),EntityData::AreaEffectCloud(data) if data.radius()==3.0 && data.duration()==600 && data.potion_duration_scale()==0.25)));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event
            == crate::level_event::VanillaGameEvent::PROJECTILE_LAND
            && event.recorded.source_entity_id == Some(shot)));
}

#[test]
fn dangerous_wither_skull_uses_point_seven_three_inertia_without_burning() {
    let mut store = EntityStore::new(123, EntityIdCounter::new());
    let mut skull = Entity::new(
        102,
        Uuid::from_u128(102),
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        EntityData::WitherSkull(crate::entity::WitherSkullData::new(
            HurtingProjectileState::new(None, 0.1),
            true,
        )),
    );
    skull.set_delta_movement(Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| hurting_projectile::tick_entity(&mut skull, &mut store, level))
    };
    assert!((skull.delta_movement().x - 0.803).abs() < 1e-7);
    assert_eq!(skull.remaining_fire_ticks(), 0);
}

#[test]
fn wind_charge_bounding_box_keeps_its_subtype_offset_across_movement() {
    let position = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let mut entity = Entity::new(
        100,
        Uuid::from_u128(100),
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        position,
        EntityData::WindCharge(crate::entity::WindChargeData::new(
            HurtingProjectileState::new(None, 0.0),
            0,
        )),
    );
    entity.set_delta_movement(Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    assert!((entity.bounding_box().min_y - (position.y - 0.15)).abs() < 1e-7);

    let mut store = EntityStore::new(91, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let mut state = HurtingProjectileState::new(None, 0.0);
    assert_eq!(
        level.with_level(|level| hurting_projectile::tick(
            &mut entity,
            &mut store,
            level,
            &mut state,
            crate::entity::hurting_projectile::WIND_CHARGE_SPEC,
            |_, _| true,
        )),
        Ok(None)
    );
    assert!((entity.bounding_box().min_y - (entity.position().y - 0.15)).abs() < 1e-7);
}

#[test]
fn projectile_sweep_does_not_hit_from_inside() {
    let mut inside_store = EntityStore::new(93, EntityIdCounter::new());
    inside_store.spawn(
        VANILLA_ENTITY_TYPE_TNT_ID,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        EntityData::Tnt(crate::entity::TntData::new()),
    );
    let inside = source(
        Vec3d {
            x: 0.5,
            y: 68.5,
            z: 0.5,
        },
        Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    );
    assert_eq!(
        {
            let mut native = NativeLevel::new();
            native.with_level(|level| {
                hit_result_on_move_vector(&inside, &inside_store, level, |_, _, _| true)
            })
        },
        Some(HitResult::Miss)
    );
}

#[test]
fn hurting_projectile_candidate_excludes_interaction_and_no_physics_armor_stand() {
    let mut store = EntityStore::new(94, EntityIdCounter::new());
    let interaction_id = crate::entity::spawn_interaction(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
    )
    .unwrap();
    assert_eq!(
        store.get(interaction_id).unwrap().entity_type_id(),
        VANILLA_ENTITY_TYPE_INTERACTION_ID
    );
    assert!(
        !ProjectileCandidate::Stored(store.get(interaction_id).unwrap()).is_alive_and_pickable()
    );

    let armor_stand_id = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 1.5,
            y: 68.0,
            z: 0.5,
        },
    )
    .unwrap();
    assert_eq!(
        store.get(armor_stand_id).unwrap().entity_type_id(),
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    assert!(
        ProjectileCandidate::Stored(store.get(armor_stand_id).unwrap()).is_alive_and_pickable()
    );
    store.get_mut(armor_stand_id).unwrap().no_gravity = true;
    let armor_stand = ProjectileCandidate::Stored(store.get(armor_stand_id).unwrap());
    assert!(armor_stand.is_alive_and_pickable());
    assert!(!armor_stand.is_alive_and_hurting_projectile_target());
}

#[test]
fn player_attack_deflection_sets_owner_aim_power_and_wind_cooldown_gate() {
    let owner = Uuid::from_u128(0xdef1ec7);
    let mut fireball = source(Vec3d::default(), Vec3d::default());
    assert!(hurting_projectile::deflect_by_player_attack(
        &mut fireball,
        owner,
        90.0,
        0.0,
    ));
    let EntityData::LargeFireball(data) = fireball.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(owner));
    assert_eq!(
        data.projectile().acceleration_power(),
        HurtingProjectileState::INITIAL_ACCELERATION_POWER
    );
    assert!((fireball.delta_movement().x + 1.0).abs() < 1e-6);
    assert!(fireball.delta_movement().z.abs() < 1e-6);
    assert!(fireball.needs_sync());

    let mut wind = Entity::new(
        101,
        Uuid::from_u128(101),
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        Vec3d::default(),
        EntityData::WindCharge(crate::entity::WindChargeData::new(
            HurtingProjectileState::new(None, 0.0),
            1,
        )),
    );
    assert!(!hurting_projectile::deflect_by_player_attack(
        &mut wind, owner, 0.0, 0.0,
    ));
}

#[test]
fn redirectable_projectile_tag_lets_a_player_attack_redirect_any_projectile() {
    let owner = Uuid::from_u128(0x0510_ba11);
    let mut snowball = Entity::new(
        102,
        Uuid::from_u128(102),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        Vec3d::default(),
        EntityData::Snowball(crate::entity::SnowballData::new(
            plain_stack("minecraft:snowball", 1).unwrap(),
        )),
    );
    let mut level = NativeLevel::new();

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:entity_type",
        "minecraft:redirectable_projectile",
        &["minecraft:snowball"],
    )]));
    assert!(level.with_level(
        |level| hurting_projectile::can_deflect_by_player_attack_in_level(&snowball, level)
    ));
    assert!(hurting_projectile::deflect_by_player_attack(
        &mut snowball,
        owner,
        90.0,
        0.0,
    ));
    let EntityData::Snowball(data) = snowball.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(owner));

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:entity_type",
        "minecraft:redirectable_projectile",
        &[],
    )]));
    assert!(!level.with_level(
        |level| hurting_projectile::can_deflect_by_player_attack_in_level(&snowball, level)
    ));
}

#[test]
fn prepared_shulker_bullet_hurt_uses_its_fixed_override_without_damage_tags() {
    let mut store = EntityStore::new(0x5a17_b011, EntityIdCounter::new());
    let target = store.spawn(
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        Vec3d::default(),
        EntityData::ShulkerBullet(crate::entity::ShulkerBulletData::from_saved(
            crate::entity::ProjectileState::default(),
            None,
            None,
            0,
            Vec3d::default(),
        )),
    );
    store.drain_lifecycle_events();
    let hurt = PendingProjectileHurt {
        target: PendingHurtEntity::Stored { entity_id: target },
        damage_type: "minecraft:unattributed_fireball",
        amount: 5.0,
        causing: None,
        direct: None,
        direct_resource_id: "minecraft:small_fireball",
        registry_tags: crate::entity::EntityRegistryTagView::Builtin26_1,
    };
    let damage_type = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == hurt.damage_type)
        .unwrap();
    let mut level = NativeLevel::new();
    let result = level.with_level(|level| {
        hurt_projectile_to_stored(
            &hurt,
            None,
            crate::entity::StoredEntityDamageSource {
                damage_type,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            crate::entity::StoredEntityDamageRules::default(),
            crate::entity::StoredEntityDamageContext::default(),
            &mut store,
            level,
        )
    });
    assert!(matches!(
        result,
        StoredHurtResult::Applied { outcome, .. } if outcome.succeeded
    ));
    assert!(store.get(target).is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.source_entity_id == Some(target)));
}

#[test]
fn deflects_projectiles_tag_bounces_a_fireball() {
    let target_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let target_resource = "minecraft:cow";
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let target = store
        .spawn_general_living(
            target_type,
            Vec3d {
                x: 1.5,
                y: 68.0,
                z: 0.5,
            },
        )
        .unwrap();
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();
    let mut projectile = source(
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    );
    let mut state = HurtingProjectileState::new(None, 0.1);

    level.install_tags(registry_tag_snapshot(&[
        ("minecraft:fluid", "minecraft:water", &[]),
        (
            "minecraft:entity_type",
            "minecraft:deflects_projectiles",
            &[target_resource],
        ),
        (
            "minecraft:entity_type",
            "minecraft:redirectable_projectile",
            &[],
        ),
    ]));
    assert_eq!(
        level.with_level(|level| hurting_projectile::tick(
            &mut projectile,
            &mut store,
            level,
            &mut state,
            FIREBALL_SPEC,
            |candidate, level| { candidate.is_alive_and_hurting_projectile_target_in_level(level) },
        )),
        Ok(None)
    );
    assert!(projectile.delta_movement().x < 0.0);
    assert_eq!(state.acceleration_power(), 0.05);

    let mut projectile = source(
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    );
    let mut state = HurtingProjectileState::new(None, 0.1);
    level.install_tags(registry_tag_snapshot(&[
        ("minecraft:fluid", "minecraft:water", &[]),
        (
            "minecraft:entity_type",
            "minecraft:deflects_projectiles",
            &[],
        ),
        (
            "minecraft:entity_type",
            "minecraft:redirectable_projectile",
            &[],
        ),
    ]));
    assert!(matches!(
        level.with_level(|level| hurting_projectile::tick(
            &mut projectile,
            &mut store,
            level,
            &mut state,
            FIREBALL_SPEC,
            |candidate, level| {
                candidate.is_alive_and_hurting_projectile_target_in_level(level)
            },
        )),
        Ok(Some(HitResult::Entity {
            target: ProjectileTarget::Stored { entity_id },
            deflection: crate::entity::projectile_hit::ProjectileDeflection::None,
            ..
        })) if entity_id == target
    ));
}

#[test]
fn wind_charge_breeze_fixed_no_deflection_does_not_query_live_tag() {
    let mut store = EntityStore::new(83, EntityIdCounter::new());
    let target = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_BREEZE_ID,
            Vec3d {
                x: 1.5,
                y: 68.0,
                z: 0.5,
            },
        )
        .unwrap();
    let mut projectile = Entity::new(
        99,
        Uuid::from_u128(99),
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        EntityData::WindCharge(crate::entity::WindChargeData::new(
            HurtingProjectileState::new(None, 0.1),
            0,
        )),
    );
    projectile.set_delta_movement(Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let mut level = NativeLevel::new();
    level.install_tags(registry_tag_snapshot(&[]));

    assert!(matches!(
        level.with_level(|level| hit_result_on_move_vector(&projectile, &store, level, |_, _, _| true)),
        Some(HitResult::Entity {
            target: ProjectileTarget::Stored { entity_id },
            deflection: crate::entity::projectile_hit::ProjectileDeflection::None,
            ..
        }) if entity_id == target
    ));
}

#[test]
fn explosion_owner_transfer_covers_every_redirectable_hurting_projectile() {
    let old_owner = Uuid::from_u128(0x1010);
    let new_owner = Uuid::from_u128(0x2020);
    let leaves = [
        (
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
            EntityData::LargeFireball(LargeFireballData::new(
                HurtingProjectileState::new(Some(old_owner), 0.2),
                plain_stack("minecraft:fire_charge", 1).unwrap(),
                1,
            )),
        ),
        (
            VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
            EntityData::WindCharge(crate::entity::WindChargeData::new(
                HurtingProjectileState::new(Some(old_owner), 0.0),
                0,
            )),
        ),
        (
            VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
            EntityData::BreezeWindCharge(crate::entity::WindChargeData::new(
                HurtingProjectileState::new(Some(old_owner), 0.0),
                0,
            )),
        ),
    ];

    for (entity_type_id, data) in leaves {
        let mut entity = Entity::new(
            entity_type_id,
            Uuid::from_u128(entity_type_id as u128),
            entity_type_id,
            Vec3d::default(),
            data,
        );
        assert!(hurting_projectile::set_owner_from_explosion(
            &mut entity,
            Some(new_owner)
        ));
        assert_eq!(hurting_projectile_owner(&entity), Some(new_owner));
        assert!(hurting_projectile::set_owner_from_explosion(
            &mut entity,
            None
        ));
        assert_eq!(hurting_projectile_owner(&entity), None);
    }
}

fn hurting_projectile_owner(entity: &Entity) -> Option<Uuid> {
    match entity.data() {
        EntityData::LargeFireball(data) => data.projectile().owner(),
        EntityData::WindCharge(data) | EntityData::BreezeWindCharge(data) => {
            data.projectile().owner()
        }
        _ => unreachable!(),
    }
}

#[test]
fn hurting_projectile_leaves_round_trip_their_saved_state() {
    let owner = Uuid::from_u128(0x5151);
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    store
        .restore(
            Uuid::from_u128(1),
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
            Vec3d {
                x: 1.0,
                y: 68.0,
                z: 1.0,
            },
            EntityData::LargeFireball(LargeFireballData::new(
                HurtingProjectileState::new(Some(owner), 0.25),
                plain_stack("minecraft:fire_charge", 1).unwrap(),
                3,
            )),
        )
        .unwrap();
    for (uuid, entity_type_id, data) in [
        (
            Uuid::from_u128(2),
            VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
            EntityData::WindCharge(crate::entity::WindChargeData::new(
                HurtingProjectileState::new(Some(owner), 0.0),
                2,
            )),
        ),
        (
            Uuid::from_u128(3),
            VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
            EntityData::BreezeWindCharge(crate::entity::WindChargeData::new(
                HurtingProjectileState::new(Some(owner), 0.0),
                0,
            )),
        ),
        (
            Uuid::from_u128(4),
            VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
            EntityData::SmallFireball(crate::entity::SmallFireballData::new(
                HurtingProjectileState::new(Some(owner), 0.15),
                plain_stack("minecraft:fire_charge", 1).unwrap(),
            )),
        ),
        (
            Uuid::from_u128(5),
            VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
            EntityData::DragonFireball(crate::entity::DragonFireballData::new(
                HurtingProjectileState::new(Some(owner), 0.3),
            )),
        ),
        (
            Uuid::from_u128(6),
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            EntityData::WitherSkull(crate::entity::WitherSkullData::new(
                HurtingProjectileState::new(Some(owner), 0.4),
                true,
            )),
        ),
    ] {
        store
            .restore(
                uuid,
                entity_type_id,
                Vec3d {
                    x: 2.0,
                    y: 68.0,
                    z: 2.0,
                },
                data,
            )
            .unwrap();
    }

    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut restored = EntityStore::new(12, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut restored, &document).unwrap();
    assert_eq!(ids.len(), 6);
    let mut kinds: Vec<_> = ids
        .into_iter()
        .map(|id| restored.get(id).unwrap().data().clone())
        .collect();
    kinds.sort_by_key(EntityData::entity_resource_id);
    assert!(kinds.iter().any(|data| matches!(
        data,
        EntityData::BreezeWindCharge(data)
            if data.projectile().owner() == Some(owner) && data.no_deflect_ticks() == 0
    )));
    assert!(kinds.iter().any(|data| matches!(
        data,
        EntityData::LargeFireball(data)
            if data.projectile().owner() == Some(owner)
                && data.projectile().acceleration_power() == 0.25
                && data.explosion_power() == 3
    )));
    assert!(kinds.iter().any(|data| matches!(
        data,
        EntityData::WindCharge(data)
            if data.projectile().owner() == Some(owner) && data.no_deflect_ticks() == 5
    )));
    assert!(kinds.iter().any(|data| matches!(
        data,
        EntityData::SmallFireball(data)
            if data.projectile().owner() == Some(owner)
                && data.projectile().acceleration_power() == 0.15
    )));
    assert!(kinds.iter().any(|data| matches!(
        data,
        EntityData::DragonFireball(data)
            if data.projectile().owner() == Some(owner)
                && data.projectile().acceleration_power() == 0.3
    )));
    assert!(kinds.iter().any(|data| matches!(
        data,
        EntityData::WitherSkull(data)
            if data.projectile().owner() == Some(owner)
                && data.projectile().acceleration_power() == 0.4
                && data.is_dangerous()
    )));
}

#[test]
fn a_silent_dragon_fireball_lands_with_event_data_minus_one() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
    level.world.set_player_position_for_test(
        crate::world::ConnectionId(1),
        Vec3d {
            x: 4.5,
            y: 68.0,
            z: 4.5,
        },
    );
    level.set(pos(0, 68, 2), state_id("minecraft:stone"));
    let shot = level
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
            Vec3d {
                x: 0.5,
                y: 68.0,
                z: 0.5,
            },
            EntityData::DragonFireball(crate::entity::DragonFireballData::new(
                HurtingProjectileState::new(None, 0.1),
            )),
        )
        .0;
    let entity = level
        .world
        .stored_entity_mut_for_test(DIMENSION, shot)
        .unwrap();
    entity.set_silent(true);
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: 2.0,
    });
    let output = level.world.tick(receiver.drain());
    let data = output.events.iter().find_map(|event| match event {
        crate::world::WorldEvent::Send {
            packet: crate::connection::ClientboundPlayPacket::LevelEvent(event),
            ..
        } if event.event_type == 2006 => Some(event.data),
        _ => None,
    });
    assert_eq!(data, Some(-1));
}
