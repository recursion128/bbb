use bbb_protocol::packets::inventory::ResolvableProfileSummary;
use bbb_protocol::{decode_nbt_summary, normalize_resource_location, NbtSummaryValue};

use crate::Result;

use super::nbt::{decode_nbt_root, find_entry, NbtValue};

/// Decodes `SkullBlockEntity.noteBlockSound`, saved as the optional
/// `note_block_sound` resource location. The note block reads this field from
/// the skull one block above when its instrument is `custom_head`.
pub(crate) fn decode_skull_note_block_sound(raw_nbt: &[u8]) -> Result<Option<String>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let Some(value) = find_entry(&entries, "note_block_sound") else {
        return Ok(None);
    };
    let NbtValue::String(sound) = value else {
        return Ok(None);
    };
    // `ValueInput.read(..., Identifier.CODEC).orElse(null)` folds a malformed
    // optional value to null rather than rejecting the block-entity update.
    Ok(normalize_resource_location(sound.clone()).ok())
}

/// Decodes `SkullBlockEntity.profile` through `ResolvableProfile.CODEC`.
///
/// Vanilla reads this optional field with `ValueInput.read(...).orElse(null)`,
/// so a malformed field folds to no owner while a malformed NBT root remains
/// an error at the block-entity packet boundary.
pub(crate) fn decode_skull_profile(raw_nbt: &[u8]) -> Result<Option<ResolvableProfileSummary>> {
    let Some(NbtSummaryValue::Compound(entries)) = decode_nbt_summary(raw_nbt)? else {
        return Ok(None);
    };
    let Some(value) = entries
        .iter()
        .find(|entry| entry.name == "profile")
        .map(|entry| &entry.value)
    else {
        return Ok(None);
    };
    Ok(crate::resolvable_profile::decode_profile(value, "skull block entity profile").ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::inventory::{PlayerSkinPatchSummary, ResolvableProfileKindSummary};
    use uuid::Uuid;

    fn compound_with_string(key: &str, value: &str) -> Vec<u8> {
        let mut nbt = vec![10, 8];
        nbt.extend_from_slice(&(key.len() as u16).to_be_bytes());
        nbt.extend_from_slice(key.as_bytes());
        nbt.extend_from_slice(&(value.len() as u16).to_be_bytes());
        nbt.extend_from_slice(value.as_bytes());
        nbt.push(0);
        nbt
    }

    fn compound_with_stored_profile() -> Vec<u8> {
        let mut nbt = vec![10, 10];
        nbt.extend_from_slice(&7u16.to_be_bytes());
        nbt.extend_from_slice(b"profile");
        nbt.push(11);
        nbt.extend_from_slice(&2u16.to_be_bytes());
        nbt.extend_from_slice(b"id");
        nbt.extend_from_slice(&4i32.to_be_bytes());
        for value in [
            0x0011_2233,
            0x4455_6677,
            0x8899_aabbu32 as i32,
            0xccdd_eeffu32 as i32,
        ] {
            nbt.extend_from_slice(&value.to_be_bytes());
        }
        nbt.push(8);
        nbt.extend_from_slice(&4u16.to_be_bytes());
        nbt.extend_from_slice(b"name");
        nbt.extend_from_slice(&4u16.to_be_bytes());
        nbt.extend_from_slice(b"Alex");
        nbt.push(0);
        nbt.push(0);
        nbt
    }

    #[test]
    fn decodes_and_normalizes_note_block_sound() {
        let nbt = compound_with_string("note_block_sound", "custom.bell");
        assert_eq!(
            decode_skull_note_block_sound(&nbt).unwrap().as_deref(),
            Some("minecraft:custom.bell")
        );
    }

    #[test]
    fn invalid_note_block_sound_resource_location_folds_to_none() {
        let nbt = compound_with_string("note_block_sound", "Invalid Path");
        assert_eq!(decode_skull_note_block_sound(&nbt).unwrap(), None);
    }

    #[test]
    fn missing_note_block_sound_is_none() {
        assert_eq!(decode_skull_note_block_sound(&[10, 0]).unwrap(), None);
    }

    #[test]
    fn decodes_string_and_stored_skull_profiles() {
        let string_profile = decode_skull_profile(&compound_with_string("profile", "Player_1"))
            .unwrap()
            .unwrap();
        assert_eq!(
            string_profile,
            ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::Partial,
                uuid: None,
                name: Some("Player_1".to_string()),
                properties: Vec::new(),
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary::default(),
            }
        );

        let stored_profile = decode_skull_profile(&compound_with_stored_profile())
            .unwrap()
            .unwrap();
        assert_eq!(
            stored_profile.kind,
            ResolvableProfileKindSummary::GameProfile
        );
        assert_eq!(
            stored_profile.uuid,
            Some(Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff))
        );
        assert_eq!(stored_profile.name.as_deref(), Some("Alex"));
    }

    #[test]
    fn invalid_or_missing_skull_profile_folds_to_none() {
        assert_eq!(
            decode_skull_profile(&compound_with_string("profile", "bad name")).unwrap(),
            None
        );
        assert_eq!(decode_skull_profile(&[10, 0]).unwrap(), None);
    }
}
