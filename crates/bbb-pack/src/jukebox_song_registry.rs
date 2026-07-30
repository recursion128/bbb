use bbb_protocol::jukebox_song_types::{
    vanilla_26_1_jukebox_song_by_holder_id, vanilla_26_1_jukebox_song_by_song_id,
    vanilla_26_1_jukebox_song_by_sound_event_id, VANILLA_26_1_JUKEBOX_SONGS,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JukeboxSongRegistry {
    by_protocol_id: Vec<Option<JukeboxSongSoundEvent>>,
    by_protocol_song_id: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct JukeboxSongSoundEvent {
    resource_id: String,
    fixed_range_bits: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JukeboxSongRegistryEntry {
    song_id: String,
    sound_event_id: Option<String>,
    fixed_range_bits: Option<u32>,
}

impl JukeboxSongRegistryEntry {
    pub fn new(
        song_id: impl Into<String>,
        sound_event_id: Option<impl Into<String>>,
        fixed_range: Option<f32>,
    ) -> Self {
        Self {
            song_id: song_id.into(),
            sound_event_id: sound_event_id.map(Into::into),
            fixed_range_bits: fixed_range.map(f32::to_bits),
        }
    }
}

impl JukeboxSongRegistry {
    pub fn vanilla_26_1() -> Self {
        Self::from_sound_event_ids(
            VANILLA_26_1_JUKEBOX_SONGS
                .iter()
                .map(|entry| Some(entry.sound_event_id)),
        )
    }

    pub fn from_sound_event_ids(ids: impl IntoIterator<Item = Option<impl Into<String>>>) -> Self {
        let mut by_protocol_id = Vec::new();
        let mut by_protocol_song_id = Vec::new();
        for id in ids {
            let id = id.map(Into::into);
            by_protocol_song_id.push(
                id.as_deref()
                    .and_then(vanilla_26_1_song_id_for_sound_event)
                    .map(str::to_string),
            );
            by_protocol_id.push(id.map(|resource_id| JukeboxSongSoundEvent {
                resource_id,
                fixed_range_bits: None,
            }));
        }
        Self {
            by_protocol_id,
            by_protocol_song_id,
        }
    }

    pub fn from_registry_entry_ids(ids: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self::from_entries(ids.into_iter().map(|id| {
            let id = id.as_ref();
            JukeboxSongRegistryEntry::new(id, vanilla_26_1_song_sound_event(id), None)
        }))
    }

    pub fn from_entries(entries: impl IntoIterator<Item = JukeboxSongRegistryEntry>) -> Self {
        let mut by_protocol_id = Vec::new();
        let mut by_protocol_song_id = Vec::new();
        for entry in entries {
            by_protocol_id.push(
                entry
                    .sound_event_id
                    .map(|resource_id| JukeboxSongSoundEvent {
                        resource_id,
                        fixed_range_bits: entry.fixed_range_bits,
                    }),
            );
            by_protocol_song_id.push(Some(entry.song_id));
        }
        Self {
            by_protocol_id,
            by_protocol_song_id,
        }
    }

    pub fn sound_event_id(&self, registry_id: i32) -> Option<&str> {
        let index = usize::try_from(registry_id).ok()?;
        self.by_protocol_id
            .get(index)
            .and_then(Option::as_ref)
            .map(|entry| entry.resource_id.as_str())
    }

    pub fn sound_event_fixed_range(&self, registry_id: i32) -> Option<f32> {
        let index = usize::try_from(registry_id).ok()?;
        self.by_protocol_id
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|entry| entry.fixed_range_bits.map(f32::from_bits))
    }

    pub fn song_id(&self, registry_id: i32) -> Option<&str> {
        let index = usize::try_from(registry_id).ok()?;
        self.by_protocol_song_id
            .get(index)
            .and_then(|entry| entry.as_deref())
    }

    pub fn len(&self) -> usize {
        self.by_protocol_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_protocol_id.is_empty()
    }

    pub fn vanilla_26_1_song_id(registry_id: i32) -> Option<&'static str> {
        vanilla_26_1_jukebox_song_by_holder_id(registry_id).map(|entry| entry.song_id)
    }

    pub fn vanilla_26_1_sound_event_id(song_id: &str) -> Option<&'static str> {
        vanilla_26_1_song_sound_event(song_id)
    }
}

fn vanilla_26_1_song_id_for_sound_event(sound_event_id: &str) -> Option<&'static str> {
    vanilla_26_1_jukebox_song_by_sound_event_id(sound_event_id).map(|entry| entry.song_id)
}

fn vanilla_26_1_song_sound_event(song_id: &str) -> Option<&'static str> {
    vanilla_26_1_jukebox_song_by_song_id(song_id).map(|entry| entry.sound_event_id)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn generated_vanilla_catalog_is_complete_and_bidirectionally_unique() {
        assert_eq!(VANILLA_26_1_JUKEBOX_SONGS.len(), 21);
        assert_eq!(
            VANILLA_26_1_JUKEBOX_SONGS
                .iter()
                .map(|entry| entry.song_id)
                .collect::<HashSet<_>>()
                .len(),
            21
        );
        assert_eq!(
            VANILLA_26_1_JUKEBOX_SONGS
                .iter()
                .map(|entry| entry.sound_event_id)
                .collect::<HashSet<_>>()
                .len(),
            21
        );
        for (index, entry) in VANILLA_26_1_JUKEBOX_SONGS.iter().enumerate() {
            assert_eq!(
                JukeboxSongRegistry::vanilla_26_1_song_id(index as i32),
                Some(entry.song_id)
            );
            assert_eq!(
                vanilla_26_1_song_id_for_sound_event(entry.sound_event_id),
                Some(entry.song_id)
            );
            assert_eq!(
                vanilla_26_1_song_sound_event(entry.song_id),
                Some(entry.sound_event_id)
            );
        }
    }

    #[test]
    fn vanilla_26_1_registry_uses_jukebox_songs_bootstrap_order() {
        let registry = JukeboxSongRegistry::vanilla_26_1();

        assert_eq!(registry.len(), 21);
        assert_eq!(registry.sound_event_id(0), Some("minecraft:music_disc.13"));
        assert_eq!(registry.sound_event_id(1), Some("minecraft:music_disc.cat"));
        assert_eq!(registry.sound_event_id(14), Some("minecraft:music_disc.5"));
        assert_eq!(
            registry.sound_event_id(20),
            Some("minecraft:music_disc.lava_chicken")
        );
        assert_eq!(registry.song_id(0), Some("minecraft:13"));
        assert_eq!(registry.song_id(1), Some("minecraft:cat"));
        assert_eq!(registry.song_id(20), Some("minecraft:lava_chicken"));
        assert_eq!(
            JukeboxSongRegistry::vanilla_26_1_song_id(20),
            Some("minecraft:lava_chicken")
        );
        assert_eq!(registry.sound_event_id(21), None);
        assert_eq!(registry.song_id(21), None);
    }

    #[test]
    fn registry_entry_ids_map_vanilla_song_ids_to_sound_events() {
        let registry = JukeboxSongRegistry::from_registry_entry_ids([
            "minecraft:cat",
            "minecraft:tears",
            "bbb:custom_song",
        ]);

        assert_eq!(registry.len(), 3);
        assert_eq!(registry.sound_event_id(0), Some("minecraft:music_disc.cat"));
        assert_eq!(
            registry.sound_event_id(1),
            Some("minecraft:music_disc.tears")
        );
        assert_eq!(registry.sound_event_id(2), None);
        assert_eq!(registry.song_id(0), Some("minecraft:cat"));
        assert_eq!(registry.song_id(1), Some("minecraft:tears"));
        assert_eq!(registry.song_id(2), Some("bbb:custom_song"));
    }

    #[test]
    fn typed_entries_preserve_custom_sound_and_fixed_range_without_key_guessing() {
        let registry = JukeboxSongRegistry::from_entries([
            JukeboxSongRegistryEntry::new(
                "example:custom",
                Some("example:music.custom"),
                Some(24.0),
            ),
            JukeboxSongRegistryEntry::new("minecraft:cat", None::<String>, None),
        ]);

        assert_eq!(registry.song_id(0), Some("example:custom"));
        assert_eq!(registry.sound_event_id(0), Some("example:music.custom"));
        assert_eq!(registry.sound_event_fixed_range(0), Some(24.0));
        assert_eq!(registry.song_id(1), Some("minecraft:cat"));
        assert_eq!(registry.sound_event_id(1), None);
        assert_eq!(registry.sound_event_fixed_range(1), None);
    }
}
