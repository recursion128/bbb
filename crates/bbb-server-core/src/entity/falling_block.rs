//! Falling-block production, movement and settlement.

use crate::world::simulation_level::LevelCtx;
use std::sync::OnceLock;

use bbb_protocol::block_items::vanilla_block_item;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_identity, vanilla_block_state_property,
    vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID;
use bbb_protocol::packets::{BlockPos, DataComponentPatchSummary, ItemStackSummary, Vec3d};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;
use bbb_protocol::NbtSummaryEntry;

use crate::block_behavior::FallingBlockFallRequest;
use crate::entity::level::fluid_state_tag_membership;
use crate::entity::store::EntityStore;
use crate::entity::{apply_gravity, Entity, EntityData, MoverType};
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};
use bbb_server_types::geometry::Vec3Ops;

/// `FallingBlockEntity.getDefaultGravity()`
/// (`net/minecraft/world/entity/item/FallingBlockEntity.java:142-144`).
///
/// This is the falling block's own value, not a shared constant:
/// `Entity.getDefaultGravity()` is `0.0` (`Entity.java:1432-1434`) and every
/// type that falls declares its own.
pub const DEFAULT_GRAVITY: f64 = 0.04;

/// The horizontal-and-vertical drag `FallingBlockEntity.tick` applies at the
/// very end of every tick, after the move and after any landing
/// (`FallingBlockEntity.java:240`).
const DRAG: f64 = 0.98;

/// The delta scaling a landed falling block gets before it tries to place
/// itself (`FallingBlockEntity.java:181`).
const LANDING_DELTA_SCALE: [f64; 3] = [0.7, -0.5, 0.7];

/// `FallingBlockEntity.tick`'s two lifetime bounds (`:172`): past 100 ticks a
/// block outside the build height is dropped, and past 600 ticks any block is.
const OUT_OF_WORLD_TIME: i32 = 100;
const MAX_TIME: i32 = 600;

/// The `FallingBlockEntity` fields (`FallingBlockEntity.java`).
#[derive(Debug, Clone, PartialEq)]
pub struct FallingBlockData {
    block_state_id: i32,
    block_data: Option<Vec<NbtSummaryEntry>>,
    start_pos: BlockPos,
    time: i32,
    drop_item: bool,
    cancel_drop: bool,
    hurt_entities: bool,
    fall_damage_max: i32,
    fall_damage_per_distance: f32,
}

impl FallingBlockData {
    pub fn block_data(&self) -> Option<&[NbtSummaryEntry]> {
        self.block_data.as_deref()
    }

    pub fn with_block_data(mut self, block_data: Option<Vec<NbtSummaryEntry>>) -> Self {
        self.block_data = block_data;
        self
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.start_pos = previous.start_pos;
    }

    /// The field defaults `FallingBlockEntity` declares
    /// (`FallingBlockEntity.java`), with the block state and the
    /// `DATA_START_POS` synched value the constructor sets (`:81`, `:88`).
    pub fn new(block_state_id: i32, start_pos: BlockPos) -> Self {
        Self {
            block_state_id,
            block_data: None,
            start_pos,
            time: 0,
            drop_item: true,
            cancel_drop: false,
            hurt_entities: false,
            fall_damage_max: 40,
            fall_damage_per_distance: 0.0,
        }
    }

    pub const fn block_state_id(&self) -> i32 {
        self.block_state_id
    }

    pub const fn start_pos(&self) -> BlockPos {
        self.start_pos
    }

    pub const fn time(&self) -> i32 {
        self.time
    }

    pub const fn drop_item(&self) -> bool {
        self.drop_item
    }

    pub const fn cancel_drop(&self) -> bool {
        self.cancel_drop
    }

    pub const fn hurt_entities(&self) -> bool {
        self.hurt_entities
    }

    pub const fn fall_damage_max(&self) -> i32 {
        self.fall_damage_max
    }

    pub const fn fall_damage_per_distance(&self) -> f32 {
        self.fall_damage_per_distance
    }

    /// `FallingBlockEntity.readAdditionalSaveData` (`:295-306`) for the fields
    /// a save carries beyond the two the constructor sets. `HurtEntities`
    /// defaults to whether the block is in `#minecraft:anvil` (`:299-300`).
    pub fn with_saved_fields(
        mut self,
        time: i32,
        drop_item: bool,
        cancel_drop: bool,
        hurt_entities: bool,
        fall_damage_per_distance: f32,
        fall_damage_max: i32,
    ) -> Self {
        self.time = time;
        self.drop_item = drop_item;
        self.cancel_drop = cancel_drop;
        self.hurt_entities = hurt_entities;
        self.fall_damage_per_distance = fall_damage_per_distance;
        self.fall_damage_max = fall_damage_max;
        self
    }

    /// `FallingBlockEntity.setHurtsEntities(float, int)`
    /// (`FallingBlockEntity.java`): the only writer of `hurtEntities`,
    /// which it sets unconditionally alongside both damage numbers.
    pub fn set_hurts_entities(&mut self, damage_per_distance: f32, damage_max: i32) {
        self.hurt_entities = true;
        self.fall_damage_per_distance = damage_per_distance;
        self.fall_damage_max = damage_max;
    }

    /// `FallingBlockEntity.disableDrop` (`FallingBlockEntity.java`).
    pub fn disable_drop(&mut self) {
        self.cancel_drop = true;
    }
}

/// `AnvilBlock.falling`'s `setHurtsEntities(2.0F, 40)` (`AnvilBlock.java:83`).
const ANVIL_FALL_DAMAGE_PER_DISTANCE: f32 = 2.0;
const ANVIL_FALL_DAMAGE_MAX: i32 = 40;

/// `FallingBlock.tick` (`FallingBlock.java`): the scheduled tick that
/// turns a placed falling block into an entity.
///
/// Returns the new entity's id when one was created. The state is re-read from
/// the level by the caller and passed in, because a scheduled tick can outlive
/// the state that scheduled it and vanilla re-reads it the same way
/// (`ServerLevel.tickBlock`, `ServerLevel.java`).
pub(crate) fn falling_block_scheduled_tick(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<i32> {
    if !is_falling_block(block_state_id) {
        return None;
    }
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    if pos.y < level.min_y()
        || crate::block_behavior::falling::is_free(level, level.block_state(below)) != Some(true)
    {
        return None;
    }
    let id = fall(store, level, pos, block_state_id);
    // `:52`. The hook runs on the block that scheduled the tick, so it reads
    // `block_state_id` rather than anything about the entity.
    if let Some(entity) = store.get_mut(id) {
        falling(entity, block_state_id);
    }
    Some(id)
}

/// `FallingBlock.falling(FallingBlockEntity)` (`FallingBlock.java`), the
/// empty hook `FallingBlock.tick` calls on the entity it just created, and its
/// one 26.1 override.
///
/// `AnvilBlock.falling` (`AnvilBlock.java`) is the only override in the
/// whole source tree, and it is `setHurtsEntities(2.0F, 40)`. Two fields it
/// writes are saved (`addAdditionalSaveData`, `FallingBlockEntity.java`),
/// so skipping the hook is visible in the entity region file of any world with
/// a falling anvil in it, not only in the damage this slice cannot deal yet.
///
/// The anvil family is named by the generated class-owner column rather than by
/// the `#minecraft:anvil` tag: `falling` is a class override, and the tag is
/// what a *different* vanilla call site uses (`:299`).
fn falling(entity: &mut Entity, block_state_id: i32) {
    if placement_owner(block_state_id) != Some(VanillaGetStateForPlacementOverride::AnvilBlock) {
        return;
    }
    // The caller built this entity two statements ago, so the variant is known;
    // the arm is written out rather than asserted because `falling` is the hook
    // a future `Fallable` would call on whatever entity it was handed.
    let EntityData::FallingBlock(data) = &mut entity.data else {
        return;
    };
    data.set_hurts_entities(ANVIL_FALL_DAMAGE_PER_DISTANCE, ANVIL_FALL_DAMAGE_MAX);
}

/// `FallingBlockEntity.fall(Level, BlockPos, BlockState)`
/// (`FallingBlockEntity.java`).
///
/// The order is load-bearing: the entity is built from the pre-clear state, the
/// block is then replaced by whatever fluid was in it, and only then is the
/// entity added. Reversing the last two would make the entity collide with the
/// block it came from.
fn fall(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> i32 {
    level.set_entity_block(
        crate::entity::level::EntityBlockContext::new(Some(&mut *store), None),
        pos,
        fallen_block_replacement(block_state_id),
        SET_BLOCK_UPDATE_ALL,
    );
    spawn_fallen_block(store, pos, block_state_id)
}

/// `state.getFluidState().createLegacyBlock()` (`FallingBlockEntity.java`):
/// air for a dry block, the equivalent liquid for a waterlogged one.
///
/// A caller that owns the level write makes it itself and then adds the entity
/// with [`spawn_fallen_block`]; `fall` is the two of them together.
pub fn fallen_block_replacement(block_state_id: i32) -> i32 {
    block_state_fluid_state(block_state_id).create_legacy_block()
}

/// One [`FallingBlockFallRequest`] a block raised, for a caller that has already
/// made the level write [`fallen_block_replacement`] names: the entity
/// `addFreshEntity` adds (`FallingBlockEntity.java`) followed by the writes
/// the asking block makes on it -- `disableDrop` (`:314-316`) for a brushable
/// block and `setHurtsEntities` (`:308-312`) for a falling stalactite tip.
pub fn spawn_requested_fallen_block(
    store: &mut EntityStore,
    request: FallingBlockFallRequest,
) -> i32 {
    let id = spawn_fallen_block(store, request.pos, request.block_state_id);
    let Some(EntityData::FallingBlock(data)) = store.get_mut(id).map(|entity| &mut entity.data)
    else {
        return id;
    };
    if request.disable_drop {
        data.disable_drop();
    }
    if let Some(hurt) = request.hurt_entities {
        data.set_hurts_entities(hurt.damage_per_distance, hurt.damage_max);
    }
    id
}

/// The entity half of `FallingBlockEntity.fall` (`FallingBlockEntity.java`),
/// built from the state the block had before the level write cleared it.
pub fn spawn_fallen_block(store: &mut EntityStore, pos: BlockPos, block_state_id: i32) -> i32 {
    let entity_state_id = with_waterlogged(block_state_id, false).unwrap_or(block_state_id);
    let position = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + 0.5,
    };
    // `BlockPos.containing(position)` of that position is `pos` itself, which
    // is what `setStartPos(this.blockPosition())` (`:88`) records.
    let data = FallingBlockData::new(entity_state_id, pos);
    store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        position,
        EntityData::FallingBlock(data),
    )
}

/// `FallingBlockEntity.tick()` (`FallingBlockEntity.java`).
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if block_state_is_air(data_of(entity).block_state_id) {
        entity.discard();
        return true;
    }
    let original_block_state = data_of(entity).block_state_id;
    let block_type_id = vanilla_block_state_block_type_id(original_block_state);
    let data = data_of_mut(entity);
    data.time = data.time.wrapping_add(1);
    apply_gravity(entity);
    let delta = entity.delta_movement;
    if crate::entity::movement::move_entity_with_ground_contact(
        entity,
        store,
        level,
        MoverType::Self_,
        delta,
    )
    .is_err()
    {
        return false;
    }
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
        return false;
    }
    if !crate::entity::handle_portal(entity, level) {
        return false;
    }
    if entity.is_alive()
        && settle(entity, store, level, original_block_state, block_type_id).is_err()
    {
        return false;
    }
    entity.delta_movement = entity.delta_movement.scale(DRAG);
    true
}

/// `forceTickAfterTeleportToDuplicate` (`FallingBlockEntity.java:157`): the
/// copy a move into or out of the End left behind still lands this tick.
pub(crate) fn land_left_behind_copy(
    mut entity: Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let state = data_of(&entity).block_state_id;
    let _ = settle(
        &mut entity,
        store,
        level,
        state,
        vanilla_block_state_block_type_id(state),
    );
}

fn data_of(entity: &Entity) -> &FallingBlockData {
    let EntityData::FallingBlock(data) = entity.data() else {
        unreachable!("falling block owner");
    };
    data
}

fn data_of_mut(entity: &mut Entity) -> &mut FallingBlockData {
    let EntityData::FallingBlock(data) = &mut entity.data else {
        unreachable!("falling block owner");
    };
    data
}

/// The server half of `FallingBlockEntity.tick` (`:157-238`).
fn settle(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    original_block_state: i32,
    block_type_id: Option<i32>,
) -> Result<(), ()> {
    let mut pos = entity.block_position;
    let is_concrete = placement_owner(data_of(entity).block_state_id)
        == Some(VanillaGetStateForPlacementOverride::ConcretePowderBlock);
    // `:160`: this one *is* the tag, so flowing water counts.
    let mut stuck_in_water = if is_concrete {
        is_in_water_tag(level, level.block_state(pos))
    } else {
        false
    };

    // FallingBlockEntity.java: first whole hit, then the live water tag.
    if is_concrete && entity.delta_movement.length_sqr() > 1.0 {
        let hit = level
            .clip_collider_source_only(
                Some(&mut *store),
                entity,
                entity.old_position(),
                entity.position(),
            )
            .ok_or(())?;
        if let Some(hit) = hit {
            if is_in_water_tag(level, level.block_state(hit)) {
                pos = hit;
                stuck_in_water = true;
            }
        }
    }

    if !entity.on_ground && !stuck_in_water {
        let outside_build_height = pos.y <= level.min_y() || pos.y > level.max_y();
        if (data_of(entity).time > OUT_OF_WORLD_TIME && outside_build_height)
            || data_of(entity).time > MAX_TIME
        {
            if data_of(entity).drop_item && level.entity_drops() {
                spawn_as_item(entity, level, block_type_id);
            }
            entity.discard();
        }
        return Ok(());
    }

    let current_state = level.block_state(pos);
    entity.delta_movement = entity.delta_movement.multiply(
        LANDING_DELTA_SCALE[0],
        LANDING_DELTA_SCALE[1],
        LANDING_DELTA_SCALE[2],
    );
    if is_moving_piston(current_state) {
        return Ok(());
    }

    if data_of(entity).cancel_drop {
        // `:232-235`: the block is gone and only the `Fallable` break hook runs.
        entity.discard();
        fallable_hooks::on_broken_after_fall(entity, level, pos, original_block_state).ok_or(())?;
        return Ok(());
    }

    // `:184-188`.
    let may_replace = can_be_replaced_by_empty_handed_place(current_state);
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    let Some(below_is_free) =
        crate::block_behavior::falling::is_free(level, level.block_state(below))
    else {
        return Err(());
    };
    let would_continue_falling = below_is_free && (!is_concrete || !stuck_in_water);
    let would_survive =
        crate::block_behavior::can_survive_at(data_of(entity).block_state_id, pos, &*level)
            && !would_continue_falling;

    if may_replace && would_survive {
        // `:190-192`: a waterloggable block that lands in *source* water lands
        // wet. This is the exact-fluid `is(Fluids.WATER)`, not the
        // `is(FluidTags.WATER)` two lines of vanilla earlier at `:160`, so a
        // block landing in flowing water is not waterlogged.
        if is_water_source(current_state) {
            if let Some(waterlogged) = with_waterlogged(data_of(entity).block_state_id, true) {
                data_of_mut(entity).block_state_id = waterlogged;
            }
        }
        if level.set_entity_block(
            crate::entity::level::EntityBlockContext::new(Some(&mut *store), Some(entity)),
            pos,
            data_of(entity).block_state_id,
            SET_BLOCK_UPDATE_ALL,
        ) {
            // `:195-197` broadcasts the placement to trackers; that is the
            // packet layer's half and this mechanism has no tracker set.
            entity.discard();
            fallable_hooks::on_land(
                entity,
                Some(&mut *store),
                level,
                pos,
                original_block_state,
                current_state,
            )
            .ok_or(())?;
            // FallingBlockEntity.java: merge only after onLand, into the actual live object.
            if let Some(block_data) = data_of(entity).block_data() {
                if bbb_server_types::block_queries::block_state_has_block_entity(
                    data_of(entity).block_state_id,
                ) {
                    level
                        .merge_falling_block_entity_data(
                            crate::entity::level::EntityBlockContext::new(
                                Some(&mut *store),
                                Some(entity),
                            ),
                            pos,
                            block_data,
                        )
                        .ok_or(())?;
                }
            }
        } else if data_of(entity).drop_item && level.entity_drops() {
            // FallingBlockEntity.java: without the drop guards the body remains alive.
            entity.discard();
            drop_as_item(entity, level, pos, original_block_state, block_type_id)?;
        }
    } else {
        // FallingBlockEntity.java: discard precedes the guarded break hook.
        entity.discard();
        drop_as_item(entity, level, pos, original_block_state, block_type_id)?;
    }
    Ok(())
}

/// `this.spawnAtLocation(serverLevel, block)` behind its two guards
/// (`FallingBlockEntity.java`, `:220-224`, `:227-230`).
fn drop_as_item(
    entity: &Entity,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    original_block_state: i32,
    block_type_id: Option<i32>,
) -> Result<(), ()> {
    if !data_of(entity).drop_item || !level.entity_drops() {
        return Ok(());
    }
    fallable_hooks::on_broken_after_fall(entity, level, pos, original_block_state).ok_or(())?;
    spawn_as_item(entity, level, block_type_id);
    Ok(())
}

fn spawn_as_item(entity: &Entity, level: &mut LevelCtx<'_, '_>, block_type_id: Option<i32>) {
    // `Block.asItem()` through `Item.BY_BLOCK`; a block no item names produces
    // an empty stack, which `spawnAtLocation` drops on the floor
    // (`Entity.java`).
    let Some(item_type_id) = block_type_id.and_then(vanilla_block_item) else {
        return;
    };
    // `new ItemStack(resource)` (`Entity.java`): one of the item, with no
    // components on it.
    level.spawn_item(
        entity.position,
        &ItemStackSummary {
            item_id: Some(item_type_id),
            count: SPAWN_AT_LOCATION_COUNT,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
}

/// `new ItemStack(ItemLike)` (`net/minecraft/world/item/ItemStack.java:244-246`)
/// is the stack `Entity.spawnAtLocation(level, ItemLike)` builds
/// (`Entity.java:2123-2125`): one of the item, and a component map that is the
/// item's own prototype -- an empty *patch*, which is what the summary carries.
const SPAWN_AT_LOCATION_COUNT: i32 = 1;

pub use crate::block_behavior::falling::is_free_with_tags as is_free;

use bbb_server_types::block_queries::is_falling_block;

/// `BlockState.canBeReplaced()` (`BlockBehaviour.java`), the cached
/// `replaceable` flag on its own. This is the form `FallingBlock.isFree` reads
/// (`FallingBlock.java`); the form `FallingBlockEntity.tick` reads is
/// [`can_be_replaced_by_empty_handed_place`].
fn is_replaceable(block_state_id: i32) -> bool {
    vanilla_block_state_support_facts(block_state_id).is_some_and(|facts| facts.replaceable())
}

/// `BlockState.canBeReplaced(BlockPlaceContext)` (`BlockBehaviour.java`)
/// for the one context `FallingBlockEntity.tick` builds (`:184-186`): a
/// `DirectionalPlaceContext` holding `ItemStack.EMPTY`, clicked face `UP`,
/// whose `getClickedPos` is the tested position itself
/// (`DirectionalPlaceContext.java`) and whose `isSecondaryUseActive` is
/// false (`:63-66`).
///
/// `BlockBehaviour.canBeReplaced(BlockState, BlockPlaceContext)` (`:270-272`)
/// is `state.canBeReplaced() && (stack.isEmpty() || !stack.is(asItem()))`,
/// which an empty stack collapses to the cached flag -- but that is only the
/// *base* implementation, and twelve classes override it. Six of them still
/// reach that `super` here, because every branch they add is gated on the held
/// stack being the block's own item, which `ItemStack.EMPTY` never is:
/// `SeaPickleBlock:96-101`, `CandleBlock:97-102`, `TurtleEggBlock:154-159`,
/// `FlowerBedBlock:54-57` and `LeafLitterBlock:48-51` (both through
/// `SegmentableBlock:43-45`), and `GrowingPlantBodyBlock:96-100`.
///
/// The other six do not, and three of those disagree with the flag outright:
/// snow layers deeper than one, a vine with every face set, and sculk vein,
/// which is the one `MultifaceBlock` vanilla does not register `.replaceable()`
/// (`Blocks.java`).
///
/// The owning class comes from the generated `getStateForPlacement` class
/// column, which names the class rather than the block, the same way
/// [`crate::block_behavior`] reads crops off the `canSurvive` column. Each of
/// the six overrides `getStateForPlacement` in the same class that overrides
/// `canBeReplaced`, and the only subclass among them,
/// `WeatheringCopperSlabBlock`, overrides neither.
pub fn can_be_replaced_by_empty_handed_place(block_state_id: i32) -> bool {
    match placement_owner(block_state_id) {
        // `ScaffoldingBlock.canBeReplaced` (`ScaffoldingBlock.java`) is
        // the held stack alone; `SlabBlock.canBeReplaced`
        // (`SlabBlock.java`) refuses outright when the stack is not its
        // own item (`:87-88`); `PitcherCropBlock.canBeReplaced`
        // (`PitcherCropBlock.java`) is an unconditional false.
        Some(
            VanillaGetStateForPlacementOverride::ScaffoldingBlock
            | VanillaGetStateForPlacementOverride::SlabBlock
            | VanillaGetStateForPlacementOverride::PitcherCropBlock,
        ) => false,
        // `MultifaceBlock.canBeReplaced` (`MultifaceBlock.java`) is
        // `!stack.is(asItem()) || hasAnyVacantFace(state)`, whose first
        // disjunct an empty stack satisfies.
        Some(VanillaGetStateForPlacementOverride::MultifaceBlock) => true,
        // `SnowLayerBlock.canBeReplaced` (`SnowLayerBlock.java`): with
        // any stack that is not a snow layer the answer is `layers == 1`.
        Some(VanillaGetStateForPlacementOverride::SnowLayerBlock) => {
            snow_layers(block_state_id) == Some(1)
        }
        // `VineBlock.canBeReplaced` (`VineBlock.java`): the clicked
        // position is this position, so `clickedState.is(this)` holds and the
        // answer is `countFaces(state) < PROPERTY_BY_DIRECTION.size()`.
        Some(VanillaGetStateForPlacementOverride::VineBlock) => {
            vine_has_a_vacant_face(block_state_id)
        }
        _ => is_replaceable(block_state_id),
    }
}

/// The class that owns a block's `getStateForPlacement`, read off the generated
/// 26.1 block catalog. It names the block's class, which is what a `canBeReplaced`
/// dispatch needs and what the catalog cannot be asked for directly.
fn placement_owner(block_state_id: i32) -> Option<VanillaGetStateForPlacementOverride> {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
}

/// `state.getValue(SnowLayerBlock.LAYERS)` (`SnowLayerBlock.java`), read
/// off the generated state identity.
fn snow_layers(block_state_id: i32) -> Option<i32> {
    vanilla_block_state_property(block_state_id, "layers")?
        .parse()
        .ok()
}

/// `VineBlock.countFaces(state) < VineBlock.PROPERTY_BY_DIRECTION.size()`
/// (`VineBlock.java`, `:33-37`).
///
/// `VineBlock.createBlockStateDefinition` adds exactly the five face properties
/// and nothing else (`VineBlock.java`), so "some property is not
/// `true`" is that comparison without writing the five names down; a test in
/// this module pins the state definition it rests on.
fn vine_has_a_vacant_face(block_state_id: i32) -> bool {
    vanilla_block_state_identity(block_state_id)
        .is_some_and(|identity| identity.properties.values().any(|value| value != "true"))
}

/// `FluidState.is(FluidTags.WATER)` over `BlockState.getFluidState()`
/// (`FallingBlockEntity.java`, `:165`). The tag holds both water
/// singletons (`FluidTags.java`), which is the family.
fn is_in_water_tag(level: &LevelCtx<'_, '_>, block_state_id: i32) -> bool {
    fluid_state_tag_membership(
        level,
        block_state_fluid_state(block_state_id),
        "minecraft:water",
    )
}

/// `FluidState.is(Fluids.WATER)` over `BlockState.getFluidState()`
/// (`FallingBlockEntity.java`).
///
/// `TypedInstance.is(T)` is `typeHolder().value() == rawType`
/// (`net/minecraft/core/TypedInstance.java`), so this is the source
/// singleton alone: `Fluids.FLOWING_WATER` is a different `Fluid` and fails it.
fn is_water_source(block_state_id: i32) -> bool {
    block_state_fluid_state(block_state_id).identity() == VanillaFluidIdentity::Water
}

/// `BlockState.is(Blocks.MOVING_PISTON)` (`FallingBlockEntity.java`).
fn is_moving_piston(block_state_id: i32) -> bool {
    static ID: OnceLock<Option<i32>> = OnceLock::new();
    let Some(moving_piston) = *ID.get_or_init(|| {
        vanilla_block_type_for_resource_id("minecraft:moving_piston").map(|entry| entry.id)
    }) else {
        return false;
    };
    vanilla_block_state_block_type_id(block_state_id) == Some(moving_piston)
}

/// `BlockState.hasProperty(WATERLOGGED)` followed by `setValue`
/// (`FallingBlockEntity.java`, `:191`).
///
/// `None` when the block has no `waterlogged` property, which is the branch
/// vanilla's ternary and `hasProperty` guard take.
fn with_waterlogged(block_state_id: i32, waterlogged: bool) -> Option<i32> {
    with_property(
        block_state_id,
        "waterlogged",
        if waterlogged { "true" } else { "false" },
    )
}

mod fall_damage;
mod fallable_hooks;
pub(crate) use fall_damage::cause_fall_damage_inner;
