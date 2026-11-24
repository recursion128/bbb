//! Iron golem flower offering.

use bbb_protocol::entity_types::vanilla_entity_type;

use super::target::conditions::TargetingConditions;
use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct OfferFlowerGoal {
    target: Option<TargetRef>,
    remaining: u16,
}

impl Goal for OfferFlowerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if targets.bright_outside() != Some(true) || context.next_int(8000) != 0 {
            return false;
        }
        let bounds = targets.body_entity().bounding_box().inflate(6.0, 2.0, 6.0);
        let conditions = TargetingConditions::for_non_combat().range(6.0);
        self.target = targets
            .store()
            .ids_in_box_sections(bounds)
            .into_iter()
            .filter_map(|id| {
                targets.store().get(id)?.living()?;
                targets.facts(id)
            })
            .chain(targets.local_players())
            .filter(|target| {
                target.bounding_box().intersects(bounds)
                    && vanilla_entity_type(target.entity_type_id()).is_some_and(|kind| {
                        targets.level().registry_tag_membership(
                            "minecraft:entity_type",
                            "minecraft:candidate_for_iron_golem_gift",
                            kind.resource_id,
                        )
                    })
                    && conditions.test(targets, target, |_| true)
            })
            .min_by(|a, b| {
                a.distance_to_facts_sqr(&targets.body())
                    .total_cmp(&b.distance_to_facts_sqr(&targets.body()))
            })
            .map(|target| target.target_ref());
        self.target.is_some()
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.remaining > 0
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.remaining = 400;
        if let Some((entity, store, level)) = targets.world_mut() {
            crate::entity::iron_golem::apply_flower_change(entity, store, level, Some(true), None);
        }
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let target = self.target.take();
        let gift = (self.remaining == 0).then_some(target).flatten();
        if let Some((entity, store, level)) = targets.world_mut() {
            crate::entity::iron_golem::apply_flower_change(entity, store, level, Some(false), gift);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(target) = self.target.and_then(|target| targets.facts_of(target)) {
            let position = target.position();
            context.set_look_at_with_limits(position.x, target.eye_y(), position.z, 30.0, 30.0);
        }
        self.remaining = self.remaining.saturating_sub(1);
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
