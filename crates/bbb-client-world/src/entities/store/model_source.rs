use super::*;

impl EntityStore {
    pub(crate) fn model_source(
        &self,
        id: i32,
        position: crate::entities::EntityVec3,
        partial_ticks: f32,
        minecart_improvements_enabled: bool,
        registries: &RegistrySet,
        profiles: &crate::ItemProfiles,
    ) -> Option<EntityRenderSource> {
        let equipment_asset_generation = profiles.equipment_asset_generation;
        let equipment_asset_names = &profiles.equipment_asset_names;
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = &entity.transform;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let client_animations = &entity.animations;
        let equipment = Some(&entity.equipment);
        let mount = Some(&entity.mount);
        let minecart_lerp = entity.family.minecart_lerp();
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let armor_item = |slot: ProtocolEquipmentSlot| -> Option<&ItemStackSummary> {
            equipment_slot_item(equipment, slot)
        };
        let resolved_equipment_asset = |item: &ItemStackSummary,
                                        required_slot: Option<ProtocolEquipmentSlot>|
         -> Option<EntityEquipmentAssetId> {
            effective_equipment_asset(item, required_slot, profiles)
        };
        let armor_equipment_asset =
            |slot: ProtocolEquipmentSlot| -> Option<EntityEquipmentAssetId> {
                resolved_equipment_asset(armor_item(slot)?, Some(slot))
            };
        let armor_equipment_asset_name = |slot: ProtocolEquipmentSlot| -> Option<&str> {
            equipment_asset_name(
                equipment_asset_generation,
                equipment_asset_names,
                armor_equipment_asset(slot)?,
            )
        };
        // The per-slot worn item's packed-RGB `dyed_color` component is carried beside the asset
        // id; only catalog layers declared dyeable consume it. A slot without a dye resolves to
        // None.
        let armor_dye = |slot: ProtocolEquipmentSlot| -> Option<u32> {
            armor_item(slot)?
                .component_patch
                .dyed_color
                .map(|dye| dye as u32)
        };
        let armor_foil = |slot: ProtocolEquipmentSlot| -> bool {
            armor_equipment_asset(slot).is_some()
                && armor_item(slot).is_some_and(ItemStackSummary::has_foil)
        };
        let armor_trim = |slot: ProtocolEquipmentSlot| {
            let asset_name = armor_equipment_asset_name(slot)?;
            armor_trim_sprite(
                registries,
                armor_item(slot)?,
                asset_name,
                if slot == ProtocolEquipmentSlot::Legs {
                    ArmorTrimLayer::HumanoidLeggings
                } else {
                    ArmorTrimLayer::Humanoid
                },
            )
        };
        let chest_wings_equipment_asset = armor_item(ProtocolEquipmentSlot::Chest)
            .and_then(|item| resolved_equipment_asset(item, None));
        let chest_wings_asset_name = chest_wings_equipment_asset.and_then(|asset| {
            equipment_asset_name(equipment_asset_generation, equipment_asset_names, asset)
        });
        let chest_wings_trim = chest_wings_asset_name.and_then(|equipment_asset| {
            armor_trim_sprite(
                registries,
                armor_item(ProtocolEquipmentSlot::Chest)?,
                equipment_asset,
                ArmorTrimLayer::Wings,
            )
        });
        let chest_wings_foil = chest_wings_asset_name
            .and_then(|_| armor_item(ProtocolEquipmentSlot::Chest))
            .is_some_and(ItemStackSummary::has_foil);
        // Vanilla `SimpleEquipmentLayer` saddle users copy `EquipmentSlot.SADDLE` into render
        // state, then render only through the stack's generation-local equipment asset.
        let saddle_equipment_item =
            || equipment_slot_item(equipment, ProtocolEquipmentSlot::Saddle);
        let saddle_equipment_asset =
            saddle_equipment_item().and_then(|item| resolved_equipment_asset(item, None));
        let consumer_saddle_asset =
            |enabled: bool| enabled.then_some(saddle_equipment_asset).flatten();
        let saddle_trim = |asset: Option<EntityEquipmentAssetId>, layer| {
            let asset_name = asset.and_then(|asset| {
                equipment_asset_name(equipment_asset_generation, equipment_asset_names, asset)
            })?;
            armor_trim_sprite(registries, saddle_equipment_item()?, asset_name, layer)
        };
        let saddle_foil = |asset: Option<EntityEquipmentAssetId>| {
            asset.is_some() && saddle_equipment_item().is_some_and(ItemStackSummary::has_foil)
        };
        let saddle_dye = |asset: Option<EntityEquipmentAssetId>| {
            asset
                .and_then(|_| saddle_equipment_item())
                .and_then(|item| item.component_patch.dyed_color)
                .map(|dye| dye as u32)
        };
        let pig_saddle_type = identity.entity_type_id == VANILLA_ENTITY_TYPE_PIG_ID;
        let pig_saddle_equipment_asset = consumer_saddle_asset(pig_saddle_type);
        let pig_saddle_dye = saddle_dye(pig_saddle_equipment_asset);
        let pig_saddle_trim = saddle_trim(pig_saddle_equipment_asset, ArmorTrimLayer::PigSaddle);
        let pig_saddle_foil = saddle_foil(pig_saddle_equipment_asset);
        let equine_saddle_type =
            vanilla_entity_renderer_uses_abstract_horse_renderer(identity.entity_type_id);
        let equine_saddle_equipment_asset = consumer_saddle_asset(equine_saddle_type);
        let equine_saddle_dye = saddle_dye(equine_saddle_equipment_asset);
        let equine_saddle_trim_layer = match identity.entity_type_id {
            VANILLA_ENTITY_TYPE_HORSE_ID => Some(ArmorTrimLayer::HorseSaddle),
            VANILLA_ENTITY_TYPE_DONKEY_ID => Some(ArmorTrimLayer::DonkeySaddle),
            VANILLA_ENTITY_TYPE_MULE_ID => Some(ArmorTrimLayer::MuleSaddle),
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => Some(ArmorTrimLayer::SkeletonHorseSaddle),
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => Some(ArmorTrimLayer::ZombieHorseSaddle),
            _ => None,
        };
        let equine_saddle_trim = equine_saddle_trim_layer
            .and_then(|layer| saddle_trim(equine_saddle_equipment_asset, layer));
        let equine_saddle_foil = saddle_foil(equine_saddle_equipment_asset);
        let equine_saddle_ridden = equine_saddle_type
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let strider_ridden = identity.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let strider_saddle_type = identity.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID;
        let strider_saddle_equipment_asset = consumer_saddle_asset(strider_saddle_type);
        let strider_saddle_dye = saddle_dye(strider_saddle_equipment_asset);
        let strider_saddle_trim = saddle_trim(
            strider_saddle_equipment_asset,
            ArmorTrimLayer::StriderSaddle,
        );
        let strider_saddle_foil = saddle_foil(strider_saddle_equipment_asset);
        let camel_saddle_type = vanilla_camel_saddle_type(identity.entity_type_id);
        let camel_saddle_equipment_asset = consumer_saddle_asset(camel_saddle_type);
        let camel_saddle_dye = saddle_dye(camel_saddle_equipment_asset);
        let camel_saddle_trim_layer =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
                ArmorTrimLayer::CamelHuskSaddle
            } else {
                ArmorTrimLayer::CamelSaddle
            };
        let camel_saddle_trim = saddle_trim(camel_saddle_equipment_asset, camel_saddle_trim_layer);
        let camel_saddle_foil = saddle_foil(camel_saddle_equipment_asset);
        let camel_saddle_ridden = camel_saddle_type
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let nautilus_saddle_type = is_vanilla_abstract_nautilus_type(identity.entity_type_id);
        let nautilus_saddle_equipment_asset = consumer_saddle_asset(nautilus_saddle_type);
        let nautilus_saddle_dye = saddle_dye(nautilus_saddle_equipment_asset);
        let nautilus_saddle_trim = saddle_trim(
            nautilus_saddle_equipment_asset,
            ArmorTrimLayer::NautilusSaddle,
        );
        let nautilus_saddle_foil = saddle_foil(nautilus_saddle_equipment_asset);
        let body_slot_item = || equipment_slot_item(equipment, ProtocolEquipmentSlot::Body);
        let body_equipment_asset =
            body_slot_item().and_then(|item| resolved_equipment_asset(item, None));
        let body_slot_item_id =
            || -> Option<i32> { body_slot_item().and_then(|item| item.item_id) };
        let body_slot_armor_dye = || -> Option<u32> {
            body_slot_item()
                .and_then(|item| item.component_patch.dyed_color)
                .map(|dye| dye as u32)
        };
        let happy_ghast_body_item = identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && body_slot_item().is_some();
        let happy_ghast_body_equipment_asset = (identity.entity_type_id
            == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID)
            .then_some(body_equipment_asset)
            .flatten();
        let happy_ghast_harness_dye =
            happy_ghast_body_equipment_asset.and_then(|_| body_slot_armor_dye());
        let happy_ghast_harness_asset_name = happy_ghast_body_equipment_asset.and_then(|asset| {
            equipment_asset_name(equipment_asset_generation, equipment_asset_names, asset)
        });
        let happy_ghast_harness_trim = happy_ghast_harness_asset_name.and_then(|asset_name| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                asset_name,
                ArmorTrimLayer::HappyGhastBody,
            )
        });
        let happy_ghast_harness_foil = happy_ghast_harness_asset_name
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let happy_ghast_body_item_is_harness = identity.entity_type_id
            == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && body_slot_item_id().is_some_and(|item_id| {
                registries.holder_is_in_tag(
                    "minecraft:item",
                    VANILLA_HAPPY_GHAST_HARNESSES_ITEM_TAG,
                    item_id,
                )
            });
        let happy_ghast_is_leash_holder = if identity.entity_type_id
            == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        {
            vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(identity.entity_type_id)
                .is_some_and(|accessors| {
                    entity_data_bool(
                        &metadata.data_values,
                        accessors.is_leash_holder_data_id,
                        accessors.is_leash_holder_default,
                    )
                })
        } else {
            false
        };
        let happy_ghast_ridden = identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let llama_body_equipment_asset = (is_vanilla_llama_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values))
        .then_some(body_equipment_asset)
        .flatten();
        let llama_body_decor_dye = llama_body_equipment_asset.and_then(|_| body_slot_armor_dye());
        let llama_body_asset_name = llama_body_equipment_asset.and_then(|asset| {
            equipment_asset_name(equipment_asset_generation, equipment_asset_names, asset)
        });
        let llama_body_decor_trim = llama_body_asset_name.and_then(|asset_name| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                asset_name,
                ArmorTrimLayer::LlamaBody,
            )
        });
        let llama_body_decor_foil = llama_body_asset_name
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let nautilus_body_item_present = is_vanilla_abstract_nautilus_type(identity.entity_type_id)
            && body_slot_item().is_some();
        let nautilus_body_equipment_asset =
            (is_vanilla_abstract_nautilus_type(identity.entity_type_id)
                && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values))
            .then_some(body_equipment_asset)
            .flatten();
        let nautilus_body_armor_dye =
            nautilus_body_equipment_asset.and_then(|_| body_slot_armor_dye());
        let nautilus_body_asset_name = nautilus_body_equipment_asset.and_then(|asset| {
            equipment_asset_name(equipment_asset_generation, equipment_asset_names, asset)
        });
        let nautilus_body_armor_trim = nautilus_body_asset_name.and_then(|asset_name| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                asset_name,
                ArmorTrimLayer::NautilusBody,
            )
        });
        let nautilus_body_armor_foil = nautilus_body_asset_name
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let horse_body_equipment_asset =
            (is_vanilla_can_wear_horse_armor_type(identity.entity_type_id)
                && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values))
            .then_some(body_equipment_asset)
            .flatten();
        let horse_body_asset_name = horse_body_equipment_asset.and_then(|asset| {
            equipment_asset_name(equipment_asset_generation, equipment_asset_names, asset)
        });
        let horse_body_armor_trim = horse_body_asset_name.and_then(|asset_name| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                asset_name,
                ArmorTrimLayer::HorseBody,
            )
        });
        let horse_body_armor_dye = horse_body_asset_name.and_then(|_| body_slot_armor_dye());
        let horse_body_armor_foil = horse_body_asset_name
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let wolf_body_equipment_asset = (identity.entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values))
        .then_some(body_equipment_asset)
        .flatten();
        let wolf_body_asset_name = wolf_body_equipment_asset.and_then(|asset| {
            equipment_asset_name(equipment_asset_generation, equipment_asset_names, asset)
        });
        let wolf_body_armor_trim = wolf_body_asset_name.and_then(|asset_name| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                asset_name,
                ArmorTrimLayer::WolfBody,
            )
        });
        let wolf_body_armor_dye = wolf_body_asset_name.and_then(|_| body_slot_armor_dye());
        let wolf_body_armor_crackiness = wolf_body_asset_name
            .and_then(|_| body_slot_item())
            .map(|item| wolf_armor_crackiness(item, profiles))
            .unwrap_or(WolfArmorCrackiness::None);
        let wolf_body_armor_foil = wolf_body_asset_name
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        // Vanilla `LivingEntityRenderer.isShaking` (base) is `Entity.isFullyFrozen`
        // (`getTicksFrozen() >= 140`), and only living entities shake.
        let is_fully_frozen = super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) && self.is_fully_frozen(id);
        // Vanilla `LivingEntityRenderer.extractRenderState`: `isInvisibleToPlayer =
        // state.isInvisible && entity.isInvisibleTo(minecraft.player)`. The store-local baseline is
        // the non-spectator / no-friendly-team-override branch, so the world aggregation can clear
        // it for spectator viewers.
        let shared_flags = self
            .metadata_byte(
                id,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            )
            .unwrap_or(entity_accessors.shared_flags_default);
        let invisible = shared_flags & entity_accessors.invisible_mask != 0;
        let appears_glowing = shared_flags & entity_accessors.glowing_mask != 0;
        // Vanilla renderer `extractRenderState` assignments project `Mob.isAggressive()` for the
        // zombie/zombie-villager/zombified-piglin, skeleton, and illager paths. PiglinRenderer
        // instead projects `entity.getArmPose()`; bbb resolves that pose from the same synced Mob
        // flag plus held-item state, so the generated AbstractPiglin class membership is composed
        // here. Other entities do not consume the bit through a supported renderer pose.
        let is_aggressive =
            (vanilla_entity_renderer_extracts_mob_aggressive(identity.entity_type_id)
                || vanilla_entity_metadata_type_is_abstract_piglin(identity.entity_type_id)
                || super::super::content_facts::mod_entity_spec(
                    identity.entity_type_id,
                    &self.content_registry,
                )
                .is_some_and(|spec| spec.is_mob))
                && super::super::content_facts::entity_mob_accessors(
                    identity.entity_type_id,
                    &self.content_registry,
                )
                .is_some_and(|accessors| {
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default)
                        & accessors.aggressive_mask
                        != 0
                });
        // Vanilla EvokerRenderer / IllusionerRenderer copy
        // `state.isCastingSpell = entity.isCastingSpell()`. Intersect that renderer assignment
        // with the generated SpellcasterIllager accessor owner before reading the shared byte id;
        // Pillager uses the same id for its BOOLEAN crossbow flag.
        let illager_spellcasting =
            vanilla_entity_renderer_extracts_casting_spell(identity.entity_type_id)
                && vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) > 0
                });
        // Vanilla `VillagerRenderer` / `WanderingTraderRenderer.extractRenderState`:
        // `state.isUnhappy = entity.getUnhappyCounter() > 0`. PandaRenderer has the same assignment
        // for Panda.UNHAPPY_COUNTER, so intersect the renderer fact with the generated
        // AbstractVillager accessor owner rather than treating the shared field spelling as one
        // type.
        let villager_unhappy =
            vanilla_entity_renderer_extracts_unhappy_counter(identity.entity_type_id)
                && vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_int(id, data_id, default).unwrap_or(default) > 0
                });
        // Vanilla `EndermanModel.setupAnim`: a carried block (`!carriedBlock.isEmpty()`,
        // the synced `DATA_CARRY_STATE`) poses both arms forward, and `isCreepy` (the
        // synced `DATA_CREEPY`) drops the head / raises the hat. Only the enderman defines
        // these accessors, so the projections are gated to it and default off otherwise.
        let enderman_accessors = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
            identity.entity_type_id,
        );
        let enderman_carried_block_state_id = enderman_accessors.and_then(|accessors| {
            self.metadata_optional_block_state(id, accessors.carried_block_state_data_id)
                .unwrap_or(accessors.carried_block_state_default)
        });
        let enderman_carrying = enderman_carried_block_state_id.is_some();
        let enderman_creepy = enderman_accessors.is_some_and(|accessors| {
            self.metadata_bool(id, accessors.creepy_data_id, accessors.creepy_default)
                .unwrap_or(accessors.creepy_default)
        });
        // Vanilla `BatModel.setupAnim` swaps to the `BAT_RESTING` hanging pose while
        // `Bat.isResting` (`DATA_ID_FLAGS & FLAG_RESTING`). Only the bat defines that flags byte,
        // so the projection is gated to it and defaults to flying otherwise.
        let bat_resting =
            vanilla_entity_metadata_bat_flags_accessor_for_entity_type(identity.entity_type_id)
                .is_some_and(|(data_id, default, resting_mask)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) & resting_mask != 0
                });
        // Vanilla `BeeModel.setupAnim` hides the stinger cube once `Bee.hasStung`
        // (`DATA_FLAGS_ID & 4`). Only the bee defines that flags byte; every other entity keeps
        // its stinger field at the `true` default (and never renders a stinger anyway).
        let bee_has_stinger =
            vanilla_entity_metadata_bee_accessors_for_entity_type(identity.entity_type_id)
                .is_none_or(|accessors| {
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default)
                        & accessors.has_stung_mask
                        == 0
                });
        // Vanilla `FoxModel.setupAnim` reads the fox's synced `DATA_FLAGS_ID` (19) crouch / sleep /
        // sit / pounce / faceplant bits directly (no easing) to pick its pose branch. Only the fox
        // defines that flags byte, so the five bool projections are gated to it and default off.
        // The interest bit (8) is read by the animation layer instead, to drive the `headRollAngle`
        // accumulator below.
        let fox_accessors =
            vanilla_entity_metadata_fox_accessors_for_entity_type(identity.entity_type_id);
        let fox_flags = fox_accessors
            .map(|accessors| {
                self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                    .unwrap_or(accessors.flags_default)
            })
            .unwrap_or(0);
        let fox_is_crouching =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.crouching_mask != 0);
        let fox_is_sleeping =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.sleeping_mask != 0);
        let fox_is_sitting =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.sitting_mask != 0);
        let fox_is_pouncing =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.pouncing_mask != 0);
        let fox_is_faceplanted =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.faceplanted_mask != 0);
        let is_feline = vanilla_entity_renderer_uses_abstract_feline_model(identity.entity_type_id);
        // Vanilla `CatRenderer` / `OcelotRenderer.extractRenderState` copies `Entity.isCrouching()`
        // and `Entity.isSprinting()` into `FelineRenderState`. The former is the synced
        // `Pose.CROUCHING`; the latter is shared entity flags bit 3. `CatRenderer` additionally
        // projects `Cat.isInSittingPose()` from the generated TamableAnimal sitting mask;
        // `OcelotRenderer` leaves `isSitting` at its default.
        let feline_is_crouching = is_feline
            && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_CROUCHING_ID;
        let feline_is_sprinting = is_feline && shared_flags & entity_accessors.sprinting_mask != 0;
        let feline_is_sitting = vanilla_is_cat(identity.entity_type_id)
            && vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|accessors| {
                self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                    .unwrap_or(accessors.flags_default)
                    & accessors.sitting_mask
                    != 0
            });
        // Vanilla `VexModel.setupAnim` levels the body (`xRot = 0`) and raises both arms
        // (`setArmsCharging`) while `Vex.isCharging` (`DATA_FLAGS_ID & FLAG_IS_CHARGING`). The
        // generated accessor owns the Vex membership, wire id, default, and named mask, so every
        // other type stays idle.
        let vex_charging =
            vanilla_entity_metadata_vex_flags_accessor_for_entity_type(identity.entity_type_id)
                .map(|(data_id, default, charging_mask)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) & charging_mask != 0
                })
                .unwrap_or(false);
        // Vanilla `SnowGolemRenderer.extractRenderState`: when `SnowGolem.hasPumpkin()` is true,
        // the renderer resolves `Blocks.CARVED_PUMPKIN.defaultBlockState()` into `state.headBlock`.
        // The generated synced-byte default has the pumpkin mask set, so a newly spawned golem
        // shows the pumpkin until the server clears that bit.
        let snow_golem_pumpkin =
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|(data_id, default, pumpkin_mask)| {
                self.metadata_byte(id, data_id, default).unwrap_or(default) & pumpkin_mask != 0
            });
        // Vanilla `WitherBossRenderer.extractRenderState`: `state.invulnerableTicks =
        // invulnerableTicks > 0 ? invulnerableTicks - partialTicks : 0.0` (the synced `DATA_ID_INV`
        // spawn countdown, lerped against the partial tick). It drives both the `scale()`
        // spawn-charge shrink and the `getTextureLocation` `wither_invulnerable.png` flicker. Only
        // the wither defines that accessor, so every other entity holds `0.0`.
        let wither_invulnerable_accessor =
            vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
                identity.entity_type_id,
            );
        let wither_invulnerable_ticks =
            if let Some((data_id, default)) = wither_invulnerable_accessor {
                let ticks = self.metadata_int(id, data_id, default).unwrap_or(default);
                if ticks > 0 {
                    ticks as f32 - partial_ticks
                } else {
                    0.0
                }
            } else {
                0.0
            };
        // Vanilla `LivingEntity.isAutoSpinAttack` (the named spin-attack bit in
        // `DATA_LIVING_ENTITY_FLAGS`):
        // a living entity mid riptide-trident spin. Non-living entities have no
        // living-entity flags byte, so they never spin.
        let living_accessors = super::super::content_facts::entity_living_accessors(
            identity.entity_type_id,
            &self.content_registry,
        );
        // Vanilla `LivingEntity.isUsingItem` plus `getUsedItemHand` read the named using/off-hand
        // bits in `DATA_LIVING_ENTITY_FLAGS`; use-item arm poses (spyglass / horn / brush) read
        // both. Non-living entities have no flags byte, so they never count as using an item.
        let (living_entity_flags, is_using_mask, off_hand_mask, spin_attack_mask) =
            living_accessors
                .map(|accessors| {
                    (
                        self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                            .unwrap_or(accessors.flags_default),
                        accessors.is_using_mask,
                        accessors.off_hand_mask,
                        accessors.spin_attack_mask,
                    )
                })
                .unwrap_or((0, 0, 0, 0));
        let is_auto_spin_attack = living_entity_flags & spin_attack_mask != 0;
        let is_using_item = living_entity_flags & is_using_mask != 0;
        let use_item_off_hand = living_entity_flags & off_hand_mask != 0;
        let main_arm_left = self.entity_main_arm_left(id, identity.entity_type_id);
        // Vanilla `LivingEntityRenderer.isEntityUpsideDown`: a non-player living
        // entity whose custom name is `Dinnerbone`/`Grumm`. The player variant
        // (`AvatarRenderer.isPlayerUpsideDown`) keys off the GameProfile name and
        // the cape model part instead, which need the player-info list, so the
        // player type is excluded here and that path stays deferred.
        let is_upside_down = super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) && identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .metadata_optional_component(id, entity_accessors.custom_name_data_id)
                .is_some_and(|name| {
                    vanilla_living_entity_renderer_name_is_upside_down(name.as_str())
                });
        // Vanilla `EntityRenderState.boundingBoxWidth` / `boundingBoxHeight`
        // project the current dimensions even for non-pickable renderers. Keep
        // this separate from `pick_bounds`, whose applicability filter would
        // collapse those entities to zero-sized flames.
        let bounding_box = super::super::content_facts::model_source_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*client_animations),
            metadata.painting_variant_projection.as_ref(),
            &self.content_registry,
        );
        let bounding_box_height = bounding_box
            .map(|bounds| bounds.max[1] - bounds.min[1])
            .unwrap_or(0.0);
        let bounding_box_width = bounding_box
            .map(|bounds| bounds.max[0] - bounds.min[0])
            .unwrap_or(0.0);
        // Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: only living
        // entities lie down. The bed orientation/offset are resolved spatially by
        // the WorldStore aggregation (which owns the block data); the per-entity
        // source defaults to the no-bed fallback.
        let is_sleeping = super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) && entity_data_pose(&metadata.data_values)
            == VANILLA_ENTITY_POSE_SLEEPING_ID;
        // Vanilla `Entity.isCrouching` (`Pose.CROUCHING`): AvatarRenderer consumes this for both
        // real players and client mannequins (the latter accepts CROUCHING in `VALID_POSES`).
        let is_crouching = matches!(
            identity.entity_type_id,
            VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_MANNEQUIN_ID
        ) && entity_data_pose(&metadata.data_values)
            == VANILLA_ENTITY_POSE_CROUCHING_ID;
        let is_fall_flying = super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) && entity_is_fall_flying(&metadata.data_values);
        let pose = entity_data_pose(&metadata.data_values);
        let is_visually_swimming = super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) && (pose == VANILLA_ENTITY_POSE_SWIMMING_ID
            || (pose == VANILLA_ENTITY_POSE_FALL_FLYING_ID && !is_fall_flying));
        let humanoid_speed_value = if is_fall_flying {
            let length_squared = (transform.delta_movement.x * transform.delta_movement.x
                + transform.delta_movement.y * transform.delta_movement.y
                + transform.delta_movement.z * transform.delta_movement.z)
                as f32;
            let normalized = length_squared / 0.2;
            (normalized * normalized * normalized).max(1.0)
        } else {
            1.0
        };
        // Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the SCALE
        // attribute): only living entities carry a render scale; everything else
        // renders at its default size.
        let scale = super::super::content_facts::render_scale_for_entity(
            identity.entity_type_id,
            &attributes.attributes,
            &self.content_registry,
        );
        // Vanilla `Sniffer.onSyncedDataUpdated`: the active `Sniffer.State` one-shot
        // `AnimationState` as `(state ordinal, elapsed seconds)`, or `(-1, -1.0)` for an
        // idling/searching sniffer and every non-sniffer (only the sniffer is given a sniffer
        // animation state).
        let (sniffer_animation_id, sniffer_animation_seconds) =
            client_animations.sniffer_animation(partial_ticks);
        let walk_animation_position = client_animations.walk_animation_position(partial_ticks);
        let (player_cape_flap, player_cape_lean, player_cape_lean2) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                client_animations.player_cape_state(partial_ticks, transform.y_rot)
            } else {
                (0.0, 0.0, 0.0)
            };
        let (
            player_fall_flying_time_in_ticks,
            player_should_apply_flying_y_rot,
            player_flying_y_rot,
        ) = if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            let look_angle = calculate_view_vector(
                transform.render_x_rot(partial_ticks),
                transform.render_y_rot(partial_ticks),
            );
            let (should_apply, y_rot) = avatar_flight_y_rotation(
                client_animations.player_previous_delta_movement(),
                transform.delta_movement,
                look_angle,
                partial_ticks,
            );
            (
                client_animations.player_fall_flying_time(partial_ticks),
                should_apply,
                y_rot,
            )
        } else {
            (0.0, false, 0.0)
        };
        // Vanilla `AvatarRenderer.extractRenderState` copies the two inherited LivingEntity INT
        // accessors into `AvatarRenderState`; only the player renderer registers their layers.
        // Preserve the synchronized values here (including hostile/negative values); the renderer
        // owns the per-frame non-positive gate and bounded submission limit.
        let (player_arrow_count, player_stinger_count) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                living_accessors
                    .map(|accessors| {
                        (
                            entity_data_int(
                                &metadata.data_values,
                                accessors.arrow_count_data_id,
                                accessors.arrow_count_default,
                            ),
                            entity_data_int(
                                &metadata.data_values,
                                accessors.stinger_count_data_id,
                                accessors.stinger_count_default,
                            ),
                        )
                    })
                    .unwrap_or((0, 0))
            } else {
                (0, 0)
            };
        let (player_left_shoulder_parrot, player_right_shoulder_parrot) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                (
                    self.metadata_optional_unsigned_int(
                        id,
                        VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
                    )
                    .unwrap_or(None)
                    .map(ParrotModelVariant::from_id),
                    self.metadata_optional_unsigned_int(
                        id,
                        VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
                    )
                    .unwrap_or(None)
                    .map(ParrotModelVariant::from_id),
                )
            } else {
                (None, None)
            };
        let is_baby = vanilla_is_baby(identity.entity_type_id, &metadata.data_values);
        let is_panda = identity.entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID;
        let panda_roll_amount = if is_panda && !is_baby {
            client_animations.panda_roll_amount(partial_ticks)
        } else {
            0.0
        };
        let is_passenger = mount
            .as_ref()
            .is_some_and(|mount| mount.vehicle_id.is_some());
        let boat_paddles = mount.as_ref().map_or([false; 2], |mount| {
            boat_paddle_states(
                identity.entity_type_id,
                &metadata.data_values,
                !mount.passengers.is_empty(),
            )
        });
        let boat_rowing_times =
            client_animations.boat_rowing_times(partial_ticks, boat_paddles[0], boat_paddles[1]);
        let (boat_hurt_time, boat_hurt_dir, boat_damage_time) =
            client_animations.boat_damage_state(partial_ticks);
        // `AbstractMinecart` chooses NewMinecartBehavior from the level's
        // enabled feature set at construction, before any step packet arrives.
        let minecart_new_render =
            is_vanilla_minecart_type(identity.entity_type_id) && minecart_improvements_enabled;
        let mut source_position = position;
        let is_shulker = identity.entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID;
        let render_head_y_rot = if is_shulker {
            transform.y_head_rot
        } else {
            transform.render_y_head_rot(partial_ticks)
        };
        let mut source_y_rot = if is_shulker && client_animations.age_ticks > 0 {
            transform.y_rot
        } else if client_animations.age_ticks == 0
            || super::super::content_facts::entity_is_living(
                identity.entity_type_id,
                &self.content_registry,
            )
        {
            client_animations.render_body_y_rot(*transform, partial_ticks)
        } else {
            transform.render_y_rot(partial_ticks)
        };
        if super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) {
            if let Some(vehicle_body_y_rot) = mount
                .as_ref()
                .and_then(|mount| mount.vehicle_id)
                .and_then(|vehicle_id| self.living_body_render_y_rot(vehicle_id, partial_ticks))
            {
                source_y_rot =
                    solve_living_passenger_body_y_rot(render_head_y_rot, vehicle_body_y_rot);
            }
        }
        let mut source_x_rot = transform.render_x_rot(partial_ticks);
        if minecart_new_render {
            if let Some(render_step) = minecart_lerp
                .as_ref()
                .and_then(|lerp| lerp.render_step(partial_ticks))
            {
                source_position = render_step.position;
                source_y_rot = render_step.y_rot;
                source_x_rot = render_step.x_rot;
            }
        }
        if is_shulker {
            if let Some(offset) =
                client_animations.shulker_render_offset(transform.position, partial_ticks, scale)
            {
                source_position = add_entity_vec3(source_position, offset);
            }
        }
        let boat_bubble_angle = client_animations.boat_bubble_angle(partial_ticks);
        let (wither_x_head_rots, wither_y_head_rots) = wither_invulnerable_accessor
            .map(|_| client_animations.wither_head_rotations())
            .unwrap_or(([0.0; 2], [0.0; 2]));
        // Vanilla `LivingEntityRenderer.extractRenderState`: worn skull animation normally mirrors
        // the entity walk position, but while riding a living entity it reads the vehicle's walk
        // animation position even though the passenger's own limb swing is stopped.
        let worn_head_animation_position = mount
            .as_ref()
            .and_then(|mount| mount.vehicle_id)
            .and_then(|vehicle_id| {
                self.living_entity_walk_animation_position(vehicle_id, partial_ticks)
            })
            .unwrap_or(walk_animation_position);
        // Vanilla `LivingEntity.recreateFromPacket` seeds squid `yBodyRot` from
        // the add-entity head yaw; `Squid.aiStep` then refines it from movement
        // while in water. Keep it as renderer-only body yaw so the synced
        // transform yaw remains canonical.
        let squid_y_body_rot = if vanilla_entity_type_is_squid(identity.entity_type_id) {
            client_animations
                .squid_y_body_rot(partial_ticks)
                .unwrap_or(transform.y_head_rot)
        } else {
            0.0
        };
        let render = EntityRenderState {
            is_riding: is_passenger,
            auto_spin_age_ticks: is_auto_spin_attack
                .then_some(client_animations.age_ticks as f32 + partial_ticks),
            upside_down_height: is_upside_down.then_some(bounding_box_height),
            invisible,
            invisible_to_player: invisible,
            appears_glowing,
            outline_color: 0,
            is_aggressive,
            is_crouching,
            is_fall_flying,
            is_visually_swimming,
            // Vanilla `HumanoidMobRenderer.extractHumanoidRenderState` copies
            // `LivingEntity.elytraAnimationState.getRotX/Y/Z(partialTick)` into
            // `HumanoidRenderState`; `WingsLayer` consumes those rotations when
            // chest equipment renders a WINGS layer.
            elytra_rot_x: client_animations.elytra_rot_x(partial_ticks),
            elytra_rot_y: client_animations.elytra_rot_y(partial_ticks),
            elytra_rot_z: client_animations.elytra_rot_z(partial_ticks),
            is_using_item,
            use_item_off_hand,
            bounding_box_height,
            scale,
            age_scale: vanilla_age_scale(identity.entity_type_id, &metadata.data_values),
            swim_amount: client_animations.living_swim_amount(partial_ticks),
            humanoid_speed_value,
            // `in_water` is likewise resolved by the WorldStore aggregation (it owns the
            // chunk fluid data); the per-entity source defaults to dry.
            in_water: false,
            // Vanilla `Entity.onGround()`: the last synced movement ground flag (default
            // `false` until a movement packet sets it).
            on_ground: transform.on_ground.unwrap_or(false),
            has_red_overlay: client_animations.has_red_overlay(),
            death_time: client_animations.death_time(partial_ticks),
            walk_animation_pos: walk_animation_position,
            walk_animation_speed: client_animations.walk_animation_speed(partial_ticks),
            worn_head_animation_pos: worn_head_animation_position,
            // Vanilla `HumanoidRenderState.attackTime` / `ArmedEntityRenderState.mainArm` /
            // `attackArm`: the lerped melee swing progress, which arm is the main arm, and which
            // arm swings. Attack progress is `0.0` for an entity that is not mid-swing.
            attack_anim: client_animations.attack_anim(partial_ticks),
            main_arm_left,
            attack_arm_off_hand: client_animations.attack_arm_off_hand(),
            // Vanilla render-state `ticksUsingItem`, reconstructed from the shared charge/use
            // counter. Crossbow draw poses and player spear kinetic use sway consume it with their
            // own durations. `0.0` for anything not mid-draw/use.
            crossbow_charge_ticks: client_animations
                .crossbow_charge_ticks_using_item(partial_ticks),
            // Vanilla `LivingEntityRenderState.ticksSinceKineticHitFeedback`
            // (`LivingEntity.getTicksSinceLastKineticHitFeedback(partialTicks)`): entity event `2`
            // starts the local feedback timer that `SpearAnimations.thirdPersonUseItem` consumes.
            ticks_since_kinetic_hit_feedback: client_animations
                .ticks_since_kinetic_hit_feedback(partial_ticks),
            // Vanilla `HumanoidArmorLayer`: each worn armor stack resolves once to its current
            // generation-local equipment asset id; renderer owns the layer-type lookup.
            head_equipment_asset: armor_equipment_asset(ProtocolEquipmentSlot::Head),
            chest_equipment_asset: armor_equipment_asset(ProtocolEquipmentSlot::Chest),
            legs_equipment_asset: armor_equipment_asset(ProtocolEquipmentSlot::Legs),
            feet_equipment_asset: armor_equipment_asset(ProtocolEquipmentSlot::Feet),
            head_armor_dye: armor_dye(ProtocolEquipmentSlot::Head),
            chest_armor_dye: armor_dye(ProtocolEquipmentSlot::Chest),
            legs_armor_dye: armor_dye(ProtocolEquipmentSlot::Legs),
            feet_armor_dye: armor_dye(ProtocolEquipmentSlot::Feet),
            head_armor_foil: armor_foil(ProtocolEquipmentSlot::Head),
            chest_armor_foil: armor_foil(ProtocolEquipmentSlot::Chest),
            legs_armor_foil: armor_foil(ProtocolEquipmentSlot::Legs),
            feet_armor_foil: armor_foil(ProtocolEquipmentSlot::Feet),
            chest_wings_equipment_asset,
            chest_wings_foil,
            flame_dimensions: ({
                // Vanilla `Blaze.isOnFire()` (`Blaze.java:137-143`) reads bit 0 of
                // `Blaze.DATA_FLAGS_ID` and bypasses `fireImmune`.
                let is_on_fire = if identity.entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID {
                    vanilla_entity_metadata_blaze_flags_accessor_for_entity_type(
                        identity.entity_type_id,
                    )
                    .is_some_and(|(flags_id, flags_default)| {
                        (entity_data_byte(&metadata.data_values, flags_id, flags_default) & 1) != 0
                    })
                } else {
                    shared_flags & entity_accessors.on_fire_mask != 0
                        && !super::super::content_facts::entity_is_fire_immune(
                            identity.entity_type_id,
                            &self.content_registry,
                        )
                };
                is_on_fire
                    && bounding_box_width.is_finite()
                    && bounding_box_height.is_finite()
                    && bounding_box_width > 1.0e-4
                    && bounding_box_height > 0.0
            })
            .then_some([bounding_box_width, bounding_box_height]),
            kind: {
                use bbb_render_types::{EntityKindRenderState as K, *};
                match identity.entity_type_id {
                    VANILLA_ENTITY_TYPE_ARROW_ID | VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => {
                        K::Arrow(ArrowRenderState {
                            shake: client_animations.arrow_shake(partial_ticks),
                        })
                    }
                    VANILLA_ENTITY_TYPE_TRIDENT_ID => K::Trident(TridentRenderState::DEFAULT),
                    VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID => {
                        K::FishingHook(FishingHookRenderState::DEFAULT)
                    }
                    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => K::PolarBear(PolarBearRenderState {
                        stand_scale: client_animations.polar_bear_stand_scale(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_CREEPER_ID => K::Creeper(CreeperRenderState {
                        swelling: client_animations.creeper_swelling(partial_ticks),
                        ..CreeperRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_VILLAGER_ID
                    | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
                    | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => K::Villager(VillagerRenderState {
                        unhappy: villager_unhappy,
                        ..VillagerRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_SHULKER_ID => K::Shulker(ShulkerRenderState {
                        peek: client_animations.shulker_peek_amount(partial_ticks),
                        attach_face: shulker_attach_face(
                            identity.entity_type_id,
                            &metadata.data_values,
                        ),
                    }),
                    VANILLA_ENTITY_TYPE_WARDEN_ID => K::Warden(WardenRenderState {
                        tendril_animation: client_animations
                            .warden_tendril_animation(partial_ticks),
                        heart_animation: client_animations.warden_heart_animation(partial_ticks),
                        roar_seconds: client_animations.warden_roar_seconds(partial_ticks),
                        sniff_seconds: client_animations.warden_sniff_seconds(partial_ticks),
                        attack_seconds: client_animations.warden_attack_seconds(partial_ticks),
                        sonic_boom_seconds: client_animations
                            .warden_sonic_boom_seconds(partial_ticks),
                        emerge_seconds: client_animations.warden_emerge_seconds(partial_ticks),
                        dig_seconds: client_animations.warden_dig_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_RABBIT_ID => K::Rabbit(RabbitRenderState {
                        hop_seconds: client_animations.rabbit_hop_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_CREAKING_ID => K::Creaking(CreakingRenderState {
                        can_move: creaking_can_move(identity.entity_type_id, &metadata.data_values),
                        attack_seconds: client_animations.creaking_attack_seconds(partial_ticks),
                        invulnerable_seconds: client_animations
                            .creaking_invulnerable_seconds(partial_ticks),
                        death_seconds: client_animations.creaking_death_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_MANNEQUIN_ID => {
                        K::Player(PlayerRenderState {
                            cape_flap: player_cape_flap,
                            cape_lean: player_cape_lean,
                            cape_lean2: player_cape_lean2,
                            fall_flying_time_in_ticks: player_fall_flying_time_in_ticks,
                            should_apply_flying_y_rot: player_should_apply_flying_y_rot,
                            flying_y_rot: player_flying_y_rot,
                            arrow_count: player_arrow_count,
                            stinger_count: player_stinger_count,
                            left_shoulder_parrot: player_left_shoulder_parrot,
                            right_shoulder_parrot: player_right_shoulder_parrot,
                            show_extra_ears: false,
                            ..PlayerRenderState::DEFAULT
                        })
                    }
                    VANILLA_ENTITY_TYPE_DROWNED_ID => K::Drowned(DrownedRenderState::DEFAULT),
                    VANILLA_ENTITY_TYPE_ENDERMAN_ID => K::Enderman(EndermanRenderState {
                        carrying: enderman_carrying,
                        creepy: enderman_creepy,
                    }),
                    VANILLA_ENTITY_TYPE_BAT_ID => K::Bat(BatRenderState {
                        resting: bat_resting,
                        flying_seconds: client_animations.bat_flying_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_BEE_ID => K::Bee(BeeRenderState {
                        has_stinger: bee_has_stinger,
                        roll_amount: client_animations.bee_roll_amount(partial_ticks),
                        ..BeeRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_FROG_ID => K::Frog(FrogRenderState {
                        croak_seconds: client_animations.frog_croak_seconds(partial_ticks),
                        tongue_seconds: client_animations.frog_tongue_seconds(partial_ticks),
                        jump_seconds: client_animations.frog_jump_seconds(partial_ticks),
                        swim_idle_seconds: client_animations.frog_swim_idle_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_SNIFFER_ID => K::Sniffer(SnifferRenderState {
                        animation_id: sniffer_animation_id,
                        animation_seconds: sniffer_animation_seconds,
                        is_searching: client_animations.sniffer_is_searching(),
                    }),
                    VANILLA_ENTITY_TYPE_ARMADILLO_ID => K::Armadillo(ArmadilloRenderState {
                        is_hiding_in_shell: client_animations.armadillo_is_hiding_in_shell(),
                        roll_up_seconds: client_animations.armadillo_roll_up_seconds(partial_ticks),
                        roll_out_seconds: client_animations
                            .armadillo_roll_out_seconds(partial_ticks),
                        peek_seconds: client_animations.armadillo_peek_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_FOX_ID => K::Fox(FoxRenderState {
                        head_roll_angle: client_animations.fox_head_roll_angle(partial_ticks),
                        crouch_amount: client_animations.fox_crouch_amount(partial_ticks),
                        is_crouching: fox_is_crouching,
                        is_sleeping: fox_is_sleeping,
                        is_sitting: fox_is_sitting,
                        is_pouncing: fox_is_pouncing,
                        is_faceplanted: fox_is_faceplanted,
                    }),
                    VANILLA_ENTITY_TYPE_WITCH_ID => K::Witch(WitchRenderState::DEFAULT),
                    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => K::CopperGolem(CopperGolemRenderState {
                        idle_seconds: client_animations.copper_golem_idle_seconds(partial_ticks),
                        get_item_seconds: client_animations
                            .copper_golem_get_item_seconds(partial_ticks),
                        get_no_item_seconds: client_animations
                            .copper_golem_get_no_item_seconds(partial_ticks),
                        drop_item_seconds: client_animations
                            .copper_golem_drop_item_seconds(partial_ticks),
                        drop_no_item_seconds: client_animations
                            .copper_golem_drop_no_item_seconds(partial_ticks),
                        ..CopperGolemRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_VEX_ID => K::Vex(VexRenderState {
                        charging: vex_charging,
                        ..VexRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_WITHER_ID => K::Wither(WitherRenderState {
                        x_head_rots: wither_x_head_rots,
                        y_head_rots: wither_y_head_rots,
                        invulnerable_ticks: wither_invulnerable_ticks,
                        ..WitherRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_PIG_ID => K::Pig(PigRenderState {
                        saddle_equipment_asset: pig_saddle_equipment_asset,
                        saddle_dye: pig_saddle_dye,
                        saddle_foil: pig_saddle_foil,
                        ..PigRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_WOLF_ID => K::Wolf(WolfRenderState {
                        wet_shade: client_animations.wolf_wet_shade(partial_ticks),
                        shake_anim: client_animations.wolf_shake_anim(partial_ticks),
                        head_roll_angle: client_animations.wolf_head_roll_angle(partial_ticks),
                        body_equipment_asset: wolf_body_equipment_asset,
                        body_armor_dye: wolf_body_armor_dye,
                        body_armor_crackiness: wolf_body_armor_crackiness,
                        body_armor_foil: wolf_body_armor_foil,
                        ..WolfRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_STRIDER_ID => K::Strider(StriderRenderState {
                        ridden: strider_ridden,
                        saddle_equipment_asset: strider_saddle_equipment_asset,
                        saddle_dye: strider_saddle_dye,
                        saddle_foil: strider_saddle_foil,
                        ..StriderRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => K::HappyGhast(HappyGhastRenderState {
                        body_item: happy_ghast_body_item,
                        body_equipment_asset: happy_ghast_body_equipment_asset,
                        harness_dye: happy_ghast_harness_dye,
                        harness_foil: happy_ghast_harness_foil,
                        body_item_is_harness: happy_ghast_body_item_is_harness,
                        is_leash_holder: happy_ghast_is_leash_holder,
                        ridden: happy_ghast_ridden,
                        ..HappyGhastRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => K::SnowGolem(SnowGolemRenderState {
                        pumpkin: snow_golem_pumpkin,
                    }),
                    VANILLA_ENTITY_TYPE_PANDA_ID => K::Panda(PandaRenderState {
                        sit_amount: client_animations.panda_sit_amount(partial_ticks),
                        lie_on_back_amount: client_animations
                            .panda_lie_on_back_amount(partial_ticks),
                        roll_amount: panda_roll_amount,
                        roll_time: client_animations.panda_roll_time(partial_ticks),
                        ..PandaRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_GOAT_ID => K::Goat(GoatRenderState::DEFAULT),
                    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => K::IronGolem(IronGolemRenderState {
                        attack_ticks_remaining: client_animations
                            .iron_golem_attack_ticks_remaining(partial_ticks),
                        offer_flower_tick: client_animations.iron_golem_offer_flower_tick(),
                    }),
                    VANILLA_ENTITY_TYPE_RAVAGER_ID => K::Ravager(RavagerRenderState {
                        stunned_ticks_remaining: client_animations
                            .ravager_stunned_ticks_remaining(partial_ticks),
                        attack_ticks_remaining: client_animations
                            .ravager_attack_ticks_remaining(partial_ticks),
                        roar_animation: client_animations.ravager_roar_animation(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => K::ArmorStand(ArmorStandRenderState {
                        wiggle: client_animations.armor_stand_wiggle(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_PARROT_ID => K::Parrot(ParrotRenderState {
                        flap_angle: client_animations.parrot_flap_angle(partial_ticks),
                        party: false,
                        ..ParrotRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_TURTLE_ID => K::Turtle(TurtleRenderState::DEFAULT),
                    VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => K::EndCrystal(EndCrystalRenderState {
                        beam: self.end_crystal_beam_source(
                            identity.entity_type_id,
                            &metadata.data_values,
                            position,
                        ),
                        ..EndCrystalRenderState::DEFAULT
                    }),
                    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => K::EnderDragon(EnderDragonRenderState {
                        death_time: client_animations.ender_dragon_death_time(partial_ticks),
                        beam: self.ender_dragon_beam_source(
                            identity.entity_type_id,
                            position,
                            partial_ticks,
                        ),
                        animation: client_animations.ender_dragon_animation_source(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_BREEZE_ID => K::Breeze(BreezeRenderState {
                        shoot_seconds: client_animations.breeze_shoot_seconds(partial_ticks),
                        slide_seconds: client_animations.breeze_slide_seconds(partial_ticks),
                        slide_back_seconds: client_animations
                            .breeze_slide_back_seconds(partial_ticks),
                        inhale_seconds: client_animations.breeze_inhale_seconds(partial_ticks),
                        long_jump_seconds: client_animations
                            .breeze_long_jump_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_CHICKEN_ID => K::Chicken(ChickenRenderState {
                        flap: client_animations.chicken_flap(partial_ticks),
                        flap_speed: client_animations.chicken_flap_speed(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => K::EvokerFangs(EvokerFangsRenderState {
                        bite_progress: client_animations.evoker_fangs_bite_progress(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_ALLAY_ID => K::Allay(AllayRenderState {
                        dancing: client_animations.allay_is_dancing(),
                        spinning: client_animations.allay_is_spinning(),
                        spinning_progress: client_animations.allay_spinning_progress(partial_ticks),
                        holding_item_progress: client_animations
                            .allay_holding_item_progress(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_AXOLOTL_ID => K::Axolotl(AxolotlRenderState {
                        playing_dead_factor: client_animations
                            .axolotl_playing_dead_factor(partial_ticks),
                        in_water_factor: client_animations.axolotl_in_water_factor(partial_ticks),
                        on_ground_factor: client_animations.axolotl_on_ground_factor(partial_ticks),
                        moving_factor: client_animations.axolotl_moving_factor(partial_ticks),
                        baby_swim_seconds: client_animations
                            .axolotl_baby_swim_seconds(partial_ticks),
                        baby_walk_seconds: client_animations
                            .axolotl_baby_walk_seconds(partial_ticks),
                        baby_idle_underwater_seconds: client_animations
                            .axolotl_baby_idle_underwater_seconds(partial_ticks),
                        baby_idle_underwater_on_ground_seconds: client_animations
                            .axolotl_baby_idle_underwater_on_ground_seconds(partial_ticks),
                        baby_idle_on_ground_seconds: client_animations
                            .axolotl_baby_idle_on_ground_seconds(partial_ticks),
                        baby_play_dead_seconds: client_animations
                            .axolotl_baby_play_dead_seconds(partial_ticks),
                    }),
                    VANILLA_ENTITY_TYPE_DOLPHIN_ID => K::Dolphin(DolphinRenderState {
                        is_moving: {
                            let delta = transform.delta_movement;
                            delta.x * delta.x + delta.z * delta.z > 1.0e-7
                        },
                    }),
                    t if vanilla_entity_renderer_extracts_sheep_head_eat(t) => {
                        K::Sheep(SheepRenderState {
                            head_eat: SheepHeadEatPose::from_eat_tick(
                                client_animations.sheep_eat_animation_tick(),
                                partial_ticks,
                            ),
                        })
                    }
                    t if is_vanilla_boat_type(t) => K::Boat(BoatRenderState {
                        rowing_time_left: boat_rowing_times[0],
                        rowing_time_right: boat_rowing_times[1],
                        bubble_angle: boat_bubble_angle,
                        underwater: false,
                        hurt: VehicleHurtRenderState {
                            time: boat_hurt_time,
                            dir: boat_hurt_dir,
                            damage_time: boat_damage_time,
                        },
                    }),
                    t if is_vanilla_minecart_type(t) => K::Minecart(MinecartRenderState {
                        new_render: minecart_new_render,
                        pos_on_rail: None,
                        front_pos: None,
                        back_pos: None,
                        rail_offset: None,
                        rail_direction: None,
                        tnt_fuse_remaining_in_ticks: client_animations
                            .minecart_tnt_fuse_remaining_in_ticks(partial_ticks),
                        hurt: VehicleHurtRenderState {
                            time: boat_hurt_time,
                            dir: boat_hurt_dir,
                            damage_time: boat_damage_time,
                        },
                    }),
                    t if crate::entities::renderer_catalog::vanilla_entity_renderer_is_illager(
                        t,
                    ) =>
                    {
                        K::Illager(IllagerRenderState {
                            spellcasting: illager_spellcasting,
                            ..IllagerRenderState::DEFAULT
                        })
                    }
                    t if vanilla_camel_saddle_type(t) => K::Camel(CamelRenderState {
                        dash_seconds: client_animations.camel_dash_seconds(partial_ticks),
                        idle_seconds: client_animations.camel_idle_seconds(partial_ticks),
                        jump_cooldown: client_animations.camel_jump_cooldown(partial_ticks),
                        saddle_equipment_asset: camel_saddle_equipment_asset,
                        saddle_dye: camel_saddle_dye,
                        saddle_foil: camel_saddle_foil,
                        saddle_ridden: camel_saddle_ridden,
                        ..CamelRenderState::DEFAULT
                    }),
                    _ if is_feline => K::Feline(FelineRenderState {
                        is_crouching: feline_is_crouching,
                        is_sprinting: feline_is_sprinting,
                        is_sitting: feline_is_sitting,
                        lie_down_amount: client_animations.feline_lie_down_amount(partial_ticks),
                        lie_down_amount_tail: client_animations
                            .feline_lie_down_amount_tail(partial_ticks),
                        relax_state_one_amount: client_animations
                            .feline_relax_state_one_amount(partial_ticks),
                        is_lying_on_top_of_sleeping_player: false,
                    }),
                    _ if equine_saddle_type => K::Equine(EquineRenderState {
                        saddle_equipment_asset: equine_saddle_equipment_asset,
                        saddle_dye: equine_saddle_dye,
                        saddle_foil: equine_saddle_foil,
                        saddle_ridden: equine_saddle_ridden,
                        animate_tail: client_animations.equine_animate_tail(),
                        eat_animation: client_animations.equine_eat_animation(partial_ticks),
                        stand_animation: client_animations.equine_stand_animation(partial_ticks),
                        feeding_animation: client_animations
                            .equine_feeding_animation(partial_ticks),
                        body_equipment_asset: horse_body_equipment_asset,
                        body_armor_dye: horse_body_armor_dye,
                        body_armor_foil: horse_body_armor_foil,
                        ..EquineRenderState::DEFAULT
                    }),
                    _ if nautilus_saddle_type => K::Nautilus(NautilusRenderState {
                        saddle_equipment_asset: nautilus_saddle_equipment_asset,
                        saddle_dye: nautilus_saddle_dye,
                        saddle_foil: nautilus_saddle_foil,
                        body_equipment_asset: nautilus_body_equipment_asset,
                        body_item_present: nautilus_body_item_present,
                        body_armor_dye: nautilus_body_armor_dye,
                        body_armor_foil: nautilus_body_armor_foil,
                        ..NautilusRenderState::DEFAULT
                    }),
                    t if is_vanilla_llama_type(t) => K::Llama(LlamaRenderState {
                        body_equipment_asset: llama_body_equipment_asset,
                        body_decor_dye: llama_body_decor_dye,
                        body_decor_foil: llama_body_decor_foil,
                        ..LlamaRenderState::DEFAULT
                    }),
                    t if vanilla_entity_metadata_type_is_abstract_piglin(t) => {
                        K::Piglin(PiglinRenderState::DEFAULT)
                    }
                    t if crate::entities::schema::vanilla_entity_metadata_type_is_hoglin_base(
                        t,
                    ) =>
                    {
                        K::Hoglin(HoglinRenderState {
                            attack_animation_tick: client_animations.hoglin_attack_animation_tick(),
                        })
                    }
                    t if vanilla_entity_type_is_squid(t) => K::Squid(SquidRenderState {
                        tentacle_angle: client_animations.squid_tentacle_angle(partial_ticks),
                        x_body_rot: client_animations.squid_x_body_rot(partial_ticks),
                        z_body_rot: client_animations.squid_z_body_rot(partial_ticks),
                    }),
                    t if vanilla_entity_metadata_type_is_guardian(t) => {
                        K::Guardian(GuardianRenderState {
                            tail_animation: client_animations
                                .guardian_tail_animation(partial_ticks),
                            spikes_animation: client_animations
                                .guardian_spikes_animation(partial_ticks),
                            beam: self.guardian_beam_source(
                                identity.entity_type_id,
                                identity.data,
                                &metadata.data_values,
                                &attributes.attributes,
                                *client_animations,
                                position,
                                partial_ticks,
                            ),
                            eye_tracking: None,
                        })
                    }
                    t if vanilla_entity_metadata_type_is_slime(t) => K::Slime(SlimeRenderState {
                        squish: client_animations.slime_squish(partial_ticks),
                    }),
                    _ => K::Other,
                }
            },
            ..EntityRenderState::default()
        };
        Some(EntityRenderSource {
            entity_id: identity.id,
            uuid: identity.uuid,
            entity_type_id: identity.entity_type_id,
            light_probe_eye_height: super::super::content_facts::eye_height_for_model_source_bounds(
                identity.entity_type_id,
                &metadata.data_values,
                bounding_box,
                &self.content_registry,
            ),
            distance_position: position,
            position: source_position,
            y_rot: source_y_rot,
            x_rot: source_x_rot,
            y_head_rot: render_head_y_rot,
            age_ticks: if identity.entity_type_id == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID {
                client_animations.end_crystal_age_ticks()
            } else {
                client_animations.age_ticks
            },
            is_fully_frozen,
            is_sleeping,
            sleeping_bed_yaw: None,
            sleeping_bed_offset: [0.0, 0.0],
            goat_lower_head_tick: client_animations.goat_lower_head_tick(),
            painting_variant_projection: (*metadata.painting_variant_projection).clone(),
            painting_facing: vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .and_then(|accessor| {
                crate::entities::dimensions::painting_facing(
                    accessor,
                    identity.data,
                    &metadata.data_values,
                )
            }),
            creaking_eyes_glowing: client_animations.creaking_eyes_glowing(),
            is_swinging: client_animations.is_swinging(),
            squid_y_body_rot,
            data_values: metadata.data_values.clone(),
            armor_trims: EntityArmorTrimSprites {
                head_armor_trim: armor_trim(ProtocolEquipmentSlot::Head),
                chest_armor_trim: armor_trim(ProtocolEquipmentSlot::Chest),
                legs_armor_trim: armor_trim(ProtocolEquipmentSlot::Legs),
                feet_armor_trim: armor_trim(ProtocolEquipmentSlot::Feet),
                chest_wings_trim,
                wolf_body_armor_trim,
                pig_saddle_trim,
                equine_saddle_trim,
                equine_body_armor_trim: horse_body_armor_trim,
                strider_saddle_trim,
                camel_saddle_trim,
                nautilus_saddle_trim,
                happy_ghast_harness_trim,
                llama_body_decor_trim,
                nautilus_body_armor_trim,
            },
            render,
        })
    }

    pub(crate) fn camera_hurt_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraHurtState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if !super::super::content_facts::entity_metadata_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) {
            return None;
        }
        let client_animations = &entity.animations;
        let (hurt_time, hurt_duration) = client_animations
            .camera_hurt_time(partial_ticks)
            .unwrap_or((-1.0, 10.0));
        let death_time = client_animations.death_time(partial_ticks);
        let hurt_dir = if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            entity.events.last_hurt_yaw.unwrap_or(0.0)
        } else {
            // `LivingEntity.getHurtDir()` is zero; only `Player` overrides it.
            0.0
        };
        Some(EntityCameraHurtState {
            hurt_time,
            hurt_duration,
            hurt_dir,
            death_time,
            is_dead_or_dying: death_time > 0.0,
        })
    }

    pub(super) fn wither_head_targets_by_id(
        &self,
    ) -> FxHashMap<i32, [WitherHeadTargetRotations; 2]> {
        let mut withers = Vec::new();
        for entity in self.records() {
            let (identity, transform, metadata, attributes) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.attributes,
            );
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_WITHER_ID {
                continue;
            }

            withers.push((
                identity.id,
                transform.position,
                transform.y_rot,
                vanilla_render_scale(identity.entity_type_id, &attributes.attributes),
                wither_side_head_target_ids(identity.entity_type_id, &metadata.data_values),
            ));
        }

        let mut eye_positions = FxHashMap::default();
        for target_id in withers
            .iter()
            .flat_map(|(_, _, _, _, target_ids)| target_ids)
            .copied()
            .filter(|target_id| *target_id > 0)
        {
            if eye_positions.contains_key(&target_id) {
                continue;
            }
            let Some(entity) = self.record(target_id) else {
                continue;
            };
            let (identity, transform, metadata) =
                (&entity.identity, &entity.transform, &entity.metadata);
            if let Some(eye_height) = vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &entity.attributes.attributes,
                Some(entity.animations),
                metadata.painting_variant_projection.as_ref(),
            ) {
                eye_positions.insert(target_id, (transform.position, eye_height));
            }
        }

        withers
            .into_iter()
            .map(|(id, position, y_body_rot, scale, target_ids)| {
                let mut targets = [
                    WitherHeadTargetRotations::fallback_to_body(y_body_rot),
                    WitherHeadTargetRotations::fallback_to_body(y_body_rot),
                ];
                for (head_index, target_id) in target_ids.into_iter().enumerate() {
                    if target_id <= 0 {
                        continue;
                    }
                    if let Some((target_position, target_eye_height)) =
                        eye_positions.get(&target_id).copied()
                    {
                        targets[head_index] = wither_side_head_target_rotation(
                            position,
                            y_body_rot,
                            scale,
                            head_index,
                            target_position,
                            target_eye_height,
                        );
                    }
                }
                (id, targets)
            })
            .collect()
    }
}
