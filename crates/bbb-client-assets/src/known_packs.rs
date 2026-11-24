use std::{collections::BTreeMap, fmt, path::Path, sync::Arc};

use anyhow::{bail, Context, Result};
use bbb_protocol::{
    packets::KnownPack, registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES,
};
use serde::Deserialize;

use crate::{resource_budget::ResourceListingAdmission, PackResourceStack, PackRoots, MC_VERSION};

const VANILLA_NAMESPACE: &str = "minecraft";
const CORE_PACK_ID: &str = "core";

#[derive(Clone)]
pub struct KnownPacksManager {
    available: Vec<KnownPackResources>,
}

#[derive(Debug, Clone)]
struct KnownPackResources {
    info: KnownPack,
    registry_elements: BTreeMap<(String, String), Arc<[u8]>>,
}

/// Immutable SERVER_DATA view assembled in the same order as the selected
/// known packs. Later packs override earlier resources, matching vanilla's
/// `MultiPackResourceManager` precedence.
#[derive(Clone, Default)]
pub struct KnownPackDataSource {
    selected: Vec<KnownPack>,
    registry_elements: BTreeMap<(String, String), Arc<[u8]>>,
}

impl fmt::Debug for KnownPacksManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KnownPacksManager")
            .field(
                "available_packs",
                &self
                    .available
                    .iter()
                    .map(|pack| &pack.info)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl fmt::Debug for KnownPackDataSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KnownPackDataSource")
            .field("selected", &self.selected)
            .field("registry_element_count", &self.registry_elements.len())
            .finish()
    }
}

impl KnownPacksManager {
    /// Loads the local vanilla core pack and every built-in feature pack
    /// packaged below `data/minecraft/datapacks`.
    ///
    /// The core pack is advertised only after all 26.1 synchronized registry
    /// directories are present and every discovered element is valid JSON.
    pub fn load(roots: &PackRoots) -> Result<Self> {
        if roots.vanilla_archive().is_some()
            || crate::virtual_store::is_virtual_path(&roots.sources_dir)
        {
            return Self::load_from_resource_stack(roots);
        }
        validate_source_version(&roots.sources_dir)?;

        let data_budget = roots.resource_data_budget();
        let core_elements = load_registry_elements(&roots.sources_dir, data_budget.clone())?;
        for registry in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
            if !core_elements
                .keys()
                .any(|(candidate, _)| candidate == registry.resource_id)
            {
                bail!(
                    "vanilla core pack has no elements for synchronized registry {}",
                    registry.resource_id
                );
            }
        }

        let mut available = vec![KnownPackResources {
            info: vanilla_known_pack(CORE_PACK_ID),
            registry_elements: core_elements,
        }];

        let built_in_root = roots
            .sources_dir
            .join("data")
            .join(VANILLA_NAMESPACE)
            .join("datapacks");
        if built_in_root.is_dir() {
            let mut admission = ResourceListingAdmission::new(data_budget.clone());
            let mut entries = std::fs::read_dir(&built_in_root)
                .with_context(|| format!("read built-in datapacks {}", built_in_root.display()))?
                .collect::<std::io::Result<Vec<_>>>()?;
            entries.sort_by_key(std::fs::DirEntry::file_name);
            for entry in entries {
                let path = entry.path();
                admission.admit(&path.to_string_lossy(), "built-in known packs")?;
                if !entry
                    .file_type()
                    .with_context(|| format!("read file type {}", path.display()))?
                    .is_dir()
                    || !path.join("pack.mcmeta").is_file()
                {
                    continue;
                }
                let id = entry
                    .file_name()
                    .into_string()
                    .map_err(|name| anyhow::anyhow!("non-utf8 built-in known-pack id {name:?}"))?;
                crate::resources::validate_resource_path(&id)
                    .with_context(|| format!("invalid built-in known-pack id {id:?}"))?;
                validate_pack_metadata(&path, &data_budget)?;
                available.push(KnownPackResources {
                    info: vanilla_known_pack(&id),
                    registry_elements: load_registry_elements(&path, data_budget.clone())?,
                });
            }
        }

        Ok(Self { available })
    }

    // Vanilla `KnownPacksManager.java:18` uses `ServerPacksSource.createVanillaTrustedRepository()`,
    // so only the trusted vanilla pack (never user/server resource packs) supplies known-pack data.
    fn load_from_resource_stack(roots: &PackRoots) -> Result<Self> {
        let vanilla_pack = crate::SelectedClientPack::vanilla(roots);
        let stack = PackResourceStack::prepare_selected_client_packs_with_budgets(
            &[vanilla_pack],
            roots.resource_image_budget(),
            roots.resource_data_budget(),
        )?;
        let version_resource = stack
            .root_resource_stack("version.json")?
            .into_iter()
            .last()
            .context("vanilla web bundle is missing version.json")?;
        let version: SourceVersion =
            stack.read_json_root_resource(&version_resource, "vanilla version metadata")?;
        if version.id != MC_VERSION || version.protocol_version != bbb_protocol::PROTOCOL_VERSION {
            bail!(
                "vanilla source version mismatch: expected {} protocol {}, found {} protocol {}",
                MC_VERSION,
                bbb_protocol::PROTOCOL_VERSION,
                version.id,
                version.protocol_version
            );
        }

        let core_elements = load_registry_elements_from_stack(&stack)?;
        for registry in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
            if !core_elements
                .keys()
                .any(|(candidate, _)| candidate == registry.resource_id)
            {
                bail!(
                    "vanilla core pack has no elements for synchronized registry {}",
                    registry.resource_id
                );
            }
        }

        let mut available = vec![KnownPackResources {
            info: vanilla_known_pack(CORE_PACK_ID),
            registry_elements: core_elements,
        }];

        for metadata_res in stack.list_data_resources("datapacks", "pack.mcmeta")? {
            if metadata_res.location.namespace() != VANILLA_NAMESPACE {
                continue;
            }
            let Some(id) = metadata_res
                .location
                .path()
                .strip_prefix("datapacks/")
                .and_then(|rest| rest.strip_suffix("/pack.mcmeta"))
            else {
                continue;
            };
            if id.contains('/') {
                continue;
            }
            crate::resources::validate_resource_path(id)
                .with_context(|| format!("invalid built-in known-pack id {id:?}"))?;
            let _: serde_json::Value =
                stack.read_json_resource(&metadata_res, "built-in pack metadata")?;
            let registry_elements = load_builtin_datapack_elements_from_stack(&stack, id)?;
            available.push(KnownPackResources {
                info: vanilla_known_pack(id),
                registry_elements,
            });
        }

        Ok(Self { available })
    }

    pub fn available_packs(&self) -> impl ExactSizeIterator<Item = &KnownPack> {
        self.available.iter().map(|pack| &pack.info)
    }

    /// Returns the exact local intersection in server offer order. Namespace,
    /// id, and version must all match; no compatible-version substitution is
    /// attempted.
    pub fn select_offered(&self, offered: &[KnownPack]) -> Vec<KnownPack> {
        offered
            .iter()
            .filter(|offered| self.available.iter().any(|pack| pack.info == **offered))
            .cloned()
            .collect()
    }

    pub fn data_source_for(&self, selected: &[KnownPack]) -> Result<KnownPackDataSource> {
        let mut registry_elements = BTreeMap::new();
        for selected_pack in selected {
            let Some(pack) = self
                .available
                .iter()
                .find(|pack| pack.info == *selected_pack)
            else {
                bail!(
                    "selected unavailable known pack {}:{}:{}",
                    selected_pack.namespace,
                    selected_pack.id,
                    selected_pack.version
                );
            };
            registry_elements.extend(
                pack.registry_elements
                    .iter()
                    .map(|(key, value)| (key.clone(), Arc::clone(value))),
            );
        }
        Ok(KnownPackDataSource {
            selected: selected.to_vec(),
            registry_elements,
        })
    }
}

impl KnownPackDataSource {
    pub fn selected_packs(&self) -> &[KnownPack] {
        &self.selected
    }

    pub fn registry_element_json(&self, registry: &str, resource_id: &str) -> Option<&[u8]> {
        self.registry_elements
            .get(&(registry.to_string(), resource_id.to_string()))
            .map(AsRef::as_ref)
    }

    pub fn registry_element_json_arc(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> Option<Arc<[u8]>> {
        self.registry_elements
            .get(&(registry.to_string(), resource_id.to_string()))
            .map(Arc::clone)
    }
}

fn vanilla_known_pack(id: &str) -> KnownPack {
    KnownPack {
        namespace: VANILLA_NAMESPACE.to_string(),
        id: id.to_string(),
        version: MC_VERSION.to_string(),
    }
}

type RegistryElements = BTreeMap<(String, String), Arc<[u8]>>;

fn load_registry_elements(
    root: &Path,
    data_budget: Arc<crate::resource_budget::ResourceDataGenerationBudget>,
) -> Result<RegistryElements> {
    let resources = PackResourceStack::from_roots_with_data_budget([root], data_budget);
    load_registry_elements_from_stack(&resources)
}

fn load_registry_elements_from_stack(resources: &PackResourceStack) -> Result<RegistryElements> {
    let mut elements = BTreeMap::new();
    for registry in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        let (_, registry_path) = registry
            .resource_id
            .split_once(':')
            .context("generated registry id must contain a namespace")?;
        let prefix = format!("{registry_path}/");
        for resource in resources.list_data_resources(registry_path, ".json")? {
            let path = resource.location.path();
            let Some(element_path) = path
                .strip_prefix(&prefix)
                .and_then(|path| path.strip_suffix(".json"))
            else {
                continue;
            };
            let resource_id = format!("{}:{element_path}", resource.location.namespace());
            let (bytes, _) = resources
                .read_resource_json_bytes_and_value_from_resource(&resource, "registry element")
                .with_context(|| format!("parse registry element {}", resource.path.display()))?;
            elements.insert(
                (registry.resource_id.to_string(), resource_id),
                Arc::from(bytes),
            );
        }
    }
    Ok(elements)
}

fn load_builtin_datapack_elements_from_stack(
    resources: &PackResourceStack,
    datapack_id: &str,
) -> Result<RegistryElements> {
    let mut elements = BTreeMap::new();
    let data_root_prefix = format!("datapacks/{datapack_id}/data");
    for resource in resources.list_data_resources(&data_root_prefix, ".json")? {
        if resource.location.namespace() != VANILLA_NAMESPACE {
            continue;
        }
        let Some(after_data) = resource
            .location
            .path()
            .strip_prefix(&format!("{data_root_prefix}/"))
        else {
            continue;
        };
        let Some((elem_namespace, rest)) = after_data.split_once('/') else {
            continue;
        };
        for registry in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
            let Some((_, registry_path)) = registry.resource_id.split_once(':') else {
                continue;
            };
            let prefix = format!("{registry_path}/");
            let Some(element_path) = rest
                .strip_prefix(&prefix)
                .and_then(|tail| tail.strip_suffix(".json"))
            else {
                continue;
            };
            let (bytes, _) = resources
                .read_resource_json_bytes_and_value_from_resource(&resource, "registry element")
                .with_context(|| format!("parse registry element {}", resource.path.display()))?;
            elements.insert(
                (
                    registry.resource_id.to_string(),
                    format!("{elem_namespace}:{element_path}"),
                ),
                Arc::from(bytes),
            );
        }
    }
    Ok(elements)
}

#[derive(Debug, Deserialize)]
struct SourceVersion {
    id: String,
    protocol_version: i32,
}

fn validate_source_version(sources_dir: &Path) -> Result<()> {
    let path = sources_dir.join("version.json");
    let version: SourceVersion =
        crate::resource_budget::read_standalone_json(&path, "vanilla version metadata")
            .with_context(|| format!("parse vanilla version metadata {}", path.display()))?;
    if version.id != MC_VERSION || version.protocol_version != bbb_protocol::PROTOCOL_VERSION {
        bail!(
            "vanilla source version mismatch: expected {} protocol {}, found {} protocol {}",
            MC_VERSION,
            bbb_protocol::PROTOCOL_VERSION,
            version.id,
            version.protocol_version
        );
    }
    Ok(())
}

fn validate_pack_metadata(
    root: &Path,
    data_budget: &crate::resource_budget::ResourceDataGenerationBudget,
) -> Result<()> {
    let path = root.join("pack.mcmeta");
    data_budget
        .read_json::<serde_json::Value>(&path, "built-in pack metadata")
        .with_context(|| format!("parse built-in pack metadata {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "bbb-known-packs-{label}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn roots(&self) -> PackRoots {
            PackRoots::from_paths(
                self.0.clone(),
                self.0.clone(),
                self.0.join("assets/minecraft"),
            )
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_file(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn complete_core(label: &str) -> TempRoot {
        let root = TempRoot::new(label);
        write_file(
            &root.0.join("version.json"),
            &format!(
                r#"{{"id":"{MC_VERSION}","protocol_version":{}}}"#,
                bbb_protocol::PROTOCOL_VERSION
            ),
        );
        for registry in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
            let (_, registry_path) = registry.resource_id.split_once(':').unwrap();
            write_file(
                &root
                    .0
                    .join("data/minecraft")
                    .join(registry_path)
                    .join("base.json"),
                &format!(
                    r#"{{"registry":"{}","source":"core"}}"#,
                    registry.resource_id
                ),
            );
        }
        root
    }

    fn known_pack(id: &str, version: &str) -> KnownPack {
        KnownPack {
            namespace: "minecraft".to_string(),
            id: id.to_string(),
            version: version.to_string(),
        }
    }

    #[test]
    fn selects_exact_available_packs_in_server_offer_order() {
        let root = complete_core("select");
        let feature = root.0.join("data/minecraft/datapacks/trade_rebalance");
        write_file(
            &feature.join("pack.mcmeta"),
            r#"{"pack":{"description":"test"}}"#,
        );

        let manager = KnownPacksManager::load(&root.roots()).unwrap();
        let offered = vec![
            known_pack("trade_rebalance", MC_VERSION),
            known_pack("core", "26.1.1"),
            KnownPack {
                namespace: "example".to_string(),
                id: "core".to_string(),
                version: MC_VERSION.to_string(),
            },
            known_pack("core", MC_VERSION),
        ];

        assert_eq!(
            manager.select_offered(&offered),
            vec![offered[0].clone(), offered[3].clone()]
        );
    }

    #[test]
    fn selected_data_uses_later_pack_precedence() {
        let root = complete_core("precedence");
        let feature = root.0.join("data/minecraft/datapacks/trade_rebalance");
        write_file(
            &feature.join("pack.mcmeta"),
            r#"{"pack":{"description":"test"}}"#,
        );
        write_file(
            &feature.join("data/minecraft/worldgen/biome/base.json"),
            r#"{"source":"feature"}"#,
        );
        let manager = KnownPacksManager::load(&root.roots()).unwrap();
        let source = manager
            .data_source_for(&[
                known_pack("core", MC_VERSION),
                known_pack("trade_rebalance", MC_VERSION),
            ])
            .unwrap();

        assert_eq!(
            source.registry_element_json("minecraft:worldgen/biome", "minecraft:base"),
            Some(r#"{"source":"feature"}"#.as_bytes())
        );
        assert!(source
            .registry_element_json("minecraft:damage_type", "minecraft:base")
            .is_some());
    }

    #[test]
    fn selected_registry_json_preserves_raw_java_strings_and_number_lexemes() {
        let root = complete_core("raw-java-strings");
        let raw = r#"{"description":{"score":{"name":"\uD800","objective":"\uDC00"}},"number":1.2300e+04,"duplicate":"first","duplicate":"last"}"#;
        write_file(&root.0.join("data/minecraft/trim_material/base.json"), raw);
        let manager = KnownPacksManager::load(&root.roots()).unwrap();
        let core = known_pack("core", MC_VERSION);
        assert_eq!(
            manager.select_offered(std::slice::from_ref(&core)),
            std::slice::from_ref(&core)
        );
        let source = manager.data_source_for(&[core]).unwrap();
        assert_eq!(
            source.registry_element_json("minecraft:trim_material", "minecraft:base"),
            Some(raw.as_bytes())
        );
    }

    #[test]
    fn invalid_registry_json_still_prevents_known_pack_loading() {
        let root = complete_core("invalid-registry-json");
        write_file(
            &root.0.join("data/minecraft/trim_material/base.json"),
            r#"{"name":"\uD800","invalid":}"#,
        );
        let error = KnownPacksManager::load(&root.roots()).unwrap_err();
        assert!(
            error.to_string().contains("parse registry element"),
            "{error:?}"
        );
    }

    #[test]
    fn registry_file_budget_rejection_sticks_during_known_pack_loading() {
        let root = complete_core("registry-file-budget");
        let path = root.0.join("data/minecraft/trim_material/base.json");
        std::fs::File::create(path)
            .unwrap()
            .set_len(crate::resource_budget::MAX_RESOURCE_JSON_FILE_BYTES + 1)
            .unwrap();
        let roots = root.roots();
        let budget = roots.resource_data_budget();
        let error = KnownPacksManager::load(&roots).unwrap_err();
        assert!(
            format!("{error:#}").contains("exceeding the 16777216 byte limit"),
            "{error:?}"
        );
        assert_eq!(budget.observation().rejections, 1);
        assert!(budget.ensure_admitted().is_err());
    }

    #[test]
    fn refuses_to_advertise_an_incomplete_core_pack() {
        let root = complete_core("incomplete");
        std::fs::remove_dir_all(root.0.join("data/minecraft/timeline")).unwrap();

        let error = KnownPacksManager::load(&root.roots()).unwrap_err();
        assert!(error
            .to_string()
            .contains("no elements for synchronized registry minecraft:timeline"));
    }

    #[test]
    fn rejects_a_selected_pack_outside_the_local_inventory() {
        let root = complete_core("unavailable");
        let manager = KnownPacksManager::load(&root.roots()).unwrap();

        let error = manager
            .data_source_for(&[known_pack("missing", MC_VERSION)])
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("selected unavailable known pack minecraft:missing:26.1"));
    }

    #[test]
    fn archive_backed_known_packs_ignore_server_and_user_resource_packs() {
        use std::io::Write;
        use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file("pack.mcmeta", opts).unwrap();
            zip.write_all(
                br#"{"pack":{"description":"vanilla","min_format":[84,0],"max_format":[84,0]}}"#,
            )
            .unwrap();
            zip.start_file("version.json", opts).unwrap();
            zip.write_all(
                format!(
                    r#"{{"id":"{MC_VERSION}","protocol_version":{}}}"#,
                    bbb_protocol::PROTOCOL_VERSION
                )
                .as_bytes(),
            )
            .unwrap();
            for registry in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
                let (_, registry_path) = registry.resource_id.split_once(':').unwrap();
                zip.start_file(format!("data/minecraft/{registry_path}/base.json"), opts)
                    .unwrap();
                zip.write_all(br#"{"source":"vanilla"}"#).unwrap();
            }
            zip.finish().unwrap();
        }
        let vanilla_zip: Arc<[u8]> = Arc::from(cursor.into_inner());
        let base_roots = PackRoots::from_web_bundle(vanilla_zip, None, None).unwrap();

        let evil_dir = TempRoot::new("evil-server-pack");
        write_file(
            &evil_dir.0.join("pack.mcmeta"),
            r#"{"pack":{"description":"evil","min_format":[84,0],"max_format":[84,0]}}"#,
        );
        write_file(
            &evil_dir.0.join("version.json"),
            r#"{"id":"99.9","protocol_version":1}"#,
        );
        write_file(
            &evil_dir.0.join("data/minecraft/worldgen/biome/base.json"),
            r#"{"source":"evil"}"#,
        );
        write_file(
            &evil_dir.0.join("data/minecraft/datapacks/evil/pack.mcmeta"),
            r#"{"pack":{"description":"evil"}}"#,
        );

        let metadata = crate::read_client_resource_pack_metadata(&evil_dir.0).unwrap();
        let server_pack =
            crate::SelectedClientPack::server("evil", 1, 0, &evil_dir.0, metadata).unwrap();
        let stack = base_roots
            .selected_client_packs()
            .unwrap()
            .with_server_packs([server_pack])
            .unwrap();
        let roots = base_roots.with_selected_client_packs(stack).unwrap();

        let manager = KnownPacksManager::load(&roots).unwrap();
        assert_eq!(manager.available_packs().len(), 1);
        assert_eq!(
            manager.available_packs().next().unwrap(),
            &known_pack("core", MC_VERSION)
        );
        let source = manager
            .data_source_for(&[known_pack("core", MC_VERSION)])
            .unwrap();
        assert_eq!(
            source.registry_element_json("minecraft:worldgen/biome", "minecraft:base"),
            Some(br#"{"source":"vanilla"}"#.as_slice())
        );
    }
}
