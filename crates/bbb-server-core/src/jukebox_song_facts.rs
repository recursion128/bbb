//! Playback rules over the current published `JukeboxSong` value.

use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::{Result, ServerWorldError};

pub const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";

/// `JukeboxSong.SONG_END_PADDING_TICKS` (`JukeboxSong.java:44`).
const SONG_END_PADDING_TICKS: i64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JukeboxSongFacts {
    pub comparator_output: i32,
    pub length_in_ticks: i32,
}

impl JukeboxSongFacts {
    pub fn resolve(
        song_id: &str,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<Self> {
        let value = resolver
            .holder_id(JUKEBOX_SONG_REGISTRY, song_id)
            .and_then(|id| resolver.jukebox_song(id))
            .ok_or(ServerWorldError::MissingCatalogFact("jukebox song"))?;
        Ok(Self {
            comparator_output: value.comparator_output,
            // JukeboxSong.java:46-48: ceil the single-precision seconds * 20.
            length_in_ticks: (f32::from_bits(value.length_in_seconds_bits) * 20.0).ceil() as i32,
        })
    }

    /// `JukeboxSong.hasFinished` (`JukeboxSong.java:50-52`).
    pub fn has_finished(self, ticks_elapsed: i64) -> bool {
        ticks_elapsed >= i64::from(self.length_in_ticks) + SONG_END_PADDING_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_song_carries_its_registered_comparator_output_and_ceiled_tick_length() {
        let cat = JukeboxSongFacts::resolve(
            "minecraft:cat",
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .unwrap();
        assert_eq!(cat.comparator_output, 2);
        assert_eq!(cat.length_in_ticks, 185 * 20);
    }

    #[test]
    fn a_song_finishes_a_padding_window_after_its_tick_length() {
        let cat = JukeboxSongFacts::resolve(
            "minecraft:cat",
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .unwrap();
        assert!(!cat.has_finished(0));
        assert!(!cat.has_finished(i64::from(cat.length_in_ticks) + 19));
        assert!(cat.has_finished(i64::from(cat.length_in_ticks) + 20));
    }
}
