#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 item declaration and protocol catalogs."""

from __future__ import annotations

import argparse
import difflib
import json
import subprocess
import sys
from pathlib import Path

from block_registry_catalog_extractor import extract_block_registry_catalog
from item_registry_catalog_extractor import (
    extract_block_item_behaviors,
    extract_bucket_item_behaviors,
    extract_item_interact_living_entity_override_owners,
    extract_item_registry_catalog,
    extract_tool_use_on_catalog,
    extract_item_use_on_override_owners,
    extract_item_use_override_owners,
    item_factory_class,
)
from sound_event_catalog_extractor import direct_sound_event_entries
from vanilla_java_source import expression_fact, parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_FACTS_OUTPUT = REPO_ROOT / "crates/bbb-pack/data/item_registry_declarations_26_1.json"
DEFAULT_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/item_types.rs"
ITEMS_PATH = Path("net/minecraft/world/item/Items.java")
ITEM_PATH = Path("net/minecraft/world/item/Item.java")
ITEM_IDS_PATH = Path("net/minecraft/references/ItemIds.java")
FOODS_PATH = Path("net/minecraft/world/food/Foods.java")
DATA_COMPONENTS_PATH = Path("net/minecraft/core/component/DataComponents.java")
SOUND_EVENTS_PATH = Path("net/minecraft/sounds/SoundEvents.java")
ITEM_PACKAGE_PATH = Path("net/minecraft/world/item")
BLOCKS_PATH = Path("net/minecraft/world/level/block/Blocks.java")
BLOCK_IDS_PATH = Path("net/minecraft/references/BlockIds.java")
BLOCK_ENTITY_TYPES_PATH = Path(
    "net/minecraft/world/level/block/entity/BlockEntityType.java"
)
WEATHERING_COPPER_PATH = Path(
    "net/minecraft/world/level/block/WeatheringCopper.java"
)
WEATHERING_COPPER_BLOCKS_PATH = Path(
    "net/minecraft/world/level/block/WeatheringCopperBlocks.java"
)
EXPECTED_TOOL_USE_ON_COUNTS = {
    "axe_strippables": 23,
    "axe_scrapeables": 45,
    "axe_wax_off": 60,
    "honeycomb_waxables": 60,
    "hoe_tillables": 5,
    "shovel_flattenables": 6,
}
EXPECTED_BLOCK_ITEM_BEHAVIOR_COUNT = 1023


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--facts-output", type=Path, default=DEFAULT_FACTS_OUTPUT)
    parser.add_argument("--types-output", type=Path, default=DEFAULT_TYPES_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def _field_access(fact: dict, owner: str) -> str | None:
    if fact.get("kind") != "field_access" or fact.get("name") is None:
        return None
    object_fact = fact.get("object")
    if object_fact and object_fact.get("canonical") == owner:
        return fact["name"]
    return None


def _string(fact: dict) -> str | None:
    if fact.get("kind") != "string_literal":
        return None
    value = json.loads(fact["canonical"])
    return value if isinstance(value, str) else None


def _resource_ids(declaration: dict, constants: dict[str, str]) -> list[str]:
    initializer = declaration["initializer"]
    name = initializer.get("name")
    arguments = initializer.get("arguments", [])
    field = declaration["field"]
    if declaration["field_type"] == "WeatheringCopperItems":
        if name != "create" or not arguments:
            raise ValueError(f"Items.{field}: unsupported weathering registration")
        block = _field_access(arguments[0], "Blocks")
        if block is None:
            raise ValueError(f"Items.{field}: weathering registration lacks Blocks constant")
        base = block.lower()
        return [
            base,
            f"exposed_{base}",
            f"weathered_{base}",
            f"oxidized_{base}",
            f"waxed_{base}",
            f"waxed_exposed_{base}",
            f"waxed_weathered_{base}",
            f"waxed_oxidized_{base}",
        ]
    if name == "registerBlock" and arguments:
        block = _field_access(arguments[0], "Blocks")
        if block is not None:
            return [block.lower()]
    if name == "registerSpawnEgg" and arguments:
        entity = _field_access(arguments[0], "EntityType")
        if entity is not None:
            return [entity.lower() + "_spawn_egg"]
    if name == "registerItem" and arguments:
        literal = _string(arguments[0])
        if literal is not None:
            return [literal]
        constant = _field_access(arguments[0], "ItemIds")
        if constant is not None:
            resource_id = constants.get(constant)
            if resource_id is None:
                raise ValueError(
                    f"Items.{field}: unresolved ItemIds.{constant} registration key"
                )
            return [resource_id]
    raise ValueError(f"Items.{field}: unsupported root registration {name!r}")


def _walk_facts(expression: dict):
    yield expression
    object_fact = expression.get("object")
    if object_fact is not None:
        yield from _walk_facts(object_fact)
    for key in ("arguments", "children"):
        for child in expression.get(key, []):
            yield from _walk_facts(child)


def _equip_on_interact_owner(declaration: dict) -> str | None:
    matches = []
    for fact in _walk_facts(declaration["initializer"]):
        if fact.get("kind") != "method_invocation":
            continue
        object_fact = fact.get("object")
        if object_fact is None or object_fact.get("canonical") != "Equippable":
            continue
        if fact.get("name") == "saddle":
            matches.append("Saddle")
        elif fact.get("name") == "harness":
            matches.append("Harness")
    if len(matches) > 1:
        raise ValueError(
            f"Items.{declaration['field']}: multiple equip-on-interact owners: {matches}"
        )
    return matches[0] if matches else None


def _break_sound_constant(expression: dict, call_names: set[str]) -> str | None:
    matches = []
    for fact in _walk_facts(expression):
        if fact.get("kind") != "method_invocation" or fact.get("name") not in call_names:
            continue
        arguments = fact.get("arguments", [])
        if len(arguments) < 2:
            continue
        component = _field_access(arguments[0], "DataComponents")
        if component is None and arguments[0].get("canonical") == "BREAK_SOUND":
            component = "BREAK_SOUND"
        sound = _field_access(arguments[1], "SoundEvents")
        if component == "BREAK_SOUND" and sound is not None:
            matches.append(sound)
    if len(matches) > 1:
        raise ValueError(f"multiple break-sound component assignments: {matches}")
    return matches[0] if matches else None


def _method_break_sound_constant(source, owner: str, method_name: str) -> str:
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == method_name and method.body is not None
    ]
    if len(methods) != 1:
        raise ValueError(f"expected exactly one {owner}.{method_name} method")
    method = methods[0]
    matches = []
    for call in source.calls_for(owner):
        if (
            call.name == "component"
            and method.body.start_byte <= call.span.start_byte
            and call.span.end_byte <= method.body.end_byte
        ):
            fact = expression_fact(source, source.expression(call.node))
            sound = _break_sound_constant(fact, {"component"})
            if sound is not None:
                matches.append(sound)
    if len(matches) != 1:
        raise ValueError(
            f"expected exactly one break-sound assignment in {owner}.{method_name}, got {matches}"
        )
    return matches[0]


def _default_break_sounds(
    declarations: list[dict],
    item_id_constants: dict[str, str],
    item_source,
    data_components_source,
    sound_events_source,
) -> tuple[str, dict[str, str]]:
    sounds = {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(sound_events_source)
    }
    common_fields = [
        field
        for field in data_components_source.fields_for(
            "net.minecraft.core.component.DataComponents"
        )
        if field.name == "COMMON_ITEM_COMPONENTS" and field.initializer is not None
    ]
    if len(common_fields) != 1:
        raise ValueError("expected exactly one DataComponents.COMMON_ITEM_COMPONENTS field")
    common_constant = _break_sound_constant(
        expression_fact(data_components_source, common_fields[0].initializer), {"set"}
    )
    if common_constant is None:
        raise ValueError("COMMON_ITEM_COMPONENTS lacks a BREAK_SOUND assignment")
    common_sound = sounds[common_constant]
    wolf_armor_constant = _method_break_sound_constant(
        item_source, "net.minecraft.world.item.Item.Properties", "wolfArmor"
    )

    overrides: dict[str, str] = {}
    for declaration in declarations:
        sound_constant = _break_sound_constant(
            declaration["initializer"], {"component"}
        )
        if sound_constant is None and any(
            fact.get("kind") == "method_invocation" and fact.get("name") == "wolfArmor"
            for fact in _walk_facts(declaration["initializer"])
        ):
            sound_constant = wolf_armor_constant
        if sound_constant is None:
            continue
        sound = sounds[sound_constant]
        if sound == common_sound:
            continue
        for resource_id in _resource_ids(declaration, item_id_constants):
            overrides["minecraft:" + resource_id] = sound
    return common_sound, overrides


def _render_types(
    resource_paths: list[str],
    use_override_owners: list[str | None],
    use_on_override_owners: list[str | None],
    interact_living_entity_override_owners: list[str | None],
    equip_on_interact_owners: list[str | None],
    bucket_behaviors: list[dict[str, str | None] | None],
    block_item_behaviors: list[dict[str, str | None] | None],
    item_constants: dict[str, int],
    common_break_sound: str,
    break_sound_overrides: dict[str, str],
    tool_use_on_catalog: dict[str, list[dict]],
) -> str:
    if len(resource_paths) != 1506:
        raise ValueError(f"expected 1506 vanilla item ids, got {len(resource_paths)}")
    resources = ["minecraft:" + path for path in resource_paths]
    if len(set(resources)) != len(resources):
        raise ValueError("duplicate vanilla item resource id")
    for behavior, owners in (
        ("use", use_override_owners),
        ("useOn", use_on_override_owners),
        ("interactLivingEntity", interact_living_entity_override_owners),
        ("Equippable.equipOnInteract", equip_on_interact_owners),
    ):
        if len(owners) != len(resources):
            raise ValueError(
                f"item {behavior}-override catalog length does not match item ids"
            )
    if len(bucket_behaviors) != len(resources):
        raise ValueError("bucket behavior catalog length does not match item ids")
    if len(block_item_behaviors) != len(resources):
        raise ValueError("block item behavior catalog length does not match item ids")
    use_override_variants = sorted(
        {owner for owner in use_override_owners if owner is not None}
    )
    use_override_count = sum(owner is not None for owner in use_override_owners)
    use_on_override_variants = sorted(
        {owner for owner in use_on_override_owners if owner is not None}
    )
    use_on_override_count = sum(
        owner is not None for owner in use_on_override_owners
    )
    interact_living_entity_override_variants = sorted(
        {
            owner
            for owner in interact_living_entity_override_owners
            if owner is not None
        }
    )
    interact_living_entity_override_count = sum(
        owner is not None for owner in interact_living_entity_override_owners
    )
    equip_on_interact_variants = sorted(
        {owner for owner in equip_on_interact_owners if owner is not None}
    )
    equip_on_interact_count = sum(
        owner is not None for owner in equip_on_interact_owners
    )
    lines = [
        "// @generated by scripts/generate-item-registry-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 item registrations/overrides, BlockItem registrations, tool transform maps, block registrations/weathering groups, component defaults, and sound ids through the shared Tree-sitter Java frontend.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-item-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-item-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaItemUseOverride {",
    ]
    lines.extend(f"    {owner}," for owner in use_override_variants)
    lines.extend([
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaItemUseOnOverride {",
    ])
    lines.extend(f"    {owner}," for owner in use_on_override_variants)
    lines.extend([
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaItemInteractLivingEntityOverride {",
    ])
    lines.extend(
        f"    {owner},"
        for owner in interact_living_entity_override_variants
    )
    lines.extend([
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaItemEquipOnInteract {",
    ])
    lines.extend(f"    {owner}," for owner in equip_on_interact_variants)
    lines.extend([
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaBucketFluid {",
        "    Empty,",
        "    Water,",
        "    Lava,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaBucketItemBehavior {",
        "    pub fluid: VanillaBucketFluid,",
        "    pub mob_empty_sound_resource_id: Option<&'static str>,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaBlockItemKind {",
        "    BannerItem,",
        "    BedItem,",
        "    BlockItem,",
        "    DoubleHighBlockItem,",
        "    GameMasterBlockItem,",
        "    HangingSignItem,",
        "    PlayerHeadItem,",
        "    ScaffoldingBlockItem,",
        "    SignItem,",
        "    StandingAndWallBlockItem,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaBlockItemBehavior {",
        "    pub kind: VanillaBlockItemKind,",
        "    pub block_type_id: i32,",
        "    pub wall_block_type_id: Option<i32>,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ItemTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub resource_id: &'static str,",
        "    pub use_override: Option<VanillaItemUseOverride>,",
        "    pub use_on_override: Option<VanillaItemUseOnOverride>,",
        "    pub interact_living_entity_override: Option<VanillaItemInteractLivingEntityOverride>,",
        "    pub equip_on_interact: Option<VanillaItemEquipOnInteract>,",
        "    pub bucket_behavior: Option<VanillaBucketItemBehavior>,",
        "    pub block_item_behavior: Option<VanillaBlockItemBehavior>,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaBlockTransform {",
        "    pub source_block_type_id: i32,",
        "    pub target_block_type_id: i32,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaHoeTillable {",
        "    pub transform: VanillaBlockTransform,",
        "    pub requires_air_above: bool,",
        "}",
        "",
        f"pub const VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE: i32 = {item_constants['DEFAULT_MAX_STACK_SIZE']};",
        f"pub const VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE: i32 = {item_constants['ABSOLUTE_MAX_STACK_SIZE']};",
        f'pub const VANILLA_ITEM_DEFAULT_BREAK_SOUND_RESOURCE_ID: &str = "{common_break_sound}";',
        "",
    ])
    for item_id, resource_id in enumerate(resources):
        suffix = resource_id.removeprefix("minecraft:").upper()
        lines.append(f"pub const VANILLA_ITEM_{suffix}_ID: i32 = {item_id};")
    for constant_name, catalog_name in (
        ("VANILLA_AXE_STRIPPABLES", "axe_strippables"),
        ("VANILLA_AXE_SCRAPEABLES", "axe_scrapeables"),
        ("VANILLA_AXE_WAX_OFF", "axe_wax_off"),
        ("VANILLA_HONEYCOMB_WAXABLES", "honeycomb_waxables"),
        ("VANILLA_SHOVEL_FLATTENABLES", "shovel_flattenables"),
    ):
        lines.extend(("", f"pub const {constant_name}: &[VanillaBlockTransform] = &["))
        for entry in tool_use_on_catalog[catalog_name]:
            lines.extend(
                (
                    "    VanillaBlockTransform {",
                    "        source_block_type_id: "
                    f"crate::block_types::VANILLA_BLOCK_{entry['source']}_ID,",
                    "        target_block_type_id: "
                    f"crate::block_types::VANILLA_BLOCK_{entry['target']}_ID,",
                    "    },",
                )
            )
        lines.append("];")
    lines.extend(("", "pub const VANILLA_HOE_TILLABLES: &[VanillaHoeTillable] = &["))
    for entry in tool_use_on_catalog["hoe_tillables"]:
        lines.extend(
            (
                "    VanillaHoeTillable {",
                "        transform: VanillaBlockTransform {",
                "            source_block_type_id: "
                f"crate::block_types::VANILLA_BLOCK_{entry['source']}_ID,",
                "            target_block_type_id: "
                f"crate::block_types::VANILLA_BLOCK_{entry['target']}_ID,",
                "        },",
                f"        requires_air_above: {str(entry['requires_air_above']).lower()},",
                "    },",
            )
        )
    lines.append("];")
    lines.extend(("", "pub const VANILLA_ITEM_TYPES: &[ItemTypeCatalogEntry] = &["))
    for item_id, (
        resource_id,
        use_override_owner,
        use_on_override_owner,
        interact_living_entity_override_owner,
        equip_on_interact_owner,
        bucket_behavior,
        block_item_behavior,
    ) in enumerate(
        zip(
            resources,
            use_override_owners,
            use_on_override_owners,
            interact_living_entity_override_owners,
            equip_on_interact_owners,
            bucket_behaviors,
            block_item_behaviors,
            strict=True,
        )
    ):
        lines.extend((
            "    ItemTypeCatalogEntry {",
            f"        id: {item_id},",
            f'        resource_id: "{resource_id}",',
            (
                f"        use_override: Some(VanillaItemUseOverride::{use_override_owner}),"
                if use_override_owner is not None
                else "        use_override: None,"
            ),
            (
                f"        use_on_override: Some(VanillaItemUseOnOverride::{use_on_override_owner}),"
                if use_on_override_owner is not None
                else "        use_on_override: None,"
            ),
            (
                "        interact_living_entity_override: "
                f"Some(VanillaItemInteractLivingEntityOverride::{interact_living_entity_override_owner}),"
                if interact_living_entity_override_owner is not None
                else "        interact_living_entity_override: None,"
            ),
            (
                "        equip_on_interact: "
                f"Some(VanillaItemEquipOnInteract::{equip_on_interact_owner}),"
                if equip_on_interact_owner is not None
                else "        equip_on_interact: None,"
            ),
            (
                "        bucket_behavior: Some(VanillaBucketItemBehavior { "
                f"fluid: VanillaBucketFluid::{bucket_behavior['fluid'].title()}, "
                "mob_empty_sound_resource_id: "
                + (
                    f"Some(\"{bucket_behavior['mob_empty_sound']}\")"
                    if bucket_behavior["mob_empty_sound"] is not None
                    else "None"
                )
                + " }),"
                if bucket_behavior is not None
                else "        bucket_behavior: None,"
            ),
            (
                "        block_item_behavior: Some(VanillaBlockItemBehavior { "
                f"kind: VanillaBlockItemKind::{block_item_behavior['kind']}, "
                "block_type_id: "
                f"crate::block_types::VANILLA_BLOCK_{block_item_behavior['block_field']}_ID, "
                "wall_block_type_id: "
                + (
                    "Some("
                    f"crate::block_types::VANILLA_BLOCK_{block_item_behavior['wall_block_field']}_ID"
                    ")"
                    if block_item_behavior["wall_block_field"] is not None
                    else "None"
                )
                + " }),"
                if block_item_behavior is not None
                else "        block_item_behavior: None,"
            ),
            "    },",
        ))
    lines.extend((
        "];", "",
        "pub fn vanilla_item_type(id: i32) -> Option<&'static ItemTypeCatalogEntry> {",
        "    VANILLA_ITEM_TYPES.get(usize::try_from(id).ok()?)", "}", "",
        "pub fn vanilla_item_type_for_resource_id(resource_id: &str) -> Option<&'static ItemTypeCatalogEntry> {",
        '    let resource_id = resource_id.strip_prefix("minecraft:").unwrap_or(resource_id);',
        "    VANILLA_ITEM_TYPES.iter().find(|entry| entry.resource_id.strip_prefix(\"minecraft:\") == Some(resource_id))",
        "}", "",
        "pub fn vanilla_item_resource_id_matches_type(resource_id: &str, item_type_id: i32) -> bool {",
        "    vanilla_item_type(item_type_id).is_some_and(|entry| entry.resource_id == resource_id)",
        "}", "",
        "pub fn vanilla_item_default_break_sound_resource_id(item_type_id: i32) -> Option<&'static str> {",
        "    vanilla_item_type(item_type_id)?;",
        "    Some(match item_type_id {",
    ))
    for resource_id, sound in sorted(
        break_sound_overrides.items(), key=lambda entry: resources.index(entry[0])
    ):
        suffix = resource_id.removeprefix("minecraft:").upper()
        lines.append(f'        VANILLA_ITEM_{suffix}_ID => "{sound}",')
    lines.extend((
        "        _ => VANILLA_ITEM_DEFAULT_BREAK_SOUND_RESOURCE_ID,",
        "    })",
        "}", "",
        "#[cfg(test)]", "mod tests {", "    use std::{collections::HashSet, fs, path::Path};", "", "    use super::*;", "",
        "    #[test]", "    fn item_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_ITEM_TYPES.len(), 1506);", "        let mut resources = HashSet::new();",
        "        for (id, entry) in VANILLA_ITEM_TYPES.iter().enumerate() {", "            assert_eq!(entry.id, id as i32);",
        '            assert!(entry.resource_id.starts_with("minecraft:"));', "            assert!(resources.insert(entry.resource_id));",
        "        }", "    }", "", "    #[test]", "    fn item_catalog_preserves_vanilla_registration_order() {",
        "        assert_eq!(VANILLA_ITEM_AIR_ID, 0);", "        assert_eq!(VANILLA_ITEM_SADDLE_ID, 838);",
        "        assert_eq!(VANILLA_ITEM_ELYTRA_ID, 863);", "        assert_eq!(VANILLA_ITEM_SNOWBALL_ID, 1017);",
        "        assert_eq!(VANILLA_ITEM_EGG_ID, 1032);", "        assert_eq!(VANILLA_ITEM_ENDER_EYE_ID, 1129);",
        "        assert_eq!(VANILLA_ITEM_SPLASH_POTION_ID, 1292);", "        assert_eq!(VANILLA_ITEM_OMINOUS_BOTTLE_ID, 1505);",
        "        assert_eq!(vanilla_item_type(-1), None);", "        assert_eq!(vanilla_item_type(1506), None);",
        f"        assert_eq!(VANILLA_ITEM_TYPES.iter().filter(|entry| entry.use_override.is_some()).count(), {use_override_count});",
        f"        assert_eq!(VANILLA_ITEM_TYPES.iter().filter(|entry| entry.use_on_override.is_some()).count(), {use_on_override_count});",
        f"        assert_eq!(VANILLA_ITEM_TYPES.iter().filter(|entry| entry.interact_living_entity_override.is_some()).count(), {interact_living_entity_override_count});",
        f"        assert_eq!(VANILLA_ITEM_TYPES.iter().filter(|entry| entry.equip_on_interact.is_some()).count(), {equip_on_interact_count});",
        f"        assert_eq!(VANILLA_ITEM_TYPES.iter().filter(|entry| entry.bucket_behavior.is_some()).count(), {sum(behavior is not None for behavior in bucket_behaviors)});",
        f"        assert_eq!(VANILLA_ITEM_TYPES.iter().filter(|entry| entry.block_item_behavior.is_some()).count(), {sum(behavior is not None for behavior in block_item_behaviors)});",
        "        assert!(VANILLA_ITEM_TYPES.iter().all(|entry| entry.block_item_behavior.is_some() == matches!(entry.use_on_override, Some(VanillaItemUseOnOverride::BlockItem))));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_STONE_ID).unwrap().use_override, None);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_STONE_ID).unwrap().use_on_override, Some(VanillaItemUseOnOverride::BlockItem));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_STONE_ID).unwrap().block_item_behavior, Some(VanillaBlockItemBehavior { kind: VanillaBlockItemKind::BlockItem, block_type_id: crate::block_types::VANILLA_BLOCK_STONE_ID, wall_block_type_id: None }));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_OAK_SIGN_ID).unwrap().block_item_behavior, Some(VanillaBlockItemBehavior { kind: VanillaBlockItemKind::SignItem, block_type_id: crate::block_types::VANILLA_BLOCK_OAK_SIGN_ID, wall_block_type_id: Some(crate::block_types::VANILLA_BLOCK_OAK_WALL_SIGN_ID) }));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_REDSTONE_ID).unwrap().block_item_behavior.unwrap().block_type_id, crate::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_EXPOSED_COPPER_BARS_ID).unwrap().block_item_behavior.unwrap().block_type_id, crate::block_types::VANILLA_BLOCK_EXPOSED_COPPER_BARS_ID);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_TALL_GRASS_ID).unwrap().block_item_behavior.unwrap().kind, VanillaBlockItemKind::DoubleHighBlockItem);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_COMMAND_BLOCK_ID).unwrap().block_item_behavior.unwrap().kind, VanillaBlockItemKind::GameMasterBlockItem);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_SCAFFOLDING_ID).unwrap().block_item_behavior.unwrap().kind, VanillaBlockItemKind::ScaffoldingBlockItem);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_STONE_ID).unwrap().interact_living_entity_override, None);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_RED_DYE_ID).unwrap().interact_living_entity_override, Some(VanillaItemInteractLivingEntityOverride::DyeItem));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_SADDLE_ID).unwrap().equip_on_interact, Some(VanillaItemEquipOnInteract::Saddle));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_WHITE_HARNESS_ID).unwrap().equip_on_interact, Some(VanillaItemEquipOnInteract::Harness));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_ELYTRA_ID).unwrap().equip_on_interact, None);",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_CARROT_ON_A_STICK_ID).unwrap().use_override, Some(VanillaItemUseOverride::FoodOnAStickItem));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_SNOWBALL_ID).unwrap().use_override, Some(VanillaItemUseOverride::SnowballItem));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_WRITABLE_BOOK_ID).unwrap().use_override, Some(VanillaItemUseOverride::WritableBookItem));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_BUCKET_ID).unwrap().bucket_behavior, Some(VanillaBucketItemBehavior { fluid: VanillaBucketFluid::Empty, mob_empty_sound_resource_id: None }));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_WATER_BUCKET_ID).unwrap().bucket_behavior, Some(VanillaBucketItemBehavior { fluid: VanillaBucketFluid::Water, mob_empty_sound_resource_id: None }));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_LAVA_BUCKET_ID).unwrap().bucket_behavior, Some(VanillaBucketItemBehavior { fluid: VanillaBucketFluid::Lava, mob_empty_sound_resource_id: None }));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_COD_BUCKET_ID).unwrap().bucket_behavior, Some(VanillaBucketItemBehavior { fluid: VanillaBucketFluid::Water, mob_empty_sound_resource_id: Some(\"minecraft:item.bucket.empty_fish\") }));",
        "        assert_eq!(vanilla_item_type(VANILLA_ITEM_MILK_BUCKET_ID).unwrap().bucket_behavior, None);",
        '        assert_eq!(vanilla_item_type_for_resource_id("snowball").unwrap().id, VANILLA_ITEM_SNOWBALL_ID);',
        '        assert_eq!(vanilla_item_type_for_resource_id("example:snowball"), None);',
        '        assert!(vanilla_item_resource_id_matches_type("minecraft:snowball", VANILLA_ITEM_SNOWBALL_ID));',
        '        assert!(!vanilla_item_resource_id_matches_type("snowball", VANILLA_ITEM_SNOWBALL_ID));',
        '        assert!(!vanilla_item_resource_id_matches_type("minecraft:snowball", VANILLA_ITEM_EGG_ID));',
        "        assert_eq!(vanilla_item_default_break_sound_resource_id(VANILLA_ITEM_STONE_ID), Some(\"minecraft:entity.item.break\"));",
        "        assert_eq!(vanilla_item_default_break_sound_resource_id(VANILLA_ITEM_SHIELD_ID), Some(\"minecraft:item.shield.break\"));",
        "        assert_eq!(vanilla_item_default_break_sound_resource_id(VANILLA_ITEM_WOLF_ARMOR_ID), Some(\"minecraft:item.wolf_armor.break\"));",
        "        assert_eq!(vanilla_item_default_break_sound_resource_id(-1), None);",
        f"        assert_eq!(VANILLA_AXE_STRIPPABLES.len(), {EXPECTED_TOOL_USE_ON_COUNTS['axe_strippables']});",
        f"        assert_eq!(VANILLA_AXE_SCRAPEABLES.len(), {EXPECTED_TOOL_USE_ON_COUNTS['axe_scrapeables']});",
        f"        assert_eq!(VANILLA_AXE_WAX_OFF.len(), {EXPECTED_TOOL_USE_ON_COUNTS['axe_wax_off']});",
        f"        assert_eq!(VANILLA_HONEYCOMB_WAXABLES.len(), {EXPECTED_TOOL_USE_ON_COUNTS['honeycomb_waxables']});",
        f"        assert_eq!(VANILLA_HOE_TILLABLES.len(), {EXPECTED_TOOL_USE_ON_COUNTS['hoe_tillables']});",
        f"        assert_eq!(VANILLA_SHOVEL_FLATTENABLES.len(), {EXPECTED_TOOL_USE_ON_COUNTS['shovel_flattenables']});",
        "        assert!(VANILLA_AXE_STRIPPABLES.iter().any(|entry| entry.source_block_type_id == crate::block_types::VANILLA_BLOCK_OAK_LOG_ID && entry.target_block_type_id == crate::block_types::VANILLA_BLOCK_STRIPPED_OAK_LOG_ID));",
        "        assert!(VANILLA_AXE_SCRAPEABLES.iter().any(|entry| entry.source_block_type_id == crate::block_types::VANILLA_BLOCK_WEATHERED_COPPER_ID && entry.target_block_type_id == crate::block_types::VANILLA_BLOCK_EXPOSED_COPPER_ID));",
        "        assert!(VANILLA_AXE_WAX_OFF.iter().any(|entry| entry.source_block_type_id == crate::block_types::VANILLA_BLOCK_WAXED_COPPER_BLOCK_ID && entry.target_block_type_id == crate::block_types::VANILLA_BLOCK_COPPER_BLOCK_ID));",
        "        assert!(VANILLA_HONEYCOMB_WAXABLES.iter().any(|entry| entry.source_block_type_id == crate::block_types::VANILLA_BLOCK_COPPER_BLOCK_ID && entry.target_block_type_id == crate::block_types::VANILLA_BLOCK_WAXED_COPPER_BLOCK_ID));",
        "        assert!(VANILLA_HOE_TILLABLES.iter().any(|entry| entry.transform.source_block_type_id == crate::block_types::VANILLA_BLOCK_ROOTED_DIRT_ID && !entry.requires_air_above));",
        "        assert!(VANILLA_SHOVEL_FLATTENABLES.iter().any(|entry| entry.source_block_type_id == crate::block_types::VANILLA_BLOCK_GRASS_BLOCK_ID && entry.target_block_type_id == crate::block_types::VANILLA_BLOCK_DIRT_PATH_ID));",
        "    }", "", "    #[test]", "    fn tool_use_on_consumer_uses_generated_catalogs() {",
        "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
        "        let path = crates.join(\"bbb-world/src/client/inventory/use_prediction.rs\");",
        "        let source = fs::read_to_string(&path).unwrap();",
        "        let production = source.split(\"#[cfg(test)]\").next().unwrap();",
        "        for catalog in [\"VANILLA_AXE_STRIPPABLES\", \"VANILLA_AXE_SCRAPEABLES\", \"VANILLA_AXE_WAX_OFF\", \"VANILLA_HONEYCOMB_WAXABLES\", \"VANILLA_HOE_TILLABLES\", \"VANILLA_SHOVEL_FLATTENABLES\"] {",
        "            assert!(production.contains(catalog), \"{} missing generated {catalog}\", path.display());",
        "        }",
        "        let mut block_ids = HashSet::new();",
        "        for transform in VANILLA_AXE_STRIPPABLES.iter().chain(VANILLA_AXE_SCRAPEABLES).chain(VANILLA_AXE_WAX_OFF).chain(VANILLA_HONEYCOMB_WAXABLES).chain(VANILLA_SHOVEL_FLATTENABLES) {",
        "            block_ids.insert(transform.source_block_type_id);",
        "            block_ids.insert(transform.target_block_type_id);",
        "        }",
        "        for tillable in VANILLA_HOE_TILLABLES {",
        "            block_ids.insert(tillable.transform.source_block_type_id);",
        "            block_ids.insert(tillable.transform.target_block_type_id);",
        "        }",
        "        for block_id in block_ids {",
        "            let resource_id = crate::block_types::vanilla_block_type(block_id).unwrap().resource_id;",
        "            let literal = format!(\"\\\"{resource_id}\\\"\");",
        "            assert!(!production.contains(&literal), \"{} restored hand-written {literal}\", path.display());",
        "        }",
        "    }", "", "    #[test]", "    fn item_stack_size_consumers_use_generated_constants() {",
        "        assert_eq!(VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE, 64);",
        "        assert_eq!(VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE, 99);",
        "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
        "        let sources = [crates.join(\"bbb-world/src/item_profiles.rs\"), crates.join(\"bbb-item-model/src/item_runtime/icon_model.rs\")];",
        "        for path in sources {",
        "            let source = fs::read_to_string(&path).unwrap();",
        "            assert!(source.contains(\"VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE\"), \"{} missing generated default stack size\", path.display());",
        "            assert!(source.contains(\"VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE\"), \"{} missing generated absolute stack size\", path.display());",
        "            for removed in [\"VANILLA_DEFAULT_MAX_STACK_SIZE\", \"VANILLA_ABSOLUTE_MAX_STACK_SIZE\"] {",
        "                assert!(!source.contains(removed), \"{} restored removed {removed}\", path.display());",
        "            }",
        "        }",
        "        for path in [",
        "            crates.join(\"bbb-world/src/client/inventory.rs\"),",
        "            crates.join(\"bbb-world/src/client/inventory/merchant.rs\"),",
        "            crates.join(\"bbb-world/src/client/inventory/slot_ops.rs\"),",
        "        ] {",
        "            let source = fs::read_to_string(&path).unwrap();",
        "            assert!(source.contains(\"VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE\"), \"{} missing generated default stack size\", path.display());",
        "            for removed in [\"VANILLA_DEFAULT_MAX_STACK_SIZE\", \"VANILLA_ABSOLUTE_MAX_STACK_SIZE\"] {",
        "                assert!(!source.contains(removed), \"{} restored removed {removed}\", path.display());",
        "            }",
        "        }",
        "    }", "", "    #[test]", "    fn item_identity_consumers_do_not_restore_item_resource_literals() {",
        "        fn assert_no_resource_literals(sources: &[&str], item_ids: &[i32]) {",
        "            let combined = sources.concat();",
        "            for &item_id in item_ids {",
        "                let resource_id = vanilla_item_type(item_id).expect(\"generated item id\").resource_id;",
        '                let literal = format!("\\\"{resource_id}\\\"");',
        '                assert!(!combined.contains(&literal), "production consumer restored {literal}");',
        "            }",
        "        }",
        "",
        '        let item_model_sources = [include_str!("../../bbb-item-model/src/item_runtime.rs"),',
        '            include_str!("../../bbb-item-model/src/item_runtime/icon.rs"),',
        '            include_str!("../../bbb-item-model/src/item_runtime/profiles.rs"),',
        '            include_str!("../../bbb-item-model/src/item_runtime/tooltip.rs")];',
        "        let item_model_ids = [",
        "            VANILLA_ITEM_FIREWORK_ROCKET_ID, VANILLA_ITEM_BOW_ID, VANILLA_ITEM_BRUSH_ID,",
        "            VANILLA_ITEM_CROSSBOW_ID, VANILLA_ITEM_SPYGLASS_ID, VANILLA_ITEM_TRIDENT_ID,",
        "            VANILLA_ITEM_ENDER_EYE_ID, VANILLA_ITEM_LEATHER_BOOTS_ID,",
        "            VANILLA_ITEM_DISC_FRAGMENT_5_ID, VANILLA_ITEM_PAINTING_ID,",
        "            VANILLA_ITEM_SPAWNER_ID, VANILLA_ITEM_TRIAL_SPAWNER_ID,",
        "            VANILLA_ITEM_NETHERITE_UPGRADE_SMITHING_TEMPLATE_ID,",
        "            VANILLA_ITEM_SKELETON_SKULL_ID, VANILLA_ITEM_WITHER_SKELETON_SKULL_ID,",
        "            VANILLA_ITEM_PLAYER_HEAD_ID, VANILLA_ITEM_ZOMBIE_HEAD_ID,",
        "            VANILLA_ITEM_CREEPER_HEAD_ID, VANILLA_ITEM_DRAGON_HEAD_ID,",
        "            VANILLA_ITEM_PIGLIN_HEAD_ID, VANILLA_ITEM_CLOCK_ID, VANILLA_ITEM_COMPASS_ID,",
        "            VANILLA_ITEM_RECOVERY_COMPASS_ID,",
        "        ];",
        "        assert_no_resource_literals(&item_model_sources, &item_model_ids);",
        '        assert!(item_model_sources[0].contains("use bbb_protocol::item_types"));',
        "",
        "        let native_sources = [",
        '            include_str!("../../bbb-native/src/item_models/first_person.rs"),',
        '            include_str!("../../bbb-native/src/entity_scene/hands.rs"),',
        '            include_str!("../../bbb-native/src/entity_scene/instance.rs")];',
        "        let native_ids = [VANILLA_ITEM_BOW_ID, VANILLA_ITEM_BRUSH_ID,",
        "            VANILLA_ITEM_BUNDLE_ID, VANILLA_ITEM_CROSSBOW_ID, VANILLA_ITEM_GOAT_HORN_ID,",
        "            VANILLA_ITEM_SHIELD_ID, VANILLA_ITEM_SPYGLASS_ID, VANILLA_ITEM_TRIDENT_ID,",
        "            VANILLA_ITEM_POTION_ID];",
        "        assert_no_resource_literals(&native_sources, &native_ids);",
        "",
        "        let pot_sources = [",
        '            include_str!("../../bbb-world/src/chunks/pot_decorations.rs"),',
        '            include_str!("../../bbb-world/src/client/dialog/body/component_value.rs"),',
        '            include_str!("../../bbb-world/src/client/dialog/body/component_value/holder_set.rs"),',
        '            include_str!("../../bbb-world/src/client/dialog/body/component_value/canonical_json.rs").split("#[cfg(test)]\\nmod tests").next().unwrap()];',
        "        assert_no_resource_literals(&pot_sources, &[VANILLA_ITEM_BRICK_ID]);",
        "",
        '        let protocol_sources = [include_str!("component/codec.rs")];',
        "        assert_no_resource_literals(&protocol_sources, &[VANILLA_ITEM_AIR_ID]);",
        "",
        '        assert!(item_model_sources.iter().chain(native_sources.iter()).chain(pot_sources.iter()).chain(protocol_sources.iter()).any(|source| source.contains("vanilla_item_resource_id_matches_type")));',
        "    }", "}", "",
    ))
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input="\n".join(lines), text=True, capture_output=True, check=True,
    )
    return formatted.stdout


def generate(source_root: Path) -> tuple[str, str]:
    items = parse_source(ITEMS_PATH.as_posix(), (source_root / ITEMS_PATH).read_bytes())
    item = parse_source(ITEM_PATH.as_posix(), (source_root / ITEM_PATH).read_bytes())
    item_ids = parse_source(ITEM_IDS_PATH.as_posix(), (source_root / ITEM_IDS_PATH).read_bytes())
    foods = parse_source(FOODS_PATH.as_posix(), (source_root / FOODS_PATH).read_bytes())
    data_components = parse_source(
        DATA_COMPONENTS_PATH.as_posix(), (source_root / DATA_COMPONENTS_PATH).read_bytes()
    )
    sound_events = parse_source(
        SOUND_EVENTS_PATH.as_posix(), (source_root / SOUND_EVENTS_PATH).read_bytes()
    )
    extracted = extract_item_registry_catalog(items, item_ids, item, foods)
    value = extracted.as_json_value()
    facts = json.dumps(value, indent=2, sort_keys=True) + "\n"
    resource_paths: list[str] = []
    use_override_owners: list[str | None] = []
    use_on_override_owners: list[str | None] = []
    interact_living_entity_override_owners: list[str | None] = []
    equip_on_interact_owners: list[str | None] = []
    bucket_behaviors: list[dict[str, str | None] | None] = []
    block_item_behaviors: list[dict[str, str | None] | None] = []
    item_class_sources = [
        parse_source(
            path.relative_to(source_root).as_posix(),
            path.read_bytes(),
        )
        for path in sorted((source_root / ITEM_PACKAGE_PATH).glob("*.java"))
    ]
    item_class_source_by_file = {
        Path(source.relative_path).name: source for source in item_class_sources
    }
    use_override_owner_by_factory = extract_item_use_override_owners(
        value["declarations"], item_class_sources
    )
    use_on_override_owner_by_factory = extract_item_use_on_override_owners(
        value["declarations"], item_class_sources
    )
    interact_living_entity_override_owner_by_factory = (
        extract_item_interact_living_entity_override_owners(
            value["declarations"], item_class_sources
        )
    )
    bucket_behavior_by_field = extract_bucket_item_behaviors(value["declarations"])
    block_item_behavior_by_field = extract_block_item_behaviors(
        value["declarations"], use_on_override_owner_by_factory
    )
    block_catalog = extract_block_registry_catalog(
        parse_source(
            BLOCKS_PATH.as_posix(), (source_root / BLOCKS_PATH).read_bytes()
        ),
        parse_source(
            BLOCK_IDS_PATH.as_posix(), (source_root / BLOCK_IDS_PATH).read_bytes()
        ),
        parse_source(
            BLOCK_ENTITY_TYPES_PATH.as_posix(),
            (source_root / BLOCK_ENTITY_TYPES_PATH).read_bytes(),
        ),
    )
    block_fields = {
        declaration["field"] for declaration in block_catalog.declarations
    }
    block_field_by_resource_id = {
        declaration["resource_id"]: declaration["field"]
        for declaration in block_catalog.declarations
    }
    sound_resources = {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(sound_events)
    }
    for declaration in value["declarations"]:
        declaration_resources = _resource_ids(
            declaration, value["item_id_constants"]
        )
        factory_class = item_factory_class(declaration)
        resource_count = len(declaration_resources)
        resource_paths.extend(declaration_resources)
        use_override_owners.extend(
            [use_override_owner_by_factory[factory_class]] * resource_count
        )
        use_on_override_owners.extend(
            [use_on_override_owner_by_factory[factory_class]] * resource_count
        )
        interact_living_entity_override_owners.extend(
            [interact_living_entity_override_owner_by_factory[factory_class]]
            * resource_count
        )
        equip_on_interact_owners.extend(
            [_equip_on_interact_owner(declaration)] * resource_count
        )
        bucket_behavior = bucket_behavior_by_field[declaration["field"]]
        if bucket_behavior is not None:
            bucket_behavior = dict(bucket_behavior)
            empty_sound = bucket_behavior["mob_empty_sound"]
            if empty_sound is not None:
                if empty_sound not in sound_resources:
                    raise ValueError(
                        f"Items.{declaration['field']}: unknown sound {empty_sound}"
                    )
                bucket_behavior["mob_empty_sound"] = sound_resources[empty_sound]
        bucket_behaviors.extend([bucket_behavior] * resource_count)
        block_item_behavior = block_item_behavior_by_field[declaration["field"]]
        if block_item_behavior is None:
            block_item_behaviors.extend([None] * resource_count)
            continue
        primary_block = block_item_behavior["primary_block"]
        if declaration["field_type"] == "WeatheringCopperItems":
            expanded_block_fields = []
            for resource_path in declaration_resources:
                resource_id = "minecraft:" + resource_path
                block_field = block_field_by_resource_id.get(resource_id)
                if block_field is None:
                    raise ValueError(
                        f"Items.{declaration['field']}: unresolved weathering "
                        f"block resource {resource_id}"
                    )
                expanded_block_fields.append(block_field)
            if expanded_block_fields[0] != primary_block:
                raise ValueError(
                    f"Items.{declaration['field']}: weathering primary block "
                    f"{primary_block} does not match {expanded_block_fields[0]}"
                )
        else:
            if primary_block not in block_fields:
                raise ValueError(
                    f"Items.{declaration['field']}: unresolved primary block "
                    f"Blocks.{primary_block}"
                )
            expanded_block_fields = [primary_block] * resource_count
        wall_block = block_item_behavior["wall_block"]
        if wall_block is not None and wall_block not in block_fields:
            raise ValueError(
                f"Items.{declaration['field']}: unresolved wall block "
                f"Blocks.{wall_block}"
            )
        block_item_behaviors.extend(
            [
                {
                    "kind": block_item_behavior["kind"],
                    "block_field": block_field,
                    "wall_block_field": wall_block,
                }
                for block_field in expanded_block_fields
            ]
        )
    actual_block_item_behavior_count = sum(
        behavior is not None for behavior in block_item_behaviors
    )
    if actual_block_item_behavior_count != EXPECTED_BLOCK_ITEM_BEHAVIOR_COUNT:
        raise ValueError(
            "unexpected vanilla 26.1 BlockItem behavior count: "
            f"{actual_block_item_behavior_count}"
        )
    common_break_sound, break_sound_overrides = _default_break_sounds(
        value["declarations"],
        value["item_id_constants"],
        item,
        data_components,
        sound_events,
    )
    tool_use_on_catalog = extract_tool_use_on_catalog(
        block_catalog.declarations,
        item_class_source_by_file["AxeItem.java"],
        item_class_source_by_file["HoeItem.java"],
        item_class_source_by_file["ShovelItem.java"],
        item_class_source_by_file["HoneycombItem.java"],
        parse_source(
            WEATHERING_COPPER_PATH.as_posix(),
            (source_root / WEATHERING_COPPER_PATH).read_bytes(),
        ),
        parse_source(
            WEATHERING_COPPER_BLOCKS_PATH.as_posix(),
            (source_root / WEATHERING_COPPER_BLOCKS_PATH).read_bytes(),
        ),
    )
    actual_tool_use_on_counts = {
        name: len(entries) for name, entries in tool_use_on_catalog.items()
    }
    if actual_tool_use_on_counts != EXPECTED_TOOL_USE_ON_COUNTS:
        raise ValueError(
            "unexpected vanilla 26.1 tool useOn catalog counts: "
            f"{actual_tool_use_on_counts}"
        )
    return facts, _render_types(
        resource_paths,
        use_override_owners,
        use_on_override_owners,
        interact_living_entity_override_owners,
        equip_on_interact_owners,
        bucket_behaviors,
        block_item_behaviors,
        extracted.item_constants,
        common_break_sound,
        break_sound_overrides,
        tool_use_on_catalog,
    )


def _check(path: Path, generated: str) -> bool:
    current = path.read_text() if path.exists() else ""
    if current == generated:
        return True
    sys.stderr.writelines(difflib.unified_diff(
        current.splitlines(keepends=True), generated.splitlines(keepends=True),
        fromfile=str(path), tofile="generated",
    ))
    return False


def main() -> int:
    args = parse_args()
    facts, types = generate(args.source_root)
    if args.check:
        facts_match = _check(args.facts_output, facts)
        types_match = _check(args.types_output, types)
        return 0 if facts_match and types_match else 1
    args.facts_output.parent.mkdir(parents=True, exist_ok=True)
    args.types_output.parent.mkdir(parents=True, exist_ok=True)
    args.facts_output.write_text(facts)
    args.types_output.write_text(types)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
