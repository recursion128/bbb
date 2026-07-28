use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID,
    VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
};
use bbb_protocol::entity_types::vanilla_entity_type_is_redirectable_projectile;
use bbb_protocol::packets::{
    PlayerCombatEnd as ProtocolPlayerCombatEnd, PlayerCombatKill as ProtocolPlayerCombatKill,
};
use bbb_protocol::RichTextComponent;
use serde::{Deserialize, Serialize};

use crate::entities::{
    calculate_view_vector, vanilla_entity_metadata_type_accepts_local_attack_hurt,
    vanilla_entity_metadata_type_is_projectile,
};
use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCombatState {
    #[serde(default)]
    pub last_combat: Option<PlayerCombatEventState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatEventState {
    pub kind: String,
    pub duration: Option<i32>,
    pub player_id: Option<i32>,
    pub message: Option<RichTextComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDeathScreenState {
    pub generation: u64,
    pub cause_of_death: RichTextComponent,
    pub score: i32,
    pub hardcore: bool,
}

/// Canonical world inputs for Vanilla's crosshair attack indicator.
///
/// Target range is intentionally absent: native's canonical crosshair pick
/// already applies the active item's `ATTACK_RANGE` before it supplies
/// `target_entity_id`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPlayerCrosshairAttackIndicatorState {
    pub strength_scale: f32,
    pub full: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCombatKillApplyOutcome {
    Ignored,
    DeathScreenOpened,
    PerformRespawn,
}

impl WorldStore {
    pub fn apply_player_combat_enter(&mut self) {
        self.counters.player_combat_enter_packets += 1;
        self.client_combat.last_combat = Some(PlayerCombatEventState {
            kind: "enter".to_string(),
            duration: None,
            player_id: None,
            message: None,
        });
    }

    pub fn apply_player_combat_end(&mut self, packet: ProtocolPlayerCombatEnd) {
        self.counters.player_combat_end_packets += 1;
        self.client_combat.last_combat = Some(PlayerCombatEventState {
            kind: "end".to_string(),
            duration: Some(packet.duration),
            player_id: None,
            message: None,
        });
    }

    pub fn apply_player_combat_kill(
        &mut self,
        packet: ProtocolPlayerCombatKill,
    ) -> PlayerCombatKillApplyOutcome {
        self.counters.player_combat_kill_packets += 1;
        if !self.preflight_packet_components(&packet, "player combat kill") {
            return PlayerCombatKillApplyOutcome::Ignored;
        }
        self.client_combat.last_combat = Some(PlayerCombatEventState {
            kind: "kill".to_string(),
            duration: None,
            player_id: Some(packet.player_id),
            message: Some(packet.message.clone()),
        });

        if self.local_player_id() != Some(packet.player_id) {
            return PlayerCombatKillApplyOutcome::Ignored;
        }
        if !self.gameplay.show_death_screen {
            return PlayerCombatKillApplyOutcome::PerformRespawn;
        }

        let score = self.local_player_score();
        let hardcore = self.is_hardcore();
        self.client_ui.replace_screen();
        self.client_ui.death_screen_generation =
            self.client_ui.death_screen_generation.saturating_add(1);
        self.client_ui.current_death_screen = Some(ClientDeathScreenState {
            generation: self.client_ui.death_screen_generation,
            cause_of_death: packet.message,
            score,
            hardcore,
        });
        PlayerCombatKillApplyOutcome::DeathScreenOpened
    }

    pub fn client_combat(&self) -> &ClientCombatState {
        &self.client_combat
    }

    pub fn last_player_combat(&self) -> Option<&PlayerCombatEventState> {
        self.client_combat.last_combat.as_ref()
    }

    pub fn death_screen(&self) -> Option<&ClientDeathScreenState> {
        self.client_ui.current_death_screen.as_ref()
    }

    pub fn close_death_screen(&mut self, generation: u64) -> bool {
        if self
            .client_ui
            .current_death_screen
            .as_ref()
            .is_none_or(|screen| screen.generation != generation)
        {
            return false;
        }
        self.client_ui.current_death_screen = None;
        true
    }

    pub fn clear_death_screen(&mut self) -> bool {
        self.client_ui.current_death_screen.take().is_some()
    }

    /// Applies the client-owned state mutations from Vanilla `Player.attack`.
    ///
    /// The server remains authoritative for damage, enchantments, critical and
    /// sweep effects, sounds, stats, item damage, and target removal. The return
    /// value reports whether `causeExtraKnockback` called `setSprinting(false)`;
    /// the native input owner uses it to keep command ordering outside world.
    pub fn apply_local_player_attack_prediction(
        &mut self,
        target_id: i32,
        sprinting: bool,
    ) -> bool {
        let Some(target_type_id) = self.entity_type_id(target_id) else {
            return false;
        };
        let Some(mut player_pose) = self.local_player.pose else {
            return false;
        };

        if vanilla_entity_type_is_redirectable_projectile(target_type_id)
            && vanilla_entity_metadata_type_is_projectile(target_type_id)
        {
            let look = calculate_view_vector(player_pose.x_rot, player_pose.y_rot);
            let _ = self
                .entities
                .with_transform_mut(target_id, |target| target.delta_movement = look);
            return false;
        }

        if !vanilla_entity_metadata_type_accepts_local_attack_hurt(target_type_id) {
            return false;
        }

        // LocalPlayer inherits Player.getEnchantedDamage, which returns the
        // input damage unchanged. Only ServerPlayer applies enchantments.
        let Some(base_damage) =
            self.local_player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
        else {
            return false;
        };
        if !base_damage.is_finite() || base_damage <= 0.0 {
            return false;
        }

        let attack_strength = self.local_player_attack_strength_scale(0.5);
        let sprint_knockback = if sprinting && attack_strength > 0.9 {
            0.5_f32
        } else {
            0.0
        };
        let attribute_knockback = self
            .local_player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID)
            .filter(|value| value.is_finite())
            .unwrap_or(0.0) as f32
            / 2.0;
        let knockback = attribute_knockback + sprint_knockback;
        if knockback <= 0.0 {
            return false;
        }

        let push_direction = calculate_view_vector(0.0, player_pose.y_rot);
        let push_x = (push_direction.x as f32 * knockback) as f64;
        let push_z = (push_direction.z as f32 * knockback) as f64;
        let pushed = self
            .entities
            .with_transform_mut(target_id, |target| {
                target.delta_movement.x += push_x;
                target.delta_movement.y += 0.1;
                target.delta_movement.z += push_z;
            })
            .is_some();
        if !pushed {
            return false;
        }

        player_pose.delta_movement.x *= 0.6;
        player_pose.delta_movement.z *= 0.6;
        self.local_player.pose = Some(player_pose);
        true
    }

    pub fn local_player_crosshair_attack_indicator_state(
        &self,
        target_entity_id: Option<i32>,
    ) -> Option<LocalPlayerCrosshairAttackIndicatorState> {
        let delay = self.local_player_attack_strength_delay()?;
        let strength_scale =
            ((self.local_player_attack_strength_ticks() as f32) / delay).clamp(0.0, 1.0);
        let full = strength_scale >= 1.0
            && delay > 5.0
            && target_entity_id.is_some_and(|entity_id| {
                self.entity_type_id(entity_id)
                    .is_some_and(crate::vanilla_entity_type_is_living)
                    && self
                        .entities
                        .living_entity_health(entity_id)
                        .is_some_and(|health| health > 0.0)
            });
        Some(LocalPlayerCrosshairAttackIndicatorState {
            strength_scale,
            full,
        })
    }

    fn local_player_attack_strength_delay(&self) -> Option<f32> {
        let Some(attack_speed) =
            self.local_player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_SPEED_ID)
        else {
            return None;
        };
        if !attack_speed.is_finite() || attack_speed <= 0.0 {
            return None;
        }
        Some((1.0 / attack_speed * 20.0) as f32)
    }

    fn local_player_attack_strength_scale(&self, partial_tick: f32) -> f32 {
        let Some(delay) = self.local_player_attack_strength_delay() else {
            return 0.0;
        };
        ((self.local_player_attack_strength_ticks() as f32 + partial_tick) / delay).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue, EntityDataValueKind, SetEntityData,
        Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    #[test]
    fn tracks_last_player_combat_event_and_counters() {
        let mut store = WorldStore::new();

        store.apply_player_combat_enter();
        assert_eq!(
            store.last_player_combat(),
            Some(&PlayerCombatEventState {
                kind: "enter".to_string(),
                duration: None,
                player_id: None,
                message: None,
            })
        );

        store.apply_player_combat_end(ProtocolPlayerCombatEnd { duration: 37 });
        assert_eq!(
            store.last_player_combat(),
            Some(&PlayerCombatEventState {
                kind: "end".to_string(),
                duration: Some(37),
                player_id: None,
                message: None,
            })
        );

        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 123,
            message: RichTextComponent::plain("You died"),
        });
        assert_eq!(
            store.last_player_combat(),
            Some(&PlayerCombatEventState {
                kind: "kill".to_string(),
                duration: None,
                player_id: Some(123),
                message: Some(RichTextComponent::plain("You died")),
            })
        );

        let counters = store.counters();
        assert_eq!(counters.player_combat_enter_packets, 1);
        assert_eq!(counters.player_combat_end_packets, 1);
        assert_eq!(counters.player_combat_kill_packets, 1);
    }

    #[test]
    fn player_combat_kill_only_opens_death_screen_for_local_player() {
        let mut store = local_player_world(7);
        store.gameplay.hardcore = true;

        assert_eq!(
            store.apply_player_combat_kill(ProtocolPlayerCombatKill {
                player_id: 8,
                message: RichTextComponent::plain("Remote death"),
            }),
            PlayerCombatKillApplyOutcome::Ignored
        );
        assert_eq!(store.death_screen(), None);

        assert_eq!(
            store.apply_player_combat_kill(ProtocolPlayerCombatKill {
                player_id: 7,
                message: RichTextComponent::plain("Local death"),
            }),
            PlayerCombatKillApplyOutcome::DeathScreenOpened
        );
        let screen = store.death_screen().expect("local death screen");
        assert_eq!(screen.generation, 1);
        assert_eq!(screen.cause_of_death.text(), "Local death");
        assert_eq!(screen.score, 42);
        assert!(screen.hardcore);
    }

    #[test]
    fn player_combat_kill_requests_immediate_respawn_when_screen_is_disabled() {
        let mut store = local_player_world(7);
        store.gameplay.show_death_screen = false;

        assert_eq!(
            store.apply_player_combat_kill(ProtocolPlayerCombatKill {
                player_id: 7,
                message: RichTextComponent::plain("Local death"),
            }),
            PlayerCombatKillApplyOutcome::PerformRespawn
        );
        assert_eq!(store.death_screen(), None);
    }

    #[test]
    fn death_screen_generation_guards_close_and_replacement() {
        let mut store = local_player_world(7);
        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: RichTextComponent::plain("First"),
        });
        let first_generation = store.death_screen().unwrap().generation;
        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: RichTextComponent::plain("Second"),
        });
        let second_generation = store.death_screen().unwrap().generation;

        assert!(second_generation > first_generation);
        assert!(!store.close_death_screen(first_generation));
        assert!(store.close_death_screen(second_generation));
        assert_eq!(store.death_screen(), None);
        assert!(!store.clear_death_screen());
    }

    #[test]
    fn death_screen_participates_in_central_screen_replacement() {
        let mut store = local_player_world(7);
        store.open_game_event_screen(|generation| crate::ClientGameEventScreenState::Win {
            generation,
        });
        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: RichTextComponent::plain("Death replaces win"),
        });
        assert_eq!(store.game_event_screen(), None);
        assert!(store.death_screen().is_some());

        store.open_game_event_screen(|generation| crate::ClientGameEventScreenState::Win {
            generation,
        });
        assert_eq!(store.death_screen(), None);
        assert!(store.game_event_screen().is_some());
    }

    #[test]
    fn local_attack_aim_deflects_redirectable_projectile_without_player_knockback() {
        let mut store = local_player_world(7);
        store.set_local_player_pose(crate::LocalPlayerPoseState {
            delta_movement: ProtocolVec3d {
                x: 0.25,
                y: 0.5,
                z: -0.75,
            },
            y_rot: 90.0,
            x_rot: -30.0,
            ..crate::LocalPlayerPoseState::default()
        });
        add_target(
            &mut store,
            8,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
            ProtocolVec3d {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
        );

        assert!(!store.apply_local_player_attack_prediction(8, true));

        let expected = calculate_view_vector(-30.0, 90.0);
        let target = store.probe_entity_transform(8).unwrap();
        assert_eq!(target.delta_movement.x, expected.x);
        assert_eq!(target.delta_movement.y, expected.y);
        assert_eq!(target.delta_movement.z, expected.z);
        assert_eq!(
            store.local_player_pose().unwrap().delta_movement,
            ProtocolVec3d {
                x: 0.25,
                y: 0.5,
                z: -0.75,
            }
        );
    }

    #[test]
    fn full_strength_sprint_attack_pushes_hurt_client_vehicle_and_slows_player() {
        let mut store = local_player_world(7);
        store.set_local_player_pose(crate::LocalPlayerPoseState {
            delta_movement: ProtocolVec3d {
                x: 0.2,
                y: 0.4,
                z: -0.3,
            },
            ..crate::LocalPlayerPoseState::default()
        });
        add_target(
            &mut store,
            8,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            ProtocolVec3d {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
        );
        store.advance_client_level_entity_ticks_with_observation(5);

        assert!(store.apply_local_player_attack_prediction(8, true));

        let target = store.probe_entity_transform(8).unwrap();
        assert_f64_near(target.delta_movement.x, 0.1);
        assert_f64_near(target.delta_movement.y, 0.3);
        assert_f64_near(target.delta_movement.z, 0.8);
        let player = store.local_player_pose().unwrap();
        assert_f64_near(player.delta_movement.x, 0.12);
        assert_f64_near(player.delta_movement.y, 0.4);
        assert_f64_near(player.delta_movement.z, -0.18);
    }

    #[test]
    fn weak_or_server_hurt_target_attack_has_no_local_knockback() {
        let mut store = local_player_world(7);
        store.set_local_player_pose(crate::LocalPlayerPoseState {
            delta_movement: ProtocolVec3d {
                x: 0.2,
                y: 0.4,
                z: -0.3,
            },
            ..crate::LocalPlayerPoseState::default()
        });
        add_target(
            &mut store,
            8,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            ProtocolVec3d::default(),
        );
        add_target(
            &mut store,
            9,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d::default(),
        );

        assert!(!store.apply_local_player_attack_prediction(8, true));
        store.advance_client_level_entity_ticks_with_observation(5);
        assert!(!store.apply_local_player_attack_prediction(9, true));

        let vehicle = store.probe_entity_transform(8).unwrap().delta_movement;
        assert_eq!([vehicle.x, vehicle.y, vehicle.z], [0.0; 3]);
        let living = store.probe_entity_transform(9).unwrap().delta_movement;
        assert_eq!([living.x, living.y, living.z], [0.0; 3]);
        assert_eq!(
            store.local_player_pose().unwrap().delta_movement,
            ProtocolVec3d {
                x: 0.2,
                y: 0.4,
                z: -0.3,
            }
        );
    }

    #[test]
    fn crosshair_attack_indicator_projects_canonical_charge_and_target_eligibility() {
        let mut store = local_player_world(7);
        add_target(
            &mut store,
            8,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d::default(),
        );
        add_target(
            &mut store,
            9,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            ProtocolVec3d::default(),
        );
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

        store.advance_client_level_entity_ticks_with_observation(5);
        assert_eq!(
            store.local_player_crosshair_attack_indicator_state(Some(8)),
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 0.5,
                full: false,
            })
        );

        store.advance_client_level_entity_ticks_with_observation(5);
        assert_eq!(
            store.local_player_crosshair_attack_indicator_state(None),
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 1.0,
                full: false,
            })
        );
        assert_eq!(
            store.local_player_crosshair_attack_indicator_state(Some(9)),
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 1.0,
                full: false,
            })
        );
        assert_eq!(
            store.local_player_crosshair_attack_indicator_state(Some(8)),
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 1.0,
                full: true,
            })
        );
    }

    #[test]
    fn crosshair_full_attack_indicator_requires_slow_weapon_and_positive_health() {
        let mut store = local_player_world(7);
        add_target(
            &mut store,
            8,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d::default(),
        );
        store.advance_client_level_entity_ticks_with_observation(5);
        assert_eq!(
            store.local_player_crosshair_attack_indicator_state(Some(8)),
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 1.0,
                full: false,
            }),
            "the default 4.0 attack speed has an exact five-tick delay"
        );

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
        store.advance_client_level_entity_ticks_with_observation(5);
        let accessors = crate::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        )
        .expect("generated zombie living accessors");
        assert!(store.apply_set_entity_data(SetEntityData {
            id: 8,
            values: vec![EntityDataValue {
                data_id: accessors.health_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(0.0),
            }],
        }));
        assert_eq!(
            store.local_player_crosshair_attack_indicator_state(Some(8)),
            Some(LocalPlayerCrosshairAttackIndicatorState {
                strength_scale: 1.0,
                full: false,
            })
        );
    }

    #[test]
    fn crosshair_attack_indicator_is_absent_without_a_local_player() {
        assert_eq!(
            WorldStore::new().local_player_crosshair_attack_indicator_state(None),
            None
        );
    }

    #[test]
    fn local_attack_hurt_policy_covers_exact_accepting_runtime_families() {
        for entity_type_id in [
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        ] {
            assert!(vanilla_entity_metadata_type_accepts_local_attack_hurt(
                entity_type_id
            ));
        }
        for entity_type_id in [
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
        ] {
            assert!(!vanilla_entity_metadata_type_accepts_local_attack_hurt(
                entity_type_id
            ));
        }
    }

    fn local_player_world(id: i32) -> WorldStore {
        let mut store = WorldStore::new();
        store.local_player_id = Some(id);
        store.apply_add_entity(ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let (score_data_id, score_default) =
            crate::vanilla_entity_metadata_player_score_accessor_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .expect("generated Player score accessor");
        assert_eq!(score_default, 0);
        assert!(store.apply_set_entity_data(SetEntityData {
            id,
            values: vec![EntityDataValue {
                data_id: score_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(42),
            }],
        }));
        store
    }

    fn add_target(
        store: &mut WorldStore,
        id: i32,
        entity_type_id: i32,
        delta_movement: ProtocolVec3d,
    ) {
        store.apply_add_entity(ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.0,
                z: 3.0,
            },
            delta_movement,
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn assert_f64_near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }
}
