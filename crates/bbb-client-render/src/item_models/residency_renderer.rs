use super::special_geometry::SpecialItemResidentPart;
use super::{ItemModelFoil, ItemModelQuad};
use crate::item_render::item_models::residency::*;
use crate::{
    entity_models::{bake_special_item_geometry, ResolvedSpecialItemTexture},
    EntityDynamicPlayerSkinStatus, Renderer, SpecialItemModel, SpecialItemModelBounds,
    SpecialItemTexture,
};
use anyhow::{bail, Result};
use std::{collections::BTreeSet, sync::Arc};

impl Renderer {
    pub fn item_model_resource_generation(&self) -> ItemModelResourceGeneration {
        self.pack.item_model_residency.resource_generation
    }

    pub fn item_model_asset_key(
        &self,
        identity: ItemModelAssetIdentity,
        atlas: ItemModelAtlasKind,
        geometry_class: ItemModelGeometryClass,
    ) -> ItemModelAssetKey {
        self.pack
            .item_model_residency
            .key(identity, atlas, geometry_class)
    }

    pub fn begin_item_model_frame(&mut self) {
        self.pack
            .item_model_residency
            .begin_frame(&mut self.counters);
    }

    /// Ensures one exact model-space owner. `build` is not evaluated on a hit
    /// or stale key, so the cache decision always precedes the large payload.
    pub fn ensure_item_model_resident(
        &mut self,
        key: ItemModelAssetKey,
        build: impl FnOnce() -> Vec<ItemModelQuad>,
    ) -> ItemModelResidencyOutcome {
        self.pack
            .item_model_residency
            .ensure(&self.device, &mut self.counters, key, build)
    }

    pub fn set_resident_item_model_instances(
        &mut self,
        instances: Vec<ItemModelInstanceRecord>,
    ) -> Result<()> {
        self.pack.item_model_residency.set_instances(
            &self.device,
            &self.queue,
            &mut self.counters,
            instances,
        )
    }

    /// Applies one atomic model-space asset/instance extraction. Exact keys
    /// are minted from the renderer's current resource generations, and each
    /// ordinary asset source is evaluated only after a miss. Special
    /// submissions first expand into their exact static parts and keys.
    pub fn set_item_model_residency_frame(
        &mut self,
        mut frame: ItemModelResidencyFrame,
    ) -> Result<()> {
        self.append_hud_block_item_residency(&mut frame);
        self.append_special_item_residency(&mut frame)?;
        if frame
            .instances
            .iter()
            .any(|request| request.scissor.is_some() && !request.target.is_gui())
        {
            bail!("resident item scissor is only valid for GUI draw targets");
        }
        self.begin_item_model_frame();
        if frame.assets.is_empty() && frame.instances.is_empty() {
            return self.set_resident_item_model_instances(Vec::new());
        }
        let mut small_keys = Vec::with_capacity(frame.assets.len().min(8));
        let mut large_keys = (frame.assets.len() > 8).then(BTreeSet::new);
        for request in frame.assets {
            let key =
                self.item_model_asset_key(request.identity, request.atlas, request.geometry_class);
            let inserted = if let Some(keys) = large_keys.as_mut() {
                keys.insert(key.clone())
            } else if small_keys.contains(&key) {
                false
            } else {
                small_keys.push(key.clone());
                true
            };
            if !inserted {
                continue;
            }
            match self.ensure_item_model_resident(key, || request.source.build()) {
                ItemModelResidencyOutcome::Reused | ItemModelResidencyOutcome::Created => {}
                ItemModelResidencyOutcome::Empty => {
                    bail!("item-model frame requested an empty resident geometry owner")
                }
                ItemModelResidencyOutcome::Stale => {
                    bail!("renderer minted a stale item-model asset key")
                }
                ItemModelResidencyOutcome::CapacityExceeded => {
                    bail!("item-model resident owner capacity exceeded")
                }
            }
        }
        let mut instances = frame
            .instances
            .into_iter()
            .map(|request| {
                ItemModelInstanceRecord::new(
                    self.item_model_asset_key(
                        request.identity,
                        request.atlas,
                        request.geometry_class,
                    ),
                    request.transform,
                    request.tint,
                    request.light,
                    request.overlay,
                    request.foil,
                    request.target,
                    request.phase,
                    request.submit_order,
                )
                .with_scissor(request.scissor)
            })
            .collect::<Vec<_>>();
        // Composite leaves publish a single monotonically increasing order.
        // Stable sorting keeps the renderer's expansion order for the several
        // geometry parts belonging to one special leaf.
        instances.sort_by_key(|instance| instance.submit_order);
        self.set_resident_item_model_instances(instances)
    }

    fn append_special_item_residency(&self, frame: &mut ItemModelResidencyFrame) -> Result<()> {
        for request in std::mem::take(&mut frame.special_items) {
            for part in self.special_item_parts(&request.model)?.iter() {
                frame.assets.push(ItemModelAssetRequest {
                    identity: part.identity.clone(),
                    atlas: part.atlas,
                    geometry_class: part.geometry_class,
                    source: ItemModelAssetSource::Quads(Arc::clone(&part.quads)),
                });
                let foil = if part.accepts_foil {
                    request.foil
                } else {
                    ItemModelFoil::None
                };
                frame.instances.push(ItemModelInstanceRequest {
                    identity: part.identity.clone(),
                    atlas: part.atlas,
                    geometry_class: part.geometry_class,
                    transform: request.transform,
                    tint: part.tint,
                    light: request.light,
                    overlay: request.overlay,
                    foil,
                    target: request.target,
                    phase: part.phase,
                    submit_order: request.submit_order,
                    scissor: request.scissor,
                });
                if part.accepts_foil && request.foil.has_foil() {
                    frame.instances.push(ItemModelInstanceRequest {
                        identity: part.identity.clone(),
                        atlas: part.atlas,
                        geometry_class: part.geometry_class,
                        transform: request.transform,
                        tint: part.tint,
                        light: request.light,
                        overlay: request.overlay,
                        foil: request.foil,
                        target: request.target,
                        phase: ItemModelDrawPhase::Glint,
                        submit_order: request.submit_order,
                        scissor: request.scissor,
                    });
                }
                if let Some(outline_color) = request.outline_color {
                    if !matches!(
                        part.phase,
                        ItemModelDrawPhase::EndPortal
                            | ItemModelDrawPhase::EndGateway
                            | ItemModelDrawPhase::Glint
                            | ItemModelDrawPhase::GlintTranslucent
                            | ItemModelDrawPhase::Outline
                    ) {
                        frame.instances.push(ItemModelInstanceRequest {
                            identity: part.identity.clone(),
                            atlas: part.atlas,
                            geometry_class: part.geometry_class,
                            transform: request.transform,
                            tint: [
                                ((outline_color >> 16) & 0xff) as f32 / 255.0,
                                ((outline_color >> 8) & 0xff) as f32 / 255.0,
                                (outline_color & 0xff) as f32 / 255.0,
                                ((outline_color >> 24) & 0xff) as f32 / 255.0,
                            ],
                            light: request.light,
                            overlay: request.overlay,
                            foil: ItemModelFoil::None,
                            target: ItemModelDrawTarget::EntityOutline,
                            phase: ItemModelDrawPhase::Outline,
                            submit_order: request.submit_order,
                            scissor: request.scissor,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// The baked resident parts of one special model, baked once per model
    /// value and resource/atlas generation.
    fn special_item_parts(
        &self,
        model: &SpecialItemModel,
    ) -> Result<Arc<[SpecialItemResidentPart]>> {
        let residency = &self.pack.item_model_residency;
        let generation = residency.special_geometry_generation();
        let frame = residency.frame_index;
        if let Some(parts) = residency
            .special_geometry
            .borrow_mut()
            .get(model, generation, frame)
        {
            return Ok(parts);
        }
        let model_variant = format!("{model:?}");
        let model_id = Arc::<str>::from(format!(
            "minecraft:special/{}",
            model.registered_type_name()
        ));
        let parts = bake_special_item_geometry(model, |texture| {
            self.resolve_special_item_texture(texture)
        })?
        .into_iter()
        .enumerate()
        .map(|(part_index, part)| {
            let part_index = u32::try_from(part_index)
                .map_err(|_| anyhow::anyhow!("special item has too many geometry parts"))?;
            let mut identity = ItemModelAssetIdentity::new(
                model_id.clone(),
                Arc::<str>::from(format!(
                    "{}|{}|{}",
                    model_variant, part.semantic_part, part.texture_identity
                )),
                part_index,
            );
            if matches!(
                part.phase,
                ItemModelDrawPhase::EndPortal | ItemModelDrawPhase::EndGateway
            ) {
                identity = identity.with_portal_metadata();
            }
            let geometry_class = match part.phase {
                ItemModelDrawPhase::Outline => {
                    bail!("special item geometry cannot target the entity-outline phase")
                }
                ItemModelDrawPhase::Pattern
                | ItemModelDrawPhase::Translucent
                | ItemModelDrawPhase::TranslucentNoCull => ItemModelGeometryClass::Translucent,
                ItemModelDrawPhase::Solid
                | ItemModelDrawPhase::SolidNoCutout
                | ItemModelDrawPhase::SolidZOffsetForward
                | ItemModelDrawPhase::CutoutNoCull
                | ItemModelDrawPhase::CutoutZOffset
                | ItemModelDrawPhase::Glint
                | ItemModelDrawPhase::GlintTranslucent
                | ItemModelDrawPhase::EndPortal
                | ItemModelDrawPhase::EndGateway => ItemModelGeometryClass::Solid,
            };
            Ok(SpecialItemResidentPart {
                identity,
                semantic_part: part.semantic_part,
                atlas: part.atlas,
                geometry_class,
                phase: part.phase,
                quads: part.quads.into(),
                tint: part.tint,
                accepts_foil: part.accepts_foil,
            })
        })
        .collect::<Result<Arc<[_]>>>()?;
        residency.special_geometry.borrow_mut().insert(
            model.clone(),
            generation,
            frame,
            Arc::clone(&parts),
        );
        Ok(parts)
    }

    fn resolve_special_item_texture(
        &self,
        texture: &SpecialItemTexture,
    ) -> Result<ResolvedSpecialItemTexture> {
        match texture {
            SpecialItemTexture::Static(texture) => {
                if texture.size.contains(&0) {
                    bail!(
                        "entity model texture `{}` has zero-sized logical dimensions",
                        texture.path
                    );
                }
                let atlas = self
                    .pack
                    .entity_model_texture_atlas
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("entity model texture atlas is not uploaded"))?;
                let uv = atlas.static_uv(texture).ok_or_else(|| {
                    anyhow::anyhow!(
                        "entity model texture `{}` is absent from the static atlas",
                        texture.path
                    )
                })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::Entity,
                    logical_size: texture.size,
                    uv,
                    identity: Arc::from(format!("static:{}", texture.path)),
                })
            }
            SpecialItemTexture::Pack(texture) => {
                let atlas = self
                    .pack
                    .entity_model_texture_atlas
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("entity model texture atlas is not uploaded"))?;
                let uv = atlas.pack_uv(texture).ok_or_else(|| {
                    anyhow::anyhow!(
                        "entity model texture `{}` is absent from the pack-owned atlas",
                        texture.path
                    )
                })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::Entity,
                    logical_size: texture.size,
                    uv,
                    identity: Arc::from(format!("pack:{}:{:?}", texture.path, texture.size)),
                })
            }
            SpecialItemTexture::Dynamic {
                texture,
                logical_size,
            } => {
                if logical_size.contains(&0) {
                    bail!("dynamic special item texture has zero-sized logical dimensions");
                }
                let atlas = self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("dynamic entity texture atlas is not uploaded")
                    })?;
                let uv = atlas.uv_for_handle(texture.handle).ok_or_else(|| {
                    anyhow::anyhow!(
                        "dynamic entity texture handle {} is absent from the atlas",
                        texture.handle
                    )
                })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::DynamicEntity,
                    logical_size: *logical_size,
                    uv,
                    identity: Arc::from(format!("dynamic:{}:{logical_size:?}", texture.handle)),
                })
            }
            SpecialItemTexture::DynamicPlayerSkin(skin) => {
                if skin.status != EntityDynamicPlayerSkinStatus::Ready {
                    bail!(
                        "dynamic player skin {} is not ready for special item rendering",
                        skin.handle
                    );
                }
                let atlas = self
                    .entity_dynamic_player_skin_atlas
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("dynamic player skin atlas is not uploaded"))?;
                let uv = atlas.uv_for_handle(skin.handle).ok_or_else(|| {
                    anyhow::anyhow!(
                        "dynamic player skin handle {} is absent from the atlas",
                        skin.handle
                    )
                })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::DynamicPlayerSkin,
                    logical_size: [64, 64],
                    uv,
                    identity: Arc::from(format!(
                        "dynamic_player_skin:{}:{:?}",
                        skin.handle, skin.model
                    )),
                })
            }
        }
    }

    /// Computes the exact model-space bounds used by Vanilla's item extent
    /// visits and oversized-GUI bounds. The same resolved atlas rectangles and
    /// posed builders feed both this query and resident geometry.
    pub fn special_item_model_bounds(
        &self,
        model: &SpecialItemModel,
    ) -> Result<SpecialItemModelBounds> {
        let mut bounds_model = model.clone();
        if let SpecialItemModel::Banner { attachment, .. } = &mut bounds_model {
            // Vanilla `BannerRenderer.getExtents` always visits the standing
            // frame and flag, independently of the configured attachment.
            *attachment = super::SpecialItemBannerAttachment::Ground;
        }
        let parts = self.special_item_parts(&bounds_model)?;
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for corner in parts
            .iter()
            // `DecoratedPotRenderer.getExtents` visits neck/top/bottom only;
            // its material side quads are submit geometry, not extents.
            .filter(|part| {
                !matches!(model, SpecialItemModel::DecoratedPot { .. })
                    || part.semantic_part.as_ref() == "decorated_pot_base"
            })
            .flat_map(|part| part.quads.iter())
            .flat_map(|quad| quad.corners)
        {
            for axis in 0..3 {
                let coordinate = corner[axis] / 16.0;
                min[axis] = min[axis].min(coordinate);
                max[axis] = max[axis].max(coordinate);
            }
        }
        if min.iter().any(|value| !value.is_finite()) || max.iter().any(|value| !value.is_finite())
        {
            bail!("special item model produced no finite extents");
        }
        Ok(SpecialItemModelBounds { min, max })
    }

    /// Advances the resource generation and drops all derived model owners.
    pub fn invalidate_item_model_resources(&mut self) {
        self.pack
            .item_model_residency
            .invalidate_resources(&mut self.counters);
    }

    pub(crate) fn invalidate_item_model_atlas(&mut self, atlas: ItemModelAtlasKind) {
        if self.resource_reload_in_progress() {
            return;
        }
        self.pack
            .item_model_residency
            .invalidate_atlas(atlas, &mut self.counters);
    }

    #[cfg(test)]
    pub(crate) fn set_item_model_residency_limits_for_test(
        &mut self,
        max_owners: usize,
        max_bytes: u64,
    ) {
        self.pack
            .item_model_residency
            .set_limits(max_owners, max_bytes);
    }

    #[cfg(test)]
    pub(crate) fn resident_item_model_instance_orders_for_test(&self) -> Vec<u32> {
        self.pack
            .item_model_residency
            .instances
            .iter()
            .map(|instance| instance.submit_order)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use glam::Mat4;

    use super::*;

    const SPECIAL_TRIDENT_TEXTURE: crate::EntityModelTextureRef = crate::EntityModelTextureRef {
        path: "textures/entity/test_special_trident.png",
        size: [64, 64],
    };
    const SPECIAL_SKY_TEXTURE: crate::EntityModelTextureRef = crate::EntityModelTextureRef {
        path: "textures/entity/test_special_sky.png",
        size: [16, 16],
    };
    const SPECIAL_PORTAL_TEXTURE: crate::EntityModelTextureRef = crate::EntityModelTextureRef {
        path: "textures/entity/test_special_portal.png",
        size: [16, 16],
    };

    fn quad(translucent: bool) -> ItemModelQuad {
        ItemModelQuad {
            corners: [
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            tint: [1.0; 4],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent,
        }
    }

    fn identity(name: &str) -> ItemModelAssetIdentity {
        ItemModelAssetIdentity::new(Arc::<str>::from(name), Arc::<str>::from("plain"), 0)
    }

    fn block_frame(transform: Mat4) -> ItemModelResidencyFrame {
        let identity = identity("minecraft:stone");
        ItemModelResidencyFrame {
            assets: vec![ItemModelAssetRequest {
                identity: identity.clone(),
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                source: ItemModelAssetSource::Quads(Arc::from([quad(false)])),
            }],
            instances: vec![ItemModelInstanceRequest {
                identity,
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                transform,
                tint: [1.0; 4],
                light: [1.0; 2],
                overlay: [0.0, 10.0],
                foil: ItemModelFoil::None,
                target: ItemModelDrawTarget::FirstPerson,
                phase: ItemModelDrawPhase::Solid,
                submit_order: 0,
                scissor: None,
            }],
            special_items: Vec::new(),
        }
    }

    #[test]
    fn special_item_texture_sources_resolve_all_entity_atlas_owners() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let owned_texture = crate::EntityModelOwnedTextureRef::new(
            "textures/entity/test_special_owned.png",
            [64, 32],
        );
        renderer
            .upload_entity_model_textures_with_owned(
                &[crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_TRIDENT_TEXTURE,
                    [1, 1],
                    vec![255; 4],
                )],
                &[crate::EntityModelOwnedTextureImage::with_physical_size(
                    owned_texture.clone(),
                    [2, 1],
                    vec![127; 8],
                )],
            )
            .unwrap();
        renderer
            .upload_dynamic_entity_texture(crate::DynamicEntityTextureImage {
                handle: 42,
                size: [2, 3],
                standalone_sampling: None,
                rgba: vec![63; 2 * 3 * 4],
            })
            .unwrap();
        renderer
            .upload_dynamic_player_skin(crate::DynamicPlayerSkinImage {
                handle: 43,
                standalone_sampling: None,
                rgba: vec![31; 64 * 64 * 4],
            })
            .unwrap();

        let static_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE))
            .unwrap();
        assert_eq!(static_texture.atlas, ItemModelAtlasKind::Entity);
        assert_eq!(static_texture.logical_size, [64, 64]);

        let pack_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Pack(owned_texture.clone()))
            .unwrap();
        assert_eq!(pack_texture.atlas, ItemModelAtlasKind::Entity);
        assert_eq!(pack_texture.logical_size, [64, 32]);
        let shared_pack_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Pack(
                crate::EntityModelOwnedTextureRef::new(owned_texture.path, [256, 256]),
            ))
            .unwrap();
        assert_eq!(shared_pack_texture.atlas, ItemModelAtlasKind::Entity);
        assert_eq!(shared_pack_texture.logical_size, [256, 256]);

        let dynamic_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Dynamic {
                texture: crate::EntityDynamicTexture { handle: 42 },
                logical_size: [128, 64],
            })
            .unwrap();
        assert_eq!(dynamic_texture.atlas, ItemModelAtlasKind::DynamicEntity);
        assert_eq!(dynamic_texture.logical_size, [128, 64]);

        let dynamic_skin = renderer
            .resolve_special_item_texture(&SpecialItemTexture::DynamicPlayerSkin(
                crate::EntityDynamicPlayerSkin {
                    handle: 43,
                    fallback: crate::EntityDefaultPlayerSkin::WideSteve,
                    model: crate::EntityPlayerSkinModel::Wide,
                    status: crate::EntityDynamicPlayerSkinStatus::Ready,
                },
            ))
            .unwrap();
        assert_eq!(dynamic_skin.atlas, ItemModelAtlasKind::DynamicPlayerSkin);
        assert_eq!(dynamic_skin.logical_size, [64, 64]);
    }

    #[test]
    fn exact_hit_does_not_evaluate_geometry_builder_or_upload_static_payload() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:stone"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        let mut builds = 0;
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || {
                builds += 1;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Created
        );
        let uploaded = renderer.counters().item_model_vertex_upload_bytes
            + renderer.counters().item_model_index_upload_bytes;
        assert!(uploaded > 0);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(key, || {
                builds += 1;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Reused
        );
        assert_eq!(builds, 1);
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
    }

    #[test]
    fn dynamic_instance_changes_upload_only_narrow_instance_payload() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:diamond"),
            ItemModelAtlasKind::Items,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        renderer
            .set_resident_item_model_instances(vec![ItemModelInstanceRecord::new(
                key.clone(),
                Mat4::IDENTITY,
                [1.0; 4],
                [1.0; 2],
                [0.0, 10.0],
                ItemModelFoil::None,
                ItemModelDrawTarget::World,
                ItemModelDrawPhase::Solid,
                9,
            )])
            .unwrap();

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), Vec::new),
            ItemModelResidencyOutcome::Reused
        );
        renderer
            .set_resident_item_model_instances(vec![ItemModelInstanceRecord::new(
                key,
                Mat4::from_translation(glam::Vec3::X),
                [0.5, 1.0, 1.0, 1.0],
                [0.5, 1.0],
                [0.0, 10.0],
                ItemModelFoil::Standard,
                ItemModelDrawTarget::World,
                ItemModelDrawPhase::Glint,
                9,
            )])
            .unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            std::mem::size_of::<ItemModelGpuInstance>() as u64
        );
        assert_eq!(
            renderer.resident_item_model_instance_orders_for_test(),
            vec![9]
        );
    }

    #[test]
    fn identical_gpu_instances_skip_upload_while_draw_metadata_refreshes() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:stone"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        renderer
            .set_resident_item_model_instances(vec![ItemModelInstanceRecord::new(
                key.clone(),
                Mat4::IDENTITY,
                [1.0; 4],
                [1.0; 2],
                [0.0, 10.0],
                ItemModelFoil::None,
                ItemModelDrawTarget::Gui,
                ItemModelDrawPhase::Solid,
                9,
            )])
            .unwrap();
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            std::mem::size_of::<ItemModelGpuInstance>() as u64
        );

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), Vec::new),
            ItemModelResidencyOutcome::Reused
        );
        let scissor = ItemModelScissorRect::new(1, 2, 3, 4);
        renderer
            .set_resident_item_model_instances(vec![ItemModelInstanceRecord::new(
                key,
                Mat4::IDENTITY,
                [1.0; 4],
                [1.0; 2],
                [0.0, 10.0],
                ItemModelFoil::None,
                ItemModelDrawTarget::Gui,
                ItemModelDrawPhase::Translucent,
                10,
            )
            .with_scissor(scissor)])
            .unwrap();

        assert_eq!(renderer.counters().item_instance_upload_bytes, 0);
        assert_eq!(renderer.pack.item_model_residency.draws().len(), 1);
        assert_eq!(
            renderer.pack.item_model_residency.draws()[0].target,
            ItemModelDrawTarget::Gui
        );
        assert_eq!(
            renderer.pack.item_model_residency.draws()[0].phase,
            ItemModelDrawPhase::Translucent
        );
        assert_eq!(
            renderer.pack.item_model_residency.draws()[0].scissor,
            scissor
        );
        assert_eq!(
            renderer.resident_item_model_instance_orders_for_test(),
            vec![10]
        );
    }

    #[test]
    fn sparse_instance_mutations_upload_bounded_changed_ranges() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        assert_eq!(std::mem::size_of::<ItemModelGpuInstance>(), 160);
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:diamond"),
            ItemModelAtlasKind::Items,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        let mut instances = (0..64)
            .map(|index| {
                ItemModelInstanceRecord::new(
                    key.clone(),
                    Mat4::from_translation(glam::Vec3::new(index as f32, 0.0, 0.0)),
                    [1.0; 4],
                    [1.0; 2],
                    [0.0, 10.0],
                    ItemModelFoil::None,
                    ItemModelDrawTarget::World,
                    ItemModelDrawPhase::Solid,
                    index,
                )
            })
            .collect::<Vec<_>>();
        renderer
            .set_resident_item_model_instances(instances.clone())
            .unwrap();
        let full_frame_bytes =
            (instances.len() * std::mem::size_of::<ItemModelGpuInstance>()) as u64;
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            full_frame_bytes
        );
        assert_eq!(renderer.counters().item_instance_upload_writes, 1);
        assert_eq!(renderer.counters().item_instance_buffer_replacements, 1);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), Vec::new),
            ItemModelResidencyOutcome::Reused
        );
        renderer
            .set_resident_item_model_instances(instances[..32].to_vec())
            .unwrap();
        assert_eq!(renderer.counters().item_instance_upload_bytes, 0);
        assert_eq!(renderer.counters().item_instance_upload_writes, 0);
        assert_eq!(renderer.counters().item_instance_buffer_replacements, 0);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(key, Vec::new),
            ItemModelResidencyOutcome::Reused
        );
        renderer
            .set_resident_item_model_instances(instances.clone())
            .unwrap();
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            (32 * std::mem::size_of::<ItemModelGpuInstance>()) as u64
        );
        assert_eq!(renderer.counters().item_instance_upload_writes, 1);
        assert_eq!(renderer.counters().item_instance_buffer_replacements, 0);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(instances[0].asset.clone(), Vec::new,),
            ItemModelResidencyOutcome::Reused
        );
        instances[31].transform *= Mat4::from_translation(glam::Vec3::Y);
        renderer
            .set_resident_item_model_instances(instances.clone())
            .unwrap();
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            std::mem::size_of::<ItemModelGpuInstance>() as u64
        );
        assert_eq!(renderer.counters().item_instance_upload_writes, 1);
        assert_eq!(renderer.counters().item_instance_buffer_replacements, 0);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(instances[0].asset.clone(), Vec::new,),
            ItemModelResidencyOutcome::Reused
        );
        instances[10].transform *= Mat4::from_translation(glam::Vec3::Y);
        instances[50].transform *= Mat4::from_translation(glam::Vec3::Y);
        renderer
            .set_resident_item_model_instances(instances.clone())
            .unwrap();
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            (2 * std::mem::size_of::<ItemModelGpuInstance>()) as u64
        );
        assert_eq!(renderer.counters().item_instance_upload_writes, 2);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(instances[0].asset.clone(), Vec::new,),
            ItemModelResidencyOutcome::Reused
        );
        for index in (0..36).step_by(2) {
            instances[index].transform *= Mat4::from_translation(glam::Vec3::Y);
        }
        renderer
            .set_resident_item_model_instances(instances)
            .unwrap();
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            (35 * std::mem::size_of::<ItemModelGpuInstance>()) as u64
        );
        assert_eq!(
            renderer.counters().item_instance_upload_writes,
            1,
            "more than the write-call budget collapses to one enclosing span"
        );
    }

    #[test]
    fn high_level_frame_reuses_static_owner_and_full_render_consumes_upload_once() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer
            .update_terrain_texture_atlas(&[255, 255, 255, 255])
            .unwrap();
        renderer.update_camera();
        renderer
            .set_item_model_residency_frame(block_frame(Mat4::IDENTITY))
            .unwrap();
        renderer.render_offscreen_frame().unwrap();
        assert!(renderer.counters().item_model_vertex_upload_bytes > 0);
        assert!(renderer.counters().item_model_index_upload_bytes > 0);
        assert!(renderer.counters().item_instance_upload_bytes > 0);
        assert_eq!(renderer.counters().item_instance_upload_writes, 1);
        assert_eq!(renderer.counters().item_instance_buffer_replacements, 1);

        renderer
            .set_item_model_residency_frame(block_frame(Mat4::from_translation(glam::Vec3::new(
                0.25, 0.0, 0.0,
            ))))
            .unwrap();
        renderer.render_offscreen_frame().unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
        assert!(renderer.counters().item_instance_upload_bytes > 0);
        assert_eq!(renderer.counters().item_instance_upload_writes, 1);
        assert_eq!(renderer.counters().item_instance_buffer_replacements, 0);

        renderer.render_offscreen_frame().unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
        assert_eq!(renderer.counters().item_instance_upload_bytes, 0);
        assert_eq!(renderer.counters().item_instance_upload_writes, 0);
        assert_eq!(renderer.counters().item_instance_buffer_replacements, 0);
    }

    #[test]
    fn consecutive_instances_share_one_draw_without_reordering() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut frame = block_frame(Mat4::IDENTITY);
        let mut second = frame.instances[0].clone();
        second.transform = Mat4::from_translation(glam::Vec3::X);
        second.submit_order = 1;
        frame.instances.push(second);
        renderer.set_item_model_residency_frame(frame).unwrap();

        assert_eq!(renderer.pack.item_model_residency.draws().len(), 1);
        assert_eq!(
            renderer.pack.item_model_residency.draws()[0].instance_count,
            2
        );
        assert_eq!(
            renderer.resident_item_model_instance_orders_for_test(),
            vec![0, 1]
        );
    }

    #[test]
    fn resident_item_model_main_and_entity_outline_instances_share_one_owner() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut frame = block_frame(Mat4::IDENTITY);
        frame.instances[0].target = ItemModelDrawTarget::BlockFeature;
        let mut outline = frame.instances[0].clone();
        outline.tint = [0.2, 0.4, 0.8, 1.0];
        outline.target = ItemModelDrawTarget::EntityOutline;
        outline.phase = ItemModelDrawPhase::Outline;
        outline.submit_order = 1;
        frame.instances.push(outline);

        renderer.set_item_model_residency_frame(frame).unwrap();

        assert_eq!(renderer.counters().item_gpu_owners_created, 1);
        assert_eq!(renderer.counters().item_model_residency_builds, 1);
        assert_eq!(renderer.pack.item_model_residency.draws().len(), 2);
        let main = &renderer.pack.item_model_residency.draws()[0];
        let outline = &renderer.pack.item_model_residency.draws()[1];
        assert_eq!(main.asset, outline.asset);
        assert_eq!(main.target, ItemModelDrawTarget::BlockFeature);
        assert_eq!(main.phase, ItemModelDrawPhase::Solid);
        assert_eq!(outline.target, ItemModelDrawTarget::EntityOutline);
        assert_eq!(outline.phase, ItemModelDrawPhase::Outline);
    }

    #[test]
    fn resident_draw_batching_splits_equal_assets_at_scissor_boundaries() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 16) else {
            return;
        };
        let mut frame = block_frame(Mat4::IDENTITY);
        frame.instances[0].target = ItemModelDrawTarget::Gui;
        frame.instances[0].scissor = ItemModelScissorRect::new(0, 0, 8, 16);
        let mut second = frame.instances[0].clone();
        second.transform = Mat4::from_translation(glam::Vec3::X);
        second.submit_order = 1;
        second.scissor = None;
        frame.instances.push(second);
        renderer.set_item_model_residency_frame(frame).unwrap();

        assert_eq!(renderer.pack.item_model_residency.draws().len(), 2);
        assert_eq!(
            renderer.pack.item_model_residency.draws()[0].scissor,
            ItemModelScissorRect::new(0, 0, 8, 16)
        );
        assert_eq!(renderer.pack.item_model_residency.draws()[1].scissor, None);
    }

    #[test]
    fn resident_scissor_rejects_non_gui_targets() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut frame = block_frame(Mat4::IDENTITY);
        frame.instances[0].scissor = ItemModelScissorRect::new(0, 0, 8, 8);
        let error = renderer
            .set_item_model_residency_frame(frame)
            .expect_err("world/first-person scissor must fail closed");
        assert!(error
            .to_string()
            .contains("scissor is only valid for GUI draw targets"));
    }

    #[test]
    fn reload_rejects_old_key_without_evaluating_builder() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:clock"),
            ItemModelAtlasKind::Items,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        let previous_generation = key.resource_generation();
        renderer.invalidate_item_model_resources();
        let mut rebuilt = false;
        assert_eq!(
            renderer.ensure_item_model_resident(key, || {
                rebuilt = true;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Stale
        );
        assert!(!rebuilt);
        assert!(renderer.item_model_resource_generation() > previous_generation);
        assert_eq!(renderer.counters().item_full_residency_replacements, 1);
    }

    #[test]
    fn atlas_replacement_invalidates_only_matching_material_owners() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let block_key = renderer.item_model_asset_key(
            identity("minecraft:stone"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        let item_key = renderer.item_model_asset_key(
            identity("minecraft:apple"),
            ItemModelAtlasKind::Items,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(block_key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        assert_eq!(
            renderer.ensure_item_model_resident(item_key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );

        renderer.invalidate_item_model_atlas(ItemModelAtlasKind::Items);
        assert_eq!(
            renderer.ensure_item_model_resident(block_key, Vec::new),
            ItemModelResidencyOutcome::Reused
        );
        let mut stale_item_built = false;
        assert_eq!(
            renderer.ensure_item_model_resident(item_key, || {
                stale_item_built = true;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Stale
        );
        assert!(!stale_item_built);
        assert_eq!(renderer.counters().item_full_residency_replacements, 0);
    }

    #[test]
    fn current_frame_pins_make_capacity_fallback_explicit() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_item_model_residency_limits_for_test(1, u64::MAX);
        renderer.begin_item_model_frame();
        let first = renderer.item_model_asset_key(
            identity("minecraft:stone"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        let second = renderer.item_model_asset_key(
            identity("minecraft:dirt"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(first, || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        let mut second_built_while_pinned = false;
        assert_eq!(
            renderer.ensure_item_model_resident(second.clone(), || {
                second_built_while_pinned = true;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::CapacityExceeded
        );
        assert!(!second_built_while_pinned);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(second, || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        assert!(renderer.counters().item_model_resident_bytes > 0);
        assert_eq!(renderer.counters().item_model_fallbacks, 1);
    }

    #[test]
    fn special_item_model_residency_reuses_entity_geometry_and_draws_end_cube_pipeline() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer
            .upload_entity_model_textures(&[
                crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_TRIDENT_TEXTURE,
                    [1, 1],
                    vec![255; 4],
                ),
                crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_SKY_TEXTURE,
                    [1, 1],
                    vec![31, 63, 95, 255],
                ),
                crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_PORTAL_TEXTURE,
                    [1, 1],
                    vec![127, 63, 191, 255],
                ),
            ])
            .unwrap();
        renderer
            .upload_dynamic_player_skin(crate::DynamicPlayerSkinImage {
                handle: 41,
                standalone_sampling: None,
                rgba: vec![0xff; 64 * 64 * 4],
            })
            .unwrap();
        renderer.update_camera();

        let frame = || ItemModelResidencyFrame {
            special_items: vec![
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Trident {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::Standard,
                    target: ItemModelDrawTarget::World,
                    outline_color: None,
                    submit_order: 7,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::EndCube {
                        sky_texture: SpecialItemTexture::Static(SPECIAL_SKY_TEXTURE),
                        portal_texture: SpecialItemTexture::Static(SPECIAL_PORTAL_TEXTURE),
                        effect: super::super::SpecialItemEndCubeEffect::Gateway,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::Standard,
                    target: ItemModelDrawTarget::World,
                    outline_color: None,
                    submit_order: 8,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Trident {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    outline_color: None,
                    submit_order: 9,
                    scissor: ItemModelScissorRect::new(1, 2, 3, 4),
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Banner {
                        attachment: super::super::SpecialItemBannerAttachment::Ground,
                        base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        pattern_base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        base_color: crate::EntityDyeColor::Blue,
                        patterns: Arc::from([super::super::SpecialItemPatternLayer::new(
                            SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                            crate::EntityDyeColor::Red,
                        )]),
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    outline_color: None,
                    submit_order: 10,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::CopperGolemStatue {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        pose: super::super::SpecialItemCopperGolemPose::Standing,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    outline_color: None,
                    submit_order: 11,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Head {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        kind: super::super::SpecialItemHeadKind::Skeleton,
                        animation: 0.0,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    outline_color: None,
                    submit_order: 12,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::PlayerHead {
                        skin: Some(crate::EntityDynamicPlayerSkin {
                            handle: 41,
                            fallback: crate::EntityDefaultPlayerSkin::WideSteve,
                            model: crate::EntityPlayerSkinModel::Wide,
                            status: crate::EntityDynamicPlayerSkinStatus::Ready,
                        }),
                        fallback_texture: SPECIAL_TRIDENT_TEXTURE,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    outline_color: None,
                    submit_order: 13,
                    scissor: None,
                },
            ],
            ..ItemModelResidencyFrame::default()
        };

        renderer.set_item_model_residency_frame(frame()).unwrap();
        assert_eq!(
            renderer.resident_item_model_instance_orders_for_test(),
            vec![7, 7, 8, 9, 10, 10, 10, 10, 11, 12, 13]
        );
        assert!(renderer
            .pack
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| draw.phase == ItemModelDrawPhase::EndGateway));
        assert!(renderer
            .pack
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| draw.phase == ItemModelDrawPhase::Pattern));
        assert!(renderer
            .pack
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| draw.scissor == ItemModelScissorRect::new(1, 2, 3, 4)));
        for phase in [
            ItemModelDrawPhase::CutoutNoCull,
            ItemModelDrawPhase::CutoutZOffset,
            ItemModelDrawPhase::TranslucentNoCull,
        ] {
            assert!(renderer
                .pack
                .item_model_residency
                .draws()
                .iter()
                .any(|draw| draw.phase == phase));
        }
        assert!(renderer
            .pack
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| {
                draw.phase == ItemModelDrawPhase::TranslucentNoCull
                    && draw.asset.atlas() == ItemModelAtlasKind::DynamicPlayerSkin
            }));
        let bounds = renderer
            .special_item_model_bounds(&SpecialItemModel::Trident {
                texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
            })
            .unwrap();
        assert!(bounds.size().iter().all(|size| *size > 0.0));
        let banner = |attachment| SpecialItemModel::Banner {
            attachment,
            base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
            pattern_base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
            base_color: crate::EntityDyeColor::White,
            patterns: Arc::from([]),
        };
        assert_eq!(
            renderer
                .special_item_model_bounds(&banner(
                    super::super::SpecialItemBannerAttachment::Ground
                ))
                .unwrap(),
            renderer
                .special_item_model_bounds(&banner(super::super::SpecialItemBannerAttachment::Wall))
                .unwrap()
        );
        renderer.render_offscreen_frame().unwrap();

        renderer.set_item_model_residency_frame(frame()).unwrap();
        renderer.render_offscreen_frame().unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);

        let mut glowing = frame();
        for special in &mut glowing.special_items {
            special.target = ItemModelDrawTarget::World;
            special.scissor = None;
            special.outline_color = Some(0xFF12_3456);
        }
        renderer.set_item_model_residency_frame(glowing).unwrap();
        let outline_draws: Vec<_> = renderer
            .pack
            .item_model_residency
            .draws()
            .iter()
            .filter(|draw| draw.target == ItemModelDrawTarget::EntityOutline)
            .collect();
        assert!(!outline_draws.is_empty());
        assert!(outline_draws
            .iter()
            .all(|draw| draw.phase == ItemModelDrawPhase::Outline));
        renderer.render_offscreen_frame().unwrap();
    }
}
