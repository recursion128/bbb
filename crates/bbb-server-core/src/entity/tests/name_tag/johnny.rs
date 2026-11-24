use super::*;

use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VINDICATOR_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn spawn(store: &mut EntityStore) -> i32 {
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_VINDICATOR_ID, pig_position())
        .unwrap()
}

fn johnny(store: &EntityStore, id: i32) -> bool {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        panic!()
    };
    assert!(!data
        .opaque_fields()
        .iter()
        .any(|field| field.name == "Johnny"));
    data.raider_retained().unwrap().johnny().unwrap()
}

fn saved_fields(store: &EntityStore, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(fields) = store.command_data(id).unwrap() else {
        panic!()
    };
    fields
}

fn assert_written(store: &EntityStore, id: i32, expected: bool) {
    assert_eq!(johnny(store, id), expected);
    let fields = saved_fields(store, id);
    let flags: Vec<_> = fields
        .iter()
        .filter(|entry| entry.name == "Johnny")
        .collect();
    assert_eq!(flags.len(), usize::from(expected));
    if expected {
        assert_eq!(flags[0].value, NbtSummaryValue::Byte(1));
    }
}

fn component(fields: Vec<NbtSummaryEntry>) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(fields)).unwrap()
}

#[test]
fn actual_name_tag_uses_server_component_string_and_consumes_once() {
    let cases = [
        (RichTextComponent::plain("Johnny"), true),
        (
            component(vec![
                entry("text", NbtSummaryValue::String("John".into())),
                entry("bold", NbtSummaryValue::Byte(1)),
                entry(
                    "extra",
                    NbtSummaryValue::List(vec![NbtSummaryValue::String("ny".into())]),
                ),
            ]),
            true,
        ),
        (RichTextComponent::plain("johnny"), false),
    ];
    for (name, expected) in cases {
        let mut store = EntityStore::new(74, EntityIdCounter::new());
        let id = spawn(&mut store);
        let mut hand = named_name_tag(2, "placeholder");
        hand.component_patch.custom_name_nbt = Some(name.component().clone());
        let outcome = leash::interact_leash(&mut store, id, &mut interacting_player(&mut hand));
        assert_eq!(outcome, leash::LeashInteraction::Named);
        assert_eq!(hand.count, 1);
        assert_eq!(store.get(id).unwrap().custom_name(), Some(&name));
        assert!(pig_persistence_required(&store, id));
        assert_written(&store, id, expected);
    }
}

#[test]
fn loaded_johnny_overwrites_name_activation_but_an_explicit_same_name_setter_can_enable_it() {
    for value in [
        None,
        Some(NbtSummaryValue::Byte(0)),
        Some(NbtSummaryValue::Byte(1)),
    ] {
        let mut initial = EntityStore::new(75, EntityIdCounter::new());
        let id = spawn(&mut initial);
        let mut fields = write_entity_chunk(&initial, (0, 0))
            .unwrap()
            .entities()
            .next()
            .unwrap()
            .raw()
            .to_vec();
        fields.push(entry(
            "CustomName",
            NbtSummaryValue::String("Johnny".into()),
        ));
        if let Some(value) = &value {
            fields.push(entry("Johnny", value.clone()));
        }
        let chunk = Java26EntityChunkData::parse(
            NamedCompound::new(
                "",
                vec![
                    entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                    entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                    entry(
                        "Entities",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(fields.clone())]),
                    ),
                ],
            ),
            0,
            0,
        )
        .unwrap();
        let mut cold = EntityStore::new(76, EntityIdCounter::new());
        let cold_id = read_entity_chunk(&mut cold, &chunk).unwrap()[0];
        let expected = value == Some(NbtSummaryValue::Byte(1));
        assert_written(&cold, cold_id, expected);
        initial.load_command_data(id, &fields).unwrap();
        assert_written(&initial, id, expected);
        let name = initial.get(id).unwrap().custom_name().cloned();
        assert!(initial.get_mut(id).unwrap().set_custom_name(name));
        assert_written(&initial, id, true);
        assert!(initial
            .get_mut(id)
            .unwrap()
            .set_custom_name(Some(RichTextComponent::plain("other"))));
        assert!(initial.get_mut(id).unwrap().set_custom_name(None));
        assert_written(&initial, id, true);
    }
}
