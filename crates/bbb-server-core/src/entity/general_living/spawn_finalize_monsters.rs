use crate::random::GameRandom;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID,
    VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID,
};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::vanilla_entity_type_dimensions;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Aabb;

use super::spawn_finalize::{
    data, data_mut, item, ok, set_drop_chance, set_item, Group, Spawn, SpawnGroupData, SpawnReason,
};
use crate::entity::ai::{MemoryGlobalPos, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::attributes::{AttributeModifier, AttributeOperation};
use crate::entity::item::plain_stack;
use crate::entity::living::ActiveMobEffect;
use crate::entity::zombie::ZombieRetainedState;
use crate::entity::{Entity, EntityData};
use bbb_server_types::item_stack::RuntimeItemStack;

const RANDOM_SPAWN_BONUS: &str = "minecraft:random_spawn_bonus";
const ZOMBIE_RANDOM_SPAWN_BONUS: &str = "minecraft:zombie_random_spawn_bonus";
const LEADER_ZOMBIE_BONUS: &str = "minecraft:leader_zombie_bonus";

impl Spawn<'_, '_> {
    pub(super) fn spider(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        self.mob(entity)?;
        if self.random.next_int(100) == 0 {
            if let Some(skeleton) = self.secondary(
                entity,
                "minecraft:skeleton",
                Some(entity.y_rot()),
                self.reason,
                None,
                |_| true,
            ) {
                self.link(entity, skeleton, false, false);
            }
        }
        // `Spider.SpiderEffectsGroupData`: the first member picks the effect.
        let effect = match group {
            Some(SpawnGroupData::Spider { effect }) => effect,
            _ if self.ctx.difficulty == Difficulty::Hard
                && self.random.next_float() < 0.1 * self.ctx.special_multiplier =>
            {
                Some(match self.random.next_int(5) {
                    0 | 1 => "minecraft:speed",
                    2 => "minecraft:strength",
                    3 => "minecraft:regeneration",
                    _ => "minecraft:invisibility",
                })
            }
            _ => None,
        };
        if let Some(resource_id) = effect {
            let effect = ActiveMobEffect::from_saved(resource_id, -1, 0, false, true, true)?;
            entity.living_mut()?.add_active_effect(effect);
        }
        Some(Some(SpawnGroupData::Spider { effect }))
    }

    /// `AbstractSkeleton.finalizeSpawn`; WitherSkeleton swaps in a stone
    /// sword and suppresses armor and enchantments.
    pub(super) fn skeleton(&mut self, entity: &mut Entity, wither: bool) -> Option<()> {
        self.mob(entity)?;
        if !wither {
            self.populate_default_armor(entity);
        }
        let weapon = if wither {
            "minecraft:stone_sword"
        } else {
            "minecraft:bow"
        };
        set_item(entity, EquipmentSlot::MainHand, weapon);
        if !wither {
            self.enchant_equipment(entity);
        }
        self.roll_pickup(entity)?;
        self.halloween_head(entity);
        if wither {
            entity
                .living_mut()?
                .attributes_mut()
                .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)?
                .set_base_value(4.0);
        }
        Some(())
    }

    /// `Zombie.finalizeSpawn` with the Drowned, Husk, ZombieVillager and
    /// ZombifiedPiglin overrides.
    pub(super) fn zombie(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let resource_id = entity.data().entity_resource_id();
        if resource_id == "minecraft:zombie_villager" {
            if super::type_variants::villager_finalized(entity) == Some(false) {
                // `ZombieVillager.initializeVillagerData` (`ZombieVillager.java:142-148`).
                let profession =
                    super::type_variants::VILLAGER_PROFESSIONS[self.random.next_int(15) as usize];
                ok(super::type_variants::set_villager_profession(
                    entity, profession,
                ))?;
            }
            self.villager_type(entity)?;
        }
        self.mob(entity)?;
        let conversion = self.reason == SpawnReason::Conversion;
        if !conversion {
            self.roll_pickup(entity)?;
        }
        let (baby, can_spawn_jockey) = match group {
            Some(SpawnGroupData::Zombie {
                baby,
                can_spawn_jockey,
            }) => (baby, can_spawn_jockey),
            _ => (self.random.next_float() < 0.05, true),
        };
        if baby {
            ok(crate::entity::zombie::set_baby(entity, true))?;
            if can_spawn_jockey {
                self.chicken_jockey(entity);
            }
        }
        let doors = self.random.next_float() < self.ctx.special_multiplier * 0.1;
        let is_baby = zombie_is_baby(entity);
        ok(crate::entity::zombie::apply_conversion_phenotype(
            entity, is_baby, doors,
        ))?;
        match resource_id {
            _ if conversion => {}
            "minecraft:drowned" => {
                if self.random.next_float() > 0.9 {
                    let weapon = if self.random.next_int(16) < 10 {
                        "minecraft:trident"
                    } else {
                        "minecraft:fishing_rod"
                    };
                    set_item(entity, EquipmentSlot::MainHand, weapon);
                }
            }
            "minecraft:zombified_piglin" => {
                let weapon = if self.random.next_int(20) == 0 {
                    "minecraft:golden_spear"
                } else {
                    "minecraft:golden_sword"
                };
                set_item(entity, EquipmentSlot::MainHand, weapon);
            }
            _ => {
                self.populate_default_armor(entity);
                let chance = if self.ctx.difficulty == Difficulty::Hard {
                    0.05
                } else {
                    0.01
                };
                if self.random.next_float() < chance {
                    let weapon = match self.random.next_int(6) {
                        0 => "minecraft:iron_sword",
                        1 => "minecraft:iron_spear",
                        _ => "minecraft:iron_shovel",
                    };
                    set_item(entity, EquipmentSlot::MainHand, weapon);
                }
            }
        }
        if !conversion {
            self.enchant_equipment(entity);
        }
        self.halloween_head(entity);
        zombie_handle_attributes(
            entity,
            self.random,
            self.ctx.special_multiplier,
            resource_id != "minecraft:zombified_piglin",
            conversion,
        )?;
        let group = Some(SpawnGroupData::Zombie {
            baby,
            can_spawn_jockey,
        });
        match resource_id {
            "minecraft:drowned" => self.drowned_suffix(entity),
            "minecraft:husk" => self.husk_suffix(entity)?,
            _ => {}
        }
        Some(group)
    }

    fn chicken_jockey(&mut self, entity: &mut Entity) {
        if self.random.next_float() < 0.05 {
            let search = entity.bounding_box().inflate(5.0, 3.0, 5.0);
            if let Some(chicken) = self.level.first_unridden("minecraft:chicken", search) {
                let store = self.level.entities();
                if store
                    .get_mut(chicken)
                    .is_some_and(crate::entity::set_spawner_chicken_jockey)
                    && store.vehicle(entity.id()).is_none()
                {
                    store.fresh_ride(entity, chicken, false);
                }
            }
        } else if self.random.next_float() < 0.05 {
            if let Some(chicken) = self.secondary(
                entity,
                "minecraft:chicken",
                Some(entity.y_rot()),
                SpawnReason::Jockey,
                None,
                crate::entity::set_spawner_chicken_jockey,
            ) {
                self.link(entity, chicken, true, false);
            }
        }
    }

    /// `Drowned.finalizeSpawn` after Zombie: the shell, then the nautilus a
    /// NATURAL or STRUCTURE trident drowned rides.
    fn drowned_suffix(&mut self, entity: &mut Entity) {
        if item(entity, EquipmentSlot::OffHand).is_none() && self.random.next_float() < 0.03 {
            set_item(entity, EquipmentSlot::OffHand, "minecraft:nautilus_shell");
            set_drop_chance(entity, EquipmentSlot::OffHand, 2.0);
        }
        let trident = item(entity, EquipmentSlot::MainHand)
            .is_some_and(|stack| stack.item_resource_id() == Some("minecraft:trident"));
        if !matches!(self.reason, SpawnReason::Natural | SpawnReason::Structure)
            || !trident
            || self.random.next_float() >= 0.5
            || zombie_is_baby(entity)
            || self.ctx.has_tag("minecraft:more_frequent_drowned_spawns")
        {
            return;
        }
        let structure = self.reason == SpawnReason::Structure;
        if let Some(vehicle) = self.secondary(
            entity,
            "minecraft:zombie_nautilus",
            Some(entity.y_rot()),
            self.reason,
            None,
            |vehicle| !structure || crate::entity::minecart_spawner::spawner_runtime::set_spawner_mob_persistence_required(
                    vehicle,
                ),
        ) {
            self.link(entity, vehicle, true, false);
        }
    }

    /// `Husk.finalizeSpawn` after Zombie: the second pickup roll, then the
    /// CamelHusk and Parched a NATURAL husk may bring.
    fn husk_suffix(&mut self, entity: &mut Entity) -> Option<()> {
        self.roll_pickup(entity)?;
        if self.reason != SpawnReason::Natural {
            return Some(());
        }
        let block = entity.block_position();
        let [width, height] = vanilla_entity_type_dimensions(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        )?;
        let bounds = Aabb::of_entity(
            Vec3d {
                x: f64::from(block.x) + 0.5,
                y: f64::from(block.y),
                z: f64::from(block.z) + 0.5,
            },
            width,
            height,
        );
        if !self.level.no_collision(bounds) || self.random.next_float() >= 0.1 {
            return Some(());
        }
        set_item(entity, EquipmentSlot::MainHand, "minecraft:iron_spear");
        let Some(camel) = self.secondary(
            entity,
            "minecraft:camel_husk",
            None,
            self.reason,
            None,
            |_| true,
        ) else {
            return Some(());
        };
        let Some(camel_id) = self.link(entity, camel, true, true) else {
            return Some(());
        };
        if let Some(mut parched) = self.secondary(
            entity,
            "minecraft:parched",
            Some(entity.y_rot()),
            self.reason,
            None,
            |_| true,
        ) {
            let store = self.level.entities();
            if store.fresh_ride(&mut parched, camel_id, false) {
                store.add_fresh(parched);
            } else {
                store.discard_fresh(parched);
            }
        }
        Some(())
    }

    /// `Pillager`, `Vindicator`, `Illusioner`, `Evoker`, `Ravager` and
    /// `Witch` over `Raider` and `PatrollingMonster`.
    pub(super) fn raider(&mut self, entity: &mut Entity) -> Option<()> {
        let resource_id = entity.data().entity_resource_id();
        match resource_id {
            "minecraft:illusioner" => set_item(entity, EquipmentSlot::MainHand, "minecraft:bow"),
            "minecraft:pillager" => {
                set_item(entity, EquipmentSlot::MainHand, "minecraft:crossbow");
                self.enchant_slot(entity, EquipmentSlot::MainHand, 0.25);
                // `Pillager.enchantSpawnedWeapon`'s own provider follows the inherited one.
                if self.random.next_int(300) == 0
                    && item(entity, EquipmentSlot::MainHand)
                        .is_some_and(|stack| stack.item_resource_id() == Some("minecraft:crossbow"))
                {
                    self.apply_provider(
                        entity,
                        EquipmentSlot::MainHand,
                        "minecraft:pillager_spawn_crossbow",
                    );
                }
                self.enchant_armor(entity);
            }
            _ => {}
        }
        let can_join_raid = resource_id != "minecraft:witch" || self.reason != SpawnReason::Natural;
        let become_leader = !matches!(
            self.reason,
            SpawnReason::Patrol | SpawnReason::Event | SpawnReason::Structure
        ) && self.random.next_float() < 0.06
            && !matches!(resource_id, "minecraft:witch" | "minecraft:ravager");
        let leader = crate::entity::raider::finalize_spawn_state(
            entity,
            can_join_raid,
            become_leader,
            self.reason == SpawnReason::Patrol,
        )?;
        if leader {
            let banner = super::spawn_finalize_equipment::ominous_banner_stack(
                self.level.entities().item_resolver(),
            );
            entity
                .living_mut()?
                .set_item_slot(EquipmentSlot::Head, Some(banner));
            set_drop_chance(entity, EquipmentSlot::Head, 2.0);
        }
        self.mob(entity)?;
        if resource_id == "minecraft:vindicator" {
            // A raid's wave equips its vindicators through `applyRaidBuffs`.
            if !crate::entity::raider::holds_current_raid(entity) {
                set_item(entity, EquipmentSlot::MainHand, "minecraft:iron_axe");
            }
            self.enchant_equipment(entity);
        }
        Some(())
    }

    pub(super) fn piglin(&mut self, entity: &mut Entity) -> Option<()> {
        if self.reason != SpawnReason::Structure {
            if self.random.next_float() < 0.2 {
                ok(super::type_variants::set_piglin_baby(entity, true))?;
            } else {
                let random = self.random;
                let weapon = if random.next_float() < 0.5 {
                    "minecraft:crossbow"
                } else if random.next_int(10) == 0 {
                    "minecraft:golden_spear"
                } else {
                    "minecraft:golden_sword"
                };
                set_item(entity, EquipmentSlot::MainHand, weapon);
            }
        }
        let baby = data(entity)?.piglin_or_zoglin_baby().unwrap_or(false);
        crate::entity::piglin::dont_hunt(entity, self.random);
        if !baby {
            for (slot, armor) in [
                (EquipmentSlot::Head, "minecraft:golden_helmet"),
                (EquipmentSlot::Chest, "minecraft:golden_chestplate"),
                (EquipmentSlot::Legs, "minecraft:golden_leggings"),
                (EquipmentSlot::Feet, "minecraft:golden_boots"),
            ] {
                if self.random.next_float() < 0.1 {
                    set_item(entity, slot, armor);
                }
            }
        }
        self.enchant_equipment(entity);
        self.mob(entity)
    }

    pub(super) fn piglin_brute(&mut self, entity: &mut Entity) -> Option<()> {
        let pos = entity.block_position();
        data_mut(entity)?.set_brain_memory(PackedMemory {
            memory_type: MemoryModuleType::Home,
            value: MemoryValue::GlobalPos(MemoryGlobalPos {
                dimension: self.ctx.dimension.clone(),
                pos,
            }),
            time_to_live: None,
        });
        set_item(entity, EquipmentSlot::MainHand, "minecraft:golden_axe");
        self.mob(entity)
    }

    pub(super) fn slime(&mut self, entity: &mut Entity) -> Option<()> {
        let mut scale = self.random.next_int(3);
        if scale < 2 && self.random.next_float() < 0.5 * self.ctx.special_multiplier {
            scale += 1;
        }
        ok(crate::entity::slime::set_size(entity, 1_i32 << scale, true))?;
        self.mob(entity)
    }

    pub(super) fn vex(&mut self, entity: &mut Entity) -> Option<()> {
        set_item(entity, EquipmentSlot::MainHand, "minecraft:iron_sword");
        set_drop_chance(entity, EquipmentSlot::MainHand, 0.0);
        self.enchant_equipment(entity);
        self.mob(entity)
    }

    pub(super) fn warden(&mut self, entity: &mut Entity) -> Option<()> {
        crate::entity::warden::lifecycle::finalize_spawn(
            entity,
            self.level.entities(),
            self.reason == SpawnReason::Triggered,
        );
        self.mob(entity)
    }

    pub(super) fn zoglin(&mut self, entity: &mut Entity) -> Option<()> {
        let baby = self.random.next_float() < 0.2;
        ok(super::type_variants::set_zoglin_baby(entity, baby))?;
        self.mob(entity)
    }

    /// `Villager.finalizeSpawn` before `AbstractVillager`.
    pub(super) fn villager(&mut self, entity: &mut Entity) -> Option<()> {
        self.villager_type(entity)?;
        if self.reason == SpawnReason::Structure {
            ok(super::type_variants::set_villager_assign_profession_when_spawned(entity))?;
        }
        Some(())
    }

    fn villager_type(&mut self, entity: &mut Entity) -> Option<()> {
        if super::type_variants::villager_finalized(entity) == Some(true) {
            return Some(());
        }
        let biome = self.ctx.biome.as_deref()?;
        ok(super::type_variants::set_villager_type(
            entity,
            crate::entity::villager_life::villager_type_for_biome(biome),
        ))
    }
}

/// `Zombie.handleAttributes` (`Zombie.java:529-556`), also run alone when a
/// zombie converts to another zombie type.
pub(crate) fn zombie_handle_attributes(
    entity: &mut Entity,
    random: &GameRandom,
    special: f32,
    reinforcements: bool,
    conversion: bool,
) -> Option<()> {
    let attributes = entity.living_mut()?.attributes_mut();
    attributes
        .get_instance(VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID)?
        .set_base_value(if reinforcements {
            random.next_double() * f64::from(0.1_f32)
        } else {
            0.0
        });
    attributes
        .get_instance(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)?
        .add_or_replace_permanent_modifier(AttributeModifier {
            id: RANDOM_SPAWN_BONUS.to_owned(),
            amount: random.next_double() * f64::from(0.05_f32),
            operation: AttributeOperation::AddValue,
        });
    let follow = random.next_double() * 1.5 * f64::from(special);
    if follow > 1.0 {
        attributes
            .get_instance(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)?
            .add_or_replace_permanent_modifier(AttributeModifier {
                id: ZOMBIE_RANDOM_SPAWN_BONUS.to_owned(),
                amount: follow,
                operation: AttributeOperation::AddMultipliedTotal,
            });
    }
    let leader = random.next_float() < special * 0.05;
    if leader {
        attributes
            .get_instance(VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID)?
            .add_or_replace_permanent_modifier(AttributeModifier {
                id: LEADER_ZOMBIE_BONUS.to_owned(),
                amount: random.next_double() * 0.25 + 0.5,
                operation: AttributeOperation::AddValue,
            });
        attributes
            .get_instance(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)?
            .add_or_replace_permanent_modifier(AttributeModifier {
                id: LEADER_ZOMBIE_BONUS.to_owned(),
                amount: random.next_double() * 3.0 + 1.0,
                operation: AttributeOperation::AddMultipliedTotal,
            });
    }
    if leader {
        if !conversion {
            let living = entity.living_mut()?;
            living.set_health(living.max_health());
        }
        let baby = zombie_is_baby(entity);
        ok(crate::entity::zombie::apply_conversion_phenotype(
            entity, baby, true,
        ))?;
    }
    Some(())
}

fn zombie_is_baby(entity: &Entity) -> bool {
    data(entity)
        .and_then(|data| data.zombie_retained())
        .is_some_and(ZombieRetainedState::is_baby)
}

/// `Raider.applyRaidBuffs` for the two wave types that own a body
/// (`Pillager.java:234-255`, `Vindicator.java:168-180`); the caller passes the
/// provider its wave selects, `None` only below the pillager's fourth wave, and
/// `enchant` applies it. False when the entity is no raider.
pub fn apply_raid_buffs(
    entity: &mut Entity,
    entity_random: &GameRandom,
    enchant_odds: f32,
    provider: Option<&str>,
    enchant: impl FnOnce(&str, &RuntimeItemStack) -> Option<RuntimeItemStack>,
) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    let weapon = match data.entity_resource_id() {
        "minecraft:pillager" => "minecraft:crossbow",
        "minecraft:vindicator" => "minecraft:iron_axe",
        "minecraft:evoker" | "minecraft:ravager" | "minecraft:witch" => return true,
        _ => return false,
    };
    let provider = (entity_random.next_float() <= enchant_odds)
        .then_some(provider)
        .flatten();
    if provider.is_none() && weapon == "minecraft:crossbow" {
        return true;
    }
    let mut stack = plain_stack(weapon, 1).expect("raider weapons are built-in 26.1 items");
    if let Some(enchanted) = provider.and_then(|provider| enchant(provider, &stack)) {
        stack = enchanted;
    }
    if let Some(living) = entity.living_mut() {
        living.set_item_slot(EquipmentSlot::MainHand, Some(stack));
    }
    true
}

#[cfg(test)]
mod tests;
