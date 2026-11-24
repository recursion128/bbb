use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID;
use bbb_protocol::packets::inventory::{
    ProfileSkinTextureSummary, ProfileTextureSummary, ProfileTexturesSummary,
};

use crate::RegistrySet;

use super::*;

const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";

#[test]
fn decodes_string_partial_and_full_profile_records() {
    let partial = ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::Partial,
        uuid: None,
        name: Some("Player_1".to_string()),
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: PlayerSkinPatchSummary::default(),
    };
    assert_eq!(
        decode_profile(
            &NbtSummaryValue::string("Player_1"),
            "dialog item component minecraft:profile",
        )
        .unwrap(),
        partial
    );
    let value = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        "minecraft:profile",
        &NbtSummaryValue::string("Player_1"),
        Some(&RegistrySet::default()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        value,
        super::super::DialogDataComponentValueState::Profile {
            profile: partial.clone()
        }
    );
    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        &NbtSummaryValue::string("Player_1"),
        None,
    );
    assert_eq!(patch.profile, Some(partial));
    assert_eq!(
        decode(&compound(Vec::new())).unwrap(),
        ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: None,
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        }
    );

    let profile = decode(&compound(vec![
        value_entry(
            "id",
            NbtSummaryValue::IntArray(vec![
                0x0011_2233,
                0x4455_6677,
                0x8899_aabbu32 as i32,
                0xccdd_eeffu32 as i32,
            ]),
        ),
        string_entry("name", "Alex"),
        value_entry(
            "properties",
            NbtSummaryValue::List(vec![compound(vec![
                string_entry("name", "textures"),
                string_entry("value", SLIM_TEXTURES_PROPERTY),
                string_entry("signature", "signed"),
            ])]),
        ),
        string_entry("texture", "entity/player/wide/steve"),
        string_entry("cape", ":entity/player/cape"),
        string_entry("elytra", "minecraft:entity/player/elytra"),
        string_entry("model", "slim"),
    ]))
    .unwrap();
    assert_eq!(
        profile,
        ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::GameProfile,
            uuid: Some(Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff,)),
            name: Some("Alex".to_string()),
            properties: vec![GameProfilePropertySummary {
                name: "textures".into(),
                value: SLIM_TEXTURES_PROPERTY.to_string().into(),
                signature: Some("signed".into()),
            }],
            profile_textures: Some(ProfileTexturesSummary {
                skin: Some(ProfileSkinTextureSummary {
                    url: "https://textures.minecraft.net/texture/skinhash".to_string(),
                    model: PlayerModelTypeSummary::Slim,
                }),
                cape: Some(ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/capehash".to_string(),
                }),
                elytra: Some(ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/elytrahash".to_string(),
                }),
            }),
            skin_patch: PlayerSkinPatchSummary {
                body: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/wide/steve".to_string(),
                    texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
                }),
                cape: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/cape".to_string(),
                    texture_path: "minecraft:textures/entity/player/cape.png".to_string(),
                }),
                elytra: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/elytra".to_string(),
                    texture_path: "minecraft:textures/entity/player/elytra.png".to_string(),
                }),
                model: Some(PlayerModelTypeSummary::Slim),
            },
        }
    );
}

#[test]
fn decodes_property_map_alternatives_and_collection_limits() {
    let map_values = (0..16)
        .map(|index| NbtSummaryValue::string(format!("value-{index}")))
        .collect::<Vec<_>>();
    let profile = decode(&compound(vec![
        value_entry(
            "id",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Long(1),
                NbtSummaryValue::Double(2.5f64.to_bits()),
                NbtSummaryValue::Byte(3),
                NbtSummaryValue::Int(4),
            ]),
        ),
        value_entry(
            "properties",
            compound(vec![value_entry(
                "custom",
                NbtSummaryValue::List(map_values),
            )]),
        ),
    ]))
    .unwrap();
    assert_eq!(profile.kind, ResolvableProfileKindSummary::Partial);
    assert_eq!(
        profile.uuid,
        Some(Uuid::from_u128((1 << 96) | (2 << 64) | (3 << 32) | 4))
    );
    assert_eq!(profile.properties.len(), 16);
    assert!(profile
        .properties
        .iter()
        .all(|property| property.name == "custom" && property.signature.is_none()));

    let oversized_map_values = (0..17)
        .map(|index| NbtSummaryValue::string(format!("value-{index}")))
        .collect::<Vec<_>>();
    assert!(decode(&compound(vec![value_entry(
        "properties",
        compound(vec![value_entry(
            "custom",
            NbtSummaryValue::List(oversized_map_values),
        )]),
    )]))
    .is_err());

    let maximum = NbtSummaryValue::List(
        (0..16)
            .map(|index| property(&format!("property-{index}"), "value", None))
            .collect(),
    );
    let profile = decode(&compound(vec![value_entry("properties", maximum)])).unwrap();
    assert_eq!(profile.properties.len(), 16);

    let empty_array = decode(&compound(vec![value_entry(
        "properties",
        NbtSummaryValue::ByteArray(Vec::new()),
    )]))
    .unwrap();
    assert!(empty_array.properties.is_empty());
}

#[test]
fn rejects_invalid_profile_alternatives_fields_and_bounds() {
    let too_many_map_names = compound(
        (0..17)
            .map(|index| {
                value_entry(
                    &format!("property-{index}"),
                    NbtSummaryValue::List(Vec::new()),
                )
            })
            .collect(),
    );
    for value in [
        NbtSummaryValue::Int(1),
        NbtSummaryValue::string("bad name"),
        compound(vec![string_entry("name", "abcdefghijklmnopq")]),
        compound(vec![string_entry(
            "id",
            "00112233-4455-6677-8899-aabbccddeeff",
        )]),
        compound(vec![value_entry(
            "id",
            NbtSummaryValue::IntArray(vec![0, 1, 2]),
        )]),
        compound(vec![value_entry(
            "id",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Int(0),
                NbtSummaryValue::Int(1),
                NbtSummaryValue::string("two"),
                NbtSummaryValue::Int(3),
            ]),
        )]),
        compound(vec![value_entry("properties", NbtSummaryValue::Int(1))]),
        compound(vec![value_entry("properties", too_many_map_names)]),
        compound(vec![value_entry(
            "properties",
            NbtSummaryValue::List(vec![property("name", "value", None); 17]),
        )]),
        compound(vec![value_entry(
            "properties",
            compound(vec![string_entry("name", "value")]),
        )]),
        compound(vec![value_entry(
            "properties",
            NbtSummaryValue::List(vec![compound(Vec::new())]),
        )]),
        compound(vec![value_entry(
            "properties",
            NbtSummaryValue::List(vec![property(&"n".repeat(65), "value", None)]),
        )]),
        compound(vec![value_entry(
            "properties",
            NbtSummaryValue::List(vec![property("name", &"v".repeat(32_768), None)]),
        )]),
        compound(vec![value_entry(
            "properties",
            NbtSummaryValue::List(vec![property("name", "value", Some(&"s".repeat(1025)))]),
        )]),
        compound(vec![string_entry("texture", "Invalid")]),
        compound(vec![string_entry("model", "default")]),
        compound(vec![value_entry("model", NbtSummaryValue::Int(0))]),
    ] {
        assert!(decode(&value).is_err());
    }
}

fn decode(value: &NbtSummaryValue) -> Result<ResolvableProfileSummary, String> {
    decode_profile(value, "dialog item component minecraft:profile")
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::string(value))
}

fn property(name: &str, value: &str, signature: Option<&str>) -> NbtSummaryValue {
    let mut entries = vec![string_entry("name", name), string_entry("value", value)];
    if let Some(signature) = signature {
        entries.push(string_entry("signature", signature));
    }
    compound(entries)
}

#[test]
fn raw_profile_properties_reach_dialog_patch_without_replacement() {
    let raw = bbb_protocol::JavaString::from(vec![0xd800, 65, 0xdc00]);
    let source = compound(vec![value_entry(
        "properties",
        NbtSummaryValue::List(vec![compound(vec![
            value_entry("name", raw.to_nbt()),
            value_entry("value", raw.to_nbt()),
            value_entry("signature", raw.to_nbt()),
        ])]),
    )]);
    let value = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        "minecraft:profile",
        &source,
        Some(&RegistrySet::default()),
    )
    .unwrap()
    .unwrap();
    let patch =
        super::super::persistent_patch(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, &source, None);
    let prepared = super::super::decode_supported_component_value_with_patch(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        "minecraft:profile",
        &source,
        super::super::DialogDecodeContext::new(None, 0),
        Some(&patch),
    )
    .unwrap()
    .unwrap();
    assert_eq!(prepared, value);
    let profile = patch.profile.unwrap();
    let super::super::DialogDataComponentValueState::Profile {
        profile: diagnostic,
    } = value
    else {
        panic!("expected diagnostic profile");
    };
    assert_eq!(diagnostic, profile);

    assert_eq!(
        profile.properties,
        vec![GameProfilePropertySummary {
            name: raw.clone(),
            value: raw.clone(),
            signature: Some(raw.clone())
        }]
    );
    let json = serde_json::to_string(&profile).unwrap();
    assert_eq!(
        serde_json::from_str::<ResolvableProfileSummary>(&json).unwrap(),
        profile
    );
    let map_source = compound(vec![value_entry(
        "properties",
        compound(vec![value_entry(
            "textures",
            NbtSummaryValue::List(vec![raw.to_nbt()]),
        )]),
    )]);
    assert_eq!(decode(&map_source).unwrap().properties[0].value, raw);
}
