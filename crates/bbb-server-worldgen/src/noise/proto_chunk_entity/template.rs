//! Writer-canonical ProtoChunk entities loaded from structure-template NBT.

use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};

use bbb_server_types::save::{SaveError, SaveResult};

pub use super::entry;

pub mod jigsaw;
#[cfg(test)]
mod test_nbt;
use jigsaw::JigsawExtras;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateMobKind {
    Villager,
    ZombieVillager,
    Cat,
    Cow,
    Pig,
    Sheep,
    Horse,
    Camel,
    IronGolem,
    ArmorStand,
    Allay,
    Hoglin,
    Piglin,
    PiglinBrute,
}

impl TemplateMobKind {
    pub fn from_resource_id(id: &str) -> Option<Self> {
        Some(match id {
            "minecraft:villager" => Self::Villager,
            "minecraft:zombie_villager" => Self::ZombieVillager,
            "minecraft:cat" => Self::Cat,
            "minecraft:cow" => Self::Cow,
            "minecraft:pig" => Self::Pig,
            "minecraft:sheep" => Self::Sheep,
            "minecraft:horse" => Self::Horse,
            "minecraft:camel" => Self::Camel,
            "minecraft:iron_golem" => Self::IronGolem,
            "minecraft:armor_stand" => Self::ArmorStand,
            "minecraft:allay" => Self::Allay,
            "minecraft:hoglin" => Self::Hoglin,
            "minecraft:piglin" => Self::Piglin,
            "minecraft:piglin_brute" => Self::PiglinBrute,
            _ => return None,
        })
    }

    pub const fn resource_id(self) -> &'static str {
        match self {
            Self::Villager => "minecraft:villager",
            Self::ZombieVillager => "minecraft:zombie_villager",
            Self::Cat => "minecraft:cat",
            Self::Cow => "minecraft:cow",
            Self::Pig => "minecraft:pig",
            Self::Sheep => "minecraft:sheep",
            Self::Horse => "minecraft:horse",
            Self::Camel => "minecraft:camel",
            Self::IronGolem => "minecraft:iron_golem",
            Self::ArmorStand => "minecraft:armor_stand",
            Self::Allay => "minecraft:allay",
            Self::Hoglin => "minecraft:hoglin",
            Self::Piglin => "minecraft:piglin",
            Self::PiglinBrute => "minecraft:piglin_brute",
        }
    }

    pub const fn max_health(self) -> f32 {
        match self {
            Self::Villager | Self::ZombieVillager | Self::ArmorStand | Self::Allay => 20.0,
            Self::Cat | Self::Cow | Self::Pig => 10.0,
            Self::Sheep => 8.0,
            Self::Horse => 53.0,
            Self::Camel => 32.0,
            Self::IronGolem => 100.0,
            Self::Hoglin => 40.0,
            Self::Piglin => 16.0,
            Self::PiglinBrute => 50.0,
        }
    }

    pub const fn is_mob(self) -> bool {
        !matches!(self, Self::ArmorStand)
    }
}

/// `Entity.save` after `EntityType.create` from template NBT and `snapTo`
/// (`StructureTemplate.java:494-505`). A finalized template's record is
/// marked for the server's `finalizeSpawn(STRUCTURE)` pass.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateMobRecord {
    kind: TemplateMobKind,
    position: [f64; 3],
    rotation: [f32; 2],
    uuid: [i32; 4],
    health: f32,
    persistence_required: bool,
    can_pick_up_loot: bool,
    left_handed: bool,
    custom_name: Option<NbtSummaryValue>,
    custom_name_visible: bool,
    equipment: Option<NbtSummaryValue>,
    extras: TemplateMobExtras,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TemplateMobExtras {
    Villager {
        villager_data: VillagerData,
        offers: Option<NbtSummaryValue>,
        xp: i32,
        age: i32,
        forced_age: i32,
        age_locked: bool,
    },
    ZombieVillager {
        villager_data: VillagerData,
        offers: Option<NbtSummaryValue>,
        xp: i32,
        is_baby: bool,
        can_break_doors: bool,
        conversion_time: i32,
    },
    Jigsaw(JigsawExtras),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VillagerData {
    villager_type: String,
    profession: String,
    level: i32,
}

impl TemplateMobRecord {
    pub fn owns_resource_id(id: &str) -> bool {
        TemplateMobKind::from_resource_id(id).is_some()
    }

    pub fn from_template_nbt(
        nbt: &NbtSummaryValue,
        position: [f64; 3],
        rotation: [f32; 2],
        uuid: [i32; 4],
    ) -> Option<SaveResult<Self>> {
        let entries = compound(nbt)?;
        let raw_id = string_field(entries, "id")?;
        let id = normalize_resource_location(raw_id.to_string()).ok()?;
        let kind = TemplateMobKind::from_resource_id(&id)?;
        Some(Self::build(kind, entries, position, rotation, uuid))
    }

    pub fn build(
        kind: TemplateMobKind,
        entries: &[NbtSummaryEntry],
        position: [f64; 3],
        rotation: [f32; 2],
        uuid: [i32; 4],
    ) -> SaveResult<Self> {
        if position.iter().any(|value| !value.is_finite())
            || rotation.iter().any(|value| !value.is_finite())
        {
            return Err(SaveError::InvalidNbt {
                path: Some(format!("{}.Pos", kind.resource_id()).into()),
                detail: "all coordinates must be finite".to_string(),
            });
        }
        let extras = match kind {
            TemplateMobKind::Villager | TemplateMobKind::ZombieVillager => {
                build_villager_extras(kind, entries)?
            }
            _ => TemplateMobExtras::Jigsaw(jigsaw::build(kind, entries)?),
        };
        Ok(Self {
            kind,
            position,
            rotation,
            uuid,
            health: float_field(entries, "Health").unwrap_or(kind.max_health()),
            persistence_required: byte_bool(entries, "PersistenceRequired").unwrap_or(false),
            can_pick_up_loot: byte_bool(entries, "CanPickUpLoot").unwrap_or(false),
            left_handed: byte_bool(entries, "LeftHanded").unwrap_or(false),
            custom_name: field(entries, "CustomName").cloned(),
            custom_name_visible: byte_bool(entries, "CustomNameVisible").unwrap_or(false),
            equipment: template_equipment(entries),
            extras,
        })
    }

    pub const fn kind(&self) -> TemplateMobKind {
        self.kind
    }

    pub fn entries(&self) -> Vec<NbtSummaryEntry> {
        let doubles = |values: [f64; 3]| {
            NbtSummaryValue::List(
                values
                    .into_iter()
                    .map(|value| NbtSummaryValue::Double(value.to_bits()))
                    .collect(),
            )
        };
        let mut entries = vec![
            entry("id", NbtSummaryValue::string(self.kind.resource_id())),
            entry("Pos", doubles(self.position)),
            entry("Motion", doubles([0.0, 0.0, 0.0])),
            entry(
                "Rotation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(self.rotation[0].to_bits()),
                    NbtSummaryValue::Float(self.rotation[1].to_bits()),
                ]),
            ),
            entry("fall_distance", NbtSummaryValue::Double(0.0_f64.to_bits())),
            entry("Fire", NbtSummaryValue::Short(0)),
            entry("Air", NbtSummaryValue::Short(300)),
            entry("OnGround", NbtSummaryValue::Byte(0)),
            entry("Invulnerable", NbtSummaryValue::Byte(0)),
            entry("PortalCooldown", NbtSummaryValue::Int(0)),
            entry(
                "UUID",
                NbtSummaryValue::IntArray(self.uuid.into_iter().collect()),
            ),
            entry("Health", NbtSummaryValue::Float(self.health.to_bits())),
            entry("HurtTime", NbtSummaryValue::Short(0)),
            entry("HurtByTimestamp", NbtSummaryValue::Int(0)),
            entry("DeathTime", NbtSummaryValue::Short(0)),
            entry(
                "AbsorptionAmount",
                NbtSummaryValue::Float(0.0_f32.to_bits()),
            ),
            entry(
                "current_impulse_context_reset_grace_time",
                NbtSummaryValue::Int(0),
            ),
            entry("attributes", NbtSummaryValue::List(Vec::new())),
            entry("FallFlying", NbtSummaryValue::Byte(0)),
            entry(
                "Brain",
                NbtSummaryValue::Compound(vec![entry(
                    "memories",
                    NbtSummaryValue::Compound(Vec::new()),
                )]),
            ),
        ];
        // `Entity.java:1991-1994`, `LivingEntity.java:749-751`.
        if let Some(name) = &self.custom_name {
            entries.push(entry("CustomName", name.clone()));
        }
        if self.custom_name_visible {
            entries.push(entry("CustomNameVisible", NbtSummaryValue::Byte(1)));
        }
        if let Some(equipment) = &self.equipment {
            entries.push(entry("equipment", equipment.clone()));
        }
        if self.kind.is_mob() {
            entries.extend([
                entry(
                    "CanPickUpLoot",
                    NbtSummaryValue::Byte(i8::from(self.can_pick_up_loot)),
                ),
                entry(
                    "PersistenceRequired",
                    NbtSummaryValue::Byte(i8::from(self.persistence_required)),
                ),
                entry(
                    "LeftHanded",
                    NbtSummaryValue::Byte(i8::from(self.left_handed)),
                ),
            ]);
        }
        match &self.extras {
            TemplateMobExtras::Villager {
                villager_data,
                offers,
                xp,
                age,
                forced_age,
                age_locked,
            } => {
                emit_villager_shared(&mut entries, villager_data, offers, *xp);
                // `AgeableMob.java:163-166`, `AbstractVillager.java:172-181`,
                // `Villager.java:485-497`.
                entries.extend([
                    entry("Age", NbtSummaryValue::Int(*age)),
                    entry("ForcedAge", NbtSummaryValue::Int(*forced_age)),
                    entry("AgeLocked", NbtSummaryValue::Byte(i8::from(*age_locked))),
                    entry("Inventory", NbtSummaryValue::List(Vec::new())),
                    entry("FoodLevel", NbtSummaryValue::Byte(0)),
                    entry("Gossips", NbtSummaryValue::List(Vec::new())),
                    entry("LastRestock", NbtSummaryValue::Long(0)),
                    entry("LastGossipDecay", NbtSummaryValue::Long(0)),
                    entry("RestocksToday", NbtSummaryValue::Int(0)),
                ]);
            }
            TemplateMobExtras::ZombieVillager {
                villager_data,
                offers,
                xp,
                is_baby,
                can_break_doors,
                conversion_time,
            } => {
                emit_villager_shared(&mut entries, villager_data, offers, *xp);
                // `Zombie.java:397-402`, `ZombieVillager.java:95-103`.
                entries.extend([
                    entry("IsBaby", NbtSummaryValue::Byte(i8::from(*is_baby))),
                    entry(
                        "CanBreakDoors",
                        NbtSummaryValue::Byte(i8::from(*can_break_doors)),
                    ),
                    entry("InWaterTime", NbtSummaryValue::Int(-1)),
                    entry("DrownedConversionTime", NbtSummaryValue::Int(-1)),
                    entry("ConversionTime", NbtSummaryValue::Int(*conversion_time)),
                ]);
            }
            TemplateMobExtras::Jigsaw(extras) => jigsaw::emit(extras, &mut entries),
        }
        entries
    }
}

/// `EntityEquipment.CODEC` keeps the stacks of serialized slot names; an empty
/// equipment is not written.
fn template_equipment(entries: &[NbtSummaryEntry]) -> Option<NbtSummaryValue> {
    const SLOTS: [&str; 8] = [
        "mainhand", "offhand", "feet", "legs", "chest", "head", "body", "saddle",
    ];
    let slots: Vec<NbtSummaryEntry> = compound(field(entries, "equipment")?)?
        .iter()
        .filter(|slot| {
            SLOTS.iter().any(|name| slot.name == *name)
                && matches!(slot.value, NbtSummaryValue::Compound(_))
        })
        .cloned()
        .collect();
    (!slots.is_empty()).then_some(NbtSummaryValue::Compound(slots))
}

pub fn build_villager_extras(
    kind: TemplateMobKind,
    entries: &[NbtSummaryEntry],
) -> SaveResult<TemplateMobExtras> {
    let villager_data = read_villager_data(entries).ok_or_else(|| SaveError::InvalidNbt {
        path: Some(format!("{}.VillagerData", kind.resource_id()).into()),
        detail: "template villager data is required".to_string(),
    })?;
    let offers = match field(entries, "Offers") {
        Some(value) => Some(
            canonicalize_offers(value).ok_or_else(|| SaveError::InvalidNbt {
                path: Some(format!("{}.Offers", kind.resource_id()).into()),
                detail: "template offers are not MerchantOffers.CODEC".to_string(),
            })?,
        ),
        None => None,
    };
    let xp = int_field(entries, "Xp").unwrap_or(0);
    Ok(match kind {
        TemplateMobKind::Villager => TemplateMobExtras::Villager {
            villager_data,
            offers,
            xp,
            age: int_field(entries, "Age").unwrap_or(0),
            forced_age: int_field(entries, "ForcedAge").unwrap_or(0),
            age_locked: byte_bool(entries, "AgeLocked").unwrap_or(false),
        },
        TemplateMobKind::ZombieVillager => TemplateMobExtras::ZombieVillager {
            villager_data,
            offers,
            xp,
            is_baby: byte_bool(entries, "IsBaby").unwrap_or(false),
            can_break_doors: byte_bool(entries, "CanBreakDoors").unwrap_or(false),
            conversion_time: int_field(entries, "ConversionTime").unwrap_or(-1),
        },
        _ => unreachable!("villager extras"),
    })
}

pub fn emit_villager_shared(
    entries: &mut Vec<NbtSummaryEntry>,
    villager_data: &VillagerData,
    offers: &Option<NbtSummaryValue>,
    xp: i32,
) {
    entries.extend([
        entry(
            "VillagerData",
            NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::string(villager_data.villager_type.clone()),
                ),
                entry(
                    "profession",
                    NbtSummaryValue::string(villager_data.profession.clone()),
                ),
                entry("level", NbtSummaryValue::Int(villager_data.level)),
            ]),
        ),
        entry("VillagerDataFinalized", NbtSummaryValue::Byte(1)),
        entry("Xp", NbtSummaryValue::Int(xp)),
    ]);
    if let Some(offers) = offers {
        entries.push(entry("Offers", offers.clone()));
    }
}

pub fn read_villager_data(entries: &[NbtSummaryEntry]) -> Option<VillagerData> {
    let data = compound(field(entries, "VillagerData")?)?;
    let villager_type = normalize_resource_location(string_field(data, "type")?).ok()?;
    let profession = normalize_resource_location(string_field(data, "profession")?).ok()?;
    let level = int_field(data, "level").unwrap_or(1).max(1);
    Some(VillagerData {
        villager_type,
        profession,
        level,
    })
}

/// `MerchantOffer.CODEC` defaults (`MerchantOffer.java:12-26`).
pub fn canonicalize_offers(value: &NbtSummaryValue) -> Option<NbtSummaryValue> {
    let entries = compound(value)?;
    let recipes = match field(entries, "Recipes") {
        Some(NbtSummaryValue::List(recipes)) if !recipes.is_empty() => recipes,
        _ => return None,
    };
    let mut canonical = Vec::with_capacity(recipes.len());
    for recipe in recipes {
        canonical.push(canonicalize_offer(recipe)?);
    }
    Some(NbtSummaryValue::Compound(vec![entry(
        "Recipes",
        NbtSummaryValue::List(canonical),
    )]))
}

pub fn canonicalize_offer(value: &NbtSummaryValue) -> Option<NbtSummaryValue> {
    let entries = compound(value)?;
    let mut offer = vec![
        entry("buy", canonicalize_item(field(entries, "buy")?)?),
        entry("sell", canonicalize_item(field(entries, "sell")?)?),
    ];
    if let Some(buy_b) = field(entries, "buyB") {
        offer.push(entry("buyB", canonicalize_item(buy_b)?));
    }
    let uses = int_field(entries, "uses").unwrap_or(0);
    if uses != 0 {
        offer.push(entry("uses", NbtSummaryValue::Int(uses)));
    }
    let max_uses = int_field(entries, "maxUses").unwrap_or(4);
    if max_uses != 4 {
        offer.push(entry("maxUses", NbtSummaryValue::Int(max_uses)));
    }
    if !byte_bool(entries, "rewardExp").unwrap_or(true) {
        offer.push(entry("rewardExp", NbtSummaryValue::Byte(0)));
    }
    let special_price = int_field(entries, "specialPrice").unwrap_or(0);
    if special_price != 0 {
        offer.push(entry("specialPrice", NbtSummaryValue::Int(special_price)));
    }
    let demand = int_field(entries, "demand").unwrap_or(0);
    if demand != 0 {
        offer.push(entry("demand", NbtSummaryValue::Int(demand)));
    }
    if let Some(multiplier) = float_field(entries, "priceMultiplier") {
        if multiplier != 0.0 {
            offer.push(entry(
                "priceMultiplier",
                NbtSummaryValue::Float(multiplier.to_bits()),
            ));
        }
    }
    let xp = int_field(entries, "xp").unwrap_or(1);
    if xp != 1 {
        offer.push(entry("xp", NbtSummaryValue::Int(xp)));
    }
    Some(NbtSummaryValue::Compound(offer))
}

pub fn canonicalize_item(value: &NbtSummaryValue) -> Option<NbtSummaryValue> {
    let entries = compound(value)?;
    let id = normalize_resource_location(string_field(entries, "id")?).ok()?;
    let count = int_field(entries, "count").unwrap_or(1);
    if count <= 0 {
        return None;
    }
    Some(NbtSummaryValue::Compound(vec![
        entry("id", NbtSummaryValue::string(id)),
        entry("count", NbtSummaryValue::Int(count)),
    ]))
}

pub fn compound(value: &NbtSummaryValue) -> Option<&[NbtSummaryEntry]> {
    match value {
        NbtSummaryValue::Compound(entries) => Some(entries),
        _ => None,
    }
}

pub fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

pub fn string_field(entries: &[NbtSummaryEntry], name: &str) -> Option<String> {
    match field(entries, name)? {
        NbtSummaryValue::String(value) => value.try_to_string().ok(),
        _ => None,
    }
}

pub fn resource_string(entries: &[NbtSummaryEntry], name: &str) -> Option<String> {
    normalize_resource_location(string_field(entries, name)?).ok()
}

pub fn int_field(entries: &[NbtSummaryEntry], name: &str) -> Option<i32> {
    match field(entries, name)? {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        _ => None,
    }
}

pub fn long_field(entries: &[NbtSummaryEntry], name: &str) -> Option<i64> {
    match field(entries, name)? {
        NbtSummaryValue::Byte(value) => Some(i64::from(*value)),
        NbtSummaryValue::Short(value) => Some(i64::from(*value)),
        NbtSummaryValue::Int(value) => Some(i64::from(*value)),
        NbtSummaryValue::Long(value) => Some(*value),
        _ => None,
    }
}

pub fn float_field(entries: &[NbtSummaryEntry], name: &str) -> Option<f32> {
    match field(entries, name)? {
        NbtSummaryValue::Float(bits) => Some(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => Some(f64::from_bits(*bits) as f32),
        _ => None,
    }
}

pub fn float_list(value: &NbtSummaryValue, len: usize) -> Option<Vec<f32>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    if values.len() != len {
        return None;
    }
    values
        .iter()
        .map(|value| match value {
            NbtSummaryValue::Float(bits) => Some(f32::from_bits(*bits)),
            NbtSummaryValue::Double(bits) => Some(f64::from_bits(*bits) as f32),
            _ => None,
        })
        .collect()
}

pub fn byte_field(entries: &[NbtSummaryEntry], name: &str) -> Option<i8> {
    match field(entries, name)? {
        NbtSummaryValue::Byte(value) => Some(*value),
        _ => None,
    }
}

pub fn byte_bool(entries: &[NbtSummaryEntry], name: &str) -> Option<bool> {
    match field(entries, name)? {
        NbtSummaryValue::Byte(0) => Some(false),
        NbtSummaryValue::Byte(1) => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::test_nbt::villager_nbt;
    use super::*;

    #[test]
    fn canonical_offers_omit_merchant_codec_defaults() {
        let record = TemplateMobRecord::from_template_nbt(
            &villager_nbt(),
            [0.5, 64.0, 0.5],
            [0.0, 0.0],
            [0, 0, 0, 1],
        )
        .unwrap()
        .unwrap();
        let Some(NbtSummaryValue::Compound(offers)) = record
            .entries()
            .into_iter()
            .find(|entry| entry.name == "Offers")
            .map(|entry| entry.value)
        else {
            panic!("villager must keep nonempty offers");
        };
        let Some(NbtSummaryValue::List(recipes)) = offers
            .iter()
            .find(|entry| entry.name == "Recipes")
            .map(|entry| &entry.value)
        else {
            panic!("offers must list recipes");
        };
        let NbtSummaryValue::Compound(recipe) = &recipes[0] else {
            panic!("recipe must be a compound");
        };
        let names: Vec<&bbb_protocol::JavaString> =
            recipe.iter().map(|entry| &entry.name).collect();
        assert_eq!(names, ["buy", "sell", "maxUses"]);
    }
}
