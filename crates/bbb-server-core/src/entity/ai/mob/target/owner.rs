//! The target goals that copy another entity's fight: a tame animal's owner,
//! a trader llama's leash holder, a vex's owner and an iron golem's village.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};

use super::conditions::TargetingConditions;
use super::goals::TargetGoalState;
use super::{Target, TargetRef, TargetView};
use crate::entity::ai::mob::MobAiContext;
use crate::entity::ai::Goal;

const ABSTRACT_HORSE: &str = "AbstractHorse";
const TAMABLE_ANIMAL: &str = "TamableAnimal";

/// Which of the owner's two fight memories an owner goal copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerMemory {
    HurtBy,
    Hurt,
}

/// `OwnerHurtByTargetGoal` and `OwnerHurtTargetGoal`.
#[derive(Debug, Clone)]
pub(crate) struct OwnerTargetGoal {
    state: TargetGoalState,
    memory: OwnerMemory,
    candidate: Option<TargetRef>,
    timestamp: i32,
}

impl OwnerTargetGoal {
    pub(crate) const fn new(memory: OwnerMemory) -> Self {
        Self {
            state: TargetGoalState::new(false),
            memory,
            candidate: None,
            timestamp: 0,
        }
    }

    fn owner_memory(&self, world: &TargetView<'_, '_, '_>) -> Option<(Option<TargetRef>, i32)> {
        let owner = world.facts(world.body().owner()?)?;
        let (id, timestamp) = match self.memory {
            OwnerMemory::HurtBy => (owner.last_hurt_by_mob(), owner.last_hurt_by_mob_timestamp()),
            OwnerMemory::Hurt => (owner.last_hurt_mob(), owner.last_hurt_mob_timestamp()),
        };
        Some((
            id.and_then(|id| world.facts(id)).map(Target::target_ref),
            timestamp,
        ))
    }
}

impl Goal for OwnerTargetGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let world = &*targets;
        if !world.body().tame() || world.body().ordered_to_sit() {
            return false;
        }
        let Some((candidate, timestamp)) = self.owner_memory(world) else {
            return false;
        };
        self.candidate = candidate;
        let Some(owner) = world.body().owner().and_then(|id| world.facts(id)) else {
            return false;
        };
        let target = candidate.and_then(|candidate| world.facts_of(candidate));
        timestamp != self.timestamp
            && TargetGoalState::can_attack(
                world,
                target.as_ref(),
                TargetingConditions::DEFAULT,
                |_| true,
            )
            && target.is_some_and(|target| wants_to_attack(world, &target, &owner))
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.state.can_continue_to_use(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.set_target(self.candidate, targets);
        if let Some((_, timestamp)) = {
            let world = &*targets;
            self.owner_memory(world)
        } {
            self.timestamp = timestamp;
        }
        self.state.start();
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.state.stop(context, targets);
    }
}

fn wants_to_attack<'w>(
    world: &TargetView<'_, '_, 'w>,
    target: &Target<'_, '_, 'w>,
    owner: &Target<'_, '_, 'w>,
) -> bool {
    if world.body().entity_type_id() != VANILLA_ENTITY_TYPE_WOLF_ID {
        return true;
    }
    match target.entity_type_id() {
        VANILLA_ENTITY_TYPE_CREEPER_ID
        | VANILLA_ENTITY_TYPE_GHAST_ID
        | VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => return false,
        VANILLA_ENTITY_TYPE_WOLF_ID => {
            return !target.tame() || target.owner() != Some(owner.entity_id());
        }
        _ => {}
    }
    if target.is_player() && owner.is_player() {
        let can_harm = world.pvp()
            && (owner.team().is_none()
                || owner.team() != target.team()
                || owner
                    .team()
                    .is_some_and(|team| world.friendly_fire_allowed(team)));
        if !can_harm {
            return false;
        }
    }
    !((super::selector::is_instance_of(target, ABSTRACT_HORSE)
        || super::selector::is_instance_of(target, TAMABLE_ANIMAL))
        && target.tame())
}

#[derive(Debug, Clone)]
pub(crate) struct TraderLlamaDefendGoal {
    state: TargetGoalState,
    candidate: Option<TargetRef>,
    timestamp: i32,
}

impl TraderLlamaDefendGoal {
    pub(crate) const fn new() -> Self {
        Self {
            state: TargetGoalState::new(false),
            candidate: None,
            timestamp: 0,
        }
    }

    fn trader<'a, 'l, 'w>(world: &'a TargetView<'_, 'l, 'w>) -> Option<Target<'a, 'l, 'w>> {
        world
            .facts(world.leash_holder()?)
            .filter(|holder| holder.entity_type_id() == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID)
    }
}

impl Goal for TraderLlamaDefendGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let world = &*targets;
        let Some(trader) = Self::trader(world) else {
            return false;
        };
        self.candidate = trader
            .last_hurt_by_mob()
            .and_then(|id| world.facts(id))
            .map(Target::target_ref);
        let target = self
            .candidate
            .and_then(|candidate| world.facts_of(candidate));
        trader.last_hurt_by_mob_timestamp() != self.timestamp
            && TargetGoalState::can_attack(
                world,
                target.as_ref(),
                TargetingConditions::DEFAULT,
                |_| true,
            )
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.state.can_continue_to_use(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.set_target(self.candidate, targets);
        if let Some(trader) = Self::trader(&*targets) {
            self.timestamp = trader.last_hurt_by_mob_timestamp();
        }
        self.state.start();
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.state.stop(context, targets);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VexCopyOwnerTargetGoal {
    state: TargetGoalState,
}

impl VexCopyOwnerTargetGoal {
    const COPY_OWNER_TARGETING: TargetingConditions = TargetingConditions::for_non_combat()
        .ignore_line_of_sight()
        .ignore_invisibility_testing();

    pub(crate) const fn new() -> Self {
        Self {
            state: TargetGoalState::new(false),
        }
    }

    fn owner_target(world: &TargetView<'_, '_, '_>) -> Option<TargetRef> {
        world.facts(world.body().owner()?)?.valid_target()
    }
}

impl Goal for VexCopyOwnerTargetGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        {
            let world = &*targets;
            {
                let target = Self::owner_target(world).and_then(|target| world.facts_of(target));
                target.is_some()
                    && TargetGoalState::can_attack(
                        world,
                        target.as_ref(),
                        Self::COPY_OWNER_TARGETING,
                        |_| true,
                    )
            }
        }
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.state.can_continue_to_use(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let target = Self::owner_target(&*targets);
        context.set_target(target, targets);
        self.state.start();
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.state.stop(context, targets);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DefendVillageTargetGoal {
    state: TargetGoalState,
    potential_target: Option<TargetRef>,
}

impl DefendVillageTargetGoal {
    const ATTACK_TARGETING: TargetingConditions = TargetingConditions::for_combat().range(64.0);

    pub(crate) const fn new() -> Self {
        Self {
            state: TargetGoalState::new(false),
            potential_target: None,
        }
    }
}

impl Goal for DefendVillageTargetGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.search(context, targets)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.state.can_continue_to_use(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.set_target(self.potential_target, targets);
        self.state.start();
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.potential_target = None;
        self.state.stop(context, targets);
    }
}

impl DefendVillageTargetGoal {
    fn search(&mut self, _context: &MobAiContext, targets: &mut TargetView<'_, '_, '_>) -> bool {
        self.potential_target = None;
        let world = &*targets;
        let grow = world.body().bounding_box().inflate(10.0, 8.0, 10.0);
        let players: Vec<Target<'_, '_, '_>> = world
            .local_players()
            .filter(|player| {
                contains(grow, player) && Self::ATTACK_TARGETING.test(world, player, |_| true)
            })
            .collect();
        if players.is_empty() {
            return false;
        }
        for villager in world.local_stored_in(grow).filter(|villager| {
            villager.entity_type_id() == VANILLA_ENTITY_TYPE_VILLAGER_ID
                && Self::ATTACK_TARGETING.test(world, villager, |_| true)
        }) {
            for player in &players {
                let reputation = villager.reputation_of(player.uuid());
                if reputation <= -100 {
                    self.potential_target = Some(player.target_ref());
                }
            }
        }
        match self.potential_target {
            None => false,
            Some(potential) => world
                .facts_of(potential)
                .is_some_and(|player| !player.spectator() && !player.creative()),
        }
    }
}

fn contains(area: bbb_server_types::geometry::Aabb, facts: &Target<'_, '_, '_>) -> bool {
    let position = facts.position();
    position.x >= area.min_x
        && position.x < area.max_x
        && position.y >= area.min_y
        && position.y < area.max_y
        && position.z >= area.min_z
        && position.z < area.max_z
}
