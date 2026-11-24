//! `DispenserBlock.dispenseFrom` and `DropperBlock.dispenseFrom`: the slot
//! draw, the dropper's container transfer, and the dispenser's per-item
//! `DispenseItemBehavior` (`DispenseItemBehavior.bootStrap`), one family per
//! module beside this file.

mod bottle;
mod brush;
mod bucket;
mod equipment;
mod head;
mod igniter;
mod projectile;
mod shears;
mod shulker_box;
mod spawn;
mod tnt;
mod transform;
mod vehicle;

use super::super::simulation_level::container_access::BlockContainer;
use super::hopper::{WorldContainer, WorldContainerAccess};
use super::*;
use crate::block_behavior::dispenser;
use crate::world::simulation_level::LevelCtx;
use crate::world::LevelId;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

/// `LevelEvent.SOUND_DISPENSER_DISPENSE`.
const DISPENSE_SOUND: i32 = 1000;
/// `LevelEvent.PARTICLES_SHOOT_SMOKE`.
const SHOOT_SMOKE: i32 = 2000;

/// `BlockSource` (`BlockSource.java`): the dispenser being fired and the slot
/// its stack came from.
#[derive(Clone, Copy)]
pub(super) struct BlockSource {
    level: LevelId,
    pos: BlockPos,
    facing: Direction,
    container: BlockContainer,
}

impl BlockSource {
    /// `pos.relative(FACING)`.
    fn target(&self) -> BlockPos {
        bbb_server_types::directions::relative(self.pos, self.facing)
    }

    fn step(&self) -> Vec3d {
        let step = self.target();
        Vec3d {
            x: f64::from(step.x - self.pos.x),
            y: f64::from(step.y - self.pos.y),
            z: f64::from(step.z - self.pos.z),
        }
    }

    /// `BlockSource.center`.
    fn center(&self) -> Vec3d {
        Vec3d {
            x: f64::from(self.pos.x) + 0.5,
            y: f64::from(self.pos.y) + 0.5,
            z: f64::from(self.pos.z) + 0.5,
        }
    }

    /// `DispenserBlock.getDispensePosition(source, scale, (0, y_offset, 0))`.
    fn dispense_position(&self, scale: f64, y_offset: f64) -> Vec3d {
        let center = self.center();
        let step = self.step();
        Vec3d {
            x: center.x + scale * step.x,
            y: center.y + scale * step.y + y_offset,
            z: center.z + scale * step.z,
        }
    }

    /// `Direction.toYRot`: `(data2d & 3) * 90`, so up and down read as east.
    fn y_rot(&self) -> f32 {
        match self.facing {
            Direction::South => 0.0,
            Direction::West => 90.0,
            Direction::North => 180.0,
            Direction::East | Direction::Up | Direction::Down => 270.0,
        }
    }
}

/// What one behaviour leaves: the stack written back to the slot and the
/// level event its `playSound` raises before the smoke.
pub(super) struct Dispensed {
    stack: RuntimeItemStack,
    sound: i32,
}

impl Dispensed {
    const fn new(stack: RuntimeItemStack, sound: i32) -> Self {
        Self { stack, sound }
    }

    /// `OptionalDispenseItemBehavior.playSound`.
    const fn optional(stack: RuntimeItemStack, success: bool) -> Self {
        Self {
            stack,
            sound: if success {
                DISPENSE_SOUND
            } else {
                dispenser::EMPTY_SOUND
            },
        }
    }
}

impl ServerWorld {
    pub(super) fn tick_dispenser(&mut self, dimension: &LevelId, pos: BlockPos, state: i32) {
        let Some(source) = self.with_level(dimension, |level| level.block_container(pos, true))
        else {
            return;
        };
        if !self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|entity| entity.dispenser().is_some())
        {
            return;
        }
        let dropper = dispenser::is_dropper(state);
        let source = BlockSource {
            level: *dimension,
            pos,
            facing: dispenser::facing(state),
            container: source,
        };
        if let Err(error) = self.dispense_from(source, dropper) {
            self.with_level(dimension, |level| level.record_item_failure(pos, error));
        }
        self.post_pending_game_events(dimension);
    }

    fn dispense_from(&mut self, source: BlockSource, dropper: bool) -> CoreResult<()> {
        let Some((slot, item)) = self.random_dispenser_item(&source)? else {
            self.empty_dispenser_event(&source, dropper);
            return Ok(());
        };
        if item.is_empty() {
            return Ok(());
        }
        let result = if dropper {
            self.drop_into_container(&source, item)?
        } else {
            let dispensed = self.dispense_item(&source, slot, item)?;
            self.dispenser_events(&source, dispensed.sound);
            dispensed.stack
        };
        self.dispenser_access(&source).set_item(
            WorldContainer::Block(source.container),
            slot,
            result,
        )
    }

    /// `DispenserBlock.getDispenseMethod` over the bootstrap registry.
    fn dispense_item(
        &mut self,
        source: &BlockSource,
        slot: usize,
        stack: RuntimeItemStack,
    ) -> CoreResult<Dispensed> {
        use bbb_protocol::item_types::*;
        let Some(id) = stack.item_id() else {
            return self.default_dispense(source, stack);
        };
        match id {
            VANILLA_ITEM_ARROW_ID
            | VANILLA_ITEM_TIPPED_ARROW_ID
            | VANILLA_ITEM_SPECTRAL_ARROW_ID
            | VANILLA_ITEM_EGG_ID
            | VANILLA_ITEM_BLUE_EGG_ID
            | VANILLA_ITEM_BROWN_EGG_ID
            | VANILLA_ITEM_SNOWBALL_ID
            | VANILLA_ITEM_EXPERIENCE_BOTTLE_ID
            | VANILLA_ITEM_SPLASH_POTION_ID
            | VANILLA_ITEM_LINGERING_POTION_ID
            | VANILLA_ITEM_FIREWORK_ROCKET_ID
            | VANILLA_ITEM_FIRE_CHARGE_ID
            | VANILLA_ITEM_WIND_CHARGE_ID => projectile::dispense(self, source, stack),
            VANILLA_ITEM_ARMOR_STAND_ID => spawn::armor_stand(self, source, stack),
            VANILLA_ITEM_CHEST_ID => equipment::chest(self, source, stack),
            VANILLA_ITEM_LAVA_BUCKET_ID
            | VANILLA_ITEM_WATER_BUCKET_ID
            | VANILLA_ITEM_POWDER_SNOW_BUCKET_ID
            | VANILLA_ITEM_SALMON_BUCKET_ID
            | VANILLA_ITEM_COD_BUCKET_ID
            | VANILLA_ITEM_PUFFERFISH_BUCKET_ID
            | VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID
            | VANILLA_ITEM_AXOLOTL_BUCKET_ID
            | VANILLA_ITEM_TADPOLE_BUCKET_ID => bucket::empty(self, source, slot, stack),
            VANILLA_ITEM_BUCKET_ID => bucket::fill(self, source, slot, stack),
            VANILLA_ITEM_FLINT_AND_STEEL_ID => igniter::dispense(self, source, stack),
            VANILLA_ITEM_BONE_MEAL_ID => transform::bone_meal(self, source, stack),
            VANILLA_ITEM_TNT_ID => tnt::dispense(self, source, stack),
            VANILLA_ITEM_WITHER_SKELETON_SKULL_ID => head::wither_skull(self, source, stack),
            VANILLA_ITEM_CARVED_PUMPKIN_ID => head::carved_pumpkin(self, source, stack),
            VANILLA_ITEM_GLASS_BOTTLE_ID => bottle::fill(self, source, slot, stack),
            VANILLA_ITEM_GLOWSTONE_ID => transform::glowstone(self, source, stack),
            VANILLA_ITEM_BRUSH_ID => brush::dispense(self, source, stack),
            VANILLA_ITEM_SHEARS_ID => shears::dispense(self, source, stack),
            VANILLA_ITEM_HONEYCOMB_ID => transform::honeycomb(self, source, stack),
            VANILLA_ITEM_POTION_ID => bottle::potion(self, source, slot, stack),
            VANILLA_ITEM_MINECART_ID
            | VANILLA_ITEM_CHEST_MINECART_ID
            | VANILLA_ITEM_FURNACE_MINECART_ID
            | VANILLA_ITEM_TNT_MINECART_ID
            | VANILLA_ITEM_HOPPER_MINECART_ID
            | VANILLA_ITEM_COMMAND_BLOCK_MINECART_ID => vehicle::minecart(self, source, stack),
            _ if vehicle::is_boat(&stack) => vehicle::boat(self, source, stack),
            _ if shulker_box::is_shulker_box(id) => shulker_box::dispense(self, source, stack),
            // `DispenserBlock.getDefaultDispenseMethod`.
            _ if equipment::has_equippable(&stack) => equipment::dispense(self, source, stack),
            _ if spawn::is_spawn_egg(&stack) => spawn::spawn_egg(self, source, stack),
            _ => self.default_dispense(source, stack),
        }
    }

    /// `DefaultDispenseItemBehavior.dispense` with its own `execute`.
    fn default_dispense(
        &mut self,
        source: &BlockSource,
        mut stack: RuntimeItemStack,
    ) -> CoreResult<Dispensed> {
        self.drop_one(source, &mut stack)?;
        Ok(Dispensed::new(stack, DISPENSE_SOUND))
    }

    /// `DefaultDispenseItemBehavior.execute`: one item flies out.
    fn drop_one(&mut self, source: &BlockSource, stack: &mut RuntimeItemStack) -> CoreResult<()> {
        let one = stack.split(1);
        self.spawn_dispensed_stack(
            &source.level,
            source.dispense_position(0.7, 0.0),
            source.facing,
            6,
            &one,
        )
    }

    /// `DefaultDispenseItemBehavior.consumeWithRemainder`: the remainder goes
    /// into the dispenser, else out of its face.
    fn consume_with_remainder(
        &mut self,
        source: &BlockSource,
        slot: usize,
        mut stack: RuntimeItemStack,
        remainder: RuntimeItemStack,
    ) -> CoreResult<RuntimeItemStack> {
        stack.shrink(1);
        if stack.is_empty() {
            return Ok(remainder);
        }
        let container = WorldContainer::Block(source.container);
        let mut access = self.dispenser_access(source);
        access.set_item(container, slot, stack.clone())?;
        let left = access.add_item(None, container, remainder, None)?;
        if !left.is_empty() {
            self.spawn_dispensed_stack(
                &source.level,
                source.dispense_position(0.7, 0.0),
                source.facing,
                6,
                &left,
            )?;
            self.dispenser_events(source, DISPENSE_SOUND);
        }
        Ok(stack)
    }

    /// `ItemStack.hurtAndBreak(amount, level, null, ...)`.
    fn hurt_dispensed(&mut self, dimension: LevelId, stack: &mut RuntimeItemStack, amount: i32) {
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_item_damage);
        let mut summary = stack.summary().clone();
        let enchantments = Self::resolve_value_effects(&summary, &table);
        let mut outcome = crate::item_durability::MineBlockOutcome::default();
        crate::item_durability::hurt_and_break(
            &mut summary,
            amount,
            false,
            &enchantments,
            &self.random,
            &self.value_effect_level(&dimension),
            &mut outcome,
        );
        drop(enchantments);
        *stack = RuntimeItemStack::from_summary(summary);
    }

    fn dispenser_access(&mut self, source: &BlockSource) -> WorldContainerAccess<'_> {
        WorldContainerAccess {
            world: self,
            dimension: source.level,
            pos: source.pos,
            facing: source.facing,
        }
    }

    /// `playSound` then `playAnimation`.
    fn dispenser_events(&mut self, source: &BlockSource, sound: i32) {
        let (pos, facing) = (source.pos, source.facing);
        self.with_level(source.level, |level| {
            LevelCtx::level_event(level, sound, pos, 0);
            LevelCtx::level_event(level, SHOOT_SMOKE, pos, i32::from(facing.id()));
        });
    }

    fn random_dispenser_item(
        &mut self,
        source: &BlockSource,
    ) -> CoreResult<Option<(usize, RuntimeItemStack)>> {
        let (pos, container) = (source.pos, source.container);
        self.with_level(source.level, |level| {
            level.unpack_container_loot(pos, None)?;
            let slot = level
                .chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::dispenser)
                .and_then(|dispenser| dispenser.random_slot(level.random));
            let Some(slot) = slot else {
                return Ok(None);
            };
            Ok(level
                .get_container_item(container, slot)?
                .map(|item| (slot, item)))
        })
    }

    /// `DropperBlock.dispenseFrom`: one item into the attached container, or
    /// the default drop when nothing is attached.
    fn drop_into_container(
        &mut self,
        source: &BlockSource,
        mut item: RuntimeItemStack,
    ) -> CoreResult<RuntimeItemStack> {
        let one = item.with_count(1);
        let mut access = self.dispenser_access(source);
        let moved = match access.attached() {
            Some((into, direction)) => access
                .add_item(
                    Some(WorldContainer::Block(source.container)),
                    into,
                    one,
                    Some(direction),
                )?
                .is_empty(),
            None => {
                let dispensed = self.default_dispense(source, one)?;
                self.dispenser_events(source, dispensed.sound);
                true
            }
        };
        if moved {
            item.shrink(1);
        }
        Ok(item)
    }

    fn empty_dispenser_event(&mut self, source: &BlockSource, dropper: bool) {
        let pos = source.pos;
        self.with_level(source.level, |level| {
            LevelCtx::level_event(level, dispenser::EMPTY_SOUND, pos, 0);
            if !dropper {
                let state = level
                    .chunks
                    .block_entities()
                    .get(pos)
                    .map(BlockEntity::block_state);
                LevelCtx::block_game_event(
                    level,
                    crate::level_event::VanillaGameEvent::BLOCK_ACTIVATE,
                    pos,
                    state,
                );
            }
        });
    }
}
