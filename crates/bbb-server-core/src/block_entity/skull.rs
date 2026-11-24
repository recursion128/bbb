//! `SkullBlockEntity` (`SkullBlockEntity.java:21-104`).
//!
//! Profile and custom name round-trip as opaque extra NBT. Wither spawn stays
//! outside this slice.

use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::{entry, unique_field};

const TAG_PROFILE: &str = "profile";
const TAG_NOTE_BLOCK_SOUND: &str = "note_block_sound";
const TAG_CUSTOM_NAME: &str = "custom_name";

/// `SkullBlockEntity` (`:21-33`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkullBlockEntity {
    profile: Option<NbtSummaryValue>,
    note_block_sound: Option<String>,
    custom_name: Option<NbtSummaryValue>,
}

impl SkullBlockEntity {
    // SkullBlockEntity.java:82-87.
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.profile = components.read("minecraft:profile")?;
        let sound = components.read("minecraft:note_block_sound")?;
        self.note_block_sound = match sound {
            Some(NbtSummaryValue::String(sound)) => sound.try_to_string().ok(),
            _ => None,
        };
        self.custom_name = components.read("minecraft:custom_name")?;
        Ok(())
    }

    /// `SkullBlockEntity.collectImplicitComponents` (`:90-95`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set("minecraft:profile", self.profile.clone());
        let sound = self.note_block_sound.clone().map(NbtSummaryValue::string);
        components.set("minecraft:note_block_sound", sound);
        components.set("minecraft:custom_name", self.custom_name.clone());
    }

    #[must_use]
    pub fn note_block_sound(&self) -> Option<&str> {
        self.note_block_sound.as_deref()
    }

    #[cfg(test)]
    pub fn set_note_block_sound(&mut self, sound: Option<String>) {
        self.note_block_sound = sound.filter(|value| !value.is_empty());
    }

    pub fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        if let Some(profile) = &self.profile {
            entries.push(entry(TAG_PROFILE, profile.clone()));
        }
        if let Some(sound) = &self.note_block_sound {
            entries.push(entry(
                TAG_NOTE_BLOCK_SOUND,
                NbtSummaryValue::string(sound.clone()),
            ));
        }
        if let Some(name) = &self.custom_name {
            entries.push(entry(TAG_CUSTOM_NAME, name.clone()));
        }
        entries
    }

    pub fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.profile = unique_field(record, TAG_PROFILE).cloned();
        self.note_block_sound = match unique_field(record, TAG_NOTE_BLOCK_SOUND) {
            Some(NbtSummaryValue::String(value)) => value
                .try_to_string()
                .ok()
                .and_then(|raw| normalize_resource_location(raw).ok()),
            _ => None,
        };
        self.custom_name = unique_field(record, TAG_CUSTOM_NAME).cloned();
    }
}
