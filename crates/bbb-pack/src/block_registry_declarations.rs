use anyhow::{bail, Context, Result};
use bbb_protocol::block_types::VANILLA_BLOCK_TYPES;
use serde::Deserialize;

use crate::{generated_java_expression::GeneratedJavaExpression, resources::ResourceLocation};

const GENERATED_BLOCK_DECLARATIONS: &str =
    include_str!("../data/block_registry_declarations_26_1.json");

#[derive(Debug, Deserialize)]
pub(crate) struct BlockSourceCatalog {
    pub(crate) vanilla_version: String,
    pub(crate) sources: Vec<String>,
    pub(crate) declarations: Vec<BlockSourceDeclaration>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BlockSourceDeclaration {
    pub(crate) field: String,
    pub(crate) declaration_order: usize,
    pub(crate) resource_id: String,
    pub(crate) source_path: String,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) initializer: GeneratedJavaExpression,
}

pub(crate) fn load_generated_block_declarations() -> Result<BlockSourceCatalog> {
    let source: BlockSourceCatalog = serde_json::from_str(GENERATED_BLOCK_DECLARATIONS)
        .context("parse generated block registry declaration catalog")?;
    if source.vanilla_version != crate::MC_VERSION
        || source.sources
            != [
                "net/minecraft/references/BlockIds.java",
                "net/minecraft/world/level/block/Blocks.java",
                "net/minecraft/world/level/block/entity/BlockEntityType.java",
            ]
    {
        bail!("unsupported generated block source catalog manifest");
    }
    let mut resources = std::collections::BTreeSet::new();
    if source.declarations.len() != VANILLA_BLOCK_TYPES.len() {
        bail!(
            "generated block declaration count {} does not match protocol block registry count {}",
            source.declarations.len(),
            VANILLA_BLOCK_TYPES.len()
        );
    }
    for (order, declaration) in source.declarations.iter().enumerate() {
        validate_declaration(declaration, order)?;
        let protocol = &VANILLA_BLOCK_TYPES[order];
        if declaration.field != protocol.constant_name
            || declaration.resource_id != protocol.resource_id
        {
            bail!(
                "generated Blocks.{} registration does not match protocol block registry entry {}",
                declaration.field,
                order
            );
        }
        if !resources.insert(declaration.resource_id.as_str()) {
            bail!(
                "duplicate generated block resource id {}",
                declaration.resource_id
            );
        }
    }
    Ok(source)
}

pub(crate) fn validate_declaration(
    declaration: &BlockSourceDeclaration,
    order: usize,
) -> Result<()> {
    if declaration.declaration_order != order
        || declaration.source_path != "net/minecraft/world/level/block/Blocks.java"
        || declaration.start_byte >= declaration.end_byte
        || declaration.initializer.kind != "method_invocation"
    {
        bail!(
            "invalid generated block declaration order/span for Blocks.{}",
            declaration.field
        );
    }
    ResourceLocation::parse(&declaration.resource_id)
        .context("invalid generated block resource id")?;
    Ok(())
}

pub(crate) fn copy_base_from_register_helper(
    expression: &GeneratedJavaExpression,
) -> Result<Option<&str>> {
    if !matches!(
        expression.name.as_deref(),
        Some("registerLegacyStair" | "registerStair")
    ) {
        return Ok(None);
    }
    let base = expression
        .arguments
        .get(1)
        .context("stair registration lacks copied base block")?;
    if base.kind != "identifier" {
        bail!("stair registration base is not a block field");
    }
    Ok(Some(base.canonical.as_str()))
}

pub(crate) fn copy_base_from_properties(
    expression: &GeneratedJavaExpression,
) -> Result<Option<&str>> {
    let Some(copy) = expression
        .calls()
        .find(|call| matches!(call.name.as_deref(), Some("ofLegacyCopy" | "ofFullCopy")))
    else {
        return Ok(None);
    };
    let base = copy
        .arguments
        .first()
        .context("block properties copy lacks base block")?;
    if base.kind != "identifier" {
        bail!("block properties copy base is not a block field");
    }
    Ok(Some(base.canonical.as_str()))
}

pub(crate) fn has_properties_of(expression: &GeneratedJavaExpression) -> bool {
    expression.calls().any(|call| {
        call.name.as_deref() == Some("of")
            && call
                .object
                .as_deref()
                .map(|object| object.canonical.as_str())
                == Some("BlockBehaviour.Properties")
    })
}
