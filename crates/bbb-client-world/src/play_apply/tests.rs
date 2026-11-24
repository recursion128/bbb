mod death_and_item_break_events;
mod entity_particles_and_local_events;
mod packet_application;
mod stream_and_audio_events;

use super::entities::{
    HONEY_BLOCK_JUMP_PARTICLE_COUNT, HONEY_BLOCK_SLIDE_PARTICLE_COUNT,
    TOTEM_TRACKING_EMITTER_LIFETIME_TICKS, TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS,
};
use super::*;
use std::collections::BTreeMap;

use crate::entities::{
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_type_death_sound_policy, VanillaEntityDeathSoundPolicy,
};
use crate::{
    DamageEffects, ExplosionEventState, LocalInteractionSoundAction, LocalPlayerPoseState,
    RegistryPacketEntry, VillagerParticleKind, DAMAGE_TYPE_REGISTRY,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID, VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_ARMADILLO_ID,
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_BEE_ID,
    VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_EGG_ID,
    VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_GUARDIAN_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_RABBIT_ID,
    VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_SKELETON_ID,
    VANILLA_ENTITY_TYPE_SNIFFER_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
    VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    VANILLA_ENTITY_TYPE_STRAY_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_WITCH_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
};
use bbb_protocol::game_event_types::{
    VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
    VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID,
    VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_SHIELD_ID, VANILLA_ITEM_SPYGLASS_ID, VANILLA_ITEM_STONE_ID,
};
use bbb_protocol::packets::SetPlayerInventory;
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_CLOUD_ID, VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
};

// Wire-event fixture ids. Production dispatch consumes the source-generated handler catalog
// through `VanillaEntityEventKind` and does not own a parallel numeric table.
const ARROW_EFFECT_CLEAR_EVENT_ID: i8 = 0;
const RABBIT_JUMP_EVENT_ID: i8 = 1;
const ATTACK_SOUND_EVENT_ID: i8 = 4;
const THROWN_ITEM_HIT_EVENT_ID: i8 = 3;
const LIVING_ENTITY_DEATH_EVENT_ID: i8 = 3;
const TAMING_FAILED_EVENT_ID: i8 = 6;
const TAMING_SUCCEEDED_EVENT_ID: i8 = 7;
const VILLAGER_HEART_EVENT_ID: i8 = 12;
const VILLAGER_ANGRY_EVENT_ID: i8 = 13;
const VILLAGER_HAPPY_EVENT_ID: i8 = 14;
const WITCH_MAGIC_EVENT_ID: i8 = 15;
const ZOMBIE_VILLAGER_CURE_EVENT_ID: i8 = 16;
const ANIMAL_LOVE_EVENT_ID: i8 = 18;
const MOB_SPAWN_EVENT_ID: i8 = 20;
const GUARDIAN_ATTACK_SOUND_EVENT_ID: i8 = 21;
const FISHING_HOOK_PULL_EVENT_ID: i8 = 31;
const ARMOR_STAND_HIT_EVENT_ID: i8 = 32;
const DOLPHIN_HAPPY_EVENT_ID: i8 = 38;
const OCELOT_TRUSTING_FAILED_EVENT_ID: i8 = 40;
const OCELOT_TRUSTING_SUCCEEDED_EVENT_ID: i8 = 41;
const VILLAGER_SPLASH_EVENT_ID: i8 = 42;
const FOX_EAT_EVENT_ID: i8 = 45;
const LIVING_ENTITY_PORTAL_EVENT_ID: i8 = 46;
const LIVING_ENTITY_MAIN_HAND_BREAK_EVENT_ID: i8 = 47;
const LIVING_ENTITY_OFF_HAND_BREAK_EVENT_ID: i8 = 48;
const LIVING_ENTITY_HEAD_BREAK_EVENT_ID: i8 = 49;
const LIVING_ENTITY_CHEST_BREAK_EVENT_ID: i8 = 50;
const LIVING_ENTITY_LEGS_BREAK_EVENT_ID: i8 = 51;
const LIVING_ENTITY_FEET_BREAK_EVENT_ID: i8 = 52;
const HONEY_BLOCK_SLIDE_EVENT_ID: i8 = 53;
const HONEY_BLOCK_JUMP_EVENT_ID: i8 = 54;
const LIVING_ENTITY_POOF_EVENT_ID: i8 = 60;
const SNIFFER_DIGGING_SOUND_EVENT_ID: i8 = 63;
const ARMADILLO_PEEK_EVENT_ID: i8 = 64;
const LIVING_ENTITY_BODY_BREAK_EVENT_ID: i8 = 65;
const LIVING_ENTITY_DROWN_EVENT_ID: i8 = 67;
const LIVING_ENTITY_SADDLE_BREAK_EVENT_ID: i8 = 68;
const RAVAGER_ROAR_EVENT_ID: i8 = 69;

use bbb_protocol::packets::{
    AddEntity, BlockPos as ProtocolBlockPos, BlockUpdate, DamageEvent, DeleteChat, EntityAnimation,
    EntityDataRegistryHolder, EntityDataValue, EntityDataValueKind, EntityEvent,
    EntityPositionSync, EquipmentSlot, EquipmentSlotUpdate, Explosion, GameEvent, ItemStackSummary,
    PackedMessageSignature, ParticlePayload, PlayTime, PlayerCombatKill, PlayerHealth,
    RemoveEntities, SetEntityData, SetEquipment, SetPassengers, SoundEvent, SoundEventHolder,
    SoundEventSummary, SoundSource, TakeItemEntity, TeleportEntity, Vec3d,
    WeightedExplosionParticle,
};
use uuid::Uuid;

#[derive(Default)]
struct RecordingEffects {
    positioned_sounds: Vec<SoundEventState>,
    explosions: Vec<ExplosionEventState>,
    explosion_callback_delta_movements: Vec<Option<Vec3d>>,
    entity_sounds: Vec<(SoundEntityEventState, Option<[f64; 3]>)>,
    llama_spit_recreation_particles: Vec<LlamaSpitRecreationParticleState>,
    llama_spit_world_entity_visible: Vec<bool>,
    elder_guardian_effect_particles: Vec<Vec3d>,
    ravager_roar_particles: Vec<RavagerRoarParticleState>,
    witch_magic_particles: Vec<WitchMagicParticleState>,
    living_entity_poof_particles: Vec<LivingEntityPoofParticleState>,
    living_entity_item_particles: Vec<LivingEntityItemParticleState>,
    living_entity_drown_particles: Vec<LivingEntityDrownParticleState>,
    living_entity_portal_particles: Vec<LivingEntityPortalParticleState>,
    arrow_effect_particles: Vec<ArrowEffectParticleState>,
    entity_taming_particles: Vec<EntityTamingParticleState>,
    villager_particles: Vec<VillagerParticleState>,
    dolphin_happy_particles: Vec<DolphinHappyParticleState>,
    fox_eat_particles: Vec<FoxEatParticleState>,
    animal_love_particles: Vec<AnimalLoveParticleState>,
    allay_duplication_particles: Vec<AllayDuplicationParticleState>,
    snowball_hit_particles: Vec<SnowballHitParticleState>,
    thrown_egg_hit_particles: Vec<ThrownEggHitParticleState>,
    honey_block_particles: Vec<HoneyBlockParticleState>,
    rabbit_sprint_particles: Vec<RabbitSprintParticleState>,
    rabbit_sprint_world_hop_active: Vec<bool>,
    tracking_emitters: Vec<(i32, EntityTrackingEmitterParticleKind, u32)>,
    vehicle_move_reports: Vec<VehicleMoveReport>,
    removed_vehicle_teleports: Vec<RemovedPlayerVehicleTeleportRecovery>,
    chat_disconnects: Vec<ChatDisconnectReason>,
    perform_respawns: usize,
}

impl PlayApplyEffects for RecordingEffects {
    fn positioned_sound(&mut self, state: &SoundEventState) {
        self.positioned_sounds.push(state.clone());
    }

    fn explosion_particles(&mut self, world: &WorldStore, state: &ExplosionEventState) {
        self.explosions.push(state.clone());
        self.explosion_callback_delta_movements
            .push(world.local_player_pose().map(|pose| pose.delta_movement));
    }

    fn entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>) {
        self.entity_sounds.push((state.clone(), position));
    }

    fn llama_spit_recreation_particles(
        &mut self,
        world: &WorldStore,
        state: LlamaSpitRecreationParticleState,
    ) {
        self.llama_spit_recreation_particles.push(state);
        self.llama_spit_world_entity_visible
            .push(world.entity_type_id(state.entity_id) == Some(VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID));
    }

    fn removed_player_vehicle_teleport(&mut self, recovery: RemovedPlayerVehicleTeleportRecovery) {
        self.removed_vehicle_teleports.push(recovery);
    }

    fn vehicle_move_report(&mut self, report: VehicleMoveReport) {
        self.vehicle_move_reports.push(report);
    }

    fn chat_disconnect(&mut self, reason: ChatDisconnectReason) {
        self.chat_disconnects.push(reason);
    }

    fn perform_respawn(&mut self) {
        self.perform_respawns += 1;
    }

    fn elder_guardian_effect_particles(&mut self, _world: &WorldStore, position: Vec3d) {
        self.elder_guardian_effect_particles.push(position);
    }

    fn ravager_roar_particles(&mut self, _world: &WorldStore, state: RavagerRoarParticleState) {
        self.ravager_roar_particles.push(state);
    }

    fn witch_magic_particles(&mut self, _world: &WorldStore, state: WitchMagicParticleState) {
        self.witch_magic_particles.push(state);
    }

    fn living_entity_poof_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityPoofParticleState,
    ) {
        self.living_entity_poof_particles.push(state);
    }

    fn living_entity_item_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityItemParticleState,
    ) {
        self.living_entity_item_particles.push(state);
    }

    fn living_entity_drown_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityDrownParticleState,
    ) {
        self.living_entity_drown_particles.push(state);
    }

    fn living_entity_portal_particles(
        &mut self,
        _world: &WorldStore,
        state: LivingEntityPortalParticleState,
    ) {
        self.living_entity_portal_particles.push(state);
    }

    fn arrow_effect_particles(&mut self, _world: &WorldStore, state: ArrowEffectParticleState) {
        self.arrow_effect_particles.push(state);
    }

    fn entity_taming_particles(&mut self, _world: &WorldStore, state: EntityTamingParticleState) {
        self.entity_taming_particles.push(state);
    }

    fn villager_particles(&mut self, _world: &WorldStore, state: VillagerParticleState) {
        self.villager_particles.push(state);
    }

    fn dolphin_happy_particles(&mut self, _world: &WorldStore, state: DolphinHappyParticleState) {
        self.dolphin_happy_particles.push(state);
    }

    fn fox_eat_particles(&mut self, _world: &WorldStore, state: FoxEatParticleState) {
        self.fox_eat_particles.push(state);
    }

    fn animal_love_particles(&mut self, _world: &WorldStore, state: AnimalLoveParticleState) {
        self.animal_love_particles.push(state);
    }

    fn allay_duplication_particles(
        &mut self,
        _world: &WorldStore,
        state: AllayDuplicationParticleState,
    ) {
        self.allay_duplication_particles.push(state);
    }

    fn snowball_hit_particles(&mut self, _world: &WorldStore, state: SnowballHitParticleState) {
        self.snowball_hit_particles.push(state);
    }

    fn thrown_egg_hit_particles(&mut self, _world: &WorldStore, state: ThrownEggHitParticleState) {
        self.thrown_egg_hit_particles.push(state);
    }

    fn honey_block_particles(&mut self, _world: &WorldStore, state: HoneyBlockParticleState) {
        self.honey_block_particles.push(state);
    }

    fn rabbit_sprint_particles(&mut self, world: &WorldStore, state: RabbitSprintParticleState) {
        self.rabbit_sprint_particles.push(state);
        self.rabbit_sprint_world_hop_active.push(
            world
                .probe_entity(state.entity_id)
                .is_some_and(|entity| entity.client_animations.rabbit_kind().is_some()),
        );
    }

    fn tracking_emitter_particles(
        &mut self,
        _world: &WorldStore,
        entity_id: i32,
        kind: EntityTrackingEmitterParticleKind,
        lifetime_ticks: u32,
    ) {
        self.tracking_emitters
            .push((entity_id, kind, lifetime_ticks));
    }
}

#[test]
fn player_combat_kill_routes_immediate_respawn_effect_once() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.gameplay.show_death_screen = false;
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();

    assert!(store
        .apply_play_packet(
            PlayClientbound::PlayerCombatKill(PlayerCombatKill {
                player_id: 8,
                message: bbb_protocol::RichTextComponent::plain("Remote"),
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert_eq!(effects.perform_respawns, 0);

    assert!(store
        .apply_play_packet(
            PlayClientbound::PlayerCombatKill(PlayerCombatKill {
                player_id: 7,
                message: bbb_protocol::RichTextComponent::plain("Local"),
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert_eq!(effects.perform_respawns, 1);

    let mut no_effects_store = WorldStore::new();
    no_effects_store.local_player_id = Some(7);
    no_effects_store.gameplay.show_death_screen = false;
    let mut no_effects = NoPlayApplyEffects;
    assert!(no_effects_store
        .apply_play_packet(
            PlayClientbound::PlayerCombatKill(PlayerCombatKill {
                player_id: 7,
                message: bbb_protocol::RichTextComponent::plain("State only"),
            }),
            &mut random,
            &mut no_effects,
        )
        .is_none());
    assert!(no_effects_store.screen_requests().is_empty());
}

fn add_entity(entity_id: i32, entity_type_id: i32, position: Vec3d) -> AddEntity {
    AddEntity {
        id: entity_id,
        uuid: Uuid::from_u128(entity_id as u128),
        entity_type_id,
        position,
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn single_block_state_chunk(block_state_id: i32) -> crate::ChunkColumn {
    let single_value = |domain, entry_count, global_id| crate::PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: crate::PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    };
    crate::ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: crate::ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(crate::ChunkSection {
            non_empty_block_count: 4096,
            fluid_count: 0,
            block_states: single_value(crate::PaletteDomain::BlockStates, 4096, block_state_id),
            biomes: single_value(crate::PaletteDomain::Biomes, 64, 0),
        })],
        block_entities: std::sync::Arc::default(),
        light: crate::LightData::default(),
    }
}

#[test]
fn add_entity_emits_post_add_sounds_and_llama_spit_recreation_effects() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let position = Vec3d {
        x: 1.25,
        y: 64.5,
        z: -2.75,
    };
    let mut spit = add_entity(301, VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, position);
    spit.delta_movement = Vec3d {
        x: 0.5,
        y: -0.25,
        z: 0.75,
    };

    for packet in [
        PlayClientbound::AddEntity(spit),
        PlayClientbound::AddEntity(add_entity(
            302,
            VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
            position,
        )),
        PlayClientbound::AddEntity(add_entity(303, VANILLA_ENTITY_TYPE_BEE_ID, position)),
    ] {
        assert!(store
            .apply_play_packet(packet, &mut random, &mut effects)
            .is_none());
    }

    assert_eq!(
        effects.llama_spit_recreation_particles,
        vec![LlamaSpitRecreationParticleState {
            entity_id: 301,
            position: crate::EntityVec3 {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            delta_movement: crate::EntityVec3 {
                x: 0.5,
                y: -0.25,
                z: 0.75,
            },
        }]
    );
    assert_eq!(effects.llama_spit_world_entity_visible, vec![true]);
    assert_eq!(effects.entity_sounds.len(), 2);
    assert_eq!(
        effects.entity_sounds[0].0.sound.location.as_deref(),
        Some("minecraft:entity.minecart.riding")
    );
    assert_eq!(
        effects.entity_sounds[1].0.sound.location.as_deref(),
        Some("minecraft:entity.bee.loop")
    );
    for (sound, sound_position) in &effects.entity_sounds {
        assert_eq!(sound.source, "neutral");
        assert_eq!(sound.volume, 0.0);
        assert_eq!(sound.pitch, 0.0);
        assert!(sound.looping);
        assert!(sound.linear_attenuation);
        assert!(sound.tickable);
        assert_eq!(*sound_position, Some([position.x, position.y, position.z]));
    }
    assert_eq!(store.tickable_entity_sounds().len(), 2);

    assert!(store
        .apply_play_packet(
            PlayClientbound::AddEntity(add_entity(304, VANILLA_ENTITY_TYPE_PLAYER_ID, position,)),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert!(store.probe_entity(304).is_none());
    assert_eq!(effects.entity_sounds.len(), 2);
    assert_eq!(effects.llama_spit_recreation_particles.len(), 1);

    assert!(store
        .apply_play_packet(
            PlayClientbound::RemoveEntities(RemoveEntities {
                entity_ids: vec![302],
            }),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert_eq!(store.tickable_entity_sounds().len(), 1);
    assert!(store
        .apply_play_packet(
            PlayClientbound::AddEntity(add_entity(303, VANILLA_ENTITY_TYPE_ZOMBIE_ID, position,)),
            &mut random,
            &mut effects,
        )
        .is_none());
    assert!(store.tickable_entity_sounds().is_empty());
    assert_eq!(effects.entity_sounds.len(), 2);
}

fn record_packet_player_info(store: &mut WorldStore, entity_id: i32) {
    let uuid = Uuid::from_u128(entity_id as u128);
    assert_eq!(
        store.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
            actions: vec![bbb_protocol::packets::PlayerInfoAction::AddPlayer],
            entries: vec![bbb_protocol::packets::PlayerInfoEntry {
                profile_id: uuid,
                profile: Some(bbb_protocol::packets::GameProfile {
                    uuid,
                    name: format!("remote-{entity_id}"),
                    properties: Vec::new(),
                }),
                listed: false,
                latency: 0,
                game_mode: bbb_protocol::game_types::GameType::default(),
                display_name: None,
                show_hat: false,
                list_order: 0,
                chat_session: None,
            }],
        }),
        1
    );
}

fn damage_type_nbt(message_id: &str, effects: &str) -> Vec<u8> {
    let mut out = vec![10];
    write_nbt_string(&mut out, "message_id", message_id);
    write_nbt_string(&mut out, "scaling", "never");
    out.push(5);
    write_nbt_text(&mut out, "exhaustion");
    out.extend_from_slice(&0.0_f32.to_be_bytes());
    write_nbt_string(&mut out, "effects", effects);
    out.push(0);
    out
}

fn write_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    write_nbt_text(out, name);
    write_nbt_text(out, value);
}

fn write_nbt_text(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn item_stack_entity_data(entity_type_id: i32, item: ItemStackSummary) -> EntityDataValue {
    let accessor =
        crate::entities::vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
            entity_type_id,
            "ITEM_STACK",
        )
        .expect("generated unique item-stack accessor must resolve");
    EntityDataValue {
        data_id: accessor.data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        value: EntityDataValueKind::ItemStack(item),
    }
}

fn int_entity_data(data_id: u8, value: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(value),
    }
}

fn bool_entity_data(data_id: u8, value: bool) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    }
}

fn sniffer_state_entity_data(state_id: i32) -> EntityDataValue {
    let (data_id, _) =
        crate::entities::vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
        )
        .expect("generated Sniffer state accessor must resolve");
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
        value: EntityDataValueKind::EnumId {
            serializer: bbb_protocol::packets::EntityDataEnumSerializer::SnifferState,
            id: state_id,
        },
    }
}

fn silent_entity_data(value: bool) -> EntityDataValue {
    let accessors = crate::entities::vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert!(!accessors.silent_default);
    bool_entity_data(accessors.silent_data_id, value)
}

fn cow_sound_variant_data(id: i32) -> EntityDataValue {
    let accessors =
        crate::entities::vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_COW_ID,
        )
        .expect("generated Cow variant metadata accessors must resolve");
    EntityDataValue {
        data_id: accessors.sound_variant_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::CowSoundVariant,
            value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(id),
        },
    }
}

fn cow_sound_variant_nbt() -> Vec<u8> {
    let mut out = vec![10];
    for field in ["ambient_sound", "hurt_sound", "death_sound", "step_sound"] {
        if field == "death_sound" {
            begin_test_nbt_compound(&mut out, field);
            write_test_nbt_string(&mut out, "sound_id", "example:entity.cow.death");
            out.push(5);
            write_test_nbt_utf(&mut out, "range");
            out.extend_from_slice(&20.0_f32.to_be_bytes());
            out.push(0);
        } else {
            write_test_nbt_string(&mut out, field, "minecraft:entity.cow.ambient");
        }
    }
    out.push(0);
    out
}

fn begin_test_nbt_compound(out: &mut Vec<u8>, name: &str) {
    out.push(10);
    write_test_nbt_utf(out, name);
}

fn write_test_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    write_test_nbt_utf(out, name);
    write_test_nbt_utf(out, value);
}

fn write_test_nbt_utf(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn vec3(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}

fn assert_vec3_close(actual: Vec3d, expected: Vec3d) {
    assert!(
        (actual.x - expected.x).abs() < 1.0e-9,
        "x: actual {:?}, expected {:?}",
        actual,
        expected
    );
    assert!(
        (actual.y - expected.y).abs() < 1.0e-9,
        "y: actual {:?}, expected {:?}",
        actual,
        expected
    );
    assert!(
        (actual.z - expected.z).abs() < 1.0e-9,
        "z: actual {:?}, expected {:?}",
        actual,
        expected
    );
}

fn ravager_roar_world(player_position: Vec3d, delta_movement: Vec3d, health: f32) -> WorldStore {
    let mut store = WorldStore::new();
    store.apply_add_entity(add_entity(
        7,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        player_position,
    ));
    store.apply_add_entity(add_entity(
        76,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
        vec3(0.0, 64.0, 0.0),
    ));
    store.local_player_id = Some(7);
    store.apply_player_health(PlayerHealth {
        health,
        food: 20,
        saturation: 5.0,
    });
    store.set_local_player_pose(LocalPlayerPoseState {
        position: player_position,
        delta_movement,
        ..LocalPlayerPoseState::default()
    });
    store
}

fn apply_ravager_roar_entity_event(store: &mut WorldStore) -> RecordingEffects {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let mut effects = RecordingEffects::default();
    let leftover = store.apply_play_packet(
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 76,
            event_id: RAVAGER_ROAR_EVENT_ID,
        }),
        &mut random,
        &mut effects,
    );
    assert!(leftover.is_none());
    effects
}
