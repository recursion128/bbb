use bbb_protocol::entity_synched_data::{
    VANILLA_PANDA_FLAGS_DATA_ID, VANILLA_PANDA_HIDDEN_GENE_DATA_ID,
    VANILLA_PANDA_MAIN_GENE_DATA_ID, VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID,
    VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID;
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::geometry::{mth_cos, mth_sin};
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::{before_hurt, read_save_data, synched_values, tick, PandaGene};
use crate::entity::general_living::AgeableMobRetainedState;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityData, EntityIdCounter, EntityLifecycleEvent, EntityStore};

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

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(field) = entries.iter_mut().find(|entry| entry.name == name) {
        field.value = value;
    } else {
        entries.push(entry(name, value));
    }
}

fn chunk_document(entries: Vec<NbtSummaryEntry>) -> NamedCompound {
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
            entry(
                "Entities",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
            ),
        ],
    )
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(chunk_document(entries), 0, 0).unwrap()
}

fn default_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x50_41_4e_44_41, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PANDA_ID, position())
        .expect("Panda is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn panda_retained_runtime_census_save_restore_and_tracking_are_exact() {
    let record = default_record();
    assert_eq!(
        field(&record, "MainGene"),
        Some(&NbtSummaryValue::string("normal"))
    );
    assert_eq!(
        field(&record, "HiddenGene"),
        Some(&NbtSummaryValue::string("normal"))
    );

    let mut nondefault = record.clone();
    set_field(&mut nondefault, "MainGene", NbtSummaryValue::string("weak"));
    set_field(
        &mut nondefault,
        "HiddenGene",
        NbtSummaryValue::string("brown"),
    );
    let mut restored = EntityStore::new(7, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &chunk(nondefault)).unwrap()[0];
    let EntityData::GeneralLiving(data) = restored.get(id).unwrap().data() else {
        unreachable!()
    };
    let state = data.panda_retained().unwrap();
    assert_eq!(
        (state.main_gene, state.hidden_gene),
        (PandaGene::Weak, PandaGene::Brown)
    );

    let mut missing = record.clone();
    missing.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "MainGene" | "HiddenGene"
        )
    });
    let mut defaults = EntityStore::new(8, EntityIdCounter::new());
    let id = read_entity_chunk(&mut defaults, &chunk(missing)).unwrap()[0];
    let EntityData::GeneralLiving(data) = defaults.get(id).unwrap().data() else {
        unreachable!()
    };
    let state = data.panda_retained().unwrap();
    assert_eq!(
        (state.main_gene, state.hidden_gene),
        (PandaGene::Normal, PandaGene::Normal)
    );

    let mut tracked = read_save_data(&[
        entry("MainGene", NbtSummaryValue::string("lazy")),
        entry("HiddenGene", NbtSummaryValue::string("aggressive")),
    ]);
    tracked.set_counters_and_flags(29, 1, 0, super::FLAG_SNEEZE | super::FLAG_SIT);
    let values = synched_values(&tracked);
    assert_eq!(
        values[0].value.data_id,
        VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID
    );
    assert_eq!(
        values[1].value.data_id,
        VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID
    );
    assert_eq!(values[3].value.data_id, VANILLA_PANDA_MAIN_GENE_DATA_ID);
    assert_eq!(values[4].value.data_id, VANILLA_PANDA_HIDDEN_GENE_DATA_ID);
    assert_eq!(values[5].value.data_id, VANILLA_PANDA_FLAGS_DATA_ID);
    assert_eq!(
        values[3].value.value,
        EntityDataValueKind::Byte(PandaGene::Lazy as i8)
    );
    assert_eq!(
        values[4].value.value,
        EntityDataValueKind::Byte(PandaGene::Aggressive as i8)
    );
}

fn first_roll_delta(age: i32) -> Vec3d {
    let mut store = EntityStore::new(i64::from(age), EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PANDA_ID, position())
        .unwrap();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_rot(90.0, 0.0);
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!()
        };
        *data.ageable_retained_mut().unwrap() = AgeableMobRetainedState::from_saved(age, 0, true);
        data.panda_retained_mut()
            .unwrap()
            .set_counters_and_flags(0, 0, 0, super::FLAG_ROLL);
    }
    let mut entity = store.detach(id).unwrap();
    crate::entity::tests::native_level::NativeLevel::new()
        .with_level(|level| tick(&mut entity, &mut store, level));
    let movement = entity.delta_movement();
    store.attach(entity);
    movement
}

#[test]
fn panda_retained_runtime_roll_uses_exact_baby_and_adult_delta() {
    let java_angle = f64::from(90.0_f32 * (std::f32::consts::PI / 180.0));
    let sin = mth_sin(java_angle);
    let cos = mth_cos(java_angle);
    let baby = first_roll_delta(-100);
    let adult = first_roll_delta(0);
    assert_eq!(
        baby,
        Vec3d {
            x: f64::from(-sin * 0.1_f32),
            y: 0.27,
            z: f64::from(cos * 0.1_f32)
        }
    );
    assert_eq!(
        adult,
        Vec3d {
            x: f64::from(-sin * 0.2_f32),
            y: 0.27,
            z: f64::from(cos * 0.2_f32)
        }
    );
}

#[test]
fn panda_retained_runtime_counters_sounds_and_hurt_sitting_are_ordered() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PANDA_ID, position())
        .unwrap();
    store.drain_lifecycle_events();
    {
        let entity = store.get_mut(id).unwrap();
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!()
        };
        data.panda_retained_mut().unwrap().set_counters_and_flags(
            29,
            0,
            0,
            super::FLAG_SNEEZE | super::FLAG_SIT,
        );
    }
    let mut entity = store.detach(id).unwrap();
    crate::entity::tests::native_level::NativeLevel::new()
        .with_level(|level| tick(&mut entity, &mut store, level));
    let dirty = entity.take_synched_dirty();
    assert!(dirty.contains(&VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID));
    assert!(dirty.contains(&VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID));
    assert_eq!(entity.x_rot(), 0.0);
    before_hurt(&mut entity);
    assert!(entity
        .take_synched_dirty()
        .contains(&VANILLA_PANDA_FLAGS_DATA_ID));
    store.attach(entity);

    let sounds: Vec<_> = store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound.sound),
            _ => None,
        })
        .collect();
    assert_eq!(sounds.len(), 2);
}

#[test]
fn panda_interaction_scared_back_and_locked_baby_stop_food_and_age_lock() {
    use crate::entity::leash::{LeashInteractingPlayer, LeashInteraction};
    use crate::entity::level::EntityRegistryTagView;
    use bbb_protocol::packets::ItemStackSummary;
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PANDA_ID, position())
        .unwrap();
    let mut hand = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BAMBOO_ID),
        count: 3,
        component_patch: Default::default(),
    };
    let mut player = LeashInteractingPlayer {
        mob_item_used: false,
        hand_stack: &mut hand,
        entity_id: 1,
        uuid: uuid::Uuid::nil(),
        position: position(),
        secondary_use_active: false,
        infinite_materials: false,
    };
    {
        let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
            unreachable!()
        };
        let state = data.panda_retained_mut().unwrap();
        state.main_gene = PandaGene::Worried;
        state.flags = super::FLAG_ON_BACK;
    }
    assert_eq!(
        super::mob_interact(
            &mut store,
            id,
            &mut player,
            &EntityRegistryTagView::Builtin26_1,
            true,
            None
        ),
        Some(LeashInteraction::Pass)
    );
    assert_eq!(player.hand_stack.count, 3);
    assert_eq!(
        super::mob_interact(
            &mut store,
            id,
            &mut player,
            &EntityRegistryTagView::Builtin26_1,
            false,
            None
        ),
        Some(LeashInteraction::PandaStoodUp)
    );
    assert_eq!(player.hand_stack.count, 3);
    {
        let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
            unreachable!()
        };
        *data.ageable_retained_mut().unwrap() =
            AgeableMobRetainedState::from_saved(-24_000, 0, true);
    }
    assert_eq!(
        super::mob_interact(
            &mut store,
            id,
            &mut player,
            &EntityRegistryTagView::Builtin26_1,
            false,
            None
        ),
        Some(LeashInteraction::Pass)
    );
    assert_eq!(player.hand_stack.count, 3);
    player.hand_stack.item_id = Some(bbb_protocol::item_types::VANILLA_ITEM_GOLDEN_DANDELION_ID);
    assert_eq!(
        super::mob_interact(
            &mut store,
            id,
            &mut player,
            &EntityRegistryTagView::Builtin26_1,
            true,
            None
        ),
        Some(LeashInteraction::Pass)
    );
    assert!(!player.mob_item_used);
}

#[test]
fn panda_spawn_genes_project_recessive_weakness_and_dominant_laziness() {
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    for (main, hidden, health, speed) in [
        (PandaGene::Weak, PandaGene::Weak, 10.0, 0.15_f32 as f64),
        (PandaGene::Weak, PandaGene::Normal, 20.0, 0.15_f32 as f64),
        (PandaGene::Lazy, PandaGene::Weak, 20.0, 0.07_f32 as f64),
    ] {
        let id = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_PANDA_ID, position())
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        assert!(super::apply_spawn_genes(
            entity,
            super::PandaGenes { main, hidden }
        ));
        let living = entity.living().unwrap();
        assert_eq!(living.max_health(), health);
        assert_eq!(
            living
                .attributes()
                .value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
            Some(speed)
        );
    }
}
