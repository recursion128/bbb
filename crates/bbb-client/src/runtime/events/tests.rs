use super::*;
use crate::audio_runtime::resolver::{AudioCommandResolver, AudioResolveError};
use crate::control::{AudioCounters, NetCounters};
use crate::particle_runtime::ParticleTarget;
use crate::runtime::{clear_color_for_day_time, clear_color_for_world};
use bbb_client_assets::{JukeboxSongRegistry, SoundCatalog, SoundEventRegistry};
use bbb_client_audio::{AudioCategory, AudioCommand};
use bbb_client_net::{NetCommand, NetEvent};
use bbb_client_world::{
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type, BlockPos, ChunkPos,
    LocalPlayerPoseState, RegistryPacketEntry, WorldDimension, WorldStore,
};
use bbb_protocol::codec::Encoder;
use bbb_protocol::entity_types::*;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
use bbb_protocol::packets::PlayClientbound;
use bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID;
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
use bbb_protocol::particle_types::*;
use bbb_protocol::recipe_wire_types::VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID;
use std::collections::BTreeMap;
use tokio::sync::mpsc;
use uuid::Uuid;

mod audio_and_death_sounds;
mod client_tick_effects;
mod entity_and_inventory_counters;
mod entity_particle_events;
mod level_event_block_and_potion_sounds;
mod level_event_particle_batches;
mod session_hud_and_vehicle_events;
mod session_protocol_events;
mod trial_spawner_and_sculk_events;
mod world_state_lifecycle;

pub(crate) use bbb_client_render::scene::synchronized_registry_test_fixture::minimal_non_empty_synchronized_registry_data;

#[test]
fn minimal_synchronized_fixture_backs_every_exact_non_empty_registry() {
    for data in minimal_non_empty_synchronized_registry_data() {
        assert_eq!(data.entries.len(), 1, "{}", data.registry);
        assert!(data.entries[0].raw_data.is_some(), "{}", data.registry);
    }
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn record_test_player_info(world: &mut WorldStore, profile_id: Uuid) {
    assert_eq!(
        world.apply_player_info_update(bbb_protocol::packets::PlayerInfoUpdate {
            actions: vec![bbb_protocol::packets::PlayerInfoAction::AddPlayer],
            entries: vec![bbb_protocol::packets::PlayerInfoEntry {
                profile_id,
                profile: Some(bbb_protocol::packets::GameProfile {
                    uuid: profile_id,
                    name: "runtime-test-player".to_string(),
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

fn firework_item_data_id() -> u8 {
    let accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    )
    .expect("generated FireworkRocket metadata accessors must resolve");
    assert_eq!(
        accessors.item_default_resource_id,
        "minecraft:firework_rocket"
    );
    accessors.item_data_id
}

fn item_entity_stack_data_id() -> u8 {
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(VANILLA_ENTITY_TYPE_ITEM_ID)
        .expect("generated ItemEntity item accessor must resolve")
}

fn entity_invisible_data() -> EntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert_eq!(accessors.shared_flags_default, 0);
    EntityDataValue {
        data_id: accessors.shared_flags_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(accessors.invisible_mask),
    }
}

fn entity_shift_key_down_data() -> EntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert_eq!(accessors.shared_flags_default, 0);
    EntityDataValue {
        data_id: accessors.shared_flags_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(accessors.shift_key_down_mask),
    }
}

fn entity_silent_data() -> EntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert!(!accessors.silent_default);
    EntityDataValue {
        data_id: accessors.silent_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(true),
    }
}

fn firework_item_entity_data_with_explosions(
    entity_id: i32,
    explosions: Vec<FireworkExplosionSummary>,
) -> SetEntityData {
    let mut stack = item_stack(901, 1);
    stack.component_patch = DataComponentPatchSummary {
        fireworks_flight_duration: Some(1),
        fireworks_explosions: explosions,
        ..DataComponentPatchSummary::default()
    }
    .into();
    SetEntityData {
        id: entity_id,
        values: vec![EntityDataValue {
            data_id: firework_item_data_id(),
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack),
        }],
    }
}

fn written_book_stack(pages: Vec<&str>) -> ItemStackSummary {
    let mut stack = item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".into(),
        title_filter: None,
        author: "Alex".into(),
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
                stonecutter: None,
                smithing: None,
            },
            group: None,
            category_id: VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
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
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
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
        name_styled: Vec::new(),
        name_component: None,
        target_name: None,
        target_name_styled: None,
        target_name_component: None,
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
        unsigned_content_styled: None,
        unsigned_content_component: None,
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

    fn play_local_sound(&mut self, state: &bbb_client_world::LocalSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_client_audio::AudioVolumeSettings::default(),
            );
            resolver.play_local_sound(state)
        };
        self.record(command);
    }

    fn play_positioned_sound(&mut self, state: &bbb_client_world::SoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_client_audio::AudioVolumeSettings::default(),
            );
            resolver.play_positioned_sound(state)
        };
        self.record(command);
    }

    fn play_entity_sound(
        &mut self,
        state: &bbb_client_world::SoundEntityEventState,
        position: Option<[f64; 3]>,
    ) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_client_audio::AudioVolumeSettings::default(),
            );
            resolver.play_entity_sound_at(state, position)
        };
        self.record(command);
    }

    fn play_jukebox_song(&mut self, state: &bbb_client_world::JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_client_audio::AudioVolumeSettings::default(),
            );
            resolver.play_jukebox_song(state)
        };
        self.record(command);
    }

    fn stop_jukebox_song(&mut self, state: &bbb_client_world::JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_client_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_jukebox_song(state)
        };
        self.commands.push(command);
    }

    fn stop_sound(&mut self, state: &bbb_client_world::StopSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_client_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_sound(state)
        };
        self.commands.push(command);
    }

    fn set_music_gain(&mut self, gain: f32) {
        self.commands.push(AudioCommand::SetMusicGain(
            bbb_client_audio::SetMusicGainCommand { gain },
        ));
    }

    fn sync_ambient_loops(&mut self, loops: &[bbb_client_world::AmbientLoopSoundState]) {
        let resolver = AudioCommandResolver::with_jukebox_registry(
            &self.catalog,
            &self.registry,
            &self.jukebox_registry,
            bbb_client_audio::AudioVolumeSettings::default(),
        );
        let mut resolved = Vec::new();
        for state in loops {
            match resolver.ambient_loop_sound(state) {
                Ok(command) => resolved.push(command),
                Err(err) => self.errors.push(err.to_string()),
            }
        }
        self.commands.push(AudioCommand::SyncAmbientLoops(
            bbb_client_audio::SyncAmbientLoopsCommand { loops: resolved },
        ));
    }

    fn tick_entity_sound_positions(
        &mut self,
        listener: Option<bbb_client_audio::AudioListenerState>,
        entity_position: &dyn Fn(i32) -> Option<[f64; 3]>,
        tickable_sounds: &[bbb_client_world::TickableEntitySoundState],
    ) {
        let entity_ids: std::collections::BTreeSet<_> = self
            .commands
            .iter()
            .filter_map(|command| match command {
                AudioCommand::PlayEntitySound(command) => Some(command.entity_id),
                _ => None,
            })
            .chain(tickable_sounds.iter().map(|state| state.event.entity_id))
            .collect();
        let mut command = bbb_client_audio::TickEntitySoundPositionsCommand {
            listener,
            entities: entity_ids
                .into_iter()
                .filter_map(|entity_id| {
                    entity_position(entity_id).map(|position| {
                        bbb_client_audio::EntitySoundPosition {
                            entity_id,
                            position,
                        }
                    })
                })
                .collect(),
            tickable_sounds: Vec::new(),
        };
        let resolver = AudioCommandResolver::with_jukebox_registry(
            &self.catalog,
            &self.registry,
            &self.jukebox_registry,
            bbb_client_audio::AudioVolumeSettings::default(),
        );
        command.tickable_sounds.extend(
            tickable_sounds
                .iter()
                .map(|state| resolver.tickable_entity_sound_update(state).unwrap()),
        );
        self.commands
            .push(AudioCommand::TickEntitySoundPositions(command));
    }
}

/// A particle resolver with every type defined, plus the engine it fills.
struct TestParticles {
    runtime: crate::particle_runtime::NativeParticleRuntime,
    engine: bbb_client_world::particles::ParticleEngine,
}

impl TestParticles {
    fn new() -> Self {
        Self {
            runtime: crate::particle_runtime::NativeParticleRuntime::for_tests(),
            engine: bbb_client_world::particles::ParticleEngine::default(),
        }
    }

    fn target(&mut self) -> Option<ParticleTarget<'_>> {
        Some(ParticleTarget {
            runtime: &mut self.runtime,
            engine: &mut self.engine,
        })
    }

    /// Kinds of the particles alive after one engine tick.
    fn live_kinds(&mut self, world: &WorldStore) -> Vec<bbb_client_world::particles::ParticleKind> {
        self.engine.advance(1, world, Default::default());
        self.engine
            .instances()
            .map(|instance| instance.kind)
            .collect()
    }
}

fn test_sound_catalog() -> SoundCatalog {
    let assets_dir = std::env::temp_dir().join("bbb-client-audio-test-assets");
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
            "entity.cow.hurt": {
                "sounds": ["mob/cow/hurt1"]
            },
            "entity.zombie.hurt": {
                "sounds": ["mob/zombie/hurt1"]
            },
            "block.grass.break": {
                "sounds": ["dig/grass1"]
            },
            "block.wood.break": {
                "sounds": ["dig/wood1"]
            },
            "block.note_block.harp": {
                "sounds": ["note/harp"]
            },
            "block.piston.extend": {
                "sounds": ["block/piston/extend"]
            },
            "item.bone_meal.use": {
                "sounds": ["item/bone_meal/use"]
            },
            "item.totem.use": {
                "sounds": ["item/totem/use"]
            },
            "entity.guardian.attack": {
                "sounds": ["mob/guardian/attack_loop"]
            },
            "entity.sniffer.digging": {
                "sounds": ["mob/sniffer/digging"]
            },
            "entity.minecart.riding": {
                "sounds": ["minecart/base"]
            },
            "entity.bee.loop": {
                "sounds": ["mob/bee/loop"]
            },
            "entity.bee.loop_aggressive": {
                "sounds": ["mob/bee/loop_aggressive"]
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
            "entity.slime.squish": {
                "sounds": ["mob/slime/big1"]
            },
            "entity.slime.squish_small": {
                "sounds": ["mob/slime/small1"]
            },
            "entity.warden.heartbeat": {
                "sounds": ["mob/warden/heartbeat"]
            },
            "entity.ender_dragon.flap": {
                "sounds": ["mob/enderdragon/wings"]
            },
            "entity.ender_dragon.growl": {
                "sounds": ["mob/enderdragon/growl"]
            },
            "entity.lightning_bolt.thunder": {
                "sounds": ["ambient/weather/thunder1"]
            },
            "entity.lightning_bolt.impact": {
                "sounds": ["ambient/weather/thunder2"]
            },
            "entity.magma_cube.squish": {
                "sounds": ["mob/slime/big2"]
            },
            "entity.magma_cube.squish_small": {
                "sounds": ["mob/slime/small2"]
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
            "entity.player.hurt_freeze": {
                "sounds": ["damage/freeze"]
            },
            "entity.player.death": {
                "sounds": ["damage/hit1"]
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
            "block.trial_spawner.about_to_spawn_item": {
                "sounds": ["block/trial_spawner/about_to_spawn_item"]
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
            },
            "custom.song": {
                "sounds": ["records/custom"]
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
    bbb_client_world::RegistrySet::vanilla_26_1()
        .block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("missing vanilla block state {name} {properties:?}"))
}

fn synthetic_native_level_chunk_packet() -> LevelChunkWithLight {
    synthetic_native_level_chunk_packet_with_sections(WorldDimension::default().section_count())
}

fn synthetic_native_level_chunk_packet_with_sections(section_count: usize) -> LevelChunkWithLight {
    let mut sections = Encoder::new();
    for _ in 0..section_count {
        sections.write_i16(0);
        sections.write_i16(0);
        sections.write_u8(0);
        sections.write_var_i32(0);
        sections.write_u8(0);
        sections.write_var_i32(0);
    }

    LevelChunkWithLight {
        x: 1,
        z: -2,
        chunk_data: LevelChunkData {
            heightmaps: vec![ChunkHeightmapData {
                kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                data: vec![42],
            }],
            section_data: sections.into_inner(),
            block_entities: vec![LevelChunkBlockEntity {
                packed_xz: 0,
                y: -64,
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
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
    payload
        .into_inner()
        .repeat(WorldDimension::default().section_count())
}

fn sign_text_nbt(front: [&str; 4], back: [&str; 4]) -> Vec<u8> {
    let mut payload = vec![10];
    write_sign_text_side(&mut payload, "front_text", front);
    write_sign_text_side(&mut payload, "back_text", back);
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

fn assert_close(actual: f32, expected: f32) {
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
