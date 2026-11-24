use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_AIR_SUPPLY_DATA_ID, VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
    VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID, VANILLA_ENTITY_POSE_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SHARED_FLAG_GLOWING,
    VANILLA_ENTITY_SILENT_DATA_ID, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
};
use bbb_protocol::RichTextComponent;
use uuid::Uuid;

use super::{Entity, EntityData, EntityStore, MAX_ENTITY_TAG_COUNT};
use crate::world::simulation_level::LevelCtx;

impl Entity {
    pub fn entity_tags(&self) -> &[bbb_protocol::JavaString] {
        &self.tags
    }

    pub fn add_tag(&mut self, tag: impl Into<bbb_protocol::JavaString>) -> bool {
        if self.tags.len() >= MAX_ENTITY_TAG_COUNT {
            return false;
        }
        let tag = tag.into();
        if self.tags.contains(&tag) {
            return false;
        }
        self.tags.push(tag);
        true
    }

    pub fn remove_tag(&mut self, tag: &bbb_protocol::JavaString) -> bool {
        let held = self.tags.iter().position(|held| held == tag);
        if let Some(index) = held {
            self.tags.remove(index);
        }
        held.is_some()
    }

    pub(crate) fn set_entity_tags(&mut self, tags: Vec<bbb_protocol::JavaString>) {
        self.tags = tags;
    }

    /// `Entity.getSharedFlag`'s whole byte (`Entity.java:2712-2714`).
    pub const fn shared_flags(&self) -> i8 {
        self.shared_flags
    }

    pub fn set_shared_flag(&mut self, mask: i8, value: bool) {
        let flags = if value {
            self.shared_flags | mask
        } else {
            self.shared_flags & !mask
        };
        self.set_synched_byte_field(
            VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
            &mut |entity| &mut entity.shared_flags,
            flags,
        );
    }

    pub const fn get_shared_flag(&self, mask: i8) -> bool {
        self.shared_flags & mask != 0
    }

    pub fn custom_name(&self) -> Option<&RichTextComponent> {
        self.custom_name.as_ref()
    }

    pub fn tamable_owner(&self) -> Option<(bool, Option<Uuid>)> {
        match &self.data {
            EntityData::GeneralLiving(data) => data
                .tamable_retained()
                .map(|state| (state.is_tame(), state.owner())),
            _ => None,
        }
    }

    pub fn ownable_owner(&self) -> Option<Option<Uuid>> {
        match &self.data {
            EntityData::GeneralLiving(data) => data
                .tamable_retained()
                .map(|state| state.owner())
                .or_else(|| data.abstract_horse_retained().map(|state| state.owner())),
            _ => None,
        }
    }

    pub(crate) fn set_custom_name(&mut self, name: Option<RichTextComponent>) -> bool {
        if let (EntityData::GeneralLiving(data), Some(name)) = (&mut self.data, &name) {
            if let Some(state) = data
                .raider_retained_mut()
                .filter(|state| state.johnny() == Some(false))
            {
                let text = bbb_server_datapack::language::component_plain_text(name.component());
                if text.iter().copied().eq("Johnny".encode_utf16()) {
                    state.mark_johnny();
                }
            }
        }
        self.load_custom_name(name);
        true
    }

    pub(crate) fn load_custom_name(&mut self, name: Option<RichTextComponent>) {
        if self.custom_name != name {
            self.custom_name = name;
            self.mark_synched_dirty(VANILLA_ENTITY_CUSTOM_NAME_DATA_ID);
        }
    }

    pub(crate) const fn custom_name_visible(&self) -> bool {
        self.custom_name_visible
    }

    pub(crate) fn set_custom_name_visible(&mut self, visible: bool) {
        if self.custom_name_visible != visible {
            self.custom_name_visible = visible;
            self.mark_synched_dirty(VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID);
        }
    }

    /// `Entity.isSilent` / `setSilent` (`Entity.java:1416-1422`).
    pub const fn is_silent(&self) -> bool {
        self.silent
    }

    pub(crate) fn set_silent(&mut self, silent: bool) {
        if self.silent != silent {
            self.silent = silent;
            self.mark_synched_dirty(VANILLA_ENTITY_SILENT_DATA_ID);
        }
    }

    pub(crate) fn set_glowing_tag(&mut self, glowing: bool) {
        self.has_glowing_tag = glowing;
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_GLOWING, glowing);
    }

    pub(crate) const fn has_glowing_tag(&self) -> bool {
        self.has_glowing_tag
    }

    pub(crate) const fn ticks_frozen(&self) -> i32 {
        self.ticks_frozen
    }

    pub(crate) fn set_ticks_frozen(&mut self, ticks: i32) {
        if self.ticks_frozen != ticks {
            self.ticks_frozen = ticks;
            self.mark_synched_dirty(VANILLA_ENTITY_TICKS_FROZEN_DATA_ID);
        }
    }

    pub(crate) const fn has_visual_fire(&self) -> bool {
        self.has_visual_fire
    }

    pub(crate) fn set_has_visual_fire(&mut self, visual_fire: bool) {
        self.has_visual_fire = visual_fire;
    }

    /// `Entity.getPose()` (`Entity.java:438-440`).
    pub const fn pose_id(&self) -> i32 {
        self.pose_id
    }

    pub fn set_pose_id(&mut self, pose_id: i32) {
        if self.pose_id != pose_id {
            self.pose_id = pose_id;
            self.mark_synched_dirty(VANILLA_ENTITY_POSE_DATA_ID);
            crate::entity::living::refresh_dimensions(self);
        }
    }

    pub(crate) fn set_pose_id_with_level(
        &mut self,
        pose_id: i32,
        level: &LevelCtx<'_, '_>,
        store: Option<&mut EntityStore>,
    ) -> Option<()> {
        self.set_pose_id_with_context(pose_id, Some(level), store)
    }

    pub(crate) fn set_pose_id_with_context(
        &mut self,
        pose_id: i32,
        level: Option<&LevelCtx<'_, '_>>,
        store: Option<&mut EntityStore>,
    ) -> Option<()> {
        if self.pose_id == pose_id {
            return Some(());
        }
        let previous = [self.bb_width(), self.bb_height()];
        self.set_pose_id(pose_id);
        crate::entity::living::resolve_dimensions_change(self, previous, level, store)
    }

    pub fn set_air_supply(&mut self, air_supply: i32) {
        if self.air_supply != air_supply {
            self.air_supply = air_supply;
            self.mark_synched_dirty(VANILLA_ENTITY_AIR_SUPPLY_DATA_ID);
        }
    }

    fn set_synched_byte_field(
        &mut self,
        data_id: u8,
        field: &mut dyn FnMut(&mut Self) -> &mut i8,
        value: i8,
    ) {
        if *field(self) != value {
            *field(self) = value;
            self.mark_synched_dirty(data_id);
        }
    }

    pub fn mark_synched_dirty(&mut self, data_id: u8) {
        self.synched_dirty[(data_id >> 6) as usize] |= 1u64 << (data_id & 63);
    }

    pub fn synched_data_is_dirty(&self) -> bool {
        (self.synched_dirty[0]
            | self.synched_dirty[1]
            | self.synched_dirty[2]
            | self.synched_dirty[3])
            != 0
    }

    pub(crate) fn take_synched_dirty_mask(&mut self) -> [u64; 4] {
        std::mem::take(&mut self.synched_dirty)
    }

    pub fn take_synched_dirty(&mut self) -> Vec<u8> {
        let mask = self.take_synched_dirty_mask();
        let total: u32 = mask.iter().map(|word| word.count_ones()).sum();
        let mut dirty = Vec::with_capacity(total as usize);
        for (word_index, mut word) in mask.into_iter().enumerate() {
            let base = (word_index * 64) as u8;
            while word != 0 {
                let id = base + word.trailing_zeros() as u8;
                dirty.push(id);
                word &= word - 1;
            }
        }
        dirty
    }
}
