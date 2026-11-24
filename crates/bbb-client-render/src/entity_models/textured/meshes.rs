use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity_models) struct EntityModelSubmitSortKey {
    pub(in crate::entity_models) order: i32,
    pub(in crate::entity_models) distance_sq: f32,
    pub(in crate::entity_models) position: [f32; 3],
    pub(in crate::entity_models) insertion_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EntityModelTexturedDrawAtlas {
    Static,
    DynamicPlayerSkin,
    DynamicPlayerTexture,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityModelTexturedDrawRange {
    pub(crate) atlas: EntityModelTexturedDrawAtlas,
    pub(crate) render_type: EntityModelLayerRenderType,
    pub(crate) surface_cull: bool,
    pub(in crate::entity_models) sort_key: EntityModelSubmitSortKey,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityModelDrawRange {
    pub(crate) render_type: EntityModelLayerRenderType,
    pub(in crate::entity_models) sort_key: EntityModelSubmitSortKey,
    pub(crate) index_start: u32,
    pub(crate) index_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EntityModelTranslucentDrawRange {
    Textured(EntityModelTexturedDrawRange),
    Geometry(EntityModelDrawRange),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityModelScrollOwner {
    BreezeWind,
    EnergySwirl,
    GuardianBeam,
    EndCrystalBeam,
    BeaconBeamOpaque,
    BeaconBeamTranslucent,
}

impl EntityModelScrollOwner {
    pub(super) const fn is_opaque_beam(self) -> bool {
        matches!(
            self,
            Self::GuardianBeam | Self::EndCrystalBeam | Self::BeaconBeamOpaque
        )
    }
}

pub(in crate::entity_models) struct EntityModelTexturedMeshes {
    /// Posed model geometry duplicated through Vanilla's
    /// `SheetedDecalTextureGenerator` contract for block-entity break progress.
    pub(in crate::entity_models) crumbling: BlockDestroyOverlayMesh,
    /// Vanilla `RenderTypes.entitySolid(texture)` geometry. Unlike cutout-cull,
    /// this cull-on state preserves sampled texels below alpha `0.1`.
    pub(in crate::entity_models) solid: EntityModelTexturedMesh,
    pub(in crate::entity_models) cutout: EntityModelTexturedMesh,
    pub(in crate::entity_models) cutout_cull: EntityModelTexturedMesh,
    /// Vanilla `RenderTypes.entitySolidZOffsetForward(texture)` geometry. This owns a dedicated
    /// bucket because it uses the forward view-offset projection instead of the ordinary entity
    /// camera matrix.
    pub(in crate::entity_models) solid_z_offset_forward: EntityModelTexturedMesh,
    /// Vanilla `RenderTypes.entityCutoutDissolve(dragon.png, dragon_exploding.png)` geometry (the
    /// dying ender dragon body). Its base UVs match the plain cutout bucket, but each vertex also
    /// carries a `mask_uv` set so the GPU dissolve pipeline can sample `dragon_exploding.png` for the
    /// per-fragment erosion; kept out of the plain cutout bucket so it can own that pipeline.
    pub(in crate::entity_models) dissolve: EntityModelDissolveMesh,
    /// Vanilla `RenderTypes.entityCutoutZOffset(texture)` geometry. Its shader state matches
    /// `entityCutout`, but vanilla also applies `VIEW_OFFSET_Z_LAYERING`; keep it out of the plain
    /// cutout bucket so the GPU path can own the z-offset pipeline separately.
    pub(in crate::entity_models) cutout_z_offset: EntityModelTexturedMesh,
    /// Vanilla `armorCutoutNoCull` geometry keeps the entity lightmap and per-face lighting, but
    /// uses the armor pipeline's no-overlay shader state.
    pub(in crate::entity_models) armor_cutout: EntityModelTexturedMesh,
    /// Armor-trim geometry is separated from ordinary equipment cutout so it can draw after the
    /// armor foil pass while still sampling the shared runtime entity texture atlas.
    pub(in crate::entity_models) armor_trim: EntityModelTexturedMesh,
    pub(in crate::entity_models) armor_trim_decal: EntityModelTexturedMesh,
    pub(in crate::entity_models) translucent: EntityModelTexturedMesh,
    /// Vanilla `armorTranslucent` geometry uses the dedicated armor translucent pipeline instead
    /// of the generic `entityTranslucent` shader.
    pub(in crate::entity_models) armor_translucent: EntityModelTexturedMesh,
    pub(in crate::entity_models) translucent_emissive: EntityModelTexturedMesh,
    pub(in crate::entity_models) item_entity_translucent: EntityModelTexturedMesh,
    pub(in crate::entity_models) item_entity_translucent_cull: EntityModelTexturedMesh,
    pub(in crate::entity_models) eyes: EntityModelTexturedMesh,
    /// CPU-retained geometry for vanilla `RenderTypes.outline(...)` submissions. Submission metadata
    /// preserves the original model tint, while the folded vertices carry `outlineColor` like
    /// vanilla's `OutlineBufferSource.EntityOutlineGenerator`. This bucket is for vanilla
    /// `OUTLINE_NO_CULL`.
    pub(in crate::entity_models) outline: EntityModelTexturedMesh,
    /// CPU-retained outline copies derived from source render types whose vanilla pipeline has
    /// culling enabled. Vanilla `RenderType.outline()` forwards `state.pipeline.isCull()`.
    pub(in crate::entity_models) outline_cull: EntityModelTexturedMesh,
    /// Ready remote player skins are rendered through a dedicated atlas, preserving their vanilla
    /// cutout/translucent render type while swapping only the texture source.
    pub(in crate::entity_models) dynamic_player_skin_solid: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_cutout: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_cutout_cull: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_solid_z_offset_forward:
        EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_cutout_z_offset: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_translucent: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_item_entity_translucent:
        EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_item_entity_translucent_cull:
        EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_outline: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_skin_outline_cull: EntityModelTexturedMesh,
    /// Ready remote non-skin player profile textures, such as capes and elytra, use a separate
    /// variable-size atlas while preserving the vanilla render type.
    pub(in crate::entity_models) dynamic_player_texture_solid: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_cutout: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_cutout_cull: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_solid_z_offset_forward:
        EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_cutout_z_offset: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_armor_cutout: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_armor_trim: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_armor_trim_decal: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_translucent: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_item_entity_translucent:
        EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_item_entity_translucent_cull:
        EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_outline: EntityModelTexturedMesh,
    pub(in crate::entity_models) dynamic_player_texture_outline_cull: EntityModelTexturedMesh,
    /// Translucent scrolling overlay (vanilla `breezeWind` — the wind charge).
    pub(in crate::entity_models) scroll: EntityModelScrollMesh,
    /// Additive scrolling overlay (vanilla `energySwirl` — the charged-creeper / wither glow).
    pub(in crate::entity_models) scroll_additive: EntityModelScrollMesh,
    /// Vanilla `GuardianRenderer` custom beam geometry through exact `entityCutout` state.
    pub(in crate::entity_models) guardian_beam: EntityModelScrollMesh,
    /// Vanilla opaque `endCrystalBeam` custom geometry.
    pub(in crate::entity_models) end_crystal_beam: EntityModelScrollMesh,
    /// Vanilla opaque `beaconBeam` core used by the End Gateway.
    pub(in crate::entity_models) beacon_beam_opaque: EntityModelScrollMesh,
    /// Vanilla translucent `beaconBeam` glow used by the End Gateway.
    pub(in crate::entity_models) beacon_beam_translucent: EntityModelScrollMesh,
    /// Vanilla `entityGlint` folded through the glint texture transform.
    pub(in crate::entity_models) entity_glint: EntityModelScrollMesh,
    /// Vanilla `armorEntityGlint` folded through the armor glint texture transform.
    pub(in crate::entity_models) armor_entity_glint: EntityModelScrollMesh,
    /// Vanilla `RenderTypes.dragonRays()` position-colour death rays. The geometry is no-texture
    /// custom geometry from `EnderDragonRenderer.submitRays`.
    pub(in crate::entity_models) dragon_rays: EntityModelMesh,
    /// Vanilla `RenderTypes.dragonRaysDepth()` replays the same ray shape into a depth-only pipeline.
    pub(in crate::entity_models) dragon_rays_depth: EntityModelMesh,
    /// Vanilla `RenderTypes.leash()` rope strips. Untextured opaque position-color-lightmap custom
    /// geometry from `LeashFeatureRenderer.renderLeash`, one two-strip rope per leash state.
    pub(in crate::entity_models) leash: EntityModelMesh,
    /// Vanilla `RenderTypes.endPortal()` block-entity cube faces. The GPU shader owns the 15-layer
    /// portal sampler formula; the mesh only carries world positions and atlas sub-rect metadata.
    pub(in crate::entity_models) end_portal: EntityModelPortalMesh,
    /// Vanilla `RenderTypes.endGateway()` block-entity cube faces. The GPU shader owns the 16-layer
    /// gateway sampler formula; the gateway beam is emitted separately through the scroll bucket.
    pub(in crate::entity_models) end_gateway: EntityModelPortalMesh,
    /// Opaque custom beam ranges preserve submit identity while sharing one GPU buffer per exact
    /// render state. They draw in the main solid feature phase, never the translucent plan.
    pub(in crate::entity_models) opaque_beam_draws: Vec<EntityModelDrawRange>,
    pub(in crate::entity_models) sorted_main_translucent_draws:
        Vec<EntityModelTranslucentDrawRange>,
    pub(in crate::entity_models) sorted_item_entity_draws: Vec<EntityModelTexturedDrawRange>,
    sort_camera_position: Option<[f32; 3]>,
    pub(super) retain_parts: bool,
    /// The camera orientation at mesh-build time, consumed by camera-facing billboard renderers
    /// (`ExperienceOrbRenderer`-style `mulPose(camera.orientation)` quads). World extraction
    /// derives it from the camera pose; GUI entity PIP derives it from
    /// `GuiEntityRenderer`'s per-preview override. The GPU path rebuilds these meshes whenever
    /// the owning camera input changes, so the baked orientation stays current.
    pub(super) camera_orientation: Option<Mat4>,
    current_submission_light: [f32; 2],
    current_submission_overlay: [f32; 2],
    current_submission_outline_color: u32,
    current_block_entity_crumbling: Option<BlockEntityCrumbling>,
    pub(super) current_force_transparent: bool,
    pub(super) current_outline_only: bool,
    pub(in crate::entity_models) current_part_origin: Option<[f32; 3]>,
    pub(super) next_submission_index: usize,
}

impl EntityModelTexturedMeshes {
    pub(in crate::entity_models) fn new(sort_camera_position: Option<[f32; 3]>) -> Self {
        Self {
            crumbling: BlockDestroyOverlayMesh::new(),
            solid: EntityModelTexturedMesh::new(),
            cutout: EntityModelTexturedMesh::new(),
            cutout_cull: EntityModelTexturedMesh::new(),
            solid_z_offset_forward: EntityModelTexturedMesh::new(),
            dissolve: EntityModelDissolveMesh::new(),
            cutout_z_offset: EntityModelTexturedMesh::new(),
            armor_cutout: EntityModelTexturedMesh::new(),
            armor_trim: EntityModelTexturedMesh::new(),
            armor_trim_decal: EntityModelTexturedMesh::new(),
            translucent: EntityModelTexturedMesh::new(),
            armor_translucent: EntityModelTexturedMesh::new(),
            translucent_emissive: EntityModelTexturedMesh::new(),
            item_entity_translucent: EntityModelTexturedMesh::new(),
            item_entity_translucent_cull: EntityModelTexturedMesh::new(),
            eyes: EntityModelTexturedMesh::new(),
            outline: EntityModelTexturedMesh::new(),
            outline_cull: EntityModelTexturedMesh::new(),
            dynamic_player_skin_solid: EntityModelTexturedMesh::new(),
            dynamic_player_skin_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_skin_cutout_cull: EntityModelTexturedMesh::new(),
            dynamic_player_skin_solid_z_offset_forward: EntityModelTexturedMesh::new(),
            dynamic_player_skin_cutout_z_offset: EntityModelTexturedMesh::new(),
            dynamic_player_skin_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_skin_item_entity_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_skin_item_entity_translucent_cull: EntityModelTexturedMesh::new(),
            dynamic_player_skin_outline: EntityModelTexturedMesh::new(),
            dynamic_player_skin_outline_cull: EntityModelTexturedMesh::new(),
            dynamic_player_texture_solid: EntityModelTexturedMesh::new(),
            dynamic_player_texture_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_texture_cutout_cull: EntityModelTexturedMesh::new(),
            dynamic_player_texture_solid_z_offset_forward: EntityModelTexturedMesh::new(),
            dynamic_player_texture_cutout_z_offset: EntityModelTexturedMesh::new(),
            dynamic_player_texture_armor_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_texture_armor_trim: EntityModelTexturedMesh::new(),
            dynamic_player_texture_armor_trim_decal: EntityModelTexturedMesh::new(),
            dynamic_player_texture_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_texture_item_entity_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_texture_item_entity_translucent_cull: EntityModelTexturedMesh::new(),
            dynamic_player_texture_outline: EntityModelTexturedMesh::new(),
            dynamic_player_texture_outline_cull: EntityModelTexturedMesh::new(),
            scroll: EntityModelScrollMesh::new(),
            scroll_additive: EntityModelScrollMesh::new(),
            guardian_beam: EntityModelScrollMesh::new(),
            end_crystal_beam: EntityModelScrollMesh::new(),
            beacon_beam_opaque: EntityModelScrollMesh::new(),
            beacon_beam_translucent: EntityModelScrollMesh::new(),
            entity_glint: EntityModelScrollMesh::new(),
            armor_entity_glint: EntityModelScrollMesh::new(),
            dragon_rays: EntityModelMesh::new(),
            dragon_rays_depth: EntityModelMesh::new(),
            leash: EntityModelMesh::new(),
            end_portal: EntityModelPortalMesh::new(),
            end_gateway: EntityModelPortalMesh::new(),
            opaque_beam_draws: Vec::new(),
            sorted_main_translucent_draws: Vec::new(),
            sorted_item_entity_draws: Vec::new(),
            sort_camera_position,
            retain_parts: false,
            camera_orientation: None,
            current_submission_light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            current_submission_overlay: ENTITY_VERTEX_NO_OVERLAY,
            current_submission_outline_color: 0,
            current_block_entity_crumbling: None,
            current_force_transparent: false,
            current_outline_only: false,
            current_part_origin: None,
            next_submission_index: 0,
        }
    }

    pub(super) fn mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type {
            EntityModelLayerRenderType::EntityCutoutZOffset => return &mut self.cutout_z_offset,
            EntityModelLayerRenderType::ArmorCutoutNoCull => return &mut self.armor_cutout,
            EntityModelLayerRenderType::ArmorTrimCutoutNoCull => return &mut self.armor_trim,
            EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull => {
                return &mut self.armor_trim_decal;
            }
            EntityModelLayerRenderType::ArmorTranslucent => return &mut self.armor_translucent,
            _ => {}
        }
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Solid => &mut self.solid,
            EntityModelLayerRenderBucket::Cutout => {
                if render_type.surface_cull() {
                    &mut self.cutout_cull
                } else {
                    &mut self.cutout
                }
            }
            EntityModelLayerRenderBucket::SolidZOffsetForward => &mut self.solid_z_offset_forward,
            EntityModelLayerRenderBucket::CutoutZOffset => &mut self.cutout_z_offset,
            EntityModelLayerRenderBucket::Translucent => &mut self.translucent,
            EntityModelLayerRenderBucket::TranslucentEmissive => &mut self.translucent_emissive,
            EntityModelLayerRenderBucket::ItemEntityTranslucent => {
                if render_type.surface_cull() {
                    &mut self.item_entity_translucent_cull
                } else {
                    &mut self.item_entity_translucent
                }
            }
            EntityModelLayerRenderBucket::Eyes => &mut self.eyes,
            EntityModelLayerRenderBucket::OutlineOnly => &mut self.outline,
            EntityModelLayerRenderBucket::Dissolve
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll
            | EntityModelLayerRenderBucket::EndCrystalBeam
            | EntityModelLayerRenderBucket::BeaconBeamOpaque
            | EntityModelLayerRenderBucket::BeaconBeamTranslucent
            | EntityModelLayerRenderBucket::PositionColor
            | EntityModelLayerRenderBucket::Portal => {
                panic!("scroll/dissolve render types are not emitted into textured mesh buckets")
            }
            EntityModelLayerRenderBucket::DepthOnly | EntityModelLayerRenderBucket::GlintOnly => {
                panic!("non-color render types are not emitted into textured mesh buckets")
            }
        }
    }

    pub(super) fn dynamic_player_skin_mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Solid => &mut self.dynamic_player_skin_solid,
            EntityModelLayerRenderBucket::Cutout => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_skin_cutout_cull
                } else {
                    &mut self.dynamic_player_skin_cutout
                }
            }
            EntityModelLayerRenderBucket::CutoutZOffset => {
                &mut self.dynamic_player_skin_cutout_z_offset
            }
            EntityModelLayerRenderBucket::SolidZOffsetForward => {
                &mut self.dynamic_player_skin_solid_z_offset_forward
            }
            EntityModelLayerRenderBucket::Translucent => &mut self.dynamic_player_skin_translucent,
            EntityModelLayerRenderBucket::ItemEntityTranslucent => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_skin_item_entity_translucent_cull
                } else {
                    &mut self.dynamic_player_skin_item_entity_translucent
                }
            }
            EntityModelLayerRenderBucket::OutlineOnly => &mut self.dynamic_player_skin_outline,
            EntityModelLayerRenderBucket::Eyes
            | EntityModelLayerRenderBucket::TranslucentEmissive
            | EntityModelLayerRenderBucket::Dissolve
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll
            | EntityModelLayerRenderBucket::EndCrystalBeam
            | EntityModelLayerRenderBucket::BeaconBeamOpaque
            | EntityModelLayerRenderBucket::BeaconBeamTranslucent
            | EntityModelLayerRenderBucket::PositionColor
            | EntityModelLayerRenderBucket::Portal
            | EntityModelLayerRenderBucket::DepthOnly
            | EntityModelLayerRenderBucket::GlintOnly => {
                panic!("unsupported dynamic player skin render type")
            }
        }
    }

    pub(super) fn dynamic_player_texture_mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        if render_type == EntityModelLayerRenderType::ArmorCutoutNoCull {
            return &mut self.dynamic_player_texture_armor_cutout;
        }
        if render_type == EntityModelLayerRenderType::ArmorTrimCutoutNoCull {
            return &mut self.dynamic_player_texture_armor_trim;
        }
        if render_type == EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull {
            return &mut self.dynamic_player_texture_armor_trim_decal;
        }
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Solid => &mut self.dynamic_player_texture_solid,
            EntityModelLayerRenderBucket::Cutout => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_texture_cutout_cull
                } else {
                    &mut self.dynamic_player_texture_cutout
                }
            }
            EntityModelLayerRenderBucket::CutoutZOffset => {
                &mut self.dynamic_player_texture_cutout_z_offset
            }
            EntityModelLayerRenderBucket::SolidZOffsetForward => {
                &mut self.dynamic_player_texture_solid_z_offset_forward
            }
            EntityModelLayerRenderBucket::Translucent => {
                &mut self.dynamic_player_texture_translucent
            }
            EntityModelLayerRenderBucket::ItemEntityTranslucent => {
                if render_type.surface_cull() {
                    &mut self.dynamic_player_texture_item_entity_translucent_cull
                } else {
                    &mut self.dynamic_player_texture_item_entity_translucent
                }
            }
            EntityModelLayerRenderBucket::OutlineOnly => &mut self.dynamic_player_texture_outline,
            EntityModelLayerRenderBucket::Eyes
            | EntityModelLayerRenderBucket::TranslucentEmissive
            | EntityModelLayerRenderBucket::Dissolve
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll
            | EntityModelLayerRenderBucket::EndCrystalBeam
            | EntityModelLayerRenderBucket::BeaconBeamOpaque
            | EntityModelLayerRenderBucket::BeaconBeamTranslucent
            | EntityModelLayerRenderBucket::PositionColor
            | EntityModelLayerRenderBucket::Portal
            | EntityModelLayerRenderBucket::DepthOnly
            | EntityModelLayerRenderBucket::GlintOnly => {
                panic!("unsupported dynamic player texture render type")
            }
        }
    }

    fn set_current_submission_state(&mut self, instance: &EntityModelInstance) {
        self.current_submission_light = instance.render_state.shader_light();
        self.current_submission_overlay = instance.render_state.overlay_coords();
        self.current_submission_outline_color = instance.render_state.outline_color;
        self.current_block_entity_crumbling = instance.block_entity.block_entity_crumbling;
        self.current_part_origin = None;
        let illusioner_body_visible = instance.illusioner_body_visible_when_invisible();
        self.current_force_transparent = instance.render_state.invisible
            && !instance.render_state.invisible_to_player
            && !illusioner_body_visible;
        self.current_outline_only = instance.render_state.invisible
            && instance.render_state.invisible_to_player
            && instance.render_state.appears_glowing
            && !illusioner_body_visible;
    }

    pub(in crate::entity_models) fn current_invisible_base_only(&self) -> bool {
        self.current_force_transparent || self.current_outline_only
    }

    pub(super) fn submission(
        &self,
        render_type: EntityModelLayerRenderType,
        texture: EntityModelTextureRef,
        tint: [f32; 4],
        transform: Mat4,
        order: i32,
    ) -> EntityModelRenderSubmission {
        EntityModelRenderSubmission {
            render_type,
            texture,
            owned_texture: None,
            dissolve_texture: None,
            tint,
            transform,
            light: self.current_submission_light,
            overlay: self.current_submission_overlay,
            outline_color: self.current_submission_outline_color,
            order,
        }
    }

    pub(super) fn take_submission_index(&mut self) -> usize {
        let index = self.next_submission_index;
        self.next_submission_index += 1;
        index
    }

    pub(super) fn sorted_upload_key(
        &self,
        submission: EntityModelRenderSubmission,
        insertion_index: usize,
    ) -> Option<EntityModelSubmitSortKey> {
        if !submission.render_type.has_blending() {
            return None;
        }
        self.geometry_draw_key(submission.transform, submission.order, insertion_index)
    }

    #[inline]
    pub(super) fn geometry_draw_key(
        &self,
        transform: Mat4,
        order: i32,
        insertion_index: usize,
    ) -> Option<EntityModelSubmitSortKey> {
        let camera_position = self.sort_camera_position?;
        let position = self
            .current_part_origin
            .map_or(Vec3::ZERO, Vec3::from_array)
            + transform.transform_point3(Vec3::ZERO);
        let camera = Vec3::from_array(camera_position);
        let distance_sq = (position - camera).length_squared();
        Some(EntityModelSubmitSortKey {
            order,
            position: position.to_array(),
            distance_sq: if distance_sq.is_finite() {
                distance_sq
            } else {
                0.0
            },
            insertion_index,
        })
    }

    pub(super) fn push_geometry_draw_range(
        &mut self,
        render_type: EntityModelLayerRenderType,
        sort_key: EntityModelSubmitSortKey,
        index_start: u32,
        index_count: u32,
    ) {
        if index_count == 0 {
            return;
        }
        self.sorted_main_translucent_draws
            .push(EntityModelTranslucentDrawRange::Geometry(
                EntityModelDrawRange {
                    render_type,
                    sort_key,
                    index_start,
                    index_count,
                },
            ));
    }

    fn push_textured_draw_range(
        &mut self,
        atlas: EntityModelTexturedDrawAtlas,
        render_type: EntityModelLayerRenderType,
        sort_key: EntityModelSubmitSortKey,
        index_start: usize,
        index_count: usize,
    ) {
        if index_count == 0 {
            return;
        }
        let draw = EntityModelTexturedDrawRange {
            atlas,
            render_type,
            surface_cull: render_type.surface_cull(),
            sort_key,
            index_start: u32::try_from(index_start).expect("textured index start fits in u32"),
            index_count: u32::try_from(index_count).expect("textured index count fits in u32"),
        };
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Translucent
            | EntityModelLayerRenderBucket::TranslucentEmissive
            | EntityModelLayerRenderBucket::Eyes => self
                .sorted_main_translucent_draws
                .push(EntityModelTranslucentDrawRange::Textured(draw)),
            EntityModelLayerRenderBucket::ItemEntityTranslucent => {
                self.sorted_item_entity_draws.push(draw)
            }
            _ => {}
        }
    }

    pub(super) fn sort_draw_ranges(&mut self) {
        self.sorted_item_entity_draws
            .sort_by(|left, right| compare_submit_sort_key(left.sort_key, right.sort_key));
        self.opaque_beam_draws
            .sort_by(|left, right| compare_submit_sort_key(left.sort_key, right.sort_key));
        self.sorted_main_translucent_draws.sort_by(|left, right| {
            compare_submit_sort_key(
                translucent_draw_sort_key(*left),
                translucent_draw_sort_key(*right),
            )
        });
    }
}

pub(super) fn translucent_draw_sort_key(
    draw: EntityModelTranslucentDrawRange,
) -> EntityModelSubmitSortKey {
    match draw {
        EntityModelTranslucentDrawRange::Textured(draw) => draw.sort_key,
        EntityModelTranslucentDrawRange::Geometry(draw) => draw.sort_key,
    }
}

pub(super) fn compare_submit_sort_key(
    left: EntityModelSubmitSortKey,
    right: EntityModelSubmitSortKey,
) -> Ordering {
    left.order
        .cmp(&right.order)
        .then_with(|| {
            right
                .distance_sq
                .partial_cmp(&left.distance_sq)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.insertion_index.cmp(&right.insertion_index))
}

impl EntityModelRenderSubmission {
    pub(super) fn with_owned_texture(mut self, index: u32) -> Self {
        self.owned_texture = Some(index);
        self
    }

    fn with_dissolve_texture(mut self, texture: EntityModelTextureRef) -> Self {
        self.dissolve_texture = Some(texture);
        self
    }

    pub(super) fn with_overlay(mut self, overlay: [f32; 2]) -> Self {
        self.overlay = overlay;
        self
    }

    pub(super) fn with_light(mut self, light: [f32; 2]) -> Self {
        self.light = light;
        self
    }
}

#[cfg(test)]
pub(in crate::entity_models) fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures(instances, atlas, None, None)
}

#[cfg(test)]
pub(in crate::entity_models) fn entity_model_textured_meshes_with_dynamic_skins(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures(
        instances,
        atlas,
        dynamic_player_skin_atlas,
        None,
    )
}

#[cfg(test)]
pub(in crate::entity_models) fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures_for_camera(
        instances,
        atlas,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        None,
        None,
    )
}

#[cfg(test)]
pub(in crate::entity_models) fn entity_model_textured_meshes_with_dynamic_textures_for_camera(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    sort_camera_position: Option<[f32; 3]>,
    camera_pose: Option<crate::CameraPose>,
) -> EntityModelTexturedMeshes {
    let equipment_assets = ResolvedEntityEquipmentAssetCatalog::default();
    entity_model_textured_meshes_with_equipment_for_camera(
        instances,
        atlas,
        &[],
        &equipment_assets,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        sort_camera_position,
        camera_pose,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::entity_models) fn entity_model_textured_meshes_with_equipment_for_camera_orientation(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    sort_camera_position: Option<[f32; 3]>,
    camera_orientation: Option<Mat4>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(sort_camera_position);
    meshes.camera_orientation = camera_orientation;
    let mut sink = TexturedSink {
        meshes: &mut meshes,
        atlas,
        owned_entries,
        equipment_assets,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        data_driven_rigs: &[],
    };
    for instance in instances {
        sink.emit_instance(instance);
    }
    meshes.sort_draw_ranges();
    meshes
}

impl TexturedSink<'_> {
    pub(super) fn emit_instance(&mut self, instance: &EntityModelInstance) {
        // Vanilla submits leash ropes before the model and its invisibility gates.
        render_leash_ropes(self.meshes, instance);
        self.meshes.set_current_submission_state(instance);
        if instance.render_state.invisible
            && instance.render_state.invisible_to_player
            && !instance.render_state.appears_glowing
            && !instance.illusioner_body_visible_when_invisible()
        {
            dispatch_invisible_living_ungated_layers(instance, self, false);
            return;
        }
        dispatch_uniform_entity_model(instance, self);
        if self.meshes.current_force_transparent || self.meshes.current_outline_only {
            dispatch_invisible_living_ungated_layers(instance, self, true);
        } else {
            dispatch_vanilla_entity_layers(instance, self);
        }
    }
}

pub(super) fn render_textured_pass_with_dynamic_player_skin(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    dynamic_player_skin: EntityDynamicPlayerSkin,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let submit = textured_layer_submission(meshes, pass, transform);
    if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
        if let Some(entry) =
            dynamic_player_skin_atlas_entry(dynamic_player_skin_atlas, dynamic_player_skin.handle)
        {
            render_textured_dynamic_player_skin_submission(meshes, submit, entry, |mesh, entry| {
                model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
            });
            return;
        }
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(super) fn render_textured_no_overlay_layer_pass(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let submit = no_overlay_layer_submission(meshes, pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(super) fn render_textured_no_overlay_layer_pass_with_dynamic_player_skin(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    dynamic_player_skin: EntityDynamicPlayerSkin,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let submit = no_overlay_layer_submission(meshes, pass, transform);
    if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
        if let Some(entry) =
            dynamic_player_skin_atlas_entry(dynamic_player_skin_atlas, dynamic_player_skin.handle)
        {
            render_textured_dynamic_player_skin_submission(meshes, submit, entry, |mesh, entry| {
                model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
            });
            return;
        }
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn render_textured_pass_with_dynamic_player_texture(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    dynamic_player_texture: EntityDynamicPlayerTexture,
    tint: [f32; 4],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let submit = meshes.submission(render_type, texture, tint, transform, 0);
    if let Some(entry) =
        dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, dynamic_player_texture)
    {
        render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
            model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
        });
        return;
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(super) fn render_textured_dynamic_player_skin_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submission: EntityModelRenderSubmission,
    entry: EntityDynamicPlayerSkinAtlasEntry,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityDynamicPlayerSkinAtlasEntry),
) {
    let insertion_index = meshes.take_submission_index();
    let current_crumbling = meshes.current_block_entity_crumbling;
    let current_part_origin = meshes.current_part_origin;
    let sort_key = meshes.sorted_upload_key(submission, insertion_index);
    let (outline_copy, crumbling_copy, index_start, index_count) = {
        let retain =
            meshes.retain_parts && current_crumbling.is_none() && submission.outline_color == 0;
        let mesh = meshes.dynamic_player_skin_mesh_mut(submission.render_type);
        mesh.retain_parts = retain;
        mesh.part_origin = current_part_origin;
        let vertex_start = mesh.vertices.len();
        mesh.submission_part_start = mesh.parts.len();
        let index_start = mesh.indices.len();
        let face_start = mesh.cutout_faces;
        emit(mesh, entry);
        mesh.part_origin = None;
        fill_standalone_texture_sampling_vertices(
            mesh,
            vertex_start,
            entry.uv,
            entry.pixel_origin,
            entry.size,
            entry.standalone_sampling,
        );
        fill_textured_submission_vertices(mesh, vertex_start, submission);
        let outline_copy = if submission_writes_outline_buffer(submission) {
            Some(outline_copy_from_submission(
                mesh,
                vertex_start,
                index_start,
                face_start,
                submission.outline_color,
            ))
        } else {
            None
        };
        let crumbling_copy = submission_crumbling_copy(
            mesh,
            vertex_start,
            index_start,
            submission,
            current_crumbling,
        );
        (
            outline_copy,
            crumbling_copy,
            index_start,
            mesh.indices.len() - index_start,
        )
    };
    if let Some((vertices, indices, cutout_faces)) = outline_copy {
        let outline = if submission.render_type.outline_cull() {
            &mut meshes.dynamic_player_skin_outline_cull
        } else {
            &mut meshes.dynamic_player_skin_outline
        };
        append_textured_outline_copy(outline, vertices, indices, cutout_faces);
    }
    if let Some(copy) = crumbling_copy {
        append_crumbling_copy(&mut meshes.crumbling, copy);
    }
    if let Some(sort_key) = sort_key {
        meshes.push_textured_draw_range(
            EntityModelTexturedDrawAtlas::DynamicPlayerSkin,
            submission.render_type,
            sort_key,
            index_start,
            index_count,
        );
    }
}

pub(super) fn render_textured_dynamic_player_texture_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submission: EntityModelRenderSubmission,
    entry: EntityDynamicPlayerTextureAtlasEntry,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityDynamicPlayerTextureAtlasEntry),
) {
    let insertion_index = meshes.take_submission_index();
    let current_crumbling = meshes.current_block_entity_crumbling;
    let current_part_origin = meshes.current_part_origin;
    let sort_key = meshes.sorted_upload_key(submission, insertion_index);
    let (outline_copy, crumbling_copy, index_start, index_count) = {
        let retain =
            meshes.retain_parts && current_crumbling.is_none() && submission.outline_color == 0;
        let mesh = meshes.dynamic_player_texture_mesh_mut(submission.render_type);
        mesh.retain_parts = retain;
        mesh.part_origin = current_part_origin;
        let vertex_start = mesh.vertices.len();
        mesh.submission_part_start = mesh.parts.len();
        let index_start = mesh.indices.len();
        let face_start = mesh.cutout_faces;
        emit(mesh, entry);
        mesh.part_origin = None;
        fill_standalone_texture_sampling_vertices(
            mesh,
            vertex_start,
            entry.uv,
            entry.pixel_origin,
            entry.size,
            entry.standalone_sampling,
        );
        fill_textured_submission_vertices(mesh, vertex_start, submission);
        let outline_copy = if submission_writes_outline_buffer(submission) {
            Some(outline_copy_from_submission(
                mesh,
                vertex_start,
                index_start,
                face_start,
                submission.outline_color,
            ))
        } else {
            None
        };
        let crumbling_copy = submission_crumbling_copy(
            mesh,
            vertex_start,
            index_start,
            submission,
            current_crumbling,
        );
        (
            outline_copy,
            crumbling_copy,
            index_start,
            mesh.indices.len() - index_start,
        )
    };
    if let Some((vertices, indices, cutout_faces)) = outline_copy {
        let outline = if submission.render_type.outline_cull() {
            &mut meshes.dynamic_player_texture_outline_cull
        } else {
            &mut meshes.dynamic_player_texture_outline
        };
        append_textured_outline_copy(outline, vertices, indices, cutout_faces);
    }
    if let Some(copy) = crumbling_copy {
        append_crumbling_copy(&mut meshes.crumbling, copy);
    }
    if let Some(sort_key) = sort_key {
        meshes.push_textured_draw_range(
            EntityModelTexturedDrawAtlas::DynamicPlayerTexture,
            submission.render_type,
            sort_key,
            index_start,
            index_count,
        );
    }
}

fn fill_standalone_texture_sampling_vertices(
    mesh: &mut EntityModelTexturedMesh,
    vertex_start: usize,
    uv: EntityModelUvRect,
    pixel_origin: [u32; 2],
    size: [u32; 2],
    sampling: Option<bbb_render_types::StandaloneTextureSampling>,
) {
    let Some(sampling) = sampling else {
        return;
    };
    let uv_size = [uv.max[0] - uv.min[0], uv.max[1] - uv.min[1]];
    debug_assert!(uv_size[0] > 0.0 && uv_size[1] > 0.0);
    let size_flags = [
        size[0] as f32,
        size[1] as f32,
        if sampling.blur { 1.0 } else { 0.0 },
        if sampling.clamp { 1.0 } else { 0.0 },
    ];
    let pixel_origin = [pixel_origin[0] as f32, pixel_origin[1] as f32];
    for part in &mut mesh.parts[mesh.submission_part_start..] {
        part.pixel_origin = pixel_origin;
        part.size_flags = size_flags;
    }
    for vertex in &mut mesh.vertices[vertex_start..] {
        vertex.standalone_local_uv = [
            (vertex.uv[0] - uv.min[0]) / uv_size[0],
            (vertex.uv[1] - uv.min[1]) / uv_size[1],
        ];
        vertex.standalone_pixel_origin = pixel_origin;
        vertex.standalone_size_flags = size_flags;
    }
}

pub(super) fn render_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelRenderSubmission,
    atlas: &EntityModelTextureAtlasLayout,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    render_textured_submission_with_owned(meshes, submit, atlas, &[], emit);
}

pub(super) fn render_textured_submission_with_owned(
    meshes: &mut EntityModelTexturedMeshes,
    submission: EntityModelRenderSubmission,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    let insertion_index = meshes.take_submission_index();
    let current_crumbling = meshes.current_block_entity_crumbling;
    let current_part_origin = meshes.current_part_origin;
    if submission
        .dissolve_texture
        .is_some_and(|texture| entity_model_texture_atlas_entry(atlas, texture).is_none())
    {
        return;
    }
    let entry = if let Some(index) = submission.owned_texture {
        owned_entries
            .get(index as usize)
            .map(|entry| EntityModelTextureAtlasEntry {
                texture: submission.texture,
                uv: entry.uv,
            })
    } else {
        entity_model_texture_atlas_entry(atlas, submission.texture)
    };
    if let Some(entry) = entry {
        if submission.render_type.mesh_bucket() == EntityModelLayerRenderBucket::GlintOnly {
            let mut scratch = EntityModelTexturedMesh::new();
            scratch.part_origin = current_part_origin;
            emit(&mut scratch, entry);
            scratch.part_origin = None;
            fill_textured_submission_vertices(&mut scratch, 0, submission);
            append_scrolled_textured_mesh(
                glint_mesh_mut(meshes, submission.render_type),
                &scratch,
                entry.uv,
                [0.0, 0.0],
            );
            return;
        }
        if submission.render_type.mesh_bucket() == EntityModelLayerRenderBucket::Dissolve {
            // The mask atlas entry is guaranteed present here: a dissolve submission whose
            // `dissolve_texture` is missing from the atlas is suppressed by the early return above.
            let mask_uv_rect = submission
                .dissolve_texture
                .and_then(|texture| entity_model_texture_atlas_entry(atlas, texture))
                .map(|mask_entry| mask_entry.uv)
                .expect("dissolve submission without a mask atlas entry is suppressed earlier");
            let mut scratch = EntityModelTexturedMesh::new();
            scratch.part_origin = current_part_origin;
            emit(&mut scratch, entry);
            scratch.part_origin = None;
            fill_textured_submission_vertices(&mut scratch, 0, submission);
            if let Some(copy) =
                submission_crumbling_copy(&scratch, 0, 0, submission, current_crumbling)
            {
                append_crumbling_copy(&mut meshes.crumbling, copy);
            }
            append_dissolve_textured_mesh(&mut meshes.dissolve, &scratch, entry.uv, mask_uv_rect);
            return;
        }
        let sort_key = meshes.sorted_upload_key(submission, insertion_index);
        let (outline_copy, crumbling_copy, index_start, index_count) = {
            let retain =
                meshes.retain_parts && current_crumbling.is_none() && submission.outline_color == 0;
            let mesh = meshes.mesh_mut(submission.render_type);
            mesh.retain_parts = retain;
            mesh.part_origin = current_part_origin;
            let vertex_start = mesh.vertices.len();
            mesh.submission_part_start = mesh.parts.len();
            let index_start = mesh.indices.len();
            let face_start = mesh.cutout_faces;
            emit(mesh, entry);
            mesh.part_origin = None;
            fill_textured_submission_vertices(mesh, vertex_start, submission);
            let outline_copy = if submission_writes_outline_buffer(submission) {
                Some(outline_copy_from_submission(
                    mesh,
                    vertex_start,
                    index_start,
                    face_start,
                    submission.outline_color,
                ))
            } else {
                None
            };
            let crumbling_copy = submission_crumbling_copy(
                mesh,
                vertex_start,
                index_start,
                submission,
                current_crumbling,
            );
            (
                outline_copy,
                crumbling_copy,
                index_start,
                mesh.indices.len() - index_start,
            )
        };
        if let Some((vertices, indices, cutout_faces)) = outline_copy {
            let outline = if submission.render_type.outline_cull() {
                &mut meshes.outline_cull
            } else {
                &mut meshes.outline
            };
            append_textured_outline_copy(outline, vertices, indices, cutout_faces);
        }
        if let Some(copy) = crumbling_copy {
            append_crumbling_copy(&mut meshes.crumbling, copy);
        }
        if let Some(sort_key) = sort_key {
            meshes.push_textured_draw_range(
                EntityModelTexturedDrawAtlas::Static,
                submission.render_type,
                sort_key,
                index_start,
                index_count,
            );
        }
    }
}

pub(super) fn append_portal_mesh(
    dest: &mut EntityModelPortalMesh,
    source: EntityModelPortalMesh,
) -> u32 {
    let index_start = u32::try_from(dest.indices.len()).expect("portal index count fits in u32");
    let base = u32::try_from(dest.vertices.len()).expect("portal vertex count fits in u32");
    dest.vertices.extend(source.vertices);
    dest.indices
        .extend(source.indices.into_iter().map(|index| index + base));
    index_start
}

pub(super) fn append_entity_model_mesh(dest: &mut EntityModelMesh, source: EntityModelMesh) -> u32 {
    let index_start = u32::try_from(dest.indices.len()).expect("entity index count fits in u32");
    let base = u32::try_from(dest.vertices.len()).expect("entity vertex count fits in u32");
    dest.vertices.extend(source.vertices);
    dest.indices
        .extend(source.indices.into_iter().map(|index| index + base));
    dest.opaque_faces += source.opaque_faces;
    index_start
}

fn submission_writes_outline_buffer(submission: EntityModelRenderSubmission) -> bool {
    submission.outline_color != 0 && submission.render_type.affects_outline()
}

fn outline_copy_from_submission(
    mesh: &EntityModelTexturedMesh,
    vertex_start: usize,
    index_start: usize,
    face_start: usize,
    outline_color: u32,
) -> (Vec<EntityModelTexturedVertex>, Vec<u32>, usize) {
    let outline_tint = argb_to_tint(outline_color);
    let vertices = mesh.vertices[vertex_start..]
        .iter()
        .map(|vertex| {
            let mut vertex = *vertex;
            vertex.tint = outline_tint;
            vertex
        })
        .collect();
    let base = u32::try_from(vertex_start + mesh.deferred_vertices)
        .expect("textured vertex start fits in u32");
    let indices = mesh.indices[index_start..]
        .iter()
        .map(|index| {
            debug_assert!(*index >= base);
            index - base
        })
        .collect();
    (vertices, indices, mesh.cutout_faces - face_start)
}

fn append_textured_outline_copy(
    outline: &mut EntityModelTexturedMesh,
    vertices: Vec<EntityModelTexturedVertex>,
    indices: Vec<u32>,
    cutout_faces: usize,
) {
    let base = u32::try_from(outline.vertex_count()).expect("outline vertex count fits in u32");
    outline.vertices.extend(vertices);
    outline
        .indices
        .extend(indices.into_iter().map(|index| index + base));
    outline.cutout_faces += cutout_faces;
}

fn crumbling_copy_from_submission(
    mesh: &EntityModelTexturedMesh,
    vertex_start: usize,
    index_start: usize,
    crumbling: BlockEntityCrumbling,
) -> BlockDestroyOverlayMesh {
    let uv_rect_size = [
        crumbling.uv.max[0] - crumbling.uv.min[0],
        crumbling.uv.max[1] - crumbling.uv.min[1],
    ];
    let vertices = mesh.vertices[vertex_start..]
        .iter()
        .map(|vertex| {
            let local_position = [
                vertex.position[0] - crumbling.origin[0],
                vertex.position[1] - crumbling.origin[1],
                vertex.position[2] - crumbling.origin[2],
            ];
            BlockDestroyVertex {
                position: vertex.position,
                normal: vertex.normal,
                decal_uv: sheeted_decal_uv(local_position, vertex.normal, 1.0),
                uv_rect_min: crumbling.uv.min,
                uv_rect_size,
            }
        })
        .collect();
    let source_base = u32::try_from(vertex_start + mesh.deferred_vertices)
        .expect("textured vertex start fits in u32");
    let indices = mesh.indices[index_start..]
        .iter()
        .map(|index| {
            debug_assert!(*index >= source_base);
            index - source_base
        })
        .collect();
    BlockDestroyOverlayMesh { vertices, indices }
}

fn append_crumbling_copy(
    destination: &mut BlockDestroyOverlayMesh,
    source: BlockDestroyOverlayMesh,
) {
    let base = u32::try_from(destination.vertices.len())
        .expect("block-entity crumbling vertex count fits in u32");
    destination.vertices.extend(source.vertices);
    destination
        .indices
        .extend(source.indices.into_iter().map(|index| index + base));
}

fn submission_crumbling_copy(
    mesh: &EntityModelTexturedMesh,
    vertex_start: usize,
    index_start: usize,
    submission: EntityModelRenderSubmission,
    current: Option<BlockEntityCrumbling>,
) -> Option<BlockDestroyOverlayMesh> {
    submission
        .render_type
        .affects_crumbling()
        .then_some(current)
        .flatten()
        .map(|crumbling| crumbling_copy_from_submission(mesh, vertex_start, index_start, crumbling))
}

pub(super) fn fill_textured_submission_vertices(
    mesh: &mut EntityModelTexturedMesh,
    start: usize,
    submission: EntityModelRenderSubmission,
) {
    for part in &mut mesh.parts[mesh.submission_part_start..] {
        part.light = submission.light;
        part.overlay = submission.overlay;
    }
    fill_entity_textured_light(mesh, start, submission.light);
    fill_entity_textured_overlay(mesh, start, submission.overlay);
    if submission.render_type == EntityModelLayerRenderType::Outline {
        let outline_tint = argb_to_tint(submission.outline_color);
        for vertex in &mut mesh.vertices[start..] {
            vertex.tint = outline_tint;
        }
    }
}

pub(super) fn scroll_owner_for_render_type(
    render_type: EntityModelLayerRenderType,
) -> EntityModelScrollOwner {
    match render_type {
        EntityModelLayerRenderType::BreezeWind => EntityModelScrollOwner::BreezeWind,
        EntityModelLayerRenderType::EnergySwirl => EntityModelScrollOwner::EnergySwirl,
        EntityModelLayerRenderType::EndCrystalBeam => EntityModelScrollOwner::EndCrystalBeam,
        EntityModelLayerRenderType::BeaconBeamOpaque => EntityModelScrollOwner::BeaconBeamOpaque,
        EntityModelLayerRenderType::BeaconBeamTranslucent => {
            EntityModelScrollOwner::BeaconBeamTranslucent
        }
        _ => panic!("only dedicated scroll render types are emitted through the scroll mesh"),
    }
}

pub(super) fn scroll_owner_mesh_mut(
    meshes: &mut EntityModelTexturedMeshes,
    owner: EntityModelScrollOwner,
) -> &mut EntityModelScrollMesh {
    match owner {
        EntityModelScrollOwner::BreezeWind => &mut meshes.scroll,
        EntityModelScrollOwner::EnergySwirl => &mut meshes.scroll_additive,
        EntityModelScrollOwner::GuardianBeam => &mut meshes.guardian_beam,
        EntityModelScrollOwner::EndCrystalBeam => &mut meshes.end_crystal_beam,
        EntityModelScrollOwner::BeaconBeamOpaque => &mut meshes.beacon_beam_opaque,
        EntityModelScrollOwner::BeaconBeamTranslucent => &mut meshes.beacon_beam_translucent,
    }
}

fn glint_mesh_mut(
    meshes: &mut EntityModelTexturedMeshes,
    render_type: EntityModelLayerRenderType,
) -> &mut EntityModelScrollMesh {
    match render_type {
        EntityModelLayerRenderType::EntityGlint => &mut meshes.entity_glint,
        EntityModelLayerRenderType::ArmorEntityGlint => &mut meshes.armor_entity_glint,
        _ => panic!("only glint render types are emitted through glint meshes"),
    }
}

fn render_textured_root_pass(
    meshes: &mut EntityModelTexturedMeshes,
    root: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if layer_pass_hidden_by_invisible(meshes, pass) {
        return;
    }
    let submit = textured_layer_submission(meshes, pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        root.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(super) fn no_overlay_submission(
    meshes: &EntityModelTexturedMeshes,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    tint: [f32; 4],
    transform: Mat4,
    order: i32,
) -> EntityModelRenderSubmission {
    meshes
        .submission(render_type, texture, tint, transform, order)
        .with_overlay(ENTITY_VERTEX_NO_OVERLAY)
}

pub(super) fn no_overlay_layer_submission(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
    transform: Mat4,
) -> EntityModelRenderSubmission {
    no_overlay_layer_submission_with_tint(meshes, pass, pass.tint, transform)
}

pub(super) fn no_overlay_layer_submission_with_tint(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
    tint: [f32; 4],
    transform: Mat4,
) -> EntityModelRenderSubmission {
    no_overlay_submission(
        meshes,
        pass.render_type,
        pass.texture,
        tint,
        transform,
        pass.order,
    )
}

pub(super) fn textured_layer_submission(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
    transform: Mat4,
) -> EntityModelRenderSubmission {
    let (render_type, tint) = if layer_pass_uses_outline_render_type(meshes, pass) {
        (
            EntityModelLayerRenderType::Outline,
            outline_layer_tint(pass),
        )
    } else if meshes.current_force_transparent && pass.is_base {
        let mut tint = pass.tint;
        tint[3] *= 38.0 / 255.0;
        let render_type = if pass.kind == EntityModelLayerKind::ArmorStandBase
            && pass.render_type == EntityModelLayerRenderType::EntityTranslucent
        {
            // Vanilla `ArmorStandRenderer.getRenderType`: marker armor stands use
            // `entityTranslucent(texture, false)` for the force-transparent branch,
            // instead of the generic living `entityTranslucentCullItemTarget`.
            EntityModelLayerRenderType::EntityTranslucent
        } else {
            EntityModelLayerRenderType::EntityTranslucentCullItemTarget
        };
        (render_type, tint)
    } else {
        (pass.render_type, pass.tint)
    };
    let submit = meshes.submission(render_type, pass.texture, tint, transform, pass.order);
    if pass.render_type == EntityModelLayerRenderType::EntityCutoutDissolve {
        submit
            .with_overlay(ENTITY_VERTEX_NO_OVERLAY)
            .with_dissolve_texture(ENDER_DRAGON_EXPLODING_TEXTURE_REF)
    } else {
        match pass.kind {
            EntityModelLayerKind::NoOverlay => submit.with_overlay(ENTITY_VERTEX_NO_OVERLAY),
            EntityModelLayerKind::ZeroWhiteOverlay
            | EntityModelLayerKind::SkeletonClothing
            | EntityModelLayerKind::SheepWool
            | EntityModelLayerKind::SlimeOuter => {
                submit.with_overlay([0.0, meshes.current_submission_overlay[1]])
            }
            _ => submit,
        }
    }
}

pub(super) fn layer_pass_hidden_by_invisible(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
) -> bool {
    if pass.visible_when_invisible {
        return false;
    }
    if meshes.current_force_transparent {
        return !pass.is_base;
    }
    if meshes.current_outline_only {
        return !pass.is_base && !layer_pass_uses_invisible_outline(pass);
    }
    false
}

fn layer_pass_uses_outline_render_type(
    meshes: &EntityModelTexturedMeshes,
    pass: EntityModelLayerPass,
) -> bool {
    meshes.current_outline_only && (pass.is_base || layer_pass_uses_invisible_outline(pass))
}

fn layer_pass_uses_invisible_outline(pass: EntityModelLayerPass) -> bool {
    matches!(
        pass.kind,
        EntityModelLayerKind::SheepWool | EntityModelLayerKind::SlimeOuter
    )
}

fn outline_layer_tint(pass: EntityModelLayerPass) -> [f32; 4] {
    match pass.kind {
        // Vanilla `SheepWoolLayer` passes ARGB `0xFF000000` for the invisible-glowing outline model.
        EntityModelLayerKind::SheepWool => [0.0, 0.0, 0.0, 1.0],
        _ => pass.tint,
    }
}

/// Render a model's full textured layer-pass list (already prepared) into `meshes`.
pub(in crate::entity_models) fn render_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    for pass in passes {
        // Vanilla `LivingEntityEmissiveLayer` skips layers whose computed alpha is effectively zero.
        if pass.tint[3] <= 1.0e-5
            && pass.render_type != EntityModelLayerRenderType::EntityCutoutDissolve
        {
            continue;
        }
        match pass.visibility {
            // A part-subset emissive overlay (vanilla `retainExactParts`): render only its named parts.
            layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                if layer_pass_hidden_by_invisible(meshes, pass) {
                    continue;
                }
                let submit = textured_layer_submission(meshes, pass, transform);
                render_textured_submission(meshes, submit, atlas, |mesh, entry| {
                    model.render_textured_retained(
                        mesh,
                        submit.transform,
                        entry.uv,
                        submit.tint,
                        "",
                        parts,
                    );
                });
            }
            // `All` (and the player-parts case, whose subset is pre-applied to the tree) render whole.
            _ => render_textured_root_pass(meshes, model, transform, pass, atlas),
        }
    }
}

/// Renders a prepared model through the shared variable-size entity-texture atlas. The static pass
/// texture remains the explicit fallback when the handle is not present in the current atlas.
pub(in crate::entity_models) fn render_textured_layers_with_dynamic_texture(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    dynamic_texture: EntityDynamicTexture,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    for pass in passes {
        if pass.tint[3] <= 1.0e-5
            && pass.render_type != EntityModelLayerRenderType::EntityCutoutDissolve
        {
            continue;
        }
        if layer_pass_hidden_by_invisible(meshes, pass) {
            continue;
        }
        let submit = textured_layer_submission(meshes, pass, transform);
        if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, dynamic_texture) {
            render_textured_dynamic_player_texture_submission(
                meshes,
                submit,
                entry,
                |mesh, entry| match pass.visibility {
                    layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                        model.render_textured_retained(
                            mesh,
                            submit.transform,
                            entry.uv,
                            submit.tint,
                            "",
                            parts,
                        );
                    }
                    _ => model.render_textured(mesh, submit.transform, entry.uv, submit.tint),
                },
            );
            continue;
        }
        render_textured_submission(meshes, submit, atlas, |mesh, entry| match pass.visibility {
            layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                model.render_textured_retained(
                    mesh,
                    submit.transform,
                    entry.uv,
                    submit.tint,
                    "",
                    parts,
                );
            }
            _ => model.render_textured(mesh, submit.transform, entry.uv, submit.tint),
        });
    }
}

pub(super) fn dynamic_player_skin_atlas_entry(
    atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    handle: u64,
) -> Option<EntityDynamicPlayerSkinAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == handle)
}

pub(super) fn dynamic_player_texture_atlas_entry(
    atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    texture: EntityDynamicPlayerTexture,
) -> Option<EntityDynamicPlayerTextureAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == texture.handle)
}

pub(super) fn dynamic_texture_atlas_entry(
    atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    texture: EntityDynamicTexture,
) -> Option<EntityDynamicPlayerTextureAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == texture.handle)
}

pub(super) fn entity_model_texture_atlas_entry(
    atlas: &EntityModelTextureAtlasLayout,
    texture: EntityModelTextureRef,
) -> Option<EntityModelTextureAtlasEntry> {
    atlas
        .entries
        .iter()
        .copied()
        .find(|entry| entry.texture == texture)
}
