use bbb_protocol::entity_synched_data::VANILLA_WITCH_USING_ITEM_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_WITCH_ID};
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};

use super::*;
use crate::entity::damage::StoredDamageCausingEntity;
use crate::entity::item::plain_stack;
use crate::entity::save::write_entity_chunk;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::thrown_potion::apply_registry_potion;
use crate::entity::tracking::non_default_synched_values;
use crate::entity::EntityIdCounter;
use crate::level_event::VanillaGameEvent;

fn witch(store: &mut EntityStore) -> i32 {
    store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_WITCH_ID,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 8.5,
            },
        )
        .expect("Witch is a generated GeneralLiving type")
}

#[test]
fn witch_runtime_metadata_is_transient_and_idle_step_stays_idle() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = witch(&mut store);
    let entity = store.get(id).expect("Witch is live");
    assert!(non_default_synched_values(entity)
        .iter()
        .all(|value| value.data_id != VANILLA_WITCH_USING_ITEM_DATA_ID));
    let saved = write_entity_chunk(&store, (0, 0)).expect("Witch save");
    let root = format!("{saved:?}");
    assert!(!root.contains("UsingItem"));
    assert!(!root.contains("usingTime"));

    let mut entity = store.detach(id).expect("Witch detaches");
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        apply_self_potion_step(&mut entity, &mut store, level, SelfPotionStep::default())
    });
    assert!(!matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.witch_state().is_some_and(WitchState::using_item)));
    store.attach(entity);
}

#[test]
fn witch_runtime_drinking_tracks_applies_and_clears() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = witch(&mut store);
    let mut entity = store.detach(id).expect("Witch detaches");
    entity.living_mut().expect("Witch living").set_health(8.0);
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        apply_self_potion_step(
            &mut entity,
            &mut store,
            level,
            SelfPotionStep {
                using_time: POTION_USE_DURATION,
                start: Some(SelfPotion::Healing),
                ..SelfPotionStep::default()
            },
        )
    });
    assert!(entity
        .take_synched_dirty()
        .contains(&VANILLA_WITCH_USING_ITEM_DATA_ID));
    let state = match entity.data() {
        EntityData::GeneralLiving(data) => data.witch_state().unwrap(),
        _ => unreachable!(),
    };
    assert!(matches!(
        synched_values(state)[0].value.value,
        EntityDataValueKind::Boolean(true)
    ));
    native.with_level(|level| {
        apply_self_potion_step(
            &mut entity,
            &mut store,
            level,
            SelfPotionStep {
                finish_drinking: true,
                ..SelfPotionStep::default()
            },
        )
    });
    assert_eq!(entity.living().unwrap().health(), 12.0);
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::DRINK
            && event.recorded.source_entity_id == Some(id)));
    assert!(!matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.witch_state().is_some_and(WitchState::using_item)));
    assert!(!entity
        .living()
        .unwrap()
        .has_item_in_slot(EquipmentSlot::MainHand));
}

#[test]
fn finishing_witch_drink_consumes_a_potion_without_effect_contents() {
    let mut store = EntityStore::new(25, EntityIdCounter::new());
    let id = witch(&mut store);
    let mut entity = store.detach(id).unwrap();
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        apply_self_potion_step(
            &mut entity,
            &mut store,
            level,
            SelfPotionStep {
                using_time: POTION_USE_DURATION,
                start: Some(SelfPotion::Healing),
                ..SelfPotionStep::default()
            },
        )
    });
    entity
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, plain_stack("minecraft:potion", 1));
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.witch_state_mut().unwrap().using_time = 0;
    }
    assert!(native.with_level(|level| before_ai_step(&mut entity, &mut store, level)));

    assert!(!entity
        .living()
        .unwrap()
        .has_item_in_slot(EquipmentSlot::MainHand));
    assert!(matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.witch_state().is_some_and(|state| !state.using_item())));
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::DRINK
            && event.recorded.source_entity_id == Some(id)));
}

#[test]
fn witch_runtime_registry_splash_effect_is_live() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let target = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 8.5,
            },
        )
        .unwrap();
    let item = registry_potion_stack("minecraft:potion", "minecraft:slowness").unwrap();
    let mut entity = store.detach(target).unwrap();
    apply_registry_potion(&mut entity, &item).expect("built-in potion projects");
    assert!(entity
        .living()
        .unwrap()
        .active_effect("minecraft:slowness")
        .is_some());
}

#[test]
fn witch_runtime_damage_resistance_and_self_immunity() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = witch(&mut store);
    let entity = store.get(id).unwrap();
    let magic = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:magic")
        .expect("magic damage type");
    let source = StoredEntityDamageSource {
        damage_type: magic,
        direct_entity_id: None,
        causing_entity: None,
        attacker_views: Default::default(),
    };
    assert_eq!(damage_multiplier(entity, source), 0.15);
    assert_eq!(
        damage_multiplier(
            entity,
            StoredEntityDamageSource {
                causing_entity: Some(StoredDamageCausingEntity::Mob { entity_id: id }),
                ..source
            }
        ),
        0.0
    );
}

#[test]
fn witch_forgets_fire_damage_forty_ticks_after_the_hit() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let id = witch(&mut store);
    let entity = store.get_mut(id).expect("Witch is live");
    remember_damage_with_is_fire(
        entity,
        StoredEntityHurtOutcome {
            applied: true,
            succeeded: true,
        },
        true,
        100,
    );
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    let state = data.witch_state().expect("Witch state");
    assert!(state.remembers_fire_damage(140));
    assert!(!state.remembers_fire_damage(141));
}

#[test]
fn witch_finishes_drinking_gains_the_effect_and_recovers_speed() {
    let mut store = EntityStore::new(26, EntityIdCounter::new());
    let id = witch(&mut store);
    let mut entity = store.detach(id).unwrap();
    entity.living_mut().unwrap().set_health(8.0);
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        apply_self_potion_step(
            &mut entity,
            &mut store,
            level,
            SelfPotionStep {
                using_time: 2,
                start: Some(SelfPotion::Healing),
                ..SelfPotionStep::default()
            },
        )
    });
    let speed = |entity: &Entity| {
        entity
            .living()
            .unwrap()
            .attributes()
            .value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap()
    };
    assert_eq!(speed(&entity), 0.0);
    for _ in 0..3 {
        assert!(native.with_level(|level| before_ai_step(&mut entity, &mut store, level)));
    }
    assert!(entity.living().unwrap().health() >= 12.0);
    assert_eq!(speed(&entity), 0.25);
    assert!(matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.witch_state().is_some_and(|state| !state.using_item())));
}

#[test]
fn witch_throws_healing_at_a_hurt_raider_target_and_drops_it() {
    use crate::connection::{ClientboundPlayPacket, ConnectionId};
    use crate::world::WorldEvent;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
    };

    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        "difficulty normal",
    );
    native.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 20.5,
        },
    );
    let spawn = |native: &mut NativeLevel, type_id, x: f64, no_ai| {
        let mut data = crate::entity::GeneralLivingData::new(type_id).unwrap();
        data.set_no_ai(no_ai);
        native.world.spawn_stored_entity_for_test(
            DIMENSION,
            type_id,
            Vec3d { x, y: 64.0, z: 8.5 },
            EntityData::GeneralLiving(data),
        )
    };
    let (id, _) = spawn(&mut native, VANILLA_ENTITY_TYPE_WITCH_ID, 8.5, false);
    let (pillager, pillager_uuid) = spawn(&mut native, VANILLA_ENTITY_TYPE_PILLAGER_ID, 12.5, true);
    native.world.tick(Vec::new());
    native
        .world
        .set_mob_target_for_test(DIMENSION, id, Some((pillager, pillager_uuid)));
    let thrown = (0..200).any(|_| {
        native.world.tick(Vec::new()).events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(add), .. }
                | WorldEvent::Broadcast { packet: ClientboundPlayPacket::AddEntity(add), .. }
                    if add.entity_type_id == VANILLA_ENTITY_TYPE_SPLASH_POTION_ID)
        })
    });
    assert!(thrown);
    assert_eq!(native.world.mob_target_for_test(DIMENSION, id), None);
}
