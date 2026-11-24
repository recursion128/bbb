//! `JukeboxBlockEntity` (`JukeboxBlockEntity.java:22-169`) and the
//! `JukeboxSongPlayer` (`JukeboxSongPlayer.java:14-93`) it owns.
//!
//! The player's broadcasts are the level's: each transition answers with the
//! [`JukeboxSongEvent`] its caller owes, because a block entity here reaches no
//! level. `HAS_RECORD` block-state writes stay outside this module for the same
//! reason.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use crate::inventory::jukebox_playable_song;
use crate::jukebox_song_facts::JukeboxSongFacts;
use bbb_registry_codec::PersistentItemStackHolderResolver;

use super::value_input::numeric_long;
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};

/// `JukeboxBlockEntity.SONG_ITEM_TAG_ID` (`JukeboxBlockEntity.java:23`).
const TAG_RECORD_ITEM: &str = "RecordItem";
/// `JukeboxBlockEntity.TICKS_SINCE_SONG_STARTED_TAG_ID` (`:24`).
const TAG_TICKS_SINCE_SONG_STARTED: &str = "ticks_since_song_started";
/// `JukeboxSongPlayer.PLAY_EVENT_INTERVAL_TICKS` (`JukeboxSongPlayer.java:15`).
const PLAY_EVENT_INTERVAL_TICKS: i64 = 20;

/// What a song-player transition leaves for the level to broadcast.
///
/// Each variant is one `JukeboxSongPlayer` body's level half; the two that
/// change the song also owe `onSongChanged` (`JukeboxBlockEntity.java:36-39`),
/// which is `updateNeighborsAt` plus `setChanged`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JukeboxSongEvent {
    /// Nothing happened: `stop` on a player holding no song, or a tick with
    /// none.
    None,
    /// `play` (`JukeboxSongPlayer.java:45-51`): `levelEvent(null, 1010, pos,
    /// songId)`, then `onSongChanged`.
    Started,
    /// `stop` (`:53-61`): `gameEvent(JUKEBOX_STOP_PLAY)`, `levelEvent(1011,
    /// pos, 0)`, then `onSongChanged`.
    Stopped,
    /// The 20-tick beat of a running song (`:68-71`):
    /// `gameEvent(JUKEBOX_PLAY)` and the note particles beside it.
    Playing,
}

/// `JukeboxSongPlayer` (`JukeboxSongPlayer.java:14-24`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JukeboxSongPlayer {
    /// `JukeboxSongPlayer.song` (`:17`), named by the registry id its holder
    /// resolves to.
    song: Option<String>,
    /// `JukeboxSongPlayer.ticksSinceSongStarted` (`:16`).
    ticks_since_song_started: i64,
}

impl JukeboxSongPlayer {
    /// `JukeboxSongPlayer.isPlaying` (`JukeboxSongPlayer.java:26-28`).
    #[must_use]
    pub const fn is_playing(&self) -> bool {
        self.song.is_some()
    }

    /// The registry id of [`Self::song`], which `play`'s level event resolves
    /// to a holder id (`:48`).
    #[must_use]
    pub fn song_id(&self) -> Option<&str> {
        self.song.as_deref()
    }

    /// `JukeboxSongPlayer.getTicksSinceSongStarted` (`:34-36`).
    #[must_use]
    pub const fn ticks_since_song_started(&self) -> i64 {
        self.ticks_since_song_started
    }

    /// `JukeboxSongPlayer.setSongWithoutPlaying` (`:38-43`).
    fn set_song_without_playing(&mut self, song: String, facts: JukeboxSongFacts, ticks: i64) {
        if !facts.has_finished(ticks) {
            self.song = Some(song);
            self.ticks_since_song_started = ticks;
        }
    }

    /// `JukeboxSongPlayer.play` (`:45-51`).
    fn play(&mut self, song: String) -> JukeboxSongEvent {
        self.song = Some(song);
        self.ticks_since_song_started = 0;
        JukeboxSongEvent::Started
    }

    /// `JukeboxSongPlayer.stop` (`:53-61`).
    fn stop(&mut self) -> JukeboxSongEvent {
        if self.song.is_none() {
            return JukeboxSongEvent::None;
        }
        self.song = None;
        self.ticks_since_song_started = 0;
        JukeboxSongEvent::Stopped
    }

    /// `JukeboxSongPlayer.tick` (`:63-76`).
    fn tick(&mut self, facts: Option<JukeboxSongFacts>) -> JukeboxSongEvent {
        let Some(facts) = facts else {
            return JukeboxSongEvent::None;
        };
        if facts.has_finished(self.ticks_since_song_started) {
            return self.stop();
        }
        // `shouldEmitJukeboxPlayingEvent` (`:78-80`).
        let beat = self.ticks_since_song_started % PLAY_EVENT_INTERVAL_TICKS == 0;
        self.ticks_since_song_started += 1;
        if beat {
            JukeboxSongEvent::Playing
        } else {
            JukeboxSongEvent::None
        }
    }
}

/// `JukeboxBlockEntity` (`JukeboxBlockEntity.java:22-26`).
#[derive(Debug, Clone, PartialEq)]
pub struct JukeboxBlockEntity {
    item: RuntimeItemStack,
    song_player: JukeboxSongPlayer,
}

impl Default for JukeboxBlockEntity {
    fn default() -> Self {
        Self {
            item: RuntimeItemStack::empty(),
            song_player: JukeboxSongPlayer::default(),
        }
    }
}

impl JukeboxBlockEntity {
    pub(super) fn original_items(&self) -> &[RuntimeItemStack] {
        std::slice::from_ref(&self.item)
    }

    pub(super) fn original_items_mut(&mut self) -> &mut [RuntimeItemStack] {
        std::slice::from_mut(&mut self.item)
    }

    /// `JukeboxBlockEntity.getTheItem` (`JukeboxBlockEntity.java:101-103`).
    #[must_use]
    pub fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    /// `JukeboxBlockEntity.getSongPlayer` (`:32-34`).
    #[must_use]
    pub const fn song_player(&self) -> &JukeboxSongPlayer {
        &self.song_player
    }

    pub fn comparator_output(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<i32> {
        let Some(song_id) = jukebox_playable_song(&self.item, resolver)? else {
            return Ok(0);
        };
        Ok(JukeboxSongFacts::resolve(&song_id, resolver)?.comparator_output)
    }

    /// `JukeboxBlockEntity.setTheItem` (`:112-123`) without its
    /// `notifyItemChangedInJukebox` half, which is the caller's `HAS_RECORD`
    /// write and `BLOCK_CHANGE` game event.
    pub fn set_the_item(
        &mut self,
        item: RuntimeItemStack,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<JukeboxSongEvent> {
        self.item = item;
        let song = Self::stack_song(&self.item, resolver)?;
        Ok(match song {
            Some((song, _)) if !self.item.is_empty() => self.song_player.play(song),
            _ => self.song_player.stop(),
        })
    }

    /// `JukeboxBlockEntity.tick` (`:64-66`).
    pub fn tick(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<JukeboxSongEvent> {
        let facts = self
            .song_player
            .song_id()
            .map(|id| JukeboxSongFacts::resolve(id, resolver))
            .transpose()?;
        Ok(self.song_player.tick(facts))
    }

    /// `JukeboxSong.fromStack(this.item)` resolved against the registry.
    fn stack_song(
        item: &RuntimeItemStack,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<Option<(String, JukeboxSongFacts)>> {
        let Some(song_id) = jukebox_playable_song(item, resolver)? else {
            return Ok(None);
        };
        let facts = JukeboxSongFacts::resolve(&song_id, resolver)?;
        Ok(Some((song_id, facts)))
    }

    /// `JukeboxBlockEntity.saveAdditional` (`JukeboxBlockEntity.java:89-98`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        if !self.item.is_empty() {
            // `output.store("RecordItem", ItemStack.CODEC, ...)` (`:92`) leaves
            // the field out when the encode fails: `ItemStack.CODEC` never
            // answers a partial value for `TagValueOutput.java:43`.
            if let Some(item) = save.write(&self.item)? {
                entries.push(entry(TAG_RECORD_ITEM, item));
            }
        }
        if self.song_player.is_playing() {
            entries.push(entry(
                TAG_TICKS_SINCE_SONG_STARTED,
                NbtSummaryValue::Long(self.song_player.ticks_since_song_started),
            ));
        }
        Ok(entries)
    }

    /// `JukeboxBlockEntity.loadAdditional` (`JukeboxBlockEntity.java:73-86`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        let new_item = match unique_field(record, TAG_RECORD_ITEM) {
            Some(value) => load_item(value, load)?.unwrap_or_else(RuntimeItemStack::empty),
            None => RuntimeItemStack::empty(),
        };
        if !self.item.is_empty() && !new_item.same_item_same_components(&self.item) {
            self.song_player.stop();
        }
        self.item = new_item;
        let Some(ticks) = unique_field(record, TAG_TICKS_SINCE_SONG_STARTED).and_then(numeric_long)
        else {
            return Ok(());
        };
        if let Some((song, facts)) = Self::stack_song(&self.item, load.resolver())? {
            self.song_player
                .set_song_without_playing(song, facts, ticks);
        }
        Ok(())
    }
}

fn load_item(
    value: &NbtSummaryValue,
    load: &mut BlockEntityItemLoad<'_>,
) -> Result<Option<RuntimeItemStack>> {
    load.read(value)
}

/// `JukeboxBlockEntity.preRemoveSideEffects` (`JukeboxBlockEntity.java:153-155`)
/// through `popOutTheItem` (`:48-62`): the record leaves the object and lands
/// above the block with a random horizontal spread.
#[must_use]
pub fn removal_drops(
    jukebox: &JukeboxBlockEntity,
    old_state: i32,
) -> super::BlockEntityRemovalDrops {
    let record = (!jukebox.item.is_empty()).then(|| jukebox.item.clone());
    super::BlockEntityRemovalDrops::Jukebox {
        state: old_state,
        // `removeTheItem` reaches `JukeboxSongPlayer.stop` (`:113-122`).
        stops_song: record.is_some() && jukebox.song_player.is_playing(),
        record,
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_JUKEBOX_ID;
    use bbb_protocol::item_types::{VANILLA_ITEM_MUSIC_DISC_CAT_ID, VANILLA_ITEM_STONE_ID};
    use bbb_protocol::packets::{BlockPos, ItemStackSummary};
    use bbb_protocol::NbtSummaryValue;

    use crate::block_entity::{new_block_entity, unique_field, BlockEntityRemovalDrops};

    use super::*;

    fn resolver() -> &'static dyn PersistentItemStackHolderResolver {
        crate::world::registry_runtime::item_stack_resolver_for_test()
    }

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_JUKEBOX_ID)
            .expect("minecraft:jukebox has a default state")
    }

    fn stack(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        }
    }

    fn disc() -> ItemStackSummary {
        stack(VANILLA_ITEM_MUSIC_DISC_CAT_ID)
    }

    /// `register(context, CAT, MUSIC_DISC_CAT, 185, 2)` (`JukeboxSongs.java:57`).
    const CAT_LENGTH_IN_TICKS: i64 = 185 * 20;

    fn jukebox() -> JukeboxBlockEntity {
        JukeboxBlockEntity::default()
    }

    #[test]
    fn inserting_a_playable_record_starts_its_song_and_answers_its_comparator_output() {
        let mut jukebox = jukebox();
        assert_eq!(
            jukebox
                .set_the_item(RuntimeItemStack::from_summary(disc()), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::Started
        );
        assert!(jukebox.song_player().is_playing());
        assert_eq!(jukebox.song_player().song_id(), Some("minecraft:cat"));
        assert_eq!(jukebox.song_player().ticks_since_song_started(), 0);
        assert_eq!(jukebox.comparator_output(resolver()).unwrap(), 2);
    }

    #[test]
    fn a_record_without_a_song_and_an_empty_stack_both_stop_the_player() {
        let mut jukebox = jukebox();
        // `setTheItem` with nothing playing has nothing to stop.
        assert_eq!(
            jukebox
                .set_the_item(stack(VANILLA_ITEM_STONE_ID).into(), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::None
        );
        assert!(!jukebox.song_player().is_playing());
        assert_eq!(jukebox.comparator_output(resolver()).unwrap(), 0);

        assert_eq!(
            jukebox
                .set_the_item(RuntimeItemStack::from_summary(disc()), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::Started
        );
        assert_eq!(
            jukebox
                .set_the_item(stack(VANILLA_ITEM_STONE_ID).into(), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::Stopped
        );
        assert!(!jukebox.song_player().is_playing());

        assert_eq!(
            jukebox
                .set_the_item(RuntimeItemStack::from_summary(disc()), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::Started
        );
        assert_eq!(
            jukebox
                .set_the_item(ItemStackSummary::empty().into(), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::Stopped
        );
        assert!(!jukebox.song_player().is_playing());
        assert_eq!(jukebox.comparator_output(resolver()).unwrap(), 0);
    }

    #[test]
    fn a_playing_song_beats_every_twenty_ticks_and_stops_a_padding_window_past_its_end() {
        let mut jukebox = jukebox();
        assert_eq!(
            jukebox
                .set_the_item(RuntimeItemStack::from_summary(disc()), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::Started
        );
        // `shouldEmitJukeboxPlayingEvent` is true on the tick the counter is a
        // multiple of twenty, which is the first one.
        assert_eq!(jukebox.tick(resolver()).unwrap(), JukeboxSongEvent::Playing);
        for _ in 1..20 {
            assert_eq!(jukebox.tick(resolver()).unwrap(), JukeboxSongEvent::None);
        }
        assert_eq!(jukebox.song_player().ticks_since_song_started(), 20);
        assert_eq!(jukebox.tick(resolver()).unwrap(), JukeboxSongEvent::Playing);

        while jukebox.song_player().ticks_since_song_started() < CAT_LENGTH_IN_TICKS + 20 {
            assert_ne!(jukebox.tick(resolver()).unwrap(), JukeboxSongEvent::Stopped);
        }
        assert_eq!(jukebox.tick(resolver()).unwrap(), JukeboxSongEvent::Stopped);
        assert!(!jukebox.song_player().is_playing());
        assert_eq!(jukebox.tick(resolver()).unwrap(), JukeboxSongEvent::None);
        // The record stays in: only the player stopped.
        assert_eq!(jukebox.comparator_output(resolver()).unwrap(), 2);
    }

    #[test]
    fn a_jukebox_round_trips_its_record_and_song_ticks() {
        let mut built =
            new_block_entity(pos(), default_state()).expect("a jukebox builds an object");
        assert_eq!(
            built
                .jukebox_mut()
                .expect("payload is a jukebox")
                .set_the_item(RuntimeItemStack::from_summary(disc()), resolver())
                .expect("fixture song"),
            JukeboxSongEvent::Started
        );
        for _ in 0..42 {
            built
                .jukebox_mut()
                .expect("payload is a jukebox")
                .tick(resolver())
                .unwrap();
        }
        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        assert!(unique_field(&record, TAG_RECORD_ITEM).is_some());
        assert_eq!(
            unique_field(&record, TAG_TICKS_SINCE_SONG_STARTED),
            Some(&NbtSummaryValue::Long(42))
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture load")
            .expect("the record names a jukebox");
        let jukebox = loaded.jukebox().expect("payload is a jukebox");
        assert_eq!(
            jukebox.item().item_id(),
            Some(VANILLA_ITEM_MUSIC_DISC_CAT_ID)
        );
        assert_eq!(jukebox.item().count(), 1);
        assert!(jukebox.song_player().is_playing());
        assert_eq!(jukebox.song_player().ticks_since_song_started(), 42);
        let again = crate::block_entity::load_test_fixture(
            pos(),
            default_state(),
            &loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
                .expect("fixture save"),
        )
        .expect("fixture load")
        .expect("the round-trip still names a jukebox");
        let again = again.jukebox().expect("payload is a jukebox");
        assert_eq!(again.item().item_id(), Some(VANILLA_ITEM_MUSIC_DISC_CAT_ID));
        assert_eq!(again.song_player().ticks_since_song_started(), 42);
        assert_eq!(
            loaded.pre_remove_side_effects(loaded.block_state()),
            BlockEntityRemovalDrops::Jukebox {
                state: loaded.block_state(),
                record: Some(again.item().clone()),
                stops_song: again.song_player().is_playing(),
            }
        );
    }

    /// `setSongWithoutPlaying` (`JukeboxSongPlayer.java:38-43`) drops a song
    /// the saved tick count has already run past.
    #[test]
    fn a_finished_tick_count_loads_as_a_record_that_is_not_playing() {
        let mut built =
            new_block_entity(pos(), default_state()).expect("a jukebox builds an object");
        built
            .jukebox_mut()
            .expect("payload is a jukebox")
            .set_the_item(RuntimeItemStack::from_summary(disc()), resolver())
            .expect("fixture song");
        let mut record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        for field in &mut record {
            if field.name == TAG_TICKS_SINCE_SONG_STARTED {
                field.value = NbtSummaryValue::Long(CAT_LENGTH_IN_TICKS + 20);
            }
        }
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture load")
            .expect("the record names a jukebox");
        let jukebox = loaded.jukebox().expect("payload is a jukebox");
        assert_eq!(
            jukebox.item().item_id(),
            Some(VANILLA_ITEM_MUSIC_DISC_CAT_ID)
        );
        assert!(!jukebox.song_player().is_playing());
        assert_eq!(jukebox.song_player().ticks_since_song_started(), 0);
    }
}
