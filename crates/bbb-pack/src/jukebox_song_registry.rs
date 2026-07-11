use serde::{Deserialize, Serialize};

mod generated_26_1;

use generated_26_1::VANILLA_26_1_JUKEBOX_SONGS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VanillaJukeboxSong {
    song_id: &'static str,
    sound_event_id: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JukeboxSongRegistry {
    by_protocol_id: Vec<Option<String>>,
    by_protocol_song_id: Vec<Option<String>>,
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
            by_protocol_id.push(id);
        }
        Self {
            by_protocol_id,
            by_protocol_song_id,
        }
    }

    pub fn from_registry_entry_ids(ids: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut by_protocol_id = Vec::new();
        let mut by_protocol_song_id = Vec::new();
        for id in ids {
            let id = id.as_ref();
            by_protocol_id.push(vanilla_26_1_song_sound_event(id).map(str::to_string));
            by_protocol_song_id.push(Some(id.to_string()));
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
            .and_then(|entry| entry.as_deref())
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
        let index = usize::try_from(registry_id).ok()?;
        VANILLA_26_1_JUKEBOX_SONGS
            .get(index)
            .map(|entry| entry.song_id)
    }
}

fn vanilla_26_1_song_id_for_sound_event(sound_event_id: &str) -> Option<&'static str> {
    VANILLA_26_1_JUKEBOX_SONGS
        .iter()
        .find(|entry| entry.sound_event_id == sound_event_id)
        .map(|entry| entry.song_id)
}

fn vanilla_26_1_song_sound_event(song_id: &str) -> Option<&'static str> {
    VANILLA_26_1_JUKEBOX_SONGS
        .iter()
        .find(|entry| entry.song_id == song_id)
        .map(|entry| entry.sound_event_id)
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
}
