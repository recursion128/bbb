use bbb_protocol::packets::{
    DataComponentPatchSummary, Direction as ProtocolDirection, EntityAnchor,
    ItemStackSummary as ProtocolItemStackSummary, PlayerAbilities as ProtocolPlayerAbilities,
    PlayerExperience as ProtocolPlayerExperience, PlayerHealth as ProtocolPlayerHealth,
    PlayerLookAt as ProtocolPlayerLookAt, PlayerPositionState as ProtocolPlayerPositionState,
    PlayerPositionUpdate as ProtocolPlayerPositionUpdate,
    PlayerRotationUpdate as ProtocolPlayerRotationUpdate, SetCamera as ProtocolSetCamera,
    SetDefaultSpawnPosition as ProtocolSetDefaultSpawnPosition, SetHeldSlot as ProtocolSetHeldSlot,
    SetSimulationDistance as ProtocolSetSimulationDistance, Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use super::local_player_movement::{
    apply_local_player_input_look, integrate_local_player_input_pose,
};
use crate::{protocol_block_pos, BlockPos, EntityVec3, WorldStore};

const STANDING_EYE_HEIGHT: f64 = 1.62;

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
    pub default_spawn: Option<DefaultSpawnState>,
    #[serde(default)]
    pub simulation_distance: Option<i32>,
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
            default_spawn: None,
            simulation_distance: None,
            camera: CameraState::default(),
            pose: None,
            last_look_at: None,
            interaction: LocalPlayerInteractionState::default(),
        }
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
    pub prediction_sequence: i32,
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

    pub fn from_position_state(state: ProtocolPlayerPositionState, last_teleport_id: i32) -> Self {
        Self {
            position: state.position,
            delta_movement: state.delta_movement,
            on_ground: false,
            horizontal_collision: false,
            fall_distance: 0.0,
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
        if self.local_player_vehicle_id.is_some() {
            return None;
        }
        let current = self
            .local_player
            .pose
            .map(LocalPlayerPoseState::position_state)
            .unwrap_or_default();
        let state = packet.apply_to_state(current);
        let pose = LocalPlayerPoseState::from_position_state(state, packet.id);
        self.local_player.pose = Some(pose);
        Some(pose)
    }

    pub fn apply_player_rotation(
        &mut self,
        packet: ProtocolPlayerRotationUpdate,
    ) -> LocalPlayerPoseState {
        self.counters.player_rotation_packets += 1;
        let current_pose = self.local_player.pose.unwrap_or_default();
        let state = packet.apply_to_state(current_pose.position_state());
        let pose = LocalPlayerPoseState::from_position_state(state, current_pose.last_teleport_id);
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
    }

    pub fn advance_local_player_input(
        &mut self,
        input: LocalPlayerInputState,
        dt_seconds: f64,
    ) -> Option<LocalPlayerPoseState> {
        let pose =
            integrate_local_player_input_pose(self, self.local_player.pose?, input, dt_seconds);
        self.local_player.pose = Some(pose);
        Some(pose)
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
        self.local_player.interaction.using_item = using_item;
    }

    pub fn take_local_using_item(&mut self) -> bool {
        let using_item = self.local_player.interaction.using_item;
        self.local_player.interaction.using_item = false;
        using_item
    }

    fn resolve_look_at_target_position(&self, packet: ProtocolPlayerLookAt) -> ProtocolVec3d {
        packet
            .target
            .and_then(|target| {
                self.probe_entity(target.entity_id)
                    .map(|entity| entity_anchor_position(entity.position, target.to_anchor))
            })
            .unwrap_or(packet.position)
    }

    fn local_selected_hotbar_item_signature(&self) -> LocalPlayerDestroyItemSignature {
        let hotbar_items = self.inventory.hotbar_items();
        let selected_slot = usize::from(self.local_player.selected_hotbar_slot.min(8));
        LocalPlayerDestroyItemSignature::from_item_stack(&hotbar_items[selected_slot])
    }
}

fn apply_look_at_to_pose(
    pose: LocalPlayerPoseState,
    from_anchor: EntityAnchor,
    target_position: ProtocolVec3d,
) -> LocalPlayerPoseState {
    let from_y = match from_anchor {
        EntityAnchor::Feet => pose.position.y,
        EntityAnchor::Eyes => pose.position.y + STANDING_EYE_HEIGHT,
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

fn entity_anchor_position(position: EntityVec3, anchor: EntityAnchor) -> ProtocolVec3d {
    ProtocolVec3d {
        x: position.x,
        y: position.y
            + match anchor {
                EntityAnchor::Feet => 0.0,
                EntityAnchor::Eyes => STANDING_EYE_HEIGHT,
            },
        z: position.z,
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
    use super::super::local_player_movement::LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND;
    use super::*;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, PlayerLookAtTarget, Vec3d as ProtocolVec3d,
        PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
    };
    use uuid::Uuid;

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
        assert!(store.take_local_using_item());
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
                prediction_sequence: 1,
            }
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
        assert_eq!(pose.last_teleport_id, 8);
        assert_eq!(store.local_player_pose(), Some(pose));

        let counters = store.counters();
        assert_eq!(counters.player_position_packets, 2);
        assert_eq!(counters.player_rotation_packets, 1);
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

    fn protocol_add_entity_at(id: i32, position: ProtocolVec3d) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: 7,
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
}
