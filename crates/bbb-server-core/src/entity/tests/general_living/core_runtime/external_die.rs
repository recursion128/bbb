use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

#[test]
fn external_die_preserves_health_and_publishes_death_loot_event_and_dying_pose() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 0xD1E);
    let mut store = {
        let ids = EntityIdCounter::new();
        ids.next_id();
        EntityStore::new(0xD1E, ids)
    };
    let victim = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .expect("victim");
    let mut victim_entity = store.detach(victim).expect("victim");
    victim_entity.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID);
    let living = victim_entity.living_mut().expect("living victim");
    living.last_hurt_by_player = Some(Uuid::from_u128(0xD1E));
    living.last_hurt_by_player_memory_time = 100;
    let health_before = living.health();
    living.set_item_slot(
        EquipmentSlot::MainHand,
        crate::entity::item::plain_stack("minecraft:diamond", 1),
    );
    let EntityData::GeneralLiving(data) = &mut victim_entity.data else {
        unreachable!()
    };
    data.mob
        .as_mut()
        .unwrap()
        .set_drop_chance(EquipmentSlot::MainHand, 1.0);
    let damage_type = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .expect("builtin generic damage");
    let source = StoredEntityDamageSource {
        damage_type,
        direct_entity_id: None,
        causing_entity: None,
        attacker_views: Default::default(),
    };

    assert!(
        native.with_level(|level| crate::entity::general_living::external_die(
            &mut victim_entity,
            &mut store,
            level,
            source,
        ))
    );
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(victim)
                && event.recorded.pos == victim_entity.block_position()
        ));
    assert!(native
        .world
        .stored_items_for_test(DIMENSION)
        .iter()
        .any(|item| item.2 == "minecraft:diamond" && item.3 == 1));
    assert!(victim_entity.living().expect("living victim").dead);
    assert_eq!(
        victim_entity.living().expect("living victim").health(),
        health_before
    );
    assert_eq!(
        victim_entity.pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DYING_ID
    );
}

#[test]
fn external_die_attributes_tame_wolf_owner_and_clears_stale_credit_without_owner() {
    let damage_type = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .expect("builtin generic damage");
    let previous = Uuid::from_u128(0xabc);
    for owner in [Some(Uuid::from_u128(0xdef)), None] {
        let mut store = {
            let ids = EntityIdCounter::new();
            ids.next_id();
            EntityStore::new(0x521, ids)
        };
        let wolf = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_WOLF_ID, position())
            .expect("wolf");
        assert!(crate::entity::taming::set_tame_offspring(
            store.get_mut(wolf).expect("wolf"),
            owner,
        ));
        let victim = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
            .expect("victim");
        let mut victim_entity = store.detach(victim).expect("victim");
        let living = victim_entity.living_mut().expect("living victim");
        living.last_hurt_by_player = Some(previous);
        living.last_hurt_by_player_memory_time = 100;
        let health_before = living.health();
        let mut native = NativeLevel::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 0xdef);
        let source = StoredEntityDamageSource {
            damage_type,
            direct_entity_id: Some(wolf),
            causing_entity: Some(StoredDamageCausingEntity::Mob { entity_id: wolf }),
            attacker_views: Default::default(),
        };
        assert!(
            native.with_level(|level| crate::entity::general_living::external_die(
                &mut victim_entity,
                &mut store,
                level,
                source,
            ))
        );
        let living = victim_entity.living().expect("living victim");
        assert_eq!(
            living.health(),
            health_before,
            "direct die leaves health for its caller"
        );
        assert_eq!(living.last_hurt_by_player, owner);
        assert_eq!(
            living.last_hurt_by_player_memory_time,
            if owner.is_some() { 100 } else { 0 }
        );
        assert_eq!(living.last_damage_type, None);
        assert_eq!(living.last_damage_memory_time, 0);
        let kills = store.drain_player_kills();
        assert_eq!(kills.len(), usize::from(owner.is_some()));
        if let Some(kill) = kills.first() {
            assert_eq!(kill.kill_credit_player_uuid, owner);
        }
        assert!(
            !native.with_level(|level| crate::entity::general_living::external_die(
                &mut victim_entity,
                &mut store,
                level,
                source,
            ))
        );
        assert!(store.drain_player_kills().is_empty());
    }
}
