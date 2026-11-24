//! `TargetingConditions` and the `canAttack`/`isAlliedTo` answers it asks of
//! the targeter.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BREEZE_ID, VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
    VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_CREEPER_HEAD_ID, VANILLA_ITEM_PIGLIN_HEAD_ID, VANILLA_ITEM_SKELETON_SKULL_ID,
    VANILLA_ITEM_ZOMBIE_HEAD_ID,
};

use super::{Target, TargetView};
use crate::entity::ai::mob::spec;
use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TargetingConditions {
    combat: bool,
    range: f64,
    check_line_of_sight: bool,
    test_invisible: bool,
}

impl TargetingConditions {
    /// `TargetingConditions.DEFAULT`.
    pub(crate) const DEFAULT: Self = Self::for_combat();

    pub(crate) const fn for_combat() -> Self {
        Self {
            combat: true,
            range: -1.0,
            check_line_of_sight: true,
            test_invisible: true,
        }
    }

    pub(crate) const fn for_non_combat() -> Self {
        Self {
            combat: false,
            ..Self::for_combat()
        }
    }

    pub(crate) const fn range(mut self, range: f64) -> Self {
        self.range = range;
        self
    }

    pub(crate) const fn ignore_line_of_sight(mut self) -> Self {
        self.check_line_of_sight = false;
        self
    }

    pub(crate) const fn ignore_invisibility_testing(mut self) -> Self {
        self.test_invisible = false;
        self
    }

    pub(crate) fn test<'w>(
        self,
        world: &TargetView<'_, '_, 'w>,
        target: &Target<'_, '_, 'w>,
        selector: impl FnOnce(&Target<'_, '_, 'w>) -> bool,
    ) -> bool {
        self.test_checked(world, target, |facts| Some(selector(facts)))
            .expect("known selector")
    }

    pub(crate) fn test_checked<'w>(
        self,
        world: &TargetView<'_, '_, 'w>,
        target: &Target<'_, '_, 'w>,
        selector: impl FnOnce(&Target<'_, '_, 'w>) -> Option<bool>,
    ) -> Option<bool> {
        let body = &world.body();
        if target.entity_id() == body.entity_id() || !can_be_seen_by_anyone(target) {
            return Some(false);
        }
        if !selector(target)? {
            return Some(false);
        }
        if self.combat
            && (!can_attack(world.peaceful(), body, target) || is_allied_to(world, body, target))
        {
            return Some(false);
        }
        if !in_targeting_range(body, target, self.range, !self.test_invisible) {
            return Some(false);
        }
        Some(!self.check_line_of_sight || target.line_of_sight())
    }
}

pub(crate) fn in_targeting_range(
    body: &Target<'_, '_, '_>,
    target: &Target<'_, '_, '_>,
    range: f64,
    ignore_invisibility: bool,
) -> bool {
    if range > 0.0 {
        let modifier = if ignore_invisibility {
            1.0
        } else {
            visibility_percent(target, body.entity_type_id())
        };
        let visibility_distance = (range * modifier).max(2.0);
        if body.distance_to_facts_sqr(target) > visibility_distance * visibility_distance {
            return false;
        }
    }
    true
}

pub(crate) fn can_be_seen_by_anyone(target: &Target<'_, '_, '_>) -> bool {
    can_be_seen_by_anyone_values(
        target.armor_stand_marker(),
        target.invisible(),
        target.spectator(),
        target.alive(),
    )
}

fn can_be_seen_by_anyone_values(
    armor_stand_marker: Option<bool>,
    invisible: bool,
    spectator: bool,
    alive: bool,
) -> bool {
    // ArmorStand does not include the ordinary alive check.
    match armor_stand_marker {
        Some(marker) => !invisible && !marker,
        None => !spectator && alive,
    }
}

pub(crate) fn nonplayer_can_be_seen_as_enemy(entity: &Entity) -> bool {
    let marker = match entity.data() {
        EntityData::ArmorStand(data) => Some(data.is_marker()),
        _ => None,
    };
    can_be_seen_as_enemy(
        false,
        false,
        crate::entity::axolotl::is_playing_dead(entity),
        entity.invulnerable(),
        can_be_seen_by_anyone_values(
            marker,
            entity.is_invisible(),
            false,
            entity.is_alive_with_health(),
        ),
    )
}

pub(crate) fn can_be_seen_as_enemy(
    peaceful_player: bool,
    creative_or_spectator: bool,
    playing_dead: bool,
    invulnerable: bool,
    visible: bool,
) -> bool {
    !peaceful_player && !creative_or_spectator && !playing_dead && !invulnerable && visible
}

// AbstractIllager excludes baby villagers.
pub(crate) fn can_attack_baby_target(
    attacker_type: i32,
    target_type: i32,
    target_baby: bool,
) -> bool {
    !(target_baby
        && spec::is_instance_of(attacker_type, "AbstractIllager")
        && spec::is_instance_of(target_type, "AbstractVillager"))
}

// Dolphin prevents baby dolphins from attacking.
pub(crate) fn can_attack_as_baby(attacker_type: i32, attacker_baby: bool) -> bool {
    !attacker_baby || attacker_type != VANILLA_ENTITY_TYPE_DOLPHIN_ID
}

// TamableAnimal excludes the resolved owner.
pub(crate) fn can_attack_owner(attacker_type: i32, owner_is_target: bool) -> bool {
    !owner_is_target || !spec::is_instance_of(attacker_type, "TamableAnimal")
}

pub(crate) fn can_attack(
    peaceful: bool,
    attacker: &Target<'_, '_, '_>,
    target: &Target<'_, '_, '_>,
) -> bool {
    let attacker_type = attacker.entity_type_id();
    if attacker_type == VANILLA_ENTITY_TYPE_BREEZE_ID
        && !(target.is_player() || target.entity_type_id() == VANILLA_ENTITY_TYPE_IRON_GOLEM_ID)
    {
        return false;
    }
    if attacker_type == VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
        && ((attacker.player_created() && target.is_player())
            || target.entity_type_id() == VANILLA_ENTITY_TYPE_CREEPER_ID)
    {
        return false;
    }
    if !can_attack_baby_target(attacker_type, target.entity_type_id(), target.baby()) {
        return false;
    }
    if !can_attack_as_baby(attacker_type, attacker.baby()) {
        return false;
    }
    if !can_attack_owner(attacker_type, attacker.owner() == Some(target.entity_id())) {
        return false;
    }
    if !attacker.is_player() && target.entity_type_id() == VANILLA_ENTITY_TYPE_GHAST_ID {
        return false;
    }
    // LivingEntity: Peaceful disallows player targets.
    can_be_seen_as_enemy(
        target.is_player() && peaceful,
        target.creative() || target.spectator(),
        target.playing_dead(),
        target.invulnerable(),
        can_be_seen_by_anyone(target),
    )
}

#[derive(Clone, Copy)]
struct AllyFacts {
    entity_id: i32,
    entity_type_id: i32,
    tame: bool,
    root_owner: Option<i32>,
    team: Option<u32>,
    illager_friend: bool,
    owner: Option<i32>,
}

impl AllyFacts {
    fn of(target: &Target<'_, '_, '_>) -> Self {
        Self {
            entity_id: target.entity_id(),
            entity_type_id: target.entity_type_id(),
            tame: target.tame(),
            root_owner: target.root_owner(),
            team: target.team(),
            illager_friend: target.illager_friend(),
            owner: target.owner(),
        }
    }
}

pub(crate) fn is_allied_to(
    world: &TargetView<'_, '_, '_>,
    first: &Target<'_, '_, '_>,
    second: &Target<'_, '_, '_>,
) -> bool {
    first.entity_id() == second.entity_id()
        || considers_entity_as_ally(world, AllyFacts::of(first), AllyFacts::of(second))
        || considers_entity_as_ally(world, AllyFacts::of(second), AllyFacts::of(first))
}

fn considers_entity_as_ally(
    world: &TargetView<'_, '_, '_>,
    mut this: AllyFacts,
    mut other: AllyFacts,
) -> bool {
    let mut seen = Vec::new();
    loop {
        if seen.contains(&(this.entity_id, other.entity_id)) {
            return false;
        }
        if spec::is_instance_of(this.entity_type_id, "TamableAnimal") && this.tame {
            if let Some(owner) = this.root_owner {
                if other.entity_id == owner {
                    return true;
                }
                if let Some(owner) = world.facts(owner) {
                    seen.push((this.entity_id, other.entity_id));
                    this = AllyFacts::of(&owner);
                    continue;
                }
            }
        }
        if this.entity_type_id == VANILLA_ENTITY_TYPE_EVOKER_ID && other.entity_id == this.entity_id
        {
            return true;
        }
        let this_team = this.team;
        if this_team.is_some() && this_team == other.team {
            return true;
        }
        if spec::is_instance_of(this.entity_type_id, "AbstractIllager")
            && other.illager_friend
            && this_team.is_none()
            && other.team.is_none()
        {
            return true;
        }
        if this.entity_type_id == VANILLA_ENTITY_TYPE_EVOKER_ID
            && other.entity_type_id == VANILLA_ENTITY_TYPE_VEX_ID
        {
            if let Some(vex_owner) = other.owner {
                if vex_owner == this.entity_id {
                    return true;
                }
                if let Some(owner) = world.facts(vex_owner) {
                    seen.push((this.entity_id, other.entity_id));
                    other = AllyFacts::of(&owner);
                    continue;
                }
            }
        }
        return false;
    }
}

pub(crate) fn visibility_percent(target: &Target<'_, '_, '_>, targeter_type_id: i32) -> f64 {
    let mut percent = 1.0;
    if target.discrete() {
        percent *= 0.8;
    }
    if target.invisible() {
        percent *= 0.7 * f64::from(target.armor_cover().max(0.1));
    }
    let disguise = match targeter_type_id {
        VANILLA_ENTITY_TYPE_SKELETON_ID => VANILLA_ITEM_SKELETON_SKULL_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID => VANILLA_ITEM_ZOMBIE_HEAD_ID,
        VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => {
            VANILLA_ITEM_PIGLIN_HEAD_ID
        }
        VANILLA_ENTITY_TYPE_CREEPER_ID => VANILLA_ITEM_CREEPER_HEAD_ID,
        _ => return percent,
    };
    if target.head_item_id() == Some(disguise) {
        percent *= 0.5;
    }
    percent
}

pub(crate) fn as_valid_target(
    peaceful: bool,
    attacker: &Target<'_, '_, '_>,
    target: &Target<'_, '_, '_>,
) -> bool {
    if attacker.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID {
        return target.warden_targetable_from(attacker);
    }
    !(target.is_player() && (target.creative() || target.spectator()))
        && can_attack(peaceful, attacker, target)
}

/// `Mob.asValidTarget` for the ticking body.
pub(crate) fn is_valid_target<'w>(
    world: &TargetView<'_, '_, 'w>,
    target: &Target<'_, '_, 'w>,
) -> bool {
    as_valid_target(world.peaceful(), &world.body(), target)
}
