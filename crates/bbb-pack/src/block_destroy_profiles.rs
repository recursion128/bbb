use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestroyProfile {
    pub destroy_time_tenths: Option<u32>,
    pub requires_correct_tool: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestroyProfileCatalog {
    profiles: BTreeMap<String, BlockDestroyProfile>,
}

impl BlockDestroyProfileCatalog {
    pub fn load(_roots: &PackRoots) -> Result<Self> {
        Self::from_generated_source_catalog()
    }

    pub(crate) fn from_generated_source_catalog() -> Result<Self> {
        let source = load_generated_block_declarations()?;

        let mut fields = BTreeMap::new();
        let mut profiles = BTreeMap::new();
        for declaration in &source.declarations {
            let source_context = || {
                format!(
                    "{}:{}..{}: resolve Blocks.{}",
                    declaration.source_path,
                    declaration.start_byte,
                    declaration.end_byte,
                    declaration.field
                )
            };
            if let Some(profile) = profile_for_declaration(&declaration.initializer, &fields)
                .with_context(source_context)?
            {
                if profiles
                    .insert(declaration.resource_id.clone(), profile.clone())
                    .is_some()
                {
                    bail!(
                        "duplicate block destroy profile {}",
                        declaration.resource_id
                    );
                }
                fields.insert(declaration.field.clone(), profile);
            }
        }

        if profiles.is_empty() {
            bail!("generated Blocks catalog did not contain block destroy profiles");
        }
        Ok(Self { profiles })
    }

    pub fn profile(&self, resource_id: &str) -> Option<&BlockDestroyProfile> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.profiles.get(&resource_id)
    }

    pub fn profiles(&self) -> &BTreeMap<String, BlockDestroyProfile> {
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
    fields: &BTreeMap<String, BlockDestroyProfile>,
) -> Result<Option<BlockDestroyProfile>> {
    let root_name = expression.name.as_deref();
    let mut profile = if let Some(base) = copy_base_from_register_helper(expression)? {
        let Some(profile) = fields.get(base).cloned() else {
            return Ok(None);
        };
        profile
    } else if let Some(base) = copy_base_from_properties(expression)? {
        let Some(profile) = fields.get(base).cloned() else {
            return Ok(None);
        };
        profile
    } else if root_name == Some("registerBed") {
        profile(2, false)
    } else if root_name == Some("registerStainedGlass") {
        profile(3, false)
    } else if let Some(helper_profile) = profile_from_known_helper(expression) {
        helper_profile
    } else if has_properties_of(expression) || expression.call("wallVariant").is_some() {
        default_profile()
    } else {
        return Ok(None);
    };

    apply_property_chain(expression, &mut profile)?;
    if let Some(host) = infested_host_from_constructor(expression)? {
        let Some(host_profile) = fields.get(host) else {
            return Ok(None);
        };
        profile.destroy_time_tenths =
            infested_destroy_time_tenths(host_profile.destroy_time_tenths);
    }
    Ok(Some(profile))
}

fn profile_from_known_helper(expression: &GeneratedJavaExpression) -> Option<BlockDestroyProfile> {
    let has = |name| expression.call(name).is_some();
    if has("logProperties") || has("netherStemProperties") {
        return Some(profile(20, false));
    }
    if has("leavesProperties") {
        return Some(profile(2, false));
    }
    if has("buttonProperties") {
        return Some(profile(5, false));
    }
    if has("flowerPotProperties") {
        return Some(profile(0, false));
    }
    if has("candleProperties") {
        return Some(profile(1, false));
    }
    if has("shulkerBoxProperties") {
        return Some(profile(20, false));
    }
    if has("pistonProperties") {
        return Some(profile(15, false));
    }
    None
}

fn infested_host_from_constructor(expression: &GeneratedJavaExpression) -> Result<Option<&str>> {
    let Some(creation) = expression.creations().find(|creation| {
        matches!(
            creation.name.as_deref(),
            Some("InfestedBlock" | "InfestedRotatedPillarBlock")
        )
    }) else {
        return Ok(None);
    };
    let host = creation
        .arguments
        .first()
        .context("infested block construction lacks host block")?;
    if host.kind != "identifier" {
        bail!("infested block host is not a block field");
    }
    Ok(Some(host.canonical.as_str()))
}

fn infested_destroy_time_tenths(host_destroy_time_tenths: Option<u32>) -> Option<u32> {
    host_destroy_time_tenths.map(|value| (f64::from(value) / 2.0).round() as u32)
}

fn apply_property_chain(
    expression: &GeneratedJavaExpression,
    profile: &mut BlockDestroyProfile,
) -> Result<()> {
    for call in expression.calls_in_source_order() {
        match call.name.as_deref() {
            Some("strength" | "destroyTime") => {
                let value = call
                    .arguments
                    .first()
                    .context("block destroy-time property lacks value")?;
                profile.destroy_time_tenths = destroy_time_tenths(&value.canonical)?;
            }
            Some("instabreak") => profile.destroy_time_tenths = Some(0),
            Some("requiresCorrectToolForDrops") => profile.requires_correct_tool = true,
            _ => {}
        }
    }
    Ok(())
}

fn destroy_time_tenths(value: &str) -> Result<Option<u32>> {
    let normalized = value.replace(' ', "");
    let normalized = normalized.strip_suffix(['F', 'f']).unwrap_or(&normalized);
    let value: f64 = normalized.parse()?;
    if value < 0.0 {
        return Ok(None);
    }
    Ok(Some((value * 10.0).round() as u32))
}

fn default_profile() -> BlockDestroyProfile {
    profile(0, false)
}

fn profile(destroy_time_tenths: u32, requires_correct_tool: bool) -> BlockDestroyProfile {
    BlockDestroyProfile {
        destroy_time_tenths: Some(destroy_time_tenths),
        requires_correct_tool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_block_destroy_profiles_cover_direct_copy_helper_and_infested_blocks() {
        let catalog = BlockDestroyProfileCatalog::from_generated_source_catalog().unwrap();

        assert_eq!(catalog.len(), 1168);
        assert_eq!(
            catalog.profile("minecraft:diamond_ore").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(30),
                requires_correct_tool: true,
            }
        );
        assert_eq!(
            catalog
                .profile("minecraft:deepslate_diamond_ore")
                .unwrap()
                .destroy_time_tenths,
            Some(45)
        );
        assert_eq!(
            catalog.profile("minecraft:infested_stone").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(8),
                requires_correct_tool: false,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:infested_cobblestone").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(10),
                requires_correct_tool: false,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:infested_deepslate").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(15),
                requires_correct_tool: false,
            }
        );
        assert_eq!(
            catalog
                .profile("minecraft:bedrock")
                .unwrap()
                .destroy_time_tenths,
            None
        );
        assert_eq!(
            catalog
                .profile("minecraft:flower_pot")
                .unwrap()
                .destroy_time_tenths,
            Some(0)
        );
        assert_eq!(
            catalog.profile("minecraft:oak_stairs").unwrap(),
            catalog.profile("minecraft:oak_planks").unwrap()
        );
        assert_eq!(
            catalog
                .profile("minecraft:white_bed")
                .unwrap()
                .destroy_time_tenths,
            Some(2)
        );
        assert_eq!(
            catalog
                .profile("minecraft:white_stained_glass")
                .unwrap()
                .destroy_time_tenths,
            Some(3)
        );
        assert_eq!(
            catalog
                .profile("minecraft:waxed_oxidized_copper_bars")
                .unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(50),
                requires_correct_tool: true,
            }
        );
    }

    #[test]
    fn generated_block_declarations_are_complete_contiguous_and_unique() {
        let source = load_generated_block_declarations().unwrap();
        assert_eq!(source.declarations.len(), 1168);
    }

    #[test]
    fn block_destroy_profile_load_does_not_require_local_java_sources() {
        let root = std::env::temp_dir().join(format!(
            "bbb-pack-block-destroy-no-java-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("sources").join(crate::MC_VERSION)).unwrap();
        let roots = PackRoots::from_root(&root).unwrap();

        let catalog = BlockDestroyProfileCatalog::load(&roots).unwrap();

        assert!(catalog.len() >= 600);
        std::fs::remove_dir_all(root).unwrap();
    }
}
