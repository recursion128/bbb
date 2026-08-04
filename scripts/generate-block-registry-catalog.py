#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 block registry declaration facts."""

from __future__ import annotations

import argparse
import difflib
import json
import subprocess
import sys
from pathlib import Path

from block_registry_catalog_extractor import (
    extract_block_analog_output_owners,
    block_factory_class,
    extract_block_bonemeal_target_override_owners,
    extract_block_bucket_pickup_owners,
    extract_block_can_survive_override_owners,
    extract_concrete_powder_targets,
    extract_block_get_state_for_placement_override_owners,
    extract_block_interface_membership,
    extract_block_liquid_container_owners,
    extract_block_registry_catalog,
    extract_block_propagates_skylight_down_override_owners,
    extract_block_redstone_signal_owners,
    extract_block_replaceable_membership,
    extract_block_subclass_membership,
    extract_block_use_item_on_override_owners,
    extract_block_use_without_item_override_owners,
)
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-pack/data/block_registry_declarations_26_1.json"
DEFAULT_PROTOCOL_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/block_types.rs"
BLOCKS_PATH = Path("net/minecraft/world/level/block/Blocks.java")
BLOCK_IDS_PATH = Path("net/minecraft/references/BlockIds.java")
BLOCK_ENTITY_TYPES_PATH = Path(
    "net/minecraft/world/level/block/entity/BlockEntityType.java"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--protocol-output", type=Path, default=DEFAULT_PROTOCOL_OUTPUT
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def extract(source_root: Path):
    blocks = parse_source(
        BLOCKS_PATH.as_posix(), (source_root / BLOCKS_PATH).read_bytes()
    )
    block_ids = parse_source(
        BLOCK_IDS_PATH.as_posix(), (source_root / BLOCK_IDS_PATH).read_bytes()
    )
    block_entity_types = parse_source(
        BLOCK_ENTITY_TYPES_PATH.as_posix(),
        (source_root / BLOCK_ENTITY_TYPES_PATH).read_bytes(),
    )
    return extract_block_registry_catalog(blocks, block_ids, block_entity_types)


def render_json(catalog) -> str:
    return json.dumps(catalog.as_json_value(), indent=2, sort_keys=True) + "\n"


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def format_rust(source: str) -> str:
    result = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input=source,
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout


def render_protocol(
    catalog,
    analog_output_owner_by_factory: dict[str, str | None],
    is_signal_source_owner_by_factory: dict[str, str | None],
    signal_owner_by_factory: dict[str, str | None],
    direct_signal_owner_by_factory: dict[str, str | None],
    use_item_on_owner_by_factory: dict[str, str | None],
    use_without_item_owner_by_factory: dict[str, str | None],
    can_survive_owner_by_factory: dict[str, str | None],
    get_state_for_placement_owner_by_factory: dict[str, str | None],
    bucket_pickup_owner_by_factory: dict[str, str | None],
    liquid_container_owner_by_factory: dict[str, str | None],
    growing_plant_head_by_factory: dict[str, bool],
    game_master_block_by_factory: dict[str, bool],
    bonemeal_target_owner_by_factory: dict[str, str | None],
    skylight_down_owner_by_factory: dict[str, str | None],
    replaceable_by_field: dict[str, bool],
    concrete_powder_targets: dict[str, str],
) -> str:
    analog_output_owners = [
        analog_output_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    is_signal_source_owners = [
        is_signal_source_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    signal_owners = [
        signal_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    direct_signal_owners = [
        direct_signal_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    use_item_on_owners = [
        use_item_on_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    use_without_item_owners = [
        use_without_item_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    can_survive_owners = [
        can_survive_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    get_state_for_placement_owners = [
        get_state_for_placement_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    bucket_pickup_owners = [
        bucket_pickup_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    liquid_container_owners = [
        liquid_container_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    growing_plant_heads = [
        growing_plant_head_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    game_master_blocks = [
        game_master_block_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    bonemeal_target_owners = [
        bonemeal_target_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    skylight_down_owners = [
        skylight_down_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    if sum(growing_plant_heads) != 4:
        raise ValueError(
            "expected 4 vanilla 26.1 GrowingPlantHeadBlock registrations, got "
            f"{sum(growing_plant_heads)}"
        )
    if sum(game_master_blocks) != 7:
        raise ValueError(
            "expected 7 vanilla 26.1 GameMasterBlock registrations, got "
            f"{sum(game_master_blocks)}"
        )
    if len(concrete_powder_targets) != 16:
        raise ValueError(
            "expected 16 vanilla 26.1 ConcretePowderBlock registrations, got "
            f"{len(concrete_powder_targets)}"
        )
    use_item_on_variants = sorted(
        {owner for owner in use_item_on_owners if owner is not None}
    )
    analog_output_variants = sorted(
        {owner for owner in analog_output_owners if owner is not None}
    )
    redstone_signal_variants = sorted(
        {
            owner
            for owners in (
                is_signal_source_owners,
                signal_owners,
                direct_signal_owners,
            )
            for owner in owners
            if owner is not None
        }
    )
    use_without_item_variants = sorted(
        {owner for owner in use_without_item_owners if owner is not None}
    )
    can_survive_variants = sorted(
        {owner for owner in can_survive_owners if owner is not None}
    )
    get_state_for_placement_variants = sorted(
        {owner for owner in get_state_for_placement_owners if owner is not None}
    )
    bucket_pickup_variants = sorted(
        {owner for owner in bucket_pickup_owners if owner is not None}
    )
    liquid_container_variants = sorted(
        {owner for owner in liquid_container_owners if owner is not None}
    )
    bonemeal_target_variants = sorted(
        {owner for owner in bonemeal_target_owners if owner is not None}
    )
    skylight_down_variants = sorted(
        {owner for owner in skylight_down_owners if owner is not None}
    )
    lines = [
        "// @generated by scripts/generate-block-registry-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Blocks.java registration order, replaceable properties, and ConcretePowderBlock targets; BlockBehaviour interaction/analog-output/canSurvive/getStateForPlacement overrides; BucketPickup/LiquidBlockContainer/GameMasterBlock interface owners; BonemealableBlock target overrides; and block class hierarchy.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-block-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-block-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaBlockAnalogOutputOverride {",
    ]
    lines.extend(f"    {owner}," for owner in analog_output_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaBlockRedstoneSignalOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in redstone_signal_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaBlockUseItemOnOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in use_item_on_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaBlockUseWithoutItemOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in use_without_item_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaCanSurviveOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in can_survive_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaGetStateForPlacementOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in get_state_for_placement_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaBucketPickupOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in bucket_pickup_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaLiquidBlockContainerOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in liquid_container_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaBonemealTargetOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in bonemeal_target_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaPropagatesSkylightDownOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in skylight_down_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct BlockTypeCatalogEntry {",
            "    pub id: i32,",
            "    pub constant_name: &'static str,",
            "    pub resource_id: &'static str,",
            "    pub block_entity_type_id: Option<i32>,",
            "    pub analog_output_override: Option<VanillaBlockAnalogOutputOverride>,",
            "    pub is_signal_source_override: Option<VanillaBlockRedstoneSignalOverride>,",
            "    pub signal_override: Option<VanillaBlockRedstoneSignalOverride>,",
            "    pub direct_signal_override: Option<VanillaBlockRedstoneSignalOverride>,",
            "    pub use_item_on_override: Option<VanillaBlockUseItemOnOverride>,",
            "    pub use_without_item_override: Option<VanillaBlockUseWithoutItemOverride>,",
            "    pub can_survive_override: Option<VanillaCanSurviveOverride>,",
            "    pub get_state_for_placement_override: Option<VanillaGetStateForPlacementOverride>,",
            "    pub bucket_pickup_override: Option<VanillaBucketPickupOverride>,",
            "    pub liquid_block_container_override: Option<VanillaLiquidBlockContainerOverride>,",
            "    pub growing_plant_head: bool,",
            "    pub bonemeal_target_override: Option<VanillaBonemealTargetOverride>,",
            "    pub propagates_skylight_down_override: Option<VanillaPropagatesSkylightDownOverride>,",
            "    pub replaceable: bool,",
            "}",
            "",
        ]
    )
    for declaration in catalog.declarations:
        lines.append(
            f"pub const VANILLA_BLOCK_{declaration['field']}_ID: i32 = "
            f"{declaration['declaration_order']};"
        )
    lines.extend(("", "pub const VANILLA_GAME_MASTER_BLOCK_IDS: &[i32] = &["))
    for declaration, is_game_master_block in zip(
        catalog.declarations, game_master_blocks, strict=True
    ):
        if is_game_master_block:
            lines.append(f"    VANILLA_BLOCK_{declaration['field']}_ID,")
    lines.extend(("];", ""))
    lines.extend(("", "pub const VANILLA_BLOCK_TYPES: &[BlockTypeCatalogEntry] = &["))
    for (
        declaration,
        analog_output_owner,
        is_signal_source_owner,
        signal_owner,
        direct_signal_owner,
        use_item_on_owner,
        use_without_item_owner,
        can_survive_owner,
        get_state_for_placement_owner,
        bucket_pickup_owner,
        liquid_container_owner,
        growing_plant_head,
        bonemeal_target_owner,
        skylight_down_owner,
    ) in zip(
        catalog.declarations,
        analog_output_owners,
        is_signal_source_owners,
        signal_owners,
        direct_signal_owners,
        use_item_on_owners,
        use_without_item_owners,
        can_survive_owners,
        get_state_for_placement_owners,
        bucket_pickup_owners,
        liquid_container_owners,
        growing_plant_heads,
        bonemeal_target_owners,
        skylight_down_owners,
        strict=True,
    ):
        block_entity_type = declaration["block_entity_type"]
        block_entity_type_id = (
            "Some(crate::block_entity_types::"
            f"VANILLA_BLOCK_ENTITY_TYPE_{block_entity_type}_ID)"
            if block_entity_type is not None
            else "None"
        )
        lines.extend(
            (
                "    BlockTypeCatalogEntry {",
                f"        id: {declaration['declaration_order']},",
                f"        constant_name: {rust_string(declaration['field'])},",
                f"        resource_id: {rust_string(declaration['resource_id'])},",
                f"        block_entity_type_id: {block_entity_type_id},",
                (
                    "        analog_output_override: "
                    f"Some(VanillaBlockAnalogOutputOverride::{analog_output_owner}),"
                    if analog_output_owner is not None
                    else "        analog_output_override: None,"
                ),
                (
                    "        is_signal_source_override: "
                    f"Some(VanillaBlockRedstoneSignalOverride::{is_signal_source_owner}),"
                    if is_signal_source_owner is not None
                    else "        is_signal_source_override: None,"
                ),
                (
                    "        signal_override: "
                    f"Some(VanillaBlockRedstoneSignalOverride::{signal_owner}),"
                    if signal_owner is not None
                    else "        signal_override: None,"
                ),
                (
                    "        direct_signal_override: "
                    f"Some(VanillaBlockRedstoneSignalOverride::{direct_signal_owner}),"
                    if direct_signal_owner is not None
                    else "        direct_signal_override: None,"
                ),
                (
                    "        use_item_on_override: "
                    f"Some(VanillaBlockUseItemOnOverride::{use_item_on_owner}),"
                    if use_item_on_owner is not None
                    else "        use_item_on_override: None,"
                ),
                (
                    "        use_without_item_override: "
                    f"Some(VanillaBlockUseWithoutItemOverride::{use_without_item_owner}),"
                    if use_without_item_owner is not None
                    else "        use_without_item_override: None,"
                ),
                (
                    "        can_survive_override: "
                    f"Some(VanillaCanSurviveOverride::{can_survive_owner}),"
                    if can_survive_owner is not None
                    else "        can_survive_override: None,"
                ),
                (
                    "        get_state_for_placement_override: "
                    f"Some(VanillaGetStateForPlacementOverride::{get_state_for_placement_owner}),"
                    if get_state_for_placement_owner is not None
                    else "        get_state_for_placement_override: None,"
                ),
                (
                    "        bucket_pickup_override: "
                    f"Some(VanillaBucketPickupOverride::{bucket_pickup_owner}),"
                    if bucket_pickup_owner is not None
                    else "        bucket_pickup_override: None,"
                ),
                (
                    "        liquid_block_container_override: "
                    f"Some(VanillaLiquidBlockContainerOverride::{liquid_container_owner}),"
                    if liquid_container_owner is not None
                    else "        liquid_block_container_override: None,"
                ),
                f"        growing_plant_head: {str(growing_plant_head).lower()},",
                (
                    "        bonemeal_target_override: "
                    f"Some(VanillaBonemealTargetOverride::{bonemeal_target_owner}),"
                    if bonemeal_target_owner is not None
                    else "        bonemeal_target_override: None,"
                ),
                (
                    "        propagates_skylight_down_override: "
                    f"Some(VanillaPropagatesSkylightDownOverride::{skylight_down_owner}),"
                    if skylight_down_owner is not None
                    else "        propagates_skylight_down_override: None,"
                ),
                f"        replaceable: {str(replaceable_by_field[declaration['field']]).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_block_type(id: i32) -> Option<&'static BlockTypeCatalogEntry> {",
            "    VANILLA_BLOCK_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_block_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static BlockTypeCatalogEntry> {",
            "    VANILLA_BLOCK_TYPES",
            "        .iter()",
            "        .find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
            "pub const fn vanilla_concrete_block_type_id_for_powder(",
            "    block_type_id: i32,",
            ") -> Option<i32> {",
            "    match block_type_id {",
        )
    )
    for declaration in catalog.declarations:
        target = concrete_powder_targets.get(declaration["field"])
        if target is not None:
            lines.append(
                f"        VANILLA_BLOCK_{declaration['field']}_ID => "
                f"Some(VANILLA_BLOCK_{target}_ID),"
            )
    lines.extend(
        (
            "        _ => None,",
            "    }",
            "}",
            "",
            "pub fn vanilla_block_entity_type_id_for_block_type_id(",
            "    block_type_id: i32,",
            ") -> Option<i32> {",
            "    vanilla_block_type(block_type_id)?.block_entity_type_id",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::{collections::HashSet, fs, path::Path};",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn block_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_BLOCK_TYPES.len(), 1168);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_BLOCK_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn block_type_catalog_follows_registration_order() {",
            "        assert_eq!(VANILLA_BLOCK_AIR_ID, 0);",
            "        assert_eq!(vanilla_block_type(0).unwrap().resource_id, \"minecraft:air\");",
            "        assert_eq!(",
            "            VANILLA_BLOCK_EXPOSED_COPPER_BARS_ID,",
            "            VANILLA_BLOCK_COPPER_BARS_ID + 1",
            "        );",
            "        assert_eq!(",
            "            vanilla_block_type_for_resource_id(\"minecraft:note_block\")",
            "                .unwrap()",
            "                .id,",
            "            VANILLA_BLOCK_NOTE_BLOCK_ID",
            "        );",
            "        assert_eq!(vanilla_block_type(-1), None);",
            "        assert_eq!(vanilla_block_type(1168), None);",
            "        assert_eq!(vanilla_block_type_for_resource_id(\"example:air\"), None);",
            "        assert_eq!(VANILLA_GAME_MASTER_BLOCK_IDS.len(), 7);",
            "        assert_eq!(vanilla_concrete_block_type_id_for_powder(VANILLA_BLOCK_WHITE_CONCRETE_POWDER_ID), Some(VANILLA_BLOCK_WHITE_CONCRETE_ID));",
            "        assert_eq!(vanilla_concrete_block_type_id_for_powder(VANILLA_BLOCK_BLACK_CONCRETE_POWDER_ID), Some(VANILLA_BLOCK_BLACK_CONCRETE_ID));",
            "        assert_eq!(vanilla_concrete_block_type_id_for_powder(VANILLA_BLOCK_STONE_ID), None);",
            "        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| vanilla_concrete_block_type_id_for_powder(entry.id).is_some()).count(), 16);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().block_entity_type_id, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CHEST_ID).unwrap().block_entity_type_id, Some(crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.analog_output_override.is_some()).count(), {sum(owner is not None for owner in analog_output_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().analog_output_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CHEST_ID).unwrap().analog_output_override, Some(VanillaBlockAnalogOutputOverride::ChestBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CAULDRON_ID).unwrap().analog_output_override, Some(VanillaBlockAnalogOutputOverride::BlockBehaviour));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.is_signal_source_override.is_some()).count(), {sum(owner is not None for owner in is_signal_source_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.signal_override.is_some()).count(), {sum(owner is not None for owner in signal_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.direct_signal_override.is_some()).count(), {sum(owner is not None for owner in direct_signal_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().is_signal_source_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REDSTONE_BLOCK_ID).unwrap().signal_override, Some(VanillaBlockRedstoneSignalOverride::PoweredBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REDSTONE_WIRE_ID).unwrap().direct_signal_override, Some(VanillaBlockRedstoneSignalOverride::RedStoneWireBlock));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.use_item_on_override.is_some()).count(), {sum(owner is not None for owner in use_item_on_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.use_without_item_override.is_some()).count(), {sum(owner is not None for owner in use_without_item_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.can_survive_override.is_some()).count(), {sum(owner is not None for owner in can_survive_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.get_state_for_placement_override.is_some()).count(), {sum(owner is not None for owner in get_state_for_placement_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.bucket_pickup_override.is_some()).count(), {sum(owner is not None for owner in bucket_pickup_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.liquid_block_container_override.is_some()).count(), {sum(owner is not None for owner in liquid_container_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().use_item_on_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().use_without_item_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().can_survive_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().get_state_for_placement_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OAK_SLAB_ID).unwrap().can_survive_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OAK_SLAB_ID).unwrap().get_state_for_placement_override, Some(VanillaGetStateForPlacementOverride::SlabBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_LADDER_ID).unwrap().can_survive_override, Some(VanillaCanSurviveOverride::LadderBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().bucket_pickup_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().liquid_block_container_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_WATER_ID).unwrap().bucket_pickup_override, Some(VanillaBucketPickupOverride::LiquidBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_POWDER_SNOW_ID).unwrap().bucket_pickup_override, Some(VanillaBucketPickupOverride::PowderSnowBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OAK_SLAB_ID).unwrap().bucket_pickup_override, Some(VanillaBucketPickupOverride::SimpleWaterloggedBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_KELP_ID).unwrap().liquid_block_container_override, Some(VanillaLiquidBlockContainerOverride::KelpBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OAK_SLAB_ID).unwrap().liquid_block_container_override, Some(VanillaLiquidBlockContainerOverride::SlabBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CAKE_ID).unwrap().use_item_on_override, Some(VanillaBlockUseItemOnOverride::CakeBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CHEST_ID).unwrap().use_without_item_override, Some(VanillaBlockUseWithoutItemOverride::ChestBlock));",
            "        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.growing_plant_head).count(), 4);",
            "        for block_id in [VANILLA_BLOCK_KELP_ID, VANILLA_BLOCK_WEEPING_VINES_ID, VANILLA_BLOCK_TWISTING_VINES_ID, VANILLA_BLOCK_CAVE_VINES_ID] {",
            "            assert!(vanilla_block_type(block_id).unwrap().growing_plant_head);",
            "        }",
            "        assert!(!vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().growing_plant_head);",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.bonemeal_target_override.is_some()).count(), {sum(owner is not None for owner in bonemeal_target_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().bonemeal_target_override, None);",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.propagates_skylight_down_override.is_some()).count(), {sum(owner is not None for owner in skylight_down_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().propagates_skylight_down_override, None);",
            "        assert!(vanilla_block_type(VANILLA_BLOCK_AIR_ID).unwrap().replaceable);",
            "        assert!(!vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().replaceable);",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.replaceable).count(), {sum(replaceable_by_field.values())});",
            "    }",
            "",
            "    #[test]",
            "    fn block_destroy_consumer_uses_generated_game_master_membership() {",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let path = crates.join(\"bbb-world/src/client/local_player_destroy.rs\");",
            "        let source = fs::read_to_string(&path).unwrap();",
            "        let production = source.split(\"#[cfg(test)]\").next().unwrap();",
            "        assert!(production.contains(\"VANILLA_GAME_MASTER_BLOCK_IDS\"));",
            "        for block_id in VANILLA_GAME_MASTER_BLOCK_IDS {",
            "            let resource_id = vanilla_block_type(*block_id).unwrap().resource_id;",
            "            let literal = format!(\"\\\"{resource_id}\\\"\");",
            "            assert!(!production.contains(&literal), \"{} restored hand-written {literal}\", path.display());",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn growing_plant_consumer_uses_generated_membership() {",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let path = crates.join(\"bbb-world/src/client/inventory/use_prediction.rs\");",
            "        let source = fs::read_to_string(&path).unwrap();",
            "        let production = source.split(\"#[cfg(test)]\").next().unwrap();",
            "        assert!(production.contains(\"block_type.growing_plant_head\"));",
            "        for block_id in [VANILLA_BLOCK_KELP_ID, VANILLA_BLOCK_WEEPING_VINES_ID, VANILLA_BLOCK_TWISTING_VINES_ID, VANILLA_BLOCK_CAVE_VINES_ID] {",
            "            let resource_id = vanilla_block_type(block_id).unwrap().resource_id;",
            "            let literal = format!(\"\\\"{resource_id}\\\"\");",
            "            assert!(!production.contains(&literal), \"{} restored hand-written {literal}\", path.display());",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn bonemeal_target_consumer_uses_generated_ownership() {",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let path = crates.join(\"bbb-world/src/client/inventory/use_prediction/bone_meal.rs\");",
            "        let source = fs::read_to_string(&path).unwrap();",
            "        let production = source.split(\"#[cfg(test)]\").next().unwrap();",
            "        assert!(production.contains(\"block_type.bonemeal_target_override\"));",
            "        for resource_id in [\"minecraft:wheat\", \"minecraft:oak_sapling\", \"minecraft:seagrass\"] {",
            "            let literal = format!(\"\\\"{resource_id}\\\"\");",
            "            assert!(!production.contains(&literal), \"{} restored hand-written {literal}\", path.display());",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn bucket_survival_consumer_uses_generated_ownership() {",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let path = crates.join(\"bbb-world/src/client/inventory/use_prediction/bucket.rs\");",
            "        let source = fs::read_to_string(&path).unwrap();",
            "        let production = source.split(\"#[cfg(test)]\").next().unwrap();",
            "        assert!(production.contains(\"block_type.can_survive_override\"));",
            "        for resource_id in [\"minecraft:ladder\", \"minecraft:tube_coral\", \"minecraft:oak_sign\"] {",
            "            let literal = format!(\"\\\"{resource_id}\\\"\");",
            "            assert!(!production.contains(&literal), \"{} restored hand-written {literal}\", path.display());",
            "        }",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def check_output(path: Path, generated: str) -> bool:
    current = path.read_text() if path.exists() else ""
    if current == generated:
        return True
    sys.stderr.writelines(
        difflib.unified_diff(
            current.splitlines(keepends=True),
            generated.splitlines(keepends=True),
            fromfile=str(path),
            tofile="generated",
        )
    )
    return False


def main() -> int:
    args = parse_args()
    catalog = extract(args.source_root)
    block_class_sources = [
        parse_source(
            path.relative_to(args.source_root).as_posix(),
            path.read_bytes(),
        )
        for path in sorted(
            (args.source_root / "net/minecraft/world/level/block").rglob("*.java")
        )
    ]
    use_item_on_owner_by_factory = extract_block_use_item_on_override_owners(
        catalog.declarations, block_class_sources
    )
    analog_output_owner_by_factory = extract_block_analog_output_owners(
        catalog.declarations, block_class_sources
    )
    (
        is_signal_source_owner_by_factory,
        signal_owner_by_factory,
        direct_signal_owner_by_factory,
    ) = extract_block_redstone_signal_owners(
        catalog.declarations, block_class_sources
    )
    use_without_item_owner_by_factory = (
        extract_block_use_without_item_override_owners(
            catalog.declarations, block_class_sources
        )
    )
    can_survive_owner_by_factory = extract_block_can_survive_override_owners(
        catalog.declarations, block_class_sources
    )
    get_state_for_placement_owner_by_factory = (
        extract_block_get_state_for_placement_override_owners(
            catalog.declarations, block_class_sources
        )
    )
    bucket_pickup_owner_by_factory = extract_block_bucket_pickup_owners(
        catalog.declarations, block_class_sources
    )
    liquid_container_owner_by_factory = extract_block_liquid_container_owners(
        catalog.declarations, block_class_sources
    )
    growing_plant_head_by_factory = extract_block_subclass_membership(
        catalog.declarations,
        block_class_sources,
        "net.minecraft.world.level.block.GrowingPlantHeadBlock",
    )
    game_master_block_by_factory = extract_block_interface_membership(
        catalog.declarations,
        block_class_sources,
        "net.minecraft.world.level.block.GameMasterBlock",
    )
    bonemeal_target_owner_by_factory = extract_block_bonemeal_target_override_owners(
        catalog.declarations,
        block_class_sources,
    )
    skylight_down_owner_by_factory = (
        extract_block_propagates_skylight_down_override_owners(
            catalog.declarations,
            block_class_sources,
        )
    )
    replaceable_by_field = extract_block_replaceable_membership(
        catalog.declarations
    )
    concrete_powder_targets = extract_concrete_powder_targets(
        catalog.declarations
    )
    outputs = (
        (args.output, render_json(catalog)),
        (
            args.protocol_output,
            format_rust(
                render_protocol(
                    catalog,
                    analog_output_owner_by_factory,
                    is_signal_source_owner_by_factory,
                    signal_owner_by_factory,
                    direct_signal_owner_by_factory,
                    use_item_on_owner_by_factory,
                    use_without_item_owner_by_factory,
                    can_survive_owner_by_factory,
                    get_state_for_placement_owner_by_factory,
                    bucket_pickup_owner_by_factory,
                    liquid_container_owner_by_factory,
                    growing_plant_head_by_factory,
                    game_master_block_by_factory,
                    bonemeal_target_owner_by_factory,
                    skylight_down_owner_by_factory,
                    replaceable_by_field,
                    concrete_powder_targets,
                )
            ),
        ),
    )
    if args.check:
        checks = [check_output(path, generated) for path, generated in outputs]
        return 0 if all(checks) else 1
    for path, generated in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
