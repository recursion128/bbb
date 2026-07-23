use std::collections::BTreeMap;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    block_registry_declarations::{
        copy_base_from_properties, copy_base_from_register_helper, has_properties_of,
        load_generated_block_declarations,
    },
    generated_java_expression::GeneratedJavaExpression,
    resources::ResourceLocation,
    PackRoots,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockOcclusionProfile {
    pub can_occlude: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockOcclusionProfileCatalog {
    profiles: BTreeMap<String, BlockOcclusionProfile>,
}

impl BlockOcclusionProfileCatalog {
    pub fn load(_roots: &PackRoots) -> Result<Self> {
        Self::from_generated_source_catalog()
    }

    pub(crate) fn from_generated_source_catalog() -> Result<Self> {
        let source = load_generated_block_declarations()?;
        let mut fields = BTreeMap::new();
        let mut profiles = BTreeMap::new();

        for declaration in &source.declarations {
            let Some(profile) = profile_for_declaration(&declaration.initializer, &fields)? else {
                continue;
            };
            if profiles
                .insert(declaration.resource_id.clone(), profile)
                .is_some()
            {
                bail!(
                    "duplicate block occlusion profile {}",
                    declaration.resource_id
                );
            }
            fields.insert(declaration.field.clone(), profile);
        }

        if profiles.len() != source.declarations.len() {
            bail!(
                "generated block occlusion profiles cover {} of {} declarations",
                profiles.len(),
                source.declarations.len()
            );
        }
        Ok(Self { profiles })
    }

    pub fn profile(&self, resource_id: &str) -> Option<&BlockOcclusionProfile> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.profiles.get(&resource_id)
    }

    pub fn profiles(&self) -> &BTreeMap<String, BlockOcclusionProfile> {
        &self.profiles
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

fn profile_for_declaration(
    expression: &GeneratedJavaExpression,
    fields: &BTreeMap<String, BlockOcclusionProfile>,
) -> Result<Option<BlockOcclusionProfile>> {
    let root_name = expression.name.as_deref();
    let mut profile = if let Some(base) = copy_base_from_register_helper(expression)? {
        let Some(profile) = fields.get(base).copied() else {
            return Ok(None);
        };
        profile
    } else if let Some(base) = copy_base_from_properties(expression)? {
        let Some(profile) = fields.get(base).copied() else {
            return Ok(None);
        };
        profile
    } else if matches!(root_name, Some("registerBed" | "registerStainedGlass")) {
        profile(false)
    } else if let Some(helper_profile) = profile_from_known_helper(expression) {
        helper_profile
    } else if has_properties_of(expression) || expression.call("wallVariant").is_some() {
        profile(true)
    } else {
        return Ok(None);
    };

    for call in expression.calls_in_source_order() {
        if matches!(call.name.as_deref(), Some("noCollision" | "noOcclusion")) {
            profile.can_occlude = false;
        }
    }
    Ok(Some(profile))
}

fn profile_from_known_helper(
    expression: &GeneratedJavaExpression,
) -> Option<BlockOcclusionProfile> {
    let has = |name| expression.call(name).is_some();
    if has("logProperties") || has("netherStemProperties") || has("pistonProperties") {
        return Some(profile(true));
    }
    if has("leavesProperties")
        || has("shulkerBoxProperties")
        || has("buttonProperties")
        || has("flowerPotProperties")
        || has("candleProperties")
    {
        return Some(profile(false));
    }
    None
}

const fn profile(can_occlude: bool) -> BlockOcclusionProfile {
    BlockOcclusionProfile { can_occlude }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_occlusion_profiles_cover_all_block_property_shapes() {
        let catalog = BlockOcclusionProfileCatalog::from_generated_source_catalog().unwrap();

        assert_eq!(catalog.len(), 1168);
        for block_name in [
            "minecraft:stone",
            "minecraft:oak_planks",
            "minecraft:oak_stairs",
            "minecraft:piston",
            "minecraft:infested_deepslate",
        ] {
            assert!(
                catalog.profile(block_name).unwrap().can_occlude,
                "{block_name}"
            );
        }
        for block_name in [
            "minecraft:glass",
            "minecraft:white_stained_glass",
            "minecraft:white_bed",
            "minecraft:oak_leaves",
            "minecraft:spawner",
            "minecraft:white_shulker_box",
            "minecraft:stone_button",
            "minecraft:flower_pot",
            "minecraft:white_candle",
        ] {
            assert!(
                !catalog.profile(block_name).unwrap().can_occlude,
                "{block_name}"
            );
        }
    }

    #[test]
    fn block_occlusion_profile_load_does_not_require_local_java_sources() {
        let root = std::env::temp_dir().join(format!(
            "bbb-pack-block-occlusion-no-java-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("sources").join(crate::MC_VERSION)).unwrap();
        let roots = PackRoots::from_root(&root).unwrap();

        let catalog = BlockOcclusionProfileCatalog::load(&roots).unwrap();

        assert_eq!(catalog.len(), 1168);
        std::fs::remove_dir_all(root).unwrap();
    }
}
