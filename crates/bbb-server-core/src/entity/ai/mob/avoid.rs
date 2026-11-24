//! `AvoidEntityGoal` and the subclasses that only gate it.

use bbb_protocol::packets::Vec3d;

use crate::entity::horse_phenotype::HorsePhenotypeState;
use crate::entity::EntityData;

use super::random_pos::{default_random_pos_away, RandomPosMob};
use super::target::conditions::TargetingConditions;
use super::target::selector::is_instance_of;
use super::target::{Target, TargetRef, TargetView};
use super::{Goal, MobAiContext};

/// `AvoidEntityGoal(mob, avoidClass, maxDist, walkSpeedModifier,
/// sprintSpeedModifier, ...)` with the gate its subclass or predicate adds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AvoidSpec {
    pub(crate) class: &'static str,
    pub(crate) max_dist: f32,
    pub(crate) walk_speed: f64,
    pub(crate) sprint_speed: f64,
    pub(crate) gate: AvoidGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AvoidGate {
    None,
    PandaWorried,
    /// `RabbitAvoidEntityGoal.canUse`.
    NotKillerBunny,
    /// `CatAvoidEntityGoal`, only registered untamed.
    Untamed,
    /// `OcelotAvoidEntityGoal`, only registered
    /// while not trusting.
    Untrusting,
    /// `WolfAvoidEntityGoal`.
    WolfFromLlama,
    /// `Fox.AVOID_PLAYERS` and the fox's trust.
    FoxFromPlayer,
    /// An untamed wolf.
    FoxFromWolf,
    FoxNotDefending,
    /// `Spider`: only an idle armadillo repels spiders.
    SpiderFromArmadillo,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AvoidEntityGoal {
    spec: AvoidSpec,
    to_avoid: Option<TargetRef>,
}

impl AvoidEntityGoal {
    pub(super) const fn new(spec: AvoidSpec) -> Self {
        Self {
            spec,
            to_avoid: None,
        }
    }

    fn body_allows(&self, targets: &TargetView<'_, '_, '_>) -> bool {
        let body = targets.body_entity();
        let EntityData::GeneralLiving(data) = body.data() else {
            return false;
        };
        match self.spec.gate {
            AvoidGate::None | AvoidGate::WolfFromLlama | AvoidGate::SpiderFromArmadillo => true,
            AvoidGate::NotKillerBunny => !crate::entity::rabbit::is_evil(body),
            AvoidGate::PandaWorried => data.panda_retained().is_some_and(|panda| {
                panda.is_worried() && panda.can_perform_action(targets.level().is_thundering())
            }),
            AvoidGate::Untamed => !data.is_tame(),
            AvoidGate::Untrusting => !data.ocelot_trusting().unwrap_or(false),
            AvoidGate::FoxFromPlayer | AvoidGate::FoxFromWolf | AvoidGate::FoxNotDefending => {
                !data.fox_retained().is_some_and(|fox| fox.is_defending())
            }
        }
    }

    fn candidate_allows(&self, targets: &TargetView<'_, '_, '_>, candidate: &Target) -> bool {
        match self.spec.gate {
            AvoidGate::FoxFromPlayer => !candidate.discrete() && !targets.trusts(candidate.uuid()),
            AvoidGate::FoxFromWolf => !candidate.tame(),
            AvoidGate::SpiderFromArmadillo => !candidate.armadillo_is_scared(),
            _ => true,
        }
    }

    // `ServerLevel.getNearestEntity` over `getEntitiesOfClass`
    fn nearest<'a, 'l, 'w>(
        &self,
        targets: &'a TargetView<'_, 'l, 'w>,
    ) -> Option<Target<'a, 'l, 'w>> {
        let max_dist = f64::from(self.spec.max_dist);
        let body = targets.body();
        let area = body.bounding_box().inflate(max_dist, 3.0, max_dist);
        let conditions = TargetingConditions::for_combat().range(max_dist);
        let origin = body.position();
        let mut best: Option<(Target<'a, 'l, 'w>, f64)> = None;
        let mut consider = |candidate: Target<'a, 'l, 'w>| {
            if is_instance_of(&candidate, self.spec.class)
                && candidate.bounding_box().intersects(area)
                && conditions.test(targets, &candidate, |candidate| {
                    self.candidate_allows(targets, candidate)
                })
            {
                let distance = candidate.distance_to_sqr(origin.x, origin.y, origin.z);
                if best
                    .as_ref()
                    .is_none_or(|(_, nearest)| distance.total_cmp(nearest).is_lt())
                {
                    best = Some((candidate, distance));
                }
            }
        };
        if self.spec.class == "Player" {
            for candidate in targets.local_players() {
                consider(candidate);
            }
        } else {
            for id in targets.store().ids_in_box_sections(area) {
                if let Some(candidate) = targets.facts(id) {
                    consider(candidate);
                }
            }
        }
        best.map(|(candidate, _)| candidate)
    }
}

impl Goal for AvoidEntityGoal {
    // `AvoidEntityGoal.canUse`; the path it
    // creates is asked of the navigation, which withdraws the admission when
    // there is none.
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !self.body_allows(targets) {
            return false;
        }
        let Some(avoid) = self.nearest(targets) else {
            return false;
        };
        if self.spec.gate == AvoidGate::WolfFromLlama && !wolf_fears(context, targets, &avoid) {
            return false;
        }
        let mob = RandomPosMob::of(targets.body_entity(), context.nav.malus);
        let avoid_position = avoid.position();
        let Some(Some(pos)) =
            default_random_pos_away(targets.level(), &context.random, mob, 16, 7, avoid_position)
        else {
            return false;
        };
        let body_position = context.body.position;
        if distance_sqr(avoid_position, pos) < distance_sqr(avoid_position, body_position) {
            return false;
        }
        self.to_avoid = Some(avoid.target_ref());
        context.try_navigate_to_position(pos, 0, self.spec.walk_speed);
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !context.nav.navigation_done
            && (!matches!(self.spec.gate, AvoidGate::Untamed | AvoidGate::Untrusting)
                || self.body_allows(targets))
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.spec.gate == AvoidGate::WolfFromLlama {
            context.set_target(None, targets);
        }
    }

    fn stop(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.to_avoid = None;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.spec.gate == AvoidGate::WolfFromLlama {
            context.set_target(None, targets);
        }
        let Some(avoid) = self.to_avoid.and_then(|avoid| targets.facts_of(avoid)) else {
            return;
        };
        let speed = if distance_sqr(context.body.position, avoid.position()) < 49.0 {
            self.spec.sprint_speed
        } else {
            self.spec.walk_speed
        };
        context.set_navigation_speed(speed);
    }
}

// `WolfAvoidEntityGoal.avoidLlama`: a tame wolf holds
// its ground, a wild one flees a llama as strong as `nextInt(5)`.
fn wolf_fears(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    llama: &Target,
) -> bool {
    let tame =
        matches!(targets.body_entity().data(), EntityData::GeneralLiving(data) if data.is_tame());
    if tame {
        return false;
    }
    let strength = targets
        .store()
        .get(llama.entity_id())
        .and_then(|entity| match entity.data() {
            EntityData::GeneralLiving(data) => data.horse_phenotype(),
            _ => None,
        })
        .map_or(0, |phenotype| match phenotype {
            HorsePhenotypeState::Llama { strength, .. } => strength,
            _ => 0,
        });
    strength >= context.next_int(5)
}

fn distance_sqr(a: Vec3d, b: Vec3d) -> f64 {
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    dx * dx + dy * dy + dz * dz
}
