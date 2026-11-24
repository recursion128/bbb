use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_LEVITATION_ID;
use bbb_protocol::packets::{
    DataComponentPatchSummary, Direction as ProtocolDirection, EntityAnchor,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate, InteractionHand,
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

use super::local_player_collision::local_player_inside_nether_portal;
use super::local_player_fluid::{local_player_eye_in_water, local_player_fluid_contact};
use super::local_player_movement::{
    advance_local_player_auto_spin_attack_after_move, apply_local_player_input_look,
    integrate_local_player_input_pose_with_world_effects, local_player_can_start_sprinting,
    local_player_is_on_climbable, local_player_should_stop_sprinting,
};
use crate::{BlockPos, WorldStore};

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
    /// Vanilla `LocalPlayer.portalEffectIntensity`: the CONFUSION-portal
    /// screen-effect ramp advanced once per client tick.
    #[serde(default)]
    pub portal_effect_intensity: f32,
    /// Vanilla `LocalPlayer.oPortalEffectIntensity` for frame interpolation.
    #[serde(default)]
    pub previous_portal_effect_intensity: f32,
    #[serde(default)]
    pub camera: CameraState,
    #[serde(default)]
    pub pose: Option<LocalPlayerPoseState>,
    /// Client-owned `Entity` sprinting shared flag for the local player.
    /// Raw keyboard state and the last outbound command baseline deliberately
    /// live outside the canonical world state.
    #[serde(default)]
    pub sprinting: bool,
    /// `Entity.updateSwimming`'s swimming flag. The swim body pose also
    /// covers `Player.updatePlayerPose`'s crawl fallback, so it is not this.
    #[serde(skip)]
    pub(crate) swimming: bool,
    /// `LocalPlayer.isShiftKeyDown`: the sneak key, not the crouch pose.
    #[serde(skip)]
    pub(crate) shift_key_down: bool,
    /// Process-configured `Options.autoJump` and LocalPlayer's private
    /// one-tick `autoJumpTime`. Neither is server-owned snapshot state.
    #[serde(skip)]
    pub(crate) auto_jump: LocalPlayerAutoJumpState,
    #[serde(skip)]
    pub(crate) movement_sound: LocalPlayerMovementSoundState,
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
            portal_effect_intensity: 0.0,
            previous_portal_effect_intensity: 0.0,
            camera: CameraState::default(),
            pose: None,
            sprinting: false,
            swimming: false,
            shift_key_down: false,
            auto_jump: LocalPlayerAutoJumpState::default(),
            movement_sound: LocalPlayerMovementSoundState::default(),
            last_look_at: None,
            interaction: LocalPlayerInteractionState::default(),
        }
    }
}

// Entity.moveDist and nextStep are independent of avatar/view-bob walk distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LocalPlayerMovementSoundState {
    pub(crate) distance: f32,
    pub(crate) next_step: f32,
    pub(crate) tick_count: i64,
    pub(crate) last_crystal_sound_tick: i64,
    pub(crate) crystal_sound_intensity: f32,
    pub(crate) was_touching_water: bool,
    pub(crate) water_state_initialized: bool,
}

impl Default for LocalPlayerMovementSoundState {
    fn default() -> Self {
        Self {
            distance: 0.0,
            next_step: 1.0,
            tick_count: 0,
            last_crystal_sound_tick: -20,
            crystal_sound_intensity: 0.0,
            was_touching_water: false,
            water_state_initialized: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalPlayerAutoJumpState {
    pub(crate) enabled: bool,
    pub(crate) ticks_remaining: u8,
}

impl Default for LocalPlayerAutoJumpState {
    fn default() -> Self {
        Self {
            enabled: true,
            ticks_remaining: 0,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPlayerDestroyItemSignature {
    pub item_id: Option<i32>,
    pub component_patch: DataComponentPatchSummary,
}

impl LocalPlayerDestroyItemSignature {
    pub(super) fn from_item_stack(stack: &ProtocolItemStackSummary) -> Self {
        Self {
            item_id: stack.item_id,
            component_patch: stack.component_patch.clone().into_inner(),
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
    pub minor_horizontal_collision: bool,
    #[serde(default)]
    pub fall_distance: f64,
    #[serde(default)]
    pub sneaking: bool,
    #[serde(default)]
    pub swimming: bool,
    #[serde(default)]
    pub no_jump_delay_ticks: u8,
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
    /// Canonical LocalPlayer sprint latch, not the raw sprint key bit carried
    /// by `ServerboundPlayerInputPacket`.
    pub sprinting: bool,
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

    pub fn from_position_state(state: ProtocolPlayerPositionState, last_teleport_id: i32) -> Self {
        Self {
            position: state.position,
            delta_movement: state.delta_movement,
            on_ground: false,
            horizontal_collision: false,
            minor_horizontal_collision: false,
            fall_distance: 0.0,
            sneaking: false,
            swimming: false,
            no_jump_delay_ticks: 0,
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
    /// Vanilla `Minecraft.getCameraEntity()`: the `Camera`'s entity, which
    /// `Camera.update` initializes to `minecraft.player` (Camera.java:94-97)
    /// and `ClientboundSetCameraPacket` retargets.
    pub fn camera_entity_id(&self) -> Option<i32> {
        if self.local_player.camera.follows_player {
            return self.local_player_id();
        }
        self.local_player.camera.entity_id
    }

    /// Vanilla `Gui.getCameraPlayer()` (Gui.java:705-707): the camera entity
    /// when it is a `Player`.
    pub fn hud_camera_player_id(&self) -> Option<i32> {
        let camera_id = self.camera_entity_id()?;
        (Some(camera_id) == self.local_player_id()
            || self.entity_type_id(camera_id)
                == Some(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID))
        .then_some(camera_id)
    }

    /// Whether `Gui.getCameraPlayer()` resolves to the local player, i.e. the
    /// camera player's `Inventory` is the client-owned one.
    pub fn hud_camera_player_is_local_player(&self) -> bool {
        self.local_player.camera.follows_player
    }

    /// Synchronizes vanilla `options.keySpectatorOutlines.isDown()` from the
    /// input runtime for `Minecraft.shouldEntityAppearGlowing`.
    pub fn set_spectator_outlines_key_down(&mut self, down: bool) {
        self.spectator_outlines_key_down = down;
    }

    pub fn set_local_player_auto_jump_enabled(&mut self, enabled: bool) {
        self.local_player.auto_jump.enabled = enabled;
    }

    pub fn local_player_auto_jump_enabled(&self) -> bool {
        self.local_player.auto_jump.enabled
    }

    pub(crate) fn consume_local_player_auto_jump_tick(&mut self) -> bool {
        if self.local_player.auto_jump.ticks_remaining == 0 {
            return false;
        }
        self.local_player.auto_jump.ticks_remaining -= 1;
        true
    }

    pub(crate) fn arm_local_player_auto_jump(&mut self) {
        self.local_player.auto_jump.ticks_remaining = 1;
    }

    pub(crate) fn local_player_auto_jump_is_armed(&self) -> bool {
        self.local_player.auto_jump.ticks_remaining > 0
    }

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
        self.push_screen_request(crate::ScreenRequest::ReconcileCreativeInventory);
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
        let auto_jump_enabled = self.local_player_auto_jump_enabled();
        self.local_player = LocalPlayerState::default();
        self.set_local_player_auto_jump_enabled(auto_jump_enabled);
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
            self.push_screen_request(crate::ScreenRequest::CloseGameRules);
        }
        self.push_screen_request(crate::ScreenRequest::RefreshCreativeInventory);
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
        // Guard against non-finite or absurd server health values (e.g. 1e9)
        // that would overflow HUD heart allocation/arithmetic.
        const MAX_SAFE_CLIENT_HEALTH: f32 = 10_000.0;
        let health = if packet.health.is_finite() {
            packet.health.min(MAX_SAFE_CLIENT_HEALTH)
        } else {
            0.0
        };
        let saturation = if packet.saturation.is_finite() {
            packet.saturation.clamp(0.0, MAX_SAFE_CLIENT_HEALTH)
        } else {
            0.0
        };
        self.local_player.health = Some(LocalPlayerHealthState {
            health,
            food: packet.food,
            saturation,
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
        true
    }

    pub fn set_local_selected_hotbar_slot(&mut self, slot: u8) -> bool {
        if slot > 8 {
            return false;
        }
        self.local_player.selected_hotbar_slot = slot;
        true
    }

    pub fn apply_default_spawn_position(&mut self, packet: ProtocolSetDefaultSpawnPosition) {
        self.counters.default_spawn_position_packets += 1;
        // Vanilla `ClientLevel.setRespawnData` routes through
        // `Level.getWorldBorderAdjustedRespawnData`: an out-of-border spawn
        // falls back to the MOTION_BLOCKING heightmap position at the border
        // center while keeping dimension/yaw/pitch; an unloaded center chunk
        // reads `Level.getHeight`'s minY fallback.
        let packet_pos = packet.pos;
        let pos = if self.world_border().contains_block_pos(packet_pos) {
            packet_pos
        } else {
            let border = self.world_border();
            let x = border.center_x.floor() as i32;
            let z = border.center_z.floor() as i32;
            let y = self
                .sample_motion_blocking_height(x, z)
                .unwrap_or(self.dimension.min_y);
            BlockPos { x, y, z }
        };
        self.local_player.default_spawn = Some(DefaultSpawnState {
            dimension: packet.dimension,
            pos,
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
            let mut state = packet.apply_to_state(current);
            sanitize_local_player_position_state(&mut state, current);
            let pose = LocalPlayerPoseState {
                sneaking: current_pose.sneaking,
                swimming: current_pose.swimming,
                ..LocalPlayerPoseState::from_position_state(state, packet.id)
            };
            if let Some(old) = &mut self.local_player_old_pose {
                let mut corrected = packet.apply_to_state(old.position_state());
                sanitize_local_player_position_state(&mut corrected, old.position_state());
                old.position = corrected.position;
                old.y_rot = corrected.y_rot;
                old.x_rot = corrected.x_rot;
            }
            self.local_player.pose = Some(pose);
            Some(pose)
        };
        self.mark_local_prediction_teleport();
        applied_pose
    }

    pub fn apply_player_rotation(
        &mut self,
        packet: ProtocolPlayerRotationUpdate,
    ) -> LocalPlayerPoseState {
        self.counters.player_rotation_packets += 1;
        let current_pose = self.local_player.pose.unwrap_or_default();
        let state = packet.apply_to_state(current_pose.position_state());
        let mut pose = current_pose;
        if state.y_rot.is_finite() {
            pose.y_rot = state.y_rot;
        }
        if state.x_rot.is_finite() {
            pose.x_rot = (state.x_rot % 360.0).clamp(-90.0, 90.0);
        }
        if let Some(old) = &mut self.local_player_old_pose {
            old.y_rot = pose.y_rot;
            old.x_rot = pose.x_rot;
        }
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

        let pose = apply_look_at_to_pose(
            self,
            self.local_player.pose?,
            packet.from_anchor,
            target_position,
        );
        self.local_player.pose = Some(pose);
        Some(pose)
    }

    pub fn set_local_player_pose(&mut self, pose: LocalPlayerPoseState) {
        self.local_player.pose = Some(pose);
        self.local_player_old_pose = Some(pose);
    }

    pub fn look_angles_toward(&self, target_position: ProtocolVec3d) -> Option<(f32, f32)> {
        let pose = apply_look_at_to_pose(
            self,
            self.local_player.pose?,
            EntityAnchor::Eyes,
            target_position,
        );
        Some((pose.y_rot, pose.x_rot.clamp(-90.0, 90.0)))
    }

    pub fn entity_look_target_position(
        &self,
        entity_id: i32,
        anchor: EntityAnchor,
    ) -> Option<ProtocolVec3d> {
        match anchor {
            EntityAnchor::Feet => self.probe_entity(entity_id).map(|entity| entity.position),
            EntityAnchor::Eyes => {
                self.probe_entity_camera_pose(entity_id)
                    .map(|pose| ProtocolVec3d {
                        x: pose.position.x,
                        y: pose.position.y + f64::from(pose.eye_height),
                        z: pose.position.z,
                    })
            }
        }
    }

    pub fn entity_look_center_position(&self, entity_id: i32) -> Option<ProtocolVec3d> {
        let entity = self.probe_entity(entity_id)?;
        if let Some(bounds) = self.probe_entity_pick_bounds(entity_id) {
            return Some(ProtocolVec3d {
                x: entity.position.x + f64::from(bounds.min[0] + bounds.max[0]) * 0.5,
                y: entity.position.y + f64::from(bounds.min[1] + bounds.max[1]) * 0.5,
                z: entity.position.z + f64::from(bounds.min[2] + bounds.max[2]) * 0.5,
            });
        }
        let eye_height = self
            .probe_entity_camera_pose(entity_id)
            .map(|pose| f64::from(pose.eye_height))
            .unwrap_or(0.0);
        Some(ProtocolVec3d {
            x: entity.position.x,
            y: entity.position.y + eye_height * 0.5,
            z: entity.position.z,
        })
    }

    pub fn set_local_player_look_angles(
        &mut self,
        yaw: f32,
        pitch: f32,
    ) -> Option<LocalPlayerPoseState> {
        let mut pose = self.local_player.pose?;
        pose.y_rot = wrap_degrees_f32(yaw);
        pose.x_rot = pitch.clamp(-90.0, 90.0);
        if let Some(old) = &mut self.local_player_old_pose {
            old.y_rot = pose.y_rot;
            old.x_rot = pose.x_rot;
        }
        self.local_player.pose = Some(pose);
        Some(pose)
    }

    pub fn advance_local_player_input(
        &mut self,
        input: LocalPlayerInputState,
        dt_seconds: f64,
    ) -> Option<LocalPlayerPoseState> {
        let old_pose = self.local_player.pose?;
        self.local_player_old_pose = Some(old_pose);
        self.local_player.shift_key_down = input.focused && input.sneak;
        let pose =
            integrate_local_player_input_pose_with_world_effects(self, old_pose, input, dt_seconds);
        let pose = advance_local_player_auto_spin_attack_after_move(self, old_pose, pose);
        self.local_player.pose = Some(pose);
        if let Some(id) = self.local_player_id {
            let _ = self.entities.advance_local_player_walk_animation(
                id,
                old_pose.position,
                pose.position,
            );
        }
        Some(pose)
    }

    fn resolve_look_at_target_position(&self, packet: ProtocolPlayerLookAt) -> ProtocolVec3d {
        packet
            .target
            .and_then(|target| match target.to_anchor {
                EntityAnchor::Feet => self
                    .probe_entity(target.entity_id)
                    .map(|entity| entity.position),
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
}

/// Projects `GameType.updatePlayerAbilities` into the client's ability state.
///
/// The vanilla rule itself is owned by `PlayerAbilities::for_game_type` in
/// `bbb-protocol`, because the same projection runs on the server
/// (`ServerPlayerGameMode.setGameModeForPlayer`) and on the client
/// (`MultiPlayerGameMode`); this only re-shapes it into the local-player state.
pub(crate) fn game_type_player_abilities(game_type: GameType) -> LocalPlayerAbilitiesState {
    let abilities = ProtocolPlayerAbilities::for_game_type(game_type);
    LocalPlayerAbilitiesState {
        invulnerable: abilities.invulnerable,
        flying: abilities.flying,
        can_fly: abilities.can_fly,
        instabuild: abilities.instabuild,
        flying_speed: abilities.flying_speed,
        walking_speed: abilities.walking_speed,
    }
}

fn apply_look_at_to_pose(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    from_anchor: EntityAnchor,
    target_position: ProtocolVec3d,
) -> LocalPlayerPoseState {
    let from_y = match from_anchor {
        EntityAnchor::Feet => pose.position.y,
        EntityAnchor::Eyes => pose.position.y + world.local_player_eye_height(pose),
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

fn sanitize_local_player_position_state(
    state: &mut bbb_protocol::packets::PlayerPositionState,
    previous: bbb_protocol::packets::PlayerPositionState,
) {
    const MAX_HORIZONTAL_COORD: f64 = 3.0e7;
    const MAX_VERTICAL_COORD: f64 = 2.0e7;
    const MAX_VELOCITY_COMPONENT: f64 = 10_000.0;

    state.position.x = if state.position.x.is_finite() {
        state
            .position
            .x
            .clamp(-MAX_HORIZONTAL_COORD, MAX_HORIZONTAL_COORD)
    } else {
        previous
            .position
            .x
            .clamp(-MAX_HORIZONTAL_COORD, MAX_HORIZONTAL_COORD)
    };
    state.position.y = if state.position.y.is_finite() {
        state
            .position
            .y
            .clamp(-MAX_VERTICAL_COORD, MAX_VERTICAL_COORD)
    } else {
        previous
            .position
            .y
            .clamp(-MAX_VERTICAL_COORD, MAX_VERTICAL_COORD)
    };
    state.position.z = if state.position.z.is_finite() {
        state
            .position
            .z
            .clamp(-MAX_HORIZONTAL_COORD, MAX_HORIZONTAL_COORD)
    } else {
        previous
            .position
            .z
            .clamp(-MAX_HORIZONTAL_COORD, MAX_HORIZONTAL_COORD)
    };
    state.delta_movement.x = if state.delta_movement.x.is_finite() {
        state
            .delta_movement
            .x
            .clamp(-MAX_VELOCITY_COMPONENT, MAX_VELOCITY_COMPONENT)
    } else {
        0.0
    };
    state.delta_movement.y = if state.delta_movement.y.is_finite() {
        state
            .delta_movement
            .y
            .clamp(-MAX_VELOCITY_COMPONENT, MAX_VELOCITY_COMPONENT)
    } else {
        0.0
    };
    state.delta_movement.z = if state.delta_movement.z.is_finite() {
        state
            .delta_movement
            .z
            .clamp(-MAX_VELOCITY_COMPONENT, MAX_VELOCITY_COMPONENT)
    } else {
        0.0
    };
    if !state.y_rot.is_finite() {
        state.y_rot = previous.y_rot;
    }
    state.x_rot = if state.x_rot.is_finite() {
        (state.x_rot % 360.0).clamp(-90.0, 90.0)
    } else {
        previous.x_rot.clamp(-90.0, 90.0)
    };
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

mod combat_and_interaction;

#[cfg(test)]
mod tests;
