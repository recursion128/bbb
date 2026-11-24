use super::*;
use crate::world::simulation_level::LevelCtx;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
enum InsideStage {
    Step,
    Snapshot,
    Contacts,
    Fluid,
    Deferred,
    Tail,
    Complete,
}

pub(super) struct InsideEffectsFrame {
    stage: InsideStage,
    movements: Vec<EntityMovement>,
    cursor: Option<body_contact::BodyContactCursor>,
    collector: StepBasedCollector,
    current: Option<body_contact::BodyBlockContact>,
    deferred: VecDeque<DeferredEffect>,
    was_on_fire: bool,
    was_freezing: bool,
    previous_fire_ticks: i32,
}

impl InsideEffectsFrame {
    pub(super) fn new(entity: &Entity, movements: Vec<EntityMovement>) -> Self {
        Self {
            stage: if entity.is_affected_by_blocks() {
                InsideStage::Step
            } else {
                InsideStage::Complete
            },
            movements,
            cursor: None,
            collector: StepBasedCollector::default(),
            current: None,
            deferred: VecDeque::new(),
            was_on_fire: false,
            was_freezing: false,
            previous_fire_ticks: 0,
        }
    }

    pub(super) fn run<F>(
        &mut self,
        entity: &mut Entity,
        level: &mut LevelCtx<'_, '_>,
        mut store: Option<&mut EntityStore>,
        callback: &mut F,
    ) -> bool
    where
        F: FnMut(
            &mut Entity,
            &mut LevelCtx<'_, '_>,
            Option<&mut EntityStore>,
            LivingInsideBlockCallback,
        ) -> bool,
    {
        loop {
            match self.stage {
                InsideStage::Step => {
                    self.stage = InsideStage::Snapshot;
                    if entity.on_ground()
                        && step::apply(entity, level, store.as_deref_mut(), callback).is_none()
                    {
                        return false;
                    }
                }
                InsideStage::Snapshot => {
                    let Ok(on_fire) = entity.is_on_fire_in(level) else {
                        return false;
                    };
                    self.was_on_fire = on_fire;
                    self.was_freezing = entity.ticks_frozen() > 0;
                    self.previous_fire_ticks = entity.remaining_fire_ticks();
                    if !living_is_alive(entity)
                        || (self.movements.len() == 1
                            && (self.movements[0].from == self.movements[0].to
                                || (self.movements[0].axis_dependent_original.is_none()
                                    && distance_to_sqr(
                                        self.movements[0].from,
                                        self.movements[0].to,
                                    ) < INSIDE_EPSILON.powi(2)))
                            && all_blocks_air_in_entity_box(
                                level.chunks(),
                                self.movements[0].to,
                                entity.width,
                                entity.height,
                            ))
                    {
                        self.movements.clear();
                        self.stage = InsideStage::Tail;
                        continue;
                    }
                    self.cursor = Some(body_contact::BodyContactCursor::new(
                        body_contact::BlockContactGeometry {
                            width: entity.width,
                            height: entity.height,
                        },
                        std::mem::take(&mut self.movements),
                    ));
                    self.stage = InsideStage::Contacts;
                }
                InsideStage::Contacts => {
                    let contact = if living_is_alive(entity) {
                        let Some(contact) = self
                            .cursor
                            .as_mut()
                            .expect("contact snapshot precedes traversal")
                            .next_contact(level.chunks(), entity.fall_distance)
                        else {
                            return false;
                        };
                        contact
                    } else {
                        None
                    };
                    let Some(contact) = contact else {
                        self.deferred = self.collector.take_effects().into();
                        self.stage = InsideStage::Deferred;
                        continue;
                    };
                    self.current = Some(contact);
                    // Entity.java: retain this contact's captured fluid across inline hurt.
                    self.stage = InsideStage::Fluid;
                    if contact.inside_block {
                        self.collector.advance_step(contact.step);
                        if !apply_block_inside(
                            entity,
                            level,
                            store.as_deref_mut(),
                            contact.pos,
                            contact.state,
                            contact.precise,
                            &mut self.collector,
                            callback,
                        ) {
                            return false;
                        }
                    }
                }
                InsideStage::Fluid => {
                    let contact = self
                        .current
                        .take()
                        .expect("block callback retains current contact");
                    if let Some(fluid) = contact.fluid {
                        self.collector.advance_step(contact.step);
                        self.collector.collect_fluid(fluid);
                    }
                    self.stage = InsideStage::Contacts;
                }
                InsideStage::Deferred => {
                    if !living_is_alive(entity) {
                        self.deferred.clear();
                    }
                    let Some(effect) = self.deferred.pop_front() else {
                        self.stage = InsideStage::Tail;
                        continue;
                    };
                    if !apply_deferred_effect(entity, level, store.as_deref_mut(), effect, callback)
                    {
                        return false;
                    }
                }
                InsideStage::Tail => {
                    if entity.is_in_rain(level) {
                        entity.clear_fire();
                    }
                    let extinguished = if self.was_on_fire {
                        let Ok(on_fire) = entity.is_on_fire_in(level) else {
                            return false;
                        };
                        !on_fire
                    } else {
                        false
                    };
                    if (extinguished || self.was_freezing && entity.ticks_frozen() == 0)
                        && !callback(
                            entity,
                            level,
                            store.as_deref_mut(),
                            LivingInsideBlockCallback::ExtinguishSound,
                        )
                    {
                        return false;
                    }
                    let ignited = entity.remaining_fire_ticks() > self.previous_fire_ticks;
                    let Ok(on_fire) = entity.is_on_fire_in(level) else {
                        return false;
                    };
                    if !on_fire && !ignited {
                        entity.set_remaining_fire_ticks(0);
                    }
                    self.stage = InsideStage::Complete;
                }
                InsideStage::Complete => return true,
            }
        }
    }
}

pub(crate) struct NonlivingInsidePause {
    frame: InsideEffectsFrame,
    id: i32,
    uuid: uuid::Uuid,
    hurt: crate::entity::projectile_action::PendingProjectileHurt,
    lava_sound: bool,
    hurt_succeeded: bool,
}

impl NonlivingInsidePause {
    pub(crate) fn victim_identity(&self) -> (i32, uuid::Uuid) {
        (self.id, self.uuid)
    }

    pub(crate) fn complete_hurt(&mut self, succeeded: bool) {
        self.hurt_succeeded = succeeded;
    }

    pub(crate) fn hurt(&self) -> &crate::entity::projectile_action::PendingProjectileHurt {
        &self.hurt
    }

    pub(crate) fn resume(
        mut self,
        entity: &mut Entity,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) -> Option<std::ops::ControlFlow<Self, ()>> {
        if entity.id() != self.id || entity.uuid() != self.uuid {
            return None;
        }
        if self.lava_sound && !lava::finish_lava_hurt(entity, store, self.hurt_succeeded) {
            return None;
        }
        match run_nonliving(&mut self.frame, entity, store, level).ok()? {
            Some((hurt, lava_sound)) => {
                self.hurt = hurt;
                self.lava_sound = lava_sound;
                self.hurt_succeeded = false;
                Some(std::ops::ControlFlow::Break(self))
            }
            None => Some(std::ops::ControlFlow::Continue(())),
        }
    }
}

pub(crate) fn start_nonliving(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    movements: Vec<EntityMovement>,
) -> Result<Option<NonlivingInsidePause>, ()> {
    let mut frame = InsideEffectsFrame::new(entity, movements);
    let hurt = run_nonliving(&mut frame, entity, store, level)?;
    Ok(hurt.map(|(hurt, lava_sound)| NonlivingInsidePause {
        frame,
        id: entity.id(),
        uuid: entity.uuid(),
        hurt,
        lava_sound,
        hurt_succeeded: false,
    }))
}

fn run_nonliving(
    frame: &mut InsideEffectsFrame,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Result<
    Option<(
        crate::entity::projectile_action::PendingProjectileHurt,
        bool,
    )>,
    (),
> {
    let mut pending = None;
    let complete = frame.run(
        entity,
        level,
        Some(store),
        &mut |entity, level, store, callback| {
            if matches!(entity.data(), crate::entity::EntityData::EndCrystal(_))
                || crate::entity::boat::is_boat(entity.data())
                || crate::entity::minecart::is_minecart(entity.data())
            {
                let damage = match callback {
                    LivingInsideBlockCallback::Damage {
                        damage_type,
                        amount,
                    } => Some((damage_type, amount, false)),
                    LivingInsideBlockCallback::LavaDamage { amount } => {
                        match entity.fire_immune_in(level) {
                            Ok(true) => return true,
                            Ok(false) => Some(("minecraft:lava", amount, true)),
                            Err(_) => return false,
                        }
                    }
                    _ => None,
                };
                if let Some((damage_type, amount, lava_sound)) = damage {
                    // EndCrystal.java: finish inline explosion before this caller resumes.
                    pending = Some((
                        crate::entity::projectile_action::PendingProjectileHurt {
                            target: crate::level_event::PendingHurtEntity::Stored {
                                entity_id: entity.id(),
                            },
                            damage_type,
                            amount,
                            causing: None,
                            direct: None,
                            direct_resource_id: entity.data().entity_resource_id(),
                            registry_tags: level.capture_registry_tags(),
                        },
                        lava_sound,
                    ));
                    return false;
                }
            }
            nonliving_inside_block_callback(entity, level, store, callback)
        },
    );
    if complete {
        Ok(None)
    } else {
        pending.map(Some).ok_or(())
    }
}

fn all_blocks_air_in_entity_box(
    chunks: &crate::chunk::ChunkStore,
    position: Vec3d,
    width: f32,
    height: f32,
) -> bool {
    let target_box = Aabb::of_entity(position, width, height).inflate(
        -INSIDE_EPSILON,
        -INSIDE_EPSILON,
        -INSIDE_EPSILON,
    );
    let min_x = target_box.min_x.floor() as i32;
    let max_x = target_box.max_x.floor() as i32;
    let min_y = target_box.min_y.floor() as i32;
    let max_y = target_box.max_y.floor() as i32;
    let min_z = target_box.min_z.floor() as i32;
    let max_z = target_box.max_z.floor() as i32;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let state = crate::world::stored_block_state(chunks, BlockPos { x, y, z });
                let Some(block_type_id) = vanilla_block_state_block_type_id(state) else {
                    return false;
                };
                if !matches!(
                    block_type_id,
                    block_types::VANILLA_BLOCK_AIR_ID
                        | block_types::VANILLA_BLOCK_CAVE_AIR_ID
                        | block_types::VANILLA_BLOCK_VOID_AIR_ID
                ) {
                    return false;
                }
            }
        }
    }
    true
}
