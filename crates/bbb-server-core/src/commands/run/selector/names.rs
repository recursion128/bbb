//! `Entity.getPlainTextName` (Nameable.java:9-11): `getName().getString()`
//! against the server's own language.

use bbb_protocol::block_types::{vanilla_block_type, vanilla_block_type_for_resource_id};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID,
};
use bbb_protocol::item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_TYPES};
use bbb_protocol::potion_types::VANILLA_POTIONS;
use bbb_protocol::{
    json_value_to_nbt_summary, vanilla_item_prototype_component_catalog, NbtSummaryEntry,
};
use bbb_server_datapack::language::component_plain_text;

use super::*;

pub(super) fn plain_text_name<'a>(
    entity: CommandDamageEntityRef<'_>,
    world: &EntityReads<'a>,
) -> Result<Vec<u16>, Refusal> {
    Ok(component_plain_text(&entity_name(entity, world)?))
}

pub(in crate::commands::run) fn entity_name<'a>(
    entity: CommandDamageEntityRef<'_>,
    world: &EntityReads<'a>,
) -> Result<NbtSummaryValue, Refusal> {
    let stored = match entity {
        // `Player.getName` (Player.java:1627-1629).
        CommandDamageEntityRef::Player(player) => return Ok(component_literal_nbt(player.name)),
        CommandDamageEntityRef::Stored(stored) => stored,
    };
    // `Entity.getName` (Entity.java:2881-2884); `ItemEntity.getName`
    // (ItemEntity.java:358-361) prefers the custom name the same way.
    if let Some(custom_name) = stored.entity.custom_name() {
        return Ok(custom_name.component().clone());
    }
    let name = match stored.entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_ITEM_ID => item_entity_name(stored, world)?,
        // `Villager.getTypeName` (Villager.java:702-704) over the profession's
        // registered name (VillagerProfession.java:112).
        VANILLA_ENTITY_TYPE_VILLAGER_ID => {
            let profession = saved(stored, world, &["VillagerData", "profession"])?;
            let NbtSummaryValue::String(profession) = profession else {
                return Err(not_projected());
            };
            profession
                .with_utf8(|profession| {
                    let (namespace, path) = split_id(profession);
                    translatable(format!("entity.{namespace}.villager.{path}"), Vec::new())
                })
                .ok_or_else(not_projected)?
        }
        // `FallingBlockEntity.getTypeName` (FallingBlockEntity.java:334-336).
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID => {
            let block = saved(stored, world, &["BlockState", "Name"])?;
            let NbtSummaryValue::String(block) = block else {
                return Err(not_projected());
            };
            translatable(
                "entity.minecraft.falling_block_type",
                vec![translatable(
                    &block
                        .with_utf8(|block| {
                            block_description_id_with_registry(block, world.content_registry())
                        })
                        .ok_or_else(not_projected)??,
                    Vec::new(),
                )],
            )
        }
        // `Entity.getTypeName` (Entity.java:2886-2888): `EntityType.getDescription`.
        _ => {
            let (namespace, path) = split_id(stored.entity.data().entity_resource_id());
            translatable(format!("entity.{namespace}.{path}"), Vec::new())
        }
    };
    Ok(name)
}

/// `ItemStack.getItemName` (ItemStack.java:826-828): `Item.getName(stack)`
/// (Item.java:341-343) and its overrides.
fn item_entity_name<'a>(
    stored: CommandStoredEntity<'_>,
    world: &EntityReads<'a>,
) -> Result<NbtSummaryValue, Refusal> {
    item_name_from_nbt_with_registry(&saved(stored, world, &["Item"])?, world.content_registry())
}

pub(in crate::commands::run) fn item_name_from_nbt_with_registry(
    item: &NbtSummaryValue,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue, Refusal> {
    let Some(NbtSummaryValue::String(id)) = compound_field(item, "id") else {
        return Err(not_projected());
    };
    id.with_utf8(|id| {
        let components = match compound_field(item, "components") {
            Some(NbtSummaryValue::Compound(entries)) => entries.as_slice(),
            None => &[],
            Some(_) => return Err(not_projected()),
        };
        let patched = |component: &str| -> Option<Option<&NbtSummaryValue>> {
            if find_entry(components, &format!("!{component}")).is_some() {
                return Some(None);
            }
            find_entry(components, component).map(Some)
        };
        let (namespace, path) = split_id(id);
        let item_description = format!("item.{namespace}.{path}");
        match id {
            // `PotionItem.getName` (PotionItem.java:73-76) and
            // `TippedArrowItem.getName` (TippedArrowItem.java:21-24): the prototype
            // of all four carries `PotionContents.EMPTY`.
            "minecraft:potion"
            | "minecraft:splash_potion"
            | "minecraft:lingering_potion"
            | "minecraft:tipped_arrow" => {
                if let Some(contents) =
                    patched("minecraft:potion_contents").unwrap_or(Some(&EMPTY_POTION))
                {
                    return Ok(translatable(
                        translation_key(
                            &format!("{item_description}.effect."),
                            &potion_suffix(contents)?,
                        ),
                        Vec::new(),
                    ));
                }
            }
            // `CompassItem.getName` (CompassItem.java:72-74).
            "minecraft:compass" => {
                if patched("minecraft:lodestone_tracker").flatten().is_some() {
                    return Ok(translatable("item.minecraft.lodestone_compass", Vec::new()));
                }
            }
            // `ShieldItem.getName` (ShieldItem.java:12-15).
            "minecraft:shield" => {
                if let Some(color) = patched("minecraft:base_color").flatten() {
                    let NbtSummaryValue::String(color) = color else {
                        return Err(not_projected());
                    };
                    return Ok(translatable(
                        translation_key(&format!("{item_description}."), color),
                        Vec::new(),
                    ));
                }
            }
            // `PlayerHeadItem.getName` (PlayerHeadItem.java:15-20), whose
            // description is the block's (Items.java:2355).
            "minecraft:player_head" => {
                let name = match patched("minecraft:profile").flatten() {
                    Some(NbtSummaryValue::String(name)) => Some(name),
                    Some(profile) => match compound_field(profile, "name") {
                        Some(NbtSummaryValue::String(name)) => Some(name),
                        _ => None,
                    },
                    None => None,
                };
                if let Some(name) = name {
                    return Ok(translatable(
                        "block.minecraft.player_head.named",
                        vec![NbtSummaryValue::string(name.clone())],
                    ));
                }
            }
            _ => {}
        }
        match patched("minecraft:item_name") {
            Some(Some(name)) => Ok(name.clone()),
            // `getOrDefault(ITEM_NAME, CommonComponents.EMPTY)`.
            Some(None) => Ok(NbtSummaryValue::string(String::new())),
            None => prototype_item_name_with_registry(id, content_registry),
        }
    })
    .unwrap_or_else(|| Err(not_projected()))
}

fn translation_key(prefix: &str, suffix: &bbb_protocol::JavaString) -> bbb_protocol::JavaString {
    let mut units = Vec::with_capacity(prefix.len() + suffix.units().len());
    units.extend(prefix.encode_utf16());
    units.extend_from_slice(suffix.units());
    units.into()
}

/// `PotionContents.EMPTY` as it is saved.
static EMPTY_POTION: NbtSummaryValue = NbtSummaryValue::Compound(Vec::new());

/// `PotionContents.getName`'s suffix (PotionContents.java:121-124): the custom
/// name, else the potion's base name, else `empty`.
fn potion_suffix(contents: &NbtSummaryValue) -> Result<bbb_protocol::JavaString, Refusal> {
    let (potion, custom_name) = match contents {
        // `PotionContents.CODEC`'s bare potion form.
        NbtSummaryValue::String(potion) => (Some(potion), None),
        NbtSummaryValue::Compound(_) => (
            match compound_field(contents, "potion") {
                Some(NbtSummaryValue::String(potion)) => Some(potion),
                None => None,
                Some(_) => return Err(not_projected()),
            },
            match compound_field(contents, "custom_name") {
                Some(NbtSummaryValue::String(name)) => Some(name),
                None => None,
                Some(_) => return Err(not_projected()),
            },
        ),
        _ => return Err(not_projected()),
    };
    if let Some(custom_name) = custom_name {
        return Ok(custom_name.clone());
    }
    let Some(potion) = potion else {
        return Ok("empty".into());
    };
    VANILLA_POTIONS
        .iter()
        .find(|entry| potion == entry.resource_id)
        .map(|entry| entry.name.into())
        .ok_or_else(not_projected)
}

/// The `DataComponents.ITEM_NAME` the item type registers.
fn prototype_item_name_with_registry(
    id: &str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue, Refusal> {
    if let Some(item) = vanilla_item_type_for_resource_id(id) {
        let name = vanilla_item_prototype_component_catalog()
            .ok()
            .and_then(|catalog| catalog.component_codec_values(item.id))
            .and_then(|components| components.get(&VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID))
            .ok_or_else(not_projected)?;
        return json_value_to_nbt_summary(name).map_err(|_| not_projected());
    }
    if let Some(custom) = content_registry.item_by_resource_id(id) {
        let (namespace, path) = split_id(custom.resource_id());
        let prefix = if custom.is_block_item {
            "block"
        } else {
            "item"
        };
        return Ok(translatable(
            format!("{prefix}.{namespace}.{path}"),
            Vec::new(),
        ));
    }
    Err(not_projected())
}

/// `Block.getDescriptionId`, which a wall variant built with
/// `wallVariant(standing, true)` takes from the standing block
/// (Blocks.java:7022-7029); the coral wall fans alone pass `false`.
pub(crate) fn block_description_id_opt(block: &str) -> Option<String> {
    let entry = vanilla_block_type_for_resource_id(block)?;
    let standing = VANILLA_ITEM_TYPES
        .iter()
        .filter_map(|item| item.block_item_behavior)
        .find(|behavior| behavior.wall_block_type_id == Some(entry.id))
        .filter(|_| !block.ends_with("_coral_wall_fan"))
        .and_then(|behavior| vanilla_block_type(behavior.block_type_id))
        .map_or(entry.resource_id, |standing| standing.resource_id);
    let (namespace, path) = split_id(standing);
    Some(format!("block.{namespace}.{path}"))
}

fn block_description_id_with_registry(
    block: &str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<String, Refusal> {
    if let Some(id) = block_description_id_opt(block) {
        return Ok(id);
    }
    let custom = content_registry
        .block_by_resource_id(block)
        .ok_or_else(not_projected)?;
    let (namespace, path) = split_id(custom.resource_id());
    Ok(format!("block.{namespace}.{path}"))
}

/// The field at `path` of this entity's `/data get entity` compound.
fn saved<'a>(
    stored: CommandStoredEntity<'_>,
    world: &EntityReads<'a>,
    path: &[&str],
) -> Result<NbtSummaryValue, Refusal> {
    let data = world
        .entity_data(stored.entity.uuid())
        .map_err(super::unanswerable)?
        .ok_or_else(not_projected)?;
    let mut value = &data;
    for field in path {
        value = compound_field(value, field).ok_or_else(not_projected)?;
    }
    Ok(value.clone())
}

fn compound_field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    match value {
        NbtSummaryValue::Compound(entries) => find_entry(entries, name),
        _ => None,
    }
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn split_id(id: &str) -> (&str, &str) {
    id.split_once(':').unwrap_or(("minecraft", id))
}

fn not_projected() -> Refusal {
    unanswerable("The saved data this entity's name is read from is not projected")
}
