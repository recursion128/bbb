use anyhow::{bail, Context, Result};

use crate::{
    image::{decode_png_file_rgba_with_budget, MAX_RESOURCE_PNG_FILE_BYTES},
    PackResourceStack, PackRootResource, PackRoots, SelectedClientPackStack,
};

const STANDARD_ICON_SIZES: [u32; 5] = [16, 32, 48, 128, 256];
const MAX_WINDOW_ICON_ICNS_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowIconSet {
    Release,
    Snapshot,
}

impl WindowIconSet {
    pub const fn for_stable(stable: bool) -> Self {
        if stable {
            Self::Release
        } else {
            Self::Snapshot
        }
    }

    fn standard_path(self, size: u32) -> String {
        match self {
            Self::Release => format!("icons/icon_{size}x{size}.png"),
            Self::Snapshot => format!("icons/snapshot/icon_{size}x{size}.png"),
        }
    }

    fn macos_path(self) -> &'static str {
        match self {
            Self::Release => "icons/minecraft.icns",
            Self::Snapshot => "icons/snapshot/minecraft.icns",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowIconFamily {
    Standard,
    MacOs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardWindowIcon {
    requested_size: u32,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl StandardWindowIcon {
    pub fn into_native_parts(self) -> (u32, [u32; 2], Vec<u8>) {
        (self.requested_size, [self.width, self.height], self.rgba)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacOsWindowIcon {
    bytes: Vec<u8>,
}

impl MacOsWindowIcon {
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowIconCatalog {
    Standard(Vec<StandardWindowIcon>),
    MacOs(MacOsWindowIcon),
}

impl PackRoots {
    /// Loads the startup icon from the default Vanilla pack only.
    ///
    /// Vanilla passes `vanillaPackResources` rather than the selected resource manager to
    /// `Window.setIcon`, so startup and server packs cannot override or filter these root assets.
    pub fn load_window_icon_catalog(
        &self,
        set: WindowIconSet,
        family: WindowIconFamily,
    ) -> Result<WindowIconCatalog> {
        let selected = SelectedClientPackStack::from_startup_dirs(self, std::iter::empty())?;
        let resources = PackResourceStack::prepare_selected_client_packs_with_budgets(
            selected.packs(),
            self.resource_image_budget(),
            self.resource_data_budget(),
        )?;
        match family {
            WindowIconFamily::Standard => load_standard_icons(&resources, set),
            WindowIconFamily::MacOs => load_macos_icon(&resources, set),
        }
    }
}

fn load_standard_icons(
    resources: &PackResourceStack,
    set: WindowIconSet,
) -> Result<WindowIconCatalog> {
    let mut budget = resources.single_image_png_decode_budget();
    let mut icons = Vec::with_capacity(STANDARD_ICON_SIZES.len());
    for size in STANDARD_ICON_SIZES {
        let path = set.standard_path(size);
        let resource = vanilla_root_resource(resources, &path)?;
        if resource
            .integrity
            .as_ref()
            .is_some_and(|integrity| integrity.size > MAX_RESOURCE_PNG_FILE_BYTES)
        {
            bail!("window icon {path} exceeds the {MAX_RESOURCE_PNG_FILE_BYTES} byte PNG limit");
        }
        verify_root_resource(&resource, "window icon")?;
        let image = decode_png_file_rgba_with_budget(&resource.path, "window icon", &mut budget)
            .with_context(|| format!("decode Vanilla window icon {path}"))?;
        icons.push(StandardWindowIcon {
            requested_size: size,
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        });
    }
    Ok(WindowIconCatalog::Standard(icons))
}

fn load_macos_icon(resources: &PackResourceStack, set: WindowIconSet) -> Result<WindowIconCatalog> {
    let path = set.macos_path().to_string();
    let resource = vanilla_root_resource(resources, &path)?;
    if resource
        .integrity
        .as_ref()
        .is_some_and(|integrity| integrity.size > MAX_WINDOW_ICON_ICNS_BYTES)
    {
        bail!("window icon {path} exceeds the {MAX_WINDOW_ICON_ICNS_BYTES} byte ICNS limit");
    }
    verify_root_resource(&resource, "window icon ICNS")?;
    let bytes = resources.data_budget().read_optional_binary(
        &resource.path,
        MAX_WINDOW_ICON_ICNS_BYTES,
        "window icon ICNS",
    )?;
    validate_icns(&bytes).with_context(|| format!("decode Vanilla window icon {path}"))?;
    Ok(WindowIconCatalog::MacOs(MacOsWindowIcon { bytes }))
}

fn vanilla_root_resource(resources: &PackResourceStack, path: &str) -> Result<PackRootResource> {
    let mut stack = resources.root_resource_stack(path)?;
    let resource = stack
        .pop()
        .with_context(|| format!("missing Vanilla root resource {path}"))?;
    if !stack.is_empty() || resource.pack_id != "vanilla" {
        bail!("window icon root resource {path} did not resolve from the vanilla-only stack");
    }
    Ok(resource)
}

fn verify_root_resource(resource: &PackRootResource, label: &str) -> Result<()> {
    if let Some(integrity) = &resource.integrity {
        integrity.verify(&resource.path, label)?;
    }
    Ok(())
}

fn validate_icns(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 8 || bytes[..4] != *b"icns" {
        bail!("window icon is not an ICNS container");
    }
    let declared = u32::from_be_bytes(bytes[4..8].try_into().expect("fixed ICNS length slice"));
    let actual = u32::try_from(bytes.len()).context("ICNS byte length does not fit u32")?;
    if declared != actual {
        bail!("window icon ICNS declares {declared} bytes but contains {actual}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use image::{Rgba, RgbaImage};
    use serde_json::json;
    use sha1::{Digest, Sha1};

    use super::*;

    #[test]
    fn loads_release_and_snapshot_standard_paths_in_vanilla_order() {
        assert_eq!(
            STANDARD_ICON_SIZES.map(|size| WindowIconSet::Release.standard_path(size)),
            [
                "icons/icon_16x16.png",
                "icons/icon_32x32.png",
                "icons/icon_48x48.png",
                "icons/icon_128x128.png",
                "icons/icon_256x256.png",
            ]
        );
        assert_eq!(
            STANDARD_ICON_SIZES.map(|size| WindowIconSet::Snapshot.standard_path(size)),
            [
                "icons/snapshot/icon_16x16.png",
                "icons/snapshot/icon_32x32.png",
                "icons/snapshot/icon_48x48.png",
                "icons/snapshot/icon_128x128.png",
                "icons/snapshot/icon_256x256.png",
            ]
        );
        let fixture = Fixture::new("standard-order");
        fixture.write_standard_icons(WindowIconSet::Release, [1, 2, 3, 255]);
        fixture.write_standard_icons(WindowIconSet::Snapshot, [5, 7, 11, 255]);
        let roots = fixture.roots();

        for (set, color) in [
            (WindowIconSet::Release, [1, 2, 3, 255]),
            (WindowIconSet::Snapshot, [5, 7, 11, 255]),
        ] {
            let WindowIconCatalog::Standard(icons) = roots
                .load_window_icon_catalog(set, WindowIconFamily::Standard)
                .unwrap()
            else {
                panic!("expected standard window icons");
            };
            assert_eq!(
                icons.iter().map(|icon| icon.width).collect::<Vec<_>>(),
                STANDARD_ICON_SIZES
            );
            assert_eq!(
                icons.iter().map(|icon| icon.height).collect::<Vec<_>>(),
                STANDARD_ICON_SIZES
            );
            assert_eq!(
                icons
                    .iter()
                    .map(|icon| icon.requested_size)
                    .collect::<Vec<_>>(),
                STANDARD_ICON_SIZES
            );
            assert!(icons.iter().all(|icon| icon.rgba[..4] == color));
        }
    }

    #[test]
    fn standard_prepare_is_all_or_nothing_and_ignores_selected_root_overrides() {
        let fixture = Fixture::new("standard-atomic");
        fixture.write_standard_icons(WindowIconSet::Release, [1, 2, 3, 255]);
        let overlay = fixture.root.join("selected");
        write_png(
            &overlay.join("icons/icon_256x256.png"),
            3,
            2,
            [99, 98, 97, 255],
        );
        std::fs::write(
            overlay.join("pack.mcmeta"),
            br#"{"pack":{"description":"fixture","min_format":[84,0],"max_format":[84,0]},"filter":{"block":[{"namespace":"minecraft","path":".*"}]}}"#,
        )
        .unwrap();
        let selected =
            SelectedClientPackStack::from_startup_dirs(&fixture.roots(), [overlay]).unwrap();
        let roots = fixture
            .roots()
            .with_selected_client_packs(selected)
            .unwrap();
        let WindowIconCatalog::Standard(icons) = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
            .unwrap()
        else {
            panic!("expected standard window icons");
        };
        assert_eq!(icons.last().unwrap().width, 256);
        assert_eq!(icons.last().unwrap().rgba[..4], [1, 2, 3, 255]);

        std::fs::remove_file(fixture.sources.join("icons/icon_48x48.png")).unwrap();
        let error = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing Vanilla root resource icons/icon_48x48.png"));
    }

    #[test]
    fn rejects_malformed_png_and_generation_image_budget() {
        let fixture = Fixture::new("standard-budget");
        fixture.write_standard_icons(WindowIconSet::Release, [1, 2, 3, 255]);
        std::fs::write(fixture.sources.join("icons/icon_48x48.png"), b"not a png").unwrap();
        let error = format!(
            "{:#}",
            fixture
                .roots()
                .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
                .unwrap_err()
        );
        assert!(error.contains("decode Vanilla window icon icons/icon_48x48.png"));

        fixture.write_standard_icons(WindowIconSet::Release, [1, 2, 3, 255]);
        let selected =
            SelectedClientPackStack::from_startup_dirs(&fixture.roots(), std::iter::empty())
                .unwrap();
        let roots = fixture
            .roots()
            .with_client_resource_generation_limits(0, selected, 16, 1_000_000)
            .unwrap();
        let error = format!(
            "{:#}",
            roots
                .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
                .unwrap_err()
        );
        assert!(
            error.contains("resource image generation would use more than 16 RGBA bytes"),
            "{error}"
        );
    }

    #[test]
    fn validates_and_accounts_optional_icns_without_poisoning_generation() {
        let fixture = Fixture::new("icns");
        fixture.write_icns(WindowIconSet::Release, b"release");
        let roots = fixture.roots();
        let WindowIconCatalog::MacOs(icon) = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::MacOs)
            .unwrap()
        else {
            panic!("expected ICNS window icon");
        };
        assert_eq!(WindowIconSet::Release.macos_path(), "icons/minecraft.icns");
        assert_eq!(&icon.bytes[8..], b"release");
        assert_eq!(
            roots
                .resource_data_memory_observation()
                .derived_logical_bytes,
            icon.bytes.len() as u64
        );

        fixture.write_raw(WindowIconSet::Snapshot.macos_path(), b"not icns");
        let error = format!(
            "{:#}",
            roots
                .load_window_icon_catalog(WindowIconSet::Snapshot, WindowIconFamily::MacOs)
                .unwrap_err()
        );
        assert!(error.contains("not an ICNS container"), "{error}");

        fixture.write_raw(
            WindowIconSet::Snapshot.macos_path(),
            b"icns\0\0\0\x10payload",
        );
        let error = format!(
            "{:#}",
            roots
                .load_window_icon_catalog(WindowIconSet::Snapshot, WindowIconFamily::MacOs)
                .unwrap_err()
        );
        assert!(
            error.contains("declares 16 bytes but contains 15"),
            "{error}"
        );
        roots.ensure_resource_data_admitted().unwrap();
    }

    #[test]
    fn icns_budget_failure_keeps_optional_fallback_admitted() {
        let fixture = Fixture::new("icns-budget");
        fixture.write_icns(WindowIconSet::Release, b"release");
        let selected =
            SelectedClientPackStack::from_startup_dirs(&fixture.roots(), std::iter::empty())
                .unwrap();
        let roots = fixture
            .roots()
            .with_client_resource_generation_limits(0, selected, 1_000_000, 8)
            .unwrap();
        let error = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::MacOs)
            .unwrap_err()
            .to_string();
        assert!(error.contains("optional resource data would use more than 8 logical bytes"));
        roots.ensure_resource_data_admitted().unwrap();
    }

    #[test]
    fn indexed_icons_validate_integrity_and_do_not_open_unrelated_objects() {
        let fixture = Fixture::new("indexed-integrity");
        let mut objects = BTreeMap::new();
        for size in STANDARD_ICON_SIZES {
            let path = WindowIconSet::Release.standard_path(size);
            let bytes = png_bytes(size, size, [1, 2, 3, 255]);
            fixture.write_indexed_object(&path, &bytes, &mut objects);
        }
        objects.insert(
            "minecraft/sounds/unrelated.ogg".to_string(),
            json!({"hash": "0".repeat(40), "size": 123}),
        );
        fixture.write_index(objects);
        let roots = PackRoots::from_root(&fixture.root).unwrap();
        let WindowIconCatalog::Standard(icons) = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
            .unwrap()
        else {
            panic!("expected standard window icons");
        };
        assert_eq!(icons.len(), 5);

        let object = roots
            .resource_stack()
            .root_resource_stack("icons/icon_48x48.png")
            .unwrap()
            .pop()
            .unwrap();
        std::fs::write(&object.path, b"changed").unwrap();
        let error = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
            .unwrap_err()
            .to_string();
        assert!(error.contains("has size"), "{error}");

        let expected_size = usize::try_from(object.integrity.as_ref().unwrap().size).unwrap();
        std::fs::write(&object.path, vec![0_u8; expected_size]).unwrap();
        let error = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
            .unwrap_err()
            .to_string();
        assert!(error.contains("has SHA-1"), "{error}");

        std::fs::remove_file(&object.path).unwrap();
        let error = roots
            .load_window_icon_catalog(WindowIconSet::Release, WindowIconFamily::Standard)
            .unwrap_err()
            .to_string();
        assert!(error.contains("open window icon"), "{error}");
    }

    #[test]
    fn loads_all_real_local_index_30_window_icon_objects_when_available() {
        let Ok(roots) = PackRoots::discover() else {
            return;
        };
        if roots
            .vanilla_asset_index_observation()
            .map_or(true, |observation| observation.index_id != "30")
        {
            return;
        }
        let expected = STANDARD_ICON_SIZES.to_vec();
        for set in [WindowIconSet::Release, WindowIconSet::Snapshot] {
            let WindowIconCatalog::Standard(icons) = roots
                .load_window_icon_catalog(set, WindowIconFamily::Standard)
                .unwrap()
            else {
                panic!("expected standard window icons");
            };
            assert_eq!(
                icons.iter().map(|icon| icon.width).collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                icons.iter().map(|icon| icon.height).collect::<Vec<_>>(),
                expected
            );
            let WindowIconCatalog::MacOs(icon) = roots
                .load_window_icon_catalog(set, WindowIconFamily::MacOs)
                .unwrap()
            else {
                panic!("expected ICNS window icon");
            };
            assert!(icon.bytes.len() > 8);
        }
        assert_eq!(
            roots.vanilla_asset_index_observation().unwrap().index_id,
            "30"
        );
        roots.ensure_resource_data_admitted().unwrap();
    }

    struct Fixture {
        root: PathBuf,
        sources: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "bbb-window-icons-{}-{label}-{nonce}",
                std::process::id()
            ));
            let sources = root.join("sources/26.1");
            std::fs::create_dir_all(&sources).unwrap();
            Self { root, sources }
        }

        fn roots(&self) -> PackRoots {
            PackRoots::from_paths(
                self.root.clone(),
                self.sources.clone(),
                self.sources.join("assets/minecraft"),
            )
        }

        fn write_standard_icons(&self, set: WindowIconSet, color: [u8; 4]) {
            for size in STANDARD_ICON_SIZES {
                write_png(
                    &self.sources.join(set.standard_path(size)),
                    size,
                    size,
                    color,
                );
            }
        }

        fn write_icns(&self, set: WindowIconSet, payload: &[u8]) {
            let mut bytes = b"icns".to_vec();
            bytes.extend_from_slice(&u32::try_from(8 + payload.len()).unwrap().to_be_bytes());
            bytes.extend_from_slice(payload);
            self.write_raw(set.macos_path(), &bytes);
        }

        fn write_raw(&self, path: &str, bytes: &[u8]) {
            let path = self.sources.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }

        fn write_indexed_object(
            &self,
            key: &str,
            bytes: &[u8],
            objects: &mut BTreeMap<String, serde_json::Value>,
        ) {
            let hash = format!("{:x}", Sha1::digest(bytes));
            let path = self
                .root
                .join("downloads/26.1/assets/objects")
                .join(&hash[..2])
                .join(&hash);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
            objects.insert(key.to_string(), json!({"hash": hash, "size": bytes.len()}));
        }

        fn write_index(&self, objects: BTreeMap<String, serde_json::Value>) {
            let index = serde_json::to_vec(&json!({"objects": objects})).unwrap();
            let index_hash = format!("{:x}", Sha1::digest(&index));
            let downloads = self.root.join("downloads/26.1");
            let path = downloads.join("assets/indexes/30.json");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, &index).unwrap();
            std::fs::write(
                downloads.join("26.1.json"),
                serde_json::to_vec(&json!({
                    "assetIndex": {"id": "30", "sha1": index_hash, "size": index.len()}
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn write_png(path: &Path, width: u32, height: u32, color: [u8; 4]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        RgbaImage::from_pixel(width, height, Rgba(color))
            .save(path)
            .unwrap();
    }

    fn png_bytes(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        RgbaImage::from_pixel(width, height, Rgba(color))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }
}
