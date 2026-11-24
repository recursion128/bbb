//! Banner markers share the map's decorations and persistent data.

use super::{
    banner_decoration_type, banner_key, block_pos_key, ServerMapBanner, ServerMapSavedData,
    ServerMapStore,
};
use crate::block_behavior::banner::is_banner;
use crate::block_entity::BlockEntityPayload;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::RichTextComponent;

fn from_world(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<ServerMapBanner> {
    let state = level.block_state(pos);
    if !is_banner(state) {
        return None;
    }
    let BlockEntityPayload::Banner(banner) = level.chunks.block_entities().get(pos)?.payload()
    else {
        return None;
    };
    let id = vanilla_block_type(vanilla_block_state_block_type_id(state)?)?.resource_id;
    let color = id.strip_prefix("minecraft:")?;
    let color = color
        .strip_suffix("_wall_banner")
        .or_else(|| color.strip_suffix("_banner"))?;
    Some(ServerMapBanner {
        pos,
        color: color.to_owned(),
        name: banner
            .custom_name()
            .cloned()
            .and_then(|name| RichTextComponent::try_from_component_nbt(name).ok()),
    })
}

impl ServerMapStore {
    pub(in crate::world) fn toggle_banner(
        &mut self,
        map_id: i32,
        level: &LevelCtx<'_, '_>,
        pos: BlockPos,
    ) -> Option<bool> {
        let map = self.ensure_loaded_mut(map_id)?;
        let scale = f64::from(1_i32 << map.scale);
        let x = f64::from(pos.x) + 0.5;
        let z = f64::from(pos.z) + 0.5;
        // MapItemSavedData.java:392-419: exact marker toggles precede the tracked limit.
        if !(-63.0..=63.0).contains(&((x - f64::from(map.center_x)) / scale))
            || !(-63.0..=63.0).contains(&((z - f64::from(map.center_z)) / scale))
        {
            return Some(false);
        }
        let Some(banner) = from_world(level, pos) else {
            return Some(false);
        };
        let key = block_pos_key(pos);
        let decoration = banner_key(pos);
        if map.banners.get(&key) == Some(&banner) {
            map.banners.remove(&key);
            map.remove_decoration(&decoration);
        } else {
            if map.tracked_decoration_count > 256 {
                return Some(false);
            }
            let type_id = banner_decoration_type(&banner.color).expect("builtin banner color");
            map.add_decoration(
                type_id,
                Some(level.game_time()),
                &decoration,
                [x, z, 180.0],
                banner.name.clone(),
            )
            .expect("builtin banner decoration");
            map.banners.insert(key, banner);
        }
        map.mark_persistent_dirty();
        Some(true)
    }
}

impl ServerMapSavedData {
    pub(super) fn check_banners_in_area(
        &mut self,
        level: &LevelCtx<'_, '_>,
        x: i32,
        z: i32,
        width: i32,
        depth: i32,
    ) {
        // MapItemSavedData.java:422-434: only observed columns invalidate markers.
        loop {
            let stale = self
                .banners
                .range((x, i32::MIN, i32::MIN)..=(x + width - 1, i32::MAX, i32::MAX))
                .find(|(_, banner)| {
                    banner.pos.z >= z
                        && banner.pos.z < z + depth
                        && from_world(level, banner.pos).as_ref() != Some(banner)
                })
                .map(|(&key, _)| key);
            let Some(key) = stale else { break };
            let banner = self.banners.remove(&key).expect("observed banner exists");
            self.remove_decoration(&banner_key(banner.pos));
            self.mark_persistent_dirty();
        }
    }
}
