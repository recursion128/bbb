use super::*;
use crate::entities::{
    vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type,
    vanilla_entity_metadata_allay_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type,
    vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type,
    vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type,
    vanilla_entity_metadata_dolphin_accessors_for_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_goat_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type,
    vanilla_entity_metadata_panda_accessors_for_entity_type,
    vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_salmon_variant_accessor_for_entity_type,
    vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type,
    vanilla_entity_metadata_wolf_accessors_for_entity_type, VANILLA_ARMADILLO_STATE_IDLE_ID,
};
use bbb_protocol::{
    builtin_enum_component_values::normalize_vanilla_dye_color_id,
    data_component_types::{
        VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
    },
    entity_types::{
        VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID, VANILLA_ENTITY_TYPE_SALMON_ID,
        VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
    },
    item_types::{
        vanilla_suspicious_stew_ingredient, VANILLA_ITEM_AXOLOTL_BUCKET_ID, VANILLA_ITEM_BOWL_ID,
        VANILLA_ITEM_BRUSH_ID, VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_CHEST_ID,
        VANILLA_ITEM_CLOSED_EYEBLOSSOM_ID, VANILLA_ITEM_COD_BUCKET_ID, VANILLA_ITEM_FIRE_CHARGE_ID,
        VANILLA_ITEM_GOLD_INGOT_ID, VANILLA_ITEM_MILK_BUCKET_ID, VANILLA_ITEM_MUSHROOM_STEW_ID,
        VANILLA_ITEM_OPEN_EYEBLOSSOM_ID, VANILLA_ITEM_PUFFERFISH_BUCKET_ID,
        VANILLA_ITEM_SALMON_BUCKET_ID, VANILLA_ITEM_SUSPICIOUS_STEW_ID,
        VANILLA_ITEM_TADPOLE_BUCKET_ID, VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID,
        VANILLA_ITEM_WATER_BUCKET_ID, VANILLA_ITEM_WITHER_ROSE_ID,
    },
    mob_effect_types::{VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_WEAKNESS_ID},
    packets::{
        DataComponentPatchSummary, ItemStackSummary, NbtSummaryEntry, NbtSummaryValue,
        SuspiciousStewEffectSummary,
    },
};

#[cfg(test)]
mod tests;

const ALLAY: &str = "net.minecraft.world.entity.animal.allay.Allay";
const ARMADILLO: &str = "net.minecraft.world.entity.animal.armadillo.Armadillo";
const AXOLOTL: &str = "net.minecraft.world.entity.animal.axolotl.Axolotl";
const BEE: &str = "net.minecraft.world.entity.animal.bee.Bee";
const CAMEL: &str = "net.minecraft.world.entity.animal.camel.Camel";
const CAT: &str = "net.minecraft.world.entity.animal.feline.Cat";
const ABSTRACT_FISH: &str = "net.minecraft.world.entity.animal.fish.AbstractFish";
const ABSTRACT_COW: &str = "net.minecraft.world.entity.animal.cow.AbstractCow";
const DOLPHIN: &str = "net.minecraft.world.entity.animal.dolphin.Dolphin";
const ABSTRACT_CHESTED_HORSE: &str =
    "net.minecraft.world.entity.animal.equine.AbstractChestedHorse";
const HAPPY_GHAST: &str = "net.minecraft.world.entity.animal.happyghast.HappyGhast";
const GOAT: &str = "net.minecraft.world.entity.animal.goat.Goat";
const HOGLIN: &str = "net.minecraft.world.entity.monster.hoglin.Hoglin";
const HORSE: &str = "net.minecraft.world.entity.animal.equine.Horse";
const OCELOT: &str = "net.minecraft.world.entity.animal.feline.Ocelot";
const PANDA: &str = "net.minecraft.world.entity.animal.panda.Panda";
const PARROT: &str = "net.minecraft.world.entity.animal.parrot.Parrot";
const PIG: &str = "net.minecraft.world.entity.animal.pig.Pig";
const PIGLIN: &str = "net.minecraft.world.entity.monster.piglin.Piglin";
const SKELETON_HORSE: &str = "net.minecraft.world.entity.animal.equine.SkeletonHorse";
const SNIFFER: &str = "net.minecraft.world.entity.animal.sniffer.Sniffer";
const STRIDER: &str = "net.minecraft.world.entity.monster.Strider";
const TADPOLE: &str = "net.minecraft.world.entity.animal.frog.Tadpole";
const WOLF: &str = "net.minecraft.world.entity.animal.wolf.Wolf";
const ZOMBIE_VILLAGER: &str = "net.minecraft.world.entity.monster.zombie.ZombieVillager";

const DUPLICATES_ALLAYS_TAG: &str = "minecraft:duplicates_allays";
const FISHES_TAG: &str = "minecraft:fishes";
const FROG_FOOD_TAG: &str = "minecraft:frog_food";
const PARROT_FOOD_TAG: &str = "minecraft:parrot_food";
const PARROT_POISONOUS_FOOD_TAG: &str = "minecraft:parrot_poisonous_food";
const PIGLIN_LOVED_TAG: &str = "minecraft:piglin_loved";
const CAT_COLLAR_DYES_TAG: &str = "minecraft:cat_collar_dyes";
const WOLF_COLLAR_DYES_TAG: &str = "minecraft:wolf_collar_dyes";
const REPAIRS_WOLF_ARMOR_TAG: &str = "minecraft:repairs_wolf_armor";

pub(super) fn generated_mob_interact_owner_is_classified(owner: &str) -> bool {
    matches!(
        owner,
        ALLAY
            | ARMADILLO
            | AXOLOTL
            | VANILLA_MOB_ENTITY_INTERACT_OWNER
            | BEE
            | VANILLA_BOGGED_MOB_INTERACT_OWNER
            | CAMEL
            | CAT
            | VANILLA_ANIMAL_MOB_INTERACT_OWNER
            | ABSTRACT_FISH
            | VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER
            | ABSTRACT_COW
            | VANILLA_CREEPER_MOB_INTERACT_OWNER
            | DOLPHIN
            | ABSTRACT_CHESTED_HORSE
            | HAPPY_GHAST
            | VANILLA_AGEABLE_MOB_INTERACT_OWNER
            | GOAT
            | HOGLIN
            | HORSE
            | VANILLA_IRON_GOLEM_MOB_INTERACT_OWNER
            | VANILLA_MOOSHROOM_MOB_INTERACT_OWNER
            | VANILLA_NAUTILUS_INTERACT_OWNER
            | OCELOT
            | PANDA
            | PARROT
            | PIG
            | PIGLIN
            | VANILLA_SHEEP_MOB_INTERACT_OWNER
            | SKELETON_HORSE
            | SNIFFER
            | VANILLA_SNOW_GOLEM_MOB_INTERACT_OWNER
            | STRIDER
            | TADPOLE
            | VANILLA_VILLAGER_MOB_INTERACT_OWNER
            | VANILLA_WANDERING_TRADER_MOB_INTERACT_OWNER
            | WOLF
            | VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
            | ZOMBIE_VILLAGER
    )
}

impl WorldStore {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn predict_local_specialized_generated_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_stack: Option<&ProtocolItemStackSummary>,
        held_item_id: Option<i32>,
        owner: &'static str,
        secondary_use_active: bool,
    ) -> Option<LocalTargetUsePrediction> {
        let result = match owner {
            ALLAY => {
                self.predict_local_allay_mob_interact(entity_id, entity_type_id, hand, held_stack)
            }
            ARMADILLO => self.predict_local_armadillo_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
            ),
            AXOLOTL => self.predict_local_bucketable_or_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            ),
            BEE => {
                self.predict_local_bee_mob_interact(entity_id, entity_type_id, hand, held_item_id)
            }
            VANILLA_BOGGED_MOB_INTERACT_OWNER => local_pass(),
            CAMEL => self.predict_local_camel_generated_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_stack,
                held_item_id,
                secondary_use_active,
                entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            ),
            CAT => self.predict_local_cat_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_stack,
                held_item_id,
            ),
            ABSTRACT_FISH => self.predict_local_bucketable_or_pass_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            ),
            VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER => {
                self.predict_local_copper_golem_mob_interact(entity_id, held_item_id)
            }
            ABSTRACT_COW => {
                self.predict_local_cow_mob_interact(entity_id, entity_type_id, hand, held_item_id)
            }
            VANILLA_CREEPER_MOB_INTERACT_OWNER => {
                self.predict_local_creeper_mob_interact(entity_id, held_item_id)
            }
            DOLPHIN => self.predict_local_dolphin_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
            ),
            ABSTRACT_CHESTED_HORSE => self.predict_local_generated_horse_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
                true,
                owner,
            ),
            HAPPY_GHAST => self.predict_local_happy_ghast_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_stack,
                held_item_id,
                secondary_use_active,
            ),
            GOAT => {
                self.predict_local_goat_mob_interact(entity_id, entity_type_id, hand, held_item_id)
            }
            HOGLIN | OCELOT => self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            ),
            HORSE => self.predict_local_generated_horse_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
                false,
                owner,
            ),
            VANILLA_IRON_GOLEM_MOB_INTERACT_OWNER => self
                .predict_local_generated_iron_golem_mob_interact(
                    entity_id,
                    entity_type_id,
                    hand,
                    held_item_id,
                ),
            VANILLA_MOOSHROOM_MOB_INTERACT_OWNER => self.predict_local_mooshroom_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
            ),
            PANDA => {
                self.predict_local_panda_mob_interact(entity_id, entity_type_id, hand, held_item_id)
            }
            PARROT => self.predict_local_parrot_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
            ),
            PIG | STRIDER => self.predict_local_saddled_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_stack,
                held_item_id,
                secondary_use_active,
                owner,
            ),
            PIGLIN => {
                self.predict_local_piglin_mob_interact(entity_id, entity_type_id, held_item_id)
            }
            VANILLA_SHEEP_MOB_INTERACT_OWNER => self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            ),
            SKELETON_HORSE => self.predict_local_skeleton_horse_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
            ),
            SNIFFER => self.predict_local_animal_with_eating_sound(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            ),
            VANILLA_SNOW_GOLEM_MOB_INTERACT_OWNER => local_pass(),
            TADPOLE => self.predict_local_tadpole_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
            ),
            VANILLA_VILLAGER_MOB_INTERACT_OWNER => self.predict_local_villager_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
            ),
            VANILLA_WANDERING_TRADER_MOB_INTERACT_OWNER => self
                .predict_local_wandering_trader_mob_interact(
                    entity_id,
                    entity_type_id,
                    hand,
                    held_item_id,
                ),
            WOLF => self.predict_local_wolf_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_stack,
                held_item_id,
            ),
            ZOMBIE_VILLAGER => {
                self.predict_local_zombie_villager_mob_interact(entity_id, hand, held_item_id)
            }
            _ => return None,
        };
        Some(result)
    }

    pub(super) fn predict_local_camel_generated_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_stack: Option<&ProtocolItemStackSummary>,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
        husk: bool,
    ) -> LocalTargetUsePrediction {
        let is_baby = if husk {
            // `CamelHusk.isBaby()` is an unconditional override even though
            // the inherited AgeableMob metadata accessor still exists.
            false
        } else {
            let Some(is_baby) = self.entities.is_baby(entity_id) else {
                return unknown_entity(entity_type_id);
            };
            is_baby
        };
        if !is_baby && secondary_use_active {
            return client_item_success();
        }
        if let Some(stack) = held_stack {
            let item_interaction = self.predict_local_item_stack_interact_living_entity(
                entity_id,
                entity_type_id,
                stack,
            );
            if item_interaction != local_pass() {
                return item_interaction;
            }
        }
        let food_tag = if husk {
            VANILLA_CAMEL_HUSK_FOOD_TAG
        } else {
            match vanilla_entity_metadata_food_predicate(entity_type_id) {
                Some(VanillaEntityFoodPredicate::ItemTag(tag)) => tag,
                _ => return unsupported(owner_for_camel(husk)),
            }
        };
        let food_matches =
            match self.local_item_matches_tag(held_item_id, food_tag, owner_for_camel(husk)) {
                Ok(matched) => matched,
                Err(prediction) => return prediction,
            };
        if food_matches == Some(true) {
            return self.predict_local_camel_food(
                entity_id,
                entity_type_id,
                hand,
                owner_for_camel(husk),
            );
        }
        let Some(passenger_count) = self.entities.entity_passenger_count(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if passenger_count < 2 && !is_baby {
            if self
                .entities
                .apply_local_horse_ride_pose_reset(entity_id)
                .is_none()
            {
                return unknown_entity(entity_type_id);
            }
        }
        if is_baby && self.local_player_holds_item(VANILLA_ITEM_GOLDEN_DANDELION_ID) {
            return self.predict_local_abstract_horse_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
                owner_for_camel(husk),
                !husk,
            );
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false),
        ))
    }

    fn predict_local_allay_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_stack: Option<&ProtocolItemStackSummary>,
    ) -> LocalTargetUsePrediction {
        let held_item_id = held_stack.and_then(|stack| stack.item_id);
        let Some((dancing_data_id, dancing_default)) =
            vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(entity_type_id)
        else {
            return unknown_entity(entity_type_id);
        };
        let Some((duplicate_data_id, duplicate_default)) =
            vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type(entity_type_id)
        else {
            return unknown_entity(entity_type_id);
        };
        let Some(dancing) =
            self.entities
                .metadata_bool_for_entity(entity_id, dancing_data_id, dancing_default)
        else {
            return unknown_entity(entity_type_id);
        };
        let Some(can_duplicate) =
            self.entities
                .metadata_bool_for_entity(entity_id, duplicate_data_id, duplicate_default)
        else {
            return unknown_entity(entity_type_id);
        };
        if dancing && can_duplicate {
            match self.local_item_matches_tag(held_item_id, DUPLICATES_ALLAYS_TAG, ALLAY) {
                Ok(Some(true)) => {
                    if self
                        .entities
                        .set_local_metadata_bool(entity_id, duplicate_data_id, false)
                        .is_none()
                        || !self.consume_local_item_in_hand(hand, 1)
                    {
                        return unsupported(ALLAY);
                    }
                    return success_with_sound(LocalInteractionSoundAction::AllayInteraction {
                        entity_id,
                        sound_resource_id: "minecraft:block.amethyst_block.chime",
                    });
                }
                Err(prediction) => return prediction,
                _ => {}
            }
        }

        let allay_item = self
            .entities
            .equipment_item(entity_id, EquipmentSlot::MainHand);
        if allay_item.is_none() {
            if let Some(mut stack) = held_stack.cloned() {
                stack.count = 1;
                if !self.consume_local_item_in_hand(hand, 1)
                    || !self.entities.set_local_equipment_item(
                        entity_id,
                        EquipmentSlot::MainHand,
                        stack,
                    )
                {
                    return unsupported(ALLAY);
                }
                return success_with_sound(LocalInteractionSoundAction::AllayInteraction {
                    entity_id,
                    sound_resource_id: "minecraft:entity.allay.item_given",
                });
            }
        } else if hand == InteractionHand::MainHand && held_stack.is_none() {
            let item = allay_item.expect("checked nonempty allay hand");
            if !self
                .entities
                .clear_local_equipment_item(entity_id, EquipmentSlot::MainHand)
            {
                return unsupported(ALLAY);
            }
            let duration = self.entity_held_item_swing_duration(entity_id, false);
            if self
                .entities
                .trigger_client_animation_swing(entity_id, false, duration)
                .is_none()
            {
                return unknown_entity(entity_type_id);
            }
            let _ = self.add_local_item_from_block_interaction(item);
            return success_with_sound(LocalInteractionSoundAction::AllayInteraction {
                entity_id,
                sound_resource_id: "minecraft:entity.allay.item_taken",
            });
        }
        local_pass()
    }

    fn predict_local_armadillo_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if held_item_id == Some(VANILLA_ITEM_BRUSH_ID) && !is_baby {
            return client_item_success();
        }
        let Some((data_id, default)) =
            vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(entity_type_id)
        else {
            return unknown_entity(entity_type_id);
        };
        let Some(state) = self
            .entities
            .metadata_int_for_entity(entity_id, data_id, default)
        else {
            return unknown_entity(entity_type_id);
        };
        if state != VANILLA_ARMADILLO_STATE_IDLE_ID {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
        } else {
            self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                ARMADILLO,
            )
        }
    }

    fn predict_local_bee_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        if matches!(
            held_item_id,
            Some(
                VANILLA_ITEM_WITHER_ROSE_ID
                    | VANILLA_ITEM_OPEN_EYEBLOSSOM_ID
                    | VANILLA_ITEM_CLOSED_EYEBLOSSOM_ID
            )
        ) {
            match self.local_item_matches_entity_food(entity_type_id, held_item_id, BEE) {
                Ok(true) => {
                    match self.use_local_mob_player_item(entity_type_id, hand, held_item_id, BEE) {
                        Ok(false) => {}
                        Ok(true) | Err(_) => return unsupported(BEE),
                    }
                    return client_item_success();
                }
                Err(prediction) => return prediction,
                Ok(false) => {}
            }
        }
        self.predict_local_animal_mob_interact(entity_id, entity_type_id, hand, held_item_id, BEE)
    }

    fn predict_local_copper_golem_mob_interact(
        &mut self,
        entity_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        if held_item_id.is_some() {
            return local_pass();
        }
        let holding_item = self
            .entities
            .equipment_item(entity_id, EquipmentSlot::MainHand)
            .is_some();
        if holding_item
            && !self
                .entities
                .clear_local_equipment_item(entity_id, EquipmentSlot::MainHand)
        {
            return unsupported(VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER);
        }
        if holding_item {
            client_item_success()
        } else {
            local_pass()
        }
    }

    fn predict_local_cow_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if held_item_id == Some(VANILLA_ITEM_BUCKET_ID) && !is_baby {
            let filled = one_item_stack(VANILLA_ITEM_MILK_BUCKET_ID);
            let Some(dropped) =
                self.replace_local_item_with_filled_result(hand, VANILLA_ITEM_BUCKET_ID, filled)
            else {
                return unsupported(ABSTRACT_COW);
            };
            return item_success_with_actions(
                dropped.then_some(InteractionHand::MainHand),
                Some(LocalInteractionSoundAction::LocalPlayerMobInteraction {
                    sound_resource_id: "minecraft:entity.cow.milk",
                }),
                None,
            );
        }
        self.predict_local_animal_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            ABSTRACT_COW,
        )
    }

    fn predict_local_creeper_mob_interact(
        &self,
        entity_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(item_id) = held_item_id else {
            return local_pass();
        };
        let Some(igniters) =
            self.registry_holder_ids_for_tag("minecraft:item", VANILLA_CREEPER_IGNITERS_TAG)
        else {
            return unsupported(VANILLA_CREEPER_MOB_INTERACT_OWNER);
        };
        if !igniters.contains(&item_id) {
            return local_pass();
        }
        success_with_sound(LocalInteractionSoundAction::CreeperIgnition {
            entity_id,
            fire_charge: item_id == VANILLA_ITEM_FIRE_CHARGE_ID,
        })
    }

    fn predict_local_dolphin_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        match self.local_item_matches_tag(held_item_id, FISHES_TAG, DOLPHIN) {
            Ok(Some(true)) => {}
            Ok(_) => return local_pass(),
            Err(prediction) => return prediction,
        }
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        let can_age_up = if is_baby {
            let Some((data_id, default)) =
                vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
            else {
                return unknown_entity(entity_type_id);
            };
            let Some(locked) = self
                .entities
                .metadata_bool_for_entity(entity_id, data_id, default)
            else {
                return unknown_entity(entity_type_id);
            };
            !locked
        } else {
            false
        };
        if !can_age_up {
            let Some(accessors) =
                vanilla_entity_metadata_dolphin_accessors_for_entity_type(entity_type_id)
            else {
                return unknown_entity(entity_type_id);
            };
            if self
                .entities
                .set_local_metadata_bool(entity_id, accessors.got_fish_data_id, true)
                .is_none()
            {
                return unknown_entity(entity_type_id);
            }
        }
        if !self.consume_local_item_in_hand(hand, 1) {
            return unsupported(DOLPHIN);
        }
        client_item_success()
    }

    fn predict_local_happy_ghast_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_stack: Option<&ProtocolItemStackSummary>,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if is_baby {
            return self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                HAPPY_GHAST,
            );
        }
        if let Some(stack) = held_stack {
            let item_result = self.predict_local_item_stack_interact_living_entity(
                entity_id,
                entity_type_id,
                stack,
            );
            if item_result != local_pass() {
                return item_result;
            }
        }
        if !secondary_use_active
            && self
                .entities
                .equipment_item(entity_id, EquipmentSlot::Body)
                .is_some()
        {
            client_item_success()
        } else {
            self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                HAPPY_GHAST,
            )
        }
    }

    fn predict_local_goat_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if held_item_id == Some(VANILLA_ITEM_BUCKET_ID) && !is_baby {
            let Some(accessors) =
                vanilla_entity_metadata_goat_accessors_for_entity_type(entity_type_id)
            else {
                return unknown_entity(entity_type_id);
            };
            let Some(screaming) = self.entities.metadata_bool_for_entity(
                entity_id,
                accessors.screaming_data_id,
                accessors.screaming_default,
            ) else {
                return unknown_entity(entity_type_id);
            };
            let Some(dropped) = self.replace_local_item_with_filled_result(
                hand,
                VANILLA_ITEM_BUCKET_ID,
                one_item_stack(VANILLA_ITEM_MILK_BUCKET_ID),
            ) else {
                return unsupported(GOAT);
            };
            return item_success_with_actions(
                dropped.then_some(InteractionHand::MainHand),
                Some(LocalInteractionSoundAction::LocalPlayerMobInteraction {
                    sound_resource_id: if screaming {
                        "minecraft:entity.goat.screaming.milk"
                    } else {
                        "minecraft:entity.goat.milk"
                    },
                }),
                None,
            );
        }
        self.predict_local_animal_with_eating_sound(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            GOAT,
        )
    }

    fn predict_local_generated_iron_golem_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        if held_item_id != Some(VANILLA_ITEM_IRON_INGOT_ID) {
            return local_pass();
        }
        let Some(healed) = self.entities.apply_local_living_heal(entity_id, 25.0) else {
            return unknown_entity(entity_type_id);
        };
        if !healed {
            return local_pass();
        }
        if !self.consume_local_item_in_hand(hand, 1) {
            return unsupported(VANILLA_IRON_GOLEM_MOB_INTERACT_OWNER);
        }
        let Some(silent) = self.entities.is_silent(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        success_with_sound(LocalInteractionSoundAction::SuppressedMobInteraction {
            entity_id,
            silent,
            random_float_count: 2,
            random_before_silent_check: true,
        })
    }

    fn predict_local_piglin_mob_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if is_baby || held_item_id != Some(VANILLA_ITEM_GOLD_INGOT_ID) {
            return local_pass();
        }
        let offhand_item_id = self
            .entities
            .equipment_item(entity_id, EquipmentSlot::OffHand)
            .and_then(|item| item.item_id);
        let offhand_loved = if let Some(item_id) = offhand_item_id {
            let Some(items) = self.registry_holder_ids_for_tag("minecraft:item", PIGLIN_LOVED_TAG)
            else {
                return unsupported(PIGLIN);
            };
            items.contains(&item_id)
        } else {
            false
        };
        let dancing = if offhand_loved {
            let Some((data_id, default)) =
                vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(entity_type_id)
            else {
                return unknown_entity(entity_type_id);
            };
            let Some(dancing) = self
                .entities
                .metadata_bool_for_entity(entity_id, data_id, default)
            else {
                return unknown_entity(entity_type_id);
            };
            dancing
        } else {
            false
        };
        if offhand_loved && !dancing {
            local_pass()
        } else {
            client_item_success()
        }
    }

    fn predict_local_skeleton_horse_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(is_tamed) = self.local_abstract_horse_is_tamed(entity_id, entity_type_id) else {
            return unknown_entity(entity_type_id);
        };
        if !is_tamed {
            return local_pass();
        }
        self.predict_local_abstract_horse_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            secondary_use_active,
            SKELETON_HORSE,
            false,
        )
    }

    fn predict_local_zombie_villager_mob_interact(
        &mut self,
        entity_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        if held_item_id != Some(VANILLA_ITEM_GOLDEN_APPLE_ID) {
            return local_pass();
        }
        if self
            .entities
            .mob_effect_amplifier(entity_id, VANILLA_MOB_EFFECT_WEAKNESS_ID)
            .is_some()
        {
            if !self.consume_local_item_in_hand(hand, 1) {
                return unsupported(ZOMBIE_VILLAGER);
            }
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction(),
            ))
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ))
        }
    }

    fn predict_local_bucketable_or_animal_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        if held_item_id == Some(VANILLA_ITEM_WATER_BUCKET_ID) {
            self.predict_local_bucketable_pickup(entity_id, entity_type_id, hand, owner)
        } else {
            self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            )
        }
    }

    fn predict_local_bucketable_or_pass_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        if held_item_id == Some(VANILLA_ITEM_WATER_BUCKET_ID) {
            self.predict_local_bucketable_pickup(entity_id, entity_type_id, hand, owner)
        } else {
            local_pass()
        }
    }

    fn predict_local_bucketable_pickup(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        let Some(health) = self.entities.living_entity_health(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if !(health > 0.0) {
            return if entity_type_id == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
                self.predict_local_animal_mob_interact(
                    entity_id,
                    entity_type_id,
                    hand,
                    Some(VANILLA_ITEM_WATER_BUCKET_ID),
                    owner,
                )
            } else {
                local_pass()
            };
        }
        let Some(mut bucket) = self.local_bucketable_item_stack(entity_id, entity_type_id) else {
            return unsupported(owner);
        };
        bucket.component_patch.added = bucket.component_patch.added_type_ids.len();
        let Some(silent) = self.entities.is_silent(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        let Some(dropped) = self.replace_local_item_with_unlimited_creative_filled_result(
            hand,
            VANILLA_ITEM_WATER_BUCKET_ID,
            bucket,
        ) else {
            return unsupported(owner);
        };
        if !self.discard_local_entity(entity_id) {
            return unsupported(owner);
        }
        item_success_with_actions(
            dropped.then_some(InteractionHand::MainHand),
            Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                entity_id,
                silent,
                random_float_count: 0,
                random_before_silent_check: false,
            }),
            None,
        )
    }

    fn local_bucketable_item_stack(
        &self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> Option<ItemStackSummary> {
        let bucket_item_id = match entity_type_id {
            VANILLA_ENTITY_TYPE_AXOLOTL_ID => VANILLA_ITEM_AXOLOTL_BUCKET_ID,
            VANILLA_ENTITY_TYPE_COD_ID => VANILLA_ITEM_COD_BUCKET_ID,
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID => VANILLA_ITEM_PUFFERFISH_BUCKET_ID,
            VANILLA_ENTITY_TYPE_SALMON_ID => VANILLA_ITEM_SALMON_BUCKET_ID,
            VANILLA_ENTITY_TYPE_TADPOLE_ID => VANILLA_ITEM_TADPOLE_BUCKET_ID,
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID,
            _ => return None,
        };
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let mob_accessors = vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)?;
        let shared_flags = self.entities.metadata_byte_for_entity(
            entity_id,
            entity_accessors.shared_flags_data_id,
            entity_accessors.shared_flags_default,
        )?;
        let mob_flags = self.entities.metadata_byte_for_entity(
            entity_id,
            mob_accessors.flags_data_id,
            mob_accessors.flags_default,
        )?;
        let mut bucket_data = Vec::new();
        if mob_flags & mob_accessors.no_ai_mask != 0 {
            bucket_data.push(nbt_entry("NoAI", NbtSummaryValue::Byte(1)));
        }
        if self.entities.metadata_bool_for_entity(
            entity_id,
            entity_accessors.silent_data_id,
            entity_accessors.silent_default,
        )? {
            bucket_data.push(nbt_entry("Silent", NbtSummaryValue::Byte(1)));
        }
        if self.entities.metadata_bool_for_entity(
            entity_id,
            entity_accessors.no_gravity_data_id,
            entity_accessors.no_gravity_default,
        )? {
            bucket_data.push(nbt_entry("NoGravity", NbtSummaryValue::Byte(1)));
        }
        if shared_flags & entity_accessors.glowing_mask != 0 {
            bucket_data.push(nbt_entry("Glowing", NbtSummaryValue::Byte(1)));
        }
        let health = self.entities.living_entity_health(entity_id)?;
        bucket_data.push(nbt_entry(
            "Health",
            NbtSummaryValue::Float(health.to_bits()),
        ));

        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID);

        if let Some(custom_name) = self
            .entities
            .metadata_optional_rich_component_override(
                entity_id,
                entity_accessors.custom_name_data_id,
            )
            .flatten()
        {
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID);
            patch.custom_name = Some(custom_name.text().to_string());
            patch.custom_name_styled = Some(custom_name.styled().to_vec());
            patch.custom_name_nbt = Some(custom_name.component().clone());
        }

        match entity_type_id {
            VANILLA_ENTITY_TYPE_AXOLOTL_ID => {
                let (variant_data_id, variant_default) =
                    vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
                        entity_type_id,
                    )?;
                let variant = self.entities.metadata_int_for_entity(
                    entity_id,
                    variant_data_id,
                    variant_default,
                )?;
                patch
                    .added_type_ids
                    .push(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID);
                patch
                    .builtin_enum_component_ids
                    .insert(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, variant);
                let age = if self.entities.is_baby(entity_id)? {
                    -1
                } else {
                    1
                };
                bucket_data.push(nbt_entry("Age", NbtSummaryValue::Int(age)));
                let (locked_data_id, locked_default) =
                    vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)?;
                let locked = self.entities.metadata_bool_for_entity(
                    entity_id,
                    locked_data_id,
                    locked_default,
                )?;
                bucket_data.push(nbt_entry(
                    "AgeLocked",
                    NbtSummaryValue::Byte(i8::from(locked)),
                ));
            }
            VANILLA_ENTITY_TYPE_SALMON_ID => {
                let (variant_data_id, variant_default) =
                    vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
                        entity_type_id,
                    )?;
                let variant = self.entities.metadata_int_for_entity(
                    entity_id,
                    variant_data_id,
                    variant_default,
                )?;
                patch
                    .added_type_ids
                    .push(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID);
                patch
                    .builtin_enum_component_ids
                    .insert(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, variant);
            }
            VANILLA_ENTITY_TYPE_TADPOLE_ID => {
                bucket_data.push(nbt_entry(
                    "Age",
                    NbtSummaryValue::Int(self.entities.local_tadpole_age_ticks(entity_id)?),
                ));
                let (locked_data_id, locked_default) =
                    vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)?;
                let locked = self.entities.metadata_bool_for_entity(
                    entity_id,
                    locked_data_id,
                    locked_default,
                )?;
                bucket_data.push(nbt_entry(
                    "AgeLocked",
                    NbtSummaryValue::Byte(i8::from(locked)),
                ));
            }
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => {
                let accessors =
                    vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
                        entity_type_id,
                    )?;
                let packed = self.entities.metadata_int_for_entity(
                    entity_id,
                    accessors.variant_data_id,
                    accessors.variant_default,
                )?;
                patch.added_type_ids.extend([
                    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
                    VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
                    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
                ]);
                patch.tropical_fish_pattern_id = Some(accessors.pattern_id(packed));
                patch.tropical_fish_base_color_id = Some(accessors.base_color_id(packed));
                patch.tropical_fish_pattern_color_id = Some(accessors.pattern_color_id(packed));
            }
            _ => {}
        }
        patch.codec_backed_nbt.insert(
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            NbtSummaryValue::Compound(bucket_data),
        );
        Some(ItemStackSummary {
            item_id: Some(bucket_item_id),
            count: 1,
            component_patch: patch,
        })
    }

    fn predict_local_camel_food(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        let Some(health) = self.entities.living_entity_health(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        let Some(max_health) = self
            .entities
            .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        else {
            return unknown_entity(entity_type_id);
        };
        let healed = if health < max_health as f32 {
            let Some(healed) = self.entities.apply_local_living_heal(entity_id, 2.0) else {
                return unknown_entity(entity_type_id);
            };
            healed
        } else {
            false
        };
        let is_baby = if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
            false
        } else {
            let Some(is_baby) = self.entities.is_baby(entity_id) else {
                return unknown_entity(entity_type_id);
            };
            is_baby
        };
        let particle_action = if is_baby {
            let Some((data_id, default)) =
                vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
            else {
                return unknown_entity(entity_type_id);
            };
            let Some(locked) = self
                .entities
                .metadata_bool_for_entity(entity_id, data_id, default)
            else {
                return unknown_entity(entity_type_id);
            };
            if !locked {
                let Some(position) = self
                    .entities
                    .next_local_equine_feeding_particle_position(entity_id)
                else {
                    return unknown_entity(entity_type_id);
                };
                Some(LocalInteractionParticleAction::EquineBabyFeeding {
                    position_bits: [
                        position.x.to_bits(),
                        position.y.to_bits(),
                        position.z.to_bits(),
                    ],
                })
            } else {
                None
            }
        } else {
            None
        };
        // Unlike AbstractHorse/Llama, Camel returns item-used when the client
        // can age a baby even though only the server applies the age change.
        let item_used = healed || particle_action.is_some();
        if item_used && !self.consume_local_item_in_hand(hand, 1) {
            return unsupported(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_equine_food(entity_id, item_used, particle_action),
        ))
    }

    fn predict_local_animal_with_eating_sound(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        let is_food = match self.local_item_matches_entity_food(entity_type_id, held_item_id, owner)
        {
            Ok(is_food) => is_food,
            Err(prediction) => return prediction,
        };
        let result = self.predict_local_animal_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            owner,
        );
        if !is_food {
            return result;
        }
        match result {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(mut success)) => {
                let repetitions = match success.sound_action {
                    Some(LocalInteractionSoundAction::SuppressedAnimalEating {
                        repetitions,
                        ..
                    }) => repetitions.saturating_add(1),
                    _ => 1,
                };
                success.sound_action = Some(LocalInteractionSoundAction::SuppressedAnimalEating {
                    entity_id,
                    repetitions,
                });
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success))
            }
            _ => result,
        }
    }

    fn predict_local_cat_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_stack: Option<&ProtocolItemStackSummary>,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some((tamed, owned)) = self.local_tamable_state(entity_id, entity_type_id) else {
            return unknown_entity(entity_type_id);
        };
        let is_food = match self.local_item_matches_entity_food(entity_type_id, held_item_id, CAT) {
            Ok(is_food) => is_food,
            Err(prediction) => return prediction,
        };
        if tamed && owned {
            match self.local_item_matches_tag(held_item_id, CAT_COLLAR_DYES_TAG, CAT) {
                Ok(Some(true)) => {
                    if let Some(stack) = held_stack {
                        let effective = self.effective_item_stack(stack);
                        if let Some(color) = effective.component_patch.dye_color_id {
                            let Some(accessors) =
                                crate::entities::vanilla_entity_metadata_cat_accessors_for_entity_type(
                                    entity_type_id,
                                )
                            else {
                                return unknown_entity(entity_type_id);
                            };
                            let Some(current) = self.entities.metadata_int_for_entity(
                                entity_id,
                                accessors.collar_color_data_id,
                                accessors.collar_color_default,
                            ) else {
                                return unknown_entity(entity_type_id);
                            };
                            if normalize_vanilla_dye_color_id(color)
                                != normalize_vanilla_dye_color_id(current)
                            {
                                return client_item_success();
                            }
                        }
                    }
                }
                Err(prediction) => return prediction,
                _ => {}
            }
            if is_food {
                let Some(health) = self.entities.living_entity_health(entity_id) else {
                    return unknown_entity(entity_type_id);
                };
                let Some(max_health) = self
                    .entities
                    .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
                else {
                    return unknown_entity(entity_type_id);
                };
                if health < max_health as f32 {
                    return client_item_success();
                }
            }
            let parent = self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                CAT,
            );
            if prediction_consumes_action(parent) {
                return parent;
            }
            if self
                .entities
                .toggle_local_ordered_to_sit(entity_id)
                .is_none()
            {
                return unknown_entity(entity_type_id);
            }
            return client_item_success();
        }
        if !tamed && is_food {
            return client_item_success();
        }
        self.predict_local_animal_mob_interact(entity_id, entity_type_id, hand, held_item_id, CAT)
    }

    fn predict_local_panda_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(accessors) =
            vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id)
        else {
            return unknown_entity(entity_type_id);
        };
        let Some(main_gene) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.main_gene_data_id,
            accessors.main_gene_default,
        ) else {
            return unknown_entity(entity_type_id);
        };
        if main_gene == 2 && self.is_thundering() {
            return local_pass();
        }
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.flags_data_id,
            accessors.flags_default,
        ) else {
            return unknown_entity(entity_type_id);
        };
        if flags & accessors.on_back_mask != 0 {
            if self
                .entities
                .set_local_metadata_byte(
                    entity_id,
                    accessors.flags_data_id,
                    flags & !accessors.on_back_mask,
                )
                .is_none()
            {
                return unknown_entity(entity_type_id);
            }
            return client_item_success();
        }
        let is_food = match self.local_item_matches_entity_food(entity_type_id, held_item_id, PANDA)
        {
            Ok(is_food) => is_food,
            Err(prediction) => return prediction,
        };
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if is_food {
            if !is_baby {
                return local_pass();
            }
            let Some((data_id, default)) =
                vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
            else {
                return unknown_entity(entity_type_id);
            };
            let Some(locked) = self
                .entities
                .metadata_bool_for_entity(entity_id, data_id, default)
            else {
                return unknown_entity(entity_type_id);
            };
            if locked {
                return local_pass();
            }
            match self.use_local_mob_player_item(entity_type_id, hand, held_item_id, PANDA) {
                Ok(false) => {}
                Ok(true) | Err(_) => return unsupported(PANDA),
            }
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction(),
            ));
        }
        if is_baby && self.local_player_holds_item(VANILLA_ITEM_GOLDEN_DANDELION_ID) {
            self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                PANDA,
            )
        } else {
            local_pass()
        }
    }

    fn predict_local_parrot_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some((tamed, owned)) = self.local_tamable_state(entity_id, entity_type_id) else {
            return unknown_entity(entity_type_id);
        };
        match self.local_item_matches_tag(held_item_id, PARROT_FOOD_TAG, PARROT) {
            Ok(Some(true)) if !tamed => {
                match self.use_local_mob_player_item(entity_type_id, hand, held_item_id, PARROT) {
                    Ok(false) => {}
                    Ok(true) | Err(_) => return unsupported(PARROT),
                }
                let Some(silent) = self.entities.is_silent(entity_id) else {
                    return unknown_entity(entity_type_id);
                };
                return success_with_sound(LocalInteractionSoundAction::SuppressedMobInteraction {
                    entity_id,
                    silent,
                    random_float_count: 2,
                    random_before_silent_check: false,
                });
            }
            Err(prediction) => return prediction,
            _ => {}
        }
        match self.local_item_matches_tag(held_item_id, PARROT_POISONOUS_FOOD_TAG, PARROT) {
            Ok(Some(true)) => {
                match self.use_local_mob_player_item(entity_type_id, hand, held_item_id, PARROT) {
                    Ok(false) => {}
                    Ok(true) | Err(_) => return unsupported(PARROT),
                }
                if !self.apply_local_mob_effect(entity_id, VANILLA_MOB_EFFECT_POISON_ID, 900) {
                    return unsupported(PARROT);
                }
                return client_item_success();
            }
            Err(prediction) => return prediction,
            _ => {}
        }
        let Some(on_ground) = self.entities.on_ground(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if on_ground == Some(true) && tamed && owned {
            return client_item_success();
        }
        self.predict_local_animal_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            PARROT,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn predict_local_generated_horse_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
        chested: bool,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        let Some(is_tamed) = self.local_abstract_horse_is_tamed(entity_id, entity_type_id) else {
            return unknown_entity(entity_type_id);
        };
        let normal_branch = !self.entities.entity_has_passengers(entity_id)
            && !(!is_baby && is_tamed && secondary_use_active)
            && (!is_baby || !self.local_player_holds_item(VANILLA_ITEM_GOLDEN_DANDELION_ID));
        if normal_branch {
            if let Some(item_id) = held_item_id {
                let is_food = match self.local_item_matches_entity_food(
                    entity_type_id,
                    held_item_id,
                    owner,
                ) {
                    Ok(is_food) => is_food,
                    Err(prediction) => return prediction,
                };
                if is_food {
                    if matches!(
                        entity_type_id,
                        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
                    ) {
                        return self.predict_local_llama_fed_food(
                            entity_id,
                            entity_type_id,
                            hand,
                            item_id,
                            owner,
                        );
                    }
                    return self.predict_local_abstract_horse_fed_food(
                        entity_id,
                        entity_type_id,
                        hand,
                        item_id,
                        owner,
                    );
                }
                if !is_tamed {
                    return client_item_success();
                }
                if chested && item_id == VANILLA_ITEM_CHEST_ID {
                    let Some((data_id, default)) =
                        vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(
                            entity_type_id,
                        )
                    else {
                        return unknown_entity(entity_type_id);
                    };
                    let Some(has_chest) = self
                        .entities
                        .metadata_bool_for_entity(entity_id, data_id, default)
                    else {
                        return unknown_entity(entity_type_id);
                    };
                    if !has_chest {
                        if self
                            .entities
                            .set_local_metadata_bool(entity_id, data_id, true)
                            .is_none()
                            || !self.consume_local_item_in_hand(hand, 1)
                        {
                            return unsupported(owner);
                        }
                        let Some(silent) = self.entities.is_silent(entity_id) else {
                            return unknown_entity(entity_type_id);
                        };
                        return success_with_sound(
                            LocalInteractionSoundAction::SuppressedMobInteraction {
                                entity_id,
                                silent,
                                random_float_count: 2,
                                random_before_silent_check: true,
                            },
                        );
                    }
                }
            }
        }
        self.predict_local_abstract_horse_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            secondary_use_active,
            owner,
            true,
        )
    }

    fn predict_local_llama_fed_food(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        item_id: i32,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        let (heal, age_up) = if item_id == VANILLA_ITEM_WHEAT_ID {
            (2.0, 10)
        } else if item_id == VANILLA_ITEM_HAY_BLOCK_ID {
            (10.0, 90)
        } else {
            return unsupported(owner);
        };
        let Some(health) = self.entities.living_entity_health(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        let Some(max_health) = self
            .entities
            .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        else {
            return unknown_entity(entity_type_id);
        };
        let healed = if health < max_health as f32 {
            let Some(healed) = self.entities.apply_local_living_heal(entity_id, heal) else {
                return unknown_entity(entity_type_id);
            };
            healed
        } else {
            false
        };
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        let particle_action = if is_baby && age_up > 0 {
            let Some((data_id, default)) =
                vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
            else {
                return unknown_entity(entity_type_id);
            };
            let Some(locked) = self
                .entities
                .metadata_bool_for_entity(entity_id, data_id, default)
            else {
                return unknown_entity(entity_type_id);
            };
            if !locked {
                let Some(position) = self
                    .entities
                    .next_local_equine_feeding_particle_position(entity_id)
                else {
                    return unknown_entity(entity_type_id);
                };
                Some(LocalInteractionParticleAction::EquineBabyFeeding {
                    position_bits: [
                        position.x.to_bits(),
                        position.y.to_bits(),
                        position.z.to_bits(),
                    ],
                })
            } else {
                None
            }
        } else {
            None
        };
        // `Llama.handleEating` emits the baby particle on both sides, but only
        // the server applies `ageUp` and marks that branch as item-used.
        // Breeding/temper are server-owned by this prediction boundary.
        let item_used = healed;
        if item_used && !self.consume_local_item_in_hand(hand, 1) {
            return unsupported(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_equine_food(entity_id, item_used, particle_action),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn predict_local_saddled_animal_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_stack: Option<&ProtocolItemStackSummary>,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        let is_food = match self.local_item_matches_entity_food(entity_type_id, held_item_id, owner)
        {
            Ok(is_food) => is_food,
            Err(prediction) => return prediction,
        };
        if !is_food
            && self.entities.saddle_slot_contains_saddle_item(
                entity_id,
                &self.items.default_item_equipment_slots,
            )
            && !self.entities.entity_has_passengers(entity_id)
            && !secondary_use_active
        {
            return client_item_success();
        }
        let mut parent = self.predict_local_animal_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            owner,
        );
        if owner == STRIDER && is_food {
            if let LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                ref mut success,
            )) = parent
            {
                let Some(silent) = self.entities.is_silent(entity_id) else {
                    return unknown_entity(entity_type_id);
                };
                success.sound_action =
                    Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                        entity_id,
                        silent,
                        random_float_count: 2,
                        random_before_silent_check: false,
                    });
            }
        }
        if prediction_consumes_action(parent) {
            return parent;
        }
        let Some(stack) = held_stack else {
            return parent;
        };
        if self.effective_item_equipment_slot(stack) != Some(EquipmentSlot::Saddle) {
            return parent;
        }
        self.predict_local_item_stack_interact_living_entity(entity_id, entity_type_id, stack)
    }

    fn predict_local_mooshroom_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if held_item_id == Some(VANILLA_ITEM_BOWL_ID) && !is_baby {
            let Some(ingredient) = self.entities.local_mooshroom_stew_ingredient(entity_id) else {
                return unknown_entity(entity_type_id);
            };
            let mut stew = one_item_stack(if ingredient.is_some() {
                VANILLA_ITEM_SUSPICIOUS_STEW_ID
            } else {
                VANILLA_ITEM_MUSHROOM_STEW_ID
            });
            if let Some(ingredient) = ingredient {
                let Some(effect) = suspicious_stew_effect_for_item(ingredient) else {
                    return unsupported(VANILLA_MOOSHROOM_MOB_INTERACT_OWNER);
                };
                stew.component_patch
                    .added_type_ids
                    .push(VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID);
                stew.component_patch.added = 1;
                stew.component_patch.suspicious_stew_effects = vec![effect];
            }
            let Some(dropped) = self.replace_local_item_with_unlimited_creative_filled_result(
                hand,
                VANILLA_ITEM_BOWL_ID,
                stew,
            ) else {
                return unsupported(VANILLA_MOOSHROOM_MOB_INTERACT_OWNER);
            };
            if self
                .entities
                .set_local_mooshroom_stew_ingredient(entity_id, None)
                .is_none()
            {
                return unknown_entity(entity_type_id);
            }
            let Some(silent) = self.entities.is_silent(entity_id) else {
                return unknown_entity(entity_type_id);
            };
            return item_success_with_actions(
                dropped.then_some(InteractionHand::MainHand),
                Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                    entity_id,
                    silent,
                    random_float_count: 0,
                    random_before_silent_check: false,
                }),
                None,
            );
        }

        let Some((variant_data_id, variant_default)) =
            vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(entity_type_id)
        else {
            return unknown_entity(entity_type_id);
        };
        let Some(variant) =
            self.entities
                .metadata_int_for_entity(entity_id, variant_data_id, variant_default)
        else {
            return unknown_entity(entity_type_id);
        };
        if variant == 1 && !is_baby {
            if let Some(item_id) = held_item_id {
                if suspicious_stew_effect_for_item(item_id).is_some() {
                    let Some(existing) = self.entities.local_mooshroom_stew_ingredient(entity_id)
                    else {
                        return unknown_entity(entity_type_id);
                    };
                    if existing.is_some() {
                        let Some(particle_action) =
                            self.local_mooshroom_particle_action(entity_id, false)
                        else {
                            return unknown_entity(entity_type_id);
                        };
                        return item_success_with_actions(None, None, Some(particle_action));
                    }
                    if !self.consume_local_item_in_hand(hand, 1)
                        || self
                            .entities
                            .set_local_mooshroom_stew_ingredient(entity_id, Some(item_id))
                            .is_none()
                    {
                        return unsupported(VANILLA_MOOSHROOM_MOB_INTERACT_OWNER);
                    }
                    let Some(particle_action) =
                        self.local_mooshroom_particle_action(entity_id, true)
                    else {
                        return unknown_entity(entity_type_id);
                    };
                    let Some(silent) = self.entities.is_silent(entity_id) else {
                        return unknown_entity(entity_type_id);
                    };
                    return item_success_with_actions(
                        None,
                        Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                            entity_id,
                            silent,
                            random_float_count: 0,
                            random_before_silent_check: false,
                        }),
                        Some(particle_action),
                    );
                }
            }
        }
        self.predict_local_cow_mob_interact(entity_id, entity_type_id, hand, held_item_id)
    }

    fn local_mooshroom_particle_action(
        &mut self,
        entity_id: i32,
        effect: bool,
    ) -> Option<LocalInteractionParticleAction> {
        let origin = self
            .entities
            .local_interaction_particle_origin(entity_id, 0.5)?;
        if effect {
            let mut positions = [[0_u64; 3]; 4];
            let mut velocity_y = [0_u64; 4];
            for index in 0..4 {
                let x = origin.x
                    + self
                        .entities
                        .next_local_interaction_random_double(entity_id)?
                        / 2.0;
                let z = origin.z
                    + self
                        .entities
                        .next_local_interaction_random_double(entity_id)?
                        / 2.0;
                let velocity = self
                    .entities
                    .next_local_interaction_random_double(entity_id)?
                    / 5.0;
                positions[index] = [x.to_bits(), origin.y.to_bits(), z.to_bits()];
                velocity_y[index] = velocity.to_bits();
            }
            Some(LocalInteractionParticleAction::MooshroomEffect {
                position_bits: positions,
                velocity_y_bits: velocity_y,
            })
        } else {
            let mut positions = [[0_u64; 3]; 2];
            let mut velocity_y = [0_u64; 2];
            for index in 0..2 {
                let x = origin.x
                    + self
                        .entities
                        .next_local_interaction_random_double(entity_id)?
                        / 2.0;
                let z = origin.z
                    + self
                        .entities
                        .next_local_interaction_random_double(entity_id)?
                        / 2.0;
                let velocity = self
                    .entities
                    .next_local_interaction_random_double(entity_id)?
                    / 5.0;
                positions[index] = [x.to_bits(), origin.y.to_bits(), z.to_bits()];
                velocity_y[index] = velocity.to_bits();
            }
            Some(LocalInteractionParticleAction::MooshroomSmoke {
                position_bits: positions,
                velocity_y_bits: velocity_y,
            })
        }
    }

    fn predict_local_tadpole_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some((locked_data_id, locked_default)) =
            vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
        else {
            return unknown_entity(entity_type_id);
        };
        let Some(locked) =
            self.entities
                .metadata_bool_for_entity(entity_id, locked_data_id, locked_default)
        else {
            return unknown_entity(entity_type_id);
        };
        match self.local_item_matches_tag(held_item_id, FROG_FOOD_TAG, TADPOLE) {
            Ok(Some(true)) if !locked => {
                if !self.consume_local_item_in_hand(hand, 1)
                    || self.entities.feed_local_tadpole(entity_id).is_none()
                {
                    return unsupported(TADPOLE);
                }
                let Some(position) = self
                    .entities
                    .local_interaction_random_position(entity_id, 1.0, 0.5)
                else {
                    return unknown_entity(entity_type_id);
                };
                return item_success_with_actions(
                    None,
                    None,
                    Some(LocalInteractionParticleAction::TadpoleFeeding {
                        position_bits: [
                            position.x.to_bits(),
                            position.y.to_bits(),
                            position.z.to_bits(),
                        ],
                    }),
                );
            }
            Err(prediction) => return prediction,
            _ => {}
        }
        if held_item_id == Some(VANILLA_ITEM_GOLDEN_DANDELION_ID) {
            let Some(timer_ticks) = self.entities.age_lock_particle_timer_ticks(entity_id) else {
                return unknown_entity(entity_type_id);
            };
            let Some(excluded_entity_type_ids) = self.registry_holder_ids_for_tag(
                "minecraft:entity_type",
                VANILLA_CANNOT_BE_AGE_LOCKED_TAG,
            ) else {
                return unsupported(TADPOLE);
            };
            if timer_ticks == 0 && !excluded_entity_type_ids.contains(&entity_type_id) {
                if self.entities.apply_local_age_lock_use(entity_id).is_none()
                    || self.entities.reset_local_tadpole_age(entity_id).is_none()
                    || !self.consume_local_item_in_hand(hand, 1)
                {
                    return unsupported(TADPOLE);
                }
                return item_success_with_actions(
                    None,
                    Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                        entity_id,
                        silent: false,
                        random_float_count: 0,
                        random_before_silent_check: false,
                    }),
                    None,
                );
            }
        }
        if held_item_id == Some(VANILLA_ITEM_WATER_BUCKET_ID) {
            return self.predict_local_bucketable_pickup(entity_id, entity_type_id, hand, TADPOLE);
        }
        local_pass()
    }

    fn predict_local_wolf_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_stack: Option<&ProtocolItemStackSummary>,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some((tamed, owned)) = self.local_tamable_state(entity_id, entity_type_id) else {
            return unknown_entity(entity_type_id);
        };
        if !tamed {
            return self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                WOLF,
            );
        }

        let is_food = match self.local_item_matches_entity_food(entity_type_id, held_item_id, WOLF)
        {
            Ok(is_food) => is_food,
            Err(prediction) => return prediction,
        };
        if is_food {
            let Some(health) = self.entities.living_entity_health(entity_id) else {
                return unknown_entity(entity_type_id);
            };
            let Some(max_health) = self
                .entities
                .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            else {
                return unknown_entity(entity_type_id);
            };
            if health < max_health as f32 {
                let heal = held_stack
                    .map(|stack| self.effective_item_stack(stack))
                    .and_then(|stack| stack.component_patch.food_nutrition)
                    .map_or(2.0, |nutrition| nutrition as f32 * 2.0);
                match self.use_local_mob_player_item(entity_type_id, hand, held_item_id, WOLF) {
                    Ok(false) => {}
                    Ok(true) | Err(_) => return unsupported(WOLF),
                }
                if self
                    .entities
                    .apply_local_living_heal(entity_id, heal)
                    .is_none()
                {
                    return unsupported(WOLF);
                }
                let eating_policy = vanilla_entity_metadata_eating_sound_policy(entity_type_id);
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    if eating_policy == Some(VanillaEntityEatingSoundPolicy::None) {
                        LocalInteractionSuccess::client_item_interaction()
                    } else {
                        LocalInteractionSuccess::client_item_interaction_with_eating(entity_id, 1)
                    },
                ));
            }
        }

        let collar_dye = match self.local_item_matches_tag(held_item_id, WOLF_COLLAR_DYES_TAG, WOLF)
        {
            Ok(Some(true)) => true,
            Ok(_) => false,
            Err(prediction) => return prediction,
        };
        if collar_dye && owned {
            if let Some(stack) = held_stack {
                let effective = self.effective_item_stack(stack);
                if let Some(color) = effective.component_patch.dye_color_id {
                    let Some(accessors) =
                        vanilla_entity_metadata_wolf_accessors_for_entity_type(entity_type_id)
                    else {
                        return unknown_entity(entity_type_id);
                    };
                    let Some(current) = self.entities.metadata_int_for_entity(
                        entity_id,
                        accessors.collar_color_data_id,
                        accessors.collar_color_default,
                    ) else {
                        return unknown_entity(entity_type_id);
                    };
                    let color = normalize_vanilla_dye_color_id(color);
                    if color != normalize_vanilla_dye_color_id(current) {
                        if self
                            .entities
                            .set_local_metadata_int(
                                entity_id,
                                accessors.collar_color_data_id,
                                color,
                            )
                            .is_none()
                            || !self.consume_local_item_in_hand(hand, 1)
                        {
                            return unsupported(WOLF);
                        }
                        return client_item_success();
                    }
                }
            }
            return self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                WOLF,
            );
        }

        let body_armor = self.entities.equipment_item(entity_id, EquipmentSlot::Body);
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return unknown_entity(entity_type_id);
        };
        if owned && !is_baby && body_armor.is_none() {
            if let Some(stack) = held_stack {
                match self.local_item_stack_is_equippable_in_horse_body_slot(entity_type_id, stack)
                {
                    Ok(true) => {
                        let mut equipped = stack.clone();
                        equipped.count = 1;
                        if !self.consume_local_item_in_hand(hand, 1)
                            || !self.entities.set_local_equipment_item(
                                entity_id,
                                EquipmentSlot::Body,
                                equipped,
                            )
                        {
                            return unsupported(WOLF);
                        }
                        return client_item_success();
                    }
                    Err(prediction) => return prediction,
                    Ok(false) => {}
                }
            }
        }

        let tamable_accessors =
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id);
        let Some(tamable_accessors) = tamable_accessors else {
            return unknown_entity(entity_type_id);
        };
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            tamable_accessors.flags_data_id,
            tamable_accessors.flags_default,
        ) else {
            return unknown_entity(entity_type_id);
        };
        if owned && flags & tamable_accessors.sitting_mask != 0 {
            if let (Some(mut armor), Some(repair_item_id)) = (body_armor, held_item_id) {
                let Some((damage, max_damage)) = self.local_stack_damage(&armor) else {
                    return unsupported(WOLF);
                };
                if damage > 0 {
                    let valid_repair =
                        match self.local_wolf_armor_accepts_repair(&armor, repair_item_id) {
                            Some(valid) => valid,
                            None => return unsupported(WOLF),
                        };
                    if valid_repair {
                        let repair_unit = (max_damage as f32 * 0.125) as i32;
                        set_stack_damage(&mut armor, (damage - repair_unit).max(0));
                        if !self.shrink_local_item_in_hand(hand, 1)
                            || !self.entities.set_local_equipment_item(
                                entity_id,
                                EquipmentSlot::Body,
                                armor,
                            )
                        {
                            return unsupported(WOLF);
                        }
                        let Some(silent) = self.entities.is_silent(entity_id) else {
                            return unknown_entity(entity_type_id);
                        };
                        return success_with_sound(
                            LocalInteractionSoundAction::SuppressedMobInteraction {
                                entity_id,
                                silent,
                                random_float_count: 0,
                                random_before_silent_check: false,
                            },
                        );
                    }
                }
            }
        }

        let parent = self.predict_local_animal_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            WOLF,
        );
        if prediction_consumes_action(parent) || !owned {
            return parent;
        }
        if self
            .entities
            .toggle_local_ordered_to_sit(entity_id)
            .is_none()
        {
            return unknown_entity(entity_type_id);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_without_item(),
        ))
    }

    fn local_stack_damage(&self, stack: &ProtocolItemStackSummary) -> Option<(i32, i32)> {
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
            || !self
                .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)
        {
            return Some((0, 0));
        }
        let max_damage = if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
        {
            stack.component_patch.max_damage
        } else {
            stack
                .item_id
                .and_then(|item_id| self.items.default_item_max_damage.get(&item_id).copied())
        }?;
        let damage = if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)
        {
            stack.component_patch.damage.unwrap_or_default()
        } else {
            0
        };
        Some((damage.clamp(0, max_damage), max_damage))
    }

    fn local_wolf_armor_accepts_repair(
        &self,
        armor: &ProtocolItemStackSummary,
        repair_item_id: i32,
    ) -> Option<bool> {
        if armor
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID)
        {
            return Some(false);
        }
        if armor
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID)
        {
            if armor
                .component_patch
                .repairable_item_registry_ids
                .contains(&repair_item_id)
            {
                return Some(true);
            }
            if let Some(tag) = armor.component_patch.repairable_item_tag.as_deref() {
                return self
                    .registry_holder_ids_for_tag("minecraft:item", tag)
                    .map(|items| items.contains(&repair_item_id));
            }
            return Some(false);
        }
        let armor_item_id = armor.item_id?;
        if !self
            .items
            .default_wolf_body_armor_materials
            .contains_key(&armor_item_id)
        {
            return Some(false);
        }
        self.registry_holder_ids_for_tag("minecraft:item", REPAIRS_WOLF_ARMOR_TAG)
            .map(|items| items.contains(&repair_item_id))
    }

    fn local_item_matches_entity_food(
        &self,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        owner: &'static str,
    ) -> Result<bool, LocalTargetUsePrediction> {
        match vanilla_entity_metadata_food_predicate(entity_type_id) {
            Some(VanillaEntityFoodPredicate::Never) => Ok(false),
            Some(VanillaEntityFoodPredicate::ItemTag(tag)) => self
                .local_item_matches_tag(held_item_id, tag, owner)
                .map(|matched| matched == Some(true)),
            _ => Err(unsupported(owner)),
        }
    }

    fn local_item_matches_tag(
        &self,
        held_item_id: Option<i32>,
        tag: &'static str,
        owner: &'static str,
    ) -> Result<Option<bool>, LocalTargetUsePrediction> {
        let Some(item_id) = held_item_id else {
            return Ok(None);
        };
        let Some(items) = self.registry_holder_ids_for_tag("minecraft:item", tag) else {
            return Err(unsupported(owner));
        };
        Ok(Some(items.contains(&item_id)))
    }

    fn local_tamable_state(&self, entity_id: i32, entity_type_id: i32) -> Option<(bool, bool)> {
        let accessors =
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)?;
        let flags = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.flags_data_id,
            accessors.flags_default,
        )?;
        let owner = self
            .entities
            .metadata_optional_living_entity_reference_for_entity(
                entity_id,
                accessors.owner_data_id,
                accessors.owner_default,
            )?;
        Some((
            flags & accessors.tame_mask != 0,
            owner.is_some() && owner == self.local_player_profile_id(),
        ))
    }
}

fn owner_for_camel(husk: bool) -> &'static str {
    if husk {
        VANILLA_CAMEL_HUSK_INTERACT_OWNER
    } else {
        CAMEL
    }
}

fn local_pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}

fn unknown_entity(entity_type_id: i32) -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id))
}

fn unsupported(owner: &'static str) -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::UnsupportedEntityInteract(owner)
}

fn client_item_success() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ))
}

fn success_with_sound(action: LocalInteractionSoundAction) -> LocalTargetUsePrediction {
    item_success_with_actions(None, Some(action), None)
}

fn item_success_with_actions(
    extra_swing: Option<InteractionHand>,
    sound_action: Option<LocalInteractionSoundAction>,
    particle_action: Option<LocalInteractionParticleAction>,
) -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
        swing_source: LocalInteractionSwingSource::Client,
        was_item_interaction: true,
        starts_using_item: false,
        extra_swing,
        sound_action,
        particle_action,
    }))
}

fn one_item_stack(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    }
}

fn prediction_consumes_action(prediction: LocalTargetUsePrediction) -> bool {
    matches!(
        prediction,
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    )
}

fn nbt_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn set_stack_damage(stack: &mut ProtocolItemStackSummary, damage: i32) {
    stack
        .component_patch
        .removed_type_ids
        .retain(|type_id| *type_id != VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    if !stack
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)
    {
        stack
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    }
    stack.component_patch.damage = Some(damage.max(0));
    stack.component_patch.added = stack.component_patch.added_type_ids.len();
}

fn suspicious_stew_effect_for_item(item_id: i32) -> Option<SuspiciousStewEffectSummary> {
    let ingredient = vanilla_suspicious_stew_ingredient(item_id)?;
    Some(SuspiciousStewEffectSummary {
        effect_id: ingredient.effect_type_id,
        duration: ingredient.duration_ticks,
    })
}
