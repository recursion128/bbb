use super::*;

#[test]
fn entity_model_sources_project_death_animation_counter() {
    const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
    const FLOAT_SERIALIZER_ID: i32 = 3;

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
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_ENTITY_HEALTH_DATA_ID,
                serializer_id: FLOAT_SERIALIZER_ID,
                value: EntityDataValueKind::Float(health),
            }],
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
fn entity_model_sources_project_ender_dragon_death_time() {
    const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
    const FLOAT_SERIALIZER_ID: i32 = 3;

    let source = |store: &WorldStore, partial: f32| {
        let position = store.entities.transform(61).unwrap().position;
        store
            .entities
            .model_source(
                61,
                position,
                partial,
                &store.registries,
                &store.items.default_item_max_damage,
                &store.items.default_item_armor_materials,
                &store.items.default_item_equipment_slots,
                &store.items.default_llama_body_decor_colors,
                &store.items.default_nautilus_body_armor_materials,
                &store.items.default_horse_body_armor_materials,
                &store.items.default_wolf_body_armor_materials,
            )
            .unwrap()
    };
    let set_health = |store: &mut WorldStore, health: f32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id: 61,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_ENTITY_HEALTH_DATA_ID,
                serializer_id: FLOAT_SERIALIZER_ID,
                value: EntityDataValueKind::Float(health),
            }],
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
    const VANILLA_ENTITY_TICKS_FROZEN_DATA_ID: u8 = 7;
    const INT_SERIALIZER_ID: i32 = 1;

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
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
                serializer_id: INT_SERIALIZER_ID,
                value: EntityDataValueKind::Int(ticks),
            }],
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
    // Vanilla LivingEntity.LIVING_ENTITY_FLAG_SPIN_ATTACK (4); IS_USING is bit 1.
    const LIVING_ENTITY_FLAG_SPIN_ATTACK: i8 = 4;
    const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

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
        LIVING_ENTITY_FLAG_SPIN_ATTACK | LIVING_ENTITY_FLAG_IS_USING,
    ));
    assert!(auto_spin(&store, 72));

    // Clearing the spin bit (other flags still set) stops the spin.
    assert!(set_flags(&mut store, 72, LIVING_ENTITY_FLAG_IS_USING));
    assert!(!auto_spin(&store, 72));

    // A non-living entity (boat) never spins even with a stray spin-attack bit at
    // the living-entity-flags id: only LivingEntityRenderer reads it.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_flags(&mut store, 73, LIVING_ENTITY_FLAG_SPIN_ATTACK));
    assert!(!auto_spin(&store, 73));
}

#[test]
fn entity_model_sources_project_using_item_flags() {
    // Vanilla LivingEntity flags: IS_USING = bit 1, OFF_HAND = bit 2, SPIN_ATTACK = bit 4.
    const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;
    const LIVING_ENTITY_FLAG_OFF_HAND: i8 = 2;
    const LIVING_ENTITY_FLAG_SPIN_ATTACK: i8 = 4;

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
        LIVING_ENTITY_FLAG_IS_USING | LIVING_ENTITY_FLAG_SPIN_ATTACK,
    ));
    assert_eq!(using(&store, 80), (true, false));

    // Using the off hand sets both bits.
    assert!(set_flags(
        &mut store,
        80,
        LIVING_ENTITY_FLAG_IS_USING | LIVING_ENTITY_FLAG_OFF_HAND,
    ));
    assert_eq!(using(&store, 80), (true, true));

    // Clearing the using bit stops it (a stray off-hand bit alone is not "using").
    assert!(set_flags(&mut store, 80, LIVING_ENTITY_FLAG_OFF_HAND));
    assert_eq!(using(&store, 80), (false, true));

    // A non-living entity (boat) never reads the flags byte.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_flags(
        &mut store,
        81,
        LIVING_ENTITY_FLAG_IS_USING | LIVING_ENTITY_FLAG_OFF_HAND,
    ));
    assert_eq!(using(&store, 81), (false, false));
}

#[test]
fn entity_model_sources_project_main_arm_left_for_item_in_hand_layer() {
    const VANILLA_AVATAR_MAIN_HAND_DATA_ID: u8 = 15;
    const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
    const VANILLA_HUMANOID_ARM_LEFT_ID: i32 = 0;
    const VANILLA_HUMANOID_ARM_RIGHT_ID: i32 = 1;
    const MOB_FLAG_LEFTHANDED: i8 = 2;

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
    assert!(!main_arm_left(&store, 83));
    // Vanilla `Mob.getMainArm`: `MOB_FLAG_LEFTHANDED` flips the default RIGHT arm.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 83,
        values: vec![protocol_byte_data(
            VANILLA_MOB_FLAGS_DATA_ID,
            MOB_FLAG_LEFTHANDED,
        )],
    }));
    assert!(main_arm_left(&store, 83));

    // Non-humanoid/non-living entities do not treat data id 15 as a main-arm source.
    store.apply_add_entity(protocol_add_entity_with_type(
        84,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 84,
        values: vec![protocol_byte_data(
            VANILLA_MOB_FLAGS_DATA_ID,
            MOB_FLAG_LEFTHANDED,
        )],
    }));
    assert!(!main_arm_left(&store, 84));
}

#[test]
fn entity_model_sources_project_aggressive_for_zombie_model_family() {
    // Vanilla Mob.DATA_MOB_FLAGS_ID (15); MOB_FLAG_AGGRESSIVE (4), LEFTHANDED is bit 2.
    const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
    const MOB_FLAG_AGGRESSIVE: i8 = 4;
    const MOB_FLAG_LEFTHANDED: i8 = 2;

    let aggressive = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_aggressive
    };
    let set_mob_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(flags),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    // A zombie with no mob flags is calm.
    assert!(!aggressive(&store, 80));

    // Vanilla Mob.isAggressive(): (DATA_MOB_FLAGS_ID & 4) != 0, detected alongside other flags.
    assert!(set_mob_flags(
        &mut store,
        80,
        MOB_FLAG_AGGRESSIVE | MOB_FLAG_LEFTHANDED,
    ));
    assert!(aggressive(&store, 80));
    // Clearing the aggressive bit (left-handed still set) returns to calm.
    assert!(set_mob_flags(&mut store, 80, MOB_FLAG_LEFTHANDED));
    assert!(!aggressive(&store, 80));

    // A chicken is a Mob too (it carries the mob-flags byte), but it does not render with the
    // zombie model's `animateZombieArms`, so the projection is gated out: a stray aggressive
    // bit never reaches the chicken's render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_mob_flags(&mut store, 81, MOB_FLAG_AGGRESSIVE));
    assert!(!aggressive(&store, 81));
}

#[test]
fn entity_model_sources_project_aggressive_for_piglin_and_illager_arm_poses() {
    // The aggressive flag also drives the piglin/brute `ATTACKING_WITH_MELEE_WEAPON`, the vindicator /
    // pillager `ATTACKING` arm pose, and the illusioner `BOW_AND_ARROW` aim — so `is_aggressive` is
    // projected for those types too. The evoker has no `isAggressive` branch in `getArmPose`, so it is
    // NOT projected.
    const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
    const MOB_FLAG_AGGRESSIVE: i8 = 4;

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
        (95, VANILLA_ENTITY_TYPE_PILLAGER_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, type_id));
        assert!(
            !aggressive(&store, id),
            "calm before the flag: type {type_id}"
        );
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_AGGRESSIVE),
            }],
        }));
        assert!(
            aggressive(&store, id),
            "aggressive projects: type {type_id}"
        );
    }

    // The evoker has no aggressive arm pose, so a stray aggressive bit never flips an unused pose.
    for (id, type_id) in [(94, VANILLA_ENTITY_TYPE_EVOKER_ID)] {
        store.apply_add_entity(protocol_add_entity_with_type(id, type_id));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_AGGRESSIVE),
            }],
        }));
        assert!(!aggressive(&store, id), "gated out: type {type_id}");
    }
}

#[test]
fn entity_model_sources_project_villager_unhappy() {
    // Vanilla AbstractVillager.DATA_UNHAPPY_COUNTER (INT id 18): read by
    // VillagerRenderer and WanderingTraderRenderer as `getUnhappyCounter() > 0`.
    const ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID: u8 = 18;

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

    assert!(!source(&store, 86).villager_unhappy);
    assert!(!source(&store, 87).villager_unhappy);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 86,
        values: vec![protocol_int_data(
            ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
            12
        )],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 87,
        values: vec![protocol_int_data(
            ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
            1
        )],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![protocol_int_data(
            ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
            12
        )],
    }));

    assert!(source(&store, 86).villager_unhappy);
    assert!(source(&store, 87).villager_unhappy);
    assert!(
        !source(&store, 88).villager_unhappy,
        "zombie villagers do not use AbstractVillagerRenderState"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 86,
        values: vec![protocol_int_data(
            ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
            0
        )],
    }));
    assert!(!source(&store, 86).villager_unhappy);
}

#[test]
fn entity_model_sources_project_enderman_carrying_and_creepy() {
    // Vanilla Enderman accessors: DATA_CARRY_STATE (16, OPTIONAL_BLOCK_STATE serializer 15),
    // DATA_CREEPY (17, BOOLEAN serializer 8).
    const CARRY_STATE_DATA_ID: u8 = 16;
    const CREEPY_DATA_ID: u8 = 17;
    const OPTIONAL_BLOCK_STATE_SERIALIZER_ID: i32 = 15;
    const BOOLEAN_SERIALIZER_ID: i32 = 8;
    const GRASS_BLOCK_STATE_ID: i32 = 9;

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
                        data_id: CARRY_STATE_DATA_ID,
                        serializer_id: OPTIONAL_BLOCK_STATE_SERIALIZER_ID,
                        value: EntityDataValueKind::OptionalBlockState(block),
                    },
                    ProtocolEntityDataValue {
                        data_id: CREEPY_DATA_ID,
                        serializer_id: BOOLEAN_SERIALIZER_ID,
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
    assert!(set_carry_and_creepy(
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
fn entity_model_sources_project_bat_resting_from_flags() {
    // Vanilla Bat.DATA_ID_FLAGS (16, BYTE); FLAG_RESTING (1).
    const VANILLA_BAT_FLAGS_DATA_ID: u8 = 16;
    const BAT_FLAG_RESTING: i8 = 1;

    let resting = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .bat_resting
    };
    let set_flags = |store: &mut WorldStore, id: i32, flags: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_BAT_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(flags),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_BAT_ID,
    ));
    // A bat with no flags is flying.
    assert!(!resting(&store, 70));
    // Setting Bat.FLAG_RESTING (DATA_ID_FLAGS & 1) projects the hanging pose; clearing it
    // returns to flying.
    assert!(set_flags(&mut store, 70, BAT_FLAG_RESTING));
    assert!(resting(&store, 70));
    assert!(set_flags(&mut store, 70, 0));
    assert!(!resting(&store, 70));

    // A chicken carries no bat flags byte; a stray bit at the same data id never reaches its
    // render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_flags(&mut store, 71, BAT_FLAG_RESTING));
    assert!(!resting(&store, 71));
}

#[test]
fn entity_model_sources_project_wither_invulnerable_ticks() {
    // Vanilla WitherBoss.DATA_ID_INV (19, INT): the spawn-invulnerability countdown.
    const VANILLA_WITHER_INV_DATA_ID: u8 = 19;

    let ticks = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .wither_invulnerable_ticks
    };
    let set_inv = |store: &mut WorldStore, id: i32, value: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_WITHER_INV_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Int(value),
            }],
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
    assert!(set_inv(&mut store, 80, 220));
    assert!((ticks(&store, 80, 0.0) - 220.0).abs() < 1.0e-6);
    assert!((ticks(&store, 80, 0.5) - 219.5).abs() < 1.0e-6);
    // Clearing it returns to 0.0 (a non-positive countdown is not lerped).
    assert!(set_inv(&mut store, 80, 0));
    assert_eq!(ticks(&store, 80, 0.5), 0.0);

    // A chicken carries no DATA_ID_INV accessor; a stray int at the same data id never reaches its
    // render state.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_inv(&mut store, 81, 220));
    assert_eq!(ticks(&store, 81, 0.0), 0.0);
}

#[test]
fn entity_model_sources_project_wither_side_head_tracking() {
    // Vanilla WitherBoss.DATA_TARGET_B/C (17/18): side-head target entity ids.
    const VANILLA_WITHER_TARGET_B_DATA_ID: u8 = 17;
    const VANILLA_WITHER_TARGET_C_DATA_ID: u8 = 18;

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
        values: vec![
            protocol_int_data(VANILLA_WITHER_TARGET_B_DATA_ID, 81),
            protocol_int_data(VANILLA_WITHER_TARGET_C_DATA_ID, 82),
        ],
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
        values: vec![
            protocol_int_data(VANILLA_WITHER_TARGET_B_DATA_ID, 0),
            protocol_int_data(VANILLA_WITHER_TARGET_C_DATA_ID, 0),
        ],
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
    // Vanilla Bee.DATA_FLAGS_ID (18, BYTE); FLAG_HAS_STUNG (4).
    const VANILLA_BEE_FLAGS_DATA_ID: u8 = 18;
    const BEE_FLAG_HAS_STUNG: i8 = 4;

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
                data_id: VANILLA_BEE_FLAGS_DATA_ID,
                serializer_id: 0,
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
    assert!(set_flags(&mut store, 72, BEE_FLAG_HAS_STUNG));
    assert!(!has_stinger(&store, 72));
    assert!(set_flags(&mut store, 72, 0));
    assert!(has_stinger(&store, 72));

    // A non-bee keeps the `true` stinger default regardless of a stray bit at the same data id.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_flags(&mut store, 73, BEE_FLAG_HAS_STUNG));
    assert!(has_stinger(&store, 73));
}

#[test]
fn entity_model_sources_gate_crouch_pose_on_the_player() {
    const POSE_STANDING: i32 = 0;
    const POSE_CROUCHING: i32 = 5;

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
            values: vec![protocol_pose_data(
                super::dimensions::ENTITY_DATA_POSE_ID,
                pose,
            )],
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
    assert!(set_pose(&mut store, 74, POSE_CROUCHING));
    assert!(crouching(&store, 74));
    assert!(set_pose(&mut store, 74, POSE_STANDING));
    assert!(!crouching(&store, 74));

    // A non-player entity is never crouched, even with a CROUCHING pose: only the player model
    // has the `HumanoidModel.setupAnim` crouch.
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(set_pose(&mut store, 75, POSE_CROUCHING));
    assert!(!crouching(&store, 75));
}

#[test]
fn entity_model_sources_project_feline_crouch_sprint_and_cat_sitting() {
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_SHARED_FLAG_SPRINTING: i8 = 1 << 3;
    const TAMABLE_ANIMAL_FLAGS_DATA_ID: u8 = 18;
    const TAMABLE_ANIMAL_SITTING_FLAG: i8 = 0x01;
    const POSE_STANDING: i32 = 0;
    const POSE_CROUCHING: i32 = 5;

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
            protocol_pose_data(super::dimensions::ENTITY_DATA_POSE_ID, POSE_CROUCHING),
            protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_SPRINTING),
            protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_SITTING_FLAG),
        ],
    ));
    assert_eq!(state(&store, 76), (true, true, true));

    assert!(set_data(
        &mut store,
        77,
        vec![
            protocol_pose_data(super::dimensions::ENTITY_DATA_POSE_ID, POSE_CROUCHING),
            protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_SITTING_FLAG),
        ],
    ));
    assert_eq!(state(&store, 77), (true, false, false));

    assert!(set_data(
        &mut store,
        78,
        vec![
            protocol_pose_data(super::dimensions::ENTITY_DATA_POSE_ID, POSE_CROUCHING),
            protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_SPRINTING),
            protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_SITTING_FLAG),
        ],
    ));
    assert_eq!(state(&store, 78), (false, false, false));

    assert!(set_data(
        &mut store,
        76,
        vec![
            protocol_pose_data(super::dimensions::ENTITY_DATA_POSE_ID, POSE_STANDING),
            protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, 0),
            protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, 0),
        ],
    ));
    assert_eq!(state(&store, 76), (false, false, false));
}
