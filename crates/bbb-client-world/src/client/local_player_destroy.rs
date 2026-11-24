use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_BLOCK_BREAK_SPEED_ID, VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID,
    VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID,
};
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_destroy_speed,
    vanilla_block_state_requires_correct_tool_for_drops,
};
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID,
    VANILLA_BLOCK_CAVE_AIR_ID, VANILLA_BLOCK_VOID_AIR_ID, VANILLA_GAME_MASTER_BLOCK_IDS,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID;
use bbb_protocol::item_types::VANILLA_DESTROY_BLOCK_DENIED_ITEM_IDS;
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_CONDUIT_POWER_ID, VANILLA_MOB_EFFECT_HASTE_ID,
    VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
};
use bbb_protocol::packets::{
    Direction as ProtocolDirection, ItemStackSummary, ToolRuleSummary, ToolSummary,
};

use super::local_player_fluid::local_player_eye_in_water;
use crate::{BlockPos, WorldStore};

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

impl WorldStore {
    pub fn apply_local_block_attack(
        &mut self,
        pos: BlockPos,
        sequence: i32,
    ) -> Option<LocalBlockAttackEffect> {
        let block = self.block_view(pos)?;
        let block_name = block.block_name?;
        match block_name {
            "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore" => {
                if block.block_properties.get("lit").map(String::as_str) == Some("false") {
                    let mut lit_properties = block.block_properties.clone();
                    lit_properties.insert("lit".to_string(), "true".to_string());
                    if let Some(lit_block_state_id) = self
                        .registries
                        .block_state_id_by_name_and_properties(block_name, &lit_properties)
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

    pub fn local_destroy_block_gamemaster_gate_allows(&self, pos: BlockPos) -> bool {
        let is_game_master_block = self
            .block_view(pos)
            .and_then(|block| block.block_name)
            .and_then(vanilla_block_type_for_resource_id)
            .is_some_and(|block_type| VANILLA_GAME_MASTER_BLOCK_IDS.contains(&block_type.id));
        !is_game_master_block || self.local_player_can_use_gamemaster_blocks()
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
        if self.local_selected_main_hand_can_destroy_block()
            && self.local_destroy_block_gamemaster_gate_allows(pos)
        {
            self.predict_local_destroy_block(pos, sequence);
        }
        Some(LocalDestroyBlockFinished {
            pos,
            face,
            sequence,
        })
    }

    pub fn local_destroying_block_will_finish_next_tick(&self) -> bool {
        let Some(pos) = self.local_player.interaction.destroying_block else {
            return false;
        };
        self.local_destroy_progress_per_tick(pos)
            .is_some_and(|progress| {
                progress > 0
                    && self
                        .local_player
                        .interaction
                        .destroying_block_progress
                        .saturating_add(progress)
                        >= LOCAL_DESTROY_PROGRESS_SCALE
            })
    }

    fn local_destroy_progress_per_tick(&self, pos: BlockPos) -> Option<u32> {
        let block_state_id = self.block_state_id_at(pos)?;
        let (block_type_id, requires_correct_tool, destroy_speed, block_resource_id) =
            if let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) {
                if matches!(
                    block_type_id,
                    VANILLA_BLOCK_AIR_ID | VANILLA_BLOCK_CAVE_AIR_ID | VANILLA_BLOCK_VOID_AIR_ID
                ) {
                    return None;
                }
                (
                    block_type_id,
                    vanilla_block_state_requires_correct_tool_for_drops(block_state_id)?,
                    vanilla_block_state_destroy_speed(block_state_id)?,
                    vanilla_block_type(block_type_id).map(|block| block.resource_id),
                )
            } else {
                let (spec, _) = self.content_registry.0.block_by_state_id(block_state_id)?;
                (
                    spec.block_id,
                    false,
                    spec.destroy_speed,
                    Some(spec.resource_id.as_str()),
                )
            };
        let tool = self
            .inventory
            .selected_hotbar_item(self.local_player.selected_hotbar_slot)
            .and_then(|item| self.effective_item_tool(item));
        let (base_speed, correct_for_drops) = tool.map_or((1.0, false), |(tool, bound)| {
            let block_holder_id = if self
                .frozen_registry_access()
                .has_server_contents("minecraft:block")
            {
                block_resource_id.and_then(|resource_id| {
                    self.registry_holder_id_by_resource_id("minecraft:block", resource_id)
                })
            } else {
                Some(block_type_id)
            };
            self.local_destroy_tool_properties(tool, block_holder_id, requires_correct_tool, bound)
        });
        let mining_speed = self.local_destroy_player_mining_speed_thousandths(base_speed);
        local_destroy_progress_per_tick_with_tool(
            destroy_speed,
            requires_correct_tool,
            mining_speed,
            correct_for_drops,
        )
    }

    fn local_destroy_tool_properties(
        &self,
        tool: &ToolSummary,
        block_holder_id: Option<i32>,
        requires_correct_tool: bool,
        bound: bool,
    ) -> (f64, bool) {
        let Some(holder_id) = block_holder_id else {
            return (f32::from_bits(tool.default_mining_speed_bits).into(), false);
        };
        let matches = |rule: &ToolRuleSummary| match rule.block_tag.as_deref() {
            Some(tag) if !bound => {
                self.registry_holder_is_in_tag("minecraft:block", tag, holder_id)
            }
            _ => rule.block_registry_ids.contains(&holder_id),
        };
        let speed_bits = tool
            .rules
            .iter()
            .find_map(|rule| rule.speed_bits.filter(|_| matches(rule)))
            .unwrap_or(tool.default_mining_speed_bits);
        let correct_for_drops = requires_correct_tool
            && tool
                .rules
                .iter()
                .find_map(|rule| rule.correct_for_drops.filter(|_| matches(rule)))
                .unwrap_or(false);
        (f32::from_bits(speed_bits).into(), correct_for_drops)
    }

    fn local_destroy_player_mining_speed_thousandths(&self, mut speed: f64) -> u32 {
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

    pub fn local_selected_main_hand_can_destroy_block(&self) -> bool {
        let Some(item) = self
            .inventory
            .selected_hotbar_item(self.local_player.selected_hotbar_slot)
        else {
            return true;
        };
        if item.count > 0
            && item
                .item_id
                .is_some_and(|item_id| VANILLA_DESTROY_BLOCK_DENIED_ITEM_IDS.contains(&item_id))
        {
            return false;
        }
        if !self
            .local_player
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.instabuild)
        {
            return true;
        }
        self.effective_item_tool(item)
            .is_none_or(|(tool, _)| tool.can_destroy_blocks_in_creative)
    }

    fn effective_item_tool<'a>(
        &'a self,
        item: &'a ItemStackSummary,
    ) -> Option<(&'a ToolSummary, bool)> {
        if item.count <= 0 || item.item_id.is_none() {
            return None;
        }
        let patch = &item.component_patch;
        if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)
        {
            return None;
        }
        let source =
            self.items
                .component_source(item.item_id, patch, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
        source
            .tool
            .as_ref()
            .map(|tool| (tool, !std::ptr::eq(source, &**patch)))
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

fn local_destroy_progress_per_tick_with_tool(
    destroy_speed: f32,
    requires_correct_tool: bool,
    mining_speed_thousandths: u32,
    correct_for_drops: bool,
) -> Option<u32> {
    if !destroy_speed.is_finite() || destroy_speed < 0.0 {
        return Some(0);
    }
    if destroy_speed == 0.0 {
        return Some(LOCAL_DESTROY_PROGRESS_SCALE);
    }

    let modifier = if !requires_correct_tool || correct_for_drops {
        30.0
    } else {
        100.0
    };
    let progress = (f64::from(LOCAL_DESTROY_PROGRESS_SCALE) * f64::from(mining_speed_thousandths))
        / (1000.0 * f64::from(destroy_speed) * modifier);
    if !progress.is_finite() || progress <= 0.0 {
        return Some(0);
    }
    Some(progress.ceil().min(f64::from(u32::MAX)) as u32)
}

fn local_destroy_stage(progress: u32) -> Option<u8> {
    if progress == 0 {
        return None;
    }
    Some(((progress.saturating_mul(10)) / LOCAL_DESTROY_PROGRESS_SCALE).min(9) as u8)
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
        PlayerAbilities as ProtocolPlayerAbilities,
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
    fn local_destroy_progress_uses_vanilla_hand_formula_for_known_blocks() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        for (name, progress) in [
            ("minecraft:grass_block", 55_556),
            ("minecraft:dirt", 66_667),
            ("minecraft:dirt_path", 51_283),
            ("minecraft:powder_snow", 133_334),
            ("minecraft:stone", 6_667),
            ("minecraft:redstone_wire", 1_000_000),
        ] {
            let block_type = vanilla_block_type_for_resource_id(name).unwrap();
            let state_id =
                bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type.id)
                    .unwrap();
            let world = world_with_block(pos, state_id);
            assert_eq!(
                world.local_destroy_progress_per_tick(pos),
                Some(progress),
                "{name}"
            );
        }
    }

    #[test]
    fn local_destroy_progress_uses_selected_item_tool() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        assert_eq!(
            world.probe_block(pos).unwrap().block_name.as_deref(),
            Some("minecraft:stone")
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));

        set_default_tools(
            &mut world,
            BTreeMap::from([(
                42,
                tool(vec![
                    tool_rule(vec!["minecraft:obsidian"], None, Some(false)),
                    tool_rule(vec!["minecraft:stone"], Some(4.0), Some(true)),
                ]),
            )]),
        );
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));

        set_default_tools(
            &mut world,
            BTreeMap::from([(
                42,
                tool(vec![
                    tool_rule(vec!["minecraft:stone"], None, Some(false)),
                    tool_rule(vec!["minecraft:stone"], Some(4.0), Some(true)),
                ]),
            )]),
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(26_667));
    }

    #[test]
    fn patched_tool_rules_override_defaults_and_removal_restores_bare_hand_semantics() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        let mut tags = bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "minecraft:test/tool_rule".to_string(),
                    entries: vec![1],
                }],
            }],
        };
        world.apply_update_tags(tags.clone());
        world.set_default_item_component_codec_values(BTreeMap::from([(
            42,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
                serde_json::json!({
                    "rules": [{
                        "blocks": "#minecraft:test/tool_rule",
                        "speed": 4.0,
                        "correct_for_drops": true
                    }],
                    "can_destroy_blocks_in_creative": false
                }),
            )]),
        )]));
        world.rebuild_item_component_defaults().unwrap();
        let default_tool = world.items.default_item_component_patches[&42]
            .tool
            .as_ref()
            .unwrap();
        assert_eq!(default_tool.rules[0].block_registry_ids, vec![1]);
        assert_eq!(
            default_tool.rules[0].block_tag.as_deref(),
            Some("minecraft:test/tool_rule")
        );

        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));
        world.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert!(!world.local_selected_main_hand_can_destroy_block());

        tags.registries[0].tags[0].entries.clear();
        world.apply_update_tags(tags.clone());
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
        tags.registries[0].tags[0].entries.push(1);
        world.apply_update_tags(tags);
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(88_889));

        let mut patched = item_stack(42, 1);
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
        assert!(world.local_selected_main_hand_can_destroy_block());

        patched.component_patch.tool.as_mut().unwrap().rules[0].correct_for_drops = None;
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: patched.clone(),
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(133_334));

        let mut empty_tool = patched.clone();
        empty_tool.component_patch.tool = Some(tool(Vec::new()));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: empty_tool,
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
        assert!(world.local_selected_main_hand_can_destroy_block());

        let mut empty_stack = item_stack(42, 0);
        empty_stack.component_patch = patched.component_patch.clone();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: empty_stack,
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
        assert!(world.local_selected_main_hand_can_destroy_block());

        let mut removed = patched;
        removed.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID];
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: removed,
        });
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(6_667));
        assert!(world.local_selected_main_hand_can_destroy_block());
    }

    #[test]
    fn generated_item_override_denies_debug_stick_destroy_in_every_game_mode() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 9);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(bbb_protocol::item_types::VANILLA_ITEM_DEBUG_STICK_ID, 1),
        });

        assert!(!world.local_selected_main_hand_can_destroy_block());
        world.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert!(!world.local_selected_main_hand_can_destroy_block());
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

        set_default_tools(
            &mut world,
            BTreeMap::from([(
                43,
                tool(vec![tool_rule(
                    vec!["minecraft:grass_block"],
                    Some(6.0),
                    Some(true),
                )]),
            )]),
        );
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
        set_default_tools(
            &mut world,
            BTreeMap::from([(
                42,
                tool(vec![tool_rule(
                    vec!["minecraft:stone"],
                    Some(4.0),
                    Some(true),
                )]),
            )]),
        );
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

        set_default_tools(
            &mut world,
            BTreeMap::from([(
                42,
                tool(vec![tool_rule(
                    vec!["minecraft:stone"],
                    Some(1.0004),
                    Some(true),
                )]),
            )]),
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(66_667));
    }

    #[test]
    fn local_destroy_progress_does_not_infer_efficiency_without_synced_attribute() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 1);
        attach_local_player_entity(&mut world, 123);
        register_test_enchantments(&mut world);
        set_default_tools(
            &mut world,
            BTreeMap::from([(
                42,
                tool(vec![tool_rule(
                    vec!["minecraft:stone"],
                    Some(4.0),
                    Some(true),
                )]),
            )]),
        );
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
    fn local_destroy_progress_uses_builtin_block_destroy_facts() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 5307);
        assert_eq!(
            world.probe_block(pos).unwrap().block_name.as_deref(),
            Some("minecraft:diamond_ore")
        );
        assert_eq!(world.local_destroy_progress_per_tick(pos), Some(3_334));

        set_default_tools(
            &mut world,
            BTreeMap::from([(
                42,
                tool(vec![tool_rule(
                    vec!["minecraft:diamond_ore"],
                    Some(8.0),
                    Some(true),
                )]),
            )]),
        );
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
    fn local_destroy_game_master_gate_uses_generated_block_membership_and_permission() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let command_block_type = vanilla_block_type_for_resource_id("minecraft:command_block")
            .expect("command block type");
        let command_block_state =
            bbb_protocol::block_state_types::vanilla_block_default_state_id(command_block_type.id)
                .expect("command block default state");
        let mut world = world_with_block(pos, command_block_state);

        assert!(!world.local_destroy_block_gamemaster_gate_allows(pos));
        assert!(world.local_destroy_block_gamemaster_gate_allows(BlockPos {
            x: pos.x,
            y: pos.y + 1,
            z: pos.z,
        }));

        world.local_player_id = Some(7);
        world.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert!(
            world.apply_entity_event(bbb_protocol::packets::EntityEvent {
                entity_id: 7,
                event_id: 26,
            })
        );
        assert!(world.local_destroy_block_gamemaster_gate_allows(pos));
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
        assert!(!world.local_destroying_block_will_finish_next_tick());

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
        assert!(world.local_destroying_block_will_finish_next_tick());

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
        assert!(!world.local_destroying_block_will_finish_next_tick());
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
    fn debug_stick_completion_sends_finish_without_local_air_prediction() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 9);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(bbb_protocol::item_types::VANILLA_ITEM_DEBUG_STICK_ID, 1),
        });
        world.set_local_destroying_block_hit(pos, ProtocolDirection::North);

        for _ in 0..17 {
            assert_eq!(world.advance_local_destroying_block_tick(), None);
        }
        assert_eq!(
            world.advance_local_destroying_block_tick(),
            Some(LocalDestroyBlockFinished {
                pos,
                face: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, 9);
        assert!(world.local_block_predictions().is_empty());
    }

    #[test]
    fn local_destroy_target_item_signature_follows_vanilla_same_item_same_components() {
        let pos = BlockPos { x: 0, y: 1, z: 3 };
        let mut world = world_with_block(pos, 9);
        world.set_local_destroying_block_hit(pos, ProtocolDirection::North);
        let mut zero_count_item = item_stack(42, 0);
        zero_count_item.component_patch.damage = Some(3);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: zero_count_item.clone(),
        });
        assert!(!world.local_destroying_block_matches_current_item());
        world.set_local_destroying_block_hit(pos, ProtocolDirection::North);
        zero_count_item.count = 1;
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: zero_count_item,
        });
        assert!(world.local_destroying_block_matches_current_item());

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
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })],
            block_entities: std::sync::Arc::default(),
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
            }
            .into(),
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

    fn set_default_tools(world: &mut WorldStore, tools: BTreeMap<i32, ToolSummary>) {
        world.items.commit_item_component_defaults(
            tools
                .into_iter()
                .map(|(item_id, tool)| {
                    (
                        item_id,
                        ProtocolDataComponentPatchSummary {
                            added_type_ids: vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID],
                            tool: Some(tool),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
        );
    }

    fn tool(rules: Vec<ToolRuleSummary>) -> ToolSummary {
        ToolSummary {
            default_mining_speed_bits: 1.0_f32.to_bits(),
            rules,
            damage_per_block: 1,
            can_destroy_blocks_in_creative: true,
        }
    }

    fn tool_rule(
        block_names: Vec<&str>,
        mining_speed: Option<f32>,
        correct_for_drops: Option<bool>,
    ) -> ToolRuleSummary {
        ToolRuleSummary {
            block_registry_ids: block_names
                .into_iter()
                .map(|name| vanilla_block_type_for_resource_id(name).unwrap().id)
                .collect(),
            block_tag: None,
            speed_bits: mining_speed.map(f32::to_bits),
            correct_for_drops,
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
