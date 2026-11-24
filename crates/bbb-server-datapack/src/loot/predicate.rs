//! The `net/minecraft/advancements/criterion` predicates loot conditions and
//! advancement criteria share: item, entity, location, damage source and
//! block state properties.
//!
//! A term vanilla reads but this server cannot answer is kept as a name, so
//! the evaluator refuses it instead of guessing.

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
use bbb_protocol::{JavaString, NbtSummaryValue};

use super::codec::{
    boolean, custom_data_tag, entries, fail, id, int, list, string, to_nbt, At, Obj, Result, V,
};
use super::number::{DoubleBounds, HolderSet, IntBounds};

/// `ItemPredicate.CODEC`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemPredicate {
    pub items: Option<HolderSet>,
    pub count: IntBounds,
    /// The exact half of `DataComponentMatchers`.
    pub components: Vec<(String, NbtSummaryValue)>,
    /// The partial half of `DataComponentMatchers`.
    pub predicates: Vec<ComponentPredicate>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentPredicate {
    CustomData(NbtSummaryValue),
    Damage {
        durability: IntBounds,
        damage: IntBounds,
    },
    PotionContents(HolderSet),
    AttributeModifiers {
        modifiers: Option<CollectionPredicate<AttributeModifierPredicate>>,
    },
    Trim {
        material: Option<HolderSet>,
        pattern: Option<HolderSet>,
    },
    JukeboxPlayable {
        song: Option<HolderSet>,
    },
    VillagerVariant(HolderSet),
    Enchantments(Vec<EnchantmentPredicate>),
    StoredEnchantments(Vec<EnchantmentPredicate>),
    FireworkExplosion(FireworkExplosionPredicate),
    Fireworks {
        explosions: Option<CollectionPredicate<FireworkExplosionPredicate>>,
        flight_duration: IntBounds,
    },
    Container {
        items: Option<CollectionPredicate<ItemPredicate>>,
    },
    BundleContents {
        items: Option<CollectionPredicate<ItemPredicate>>,
    },
    WritableBook {
        pages: Option<CollectionPredicate<JavaString>>,
    },
    WrittenBook {
        pages: Option<CollectionPredicate<NbtSummaryValue>>,
        author: Option<JavaString>,
        title: Option<JavaString>,
        generation: IntBounds,
        resolved: Option<bool>,
    },
    /// `AnyValue` (`AnyValue.java:6-11`): a component type that is not a
    /// predicate type only asks whether the stack has the component.
    AnyValue(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeModifierPredicate {
    pub attribute: Option<HolderSet>,
    pub id: Option<String>,
    pub amount: DoubleBounds,
    pub operation: Option<i32>,
    pub slot: Option<i32>,
}

impl AttributeModifierPredicate {
    fn decode(value: V<'_>) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            attribute: obj.opt("attribute", HolderSet::decode)?,
            id: obj.opt("id", id)?,
            amount: obj.or("amount", DoubleBounds::default(), DoubleBounds::decode)?,
            operation: obj.opt("operation", |value| {
                bbb_protocol::attribute_modifier_types::attribute_modifier_operation_id(&string(
                    value,
                )?)
                .map_or_else(|| fail("unknown attribute modifier operation"), Ok)
            })?,
            slot: obj.opt("slot", |value| {
                bbb_protocol::equipment_slots::vanilla_equipment_slot_group_for_serialized_name(
                    &string(value)?,
                )
                .map(|group| group.id)
                .map_or_else(|| fail("unknown equipment slot group"), Ok)
            })?,
        })
    }
}

/// Shape and effects shared by explosion components and rocket explosions.
#[derive(Debug, Clone, PartialEq)]
pub struct FireworkExplosionPredicate {
    shape: Option<FireworkExplosionShapeSummary>,
    has_twinkle: Option<bool>,
    has_trail: Option<bool>,
}

impl FireworkExplosionPredicate {
    fn decode(value: V<'_>) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            shape: obj.opt("shape", |value| {
                FireworkExplosionShapeSummary::from_serialized_name(&string(value)?)
                    .map_or_else(|| fail("unknown firework shape"), Ok)
            })?,
            has_twinkle: obj.opt("has_twinkle", boolean)?,
            has_trail: obj.opt("has_trail", boolean)?,
        })
    }

    pub fn matches(
        &self,
        shape: FireworkExplosionShapeSummary,
        twinkle: bool,
        trail: bool,
    ) -> bool {
        self.shape.is_none_or(|expected| expected == shape)
            && self.has_twinkle.is_none_or(|expected| expected == twinkle)
            && self.has_trail.is_none_or(|expected| expected == trail)
    }
}

/// Collection contents, matching counts and size, shared by component predicates.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionPredicate<P> {
    contains: Vec<P>,
    counts: Vec<(P, IntBounds)>,
    size: IntBounds,
}

impl<P> CollectionPredicate<P> {
    fn decode(value: V<'_>, decode: impl Fn(V<'_>) -> Result<P> + Copy) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            contains: obj.or("contains", Vec::new(), |value| list(value, decode))?,
            counts: obj.or("count", Vec::new(), |value| {
                list(value, |value| {
                    let obj = Obj::of(value)?;
                    Ok((
                        obj.req("test", decode)?,
                        obj.req("count", IntBounds::decode)?,
                    ))
                })
            })?,
            size: obj.or("size", IntBounds::default(), IntBounds::decode)?,
        })
    }

    pub fn predicates(&self) -> impl Iterator<Item = &P> {
        self.contains
            .iter()
            .chain(self.counts.iter().map(|(test, _)| test))
    }

    pub fn matches<T, E>(
        &self,
        values: &[T],
        mut test: impl FnMut(&P, &T) -> std::result::Result<bool, E>,
    ) -> std::result::Result<bool, E> {
        if !self.size.matches(values.len() as i32) {
            return Ok(false);
        }
        for expected in &self.contains {
            let mut found = false;
            for value in values {
                if test(expected, value)? {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(false);
            }
        }
        for (expected, count) in &self.counts {
            let mut matched = 0;
            for value in values {
                matched += i32::from(test(expected, value)?);
            }
            if !count.matches(matched) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn java_string(value: V<'_>) -> Result<JavaString> {
    value
        .java_string()
        .map_or_else(|| fail("expected a string"), Ok)
}

/// `EnchantmentPredicate.CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantmentPredicate {
    pub enchantments: Option<HolderSet>,
    pub levels: IntBounds,
}

impl ItemPredicate {
    pub fn decode(value: V<'_>) -> Result<Self> {
        Self::decode_with_content_registry(value, ContentRegistry::vanilla())
    }

    pub fn decode_with_content_registry(
        value: V<'_>,
        content_registry: &ContentRegistry,
    ) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            items: obj.opt("items", HolderSet::decode)?,
            count: obj.or("count", IntBounds::default(), IntBounds::decode)?,
            components: obj.or("components", Vec::new(), exact_components)?,
            predicates: obj.or("predicates", Vec::new(), |value| {
                component_predicates(value, content_registry)
            })?,
        })
    }
}

fn component_predicates(
    value: V<'_>,
    content_registry: &ContentRegistry,
) -> Result<Vec<ComponentPredicate>> {
    Obj::of(value)?
        .entries()
        .into_iter()
        .map(|(key, value)| {
            let component = component_id(key).at(&key)?;
            Ok(match component.as_str() {
                "minecraft:custom_data" => ComponentPredicate::CustomData(
                    NbtSummaryValue::Compound(custom_data_tag(value).at(&key)?),
                ),
                "minecraft:damage" => {
                    let obj = Obj::of(value).at(&key)?;
                    ComponentPredicate::Damage {
                        durability: obj.or(
                            "durability",
                            IntBounds::default(),
                            IntBounds::decode,
                        )?,
                        damage: obj.or("damage", IntBounds::default(), IntBounds::decode)?,
                    }
                }
                "minecraft:potion_contents" => {
                    ComponentPredicate::PotionContents(HolderSet::decode(value).at(&key)?)
                }
                "minecraft:enchantments" => ComponentPredicate::Enchantments(
                    list(value, EnchantmentPredicate::decode).at(&key)?,
                ),
                "minecraft:stored_enchantments" => ComponentPredicate::StoredEnchantments(
                    list(value, EnchantmentPredicate::decode).at(&key)?,
                ),
                "minecraft:firework_explosion" => ComponentPredicate::FireworkExplosion(
                    FireworkExplosionPredicate::decode(value).at(&key)?,
                ),
                "minecraft:fireworks" => {
                    let obj = Obj::of(value)?;
                    ComponentPredicate::Fireworks {
                        explosions: obj.opt("explosions", |value| {
                            CollectionPredicate::decode(value, FireworkExplosionPredicate::decode)
                        })?,
                        flight_duration: obj.or(
                            "flight_duration",
                            IntBounds::default(),
                            IntBounds::decode,
                        )?,
                    }
                }
                "minecraft:trim" => {
                    let obj = Obj::of(value)?;
                    ComponentPredicate::Trim {
                        material: obj.opt("material", HolderSet::decode)?,
                        pattern: obj.opt("pattern", HolderSet::decode)?,
                    }
                }
                "minecraft:jukebox_playable" => ComponentPredicate::JukeboxPlayable {
                    song: Obj::of(value)?.opt("song", HolderSet::decode)?,
                },
                "minecraft:villager/variant" => {
                    ComponentPredicate::VillagerVariant(HolderSet::decode(value)?)
                }
                "minecraft:container" => ComponentPredicate::Container {
                    items: Obj::of(value)?.opt("items", |value| {
                        CollectionPredicate::decode(value, |value| {
                            ItemPredicate::decode_with_content_registry(value, content_registry)
                        })
                    })?,
                },
                "minecraft:bundle_contents" => ComponentPredicate::BundleContents {
                    items: Obj::of(value)?.opt("items", |value| {
                        CollectionPredicate::decode(value, |value| {
                            ItemPredicate::decode_with_content_registry(value, content_registry)
                        })
                    })?,
                },
                "minecraft:writable_book_content" => ComponentPredicate::WritableBook {
                    pages: Obj::of(value)?.opt("pages", |value| {
                        CollectionPredicate::decode(value, java_string)
                    })?,
                },
                "minecraft:written_book_content" => {
                    let obj = Obj::of(value)?;
                    ComponentPredicate::WrittenBook {
                        pages: obj.opt("pages", |value| {
                            CollectionPredicate::decode(value, |value| {
                                super::codec::text_parameter_with_content_registry(
                                    value,
                                    content_registry,
                                )
                            })
                        })?,
                        author: obj.opt("author", java_string)?,
                        title: obj.opt("title", java_string)?,
                        generation: obj.or(
                            "generation",
                            IntBounds::default(),
                            IntBounds::decode,
                        )?,
                        resolved: obj.opt("resolved", boolean)?,
                    }
                }
                "minecraft:attribute_modifiers" => ComponentPredicate::AttributeModifiers {
                    modifiers: Obj::of(value)?.opt("modifiers", |value| {
                        CollectionPredicate::decode(value, AttributeModifierPredicate::decode)
                    })?,
                },
                _ => ComponentPredicate::AnyValue(component),
            })
        })
        .collect()
}

impl EnchantmentPredicate {
    fn decode(value: V<'_>) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            enchantments: obj.opt("enchantments", HolderSet::decode)?,
            levels: obj.or("levels", IntBounds::default(), IntBounds::decode)?,
        })
    }
}

fn component_id(raw: &JavaString) -> Result<String> {
    bbb_protocol::normalize_resource_location(
        raw.try_to_string()
            .or_else(|_| fail("invalid component identifier"))?,
    )
    .or_else(|_| fail(format!("invalid component type `{raw}`")))
}

/// `DataComponentExactPredicate.CODEC`: component id to expected value.
fn exact_components(value: V<'_>) -> Result<Vec<(String, NbtSummaryValue)>> {
    Obj::of(value)?
        .entries()
        .into_iter()
        .map(|(key, value)| Ok((component_id(key)?, to_nbt(value).at(&key)?)))
        .collect()
}

/// `EntityPredicate.CODEC`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityPredicate {
    pub entity_type: Option<HolderSet>,
    pub distance: Option<DistancePredicate>,
    pub movement: Option<MovementPredicate>,
    pub location: Option<LocationPredicate>,
    pub movement_affected_by: Option<LocationPredicate>,
    pub stepping_on: Option<LocationPredicate>,
    pub nbt: Option<NbtSummaryValue>,
    pub flags: Option<EntityFlags>,
    pub equipment: Option<Vec<(EquipmentSlot, ItemPredicate)>>,
    pub effects: Option<Vec<(String, MobEffectInstancePredicate)>>,
    pub slots: Option<Vec<(Vec<i32>, ItemPredicate)>>,
    pub type_specific: Option<EntitySubPredicate>,
    pub periodic_tick: Option<i32>,
    pub vehicle: Option<Box<EntityPredicate>>,
    pub passenger: Option<Box<EntityPredicate>>,
    pub targeted_entity: Option<Box<EntityPredicate>>,
    pub team: Option<JavaString>,
    pub components: Vec<(String, NbtSummaryValue)>,
    pub predicates: Vec<ComponentPredicate>,
}

/// `MobEffectsPredicate.MobEffectInstancePredicate.CODEC`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MobEffectInstancePredicate {
    pub amplifier: IntBounds,
    pub duration: IntBounds,
    pub ambient: Option<bool>,
    pub visible: Option<bool>,
}

impl MobEffectInstancePredicate {
    fn decode(value: V<'_>) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            amplifier: obj.or("amplifier", IntBounds::default(), IntBounds::decode)?,
            duration: obj.or("duration", IntBounds::default(), IntBounds::decode)?,
            ambient: obj.opt("ambient", boolean)?,
            visible: obj.opt("visible", boolean)?,
        })
    }
}

pub(super) fn mob_effects_predicate(
    value: V<'_>,
) -> Result<Vec<(String, MobEffectInstancePredicate)>> {
    Obj::of(value)?
        .entries()
        .into_iter()
        .map(|(key, value)| {
            let effect = bbb_protocol::normalize_resource_location(
                key.try_to_string()
                    .or_else(|_| fail("invalid identifier"))?,
            )
            .or_else(|_| fail(format!("invalid effect `{key}`")))?;
            if bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(&effect)
                .is_none()
            {
                return fail(format!("unknown effect `{effect}`"));
            }
            Ok((effect, MobEffectInstancePredicate::decode(value).at(&key)?))
        })
        .collect()
}

/// `DistancePredicate.CODEC`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DistancePredicate {
    pub x: DoubleBounds,
    pub y: DoubleBounds,
    pub z: DoubleBounds,
    pub horizontal: DoubleBounds,
    pub absolute: DoubleBounds,
}

impl DistancePredicate {
    pub fn decode(value: V<'_>) -> Result<Self> {
        let obj = Obj::of(value)?;
        let field = |name| obj.or(name, DoubleBounds::default(), DoubleBounds::decode);
        Ok(Self {
            x: field("x")?,
            y: field("y")?,
            z: field("z")?,
            horizontal: field("horizontal")?,
            absolute: field("absolute")?,
        })
    }

    #[inline]
    pub fn matches_delta(&self, [x, y, z]: [f32; 3]) -> bool {
        // Squared sums remain f32 before bounds compare them as f64.
        self.x.matches(f64::from(x.abs()))
            && self.y.matches(f64::from(y.abs()))
            && self.z.matches(f64::from(z.abs()))
            && self.horizontal.matches_sqr(f64::from(x * x + z * z))
            && self.absolute.matches_sqr(f64::from(x * x + y * y + z * z))
    }
}

/// `MovementPredicate.CODEC`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MovementPredicate {
    pub x: DoubleBounds,
    pub y: DoubleBounds,
    pub z: DoubleBounds,
    pub speed: DoubleBounds,
    pub horizontal_speed: DoubleBounds,
    pub vertical_speed: DoubleBounds,
    pub fall_distance: DoubleBounds,
}

/// `EntityFlagsPredicate.CODEC`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityFlags {
    pub on_ground: Option<bool>,
    pub on_fire: Option<bool>,
    pub crouching: Option<bool>,
    pub sprinting: Option<bool>,
    pub swimming: Option<bool>,
    pub flying: Option<bool>,
    pub baby: Option<bool>,
    pub in_water: Option<bool>,
    pub fall_flying: Option<bool>,
}

/// `EntitySubPredicate.CODEC` for the types this server reads.
#[derive(Debug, Clone, PartialEq)]
pub enum EntitySubPredicate {
    Sheep {
        sheared: Option<bool>,
    },
    Slime {
        size: IntBounds,
    },
    Raider {
        has_raid: bool,
        is_captain: bool,
    },
    FishingHook {
        in_open_water: Option<bool>,
    },
    /// `LightningBoltPredicate` (`LightningBoltPredicate.java:12-37`).
    Lightning {
        blocks_set_on_fire: IntBounds,
        entity_struck: Option<Box<EntityPredicate>>,
    },
    /// `PlayerPredicate` (`PlayerPredicate.java:46-276`).
    Player {
        level: IntBounds,
        food_level: IntBounds,
        food_saturation: DoubleBounds,
        /// `GameTypePredicate`'s names; `None` is any.
        gamemode: Option<Vec<String>>,
        terms: PlayerTerms,
    },
    Unsupported(String),
}

/// The `PlayerPredicate` terms that read the player's own records.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerTerms {
    pub stats: Vec<(bbb_server_types::stats::StatKey, IntBounds)>,
    pub recipes: Vec<(String, bool)>,
    pub advancements: Vec<(String, AdvancementTerm)>,
    pub looking_at: Option<Box<EntityPredicate>>,
    pub input: Option<InputPredicate>,
}

impl PlayerTerms {
    /// Whether a term other than `looking_at` reads the player's records.
    pub fn reads_records(&self) -> bool {
        !self.stats.is_empty()
            || !self.recipes.is_empty()
            || !self.advancements.is_empty()
            || self.input.is_some()
    }
}

/// `PlayerPredicate.AdvancementPredicate` (`PlayerPredicate.java:144-182`).
#[derive(Debug, Clone, PartialEq)]
pub enum AdvancementTerm {
    Done(bool),
    Criteria(Vec<(JavaString, bool)>),
}

/// `InputPredicate` (`InputPredicate.java:8-38`); `None` is any.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputPredicate {
    pub forward: Option<bool>,
    pub backward: Option<bool>,
    pub left: Option<bool>,
    pub right: Option<bool>,
    pub jump: Option<bool>,
    pub sneak: Option<bool>,
    pub sprint: Option<bool>,
}

fn bool_map(value: V<'_>) -> Result<Vec<(JavaString, bool)>> {
    Obj::of(value)?
        .entries()
        .into_iter()
        .map(|(name, value)| Ok((name.clone(), boolean(value)?)))
        .collect()
}

impl PlayerTerms {
    fn decode(obj: &Obj<'_>, content_registry: &ContentRegistry) -> Result<Self> {
        Ok(Self {
            stats: obj.or("stats", Vec::new(), |value| {
                list(value, |stat| {
                    let stat = Obj::of(stat)?;
                    let stat_type = stat.req("type", id)?;
                    let value = stat.req("stat", id)?;
                    let key = bbb_server_types::stats::StatKey::from_resource_ids_with_registry(
                        &stat_type,
                        &value,
                        content_registry,
                    )
                    .map_or_else(|| fail(format!("unknown stat {stat_type} {value}")), Ok)?;
                    Ok((
                        key,
                        stat.or("value", IntBounds::default(), IntBounds::decode)?,
                    ))
                })
            })?,
            recipes: obj
                .or("recipes", Vec::new(), bool_map)?
                .into_iter()
                .map(|(recipe, present)| {
                    bbb_protocol::normalize_resource_location(
                        recipe
                            .try_to_string()
                            .or_else(|_| fail("invalid recipe identifier"))?,
                    )
                    .map(|recipe| (recipe, present))
                    .map_or_else(|_| fail("recipe key is not an identifier"), Ok)
                })
                .collect::<Result<_>>()?,
            advancements: obj.or("advancements", Vec::new(), |value| {
                Obj::of(value)?
                    .entries()
                    .into_iter()
                    .map(|(name, value)| {
                        let id = bbb_protocol::normalize_resource_location(
                            name.try_to_string()
                                .or_else(|_| fail("invalid advancement identifier"))?,
                        )
                        .map_or_else(|_| fail("advancement key is not an identifier"), Ok)?;
                        let term = match boolean(value) {
                            Ok(done) => AdvancementTerm::Done(done),
                            Err(_) => AdvancementTerm::Criteria(bool_map(value)?),
                        };
                        Ok((id, term))
                    })
                    .collect()
            })?,
            looking_at: obj.opt("looking_at", |value| {
                EntityPredicate::decode_with_content_registry(value, content_registry).map(Box::new)
            })?,
            input: obj.opt("input", |value| {
                let input = Obj::of(value)?;
                let flag = |name| input.opt(name, boolean);
                Ok(InputPredicate {
                    forward: flag("forward")?,
                    backward: flag("backward")?,
                    left: flag("left")?,
                    right: flag("right")?,
                    jump: flag("jump")?,
                    sneak: flag("sneak")?,
                    sprint: flag("sprint")?,
                })
            })?,
        })
    }
}

impl EntityPredicate {
    pub fn decode(value: V<'_>) -> Result<Self> {
        Self::decode_with_content_registry(value, ContentRegistry::vanilla())
    }

    pub fn decode_with_content_registry(
        value: V<'_>,
        content_registry: &ContentRegistry,
    ) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            entity_type: obj.opt("type", HolderSet::decode)?,
            distance: obj.opt("distance", DistancePredicate::decode)?,
            movement: obj.opt("movement", |value| {
                let obj = Obj::of(value)?;
                let field = |name| obj.or(name, DoubleBounds::default(), DoubleBounds::decode);
                Ok(MovementPredicate {
                    x: field("x")?,
                    y: field("y")?,
                    z: field("z")?,
                    speed: field("speed")?,
                    horizontal_speed: field("horizontal_speed")?,
                    vertical_speed: field("vertical_speed")?,
                    fall_distance: field("fall_distance")?,
                })
            })?,
            location: obj.opt("location", |value| {
                LocationPredicate::decode_with_content_registry(value, content_registry)
            })?,
            movement_affected_by: obj.opt("movement_affected_by", |value| {
                LocationPredicate::decode_with_content_registry(value, content_registry)
            })?,
            stepping_on: obj.opt("stepping_on", |value| {
                LocationPredicate::decode_with_content_registry(value, content_registry)
            })?,
            nbt: obj.opt("nbt", |value| {
                custom_data_tag(value).map(NbtSummaryValue::Compound)
            })?,
            flags: obj.opt("flags", |value| {
                let obj = Obj::of(value)?;
                let flag = |name| obj.opt(name, boolean);
                Ok(EntityFlags {
                    on_ground: flag("is_on_ground")?,
                    on_fire: flag("is_on_fire")?,
                    crouching: flag("is_sneaking")?,
                    sprinting: flag("is_sprinting")?,
                    swimming: flag("is_swimming")?,
                    flying: flag("is_flying")?,
                    baby: flag("is_baby")?,
                    in_water: flag("is_in_water")?,
                    fall_flying: flag("is_fall_flying")?,
                })
            })?,
            equipment: obj.opt("equipment", |value| {
                let obj = Obj::of(value)?;
                let mut slots = Vec::new();
                for (name, slot) in [
                    ("head", EquipmentSlot::Head),
                    ("chest", EquipmentSlot::Chest),
                    ("legs", EquipmentSlot::Legs),
                    ("feet", EquipmentSlot::Feet),
                    ("body", EquipmentSlot::Body),
                    ("mainhand", EquipmentSlot::MainHand),
                    ("offhand", EquipmentSlot::OffHand),
                ] {
                    if let Some(predicate) = obj.opt(name, |value| {
                        ItemPredicate::decode_with_content_registry(value, content_registry)
                    })? {
                        slots.push((slot, predicate));
                    }
                }
                Ok(slots)
            })?,
            slots: obj
                .opt("slots", |value| {
                    Obj::of(value)?
                        .entries()
                        .into_iter()
                        .map(|(name, value)| {
                            let Some(slots) = name
                                .with_utf8(bbb_server_types::slot_ranges::slot_range)
                                .flatten()
                            else {
                                return fail(format!("unknown slot range `{name}`"));
                            };
                            Ok((
                                slots.to_vec(),
                                ItemPredicate::decode_with_content_registry(
                                    value,
                                    content_registry,
                                )
                                .at(&name)?,
                            ))
                        })
                        .collect::<Result<Vec<_>>>()
                })?
                .filter(|slots| !slots.is_empty()),
            effects: obj.opt("effects", mob_effects_predicate)?,
            type_specific: obj.opt("type_specific", |value| {
                EntitySubPredicate::decode(value, content_registry)
            })?,
            periodic_tick: obj.opt("periodic_tick", |value| {
                let period = int(value)?;
                if period <= 0 {
                    return fail(format!("Value must be positive: {period}"));
                }
                Ok(period)
            })?,
            vehicle: obj.opt("vehicle", |value| {
                Self::decode_with_content_registry(value, content_registry).map(Box::new)
            })?,
            passenger: obj.opt("passenger", |value| {
                Self::decode_with_content_registry(value, content_registry).map(Box::new)
            })?,
            targeted_entity: obj.opt("targeted_entity", |value| {
                Self::decode_with_content_registry(value, content_registry).map(Box::new)
            })?,
            team: obj.opt("team", java_string)?,
            components: obj.or("components", Vec::new(), exact_components)?,
            predicates: obj.or("predicates", Vec::new(), |value| {
                component_predicates(value, content_registry)
            })?,
        })
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl EntitySubPredicate {
    fn decode(value: V<'_>, content_registry: &ContentRegistry) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(match obj.req("type", id)?.as_str() {
            "minecraft:sheep" => Self::Sheep {
                sheared: obj.opt("sheared", boolean)?,
            },
            "minecraft:slime" => Self::Slime {
                size: obj.or("size", IntBounds::default(), IntBounds::decode)?,
            },
            "minecraft:raider" => Self::Raider {
                has_raid: obj.or("has_raid", false, boolean)?,
                is_captain: obj.or("is_captain", false, boolean)?,
            },
            "minecraft:fishing_hook" => Self::FishingHook {
                in_open_water: obj.opt("in_open_water", boolean)?,
            },
            "minecraft:lightning" => Self::Lightning {
                blocks_set_on_fire: obj.or(
                    "blocks_set_on_fire",
                    IntBounds::default(),
                    IntBounds::decode,
                )?,
                entity_struck: obj.opt("entity_struck", |value| {
                    EntityPredicate::decode_with_content_registry(value, content_registry)
                        .map(Box::new)
                })?,
            },
            "minecraft:player" => {
                let food = obj.opt("food", Obj::of)?;
                let food_field = |name| food.as_ref().map_or(Ok(None), |food| food.opt(name, Ok));
                Self::Player {
                    level: obj.or("level", IntBounds::default(), IntBounds::decode)?,
                    food_level: food_field("level")?
                        .map_or(Ok(IntBounds::default()), IntBounds::decode)?,
                    food_saturation: food_field("saturation")?
                        .map_or(Ok(DoubleBounds::default()), DoubleBounds::decode)?,
                    gamemode: obj.opt("gamemode", |value| list(value, |name| string(name)))?,
                    terms: PlayerTerms::decode(&obj, content_registry)?,
                }
            }
            other => Self::Unsupported(other.to_string()),
        })
    }
}

/// `LocationPredicate.CODEC`. The terms that need a level fact this server
/// cannot read are kept so the evaluator can refuse them where vanilla reads
/// them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocationPredicate {
    pub position: Option<[DoubleBounds; 3]>,
    pub biomes: Option<HolderSet>,
    pub structures: Option<HolderSet>,
    pub dimension: Option<String>,
    pub smokey: Option<bool>,
    pub light: Option<IntBounds>,
    pub block: Option<BlockPredicate>,
    pub fluid: Option<FluidPredicate>,
    pub can_see_sky: Option<bool>,
}

/// `BlockPredicate.CODEC` (`net/minecraft/advancements/criterion`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BlockPredicate {
    pub blocks: Option<HolderSet>,
    pub state: Option<StatePropertiesPredicate>,
    pub nbt: Option<NbtSummaryValue>,
    pub components: Vec<(String, NbtSummaryValue)>,
    pub predicates: Vec<ComponentPredicate>,
}

/// `FluidPredicate.CODEC`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FluidPredicate {
    pub fluids: Option<HolderSet>,
    pub state: Option<StatePropertiesPredicate>,
}

impl LocationPredicate {
    pub fn decode(value: V<'_>) -> Result<Self> {
        Self::decode_with_content_registry(value, ContentRegistry::vanilla())
    }

    pub fn decode_with_content_registry(
        value: V<'_>,
        content_registry: &ContentRegistry,
    ) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            position: obj.opt("position", |value| {
                let obj = Obj::of(value)?;
                let axis = |name| obj.or(name, DoubleBounds::default(), DoubleBounds::decode);
                Ok([axis("x")?, axis("y")?, axis("z")?])
            })?,
            biomes: obj.opt("biomes", HolderSet::decode)?,
            structures: obj.opt("structures", HolderSet::decode)?,
            dimension: obj.opt("dimension", id)?,
            smokey: obj.opt("smokey", boolean)?,
            light: obj.opt("light", |value| {
                Obj::of(value)?.or("light", IntBounds::default(), IntBounds::decode)
            })?,
            block: obj.opt("block", |value| {
                let obj = Obj::of(value)?;
                Ok(BlockPredicate {
                    blocks: obj.opt("blocks", HolderSet::decode)?,
                    state: obj.opt("state", StatePropertiesPredicate::decode)?,
                    nbt: obj.opt("nbt", |value| {
                        custom_data_tag(value).map(NbtSummaryValue::Compound)
                    })?,
                    components: obj.or("components", Vec::new(), exact_components)?,
                    predicates: obj.or("predicates", Vec::new(), |value| {
                        component_predicates(value, content_registry)
                    })?,
                })
            })?,
            fluid: obj.opt("fluid", |value| {
                let obj = Obj::of(value)?;
                Ok(FluidPredicate {
                    fluids: obj.opt("fluids", HolderSet::decode)?,
                    state: obj.opt("state", StatePropertiesPredicate::decode)?,
                })
            })?,
            can_see_sky: obj.opt("can_see_sky", boolean)?,
        })
    }
}

/// `StatePropertiesPredicate.CODEC`: property name to an exact value or a range.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatePropertiesPredicate(pub Vec<(JavaString, PropertyMatcher)>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyMatcher {
    Exact(JavaString),
    Ranged {
        min: Option<JavaString>,
        max: Option<JavaString>,
    },
}

impl StatePropertiesPredicate {
    pub fn decode(value: V<'_>) -> Result<Self> {
        let properties = entries(value).map_or_else(|| fail("expected an object"), Ok)?;
        properties
            .into_iter()
            .map(|(name, matcher)| {
                let matcher = if let Some(exact) = matcher.java_string() {
                    PropertyMatcher::Exact(exact)
                } else {
                    let obj = Obj::of(matcher).at(&name)?;
                    let end = |field| obj.opt(field, java_string);
                    PropertyMatcher::Ranged {
                        min: end("min").at(&name)?,
                        max: end("max").at(&name)?,
                    }
                };
                Ok((name.clone(), matcher))
            })
            .collect::<Result<_>>()
            .map(Self)
    }
}

/// `DamageSourcePredicate.CODEC`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DamageSourcePredicate {
    /// `TagPredicate`s over the damage type: tag id and expected membership.
    pub tags: Vec<(String, bool)>,
    pub direct_entity: Option<EntityPredicate>,
    pub source_entity: Option<EntityPredicate>,
    pub is_direct: Option<bool>,
}

impl DamageSourcePredicate {
    pub fn decode(value: V<'_>) -> Result<Self> {
        Self::decode_with_content_registry(value, ContentRegistry::vanilla())
    }

    pub fn decode_with_content_registry(
        value: V<'_>,
        content_registry: &ContentRegistry,
    ) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            tags: obj.or("tags", Vec::new(), |value| {
                list(value, |value| {
                    let obj = Obj::of(value)?;
                    Ok((obj.req("id", id)?, obj.req("expected", boolean)?))
                })
            })?,
            direct_entity: obj.opt("direct_entity", |value| {
                EntityPredicate::decode_with_content_registry(value, content_registry)
            })?,
            source_entity: obj.opt("source_entity", |value| {
                EntityPredicate::decode_with_content_registry(value, content_registry)
            })?,
            is_direct: obj.opt("is_direct", boolean)?,
        })
    }
}
