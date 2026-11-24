//! The shared record's constructor, accessors and validated-field restore.

use super::type_variants::VillagerState;
use super::*;

impl GeneralLivingData {
    /// Constructs the base state when `DefaultAttributes` proves the catalog
    /// type is living. Players remain session-owned and armor stands keep their
    /// specialized runtime.
    pub fn new(entity_type_id: i32) -> Option<Self> {
        if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        ) {
            return None;
        }
        let catalog = vanilla_entity_type(entity_type_id)?;
        let mut living = LivingEntityState::new(catalog.resource_id)?;
        if entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID {
            crate::entity::shulker::update_armor(&mut living, 0);
        }
        let mut mob =
            (entity_type_id != VANILLA_ENTITY_TYPE_MANNEQUIN_ID).then(MobRetainedState::default);
        if matches!(
            entity_type_id,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
        ) {
            // These constructors enable loot pickup (`Dolphin.java:80-85`, `Fox.java:145-152`,
            // `Villager.java:195`, `AbstractPiglin.java:33-35`);
            // Panda's guards on `isBaby()`, false while constructing (`Panda.java:105-111`).
            mob.as_mut()
                .expect("classified entity is a Mob")
                .set_can_pick_up_loot(true);
        }
        // ElderGuardian.java:26-31.
        if entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID {
            mob.as_mut()
                .expect("elder guardians are mobs")
                .set_persistence_required(true);
        }
        let mut opaque_fields = Vec::new();
        construction::extend_opaque_constructor_defaults(catalog.resource_id, &mut opaque_fields);
        let neutral_anger = match entity_type_id {
            // ZombifiedPiglin's Java long starts at zero and NeutralMob always writes it
            // (`ZombifiedPiglin.java:48-66`, `NeutralMob.java:34-37`).
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
                Some(crate::entity::ai::mob::target::anger::NeutralAnger {
                    end_time: 0,
                    target: None,
                })
            }
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID => {
                Some(crate::entity::ai::mob::target::anger::NeutralAnger {
                    end_time: -1,
                    target: None,
                })
            }
            _ => None,
        };
        let animal = AnimalRetainedState::new(entity_type_id);
        let ageable = AgeableMobRetainedState::new(entity_type_id, animal.is_some());
        Some(Self {
            entity_resource_id: catalog.resource_id,
            body: Box::new(GeneralLivingBody {
                living,
                // `Mob.defineSynchedData` and its flag accessors (`Mob.java:264-267`,
                // `Mob.java:1282-1306`). Mannequin extends the non-Mob Avatar
                // (`Mannequin.java:27-27`, `Avatar.java:13-13`).
                mob,
                ageable,
                animal,
                internal_inventory:
                    crate::entity::living_internal_inventory::LivingInternalInventory::new(
                        entity_type_id,
                    ),
                brain: BrainPacked::empty(),
                variants: type_variants::TypeVariants::new(entity_type_id),
                neutral_anger,
                kind: MobVariantState::new(entity_type_id),
                opaque_fields,
            }),
        })
    }

    pub fn new_with_plugins(
        entity_type_id: i32,
        plugins: &crate::plugin::ServerPluginRegistry,
    ) -> Option<Self> {
        if vanilla_entity_type(entity_type_id).is_some() {
            return Self::new(entity_type_id);
        }
        let entry = plugins.mob_entry(entity_type_id)?;
        if !entry.spec.is_living {
            return None;
        }
        let living = LivingEntityState::new_modded(&entry.spec.default_attributes);
        let mob = entry.spec.is_mob.then(MobRetainedState::default);
        let plugin_state = crate::entity::plugin_mob::PluginMobState::from_spec(
            &entry.spec,
            entry.behavior.as_ref(),
        );
        Some(Self {
            entity_resource_id: entry.spec.resource_id_static(),
            body: Box::new(GeneralLivingBody {
                living,
                mob,
                ageable: None,
                animal: None,
                internal_inventory: None,
                brain: BrainPacked::empty(),
                variants: type_variants::TypeVariants::new(entity_type_id),
                neutral_anger: None,
                kind: MobVariantState::Plugin(plugin_state),
                opaque_fields: Vec::new(),
            }),
        })
    }

    pub const fn entity_resource_id(&self) -> &'static str {
        self.entity_resource_id
    }

    pub fn living(&self) -> &LivingEntityState {
        &self.body.living
    }

    pub fn living_mut(&mut self) -> &mut LivingEntityState {
        &mut self.body.living
    }

    pub fn is_mob(&self) -> bool {
        self.body.mob.is_some()
    }

    pub(crate) fn mob_flags(&self) -> Option<i8> {
        self.body.mob.as_ref().map(MobRetainedState::flags)
    }

    pub(crate) fn mob_retained(&self) -> Option<&MobRetainedState> {
        self.body.mob.as_ref()
    }

    pub(crate) fn mob_retained_mut(&mut self) -> Option<&mut MobRetainedState> {
        self.body.mob.as_mut()
    }

    pub fn plugin_mob(&self) -> Option<&crate::entity::plugin_mob::PluginMobState> {
        match &self.body.kind {
            MobVariantState::Plugin(plugin) => Some(plugin),
            _ => None,
        }
    }

    pub fn plugin_mob_mut(&mut self) -> Option<&mut crate::entity::plugin_mob::PluginMobState> {
        match &mut self.body.kind {
            MobVariantState::Plugin(plugin) => Some(plugin),
            _ => None,
        }
    }

    pub(crate) fn ageable_retained(&self) -> Option<AgeableMobRetainedState> {
        self.body.ageable
    }

    /// `LivingEntity.isBaby()` (`:528-530`) as the overrides this crate
    /// carries answer it: `AgeableMob` (`AgeableMob.java:241-243`) and the
    /// `DATA_BABY_ID` families (`Zombie.java:173-175`, `Zoglin.java:243-245`,
    /// `Piglin.java:254-256`).
    pub(crate) fn is_baby(&self) -> bool {
        // Frog.java:250-252; Parrot.java:157-159: negative Age still means adult geometry.
        if matches!(
            self.entity_resource_id(),
            "minecraft:zombie_nautilus"
                | "minecraft:camel_husk"
                | "minecraft:frog"
                | "minecraft:parrot"
        ) {
            return false;
        }
        self.ageable_retained()
            .is_some_and(AgeableMobRetainedState::is_baby)
            || self
                .zombie_retained()
                .is_some_and(crate::entity::zombie::ZombieRetainedState::is_baby)
            || self.piglin_or_zoglin_baby() == Some(true)
    }

    pub(crate) fn ageable_retained_mut(&mut self) -> Option<&mut AgeableMobRetainedState> {
        self.ageable.as_mut()
    }

    pub(crate) fn animal_retained(&self) -> Option<AnimalRetainedState> {
        self.body.animal
    }

    pub(crate) fn animal_retained_mut(&mut self) -> Option<&mut AnimalRetainedState> {
        self.body.animal.as_mut()
    }

    /// `HappyGhast.notifyLeashHolder` (`HappyGhast.java:516-519`), reached by
    /// `Leashable.tickLeash`'s `whenLeashedTo` (`Leashable.java:159`, `:197-199`)
    /// for a quad-leash leashee.
    pub(crate) fn note_leash_holder(&mut self) {
        if let Some(state) = self.happy_ghast_state_mut() {
            state.time = 5;
        }
    }
    /// The published `IS_LEASH_HOLDER` value (`HappyGhast.java:472-474`).
    #[cfg(test)]
    pub(crate) fn happy_ghast_is_leash_holder(&self) -> Option<bool> {
        self.happy_ghast_state().map(|state| state.is_leash_holder)
    }
    /// `HappyGhast.tick`'s holder-clock decay (`HappyGhast.java:407-415`);
    /// true when the published flag flipped and needs a dirty mark.
    pub(crate) fn tick_leash_holder(&mut self) -> bool {
        let Some(state) = self.happy_ghast_state_mut() else {
            return false;
        };
        if state.time > 0 {
            state.time -= 1;
        }
        let now = state.time > 0;
        if now == state.is_leash_holder {
            return false;
        }
        state.is_leash_holder = now;
        true
    }
    pub(crate) fn is_no_ai(&self) -> bool {
        self.mob
            .as_ref()
            .is_some_and(|mob| mob.flags & VANILLA_MOB_FLAG_NO_AI != 0)
    }

    pub(crate) fn is_left_handed(&self) -> bool {
        self.mob
            .as_ref()
            .is_some_and(|mob| mob.flags & VANILLA_MOB_FLAG_LEFT_HANDED != 0)
    }

    #[cfg(test)]
    pub(crate) fn set_no_ai(&mut self, value: bool) {
        if let Some(mob) = &mut self.mob {
            mob.set_no_ai(value);
        }
    }

    pub(crate) fn set_left_handed(&mut self, value: bool) {
        let Some(mob) = &mut self.mob else {
            return;
        };
        mob.set_left_handed(value);
    }

    /// `readAdditionalSaveData` of the Mob base and the per-type variant,
    /// leaving every field nothing here reads in the opaque remainder.
    pub(crate) fn read_save(
        &mut self,
        entity_type_id: i32,
        entries: &[NbtSummaryEntry],
        position: [f64; 3],
        items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) {
        use crate::entity::save::{optional_bool, optional_int, optional_uuid};
        if let Some(mob) = &mut self.mob {
            *mob = crate::entity::save::read_mob_retained_state(entries);
            // AbstractPiglin.java:71-75.
            if matches!(
                entity_type_id,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            ) {
                mob.set_can_pick_up_loot(optional_bool(entries, "CanPickUpLoot").unwrap_or(true));
            }
            if entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID {
                mob.set_can_pick_up_loot(true);
            }
        }
        if let Some(ageable) = &mut self.ageable {
            *ageable = AgeableMobRetainedState::from_saved(
                optional_int(entries, "Age").unwrap_or(0),
                optional_int(entries, "ForcedAge").unwrap_or(0),
                optional_bool(entries, "AgeLocked").unwrap_or(false),
            );
            if entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
            {
                ageable.set_age(ageable.age().max(0));
            }
        }
        if let Some(animal) = &mut self.animal {
            *animal = AnimalRetainedState::from_saved(
                optional_int(entries, "InLove").unwrap_or(0),
                optional_uuid(entries, "LoveCause"),
            );
        }
        if let Some(brain) = entries
            .iter()
            .find(|entry| entry.name == "Brain")
            .and_then(|entry| BrainPacked::decode(&entry.value).ok())
        {
            self.brain = brain;
        }
        if let Some(anger) = &mut self.neutral_anger {
            *anger = crate::entity::ai::mob::target::anger::read_saved_fields(entries, 0);
        }
        self.variants.apply_saved(entries);
        let _ = self
            .variants
            .resolve_registry_holders(|registry, resource_id| {
                items.holder_id(registry, resource_id)
            });
        self.kind
            .read_save(entity_type_id, entries, position, items);
        let body = &mut *self.body;
        if let MobVariantState::Warden(state) = &mut body.kind {
            state.take_cooldowns(&mut body.brain);
        }
        let mut opaque = std::mem::take(&mut self.opaque_fields);
        opaque.clear();
        opaque.extend(
            entries
                .iter()
                .filter(|entry| {
                    !entry
                        .name
                        .with_utf8(|name| {
                            crate::entity::save::is_living_base_save_field(name)
                                || self.owns_save_field(entity_type_id, name)
                        })
                        .unwrap_or(false)
                })
                .cloned(),
        );
        self.opaque_fields = opaque;
    }

    pub(crate) fn neutral_anger(
        &self,
    ) -> Option<crate::entity::ai::mob::target::anger::NeutralAnger> {
        self.body.neutral_anger
    }

    pub(crate) fn neutral_anger_mut(
        &mut self,
    ) -> Option<&mut crate::entity::ai::mob::target::anger::NeutralAnger> {
        self.neutral_anger.as_mut()
    }

    fn owns_save_field(&self, entity_type_id: i32, name: &str) -> bool {
        (self.mob.is_some()
            && matches!(
                name,
                "CanPickUpLoot"
                    | "PersistenceRequired"
                    | "drop_chances"
                    | "LeftHanded"
                    | "DeathLootTable"
                    | "DeathLootTableSeed"
                    | "NoAI"
                    | "leash"
                    | "home_radius"
                    | "home_pos"
            ))
            || (self.ageable.is_some() && matches!(name, "Age" | "ForcedAge" | "AgeLocked"))
            || (self.animal.is_some() && matches!(name, "InLove" | "LoveCause"))
            || (self.neutral_anger.is_some()
                && matches!(name, "anger_end_time" | "angry_at" | "AngerTime"))
            || (name == "Inventory"
                && crate::entity::living_internal_inventory::plain_inventory_capacity(
                    entity_type_id,
                )
                .is_some())
            || name == "Brain"
            || self.variants.owns_save_field(name)
            || self.kind.owns_save_field(name)
    }

    /// `addAdditionalSaveData` of the brain, the shared variants and the
    /// per-type state.
    pub(crate) fn write_variant_save(
        &self,
        entries: &mut Vec<NbtSummaryEntry>,
        entity: &crate::entity::Entity,
        live_brain: Option<BrainPacked>,
        items: crate::entity::item::ItemResolver<'_>,
    ) -> Result<(), crate::entity::save::EntitySaveError> {
        let brain = if let MobVariantState::Warden(state) = &self.kind {
            let mut brain = live_brain.unwrap_or_else(|| self.brain.clone());
            state.write_cooldowns(&mut brain);
            brain.encode()
        } else {
            live_brain.as_ref().unwrap_or(&self.brain).encode()
        };
        entries.push(NbtSummaryEntry {
            name: "Brain".into(),
            value: brain,
        });
        if let Some(anger) = self.neutral_anger {
            entries.push(NbtSummaryEntry {
                name: "anger_end_time".into(),
                value: NbtSummaryValue::Long(anger.end_time),
            });
            if let Some(target) = anger.target {
                entries.push(NbtSummaryEntry {
                    name: "angry_at".into(),
                    value: crate::entity::save::uuid_int_array(target),
                });
            }
        }
        self.variants.write_save_data(entries);
        self.kind.write_save(entries, entity, self, items)
    }

    /// The synched rows the per-type state and shared variants define.
    pub(crate) fn variant_synched_values(
        &self,
        entity_type_id: i32,
        rows: &mut Vec<living::SynchedValue>,
    ) {
        if let Some(ageable) = self.ageable {
            rows.push(super::mob_variant::bool_row(
                VANILLA_AGEABLE_MOB_BABY_DATA_ID,
                ageable.is_baby(),
            ));
            rows.push(super::mob_variant::bool_row(
                bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                ageable.age_locked(),
            ));
        }
        self.kind.synched_values(entity_type_id, rows);
        self.variants.synched_values(rows);
    }

    #[cfg(test)]
    pub(crate) fn apply_saved_variant_fields(&mut self, entries: &[NbtSummaryEntry]) {
        self.variants.apply_saved(entries);
    }

    pub(crate) fn opaque_fields(&self) -> &[NbtSummaryEntry] {
        &self.opaque_fields
    }

    pub(crate) fn opaque_fields_mut(&mut self) -> &mut Vec<NbtSummaryEntry> {
        &mut self.opaque_fields
    }

    /// Rewrites one opaque string field this body already carries.
    ///
    /// It never adds a field, so a type whose construction did not seed the
    /// name keeps exactly the shape its save form expects.
    pub(crate) fn replace_opaque_string(&mut self, name: &str, value: &str) -> bool {
        let Some(entry) = self
            .opaque_fields
            .iter_mut()
            .find(|entry| entry.name == name)
        else {
            return false;
        };
        if !matches!(entry.value, NbtSummaryValue::String(_)) {
            return false;
        }
        entry.value = NbtSummaryValue::string(value);
        true
    }

    /// The `Variant` int this entity carries, which a component read answers.
    pub(crate) fn component_simple_variant(&self) -> Option<i32> {
        self.variants.component_simple_variant()
    }

    pub(crate) fn component_villager_type(&self) -> Option<&str> {
        self.villager_state().map(VillagerState::villager_type)
    }

    /// The profession and level `VillagerData` holds.
    pub(crate) fn villager_profession_and_level(&self) -> Option<(&str, i32)> {
        self.villager_state()
            .map(|state| (state.profession(), state.level()))
    }

    /// `(profession registry id, level, assignProfessionWhenSpawned)`.
    pub(crate) fn villager_brain_state(&self) -> Option<(i32, i32, bool)> {
        self.villager_state().map(|state| {
            (
                state.profession_id(),
                state.level(),
                state.assign_profession_when_spawned(),
            )
        })
    }

    pub(crate) fn villager_work(&self) -> Option<&super::type_variants::VillagerWork> {
        self.villager_state().map(|state| &state.work)
    }

    pub(crate) fn villager_work_mut(&mut self) -> Option<&mut super::type_variants::VillagerWork> {
        self.villager_state_mut().map(|state| &mut state.work)
    }

    pub(crate) fn villager_gossips(&self) -> Option<&[super::type_variants::VillagerGossip]> {
        self.villager_state().map(|state| state.gossips.as_slice())
    }

    pub(crate) fn villager_gossips_mut(
        &mut self,
    ) -> Option<&mut Vec<super::type_variants::VillagerGossip>> {
        self.villager_state_mut().map(|state| &mut state.gossips)
    }

    pub(crate) fn villager_food_level(&self) -> Option<i8> {
        self.villager_state().map(|state| state.food_level)
    }

    pub(crate) fn set_villager_food_level(&mut self, value: i8) {
        if let Some(state) = self.villager_state_mut() {
            state.food_level = value;
        }
    }

    pub(crate) fn registry_variant(&self, registry: &str) -> Option<&str> {
        self.variants.registry_variant(registry)
    }

    pub fn mannequin_profile(&self) -> Option<&NbtSummaryValue> {
        (self.entity_resource_id == "minecraft:mannequin")
            .then(|| {
                self.opaque_fields
                    .iter()
                    .rev()
                    .find(|entry| entry.name == "profile")
                    .map(|entry| &entry.value)
            })
            .flatten()
    }
}
