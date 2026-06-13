use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, OnceLock},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use bbb_control::{
    shared_snapshot, DefaultSpawn, NetCounters, NetVec3, PlayerAbilities, PlayerExperience,
    PlayerHealth, PlayerPose, RendererCounters, SharedSnapshot, SystemChatLine,
};
use bbb_net::{ConnectionOptions, NetCommand, NetEvent, PlayerMoveCommand};
use bbb_pack::{
    AtlasLayout, AtlasPacker, BiomeColorCatalog, BiomeColorProfile, BlockFaceTextures,
    BlockModelCatalog, BlockModelShape, GrassColorModifier, PackRoots, TerrainColorMaps,
};
use bbb_platform::WindowConfig;
use bbb_protocol::packets::{
    BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos,
    Direction as ProtocolDirection, InteractionHand, PickItemFromBlock, PlayerAction,
    PlayerActionKind, PlayerCommand, PlayerCommandAction, PlayerInput, PlayerPositionState,
    UseItem, UseItemOn,
};
use bbb_renderer::terrain::{
    build_terrain_mesh_layers_with_atlas, TerrainCell, TerrainChunkSnapshot, TerrainLight,
    TerrainMaterialClass, TerrainRenderShape, TerrainTextureAtlas, TerrainTint, TerrainUvRect,
};
use bbb_renderer::{CameraPose, ClearColor, SelectionOutline};
use bbb_world::{BlockPos, ChunkPos, WorldStore};
use clap::Parser;
use tokio::sync::mpsc;
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

const MAX_TERRAIN_UPLOAD_CHUNKS: usize = 49;
const INPUT_MOUSE_SENSITIVITY_DEGREES: f32 = 0.12;
const INPUT_WALK_SPEED_BLOCKS_PER_SECOND: f64 = 4.317;
const INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND: f64 = 5.612;
const MOVE_COMMAND_INTERVAL: Duration = Duration::from_millis(50);
const SELECTION_MAX_DISTANCE: f64 = 5.0;
const SELECTION_RAY_STEP: f64 = 0.05;
const SWAMP_GRASS_DARK: [u8; 3] = [0x4c, 0x76, 0x3c];
const SWAMP_GRASS_LIGHT: [u8; 3] = [0x6a, 0x70, 0x39];

static BIOME_INFO_NOISE: OnceLock<SimplexNoise> = OnceLock::new();

#[derive(Debug, Parser)]
#[command(name = "bbb-native")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:25565")]
    server: String,
    #[arg(long, default_value = "bbb-client")]
    username: String,
    #[arg(long)]
    probe_server: bool,
    #[arg(long)]
    connect_server: bool,
    #[arg(long)]
    control: Option<SocketAddr>,
    #[arg(long)]
    screenshot: Option<PathBuf>,
    #[arg(long)]
    exit_after_screenshot: bool,
}

#[derive(Debug, Default)]
struct TerrainUploadState {
    decoded_chunks: usize,
    block_updates_applied: usize,
    light_updates_applied: usize,
    biome_updates_applied: usize,
    uploaded_chunks: usize,
    observed_decoded_chunks: usize,
    observed_block_updates_applied: usize,
    observed_light_updates_applied: usize,
    observed_biome_updates_applied: usize,
    last_observed_change: Option<Instant>,
}

#[derive(Debug, Clone)]
struct TerrainTextureState {
    atlas: TerrainTextureAtlas,
    indices: HashMap<String, u32>,
    block_models: Option<BlockModelCatalog>,
    colormaps: Option<TerrainColorMaps>,
    biome_colors: Option<BiomeColorCatalog>,
    fallback_index: u32,
}

impl Default for TerrainTextureState {
    fn default() -> Self {
        Self {
            atlas: TerrainTextureAtlas::unit(),
            indices: HashMap::new(),
            block_models: None,
            colormaps: None,
            biome_colors: None,
            fallback_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BlockRenderPosition {
    x: i32,
    z: i32,
}

impl TerrainTextureState {
    fn from_layout(
        layout: &AtlasLayout,
        block_models: Option<BlockModelCatalog>,
        colormaps: Option<TerrainColorMaps>,
        biome_colors: Option<BiomeColorCatalog>,
    ) -> Self {
        let mut indices = HashMap::new();
        let mut rects = Vec::with_capacity(layout.sprites.len());
        for (index, sprite) in layout.sprites.iter().enumerate() {
            indices.insert(sprite.id.clone(), index as u32);
            rects.push(terrain_uv_rect(layout, sprite));
        }
        let fallback_index = indices.get("minecraft:block/stone").copied().unwrap_or(0);
        Self {
            atlas: TerrainTextureAtlas {
                rects,
                fallback_index,
            },
            indices,
            block_models,
            colormaps,
            biome_colors,
            fallback_index,
        }
    }

    fn texture_index(&self, texture_id: &str) -> u32 {
        self.indices
            .get(texture_id)
            .copied()
            .unwrap_or(self.fallback_index)
    }

    fn block_render_data(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_world::TerrainMaterialClass,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> ([u32; 6], [TerrainTint; 6], TerrainRenderShape) {
        let Some(block_name) = block_name else {
            return (
                [self.fallback_index; 6],
                [TerrainTint::WHITE; 6],
                TerrainRenderShape::Cube,
            );
        };

        if let Some(model) = self
            .block_models
            .as_ref()
            .and_then(|models| models.block_render_model(block_name, properties))
        {
            let texture_indices = self.face_texture_indices(&model.face_textures);
            let tint = self.face_tints(
                block_name,
                material,
                &model.face_textures,
                biome_id,
                position,
            );
            return (
                texture_indices,
                tint,
                self.terrain_render_shape_for_block(
                    block_name,
                    properties,
                    material,
                    model.shape,
                    texture_indices,
                    tint,
                    biome_id,
                    position,
                ),
            );
        }

        let all = self.texture_index(&block_fallback_texture_id(block_name));
        let texture_indices = [all; 6];
        let tint = self.fallback_face_tints(block_name, material, biome_id, position);
        (
            texture_indices,
            tint,
            self.terrain_render_shape_for_block(
                block_name,
                properties,
                material,
                BlockModelShape::Cube,
                texture_indices,
                tint,
                biome_id,
                position,
            ),
        )
    }

    fn face_texture_indices(&self, face_textures: &BlockFaceTextures) -> [u32; 6] {
        std::array::from_fn(|index| self.texture_index(&face_textures.textures[index]))
    }

    fn face_tints(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        face_textures: &BlockFaceTextures,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [TerrainTint; 6] {
        std::array::from_fn(|index| {
            self.block_tint(
                block_name,
                material,
                face_textures.tint_indices[index],
                biome_id,
                position,
            )
        })
    }

    fn terrain_render_shape_for_block(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        material: bbb_world::TerrainMaterialClass,
        model_shape: BlockModelShape,
        fallback_texture_indices: [u32; 6],
        fallback_tint: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainRenderShape {
        if matches!(material, bbb_world::TerrainMaterialClass::Fluid) {
            if let Some(shape) = fluid_render_shape(block_name, properties) {
                return shape;
            }
        }
        self.terrain_render_shape(
            block_name,
            material,
            model_shape,
            fallback_texture_indices,
            fallback_tint,
            biome_id,
            position,
        )
    }

    fn terrain_render_shape(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        shape: BlockModelShape,
        fallback_texture_indices: [u32; 6],
        fallback_tint: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainRenderShape {
        match shape {
            BlockModelShape::Cross => TerrainRenderShape::Cross,
            BlockModelShape::Box(model_box) => TerrainRenderShape::Box {
                from: model_box.from,
                to: model_box.to,
                face_present: model_box.face_present,
                face_uvs: model_box.face_uvs,
                face_cull: model_box.face_cull,
            },
            BlockModelShape::Boxes(model_boxes) => TerrainRenderShape::Boxes(
                model_boxes
                    .into_iter()
                    .map(|model_box| bbb_renderer::terrain::TerrainBox {
                        from: model_box.from,
                        to: model_box.to,
                        face_present: model_box.face_present,
                        face_uvs: model_box.face_uvs,
                        face_cull: model_box.face_cull,
                        texture_indices: self
                            .model_box_texture_indices(&model_box, fallback_texture_indices),
                        tint: self.model_box_face_tints(
                            block_name,
                            material,
                            &model_box,
                            fallback_tint,
                            biome_id,
                            position,
                        ),
                    })
                    .collect(),
            ),
            BlockModelShape::Cube | BlockModelShape::Custom => TerrainRenderShape::Cube,
        }
    }

    fn model_box_texture_indices(
        &self,
        model_box: &bbb_pack::BlockModelBox,
        fallback: [u32; 6],
    ) -> [u32; 6] {
        std::array::from_fn(|index| {
            model_box.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_index(texture))
                .unwrap_or(fallback[index])
        })
    }

    fn fallback_face_tints(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [TerrainTint; 6] {
        [self.block_tint(block_name, material, Some(0), biome_id, position); 6]
    }

    fn model_box_face_tints(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        model_box: &bbb_pack::BlockModelBox,
        fallback: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [TerrainTint; 6] {
        std::array::from_fn(|index| {
            if model_box.face_present[index] {
                self.block_tint(
                    block_name,
                    material,
                    model_box.face_tint_indices[index],
                    biome_id,
                    position,
                )
            } else {
                fallback[index]
            }
        })
    }

    fn block_tint(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        tint_index: Option<i32>,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainTint {
        if matches!(block_name, "minecraft:water" | "minecraft:water_cauldron") {
            return self.water_tint(biome_id);
        }
        if tint_index.is_none() {
            return TerrainTint::WHITE;
        }
        if matches!(block_name, "minecraft:spruce_leaves") {
            return TerrainTint::from_rgb_u8(0x61, 0x99, 0x61);
        }
        if matches!(block_name, "minecraft:birch_leaves") {
            return TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55);
        }
        if is_dry_foliage_tinted_block(block_name) {
            return self.dry_foliage_tint(biome_id);
        }
        if is_foliage_tinted_block(block_name) {
            return self.foliage_tint(biome_id);
        }
        if is_grass_tinted_block(block_name) {
            return self.grass_tint(biome_id, position);
        }
        if matches!(material, bbb_world::TerrainMaterialClass::Fluid) {
            return TerrainTint::WHITE;
        }
        TerrainTint::WHITE
    }

    fn grass_tint(
        &self,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainTint {
        let profile = self.biome_profile(biome_id);
        let base = profile.and_then(|profile| profile.grass_color).or_else(|| {
            self.colormaps.as_ref().map(|colormaps| {
                let (temperature, downfall) = biome_colormap_climate(profile);
                colormaps
                    .grass
                    .sample_temperature_downfall(temperature, downfall)
            })
        });
        let Some(base) = base else {
            return TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
        };
        terrain_tint_from_rgb(apply_grass_color_modifier(
            profile.map_or(GrassColorModifier::None, |profile| {
                profile.grass_color_modifier
            }),
            base,
            position,
        ))
    }

    fn foliage_tint(&self, biome_id: Option<i32>) -> TerrainTint {
        let profile = self.biome_profile(biome_id);
        profile
            .and_then(|profile| profile.foliage_color)
            .or_else(|| {
                self.colormaps.as_ref().map(|colormaps| {
                    let (temperature, downfall) = biome_colormap_climate(profile);
                    colormaps
                        .foliage
                        .sample_temperature_downfall(temperature, downfall)
                })
            })
            .map(terrain_tint_from_rgb)
            .unwrap_or_else(|| TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18))
    }

    fn dry_foliage_tint(&self, biome_id: Option<i32>) -> TerrainTint {
        let profile = self.biome_profile(biome_id);
        profile
            .and_then(|profile| profile.dry_foliage_color)
            .or_else(|| {
                self.colormaps
                    .as_ref()
                    .and_then(|colormaps| colormaps.dry_foliage.as_ref())
                    .map(|colormap| {
                        let (temperature, downfall) = biome_colormap_climate(profile);
                        colormap.sample_temperature_downfall(temperature, downfall)
                    })
            })
            .map(terrain_tint_from_rgb)
            .unwrap_or_else(|| TerrainTint::from_rgb_u8(0x5c, 0x3c, 0x32))
    }

    fn water_tint(&self, biome_id: Option<i32>) -> TerrainTint {
        self.biome_profile(biome_id)
            .and_then(|profile| profile.water_color)
            .map(terrain_tint_from_rgb)
            .unwrap_or_else(|| TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4))
    }

    fn biome_profile(&self, biome_id: Option<i32>) -> Option<&BiomeColorProfile> {
        self.biome_colors.as_ref()?.profile(biome_id?)
    }
}

#[derive(Debug, Clone, Default)]
struct ClientInputState {
    focused: bool,
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    jump: bool,
    sneak: bool,
    sprint: bool,
    mouse_delta_x: f64,
    mouse_delta_y: f64,
    last_step: Option<Instant>,
    last_move_command_at: Option<Instant>,
    last_move_command_pose: Option<PlayerPose>,
    destroying_block: Option<CrosshairBlockHit>,
    using_item: bool,
    prediction_sequence: i32,
}

impl ClientInputState {
    fn new(focused: bool) -> Self {
        Self {
            focused,
            ..Self::default()
        }
    }

    fn clear_pressed(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.jump = false;
        self.sneak = false;
        self.sprint = false;
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
    }

    fn next_prediction_sequence(&mut self) -> i32 {
        self.prediction_sequence = if self.prediction_sequence == i32::MAX {
            1
        } else {
            self.prediction_sequence + 1
        };
        self.prediction_sequence
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let runtime = tokio::runtime::Runtime::new()?;

    if args.probe_server {
        let options = ConnectionOptions::offline(&args.server, &args.username)?;
        let report = runtime.block_on(bbb_net::run_offline_probe(options))?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let snapshot = shared_snapshot(format!(
        "bbb-native {} / protocol {}",
        bbb_protocol::MC_VERSION,
        bbb_protocol::PROTOCOL_VERSION
    ));
    let mut world = WorldStore::new();
    let mut net_counters = NetCounters::default();
    let mut net_events = None;
    let mut net_commands = None;

    if args.connect_server {
        let options = ConnectionOptions::offline(&args.server, &args.username)?;
        let (tx, rx) = mpsc::channel(8192);
        let (command_tx, command_rx) = mpsc::channel(256);
        let disconnect_tx = tx.clone();
        runtime.spawn(async move {
            let reason = match bbb_net::run_offline_event_stream(options, tx, command_rx).await {
                Ok(()) => None,
                Err(err) => Some(err.to_string()),
            };
            let _ = disconnect_tx.send(NetEvent::Disconnected { reason }).await;
        });
        net_events = Some(rx);
        net_commands = Some(command_tx);
    }

    if let Some(addr) = args.control {
        let snapshot = Arc::clone(&snapshot);
        runtime.spawn(async move {
            if let Err(err) = bbb_control::serve(addr, snapshot).await {
                tracing::error!(?err, "control API stopped");
            }
        });
    }

    let event_loop = EventLoop::new()?;
    let config = WindowConfig::default();
    let window = WindowBuilder::new()
        .with_title(config.title.clone())
        .with_inner_size(config.physical_size())
        .build(&event_loop)
        .context("create native window")?;
    let mut input = ClientInputState::new(window.has_focus());
    let event_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        while event_proxy.send_event(()).is_ok() {
            thread::sleep(Duration::from_millis(16));
        }
    });

    let mut renderer = pollster::block_on(bbb_renderer::Renderer::new(&window))?;
    let terrain_textures = load_terrain_textures(&mut renderer);
    load_hud_textures(&mut renderer);
    let mut screenshot = args.screenshot;
    let screenshot_after_terrain = args.connect_server;
    let exit_after_screenshot = args.exit_after_screenshot;
    let mut terrain_upload = TerrainUploadState::default();
    let mut net_disconnect_requested = false;

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(size) => renderer.resize(size),
                WindowEvent::Focused(focused) => {
                    handle_focus_change(&mut input, &mut net_counters, &net_commands, focused)
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    handle_key_input(
                        &mut input,
                        &mut net_counters,
                        &net_commands,
                        event.physical_key,
                        event.state,
                    );
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    handle_mouse_input(
                        &mut input,
                        &world,
                        &mut net_counters,
                        &net_commands,
                        button,
                        state,
                    );
                }
                WindowEvent::RedrawRequested => {
                    if !pump_network_and_terrain(
                        &mut net_events,
                        &net_commands,
                        &mut input,
                        &mut world,
                        &mut renderer,
                        &mut net_counters,
                        &mut terrain_upload,
                        &terrain_textures,
                        &snapshot,
                    ) {
                        target.exit();
                        return;
                    }
                    renderer.set_clear_color(clear_color_for_world(&net_counters));

                    let terrain_ready_for_screenshot =
                        !screenshot_after_terrain || terrain_upload.uploaded_chunks > 0;
                    let cli_screenshot_path = screenshot
                        .as_deref()
                        .filter(|_| terrain_ready_for_screenshot);
                    let control_screenshot_path = if cli_screenshot_path.is_none() {
                        take_control_screenshot(&snapshot)
                    } else {
                        None
                    };
                    let render_path = cli_screenshot_path.or(control_screenshot_path.as_deref());
                    let wrote_cli_screenshot = cli_screenshot_path.is_some();

                    if let Err(err) = renderer.render(render_path) {
                        tracing::error!(?err, "render failed");
                        target.exit();
                        return;
                    }

                    if wrote_cli_screenshot {
                        screenshot = None;
                        if exit_after_screenshot {
                            target.exit();
                        }
                    }

                    if !publish_snapshot(&snapshot, renderer.counters(), &net_counters, &world) {
                        target.exit();
                    }
                }
                _ => {}
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                handle_mouse_motion(&mut input, delta);
            }
            Event::UserEvent(()) => {
                if !snapshot_is_running(&snapshot) {
                    request_net_disconnect(&net_commands, &mut net_disconnect_requested);
                    target.exit();
                    return;
                }
                window.request_redraw();
            }
            Event::AboutToWait => {
                if !snapshot_is_running(&snapshot) {
                    request_net_disconnect(&net_commands, &mut net_disconnect_requested);
                    target.exit();
                    return;
                }
                if !pump_network_and_terrain(
                    &mut net_events,
                    &net_commands,
                    &mut input,
                    &mut world,
                    &mut renderer,
                    &mut net_counters,
                    &mut terrain_upload,
                    &terrain_textures,
                    &snapshot,
                ) {
                    target.exit();
                    return;
                }
                window.request_redraw();
            }
            Event::LoopExiting => {
                request_net_disconnect(&net_commands, &mut net_disconnect_requested);
                if let Ok(mut guard) = snapshot.write() {
                    guard.app.running = false;
                    guard.net.connected = false;
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

fn snapshot_is_running(snapshot: &SharedSnapshot) -> bool {
    snapshot
        .read()
        .map(|guard| guard.app.running)
        .unwrap_or(false)
}

fn request_net_disconnect(net_commands: &Option<mpsc::Sender<NetCommand>>, requested: &mut bool) {
    if *requested {
        return;
    }
    *requested = true;
    if let Some(tx) = net_commands {
        let _ = tx.try_send(NetCommand::Disconnect);
    }
}

fn take_control_screenshot(snapshot: &SharedSnapshot) -> Option<PathBuf> {
    snapshot
        .write()
        .ok()?
        .screenshot_request
        .take()
        .map(PathBuf::from)
}

fn pump_network_and_terrain(
    net_events: &mut Option<mpsc::Receiver<NetEvent>>,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    input: &mut ClientInputState,
    world: &mut WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    net_counters: &mut NetCounters,
    terrain_upload: &mut TerrainUploadState,
    terrain_textures: &TerrainTextureState,
    snapshot: &SharedSnapshot,
) -> bool {
    if let Some(rx) = net_events.as_mut() {
        drain_net_events(rx, world, net_counters);
    }
    advance_player_input(input, net_counters, net_commands, Instant::now());
    renderer.set_hud_health(net_counters.player_health.map(|health| health.health));
    renderer.set_hud_food(net_counters.player_health.map(|health| health.food));
    renderer.set_hud_experience_progress(
        net_counters
            .player_experience
            .map(|experience| experience.progress),
    );
    renderer.set_hud_selected_slot(net_counters.selected_hotbar_slot);
    renderer.set_camera_pose(net_counters.player_pose.map(camera_pose_from_player));
    renderer.set_selection_outline(selection_outline_from_crosshair(
        world,
        net_counters.player_pose,
    ));
    maybe_upload_decoded_terrain(
        world,
        renderer,
        net_counters,
        terrain_upload,
        terrain_textures,
    );
    publish_snapshot(snapshot, renderer.counters(), net_counters, world)
}

fn handle_focus_change(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    focused: bool,
) {
    input.focused = focused;
    if !focused {
        let before = player_input_from_state(input);
        input.clear_pressed();
        let after = player_input_from_state(input);
        if after != before {
            queue_player_input_command(counters, net_commands, after);
        }
        if let Some(hit) = input.destroying_block.take() {
            queue_player_action_command(
                counters,
                net_commands,
                PlayerActionKind::AbortDestroyBlock,
                hit.pos,
                ProtocolDirection::Down,
                0,
            );
        }
        if input.using_item {
            input.using_item = false;
            queue_zero_pos_player_action_command(
                counters,
                net_commands,
                PlayerActionKind::ReleaseUseItem,
            );
        }
    }
}

fn handle_key_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    let pressed = matches!(state, ElementState::Pressed);
    let PhysicalKey::Code(code) = physical_key else {
        return;
    };

    if pressed {
        if let Some(slot) = hotbar_slot_for_key(code) {
            select_hotbar_slot(counters, net_commands, slot);
            return;
        }
        match code {
            KeyCode::KeyQ => {
                let action = if input.sprint {
                    PlayerActionKind::DropAllItems
                } else {
                    PlayerActionKind::DropItem
                };
                queue_zero_pos_player_action_command(counters, net_commands, action);
                return;
            }
            KeyCode::KeyF => {
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::SwapItemWithOffhand,
                );
                return;
            }
            KeyCode::KeyE => {
                queue_player_command_action(
                    counters,
                    net_commands,
                    PlayerCommandAction::OpenInventory,
                    0,
                );
                return;
            }
            _ => {}
        }
    }

    let before = player_input_from_state(input);
    let handled = match code {
        KeyCode::KeyW | KeyCode::ArrowUp => {
            input.forward = pressed;
            true
        }
        KeyCode::KeyS | KeyCode::ArrowDown => {
            input.backward = pressed;
            true
        }
        KeyCode::KeyA | KeyCode::ArrowLeft => {
            input.left = pressed;
            true
        }
        KeyCode::KeyD | KeyCode::ArrowRight => {
            input.right = pressed;
            true
        }
        KeyCode::Space => {
            input.jump = pressed;
            true
        }
        KeyCode::ShiftLeft | KeyCode::ShiftRight => {
            input.sneak = pressed;
            true
        }
        KeyCode::ControlLeft | KeyCode::ControlRight => {
            input.sprint = pressed;
            true
        }
        _ => false,
    };
    if handled {
        let after = player_input_from_state(input);
        if after != before {
            queue_player_input_command(counters, net_commands, after);
            if before.sprint != after.sprint {
                queue_sprint_command(counters, net_commands, after.sprint);
            }
        }
    }
}

fn player_input_from_state(input: &ClientInputState) -> PlayerInput {
    PlayerInput {
        forward: input.forward,
        backward: input.backward,
        left: input.left,
        right: input.right,
        jump: input.jump,
        shift: input.sneak,
        sprint: input.sprint,
    }
}

fn queue_player_input_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    input: PlayerInput,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::PlayerInput(input)).is_ok() {
            counters.player_input_commands_queued += 1;
        }
    }
}

fn queue_sprint_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    sprinting: bool,
) {
    let action = if sprinting {
        PlayerCommandAction::StartSprinting
    } else {
        PlayerCommandAction::StopSprinting
    };
    queue_player_command_action(counters, net_commands, action, 0);
}

fn queue_player_command_action(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    action: PlayerCommandAction,
    data: i32,
) {
    let (Some(tx), Some(entity_id)) = (net_commands, counters.player_entity_id) else {
        return;
    };
    let command = PlayerCommand {
        entity_id,
        action,
        data,
    };
    if tx.try_send(NetCommand::PlayerCommand(command)).is_ok() {
        counters.player_command_commands_queued += 1;
    }
}

fn hotbar_slot_for_key(code: KeyCode) -> Option<u8> {
    match code {
        KeyCode::Digit1 => Some(0),
        KeyCode::Digit2 => Some(1),
        KeyCode::Digit3 => Some(2),
        KeyCode::Digit4 => Some(3),
        KeyCode::Digit5 => Some(4),
        KeyCode::Digit6 => Some(5),
        KeyCode::Digit7 => Some(6),
        KeyCode::Digit8 => Some(7),
        KeyCode::Digit9 => Some(8),
        _ => None,
    }
}

fn select_hotbar_slot(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot: u8,
) {
    let slot = slot.min(8);
    counters.selected_hotbar_slot = slot;
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::SetHeldSlot(slot)).is_ok() {
            counters.held_slot_commands_queued += 1;
        }
    }
}

fn handle_mouse_motion(input: &mut ClientInputState, delta: (f64, f64)) {
    if !input.focused {
        return;
    }
    input.mouse_delta_x += delta.0;
    input.mouse_delta_y += delta.1;
}

fn handle_mouse_input(
    input: &mut ClientInputState,
    world: &WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
) {
    if !input.focused {
        return;
    }
    match (button, state) {
        (MouseButton::Left, ElementState::Pressed) => {
            if let Some(hit) = crosshair_block_hit_from_world(world, counters.player_pose) {
                let sequence = input.next_prediction_sequence();
                queue_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::StartDestroyBlock,
                    hit.pos,
                    hit.face,
                    sequence,
                );
                input.destroying_block = Some(hit);
            }
            queue_swing_command(counters, net_commands, InteractionHand::MainHand);
        }
        (MouseButton::Left, ElementState::Released) => {
            if let Some(hit) = input.destroying_block.take() {
                queue_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::AbortDestroyBlock,
                    hit.pos,
                    ProtocolDirection::Down,
                    0,
                );
            }
        }
        (MouseButton::Right, ElementState::Pressed) => {
            if let Some(hit) = crosshair_block_hit_from_world(world, counters.player_pose) {
                let sequence = input.next_prediction_sequence();
                queue_use_item_on_command(counters, net_commands, hit, sequence);
            } else if let Some(pose) = counters.player_pose {
                let sequence = input.next_prediction_sequence();
                input.using_item = queue_use_item_command(
                    counters,
                    net_commands,
                    InteractionHand::MainHand,
                    pose,
                    sequence,
                );
            }
        }
        (MouseButton::Right, ElementState::Released) => {
            if input.using_item {
                input.using_item = false;
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::ReleaseUseItem,
                );
            }
        }
        (MouseButton::Middle, ElementState::Pressed) => {
            if let Some(hit) = crosshair_block_hit_from_world(world, counters.player_pose) {
                queue_pick_item_from_block_command(counters, net_commands, hit.pos, input.sprint);
            }
        }
        _ => {}
    }
}

fn queue_player_action_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    action_kind: PlayerActionKind,
    pos: BlockPos,
    direction: ProtocolDirection,
    sequence: i32,
) {
    let Some(tx) = net_commands else {
        return;
    };
    let action = PlayerAction {
        action: action_kind,
        pos: protocol_block_pos_from_world(pos),
        direction,
        sequence,
    };
    if tx.try_send(NetCommand::PlayerAction(action)).is_ok() {
        counters.player_action_commands_queued += 1;
    }
}

fn queue_zero_pos_player_action_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    action_kind: PlayerActionKind,
) {
    queue_player_action_command(
        counters,
        net_commands,
        action_kind,
        BlockPos { x: 0, y: 0, z: 0 },
        ProtocolDirection::Down,
        0,
    );
}

fn queue_swing_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hand: InteractionHand,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::Swing(hand)).is_ok() {
            counters.swing_commands_queued += 1;
        }
    }
}

fn queue_use_item_on_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hit: CrosshairBlockHit,
    sequence: i32,
) {
    if let Some(tx) = net_commands {
        let packet = UseItemOn {
            hand: InteractionHand::MainHand,
            hit: protocol_block_hit_result_from_crosshair_hit(hit),
            sequence,
        };
        if tx.try_send(NetCommand::UseItemOn(packet)).is_ok() {
            counters.use_item_on_commands_queued += 1;
        }
    }
}

fn queue_use_item_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hand: InteractionHand,
    pose: PlayerPose,
    sequence: i32,
) -> bool {
    if let Some(tx) = net_commands {
        let packet = UseItem {
            hand,
            sequence,
            y_rot: pose.y_rot,
            x_rot: pose.x_rot,
        };
        if tx.try_send(NetCommand::UseItem(packet)).is_ok() {
            counters.use_item_commands_queued += 1;
            return true;
        }
    }
    false
}

fn queue_pick_item_from_block_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    pos: BlockPos,
    include_data: bool,
) {
    if let Some(tx) = net_commands {
        let packet = PickItemFromBlock {
            pos: protocol_block_pos_from_world(pos),
            include_data,
        };
        if tx.try_send(NetCommand::PickItemFromBlock(packet)).is_ok() {
            counters.pick_item_from_block_commands_queued += 1;
        }
    }
}

fn advance_player_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    now: Instant,
) {
    let dt_seconds = input
        .last_step
        .and_then(|last| now.checked_duration_since(last))
        .unwrap_or_default()
        .as_secs_f64()
        .min(0.25);
    input.last_step = Some(now);

    let Some(current_pose) = counters.player_pose else {
        input.mouse_delta_x = 0.0;
        input.mouse_delta_y = 0.0;
        return;
    };

    let pose = integrate_player_input_pose(current_pose, input, dt_seconds);
    input.mouse_delta_x = 0.0;
    input.mouse_delta_y = 0.0;
    counters.player_pose = Some(pose);
    maybe_queue_player_move_command(input, counters, net_commands, pose, now);
}

fn integrate_player_input_pose(
    mut pose: PlayerPose,
    input: &ClientInputState,
    dt_seconds: f64,
) -> PlayerPose {
    if input.focused {
        pose.y_rot =
            wrap_degrees(pose.y_rot + input.mouse_delta_x as f32 * INPUT_MOUSE_SENSITIVITY_DEGREES);
        pose.x_rot = (pose.x_rot + input.mouse_delta_y as f32 * INPUT_MOUSE_SENSITIVITY_DEGREES)
            .clamp(-90.0, 90.0);
    }

    let forward_input = axis(input.forward, input.backward);
    let strafe_input = axis(input.right, input.left);
    let vertical_input = axis(input.jump, input.sneak);
    let speed = if input.sprint {
        INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND
    } else {
        INPUT_WALK_SPEED_BLOCKS_PER_SECOND
    };
    let yaw = f64::from(pose.y_rot).to_radians();
    let forward = (-yaw.sin(), yaw.cos());
    let right = (-yaw.cos(), -yaw.sin());
    let mut move_x = forward.0 * forward_input + right.0 * strafe_input;
    let mut move_z = forward.1 * forward_input + right.1 * strafe_input;
    let horizontal_len = (move_x * move_x + move_z * move_z).sqrt();
    if horizontal_len > f64::EPSILON {
        move_x /= horizontal_len;
        move_z /= horizontal_len;
    }

    pose.position.x += move_x * speed * dt_seconds;
    pose.position.y += vertical_input * speed * dt_seconds;
    pose.position.z += move_z * speed * dt_seconds;
    pose.delta_movement = NetVec3 {
        x: move_x * speed / 20.0,
        y: vertical_input * speed / 20.0,
        z: move_z * speed / 20.0,
    };
    pose
}

fn maybe_queue_player_move_command(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    pose: PlayerPose,
    now: Instant,
) {
    let Some(tx) = net_commands else {
        return;
    };
    let command_due = input
        .last_move_command_at
        .and_then(|last| now.checked_duration_since(last))
        .map_or(true, |elapsed| elapsed >= MOVE_COMMAND_INTERVAL);
    if !command_due || input.last_move_command_pose == Some(pose) {
        return;
    }

    let command = NetCommand::MovePlayer(PlayerMoveCommand {
        state: player_position_state_from_pose(pose),
        on_ground: pose.delta_movement.y.abs() <= f64::EPSILON,
        horizontal_collision: false,
    });
    if tx.try_send(command).is_ok() {
        input.last_move_command_at = Some(now);
        input.last_move_command_pose = Some(pose);
        counters.player_move_commands_queued += 1;
    }
}

fn axis(positive: bool, negative: bool) -> f64 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn wrap_degrees(degrees: f32) -> f32 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

fn maybe_upload_decoded_terrain(
    world: &WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    counters: &NetCounters,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
) {
    let world_counters = world.counters();
    let chunk_count = world.chunk_count();
    if chunk_count == 0
        || (upload.decoded_chunks == world_counters.chunks_decoded
            && upload.block_updates_applied == world_counters.block_updates_applied
            && upload.light_updates_applied == world_counters.light_updates_applied
            && upload.biome_updates_applied == world_counters.biome_updates_applied
            && upload.uploaded_chunks == chunk_count)
    {
        return;
    }
    if upload.observed_decoded_chunks != world_counters.chunks_decoded
        || upload.observed_block_updates_applied != world_counters.block_updates_applied
        || upload.observed_light_updates_applied != world_counters.light_updates_applied
        || upload.observed_biome_updates_applied != world_counters.biome_updates_applied
    {
        upload.observed_decoded_chunks = world_counters.chunks_decoded;
        upload.observed_block_updates_applied = world_counters.block_updates_applied;
        upload.observed_light_updates_applied = world_counters.light_updates_applied;
        upload.observed_biome_updates_applied = world_counters.biome_updates_applied;
        upload.last_observed_change = Some(Instant::now());
        return;
    }
    if upload
        .last_observed_change
        .is_some_and(|changed_at| changed_at.elapsed() < Duration::from_millis(250))
    {
        return;
    }

    let center = counters
        .chunk_cache_center
        .or(counters.first_chunk)
        .unwrap_or_else(|| {
            world
                .chunk_positions()
                .into_iter()
                .next()
                .unwrap_or(ChunkPos { x: 0, z: 0 })
        });
    let mut positions = world.chunk_positions();
    positions.sort_by_key(|pos| chunk_distance_key(*pos, center));

    let mut snapshots: Vec<_> = positions
        .into_iter()
        .take(MAX_TERRAIN_UPLOAD_CHUNKS)
        .filter_map(|pos| world.extract_terrain_chunk(pos))
        .collect();
    if snapshots.is_empty() {
        return;
    }

    snapshots.sort_by_key(|snapshot| chunk_distance_key(snapshot.pos, center));
    let renderer_snapshots: Vec<_> = snapshots
        .into_iter()
        .map(|snapshot| convert_terrain_snapshot(snapshot, textures))
        .collect();
    let meshes = build_terrain_mesh_layers_with_atlas(&renderer_snapshots, &textures.atlas);

    renderer.upload_terrain_mesh_layers(meshes);
    upload.decoded_chunks = world_counters.chunks_decoded;
    upload.block_updates_applied = world_counters.block_updates_applied;
    upload.light_updates_applied = world_counters.light_updates_applied;
    upload.biome_updates_applied = world_counters.biome_updates_applied;
    upload.uploaded_chunks = chunk_count;
}

fn chunk_distance_key(pos: ChunkPos, center: ChunkPos) -> i64 {
    let dx = i64::from(pos.x - center.x);
    let dz = i64::from(pos.z - center.z);
    dx * dx + dz * dz
}

fn load_terrain_textures(renderer: &mut bbb_renderer::Renderer) -> TerrainTextureState {
    match try_load_terrain_textures(renderer) {
        Ok(textures) => textures,
        Err(err) => {
            tracing::warn!(?err, "falling back to default terrain texture atlas");
            TerrainTextureState::default()
        }
    }
}

fn try_load_terrain_textures(renderer: &mut bbb_renderer::Renderer) -> Result<TerrainTextureState> {
    let roots = PackRoots::discover()?;
    let images = roots.load_block_texture_images()?;
    let block_models = roots.load_block_model_catalog()?;
    let colormaps = match roots.load_terrain_colormaps() {
        Ok(colormaps) => Some(colormaps),
        Err(err) => {
            tracing::warn!(?err, "falling back to constant terrain tint colors");
            None
        }
    };
    let biome_colors = match roots.load_biome_color_catalog() {
        Ok(biome_colors) => Some(biome_colors),
        Err(err) => {
            tracing::warn!(?err, "falling back to default terrain biome tint");
            None
        }
    };
    let atlas = AtlasPacker::new(4096, 1)?.stitch(&images)?;
    renderer.upload_terrain_texture_atlas(atlas.layout.width, atlas.layout.height, &atlas.rgba)?;
    tracing::info!(
        width = atlas.layout.width,
        height = atlas.layout.height,
        sprites = atlas.layout.sprites.len(),
        blockstates = block_models.len(),
        colormaps = colormaps.is_some(),
        biome_colors = biome_colors.as_ref().map_or(0, |colors| colors.len()),
        "loaded terrain texture atlas"
    );
    Ok(TerrainTextureState::from_layout(
        &atlas.layout,
        Some(block_models),
        colormaps,
        biome_colors,
    ))
}

fn load_hud_textures(renderer: &mut bbb_renderer::Renderer) {
    if let Err(err) = try_load_hud_textures(renderer) {
        tracing::warn!(?err, "continuing without vanilla HUD sprites");
    }
}

fn try_load_hud_textures(renderer: &mut bbb_renderer::Renderer) -> Result<()> {
    let roots = PackRoots::discover()?;
    let crosshair = roots.load_gui_sprite_image("hud/crosshair")?;
    renderer.upload_hud_crosshair(crosshair.width, crosshair.height, &crosshair.rgba)?;
    let hotbar = roots.load_gui_sprite_image("hud/hotbar")?;
    renderer.upload_hud_hotbar(hotbar.width, hotbar.height, &hotbar.rgba)?;
    let hotbar_selection = roots.load_gui_sprite_image("hud/hotbar_selection")?;
    renderer.upload_hud_hotbar_selection(
        hotbar_selection.width,
        hotbar_selection.height,
        &hotbar_selection.rgba,
    )?;
    let experience_background = roots.load_gui_sprite_image("hud/experience_bar_background")?;
    renderer.upload_hud_experience_background(
        experience_background.width,
        experience_background.height,
        &experience_background.rgba,
    )?;
    let experience_progress = roots.load_gui_sprite_image("hud/experience_bar_progress")?;
    renderer.upload_hud_experience_progress(
        experience_progress.width,
        experience_progress.height,
        &experience_progress.rgba,
    )?;
    let heart_container = roots.load_gui_sprite_image("hud/heart/container")?;
    renderer.upload_hud_heart_container(
        heart_container.width,
        heart_container.height,
        &heart_container.rgba,
    )?;
    let heart_full = roots.load_gui_sprite_image("hud/heart/full")?;
    renderer.upload_hud_heart_full(heart_full.width, heart_full.height, &heart_full.rgba)?;
    let heart_half = roots.load_gui_sprite_image("hud/heart/half")?;
    renderer.upload_hud_heart_half(heart_half.width, heart_half.height, &heart_half.rgba)?;
    let food_empty = roots.load_gui_sprite_image("hud/food_empty")?;
    renderer.upload_hud_food_empty(food_empty.width, food_empty.height, &food_empty.rgba)?;
    let food_full = roots.load_gui_sprite_image("hud/food_full")?;
    renderer.upload_hud_food_full(food_full.width, food_full.height, &food_full.rgba)?;
    let food_half = roots.load_gui_sprite_image("hud/food_half")?;
    renderer.upload_hud_food_half(food_half.width, food_half.height, &food_half.rgba)?;
    tracing::info!(
        crosshair = ?(crosshair.width, crosshair.height),
        hotbar = ?(hotbar.width, hotbar.height),
        experience = ?(experience_background.width, experience_background.height),
        heart = ?(heart_full.width, heart_full.height),
        food = ?(food_full.width, food_full.height),
        "loaded vanilla HUD sprites"
    );
    Ok(())
}

fn terrain_uv_rect(layout: &AtlasLayout, sprite: &bbb_pack::AtlasSprite) -> TerrainUvRect {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let x0 = sprite.content.x as f32;
    let y0 = sprite.content.y as f32;
    let x1 = (sprite.content.x + sprite.content.width) as f32;
    let y1 = (sprite.content.y + sprite.content.height) as f32;
    TerrainUvRect {
        min: [(x0 + 0.5) / width, (y0 + 0.5) / height],
        max: [(x1 - 0.5) / width, (y1 - 0.5) / height],
    }
}

fn block_fallback_texture_id(block_name: &str) -> String {
    let stem = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    format!("minecraft:block/{stem}")
}

fn terrain_tint_from_rgb(rgb: [u8; 3]) -> TerrainTint {
    TerrainTint::from_rgb_u8(rgb[0], rgb[1], rgb[2])
}

fn biome_colormap_climate(profile: Option<&BiomeColorProfile>) -> (f32, f32) {
    profile
        .map(|profile| {
            (
                profile.temperature.clamp(0.0, 1.0),
                profile.downfall.clamp(0.0, 1.0),
            )
        })
        .unwrap_or((0.5, 1.0))
}

fn apply_grass_color_modifier(
    modifier: GrassColorModifier,
    base: [u8; 3],
    position: Option<BlockRenderPosition>,
) -> [u8; 3] {
    match modifier {
        GrassColorModifier::None => base,
        GrassColorModifier::DarkForest => {
            let base = rgb_to_u24(base);
            u24_to_rgb(((base & 0xfe_fe_fe) + 0x28_34_0a) >> 1)
        }
        GrassColorModifier::Swamp => {
            let position = position.unwrap_or(BlockRenderPosition { x: 0, z: 0 });
            if biome_info_noise(position.x as f64, position.z as f64) < -0.1 {
                SWAMP_GRASS_DARK
            } else {
                SWAMP_GRASS_LIGHT
            }
        }
    }
}

fn biome_info_noise(x: f64, z: f64) -> f64 {
    BIOME_INFO_NOISE
        .get_or_init(|| SimplexNoise::new(LegacyRandomSource::new(2345)))
        .get_value(x * 0.0225, z * 0.0225)
}

#[derive(Debug, Clone)]
struct LegacyRandomSource {
    seed: u64,
}

impl LegacyRandomSource {
    const MASK: u64 = (1u64 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;

    fn new(seed: i64) -> Self {
        let mut random = Self { seed: 0 };
        random.set_seed(seed);
        random
    }

    fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK;
    }

    fn next(&mut self, bits: u8) -> i32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return (((bound as i64) * i64::from(self.next(31))) >> 31) as i32;
        }

        loop {
            let sample = self.next(31);
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo;
            }
        }
    }

    fn next_double(&mut self) -> f64 {
        let upper = i64::from(self.next(26));
        let lower = i64::from(self.next(27));
        (((upper << 27) + lower) as f64) * 1.110223e-16
    }
}

#[derive(Debug, Clone)]
struct SimplexNoise {
    p: [i32; 256],
}

impl SimplexNoise {
    const GRADIENT: [[i32; 3]; 16] = [
        [1, 1, 0],
        [-1, 1, 0],
        [1, -1, 0],
        [-1, -1, 0],
        [1, 0, 1],
        [-1, 0, 1],
        [1, 0, -1],
        [-1, 0, -1],
        [0, 1, 1],
        [0, -1, 1],
        [0, 1, -1],
        [0, -1, -1],
        [1, 1, 0],
        [0, -1, 1],
        [-1, 1, 0],
        [0, -1, -1],
    ];
    const F2: f64 = 0.366_025_403_784_438_6;
    const G2: f64 = 0.211_324_865_405_187_13;

    fn new(mut random: LegacyRandomSource) -> Self {
        let _xo = random.next_double() * 256.0;
        let _yo = random.next_double() * 256.0;
        let _zo = random.next_double() * 256.0;
        let mut p = [0; 256];
        for (index, value) in p.iter_mut().enumerate() {
            *value = index as i32;
        }
        for ix in 0..256usize {
            let offset = random.next_int((256 - ix) as i32) as usize;
            p.swap(ix, ix + offset);
        }
        Self { p }
    }

    fn p(&self, x: i32) -> i32 {
        self.p[(x & 0xff) as usize]
    }

    fn get_value(&self, xin: f64, yin: f64) -> f64 {
        let s = (xin + yin) * Self::F2;
        let i = floor_i32(xin + s);
        let j = floor_i32(yin + s);
        let t = f64::from(i + j) * Self::G2;
        let x0 = xin - (f64::from(i) - t);
        let y0 = yin - (f64::from(j) - t);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - f64::from(i1) + Self::G2;
        let y1 = y0 - f64::from(j1) + Self::G2;
        let x2 = x0 - 1.0 + 2.0 * Self::G2;
        let y2 = y0 - 1.0 + 2.0 * Self::G2;
        let ii = i & 0xff;
        let jj = j & 0xff;
        let gi0 = self.p(ii + self.p(jj)) % 12;
        let gi1 = self.p(ii + i1 + self.p(jj + j1)) % 12;
        let gi2 = self.p(ii + 1 + self.p(jj + 1)) % 12;
        let n0 = simplex_corner_noise(gi0 as usize, x0, y0, 0.0, 0.5);
        let n1 = simplex_corner_noise(gi1 as usize, x1, y1, 0.0, 0.5);
        let n2 = simplex_corner_noise(gi2 as usize, x2, y2, 0.0, 0.5);
        70.0 * (n0 + n1 + n2)
    }
}

fn floor_i32(value: f64) -> i32 {
    value.floor() as i32
}

fn simplex_corner_noise(index: usize, x: f64, y: f64, z: f64, base: f64) -> f64 {
    let mut t0 = base - x * x - y * y - z * z;
    if t0 < 0.0 {
        0.0
    } else {
        t0 *= t0;
        t0 * t0 * simplex_dot(SimplexNoise::GRADIENT[index], x, y, z)
    }
}

fn simplex_dot(g: [i32; 3], x: f64, y: f64, z: f64) -> f64 {
    f64::from(g[0]) * x + f64::from(g[1]) * y + f64::from(g[2]) * z
}

fn rgb_to_u24(rgb: [u8; 3]) -> u32 {
    u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2])
}

fn u24_to_rgb(value: u32) -> [u8; 3] {
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

fn is_grass_tinted_block(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:grass_block"
            | "minecraft:short_grass"
            | "minecraft:tall_grass"
            | "minecraft:fern"
            | "minecraft:large_fern"
            | "minecraft:potted_fern"
            | "minecraft:bush"
            | "minecraft:sugar_cane"
    ) || block_name.contains("vine")
}

fn is_foliage_tinted_block(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:oak_leaves"
            | "minecraft:jungle_leaves"
            | "minecraft:acacia_leaves"
            | "minecraft:dark_oak_leaves"
            | "minecraft:mangrove_leaves"
            | "minecraft:vine"
    )
}

fn is_dry_foliage_tinted_block(block_name: &str) -> bool {
    matches!(block_name, "minecraft:leaf_litter")
}

fn fluid_render_shape(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<TerrainRenderShape> {
    if !matches!(block_name, "minecraft:water" | "minecraft:lava") {
        return None;
    }

    let level = properties
        .get("level")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    Some(fluid_box_shape(fluid_height_units(level)))
}

fn fluid_height_units(level: u8) -> u8 {
    let amount = match level {
        0 => 8,
        1..=7 => 8 - level,
        _ => 8,
    };
    ((amount as u16 * 16 + 4) / 9).clamp(1, 16) as u8
}

fn fluid_box_shape(height: u8) -> TerrainRenderShape {
    let height = height.clamp(1, 16);
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    let side_v0 = 16 - height;
    face_uvs[2] = [0, side_v0, 16, 16];
    face_uvs[3] = [0, side_v0, 16, 16];
    face_uvs[4] = [0, side_v0, 16, 16];
    face_uvs[5] = [0, side_v0, 16, 16];
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, height, 16],
        face_present: [true; 6],
        face_uvs,
        face_cull: [true; 6],
    }
}

fn convert_terrain_snapshot(
    snapshot: bbb_world::TerrainChunkSnapshot,
    textures: &TerrainTextureState,
) -> TerrainChunkSnapshot {
    let chunk_origin_x = snapshot.pos.x * 16;
    let chunk_origin_z = snapshot.pos.z * 16;
    let cells = snapshot
        .cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            let local_x = (index % 16) as i32;
            let local_z = ((index / 16) % 16) as i32;
            let position = BlockRenderPosition {
                x: chunk_origin_x + local_x,
                z: chunk_origin_z + local_z,
            };
            let world_material = cell.material;
            let (texture_indices, tint, render_shape) = textures.block_render_data(
                cell.block_name.as_deref(),
                &cell.block_properties,
                world_material,
                cell.biome_id,
                Some(position),
            );
            TerrainCell {
                block_state_id: cell.block_state_id,
                texture_indices,
                render_shape,
                material: match cell.material {
                    bbb_world::TerrainMaterialClass::Empty => TerrainMaterialClass::Empty,
                    bbb_world::TerrainMaterialClass::Opaque => TerrainMaterialClass::Opaque,
                    bbb_world::TerrainMaterialClass::Cutout => TerrainMaterialClass::Cutout,
                    bbb_world::TerrainMaterialClass::Fluid => TerrainMaterialClass::Fluid,
                    bbb_world::TerrainMaterialClass::Translucent => {
                        TerrainMaterialClass::Translucent
                    }
                },
                light: TerrainLight {
                    sky: cell.light.sky,
                    block: cell.light.block,
                },
                tint,
            }
        })
        .collect();
    TerrainChunkSnapshot::new(
        snapshot.pos.x,
        snapshot.pos.z,
        snapshot.min_y,
        snapshot.height,
        cells,
    )
}

fn drain_net_events(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
) -> usize {
    let mut drained = 0;
    while drained < 4096 {
        let event = match rx.try_recv() {
            Ok(event) => event,
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => {
                counters.connected = false;
                break;
            }
        };
        drained += 1;

        match event {
            NetEvent::Connected => {
                counters.connected = true;
                counters.last_error = None;
            }
            NetEvent::Disconnected { reason } => {
                counters.connected = false;
                counters.last_error = reason;
            }
            NetEvent::StateChanged { state } => {
                counters.state = Some(format!("{state:?}"));
            }
            NetEvent::CompressionSet { threshold } => {
                counters.compression_threshold = Some(threshold);
            }
            NetEvent::PacketSeen { .. } => {
                counters.packets_seen += 1;
            }
            NetEvent::ContainerClose(update) => {
                world.apply_container_close(update);
            }
            NetEvent::ContainerSetContent(update) => {
                world.apply_container_set_content(update);
            }
            NetEvent::ContainerSetData(update) => {
                world.apply_container_set_data(update);
            }
            NetEvent::ContainerSetSlot(update) => {
                world.apply_container_set_slot(update);
            }
            NetEvent::OpenScreen(update) => {
                world.apply_open_screen(update);
            }
            NetEvent::SetCursorItem(update) => {
                world.apply_set_cursor_item(update);
            }
            NetEvent::SetPlayerInventory(update) => {
                world.apply_set_player_inventory(update);
            }
            NetEvent::AddEntity(entity) => {
                world.apply_add_entity(entity);
            }
            NetEvent::EntityAnimation(update) => {
                world.apply_entity_animation(update);
            }
            NetEvent::EntityEvent(update) => {
                world.apply_entity_event(update);
            }
            NetEvent::HurtAnimation(update) => {
                world.apply_hurt_animation(update);
            }
            NetEvent::MoveEntity(update) => {
                world.apply_entity_move(update);
            }
            NetEvent::EntityPositionSync(update) => {
                world.apply_entity_position_sync(update);
            }
            NetEvent::RemoveEntities(update) => {
                world.apply_remove_entities(update);
            }
            NetEvent::RotateHead(update) => {
                world.apply_rotate_head(update);
            }
            NetEvent::SetEntityMotion(update) => {
                world.apply_set_entity_motion(update);
            }
            NetEvent::SetEntityLink(update) => {
                world.apply_set_entity_link(update);
            }
            NetEvent::SetEquipment(update) => {
                world.apply_set_equipment(update);
            }
            NetEvent::SetPassengers(update) => {
                world.apply_set_passengers(update);
            }
            NetEvent::UpdateAttributes(update) => {
                world.apply_update_attributes(update);
            }
            NetEvent::SetEntityData(update) => {
                world.apply_set_entity_data(update);
            }
            NetEvent::TeleportEntity(update) => {
                world.apply_teleport_entity(update);
            }
            NetEvent::RegistryData {
                registry,
                raw_payload_len,
            } => {
                world.record_registry(registry, raw_payload_len);
                counters.registries_seen = world.counters().registries_seen;
            }
            NetEvent::Login(login) => {
                counters.player_entity_id = Some(login.player_id);
                world.apply_login(&login);
            }
            NetEvent::Respawn(respawn) => {
                world.apply_respawn(&respawn);
            }
            NetEvent::PlayerPosition(update) => {
                apply_player_position_update(counters, update);
            }
            NetEvent::PlayerRotation(update) => {
                apply_player_rotation_update(counters, update);
            }
            NetEvent::PlayerAbilities(abilities) => {
                apply_player_abilities_update(counters, abilities);
            }
            NetEvent::PlayerHealth(health) => {
                apply_player_health_update(counters, health);
            }
            NetEvent::PlayerExperience(experience) => {
                apply_player_experience_update(counters, experience);
            }
            NetEvent::HeldSlot(slot) => {
                apply_held_slot_update(counters, slot);
            }
            NetEvent::SetDefaultSpawnPosition(spawn) => {
                apply_default_spawn_update(counters, spawn);
            }
            NetEvent::SetSimulationDistance(distance) => {
                apply_simulation_distance_update(counters, distance);
            }
            NetEvent::SystemChat(chat) => {
                apply_system_chat_update(counters, chat);
            }
            NetEvent::BlockChangedAck(ack) => {
                apply_block_changed_ack(counters, ack);
            }
            NetEvent::BlockEntityData(update) => match world.apply_block_entity_data(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::GameEvent(event) => {
                apply_game_event(counters, event);
            }
            NetEvent::SetTime(time) => {
                apply_world_time_update(counters, time);
            }
            NetEvent::LevelChunkWithLight(chunk) => {
                match world.insert_level_chunk_with_light(chunk) {
                    Ok(pos) => {
                        counters.first_chunk.get_or_insert(pos);
                    }
                    Err(err) => {
                        counters.last_error = Some(err.to_string());
                    }
                }
            }
            NetEvent::LightUpdate(update) => match world.apply_light_update(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::ChunksBiomes(update) => match world.apply_biome_update(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::ForgetLevelChunk(update) => {
                world.forget_chunk(ChunkPos {
                    x: update.pos.x,
                    z: update.pos.z,
                });
            }
            NetEvent::BlockUpdate(update) => {
                world.apply_block_update(update);
            }
            NetEvent::SectionBlocksUpdate(update) => {
                world.apply_section_blocks_update(update);
            }
            NetEvent::SetChunkCacheCenter(update) => {
                counters.chunk_cache_center = Some(ChunkPos {
                    x: update.chunk_x,
                    z: update.chunk_z,
                });
            }
            NetEvent::SetChunkCacheRadius(update) => {
                counters.chunk_cache_radius = Some(update.radius);
            }
        }
    }
    drained
}

fn apply_world_time_update(counters: &mut NetCounters, time: bbb_protocol::packets::PlayTime) {
    let day_time = time
        .clock_updates
        .first()
        .map(|clock| clock.total_ticks)
        .unwrap_or(time.game_time);
    counters.world_time = Some(bbb_control::WorldTime {
        game_time: time.game_time,
        day_time,
        clock_updates: time.clock_updates.len(),
    });
    counters.world_time_packets += 1;
}

fn apply_game_event(counters: &mut NetCounters, event: bbb_protocol::packets::GameEvent) {
    counters.weather.last_game_event_id = Some(event.event_id);
    counters.weather.last_game_event_param = event.param;
    counters.game_event_packets += 1;

    match event.event_id {
        1 => {
            counters.weather.raining = true;
            counters.weather.rain_level = counters.weather.rain_level.max(1.0);
        }
        2 => {
            counters.weather.raining = false;
            counters.weather.rain_level = 0.0;
            counters.weather.thunder_level = 0.0;
        }
        7 => {
            counters.weather.rain_level = event.param.clamp(0.0, 1.0);
            counters.weather.raining = counters.weather.rain_level > 0.0;
        }
        8 => {
            counters.weather.thunder_level = event.param.clamp(0.0, 1.0);
        }
        _ => {}
    }
}

fn apply_block_changed_ack(
    counters: &mut NetCounters,
    ack: bbb_protocol::packets::BlockChangedAck,
) {
    counters.block_changed_ack_packets += 1;
    counters.last_block_changed_ack_sequence = Some(ack.sequence);
}

fn apply_player_abilities_update(
    counters: &mut NetCounters,
    abilities: bbb_protocol::packets::PlayerAbilities,
) {
    counters.player_abilities = Some(PlayerAbilities {
        invulnerable: abilities.invulnerable,
        flying: abilities.flying,
        can_fly: abilities.can_fly,
        instabuild: abilities.instabuild,
        flying_speed: abilities.flying_speed,
        walking_speed: abilities.walking_speed,
    });
    counters.player_abilities_packets += 1;
}

fn apply_default_spawn_update(
    counters: &mut NetCounters,
    spawn: bbb_protocol::packets::SetDefaultSpawnPosition,
) {
    counters.default_spawn = Some(DefaultSpawn {
        dimension: spawn.dimension,
        pos: BlockPos {
            x: spawn.pos.x,
            y: spawn.pos.y,
            z: spawn.pos.z,
        },
        yaw: spawn.yaw,
        pitch: spawn.pitch,
    });
    counters.default_spawn_position_packets += 1;
}

fn apply_simulation_distance_update(
    counters: &mut NetCounters,
    distance: bbb_protocol::packets::SetSimulationDistance,
) {
    counters.simulation_distance = Some(distance.distance);
    counters.simulation_distance_packets += 1;
}

fn apply_system_chat_update(counters: &mut NetCounters, chat: bbb_protocol::packets::SystemChat) {
    counters.last_system_chat = Some(SystemChatLine {
        content: chat.content,
        overlay: chat.overlay,
    });
    counters.system_chat_packets += 1;
}

fn clear_color_for_world(counters: &NetCounters) -> ClearColor {
    let day_time = counters
        .world_time
        .map(|time| time.day_time)
        .unwrap_or(6000);
    let rain = counters.weather.rain_level.clamp(0.0, 1.0) as f64;
    let thunder = counters.weather.thunder_level.clamp(0.0, 1.0) as f64;
    clear_color_for_day_time(day_time, rain, thunder)
}

fn clear_color_for_day_time(day_time: i64, rain_level: f64, thunder_level: f64) -> ClearColor {
    let phase = day_time.rem_euclid(24_000) as f64 / 24_000.0;
    let noon_aligned = (phase - 0.25) * std::f64::consts::TAU;
    let daylight = ((noon_aligned.cos() + 1.0) * 0.5).powf(0.65);
    let weather_dim = (1.0 - rain_level * 0.25 - thunder_level * 0.45).clamp(0.25, 1.0);
    let night = [0.015, 0.025, 0.055];
    let day = [0.50, 0.72, 0.95];
    ClearColor {
        r: (night[0] + (day[0] - night[0]) * daylight) * weather_dim,
        g: (night[1] + (day[1] - night[1]) * daylight) * weather_dim,
        b: (night[2] + (day[2] - night[2]) * daylight) * weather_dim,
        a: 1.0,
    }
}

fn apply_player_health_update(
    counters: &mut NetCounters,
    health: bbb_protocol::packets::PlayerHealth,
) {
    counters.player_health = Some(PlayerHealth {
        health: health.health,
        food: health.food,
        saturation: health.saturation,
    });
    counters.player_health_packets += 1;
}

fn apply_player_experience_update(
    counters: &mut NetCounters,
    experience: bbb_protocol::packets::PlayerExperience,
) {
    counters.player_experience = Some(PlayerExperience {
        progress: experience.progress,
        level: experience.level,
        total: experience.total,
    });
    counters.player_experience_packets += 1;
}

fn apply_held_slot_update(counters: &mut NetCounters, slot: bbb_protocol::packets::SetHeldSlot) {
    if (0..=8).contains(&slot.slot) {
        counters.selected_hotbar_slot = slot.slot as u8;
    }
    counters.held_slot_packets += 1;
}

fn apply_player_position_update(
    counters: &mut NetCounters,
    update: bbb_protocol::packets::PlayerPositionUpdate,
) {
    let current = counters
        .player_pose
        .map(player_position_state_from_pose)
        .unwrap_or_default();
    let state = update.apply_to_state(current);

    counters.player_pose = Some(PlayerPose {
        position: net_vec3_from_protocol(state.position),
        delta_movement: net_vec3_from_protocol(state.delta_movement),
        y_rot: state.y_rot,
        x_rot: state.x_rot,
        last_teleport_id: update.id,
    });
    counters.player_position_packets += 1;
}

fn apply_player_rotation_update(
    counters: &mut NetCounters,
    update: bbb_protocol::packets::PlayerRotationUpdate,
) {
    let current = counters
        .player_pose
        .map(player_position_state_from_pose)
        .unwrap_or_default();
    let state = update.apply_to_state(current);
    let last_teleport_id = counters
        .player_pose
        .map(|pose| pose.last_teleport_id)
        .unwrap_or_default();

    counters.player_pose = Some(PlayerPose {
        position: net_vec3_from_protocol(state.position),
        delta_movement: net_vec3_from_protocol(state.delta_movement),
        y_rot: state.y_rot,
        x_rot: state.x_rot,
        last_teleport_id,
    });
    counters.player_rotation_packets += 1;
}

fn player_position_state_from_pose(player: PlayerPose) -> PlayerPositionState {
    PlayerPositionState {
        position: protocol_vec3_from_net(player.position),
        delta_movement: protocol_vec3_from_net(player.delta_movement),
        y_rot: player.y_rot,
        x_rot: player.x_rot,
    }
}

fn protocol_vec3_from_net(vec: NetVec3) -> bbb_protocol::packets::Vec3d {
    bbb_protocol::packets::Vec3d {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

fn net_vec3_from_protocol(vec: bbb_protocol::packets::Vec3d) -> NetVec3 {
    NetVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

fn camera_pose_from_player(player: PlayerPose) -> CameraPose {
    CameraPose {
        position: [
            player.position.x as f32,
            player.position.y as f32,
            player.position.z as f32,
        ],
        y_rot: player.y_rot,
        x_rot: player.x_rot,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    }
}

fn selection_outline_from_crosshair(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<SelectionOutline> {
    let hit = crosshair_block_hit_from_world(world, pose)?;
    Some(selection_outline_for_block(hit.pos))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CrosshairBlockHit {
    pos: BlockPos,
    face: ProtocolDirection,
    cursor: [f32; 3],
    inside: bool,
}

fn crosshair_block_hit_from_world(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit(pose?, SELECTION_MAX_DISTANCE, SELECTION_RAY_STEP, |pos| {
        world.probe_block(pos).map(|probe| probe.material)
    })
}

fn raycast_crosshair_block<F>(
    pose: PlayerPose,
    max_distance: f64,
    step: f64,
    material_at: F,
) -> Option<BlockPos>
where
    F: FnMut(BlockPos) -> Option<bbb_world::TerrainMaterialClass>,
{
    raycast_crosshair_block_hit(pose, max_distance, step, material_at).map(|hit| hit.pos)
}

fn raycast_crosshair_block_hit<F>(
    pose: PlayerPose,
    max_distance: f64,
    step: f64,
    mut material_at: F,
) -> Option<CrosshairBlockHit>
where
    F: FnMut(BlockPos) -> Option<bbb_world::TerrainMaterialClass>,
{
    if max_distance <= 0.0 || step <= 0.0 {
        return None;
    }

    let eye = [
        pose.position.x,
        pose.position.y + f64::from(CameraPose::STANDING_EYE_HEIGHT),
        pose.position.z,
    ];
    let direction = look_direction_from_player_pose(pose);
    if direction == [0.0, 0.0, 0.0] {
        return None;
    }

    let mut distance = 0.0;
    let mut last_pos = None;
    while distance <= max_distance {
        let pos = BlockPos {
            x: (eye[0] + direction[0] * distance).floor() as i32,
            y: (eye[1] + direction[1] * distance).floor() as i32,
            z: (eye[2] + direction[2] * distance).floor() as i32,
        };
        if last_pos != Some(pos) {
            if material_at(pos).is_some_and(is_selectable_crosshair_material) {
                return Some(CrosshairBlockHit {
                    pos,
                    face: block_hit_face(last_pos, pos, direction),
                    cursor: block_hit_cursor(eye, direction, distance, pos),
                    inside: last_pos.is_none(),
                });
            }
            last_pos = Some(pos);
        }
        distance += step;
    }

    None
}

fn block_hit_face(
    previous: Option<BlockPos>,
    current: BlockPos,
    direction: [f64; 3],
) -> ProtocolDirection {
    if let Some(previous) = previous {
        let dx = current.x - previous.x;
        let dy = current.y - previous.y;
        let dz = current.z - previous.z;
        let mut axis = None;
        if dx != 0 {
            axis = Some((0, direction[0].abs(), dx));
        }
        if dy != 0 && axis.is_none_or(|(_, best, _)| direction[1].abs() > best) {
            axis = Some((1, direction[1].abs(), dy));
        }
        if dz != 0 && axis.is_none_or(|(_, best, _)| direction[2].abs() > best) {
            axis = Some((2, direction[2].abs(), dz));
        }
        if let Some((axis, _, delta)) = axis {
            return face_for_axis_delta(axis, delta);
        }
    }
    face_opposing_dominant_direction(direction)
}

fn face_for_axis_delta(axis: u8, delta: i32) -> ProtocolDirection {
    match (axis, delta.signum()) {
        (0, 1) => ProtocolDirection::West,
        (0, -1) => ProtocolDirection::East,
        (1, 1) => ProtocolDirection::Down,
        (1, -1) => ProtocolDirection::Up,
        (2, 1) => ProtocolDirection::North,
        (2, -1) => ProtocolDirection::South,
        _ => ProtocolDirection::North,
    }
}

fn face_opposing_dominant_direction(direction: [f64; 3]) -> ProtocolDirection {
    let ax = direction[0].abs();
    let ay = direction[1].abs();
    let az = direction[2].abs();
    if ax >= ay && ax >= az {
        if direction[0] >= 0.0 {
            ProtocolDirection::West
        } else {
            ProtocolDirection::East
        }
    } else if ay >= az {
        if direction[1] >= 0.0 {
            ProtocolDirection::Down
        } else {
            ProtocolDirection::Up
        }
    } else if direction[2] >= 0.0 {
        ProtocolDirection::North
    } else {
        ProtocolDirection::South
    }
}

fn block_hit_cursor(eye: [f64; 3], direction: [f64; 3], distance: f64, pos: BlockPos) -> [f32; 3] {
    [
        ((eye[0] + direction[0] * distance) - f64::from(pos.x)).clamp(0.0, 1.0) as f32,
        ((eye[1] + direction[1] * distance) - f64::from(pos.y)).clamp(0.0, 1.0) as f32,
        ((eye[2] + direction[2] * distance) - f64::from(pos.z)).clamp(0.0, 1.0) as f32,
    ]
}

fn protocol_block_pos_from_world(pos: BlockPos) -> ProtocolBlockPos {
    ProtocolBlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn protocol_block_hit_result_from_crosshair_hit(hit: CrosshairBlockHit) -> ProtocolBlockHitResult {
    ProtocolBlockHitResult {
        pos: protocol_block_pos_from_world(hit.pos),
        direction: hit.face,
        cursor_x: hit.cursor[0],
        cursor_y: hit.cursor[1],
        cursor_z: hit.cursor[2],
        inside: hit.inside,
        world_border_hit: false,
    }
}

fn look_direction_from_player_pose(pose: PlayerPose) -> [f64; 3] {
    let yaw = f64::from(pose.y_rot).to_radians();
    let pitch = f64::from(pose.x_rot).to_radians();
    let cos_pitch = pitch.cos();
    let x = -yaw.sin() * cos_pitch;
    let y = -pitch.sin();
    let z = yaw.cos() * cos_pitch;
    let len = (x * x + y * y + z * z).sqrt();
    if len <= f64::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [x / len, y / len, z / len]
    }
}

fn is_selectable_crosshair_material(material: bbb_world::TerrainMaterialClass) -> bool {
    matches!(
        material,
        bbb_world::TerrainMaterialClass::Opaque
            | bbb_world::TerrainMaterialClass::Cutout
            | bbb_world::TerrainMaterialClass::Translucent
    )
}

fn selection_outline_for_block(pos: BlockPos) -> SelectionOutline {
    SelectionOutline {
        min: [pos.x as f32, pos.y as f32, pos.z as f32],
        max: [(pos.x + 1) as f32, (pos.y + 1) as f32, (pos.z + 1) as f32],
    }
}

fn publish_snapshot(
    snapshot: &SharedSnapshot,
    renderer: RendererCounters,
    net: &NetCounters,
    world: &WorldStore,
) -> bool {
    if let Ok(mut guard) = snapshot.write() {
        guard.renderer = renderer;
        guard.net = net.clone();
        guard.world = world.counters();
        guard.world_store = world.clone();
        guard.app.running
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
    };

    #[test]
    fn fluid_height_units_follow_vanilla_legacy_level_amounts() {
        assert_eq!(fluid_height_units(0), 14);
        assert_eq!(fluid_height_units(1), 12);
        assert_eq!(fluid_height_units(2), 11);
        assert_eq!(fluid_height_units(3), 9);
        assert_eq!(fluid_height_units(4), 7);
        assert_eq!(fluid_height_units(5), 5);
        assert_eq!(fluid_height_units(6), 4);
        assert_eq!(fluid_height_units(7), 2);
        assert_eq!(fluid_height_units(8), 14);
        assert_eq!(fluid_height_units(15), 14);
    }

    #[test]
    fn water_level_shape_uses_cropped_fluid_box() {
        let shape = fluid_render_shape("minecraft:water", &properties([("level", "3")]))
            .expect("water has a fluid render shape");

        assert_eq!(
            shape,
            TerrainRenderShape::Box {
                from: [0, 0, 0],
                to: [16, 9, 16],
                face_present: [true; 6],
                face_uvs: [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [0, 7, 16, 16],
                    [0, 7, 16, 16],
                    [0, 7, 16, 16],
                    [0, 7, 16, 16],
                ],
                face_cull: [true; 6],
            }
        );
    }

    #[test]
    fn fluid_material_overrides_particle_only_model_shape() {
        let textures = TerrainTextureState::default();
        let shape = textures.terrain_render_shape_for_block(
            "minecraft:lava",
            &properties([("level", "8")]),
            bbb_world::TerrainMaterialClass::Fluid,
            BlockModelShape::Custom,
            [0; 6],
            [TerrainTint::WHITE; 6],
            None,
            None,
        );

        assert!(matches!(
            shape,
            TerrainRenderShape::Box {
                to: [16, 14, 16],
                ..
            }
        ));

        let non_fluid_shape = textures.terrain_render_shape_for_block(
            "minecraft:lava",
            &properties([("level", "8")]),
            bbb_world::TerrainMaterialClass::Opaque,
            BlockModelShape::Custom,
            [0; 6],
            [TerrainTint::WHITE; 6],
            None,
            None,
        );
        assert_eq!(non_fluid_shape, TerrainRenderShape::Cube);
    }

    #[test]
    fn model_boxes_preserve_per_element_textures_and_tints() {
        let mut texture_state = TerrainTextureState::default();
        texture_state
            .indices
            .insert("minecraft:block/base".to_string(), 1);
        texture_state
            .indices
            .insert("minecraft:block/overlay".to_string(), 2);
        let base = block_model_box_with_face_texture(
            bbb_pack::BlockModelFace::North,
            "minecraft:block/base",
            None,
        );
        let overlay = block_model_box_with_face_texture(
            bbb_pack::BlockModelFace::North,
            "minecraft:block/overlay",
            Some(0),
        );

        let shape = texture_state.terrain_render_shape_for_block(
            "minecraft:grass_block",
            &BTreeMap::new(),
            bbb_world::TerrainMaterialClass::Opaque,
            BlockModelShape::Boxes(vec![base, overlay]),
            [0; 6],
            [TerrainTint::WHITE; 6],
            Some(4),
            None,
        );

        let TerrainRenderShape::Boxes(boxes) = shape else {
            panic!("expected boxes render shape");
        };
        let north = bbb_pack::BlockModelFace::North.index();
        assert_eq!(boxes[0].texture_indices[north], 1);
        assert_eq!(boxes[0].tint[north], TerrainTint::WHITE);
        assert_eq!(boxes[1].texture_indices[north], 2);
        assert_eq!(
            boxes[1].tint[north],
            TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
        );
    }

    #[test]
    fn block_tint_uses_default_vanilla_color_classes() {
        let textures = TerrainTextureState::default();
        assert_eq!(
            textures.block_tint(
                "minecraft:stone",
                bbb_world::TerrainMaterialClass::Opaque,
                None,
                None,
                None
            ),
            TerrainTint::WHITE
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:oak_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:spruce_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x61, 0x99, 0x61)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:birch_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:leaf_litter",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x5c, 0x3c, 0x32)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:water",
                bbb_world::TerrainMaterialClass::Fluid,
                None,
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4)
        );
    }

    #[test]
    fn block_tint_samples_loaded_colormaps() {
        let mut textures = TerrainTextureState::default();
        textures.colormaps = Some(TerrainColorMaps {
            grass: flat_colormap([10, 20, 30]),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        });

        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                Some(4),
                None
            ),
            TerrainTint::from_rgb_u8(10, 20, 30)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:oak_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(4),
                None
            ),
            TerrainTint::from_rgb_u8(40, 50, 60)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:leaf_litter",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(4),
                None
            ),
            TerrainTint::from_rgb_u8(70, 80, 90)
        );
    }

    #[test]
    fn block_tint_uses_loaded_biome_color_profiles() {
        let mut textures = TerrainTextureState::default();
        textures.colormaps = Some(TerrainColorMaps {
            grass: flat_colormap([10, 20, 30]),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        });
        textures.biome_colors = Some(BiomeColorCatalog::new([BiomeColorProfile {
            id: 42,
            name: "minecraft:test_biome".to_string(),
            temperature: 0.2,
            downfall: 0.3,
            grass_color: Some([1, 2, 3]),
            foliage_color: Some([4, 5, 6]),
            dry_foliage_color: Some([7, 8, 9]),
            water_color: Some([10, 11, 12]),
            grass_color_modifier: GrassColorModifier::None,
        }]));

        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                Some(42),
                Some(BlockRenderPosition { x: 0, z: 0 })
            ),
            TerrainTint::from_rgb_u8(1, 2, 3)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:oak_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(42),
                None
            ),
            TerrainTint::from_rgb_u8(4, 5, 6)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:leaf_litter",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(42),
                None
            ),
            TerrainTint::from_rgb_u8(7, 8, 9)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:water",
                bbb_world::TerrainMaterialClass::Fluid,
                None,
                Some(42),
                None
            ),
            TerrainTint::from_rgb_u8(10, 11, 12)
        );
    }

    #[test]
    fn biome_climate_changes_colormap_sample() {
        let mut textures = TerrainTextureState::default();
        textures.colormaps = Some(TerrainColorMaps {
            grass: coordinate_colormap(),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        });
        textures.biome_colors = Some(BiomeColorCatalog::new([BiomeColorProfile {
            id: 7,
            name: "minecraft:dry_biome".to_string(),
            temperature: 0.0,
            downfall: 1.0,
            grass_color: None,
            foliage_color: None,
            dry_foliage_color: None,
            water_color: None,
            grass_color_modifier: GrassColorModifier::None,
        }]));

        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                Some(7),
                None
            ),
            TerrainTint::from_rgb_u8(30, 60, 6)
        );
    }

    #[test]
    fn biome_info_noise_matches_vanilla_seed_samples() {
        assert_eq!(biome_info_noise(0.0, 0.0), 0.0);
        let dark_sample = biome_info_noise(-496.0, -512.0);
        assert!((dark_sample - -0.102_904_227_905_454_12).abs() < 1.0e-12);
    }

    #[test]
    fn swamp_grass_modifier_uses_biome_info_noise() {
        assert_eq!(
            apply_grass_color_modifier(
                GrassColorModifier::Swamp,
                [1, 2, 3],
                Some(BlockRenderPosition { x: 0, z: 0 })
            ),
            SWAMP_GRASS_LIGHT
        );
        assert_eq!(
            apply_grass_color_modifier(
                GrassColorModifier::Swamp,
                [1, 2, 3],
                Some(BlockRenderPosition { x: -496, z: -512 })
            ),
            SWAMP_GRASS_DARK
        );
    }

    #[test]
    fn player_position_updates_absolute_and_relative_pose() {
        let mut counters = NetCounters::default();
        apply_player_position_update(
            &mut counters,
            player_position_update(1, [10.0, 64.0, -5.0], [0.125, 0.0, 0.0], 90.0, 15.0, 0),
        );
        let pose = counters.player_pose.unwrap();
        assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
        assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
        assert_eq!(pose.y_rot, 90.0);
        assert_eq!(pose.x_rot, 15.0);
        assert_eq!(pose.last_teleport_id, 1);
        assert_eq!(counters.player_position_packets, 1);

        apply_player_position_update(
            &mut counters,
            player_position_update(
                2,
                [1.5, -2.0, 7.0],
                [0.25, 0.5, 0.75],
                20.0,
                -120.0,
                PLAYER_RELATIVE_X
                    | PLAYER_RELATIVE_Y_ROT
                    | PLAYER_RELATIVE_X_ROT
                    | PLAYER_RELATIVE_DELTA_X,
            ),
        );
        let pose = counters.player_pose.unwrap();
        assert_eq!(pose.position, vec3(11.5, -2.0, 7.0));
        assert_eq!(pose.delta_movement, vec3(0.375, 0.5, 0.75));
        assert_eq!(pose.y_rot, 110.0);
        assert_eq!(pose.x_rot, -90.0);
        assert_eq!(pose.last_teleport_id, 2);
        assert_eq!(counters.player_position_packets, 2);
    }

    #[test]
    fn player_rotation_updates_pose_orientation() {
        let mut counters = NetCounters {
            player_pose: Some(PlayerPose {
                position: vec3(10.0, 64.0, -5.0),
                delta_movement: vec3(0.125, 0.0, 0.0),
                y_rot: 90.0,
                x_rot: 15.0,
                last_teleport_id: 7,
            }),
            ..NetCounters::default()
        };

        apply_player_rotation_update(
            &mut counters,
            bbb_protocol::packets::PlayerRotationUpdate {
                y_rot: 20.0,
                relative_y: true,
                x_rot: -120.0,
                relative_x: false,
            },
        );

        let pose = counters.player_pose.unwrap();
        assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
        assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
        assert_eq!(pose.y_rot, 110.0);
        assert_eq!(pose.x_rot, -90.0);
        assert_eq!(pose.last_teleport_id, 7);
        assert_eq!(counters.player_rotation_packets, 1);
    }

    #[test]
    fn player_health_updates_snapshot_counters() {
        let mut counters = NetCounters::default();

        apply_player_health_update(
            &mut counters,
            bbb_protocol::packets::PlayerHealth {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            },
        );

        assert_eq!(
            counters.player_health,
            Some(PlayerHealth {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            })
        );
        assert_eq!(counters.player_health_packets, 1);
    }

    #[test]
    fn player_experience_updates_snapshot_counters() {
        let mut counters = NetCounters::default();

        apply_player_experience_update(
            &mut counters,
            bbb_protocol::packets::PlayerExperience {
                progress: 0.75,
                level: 8,
                total: 123,
            },
        );

        assert_eq!(
            counters.player_experience,
            Some(PlayerExperience {
                progress: 0.75,
                level: 8,
                total: 123,
            })
        );
        assert_eq!(counters.player_experience_packets, 1);
    }

    #[test]
    fn held_slot_updates_snapshot_counters() {
        let mut counters = NetCounters::default();

        apply_held_slot_update(
            &mut counters,
            bbb_protocol::packets::SetHeldSlot { slot: 5 },
        );

        assert_eq!(counters.selected_hotbar_slot, 5);
        assert_eq!(counters.held_slot_packets, 1);

        apply_held_slot_update(
            &mut counters,
            bbb_protocol::packets::SetHeldSlot { slot: 99 },
        );

        assert_eq!(counters.selected_hotbar_slot, 5);
        assert_eq!(counters.held_slot_packets, 2);
    }

    #[test]
    fn player_abilities_spawn_distance_and_chat_update_snapshot_counters() {
        let mut counters = NetCounters::default();

        apply_player_abilities_update(
            &mut counters,
            bbb_protocol::packets::PlayerAbilities {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            },
        );
        apply_default_spawn_update(
            &mut counters,
            bbb_protocol::packets::SetDefaultSpawnPosition {
                dimension: "minecraft:overworld".to_string(),
                pos: ProtocolBlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                yaw: 90.0,
                pitch: -10.0,
            },
        );
        apply_simulation_distance_update(
            &mut counters,
            bbb_protocol::packets::SetSimulationDistance { distance: 12 },
        );
        apply_system_chat_update(
            &mut counters,
            bbb_protocol::packets::SystemChat {
                content: "Server restarting".to_string(),
                overlay: true,
            },
        );

        assert_eq!(
            counters.player_abilities,
            Some(PlayerAbilities {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            })
        );
        assert_eq!(
            counters.default_spawn,
            Some(DefaultSpawn {
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
        assert_eq!(counters.simulation_distance, Some(12));
        assert_eq!(
            counters.last_system_chat,
            Some(SystemChatLine {
                content: "Server restarting".to_string(),
                overlay: true,
            })
        );
        assert_eq!(counters.player_abilities_packets, 1);
        assert_eq!(counters.default_spawn_position_packets, 1);
        assert_eq!(counters.simulation_distance_packets, 1);
        assert_eq!(counters.system_chat_packets, 1);
    }

    #[test]
    fn block_changed_ack_updates_snapshot_counters() {
        let mut counters = NetCounters::default();

        apply_block_changed_ack(
            &mut counters,
            bbb_protocol::packets::BlockChangedAck { sequence: 17 },
        );

        assert_eq!(counters.block_changed_ack_packets, 1);
        assert_eq!(counters.last_block_changed_ack_sequence, Some(17));
    }

    #[test]
    fn prediction_sequence_starts_at_one_and_wraps_positive() {
        let mut input = ClientInputState::new(true);

        assert_eq!(input.next_prediction_sequence(), 1);
        assert_eq!(input.next_prediction_sequence(), 2);

        input.prediction_sequence = i32::MAX;
        assert_eq!(input.next_prediction_sequence(), 1);
    }

    #[test]
    fn digit_key_selects_hotbar_slot_and_queues_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::Digit5),
            ElementState::Pressed,
        );

        assert_eq!(counters.selected_hotbar_slot, 4);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(4));
    }

    #[test]
    fn drop_key_queues_drop_item_action() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyQ),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::DropItem,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn control_drop_key_queues_drop_all_items_action() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sprint = true;
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyQ),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::DropAllItems,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn swap_offhand_key_queues_swap_action() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyF),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::SwapItemWithOffhand,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn inventory_key_queues_open_inventory_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters {
            player_entity_id: Some(77),
            ..NetCounters::default()
        };

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyE),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_command_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(PlayerCommand {
                entity_id: 77,
                action: PlayerCommandAction::OpenInventory,
                data: 0,
            })
        );
    }

    #[test]
    fn movement_key_changes_queue_player_input_commands() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput {
                forward: true,
                ..PlayerInput::default()
            })
        );

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.player_input_commands_queued, 1);

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Released,
        );

        assert_eq!(counters.player_input_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput::default())
        );
    }

    #[test]
    fn sprint_key_queues_player_input_and_sprint_commands() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters {
            player_entity_id: Some(77),
            ..NetCounters::default()
        };

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::ControlLeft),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(counters.player_command_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput {
                sprint: true,
                ..PlayerInput::default()
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(PlayerCommand {
                entity_id: 77,
                action: PlayerCommandAction::StartSprinting,
                data: 0,
            })
        );

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::ControlLeft),
            ElementState::Released,
        );

        assert_eq!(counters.player_input_commands_queued, 2);
        assert_eq!(counters.player_command_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput::default())
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerCommand(PlayerCommand {
                entity_id: 77,
                action: PlayerCommandAction::StopSprinting,
                data: 0,
            })
        );
    }

    #[test]
    fn sprint_key_without_player_entity_id_only_queues_input() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();

        handle_key_input(
            &mut input,
            &mut counters,
            &commands,
            PhysicalKey::Code(KeyCode::ControlLeft),
            ElementState::Pressed,
        );

        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(counters.player_command_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput {
                sprint: true,
                ..PlayerInput::default()
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn focus_loss_clears_pressed_input_and_queues_release() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.forward = true;
        input.jump = true;
        input.sprint = true;
        let mut counters = NetCounters::default();

        handle_focus_change(&mut input, &mut counters, &commands, false);

        assert!(!input.focused);
        assert_eq!(player_input_from_state(&input), PlayerInput::default());
        assert_eq!(counters.player_input_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerInput(PlayerInput::default())
        );
    }

    #[test]
    fn focus_loss_aborts_destroying_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroying_block = Some(CrosshairBlockHit {
            pos: BlockPos { x: 4, y: 70, z: -6 },
            face: ProtocolDirection::North,
            cursor: [0.5, 0.5, 0.0],
            inside: false,
        });
        let mut counters = NetCounters::default();

        handle_focus_change(&mut input, &mut counters, &commands, false);

        assert!(input.destroying_block.is_none());
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 4, y: 70, z: -6 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn focus_loss_releases_using_item() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.using_item = true;
        let mut counters = NetCounters::default();

        handle_focus_change(&mut input, &mut counters, &commands, false);

        assert!(!input.using_item);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::ReleaseUseItem,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn left_mouse_press_queues_main_hand_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert!(input.destroying_block.is_none());

        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.swing_commands_queued, 1);
    }

    #[test]
    fn unfocused_mouse_press_does_not_queue_swing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(false);
        let world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn queues_start_destroy_block_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let hit = CrosshairBlockHit {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            face: ProtocolDirection::West,
            cursor: [0.0, 0.5, 0.5],
            inside: false,
        };

        queue_player_action_command(
            &mut counters,
            &commands,
            PlayerActionKind::StartDestroyBlock,
            hit.pos,
            hit.face,
            3,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
                direction: ProtocolDirection::West,
                sequence: 3,
            })
        );
    }

    #[test]
    fn left_mouse_release_aborts_destroying_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroying_block = Some(CrosshairBlockHit {
            pos: BlockPos { x: 2, y: 65, z: -3 },
            face: ProtocolDirection::East,
            cursor: [1.0, 0.5, 0.5],
            inside: false,
        });
        let world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );

        assert!(input.destroying_block.is_none());
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 2, y: 65, z: -3 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn queues_use_item_on_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let hit = CrosshairBlockHit {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            face: ProtocolDirection::South,
            cursor: [0.25, 0.5, 0.75],
            inside: false,
        };

        queue_use_item_on_command(&mut counters, &commands, hit, 5);

        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: ProtocolBlockHitResult {
                    pos: ProtocolBlockPos {
                        x: -5,
                        y: 70,
                        z: 12
                    },
                    direction: ProtocolDirection::South,
                    cursor_x: 0.25,
                    cursor_y: 0.5,
                    cursor_z: 0.75,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 5,
            })
        );
    }

    #[test]
    fn queues_pick_item_from_block_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_pick_item_from_block_command(
            &mut counters,
            &commands,
            BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            true,
        );

        assert_eq!(counters.pick_item_from_block_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromBlock(PickItemFromBlock {
                pos: ProtocolBlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                include_data: true,
            })
        );
    }

    #[test]
    fn right_mouse_press_without_block_queues_use_item() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let world = WorldStore::new();
        let mut counters = NetCounters {
            player_pose: Some(PlayerPose {
                y_rot: 45.0,
                x_rot: -20.0,
                ..PlayerPose::default()
            }),
            ..NetCounters::default()
        };

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(input.using_item);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            })
        );

        handle_mouse_input(
            &mut input,
            &world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
        );

        assert!(!input.using_item);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::ReleaseUseItem,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn world_time_and_weather_update_snapshot_and_clear_color() {
        let mut counters = NetCounters::default();

        apply_world_time_update(
            &mut counters,
            bbb_protocol::packets::PlayTime {
                game_time: 123,
                clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                    clock_id: 0,
                    total_ticks: 6000,
                    partial_tick: 0.0,
                    rate: 1.0,
                }],
            },
        );
        apply_game_event(
            &mut counters,
            bbb_protocol::packets::GameEvent {
                event_id: 7,
                param: 0.5,
            },
        );

        assert_eq!(
            counters.world_time,
            Some(bbb_control::WorldTime {
                game_time: 123,
                day_time: 6000,
                clock_updates: 1,
            })
        );
        assert!(counters.weather.raining);
        assert_eq!(counters.weather.rain_level, 0.5);
        assert_eq!(counters.world_time_packets, 1);
        assert_eq!(counters.game_event_packets, 1);

        let day = clear_color_for_day_time(6000, 0.0, 0.0);
        let night = clear_color_for_day_time(18000, 0.0, 0.0);
        let storm = clear_color_for_day_time(6000, 1.0, 1.0);
        assert!(day.b > night.b);
        assert!(storm.r < day.r);
        assert!(storm.g < day.g);
        assert!(storm.b < day.b);
    }

    #[test]
    fn camera_pose_uses_standing_eye_height() {
        let pose = camera_pose_from_player(PlayerPose {
            position: vec3(1.0, 2.0, 3.0),
            y_rot: 45.0,
            x_rot: -10.0,
            ..PlayerPose::default()
        });

        assert_eq!(pose.position, [1.0, 2.0, 3.0]);
        assert_eq!(pose.y_rot, 45.0);
        assert_eq!(pose.x_rot, -10.0);
        assert_eq!(pose.eye_height, CameraPose::STANDING_EYE_HEIGHT);
    }

    #[test]
    fn crosshair_raycast_hits_first_selectable_block() {
        let pose = PlayerPose {
            position: vec3(0.0, 0.0, 0.0),
            y_rot: 0.0,
            x_rot: 0.0,
            ..PlayerPose::default()
        };
        let hit = raycast_crosshair_block(pose, 5.0, 0.05, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
    }

    #[test]
    fn crosshair_raycast_reports_hit_face() {
        let pose = PlayerPose {
            position: vec3(0.0, 0.0, 0.0),
            y_rot: 0.0,
            x_rot: 0.0,
            ..PlayerPose::default()
        };

        let hit = raycast_crosshair_block_hit(pose, 5.0, 1.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                None
            }
        });

        assert_eq!(
            hit,
            Some(CrosshairBlockHit {
                pos: BlockPos { x: 0, y: 1, z: 3 },
                face: ProtocolDirection::North,
                cursor: [0.0, 0.62, 0.0],
                inside: false,
            })
        );
    }

    #[test]
    fn crosshair_raycast_ignores_fluid_blocks() {
        let pose = PlayerPose {
            position: vec3(0.0, 0.0, 0.0),
            y_rot: 0.0,
            x_rot: 0.0,
            ..PlayerPose::default()
        };
        let hit = raycast_crosshair_block(pose, 5.0, 0.05, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 2 }) {
                Some(bbb_world::TerrainMaterialClass::Fluid)
            } else if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
    }

    #[test]
    fn selection_outline_uses_block_bounds() {
        assert_eq!(
            selection_outline_for_block(BlockPos { x: -2, y: 63, z: 4 }),
            SelectionOutline {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 64.0, 5.0],
            }
        );
    }

    #[test]
    fn player_input_moves_forward_with_minecraft_yaw() {
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let pose = integrate_player_input_pose(
            PlayerPose {
                position: vec3(0.0, 64.0, 0.0),
                y_rot: 0.0,
                ..PlayerPose::default()
            },
            &input,
            1.0,
        );

        assert_f64_near(pose.position.x, 0.0, 0.000001);
        assert_f64_near(pose.position.y, 64.0, 0.000001);
        assert_f64_near(
            pose.position.z,
            INPUT_WALK_SPEED_BLOCKS_PER_SECOND,
            0.000001,
        );
        assert_f64_near(
            pose.delta_movement.z,
            INPUT_WALK_SPEED_BLOCKS_PER_SECOND / 20.0,
            0.000001,
        );
    }

    #[test]
    fn player_input_rotates_and_clamps_pitch() {
        let mut input = ClientInputState::new(true);
        input.mouse_delta_x = 100.0;
        input.mouse_delta_y = 1000.0;
        let pose = integrate_player_input_pose(
            PlayerPose {
                position: vec3(0.0, 64.0, 0.0),
                ..PlayerPose::default()
            },
            &input,
            0.0,
        );

        assert_eq!(pose.y_rot, 12.0);
        assert_eq!(pose.x_rot, 90.0);
    }

    #[test]
    fn advance_player_input_queues_move_commands_at_tick_interval() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let start = Instant::now();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters {
            player_pose: Some(PlayerPose {
                position: vec3(0.0, 64.0, 0.0),
                ..PlayerPose::default()
            }),
            ..NetCounters::default()
        };

        advance_player_input(&mut input, &mut counters, &commands, start);
        let first = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            NetCommand::PlayerAction(_) => panic!("expected move command"),
            NetCommand::PlayerCommand(_) => panic!("expected move command"),
            NetCommand::PlayerInput(_) => panic!("expected move command"),
            NetCommand::SetHeldSlot(_) => panic!("expected move command"),
            NetCommand::Swing(_) => panic!("expected move command"),
            NetCommand::UseItemOn(_) => panic!("expected move command"),
            NetCommand::UseItem(_) => panic!("expected move command"),
            NetCommand::PickItemFromBlock(_) => panic!("expected move command"),
            NetCommand::Disconnect => panic!("expected move command"),
        };
        assert_f64_near(first.state.position.y, 64.0, 0.000001);
        assert!(first.on_ground);
        assert_eq!(counters.player_move_commands_queued, 1);

        input.forward = true;
        advance_player_input(
            &mut input,
            &mut counters,
            &commands,
            start + Duration::from_millis(25),
        );
        assert!(rx.try_recv().is_err());

        advance_player_input(
            &mut input,
            &mut counters,
            &commands,
            start + Duration::from_millis(50),
        );
        let second = match rx.try_recv().unwrap() {
            NetCommand::MovePlayer(command) => command,
            NetCommand::PlayerAction(_) => panic!("expected move command"),
            NetCommand::PlayerCommand(_) => panic!("expected move command"),
            NetCommand::PlayerInput(_) => panic!("expected move command"),
            NetCommand::SetHeldSlot(_) => panic!("expected move command"),
            NetCommand::Swing(_) => panic!("expected move command"),
            NetCommand::UseItemOn(_) => panic!("expected move command"),
            NetCommand::UseItem(_) => panic!("expected move command"),
            NetCommand::PickItemFromBlock(_) => panic!("expected move command"),
            NetCommand::Disconnect => panic!("expected move command"),
        };
        assert!(second.state.position.z > 0.0);
        assert_eq!(counters.player_move_commands_queued, 2);
    }

    fn properties<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    fn flat_colormap(rgb: [u8; 3]) -> bbb_pack::ColorMapImage {
        bbb_pack::ColorMapImage::new(
            2,
            2,
            [rgb, rgb, rgb, rgb]
                .into_iter()
                .flat_map(|[r, g, b]| [r, g, b, 255])
                .collect(),
        )
        .unwrap()
    }

    fn coordinate_colormap() -> bbb_pack::ColorMapImage {
        let mut rgba = Vec::new();
        for y in 0u8..4 {
            for x in 0u8..4 {
                rgba.extend([x * 10, y * 20, x + y, 255]);
            }
        }
        bbb_pack::ColorMapImage::new(4, 4, rgba).unwrap()
    }

    fn block_model_box_with_face_texture(
        face: bbb_pack::BlockModelFace,
        texture: &str,
        tint_index: Option<i32>,
    ) -> bbb_pack::BlockModelBox {
        let mut face_present = [false; 6];
        let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
        let mut face_tint_indices = [None; 6];
        face_present[face.index()] = true;
        face_textures[face.index()] = Some(texture.to_string());
        face_tint_indices[face.index()] = tint_index;
        bbb_pack::BlockModelBox {
            from: [0, 0, 0],
            to: [16, 16, 16],
            face_present,
            face_uvs: [[0, 0, 16, 16]; 6],
            face_cull: [false; 6],
            face_tint_indices,
            face_textures,
        }
    }

    fn player_position_update(
        id: i32,
        position: [f64; 3],
        delta_movement: [f64; 3],
        y_rot: f32,
        x_rot: f32,
        relatives_mask: i32,
    ) -> bbb_protocol::packets::PlayerPositionUpdate {
        bbb_protocol::packets::PlayerPositionUpdate {
            id,
            position: bbb_protocol::packets::Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: bbb_protocol::packets::Vec3d {
                x: delta_movement[0],
                y: delta_movement[1],
                z: delta_movement[2],
            },
            y_rot,
            x_rot,
            relatives_mask,
        }
    }

    fn vec3(x: f64, y: f64, z: f64) -> NetVec3 {
        NetVec3 { x, y, z }
    }

    fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}"
        );
    }
}
