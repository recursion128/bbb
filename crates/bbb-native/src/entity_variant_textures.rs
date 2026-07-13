use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use bbb_pack::PackRoots;
use bbb_renderer::{DynamicEntityTextureImage, EntityDynamicTexture, Renderer};
use bbb_world::{WorldStore, CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY};

const FARM_ANIMAL_VARIANT_REGISTRIES: [&str; 3] = [
    CHICKEN_VARIANT_REGISTRY,
    COW_VARIANT_REGISTRY,
    PIG_VARIANT_REGISTRY,
];

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

/// Owns the startup-pack-backed texture state used by synchronized farm-animal presentations.
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

    /// Discovers every synchronized adult/baby asset and uploads each unique resource once. This
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
        for registry in FARM_ANIMAL_VARIANT_REGISTRIES {
            let Some(entry_count) = world
                .registry_resource_ids_in_holder_order(registry)
                .map(|entries| entries.len())
            else {
                continue;
            };
            for holder_id in 0..entry_count {
                let holder_id = i32::try_from(holder_id)
                    .expect("registry entry count must fit the protocol holder id");
                match world.synchronized_farm_animal_variant_presentation(registry, holder_id) {
                    Ok(Some(presentation)) => {
                        self.ensure_asset_with_uploader(&presentation.adult_asset_id, uploader);
                        self.ensure_asset_with_uploader(&presentation.baby_asset_id, uploader);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        if self
                            .rejected_presentations
                            .insert((registry.to_string(), holder_id))
                        {
                            tracing::warn!(
                                registry,
                                holder_id,
                                field = %error.field,
                                reason = error.reason,
                                "ignoring malformed farm-animal variant presentation"
                            );
                        }
                    }
                }
            }
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
                    "using built-in farm-animal presentation after dynamic texture failure"
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
