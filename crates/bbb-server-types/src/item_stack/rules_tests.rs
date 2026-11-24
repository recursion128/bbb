use super::*;
use bbb_protocol::packets::{EquippableSummary, SoundEventSummary};

/// LivingEntity.java:3821-3825 reads the slot off a patched `equippable`.
#[test]
fn a_patched_equippable_names_the_slot_it_fits() {
    let sound = SoundEventSummary {
        registry_id: None,
        sound_id: None,
        fixed_range_bits: None,
    };
    let mut stick = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STICK_ID),
        count: 1,
        component_patch: Default::default(),
    };
    assert!(!is_equippable_in_slot(&stick, EquipmentSlot::Head).unwrap());
    stick.component_patch.equippable = Some(EquippableSummary {
        slot_id: bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOT_HEAD_ID,
        equip_sound: sound.clone(),
        asset_id: None,
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: sound,
    });
    stick
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    assert!(is_equippable_in_slot(&stick, EquipmentSlot::Head).unwrap());
    assert!(!is_equippable_in_slot(&stick, EquipmentSlot::Chest).unwrap());
    assert_eq!(
        equipment_slot_for_item(&stick).unwrap(),
        EquipmentSlot::Head
    );
}

#[test]
fn durability_uses_has_component_with_removed_precedence() {
    let pickaxe =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:diamond_pickaxe")
            .unwrap()
            .id;
    let mut stack = ItemStackSummary {
        item_id: Some(pickaxe),
        count: 1,
        component_patch: Default::default(),
    };
    assert!(damageable_max_damage(&stack).is_some());

    // Adding UNBREAKABLE makes the stack non-damageable.
    add_patched_component(&mut stack, VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID);
    stack.component_patch.unbreakable = true;
    assert_eq!(damageable_max_damage(&stack), None);

    // Removing DAMAGE (even if also present in added_type_ids) obeys removed-first precedence.
    stack.component_patch.added_type_ids.clear();
    stack.component_patch.unbreakable = false;
    stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    stack
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    assert_eq!(damageable_max_damage(&stack), None);
}

#[test]
fn current_damage_clamps_negative_max_damage_and_mod_max_stack_size_applies() {
    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModItemRegistration};

    let mut builder = ContentRegistryBuilder::new();
    builder.register_item(ModItemRegistration {
        resource_id: "modtest:amulet".to_string(),
        max_stack_size: 16,
        is_block_item: false,
    });
    let registry = builder.freeze_for_session().unwrap();
    let mod_item = registry.item_by_resource_id("modtest:amulet").unwrap();

    let mut stack = ItemStackSummary {
        item_id: Some(mod_item.id),
        count: 1,
        component_patch: Default::default(),
    };
    assert_eq!(max_stack_size_with_registry(&stack, &registry).unwrap(), 16);

    stack.component_patch.damage = Some(5);
    add_patched_component(&mut stack, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    assert_eq!(current_damage(&stack, -10), 0);
}
