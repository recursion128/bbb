//! Partner selection for goal-driven animal breeding.

use bbb_server_types::geometry::Vec3Ops;

use crate::entity::animal_breeding::is_in_love;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

use super::spec::{is_instance_of, runtime_class};
use super::target::{conditions::TargetingConditions, TargetView};
use super::{reduced_tick_delay, Goal, MobAiContext};

/// `BreedGoal.PARTNER_TARGETING` (`BreedGoal.java:11`): non-combat, range 8.0,
/// line of sight ignored.
const PARTNER_RANGE: f64 = 8.0;

/// `this.animal.distanceToSqr(this.partner) < 9.0` (`BreedGoal.java:57`).
const MATING_DISTANCE_SQR: f64 = 9.0;

/// `this.loveTime < 60` (`BreedGoal.java:43`).
const LOVE_TIME_LIMIT: i32 = 60;

/// Which `canMate` the body runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MateGate {
    Base,
    Cat,
    Wolf,
    /// `Horse.canMate` / `Donkey.canMate` (`Horse`).
    Equine,
    /// `Llama.canMate`.
    Llama,
    /// `Camel.canMate`.
    Camel,
    /// `Sniffer.canMate`.
    Sniffer,
}

impl MateGate {
    fn of(entity_type_id: i32) -> Self {
        match entity_type_id {
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID => Self::Camel,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID => Self::Sniffer,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID => Self::Cat,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID => Self::Wolf,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID => Self::Equine,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => Self::Llama,
            _ => Self::Base,
        }
    }

    /// `BreedGoal`'s partner class: `AbstractHorse.class` for the horse family
    /// the body's own class otherwise.
    fn partner_class(self, entity_type_id: i32) -> Option<&'static str> {
        match self {
            Self::Equine => Some("AbstractHorse"),
            _ => runtime_class(entity_type_id),
        }
    }

    /// `canMate`: the override's terms, then `Animal.canMate`'s same-class
    /// rule for the bodies that keep it.
    fn admits(self, class: &str, candidate: &Entity, store: &EntityStore) -> bool {
        let type_id = candidate.entity_type_id();
        match self {
            Self::Base => runtime_class(type_id) == Some(class),
            Self::Cat => runtime_class(type_id) == Some(class) && tame_state(candidate).0,
            Self::Wolf => {
                let (tame, sitting) = tame_state(candidate);
                runtime_class(type_id) == Some(class) && tame && !sitting
            }
            Self::Equine => {
                matches!(
                    type_id,
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID
                        | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID
                ) && can_parent(candidate, store)
            }
            Self::Camel => {
                type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID
                    && can_parent(candidate, store)
            }
            Self::Llama => is_instance_of(type_id, "Llama") && can_parent(candidate, store),
            Self::Sniffer => runtime_class(type_id) == Some(class) && can_parent(candidate, store),
        }
    }
}

fn tame_state(entity: &Entity) -> (bool, bool) {
    match entity.data() {
        EntityData::GeneralLiving(data) => match data.tamable_retained() {
            Some(tamable) => (tamable.is_tame(), tamable.is_in_sitting_pose()),
            None => (false, false),
        },
        _ => (false, false),
    }
}

/// `AbstractHorse.canParent`, or a sniffer's
/// mating state; false for any other body.
fn can_parent(entity: &Entity, store: &EntityStore) -> bool {
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID {
        return crate::entity::sniffer::can_mate_in_state(entity);
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    let tamed = entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID
        || data
            .abstract_horse_retained()
            .is_some_and(|horse| horse.is_tame());
    let baby = data
        .ageable_retained()
        .is_some_and(|ageable| ageable.is_baby());
    tamed
        && !baby
        && store.passengers(entity.id()).is_empty()
        && store.vehicle(entity.id()).is_none()
        && entity
            .living()
            .is_some_and(|living| living.health() >= living.max_health())
        && is_in_love(entity)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BreedGoal {
    speed_modifier: f64,
    partner_id: Option<i32>,
    love_time: i32,
    next_panda_check: i32,
    unhappy_cooldown: i32,
}

impl BreedGoal {
    pub(super) const fn new(speed_modifier: f64) -> Self {
        Self {
            speed_modifier,
            partner_id: None,
            love_time: 0,
            next_panda_check: 0,
            unhappy_cooldown: 0,
        }
    }

    fn partner<'a>(&self, targets: &'a TargetView<'_, '_, '_>) -> Option<&'a Entity> {
        targets.store().get(self.partner_id?)
    }
}

impl Goal for BreedGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let source = targets.body_entity();
        // `TurtleBreedGoal.canUse` gates only the actor's egg.
        if !is_in_love(source)
            || matches!(source.data(), EntityData::GeneralLiving(data)
                if data.turtle_retained().is_some_and(|turtle| turtle.has_egg()))
        {
            return false;
        }
        let mate_gate = MateGate::of(source.entity_type_id());
        let ready = match mate_gate {
            MateGate::Base => true,
            MateGate::Cat | MateGate::Wolf => tame_state(source).0,
            _ => can_parent(source, targets.store()),
        };
        if !ready {
            return false;
        }
        let Some(class) = mate_gate.partner_class(source.entity_type_id()) else {
            return false;
        };
        let search = source
            .bounding_box()
            .inflate(PARTNER_RANGE, PARTNER_RANGE, PARTNER_RANGE);
        let conditions = TargetingConditions::for_non_combat()
            .range(PARTNER_RANGE)
            .ignore_line_of_sight();
        let mut closest: Option<(f64, i32)> = None;
        targets.store().for_each_box_section_id(search, |id| {
            let Some(candidate) = targets.store().get(id) else {
                return;
            };
            if id == source.id()
                || !is_instance_of(candidate.entity_type_id(), class)
                || !candidate.bounding_box().intersects(search)
                || !is_in_love(candidate)
                || targets
                    .store()
                    .mob_ai(id)
                    .is_some_and(|runtime| runtime.is_panicking())
                || !mate_gate.admits(class, candidate, targets.store())
            {
                return;
            }
            let target = targets.facts(id).expect("stored breeding candidate");
            if !conditions.test(targets, &target, |_| true) {
                return;
            }
            let distance = context.body.position.distance_to_sqr(candidate.position());
            if closest.is_none_or(|(best, _)| distance < best) {
                closest = Some((distance, id));
            }
        });
        self.partner_id = closest.map(|(_, id)| id);
        if self.partner_id.is_none() {
            return false;
        }
        let Some(unhappy) = crate::entity::panda::unhappy_counter(targets.body_entity()) else {
            return true;
        };
        if unhappy != 0 || context.tick_count < self.next_panda_check {
            return false;
        }
        self.next_panda_check = context.tick_count.saturating_add(20);
        // Panda: BAMBOO within seven horizontal blocks and y offsets 0..2.
        let origin = targets.body_entity().block_position();
        for y in 0..3 {
            for x in -7..=7 {
                for z in -7..=7 {
                    let pos = bbb_protocol::packets::BlockPos {
                        x: origin.x + x,
                        y: origin.y + y,
                        z: origin.z + z,
                    };
                    if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(
                        targets.level().block_state(pos),
                    ) == Some(bbb_protocol::block_types::VANILLA_BLOCK_BAMBOO_ID)
                    {
                        return true;
                    }
                }
            }
        }
        if context.tick_count >= self.unhappy_cooldown {
            self.unhappy_cooldown = context.tick_count.saturating_add(600);
            if let Some((entity, _, _)) = targets.world_mut() {
                crate::entity::panda::set_breeding_unhappy(entity);
            }
            let conditions =
                super::target::conditions::TargetingConditions::for_non_combat().range(8.0);
            if let Some(player) = targets
                .local_players()
                .filter(|player| conditions.test(targets, player, |_| true))
                .min_by(|a, b| {
                    context
                        .body
                        .position
                        .distance_to_sqr(a.position())
                        .total_cmp(&context.body.position.distance_to_sqr(b.position()))
                })
            {
                let position = player.position();
                context.set_look_at(position.x, player.eye_y(), position.z);
            }
        }
        false
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if targets.body().entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID
        {
            if let Some(partner_id) = self.partner_id {
                if let Some((entity, store, _)) = targets.world_mut() {
                    crate::entity::fox::breeding::clear_parent_states(entity, store, partner_id);
                }
            }
        }
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        // HoglinAi: hurt clears the breeding target.
        if targets.body().entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID
            && !is_in_love(targets.body_entity())
        {
            return false;
        }
        let Some(partner) = self.partner(targets) else {
            return false;
        };
        partner.is_alive_with_health()
            && is_in_love(partner)
            && self.love_time < LOVE_TIME_LIMIT
            && !targets
                .store()
                .mob_ai(partner.id())
                .is_some_and(|runtime| runtime.is_panicking())
    }

    fn stop(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.partner_id = None;
        self.love_time = 0;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(partner) = self.partner(targets) else {
            return;
        };
        let position = partner.position();
        context.set_look_at(
            position.x,
            position.y + f64::from(partner.eye_height()),
            position.z,
        );
        context.navigate_to(position, self.speed_modifier);
        self.love_time += 1;
        let delay = reduced_tick_delay(LOVE_TIME_LIMIT);
        if self.love_time >= delay
            && context.body.position.distance_to_sqr(position) < MATING_DISTANCE_SQR
            && !context.post_step.iter().any(|action| {
                matches!(
                    action,
                    super::MobPostStepAction::SpawnChildFromBreeding { .. }
                )
            })
        {
            context
                .post_step
                .push(super::MobPostStepAction::SpawnChildFromBreeding {
                    partner_id: partner.id(),
                });
        }
    }
}

#[cfg(test)]
mod tests;
