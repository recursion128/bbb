use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    resources::{PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticleDefinitionCatalog {
    definitions: BTreeMap<String, ParticleDefinition>,
}

impl ParticleDefinitionCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut definitions = BTreeMap::new();
        for resource in stack.list_resources("particles", ".json")? {
            let particle_id = particle_id_from_resource(&resource.location)?;
            let bytes = std::fs::read(&resource.path)
                .with_context(|| format!("read particle definition {}", resource.path.display()))?;
            let definition = ParticleDefinition::from_json_bytes(&bytes).with_context(|| {
                format!("parse particle definition {}", resource.path.display())
            })?;
            definitions.insert(particle_id, definition);
        }
        Ok(Self { definitions })
    }

    pub fn definition(&self, particle_id: &str) -> Option<&ParticleDefinition> {
        let particle_id = ResourceLocation::parse(particle_id).ok()?.id();
        self.definitions.get(&particle_id)
    }

    pub fn definitions(&self) -> &BTreeMap<String, ParticleDefinition> {
        &self.definitions
    }

    pub fn unique_sprite_ids(&self) -> Vec<String> {
        self.definitions
            .values()
            .flat_map(|definition| definition.textures.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticleDefinition {
    pub textures: Vec<String>,
}

impl ParticleDefinition {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let raw: RawParticleDefinition = serde_json::from_slice(bytes)?;
        raw.into_definition()
    }

    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawParticleDefinition {
    #[serde(default)]
    textures: Vec<String>,
}

impl RawParticleDefinition {
    fn into_definition(self) -> Result<ParticleDefinition> {
        Ok(ParticleDefinition {
            textures: self
                .textures
                .into_iter()
                .map(|texture| ResourceLocation::parse(&texture).map(|location| location.id()))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

fn particle_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("particles/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "particle definition resource {} is outside particles",
                location.id()
            )
        })?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn particle_definition_catalog_loads_textures_and_defaults() {
        let root = unique_temp_dir("particle-definition-simple");
        write_json(
            &particle_dir(&root).join("cloud.json"),
            r#"{
              "textures": [
                "minecraft:generic_7",
                "minecraft:generic_6"
              ]
            }"#,
        );
        write_json(&particle_dir(&root).join("custom_empty.json"), r#"{}"#);

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_particle_definition_catalog()
            .unwrap();

        assert_eq!(catalog.len(), 2);
        assert_eq!(
            catalog.definition("cloud").unwrap().textures,
            vec![
                "minecraft:generic_7".to_string(),
                "minecraft:generic_6".to_string(),
            ]
        );
        assert!(catalog
            .definition("custom_empty")
            .unwrap()
            .textures
            .is_empty());
        assert_eq!(
            catalog.unique_sprite_ids(),
            vec![
                "minecraft:generic_6".to_string(),
                "minecraft:generic_7".to_string(),
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn particle_definition_catalog_uses_resource_pack_overrides() {
        let root = unique_temp_dir("particle-definition-pack");
        let pack = root.join("resource-pack");
        write_json(
            &particle_dir(&root).join("flame.json"),
            r#"{
              "textures": [
                "minecraft:flame"
              ]
            }"#,
        );
        write_json(
            &pack
                .join("assets")
                .join("minecraft")
                .join("particles")
                .join("flame.json"),
            r#"{
              "textures": [
                "custom:blue_flame",
                "custom:blue_flame_1"
              ]
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack])
            .load_particle_definition_catalog()
            .unwrap();

        assert_eq!(
            catalog.definition("minecraft:flame").unwrap().textures,
            vec![
                "custom:blue_flame".to_string(),
                "custom:blue_flame_1".to_string(),
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn particle_definition_rejects_invalid_sprite_ids() {
        let err = ParticleDefinition::from_json_bytes(
            br#"{
              "textures": [
                "BadNamespace:generic"
              ]
            }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid resource namespace"));
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_particle_definitions() {
        let catalog = PackRoots::discover()
            .unwrap()
            .load_particle_definition_catalog()
            .unwrap();
        assert_eq!(catalog.len(), 106);
        assert_eq!(
            catalog.definition("minecraft:cloud").unwrap().textures,
            vec![
                "minecraft:generic_7".to_string(),
                "minecraft:generic_6".to_string(),
                "minecraft:generic_5".to_string(),
                "minecraft:generic_4".to_string(),
                "minecraft:generic_3".to_string(),
                "minecraft:generic_2".to_string(),
                "minecraft:generic_1".to_string(),
                "minecraft:generic_0".to_string(),
            ]
        );
        assert_eq!(
            catalog
                .definition("minecraft:campfire_signal_smoke")
                .unwrap()
                .texture_count(),
            12
        );
        assert_eq!(catalog.unique_sprite_ids().len(), 251);
    }

    fn particle_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(crate::MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("particles")
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
