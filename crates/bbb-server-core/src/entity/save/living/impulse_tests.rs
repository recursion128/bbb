use super::*;
use crate::entity::living::FallImpulseContext;
use crate::entity::store::EntityIdCounter;

const IMPACT: &str = "current_explosion_impact_pos";
const GRACE: &str = "current_impulse_context_reset_grace_time";

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn fixture(armor_stand: bool) -> (EntityStore, i32) {
    let mut store = EntityStore::new(47, EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = if armor_stand {
        crate::entity::spawn_armor_stand(&mut store, position)
    } else {
        store.spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            position,
        )
    }
    .expect("living entity is stored");
    (store, id)
}

fn command_entries(store: &EntityStore, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = store.command_data(id).expect("query succeeds") else {
        panic!("command data is a compound")
    };
    entries
}

fn context(store: &EntityStore, id: i32) -> FallImpulseContext {
    store.get(id).unwrap().living().unwrap().fall_impulse
}

fn set_context(store: &mut EntityStore, id: i32, context: FallImpulseContext) {
    store
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap()
        .fall_impulse = context;
}

fn assert_fields(entries: &[NbtSummaryEntry], expected: FallImpulseContext) {
    let grace: Vec<_> = entries.iter().filter(|entry| entry.name == GRACE).collect();
    assert_eq!(grace.len(), 1);
    assert_eq!(
        grace[0].value,
        NbtSummaryValue::Int(expected.reset_grace_time)
    );
    let impact: Vec<_> = entries
        .iter()
        .filter(|entry| entry.name == IMPACT)
        .collect();
    if let Some(position) = expected.impact_position {
        assert_eq!(impact.len(), 1);
        assert_eq!(impact[0].value, double_list(position));
    } else {
        assert!(impact.is_empty());
    }
}

#[test]
fn stored_impulse_save_roundtrips_general_living_and_armor_stand() {
    for armor_stand in [false, true] {
        for grace in [-7, 40] {
            let (mut store, id) = fixture(armor_stand);
            let expected = FallImpulseContext {
                reset_grace_time: grace,
                impact_position: Some(Vec3d {
                    x: -0.0,
                    y: 80.25,
                    z: 1.0e200,
                }),
            };
            set_context(&mut store, id, expected);
            assert_fields(&command_entries(&store, id), expected);
            let chunk = write_entity_chunk(&store, (0, 0)).expect("finite context saves");
            assert_fields(chunk.entities().next().unwrap().raw(), expected);
            let mut reloaded = EntityStore::new(53, EntityIdCounter::new());
            let ids = read_entity_chunk(&mut reloaded, &chunk).expect("context reloads");
            assert_eq!(ids.len(), 1);
            assert_eq!(context(&reloaded, ids[0]), expected);
            assert_fields(&command_entries(&reloaded, ids[0]), expected);
        }
    }
}

#[test]
fn stored_impulse_save_defaults_and_single_field_owner() {
    for armor_stand in [false, true] {
        let (mut store, id) = fixture(armor_stand);
        assert_fields(&command_entries(&store, id), FallImpulseContext::default());
        let expected = FallImpulseContext {
            reset_grace_time: -3,
            impact_position: Some(Vec3d {
                x: 1.0,
                y: 70.0,
                z: 3.0,
            }),
        };
        let mut entries = command_entries(&store, id);
        entries.retain(|entry| entry.name != GRACE);
        entries.push(entry(
            GRACE,
            NbtSummaryValue::Int(expected.reset_grace_time),
        ));
        entries.push(entry(
            IMPACT,
            double_list(expected.impact_position.unwrap()),
        ));
        store.load_command_data(id, &entries).unwrap();
        assert_eq!(context(&store, id), expected);
        assert_fields(&command_entries(&store, id), expected);
        set_context(&mut store, id, FallImpulseContext::default());
        let chunk = write_entity_chunk(&store, (0, 0)).unwrap();
        assert_fields(
            chunk.entities().next().unwrap().raw(),
            FallImpulseContext::default(),
        );
    }
}

#[test]
fn stored_impulse_save_command_context_wins_over_previous_transients() {
    for armor_stand in [false, true] {
        let (mut store, id) = fixture(armor_stand);
        set_context(
            &mut store,
            id,
            FallImpulseContext {
                reset_grace_time: 40,
                impact_position: Some(Vec3d {
                    x: 10.0,
                    y: 90.0,
                    z: 30.0,
                }),
            },
        );
        let living = store.get_mut(id).unwrap().living_mut().unwrap();
        living.hurt_duration = 17;
        living.invulnerable_time = 13;
        let expected = FallImpulseContext {
            reset_grace_time: -19,
            impact_position: Some(Vec3d {
                x: -1.0,
                y: 74.0,
                z: -3.0,
            }),
        };
        let mut entries = command_entries(&store, id);
        entries.retain(|entry| entry.name != IMPACT && entry.name != GRACE);
        entries.push(entry(
            GRACE,
            NbtSummaryValue::Int(expected.reset_grace_time),
        ));
        entries.push(entry(
            IMPACT,
            double_list(expected.impact_position.unwrap()),
        ));
        store.load_command_data(id, &entries).unwrap();
        assert_eq!(context(&store, id), expected);
        let living = store.get(id).unwrap().living().unwrap();
        assert_eq!(living.hurt_duration, 17);
        assert_eq!(living.invulnerable_time, 13);
        entries.retain(|entry| entry.name != IMPACT && entry.name != GRACE);
        store.load_command_data(id, &entries).unwrap();
        assert_eq!(context(&store, id), FallImpulseContext::default());
        assert_fields(&command_entries(&store, id), FallImpulseContext::default());
    }
}
