use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
};
use bbb_protocol::packets::MapPostProcessingSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::recipe::{CraftingInput, CraftingMapData};

impl ServerMapStore {
    pub(crate) fn crafting_map_data(&mut self, input: &CraftingInput) -> Option<CraftingMapData> {
        let id = input
            .items()
            .iter()
            .find_map(|item| item.component_patch.map_id)?;
        let map = self.ensure_loaded(id)?;
        // MapItemSavedData.java:463-471: inspect saved decorations, not the stack decoration component.
        Some(CraftingMapData {
            scale: map.scale,
            exploration: map.decorations.iter().any(|decoration| {
                bbb_facts::vanilla_map_decoration_type(decoration.value.type_id)
                    .is_some_and(|entry| entry.exploration_map_element)
            }),
        })
    }

    // MapItem.java:290-322: remove the transient operation even if the original map is absent.
    pub(crate) fn post_process_crafted_map(
        &mut self,
        item: RuntimeItemStack,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<RuntimeItemStack> {
        if item.item_id() != Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID) {
            return Ok(item);
        }
        let fail = |error: String| bbb_server_types::ServerWorldError::InvalidConfiguration(error);
        let Some(operation) = item.summary().component_patch.map_post_processing else {
            return Ok(item);
        };
        let original_id = item.map_id();
        let item = item.with_component_nbt(
            VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
            None,
            resolver,
        )?;
        let Some(original) = original_id.and_then(|id| self.ensure_loaded(id)).cloned() else {
            return Ok(item);
        };
        let id = self.last_id.wrapping_add(1);
        let item = item.with_component_nbt(
            VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
            Some(&NbtSummaryValue::Int(id)),
            resolver,
        )?;
        match operation {
            MapPostProcessingSummary::Scale => {
                self.create_fresh(
                    original.dimension,
                    f64::from(original.center_x),
                    f64::from(original.center_z),
                    original.scale.saturating_add(1).clamp(0, 4),
                    original.tracking_position,
                    original.unlimited_tracking,
                )
                .map_err(|error| fail(format!("crafted map: {error:?}")))?;
            }
            MapPostProcessingSummary::Lock => {
                // MapItemSavedData.java:151-159: decorations and banners copy, frames and carriers do not.
                let mut map = original;
                map.locked = true;
                map.frames.clear();
                map.holding_order.clear();
                map.holding_players.clear();
                map.persistent_epoch = 0;
                map.dirty_epoch = None;
                map.mark_persistent_dirty();
                self.last_id = id;
                self.index_dirty = true;
                self.stored_maps.remove(&id);
                self.maps.insert(id, map);
            }
        }
        Ok(item)
    }

    /// The scale and lock of the referenced maps (plus any already-loaded maps)
    /// that the cartography table may read.
    pub(in crate::world) fn cartography_facts_for(
        &mut self,
        referenced_ids: impl IntoIterator<Item = i32>,
    ) -> BTreeMap<i32, crate::inventory::MapFacts> {
        for id in referenced_ids {
            let _ = self.ensure_loaded(id);
        }
        self.maps
            .iter()
            .map(|(&id, map)| {
                (
                    id,
                    crate::inventory::MapFacts {
                        scale: map.scale,
                        locked: map.locked,
                    },
                )
            })
            .collect()
    }
}
