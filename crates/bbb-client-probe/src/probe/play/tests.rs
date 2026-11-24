use super::*;

use bbb_client_net::RawConnection;

use bbb_client_world::{
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type, BlockPos, ChunkPos,
    LocalPlayerPoseState, RegistrySet,
};

use bbb_protocol::entity_types::*;

use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;

use bbb_protocol::menu_types::VANILLA_MENU_TYPE_MERCHANT_ID;

use bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_MINING_FATIGUE_ID;

use bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID;

use bbb_protocol::packets::{
    AddEntity, AdvancementCriterionProgressSummary, AdvancementProgressSummary, AdvancementSummary,
    AttributeModifier, AttributeSnapshot, AwardStats, BlockChangedAck, BlockEntityData, BlockEvent,
    BlockPos as ProtocolBlockPos, BlockUpdate, BossBarColor, BossBarOverlay, BossEvent,
    BossEventFlags, BossEventOperation, ChangeDifficulty, ChatFormatting, ChatTypeBound,
    ChatTypeHolder, ChunkBiomeData, ChunkHeightmapData, ChunkPos as ProtocolChunkPos, ChunksBiomes,
    ClearTitles, ClockUpdate, CommandArgumentParser, CommandNode, CommandNodeType,
    CommandSuggestion, CommandSuggestions, Commands, CommonPlayerSpawnInfo, ContainerClose,
    ContainerSetContent, ContainerSetData, ContainerSetSlot, CookieRequest, Cooldown,
    CustomChatCompletions, CustomChatCompletionsAction, CustomPayload, CustomPayloadBody,
    CustomReportDetails, DamageEvent, DebugBlockValue, DebugChunkValue, DebugEntityValue,
    DebugEvent, DebugSample, DeleteChat, DialogHolder, Difficulty, Disconnect, DisguisedChat,
    EntityAnchor, EntityAnimation, EntityDataValue, EntityDataValueKind, EntityEvent, EntityMove,
    EntityPositionSync, EquipmentSlot, EquipmentSlotUpdate, Explosion, FilterMask, FilterMaskKind,
    ForgetLevelChunk, GameEvent, GameProfile, GameProfileProperty, GameRuleValue, GameRuleValues,
    GameTestHighlightPos, GameType, HurtAnimation, IngredientSummary, InitializeBorder,
    InteractionHand, ItemCostSummary, ItemStackSummary, LevelChunkBlockEntity, LevelChunkData,
    LevelChunkWithLight, LevelEvent, LevelParticles, LightUpdate, LightUpdateData, MapColorPatch,
    MapDecoration, MapItemData, MerchantOffer, MerchantOffers, MessageSignature, MinecartStep,
    MobEffectFlags, MountScreenOpen, MoveMinecartAlongTrack, MoveVehicle, ObjectiveRenderType,
    OpenBook, OpenScreen, OpenSignEditor, PackedMessageSignature, ParticlePayload,
    PlaceGhostRecipe, PlayLogin, PlayTime, PlayerAbilities, PlayerChat, PlayerCombatEnd,
    PlayerCombatKill, PlayerExperience, PlayerHealth, PlayerInfoAction, PlayerInfoChatSession,
    PlayerInfoEntry, PlayerInfoRemove, PlayerInfoUpdate, PlayerLookAt, PlayerPositionUpdate,
    PlayerRotationUpdate, PlayerTeamMethod, PlayerTeamParameters, PongResponse, ProjectilePower,
    RecipeBookAdd, RecipeBookAddEntry, RecipeBookRemove, RecipeBookSettings,
    RecipeBookTypeSettings, RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipeDisplayType, RecipePropertySetSummary, RegistryTags, RemoteDebugSampleType,
    RemoveEntities, RemoveMobEffect, ResetScore, ResourcePackPop, ResourcePackPush,
    ResourcePackResponseAction, Respawn, RotateHead, ScoreboardDisplaySlot, SectionBlocksUpdate,
    SelectAdvancementsTab, ServerData, ServerLinkEntry, ServerLinkKnownType, ServerLinkType,
    ServerLinks, SetActionBarText, SetBorderCenter, SetBorderLerpSize, SetBorderSize,
    SetBorderWarningDelay, SetBorderWarningDistance, SetCamera, SetChunkCacheCenter,
    SetChunkCacheRadius, SetCursorItem, SetDefaultSpawnPosition, SetDisplayObjective,
    SetEntityData, SetEntityLink, SetEntityMotion, SetEquipment, SetHeldSlot, SetObjective,
    SetObjectiveMethod, SetObjectiveParameters, SetPassengers, SetPlayerInventory, SetPlayerTeam,
    SetScore, SetSimulationDistance, SetSubtitleText, SetTitleText, SetTitlesAnimation, ShowDialog,
    SignedMessageBody, SlotDisplaySummary, SoundEntityEvent, SoundEvent, SoundEventHolder,
    SoundSource, StatUpdate, StonecutterSelectableRecipeSummary, StopSound, StoreCookie,
    SystemChat, TabList, TagNetworkPayload, TagQuery, TakeItemEntity, TeamCollisionRule,
    TeamVisibility, TeleportEntity, TestInstanceBlockStatus, TickingState, TickingStep,
    TrackedWaypoint, TrackedWaypointPacket, Transfer, UpdateAdvancements, UpdateAttributes,
    UpdateMobEffect, UpdateRecipes, UpdateTags, Vec3d as ProtocolVec3d, Vec3i as ProtocolVec3i,
    WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation, WaypointVec3i,
    WrittenBookContentSummary,
};

use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_HEART_ID;

use bbb_protocol::{
    codec::{Decoder, Encoder},
    ids,
};

use std::{collections::BTreeMap, time::Duration};

use tokio::net::TcpListener;

use tokio::time::timeout;

use uuid::Uuid;

fn probe_recipe_category_id(id: i32) -> i32 {
    id + 10
}

fn probe_recipe_entry(id: i32, notification: bool, highlight: bool) -> RecipeBookAddEntry {
    RecipeBookAddEntry {
        contents: RecipeDisplayEntry {
            id: RecipeDisplayId { index: id },
            display: RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: vec![3, id as u8],
                crafting: None,
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
            group: None,
            category_id: probe_recipe_category_id(id),
            crafting_requirements: Some(vec![IngredientSummary {
                tag: None,
                item_ids: vec![40 + id],
            }]),
        },
        flags: (u8::from(notification)) | (u8::from(highlight) << 1),
        notification,
        highlight,
    }
}

async fn raw_connection_pair() -> (RawConnection, RawConnection) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = tokio::spawn(async move {
        RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap()
    });
    let (server_stream, _) = listener.accept().await.unwrap();
    let client = client.await.unwrap();
    let server = RawConnection::from_stream(server_stream, None);
    (client, server)
}

const LIGHT_ARRAY_BYTES: usize = 2048;

fn synthetic_probe_level_chunk_packet() -> LevelChunkWithLight {
    let mut sections = Encoder::new();
    for _ in 0..bbb_client_world::WorldDimension::default().section_count() {
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
        light_data: LightUpdateData {
            sky_y_mask: Vec::new(),
            block_y_mask: Vec::new(),
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: Vec::new(),
            block_updates: Vec::new(),
        },
    }
}

fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    let properties = BTreeMap::from(props.map(|(key, value)| (key.to_string(), value.to_string())));
    RegistrySet::vanilla_26_1()
        .block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("missing vanilla block state {name} {properties:?}"))
}

fn single_biome_payload(biome_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_u8(0);
    payload.write_var_i32(biome_id);
    payload
        .into_inner()
        .repeat(bbb_client_world::WorldDimension::default().section_count())
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

fn set_light_nibble(layer: &mut [u8], nibble_index: usize, value: u8) {
    let byte = layer.get_mut(nibble_index / 2).unwrap();
    let shift = (nibble_index % 2) * 4;
    *byte = (*byte & !(0x0f << shift)) | ((value & 0x0f) << shift);
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
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

fn entity_air_supply_data(air_supply: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    EntityDataValue {
        data_id: accessors.air_supply_data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(air_supply),
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

fn item_stack_entity_data(item: ItemStackSummary) -> EntityDataValue {
    let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("generated ItemEntity item accessor must resolve");
    EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        value: EntityDataValueKind::ItemStack(item),
    }
}

fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

fn merchant_offers(container_id: i32, offer_count: usize) -> MerchantOffers {
    MerchantOffers {
        container_id,
        offers: (0..offer_count)
            .map(|index| MerchantOffer {
                buy_a: item_cost(42 + index as i32, 3),
                sell: item_stack(99 + index as i32, 1),
                buy_b: None,
                is_out_of_stock: false,
                uses: 1,
                max_uses: 12,
                xp: 8,
                special_price_diff: -2,
                price_multiplier: 0.05,
                demand: 6,
            })
            .collect(),
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
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

fn command_tree_packet(literal: &str) -> Commands {
    Commands {
        root_index: 0,
        nodes: vec![
            CommandNode {
                node_type: CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Literal,
                flags: 1,
                children: vec![2],
                redirect: None,
                name: Some(literal.to_string()),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Argument,
                flags: 54,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                    name: "minecraft:message".into(),
                    properties: Vec::new(),
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: true,
            },
        ],
    }
}

fn protocol_player_chat_with_signature(
    global_index: i32,
    signature: MessageSignature,
) -> PlayerChat {
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
        chat_type: ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: "Alice".into(),
            name_styled: Vec::new(),
            name_component: None,
            target_name: None,
            target_name_styled: None,
            target_name_component: None,
        },
    }
}

fn protocol_add_entity(id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
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

fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
    AddEntity {
        entity_type_id,
        ..protocol_add_entity(id)
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn minecart_step(
    position: [f64; 3],
    movement: [f64; 3],
    y_rot: f32,
    x_rot: f32,
    weight: f32,
) -> MinecartStep {
    MinecartStep {
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        movement: ProtocolVec3d {
            x: movement[0],
            y: movement[1],
            z: movement[2],
        },
        y_rot,
        x_rot,
        weight,
    }
}

mod batch_01;
mod batch_02;
