//! The brewing stand: `BrewingStandBlockEntity.serverTick`
//! (`BrewingStandBlockEntity.java:97-139`) driven by the live block ticker,
//! and `BrewingStandBlock`'s menu (`BrewingStandMenu.java:41-55`).

use crate::world::LevelId;
use std::collections::BTreeSet;
use std::sync::Arc;

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_BREWING_STAND_ID,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};

use crate::block_behavior::level::SET_BLOCK_UPDATE_CLIENTS;
use crate::block_entity::brewing_stand::{BrewingStandTickFacts, SLOT_FUEL, SLOT_INGREDIENT};
use crate::block_entity::BlockEntity;
use crate::connection::ConnectionId;
use crate::inventory::MenuLayout;
use crate::potion_brewing::PotionBrewing;
use bbb_server_datapack::recipe::ingredient::crafting_remainder;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::world::{ServerWorld, WorldEvent};

use super::BlockUseContext;

/// `BrewingStandBlockEntity.HAS_BOTTLE` (`BrewingStandBlock.java:34-36`) over
/// `BlockStateProperties.HAS_BOTTLE_0` and its two siblings.
const HAS_BOTTLE: [&str; 3] = ["has_bottle_0", "has_bottle_1", "has_bottle_2"];

/// `LevelEvent.BREWING_STAND_BREW` (`BrewingStandBlockEntity.java:192`).
const BREWING_STAND_BREW_EVENT: i32 = 1035;

/// `BrewingStandBlock`'s `useWithoutItem` owner: the generated class tag
/// names the one block.
#[must_use]
pub(super) fn is_brewing_stand(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::BrewingStandBlock)
}

/// `BrewingStandBlock.useWithoutItem` (`BrewingStandBlock.java:65-74`), whose
/// body is the block entity's menu plus `INTERACT_WITH_BREWINGSTAND`.
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_brewing_stand_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}

/// The `MenuProvider` a spectator reaches: `BaseEntityBlock.getMenuProvider`
/// (`BaseEntityBlock.java:30-33`) answering the block entity itself.
pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.open_brewing_stand_menu(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}

/// The slots a hopper reaches on one face of a brewing stand
/// (`BrewingStandBlockEntity.java:31-33`, `:229-246`).
pub(in crate::world) struct BrewingStandHopperFace {
    insert_slots: &'static [usize],
    take_slots: &'static [usize],
    /// `canPlaceItem`'s bottle rule (`:224-225`): only an empty slot takes a
    /// bottle.
    bottle_slots_occupied: [bool; 3],
    /// The live `minecraft:brewing_fuel` members (`:223`).
    brewing_fuel: Arc<BTreeSet<i32>>,
}

/// `BrewingStandBlockEntity.SLOTS_FOR_UP` / `_DOWN` / `_SIDES` (`:31-33`).
const SLOTS_FOR_UP: &[usize] = &[SLOT_INGREDIENT];
const SLOTS_FOR_DOWN: &[usize] = &[0, 1, 2, SLOT_INGREDIENT];
const SLOTS_FOR_SIDES: &[usize] = &[0, 1, 2, SLOT_FUEL];

impl BrewingStandHopperFace {
    pub(in crate::world) fn slots(&self) -> &[usize] {
        self.insert_slots
    }

    /// `canPlaceItemThroughFace` (`:239-241`) over `canPlaceItem`
    /// (`:217-227`).
    pub(in crate::world) fn allows_insert(&self, slot: usize, item: &ItemStackSummary) -> bool {
        self.insert_slots.contains(&slot) && self.allows_place(slot, item)
    }

    pub(in crate::world) fn allows_place(&self, slot: usize, item: &ItemStackSummary) -> bool {
        if slot == SLOT_INGREDIENT {
            return PotionBrewing::vanilla().is_ingredient(item);
        }
        if slot == SLOT_FUEL {
            return item
                .item_id
                .is_some_and(|item_id| self.brewing_fuel.contains(&item_id));
        }
        brewing_potion_item(item) && !self.bottle_slots_occupied[slot]
    }

    /// `canTakeItemThroughFace` (`:243-246`): the ingredient slot gives up an
    /// empty bottle and nothing else; the other slots give up anything.
    pub(in crate::world) fn allows_take(&self, slot: usize, item: &ItemStackSummary) -> bool {
        if !self.take_slots.contains(&slot) {
            return false;
        }
        slot != SLOT_INGREDIENT
            || item.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID)
    }
}

/// `BrewingStandMenu.PotionSlot.mayPlaceItem` (`BrewingStandMenu.java:184-186`)
/// over the same item set the block entity's bottle slots admit.
fn brewing_potion_item(item: &ItemStackSummary) -> bool {
    crate::inventory::brewing_potion_item(item)
}

impl ServerWorld {
    /// The face a hopper reaches a brewing stand through, or `None` when the
    /// position is not one.
    pub(in crate::world) fn brewing_stand_hopper_face(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        face: bbb_protocol::packets::Direction,
    ) -> Option<BrewingStandHopperFace> {
        let block_state_id = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)?;
        if vanilla_block_state_block_type_id(block_state_id) != Some(VANILLA_BLOCK_BREWING_STAND_ID)
        {
            return None;
        }
        let stand = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::brewing_stand)?;
        let mut bottle_slots_occupied = [false; 3];
        let items = stand.original_items();
        for (index, occupied) in bottle_slots_occupied.iter_mut().enumerate() {
            *occupied = !items[index].is_empty();
        }
        let (insert_slots, take_slots) = match face {
            bbb_protocol::packets::Direction::Up => (SLOTS_FOR_UP, &[] as &[usize]),
            bbb_protocol::packets::Direction::Down => (SLOTS_FOR_DOWN, SLOTS_FOR_DOWN),
            _ => (SLOTS_FOR_SIDES, &[] as &[usize]),
        };
        Some(BrewingStandHopperFace {
            insert_slots,
            take_slots,
            bottle_slots_occupied,
            brewing_fuel: self.brewing_fuel_tag()?,
        })
    }

    /// `BrewingStandBlock.useWithoutItem` (`BrewingStandBlock.java:65-74`):
    /// the block entity's own menu, then the interaction statistic, both
    /// inside the `instanceof` test. The answer is `InteractionResult.SUCCESS`
    /// whatever happened (`:73`).
    pub(in crate::world) fn use_brewing_stand_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let is_stand = self.has_container_block_entity(dimension, pos, |at| {
            BlockEntity::brewing_stand(at).is_some()
        });
        self.open_brewing_stand_menu(connection_id, dimension, pos, events);
        if is_stand {
            self.award_opener_stat(connection_id, "minecraft:interact_with_brewingstand");
        }
        true
    }

    /// `BrewingStandBlockEntity` as a `MenuProvider`: `BrewingStandMenu` over
    /// the block entity's five slots and its two `ContainerData` values
    /// (`BrewingStandMenu.java:41-55`).
    ///
    /// The fuel slot's admission is the live `minecraft:brewing_fuel` tag, so
    /// a menu without a loaded generation is not this menu.
    pub(in crate::world) fn open_brewing_stand_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        if instances[0].container_slots().is_none() {
            return false;
        }
        let Some(fuels) = self.brewing_fuel_tag() else {
            return false;
        };
        if !self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            MenuLayout::BrewingStand,
            "container.brewing",
            events,
        ) {
            return false;
        }
        self.levels[connection_id]
            .inventory
            .attach_brewing_fuel(fuels);
        true
    }

    /// The `minecraft:brewing_fuel` members
    /// (`BrewingStandMenu.java:135-137`) over the live item tags.
    fn brewing_fuel_tag(&self) -> Option<Arc<BTreeSet<i32>>> {
        self.datapack_tables.brewing_fuel_items(self.item_tags())
    }

    /// One `BlockEntityTicker.tick` for a brewing stand
    /// (`BrewingStandBlockEntity.java:97-139`).
    pub(in crate::world) fn tick_brewing_stand(&mut self, dimension: &LevelId, pos: BlockPos) {
        let Some(block_state_id) = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
        else {
            return;
        };
        if vanilla_block_state_block_type_id(block_state_id) != Some(VANILLA_BLOCK_BREWING_STAND_ID)
        {
            return;
        }
        let Some(stand) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::brewing_stand)
        else {
            return;
        };
        let items = stand.original_items();
        let fuel_is_brewing_fuel = stand.fuel() <= 0
            && self
                .item_tags()
                .zip(items[SLOT_FUEL].item_id())
                .is_some_and(|(tags, item_id)| tags.contains("minecraft:brewing_fuel", item_id));
        let ingredient_remainder = (stand.brew_time() == 1)
            .then(|| {
                items[SLOT_INGREDIENT]
                    .item_id()
                    .and_then(crafting_remainder)
                    .map(RuntimeItemStack::from_summary)
            })
            .flatten();
        let facts = BrewingStandTickFacts {
            brewing: PotionBrewing::vanilla(),
            fuel_is_brewing_fuel,
            ingredient_remainder,
        };
        let Some(stand) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::brewing_stand_mut)
        else {
            return;
        };
        let tick = match stand.server_tick(&facts) {
            Ok(tick) => tick,
            Err(error) => {
                tracing::warn!("brewing stand tick: {error}");
                return;
            }
        };
        if let Some(remainder) = tick.dropped_remainder {
            self.drop_brewing_stand_remainder(dimension, pos, &remainder);
        }
        if tick.brewed {
            self.level_mut_by_id(dimension).event_sink.level_event(
                BREWING_STAND_BREW_EVENT,
                pos,
                0,
            );
        }
        if tick.changed {
            self.ticked_block_entity_changed(dimension, pos);
        }
        if let Some(bits) = tick.potion_bits {
            // `:128-137`: `level.setBlock(pos, state, 2)`, which tells the
            // clients and leaves the neighbours alone.
            let mut next = Some(block_state_id);
            for (index, bit) in bits.iter().enumerate() {
                next = next.and_then(|state| {
                    crate::fluid::with_property(
                        state,
                        HAS_BOTTLE[index],
                        if *bit { "true" } else { "false" },
                    )
                });
            }
            if let Some(next) = next {
                self.set_block(dimension, pos, next, SET_BLOCK_UPDATE_CLIENTS);
            }
        }
    }

    /// `Containers.dropItemStack(level, pos.getX(), pos.getY(), pos.getZ(), remainder)`
    /// (`BrewingStandBlockEntity.java:187`), the same seam the container drops
    /// use.
    fn drop_brewing_stand_remainder(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        remainder: &RuntimeItemStack,
    ) {
        if remainder.is_empty() {
            return;
        }
        let position = Vec3d {
            x: f64::from(pos.x),
            y: f64::from(pos.y),
            z: f64::from(pos.z),
        };
        self.drop_runtime_container_contents(dimension, position, std::slice::from_ref(remainder));
    }
}
