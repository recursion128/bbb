use super::{
    redstone_wire::wire_connected_sides, LocalBlockUseContext, LocalInteractionParticleAction,
    LocalInteractionResult, LocalInteractionSoundAction, LocalInteractionSuccess,
    LocalLeverAttachFace, LocalTargetUsePrediction,
};
#[cfg(test)]
use crate::BlockPos;
use crate::{BlockView, LocalPlayerHealthState, WorldStore};
use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
        VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID, VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
        VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
    },
    block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_cycle_property,
        vanilla_block_state_with_property,
    },
    block_types::{
        BlockTypeCatalogEntry, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_AIR_ID,
        VANILLA_BLOCK_CAKE_ID, VANILLA_BLOCK_FLOWER_POT_ID, VANILLA_BLOCK_IRON_DOOR_ID,
        VANILLA_BLOCK_IRON_TRAPDOOR_ID, VANILLA_BLOCK_TYPES,
    },
    item_types::VANILLA_ITEM_TYPES,
    packets::{
        DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
        Direction as ProtocolDirection, InteractionHand,
        ItemStackSummary as ProtocolItemStackSummary,
    },
};
use std::collections::BTreeMap;

impl WorldStore {
    pub(super) fn predict_local_block_use_without_item(
        &mut self,
        owner: VanillaBlockUseWithoutItemOverride,
        block_type: &BlockTypeCatalogEntry,
        block: &BlockView,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        match owner {
            VanillaBlockUseWithoutItemOverride::AbstractFurnaceBlock
            | VanillaBlockUseWithoutItemOverride::AnvilBlock
            | VanillaBlockUseWithoutItemOverride::BarrelBlock
            | VanillaBlockUseWithoutItemOverride::BeaconBlock
            | VanillaBlockUseWithoutItemOverride::BrewingStandBlock
            | VanillaBlockUseWithoutItemOverride::CartographyTableBlock
            | VanillaBlockUseWithoutItemOverride::ChestBlock
            | VanillaBlockUseWithoutItemOverride::CrafterBlock
            | VanillaBlockUseWithoutItemOverride::CraftingTableBlock
            | VanillaBlockUseWithoutItemOverride::DispenserBlock
            | VanillaBlockUseWithoutItemOverride::EnchantingTableBlock
            | VanillaBlockUseWithoutItemOverride::EnderChestBlock
            | VanillaBlockUseWithoutItemOverride::GrindstoneBlock
            | VanillaBlockUseWithoutItemOverride::HopperBlock
            | VanillaBlockUseWithoutItemOverride::LoomBlock
            | VanillaBlockUseWithoutItemOverride::ShulkerBoxBlock
            | VanillaBlockUseWithoutItemOverride::SmithingTableBlock
            | VanillaBlockUseWithoutItemOverride::StonecutterBlock => {
                // These owners open or request a server-owned menu, but their
                // client branch unconditionally returns `SUCCESS`.
                success(LocalInteractionSuccess::client_item_interaction())
            }
            VanillaBlockUseWithoutItemOverride::BedBlock => {
                // `BedBlock` returns `SUCCESS_SERVER` immediately on a
                // `ClientLevel`; sleep, explosions, occupancy, and messages
                // remain authoritative server behavior.
                success(LocalInteractionSuccess::server_item_interaction())
            }
            VanillaBlockUseWithoutItemOverride::BellBlock => {
                match bell_hit_is_proper(block.block_properties, context.face, context.cursor[1]) {
                    Some(true) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(false) => pass(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::ButtonBlock => {
                let Some(powered) = boolean_property(block.block_properties, "powered") else {
                    return unsupported();
                };
                if powered {
                    return success(LocalInteractionSuccess::consume(false));
                }
                let sound_action = LocalInteractionSoundAction::BlockPositioned {
                    pos: context.pos,
                    sound_resource_id: button_click_on_sound_id(block_type.resource_id),
                    volume_bits: 1.0_f32.to_bits(),
                    pitch_bits: 1.0_f32.to_bits(),
                };
                self.predict_local_block_state_interaction_with_sound(
                    context,
                    vanilla_block_state_with_property(block.block_state_id, "powered", "true"),
                    owner,
                    Some(sound_action),
                )
            }
            VanillaBlockUseWithoutItemOverride::LeverBlock => {
                let Some(powered) = boolean_property(block.block_properties, "powered") else {
                    return unsupported();
                };
                let particle_action = if powered {
                    None
                } else {
                    let Some(facing) =
                        horizontal_direction_property(block.block_properties, "facing")
                    else {
                        return unsupported();
                    };
                    let Some(face) = lever_attach_face(block.block_properties) else {
                        return unsupported();
                    };
                    Some(LocalInteractionParticleAction::LeverActivation {
                        pos: context.pos,
                        facing,
                        face,
                    })
                };
                success(LocalInteractionSuccess {
                    particle_action,
                    ..LocalInteractionSuccess::client_item_interaction()
                })
            }
            VanillaBlockUseWithoutItemOverride::DoorBlock
            | VanillaBlockUseWithoutItemOverride::TrapDoorBlock => {
                if matches!(
                    block_type.id,
                    VANILLA_BLOCK_IRON_DOOR_ID | VANILLA_BLOCK_IRON_TRAPDOOR_ID
                ) {
                    return pass();
                }
                let will_open = !boolean_property(block.block_properties, "open").unwrap_or(false);
                let is_trapdoor =
                    matches!(owner, VanillaBlockUseWithoutItemOverride::TrapDoorBlock);
                let sound_action = LocalInteractionSoundAction::BlockToggleSound {
                    pos: context.pos,
                    sound_resource_id: door_or_trapdoor_toggle_sound_id(
                        block_type.resource_id,
                        is_trapdoor,
                        will_open,
                    ),
                };
                self.predict_local_block_state_interaction_with_sound(
                    context,
                    vanilla_block_state_cycle_property(block.block_state_id, "open"),
                    owner,
                    Some(sound_action),
                )
            }
            VanillaBlockUseWithoutItemOverride::FenceGateBlock => {
                let Some(open) = boolean_property(block.block_properties, "open") else {
                    return unsupported();
                };
                let mut state_id = block.block_state_id;
                if !open {
                    let Some(player_direction) = self.local_player_horizontal_direction() else {
                        return unsupported();
                    };
                    let Some(facing) =
                        horizontal_direction_property(block.block_properties, "facing")
                    else {
                        return unsupported();
                    };
                    if facing == opposite_horizontal(player_direction) {
                        let Some(updated) = vanilla_block_state_with_property(
                            state_id,
                            "facing",
                            direction_name(player_direction),
                        ) else {
                            return unsupported();
                        };
                        state_id = updated;
                    }
                }
                let sound_action = LocalInteractionSoundAction::BlockToggleSound {
                    pos: context.pos,
                    sound_resource_id: fence_gate_toggle_sound_id(block_type.resource_id, !open),
                };
                self.predict_local_block_state_interaction_with_sound(
                    context,
                    vanilla_block_state_cycle_property(state_id, "open"),
                    owner,
                    Some(sound_action),
                )
            }
            VanillaBlockUseWithoutItemOverride::RepeaterBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                self.predict_local_block_state_interaction(
                    context,
                    vanilla_block_state_cycle_property(block.block_state_id, "delay"),
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::CaveVinesBlock
            | VanillaBlockUseWithoutItemOverride::CaveVinesPlantBlock => {
                match boolean_property(block.block_properties, "berries") {
                    Some(true) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(false) => pass(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::CakeBlock
            | VanillaBlockUseWithoutItemOverride::CandleCakeBlock => {
                self.predict_local_cake_eat(context, block, owner)
            }
            VanillaBlockUseWithoutItemOverride::ComposterBlock => {
                match integer_property(block.block_properties, "level", 0..=8) {
                    Some(8) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(_) => pass(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::ChiseledBookShelfBlock => {
                if self.block_entity_type_id_at(context.pos)
                    != Some(VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID)
                {
                    return pass();
                }
                let Some(facing) = horizontal_direction_property(block.block_properties, "facing")
                else {
                    return unsupported();
                };
                let Some(slot) = chiseled_bookshelf_hit_slot(facing, context.face, context.cursor)
                else {
                    return pass();
                };
                let property = format!("slot_{slot}_occupied");
                match boolean_property(block.block_properties, &property) {
                    Some(true) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(false) => success(LocalInteractionSuccess::consume(false)),
                    None => unsupported(),
                }
            }
            owner @ (VanillaBlockUseWithoutItemOverride::CommandBlock
            | VanillaBlockUseWithoutItemOverride::JigsawBlock
            | VanillaBlockUseWithoutItemOverride::StructureBlock
            | VanillaBlockUseWithoutItemOverride::TestBlock
            | VanillaBlockUseWithoutItemOverride::TestInstanceBlock) => {
                let expected_type_id = match owner {
                    VanillaBlockUseWithoutItemOverride::CommandBlock => {
                        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID
                    }
                    VanillaBlockUseWithoutItemOverride::JigsawBlock => {
                        VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID
                    }
                    VanillaBlockUseWithoutItemOverride::StructureBlock => {
                        VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID
                    }
                    VanillaBlockUseWithoutItemOverride::TestBlock => {
                        VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID
                    }
                    VanillaBlockUseWithoutItemOverride::TestInstanceBlock => {
                        VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID
                    }
                    _ => unreachable!("owner is restricted by the enclosing pattern"),
                };
                if self.block_entity_type_id_at(context.pos) == Some(expected_type_id)
                    && self.local_player_can_use_gamemaster_blocks()
                {
                    success(LocalInteractionSuccess::client_item_interaction())
                } else {
                    pass()
                }
            }
            VanillaBlockUseWithoutItemOverride::DaylightDetectorBlock => {
                if self.local_player_may_build() {
                    // `DaylightDetectorBlock.useWithoutItem` changes the state
                    // only on the logical server, but still returns client
                    // `SUCCESS` and therefore swings immediately.
                    success(LocalInteractionSuccess::client_item_interaction())
                } else {
                    pass()
                }
            }
            VanillaBlockUseWithoutItemOverride::DragonEggBlock => {
                // The random teleport is synchronized by the resulting block
                // updates; the client still consumes and swings immediately.
                success(LocalInteractionSuccess::client_item_interaction())
            }
            VanillaBlockUseWithoutItemOverride::DecoratedPotBlock => {
                if self.block_entity_type_id_at(context.pos)
                    == Some(VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID)
                {
                    // Pot wobble is sent as a block event by the server.
                    success(LocalInteractionSuccess::client_item_interaction())
                } else {
                    pass()
                }
            }
            VanillaBlockUseWithoutItemOverride::FenceBlock
            | VanillaBlockUseWithoutItemOverride::MovingPistonBlock => {
                // Fence leash binding and stale moving-piston removal execute
                // only on the logical server. Their `ClientLevel` result is
                // exactly `PASS`.
                pass()
            }
            VanillaBlockUseWithoutItemOverride::FlowerPotBlock => {
                if block_type.resource_id == "minecraft:flower_pot" {
                    return success(LocalInteractionSuccess::consume(false));
                }
                let Some(item_id) = potted_flower_item_id(block_type) else {
                    return unsupported();
                };
                let Some(state_id) = vanilla_block_default_state_id(VANILLA_BLOCK_FLOWER_POT_ID)
                else {
                    return unsupported();
                };
                if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
                    return unsupported();
                }
                let added = self.add_local_item_from_block_interaction(ProtocolItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: ProtocolDataComponentPatchSummary::default().into(),
                });
                success(LocalInteractionSuccess {
                    extra_swing: (!added).then_some(InteractionHand::MainHand),
                    ..LocalInteractionSuccess::client_item_interaction()
                })
            }
            VanillaBlockUseWithoutItemOverride::LecternBlock => {
                match boolean_property(block.block_properties, "has_book") {
                    Some(true) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(false) => success(LocalInteractionSuccess::consume(false)),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::LightBlock => {
                // The state cycle is guarded by `!level.isClientSide()`;
                // every client invocation therefore returns `CONSUME`.
                success(LocalInteractionSuccess::consume(false))
            }
            VanillaBlockUseWithoutItemOverride::JukeboxBlock => {
                match boolean_property(block.block_properties, "has_record") {
                    Some(true)
                        if self.block_entity_type_id_at(context.pos)
                            == Some(VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID) =>
                    {
                        // `JukeboxBlockEntity.popOutTheItem` is server-only;
                        // the client result still swings immediately.
                        success(LocalInteractionSuccess::client_item_interaction())
                    }
                    Some(_) => pass(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::NoteBlock => {
                // Tuning and note playback are server-side, while the client
                // result is an unconditional immediate success.
                success(LocalInteractionSuccess::client_item_interaction())
            }
            VanillaBlockUseWithoutItemOverride::ComparatorBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                self.predict_local_comparator_use(context, block, owner)
            }
            VanillaBlockUseWithoutItemOverride::RespawnAnchorBlock => {
                match integer_property(block.block_properties, "charges", 0..=4) {
                    Some(0) => pass(),
                    Some(_) => success(LocalInteractionSuccess::consume(false)),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::RedStoneWireBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                self.predict_local_redstone_wire_toggle(context, block, owner)
            }
            VanillaBlockUseWithoutItemOverride::SweetBerryBushBlock => {
                match integer_property(block.block_properties, "age", 0..=3) {
                    Some(2 | 3) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(_) => pass(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::SignBlock => {
                if matches!(
                    self.block_entity_type_id_at(context.pos),
                    Some(
                        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID
                            | VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
                    )
                ) {
                    // `SignBlock.useWithoutItem` should only be invoked on the
                    // server, but its defensive `ClientLevel` branch is the
                    // exact no-swing `CONSUME` result.
                    success(LocalInteractionSuccess::consume(false))
                } else {
                    pass()
                }
            }
        }
    }

    fn predict_local_cake_eat(
        &mut self,
        context: LocalBlockUseContext,
        block: &BlockView,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(abilities) = self.local_player.abilities else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        let can_eat = abilities.invulnerable
            || self
                .local_player
                .health
                .is_some_and(|health| health.food < 20);
        if !can_eat {
            if owner == VanillaBlockUseWithoutItemOverride::CakeBlock
                && self.local_item_in_hand(InteractionHand::MainHand).is_none()
            {
                return success(LocalInteractionSuccess::consume(false));
            }
            return pass();
        }

        let state_id = if owner == VanillaBlockUseWithoutItemOverride::CakeBlock {
            let Some(bites) = integer_property(block.block_properties, "bites", 0..=6) else {
                return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
            };
            if bites < 6 {
                vanilla_block_state_cycle_property(block.block_state_id, "bites")
            } else {
                vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID)
            }
        } else {
            vanilla_block_default_state_id(VANILLA_BLOCK_CAKE_ID)
                .and_then(|state| vanilla_block_state_with_property(state, "bites", "1"))
        };
        let Some(state_id) = state_id else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        }

        let health = self
            .local_player
            .health
            .get_or_insert(LocalPlayerHealthState {
                health: 20.0,
                food: 20,
                saturation: 5.0,
            });
        health.food = (health.food + 2).clamp(0, 20);
        health.saturation = (health.saturation + 0.4_f32).clamp(0.0, health.food as f32);
        success(LocalInteractionSuccess::client_item_interaction())
    }

    fn predict_local_redstone_wire_toggle(
        &mut self,
        context: LocalBlockUseContext,
        block: &BlockView,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(connected) = wire_connected_sides(block.block_properties) else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        let is_cross = connected.iter().all(|connected| *connected);
        let is_dot = connected.iter().all(|connected| !*connected);
        if !is_cross && !is_dot {
            return pass();
        }

        // Vanilla computes `wasDot` from the replacement state: a cross input
        // starts from the default dot, while a dot input starts from the cross.
        let Some(updated) =
            self.local_redstone_wire_connection_state(context.pos, block.block_state_id, is_cross)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        if updated == block.block_state_id {
            return pass();
        }
        if !self.predict_local_block_state(context.pos, updated, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        }
        success(LocalInteractionSuccess::client_item_interaction())
    }

    fn predict_local_block_state_interaction(
        &mut self,
        context: LocalBlockUseContext,
        state_id: Option<i32>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        self.predict_local_block_state_interaction_with_sound(context, state_id, owner, None)
    }

    fn predict_local_block_state_interaction_with_sound(
        &mut self,
        context: LocalBlockUseContext,
        state_id: Option<i32>,
        owner: VanillaBlockUseWithoutItemOverride,
        sound_action: Option<LocalInteractionSoundAction>,
    ) -> LocalTargetUsePrediction {
        let Some(state_id) = state_id else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        }
        success(LocalInteractionSuccess {
            sound_action,
            ..LocalInteractionSuccess::client_item_interaction()
        })
    }

    fn local_player_may_build(&self) -> bool {
        !self.local_player_is_adventure() && !self.local_player_is_spectator()
    }

    fn local_player_horizontal_direction(&self) -> Option<ProtocolDirection> {
        let yaw = f64::from(self.local_player().pose?.y_rot);
        if !yaw.is_finite() {
            return None;
        }
        match ((yaw / 90.0 + 0.5).floor() as i64).rem_euclid(4) {
            0 => Some(ProtocolDirection::South),
            1 => Some(ProtocolDirection::West),
            2 => Some(ProtocolDirection::North),
            3 => Some(ProtocolDirection::East),
            _ => unreachable!("rem_euclid(4) is in 0..4"),
        }
    }
}

fn button_click_on_sound_id(block_name: &str) -> &'static str {
    // Vanilla `BlockSetType.java:29-217`.
    match block_name {
        "minecraft:stone_button" | "minecraft:polished_blackstone_button" => {
            "minecraft:block.stone_button.click_on"
        }
        "minecraft:cherry_button" => "minecraft:block.cherry_wood_button.click_on",
        "minecraft:bamboo_button" => "minecraft:block.bamboo_wood_button.click_on",
        "minecraft:crimson_button" | "minecraft:warped_button" => {
            "minecraft:block.nether_wood_button.click_on"
        }
        _ => "minecraft:block.wooden_button.click_on",
    }
}

fn door_or_trapdoor_toggle_sound_id(
    block_name: &str,
    is_trapdoor: bool,
    opening: bool,
) -> &'static str {
    // Vanilla `BlockSetType.java:29-217`.
    let is_copper = block_name.contains("copper");
    let is_cherry = block_name.starts_with("minecraft:cherry_");
    let is_bamboo = block_name.starts_with("minecraft:bamboo_");
    let is_nether =
        block_name.starts_with("minecraft:crimson_") || block_name.starts_with("minecraft:warped_");
    match (is_trapdoor, opening) {
        (false, true) => {
            if is_copper {
                "minecraft:block.copper_door.open"
            } else if is_cherry {
                "minecraft:block.cherry_wood_door.open"
            } else if is_bamboo {
                "minecraft:block.bamboo_wood_door.open"
            } else if is_nether {
                "minecraft:block.nether_wood_door.open"
            } else {
                "minecraft:block.wooden_door.open"
            }
        }
        (false, false) => {
            if is_copper {
                "minecraft:block.copper_door.close"
            } else if is_cherry {
                "minecraft:block.cherry_wood_door.close"
            } else if is_bamboo {
                "minecraft:block.bamboo_wood_door.close"
            } else if is_nether {
                "minecraft:block.nether_wood_door.close"
            } else {
                "minecraft:block.wooden_door.close"
            }
        }
        (true, true) => {
            if is_copper {
                "minecraft:block.copper_trapdoor.open"
            } else if is_cherry {
                "minecraft:block.cherry_wood_trapdoor.open"
            } else if is_bamboo {
                "minecraft:block.bamboo_wood_trapdoor.open"
            } else if is_nether {
                "minecraft:block.nether_wood_trapdoor.open"
            } else {
                "minecraft:block.wooden_trapdoor.open"
            }
        }
        (true, false) => {
            if is_copper {
                "minecraft:block.copper_trapdoor.close"
            } else if is_cherry {
                "minecraft:block.cherry_wood_trapdoor.close"
            } else if is_bamboo {
                "minecraft:block.bamboo_wood_trapdoor.close"
            } else if is_nether {
                "minecraft:block.nether_wood_trapdoor.close"
            } else {
                "minecraft:block.wooden_trapdoor.close"
            }
        }
    }
}

fn fence_gate_toggle_sound_id(block_name: &str, opening: bool) -> &'static str {
    // Vanilla `WoodType.java:16-67`.
    match (block_name, opening) {
        ("minecraft:cherry_fence_gate", true) => "minecraft:block.cherry_wood_fence_gate.open",
        ("minecraft:cherry_fence_gate", false) => "minecraft:block.cherry_wood_fence_gate.close",
        ("minecraft:bamboo_fence_gate", true) => "minecraft:block.bamboo_wood_fence_gate.open",
        ("minecraft:bamboo_fence_gate", false) => "minecraft:block.bamboo_wood_fence_gate.close",
        ("minecraft:crimson_fence_gate" | "minecraft:warped_fence_gate", true) => {
            "minecraft:block.nether_wood_fence_gate.open"
        }
        ("minecraft:crimson_fence_gate" | "minecraft:warped_fence_gate", false) => {
            "minecraft:block.nether_wood_fence_gate.close"
        }
        (_, true) => "minecraft:block.fence_gate.open",
        (_, false) => "minecraft:block.fence_gate.close",
    }
}

fn boolean_property(properties: &BTreeMap<String, String>, property: &str) -> Option<bool> {
    match properties.get(property)?.as_str() {
        "false" => Some(false),
        "true" => Some(true),
        _ => None,
    }
}

fn integer_property(
    properties: &BTreeMap<String, String>,
    property: &str,
    range: std::ops::RangeInclusive<u8>,
) -> Option<u8> {
    properties
        .get(property)?
        .parse::<u8>()
        .ok()
        .filter(|value| range.contains(value))
}

fn bell_hit_is_proper(
    properties: &BTreeMap<String, String>,
    clicked_direction: ProtocolDirection,
    click_y: f64,
) -> Option<bool> {
    if direction_axis(clicked_direction) == 'y' || click_y > f64::from(0.8124_f32) {
        return Some(false);
    }
    let facing = horizontal_direction_property(properties, "facing")?;
    let same_axis = direction_axis(facing) == direction_axis(clicked_direction);
    match properties.get("attachment")?.as_str() {
        "floor" => Some(same_axis),
        "single_wall" | "double_wall" => Some(!same_axis),
        "ceiling" => Some(true),
        _ => None,
    }
}

fn chiseled_bookshelf_hit_slot(
    facing: ProtocolDirection,
    clicked_direction: ProtocolDirection,
    cursor: [f64; 3],
) -> Option<usize> {
    if facing != clicked_direction {
        return None;
    }
    let relative_x = match clicked_direction {
        ProtocolDirection::North => 1.0 - cursor[0] as f32,
        ProtocolDirection::South => cursor[0] as f32,
        ProtocolDirection::West => cursor[2] as f32,
        ProtocolDirection::East => 1.0 - cursor[2] as f32,
        ProtocolDirection::Down | ProtocolDirection::Up => return None,
    };
    let row = selectable_slot_section(1.0 - cursor[1] as f32, 2);
    let column = selectable_slot_section(relative_x, 3);
    Some(column + row * 3)
}

fn selectable_slot_section(relative_coordinate: f32, sections: usize) -> usize {
    let targeted_pixel = relative_coordinate * 16.0;
    let section_size = 16.0 / sections as f32;
    (targeted_pixel / section_size)
        .floor()
        .clamp(0.0, (sections - 1) as f32) as usize
}

fn horizontal_direction_property(
    properties: &BTreeMap<String, String>,
    property: &str,
) -> Option<ProtocolDirection> {
    match properties.get(property)?.as_str() {
        "north" => Some(ProtocolDirection::North),
        "south" => Some(ProtocolDirection::South),
        "west" => Some(ProtocolDirection::West),
        "east" => Some(ProtocolDirection::East),
        _ => None,
    }
}

fn direction_axis(direction: ProtocolDirection) -> char {
    match direction {
        ProtocolDirection::West | ProtocolDirection::East => 'x',
        ProtocolDirection::Down | ProtocolDirection::Up => 'y',
        ProtocolDirection::North | ProtocolDirection::South => 'z',
    }
}

fn lever_attach_face(properties: &BTreeMap<String, String>) -> Option<LocalLeverAttachFace> {
    match properties.get("face")?.as_str() {
        "floor" => Some(LocalLeverAttachFace::Floor),
        "wall" => Some(LocalLeverAttachFace::Wall),
        "ceiling" => Some(LocalLeverAttachFace::Ceiling),
        _ => None,
    }
}

fn opposite_horizontal(direction: ProtocolDirection) -> ProtocolDirection {
    match direction {
        ProtocolDirection::North => ProtocolDirection::South,
        ProtocolDirection::South => ProtocolDirection::North,
        ProtocolDirection::West => ProtocolDirection::East,
        ProtocolDirection::East => ProtocolDirection::West,
        ProtocolDirection::Down | ProtocolDirection::Up => {
            unreachable!("horizontal direction cannot be vertical")
        }
    }
}

fn direction_name(direction: ProtocolDirection) -> &'static str {
    match direction {
        ProtocolDirection::North => "north",
        ProtocolDirection::South => "south",
        ProtocolDirection::West => "west",
        ProtocolDirection::East => "east",
        ProtocolDirection::Down | ProtocolDirection::Up => {
            unreachable!("horizontal direction cannot be vertical")
        }
    }
}

fn potted_flower_item_id(block_type: &BlockTypeCatalogEntry) -> Option<i32> {
    let content_constant = block_type.constant_name.strip_prefix("POTTED_")?;
    let content = VANILLA_BLOCK_TYPES
        .iter()
        .find(|candidate| candidate.constant_name == content_constant)?;
    VANILLA_ITEM_TYPES
        .iter()
        .find(|item| {
            item.block_item_behavior
                .is_some_and(|behavior| behavior.block_type_id == content.id)
        })
        .map(|item| item.id)
}

#[cfg(test)]
fn relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
    match direction {
        ProtocolDirection::Down => below(pos),
        ProtocolDirection::Up => above(pos),
        ProtocolDirection::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        ProtocolDirection::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        ProtocolDirection::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        ProtocolDirection::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}

#[cfg(test)]
fn above(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y + 1,
        ..pos
    }
}

#[cfg(test)]
fn below(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y - 1,
        ..pos
    }
}

fn success(success: LocalInteractionSuccess) -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success))
}

fn pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState,
        ItemFrameFacing, LightData, LocalInteractionSoundAction, LocalPlayerPoseState,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::{
        block_entity_types::{
            VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
            VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
        },
        block_types::{
            vanilla_block_type_for_resource_id, VANILLA_BLOCK_POTTED_POPPY_ID,
            VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_TYPES,
        },
        builtin_enum_component_values::VANILLA_DYE_COLOR_WHITE_ID,
        data_component_types::{
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        },
        entity_data_serializers::{
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        },
        entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        game_types::{
            VANILLA_GAME_TYPE_ADVENTURE_ID, VANILLA_GAME_TYPE_CREATIVE_ID,
            VANILLA_GAME_TYPE_SURVIVAL_ID,
        },
        item_types::{
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_BONE_MEAL_ID, VANILLA_ITEM_BOOK_ID,
            VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_CANDLE_ID, VANILLA_ITEM_FIRE_CHARGE_ID,
            VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_GLASS_BOTTLE_ID,
            VANILLA_ITEM_GLOWSTONE_ID, VANILLA_ITEM_HONEYCOMB_ID, VANILLA_ITEM_HONEY_BOTTLE_ID,
            VANILLA_ITEM_IRON_AXE_ID, VANILLA_ITEM_LAVA_BUCKET_ID, VANILLA_ITEM_LEATHER_HELMET_ID,
            VANILLA_ITEM_MUSIC_DISC_13_ID, VANILLA_ITEM_NOTE_BLOCK_ID, VANILLA_ITEM_OAK_LOG_ID,
            VANILLA_ITEM_POPPY_ID, VANILLA_ITEM_POTION_ID, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID,
            VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_SHEARS_ID, VANILLA_ITEM_STONE_ID,
            VANILLA_ITEM_WATER_BUCKET_ID, VANILLA_ITEM_WHITE_BANNER_ID,
            VANILLA_ITEM_WHITE_SHULKER_BOX_ID,
        },
        packets::{
            AddEntity, BannerPatternLayerSummary, BlockPos as ProtocolBlockPos, BlockUpdate,
            CommonPlayerSpawnInfo, DataComponentPatchSummary, Direction, EntityDataValue,
            EntityDataValueKind, EntityEvent, InteractionHand, ItemStackSummary, PlayLogin,
            PlayerHealth, RecipePropertySetSummary, RegistryTags, SetChunkCacheCenter,
            SetEntityData, SetPlayerInventory, TagNetworkPayload, UpdateRecipes, UpdateTags, Vec3d,
        },
    };
    use uuid::Uuid;

    mod cake_and_flower_pot;
    mod item_on_owners;
    mod redstone_and_openables;

    const POS: BlockPos = BlockPos { x: 0, y: 8, z: 0 };

    #[test]
    fn server_owned_menu_blocks_return_client_success_for_every_generated_owner_family() {
        for name in [
            "minecraft:furnace",
            "minecraft:anvil",
            "minecraft:barrel",
            "minecraft:beacon",
            "minecraft:brewing_stand",
            "minecraft:cartography_table",
            "minecraft:chest",
            "minecraft:crafter",
            "minecraft:crafting_table",
            "minecraft:dispenser",
            "minecraft:enchanting_table",
            "minecraft:ender_chest",
            "minecraft:grindstone",
            "minecraft:hopper",
            "minecraft:loom",
            "minecraft:shulker_box",
            "minecraft:smithing_table",
            "minecraft:stonecutter",
        ] {
            let mut world = block_use_world_first_state(name, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            assert_eq!(
                use_main_hand(&mut world, 81),
                success(LocalInteractionSuccess::client_item_interaction()),
                "{name}"
            );
            assert!(
                world.local_block_predictions().is_empty(),
                "{name} keeps menu contents and block state server-owned"
            );
        }
    }

    fn use_main_hand(world: &mut WorldStore, sequence: i32) -> LocalTargetUsePrediction {
        world.predict_local_block_use(
            POS,
            Direction::Up,
            InteractionHand::MainHand,
            false,
            sequence,
        )
    }

    fn use_item_on(
        world: &mut WorldStore,
        block_name: &str,
        hand: InteractionHand,
        face: Direction,
        cursor: [f64; 3],
        sequence: i32,
    ) -> LocalTargetUsePrediction {
        let block_type = vanilla_block_type_for_resource_id(block_name)
            .unwrap_or_else(|| panic!("missing {block_name} block type"));
        let owner = block_type
            .use_item_on_override
            .unwrap_or_else(|| panic!("missing {block_name} useItemOn owner"));
        let block = world.block_view(POS).expect("loaded target");
        world.predict_local_block_use_item_on(
            owner,
            block_type,
            &block,
            hand,
            LocalBlockUseContext {
                pos: POS,
                face,
                cursor,
                inside: false,
                sequence,
                secondary_use_active: false,
            },
        )
    }

    fn set_main_hand(world: &mut WorldStore, item_id: i32) {
        set_main_hand_stack(world, item_stack(item_id, 1));
    }

    fn set_main_hand_stack(world: &mut WorldStore, item: ItemStackSummary) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item }));
    }

    fn apply_cauldron_remove_dye_tag(world: &mut WorldStore, entries: Vec<i32>) {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:cauldron_can_remove_dye".to_string(),
                    entries,
                }],
            }],
        });
    }

    fn block_property(world: &WorldStore, property: &str) -> Option<String> {
        world
            .probe_block(POS)
            .and_then(|block| block.block_properties.get(property).cloned())
    }

    fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn assert_wire_sides(world: &WorldStore, sides: [&str; 4], power: &str) {
        for (property, expected) in ["north", "east", "south", "west"].into_iter().zip(sides) {
            assert_eq!(
                block_property(world, property).as_deref(),
                Some(expected),
                "{property}"
            );
        }
        assert_eq!(block_property(world, "power").as_deref(), Some(power));
    }

    fn wire_properties(
        north: &'static str,
        east: &'static str,
        south: &'static str,
        west: &'static str,
        power: &'static str,
    ) -> Vec<(&'static str, &'static str)> {
        vec![
            ("north", north),
            ("east", east),
            ("south", south),
            ("west", west),
            ("power", power),
        ]
    }

    fn block_use_world<const N: usize>(
        name: &str,
        properties: [(&str, &str); N],
        game_type: i8,
    ) -> WorldStore {
        block_use_world_vec(name, properties.into_iter().collect(), game_type)
    }

    fn block_use_world_first_state(name: &str, game_type: i8) -> WorldStore {
        block_use_world_first_state_with_optional_entity(name, game_type, None)
    }

    fn block_use_world_first_state_with_entity(
        name: &str,
        game_type: i8,
        block_entity_type_id: i32,
    ) -> WorldStore {
        block_use_world_first_state_with_optional_entity(
            name,
            game_type,
            Some(block_entity_type_id),
        )
    }

    fn block_use_world_first_state_with_optional_entity(
        name: &str,
        game_type: i8,
        block_entity_type_id: Option<i32>,
    ) -> WorldStore {
        let registry = crate::registries::BlockStateRegistry::vanilla_26_1();
        let state = registry
            .iter()
            .find(|state| state.name == name)
            .unwrap_or_else(|| panic!("missing {name} state"));
        block_use_world_vec_with_optional_entity(
            name,
            state
                .properties
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()))
                .collect(),
            game_type,
            block_entity_type_id,
        )
    }

    fn block_use_world_vec(name: &str, properties: Vec<(&str, &str)>, game_type: i8) -> WorldStore {
        block_use_world_vec_with_optional_entity(name, properties, game_type, None)
    }

    fn block_use_world_with_entity<const N: usize>(
        name: &str,
        properties: [(&str, &str); N],
        game_type: i8,
        block_entity_type_id: i32,
    ) -> WorldStore {
        block_use_world_vec_with_optional_entity(
            name,
            properties.into_iter().collect(),
            game_type,
            Some(block_entity_type_id),
        )
    }

    fn block_use_world_vec_with_optional_entity(
        name: &str,
        properties: Vec<(&str, &str)>,
        game_type: i8,
        block_entity_type_id: Option<i32>,
    ) -> WorldStore {
        let air = state_id("minecraft:air", []);
        let single_value = |domain, entry_count, global_id| PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        };
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.apply_login(&PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.apply_set_chunk_cache_center(SetChunkCacheCenter {
            chunk_x: 0,
            chunk_z: 0,
        });
        let section_count =
            usize::try_from(world.dimension().height.div_euclid(16)).expect("section count");
        let block_entities = block_entity_type_id
            .map(|type_id| {
                let mut record = BlockEntityRecord::empty(
                    POS.x.rem_euclid(16) as u8,
                    POS.y as i16,
                    POS.z.rem_euclid(16) as u8,
                    type_id,
                );
                if matches!(
                    type_id,
                    VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID | VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
                ) {
                    record.payload = crate::BlockEntityPayload::SignText(Box::default());
                }
                vec![record]
            })
            .unwrap_or_default();
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..section_count)
                .map(|_| {
                    std::sync::Arc::new(ChunkSection {
                        non_empty_block_count: 0,
                        fluid_count: 0,
                        block_states: single_value(PaletteDomain::BlockStates, 4096, air),
                        biomes: single_value(PaletteDomain::Biomes, 64, 0),
                    })
                })
                .collect(),
            block_entities: std::sync::Arc::new(block_entities),
            light: LightData::default(),
        });
        set_test_block(&mut world, state_id_vec(name, properties));
        world
    }

    fn set_test_block(world: &mut WorldStore, block_state_id: i32) {
        set_block_at(world, POS, block_state_id);
    }

    fn insert_air_chunk(world: &mut WorldStore, pos: ChunkPos) {
        let air = state_id("minecraft:air", []);
        let section_count =
            usize::try_from(world.dimension().height.div_euclid(16)).expect("section count");
        world.insert_decoded_chunk(ChunkColumn {
            pos,
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..section_count)
                .map(|_| {
                    std::sync::Arc::new(ChunkSection {
                        non_empty_block_count: 0,
                        fluid_count: 0,
                        block_states: PalettedContainerData {
                            domain: PaletteDomain::BlockStates,
                            bits_per_entry: 0,
                            palette_kind: PaletteKind::SingleValue,
                            palette_global_ids: vec![air],
                            packed_data: Vec::new(),
                            entry_count: 4096,
                        },
                        biomes: PalettedContainerData {
                            domain: PaletteDomain::Biomes,
                            bits_per_entry: 0,
                            palette_kind: PaletteKind::SingleValue,
                            palette_global_ids: vec![0],
                            packed_data: Vec::new(),
                            entry_count: 64,
                        },
                    })
                })
                .collect(),
            block_entities: std::sync::Arc::default(),
            light: LightData::default(),
        });
    }

    fn set_block_at(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        // Scene setup tolerates the vanilla identical-state no-op; the block
        // must hold the requested state either way.
        world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        });
        assert_eq!(
            world.probe_block(pos).map(|block| block.block_state_id),
            Some(block_state_id)
        );
    }

    fn insert_block_entity_record(
        world: &mut WorldStore,
        pos: BlockPos,
        type_id: i32,
        raw_nbt: Vec<u8>,
    ) {
        let chunk = world
            .chunks
            .get_mut(pos.chunk())
            .expect("block entity chunk");
        let mut record = BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            pos.y as i16,
            pos.z.rem_euclid(16) as u8,
            type_id,
        );
        record.raw_nbt = raw_nbt;
        let block_entities = std::sync::Arc::make_mut(&mut chunk.block_entities);
        if let Some(existing) = block_entities.iter_mut().find(|existing| {
            existing.local_x == record.local_x
                && existing.y == record.y
                && existing.local_z == record.local_z
        }) {
            *existing = record;
        } else {
            block_entities.push(record);
        }
    }

    fn decorated_pot_item_nbt(resource_id: &str, count: i32) -> Vec<u8> {
        let mut nbt = vec![10, 10];
        write_nbt_string(&mut nbt, "item");
        nbt.push(8);
        write_nbt_string(&mut nbt, "id");
        write_nbt_string(&mut nbt, resource_id);
        nbt.push(3);
        write_nbt_string(&mut nbt, "count");
        nbt.extend_from_slice(&count.to_be_bytes());
        nbt.extend_from_slice(&[0, 0]);
        nbt
    }

    fn lectern_book_nbt(page_count: usize, page: i32) -> Vec<u8> {
        let mut nbt = vec![10, 10];
        write_nbt_string(&mut nbt, "Book");
        nbt.push(8);
        write_nbt_string(&mut nbt, "id");
        write_nbt_string(&mut nbt, "minecraft:written_book");
        nbt.push(10);
        write_nbt_string(&mut nbt, "components");
        nbt.push(10);
        write_nbt_string(&mut nbt, "minecraft:written_book_content");
        nbt.push(9);
        write_nbt_string(&mut nbt, "pages");
        nbt.push(10);
        nbt.extend_from_slice(&(page_count as i32).to_be_bytes());
        nbt.extend(std::iter::repeat_n(0, page_count));
        nbt.extend_from_slice(&[0, 0, 0]);
        nbt.push(3);
        write_nbt_string(&mut nbt, "Page");
        nbt.extend_from_slice(&page.to_be_bytes());
        nbt.push(0);
        nbt
    }

    fn int_nbt(name: &str, value: i32) -> Vec<u8> {
        let mut nbt = vec![10, 3];
        write_nbt_string(&mut nbt, name);
        nbt.extend_from_slice(&value.to_be_bytes());
        nbt.push(0);
        nbt
    }

    fn container_item_nbt(slot: i8, resource_id: &str, count: i32) -> Vec<u8> {
        let mut nbt = vec![10, 9];
        write_nbt_string(&mut nbt, "Items");
        nbt.push(10);
        nbt.extend_from_slice(&1_i32.to_be_bytes());
        nbt.push(1);
        write_nbt_string(&mut nbt, "Slot");
        nbt.push(slot as u8);
        nbt.push(8);
        write_nbt_string(&mut nbt, "id");
        write_nbt_string(&mut nbt, resource_id);
        nbt.push(3);
        write_nbt_string(&mut nbt, "count");
        nbt.extend_from_slice(&count.to_be_bytes());
        nbt.extend_from_slice(&[0, 0]);
        nbt
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    fn add_item_frame(
        world: &mut WorldStore,
        id: i32,
        pos: BlockPos,
        direction_data: i32,
        rotation: i32,
    ) {
        world.apply_add_entity(AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id: VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            position: Vec3d {
                x: f64::from(pos.x),
                y: f64::from(pos.y),
                z: f64::from(pos.z),
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: direction_data,
        });
        let accessors =
            crate::entities::vanilla_entity_metadata_item_frame_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            )
            .expect("item-frame accessors");
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![
                EntityDataValue {
                    data_id: accessors.item_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    value: EntityDataValueKind::ItemStack(ItemStackSummary {
                        item_id: Some(VANILLA_ITEM_STONE_ID),
                        count: 1,
                        component_patch: DataComponentPatchSummary::default().into(),
                    }),
                },
                EntityDataValue {
                    data_id: accessors.rotation_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(rotation),
                },
            ],
        }));
    }

    fn state_id<const N: usize>(name: &str, properties: [(&str, &str); N]) -> i32 {
        state_id_vec(name, properties.into_iter().collect())
    }

    fn state_id_vec(name: &str, properties: Vec<(&str, &str)>) -> i32 {
        crate::vanilla_block_state_id(
            name,
            &properties
                .into_iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        )
        .unwrap_or_else(|| {
            let owner = vanilla_block_type_for_resource_id(name)
                .map(|block| block.constant_name)
                .unwrap_or("unknown");
            panic!("missing {owner} state")
        })
    }

    fn state_id_with_property(name: &str, property: &str, value: &str) -> i32 {
        let registry = crate::registries::BlockStateRegistry::vanilla_26_1();
        let state_id = registry
            .iter()
            .find(|state| {
                state.name == name
                    && state
                        .properties
                        .get(property)
                        .is_some_and(|actual| actual == value)
            })
            .map(|state| state.id)
            .unwrap_or_else(|| panic!("missing {name} state with {property}={value}"));
        state_id
    }
}
