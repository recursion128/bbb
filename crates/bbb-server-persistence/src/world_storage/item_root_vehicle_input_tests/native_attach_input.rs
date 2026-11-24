use super::*;

fn tree() -> Vec<NbtSummaryEntry> {
    let mut root = entity_nbt("minecraft:oak_boat", Uuid::from_u128(1), 1.0);
    root.push(nbt_entry(
        "Passengers",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entity_nbt(
            "minecraft:oak_boat",
            Uuid::from_u128(2),
            1.0,
        ))]),
    ));
    root
}

fn disk_handoff(name: &str, cases: &[Option<NbtSummaryValue>]) {
    let uuid = Uuid::from_u128(7);
    let entity = tree();
    let (_cleanup, storage) = storage(name);
    for attach in cases {
        let mut raw = player_document(uuid, entity.clone());
        let NbtSummaryValue::Compound(wrapper) = &mut raw
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.name == "RootVehicle")
            .unwrap()
            .value
        else {
            unreachable!()
        };
        wrapper.retain(|entry| entry.name != "Attach");
        if let Some(value) = attach {
            wrapper.push(nbt_entry("Attach", value.clone()));
        }
        let bytes = write_raw_player(&storage, uuid, &raw);
        let candidate = storage.load_candidate().unwrap();
        assert!(candidate.players.is_empty());
        assert_eq!(candidate.stored_players.len(), 1);
        let (_, saved) = candidate.load_player(uuid).unwrap().unwrap();
        assert!(!saved.has_retained_raw());
        let vehicle = saved.root_vehicle().unwrap();
        assert_eq!(vehicle.attach_input(), attach.as_ref());
        assert_eq!(vehicle.entity_entries(), entity);
        assert_eq!(vehicle.entity().uuid(), Uuid::from_u128(1));
        assert_eq!(
            vehicle.entity().passengers().next().unwrap().uuid(),
            Uuid::from_u128(2)
        );
        let path = storage
            .paths()
            .player_data_dir()
            .join(format!("{}.dat", uuid.hyphenated()));
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn native_attach_input_disk_missing_and_wrong_scalar_preserve_tree_and_file() {
    disk_handoff(
        "attach-missing-wrong",
        &[
            None,
            Some(NbtSummaryValue::String("bad UUID".into())),
            Some(NbtSummaryValue::Long(42)),
        ],
    );
}

#[test]
fn native_attach_input_disk_partial_overlong_and_numeric_list_are_not_projected() {
    disk_handoff(
        "attach-numeric-shapes",
        &[
            Some(NbtSummaryValue::IntArray(vec![0, 0, 1])),
            Some(NbtSummaryValue::IntArray(vec![0, 0, 0, 1, 42])),
            Some(NbtSummaryValue::List(vec![
                NbtSummaryValue::Int(0),
                NbtSummaryValue::Int(0),
                NbtSummaryValue::Int(0),
                NbtSummaryValue::Int(2),
            ])),
        ],
    );
}

#[test]
fn native_attach_input_disk_outside_tree_and_valid_passenger_keep_original_uuid() {
    disk_handoff(
        "attach-tree-membership",
        &[
            Some(uuid_nbt(Uuid::from_u128(99))),
            Some(uuid_nbt(Uuid::from_u128(2))),
        ],
    );
}
