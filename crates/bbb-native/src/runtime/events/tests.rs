use super::*;
use crate::audio_runtime::resolver::{AudioCommandResolver, AudioResolveError};
use crate::particle_runtime::{
    LevelEventDripstoneDripParticle, LevelEventGrowthParticleContext, LevelEventGrowthParticleMode,
    LevelEventGrowthParticleSupport, LevelEventParticleContext, LevelParticleSpawnContext,
    ParticleEventSink,
};
use crate::runtime::{clear_color_for_day_time, clear_color_for_world};
use bbb_audio::{AudioCategory, AudioCommand};
use bbb_control::{AudioCounters, NetCounters};
use bbb_net::{NetCommand, NetEvent};
use bbb_pack::{JukeboxSongRegistry, SoundCatalog, SoundEventRegistry};
use bbb_protocol::codec::Encoder;
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::PlayClientbound;
use bbb_protocol::packets::{
    AddEntity, AdvancementCriterionProgressSummary, AdvancementProgressSummary, AdvancementSummary,
    AttributeSnapshot, AwardStats, BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate,
    ChatTypeBound, ChatTypeHolder, ChunkBiomeData, ChunkHeightmapData,
    ChunkPos as ProtocolChunkPos, ChunksBiomes, CommonPlayerSpawnInfo, ContainerClose,
    ContainerSetContent, ContainerSetData, ContainerSetSlot, CustomChatCompletions,
    CustomChatCompletionsAction, CustomPayload, CustomPayloadBody, DataComponentPatchSummary,
    DebugBlockValue, DebugChunkValue, DebugEntityValue, DebugEvent, DebugSample, DeleteChat,
    DialogHolder, DisguisedChat, EntityAnchor, EntityAnimation, EntityDataValue,
    EntityDataValueKind, EntityEvent, EntityMove, EntityPositionSync, EquipmentSlot,
    EquipmentSlotUpdate, Explosion, FilterMask, FilterMaskKind, FireworkExplosionShapeSummary,
    FireworkExplosionSummary, ForgetLevelChunk, GameRuleValue, GameRuleValues,
    GameTestHighlightPos, HurtAnimation, IngredientSummary, InteractionHand, ItemCostSummary,
    ItemStackSummary, LevelChunkBlockEntity, LevelChunkData, LevelChunkWithLight, LevelEvent,
    LevelParticles, LightUpdate, LightUpdateData, MapColorPatch, MapDecoration, MapItemData,
    MerchantOffer, MerchantOffers, MessageSignature, MinecartStep, MountScreenOpen,
    MoveMinecartAlongTrack, OpenBook, OpenScreen, OpenSignEditor, PackedMessageSignature,
    ParticlePayload, PlaceGhostRecipe, PlayLogin, PlayerChat, PlayerCombatEnd, PlayerCombatKill,
    PlayerLookAt, PlayerLookAtTarget, PlayerPositionUpdate, PlayerRotationUpdate, PongResponse,
    ProjectilePower, RecipeBookAdd, RecipeBookAddEntry, RecipeBookRemove, RecipeBookSettings,
    RecipeBookTypeSettings, RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipeDisplayType, RecipePropertySetSummary, RegistryData, RegistryDataEntry, RegistryTags,
    RemoteDebugSampleType, RemoveEntities, Respawn, RotateHead, SectionBlocksUpdate,
    SelectAdvancementsTab, ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks,
    SetChunkCacheCenter, SetChunkCacheRadius, SetCursorItem, SetEntityData, SetEntityLink,
    SetEntityMotion, SetEquipment, SetPassengers, SetPlayerInventory, ShowDialog,
    SignedMessageBody, SlotDisplaySummary, SoundEntityEvent, SoundEvent, SoundEventHolder,
    SoundSource, StatUpdate, StonecutterSelectableRecipeSummary, StopSound, TagNetworkPayload,
    TagQuery, TeleportEntity, TestInstanceBlockStatus, TrackedWaypoint, TrackedWaypointPacket,
    UpdateAdvancements, UpdateAttributes, UpdateRecipes, UpdateTags, Vec3d as ProtocolVec3d,
    Vec3i as ProtocolVec3i, WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation,
    WaypointVec3i, WrittenBookContentSummary,
};
use bbb_world::{
    advance_cobweb_place_particle_randoms, AllayDuplicationParticleState, AnimalLoveParticleState,
    ArrowEffectParticleState, BlockPos, ChunkPos, DolphinHappyParticleState,
    EntityTamingParticleState, FireworkRocketExplosionParticleState,
    FireworkRocketTrailParticleState, FoxEatParticleState, HoneyBlockParticleState,
    LivingEntityDrownParticleState, LivingEntityPoofParticleState, LivingEntityPortalParticleState,
    LocalPlayerPoseState, OminousItemSpawnerParticleState, RavagerRoarParticleState,
    RegistryPacketEntry, SnowballHitParticleState, TakeItemEntityPickupParticleState,
    ThrownEggHitParticleState, VillagerParticleKind, VillagerParticleState,
    WitchMagicParticleState, WorldBlockSoundProfile, WorldStore,
};
use std::collections::BTreeMap;
use tokio::sync::mpsc;
use uuid::Uuid;

fn advance_growth_randoms_for_context(
    event: &bbb_protocol::packets::LevelEvent,
    context: &LevelEventParticleContext,
    random: &mut LevelEventSoundRandomState,
) {
    if let Some(growth) = &context.growth_particles {
        let mode = match growth.mode {
            LevelEventGrowthParticleMode::InBlock { .. } => {
                bbb_world::LevelEventGrowthRandomMode::InBlock
            }
            LevelEventGrowthParticleMode::WideNoFloating { .. } => {
                bbb_world::LevelEventGrowthRandomMode::WideNoFloating
            }
        };
        advance_growth_level_event_particle_randoms(event.data, mode, random);
    }
}

mod audio_and_death_sounds;
mod entity_and_inventory_counters;
mod entity_particle_events;
mod level_event_block_and_potion_sounds;
mod level_event_particle_batches;
mod session_hud_and_vehicle_events;
mod session_protocol_events;
mod trial_spawner_and_sculk_events;
mod world_state_lifecycle;

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn firework_item_entity_data(entity_id: i32, explosions_count: Option<usize>) -> SetEntityData {
    let mut stack = item_stack(901, 1);
    stack.component_patch = DataComponentPatchSummary {
        fireworks_flight_duration: Some(1),
        fireworks_explosions_count: explosions_count,
        ..DataComponentPatchSummary::default()
    };
    SetEntityData {
        id: entity_id,
        values: vec![EntityDataValue {
            data_id: 8,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(stack),
        }],
    }
}

fn firework_item_entity_data_with_explosions(
    entity_id: i32,
    explosions: Vec<FireworkExplosionSummary>,
) -> SetEntityData {
    let mut stack = item_stack(901, 1);
    stack.component_patch = DataComponentPatchSummary {
        fireworks_flight_duration: Some(1),
        fireworks_explosions_count: Some(explosions.len()),
        fireworks_explosions: explosions,
        ..DataComponentPatchSummary::default()
    };
    SetEntityData {
        id: entity_id,
        values: vec![EntityDataValue {
            data_id: 8,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(stack),
        }],
    }
}

fn written_book_stack(pages: Vec<&str>) -> ItemStackSummary {
    let mut stack = item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".to_string(),
        title_filter: None,
        author: "Alex".to_string(),
        generation: 0,
        pages,
        page_filters,
        resolved: true,
    });
    stack
}

fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

fn recipe_book_entry(id: i32, notification: bool, highlight: bool) -> RecipeBookAddEntry {
    RecipeBookAddEntry {
        contents: RecipeDisplayEntry {
            id: RecipeDisplayId { index: id },
            display: RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 0, 0, 0],
                crafting: None,
                furnace: None,
            },
            group: None,
            category_id: 10,
            crafting_requirements: Some(vec![IngredientSummary {
                tag: None,
                item_ids: vec![42],
            }]),
        },
        flags: (u8::from(notification)) | (u8::from(highlight) << 1),
        notification,
        highlight,
    }
}

fn protocol_play_login(player_id: i32) -> PlayLogin {
    PlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    }
}

fn command_tree_packet(literal: &str) -> bbb_protocol::packets::Commands {
    bbb_protocol::packets::Commands {
        root_index: 0,
        nodes: vec![
            bbb_protocol::packets::CommandNode {
                node_type: bbb_protocol::packets::CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            bbb_protocol::packets::CommandNode {
                node_type: bbb_protocol::packets::CommandNodeType::Literal,
                flags: 1,
                children: vec![2],
                redirect: None,
                name: Some(literal.to_string()),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            bbb_protocol::packets::CommandNode {
                node_type: bbb_protocol::packets::CommandNodeType::Argument,
                flags: 54,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(bbb_protocol::packets::CommandArgumentParser {
                    type_id: 5,
                    name: "brigadier:string".to_string(),
                    properties: vec![2],
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: true,
            },
        ],
    }
}

fn protocol_chat_type(name: &str) -> ChatTypeBound {
    ChatTypeBound {
        chat_type: ChatTypeHolder::Registry { id: 0 },
        name: name.to_string(),
        target_name: None,
    }
}

fn player_chat_with_signature(global_index: i32, signature: MessageSignature) -> PlayerChat {
    PlayerChat {
        global_index,
        sender: Uuid::from_u128(0x1234),
        index: global_index,
        signature: Some(signature),
        body: SignedMessageBody {
            content: format!("message {global_index}"),
            timestamp_millis: i64::from(global_index),
            salt: i64::from(global_index) + 1,
            last_seen: Vec::new(),
        },
        unsigned_content: None,
        filter_mask: FilterMask {
            kind: FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: protocol_chat_type("Alice"),
    }
}

struct RecordingAudioSink {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    jukebox_registry: JukeboxSongRegistry,
    commands: Vec<AudioCommand>,
    errors: Vec<String>,
}

impl RecordingAudioSink {
    fn new(catalog: SoundCatalog, registry: SoundEventRegistry) -> Self {
        Self {
            catalog,
            registry,
            jukebox_registry: JukeboxSongRegistry::vanilla_26_1(),
            commands: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn record(&mut self, command: std::result::Result<AudioCommand, AudioResolveError>) {
        match command {
            Ok(command) => self.commands.push(command),
            Err(err) => self.errors.push(err.to_string()),
        }
    }
}

impl crate::audio_runtime::AudioEventSink for RecordingAudioSink {
    fn counters(&self) -> AudioCounters {
        AudioCounters {
            enabled: true,
            catalog_events: self.catalog.len(),
            registry_entries: self.registry.len(),
            commands_submitted: self.commands.len() as u64,
            resolve_failures: self.errors.len() as u64,
            last_resolve_error: self.errors.last().cloned(),
            ..AudioCounters::default()
        }
    }

    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry) {
        self.registry = registry;
    }

    fn set_jukebox_song_registry(&mut self, registry: JukeboxSongRegistry) {
        self.jukebox_registry = registry;
    }

    fn play_local_sound(&mut self, state: &bbb_world::LocalSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_local_sound(state)
        };
        self.record(command);
    }

    fn play_positioned_sound(&mut self, state: &bbb_world::SoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_positioned_sound(state)
        };
        self.record(command);
    }

    fn play_entity_sound(
        &mut self,
        state: &bbb_world::SoundEntityEventState,
        position: Option<[f64; 3]>,
    ) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_entity_sound_at(state, position)
        };
        self.record(command);
    }

    fn play_jukebox_song(&mut self, state: &bbb_world::JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_jukebox_song(state)
        };
        self.record(command);
    }

    fn stop_jukebox_song(&mut self, state: &bbb_world::JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_jukebox_song(state)
        };
        self.commands.push(command);
    }

    fn stop_sound(&mut self, state: &bbb_world::StopSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_sound(state)
        };
        self.commands.push(command);
    }

    fn tick_entity_sound_positions(&mut self, command: bbb_audio::TickEntitySoundPositionsCommand) {
        self.commands
            .push(AudioCommand::TickEntitySoundPositions(command));
    }
}

#[derive(Default)]
struct RecordingParticleSink {
    packets: Vec<LevelParticles>,
    contexts: Vec<LevelParticleSpawnContext>,
    level_events: Vec<LevelEvent>,
    level_event_contexts: Vec<LevelEventParticleContext>,
    firework_empty_explosion_positions: Vec<[f64; 3]>,
    firework_empty_explosion_camera_positions: Vec<Option<[f64; 3]>>,
    firework_explosion_states: Vec<FireworkRocketExplosionParticleState>,
    firework_explosion_camera_positions: Vec<Option<[f64; 3]>>,
    firework_rocket_trail_states: Vec<FireworkRocketTrailParticleState>,
    ominous_item_spawner_states: Vec<OminousItemSpawnerParticleState>,
    tracking_emitter_states: Vec<crate::particle_runtime::TrackingEmitterParticleState>,
    take_item_entity_pickup_states: Vec<TakeItemEntityPickupParticleState>,
    ravager_roar_states: Vec<RavagerRoarParticleState>,
    witch_magic_states: Vec<WitchMagicParticleState>,
    living_entity_poof_states: Vec<LivingEntityPoofParticleState>,
    living_entity_drown_states: Vec<LivingEntityDrownParticleState>,
    living_entity_portal_states: Vec<LivingEntityPortalParticleState>,
    arrow_effect_states: Vec<ArrowEffectParticleState>,
    entity_taming_states: Vec<EntityTamingParticleState>,
    villager_states: Vec<VillagerParticleState>,
    dolphin_happy_states: Vec<DolphinHappyParticleState>,
    fox_eat_states: Vec<FoxEatParticleState>,
    animal_love_states: Vec<AnimalLoveParticleState>,
    allay_duplication_states: Vec<AllayDuplicationParticleState>,
    snowball_hit_states: Vec<SnowballHitParticleState>,
    thrown_egg_hit_states: Vec<ThrownEggHitParticleState>,
    honey_block_states: Vec<HoneyBlockParticleState>,
    batches: Vec<bbb_renderer::ParticleSpawnBatch>,
}

impl ParticleEventSink for RecordingParticleSink {
    fn spawn_level_particles(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
        _biome_sampler: Option<&dyn crate::particle_runtime::ParticleBiomeSampler>,
        _item_runtime: Option<&bbb_item_model::NativeItemRuntime>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.packets.push(packet.clone());
        self.contexts.push(context);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_definition_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_level_event_particles(
        &mut self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        if event.event_type == 3018 {
            advance_cobweb_place_particle_randoms(random);
        } else if matches!(event.event_type, 2002 | 2007) {
            advance_potion_break_level_event_particle_randoms(random);
        } else if event.event_type == 2006 {
            advance_dragon_fireball_explode_level_event_particle_randoms(random);
        } else if event.event_type == 3003 {
            advance_wax_on_level_event_particle_randoms(random);
        } else if event.event_type == 1505 {
            advance_growth_randoms_for_context(event, &context, random);
        } else if event.event_type == 3015 && context.vault_block_entity_at_event_pos {
            bbb_world::advance_vault_activation_particle_randoms_with_connections(
                random,
                context
                    .vault_connection_particles
                    .as_ref()
                    .map(|state| state.targets.len())
                    .unwrap_or(0),
            );
        } else if event.event_type == 3016 {
            bbb_world::advance_vault_deactivation_particle_randoms(random);
        }
        self.level_events.push(*event);
        self.level_event_contexts.push(context);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_firework_empty_explosion_particles(
        &mut self,
        position: [f64; 3],
        camera_position: Option<[f64; 3]>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.firework_empty_explosion_positions.push(position);
        self.firework_empty_explosion_camera_positions
            .push(camera_position);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_firework_explosion_particles(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.firework_explosion_states.push(state.clone());
        self.firework_explosion_camera_positions
            .push(camera_position);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_firework_rocket_trail_particles(
        &mut self,
        state: FireworkRocketTrailParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.firework_rocket_trail_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_ominous_item_spawner_particles(
        &mut self,
        state: OminousItemSpawnerParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.ominous_item_spawner_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_tracking_emitter_particles(
        &mut self,
        state: crate::particle_runtime::TrackingEmitterParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.tracking_emitter_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_take_item_entity_pickup_particles(
        &mut self,
        state: &TakeItemEntityPickupParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.take_item_entity_pickup_states.push(state.clone());
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_ravager_roar_particles(
        &mut self,
        state: RavagerRoarParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.ravager_roar_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_witch_magic_particles(
        &mut self,
        state: WitchMagicParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.witch_magic_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_living_entity_poof_particles(
        &mut self,
        state: LivingEntityPoofParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.living_entity_poof_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_living_entity_drown_particles(
        &mut self,
        state: LivingEntityDrownParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.living_entity_drown_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_living_entity_portal_particles(
        &mut self,
        state: LivingEntityPortalParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.living_entity_portal_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_arrow_effect_particles(
        &mut self,
        state: ArrowEffectParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.arrow_effect_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_entity_taming_particles(
        &mut self,
        state: EntityTamingParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.entity_taming_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_villager_particles(
        &mut self,
        state: VillagerParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.villager_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_dolphin_happy_particles(
        &mut self,
        state: DolphinHappyParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.dolphin_happy_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_fox_eat_particles(
        &mut self,
        state: FoxEatParticleState,
        _item_runtime: Option<&bbb_item_model::NativeItemRuntime>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.fox_eat_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_animal_love_particles(
        &mut self,
        state: AnimalLoveParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.animal_love_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_allay_duplication_particles(
        &mut self,
        state: AllayDuplicationParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.allay_duplication_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_snowball_hit_particles(
        &mut self,
        state: SnowballHitParticleState,
        _item_runtime: Option<&bbb_item_model::NativeItemRuntime>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.snowball_hit_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_thrown_egg_hit_particles(
        &mut self,
        state: ThrownEggHitParticleState,
        _item_runtime: Option<&bbb_item_model::NativeItemRuntime>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.thrown_egg_hit_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_honey_block_particles(
        &mut self,
        state: HoneyBlockParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.honey_block_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }
}

fn test_sound_catalog() -> SoundCatalog {
    let assets_dir = std::env::temp_dir().join("bbb-native-audio-test-assets");
    SoundCatalog::from_json_bytes(
        "minecraft",
        &assets_dir,
        br#"{
            "ambient.cave": {
                "sounds": ["ambient/cave/cave1"]
            },
            "entity.cat.ambient": {
                "sounds": ["mob/cat/meow1"]
            },
            "block.grass.break": {
                "sounds": ["dig/grass1"]
            },
            "item.bone_meal.use": {
                "sounds": ["item/bone_meal/use"]
            },
            "item.totem.use": {
                "sounds": ["item/totem/use"]
            },
            "entity.ravager.attack": {
                "sounds": ["mob/ravager/attack1"]
            },
            "entity.iron_golem.attack": {
                "sounds": ["mob/irongolem/throw"]
            },
            "entity.evoker_fangs.attack": {
                "sounds": ["mob/evocation/fangs"]
            },
            "entity.zombie_villager.cure": {
                "sounds": ["mob/zombie_villager/cure"]
            },
            "entity.armadillo.peek": {
                "sounds": ["mob/armadillo/peek"]
            },
            "entity.armor_stand.hit": {
                "sounds": ["mob/armorstand/hit1"]
            },
            "entity.armor_stand.break": {
                "sounds": ["mob/armorstand/break"]
            },
            "entity.zombie.death": {
                "sounds": ["mob/zombie/death"]
            },
            "entity.zombie_villager.death": {
                "sounds": ["mob/zombie_villager/death"]
            },
            "entity.ravager.death": {
                "sounds": ["mob/ravager/death"]
            },
            "entity.iron_golem.death": {
                "sounds": ["mob/irongolem/death"]
            },
            "entity.villager.death": {
                "sounds": ["mob/villager/death"]
            },
            "entity.witch.death": {
                "sounds": ["mob/witch/death"]
            },
            "entity.skeleton.death": {
                "sounds": ["mob/skeleton/death"]
            },
            "entity.stray.death": {
                "sounds": ["mob/stray/death"]
            },
            "entity.bogged.death": {
                "sounds": ["mob/bogged/death"]
            },
            "entity.item.pickup": {
                "sounds": ["random/pop"]
            },
            "entity.experience_orb.pickup": {
                "sounds": ["random/orb"]
            },
            "entity.arrow.hit_player": {
                "sounds": ["random/bowhit"]
            },
            "entity.puffer_fish.sting": {
                "sounds": ["mob/puffer_fish/sting"]
            },
            "entity.elder_guardian.curse": {
                "sounds": ["mob/elderguardian/curse"]
            },
            "entity.firework_rocket.shoot": {
                "sounds": ["fireworks/launch1"]
            },
            "block.lava.extinguish": {
                "sounds": ["random/fizz"]
            },
            "block.redstone_torch.burnout": {
                "sounds": ["random/fizz"]
            },
            "block.end_portal_frame.fill": {
                "sounds": ["block/end_portal_frame/fill"]
            },
            "entity.splash_potion.break": {
                "sounds": ["random/glass"]
            },
            "entity.dragon_fireball.explode": {
                "sounds": ["mob/enderdragon/fireball"]
            },
            "entity.ghast.warn": {
                "sounds": ["mob/ghast/affectionate_scream"]
            },
            "block.portal.travel": {
                "sounds": ["portal/travel"]
            },
            "entity.wither.spawn": {
                "sounds": ["mob/wither/spawn"]
            },
            "entity.ender_dragon.death": {
                "sounds": ["mob/enderdragon/end"]
            },
            "entity.ender_dragon.growl": {
                "sounds": ["mob/enderdragon/growl"]
            },
            "block.end_gateway.spawn": {
                "sounds": ["block/end_gateway/spawn"]
            },
            "block.trial_spawner.spawn_mob": {
                "sounds": ["block/trial_spawner/spawn_mob"]
            },
            "block.trial_spawner.detect_player": {
                "sounds": ["block/trial_spawner/detect_player"]
            },
            "block.trial_spawner.eject_item": {
                "sounds": ["block/trial_spawner/eject_item"]
            },
            "block.trial_spawner.ominous_activate": {
                "sounds": ["block/trial_spawner/ominous_activate"]
            },
            "block.trial_spawner.spawn_item": {
                "sounds": ["block/trial_spawner/spawn_item"]
            },
            "item.honeycomb.wax_on": {
                "sounds": ["item/honeycomb/wax_on"]
            },
            "item.brush.brushing.sand": {
                "sounds": ["item/brush/brushing/sand"]
            },
            "item.brush.brushing.gravel": {
                "sounds": ["item/brush/brushing/gravel"]
            },
            "block.sculk.charge": {
                "sounds": ["block/sculk/charge"]
            },
            "block.sculk_shrieker.shriek": {
                "sounds": ["block/sculk_shrieker/shriek"]
            },
            "block.cobweb.place": {
                "sounds": ["block/cobweb/place"]
            },
            "block.vault.activate": {
                "sounds": ["block/vault/activate"]
            },
            "block.vault.deactivate": {
                "sounds": ["block/vault/deactivate"]
            },
            "block.end_portal.spawn": {
                "sounds": ["portal/endportal"]
            },
            "music_disc.cat": {
                "sounds": ["records/cat"]
            },
            "music_disc.tears": {
                "sounds": ["records/tears"]
            }
        }"#,
    )
    .unwrap()
}

fn protocol_add_entity(id: i32) -> AddEntity {
    protocol_add_entity_with_type(id, 7)
}

fn protocol_update_mob_effect(
    entity_id: i32,
    effect_id: i32,
) -> bbb_protocol::packets::UpdateMobEffect {
    bbb_protocol::packets::UpdateMobEffect {
        entity_id,
        effect_id,
        amplifier: 2,
        duration_ticks: 400,
        flags: bbb_protocol::packets::MobEffectFlags {
            raw: 0b1011,
            ambient: true,
            visible: true,
            show_icon: false,
            blend: true,
        },
    }
}

fn level_event_at(event_type: i32, x: i32, y: i32, z: i32) -> LevelEvent {
    level_event_at_with_data(event_type, x, y, z, 0)
}

fn level_event_at_with_data(event_type: i32, x: i32, y: i32, z: i32, data: i32) -> LevelEvent {
    LevelEvent {
        event_type,
        pos: block_pos(x, y, z),
        data,
        global: false,
    }
}

fn set_test_block(world: &mut WorldStore, pos: ProtocolBlockPos, block_state_id: i32) {
    assert!(world.apply_block_update(BlockUpdate {
        pos,
        block_state_id,
    }));
}

fn block_pos(x: i32, y: i32, z: i32) -> ProtocolBlockPos {
    ProtocolBlockPos { x, y, z }
}

fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    let properties = BTreeMap::from(props.map(|(key, value)| (key.to_string(), value.to_string())));
    bbb_world::RegistrySet::vanilla_26_1()
        .block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("missing vanilla block state {name} {properties:?}"))
}

fn synthetic_native_level_chunk_packet() -> LevelChunkWithLight {
    let mut sections = Encoder::new();
    sections.write_i16(0);
    sections.write_i16(0);
    sections.write_u8(0);
    sections.write_var_i32(0);
    sections.write_u8(0);
    sections.write_var_i32(0);

    LevelChunkWithLight {
        x: 1,
        z: -2,
        chunk_data: LevelChunkData {
            heightmaps: vec![ChunkHeightmapData {
                kind_id: 1,
                data: vec![42],
            }],
            section_data: sections.into_inner(),
            block_entities: vec![LevelChunkBlockEntity {
                packed_xz: 0,
                y: -64,
                block_entity_type_id: 7,
                raw_nbt: vec![0],
            }],
        },
        light_data: empty_light_update_data(),
    }
}

fn empty_light_update_data() -> LightUpdateData {
    LightUpdateData {
        sky_y_mask: Vec::new(),
        block_y_mask: Vec::new(),
        empty_sky_y_mask: Vec::new(),
        empty_block_y_mask: Vec::new(),
        sky_updates: Vec::new(),
        block_updates: Vec::new(),
    }
}

fn single_biome_payload(biome_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_u8(0);
    payload.write_var_i32(biome_id);
    payload.into_inner()
}

fn sign_text_nbt(front: [&str; 4], back: [&str; 4]) -> Vec<u8> {
    let mut payload = vec![10];
    write_sign_text_side(&mut payload, "front_text", front);
    write_sign_text_side(&mut payload, "back_text", back);
    payload.push(0);
    payload
}

fn vault_shared_data_nbt(players: &[Uuid], connected_particles_range: Option<f64>) -> Vec<u8> {
    let mut payload = vec![10, 10];
    write_nbt_string(&mut payload, "shared_data");
    if !players.is_empty() {
        payload.push(9);
        write_nbt_string(&mut payload, "connected_players");
        payload.push(11);
        payload.extend_from_slice(&(players.len() as i32).to_be_bytes());
        for player in players {
            write_nbt_uuid_int_array(&mut payload, *player);
        }
    }
    if let Some(range) = connected_particles_range {
        payload.push(6);
        write_nbt_string(&mut payload, "connected_particles_range");
        payload.extend_from_slice(&range.to_be_bytes());
    }
    payload.push(0);
    payload.push(0);
    payload
}

fn write_sign_text_side(out: &mut Vec<u8>, name: &str, lines: [&str; 4]) {
    out.push(10);
    write_nbt_string(out, name);
    out.push(9);
    write_nbt_string(out, "messages");
    out.push(8);
    out.extend_from_slice(&4i32.to_be_bytes());
    for line in lines {
        write_nbt_string(out, line);
    }
    out.push(0);
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn write_nbt_uuid_int_array(out: &mut Vec<u8>, uuid: Uuid) {
    let value = uuid.as_u128();
    let ints = [
        (value >> 96) as u32,
        (value >> 64) as u32,
        (value >> 32) as u32,
        value as u32,
    ];
    out.extend_from_slice(&4i32.to_be_bytes());
    for value in ints {
        out.extend_from_slice(&(value as i32).to_be_bytes());
    }
}

fn vault_block_state_id(facing: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("ominous".to_string(), "false".to_string()),
        ("vault_state".to_string(), "active".to_string()),
    ]);
    bbb_world::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:vault", &properties)
        .unwrap()
        .id
}

fn vault_test_player(id: i32, uuid: Uuid, position: ProtocolVec3d) -> AddEntity {
    AddEntity {
        id,
        uuid,
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position,
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn assert_close64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
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
