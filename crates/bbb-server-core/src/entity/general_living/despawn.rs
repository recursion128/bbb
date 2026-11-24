//! Shared distance-despawn rules and `Mob.checkDespawn`.

use super::*;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types;

impl Entity {
    pub(crate) fn remove_when_far_away(&self, distance_squared: f64) -> Option<bool> {
        let EntityData::GeneralLiving(data) = self.data() else {
            return None;
        };
        let mob = data.mob_retained()?;
        // AbstractFish.java:54-56; Axolotl.java:549-551.
        if let Some(from_bucket) = data.is_from_bucket() {
            return Some(!from_bucket && self.custom_name().is_none());
        }
        Some(match &data.kind {
            // Chicken.java:268-270; Ocelot.java:130-132; Cat.java:452-454.
            MobVariantState::Chicken { jockey, .. } => *jockey,
            MobVariantState::Ocelot { trusting } => !*trusting && self.tick_count() > 2400,
            _ if self.entity_type_id() == entity_types::VANILLA_ENTITY_TYPE_CAT_ID => {
                !data.is_tame() && self.tick_count() > 2400
            }
            // CamelHusk.java:30-32; ZombieHorse.java:63-65; AbstractNautilus.java:447-449.
            _ if matches!(
                self.entity_type_id(),
                entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
                    | entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
                    | entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
                    | entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
            ) =>
            {
                true
            }
            // Hoglin.java:190-192; Piglin.java:193-195.
            MobVariantState::Hoglin { .. } => true,
            MobVariantState::Piglin { .. } => !mob.persistence_required(),
            // ZombieVillager.java:192-194.
            MobVariantState::ZombieVillager(_, _, cure) => {
                !cure.converting() && data.villager_work()?.xp == 0
            }
            // Raider.java:234-235; PatrollingMonster.java:98-100.
            MobVariantState::Raider(raider) | MobVariantState::Witch(raider, _) => {
                !matches!(raider.raid(), crate::entity::raider::RaiderRaid::Current(_))
                    && (!raider.patrolling() || distance_squared > 16384.0)
            }
            // AbstractGolem.java:36-38; Warden.java:470-472;
            // Villager.java:525-527; WanderingTrader.java:153-155.
            MobVariantState::CopperGolem(_)
            | MobVariantState::Shulker(_)
            | MobVariantState::Warden(_)
            | MobVariantState::Villager(_, _)
            | MobVariantState::WanderingTrader(_, _) => false,
            // Allay.java:422-424; AbstractGolem.java:36-38.
            _ if matches!(
                self.entity_type_id(),
                entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID
                    | entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
                    | entity_types::VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID
            ) =>
            {
                false
            }
            // Animal.java:127-129; Mob.java:646-647.
            _ => data.animal_retained().is_none(),
        })
    }
}

impl GeneralLivingData {
    // Mob.java:650-652; AbstractFish.java:49-51; Axolotl.java:493-495; Tadpole.java:174-176.
    // AbstractNautilus.java:533-535; EnderMan.java:397-399; Raider.java:239-241.
    pub(crate) fn requires_custom_persistence(
        &self,
        store: &EntityStore,
        entity: &Entity,
    ) -> Option<bool> {
        if store.vehicle(entity.id()).is_some()
            || crate::entity::leash::is_leashed(entity)
            || self.is_from_bucket() == Some(true)
            || matches!(
                &self.kind,
                MobVariantState::Tamable(tamable, _, _, Some(_), _) if tamable.is_tame()
            )
            || self
                .enderman_retained()
                .is_some_and(|state| state.carried_block_state_id().is_some())
        {
            return Some(true);
        }
        // A saved raid id is unknown until the level resolves it.
        match self.raider_retained().map(|state| state.raid()) {
            Some(crate::entity::raider::RaiderRaid::Current(_)) => Some(true),
            Some(crate::entity::raider::RaiderRaid::Unresolved(_)) => None,
            Some(crate::entity::raider::RaiderRaid::None) | None => Some(false),
        }
    }
}

/// `MobCategory`'s distances (`MobCategory.java:6-29`, `MobCategory.java:53-59`).
pub(crate) const fn mob_category_despawn_distance(category: VanillaEntityMobCategory) -> i32 {
    if matches!(category, VanillaEntityMobCategory::WaterAmbient) {
        64
    } else {
        128
    }
}

/// `Mob.checkDespawn` (`Mob.java`).
pub(crate) fn check_despawn(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    // EnderDragon.java:744-745: the dragon bypasses every despawn check.
    if entity.entity_type_id() == entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
        return;
    }
    if crate::entity::wither::check_despawn(entity, store.difficulty()) {
        return;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(mob) = data.mob_retained() else {
        return;
    };
    let Some((allowed_in_peaceful, mob_category)) = vanilla_entity_type(entity.entity_type_id())
        .map(|facts| (facts.allowed_in_peaceful, facts.mob_category))
        .or_else(|| {
            store
                .plugins()
                .content_registry()
                .entity_by_id(entity.entity_type_id())
                .map(|spec| (spec.allowed_in_peaceful, spec.category))
        })
    else {
        return;
    };
    if store.difficulty() == Some(Difficulty::Peaceful) && !allowed_in_peaceful {
        entity.discard();
        return;
    }

    if mob.persistence_required() || data.requires_custom_persistence(store, entity) == Some(true) {
        if let Some(living) = entity.living_mut() {
            living.no_action_time = 0;
        }
        return;
    }

    let Some(player) = level.nearest_player(entity.position(), -1.0) else {
        return;
    };
    let dx = player.position.x - entity.position().x;
    let dy = player.position.y - entity.position().y;
    let dz = player.position.z - entity.position().z;
    let distance_sqr = dx * dx + dy * dy + dz * dz;
    let distance = f64::from(mob_category_despawn_distance(mob_category));
    let remove_when_far = entity.remove_when_far_away(distance_sqr) == Some(true);
    if distance_sqr > distance * distance && remove_when_far {
        entity.discard();
    }

    let no_action_time = entity.living().map_or(0, |living| living.no_action_time);
    let random_hit = no_action_time > 600
        && store
            .mob_ai_mut(entity.id())
            .is_some_and(|runtime| runtime.next_random_int(800) == 0);
    let no_despawn_distance_sqr = 32.0 * 32.0;
    if random_hit && distance_sqr > no_despawn_distance_sqr && remove_when_far {
        entity.discard();
    } else if distance_sqr < no_despawn_distance_sqr {
        if let Some(living) = entity.living_mut() {
            living.no_action_time = 0;
        }
    }
}
