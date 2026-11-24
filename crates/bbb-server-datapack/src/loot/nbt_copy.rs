use bbb_protocol::CommandNbtPath;
use bbb_registry_codec::loot_context_types::VanillaLootContextParam;

use super::codec::{fail, id, list, string, At, Obj, Result, V};
use super::number::{entity_target, nbt_path};

/// `CopyCustomDataFunction`'s source and operations.
#[derive(Debug, Clone, PartialEq)]
pub struct LootNbtCopy {
    pub source: LootNbtProvider,
    pub operations: Vec<LootNbtCopyOperation>,
}

/// `NbtProviders.CODEC`: a bare context target, or a typed provider.
#[derive(Debug, Clone, PartialEq)]
pub enum LootNbtProvider {
    Context(VanillaLootContextParam),
    Storage(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LootNbtCopyOperation {
    pub source: CommandNbtPath,
    pub target: CommandNbtPath,
    pub strategy: LootNbtCopyStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LootNbtCopyStrategy {
    Replace,
    Append,
    Merge,
}

pub(super) fn decode(obj: &Obj<'_>) -> Result<LootNbtCopy> {
    Ok(LootNbtCopy {
        source: obj.req("source", provider)?,
        operations: obj.req("ops", |value| {
            list(value, |value| {
                let obj = Obj::of(value)?;
                Ok(LootNbtCopyOperation {
                    source: obj.req("source", nbt_path)?,
                    target: obj.req("target", nbt_path)?,
                    strategy: obj.req("op", |value| match string(value)?.as_str() {
                        "replace" => Ok(LootNbtCopyStrategy::Replace),
                        "append" => Ok(LootNbtCopyStrategy::Append),
                        "merge" => Ok(LootNbtCopyStrategy::Merge),
                        other => fail(format!("unknown copy operation {other}")),
                    })?,
                })
            })
        })?,
    })
}

fn provider(value: V<'_>) -> Result<LootNbtProvider> {
    if value.java_string_ref().is_some() {
        return context_target(value);
    }
    let obj = Obj::of(value)?;
    match obj.req("type", id)?.as_str() {
        "minecraft:context" => obj.req("target", context_target),
        "minecraft:storage" => obj.req("source", id).map(LootNbtProvider::Storage),
        other => fail(format!("unknown NBT provider type {other}")).at("type"),
    }
}

/// `ContextNbtProvider`'s target: the block entity or one entity slot.
fn context_target(value: V<'_>) -> Result<LootNbtProvider> {
    if value
        .java_string_ref()
        .is_some_and(|value| value == "block_entity")
    {
        return Ok(LootNbtProvider::Context(
            VanillaLootContextParam::BlockEntity,
        ));
    }
    entity_target(value).map(LootNbtProvider::Context)
}
