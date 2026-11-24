use super::*;
use crate::entity::store::{EntityIdCounter, EntityStore};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID;
use bbb_protocol::packets::Vec3d;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn row(entity: &Entity, data_id: u8) -> Option<EntityDataValueKind> {
    crate::entity::tracking::non_default_synched_values(entity)
        .into_iter()
        .find(|value| value.data_id == data_id)
        .map(|value| value.value)
}

/// A fresh mannequin pairs with every layer shown and the default profile and
/// label left out; `/data` edits mark the changed rows dirty and publish the
/// profile, arm, layers, immovability and hidden description.
#[test]
fn mannequin_rows_pair_at_spawn_and_follow_data_edits() {
    let mut store = EntityStore::new(83, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, Vec3d::default())
        .unwrap();
    let entity = store.get(id).unwrap();
    assert_eq!(
        row(entity, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID),
        Some(EntityDataValueKind::Byte(ALL_LAYERS))
    );
    for absent in [
        VANILLA_AVATAR_MAIN_HAND_DATA_ID,
        VANILLA_MANNEQUIN_PROFILE_DATA_ID,
        VANILLA_MANNEQUIN_IMMOVABLE_DATA_ID,
        VANILLA_MANNEQUIN_DESCRIPTION_DATA_ID,
    ] {
        assert_eq!(row(entity, absent), None);
    }
    store.get_mut(id).unwrap().take_synched_dirty();

    let NbtSummaryValue::Compound(mut data) = store.command_data(id).unwrap() else {
        panic!("entity data is a compound");
    };
    data.retain(|field| {
        !matches!(
            field.name.to_jdk_string().as_str(),
            "profile" | "main_hand" | "hidden_layers" | "immovable"
        )
    });
    data.extend([
        entry(
            "profile",
            NbtSummaryValue::Compound(vec![
                entry("name", NbtSummaryValue::String("Alex".into())),
                entry("model", NbtSummaryValue::String("slim".into())),
            ]),
        ),
        entry("main_hand", NbtSummaryValue::String("left".into())),
        entry(
            "hidden_layers",
            NbtSummaryValue::List(vec![NbtSummaryValue::String("hat".into())]),
        ),
        entry("immovable", NbtSummaryValue::Byte(1)),
        entry("hide_description", NbtSummaryValue::Byte(1)),
    ]);
    store.load_command_data(id, &data).unwrap();
    let entity = store.get_mut(id).unwrap();
    let mut dirty = entity.take_synched_dirty();
    dirty.sort_unstable();
    for id in [
        VANILLA_AVATAR_MAIN_HAND_DATA_ID,
        VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        VANILLA_MANNEQUIN_PROFILE_DATA_ID,
        VANILLA_MANNEQUIN_IMMOVABLE_DATA_ID,
        VANILLA_MANNEQUIN_DESCRIPTION_DATA_ID,
    ] {
        assert!(dirty.contains(&id), "row {id} is dirty");
    }
    assert_eq!(
        row(entity, VANILLA_AVATAR_MAIN_HAND_DATA_ID),
        Some(EntityDataValueKind::HumanoidArm(LEFT_ARM))
    );
    assert_eq!(
        row(entity, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID),
        Some(EntityDataValueKind::Byte(ALL_LAYERS & !0x40))
    );
    assert_eq!(
        row(entity, VANILLA_MANNEQUIN_PROFILE_DATA_ID),
        Some(EntityDataValueKind::ResolvableProfile(
            ResolvableProfileData {
                profile: ResolvableProfileKind::Partial {
                    name: Some("Alex".into()),
                    id: None,
                    properties: Vec::new(),
                },
                skin_patch: PlayerSkinPatchData {
                    body: None,
                    cape: None,
                    elytra: None,
                    slim_model: Some(true),
                },
            }
        ))
    );
    assert_eq!(
        row(entity, VANILLA_MANNEQUIN_IMMOVABLE_DATA_ID),
        Some(EntityDataValueKind::Boolean(true))
    );
    assert_eq!(
        row(entity, VANILLA_MANNEQUIN_DESCRIPTION_DATA_ID),
        Some(EntityDataValueKind::OptionalComponent(None))
    );
}

#[test]
fn mannequin_profile_preserves_list_and_map_properties() {
    let mut store = EntityStore::new(83, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, Vec3d::default())
        .unwrap();
    let uuid = uuid::Uuid::from_u128(0x11223344_5566_7788_99aa_bbccddeeff00);
    let words = uuid.as_u128();
    let uuid_ints = vec![
        (words >> 96) as i32,
        (words >> 64) as i32,
        (words >> 32) as i32,
        words as i32,
    ];
    let NbtSummaryValue::Compound(mut data) = store.command_data(id).unwrap() else {
        panic!("entity data is a compound");
    };
    data.retain(|field| field.name != "profile");
    data.push(entry(
        "profile",
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::IntArray(uuid_ints)),
            entry("name", NbtSummaryValue::String("Steve".into())),
            entry(
                "properties",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry("name", NbtSummaryValue::String("textures".into())),
                    entry(
                        "value",
                        NbtSummaryValue::String("eyJ0ZXh0dXJlcyI6e319".into()),
                    ),
                    entry("signature", NbtSummaryValue::String("sig-1".into())),
                ])]),
            ),
        ]),
    ));
    store.load_command_data(id, &data).unwrap();
    let entity = store.get(id).unwrap();
    assert_eq!(
        row(entity, VANILLA_MANNEQUIN_PROFILE_DATA_ID),
        Some(EntityDataValueKind::ResolvableProfile(
            ResolvableProfileData {
                profile: ResolvableProfileKind::GameProfile {
                    id: uuid,
                    name: "Steve".into(),
                    properties: vec![GameProfileProperty {
                        name: "textures".into(),
                        value: "eyJ0ZXh0dXJlcyI6e319".into(),
                        signature: Some("sig-1".into()),
                    }],
                },
                skin_patch: PlayerSkinPatchData {
                    body: None,
                    cape: None,
                    elytra: None,
                    slim_model: None,
                },
            }
        ))
    );

    // Map form (`ExtraCodecs.PROPERTY_MAP` left branch) on an otherwise unnamed partial profile
    // is also non-default and publishes its unsigned properties.
    data.retain(|field| field.name != "profile");
    data.push(entry(
        "profile",
        NbtSummaryValue::Compound(vec![entry(
            "properties",
            NbtSummaryValue::Compound(vec![entry(
                "textures",
                NbtSummaryValue::List(vec![NbtSummaryValue::String("map-tex".into())]),
            )]),
        )]),
    ));
    store.load_command_data(id, &data).unwrap();
    let entity = store.get(id).unwrap();
    assert_eq!(
        row(entity, VANILLA_MANNEQUIN_PROFILE_DATA_ID),
        Some(EntityDataValueKind::ResolvableProfile(
            ResolvableProfileData {
                profile: ResolvableProfileKind::Partial {
                    name: None,
                    id: None,
                    properties: vec![GameProfileProperty {
                        name: "textures".into(),
                        value: "map-tex".into(),
                        signature: None,
                    }],
                },
                skin_patch: PlayerSkinPatchData {
                    body: None,
                    cape: None,
                    elytra: None,
                    slim_model: None,
                },
            }
        ))
    );
}
