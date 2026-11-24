//! The spear's two weapon components read off a stack, `PiercingWeapon` (the
//! jab) and `KineticWeapon` (the charge), and the reach both strike along,
//! `ProjectileUtil.getHitEntitiesAlong`. Nothing here assumes the attacker is
//! a session player, so a mob's spear goal reads the same answers.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
};
use bbb_protocol::packets::{
    ItemStackSummary, KineticWeaponConditionSummary, SoundEventHolder, Vec3d,
};
use bbb_server_types::geometry::{Aabb, Vec3Ops};
use bbb_server_types::item_stack::static_item_component;

use crate::entity::item_components::sound_holder;
use crate::entity::projectile_hit::{
    aabb_contains, exact_or_grazing_hit, ProjectileCandidate, ProjectileTarget,
};
use crate::entity::store::EntityStore;
use crate::entity::EntityData;
use crate::world::simulation_level::LevelCtx;

/// `PiercingWeapon` (`PiercingWeapon.java:25-95`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PiercingWeapon {
    pub(crate) deals_knockback: bool,
    pub(crate) dismounts: bool,
    pub(crate) sound: Option<SoundEventHolder>,
    pub(crate) hit_sound: Option<SoundEventHolder>,
}

pub(crate) fn piercing_weapon(stack: &ItemStackSummary) -> Option<PiercingWeapon> {
    let patch = static_item_component(stack, VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID)
        .expect("validated built-in weapon component")?;
    let value = patch.piercing_weapon.as_ref()?;
    Some(PiercingWeapon {
        deals_knockback: value.deals_knockback,
        dismounts: value.dismounts,
        sound: value.sound.as_ref().and_then(sound_holder),
        hit_sound: value.hit_sound.as_ref().and_then(sound_holder),
    })
}

/// `KineticWeapon.Condition` (`KineticWeapon.java:148-178`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KineticCondition {
    max_duration_ticks: i32,
    min_speed: f32,
    min_relative_speed: f32,
}

impl KineticCondition {
    fn from_summary(value: KineticWeaponConditionSummary) -> Self {
        Self {
            max_duration_ticks: value.max_duration_ticks,
            min_speed: f32::from_bits(value.min_speed_bits),
            min_relative_speed: f32::from_bits(value.min_relative_speed_bits),
        }
    }

    /// `Condition.test` (`KineticWeapon.java:167-169`).
    fn test(&self, ticks_used: i32, attacker_speed: f64, relative_speed: f64, factor: f64) -> bool {
        ticks_used <= self.max_duration_ticks
            && attacker_speed >= f64::from(self.min_speed) * factor
            && relative_speed >= f64::from(self.min_relative_speed) * factor
    }
}

/// `KineticWeapon` (`KineticWeapon.java:30-179`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KineticWeapon {
    pub(crate) contact_cooldown_ticks: i32,
    pub(crate) delay_ticks: i32,
    dismount_conditions: Option<KineticCondition>,
    knockback_conditions: Option<KineticCondition>,
    damage_conditions: Option<KineticCondition>,
    damage_multiplier: f32,
    pub(crate) sound: Option<SoundEventHolder>,
    pub(crate) hit_sound: Option<SoundEventHolder>,
}

pub(crate) fn kinetic_weapon(stack: &ItemStackSummary) -> Option<KineticWeapon> {
    let patch = static_item_component(stack, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
        .expect("validated built-in weapon component")?;
    let value = patch.kinetic_weapon.as_ref()?;
    Some(KineticWeapon {
        contact_cooldown_ticks: value.contact_cooldown_ticks,
        delay_ticks: value.delay_ticks,
        dismount_conditions: value
            .dismount_conditions
            .map(KineticCondition::from_summary),
        knockback_conditions: value
            .knockback_conditions
            .map(KineticCondition::from_summary),
        damage_conditions: value.damage_conditions.map(KineticCondition::from_summary),
        damage_multiplier: f32::from_bits(value.damage_multiplier_bits),
        sound: value.sound.as_ref().and_then(sound_holder),
        hit_sound: value.hit_sound.as_ref().and_then(sound_holder),
    })
}

/// What one kinetic contact does to its target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KineticStrike {
    pub(crate) damage: f32,
    pub(crate) deals_damage: bool,
    pub(crate) deals_knockback: bool,
    pub(crate) dismounts: bool,
}

impl KineticWeapon {
    /// `KineticWeapon.computeDamageUseDuration` (`KineticWeapon.java:97-99`).
    pub(crate) fn damage_use_duration(&self) -> i32 {
        self.delay_ticks
            + self
                .damage_conditions
                .map_or(0, |condition| condition.max_duration_ticks)
    }

    /// The per-target arm of `KineticWeapon.damageEntities`
    /// (`KineticWeapon.java:116-136`): `ticks_used` counts from the end of the
    /// delay, the speeds are along the attacker's look, `action_factor` is 1
    /// for a player and 0.2 for a mob, and `None` is a contact none of the
    /// three conditions accepts.
    pub(crate) fn strike(
        &self,
        ticks_used: i32,
        attacker_speed: f64,
        target_speed: f64,
        action_factor: f64,
        base_damage: f64,
    ) -> Option<KineticStrike> {
        let relative_speed = (attacker_speed - target_speed).max(0.0);
        let test = |condition: Option<KineticCondition>| {
            condition.is_some_and(|condition| {
                condition.test(ticks_used, attacker_speed, relative_speed, action_factor)
            })
        };
        let strike = KineticStrike {
            damage: base_damage as f32
                + (relative_speed * f64::from(self.damage_multiplier)).floor() as i32 as f32,
            deals_damage: test(self.damage_conditions),
            deals_knockback: test(self.knockback_conditions),
            dismounts: test(self.dismount_conditions),
        };
        (strike.deals_damage || strike.deals_knockback || strike.dismounts).then_some(strike)
    }
}

/// `KineticWeapon.getMotion` (`KineticWeapon.java:78-84`) from a known speed.
pub(crate) fn motion(known_speed: Vec3d) -> Vec3d {
    known_speed.scale(20.0)
}

/// The segment an attack reaches along: from the eye, `min` to `max` blocks
/// out along `look` (`max` grown by the attacker's own forward movement), with
/// `margin` around each target box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Reach {
    pub(crate) eye: Vec3d,
    pub(crate) look: Vec3d,
    pub(crate) known_movement: Vec3d,
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) margin: f32,
}

/// `PiercingWeapon.canHitEntity` (`PiercingWeapon.java:61-73`) short of its
/// two attacker-relative refusals (player harm and a shared vehicle), which
/// the attacker's owner applies.
pub(crate) fn can_hit(candidate: ProjectileCandidate<'_>, level: &LevelCtx<'_, '_>) -> bool {
    match candidate {
        ProjectileCandidate::Stored(entity) => {
            !entity.invulnerable()
                && entity.is_alive()
                && (matches!(entity.data(), EntityData::Interaction(_))
                    || entity.can_be_hit_by_projectile_in_level(level))
        }
        ProjectileCandidate::Player(player) => {
            !player.invulnerable && !player.spectator && !player.dead_or_dying
        }
    }
}

/// `ProjectileUtil.getHitEntitiesAlong` (`ProjectileUtil.java:38-47`,
/// `:86-107`) over `getManyEntityHitResult` with `includeFromEntity`
/// (`:198-240`), block clips by collision shape. Answers every target the
/// reach strikes; a block nearer than the reach's start answers none.
pub(crate) fn hits_along(
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    reach: Reach,
    matching: impl Fn(ProjectileCandidate<'_>, &LevelCtx<'_, '_>) -> bool,
) -> Vec<ProjectileTarget> {
    let from = reach.eye.plus(reach.look.scale(f64::from(reach.min)));
    let forward = reach.known_movement.x * reach.look.x
        + reach.known_movement.y * reach.look.y
        + reach.known_movement.z * reach.look.z;
    let mut to = reach
        .eye
        .plus(reach.look.scale(f64::from(reach.max) + forward.max(0.0)));
    if let Some((hit, ..)) =
        crate::entity::projectile_hit::clip_including_border(level, reach.eye, to)
    {
        to = hit;
        if reach.eye.distance_to_sqr(to) < reach.eye.distance_to_sqr(from) {
            return Vec::new();
        }
    }
    let margin = f64::from(reach.margin);
    let area = Aabb::of_size(from, margin, margin, margin)
        .expand_towards(Vec3d {
            x: to.x - from.x,
            y: to.y - from.y,
            z: to.z - from.z,
        })
        .inflate(1.0, 1.0, 1.0);
    let mut hits = Vec::new();
    let mut consider = |candidate: ProjectileCandidate<'_>, bounds: Aabb| {
        if matching(candidate, level)
            && (aabb_contains(bounds, from)
                || exact_or_grazing_hit(level, bounds, from, to, margin).is_some())
        {
            hits.push(candidate.target());
        }
    };
    store.for_each_box_section_id(area, |id| {
        if let Some(entity) = store
            .get(id)
            .filter(|entity| entity.bounding_box().intersects(area))
        {
            consider(ProjectileCandidate::Stored(entity), entity.bounding_box());
        }
    });
    for player in level.players_in_box(area) {
        consider(ProjectileCandidate::Player(&player), player.bounding_box());
    }
    // EnderDragon.java:752-753; PiercingWeapon.java:61-84; KineticWeapon.java:112-123:
    // test pickable dragon parts and emit the nearest hit part per dragon.
    for dragon in store.dragons() {
        if dragon.invulnerable() || !dragon.is_alive() {
            continue;
        }
        let nearest = crate::entity::ender_dragon::projectile_parts(dragon)
            .filter(|(_, bounds, _)| bounds.intersects(area))
            .filter_map(|(part_id, bounds, _)| {
                let part = crate::entity::ender_dragon::part_entity(store, part_id)?;
                if !matching(ProjectileCandidate::Stored(part), level) {
                    return None;
                }
                let dist_sqr = if aabb_contains(bounds, from) {
                    0.0
                } else {
                    let hit = exact_or_grazing_hit(level, bounds, from, to, margin)?;
                    from.distance_to_sqr(hit)
                };
                Some((part_id, dist_sqr))
            })
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.cmp(&right.0))
            });
        if let Some((entity_id, _)) = nearest {
            hits.push(ProjectileTarget::Stored { entity_id });
        }
    }
    hits
}
