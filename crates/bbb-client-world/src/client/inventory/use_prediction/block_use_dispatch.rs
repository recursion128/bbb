use super::*;
use crate::BlockView;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;

impl WorldStore {
    /// Predicts targeted use with the exact native block-hit cursor and
    /// inside flag carried by the serverbound `UseItemOn` packet.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_local_block_use_with_hit(
        &mut self,
        pos: BlockPos,
        face: ProtocolDirection,
        cursor: [f64; 3],
        inside: bool,
        hand: InteractionHand,
        secondary_use_active: bool,
        sequence: i32,
    ) -> LocalTargetUsePrediction {
        let context = LocalBlockUseContext {
            pos,
            face,
            cursor,
            inside,
            sequence,
            secondary_use_active,
        };
        self.predict_local_block_use_with_context(context, hand, None)
    }

    /// Predicts targeted use while sharing the native `ClientLevel.random`
    /// owner required by random-consuming placement states.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_local_block_use_with_hit_and_random(
        &mut self,
        pos: BlockPos,
        face: ProtocolDirection,
        cursor: [f64; 3],
        inside: bool,
        hand: InteractionHand,
        secondary_use_active: bool,
        sequence: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> LocalTargetUsePrediction {
        let context = LocalBlockUseContext {
            pos,
            face,
            cursor,
            inside,
            sequence,
            secondary_use_active,
        };
        self.predict_local_block_use_with_context(context, hand, Some(random))
    }

    fn predict_local_block_use_with_context(
        &mut self,
        context: LocalBlockUseContext,
        hand: InteractionHand,
        mut random: Option<&mut LevelEventSoundRandomState>,
    ) -> LocalTargetUsePrediction {
        let LocalBlockUseContext {
            pos,
            secondary_use_active,
            ..
        } = context;
        if self.local_player_is_spectator() {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume_without_item(),
            ));
        }
        let hands_have_item = self.local_item_in_hand(InteractionHand::MainHand).is_some()
            || self.local_item_in_hand(InteractionHand::OffHand).is_some();
        if secondary_use_active && hands_have_item {
            return self.predict_local_item_use_on(hand, Some(context), random.as_deref_mut());
        }
        let Some(block) = self.block_view(pos) else {
            return LocalTargetUsePrediction::UnknownBlockType(None);
        };
        let Some(block_type) =
            vanilla_block_state_block_type_id(block.block_state_id).and_then(vanilla_block_type)
        else {
            if !self.content_registry().is_vanilla_only()
                && self
                    .content_registry()
                    .block_by_state_id(block.block_state_id)
                    .is_some()
            {
                return self.predict_local_item_use_on(hand, Some(context), random.as_deref_mut());
            }
            return LocalTargetUsePrediction::UnknownBlockType(Some(block.block_state_id));
        };
        if !self.is_feature_enabled("minecraft:vanilla") {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        }
        let ignition_reaches_item_use =
            self.local_ignition_item_reaches_use_on(block_type.use_item_on_override, hand);
        let hanging_sign_reaches_item_use = self.local_hanging_sign_item_reaches_use_on(
            block_type.use_item_on_override,
            hand,
            context,
            &block,
        );
        let specialized_item_reaches_use_on =
            ignition_reaches_item_use || hanging_sign_reaches_item_use;
        let item_on_prediction = match block_type.use_item_on_override {
            Some(_) if specialized_item_reaches_use_on => {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
            }
            Some(owner) => {
                self.predict_local_block_use_item_on(owner, block_type, &block, hand, context)
            }
            None => {
                LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
            }
        };
        match item_on_prediction {
            LocalTargetUsePrediction::Result(
                LocalInteractionResult::Pass | LocalInteractionResult::TryEmptyHandInteraction,
            ) => {}
            prediction => return prediction,
        }
        if hand == InteractionHand::MainHand
            && matches!(
                item_on_prediction,
                LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
            )
        {
            if let Some(owner) = block_type.use_without_item_override {
                let prediction =
                    self.predict_local_block_use_without_item(owner, block_type, &block, context);
                if !matches!(
                    prediction,
                    LocalTargetUsePrediction::Result(
                        LocalInteractionResult::Pass
                            | LocalInteractionResult::TryEmptyHandInteraction
                    )
                ) {
                    return prediction;
                }
            }
        }
        self.predict_local_item_use_on(hand, Some(context), random)
    }

    pub(super) fn local_hanging_sign_item_reaches_use_on(
        &self,
        block_owner: Option<VanillaBlockUseItemOnOverride>,
        hand: InteractionHand,
        context: LocalBlockUseContext,
        block: &BlockView,
    ) -> bool {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return false;
        };
        let Some(item_type) = stack.item_id.and_then(vanilla_item_type) else {
            return false;
        };
        if item_type
            .block_item_behavior
            .is_none_or(|behavior| behavior.kind != VanillaBlockItemKind::HangingSignItem)
            || !self.has_sign_block_entity_at(context.pos)
            || !self.sign_run_commands_are_proven_absent(context.pos)
        {
            return false;
        }
        match block_owner {
            Some(VanillaBlockUseItemOnOverride::CeilingHangingSignBlock) => {
                context.face == ProtocolDirection::Down
            }
            Some(VanillaBlockUseItemOnOverride::WallHangingSignBlock) => {
                let Some(facing) = block.block_properties.get("facing") else {
                    return false;
                };
                let Some(facing_axis) = horizontal_property_axis(facing) else {
                    return false;
                };
                protocol_direction_axis(context.face) != facing_axis
            }
            _ => false,
        }
    }

    pub(super) fn local_ignition_item_reaches_use_on(
        &self,
        block_owner: Option<VanillaBlockUseItemOnOverride>,
        hand: InteractionHand,
    ) -> bool {
        let Some(item_id) = self
            .local_item_in_hand(hand)
            .and_then(|stack| stack.item_id)
        else {
            return false;
        };
        let ignition = vanilla_item_type(item_id)
            .and_then(|item| item.use_on_override)
            .is_some_and(|owner| {
                matches!(
                    owner,
                    VanillaItemUseOnOverride::FlintAndSteelItem
                        | VanillaItemUseOnOverride::FireChargeItem
                )
            });
        if !ignition {
            return false;
        }
        match block_owner {
            Some(VanillaBlockUseItemOnOverride::CandleBlock)
            | Some(VanillaBlockUseItemOnOverride::CandleCakeBlock) => true,
            Some(VanillaBlockUseItemOnOverride::CampfireBlock) => !self
                .recipes()
                .property_sets
                .get("minecraft:campfire_input")
                .is_some_and(|items| items.contains(&item_id)),
            _ => false,
        }
    }

    pub(super) fn predict_local_item_use_on(
        &mut self,
        hand: InteractionHand,
        context: Option<LocalBlockUseContext>,
        random: Option<&mut LevelEventSoundRandomState>,
    ) -> LocalTargetUsePrediction {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        };
        if self.local_item_stack_is_on_cooldown(stack) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if let Some(context) = context {
            if self.local_player_is_adventure()
                && !self.local_item_in_hand_can_place_on_block_in_adventure_mode(hand, context.pos)
            {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
            }
        }
        let Some(item_id) = stack.item_id else {
            return LocalTargetUsePrediction::UnknownItemType(None);
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            if !self.content_registry().is_vanilla_only()
                && self.content_registry().item_by_id(item_id).is_some()
            {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
            }
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        if let Some(owner) = item_type.use_on_override {
            if let Some(context) = context {
                let result = match generated_item_use_on_route(owner) {
                    GeneratedItemUseOnRoute::Tool => {
                        self.predict_local_tool_item_use_on(owner, hand, context)
                    }
                    GeneratedItemUseOnRoute::ClientOwned => self
                        .predict_local_client_owned_item_use_on(
                            owner, item_type, hand, context, random,
                        ),
                    GeneratedItemUseOnRoute::EntitySpawning => {
                        self.predict_local_entity_spawning_item_use_on(owner, hand, context)
                    }
                };
                if let Some(result) = result {
                    return result;
                }
            }
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    pub(super) fn predict_local_client_owned_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        item_type: &ItemTypeCatalogEntry,
        hand: InteractionHand,
        context: LocalBlockUseContext,
        random: Option<&mut LevelEventSoundRandomState>,
    ) -> Option<LocalTargetUsePrediction> {
        let client_success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        let consume = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true),
            ))
        };
        let pass = || LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        match owner {
            VanillaItemUseOnOverride::BlockItem => {
                Some(self.predict_local_block_item_use_on(owner, item_type, hand, context, random))
            }
            // `BrushItem.useOn` always consumes the action and its block hit
            // starts the fixed-duration brushing use animation.
            VanillaItemUseOnOverride::BrushItem => Some(consume()),
            // The permission, component, and block-property work is confined
            // to `ServerPlayer`; the client returns success immediately.
            VanillaItemUseOnOverride::DebugStickItem => Some(client_success()),
            // `LeadItem` only binds mobs on the logical server, while the
            // targeted entry point of `PlaceOnWaterBlockItem` always passes.
            VanillaItemUseOnOverride::LeadItem
            | VanillaItemUseOnOverride::PlaceOnWaterBlockItem => Some(pass()),
            VanillaItemUseOnOverride::CompassItem => {
                let block_type_id =
                    vanilla_block_state_block_type_id(self.block_state_id_at(context.pos)?)?;
                if block_type_id != VANILLA_BLOCK_LODESTONE_ID {
                    return Some(pass());
                }
                let Some(dimension) = self.level_info().map(|level| level.dimension.clone()) else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                let Some(mut lodestone_compass) = self.local_item_in_hand(hand).cloned() else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                lodestone_compass.count = 1;
                let patch = &mut lodestone_compass.component_patch;
                patch.removed_type_ids.retain(|component_id| {
                    *component_id != VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID
                });
                if !patch
                    .added_type_ids
                    .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
                {
                    patch
                        .added_type_ids
                        .push(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID);
                }
                patch.lodestone_target = Some(LodestoneTargetSummary {
                    dimension,
                    pos: ProtocolBlockPos {
                        x: context.pos.x,
                        y: context.pos.y,
                        z: context.pos.z,
                    },
                });
                patch.lodestone_tracked = Some(true);
                let Some(dropped) = self.replace_local_item_with_filled_result(
                    CreativeFilledResult::AlwaysAdd,
                    hand,
                    VANILLA_ITEM_COMPASS_ID,
                    lodestone_compass,
                ) else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                Some(LocalTargetUsePrediction::Result(
                    LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction_with_drop(dropped),
                    ),
                ))
            }
            VanillaItemUseOnOverride::FlintAndSteelItem
            | VanillaItemUseOnOverride::FireChargeItem => {
                Some(self.predict_local_ignition_item_use_on(owner, hand, context))
            }
            VanillaItemUseOnOverride::BoneMealItem => {
                Some(self.predict_local_bone_meal_item_use_on(owner, context))
            }
            VanillaItemUseOnOverride::PotionItem => {
                Some(self.predict_local_potion_item_use_on(owner, hand, context))
            }
            VanillaItemUseOnOverride::SolidBucketItem => {
                Some(self.predict_local_solid_bucket_item_use_on(owner, hand, context))
            }
            VanillaItemUseOnOverride::EnderEyeItem => {
                let block = self.block_view(context.pos)?;
                let block_type_id = vanilla_block_state_block_type_id(block.block_state_id)?;
                if block_type_id != VANILLA_BLOCK_END_PORTAL_FRAME_ID {
                    return Some(pass());
                }
                match block.block_properties.get("eye").map(String::as_str) {
                    Some("false") => Some(client_success()),
                    Some("true") => Some(pass()),
                    _ => Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner)),
                }
            }
            VanillaItemUseOnOverride::MapItem => {
                let block_type_id =
                    vanilla_block_state_block_type_id(self.block_state_id_at(context.pos)?)?;
                let Some(banners) =
                    self.registry_holder_ids_for_tag("minecraft:block", "minecraft:banners")
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                Some(if banners.contains(&block_type_id) {
                    client_success()
                } else {
                    pass()
                })
            }
            _ => None,
        }
    }

    pub(super) fn predict_local_entity_spawning_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> Option<LocalTargetUsePrediction> {
        let client_success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        let pass = || LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        let fail = || LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        match owner {
            VanillaItemUseOnOverride::ArmorStandItem => {
                Some(self.predict_local_armor_stand_item_use_on(owner, hand, context))
            }
            VanillaItemUseOnOverride::HangingEntityItem => {
                Some(self.predict_local_hanging_entity_item_use_on(owner, hand, context))
            }
            // `SpawnEggItem.useOn` returns before all server-only type,
            // spawner, difficulty, spawn, and stack-mutation work.
            VanillaItemUseOnOverride::SpawnEggItem => Some(client_success()),
            VanillaItemUseOnOverride::FireworkRocketItem => {
                let fall_flying = self
                    .local_player_id()
                    .and_then(|id| self.entities.is_fall_flying(id))
                    .unwrap_or(false);
                Some(if fall_flying {
                    pass()
                } else {
                    client_success()
                })
            }
            VanillaItemUseOnOverride::EndCrystalItem => {
                let block_type_id =
                    vanilla_block_state_block_type_id(self.block_state_id_at(context.pos)?)?;
                if !matches!(
                    block_type_id,
                    VANILLA_BLOCK_OBSIDIAN_ID | VANILLA_BLOCK_BEDROCK_ID
                ) {
                    return Some(fail());
                }
                let spawn_pos = above(context.pos);
                if !self.local_block_is_air(spawn_pos)? {
                    return Some(fail());
                }
                let query_min = [
                    f64::from(spawn_pos.x),
                    f64::from(spawn_pos.y),
                    f64::from(spawn_pos.z),
                ];
                let query_max = [query_min[0] + 1.0, query_min[1] + 2.0, query_min[2] + 1.0];
                match self
                    .entities
                    .any_entity_bounds_intersecting(query_min, query_max, |_| true)
                {
                    Some(true) => Some(fail()),
                    Some(false) if self.shrink_local_item_in_hand(hand, 1) => {
                        Some(client_success())
                    }
                    Some(false) => Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner)),
                    None => Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner)),
                }
            }
            VanillaItemUseOnOverride::MinecartItem => {
                let block = self.block_view(context.pos)?;
                let block_type_id = vanilla_block_state_block_type_id(block.block_state_id)?;
                let Some(rails) =
                    self.registry_holder_ids_for_tag("minecraft:block", "minecraft:rails")
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                if !rails.contains(&block_type_id) {
                    return Some(fail());
                }
                if self.is_feature_enabled("minecraft:minecart_improvements") {
                    let bounds = vanilla_pick_bounds_for_type(VANILLA_ENTITY_TYPE_MINECART_ID)?;
                    let slope_offset = if block
                        .block_properties
                        .get("shape")
                        .is_some_and(|shape| shape.starts_with("ascending_"))
                    {
                        0.5
                    } else {
                        0.0
                    };
                    let spawn_position = [
                        f64::from(context.pos.x) + 0.5,
                        f64::from(context.pos.y) + 0.0625 + slope_offset,
                        f64::from(context.pos.z) + 0.5,
                    ];
                    let query_min = std::array::from_fn(|axis| {
                        spawn_position[axis] + f64::from(bounds.min[axis])
                    });
                    let query_max = std::array::from_fn(|axis| {
                        spawn_position[axis] + f64::from(bounds.max[axis])
                    });
                    match self.entities.any_entity_bounds_intersecting(
                        query_min,
                        query_max,
                        vanilla_entity_type_is_minecart,
                    ) {
                        Some(true) => return Some(fail()),
                        Some(false) => {}
                        None => {
                            return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                        }
                    }
                }
                Some(if self.shrink_local_item_in_hand(hand, 1) {
                    client_success()
                } else {
                    LocalTargetUsePrediction::UnsupportedItemUseOn(owner)
                })
            }
            _ => None,
        }
    }

    pub(super) fn predict_local_transformed_block_state(
        &mut self,
        context: LocalBlockUseContext,
        transform: &VanillaBlockTransform,
        source_state_id: i32,
    ) -> bool {
        let Some(mut predicted_block_state_id) =
            vanilla_block_default_state_id(transform.target_block_type_id)
        else {
            return false;
        };
        for (property, value) in vanilla_block_state_properties(source_state_id) {
            let Some(updated) =
                vanilla_block_state_with_property(predicted_block_state_id, property, value)
            else {
                return false;
            };
            predicted_block_state_id = updated;
        }
        self.predict_local_player_block_state(
            context.pos,
            predicted_block_state_id,
            context.sequence,
        )
    }

    pub(super) fn local_block_is_air(&self, pos: BlockPos) -> Option<bool> {
        Some(
            vanilla_block_state_block_type_id(self.block_state_id_at(pos)?)?
                == VANILLA_BLOCK_AIR_ID,
        )
    }

    pub(super) fn local_block_is_replaceable_by_different_item(
        &self,
        pos: BlockPos,
    ) -> Option<bool> {
        let block = self.block_view(pos)?;
        let block_type =
            vanilla_block_type(vanilla_block_state_block_type_id(block.block_state_id)?)?;
        Some(match block_type.can_survive_override {
            // A held item different from the snow-layer block item reaches
            // Vanilla's one-layer-only replacement branch.
            Some(VanillaCanSurviveOverride::SnowLayerBlock) => block
                .block_properties
                .get("layers")
                .is_some_and(|layers| layers == "1"),
            // `MultifaceBlock` accepts every different held item.
            Some(VanillaCanSurviveOverride::MultifaceBlock) => true,
            // A clicked vine is replaceable until all five supported faces
            // are occupied, independently of its static replaceable property.
            Some(VanillaCanSurviveOverride::VineBlock) => {
                ["up", "north", "south", "west", "east"]
                    .into_iter()
                    .filter(|property| {
                        block
                            .block_properties
                            .get(*property)
                            .is_some_and(|value| value == "true")
                    })
                    .count()
                    < 5
            }
            _ => block_type.replaceable,
        })
    }
}
