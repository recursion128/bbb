use crate::entity::natural_spawn_facts_26_1::vanilla_natural_spawn_type_facts_by_id_26_1;
use crate::entity::{Entity, EntityData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalSpawnPreFinalizeMobFacts {
    pub remove_when_far_away: bool,
    pub turtle_going_home: bool,
    pub strider_is_in_lava: bool,
}

// `NaturalSpawner.java:286-290`.
pub fn natural_spawn_pre_finalize_mob_facts(
    entity: &Entity,
    distance_squared: f64,
) -> Option<NaturalSpawnPreFinalizeMobFacts> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let resource_id =
        if let Some(facts) = vanilla_natural_spawn_type_facts_by_id_26_1(entity.entity_type_id()) {
            if !facts.is_mob || data.entity_resource_id() != facts.resource_id {
                return None;
            }
            facts.resource_id
        } else if data.plugin_mob().is_some() && data.is_mob() {
            data.entity_resource_id()
        } else {
            return None;
        };
    // The pre-finalize query cannot resolve a saved raid reference.
    if data.raider_retained().is_some_and(|raider| {
        matches!(
            raider.raid(),
            crate::entity::raider::RaiderRaid::Unresolved(_)
        )
    }) {
        return None;
    }
    let remove_when_far_away = entity.remove_when_far_away(distance_squared)?;
    let turtle_going_home = match resource_id {
        "minecraft:turtle" => data.turtle_retained()?.going_home(),
        _ => false,
    };
    let strider_is_in_lava = match resource_id {
        "minecraft:strider" => {
            data.strider_state()?;
            entity.is_in_lava()
        }
        _ => false,
    };
    Some(NaturalSpawnPreFinalizeMobFacts {
        remove_when_far_away,
        turtle_going_home,
        strider_is_in_lava,
    })
}

#[cfg(test)]
mod tests;
