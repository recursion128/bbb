use super::*;

impl WorldStore {
    pub(crate) fn queue_local_interaction_particle_action(
        &mut self,
        action: LocalInteractionParticleAction,
    ) {
        if self.pending_local_interaction_particle_actions.len()
            >= MAX_PENDING_LOCAL_INTERACTION_PARTICLE_ACTIONS
        {
            self.pending_local_interaction_particle_actions.remove(0);
        }
        self.pending_local_interaction_particle_actions.push(action);
    }

    pub fn take_pending_local_interaction_particle_actions(
        &mut self,
    ) -> Vec<LocalInteractionParticleAction> {
        std::mem::take(&mut self.pending_local_interaction_particle_actions)
    }

    /// Predicts vanilla `ItemStack.use` for the default implementation and
    /// implemented source-derived specialized owners. Remaining specialized
    /// classes stay explicit instead of being guessed from component shape.
    pub fn predict_local_item_use(&mut self, hand: InteractionHand) -> LocalItemUsePrediction {
        self.predict_local_item_use_with_pov(hand, None)
    }

    /// Predicts `ItemStack.use` with the exact native player-eye POV result.
    /// `None` means that the caller did not supply a required raycast, while
    /// `Some(Miss)` is Vanilla's explicit miss result.
    pub fn predict_local_item_use_with_pov(
        &mut self,
        hand: InteractionHand,
        pov: Option<LocalItemUsePovResult>,
    ) -> LocalItemUsePrediction {
        self.predict_local_item_use_with_pov_context(hand, pov, None, false)
    }

    pub(super) fn predict_local_item_use_with_pov_context(
        &mut self,
        hand: InteractionHand,
        pov: Option<LocalItemUsePovResult>,
        sequence: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalItemUsePrediction {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let Some(item_id) = stack.item_id else {
            return LocalItemUsePrediction::UnknownItemType(None);
        };
        let item_type = vanilla_item_type(item_id);
        if item_type.is_none()
            && (self.content_registry().is_vanilla_only()
                || self.content_registry().item_by_id(item_id).is_none())
        {
            return LocalItemUsePrediction::UnknownItemType(Some(item_id));
        }
        if self.local_item_stack_is_on_cooldown(stack) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if let Some((item_type, use_override)) =
            item_type.and_then(|item_type| item_type.use_override.map(|ov| (item_type, ov)))
        {
            return match use_override {
                VanillaItemUseOverride::InstrumentItem => {
                    self.predict_local_instrument_item_use(hand)
                }
                VanillaItemUseOverride::BowItem => self.predict_local_bow_item_use(),
                VanillaItemUseOverride::WritableBookItem
                | VanillaItemUseOverride::WrittenBookItem => self.predict_local_book_item_use(),
                VanillaItemUseOverride::BundleItem => LocalItemUsePrediction::Result(
                    LocalInteractionResult::Success(LocalInteractionSuccess::bundle_use()),
                ),
                VanillaItemUseOverride::CrossbowItem => self.predict_local_crossbow_item_use(stack),
                VanillaItemUseOverride::EmptyMapItem => self.predict_local_empty_map_item_use(),
                VanillaItemUseOverride::FireworkRocketItem => {
                    self.predict_local_firework_rocket_item_use()
                }
                VanillaItemUseOverride::FishingRodItem => self.predict_local_fishing_rod_item_use(),
                VanillaItemUseOverride::FoodOnAStickItem => {
                    self.predict_local_food_on_a_stick_item_use()
                }
                VanillaItemUseOverride::KnowledgeBookItem => {
                    self.predict_local_knowledge_book_item_use(hand)
                }
                VanillaItemUseOverride::BoatItem => {
                    self.predict_local_boat_item_use(item_type, pov)
                }
                VanillaItemUseOverride::BottleItem => self.predict_local_bottle_item_use(hand, pov),
                VanillaItemUseOverride::BucketItem => self.predict_local_bucket_item_use(
                    hand,
                    item_type,
                    pov,
                    sequence,
                    secondary_use_active,
                ),
                VanillaItemUseOverride::EnderEyeItem => self.predict_local_ender_eye_item_use(pov),
                VanillaItemUseOverride::PlaceOnWaterBlockItem => {
                    self.predict_local_place_on_water_block_item_use(hand, item_type, pov, sequence)
                }
                VanillaItemUseOverride::SpawnEggItem => self.predict_local_spawn_egg_item_use(pov),
                VanillaItemUseOverride::TridentItem => self.predict_local_trident_item_use(hand),
                VanillaItemUseOverride::SpyglassItem => LocalItemUsePrediction::Result(
                    LocalInteractionResult::Success(LocalInteractionSuccess::spyglass_use()),
                ),
                VanillaItemUseOverride::EggItem
                | VanillaItemUseOverride::EnderpearlItem
                | VanillaItemUseOverride::ExperienceBottleItem
                | VanillaItemUseOverride::LingeringPotionItem
                | VanillaItemUseOverride::SnowballItem
                | VanillaItemUseOverride::SplashPotionItem
                | VanillaItemUseOverride::WindChargeItem => {
                    self.predict_local_thrown_projectile_item_use(hand)
                }
            };
        }

        if self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
        {
            if !self.local_item_in_hand_can_start_consuming(hand) {
                return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
            }
            let Some(consumable) = self.effective_item_consumable(stack) else {
                return LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                );
            };
            return LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::start_consuming(hand, consumable),
            ));
        }

        if let Some(prediction) = self.predict_local_swappable_equippable_use(hand) {
            return prediction;
        }

        let stack = self
            .local_item_in_hand(hand)
            .expect("equippable pass leaves the hand unchanged");
        if self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        ) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true),
            ));
        }
        if self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        ) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess {
                    sound_action: Some(LocalInteractionSoundAction::KineticWeaponUse { hand }),
                    ..LocalInteractionSuccess::consume(true)
                },
            ));
        }

        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    }

    /// Queues the client-local `PiercingWeapon.makeSound` (`PiercingWeapon.java:47-52`)
    /// sound when the local player triggers a spear jab with the selected main-hand item.
    pub fn queue_local_piercing_weapon_use_sound(&mut self) {
        if self.local_selected_main_hand_has_piercing_weapon() {
            self.queue_local_interaction_sound_action(
                LocalInteractionSoundAction::PiercingWeaponUse {
                    hand: InteractionHand::MainHand,
                },
            );
        }
    }

    pub(super) fn effective_item_consumable<'a>(
        &'a self,
        stack: &'a ProtocolItemStackSummary,
    ) -> Option<&'a ConsumableSummary> {
        self.items
            .component_source(
                stack.item_id,
                &stack.component_patch,
                VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            )
            .consumable
            .as_ref()
    }

    pub(super) fn local_item_stack_is_on_cooldown(&self, stack: &ProtocolItemStackSummary) -> bool {
        !self.cooldowns().is_empty()
            && self
                .local_item_stack_cooldown_group(stack)
                .is_some_and(|group| self.cooldown(&group).is_some())
    }

    pub(super) fn local_item_stack_cooldown_group(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Option<String> {
        if self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID)
        {
            let effective = self.effective_item_stack(stack);
            if let Some(group) = effective.component_patch.use_cooldown_group.as_deref() {
                return Some(group.to_owned());
            }
        }
        stack
            .item_id
            .and_then(vanilla_item_type)
            .map(|item| item.resource_id.to_owned())
    }
}
