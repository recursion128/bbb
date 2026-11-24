//! The server's generated/datapack template lookup and repository.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::normalize_resource_location;
use bbb_server_datapack::pack_create::is_path_part_portable;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::structure_template::{
    parse_structure_template_with_content_registry, StructureTemplate, StructureTemplateSet,
};

pub(super) struct StructureTemplateManager {
    generated_directory: Option<PathBuf>,
    content_registry: Arc<ContentRegistry>,
    repository: RefCell<BTreeMap<String, Option<StructureTemplate>>>,
}

impl StructureTemplateManager {
    #[cfg(test)]
    pub(super) fn new(generated_directory: Option<PathBuf>) -> Self {
        Self::new_with_content_registry(generated_directory, Arc::clone(ContentRegistry::vanilla()))
    }

    pub(super) fn new_with_content_registry(
        generated_directory: Option<PathBuf>,
        content_registry: Arc<ContentRegistry>,
    ) -> Self {
        Self {
            generated_directory,
            content_registry,
            repository: RefCell::new(BTreeMap::new()),
        }
    }

    // StructureTemplateManager.java:94-118: misses are cached until resource reload.
    pub(super) fn get(
        &self,
        id: &str,
        datapacks: Option<&StructureTemplateSet>,
    ) -> Option<StructureTemplate> {
        let id = normalize_resource_location(id.to_owned()).ok()?;
        if let Some(cached) = self.repository.borrow().get(&id) {
            return cached.clone();
        }
        // StructureTemplateManager.java:69-79: generated templates precede datapack resources.
        let template = self.load_generated(&id).or_else(|| {
            datapacks
                .and_then(|templates| templates.template(&id))
                .map(|entry| entry.template.clone())
        });
        self.repository.borrow_mut().insert(id, template.clone());
        template
    }

    // StructureTemplateManager.java:83-92: a cached miss becomes an empty template.
    pub(super) fn get_or_create(
        &self,
        id: &str,
        datapacks: Option<&StructureTemplateSet>,
    ) -> Option<StructureTemplate> {
        let id = normalize_resource_location(id.to_owned()).ok()?;
        if let Some(template) = self.get(&id, datapacks) {
            return Some(template);
        }
        let template = StructureTemplate::empty();
        self.repository
            .borrow_mut()
            .insert(id, Some(template.clone()));
        Some(template)
    }

    pub(super) fn put(&self, id: &str, template: StructureTemplate) -> bool {
        let Some(id) = normalized_template_id(id) else {
            return false;
        };
        self.repository.borrow_mut().insert(id, Some(template));
        true
    }

    // StructureTemplateManager.java:121-162: saving never evicts the in-memory template.
    pub(super) fn save(&self, id: &str) -> bool {
        let Ok(id) = normalize_resource_location(id.to_owned()) else {
            return false;
        };
        let Some(template) = self.repository.borrow().get(&id).cloned().flatten() else {
            return false;
        };
        let Some(path) = self.generated_path(&id) else {
            tracing::warn!(
                template = id,
                "could not resolve generated structure template path"
            );
            return false;
        };
        // TemplatePathFactory.java:26-35: validate converted structure/<path>.nbt parts.
        let resource_path = format!(
            "structure/{}.nbt",
            id.split_once(':').expect("normalized id").1
        );
        if !resource_path.split('/').all(is_path_part_portable) {
            tracing::warn!(template = id, path = %path.display(), "structure template path is not portable");
            return false;
        }
        let Some(parent) = path.parent() else {
            return false;
        };
        if let Err(error) =
            std::fs::create_dir_all(parent).and_then(|()| std::fs::write(&path, template.raw()))
        {
            tracing::warn!(template = id, path = %path.display(), %error, "could not save structure template");
            return false;
        }
        true
    }

    // StructureTemplateManager.java:172-174: disk contents survive repository removal.
    pub(super) fn remove(&self, id: &str) {
        if let Ok(id) = normalize_resource_location(id.to_owned()) {
            self.repository.borrow_mut().remove(&id);
        }
    }

    // StructureTemplateManager.java:98-100; DirectoryTemplateSource.java:65-81.
    pub(super) fn list_templates(&self, datapacks: Option<&StructureTemplateSet>) -> Vec<String> {
        let mut ids = std::collections::BTreeSet::new();
        if let Some(directory) = &self.generated_directory {
            if let Ok(namespaces) = std::fs::read_dir(directory) {
                for namespace in namespaces.flatten() {
                    if !namespace.path().is_dir() {
                        continue;
                    }
                    let name = namespace.file_name();
                    let Some(namespace) = name.to_str() else {
                        continue;
                    };
                    let root = directory.join(namespace).join("structure");
                    let mut pending = vec![root.clone()];
                    while let Some(path) = pending.pop() {
                        if !std::fs::symlink_metadata(&path).is_ok_and(|entry| entry.is_dir()) {
                            continue;
                        }
                        let Ok(entries) = std::fs::read_dir(path) else {
                            continue;
                        };
                        for entry in entries.flatten() {
                            let Ok(kind) = entry.file_type() else {
                                continue;
                            };
                            let path = entry.path();
                            if kind.is_dir() {
                                pending.push(path);
                            } else if kind.is_file() {
                                let Ok(relative) = path.strip_prefix(&root) else {
                                    continue;
                                };
                                let Some(joined) = relative
                                    .components()
                                    .map(|component| match component {
                                        std::path::Component::Normal(part) => part.to_str(),
                                        _ => None,
                                    })
                                    .collect::<Option<Vec<_>>>()
                                    .map(|parts| parts.join("/"))
                                else {
                                    continue;
                                };
                                let Some(path) = joined.strip_suffix(".nbt") else {
                                    continue;
                                };
                                if let Ok(id) =
                                    normalize_resource_location(format!("{namespace}:{path}"))
                                {
                                    ids.insert(id);
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(datapacks) = datapacks {
            ids.extend(datapacks.templates.iter().map(|entry| entry.id.clone()));
        }
        ids.into_iter().collect()
    }

    pub(super) fn on_resource_reload(&self) {
        self.repository.borrow_mut().clear();
    }

    fn generated_path(&self, id: &str) -> Option<PathBuf> {
        let directory = self.generated_directory.as_ref()?;
        let id = normalized_template_id(id)?;
        let (namespace, path) = id.split_once(':')?;
        Some(
            directory
                .join(namespace)
                .join("structure")
                .join(format!("{path}.nbt")),
        )
    }

    fn load_generated(&self, id: &str) -> Option<StructureTemplate> {
        let path = self.generated_path(id)?;
        let file = File::open(&path).ok()?;
        let limit = DatapackLoadLimits::default().max_element_bytes;
        let mut bytes = Vec::new();
        if let Err(error) = file.take(limit as u64 + 1).read_to_end(&mut bytes) {
            tracing::warn!(template = id, path = %path.display(), %error, "could not read structure template");
            return None;
        }
        if bytes.len() > limit {
            tracing::warn!(template = id, path = %path.display(), "structure template exceeds load limit");
            return None;
        }
        match parse_structure_template_with_content_registry(&bytes, &self.content_registry) {
            Ok(template) => Some(template),
            Err(error) => {
                tracing::warn!(template = id, path = %path.display(), %error, "could not load structure template");
                None
            }
        }
    }
}

fn normalized_template_id(id: &str) -> Option<String> {
    let id = normalize_resource_location(id.to_owned()).ok()?;
    let (_, path) = id.split_once(':')?;
    if path
        .split('/')
        .any(|component| matches!(component, "" | "." | ".."))
    {
        return None;
    }
    Some(id)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use bbb_protocol::packets::BlockPos;
    use bbb_server_datapack::structure_template::{
        MaterializedStructureTemplate, StructureTemplateBlock, StructureTemplatePalette,
    };

    use super::*;

    #[test]
    fn cached_capture_saves_and_reloads_after_repository_removal() {
        let directory =
            std::env::temp_dir().join(format!("bbb-template-manager-{}", uuid::Uuid::new_v4()));
        let manager = StructureTemplateManager::new(Some(directory.clone()));
        let id = "example:captured/room";
        assert!(manager.get(id, None).is_none());
        let empty = manager.get_or_create(id, None).unwrap();
        assert!(empty.materialized().palettes.is_empty());
        assert_eq!(manager.get(id, None), Some(empty));
        assert!(manager.list_templates(None).is_empty());
        let mut template = StructureTemplate::from_materialized(MaterializedStructureTemplate {
            size: [1, 1, 1],
            palettes: vec![StructureTemplatePalette {
                blocks: vec![StructureTemplateBlock {
                    position: BlockPos { x: 0, y: 0, z: 0 },
                    state: vanilla_block_state_id_by_name_and_properties(
                        "minecraft:stone",
                        &BTreeMap::new(),
                    )
                    .unwrap(),
                    nbt: None,
                }],
                jigsaws: Vec::new(),
            }],
            entities: Vec::new(),
        })
        .unwrap();
        template.set_author("Builder".to_owned());
        assert!(manager.put(id, template.clone()));
        assert_eq!(manager.get_or_create(id, None), Some(template.clone()));
        assert!(manager.save(id));
        assert_eq!(manager.get(id, None).unwrap().author(), "Builder");
        assert_eq!(manager.list_templates(None), vec![id.to_owned()]);
        manager.remove(id);
        let reloaded = manager.get(id, None).unwrap();
        assert_eq!(reloaded.materialized(), template.materialized());
        assert_eq!(reloaded.author(), "?");
        assert!(directory
            .join("example/structure/captured/room.nbt")
            .is_file());
        assert!(manager.put(id, template));
        manager.on_resource_reload();
        assert_eq!(manager.get_or_create(id, None), Some(reloaded));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
