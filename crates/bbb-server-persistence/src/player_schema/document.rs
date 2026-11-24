//! The `playerdata` document a player the save has never held is created with.

use super::field_patches::write_save_patch;
use super::{Java26PlayerSavePatch, Java26WardenSpawnTracker};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::save::{NamedCompound, SaveResult, JAVA_26_1_DATA_VERSION};

/// `Entity.saveWithoutId` for a fresh `ServerPlayer`: the live fields from
/// `patch`, plus the fields a new player carries that the session does not own.
pub fn created_player_document(
    patch: &Java26PlayerSavePatch,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> SaveResult<NamedCompound> {
    let mut entries = vec![
        entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        entry("UUID", crate::schema_support::uuid_nbt(patch.uuid)),
        // Brain.Packed.CODEC has a non-optional `memories` map (Brain.java:463-468).
        entry(
            "Brain",
            NbtSummaryValue::Compound(vec![entry(
                "memories",
                NbtSummaryValue::Compound(Vec::new()),
            )]),
        ),
        entry("XpSeed", NbtSummaryValue::Int(0)),
        entry("Score", NbtSummaryValue::Int(0)),
        entry("seenCredits", NbtSummaryValue::Byte(0)),
    ];
    let tracker = patch
        .warden_spawn_tracker
        .as_ref()
        .unwrap_or(&Java26WardenSpawnTracker::FRESH);
    write_save_patch(&mut entries, patch, tracker, resolver)?;
    Ok(NamedCompound::new("", entries))
}
