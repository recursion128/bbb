//! The attribute runtime (`net/minecraft/world/entity/ai/attributes/`):
//! `AttributeInstance`, `AttributeMap`, `AttributeSupplier` and the
//! `RangedAttribute` bounds (`Attributes.java:10-103`). Ids, defaults, bounds
//! and sync flags come from `bbb_protocol::attribute_types`; per-type bases
//! from the generated [`vanilla_entity_default_attributes_26_1`].
//!
//! Iteration-order deviations: modifiers of one operation are summed in
//! insertion order where vanilla sums a hash map (`AttributeInstance.java:25`),
//! and the map walks attribute ids ascending where vanilla walks an
//! identity-hashed map (`AttributeMap.java:16`) -- an order the official server
//! itself does not reproduce between runs. `dirty`/`cachedValue` (`:29-30`) is
//! a cache and is recomputed instead.

use bbb_protocol::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
};
use bbb_protocol::attribute_types::{
    vanilla_attribute_type, vanilla_attribute_type_for_resource_id, VANILLA_ATTRIBUTE_TYPES,
};
use bbb_protocol::mob_effect_types::MobEffectTypeCatalogEntry;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::save::read_list_codec;
use bbb_facts::default_attributes_26_1::vanilla_entity_default_attributes_26_1;

/// `AttributeModifier.Operation` (`AttributeModifier.java:38-64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AttributeOperation {
    /// `ADD_VALUE("add_value", 0)`.
    AddValue,
    /// `ADD_MULTIPLIED_BASE("add_multiplied_base", 1)`.
    AddMultipliedBase,
    /// `ADD_MULTIPLIED_TOTAL("add_multiplied_total", 2)`.
    AddMultipliedTotal,
}

impl AttributeOperation {
    pub(crate) fn from_java_string(name: &bbb_protocol::JavaString) -> Option<Self> {
        name.with_utf8(Self::from_serialized_name).flatten()
    }

    /// Every operation, in `Operation.values()` order.
    pub const ALL: [Self; 3] = [
        Self::AddValue,
        Self::AddMultipliedBase,
        Self::AddMultipliedTotal,
    ];

    /// `Operation.id()` (`AttributeModifier.java:54-56`), the wire id.
    pub const fn id(self) -> i32 {
        match self {
            Self::AddValue => VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            Self::AddMultipliedBase => VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            Self::AddMultipliedTotal => {
                VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
            }
        }
    }

    /// `Operation.BY_ID` (`AttributeModifier.java:43-45`): a
    /// `ByIdMap.continuous` with `OutOfBoundsStrategy.ZERO`, so an id outside
    /// the table reads as `ADD_VALUE` rather than failing.
    pub fn by_id(id: i32) -> Self {
        Self::ALL
            .into_iter()
            .find(|operation| operation.id() == id)
            .unwrap_or(Self::AddValue)
    }

    /// `Operation.getSerializedName()` (`AttributeModifier.java:61-63`), the
    /// save and data-pack spelling.
    pub fn serialized_name(self) -> &'static str {
        VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS
            .iter()
            .find(|entry| entry.id == self.id())
            .map(|entry| entry.serialized_name)
            .expect("every operation is in the generated catalog")
    }

    /// The inverse of [`AttributeOperation::serialized_name`]: `Operation.CODEC`
    /// (`AttributeModifier.java:45`) refuses any other spelling.
    pub fn from_serialized_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operation| operation.serialized_name() == name)
    }
}

/// `AttributeModifier` (`AttributeModifier.java:14`): a named amount with the
/// operation that folds it into the attribute's value.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeModifier {
    /// `id`, an `Identifier` in canonical `namespace:path` form.
    pub id: String,
    pub amount: f64,
    pub operation: AttributeOperation,
}

/// `AttributeInstance.Packed` (`AttributeInstance.java:201-210`): the save
/// form of one instance, with only the permanent modifiers.
#[derive(Debug, Clone, PartialEq)]
pub struct PackedAttribute {
    pub attribute_id: i32,
    /// `base`, which the codec defaults to `0.0` when absent (`:205`).
    pub base_value: f64,
    /// `modifiers`, which the codec defaults to empty when absent (`:206`).
    pub modifiers: Vec<AttributeModifier>,
}

fn saved_field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn read_saved_identifier(value: Option<&NbtSummaryValue>) -> Option<String> {
    match value? {
        NbtSummaryValue::String(id) => id
            .try_to_string()
            .ok()
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok()),
        _ => None,
    }
}

impl AttributeModifier {
    /// `AttributeModifier.MAP_CODEC` (`AttributeModifier.java:15-22`), whose
    /// three fields are all plain `fieldOf`s and so are all always written.
    fn to_saved_nbt(&self) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(self.id.clone()),
            },
            NbtSummaryEntry {
                name: "amount".into(),
                value: NbtSummaryValue::Double(
                    bbb_protocol::nbt_summary::canonical_double_tag_bits(self.amount),
                ),
            },
            NbtSummaryEntry {
                name: "operation".into(),
                value: NbtSummaryValue::string(self.operation.serialized_name()),
            },
        ])
    }

    // AttributeModifier.java:15-22: all three fields are required.
    fn read_saved_codec(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(fields) = value else {
            return None;
        };
        let operation = match saved_field(fields, "operation")? {
            NbtSummaryValue::String(name) => AttributeOperation::from_java_string(name)?,
            _ => return None,
        };
        Some(Self {
            id: read_saved_identifier(saved_field(fields, "id"))?,
            amount: crate::entity::save::command_number_f64(saved_field(fields, "amount")?)?,
            operation,
        })
    }
}

impl PackedAttribute {
    /// `AttributeInstance.Packed.CODEC` (`AttributeInstance.java:202-209`).
    ///
    /// `id` and `base` are plain `fieldOf`s and are written whatever they hold;
    /// `modifiers` is an `optionalFieldOf` with an empty default (`:206`), so an
    /// instance carrying none omits the field rather than writing an empty list.
    pub fn to_saved_nbt(&self) -> NbtSummaryValue {
        let mut fields = vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(
                    vanilla_attribute_type(self.attribute_id)
                        .expect("a packed attribute names a catalog entry")
                        .resource_id
                        .to_owned(),
                ),
            },
            NbtSummaryEntry {
                name: "base".into(),
                value: NbtSummaryValue::Double(
                    bbb_protocol::nbt_summary::canonical_double_tag_bits(self.base_value),
                ),
            },
        ];
        if !self.modifiers.is_empty() {
            fields.push(NbtSummaryEntry {
                name: "modifiers".into(),
                value: NbtSummaryValue::List(
                    self.modifiers
                        .iter()
                        .map(AttributeModifier::to_saved_nbt)
                        .collect(),
                ),
            });
        }
        NbtSummaryValue::Compound(fields)
    }

    pub(crate) fn from_saved_list(values: &[NbtSummaryValue]) -> Vec<Self> {
        values.iter().filter_map(Self::read_saved_codec).collect()
    }

    pub(crate) fn read_saved_list_codec(value: &NbtSummaryValue) -> Option<Vec<Self>> {
        read_list_codec(value, Self::read_saved_codec)
    }

    pub(crate) fn read_saved_codec(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(fields) = value else {
            return None;
        };
        let attribute_id = read_saved_identifier(saved_field(fields, "id"))
            .and_then(|id| vanilla_attribute_type_for_resource_id(&id))?
            .id;
        // AttributeInstance.java:205: orElse consumes both missing-field and type errors.
        let base_value = saved_field(fields, "base")
            .and_then(crate::entity::save::command_number_f64)
            .unwrap_or(0.0);
        // AttributeInstance.java:206: optionalFieldOf keeps the decodable modifiers.
        let modifiers = saved_field(fields, "modifiers")
            .and_then(|value| read_list_codec(value, AttributeModifier::read_saved_codec))
            .unwrap_or_default();
        Some(Self {
            attribute_id,
            base_value,
            modifiers,
        })
    }
}

/// The reason [`AttributeInstance::add_transient_modifier`] and its permanent
/// twin refuse a modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeModifierError {
    /// `AttributeInstance.addModifier` (`AttributeInstance.java:72-80`) throws
    /// `IllegalArgumentException("Modifier is already applied on this
    /// attribute!")` for an id the instance already carries.
    AlreadyApplied,
}

/// `RangedAttribute.minValue`/`maxValue` (`Attributes.java:10-103`): every
/// attribute the 26.1 registry holds is a `RangedAttribute`, and the pair is
/// what `sanitizeValue` clamps to. `None` for an id outside the catalog.
pub fn attribute_bounds(attribute_id: i32) -> Option<(f64, f64)> {
    vanilla_attribute_type(attribute_id).map(|entry| {
        (
            f64::from_bits(entry.min_value_bits),
            f64::from_bits(entry.max_value_bits),
        )
    })
}

/// `Attribute.getDefaultValue()` (`Attribute.java:25-27`) off the generated
/// catalog's IEEE-754 bits.
pub fn attribute_default_value(attribute_id: i32) -> Option<f64> {
    vanilla_attribute_type(attribute_id).map(|entry| f64::from_bits(entry.default_value_bits))
}

/// `Attribute.isClientSyncable()` (`Attribute.java:29-31`).
pub fn attribute_is_syncable(attribute_id: i32) -> bool {
    vanilla_attribute_type(attribute_id).is_some_and(|entry| entry.syncable)
}

/// `RangedAttribute.sanitizeValue(double)` (`RangedAttribute.java:35-37`):
/// `NaN` collapses to the minimum, everything else is `Mth.clamp`ed
/// (`net/minecraft/util/Mth.java:105-107`, `value < min ? min :
/// Math.min(value, max)`).
///
/// An attribute outside the catalog has no `RangedAttribute` and keeps the
/// plain `Attribute.sanitizeValue` identity (`Attribute.java:43-45`); no such
/// attribute can reach a map built from the catalog, so the arm is a formality.
pub fn sanitize_attribute_value(attribute_id: i32, value: f64) -> f64 {
    let Some((min, max)) = attribute_bounds(attribute_id) else {
        return value;
    };
    if value.is_nan() || value < min {
        min
    } else {
        // `Math.min(double, double)` is `NaN`-propagating and orders `-0.0`
        // below `0.0`; neither case reaches here, `NaN` having been handled
        // and the clamp comparing magnitudes.
        value.min(max)
    }
}

/// `AttributeInstance` (`AttributeInstance.java:22-199`): one attribute's base
/// value and the modifiers on it.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeInstance {
    attribute_id: i32,
    base_value: f64,
    /// `modifierById` (`:26`), an `Object2ObjectArrayMap`, so insertion order.
    modifiers: Vec<AttributeModifier>,
    /// Membership in `permanentModifiers` (`AttributeInstance.java:24`), indexed in the order
    /// `pack()` lists the permanent modifiers (`:186`).
    permanent: Vec<usize>,
    /// Whether `setDirty()` (`:111-114`) has fired since the owning map last
    /// looked: the `onDirty` consumer, delivered late rather than inline.
    dirty_notification: bool,
}

impl AttributeInstance {
    /// `new AttributeInstance(attribute, onDirty)` (`AttributeInstance.java:30-34`):
    /// the base value starts at the attribute's registry default.
    pub fn new(attribute_id: i32) -> Self {
        Self {
            attribute_id,
            base_value: attribute_default_value(attribute_id).unwrap_or(0.0),
            modifiers: Vec::new(),
            permanent: Vec::new(),
            dirty_notification: false,
        }
    }

    /// `getAttribute()` (`:36-38`), as the catalog id.
    pub const fn attribute_id(&self) -> i32 {
        self.attribute_id
    }

    /// `getBaseValue()` (`:40-42`).
    pub const fn base_value(&self) -> f64 {
        self.base_value
    }

    /// `setBaseValue(double)` (`:44-49`): only a changed value marks dirty.
    pub fn set_base_value(&mut self, base_value: f64) {
        if base_value != self.base_value {
            self.base_value = base_value;
            self.set_dirty();
        }
    }

    /// `getModifiers()` (`:56-58`), in insertion order.
    pub fn modifiers(&self) -> &[AttributeModifier] {
        &self.modifiers
    }

    /// `getModifier(Identifier)` (`:64-66`).
    pub fn modifier(&self, id: &str) -> Option<&AttributeModifier> {
        self.modifiers.iter().find(|modifier| modifier.id == id)
    }

    /// `hasModifier(Identifier)` (`:68-70`).
    pub fn has_modifier(&self, id: &str) -> bool {
        self.modifier(id).is_some()
    }

    /// `addModifier(AttributeModifier)` (`:72-80`).
    fn add_modifier(&mut self, modifier: AttributeModifier) -> Result<(), AttributeModifierError> {
        if self.has_modifier(&modifier.id) {
            return Err(AttributeModifierError::AlreadyApplied);
        }
        self.modifiers.push(modifier);
        self.set_dirty();
        Ok(())
    }

    /// `addOrUpdateTransientModifier(AttributeModifier)` (`:82-88`).
    ///
    /// Vanilla's `modifier != oldModifier` is a reference comparison. Every
    /// caller but `Rabbit.setVariant` (`Rabbit.java:375`) passes a static
    /// constant, so value equality stands in for that identity: a per-tick
    /// repeat leaves the instance clean instead of resyncing it every tick.
    pub fn add_or_update_transient_modifier(&mut self, modifier: AttributeModifier) {
        if let Some(existing) = self.modifiers.iter_mut().find(|m| m.id == modifier.id) {
            if *existing == modifier {
                return;
            }
            *existing = modifier;
        } else {
            self.modifiers.push(modifier);
        }
        self.set_dirty();
    }

    /// `addTransientModifier(AttributeModifier)` (`:90-92`).
    pub fn add_transient_modifier(
        &mut self,
        modifier: AttributeModifier,
    ) -> Result<(), AttributeModifierError> {
        self.add_modifier(modifier)
    }

    /// `addOrReplacePermanentModifier(AttributeModifier)` (`:94-98`).
    pub fn add_or_replace_permanent_modifier(&mut self, modifier: AttributeModifier) {
        self.remove_modifier(&modifier.id);
        self.add_permanent_modifier(modifier)
            .expect("the id was just removed");
    }

    /// `addPermanentModifier(AttributeModifier)` (`:100-103`).
    pub fn add_permanent_modifier(
        &mut self,
        modifier: AttributeModifier,
    ) -> Result<(), AttributeModifierError> {
        self.add_modifier(modifier)?;
        self.permanent.push(self.modifiers.len() - 1);
        Ok(())
    }

    /// `setDirty()` (`:111-114`): the half that reaches the owning map's
    /// `onAttributeModified`, which [`AttributeMap`] collects lazily.
    fn set_dirty(&mut self) {
        self.dirty_notification = true;
    }

    /// `removeModifier(Identifier)` (`:120-130`).
    pub fn remove_modifier(&mut self, id: &str) -> bool {
        let Some(index) = self.modifiers.iter().position(|modifier| modifier.id == id) else {
            return false;
        };
        self.modifiers.remove(index);
        self.permanent.retain_mut(|permanent| {
            if *permanent == index {
                return false;
            }
            *permanent -= usize::from(*permanent > index);
            true
        });
        self.set_dirty();
        true
    }

    /// `getValue()` (`:138-145`) over `calculateValue()` (`:147-165`).
    pub fn value(&self) -> f64 {
        let mut base = self.base_value;
        for modifier in self.modifiers_of(AttributeOperation::AddValue) {
            base += modifier.amount;
        }
        let mut result = base;
        for modifier in self.modifiers_of(AttributeOperation::AddMultipliedBase) {
            result += base * modifier.amount;
        }
        for modifier in self.modifiers_of(AttributeOperation::AddMultipliedTotal) {
            result *= 1.0 + modifier.amount;
        }
        sanitize_attribute_value(self.attribute_id, result)
    }

    /// `getModifiersOrEmpty(Operation)` (`:167-169`).
    fn modifiers_of(
        &self,
        operation: AttributeOperation,
    ) -> impl Iterator<Item = &AttributeModifier> {
        self.modifiers
            .iter()
            .filter(move |modifier| modifier.operation == operation)
    }

    /// `pack()` (`:185-187`): base and permanent modifiers only.
    pub fn pack(&self) -> PackedAttribute {
        PackedAttribute {
            attribute_id: self.attribute_id,
            base_value: self.base_value,
            modifiers: self
                .permanent
                .iter()
                .map(|&index| self.modifiers[index].clone())
                .collect(),
        }
    }

    /// `apply(Packed)` (`:189-199`): the packed base replaces the current one
    /// and every packed modifier is put (not added, so a duplicate id replaces
    /// rather than throws) as permanent.
    pub fn apply(&mut self, packed: &PackedAttribute) {
        self.base_value = packed.base_value;
        for modifier in &packed.modifiers {
            let index = if let Some(index) = self.modifiers.iter().position(|m| m.id == modifier.id)
            {
                self.modifiers[index] = modifier.clone();
                index
            } else {
                self.modifiers.push(modifier.clone());
                self.modifiers.len() - 1
            };
            if !self.permanent.contains(&index) {
                self.permanent.push(index);
            }
        }
        self.set_dirty();
    }
}

/// `AttributeSupplier` (`AttributeSupplier.java:10-95`): the per-type template
/// an `AttributeMap` creates its instances from, built by
/// `DefaultAttributes` (`DefaultAttributes.java:189-191`) from each type's
/// `createAttributes` builder.
///
/// The builder calls are the generated default-attribute table: a `None` base
/// is `Builder.add(Holder)` (`AttributeSupplier.java:81-84`), which takes the
/// attribute's registry default, and a `Some` base is
/// `Builder.add(Holder, double)` (`:86-90`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttributeSupplier {
    defaults: &'static [(i32, Option<f64>)],
}

impl AttributeSupplier {
    /// `DefaultAttributes.getSupplier(EntityType)` (`DefaultAttributes.java:189-191`)
    /// for the type named by `resource_id`; `None` where `DefaultAttributes`
    /// has no row, which is every non-living type.
    pub fn for_entity_type(resource_id: &str) -> Option<Self> {
        // The generated table has an empty row for every non-living type;
        // `DefaultAttributes.hasSupplier` (`DefaultAttributes.java:193-195`)
        // is false for those.
        vanilla_entity_default_attributes_26_1(resource_id)
            .filter(|defaults| !defaults.is_empty())
            .map(|defaults| Self { defaults })
    }

    /// `hasAttribute(Holder)` (`AttributeSupplier.java:58-60`).
    pub fn has_attribute(&self, attribute_id: i32) -> bool {
        self.defaults.iter().any(|(id, _)| *id == attribute_id)
    }

    /// `getBaseValue(Holder)` (`:30-32`): the template's base. Vanilla throws
    /// `IllegalArgumentException` for an attribute the supplier lacks
    /// (`:17-24`); that is `None` here.
    pub fn base_value(&self, attribute_id: i32) -> Option<f64> {
        self.defaults
            .iter()
            .find(|(id, _)| *id == attribute_id)
            .map(|(id, base)| base.unwrap_or_else(|| attribute_default_value(*id).unwrap_or(0.0)))
    }

    /// `getValue(Holder)` (`:26-28`): the template carries no modifiers, so
    /// this is the sanitized base.
    pub fn value(&self, attribute_id: i32) -> Option<f64> {
        self.base_value(attribute_id)
            .map(|base| sanitize_attribute_value(attribute_id, base))
    }

    /// `createInstance(Consumer, Holder)` (`:43-52`): a fresh dirty instance
    /// with the template's base.
    pub fn create_instance(&self, attribute_id: i32) -> Option<AttributeInstance> {
        let base = self.base_value(attribute_id)?;
        let mut instance = AttributeInstance::new(attribute_id);
        instance.base_value = base;
        instance.set_dirty();
        Some(instance)
    }
}

/// `AttributeMap` (`AttributeMap.java:16-150`): a `LivingEntity`'s attributes,
/// instantiated lazily from its [`AttributeSupplier`].
///
/// `getInstance` is the only creator (`:45-47`), and creation runs
/// `replaceFrom` on the new instance, so the act of asking for an instance is
/// itself a modification the map records (`:26-31`). Reading a value never
/// creates one (`:58-61`), which is why a freshly constructed entity saves an
/// empty `attributes` list (`:132-140`).
const ATTRIBUTE_COUNT: usize = VANILLA_ATTRIBUTE_TYPES.len();

#[derive(Debug, Clone)]
pub struct AttributeIdIter(u64);

impl Iterator for AttributeIdIter {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let id = self.0.trailing_zeros();
        self.0 &= self.0 - 1;
        Some(id as i32)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.count_ones() as usize;
        (len, Some(len))
    }
}

impl ExactSizeIterator for AttributeIdIter {}

#[derive(Debug, Clone)]
pub struct AttributeMap {
    supplier: AttributeSupplier,
    /// Dense slot index into `instances` keyed by `attribute_id`, or `u8::MAX` if absent.
    slots: [u8; ATTRIBUTE_COUNT],
    instances: Vec<AttributeInstance>,
    present_mask: u64,
    /// `attributesToSync` (`:18`).
    attributes_to_sync: u64,
    /// `attributesToUpdate` (`:19`).
    attributes_to_update: u64,
}

impl PartialEq for AttributeMap {
    fn eq(&self, other: &Self) -> bool {
        self.supplier == other.supplier
            && self.present_mask == other.present_mask
            && self.attributes_to_sync == other.attributes_to_sync
            && self.attributes_to_update == other.attributes_to_update
            && AttributeIdIter(self.present_mask).all(|id| self.instance(id) == other.instance(id))
    }
}

impl AttributeMap {
    /// `new AttributeMap(AttributeSupplier)` (`AttributeMap.java:22-24`).
    pub fn new(supplier: AttributeSupplier) -> Self {
        Self {
            supplier,
            slots: [u8::MAX; ATTRIBUTE_COUNT],
            instances: Vec::new(),
            present_mask: 0,
            attributes_to_sync: 0,
            attributes_to_update: 0,
        }
    }

    #[inline]
    fn slot_index(attribute_id: i32) -> Option<usize> {
        usize::try_from(attribute_id)
            .ok()
            .filter(|&id| id < ATTRIBUTE_COUNT)
    }

    pub const fn supplier(&self) -> AttributeSupplier {
        self.supplier
    }

    /// `onAttributeModified(AttributeInstance)` (`:26-31`), delivered for every
    /// instance whose `setDirty` has fired since the last delivery.
    fn collect_modifications(&mut self) {
        for instance in &mut self.instances {
            if std::mem::take(&mut instance.dirty_notification) {
                let bit = 1u64 << (instance.attribute_id as u32);
                self.attributes_to_update |= bit;
                if attribute_is_syncable(instance.attribute_id) {
                    self.attributes_to_sync |= bit;
                }
            }
        }
    }

    /// `getAttributesToSync()` (`:33-35`) followed by the caller's `clear()`
    /// (`ServerEntity.java:212`): the syncable instances modified since the
    /// last drain, ascending by attribute id.
    pub fn drain_attributes_to_sync(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &AttributeInstance> {
        self.collect_modifications();
        let ids = std::mem::take(&mut self.attributes_to_sync);
        let slots = &self.slots;
        let instances = &self.instances;
        AttributeIdIter(ids).map(move |id| &instances[slots[id as usize] as usize])
    }

    /// `getAttributesToUpdate()` (`:37-39`) followed by the caller's `clear()`
    /// (`LivingEntity.java:1109`): the attribute ids modified since the last
    /// drain, ascending.
    pub fn drain_attributes_to_update(&mut self) -> AttributeIdIter {
        self.collect_modifications();
        AttributeIdIter(std::mem::take(&mut self.attributes_to_update))
    }

    /// `getSyncableAttributes()` (`:41-43`): every instantiated attribute the
    /// client is told about, ascending by attribute id.
    pub fn syncable_attributes(&self) -> impl Iterator<Item = &AttributeInstance> {
        self.instances()
            .filter(|instance| attribute_is_syncable(instance.attribute_id))
    }

    /// `getInstance(Holder)` (`:45-47`): the instance, created from the
    /// supplier on first use; `None` for an attribute the supplier lacks.
    pub fn get_instance(&mut self, attribute_id: i32) -> Option<&mut AttributeInstance> {
        let id = Self::slot_index(attribute_id)?;
        if self.slots[id] == u8::MAX {
            let instance = self.supplier.create_instance(attribute_id)?;
            let index = self.instances.len() as u8;
            self.instances.push(instance);
            self.slots[id] = index;
            self.present_mask |= 1u64 << id;
        }
        Some(&mut self.instances[self.slots[id] as usize])
    }

    // LivingEntity.java:155-158,2271-2278: sprinting is a transient total speed multiplier.
    pub(crate) fn set_sprinting_modifier(&mut self, sprinting: bool) {
        let speed = self
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .expect("the sprinting entity supplier declares movement speed");
        if sprinting
            && speed
                .modifiers
                .iter()
                .position(|modifier| modifier.id == "minecraft:sprinting")
                .is_some_and(|index| {
                    let modifier = &speed.modifiers[index];
                    modifier.amount == f64::from(0.3_f32)
                        && modifier.operation == AttributeOperation::AddMultipliedTotal
                        && !speed.permanent.contains(&index)
                })
        {
            return;
        }
        speed.remove_modifier("minecraft:sprinting");
        if sprinting {
            speed.add_or_update_transient_modifier(AttributeModifier {
                id: "minecraft:sprinting".to_owned(),
                amount: f64::from(0.3_f32),
                operation: AttributeOperation::AddMultipliedTotal,
            });
        }
    }

    /// `MobEffect.removeAttributeModifiers`/`addAttributeModifiers`
    /// (`MobEffect.java:136-173`): the generated effect descriptors are the
    /// sole attribute-modifier inventory.
    pub(crate) fn replace_mob_effect_modifiers(
        &mut self,
        effect: &MobEffectTypeCatalogEntry,
        amplifier: i32,
    ) {
        for descriptor in effect.attribute_modifiers {
            let Some(instance) = self.get_instance(descriptor.attribute_id) else {
                continue;
            };
            instance.remove_modifier(descriptor.modifier_id);
            instance
                .add_permanent_modifier(AttributeModifier {
                    id: descriptor.modifier_id.to_owned(),
                    amount: f64::from_bits(descriptor.amount_bits) * f64::from(amplifier + 1),
                    operation: AttributeOperation::by_id(descriptor.operation_id),
                })
                .expect("the generated modifier id was just removed");
        }
    }

    pub(crate) fn remove_mob_effect_modifiers(&mut self, effect: &MobEffectTypeCatalogEntry) {
        for descriptor in effect.attribute_modifiers {
            if let Some(instance) = self.instance_mut(descriptor.attribute_id) {
                instance.remove_modifier(descriptor.modifier_id);
            }
        }
    }

    /// The instance if it has already been created, without creating it:
    /// `this.attributes.get(attribute)` as `getValue` and friends use it.
    pub fn instance(&self, attribute_id: i32) -> Option<&AttributeInstance> {
        let id = Self::slot_index(attribute_id)?;
        let slot = self.slots[id];
        (slot != u8::MAX).then(|| &self.instances[slot as usize])
    }

    /// Mutable borrow of an already-created instance without lazily creating
    /// or dirtying a new entry from the supplier.
    pub fn instance_mut(&mut self, attribute_id: i32) -> Option<&mut AttributeInstance> {
        let id = Self::slot_index(attribute_id)?;
        let slot = self.slots[id];
        (slot != u8::MAX).then(|| &mut self.instances[slot as usize])
    }

    /// `hasAttribute(Holder)` (`:49-51`).
    pub fn has_attribute(&self, attribute_id: i32) -> bool {
        Self::slot_index(attribute_id).is_some_and(|id| (self.present_mask & (1u64 << id)) != 0)
            || self.supplier.has_attribute(attribute_id)
    }

    /// `hasModifier(Holder, Identifier)` (`:53-56`); the supplier's templates
    /// never carry modifiers.
    pub fn has_modifier(&self, attribute_id: i32, modifier_id: &str) -> bool {
        self.instance(attribute_id)
            .is_some_and(|instance| instance.has_modifier(modifier_id))
    }

    /// `getValue(Holder)` (`:58-61`): the own instance's value when one exists,
    /// the supplier's otherwise. `None` is the supplier's
    /// `IllegalArgumentException` for an attribute this type does not have.
    pub fn value(&self, attribute_id: i32) -> Option<f64> {
        match self.instance(attribute_id) {
            Some(instance) => Some(instance.value()),
            None => self.supplier.value(attribute_id),
        }
    }

    /// `getBaseValue(Holder)` (`:63-66`).
    pub fn base_value(&self, attribute_id: i32) -> Option<f64> {
        match self.instance(attribute_id) {
            Some(instance) => Some(instance.base_value()),
            None => self.supplier.base_value(attribute_id),
        }
    }

    /// `resetBaseValue(Holder)` (`:119-130`).
    pub fn reset_base_value(&mut self, attribute_id: i32) -> bool {
        let Some(base) = self.supplier.base_value(attribute_id) else {
            return false;
        };
        if let Some(id) = Self::slot_index(attribute_id) {
            let slot = self.slots[id];
            if slot != u8::MAX {
                self.instances[slot as usize].set_base_value(base);
            }
        }
        true
    }

    /// `pack()` (`:132-140`): every instantiated attribute, ascending by id.
    pub fn pack(&self) -> Vec<PackedAttribute> {
        self.instances().map(AttributeInstance::pack).collect()
    }

    /// `apply(List<Packed>)` (`:142-149`): a packed attribute the supplier
    /// does not declare is skipped, because `getInstance` returns null for it.
    pub fn apply(&mut self, packed_attributes: &[PackedAttribute]) {
        for packed in packed_attributes {
            if let Some(instance) = self.get_instance(packed.attribute_id) {
                instance.apply(packed);
            }
        }
    }

    /// Every instantiated attribute, ascending by id.
    pub fn instances(&self) -> impl Iterator<Item = &AttributeInstance> {
        let slots = &self.slots;
        let instances = &self.instances;
        AttributeIdIter(self.present_mask).map(move |id| &instances[slots[id as usize] as usize])
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
        VANILLA_ATTRIBUTE_GRAVITY_ID, VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
        VANILLA_ATTRIBUTE_LUCK_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
        VANILLA_ATTRIBUTE_TYPES,
    };

    use super::*;

    fn modifier(id: &str, amount: f64, operation: AttributeOperation) -> AttributeModifier {
        AttributeModifier {
            id: id.to_owned(),
            amount,
            operation,
        }
    }

    #[test]
    fn every_catalog_attribute_has_bounds_that_admit_its_default() {
        // `RangedAttribute`'s constructor (`RangedAttribute.java:10-24`)
        // throws when the default lies outside `[min, max]`, so the
        // generated defaults double as a check on the transcribed table.
        assert_eq!(VANILLA_ATTRIBUTE_TYPES.len(), 35);
        for entry in VANILLA_ATTRIBUTE_TYPES {
            let (min, max) = attribute_bounds(entry.id)
                .unwrap_or_else(|| panic!("no bounds for {}", entry.resource_id));
            let default = f64::from_bits(entry.default_value_bits);
            assert!(min <= max, "{}", entry.resource_id);
            assert!(
                (min..=max).contains(&default),
                "{} default {default} outside [{min}, {max}]",
                entry.resource_id
            );
        }
        assert_eq!(attribute_bounds(35), None);
    }

    #[test]
    fn sanitize_clamps_and_collapses_nan_to_the_minimum() {
        // `RangedAttribute.sanitizeValue` (`RangedAttribute.java:35-37`).
        assert_eq!(
            sanitize_attribute_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, f64::NAN),
            1.0
        );
        assert_eq!(
            sanitize_attribute_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 0.0),
            1.0
        );
        assert_eq!(
            sanitize_attribute_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 2000.0),
            1024.0
        );
        assert_eq!(
            sanitize_attribute_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 20.0),
            20.0
        );
        assert_eq!(
            sanitize_attribute_value(VANILLA_ATTRIBUTE_GRAVITY_ID, -3.0),
            -1.0
        );
        assert_eq!(
            sanitize_attribute_value(VANILLA_ATTRIBUTE_LUCK_ID, f64::NAN),
            -1024.0
        );
    }

    #[test]
    fn operations_round_trip_through_the_generated_catalog() {
        // `AttributeModifier.Operation` (`AttributeModifier.java:38-64`).
        for operation in AttributeOperation::ALL {
            assert_eq!(AttributeOperation::by_id(operation.id()), operation);
            assert_eq!(
                AttributeOperation::from_serialized_name(operation.serialized_name()),
                Some(operation)
            );
        }
        assert_eq!(AttributeOperation::AddValue.serialized_name(), "add_value");
        assert_eq!(
            AttributeOperation::AddMultipliedBase.serialized_name(),
            "add_multiplied_base"
        );
        assert_eq!(
            AttributeOperation::AddMultipliedTotal.serialized_name(),
            "add_multiplied_total"
        );
        // `OutOfBoundsStrategy.ZERO` (`:42-44`).
        assert_eq!(AttributeOperation::by_id(7), AttributeOperation::AddValue);
        assert_eq!(AttributeOperation::from_serialized_name("multiply"), None);
    }

    #[test]
    fn instance_value_folds_the_three_operations_in_order() {
        // `AttributeInstance.calculateValue` (`AttributeInstance.java:147-165`):
        // additive first, then multiplied-base against the *summed* base, then
        // multiplied-total compounding.
        let mut instance = AttributeInstance::new(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
        assert_eq!(instance.base_value(), 0.7);
        instance.set_base_value(0.1);
        instance
            .add_transient_modifier(modifier("minecraft:a", 0.1, AttributeOperation::AddValue))
            .unwrap();
        instance
            .add_transient_modifier(modifier(
                "minecraft:b",
                0.5,
                AttributeOperation::AddMultipliedBase,
            ))
            .unwrap();
        instance
            .add_transient_modifier(modifier(
                "minecraft:c",
                1.0,
                AttributeOperation::AddMultipliedTotal,
            ))
            .unwrap();
        // base 0.2; +0.2*0.5 = 0.3; *(1+1) = 0.6
        let expected = ((0.1_f64 + 0.1) + (0.1 + 0.1) * 0.5) * 2.0;
        assert_eq!(instance.value(), expected);
        // A second modifier with the same id is refused (`:72-80`).
        assert_eq!(
            instance.add_transient_modifier(modifier(
                "minecraft:a",
                5.0,
                AttributeOperation::AddValue
            )),
            Err(AttributeModifierError::AlreadyApplied)
        );
        assert!(instance.remove_modifier("minecraft:c"));
        assert!(!instance.remove_modifier("minecraft:c"));
        assert_eq!(instance.value(), 0.1 + 0.1 + (0.1 + 0.1) * 0.5);
    }

    /// `AttributeInstance.Packed.CODEC` (`AttributeInstance.java:202-209`).
    #[test]
    fn a_packed_attribute_round_trips_through_the_saved_codec() {
        let packed = PackedAttribute {
            attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            base_value: 20.0,
            modifiers: vec![
                modifier(
                    "minecraft:effect.health_boost",
                    4.0,
                    AttributeOperation::AddValue,
                ),
                modifier(
                    "example:scaled",
                    0.5,
                    AttributeOperation::AddMultipliedTotal,
                ),
            ],
        };
        assert_eq!(
            PackedAttribute::read_saved_codec(&packed.to_saved_nbt()).unwrap(),
            packed
        );

        let bare = PackedAttribute {
            modifiers: Vec::new(),
            ..packed
        };
        let NbtSummaryValue::Compound(fields) = bare.to_saved_nbt() else {
            panic!("a packed attribute is a compound");
        };
        assert_eq!(
            fields.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
            ["id", "base"],
            "an empty modifier list is the codec default and is not written",
        );
        assert_eq!(
            PackedAttribute::read_saved_codec(&bare.to_saved_nbt()).unwrap(),
            bare
        );

        assert!(
            PackedAttribute::read_saved_codec(&NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("example:not_an_attribute"),
            }]))
            .is_none()
        );
    }

    #[test]
    fn pack_keeps_only_permanent_modifiers_and_apply_puts_them_back() {
        // `AttributeInstance.pack`/`apply` (`AttributeInstance.java:185-199`).
        let mut instance = AttributeInstance::new(VANILLA_ATTRIBUTE_ARMOR_ID);
        instance
            .add_transient_modifier(modifier("minecraft:t", 1.0, AttributeOperation::AddValue))
            .unwrap();
        instance
            .add_permanent_modifier(modifier("minecraft:p", 2.0, AttributeOperation::AddValue))
            .unwrap();
        instance.add_or_replace_permanent_modifier(modifier(
            "minecraft:p",
            3.0,
            AttributeOperation::AddValue,
        ));
        assert!(instance.remove_modifier("minecraft:t"));
        let packed = instance.pack();
        assert_eq!(packed.attribute_id, VANILLA_ATTRIBUTE_ARMOR_ID);
        assert_eq!(packed.base_value, 0.0);
        assert_eq!(
            packed.modifiers,
            vec![modifier("minecraft:p", 3.0, AttributeOperation::AddValue)]
        );

        let mut restored = AttributeInstance::new(VANILLA_ATTRIBUTE_ARMOR_ID);
        restored.apply(&packed);
        assert_eq!(restored.value(), 3.0);
        assert_eq!(restored.pack().modifiers, packed.modifiers);
        // `apply` puts rather than adds (`:193-195`): a repeat does not throw
        // and does not double the modifier.
        restored.apply(&packed);
        assert_eq!(restored.modifiers().len(), 1);
        assert_eq!(restored.value(), 3.0);
    }

    #[test]
    fn supplier_comes_from_the_generated_default_table() {
        // `DefaultAttributes` row 97 is `ArmorStand.createAttributes()`:
        // `createLivingAttributes()` (`LivingEntity.java:318-341`) plus
        // `.add(Attributes.STEP_HEIGHT, 0.0)` (`ArmorStand.java:100-102`).
        let supplier = AttributeSupplier::for_entity_type("minecraft:armor_stand").unwrap();
        assert_eq!(supplier.defaults.len(), 21);
        assert_eq!(
            supplier.base_value(VANILLA_ATTRIBUTE_STEP_HEIGHT_ID),
            Some(0.0)
        );
        assert_eq!(
            supplier.base_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID),
            Some(20.0)
        );
        assert_eq!(supplier.value(VANILLA_ATTRIBUTE_GRAVITY_ID), Some(0.08));
        assert_eq!(
            supplier.base_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID),
            None
        );
        assert!(!supplier.has_attribute(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID));
        assert_eq!(AttributeSupplier::for_entity_type("minecraft:item"), None);
    }

    #[test]
    fn map_reads_never_instantiate_and_get_instance_marks_dirty() {
        // `AttributeMap.getValue` (`AttributeMap.java:58-61`) falls through to
        // the supplier; `getInstance` (`:45-47`) creates and, via
        // `replaceFrom` (`AttributeSupplier.java:43-52`,
        // `AttributeInstance.java:171-183`), fires `onAttributeModified`
        // (`AttributeMap.java:26-31`).
        let supplier = AttributeSupplier::for_entity_type("minecraft:armor_stand").unwrap();
        let mut map = AttributeMap::new(supplier);
        assert_eq!(map.value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID), Some(20.0));
        assert!(map.pack().is_empty());
        assert!(map.syncable_attributes().next().is_none());
        assert!(map.drain_attributes_to_sync().next().is_none());
        assert!(map.drain_attributes_to_update().next().is_none());

        // MAX_HEALTH is syncable, KNOCKBACK_RESISTANCE is not
        // (`Attributes.java:58-62`).
        map.get_instance(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
            .unwrap();
        map.get_instance(VANILLA_ATTRIBUTE_MAX_HEALTH_ID).unwrap();
        assert!(map
            .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
            .is_none());
        assert_eq!(
            map.drain_attributes_to_update().collect::<Vec<_>>(),
            vec![
                VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                VANILLA_ATTRIBUTE_MAX_HEALTH_ID
            ]
        );
        assert_eq!(
            map.drain_attributes_to_sync()
                .map(AttributeInstance::attribute_id)
                .collect::<Vec<_>>(),
            [VANILLA_ATTRIBUTE_MAX_HEALTH_ID]
        );
        assert!(map.drain_attributes_to_sync().next().is_none());

        map.get_instance(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap()
            .set_base_value(40.0);
        assert_eq!(map.value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID), Some(40.0));
        assert_eq!(
            map.drain_attributes_to_update().collect::<Vec<_>>(),
            vec![VANILLA_ATTRIBUTE_MAX_HEALTH_ID]
        );
        // An unchanged base does not re-mark (`AttributeInstance.java:44-49`).
        map.get_instance(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap()
            .set_base_value(40.0);
        assert!(map.drain_attributes_to_update().next().is_none());

        assert_eq!(map.pack().len(), 2);
        assert!(map.reset_base_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID));
        assert_eq!(map.value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID), Some(20.0));
        assert!(!map.reset_base_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID));
    }

    #[test]
    fn map_apply_instantiates_what_the_supplier_declares() {
        // `AttributeMap.apply` (`AttributeMap.java:142-149`).
        let supplier = AttributeSupplier::for_entity_type("minecraft:armor_stand").unwrap();
        let mut map = AttributeMap::new(supplier);
        map.apply(&[
            PackedAttribute {
                attribute_id: VANILLA_ATTRIBUTE_ARMOR_ID,
                base_value: 4.0,
                modifiers: vec![modifier("minecraft:m", 1.0, AttributeOperation::AddValue)],
            },
            PackedAttribute {
                attribute_id: VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                base_value: 9.0,
                modifiers: Vec::new(),
            },
        ]);
        assert_eq!(map.value(VANILLA_ATTRIBUTE_ARMOR_ID), Some(5.0));
        assert_eq!(map.value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID), None);
        let packed = map.pack();
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0].base_value, 4.0);
        assert_eq!(packed[0].modifiers.len(), 1);
        assert_eq!(map.syncable_attributes().count(), 1);
    }
}

#[cfg(test)]
mod saved_codec_tests;
