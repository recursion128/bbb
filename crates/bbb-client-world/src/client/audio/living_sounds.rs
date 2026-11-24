use super::*;

impl WorldStore {
    pub fn sniffer_digging_sound_for_entity(
        &mut self,
        entity_id: i32,
    ) -> Option<SoundEntityEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_SNIFFER_ID
            || self.entities.is_silent(entity_id)?
        {
            return None;
        }
        let seed = self.next_sound_seed();
        let state = SoundEntityEventState {
            sound: direct_sound_holder(SNIFFER_DIGGING_SOUND_EVENT),
            source: SoundSource::Neutral.as_str().to_string(),
            entity_id,
            volume: 1.0,
            pitch: 1.0,
            seed,
            looping: false,
            linear_attenuation: true,
            tickable: true,
        };
        self.client_audio.active_tickable_entity_sounds.insert(
            entity_id,
            ActiveTickableEntitySound::SnifferDigging { seed },
        );
        self.client_audio.last_sound_entity = Some(state.clone());
        Some(state)
    }

    /// Vanilla `ClientLevel.postAddEntitySoundInstance`: minecarts and bees
    /// immediately acquire a client-owned looping sound instance after their
    /// canonical entity has been added to the level.
    pub(crate) fn post_add_entity_sound_for_entity(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> Option<SoundEntityEventState> {
        let active = if is_vanilla_minecart_type(entity_type_id) {
            ActiveTickableEntitySound::Minecart {
                seed: self.next_sound_seed(),
                volume: 0.0,
                pitch: 0.0,
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_BEE_ID {
            ActiveTickableEntitySound::Bee {
                seed: self.next_sound_seed(),
            }
        } else {
            return None;
        };
        self.client_audio
            .active_tickable_entity_sounds
            .insert(entity_id, active);
        if self.entities.is_silent(entity_id).unwrap_or(false) {
            return None;
        }
        let state = SoundEntityEventState {
            sound: direct_sound_holder(if is_vanilla_minecart_type(entity_type_id) {
                MINECART_RIDING_SOUND_EVENT
            } else {
                BEE_LOOP_SOUND_EVENT
            }),
            source: SoundSource::Neutral.as_str().to_string(),
            entity_id,
            volume: 0.0,
            pitch: 0.0,
            seed: match active {
                ActiveTickableEntitySound::Minecart { seed, .. }
                | ActiveTickableEntitySound::Bee { seed } => seed,
                ActiveTickableEntitySound::GuardianAttack { .. }
                | ActiveTickableEntitySound::SnifferDigging { .. } => unreachable!(),
            },
            looping: true,
            linear_attenuation: true,
            tickable: true,
        };
        self.client_audio.last_sound_entity = Some(state.clone());
        Some(state)
    }

    pub fn totem_use_sound_for_entity(&mut self, entity_id: i32) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(TOTEM_USE_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: transform.position,
            volume: 1.0,
            pitch: 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    /// Resolves vanilla `LivingEntity.breakItem` audio from the effective stack component.
    /// A synchronized patch wins, an explicit removal suppresses sound, and otherwise the
    /// source-generated item prototype default supplies the common/Shield/Wolf Armor event.
    pub fn living_entity_item_break_sound(
        &mut self,
        state: &LivingEntityItemParticleState,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        if self.entities.is_silent(state.entity_id)? {
            return None;
        }
        let patch = &state.item_stack.component_patch;
        let sound = if let Some(summary) = patch.break_sound.as_ref() {
            sound_event_summary_holder(summary)?
        } else if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID)
        {
            return None;
        } else {
            direct_sound_holder(vanilla_item_default_break_sound_resource_id(
                state.item_stack.item_id?,
            )?)
        };
        let entity_type_id = self.probe_entity_transform(state.entity_id)?.entity_type_id;
        let sound = SoundEventState {
            sound,
            source: vanilla_entity_sound_source(entity_type_id)
                .as_str()
                .to_string(),
            position: state.position,
            volume: 0.8,
            pitch: 0.8 + next_float().clamp(0.0, 1.0) * 0.4,
            seed: self.next_sound_seed(),
            distance_delay: false,
        };
        Some(self.record_positioned_sound(sound))
    }

    pub fn living_damage_hurt_sound_for_entity(
        &mut self,
        entity_id: i32,
        source_type_id: i32,
        effects: DamageEffects,
        next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        self.living_damage_hurt_sound_for_entity_with_optional_effects(
            entity_id,
            source_type_id,
            Some(effects),
            next_float,
        )
    }

    pub(crate) fn living_damage_hurt_sound_for_entity_with_optional_effects(
        &mut self,
        entity_id: i32,
        source_type_id: i32,
        effects: Option<DamageEffects>,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let policy = vanilla_entity_metadata_type_hurt_sound_policy(transform.entity_type_id)?;
        let sound = match policy {
            VanillaEntityHurtSoundPolicy::NonLiving => return None,
            VanillaEntityHurtSoundPolicy::Fixed(sound) => direct_sound_holder(sound),
            VanillaEntityHurtSoundPolicy::Conditional {
                condition,
                when_true,
                when_false,
            } => direct_sound_holder(if self.living_sound_condition(entity_id, condition)? {
                when_true
            } else {
                when_false
            }),
            VanillaEntityHurtSoundPolicy::NautilusAgeWater {
                baby_underwater,
                baby_land,
                adult_underwater,
                adult_land,
            } => {
                let baby = self.entities.is_baby(entity_id)?;
                let underwater = self.living_sound_fluid_state(entity_id).1;
                direct_sound_holder(match (baby, underwater) {
                    (true, true) => baby_underwater,
                    (true, false) => baby_land,
                    (false, true) => adult_underwater,
                    (false, false) => adult_land,
                })
            }
            VanillaEntityHurtSoundPolicy::CopperGolemOxidation {
                unaffected,
                exposed,
                weathered,
                oxidized,
            } => {
                let accessors = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
                    transform.entity_type_id,
                )?;
                let state = self.entities.metadata_int_for_entity(
                    entity_id,
                    accessors.weather_state_data_id,
                    accessors.weather_state_default,
                )?;
                direct_sound_holder(match state {
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID => unaffected,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID => exposed,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => weathered,
                    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => oxidized,
                    _ => return None,
                })
            }
            VanillaEntityHurtSoundPolicy::SoundVariant(family) => self.living_sound_variant(
                entity_id,
                transform.entity_type_id,
                family,
                RegistrySoundKind::Hurt,
            )?,
            VanillaEntityHurtSoundPolicy::WolfVariant { armor_damage } => {
                if self
                    .equipment_item(entity_id, EquipmentSlot::Body)
                    .and_then(|item| item.item_id)
                    == Some(VANILLA_ITEM_WOLF_ARMOR_ID)
                    && !self.registries().holder_is_in_tag(
                        DAMAGE_TYPE_REGISTRY,
                        "minecraft:bypasses_wolf_armor",
                        source_type_id,
                    )
                {
                    direct_sound_holder(armor_damage)
                } else {
                    self.living_damage_wolf_sound_variant(entity_id, transform.entity_type_id)?
                }
            }
            VanillaEntityHurtSoundPolicy::PlayerDamageType => {
                direct_sound_holder(effects?.sound_resource_id())
            }
        };
        let volume = self.living_sound_volume(entity_id, transform.entity_type_id)?;
        let state = SoundEventState {
            sound,
            source: self
                .living_sound_source(entity_id, transform.entity_type_id)?
                .as_str()
                .to_string(),
            position: transform.position,
            volume,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub(super) fn living_damage_wolf_sound_variant(
        &self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> Option<SoundHolderState> {
        self.living_sound_variant(
            entity_id,
            entity_type_id,
            VanillaEntitySoundVariantFamily::Wolf,
            RegistrySoundKind::Hurt,
        )
    }

    pub(super) fn living_sound_volume(&self, entity_id: i32, entity_type_id: i32) -> Option<f32> {
        match vanilla_entity_metadata_type_sound_volume_policy(entity_type_id)? {
            VanillaEntitySoundVolumePolicy::NonLiving => None,
            VanillaEntitySoundVolumePolicy::Fixed(bits) => Some(f32::from_bits(bits)),
            VanillaEntitySoundVolumePolicy::SlimeSize => {
                let (data_id, default) =
                    vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)?;
                Some(
                    0.4 * self
                        .entities
                        .metadata_int_for_entity(entity_id, data_id, default)?
                        as f32,
                )
            }
            VanillaEntitySoundVolumePolicy::HappyGhastAge => {
                Some(if self.entities.is_baby(entity_id)? {
                    1.0
                } else {
                    4.0
                })
            }
        }
    }

    pub(super) fn living_sound_condition(
        &self,
        entity_id: i32,
        condition: VanillaEntitySoundCondition,
    ) -> Option<bool> {
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        match condition {
            VanillaEntitySoundCondition::ArmadilloScared => {
                let (data_id, default) =
                    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
                        entity_type_id,
                    )?;
                Some(
                    self.entities
                        .metadata_int_for_entity(entity_id, data_id, default)?
                        != VANILLA_ARMADILLO_STATE_IDLE_ID,
                )
            }
            VanillaEntitySoundCondition::InWater => {
                Some(self.living_sound_fluid_state(entity_id).0)
            }
            VanillaEntitySoundCondition::UnderWater => {
                Some(self.living_sound_fluid_state(entity_id).1)
            }
            VanillaEntitySoundCondition::Baby => self.entities.is_baby(entity_id),
            VanillaEntitySoundCondition::GoatScreaming => {
                let accessors =
                    vanilla_entity_metadata_goat_accessors_for_entity_type(entity_type_id)?;
                self.entities.metadata_bool_for_entity(
                    entity_id,
                    accessors.screaming_data_id,
                    accessors.screaming_default,
                )
            }
            VanillaEntitySoundCondition::Tiny => {
                let (data_id, default) =
                    vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)?;
                Some(
                    self.entities
                        .metadata_int_for_entity(entity_id, data_id, default)?
                        <= 1,
                )
            }
            VanillaEntitySoundCondition::ShulkerClosed => {
                let accessors =
                    vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)?;
                Some(
                    self.entities.metadata_byte_for_entity(
                        entity_id,
                        accessors.peek_data_id,
                        accessors.peek_default,
                    )? == 0,
                )
            }
            VanillaEntitySoundCondition::CreakingHeartBound => {
                self.entities.creaking_is_heart_bound(entity_id)
            }
        }
    }

    pub(super) fn living_sound_fluid_state(&self, entity_id: i32) -> (bool, bool) {
        let Some(input) = self.entities.fluid_contact_input(entity_id) else {
            return (false, false);
        };
        let contact = crate::client::entity_fluid_contact(
            self,
            input.aabb_min,
            input.aabb_max,
            input.position.y,
            input.eye_position,
        );
        let in_water = contact.in_water();
        (in_water, in_water && contact.eye_in_water)
    }

    pub(super) fn living_sound_source(
        &self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> Option<SoundSource> {
        if vanilla_entity_sound_source_kind(entity_type_id)
            != Some(VanillaEntitySoundSourceKind::Dynamic)
        {
            return Some(vanilla_entity_sound_source(entity_type_id));
        }
        let (data_id, default) =
            crate::entities::vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
                entity_type_id,
            )?;
        Some(
            if self
                .entities
                .metadata_int_for_entity(entity_id, data_id, default)?
                == VANILLA_RABBIT_VARIANT_EVIL_ID
            {
                SoundSource::Hostile
            } else {
                SoundSource::Neutral
            },
        )
    }

    pub fn evoker_fangs_attack_sound_for_entity(
        &mut self,
        entity_id: i32,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID {
            return None;
        }
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(EVOKER_FANGS_ATTACK_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: transform.position,
            volume: 1.0,
            pitch: next_float().clamp(0.0, 1.0) * 0.2 + 0.85,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn zombie_villager_cure_sound_for_entity(
        &mut self,
        entity_id: i32,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID {
            return None;
        }
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        let pose = self.probe_entity_camera_pose(entity_id)?;
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(ZOMBIE_VILLAGER_CURE_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: ProtocolVec3d {
                x: pose.position.x,
                y: pose.position.y + f64::from(pose.eye_height),
                z: pose.position.z,
            },
            volume: 1.0 + next_float().clamp(0.0, 1.0),
            pitch: next_float().clamp(0.0, 1.0) * 0.7 + 0.3,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn armadillo_peek_sound_for_entity(&mut self, entity_id: i32) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ARMADILLO_ID {
            return None;
        }
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(ARMADILLO_PEEK_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: transform.position,
            volume: 1.0,
            pitch: 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn armor_stand_hit_sound_for_entity(&mut self, entity_id: i32) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            return None;
        }
        let source = vanilla_entity_sound_source(transform.entity_type_id);
        let state = SoundEventState {
            sound: direct_sound_holder(ARMOR_STAND_HIT_SOUND_EVENT),
            source: source.as_str().to_string(),
            position: transform.position,
            volume: 0.3,
            pitch: 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub fn kinetic_hit_sound_for_entity(
        &mut self,
        entity_id: i32,
    ) -> Option<SoundEntityEventState> {
        if self.entities.is_silent(entity_id)? {
            return None;
        }
        // Vanilla `LivingEntity.onKineticHit` (`LivingEntity.java:2124-2132`):
        // only triggers once every >10 ticks and reads `KineticWeapon.hitSound`
        // from the entity's active `useItem`.
        if !self.entities.can_trigger_kinetic_hit_feedback(entity_id) {
            return None;
        }
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        let active_stack = if self.local_player_id == Some(entity_id) {
            let hand = self
                .local_player
                .interaction
                .using_item_hand
                .or_else(|| self.entities.active_use_item_hand(entity_id))?;
            self.local_item_in_hand(hand).cloned()?
        } else {
            let hand = self.entities.active_use_item_hand(entity_id)?;
            let slot = match hand {
                bbb_protocol::packets::InteractionHand::MainHand => {
                    bbb_protocol::packets::EquipmentSlot::MainHand
                }
                bbb_protocol::packets::InteractionHand::OffHand => {
                    bbb_protocol::packets::EquipmentSlot::OffHand
                }
            };
            self.entities.equipment_item(entity_id, slot).cloned()?
        };
        if !self.item_stack_has_effective_component(
            &active_stack,
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        ) {
            return None;
        }
        let hit_sound = self
            .items
            .component_source(
                active_stack.item_id,
                &active_stack.component_patch,
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            )
            .kinetic_weapon
            .as_ref()
            .and_then(|weapon| weapon.hit_sound.as_ref())?;
        let sound = sound_event_summary_holder(hit_sound)?;
        let seed = self.next_sound_seed();
        let state = SoundEntityEventState {
            sound,
            source: vanilla_entity_sound_source(entity_type_id)
                .as_str()
                .to_string(),
            entity_id,
            volume: 1.0,
            pitch: 1.0,
            seed,
            looping: false,
            linear_attenuation: true,
            tickable: false,
        };
        self.client_audio.last_sound_entity = Some(state.clone());
        Some(state)
    }

    pub(crate) fn local_player_death_sound(
        &mut self,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let position = self.local_player_pose()?.position;
        let state = SoundEventState {
            sound: direct_sound_holder(PLAYER_DEATH_SOUND_EVENT),
            source: SoundSource::Players.as_str().to_string(),
            position,
            volume: 1.0,
            pitch: (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * 0.2 + 1.0,
            seed: 0,
            distance_delay: false,
        };
        Some(self.record_positioned_sound(state))
    }

    pub(super) fn living_sound_variant(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        family: VanillaEntitySoundVariantFamily,
        kind: RegistrySoundKind,
    ) -> Option<SoundHolderState> {
        let baby = self.entities.is_baby(entity_id)?;
        let (base_entity_type, serializer) = match family {
            VanillaEntitySoundVariantFamily::Cat => {
                (entity_type_id, EntityDataRegistryHolder::CatSoundVariant)
            }
            VanillaEntitySoundVariantFamily::Chicken => (
                VANILLA_ENTITY_TYPE_CHICKEN_ID,
                EntityDataRegistryHolder::ChickenSoundVariant,
            ),
            VanillaEntitySoundVariantFamily::Cow => (
                VANILLA_ENTITY_TYPE_COW_ID,
                EntityDataRegistryHolder::CowSoundVariant,
            ),
            VanillaEntitySoundVariantFamily::Pig => (
                VANILLA_ENTITY_TYPE_PIG_ID,
                EntityDataRegistryHolder::PigSoundVariant,
            ),
            VanillaEntitySoundVariantFamily::Wolf => {
                (entity_type_id, EntityDataRegistryHolder::WolfSoundVariant)
            }
        };
        let (data_id, default_resource_id) = match family {
            VanillaEntitySoundVariantFamily::Cat | VanillaEntitySoundVariantFamily::Wolf => {
                let accessors =
                    vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
                        base_entity_type,
                    )?;
                (
                    accessors.sound_variant_data_id,
                    accessors.sound_variant_default_resource_id,
                )
            }
            VanillaEntitySoundVariantFamily::Chicken
            | VanillaEntitySoundVariantFamily::Cow
            | VanillaEntitySoundVariantFamily::Pig => {
                let accessors =
                    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
                        base_entity_type,
                    )?;
                (
                    accessors.sound_variant_data_id,
                    accessors.sound_variant_default_resource_id,
                )
            }
        };
        let registry = serializer.registry();
        let metadata_reference = self
            .entities
            .metadata_registry_reference_for_entity(entity_id, data_id, serializer)?;
        let metadata_identity = match metadata_reference.as_ref() {
            Some(reference) => Some(reference.identity()?),
            None => None,
        };
        if let Some(identity) = metadata_identity {
            let revision = self.registry_access_revision();
            let identity_revision = identity.revision();
            if identity.registry() != registry
                || identity_revision.session != revision.session
                || identity_revision.contents != revision.contents
                || identity_revision.tags != revision.tags
            {
                return None;
            }
        }
        let resource_id = if self.registry_content(registry).is_some() {
            let holder_id = metadata_reference
                .as_ref()
                .map(|reference| reference.holder_id())
                .or_else(|| {
                    self.registry_holder_id_by_resource_id(registry, default_resource_id)
                })?;
            if let Some(sound) =
                self.synchronized_sound_variant_sound(registry, holder_id, baby, kind)
            {
                return Some(registry_sound_holder(sound));
            }
            let entry = self.registry_entry_by_holder_id(registry, holder_id)?;
            if entry.has_data || entry.known_pack_data().is_some() {
                return None;
            }
            entry.id.as_str()
        } else {
            metadata_identity
                .map(|identity| identity.resource_id())
                .unwrap_or(default_resource_id)
        };
        let sound = match family {
            VanillaEntitySoundVariantFamily::Cat => {
                let identity = CatSoundVariantIdentity::for_resource_id(resource_id)?;
                match kind {
                    RegistrySoundKind::Hurt => identity.builtin_hurt_sound_event(baby),
                    RegistrySoundKind::Death => identity.builtin_death_sound_event(baby),
                }
            }
            VanillaEntitySoundVariantFamily::Chicken => {
                let identity = ChickenSoundVariantIdentity::for_resource_id(resource_id)?;
                match kind {
                    RegistrySoundKind::Hurt => identity.builtin_hurt_sound_event(baby),
                    RegistrySoundKind::Death => identity.builtin_death_sound_event(baby),
                }
            }
            VanillaEntitySoundVariantFamily::Cow => {
                let identity = CowSoundVariantIdentity::for_resource_id(resource_id)?;
                match kind {
                    RegistrySoundKind::Hurt => identity.builtin_hurt_sound_event(baby),
                    RegistrySoundKind::Death => identity.builtin_death_sound_event(baby),
                }
            }
            VanillaEntitySoundVariantFamily::Pig => {
                let identity = PigSoundVariantIdentity::for_resource_id(resource_id)?;
                match kind {
                    RegistrySoundKind::Hurt => identity.builtin_hurt_sound_event(baby),
                    RegistrySoundKind::Death => identity.builtin_death_sound_event(baby),
                }
            }
            VanillaEntitySoundVariantFamily::Wolf => {
                let identity = WolfSoundVariantIdentity::for_resource_id(resource_id)?;
                match kind {
                    RegistrySoundKind::Hurt => identity.builtin_hurt_sound_event(baby),
                    RegistrySoundKind::Death => identity.builtin_death_sound_event(baby),
                }
            }
        };
        Some(direct_sound_holder(sound))
    }

    pub fn take_item_entity_pickup_sound_with_random(
        &self,
        item_id: i32,
        mut next_float: impl FnMut() -> f32,
    ) -> Option<SoundEventState> {
        let transform = self.entities.transform_state(item_id)?;
        let (sound, volume, pitch) =
            if transform.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
                (
                    EXPERIENCE_ORB_PICKUP_SOUND_EVENT,
                    0.1,
                    (next_float() - next_float()) * 0.35 + 0.9,
                )
            } else {
                (
                    ITEM_PICKUP_SOUND_EVENT,
                    0.2,
                    (next_float() - next_float()) * 1.4 + 2.0,
                )
            };
        Some(SoundEventState {
            sound: direct_sound_holder(sound),
            source: SoundSource::Players.as_str().to_string(),
            position: transform.position,
            volume,
            pitch,
            seed: 0,
            distance_delay: false,
        })
    }
}
