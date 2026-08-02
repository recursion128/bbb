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

fn load_registry_elements(
    root: &Path,
    data_budget: Arc<crate::resource_budget::ResourceDataGenerationBudget>,
) -> Result<BTreeMap<(String, String), Arc<[u8]>>> {
    let resources = PackResourceStack::from_roots_with_data_budget([root], data_budget);
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
                .read_json_bytes_and_value(&resource.path, "registry element")
                .with_context(|| format!("parse registry element {}", resource.path.display()))?;
            elements.insert(
                (registry.resource_id.to_string(), resource_id),
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
}
