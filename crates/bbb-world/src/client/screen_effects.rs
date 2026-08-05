//! Vanilla `ScreenEffectRenderer` world-state queries: the eight-corner
//! view-blocking eye probe, the underwater overlay brightness sample, and the
//! local-player sleeping-pose gate that suppresses first-person screen
//! effects.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;

use super::local_player_collision::LOCAL_PLAYER_HALF_WIDTH;
use crate::entities::vanilla_lightmap_brightness;
use crate::terrain::block_name_has_invisible_render_shape;
use crate::{BlockPos, WorldStore};

impl WorldStore {
    /// Vanilla `ScreenEffectRenderer.getViewBlockingState`: the first of eight
    /// eye-corner samples whose block state both renders (its `RenderShape` is
    /// not `INVISIBLE`) and reports `isViewBlocking`. The corners span
    /// `±0.5 * bbWidth * 0.8` horizontally and `±0.05 * scale` around the eye
    /// vertically, in bit order x, y, z.
    pub fn local_player_view_blocking_state(&self) -> Option<i32> {
        let pose = self.local_player_pose()?;
        let eye_y = pose.position.y + pose.eye_height();
        let scale = self
            .local_player_id()
            .and_then(|id| self.entities.render_scale(id))
            .unwrap_or(1.0);
        let width = LOCAL_PLAYER_HALF_WIDTH * 2.0 * f64::from(scale);
        for corner in 0..8 {
            let x = pose.position.x + (f64::from(corner % 2) - 0.5) * width * 0.8;
            let y = eye_y + (f64::from((corner >> 1) % 2) - 0.5) * 0.1 * f64::from(scale);
            let z = pose.position.z + (f64::from((corner >> 2) % 2) - 0.5) * width * 0.8;
            let pos = BlockPos {
                x: x.floor() as i32,
                y: y.floor() as i32,
                z: z.floor() as i32,
            };
            let Some(probe) = self.probe_block(pos) else {
                continue;
            };
            if probe
                .block_name
                .as_deref()
                .is_some_and(block_name_has_invisible_render_shape)
            {
                continue;
            }
            if vanilla_block_state_support_facts(probe.block_state_id)
                .is_some_and(|facts| facts.view_blocking())
            {
                return Some(probe.block_state_id);
            }
        }
        None
    }

    /// Vanilla `ScreenEffectRenderer.renderWater` brightness:
    /// `Lightmap.getBrightness(dimensionType, level.getMaxLocalRawBrightness(eye))`
    /// where the raw brightness is `max(blockLight, skyLight - skyDarken)` at
    /// the eye position.
    pub fn local_player_eye_brightness(&self, sky_darken: u8) -> Option<f32> {
        let pose = self.local_player_pose()?;
        let eye_y = pose.position.y + pose.eye_height();
        let pos = BlockPos {
            x: pose.position.x.floor() as i32,
            y: eye_y.floor() as i32,
            z: pose.position.z.floor() as i32,
        };
        // A loaded vanilla chunk always carries light; an absent bbb light
        // section degrades to raw brightness 0 rather than suppressing the
        // overlay entirely.
        let raw = self.sample_block_light(pos).map_or(0, |light| {
            light.block.max(light.sky.saturating_sub(sky_darken))
        });
        let ambient_light = self.level_info().map_or(0.0, |level| level.ambient_light());
        Some(vanilla_lightmap_brightness(ambient_light, raw))
    }
}

#[cfg(test)]
mod tests {
    use super::super::local_player::LocalPlayerPoseState;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension, WorldStore,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate, Vec3d};
    use std::collections::BTreeMap;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_named_block(world: &mut WorldStore, x: i32, y: i32, z: i32, name: &str) -> i32 {
        let properties = if name == "minecraft:barrier" {
            BTreeMap::from([("waterlogged".to_string(), "false".to_string())])
        } else {
            BTreeMap::new()
        };
        let state = crate::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("block state for {name}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x, y, z },
            block_state_id: state,
        }));
        state
    }

    fn place_player(world: &mut WorldStore, x: f64, z: f64) {
        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d { x, y: 0.0, z },
            ..LocalPlayerPoseState::default()
        });
    }

    #[test]
    fn view_blocking_probe_reports_the_block_at_the_eye() {
        let mut world = world_with_air_chunk();
        place_player(&mut world, 0.5, 0.5);
        assert_eq!(world.local_player_view_blocking_state(), None);

        let stone = set_named_block(&mut world, 0, 1, 0, "minecraft:stone");
        assert_eq!(world.local_player_view_blocking_state(), Some(stone));
    }

    #[test]
    fn view_blocking_probe_skips_invisible_render_shapes() {
        // Vanilla `getViewBlockingState` requires `getRenderShape() != INVISIBLE`
        // before `isViewBlocking`; a barrier satisfies the latter but not the
        // former.
        let mut world = world_with_air_chunk();
        place_player(&mut world, 0.5, 0.5);
        set_named_block(&mut world, 0, 1, 0, "minecraft:barrier");
        assert_eq!(world.local_player_view_blocking_state(), None);
    }

    #[test]
    fn view_blocking_probe_reaches_the_eight_eye_corners() {
        // The corner samples span half of `bbWidth * 0.8` (0.24 for a player)
        // around the eye, so a block in the neighboring column still triggers
        // the overlay when the player hugs the cell boundary.
        let mut world = world_with_air_chunk();
        let stone = set_named_block(&mut world, 1, 1, 0, "minecraft:stone");

        place_player(&mut world, 0.5, 0.5);
        assert_eq!(world.local_player_view_blocking_state(), None);

        place_player(&mut world, 0.9, 0.5);
        assert_eq!(world.local_player_view_blocking_state(), Some(stone));
    }
}
