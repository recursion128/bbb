use serde::{Deserialize, Serialize};

const VANILLA_26_1_JUKEBOX_SONGS: &str = include_str!("../data/jukebox_songs_26_1.txt");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JukeboxSongRegistry {
    by_protocol_id: Vec<Option<String>>,
}

impl JukeboxSongRegistry {
    pub fn vanilla_26_1() -> Self {
        Self::from_sound_event_ids(
            VANILLA_26_1_JUKEBOX_SONGS
                .lines()
                .filter(|line| !line.is_empty())
                .map(Some),
        )
    }

    pub fn from_sound_event_ids(ids: impl IntoIterator<Item = Option<impl Into<String>>>) -> Self {
        Self {
            by_protocol_id: ids.into_iter().map(|id| id.map(Into::into)).collect(),
        }
    }

    pub fn from_registry_entry_ids(ids: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self::from_sound_event_ids(
            ids.into_iter()
                .map(|id| vanilla_26_1_song_sound_event(id.as_ref()).map(str::to_string)),
        )
    }

    pub fn sound_event_id(&self, registry_id: i32) -> Option<&str> {
        let index = usize::try_from(registry_id).ok()?;
        self.by_protocol_id
            .get(index)
            .and_then(|entry| entry.as_deref())
    }

    pub fn len(&self) -> usize {
        self.by_protocol_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_protocol_id.is_empty()
    }
}

fn vanilla_26_1_song_sound_event(song_id: &str) -> Option<&'static str> {
    let sound = match song_id {
        "minecraft:13" => "minecraft:music_disc.13",
        "minecraft:cat" => "minecraft:music_disc.cat",
        "minecraft:blocks" => "minecraft:music_disc.blocks",
        "minecraft:chirp" => "minecraft:music_disc.chirp",
        "minecraft:far" => "minecraft:music_disc.far",
        "minecraft:mall" => "minecraft:music_disc.mall",
        "minecraft:mellohi" => "minecraft:music_disc.mellohi",
        "minecraft:stal" => "minecraft:music_disc.stal",
        "minecraft:strad" => "minecraft:music_disc.strad",
        "minecraft:ward" => "minecraft:music_disc.ward",
        "minecraft:11" => "minecraft:music_disc.11",
        "minecraft:wait" => "minecraft:music_disc.wait",
        "minecraft:pigstep" => "minecraft:music_disc.pigstep",
        "minecraft:otherside" => "minecraft:music_disc.otherside",
        "minecraft:5" => "minecraft:music_disc.5",
        "minecraft:relic" => "minecraft:music_disc.relic",
        "minecraft:precipice" => "minecraft:music_disc.precipice",
        "minecraft:creator" => "minecraft:music_disc.creator",
        "minecraft:creator_music_box" => "minecraft:music_disc.creator_music_box",
        "minecraft:tears" => "minecraft:music_disc.tears",
        "minecraft:lava_chicken" => "minecraft:music_disc.lava_chicken",
        _ => return None,
    };
    Some(sound)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(registry.sound_event_id(21), None);
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
    }
}
