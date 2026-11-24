use crate::entity::fish::schooling_fish_state_mut;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
    VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::nbt_summary::compound_field as field;
use bbb_protocol::nbt_summary::entry;

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_SOUND_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID, VANILLA_ENTITY_DATA_SERIALIZER_FROG_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_PIG_SOUND_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_PIG_VARIANT_ID, VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_WOLF_SOUND_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_WOLF_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_ZOMBIE_NAUTILUS_VARIANT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_AXOLOTL_VARIANT_DATA_ID, VANILLA_CAT_SOUND_VARIANT_DATA_ID,
    VANILLA_CAT_VARIANT_DATA_ID, VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID,
    VANILLA_CHICKEN_VARIANT_DATA_ID, VANILLA_COW_SOUND_VARIANT_DATA_ID,
    VANILLA_COW_VARIANT_DATA_ID, VANILLA_FROG_VARIANT_DATA_ID, VANILLA_PARROT_VARIANT_DATA_ID,
    VANILLA_PIGLIN_BABY_DATA_ID, VANILLA_PIG_SOUND_VARIANT_DATA_ID, VANILLA_PIG_VARIANT_DATA_ID,
    VANILLA_VILLAGER_DATA_DATA_ID, VANILLA_VILLAGER_DATA_FINALIZED_DATA_ID,
    VANILLA_WOLF_SOUND_VARIANT_DATA_ID, VANILLA_WOLF_VARIANT_DATA_ID, VANILLA_ZOGLIN_BABY_DATA_ID,
    VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID, VANILLA_ZOMBIE_VILLAGER_DATA_DATA_ID,
    VANILLA_ZOMBIE_VILLAGER_DATA_FINALIZED_DATA_ID,
};
use bbb_protocol::packets::{
    EntityDataRegistryHolder, EntityDataRegistryReference, EntityDataValue, EntityDataValueKind,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::mob_variant::MobVariantState;
use crate::entity::ai::{BrainPacked, MemoryModuleType, PackedMemory};
use crate::entity::attributes::{AttributeModifier, AttributeOperation};
use crate::entity::fish::SchoolingFishState;
use crate::entity::living::SynchedValue;
use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryValue {
    registry: &'static str,
    resource_id: String,
    holder_id: i32,
    serializer_id: i32,
    serializer: EntityDataRegistryHolder,
    data_id: u8,
    default_resource_id: &'static str,
}

impl RegistryValue {
    fn set(&mut self, resource_id: &str, holder_id: i32) -> bool {
        if self.resource_id == resource_id && self.holder_id == holder_id {
            return false;
        }
        self.resource_id.clear();
        self.resource_id.push_str(resource_id);
        self.holder_id = holder_id;
        true
    }

    fn synched_value(&self) -> SynchedValue {
        SynchedValue {
            value: EntityDataValue {
                data_id: self.data_id,
                serializer_id: self.serializer_id,
                value: EntityDataValueKind::RegistryId {
                    serializer: self.serializer,
                    value: EntityDataRegistryReference::unresolved(self.holder_id),
                },
            },
            is_default: self.resource_id == self.default_resource_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SimpleVariant {
    value: i32,
    data_id: u8,
}

/// `GossipType` (`GossipType.java:7-11`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GossipKind {
    MajorNegative,
    MinorNegative,
    MinorPositive,
    MajorPositive,
    Trading,
}

impl GossipKind {
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "major_negative" => Some(Self::MajorNegative),
            "minor_negative" => Some(Self::MinorNegative),
            "minor_positive" => Some(Self::MinorPositive),
            "major_positive" => Some(Self::MajorPositive),
            "trading" => Some(Self::Trading),
            _ => None,
        }
    }

    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::MajorNegative => "major_negative",
            Self::MinorNegative => "minor_negative",
            Self::MinorPositive => "minor_positive",
            Self::MajorPositive => "major_positive",
            Self::Trading => "trading",
        }
    }

    pub(crate) const fn debug_name(self) -> &'static str {
        match self {
            Self::MajorNegative => "MAJOR_NEGATIVE",
            Self::MinorNegative => "MINOR_NEGATIVE",
            Self::MinorPositive => "MINOR_POSITIVE",
            Self::MajorPositive => "MAJOR_POSITIVE",
            Self::Trading => "TRADING",
        }
    }

    pub(crate) const fn weight(self) -> i32 {
        match self {
            Self::MajorNegative => -5,
            Self::MinorNegative => -1,
            Self::MinorPositive | Self::Trading => 1,
            Self::MajorPositive => 5,
        }
    }

    pub(crate) const fn decay_per_day(self) -> i32 {
        match self {
            Self::MajorNegative => 10,
            Self::MinorNegative => 20,
            Self::MinorPositive => 1,
            Self::MajorPositive => 0,
            Self::Trading => 2,
        }
    }

    pub(crate) const fn decay_per_transfer(self) -> i32 {
        match self {
            Self::MajorNegative => 10,
            Self::MinorNegative | Self::MajorPositive | Self::Trading => 20,
            Self::MinorPositive => 5,
        }
    }
}

/// One typed `GossipContainer.GossipEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VillagerGossip {
    pub(crate) target: uuid::Uuid,
    pub(crate) kind: GossipKind,
    pub(crate) value: i32,
}

/// `GossipContainer.getReputation` (`GossipContainer.java:108-111`).
pub(crate) fn gossip_reputation(gossips: &[VillagerGossip], player: uuid::Uuid) -> i32 {
    gossips
        .iter()
        .filter(|gossip| gossip.target == player)
        .fold(0i32, |sum, gossip| {
            sum.saturating_add(gossip.value.saturating_mul(gossip.kind.weight()))
        })
}

/// `GossipContainer.add` (`GossipContainer.java:117-124`, `:164-167`, `:204-213`).
pub(crate) fn add_gossip_entry(
    gossips: &mut Vec<VillagerGossip>,
    target: uuid::Uuid,
    kind: GossipKind,
    amount: i32,
    max: i32,
) {
    let existing = gossips
        .iter()
        .position(|gossip| gossip.target == target && gossip.kind == kind);
    let old = existing.map_or(0, |index| gossips[index].value);
    let sum = old.saturating_add(amount);
    let value = if sum > max { max.max(old) } else { sum }.min(max);
    let entry = VillagerGossip {
        target,
        kind,
        value,
    };
    match existing {
        Some(index) => gossips[index] = entry,
        None => gossips.push(entry),
    }
}

fn decode_gossips(entries: &[NbtSummaryEntry]) -> Vec<VillagerGossip> {
    let Some(NbtSummaryValue::List(list)) = field(entries, "Gossips") else {
        return Vec::new();
    };
    let mut result: Vec<VillagerGossip> = Vec::with_capacity(list.len());
    for item in list {
        let NbtSummaryValue::Compound(fields) = item else {
            continue;
        };
        let Some(target) = crate::entity::save::optional_uuid(fields, "Target") else {
            continue;
        };
        let Some(NbtSummaryValue::String(raw_kind)) = field(fields, "Type") else {
            continue;
        };
        let Ok(kind_str) = raw_kind.try_to_string() else {
            continue;
        };
        let Some(kind) = GossipKind::from_id(&kind_str) else {
            continue;
        };
        let Some(value) = crate::entity::save::optional_int(fields, "Value") else {
            continue;
        };
        let gossip = VillagerGossip {
            target,
            kind,
            value,
        };
        if let Some(existing) = result
            .iter_mut()
            .find(|existing| existing.target == target && existing.kind == kind)
        {
            *existing = gossip;
        } else {
            result.push(gossip);
        }
    }
    result
}

fn encode_gossips(gossips: &[VillagerGossip]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        gossips
            .iter()
            .map(|gossip| {
                NbtSummaryValue::Compound(vec![
                    entry("Target", crate::entity::save::uuid_int_array(gossip.target)),
                    entry("Type", NbtSummaryValue::string(gossip.kind.id())),
                    entry("Value", NbtSummaryValue::Int(gossip.value)),
                ])
            })
            .collect(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VillagerState {
    villager_type: String,
    villager_type_id: i32,
    profession: String,
    profession_id: i32,
    level: i32,
    finalized: bool,
    assign_profession_when_spawned: bool,
    data_id: u8,
    finalized_data_id: u8,
    pub(crate) work: VillagerWork,
    pub(crate) gossips: Vec<VillagerGossip>,
    /// `Villager.foodLevel`, saved as the `FoodLevel` byte (`Villager.java:489,515`).
    pub(crate) food_level: i8,
}

/// `Villager`'s trade experience and restock bookkeeping
/// (`Villager.java:116-127`); a zombie villager keeps only `xp`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VillagerWork {
    pub(crate) xp: i32,
    pub(crate) last_restock: i64,
    pub(crate) restocks_today: i32,
    pub(crate) last_gossip_decay: i64,
    pub(crate) update_merchant_timer: i32,
    pub(crate) increase_level_on_update: bool,
    pub(crate) last_traded_player: Option<uuid::Uuid>,
    /// `Villager.lastGossipTime` (`Villager.java:122`), never saved.
    pub(crate) last_gossip_time: i64,
    /// `Villager.lastRestockCheckDay` (`Villager.java:127`), never saved.
    pub(crate) last_restock_check_day: i64,
}

impl VillagerState {
    pub(super) fn villager() -> Self {
        Self::new(
            VANILLA_VILLAGER_DATA_DATA_ID,
            VANILLA_VILLAGER_DATA_FINALIZED_DATA_ID,
        )
    }

    pub(super) fn zombie_villager() -> Self {
        Self::new(
            VANILLA_ZOMBIE_VILLAGER_DATA_DATA_ID,
            VANILLA_ZOMBIE_VILLAGER_DATA_FINALIZED_DATA_ID,
        )
    }

    fn new(data_id: u8, finalized_data_id: u8) -> Self {
        Self {
            villager_type: "minecraft:plains".to_owned(),
            villager_type_id: 2,
            profession: "minecraft:none".to_owned(),
            profession_id: 0,
            level: 1,
            finalized: false,
            assign_profession_when_spawned: false,
            data_id,
            finalized_data_id,
            work: VillagerWork::default(),
            gossips: Vec::new(),
            food_level: 0,
        }
    }

    fn is_villager(&self) -> bool {
        self.data_id == VANILLA_VILLAGER_DATA_DATA_ID
    }

    pub(crate) fn villager_type(&self) -> &str {
        &self.villager_type
    }

    /// The villager profession and level `Villager.updateTrades` reads
    /// (`Villager.java:802-805`).
    pub(crate) fn profession(&self) -> &str {
        &self.profession
    }

    pub(crate) const fn level(&self) -> i32 {
        self.level
    }

    /// The profession's registry id, its bootstrap order.
    pub(crate) const fn profession_id(&self) -> i32 {
        self.profession_id
    }

    pub(crate) const fn assign_profession_when_spawned(&self) -> bool {
        self.assign_profession_when_spawned
    }

    pub(super) fn owns_save_field(name: &str) -> bool {
        matches!(
            name,
            "VillagerData" | "VillagerDataFinalized" | "Xp" | "Gossips"
        )
    }

    pub(super) fn read_save(&mut self, entries: &[NbtSummaryEntry]) {
        // `VillagerData.CODEC` (`VillagerData.java:18-31`): each member falls back to its default.
        let decoded_data = match field(entries, "VillagerData") {
            Some(NbtSummaryValue::Compound(data)) => {
                let registry_name = |name: &str, known: fn(&str) -> Option<i32>| {
                    let NbtSummaryValue::String(raw) = field(data, name)? else {
                        return None;
                    };
                    raw.try_to_string()
                        .ok()
                        .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
                        .filter(|id| known(id).is_some())
                };
                self.villager_type = registry_name("type", villager_type_id)
                    .unwrap_or_else(|| "minecraft:plains".to_owned());
                self.villager_type_id =
                    villager_type_id(&self.villager_type).unwrap_or(self.villager_type_id);
                self.profession = registry_name("profession", villager_profession_id)
                    .unwrap_or_else(|| "minecraft:none".to_owned());
                self.profession_id =
                    villager_profession_id(&self.profession).unwrap_or(self.profession_id);
                self.level = crate::entity::save::optional_int(data, "level")
                    .unwrap_or(1)
                    .max(1);
                true
            }
            _ => false,
        };
        if self.is_villager() {
            if let Some(value) =
                crate::entity::save::optional_bool(entries, "AssignProfessionWhenSpawned")
            {
                self.assign_profession_when_spawned = value;
            }
        }
        // Villager.java:503-510; ZombieVillager.java:107-114: decoded data finalizes both types.
        if let Some(value) = crate::entity::save::optional_bool(entries, "VillagerDataFinalized") {
            self.finalized = value;
        }
        self.finalized |= decoded_data;
        let int = |name| crate::entity::save::optional_int(entries, name);
        self.work.xp = int("Xp").unwrap_or(0);
        self.gossips = decode_gossips(entries);
        if self.is_villager() {
            self.work.last_restock =
                crate::entity::save::optional_long(entries, "LastRestock").unwrap_or(0);
            self.work.restocks_today = int("RestocksToday").unwrap_or(0);
            self.work.last_gossip_decay =
                crate::entity::save::optional_long(entries, "LastGossipDecay").unwrap_or(0);
            self.food_level = field(entries, "FoodLevel")
                .and_then(crate::entity::save::numeric_tag_byte_value)
                .unwrap_or(0);
        }
    }

    fn resolve_registry_holders(&mut self) -> bool {
        let (Some(villager_type_id), Some(profession_id)) = (
            villager_type_id(&self.villager_type),
            villager_profession_id(&self.profession),
        ) else {
            return false;
        };
        self.villager_type_id = villager_type_id;
        self.profession_id = profession_id;
        true
    }

    pub(super) fn write_save(&self, entries: &mut Vec<NbtSummaryEntry>) {
        entries.extend([
            entry(
                "VillagerData",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::string(self.villager_type.clone())),
                    entry(
                        "profession",
                        NbtSummaryValue::string(self.profession.clone()),
                    ),
                    entry("level", NbtSummaryValue::Int(self.level)),
                ]),
            ),
            entry(
                "VillagerDataFinalized",
                NbtSummaryValue::Byte(i8::from(self.finalized)),
            ),
        ]);
        entries.push(entry("Xp", NbtSummaryValue::Int(self.work.xp)));
        entries.push(entry("Gossips", encode_gossips(&self.gossips)));
        if self.is_villager() {
            entries.extend([
                entry("FoodLevel", NbtSummaryValue::Byte(self.food_level)),
                entry("LastRestock", NbtSummaryValue::Long(self.work.last_restock)),
                entry(
                    "RestocksToday",
                    NbtSummaryValue::Int(self.work.restocks_today),
                ),
                entry(
                    "LastGossipDecay",
                    NbtSummaryValue::Long(self.work.last_gossip_decay),
                ),
            ]);
        }
        // `Villager.java:495-496`: false is represented by an absent field.
        if self.is_villager() && self.assign_profession_when_spawned {
            entries.push(entry(
                "AssignProfessionWhenSpawned",
                NbtSummaryValue::Byte(1),
            ));
        }
    }

    pub(super) fn synched_values(&self, rows: &mut Vec<SynchedValue>) {
        rows.extend([
            SynchedValue {
                value: EntityDataValue {
                    data_id: self.data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID,
                    value: EntityDataValueKind::VillagerData {
                        villager_type: EntityDataRegistryReference::unresolved(
                            self.villager_type_id,
                        ),
                        profession: EntityDataRegistryReference::unresolved(self.profession_id),
                        level: self.level,
                    },
                },
                is_default: self.villager_type_id == 2
                    && self.profession_id == 0
                    && self.level == 1,
            },
            SynchedValue {
                value: EntityDataValue {
                    data_id: self.finalized_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(self.finalized),
                },
                is_default: !self.finalized,
            },
        ]);
    }
}

/// The registry-selected and `Variant`-int variants a few unrelated families
/// share: the synched value `defineSynchedData` seeds with the registry
/// default and `finalizeSpawn` replaces, and the field the save writes.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TypeVariants {
    simple_variant: Option<SimpleVariant>,
    registry_variant: Option<RegistryValue>,
    sound_variant: Option<RegistryValue>,
}

impl TypeVariants {
    pub(super) fn new(entity_type_id: i32) -> Self {
        let simple_variant = match entity_type_id {
            VANILLA_ENTITY_TYPE_AXOLOTL_ID => Some(SimpleVariant {
                value: 0,
                data_id: VANILLA_AXOLOTL_VARIANT_DATA_ID,
            }),
            VANILLA_ENTITY_TYPE_PARROT_ID => Some(SimpleVariant {
                value: 0,
                data_id: VANILLA_PARROT_VARIANT_DATA_ID,
            }),
            _ => None,
        };
        Self {
            simple_variant,
            registry_variant: registry_value(entity_type_id, false),
            sound_variant: registry_value(entity_type_id, true),
        }
    }

    pub(super) fn apply_saved(&mut self, entries: &[NbtSummaryEntry]) {
        if let Some(simple) = &mut self.simple_variant {
            if let Some(value) = crate::entity::save::optional_int(entries, "Variant") {
                simple.value = value;
            }
        }
        if let Some(variant) = &mut self.registry_variant {
            if let Some(NbtSummaryValue::String(value)) = field(entries, "variant") {
                if let Ok(value) = value.try_to_string() {
                    variant.resource_id =
                        bbb_protocol::normalize_resource_location(value.clone()).unwrap_or(value);
                }
            }
        }
        if let Some(sound) = &mut self.sound_variant {
            if let Some(NbtSummaryValue::String(value)) = field(entries, "sound_variant") {
                if let Ok(value) = value.try_to_string() {
                    sound.resource_id =
                        bbb_protocol::normalize_resource_location(value.clone()).unwrap_or(value);
                }
            }
        }
    }

    pub(super) fn owns_save_field(&self, name: &str) -> bool {
        match name {
            "Variant" => self.simple_variant.is_some(),
            "variant" => self.registry_variant.is_some(),
            "sound_variant" => self.sound_variant.is_some(),
            _ => false,
        }
    }

    pub(super) fn resolve_registry_holders(
        &mut self,
        mut resolve: impl FnMut(&str, &str) -> Option<i32>,
    ) -> bool {
        for value in [&mut self.registry_variant, &mut self.sound_variant]
            .into_iter()
            .flatten()
        {
            // VariantUtils.java:30-32: an id the registry lacks leaves the default.
            match resolve(value.registry, &value.resource_id) {
                Some(holder_id) => value.holder_id = holder_id,
                None => {
                    let Some(holder_id) = resolve(value.registry, value.default_resource_id) else {
                        return false;
                    };
                    value.resource_id = value.default_resource_id.to_owned();
                    value.holder_id = holder_id;
                }
            }
        }
        true
    }

    pub(super) fn write_save_data(&self, entries: &mut Vec<NbtSummaryEntry>) {
        if let Some(simple) = &self.simple_variant {
            entries.push(entry("Variant", NbtSummaryValue::Int(simple.value)));
        }
        if let Some(variant) = &self.registry_variant {
            entries.push(entry(
                "variant",
                NbtSummaryValue::string(variant.resource_id.clone()),
            ));
        }
        if let Some(sound) = &self.sound_variant {
            entries.push(entry(
                "sound_variant",
                NbtSummaryValue::string(sound.resource_id.clone()),
            ));
        }
    }

    pub(super) fn synched_values(&self, rows: &mut Vec<SynchedValue>) {
        if let Some(simple) = &self.simple_variant {
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: simple.data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(simple.value),
                },
                is_default: simple.value == 0,
            });
        }
        if let Some(variant) = &self.registry_variant {
            rows.push(variant.synched_value());
        }
        if let Some(sound) = &self.sound_variant {
            rows.push(sound.synched_value());
        }
    }

    pub(super) fn component_simple_variant(&self) -> Option<i32> {
        self.simple_variant.as_ref().map(|variant| variant.value)
    }

    pub(super) fn registry_variant(&self, registry: &str) -> Option<&str> {
        self.registry_variant
            .as_ref()
            .filter(|value| value.registry == registry)
            .map(|value| value.resource_id.as_str())
    }
}

pub fn set_simple_variant(entity: &mut Entity, value: i32) -> bool {
    let changed_data_id = {
        let Some(data) = general_living_mut(entity) else {
            return false;
        };
        let Some(variant) = &mut data.variants.simple_variant else {
            return false;
        };
        (variant.value != value).then(|| {
            variant.value = value;
            variant.data_id
        })
    };
    if let Some(data_id) = changed_data_id {
        entity.mark_synched_dirty(data_id);
    }
    true
}

pub fn set_registry_value(
    entity: &mut Entity,
    registry: &str,
    resource_id: &str,
    holder_id: i32,
) -> bool {
    let Some(data) = general_living_mut(entity) else {
        return false;
    };
    let variants = &mut data.variants;
    let value = [&mut variants.registry_variant, &mut variants.sound_variant]
        .into_iter()
        .find_map(|value| value.as_mut().filter(|value| value.registry == registry));
    let Some(value) = value else {
        return false;
    };
    let data_id = value.data_id;
    if value.set(resource_id, holder_id) {
        entity.mark_synched_dirty(data_id);
    }
    true
}

/// `WolfSoundVariants.pickRandomSoundVariant` (`WolfSoundVariants.java:41-43`):
/// chooses from the actual published registry.
pub(crate) fn pick_random_wolf_sound_variant(
    random: &crate::random::GameRandom,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Option<(String, i32)> {
    let count = i32::try_from(resolver.wolf_sound_variant_count()?).ok()?;
    if count == 0 {
        return None;
    }
    let holder_id = random.next_int(count);
    let resource_id = resolver.holder_resource_id("minecraft:wolf_sound_variant", holder_id)?;
    Some((resource_id, holder_id))
}

impl Entity {
    pub(crate) fn set_bucket_hunting_cooldown(&mut self, time_to_live: Option<i64>) {
        let Some(data) = general_living_mut(self) else {
            return;
        };
        data.brain
            .memories
            .retain(|memory| memory.memory_type != MemoryModuleType::HasHuntingCooldown);
        if let Some(time_to_live) = time_to_live {
            data.brain.memories.push(PackedMemory {
                memory_type: MemoryModuleType::HasHuntingCooldown,
                value: crate::entity::ai::MemoryValue::Bool(true),
                time_to_live: Some(time_to_live),
            });
        }
    }
}

impl super::GeneralLivingData {
    pub(crate) fn retained_brain_mut(&mut self) -> &mut BrainPacked {
        &mut self.brain
    }

    pub(super) fn set_brain_memory(&mut self, memory: PackedMemory) {
        self.brain
            .memories
            .retain(|current| current.memory_type != memory.memory_type);
        self.brain.memories.push(memory);
    }
}

pub fn set_brain_memory(entity: &mut Entity, memory: PackedMemory) -> bool {
    let Some(data) = general_living_mut(entity) else {
        return false;
    };
    data.set_brain_memory(memory);
    true
}

pub fn brain_memory(entity: &Entity, memory_type: MemoryModuleType) -> Option<&PackedMemory> {
    general_living(entity)?
        .brain
        .memories
        .iter()
        .find(|memory| memory.memory_type == memory_type)
}

/// The `Brain` compound the entity carries, which is what a Mob's live brain
/// is built from and what it replaces on save.
pub fn brain_packed(entity: &Entity) -> Option<&BrainPacked> {
    Some(&general_living(entity)?.brain)
}

pub(super) fn set_villager_assign_profession_when_spawned(entity: &mut Entity) -> bool {
    let Some(villager) = general_living_mut(entity).and_then(|data| data.villager_state_mut())
    else {
        return false;
    };
    villager.assign_profession_when_spawned = true;
    true
}

/// `Villager.customServerAiStep` (`Villager.java:249-251`): the flag lives for
/// one brain tick.
pub(crate) fn clear_villager_assign_profession_when_spawned(entity: &mut Entity) {
    if let Some(villager) = general_living_mut(entity).and_then(|data| data.villager_state_mut()) {
        villager.assign_profession_when_spawned = false;
    }
}

/// `Villager.applyImplicitComponent` replaces only the type in its villager data.
pub(crate) fn set_component_villager_type(entity: &mut Entity, villager_type: &str) -> bool {
    let holder_id = villager_type_id(villager_type).expect("a decoded villager type");
    let Some(villager) = general_living_mut(entity).and_then(|data| data.villager_state_mut())
    else {
        return false;
    };
    if villager.villager_type != villager_type {
        villager.villager_type.clear();
        villager.villager_type.push_str(villager_type);
        villager.villager_type_id = holder_id;
        let data_id = villager.data_id;
        entity.mark_synched_dirty(data_id);
    }
    true
}

pub fn set_villager_type(entity: &mut Entity, villager_type: &str) -> bool {
    if villager_type_id(villager_type).is_none() {
        return false;
    }
    let Some(villager) = general_living_mut(entity).and_then(|data| data.villager_state_mut())
    else {
        return false;
    };
    if villager.finalized {
        return true;
    }
    villager.finalized = true;
    let finalized_id = villager.finalized_data_id;
    entity.mark_synched_dirty(finalized_id);
    set_component_villager_type(entity, villager_type)
}

/// The `sound_variant` this body carries, which `Pig.getSoundSet` reads
/// (`Pig.java:256-266`).
pub(crate) fn sound_variant_resource_id(entity: &Entity) -> Option<&str> {
    Some(
        general_living(entity)?
            .variants
            .sound_variant
            .as_ref()?
            .resource_id
            .as_str(),
    )
}

/// `setVillagerData(source.getVillagerData())` and `setVillagerXp` across a
/// villager conversion; the target keeps its own finalized flag.
pub(crate) fn copy_villager_data(target: &mut Entity, source: &Entity) -> bool {
    let Some(data) = general_living(source).and_then(|data| data.villager_state()) else {
        return false;
    };
    let (villager_type, villager_type_id, profession, profession_id, level, xp) = (
        data.villager_type.clone(),
        data.villager_type_id,
        data.profession.clone(),
        data.profession_id,
        data.level,
        data.work.xp,
    );
    let Some(villager) = general_living_mut(target).and_then(|data| data.villager_state_mut())
    else {
        return false;
    };
    villager.villager_type = villager_type;
    villager.villager_type_id = villager_type_id;
    villager.profession = profession;
    villager.profession_id = profession_id;
    villager.level = level;
    villager.work.xp = xp;
    let data_id = villager.data_id;
    target.mark_synched_dirty(data_id);
    true
}

/// `Villager.setVillagerData(data.withProfession(..))` (`Villager.java:553-560`):
/// a changed profession drops the offers so `getOffers` regenerates them.
pub(crate) fn set_villager_profession(entity: &mut Entity, profession: &str) -> bool {
    let Some(profession_id) = villager_profession_id(profession) else {
        return false;
    };
    let Some(data) = general_living_mut(entity) else {
        return false;
    };
    let Some(villager) = data.villager_state_mut() else {
        return false;
    };
    if villager.profession == profession {
        return true;
    }
    villager.profession.clear();
    villager.profession.push_str(profession);
    villager.profession_id = profession_id;
    let data_id = villager.data_id;
    if let Some(merchant) = data.abstract_villager_retained_mut() {
        merchant.clear_offers();
    }
    entity.mark_synched_dirty(data_id);
    true
}

/// `Villager.setVillagerData(data.withLevel(level))`.
pub(crate) fn set_villager_level(entity: &mut Entity, level: i32) -> bool {
    let Some(villager) = general_living_mut(entity).and_then(|data| data.villager_state_mut())
    else {
        return false;
    };
    villager.level = level.max(1);
    let data_id = villager.data_id;
    entity.mark_synched_dirty(data_id);
    true
}

pub fn villager_finalized(entity: &Entity) -> Option<bool> {
    Some(general_living(entity)?.villager_state()?.finalized)
}

fn set_baby_flag(entity: &mut Entity, baby: bool, data_id: u8) -> Option<()> {
    let current = match &mut general_living_mut(entity)?.kind {
        MobVariantState::Piglin { baby, .. } | MobVariantState::Zoglin { baby } => baby,
        _ => return None,
    };
    if *current != baby {
        *current = baby;
        entity.mark_synched_dirty(data_id);
        crate::entity::living::refresh_dimensions(entity);
    }
    Some(())
}

pub fn set_piglin_baby(entity: &mut Entity, baby: bool) -> bool {
    if entity.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID
        || set_baby_flag(entity, baby, VANILLA_PIGLIN_BABY_DATA_ID).is_none()
    {
        return false;
    }
    // Piglin.java:242-251: even a same-value set replaces the speed modifier.
    let speed = entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .expect("Piglin declares movement speed");
    speed.remove_modifier("minecraft:baby");
    if baby {
        speed
            .add_transient_modifier(AttributeModifier {
                id: "minecraft:baby".into(),
                amount: f64::from(0.2_f32),
                operation: AttributeOperation::AddMultipliedBase,
            })
            .expect("the baby modifier was removed");
    }
    true
}

pub fn set_zoglin_baby(entity: &mut Entity, baby: bool) -> bool {
    if entity.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOGLIN_ID
        || set_baby_flag(entity, baby, VANILLA_ZOGLIN_BABY_DATA_ID).is_none()
    {
        return false;
    }
    if baby {
        // Zoglin.java:235-240: becoming adult does not restore attack damage.
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
            .expect("Zoglin declares attack damage")
            .set_base_value(0.5);
    }
    true
}

pub(crate) fn finish_baby_load(entity: &mut Entity) {
    let Some(baby) = general_living(entity).and_then(|data| data.piglin_or_zoglin_baby()) else {
        return;
    };
    let _ = set_piglin_baby(entity, baby) || set_zoglin_baby(entity, baby);
}

pub fn set_tropical_fish_schooling(entity: &mut Entity, enabled: bool) -> bool {
    match general_living_mut(entity).and_then(|data| data.fish_state_mut()) {
        Some(fish) if entity_is_tropical(fish) => {
            fish.tropical_school = enabled;
            true
        }
        _ => false,
    }
}

fn entity_is_tropical(fish: &super::FishState) -> bool {
    matches!(
        fish.leaf,
        Some(crate::entity::fish::FishLeafState::Tropical(_))
    )
}

pub fn tropical_fish_is_school(entity: &Entity) -> Option<bool> {
    let fish = general_living(entity)?.fish_state()?;
    entity_is_tropical(fish).then_some(fish.tropical_school)
}

pub fn schooling_fish_state(entity: &Entity) -> Option<SchoolingFishState> {
    general_living(entity)?.fish_state()?.school
}

pub(crate) fn set_schooling_fish_leader(entity: &mut Entity, leader: Option<uuid::Uuid>) -> bool {
    let Some(state) = schooling_fish_state_mut(entity) else {
        return false;
    };
    state.set_leader_uuid(leader);
    true
}

pub(crate) fn increment_schooling_fish_size(entity: &mut Entity) -> bool {
    let Some(state) = schooling_fish_state_mut(entity) else {
        return false;
    };
    state.increment_school_size();
    true
}

pub fn configure_fox_target_goals(entity: &mut Entity) -> bool {
    let Some(MobVariantState::Fox(fox, goals)) =
        general_living_mut(entity).map(|data| &mut data.kind)
    else {
        return false;
    };
    // Fox.java:359-369; WrappedGoal.java:89-100: repeated adds preserve the first priorities.
    goals.get_or_insert(fox.variant());
    true
}

pub(crate) fn fox_target_goals(entity: &Entity) -> Option<crate::entity::fox::FoxVariant> {
    match general_living(entity)?.kind {
        MobVariantState::Fox(_, goals) => goals,
        _ => None,
    }
}

pub fn set_spawn_experience_reward(entity: &mut Entity, reward: i32) -> bool {
    let Some(MobVariantState::Hoglin {
        experience_reward, ..
    }) = general_living_mut(entity).map(|data| &mut data.kind)
    else {
        return false;
    };
    *experience_reward = reward;
    true
}

pub fn resolve_registry_holders(
    entity: &mut Entity,
    resolve: impl FnMut(&str, &str) -> Option<i32>,
) -> bool {
    let Some(data) = general_living_mut(entity) else {
        return true;
    };
    data.variants.resolve_registry_holders(resolve)
        && data
            .villager_state_mut()
            .is_none_or(VillagerState::resolve_registry_holders)
}

/// Partner == parent, so a `this`/`partner` mix copies the parent.
#[derive(Debug, Clone)]
pub(crate) struct ParentVariantCopy {
    simple: Option<i32>,
    registry: Option<(&'static str, String, i32)>,
}

pub(crate) fn parent_variant_copy(entity: &Entity) -> ParentVariantCopy {
    let Some(data) = general_living(entity) else {
        return ParentVariantCopy {
            simple: None,
            registry: None,
        };
    };
    ParentVariantCopy {
        simple: data.variants.component_simple_variant(),
        registry: data.variants.registry_variant.as_ref().map(|variant| {
            (
                variant.registry,
                variant.resource_id.clone(),
                variant.holder_id,
            )
        }),
    }
}

pub(crate) fn apply_parent_variant_copy(entity: &mut Entity, copy: &ParentVariantCopy) {
    if let Some(value) = copy.simple {
        let _ = set_simple_variant(entity, value);
    }
    if let Some((registry, resource_id, holder_id)) = &copy.registry {
        let _ = set_registry_value(entity, registry, resource_id, *holder_id);
    }
}

fn general_living(entity: &Entity) -> Option<&super::GeneralLivingData> {
    match entity.data() {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

fn general_living_mut(entity: &mut Entity) -> Option<&mut super::GeneralLivingData> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

fn registry_value(entity_type_id: i32, sound: bool) -> Option<RegistryValue> {
    let (registry, default, holder_id, serializer_id, serializer, data_id) =
        match (entity_type_id, sound) {
            (VANILLA_ENTITY_TYPE_CAT_ID, false) => (
                "minecraft:cat_variant",
                "minecraft:black",
                1,
                VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID,
                EntityDataRegistryHolder::CatVariant,
                VANILLA_CAT_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_CAT_ID, true) => (
                "minecraft:cat_sound_variant",
                "minecraft:classic",
                0,
                VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
                EntityDataRegistryHolder::CatSoundVariant,
                VANILLA_CAT_SOUND_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_CHICKEN_ID, false) => (
                "minecraft:chicken_variant",
                "minecraft:temperate",
                1,
                VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_VARIANT_ID,
                EntityDataRegistryHolder::ChickenVariant,
                VANILLA_CHICKEN_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_CHICKEN_ID, true) => (
                "minecraft:chicken_sound_variant",
                "minecraft:classic",
                0,
                VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_SOUND_VARIANT_ID,
                EntityDataRegistryHolder::ChickenSoundVariant,
                VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_COW_ID, false) => (
                "minecraft:cow_variant",
                "minecraft:temperate",
                1,
                VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID,
                EntityDataRegistryHolder::CowVariant,
                VANILLA_COW_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_COW_ID, true) => (
                "minecraft:cow_sound_variant",
                "minecraft:classic",
                0,
                VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID,
                EntityDataRegistryHolder::CowSoundVariant,
                VANILLA_COW_SOUND_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_FROG_ID, false) => (
                "minecraft:frog_variant",
                "minecraft:temperate",
                1,
                VANILLA_ENTITY_DATA_SERIALIZER_FROG_VARIANT_ID,
                EntityDataRegistryHolder::FrogVariant,
                VANILLA_FROG_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_PIG_ID, false) => (
                "minecraft:pig_variant",
                "minecraft:temperate",
                1,
                VANILLA_ENTITY_DATA_SERIALIZER_PIG_VARIANT_ID,
                EntityDataRegistryHolder::PigVariant,
                VANILLA_PIG_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_PIG_ID, true) => (
                "minecraft:pig_sound_variant",
                "minecraft:classic",
                0,
                VANILLA_ENTITY_DATA_SERIALIZER_PIG_SOUND_VARIANT_ID,
                EntityDataRegistryHolder::PigSoundVariant,
                VANILLA_PIG_SOUND_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_WOLF_ID, false) => (
                "minecraft:wolf_variant",
                "minecraft:pale",
                3,
                VANILLA_ENTITY_DATA_SERIALIZER_WOLF_VARIANT_ID,
                EntityDataRegistryHolder::WolfVariant,
                VANILLA_WOLF_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_WOLF_ID, true) => (
                "minecraft:wolf_sound_variant",
                "minecraft:classic",
                2,
                VANILLA_ENTITY_DATA_SERIALIZER_WOLF_SOUND_VARIANT_ID,
                EntityDataRegistryHolder::WolfSoundVariant,
                VANILLA_WOLF_SOUND_VARIANT_DATA_ID,
            ),
            (VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, false) => (
                "minecraft:zombie_nautilus_variant",
                "minecraft:temperate",
                0,
                VANILLA_ENTITY_DATA_SERIALIZER_ZOMBIE_NAUTILUS_VARIANT_ID,
                EntityDataRegistryHolder::ZombieNautilusVariant,
                VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
            ),
            _ => return None,
        };
    Some(RegistryValue {
        registry,
        resource_id: default.to_owned(),
        holder_id,
        serializer_id,
        serializer,
        data_id,
        default_resource_id: default,
    })
}

// Fixed-registry IDs follow bootstrap order (`VillagerType.java:69-76`;
// `VillagerProfession.java:123-296`).
fn villager_type_id(resource_id: &str) -> Option<i32> {
    Some(match resource_id {
        "minecraft:desert" => 0,
        "minecraft:jungle" => 1,
        "minecraft:plains" => 2,
        "minecraft:savanna" => 3,
        "minecraft:snow" => 4,
        "minecraft:swamp" => 5,
        "minecraft:taiga" => 6,
        _ => return None,
    })
}

/// `BuiltInRegistries.VILLAGER_PROFESSION` in registry order.
pub(super) const VILLAGER_PROFESSIONS: [&str; 15] = [
    "minecraft:none",
    "minecraft:armorer",
    "minecraft:butcher",
    "minecraft:cartographer",
    "minecraft:cleric",
    "minecraft:farmer",
    "minecraft:fisherman",
    "minecraft:fletcher",
    "minecraft:leatherworker",
    "minecraft:librarian",
    "minecraft:mason",
    "minecraft:nitwit",
    "minecraft:shepherd",
    "minecraft:toolsmith",
    "minecraft:weaponsmith",
];

fn villager_profession_id(resource_id: &str) -> Option<i32> {
    Some(match resource_id {
        "minecraft:none" => 0,
        "minecraft:armorer" => 1,
        "minecraft:butcher" => 2,
        "minecraft:cartographer" => 3,
        "minecraft:cleric" => 4,
        "minecraft:farmer" => 5,
        "minecraft:fisherman" => 6,
        "minecraft:fletcher" => 7,
        "minecraft:leatherworker" => 8,
        "minecraft:librarian" => 9,
        "minecraft:mason" => 10,
        "minecraft:nitwit" => 11,
        "minecraft:shepherd" => 12,
        "minecraft:toolsmith" => 13,
        "minecraft:weaponsmith" => 14,
        _ => return None,
    })
}

pub fn set_cat_spawn_persistence(entity: &mut Entity) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_CAT_ID {
        return false;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(mob) = data.mob.as_mut() else {
        return false;
    };
    mob.set_persistence_required(true);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombie_villager_fixed_registry_holder_ids() {
        for (resource_id, expected) in [
            ("minecraft:desert", 0),
            ("minecraft:jungle", 1),
            ("minecraft:plains", 2),
            ("minecraft:savanna", 3),
            ("minecraft:snow", 4),
            ("minecraft:swamp", 5),
            ("minecraft:taiga", 6),
        ] {
            assert_eq!(villager_type_id(resource_id), Some(expected));
        }
        for (expected, resource_id) in [
            "none",
            "armorer",
            "butcher",
            "cartographer",
            "cleric",
            "farmer",
            "fisherman",
            "fletcher",
            "leatherworker",
            "librarian",
            "mason",
            "nitwit",
            "shepherd",
            "toolsmith",
            "weaponsmith",
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                villager_profession_id(&format!("minecraft:{resource_id}")),
                Some(expected as i32)
            );
        }
    }
}
