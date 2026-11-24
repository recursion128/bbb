//! Shrieker payload and listener admission rules.

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_SCULK_SHRIEKER_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::{removal_response, SculkShriekerBlockEntity, ShriekerVibration, TAG_WARNING_LEVEL};
use crate::block_entity::vibration::{GameEventTag, VibrationData, VibrationSystem};
use crate::block_entity::{new_block_entity, unique_field, BlockEntityPayload};

const TENDRILS: &str = "minecraft:sculk_sensor_tendrils_clicking";

fn pos() -> BlockPos {
    BlockPos { x: 2, y: 3, z: 4 }
}

fn sculk_shrieker() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_SCULK_SHRIEKER_ID)
        .expect("minecraft:sculk_shrieker has a default state")
}

fn state_of(properties: &[(&str, &str)]) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:sculk_shrieker",
        &properties
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .chain([("waterlogged".to_string(), "false".to_string())])
            .collect(),
    )
    .expect("built-in Java 26.1 block state")
}

fn upsert(record: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(existing) = record.iter_mut().find(|entry| entry.name == name) {
        existing.value = value;
    } else {
        record.push(crate::block_entity::entry(name, value));
    }
}

#[test]
fn only_a_shrieking_shrieker_answers_on_removal() {
    let mut shrieker = SculkShriekerBlockEntity::default();
    shrieker.set_warning_level_for_test(3);
    let shrieking = state_of(&[("can_summon", "true"), ("shrieking", "true")]);
    assert_eq!(
        removal_response(&shrieker, shrieking),
        crate::block_entity::BlockEntityRemovalDrops::SculkShriekerResponse {
            warning_level: 3,
            can_summon: true,
        }
    );
    let idle = state_of(&[("can_summon", "true"), ("shrieking", "false")]);
    assert_eq!(
        removal_response(&shrieker, idle),
        crate::block_entity::BlockEntityRemovalDrops::None
    );
    let closed = state_of(&[("can_summon", "false"), ("shrieking", "true")]);
    assert_eq!(
        removal_response(&shrieker, closed),
        crate::block_entity::BlockEntityRemovalDrops::SculkShriekerResponse {
            warning_level: 3,
            can_summon: false,
        }
    );
}

#[test]
fn sculk_shrieker_save_load_round_trips_warning_level_and_listener() {
    let state = sculk_shrieker();
    let mut record = new_block_entity(pos(), state)
        .expect("a sculk shrieker builds")
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    let mut data = VibrationData::default();
    data.set_current_vibration(Some(crate::block_entity::vibration::VibrationInfo {
        game_event: TENDRILS,
        distance: 2.5,
        pos: Vec3d {
            x: 0.5,
            y: 64.5,
            z: 0.5,
        },
        source: Some(Uuid::from_u128(5)),
        projectile_owner: None,
    }));
    data.set_travel_time_in_ticks(2);
    upsert(&mut record, TAG_WARNING_LEVEL, NbtSummaryValue::Int(3));
    upsert(
        &mut record,
        crate::block_entity::vibration::TAG_LISTENER,
        NbtSummaryValue::Compound(data.save()),
    );
    let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
        .expect("fixture codec succeeds")
        .expect("shrieker loads");
    let saved = loaded
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert_eq!(
        unique_field(&saved, TAG_WARNING_LEVEL),
        Some(&NbtSummaryValue::Int(3))
    );
    let shrieker = match loaded.payload() {
        BlockEntityPayload::SculkShrieker(payload) => payload,
        other => panic!("expected a sculk shrieker payload, got {other:?}"),
    };
    assert_eq!(shrieker.warning_level(), 3);
    assert_eq!(
        shrieker.vibration_data().current_vibration(),
        data.current_vibration()
    );
    assert_eq!(
        shrieker.vibration_data().travel_time_in_ticks(),
        data.travel_time_in_ticks()
    );
    assert_eq!(
        shrieker.vibration_data().selection_strategy(),
        data.selection_strategy()
    );
}

#[test]
fn the_shrieker_listens_only_to_the_shrieker_tag() {
    let mut shrieker = SculkShriekerBlockEntity::default();
    let view = ShriekerVibration {
        pos: pos(),
        shrieker: &mut shrieker,
    };
    assert_eq!(
        view.vibration_listenable_events(),
        GameEventTag::ShriekerCanListen
    );
    let tag = view.vibration_listenable_events();
    assert!(tag.contains(TENDRILS));
    for event in [
        "minecraft:block_place",
        "minecraft:entity_die",
        "minecraft:step",
        "minecraft:sculk_sensor_tendrils_clicking_x",
    ] {
        assert!(!tag.contains(event));
    }
}
