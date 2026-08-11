use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use bbb_pack::PackRoots;
use bbb_protocol::data_component_types::vanilla_data_component_animal_texture_variant_registries;
use bbb_renderer::{
    DynamicEntityTextureImage, EntityDynamicTexture, Renderer, StandaloneTextureSampling,
};
use bbb_world::{
    WorldStore, CAT_VARIANT_REGISTRY, FROG_VARIANT_REGISTRY, WOLF_VARIANT_REGISTRY,
    ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityVariantTextureStatus {
    Pending,
    Ready,
    Failed,
}

#[derive(Debug, Clone)]
struct EntityVariantTextureEntry {
    texture: EntityDynamicTexture,
    status: EntityVariantTextureStatus,
}

trait DynamicEntityTextureUploader {
    fn upload_dynamic_entity_texture(&mut self, image: DynamicEntityTextureImage) -> Result<()>;
    fn upload_dynamic_entity_textures(
        &mut self,
        images: Vec<DynamicEntityTextureImage>,
    ) -> Result<()>;
    fn finish_upload_batch(&mut self) -> Result<()>;
}

impl DynamicEntityTextureUploader for Renderer {
    fn upload_dynamic_entity_texture(&mut self, image: DynamicEntityTextureImage) -> Result<()> {
        Renderer::upload_dynamic_entity_texture(self, image)
    }

    fn upload_dynamic_entity_textures(
        &mut self,
        images: Vec<DynamicEntityTextureImage>,
    ) -> Result<()> {
        Renderer::upload_dynamic_entity_textures(self, images)
    }

    fn finish_upload_batch(&mut self) -> Result<()> {
        Renderer::finish_upload_batch(self)
    }
}

/// Owns startup-pack-backed runtime entity textures for synchronized animal presentations and the
/// generated armor-trim/banner-pattern atlases. Entries never become visible to entity extraction
/// until their image upload succeeds.
pub(crate) struct NativeEntityVariantTextureRuntime {
    pack_roots: Option<PackRoots>,
    resource_generation: u64,
    session: Option<(u64, bool)>,
    entries: HashMap<String, EntityVariantTextureEntry>,
    rejected_presentations: HashSet<(String, i32)>,
    armor_trim_atlas_synchronized: bool,
    banner_pattern_atlas_synchronized: bool,
    banner_pattern_atlas_ready: bool,
    shield_pattern_atlas_synchronized: bool,
    shield_pattern_atlas_ready: bool,
    painting_atlas_synchronized: bool,
    painting_atlas_ready: bool,
}

impl NativeEntityVariantTextureRuntime {
    pub(crate) fn new(pack_roots: Option<PackRoots>) -> Self {
        let resource_generation = pack_roots
            .as_ref()
            .and_then(PackRoots::client_resource_generation)
            .map_or(0, bbb_pack::ClientResourceGeneration::generation);
        Self {
            pack_roots,
            resource_generation,
            session: None,
            entries: HashMap::new(),
            rejected_presentations: HashSet::new(),
            armor_trim_atlas_synchronized: false,
            banner_pattern_atlas_synchronized: false,
            banner_pattern_atlas_ready: false,
            shield_pattern_atlas_synchronized: false,
            shield_pattern_atlas_ready: false,
            painting_atlas_synchronized: false,
            painting_atlas_ready: false,
        }
    }

    pub(crate) fn synchronize_session(
        &mut self,
        connection_generation: u64,
        connected: bool,
    ) -> bool {
        let next = (connection_generation, connected);
        if self.session == Some(next) {
            return false;
        }
        self.session = Some(next);
        self.entries.clear();
        self.rejected_presentations.clear();
        self.armor_trim_atlas_synchronized = false;
        self.banner_pattern_atlas_synchronized = false;
        self.banner_pattern_atlas_ready = false;
        self.shield_pattern_atlas_synchronized = false;
        self.shield_pattern_atlas_ready = false;
        self.painting_atlas_synchronized = false;
        self.painting_atlas_ready = false;
        true
    }

    /// Discovers every synchronized variant asset and uploads each unique resource once. This
    /// runs after network draining, before entity extraction, so a successful image becomes usable
    /// together with its registry model selector in the same render frame.
    pub(crate) fn synchronize(&mut self, world: &WorldStore, renderer: &mut Renderer) {
        self.synchronize_with_uploader(world, renderer);
    }

    pub(crate) fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        self.entries
            .get(asset_id)
            .filter(|entry| entry.status == EntityVariantTextureStatus::Ready)
            .map(|entry| entry.texture)
    }

    /// Resolves one `Sheets.BANNER_MAPPER` sprite from the startup banner-pattern atlas. Once the
    /// atlas itself is ready, an undeclared data-driven sprite follows vanilla's atlas lookup to
    /// `missingno`; an unavailable atlas leaves the caller on its static bootstrap texture path.
    pub(crate) fn ready_banner_pattern_texture(
        &self,
        sprite_id: &str,
    ) -> Option<EntityDynamicTexture> {
        if !self.banner_pattern_atlas_ready {
            return None;
        }
        self.ready_texture(sprite_id)
            .or_else(|| self.ready_texture("minecraft:missingno"))
    }

    /// Resolves one `Sheets.SHIELD_MAPPER` sprite from the startup
    /// shield-pattern atlas with the same missing-sprite behavior as banners.
    pub(crate) fn ready_shield_pattern_texture(
        &self,
        sprite_id: &str,
    ) -> Option<EntityDynamicTexture> {
        if !self.shield_pattern_atlas_ready {
            return None;
        }
        self.ready_texture(sprite_id)
            .or_else(|| self.ready_texture("minecraft:missingno"))
    }

    /// Resolves one `PaintingRenderer` sprite from the startup `paintings` atlas. Once the atlas is
    /// ready, an undeclared synchronized asset follows `TextureAtlas.getSprite` to `missingno`.
    pub(crate) fn ready_painting_texture(&self, sprite_id: &str) -> Option<EntityDynamicTexture> {
        if !self.painting_atlas_ready {
            return None;
        }
        self.ready_texture(sprite_id)
            .or_else(|| self.ready_texture("minecraft:missingno"))
    }

    fn synchronize_with_uploader(
        &mut self,
        world: &WorldStore,
        uploader: &mut impl DynamicEntityTextureUploader,
    ) {
        if matches!(self.session, Some((_, false))) {
            return;
        }
        self.synchronize_armor_trim_atlas(uploader);
        self.synchronize_banner_pattern_atlas(uploader);
        self.synchronize_shield_pattern_atlas(uploader);
        self.synchronize_painting_atlas(uploader);
        for (_, registry) in vanilla_data_component_animal_texture_variant_registries() {
            let Some(entry_count) = world
                .registry_resource_ids_in_holder_order(registry)
                .map(|entries| entries.len())
            else {
                continue;
            };
            for holder_id in 0..entry_count {
                let holder_id = i32::try_from(holder_id)
                    .expect("registry entry count must fit the protocol holder id");
                let presentation = match registry {
                    CAT_VARIANT_REGISTRY => world
                        .synchronized_cat_variant_presentation(holder_id)
                        .map(|presentation| {
                            presentation.map(|presentation| {
                                vec![presentation.adult_asset_id, presentation.baby_asset_id]
                            })
                        })
                        .map_err(|error| (error.field, error.reason)),
                    FROG_VARIANT_REGISTRY => world
                        .synchronized_frog_variant_presentation(holder_id)
                        .map(|presentation| {
                            presentation.map(|presentation| vec![presentation.asset_id])
                        })
                        .map_err(|error| (error.field, error.reason)),
                    WOLF_VARIANT_REGISTRY => world
                        .synchronized_wolf_variant_presentation(holder_id)
                        .map(|presentation| {
                            presentation.map(|presentation| {
                                vec![
                                    presentation.adult.wild_asset_id,
                                    presentation.adult.tame_asset_id,
                                    presentation.adult.angry_asset_id,
                                    presentation.baby.wild_asset_id,
                                    presentation.baby.tame_asset_id,
                                    presentation.baby.angry_asset_id,
                                ]
                            })
                        })
                        .map_err(|error| (error.field, error.reason)),
                    ZOMBIE_NAUTILUS_VARIANT_REGISTRY => world
                        .synchronized_zombie_nautilus_variant_presentation(holder_id)
                        .map(|presentation| {
                            presentation.map(|presentation| vec![presentation.asset_id])
                        })
                        .map_err(|error| (error.field, error.reason)),
                    _ => world
                        .synchronized_farm_animal_variant_presentation(registry, holder_id)
                        .map(|presentation| {
                            presentation.map(|presentation| {
                                vec![presentation.adult_asset_id, presentation.baby_asset_id]
                            })
                        })
                        .map_err(|error| (error.field, error.reason)),
                };
                match presentation {
                    Ok(Some(asset_ids)) => {
                        for asset_id in asset_ids {
                            self.ensure_asset_with_uploader(&asset_id, uploader);
                        }
                    }
                    Ok(None) => {}
                    Err((field, reason)) => {
                        self.record_rejected_presentation(registry, holder_id, &field, reason)
                    }
                }
            }
        }
    }

    fn synchronize_armor_trim_atlas(&mut self, uploader: &mut impl DynamicEntityTextureUploader) {
        if self.armor_trim_atlas_synchronized {
            return;
        }
        self.armor_trim_atlas_synchronized = true;
        self.synchronize_texture_atlas("armor_trims", "armor-trim", uploader);
    }

    fn synchronize_banner_pattern_atlas(
        &mut self,
        uploader: &mut impl DynamicEntityTextureUploader,
    ) {
        if self.banner_pattern_atlas_synchronized {
            return;
        }
        self.banner_pattern_atlas_synchronized = true;
        self.banner_pattern_atlas_ready =
            self.synchronize_texture_atlas("banner_patterns", "banner-pattern", uploader);
    }

    fn synchronize_shield_pattern_atlas(
        &mut self,
        uploader: &mut impl DynamicEntityTextureUploader,
    ) {
        if self.shield_pattern_atlas_synchronized {
            return;
        }
        self.shield_pattern_atlas_synchronized = true;
        self.shield_pattern_atlas_ready =
            self.synchronize_texture_atlas("shield_patterns", "shield-pattern", uploader);
    }

    fn synchronize_painting_atlas(&mut self, uploader: &mut impl DynamicEntityTextureUploader) {
        if self.painting_atlas_synchronized {
            return;
        }
        self.painting_atlas_synchronized = true;
        self.painting_atlas_ready =
            self.synchronize_texture_atlas("paintings", "painting", uploader);
    }

    fn synchronize_texture_atlas(
        &mut self,
        atlas_name: &'static str,
        description: &'static str,
        uploader: &mut impl DynamicEntityTextureUploader,
    ) -> bool {
        let images = match self
            .pack_roots
            .as_ref()
            .ok_or_else(|| anyhow!("pack roots are unavailable"))
            .and_then(|roots| roots.load_atlas_texture_images(atlas_name))
        {
            Ok(images) => images,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    atlas_name,
                    description,
                    "continuing without runtime entity atlas sprites"
                );
                return false;
            }
        };

        let mut ids = Vec::with_capacity(images.len());
        let mut uploads = Vec::with_capacity(images.len());
        for image in images {
            if self.entries.contains_key(&image.id) {
                continue;
            }
            let texture = EntityDynamicTexture {
                handle: self.unique_texture_handle(&image.id),
            };
            ids.push(image.id.clone());
            uploads.push(DynamicEntityTextureImage {
                handle: texture.handle,
                size: [image.width, image.height],
                standalone_sampling: None,
                rgba: image.rgba,
            });
            self.entries.insert(
                image.id,
                EntityVariantTextureEntry {
                    texture,
                    status: EntityVariantTextureStatus::Pending,
                },
            );
        }

        let result = uploader
            .upload_dynamic_entity_textures(uploads)
            .and_then(|()| uploader.finish_upload_batch());
        for id in &ids {
            let entry = self
                .entries
                .get_mut(id)
                .expect("pending runtime atlas texture entry must remain present");
            entry.status = if result.is_ok() {
                EntityVariantTextureStatus::Ready
            } else {
                EntityVariantTextureStatus::Failed
            };
        }
        let ready = result.is_ok();
        match result {
            Ok(()) => tracing::info!(
                textures = ids.len(),
                atlas_name,
                description,
                "loaded runtime entity atlas sprites"
            ),
            Err(error) => tracing::warn!(
                ?error,
                textures = ids.len(),
                atlas_name,
                description,
                "continuing without runtime entity atlas sprites"
            ),
        }
        ready
    }

    fn record_rejected_presentation(
        &mut self,
        registry: &str,
        holder_id: i32,
        field: &str,
        reason: &'static str,
    ) {
        if self
            .rejected_presentations
            .insert((registry.to_string(), holder_id))
        {
            tracing::warn!(
                registry,
                holder_id,
                field,
                reason,
                "ignoring malformed animal variant presentation"
            );
        }
    }

    fn ensure_asset_with_uploader(
        &mut self,
        asset_id: &str,
        uploader: &mut impl DynamicEntityTextureUploader,
    ) {
        if self.entries.contains_key(asset_id) {
            return;
        }

        let texture = EntityDynamicTexture {
            handle: self.unique_texture_handle(asset_id),
        };
        self.entries.insert(
            asset_id.to_string(),
            EntityVariantTextureEntry {
                texture,
                status: EntityVariantTextureStatus::Pending,
            },
        );

        let result = self
            .pack_roots
            .as_ref()
            .ok_or_else(|| anyhow!("pack roots are unavailable"))
            .and_then(|roots| roots.load_resource_texture_image(asset_id))
            .and_then(|image| {
                uploader.upload_dynamic_entity_texture(DynamicEntityTextureImage {
                    handle: texture.handle,
                    size: image.size,
                    standalone_sampling: Some(StandaloneTextureSampling::new(
                        image.texture_metadata.blur,
                        image.texture_metadata.clamp,
                    )),
                    rgba: image.rgba,
                })
            })
            .and_then(|()| uploader.finish_upload_batch());

        let entry = self
            .entries
            .get_mut(asset_id)
            .expect("pending entity variant texture entry must remain present");
        match result {
            Ok(()) => entry.status = EntityVariantTextureStatus::Ready,
            Err(error) => {
                entry.status = EntityVariantTextureStatus::Failed;
                tracing::warn!(
                    asset_id,
                    handle = texture.handle,
                    ?error,
                    "using built-in animal presentation after dynamic texture failure"
                );
            }
        }
    }

    fn unique_texture_handle(&self, asset_id: &str) -> u64 {
        // Keep resource-backed handles stable within and across runs while tagging the hashed input
        // away from raw profile URLs. Resolve the extremely unlikely in-cache collision by probing.
        let (connection_generation, connected) = self.session.unwrap_or((0, false));
        let mut handle = fnv1a64(
            format!(
                "bbb:resource-texture:{}:{connection_generation}:{connected}:{asset_id}",
                self.resource_generation
            )
            .as_bytes(),
        );
        while self.entries.iter().any(|(existing_asset, entry)| {
            existing_asset != asset_id && entry.texture.handle == handle
        }) {
            handle = handle.wrapping_add(0x9e37_79b9_7f4a_7c15);
        }
        handle
    }

    #[cfg(test)]
    fn status(&self, asset_id: &str) -> Option<EntityVariantTextureStatus> {
        self.entries.get(asset_id).map(|entry| entry.status)
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use bbb_protocol::{
        entity_data_serializers::{
            VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FROG_VARIANT_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_WOLF_VARIANT_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_ZOMBIE_NAUTILUS_VARIANT_ID,
        },
        entity_types::{
            VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        },
        packets::{
            AddEntity, EntityDataRegistryHolder, EntityDataValue, EntityDataValueKind,
            SetEntityData, Vec3d,
        },
    };
    use bbb_renderer::{EntityModelKind, ZombieNautilusModelVariant};
    use bbb_world::{
        vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
        vanilla_entity_metadata_frog_variant_accessor_for_entity_type,
        vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type,
        COW_VARIANT_REGISTRY,
    };
    use uuid::Uuid;

    #[derive(Default)]
    struct TestUploader {
        images: Vec<DynamicEntityTextureImage>,
        fail_after: Option<usize>,
        batch_calls: usize,
        finish_calls: usize,
        fail_finish: bool,
    }

    impl DynamicEntityTextureUploader for TestUploader {
        fn upload_dynamic_entity_texture(
            &mut self,
            image: DynamicEntityTextureImage,
        ) -> Result<()> {
            self.images.push(image);
            if self
                .fail_after
                .is_some_and(|successful_uploads| self.images.len() > successful_uploads)
            {
                Err(anyhow!("test upload failure"))
            } else {
                Ok(())
            }
        }

        fn upload_dynamic_entity_textures(
            &mut self,
            images: Vec<DynamicEntityTextureImage>,
        ) -> Result<()> {
            self.batch_calls += 1;
            for image in images {
                self.upload_dynamic_entity_texture(image)?;
            }
            Ok(())
        }

        fn finish_upload_batch(&mut self) -> Result<()> {
            self.finish_calls += 1;
            if self.fail_finish {
                Err(anyhow!("test upload completion failure"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn texture_entries_become_ready_only_after_upload_and_fail_without_retry() {
        let root = unique_temp_dir("entity-variant-texture-runtime");
        let sources = root.join("sources/26.1");
        write_png(
            &sources.join("assets/example/textures/entity/cow/adult.png"),
            3,
            2,
        );
        std::fs::write(
            sources.join("assets/example/textures/entity/cow/adult.png.mcmeta"),
            r#"{"texture":{"blur":true,"clamp":true}}"#,
        )
        .unwrap();
        write_png(
            &sources.join("assets/example/textures/entity/cow/baby.png"),
            2,
            1,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader {
            images: Vec::new(),
            fail_after: Some(1),
            batch_calls: 0,
            finish_calls: 0,
            fail_finish: false,
        };
        let mut world = WorldStore::new();
        world.record_registry_entries(
            COW_VARIANT_REGISTRY,
            0,
            vec![bbb_world::RegistryPacketEntry::with_raw_data(
                "example:highland",
                farm_animal_variant_nbt(
                    "cold",
                    "example:entity/cow/adult",
                    "example:entity/cow/baby",
                ),
            )],
        );

        runtime.synchronize_with_uploader(&world, &mut uploader);
        let ready = runtime
            .ready_texture("example:entity/cow/adult")
            .expect("successful upload must publish the handle");
        assert_eq!(
            runtime.status("example:entity/cow/adult"),
            Some(EntityVariantTextureStatus::Ready)
        );
        assert_eq!(uploader.images.len(), 2);
        assert_eq!(uploader.images[0].handle, ready.handle);
        assert_eq!(uploader.images[0].size, [3, 2]);
        assert_eq!(
            uploader.images[0].standalone_sampling,
            Some(StandaloneTextureSampling::new(true, true))
        );
        assert_eq!(
            uploader.images[1].standalone_sampling,
            Some(StandaloneTextureSampling::default())
        );
        assert_eq!(
            runtime.status("example:entity/cow/baby"),
            Some(EntityVariantTextureStatus::Failed)
        );
        assert_eq!(runtime.ready_texture("example:entity/cow/baby"), None);
        runtime.synchronize_with_uploader(&world, &mut uploader);
        assert_eq!(
            uploader.images.len(),
            2,
            "ready and failed assets must both stay deduplicated across frames"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn session_edges_rebuild_pack_backed_texture_owners_without_same_session_churn() {
        let root = unique_temp_dir("entity-variant-texture-session");
        let sources = root.join("sources/26.1");
        write_png(
            &sources.join("assets/example/textures/entity/cow/adult.png"),
            3,
            2,
        );
        write_png(
            &sources.join("assets/example/textures/entity/cow/baby.png"),
            2,
            1,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let mut world = WorldStore::new();
        world.record_registry_entries(
            COW_VARIANT_REGISTRY,
            0,
            vec![bbb_world::RegistryPacketEntry::with_raw_data(
                "example:highland",
                farm_animal_variant_nbt(
                    "cold",
                    "example:entity/cow/adult",
                    "example:entity/cow/baby",
                ),
            )],
        );

        assert!(runtime.synchronize_session(4, true));
        runtime.synchronize_with_uploader(&world, &mut uploader);
        let first_handle = runtime
            .ready_texture("example:entity/cow/adult")
            .unwrap()
            .handle;
        assert_eq!(uploader.images.len(), 2);

        assert!(!runtime.synchronize_session(4, true));
        runtime.synchronize_with_uploader(&world, &mut uploader);
        assert_eq!(uploader.images.len(), 2);

        assert!(runtime.synchronize_session(4, false));
        assert!(runtime.ready_texture("example:entity/cow/adult").is_none());
        runtime.synchronize_with_uploader(&world, &mut uploader);
        assert_eq!(uploader.images.len(), 2);

        assert!(runtime.synchronize_session(5, true));
        runtime.synchronize_with_uploader(&world, &mut uploader);
        assert_eq!(uploader.images.len(), 4);
        assert_ne!(
            runtime
                .ready_texture("example:entity/cow/adult")
                .unwrap()
                .handle,
            first_handle
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn armor_trim_atlas_uploads_as_one_batch_and_stays_deduplicated() {
        let root = unique_temp_dir("armor-trim-texture-runtime");
        let sources = root.join("sources/26.1");
        let atlas_path = sources.join("assets/minecraft/atlases/armor_trims.json");
        std::fs::create_dir_all(atlas_path.parent().unwrap()).unwrap();
        std::fs::write(
            &atlas_path,
            r#"{"sources":[{"type":"minecraft:single","resource":"example:trims/entity/humanoid/spiral_ruby"}]}"#,
        )
        .unwrap();
        write_png(
            &sources.join("assets/example/textures/trims/entity/humanoid/spiral_ruby.png"),
            64,
            32,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let world = WorldStore::new();

        runtime.synchronize_with_uploader(&world, &mut uploader);

        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.finish_calls, 1);
        assert_eq!(uploader.images.len(), 2, "missingno plus the trim sprite");
        assert_eq!(uploader.images[1].size, [64, 32]);
        let trim = runtime
            .ready_texture("example:trims/entity/humanoid/spiral_ruby")
            .expect("successful batch publishes the trim texture handle");
        assert_eq!(uploader.images[1].handle, trim.handle);

        runtime.synchronize_with_uploader(&world, &mut uploader);
        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.finish_calls, 1);
        assert_eq!(uploader.images.len(), 2);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atlas_entries_stay_failed_when_upload_completion_fails() {
        let root = unique_temp_dir("entity-atlas-upload-completion");
        let sources = root.join("sources/26.1");
        let atlas_path = sources.join("assets/minecraft/atlases/banner_patterns.json");
        std::fs::create_dir_all(atlas_path.parent().unwrap()).unwrap();
        std::fs::write(
            &atlas_path,
            r#"{"sources":[{"type":"minecraft:single","resource":"example:entity/banner/custom"}]}"#,
        )
        .unwrap();
        write_png(
            &sources.join("assets/example/textures/entity/banner/custom.png"),
            16,
            16,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader {
            fail_finish: true,
            ..TestUploader::default()
        };

        runtime.synchronize_with_uploader(&WorldStore::new(), &mut uploader);

        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.finish_calls, 1);
        assert_eq!(
            runtime.status("example:entity/banner/custom"),
            Some(EntityVariantTextureStatus::Failed)
        );
        assert!(runtime
            .ready_banner_pattern_texture("example:entity/banner/custom")
            .is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn banner_pattern_atlas_uploads_custom_sprites_and_uses_missingno() {
        let root = unique_temp_dir("banner-pattern-texture-runtime");
        let sources = root.join("sources/26.1");
        let atlas_path = sources.join("assets/minecraft/atlases/banner_patterns.json");
        std::fs::create_dir_all(atlas_path.parent().unwrap()).unwrap();
        std::fs::write(
            &atlas_path,
            r#"{"sources":[{"type":"minecraft:single","resource":"example:entity/banner/custom"}]}"#,
        )
        .unwrap();
        write_png(
            &sources.join("assets/example/textures/entity/banner/custom.png"),
            64,
            64,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let world = WorldStore::new();

        runtime.synchronize_with_uploader(&world, &mut uploader);

        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.images.len(), 2, "missingno plus the custom sprite");
        let custom = runtime
            .ready_banner_pattern_texture("example:entity/banner/custom")
            .expect("successful batch publishes the custom banner sprite");
        assert_eq!(uploader.images[1].handle, custom.handle);
        assert_eq!(uploader.images[1].size, [64, 64]);
        let missing = runtime
            .ready_banner_pattern_texture("example:entity/banner/undeclared")
            .expect("an undeclared sprite resolves through the atlas missingno entry");
        assert_eq!(uploader.images[0].handle, missing.handle);

        runtime.synchronize_with_uploader(&world, &mut uploader);
        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.images.len(), 2);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shield_pattern_atlas_uploads_custom_sprites_and_uses_missingno() {
        let root = unique_temp_dir("shield-pattern-texture-runtime");
        let sources = root.join("sources/26.1");
        let atlas_path = sources.join("assets/minecraft/atlases/shield_patterns.json");
        std::fs::create_dir_all(atlas_path.parent().unwrap()).unwrap();
        std::fs::write(
            &atlas_path,
            r#"{"sources":[{"type":"minecraft:single","resource":"example:entity/shield/custom"}]}"#,
        )
        .unwrap();
        write_png(
            &sources.join("assets/example/textures/entity/shield/custom.png"),
            64,
            64,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let world = WorldStore::new();

        runtime.synchronize_with_uploader(&world, &mut uploader);

        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.images.len(), 2, "missingno plus the custom sprite");
        let custom = runtime
            .ready_shield_pattern_texture("example:entity/shield/custom")
            .expect("successful batch publishes the custom shield sprite");
        assert_eq!(uploader.images[1].handle, custom.handle);
        assert_eq!(uploader.images[1].size, [64, 64]);
        let missing = runtime
            .ready_shield_pattern_texture("example:entity/shield/undeclared")
            .expect("an undeclared sprite resolves through the atlas missingno entry");
        assert_eq!(uploader.images[0].handle, missing.handle);

        runtime.synchronize_with_uploader(&world, &mut uploader);
        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.images.len(), 2);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn painting_atlas_uploads_front_back_and_missingno_sprites() {
        let root = unique_temp_dir("painting-texture-runtime");
        let sources = root.join("sources/26.1");
        let atlas_path = sources.join("assets/minecraft/atlases/paintings.json");
        std::fs::create_dir_all(atlas_path.parent().unwrap()).unwrap();
        std::fs::write(
            &atlas_path,
            r#"{"sources":[{"type":"minecraft:directory","source":"painting","prefix":""}]}"#,
        )
        .unwrap();
        write_png(
            &sources.join("assets/example/textures/painting/gallery/wide.png"),
            64,
            32,
        );
        write_png(
            &sources.join("assets/minecraft/textures/painting/back.png"),
            16,
            16,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();

        runtime.synchronize_with_uploader(&WorldStore::new(), &mut uploader);

        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.images.len(), 3);
        assert_eq!(
            runtime
                .ready_painting_texture("example:gallery/wide")
                .map(|texture| texture.handle),
            runtime
                .ready_texture("example:gallery/wide")
                .map(|texture| texture.handle)
        );
        assert!(runtime.ready_painting_texture("minecraft:back").is_some());
        assert_eq!(
            runtime.ready_painting_texture("example:undeclared"),
            runtime.ready_texture("minecraft:missingno")
        );

        runtime.synchronize_with_uploader(&WorldStore::new(), &mut uploader);
        assert_eq!(uploader.batch_calls, 1);
        assert_eq!(uploader.images.len(), 3);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn synchronized_cat_assets_are_discovered_in_server_holder_order() {
        let root = unique_temp_dir("cat-variant-texture-runtime");
        let sources = root.join("sources/26.1");
        write_png(
            &sources.join("assets/example/textures/entity/cat/lynx.png"),
            64,
            32,
        );
        write_png(
            &sources.join("assets/example/textures/entity/cat/lynx_baby.png"),
            32,
            32,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let mut world = WorldStore::new();
        world.record_registry_entries(
            CAT_VARIANT_REGISTRY,
            0,
            vec![bbb_world::RegistryPacketEntry::with_raw_data(
                "example:lynx",
                cat_variant_nbt("example:entity/cat/lynx", "example:entity/cat/lynx_baby"),
            )],
        );

        runtime.synchronize_with_uploader(&world, &mut uploader);

        assert_eq!(uploader.images.len(), 2);
        assert_eq!(uploader.images[0].size, [64, 32]);
        assert_eq!(uploader.images[1].size, [32, 32]);
        assert!(runtime.ready_texture("example:entity/cat/lynx").is_some());
        assert!(runtime
            .ready_texture("example:entity/cat/lynx_baby")
            .is_some());

        world.apply_add_entity(AddEntity {
            id: 73,
            uuid: Uuid::from_u128(73),
            entity_type_id: VANILLA_ENTITY_TYPE_CAT_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_CAT_ID,
        )
        .expect("generated Cat variant metadata accessors");
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 73,
            values: vec![EntityDataValue {
                data_id: accessors.variant_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CatVariant,
                    value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(0),
                },
            }],
        }));
        let instances =
            crate::entity_scene::entity_model_instances_from_world_with_variant_textures_at_partial_tick(
                &world,
                None,
                &runtime,
                1.0,
            );
        assert_eq!(instances.len(), 1);
        assert_eq!(
            instances[0].dynamic_texture,
            runtime.ready_texture("example:entity/cat/lynx")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn synchronized_frog_asset_reaches_the_entity_instance() {
        let root = unique_temp_dir("frog-variant-texture-runtime");
        let sources = root.join("sources/26.1");
        write_png(
            &sources.join("assets/example/textures/entity/frog/blue.png"),
            48,
            48,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let mut world = WorldStore::new();
        world.record_registry_entries(
            FROG_VARIANT_REGISTRY,
            0,
            vec![bbb_world::RegistryPacketEntry::with_raw_data(
                "example:blue",
                frog_variant_nbt("example:entity/frog/blue"),
            )],
        );

        runtime.synchronize_with_uploader(&world, &mut uploader);

        assert_eq!(uploader.images.len(), 1);
        assert_eq!(uploader.images[0].size, [48, 48]);
        let texture = runtime
            .ready_texture("example:entity/frog/blue")
            .expect("successful upload must publish the frog texture");
        world.apply_add_entity(AddEntity {
            id: 74,
            uuid: Uuid::from_u128(74),
            entity_type_id: VANILLA_ENTITY_TYPE_FROG_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let (frog_variant_data_id, _) =
            vanilla_entity_metadata_frog_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_FROG_ID,
            )
            .expect("generated Frog variant metadata accessor must resolve");
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 74,
            values: vec![EntityDataValue {
                data_id: frog_variant_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FROG_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::FrogVariant,
                    value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(0),
                },
            }],
        }));
        let instances =
            crate::entity_scene::entity_model_instances_from_world_with_variant_textures_at_partial_tick(
                &world,
                None,
                &runtime,
                1.0,
            );
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].dynamic_texture, Some(texture));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn synchronized_wolf_assets_upload_and_wild_adult_reaches_the_entity_instance() {
        let root = unique_temp_dir("wolf-variant-texture-runtime");
        let sources = root.join("sources/26.1");
        let assets = [
            "entity/wolf/adult_wild",
            "entity/wolf/adult_tame",
            "entity/wolf/adult_angry",
            "entity/wolf/baby_wild",
            "entity/wolf/baby_tame",
            "entity/wolf/baby_angry",
        ];
        for (index, asset) in assets.iter().enumerate() {
            write_png(
                &sources.join(format!("assets/example/textures/{asset}.png")),
                32 + index as u32,
                16,
            );
        }
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let mut world = WorldStore::new();
        world.record_registry_entries(
            WOLF_VARIANT_REGISTRY,
            0,
            vec![bbb_world::RegistryPacketEntry::with_raw_data(
                "example:dire",
                wolf_variant_nbt(
                    [
                        "example:entity/wolf/adult_wild",
                        "example:entity/wolf/adult_tame",
                        "example:entity/wolf/adult_angry",
                    ],
                    [
                        "example:entity/wolf/baby_wild",
                        "example:entity/wolf/baby_tame",
                        "example:entity/wolf/baby_angry",
                    ],
                ),
            )],
        );

        runtime.synchronize_with_uploader(&world, &mut uploader);

        assert_eq!(uploader.images.len(), 6);
        assert_eq!(uploader.images[0].size, [32, 16]);
        assert_eq!(uploader.images[5].size, [37, 16]);
        for asset in assets {
            assert_eq!(
                runtime.status(&format!("example:{asset}")),
                Some(EntityVariantTextureStatus::Ready)
            );
        }
        let texture = runtime
            .ready_texture("example:entity/wolf/adult_wild")
            .expect("successful upload must publish the adult wild wolf texture");
        world.apply_add_entity(AddEntity {
            id: 76,
            uuid: Uuid::from_u128(76),
            entity_type_id: VANILLA_ENTITY_TYPE_WOLF_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_WOLF_ID,
        )
        .expect("generated Wolf variant metadata accessors");
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 76,
            values: vec![EntityDataValue {
                data_id: accessors.variant_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_WOLF_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::WolfVariant,
                    value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(0),
                },
            }],
        }));
        let instances =
            crate::entity_scene::entity_model_instances_from_world_with_variant_textures_at_partial_tick(
                &world,
                None,
                &runtime,
                1.0,
            );
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].dynamic_texture, Some(texture));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn synchronized_zombie_nautilus_model_and_texture_reach_the_entity_instance_atomically() {
        let root = unique_temp_dir("zombie-nautilus-variant-texture-runtime");
        let sources = root.join("sources/26.1");
        write_png(
            &sources.join("assets/example/textures/entity/nautilus/coral.png"),
            128,
            64,
        );
        let roots = fixture_pack_roots(&root, &sources);
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader::default();
        let mut world = WorldStore::new();
        world.record_registry_entries(
            ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
            0,
            vec![bbb_world::RegistryPacketEntry::with_raw_data(
                "example:coral",
                zombie_nautilus_variant_nbt("warm", "example:entity/nautilus/coral"),
            )],
        );

        runtime.synchronize_with_uploader(&world, &mut uploader);

        assert_eq!(uploader.images.len(), 1);
        let texture = runtime
            .ready_texture("example:entity/nautilus/coral")
            .expect("successful upload must publish the zombie-nautilus texture");
        world.apply_add_entity(AddEntity {
            id: 75,
            uuid: Uuid::from_u128(75),
            entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let (zombie_nautilus_variant_data_id, _) =
            vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            )
            .expect("generated ZombieNautilus variant metadata accessor must resolve");
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 75,
            values: vec![EntityDataValue {
                data_id: zombie_nautilus_variant_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ZOMBIE_NAUTILUS_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::ZombieNautilusVariant,
                    value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(0),
                },
            }],
        }));
        let instances =
            crate::entity_scene::entity_model_instances_from_world_with_variant_textures_at_partial_tick(
                &world,
                None,
                &runtime,
                1.0,
            );
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].dynamic_texture, Some(texture));
        assert_eq!(
            instances[0].kind,
            EntityModelKind::ZombieNautilus {
                variant: ZombieNautilusModelVariant::Warm,
            }
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    fn write_png(path: &Path, width: u32, height: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(width, height, image::Rgba([7, 11, 13, 255]))
            .save(path)
            .unwrap();
    }

    fn farm_animal_variant_nbt(model: &str, adult_asset_id: &str, baby_asset_id: &str) -> Vec<u8> {
        let mut payload = vec![10];
        write_nbt_string_tag(&mut payload, "model", model);
        write_nbt_string_tag(&mut payload, "asset_id", adult_asset_id);
        write_nbt_string_tag(&mut payload, "baby_asset_id", baby_asset_id);
        payload.push(0);
        payload
    }

    fn cat_variant_nbt(adult_asset_id: &str, baby_asset_id: &str) -> Vec<u8> {
        let mut payload = vec![10];
        write_nbt_string_tag(&mut payload, "asset_id", adult_asset_id);
        write_nbt_string_tag(&mut payload, "baby_asset_id", baby_asset_id);
        payload.push(0);
        payload
    }

    fn frog_variant_nbt(asset_id: &str) -> Vec<u8> {
        let mut payload = vec![10];
        write_nbt_string_tag(&mut payload, "asset_id", asset_id);
        payload.push(0);
        payload
    }

    fn zombie_nautilus_variant_nbt(model: &str, asset_id: &str) -> Vec<u8> {
        let mut payload = vec![10];
        write_nbt_string_tag(&mut payload, "model", model);
        write_nbt_string_tag(&mut payload, "asset_id", asset_id);
        payload.push(0);
        payload
    }

    fn wolf_variant_nbt(adult: [&str; 3], baby: [&str; 3]) -> Vec<u8> {
        let mut payload = vec![10];
        write_wolf_asset_compound(&mut payload, "assets", adult);
        write_wolf_asset_compound(&mut payload, "baby_assets", baby);
        payload.push(0);
        payload
    }

    fn write_wolf_asset_compound(payload: &mut Vec<u8>, name: &str, assets: [&str; 3]) {
        payload.push(10);
        write_nbt_string(payload, name);
        write_nbt_string_tag(payload, "wild", assets[0]);
        write_nbt_string_tag(payload, "tame", assets[1]);
        write_nbt_string_tag(payload, "angry", assets[2]);
        payload.push(0);
    }

    fn write_nbt_string_tag(payload: &mut Vec<u8>, name: &str, value: &str) {
        payload.push(8);
        write_nbt_string(payload, name);
        write_nbt_string(payload, value);
    }

    fn write_nbt_string(payload: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        payload.extend((bytes.len() as u16).to_be_bytes());
        payload.extend(bytes);
    }

    fn fixture_pack_roots(root: &Path, pack: &Path) -> PackRoots {
        let vanilla = root.join("empty-vanilla");
        PackRoots::from_paths(
            root.to_path_buf(),
            &vanilla,
            vanilla.join("assets/minecraft"),
        )
        .with_resource_pack_dirs([pack.to_path_buf()])
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-{label}-{}-{nanos}", std::process::id()))
    }
}
