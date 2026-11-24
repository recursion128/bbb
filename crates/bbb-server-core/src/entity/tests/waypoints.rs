use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
use bbb_protocol::packets::{Vec3d, WaypointIcon};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;

use crate::entity::living::refresh_dirty_attributes;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityData, EntityIdCounter, EntityLifecycleEvent, EntityStore};

fn position(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 0.5 }
}

fn pig_store() -> (EntityStore, i32) {
    let mut store = EntityStore::new(0x5741_5950_4f49_4e54, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position(0.5))
        .expect("pig is a stored living entity");
    (store, id)
}

fn enable_waypoint_transmission(store: &mut EntityStore, id: i32) {
    let attributes = store
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap()
        .attributes_mut();
    attributes
        .get_instance(VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
        .expect("pig has the generated waypoint attribute")
        .set_base_value(48.0);
    attributes.drain_attributes_to_update();
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn saved_locator_icon(store: &EntityStore) -> Option<NbtSummaryValue> {
    write_entity_chunk(store, (0, 0))
        .expect("living entity save is canonical")
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .find(|entry| entry.name == "locator_bar_icon")
        .map(|entry| entry.value.clone())
}

#[test]
fn waypoint_fresh_icon_defaults_and_is_omitted_from_save() {
    let (store, id) = pig_store();
    assert_eq!(
        store.get(id).unwrap().living().unwrap().waypoint_icon(),
        &WaypointIcon {
            style: "minecraft:default".to_string(),
            color_rgb: None,
        }
    );
    assert_eq!(saved_locator_icon(&store), None);
}

#[test]
fn waypoint_non_default_style_and_explicit_rgb_use_the_typed_save_shape() {
    let (mut store, id) = pig_store();
    store
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_waypoint_icon(WaypointIcon {
            style: "minecraft:green_arrow".to_string(),
            color_rgb: None,
        });
    assert_eq!(
        saved_locator_icon(&store),
        Some(NbtSummaryValue::Compound(vec![entry(
            "style",
            NbtSummaryValue::string("minecraft:green_arrow"),
        )]))
    );

    store
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_waypoint_icon(WaypointIcon {
            style: "minecraft:default".to_string(),
            color_rgb: Some(0x12_34_56),
        });
    assert_eq!(
        saved_locator_icon(&store),
        Some(NbtSummaryValue::Compound(vec![
            entry("style", NbtSummaryValue::string("minecraft:default"),),
            entry("color", NbtSummaryValue::Int(0x12_34_56)),
        ]))
    );
}

#[test]
fn waypoint_icon_round_trips_without_an_opaque_duplicate() {
    let (store, _) = pig_store();
    let document = write_entity_chunk(&store, (0, 0)).expect("fresh pig saves");
    let mut raw = document.into_raw();
    let entities = raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Entities")
        .expect("entity chunk has an entity list");
    let NbtSummaryValue::List(records) = &mut entities.value else {
        panic!("Entities is a list")
    };
    let NbtSummaryValue::Compound(record) = &mut records[0] else {
        panic!("pig record is a compound")
    };
    record.push(entry(
        "locator_bar_icon",
        NbtSummaryValue::Compound(vec![
            entry("style", NbtSummaryValue::string("minecraft:green_arrow")),
            entry("color", NbtSummaryValue::Int(0xff_80_01)),
        ]),
    ));
    let document = Java26EntityChunkData::parse(raw, 0, 0).expect("locator icon is valid");
    let mut reloaded = EntityStore::new(1, EntityIdCounter::new());
    let [id] = read_entity_chunk(&mut reloaded, &document)
        .expect("locator icon reads")
        .try_into()
        .expect("one pig reloads");
    assert_eq!(
        reloaded.get(id).unwrap().living().unwrap().waypoint_icon(),
        &WaypointIcon {
            style: "minecraft:green_arrow".to_string(),
            color_rgb: Some(0xff_80_01),
        }
    );
    let EntityData::GeneralLiving(data) = reloaded.get(id).unwrap().data() else {
        panic!("pig remains general living")
    };
    assert!(data
        .opaque_fields()
        .iter()
        .all(|entry| entry.name != "locator_bar_icon"));
    let rewritten = write_entity_chunk(&reloaded, (0, 0)).expect("typed icon rewrites");
    assert_eq!(
        rewritten
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .filter(|entry| entry.name == "locator_bar_icon")
            .count(),
        1
    );
}

#[test]
fn waypoint_first_tick_position_change_does_not_publish_an_update() {
    let (mut store, id) = pig_store();
    store.drain_lifecycle_events();
    assert!(store.get(id).unwrap().is_first_tick());
    enable_waypoint_transmission(&mut store, id);
    store.get_mut(id).unwrap().set_pos(position(0.75));
    store.refile(id);
    assert!(store.drain_lifecycle_events().is_empty());
}

#[test]
fn waypoint_same_section_and_cross_section_moves_publish_once_after_commit() {
    let (mut store, id) = pig_store();
    store.drain_lifecycle_events();
    store.get_mut(id).unwrap().first_tick = false;
    enable_waypoint_transmission(&mut store, id);

    store.get_mut(id).unwrap().set_pos(position(0.75));
    store.refile(id);
    assert_eq!(store.get(id).unwrap().position(), position(0.75));
    assert_eq!(
        store.drain_lifecycle_events(),
        vec![EntityLifecycleEvent::WaypointUpdated { entity_id: id }]
    );

    store.get_mut(id).unwrap().set_pos(position(16.5));
    store.get_mut(id).unwrap().set_pos(position(17.5));
    store.refile(id);
    assert_eq!(store.get(id).unwrap().position(), position(17.5));
    assert_eq!(
        store.drain_lifecycle_events(),
        vec![
            EntityLifecycleEvent::Moved {
                entity_id: id,
                from: crate::entity::SectionKey::new(0, 4, 0),
                to: crate::entity::SectionKey::new(1, 4, 0),
            },
            EntityLifecycleEvent::WaypointUpdated { entity_id: id },
        ]
    );
}

#[test]
fn waypoint_transmit_attribute_dirty_publishes_a_reevaluation() {
    let (mut store, id) = pig_store();
    store.drain_lifecycle_events();
    let mut entity = store.detach(id).unwrap();
    entity.first_tick = false;
    assert_eq!(entity.living().unwrap().waypoint_transmit_range(), 0.0);
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
        .expect("pig has the generated waypoint attribute")
        .set_base_value(48.0);
    refresh_dirty_attributes(&mut entity);
    assert_eq!(entity.living().unwrap().waypoint_transmit_range(), 48.0);
    store.attach(entity);
    assert_eq!(
        store.drain_lifecycle_events(),
        vec![EntityLifecycleEvent::WaypointUpdated { entity_id: id }]
    );
}
