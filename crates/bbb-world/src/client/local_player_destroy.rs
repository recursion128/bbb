use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_BLOCK_BREAK_SPEED_ID, VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID,
    VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID;
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_CONDUIT_POWER_ID, VANILLA_MOB_EFFECT_HASTE_ID,
    VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
};
use bbb_protocol::packets::{
    Direction as ProtocolDirection, ItemStackSummary, ToolRuleSummary, ToolSummary,
};
use serde::{Deserialize, Serialize};

use super::local_player_fluid::local_player_eye_in_water;
use crate::{
    block_name_is_air, block_name_is_leaves, block_name_is_log, block_name_is_planks, BlockPos,
    WorldStore,
};

const LOCAL_DESTROY_PROGRESS_SCALE: u32 = 1_000_000;
pub(crate) const LOCAL_DESTROY_COMPLETION_DELAY_TICKS: u8 = 5;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDestroyBlockFinished {
    pub pos: BlockPos,
    pub face: ProtocolDirection,
    pub sequence: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalBlockAttackEffect {
    RedstoneOre { pos: BlockPos },
    DragonEgg { pos: BlockPos },
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
    #[serde(default = "default_tool_can_destroy_blocks_in_creative")]
    pub can_destroy_blocks_in_creative: bool,
}

const fn default_tool_can_destroy_blocks_in_creative() -> bool {
    true
}

#[derive(Debug, Clone, Copy)]
enum EffectiveItemTool<'a> {
    Patched(&'a ToolSummary),
    Default(&'a WorldItemMiningProfile),
    Absent,
}

impl WorldStore {
    pub fn apply_local_block_attack(
        &mut self,
        pos: BlockPos,
        sequence: i32,
    ) -> Option<LocalBlockAttackEffect> {
        let block = self.probe_block(pos)?;
        let block_name = block.block_name?;
        match block_name.as_str() {
            "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore" => {
                if block.block_properties.get("lit").map(String::as_str) == Some("false") {
                    let mut lit_properties = block.block_properties;
                    lit_properties.insert("lit".to_string(), "true".to_string());
                    if let Some(lit_block_state_id) = self
                        .registries
                        .block_state_id_by_name_and_properties(&block_name, &lit_properties)
                    {
                        self.predict_local_block_state(pos, lit_block_state_id, sequence);
                    }
                }
                Some(LocalBlockAttackEffect::RedstoneOre { pos })
            }
            "minecraft:dragon_egg" => Some(LocalBlockAttackEffect::DragonEgg { pos }),
            // NoteBlock.attack is server-only; every other Vanilla block uses
            // BlockBehaviour's empty implementation.
            _ => None,
        }
    }

    pub fn set_default_block_destroy_profiles(
        &mut self,
        profiles: std::collections::BTreeMap<String, WorldBlockDestroyProfile>,
    ) {
        self.items.default_block_destroy_profiles = profiles
            .into_iter()
            .filter(|(block_name, _)| !block_name.is_empty())
            .collect::<std::collections::BTreeMap<_, _>>()
            .into();
    }

    pub fn set_default_item_mining_profiles(
        &mut self,
        profiles: std::collections::BTreeMap<i32, WorldItemMiningProfile>,
    ) {
        self.items.default_item_mining_profiles = profiles
            .into_iter()
            .filter(|(item_id, profile)| {
                *item_id >= 0 && profile.default_mining_speed_thousandths > 0
            })
            .collect::<std::collections::BTreeMap<_, _>>()
            .into();
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

        let progress = {
            let interaction = &mut self.local_player.interaction;
            interaction.destroying_block_progress = interaction
                .destroying_block_progress
                .saturating_add(progress_delta);
            interaction.destroying_block_ticks =
                interaction.destroying_block_ticks.saturating_add(1);
            interaction
                .destroying_block_progress
                .min(LOCAL_DESTROY_PROGRESS_SCALE)
        };
        self.tutorial_on_destroy_block(
            pos,
            i32::try_from(progress).expect("destroy progress scale fits i32"),
        );
        let interaction = &mut self.local_player.interaction;
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
        let block_name = block.block_name.as_deref()?;
        if block_name_is_air(block_name) {
            return None;
        }
        let hotbar_items = self.inventory.hotbar_items();
        let selected_slot = usize::from(self.local_player.selected_hotbar_slot.min(8));
        let item = &hotbar_items[selected_slot];
        let mining_speed = self.local_destroy_player_mining_speed_thousandths(
            self.local_destroy_item_mining_speed_thousandths(block_name, item),
        );
        let correct_for_drops = self.local_destroy_item_is_correct_for_drops(block_name, item);
        let block_profile = self.local_block_destroy_profile(block_name)?;
        local_destroy_progress_per_tick_with_tool(block_profile, mining_speed, correct_for_drops)
    }

    fn local_destroy_item_mining_speed_thousandths(
        &self,
        block_name: &str,
        item: &ItemStackSummary,
    ) -> u32 {
        match self.effective_item_tool(item) {
            EffectiveItemTool::Patched(tool) => tool
                .rules
                .iter()
                .find_map(|rule| {
                    rule.speed_bits
                        .filter(|_| self.tool_rule_matches(rule, block_name))
                })
                .map(|bits| mining_speed_thousandths_from_float(f32::from_bits(bits).into()))
                .unwrap_or_else(|| {
                    mining_speed_thousandths_from_float(
                        f32::from_bits(tool.default_mining_speed_bits).into(),
                    )
                }),
            EffectiveItemTool::Default(profile) => {
                item_mining_speed_thousandths(profile, block_name)
            }
            EffectiveItemTool::Absent => 1_000,
        }
    }

    fn local_destroy_player_mining_speed_thousandths(&self, base_speed_thousandths: u32) -> u32 {
        let mut speed = f64::from(base_speed_thousandths) / 1_000.0;
        if speed > 1.0 {
            speed += self
                .local_player_attribute_value(VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID)
                .unwrap_or(0.0);
        }
        if let Some(amplifier) = self.local_player_dig_speed_amplifier() {
            speed *= 1.0 + f64::from(amplifier.saturating_add(1)) * 0.2;
        }
        if let Some(fatigue) =
            self.local_player_mob_effect_amplifier(VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
        {
            speed *= mining_fatigue_scale(fatigue);
        }
        speed *= self
            .local_player_attribute_value(VANILLA_ATTRIBUTE_BLOCK_BREAK_SPEED_ID)
            .unwrap_or(1.0);
        if local_player_eye_in_water(self) {
            speed *= self
                .local_player_attribute_value(VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID)
                .unwrap_or(0.2);
        }
        if self.local_player().pose.is_some_and(|pose| !pose.on_ground) {
            speed /= 5.0;
        }

        mining_speed_thousandths_from_float(speed)
    }

    fn local_destroy_item_is_correct_for_drops(
        &self,
        block_name: &str,
        item: &ItemStackSummary,
    ) -> bool {
        match self.effective_item_tool(item) {
            EffectiveItemTool::Patched(tool) => tool.rules.iter().find_map(|rule| {
                rule.correct_for_drops
                    .filter(|_| self.tool_rule_matches(rule, block_name))
            }),
            EffectiveItemTool::Default(profile) => {
                Some(item_is_correct_for_drops(profile, block_name))
            }
            EffectiveItemTool::Absent => None,
        }
        .unwrap_or(false)
    }

    pub fn local_selected_main_hand_can_destroy_block_in_creative(&self) -> bool {
        let hotbar_items = self.inventory.hotbar_items();
        let selected_slot = usize::from(self.local_player.selected_hotbar_slot.min(8));
        match self.effective_item_tool(&hotbar_items[selected_slot]) {
            EffectiveItemTool::Patched(tool) => tool.can_destroy_blocks_in_creative,
            EffectiveItemTool::Default(profile) => profile.can_destroy_blocks_in_creative,
            EffectiveItemTool::Absent => true,
        }
    }

    fn effective_item_tool<'a>(&'a self, item: &'a ItemStackSummary) -> EffectiveItemTool<'a> {
        let Some(item_id) = item.item_id.filter(|_| item.count > 0) else {
            return EffectiveItemTool::Absent;
        };
        let patch = &item.component_patch;
        if patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)
        {
            return patch
                .tool
                .as_ref()
                .map_or(EffectiveItemTool::Absent, EffectiveItemTool::Patched);
        }
        if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)
        {
            return EffectiveItemTool::Absent;
        }
        self.items
            .default_item_mining_profiles
            .get(&item_id)
            .map_or(EffectiveItemTool::Absent, EffectiveItemTool::Default)
    }

    fn tool_rule_matches(&self, rule: &ToolRuleSummary, block_name: &str) -> bool {
        let Some(holder_id) = self.registry_holder_id_by_resource_id("minecraft:block", block_name)
        else {
            return false;
        };
        match (
            rule.block_registry_ids.as_slice(),
            rule.block_tag.as_deref(),
        ) {
            (ids, None) => ids.contains(&holder_id),
            ([], Some(tag)) => self
                .registry_holder_ids_for_tag("minecraft:block", tag)
                .is_some_and(|ids| ids.contains(&holder_id)),
            (_, Some(_)) => false,
        }
    }

    fn local_block_destroy_profile(&self, block_name: &str) -> Option<LocalBlockDestroyProfile> {
        self.items
            .default_block_destroy_profiles
            .get(block_name)
            .map(LocalBlockDestroyProfile::from)
            .or_else(|| fallback_local_block_destroy_profile(block_name))
    }

    fn local_player_mob_effect_amplifier(&self, effect_id: i32) -> Option<i32> {
        self.local_player_id
            .and_then(|id| self.entity_effect(id, effect_id))
            .map(|effect| effect.amplifier)
    }

    fn local_player_dig_speed_amplifier(&self) -> Option<i32> {
        [
            VANILLA_MOB_EFFECT_HASTE_ID,
            VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        ]
        .into_iter()
        .filter_map(|effect_id| self.local_player_mob_effect_amplifier(effect_id))
        .max()
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
        name if block_name_is_log(name) || block_name_is_planks(name) => Some(profile(20, false)),
        name if block_name_is_leaves(name) => Some(profile(2, false)),
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

fn mining_fatigue_scale(amplifier: i32) -> f64 {
    match amplifier {
        i32::MIN..=0 => 0.3,
        1 => 0.09,
        2 => 0.0027,
        _ => 0.00081,
    }
}

fn mining_speed_thousandths_from_float(speed: f64) -> u32 {
    if !speed.is_finite() || speed <= 0.0 {
        return 0;
    }
    (speed * 1_000.0).round().min(f64::from(u32::MAX)) as u32
}

fn ceil_div_u64(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    numerator / denominator + u64::from(numerator % denominator != 0)
}

#[cfg(test)]
mod tests {
    use super::super::local_player::LocalPlayerPoseState;
    use super::*;
    use bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_HEAD_SLOT;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AttributeSnapshot as ProtocolAttributeSnapshot,
        BlockPos as ProtocolBlockPos, BlockUpdate,
        DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
        Direction as ProtocolDirection, ItemEnchantmentSummary as ProtocolItemEnchantmentSummary,
        ItemStackSummary as ProtocolItemStackSummary, MobEffectFlags as ProtocolMobEffectFlags,
        SetPlayerInventory as ProtocolSetPlayerInventory,
        UpdateAttributes as ProtocolUpdateAttributes, UpdateMobEffect as ProtocolUpdateMobEffect,
        Vec3d as ProtocolVec3d,
    };
    use std::collections::BTreeMap;
    use uuid::Uuid;

    use crate::{
        entities::VANILLA_ENTITY_TYPE_PLAYER_ID, registries::RegistryPacketEntry, ChunkColumn,
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID: i32 = 89;

    #[test]
    fn local_block_attack_predicts_both_redstone_ore_families_lit() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        for block_name in ["minecraft:redstone_ore", "minecraft:deepslate_redstone_ore"] {
            let unlit = block_state_id(block_name, &[("lit", "false")]);
            let lit = block_state_id(block_name, &[("lit", "true")]);
            let mut world = world_with_block(pos, unlit);

            assert_eq!(
                world.apply_local_block_attack(pos, 7),
                Some(LocalBlockAttackEffect::RedstoneOre { pos })
            );
            assert_eq!(world.probe_block(pos).unwrap().block_state_id, lit);
            assert_eq!(world.local_block_predictions().len(), 1);
            assert_eq!(world.local_block_predictions()[0].sequence, 7);
            assert_eq!(
                world.local_block_predictions()[0].server_block_state_id,
                unlit
            );
            assert_eq!(
                world.local_block_predictions()[0].predicted_block_state_id,
                lit
            );

            world.apply_block_changed_ack(bbb_protocol::packets::BlockChangedAck { sequence: 7 });
            assert_eq!(world.probe_block(pos).unwrap().block_state_id, unlit);
        }
    }

    #[test]
    fn local_block_attack_preserves_lit_redstone_ore_while_requesting_particles() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let lit = block_state_id("minecraft:redstone_ore", &[("lit", "true")]);
        let mut world = world_with_block(pos, lit);

        assert_eq!(
            world.apply_local_block_attack(pos, 7),
            Some(LocalBlockAttackEffect::RedstoneOre { pos })
        );
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, lit);
        assert!(world.local_block_predictions().is_empty());
    }

    #[test]
    fn local_block_attack_dispatches_only_client_observable_overrides() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let dragon_egg = block_state_id("minecraft:dragon_egg", &[]);
        let mut dragon_world = world_with_block(pos, dragon_egg);
        assert_eq!(
            dragon_world.apply_local_block_attack(pos, 7),
            Some(LocalBlockAttackEffect::DragonEgg { pos })
        );
        assert_eq!(
            dragon_world.probe_block(pos).unwrap().block_state_id,
            dragon_egg
        );
        assert!(dragon_world.local_block_predictions().is_empty());

        let note_block = block_state_id(
            "minecraft:note_block",
            &[("instrument", "harp"), ("note", "0"), ("powered", "false")],
        );
        let mut note_world = world_with_block(pos, note_block);
        assert_eq!(note_world.apply_local_block_attack(pos, 7), None);
        assert_eq!(
            note_world.probe_block(pos).unwrap().block_state_id,
            note_block
        );
        assert!(note_world.local_block_predictions().is_empty());

        let mut stone_world = world_with_block(pos, 1);
        assert_eq!(stone_world.apply_local_block_attack(pos, 7), None);
        assert_eq!(stone_world.probe_block(pos).unwrap().block_state_id, 1);
        assert!(stone_world.local_block_predictions().is_empty());
    }

    #[test]
    fn fallback_destroy_profile_uses_exact_leaves_membership() {
        let leaves = fallback_local_block_destroy_profile("minecraft:oak_leaves")
            .expect("generated vanilla leaves member");
        assert_eq!(leaves.destroy_time_tenths, Some(2));
        assert!(!leaves.requires_correct_tool);
        assert!(fallback_local_block_destroy_profile("minecraft:custom_leaves").is_none());
        assert!(fallback_local_block_destroy_profile("example:oak_leaves").is_none());
    }

    #[test]
    fn fallback_destroy_profile_uses_exact_log_and_plank_memberships() {
        for block_name in [
            "minecraft:oak_log",
            "minecraft:stripped_mangrove_wood",
            "minecraft:crimson_stem",
            "minecraft:warped_hyphae",
            "minecraft:bamboo_planks",
        ] {
            let profile = fallback_local_block_destroy_profile(block_name)
                .expect("generated vanilla log/plank member");
            assert_eq!(profile.destroy_time_tenths, Some(20), "{block_name}");
            assert!(!profile.requires_correct_tool, "{block_name}");
        }
        for block_name in [
            "minecraft:custom_log",
            "minecraft:custom_wood",
            "minecraft:custom_stem",
            "minecraft:custom_hyphae",
            "minecraft:custom_planks",
            "example:oak_log",
        ] {
            assert!(
                fallback_local_block_destroy_profile(block_name).is_none(),
                "{block_name}"
            );
        }
    }

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
    fn patched_tool_rules_override_defaults_and_removal_restores_bare_hand_semantics() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        let mut default_profile = mining_profile(vec![mining_rule(
            vec!["minecraft:stone"],
            Some(4_000),
            Some(true),
        )]);
        default_profile.can_destroy_blocks_in_creative = false;
        world.set_default_item_mining_profiles(BTreeMap::from([(42, default_profile)]));
        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "minecraft:test/tool_rule".to_string(),
                    entries: vec![1],
                }],
            }],
        });

        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));
        assert!(!world.local_selected_main_hand_can_destroy_block_in_creative());

        let mut patched = item_stack(42, 1);
        patched.component_patch.added = 1;
        patched.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID];
        patched.component_patch.tool = Some(ToolSummary {
            rules: vec![
                ToolRuleSummary {
                    block_registry_ids: vec![1],
                    block_tag: None,
                    speed_bits: None,
                    correct_for_drops: Some(false),
                },
                ToolRuleSummary {
                    block_registry_ids: Vec::new(),
                    block_tag: Some("minecraft:test/tool_rule".to_string()),
                    speed_bits: Some(6.0_f32.to_bits()),
                    correct_for_drops: Some(true),
                },
            ],
            default_mining_speed_bits: 0.5_f32.to_bits(),
            damage_per_block: 2,
            can_destroy_blocks_in_creative: true,
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: patched.clone(),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(40_000));
        assert!(world.local_selected_main_hand_can_destroy_block_in_creative());

        patched.component_patch.tool.as_mut().unwrap().rules[0].correct_for_drops = None;
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: patched,
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(133_334));

        let mut removed = item_stack(42, 1);
        removed.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID];
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: removed,
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
        assert!(world.local_selected_main_hand_can_destroy_block_in_creative());
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
    fn local_destroy_progress_applies_dig_speed_mob_effects() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));

        assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_HASTE_ID, 1,)));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(9_334));

        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
            2,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(10_667));
    }

    #[test]
    fn local_destroy_progress_applies_mining_fatigue() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);

        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
            0,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(2_000));

        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
            1,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(600));
    }

    #[test]
    fn local_destroy_progress_applies_synced_mining_attributes() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);
        world.set_default_item_mining_profiles(BTreeMap::from([(
            42,
            mining_profile(vec![mining_rule(
                vec!["minecraft:stone"],
                Some(4_000),
                Some(true),
            )]),
        )]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));

        assert!(world.apply_update_attributes(attribute_update(
            123,
            VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID,
            5.0,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(200_000));

        assert!(world.apply_update_attributes(attribute_update(
            123,
            VANILLA_ATTRIBUTE_BLOCK_BREAK_SPEED_ID,
            0.5,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(100_000));
    }

    #[test]
    fn local_destroy_progress_does_not_infer_efficiency_without_synced_attribute() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);
        register_test_enchantments(&mut world);
        world.set_default_item_mining_profiles(BTreeMap::from([(
            42,
            mining_profile(vec![mining_rule(
                vec!["minecraft:stone"],
                Some(4_000),
                Some(true),
            )]),
        )]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: enchanted_item_stack(42, 1, vec![(1, 3)]),
        });

        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));

        assert!(world.apply_update_attributes(attribute_update(
            123,
            VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID,
            10.0,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(311_112));
    }

    #[test]
    fn local_destroy_progress_applies_airborne_slowdown() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(1_334));
    }

    #[test]
    fn local_destroy_progress_applies_submerged_mining_speed() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let eye_pos = BlockPos { x: 0, y: 1, z: 0 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 0.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        set_block(&mut world, eye_pos, SOURCE_WATER_BLOCK_STATE_ID);

        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(1_334));

        assert!(world.apply_update_attributes(attribute_update(
            123,
            VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID,
            1.0,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
    }

    #[test]
    fn local_destroy_progress_does_not_apply_submerged_speed_below_low_water_surface() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let eye_pos = BlockPos { x: 0, y: 1, z: 0 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 0.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        set_block(&mut world, eye_pos, FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID);

        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
    }

    #[test]
    fn local_destroy_progress_does_not_infer_aqua_affinity_without_synced_attribute() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let eye_pos = BlockPos { x: 0, y: 1, z: 0 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);
        register_test_enchantments(&mut world);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 0.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        set_block(&mut world, eye_pos, SOURCE_WATER_BLOCK_STATE_ID);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
            item: enchanted_item_stack(77, 1, vec![(0, 1)]),
        });

        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(1_334));

        assert!(world.apply_update_attributes(attribute_update(
            123,
            VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID,
            1.0,
        )));
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
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
        set_block(&mut world, pos, block_state_id);
        world
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
    }

    fn attach_local_player_entity(world: &mut WorldStore, id: i32) {
        world.local_player_id = Some(id);
        world.apply_add_entity(ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn attribute_update(entity_id: i32, attribute_id: i32, base: f64) -> ProtocolUpdateAttributes {
        ProtocolUpdateAttributes {
            entity_id,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id,
                base,
                modifiers: Vec::new(),
            }],
        }
    }

    fn mob_effect(entity_id: i32, effect_id: i32, amplifier: i32) -> ProtocolUpdateMobEffect {
        ProtocolUpdateMobEffect {
            entity_id,
            effect_id,
            amplifier,
            duration_ticks: 200,
            flags: ProtocolMobEffectFlags::default(),
        }
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn enchanted_item_stack(
        item_id: i32,
        count: i32,
        enchantments: Vec<(i32, i32)>,
    ) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: ProtocolDataComponentPatchSummary {
                enchantments: enchantments
                    .into_iter()
                    .map(|(holder_id, level)| ProtocolItemEnchantmentSummary { holder_id, level })
                    .collect(),
                ..ProtocolDataComponentPatchSummary::default()
            },
        }
    }

    fn register_test_enchantments(world: &mut WorldStore) {
        world.record_registry_entries(
            "minecraft:enchantment",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:aqua_affinity"),
                RegistryPacketEntry::stub("minecraft:efficiency"),
            ],
        );
    }

    fn mining_profile(rules: Vec<WorldItemMiningRule>) -> WorldItemMiningProfile {
        WorldItemMiningProfile {
            default_mining_speed_thousandths: 1_000,
            rules,
            can_destroy_blocks_in_creative: true,
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

    fn block_state_id(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties = properties
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        crate::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("missing Vanilla block state {name} {properties:?}"))
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
