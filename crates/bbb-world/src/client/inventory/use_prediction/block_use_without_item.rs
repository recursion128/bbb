use super::{
    redstone_wire::wire_connected_sides, LocalBlockUseContext, LocalInteractionParticleAction,
    LocalInteractionResult, LocalInteractionSuccess, LocalLeverAttachFace,
    LocalTargetUsePrediction,
};
#[cfg(test)]
use crate::BlockPos;
use crate::{LocalPlayerHealthState, WorldStore};
use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
        VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID, VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
        VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
    },
    block_types::{
        BlockTypeCatalogEntry, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_IRON_DOOR_ID,
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
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        let Some(block) = self.probe_block(context.pos) else {
            return unsupported();
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return unsupported();
        };
        if block_name != block_type.resource_id {
            return unsupported();
        }

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
                match bell_hit_is_proper(&block.block_properties, context.face, context.cursor[1]) {
                    Some(true) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(false) => pass(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::ButtonBlock => {
                let Some(powered) = boolean_property(&block.block_properties, "powered") else {
                    return unsupported();
                };
                if powered {
                    return success(LocalInteractionSuccess::consume(false));
                }
                self.predict_local_block_property(
                    context,
                    block_name,
                    block.block_properties,
                    "powered",
                    "true",
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::LeverBlock => {
                let Some(powered) = boolean_property(&block.block_properties, "powered") else {
                    return unsupported();
                };
                let particle_action = if powered {
                    None
                } else {
                    let Some(facing) =
                        horizontal_direction_property(&block.block_properties, "facing")
                    else {
                        return unsupported();
                    };
                    let Some(face) = lever_attach_face(&block.block_properties) else {
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
            VanillaBlockUseWithoutItemOverride::DoorBlock => {
                if block_type.id == VANILLA_BLOCK_IRON_DOOR_ID {
                    return pass();
                }
                self.predict_local_toggled_boolean_property(
                    context,
                    block_name,
                    block.block_properties,
                    "open",
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::TrapDoorBlock => {
                if block_type.id == VANILLA_BLOCK_IRON_TRAPDOOR_ID {
                    return pass();
                }
                self.predict_local_toggled_boolean_property(
                    context,
                    block_name,
                    block.block_properties,
                    "open",
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::FenceGateBlock => {
                let Some(open) = boolean_property(&block.block_properties, "open") else {
                    return unsupported();
                };
                let mut properties = block.block_properties;
                if open {
                    properties.insert("open".to_string(), "false".to_string());
                } else {
                    let Some(player_direction) = self.local_player_horizontal_direction() else {
                        return unsupported();
                    };
                    let Some(facing) = horizontal_direction_property(&properties, "facing") else {
                        return unsupported();
                    };
                    if facing == opposite_horizontal(player_direction) {
                        properties.insert(
                            "facing".to_string(),
                            direction_name(player_direction).to_string(),
                        );
                    }
                    properties.insert("open".to_string(), "true".to_string());
                }
                self.predict_local_block_properties(context, block_name, properties, owner)
            }
            VanillaBlockUseWithoutItemOverride::RepeaterBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                let Some(delay) = block
                    .block_properties
                    .get("delay")
                    .and_then(|delay| delay.parse::<u8>().ok())
                    .filter(|delay| (1..=4).contains(delay))
                else {
                    return unsupported();
                };
                self.predict_local_block_property(
                    context,
                    block_name,
                    block.block_properties,
                    "delay",
                    if delay == 4 {
                        "1".to_string()
                    } else {
                        (delay + 1).to_string()
                    },
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::CaveVinesBlock
            | VanillaBlockUseWithoutItemOverride::CaveVinesPlantBlock => {
                match boolean_property(&block.block_properties, "berries") {
                    Some(true) => success(LocalInteractionSuccess::client_item_interaction()),
                    Some(false) => pass(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::CakeBlock => self.predict_local_cake_eat(
                context,
                block_name,
                block.block_properties,
                true,
                owner,
            ),
            VanillaBlockUseWithoutItemOverride::CandleCakeBlock => self.predict_local_cake_eat(
                context,
                "minecraft:cake",
                BTreeMap::from([("bites".to_string(), "1".to_string())]),
                false,
                owner,
            ),
            VanillaBlockUseWithoutItemOverride::ComposterBlock => {
                match integer_property(&block.block_properties, "level", 0..=8) {
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
                let Some(facing) = horizontal_direction_property(&block.block_properties, "facing")
                else {
                    return unsupported();
                };
                let Some(slot) = chiseled_bookshelf_hit_slot(facing, context.face, context.cursor)
                else {
                    return pass();
                };
                let property = format!("slot_{slot}_occupied");
                match boolean_property(&block.block_properties, &property) {
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
                    // The `playSound(null, ...)` call advances the client sound
                    // seed but is suppressed; wobble sends a block event only
                    // from the logical server.
                    success(LocalInteractionSuccess {
                        sound_action: Some(
                            super::LocalInteractionSoundAction::SuppressedDecoratedPotInsertFail,
                        ),
                        ..LocalInteractionSuccess::client_item_interaction()
                    })
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
                let Some(state_id) = self.registries.block_state_id_by_name_and_properties(
                    "minecraft:flower_pot",
                    &BTreeMap::new(),
                ) else {
                    return unsupported();
                };
                if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
                    return unsupported();
                }
                let added = self.add_local_item_from_block_interaction(ProtocolItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: ProtocolDataComponentPatchSummary::default(),
                });
                success(LocalInteractionSuccess {
                    extra_swing: (!added).then_some(InteractionHand::MainHand),
                    ..LocalInteractionSuccess::client_item_interaction()
                })
            }
            VanillaBlockUseWithoutItemOverride::LecternBlock => {
                match boolean_property(&block.block_properties, "has_book") {
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
                match boolean_property(&block.block_properties, "has_record") {
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
                let mode = match block.block_properties.get("mode").map(String::as_str) {
                    Some("compare") => "subtract",
                    Some("subtract") => "compare",
                    _ => return unsupported(),
                };
                // Vanilla immediately calls `refreshOutputState` after this
                // mode mutation. Its output derives from arbitrary analog
                // block entities and item frames, which remain synchronized
                // server state here; the mode itself is client canonical.
                self.predict_local_block_property(
                    context,
                    block_name,
                    block.block_properties,
                    "mode",
                    mode,
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::RespawnAnchorBlock => {
                match integer_property(&block.block_properties, "charges", 0..=4) {
                    Some(0) => pass(),
                    Some(_) => success(LocalInteractionSuccess::consume(false)),
                    None => unsupported(),
                }
            }
            VanillaBlockUseWithoutItemOverride::RedStoneWireBlock => {
                if !self.local_player_may_build() {
                    return pass();
                }
                self.predict_local_redstone_wire_toggle(
                    context,
                    block_name,
                    block.block_properties,
                    owner,
                )
            }
            VanillaBlockUseWithoutItemOverride::SweetBerryBushBlock => {
                match integer_property(&block.block_properties, "age", 0..=3) {
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
        target_block_name: &str,
        mut target_properties: BTreeMap<String, String>,
        cake_empty_hand_consumes: bool,
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
            if cake_empty_hand_consumes
                && self.local_item_in_hand(InteractionHand::MainHand).is_none()
            {
                return success(LocalInteractionSuccess::consume(false));
            }
            return pass();
        }

        if owner == VanillaBlockUseWithoutItemOverride::CakeBlock {
            let Some(bites) = integer_property(&target_properties, "bites", 0..=6) else {
                return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
            };
            if bites < 6 {
                target_properties.insert("bites".to_string(), (bites + 1).to_string());
            } else {
                target_properties.clear();
            }
        }
        let resolved_block_name = if owner == VanillaBlockUseWithoutItemOverride::CakeBlock
            && target_properties.is_empty()
        {
            "minecraft:air"
        } else {
            target_block_name
        };
        let Some(state_id) = self
            .registries
            .block_state_id_by_name_and_properties(resolved_block_name, &target_properties)
        else {
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
        block_name: &str,
        properties: BTreeMap<String, String>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(power) = properties
            .get("power")
            .and_then(|power| power.parse::<u8>().ok())
            .filter(|power| *power <= 15)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        let Some(connected) = wire_connected_sides(&properties) else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        let is_cross = connected.iter().all(|connected| *connected);
        let is_dot = connected.iter().all(|connected| !*connected);
        if !is_cross && !is_dot {
            return pass();
        }

        let updated = BTreeMap::from([
            ("north".to_string(), "none".to_string()),
            ("east".to_string(), "none".to_string()),
            ("south".to_string(), "none".to_string()),
            ("west".to_string(), "none".to_string()),
            ("power".to_string(), power.to_string()),
        ]);
        // Vanilla computes `wasDot` from the replacement state: a cross input
        // starts from the default dot, while a dot input starts from the cross.
        let replacement_was_dot = is_cross;
        let Some(updated) =
            self.local_redstone_wire_connection_state(context.pos, updated, replacement_was_dot)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        if updated == properties {
            return pass();
        }
        // Preserve the exact resource rather than assuming only the built-in
        // redstone-wire name can own this generated override.
        self.predict_local_block_properties(context, block_name, updated, owner)
    }

    fn predict_local_toggled_boolean_property(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        properties: BTreeMap<String, String>,
        property: &str,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(value) = boolean_property(&properties, property) else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        self.predict_local_block_property(
            context,
            block_name,
            properties,
            property,
            if value { "false" } else { "true" },
            owner,
        )
    }

    fn predict_local_block_property(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        mut properties: BTreeMap<String, String>,
        property: &str,
        value: impl Into<String>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        properties.insert(property.to_string(), value.into());
        self.predict_local_block_properties(context, block_name, properties, owner)
    }

    fn predict_local_block_properties(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        properties: BTreeMap<String, String>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let Some(state_id) = self
            .registries
            .block_state_id_by_name_and_properties(block_name, &properties)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        };
        if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        }
        success(LocalInteractionSuccess::client_item_interaction())
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
        BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        LocalInteractionSoundAction, LocalPlayerPoseState, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::{
        block_types::{
            vanilla_block_type_for_resource_id, VANILLA_BLOCK_POTTED_POPPY_ID, VANILLA_BLOCK_TYPES,
        },
        data_component_types::{
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        },
        game_types::{
            VANILLA_GAME_TYPE_ADVENTURE_ID, VANILLA_GAME_TYPE_CREATIVE_ID,
            VANILLA_GAME_TYPE_SURVIVAL_ID,
        },
        item_types::{
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_BONE_MEAL_ID, VANILLA_ITEM_BOOK_ID,
            VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_CANDLE_ID, VANILLA_ITEM_FIRE_CHARGE_ID,
            VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_GLASS_BOTTLE_ID,
            VANILLA_ITEM_GLOWSTONE_ID, VANILLA_ITEM_IRON_AXE_ID, VANILLA_ITEM_LAVA_BUCKET_ID,
            VANILLA_ITEM_LEATHER_HELMET_ID, VANILLA_ITEM_MUSIC_DISC_13_ID,
            VANILLA_ITEM_NOTE_BLOCK_ID, VANILLA_ITEM_OAK_LOG_ID, VANILLA_ITEM_POPPY_ID,
            VANILLA_ITEM_POTION_ID, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID, VANILLA_ITEM_RED_DYE_ID,
            VANILLA_ITEM_SHEARS_ID, VANILLA_ITEM_STONE_ID, VANILLA_ITEM_WATER_BUCKET_ID,
            VANILLA_ITEM_WHITE_BANNER_ID, VANILLA_ITEM_WHITE_SHULKER_BOX_ID,
        },
        packets::{
            BannerPatternLayerSummary, BlockPos as ProtocolBlockPos, BlockUpdate,
            CommonPlayerSpawnInfo, DataComponentPatchSummary, Direction, EntityEvent,
            InteractionHand, ItemStackSummary, PlayLogin, PlayerHealth, RecipePropertySetSummary,
            RegistryTags, SetChunkCacheCenter, SetPlayerInventory, TagNetworkPayload,
            UpdateRecipes, UpdateTags,
        },
    };
    use std::collections::BTreeSet;

    const POS: BlockPos = BlockPos { x: 0, y: 8, z: 0 };

    #[test]
    fn button_press_and_powered_consume_match_vanilla_results() {
        let mut world = block_use_world(
            "minecraft:stone_button",
            [("face", "wall"), ("facing", "north"), ("powered", "false")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut world, 11),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&world, "powered"), Some("true".to_string()));
        assert_eq!(world.local_block_predictions()[0].sequence, 11);

        assert_eq!(
            use_main_hand(&mut world, 12),
            success(LocalInteractionSuccess::consume(false))
        );
        assert_eq!(world.local_block_predictions().len(), 1);
    }

    #[test]
    fn lever_only_requests_client_particle_when_switching_toward_powered() {
        let mut world = block_use_world(
            "minecraft:lever",
            [
                ("face", "ceiling"),
                ("facing", "east"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut world, 1),
            success(LocalInteractionSuccess {
                particle_action: Some(LocalInteractionParticleAction::LeverActivation {
                    pos: POS,
                    facing: Direction::East,
                    face: LocalLeverAttachFace::Ceiling,
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(
            block_property(&world, "powered"),
            Some("false".to_string()),
            "the client-side branch does not pull the lever state"
        );
        set_test_block(
            &mut world,
            state_id(
                "minecraft:lever",
                [("face", "ceiling"), ("facing", "east"), ("powered", "true")],
            ),
        );
        assert_eq!(
            use_main_hand(&mut world, 2),
            success(LocalInteractionSuccess::client_item_interaction())
        );
    }

    #[test]
    fn wooden_and_copper_openables_toggle_while_iron_passes_to_item_use() {
        for (name, properties) in [
            (
                "minecraft:oak_door",
                vec![
                    ("facing", "north"),
                    ("half", "lower"),
                    ("hinge", "left"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            ),
            (
                "minecraft:copper_trapdoor",
                vec![
                    ("facing", "north"),
                    ("half", "bottom"),
                    ("open", "false"),
                    ("powered", "false"),
                    ("waterlogged", "false"),
                ],
            ),
        ] {
            let mut world =
                block_use_world_vec(name, properties, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            assert_eq!(
                use_main_hand(&mut world, 4),
                success(LocalInteractionSuccess::client_item_interaction())
            );
            assert_eq!(block_property(&world, "open"), Some("true".to_string()));
        }

        for (name, properties) in [
            (
                "minecraft:iron_door",
                vec![
                    ("facing", "north"),
                    ("half", "lower"),
                    ("hinge", "left"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            ),
            (
                "minecraft:iron_trapdoor",
                vec![
                    ("facing", "north"),
                    ("half", "bottom"),
                    ("open", "false"),
                    ("powered", "false"),
                    ("waterlogged", "false"),
                ],
            ),
        ] {
            let mut world =
                block_use_world_vec(name, properties, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            assert_eq!(use_main_hand(&mut world, 5), pass());
            assert!(world.local_block_predictions().is_empty());
        }
    }

    #[test]
    fn fence_gate_uses_vanilla_player_direction_only_when_opening() {
        let mut world = block_use_world(
            "minecraft:oak_fence_gate",
            [
                ("facing", "north"),
                ("in_wall", "false"),
                ("open", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        assert_eq!(
            use_main_hand(&mut world, 21),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&world, "open"), Some("true".to_string()));
        assert_eq!(
            block_property(&world, "facing"),
            Some("south".to_string()),
            "SOUTH player direction replaces opposite NORTH facing"
        );

        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: f32::NAN,
            ..LocalPlayerPoseState::default()
        });
        assert_eq!(
            use_main_hand(&mut world, 22),
            success(LocalInteractionSuccess::client_item_interaction()),
            "closing does not query player direction"
        );
        assert_eq!(block_property(&world, "open"), Some("false".to_string()));
    }

    #[test]
    fn repeater_cycles_delay_only_with_may_build_and_daylight_is_server_owned() {
        let mut survival = block_use_world(
            "minecraft:repeater",
            [
                ("delay", "4"),
                ("facing", "north"),
                ("locked", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut survival, 31),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&survival, "delay"), Some("1".to_string()));

        let mut adventure = block_use_world(
            "minecraft:repeater",
            [
                ("delay", "2"),
                ("facing", "north"),
                ("locked", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_ADVENTURE_ID as i8,
        );
        assert_eq!(use_main_hand(&mut adventure, 32), pass());
        assert!(adventure.local_block_predictions().is_empty());

        let mut detector = block_use_world(
            "minecraft:daylight_detector",
            [("inverted", "false"), ("power", "7")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut detector, 33),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            block_property(&detector, "inverted"),
            Some("false".to_string())
        );
        assert!(detector.local_block_predictions().is_empty());
    }

    #[test]
    fn comparator_cycles_mode_with_may_build_and_keeps_server_output_state() {
        let mut survival = block_use_world(
            "minecraft:comparator",
            [
                ("facing", "north"),
                ("mode", "compare"),
                ("powered", "true"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut survival, 41),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            block_property(&survival, "mode"),
            Some("subtract".to_string())
        );
        assert_eq!(
            block_property(&survival, "powered"),
            Some("true".to_string())
        );

        let mut adventure = block_use_world(
            "minecraft:comparator",
            [
                ("facing", "north"),
                ("mode", "subtract"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_ADVENTURE_ID as i8,
        );
        assert_eq!(use_main_hand(&mut adventure, 42), pass());
        assert_eq!(
            block_property(&adventure, "mode"),
            Some("subtract".to_string())
        );
    }

    #[test]
    fn redstone_wire_toggles_cross_and_dot_with_preserved_power() {
        let mut world = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "9"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut world, 51),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&world, ["none", "none", "none", "none"], "9");

        set_test_block(
            &mut world,
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "9"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut world, 52),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&world, ["side", "side", "side", "side"], "9");
    }

    #[test]
    fn redstone_wire_recomputes_real_side_and_up_connections() {
        let mut side = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "0"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(
            &mut side,
            relative(POS, Direction::East),
            state_id_vec(
                "minecraft:repeater",
                vec![
                    ("delay", "1"),
                    ("facing", "east"),
                    ("locked", "false"),
                    ("powered", "false"),
                ],
            ),
        );
        assert_eq!(
            use_main_hand(&mut side, 61),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&side, ["none", "side", "none", "side"], "0");

        let mut climb = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "4"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        let east = relative(POS, Direction::East);
        set_block_at(&mut climb, east, state_id("minecraft:stone", []));
        set_block_at(
            &mut climb,
            above(east),
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "4"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut climb, 62),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&climb, ["none", "up", "none", "side"], "4");

        let mut support_override = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "4"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(
            &mut support_override,
            east,
            state_id("minecraft:soul_sand", []),
        );
        set_block_at(
            &mut support_override,
            above(east),
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "4"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut support_override, 63),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&support_override, ["none", "up", "none", "side"], "4");

        let mut blocked_climb = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "4"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(
            &mut blocked_climb,
            above(POS),
            state_id("minecraft:stone", []),
        );
        set_block_at(&mut blocked_climb, east, state_id("minecraft:stone", []));
        set_block_at(
            &mut blocked_climb,
            above(east),
            state_id_vec(
                "minecraft:redstone_wire",
                wire_properties("none", "none", "none", "none", "4"),
            ),
        );
        assert_eq!(
            use_main_hand(&mut blocked_climb, 64),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&blocked_climb, ["none", "none", "none", "none"], "4");
    }

    #[test]
    fn redstone_wire_honors_always_and_never_conductor_overrides() {
        let east = relative(POS, Direction::East);
        let below_east = below(east);
        let lower_wire = state_id_vec(
            "minecraft:redstone_wire",
            wire_properties("none", "none", "none", "none", "0"),
        );

        let mut soul_sand = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "0"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(&mut soul_sand, east, state_id("minecraft:soul_sand", []));
        set_block_at(&mut soul_sand, below_east, lower_wire);
        assert_eq!(
            use_main_hand(&mut soul_sand, 71),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&soul_sand, ["none", "none", "none", "none"], "0");

        let mut glass = block_use_world_vec(
            "minecraft:redstone_wire",
            wire_properties("side", "side", "side", "side", "0"),
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_block_at(&mut glass, east, state_id("minecraft:glass", []));
        set_block_at(&mut glass, below_east, lower_wire);
        assert_eq!(
            use_main_hand(&mut glass, 72),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_wire_sides(&glass, ["none", "side", "none", "side"], "0");
    }

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

    #[test]
    fn fixed_client_branch_results_match_bed_note_light_fence_piston_and_dragon_egg() {
        for (name, expected) in [
            (
                "minecraft:red_bed",
                success(LocalInteractionSuccess::server_item_interaction()),
            ),
            (
                "minecraft:note_block",
                success(LocalInteractionSuccess::client_item_interaction()),
            ),
            (
                "minecraft:light",
                success(LocalInteractionSuccess::consume(false)),
            ),
            ("minecraft:oak_fence", pass()),
            ("minecraft:moving_piston", pass()),
            (
                "minecraft:dragon_egg",
                success(LocalInteractionSuccess::client_item_interaction()),
            ),
        ] {
            let mut world = block_use_world_first_state(name, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            assert_eq!(use_main_hand(&mut world, 82), expected, "{name}");
            assert!(world.local_block_predictions().is_empty(), "{name}");
        }
    }

    #[test]
    fn bell_result_follows_vanilla_attachment_face_and_hit_height() {
        let mut world = block_use_world(
            "minecraft:bell",
            [
                ("attachment", "floor"),
                ("facing", "north"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            world.predict_local_block_use_with_hit(
                POS,
                Direction::North,
                [0.5, 0.75, 0.0],
                false,
                InteractionHand::MainHand,
                false,
                91,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        for (face, cursor) in [
            (Direction::East, [1.0, 0.5, 0.5]),
            (Direction::North, [0.5, 0.9, 0.0]),
            (Direction::Up, [0.5, 1.0, 0.5]),
        ] {
            assert_eq!(
                world.predict_local_block_use_with_hit(
                    POS,
                    face,
                    cursor,
                    false,
                    InteractionHand::MainHand,
                    false,
                    92,
                ),
                pass()
            );
        }

        set_test_block(
            &mut world,
            state_id(
                "minecraft:bell",
                [
                    ("attachment", "single_wall"),
                    ("facing", "north"),
                    ("powered", "false"),
                ],
            ),
        );
        assert_eq!(
            world.predict_local_block_use_with_hit(
                POS,
                Direction::East,
                [1.0, 0.5, 0.5],
                false,
                InteractionHand::MainHand,
                false,
                93,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
    }

    #[test]
    fn chiseled_bookshelf_routes_front_hit_to_exact_occupied_slot() {
        let properties = [
            ("facing", "north"),
            ("slot_0_occupied", "false"),
            ("slot_1_occupied", "false"),
            ("slot_2_occupied", "false"),
            ("slot_3_occupied", "false"),
            ("slot_4_occupied", "false"),
            ("slot_5_occupied", "false"),
        ];
        let mut world = block_use_world_with_entity(
            "minecraft:chiseled_bookshelf",
            properties,
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
        );
        assert_eq!(
            world.predict_local_block_use_with_hit(
                POS,
                Direction::North,
                [0.9, 0.75, 0.0],
                false,
                InteractionHand::MainHand,
                false,
                101,
            ),
            success(LocalInteractionSuccess::consume(false)),
            "NORTH mirrors X, targeting empty slot 0"
        );
        assert_eq!(
            world.predict_local_block_use_with_hit(
                POS,
                Direction::East,
                [1.0, 0.75, 0.5],
                false,
                InteractionHand::MainHand,
                false,
                102,
            ),
            pass(),
            "non-front faces expose no selectable slot"
        );

        set_test_block(
            &mut world,
            state_id(
                "minecraft:chiseled_bookshelf",
                [
                    ("facing", "north"),
                    ("slot_0_occupied", "true"),
                    ("slot_1_occupied", "false"),
                    ("slot_2_occupied", "false"),
                    ("slot_3_occupied", "false"),
                    ("slot_4_occupied", "false"),
                    ("slot_5_occupied", "false"),
                ],
            ),
        );
        assert_eq!(
            world.predict_local_block_use_with_hit(
                POS,
                Direction::North,
                [0.9, 0.75, 0.0],
                false,
                InteractionHand::MainHand,
                false,
                103,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
    }

    #[test]
    fn gamemaster_block_owners_require_matching_entity_permission_and_instabuild() {
        for (name, type_id) in [
            (
                "minecraft:command_block",
                VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
            ),
            ("minecraft:jigsaw", VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID),
            (
                "minecraft:structure_block",
                VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
            ),
            (
                "minecraft:test_block",
                VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
            ),
            (
                "minecraft:test_instance_block",
                VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
            ),
        ] {
            let mut creative = block_use_world_first_state_with_entity(
                name,
                VANILLA_GAME_TYPE_CREATIVE_ID as i8,
                type_id,
            );
            assert_eq!(use_main_hand(&mut creative, 111), pass(), "{name}");
            assert!(creative.apply_entity_event(EntityEvent {
                entity_id: 7,
                event_id: 26,
            }));
            assert_eq!(
                use_main_hand(&mut creative, 112),
                success(LocalInteractionSuccess::client_item_interaction()),
                "{name}"
            );

            let mut survival = block_use_world_first_state_with_entity(
                name,
                VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
                type_id,
            );
            assert!(survival.apply_entity_event(EntityEvent {
                entity_id: 7,
                event_id: 26,
            }));
            assert_eq!(
                use_main_hand(&mut survival, 113),
                pass(),
                "{name} permission without instabuild"
            );
        }
    }

    #[test]
    fn entity_backed_pot_jukebox_and_sign_results_keep_server_mutations_authoritative() {
        for (name, type_id, expected) in [
            (
                "minecraft:decorated_pot",
                VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
                success(LocalInteractionSuccess::client_item_interaction()),
            ),
            (
                "minecraft:oak_sign",
                VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                success(LocalInteractionSuccess::consume(false)),
            ),
            (
                "minecraft:oak_hanging_sign",
                VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
                success(LocalInteractionSuccess::consume(false)),
            ),
        ] {
            let mut world = block_use_world_first_state_with_entity(
                name,
                VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
                type_id,
            );
            assert_eq!(use_main_hand(&mut world, 121), expected, "{name}");

            let mut missing =
                block_use_world_first_state(name, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            missing.remove_block_entity_record_at(POS);
            assert_eq!(use_main_hand(&mut missing, 122), pass(), "{name}");
        }

        let mut decorated = block_use_world_first_state_with_entity(
            "minecraft:decorated_pot",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
        );
        assert_eq!(
            decorated.predict_local_block_use_without_item(
                VanillaBlockUseWithoutItemOverride::DecoratedPotBlock,
                vanilla_block_type_for_resource_id("minecraft:decorated_pot").unwrap(),
                LocalBlockUseContext {
                    pos: POS,
                    face: Direction::Up,
                    cursor: [0.5, 1.0, 0.5],
                    inside: false,
                    sequence: 125,
                    secondary_use_active: false,
                },
            ),
            success(LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedDecoratedPotInsertFail,),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );

        let mut jukebox = block_use_world_with_entity(
            "minecraft:jukebox",
            [("has_record", "true")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID,
        );
        assert_eq!(
            use_main_hand(&mut jukebox, 123),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_test_block(
            &mut jukebox,
            state_id("minecraft:jukebox", [("has_record", "false")]),
        );
        assert_eq!(use_main_hand(&mut jukebox, 124), pass());
    }

    #[test]
    fn fixed_item_on_owners_route_exact_client_results() {
        let mut pumpkin =
            block_use_world_first_state("minecraft:pumpkin", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        set_main_hand(&mut pumpkin, VANILLA_ITEM_SHEARS_ID);
        assert_eq!(
            use_item_on(
                &mut pumpkin,
                "minecraft:pumpkin",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                126,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_main_hand(&mut pumpkin, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            use_item_on(
                &mut pumpkin,
                "minecraft:pumpkin",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                127,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );

        let mut bush = block_use_world_first_state(
            "minecraft:sweet_berry_bush",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_test_block(
            &mut bush,
            state_id_with_property("minecraft:sweet_berry_bush", "age", "2"),
        );
        set_main_hand(&mut bush, VANILLA_ITEM_BONE_MEAL_ID);
        assert_eq!(
            use_item_on(
                &mut bush,
                "minecraft:sweet_berry_bush",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                128,
            ),
            pass()
        );
        set_test_block(
            &mut bush,
            state_id_with_property("minecraft:sweet_berry_bush", "age", "3"),
        );
        assert_eq!(
            use_item_on(
                &mut bush,
                "minecraft:sweet_berry_bush",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                129,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );

        for item_id in [VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FIRE_CHARGE_ID] {
            let mut tnt =
                block_use_world_first_state("minecraft:tnt", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            set_main_hand(&mut tnt, item_id);
            assert_eq!(
                use_item_on(
                    &mut tnt,
                    "minecraft:tnt",
                    InteractionHand::MainHand,
                    Direction::Up,
                    [0.5, 1.0, 0.5],
                    130,
                ),
                success(LocalInteractionSuccess::client_item_interaction())
            );
            assert!(tnt.local_block_predictions().is_empty());
        }

        let mut vault =
            block_use_world_first_state("minecraft:vault", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        set_test_block(
            &mut vault,
            state_id_with_property("minecraft:vault", "vault_state", "active"),
        );
        set_main_hand(&mut vault, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            use_item_on(
                &mut vault,
                "minecraft:vault",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                131,
            ),
            success(LocalInteractionSuccess::server_item_interaction())
        );
    }

    #[test]
    fn composter_item_on_uses_generated_membership_without_client_mutation() {
        let mut world = block_use_world(
            "minecraft:composter",
            [("level", "7")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
        assert_eq!(
            use_item_on(
                &mut world,
                "minecraft:composter",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                147,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&world, "level").as_deref(), Some("7"));
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&item_stack(VANILLA_ITEM_APPLE_ID, 1))
        );
        assert!(world.local_block_predictions().is_empty());

        set_test_block(
            &mut world,
            state_id_with_property("minecraft:composter", "level", "8"),
        );
        assert_eq!(
            use_item_on(
                &mut world,
                "minecraft:composter",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                148,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );
        set_test_block(
            &mut world,
            state_id_with_property("minecraft:composter", "level", "6"),
        );
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            use_item_on(
                &mut world,
                "minecraft:composter",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                149,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );
    }

    #[test]
    fn cauldron_generated_item_routes_apply_exact_level_and_potion_predicates() {
        let mut empty =
            block_use_world_first_state("minecraft:cauldron", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        set_main_hand(&mut empty, VANILLA_ITEM_WATER_BUCKET_ID);
        assert_eq!(
            use_item_on(
                &mut empty,
                "minecraft:cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                150,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert!(empty.local_block_predictions().is_empty());

        let water_potion =
            bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:water").unwrap();
        set_main_hand_stack(
            &mut empty,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_POTION_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                    potion_id: Some(water_potion.id),
                    ..DataComponentPatchSummary::default()
                },
            },
        );
        assert_eq!(
            use_item_on(
                &mut empty,
                "minecraft:cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                151,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        let mundane =
            bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:mundane")
                .unwrap();
        set_main_hand_stack(
            &mut empty,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_POTION_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                    potion_id: Some(mundane.id),
                    ..DataComponentPatchSummary::default()
                },
            },
        );
        assert_eq!(
            use_item_on(
                &mut empty,
                "minecraft:cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                152,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );

        let mut water = block_use_world(
            "minecraft:water_cauldron",
            [("level", "2")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_main_hand(&mut water, VANILLA_ITEM_BUCKET_ID);
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                153,
            ),
            LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                bbb_protocol::block_types::VanillaBlockUseItemOnOverride::AbstractCauldronBlock
            ),
            "tag dispatch precedes direct item dispatch"
        );
        apply_cauldron_remove_dye_tag(&mut water, Vec::new());
        set_main_hand_stack(
            &mut water,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_POTION_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                    potion_id: Some(water_potion.id),
                    ..DataComponentPatchSummary::default()
                },
            },
        );
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                154,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_main_hand(&mut water, VANILLA_ITEM_BUCKET_ID);
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                155,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );
        set_test_block(
            &mut water,
            state_id_with_property("minecraft:water_cauldron", "level", "3"),
        );
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                156,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_main_hand_stack(
            &mut water,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_POTION_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                    potion_id: Some(water_potion.id),
                    ..DataComponentPatchSummary::default()
                },
            },
        );
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                157,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );
        set_main_hand(&mut water, VANILLA_ITEM_GLASS_BOTTLE_ID);
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                158,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert!(water.local_block_predictions().is_empty());
    }

    #[test]
    fn cauldron_tag_and_component_handlers_preserve_dispatch_priority() {
        let mut water = block_use_world(
            "minecraft:water_cauldron",
            [("level", "1")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        apply_cauldron_remove_dye_tag(&mut water, vec![VANILLA_ITEM_LEATHER_HELMET_ID]);
        set_main_hand_stack(
            &mut water,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_LEATHER_HELMET_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID],
                    dyed_color: Some(0x12_34_56),
                    ..DataComponentPatchSummary::default()
                },
            },
        );
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                157,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_main_hand(&mut water, VANILLA_ITEM_LEATHER_HELMET_ID);
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                158,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );

        set_main_hand_stack(
            &mut water,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_WHITE_BANNER_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID],
                    banner_pattern_layers: vec![BannerPatternLayerSummary {
                        registry_id: Some(0),
                        asset_id: None,
                        translation_key: None,
                        color_id: 0,
                    }],
                    ..DataComponentPatchSummary::default()
                },
            },
        );
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                159,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_main_hand(&mut water, VANILLA_ITEM_WHITE_BANNER_ID);
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                160,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );
        set_main_hand(&mut water, VANILLA_ITEM_WHITE_SHULKER_BOX_ID);
        assert_eq!(
            use_item_on(
                &mut water,
                "minecraft:water_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                161,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
    }

    #[test]
    fn cauldron_lava_and_powder_bucket_results_follow_water_above_and_level() {
        let mut empty =
            block_use_world_first_state("minecraft:cauldron", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        set_main_hand(&mut empty, VANILLA_ITEM_LAVA_BUCKET_ID);
        assert_eq!(
            use_item_on(
                &mut empty,
                "minecraft:cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                162,
            ),
            LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                bbb_protocol::block_types::VanillaBlockUseItemOnOverride::AbstractCauldronBlock
            )
        );
        let water_holder = empty
            .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:water")
            .unwrap();
        let flowing_water_holder = empty
            .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:flowing_water")
            .unwrap();
        empty.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:fluid".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:water".to_string(),
                    entries: vec![water_holder, flowing_water_holder],
                }],
            }],
        });
        assert_eq!(
            use_item_on(
                &mut empty,
                "minecraft:cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                163,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_block_at(
            &mut empty,
            above(POS),
            state_id("minecraft:water", [("level", "0")]),
        );
        assert_eq!(
            use_item_on(
                &mut empty,
                "minecraft:cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                164,
            ),
            success(LocalInteractionSuccess::consume(false))
        );

        let mut powder = block_use_world(
            "minecraft:powder_snow_cauldron",
            [("level", "2")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_main_hand(&mut powder, VANILLA_ITEM_BUCKET_ID);
        assert_eq!(
            use_item_on(
                &mut powder,
                "minecraft:powder_snow_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                165,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );
        set_test_block(
            &mut powder,
            state_id_with_property("minecraft:powder_snow_cauldron", "level", "3"),
        );
        assert_eq!(
            use_item_on(
                &mut powder,
                "minecraft:powder_snow_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                166,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_main_hand(&mut powder, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID);
        assert_eq!(
            use_item_on(
                &mut powder,
                "minecraft:powder_snow_cauldron",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                167,
            ),
            LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                bbb_protocol::block_types::VanillaBlockUseItemOnOverride::AbstractCauldronBlock
            ),
            "filled powder bucket still needs synchronized water fluid tag"
        );
    }

    #[test]
    fn tag_gated_item_on_owners_use_synchronized_item_tags() {
        let tags = UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![
                    TagNetworkPayload {
                        tag: "minecraft:bookshelf_books".to_string(),
                        entries: vec![VANILLA_ITEM_BOOK_ID],
                    },
                    TagNetworkPayload {
                        tag: "minecraft:lectern_books".to_string(),
                        entries: vec![VANILLA_ITEM_BOOK_ID],
                    },
                    TagNetworkPayload {
                        tag: "minecraft:noteblock_top_instruments".to_string(),
                        entries: vec![VANILLA_ITEM_NOTE_BLOCK_ID],
                    },
                ],
            }],
        };

        let mut shelf = block_use_world_with_entity(
            "minecraft:chiseled_bookshelf",
            [
                ("facing", "north"),
                ("slot_0_occupied", "false"),
                ("slot_1_occupied", "false"),
                ("slot_2_occupied", "false"),
                ("slot_3_occupied", "false"),
                ("slot_4_occupied", "false"),
                ("slot_5_occupied", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
        );
        shelf.apply_update_tags(tags.clone());
        set_main_hand(&mut shelf, VANILLA_ITEM_BOOK_ID);
        assert_eq!(
            use_item_on(
                &mut shelf,
                "minecraft:chiseled_bookshelf",
                InteractionHand::MainHand,
                Direction::North,
                [0.9, 0.75, 0.0],
                132,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        set_test_block(
            &mut shelf,
            state_id(
                "minecraft:chiseled_bookshelf",
                [
                    ("facing", "north"),
                    ("slot_0_occupied", "true"),
                    ("slot_1_occupied", "false"),
                    ("slot_2_occupied", "false"),
                    ("slot_3_occupied", "false"),
                    ("slot_4_occupied", "false"),
                    ("slot_5_occupied", "false"),
                ],
            ),
        );
        assert_eq!(
            use_item_on(
                &mut shelf,
                "minecraft:chiseled_bookshelf",
                InteractionHand::MainHand,
                Direction::North,
                [0.9, 0.75, 0.0],
                133,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );

        let mut lectern =
            block_use_world_first_state("minecraft:lectern", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        lectern.apply_update_tags(tags.clone());
        set_test_block(
            &mut lectern,
            state_id_with_property("minecraft:lectern", "has_book", "false"),
        );
        set_main_hand(&mut lectern, VANILLA_ITEM_BOOK_ID);
        assert_eq!(
            use_item_on(
                &mut lectern,
                "minecraft:lectern",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                134,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );

        let mut note = block_use_world_first_state(
            "minecraft:note_block",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        note.apply_update_tags(tags);
        set_main_hand(&mut note, VANILLA_ITEM_NOTE_BLOCK_ID);
        assert_eq!(
            use_item_on(
                &mut note,
                "minecraft:note_block",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                135,
            ),
            pass()
        );
        assert_eq!(
            use_item_on(
                &mut note,
                "minecraft:note_block",
                InteractionHand::MainHand,
                Direction::North,
                [0.5, 0.5, 0.0],
                136,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
        );
    }

    #[test]
    fn entity_and_data_driven_item_on_owners_keep_server_mutations_authoritative() {
        let mut pot = block_use_world_first_state_with_entity(
            "minecraft:decorated_pot",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
        );
        set_main_hand(&mut pot, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            use_item_on(
                &mut pot,
                "minecraft:decorated_pot",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                137,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );

        let mut shelf = block_use_world_with_entity(
            "minecraft:oak_shelf",
            [
                ("facing", "north"),
                ("powered", "false"),
                ("side_chain", "unconnected"),
                ("waterlogged", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID,
        );
        set_main_hand(&mut shelf, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            use_item_on(
                &mut shelf,
                "minecraft:oak_shelf",
                InteractionHand::MainHand,
                Direction::North,
                [0.5, 0.5, 0.0],
                138,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            use_item_on(
                &mut shelf,
                "minecraft:oak_shelf",
                InteractionHand::OffHand,
                Direction::North,
                [0.5, 0.5, 0.0],
                139,
            ),
            pass()
        );

        let mut campfire = block_use_world_first_state_with_entity(
            "minecraft:campfire",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
        );
        campfire.apply_update_recipes(UpdateRecipes {
            property_sets: vec![RecipePropertySetSummary {
                key: "minecraft:campfire_input".to_string(),
                item_ids: vec![VANILLA_ITEM_OAK_LOG_ID],
            }],
            stonecutter_recipes: Vec::new(),
        });
        set_main_hand(&mut campfire, VANILLA_ITEM_OAK_LOG_ID);
        assert_eq!(
            use_item_on(
                &mut campfire,
                "minecraft:campfire",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                140,
            ),
            success(LocalInteractionSuccess::consume(false))
        );

        let mut jukebox = block_use_world(
            "minecraft:jukebox",
            [("has_record", "false")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        jukebox.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_MUSIC_DISC_13_ID,
            BTreeSet::from([VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID]),
        )]));
        set_main_hand(&mut jukebox, VANILLA_ITEM_MUSIC_DISC_13_ID);
        assert_eq!(
            use_item_on(
                &mut jukebox,
                "minecraft:jukebox",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                141,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );

        let mut sign = block_use_world_first_state_with_entity(
            "minecraft:oak_sign",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        );
        set_main_hand(&mut sign, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            use_item_on(
                &mut sign,
                "minecraft:oak_sign",
                InteractionHand::MainHand,
                Direction::North,
                [0.5, 0.5, 0.0],
                142,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
    }

    #[test]
    fn stateful_item_on_owners_predict_vanilla_state_inventory_and_effect_actions() {
        let mut candle = block_use_world(
            "minecraft:candle",
            [("candles", "4"), ("lit", "true"), ("waterlogged", "false")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_item_on(
                &mut candle,
                "minecraft:candle",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                143,
            ),
            success(LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedCandleExtinguish),
                particle_action: Some(LocalInteractionParticleAction::CandleExtinguish {
                    pos: POS,
                    candles: 4,
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(block_property(&candle, "lit").as_deref(), Some("false"));

        let mut anchor = block_use_world(
            "minecraft:respawn_anchor",
            [("charges", "0")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        set_main_hand(&mut anchor, VANILLA_ITEM_GLOWSTONE_ID);
        assert_eq!(
            use_item_on(
                &mut anchor,
                "minecraft:respawn_anchor",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                144,
            ),
            success(LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedRespawnAnchorCharge),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(block_property(&anchor, "charges").as_deref(), Some("1"));
        assert!(anchor
            .local_item_in_hand(InteractionHand::MainHand)
            .is_none());

        let mut statue = block_use_world(
            "minecraft:waxed_copper_golem_statue",
            [
                ("facing", "north"),
                ("copper_golem_pose", "standing"),
                ("waterlogged", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_item_on(
                &mut statue,
                "minecraft:waxed_copper_golem_statue",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                145,
            ),
            success(LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedCopperGolemBecomeStatue),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(
            block_property(&statue, "copper_golem_pose").as_deref(),
            Some("sitting")
        );

        statue.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:axes".to_string(),
                    entries: vec![VANILLA_ITEM_IRON_AXE_ID],
                }],
            }],
        });
        set_main_hand(&mut statue, VANILLA_ITEM_IRON_AXE_ID);
        assert_eq!(
            use_item_on(
                &mut statue,
                "minecraft:waxed_copper_golem_statue",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                146,
            ),
            pass()
        );
        assert_eq!(
            block_property(&statue, "copper_golem_pose").as_deref(),
            Some("sitting")
        );
    }

    #[test]
    fn property_gated_server_owned_actions_return_exact_client_results() {
        for (name, property, active, inactive, active_result, inactive_result) in [
            (
                "minecraft:cave_vines_plant",
                "berries",
                "true",
                "false",
                success(LocalInteractionSuccess::client_item_interaction()),
                pass(),
            ),
            (
                "minecraft:composter",
                "level",
                "8",
                "7",
                success(LocalInteractionSuccess::client_item_interaction()),
                pass(),
            ),
            (
                "minecraft:lectern",
                "has_book",
                "true",
                "false",
                success(LocalInteractionSuccess::client_item_interaction()),
                pass(),
            ),
            (
                "minecraft:respawn_anchor",
                "charges",
                "1",
                "0",
                success(LocalInteractionSuccess::consume(false)),
                pass(),
            ),
            (
                "minecraft:sweet_berry_bush",
                "age",
                "2",
                "1",
                success(LocalInteractionSuccess::client_item_interaction()),
                pass(),
            ),
        ] {
            let mut world = block_use_world_first_state(name, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
            set_test_block(&mut world, state_id_with_property(name, property, active));
            assert_eq!(
                use_main_hand(&mut world, 83),
                active_result,
                "{name} active"
            );

            set_test_block(&mut world, state_id_with_property(name, property, inactive));
            assert_eq!(
                use_main_hand(&mut world, 84),
                inactive_result,
                "{name} inactive"
            );
            assert!(world.local_block_predictions().is_empty(), "{name}");
        }
    }

    #[test]
    fn cake_eating_updates_food_saturation_and_the_exact_predicted_state() {
        let mut cake = block_use_world(
            "minecraft:cake",
            [("bites", "0")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        cake.apply_player_health(PlayerHealth {
            health: 20.0,
            food: 18,
            saturation: 2.0,
        });
        assert_eq!(
            use_main_hand(&mut cake, 131),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(block_property(&cake, "bites").as_deref(), Some("1"));
        assert_eq!(
            cake.local_player().health,
            Some(LocalPlayerHealthState {
                health: 20.0,
                food: 20,
                saturation: 2.4,
            })
        );
        assert_eq!(cake.local_block_predictions()[0].sequence, 131);

        let mut last_slice = block_use_world(
            "minecraft:cake",
            [("bites", "6")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        last_slice.apply_player_health(PlayerHealth {
            health: 19.0,
            food: 17,
            saturation: 1.0,
        });
        assert_eq!(
            use_main_hand(&mut last_slice, 132),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            last_slice
                .probe_block(POS)
                .and_then(|block| block.block_name),
            Some("minecraft:air".to_string())
        );
        assert_eq!(last_slice.local_player().health.unwrap().food, 19);
    }

    #[test]
    fn full_food_cake_consumes_empty_hand_but_candle_cake_passes() {
        let mut cake = block_use_world(
            "minecraft:cake",
            [("bites", "0")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        cake.apply_player_health(PlayerHealth {
            health: 20.0,
            food: 20,
            saturation: 5.0,
        });
        assert_eq!(
            use_main_hand(&mut cake, 133),
            success(LocalInteractionSuccess::consume(false))
        );
        assert_eq!(block_property(&cake, "bites").as_deref(), Some("0"));
        assert!(cake.local_block_predictions().is_empty());

        let mut candle_cake = block_use_world(
            "minecraft:candle_cake",
            [("lit", "false")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        candle_cake.apply_player_health(PlayerHealth {
            health: 20.0,
            food: 20,
            saturation: 5.0,
        });
        assert_eq!(use_main_hand(&mut candle_cake, 134), pass());
        assert_eq!(
            candle_cake
                .probe_block(POS)
                .and_then(|block| block.block_name),
            Some("minecraft:candle_cake".to_string())
        );
    }

    #[test]
    fn candle_cake_eating_replaces_it_with_one_bite_of_cake() {
        let mut world = block_use_world(
            "minecraft:white_candle_cake",
            [("lit", "false")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        world.apply_player_health(PlayerHealth {
            health: 20.0,
            food: 16,
            saturation: 1.0,
        });
        assert_eq!(
            use_main_hand(&mut world, 135),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            world.probe_block(POS).and_then(|block| block.block_name),
            Some("minecraft:cake".to_string())
        );
        assert_eq!(block_property(&world, "bites").as_deref(), Some("1"));
        assert_eq!(world.local_player().health.unwrap().food, 18);
    }

    #[test]
    fn lit_candle_cake_hit_extinguishes_before_the_empty_hand_eat_path() {
        let mut world = block_use_world(
            "minecraft:red_candle_cake",
            [("lit", "true")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        world.apply_player_health(PlayerHealth {
            health: 20.0,
            food: 16,
            saturation: 1.0,
        });
        assert_eq!(
            world.predict_local_block_use_with_hit(
                POS,
                Direction::Up,
                [0.5, 0.75, 0.5],
                false,
                InteractionHand::MainHand,
                false,
                136,
            ),
            success(LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedCandleExtinguish),
                particle_action: Some(LocalInteractionParticleAction::CandleCakeExtinguish {
                    pos: POS,
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(block_property(&world, "lit").as_deref(), Some("false"));
        assert_eq!(world.local_player().health.unwrap().food, 16);
        assert_eq!(world.local_block_predictions()[0].sequence, 136);
    }

    #[test]
    fn candle_item_on_whole_cake_predicts_candle_cake_and_consumes_one() {
        let mut world = block_use_world(
            "minecraft:cake",
            [("bites", "0")],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_CANDLE_ID, 2),
        }));
        assert_eq!(
            use_main_hand(&mut world, 137),
            success(LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedCakeAddCandle),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(
            world.probe_block(POS).and_then(|block| block.block_name),
            Some("minecraft:candle_cake".to_string())
        );
        assert_eq!(block_property(&world, "lit").as_deref(), Some("false"));
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&item_stack(VANILLA_ITEM_CANDLE_ID, 1))
        );
    }

    #[test]
    fn flower_pot_empty_and_potted_branches_update_inventory_and_block_state() {
        let mut empty = block_use_world_first_state(
            "minecraft:flower_pot",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut empty, 141),
            success(LocalInteractionSuccess::consume(false))
        );
        assert!(empty.local_block_predictions().is_empty());

        let mut potted = block_use_world_first_state(
            "minecraft:potted_poppy",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            use_main_hand(&mut potted, 142),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            potted.probe_block(POS).and_then(|block| block.block_name),
            Some("minecraft:flower_pot".to_string())
        );
        assert_eq!(
            potted.local_item_in_hand(InteractionHand::MainHand),
            Some(&item_stack(VANILLA_ITEM_POPPY_ID, 1))
        );

        let mut plant = block_use_world_first_state(
            "minecraft:flower_pot",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert!(plant.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_POPPY_ID, 2),
        }));
        assert_eq!(
            use_main_hand(&mut plant, 144),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert_eq!(
            plant.probe_block(POS).and_then(|block| block.block_name),
            Some("minecraft:potted_poppy".to_string())
        );
        assert_eq!(
            plant.local_item_in_hand(InteractionHand::MainHand),
            Some(&item_stack(VANILLA_ITEM_POPPY_ID, 1))
        );

        assert_eq!(
            use_main_hand(&mut plant, 145),
            success(LocalInteractionSuccess::consume(false)),
            "a pottable item against an occupied pot consumes without replacing it"
        );
        assert_eq!(
            plant.probe_block(POS).and_then(|block| block.block_name),
            Some("minecraft:potted_poppy".to_string())
        );
    }

    #[test]
    fn potted_flower_full_inventory_requests_vanilla_drop_swing() {
        let mut world = block_use_world_first_state(
            "minecraft:potted_poppy",
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        for slot in (0..36).chain(std::iter::once(40)) {
            assert!(world.apply_set_player_inventory(SetPlayerInventory {
                slot,
                item: item_stack(VANILLA_ITEM_STONE_ID, 64),
            }));
        }
        assert_eq!(
            use_main_hand(&mut world, 143),
            success(LocalInteractionSuccess {
                extra_swing: Some(InteractionHand::MainHand),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(
            world.probe_block(POS).and_then(|block| block.block_name),
            Some("minecraft:flower_pot".to_string())
        );
    }

    #[test]
    fn every_generated_potted_flower_owner_maps_back_to_its_block_item() {
        let mut owner_count = 0;
        for block_type in VANILLA_BLOCK_TYPES.iter().filter(|block_type| {
            block_type.use_without_item_override
                == Some(VanillaBlockUseWithoutItemOverride::FlowerPotBlock)
        }) {
            owner_count += 1;
            if block_type.resource_id == "minecraft:flower_pot" {
                assert_eq!(potted_flower_item_id(block_type), None);
            } else {
                assert!(
                    potted_flower_item_id(block_type).is_some(),
                    "{}",
                    block_type.resource_id
                );
            }
        }
        assert!(owner_count > 1);
        assert_eq!(
            potted_flower_item_id(&VANILLA_BLOCK_TYPES[VANILLA_BLOCK_POTTED_POPPY_ID as usize]),
            Some(VANILLA_ITEM_POPPY_ID)
        );
    }

    #[test]
    fn offhand_skips_empty_hand_block_owner_and_main_hand_pass_continues_dispatch() {
        let mut world = block_use_world(
            "minecraft:oak_door",
            [
                ("facing", "north"),
                ("half", "lower"),
                ("hinge", "left"),
                ("open", "false"),
                ("powered", "false"),
            ],
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        );
        assert_eq!(
            world.predict_local_block_use(POS, Direction::Up, InteractionHand::OffHand, false, 1,),
            pass()
        );
        assert_eq!(block_property(&world, "open"), Some("false".to_string()));
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
        world.predict_local_block_use_item_on(
            owner,
            block_type,
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
                    record.sign_text = Some(crate::SignBlockEntityTextState::default());
                }
                vec![record]
            })
            .unwrap_or_default();
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..section_count)
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value(PaletteDomain::BlockStates, 4096, air),
                    biomes: single_value(PaletteDomain::Biomes, 64, 0),
                })
                .collect(),
            block_entities,
            light: LightData::default(),
        });
        set_test_block(&mut world, state_id_vec(name, properties));
        world
    }

    fn set_test_block(world: &mut WorldStore, block_state_id: i32) {
        set_block_at(world, POS, block_state_id);
    }

    fn set_block_at(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
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
