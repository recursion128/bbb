use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, VANILLA_HORSE_VARIANT_DATA_ID,
    VANILLA_LLAMA_STRENGTH_DATA_ID, VANILLA_LLAMA_VARIANT_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::*;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn spawn_record(entity_type_id: i32) -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x48_4f_52_53, EntityIdCounter::new());
    store
        .spawn_general_living(entity_type_id, position())
        .expect("horse family is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn horse_phenotype_spawn_defaults() {
    let horse = spawn_record(VANILLA_ENTITY_TYPE_HORSE_ID);
    assert_eq!(field(&horse, "Variant"), Some(&NbtSummaryValue::Int(0)));
    assert!(field(&horse, "ChestedHorse").is_none());
    assert!(field(&horse, "Strength").is_none());

    let donkey = spawn_record(VANILLA_ENTITY_TYPE_DONKEY_ID);
    assert_eq!(
        field(&donkey, "ChestedHorse"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert!(field(&donkey, "Variant").is_none());

    let llama = spawn_record(VANILLA_ENTITY_TYPE_LLAMA_ID);
    assert_eq!(
        field(&llama, "ChestedHorse"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(field(&llama, "Variant"), Some(&NbtSummaryValue::Int(0)));
    assert_eq!(field(&llama, "Strength"), Some(&NbtSummaryValue::Int(1)));
}

fn round_trip(entity_type_id: i32) -> HorsePhenotypeState {
    let mut store = EntityStore::new(0x48_4f_52_53, EntityIdCounter::new());
    store
        .spawn_general_living(entity_type_id, position())
        .expect("horse family is GeneralLiving");
    let data = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut restored = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &data).unwrap()[0];
    let EntityData::GeneralLiving(living) = restored.get(id).unwrap().data() else {
        unreachable!()
    };
    living.horse_phenotype().expect("phenotype restored")
}

#[test]
fn horse_phenotype_restore_round_trips() {
    assert_eq!(
        round_trip(VANILLA_ENTITY_TYPE_HORSE_ID),
        HorsePhenotypeState::Horse { packed_variant: 0 }
    );
    assert_eq!(
        round_trip(VANILLA_ENTITY_TYPE_DONKEY_ID),
        HorsePhenotypeState::Chested { has_chest: false }
    );
    assert_eq!(
        round_trip(VANILLA_ENTITY_TYPE_LLAMA_ID),
        HorsePhenotypeState::Llama {
            has_chest: false,
            strength: 1,
            variant: 0,
        }
    );

    assert_eq!(
        read_save_data(
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            &[
                entry("ChestedHorse", NbtSummaryValue::Byte(1)),
                entry("Strength", NbtSummaryValue::Int(5)),
                entry("Variant", NbtSummaryValue::Int(3)),
            ],
        ),
        Some(HorsePhenotypeState::Llama {
            has_chest: true,
            strength: 5,
            variant: 3,
        })
    );
}

#[test]
fn horse_phenotype_runtime_tracking_serializers_and_defaults_are_exact() {
    let mut store = EntityStore::new(6, EntityIdCounter::new());
    let horse_id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HORSE_ID, position())
        .unwrap();
    assert!(non_default_synched_values(store.get(horse_id).unwrap())
        .iter()
        .all(|value| value.data_id != VANILLA_HORSE_VARIANT_DATA_ID));
    assert!(set_state(
        store.get_mut(horse_id).unwrap(),
        HorsePhenotypeState::Horse { packed_variant: 7 }
    ));
    let horse = write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec();
    assert_eq!(field(&horse, "Variant"), Some(&NbtSummaryValue::Int(7)));
    let tracked = non_default_synched_values(store.get(horse_id).unwrap());
    let variant = tracked
        .iter()
        .find(|value| value.data_id == VANILLA_HORSE_VARIANT_DATA_ID)
        .expect("horse variant is dirty");
    assert_eq!(variant.serializer_id, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID);
    assert_eq!(variant.value, EntityDataValueKind::Int(7));

    let donkey_id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_DONKEY_ID, position())
        .unwrap();
    assert!(set_state(
        store.get_mut(donkey_id).unwrap(),
        HorsePhenotypeState::Chested { has_chest: true }
    ));
    let tracked = non_default_synched_values(store.get(donkey_id).unwrap());
    let chest = tracked
        .iter()
        .find(|value| value.data_id == VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID)
        .expect("donkey chest is dirty");
    assert_eq!(
        chest.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
    );
    assert_eq!(chest.value, EntityDataValueKind::Boolean(true));

    let llama_id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, position())
        .unwrap();
    assert!(set_state(
        store.get_mut(llama_id).unwrap(),
        HorsePhenotypeState::Llama {
            has_chest: true,
            strength: 5,
            variant: 3,
        }
    ));
    let tracked = non_default_synched_values(store.get(llama_id).unwrap());
    let ids = tracked
        .iter()
        .map(|value| value.data_id)
        .filter(|id| {
            matches!(
                *id,
                VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID
                    | VANILLA_LLAMA_STRENGTH_DATA_ID
                    | VANILLA_LLAMA_VARIANT_DATA_ID
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID,
            VANILLA_LLAMA_STRENGTH_DATA_ID,
            VANILLA_LLAMA_VARIANT_DATA_ID,
        ]
    );
}

#[test]
fn horse_finalize_accepts_the_default_white_unmarked_phenotype() {
    // A draw of variant 0 with markings 0 is the fresh horse's own state.
    let seed = (0..)
        .find(|&seed| {
            let random = GameRandom::with_seed(seed);
            random.next_int(7) == 0 && random.next_int(5) == 0
        })
        .unwrap();
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let mut horse =
        crate::entity::save::create_fresh_entity(&mut store, "minecraft:horse", position())
            .unwrap();
    assert_eq!(
        finalize_group_horse_phenotype(&mut horse, &GameRandom::with_seed(seed), None),
        Some(0)
    );
}
