//! `Mannequin`'s synched rows (`Mannequin.java:72-77`, `Avatar.java:48-49`),
//! projected from the save fields the general living body keeps.

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_RESOLVABLE_PROFILE_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_AVATAR_MAIN_HAND_DATA_ID, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    VANILLA_MANNEQUIN_DESCRIPTION_DATA_ID, VANILLA_MANNEQUIN_IMMOVABLE_DATA_ID,
    VANILLA_MANNEQUIN_PROFILE_DATA_ID,
};
use bbb_protocol::packets::{
    EntityDataValue, EntityDataValueKind, GameProfileProperty, PlayerSkinPatchData,
    ResolvableProfileData, ResolvableProfileKind,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_bool, optional_field, read_uuid_codec_input};
use crate::entity::{Entity, EntityData};

/// `PlayerModelPart` by codec name and mask (`PlayerModelPart.java:8-14`).
const MODEL_PARTS: [(&str, i8); 7] = [
    ("cape", 0x01),
    ("jacket", 0x02),
    ("left_sleeve", 0x04),
    ("right_sleeve", 0x08),
    ("left_pants_leg", 0x10),
    ("right_pants_leg", 0x20),
    ("hat", 0x40),
];

/// `Mannequin.ALL_LAYERS` (`Mannequin.java:35`).
const ALL_LAYERS: i8 = 0x7F;

/// `Mannequin.DEFAULT_DESCRIPTION` (`Mannequin.java:46`).
const DEFAULT_DESCRIPTION_KEY: &str = "entity.minecraft.mannequin.label";

/// `HumanoidArm.LEFT` / `RIGHT` ids (`HumanoidArm.java:13-14`).
const LEFT_ARM: i32 = 0;
const RIGHT_ARM: i32 = 1;

pub(crate) fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return Vec::new();
    };
    if data.entity_resource_id() != "minecraft:mannequin" {
        return Vec::new();
    }
    let fields = data.opaque_fields();
    let row = |data_id, serializer_id, value, is_default| SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id,
            value,
        },
        is_default,
    };
    // `readAdditionalSaveData` (`Mannequin.java:143-152`).
    let arm = match optional_field(fields, "main_hand") {
        Some(NbtSummaryValue::String(arm)) if arm == "left" => LEFT_ARM,
        _ => RIGHT_ARM,
    };
    let layers = model_customisation(optional_field(fields, "hidden_layers"));
    let profile = profile(optional_field(fields, "profile"));
    let profile_is_default = is_empty_profile(&profile);
    let immovable = optional_bool(fields, "immovable").unwrap_or(false);
    let (description, description_is_default) = description(fields);
    vec![
        row(
            VANILLA_AVATAR_MAIN_HAND_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
            EntityDataValueKind::HumanoidArm(arm),
            arm == RIGHT_ARM,
        ),
        row(
            VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(layers),
            layers == 0,
        ),
        row(
            VANILLA_MANNEQUIN_PROFILE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_RESOLVABLE_PROFILE_ID,
            EntityDataValueKind::ResolvableProfile(profile),
            profile_is_default,
        ),
        row(
            VANILLA_MANNEQUIN_IMMOVABLE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            EntityDataValueKind::Boolean(immovable),
            !immovable,
        ),
        row(
            VANILLA_MANNEQUIN_DESCRIPTION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            EntityDataValueKind::OptionalComponent(description),
            description_is_default,
        ),
    ]
}

/// `Mannequin.LAYERS_CODEC` (`Mannequin.java:39-44`): every part shown but
/// the listed ones.
fn model_customisation(hidden: Option<&NbtSummaryValue>) -> i8 {
    let Some(NbtSummaryValue::List(hidden)) = hidden else {
        return ALL_LAYERS;
    };
    hidden.iter().fold(ALL_LAYERS, |mask, part| {
        let bit = match part {
            NbtSummaryValue::String(name) => MODEL_PARTS
                .iter()
                .find(|(part, _)| name == *part)
                .map_or(0, |(_, bit)| *bit),
            _ => 0,
        };
        mask & !bit
    })
}

/// `ResolvableProfile.CODEC` (`ResolvableProfile.java:28-35`) and
/// `ExtraCodecs.PROPERTY_MAP` (`ExtraCodecs.java:194-211`).
fn profile(value: Option<&NbtSummaryValue>) -> ResolvableProfileData {
    let empty_patch = PlayerSkinPatchData {
        body: None,
        cape: None,
        elytra: None,
        slim_model: None,
    };
    let fields = match value {
        Some(NbtSummaryValue::String(name)) => {
            return ResolvableProfileData {
                profile: ResolvableProfileKind::Partial {
                    name: name
                        .try_to_string()
                        .ok()
                        .filter(|name| bbb_protocol::player_names::valid_player_name(name)),
                    id: None,
                    properties: Vec::new(),
                },
                skin_patch: empty_patch,
            }
        }
        Some(NbtSummaryValue::Compound(fields)) => fields.as_slice(),
        _ => &[],
    };
    let string = |name: &str| match optional_field(fields, name) {
        Some(NbtSummaryValue::String(value)) => Some(value),
        _ => None,
    };
    let id = optional_field(fields, "id").and_then(read_uuid_codec_input);
    let properties = optional_field(fields, "properties")
        .and_then(parse_profile_properties)
        .unwrap_or_default();
    let profile = match (
        id,
        string("name")
            .and_then(|name| name.try_to_string().ok())
            .filter(|name| bbb_protocol::player_names::valid_player_name(name)),
    ) {
        (Some(id), Some(name)) => ResolvableProfileKind::GameProfile {
            id,
            name,
            properties,
        },
        (id, name) => ResolvableProfileKind::Partial {
            name,
            id,
            properties,
        },
    };
    // `PlayerSkin.Patch.MAP_CODEC` (`PlayerSkin.java:40-48`): validate resource locations.
    let asset_id = |name: &str| {
        string(name)
            .and_then(|value| value.try_to_string().ok())
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
    };
    let skin_patch = PlayerSkinPatchData {
        body: asset_id("texture"),
        cape: asset_id("cape"),
        elytra: asset_id("elytra"),
        slim_model: string("model").map(|model| model == "slim"),
    };
    ResolvableProfileData {
        profile,
        skin_patch,
    }
}

fn parse_profile_properties(value: &NbtSummaryValue) -> Option<Vec<GameProfileProperty>> {
    let bounded = |value: &NbtSummaryValue, max_utf16: usize| match value {
        NbtSummaryValue::String(text) if text.units().len() <= max_utf16 => {
            text.try_to_string().ok()
        }
        _ => None,
    };
    match value {
        NbtSummaryValue::Compound(entries) if entries.len() <= 16 => {
            let mut properties = Vec::new();
            for entry in entries {
                if entry.name.units().len() > 64 {
                    return None;
                }
                let name = entry.name.try_to_string().ok()?;
                let NbtSummaryValue::List(values) = &entry.value else {
                    return None;
                };
                for item in values {
                    if properties.len() >= 16 {
                        return None;
                    }
                    properties.push(GameProfileProperty {
                        name: name.clone(),
                        value: bounded(item, 32_767)?,
                        signature: None,
                    });
                }
            }
            Some(properties)
        }
        NbtSummaryValue::List(items) if items.len() <= 16 => {
            let mut properties: Vec<GameProfileProperty> = Vec::with_capacity(items.len());
            for item in items {
                let NbtSummaryValue::Compound(fields) = item else {
                    return None;
                };
                let name = optional_field(fields, "name").and_then(|v| bounded(v, 64))?;
                let value = optional_field(fields, "value").and_then(|v| bounded(v, 32_767))?;
                let signature = match optional_field(fields, "signature") {
                    Some(sig) => Some(bounded(sig, 1024)?),
                    None => None,
                };
                let pos = properties
                    .iter()
                    .rposition(|existing| existing.name == name)
                    .map_or(properties.len(), |index| index + 1);
                properties.insert(
                    pos,
                    GameProfileProperty {
                        name,
                        value,
                        signature,
                    },
                );
            }
            Some(properties)
        }
        _ => None,
    }
}

/// `ResolvableProfile.Static.EMPTY` (`ResolvableProfile.java:163`).
fn is_empty_profile(profile: &ResolvableProfileData) -> bool {
    matches!(
        &profile.profile,
        ResolvableProfileKind::Partial {
            name: None,
            id: None,
            properties,
        } if properties.is_empty()
    ) && profile.skin_patch
        == PlayerSkinPatchData {
            body: None,
            cape: None,
            elytra: None,
            slim_model: None,
        }
}

/// `Mannequin.updateDescription` (`Mannequin.java:110-112`), with whether
/// the value is the define-time `Optional.of(DEFAULT_DESCRIPTION)`.
fn description(fields: &[NbtSummaryEntry]) -> (Option<RichTextComponent>, bool) {
    if optional_bool(fields, "hide_description").unwrap_or(false) {
        return (None, false);
    }
    let default = || {
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "translate".into(),
            value: NbtSummaryValue::string(DEFAULT_DESCRIPTION_KEY),
        }])
    };
    let component = optional_field(fields, "description")
        .cloned()
        .unwrap_or_else(default);
    let is_default = component == default();
    let description = RichTextComponent::try_from_component_nbt(component)
        .or_else(|_| RichTextComponent::try_from_component_nbt(default()))
        .ok();
    (description, is_default)
}

#[cfg(test)]
mod tests;

/// `Mannequin.VALID_POSES` (`Mannequin.java:36`).
const VALID_POSES: [i32; 5] = [
    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID,
    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID,
    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID,
];

fn is_mannequin(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data) if data.entity_resource_id() == "minecraft:mannequin")
}

/// `Mannequin.getImmovable` / `isImmobile` (`Mannequin.java:115-122`).
pub(crate) fn is_immovable(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.entity_resource_id() == "minecraft:mannequin"
        && optional_bool(data.opaque_fields(), "immovable").unwrap_or(false)
}

/// `readAdditionalSaveData` (`Mannequin.java:148`): the saved pose, or
/// standing for anything `POSE_CODEC` refuses.
pub(crate) fn finish_load(
    entity: &mut Entity,
    level: Option<&crate::world::simulation_level::LevelCtx<'_, '_>>,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if !is_mannequin(entity) {
        return;
    }
    let pose = match optional_field(data.opaque_fields(), "pose") {
        Some(NbtSummaryValue::String(name)) => bbb_protocol::entity_poses::VANILLA_ENTITY_POSES
            .iter()
            .find(|pose| name == pose.serialized_name)
            .map(|pose| pose.id)
            .filter(|id| VALID_POSES.contains(id)),
        _ => None,
    }
    .unwrap_or(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID);
    let _ = entity.set_pose_id_with_context(pose, level, None);
}

/// `addAdditionalSaveData` (`Mannequin.java:130`) writes the live pose.
pub(crate) fn write_pose(entity: &Entity, entries: &mut [NbtSummaryEntry]) {
    if !is_mannequin(entity) || !VALID_POSES.contains(&entity.pose_id()) {
        return;
    }
    let Some(name) = bbb_protocol::entity_poses::VANILLA_ENTITY_POSES
        .iter()
        .find(|pose| pose.id == entity.pose_id())
        .map(|pose| pose.serialized_name)
    else {
        return;
    };
    if let Some(entry) = entries.iter_mut().find(|entry| entry.name == "pose") {
        entry.value = NbtSummaryValue::string(name);
    }
}
