use super::*;

const TYPES: [&str; 3] = [
    "minecraft:arrow",
    "minecraft:spectral_arrow",
    "minecraft:trident",
];

fn canonical_arrow(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = entity_nbt(id, uuid, 1.0);
    entity.extend([
        nbt_entry("HasBeenShot", NbtSummaryValue::Byte(0)),
        nbt_entry("life", NbtSummaryValue::Short(0)),
        nbt_entry("shake", NbtSummaryValue::Byte(0)),
        nbt_entry("inGround", NbtSummaryValue::Byte(0)),
        nbt_entry("pickup", NbtSummaryValue::Byte(0)),
        nbt_entry("damage", NbtSummaryValue::Double(2.0_f64.to_bits())),
        nbt_entry("crit", NbtSummaryValue::Byte(0)),
        nbt_entry("PierceLevel", NbtSummaryValue::Byte(0)),
        nbt_entry(
            "SoundEvent",
            NbtSummaryValue::String("minecraft:entity.arrow.hit".into()),
        ),
        nbt_entry(
            "item",
            NbtSummaryValue::Compound(vec![
                nbt_entry("id", NbtSummaryValue::String(id.into())),
                nbt_entry("count", NbtSummaryValue::Int(1)),
            ]),
        ),
    ]);
    match id {
        "minecraft:spectral_arrow" => entity.push(nbt_entry("Duration", NbtSummaryValue::Int(200))),
        "minecraft:trident" => entity.push(nbt_entry("DealtDamage", NbtSummaryValue::Byte(0))),
        _ => {}
    }
    entity
}

fn with_passenger(mut root: Vec<NbtSummaryEntry>, id: &str) -> Vec<NbtSummaryEntry> {
    root.push(nbt_entry(
        "Passengers",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(canonical_arrow(
            id,
            Uuid::from_u128(2),
        ))]),
    ));
    root
}

fn disk_handoff(name: &str, root: Vec<NbtSummaryEntry>) {
    let uuid = Uuid::from_u128(7);
    let raw = player_document(uuid, root.clone());
    let parsed = Java26PlayerData::parse(raw.clone(), uuid).unwrap();
    assert_eq!(parsed.raw(), &raw);
    let (_cleanup, storage) = storage(name);
    let bytes = write_raw_player(&storage, uuid, &raw);
    let candidate = storage.load_candidate().unwrap();
    assert!(candidate.players.is_empty());
    assert_eq!(candidate.stored_players.len(), 1);
    let (_, saved) = candidate.load_player(uuid).unwrap().unwrap();
    assert!(!saved.has_retained_raw());
    let vehicle = saved.root_vehicle().unwrap();
    assert_eq!(vehicle.entity_entries(), root);
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

#[test]
fn native_arrow_input_disk_missing_owned_fields_are_handed_off_for_all_three_subtypes() {
    for (index, id) in TYPES.into_iter().enumerate() {
        disk_handoff(
            &format!("arrow-missing-{index}"),
            with_passenger(entity_nbt(id, Uuid::from_u128(1), 1.0), id),
        );
    }
}

#[test]
fn native_arrow_input_disk_wrong_fields_and_partial_stacks_keep_original_payload() {
    for (index, id) in TYPES.into_iter().enumerate() {
        let mut root = canonical_arrow(id, Uuid::from_u128(1));
        root.retain(|entry| {
            !matches!(
                entry.name.to_jdk_string().as_str(),
                "item" | "weapon" | "Owner" | "life" | "Duration" | "DealtDamage"
            )
        });
        root.extend([
            nbt_entry("Owner", NbtSummaryValue::IntArray(vec![1, 2, 3])),
            nbt_entry("life", NbtSummaryValue::String("bad scalar".into())),
            nbt_entry("weapon", NbtSummaryValue::String("bad stack".into())),
            nbt_entry(
                "item",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("id", NbtSummaryValue::String(id.into())),
                    nbt_entry(
                        "components",
                        NbtSummaryValue::Compound(vec![nbt_entry(
                            "minecraft:damage",
                            NbtSummaryValue::String("usable partial".into()),
                        )]),
                    ),
                ]),
            ),
        ]);
        if id == "minecraft:spectral_arrow" {
            root.push(nbt_entry(
                "Duration",
                NbtSummaryValue::Float(2.5_f32.to_bits()),
            ));
        } else if id == "minecraft:trident" {
            root.push(nbt_entry(
                "DealtDamage",
                NbtSummaryValue::String("bad boolean".into()),
            ));
        }
        disk_handoff(&format!("arrow-partial-{index}"), with_passenger(root, id));
    }
}

#[test]
fn native_arrow_input_disk_pickup_three_stays_raw_in_root_and_passenger() {
    for (index, id) in TYPES.into_iter().enumerate() {
        let mut root = canonical_arrow(id, Uuid::from_u128(1));
        root.iter_mut()
            .find(|entry| entry.name == "pickup")
            .unwrap()
            .value = NbtSummaryValue::Byte(3);
        let mut child = canonical_arrow(id, Uuid::from_u128(2));
        child
            .iter_mut()
            .find(|entry| entry.name == "pickup")
            .unwrap()
            .value = NbtSummaryValue::Byte(3);
        root.push(nbt_entry(
            "Passengers",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(child)]),
        ));
        disk_handoff(&format!("arrow-pickup-three-{index}"), root);
    }
}

#[test]
fn native_arrow_input_disk_canonical_control_and_mistyped_common_field_keeps_the_vehicle() {
    for (index, id) in TYPES.into_iter().enumerate() {
        let root = with_passenger(canonical_arrow(id, Uuid::from_u128(1)), id);
        disk_handoff(&format!("arrow-canonical-{index}"), root.clone());
        let mut bad = root;
        bad.iter_mut()
            .find(|entry| entry.name == "Air")
            .unwrap()
            .value = NbtSummaryValue::String("bad common field".into());
        let uuid = Uuid::from_u128(7);
        let parsed = Java26PlayerData::parse(player_document(uuid, bad), uuid).unwrap();
        assert!(parsed.root_vehicle().is_some());
    }
}
