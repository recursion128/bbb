use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_LEVITATION_ID;
use bbb_protocol::packets::{
    DataComponentPatchSummary, Direction as ProtocolDirection, EntityAnchor, InteractionHand,
    ItemStackSummary as ProtocolItemStackSummary, PlayerAbilities as ProtocolPlayerAbilities,
    PlayerExperience as ProtocolPlayerExperience, PlayerHealth as ProtocolPlayerHealth,
    PlayerLookAt as ProtocolPlayerLookAt, PlayerPositionState as ProtocolPlayerPositionState,
    PlayerPositionUpdate as ProtocolPlayerPositionUpdate,
    PlayerRotationUpdate as ProtocolPlayerRotationUpdate, SetCamera as ProtocolSetCamera,
    SetDefaultSpawnPosition as ProtocolSetDefaultSpawnPosition, SetHeldSlot as ProtocolSetHeldSlot,
    SetSimulationDistance as ProtocolSetSimulationDistance, Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::local_player_fluid::{local_player_eye_in_water, local_player_fluid_contact};
use super::local_player_movement::{
    advance_local_player_auto_spin_attack_after_move, apply_local_player_input_look,
    integrate_local_player_input_pose_with_world_effects, local_player_effective_sprint,
    local_player_is_on_climbable,
};
use crate::{protocol_block_pos, BlockPos, WorldStore};

const STANDING_EYE_HEIGHT: f64 = 1.62;
const CROUCHING_EYE_HEIGHT: f64 = 1.27;
const SWIMMING_EYE_HEIGHT: f64 = 0.4;
const STANDING_BODY_HEIGHT: f64 = 1.8;
const CROUCHING_BODY_HEIGHT: f64 = 1.5;
const SWIMMING_BODY_HEIGHT: f64 = 0.6;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerState {
    #[serde(default)]
    pub abilities: Option<LocalPlayerAbilitiesState>,
    #[serde(default)]
    pub health: Option<LocalPlayerHealthState>,
    #[serde(default)]
    pub experience: Option<LocalPlayerExperienceState>,
    #[serde(default)]
    pub selected_hotbar_slot: u8,
    #[serde(default)]
    pub permission_level: LocalPlayerPermissionLevel,
    #[serde(default)]
    pub default_spawn: Option<DefaultSpawnState>,
    #[serde(default)]
    pub simulation_distance: Option<i32>,
    /// Server-synchronized `Entity.portalCooldown` for the multiplayer
    /// LocalPlayer. This is client tick state, not portal/world authority.
    #[serde(default)]
    pub portal_cooldown: i32,
    #[serde(default)]
    pub camera: CameraState,
    #[serde(default)]
    pub pose: Option<LocalPlayerPoseState>,
    #[serde(default)]
    pub last_look_at: Option<LocalPlayerLookAtState>,
    #[serde(default)]
    pub interaction: LocalPlayerInteractionState,
}

impl Default for LocalPlayerState {
    fn default() -> Self {
        Self {
            abilities: None,
            health: None,
            experience: None,
            selected_hotbar_slot: 0,
            permission_level: LocalPlayerPermissionLevel::default(),
            default_spawn: None,
            simulation_distance: None,
            portal_cooldown: 0,
            camera: CameraState::default(),
            pose: None,
            last_look_at: None,
            interaction: LocalPlayerInteractionState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalPlayerPermissionLevel {
    #[default]
    NoPermissions,
    Moderator,
    Gamemaster,
    Admin,
    Owner,
}

impl LocalPlayerPermissionLevel {
    fn from_entity_event_id(event_id: i8) -> Option<Self> {
        match event_id {
            24 => Some(Self::NoPermissions),
            25 => Some(Self::Moderator),
            26 => Some(Self::Gamemaster),
            27 => Some(Self::Admin),
            28 => Some(Self::Owner),
            _ => None,
        }
    }

    pub fn has_gamemaster_permission(self) -> bool {
        matches!(self, Self::Gamemaster | Self::Admin | Self::Owner)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerAbilitiesState {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instabuild: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerHealthState {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerExperienceState {
    pub progress: f32,
    pub level: i32,
    pub total: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPlayerInteractionState {
    #[serde(default)]
    pub destroying_block: Option<BlockPos>,
    #[serde(default)]
    pub destroying_block_face: Option<ProtocolDirection>,
    #[serde(default)]
    pub destroying_item_signature: Option<LocalPlayerDestroyItemSignature>,
    #[serde(default)]
    pub destroying_block_progress: u32,
    #[serde(default)]
    pub destroying_block_stage: Option<u8>,
    #[serde(default)]
    pub destroying_block_ticks: u32,
    #[serde(default)]
    pub destroy_delay_ticks: u8,
    #[serde(default)]
    pub using_item: bool,
    #[serde(default)]
    pub using_item_hand: Option<InteractionHand>,
    #[serde(default)]
    pub using_item_ticks: u32,
    /// Client-private `LivingEntity.autoSpinAttack*` state started by a
    /// locally released Riptide trident. The shared render flag remains
    /// server-synchronized entity metadata, matching `Player.startAutoSpinAttack`.
    #[serde(default)]
    pub auto_spin_attack: Option<LocalPlayerAutoSpinAttackState>,
    #[serde(default)]
    pub prediction_sequence: i32,
    /// Prediction sequence current when the last player-position teleport was
    /// applied. Predictions at or before this sequence may still reconcile
    /// blocks, but must not move the player back across the teleport.
    #[serde(default)]
    pub last_teleport_prediction_sequence: Option<i32>,
    #[serde(default)]
    pub attack: LocalPlayerAttackState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPlayerAutoSpinAttackState {
    pub ticks_remaining: u32,
    pub damage_bits: u32,
    pub item: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPlayerAttackState {
    #[serde(default)]
    pub attack_strength_ticks: u32,
    #[serde(default)]
    pub item_swap_ticks: u32,
    #[serde(default)]
    pub last_main_hand_item: Option<ProtocolItemStackSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPlayerDestroyItemSignature {
    pub item_id: Option<i32>,
    pub component_patch: DataComponentPatchSummary,
}

impl LocalPlayerDestroyItemSignature {
    fn from_item_stack(stack: &ProtocolItemStackSummary) -> Self {
        Self {
            item_id: stack.item_id,
            component_patch: stack.component_patch.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerPoseState {
    pub position: ProtocolVec3d,
    pub delta_movement: ProtocolVec3d,
    #[serde(default)]
    pub on_ground: bool,
    #[serde(default)]
    pub horizontal_collision: bool,
    #[serde(default)]
    pub fall_distance: f64,
    #[serde(default)]
    pub sneaking: bool,
    #[serde(default)]
    pub swimming: bool,
    pub y_rot: f32,
    pub x_rot: f32,
    pub last_teleport_id: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerInputState {
    pub focused: bool,
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    pub mouse_delta_x: f64,
    pub mouse_delta_y: f64,
}

impl LocalPlayerPoseState {
    pub fn position_state(self) -> ProtocolPlayerPositionState {
        ProtocolPlayerPositionState {
            position: self.position,
            delta_movement: self.delta_movement,
            y_rot: self.y_rot,
            x_rot: self.x_rot,
        }
    }

    pub fn eye_height(self) -> f64 {
        if self.swimming {
            SWIMMING_EYE_HEIGHT
        } else if self.sneaking {
            CROUCHING_EYE_HEIGHT
        } else {
            STANDING_EYE_HEIGHT
        }
    }

    pub fn body_height(self) -> f64 {
        if self.swimming {
            SWIMMING_BODY_HEIGHT
        } else if self.sneaking {
            CROUCHING_BODY_HEIGHT
        } else {
            STANDING_BODY_HEIGHT
        }
    }

    pub fn from_position_state(state: ProtocolPlayerPositionState, last_teleport_id: i32) -> Self {
        Self {
            position: state.position,
            delta_movement: state.delta_movement,
            on_ground: false,
            horizontal_collision: false,
            fall_distance: 0.0,
            sneaking: false,
            swimming: false,
            y_rot: state.y_rot,
            x_rot: state.x_rot,
            last_teleport_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerLookAtState {
    pub from_anchor: EntityAnchor,
    pub position: ProtocolVec3d,
    pub target_entity_id: Option<i32>,
    pub to_anchor: Option<EntityAnchor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultSpawnState {
    pub dimension: String,
    pub pos: BlockPos,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraState {
    pub entity_id: Option<i32>,
    pub follows_player: bool,
    pub entity_known: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            entity_id: None,
            follows_player: true,
            entity_known: true,
        }
    }
}

impl WorldStore {
    pub fn set_local_player_profile_id(&mut self, profile_id: Uuid) {
        self.local_player_profile_id = Some(profile_id);
    }

    pub fn local_player_profile_id(&self) -> Option<Uuid> {
        self.local_player_profile_id
    }

    pub fn apply_player_abilities(&mut self, packet: ProtocolPlayerAbilities) {
        self.counters.player_abilities_packets += 1;
        self.local_player.abilities = Some(LocalPlayerAbilitiesState {
            invulnerable: packet.invulnerable,
            flying: packet.flying,
            can_fly: packet.can_fly,
            instabuild: packet.instabuild,
            flying_speed: packet.flying_speed,
            walking_speed: packet.walking_speed,
        });
    }

    pub fn set_local_flying(&mut self, flying: bool) -> bool {
        let Some(abilities) = self.local_player.abilities.as_mut() else {
            return false;
        };
        if !abilities.can_fly {
            return false;
        }
        abilities.flying = flying;
        true
    }

    pub(crate) fn reset_local_player_for_login(&mut self, player_id: i32, game_type_id: i32) {
        self.local_player = LocalPlayerState::default();
        self.local_player.abilities =
            Some(game_type_player_abilities(GameType::from_id(game_type_id)));
        self.local_player.camera = CameraState {
            entity_id: Some(player_id),
            follows_player: true,
            entity_known: true,
        };
    }

    pub(crate) fn apply_local_player_permission_entity_event(
        &mut self,
        entity_id: i32,
        event_id: i8,
    ) -> bool {
        if self.local_player_id != Some(entity_id) {
            return false;
        }
        let Some(permission_level) = LocalPlayerPermissionLevel::from_entity_event_id(event_id)
        else {
            return false;
        };
        self.local_player.permission_level = permission_level;
        if !permission_level.has_gamemaster_permission() {
            self.close_game_rules_screen();
        }
        true
    }

    pub fn local_player_permission_level(&self) -> LocalPlayerPermissionLevel {
        self.local_player.permission_level
    }

    pub fn local_player_has_gamemaster_permission(&self) -> bool {
        self.local_player_id.is_some()
            && self
                .local_player
                .permission_level
                .has_gamemaster_permission()
    }

    /// Vanilla `Player.canUseGameMasterBlocks`: command/structure/test block
    /// interaction requires both the synchronized game-master permission and
    /// the current `Abilities.instabuild` capability.
    pub fn local_player_can_use_gamemaster_blocks(&self) -> bool {
        self.local_player_has_gamemaster_permission()
            && self
                .local_player
                .abilities
                .as_ref()
                .is_some_and(|abilities| abilities.instabuild)
    }

    pub fn adjust_local_flying_speed(&mut self, delta: f32) -> bool {
        let Some(abilities) = self.local_player.abilities.as_mut() else {
            return false;
        };
        let speed = (abilities.flying_speed + delta).clamp(0.0, 0.2);
        let changed = abilities.flying_speed != speed;
        abilities.flying_speed = speed;
        changed
    }

    pub fn apply_player_health(&mut self, packet: ProtocolPlayerHealth) {
        self.counters.player_health_packets += 1;
        self.local_player.health = Some(LocalPlayerHealthState {
            health: packet.health,
            food: packet.food,
            saturation: packet.saturation,
        });
    }

    pub fn local_player_is_dead(&self) -> bool {
        self.local_player
            .health
            .is_some_and(|health| health.health <= 0.0)
    }

    pub fn apply_player_experience(&mut self, packet: ProtocolPlayerExperience) {
        self.counters.player_experience_packets += 1;
        self.local_player.experience = Some(LocalPlayerExperienceState {
            progress: packet.progress,
            level: packet.level,
            total: packet.total,
        });
    }

    pub fn apply_held_slot(&mut self, packet: ProtocolSetHeldSlot) -> bool {
        self.counters.held_slot_packets += 1;
        if !(0..=8).contains(&packet.slot) {
            self.counters.held_slot_updates_ignored += 1;
            return false;
        }
        self.local_player.selected_hotbar_slot = packet.slot as u8;
        self.counters.held_slot_updates_applied += 1;
        self.close_invalid_writable_book_screen();
        true
    }

    pub fn set_local_selected_hotbar_slot(&mut self, slot: u8) -> bool {
        if slot > 8 {
            return false;
        }
        self.local_player.selected_hotbar_slot = slot;
        self.close_invalid_writable_book_screen();
        true
    }

    pub fn apply_default_spawn_position(&mut self, packet: ProtocolSetDefaultSpawnPosition) {
        self.counters.default_spawn_position_packets += 1;
        self.local_player.default_spawn = Some(DefaultSpawnState {
            dimension: packet.dimension,
            pos: protocol_block_pos(packet.pos),
            yaw: packet.yaw,
            pitch: packet.pitch,
        });
    }

    pub fn apply_simulation_distance(&mut self, packet: ProtocolSetSimulationDistance) {
        self.counters.simulation_distance_packets += 1;
        self.local_player.simulation_distance = Some(packet.distance);
    }

    pub fn apply_set_camera(&mut self, packet: ProtocolSetCamera) -> bool {
        self.counters.set_camera_packets += 1;
        let follows_player = self.local_player_id == Some(packet.camera_id);
        let entity_known = follows_player || self.entities.contains(packet.camera_id);
        if !entity_known {
            self.counters.set_camera_updates_ignored += 1;
            return false;
        }
        self.local_player.camera = CameraState {
            entity_id: Some(packet.camera_id),
            follows_player,
            entity_known,
        };
        self.counters.set_camera_updates_applied += 1;
        true
    }

    pub fn apply_player_position(
        &mut self,
        packet: ProtocolPlayerPositionUpdate,
    ) -> Option<LocalPlayerPoseState> {
        self.counters.player_position_packets += 1;
        let applied_pose = if self.local_player_vehicle_id.is_some() {
            None
        } else {
            let current_pose = self.local_player.pose.unwrap_or_default();
            let current = current_pose.position_state();
            let state = packet.apply_to_state(current);
            let pose = LocalPlayerPoseState {
                sneaking: current_pose.sneaking,
                swimming: current_pose.swimming,
                ..LocalPlayerPoseState::from_position_state(state, packet.id)
            };
            self.local_player.pose = Some(pose);
            Some(pose)
        };
        self.local_player
            .interaction
            .last_teleport_prediction_sequence =
            Some(self.local_player.interaction.prediction_sequence);
        applied_pose
    }

    pub fn apply_player_rotation(
        &mut self,
        packet: ProtocolPlayerRotationUpdate,
    ) -> LocalPlayerPoseState {
        self.counters.player_rotation_packets += 1;
        let current_pose = self.local_player.pose.unwrap_or_default();
        let state = packet.apply_to_state(current_pose.position_state());
        let pose = LocalPlayerPoseState {
            sneaking: current_pose.sneaking,
            swimming: current_pose.swimming,
            ..LocalPlayerPoseState::from_position_state(state, current_pose.last_teleport_id)
        };
        self.local_player.pose = Some(pose);
        pose
    }

    pub fn apply_player_look_at(
        &mut self,
        packet: ProtocolPlayerLookAt,
    ) -> Option<LocalPlayerPoseState> {
        self.counters.player_look_at_packets += 1;
        let target_position = self.resolve_look_at_target_position(packet);
        self.local_player.last_look_at = Some(LocalPlayerLookAtState {
            from_anchor: packet.from_anchor,
            position: target_position,
            target_entity_id: packet.target.map(|target| target.entity_id),
            to_anchor: packet.target.map(|target| target.to_anchor),
        });

        let pose =
            apply_look_at_to_pose(self.local_player.pose?, packet.from_anchor, target_position);
        self.local_player.pose = Some(pose);
        Some(pose)
    }

    pub fn set_local_player_pose(&mut self, pose: LocalPlayerPoseState) {
        self.local_player.pose = Some(pose);
        self.local_player_old_pose = Some(pose);
    }

    pub fn advance_local_player_input(
        &mut self,
        input: LocalPlayerInputState,
        dt_seconds: f64,
    ) -> Option<LocalPlayerPoseState> {
        let old_pose = self.local_player.pose?;
        self.local_player_old_pose = Some(old_pose);
        let pose =
            integrate_local_player_input_pose_with_world_effects(self, old_pose, input, dt_seconds);
        let pose = advance_local_player_auto_spin_attack_after_move(self, old_pose, pose);
        self.local_player.pose = Some(pose);
        Some(pose)
    }

    pub fn local_player_effective_sprint(&self, input: LocalPlayerInputState) -> bool {
        local_player_effective_sprint(self, input)
    }

    pub fn local_player_eye_in_water(&self) -> bool {
        local_player_eye_in_water(self)
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
            .and_then(|id| {
                self.entities
                    .attribute_value(id, VANILLA_ATTRIBUTE_ARMOR_ID)
            })
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
            .and_then(|id| self.entities.air_supply(id))
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
            .and_then(|id| self.entities.player_absorption(id))
            .unwrap_or(0.0)
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
        let pose = apply_local_player_input_look(self.local_player.pose?, input);
        self.local_player.pose = Some(pose);
        Some(pose)
    }

    pub fn local_player_pose(&self) -> Option<LocalPlayerPoseState> {
        self.local_player.pose
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
            position: ProtocolVec3d {
                x: old.position.x + (current.position.x - old.position.x) * partial_tick_f64,
                y: old.position.y + (current.position.y - old.position.y) * partial_tick_f64,
                z: old.position.z + (current.position.z - old.position.z) * partial_tick_f64,
            },
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

    pub fn next_local_prediction_sequence(&mut self) -> i32 {
        let sequence = self.local_player.interaction.prediction_sequence;
        self.local_player.interaction.prediction_sequence = if sequence == i32::MAX {
            1
        } else {
            sequence + 1
        };
        self.local_player.interaction.prediction_sequence
    }

    pub fn set_local_prediction_sequence(&mut self, sequence: i32) {
        self.local_player.interaction.prediction_sequence = sequence.max(0);
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

    fn resolve_look_at_target_position(&self, packet: ProtocolPlayerLookAt) -> ProtocolVec3d {
        packet
            .target
            .and_then(|target| match target.to_anchor {
                EntityAnchor::Feet => {
                    self.probe_entity(target.entity_id)
                        .map(|entity| ProtocolVec3d {
                            x: entity.position.x,
                            y: entity.position.y,
                            z: entity.position.z,
                        })
                }
                EntityAnchor::Eyes => {
                    self.probe_entity_camera_pose(target.entity_id)
                        .map(|pose| ProtocolVec3d {
                            x: pose.position.x,
                            y: pose.position.y + f64::from(pose.eye_height),
                            z: pose.position.z,
                        })
                }
            })
            .unwrap_or(packet.position)
    }

    fn local_selected_hotbar_item_signature(&self) -> LocalPlayerDestroyItemSignature {
        let hotbar_items = self.inventory.hotbar_items();
        let selected_slot = usize::from(self.local_player.selected_hotbar_slot.min(8));
        LocalPlayerDestroyItemSignature::from_item_stack(&hotbar_items[selected_slot])
    }
}

pub(crate) fn game_type_player_abilities(game_type: GameType) -> LocalPlayerAbilitiesState {
    let (invulnerable, flying, can_fly, instabuild) = match game_type {
        GameType::Creative => (true, false, true, true),
        GameType::Spectator => (true, true, true, false),
        GameType::Survival | GameType::Adventure => (false, false, false, false),
    };
    LocalPlayerAbilitiesState {
        invulnerable,
        flying,
        can_fly,
        instabuild,
        flying_speed: 0.05,
        walking_speed: 0.1,
    }
}

fn apply_look_at_to_pose(
    pose: LocalPlayerPoseState,
    from_anchor: EntityAnchor,
    target_position: ProtocolVec3d,
) -> LocalPlayerPoseState {
    let from_y = match from_anchor {
        EntityAnchor::Feet => pose.position.y,
        EntityAnchor::Eyes => pose.position.y + pose.eye_height(),
    };
    let dx = target_position.x - pose.position.x;
    let dy = target_position.y - from_y;
    let dz = target_position.z - pose.position.z;
    let horizontal = (dx * dx + dz * dz).sqrt();
    let x_rot = wrap_degrees_f32(-(dy.atan2(horizontal).to_degrees() as f32));
    let y_rot = wrap_degrees_f32(dz.atan2(dx).to_degrees() as f32 - 90.0);

    LocalPlayerPoseState {
        y_rot,
        x_rot,
        ..pose
    }
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::super::local_player_movement::LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND;
    use super::*;
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityEvent as ProtocolEntityEvent, PlayerLookAtTarget,
        Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT,
        PLAYER_RELATIVE_Y_ROT,
    };
    use uuid::Uuid;

    #[test]
    fn login_player_abilities_match_vanilla_game_types() {
        let survival = LocalPlayerAbilitiesState {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        };
        assert_eq!(game_type_player_abilities(GameType::Survival), survival);
        assert_eq!(game_type_player_abilities(GameType::Adventure), survival);
        assert_eq!(
            game_type_player_abilities(GameType::Creative),
            LocalPlayerAbilitiesState {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            }
        );
        assert_eq!(
            game_type_player_abilities(GameType::Spectator),
            LocalPlayerAbilitiesState {
                invulnerable: true,
                flying: true,
                can_fly: true,
                instabuild: false,
                flying_speed: 0.05,
                walking_speed: 0.1,
            }
        );
    }

    #[test]
    fn local_player_packets_update_canonical_state() {
        let mut store = WorldStore::new();

        store.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        store.apply_player_health(ProtocolPlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        });
        store.apply_player_experience(ProtocolPlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        });
        assert!(store.apply_held_slot(ProtocolSetHeldSlot { slot: 5 }));
        assert!(!store.apply_held_slot(ProtocolSetHeldSlot { slot: 99 }));
        store.apply_default_spawn_position(ProtocolSetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: bbb_protocol::packets::BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        });
        store.apply_simulation_distance(ProtocolSetSimulationDistance { distance: 12 });

        let local = store.local_player();
        assert_eq!(
            local.abilities,
            Some(LocalPlayerAbilitiesState {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            })
        );
        assert_eq!(
            local.health,
            Some(LocalPlayerHealthState {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            })
        );
        assert_eq!(
            local.experience,
            Some(LocalPlayerExperienceState {
                progress: 0.75,
                level: 8,
                total: 123,
            })
        );
        assert_eq!(local.selected_hotbar_slot, 5);
        assert_eq!(
            local.default_spawn,
            Some(DefaultSpawnState {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                yaw: 90.0,
                pitch: -10.0,
            })
        );
        assert_eq!(local.simulation_distance, Some(12));

        let counters = store.counters();
        assert_eq!(counters.player_abilities_packets, 1);
        assert_eq!(counters.player_health_packets, 1);
        assert_eq!(counters.player_experience_packets, 1);
        assert_eq!(counters.held_slot_packets, 2);
        assert_eq!(counters.held_slot_updates_applied, 1);
        assert_eq!(counters.held_slot_updates_ignored, 1);
        assert_eq!(counters.default_spawn_position_packets, 1);
        assert_eq!(counters.simulation_distance_packets, 1);
    }

    #[test]
    fn local_player_is_dead_tracks_canonical_health() {
        let mut store = WorldStore::new();

        assert!(!store.local_player_is_dead());

        store.apply_player_health(ProtocolPlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        });
        assert!(!store.local_player_is_dead());

        store.apply_player_health(ProtocolPlayerHealth {
            health: 0.0,
            food: 16,
            saturation: 2.0,
        });
        assert!(store.local_player_is_dead());

        store.apply_player_health(ProtocolPlayerHealth {
            health: 1.0,
            food: 16,
            saturation: 2.0,
        });
        assert!(!store.local_player_is_dead());
    }

    #[test]
    fn local_player_permission_entity_events_update_gamemaster_gate_without_entity_projection() {
        let mut store = WorldStore::new();
        store.local_player_id = Some(7);

        assert_eq!(
            store.local_player_permission_level(),
            LocalPlayerPermissionLevel::NoPermissions
        );
        assert!(!store.local_player_has_gamemaster_permission());

        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 7,
            event_id: 26,
        }));
        assert_eq!(
            store.local_player_permission_level(),
            LocalPlayerPermissionLevel::Gamemaster
        );
        assert!(store.local_player_has_gamemaster_permission());

        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 7,
            event_id: 25,
        }));
        assert_eq!(
            store.local_player_permission_level(),
            LocalPlayerPermissionLevel::Moderator
        );
        assert!(!store.local_player_has_gamemaster_permission());

        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 7,
            event_id: 28,
        }));
        assert_eq!(
            store.local_player_permission_level(),
            LocalPlayerPermissionLevel::Owner
        );
        assert!(store.local_player_has_gamemaster_permission());

        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 7,
            event_id: 24,
        }));
        assert_eq!(
            store.local_player_permission_level(),
            LocalPlayerPermissionLevel::NoPermissions
        );
        assert!(!store.local_player_has_gamemaster_permission());
        assert_eq!(store.counters().entity_events_applied, 4);
        assert_eq!(store.counters().entity_events_ignored, 0);
    }

    #[test]
    fn remote_permission_entity_event_does_not_update_local_player_permission() {
        let mut store = WorldStore::new();
        store.local_player_id = Some(7);
        store.apply_add_entity(ProtocolAddEntity {
            id: 8,
            uuid: Uuid::from_u128(8),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });

        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 8,
            event_id: 26,
        }));

        assert_eq!(
            store.local_player_permission_level(),
            LocalPlayerPermissionLevel::NoPermissions
        );
        assert!(!store.local_player_has_gamemaster_permission());
        assert_eq!(store.counters().entity_events_applied, 1);
        assert_eq!(store.counters().entity_events_ignored, 0);
    }

    #[test]
    fn local_hotbar_selection_updates_without_counting_server_packet() {
        let mut store = WorldStore::new();

        assert!(store.set_local_selected_hotbar_slot(7));
        assert_eq!(store.local_player().selected_hotbar_slot, 7);
        assert_eq!(store.counters().held_slot_packets, 0);
        assert_eq!(store.counters().held_slot_updates_applied, 0);
        assert_eq!(store.counters().held_slot_updates_ignored, 0);

        assert!(!store.set_local_selected_hotbar_slot(9));
        assert_eq!(store.local_player().selected_hotbar_slot, 7);
        assert_eq!(store.counters().held_slot_packets, 0);
        assert_eq!(store.counters().held_slot_updates_applied, 0);
        assert_eq!(store.counters().held_slot_updates_ignored, 0);
    }

    #[test]
    fn local_flying_updates_only_when_server_allows_flight() {
        let mut store = WorldStore::new();

        assert!(!store.set_local_flying(true));

        store.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert!(!store.set_local_flying(true));
        assert!(!store.local_player().abilities.unwrap().flying);

        store.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert!(store.set_local_flying(true));
        assert!(store.local_player().abilities.unwrap().flying);
        assert!(store.set_local_flying(false));
        assert!(!store.local_player().abilities.unwrap().flying);
        assert_eq!(store.counters().player_abilities_packets, 2);
    }

    #[test]
    fn local_flying_speed_adjusts_without_counting_server_packet() {
        let mut store = WorldStore::new();

        assert!(!store.adjust_local_flying_speed(0.005));

        store.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: false,
            flying: true,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });

        assert!(store.adjust_local_flying_speed(0.005));
        assert_eq!(store.local_player().abilities.unwrap().flying_speed, 0.055);
        assert!(store.adjust_local_flying_speed(1.0));
        assert_eq!(store.local_player().abilities.unwrap().flying_speed, 0.2);
        assert!(!store.adjust_local_flying_speed(1.0));
        assert_eq!(store.counters().player_abilities_packets, 1);
    }

    #[test]
    fn local_player_input_moves_forward_with_minecraft_yaw() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            y_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });

        let pose = store
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                0.05,
            )
            .unwrap();

        assert_f64_near(pose.position.x, 0.0, 0.000001);
        assert_f64_near(pose.position.y, 64.0, 0.000001);
        assert_f64_near(
            pose.position.z,
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.05,
            0.000001,
        );
        assert_f64_near(
            pose.delta_movement.z,
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND / 20.0,
            0.000001,
        );
        assert!(!pose.on_ground);
        assert_eq!(store.local_player_pose(), Some(pose));
    }

    #[test]
    fn local_player_input_rotates_and_clamps_pitch() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        });

        let pose = store
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    mouse_delta_x: 100.0,
                    mouse_delta_y: 1000.0,
                    ..LocalPlayerInputState::default()
                },
                0.0,
            )
            .unwrap();

        assert_eq!(pose.y_rot, 12.0);
        assert_eq!(pose.x_rot, 90.0);
    }

    #[test]
    fn local_player_tick_retains_partial_camera_pose_endpoints() {
        let mut store = WorldStore::new();
        let initial = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            y_rot: 170.0,
            x_rot: -10.0,
            ..LocalPlayerPoseState::default()
        };
        store.set_local_player_pose(initial);

        let current = store
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    mouse_delta_x: 200.0,
                    mouse_delta_y: 100.0,
                    ..LocalPlayerInputState::default()
                },
                0.05,
            )
            .unwrap();
        assert_eq!(store.local_player_old_pose, Some(initial));

        let old = store.local_player_pose_at_partial_tick(0.0).unwrap();
        let mid = store.local_player_pose_at_partial_tick(0.5).unwrap();
        let end = store.local_player_pose_at_partial_tick(1.0).unwrap();
        assert_eq!(old.position, initial.position);
        assert_eq!(end.position, current.position);
        assert_f64_near(
            mid.position.z,
            (initial.position.z + current.position.z) * 0.5,
            0.000001,
        );
        assert_eq!(old.y_rot, 170.0);
        assert_eq!(mid.y_rot, 182.0);
        assert_eq!(end.y_rot, current.y_rot);
        assert_eq!(mid.x_rot, -4.0);
    }

    #[test]
    fn local_player_look_input_rotates_without_moving_position() {
        let mut store = WorldStore::new();
        let initial = LocalPlayerPoseState {
            position: vec3(3.0, 64.0, -2.0),
            delta_movement: vec3(0.2, 0.0, -0.1),
            y_rot: 170.0,
            x_rot: 10.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        };
        store.set_local_player_pose(initial);

        let pose = store
            .advance_local_player_look_input(LocalPlayerInputState {
                focused: true,
                mouse_delta_x: 200.0,
                mouse_delta_y: -1000.0,
                ..LocalPlayerInputState::default()
            })
            .unwrap();

        assert_eq!(pose.position, initial.position);
        assert_eq!(pose.delta_movement, initial.delta_movement);
        assert_eq!(pose.y_rot, -166.0);
        assert_eq!(pose.x_rot, -90.0);
        assert_eq!(store.local_player_pose(), Some(pose));
    }

    #[test]
    fn local_player_input_is_ignored_without_pose() {
        let mut store = WorldStore::new();

        assert_eq!(
            store.advance_local_player_input(LocalPlayerInputState::default(), 1.0),
            None
        );
    }

    #[test]
    fn local_player_pose_eye_height_tracks_sneaking() {
        assert_eq!(
            LocalPlayerPoseState::default().eye_height(),
            STANDING_EYE_HEIGHT
        );
        assert_eq!(
            LocalPlayerPoseState {
                sneaking: true,
                ..LocalPlayerPoseState::default()
            }
            .eye_height(),
            CROUCHING_EYE_HEIGHT
        );
        assert_eq!(
            LocalPlayerPoseState {
                swimming: true,
                ..LocalPlayerPoseState::default()
            }
            .eye_height(),
            SWIMMING_EYE_HEIGHT
        );
    }

    #[test]
    fn local_player_pose_body_height_tracks_sneaking() {
        assert_eq!(
            LocalPlayerPoseState::default().body_height(),
            STANDING_BODY_HEIGHT
        );
        assert_eq!(
            LocalPlayerPoseState {
                sneaking: true,
                ..LocalPlayerPoseState::default()
            }
            .body_height(),
            CROUCHING_BODY_HEIGHT
        );
        assert_eq!(
            LocalPlayerPoseState {
                swimming: true,
                ..LocalPlayerPoseState::default()
            }
            .body_height(),
            SWIMMING_BODY_HEIGHT
        );
    }

    #[test]
    fn local_interaction_state_tracks_prediction_destroy_and_use_item() {
        let mut store = WorldStore::new();

        assert_eq!(store.next_local_prediction_sequence(), 1);
        assert_eq!(store.next_local_prediction_sequence(), 2);
        store.set_local_prediction_sequence(i32::MAX);
        assert_eq!(store.next_local_prediction_sequence(), 1);
        store.set_local_prediction_sequence(-5);
        assert_eq!(store.next_local_prediction_sequence(), 1);

        let pos = BlockPos { x: 4, y: 70, z: -6 };
        store.set_local_destroying_block(pos);
        store.local_player.interaction.destroying_block_progress = 123;
        store.local_player.interaction.destroying_block_stage = Some(1);
        assert_eq!(store.local_player().interaction.destroying_block, Some(pos));
        assert_eq!(store.local_player().interaction.destroying_block_face, None);
        assert_eq!(
            store.take_local_destroying_block_hit(),
            Some((pos, ProtocolDirection::Down))
        );
        assert_eq!(
            store.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(store.take_local_destroying_block_hit(), None);

        store.set_local_destroying_block_hit(pos, ProtocolDirection::North);
        store.local_player.interaction.destroying_block_progress = 456;
        store.local_player.interaction.destroying_block_stage = Some(4);
        assert_eq!(store.local_player().interaction.destroying_block, Some(pos));
        assert_eq!(
            store.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::North)
        );
        assert_eq!(store.take_local_destroying_block(), Some(pos));
        assert_eq!(store.take_local_destroying_block(), None);
        assert_eq!(store.local_player().interaction.destroying_block_face, None);
        assert_eq!(
            store.local_player().interaction.destroying_block_stage,
            None
        );

        store.set_local_using_item(true);
        assert_eq!(store.local_player().interaction.using_item_ticks, 0);
        store.advance_local_using_item_ticks(3);
        assert_eq!(store.local_player().interaction.using_item_ticks, 3);
        assert!(store.take_local_using_item());
        assert_eq!(store.local_player().interaction.using_item_ticks, 0);
        assert!(!store.take_local_using_item());
    }

    #[test]
    fn local_interaction_state_survives_world_store_json_round_trip() {
        let mut store = WorldStore::new();
        store.set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
        store.set_local_using_item(true);
        assert_eq!(store.next_local_prediction_sequence(), 1);

        let restored: WorldStore = serde_json::from_value(serde_json::to_value(&store).unwrap())
            .expect("world store local interaction should deserialize");

        assert_eq!(
            restored.local_player().interaction,
            LocalPlayerInteractionState {
                destroying_block: Some(BlockPos { x: 1, y: 2, z: 3 }),
                destroying_block_face: None,
                destroying_item_signature: Some(LocalPlayerDestroyItemSignature::from_item_stack(
                    &ProtocolItemStackSummary::empty(),
                )),
                destroying_block_progress: 0,
                destroying_block_stage: None,
                destroying_block_ticks: 0,
                destroy_delay_ticks: 0,
                using_item: true,
                using_item_hand: Some(InteractionHand::MainHand),
                using_item_ticks: 0,
                auto_spin_attack: None,
                prediction_sequence: 1,
                last_teleport_prediction_sequence: None,
                attack: LocalPlayerAttackState::default(),
            }
        );
    }

    #[test]
    fn local_attack_ticks_follow_main_hand_match_and_item_identity_changes() {
        let mut store = local_attack_player_world();
        store.advance_local_player_attack_strength_ticks(3);
        assert_eq!(store.local_player_attack_strength_ticks(), 3);
        assert_eq!(store.local_player_item_swap_ticks(), 3);
        assert_eq!(store.local_player_item_swap_scale(1.0), 0.8);
        assert_eq!(store.local_player_item_swap_scale_ticks_ago(2, 1.0), 0.4);

        store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: local_attack_item(42, 1),
        });
        store.advance_local_player_attack_strength_ticks(3);
        assert_eq!(store.local_player_attack_strength_ticks(), 2);
        assert_eq!(store.local_player_item_swap_ticks(), 2);

        store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: local_attack_item(42, 2),
        });
        store.advance_local_player_attack_strength_ticks(2);
        assert_eq!(store.local_player_attack_strength_ticks(), 4);
        assert_eq!(store.local_player_item_swap_ticks(), 4);
        assert_eq!(
            store
                .local_player()
                .interaction
                .attack
                .last_main_hand_item
                .as_ref()
                .map(|item| item.count),
            Some(2)
        );

        let mut patched = local_attack_item(42, 2);
        patched.component_patch.minimum_attack_charge_bits = Some(0.5_f32.to_bits());
        store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: patched,
        });
        store.advance_local_player_attack_strength_ticks(1);
        assert_eq!(store.local_player_attack_strength_ticks(), 5);

        store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: local_attack_item(43, 1),
        });
        store.advance_local_player_attack_strength_ticks(4);
        assert_eq!(store.local_player_attack_strength_ticks(), 3);
        assert_eq!(store.local_player_item_swap_ticks(), 3);

        store.reset_local_player_attack_strength();
        assert_eq!(store.local_player_attack_strength_ticks(), 0);
        assert_eq!(store.local_player_item_swap_ticks(), 0);
    }

    #[test]
    fn local_attack_charge_uses_actual_attack_speed_and_inclusive_boundary() {
        let mut store = local_attack_player_world();
        store.set_default_item_minimum_attack_charges(BTreeMap::from([(42, 1.0)]));
        store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: local_attack_item(42, 1),
        });
        store.advance_local_player_attack_strength_ticks(1);
        assert_eq!(
            store.local_player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_SPEED_ID),
            Some(4.0)
        );
        store.advance_local_player_attack_strength_ticks(4);
        assert!(store.local_player_cannot_attack_with_selected_item());
        store.advance_local_player_attack_strength_ticks(1);
        assert!(!store.local_player_cannot_attack_with_selected_item());

        let mut patched = local_attack_item(42, 1);
        patched.component_patch.minimum_attack_charge_bits = Some(0.5_f32.to_bits());
        store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: patched,
        });
        assert!(
            store.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
                entity_id: 7,
                attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
                    base: 2.0,
                    modifiers: Vec::new(),
                }],
            })
        );
        store.reset_local_player_attack_strength();
        store.advance_local_player_attack_strength_ticks(4);
        assert!(store.local_player_cannot_attack_with_selected_item());
        store.advance_local_player_attack_strength_ticks(1);
        assert!(!store.local_player_cannot_attack_with_selected_item());

        assert!(
            store.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
                entity_id: 7,
                attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
                    base: 0.0,
                    modifiers: Vec::new(),
                }],
            })
        );
        assert!(store.local_player_cannot_attack_with_selected_item());
    }

    #[test]
    fn local_attack_state_serializes_and_resets_with_the_client_level() {
        let mut store = local_attack_player_world();
        store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: local_attack_item(42, 3),
        });
        store.advance_local_player_attack_strength_ticks(6);
        let restored: WorldStore = serde_json::from_value(serde_json::to_value(&store).unwrap())
            .expect("local attack state should deserialize");
        assert_eq!(
            restored.local_player().interaction.attack,
            store.local_player().interaction.attack
        );

        store.clear_client_level();
        assert_eq!(
            store.local_player().interaction.attack,
            LocalPlayerAttackState::default()
        );
    }

    #[test]
    fn camera_updates_only_for_local_or_known_entities() {
        let mut store = WorldStore::new();
        store.local_player_id = Some(9);

        assert!(!store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
        assert_eq!(store.local_player().camera, CameraState::default());

        assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 9 }));
        assert_eq!(
            store.local_player().camera,
            CameraState {
                entity_id: Some(9),
                follows_player: true,
                entity_known: true,
            }
        );

        store.apply_add_entity(protocol_add_entity(123));
        assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
        assert_eq!(
            store.local_player().camera,
            CameraState {
                entity_id: Some(123),
                follows_player: false,
                entity_known: true,
            }
        );
        assert_eq!(store.counters().set_camera_packets, 3);
        assert_eq!(store.counters().set_camera_updates_applied, 2);
        assert_eq!(store.counters().set_camera_updates_ignored, 1);
    }

    #[test]
    fn local_player_position_and_rotation_update_canonical_pose() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            sneaking: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = store
            .apply_player_position(ProtocolPlayerPositionUpdate {
                id: 7,
                position: vec3(10.0, 64.0, -5.0),
                delta_movement: vec3(0.125, 0.0, 0.0),
                y_rot: 90.0,
                x_rot: 15.0,
                relatives_mask: 0,
            })
            .unwrap();
        assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
        assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
        assert!(pose.sneaking);
        assert_eq!(pose.last_teleport_id, 7);

        let pose = store
            .apply_player_position(ProtocolPlayerPositionUpdate {
                id: 8,
                position: vec3(1.5, -2.0, 7.0),
                delta_movement: vec3(0.25, 0.5, 0.75),
                y_rot: 20.0,
                x_rot: -120.0,
                relatives_mask: PLAYER_RELATIVE_X
                    | PLAYER_RELATIVE_Y_ROT
                    | PLAYER_RELATIVE_X_ROT
                    | PLAYER_RELATIVE_DELTA_X,
            })
            .unwrap();
        assert_eq!(pose.position, vec3(11.5, -2.0, 7.0));
        assert_eq!(pose.delta_movement, vec3(0.375, 0.5, 0.75));
        assert_eq!(pose.y_rot, 110.0);
        assert_eq!(pose.x_rot, -90.0);
        assert!(pose.sneaking);
        assert_eq!(pose.last_teleport_id, 8);

        let pose = store.apply_player_rotation(ProtocolPlayerRotationUpdate {
            y_rot: -10.0,
            relative_y: true,
            x_rot: 30.0,
            relative_x: false,
        });
        assert_eq!(pose.position, vec3(11.5, -2.0, 7.0));
        assert_eq!(pose.delta_movement, vec3(0.375, 0.5, 0.75));
        assert_eq!(pose.y_rot, 100.0);
        assert_eq!(pose.x_rot, 30.0);
        assert!(pose.sneaking);
        assert_eq!(pose.last_teleport_id, 8);
        assert_eq!(store.local_player_pose(), Some(pose));

        let counters = store.counters();
        assert_eq!(counters.player_position_packets, 2);
        assert_eq!(counters.player_rotation_packets, 1);
    }

    #[test]
    fn local_player_position_and_rotation_preserve_swimming_pose() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            swimming: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = store
            .apply_player_position(ProtocolPlayerPositionUpdate {
                id: 7,
                position: vec3(10.0, 64.0, -5.0),
                delta_movement: vec3(0.125, 0.0, 0.0),
                y_rot: 90.0,
                x_rot: 15.0,
                relatives_mask: 0,
            })
            .unwrap();
        assert!(!pose.sneaking);
        assert!(pose.swimming);

        let pose = store.apply_player_rotation(ProtocolPlayerRotationUpdate {
            y_rot: -10.0,
            relative_y: true,
            x_rot: 30.0,
            relative_x: false,
        });
        assert!(!pose.sneaking);
        assert!(pose.swimming);
        assert_eq!(store.local_player_pose(), Some(pose));
    }

    #[test]
    fn local_player_position_does_not_move_mounted_player() {
        let mut store = WorldStore::new();
        let initial_pose = LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            delta_movement: vec3(0.0, 0.0, 0.0),
            y_rot: 0.0,
            x_rot: 0.0,
            last_teleport_id: 3,
            ..LocalPlayerPoseState::default()
        };
        store.set_local_player_pose(initial_pose);
        store.local_player_vehicle_id = Some(10);

        assert_eq!(
            store.apply_player_position(ProtocolPlayerPositionUpdate {
                id: 9,
                position: vec3(10.0, 70.0, 10.0),
                delta_movement: vec3(1.0, 1.0, 1.0),
                y_rot: 90.0,
                x_rot: 45.0,
                relatives_mask: 0,
            }),
            None
        );
        assert_eq!(store.local_player_pose(), Some(initial_pose));
        assert_eq!(
            store
                .local_player()
                .interaction
                .last_teleport_prediction_sequence,
            Some(0)
        );
        assert_eq!(store.counters().player_position_packets, 1);
    }

    #[test]
    fn local_player_look_at_uses_known_target_entity_anchor() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            delta_movement: vec3(0.0, 0.0, 0.0),
            y_rot: 90.0,
            x_rot: 30.0,
            last_teleport_id: 7,
            ..LocalPlayerPoseState::default()
        });
        store.apply_add_entity(protocol_add_entity_at(123, vec3(0.0, 70.0, 10.0)));

        let pose = store
            .apply_player_look_at(ProtocolPlayerLookAt {
                from_anchor: EntityAnchor::Eyes,
                position: vec3(50.0, 50.0, 50.0),
                target: Some(PlayerLookAtTarget {
                    entity_id: 123,
                    to_anchor: EntityAnchor::Feet,
                }),
            })
            .unwrap();

        let expected_x_rot = -((70.0 - 65.62_f64).atan2(10.0).to_degrees() as f32);
        assert!((pose.y_rot - 0.0).abs() < 0.001);
        assert!((pose.x_rot - expected_x_rot).abs() < 0.001);
        assert_eq!(pose.last_teleport_id, 7);
        assert_eq!(
            store.local_player().last_look_at,
            Some(LocalPlayerLookAtState {
                from_anchor: EntityAnchor::Eyes,
                position: vec3(0.0, 70.0, 10.0),
                target_entity_id: Some(123),
                to_anchor: Some(EntityAnchor::Feet),
            })
        );
        assert_eq!(store.counters().player_look_at_packets, 1);
    }

    #[test]
    fn local_player_look_at_uses_target_entity_specific_eye_height() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            ..LocalPlayerPoseState::default()
        });
        store.apply_add_entity(protocol_add_entity_at(123, vec3(0.0, 70.0, 10.0)));
        let target_eye_height = f64::from(
            store
                .probe_entity_camera_pose(123)
                .expect("known target camera pose")
                .eye_height,
        );

        let pose = store
            .apply_player_look_at(ProtocolPlayerLookAt {
                from_anchor: EntityAnchor::Eyes,
                position: vec3(50.0, 50.0, 50.0),
                target: Some(PlayerLookAtTarget {
                    entity_id: 123,
                    to_anchor: EntityAnchor::Eyes,
                }),
            })
            .unwrap();

        let expected_x_rot = -((70.0 + target_eye_height - 65.62_f64)
            .atan2(10.0)
            .to_degrees() as f32);
        assert!((pose.x_rot - expected_x_rot).abs() < 0.001);
        assert_eq!(
            store.local_player().last_look_at.unwrap().position,
            vec3(0.0, 70.0 + target_eye_height, 10.0)
        );
    }

    #[test]
    fn local_player_look_at_uses_crouching_eye_height_when_sneaking() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.0, 64.0, 0.0),
            sneaking: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = store
            .apply_player_look_at(ProtocolPlayerLookAt {
                from_anchor: EntityAnchor::Eyes,
                position: vec3(0.0, 70.0, 10.0),
                target: None,
            })
            .unwrap();

        let expected_x_rot = -((70.0 - (64.0 + CROUCHING_EYE_HEIGHT))
            .atan2(10.0)
            .to_degrees() as f32);
        assert!((pose.y_rot - 0.0).abs() < 0.001);
        assert!((pose.x_rot - expected_x_rot).abs() < 0.001);
        assert!(pose.sneaking);
    }

    fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
        protocol_add_entity_at(
            id,
            ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        )
    }

    fn local_attack_player_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.local_player_id = Some(7);
        world.apply_add_entity(ProtocolAddEntity {
            id: 7,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: vec3(0.5, 64.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world
    }

    fn local_attack_item(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn protocol_add_entity_at(id: i32, position: ProtocolVec3d) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            position,
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -10.0,
            y_rot: 20.0,
            y_head_rot: 30.0,
            data: 99,
        }
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}"
        );
    }

    fn player_world_with_armor(
        modifiers: Vec<bbb_protocol::packets::AttributeModifier>,
        base: f64,
    ) -> WorldStore {
        let mut world = WorldStore::new();
        world.local_player_id = Some(7);
        world.apply_add_entity(ProtocolAddEntity {
            id: 7,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(
            world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
                entity_id: 7,
                attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                    // ARMOR is registration index 0 in BuiltInRegistries.ATTRIBUTE.
                    attribute_id: VANILLA_ATTRIBUTE_ARMOR_ID,
                    base,
                    modifiers,
                }],
            })
        );
        world
    }

    #[test]
    fn local_player_armor_value_defaults_to_zero_without_attribute() {
        // No local player at all.
        let world = WorldStore::new();
        assert_eq!(world.local_player_armor_value(), 0);

        // A local player entity with no synced ARMOR attribute still reads 0
        // (vanilla `RangedAttribute` default), not a panic.
        let mut world = WorldStore::new();
        world.local_player_id = Some(7);
        world.apply_add_entity(ProtocolAddEntity {
            id: 7,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert_eq!(world.local_player_armor_value(), 0);
    }

    #[test]
    fn local_player_armor_value_floors_the_armor_attribute() {
        // Vanilla `getArmorValue()` = `Mth.floor(...)`, so 7.5 -> 7.
        let world = player_world_with_armor(Vec::new(), 7.5);
        assert_eq!(world.local_player_armor_value(), 7);

        // Exact integers pass through unchanged.
        let world = player_world_with_armor(Vec::new(), 20.0);
        assert_eq!(world.local_player_armor_value(), 20);
    }

    #[test]
    fn local_player_armor_value_folds_modifiers_through_calculate_value() {
        // `AttributeInstance.calculateValue`: base add (op 0), then multiply_base
        // (op 1), then multiply_total (op 2).
        //   base    = 4.0 + 2.0            = 6.0
        //   result  = 6.0 + 6.0 * 0.5      = 9.0
        //   result  = 9.0 * (1.0 + 0.1)    = 9.9
        //   floor(9.9)                     = 9
        let world = player_world_with_armor(
            vec![
                bbb_protocol::packets::AttributeModifier {
                    id: "test:add".to_string(),
                    amount: 2.0,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                },
                bbb_protocol::packets::AttributeModifier {
                    id: "test:multiply_base".to_string(),
                    amount: 0.5,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                },
                bbb_protocol::packets::AttributeModifier {
                    id: "test:multiply_total".to_string(),
                    amount: 0.1,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                },
            ],
            4.0,
        );
        assert_eq!(world.local_player_armor_value(), 9);
        assert_eq!(world.local_player_attribute_base_value(0), Some(4.0));
        assert_eq!(
            world.local_player_attribute_base_value(
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_SPEED_ID
            ),
            Some(4.0),
        );
        assert_eq!(
            world.local_player_attribute_base_value(
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID
            ),
            None,
        );
    }

    #[test]
    fn local_player_fall_flying_gate_composes_vanilla_state_and_glider_rules() {
        fn world_with_glider() -> WorldStore {
            let mut world = spawned_player_world();
            world.set_local_player_pose(LocalPlayerPoseState {
                position: vec3(0.5, 1.0, 0.5),
                on_ground: false,
                ..LocalPlayerPoseState::default()
            });
            world.set_default_item_component_type_ids(BTreeMap::from([(
                42,
                BTreeSet::from([
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                ]),
            )]));
            world.set_default_item_equipment_slots(BTreeMap::from([(
                42,
                crate::ItemEquipmentSlot::Chest,
            )]));
            world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
                slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
                item: ProtocolItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            });
            world
        }

        assert!(world_with_glider().local_player_can_start_fall_flying());

        let mut grounded = world_with_glider();
        let mut pose = grounded.local_player_pose().unwrap();
        pose.on_ground = true;
        grounded.set_local_player_pose(pose);
        assert!(!grounded.local_player_can_start_fall_flying());

        let mut creative_flying = world_with_glider();
        creative_flying.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert!(!creative_flying.local_player_can_start_fall_flying());

        let mut levitating = world_with_glider();
        assert!(
            levitating.apply_update_mob_effect(bbb_protocol::packets::UpdateMobEffect {
                entity_id: 7,
                effect_id: VANILLA_MOB_EFFECT_LEVITATION_ID,
                amplifier: 0,
                duration_ticks: 200,
                flags: bbb_protocol::packets::MobEffectFlags::default(),
            })
        );
        assert!(!levitating.local_player_can_start_fall_flying());

        let mut already_flying = world_with_glider();
        let accessors = entity_metadata_accessors();
        assert!(set_entity_data(
            &mut already_flying,
            7,
            accessors.shared_flags_data_id,
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            bbb_protocol::packets::EntityDataValueKind::Byte(accessors.fall_flying_mask),
        ));
        assert!(!already_flying.local_player_can_start_fall_flying());

        let mut passenger = world_with_glider();
        passenger.apply_add_entity(ProtocolAddEntity {
            id: 8,
            uuid: Uuid::from_u128(0x87654321876543218765432187654321),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(
            passenger.apply_set_passengers(bbb_protocol::packets::SetPassengers {
                vehicle_id: 8,
                passenger_ids: vec![7],
            })
        );
        assert!(!passenger.local_player_can_start_fall_flying());
    }

    fn spawned_player_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.local_player_id = Some(7);
        world.apply_add_entity(ProtocolAddEntity {
            id: 7,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world
    }

    fn set_entity_data(
        world: &mut WorldStore,
        entity_id: i32,
        data_id: u8,
        serializer_id: i32,
        value: bbb_protocol::packets::EntityDataValueKind,
    ) -> bool {
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: entity_id,
            values: vec![bbb_protocol::packets::EntityDataValue {
                data_id,
                serializer_id,
                value,
            }],
        })
    }

    fn entity_metadata_accessors() -> crate::VanillaEntityMetadataAccessors {
        crate::vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors")
    }

    #[test]
    fn local_player_air_supply_reads_the_synced_air_metadata() {
        // Vanilla `Entity.DATA_AIR_SUPPLY_ID` is a generated INT accessor with the virtual
        // define-time default `getMaxAirSupply()` = 300 (Entity.java:312,2725-2727).
        let accessors = entity_metadata_accessors();

        // No local player at all: the full (hidden-row) supply.
        let world = WorldStore::new();
        assert_eq!(world.local_player_air_supply(), 300);
        assert_eq!(world.local_player_max_air_supply(), 300);

        // A spawned player without a synced value still reads the 300 default.
        let mut world = spawned_player_world();
        assert_eq!(world.local_player_air_supply(), 300);

        assert!(set_entity_data(
            &mut world,
            7,
            accessors.air_supply_data_id,
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            bbb_protocol::packets::EntityDataValueKind::Int(150),
        ));
        assert_eq!(world.local_player_air_supply(), 150);
        assert_eq!(world.local_player_max_air_supply(), 300);
    }

    #[test]
    fn local_player_absorption_reads_the_synced_player_metadata() {
        // Vanilla `Player.DATA_PLAYER_ABSORPTION_ID` is a generated Player-only FLOAT accessor
        // with define-time default 0.0F (Player.java:134,224).
        let (absorption_data_id, absorption_default) =
            crate::vanilla_entity_metadata_player_absorption_accessor_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .expect("generated Player absorption accessor");
        assert_eq!(absorption_default, 0.0);

        let world = WorldStore::new();
        assert_eq!(world.local_player_absorption(), 0.0);

        let mut world = spawned_player_world();
        assert_eq!(world.local_player_absorption(), 0.0);

        assert!(set_entity_data(
            &mut world,
            7,
            absorption_data_id,
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            bbb_protocol::packets::EntityDataValueKind::Float(8.0),
        ));
        assert_eq!(world.local_player_absorption(), 8.0);

        // A non-Player using the same numeric slot must not project Player absorption.
        let mut world = WorldStore::new();
        world.local_player_id = Some(8);
        world.apply_add_entity(ProtocolAddEntity {
            id: 8,
            uuid: Uuid::from_u128(8),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(!set_entity_data(
            &mut world,
            8,
            absorption_data_id,
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            bbb_protocol::packets::EntityDataValueKind::Float(9.0),
        ));
        assert_eq!(world.local_player_absorption(), 0.0);
    }

    #[test]
    fn local_player_vehicle_health_projects_living_vehicles_only() {
        // Vanilla `LivingEntity.DATA_HEALTH_ID` is a generated inherited FLOAT accessor with
        // define-time default 1.0F (LivingEntity.java:179,314).
        let living_accessors =
            crate::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
            )
            .expect("generated LivingEntity metadata accessors");
        assert_eq!(living_accessors.health_default, 1.0);
        // `minecraft:max_health` uses the generated `Attributes.java` registration id.

        let add_vehicle = |world: &mut WorldStore, id: i32, entity_type_id: i32| {
            world.apply_add_entity(ProtocolAddEntity {
                id,
                uuid: Uuid::from_u128(0x2222_0000 + id as u128),
                entity_type_id,
                position: vec3(0.5, 1.0, 0.5),
                delta_movement: ProtocolVec3d::default(),
                x_rot: 0.0,
                y_rot: 0.0,
                y_head_rot: 0.0,
                data: 0,
            });
        };
        let mount = |world: &mut WorldStore, vehicle_id: i32| {
            assert!(
                world.apply_set_passengers(bbb_protocol::packets::SetPassengers {
                    vehicle_id,
                    passenger_ids: vec![7],
                })
            );
        };

        // No vehicle at all.
        let mut world = spawned_player_world();
        assert_eq!(world.local_player_vehicle_health(), None);

        // A living vehicle (horse) with synced health and MAX_HEALTH: both
        // project (vanilla `getPlayerVehicleWithHealth` + `getMaxHealth`).
        add_vehicle(
            &mut world,
            30,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
        );
        mount(&mut world, 30);
        assert!(set_entity_data(
            &mut world,
            30,
            living_accessors.health_data_id,
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            bbb_protocol::packets::EntityDataValueKind::Float(7.0),
        ));
        assert!(
            world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
                entity_id: 30,
                attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                    base: 15.0,
                    modifiers: Vec::new(),
                }],
            })
        );
        assert_eq!(
            world.local_player_vehicle_health(),
            Some(crate::LocalPlayerVehicleHealth {
                health: 7.0,
                max_health: 15.0,
            })
        );

        // Without a synced MAX_HEALTH attribute the max falls back to Pig's
        // `DefaultAttributes` family override 10.0, and without a synced
        // health the metadata default 1.0F applies.
        let mut world = spawned_player_world();
        add_vehicle(
            &mut world,
            31,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
        );
        mount(&mut world, 31);
        assert_eq!(
            world.local_player_vehicle_health(),
            Some(crate::LocalPlayerVehicleHealth {
                health: 1.0,
                max_health: 10.0,
            })
        );

        // A non-living vehicle (boat) is vanilla `showVehicleHealth() == false`
        // (`Entity` base: `this instanceof LivingEntity`, Entity.java:2349-2351).
        let mut world = spawned_player_world();
        add_vehicle(
            &mut world,
            32,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        );
        mount(&mut world, 32);
        assert_eq!(world.local_player_vehicle_health(), None);
    }

    #[test]
    fn local_player_max_health_reads_the_synced_attribute() {
        // `minecraft:max_health` uses the generated `Attributes.java` registration id; the
        // `RangedAttribute` default is 20.0 (Attributes.java:58-60).

        // No local player / no synced attribute: the RangedAttribute default.
        assert_eq!(WorldStore::new().local_player_max_health(), 20.0);
        let mut world = spawned_player_world();
        assert_eq!(world.local_player_max_health(), 20.0);

        // A Health Boost / attribute bump folds through `calculateValue`.
        assert!(
            world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
                entity_id: 7,
                attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                    attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                    base: 30.0,
                    modifiers: Vec::new(),
                }],
            })
        );
        assert_eq!(world.local_player_max_health(), 30.0);
    }

    #[test]
    fn local_player_is_fully_frozen_tracks_the_freeze_threshold() {
        // Vanilla `isFullyFrozen()` = `getTicksFrozen() >= 140`.
        // `DATA_TICKS_FROZEN_ID` is a generated base Entity INT accessor.
        let accessors = entity_metadata_accessors();
        assert_eq!(accessors.ticks_frozen_default, 0);

        assert!(!WorldStore::new().local_player_is_fully_frozen());
        let mut world = spawned_player_world();
        assert!(!world.local_player_is_fully_frozen());

        // One tick short of the threshold: not yet fully frozen.
        assert!(set_entity_data(
            &mut world,
            7,
            accessors.ticks_frozen_data_id,
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            bbb_protocol::packets::EntityDataValueKind::Int(139),
        ));
        assert!(!world.local_player_is_fully_frozen());

        // At the 140-tick threshold: fully frozen.
        assert!(set_entity_data(
            &mut world,
            7,
            accessors.ticks_frozen_data_id,
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            bbb_protocol::packets::EntityDataValueKind::Int(140),
        ));
        assert!(world.local_player_is_fully_frozen());
    }
}
