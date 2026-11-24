//! Cat and ocelot movement poses and tame cat rest goals.

use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_property,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_FURNACE_ID};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_CAT_IS_LYING_DATA_ID, VANILLA_CAT_RELAX_STATE_ONE_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING, VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_OCELOT_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::block_queries::bool_property;
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{Aabb, Vec3Ops};

pub(super) fn update_movement_pose(
    entity: &mut Entity,
    wanted_speed: Option<f64>,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<()> {
    if !matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_CAT_ID | VANILLA_ENTITY_TYPE_OCELOT_ID
    ) {
        return Some(());
    }
    let pose = if wanted_speed == Some(0.6) {
        VANILLA_ENTITY_POSE_CROUCHING_ID
    } else {
        VANILLA_ENTITY_POSE_STANDING_ID
    };
    let sprinting = wanted_speed == Some(1.33);
    entity.set_pose_id_with_level(pose, level, Some(store))?;
    entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING, sprinting);
    entity
        .living_mut()?
        .attributes_mut()
        .set_sprinting_modifier(sprinting);
    Some(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CatRestKind {
    LieOnBed,
    SitOnBlock,
}

pub(super) fn can_move(targets: &TargetView<'_, '_, '_>) -> bool {
    matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
        if data.tamable_retained().is_some_and(|tame| tame.is_tame() && !tame.ordered_to_sit()))
}

// `CatLieOnBedGoal.isValidTarget`.
pub(super) fn valid_target(level: &LevelCtx<'_, '_>, pos: BlockPos, kind: CatRestKind) -> bool {
    let above = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    if !level.has_ticking_column_at(pos) || !block_state_is_air(level.block_state(above)) {
        return false;
    }
    let state = level.block_state(pos);
    let bed = level.block_state_in_tag(state, "minecraft:beds");
    match kind {
        CatRestKind::LieOnBed => bed,
        // `CatSitOnBlockGoal.isValidTarget`.
        CatRestKind::SitOnBlock => match vanilla_block_state_block_type_id(state) {
            Some(VANILLA_BLOCK_CHEST_ID) => level.chest_open_count(pos) < 1,
            Some(VANILLA_BLOCK_FURNACE_ID) if bool_property(state, "lit") == Some(true) => true,
            _ => bed && vanilla_block_state_property(state, "part") != Some("head"),
        },
    }
}

pub(super) fn apply_lying(entity: &mut Entity, lying: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let changed = data.set_cat_lying(lying);
    let cleared_sitting = data.tamable_retained_mut().is_some_and(|tame| {
        if tame.ordered_to_sit() {
            return false;
        }
        let sitting = tame.is_in_sitting_pose();
        tame.set_in_sitting_pose(false);
        sitting
    });
    if changed {
        entity.mark_synched_dirty(VANILLA_CAT_IS_LYING_DATA_ID);
    }
    if cleared_sitting {
        entity.mark_synched_dirty(VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID);
    }
}

pub(super) fn apply_sitting(entity: &mut Entity, sitting: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(tame) = data.tamable_retained_mut() else {
        return;
    };
    if tame.ordered_to_sit() || tame.is_in_sitting_pose() == sitting {
        return;
    }
    tame.set_in_sitting_pose(sitting);
    entity.mark_synched_dirty(VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID);
}

#[derive(Clone, Default)]
pub(super) struct CatRelaxOnOwnerGoal {
    target: Option<BlockPos>,
    on_bed_ticks: i32,
    next_path: i64,
}

impl CatRelaxOnOwnerGoal {
    fn owner<'a>(
        &self,
        targets: &'a TargetView<'_, '_, '_>,
    ) -> Option<&'a crate::world::PlayerState> {
        let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
            return None;
        };
        let uuid = data.tamable_retained()?.owner()?;
        targets.store().player_by_uuid(uuid)
    }

    fn occupied(&self, targets: &TargetView<'_, '_, '_>) -> bool {
        let Some(pos) = self.target else {
            return false;
        };
        let bounds = Aabb {
            min_x: f64::from(pos.x) - 2.0,
            min_y: f64::from(pos.y) - 2.0,
            min_z: f64::from(pos.z) - 2.0,
            max_x: f64::from(pos.x) + 3.0,
            max_y: f64::from(pos.y) + 3.0,
            max_z: f64::from(pos.z) + 3.0,
        };
        targets
            .store()
            .ids_in_box_sections(bounds)
            .into_iter()
            .any(|id| {
                id != targets.body_entity().id()
                    && targets.store().get(id).is_some_and(|other| {
                        other.entity_type_id() == VANILLA_ENTITY_TYPE_CAT_ID
                            && other.bounding_box().intersects(bounds)
                            && matches!(other.data(), EntityData::GeneralLiving(data)
                        if data.cat_lying() || data.cat_relaxing())
                    })
            })
    }

    fn navigate(&self, context: &mut MobAiContext) {
        context.species.cat_sitting = Some(false);
        if let Some(pos) = self.target {
            context.navigate_to_position(
                Vec3d {
                    x: f64::from(pos.x),
                    y: f64::from(pos.y),
                    z: f64::from(pos.z),
                },
                1.1,
            );
        }
    }
}

// `Cat.CatRelaxOnOwnerGoal`.
impl Goal for CatRelaxOnOwnerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !can_move(targets) {
            return false;
        }
        let Some(owner) = self
            .owner(targets)
            .filter(|owner| owner.sleeping_pos().is_some())
        else {
            return false;
        };
        if context
            .body
            .position
            .distance_to_sqr(owner.body().position())
            > 100.0
        {
            return false;
        }
        let pos = owner.body().block_position();
        let state = targets.level().block_state(pos);
        if !targets.level().block_state_in_tag(state, "minecraft:beds") {
            return false;
        }
        self.target = Some(
            crate::block_behavior::direction_property(state, "facing")
                .map_or(pos, |direction| relative(pos, opposite(direction))),
        );
        !self.occupied(targets)
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        can_move(targets)
            && self.target.is_some()
            && self
                .owner(targets)
                .is_some_and(|owner| owner.sleeping_pos().is_some())
            && !self.occupied(targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.navigate(context);
        self.next_path = targets.game_time() + 40;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.species.cat_sitting = Some(false);
        if targets.game_time() >= self.next_path {
            self.navigate(context);
            self.next_path = targets.game_time() + 40;
        }
        let Some(owner) = self.owner(targets) else {
            return;
        };
        let body = owner.body();
        if context.body.position.distance_to_sqr(body.position()) < 2.5 {
            self.on_bed_ticks += 1;
            if self.on_bed_ticks > reduced_tick_delay(16) {
                context.species.cat_lying = Some(true);
                context.species.cat_relaxing = Some(false);
            } else {
                let pos = body.position();
                context.set_look_at_with_limits(
                    pos.x,
                    pos.y + f64::from(body.eye_height()),
                    pos.z,
                    45.0,
                    45.0,
                );
                context.species.cat_relaxing = Some(true);
            }
        } else {
            context.species.cat_lying = Some(false);
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self
            .owner(targets)
            .is_some_and(|owner| owner.sleep_timer() >= 100)
        {
            if let Some((entity, store, level)) = targets.world_mut() {
                if crate::entity::gift_loot::cat_morning_gift(entity, store, level).is_none() {
                    targets.abort();
                }
            }
        }
        context.species.cat_lying = Some(false);
        context.species.cat_relaxing = Some(false);
        context.stop_navigation();
        self.on_bed_ticks = 0;
        self.next_path = 0;
        self.target = None;
    }
}

pub(super) fn apply_relaxing(entity: &mut Entity, relaxing: bool) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if data.set_cat_relaxing(relaxing) {
            entity.mark_synched_dirty(VANILLA_CAT_RELAX_STATE_ONE_DATA_ID);
        }
    }
}

// `Cat.tick`: the untamed beg sound and `handleLieDown`.
pub(crate) fn tick(entity: &Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_CAT_ID || entity.is_silent() {
        return;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if entity.tick_count() % 100 == 0
        && !data.is_tame()
        && store
            .mob_ai(entity.id())
            .is_some_and(super::MobAiRuntime::tempt_goal_running)
    {
        if let Some(sound) = crate::entity::cat_sounds::resolve(
            store,
            entity,
            crate::entity::cat_sounds::CatSound::BegForFood,
        ) {
            crate::entity::sound_variants::play(
                store,
                sound,
                entity.position(),
                crate::entity::living::living_sound_source(entity),
                1.0,
                1.0,
            );
        }
    }
    if entity.tick_count() % 5 != 0 || (!data.cat_lying() && !data.cat_relaxing()) {
        return;
    }
    let Some(sound) = crate::entity::cat_sounds::resolve(
        store,
        entity,
        crate::entity::cat_sounds::CatSound::Purr,
    ) else {
        return;
    };
    let random = level.level_random();
    crate::entity::sound_variants::play(
        store,
        sound,
        entity.position(),
        crate::entity::living::living_sound_source(entity),
        0.6 + 0.4 * (random.next_float() - random.next_float()),
        1.0,
    );
}
