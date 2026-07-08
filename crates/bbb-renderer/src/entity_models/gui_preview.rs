//! GPU picture-in-picture drawing for GUI entity previews.
//!
//! Vanilla `PictureInPictureRenderer.prepare` renders each `GuiEntityRenderState` into a private
//! color texture (RGBA8) plus a private depth texture sized `bounds × guiScale`, cleared per
//! prepare (`clearColorAndDepthTextures(color, 0, depth, 1.0)`), under a
//! `setupOrtho(-1000, 1000, width, height, invertY)` projection and the pose chain
//! `translate(w/2, h/2, 0) · scale(s, s, -s) · translate(state.translation) ·
//! mulPose(state.rotation)`; `GuiEntityRenderer.renderToTexture` submits the entity render state
//! at the origin under `Lighting.Entry.ENTITY_IN_UI`, and `blitTexture` enqueues the color
//! texture onto the current GUI layer. bbb mirrors that shape: each sanitized
//! [`HudEntityPreview`](crate::hud::HudEntityPreview) owns a persistent
//! [`HudEntityPreviewPipTarget`] (color + depth textures recreated only on bounds resize, vanilla
//! `needsAResize`), the preview's `EntityModelInstance` is baked through the production entity
//! model mesh pipeline at the origin, drawn with the existing entity model pipelines under a
//! [`CameraUniform::hud_entity_preview_pip`] camera, and the HUD pass blits the color texture
//! through the HUD sprite pipeline in GUI submission order.

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt as _;

use crate::camera::{CameraUniform, LightingEntry};
use crate::frame_buffers::FrameDataBuffer;
use crate::gpu::create_depth_target;
use crate::hud::HudEntityPreview;
use crate::item_models::GuiItemLightingEntry;
use crate::Renderer;

use super::catalog::{
    EntityDynamicPlayerSkinAtlasLayout, EntityDynamicPlayerTextureAtlasLayout, EntityModelKind,
    EntityModelTextureAtlasLayout, SignModelAttachment, SignModelWood,
};
use super::colored::{entity_model_colored_runtime_mesh, SIGN_RENDER_SCALE};
use super::dispatch::{EntityModelSink, TexturedSink};
use super::geometry::{
    EntityModelMesh, EntityModelScrollMesh, EntityModelScrollVertex, EntityModelTexturedMesh,
    EntityModelTexturedVertex, EntityModelVertex,
};
use super::instances::EntityModelInstance;
use super::model_layers::SignModel;
use super::textured::{
    entity_model_textured_meshes_with_dynamic_textures_for_camera, sign_textured_layer_passes,
    EntityModelTexturedDrawAtlas, EntityModelTexturedMeshes,
};

pub(crate) const HUD_ENTITY_PREVIEW_PIP_PASS_LABEL: &str = "bbb-native-hud-entity-preview-pip-pass";

/// The entity model pipeline a PIP textured draw range runs through, mirroring the world path's
/// render-type → pipeline mapping (`main_world_pass` cutout family +
/// `draw_entity_translucent_features` translucent family).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudEntityPreviewPipPipeline {
    Cutout,
    CutoutCull,
    CutoutZOffset,
    ArmorCutout,
    Translucent,
    ArmorTranslucent,
    TranslucentEmissive,
    Eyes,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HudEntityPreviewPipTexturedDraw {
    pub(crate) pipeline: HudEntityPreviewPipPipeline,
    pub(crate) atlas: EntityModelTexturedDrawAtlas,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

/// A scroll-mesh glint draw range (vanilla `armorEntityGlint` / `entityGlint` — foil armor on the
/// smithing preview).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HudEntityPreviewPipScrollDraw {
    pub(crate) armor: bool,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

/// CPU geometry for one preview frame: every textured bucket the preview's entity model produces,
/// concatenated (indices rebased) into one vertex/index stream per vertex layout, with per-bucket
/// draw ranges in the world path's bucket order.
#[derive(Default)]
pub(crate) struct HudEntityPreviewPipGeometry {
    textured_vertices: Vec<EntityModelTexturedVertex>,
    pub(crate) textured_indices: Vec<u32>,
    pub(crate) textured_draws: Vec<HudEntityPreviewPipTexturedDraw>,
    scroll_vertices: Vec<EntityModelScrollVertex>,
    pub(crate) scroll_indices: Vec<u32>,
    pub(crate) scroll_draws: Vec<HudEntityPreviewPipScrollDraw>,
    colored_vertices: Vec<EntityModelVertex>,
    pub(crate) colored_indices: Vec<u32>,
}

impl HudEntityPreviewPipGeometry {
    pub(crate) fn textured_vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.textured_vertices)
    }

    pub(crate) fn scroll_vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.scroll_vertices)
    }

    pub(crate) fn colored_vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.colored_vertices)
    }

    /// The first textured face's model-space center (mean of its four vertices) — a
    /// deterministic sentinel anchor for readback tests: the face's own pixels cover its
    /// projected center regardless of which (equally textured) part wins the depth test.
    #[cfg(test)]
    pub(crate) fn first_textured_face_center(&self) -> Option<glam::Vec3> {
        let face = self.textured_vertices.get(0..4)?;
        let mut center = glam::Vec3::ZERO;
        for vertex in face {
            center += glam::Vec3::from_array(vertex.position);
        }
        Some(center / 4.0)
    }

    fn append_textured(
        &mut self,
        pipeline: HudEntityPreviewPipPipeline,
        atlas: EntityModelTexturedDrawAtlas,
        mesh: EntityModelTexturedMesh,
    ) {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return;
        }
        let vertex_base = u32::try_from(self.textured_vertices.len())
            .expect("preview textured vertex count fits in u32");
        let index_start = u32::try_from(self.textured_indices.len())
            .expect("preview textured index count fits in u32");
        let index_count =
            u32::try_from(mesh.indices.len()).expect("preview textured index count fits in u32");
        self.textured_vertices.extend_from_slice(&mesh.vertices);
        self.textured_indices
            .extend(mesh.indices.iter().map(|index| index + vertex_base));
        self.textured_draws.push(HudEntityPreviewPipTexturedDraw {
            pipeline,
            atlas,
            index_start,
            index_count,
        });
    }

    fn append_scroll(&mut self, armor: bool, mesh: EntityModelScrollMesh) {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return;
        }
        let vertex_base = u32::try_from(self.scroll_vertices.len())
            .expect("preview scroll vertex count fits in u32");
        let index_start = u32::try_from(self.scroll_indices.len())
            .expect("preview scroll index count fits in u32");
        let index_count =
            u32::try_from(mesh.indices.len()).expect("preview scroll index count fits in u32");
        self.scroll_vertices.extend_from_slice(&mesh.vertices);
        self.scroll_indices
            .extend(mesh.indices.iter().map(|index| index + vertex_base));
        self.scroll_draws.push(HudEntityPreviewPipScrollDraw {
            armor,
            index_start,
            index_count,
        });
    }

    fn set_colored(&mut self, mesh: EntityModelMesh) {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return;
        }
        self.colored_vertices = mesh.vertices;
        self.colored_indices = mesh.indices;
    }
}

/// Bakes one preview entity through the production entity model mesh pipeline (the same
/// dispatch + layer path the world scene uses) and folds the buckets into a PIP draw list. No
/// sort camera: the PIP target owns a private depth buffer, so buckets draw in vanilla
/// render-type order (cutout family, then glint, then the translucent family) and depth testing
/// resolves overlap exactly like the isolated vanilla PIP pass.
pub(crate) fn bake_hud_preview_pip_geometry(
    preview: &HudEntityPreview,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> HudEntityPreviewPipGeometry {
    if preview.lighting == GuiItemLightingEntry::ItemsFlat {
        if let EntityModelKind::Sign { wood, attachment } = preview.entity.kind {
            return bake_hud_gui_sign_preview_pip_geometry(
                &preview.entity,
                wood,
                attachment,
                atlas,
            );
        }
    }
    bake_hud_entity_preview_pip_geometry(
        &preview.entity,
        atlas,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
    )
}

pub(crate) fn bake_hud_entity_preview_pip_geometry(
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> HudEntityPreviewPipGeometry {
    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        std::slice::from_ref(instance),
        atlas,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        None,
    );
    let mut geometry = HudEntityPreviewPipGeometry::default();

    use EntityModelTexturedDrawAtlas::{DynamicPlayerSkin, DynamicPlayerTexture, Static};
    use HudEntityPreviewPipPipeline::*;
    // Cutout family, in main_world_pass order.
    geometry.append_textured(CutoutCull, Static, meshes.cutout_cull);
    geometry.append_textured(Cutout, Static, meshes.cutout);
    geometry.append_textured(CutoutZOffset, Static, meshes.cutout_z_offset);
    geometry.append_textured(ArmorCutout, Static, meshes.armor_cutout);
    geometry.append_textured(
        CutoutCull,
        DynamicPlayerSkin,
        meshes.dynamic_player_skin_cutout_cull,
    );
    geometry.append_textured(Cutout, DynamicPlayerSkin, meshes.dynamic_player_skin_cutout);
    geometry.append_textured(
        CutoutZOffset,
        DynamicPlayerSkin,
        meshes.dynamic_player_skin_cutout_z_offset,
    );
    geometry.append_textured(
        CutoutCull,
        DynamicPlayerTexture,
        meshes.dynamic_player_texture_cutout_cull,
    );
    geometry.append_textured(
        Cutout,
        DynamicPlayerTexture,
        meshes.dynamic_player_texture_cutout,
    );
    geometry.append_textured(
        CutoutZOffset,
        DynamicPlayerTexture,
        meshes.dynamic_player_texture_cutout_z_offset,
    );
    geometry.append_textured(
        ArmorCutout,
        DynamicPlayerTexture,
        meshes.dynamic_player_texture_armor_cutout,
    );
    // Glint scroll overlays (foil armor / foil hand items on the armor-stand preview).
    geometry.append_scroll(true, meshes.armor_entity_glint);
    geometry.append_scroll(false, meshes.entity_glint);
    // Translucent family, in draw_entity_translucent_features order.
    geometry.append_textured(Translucent, Static, meshes.translucent);
    geometry.append_textured(ArmorTranslucent, Static, meshes.armor_translucent);
    geometry.append_textured(TranslucentEmissive, Static, meshes.translucent_emissive);
    geometry.append_textured(
        Translucent,
        DynamicPlayerSkin,
        meshes.dynamic_player_skin_translucent,
    );
    geometry.append_textured(
        Translucent,
        DynamicPlayerTexture,
        meshes.dynamic_player_texture_translucent,
    );
    geometry.append_textured(Eyes, Static, meshes.eyes);
    // Texture-less colored runtime geometry (empty for the current preview entities, kept for
    // parity with the world path).
    geometry.set_colored(entity_model_colored_runtime_mesh(std::slice::from_ref(
        instance,
    )));
    geometry
}

fn bake_hud_gui_sign_preview_pip_geometry(
    instance: &EntityModelInstance,
    wood: SignModelWood,
    attachment: SignModelAttachment,
    atlas: &EntityModelTextureAtlasLayout,
) -> HudEntityPreviewPipGeometry {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    let scale = if attachment.is_hanging() {
        1.0
    } else {
        SIGN_RENDER_SCALE
    };
    let transform = Mat4::from_scale(Vec3::new(scale, -scale, -scale));
    let passes = sign_textured_layer_passes(wood, attachment);
    let mut sink = TexturedSink {
        meshes: &mut meshes,
        atlas,
        dynamic_player_skin_atlas: None,
        dynamic_player_texture_atlas: None,
    };
    sink.model(SignModel::new(attachment), transform, instance, &passes);

    let mut geometry = HudEntityPreviewPipGeometry::default();
    use EntityModelTexturedDrawAtlas::Static;
    use HudEntityPreviewPipPipeline::Cutout;
    geometry.append_textured(Cutout, Static, meshes.cutout);
    geometry
}

fn hud_preview_lighting_entry(lighting: GuiItemLightingEntry) -> LightingEntry {
    match lighting {
        GuiItemLightingEntry::ItemsFlat => LightingEntry::ItemsFlat,
        GuiItemLightingEntry::Items3d => LightingEntry::Items3d,
        GuiItemLightingEntry::EntityInUi => LightingEntry::EntityInUi,
    }
}

/// Persistent GPU state for one GUI entity preview slot: the vanilla-private color + depth
/// textures (recreated only when the preview bounds change), the PIP camera buffer, the HUD blit
/// bind group over the color texture, and the per-frame vertex/index streams.
pub(crate) struct HudEntityPreviewPipTarget {
    width: u32,
    height: u32,
    _color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    camera_buffer: wgpu::Buffer,
    _blit_sampler: wgpu::Sampler,
    pub(crate) blit_bind_group: wgpu::BindGroup,
    textured_vertices: FrameDataBuffer,
    textured_indices: FrameDataBuffer,
    scroll_vertices: FrameDataBuffer,
    scroll_indices: FrameDataBuffer,
    colored_vertices: FrameDataBuffer,
    colored_indices: FrameDataBuffer,
}

impl HudEntityPreviewPipTarget {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        hud_bind_group_layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-hud-entity-preview-pip-color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth_target(device, width, height);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-hud-entity-preview-pip-camera"),
            contents: bytemuck::bytes_of(&CameraUniform::identity()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // Vanilla blits the PIP texture with a NEAREST sampler
        // (`RenderSystem.getSamplerCache().getRepeat(FilterMode.NEAREST)`).
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bbb-hud-entity-preview-pip-blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bbb-hud-entity-preview-pip-blit-bind-group"),
            layout: hud_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        });
        Self {
            width,
            height,
            _color_texture: color_texture,
            color_view,
            _depth_texture: depth._texture,
            depth_view: depth.view,
            camera_buffer,
            _blit_sampler: blit_sampler,
            blit_bind_group,
            textured_vertices: FrameDataBuffer::vertex("bbb-hud-entity-preview-pip-vertices"),
            textured_indices: FrameDataBuffer::index("bbb-hud-entity-preview-pip-indices"),
            scroll_vertices: FrameDataBuffer::vertex("bbb-hud-entity-preview-pip-scroll-vertices"),
            scroll_indices: FrameDataBuffer::index("bbb-hud-entity-preview-pip-scroll-indices"),
            colored_vertices: FrameDataBuffer::vertex(
                "bbb-hud-entity-preview-pip-colored-vertices",
            ),
            colored_indices: FrameDataBuffer::index("bbb-hud-entity-preview-pip-colored-indices"),
        }
    }
}

fn atlas_camera_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-hud-entity-preview-pip-atlas-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

impl Renderer {
    /// Reuses the preview slot's persistent PIP target when its size still matches the preview
    /// bounds, otherwise creates a fresh one (vanilla `needsAResize` texture recreation).
    pub(crate) fn ensure_hud_entity_preview_pip_target(
        &self,
        existing: Option<HudEntityPreviewPipTarget>,
        width: u32,
        height: u32,
    ) -> HudEntityPreviewPipTarget {
        let width = width.max(1);
        let height = height.max(1);
        match existing {
            Some(target) if target.width == width && target.height == height => target,
            _ => HudEntityPreviewPipTarget::new(
                &self.device,
                self.config.format,
                &self.hud_bind_group_layout,
                width,
                height,
            ),
        }
    }

    /// Renders one sanitized preview into its PIP target: writes the PIP camera, bakes and
    /// uploads the entity geometry, then encodes the isolated color+depth pass (cleared to
    /// transparent color and depth 1.0 per preview, vanilla `clearColorAndDepthTextures`).
    /// Returns `(draw_calls, pipeline_switches)`.
    pub(crate) fn encode_hud_entity_preview_pip(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &mut HudEntityPreviewPipTarget,
        preview: &HudEntityPreview,
    ) -> (u64, u64) {
        let camera = if preview.lighting == GuiItemLightingEntry::EntityInUi {
            CameraUniform::hud_entity_preview_pip(
                target.width as f32,
                target.height as f32,
                preview.scale,
                preview.translation,
                preview.rotation,
            )
        } else {
            CameraUniform::hud_picture_in_picture(
                target.width as f32,
                target.height as f32,
                preview.scale,
                preview.translation,
                preview.rotation,
                hud_preview_lighting_entry(preview.lighting),
            )
        };
        self.queue
            .write_buffer(&target.camera_buffer, 0, bytemuck::bytes_of(&camera));

        let geometry = self.entity_model_texture_atlas.as_ref().map(|atlas| {
            bake_hud_preview_pip_geometry(
                preview,
                &atlas.layout,
                self.entity_dynamic_player_skin_atlas
                    .as_ref()
                    .map(|atlas| &atlas.layout),
                self.entity_dynamic_player_texture_atlas
                    .as_ref()
                    .map(|atlas| &atlas.layout),
            )
        });

        let mut has_textured = false;
        let mut has_scroll = false;
        let mut has_colored = false;
        if let Some(geometry) = &geometry {
            has_textured = target.textured_vertices.upload(
                &self.device,
                &self.queue,
                geometry.textured_vertex_bytes(),
            ) && target.textured_indices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&geometry.textured_indices),
            );
            has_scroll = target.scroll_vertices.upload(
                &self.device,
                &self.queue,
                geometry.scroll_vertex_bytes(),
            ) && target.scroll_indices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&geometry.scroll_indices),
            );
            has_colored = target.colored_vertices.upload(
                &self.device,
                &self.queue,
                geometry.colored_vertex_bytes(),
            ) && target.colored_indices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&geometry.colored_indices),
            );
        }

        let static_bind_group = self.entity_model_texture_atlas.as_ref().map(|atlas| {
            let (view, sampler) = atlas.view_and_sampler();
            atlas_camera_bind_group(
                &self.device,
                &self.terrain_bind_group_layout,
                &target.camera_buffer,
                view,
                sampler,
            )
        });
        let dynamic_skin_bind_group = self.entity_dynamic_player_skin_atlas.as_ref().map(|atlas| {
            let (view, sampler) = atlas.view_and_sampler();
            atlas_camera_bind_group(
                &self.device,
                &self.terrain_bind_group_layout,
                &target.camera_buffer,
                view,
                sampler,
            )
        });
        let dynamic_texture_bind_group =
            self.entity_dynamic_player_texture_atlas
                .as_ref()
                .map(|atlas| {
                    let (view, sampler) = atlas.view_and_sampler();
                    atlas_camera_bind_group(
                        &self.device,
                        &self.terrain_bind_group_layout,
                        &target.camera_buffer,
                        view,
                        sampler,
                    )
                });

        let mut draw_calls = 0u64;
        let mut pipeline_switches = 0u64;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(HUD_ENTITY_PREVIEW_PIP_PASS_LABEL),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            // The vanilla PIP depth texture is private and cleared to 1.0 per preview: preview
            // depth never mixes with the world depth target (`depth_isolated`).
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let Some(geometry) = &geometry else {
            return (draw_calls, pipeline_switches);
        };
        let Some(static_bind_group) = &static_bind_group else {
            return (draw_calls, pipeline_switches);
        };

        if has_colored {
            let (Some(vertex_buffer), Some(index_buffer)) = (
                target.colored_vertices.buffer(),
                target.colored_indices.buffer(),
            ) else {
                unreachable!("colored buffers uploaded above");
            };
            pass.set_pipeline(&self.entity_model_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, static_bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..geometry.colored_indices.len() as u32, 0, 0..1);
            draw_calls += 1;
        }

        if has_textured {
            let (Some(vertex_buffer), Some(index_buffer)) = (
                target.textured_vertices.buffer(),
                target.textured_indices.buffer(),
            ) else {
                unreachable!("textured buffers uploaded above");
            };
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            for draw in &geometry.textured_draws {
                let bind_group = match draw.atlas {
                    EntityModelTexturedDrawAtlas::Static => static_bind_group,
                    EntityModelTexturedDrawAtlas::DynamicPlayerSkin => {
                        match &dynamic_skin_bind_group {
                            Some(bind_group) => bind_group,
                            None => continue,
                        }
                    }
                    EntityModelTexturedDrawAtlas::DynamicPlayerTexture => {
                        match &dynamic_texture_bind_group {
                            Some(bind_group) => bind_group,
                            None => continue,
                        }
                    }
                };
                let (pipeline, uses_lightmap) = match draw.pipeline {
                    HudEntityPreviewPipPipeline::Cutout => {
                        (&self.entity_model_textured_pipeline, true)
                    }
                    HudEntityPreviewPipPipeline::CutoutCull => {
                        (&self.entity_model_textured_cull_pipeline, true)
                    }
                    HudEntityPreviewPipPipeline::CutoutZOffset => {
                        (&self.entity_model_cutout_z_offset_pipeline, true)
                    }
                    HudEntityPreviewPipPipeline::ArmorCutout => {
                        (&self.entity_model_armor_cutout_pipeline, true)
                    }
                    HudEntityPreviewPipPipeline::Translucent => {
                        (&self.entity_model_translucent_pipeline, true)
                    }
                    HudEntityPreviewPipPipeline::ArmorTranslucent => {
                        (&self.entity_model_armor_translucent_pipeline, true)
                    }
                    HudEntityPreviewPipPipeline::TranslucentEmissive => {
                        (&self.entity_model_translucent_emissive_pipeline, false)
                    }
                    HudEntityPreviewPipPipeline::Eyes => (&self.entity_model_eyes_pipeline, false),
                };
                pass.set_pipeline(pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, bind_group, &[]);
                if uses_lightmap {
                    pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                }
                pass.draw_indexed(
                    draw.index_start..draw.index_start + draw.index_count,
                    0,
                    0..1,
                );
                draw_calls += 1;
            }
        }

        if has_scroll {
            let (Some(vertex_buffer), Some(index_buffer)) = (
                target.scroll_vertices.buffer(),
                target.scroll_indices.buffer(),
            ) else {
                unreachable!("scroll buffers uploaded above");
            };
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            for draw in &geometry.scroll_draws {
                if draw.armor {
                    pass.set_pipeline(&self.entity_model_armor_entity_glint_pipeline);
                } else {
                    pass.set_pipeline(&self.entity_model_entity_glint_pipeline);
                }
                pipeline_switches += 1;
                pass.set_bind_group(0, static_bind_group, &[]);
                pass.draw_indexed(
                    draw.index_start..draw.index_start + draw.index_count,
                    0,
                    0..1,
                );
                draw_calls += 1;
            }
        }

        (draw_calls, pipeline_switches)
    }
}

#[cfg(test)]
mod tests {
    use super::super::gpu::build_entity_model_texture_atlas;
    use super::super::model_layers::armor_stand_entity_texture_refs;
    use super::super::{
        EntityArmorMaterial, EntityModelTextureImage, DEFAULT_ARMOR_STAND_MODEL_POSE,
    };
    use super::*;

    fn armor_stand_preview_instance() -> EntityModelInstance {
        EntityModelInstance::armor_stand(
            -1,
            [0.0, 0.0, 0.0],
            210.0,
            false,
            true,
            false,
            DEFAULT_ARMOR_STAND_MODEL_POSE,
        )
        .with_head_look(0.0, 25.0)
    }

    fn armor_stand_atlas() -> EntityModelTextureAtlasLayout {
        let images: Vec<EntityModelTextureImage> = armor_stand_entity_texture_refs()
            .iter()
            .map(|texture| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![0xff; len])
            })
            .collect();
        build_entity_model_texture_atlas(&images).expect("atlas").0
    }

    #[test]
    fn bake_concatenates_buckets_with_rebased_contiguous_draw_ranges() {
        let atlas = armor_stand_atlas();
        let geometry = bake_hud_entity_preview_pip_geometry(
            &armor_stand_preview_instance(),
            &atlas,
            None,
            None,
        );

        assert!(!geometry.textured_vertices.is_empty());
        assert!(!geometry.textured_draws.is_empty());
        // Draw ranges tile the concatenated index stream exactly, in bucket append order.
        let mut expected_start = 0u32;
        for draw in &geometry.textured_draws {
            assert_eq!(draw.index_start, expected_start, "contiguous draw ranges");
            assert!(draw.index_count > 0);
            expected_start += draw.index_count;
        }
        assert_eq!(expected_start as usize, geometry.textured_indices.len());
        // Rebased indices stay inside the concatenated vertex stream.
        let vertex_count = geometry.textured_vertices.len() as u32;
        assert!(geometry
            .textured_indices
            .iter()
            .all(|index| *index < vertex_count));
        // The bare armor stand is cutout-family only.
        assert!(geometry.textured_draws.iter().all(|draw| matches!(
            draw.pipeline,
            HudEntityPreviewPipPipeline::Cutout | HudEntityPreviewPipPipeline::CutoutCull
        )));
        assert!(geometry
            .textured_draws
            .iter()
            .all(|draw| draw.atlas == EntityModelTexturedDrawAtlas::Static));
        assert!(geometry.scroll_draws.is_empty());
        assert!(geometry.colored_indices.is_empty());
    }

    #[test]
    fn bake_routes_foil_armor_through_armor_cutout_and_armor_glint_buckets() {
        // The smithing result projection: iron chestplate with foil — vanilla `HumanoidArmorLayer`
        // submits `armorCutoutNoCull` plus `armorEntityGlint`. The full texture-ref atlas keeps
        // the iron equipment texture and the enchanted-glint texture resolvable.
        let images: Vec<EntityModelTextureImage> =
            super::super::model_layers::entity_model_texture_refs()
                .iter()
                .map(|texture| {
                    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                    EntityModelTextureImage::new(*texture, vec![0xff; len])
                })
                .collect();
        let atlas = build_entity_model_texture_atlas(&images).expect("atlas").0;
        let mut instance = armor_stand_preview_instance();
        instance.render_state.chest_armor = Some(EntityArmorMaterial::Iron);
        instance.render_state.chest_armor_foil = true;
        let geometry = bake_hud_entity_preview_pip_geometry(&instance, &atlas, None, None);

        assert!(geometry
            .textured_draws
            .iter()
            .any(|draw| draw.pipeline == HudEntityPreviewPipPipeline::ArmorCutout));
        assert!(geometry.scroll_draws.iter().any(|draw| draw.armor));
        // Glint indices are rebased against the scroll vertex stream.
        let scroll_vertex_count = geometry.scroll_vertices.len() as u32;
        assert!(geometry
            .scroll_indices
            .iter()
            .all(|index| *index < scroll_vertex_count));
    }

    #[test]
    fn pip_pass_owns_isolated_cleared_color_and_depth_targets() {
        // The depth_isolated contract: the PIP pass attaches only the target's private color +
        // depth views, clears both per preview (vanilla `clearColorAndDepthTextures(color, 0,
        // depth, 1.0)`), and never references the frame's shared depth target.
        let source = include_str!("gui_preview.rs");
        let pass_start = source
            .find("label: Some(HUD_ENTITY_PREVIEW_PIP_PASS_LABEL)")
            .expect("PIP pass descriptor");
        let pass_source = &source[pass_start..];
        let color_clear = pass_source
            .find("load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)")
            .expect("PIP color cleared to transparent per preview");
        let depth_view = pass_source
            .find("view: &target.depth_view")
            .expect("PIP pass attaches the target's private depth view");
        let depth_clear = pass_source
            .find("load: wgpu::LoadOp::Clear(1.0)")
            .expect("PIP depth cleared to 1.0 per preview");
        assert!(color_clear < depth_view && depth_view < depth_clear);
        // Runtime-built needle so this assertion does not match its own source text.
        let shared_depth_target = ["self", "depth"].join(".");
        assert!(
            !source.contains(&shared_depth_target),
            "the PIP pass never touches the frame's shared depth target"
        );
        assert!(
            source.contains("view: &target.color_view"),
            "the PIP pass renders into the target's private color view"
        );
    }

    #[test]
    fn pip_target_blits_through_nearest_sampler_and_recreates_only_on_resize() {
        // Vanilla blitTexture samples with FilterMode.NEAREST; prepare() recreates textures only
        // when the bounds change (`needsAResize`).
        let source = include_str!("gui_preview.rs");
        let sampler_start = source
            .find("bbb-hud-entity-preview-pip-blit-sampler")
            .expect("blit sampler");
        let sampler_source = &source[sampler_start..];
        assert!(
            sampler_source
                .find("mag_filter: wgpu::FilterMode::Nearest")
                .expect("nearest mag")
                < sampler_source
                    .find("blit_bind_group")
                    .expect("bind group follows sampler")
        );
        let ensure_start = source
            .find("fn ensure_hud_entity_preview_pip_target")
            .expect("ensure fn");
        let ensure_source = &source[ensure_start..];
        assert!(
            ensure_source
                .contains("Some(target) if target.width == width && target.height == height"),
            "matching-size targets are reused, not recreated"
        );
    }
}
