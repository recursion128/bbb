use super::*;
use crate::entity::ai::mob::target::{sense, TargetRef};
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::Entity;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::VANILLA_BEE_ANGER_END_TIME_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_COW_ID};

fn reference(store: &EntityStore, id: i32) -> TargetRef {
    let entity = store.get(id).unwrap();
    TargetRef {
        entity_id: id,
        uuid: entity.uuid(),
    }
}

fn fixture() -> (EntityStore, NativeLevel, i32, i32, i32) {
    let (level, _sender, _receiver) = joined_level();
    let mut store = EntityStore::new(83, unoccupied_ids(&level));
    store.set_difficulty(Difficulty::Normal);
    let root = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BEE_ID, FEET)
        .unwrap();
    let foreign = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BEE_ID, Vec3d { x: 2.5, ..FEET })
        .unwrap();
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d { x: 4.5, ..FEET })
        .unwrap();
    store.drain_lifecycle_events();
    (store, level, root, foreign, cow)
}

fn typed_anger(entity: &Entity) -> (i64, Option<Uuid>) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        panic!("Bee is GeneralLiving")
    };
    data.bee_retained().unwrap().persistent_anger()
}

fn anger_metadata(entity: &Entity) -> crate::entity::living::SynchedValue {
    let EntityData::GeneralLiving(data) = entity.data() else {
        panic!("Bee is GeneralLiving")
    };
    crate::entity::bee::synched_values(data.bee_retained().unwrap())
        .into_iter()
        .find(|row| row.value.data_id == VANILLA_BEE_ANGER_END_TIME_DATA_ID)
        .unwrap()
}

#[test]
fn hurt_bee_root_acquires_live_stored_attacker_and_refreshes_typed_anger() {
    let (mut store, mut level, root, _, cow) = fixture();
    let target = reference(&store, cow);
    store
        .get_mut(root)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_last_hurt_by_mob(
            Some(crate::entity::LastHurtByMob::Stored {
                entity_id: cow,
                uuid: Some(target.uuid),
            }),
            1,
        );
    assert_eq!(store.mob_ai(root).unwrap().target_ref(), None);
    level.with_level(|ctx| crate::entity::passenger::tick_root(&mut store, ctx, root));
    assert_eq!(store.mob_ai(root).unwrap().target_ref(), Some(target));
    level.with_level(|ctx| crate::entity::passenger::tick_root(&mut store, ctx, root));
    assert_eq!(store.mob_ai(root).unwrap().target_ref(), Some(target));
    let body = store.get(root).unwrap();
    let (end_time, angry_at) = typed_anger(body);
    assert_eq!(angry_at, Some(target.uuid));
    assert!(end_time > level.world.game_time());
    assert_eq!(
        anger_metadata(body).value.value,
        bbb_protocol::packets::EntityDataValueKind::Long(end_time)
    );
    assert!(body.synched_dirty[0] & (1u64 << VANILLA_BEE_ANGER_END_TIME_DATA_ID) != 0);
}

fn angry_at_field(uuid: Uuid) -> bbb_protocol::NbtSummaryEntry {
    let bits = uuid.as_u128();
    bbb_protocol::NbtSummaryEntry {
        name: "angry_at".into(),
        value: bbb_protocol::NbtSummaryValue::IntArray(vec![
            (bits >> 96) as i32,
            (bits >> 64) as i32,
            (bits >> 32) as i32,
            bits as i32,
        ]),
    }
}

#[test]
fn command_bee_target_changes_immediately_without_tick() {
    let (mut store, mut level, root, _, cow) = fixture();
    let player = TargetRef {
        entity_id: level.world.publication().players[0].entity_id,
        uuid: level.world.publication().players[0].uuid,
    };
    level.with_level(|ctx| sense::set_mob_target(&mut store, ctx, root, Some(player)));
    let bbb_protocol::NbtSummaryValue::Compound(mut fields) = store.command_data(root).unwrap()
    else {
        unreachable!()
    };
    fields.retain(|field| field.name != "angry_at" && field.name != "NoAI");
    fields.push(bbb_protocol::NbtSummaryEntry {
        name: "NoAI".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(1),
    });
    let target = reference(&store, cow);
    fields.push(angry_at_field(target.uuid));
    level
        .with_level(|ctx| store.load_command_data_with_level(root, &fields, ctx))
        .unwrap();
    assert_eq!(store.mob_ai(root).unwrap().target_ref(), Some(target));
    fields.retain(|field| field.name != "angry_at");
    level
        .with_level(|ctx| store.load_command_data_with_level(root, &fields, ctx))
        .unwrap();
    assert_eq!(store.mob_ai(root).unwrap().target_ref(), None);
    assert_eq!(typed_anger(store.get(root).unwrap()).1, None);
}
