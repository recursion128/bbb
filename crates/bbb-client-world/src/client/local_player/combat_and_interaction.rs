use super::*;

impl WorldStore {
    /// Vanilla `LocalPlayer.handlePortalTransitionEffect` driven by
    /// `getActivePortalLocalTransition() == CONFUSION` (the nether portal is
    /// the only confusion-transition portal): inside the portal the intensity
    /// ramps `+0.0125` per tick, requests the vanilla screen close and fires
    /// the `PORTAL_TRIGGER` ambience on the `0 →` rising edge; outside it
    /// decays `-0.05`, clamped to `0..=1`.
    pub(crate) fn advance_local_player_portal_transition_effect(&mut self) {
        // Vanilla `LocalPlayer.aiStep` (java:768-771): pause portal intensity
        // and cooldown updates while `LevelLoadingScreen` is displayed.
        if self.level_load().screen_is_up() {
            return;
        }
        let inside = self
            .local_player
            .pose
            .is_some_and(|pose| local_player_inside_nether_portal(self, pose));
        let on_cooldown = self.local_player.portal_cooldown > 0;
        self.local_player.previous_portal_effect_intensity =
            self.local_player.portal_effect_intensity;
        let mut step = 0.0_f32;
        if inside && !on_cooldown {
            if self.local_player.portal_effect_intensity == 0.0 {
                self.pending_portal_transition_trigger_sound = true;
            }
            step = 0.0125;
            if self.open_container_id().is_some() {
                self.pending_portal_transition_container_close = true;
            }
        } else if self.local_player.portal_effect_intensity > 0.0 {
            step = -0.05;
        }
        self.local_player.portal_effect_intensity =
            (self.local_player.portal_effect_intensity + step).clamp(0.0, 1.0);
        // Vanilla `Entity.setAsInsidePortal` / `processPortalCooldown`: staying
        // inside a portal while on cooldown keeps resetting the 300-tick player
        // cooldown; leaving lets it tick down.
        if on_cooldown {
            if inside {
                self.local_player.portal_cooldown = 300;
            } else {
                self.local_player.portal_cooldown -= 1;
            }
        }
    }

    /// Vanilla `Gui.extractCameraOverlays`'s
    /// `Mth.lerp(partialTicks, oPortalEffectIntensity, portalEffectIntensity)`.
    pub fn local_player_portal_effect_intensity(&self, partial_tick: f32) -> f32 {
        let partial_tick = if partial_tick.is_finite() {
            partial_tick.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let previous = self.local_player.previous_portal_effect_intensity;
        previous + (self.local_player.portal_effect_intensity - previous) * partial_tick
    }

    /// Drains the one-shot portal-transition side effects for the native
    /// consumer: `(trigger_sound, close_container)`.
    pub fn take_pending_portal_transition_effects(&mut self) -> (bool, bool) {
        let effects = (
            self.pending_portal_transition_trigger_sound,
            self.pending_portal_transition_container_close,
        );
        self.pending_portal_transition_trigger_sound = false;
        self.pending_portal_transition_container_close = false;
        effects
    }

    pub fn local_player_is_sprinting(&self) -> bool {
        self.local_player.sprinting
    }

    /// Applies the exact `LocalPlayer.aiStep` sprint start/stop decision to the
    /// canonical local-player latch. `start_requested` is true for the held
    /// sprint key or one accepted double-tap-forward edge.
    pub fn reconcile_local_player_sprinting(
        &mut self,
        input: LocalPlayerInputState,
        start_requested: bool,
    ) -> bool {
        let previous = self.local_player.sprinting;
        let sprinting = if previous {
            !local_player_should_stop_sprinting(self, input)
        } else {
            start_requested && local_player_can_start_sprinting(self, input)
        };
        self.local_player.sprinting = sprinting;
        sprinting != previous
    }

    pub fn local_player_can_start_sprinting(&self, input: LocalPlayerInputState) -> bool {
        !self.local_player.sprinting && local_player_can_start_sprinting(self, input)
    }

    pub fn local_player_eye_in_water(&self) -> bool {
        local_player_eye_in_water(self)
    }

    pub fn entity_eye_in_water(&self, id: i32) -> bool {
        crate::client::local_player_fluid::entity_eye_in_water(self, id)
    }

    /// The client-side composition of `LocalPlayer`'s jump-edge guard,
    /// `Player.tryToStartFallFlying`, `Player.canGlide`, and
    /// `LivingEntity.canGlide`. The server remains authoritative for accepting
    /// the command and setting the fall-flying shared flag.
    pub fn local_player_can_start_fall_flying(&self) -> bool {
        let Some(local_player_id) = self.local_player_id else {
            return false;
        };
        let Some(pose) = self.local_player.pose else {
            return false;
        };
        !pose.on_ground
            && self.local_player_root_vehicle_id().is_none()
            && !self
                .local_player
                .abilities
                .is_some_and(|abilities| abilities.flying)
            && self
                .entity_effect(local_player_id, VANILLA_MOB_EFFECT_LEVITATION_ID)
                .is_none()
            && !self
                .entities
                .is_fall_flying(local_player_id)
                .unwrap_or(false)
            && !local_player_fluid_contact(self, pose).in_water()
            && !local_player_is_on_climbable(self, pose)
            && self.local_player_has_usable_glider()
    }

    /// Vanilla `LivingEntity.getArmorValue()` =
    /// `Mth.floor(getAttributeValue(Attributes.ARMOR))` (LivingEntity.java:1845-1846),
    /// the value `Gui.extractArmor` reads to draw the armor bar (Gui.java:799). The
    /// synced ARMOR attribute's modifiers fold through the shared
    /// `AttributeInstance.calculateValue` implementation (`entities::store`'s
    /// `vanilla_attribute_value`: base add, then multiply_base, then multiply_total).
    /// Returns `0` when the player has no synced ARMOR attribute, matching vanilla's
    /// `RangedAttribute` default of `0.0` (and `Mth.floor` toward negative infinity).
    pub fn local_player_armor_value(&self) -> i32 {
        self.local_player_id
            .map_or(0, |id| self.player_armor_value(id))
    }

    pub fn player_armor_value(&self, id: i32) -> i32 {
        self.entities
            .attribute_value(id, VANILLA_ATTRIBUTE_ARMOR_ID)
            .map(|value| value.floor() as i32)
            .unwrap_or(0)
    }

    /// Returns the server-synchronized base value for one local-player
    /// attribute. Item tooltip projection uses the base rather than the
    /// modifier-folded value, matching `Player.getAttributeBaseValue` in
    /// `ItemAttributeModifiers.Display.Default.apply`.
    pub fn local_player_attribute_base_value(&self, attribute_id: i32) -> Option<f64> {
        self.local_player_id
            .and_then(|id| self.entities.attribute_base_value(id, attribute_id))
    }

    /// Returns the modifier-folded value of one local-player attribute.
    pub fn local_player_attribute_value(&self, attribute_id: i32) -> Option<f64> {
        self.local_player_id
            .and_then(|id| self.entities.attribute_value(id, attribute_id))
    }

    pub fn local_player_attack_strength_ticks(&self) -> u32 {
        self.local_player.interaction.attack.attack_strength_ticks
    }

    pub fn local_player_item_swap_ticks(&self) -> u32 {
        self.local_player.interaction.attack.item_swap_ticks
    }

    /// Vanilla `Player.getItemSwapScale`: the first-person main-hand
    /// replacement target uses the item-swap ticker over the current attack
    /// delay, independently from the attack-strength ticker.
    pub fn local_player_item_swap_scale(&self, partial_tick: f32) -> f32 {
        self.local_player_item_swap_scale_for_ticks(
            self.local_player_item_swap_ticks(),
            partial_tick,
        )
    }

    /// Historical item-swap scale within the current client-tick catch-up
    /// batch. The entity animation owner has already advanced the canonical
    /// ticker for every tick when `ItemInHandRenderer.tick` is replayed.
    pub fn local_player_item_swap_scale_ticks_ago(&self, ticks_ago: u32, partial_tick: f32) -> f32 {
        self.local_player_item_swap_scale_for_ticks(
            self.local_player_item_swap_ticks()
                .saturating_sub(ticks_ago),
            partial_tick,
        )
    }

    fn local_player_item_swap_scale_for_ticks(
        &self,
        item_swap_ticks: u32,
        partial_tick: f32,
    ) -> f32 {
        let Some(attack_speed) =
            self.local_player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_SPEED_ID)
        else {
            return 0.0;
        };
        if !attack_speed.is_finite() || attack_speed <= 0.0 {
            return 0.0;
        }
        let delay = (1.0 / attack_speed * 20.0) as f32;
        ((item_swap_ticks as f32 + partial_tick) / delay).clamp(0.0, 1.0)
    }

    pub fn reset_local_player_attack_strength(&mut self) {
        let attack = &mut self.local_player.interaction.attack;
        attack.attack_strength_ticks = 0;
        attack.item_swap_ticks = 0;
    }

    pub fn reset_only_local_player_attack_strength(&mut self) {
        self.local_player.interaction.attack.attack_strength_ticks = 0;
    }

    pub fn local_player_cannot_attack_with_selected_item(&self) -> bool {
        let required_strength = self.local_selected_main_hand_minimum_attack_charge();
        if required_strength <= 0.0 {
            return false;
        }
        let Some(attack_speed) =
            self.local_player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_SPEED_ID)
        else {
            return true;
        };
        if !attack_speed.is_finite() || attack_speed <= 0.0 {
            return true;
        }
        let delay = (20.0 / attack_speed) as f32;
        let optimistic_strength = self.local_player_attack_strength_ticks() as f32 / delay;
        optimistic_strength < required_strength
    }

    pub(crate) fn advance_local_player_attack_strength_ticks(&mut self, ticks: u32) {
        if ticks == 0
            || !self
                .local_player_id
                .is_some_and(|id| self.entities.contains(id))
        {
            return;
        }

        let current_main_hand = self.local_item_in_hand(InteractionHand::MainHand).cloned();
        let attack = &mut self.local_player.interaction.attack;
        let stack_changed = attack.last_main_hand_item != current_main_hand;
        let item_changed = stack_changed
            && attack
                .last_main_hand_item
                .as_ref()
                .and_then(|item| item.item_id)
                != current_main_hand.as_ref().and_then(|item| item.item_id);

        if item_changed {
            let ticks_after_reset = ticks.saturating_sub(1);
            attack.attack_strength_ticks = ticks_after_reset;
            attack.item_swap_ticks = ticks_after_reset;
        } else {
            attack.attack_strength_ticks = attack.attack_strength_ticks.saturating_add(ticks);
            attack.item_swap_ticks = attack.item_swap_ticks.saturating_add(ticks);
        }
        if stack_changed {
            attack.last_main_hand_item = current_main_hand;
        }
    }

    /// Vanilla `player.getAirSupply()` read by `Gui.extractAirBubbles`
    /// (Gui.java:889): the synced `Entity.DATA_AIR_SUPPLY_ID` int (data id 1,
    /// see `entities::VANILLA_ENTITY_AIR_SUPPLY_DATA_ID` for the derivation).
    /// Falls back to the full 300-tick supply while there is no local player
    /// entity or no synced value — the state in which vanilla hides the row.
    pub fn local_player_air_supply(&self) -> i32 {
        self.local_player_id
            .map_or(crate::entities::VANILLA_MAX_AIR_SUPPLY_TICKS, |id| {
                self.player_air_supply(id)
            })
    }

    pub fn player_air_supply(&self, id: i32) -> i32 {
        self.entities
            .air_supply(id)
            .unwrap_or(crate::entities::VANILLA_MAX_AIR_SUPPLY_TICKS)
    }

    /// Vanilla `player.getMaxAirSupply()` read by `Gui.extractAirBubbles`
    /// (Gui.java:888): the fixed 300-tick `Entity.getMaxAirSupply()`
    /// (Entity.java:2725-2727) — `Player` does not override it.
    pub fn local_player_max_air_supply(&self) -> i32 {
        crate::entities::VANILLA_MAX_AIR_SUPPLY_TICKS
    }

    /// Vanilla `player.getAbsorptionAmount()` (`Gui.extractPlayerHealth`,
    /// Gui.java:769): the synced `Player.DATA_PLAYER_ABSORPTION_ID` float
    /// (data id 17, see `entities::store`'s derivation comment), `0.0` while
    /// unsynced. Stored for the heart-variant rows; the current HUD does not
    /// draw absorption hearts yet.
    pub fn local_player_absorption(&self) -> f32 {
        self.local_player_id
            .map_or(0.0, |id| self.player_absorption_amount(id))
    }

    pub fn player_absorption_amount(&self, id: i32) -> f32 {
        self.entities.player_absorption(id).unwrap_or(0.0)
    }

    /// Vanilla `Player.getScore()` read by `DeathScreen`: the synced
    /// `Player.DATA_SCORE_ID` int, defaulting to the define-time zero while
    /// the local entity has not been spawned or synchronized.
    pub fn local_player_score(&self) -> i32 {
        self.local_player_id
            .and_then(|id| self.entities.player_score(id))
            .unwrap_or(0)
    }

    pub fn advance_local_player_look_input(
        &mut self,
        input: LocalPlayerInputState,
    ) -> Option<LocalPlayerPoseState> {
        let mut pose = apply_local_player_input_look(self.local_player.pose?, input);
        // Vanilla `AbstractBoat.onPassengerTurned` (`AbstractBoat.java:671-673`):
        // clamp the passenger's yaw to within ±105° of the boat they are riding.
        if let Some(boat_y_rot) = self
            .local_player_vehicle_id
            .filter(|&vehicle_id| {
                self.entities
                    .entity_type_id(vehicle_id)
                    .is_some_and(crate::entities::is_vanilla_boat_type)
            })
            .and_then(|vehicle_id| self.entities.transform(vehicle_id))
            .map(|transform| transform.y_rot)
        {
            crate::entities::clamp_local_player_boat_yaw(
                &mut pose,
                self.local_player_old_pose.as_mut(),
                boat_y_rot,
            );
        }
        self.local_player.pose = Some(pose);
        Some(pose)
    }

    pub fn local_player_pose(&self) -> Option<LocalPlayerPoseState> {
        self.local_player.pose
    }

    /// Projects the two pieces of the local player's entity that the client
    /// owns elsewhere: the transform from the pose (locally simulated motion)
    /// and the equipment from the inventory. Everything the server synchronizes
    /// (data, attributes, effects, animations) lives only on the entity record.
    /// Called once per frame after local movement, before the scene reads it.
    pub fn sync_local_player_entity(&mut self) {
        self.sync_local_player_passenger_position();
        let Some(id) = self.local_player_id else {
            return;
        };
        let inventory = &self.inventory;
        let selected_hotbar_slot = self.local_player.selected_hotbar_slot;
        let _ = self.entities.with_equipment_mut(id, |current| {
            let equipment = inventory
                .local_player_equipment(selected_hotbar_slot)
                .filter(|(_, item)| item.item_id.is_some() && item.count > 0);
            if !equipment
                .clone()
                .map(|(entry, item)| (entry.slot, item))
                .eq(current.equipment.iter().map(|item| (item.slot, &item.item)))
            {
                current.equipment = equipment
                    .map(|(entry, item)| ProtocolEquipmentSlotUpdate {
                        slot: entry.slot,
                        item: item.clone(),
                    })
                    .collect::<Vec<_>>()
                    .into();
            }
        });
        let Some(pose) = self.local_player.pose else {
            return;
        };
        let old = self.local_player_old_pose.unwrap_or(pose);
        let position = pose.position;
        let _ = self.entities.with_transform_mut(id, |transform| {
            transform.position = position;
            transform.position_base = position;
            transform.old_position = old.position;
            transform.delta_movement = pose.delta_movement;
            transform.y_rot = pose.y_rot;
            transform.old_y_rot = old.y_rot;
            transform.x_rot = pose.x_rot;
            transform.old_x_rot = old.x_rot;
            // Player.serverAiStep: yHeadRot = getYRot().
            transform.y_head_rot = pose.y_rot;
            transform.old_y_head_rot = old.y_rot;
            transform.on_ground = Some(pose.on_ground);
            transform.interpolation = None;
        });
        self.project_local_player_owned_data();
    }

    pub(crate) fn project_local_player_owned_data(&mut self) {
        let (Some(id), Some(pose)) = (self.local_player_id, self.local_player.pose) else {
            return;
        };
        let interaction = &self.local_player.interaction;
        let owned = crate::entities::LocalPlayerOwnedData {
            shift_key_down: self.local_player.shift_key_down,
            swimming: self.local_player.swimming,
            crouching_pose: pose.sneaking,
            swim_or_crawl_pose: pose.swimming,
            sprinting: self.local_player.sprinting,
            using_hand: interaction.using_item.then(|| {
                interaction
                    .using_item_hand
                    .unwrap_or(InteractionHand::MainHand)
            }),
        };
        let _ = self.entities.project_local_player_owned_data(id, owned);
    }

    pub fn local_player_pose_at_partial_tick(
        &self,
        partial_tick: f32,
    ) -> Option<LocalPlayerPoseState> {
        let current = self.local_player.pose?;
        let old = self.local_player_old_pose.unwrap_or(current);
        let partial_tick = if partial_tick.is_finite() {
            partial_tick.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let partial_tick_f64 = f64::from(partial_tick);
        Some(LocalPlayerPoseState {
            position: self
                .local_player_minecart_render_position(partial_tick)
                .unwrap_or(ProtocolVec3d {
                    x: old.position.x + (current.position.x - old.position.x) * partial_tick_f64,
                    y: old.position.y + (current.position.y - old.position.y) * partial_tick_f64,
                    z: old.position.z + (current.position.z - old.position.z) * partial_tick_f64,
                }),
            y_rot: if partial_tick == 1.0 {
                current.y_rot
            } else {
                old.y_rot + wrap_degrees_f32(current.y_rot - old.y_rot) * partial_tick
            },
            x_rot: old.x_rot + (current.x_rot - old.x_rot) * partial_tick,
            ..current
        })
    }

    pub fn local_player(&self) -> &LocalPlayerState {
        &self.local_player
    }

    pub fn client_local_player(&self) -> &LocalPlayerState {
        self.local_player()
    }

    pub fn set_local_destroying_block(&mut self, pos: BlockPos) {
        let item_signature = self.local_selected_hotbar_item_signature();
        self.local_player.interaction.destroying_block = Some(pos);
        self.local_player.interaction.destroying_block_face = None;
        self.local_player.interaction.destroying_item_signature = Some(item_signature);
        self.local_player.interaction.destroying_block_progress = 0;
        self.local_player.interaction.destroying_block_stage = None;
        self.local_player.interaction.destroying_block_ticks = 0;
        self.local_player.interaction.destroy_delay_ticks = 0;
    }

    pub fn set_local_destroying_block_hit(&mut self, pos: BlockPos, face: ProtocolDirection) {
        let item_signature = self.local_selected_hotbar_item_signature();
        self.local_player.interaction.destroying_block = Some(pos);
        self.local_player.interaction.destroying_block_face = Some(face);
        self.local_player.interaction.destroying_item_signature = Some(item_signature);
        self.local_player.interaction.destroying_block_progress = 0;
        self.local_player.interaction.destroying_block_stage = None;
        self.local_player.interaction.destroying_block_ticks = 0;
        self.local_player.interaction.destroy_delay_ticks = 0;
    }

    pub fn local_destroying_block_matches_current_item(&self) -> bool {
        self.local_player
            .interaction
            .destroying_item_signature
            .as_ref()
            .is_none_or(|signature| signature == &self.local_selected_hotbar_item_signature())
    }

    pub fn update_local_destroying_block_face(&mut self, face: ProtocolDirection) {
        if self.local_player.interaction.destroying_block.is_some() {
            self.local_player.interaction.destroying_block_face = Some(face);
        }
    }

    pub fn take_local_destroying_block(&mut self) -> Option<BlockPos> {
        self.local_player.interaction.destroying_block_face = None;
        self.local_player.interaction.destroying_item_signature = None;
        self.local_player.interaction.destroying_block_progress = 0;
        self.local_player.interaction.destroying_block_stage = None;
        self.local_player.interaction.destroying_block_ticks = 0;
        self.local_player.interaction.destroying_block.take()
    }

    pub fn take_local_destroying_block_hit(&mut self) -> Option<(BlockPos, ProtocolDirection)> {
        let pos = self.local_player.interaction.destroying_block.take()?;
        let face = self
            .local_player
            .interaction
            .destroying_block_face
            .take()
            .unwrap_or(ProtocolDirection::Down);
        self.local_player.interaction.destroying_item_signature = None;
        self.local_player.interaction.destroying_block_progress = 0;
        self.local_player.interaction.destroying_block_stage = None;
        self.local_player.interaction.destroying_block_ticks = 0;
        Some((pos, face))
    }

    pub fn set_local_using_item(&mut self, using_item: bool) {
        if using_item && !self.local_player.interaction.using_item {
            self.local_player.interaction.using_item_ticks = 0;
        }
        if !using_item {
            self.local_player.interaction.using_item_ticks = 0;
        }
        self.local_player.interaction.using_item = using_item;
        self.local_player.interaction.using_item_hand =
            using_item.then_some(InteractionHand::MainHand);
    }

    pub fn set_local_using_item_with_hand(&mut self, using_item: bool, hand: InteractionHand) {
        if using_item && !self.local_player.interaction.using_item {
            self.local_player.interaction.using_item_ticks = 0;
        }
        if !using_item {
            self.local_player.interaction.using_item_ticks = 0;
        }
        self.local_player.interaction.using_item = using_item;
        self.local_player.interaction.using_item_hand = using_item.then_some(hand);
    }

    pub fn advance_local_using_item_ticks(&mut self, ticks: u32) {
        if self.local_player.interaction.using_item {
            self.local_player.interaction.using_item_ticks = self
                .local_player
                .interaction
                .using_item_ticks
                .saturating_add(ticks);
        } else {
            self.local_player.interaction.using_item_ticks = 0;
        }
    }

    pub fn take_local_using_item(&mut self) -> bool {
        let using_item = self.local_player.interaction.using_item;
        self.local_player.interaction.using_item = false;
        self.local_player.interaction.using_item_hand = None;
        self.local_player.interaction.using_item_ticks = 0;
        using_item
    }

    pub fn local_player_auto_spin_attack(&self) -> Option<&LocalPlayerAutoSpinAttackState> {
        self.local_player.interaction.auto_spin_attack.as_ref()
    }

    pub(crate) fn start_local_player_auto_spin_attack(
        &mut self,
        ticks: u32,
        damage: f32,
        item: ProtocolItemStackSummary,
    ) {
        self.local_player.interaction.auto_spin_attack = Some(LocalPlayerAutoSpinAttackState {
            ticks_remaining: ticks,
            damage_bits: damage.to_bits(),
            item,
        });
    }

    pub(crate) fn advance_local_player_auto_spin_attack_tick(&mut self) -> bool {
        let Some(state) = &mut self.local_player.interaction.auto_spin_attack else {
            return false;
        };
        if state.ticks_remaining == 0 {
            return false;
        }
        state.ticks_remaining -= 1;
        true
    }

    pub(crate) fn stop_local_player_auto_spin_attack(&mut self) {
        if let Some(state) = &mut self.local_player.interaction.auto_spin_attack {
            state.ticks_remaining = 0;
        }
    }

    fn local_selected_hotbar_item_signature(&self) -> LocalPlayerDestroyItemSignature {
        self.inventory
            .selected_hotbar_item(self.local_player.selected_hotbar_slot)
            .map_or_else(
                LocalPlayerDestroyItemSignature::default,
                LocalPlayerDestroyItemSignature::from_item_stack,
            )
    }
}
