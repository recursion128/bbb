use super::*;
use bbb_protocol::{
    block_sound_facts_26_1::vanilla_block_state_sound_type_26_1,
    block_state_types::{vanilla_block_state_id, vanilla_block_state_name},
};

impl WorldStore {
    pub fn client_audio(&self) -> &ClientAudioState {
        &self.client_audio
    }

    /// Draws the next deterministic client-local sound seed, matching vanilla's
    /// `RandomSource.nextLong()` sound-seed derivation
    /// (`Level.soundSeedGenerator` / `ClientLevel.random`) with a fixed local
    /// seed instead of a wall-clock one.
    pub fn next_sound_seed(&mut self) -> i64 {
        self.client_audio.sound_seed_random.next_long()
    }

    /// Resolves a narrow entity-movement sound into the same positioned sound
    /// state used by packet and client-local audio, consuming the canonical
    /// client sound seed at the movement effect's ordered position.
    pub fn record_entity_client_movement_sound(
        &mut self,
        state: EntityClientMovementSoundState,
    ) -> SoundEventState {
        let (sound, source, volume, pitch) = match state.kind {
            EntityClientMovementSoundKind::BlazeBurn { volume, pitch } => {
                (BLAZE_BURN_SOUND_EVENT, "hostile", volume, pitch)
            }
            EntityClientMovementSoundKind::WardenHeartbeat { pitch } => {
                (WARDEN_HEARTBEAT_SOUND_EVENT, "hostile", 5.0, pitch)
            }
            EntityClientMovementSoundKind::EnderDragonFlap { pitch } => {
                (ENDER_DRAGON_FLAP_SOUND_EVENT, "hostile", 5.0, pitch)
            }
            EntityClientMovementSoundKind::EnderDragonGrowl { pitch } => {
                (ENDER_DRAGON_GROWL_SOUND_EVENT, "hostile", 2.5, pitch)
            }
            EntityClientMovementSoundKind::LightningBoltThunder { pitch } => (
                LIGHTNING_BOLT_THUNDER_SOUND_EVENT,
                "weather",
                10_000.0,
                pitch,
            ),
            EntityClientMovementSoundKind::LightningBoltImpact { pitch } => {
                (LIGHTNING_BOLT_IMPACT_SOUND_EVENT, "weather", 2.0, pitch)
            }
            EntityClientMovementSoundKind::GuardianFlop { elder } => (
                if elder {
                    ELDER_GUARDIAN_FLOP_SOUND_EVENT
                } else {
                    GUARDIAN_FLOP_SOUND_EVENT
                },
                "hostile",
                1.0,
                1.0,
            ),
            EntityClientMovementSoundKind::PlayerBlockStep {
                block_state_id,
                muffled,
            } => {
                let profile = vanilla_block_state_id(block_state_id)
                    .map(vanilla_block_state_sound_type_26_1)
                    .unwrap_or_else(|| vanilla_block_state_sound_type_26_1(0));
                (
                    profile.step_sound,
                    "player",
                    profile.volume * if muffled { 0.05 } else { 0.15 },
                    profile.pitch * if muffled { 0.8 } else { 1.0 },
                )
            }
            EntityClientMovementSoundKind::PlayerSwim { volume, pitch } => {
                ("minecraft:entity.player.swim", "player", volume, pitch)
            }
            EntityClientMovementSoundKind::PlayerSplash {
                high_speed,
                volume,
                pitch,
            } => (
                if high_speed {
                    "minecraft:entity.player.splash.high_speed"
                } else {
                    "minecraft:entity.player.splash"
                },
                "player",
                volume,
                pitch,
            ),
            EntityClientMovementSoundKind::PlayerCrystalChime { volume, pitch } => (
                "minecraft:block.amethyst_block.chime",
                "player",
                volume,
                pitch,
            ),
            EntityClientMovementSoundKind::PlayerHoneyBlockSlide => {
                (HONEY_BLOCK_SLIDE_SOUND_EVENT, "player", 1.0, 1.0)
            }
            EntityClientMovementSoundKind::PlayerPowderSnowFall { big }
            | EntityClientMovementSoundKind::PlayerFall { big } => (
                if big {
                    PLAYER_BIG_FALL_SOUND_EVENT
                } else {
                    PLAYER_SMALL_FALL_SOUND_EVENT
                },
                "player",
                1.0,
                1.0,
            ),
            EntityClientMovementSoundKind::PlayerBlockFall { block_state_id } => {
                let profile = vanilla_block_state_id(block_state_id)
                    .map(vanilla_block_state_sound_type_26_1)
                    .unwrap_or_else(|| vanilla_block_state_sound_type_26_1(0));
                (
                    profile.fall_sound,
                    "player",
                    profile.volume * 0.5,
                    profile.pitch * 0.75,
                )
            }
            EntityClientMovementSoundKind::EndermanStare => {
                // Vanilla `EnderMan.playStareSound` (`EnderMan.java:173-180`).
                ("minecraft:entity.enderman.stare", "hostile", 2.5, 1.0)
            }
            EntityClientMovementSoundKind::BreezeWhirl { volume, pitch } => {
                // Vanilla `Breeze.playWhirlSound` (`Breeze.java:189-193`).
                ("minecraft:entity.breeze.whirl", "hostile", volume, pitch)
            }
            EntityClientMovementSoundKind::BreezeIdle { on_ground } => (
                // Vanilla `Breeze.playAmbientSound` / `getAmbientSound` (`Breeze.java:183-187, 215-217`).
                if on_ground {
                    "minecraft:entity.breeze.idle_ground"
                } else {
                    "minecraft:entity.breeze.idle_air"
                },
                "hostile",
                1.0,
                1.0,
            ),
            EntityClientMovementSoundKind::PhantomFlap { volume, pitch } => {
                // Vanilla `Phantom.tick` (`Phantom.java:115-127`).
                ("minecraft:entity.phantom.flap", "hostile", volume, pitch)
            }
            EntityClientMovementSoundKind::SnifferSearching => {
                // Vanilla `Sniffer.playSearchingSound` (`Sniffer.java:390-394`).
                ("minecraft:entity.sniffer.searching", "neutral", 1.0, 1.0)
            }
            EntityClientMovementSoundKind::SnifferDigHit { block_state_id } => {
                // Vanilla `Sniffer.emitDiggingParticles` (`Sniffer.java:296-312`).
                let profile = vanilla_block_state_id(block_state_id)
                    .map(vanilla_block_state_sound_type_26_1)
                    .unwrap_or_else(|| vanilla_block_state_sound_type_26_1(0));
                (profile.hit_sound, "neutral", 0.5, 0.5)
            }
            EntityClientMovementSoundKind::CopperGolemSpin { weather_state } => {
                // Vanilla `CopperGolem.setupAnimationStates` (`CopperGolem.java:324-327`,
                // `CopperGolemOxidationLevels.java:9-40`).
                let sound = match weather_state {
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => {
                        "minecraft:entity.copper_golem_weathered.spin"
                    }
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => {
                        "minecraft:entity.copper_golem_oxidized.spin"
                    }
                    _ => "minecraft:entity.copper_golem.spin",
                };
                (sound, "neutral", 1.0, 1.0)
            }
        };
        let event = SoundEventState {
            sound: direct_sound_holder(sound),
            source: source.to_string(),
            position: state.position,
            volume,
            pitch,
            seed: self.next_sound_seed(),
            distance_delay: false,
        };
        self.record_positioned_sound(event)
    }

    pub fn last_sound(&self) -> Option<&SoundEventState> {
        self.client_audio.last_sound.as_ref()
    }

    pub fn last_local_sound(&self) -> Option<&LocalSoundEventState> {
        self.client_audio.last_local_sound.as_ref()
    }

    pub fn last_sound_entity(&self) -> Option<&SoundEntityEventState> {
        self.client_audio.last_sound_entity.as_ref()
    }

    pub fn last_stop_sound(&self) -> Option<&StopSoundEventState> {
        self.client_audio.last_stop_sound.as_ref()
    }

    pub fn playing_jukebox_songs(&self) -> &[JukeboxSongState] {
        &self.client_audio.playing_jukebox_songs
    }

    pub fn last_jukebox_event(&self) -> Option<&JukeboxLevelEventState> {
        self.client_audio.last_jukebox_event.as_ref()
    }

    pub(crate) fn prune_playing_jukebox_songs_in_chunk(&mut self, chunk: crate::ChunkPos) {
        if self.client_audio.playing_jukebox_songs.is_empty() {
            return;
        }
        let before = self.client_audio.playing_jukebox_songs.len();
        self.client_audio.playing_jukebox_songs.retain(|song| {
            song.pos.x.div_euclid(16) != chunk.x || song.pos.z.div_euclid(16) != chunk.z
        });
        if self.client_audio.playing_jukebox_songs.len() != before {
            self.client_audio.playing_jukebox_song_index.0.clear();
            for (index, song) in self.client_audio.playing_jukebox_songs.iter().enumerate() {
                self.client_audio
                    .playing_jukebox_song_index
                    .0
                    .insert(song.pos, index);
            }
        }
    }

    pub(crate) fn record_jukebox_level_event(
        &mut self,
        event: ProtocolLevelEvent,
    ) -> Option<JukeboxLevelEventState> {
        const MAX_PLAYING_JUKEBOX_SONGS: usize = 256;
        let pos = event.pos;
        if self.client_audio.playing_jukebox_song_index.0.len()
            != self.client_audio.playing_jukebox_songs.len()
        {
            self.client_audio.playing_jukebox_song_index.0.clear();
            for (index, song) in self.client_audio.playing_jukebox_songs.iter().enumerate() {
                self.client_audio
                    .playing_jukebox_song_index
                    .0
                    .insert(song.pos, index);
            }
        }
        let state = match event.event_type {
            JUKEBOX_PLAY_LEVEL_EVENT if event.data >= 0 => {
                let presentation = self
                    .jukebox_song_level_event_presentation(event.data)
                    .ok()
                    .flatten()?;
                let stopped_existing = if let Some(&index) =
                    self.client_audio.playing_jukebox_song_index.0.get(&pos)
                {
                    self.client_audio.playing_jukebox_songs[index].song_registry_id = event.data;
                    true
                } else {
                    if self.client_audio.playing_jukebox_songs.len() >= MAX_PLAYING_JUKEBOX_SONGS {
                        let evicted = self.client_audio.playing_jukebox_songs.swap_remove(0);
                        self.client_audio
                            .playing_jukebox_song_index
                            .0
                            .remove(&evicted.pos);
                        if let Some(moved) = self.client_audio.playing_jukebox_songs.first() {
                            self.client_audio
                                .playing_jukebox_song_index
                                .0
                                .insert(moved.pos, 0);
                        }
                    }
                    let index = self.client_audio.playing_jukebox_songs.len();
                    self.client_audio
                        .playing_jukebox_songs
                        .push(JukeboxSongState {
                            pos,
                            song_registry_id: event.data,
                        });
                    self.client_audio
                        .playing_jukebox_song_index
                        .0
                        .insert(pos, index);
                    false
                };
                self.set_jukebox_now_playing(presentation.description);
                JukeboxLevelEventState {
                    action: JukeboxLevelEventAction::Start,
                    pos,
                    song_registry_id: Some(event.data),
                    stopped_existing,
                }
            }
            JUKEBOX_STOP_LEVEL_EVENT => {
                let stopped_existing = if let Some(index) =
                    self.client_audio.playing_jukebox_song_index.0.remove(&pos)
                {
                    self.client_audio.playing_jukebox_songs.swap_remove(index);
                    if let Some(moved) = self.client_audio.playing_jukebox_songs.get(index) {
                        self.client_audio
                            .playing_jukebox_song_index
                            .0
                            .insert(moved.pos, index);
                    }
                    true
                } else {
                    false
                };
                JukeboxLevelEventState {
                    action: JukeboxLevelEventAction::Stop,
                    pos,
                    song_registry_id: None,
                    stopped_existing,
                }
            }
            _ => return None,
        };
        self.client_audio.last_jukebox_event = Some(state);
        Some(state)
    }

    pub fn local_block_hit_sound(&self, pos: crate::BlockPos) -> Option<SoundEventState> {
        let block = self.block_view(pos)?;
        let block_name = block.block_name?;
        if block_name_is_air(block_name) {
            return None;
        }
        let state_id = vanilla_block_state_id(block.block_state_id)?;
        let profile = vanilla_block_state_sound_type_26_1(state_id);
        Some(block_sound_state(
            pos,
            profile.hit_sound,
            (profile.volume + 1.0) / 8.0,
            profile.pitch * 0.5,
            "block",
        ))
    }

    pub fn level_event_block_break_sound(
        &self,
        event: ProtocolLevelEvent,
    ) -> Option<SoundEventState> {
        self.level_event_block_break_sound_state(event)
    }

    pub fn level_event_sound(&self, event: ProtocolLevelEvent) -> Option<SoundEventState> {
        if let Some(state) = self.level_event_block_break_sound_state(event) {
            return Some(state);
        }
        if let Some(state) = self.level_event_brush_complete_sound_state(event) {
            return Some(state);
        }
        let sound = fixed_level_event_sound(event.event_type)?;
        Some(block_sound_state(
            event.pos,
            sound.event_id,
            sound.volume,
            sound.pitch,
            sound.source,
        ))
    }

    pub fn level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if let Some(state) = self.level_event_sound(event) {
            return Some(state);
        }
        if let Some(sound) =
            random_distance_delayed_level_event_sound(event.event_type, event.data, &mut next_float)
        {
            return Some(block_sound_state_with_distance_delay(
                event.pos,
                sound.event_id,
                sound.volume,
                sound.pitch,
                sound.source,
                true,
            ));
        }
        let sound = random_level_event_sound(event.event_type, event.data, &mut next_float)?;
        Some(block_sound_state(
            event.pos,
            sound.event_id,
            sound.volume,
            sound.pitch,
            sound.source,
        ))
    }

    pub fn cobweb_place_level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if event.event_type != COBWEB_PLACE_LEVEL_EVENT {
            return None;
        }
        Some(block_sound_state_with_distance_delay(
            event.pos,
            "minecraft:block.cobweb.place",
            1.0,
            triangle_pitch(1.0, 0.2, &mut next_float),
            "block",
            true,
        ))
    }

    pub fn sculk_shrieker_level_event_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if event.event_type != SCULK_SHRIEKER_LEVEL_EVENT {
            return None;
        }
        let pos = event.pos;
        if self
            .block_view(pos)
            .and_then(|probe| probe.block_properties.get("waterlogged").cloned())
            .is_some_and(|waterlogged| waterlogged == "true")
        {
            return None;
        }
        Some(SoundEventState {
            sound: direct_sound_holder("minecraft:block.sculk_shrieker.shriek"),
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + SCULK_SHRIEKER_TOP_Y,
                z: f64::from(pos.z) + 0.5,
            },
            volume: 2.0,
            pitch: ranged_pitch(0.6, 0.4, &mut next_float),
            seed: 0,
            distance_delay: false,
        })
    }

    pub fn level_event_local_sound_with_random(
        &self,
        event: ProtocolLevelEvent,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<LocalSoundEventState> {
        let sound = local_level_event_sound(event.event_type, &mut next_float)?;
        Some(LocalSoundEventState {
            sound: direct_sound_holder(sound.event_id),
            source: sound.source.to_string(),
            volume: sound.volume,
            pitch: sound.pitch,
            seed: 0,
        })
    }

    pub fn record_local_sound(&mut self, state: LocalSoundEventState) -> LocalSoundEventState {
        self.client_audio.last_local_sound = Some(state.clone());
        state
    }

    pub(crate) fn record_explosion_sound(
        &mut self,
        explosion: &crate::ExplosionEventState,
        random: &mut LevelEventSoundRandomState,
    ) -> SoundEventState {
        let pitch = (1.0 + (random.next_float() - random.next_float()) * 0.2) * 0.7;
        let state = SoundEventState {
            sound: sound_holder_state(explosion.explosion_sound.clone()),
            source: SoundSource::Blocks.as_str().to_string(),
            position: explosion.center,
            volume: 4.0,
            pitch,
            seed: random.next_long(),
            distance_delay: false,
        };
        self.record_positioned_sound(state)
    }

    pub(super) fn level_event_block_break_sound_state(
        &self,
        event: ProtocolLevelEvent,
    ) -> Option<SoundEventState> {
        if event.event_type != BLOCK_BREAK_LEVEL_EVENT {
            return None;
        }
        let block_name = vanilla_block_state_name(event.data)?;
        if block_name_is_air(block_name) {
            return None;
        }
        let state_id = vanilla_block_state_id(event.data)?;
        let profile = vanilla_block_state_sound_type_26_1(state_id);
        Some(block_sound_state(
            event.pos,
            profile.break_sound,
            (profile.volume + 1.0) / 2.0,
            profile.pitch * 0.8,
            "block",
        ))
    }

    pub(super) fn level_event_brush_complete_sound_state(
        &self,
        event: ProtocolLevelEvent,
    ) -> Option<SoundEventState> {
        if event.event_type != BRUSH_BLOCK_COMPLETE_LEVEL_EVENT {
            return None;
        }
        let block_name = vanilla_block_state_name(event.data)?;
        let sound = match block_name {
            // Vanilla 26.1 Blocks.SUSPICIOUS_* constructs BrushableBlock with the
            // same brush sound for regular and completed brush events.
            "minecraft:suspicious_sand" => "minecraft:item.brush.brushing.sand",
            "minecraft:suspicious_gravel" => "minecraft:item.brush.brushing.gravel",
            _ => return None,
        };
        Some(block_sound_state(event.pos, sound, 1.0, 1.0, "player"))
    }
}
