use crate::loot_runtime::LootRuntimeReason;
use bbb_protocol::builtin_enum_component_values::builtin_enum_component_value_name;
use bbb_protocol::data_component_types::*;
use bbb_protocol::tropical_fish_patterns::vanilla_tropical_fish_pattern;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::fish::FishLeafState;
use crate::entity::horse_phenotype::HorsePhenotypeState;
use crate::entity::{Entity, EntityData};

impl Entity {
    // Entity.java:3957-3962; EntityType components are empty in DataComponentInitializers.java:76-81.
    pub fn loot_component(
        &self,
        type_id: i32,
    ) -> Result<Option<NbtSummaryValue>, LootRuntimeReason> {
        if type_id == VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID {
            return Ok(self.custom_name().map(|name| name.component().clone()));
        }
        if type_id == VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID {
            return Ok(Some(NbtSummaryValue::Compound(self.custom_data().to_vec())));
        }
        let kind = self.data().entity_resource_id();
        let living = match self.data() {
            EntityData::GeneralLiving(data) => Some(data),
            _ => None,
        };
        let state = || living.ok_or_else(|| refusal("component getter requires its living state"));
        let missing = || {
            refusal(format!(
                "{kind} component {type_id} has no actual runtime state"
            ))
        };
        let enum_value = |id| enum_component(type_id, id).ok_or_else(missing);
        let string = |value: &str| NbtSummaryValue::string(value);
        let registry = |registry: &str| {
            state()?
                .registry_variant(registry)
                .map(string)
                .ok_or_else(missing)
        };
        let value = match (type_id, kind) {
            (VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID, "minecraft:sheep") => enum_value(
                i32::from(state()?.sheep_retained().ok_or_else(missing)?.color()),
            )?,
            (VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, "minecraft:rabbit") => {
                enum_value(state()?.rabbit_retained().ok_or_else(missing)?.variant())?
            }
            (VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, "minecraft:fox") => string(
                state()?
                    .fox_retained()
                    .ok_or_else(missing)?
                    .variant()
                    .serialized_name(),
            ),
            (VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID, "minecraft:mooshroom") => {
                enum_value(crate::entity::mooshroom::variant_of(self).ok_or_else(missing)?)?
            }
            (VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID, "minecraft:cat")
            | (VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, "minecraft:wolf") => enum_value(
                i32::from(crate::entity::collar::color_of(self).ok_or_else(missing)?),
            )?,
            // Horse.java:getVariant takes the coat byte, excluding marking bits.
            (VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, "minecraft:horse") => {
                let Some(HorsePhenotypeState::Horse { packed_variant }) =
                    state()?.horse_phenotype()
                else {
                    return Err(missing());
                };
                enum_value(packed_variant & 0xff)?
            }
            (
                VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                "minecraft:llama" | "minecraft:trader_llama",
            ) => {
                let Some(HorsePhenotypeState::Llama { variant, .. }) = state()?.horse_phenotype()
                else {
                    return Err(missing());
                };
                enum_value(variant)?
            }
            (VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, "minecraft:salmon") => {
                let Some(FishLeafState::Salmon(variant)) = state()?.fish_leaf() else {
                    return Err(missing());
                };
                string(variant.serialized_name())
            }
            (
                VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID
                | VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID
                | VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
                "minecraft:tropical_fish",
            ) => {
                let Some(FishLeafState::Tropical(variant)) = state()?.fish_leaf() else {
                    return Err(missing());
                };
                let packed = variant.packed();
                match type_id {
                    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => {
                        string(vanilla_tropical_fish_pattern(packed & 0xffff).serialized_name)
                    }
                    VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => {
                        enum_value((packed >> 16) & 0xff)?
                    }
                    _ => enum_value((packed >> 24) & 0xff)?,
                }
            }
            (VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, "minecraft:axolotl")
            | (VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID, "minecraft:parrot") => {
                enum_value(state()?.component_simple_variant().ok_or_else(missing)?)?
            }
            (VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID, "minecraft:frog") => {
                registry("minecraft:frog_variant")?
            }
            (VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID, "minecraft:cat") => {
                registry("minecraft:cat_variant")?
            }
            (VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID, "minecraft:chicken") => {
                registry("minecraft:chicken_variant")?
            }
            (VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID, "minecraft:cow") => {
                registry("minecraft:cow_variant")?
            }
            (VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID, "minecraft:pig") => {
                registry("minecraft:pig_variant")?
            }
            (VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID, "minecraft:wolf") => {
                registry("minecraft:wolf_variant")?
            }
            (
                VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
                "minecraft:zombie_nautilus",
            ) => registry("minecraft:zombie_nautilus_variant")?,
            (VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID, "minecraft:cat")
            | (VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID, "minecraft:chicken")
            | (VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID, "minecraft:cow")
            | (VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID, "minecraft:pig")
            | (VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID, "minecraft:wolf") => string(
                crate::entity::general_living::sound_variant_resource_id(self)
                    .ok_or_else(missing)?,
            ),
            (
                VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
                "minecraft:villager" | "minecraft:zombie_villager",
            ) => string(state()?.component_villager_type().ok_or_else(missing)?),
            (VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID, "minecraft:painting") => {
                let EntityData::Painting(data) = self.data() else {
                    return Err(missing());
                };
                string(data.variant().resource_id())
            }
            (VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, "minecraft:mannequin") => state()?
                .mannequin_profile()
                .cloned()
                .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new())),
            (VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID, "minecraft:shulker") => {
                let color = state()?.shulker_state().ok_or_else(missing)?.color();
                // Shulker.java:552-554: 16 and values above 15 mean no color.
                if color == 16 || color > 15 {
                    return Ok(None);
                }
                enum_value(i32::from(color).max(0))?
            }
            (VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, "minecraft:area_effect_cloud") => {
                let EntityData::AreaEffectCloud(data) = self.data() else {
                    return Err(missing());
                };
                match data.potion_contents() {
                    Some(contents) => bbb_registry_codec::persistent_item_stack_projection::potion_contents::write(
                        contents,
                        bbb_protocol::nbt_summary::NbtSummaryDecodeLimits::SAVED_FILE.max_depth,
                    )
                        .map_err(|error| refusal(error.to_string()))?,
                    None => NbtSummaryValue::Compound(Vec::new()),
                }
            }
            (
                VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                "minecraft:area_effect_cloud",
            ) => {
                let EntityData::AreaEffectCloud(data) = self.data() else {
                    return Err(missing());
                };
                NbtSummaryValue::Float(data.potion_duration_scale().to_bits())
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    pub fn custom_data(&self) -> &[NbtSummaryEntry] {
        match self.data() {
            EntityData::Marker(data) => data.data(),
            _ => self.custom_data.as_deref().expect("base custom data owner"),
        }
    }

    pub(crate) fn load_custom_data(&mut self, entries: Vec<NbtSummaryEntry>) {
        match &mut self.data {
            EntityData::Marker(data) => *data = crate::entity::MarkerData::with_data(entries),
            _ => self.custom_data = Some(entries),
        }
    }
}

fn enum_component(type_id: i32, id: i32) -> Option<NbtSummaryValue> {
    builtin_enum_component_value_name(type_id, id).map(NbtSummaryValue::string)
}

fn refusal(detail: impl Into<String>) -> LootRuntimeReason {
    LootRuntimeReason::UnsupportedField {
        owner: "entity component getter",
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityIdCounter, EntityStore};
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
    use bbb_protocol::packets::Vec3d;

    #[test]
    fn loot_component_reads_command_loaded_instance_and_preserves_base_custom_data_on_save() {
        let mut store = EntityStore::new(13, EntityIdCounter::new());
        let id = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_SHEEP_ID, Vec3d::default())
            .unwrap();
        let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
            panic!("entity save is a compound")
        };
        let data = vec![NbtSummaryEntry {
            name: "answer".into(),
            value: NbtSummaryValue::Short(42),
        }];
        entries.push(NbtSummaryEntry {
            name: "data".into(),
            value: NbtSummaryValue::Compound(data.clone()),
        });
        entries
            .iter_mut()
            .find(|entry| entry.name == "Color")
            .unwrap()
            .value = NbtSummaryValue::Byte(11);
        store.load_command_data(id, &entries).unwrap();
        let entity = store.get(id).unwrap();
        let color = entity
            .loot_component(VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID)
            .unwrap()
            .unwrap();
        assert_eq!(Some(color), Some(NbtSummaryValue::String("blue".into())));
        let copied = entity
            .loot_component(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID)
            .unwrap()
            .unwrap();
        assert_eq!(Some(copied), Some(NbtSummaryValue::Compound(data.clone())));
        let NbtSummaryValue::Compound(saved) = store.command_data(id).unwrap() else {
            panic!("entity save is a compound")
        };
        let fields = saved
            .iter()
            .filter(|entry| entry.name == "data")
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].value, NbtSummaryValue::Compound(data));
        let mannequin = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
                Vec3d::default(),
            )
            .unwrap();
        let profile = store
            .get(mannequin)
            .unwrap()
            .loot_component(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID)
            .unwrap()
            .unwrap();
        assert_eq!(Some(profile), Some(NbtSummaryValue::Compound(Vec::new())));
        let cloud = crate::entity::area_effect_cloud::spawn_area_effect_cloud(
            &mut store,
            Vec3d::default(),
            crate::entity::AreaEffectCloudData::new(),
        )
        .unwrap();
        let NbtSummaryValue::Compound(mut entries) = store.command_data(cloud).unwrap() else {
            unreachable!()
        };
        entries.push(NbtSummaryEntry {
            name: "potion_contents".into(),
            value: NbtSummaryValue::String("healing".into()),
        });
        store.load_command_data(cloud, &entries).unwrap();
        let potion = store
            .get(cloud)
            .unwrap()
            .loot_component(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
            .unwrap()
            .unwrap();
        assert_eq!(
            Some(potion),
            Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "potion".into(),
                value: NbtSummaryValue::String("minecraft:healing".into())
            }]))
        );
    }
}
