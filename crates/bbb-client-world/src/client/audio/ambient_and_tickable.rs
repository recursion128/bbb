use super::*;

impl WorldStore {
    /// Completes the post-movement ambient-handler pass for the current tick.
    pub fn complete_local_player_ambient_audio_tick(&mut self) -> bool {
        if !self
            .client_audio
            .local_player_ambient_audio_tick_pending_completion
        {
            return false;
        }
        self.client_audio
            .local_player_ambient_audio_tick_pending_completion = false;
        let Some(pose) = self.local_player.pose else {
            return false;
        };
        let bubble_column = local_player_bubble_column_drag_down(self, pose);
        let was_in_bubble_column = self.client_audio.local_player_was_in_bubble_column;
        let first_tick = !self.client_audio.local_player_ambient_audio_started;
        self.client_audio.local_player_was_in_bubble_column = bubble_column.is_some();
        self.client_audio.local_player_ambient_audio_started = true;
        let bubble_column_transition =
            if was_in_bubble_column || first_tick || self.local_player_is_spectator() {
                None
            } else {
                bubble_column.map(|drag_down| {
                    if drag_down {
                        LocalPlayerBubbleColumnAudioTransition::Whirlpool
                    } else {
                        LocalPlayerBubbleColumnAudioTransition::Upwards
                    }
                })
            };
        let Some(tick) = self
            .client_audio
            .pending_local_player_ambient_audio_ticks
            .back_mut()
        else {
            return false;
        };
        tick.bubble_column_position = pose.position;
        tick.bubble_column_transition = bubble_column_transition;
        true
    }

    /// Advances both phases when no local movement step separates them.
    pub fn advance_local_player_ambient_audio_tick(&mut self) -> bool {
        self.begin_local_player_ambient_audio_tick()
            && self.complete_local_player_ambient_audio_tick()
    }

    pub fn take_pending_local_player_ambient_audio_ticks(
        &mut self,
    ) -> Vec<LocalPlayerAmbientAudioTickState> {
        self.client_audio
            .pending_local_player_ambient_audio_ticks
            .drain(..)
            .collect()
    }

    pub(crate) fn reset_local_player_ambient_audio(&mut self) {
        self.client_audio.local_player_was_underwater = false;
        self.client_audio.local_player_was_in_bubble_column = false;
        self.client_audio.local_player_ambient_audio_started = false;
        self.client_audio
            .local_player_ambient_audio_tick_pending_completion = false;
        self.client_audio
            .pending_local_player_ambient_audio_ticks
            .clear();
    }

    pub(crate) fn queue_local_interaction_sound_action(
        &mut self,
        action: LocalInteractionSoundAction,
    ) {
        if self
            .client_audio
            .pending_local_interaction_sound_actions
            .len()
            >= MAX_PENDING_LOCAL_INTERACTION_SOUND_ACTIONS
        {
            self.client_audio
                .pending_local_interaction_sound_actions
                .pop_front();
        }
        self.client_audio
            .pending_local_interaction_sound_actions
            .push_back(action);
    }

    pub fn take_pending_local_interaction_sound_actions(
        &mut self,
    ) -> Vec<LocalInteractionSoundAction> {
        self.client_audio
            .pending_local_interaction_sound_actions
            .drain(..)
            .collect()
    }

    /// Advances retained state whose vanilla sound instance mutates once per
    /// client tick. World time is advanced before callers project bee anger;
    /// minecart pitch itself advances only while the tick-rate manager runs.
    pub fn advance_tickable_entity_sounds(&mut self, client_ticks: u32, running_ticks: u32) {
        if client_ticks == 0 {
            return;
        }
        let minecart_updates = self
            .client_audio
            .active_tickable_entity_sounds
            .iter()
            .filter_map(|(&entity_id, active)| {
                let ActiveTickableEntitySound::Minecart { pitch, .. } = *active else {
                    return None;
                };
                let speed = self
                    .entities
                    .transform_state(entity_id)
                    .map(|transform| horizontal_speed(transform.delta_movement));
                let (next_volume, next_pitch) = match speed {
                    Some(speed)
                        if running_ticks > 0
                            && !self.minecart_sound_off_rail(entity_id)
                            && speed >= 0.01 =>
                    {
                        (
                            sound_lerp(speed.clamp(0.0, 0.5), 0.0, 0.7),
                            (pitch + 0.0025 * running_ticks as f32).clamp(0.0, 1.0),
                        )
                    }
                    _ => (0.0, 0.0),
                };
                Some((entity_id, next_volume, next_pitch))
            })
            .collect::<Vec<_>>();
        for (entity_id, volume, pitch) in minecart_updates {
            if let Some(ActiveTickableEntitySound::Minecart {
                volume: active_volume,
                pitch: active_pitch,
                ..
            }) = self
                .client_audio
                .active_tickable_entity_sounds
                .get_mut(&entity_id)
            {
                *active_volume = volume;
                *active_pitch = pitch;
            }
        }
        self.advance_local_player_elytra_sound(client_ticks);
        self.sync_local_player_riding_sounds();
    }

    fn advance_local_player_elytra_sound(&mut self, client_ticks: u32) {
        let local_player_id = self.local_player_id;
        let is_fall_flying = local_player_id
            .and_then(|id| self.entities.is_fall_flying(id))
            .unwrap_or(false);
        if is_fall_flying && !self.client_audio.local_player_was_fall_flying {
            // Vanilla `LocalPlayer.onSyncedDataUpdated` starts a fresh
            // `ElytraOnPlayerSoundInstance` on `!wasFallFlying && isFallFlying`.
            self.client_audio.local_player_elytra_sound = Some(ActiveElytraSound {
                seed: self.next_sound_seed(),
                time: 0,
                volume: 0.1,
                pitch: 1.0,
            });
        }
        self.client_audio.local_player_was_fall_flying = is_fall_flying;

        let Some(mut elytra) = self.client_audio.local_player_elytra_sound else {
            return;
        };
        let Some(local_id) = local_player_id else {
            self.client_audio.local_player_elytra_sound = None;
            return;
        };
        let [dx, dy, dz] = self
            .local_player
            .pose
            .map(|pose| {
                [
                    pose.delta_movement.x,
                    pose.delta_movement.y,
                    pose.delta_movement.z,
                ]
            })
            .or_else(|| {
                self.entities.transform_state(local_id).map(|transform| {
                    [
                        transform.delta_movement.x,
                        transform.delta_movement.y,
                        transform.delta_movement.z,
                    ]
                })
            })
            .unwrap_or([0.0, 0.0, 0.0]);
        let speed_sqr = (dx * dx + dy * dy + dz * dz) as f32;
        for _ in 0..client_ticks {
            elytra.time = elytra.time.saturating_add(1);
            // Vanilla `ElytraOnPlayerSoundInstance.tick`: stop once past the
            // 20-tick delay and no longer fall-flying.
            if elytra.time > 20 && !is_fall_flying {
                self.client_audio.local_player_elytra_sound = None;
                return;
            }
            let mut volume = if speed_sqr >= 1.0e-7 {
                (speed_sqr / 4.0).clamp(0.0, 1.0)
            } else {
                0.0
            };
            if elytra.time < 20 {
                volume = 0.0;
            } else if elytra.time < 40 {
                volume *= (elytra.time - 20) as f32 / 20.0;
            }
            elytra.volume = volume;
            elytra.pitch = if volume > 0.8 {
                1.0 + (volume - 0.8)
            } else {
                1.0
            };
        }
        self.client_audio.local_player_elytra_sound = Some(elytra);
    }

    fn sync_local_player_riding_sounds(&mut self) {
        let Some(vehicle_id) = self.local_player_vehicle_id else {
            self.client_audio.local_player_riding_sounds = None;
            return;
        };
        let Some(transform) = self.entities.transform_state(vehicle_id) else {
            self.client_audio.local_player_riding_sounds = None;
            return;
        };
        let entity_type_id = transform.entity_type_id;
        let matches_existing = match self.client_audio.local_player_riding_sounds {
            Some(ActiveLocalPlayerRidingSounds::Minecart {
                vehicle_id: current,
                ..
            }) => current == vehicle_id && is_vanilla_minecart_type(entity_type_id),
            Some(ActiveLocalPlayerRidingSounds::HappyGhast {
                vehicle_id: current,
                ..
            }) => current == vehicle_id && entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            Some(ActiveLocalPlayerRidingSounds::Nautilus {
                vehicle_id: current,
                ..
            }) => current == vehicle_id && is_vanilla_abstract_nautilus_type(entity_type_id),
            None => false,
        };
        if matches_existing {
            return;
        }
        // Vanilla `LocalPlayer.startRiding`: allocate looping riding sound
        // instances for minecarts (underwater + dry), happy ghasts, and nautiluses.
        self.client_audio.local_player_riding_sounds = if is_vanilla_minecart_type(entity_type_id) {
            Some(ActiveLocalPlayerRidingSounds::Minecart {
                vehicle_id,
                underwater_seed: self.next_sound_seed(),
                dry_seed: self.next_sound_seed(),
            })
        } else if entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            Some(ActiveLocalPlayerRidingSounds::HappyGhast {
                vehicle_id,
                seed: self.next_sound_seed(),
            })
        } else if is_vanilla_abstract_nautilus_type(entity_type_id) {
            Some(ActiveLocalPlayerRidingSounds::Nautilus {
                vehicle_id,
                seed: self.next_sound_seed(),
            })
        } else {
            None
        };
    }

    pub fn tickable_entity_sounds(&self) -> Vec<TickableEntitySoundState> {
        let mut sounds: Vec<TickableEntitySoundState> = self
            .client_audio
            .active_tickable_entity_sounds
            .iter()
            .filter_map(|(&entity_id, active)| self.tickable_entity_sound_for(entity_id, *active))
            .collect();
        if let Some(elytra_sound) = self.local_player_elytra_tickable_sound() {
            sounds.push(elytra_sound);
        }
        self.push_local_player_riding_tickable_sounds(&mut sounds);
        sounds
    }

    fn local_player_elytra_tickable_sound(&self) -> Option<TickableEntitySoundState> {
        let elytra = self.client_audio.local_player_elytra_sound?;
        let local_id = self.local_player_id?;
        let position = self
            .local_player
            .pose
            .map(|pose| pose.position)
            .or_else(|| {
                self.entities
                    .transform_state(local_id)
                    .map(|transform| transform.position)
            })?;
        Some(TickableEntitySoundState {
            event: SoundEntityEventState {
                sound: direct_sound_holder(ELYTRA_FLYING_SOUND_EVENT),
                source: SoundSource::Players.as_str().to_string(),
                entity_id: local_id,
                volume: elytra.volume,
                pitch: elytra.pitch,
                seed: elytra.seed,
                looping: true,
                linear_attenuation: true,
                tickable: true,
            },
            position,
        })
    }

    fn push_local_player_riding_tickable_sounds(&self, out: &mut Vec<TickableEntitySoundState>) {
        let Some(riding) = self.client_audio.local_player_riding_sounds else {
            return;
        };
        let vehicle_id = riding.vehicle_id();
        if self.local_player_vehicle_id != Some(vehicle_id) {
            return;
        }
        if self.entities.is_silent(vehicle_id).unwrap_or(true) {
            return;
        }
        let Some(transform) = self.entities.transform_state(vehicle_id) else {
            return;
        };
        match riding {
            ActiveLocalPlayerRidingSounds::Minecart {
                vehicle_id,
                dry_seed,
                underwater_seed,
            } => {
                if !is_vanilla_minecart_type(transform.entity_type_id) {
                    return;
                }
                let player_underwater = self.local_player_eye_in_water();
                let speed = horizontal_speed(transform.delta_movement);
                let active_volume = if speed >= 0.01 && !self.minecart_sound_off_rail(vehicle_id) {
                    sound_lerp(speed.clamp(0.0, 1.0), 0.0, 0.75)
                } else {
                    0.0
                };
                out.push(TickableEntitySoundState {
                    event: SoundEntityEventState {
                        sound: direct_sound_holder(MINECART_INSIDE_UNDERWATER_SOUND_EVENT),
                        source: SoundSource::Neutral.as_str().to_string(),
                        entity_id: vehicle_id,
                        volume: if player_underwater {
                            active_volume
                        } else {
                            0.0
                        },
                        pitch: 1.0,
                        seed: underwater_seed,
                        looping: true,
                        linear_attenuation: false,
                        tickable: true,
                    },
                    position: transform.position,
                });
                out.push(TickableEntitySoundState {
                    event: SoundEntityEventState {
                        sound: direct_sound_holder(MINECART_INSIDE_SOUND_EVENT),
                        source: SoundSource::Neutral.as_str().to_string(),
                        entity_id: vehicle_id,
                        volume: if !player_underwater {
                            active_volume
                        } else {
                            0.0
                        },
                        pitch: 1.0,
                        seed: dry_seed,
                        looping: true,
                        linear_attenuation: false,
                        tickable: true,
                    },
                    position: transform.position,
                });
            }
            ActiveLocalPlayerRidingSounds::HappyGhast { vehicle_id, seed } => {
                if transform.entity_type_id != VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
                    return;
                }
                let underwater = self.entity_eye_in_water(vehicle_id);
                let speed = vector_length(transform.delta_movement);
                let volume = if !underwater && speed >= 0.01 {
                    5.0 * sound_lerp(speed.clamp(0.0, 1.0), 0.0, 1.0)
                } else {
                    0.0
                };
                out.push(TickableEntitySoundState {
                    event: SoundEntityEventState {
                        sound: direct_sound_holder(HAPPY_GHAST_RIDING_SOUND_EVENT),
                        source: vanilla_entity_sound_source(transform.entity_type_id)
                            .as_str()
                            .to_string(),
                        entity_id: vehicle_id,
                        volume,
                        pitch: 1.0,
                        seed,
                        looping: true,
                        linear_attenuation: false,
                        tickable: true,
                    },
                    position: transform.position,
                });
            }
            ActiveLocalPlayerRidingSounds::Nautilus { vehicle_id, seed } => {
                if !is_vanilla_abstract_nautilus_type(transform.entity_type_id) {
                    return;
                }
                let underwater = self.entity_eye_in_water(vehicle_id);
                let speed = vector_length(transform.delta_movement);
                let volume = if underwater && speed >= 0.01 {
                    5.0 * sound_lerp(speed.clamp(0.0, 1.0), 0.0, 1.0)
                } else {
                    0.0
                };
                out.push(TickableEntitySoundState {
                    event: SoundEntityEventState {
                        sound: direct_sound_holder(NAUTILUS_RIDING_SOUND_EVENT),
                        source: vanilla_entity_sound_source(transform.entity_type_id)
                            .as_str()
                            .to_string(),
                        entity_id: vehicle_id,
                        volume,
                        pitch: 1.0,
                        seed,
                        looping: true,
                        linear_attenuation: false,
                        tickable: true,
                    },
                    position: transform.position,
                });
            }
        }
    }

    pub(super) fn tickable_entity_sound_for(
        &self,
        entity_id: i32,
        active: ActiveTickableEntitySound,
    ) -> Option<TickableEntitySoundState> {
        let (position, sound, source, volume, pitch, seed, looping, attenuation) = match active {
            ActiveTickableEntitySound::GuardianAttack { seed } => {
                if self.entities.is_silent(entity_id)? {
                    return None;
                }
                let (position, attack_scale) =
                    self.entities.guardian_attack_sound_state(entity_id)?;
                (
                    position,
                    GUARDIAN_ATTACK_SOUND_EVENT,
                    SoundSource::Hostile,
                    attack_scale * attack_scale,
                    0.7 + 0.5 * attack_scale,
                    seed,
                    true,
                    false,
                )
            }
            ActiveTickableEntitySound::SnifferDigging { seed } => {
                if self.entities.is_silent(entity_id)? {
                    return None;
                }
                if !self.entities.sniffer_can_play_digging_sound(entity_id)? {
                    return None;
                }
                let transform = self.entities.transform_state(entity_id)?;
                (
                    transform.position,
                    SNIFFER_DIGGING_SOUND_EVENT,
                    SoundSource::Neutral,
                    1.0,
                    1.0,
                    seed,
                    false,
                    true,
                )
            }
            ActiveTickableEntitySound::Minecart {
                seed,
                volume,
                pitch,
            } => {
                if self.entities.is_silent(entity_id)? {
                    return None;
                }
                let transform = self.entities.transform_state(entity_id)?;
                if !is_vanilla_minecart_type(transform.entity_type_id) {
                    return None;
                }
                (
                    transform.position,
                    MINECART_RIDING_SOUND_EVENT,
                    SoundSource::Neutral,
                    volume,
                    pitch,
                    seed,
                    true,
                    true,
                )
            }
            ActiveTickableEntitySound::Bee { seed } => {
                if self.entities.is_silent(entity_id)? {
                    return None;
                }
                let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
                let bee = self.entities.bee_sound_state(entity_id, game_time)?;
                let speed = horizontal_speed(bee.transform.delta_movement);
                let (pitch, volume) = if speed >= 0.01 {
                    let (min_pitch, max_pitch) = if bee.is_baby { (1.1, 1.5) } else { (0.7, 1.1) };
                    (
                        sound_lerp(speed.clamp(min_pitch, max_pitch), min_pitch, max_pitch),
                        sound_lerp(speed.clamp(0.0, 0.5), 0.0, 1.2),
                    )
                } else {
                    (0.0, 0.0)
                };
                (
                    bee.transform.position,
                    if bee.angry {
                        BEE_AGGRESSIVE_LOOP_SOUND_EVENT
                    } else {
                        BEE_LOOP_SOUND_EVENT
                    },
                    SoundSource::Neutral,
                    volume,
                    pitch,
                    seed,
                    true,
                    true,
                )
            }
        };
        Some(TickableEntitySoundState {
            event: SoundEntityEventState {
                sound: direct_sound_holder(sound),
                source: source.as_str().to_string(),
                entity_id,
                volume,
                pitch,
                seed,
                looping,
                linear_attenuation: attenuation,
                tickable: true,
            },
            position,
        })
    }

    pub(super) fn minecart_sound_off_rail(&self, entity_id: i32) -> bool {
        if !self.is_feature_enabled("minecraft:minecart_improvements") {
            return false;
        }
        let Some(transform) = self.entities.transform_state(entity_id) else {
            return true;
        };
        let rail_below = BlockPos {
            x: transform.position.x.floor() as i32,
            y: (transform.position.y - 0.1 - 1.0e-5).floor() as i32,
            z: transform.position.z.floor() as i32,
        };
        let current_block = BlockPos {
            y: transform.position.y.floor() as i32,
            ..rail_below
        };
        ![rail_below, current_block].into_iter().any(|pos| {
            self.block_view(pos)
                .and_then(|block| block.block_name)
                .is_some_and(block_name_is_rail)
        })
    }

    pub(crate) fn clear_tickable_entity_sounds(&mut self) {
        self.client_audio.active_tickable_entity_sounds.clear();
        self.client_audio.local_player_was_fall_flying = false;
        self.client_audio.local_player_elytra_sound = None;
        self.client_audio.local_player_riding_sounds = None;
    }

    pub(crate) fn remove_tickable_entity_sounds(&mut self, entity_ids: &[i32]) {
        for entity_id in entity_ids {
            self.client_audio
                .active_tickable_entity_sounds
                .remove(entity_id);
        }
        if let Some(riding) = self.client_audio.local_player_riding_sounds {
            if entity_ids.contains(&riding.vehicle_id()) {
                self.client_audio.local_player_riding_sounds = None;
            }
        }
        if let Some(local_id) = self.local_player_id {
            if entity_ids.contains(&local_id) {
                self.client_audio.local_player_was_fall_flying = false;
                self.client_audio.local_player_elytra_sound = None;
            }
        }
    }
}

fn vector_length(delta: crate::EntityVec3) -> f32 {
    (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt() as f32
}
