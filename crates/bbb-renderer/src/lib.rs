use anyhow::{anyhow, Result};
use bbb_control::RendererCounters;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

mod camera;
mod gpu;
mod hud;
mod render;
mod screenshot;
mod selection;

pub use camera::{CameraPose, ClearColor};
pub(crate) use camera::{CameraUniform, TerrainBounds};
use gpu::{
    create_camera_buffer, create_depth_target, create_terrain_atlas_gpu, create_terrain_bind_group,
    create_terrain_bind_group_layout, create_terrain_pipeline, create_terrain_translucent_pipeline,
    DepthTarget, TerrainAtlasGpu,
};
use hud::{create_hud_bind_group_layout, create_hud_pipeline, HudSpriteGpu};
pub use selection::SelectionOutline;
use selection::{create_selection_outline_gpu, create_selection_pipeline, SelectionOutlineGpu};

pub mod terrain;

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    clear: ClearColor,
    counters: RendererCounters,
    depth: DepthTarget,
    terrain_pipeline: wgpu::RenderPipeline,
    terrain_translucent_pipeline: wgpu::RenderPipeline,
    selection_pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,
    hud_bind_group_layout: wgpu::BindGroupLayout,
    terrain_bind_group_layout: wgpu::BindGroupLayout,
    camera_buffer: wgpu::Buffer,
    terrain_atlas: TerrainAtlasGpu,
    terrain_bind_group: wgpu::BindGroup,
    terrain_opaque: Vec<ResidentTerrainMesh>,
    terrain_cutout: Vec<ResidentTerrainMesh>,
    terrain_translucent: Vec<ResidentTerrainMesh>,
    terrain_source_sections: usize,
    terrain_bounds: Option<TerrainBounds>,
    camera_pose: Option<CameraPose>,
    selection_outline: Option<SelectionOutlineGpu>,
    hud_crosshair: Option<HudSpriteGpu>,
    hud_hotbar: Option<HudSpriteGpu>,
    hud_hotbar_selection: Option<HudSpriteGpu>,
    hud_experience_background: Option<HudSpriteGpu>,
    hud_experience_progress: Option<HudSpriteGpu>,
    hud_heart_container: Option<HudSpriteGpu>,
    hud_heart_full: Option<HudSpriteGpu>,
    hud_heart_half: Option<HudSpriteGpu>,
    hud_food_empty: Option<HudSpriteGpu>,
    hud_food_full: Option<HudSpriteGpu>,
    hud_food_half: Option<HudSpriteGpu>,
    hud_health: Option<f32>,
    hud_food: Option<i32>,
    hud_experience_progress_value: Option<f32>,
    hud_selected_slot: u8,
}

struct ResidentTerrainMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_count: usize,
    index_count: usize,
    opaque_faces: usize,
    cutout_faces: usize,
    translucent_faces: usize,
    culled_faces: usize,
    resident_bytes: u64,
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(window)?)?
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("no suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("bbb-native-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = choose_format(&caps.formats)?;
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes
                .first()
                .copied()
                .ok_or_else(|| anyhow!("surface has no present modes"))?
        };
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let depth = create_depth_target(&device, config.width, config.height);
        let terrain_bind_group_layout = create_terrain_bind_group_layout(&device);
        let hud_bind_group_layout = create_hud_bind_group_layout(&device);
        let camera_buffer = create_camera_buffer(&device);
        let terrain_atlas = create_terrain_atlas_gpu(&device, &queue, 1, 1, &[255, 255, 255, 255])?;
        let terrain_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &camera_buffer,
            &terrain_atlas,
        );
        let terrain_pipeline = create_terrain_pipeline(&device, format, &terrain_bind_group_layout);
        let terrain_translucent_pipeline =
            create_terrain_translucent_pipeline(&device, format, &terrain_bind_group_layout);
        let selection_pipeline =
            create_selection_pipeline(&device, format, &terrain_bind_group_layout);
        let hud_pipeline = create_hud_pipeline(&device, format, &hud_bind_group_layout);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            clear: ClearColor::default(),
            counters: RendererCounters {
                width: size.width,
                height: size.height,
                ..RendererCounters::default()
            },
            depth,
            terrain_pipeline,
            terrain_translucent_pipeline,
            selection_pipeline,
            hud_pipeline,
            hud_bind_group_layout,
            terrain_bind_group_layout,
            camera_buffer,
            terrain_atlas,
            terrain_bind_group,
            terrain_opaque: Vec::new(),
            terrain_cutout: Vec::new(),
            terrain_translucent: Vec::new(),
            terrain_source_sections: 0,
            terrain_bounds: None,
            camera_pose: None,
            selection_outline: None,
            hud_crosshair: None,
            hud_hotbar: None,
            hud_hotbar_selection: None,
            hud_experience_background: None,
            hud_experience_progress: None,
            hud_heart_container: None,
            hud_heart_full: None,
            hud_heart_half: None,
            hud_food_empty: None,
            hud_food_full: None,
            hud_food_half: None,
            hud_health: None,
            hud_food: None,
            hud_experience_progress_value: None,
            hud_selected_slot: 0,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth = create_depth_target(&self.device, self.config.width, self.config.height);
        self.update_camera();
        self.counters.width = size.width;
        self.counters.height = size.height;
    }

    pub fn counters(&self) -> RendererCounters {
        self.counters.clone()
    }

    pub fn set_camera_pose(&mut self, pose: Option<CameraPose>) {
        if self.camera_pose == pose {
            return;
        }
        self.camera_pose = pose;
        self.update_camera();
    }

    pub fn set_clear_color(&mut self, clear: ClearColor) {
        self.clear = clear;
    }

    pub fn set_selection_outline(&mut self, outline: Option<SelectionOutline>) {
        if self
            .selection_outline
            .as_ref()
            .map(|selection| selection.outline)
            == outline
        {
            return;
        }
        self.selection_outline =
            outline.map(|outline| create_selection_outline_gpu(&self.device, outline));
    }

    pub fn upload_terrain_meshes(&mut self, meshes: Vec<terrain::TerrainMesh>) {
        let source_sections = meshes.iter().map(|mesh| mesh.source_sections).sum();
        self.upload_terrain_mesh_layers(terrain::TerrainMeshLayers {
            opaque: meshes,
            cutout: Vec::new(),
            translucent: Vec::new(),
            source_sections,
        });
    }

    pub fn upload_terrain_mesh_layers(&mut self, layers: terrain::TerrainMeshLayers) {
        self.terrain_source_sections = layers.source_sections;
        self.counters.queued_sections = self.terrain_source_sections;
        self.counters.meshed_sections = self.terrain_source_sections;
        self.counters.upload_bytes = 0;
        self.terrain_opaque.clear();
        self.terrain_cutout.clear();
        self.terrain_translucent.clear();
        self.terrain_bounds = None;

        for mesh in layers.opaque {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_opaque.push(resident);
            }
        }
        for mesh in layers.cutout {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_cutout.push(resident);
            }
        }
        for mesh in layers.translucent {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_translucent.push(resident);
            }
        }

        self.update_camera();
        self.refresh_terrain_counters();
    }

    pub fn upload_terrain_texture_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.terrain_atlas =
            create_terrain_atlas_gpu(&self.device, &self.queue, width, height, rgba)?;
        self.terrain_bind_group = create_terrain_bind_group(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            &self.terrain_atlas,
        );
        self.counters.atlas_pages = 1;
        self.counters.atlas_reallocations += 1;
        self.counters.atlas_width = width;
        self.counters.atlas_height = height;
        Ok(())
    }

    fn surface_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.config.width.max(1), self.config.height.max(1))
    }

    fn refresh_terrain_counters(&mut self) {
        self.counters.uploaded_sections = if self.terrain_opaque.is_empty()
            && self.terrain_cutout.is_empty()
            && self.terrain_translucent.is_empty()
        {
            0
        } else {
            self.terrain_source_sections
        };
        self.counters.visible_sections = self.counters.uploaded_sections;
        self.counters.resident_bytes = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.resident_bytes)
            .sum();
        self.counters.terrain_vertices = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.vertex_count)
            .sum();
        self.counters.terrain_indices = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.index_count)
            .sum();
        self.counters.opaque_faces = self
            .terrain_opaque
            .iter()
            .map(|mesh| mesh.opaque_faces)
            .sum();
        self.counters.cutout_faces = self
            .terrain_cutout
            .iter()
            .map(|mesh| mesh.cutout_faces)
            .sum();
        self.counters.translucent_faces = self
            .terrain_translucent
            .iter()
            .map(|mesh| mesh.translucent_faces)
            .sum();
        self.counters.culled_faces = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.culled_faces)
            .sum();
    }

    fn create_resident_terrain_mesh(
        &mut self,
        mesh: terrain::TerrainMesh,
    ) -> Option<ResidentTerrainMesh> {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return None;
        }

        let bounds = TerrainBounds::from_vertices(&mesh.vertices);
        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes = bytemuck::cast_slice(&mesh.indices);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-terrain-vertex-buffer"),
                contents: vertex_bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-terrain-index-buffer"),
                contents: index_bytes,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
        let resident_bytes = (vertex_bytes.len() + index_bytes.len()) as u64;
        self.counters.upload_bytes += resident_bytes;
        if let Some(bounds) = bounds {
            if let Some(existing) = &mut self.terrain_bounds {
                existing.include_bounds(bounds);
            } else {
                self.terrain_bounds = Some(bounds);
            }
        }
        Some(ResidentTerrainMesh {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len(),
            index_count: mesh.indices.len(),
            opaque_faces: mesh.opaque_faces,
            cutout_faces: mesh.cutout_faces,
            translucent_faces: mesh.translucent_faces,
            culled_faces: mesh.culled_faces,
            resident_bytes,
        })
    }

    fn update_camera(&self) {
        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        let uniform = if let Some(pose) = self.camera_pose {
            CameraUniform::from_pose(pose, aspect)
        } else {
            self.terrain_bounds
                .map(|bounds| CameraUniform::from_bounds(bounds, aspect))
                .unwrap_or_else(CameraUniform::identity)
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }
}

fn choose_format(formats: &[wgpu::TextureFormat]) -> Result<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| {
            matches!(
                f,
                wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb
            )
        })
        .or_else(|| {
            formats.iter().copied().find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
        })
        .ok_or_else(|| anyhow!("surface does not expose an RGBA/BGRA 8-bit format"))
}
