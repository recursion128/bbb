//! The entity one loot context key carries: a live stored entity read in
//! place, or an owned snapshot for an entity no store holds.
//!
//! Every read answers `None` for a value the caller did not supply, which
//! makes the term that asked a refusal and never a silent false. A null
//! vehicle and a failed `instanceof` are [`LootEntityAnswer::NotApplicable`].

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::data_component_types::{
    vanilla_data_component_type, VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID as AXOLOTL_VARIANT,
    VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID as CAT_COLLAR,
    VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID as CAT_SOUND_VARIANT,
    VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID as CAT_VARIANT,
    VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID as CHICKEN_SOUND_VARIANT,
    VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID as CHICKEN_VARIANT,
    VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID as COW_SOUND_VARIANT,
    VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID as COW_VARIANT,
    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID as CUSTOM_DATA,
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID as CUSTOM_NAME,
    VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID as FOX_VARIANT,
    VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID as FROG_VARIANT,
    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID as HORSE_VARIANT,
    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID as LLAMA_VARIANT,
    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID as MOOSHROOM_VARIANT,
    VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID as PAINTING_VARIANT,
    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID as PARROT_VARIANT,
    VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID as PIG_SOUND_VARIANT,
    VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID as PIG_VARIANT,
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID as POTION_CONTENTS,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID as POTION_DURATION_SCALE,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID as PROFILE,
    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID as RABBIT_VARIANT,
    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID as SALMON_SIZE,
    VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID as SHEEP_COLOR,
    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID as SHULKER_COLOR,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID as TROPICAL_FISH_BASE_COLOR,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID as TROPICAL_FISH_PATTERN_COLOR,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID as TROPICAL_FISH_PATTERN,
    VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID as VILLAGER_VARIANT,
    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID as WOLF_COLLAR,
    VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID as WOLF_SOUND_VARIANT,
    VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID as WOLF_VARIANT,
    VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID as ZOMBIE_NAUTILUS_VARIANT,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::{
    LootEntityAnswer, LootEntityFlags, LootEntityMovement, LootEntityPosition, LootPlayerFacts,
    LootRaiderFlags, LootRuntimeReason, LootTool,
};
use crate::enchantment_facts::EnchantmentSlots;
use crate::entity::mob_death_loot::{
    loot_entity_flags, loot_entity_position, loot_raider_flags, loot_slime_size,
    loot_tool_for_persistent_stack,
};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData};

/// What one of the six entity keys carries.
#[derive(Clone, Copy, Default)]
pub enum LootEntity<'a> {
    /// `null`, which `EntityPredicate.matches` rejects before reading a field.
    #[default]
    Absent,
    /// An entity the caller has but cannot show: only the null test answers.
    Opaque,
    Stored(StoredLootEntity<'a>),
    Facts(&'a LootEntityFacts),
}

/// A live entity. Reads that need its store refuse without one.
#[derive(Clone, Copy)]
pub struct StoredLootEntity<'a> {
    pub entity: &'a Entity,
    pub bodies: Option<EntityBodyReadView<'a>>,
    /// `ServerLevel.getRaidAt(entity.blockPosition())`, the second disjunct of
    /// `Raider.hasRaid`.
    pub raid_at_position: Option<bool>,
}

/// An owned snapshot: a session player, or an entity read before the level is
/// taken mutably. `None` is a value the snapshot does not hold.
#[derive(Clone, Default)]
pub struct LootEntityFacts {
    pub entity_type: Option<&'static str>,
    pub living: Option<bool>,
    pub effects: Option<crate::entity::living::mob_effects::ActiveMobEffects>,
    /// `Sheep.isSheared`; `None` is not a sheep.
    pub sheared: Option<bool>,
    pub position: Option<LootEntityPosition>,
    pub flags: LootEntityFlags,
    pub movement: Option<LootEntityMovement>,
    pub movement_block: Option<[i32; 3]>,
    /// `Entity.mainSupportingBlockPos`, used to compute `movement_block` when
    /// the caller only supplies block lookup through the predicate world.
    pub supporting_block: Option<[i32; 3]>,
    pub on_pos: Option<[i32; 3]>,
    pub tick_count: Option<i32>,
    pub slime_size: Option<LootEntityAnswer<i32>>,
    pub raider_flags: Option<LootEntityAnswer<LootRaiderFlags>>,
    /// `instanceof ServerPlayer`; `None` is not a player.
    pub player: Option<LootPlayerFacts>,
    /// `Some(None)` rides nothing.
    pub vehicle: Option<Option<Box<LootEntityFacts>>>,
    /// A player's uuid, which item environments resolve full player NBT by.
    pub player_uuid: Option<Uuid>,
    /// Actual nonplayer identity; absent in partial facts.
    pub entity_uuid: Option<Uuid>,
    /// The initial owner for inherited teams, known when entity_uuid is present.
    pub tame_owner: Option<Uuid>,
    pub custom_name: Option<Option<NbtSummaryValue>>,
    pub custom_data: Option<Vec<NbtSummaryEntry>>,
    /// Captured getter results; a present null is a known absent component.
    pub components: Vec<(i32, Option<NbtSummaryValue>)>,
    /// Canonical saveWithoutId captured before the store borrow ends.
    pub nbt: Option<Arc<NbtSummaryValue>>,
    /// The resolved `GameProfile` only a `Player` supplies.
    pub profile: Option<LootEntityAnswer<NbtSummaryValue>>,
    /// Existing command slots, including empty slots; absent keys do not exist.
    pub slots: Option<BTreeMap<i32, LootTool>>,
    /// When set, equipment command slots are served directly from `equipment`
    /// without duplicating them into `slots`.
    pub(crate) slots_from_equipment: bool,
    /// `getItemBySlot` per occupied slot; `None` when a held enchantment does
    /// not resolve in this generation.
    pub equipment: Option<BTreeMap<EquipmentSlot, LootTool>>,
    pub enchantment_slots: Option<Arc<Vec<EnchantmentSlots>>>,
}

impl std::fmt::Debug for LootEntity<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Absent => "Absent",
            Self::Opaque => "Opaque",
            Self::Stored(_) => "Stored",
            Self::Facts(_) => "Facts",
        })
    }
}

impl<'a> LootEntity<'a> {
    /// The `entity == null` test.
    #[must_use]
    pub fn present(&self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// The entity when its values can be read.
    pub(crate) fn readable(self) -> Option<Self> {
        matches!(self, Self::Stored(_) | Self::Facts(_)).then_some(self)
    }

    /// `Entity.get(type)` as the component's persistent value.
    pub(crate) fn component(
        self,
        type_id: i32,
    ) -> Result<Option<NbtSummaryValue>, LootRuntimeReason> {
        let unknown = || LootRuntimeReason::Catalog("entity component is not supplied".into());
        match self {
            Self::Stored(stored) => stored.entity.loot_component(type_id),
            Self::Facts(facts) => match type_id {
                CUSTOM_NAME => self.custom_name().ok_or_else(unknown),
                CUSTOM_DATA => facts
                    .custom_data
                    .clone()
                    .map(|data| Some(NbtSummaryValue::Compound(data)))
                    .or_else(|| {
                        let data = saved_base_component(facts, "data")?
                            .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
                        matches!(&data, NbtSummaryValue::Compound(_)).then_some(Some(data))
                    })
                    .ok_or_else(unknown),
                _ => {
                    if let Some((_, value)) = facts.components.iter().find(|(id, _)| *id == type_id)
                    {
                        return Ok(value.clone());
                    }
                    let component = vanilla_data_component_type(type_id).ok_or_else(unknown)?;
                    let carrier = match type_id {
                        PROFILE => Some("mannequin"),
                        POTION_CONTENTS | POTION_DURATION_SCALE => Some("area_effect_cloud"),
                        _ => component
                            .resource_id
                            .split_once('/')
                            .map(|(carrier, _)| carrier.trim_start_matches("minecraft:")),
                    };
                    if let Some(carrier) = carrier {
                        // Uncaptured carrier variants require actual runtime state.
                        let entity_type = facts.entity_type.ok_or_else(unknown)?;
                        if entity_type.ends_with(carrier) {
                            return Err(unknown());
                        }
                    }
                    Ok(None)
                }
            },
            _ => Err(unknown()),
        }
    }

    pub(crate) fn player_uuid(self) -> Option<Uuid> {
        match self {
            Self::Stored(stored) if matches!(stored.entity.data(), EntityData::Player) => {
                Some(stored.entity.uuid())
            }
            Self::Facts(facts) => facts.player_uuid,
            _ => None,
        }
    }

    pub(crate) fn stored_scoreboard_identity(self) -> Option<(Uuid, Option<Uuid>)> {
        match self {
            Self::Stored(stored) if !matches!(stored.entity.data(), EntityData::Player) => Some((
                stored.entity.uuid(),
                stored
                    .entity
                    .tamable_owner()
                    .filter(|(tame, _)| *tame)
                    .and_then(|(_, owner)| owner),
            )),
            Self::Facts(facts) if facts.player_uuid.is_none() => {
                Some((facts.entity_uuid?, facts.tame_owner))
            }
            _ => None,
        }
    }

    /// `Entity.saveWithoutId`.
    pub(crate) fn nbt_data(self) -> Result<NbtSummaryValue, LootRuntimeReason> {
        match self {
            Self::Stored(StoredLootEntity {
                entity,
                bodies: Some(bodies),
                ..
            }) => crate::entity::save::entity_record_tree_without_id(bodies.store(), entity)
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string())),
            Self::Facts(LootEntityFacts { nbt: Some(nbt), .. }) => Ok((**nbt).clone()),
            _ => Err(LootRuntimeReason::Catalog(
                "entity NBT requires the actual save owner".into(),
            )),
        }
    }

    // CopyNameFunction.java:37-42: a known null custom name removes the item name.
    pub(crate) fn custom_name(self) -> Option<Option<NbtSummaryValue>> {
        match self {
            Self::Stored(stored) => Some(
                stored
                    .entity
                    .custom_name()
                    .map(|name| name.component().clone()),
            ),
            Self::Facts(facts) => facts
                .custom_name
                .clone()
                .or_else(|| saved_base_component(facts, "CustomName")),
            _ => None,
        }
    }

    // FillPlayerHead.java:38-43: only a Player supplies a resolved GameProfile.
    pub(crate) fn player_profile(self) -> Option<LootEntityAnswer<NbtSummaryValue>> {
        match self {
            Self::Stored(_) => Some(LootEntityAnswer::NotApplicable),
            Self::Facts(facts) => facts.profile.clone(),
            _ => None,
        }
    }

    pub(crate) fn sheared(self) -> Option<bool> {
        match self {
            Self::Stored(stored) => match stored.entity.data() {
                EntityData::GeneralLiving(data) => data.sheep_is_sheared(),
                _ => None,
            },
            Self::Facts(facts) => facts.sheared,
            _ => None,
        }
    }

    /// `FishingHook.isOpenWaterFishing`, or `None` for anything but a hook.
    pub(crate) fn fishing_hook_open_water(self) -> Option<bool> {
        match self {
            Self::Stored(stored) => match stored.entity.data() {
                EntityData::FishingBobber(data) => Some(data.open_water()),
                _ => None,
            },
            _ => None,
        }
    }

    /// `getBlocksSetOnFire` and the still-living `getHitEntities` of a
    /// `LightningBolt`, or `None` for anything but a stored bolt.
    pub(crate) fn lightning(self) -> Option<(i32, Vec<LootEntity<'a>>)> {
        let Self::Stored(stored) = self else {
            return None;
        };
        let EntityData::LightningBolt(data) = stored.entity.data() else {
            return None;
        };
        let struck = stored.bodies.map_or_else(Vec::new, |bodies| {
            data.hit_entities()
                .iter()
                .filter_map(|id| {
                    bodies.get(*id).or_else(|| {
                        bodies
                            .store()
                            .player(*id)
                            .map(crate::world::PlayerState::body)
                    })
                })
                .filter(|entity| entity.is_alive_with_health())
                .map(|entity| {
                    Self::Stored(StoredLootEntity {
                        entity,
                        bodies: Some(bodies),
                        raid_at_position: None,
                    })
                })
                .collect()
        });
        Some((data.blocks_set_on_fire(), struck))
    }

    /// `entity instanceof LivingEntity`.
    pub(crate) fn living(self) -> Option<bool> {
        match self {
            Self::Stored(stored) => Some(stored.entity.living().is_some()),
            Self::Facts(facts) => facts.living,
            _ => None,
        }
    }

    pub(crate) fn active_effect(
        self,
        resource_id: &str,
    ) -> Option<Option<&'a crate::entity::living::ActiveMobEffect>> {
        match self {
            Self::Stored(stored) => Some(stored.entity.living()?.active_effect(resource_id)),
            Self::Facts(facts) => Some(facts.effects.as_ref()?.get(resource_id)),
            _ => None,
        }
    }

    pub(crate) fn flags(self) -> LootEntityFlags {
        match self {
            Self::Stored(stored) => loot_entity_flags(
                stored.entity,
                stored
                    .entity
                    .is_on_fire_with_publication(stored.items())
                    .ok(),
            ),
            Self::Facts(facts) => facts.flags,
            _ => LootEntityFlags::default(),
        }
    }

    pub(crate) fn entity_type(self) -> Option<&'static str> {
        match self {
            Self::Stored(stored) => Some(stored.entity.data().entity_resource_id()),
            Self::Facts(facts) => facts.entity_type,
            _ => None,
        }
    }

    pub(crate) fn position(self) -> Option<LootEntityPosition> {
        match self {
            Self::Stored(stored) => Some(loot_entity_position(stored.entity)),
            Self::Facts(facts) => facts.position,
            _ => None,
        }
    }

    pub(crate) fn tick_count(self) -> Option<i32> {
        match self {
            Self::Stored(stored) => Some(stored.entity.tick_count()),
            Self::Facts(facts) => facts.tick_count,
            _ => None,
        }
    }

    pub(crate) fn on_pos(self) -> Option<[i32; 3]> {
        match self {
            Self::Stored(stored) => {
                let pos = stored.entity.get_on_pos();
                Some([pos.x, pos.y, pos.z])
            }
            Self::Facts(facts) => facts.on_pos,
            _ => None,
        }
    }

    /// `Entity.getBlockPosBelowThatAffectsMyMovement()`.
    pub(crate) fn block_pos_below_that_affects_movement(self) -> Option<[i32; 3]> {
        match self {
            Self::Stored(stored) => {
                let pos = stored.entity.block_pos_below_that_affects_my_movement();
                Some([pos.x, pos.y, pos.z])
            }
            Self::Facts(facts) => facts.movement_block,
            _ => None,
        }
    }

    /// `Entity.getKnownMovement()` and `Entity.fallDistance`.
    pub(crate) fn movement(self) -> Option<LootEntityMovement> {
        match self {
            Self::Stored(stored) => {
                let movement =
                    crate::entity::location_effects::known_movement(stored.entity, stored.bodies?)?;
                Some(LootEntityMovement {
                    known_movement: [movement.x, movement.y, movement.z],
                    fall_distance: stored.entity.fall_distance(),
                })
            }
            Self::Facts(facts) => facts.movement,
            _ => None,
        }
    }

    pub(crate) fn slot_item<'e>(
        self,
        slot: i32,
        environment: Option<&'e dyn super::LootItemEnvironment>,
    ) -> Result<
        Option<(
            LootTool,
            &'e dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        )>,
        LootRuntimeReason,
    >
    where
        'a: 'e,
    {
        let unavailable = || {
            super::unsupported_field(
                "entity_properties",
                "predicate.slots needs supplied entity slots and their item registry",
            )
        };
        if let Some(uuid) = self.player_uuid() {
            let environment = environment.ok_or_else(unavailable)?;
            return environment
                .player_item_slot(uuid, slot)
                .map(|tool| tool.map(|tool| (tool, environment.item_resolver())));
        }
        let Self::Stored(stored) = self else {
            let Self::Facts(facts) = self else {
                return Err(unavailable());
            };
            if facts.slots.is_none() && !facts.slots_from_equipment {
                return Err(unavailable());
            }
            let resolver = environment.ok_or_else(unavailable)?.item_resolver();
            if let Some(tool) = facts.slots.as_ref().and_then(|slots| slots.get(&slot)) {
                return Ok(Some((tool.clone(), resolver)));
            }
            if facts.slots_from_equipment {
                if let Some(equipment_slot) =
                    crate::entity::item_slots::equipment_slot_for_command_slot(slot)
                {
                    let tool = facts
                        .equipment
                        .as_ref()
                        .and_then(|equipment| equipment.get(&equipment_slot))
                        .cloned()
                        .unwrap_or_else(LootTool::empty);
                    return Ok(Some((tool, resolver)));
                }
            }
            return Ok(None);
        };
        let resolver = stored
            .bodies
            .map(|bodies| bodies.store().item_resolver())
            .or_else(|| environment.map(super::LootItemEnvironment::item_resolver))
            .ok_or_else(unavailable)?;
        let registry = stored
            .bodies
            .map_or(&[][..], |bodies| bodies.store().enchantment_registry());
        crate::entity::item_slots::read(stored.entity, slot)
            .map_err(LootRuntimeReason::Catalog)
            .map(|stack| {
                stack.map(|stack| (loot_tool_for_persistent_stack(&stack, registry), resolver))
            })
    }

    /// `LivingEntity.getItemBySlot(slot)`; an empty slot is `LootTool::empty()`.
    pub(crate) fn equipment_item(self, slot: EquipmentSlot) -> Option<LootTool> {
        match self {
            Self::Stored(stored) => {
                let living = stored.entity.living()?;
                let registry = stored.bodies?.store().enchantment_registry();
                Some(
                    living
                        .item_by_slot(slot)
                        .map(|stack| loot_tool_for_persistent_stack(stack, registry))
                        .unwrap_or_else(LootTool::empty),
                )
            }
            Self::Facts(facts) => Some(
                facts
                    .equipment
                    .as_ref()?
                    .get(&slot)
                    .cloned()
                    .unwrap_or_else(LootTool::empty),
            ),
            _ => None,
        }
    }

    /// `EnchantmentHelper.getEnchantmentLevel(enchantment, entity)`: the
    /// highest level over the stacks in the enchantment's declared slots. Both
    /// call sites gate on `instanceof LivingEntity`, so a non-living entity is
    /// level 0; an enchantment this generation lacks refuses.
    pub(crate) fn enchantment_level(self, enchantment: &str) -> Option<i32> {
        if !self.living()? {
            return Some(0);
        }
        let slots: &[EquipmentSlot] = match self {
            Self::Stored(stored) => stored.bodies?.store().enchantment_slots(enchantment)?,
            Self::Facts(facts) => {
                facts.equipment.as_ref()?;
                &facts
                    .enchantment_slots
                    .as_ref()?
                    .iter()
                    .find(|entry| entry.resource_id == enchantment)?
                    .slots
            }
            _ => return None,
        };
        let mut level = 0;
        for slot in slots {
            level = level.max(self.equipment_item(*slot)?.enchantment_level(enchantment));
        }
        Some(level)
    }

    /// `Entity.getVehicle()`.
    pub(crate) fn vehicle(self) -> Option<LootEntityAnswer<LootEntity<'a>>> {
        match self {
            Self::Stored(stored) => {
                let bodies = stored.bodies?;
                let Some(vehicle) = bodies.store().vehicle(stored.entity.id()) else {
                    return Some(LootEntityAnswer::NotApplicable);
                };
                Some(LootEntityAnswer::Value(Self::Stored(StoredLootEntity {
                    entity: bodies.get(vehicle)?,
                    bodies: Some(bodies),
                    raid_at_position: None,
                })))
            }
            Self::Facts(facts) => Some(match facts.vehicle.as_ref()? {
                None => LootEntityAnswer::NotApplicable,
                Some(vehicle) => LootEntityAnswer::Value(Self::Facts(vehicle)),
            }),
            _ => None,
        }
    }

    /// `Entity.getPassengers()`, including online players held in the store.
    pub(crate) fn passengers(self) -> Option<impl Iterator<Item = Option<LootEntity<'a>>> + 'a> {
        let Self::Stored(stored) = self else {
            return None;
        };
        let bodies = stored.bodies?;
        Some(
            bodies
                .store()
                .passengers(stored.entity.id())
                .iter()
                .map(move |id| {
                    let entity = bodies.get(*id).or_else(|| {
                        bodies
                            .store()
                            .player(*id)
                            .map(crate::world::PlayerState::body)
                    })?;
                    Some(Self::Stored(StoredLootEntity {
                        entity,
                        bodies: Some(bodies),
                        raid_at_position: None,
                    }))
                }),
        )
    }

    /// `Mob.getTarget()`; other entities have no targeted entity.
    pub(crate) fn targeted_entity(self) -> Option<LootEntityAnswer<LootEntity<'a>>> {
        let Self::Stored(stored) = self else {
            return None;
        };
        if !matches!(stored.entity.data(), EntityData::GeneralLiving(data) if data.is_mob()) {
            return Some(LootEntityAnswer::NotApplicable);
        }
        let bodies = stored.bodies?;
        let Some(entity) = bodies.qualified_mob_target(stored.entity.id())? else {
            return Some(LootEntityAnswer::NotApplicable);
        };
        Some(LootEntityAnswer::Value(Self::Stored(StoredLootEntity {
            entity,
            bodies: Some(bodies),
            raid_at_position: None,
        })))
    }

    /// The player reads behind `instanceof ServerPlayer`.
    pub(crate) fn player(self) -> Option<LootPlayerFacts> {
        match self {
            Self::Stored(stored) => stored
                .bodies?
                .store()
                .player(stored.entity.id())
                .map(crate::world::PlayerState::loot_player_facts),
            Self::Facts(facts) => facts.player,
            _ => None,
        }
    }

    /// `Slime.getSize()` behind `instanceof Slime`.
    pub(crate) fn slime_size(self) -> Option<LootEntityAnswer<i32>> {
        match self {
            Self::Stored(stored) => Some(loot_slime_size(stored.entity)),
            Self::Facts(facts) => facts.slime_size,
            _ => None,
        }
    }

    /// `Raider.hasRaid()` and `isCaptain()` behind `instanceof Raider`.
    pub(crate) fn raider_flags(self) -> Option<LootEntityAnswer<LootRaiderFlags>> {
        match self {
            Self::Stored(stored) => Some(loot_raider_flags(
                stored.entity,
                stored.raid_at_position,
                stored.items(),
            )),
            Self::Facts(facts) => facts.raider_flags,
            _ => None,
        }
    }
}

// Complete canonical saves preserve absent base fields through their defaults.
fn saved_base_component(facts: &LootEntityFacts, name: &str) -> Option<Option<NbtSummaryValue>> {
    let NbtSummaryValue::Compound(fields) = facts.nbt.as_deref()? else {
        return None;
    };
    Some(
        fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.value.clone()),
    )
}

impl StoredLootEntity<'_> {
    fn items(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self.bodies
            .map_or(&bbb_registry_codec::UnavailableHolderResolver, |bodies| {
                bodies.store().item_resolver()
            })
    }

    /// The reads a caller keeps past the store borrow, with fire immunity and
    /// captaincy resolved against `items`.
    pub(crate) fn snapshot(
        self,
        items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> LootEntityFacts {
        self.snapshot_inner(items, true)
    }

    /// Lightweight snapshot for an entity that stays live in `bodies` during
    /// predicate evaluation: skips eager NBT tree serialization and slot scans,
    /// letting `entity_for_relations` resolve them on demand when referenced.
    pub(crate) fn snapshot_live(
        self,
        items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> LootEntityFacts {
        self.snapshot_inner(items, false)
    }

    fn snapshot_inner(
        self,
        items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        include_nbt_and_slots: bool,
    ) -> LootEntityFacts {
        let entity = LootEntity::Stored(self);
        let identity = entity.stored_scoreboard_identity();
        let registry = self
            .bodies
            .map_or(&[][..], |bodies| bodies.store().enchantment_registry());
        let kind = self.entity.data().entity_resource_id();
        let (slots, slots_from_equipment) = if include_nbt_and_slots {
            snapshot_slots(self.entity, registry)
        } else {
            (None, false)
        };
        LootEntityFacts {
            entity_type: entity.entity_type(),
            living: entity.living(),
            effects: self.entity.living().map(|living| {
                crate::entity::living::mob_effects::ActiveMobEffects::from_decoded_saved_effects(
                    living.active_effects().cloned(),
                )
            }),
            sheared: entity.sheared(),
            position: entity.position(),
            movement: entity.movement(),
            flags: loot_entity_flags(
                self.entity,
                self.entity.is_on_fire_with_publication(items).ok(),
            ),
            movement_block: entity.block_pos_below_that_affects_movement(),
            on_pos: entity.on_pos(),
            tick_count: entity.tick_count(),
            slime_size: entity.slime_size(),
            raider_flags: Some(loot_raider_flags(self.entity, self.raid_at_position, items)),
            custom_name: entity.custom_name(),
            custom_data: Some(self.entity.custom_data().to_vec()),
            components: entity_component_ids(kind)
                .iter()
                .copied()
                .filter_map(|id| {
                    let value = self.entity.loot_component(id).ok()?;
                    (value.is_some() || (id == SHULKER_COLOR && kind == "minecraft:shulker"))
                        .then_some((id, value))
                })
                .collect(),
            entity_uuid: identity.map(|(uuid, _)| uuid),
            tame_owner: identity.and_then(|(_, owner)| owner),
            nbt: include_nbt_and_slots
                .then(|| entity.nbt_data().ok().map(Arc::new))
                .flatten(),
            profile: entity.player_profile(),
            slots,
            slots_from_equipment,
            equipment: self.entity.living().map(|living| {
                living
                    .equipment()
                    .iter()
                    .map(|(slot, stack)| (slot, loot_tool_for_persistent_stack(stack, registry)))
                    .collect()
            }),
            // `EntityPredicate.java:152` reads `getVehicle()` on the same entity.
            vehicle: match entity.vehicle() {
                Some(LootEntityAnswer::NotApplicable) => Some(None),
                Some(LootEntityAnswer::Value(LootEntity::Stored(vehicle))) => Some(Some(Box::new(
                    vehicle.snapshot_inner(items, include_nbt_and_slots),
                ))),
                _ => None,
            },
            ..LootEntityFacts::default()
        }
    }
}

fn entity_component_ids(kind: &str) -> &'static [i32] {
    match kind {
        "minecraft:sheep" => &[SHEEP_COLOR],
        "minecraft:mooshroom" => &[MOOSHROOM_VARIANT],
        "minecraft:villager" | "minecraft:zombie_villager" => &[VILLAGER_VARIANT],
        "minecraft:rabbit" => &[RABBIT_VARIANT],
        "minecraft:fox" => &[FOX_VARIANT],
        "minecraft:chicken" => &[CHICKEN_VARIANT, CHICKEN_SOUND_VARIANT],
        "minecraft:cow" => &[COW_VARIANT, COW_SOUND_VARIANT],
        "minecraft:pig" => &[PIG_VARIANT, PIG_SOUND_VARIANT],
        "minecraft:horse" => &[HORSE_VARIANT],
        "minecraft:llama" | "minecraft:trader_llama" => &[LLAMA_VARIANT],
        "minecraft:frog" => &[FROG_VARIANT],
        "minecraft:zombie_nautilus" => &[ZOMBIE_NAUTILUS_VARIANT],
        "minecraft:axolotl" => &[AXOLOTL_VARIANT],
        "minecraft:parrot" => &[PARROT_VARIANT],
        "minecraft:shulker" => &[SHULKER_COLOR],
        "minecraft:mannequin" => &[PROFILE],
        "minecraft:painting" => &[PAINTING_VARIANT],
        "minecraft:area_effect_cloud" => &[POTION_CONTENTS, POTION_DURATION_SCALE],
        "minecraft:cat" => &[CAT_VARIANT, CAT_SOUND_VARIANT, CAT_COLLAR],
        "minecraft:wolf" => &[WOLF_VARIANT, WOLF_SOUND_VARIANT, WOLF_COLLAR],
        "minecraft:salmon" => &[SALMON_SIZE],
        "minecraft:tropical_fish" => &[
            TROPICAL_FISH_PATTERN,
            TROPICAL_FISH_BASE_COLOR,
            TROPICAL_FISH_PATTERN_COLOR,
        ],
        _ => &[],
    }
}

fn snapshot_slots(
    entity: &Entity,
    registry: &[EnchantmentSlots],
) -> (Option<BTreeMap<i32, LootTool>>, bool) {
    if let Some(size) = crate::entity::item_slots::container_size(entity) {
        let slots = (0..size)
            .filter_map(|index| {
                let slot = i32::try_from(index).ok()?;
                crate::entity::item_slots::read(entity, slot)
                    .transpose()
                    .map(|stack| {
                        stack.map(|stack| (slot, loot_tool_for_persistent_stack(&stack, registry)))
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .ok();
        return (slots, false);
    }
    if entity.living().is_some() {
        let EntityData::GeneralLiving(data) = entity.data() else {
            return (None, true);
        };
        let has_chest_slot = data.horse_phenotype().is_some_and(|state| {
            matches!(
                state,
                crate::entity::horse_phenotype::HorsePhenotypeState::Chested { .. }
                    | crate::entity::horse_phenotype::HorsePhenotypeState::Llama { .. }
            )
        });
        let inventory_offset = if data.internal_inventory().is_some()
            && entity.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID
        {
            if crate::entity::general_living::is_abstract_horse_entity_type(entity.entity_type_id())
            {
                Some(500)
            } else {
                Some(300)
            }
        } else {
            None
        };
        if !has_chest_slot && inventory_offset.is_none() {
            return (None, true);
        }
        let mut extra = BTreeMap::new();
        if has_chest_slot {
            match crate::entity::item_slots::read(entity, 499) {
                Ok(Some(stack)) => {
                    extra.insert(499, loot_tool_for_persistent_stack(&stack, registry));
                }
                Ok(None) => {}
                Err(_) => return (None, false),
            }
        }
        if let Some(offset) = inventory_offset {
            if let Some(inventory) = data.internal_inventory() {
                for index in 0..inventory.slots().len() {
                    let Ok(slot) = i32::try_from(index).map(|index| offset + index) else {
                        continue;
                    };
                    match crate::entity::item_slots::read(entity, slot) {
                        Ok(Some(stack)) => {
                            extra.insert(slot, loot_tool_for_persistent_stack(&stack, registry));
                        }
                        Ok(None) => {}
                        Err(_) => return (None, false),
                    }
                }
            }
        }
        return (Some(extra), true);
    }
    match crate::entity::item_slots::read(entity, 0) {
        Ok(Some(stack)) => {
            let mut slots = BTreeMap::new();
            slots.insert(0, loot_tool_for_persistent_stack(&stack, registry));
            (Some(slots), false)
        }
        Ok(None) => (Some(BTreeMap::new()), false),
        Err(_) => (None, false),
    }
}
