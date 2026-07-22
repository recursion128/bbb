use std::collections::BTreeMap;
use std::sync::OnceLock;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    block_destroy_profiles::BlockDestroyProfileCatalog,
    block_registry_declarations::{
        copy_base_from_properties, copy_base_from_register_helper,
        load_generated_block_declarations,
    },
    generated_java_expression::GeneratedJavaExpression,
    resources::ResourceLocation,
    PackRoots,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockPistonPushReaction {
    Normal,
    Destroy,
    Block,
    Ignore,
    PushOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockPistonProfile {
    pub push_reaction: BlockPistonPushReaction,
    pub unbreakable: bool,
    pub has_block_entity: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockPistonProfileCatalog {
    profiles: BTreeMap<String, BlockPistonProfile>,
}

impl BlockPistonProfileCatalog {
    pub fn load(_roots: &PackRoots) -> Result<Self> {
        Self::from_generated_source_catalog()
    }

    pub fn load_builtin() -> Result<Self> {
        Self::from_generated_source_catalog()
    }

    fn from_generated_source_catalog() -> Result<Self> {
        let source = load_generated_block_declarations()?;
        let destroy = BlockDestroyProfileCatalog::from_generated_source_catalog()?;
        let mut fields = BTreeMap::new();
        let mut profiles = BTreeMap::new();

        for declaration in &source.declarations {
            let reaction = push_reaction_for_declaration(&declaration.initializer, &fields)
                .with_context(|| {
                    format!(
                        "{}:{}..{}: resolve piston profile for Blocks.{}",
                        declaration.source_path,
                        declaration.start_byte,
                        declaration.end_byte,
                        declaration.field
                    )
                })?;
            let destroy_profile = destroy.profile(&declaration.resource_id).with_context(|| {
                format!(
                    "missing generated destroy profile for {}",
                    declaration.resource_id
                )
            })?;
            let profile = BlockPistonProfile {
                push_reaction: reaction,
                unbreakable: destroy_profile.destroy_time_tenths.is_none(),
                has_block_entity: declaration.has_block_entity,
            };
            fields.insert(declaration.field.clone(), reaction);
            if profiles
                .insert(declaration.resource_id.clone(), profile)
                .is_some()
            {
                bail!("duplicate block piston profile {}", declaration.resource_id);
            }
        }

        if profiles.len() != bbb_protocol::block_types::VANILLA_BLOCK_TYPES.len() {
            bail!(
                "generated block piston profile count {} does not match protocol block count {}",
                profiles.len(),
                bbb_protocol::block_types::VANILLA_BLOCK_TYPES.len()
            );
        }
        Ok(Self { profiles })
    }

    pub fn profile(&self, resource_id: &str) -> Option<&BlockPistonProfile> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.profiles.get(&resource_id)
    }

    pub fn profiles(&self) -> &BTreeMap<String, BlockPistonProfile> {
        &self.profiles
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

fn push_reaction_for_declaration(
    expression: &GeneratedJavaExpression,
    fields: &BTreeMap<String, BlockPistonPushReaction>,
) -> Result<BlockPistonPushReaction> {
    let mut reaction = if let Some(base) = copy_base_from_register_helper(expression)? {
        *fields
            .get(base)
            .with_context(|| format!("stair base Blocks.{base} has no piston profile"))?
    } else if let Some(base) = copy_base_from_properties(expression)? {
        *fields
            .get(base)
            .with_context(|| format!("properties base Blocks.{base} has no piston profile"))?
    } else if expression.name.as_deref() == Some("registerBed") {
        BlockPistonPushReaction::Destroy
    } else {
        helper_push_reaction(expression).unwrap_or(BlockPistonPushReaction::Normal)
    };

    for call in expression.calls_in_source_order() {
        if call.name.as_deref() != Some("pushReaction") {
            continue;
        }
        let value = call
            .arguments
            .first()
            .context("pushReaction property lacks a value")?;
        reaction = parse_push_reaction(&value.canonical)?;
    }
    Ok(reaction)
}

fn helper_push_reaction(expression: &GeneratedJavaExpression) -> Option<BlockPistonPushReaction> {
    for call in expression.calls() {
        let reaction = match call.name.as_deref() {
            Some(
                "leavesProperties"
                | "shulkerBoxProperties"
                | "buttonProperties"
                | "flowerPotProperties"
                | "candleProperties",
            ) => BlockPistonPushReaction::Destroy,
            Some("pistonProperties") => BlockPistonPushReaction::Block,
            _ => continue,
        };
        return Some(reaction);
    }
    None
}

fn parse_push_reaction(value: &str) -> Result<BlockPistonPushReaction> {
    let value = value.replace(' ', "");
    match value.as_str() {
        "PushReaction.NORMAL" => Ok(BlockPistonPushReaction::Normal),
        "PushReaction.DESTROY" => Ok(BlockPistonPushReaction::Destroy),
        "PushReaction.BLOCK" => Ok(BlockPistonPushReaction::Block),
        "PushReaction.IGNORE" => Ok(BlockPistonPushReaction::Ignore),
        "PushReaction.PUSH_ONLY" => Ok(BlockPistonPushReaction::PushOnly),
        _ => bail!("unsupported piston push reaction {value}"),
    }
}

static BUILTIN_PISTON_PROFILES: OnceLock<std::result::Result<BlockPistonProfileCatalog, String>> =
    OnceLock::new();

pub fn builtin_block_piston_profiles() -> Result<&'static BlockPistonProfileCatalog> {
    match BUILTIN_PISTON_PROFILES.get_or_init(|| {
        BlockPistonProfileCatalog::load_builtin().map_err(|error| format!("{error:#}"))
    }) {
        Ok(catalog) => Ok(catalog),
        Err(error) => bail!(error.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_profiles_cover_every_block_and_vanilla_reaction_families() {
        let catalog = BlockPistonProfileCatalog::load_builtin().unwrap();
        assert_eq!(catalog.len(), 1168);
        assert_eq!(
            catalog.profile("minecraft:stone").unwrap(),
            &BlockPistonProfile {
                push_reaction: BlockPistonPushReaction::Normal,
                unbreakable: false,
                has_block_entity: false,
            }
        );
        assert_eq!(
            catalog
                .profile("minecraft:oak_leaves")
                .unwrap()
                .push_reaction,
            BlockPistonPushReaction::Destroy
        );
        assert_eq!(
            catalog
                .profile("minecraft:white_bed")
                .unwrap()
                .push_reaction,
            BlockPistonPushReaction::Destroy
        );
        assert_eq!(
            catalog
                .profile("minecraft:sticky_piston")
                .unwrap()
                .push_reaction,
            BlockPistonPushReaction::Block
        );
        assert_eq!(
            catalog
                .profile("minecraft:white_glazed_terracotta")
                .unwrap()
                .push_reaction,
            BlockPistonPushReaction::PushOnly
        );
        assert!(catalog.profile("minecraft:bedrock").unwrap().unbreakable);
        assert!(
            catalog
                .profile("minecraft:copper_chest")
                .unwrap()
                .has_block_entity
        );
        assert!(
            !catalog
                .profile("minecraft:note_block")
                .unwrap()
                .has_block_entity
        );
        assert_eq!(
            catalog
                .profiles()
                .values()
                .filter(|profile| profile.has_block_entity)
                .count(),
            201
        );
        for (reaction, expected) in [
            (BlockPistonPushReaction::Normal, 784),
            (BlockPistonPushReaction::Destroy, 355),
            (BlockPistonPushReaction::Block, 13),
            (BlockPistonPushReaction::Ignore, 0),
            (BlockPistonPushReaction::PushOnly, 16),
        ] {
            assert_eq!(
                catalog
                    .profiles()
                    .values()
                    .filter(|profile| profile.push_reaction == reaction)
                    .count(),
                expected,
                "unexpected {reaction:?} profile count"
            );
        }
    }

    #[test]
    fn builtin_profile_cache_reuses_the_validated_catalog() {
        let first = builtin_block_piston_profiles().unwrap();
        let second = builtin_block_piston_profiles().unwrap();
        assert!(std::ptr::eq(first, second));
    }
}
