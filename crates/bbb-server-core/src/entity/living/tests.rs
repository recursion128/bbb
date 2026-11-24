use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID;

use super::*;

#[test]
fn constructor_sets_health_to_max_health_from_the_supplier() {
    let living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    assert_eq!(living.health(), 20.0);
    assert_eq!(living.max_health(), 20.0);
    assert!(!living.is_dead_or_dying());
    assert!(living.attributes().pack().is_empty());
    assert!(LivingEntityState::new("minecraft:item").is_none());
}

#[test]
fn set_health_clamps_between_zero_and_max() {
    let mut living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    living.set_health(-5.0);
    assert_eq!(living.health(), 0.0);
    assert!(living.is_dead_or_dying());
    living.set_health(99.0);
    assert_eq!(living.health(), 20.0);
    living.set_health(3.5);
    living.heal(1.0);
    assert_eq!(living.health(), 4.5);
    living.set_health(0.0);
    living.heal(5.0);
    assert_eq!(living.health(), 0.0);
}

#[test]
fn refresh_dirty_attributes_clamps_health_to_a_lowered_max() {
    let mut store = EntityStore::new(0, crate::entity::EntityIdCounter::new());
    let id = crate::entity::armor_stand::spawn(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        },
    )
    .unwrap();
    let entity = store.get_mut(id).unwrap();
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        .unwrap()
        .set_base_value(4.0);
    assert_eq!(entity.living().unwrap().health(), 20.0);
    refresh_dirty_attributes(entity);
    assert_eq!(entity.living().unwrap().health(), 4.0);
    assert!(entity
        .take_synched_dirty()
        .contains(&VANILLA_LIVING_ENTITY_HEALTH_DATA_ID));
    assert!(entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .drain_attributes_to_update()
        .next()
        .is_none());
}

#[test]
fn ordinary_living_death_keeps_its_twenty_tick_removal() {
    let mut store = EntityStore::new(0, crate::entity::EntityIdCounter::new());
    let type_id = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
    let id = store.spawn(
        type_id,
        Vec3d::default(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(type_id).expect("zombie living data"),
        ),
    );
    let mut entity = store.detach(id).expect("zombie");
    set_health(&mut entity, 0.0);
    for tick in 1..=20 {
        tick_death(&mut entity, &mut store);
        assert_eq!(entity.is_alive(), tick < 20);
    }
    assert_eq!(
        entity.removal_reason(),
        Some(crate::entity::RemovalReason::Killed)
    );
}

#[test]
fn hurt_cooldown_swallows_weaker_hits_inside_the_window() {
    let mut living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    assert_eq!(hurt_cooldown(&mut living, false, 3.0), Some((3.0, true)));
    assert_eq!(living.invulnerable_time, 20);
    assert_eq!(living.hurt_time, 10);
    assert_eq!(living.last_hurt, 3.0);
    assert_eq!(hurt_cooldown(&mut living, false, 2.0), None);
    assert_eq!(hurt_cooldown(&mut living, false, 5.0), Some((2.0, false)));
    assert_eq!(living.last_hurt, 5.0);
    living.invulnerable_time = 10;
    assert_eq!(hurt_cooldown(&mut living, false, 1.0), Some((1.0, true)));
    living.invulnerable_time = 20;
    assert_eq!(hurt_cooldown(&mut living, true, 1.0), Some((1.0, true)));
}

#[test]
fn actually_hurt_spends_absorption_before_health() {
    let mut living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    living
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID)
        .unwrap()
        .set_base_value(4.0);
    living.set_absorption_amount(3.0);
    assert_eq!(actually_hurt(&mut living, true, 5.0), 2.0);
    assert_eq!(living.absorption_amount(), 0.0);
    assert_eq!(living.health(), 18.0);
    // Armour: 4 points against 5 damage, toughness 0.
    living
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_ARMOR_ID)
        .unwrap()
        .set_base_value(4.0);
    let expected = damage_after_absorb(5.0, 4.0, 0.0);
    assert_eq!(actually_hurt(&mut living, false, 5.0), expected);
    assert_eq!(living.health(), 18.0 - expected);
}

#[test]
fn combat_rules_absorb_matches_the_formula() {
    assert_eq!(
        damage_after_absorb(10.0, 20.0, 0.0),
        10.0_f32 * (1.0 - 15.0_f32 / 25.0)
    );
    assert_eq!(damage_after_absorb(10.0, 0.0, 0.0), 10.0);
}

#[test]
fn equipment_keeps_only_nonempty_stacks_in_slot_order() {
    let mut equipment = EntityEquipment::new();
    assert!(equipment.is_empty());
    let helmet = crate::entity::item::plain_stack("minecraft:iron_helmet", 1).unwrap();
    let boots = crate::entity::item::plain_stack("minecraft:iron_boots", 1).unwrap();
    assert_eq!(
        equipment.set(EquipmentSlot::Head, Some(helmet.clone())),
        None
    );
    assert_eq!(
        equipment.set(EquipmentSlot::Feet, Some(boots.clone())),
        None
    );
    assert!(!equipment.is_empty());
    let slots: Vec<EquipmentSlot> = equipment.iter().map(|(slot, _)| slot).collect();
    assert_eq!(slots, vec![EquipmentSlot::Feet, EquipmentSlot::Head]);
    assert_eq!(
        equipment.set(EquipmentSlot::Head, crate::entity::item::empty_stack()),
        Some(helmet)
    );
    assert!(equipment.get(EquipmentSlot::Head).is_none());
    assert_eq!(equipment.get(EquipmentSlot::Feet), Some(&boots));
}

#[test]
fn detect_equipment_updates_reports_changed_slots_once() {
    let mut living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    assert!(detect_equipment_updates(&living).is_empty());
    let helmet = crate::entity::item::plain_stack("minecraft:iron_helmet", 1).unwrap();
    living.set_item_slot(EquipmentSlot::Head, Some(helmet.clone()));
    let changed = vec![(EquipmentSlot::Head, Some(helmet))];
    assert_eq!(detect_equipment_updates(&living), changed);
    assert_eq!(detect_equipment_updates(&living), changed);
    acknowledge_equipment_updates(&mut living, &changed);
    assert!(detect_equipment_updates(&living).is_empty());
    living.set_item_slot(EquipmentSlot::Head, None);
    let changed = vec![(EquipmentSlot::Head, None)];
    assert_eq!(detect_equipment_updates(&living), changed);
    acknowledge_equipment_updates(&mut living, &changed);
    assert!(detect_equipment_updates(&living).is_empty());
}

#[test]
fn wrap_degrees_matches_mth() {
    assert_eq!(wrap_degrees(180.0), -180.0);
    assert_eq!(wrap_degrees(-180.0), -180.0);
    assert_eq!(wrap_degrees(540.0), -180.0);
    assert_eq!(wrap_degrees(-190.0), 170.0);
    assert_eq!(wrap_degrees(90.0), 90.0);
}

#[test]
fn fall_damage_uses_safe_fall_distance_and_multiplier() {
    let living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    assert_eq!(calculate_fall_damage(&living, 2.5, 1.0, false), -1);
    assert_eq!(calculate_fall_damage(&living, 3.0, 1.0, false), 0);
    assert_eq!(calculate_fall_damage(&living, 4.0, 1.0, false), 1);
    assert_eq!(calculate_fall_damage(&living, 10.0, 1.0, false), 7);
    assert_eq!(calculate_fall_damage(&living, 10.0, 1.0, true), 0);
    assert_eq!(
        fall_damage_sound(4, GENERIC_SMALL_FALL_SOUND, GENERIC_BIG_FALL_SOUND),
        GENERIC_SMALL_FALL_SOUND
    );
    assert_eq!(
        fall_damage_sound(5, GENERIC_SMALL_FALL_SOUND, GENERIC_BIG_FALL_SOUND),
        GENERIC_BIG_FALL_SOUND
    );
}

#[test]
fn living_speed_factor_lerps_toward_one_by_movement_efficiency() {
    let mut living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    assert_eq!(living_block_speed_factor(&living, 0.4), 0.4);
    living
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID)
        .unwrap()
        .set_base_value(0.5);
    assert_eq!(
        living_block_speed_factor(&living, 0.4),
        0.4_f32 + 0.5 * (1.0 - 0.4_f32)
    );
}
