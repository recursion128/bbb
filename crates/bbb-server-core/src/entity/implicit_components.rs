//! The entity state `Entity.applyComponentsFromItemStack` takes from a stack.

use bbb_facts::VillagerModelType;
use bbb_protocol::builtin_enum_component_values::{
    builtin_enum_component_value_name, VANILLA_FOX_VARIANT_SNOW_ID,
};
use bbb_protocol::data_component_types::*;
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::PersistentItemStackHolderResolver;

use crate::entity::fish::{FishLeafState, SalmonVariant, TropicalFishVariant};
use crate::entity::horse_phenotype::HorsePhenotypeState;
use crate::entity::{Entity, EntityData};

pub(crate) fn apply_components_from_stack(
    entity: &mut Entity,
    stack: &ItemStackSummary,
    resolver: &dyn PersistentItemStackHolderResolver,
) {
    if let Some(name) = crate::entity::custom_name_from_stack(stack) {
        entity.set_custom_name(Some(name));
    }
    if let Some(NbtSummaryValue::Compound(data)) = &stack.component_patch.custom_data {
        entity.load_custom_data(data.clone());
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let entity_type_id = entity.entity_type_id();
    let horse = data.horse_phenotype();
    let fish = data.fish_leaf();
    let patch = &stack.component_patch;
    for (&type_id, &value) in &patch.builtin_enum_component_ids {
        match (type_id, entity_type_id) {
            (VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, VANILLA_ENTITY_TYPE_WOLF_ID)
            | (VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID, VANILLA_ENTITY_TYPE_CAT_ID) => {
                crate::entity::collar::set_collar_color(entity, value as i8);
            }
            (VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID, VANILLA_ENTITY_TYPE_SHEEP_ID) => {
                crate::entity::sheep::set_spawn_color(entity, value as i8);
            }
            (VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID, VANILLA_ENTITY_TYPE_SHULKER_ID) => {
                crate::entity::shulker::set_color(entity, value as i8);
            }
            (VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, VANILLA_ENTITY_TYPE_RABBIT_ID) => {
                crate::entity::rabbit::set_variant(entity, value);
            }
            (VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID, VANILLA_ENTITY_TYPE_PARROT_ID)
            | (VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID) => {
                crate::entity::general_living::set_spawner_simple_variant(entity, value);
            }
            (
                VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            ) => {
                crate::entity::mooshroom::set_variant(entity, value);
            }
            (VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, VANILLA_ENTITY_TYPE_FOX_ID) => {
                crate::entity::fox::set_spawn_variant(entity, value == VANILLA_FOX_VARIANT_SNOW_ID);
            }
            (VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, VANILLA_ENTITY_TYPE_SALMON_ID) => {
                let name = builtin_enum_component_value_name(type_id, value)
                    .expect("a decoded salmon size");
                crate::entity::fish::set_spawn_salmon_variant(
                    entity,
                    SalmonVariant::from_name(name).expect("a decoded salmon size"),
                );
            }
            (VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, VANILLA_ENTITY_TYPE_HORSE_ID) => {
                if let Some(HorsePhenotypeState::Horse { packed_variant }) = horse {
                    // Horse.setVariant keeps the markings byte.
                    crate::entity::horse_phenotype::set_state(
                        entity,
                        HorsePhenotypeState::horse(value & 0xff | packed_variant & !0xff),
                    );
                }
            }
            (
                VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            ) => {
                if let Some(HorsePhenotypeState::Llama {
                    has_chest,
                    strength,
                    ..
                }) = horse
                {
                    crate::entity::horse_phenotype::set_state(
                        entity,
                        HorsePhenotypeState::llama(has_chest, strength, value),
                    );
                }
            }
            _ => {}
        }
    }
    if matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_VILLAGER_ID | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
    ) {
        if let Some(kind) = patch
            .villager_variant_id
            .and_then(VillagerModelType::for_vanilla_registry_id)
        {
            crate::entity::general_living::set_component_villager_type(entity, kind.resource_id());
        }
    }
    if let Some(FishLeafState::Tropical(current)) = fish {
        let packed = current.packed();
        if patch.tropical_fish_pattern_id.is_some()
            || patch.tropical_fish_base_color_id.is_some()
            || patch.tropical_fish_pattern_color_id.is_some()
        {
            let variant = TropicalFishVariant::new(
                patch.tropical_fish_pattern_id.unwrap_or(packed & 0xffff),
                patch
                    .tropical_fish_base_color_id
                    .unwrap_or((packed >> 16) & 0xff),
                patch
                    .tropical_fish_pattern_color_id
                    .unwrap_or((packed >> 24) & 0xff),
            )
            .expect("decoded tropical fish components");
            crate::entity::fish::set_spawn_tropical_variant(entity, variant);
        }
    }
    let mut set_registry = |registry, holder_id: Option<i32>| {
        if let Some(holder_id) = holder_id {
            if let Some(resource_id) = resolver.holder_resource_id(registry, holder_id) {
                crate::entity::general_living::set_spawner_registry_value(
                    entity,
                    registry,
                    &resource_id,
                    holder_id,
                );
            }
        }
    };
    match entity_type_id {
        VANILLA_ENTITY_TYPE_WOLF_ID => {
            set_registry("minecraft:wolf_variant", patch.wolf_variant_id);
            set_registry("minecraft:wolf_sound_variant", patch.wolf_sound_variant_id);
        }
        VANILLA_ENTITY_TYPE_CAT_ID => {
            set_registry("minecraft:cat_variant", patch.cat_variant_id);
            set_registry("minecraft:cat_sound_variant", patch.cat_sound_variant_id);
        }
        VANILLA_ENTITY_TYPE_PIG_ID => {
            set_registry("minecraft:pig_variant", patch.pig_variant_id);
            set_registry("minecraft:pig_sound_variant", patch.pig_sound_variant_id);
        }
        VANILLA_ENTITY_TYPE_COW_ID => {
            set_registry("minecraft:cow_variant", patch.cow_variant_id);
            set_registry("minecraft:cow_sound_variant", patch.cow_sound_variant_id);
        }
        VANILLA_ENTITY_TYPE_CHICKEN_ID => {
            set_registry("minecraft:chicken_variant", patch.chicken_variant_id);
            set_registry(
                "minecraft:chicken_sound_variant",
                patch.chicken_sound_variant_id,
            );
        }
        VANILLA_ENTITY_TYPE_FROG_ID => {
            set_registry("minecraft:frog_variant", patch.frog_variant_id);
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            set_registry(
                "minecraft:zombie_nautilus_variant",
                patch.zombie_nautilus_variant_id,
            );
        }
        _ => {}
    }
}
