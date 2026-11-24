use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_CREAKING_CAN_MOVE_DATA_ID, VANILLA_CREAKING_HOME_POS_DATA_ID,
    VANILLA_CREAKING_IS_ACTIVE_DATA_ID, VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};

use super::native_level::NativeLevel;
use super::*;
use crate::entity::tracking::non_default_synched_values;
use crate::entity::CreakingHurtDecision;
use crate::entity::GeneralLivingData;

fn protector_at(pos: Vec3d) -> (EntityStore, i32) {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        pos,
        EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID).expect("creaking is living"),
        ),
    );
    (store, id)
}

#[test]
fn an_unbound_creaking_is_not_fire_immune_and_knows_no_home() {
    let (store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get(id).expect("the creaking is live");
    assert_eq!(entity.creaking_home_pos(), None);
    assert!(!entity.is_heart_bound());
    assert!(!entity.fire_immune());
}

#[test]
fn a_transient_binding_binds_the_protector_and_makes_it_fire_immune() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let heart = BlockPos { x: 4, y: 64, z: -2 };
    assert!(store
        .get_mut(id)
        .expect("the creaking is live")
        .set_creaking_transient(heart));
    let entity = store.get(id).expect("the creaking is live");
    assert_eq!(entity.creaking_home_pos(), Some(heart));
    assert!(entity.is_heart_bound());
    assert!(entity.fire_immune());
}

#[test]
fn the_stuck_counter_needs_five_consecutive_ticks_inside_the_box() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get_mut(id).expect("the creaking is live");
    for tick in 1..=4 {
        assert!(!entity.creaking_note_player_eye_inside(true), "tick {tick}");
    }
    assert!(entity.creaking_note_player_eye_inside(true));
    assert!(!entity.creaking_note_player_eye_inside(false));
    for tick in 1..=4 {
        assert!(!entity.creaking_note_player_eye_inside(true), "tick {tick}");
    }
    assert!(entity.creaking_note_player_eye_inside(true));
}

#[test]
fn a_heart_bound_protector_absorbs_a_player_hit_for_eight_ticks() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let heart = BlockPos { x: 4, y: 64, z: -2 };
    let entity = store.get_mut(id).expect("the creaking is live");
    assert_eq!(
        entity.creaking_hurt_decision(false, false, true, true),
        CreakingHurtDecision::PassThrough
    );
    assert!(entity.set_creaking_transient(heart));
    assert_eq!(
        entity.creaking_hurt_decision(false, false, true, true),
        CreakingHurtDecision::Absorb
    );
    assert_eq!(
        entity.creaking_hurt_decision(true, false, true, true),
        CreakingHurtDecision::PassThrough
    );
    assert_eq!(
        entity.creaking_hurt_decision(false, false, false, false),
        CreakingHurtDecision::Refuse
    );
    assert_eq!(
        entity.creaking_hurt_decision(false, true, false, false),
        CreakingHurtDecision::Refuse
    );
    assert_eq!(
        entity.creaking_hurt_decision(false, true, true, true),
        CreakingHurtDecision::Refuse
    );

    assert!(entity.begin_creaking_hurt_animation());
    for tick in (1..=7).rev() {
        assert_eq!(
            entity.creaking_hurt_decision(false, false, true, true),
            CreakingHurtDecision::Refuse,
            "the window is closed for {tick} more ticks"
        );
        entity.tick_creaking_animations();
    }
    entity.tick_creaking_animations();
    assert_eq!(
        entity.creaking_hurt_decision(false, false, true, true),
        CreakingHurtDecision::Absorb
    );
}

#[test]
fn a_heart_bound_protector_ignores_environment_damage() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let mut level = NativeLevel::new();
    let mut entity = store.detach(id).expect("the creaking is live");
    assert!(level.with_level(
        |level| crate::entity::general_living::hurt_from_environment(
            &mut entity,
            &mut store,
            level,
            "minecraft:drown",
            1.0,
        )
    ));
    assert_eq!(
        entity.living().map(|living| living.health()),
        Some(0.0),
        "an unbound creaking drowns"
    );

    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let mut entity = store.detach(id).expect("the creaking is live");
    assert!(entity.set_creaking_transient(BlockPos { x: 4, y: 64, z: -2 }));
    assert!(level.with_level(
        |level| crate::entity::general_living::hurt_from_environment(
            &mut entity,
            &mut store,
            level,
            "minecraft:drown",
            1.0,
        )
    ));
    assert_eq!(
        entity.living().map(|living| living.health()),
        Some(1.0),
        "the heart-bound protector takes no drowning damage"
    );
}

#[test]
fn the_binding_round_trips_the_entity_chunk() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let heart = BlockPos { x: -1, y: 70, z: 3 };
    assert!(store
        .get_mut(id)
        .expect("the creaking is live")
        .set_creaking_transient(heart));
    let saved = crate::entity::save::write_entity_chunk(&store, (0, 0)).expect("the chunk saves");
    let record = saved
        .entities()
        .next()
        .expect("the creaking is in the chunk");
    assert_eq!(
        record
            .raw()
            .iter()
            .find(|entry| entry.name == "home_pos")
            .map(|entry| entry.value.clone()),
        Some(bbb_protocol::NbtSummaryValue::IntArray(vec![
            heart.x, heart.y, heart.z
        ]))
    );

    let mut restored = EntityStore::new(0, EntityIdCounter::new());
    let ids =
        crate::entity::save::read_entity_chunk(&mut restored, &saved).expect("the chunk loads");
    let restored = restored.get(ids[0]).expect("the creaking is restored");
    assert_eq!(restored.creaking_home_pos(), Some(heart));
    assert!(restored.is_heart_bound());
}

#[test]
fn the_binding_is_the_only_non_default_synched_row() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get(id).expect("the creaking is live");
    assert!(non_default_synched_values(entity).is_empty());
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        panic!("the creaking is a living entity");
    };
    let rows = crate::entity::creaking::synched_values(
        data.creaking_retained().expect("creaking retained state"),
    );
    let ids: Vec<(u8, bool)> = rows
        .iter()
        .map(|row| (row.value.data_id, row.is_default))
        .collect();
    assert_eq!(
        ids,
        vec![
            (VANILLA_CREAKING_CAN_MOVE_DATA_ID, true),
            (VANILLA_CREAKING_IS_ACTIVE_DATA_ID, true),
            (VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID, true),
            (VANILLA_CREAKING_HOME_POS_DATA_ID, true),
        ]
    );
    assert_eq!(
        rows[3].value.value,
        EntityDataValueKind::OptionalBlockPos(None)
    );

    let heart = BlockPos { x: 4, y: 64, z: -2 };
    let entity = store.get_mut(id).expect("the creaking is live");
    assert!(entity.set_creaking_transient(heart));
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_CREAKING_HOME_POS_DATA_ID],
        "`setHomePos` writes the accessor and marks it dirty"
    );
    let published = non_default_synched_values(store.get(id).expect("the creaking is live"));
    assert_eq!(published.len(), 1, "{published:?}");
    assert_eq!(published[0].data_id, VANILLA_CREAKING_HOME_POS_DATA_ID);
    assert_eq!(
        published[0].serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID
    );
    assert_eq!(
        published[0].value,
        EntityDataValueKind::OptionalBlockPos(Some(heart))
    );
}

#[test]
fn a_dying_protector_refuses_the_hit() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let heart = BlockPos { x: 4, y: 64, z: -2 };
    let entity = store.get_mut(id).expect("the creaking is live");
    assert!(entity.set_creaking_transient(heart));
    assert_eq!(
        entity.creaking_hurt_decision(false, false, true, true),
        CreakingHurtDecision::Absorb
    );
    entity
        .living_mut()
        .expect("the creaking is living")
        .set_health(0.0);
    assert!(entity
        .living()
        .is_some_and(|living| living.is_dead_or_dying()));
    assert_eq!(
        entity.creaking_hurt_decision(false, false, true, true),
        CreakingHurtDecision::Refuse,
        "`isDeadOrDying` closes the branch"
    );
}

#[test]
fn the_freeze_rows_carry_what_the_ai_step_writes() {
    let (mut store, id) = protector_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get(id).expect("the creaking is live");
    assert!(entity.creaking_can_move(), "`CAN_MOVE` is defined `true`");
    assert!(
        !entity.creaking_is_active(),
        "`IS_ACTIVE` is defined `false`"
    );

    let entity = store.get_mut(id).expect("the creaking is live");
    assert!(entity.set_creaking_can_move(false));
    assert!(entity.set_creaking_active(true));
    assert_eq!(
        entity.take_synched_dirty(),
        vec![
            VANILLA_CREAKING_CAN_MOVE_DATA_ID,
            VANILLA_CREAKING_IS_ACTIVE_DATA_ID,
        ]
    );
    assert!(!entity.set_creaking_can_move(false));
    assert!(entity.take_synched_dirty().is_empty());

    let entity = store.get(id).expect("the creaking is live");
    assert!(!entity.creaking_can_move());
    assert!(entity.creaking_is_active());
    let published = non_default_synched_values(entity);
    let ids: Vec<(u8, EntityDataValueKind)> = published
        .iter()
        .map(|row| (row.data_id, row.value.clone()))
        .collect();
    assert_eq!(
        ids,
        vec![
            (
                VANILLA_CREAKING_CAN_MOVE_DATA_ID,
                EntityDataValueKind::Boolean(false)
            ),
            (
                VANILLA_CREAKING_IS_ACTIVE_DATA_ID,
                EntityDataValueKind::Boolean(true)
            ),
        ]
    );
}
