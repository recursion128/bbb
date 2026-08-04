//! Client-side copper golem statue block-entity render source.
//!
//! `CopperGolemStatueBlockRenderer.extractRenderState` reads everything it needs
//! off the block state — `FACING`, `POSE`, and the block's own
//! `WeatheringCopper.WeatherState` — so the block entity exists purely to give
//! the renderer a hook. This module owns the block-state projection.

use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID;

use crate::{BlockPos, WorldStore};

/// `CopperGolemStatueBlock.Pose`, in vanilla declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopperGolemStatuePoseKind {
    Standing,
    Sitting,
    Running,
    Star,
}

/// The `WeatheringCopper.WeatherState` each statue block is constructed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopperGolemStatueWeatherState {
    Unaffected,
    Exposed,
    Weathered,
    Oxidized,
}

/// One `CopperGolemStatueRenderState`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopperGolemStatueModelSourceState {
    pub pos: BlockPos,
    /// `Direction.toYRot()` of `CopperGolemStatueBlock.FACING`: south=0,
    /// west=90, north=180, east=270.
    pub facing_y_rot: f32,
    pub pose: CopperGolemStatuePoseKind,
    pub weather_state: CopperGolemStatueWeatherState,
}

impl WorldStore {
    pub fn copper_golem_statue_model_source_states(
        &self,
    ) -> Vec<CopperGolemStatueModelSourceState> {
        let mut states = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID {
                continue;
            }
            let Some(block) = self.probe_block(pos) else {
                continue;
            };
            let Some(name) = block.block_name.as_deref() else {
                continue;
            };
            let Some(weather_state) = copper_golem_statue_weather_state(name) else {
                continue;
            };
            let Some(facing_y_rot) = block
                .block_properties
                .get("facing")
                .and_then(|facing| horizontal_facing_y_rot(facing))
            else {
                continue;
            };
            let pose = match block
                .block_properties
                .get("copper_golem_pose")
                .map(String::as_str)
            {
                Some("standing") => CopperGolemStatuePoseKind::Standing,
                Some("sitting") => CopperGolemStatuePoseKind::Sitting,
                Some("running") => CopperGolemStatuePoseKind::Running,
                Some("star") => CopperGolemStatuePoseKind::Star,
                // `BlockStateProperties.COPPER_GOLEM_POSE` is mandatory;
                // anything else is not a statue state this client can render.
                _ => continue,
            };
            states.push(CopperGolemStatueModelSourceState {
                pos,
                facing_y_rot,
                pose,
                weather_state,
            });
        }
        states.sort_by_key(|state| state.pos);
        states
    }
}

/// The weathering level baked into each `CopperGolemStatueBlock` instance.
/// Waxed variants share their unwaxed block's state, matching vanilla's
/// `Blocks` registrations.
pub fn copper_golem_statue_weather_state(
    block_name: &str,
) -> Option<CopperGolemStatueWeatherState> {
    match block_name {
        "minecraft:copper_golem_statue" | "minecraft:waxed_copper_golem_statue" => {
            Some(CopperGolemStatueWeatherState::Unaffected)
        }
        "minecraft:exposed_copper_golem_statue" | "minecraft:waxed_exposed_copper_golem_statue" => {
            Some(CopperGolemStatueWeatherState::Exposed)
        }
        "minecraft:weathered_copper_golem_statue"
        | "minecraft:waxed_weathered_copper_golem_statue" => {
            Some(CopperGolemStatueWeatherState::Weathered)
        }
        "minecraft:oxidized_copper_golem_statue"
        | "minecraft:waxed_oxidized_copper_golem_statue" => {
            Some(CopperGolemStatueWeatherState::Oxidized)
        }
        _ => None,
    }
}

/// `Direction.toYRot()` for the four horizontal directions.
fn horizontal_facing_y_rot(facing: &str) -> Option<f32> {
    match facing {
        "south" => Some(0.0),
        "west" => Some(90.0),
        "north" => Some(180.0),
        "east" => Some(270.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    #[test]
    fn every_weathering_variant_including_waxed_maps_to_its_block_state() {
        for (name, expected) in [
            (
                "minecraft:copper_golem_statue",
                CopperGolemStatueWeatherState::Unaffected,
            ),
            (
                "minecraft:waxed_copper_golem_statue",
                CopperGolemStatueWeatherState::Unaffected,
            ),
            (
                "minecraft:exposed_copper_golem_statue",
                CopperGolemStatueWeatherState::Exposed,
            ),
            (
                "minecraft:waxed_exposed_copper_golem_statue",
                CopperGolemStatueWeatherState::Exposed,
            ),
            (
                "minecraft:weathered_copper_golem_statue",
                CopperGolemStatueWeatherState::Weathered,
            ),
            (
                "minecraft:waxed_weathered_copper_golem_statue",
                CopperGolemStatueWeatherState::Weathered,
            ),
            (
                "minecraft:oxidized_copper_golem_statue",
                CopperGolemStatueWeatherState::Oxidized,
            ),
            (
                "minecraft:waxed_oxidized_copper_golem_statue",
                CopperGolemStatueWeatherState::Oxidized,
            ),
        ] {
            assert_eq!(copper_golem_statue_weather_state(name), Some(expected));
        }
        assert_eq!(
            copper_golem_statue_weather_state("minecraft:copper_block"),
            None
        );
    }

    #[test]
    fn source_state_carries_the_block_states_facing_pose_and_weathering() {
        let world = statue_world("minecraft:weathered_copper_golem_statue", "east", "star");
        let states = world.copper_golem_statue_model_source_states();
        assert_eq!(
            states,
            vec![CopperGolemStatueModelSourceState {
                pos: BlockPos { x: 0, y: 0, z: 0 },
                facing_y_rot: 270.0,
                pose: CopperGolemStatuePoseKind::Star,
                weather_state: CopperGolemStatueWeatherState::Weathered,
            }]
        );
    }

    #[test]
    fn statues_whose_block_no_longer_matches_are_dropped() {
        let mut world = statue_world("minecraft:copper_golem_statue", "north", "sitting");
        assert_eq!(world.copper_golem_statue_model_source_states().len(), 1);
        // The block entity outliving its block leaves nothing to project.
        let air = crate::vanilla_air_block_state_id();
        world.chunks[0].sections[0].block_states = single_value_palette(air);
        assert!(world.copper_golem_statue_model_source_states().is_empty());
    }

    fn statue_world(name: &str, facing: &str, pose: &str) -> WorldStore {
        let mut properties = BTreeMap::new();
        properties.insert("facing".to_string(), facing.to_string());
        properties.insert("copper_golem_pose".to_string(), pose.to_string());
        properties.insert("waterlogged".to_string(), "false".to_string());
        let state_id = crate::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));

        let mut store = WorldStore::new();
        store.dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        store.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 0,
                block_states: single_value_palette(state_id),
                biomes: PalettedContainerData {
                    domain: PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: vec![BlockEntityRecord::empty(
                0,
                0,
                0,
                VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
            )],
            light: LightData::default(),
        });
        store
    }

    fn single_value_palette(global_id: i32) -> PalettedContainerData {
        PalettedContainerData {
            domain: PaletteDomain::BlockStates,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count: 4096,
        }
    }
}
