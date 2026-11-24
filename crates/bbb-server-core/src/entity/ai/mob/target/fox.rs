use crate::entity::ai::mob::spec::{nearest, GoalSpec, MobGoal};
use crate::entity::ai::mob::MobAiRuntime;
use crate::entity::fox::FoxVariant;
use crate::entity::Entity;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;

use super::goals::NearestAttackableTargetGoal;
use super::selector::TargetSelector;

// `Fox.registerGoals` builds the three prey goals once (`Fox.java:166-172`).
const LAND: GoalSpec = nearest("Animal", false).selector(TargetSelector::ChickenOrRabbit);
const TURTLE_EGG: GoalSpec = nearest("Turtle", false).selector(TargetSelector::TurtleBabyOnLand);
const FISH: GoalSpec = nearest("AbstractFish", false)
    .every(20)
    .selector(TargetSelector::SchoolingFish);

/// `Fox.setTargetGoals`.
fn prey_goals(variant: FoxVariant) -> [(i32, GoalSpec); 3] {
    match variant {
        FoxVariant::Red => [(4, LAND), (4, TURTLE_EGG), (6, FISH)],
        FoxVariant::Snow => [(4, FISH), (6, LAND), (6, TURTLE_EGG)],
    }
}

impl MobAiRuntime {
    /// Re-adding the same goal instance is a no-op in Vanilla, so only the
    /// first configuration installs anything.
    pub(crate) fn install_fox_target_goals(&mut self, body: &Entity) {
        let Some(variant) = crate::entity::general_living::fox_target_goals(body) else {
            return;
        };
        if self.target_state.fox_prey_installed {
            return;
        }
        for (priority, goal) in prey_goals(variant) {
            let GoalSpec::Nearest(spec) = goal else {
                unreachable!("fox prey goals are nearest-target goals")
            };
            let built = NearestAttackableTargetGoal::from_spec(spec, VANILLA_ENTITY_TYPE_FOX_ID)
                .expect("the fox has a follow range");
            self.target_selector
                .add(priority, goal, MobGoal::Nearest(built));
        }
        self.target_state.fox_prey_installed = true;
    }
}

#[cfg(test)]
mod tests;
