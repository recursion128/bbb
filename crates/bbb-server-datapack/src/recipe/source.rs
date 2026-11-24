use bbb_protocol::{normalize_resource_location, JavaString};
use bbb_registry_codec::{
    registry_codec::RegistryReferenceLookup, PersistentItemStackHolderResolver,
    RegistryElementSource, RegistryElementValueRef,
};

use super::RecipeDecodeError;
use super::{
    other::{
        CookingKind, CookingRecipe, SmithingTransformRecipe, SmithingTrimRecipe, StonecuttingRecipe,
    },
    shaped::{ShapedRecipe, ShapedRecipePattern},
    shapeless::ShapelessRecipe,
    special::{
        BannerDuplicateRecipe, BookCloningRecipe, DecoratedPotRecipe, DyeRecipe,
        FireworkRocketRecipe, FireworkStarFadeRecipe, FireworkStarRecipe, ImbueRecipe,
        MapExtendingRecipe, ShieldDecorationRecipe, TransmuteRecipe,
    },
    Ingredient, Recipe, RecipeKind, RECIPE_SERIALIZER_TYPES_26_1,
};
use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
use bbb_protocol::recipe_wire_types::{
    VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_BLOCKS_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID, VANILLA_RECIPE_BOOK_CATEGORY_CAMPFIRE_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_REDSTONE_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID, VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_FOOD_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_MISC_ID, VANILLA_RECIPE_BOOK_CATEGORY_SMITHING_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_SMOKER_FOOD_ID, VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
};

pub(super) type Source<'a> = RegistryElementValueRef<'a>;

pub(super) fn error(message: impl Into<String>) -> RecipeDecodeError {
    RecipeDecodeError {
        message: message.into(),
    }
}

pub(super) fn required<'a>(root: Source<'a>, name: &str) -> Result<Source<'a>, RecipeDecodeError> {
    root.field(name)
        .ok_or_else(|| error(format!("recipe lacks `{name}`")))
}

pub(super) fn string(value: Source<'_>, field: &str) -> Result<JavaString, RecipeDecodeError> {
    value
        .java_string()
        .ok_or_else(|| error(format!("`{field}` must be a string")))
}

pub(super) fn resource(value: Source<'_>, field: &str) -> Result<String, RecipeDecodeError> {
    let raw = string(value, field)?;
    let raw = String::from_utf16(raw.units())
        .map_err(|_| error(format!("`{field}` is not a resource identifier")))?;
    normalize_resource_location(raw).map_err(|err| error(format!("`{field}`: {err}")))
}

pub(super) fn optional_bool(
    root: Source<'_>,
    name: &str,
    default: bool,
) -> Result<bool, RecipeDecodeError> {
    root.field(name)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| error(format!("`{name}` must be a bool")))
        })
        .unwrap_or(Ok(default))
}

pub(super) fn optional_string(
    root: Source<'_>,
    name: &str,
) -> Result<Option<JavaString>, RecipeDecodeError> {
    root.field(name)
        .map(|value| string(value, name).map(Some))
        .unwrap_or(Ok(None))
}

pub(super) fn optional_i32(
    root: Source<'_>,
    name: &str,
    default: i32,
) -> Result<i32, RecipeDecodeError> {
    root.field(name)
        .map(|value| {
            value
                .codec_i32()
                .ok_or_else(|| error(format!("`{name}` must be an int")))
        })
        .unwrap_or(Ok(default))
}

pub(super) fn optional_f32(
    root: Source<'_>,
    name: &str,
    default: f32,
) -> Result<f32, RecipeDecodeError> {
    root.field(name)
        .map(|value| {
            value
                .codec_f32()
                .ok_or_else(|| error(format!("`{name}` must be a number")))
        })
        .unwrap_or(Ok(default))
}

pub(super) fn entries<'a>(
    root: Source<'a>,
    field: &str,
) -> Result<impl ExactSizeIterator<Item = (&'a JavaString, Source<'a>)> + 'a, RecipeDecodeError> {
    let len = root
        .map_len()
        .ok_or_else(|| error(format!("`{field}` must be an object")))?;
    Ok((0..len).map(move |index| {
        root.map_entry_java_string_ref(index)
            .expect("map length and indexed entries agree")
    }))
}

pub(super) fn list<'a>(
    root: Source<'a>,
    field: &str,
) -> Result<impl ExactSizeIterator<Item = Source<'a>> + 'a, RecipeDecodeError> {
    let len = root
        .list_len()
        .ok_or_else(|| error(format!("`{field}` must be an array")))?;
    Ok((0..len).map(move |index| {
        root.list_element(index)
            .expect("list length and indexed elements agree")
    }))
}

pub(super) fn template(
    input: Source<'_>,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Result<super::ItemStackTemplate, RecipeDecodeError> {
    super::ItemStackTemplate::from_source(input, RegistryElementSource::ServerDatapackJson, lookup)
        .map_err(|err| error(err.message))
}

pub(super) fn parse_recipe_source(
    root: Source<'_>,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Result<Recipe, RecipeDecodeError> {
    if !root.is_compound() {
        return Err(error("recipe must be an object"));
    }
    let type_id = resource(required(root, "type")?, "type")?;
    if !RECIPE_SERIALIZER_TYPES_26_1.contains(&type_id.as_str()) {
        return Err(error(format!("unknown recipe serializer type {type_id}")));
    }
    let special = super::is_special_type(&type_id);
    let show_notification = if special {
        false
    } else {
        optional_bool(root, "show_notification", true)?
    };
    let group = if matches!(
        type_id.as_str(),
        "minecraft:crafting_shaped"
            | "minecraft:crafting_shapeless"
            | "minecraft:crafting_transmute"
            | "minecraft:crafting_dye"
            | "minecraft:crafting_imbue"
            | "minecraft:smelting"
            | "minecraft:blasting"
            | "minecraft:smoking"
            | "minecraft:campfire_cooking"
    ) {
        optional_string(root, "group")?.unwrap_or_default()
    } else {
        JavaString::default()
    };
    let category = if matches!(
        type_id.as_str(),
        "minecraft:crafting_shaped"
            | "minecraft:crafting_shapeless"
            | "minecraft:crafting_transmute"
            | "minecraft:crafting_dye"
            | "minecraft:crafting_imbue"
            | "minecraft:smelting"
            | "minecraft:blasting"
            | "minecraft:smoking"
            | "minecraft:campfire_cooking"
    ) {
        root.field("category")
            .map(resource_category)
            .transpose()?
            .unwrap_or_else(|| "misc".to_string())
    } else {
        "misc".to_string()
    };
    let category_id = category_id(&type_id, &category)?;
    let content_registry = lookup.content_registry();
    let kind = match type_id.as_str() {
        "minecraft:crafting_shaped" => {
            RecipeKind::Shaped(ShapedRecipe::parse_source(root, lookup)?)
        }
        "minecraft:crafting_shapeless" => {
            RecipeKind::Shapeless(ShapelessRecipe::parse_source(root, lookup)?)
        }
        "minecraft:crafting_transmute" => {
            // TransmuteRecipe.java:24-27.
            let (material_min, material_max) = bounds(root, "material_count", 1, 8, (1, 1))?;
            RecipeKind::Transmute(TransmuteRecipe {
                input: ingredient(root, "input", content_registry)?,
                material: ingredient(root, "material", content_registry)?,
                material_min,
                material_max,
                result: template(required(root, "result")?, lookup)?,
                add_material_count_to_result: optional_bool(
                    root,
                    "add_material_count_to_result",
                    false,
                )?,
            })
        }
        "minecraft:crafting_dye" => RecipeKind::Dye(DyeRecipe {
            target: ingredient(root, "target", content_registry)?,
            dye: ingredient(root, "dye", content_registry)?,
            result: template(required(root, "result")?, lookup)?,
        }),
        "minecraft:crafting_imbue" => RecipeKind::Imbue(ImbueRecipe {
            source: ingredient(root, "source", content_registry)?,
            material: ingredient(root, "material", content_registry)?,
            result: template(required(root, "result")?, lookup)?,
        }),
        "minecraft:crafting_decorated_pot" => RecipeKind::DecoratedPot(DecoratedPotRecipe {
            back: ingredient(root, "back", content_registry)?,
            left: ingredient(root, "left", content_registry)?,
            right: ingredient(root, "right", content_registry)?,
            front: ingredient(root, "front", content_registry)?,
            result: template(required(root, "result")?, lookup)?,
        }),
        "minecraft:crafting_special_bookcloning" => {
            // BookCloningRecipe.java:17-20.
            let (generation_min, generation_max) =
                bounds(root, "allowed_generations", 0, 2, (0, 1))?;
            RecipeKind::BookCloning(BookCloningRecipe {
                source: ingredient(root, "source", content_registry)?,
                material: ingredient(root, "material", content_registry)?,
                generation_min,
                generation_max,
                result: template(required(root, "result")?, lookup)?,
            })
        }
        "minecraft:crafting_special_mapextending" => RecipeKind::MapExtending(MapExtendingRecipe {
            pattern: ShapedRecipePattern::map_extending(
                ingredient(root, "material", content_registry)?,
                ingredient(root, "map", content_registry)?,
            ),
            result: template(required(root, "result")?, lookup)?,
        }),
        "minecraft:crafting_special_firework_rocket" => {
            RecipeKind::FireworkRocket(FireworkRocketRecipe {
                shell: ingredient(root, "shell", content_registry)?,
                fuel: ingredient(root, "fuel", content_registry)?,
                star: ingredient(root, "star", content_registry)?,
                result: template(required(root, "result")?, lookup)?,
            })
        }
        "minecraft:crafting_special_firework_star" => {
            let mut shapes = Vec::new();
            for (name, value) in entries(required(root, "shapes")?, "shapes")? {
                let name = String::from_utf16(name.units())
                    .map_err(|_| error("firework shape key is not a known shape"))?;
                let shape = FireworkExplosionShapeSummary::from_serialized_name(&name)
                    .ok_or_else(|| error(format!("unknown firework shape {name:?}")))?;
                shapes.push((
                    shape,
                    Ingredient::parse_source_with_content_registry(value, content_registry)?,
                ));
            }
            RecipeKind::FireworkStar(FireworkStarRecipe {
                shapes,
                trail: ingredient(root, "trail", content_registry)?,
                twinkle: ingredient(root, "twinkle", content_registry)?,
                fuel: ingredient(root, "fuel", content_registry)?,
                dye: ingredient(root, "dye", content_registry)?,
                result: template(required(root, "result")?, lookup)?,
            })
        }
        "minecraft:crafting_special_firework_star_fade" => {
            RecipeKind::FireworkStarFade(FireworkStarFadeRecipe {
                target: ingredient(root, "target", content_registry)?,
                dye: ingredient(root, "dye", content_registry)?,
                result: template(required(root, "result")?, lookup)?,
            })
        }
        "minecraft:crafting_special_bannerduplicate" => {
            RecipeKind::BannerDuplicate(BannerDuplicateRecipe {
                banner: ingredient(root, "banner", content_registry)?,
                result: template(required(root, "result")?, lookup)?,
            })
        }
        "minecraft:crafting_special_shielddecoration" => {
            RecipeKind::ShieldDecoration(ShieldDecorationRecipe {
                banner: ingredient(root, "banner", content_registry)?,
                target: ingredient(root, "target", content_registry)?,
                result: template(required(root, "result")?, lookup)?,
            })
        }
        "minecraft:crafting_special_repairitem" => RecipeKind::RepairItem,
        "minecraft:smelting" => RecipeKind::Cooking(CookingRecipe::parse_source(
            root,
            CookingKind::Smelting,
            lookup,
        )?),
        "minecraft:blasting" => RecipeKind::Cooking(CookingRecipe::parse_source(
            root,
            CookingKind::Blasting,
            lookup,
        )?),
        "minecraft:smoking" => RecipeKind::Cooking(CookingRecipe::parse_source(
            root,
            CookingKind::Smoking,
            lookup,
        )?),
        "minecraft:campfire_cooking" => RecipeKind::Cooking(CookingRecipe::parse_source(
            root,
            CookingKind::Campfire,
            lookup,
        )?),
        "minecraft:smithing_transform" => {
            RecipeKind::SmithingTransform(SmithingTransformRecipe::parse_source(root, lookup)?)
        }
        "minecraft:smithing_trim" => {
            RecipeKind::SmithingTrim(SmithingTrimRecipe::parse_source(root, content_registry)?)
        }
        "minecraft:stonecutting" => {
            RecipeKind::Stonecutting(StonecuttingRecipe::parse_source(root, lookup)?)
        }
        _ => unreachable!("validated recipe serializer"),
    };
    Ok(Recipe {
        type_id,
        kind,
        group,
        category_id,
        show_notification,
    })
}

fn resource_category(value: Source<'_>) -> Result<String, RecipeDecodeError> {
    let value = string(value, "category")?;
    String::from_utf16(value.units())
        .map_err(|_| error("`category` must be a valid category string"))
}

fn category_id(type_id: &str, category: &str) -> Result<i32, RecipeDecodeError> {
    let id = match type_id {
        "minecraft:crafting_shaped"
        | "minecraft:crafting_shapeless"
        | "minecraft:crafting_transmute"
        | "minecraft:crafting_dye"
        | "minecraft:crafting_imbue" => match category {
            "building" => VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID,
            "redstone" => VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_REDSTONE_ID,
            "equipment" => VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID,
            "misc" => VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
            _ => {
                return Err(error(format!(
                    "invalid category {category:?} for {type_id}"
                )))
            }
        },
        "minecraft:smelting" => match category {
            "food" => VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_FOOD_ID,
            "blocks" => VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID,
            "misc" => VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_MISC_ID,
            _ => {
                return Err(error(format!(
                    "invalid category {category:?} for {type_id}"
                )))
            }
        },
        "minecraft:blasting" => match category {
            "blocks" => VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_BLOCKS_ID,
            "food" | "misc" => VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID,
            _ => {
                return Err(error(format!(
                    "invalid category {category:?} for {type_id}"
                )))
            }
        },
        "minecraft:smoking" => match category {
            "food" | "blocks" | "misc" => VANILLA_RECIPE_BOOK_CATEGORY_SMOKER_FOOD_ID,
            _ => {
                return Err(error(format!(
                    "invalid category {category:?} for {type_id}"
                )))
            }
        },
        "minecraft:campfire_cooking" => match category {
            "food" | "blocks" | "misc" => VANILLA_RECIPE_BOOK_CATEGORY_CAMPFIRE_ID,
            _ => {
                return Err(error(format!(
                    "invalid category {category:?} for {type_id}"
                )))
            }
        },
        "minecraft:stonecutting" => VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
        "minecraft:smithing_transform" | "minecraft:smithing_trim" => {
            VANILLA_RECIPE_BOOK_CATEGORY_SMITHING_ID
        }
        _ => VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
    };
    Ok(id)
}

fn ingredient(
    root: Source<'_>,
    field: &str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<Ingredient, RecipeDecodeError> {
    Ingredient::parse_source_with_content_registry(required(root, field)?, content_registry)
}

/// `MinMaxBounds.Ints.CODEC` (`MinMaxBounds.java:270-272`) over
/// `Bounds.createCodec` (`:103-114`): either a record of optional `min`/`max`
/// or a bare number read as `exactly`, then `validateSwappedBoundsInCodec`
/// (`:54-58`) and `validateContainedInRange` (`:39-46`) against the closed
/// range the recipe allows. `Range.closed` encloses no range that leaves a side
/// unbounded, so a half-declared bound is refused rather than filled in.
fn bounds(
    root: Source<'_>,
    field: &str,
    min: i32,
    max: i32,
    default: (i32, i32),
) -> Result<(i32, i32), RecipeDecodeError> {
    let Some(value) = root.field(field) else {
        return Ok(default);
    };
    let pair = if value.is_compound() {
        (
            value.field("min").and_then(Source::codec_i32),
            value.field("max").and_then(Source::codec_i32),
        )
    } else {
        let exact = value.codec_i32();
        (exact, exact)
    };
    match pair {
        (Some(low), Some(high)) if low > high => Err(error(format!(
            "`{field}` has swapped bounds: {low} is higher than {high}"
        ))),
        (Some(low), Some(high)) if low <= high && low >= min && high <= max => Ok((low, high)),
        _ => Err(error(format!(
            "`{field}` must be a range within {min}..={max}"
        ))),
    }
}
