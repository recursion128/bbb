use bbb_protocol::normalize_resource_location;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn compound_with_string(key: &str, value: &str) -> Vec<u8> {
        let mut nbt = vec![10, 8];
        nbt.extend_from_slice(&(key.len() as u16).to_be_bytes());
        nbt.extend_from_slice(key.as_bytes());
        nbt.extend_from_slice(&(value.len() as u16).to_be_bytes());
        nbt.extend_from_slice(value.as_bytes());
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
}
