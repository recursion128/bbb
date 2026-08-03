use anyhow::{bail, Context, Result};

use crate::{PackResourceStack, PackRoots, ResourceLocation, SpriteImage};

const PANORAMA_BASE: &str = "minecraft:textures/gui/title/background/panorama";
const PANORAMA_OVERLAY: &str = "minecraft:textures/gui/title/background/panorama_overlay.png";

// CubeMapTexture.SUFFIXES is layer order, not numeric filename order.
const PANORAMA_FACE_SUFFIXES: [&str; 6] =
    ["_1.png", "_3.png", "_5.png", "_4.png", "_0.png", "_2.png"];

/// Immutable CPU resource generation for Vanilla's title panorama.
///
/// Resolution, filtering, companion metadata, integrity, and image admission all remain owned by
/// the selected `PackResourceStack`. The renderer only receives the completed seven-image value.
#[derive(Debug, Clone, PartialEq)]
pub struct PanoramaCatalog {
    faces: [SpriteImage; 6],
    overlay: SpriteImage,
}

impl PanoramaCatalog {
    fn load(resources: &PackResourceStack) -> Result<Self> {
        let mut faces = Vec::with_capacity(PANORAMA_FACE_SUFFIXES.len());
        for suffix in PANORAMA_FACE_SUFFIXES {
            faces.push(load_image(resources, &format!("{PANORAMA_BASE}{suffix}"))?);
        }
        let faces: [SpriteImage; 6] = faces
            .try_into()
            .expect("Vanilla panorama has exactly six face suffixes");
        let expected = [faces[0].width, faces[0].height];
        for face in &faces[1..] {
            let actual = [face.width, face.height];
            if actual != expected {
                bail!(
                    "panorama face {} is {}x{}, expected {}x{}",
                    face.id,
                    face.width,
                    face.height,
                    expected[0],
                    expected[1]
                );
            }
        }
        let overlay = load_image(resources, PANORAMA_OVERLAY)?;
        Ok(Self { faces, overlay })
    }

    pub fn faces(&self) -> &[SpriteImage; 6] {
        &self.faces
    }

    pub fn overlay(&self) -> &SpriteImage {
        &self.overlay
    }

    pub fn into_parts(self) -> ([SpriteImage; 6], SpriteImage) {
        (self.faces, self.overlay)
    }
}

impl PackRoots {
    pub fn load_panorama_catalog(&self) -> Result<PanoramaCatalog> {
        PanoramaCatalog::load(&self.resource_stack())
    }
}

fn load_image(resources: &PackResourceStack, id: &str) -> Result<SpriteImage> {
    let location = ResourceLocation::parse(id)?;
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing panorama texture {}", location.id()))?;
    SpriteImage::from_pack_resource(location.id(), &resource, resources)
        .with_context(|| format!("load panorama texture {}", location.id()))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
    use serde_json::json;
    use sha1::{Digest, Sha1};

    use super::*;
    use crate::{SelectedClientPackStack, MC_VERSION};

    const FACE_COLORS: [[u8; 4]; 6] = [
        [1, 2, 3, 255],
        [5, 7, 11, 255],
        [13, 17, 19, 255],
        [23, 29, 31, 255],
        [37, 41, 43, 255],
        [47, 53, 59, 255],
    ];

    #[test]
    fn loads_vanilla_layer_order_and_selected_override() {
        let fixture = Fixture::new("order-override");
        fixture.write_source_catalog(1, 1);
        let roots = fixture.roots();
        let catalog = roots.load_panorama_catalog().unwrap();
        assert_eq!(
            catalog
                .faces()
                .iter()
                .map(|face| face.id.clone())
                .collect::<Vec<_>>(),
            PANORAMA_FACE_SUFFIXES
                .map(|suffix| format!("{PANORAMA_BASE}{suffix}"))
                .to_vec()
        );
        assert_eq!(
            catalog
                .faces()
                .iter()
                .map(|face| face.rgba[..4].try_into().unwrap())
                .collect::<Vec<[u8; 4]>>(),
            FACE_COLORS
        );
        assert_eq!(catalog.overlay().id, PANORAMA_OVERLAY);
        assert_eq!(catalog.overlay().rgba, [61, 67, 71, 127]);

        let selected = fixture.root.join("selected");
        fixture.write_pack_image(
            &selected,
            &format!("{PANORAMA_BASE}_1.png"),
            1,
            1,
            [73, 79, 83, 255],
        );
        let catalog = roots
            .with_resource_pack_dirs([selected])
            .load_panorama_catalog()
            .unwrap();
        assert_eq!(catalog.faces()[0].rgba, [73, 79, 83, 255]);
        assert_eq!(catalog.faces()[1].rgba, FACE_COLORS[1]);
    }

    #[test]
    fn filter_missing_corrupt_and_mismatched_faces_reject_the_whole_catalog() {
        let fixture = Fixture::new("reject");
        fixture.write_source_catalog(2, 2);
        let face = fixture.source_path(&format!("{PANORAMA_BASE}_5.png"));

        std::fs::remove_file(&face).unwrap();
        let error = fixture
            .roots()
            .load_panorama_catalog()
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing panorama texture"), "{error}");

        fixture.write_source_catalog(2, 2);
        std::fs::write(&face, [0_u8; 24]).unwrap();
        let error = format!("{:#}", fixture.roots().load_panorama_catalog().unwrap_err());
        assert!(error.contains("is not a PNG"), "{error}");

        fixture.write_source_catalog(2, 2);
        fixture.write_source_image(&format!("{PANORAMA_BASE}_5.png"), 1, 2, FACE_COLORS[2]);
        let error = fixture
            .roots()
            .load_panorama_catalog()
            .unwrap_err()
            .to_string();
        assert!(error.contains("is 1x2, expected 2x2"), "{error}");

        fixture.write_source_catalog(2, 2);
        let filter = fixture.root.join("filter");
        std::fs::create_dir_all(&filter).unwrap();
        std::fs::write(
            filter.join("pack.mcmeta"),
            br#"{"filter":{"block":[{"namespace":"minecraft","path":"textures/gui/title/background/panorama_5\\.png"}]}}"#,
        )
        .unwrap();
        let error = fixture
            .roots()
            .with_resource_pack_dirs([filter])
            .load_panorama_catalog()
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing panorama texture"), "{error}");
    }

    #[test]
    fn companion_metadata_and_generation_image_budget_follow_the_resource_stack() {
        let fixture = Fixture::new("metadata-budget");
        fixture.write_source_catalog(1, 1);
        let roots = fixture.roots();
        let selected = fixture.root.join("selected");
        fixture.write_pack_image(
            &selected,
            &format!("{PANORAMA_BASE}_1.png"),
            1,
            1,
            [73, 79, 83, 255],
        );
        let metadata = fixture.pack_path(&selected, &format!("{PANORAMA_BASE}_1.png.mcmeta"));
        std::fs::write(&metadata, br#"{"texture":{"blur":true,"clamp":true}}"#).unwrap();
        let catalog = roots
            .clone()
            .with_resource_pack_dirs([selected])
            .load_panorama_catalog()
            .unwrap();
        assert!(catalog.faces()[0].texture_metadata.blur);
        assert!(catalog.faces()[0].texture_metadata.clamp);

        let selected = SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots
            .with_client_resource_generation_image_limit(7, selected, 28)
            .unwrap();
        roots.load_panorama_catalog().unwrap();
        assert_eq!(
            roots.resource_image_memory_observation().charged_rgba_bytes,
            28
        );

        let selected = SelectedClientPackStack::from_startup_dirs(&fixture.roots(), []).unwrap();
        let constrained = fixture
            .roots()
            .with_client_resource_generation_image_limit(8, selected, 27)
            .unwrap();
        let error = format!("{:#}", constrained.load_panorama_catalog().unwrap_err());
        assert!(error.contains("more than 27 RGBA bytes"), "{error}");
        assert_eq!(
            constrained
                .resource_image_memory_observation()
                .charged_rgba_bytes,
            24,
            "six admitted faces remain charged after the atomic catalog prepare rejects overlay"
        );
    }

    #[test]
    fn indexed_objects_are_lazy_but_each_consumed_object_is_integrity_checked() {
        let fixture = Fixture::new("indexed-integrity");
        fixture.write_indexed_catalog();
        let roots = fixture.roots();
        let observation = roots.vanilla_asset_index_observation().unwrap();
        assert_eq!(observation.index_id, "30");
        assert_eq!(observation.object_count, 8);
        let catalog = roots.load_panorama_catalog().unwrap();
        assert_eq!(catalog.faces()[0].width, 1);

        let stack = roots.resource_stack();
        let face = stack
            .get_resource(&ResourceLocation::parse(&format!("{PANORAMA_BASE}_1.png")).unwrap())
            .unwrap();
        let expected = face.integrity.as_ref().unwrap().size as usize;
        std::fs::write(&face.path, vec![0; expected - 1]).unwrap();
        let error = format!("{:#}", roots.load_panorama_catalog().unwrap_err());
        assert!(error.contains("has size"), "{error}");

        std::fs::write(&face.path, vec![0; expected]).unwrap();
        let error = format!("{:#}", roots.load_panorama_catalog().unwrap_err());
        assert!(error.contains("has SHA-1"), "{error}");

        std::fs::remove_file(&face.path).unwrap();
        let error = format!("{:#}", roots.load_panorama_catalog().unwrap_err());
        assert!(error.contains("open sprite image"), "{error}");
    }

    #[test]
    fn production_catalog_has_no_index_specific_resolver() {
        let production = include_str!("panorama.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(production.contains("self.resource_stack()"));
        for forbidden in [
            "indexed_vanilla_assets",
            "VanillaAssetIndex",
            "BBB_MC_ASSETS_DIR",
            "BBB_MC_GENERATED_ASSETS_DIR",
        ] {
            assert!(
                !production.contains(forbidden),
                "restored {forbidden} bypass"
            );
        }
    }

    #[test]
    #[ignore = "requires the local Vanilla 26.1 asset object store"]
    fn loads_real_index_30_panorama_through_the_default_pack() {
        let roots = PackRoots::discover().unwrap();
        let observation = roots
            .vanilla_asset_index_observation()
            .expect("local Vanilla index must be mounted");
        assert_eq!(observation.index_id, "30");
        assert_eq!(observation.object_count, 4_749);
        let catalog = roots.load_panorama_catalog().unwrap();
        assert!(catalog
            .faces()
            .iter()
            .all(|face| [face.width, face.height] == [1_024, 1_024]));
        assert_eq!([catalog.overlay().width, catalog.overlay().height], [1, 1]);
        assert_eq!(
            roots.resource_image_memory_observation().charged_rgba_bytes,
            6 * 1_024 * 1_024 * 4 + 4
        );
    }

    struct Fixture {
        root: PathBuf,
        sources: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("bbb-panorama-{label}-{nanos}"));
            let sources = root.join("sources").join(MC_VERSION);
            std::fs::create_dir_all(&sources).unwrap();
            Self { root, sources }
        }

        fn roots(&self) -> PackRoots {
            PackRoots::from_root(&self.root).unwrap()
        }

        fn source_path(&self, id: &str) -> PathBuf {
            let location = ResourceLocation::parse(id).unwrap();
            self.sources
                .join("assets")
                .join(location.namespace())
                .join(location.path())
        }

        fn write_source_catalog(&self, width: u32, height: u32) {
            for (suffix, color) in PANORAMA_FACE_SUFFIXES.into_iter().zip(FACE_COLORS) {
                self.write_source_image(&format!("{PANORAMA_BASE}{suffix}"), width, height, color);
            }
            self.write_source_image(PANORAMA_OVERLAY, 1, 1, [61, 67, 71, 127]);
        }

        fn write_source_image(&self, id: &str, width: u32, height: u32, color: [u8; 4]) {
            write_png(&self.source_path(id), width, height, color);
        }

        fn write_pack_image(&self, pack: &Path, id: &str, width: u32, height: u32, color: [u8; 4]) {
            write_png(&self.pack_path(pack, id), width, height, color);
        }

        fn pack_path(&self, pack: &Path, id: &str) -> PathBuf {
            let location = ResourceLocation::parse(id).unwrap();
            let path = pack
                .join("assets")
                .join(location.namespace())
                .join(location.path());
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            path
        }

        fn write_indexed_catalog(&self) {
            let mut objects = BTreeMap::new();
            for (suffix, color) in PANORAMA_FACE_SUFFIXES.into_iter().zip(FACE_COLORS) {
                objects.insert(
                    format!("minecraft/textures/gui/title/background/panorama{suffix}"),
                    Some(png_bytes(1, 1, color)),
                );
            }
            objects.insert(
                "minecraft/textures/gui/title/background/panorama_overlay.png".to_string(),
                Some(png_bytes(1, 1, [61, 67, 71, 127])),
            );
            objects.insert("minecraft/sounds/unrelated-missing.ogg".to_string(), None);
            write_asset_index(&self.root, objects);
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn write_png(path: &Path, width: u32, height: u32, color: [u8; 4]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, png_bytes(width, height, color)).unwrap();
    }

    fn png_bytes(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
        let rgba = color.repeat((width * height) as usize);
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&rgba, width, height, ColorType::Rgba8.into())
            .unwrap();
        bytes
    }

    fn write_asset_index(root: &Path, objects: BTreeMap<String, Option<Vec<u8>>>) {
        let mut document = serde_json::Map::new();
        for (name, bytes) in objects {
            let materialize = bytes.is_some();
            let bytes = bytes.unwrap_or_else(|| b"unmaterialized".to_vec());
            let hash = format!("{:x}", Sha1::digest(&bytes));
            document.insert(name, json!({"hash": hash, "size": bytes.len()}));
            if materialize {
                let path = root
                    .join("downloads/26.1/assets/objects")
                    .join(&hash[..2])
                    .join(&hash);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, bytes).unwrap();
            }
        }
        let index = serde_json::to_vec(&json!({"objects": document})).unwrap();
        let index_hash = format!("{:x}", Sha1::digest(&index));
        let index_path = root.join("downloads/26.1/assets/indexes/30.json");
        std::fs::create_dir_all(index_path.parent().unwrap()).unwrap();
        std::fs::write(index_path, &index).unwrap();
        std::fs::write(
            root.join("downloads/26.1/26.1.json"),
            serde_json::to_vec(&json!({
                "assetIndex": {"id": "30", "sha1": index_hash, "size": index.len()}
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
