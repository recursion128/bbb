//! `BlockState.getCloneItemStack` and `addBlockDataToItem` for
//! `handlePickItemFromBlock` (ServerGamePacketListenerImpl.java:691-722).

use bbb_protocol::block_entity_types::vanilla_block_entity_type;
use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::block_types::{BlockTypeCatalogEntry, VanillaCloneItemStackOverride as Clone};
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::block_entity::{entry, BlockEntityItemProjection};
use crate::world::{LevelId, ServerWorld};

/// Tags each `removeComponentsFromTag` override discards; `item` is only
/// `DecoratedPotBlockEntity`'s (`:127-129`).
const COMPONENT_TAGS: &[&str] = &[
    "CustomName",
    "custom_name",
    "lock",
    "Items",
    "LootTable",
    "LootTableSeed",
    "conditionMet",
    "powered",
    "bees",
    "patterns",
    "sherds",
    "profile",
    "note_block_sound",
];

enum CloneBase {
    Empty,
    Item(String),
    /// The block's own item.
    Block,
}

/// The fixed item each override returns, by block family.
fn clone_base(block: &BlockTypeCatalogEntry, state: i32) -> CloneBase {
    let name = block.resource_id.trim_start_matches("minecraft:");
    let item = |id: &str| CloneBase::Item(format!("minecraft:{id}"));
    match block.clone_item_stack_override {
        None
        | Some(
            Clone::AbstractBannerBlock
            | Clone::BeehiveBlock
            | Clone::CopperGolemStatueBlock
            | Clone::DecoratedPotBlock
            | Clone::LightBlock
            | Clone::TestBlock,
        ) => CloneBase::Block,
        Some(
            Clone::EndGatewayBlock
            | Clone::EndPortalBlock
            | Clone::FrostedIceBlock
            | Clone::MovingPistonBlock
            | Clone::NetherPortalBlock,
        ) => CloneBase::Empty,
        // `getBaseSeedId` of each crop (CropBlock.java:169-171).
        Some(Clone::CropBlock) => item(match name {
            "carrots" => "carrot",
            "potatoes" => "potato",
            "beetroots" => "beetroot_seeds",
            "torchflower_crop" => "torchflower_seeds",
            _ => "wheat_seeds",
        }),
        Some(Clone::StemBlock | Clone::AttachedStemBlock) => item(if name.contains("pumpkin") {
            "pumpkin_seeds"
        } else {
            "melon_seeds"
        }),
        Some(Clone::NetherWartBlock) => item("nether_wart"),
        Some(Clone::SweetBerryBushBlock) => item("sweet_berries"),
        Some(Clone::CaveVinesBlock | Clone::CaveVinesPlantBlock) => item("glow_berries"),
        Some(Clone::BigDripleafStemBlock) => item("big_dripleaf"),
        Some(Clone::TallSeagrassBlock) => item("seagrass"),
        Some(Clone::BambooSaplingBlock) => item("bamboo"),
        Some(Clone::GrowingPlantBodyBlock) => item(name.trim_end_matches("_plant")),
        Some(Clone::CandleCakeBlock) => item("cake"),
        Some(Clone::PistonHeadBlock) => item(
            if vanilla_block_state_identity(state)
                .and_then(|identity| identity.properties.get("type"))
                .is_some_and(|kind| kind == "sticky")
            {
                "sticky_piston"
            } else {
                "piston"
            },
        ),
        // `this.potted` (FlowerPotBlock.java:104-106); the azalea pots name
        // their bush.
        Some(Clone::FlowerPotBlock) => match name.strip_prefix("potted_") {
            None => CloneBase::Block,
            Some(potted)
                if vanilla_item_type_for_resource_id(&format!("minecraft:{potted}")).is_some() =>
            {
                item(potted)
            }
            Some(potted) => item(potted.trim_end_matches("_bush")),
        },
    }
}

fn set(components: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    components.retain(|component| component.name != name);
    components.push(entry(name, value));
}

fn block_state_component(property: &str, value: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![entry(property, NbtSummaryValue::string(value))])
}

impl ServerWorld {
    /// `blockState.getCloneItemStack(level, pos, includeData)` then, with
    /// `includeData`, `addBlockDataToItem`. `None` is the empty stack.
    pub(super) fn clone_item_stack(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        state: i32,
        block: &BlockTypeCatalogEntry,
        include_data: bool,
    ) -> Option<RuntimeItemStack> {
        let item_id = match clone_base(block, state) {
            CloneBase::Empty => return None,
            CloneBase::Item(item) => item,
            // `Item.BY_BLOCK` has no entry for air and the like.
            CloneBase::Block => {
                let item = bbb_protocol::block_items::vanilla_block_item(block.id)?;
                bbb_protocol::item_types::vanilla_item_type(item)?
                    .resource_id
                    .to_string()
            }
        };
        let resolver = self.item_stack_resolver();
        let block_entity = self.level_by_id(dimension).chunks.block_entities().get(pos);
        let collected: Vec<NbtSummaryEntry> = block_entity
            .map(|block_entity| {
                block_entity
                    .collect_components(BlockEntityItemProjection::with_resolver(resolver.as_dyn()))
                    .entries()
                    .iter()
                    .filter_map(|component| {
                        Some(entry(&component.id, component.value.as_ref().ok()?.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let property = |name: &str| {
            vanilla_block_state_identity(state)
                .and_then(|identity| identity.properties.get(name))
                .map(String::as_str)
        };
        let mut components = Vec::new();
        match block.clone_item_stack_override {
            // `BannerBlockEntity.getItem` (:78-82).
            Some(Clone::AbstractBannerBlock) => components.extend(collected.iter().cloned()),
            // `CopperGolemStatueBlockEntity.getItem` (:49-53).
            Some(Clone::CopperGolemStatueBlock) if block_entity.is_some() => {
                components.extend(collected.iter().cloned());
                if let Some(pose) = property("copper_golem_pose") {
                    set(
                        &mut components,
                        "minecraft:block_state",
                        block_state_component("copper_golem_pose", pose),
                    );
                }
            }
            // `createDecoratedPotTemplate` (DecoratedPotBlockEntity.java:83-85).
            Some(Clone::DecoratedPotBlock) => components.extend(
                collected
                    .iter()
                    .filter(|component| component.name == "minecraft:pot_decorations")
                    .cloned(),
            ),
            Some(Clone::LightBlock) => {
                if let Some(level) = property("level") {
                    set(
                        &mut components,
                        "minecraft:block_state",
                        block_state_component("level", level),
                    );
                }
            }
            Some(Clone::TestBlock) => {
                if let Some(mode) = property("mode") {
                    set(
                        &mut components,
                        "minecraft:block_state",
                        block_state_component("mode", mode),
                    );
                }
            }
            Some(Clone::BeehiveBlock) if include_data => {
                if let Some(honey) = property("honey_level") {
                    set(
                        &mut components,
                        "minecraft:block_state",
                        block_state_component("honey_level", honey),
                    );
                }
            }
            _ => {}
        }
        if include_data {
            if let Some(block_entity) = block_entity {
                // `saveCustomOnly` minus `removeComponentsFromTag`, then
                // `BlockItem.setBlockEntityData` (BlockItem.java:206-214).
                let decorated_pot =
                    block.clone_item_stack_override == Some(Clone::DecoratedPotBlock);
                let mut data = block_entity
                    .save_custom_only_with_resolver(resolver.as_dyn())
                    .ok()?;
                data.retain(|field| {
                    field.name != "id"
                        && !COMPONENT_TAGS.iter().any(|tag| field.name == *tag)
                        && !(decorated_pot && field.name == "item")
                });
                if data.is_empty() {
                    components.retain(|component| component.name != "minecraft:block_entity_data");
                } else {
                    let type_id = vanilla_block_entity_type(block_entity.type_id())?.resource_id;
                    data.push(entry("id", NbtSummaryValue::string(type_id)));
                    set(
                        &mut components,
                        "minecraft:block_entity_data",
                        NbtSummaryValue::Compound(data),
                    );
                }
                for component in collected {
                    components.retain(|held| held.name != component.name);
                    components.push(component);
                }
            }
        }
        let saved = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string(item_id)),
            entry("count", NbtSummaryValue::Int(1)),
            entry("components", NbtSummaryValue::Compound(components)),
        ]);
        match RuntimeItemStack::try_read(&saved, &resolver) {
            Ok(stack) => Some(stack),
            Err(error) => {
                tracing::warn!("pick block stack: {error}");
                None
            }
        }
    }
}
