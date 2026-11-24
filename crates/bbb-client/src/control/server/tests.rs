use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
    sync::{Arc, Mutex, MutexGuard},
};

use super::params::{
    EDIT_BOOK_MAX_PAGES, EDIT_BOOK_MAX_PAGE_CHARS, EDIT_BOOK_MAX_TITLE_CHARS,
    RENAME_ITEM_MAX_NAME_CHARS,
};

use super::*;

use crate::control::types::{AudioCounters, NetCounters, RecipeBookTypeControl};

use crate::control::{
    ControlRequestSink, ControlSnapshot, ControlSubmitError, ControlSubmitReceipt,
    ControlWorldSnapshot, ControlWriteRequest, SharedControlRequests,
};

use bbb_client_world::{
    BlockEntityRecord, ChunkColumn, ChunkSection, ChunkState, HeightmapData, LightData,
    LocalPlayerPoseState, PaletteDomain, PaletteKind, PalettedContainerData,
    ResourcePackFailureStage, ResourcePackRuntimeFailure, ResourcePackRuntimePhase,
    ResourcePackRuntimeState, WorldDimension, WorldStore,
};

use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;

use bbb_protocol::menu_types::{VANILLA_MENU_TYPE_GENERIC_9X3_ID, VANILLA_MENU_TYPE_MERCHANT_ID};

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AdvancementSummary, AwardStats, BlockChangedAck,
    BlockDestruction, BlockEvent, BlockPos as ProtocolBlockPos, ChatFormatting, ChatTypeBound,
    ChatTypeHolder, CommandArgumentParser, CommandNode, CommandNodeType, Commands,
    CommonPlayerSpawnInfo as ProtocolSpawnInfo, ContainerClose, ContainerSetContent,
    ContainerSetData, ContainerSetSlot, Cooldown, CustomChatCompletions,
    CustomChatCompletionsAction, CustomPayload, CustomPayloadBody, CustomReportDetails,
    DamageEvent as ProtocolDamageEvent, DebugBlockValue, DialogHolder,
    DisguisedChat as ProtocolDisguisedChat, EntityEvent as ProtocolEntityEvent,
    EntityPositionSync as ProtocolEntityPositionSync, Explosion as ProtocolExplosion, GameEvent,
    GameProfile, GameProfileProperty, GameRuleValue, GameRuleValues, GameType,
    HurtAnimation as ProtocolHurtAnimation, IngredientSummary, InitializeBorder, InteractionHand,
    ItemCostSummary, ItemStackSummary, KnownPack, LevelEvent,
    LevelParticles as ProtocolLevelParticles, MapColorPatch, MapDecoration, MapItemData,
    MerchantOffer, MerchantOffers, MobEffectFlags, MountScreenOpen, ObjectiveRenderType, OpenBook,
    OpenScreen, OpenSignEditor, ParticlePayload, PlaceGhostRecipe, PlayLogin as ProtocolPlayLogin,
    PlayTime, PlayerAbilities, PlayerCombatKill, PlayerExperience, PlayerHealth, PlayerInfoAction,
    PlayerInfoEntry, PlayerInfoUpdate, PlayerTeamMethod, PlayerTeamParameters, PongResponse,
    ProjectilePower, RecipeBookAdd, RecipeBookAddEntry, RecipeBookRemove, RecipeBookSettings,
    RecipeBookTypeSettings, RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipeDisplayType, RecipePropertySetSummary, RegistryData, RegistryDataEntry, RegistryTags,
    ScoreboardDisplaySlot, SelectAdvancementsTab, ServerLinkEntry, ServerLinkKnownType,
    ServerLinkType, ServerLinks, SetActionBarText, SetBorderCenter, SetBorderLerpSize,
    SetBorderWarningDelay, SetBorderWarningDistance, SetChunkCacheCenter, SetChunkCacheRadius,
    SetCursorItem, SetDefaultSpawnPosition, SetDisplayObjective,
    SetEntityMotion as ProtocolSetEntityMotion, SetObjective, SetObjectiveMethod,
    SetObjectiveParameters, SetPlayerInventory, SetPlayerTeam, SetScore, SetSimulationDistance,
    SetSubtitleText, SetTitleText, SetTitlesAnimation, ShowDialog, SlotDisplaySummary, SoundEvent,
    SoundEventHolder, SoundSource, StatUpdate, StonecutterSelectableRecipeSummary, StopSound,
    SystemChat, TagNetworkPayload, TagQuery, TeamCollisionRule, TeamVisibility, TickingState,
    TickingStep, TrackedWaypoint, TrackedWaypointPacket, Transfer, UpdateAdvancements,
    UpdateEnabledFeatures, UpdateMobEffect, UpdateRecipes, UpdateTags, Vec3d as ProtocolVec3d,
    WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation, WaypointVec3i,
    WeightedExplosionParticle, VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
};

use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_CLOUD_ID, VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
    VANILLA_PARTICLE_TYPE_HEART_ID,
};

use bbb_protocol::recipe_wire_types::{
    VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
};

use bbb_protocol::stat_types::{VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_CUSTOM_ID};

use serde_json::json;

use uuid::Uuid;

#[derive(Debug, Default)]
struct RecordedControlRequests {
    shutdown_requested: bool,
    screenshot_request: Option<String>,
    wait_ticks_requests: Vec<u32>,
    video_requests: Vec<crate::control::RecordingControlRequest>,
    code_of_conduct_requests: Vec<CodeOfConductControlRequest>,
    net_requests: Vec<NetControlRequest>,
    input_requests: Vec<InputControlRequest>,
}

#[derive(Debug, Default)]
struct RecordingControlRequestSink {
    requests: Mutex<RecordedControlRequests>,
}

impl RecordingControlRequestSink {
    fn lock(&self) -> std::sync::LockResult<MutexGuard<'_, RecordedControlRequests>> {
        self.requests.lock()
    }
}

impl ControlRequestSink for RecordingControlRequestSink {
    fn submit(
        &self,
        request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pending = match request {
            ControlWriteRequest::Shutdown => {
                requests.shutdown_requested = true;
                1
            }
            ControlWriteRequest::Screenshot { path } => {
                requests.screenshot_request = Some(path);
                1
            }
            ControlWriteRequest::LockstepWake => 0,
            ControlWriteRequest::WaitTicks { ticks } => {
                requests.wait_ticks_requests.push(ticks);
                requests.wait_ticks_requests.len()
            }
            ControlWriteRequest::Recording(request) => {
                requests.video_requests.push(request);
                requests.video_requests.len()
            }
            ControlWriteRequest::CodeOfConduct(request) => {
                requests.code_of_conduct_requests.push(request);
                requests.code_of_conduct_requests.len()
            }
            ControlWriteRequest::Net(request) => {
                requests.net_requests.push(request);
                requests.net_requests.len()
            }
            ControlWriteRequest::Input(request) => {
                requests.input_requests.push(request);
                requests.input_requests.len()
            }
        };
        Ok(ControlSubmitReceipt { pending })
    }
}

#[derive(Debug)]
struct ClosedControlRequestSink;

impl ControlRequestSink for ClosedControlRequestSink {
    fn submit(
        &self,
        _request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        Err(ControlSubmitError::Closed)
    }
}

struct TestControlState {
    control: ControlState,
    requests: Arc<RecordingControlRequestSink>,
}

impl Deref for TestControlState {
    type Target = ControlState;

    fn deref(&self) -> &Self::Target {
        &self.control
    }
}

fn test_control_state() -> TestControlState {
    let requests = Arc::new(RecordingControlRequestSink::default());
    let control = ControlState::new("test", SharedControlRequests::new(requests.clone()));
    TestControlState { control, requests }
}

fn publish_test_world(state: &ControlState, mut world: WorldStore) {
    let mut input = crate::input::ClientInputState::default();
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let current = state.snapshot.load();
    state.snapshot.store(Arc::new(ControlSnapshot {
        generation: current.generation.saturating_add(1),
        world_view: ControlWorldSnapshot {
            client_ui: input.dialog_control_projection(
                bbb_protocol::to_display_json_value(world.client_ui()).unwrap(),
            ),
            creative_inventory: None,
            world: Arc::new(world),
        },
        ..(**current).clone()
    }));
}

fn single_section_chunk() -> ChunkColumn {
    ChunkColumn {
        pos: ChunkPos { x: 1, z: -2 },
        state: ChunkState::Decoded,
        heightmaps: vec![HeightmapData {
            kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            data: vec![0],
        }],
        sections: vec![Arc::new(ChunkSection {
            non_empty_block_count: 1,
            fluid_count: 0,
            block_states: single_value_container(PaletteDomain::BlockStates, 42, 4096),
            biomes: single_value_container(PaletteDomain::Biomes, 3, 64),
        })],
        block_entities: Arc::new(Vec::<BlockEntityRecord>::new()),
        light: LightData::default(),
    }
}

fn single_value_container(
    domain: PaletteDomain,
    global_id: i32,
    entry_count: usize,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
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
        flags: u8::from(notification) | (u8::from(highlight) << 1),
        notification,
        highlight,
    }
}

fn command_tree(literal: &str) -> Commands {
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
                flags: 2,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    name: "brigadier:string".into(),
                    properties: vec![1],
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: true,
            },
        ],
    }
}

fn protocol_add_entity(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: uuid::Uuid::from_u128(id as u128 + 1),
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

mod batch_01;
mod batch_02;

#[test]
fn sim_step_params_require_bounded_positive_integers() {
    assert_eq!(parse_sim_step_params(&json!({})), Ok((1, 50_000_000)));
    assert_eq!(
        parse_sim_step_params(&json!({ "ticks": 4, "tick_nanos": 25_000_000 })),
        Ok((4, 25_000_000))
    );
    for invalid in [
        json!({ "ticks": 0 }),
        json!({ "ticks": -3 }),
        json!({ "ticks": 1.5 }),
        json!({ "ticks": MAX_STEP_TICKS + 1 }),
        json!({ "tick_nanos": 0 }),
        json!({ "tick_nanos": -1 }),
        json!({ "tick_nanos": 50.5 }),
        json!({ "tick_nanos": MAX_STEP_TICK_NANOS + 1 }),
    ] {
        assert!(parse_sim_step_params(&invalid).is_err(), "{invalid}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn control_stream_rejects_oversized_newline_free_line_without_oom() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let state = ControlState::new(
        "test",
        SharedControlRequests::new(Arc::new(RecordingControlRequestSink::default())),
    );
    let (mut client, server) = tokio::io::duplex(4096);
    let task = tokio::spawn(handle_client_stream_with_timeout(
        server,
        state,
        64,
        Duration::from_secs(1),
    ));
    client.write_all(&[b'x'; 256]).await.unwrap();
    let mut reply = Vec::new();
    client.read_to_end(&mut reply).await.unwrap();
    let response: ControlResponse =
        serde_json::from_str(std::str::from_utf8(&reply).unwrap().trim()).unwrap();
    assert!(!response.ok);
    assert!(response
        .error
        .as_deref()
        .is_some_and(|err| err.contains("exceeds 64 bytes")));
    assert!(task.await.unwrap().is_err());
}
