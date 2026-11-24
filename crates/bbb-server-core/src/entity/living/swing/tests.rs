use super::*;
use crate::entity::living::mob_effects::ActiveMobEffect;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

fn living() -> LivingEntityState {
    LivingEntityState::new("minecraft:zombie").unwrap()
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn item(id: &str, patch: Option<Option<i32>>) -> RuntimeItemStack {
    let mut fields = vec![
        entry("id", NbtSummaryValue::string(id)),
        entry("count", NbtSummaryValue::Int(1)),
    ];
    if let Some(patch) = patch {
        let component = match patch {
            Some(duration) => entry(
                "minecraft:swing_animation",
                NbtSummaryValue::Compound(vec![entry("duration", NbtSummaryValue::Int(duration))]),
            ),
            None => entry(
                "!minecraft:swing_animation",
                NbtSummaryValue::Compound(vec![]),
            ),
        };
        fields.push(entry(
            "components",
            NbtSummaryValue::Compound(vec![component]),
        ));
    }
    RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(fields),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

fn effect(id: &'static str, amplifier: i32) -> ActiveMobEffect {
    ActiveMobEffect::new(id, 200, amplifier).unwrap()
}

#[test]
fn swing_uses_each_arm_patch_prototype_and_removed_default() {
    let mut state = living();
    assert_eq!(state.current_swing_duration(), 6);
    state.set_item_slot(
        EquipmentSlot::MainHand,
        Some(item("minecraft:wooden_spear", None)),
    );
    assert_eq!(state.current_swing_duration(), 13);
    state.set_item_slot(
        EquipmentSlot::OffHand,
        Some(item("minecraft:stick", Some(Some(10)))),
    );
    assert!(state.swing(InteractionHand::OffHand));
    assert_eq!(state.current_swing_duration(), 10);
    state.set_item_slot(
        EquipmentSlot::OffHand,
        Some(item("minecraft:wooden_spear", Some(None))),
    );
    assert_eq!(state.current_swing_duration(), 6);
    state.set_item_slot(
        EquipmentSlot::OffHand,
        Some(item("minecraft:wooden_spear", Some(Some(9)))),
    );
    assert_eq!(state.current_swing_duration(), 9);
}

#[test]
fn swing_restart_uses_previous_arm_duration_and_negative_time_admission() {
    let mut state = living();
    state.set_item_slot(
        EquipmentSlot::OffHand,
        Some(item("minecraft:stick", Some(Some(20)))),
    );
    assert!(state.swing(InteractionHand::MainHand));
    assert!(state.swing(InteractionHand::MainHand));
    for _ in 0..3 {
        state.update_swing_time();
    }
    assert_eq!(state.swing.time, 2);
    assert!(!state.swing(InteractionHand::OffHand));
    assert_eq!(state.swing.arm, Some(InteractionHand::MainHand));
    state.update_swing_time();
    assert!(state.swing(InteractionHand::OffHand));
    assert_eq!(state.swing.time, -1);
    state.update_swing_time();
    state.update_swing_time();
    assert_eq!(state.attack_anim(), 1.0 / 20.0);
    assert!(!state.swing(InteractionHand::MainHand));
}

#[test]
fn swing_recomputes_duration_for_changed_items_and_effects() {
    let mut state = living();
    state.swing(InteractionHand::MainHand);
    for _ in 0..4 {
        state.update_swing_time();
    }
    assert_eq!(state.attack_anim(), 0.5);
    state.set_item_slot(
        EquipmentSlot::MainHand,
        Some(item("minecraft:stick", Some(Some(10)))),
    );
    state.update_swing_time();
    assert_eq!(state.attack_anim(), 0.4);
    state.replace_active_effects(vec![effect("minecraft:mining_fatigue", 2)]);
    assert_eq!(state.current_swing_duration(), 16);
    state.replace_active_effects(vec![
        effect("minecraft:mining_fatigue", 2),
        effect("minecraft:haste", 1),
        effect("minecraft:conduit_power", 3),
    ]);
    assert_eq!(state.current_swing_duration(), 6);
    state.update_swing_time();
    assert_eq!(state.attack_anim(), 5.0 / 6.0);
    state.update_swing_time();
    assert!(!state.is_swinging());
    assert_eq!(state.attack_anim(), 0.0);
    assert_eq!(state.swing.arm, Some(InteractionHand::MainHand));
}

#[test]
fn swing_command_restore_preserves_state_and_used_hand_is_independent() {
    let mut previous = living();
    previous.living_entity_flags = 2;
    assert_eq!(previous.used_item_hand(), InteractionHand::OffHand);
    previous.living_entity_flags = 1;
    assert_eq!(previous.used_item_hand(), InteractionHand::MainHand);
    previous.swing(InteractionHand::OffHand);
    previous.update_swing_time();
    previous.update_swing_time();
    let mut restored = living();
    restored.restore_command_transients(&previous);
    assert!(restored.is_swinging());
    assert_eq!(restored.swing.time, previous.swing.time);
    assert_eq!(restored.swing.arm, previous.swing.arm);
    assert_eq!(restored.attack_anim(), previous.attack_anim());
    restored.update_swing_time();
    assert_eq!(previous.swing.time, 1);
    assert_eq!(restored.swing.time, 2);
}

#[test]
fn swing_disk_roundtrip_resets_transient_animation() {
    use crate::entity::{EntityData, EntityIdCounter, EntityStore};
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
    use bbb_protocol::packets::Vec3d;
    let mut store = EntityStore::new(19, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
        panic!("zombie")
    };
    data.living_mut().swing(InteractionHand::OffHand);
    data.living_mut().update_swing_time();
    data.living_mut().update_swing_time();
    let saved = crate::entity::save::write_entity_chunk(&store, (0, 0)).unwrap();
    let mut restored = EntityStore::new(20, EntityIdCounter::new());
    let ids = crate::entity::save::read_entity_chunk(&mut restored, &saved).unwrap();
    let EntityData::GeneralLiving(data) = restored.get(ids[0]).unwrap().data() else {
        panic!("zombie")
    };
    assert!(!data.living().is_swinging());
    assert_eq!(data.living().attack_anim(), 0.0);
    assert_eq!(data.living().swing.time, 0);
    assert_eq!(data.living().swing.arm, None);
}

#[test]
fn actual_monster_and_mannequin_ticks_allow_a_new_swing_after_the_animation() {
    use crate::entity::tests::native_level::NativeLevel;
    use crate::entity::{
        tick_entities, EntityData, EntityIdCounter, EntityStore, GeneralLivingData,
    };
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
    };
    use bbb_protocol::packets::Vec3d;
    for kind in [
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
    ] {
        let mut store = EntityStore::new(23, EntityIdCounter::new());
        let mut data = GeneralLivingData::new(kind).unwrap();
        data.set_no_ai(true);
        let id = store
            .restore_initialized(
                uuid::Uuid::from_u128(kind as u128),
                kind,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
                EntityData::GeneralLiving(data),
                |entity| entity.set_no_gravity(true),
            )
            .unwrap();
        assert!(store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .swing(InteractionHand::MainHand));
        let mut level = NativeLevel::new();
        level.with_level(|level| tick_entities(&mut store, level, true));
        assert!(
            !store
                .get_mut(id)
                .unwrap()
                .living_mut()
                .unwrap()
                .swing(InteractionHand::OffHand),
            "type {kind} cannot restart an unfinished swing"
        );
        for _ in 0..24 {
            level.with_level(|level| tick_entities(&mut store, level, true));
        }
        let living = store.get_mut(id).unwrap().living_mut().unwrap();
        assert!(!living.is_swinging(), "type {kind} finishes its swing");
        assert!(living.swing(InteractionHand::OffHand));
    }
}
