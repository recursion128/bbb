//! `Mob.finalizeSpawn` and its subclass overrides: one synchronous pass per
//! spawned mob, which creates, finalizes and links the riders it brings.

use crate::random::GameRandom;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::geometry::Aabb;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::fish::TropicalFishVariant;
use crate::entity::item::plain_stack;
use crate::entity::save::create_fresh_entity;
use crate::entity::store::EntityStore;
use crate::entity::{finalize_spawner_mob, Entity, EntityData, GeneralLivingData};

/// `EntitySpawnReason`, limited to the reasons a finalizer here can see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnReason {
    Natural,
    ChunkGeneration,
    Spawner,
    Structure,
    Command,
    SpawnItemUse,
    Dispenser,
    Bucket,
    Patrol,
    Event,
    Jockey,
    Triggered,
    MobSummoned,
    Reinforcement,
    Conversion,
    Breeding,
}

/// What `finalizeSpawn` reads from the level at the mob's block position.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnContext<'registry> {
    pub game_time: i64,
    pub difficulty: Difficulty,
    pub special_multiplier: f32,
    pub dimension: String,
    pub biome: Option<String>,
    pub biome_tags: Vec<&'static str>,
    /// `(registry, resource id, holder id)` of each datapack variant whose
    /// spawn conditions select it here.
    pub variants: Vec<(&'static str, &'registry str, i32)>,
    pub halloween: bool,
}

impl SpawnContext<'_> {
    pub(super) fn has_tag(&self, tag: &str) -> bool {
        self.biome_tags.contains(&tag)
    }
}

/// `SpawnGroupData`: what one natural cluster's members share.
#[derive(Clone, Debug, PartialEq)]
pub enum SpawnGroupData {
    Ageable(AgeableGroup),
    Zombie {
        baby: bool,
        can_spawn_jockey: bool,
    },
    Spider {
        effect: Option<&'static str>,
    },
    School {
        leader: Uuid,
        tropical: Option<TropicalFishVariant>,
    },
}

/// `AgeableMob.AgeableMobGroupData` and the subclasses that also carry a variant.
#[derive(Clone, Debug, PartialEq)]
pub struct AgeableGroup {
    pub(super) size: i32,
    should_spawn_baby: bool,
    baby_chance: f32,
    pub(super) kind: AgeableKind,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AgeableKind {
    Plain,
    Axolotl([i32; 2]),
    Fox { snow: bool },
    Horse(i32),
    Llama(i32),
    Rabbit(i32),
    Wolf { resource_id: String, holder_id: i32 },
}

impl AgeableGroup {
    pub(super) fn new(should_spawn_baby: bool, baby_chance: f32) -> Self {
        Self {
            size: 0,
            should_spawn_baby,
            baby_chance,
            kind: AgeableKind::Plain,
        }
    }

    pub(super) fn with(mut self, kind: AgeableKind) -> Self {
        self.kind = kind;
        self
    }
}

/// The level half of `finalizeSpawn`: entity storage, per-position facts and
/// the world reads a few finalizers make.
pub(crate) trait SpawnLevel<'registry> {
    fn entities(&mut self) -> &mut EntityStore;
    fn spawn_context(&self, resource_id: &str, pos: BlockPos) -> Option<SpawnContext<'registry>>;
    /// `EnchantmentHelper.enchantItemFromProvider`; `None` leaves the stack as is.
    fn enchant(
        &self,
        provider_id: &str,
        stack: &RuntimeItemStack,
        special_multiplier: f32,
        random: &GameRandom,
    ) -> Option<RuntimeItemStack>;
    /// The first living, unridden `resource_id` entity in `search_box`.
    fn first_unridden(&self, resource_id: &str, search_box: Aabb) -> Option<i32>;
    fn no_collision(&self, bounds: Aabb) -> bool;
}

pub(super) type Group = Option<SpawnGroupData>;

/// Finalizes a fresh mob with the facts at its block position. `None` when the
/// level has no facts there; otherwise the group data the next member inherits.
pub(crate) fn finalize_spawn<'registry>(
    level: &mut dyn SpawnLevel<'registry>,
    entity: &mut Entity,
    reason: SpawnReason,
    group: Group,
    random: &GameRandom,
) -> Option<Group> {
    let ctx = level.spawn_context(entity.data().entity_resource_id(), entity.block_position())?;
    finalize_spawn_with(level, entity, reason, group, &ctx, random)
}

/// [`finalize_spawn`] under facts the caller prepared itself.
pub(crate) fn finalize_spawn_with<'registry>(
    level: &mut dyn SpawnLevel<'registry>,
    entity: &mut Entity,
    reason: SpawnReason,
    group: Group,
    ctx: &SpawnContext<'registry>,
    random: &GameRandom,
) -> Option<Group> {
    Spawn {
        level,
        random,
        ctx,
        reason,
    }
    .run(entity, group)
}

pub(super) struct Spawn<'a, 'registry> {
    pub(super) level: &'a mut dyn SpawnLevel<'registry>,
    pub(super) random: &'a GameRandom,
    pub(super) ctx: &'a SpawnContext<'registry>,
    pub(super) reason: SpawnReason,
}

impl Spawn<'_, '_> {
    fn run(&mut self, entity: &mut Entity, group: Group) -> Option<Group> {
        let resource_id = match entity.data() {
            EntityData::GeneralLiving(data) if data.is_mob() => data.entity_resource_id(),
            _ => return Some(group),
        };
        match resource_id {
            "minecraft:axolotl" => return self.axolotl(entity, group),
            "minecraft:cat" => return self.cat(entity, group),
            "minecraft:fox" => return self.fox(entity, group),
            "minecraft:horse" => return self.horse(entity, group),
            "minecraft:llama" | "minecraft:trader_llama" => return self.llama(entity, group),
            "minecraft:rabbit" => return self.rabbit(entity, group),
            "minecraft:strider" => return self.strider(entity, group),
            "minecraft:wolf" => return self.wolf(entity, group),
            "minecraft:zombie_horse" => return self.zombie_horse(entity, group),
            "minecraft:cod" | "minecraft:salmon" | "minecraft:tropical_fish" => {
                return self.schooling_fish(entity, group)
            }
            "minecraft:spider" => return self.spider(entity, group),
            "minecraft:drowned"
            | "minecraft:husk"
            | "minecraft:zombie"
            | "minecraft:zombie_villager"
            | "minecraft:zombified_piglin" => return self.zombie(entity, group),
            // `CaveSpider.finalizeSpawn` calls neither Spider nor Mob.
            "minecraft:cave_spider" => return Some(group),
            _ => {}
        }
        if let Some((baby, chance)) =
            super::spawn_finalize_animals::plain_ageable_group(resource_id)
        {
            self.ageable_prelude(entity, resource_id)?;
            return self.ageable(entity, group, || AgeableGroup::new(baby, chance));
        }
        match resource_id {
            "minecraft:bogged" | "minecraft:parched" | "minecraft:skeleton" | "minecraft:stray" => {
                self.skeleton(entity, false)
            }
            "minecraft:wither_skeleton" => self.skeleton(entity, true),
            "minecraft:evoker"
            | "minecraft:illusioner"
            | "minecraft:pillager"
            | "minecraft:ravager"
            | "minecraft:vindicator"
            | "minecraft:witch" => self.raider(entity),
            "minecraft:piglin" => self.piglin(entity),
            "minecraft:piglin_brute" => self.piglin_brute(entity),
            "minecraft:slime" | "minecraft:magma_cube" => self.slime(entity),
            "minecraft:vex" => self.vex(entity),
            "minecraft:warden" => self.warden(entity),
            "minecraft:zoglin" => self.zoglin(entity),
            "minecraft:shulker" => {
                entity.set_rot(0.0, entity.x_rot());
                self.mob(entity)
            }
            "minecraft:phantom" => {
                ok(crate::entity::phantom::finalize_spawn_state(entity))?;
                self.mob(entity)
            }
            "minecraft:copper_golem" => {
                crate::entity::copper_golem::play_spawn_sound(entity, self.level.entities());
                self.mob(entity)
            }
            _ => {
                self.mob(entity)?;
                let behavior = entity.plugin_mob().and_then(|_| {
                    self.level
                        .entities()
                        .plugins()
                        .mob_entry(entity.entity_type_id())
                        .map(|entry| std::sync::Arc::clone(&entry.behavior))
                });
                if let Some(behavior) = behavior {
                    behavior.on_finalize_spawn(entity, self.level.entities());
                }
                Some(())
            }
        }?;
        Some(group)
    }

    /// `Mob.finalizeSpawn`.
    pub(super) fn mob(&mut self, entity: &mut Entity) -> Option<()> {
        ok(finalize_spawner_mob(entity, self.random))
    }

    /// `AgeableMob.finalizeSpawn` after the subclass prelude.
    pub(super) fn ageable(
        &mut self,
        entity: &mut Entity,
        group: Group,
        default: impl FnOnce() -> AgeableGroup,
    ) -> Option<Group> {
        let mut group = match group {
            Some(SpawnGroupData::Ageable(group)) => group,
            _ => default(),
        };
        if group.should_spawn_baby
            && group.size > 0
            && self.random.next_float() <= group.baby_chance
        {
            ok(crate::entity::set_spawner_age(entity, -24_000))?;
        }
        group.size = group.size.wrapping_add(1);
        self.mob(entity)?;
        Some(Some(SpawnGroupData::Ageable(group)))
    }

    /// Creates and finalizes a rider or mount at `root`'s position; `yaw` is
    /// the `snapTo` rotation, absent for a plain `setPos`.
    pub(super) fn secondary(
        &mut self,
        root: &Entity,
        resource_id: &str,
        yaw: Option<f32>,
        reason: SpawnReason,
        group: Group,
        prepare: impl FnOnce(&mut Entity) -> bool,
    ) -> Option<Box<Entity>> {
        let mut child = create_fresh_entity(self.level.entities(), resource_id, root.position())?;
        if let Some(yaw) = yaw {
            child.set_rot(yaw, 0.0);
            child.set_old_pos_and_rot();
        }
        let finalized = prepare(&mut child)
            && self
                .level
                .spawn_context(resource_id, child.block_position())
                .is_some_and(|ctx| {
                    Spawn {
                        level: &mut *self.level,
                        random: self.random,
                        ctx: &ctx,
                        reason,
                    }
                    .run(&mut child, group)
                    .is_some()
                });
        if !finalized {
            self.level.entities().discard_fresh(child);
            return None;
        }
        Some(child)
    }

    /// Publishes `child` linked to the still-fresh `root`.
    pub(super) fn link(
        &mut self,
        root: &mut Entity,
        child: Box<Entity>,
        child_is_vehicle: bool,
        forced: bool,
    ) -> Option<i32> {
        self.level
            .entities()
            .add_fresh_linked(root, child, child_is_vehicle, forced)
    }

    pub(super) fn roll_pickup(&mut self, entity: &mut Entity) -> Option<()> {
        let pickup = self.random.next_float() < 0.55 * self.ctx.special_multiplier;
        data_mut(entity)?.mob.as_mut()?.set_can_pick_up_loot(pickup);
        Some(())
    }

    /// `Mob.populateDefaultEquipmentSlots`.
    pub(super) fn populate_default_armor(&mut self, entity: &mut Entity) {
        let random = self.random;
        if random.next_float() >= 0.15 * self.ctx.special_multiplier {
            return;
        }
        let mut armor_type = random.next_int(3);
        for _ in 0..3 {
            if random.next_float() < 0.1087 {
                armor_type += 1;
            }
        }
        let partial_chance = if self.ctx.difficulty == Difficulty::Hard {
            0.1
        } else {
            0.25
        };
        let material = match armor_type {
            0 => "leather",
            1 => "copper",
            2 => "golden",
            3 => "chainmail",
            4 => "iron",
            _ => "diamond",
        };
        for (index, (slot, piece)) in [
            (EquipmentSlot::Head, "helmet"),
            (EquipmentSlot::Chest, "chestplate"),
            (EquipmentSlot::Legs, "leggings"),
            (EquipmentSlot::Feet, "boots"),
        ]
        .into_iter()
        .enumerate()
        {
            if index != 0 && random.next_float() < partial_chance {
                break;
            }
            if item(entity, slot).is_none() {
                set_item(entity, slot, &format!("minecraft:{material}_{piece}"));
            }
        }
    }

    /// `Mob.populateDefaultEquipmentEnchantments`.
    pub(super) fn enchant_equipment(&mut self, entity: &mut Entity) {
        self.enchant_slot(entity, EquipmentSlot::MainHand, 0.25);
        self.enchant_armor(entity);
    }

    pub(super) fn enchant_armor(&mut self, entity: &mut Entity) {
        for slot in [
            EquipmentSlot::Feet,
            EquipmentSlot::Legs,
            EquipmentSlot::Chest,
            EquipmentSlot::Head,
        ] {
            self.enchant_slot(entity, slot, 0.5);
        }
    }

    pub(super) fn enchant_slot(&mut self, entity: &mut Entity, slot: EquipmentSlot, chance: f32) {
        if item(entity, slot).is_some()
            && self.random.next_float() < chance * self.ctx.special_multiplier
        {
            self.apply_provider(entity, slot, "minecraft:mob_spawn_equipment");
        }
    }

    pub(super) fn apply_provider(
        &mut self,
        entity: &mut Entity,
        slot: EquipmentSlot,
        provider: &str,
    ) {
        let Some(stack) = item(entity, slot) else {
            return;
        };
        if let Some(stack) =
            self.level
                .enchant(provider, &stack, self.ctx.special_multiplier, self.random)
        {
            if let Some(living) = entity.living_mut() {
                living.set_item_slot(slot, Some(stack));
            }
        }
    }

    pub(super) fn halloween_head(&mut self, entity: &mut Entity) {
        if item(entity, EquipmentSlot::Head).is_none()
            && self.ctx.halloween
            && self.random.next_float() < 0.25
        {
            let head = if self.random.next_float() < 0.1 {
                "minecraft:jack_o_lantern"
            } else {
                "minecraft:carved_pumpkin"
            };
            set_item(entity, EquipmentSlot::Head, head);
            set_drop_chance(entity, EquipmentSlot::Head, 0.0);
        }
    }
}

pub(super) fn ok(value: bool) -> Option<()> {
    value.then_some(())
}

pub(super) fn data(entity: &Entity) -> Option<&GeneralLivingData> {
    match entity.data() {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

pub(super) fn data_mut(entity: &mut Entity) -> Option<&mut GeneralLivingData> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

pub(super) fn item(entity: &Entity, slot: EquipmentSlot) -> Option<RuntimeItemStack> {
    entity.living()?.item_by_slot(slot).cloned()
}

pub(super) fn set_item(entity: &mut Entity, slot: EquipmentSlot, resource_id: &str) {
    let stack = plain_stack(resource_id, 1)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in 26.1 item"));
    if let Some(living) = entity.living_mut() {
        living.set_item_slot(slot, Some(stack));
    }
}

pub(super) fn set_drop_chance(entity: &mut Entity, slot: EquipmentSlot, chance: f32) {
    if let Some(mob) = data_mut(entity).and_then(|data| data.mob.as_mut()) {
        mob.set_drop_chance(slot, chance);
    }
}

#[cfg(test)]
pub(crate) mod test_level;
