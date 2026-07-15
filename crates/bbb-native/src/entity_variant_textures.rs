use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use bbb_pack::PackRoots;
use bbb_protocol::data_component_types::vanilla_data_component_animal_texture_variant_registries;
use bbb_renderer::{DynamicEntityTextureImage, EntityDynamicTexture, Renderer};
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
}

impl DynamicEntityTextureUploader for Renderer {
    fn upload_dynamic_entity_texture(&mut self, image: DynamicEntityTextureImage) -> Result<()> {
        Renderer::upload_dynamic_entity_texture(self, image)
    }
}

/// Owns the startup-pack-backed texture state used by synchronized animal presentations.
/// Entries never become visible to entity extraction until their image upload succeeds.
pub(crate) struct NativeEntityVariantTextureRuntime {
    pack_roots: Option<PackRoots>,
    entries: HashMap<String, EntityVariantTextureEntry>,
    rejected_presentations: HashSet<(String, i32)>,
}

impl NativeEntityVariantTextureRuntime {
    pub(crate) fn new(pack_roots: Option<PackRoots>) -> Self {
        Self {
            pack_roots,
            entries: HashMap::new(),
            rejected_presentations: HashSet::new(),
        }
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

    fn synchronize_with_uploader(
        &mut self,
        world: &WorldStore,
        uploader: &mut impl DynamicEntityTextureUploader,
    ) {
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
                    rgba: image.rgba,
                })
            });

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
        let mut handle = fnv1a64(format!("bbb:resource-texture:{asset_id}").as_bytes());
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
        COW_VARIANT_REGISTRY, VANILLA_CAT_VARIANT_DATA_ID, VANILLA_FROG_VARIANT_DATA_ID,
        VANILLA_WOLF_VARIANT_DATA_ID, VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
    };
    use uuid::Uuid;

    #[derive(Default)]
    struct TestUploader {
        images: Vec<DynamicEntityTextureImage>,
        fail_after: Option<usize>,
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
        write_png(
            &sources.join("assets/example/textures/entity/cow/baby.png"),
            2,
            1,
        );
        let roots = PackRoots {
            mc_code_root: root.clone(),
            sources_dir: sources.clone(),
            assets_dir: sources.join("assets/minecraft"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };
        let mut runtime = NativeEntityVariantTextureRuntime::new(Some(roots));
        let mut uploader = TestUploader {
            images: Vec::new(),
            fail_after: Some(1),
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
        let roots = PackRoots {
            mc_code_root: root.clone(),
            sources_dir: sources.clone(),
            assets_dir: sources.join("assets/minecraft"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };
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
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 73,
            values: vec![EntityDataValue {
                data_id: VANILLA_CAT_VARIANT_DATA_ID,
                serializer_id: 21,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CatVariant,
                    id: 0,
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
        let roots = PackRoots {
            mc_code_root: root.clone(),
            sources_dir: sources.clone(),
            assets_dir: sources.join("assets/minecraft"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };
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
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 74,
            values: vec![EntityDataValue {
                data_id: VANILLA_FROG_VARIANT_DATA_ID,
                serializer_id: 27,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::FrogVariant,
                    id: 0,
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
        let roots = PackRoots {
            mc_code_root: root.clone(),
            sources_dir: sources.clone(),
            assets_dir: sources.join("assets/minecraft"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };
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
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 76,
            values: vec![EntityDataValue {
                data_id: VANILLA_WOLF_VARIANT_DATA_ID,
                serializer_id: 25,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::WolfVariant,
                    id: 0,
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
        let roots = PackRoots {
            mc_code_root: root.clone(),
            sources_dir: sources.clone(),
            assets_dir: sources.join("assets/minecraft"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };
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
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 75,
            values: vec![EntityDataValue {
                data_id: VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
                serializer_id: 32,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::ZombieNautilusVariant,
                    id: 0,
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

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-{label}-{}-{nanos}", std::process::id()))
    }
}
