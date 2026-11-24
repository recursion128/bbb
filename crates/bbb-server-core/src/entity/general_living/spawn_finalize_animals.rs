use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;

use super::spawn_finalize::{
    data, ok, set_drop_chance, set_item, AgeableGroup, AgeableKind, Group, Spawn, SpawnGroupData,
    SpawnReason,
};
use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::fish::{SalmonVariant, TropicalFishVariant};
use crate::entity::goat::SpawnHornRemoval;
use crate::entity::Entity;

const TROPICAL_COMMON_VARIANTS: [(i32, i32, i32); 22] = [
    (0x101, 1, 7),
    (0x001, 7, 7),
    (0x001, 7, 11),
    (0x501, 0, 7),
    (0x100, 11, 7),
    (0x000, 1, 0),
    (0x500, 6, 3),
    (0x301, 10, 4),
    (0x501, 0, 14),
    (0x500, 0, 4),
    (0x201, 0, 7),
    (0x501, 0, 1),
    (0x300, 9, 6),
    (0x400, 5, 3),
    (0x401, 14, 0),
    (0x200, 7, 14),
    (0x301, 14, 0),
    (0x001, 0, 4),
    (0x000, 14, 0),
    (0x100, 7, 0),
    (0x300, 9, 4),
    (0x001, 4, 4),
];
const TROPICAL_PATTERN_IDS: [i32; 12] = [
    0x000, 0x100, 0x200, 0x300, 0x400, 0x500, 0x001, 0x101, 0x201, 0x301, 0x401, 0x501,
];

/// `(shouldSpawnBaby, babySpawnChance)` of the `AgeableMobGroupData` a plain
/// ageable starts its group with.
pub(super) fn plain_ageable_group(resource_id: &str) -> Option<(bool, f32)> {
    Some(match resource_id {
        "minecraft:armadillo"
        | "minecraft:bee"
        | "minecraft:camel"
        | "minecraft:camel_husk"
        | "minecraft:chicken"
        | "minecraft:cow"
        | "minecraft:frog"
        | "minecraft:glow_squid"
        | "minecraft:goat"
        | "minecraft:happy_ghast"
        | "minecraft:hoglin"
        | "minecraft:mooshroom"
        | "minecraft:nautilus"
        | "minecraft:pig"
        | "minecraft:sheep"
        | "minecraft:sniffer"
        | "minecraft:squid"
        | "minecraft:turtle"
        | "minecraft:zombie_nautilus" => (true, 0.05),
        "minecraft:dolphin" => (true, 0.1),
        "minecraft:donkey" | "minecraft:mule" | "minecraft:panda" | "minecraft:skeleton_horse" => {
            (true, 0.2)
        }
        "minecraft:ocelot" | "minecraft:polar_bear" => (true, 1.0),
        "minecraft:parrot" | "minecraft:villager" | "minecraft:wandering_trader" => (false, 0.05),
        _ => return None,
    })
}

impl<'registry> Spawn<'_, 'registry> {
    /// The subclass work a plain ageable does before `AgeableMob`.
    pub(super) fn ageable_prelude(&mut self, entity: &mut Entity, resource_id: &str) -> Option<()> {
        match resource_id {
            "minecraft:camel" | "minecraft:camel_husk" => ok(
                crate::entity::camel::finalize_spawn_pose(entity, self.ctx.game_time),
            ),
            "minecraft:chicken" => {
                self.registry_variant(entity, "minecraft:chicken_variant")?;
                self.registry_variant(entity, "minecraft:chicken_sound_variant")
            }
            "minecraft:cow" => {
                self.registry_variant(entity, "minecraft:cow_variant")?;
                self.registry_variant(entity, "minecraft:cow_sound_variant")
            }
            "minecraft:pig" => {
                self.registry_variant(entity, "minecraft:pig_variant")?;
                self.registry_variant(entity, "minecraft:pig_sound_variant")
            }
            "minecraft:frog" => {
                self.registry_variant(entity, "minecraft:frog_variant")?;
                let cooldown = self.random.next_int(41) as u32 + 100;
                set_memory(entity, MemoryModuleType::LongJumpCooldownTicks, cooldown)
            }
            "minecraft:goat" => self.goat(entity),
            "minecraft:hoglin" => self.hoglin(entity),
            "minecraft:nautilus" => self.nautilus(entity),
            "minecraft:zombie_nautilus" => {
                self.registry_variant(entity, "minecraft:zombie_nautilus_variant")?;
                self.nautilus(entity)
            }
            "minecraft:sheep" => self.sheep(entity),
            "minecraft:dolphin" => {
                entity.set_air_supply(4_800);
                entity.set_rot(entity.y_rot(), 0.0);
                Some(())
            }
            "minecraft:donkey" | "minecraft:mule" | "minecraft:skeleton_horse" => ok(
                crate::entity::horse_phenotype::finalize_spawn_phenotype(entity, self.random),
            ),
            "minecraft:panda" => ok(crate::entity::panda::finalize_spawn_genes(
                entity,
                self.random,
            )),
            "minecraft:parrot" => {
                let variant = self.random.next_int(5);
                ok(crate::entity::set_spawner_simple_variant(entity, variant))
            }
            "minecraft:turtle" => {
                let home = entity.block_position();
                ok(crate::entity::turtle::set_spawn_home(entity, home))
            }
            "minecraft:villager" => self.villager(entity),
            _ => Some(()),
        }
    }

    pub(super) fn axolotl(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        // `Axolotl.java:159-161`: a bucketed axolotl keeps what its bucket holds.
        if self.reason == SpawnReason::Bucket {
            return Some(group);
        }
        let (group, baby) = match group {
            Some(SpawnGroupData::Ageable(
                group @ AgeableGroup {
                    kind: AgeableKind::Axolotl(_),
                    ..
                },
            )) => {
                let baby = group.size >= 2;
                (group, baby)
            }
            _ => {
                let variants = [self.random.next_int(4), self.random.next_int(4)];
                let group = AgeableGroup::new(false, 0.05).with(AgeableKind::Axolotl(variants));
                (group, false)
            }
        };
        let AgeableKind::Axolotl(variants) = group.kind else {
            unreachable!("axolotl group")
        };
        let variant = variants[self.random.next_int(2) as usize];
        ok(crate::entity::set_spawner_simple_variant(entity, variant))?;
        if baby {
            ok(crate::entity::set_spawner_age(entity, -24_000))?;
        }
        self.ageable(entity, Some(SpawnGroupData::Ageable(group)), || {
            unreachable!("axolotl group")
        })
    }

    pub(super) fn fox(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let group = match group {
            Some(SpawnGroupData::Ageable(
                group @ AgeableGroup {
                    kind: AgeableKind::Fox { .. },
                    ..
                },
            )) => group,
            _ => AgeableGroup::new(false, 0.05).with(AgeableKind::Fox {
                snow: self.ctx.has_tag("minecraft:spawns_snow_foxes"),
            }),
        };
        let AgeableKind::Fox { snow } = group.kind else {
            unreachable!("fox group")
        };
        ok(crate::entity::fox::set_spawn_variant(entity, snow))?;
        if group.size >= 2 {
            ok(crate::entity::set_spawner_age(entity, -24_000))?;
        }
        ok(crate::entity::configure_fox_target_goals(entity))?;
        if self.random.next_float() < 0.2 {
            let odds = self.random.next_float();
            let item = if odds < 0.05 {
                "minecraft:emerald"
            } else if odds < 0.2 {
                "minecraft:egg"
            } else if odds < 0.4 {
                if self.random.next_boolean() {
                    "minecraft:rabbit_foot"
                } else {
                    "minecraft:rabbit_hide"
                }
            } else if odds < 0.6 {
                "minecraft:wheat"
            } else if odds < 0.8 {
                "minecraft:leather"
            } else {
                "minecraft:feather"
            };
            set_item(entity, EquipmentSlot::MainHand, item);
        }
        self.ageable(entity, Some(SpawnGroupData::Ageable(group)), || {
            unreachable!("fox group")
        })
    }

    pub(super) fn rabbit(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        // `Rabbit.getRandomRabbitVariant` draws before it reads the group.
        let drawn = random_rabbit_variant(self.random.next_int(100), |tag| self.ctx.has_tag(tag));
        let group = match group {
            Some(SpawnGroupData::Ageable(
                group @ AgeableGroup {
                    kind: AgeableKind::Rabbit(_),
                    ..
                },
            )) => group,
            _ => AgeableGroup::new(true, 1.0).with(AgeableKind::Rabbit(drawn)),
        };
        let AgeableKind::Rabbit(variant) = group.kind else {
            unreachable!("rabbit group")
        };
        ok(crate::entity::rabbit::set_spawn_variant(entity, variant))?;
        self.ageable(entity, Some(SpawnGroupData::Ageable(group)), || {
            unreachable!("rabbit group")
        })
    }

    pub(super) fn horse(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let carried = match &group {
            Some(SpawnGroupData::Ageable(AgeableGroup {
                kind: AgeableKind::Horse(variant),
                ..
            })) => Some(*variant),
            _ => None,
        };
        let variant = crate::entity::horse_phenotype::finalize_group_horse_phenotype(
            entity,
            self.random,
            carried,
        )?;
        let group = match carried {
            Some(_) => group,
            None => Some(SpawnGroupData::Ageable(
                AgeableGroup::new(true, 0.05).with(AgeableKind::Horse(variant)),
            )),
        };
        self.ageable(entity, group, || unreachable!("horse group"))
    }

    pub(super) fn llama(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        if self.reason == SpawnReason::Event
            && entity.entity_type_id() == VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
        {
            ok(crate::entity::set_spawner_age(entity, 0))?;
        }
        let carried = match &group {
            Some(SpawnGroupData::Ageable(AgeableGroup {
                kind: AgeableKind::Llama(variant),
                ..
            })) => Some(*variant),
            _ => None,
        };
        let variant = crate::entity::horse_phenotype::finalize_group_llama_phenotype(
            entity,
            self.random,
            carried,
        )?;
        let group = match carried {
            Some(_) => group,
            None => Some(SpawnGroupData::Ageable(
                AgeableGroup::new(true, 0.05).with(AgeableKind::Llama(variant)),
            )),
        };
        self.ageable(entity, group, || unreachable!("llama group"))
    }

    pub(super) fn wolf(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let group = match group {
            Some(SpawnGroupData::Ageable(
                group @ AgeableGroup {
                    kind: AgeableKind::Wolf { .. },
                    ..
                },
            )) => group,
            _ => {
                let (_, resource_id, holder_id) = self.select_variant("minecraft:wolf_variant")?;
                AgeableGroup::new(false, 0.05).with(AgeableKind::Wolf {
                    resource_id: resource_id.to_owned(),
                    holder_id,
                })
            }
        };
        let AgeableKind::Wolf {
            resource_id,
            holder_id,
        } = &group.kind
        else {
            unreachable!("wolf group")
        };
        ok(crate::entity::set_spawner_registry_value(
            entity,
            "minecraft:wolf_variant",
            resource_id,
            *holder_id,
        ))?;
        // `Wolf.java:268-269`: each wolf chooses its own sound variant.
        let (resource_id, holder_id) = super::pick_random_wolf_sound_variant(
            self.random,
            self.level.entities().item_resolver(),
        )?;
        ok(crate::entity::set_spawner_registry_value(
            entity,
            "minecraft:wolf_sound_variant",
            &resource_id,
            holder_id,
        ))?;
        self.ageable(entity, Some(SpawnGroupData::Ageable(group)), || {
            unreachable!("wolf group")
        })
    }

    pub(super) fn cat(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let group = self.ageable(entity, group, || AgeableGroup::new(true, 0.05))?;
        if self.variant_count("minecraft:cat_variant") != 0 {
            self.registry_variant(entity, "minecraft:cat_variant")?;
        }
        self.registry_variant(entity, "minecraft:cat_sound_variant")?;
        Some(group)
    }

    pub(super) fn strider(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let baby = data(entity)
            .and_then(|data| data.ageable_retained())
            .is_some_and(|age| age.is_baby());
        if baby {
            return self.ageable(entity, group, || AgeableGroup::new(true, 0.05));
        }
        let yaw = Some(entity.y_rot());
        let group = if self.random.next_int(30) == 0 {
            let rider_group = SpawnGroupData::Zombie {
                baby: self.random.next_float() < 0.05,
                can_spawn_jockey: false,
            };
            if let Some(mut rider) = self.secondary(
                entity,
                "minecraft:zombified_piglin",
                yaw,
                SpawnReason::Jockey,
                Some(rider_group),
                |_| true,
            ) {
                set_item(
                    &mut rider,
                    EquipmentSlot::MainHand,
                    "minecraft:warped_fungus_on_a_stick",
                );
                self.link(entity, rider, false, true);
                set_item(entity, EquipmentSlot::Saddle, "minecraft:saddle");
                set_drop_chance(entity, EquipmentSlot::Saddle, 2.0);
            }
            AgeableGroup::new(true, 0.0)
        } else if self.random.next_int(10) == 0 {
            if let Some(rider) = self.secondary(
                entity,
                "minecraft:strider",
                yaw,
                SpawnReason::Jockey,
                None,
                |rider| crate::entity::set_spawner_age(rider, -24_000),
            ) {
                self.link(entity, rider, false, true);
            }
            AgeableGroup::new(true, 0.0)
        } else {
            AgeableGroup::new(true, 0.5)
        };
        self.ageable(entity, Some(SpawnGroupData::Ageable(group)), || {
            unreachable!("strider group")
        })
    }

    pub(super) fn zombie_horse(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        if self.reason == SpawnReason::Natural {
            if let Some(mut rider) = self.secondary(
                entity,
                "minecraft:zombie",
                Some(entity.y_rot()),
                self.reason,
                None,
                |_| true,
            ) {
                set_item(&mut rider, EquipmentSlot::MainHand, "minecraft:iron_spear");
                self.link(entity, rider, false, false);
            }
        }
        ok(crate::entity::horse_phenotype::finalize_spawn_phenotype(
            entity,
            self.random,
        ))?;
        self.ageable(entity, group, || AgeableGroup::new(true, 0.2))
    }

    /// `AbstractSchoolingFish.finalizeSpawn` with the Salmon and TropicalFish
    /// overrides around it.
    pub(super) fn schooling_fish(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let resource_id = entity.data().entity_resource_id();
        if resource_id == "minecraft:salmon" {
            let roll = self.random.next_int(95);
            let variant = match roll {
                0..=29 => SalmonVariant::Small,
                30..=79 => SalmonVariant::Medium,
                _ => SalmonVariant::Large,
            };
            ok(crate::entity::fish::set_spawn_salmon_variant(
                entity, variant,
            ))?;
        }
        self.mob(entity)?;
        let (leader, tropical) = match group {
            Some(SpawnGroupData::School { leader, tropical }) => (Some(leader), tropical),
            _ => (None, None),
        };
        ok(crate::entity::general_living::set_schooling_fish_leader(
            entity, leader,
        ))?;
        let leader = leader.unwrap_or_else(|| entity.uuid());
        if resource_id != "minecraft:tropical_fish" {
            return Some(Some(SpawnGroupData::School { leader, tropical }));
        }
        let (variant, group) = match tropical {
            Some(variant) => (variant, SpawnGroupData::School { leader, tropical }),
            None if self.random.next_float() < 0.9 => {
                let (pattern, base, color) =
                    TROPICAL_COMMON_VARIANTS[self.random.next_int(22) as usize];
                let variant = TropicalFishVariant::new(pattern, base, color)?;
                let group = SpawnGroupData::School {
                    leader: entity.uuid(),
                    tropical: Some(variant),
                };
                (variant, group)
            }
            None => {
                let pattern = TROPICAL_PATTERN_IDS[self.random.next_int(12) as usize];
                let base = self.random.next_int(16);
                let color = self.random.next_int(16);
                ok(crate::entity::set_tropical_fish_schooling(entity, false))?;
                let group = SpawnGroupData::School {
                    leader,
                    tropical: None,
                };
                (TropicalFishVariant::new(pattern, base, color)?, group)
            }
        };
        ok(crate::entity::fish::set_spawn_tropical_variant(
            entity, variant,
        ))?;
        Some(Some(group))
    }

    fn goat(&mut self, entity: &mut Entity) -> Option<()> {
        let long_jump = self.random.next_int(601) as u32 + 600;
        let ram = self.random.next_int(5_401) as u32 + 600;
        set_memory(entity, MemoryModuleType::LongJumpCooldownTicks, long_jump)?;
        set_memory(entity, MemoryModuleType::RamCooldownTicks, ram)?;
        let screaming = self.random.next_double() < 0.02;
        let removal = if self.random.next_float() < 0.1 {
            if self.random.next_boolean() {
                SpawnHornRemoval::Left
            } else {
                SpawnHornRemoval::Right
            }
        } else {
            SpawnHornRemoval::None
        };
        ok(crate::entity::goat::apply_spawn_traits(
            entity, screaming, removal,
        ))
    }

    fn hoglin(&mut self, entity: &mut Entity) -> Option<()> {
        let baby = self.random.next_float() < 0.2;
        ok(crate::entity::set_spawner_age(
            entity,
            if baby { -24_000 } else { 0 },
        ))?;
        ok(crate::entity::set_spawn_experience_reward(
            entity,
            if baby { 3 } else { 5 },
        ))?;
        entity
            .living_mut()?
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)?
            .set_base_value(if baby { 0.5 } else { 6.0 });
        Some(())
    }

    fn nautilus(&mut self, entity: &mut Entity) -> Option<()> {
        let cooldown = self.random.next_int(1_201) as u32 + 2_400;
        set_memory(entity, MemoryModuleType::AttackTargetCooldown, cooldown)
    }

    fn sheep(&mut self, entity: &mut Entity) -> Option<()> {
        // `SheepColorSpawnRules`: the 499:1 pink draw only on the common arm.
        let roll = self.random.next_int(100);
        let warm = self
            .ctx
            .has_tag("minecraft:spawns_warm_variant_farm_animals");
        let cold = self
            .ctx
            .has_tag("minecraft:spawns_cold_variant_farm_animals");
        let color = match roll {
            0..=4 if warm => 7,
            5..=9 if warm => 8,
            10..=14 if warm => 0,
            15..=17 if warm => 15,
            0..=4 if cold => 8,
            5..=9 if cold => 7,
            10..=14 if cold => 0,
            15..=17 if cold => 12,
            0..=4 => 15,
            5..=9 => 7,
            10..=14 => 8,
            15..=17 => 12,
            _ if self.random.next_int(500) == 499 => 6,
            _ if warm => 12,
            _ if cold => 15,
            _ => 0,
        };
        ok(crate::entity::sheep::set_spawn_color(entity, color))
    }

    fn variant_count(&self, registry: &str) -> usize {
        self.ctx
            .variants
            .iter()
            .filter(|(candidate, ..)| *candidate == registry)
            .count()
    }

    fn select_variant(&mut self, registry: &str) -> Option<(&'static str, &'registry str, i32)> {
        let count = i32::try_from(self.variant_count(registry)).ok()?;
        if count == 0 {
            return None;
        }
        let selected = self.random.next_int(count) as usize;
        self.ctx
            .variants
            .iter()
            .filter(|(candidate, ..)| *candidate == registry)
            .nth(selected)
            .copied()
    }

    fn registry_variant(&mut self, entity: &mut Entity, registry: &str) -> Option<()> {
        let (registry, resource_id, holder_id) = self.select_variant(registry)?;
        ok(crate::entity::set_spawner_registry_value(
            entity,
            registry,
            resource_id,
            holder_id,
        ))
    }
}

fn set_memory(entity: &mut Entity, memory_type: MemoryModuleType, value: u32) -> Option<()> {
    ok(crate::entity::set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type,
            value: MemoryValue::Int(i32::try_from(value).ok()?),
            time_to_live: None,
        },
    ))
}

#[cfg(test)]
mod tests;

/// `Rabbit.getRandomRabbitVariant` (`Rabbit.java:422-437`) over its
/// `nextInt(100)` roll and the biome's tags.
pub(crate) fn random_rabbit_variant(roll: i32, has_tag: impl Fn(&str) -> bool) -> i32 {
    if has_tag("minecraft:spawns_white_rabbits") {
        if roll < 80 {
            1
        } else {
            3
        }
    } else if has_tag("minecraft:spawns_gold_rabbits") {
        4
    } else if roll < 50 {
        0
    } else if roll < 90 {
        5
    } else {
        2
    }
}
