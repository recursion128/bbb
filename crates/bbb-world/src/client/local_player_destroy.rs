use bbb_protocol::packets::Direction as ProtocolDirection;
use serde::{Deserialize, Serialize};

use crate::{BlockPos, WorldStore};

const LOCAL_DESTROY_PROGRESS_SCALE: u32 = 1_000_000;
pub(crate) const LOCAL_DESTROY_COMPLETION_DELAY_TICKS: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDestroyBlockFinished {
    pub pos: BlockPos,
    pub face: ProtocolDirection,
    pub sequence: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldBlockDestroyProfile {
    pub destroy_time_tenths: Option<u32>,
    pub requires_correct_tool: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldItemMiningRule {
    pub block_names: Vec<String>,
    pub mining_speed_thousandths: Option<u32>,
    pub correct_for_drops: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldItemMiningProfile {
    pub default_mining_speed_thousandths: u32,
    pub rules: Vec<WorldItemMiningRule>,
}

impl WorldStore {
    pub fn set_default_block_destroy_profiles(
        &mut self,
        profiles: std::collections::BTreeMap<String, WorldBlockDestroyProfile>,
    ) {
        self.default_block_destroy_profiles = profiles
            .into_iter()
            .filter(|(block_name, _)| !block_name.is_empty())
            .collect();
    }

    pub fn set_default_item_mining_profiles(
        &mut self,
        profiles: std::collections::BTreeMap<i32, WorldItemMiningProfile>,
    ) {
        self.default_item_mining_profiles = profiles
            .into_iter()
            .filter(|(item_id, profile)| {
                *item_id >= 0
                    && profile.default_mining_speed_thousandths > 0
                    && !profile.rules.is_empty()
            })
            .collect();
    }

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
            interaction.destroying_block_stage =
                local_destroy_stage(interaction.destroying_block_progress);
            return None;
        }

        let face = interaction
            .destroying_block_face
            .take()
            .unwrap_or(ProtocolDirection::Down);
        interaction.destroying_block = None;
        interaction.destroying_item_signature = None;
        interaction.destroying_block_progress = 0;
        interaction.destroying_block_stage = None;
        interaction.destroying_block_ticks = 0;
        interaction.destroy_delay_ticks = LOCAL_DESTROY_COMPLETION_DELAY_TICKS;
        let sequence = self.next_local_prediction_sequence();
        self.predict_local_destroy_block(pos, sequence);
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
        let hotbar_items = self.inventory.hotbar_items();
        let selected_slot = usize::from(self.local_player.selected_hotbar_slot.min(8));
        let item = &hotbar_items[selected_slot];
        let mining_speed = self.local_destroy_mining_speed_thousandths(block_name, item);
        let correct_for_drops = self.local_destroy_item_is_correct_for_drops(block_name, item);
        let block_profile = self.local_block_destroy_profile(block_name)?;
        local_destroy_progress_per_tick_with_tool(block_profile, mining_speed, correct_for_drops)
    }

    fn local_destroy_mining_speed_thousandths(
        &self,
        block_name: &str,
        item: &bbb_protocol::packets::ItemStackSummary,
    ) -> u32 {
        item.item_id
            .filter(|_| item.count > 0)
            .and_then(|item_id| self.default_item_mining_profiles.get(&item_id))
            .map(|profile| item_mining_speed_thousandths(profile, block_name))
            .unwrap_or(1_000)
    }

    fn local_destroy_item_is_correct_for_drops(
        &self,
        block_name: &str,
        item: &bbb_protocol::packets::ItemStackSummary,
    ) -> bool {
        item.item_id
            .filter(|_| item.count > 0)
            .and_then(|item_id| self.default_item_mining_profiles.get(&item_id))
            .is_some_and(|profile| item_is_correct_for_drops(profile, block_name))
    }

    fn local_block_destroy_profile(&self, block_name: &str) -> Option<LocalBlockDestroyProfile> {
        self.default_block_destroy_profiles
            .get(block_name)
            .map(LocalBlockDestroyProfile::from)
            .or_else(|| fallback_local_block_destroy_profile(block_name))
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalBlockDestroyProfile {
    destroy_time_tenths: Option<u32>,
    requires_correct_tool: bool,
}

impl From<&WorldBlockDestroyProfile> for LocalBlockDestroyProfile {
    fn from(profile: &WorldBlockDestroyProfile) -> Self {
        Self {
            destroy_time_tenths: profile.destroy_time_tenths,
            requires_correct_tool: profile.requires_correct_tool,
        }
    }
}

#[cfg(test)]
fn local_destroy_progress_per_tick(block_name: &str) -> Option<u32> {
    local_destroy_progress_per_tick_with_tool(
        fallback_local_block_destroy_profile(block_name)?,
        1_000,
        false,
    )
}

fn local_destroy_progress_per_tick_with_tool(
    profile: LocalBlockDestroyProfile,
    mining_speed_thousandths: u32,
    correct_for_drops: bool,
) -> Option<u32> {
    let destroy_time_tenths = match profile.destroy_time_tenths {
        Some(0) => return Some(LOCAL_DESTROY_PROGRESS_SCALE),
        Some(destroy_time_tenths) => destroy_time_tenths,
        None => return Some(0),
    };

    let modifier = if !profile.requires_correct_tool || correct_for_drops {
        30_u64
    } else {
        100_u64
    };
    let numerator =
        u64::from(LOCAL_DESTROY_PROGRESS_SCALE).saturating_mul(u64::from(mining_speed_thousandths));
    let denominator = u64::from(destroy_time_tenths)
        .saturating_mul(100)
        .saturating_mul(modifier);
    Some(ceil_div_u64(numerator, denominator).min(u64::from(u32::MAX)) as u32)
}

fn local_destroy_stage(progress: u32) -> Option<u8> {
    if progress == 0 {
        return None;
    }
    Some(((progress.saturating_mul(10)) / LOCAL_DESTROY_PROGRESS_SCALE).min(9) as u8)
}

fn fallback_local_block_destroy_profile(block_name: &str) -> Option<LocalBlockDestroyProfile> {
    let profile = |destroy_time_tenths, requires_correct_tool| LocalBlockDestroyProfile {
        destroy_time_tenths: Some(destroy_time_tenths),
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

fn item_mining_speed_thousandths(profile: &WorldItemMiningProfile, block_name: &str) -> u32 {
    profile
        .rules
        .iter()
        .find_map(|rule| {
            rule.mining_speed_thousandths
                .filter(|_| mining_rule_matches(rule, block_name))
        })
        .unwrap_or(profile.default_mining_speed_thousandths)
}

fn item_is_correct_for_drops(profile: &WorldItemMiningProfile, block_name: &str) -> bool {
    profile
        .rules
        .iter()
        .find_map(|rule| {
            rule.correct_for_drops
                .filter(|_| mining_rule_matches(rule, block_name))
        })
        .unwrap_or(false)
}

fn mining_rule_matches(rule: &WorldItemMiningRule, block_name: &str) -> bool {
    rule.block_names.iter().any(|name| name == block_name)
}

fn ceil_div_u64(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    numerator / denominator + u64::from(numerator % denominator != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockPos as ProtocolBlockPos, BlockUpdate, Direction as ProtocolDirection,
        ItemStackSummary as ProtocolItemStackSummary,
        SetPlayerInventory as ProtocolSetPlayerInventory,
    };
    use std::collections::BTreeMap;

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
    fn local_destroy_progress_uses_selected_item_mining_profile() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        assert_eq!(
            world.probe_block(pos).unwrap().block_name.as_deref(),
            Some("minecraft:stone")
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));

        world.set_default_item_mining_profiles(BTreeMap::from([(
            42,
            mining_profile(vec![
                mining_rule(vec!["minecraft:obsidian"], None, Some(false)),
                mining_rule(vec!["minecraft:stone"], Some(4_000), Some(true)),
            ]),
        )]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));

        world.set_default_item_mining_profiles(BTreeMap::from([(
            42,
            mining_profile(vec![
                mining_rule(vec!["minecraft:stone"], None, Some(false)),
                mining_rule(vec!["minecraft:stone"], Some(4_000), Some(true)),
            ]),
        )]));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(26_667));
    }

    #[test]
    fn local_destroy_progress_uses_tool_speed_for_blocks_without_required_tool() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 9);
        assert_eq!(
            world.probe_block(pos).unwrap().block_name.as_deref(),
            Some("minecraft:grass_block")
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(55_556));

        world.set_default_item_mining_profiles(BTreeMap::from([(
            43,
            mining_profile(vec![mining_rule(
                vec!["minecraft:grass_block"],
                Some(6_000),
                Some(true),
            )]),
        )]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(43, 1),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(333_334));
    }

    #[test]
    fn local_destroy_progress_uses_injected_block_destroy_profiles() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 5307);
        assert_eq!(
            world.probe_block(pos).unwrap().block_name.as_deref(),
            Some("minecraft:diamond_ore")
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), None);

        world.set_default_block_destroy_profiles(BTreeMap::from([(
            "minecraft:diamond_ore".to_string(),
            block_destroy_profile(Some(30), true),
        )]));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(3_334));

        world.set_default_item_mining_profiles(BTreeMap::from([(
            42,
            mining_profile(vec![mining_rule(
                vec!["minecraft:diamond_ore"],
                Some(8_000),
                Some(true),
            )]),
        )]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));
    }

    #[test]
    fn local_destroy_progress_keeps_unbreakable_blocks_active_without_progress() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 85);
        assert_eq!(
            world.probe_block(pos).unwrap().block_name.as_deref(),
            Some("minecraft:bedrock")
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), None);

        world.set_default_block_destroy_profiles(BTreeMap::from([(
            "minecraft:bedrock".to_string(),
            block_destroy_profile(None, false),
        )]));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(0));

        world.set_local_destroying_block_hit(pos, ProtocolDirection::North);
        assert_eq!(world.advance_local_destroying_block_tick(), None);
        assert_eq!(world.local_player().interaction.destroying_block, Some(pos));
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
    }

    #[test]
    fn local_destroy_stage_tracks_active_progress_range() {
        assert_eq!(local_destroy_stage(0), None);
        assert_eq!(local_destroy_stage(1), Some(0));
        assert_eq!(
            local_destroy_stage(LOCAL_DESTROY_PROGRESS_SCALE / 2),
            Some(5)
        );
        assert_eq!(
            local_destroy_stage(LOCAL_DESTROY_PROGRESS_SCALE - 1),
            Some(9)
        );
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
            world.local_player().interaction.destroying_block_stage,
            Some(9)
        );

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
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, 0);
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(world.local_block_predictions()[0].sequence, 1);
        assert_eq!(world.local_block_predictions()[0].server_block_state_id, 9);
        assert_eq!(
            world.local_block_predictions()[0].predicted_block_state_id,
            0
        );
    }

    #[test]
    fn local_destroy_target_item_signature_follows_vanilla_same_item_same_components() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 9);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        world.set_local_destroying_block_hit(pos, ProtocolDirection::North);

        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 64),
        });
        assert!(world.local_destroying_block_matches_current_item());

        let mut damaged_item = item_stack(42, 1);
        damaged_item.component_patch.damage = Some(3);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: damaged_item,
        });
        assert!(!world.local_destroying_block_matches_current_item());

        world.set_local_destroying_block_hit(pos, ProtocolDirection::North);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(43, 1),
        });
        assert!(!world.local_destroying_block_matches_current_item());
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

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn mining_profile(rules: Vec<WorldItemMiningRule>) -> WorldItemMiningProfile {
        WorldItemMiningProfile {
            default_mining_speed_thousandths: 1_000,
            rules,
        }
    }

    fn mining_rule(
        block_names: Vec<&str>,
        mining_speed_thousandths: Option<u32>,
        correct_for_drops: Option<bool>,
    ) -> WorldItemMiningRule {
        WorldItemMiningRule {
            block_names: block_names.into_iter().map(str::to_string).collect(),
            mining_speed_thousandths,
            correct_for_drops,
        }
    }

    fn block_destroy_profile(
        destroy_time_tenths: Option<u32>,
        requires_correct_tool: bool,
    ) -> WorldBlockDestroyProfile {
        WorldBlockDestroyProfile {
            destroy_time_tenths,
            requires_correct_tool,
        }
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
