use bbb_protocol::builtin_enum_component_values::{
    VANILLA_DYE_COLOR_BLACK_ID, VANILLA_DYE_COLOR_RED_ID, VANILLA_SALMON_VARIANT_LARGE_ID,
    VANILLA_SALMON_VARIANT_MEDIUM_ID, VANILLA_SALMON_VARIANT_SMALL_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_LIVING_ENTITY_HEALTH_DATA_ID, VANILLA_SALMON_VARIANT_DATA_ID,
    VANILLA_TROPICAL_FISH_VARIANT_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_SALMON_ID, VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
use bbb_protocol::tropical_fish_patterns::VANILLA_TROPICAL_FISH_PATTERN_CLAYFISH_PACKED_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::EntityLifecycleEvent;
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

use super::{
    read_save_data, synched_values, FishLeafState, FishSpawnKind, SalmonVariant,
    TropicalFishVariant,
};

fn position() -> Vec3d {
    Vec3d {
        x: 3.25,
        y: 62.0,
        z: -7.75,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn fish_leaf_save_defaults_and_variants() {
    assert_eq!(
        read_save_data(VANILLA_ENTITY_TYPE_SALMON_ID, &[]),
        Some(FishLeafState::Salmon(SalmonVariant::Medium))
    );
    for (variant, name, id) in [
        (
            SalmonVariant::Small,
            "small",
            VANILLA_SALMON_VARIANT_SMALL_ID,
        ),
        (
            SalmonVariant::Medium,
            "medium",
            VANILLA_SALMON_VARIANT_MEDIUM_ID,
        ),
        (
            SalmonVariant::Large,
            "large",
            VANILLA_SALMON_VARIANT_LARGE_ID,
        ),
    ] {
        assert_eq!(variant.serialized_name(), name);
        assert_eq!(variant.id(), id);
        assert_eq!(
            read_save_data(
                VANILLA_ENTITY_TYPE_SALMON_ID,
                &[entry("type", NbtSummaryValue::string(name))],
            ),
            Some(FishLeafState::Salmon(variant))
        );
    }
}

#[test]
fn fish_construction_publishes_exact_leaf_state_tracking_and_save() {
    let tropical = TropicalFishVariant::new(
        VANILLA_TROPICAL_FISH_PATTERN_CLAYFISH_PACKED_ID,
        VANILLA_DYE_COLOR_RED_ID,
        VANILLA_DYE_COLOR_BLACK_ID,
    )
    .unwrap();
    for (seed, kind, entity_type_id) in [
        (71, FishSpawnKind::Cod, VANILLA_ENTITY_TYPE_COD_ID),
        (
            72,
            FishSpawnKind::Salmon(SalmonVariant::Large),
            VANILLA_ENTITY_TYPE_SALMON_ID,
        ),
        (
            73,
            FishSpawnKind::Tropical(tropical),
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        ),
    ] {
        let mut store = EntityStore::new(seed, EntityIdCounter::new());
        let id = store.spawn_fish(position(), 137.0, kind).unwrap();
        let events = store.drain_lifecycle_events();
        let [EntityLifecycleEvent::Added { entity: added, .. }] = events.as_slice() else {
            panic!("one complete fish snapshot must be published")
        };
        assert_eq!(added.id(), id);
        assert_eq!(added.entity_type_id(), entity_type_id);
        assert_eq!(added.position(), position());
        assert_eq!((added.y_rot(), added.x_rot()), (137.0, 0.0));
        let EntityData::GeneralLiving(data) = added.data() else {
            unreachable!()
        };
        assert_eq!(data.is_from_bucket(), Some(false));
        let expected_leaf = match kind {
            FishSpawnKind::Cod => None,
            FishSpawnKind::Salmon(variant) => Some(FishLeafState::Salmon(variant)),
            FishSpawnKind::Tropical(variant) => Some(FishLeafState::Tropical(variant)),
        };
        assert_eq!(data.fish_leaf(), expected_leaf);
        let changed = non_default_synched_values(added);
        assert!(changed.iter().any(|value| {
            value.data_id == VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
                && value.value == EntityDataValueKind::Float(3.0)
        }));
        match kind {
            FishSpawnKind::Cod => assert_eq!(changed.len(), 1),
            FishSpawnKind::Salmon(_) => assert!(changed.iter().any(|value| {
                value.data_id == VANILLA_SALMON_VARIANT_DATA_ID
                    && value.value == EntityDataValueKind::Int(VANILLA_SALMON_VARIANT_LARGE_ID)
            })),
            FishSpawnKind::Tropical(variant) => assert!(changed.iter().any(|value| {
                value.data_id == VANILLA_TROPICAL_FISH_VARIANT_DATA_ID
                    && value.value == EntityDataValueKind::Int(variant.packed())
            })),
        }
        assert_eq!(
            changed.len(),
            usize::from(!matches!(kind, FishSpawnKind::Cod)) + 1
        );

        let chunk = write_entity_chunk(&store, (0, -1)).unwrap();
        let fields = chunk.entities().next().unwrap().raw();
        match kind {
            FishSpawnKind::Cod => {
                assert!(fields
                    .iter()
                    .all(|field| field.name != "type" && field.name != "Variant"));
            }
            FishSpawnKind::Salmon(_) => assert!(fields.iter().any(|field| {
                field.name == "type" && field.value == NbtSummaryValue::string("large")
            })),
            FishSpawnKind::Tropical(variant) => assert!(fields.iter().any(|field| {
                field.name == "Variant" && field.value == NbtSummaryValue::Int(variant.packed())
            })),
        }
    }
}

#[test]
fn schooling_links_and_counts_are_transient_across_save_restore() {
    let mut store = EntityStore::new(77, EntityIdCounter::new());
    let id = store
        .spawn_fish(position(), 0.0, FishSpawnKind::Cod)
        .unwrap();
    assert!(crate::entity::general_living::set_schooling_fish_leader(
        store.get_mut(id).unwrap(),
        Some(Uuid::from_u128(91)),
    ));
    assert!(
        crate::entity::general_living::increment_schooling_fish_size(store.get_mut(id).unwrap(),)
    );
    let chunk = write_entity_chunk(&store, (0, -1)).unwrap();

    let mut restored = EntityStore::new(78, EntityIdCounter::new());
    let roots = read_entity_chunk(&mut restored, &chunk).unwrap();
    let state = crate::entity::schooling_fish_state(restored.get(roots[0]).unwrap()).unwrap();
    assert_eq!(state.leader_uuid(), None);
    assert_eq!(state.school_size(), 1);
}

#[test]
fn fish_leaf_tracking_defaults_match_generated_accessors() {
    for (state, data_id, value, is_default) in [
        (
            FishLeafState::Salmon(SalmonVariant::Medium),
            VANILLA_SALMON_VARIANT_DATA_ID,
            VANILLA_SALMON_VARIANT_MEDIUM_ID,
            true,
        ),
        (
            FishLeafState::Tropical(TropicalFishVariant::default()),
            VANILLA_TROPICAL_FISH_VARIANT_DATA_ID,
            0,
            true,
        ),
        (
            FishLeafState::Salmon(SalmonVariant::Small),
            VANILLA_SALMON_VARIANT_DATA_ID,
            VANILLA_SALMON_VARIANT_SMALL_ID,
            false,
        ),
    ] {
        let [row] = synched_values(state);
        assert_eq!(row.value.data_id, data_id);
        assert_eq!(row.value.value, EntityDataValueKind::Int(value));
        assert_eq!(row.is_default, is_default);
    }
}

#[path = "tests/geometry.rs"]
mod geometry;
