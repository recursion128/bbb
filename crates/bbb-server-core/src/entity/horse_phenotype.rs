use crate::random::GameRandom;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, VANILLA_HORSE_VARIANT_DATA_ID,
    VANILLA_LLAMA_STRENGTH_DATA_ID, VANILLA_LLAMA_VARIANT_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_MULE_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_bool, optional_int};
use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HorsePhenotypeState {
    Horse {
        packed_variant: i32,
    },
    Chested {
        has_chest: bool,
    },
    Llama {
        has_chest: bool,
        strength: i32,
        variant: i32,
    },
}

impl HorsePhenotypeState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        match entity_type_id {
            VANILLA_ENTITY_TYPE_HORSE_ID => Some(Self::horse(0)),
            VANILLA_ENTITY_TYPE_DONKEY_ID | VANILLA_ENTITY_TYPE_MULE_ID => {
                Some(Self::chested(false))
            }
            VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
                Some(Self::llama(false, 1, 0))
            }
            _ => None,
        }
    }

    pub(crate) const fn horse(packed_variant: i32) -> Self {
        Self::Horse { packed_variant }
    }

    pub(crate) const fn chested(has_chest: bool) -> Self {
        Self::Chested { has_chest }
    }

    pub(crate) const fn llama(has_chest: bool, strength: i32, variant: i32) -> Self {
        Self::Llama {
            has_chest,
            strength,
            variant,
        }
    }

    pub(crate) fn inventory_size(self) -> usize {
        match self {
            Self::Horse { .. } => 0,
            Self::Chested { has_chest: true } => 15,
            Self::Llama {
                has_chest: true,
                strength,
                ..
            } => (strength.clamp(1, 5) * 3) as usize,
            _ => 0,
        }
    }

    const fn same_kind(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Horse { .. }, Self::Horse { .. })
                | (Self::Chested { .. }, Self::Chested { .. })
                | (Self::Llama { .. }, Self::Llama { .. })
        )
    }
}

pub(crate) fn set_state(entity: &mut Entity, state: HorsePhenotypeState) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(current) = data.horse_phenotype() else {
        return false;
    };
    if !current.same_kind(state) || current == state {
        return false;
    }
    let dirty = match (current, state) {
        (HorsePhenotypeState::Horse { .. }, HorsePhenotypeState::Horse { .. }) => {
            [Some(VANILLA_HORSE_VARIANT_DATA_ID), None, None]
        }
        (HorsePhenotypeState::Chested { .. }, HorsePhenotypeState::Chested { .. }) => [
            Some(VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID),
            None,
            None,
        ],
        (
            HorsePhenotypeState::Llama {
                has_chest: old_chest,
                strength: old_strength,
                variant: old_variant,
            },
            HorsePhenotypeState::Llama {
                has_chest,
                strength,
                variant,
            },
        ) => [
            (old_chest != has_chest).then_some(VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID),
            (old_strength != strength).then_some(VANILLA_LLAMA_STRENGTH_DATA_ID),
            (old_variant != variant).then_some(VANILLA_LLAMA_VARIANT_DATA_ID),
        ],
        _ => return false,
    };
    data.horse_state_mut().expect("a horse phenotype").phenotype = Some(state);
    for data_id in dirty.into_iter().flatten() {
        entity.mark_synched_dirty(data_id);
    }
    true
}

pub(crate) fn set_command_chest(
    entity: &mut Entity,
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
) -> Result<bool, String> {
    // AbstractChestedHorse.java:112-139: accepted no-ops do not recreate inventory.
    let has_chest = if stack.is_empty() {
        false
    } else if stack.item_resource_id() == Some("minecraft:chest") {
        true
    } else {
        return Ok(false);
    };
    let EntityData::GeneralLiving(data) = entity.data() else {
        return Ok(false);
    };
    let state = match data.horse_phenotype() {
        Some(HorsePhenotypeState::Chested { .. }) => HorsePhenotypeState::chested(has_chest),
        Some(HorsePhenotypeState::Llama {
            strength, variant, ..
        }) => HorsePhenotypeState::llama(has_chest, strength, variant),
        _ => return Ok(false),
    };
    if data.horse_phenotype() == Some(state) {
        return Ok(true);
    }
    let mut inventory = data
        .internal_inventory()
        .ok_or_else(|| "chested horse has no internal inventory".to_owned())?
        .clone();
    // AbstractHorse.java:299-312: copy prefix through SimpleContainer.setItem, without drops.
    inventory.resize_for_horse(state);
    for slot in 0..inventory.slots().len() {
        let Some(stack) = inventory.item(slot).cloned() else {
            continue;
        };
        let max = crate::entity::item::stack_max_size(&stack)
            .ok_or_else(|| "horse cargo stack size cannot be resolved".to_owned())?
            .min(99);
        let stack = stack.with_count(crate::entity::item::stack_count(&stack).min(max));
        inventory.set_item(slot, Some(stack));
    }
    if !set_state(entity, state) {
        return Err("chested horse state could not be updated".into());
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!("set_state preserves entity data kind")
    };
    *data
        .internal_inventory_mut()
        .expect("checked before chest mutation") = inventory;
    Ok(true)
}

pub(crate) fn finalize_spawn_phenotype(entity: &mut Entity, random: &GameRandom) -> bool {
    // Concrete phenotype draws precede the dynamically dispatched AbstractHorse
    // attribute hook (`Horse.java:237-250`, `AbstractHorse.java:1018-1026`).
    match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_HORSE_ID => {
            finalize_group_horse_phenotype(entity, random, None).is_some()
        }
        VANILLA_ENTITY_TYPE_DONKEY_ID | VANILLA_ENTITY_TYPE_MULE_ID => {
            randomize_chested_horse_attributes(entity, random)
        }
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => set_attribute(
            // `SkeletonHorse.java:59-61`.
            entity,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
            generate_jump_strength(random),
        ),
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            // `ZombieHorse.java:73-87`.
            let jump = 0.5
                + random.next_double() * 0.066_666_666_666_666_67
                + random.next_double() * 0.066_666_666_666_666_67
                + random.next_double() * 0.066_666_666_666_666_67;
            let speed = (9.0 + random.next_double() + random.next_double() + random.next_double())
                / f64::from(42.16_f32);
            set_attribute(
                entity,
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
                jump,
            ) && set_attribute(
                entity,
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                speed,
            )
        }
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            finalize_group_llama_phenotype(entity, random, None).is_some()
        }
        _ => false,
    }
}

pub(crate) fn finalize_group_horse_phenotype(
    entity: &mut Entity,
    random: &GameRandom,
    group_variant: Option<i32>,
) -> Option<i32> {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_HORSE_ID {
        return None;
    }
    // `Horse.java:237-250`; the group shares only Variant, never Markings.
    let variant = group_variant.unwrap_or_else(|| random.next_int(7));
    let markings = random.next_int(5);
    let state = HorsePhenotypeState::horse(variant | markings << 8);
    let already_current = matches!(entity.data(), EntityData::GeneralLiving(data) if data.horse_phenotype() == Some(state));
    ((already_current || set_state(entity, state)) && randomize_horse_attributes(entity, random))
        .then_some(variant)
}

pub(crate) fn finalize_group_llama_phenotype(
    entity: &mut Entity,
    random: &GameRandom,
    group_variant: Option<i32>,
) -> Option<i32> {
    if !matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
    ) {
        return None;
    }
    // `Llama.java:92-95,241-255`: every member draws strength before the
    // first member selects the shared variant.
    let max_strength = if random.next_float() < 0.04 { 5 } else { 3 };
    let strength = 1 + random.next_int(max_strength);
    let variant = group_variant.unwrap_or_else(|| random.next_int(4));
    let state = HorsePhenotypeState::llama(false, strength, variant);
    let already_current = matches!(entity.data(), EntityData::GeneralLiving(data) if data.horse_phenotype() == Some(state));
    ((already_current || set_state(entity, state))
        && randomize_chested_horse_attributes(entity, random))
    .then_some(variant)
}

fn randomize_horse_attributes(entity: &mut Entity, random: &GameRandom) -> bool {
    let max_health = 15.0 + f64::from(random.next_int(8) + random.next_int(9));
    let speed = (f64::from(0.45_f32)
        + random.next_double() * 0.3
        + random.next_double() * 0.3
        + random.next_double() * 0.3)
        * 0.25;
    let jump = generate_jump_strength(random);
    set_attribute(
        entity,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        max_health,
    ) && set_attribute(
        entity,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        speed,
    ) && set_attribute(
        entity,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
        jump,
    )
}

fn randomize_chested_horse_attributes(entity: &mut Entity, random: &GameRandom) -> bool {
    set_attribute(
        entity,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        15.0 + f64::from(random.next_int(8) + random.next_int(9)),
    )
}

fn generate_jump_strength(random: &GameRandom) -> f64 {
    f64::from(0.4_f32)
        + random.next_double() * 0.2
        + random.next_double() * 0.2
        + random.next_double() * 0.2
}

fn set_attribute(entity: &mut Entity, attribute_id: i32, value: f64) -> bool {
    let Some(living) = entity.living_mut() else {
        return false;
    };
    let Some(attribute) = living.attributes_mut().get_instance(attribute_id) else {
        return false;
    };
    attribute.set_base_value(value);
    true
}

/// What `getBreedOffspring` writes onto an equine child besides its type
/// (`Horse.java:186-222`, `Donkey.java:59-67`, `Llama.java:318-333`).
pub(crate) struct EquineOffspring {
    state: Option<HorsePhenotypeState>,
    /// `MAX_HEALTH`, `JUMP_STRENGTH`, `MOVEMENT_SPEED` (`AbstractHorse.java:825-829`).
    attributes: [(i32, f64); 3],
}

impl EquineOffspring {
    pub(crate) fn apply(&self, child: &mut Entity) {
        if let Some(state) = self.state {
            let _ = set_state(child, state);
        }
        for (attribute_id, value) in self.attributes {
            let _ = set_attribute(child, attribute_id, value);
        }
    }
}

/// The equine half of `getBreedOffspring` for `body` mating with `partner`;
/// `None` for a pair that is not two equines.
pub(crate) fn equine_offspring(
    body: &Entity,
    partner: &Entity,
    random: &GameRandom,
) -> Option<EquineOffspring> {
    let (EntityData::GeneralLiving(body_data), EntityData::GeneralLiving(partner_data)) =
        (body.data(), partner.data())
    else {
        return None;
    };
    let (body_state, partner_state) = (
        body_data.horse_phenotype()?,
        partner_data.horse_phenotype()?,
    );
    let horse_state = match (body_state, partner_state) {
        (
            HorsePhenotypeState::Horse {
                packed_variant: own,
            },
            HorsePhenotypeState::Horse {
                packed_variant: other,
            },
        ) => {
            // `Horse.java:198-216`: seven variants, five markings.
            let select_skin = random.next_int(9);
            let variant = match select_skin {
                0..4 => own & 0xff,
                4..8 => other & 0xff,
                _ => random.next_int(7),
            };
            let select_marking = random.next_int(5);
            let markings = match select_marking {
                0..2 => own >> 8 & 0xff,
                2..4 => other >> 8 & 0xff,
                _ => random.next_int(5),
            };
            Some(HorsePhenotypeState::horse(variant | markings << 8))
        }
        _ => None,
    };
    let attributes = offspring_attributes(body, partner, random)?;
    let state = match (body_state, partner_state) {
        (
            HorsePhenotypeState::Llama {
                strength: own_strength,
                variant: own_variant,
                ..
            },
            HorsePhenotypeState::Llama {
                strength: other_strength,
                variant: other_variant,
                ..
            },
        ) => {
            // `Llama.java:323-329`, `setStrength` clamps to 1..=5 (`:88-90`).
            let mut strength = random.next_int(own_strength.max(other_strength).max(1)) + 1;
            if random.next_float() < 0.03 {
                strength += 1;
            }
            let variant = if random.next_boolean() {
                own_variant
            } else {
                other_variant
            };
            Some(HorsePhenotypeState::llama(
                false,
                strength.clamp(1, 5),
                variant,
            ))
        }
        _ => horse_state,
    };
    Some(EquineOffspring { state, attributes })
}

/// `setOffspringAttributes` over `createOffspringAttribute`
/// (`AbstractHorse.java:825-862`), with the ranges of `:83-88`.
fn offspring_attributes(
    body: &Entity,
    partner: &Entity,
    random: &GameRandom,
) -> Option<[(i32, f64); 3]> {
    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
    };
    let ranges = [
        (VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 15.0, 30.0),
        (
            VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
            f64::from(0.4_f32),
            f64::from((f64::from(0.4_f32) + 0.6) as f32),
        ),
        (
            VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            f64::from((f64::from(0.45_f32) * 0.25) as f32),
            f64::from(((f64::from(0.45_f32) + 0.9) * 0.25) as f32),
        ),
    ];
    let base =
        |entity: &Entity, attribute_id| entity.living()?.attributes().base_value(attribute_id);
    let mut values = [(0, 0.0); 3];
    for (slot, (attribute_id, min, max)) in values.iter_mut().zip(ranges) {
        let a = base(body, attribute_id)?.clamp(min, max);
        let b = base(partner, attribute_id)?.clamp(min, max);
        let margin = 0.15 * (max - min);
        let range = (a - b).abs() + margin * 2.0;
        let average = (a + b) / 2.0;
        let quality =
            (random.next_double() + random.next_double() + random.next_double()) / 3.0 - 0.5;
        let value = average + range * quality;
        let value = if value > max {
            max - (value - max)
        } else if value < min {
            min + (min - value)
        } else {
            value
        };
        *slot = (attribute_id, value);
    }
    Some(values)
}

pub(crate) fn owns_save_field(state: HorsePhenotypeState, name: &str) -> bool {
    match state {
        HorsePhenotypeState::Horse { .. } => name == "Variant",
        HorsePhenotypeState::Chested { .. } => matches!(name, "ChestedHorse" | "Items"),
        HorsePhenotypeState::Llama { .. } => {
            matches!(name, "ChestedHorse" | "Items" | "Variant" | "Strength")
        }
    }
}

pub(crate) fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    state: HorsePhenotypeState,
    slots: &[Option<bbb_server_types::item_stack::RuntimeItemStack>],
    items: crate::entity::item::ItemResolver<'_>,
) {
    // Concrete writers append after AbstractHorse
    // (`Horse.java:65-67`, `AbstractChestedHorse.java:82-84`, `Llama.java:102-105`).
    match state {
        HorsePhenotypeState::Horse { packed_variant } => {
            entries.push(int_entry("Variant", packed_variant));
        }
        HorsePhenotypeState::Chested { has_chest } => {
            entries.push(bool_entry("ChestedHorse", has_chest));
            if has_chest {
                crate::entity::save::write_container_items(entries, slots, items);
            }
        }
        HorsePhenotypeState::Llama {
            has_chest,
            strength,
            variant,
        } => {
            entries.push(bool_entry("ChestedHorse", has_chest));
            if has_chest {
                crate::entity::save::write_container_items(entries, slots, items);
            }
            entries.extend([
                int_entry("Variant", variant),
                int_entry("Strength", strength),
            ]);
        }
    }
}

pub(crate) fn read_save_data(
    entity_type_id: i32,
    entries: &[NbtSummaryEntry],
) -> Option<HorsePhenotypeState> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_HORSE_ID => Some(HorsePhenotypeState::horse(
            optional_int(entries, "Variant").unwrap_or(0),
        )),
        VANILLA_ENTITY_TYPE_DONKEY_ID | VANILLA_ENTITY_TYPE_MULE_ID => Some(
            HorsePhenotypeState::chested(optional_bool(entries, "ChestedHorse").unwrap_or(false)),
        ),
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            Some(HorsePhenotypeState::llama(
                optional_bool(entries, "ChestedHorse").unwrap_or(false),
                // `setStrength` clamps the reader's missing-field zero to one
                // (`Llama.java:88-112`).
                optional_int(entries, "Strength").unwrap_or(0).clamp(1, 5),
                optional_int(entries, "Variant").unwrap_or(0).clamp(0, 3),
            ))
        }
        _ => None,
    }
}

pub(crate) fn synched_values(state: HorsePhenotypeState) -> Vec<SynchedValue> {
    // The concrete accessors follow AbstractHorse's ID 18 flag byte
    // (`Horse.java:40-61`, `AbstractChestedHorse.java:32-52`, `Llama.java:69-70,137-140`).
    match state {
        HorsePhenotypeState::Horse { packed_variant } => vec![int_synched(
            VANILLA_HORSE_VARIANT_DATA_ID,
            packed_variant,
            0,
        )],
        HorsePhenotypeState::Chested { has_chest } => vec![bool_synched(
            VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID,
            has_chest,
        )],
        HorsePhenotypeState::Llama {
            has_chest,
            strength,
            variant,
        } => vec![
            bool_synched(VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, has_chest),
            int_synched(VANILLA_LLAMA_STRENGTH_DATA_ID, strength, 0),
            int_synched(VANILLA_LLAMA_VARIANT_DATA_ID, variant, 0),
        ],
    }
}

fn int_synched(data_id: u8, value: i32, default: i32) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(value),
        },
        is_default: value == default,
    }
}

fn bool_synched(data_id: u8, value: bool) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(value),
        },
        is_default: !value,
    }
}

fn int_entry(name: &str, value: i32) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Int(value),
    }
}

fn bool_entry(name: &str, value: bool) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    }
}

#[cfg(test)]
#[path = "horse_phenotype/tests.rs"]
mod tests;
