//! `SpawnEggItem.spawnOffspringFromSpawnEgg` (`SpawnEggItem.java:151-179`)
//! through `Mob.checkAndHandleImportantInteractions` (`Mob.java:1130-1141`).

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID;
use bbb_protocol::entity_types::{
    vanilla_entity_type_id_for_resource_id, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::item_types::{vanilla_item_type, VanillaItemUseOverride};
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::vanilla_item_prototype_component_catalog;

use uuid::Uuid;

use crate::entity::animal_breeding::offspring;
use crate::entity::level::EntityRegistryTagView;
use crate::entity::store::EntityStore;
use crate::entity::EntityData;
use crate::random::GameRandom;

/// What `spawnOffspringFromSpawnEgg` answered through the Mob interact arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpawnEggOffspring {
    /// Empty optional → `PASS` (`Mob.java:1136-1137`); leash may still run.
    Pass,
    /// Present → `SUCCESS_SERVER` (`:1141`).
    Spawned,
}

/// Whether the stack's item class is `SpawnEggItem`.
pub fn held_is_spawn_egg(stack: &ItemStackSummary) -> bool {
    stack
        .item_id
        .and_then(vanilla_item_type)
        .is_some_and(|item| item.use_override == Some(VanillaItemUseOverride::SpawnEggItem))
}

/// `SpawnEggItem.getType` (`SpawnEggItem.java:146-149`) over patch-or-prototype
/// `DataComponents.ENTITY_DATA`.
pub fn spawn_egg_entity_type_id(stack: &ItemStackSummary) -> Option<i32> {
    if let Some(id) = stack.component_patch.entity_data_entity_type_id {
        return Some(id);
    }
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID)
    {
        return None;
    }
    let item_id = stack.item_id?;
    let id = vanilla_item_prototype_component_catalog()
        .ok()?
        .component_codec_values(item_id)?
        .get(&VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID)?
        .get("id")?
        .as_str()?;
    vanilla_entity_type_id_for_resource_id(id)
}

/// The level reads `getBreedOffspring` and `onOffspringSpawnedFromEgg` take
/// at the parent: its biome and the local `getSpecialMultiplier`.
#[derive(Debug, Clone, Default)]
pub struct SpawnEggLevel {
    pub biome: Option<String>,
    pub special_multiplier: f32,
}

/// The player using the egg.
pub(crate) struct SpawnEggUser<'a> {
    pub(crate) hand_stack: &'a mut ItemStackSummary,
    pub(crate) infinite_materials: bool,
    pub(crate) uuid: Uuid,
}

/// `SpawnEggItem.spawnOffspringFromSpawnEgg` (`SpawnEggItem.java:151-179`)
/// and `onOffspringSpawnedFromEgg` (`Mob.java:1135`).
pub(crate) fn spawn_offspring_from_spawn_egg(
    store: &mut EntityStore,
    parent_id: i32,
    user: SpawnEggUser<'_>,
    level: Option<&SpawnEggLevel>,
    tags: &EntityRegistryTagView,
    random: &GameRandom,
) -> SpawnEggOffspring {
    let Some(parent) = store.get(parent_id) else {
        return SpawnEggOffspring::Pass;
    };
    let EntityData::GeneralLiving(data) = parent.data() else {
        return SpawnEggOffspring::Pass;
    };
    let parent_type = parent.entity_type_id();
    // `SpawnEggItem.spawnsEntity` (`SpawnEggItem.java:138-140`).
    if spawn_egg_entity_type_id(user.hand_stack) != Some(parent_type) {
        return SpawnEggOffspring::Pass;
    }
    let biome = level.and_then(|level| level.biome.as_deref());
    let baby_id = if data.ageable_retained().is_some() {
        match parent_type {
            // `getBreedOffspring` is null (`Parrot.java:322-324`, `CamelHusk.java:66-68`,
            // `ZombieNautilus.java:55-57`, `WanderingTrader.java:97-99`), or the
            // baby can never answer `isBaby` (`Frog.java:250-256`).
            VANILLA_ENTITY_TYPE_PARROT_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
            | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
            | VANILLA_ENTITY_TYPE_FROG_ID => None,
            VANILLA_ENTITY_TYPE_VILLAGER_ID => spawn_villager(store, parent_id, biome, random),
            _ => spawn_breed_offspring(store, parent_id, biome, tags, random),
        }
    } else {
        spawn_non_ageable_baby(store, parent_id, level, random)
    };
    let Some(baby_id) = baby_id else {
        return SpawnEggOffspring::Pass;
    };
    if parent_type == VANILLA_ENTITY_TYPE_FOX_ID {
        // `Fox.onOffspringSpawnedFromEgg` (`Fox.java:591-593`).
        if let Some(baby) = store.get_mut(baby_id) {
            crate::entity::fox::breeding::set_child_trust(baby, [Some(user.uuid), None]);
        }
    }
    // `SpawnEggItem.java:175`.
    let publication = store.item_publication();
    crate::entity::implicit_components::apply_components_from_stack(
        store.get_mut(baby_id).expect("a spawned offspring"),
        user.hand_stack,
        publication.resolver(),
    );
    // `ItemStack.consume` (`ItemStack.java:1082-1086`).
    if !user.infinite_materials {
        user.hand_stack.count -= 1;
        if user.hand_stack.count <= 0 {
            *user.hand_stack = ItemStackSummary::empty();
        }
    }
    SpawnEggOffspring::Spawned
}

/// `getBreedOffspring(level, parent)` with the parent as its own partner.
fn spawn_breed_offspring(
    store: &mut EntityStore,
    parent_id: i32,
    biome: Option<&str>,
    tags: &EntityRegistryTagView,
    random: &GameRandom,
) -> Option<i32> {
    let parent = store.get(parent_id)?.clone();
    // One parent's colour mixed with itself is that colour.
    let plan = offspring::plan_offspring(&parent, store, &mut |color, _| color, parent_id, random);
    let Some(plan) = plan else {
        // A plain `EntityType.X.create` (`Squid.java:102-104`, `Mule.java:54-56`, ...).
        let baby_id = store.spawn_general_living(parent.entity_type_id(), parent.position())?;
        let baby = store.get_mut(baby_id)?;
        crate::entity::set_spawner_age(baby, offspring::BABY_START_AGE);
        baby.set_rot(0.0, 0.0);
        return Some(baby_id);
    };
    let variant = offspring::parent_variant(plan, &parent, parent_id, store)?;
    let biome_has_tag = |tag: &str| {
        biome.is_some_and(|biome| tags.contains("minecraft:worldgen/biome", tag, biome))
    };
    offspring::spawn_offspring(
        plan,
        Some(&variant),
        &parent,
        parent_id,
        store,
        &biome_has_tag,
        random,
    )
}

/// `Villager.getBreedOffspring` (`Villager.java:741-753`).
fn spawn_villager(
    store: &mut EntityStore,
    parent_id: i32,
    biome: Option<&str>,
    random: &GameRandom,
) -> Option<i32> {
    let parent = store.get(parent_id)?;
    let position = parent.position();
    let parent_type = match parent.data() {
        EntityData::GeneralLiving(data) => data.component_villager_type()?.to_owned(),
        _ => return None,
    };
    let child_type = if random.next_double() < 0.5 {
        crate::entity::villager_life::villager_type_for_biome(biome.unwrap_or("minecraft:plains"))
            .to_owned()
    } else {
        parent_type
    };
    let baby_id = store.spawn_general_living(VANILLA_ENTITY_TYPE_VILLAGER_ID, position)?;
    let baby = store.get_mut(baby_id)?;
    crate::entity::set_spawner_age(baby, offspring::BABY_START_AGE);
    crate::entity::general_living::finalize_spawner_villager_type(baby, &child_type);
    baby.set_rot(0.0, 0.0);
    Some(baby_id)
}

/// `type.create` then `setBaby(true)`; only the `DATA_BABY_ID` families answer
/// `isBaby` afterwards (`Zombie.java:187-195`, `Piglin.java:242-251`,
/// `Zoglin.java:235-240`).
fn spawn_non_ageable_baby(
    store: &mut EntityStore,
    parent_id: i32,
    level: Option<&SpawnEggLevel>,
    random: &GameRandom,
) -> Option<i32> {
    let parent = store.get(parent_id)?;
    let EntityData::GeneralLiving(data) = parent.data() else {
        return None;
    };
    let zombie = data.zombie_retained().is_some();
    let parent_type = parent.entity_type_id();
    if !zombie
        && !matches!(
            parent_type,
            VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID
        )
    {
        return None;
    }
    let position = parent.position();
    let baby_id = store.spawn_general_living(parent_type, position)?;
    let baby = store.get_mut(baby_id)?;
    let became_baby = if zombie {
        crate::entity::zombie::set_baby(baby, true);
        // `Zombie.onOffspringSpawnedFromEgg` (`Zombie.java:508-513`).
        let special = level.map_or(0.0, |level| level.special_multiplier);
        let pick_up = random.next_float() < 0.55 * special;
        if let EntityData::GeneralLiving(data) = &mut baby.data {
            if let Some(mob) = data.mob.as_mut() {
                mob.set_can_pick_up_loot(pick_up);
            }
        }
        true
    } else if parent_type == VANILLA_ENTITY_TYPE_PIGLIN_ID {
        crate::entity::general_living::set_piglin_baby(baby, true)
    } else {
        crate::entity::general_living::set_zoglin_baby(baby, true)
    };
    if became_baby && matches!(baby.data(), EntityData::GeneralLiving(data) if data.is_baby()) {
        baby.set_rot(0.0, 0.0);
        return Some(baby_id);
    }
    store.remove(baby_id, crate::entity::RemovalReason::Discarded);
    None
}

#[cfg(test)]
mod tests;
