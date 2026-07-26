use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::vanilla_block_state_support_facts,
    block_types::{
        vanilla_block_type_for_resource_id, VanillaBlockAnalogOutputOverride,
        VanillaBlockRedstoneSignalOverride, VanillaBlockUseWithoutItemOverride,
    },
    item_types::vanilla_item_type_for_resource_id,
    packets::{DataComponentPatchSummary, Direction},
};

use super::{
    copper_golem_pose_ordinal, LocalBlockUseContext, LocalInteractionResult,
    LocalInteractionSuccess, LocalTargetUsePrediction,
};
use crate::{
    chunks::{decode_nbt_root, find_nbt_entry, NbtValue},
    BlockPos, BlockProbe, ItemFrameFacing, WorldStore,
};

impl WorldStore {
    pub(super) fn predict_local_comparator_use(
        &mut self,
        context: LocalBlockUseContext,
        block_name: &str,
        mut properties: BTreeMap<String, String>,
        owner: VanillaBlockUseWithoutItemOverride,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
        let mode = match properties.get("mode").map(String::as_str) {
            Some("compare") => "subtract",
            Some("subtract") => "compare",
            _ => return unsupported(),
        };
        let Some(facing) = direction_property(&properties, "facing").filter(is_horizontal) else {
            return unsupported();
        };
        let Some(was_powered) = boolean_property(&properties, "powered") else {
            return unsupported();
        };
        properties.insert("mode".to_string(), mode.to_string());

        let Some(input) = self.comparator_input_signal(context.pos, facing) else {
            return unsupported();
        };
        let Some(side_input) = self.comparator_alternate_signal(context.pos, facing) else {
            return unsupported();
        };
        let output = if input == 0 || side_input > input {
            0
        } else if mode == "subtract" {
            input - side_input
        } else {
            input
        };
        let old_output = self.comparator_output_at(context.pos).unwrap_or(0);
        if old_output != output || mode == "compare" {
            let should_power =
                input != 0 && (input > side_input || (input == side_input && mode == "compare"));
            if should_power != was_powered {
                properties.insert("powered".to_string(), should_power.to_string());
            }
        }

        let Some(state_id) = self
            .registries
            .block_state_id_by_name_and_properties(block_name, &properties)
        else {
            return unsupported();
        };
        if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
            return unsupported();
        }
        let _ = self.set_comparator_output_at(context.pos, output);
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }

    fn comparator_input_signal(&self, pos: BlockPos, facing: Direction) -> Option<i32> {
        let target_pos = relative(pos, facing);
        let mut signal = self.redstone_signal_at(target_pos, facing)?;
        if signal < 15 {
            let target = self.probe_block(target_pos)?;
            if target.block_name.as_deref() == Some("minecraft:redstone_wire") {
                signal = signal.max(integer_property(&target, "power", 0..=15)?);
            }
        }

        let target = self.probe_block(target_pos)?;
        let target_type = vanilla_block_type_for_resource_id(target.block_name.as_deref()?)?;
        if let Some(owner) = target_type.analog_output_override {
            return self.analog_output_signal(target_pos, &target, owner, opposite(facing));
        }
        if signal >= 15 || !self.redstone_conductor(&target)? {
            return Some(signal);
        }

        let second_pos = relative(target_pos, facing);
        let second = self.probe_block(second_pos)?;
        let second_type = vanilla_block_type_for_resource_id(second.block_name.as_deref()?)?;
        let block_signal = if let Some(owner) = second_type.analog_output_override {
            Some(self.analog_output_signal(second_pos, &second, owner, opposite(facing))?)
        } else {
            None
        };
        let frame_signal = self
            .entities
            .comparator_item_frame_analog_output(second_pos, item_frame_facing(facing))?;
        Some(
            block_signal
                .into_iter()
                .chain(frame_signal)
                .max()
                .unwrap_or(signal),
        )
    }

    fn comparator_alternate_signal(&self, pos: BlockPos, facing: Direction) -> Option<i32> {
        let clockwise = clockwise(facing)?;
        let counter_clockwise = opposite(clockwise);
        Some(
            self.redstone_control_input(relative(pos, clockwise), clockwise)?
                .max(
                    self.redstone_control_input(
                        relative(pos, counter_clockwise),
                        counter_clockwise,
                    )?,
                ),
        )
    }

    fn redstone_control_input(&self, pos: BlockPos, direction: Direction) -> Option<i32> {
        let block = self.probe_block(pos)?;
        let block_type = vanilla_block_type_for_resource_id(block.block_name.as_deref()?)?;
        if block_type.resource_id == "minecraft:redstone_block" {
            return Some(15);
        }
        if block_type.resource_id == "minecraft:redstone_wire" {
            return integer_property(&block, "power", 0..=15);
        }
        if block_type.is_signal_source_override.is_none() {
            return Some(0);
        }
        self.direct_redstone_signal(pos, &block, direction)
    }

    fn redstone_signal_at(&self, pos: BlockPos, direction: Direction) -> Option<i32> {
        let block = self.probe_block(pos)?;
        let weak = self.weak_redstone_signal(pos, &block, direction)?;
        if !self.redstone_conductor(&block)? {
            return Some(weak);
        }
        let mut direct = 0;
        for neighbor_direction in DIRECTIONS {
            let neighbor_pos = relative(pos, neighbor_direction);
            let neighbor = self.probe_block(neighbor_pos)?;
            direct = direct.max(self.direct_redstone_signal(
                neighbor_pos,
                &neighbor,
                neighbor_direction,
            )?);
            if direct >= 15 {
                break;
            }
        }
        Some(weak.max(direct))
    }

    fn weak_redstone_signal(
        &self,
        pos: BlockPos,
        block: &BlockProbe,
        direction: Direction,
    ) -> Option<i32> {
        let block_type = vanilla_block_type_for_resource_id(block.block_name.as_deref()?)?;
        let Some(owner) = block_type.signal_override else {
            return Some(0);
        };
        match owner {
            VanillaBlockRedstoneSignalOverride::BasePressurePlateBlock => {
                pressure_plate_signal(block)
            }
            VanillaBlockRedstoneSignalOverride::ButtonBlock
            | VanillaBlockRedstoneSignalOverride::LeverBlock => {
                Some(if boolean_property(&block.block_properties, "powered")? {
                    15
                } else {
                    0
                })
            }
            VanillaBlockRedstoneSignalOverride::CalibratedSculkSensorBlock => {
                if direction == direction_property(&block.block_properties, "facing")? {
                    Some(0)
                } else {
                    integer_property(block, "power", 0..=15)
                }
            }
            VanillaBlockRedstoneSignalOverride::DaylightDetectorBlock => {
                integer_property(block, "power", 0..=15)
            }
            VanillaBlockRedstoneSignalOverride::DetectorRailBlock
            | VanillaBlockRedstoneSignalOverride::TripWireHookBlock => powered_signal(block),
            VanillaBlockRedstoneSignalOverride::DiodeBlock => {
                if !boolean_property(&block.block_properties, "powered")?
                    || direction_property(&block.block_properties, "facing")? != direction
                {
                    return Some(0);
                }
                if block_type.resource_id == "minecraft:comparator" {
                    Some(self.comparator_output_at(pos).unwrap_or(0).clamp(0, 15))
                } else {
                    Some(15)
                }
            }
            VanillaBlockRedstoneSignalOverride::JukeboxBlock => Some(
                if self
                    .playing_jukebox_songs()
                    .iter()
                    .any(|song| song.pos == pos)
                {
                    15
                } else {
                    0
                },
            ),
            VanillaBlockRedstoneSignalOverride::LecternBlock => powered_signal(block),
            VanillaBlockRedstoneSignalOverride::LightningRodBlock
            | VanillaBlockRedstoneSignalOverride::ObserverBlock => powered_signal(block),
            VanillaBlockRedstoneSignalOverride::PoweredBlock => Some(15),
            VanillaBlockRedstoneSignalOverride::RedStoneWireBlock => {
                self.redstone_wire_signal(pos, block, direction)
            }
            VanillaBlockRedstoneSignalOverride::RedstoneTorchBlock => Some(
                if boolean_property(&block.block_properties, "lit")? && direction != Direction::Up {
                    15
                } else {
                    0
                },
            ),
            VanillaBlockRedstoneSignalOverride::RedstoneWallTorchBlock => Some(
                if boolean_property(&block.block_properties, "lit")?
                    && direction_property(&block.block_properties, "facing")? != direction
                {
                    15
                } else {
                    0
                },
            ),
            VanillaBlockRedstoneSignalOverride::SculkSensorBlock => {
                integer_property(block, "power", 0..=15)
            }
            VanillaBlockRedstoneSignalOverride::TargetBlock => {
                integer_property(block, "power", 0..=15)
            }
            VanillaBlockRedstoneSignalOverride::TestBlock => Some(
                if block.block_properties.get("mode").map(String::as_str) == Some("start")
                    && self
                        .test_block_data_at(pos)
                        .is_some_and(|data| data.powered)
                {
                    15
                } else {
                    0
                },
            ),
            VanillaBlockRedstoneSignalOverride::TrappedChestBlock => {
                Some(i32::from(self.chest_open_count_at(pos)).clamp(0, 15))
            }
        }
    }

    fn direct_redstone_signal(
        &self,
        pos: BlockPos,
        block: &BlockProbe,
        direction: Direction,
    ) -> Option<i32> {
        let block_type = vanilla_block_type_for_resource_id(block.block_name.as_deref()?)?;
        let Some(owner) = block_type.direct_signal_override else {
            return Some(0);
        };
        match owner {
            VanillaBlockRedstoneSignalOverride::BasePressurePlateBlock
            | VanillaBlockRedstoneSignalOverride::DetectorRailBlock
            | VanillaBlockRedstoneSignalOverride::LecternBlock
            | VanillaBlockRedstoneSignalOverride::SculkSensorBlock
            | VanillaBlockRedstoneSignalOverride::TrappedChestBlock => {
                if direction == Direction::Up {
                    self.weak_redstone_signal(pos, block, direction)
                } else {
                    Some(0)
                }
            }
            VanillaBlockRedstoneSignalOverride::ButtonBlock
            | VanillaBlockRedstoneSignalOverride::LeverBlock => {
                if direction == connected_direction(block)? {
                    self.weak_redstone_signal(pos, block, direction)
                } else {
                    Some(0)
                }
            }
            VanillaBlockRedstoneSignalOverride::DiodeBlock
            | VanillaBlockRedstoneSignalOverride::RedStoneWireBlock => {
                self.weak_redstone_signal(pos, block, direction)
            }
            VanillaBlockRedstoneSignalOverride::LightningRodBlock
            | VanillaBlockRedstoneSignalOverride::ObserverBlock
            | VanillaBlockRedstoneSignalOverride::TripWireHookBlock => {
                if direction == direction_property(&block.block_properties, "facing")? {
                    self.weak_redstone_signal(pos, block, direction)
                } else {
                    Some(0)
                }
            }
            VanillaBlockRedstoneSignalOverride::RedstoneTorchBlock => {
                if direction == Direction::Down {
                    self.weak_redstone_signal(pos, block, direction)
                } else {
                    Some(0)
                }
            }
            VanillaBlockRedstoneSignalOverride::CalibratedSculkSensorBlock
            | VanillaBlockRedstoneSignalOverride::DaylightDetectorBlock
            | VanillaBlockRedstoneSignalOverride::JukeboxBlock
            | VanillaBlockRedstoneSignalOverride::PoweredBlock
            | VanillaBlockRedstoneSignalOverride::RedstoneWallTorchBlock
            | VanillaBlockRedstoneSignalOverride::TargetBlock
            | VanillaBlockRedstoneSignalOverride::TestBlock => Some(0),
        }
    }

    fn redstone_conductor(&self, block: &BlockProbe) -> Option<bool> {
        Some(vanilla_block_state_support_facts(block.block_state_id)?.redstone_conductor())
    }

    fn redstone_wire_signal(
        &self,
        pos: BlockPos,
        block: &BlockProbe,
        direction: Direction,
    ) -> Option<i32> {
        if direction == Direction::Down {
            return Some(0);
        }
        let power = integer_property(block, "power", 0..=15)?;
        if power == 0 || direction == Direction::Up {
            return Some(power);
        }

        let was_dot = super::redstone_wire::wire_connected_sides(&block.block_properties)?
            .iter()
            .all(|connected| !connected);
        let mut connection_seed = block.block_properties.clone();
        for property in ["north", "east", "south", "west"] {
            connection_seed.insert(property.to_string(), "none".to_string());
        }
        let connections =
            self.local_redstone_wire_connection_state(pos, connection_seed, was_dot)?;
        match connections
            .get(direction_name(opposite(direction)))?
            .as_str()
        {
            "side" | "up" => Some(power),
            "none" => Some(0),
            _ => None,
        }
    }

    fn analog_output_signal(
        &self,
        pos: BlockPos,
        block: &BlockProbe,
        owner: VanillaBlockAnalogOutputOverride,
        _direction: Direction,
    ) -> Option<i32> {
        match owner {
            VanillaBlockAnalogOutputOverride::AbstractFurnaceBlock => {
                self.container_block_entity_signal(pos, 3)
            }
            VanillaBlockAnalogOutputOverride::BarrelBlock => {
                self.container_block_entity_signal(pos, 27)
            }
            VanillaBlockAnalogOutputOverride::BeehiveBlock => {
                integer_property(block, "honey_level", 0..=5)
            }
            VanillaBlockAnalogOutputOverride::BlockBehaviour => Some(0),
            VanillaBlockAnalogOutputOverride::BrewingStandBlock => {
                self.container_block_entity_signal(pos, 5)
            }
            VanillaBlockAnalogOutputOverride::CakeBlock => {
                Some((7 - integer_property(block, "bites", 0..=6)?) * 2)
            }
            VanillaBlockAnalogOutputOverride::CandleCakeBlock => Some(14),
            VanillaBlockAnalogOutputOverride::ChestBlock => self.chest_container_signal(pos, block),
            VanillaBlockAnalogOutputOverride::ChiseledBookShelfBlock
            | VanillaBlockAnalogOutputOverride::CreakingHeartBlock
            | VanillaBlockAnalogOutputOverride::DetectorRailBlock
            | VanillaBlockAnalogOutputOverride::ShelfBlock => Some(0),
            VanillaBlockAnalogOutputOverride::CommandBlock => Some(
                self.command_block_data_at(pos)
                    .map_or(0, |data| data.success_count),
            ),
            VanillaBlockAnalogOutputOverride::ComposterBlock => {
                integer_property(block, "level", 0..=8)
            }
            VanillaBlockAnalogOutputOverride::CopperBulbBlock => {
                Some(if boolean_property(&block.block_properties, "lit")? {
                    15
                } else {
                    0
                })
            }
            VanillaBlockAnalogOutputOverride::CopperGolemStatueBlock => Some(
                copper_golem_pose_ordinal(
                    block.block_properties.get("copper_golem_pose")?.as_str(),
                )? + 1,
            ),
            VanillaBlockAnalogOutputOverride::CrafterBlock => self.crafter_signal(pos),
            VanillaBlockAnalogOutputOverride::DecoratedPotBlock => self.decorated_pot_signal(pos),
            VanillaBlockAnalogOutputOverride::DispenserBlock => {
                self.container_block_entity_signal(pos, 9)
            }
            VanillaBlockAnalogOutputOverride::EndPortalFrameBlock => {
                Some(if boolean_property(&block.block_properties, "eye")? {
                    15
                } else {
                    0
                })
            }
            VanillaBlockAnalogOutputOverride::HopperBlock => {
                self.container_block_entity_signal(pos, 5)
            }
            VanillaBlockAnalogOutputOverride::JukeboxBlock => self.jukebox_signal(pos, block),
            VanillaBlockAnalogOutputOverride::LavaCauldronBlock => Some(3),
            VanillaBlockAnalogOutputOverride::LayeredCauldronBlock => {
                integer_property(block, "level", 1..=3)
            }
            VanillaBlockAnalogOutputOverride::LecternBlock => self.lectern_signal(pos, block),
            VanillaBlockAnalogOutputOverride::RespawnAnchorBlock => {
                Some(integer_property(block, "charges", 0..=4)? * 15 / 4)
            }
            VanillaBlockAnalogOutputOverride::SculkSensorBlock => {
                self.sculk_sensor_signal(pos, block)
            }
            VanillaBlockAnalogOutputOverride::ShulkerBoxBlock => {
                self.container_block_entity_signal(pos, 27)
            }
        }
    }

    fn container_block_entity_signal(&self, pos: BlockPos, size: usize) -> Option<i32> {
        let Some(raw_nbt) = self.block_entity_raw_nbt_at(pos) else {
            return Some(0);
        };
        let slots = self.container_slots(raw_nbt, size)?;
        Some(container_signal(&slots))
    }

    fn chest_container_signal(&self, pos: BlockPos, block: &BlockProbe) -> Option<i32> {
        if self.chest_is_blocked(pos)? {
            return Some(0);
        }
        let own = if let Some(raw_nbt) = self.block_entity_raw_nbt_at(pos) {
            self.container_slots(raw_nbt, 27)?
        } else {
            vec![None; 27]
        };
        match block.block_properties.get("type").map(String::as_str) {
            Some("single") => Some(container_signal(&own)),
            Some("left" | "right") => {
                let facing = direction_property(&block.block_properties, "facing")?;
                let partner_direction =
                    if block.block_properties.get("type").map(String::as_str) == Some("left") {
                        clockwise(facing)?
                    } else {
                        opposite(clockwise(facing)?)
                    };
                let partner_pos = relative(pos, partner_direction);
                let partner = self.probe_block(partner_pos)?;
                let expected_partner_type =
                    if block.block_properties.get("type").map(String::as_str) == Some("left") {
                        "right"
                    } else {
                        "left"
                    };
                if partner.block_name.as_deref() != block.block_name.as_deref()
                    || partner.block_properties.get("facing")
                        != block.block_properties.get("facing")
                    || partner.block_properties.get("type").map(String::as_str)
                        != Some(expected_partner_type)
                    || self.chest_is_blocked(partner_pos)?
                {
                    return Some(0);
                }
                let partner_slots = if let Some(raw_nbt) = self.block_entity_raw_nbt_at(partner_pos)
                {
                    self.container_slots(raw_nbt, 27)?
                } else {
                    vec![None; 27]
                };
                let slots = own.into_iter().chain(partner_slots).collect::<Vec<_>>();
                Some(container_signal(&slots))
            }
            _ => None,
        }
    }

    fn chest_is_blocked(&self, pos: BlockPos) -> Option<bool> {
        let above = self.probe_block(relative(pos, Direction::Up))?;
        Some(
            self.redstone_conductor(&above)?
                || self.entities.sitting_cat_intersects_chest_top(pos)?,
        )
    }

    fn crafter_signal(&self, pos: BlockPos) -> Option<i32> {
        let Some(raw_nbt) = self.block_entity_raw_nbt_at(pos) else {
            return Some(0);
        };
        let root = decode_nbt_compound(raw_nbt)?;
        let slots = container_slots_from_root(self, &root, 9);
        let mut occupied = slots.iter().filter(|slot| slot.is_some()).count();
        if let Some(NbtValue::IntArray(disabled)) = find_nbt_entry(&root, "disabled_slots") {
            for index in disabled {
                if (0..9).contains(index) && slots[usize::try_from(*index).ok()?].is_none() {
                    occupied += 1;
                }
            }
        }
        i32::try_from(occupied).ok()
    }

    fn decorated_pot_signal(&self, pos: BlockPos) -> Option<i32> {
        let Some(raw_nbt) = self.block_entity_raw_nbt_at(pos) else {
            return Some(0);
        };
        let root = decode_nbt_compound(raw_nbt)?;
        let Some(NbtValue::Compound(item)) = find_nbt_entry(&root, "item") else {
            return Some(0);
        };
        let slot = item_stack_fullness(self, item)?;
        Some(container_signal(&[slot]))
    }

    fn jukebox_signal(&self, pos: BlockPos, block: &BlockProbe) -> Option<i32> {
        if !boolean_property(&block.block_properties, "has_record")? {
            return Some(0);
        }
        let raw_nbt = self.block_entity_raw_nbt_at(pos)?;
        let root = decode_nbt_compound(raw_nbt)?;
        let NbtValue::Compound(record_item) = find_nbt_entry(&root, "RecordItem")? else {
            return None;
        };
        let song_id = find_nbt_entry(record_item, "components")
            .and_then(|components| match components {
                NbtValue::Compound(components) => {
                    find_nbt_entry(components, "minecraft:jukebox_playable")
                }
                _ => None,
            })
            .and_then(|song| match song {
                NbtValue::String(resource_id) => {
                    self.registry_holder_id_by_resource_id("minecraft:jukebox_song", resource_id)
                }
                _ => None,
            })
            .or_else(|| {
                let NbtValue::String(resource_id) = find_nbt_entry(record_item, "id")? else {
                    return None;
                };
                let item_id = vanilla_item_type_for_resource_id(resource_id)?.id;
                self.items
                    .effective_component_patch(item_id, &DataComponentPatchSummary::default())
                    .and_then(|patch| patch.jukebox_song_id)
            })?;
        self.synchronized_jukebox_song_presentation(song_id)
            .ok()
            .flatten()
            .map(|song| song.comparator_output)
    }

    fn lectern_signal(&self, pos: BlockPos, block: &BlockProbe) -> Option<i32> {
        if !boolean_property(&block.block_properties, "has_book")? {
            return Some(0);
        }
        let raw_nbt = self.block_entity_raw_nbt_at(pos)?;
        let root = decode_nbt_compound(raw_nbt)?;
        let NbtValue::Compound(book) = find_nbt_entry(&root, "Book")? else {
            return None;
        };
        let page_count = find_nbt_entry(book, "components")
            .and_then(|components| match components {
                NbtValue::Compound(components) => [
                    "minecraft:written_book_content",
                    "minecraft:writable_book_content",
                ]
                .into_iter()
                .find_map(|key| find_nbt_entry(components, key)),
                _ => None,
            })
            .and_then(|content| match content {
                NbtValue::Compound(content) => find_nbt_entry(content, "pages"),
                _ => None,
            })
            .and_then(|pages| match pages {
                NbtValue::List(pages) => i32::try_from(pages.len()).ok(),
                _ => None,
            })
            .unwrap_or(0);
        let page = find_nbt_entry(&root, "Page")
            .and_then(nbt_int)
            .unwrap_or(0)
            .clamp(0, page_count.saturating_sub(1));
        let page_progress = if page_count > 1 {
            page as f32 / (page_count - 1) as f32
        } else {
            1.0
        };
        Some((page_progress * 14.0).floor() as i32 + 1)
    }

    fn sculk_sensor_signal(&self, pos: BlockPos, block: &BlockProbe) -> Option<i32> {
        if block
            .block_properties
            .get("sculk_sensor_phase")
            .map(String::as_str)
            != Some("active")
        {
            return Some(0);
        }
        let raw_nbt = self.block_entity_raw_nbt_at(pos)?;
        let root = decode_nbt_compound(raw_nbt)?;
        Some(
            find_nbt_entry(&root, "last_vibration_frequency")
                .and_then(nbt_int)
                .unwrap_or(0)
                .clamp(0, 15),
        )
    }

    fn container_slots(&self, raw_nbt: &[u8], size: usize) -> Option<Vec<Option<(i32, i32)>>> {
        let root = decode_nbt_compound(raw_nbt)?;
        Some(container_slots_from_root(self, &root, size))
    }
}

const DIRECTIONS: [Direction; 6] = [
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

fn container_slots_from_root(
    world: &WorldStore,
    root: &[(String, NbtValue)],
    size: usize,
) -> Vec<Option<(i32, i32)>> {
    let mut slots = vec![None; size];
    if find_nbt_entry(root, "LootTable").is_some() {
        return slots;
    }
    let Some(NbtValue::List(items)) = find_nbt_entry(root, "Items") else {
        return slots;
    };
    for item in items {
        let NbtValue::Compound(item) = item else {
            continue;
        };
        let Some(slot) = find_nbt_entry(item, "Slot")
            .and_then(nbt_int)
            .and_then(|slot| usize::try_from(slot).ok())
            .filter(|slot| *slot < size)
        else {
            continue;
        };
        slots[slot] = item_stack_fullness(world, item).flatten();
    }
    slots
}

fn item_stack_fullness(
    world: &WorldStore,
    item: &[(String, NbtValue)],
) -> Option<Option<(i32, i32)>> {
    let Some(NbtValue::String(resource_id)) = find_nbt_entry(item, "id") else {
        return Some(None);
    };
    let Some(item_type) = vanilla_item_type_for_resource_id(resource_id) else {
        return Some(None);
    };
    let count = find_nbt_entry(item, "count").and_then(nbt_int).unwrap_or(1);
    if !(1..=99).contains(&count) {
        return Some(None);
    }
    let component_max = find_nbt_entry(item, "components")
        .and_then(|value| match value {
            NbtValue::Compound(entries) => find_nbt_entry(entries, "minecraft:max_stack_size"),
            _ => None,
        })
        .and_then(nbt_int);
    let max_stack_size = component_max
        .filter(|value| (1..=99).contains(value))
        .unwrap_or_else(|| world.item_max_stack_size_for_protocol_id(item_type.id))
        .clamp(1, 99);
    if count > max_stack_size {
        return Some(None);
    }
    Some(Some((count, max_stack_size)))
}

fn decode_nbt_compound(raw_nbt: &[u8]) -> Option<Vec<(String, NbtValue)>> {
    if raw_nbt.is_empty() {
        return Some(Vec::new());
    }
    match decode_nbt_root(raw_nbt).ok().flatten()? {
        NbtValue::Compound(entries) => Some(entries),
        _ => None,
    }
}

fn container_signal(slots: &[Option<(i32, i32)>]) -> i32 {
    if slots.is_empty() {
        return 0;
    }
    let total_percent = slots.iter().flatten().fold(0.0_f32, |total, (count, max)| {
        total + *count as f32 / *max as f32
    }) / slots.len() as f32;
    (total_percent * 14.0).floor() as i32 + i32::from(total_percent > 0.0)
}

fn nbt_int(value: &NbtValue) -> Option<i32> {
    match value {
        NbtValue::Byte(value) => Some(i32::from(*value)),
        NbtValue::Short(value) => Some(i32::from(*value)),
        NbtValue::Int(value) => Some(*value),
        _ => None,
    }
}

fn pressure_plate_signal(block: &BlockProbe) -> Option<i32> {
    if block.block_properties.contains_key("power") {
        integer_property(block, "power", 0..=15)
    } else {
        powered_signal(block)
    }
}

fn powered_signal(block: &BlockProbe) -> Option<i32> {
    Some(if boolean_property(&block.block_properties, "powered")? {
        15
    } else {
        0
    })
}

fn connected_direction(block: &BlockProbe) -> Option<Direction> {
    match block.block_properties.get("face")?.as_str() {
        "ceiling" => Some(Direction::Down),
        "floor" => Some(Direction::Up),
        "wall" => direction_property(&block.block_properties, "facing"),
        _ => None,
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
    block: &BlockProbe,
    property: &str,
    range: std::ops::RangeInclusive<i32>,
) -> Option<i32> {
    block
        .block_properties
        .get(property)?
        .parse::<i32>()
        .ok()
        .filter(|value| range.contains(value))
}

fn direction_property(properties: &BTreeMap<String, String>, property: &str) -> Option<Direction> {
    match properties.get(property)?.as_str() {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

fn is_horizontal(direction: &Direction) -> bool {
    matches!(
        direction,
        Direction::North | Direction::South | Direction::West | Direction::East
    )
}

fn clockwise(direction: Direction) -> Option<Direction> {
    match direction {
        Direction::North => Some(Direction::East),
        Direction::East => Some(Direction::South),
        Direction::South => Some(Direction::West),
        Direction::West => Some(Direction::North),
        Direction::Down | Direction::Up => None,
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    match direction {
        Direction::Down => BlockPos {
            y: pos.y - 1,
            ..pos
        },
        Direction::Up => BlockPos {
            y: pos.y + 1,
            ..pos
        },
        Direction::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        Direction::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        Direction::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        Direction::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

fn item_frame_facing(direction: Direction) -> ItemFrameFacing {
    match direction {
        Direction::Down => ItemFrameFacing::Down,
        Direction::Up => ItemFrameFacing::Up,
        Direction::North => ItemFrameFacing::North,
        Direction::South => ItemFrameFacing::South,
        Direction::West => ItemFrameFacing::West,
        Direction::East => ItemFrameFacing::East,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_signal_matches_vanilla_lerp_discrete() {
        assert_eq!(container_signal(&[None; 5]), 0);
        assert_eq!(
            container_signal(&[Some((1, 64)), None, None, None, None]),
            1
        );
        assert_eq!(container_signal(&[Some((64, 64)); 5]), 15);
        assert_eq!(container_signal(&[Some((32, 64)), Some((1, 1)), None]), 8);
    }
}
