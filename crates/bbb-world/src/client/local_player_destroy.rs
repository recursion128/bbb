use bbb_protocol::packets::Direction as ProtocolDirection;

use crate::{BlockPos, WorldStore};

const LOCAL_DESTROY_PROGRESS_SCALE: u32 = 1_000_000;
pub(crate) const LOCAL_DESTROY_COMPLETION_DELAY_TICKS: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDestroyBlockFinished {
    pub pos: BlockPos,
    pub face: ProtocolDirection,
    pub sequence: i32,
}

impl WorldStore {
    pub fn set_local_destroy_delay_ticks(&mut self, ticks: u8) {
        self.local_player.interaction.destroy_delay_ticks = ticks;
    }

    pub fn tick_local_destroy_delay(&mut self) -> bool {
        let delay = &mut self.local_player.interaction.destroy_delay_ticks;
        if *delay == 0 {
            return false;
        }
        *delay -= 1;
        true
    }

    pub fn local_destroy_block_is_immediate(&self, pos: BlockPos) -> bool {
        self.local_destroy_progress_per_tick(pos)
            .is_some_and(|progress| progress >= LOCAL_DESTROY_PROGRESS_SCALE)
    }

    pub fn advance_local_destroying_block_tick(&mut self) -> Option<LocalDestroyBlockFinished> {
        let pos = self.local_player.interaction.destroying_block?;
        let progress_delta = match self.local_destroy_progress_per_tick(pos) {
            Some(progress_delta) if progress_delta > 0 => progress_delta,
            Some(_) => return None,
            None => {
                self.take_local_destroying_block();
                return None;
            }
        };

        let interaction = &mut self.local_player.interaction;
        interaction.destroying_block_progress = interaction
            .destroying_block_progress
            .saturating_add(progress_delta);
        interaction.destroying_block_ticks = interaction.destroying_block_ticks.saturating_add(1);
        if interaction.destroying_block_progress < LOCAL_DESTROY_PROGRESS_SCALE {
            return None;
        }

        let face = interaction
            .destroying_block_face
            .take()
            .unwrap_or(ProtocolDirection::Down);
        interaction.destroying_block = None;
        interaction.destroying_block_progress = 0;
        interaction.destroying_block_ticks = 0;
        interaction.destroy_delay_ticks = LOCAL_DESTROY_COMPLETION_DELAY_TICKS;
        let sequence = self.next_local_prediction_sequence();
        Some(LocalDestroyBlockFinished {
            pos,
            face,
            sequence,
        })
    }

    fn local_destroy_progress_per_tick(&self, pos: BlockPos) -> Option<u32> {
        let block = self.probe_block(pos)?;
        let block_name = match block.block_name.as_deref() {
            Some("minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") | None => {
                return None;
            }
            Some(block_name) => block_name,
        };
        local_destroy_progress_per_tick(block_name)
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalBlockDestroyProfile {
    destroy_time_tenths: u32,
    requires_correct_tool: bool,
}

fn local_destroy_progress_per_tick(block_name: &str) -> Option<u32> {
    let profile = local_block_destroy_profile(block_name)?;
    if profile.destroy_time_tenths == 0 {
        return Some(LOCAL_DESTROY_PROGRESS_SCALE);
    }

    let modifier = if profile.requires_correct_tool {
        100
    } else {
        30
    };
    let denominator = profile.destroy_time_tenths.saturating_mul(modifier);
    Some(ceil_div(
        LOCAL_DESTROY_PROGRESS_SCALE.saturating_mul(10),
        denominator,
    ))
}

fn local_block_destroy_profile(block_name: &str) -> Option<LocalBlockDestroyProfile> {
    let profile = |destroy_time_tenths, requires_correct_tool| LocalBlockDestroyProfile {
        destroy_time_tenths,
        requires_correct_tool,
    };
    match block_name {
        "minecraft:fire"
        | "minecraft:redstone_wire"
        | "minecraft:flower_pot"
        | "minecraft:potted_dandelion"
        | "minecraft:potted_poppy" => Some(profile(0, false)),
        "minecraft:grass_block" => Some(profile(6, false)),
        "minecraft:dirt"
        | "minecraft:coarse_dirt"
        | "minecraft:podzol"
        | "minecraft:rooted_dirt" => Some(profile(5, false)),
        "minecraft:sand" | "minecraft:red_sand" | "minecraft:gravel" => Some(profile(5, false)),
        "minecraft:clay" | "minecraft:sponge" | "minecraft:wet_sponge" => Some(profile(6, false)),
        "minecraft:glass" | "minecraft:glowstone" | "minecraft:sea_lantern" => {
            Some(profile(3, false))
        }
        "minecraft:netherrack" => Some(profile(4, false)),
        "minecraft:stone"
        | "minecraft:granite"
        | "minecraft:diorite"
        | "minecraft:andesite"
        | "minecraft:deepslate"
        | "minecraft:tuff"
        | "minecraft:calcite" => Some(profile(15, true)),
        "minecraft:cobblestone" | "minecraft:mossy_cobblestone" => Some(profile(20, true)),
        "minecraft:obsidian" => Some(profile(500, true)),
        name if name.ends_with("_log")
            || name.ends_with("_wood")
            || name.ends_with("_stem")
            || name.ends_with("_hyphae")
            || name.ends_with("_planks") =>
        {
            Some(profile(20, false))
        }
        name if name.ends_with("_leaves") => Some(profile(2, false)),
        _ => None,
    }
}

fn ceil_div(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    numerator / denominator + u32::from(numerator % denominator != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockPos as ProtocolBlockPos, BlockUpdate, Direction as ProtocolDirection,
    };

    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    #[test]
    fn local_destroy_progress_uses_vanilla_hand_formula_for_known_blocks() {
        assert_eq!(
            local_destroy_progress_per_tick("minecraft:grass_block"),
            Some(55_556)
        );
        assert_eq!(
            local_destroy_progress_per_tick("minecraft:dirt"),
            Some(66_667)
        );
        assert_eq!(
            local_destroy_progress_per_tick("minecraft:stone"),
            Some(6_667)
        );
        assert_eq!(
            local_destroy_progress_per_tick("minecraft:redstone_wire"),
            Some(1_000_000)
        );
        assert_eq!(local_destroy_progress_per_tick("minecraft:unknown"), None);
    }

    #[test]
    fn local_destroy_progress_finishes_known_block_after_client_ticks() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 9);
        world.set_local_destroying_block_hit(pos, ProtocolDirection::North);

        for _ in 0..17 {
            assert_eq!(world.advance_local_destroying_block_tick(), None);
        }
        assert_eq!(world.local_player().interaction.destroying_block, Some(pos));
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 17);
        assert!(world.local_player().interaction.destroying_block_progress > 0);

        assert_eq!(
            world.advance_local_destroying_block_tick(),
            Some(LocalDestroyBlockFinished {
                pos,
                face: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
    }

    fn world_with_block(pos: BlockPos, block_state_id: i32) -> WorldStore {
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
                block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
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
}
