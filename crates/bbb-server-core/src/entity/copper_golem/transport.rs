//! Copper chest transport using the live body, containers and navigation.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_TRAPPED_CHEST_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::geometry::{clip::clip_with, Aabb, Vec3Ops};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{set_animation_state, AnimationState};
use crate::entity::ai::mob::navigation::MobPathingSnapshot;
use crate::entity::store::PendingEntitySound;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::container_access::BlockContainer;
use crate::world::simulation_level::LevelCtx;

#[derive(Clone, Default)]
pub(crate) struct TransportRuntime {
    target: Option<BlockPos>,
    opened: Option<BlockContainer>,
    interaction: Option<(i32, AnimationState)>,
    cooldown_until: i32,
    search_after: i32,
    visited: Vec<BlockPos>,
    unreachable: Vec<BlockPos>,
    forget_at: i32,
}

impl TransportRuntime {
    pub(crate) fn opened_chest(&self) -> Option<BlockPos> {
        self.opened.and(self.target)
    }

    /// Whether `TRANSPORT_ITEMS_COOLDOWN_TICKS` is counting down.
    pub(crate) fn resting(&self, now: i32) -> bool {
        self.target.is_none() && now < self.cooldown_until
    }

    fn close(
        &mut self,
        entity: &mut Entity,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) {
        if let Some(container) = self.opened.take() {
            change_openers(entity, store, level, container, false);
        }
        self.interaction = None;
        set_animation_state(entity, AnimationState::Idle);
    }

    fn stop(
        &mut self,
        entity: &mut Entity,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
        stop_path: bool,
    ) {
        if self.target.take().is_some() && stop_path {
            stop_navigation(entity, store, level);
        }
        self.close(entity, store, level);
    }

    fn cooldown(
        &mut self,
        entity: &mut Entity,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) {
        self.stop(entity, store, level, true);
        self.visited.clear();
        self.unreachable.clear();
        self.cooldown_until = entity.tick_count() + 140;
    }

    // TransportItemsBetweenContainers.java:42-52,113-199,278-323,397-451.
    fn tick(
        &mut self,
        entity: &mut Entity,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) -> Result<(), ()> {
        let now = entity.tick_count();
        let panicking = store
            .mob_ai(entity.id())
            .is_some_and(|ai| ai.is_panicking());
        if !entity.is_alive_with_health()
            || matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
            || crate::entity::leash::is_leashed(entity)
            || panicking
        {
            self.stop(entity, store, level, !panicking);
            self.search_after = now + 20;
            return Ok(());
        }
        if self.cooldown_until == 0 {
            // CopperGolem.java:89-97.
            self.cooldown_until = now + store.random().next_int_between_inclusive(60, 100);
        }
        if now < self.cooldown_until {
            return Ok(());
        }
        if now >= self.forget_at {
            self.visited.clear();
            self.unreachable.clear();
            self.forget_at = now + 6000;
        }
        if let Some(pos) = self.target {
            let valid = wanted_block(entity, level, pos)
                && target_container(entity, store, level, pos).is_some_and(|container| {
                    !level.container_is_locked(container)
                        && self
                            .opened
                            .is_none_or(|opened| opened.positions() == container.positions())
                });
            if !valid {
                self.stop(entity, store, level, true);
                self.search_after = now + 20;
            }
        }
        if self.target.is_none() {
            if now < self.search_after {
                return Ok(());
            }
            self.search_after = now + 20;
            let Some(pos) = self.find_target(entity, store, level) else {
                self.cooldown(entity, store, level);
                return Ok(());
            };
            let container = target_container(entity, store, level, pos).ok_or(())?;
            self.target = Some(pos);
            self.forget_at = now + 6000;
            self.visited.extend(container.positions());
            if self.visited.len() > 10 {
                self.cooldown(entity, store, level);
                return Ok(());
            }
        }
        let pos = self.target.expect("selected transport target");
        let container = target_container(entity, store, level, pos).ok_or(())?;
        if let Some((started, animation)) = self.interaction {
            if !within_range(entity.bounding_box(), entity, level, pos, 2.0) {
                self.close(entity, store, level);
                return Ok(());
            }
            stop_in_place(entity, store, level);
            look_at(entity, store, pos);
            let elapsed = now - started;
            if elapsed == 9 && !entity.is_silent() {
                store.play_entity_sound(PendingEntitySound {
                    position: entity.position(),
                    resource_id: interaction_sound(animation).into(),
                    source: SoundSource::Neutral,
                    volume: 1.0,
                    pitch: 1.0,
                });
            }
            if elapsed >= 60 {
                self.close(entity, store, level);
                let current = target_container(entity, store, level, pos).filter(|container| {
                    wanted_block(entity, level, pos) && !level.container_is_locked(*container)
                });
                let matched = match current {
                    Some(container) => transfer(entity, store, level, container)?,
                    None => false,
                };
                self.stop(entity, store, level, true);
                if matched && (hand(entity).is_none() || wanted_source(level, pos)) {
                    self.visited.clear();
                    self.unreachable.clear();
                }
                self.search_after = now + 1;
            }
            return Ok(());
        }
        if within_range(entity.bounding_box(), entity, level, pos, 3.0)
            && container.positions().iter().any(|&at| {
                level
                    .chunks()
                    .block_entities()
                    .get(at)
                    .and_then(crate::block_entity::BlockEntity::chest)
                    .is_some_and(|chest| chest.open_count() > 0)
            })
        {
            stop_in_place(entity, store, level);
            look_at(entity, store, pos);
            return Ok(());
        }
        let finished_path = navigation_owner(entity, store, level)
            .and_then(|id| store.ground_path(id))
            .is_some_and(|path| path.is_done());
        if within_range(
            entity.bounding_box(),
            entity,
            level,
            pos,
            if finished_path { 1.0 } else { 0.5 },
        ) && sees_chest(entity, level, pos, entity.bounding_box().center())
        {
            stop_in_place(entity, store, level);
            look_at(entity, store, pos);
            let animation = if hand(entity).is_none() {
                if first_item(level, container)?.is_some() {
                    AnimationState::GettingItem
                } else {
                    AnimationState::GettingNoItem
                }
            } else if accepts_hand(entity, level, container)? {
                AnimationState::DroppingItem
            } else {
                AnimationState::DroppingNoItem
            };
            self.opened = Some(container);
            self.interaction = Some((now, animation));
            change_openers(entity, store, level, container, true);
            set_animation_state(entity, animation);
            return Ok(());
        }
        let path_missing = navigation_owner(entity, store, level)
            .and_then(|id| store.ground_path(id))
            .is_none();
        if (path_missing || finished_path) && !navigate(entity, store, level, pos) {
            self.visited
                .retain(|at| !container.positions().contains(at));
            self.unreachable.extend(container.positions());
            self.stop(entity, store, level, true);
            if self.unreachable.len() > 50 {
                self.cooldown(entity, store, level);
            }
        }
        Ok(())
    }

    fn find_target(
        &self,
        entity: &Entity,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) -> Option<BlockPos> {
        let origin = entity.block_position();
        let riding = store.vehicle(entity.id()).is_some();
        let level = &level.with_entity_context(crate::entity::level::EntityBlockContext::new(
            Some(store),
            Some(entity),
        ));
        let (horizontal, vertical) = if riding { (1.0, 1.0) } else { (32.0, 8.0) };
        let bounds = Aabb {
            min_x: f64::from(origin.x) - horizontal,
            min_y: f64::from(origin.y) - vertical,
            min_z: f64::from(origin.z) - horizontal,
            max_x: f64::from(origin.x + 1) + horizontal,
            max_y: f64::from(origin.y + 1) + vertical,
            max_z: f64::from(origin.z + 1) + horizontal,
        };
        level
            .chest_positions_in(bounds)
            .filter(|&pos| wanted_block(entity, level, pos))
            .filter(|&pos| {
                level.block_container(pos, false).is_some_and(|container| {
                    !level.container_is_locked(container)
                        && container
                            .positions()
                            .iter()
                            .all(|at| !self.visited.contains(at) && !self.unreachable.contains(at))
                })
            })
            .min_by(|&left, &right| {
                entity
                    .position()
                    .distance_to_sqr(center(left))
                    .total_cmp(&entity.position().distance_to_sqr(center(right)))
            })
    }
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let Some(mut state) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.take_copper_golem_transport())
    else {
        return true;
    };
    let result = state.tick(entity, store, level);
    if result.is_err() {
        state.stop(entity, store, level, true);
    }
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.return_copper_golem_transport(state);
    }
    result.is_ok()
}

fn target_container(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<BlockContainer> {
    level
        .with_entity_context(crate::entity::level::EntityBlockContext::new(
            Some(store),
            Some(entity),
        ))
        .block_container(pos, false)
}

fn hand(entity: &Entity) -> Option<&RuntimeItemStack> {
    entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .filter(|stack| !stack.is_empty())
}

fn wanted_source(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.block_state_in_tag(level.block_state(pos), "minecraft:copper_chests")
}

fn wanted_block(entity: &Entity, level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    if hand(entity).is_none() {
        wanted_source(level, pos)
    } else {
        matches!(
            vanilla_block_state_block_type_id(level.block_state(pos)),
            Some(VANILLA_BLOCK_CHEST_ID | VANILLA_BLOCK_TRAPPED_CHEST_ID)
        )
    }
}

// TransportItemsBetweenContainers.java:494-572.
fn first_item(
    level: &mut LevelCtx<'_, '_>,
    container: BlockContainer,
) -> Result<Option<(usize, RuntimeItemStack)>, ()> {
    for slot in 0..container.size() {
        if let Some(stack) = level
            .get_container_item(container, slot)
            .map_err(|_| ())?
            .filter(|stack| !stack.is_empty())
        {
            return Ok(Some((slot, stack)));
        }
    }
    Ok(None)
}

fn accepts_hand(
    entity: &Entity,
    level: &mut LevelCtx<'_, '_>,
    container: BlockContainer,
) -> Result<bool, ()> {
    let held = hand(entity).and_then(RuntimeItemStack::item_id);
    let mut empty = true;
    for slot in 0..container.size() {
        if let Some(stack) = level
            .get_container_item(container, slot)
            .map_err(|_| ())?
            .filter(|stack| !stack.is_empty())
        {
            empty = false;
            if stack.item_id() == held {
                return Ok(true);
            }
        }
    }
    Ok(empty)
}

fn transfer(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    container: BlockContainer,
) -> Result<bool, ()> {
    let mut held = hand(entity).cloned().unwrap_or_default();
    if held.is_empty() {
        let Some((slot, mut stack)) = first_item(level, container)? else {
            return Ok(false);
        };
        held = stack.split(16);
        let (pos, local) = container.slot(slot).ok_or(())?;
        level
            .set_randomizable_container_item(pos, local, stack)
            .map_err(|_| ())?;
        set_hand(entity, store, level, held)?;
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            if let Some(mob) = data.mob_retained_mut() {
                mob.set_guaranteed_drop(EquipmentSlot::MainHand);
            }
        }
        return Ok(true);
    }
    if !accepts_hand(entity, level, container)? {
        return Ok(false);
    }
    let before = held.count();
    for slot in 0..container.size() {
        let mut destination = level
            .get_container_item(container, slot)
            .map_err(|_| ())?
            .unwrap_or_default();
        if destination.is_empty() {
            destination = held.split(held.count());
        } else if destination
            .same_item_same_effective_components(&held, level.item_stack_resolver())
            .map_err(|_| ())?
        {
            let available = destination.max_stack_size().map_err(|_| ())? - destination.count();
            let moved = available.max(0).min(held.count());
            if moved == 0 {
                continue;
            }
            destination.grow(moved);
            held.shrink(moved);
        } else {
            continue;
        }
        let (pos, local) = container.slot(slot).ok_or(())?;
        level
            .set_randomizable_container_item(pos, local, destination)
            .map_err(|_| ())?;
        set_hand(entity, store, level, held.clone())?;
        if held.is_empty() {
            break;
        }
    }
    Ok(held.count() != before)
}

fn set_hand(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    stack: RuntimeItemStack,
) -> Result<(), ()> {
    crate::entity::item_slots::set_living_item_in_tick(
        entity,
        store,
        level,
        EquipmentSlot::MainHand,
        (!stack.is_empty()).then_some(stack),
    )
    .map_err(|_| ())
}

fn change_openers(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    container: BlockContainer,
    opening: bool,
) {
    let mut level = level.with_entity_context(crate::entity::level::EntityBlockContext::new(
        Some(store),
        Some(entity),
    ));
    for &pos in container.positions() {
        level.change_chest_openers(pos, entity.id(), 3.0, opening);
    }
}

fn center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

fn navigation_owner(entity: &Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) -> Option<i32> {
    store.mob_control_owner(entity, level)
}

fn stop_navigation(entity: &Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    if let Some(owner) = navigation_owner(entity, store, level) {
        store.stop_mob_navigation(owner);
    }
}

fn stop_in_place(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    stop_navigation(entity, store, level);
    if let Some(living) = entity.living_mut() {
        living.xxa = 0.0;
        living.yya = 0.0;
        living.zza = 0.0;
        living.set_speed(0.0);
    }
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: entity.delta_movement().y,
        z: 0.0,
    });
}

fn look_at(entity: &Entity, store: &mut EntityStore, pos: BlockPos) {
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.set_transport_look(center(pos));
    }
}

// TransportItemsBetweenContainers.java:352-358,455-482.
fn navigate(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> bool {
    let Some(owner) = navigation_owner(entity, store, level) else {
        return false;
    };
    let body = if owner == entity.id() {
        entity
    } else {
        let Some(body) = store.get(owner) else {
            return false;
        };
        body
    };
    let Some(malus) = store.effective_pathfinding_malus(body, level) else {
        return false;
    };
    let Some(mob) = MobPathingSnapshot::from_entity(
        body,
        false,
        store.vehicle(owner).is_some(),
        store.difficulty(),
        malus,
    ) else {
        return false;
    };
    if !store
        .mob_ai_mut(owner)
        .is_some_and(|runtime| runtime.navigate_to(level, mob, pos, 1, 1.0))
    {
        return false;
    }
    store.ground_path(owner).is_some_and(|path| {
        path.nodes().last().is_some_and(|end| {
            let bottom = Vec3d {
                x: f64::from(end.pos.x) + 0.5,
                y: f64::from(end.pos.y),
                z: f64::from(end.pos.z) + 0.5,
            };
            let bounds = entity
                .bounding_box()
                .moved(bottom.plus(entity.position().scale(-1.0)));
            within_range(bounds, entity, level, pos, 1.0)
                && sees_chest(entity, level, pos, bounds.center())
        })
    })
}

fn within_range(
    bounds: Aabb,
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    range: f64,
) -> bool {
    level
        .block_collision_boxes_for_entity(entity, pos)
        .is_some_and(|shapes| {
            shapes.into_iter().any(|shape| {
                shape
                    .inflate(range, 0.5, range)
                    .moved(Vec3d {
                        x: f64::from(pos.x),
                        y: f64::from(pos.y),
                        z: f64::from(pos.z),
                    })
                    .intersects(bounds)
            })
        })
}

fn sees_chest(entity: &Entity, level: &LevelCtx<'_, '_>, pos: BlockPos, from: Vec3d) -> bool {
    [
        (0.5, 0.0, 0.0),
        (-0.5, 0.0, 0.0),
        (0.0, 0.5, 0.0),
        (0.0, -0.5, 0.0),
        (0.0, 0.0, 0.5),
        (0.0, 0.0, -0.5),
    ]
    .into_iter()
    .any(|(x, y, z)| {
        clip_with(from, center(pos).plus(Vec3d { x, y, z }), |at| {
            level.block_state_checked(at)?;
            Some((level.block_collision_boxes_for_entity(entity, at)?, None))
        })
        .flatten()
        .is_some_and(|hit| hit.pos == pos)
    })
}

// CopperGolemAi.java:100-127; SoundEvents.java:425-428.
fn interaction_sound(animation: AnimationState) -> &'static str {
    match animation {
        AnimationState::GettingItem => "minecraft:entity.copper_golem.no_item_get",
        AnimationState::GettingNoItem => "minecraft:entity.copper_golem.no_item_no_get",
        AnimationState::DroppingItem => "minecraft:entity.copper_golem.item_drop",
        AnimationState::DroppingNoItem => "minecraft:entity.copper_golem.item_no_drop",
        AnimationState::Idle => unreachable!("an interaction has an animation"),
    }
}
