use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_MARKER_ID, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::entity::marker::MarkerData;
use crate::entity::store::{section_of_block, EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::EntityData;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn marker_store(position: Vec3d) -> (EntityStore, i32) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = store
        .restore(
            Uuid::from_u128(0x1234),
            VANILLA_ENTITY_TYPE_MARKER_ID,
            position,
            EntityData::Marker(MarkerData::with_data(vec![entry(
                "answer",
                NbtSummaryValue::Int(42),
            )])),
        )
        .expect("marker is stored");
    (store, id)
}

fn command_entries(store: &EntityStore, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = store.command_data(id).expect("query succeeds") else {
        panic!("command data is a compound")
    };
    entries
}

fn replace(entries: &mut [NbtSummaryEntry], name: &str, value: NbtSummaryValue) {
    entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("{name} is present"))
        .value = value;
}

#[test]
fn command_data_query_is_save_without_type_id() {
    let (store, id) = marker_store(Vec3d {
        x: 1.5,
        y: 64.0,
        z: -2.5,
    });
    let entries = command_entries(&store, id);

    assert!(entries.iter().all(|entry| entry.name != "id"));
    assert!(entries.iter().any(|entry| entry.name == "UUID"));
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "data")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "answer",
            NbtSummaryValue::Int(42),
        )]))
    );
}

#[test]
fn command_data_load_preserves_identity_and_ignores_malicious_identity_fields() {
    let (mut store, id) = marker_store(Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    });
    let before = store.get(id).expect("entity exists").clone();
    let mut entries = command_entries(&store, id);
    entries.push(entry("id", NbtSummaryValue::string("minecraft:item")));
    entries.push(entry("UUID", NbtSummaryValue::IntArray(vec![1, 2, 3, 4])));
    replace(
        &mut entries,
        "data",
        NbtSummaryValue::Compound(vec![entry("answer", NbtSummaryValue::Int(99))]),
    );

    store
        .load_command_data(id, &entries)
        .expect("valid mutable data loads");
    let after = store.get(id).expect("entity remains live");
    assert_eq!(after.id(), before.id());
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.entity_type_id(), before.entity_type_id());
    assert_eq!(store.entity_id_for_uuid(before.uuid()), Some(id));
    let EntityData::Marker(marker) = after.data() else {
        panic!("entity type and data remain a marker")
    };
    assert_eq!(marker.data(), &[entry("answer", NbtSummaryValue::Int(99))]);
}

#[test]
fn command_data_load_preserves_vehicle_and_passenger_indices() {
    let mut store = EntityStore::new(19, EntityIdCounter::new());
    let position = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    let vehicle_id = crate::entity::spawn_chest_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        position,
        0.0,
    )
    .expect("chest boat is stored");
    let passenger_position = Vec3d {
        x: 99.5,
        y: 70.0,
        z: -99.5,
    };
    let passenger_id =
        crate::entity::spawn_armor_stand(&mut store, passenger_position).expect("rider is stored");
    let vehicle_uuid = store.get(vehicle_id).expect("vehicle exists").uuid();
    let passenger_uuid = store.get(passenger_id).expect("passenger exists").uuid();
    assert!(store.link_existing_passenger_forced(vehicle_id, passenger_id));
    let entries = command_entries(&store, vehicle_id);
    let passengers = entries
        .iter()
        .find(|entry| entry.name == "Passengers")
        .expect("saveWithoutId includes the passenger tree");
    let NbtSummaryValue::List(passengers) = &passengers.value else {
        panic!("Passengers is a list")
    };
    let [NbtSummaryValue::Compound(passenger)] = passengers.as_slice() else {
        panic!("one compound passenger is present")
    };
    assert!(passenger.iter().any(|entry| entry.name == "id"));
    let passenger_pos_entry = passenger
        .iter()
        .find(|entry| entry.name == "Pos")
        .expect("passenger has Pos");
    assert_eq!(
        passenger_pos_entry.value,
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Double(position.x.to_bits()),
            NbtSummaryValue::Double(passenger_position.y.to_bits()),
            NbtSummaryValue::Double(position.z.to_bits()),
        ])
    );

    store
        .load_command_data(vehicle_id, &entries)
        .expect("unchanged vehicle data reloads");

    assert_eq!(store.passengers(vehicle_id), &[passenger_id]);
    assert_eq!(store.vehicle(passenger_id), Some(vehicle_id));
    assert_eq!(store.entity_id_for_uuid(vehicle_uuid), Some(vehicle_id));
    assert_eq!(store.entity_id_for_uuid(passenger_uuid), Some(passenger_id));
}

#[test]
fn no_save_entity_still_has_command_data_and_preserves_subtype_runtime() {
    let mut store = EntityStore::new(21, EntityIdCounter::new());
    let id = crate::entity::spawn_lightning_bolt(
        &mut store,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        },
        true,
        Some(Uuid::from_u128(0xfeed)),
    )
    .expect("lightning is stored");
    let before = store.get(id).expect("lightning exists").data().clone();
    let entries = command_entries(&store, id);

    store
        .load_command_data(id, &entries)
        .expect("noSave affects persistence, not Entity.saveWithoutId");

    assert_eq!(store.get(id).expect("lightning exists").data(), &before);
}

#[test]
fn base_runtime_state_survives_while_persistent_data_changes() {
    let (mut store, id) = marker_store(Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    });
    {
        let entity = store.get_mut(id).expect("entity exists");
        entity.tick_count = 91;
        entity.boarding_cooldown = 37;
        entity.horizontal_collision = true;
        entity.was_touching_water = true;
        entity.first_tick = false;
        entity.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID);
    }
    let mut entries = command_entries(&store, id);
    replace(
        &mut entries,
        "data",
        NbtSummaryValue::Compound(vec![entry("answer", NbtSummaryValue::Int(7))]),
    );

    store
        .load_command_data(id, &entries)
        .expect("persistent marker data loads");

    let entity = store.get(id).expect("entity exists");
    assert_eq!(entity.tick_count(), 91);
    assert_eq!(entity.boarding_cooldown(), 37);
    assert!(entity.horizontal_collision());
    assert!(entity.was_touching_water());
    assert!(!entity.first_tick);
    assert_eq!(
        entity.pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID
    );
    let EntityData::Marker(marker) = entity.data() else {
        panic!("entity remains a marker")
    };
    assert_eq!(marker.data(), &[entry("answer", NbtSummaryValue::Int(7))]);
}

#[test]
fn missing_or_unreadable_air_uses_the_mobs_maximum() {
    for (kind, air, expected) in [
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            None,
            4800,
        ),
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            Some(NbtSummaryValue::string("bad")),
            6000,
        ),
    ] {
        let mut store = EntityStore::new(27, EntityIdCounter::new());
        let id = store.spawn_general_living(kind, Vec3d::default()).unwrap();
        let mut entries = command_entries(&store, id);
        entries.retain(|entry| entry.name != "Air");
        if let Some(air) = air {
            entries.push(entry("Air", air));
        }
        store.load_command_data(id, &entries).unwrap();
        assert_eq!(store.get(id).unwrap().air_supply(), expected);
    }
}

#[test]
fn hanging_command_load_uses_position_and_keeps_an_unreadable_painting_variant() {
    use crate::entity::hanging::{spawn_item_frame, spawn_painting, PaintingVariant};
    use bbb_protocol::packets::{BlockPos, Direction};

    let mut store = EntityStore::new(28, EntityIdCounter::new());
    let attached = BlockPos { x: 1, y: 64, z: 1 };
    let variant = PaintingVariant::vanilla("minecraft:pool");
    let painting = spawn_painting(&mut store, attached, Direction::North, variant.clone()).unwrap();
    let frame = spawn_item_frame(&mut store, attached, Direction::North, false).unwrap();
    for id in [painting, frame] {
        let mut entries = command_entries(&store, id);
        entries.retain(|entry| {
            !entry
                .name
                .with_utf8(|name| matches!(name, "block_pos" | "variant" | "facing" | "Facing"))
                .unwrap_or(false)
        });
        replace(
            &mut entries,
            "Pos",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Int(5),
                NbtSummaryValue::Int(70),
                NbtSummaryValue::Int(6),
            ]),
        );
        store.load_command_data(id, &entries).unwrap();
        let entity = store.get(id).unwrap();
        let pose = match entity.data() {
            EntityData::Painting(data) => {
                assert_eq!(data.variant(), &variant);
                assert_eq!(data.pose().direction(), Direction::South);
                data.pose()
            }
            EntityData::ItemFrame(data) => {
                assert_eq!(data.pose().direction(), Direction::Down);
                data.pose()
            }
            _ => unreachable!(),
        };
        assert_eq!(pose.attached(), BlockPos { x: 5, y: 70, z: 6 });
    }
    let mut entries = command_entries(&store, painting);
    replace(
        &mut entries,
        "variant",
        NbtSummaryValue::string("minecraft:unknown"),
    );
    store.load_command_data(painting, &entries).unwrap();
    let EntityData::Painting(data) = store.get(painting).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(data.variant(), &variant);
}

#[test]
fn persistent_synched_change_marks_only_the_changed_row_dirty() {
    let position = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    };
    let mut store = EntityStore::new(29, EntityIdCounter::new());
    let id = crate::entity::spawn_end_crystal(&mut store, position, true).expect("crystal stored");
    store
        .get_mut(id)
        .expect("crystal exists")
        .take_synched_dirty();
    let mut entries = command_entries(&store, id);
    replace(&mut entries, "ShowBottom", NbtSummaryValue::Byte(0));

    store
        .load_command_data(id, &entries)
        .expect("synched persistent field loads");

    assert_eq!(
        store
            .get_mut(id)
            .expect("crystal exists")
            .take_synched_dirty(),
        vec![bbb_protocol::entity_synched_data::VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID]
    );
    store
        .load_command_data(id, &entries)
        .expect("same value reloads");
    assert!(store
        .get_mut(id)
        .expect("crystal exists")
        .take_synched_dirty()
        .is_empty());
}

#[test]
fn position_load_refiles_and_publishes_one_move() {
    let (mut store, id) = marker_store(Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    });
    store.drain_lifecycle_events();
    let old_section = section_of_block(store.get(id).expect("entity exists").block_position());
    let mut entries = command_entries(&store, id);
    replace(
        &mut entries,
        "Pos",
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Double(33.5_f64.to_bits()),
            NbtSummaryValue::Double(80.0_f64.to_bits()),
            NbtSummaryValue::Double((-34.5_f64).to_bits()),
        ]),
    );

    store
        .load_command_data(id, &entries)
        .expect("valid position loads");
    let new_section = section_of_block(store.get(id).expect("entity exists").block_position());
    let entity = store.get(id).expect("entity exists");
    assert_eq!(entity.old_position(), entity.position());
    assert!(entity.needs_sync());
    assert_eq!(entity.bounding_box().center().x, 33.5);
    assert_eq!(entity.bounding_box().center().z, -34.5);
    assert_ne!(old_section, new_section);
    assert!(store.ids_in_section(old_section).is_empty());
    assert_eq!(store.ids_in_section(new_section), vec![id]);
    assert_eq!(
        store.drain_lifecycle_events(),
        vec![EntityLifecycleEvent::Moved {
            entity_id: id,
            from: old_section,
            to: new_section,
        }]
    );
}

/// `/data merge entity` reaches `Mob.readAdditionalSaveData`'s
/// `DeathLootTable`/`DeathLootTableSeed` (`Mob.java:401-402`), and the writer
/// puts both back (`:378-380`) -- the seed only when it is not zero (`:379`).
#[test]
fn a_merged_death_loot_table_and_seed_survive_the_record() {
    let mut store = EntityStore::new(31, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .expect("the cow is stored");
    let written = command_entries(&store, id);
    assert!(
        written
            .iter()
            .all(|entry| entry.name != "DeathLootTable" && entry.name != "DeathLootTableSeed"),
        "an unset override writes neither field"
    );

    let mut entries = written;
    entries.push(entry(
        "DeathLootTable",
        NbtSummaryValue::string("minecraft:entities/zombie"),
    ));
    entries.push(entry("DeathLootTableSeed", NbtSummaryValue::Long(-4242)));
    store
        .load_command_data(id, &entries)
        .expect("the merged record loads");

    let EntityData::GeneralLiving(data) = store.get(id).expect("the cow is live").data() else {
        panic!("a cow is a general living entity")
    };
    let mob = data.mob_retained().expect("a cow is a Mob");
    assert_eq!(mob.death_loot_table(), Some("minecraft:entities/zombie"));
    assert_eq!(mob.death_loot_table_seed(), -4242);

    let written = command_entries(&store, id);
    assert_eq!(
        written
            .iter()
            .find(|entry| entry.name == "DeathLootTable")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::string("minecraft:entities/zombie"))
    );
    assert_eq!(
        written
            .iter()
            .find(|entry| entry.name == "DeathLootTableSeed")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Long(-4242))
    );
}

/// `TagValueInput.getLongOr` converts every `NumericTag` via `longValue()`
/// (`TagValueInput.java:94-100`); `Mob.readAdditionalSaveData` uses it for
/// this seed (`Mob.java:402`).
#[test]
fn death_loot_seed_accepts_narrower_numeric_nbt() {
    for (value, expected) in [
        (NbtSummaryValue::Byte(-3), -3),
        (NbtSummaryValue::Int(1), 1),
    ] {
        let mut store = EntityStore::new(33, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            )
            .unwrap();
        let mut entries = command_entries(&store, id);
        entries.push(entry("DeathLootTableSeed", value.clone()));
        store
            .load_command_data(id, &entries)
            .unwrap_or_else(|error| panic!("{value:?} must load: {error:?}"));
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!()
        };
        assert_eq!(
            data.mob_retained().unwrap().death_loot_table_seed(),
            expected,
            "{value:?}"
        );
    }
}

/// `/data merge entity` swaps a string `CustomName` for a styled one: the
/// component codec reads either shape.
#[test]
fn a_merged_custom_name_may_change_shape() {
    let mut store = EntityStore::new(32, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .expect("the cow is stored");
    let mut entries = command_entries(&store, id);
    entries.push(entry("CustomName", NbtSummaryValue::String("Old".into())));
    store
        .load_command_data(id, &entries)
        .expect("the name loads");

    let mut entries = command_entries(&store, id);
    let styled = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::String("New".into())),
        entry("color", NbtSummaryValue::String("red".into())),
    ]);
    replace(&mut entries, "CustomName", styled);
    store
        .load_command_data(id, &entries)
        .expect("the styled name loads");

    let name = store
        .get(id)
        .and_then(crate::entity::Entity::custom_name)
        .map(|name| format!("{:?}", name.component()))
        .expect("the cow keeps a name");
    assert!(name.contains("New") && name.contains("red"), "{name}");
}
