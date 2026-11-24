//! Painting, item-frame and glow-item-frame live runtime.
//!

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_state_name, vanilla_block_state_support_facts,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ITEM_FRAME_ITEM_DATA_ID, VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID;
use bbb_protocol::packets::{BlockPos, Direction, ItemMapDecorationSummary, SoundSource, Vec3d};
use bbb_server_types::directions::{counter_clock_wise, opposite, step_vec};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::item::{plain_stack, spawn_dropped_item_entity};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::geometry::{
    collect_block_collisions_with_moving_pistons, Aabb, Axis, Vec3Ops,
};

mod movement;
pub(crate) use movement::move_by_piston;

mod post_actions;
pub(crate) use post_actions::{
    apply_hanging_post_actions, apply_hanging_post_actions_in_level, hurt_server_with_post_actions,
    ResolvedHangingPostActions,
};

/// `HangingEntity.DEFAULT_DIRECTION` (`HangingEntity.java:27`).
const DEFAULT_DIRECTION: Direction = Direction::South;
/// `Painting.DEPTH` / `ItemFrame.DEPTH` (`Painting.java:43`, `ItemFrame.java:48`).
const DEPTH: f64 = 0.0625;
/// `Painting` / `ItemFrame` `shiftToBlockWall` (`Painting.java:152`, `ItemFrame.java:114`).
const SHIFT_TO_WALL: f64 = 0.46875;
/// `ItemFrame.WIDTH` / `HEIGHT` (`ItemFrame.java:49-50`).
const FRAME_SIZE: f64 = 0.75;
/// `HangingEntity.spawnAtLocation` step scale (`HangingEntity.java:122-124`).
const DROP_STEP: f64 = 0.15;
/// `BlockAttachedEntity.checkInterval` period (`BlockAttachedEntity.java:42`).
const SURVIVAL_CHECK_INTERVAL: i32 = 100;
/// `LeashFenceKnotEntity.dropItem` (`LeashFenceKnotEntity.java:61-63`).
/// `SoundEvents.LEAD_UNTIED` (`SoundEvents.java:919`).
const LEASH_UNTIED_SOUND: &str = "minecraft:item.lead.untied";
const ITEM_FRAME_BREAK_SOUND: &str = "minecraft:entity.item_frame.break";
const GLOW_ITEM_FRAME_BREAK_SOUND: &str = "minecraft:entity.glow_item_frame.break";
const ITEM_FRAME_ADD_ITEM_SOUND: &str = "minecraft:entity.item_frame.add_item";
const GLOW_ITEM_FRAME_ADD_ITEM_SOUND: &str = "minecraft:entity.glow_item_frame.add_item";
const ITEM_FRAME_REMOVE_ITEM_SOUND: &str = "minecraft:entity.item_frame.remove_item";
const GLOW_ITEM_FRAME_REMOVE_ITEM_SOUND: &str = "minecraft:entity.glow_item_frame.remove_item";
const ITEM_FRAME_ROTATE_ITEM_SOUND: &str = "minecraft:entity.item_frame.rotate_item";
const GLOW_ITEM_FRAME_ROTATE_ITEM_SOUND: &str = "minecraft:entity.glow_item_frame.rotate_item";
const PAINTING_BREAK_SOUND: &str = "minecraft:entity.painting.break";
/// `ItemFrame.NUM_ROTATIONS` (`ItemFrame.java:47`).
const NUM_ROTATIONS: i32 = 8;
/// `ItemFrame.DEFAULT_DROP_CHANCE` (`ItemFrame.java:52`).
const DEFAULT_DROP_CHANCE: f32 = 1.0;
/// `AABB.deflate(1.0E-7)` on the support box (`HangingEntity.java:95`).
const SUPPORT_DEFLATE: f64 = 1.0e-7;

/// Shared hanging pose: attached cell, facing, and the 100-tick survival counter.
#[derive(Debug, Clone, PartialEq)]
pub struct HangingPose {
    attached: BlockPos,
    direction: Direction,
    check_interval: i32,
}

/// `LeashFenceKnotEntity`'s attached block and inherited check counter.
#[derive(Debug, Clone, PartialEq)]
pub struct LeashKnotData {
    attached: BlockPos,
    check_interval: i32,
}

impl LeashKnotData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.check_interval = previous.check_interval;
    }

    pub const fn new(attached: BlockPos) -> Self {
        Self {
            attached,
            check_interval: 0,
        }
    }

    pub const fn attached(&self) -> BlockPos {
        self.attached
    }
}

impl HangingPose {
    fn restore_command_transients(&mut self, previous: &Self) {
        self.check_interval = previous.check_interval;
    }

    pub fn new(attached: BlockPos, direction: Direction) -> Self {
        Self {
            attached,
            direction,
            check_interval: 0,
        }
    }

    pub const fn attached(&self) -> BlockPos {
        self.attached
    }

    pub const fn direction(&self) -> Direction {
        self.direction
    }
}

/// A `Holder<PaintingVariant>` of the live `minecraft:painting_variant`
/// registry, with the size the bounding box reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintingVariant {
    resource_id: String,
    holder_id: i32,
    width: i32,
    height: i32,
}

pub(crate) const PAINTING_VARIANT_REGISTRY: &str = "minecraft:painting_variant";

impl PaintingVariant {
    /// The published element named `resource_id`; a detached store without a
    /// publication answers from the built-in 26.1 catalog.
    pub(crate) fn resolve(
        items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        resource_id: &str,
    ) -> Option<Self> {
        let vanilla =
            bbb_protocol::painting_variant_types::vanilla_painting_variant_for_resource_id(
                resource_id,
            );
        let holder_id = items
            .holder_id(PAINTING_VARIANT_REGISTRY, resource_id)
            .or(vanilla.map(|variant| variant.id))?;
        let [width, height] = items
            .painting_variant_size(resource_id)
            .or(vanilla.map(|variant| [variant.width, variant.height]))?;
        Some(Self {
            resource_id: resource_id.to_string(),
            holder_id,
            width,
            height,
        })
    }

    /// `VariantUtils.getAny`, the synched default (`Painting.java:52`): the
    /// first element (`MappedRegistry.java:165-167`).
    pub(crate) fn default_variant(
        items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> Self {
        items
            .holder_resource_id(PAINTING_VARIANT_REGISTRY, 0)
            .and_then(|id| Self::resolve(items, &id))
            .unwrap_or(Self {
                resource_id: "minecraft:kebab".into(),
                holder_id: 0,
                width: 1,
                height: 1,
            })
    }

    /// The built-in 26.1 element, for fixtures without a live publication.
    #[cfg(test)]
    pub(crate) fn vanilla(resource_id: &str) -> Self {
        Self::resolve(&bbb_registry_codec::UnavailableHolderResolver, resource_id)
            .expect("a built-in painting variant")
    }

    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub const fn holder_id(&self) -> i32 {
        self.holder_id
    }

    pub const fn area(&self) -> i32 {
        self.width * self.height
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintingData {
    pose: HangingPose,
    variant: PaintingVariant,
}

impl PaintingData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.pose.restore_command_transients(&previous.pose);
    }

    pub fn new(attached: BlockPos, direction: Direction, variant: PaintingVariant) -> Self {
        Self {
            pose: HangingPose::new(attached, force_horizontal(direction)),
            variant,
        }
    }

    pub const fn pose(&self) -> &HangingPose {
        &self.pose
    }

    pub const fn variant(&self) -> &PaintingVariant {
        &self.variant
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameData {
    pose: HangingPose,
    item: Option<RuntimeItemStack>,
    rotation: i32,
    drop_chance: f32,
    fixed: bool,
    invisible: bool,
}

impl ItemFrameData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.pose.restore_command_transients(&previous.pose);
    }

    pub fn new(attached: BlockPos, direction: Direction) -> Self {
        Self {
            pose: HangingPose::new(attached, direction),
            item: None,
            rotation: 0,
            drop_chance: DEFAULT_DROP_CHANCE,
            fixed: false,
            invisible: false,
        }
    }

    pub const fn pose(&self) -> &HangingPose {
        &self.pose
    }

    pub fn item(&self) -> Option<&RuntimeItemStack> {
        self.item.as_ref()
    }

    pub const fn rotation(&self) -> i32 {
        self.rotation
    }

    pub const fn drop_chance(&self) -> f32 {
        self.drop_chance
    }

    pub const fn fixed(&self) -> bool {
        self.fixed
    }

    pub const fn invisible(&self) -> bool {
        self.invisible
    }

    pub fn with_saved_fields(
        mut self,
        item: Option<RuntimeItemStack>,
        rotation: i32,
        drop_chance: f32,
        fixed: bool,
        invisible: bool,
    ) -> Self {
        self.item = item;
        self.rotation = rotation % NUM_ROTATIONS;
        self.drop_chance = drop_chance;
        self.fixed = fixed;
        self.invisible = invisible;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramedMapAdmission {
    NotFramedMap,
    SavedDataMissing,
    Present { tracked_decoration_count: usize },
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedMapFrame {
    pub map_id: i32,
    pub entity_id: i32,
    pub attached: BlockPos,
    pub direction: Direction,
    pub static_decorations: Vec<ItemMapDecorationSummary>,
}

pub(crate) fn framed_map_frame(entity: &Entity) -> Option<FramedMapFrame> {
    let frame = match entity.data() {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame,
        _ => return None,
    };
    let item = frame.item()?;
    if item.item_id() != Some(VANILLA_ITEM_FILLED_MAP_ID) {
        return None;
    }
    Some(FramedMapFrame {
        map_id: item.map_id()?,
        entity_id: entity.id(),
        attached: frame.pose().attached(),
        direction: frame.pose().direction(),
        static_decorations: item
            .summary()
            .component_patch
            .map_decorations
            .clone()
            .unwrap_or_default(),
    })
}

pub(crate) fn framed_map_removal(
    entity: &Entity,
) -> Option<crate::entity::store::FramedMapRemoval> {
    let frame = match entity.data() {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame,
        _ => return None,
    };
    Some(crate::entity::store::FramedMapRemoval {
        map_id: frame.item()?.map_id()?,
        pos: frame.pose().attached(),
        entity_id: entity.id(),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum HangInteract {
    Pass,
    Fail,
    Add {
        item: RuntimeItemStack,
        sound_resource_id: &'static str,
        sound_position: Vec3d,
        attached: BlockPos,
    },
    Rotate {
        previous_rotation: i32,
        rotation: i32,
        sound_resource_id: &'static str,
        sound_position: Vec3d,
        attached: BlockPos,
    },
}

pub fn spawn_painting(
    store: &mut EntityStore,
    attached: BlockPos,
    direction: Direction,
    variant: PaintingVariant,
) -> Option<i32> {
    let data = EntityData::Painting(PaintingData::new(attached, direction, variant));
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_PAINTING_ID,
        Vec3d {
            x: f64::from(attached.x) + 0.5,
            y: f64::from(attached.y) + 0.5,
            z: f64::from(attached.z) + 0.5,
        },
        data,
        apply_direction,
    )
}

pub fn spawn_item_frame(
    store: &mut EntityStore,
    attached: BlockPos,
    direction: Direction,
    glow: bool,
) -> Option<i32> {
    let (type_id, data) = if glow {
        (
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            EntityData::GlowItemFrame(ItemFrameData::new(attached, direction)),
        )
    } else {
        (
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            EntityData::ItemFrame(ItemFrameData::new(attached, direction)),
        )
    };
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        type_id,
        Vec3d {
            x: f64::from(attached.x) + 0.5,
            y: f64::from(attached.y) + 0.5,
            z: f64::from(attached.z) + 0.5,
        },
        data,
        apply_direction,
    )
}

/// A painting or item frame built at `attached` but not filed, for the
/// `survives` test `HangingEntityItem.useOn` runs before `addFreshEntity`
/// (`HangingEntityItem.java:46-64`).
pub(crate) fn placement_candidate(
    entity_type_id: i32,
    attached: BlockPos,
    direction: Direction,
    painting_variant: Option<PaintingVariant>,
) -> Option<Entity> {
    let data = match entity_type_id {
        VANILLA_ENTITY_TYPE_PAINTING_ID => {
            EntityData::Painting(PaintingData::new(attached, direction, painting_variant?))
        }
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID => {
            EntityData::ItemFrame(ItemFrameData::new(attached, direction))
        }
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID => {
            EntityData::GlowItemFrame(ItemFrameData::new(attached, direction))
        }
        _ => return None,
    };
    let mut entity = Entity::new(
        i32::MIN,
        uuid::Uuid::nil(),
        entity_type_id,
        Vec3d::default(),
        data,
    );
    apply_direction(&mut entity);
    Some(entity)
}

/// Files a [`placement_candidate`] under a fresh UUID.
pub(crate) fn spawn_candidate(store: &mut EntityStore, candidate: &Entity) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        candidate.entity_type_id(),
        candidate.position(),
        candidate.data().clone(),
        apply_direction,
    )
}

pub fn spawn_structure_item_frame(
    store: &mut EntityStore,
    attached: BlockPos,
    direction: Direction,
    item: RuntimeItemStack,
) -> Option<i32> {
    let item = crate::entity::item::stack_with_count(&item, 1)?;
    let uuid = store.next_uuid();
    let result = store.restore_initialized_with_effects(
        uuid,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        Vec3d::default(),
        EntityData::ItemFrame(ItemFrameData::new(attached, direction)),
        (false, &mut 0),
        |entity, store| {
            apply_direction(entity);
            set_frame_item(entity, item, |_, sound| store.play_entity_sound(sound));
            true
        },
    )?;
    match result {
        crate::entity::store::EntityRestoreOutcome::Published(id) => Some(id),
        _ => None,
    }
}

fn set_frame_item(
    entity: &mut Entity,
    item: RuntimeItemStack,
    emit_sound: impl FnOnce(&Entity, PendingEntitySound),
) {
    let sound = if matches!(entity.data(), EntityData::GlowItemFrame(_)) {
        GLOW_ITEM_FRAME_ADD_ITEM_SOUND
    } else {
        ITEM_FRAME_ADD_ITEM_SOUND
    };
    let nonempty = !item.is_empty();
    frame_mut(entity).expect("item frame").item = nonempty.then_some(item);
    entity.mark_synched_dirty(VANILLA_ITEM_FRAME_ITEM_DATA_ID);
    recalculate_bounding_box(entity);
    if !nonempty {
        return;
    }
    emit_sound(
        entity,
        PendingEntitySound {
            position: entity.position(),
            resource_id: sound.to_owned(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        },
    );
}

pub(crate) fn set_slot_item(
    entity: &mut Entity,
    item: RuntimeItemStack,
) -> Result<(Option<PendingEntitySound>, BlockPos), String> {
    let attached = frame_mut(entity)
        .ok_or("entity is not an item frame")?
        .pose
        .attached;
    let item = if item.is_empty() {
        item
    } else {
        item.with_count(1)
    };
    let mut sound = None;
    set_frame_item(entity, item, |_, value| sound = Some(value));
    Ok((sound, attached))
}

pub fn spawn_leash_knot(store: &mut EntityStore, attached: BlockPos) -> Option<i32> {
    let data = EntityData::LeashKnot(LeashKnotData::new(attached));
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
        leash_knot_position(attached),
        data,
        apply_leash_knot_pose,
    )
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<ResolvedHangingPostActions> {
    if entity.block_position().y < level.min_y() {
        entity.discard();
        return Some(ResolvedHangingPostActions::empty());
    }
    let due = match &mut entity.data {
        EntityData::LeashKnot(knot) => {
            let due = knot.check_interval == SURVIVAL_CHECK_INTERVAL;
            knot.check_interval += 1;
            if due {
                knot.check_interval = 0;
            }
            due
        }
        _ => {
            let Some(pose) = pose_mut(entity) else {
                return Some(ResolvedHangingPostActions::empty());
            };
            let due = pose.check_interval == SURVIVAL_CHECK_INTERVAL;
            pose.check_interval += 1;
            if due {
                pose.check_interval = 0;
            }
            due
        }
    };
    if !due {
        return Some(ResolvedHangingPostActions::empty());
    }
    if entity.removal_reason().is_some() || survives(entity, store, level) {
        return Some(ResolvedHangingPostActions::empty());
    }
    let frame = matches!(
        entity.data(),
        EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_)
    );
    entity.discard();
    let mut actions = prepare_drop_contents(entity, level.entity_drops(), true, true, false);
    if frame {
        actions.push(HangingPostAction::BlockChange(entity.block_position()));
    }
    post_actions::resolve(store, actions)
}

/// `HangingEntity.survives` / `ItemFrame.survives`.
pub(crate) fn survives(entity: &Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) -> bool {
    match entity.data() {
        EntityData::LeashKnot(knot) => {
            level.block_state_in_tag(level.block_state(knot.attached), "minecraft:fences")
        }
        EntityData::Painting(_) => hanging_survives(entity, store, level, false),
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => {
            if frame.fixed {
                return true;
            }
            item_frame_survives(entity, store, level)
        }
        _ => false,
    }
}

fn hanging_survives(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    allow_same_type: bool,
) -> bool {
    if has_level_collision(level, pop_box(entity)) {
        return false;
    }
    support_positions(entity).into_iter().all(|pos| {
        let state = level.block_state(pos);
        legacy_solid(state) || is_diode(state)
    }) && can_coexist(entity, store, allow_same_type)
}

fn item_frame_survives(entity: &Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) -> bool {
    if has_level_collision(level, pop_box(entity)) {
        return false;
    }
    let pose = pose_of(entity.data());
    let support = bbb_server_types::directions::relative(pose.attached, opposite(pose.direction));
    let state = level.block_state(support);
    if !(legacy_solid(state) || (is_horizontal(pose.direction) && is_diode(state))) {
        return false;
    }
    can_coexist(entity, store, true)
}

/// `ItemFrame.hurtServer` two-step drop, else `BlockAttachedEntity.hurtServer`.
pub(crate) fn hurt_server(
    entity: &mut Entity,
    entity_drops: bool,
    source: HangHurtSource,
) -> HangingHurtEffects {
    let refused = || HangingHurtEffects {
        outcome: false,
        actions: Vec::new(),
        emits_entity_die: false,
    };
    let fixed = match entity.data() {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame.fixed,
        _ => false,
    };
    if fixed && !(source.bypasses_invulnerability || source.creative_player) {
        return refused();
    }
    if entity.removal_reason().is_some()
        || entity.invulnerable() && !source.bypasses_invulnerability && !source.creative_player
        || source.is_fire && entity.fire_immune()
        || source.is_fall && source.fall_damage_immune
    {
        return refused();
    }
    if matches!(entity.data(), EntityData::LeashKnot(_)) {
        if !source.mob_griefing && source.from_mob {
            return refused();
        }
        entity.set_removed(crate::entity::RemovalReason::Killed);
        entity.mark_hurt();
        return HangingHurtEffects {
            outcome: true,
            actions: prepare_drop_contents(
                entity,
                entity_drops,
                true,
                false,
                source.creative_player,
            ),
            emits_entity_die: true,
        };
    }
    let framed_item = match entity.data() {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame.item.clone(),
        _ => None,
    };
    if !fixed && !source.is_explosion && framed_item.is_some() {
        let mut actions = prepare_drop_contents(
            entity,
            entity_drops,
            false,
            entity_drops || !source.caused_by_entity,
            source.creative_player,
        );
        actions.push(HangingPostAction::BlockChange(entity.block_position()));
        if !entity.is_silent() {
            actions.push(HangingPostAction::Sound {
                position: entity.position(),
                resource_id: frame_remove_item_sound(entity),
            });
        }
        return HangingHurtEffects {
            outcome: true,
            actions,
            emits_entity_die: false,
        };
    }
    if !source.mob_griefing && source.from_mob {
        return refused();
    }
    let frame = matches!(
        entity.data(),
        EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_)
    );
    entity.set_removed(crate::entity::RemovalReason::Killed);
    entity.mark_hurt();
    let mut actions =
        prepare_drop_contents(entity, entity_drops, true, true, source.creative_player);
    if frame {
        actions.push(HangingPostAction::BlockChange(entity.block_position()));
    }
    HangingHurtEffects {
        outcome: true,
        actions,
        emits_entity_die: true,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HangHurtSource {
    pub is_explosion: bool,
    pub is_fire: bool,
    pub is_fall: bool,
    pub fall_damage_immune: bool,
    pub from_mob: bool,
    pub mob_griefing: bool,
    pub caused_by_entity: bool,
    pub creative_player: bool,
    pub bypasses_invulnerability: bool,
}

pub(crate) struct ResolvedHangingHurtEffects {
    pub(crate) outcome: bool,
    pub(crate) actions: ResolvedHangingPostActions,
    pub(crate) emits_entity_die: bool,
}

pub(crate) struct HangingHurtEffects {
    pub(crate) outcome: bool,
    pub(crate) actions: Vec<HangingPostAction>,
    /// Whether the body reached `BlockAttachedEntity.hurtServer`'s
    /// `this.kill(level)` (`:84`), whose `ENTITY_DIE` the caller raises.
    pub(crate) emits_entity_die: bool,
}

pub(crate) enum HangingPostAction {
    Sound {
        position: Vec3d,
        resource_id: &'static str,
    },
    Drop {
        position: Vec3d,
        stack: RuntimeItemStack,
    },
    DropWithChance {
        position: Vec3d,
        stack: RuntimeItemStack,
        chance: f32,
    },
    BlockChange(BlockPos),
    RemoveFramedMap(crate::entity::store::FramedMapRemoval),
    UpdateOutputSignal(BlockPos),
}

pub fn prepare_interact(
    entity: &Entity,
    held: Option<RuntimeItemStack>,
    map_admission: FramedMapAdmission,
) -> HangInteract {
    if entity.removal_reason().is_some() {
        return HangInteract::Pass;
    }
    let (frame, add_sound, rotate_sound) = match entity.data() {
        EntityData::ItemFrame(frame) => (
            frame,
            ITEM_FRAME_ADD_ITEM_SOUND,
            ITEM_FRAME_ROTATE_ITEM_SOUND,
        ),
        EntityData::GlowItemFrame(frame) => (
            frame,
            GLOW_ITEM_FRAME_ADD_ITEM_SOUND,
            GLOW_ITEM_FRAME_ROTATE_ITEM_SOUND,
        ),
        _ => return HangInteract::Pass,
    };
    if frame.fixed {
        return HangInteract::Pass;
    }
    if frame.item.is_none() {
        let Some(stack) = held.filter(|stack| !stack_is_empty(stack)) else {
            return HangInteract::Pass;
        };
        let framed_map = stack_has_map_id(Some(&stack));
        let admitted = match (framed_map, map_admission) {
            (false, FramedMapAdmission::NotFramedMap)
            | (true, FramedMapAdmission::SavedDataMissing) => true,
            (
                true,
                FramedMapAdmission::Present {
                    tracked_decoration_count,
                },
            ) => tracked_decoration_count <= 256,
            _ => false,
        };
        if !admitted {
            return HangInteract::Fail;
        }
        HangInteract::Add {
            item: copy_count_one(stack),
            sound_resource_id: add_sound,
            sound_position: entity.position(),
            attached: frame.pose.attached,
        }
    } else {
        HangInteract::Rotate {
            previous_rotation: frame.rotation,
            rotation: frame.rotation.wrapping_add(1) % NUM_ROTATIONS,
            sound_resource_id: rotate_sound,
            sound_position: entity.position(),
            attached: frame.pose.attached,
        }
    }
}

pub fn apply_interact(
    entity: &mut Entity,
    plan: &HangInteract,
    emit_sound: impl FnOnce(&Entity, PendingEntitySound),
) -> bool {
    match plan {
        HangInteract::Pass | HangInteract::Fail => false,
        HangInteract::Add {
            item,
            sound_resource_id,
            sound_position,
            attached,
        } => {
            let valid = entity.removal_reason().is_none()
                && item.count() == 1
                && *sound_position == entity.position()
                && (matches!(
                    entity.data(),
                    EntityData::ItemFrame(frame)
                        if !frame.fixed
                            && frame.item.is_none()
                            && frame.pose.attached == *attached
                            && *sound_resource_id == ITEM_FRAME_ADD_ITEM_SOUND
                ) || matches!(
                    entity.data(),
                    EntityData::GlowItemFrame(frame)
                        if !frame.fixed
                            && frame.item.is_none()
                            && frame.pose.attached == *attached
                            && *sound_resource_id == GLOW_ITEM_FRAME_ADD_ITEM_SOUND
                ));
            if !valid {
                return false;
            }
            set_frame_item(entity, item.clone(), emit_sound);
            true
        }
        HangInteract::Rotate {
            previous_rotation,
            rotation,
            sound_resource_id,
            sound_position,
            attached,
        } => {
            let valid = entity.removal_reason().is_none()
                && *sound_position == entity.position()
                && (matches!(
                    entity.data(),
                    EntityData::ItemFrame(frame)
                        if !frame.fixed
                            && frame.item.is_some()
                            && frame.rotation == *previous_rotation
                            && frame.pose.attached == *attached
                            && *sound_resource_id == ITEM_FRAME_ROTATE_ITEM_SOUND
                ) || matches!(
                    entity.data(),
                    EntityData::GlowItemFrame(frame)
                        if !frame.fixed
                            && frame.item.is_some()
                            && frame.rotation == *previous_rotation
                            && frame.pose.attached == *attached
                            && *sound_resource_id == GLOW_ITEM_FRAME_ROTATE_ITEM_SOUND
                ));
            if !valid || *rotation != previous_rotation.wrapping_add(1) % NUM_ROTATIONS {
                return false;
            }
            emit_sound(
                entity,
                PendingEntitySound {
                    position: *sound_position,
                    resource_id: (*sound_resource_id).to_owned(),
                    source: SoundSource::Neutral,
                    volume: 1.0,
                    pitch: 1.0,
                },
            );
            frame_mut(entity).expect("validated item frame").rotation = *rotation;
            entity.mark_synched_dirty(VANILLA_ITEM_FRAME_ROTATION_DATA_ID);
            true
        }
    }
}

pub fn transform_template_pose(
    entity: &mut Entity,
    quarter_turns: u8,
    mirror: bbb_protocol::block_state_types::VanillaBlockStateMirror,
) -> Option<[f32; 2]> {
    use bbb_protocol::block_state_types::VanillaBlockStateMirror;
    if !matches!(
        entity.data(),
        EntityData::Painting(_) | EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_)
    ) {
        return None;
    }
    let rotated = rotate_template_hanging(entity, quarter_turns);
    let direction = pose_of(entity.data()).direction;
    let mirrored_turns = match (mirror, direction) {
        (VanillaBlockStateMirror::LeftRight, Direction::North | Direction::South)
        | (VanillaBlockStateMirror::FrontBack, Direction::West | Direction::East) => 2,
        _ => 0,
    };
    let mirrored = rotate_template_hanging(entity, mirrored_turns);
    Some([rotated + mirrored - entity.y_rot(), entity.x_rot()])
}

fn rotate_template_hanging(entity: &mut Entity, quarter_turns: u8) -> f32 {
    let direction = pose_of(entity.data()).direction;
    if is_horizontal(direction) {
        let direction = match quarter_turns % 4 {
            1 => counter_clock_wise(opposite(direction)),
            2 => opposite(direction),
            3 => counter_clock_wise(direction),
            _ => direction,
        };
        pose_mut(entity).expect("hanging pose").direction = direction;
        apply_direction(entity);
    }
    let mut yaw = entity.y_rot() % 360.0;
    if yaw >= 180.0 {
        yaw -= 360.0;
    }
    if yaw < -180.0 {
        yaw += 360.0;
    }
    yaw + match quarter_turns % 4 {
        1 => 270.0,
        2 => 180.0,
        3 => 90.0,
        _ => 0.0,
    }
}

pub fn apply_direction(entity: &mut Entity) {
    match entity.data() {
        EntityData::Painting(_) => {
            let direction = force_horizontal(pose_of(entity.data()).direction);
            if let Some(pose) = pose_mut(entity) {
                pose.direction = direction;
            }
            entity.set_rot(f32::from(horizontal_id(direction)) * 90.0, 0.0);
        }
        EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_) => {
            let direction = pose_of(entity.data()).direction;
            if is_horizontal(direction) {
                entity.set_rot(f32::from(horizontal_id(direction)) * 90.0, 0.0);
            } else {
                let x_rot = -90.0 * f32::from(axis_step(direction));
                entity.set_rot(0.0, x_rot);
            }
        }
        _ => return,
    }
    recalculate_bounding_box(entity);
}

fn apply_leash_knot_pose(entity: &mut Entity) {
    let EntityData::LeashKnot(knot) = entity.data() else {
        return;
    };
    let position = leash_knot_position(knot.attached);
    let [width, height] = bbb_protocol::entity_types::vanilla_entity_type_dimensions(
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
    )
    .expect("built-in leash knot dimensions");
    entity.set_pos_raw_and_bounding_box(position, Aabb::of_entity(position, width, height));
}

pub fn recalculate_bounding_box(entity: &mut Entity) {
    let aabb = calculated_box(entity.data());
    entity.set_pos_raw_and_bounding_box(aabb.center(), aabb);
}

pub fn set_attached_pos(entity: &mut Entity, position: Vec3d) -> bool {
    let attached = crate::entity::containing_block(position);
    match &mut entity.data {
        EntityData::Painting(painting) => painting.pose.attached = attached,
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => {
            frame.pose.attached = attached;
        }
        EntityData::LeashKnot(knot) => {
            knot.attached = attached;
            apply_leash_knot_pose(entity);
            entity.set_needs_sync(true);
            return true;
        }
        _ => return false,
    }
    recalculate_bounding_box(entity);
    entity.set_needs_sync(true);
    true
}

pub fn calculated_box(data: &EntityData) -> Aabb {
    match data {
        EntityData::Painting(painting) => painting_box(painting),
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame_box(
            &frame.pose,
            frame.item.is_some() && stack_has_map_id(frame.item.as_ref()),
        ),
        _ => Aabb::of_size(
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            0.0,
            0.0,
            0.0,
        ),
    }
}

fn painting_box(painting: &PaintingData) -> Aabb {
    let direction = painting.pose.direction;
    let width = f64::from(painting.variant.width);
    let height = f64::from(painting.variant.height);
    let wall = attached_wall(&painting.pose);
    let left = counter_clock_wise(direction);
    let position = relative(
        relative(wall, left, offset_for_size(width)),
        Direction::Up,
        offset_for_size(height),
    );
    let axis = axis_of(direction);
    let x_size = if axis == Axis::X { DEPTH } else { width };
    let z_size = if axis == Axis::Z { DEPTH } else { width };
    Aabb::of_size(position, x_size, height, z_size)
}

fn frame_box(pose: &HangingPose, has_framed_map: bool) -> Aabb {
    let direction = pose.direction;
    let position = attached_wall(pose);
    let size = if has_framed_map { 1.0 } else { FRAME_SIZE };
    let axis = axis_of(direction);
    let x_size = if axis == Axis::X { DEPTH } else { size };
    let y_size = if axis == Axis::Y { DEPTH } else { size };
    let z_size = if axis == Axis::Z { DEPTH } else { size };
    Aabb::of_size(position, x_size, y_size, z_size)
}

fn pop_box(entity: &Entity) -> Aabb {
    match entity.data() {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => {
            frame_box(&frame.pose, false)
        }
        _ => entity.bounding_box(),
    }
}

fn attached_wall(pose: &HangingPose) -> Vec3d {
    relative(block_center(pose.attached), pose.direction, -SHIFT_TO_WALL)
}

fn offset_for_size(size: f64) -> f64 {
    if (size as i32) % 2 == 0 {
        0.5
    } else {
        0.0
    }
}

fn prepare_drop_contents(
    entity: &mut Entity,
    entity_drops: bool,
    with_frame: bool,
    remove_framed_map: bool,
    caused_by_creative_player: bool,
) -> Vec<HangingPostAction> {
    if matches!(entity.data(), EntityData::LeashKnot(_)) {
        if entity.is_silent() {
            return Vec::new();
        }
        return vec![HangingPostAction::Sound {
            position: entity.position(),
            resource_id: LEASH_UNTIED_SOUND,
        }];
    }
    let origin = drop_origin(entity);
    let kind = entity.data().entity_resource_id();
    let (fixed, framed, chance) = match entity.data() {
        EntityData::Painting(_) => (false, None, DEFAULT_DROP_CHANCE),
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => {
            (frame.fixed, frame.item.clone(), frame.drop_chance)
        }
        _ => return Vec::new(),
    };
    let mut actions = Vec::new();
    if with_frame && !entity.is_silent() {
        let sound = match kind {
            "minecraft:item_frame" => Some(ITEM_FRAME_BREAK_SOUND),
            "minecraft:glow_item_frame" => Some(GLOW_ITEM_FRAME_BREAK_SOUND),
            "minecraft:painting" if entity_drops => Some(PAINTING_BREAK_SOUND),
            _ => None,
        };
        if let Some(resource_id) = sound {
            actions.push(HangingPostAction::Sound {
                position: entity.position(),
                resource_id,
            });
        }
    }
    if kind != "minecraft:painting" && fixed {
        return actions;
    }
    if remove_framed_map {
        if let Some(removal) = framed_map_removal(entity) {
            actions.push(HangingPostAction::RemoveFramedMap(removal));
        }
    }
    if let Some(frame) = frame_mut(entity) {
        let attached = frame.pose.attached();
        frame.item = None;
        // ItemFrame.java:224,269-287: setItem(EMPTY, true) updates neighbour comparators.
        actions.push(HangingPostAction::UpdateOutputSignal(attached));
    }
    if framed.is_some() {
        entity.mark_synched_dirty(VANILLA_ITEM_FRAME_ITEM_DATA_ID);
        recalculate_bounding_box(entity);
    }
    if !entity_drops || caused_by_creative_player {
        return actions;
    }
    if kind == "minecraft:painting" {
        if let Some(stack) = plain_stack("minecraft:painting", 1) {
            actions.push(HangingPostAction::Drop {
                position: origin,
                stack,
            });
        }
        return actions;
    }
    if with_frame {
        let frame_id = if kind == "minecraft:glow_item_frame" {
            "minecraft:glow_item_frame"
        } else {
            "minecraft:item_frame"
        };
        if let Some(stack) = plain_stack(frame_id, 1) {
            actions.push(HangingPostAction::Drop {
                position: origin,
                stack,
            });
        }
    }
    if let Some(stack) = framed {
        actions.push(HangingPostAction::DropWithChance {
            position: origin,
            stack,
            chance,
        });
    }
    actions
}

fn frame_remove_item_sound(entity: &Entity) -> &'static str {
    match entity.data() {
        EntityData::ItemFrame(_) => ITEM_FRAME_REMOVE_ITEM_SOUND,
        EntityData::GlowItemFrame(_) => GLOW_ITEM_FRAME_REMOVE_ITEM_SOUND,
        _ => unreachable!("only an item frame has a remove-item sound"),
    }
}

fn drop_origin(entity: &Entity) -> Vec3d {
    let direction = pose_of(entity.data()).direction;
    let step = step_vec(direction);
    let pos = entity.position();
    Vec3d {
        x: pos.x + step.x * DROP_STEP,
        y: pos.y,
        z: pos.z + step.z * DROP_STEP,
    }
}

fn has_level_collision(level: &LevelCtx<'_, '_>, pop_box: Aabb) -> bool {
    !collect_block_collisions_with_moving_pistons(
        pop_box,
        |pos| level.is_moving_piston_at(pos).unwrap_or(true),
        |pos| level.block_collision_boxes(pos),
    )
    .is_empty()
}

fn support_positions(entity: &Entity) -> Vec<BlockPos> {
    let direction = pose_of(entity.data()).direction;
    let support = entity
        .bounding_box()
        .moved(step_vec(direction).scale(-0.5))
        .inflate(-SUPPORT_DEFLATE, -SUPPORT_DEFLATE, -SUPPORT_DEFLATE);
    closed_blocks(support)
}

fn can_coexist(entity: &Entity, store: &EntityStore, allow_same_type: bool) -> bool {
    let pop = pop_box(entity);
    let direction = pose_of(entity.data()).direction;
    let type_id = entity.entity_type_id();
    for id in store.ids_in_box_sections(pop) {
        if id == entity.id() {
            continue;
        }
        let Some(other) = store.get(id) else {
            continue;
        };
        if pose_opt(other.data()).is_none() {
            continue;
        }
        if !pop.intersects(other.bounding_box()) {
            continue;
        }
        let same_type = other.entity_type_id() == type_id;
        let same_direction = pose_of(other.data()).direction == direction;
        if (!allow_same_type && same_type) || same_direction {
            return false;
        }
    }
    true
}

fn closed_blocks(box_: Aabb) -> Vec<BlockPos> {
    let min_x = box_.min_x.floor() as i32;
    let min_y = box_.min_y.floor() as i32;
    let min_z = box_.min_z.floor() as i32;
    let max_x = box_.max_x.floor() as i32;
    let max_y = box_.max_y.floor() as i32;
    let max_z = box_.max_z.floor() as i32;
    let mut positions = Vec::new();
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                positions.push(BlockPos { x, y, z });
            }
        }
    }
    positions
}

fn legacy_solid(state: i32) -> bool {
    vanilla_block_state_support_facts(state)
        .map(|facts| facts.legacy_solid())
        .unwrap_or(false)
}

fn is_diode(state: i32) -> bool {
    matches!(
        vanilla_block_state_name(state),
        Some("minecraft:repeater" | "minecraft:comparator")
    )
}

fn leash_knot_position(attached: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(attached.x) + 0.5,
        y: f64::from(attached.y) + 0.375,
        z: f64::from(attached.z) + 0.5,
    }
}

fn pose_of(data: &EntityData) -> &HangingPose {
    pose_opt(data).expect("hanging dispatch selected a hanging entity")
}

fn pose_opt(data: &EntityData) -> Option<&HangingPose> {
    match data {
        EntityData::Painting(painting) => Some(&painting.pose),
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => Some(&frame.pose),
        _ => None,
    }
}

fn pose_mut(entity: &mut Entity) -> Option<&mut HangingPose> {
    match &mut entity.data {
        EntityData::Painting(painting) => Some(&mut painting.pose),
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => Some(&mut frame.pose),
        _ => None,
    }
}

fn frame_mut(entity: &mut Entity) -> Option<&mut ItemFrameData> {
    match &mut entity.data {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => Some(frame),
        _ => None,
    }
}

fn stack_is_empty(stack: &RuntimeItemStack) -> bool {
    stack.is_empty()
}

fn copy_count_one(stack: RuntimeItemStack) -> RuntimeItemStack {
    stack.with_count(1)
}

fn stack_has_map_id(item: Option<&RuntimeItemStack>) -> bool {
    item.and_then(RuntimeItemStack::map_id).is_some()
}

fn block_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

fn relative(origin: Vec3d, direction: Direction, amount: f64) -> Vec3d {
    let step = step_vec(direction);
    Vec3d {
        x: origin.x + step.x * amount,
        y: origin.y + step.y * amount,
        z: origin.z + step.z * amount,
    }
}

fn force_horizontal(direction: Direction) -> Direction {
    if is_horizontal(direction) {
        direction
    } else {
        DEFAULT_DIRECTION
    }
}

fn is_horizontal(direction: Direction) -> bool {
    !matches!(direction, Direction::Down | Direction::Up)
}

fn axis_of(direction: Direction) -> Axis {
    match direction {
        Direction::Down | Direction::Up => Axis::Y,
        Direction::West | Direction::East => Axis::X,
        Direction::North | Direction::South => Axis::Z,
    }
}

fn axis_step(direction: Direction) -> i8 {
    match direction {
        Direction::Down | Direction::North | Direction::West => -1,
        Direction::Up | Direction::South | Direction::East => 1,
    }
}

/// `Direction.get2DDataValue` (`Direction.java:159-161`).
pub const fn horizontal_id(direction: Direction) -> i8 {
    match direction {
        Direction::South => 0,
        Direction::West => 1,
        Direction::North => 2,
        Direction::East => 3,
        _ => 0,
    }
}

/// `Direction.from2DDataValue` (`Direction.java:287`).
pub const fn from_horizontal_id(id: i8) -> Direction {
    match id.rem_euclid(4) {
        0 => Direction::South,
        1 => Direction::West,
        2 => Direction::North,
        _ => Direction::East,
    }
}

/// `Direction.get3DDataValue` matches [`Direction::id`].
pub const fn from_3d_id(id: i8) -> Direction {
    match (id % 6).abs() {
        0 => Direction::Down,
        1 => Direction::Up,
        2 => Direction::North,
        3 => Direction::South,
        4 => Direction::West,
        _ => Direction::East,
    }
}

impl EntityData {
    pub fn hanging_add_packet_data(&self) -> Option<i32> {
        pose_opt(self).map(|pose| i32::from(pose.direction.id()))
    }

    /// The block a `BlockAttachedEntity` hangs on, which its add packet sends
    /// instead of its centre (`Painting.java:196-198`, `ItemFrame.java:401-403`,
    /// `LeashFenceKnotEntity.java:155-157`).
    pub fn attached_block(&self) -> Option<BlockPos> {
        match self {
            EntityData::LeashKnot(knot) => Some(knot.attached),
            _ => pose_opt(self).map(|pose| pose.attached),
        }
    }
}

#[cfg(test)]
mod tests;
