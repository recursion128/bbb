use super::*;
use hashbrown::{hash_map::Entry, HashMap};
use std::mem::Discriminant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EntityCacheKey {
    entity_id: i32,
    kind_tag: Discriminant<EntityModelKind>,
    /// Block-entity sentinels share negative `entity_id`s (typically `-1`) and are stably
    /// identified by their world block position; network entities (`entity_id >= 0`) keep `[0; 3]`
    /// so moving entities update their entry in place without churning keys.
    sentinel_pos_bits: [u32; 3],
    /// Disambiguates multi-part submissions that share `(entity_id, kind_tag, sentinel_pos_bits)`
    /// within the same frame (e.g. PaintingTile segments, Conduit parts, BeaconBeam sections).
    occurrence: u32,
}

impl EntityCacheKey {
    fn base(instance: &EntityModelInstance) -> Self {
        Self {
            entity_id: instance.entity_id,
            kind_tag: std::mem::discriminant(&instance.kind),
            sentinel_pos_bits: if instance.entity_id < 0 {
                instance.position.map(f32::to_bits)
            } else {
                [0; 3]
            },
            occurrence: 0,
        }
    }
}

#[derive(Default)]
pub(crate) struct EntityExtractionCache {
    entries: HashMap<EntityCacheKey, CachedEntity>,
    generation: u64,
    solid_scratch: EntityModelTexturedMesh,
    cutout_scratch: EntityModelTexturedMesh,
    cutout_cull_scratch: EntityModelTexturedMesh,
    #[cfg(test)]
    last_build_hits: usize,
    #[cfg(test)]
    last_build_misses: usize,
}
struct CachedEntity {
    /// Out-of-line instance snapshot keeps `HashMap` slots compact (~88 bytes vs >1 KB)
    /// and is updated in place for moving entities without reallocating.
    instance: Box<EntityModelInstance>,
    orientation: Option<Mat4>,
    geometry: Option<Box<(EntityModelTexturedMeshes, EntityModelMesh)>>,
    last_seen_generation: u64,
}

impl EntityExtractionCache {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.generation = 0;
        self.solid_scratch.clear();
        self.cutout_scratch.clear();
        self.cutout_cull_scratch.clear();
    }

    pub(in crate::entity_models) fn recycle_surface_meshes(
        &mut self,
        mut solid: EntityModelTexturedMesh,
        mut cutout: EntityModelTexturedMesh,
        mut cutout_cull: EntityModelTexturedMesh,
    ) {
        solid.clear();
        cutout.clear();
        cutout_cull.clear();
        self.solid_scratch = solid;
        self.cutout_scratch = cutout;
        self.cutout_cull_scratch = cutout_cull;
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::entity_models) fn build(
        &mut self,
        instances: &[EntityModelInstance],
        atlas: &EntityModelTextureAtlasLayout,
        owned: &[EntityModelOwnedTextureAtlasEntry],
        equipment: &ResolvedEntityEquipmentAssetCatalog,
        skin: Option<&EntityDynamicPlayerSkinAtlasLayout>,
        textures: Option<&EntityDynamicPlayerTextureAtlasLayout>,
        data_driven_rigs: &[crate::entity_models::BakedDataDrivenRig],
        camera: Option<[f32; 3]>,
        pose: Option<crate::CameraPose>,
    ) -> (EntityModelTexturedMeshes, EntityModelMesh) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.entries.clear();
            self.generation = 1;
        }
        let generation = self.generation;
        #[cfg(test)]
        {
            self.last_build_hits = 0;
            self.last_build_misses = 0;
        }
        // A sort key is needed even without a camera: that frame uses the origin.
        let mut result = EntityModelTexturedMeshes::new(Some([0.0; 3]));
        result.solid = std::mem::take(&mut self.solid_scratch);
        result.cutout = std::mem::take(&mut self.cutout_scratch);
        result.cutout_cull = std::mem::take(&mut self.cutout_cull_scratch);
        let mut water = EntityModelMesh::new();
        for instance in instances {
            let orientation = super::super::gpu::entity_model_instances_have_camera_billboards(
                std::slice::from_ref(instance),
            )
            .then(|| pose.map(camera_billboard_orientation))
            .flatten();
            let emit = |meshes: &mut EntityModelTexturedMeshes, water: &mut EntityModelMesh| {
                meshes.retain_parts = true;
                meshes.camera_orientation = orientation;
                TexturedSink {
                    meshes,
                    atlas,
                    owned_entries: owned,
                    equipment_assets: equipment,
                    dynamic_player_skin_atlas: skin,
                    dynamic_player_texture_atlas: textures,
                    data_driven_rigs,
                }
                .emit_instance(instance);
                special_models::emit_entity_water_mask(water, instance);
            };
            let mut key = EntityCacheKey::base(instance);
            while self
                .entries
                .get(&key)
                .is_some_and(|entry| entry.last_seen_generation == generation)
            {
                key.occurrence = key.occurrence.saturating_add(1);
            }
            match self.entries.entry(key) {
                Entry::Occupied(mut occupied) => {
                    let entry = occupied.get_mut();
                    if *entry.instance == *instance && entry.orientation == orientation {
                        entry.last_seen_generation = generation;
                        let cached = entry.geometry.get_or_insert_with(|| {
                            let mut meshes = EntityModelTexturedMeshes::new(Some([0.0; 3]));
                            let mut water = EntityModelMesh::new();
                            emit(&mut meshes, &mut water);
                            Box::new((meshes, water))
                        });
                        result.append_cached(&cached.0);
                        append_mesh!(water, cached.1, vertices.len());
                        water.opaque_faces += cached.1.opaque_faces;
                        #[cfg(test)]
                        {
                            self.last_build_hits += 1;
                        }
                    } else {
                        // Reuse the existing slot and Box allocation in place for moving entities.
                        emit(&mut result, &mut water);
                        *entry.instance = *instance;
                        entry.orientation = orientation;
                        entry.geometry = None;
                        entry.last_seen_generation = generation;
                        #[cfg(test)]
                        {
                            self.last_build_misses += 1;
                        }
                    }
                }
                Entry::Vacant(vacant) => {
                    // Only retain separate geometry once this instance repeats across builds.
                    emit(&mut result, &mut water);
                    vacant.insert(CachedEntity {
                        instance: Box::new(*instance),
                        orientation,
                        geometry: None,
                        last_seen_generation: generation,
                    });
                    #[cfg(test)]
                    {
                        self.last_build_misses += 1;
                    }
                }
            }
        }
        self.entries
            .retain(|_, entry| entry.last_seen_generation == generation);
        result.resort(camera);
        (result, water)
    }
}

macro_rules! append_mesh {
    ($dest:expr, $src:expr, $($count:tt)*) => {{
        let base = $dest.$($count)* as u32;
        $dest.vertices.extend_from_slice(&$src.vertices);
        $dest.indices.extend($src.indices.iter().map(|index| index + base));
    }};
}
use append_mesh;

impl EntityModelTexturedMeshes {
    fn textured_offset(&mut self, draw: EntityModelTexturedDrawRange) -> u32 {
        let mesh = match draw.atlas {
            EntityModelTexturedDrawAtlas::Static => self.mesh_mut(draw.render_type),
            EntityModelTexturedDrawAtlas::DynamicPlayerSkin => {
                self.dynamic_player_skin_mesh_mut(draw.render_type)
            }
            EntityModelTexturedDrawAtlas::DynamicPlayerTexture => {
                self.dynamic_player_texture_mesh_mut(draw.render_type)
            }
        };
        mesh.indices.len() as u32
    }
    fn geometry_offset(&self, kind: EntityModelLayerRenderType) -> u32 {
        use EntityModelLayerRenderType::*;
        (match kind {
            EndPortal => self.end_portal.indices.len(),
            EndGateway => self.end_gateway.indices.len(),
            DragonRays => self.dragon_rays.indices.len(),
            DragonRaysDepth => self.dragon_rays_depth.indices.len(),
            Leash => self.leash.indices.len(),
            BreezeWind => self.scroll.indices.len(),
            EnergySwirl => self.scroll_additive.indices.len(),
            BeaconBeamTranslucent => self.beacon_beam_translucent.indices.len(),
            EntityCutout => self.guardian_beam.indices.len(),
            EndCrystalBeam => self.end_crystal_beam.indices.len(),
            BeaconBeamOpaque => self.beacon_beam_opaque.indices.len(),
            _ => unreachable!("custom entity geometry render type"),
        }) as u32
    }
    fn append_cached(&mut self, source: &Self) {
        for draw in &source.sorted_item_entity_draws {
            let mut draw = *draw;
            draw.index_start += self.textured_offset(draw);
            draw.sort_key.insertion_index += self.next_submission_index;
            self.sorted_item_entity_draws.push(draw);
        }
        for draw in &source.sorted_main_translucent_draws {
            let draw = match *draw {
                EntityModelTranslucentDrawRange::Textured(mut draw) => {
                    draw.index_start += self.textured_offset(draw);
                    draw.sort_key.insertion_index += self.next_submission_index;
                    EntityModelTranslucentDrawRange::Textured(draw)
                }
                EntityModelTranslucentDrawRange::Geometry(mut draw) => {
                    draw.index_start += self.geometry_offset(draw.render_type);
                    draw.sort_key.insertion_index += self.next_submission_index;
                    EntityModelTranslucentDrawRange::Geometry(draw)
                }
            };
            self.sorted_main_translucent_draws.push(draw);
        }
        for draw in &source.opaque_beam_draws {
            let mut draw = *draw;
            draw.index_start += self.geometry_offset(draw.render_type);
            draw.sort_key.insertion_index += self.next_submission_index;
            self.opaque_beam_draws.push(draw);
        }
        append_mesh!(self.crumbling, source.crumbling, vertices.len());
        append_textured(&mut self.solid, &source.solid);
        append_textured(&mut self.cutout, &source.cutout);
        append_textured(&mut self.cutout_cull, &source.cutout_cull);
        append_textured(
            &mut self.solid_z_offset_forward,
            &source.solid_z_offset_forward,
        );
        append_mesh!(self.dissolve, source.dissolve, vertices.len());
        self.dissolve.cutout_faces += source.dissolve.cutout_faces;
        append_textured(&mut self.cutout_z_offset, &source.cutout_z_offset);
        append_textured(&mut self.armor_cutout, &source.armor_cutout);
        append_textured(&mut self.armor_trim, &source.armor_trim);
        append_textured(&mut self.armor_trim_decal, &source.armor_trim_decal);
        append_textured(&mut self.translucent, &source.translucent);
        append_textured(&mut self.armor_translucent, &source.armor_translucent);
        append_textured(&mut self.translucent_emissive, &source.translucent_emissive);
        append_textured(
            &mut self.item_entity_translucent,
            &source.item_entity_translucent,
        );
        append_textured(
            &mut self.item_entity_translucent_cull,
            &source.item_entity_translucent_cull,
        );
        append_textured(&mut self.eyes, &source.eyes);
        append_textured(&mut self.outline, &source.outline);
        append_textured(&mut self.outline_cull, &source.outline_cull);
        append_textured(
            &mut self.dynamic_player_skin_solid,
            &source.dynamic_player_skin_solid,
        );
        append_textured(
            &mut self.dynamic_player_skin_cutout,
            &source.dynamic_player_skin_cutout,
        );
        append_textured(
            &mut self.dynamic_player_skin_cutout_cull,
            &source.dynamic_player_skin_cutout_cull,
        );
        append_textured(
            &mut self.dynamic_player_skin_solid_z_offset_forward,
            &source.dynamic_player_skin_solid_z_offset_forward,
        );
        append_textured(
            &mut self.dynamic_player_skin_cutout_z_offset,
            &source.dynamic_player_skin_cutout_z_offset,
        );
        append_textured(
            &mut self.dynamic_player_skin_translucent,
            &source.dynamic_player_skin_translucent,
        );
        append_textured(
            &mut self.dynamic_player_skin_item_entity_translucent,
            &source.dynamic_player_skin_item_entity_translucent,
        );
        append_textured(
            &mut self.dynamic_player_skin_item_entity_translucent_cull,
            &source.dynamic_player_skin_item_entity_translucent_cull,
        );
        append_textured(
            &mut self.dynamic_player_skin_outline,
            &source.dynamic_player_skin_outline,
        );
        append_textured(
            &mut self.dynamic_player_skin_outline_cull,
            &source.dynamic_player_skin_outline_cull,
        );
        append_textured(
            &mut self.dynamic_player_texture_solid,
            &source.dynamic_player_texture_solid,
        );
        append_textured(
            &mut self.dynamic_player_texture_cutout,
            &source.dynamic_player_texture_cutout,
        );
        append_textured(
            &mut self.dynamic_player_texture_cutout_cull,
            &source.dynamic_player_texture_cutout_cull,
        );
        append_textured(
            &mut self.dynamic_player_texture_solid_z_offset_forward,
            &source.dynamic_player_texture_solid_z_offset_forward,
        );
        append_textured(
            &mut self.dynamic_player_texture_cutout_z_offset,
            &source.dynamic_player_texture_cutout_z_offset,
        );
        append_textured(
            &mut self.dynamic_player_texture_armor_cutout,
            &source.dynamic_player_texture_armor_cutout,
        );
        append_textured(
            &mut self.dynamic_player_texture_armor_trim,
            &source.dynamic_player_texture_armor_trim,
        );
        append_textured(
            &mut self.dynamic_player_texture_armor_trim_decal,
            &source.dynamic_player_texture_armor_trim_decal,
        );
        append_textured(
            &mut self.dynamic_player_texture_translucent,
            &source.dynamic_player_texture_translucent,
        );
        append_textured(
            &mut self.dynamic_player_texture_item_entity_translucent,
            &source.dynamic_player_texture_item_entity_translucent,
        );
        append_textured(
            &mut self.dynamic_player_texture_item_entity_translucent_cull,
            &source.dynamic_player_texture_item_entity_translucent_cull,
        );
        append_textured(
            &mut self.dynamic_player_texture_outline,
            &source.dynamic_player_texture_outline,
        );
        append_textured(
            &mut self.dynamic_player_texture_outline_cull,
            &source.dynamic_player_texture_outline_cull,
        );
        append_mesh!(self.scroll, source.scroll, vertices.len());
        append_mesh!(self.scroll_additive, source.scroll_additive, vertices.len());
        append_mesh!(self.guardian_beam, source.guardian_beam, vertices.len());
        append_mesh!(
            self.end_crystal_beam,
            source.end_crystal_beam,
            vertices.len()
        );
        append_mesh!(
            self.beacon_beam_opaque,
            source.beacon_beam_opaque,
            vertices.len()
        );
        append_mesh!(
            self.beacon_beam_translucent,
            source.beacon_beam_translucent,
            vertices.len()
        );
        append_mesh!(self.entity_glint, source.entity_glint, vertices.len());
        append_mesh!(
            self.armor_entity_glint,
            source.armor_entity_glint,
            vertices.len()
        );
        append_mesh!(self.end_portal, source.end_portal, vertices.len());
        append_mesh!(self.end_gateway, source.end_gateway, vertices.len());
        append_mesh!(self.dragon_rays, source.dragon_rays, vertices.len());
        self.dragon_rays.opaque_faces += source.dragon_rays.opaque_faces;
        append_mesh!(
            self.dragon_rays_depth,
            source.dragon_rays_depth,
            vertices.len()
        );
        self.dragon_rays_depth.opaque_faces += source.dragon_rays_depth.opaque_faces;
        append_mesh!(self.leash, source.leash, vertices.len());
        self.leash.opaque_faces += source.leash.opaque_faces;
        self.next_submission_index += source.next_submission_index;
    }
    fn resort(&mut self, camera: Option<[f32; 3]>) {
        let camera = camera.unwrap_or([0.0; 3]);
        resort_ranges(
            &mut self.opaque_beam_draws,
            &mut self.sorted_main_translucent_draws,
            &mut self.sorted_item_entity_draws,
            camera,
        );
    }
}

pub(in crate::entity_models) fn resort_ranges(
    opaque: &mut [EntityModelDrawRange],
    main: &mut [EntityModelTranslucentDrawRange],
    items: &mut [EntityModelTexturedDrawRange],
    camera: [f32; 3],
) {
    let update = |key: &mut EntityModelSubmitSortKey| {
        key.distance_sq =
            (Vec3::from_array(key.position) - Vec3::from_array(camera)).length_squared();
    };
    for draw in items.iter_mut() {
        update(&mut draw.sort_key);
    }
    for draw in main.iter_mut() {
        match draw {
            EntityModelTranslucentDrawRange::Textured(draw) => update(&mut draw.sort_key),
            EntityModelTranslucentDrawRange::Geometry(draw) => update(&mut draw.sort_key),
        }
    }
    opaque.sort_by(|a, b| compare_submit_sort_key(a.sort_key, b.sort_key));
    items.sort_by(|a, b| compare_submit_sort_key(a.sort_key, b.sort_key));
    main.sort_by(|a, b| {
        compare_submit_sort_key(translucent_draw_sort_key(*a), translucent_draw_sort_key(*b))
    });
}

fn append_textured(dest: &mut EntityModelTexturedMesh, source: &EntityModelTexturedMesh) {
    let base = dest.vertex_count();
    append_mesh!(dest, source, vertex_count());
    dest.parts
        .extend(source.parts.iter().cloned().map(|mut part| {
            part.vertex_start += base;
            part
        }));
    dest.deferred_vertices += source.deferred_vertices;
    dest.cutout_faces += source.cutout_faces;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_models::{build_entity_model_texture_atlas, EntityModelTextureImage};

    #[test]
    fn inserting_or_removing_earlier_entity_keeps_downstream_entries_cached() {
        let images = [
            EntityModelTextureImage::new(
                END_SKY_TEXTURE_REF,
                vec![255; (END_SKY_TEXTURE_REF.size[0] * END_SKY_TEXTURE_REF.size[1] * 4) as usize],
            ),
            EntityModelTextureImage::new(
                END_PORTAL_TEXTURE_REF,
                vec![
                    255;
                    (END_PORTAL_TEXTURE_REF.size[0] * END_PORTAL_TEXTURE_REF.size[1] * 4) as usize
                ],
            ),
            EntityModelTextureImage::new(
                CREEPER_TEXTURE_REF,
                vec![255; (CREEPER_TEXTURE_REF.size[0] * CREEPER_TEXTURE_REF.size[1] * 4) as usize],
            ),
        ];
        let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
        let equipment = ResolvedEntityEquipmentAssetCatalog::default();
        let mut cache = EntityExtractionCache::default();

        let e5 = EntityModelInstance::new(5, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
        let e10 = EntityModelInstance::new(10, EntityModelKind::Creeper, [2.0, 64.0, 0.0], 0.0);
        let e20 = EntityModelInstance::new(20, EntityModelKind::Creeper, [4.0, 64.0, 0.0], 0.0);
        let portal_a = EntityModelInstance::new(
            -1,
            EntityModelKind::EndPortalBlock {
                kind: EndPortalModelKind::EndGateway,
                faces: EndPortalModelFaces::ALL,
            },
            [1.0, 64.0, 1.0],
            0.0,
        );
        let portal_b = EntityModelInstance::new(
            -1,
            EntityModelKind::EndPortalBlock {
                kind: EndPortalModelKind::EndGateway,
                faces: EndPortalModelFaces::ALL,
            },
            [2.0, 64.0, 1.0],
            0.0,
        );
        let portal_c = EntityModelInstance::new(
            -1,
            EntityModelKind::EndPortalBlock {
                kind: EndPortalModelKind::EndGateway,
                faces: EndPortalModelFaces::ALL,
            },
            [3.0, 64.0, 1.0],
            0.0,
        );

        let mut build = |instances: &[EntityModelInstance]| {
            let (meshes, _water) = cache.build(
                instances,
                &atlas,
                &[],
                &equipment,
                None,
                None,
                &[],
                None,
                None,
            );
            cache.recycle_surface_meshes(meshes.solid, meshes.cutout, meshes.cutout_cull);
            (cache.last_build_hits, cache.last_build_misses)
        };

        // Frame 1: cold insert for e10, e20, portal_b, portal_c.
        assert_eq!(build(&[e10, e20, portal_b, portal_c]), (0, 4));

        // Frame 2: steady frame populates cached geometry for all 4.
        assert_eq!(build(&[e10, e20, portal_b, portal_c]), (4, 0));

        // Frame 3: insert e5 and portal_a BEFORE the existing entries — zero downstream misses!
        assert_eq!(build(&[e5, e10, e20, portal_a, portal_b, portal_c]), (4, 2));

        // Frame 4: remove e10 and portal_a — remaining 4 entries all hit and pruned entries are swept!
        assert_eq!(build(&[e5, e20, portal_b, portal_c]), (4, 0));
        drop(build);
        assert_eq!(cache.entries.len(), 4);
        assert!(
            std::mem::size_of::<CachedEntity>() <= 128,
            "CachedEntity should stay compact instead of embedding EntityModelInstance inline: {} bytes",
            std::mem::size_of::<CachedEntity>()
        );
    }
}
