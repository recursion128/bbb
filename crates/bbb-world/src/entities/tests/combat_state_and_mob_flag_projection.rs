use super::*;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};

fn entity_metadata_accessors() -> VanillaEntityMetadataAccessors {
    vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve")
}

fn entity_pose_data(pose_id: i32) -> ProtocolEntityDataValue {
    let accessors = entity_metadata_accessors();
    assert_eq!(accessors.pose_default, VANILLA_ENTITY_POSE_STANDING_ID);
    protocol_pose_data(accessors.pose_data_id, pose_id)
}

fn entity_shared_flags_data(flags: i8) -> ProtocolEntityDataValue {
    protocol_byte_data(entity_metadata_accessors().shared_flags_data_id, flags)
}

fn entity_ticks_frozen_data(ticks: i32) -> ProtocolEntityDataValue {
    let accessors = entity_metadata_accessors();
    assert_eq!(accessors.ticks_frozen_default, 0);
    protocol_int_data(accessors.ticks_frozen_data_id, ticks)
}

fn living_health_data(entity_type_id: i32, health: f32) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated LivingEntity metadata accessors must resolve");
    assert_eq!(accessors.health_default, 1.0);
    protocol_float_data(accessors.health_data_id, health)
}

fn mob_metadata_accessors(entity_type_id: i32) -> VanillaMobMetadataAccessors {
    let accessors = vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
        .expect("generated inherited Mob metadata accessors must resolve");
    assert_eq!(accessors.flags_default, 0);
    accessors
}

fn mob_flags_data(entity_type_id: i32, flags: i8) -> ProtocolEntityDataValue {
    protocol_byte_data(mob_metadata_accessors(entity_type_id).flags_data_id, flags)
}

/// Deliberately sends a valid Mob flags field to a non-Mob to cover data-id collision gating.
fn mob_flags_collision_data(flags: i8) -> ProtocolEntityDataValue {
    mob_flags_data(VANILLA_ENTITY_TYPE_ZOMBIE_ID, flags)
}

fn abstract_villager_unhappy_data(entity_type_id: i32, counter: i32) -> ProtocolEntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(entity_type_id)
            .expect("generated inherited AbstractVillager unhappy accessor must resolve");
    assert_eq!(default, 0);
    protocol_int_data(data_id, counter)
}

/// Deliberately sends an AbstractVillager counter to a non-member to cover data-id/type gating.
fn abstract_villager_unhappy_collision_data(counter: i32) -> ProtocolEntityDataValue {
    abstract_villager_unhappy_data(VANILLA_ENTITY_TYPE_VILLAGER_ID, counter)
}

fn bat_flags_data(entity_type_id: i32, flags: i8) -> ProtocolEntityDataValue {
    let (data_id, default, _resting_mask) =
        vanilla_entity_metadata_bat_flags_accessor_for_entity_type(entity_type_id)
            .expect("generated Bat flags accessor must resolve");
    assert_eq!(default, 0);
    protocol_byte_data(data_id, flags)
}

/// Deliberately sends the Bat flags field to a non-Bat to cover data-id/type gating.
fn bat_flags_collision_data(flags: i8) -> ProtocolEntityDataValue {
    bat_flags_data(VANILLA_ENTITY_TYPE_BAT_ID, flags)
}

fn vex_flags_data(entity_type_id: i32, flags: i8) -> ProtocolEntityDataValue {
    let (data_id, default, _charging_mask) =
        vanilla_entity_metadata_vex_flags_accessor_for_entity_type(entity_type_id)
            .expect("generated Vex flags accessor must resolve");
    assert_eq!(default, 0);
    protocol_byte_data(data_id, flags)
}

/// Deliberately sends the Vex flags field to a non-Vex to cover data-id/type gating.
fn vex_flags_collision_data(flags: i8) -> ProtocolEntityDataValue {
    vex_flags_data(VANILLA_ENTITY_TYPE_VEX_ID, flags)
}

fn wither_invulnerability_data(entity_type_id: i32, ticks: i32) -> ProtocolEntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(entity_type_id)
            .expect("generated WitherBoss invulnerability accessor must resolve");
    assert_eq!(default, 0);
    protocol_int_data(data_id, ticks)
}

/// Deliberately sends the WitherBoss field to a non-Wither to cover data-id/type gating.
fn wither_invulnerability_collision_data(ticks: i32) -> ProtocolEntityDataValue {
    wither_invulnerability_data(VANILLA_ENTITY_TYPE_WITHER_ID, ticks)
}

fn wither_side_head_target_data(target_b: i32, target_c: i32) -> Vec<ProtocolEntityDataValue> {
    let accessors =
        crate::entities::schema::vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_WITHER_ID,
        )
        .expect("generated WitherBoss side-head target accessors must resolve");
    vec![
        protocol_int_data(accessors.target_b_data_id, target_b),
        protocol_int_data(accessors.target_c_data_id, target_c),
    ]
}

#[test]
fn entity_model_sources_project_death_animation_counter() {
    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 60)
            .unwrap()
    };
    let set_health = |store: &mut WorldStore, health: f32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id: 60,
            values: vec![living_health_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, health)],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // A healthy living entity is not dying.
    assert!(set_health(&mut store, 4.0));
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert!(!source(&store, 0.0).has_red_overlay);

    // Vanilla isDeadOrDying(): health <= 0 begins the death animation. Before the
    // first tickDeath, deathTime is 0, so the model is upright and not yet red.
    assert!(set_health(&mut store, 0.0));
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert!(!source(&store, 0.0).has_red_overlay);

    // tickDeath increments deathTime each client tick; the projected value lerps
    // by the partial tick (entity.deathTime + partialTick) and drives the red
    // overlay (hasRedOverlay = deathTime > 0).
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 0.0).death_time, 1.0);
    assert_eq!(source(&store, 0.5).death_time, 1.5);
    assert!(source(&store, 0.0).has_red_overlay);

    store.advance_entity_client_animations(10);
    assert_eq!(source(&store, 0.0).death_time, 11.0);

    // The counter caps at 20 (vanilla removes the entity at deathTime >= 20).
    store.advance_entity_client_animations(20);
    assert_eq!(source(&store, 0.0).death_time, 20.0);
    store.advance_entity_client_animations(5);
    assert_eq!(source(&store, 0.0).death_time, 20.0);

    // Restoring health clears the death animation (the model stands back up).
    assert!(set_health(&mut store, 6.0));
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert!(!source(&store, 0.0).has_red_overlay);
}

#[test]
fn simulation_distance_only_gates_living_death_ticks_by_chunk_distance() {
    let mut store = WorldStore::new();
    let mut login = protocol_play_login(99);
    login.simulation_distance = 2;
    store.apply_login(&login);
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 15.9,
            y: 64.0,
            z: 15.9,
        },
        ..crate::LocalPlayerPoseState::default()
    });
    for pos in [
        crate::ChunkPos { x: 2, z: -1 },
        crate::ChunkPos { x: 3, z: 0 },
        crate::ChunkPos { x: -1, z: -2 },
    ] {
        let mut chunk = empty_test_chunk();
        chunk.pos = pos;
        store.insert_decoded_chunk(chunk);
    }
    for (id, position) in [
        (62, [47.9, 64.0, -16.0]),
        (63, [48.0, 64.0, 0.0]),
        (64, [-0.01, 64.0, -16.01]),
    ] {
        store.apply_add_entity(ProtocolAddEntity {
            position: ProtocolVec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            ..protocol_add_entity_with_type(id, VANILLA_ENTITY_TYPE_CHICKEN_ID)
        });
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![living_health_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, 0.0)],
        }));
    }
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 63, yaw: 0.0 }));

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 62).death_time, 1.0);
    assert_eq!(source(&store, 64).death_time, 1.0);
    assert_eq!(source(&store, 63).death_time, 0.0);
    assert_eq!(source(&store, 63).age_ticks, 1);
    assert_eq!(
        store
            .probe_entity_camera_hurt_state(63, 0.0)
            .unwrap()
            .hurt_time,
        9.0
    );

    let mut player_pose = store.local_player_pose().unwrap();
    player_pose.position.x = 48.0;
    player_pose.position.z = 0.0;
    store.set_local_player_pose(player_pose);
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 62).death_time, 2.0);
    assert_eq!(source(&store, 63).death_time, 1.0);
    assert_eq!(source(&store, 64).death_time, 1.0);

    store.apply_simulation_distance(bbb_protocol::packets::SetSimulationDistance { distance: 0 });
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 62).death_time, 2.0);
    assert_eq!(source(&store, 63).death_time, 2.0);
}

#[test]
fn entity_model_sources_project_ender_dragon_death_time() {
    let source = |store: &WorldStore, partial: f32| {
        let position = store.entities.transform(61).unwrap().position;
        store
            .entities
            .model_source(
                61,
                position,
                partial,
                false,
                &store.registries,
                &store.items.default_item_max_damage,
                &store.items.default_item_equipment_assets,
                store.items.equipment_asset_generation,
                &store.items.equipment_asset_names,
                &store.items.default_item_equipment_slots,
            )
            .unwrap()
    };
    let set_health = |store: &mut WorldStore, health: f32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id: 61,
            values: vec![living_health_data(
                VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
                health,
            )],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    ));
    assert!(set_health(&mut store, 200.0));
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert_eq!(source(&store, 0.0).ender_dragon_death_time, 0.0);

    // Vanilla `EnderDragonRenderer.extractRenderState`: `dragonDeathTime > 0 ?
    // dragonDeathTime + partialTicks : 0`. The dragon's counter is distinct from
    // the generic 20-tick living death flip and runs to 200 ticks.
    assert!(set_health(&mut store, 0.0));
    assert_eq!(source(&store, 0.0).ender_dragon_death_time, 0.0);
    store.advance_entity_client_animations(2);
    assert_eq!(source(&store, 0.25).death_time, 0.0);
    assert_eq!(source(&store, 0.25).ender_dragon_death_time, 2.25);

    store.advance_entity_client_animations(250);
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert_eq!(source(&store, 0.0).ender_dragon_death_time, 200.0);

    assert!(set_health(&mut store, 120.0));
    assert_eq!(source(&store, 0.0).death_time, 0.0);
    assert_eq!(source(&store, 0.0).ender_dragon_death_time, 0.0);
}

#[test]
fn entity_model_sources_project_full_freeze_for_living_entities() {
    let fully_frozen = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_fully_frozen
    };
    let set_ticks_frozen = |store: &mut WorldStore, id: i32, ticks: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![entity_ticks_frozen_data(ticks)],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!fully_frozen(&store, 70));

    // Vanilla Entity.isFullyFrozen(): ticksFrozen >= getTicksRequiredToFreeze()
    // (140). One tick below the threshold is not yet fully frozen.
    assert!(set_ticks_frozen(&mut store, 70, 139));
    assert!(!fully_frozen(&store, 70));
    assert!(set_ticks_frozen(&mut store, 70, 140));
    assert!(fully_frozen(&store, 70));

    // A non-living entity (boat) never counts as fully frozen even past the
    // threshold: only LivingEntityRenderer shakes.
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_ticks_frozen(&mut store, 71, 200));
    assert!(!fully_frozen(&store, 71));
}

#[test]
fn entity_model_sources_project_auto_spin_attack_flag() {
    let living_flags = living_entity_metadata_accessors();

    let auto_spin = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_auto_spin_attack
    };
    let set_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![living_entity_flags_data(flags)],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // A living entity with no living-entity flags is not spinning.
    assert!(!auto_spin(&store, 72));

    // Vanilla LivingEntity.isAutoSpinAttack(): (DATA_LIVING_ENTITY_FLAGS & 4) != 0.
    // The bit is detected even alongside other living-entity flags.
    assert!(set_flags(
        &mut store,
        72,
        living_flags.spin_attack_mask | living_flags.is_using_mask,
    ));
    assert!(auto_spin(&store, 72));

    // Clearing the spin bit (other flags still set) stops the spin.
    assert!(set_flags(&mut store, 72, living_flags.is_using_mask));
    assert!(!auto_spin(&store, 72));

    // A non-living entity (boat) never spins even with a stray spin-attack bit at
    // the living-entity-flags id: only LivingEntityRenderer reads it.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(!set_flags(&mut store, 73, living_flags.spin_attack_mask));
    assert!(!auto_spin(&store, 73));
}

#[test]
fn entity_model_sources_project_using_item_flags() {
    let living_flags = living_entity_metadata_accessors();

    let using = |store: &WorldStore, id: i32| {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (source.is_using_item, source.use_item_off_hand)
    };
    let set_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![living_entity_flags_data(flags)],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    // No flags → not using.
    assert_eq!(using(&store, 80), (false, false));

    // Using the main hand sets IS_USING but not the off-hand bit (detected alongside other flags).
    assert!(set_flags(
        &mut store,
        80,
        living_flags.is_using_mask | living_flags.spin_attack_mask,
    ));
    assert_eq!(using(&store, 80), (true, false));

    // Using the off hand sets both bits.
    assert!(set_flags(
        &mut store,
        80,
        living_flags.is_using_mask | living_flags.off_hand_mask,
    ));
    assert_eq!(using(&store, 80), (true, true));

    // Clearing the using bit stops it (a stray off-hand bit alone is not "using").
    assert!(set_flags(&mut store, 80, living_flags.off_hand_mask));
    assert_eq!(using(&store, 80), (false, true));

    // A non-living entity (boat) never reads the flags byte.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(!set_flags(
        &mut store,
        81,
        living_flags.is_using_mask | living_flags.off_hand_mask,
    ));
    assert_eq!(using(&store, 81), (false, false));
}

#[test]
fn entity_model_sources_project_main_arm_left_for_item_in_hand_layer() {
    let main_arm_left = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .main_arm_left
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(!main_arm_left(&store, 82));

    // Vanilla `Avatar.getMainArm` reads DATA_PLAYER_MAIN_HAND (HumanoidArm.LEFT id 0).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![protocol_humanoid_arm_data(
            VANILLA_AVATAR_MAIN_HAND_DATA_ID,
            VANILLA_HUMANOID_ARM_LEFT_ID,
        )],
    }));
    assert!(main_arm_left(&store, 82));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![protocol_humanoid_arm_data(
            VANILLA_AVATAR_MAIN_HAND_DATA_ID,
            VANILLA_HUMANOID_ARM_RIGHT_ID,
        )],
    }));
    assert!(!main_arm_left(&store, 82));

    store.apply_add_entity(protocol_add_entity_with_type(
        83,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    let zombie_accessors = mob_metadata_accessors(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert!(!main_arm_left(&store, 83));
    // Vanilla `Mob.getMainArm`: the generated left-handed flag flips the default RIGHT arm.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 83,
        values: vec![mob_flags_data(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            zombie_accessors.left_handed_mask,
        )],
    }));
    assert!(main_arm_left(&store, 83));

    // AllayRenderer and CopperGolemRenderer add ItemInHandLayer directly rather than through
    // HumanoidMobRenderer. Both still extract ArmedEntityRenderState from a Mob, so the generated
    // renderer membership must project their inherited left-handed flag as well.
    for (id, entity_type_id) in [
        (85, VANILLA_ENTITY_TYPE_ALLAY_ID),
        (86, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID),
    ] {
        let accessors = mob_metadata_accessors(entity_type_id);
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![mob_flags_data(entity_type_id, accessors.left_handed_mask)],
        }));
        assert!(main_arm_left(&store, id));
    }

    // ArmorStandRenderer also owns an ItemInHandLayer, but ArmorStand is a LivingEntity rather
    // than a Mob and data id 15 is its own flags byte, not Mob.DATA_MOB_FLAGS_ID.
    store.apply_add_entity(protocol_add_entity_with_type(
        87,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 87,
        values: vec![mob_flags_collision_data(zombie_accessors.left_handed_mask)],
    }));
    assert!(!main_arm_left(&store, 87));

    // Non-humanoid/non-living entities do not treat data id 15 as a main-arm source.
    store.apply_add_entity(protocol_add_entity_with_type(
        84,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 84,
        values: vec![mob_flags_collision_data(zombie_accessors.left_handed_mask)],
    }));
    assert!(!main_arm_left(&store, 84));
}

#[test]
fn entity_model_sources_project_aggressive_for_renderer_extractors() {
    let aggressive = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_aggressive
    };
    let set_mob_flags = |store: &mut WorldStore, id: i32, entity_type_id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![mob_flags_data(entity_type_id, flags)],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    let zombie_accessors = mob_metadata_accessors(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    // A zombie with no mob flags is calm.
    assert!(!aggressive(&store, 80));

    // Vanilla Mob.isAggressive(): (DATA_MOB_FLAGS_ID & 4) != 0, detected alongside other flags.
    assert!(set_mob_flags(
        &mut store,
        80,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        zombie_accessors.aggressive_mask | zombie_accessors.left_handed_mask,
    ));
    assert!(aggressive(&store, 80));
    // Clearing the aggressive bit (left-handed still set) returns to calm.
    assert!(set_mob_flags(
        &mut store,
        80,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        zombie_accessors.left_handed_mask,
    ));
    assert!(!aggressive(&store, 80));

    // These renderer paths assign `state.isAggressive = entity.isAggressive()` directly, including
    // the skeleton and zombified-piglin families that the old hand-maintained table missed.
    for (id, entity_type_id) in [
        (81, VANILLA_ENTITY_TYPE_SKELETON_ID),
        (82, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID),
    ] {
        let accessors = mob_metadata_accessors(entity_type_id);
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        assert!(set_mob_flags(
            &mut store,
            id,
            entity_type_id,
            accessors.aggressive_mask,
        ));
        assert!(aggressive(&store, id));
    }

    // GiantMobRenderer uses ZombieRenderState but never assigns its isAggressive field; chicken has
    // no renderer consumer for the flag either. Both stay at the vanilla render-state default.
    for (id, entity_type_id) in [
        (83, VANILLA_ENTITY_TYPE_GIANT_ID),
        (84, VANILLA_ENTITY_TYPE_CHICKEN_ID),
    ] {
        let accessors = mob_metadata_accessors(entity_type_id);
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        assert!(set_mob_flags(
            &mut store,
            id,
            entity_type_id,
            accessors.aggressive_mask,
        ));
        assert!(!aggressive(&store, id));
    }
}

#[test]
fn entity_model_sources_project_aggressive_for_piglin_and_illager_arm_poses() {
    // The aggressive flag also drives the piglin/brute `ATTACKING_WITH_MELEE_WEAPON`, the vindicator /
    // pillager `ATTACKING` arm pose, and the illusioner `BOW_AND_ARROW` aim. IllagerRenderer extracts
    // the flag for every illager render state; EvokerModel simply does not consume it for arm pose.
    let aggressive = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_aggressive
    };

    let mut store = WorldStore::new();
    // Types whose rendered arm pose reads `isAggressive` → projected.
    for (id, type_id) in [
        (90, VANILLA_ENTITY_TYPE_PIGLIN_ID),
        (91, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID),
        (92, VANILLA_ENTITY_TYPE_VINDICATOR_ID),
        (93, VANILLA_ENTITY_TYPE_ILLUSIONER_ID),
        (94, VANILLA_ENTITY_TYPE_EVOKER_ID),
        (95, VANILLA_ENTITY_TYPE_PILLAGER_ID),
    ] {
        let accessors = mob_metadata_accessors(type_id);
        store.apply_add_entity(protocol_add_entity_with_type(id, type_id));
        assert!(
            !aggressive(&store, id),
            "calm before the flag: type {type_id}"
        );
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![mob_flags_data(type_id, accessors.aggressive_mask)],
        }));
        assert!(
            aggressive(&store, id),
            "aggressive projects: type {type_id}"
        );
    }
}

#[test]
fn entity_model_sources_project_spellcaster_illager_casting() {
    // Vanilla SpellcasterIllager.DATA_SPELL_CASTING_ID stores the spell ordinal;
    // EvokerRenderer and IllusionerRenderer copy `isCastingSpell()` into their render states.
    let (casting_data_id, casting_default) =
        super::schema::vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_EVOKER_ID,
        )
        .expect("generated SpellcasterIllager casting accessor");
    assert_eq!(casting_default, 0);

    let casting = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .illager_spellcasting
    };

    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (96, VANILLA_ENTITY_TYPE_EVOKER_ID),
        (97, VANILLA_ENTITY_TYPE_ILLUSIONER_ID),
        (98, VANILLA_ENTITY_TYPE_VINDICATOR_ID),
        (99, VANILLA_ENTITY_TYPE_PILLAGER_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        assert!(!casting(&store, id));
        assert_eq!(
            store.apply_set_entity_data(ProtocolSetEntityData {
                id,
                values: vec![protocol_byte_data(casting_data_id, 2)],
            }),
            matches!(
                entity_type_id,
                VANILLA_ENTITY_TYPE_EVOKER_ID | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
            )
        );
    }

    assert!(casting(&store, 96));
    assert!(casting(&store, 97));
    assert!(
        !casting(&store, 98),
        "vindicator has no SpellcasterIllager accessor or casting-state renderer assignment"
    );
    assert!(
        !casting(&store, 99),
        "pillager data id 17 is the BOOLEAN crossbow flag, not the spell byte"
    );
}

#[test]
fn entity_model_sources_project_villager_unhappy() {
    // Vanilla VillagerRenderer and WanderingTraderRenderer project
    // `AbstractVillager.getUnhappyCounter() > 0` through the inherited generated accessor.

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        86,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        87,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        88,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        89,
        VANILLA_ENTITY_TYPE_PANDA_ID,
    ));

    assert!(!source(&store, 86).villager_unhappy);
    assert!(!source(&store, 87).villager_unhappy);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 86,
        values: vec![abstract_villager_unhappy_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            12,
        )],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 87,
        values: vec![abstract_villager_unhappy_data(
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            1,
        )],
    }));
    assert!(!store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![abstract_villager_unhappy_collision_data(12)],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 89,
        values: vec![abstract_villager_unhappy_collision_data(12)],
    }));

    assert!(source(&store, 86).villager_unhappy);
    assert!(source(&store, 87).villager_unhappy);
    assert!(
        !source(&store, 88).villager_unhappy,
        "zombie villagers do not use AbstractVillagerRenderState"
    );
    assert!(
        !source(&store, 89).villager_unhappy,
        "PandaRenderer uses the same assignment with Panda.UNHAPPY_COUNTER, not the AbstractVillager accessor"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 86,
        values: vec![abstract_villager_unhappy_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            0,
        )],
    }));
    assert!(!source(&store, 86).villager_unhappy);
}

#[test]
fn entity_model_sources_project_enderman_carrying_and_creepy() {
    const GRASS_BLOCK_STATE_ID: i32 = 9;
    let accessors = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    )
    .expect("generated EnderMan render metadata accessors");

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };
    let set_carry_and_creepy =
        |store: &mut WorldStore, id: i32, block: Option<i32>, creepy: bool| {
            store.apply_set_entity_data(ProtocolSetEntityData {
                id,
                values: vec![
                    ProtocolEntityDataValue {
                        data_id: accessors.carried_block_state_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                        value: EntityDataValueKind::OptionalBlockState(block),
                    },
                    ProtocolEntityDataValue {
                        data_id: accessors.creepy_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                        value: EntityDataValueKind::Boolean(creepy),
                    },
                ],
            })
        };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    ));
    // A freshly spawned enderman carries nothing and is not creepy.
    let calm = source(&store, 90);
    assert!(!calm.enderman_carrying);
    assert_eq!(calm.enderman_carried_block, None);
    assert!(!calm.enderman_creepy);

    // A present carried block (non-zero state id → `Some`) poses the arms; `isCreepy` true
    // drops the head. The carried block's model state resolves through the vanilla block-state registry.
    assert!(set_carry_and_creepy(
        &mut store,
        90,
        Some(GRASS_BLOCK_STATE_ID),
        true
    ));
    let primed = source(&store, 90);
    assert!(primed.enderman_carrying);
    assert_eq!(
        primed.enderman_carried_block,
        Some(EntityBlockModelState {
            name: "minecraft:grass_block".to_string(),
            properties: BTreeMap::from([("snowy".to_string(), "false".to_string())]),
        })
    );
    assert_eq!(
        store.enderman_carried_block_state(90),
        primed.enderman_carried_block
    );
    assert!(primed.enderman_creepy);

    // Dropping the block (empty optional) and clearing creepy returns to rest.
    assert!(set_carry_and_creepy(&mut store, 90, None, false));
    let rest = source(&store, 90);
    assert!(!rest.enderman_carrying);
    assert_eq!(rest.enderman_carried_block, None);
    assert_eq!(store.enderman_carried_block_state(90), None);
    assert!(!rest.enderman_creepy);

    // A zombie does not define the enderman accessors, so even if the same data ids arrive
    // the projection is gated out and both flags stay false.
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    assert!(!set_carry_and_creepy(
        &mut store,
        91,
        Some(GRASS_BLOCK_STATE_ID),
        true
    ));
    let zombie = source(&store, 91);
    assert!(!zombie.enderman_carrying);
    assert_eq!(zombie.enderman_carried_block, None);
    assert_eq!(store.enderman_carried_block_state(91), None);
    assert!(!zombie.enderman_creepy);
}

#[test]
fn entity_model_sources_project_bat_resting_and_flying_start_phase() {
    // Vanilla Bat.DATA_ID_FLAGS (BYTE); FLAG_RESTING.
    let (_, default, resting_mask) =
        vanilla_entity_metadata_bat_flags_accessor_for_entity_type(VANILLA_ENTITY_TYPE_BAT_ID)
            .expect("generated Bat flags accessor must resolve");
    let state = |store: &WorldStore, id: i32, partial_tick: f32| {
        store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == id)
            .map(|source| (source.bat_resting, source.bat_flying_seconds))
            .unwrap()
    };
    let set_flags = |store: &mut WorldStore, id: i32, value: ProtocolEntityDataValue| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![value],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_BAT_ID,
    ));
    // Before its first client tick vanilla has not started either AnimationState, even though the
    // default flags select flying.
    assert_eq!(state(&store, 70, 0.0), (false, -1.0));
    store.advance_entity_client_animations(1);
    assert_eq!(state(&store, 70, 0.0), (false, 0.0));
    assert_eq!(state(&store, 70, 0.5), (false, 0.025));
    store.advance_entity_client_animations(3);
    assert_eq!(state(&store, 70, 0.0), (false, 0.15));

    // Setting Bat.FLAG_RESTING projects the hanging pose; clearing it
    // returns to flying. `setupAnimationStates` observes each transition on the next client tick,
    // stopping the old flying timer and restarting it from zero after takeoff.
    assert!(set_flags(
        &mut store,
        70,
        bat_flags_data(VANILLA_ENTITY_TYPE_BAT_ID, resting_mask)
    ));
    assert_eq!(state(&store, 70, 0.0), (true, 0.15));
    store.advance_entity_client_animations(1);
    assert_eq!(state(&store, 70, 0.0), (true, -1.0));
    assert!(set_flags(
        &mut store,
        70,
        bat_flags_data(VANILLA_ENTITY_TYPE_BAT_ID, default)
    ));
    assert_eq!(state(&store, 70, 0.0), (false, -1.0));
    store.advance_entity_client_animations(1);
    assert_eq!(state(&store, 70, 0.0), (false, 0.0));

    // A chicken carries no bat flags byte; a stray bit at the same data id never reaches its
    // render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!set_flags(
        &mut store,
        71,
        bat_flags_collision_data(resting_mask)
    ));
    assert_eq!(state(&store, 71, 0.0), (false, -1.0));
}

#[test]
fn entity_model_sources_project_vex_charging_from_flags() {
    // Vanilla Vex.DATA_FLAGS_ID (BYTE); FLAG_IS_CHARGING.
    let (_, default, charging_mask) =
        vanilla_entity_metadata_vex_flags_accessor_for_entity_type(VANILLA_ENTITY_TYPE_VEX_ID)
            .expect("generated Vex flags accessor must resolve");
    let charging = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .vex_charging
    };
    let set_flags = |store: &mut WorldStore, id: i32, value: ProtocolEntityDataValue| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![value],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_VEX_ID,
    ));
    assert!(!charging(&store, 72));
    assert!(set_flags(
        &mut store,
        72,
        vex_flags_data(VANILLA_ENTITY_TYPE_VEX_ID, charging_mask)
    ));
    assert!(charging(&store, 72));
    assert!(set_flags(
        &mut store,
        72,
        vex_flags_data(VANILLA_ENTITY_TYPE_VEX_ID, default)
    ));
    assert!(!charging(&store, 72));

    // A chicken carries no Vex flags accessor; a stray bit at the same data id remains idle.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!set_flags(
        &mut store,
        73,
        vex_flags_collision_data(charging_mask)
    ));
    assert!(!charging(&store, 73));
}

#[test]
fn entity_model_sources_project_wither_invulnerable_ticks() {
    // Vanilla WitherBoss.DATA_ID_INV (INT): the spawn-invulnerability countdown.
    let (_, default) = vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_WITHER_ID,
    )
    .expect("generated WitherBoss invulnerability accessor must resolve");

    let ticks = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .wither_invulnerable_ticks
    };
    let set_inv = |store: &mut WorldStore, id: i32, value: ProtocolEntityDataValue| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![value],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_WITHER_ID,
    ));
    // A fully-spawned wither (DATA_ID_INV = 0) projects 0.0.
    assert_eq!(ticks(&store, 80, 0.0), 0.0);
    // A freshly-summoned wither (220) lerps `invulnerableTicks - partialTicks`.
    assert!(set_inv(
        &mut store,
        80,
        wither_invulnerability_data(VANILLA_ENTITY_TYPE_WITHER_ID, 220)
    ));
    assert!((ticks(&store, 80, 0.0) - 220.0).abs() < 1.0e-6);
    assert!((ticks(&store, 80, 0.5) - 219.5).abs() < 1.0e-6);
    // Clearing it returns to 0.0 (a non-positive countdown is not lerped).
    assert!(set_inv(
        &mut store,
        80,
        wither_invulnerability_data(VANILLA_ENTITY_TYPE_WITHER_ID, default)
    ));
    assert_eq!(ticks(&store, 80, 0.5), 0.0);

    // A chicken carries no DATA_ID_INV accessor; a stray int at the same data id never reaches its
    // render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!set_inv(
        &mut store,
        81,
        wither_invulnerability_collision_data(220)
    ));
    assert_eq!(ticks(&store, 81, 0.0), 0.0);
}

#[test]
fn entity_model_sources_project_wither_side_head_tracking() {
    // Vanilla WitherBoss.DATA_TARGET_B/C: side-head target entity ids.

    let add_at = |id, entity_type_id, position: [f64; 3], y_rot| ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
        entity_type_id,
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot,
        y_head_rot: y_rot,
        data: 0,
    };
    let sync_position = |store: &mut WorldStore, id, position: [f64; 3], y_rot| {
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id,
            position: ProtocolVec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot,
            x_rot: 0.0,
            on_ground: true,
        })
    };
    let heads = |store: &WorldStore, id, partial| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (source.wither_x_head_rots, source.wither_y_head_rots)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(add_at(
        80,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        [0.0, 64.0, 0.0],
        0.0,
    ));
    store.apply_add_entity(add_at(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
        0.0,
    ));
    store.apply_add_entity(add_at(
        82,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
        0.0,
    ));

    // Wither side-head origins for bodyRot=0 and scale=1:
    //   right (index 1): (x + 1.3, y + 2.2, z)
    //   left  (index 2): (x - 1.3, y + 2.2, z)
    let right_head = [1.3, 66.2, 0.0];
    let left_head = [-1.3, 66.2, 0.0];
    let right_eye = f64::from(store.probe_entity_camera_pose(81).unwrap().eye_height);
    let left_eye = f64::from(store.probe_entity_camera_pose(82).unwrap().eye_height);
    assert!(sync_position(
        &mut store,
        81,
        [
            right_head[0] + 10.0,
            right_head[1] - right_eye,
            right_head[2]
        ],
        0.0,
    ));
    assert!(sync_position(
        &mut store,
        82,
        [
            left_head[0] - 10.0,
            left_head[1] + 10.0 - left_eye,
            left_head[2]
        ],
        0.0,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: wither_side_head_target_data(81, 82),
    }));

    assert_eq!(heads(&store, 80, 0.5), ([0.0; 2], [0.0; 2]));
    store.advance_entity_client_animations(1);
    let (x_rots, y_rots) = heads(&store, 80, 0.0);
    assert!(
        x_rots[0].abs() < 1.0e-6,
        "right target is level with the head"
    );
    assert_eq!(
        x_rots[1], -40.0,
        "left target pitch wants -45 degrees but rotlerp clamps to 40 degrees on the first tick"
    );
    assert_eq!(y_rots, [-10.0, 10.0]);
    assert_eq!(
        heads(&store, 80, 0.75),
        (x_rots, y_rots),
        "26.1 WitherBossRenderer copies current side-head arrays without partial lerp"
    );

    store.advance_entity_client_animations(1);
    let (x_rots, y_rots) = heads(&store, 80, 0.0);
    assert_eq!(x_rots[1], -45.0);
    assert_eq!(y_rots, [-20.0, 20.0]);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: wither_side_head_target_data(0, 0),
    }));
    store.advance_entity_client_animations(1);
    let (x_rots, y_rots) = heads(&store, 80, 0.0);
    assert_eq!(
        x_rots[1], -45.0,
        "no-target fallback leaves pitch at the last tracked value"
    );
    assert_eq!(
        y_rots,
        [-10.0, 10.0],
        "no-target fallback lerps yaw back toward bodyRot=0"
    );

    store.apply_add_entity(add_at(
        83,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        [4.0, 64.0, 0.0],
        30.0,
    ));
    store.advance_entity_client_animations(1);
    assert_eq!(
        heads(&store, 83, 0.0).1,
        [10.0, 10.0],
        "a wither with no target lerps both side-head yaws toward yBodyRot"
    );
}

#[test]
fn entity_model_sources_project_bee_stinger_from_flags() {
    // Vanilla Bee.DATA_FLAGS_ID and FLAG_HAS_STUNG are generated from Bee.java.
    let accessors =
        vanilla_entity_metadata_bee_accessors_for_entity_type(VANILLA_ENTITY_TYPE_BEE_ID)
            .expect("generated Bee metadata accessors");

    let has_stinger = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .bee_has_stinger
    };
    let set_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(flags),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_BEE_ID,
    ));
    // A fresh bee has not stung, so it keeps its stinger.
    assert!(has_stinger(&store, 72));
    // Setting Bee.hasStung (DATA_FLAGS_ID & 4) hides the stinger; clearing it restores it.
    assert!(set_flags(&mut store, 72, accessors.has_stung_mask));
    assert!(!has_stinger(&store, 72));
    assert!(set_flags(&mut store, 72, 0));
    assert!(has_stinger(&store, 72));

    // A non-bee keeps the `true` stinger default regardless of a stray bit at the same data id.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!set_flags(&mut store, 73, accessors.has_stung_mask));
    assert!(has_stinger(&store, 73));
}

#[test]
fn entity_model_sources_gate_crouch_pose_on_the_player() {
    let crouching = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_crouching
    };
    let set_pose = |store: &mut WorldStore, id: i32, pose: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![entity_pose_data(pose)],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    // A standing player is not crouching.
    assert!(!crouching(&store, 74));
    // Vanilla Pose.CROUCHING marks the player sneaking; standing again clears it.
    assert!(set_pose(&mut store, 74, VANILLA_ENTITY_POSE_CROUCHING_ID));
    assert!(crouching(&store, 74));
    assert!(set_pose(&mut store, 74, VANILLA_ENTITY_POSE_STANDING_ID));
    assert!(!crouching(&store, 74));

    // A non-player entity is never crouched, even with a CROUCHING pose: only the player model
    // has the `HumanoidModel.setupAnim` crouch.
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_pose(&mut store, 75, VANILLA_ENTITY_POSE_CROUCHING_ID));
    assert!(!crouching(&store, 75));
}

#[test]
fn entity_model_sources_project_feline_crouch_sprint_and_cat_sitting() {
    let entity_accessors = entity_metadata_accessors();
    let tamable_accessors = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CAT_ID,
    )
    .expect("generated TamableAnimal metadata accessors must resolve");

    let state = |store: &WorldStore, id: i32| {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (
            source.feline_is_crouching,
            source.feline_is_sprinting,
            source.feline_is_sitting,
        )
    };
    let set_data = |store: &mut WorldStore, id: i32, values: Vec<ProtocolEntityDataValue>| {
        store.apply_set_entity_data(ProtocolSetEntityData { id, values })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_CAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_OCELOT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert_eq!(state(&store, 76), (false, false, false));
    assert_eq!(state(&store, 77), (false, false, false));

    assert!(set_data(
        &mut store,
        76,
        vec![
            entity_pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID),
            entity_shared_flags_data(entity_accessors.sprinting_mask),
            protocol_byte_data(
                tamable_accessors.flags_data_id,
                tamable_accessors.sitting_mask,
            ),
        ],
    ));
    assert_eq!(state(&store, 76), (true, true, true));

    assert!(set_data(
        &mut store,
        77,
        vec![entity_pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID)],
    ));
    assert!(!set_data(
        &mut store,
        77,
        vec![protocol_byte_data(
            tamable_accessors.flags_data_id,
            tamable_accessors.sitting_mask,
        )],
    ));
    assert_eq!(state(&store, 77), (true, false, false));

    assert!(set_data(
        &mut store,
        78,
        vec![
            entity_pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID),
            entity_shared_flags_data(entity_accessors.sprinting_mask),
        ],
    ));
    assert!(!set_data(
        &mut store,
        78,
        vec![protocol_byte_data(
            tamable_accessors.flags_data_id,
            tamable_accessors.sitting_mask,
        )],
    ));
    assert_eq!(state(&store, 78), (false, false, false));

    assert!(set_data(
        &mut store,
        76,
        vec![
            entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID),
            entity_shared_flags_data(entity_accessors.shared_flags_default),
            protocol_byte_data(
                tamable_accessors.flags_data_id,
                tamable_accessors.flags_default
            ),
        ],
    ));
    assert_eq!(state(&store, 76), (false, false, false));
}
