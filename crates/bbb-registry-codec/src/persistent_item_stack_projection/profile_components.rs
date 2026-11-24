//! `minecraft:profile` in both directions, transcribed from
//! `ResolvableProfile.CODEC`, its `FULL_CODEC`
//! and `ExtraCodecs.PROPERTY_MAP`

use crate::registry_codec::RegistryElementValueRef;
use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID;
use bbb_protocol::nbt_summary::uuid_from_words;
use bbb_protocol::packets::{
    decode_profile_textures_from_properties, GameProfilePropertySummary, PlayerModelTypeSummary,
    PlayerSkinPatchSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
    ResourceTextureSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::error::{invalid_persistent_component, Result};

/// `PROPERTY.sizeLimitedListOf(16)` (`ExtraCodecs.java:197`).
const PERSISTENT_PROFILE_PROPERTIES: usize = 16;

/// `ResolvableProfile.CODEC` is
/// `Codec.withAlternative(FULL_CODEC, ExtraCodecs.PLAYER_NAME, ...)`
/// and `withAlternative` only overrides
/// decoding, so every encoding goes through `FULL_CODEC`
/// the record of
/// `Codec.mapEither(ExtraCodecs.STORED_GAME_PROFILE, Partial.MAP_CODEC)` beside
/// `PlayerSkin.Patch.MAP_CODEC`.
///
/// Which branch the either field writes is decided by `ResolvableProfile::unpack`
/// a profile the wire carried as
/// a full `GameProfile` is `Static(Either.left)` and writes
/// `STORED_GAME_PROFILE` -- required `id` and
/// `name` beside an optional `properties`; a profile the wire carried as a
/// `Partial` is always `Either.right` and writes that partial's own
/// `name`/`id`/`properties`, including the
/// `Dynamic` case where `ResolvableProfile::create` kept only the one of the
/// two it was given.
pub(super) fn persistent_profile_nbt(
    type_id: i32,
    resource_id: &'static str,
    profile: &ResolvableProfileSummary,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let mut fields = Vec::new();
    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            let uuid = profile
                .uuid
                .ok_or_else(|| invalid("a full game profile carries no id".to_string()))?;
            let name = profile
                .name
                .as_deref()
                .ok_or_else(|| invalid("a full game profile carries no name".to_string()))?;
            if !bbb_facts::player_names::valid_player_name(name) {
                return Err(invalid(format!("invalid player name {name}")));
            }
            fields.push(NbtSummaryEntry {
                name: "id".into(),
                value: persistent_uuid_nbt(uuid),
            });
            fields.push(NbtSummaryEntry {
                name: "name".into(),
                value: NbtSummaryValue::String(name.into()),
            });
        }
        ResolvableProfileKindSummary::Partial => {
            if let Some(name) = profile.name.as_deref() {
                if !bbb_facts::player_names::valid_player_name(name) {
                    return Err(invalid(format!("invalid player name {name}")));
                }
                fields.push(NbtSummaryEntry {
                    name: "name".into(),
                    value: NbtSummaryValue::String(name.into()),
                });
            }
            if let Some(uuid) = profile.uuid {
                fields.push(NbtSummaryEntry {
                    name: "id".into(),
                    value: persistent_uuid_nbt(uuid),
                });
            }
        }
    }
    if let Some(properties) =
        persistent_profile_properties(type_id, resource_id, &profile.properties)?
    {
        fields.push(properties);
    }
    // `PlayerSkin.Patch.MAP_CODEC` appends
    // `texture`/`cape`/`elytra`/`model`, each dropped while encoding when
    // absent.
    let patch = &profile.skin_patch;
    for (name, texture) in [
        ("texture", patch.body.as_ref()),
        ("cape", patch.cape.as_ref()),
        ("elytra", patch.elytra.as_ref()),
    ] {
        if let Some(texture) = texture {
            let asset_id = bbb_protocol::normalize_resource_location(texture.asset_id.as_str())
                .map_err(|error| invalid(error.to_string()))?;
            fields.push(NbtSummaryEntry {
                name: name.into(),
                value: NbtSummaryValue::string(&asset_id),
            });
        }
    }
    if let Some(model) = patch.model {
        // `PlayerModelType.CODEC` is `StringRepresentable.fromEnum`
        // over `getSerializedName`
        // i.e. `slim`/`wide`.
        fields.push(NbtSummaryEntry {
            name: "model".into(),
            value: NbtSummaryValue::String(player_model_name(model).into()),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// Use the property record list when it fits. A property map can also carry
/// unsigned values whose names, values, or total count exceed the list limits
fn persistent_profile_properties(
    type_id: i32,
    resource_id: &'static str,
    properties: &[GameProfilePropertySummary],
) -> Result<Option<NbtSummaryEntry>> {
    if properties.is_empty() {
        return Ok(None);
    }
    let fits_record_list = properties.len() <= PERSISTENT_PROFILE_PROPERTIES
        && properties.iter().all(|property| {
            property.name.units().len() <= 64
                && property.value.units().len() <= 32_767
                && property
                    .signature
                    .as_ref()
                    .is_none_or(|signature| signature.units().len() <= 1_024)
        });
    let value = if fits_record_list {
        NbtSummaryValue::List(
            properties
                .iter()
                .map(|property| {
                    let mut fields = vec![
                        NbtSummaryEntry {
                            name: "name".into(),
                            value: property.name.to_nbt(),
                        },
                        NbtSummaryEntry {
                            name: "value".into(),
                            value: property.value.to_nbt(),
                        },
                    ];
                    if let Some(signature) = &property.signature {
                        fields.push(NbtSummaryEntry {
                            name: "signature".into(),
                            value: signature.to_nbt(),
                        });
                    }
                    NbtSummaryValue::Compound(fields)
                })
                .collect(),
        )
    } else {
        if properties
            .iter()
            .any(|property| property.signature.is_some())
        {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                "signed profile properties exceed the record list limits".to_string(),
            ));
        }
        let mut fields: Vec<NbtSummaryEntry> = Vec::new();
        for property in properties {
            if let Some(field) = fields.iter_mut().find(|field| field.name == property.name) {
                let NbtSummaryValue::List(values) = &mut field.value else {
                    unreachable!("profile map values are lists")
                };
                values.push(property.value.to_nbt());
            } else {
                if fields.len() == PERSISTENT_PROFILE_PROPERTIES {
                    return Err(invalid_persistent_component(
                        type_id,
                        resource_id,
                        "profile carries too many property names".to_string(),
                    ));
                }
                fields.push(NbtSummaryEntry {
                    name: property.name.clone(),
                    value: NbtSummaryValue::List(vec![property.value.to_nbt()]),
                });
            }
        }
        NbtSummaryValue::Compound(fields)
    };
    Ok(Some(NbtSummaryEntry {
        name: "properties".into(),
        value,
    }))
}

/// `UUIDUtil.CODEC` is `Codec.INT_STREAM` over four ints
/// which `NbtOps` writes as an `IntArrayTag`.
use bbb_protocol::nbt_summary::uuid_nbt as persistent_uuid_nbt;

/// The wire projection of `ResolvableProfile.CODEC`
///
/// The either branch is the wire's own `ByteBufCodecs.either(GAME_PROFILE,
/// Partial.STREAM_CODEC)` choice, which the
/// summary keeps as [`ResolvableProfileSummary::kind`]. The save form has no
/// branch marker, so it is read back the way `EitherMapCodec` reads it: the
/// first branch that decodes wins, and `STORED_GAME_PROFILE` needs both `id`
/// and `name`, so a record carrying both is a full
/// game profile and anything else is a partial.
pub(super) fn project_profile(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<ResolvableProfileSummary> {
    let mut profile = parse_profile(type_id, resource_id, RegistryElementValueRef::Nbt(value))?;
    profile.profile_textures = decode_profile_textures_from_properties(
        profile
            .properties
            .iter()
            .map(|property| (&property.name, &property.value)),
    );
    Ok(profile)
}

pub(crate) fn canonical_profile_source_nbt(
    value: RegistryElementValueRef<'_>,
) -> Result<NbtSummaryValue> {
    let profile = parse_profile(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        "minecraft:profile",
        value,
    )?;
    persistent_profile_nbt(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        "minecraft:profile",
        &profile,
    )
}

fn parse_profile(
    type_id: i32,
    resource_id: &'static str,
    value: RegistryElementValueRef<'_>,
) -> Result<ResolvableProfileSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if let Some(name) = value.java_string_ref() {
        return Ok(ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some(project_player_name(type_id, resource_id, name)?),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        });
    }
    if !value.is_compound() {
        return Err(invalid("profile is not a compound".to_string()));
    }
    let uuid = value
        .field("id")
        .map(|value| project_uuid(type_id, resource_id, value))
        .transpose()?;
    let name = value
        .field("name")
        .map(|value| {
            let name = value
                .java_string_ref()
                .ok_or_else(|| invalid("profile name is not a string".to_string()))?;
            project_player_name(type_id, resource_id, name)
        })
        .transpose()?;
    let properties = value
        .field("properties")
        .map(|value| project_profile_properties(type_id, resource_id, value))
        .transpose()?
        .unwrap_or_default();
    let kind = if uuid.is_some() && name.is_some() {
        ResolvableProfileKindSummary::GameProfile
    } else {
        ResolvableProfileKindSummary::Partial
    };
    let skin_patch = project_player_skin_patch(type_id, resource_id, value)?;
    Ok(ResolvableProfileSummary {
        kind,
        uuid,
        name,
        properties,
        profile_textures: None,
        skin_patch,
    })
}

fn project_player_name(
    type_id: i32,
    resource_id: &'static str,
    name: &bbb_protocol::JavaString,
) -> Result<String> {
    // `ExtraCodecs.PLAYER_NAME` over `StringUtil.isValidPlayerName`
    match name.try_to_string() {
        Ok(name) if bbb_facts::player_names::valid_player_name(&name) => Ok(name),
        _ => Err(invalid_persistent_component(
            type_id,
            resource_id,
            "profile name is not a valid player name".to_string(),
        )),
    }
}

/// `ExtraCodecs.PROPERTY_MAP`'s xmap builds an `ImmutableMultimap`, so its
/// right branch encodes `propertyMap.values()`:
/// the list is grouped by first-seen key, and `[a1, b1, a2]` re-encodes as
/// `[a1, a2, b1]`. The wire codec walks the same `values()` view, so the
/// summary the wire decode produced is already in that order, but a save file
/// is free to spell it interleaved.
fn project_profile_properties(
    type_id: i32,
    resource_id: &'static str,
    value: RegistryElementValueRef<'_>,
) -> Result<Vec<GameProfilePropertySummary>> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if let Some(len) = value.map_len() {
        let fields = value.visible_map_entries(len);
        if fields.len() > PERSISTENT_PROFILE_PROPERTIES {
            return Err(invalid(
                "profile carries too many property names".to_string(),
            ));
        }
        let mut properties = Vec::new();
        for (name, values) in fields {
            if name.units().len() > 64 {
                return Err(invalid(format!(
                    "profile property name exceeds 64 UTF-16 code units ({})",
                    name.units().len()
                )));
            }
            let len = values
                .list_len()
                .ok_or_else(|| invalid("profile property values are not a list".to_string()))?;
            if properties.len().saturating_add(len) > PERSISTENT_PROFILE_PROPERTIES {
                return Err(invalid(format!(
                    "profile carries {} properties, at most {PERSISTENT_PROFILE_PROPERTIES}",
                    properties.len().saturating_add(len)
                )));
            }
            for index in 0..len {
                let prop_value = values
                    .list_element(index)
                    .unwrap()
                    .java_string()
                    .ok_or_else(|| invalid("profile property is not a string".to_string()))?;
                if prop_value.units().len() > 32_767 {
                    return Err(invalid(format!(
                        "profile property value exceeds 32767 UTF-16 code units ({})",
                        prop_value.units().len()
                    )));
                }
                properties.push(GameProfilePropertySummary {
                    name: name.clone(),
                    value: prop_value,
                    signature: None,
                });
            }
        }
        return Ok(properties);
    }
    let len = value
        .list_len()
        .ok_or_else(|| invalid("profile properties are not a list".to_string()))?;
    if len > PERSISTENT_PROFILE_PROPERTIES {
        return Err(invalid(format!(
            "profile carries {len} properties, at most {PERSISTENT_PROFILE_PROPERTIES}"
        )));
    }
    let mut properties: Vec<GameProfilePropertySummary> = Vec::with_capacity(len);
    for index in 0..len {
        let entry = value.list_element(index).unwrap();
        if !entry.is_compound() {
            return Err(invalid("profile property is not a compound".to_string()));
        }
        let property = GameProfilePropertySummary {
            name: java_string_field(type_id, resource_id, entry, "name", 64)?,
            value: java_string_field(type_id, resource_id, entry, "value", 32_767)?,
            signature: entry
                .field("signature")
                .map(|value| bounded_java_string(type_id, resource_id, value, "signature", 1_024))
                .transpose()?,
        };
        let position = properties
            .iter()
            .rposition(|existing| existing.name == property.name)
            .map_or(properties.len(), |index| index + 1);
        properties.insert(position, property);
    }
    Ok(properties)
}

fn java_string_field(
    type_id: i32,
    resource_id: &'static str,
    input: RegistryElementValueRef<'_>,
    name: &str,
    max_utf16: usize,
) -> Result<bbb_protocol::JavaString> {
    let value = input.field(name).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            format!("profile property carries no {name}"),
        )
    })?;
    bounded_java_string(type_id, resource_id, value, name, max_utf16)
}

fn bounded_java_string(
    type_id: i32,
    resource_id: &'static str,
    value: RegistryElementValueRef<'_>,
    name: &str,
    max_utf16: usize,
) -> Result<bbb_protocol::JavaString> {
    value
        .java_string()
        .filter(|value| value.units().len() <= max_utf16)
        .ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("profile property {name} is not a string within {max_utf16} UTF-16 units"),
            )
        })
}

fn project_uuid(
    type_id: i32,
    resource_id: &'static str,
    value: RegistryElementValueRef<'_>,
) -> Result<uuid::Uuid> {
    let invalid = |detail| invalid_persistent_component(type_id, resource_id, detail);
    let len = value
        .list_len()
        .ok_or_else(|| invalid("profile id is not an int stream".to_string()))?;
    if len != 4 {
        return Err(invalid(format!(
            "profile id carries {} ints, expected 4",
            len
        )));
    }
    let mut ints = [0_i32; 4];
    for (index, target) in ints.iter_mut().enumerate() {
        *target = value
            .list_element(index)
            .unwrap()
            .codec_i32()
            .ok_or_else(|| invalid("profile id element is not numeric".to_string()))?;
    }
    Ok(uuid_from_words(ints))
}

/// `PlayerSkin.Patch`, whose three textures are
/// `ClientAsset.ResourceTexture`s.
///
/// `ResourceTexture.CODEC` is `Identifier.CODEC.xmap(..., ResourceTexture::id)`
/// so the save form carries only the asset id; the
/// texture path is derived from it by the record's own constructor
fn project_player_skin_patch(
    type_id: i32,
    resource_id: &'static str,
    input: RegistryElementValueRef<'_>,
) -> Result<PlayerSkinPatchSummary> {
    let texture = |name: &str| -> Result<Option<ResourceTextureSummary>> {
        input
            .field(name)
            .map(|value| {
                let asset_id = value.java_string_ref().ok_or_else(|| {
                    invalid_persistent_component(
                        type_id,
                        resource_id,
                        format!("profile {name} is not a resource id"),
                    )
                })?;
                resource_texture_summary(asset_id)
            })
            .transpose()
    };
    let model = input
        .field("model")
        .map(|value| {
            let name = value.java_string_ref().ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    "profile model is not a string".to_string(),
                )
            })?;
            name.with_utf8(player_model_type).flatten().ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("unknown player model {name}"),
                )
            })
        })
        .transpose()?;
    Ok(PlayerSkinPatchSummary {
        body: texture("texture")?,
        cape: texture("cape")?,
        elytra: texture("elytra")?,
        model,
    })
}

/// `ClientAsset.ResourceTexture`'s derived `texturePath`
/// `texture.withPath(path -> "textures/" + path +
/// ".png")` over the `Identifier` the persistent codec normalized.
fn resource_texture_summary(
    asset_id: &bbb_protocol::JavaString,
) -> super::error::Result<ResourceTextureSummary> {
    let asset_id = super::nbt_support::normalize_resource_id_units(asset_id.units())?;
    let (namespace, path) = asset_id
        .split_once(':')
        .expect("a normalized resource location carries a namespace");
    Ok(ResourceTextureSummary {
        texture_path: format!("{namespace}:textures/{path}.png"),
        asset_id,
    })
}

/// `PlayerModelType.getSerializedName`.
fn player_model_name(model: PlayerModelTypeSummary) -> &'static str {
    match model {
        PlayerModelTypeSummary::Slim => "slim",
        PlayerModelTypeSummary::Wide => "wide",
    }
}

fn player_model_type(name: &str) -> Option<PlayerModelTypeSummary> {
    match name {
        "slim" => Some(PlayerModelTypeSummary::Slim),
        "wide" => Some(PlayerModelTypeSummary::Wide),
        _ => None,
    }
}
