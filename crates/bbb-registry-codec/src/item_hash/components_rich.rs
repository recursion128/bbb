//! HashOps encoding for rich `Component` trees and `ResolvableProfile`.

use bbb_facts::data_component_types::{
    vanilla_data_component_type, VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
};
use bbb_facts::player_names::valid_player_name;
use bbb_protocol::component::{
    canonical_component_codec_value_with_registry, CanonicalComponentCodecValue,
    CanonicalComponentOwnerValue,
};
use bbb_protocol::packets::{
    PlayerModelTypeSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
};
use bbb_protocol::NbtSummaryValue;

use super::components::hash_component_value_at_depth;
use super::ops::{
    hash_boolean, hash_empty_map, hash_int, hash_int_array, hash_java_string, hash_list, hash_map,
    hash_nbt, hash_record, hash_string, normalize_codec_identifier, normalize_java_identifier,
};
use super::ItemHashLookup;

const MAX_COMPONENT_HASH_DEPTH: usize = 128;

pub fn hash_component(lookup: &dyn ItemHashLookup, value: &NbtSummaryValue) -> Option<u32> {
    hash_component_at_depth(lookup, value, 0)
}

pub(super) fn hash_component_at_depth(
    lookup: &dyn ItemHashLookup,
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<u32> {
    if depth >= MAX_COMPONENT_HASH_DEPTH {
        return None;
    }
    let canonical =
        canonical_component_codec_value_with_registry(value, lookup.content_registry()).ok()?;
    hash_canonical_component_value(lookup, &canonical, depth)
}

fn hash_canonical_component_value(
    lookup: &dyn ItemHashLookup,
    value: &CanonicalComponentCodecValue<'_>,
    depth: usize,
) -> Option<u32> {
    match value {
        CanonicalComponentCodecValue::Boolean(value) => Some(hash_boolean(*value)),
        CanonicalComponentCodecValue::Int(value) => Some(hash_int(*value)),
        CanonicalComponentCodecValue::String(value) => Some(hash_java_string(value)),
        CanonicalComponentCodecValue::List(values) => {
            let mut hashes = Vec::with_capacity(values.len());
            for item in values {
                hashes.push(hash_canonical_component_value(lookup, item, depth)?);
            }
            Some(hash_list(hashes))
        }
        CanonicalComponentCodecValue::IntArray(values) => {
            Some(hash_int_array(values.iter().copied()))
        }
        CanonicalComponentCodecValue::Object(fields) => {
            let mut entries = Vec::with_capacity(fields.len());
            for (name, field_value) in fields {
                if *name == "components" {
                    if let CanonicalComponentCodecValue::Owner(
                        CanonicalComponentOwnerValue::ItemComponentPatch,
                        raw,
                    ) = field_value
                    {
                        if let Some(patch_hash) =
                            hash_hover_item_component_patch(lookup, raw, depth)?
                        {
                            entries.push((hash_string(name), patch_hash));
                        }
                        continue;
                    }
                }
                entries.push((
                    hash_string(name),
                    hash_canonical_component_value(lookup, field_value, depth)?,
                ));
            }
            Some(hash_map(entries))
        }
        CanonicalComponentCodecValue::Nbt(value) => Some(hash_nbt(value, false)),
        CanonicalComponentCodecValue::Owner(kind, value) => {
            hash_component_owner_value(lookup, *kind, value, depth)
        }
    }
}

fn hash_component_owner_value(
    lookup: &dyn ItemHashLookup,
    kind: CanonicalComponentOwnerValue,
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<u32> {
    match kind {
        CanonicalComponentOwnerValue::PlayerProfile => {
            let patch =
                crate::read_item_component(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, value, lookup)
                    .ok()?;
            hash_resolvable_profile(patch.profile.as_ref()?)
        }
        CanonicalComponentOwnerValue::Dialog => {
            let NbtSummaryValue::String(dialog_raw) = value else {
                // Inline dialog click actions stay server-authoritative.
                return None;
            };
            let dialog_id = normalize_java_identifier(dialog_raw)?;
            if !lookup.contains_dialog(&dialog_id) {
                return None;
            }
            Some(hash_string(&dialog_id))
        }
        CanonicalComponentOwnerValue::ItemComponentPatch => {
            let patch_hash = hash_hover_item_component_patch(lookup, value, depth)?;
            Some(patch_hash.unwrap_or_else(hash_empty_map))
        }
    }
}

/// Hashes a `HoverEvent.ShowItem`'s `components` patch (`ItemStackTemplate.MAP_CODEC`,
/// `HoverEvent.java:127`), returning `Ok(None)` (`Some(None)`) when the patch is empty so
/// `optionalFieldOf("components", DataComponentPatch.EMPTY)` omits the field.
fn hash_hover_item_component_patch(
    lookup: &dyn ItemHashLookup,
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<Option<u32>> {
    let patch =
        crate::read_component_patch_with_content_registry(value, lookup, lookup.content_registry())
            .ok()?;
    if patch.added_type_ids.is_empty() && patch.removed_type_ids.is_empty() {
        return Some(None);
    }
    let mut component_pairs =
        Vec::with_capacity(patch.added_type_ids.len() + patch.removed_type_ids.len());
    // `DataComponentPatch.CODEC` (`DataComponentPatch.java:46`) skips transient entries.
    for &type_id in &patch.added_type_ids {
        let comp = vanilla_data_component_type(type_id)?;
        if !comp.persistent {
            continue;
        }
        let value_hash = hash_component_value_at_depth(lookup, &patch, type_id, depth + 1)?;
        component_pairs.push((hash_string(comp.resource_id), value_hash));
    }
    for &type_id in &patch.removed_type_ids {
        let comp = vanilla_data_component_type(type_id)?;
        if !comp.persistent {
            continue;
        }
        component_pairs.push((
            hash_string(&format!("!{}", comp.resource_id)),
            hash_empty_map(),
        ));
    }
    Some(Some(hash_map(component_pairs)))
}

/// `ResolvableProfile.CODEC` (`ResolvableProfile.java:28-35`).
pub fn hash_resolvable_profile(profile: &ResolvableProfileSummary) -> Option<u32> {
    let mut pairs = Vec::new();
    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            let uuid = profile.uuid?;
            let bits = uuid.as_u128();
            pairs.push((
                hash_string("id"),
                hash_int_array([
                    (bits >> 96) as i32,
                    (bits >> 64) as i32,
                    (bits >> 32) as i32,
                    bits as i32,
                ]),
            ));
            let name = profile.name.as_deref()?;
            if !valid_player_name(name) {
                return None;
            }
            pairs.push((hash_string("name"), hash_string(name)));
        }
        ResolvableProfileKindSummary::Partial => {
            if let Some(uuid) = profile.uuid {
                let bits = uuid.as_u128();
                pairs.push((
                    hash_string("id"),
                    hash_int_array([
                        (bits >> 96) as i32,
                        (bits >> 64) as i32,
                        (bits >> 32) as i32,
                        bits as i32,
                    ]),
                ));
            }
            if let Some(name) = &profile.name {
                if !valid_player_name(name) {
                    return None;
                }
                pairs.push((hash_string("name"), hash_string(name)));
            }
        }
    }
    if !profile.properties.is_empty() {
        let mut prop_hashes = Vec::with_capacity(profile.properties.len());
        for prop in &profile.properties {
            let mut prop_fields = vec![
                ("name", hash_java_string(&prop.name)),
                ("value", hash_java_string(&prop.value)),
            ];
            if let Some(sig) = &prop.signature {
                prop_fields.push(("signature", hash_java_string(sig)));
            }
            prop_hashes.push(hash_record(prop_fields));
        }
        pairs.push((hash_string("properties"), hash_list(prop_hashes)));
    }
    if let Some(body) = &profile.skin_patch.body {
        pairs.push((
            hash_string("texture"),
            hash_string(&normalize_codec_identifier(&body.asset_id)?),
        ));
    }
    if let Some(cape) = &profile.skin_patch.cape {
        pairs.push((
            hash_string("cape"),
            hash_string(&normalize_codec_identifier(&cape.asset_id)?),
        ));
    }
    if let Some(elytra) = &profile.skin_patch.elytra {
        pairs.push((
            hash_string("elytra"),
            hash_string(&normalize_codec_identifier(&elytra.asset_id)?),
        ));
    }
    if let Some(model) = profile.skin_patch.model {
        let model_name = match model {
            PlayerModelTypeSummary::Slim => "slim",
            PlayerModelTypeSummary::Wide => "wide",
        };
        pairs.push((hash_string("model"), hash_string(model_name)));
    }
    Some(hash_map(pairs))
}
