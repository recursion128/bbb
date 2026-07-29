#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 block membership catalogs."""

from __future__ import annotations

import argparse
import difflib
import subprocess
import sys
from pathlib import Path

from block_membership_catalog_extractor import (
    air_blocks,
    block_friction_overrides,
    block_speed_factor_overrides,
    block_tag_membership,
    constructor_family_blocks,
    constructor_types_extending,
    fire_flammable_blocks,
    invisible_render_shape_constructor_types,
    no_collision_blocks,
    no_terrain_particle_blocks,
)
from block_registry_catalog_extractor import (
    extract_block_id_constants,
    extract_block_registry_catalog,
)
from vanilla_java_source import parse_source
from vanilla_tag_source import load_tag_resources


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-world/src/terrain/generated_26_1.rs"
BLOCKS_SOURCE = Path("net/minecraft/world/level/block/Blocks.java")
FIRE_BLOCK_SOURCE = Path("net/minecraft/world/level/block/FireBlock.java")
BLOCK_IDS_SOURCE = Path("net/minecraft/references/BlockIds.java")
BLOCK_ENTITY_TYPES_SOURCE = Path(
    "net/minecraft/world/level/block/entity/BlockEntityType.java"
)
BLOCK_TAG_SOURCE_DIR = Path("data/minecraft/tags/block")
BLOCK_CLASS_SOURCE_DIR = Path("net/minecraft/world/level/block")

BLOCK_TAG_CATALOGS = (
    ("VANILLA_IMPERMEABLE_BLOCKS", "minecraft:impermeable", 19),
    (
        "VANILLA_TRIGGERS_AMBIENT_DRIED_GHAST_BLOCK_SOUNDS",
        "minecraft:triggers_ambient_dried_ghast_block_sounds",
        2,
    ),
    ("VANILLA_BARS_BLOCKS", "minecraft:bars", 9),
    ("VANILLA_SLAB_BLOCKS", "minecraft:slabs", 62),
    ("VANILLA_STAIR_BLOCKS", "minecraft:stairs", 58),
    ("VANILLA_WALL_BLOCKS", "minecraft:walls", 26),
    ("VANILLA_FENCE_BLOCKS", "minecraft:fences", 13),
    ("VANILLA_FENCE_GATE_BLOCKS", "minecraft:fence_gates", 12),
    ("VANILLA_WOOL_CARPET_BLOCKS", "minecraft:wool_carpets", 16),
    ("VANILLA_FLOWER_POT_BLOCKS", "minecraft:flower_pots", 39),
    ("VANILLA_BED_BLOCKS", "minecraft:beds", 16),
    ("VANILLA_CANDLE_BLOCKS", "minecraft:candles", 17),
    ("VANILLA_CANDLE_CAKE_BLOCKS", "minecraft:candle_cakes", 17),
    ("VANILLA_CHAIN_BLOCKS", "minecraft:chains", 9),
    (
        "VANILLA_COPPER_GOLEM_STATUE_BLOCKS",
        "minecraft:copper_golem_statues",
        8,
    ),
    ("VANILLA_LANTERN_BLOCKS", "minecraft:lanterns", 10),
    ("VANILLA_LIGHTNING_ROD_BLOCKS", "minecraft:lightning_rods", 8),
    (
        "VANILLA_HANGING_SIGN_BLOCKS",
        "minecraft:all_hanging_signs",
        24,
    ),
    ("VANILLA_SHULKER_BOX_BLOCKS", "minecraft:shulker_boxes", 17),
    ("VANILLA_ANVIL_BLOCKS", "minecraft:anvil", 3),
    ("VANILLA_CAMPFIRE_BLOCKS", "minecraft:campfires", 2),
    (
        "VANILLA_SOUL_FIRE_BASE_BLOCKS",
        "minecraft:soul_fire_base_blocks",
        2,
    ),
    ("VANILLA_CAULDRON_BLOCKS", "minecraft:cauldrons", 4),
    ("VANILLA_LOG_BLOCKS", "minecraft:logs", 44),
    ("VANILLA_PALE_OAK_LOG_BLOCKS", "minecraft:pale_oak_logs", 4),
    ("VANILLA_PLANK_BLOCKS", "minecraft:planks", 12),
    ("VANILLA_DOOR_BLOCKS", "minecraft:doors", 21),
    ("VANILLA_TRAPDOOR_BLOCKS", "minecraft:trapdoors", 21),
    ("VANILLA_RAIL_BLOCKS", "minecraft:rails", 4),
    ("VANILLA_STANDING_SIGN_BLOCKS", "minecraft:standing_signs", 12),
    ("VANILLA_WALL_SIGN_BLOCKS", "minecraft:wall_signs", 12),
    ("VANILLA_BUTTON_BLOCKS", "minecraft:buttons", 14),
    (
        "VANILLA_PRESSURE_PLATE_BLOCKS",
        "minecraft:pressure_plates",
        16,
    ),
    ("VANILLA_CROP_BLOCKS", "minecraft:crops", 8),
)

BLOCK_TAG_UNION_CATALOGS = (
    (
        "VANILLA_ORE_BLOCKS",
        (
            ("minecraft:coal_ores", 2),
            ("minecraft:copper_ores", 2),
            ("minecraft:diamond_ores", 2),
            ("minecraft:emerald_ores", 2),
            ("minecraft:gold_ores", 3),
            ("minecraft:iron_ores", 2),
            ("minecraft:lapis_ores", 2),
            ("minecraft:redstone_ores", 2),
        ),
        17,
    ),
)

CONSTRUCTOR_FAMILY_CATALOGS = (
    (
        "VANILLA_SAME_BLOCK_ALL_FACE_SKIP_RENDERING_BLOCKS",
        frozenset({"PowderSnowBlock"}),
        1,
    ),
    (
        "VANILLA_SAME_BLOCK_VERTICAL_FACE_SKIP_RENDERING_BLOCKS",
        frozenset({"MangroveRootsBlock"}),
        1,
    ),
    (
        "VANILLA_FLOOR_TORCH_BLOCKS",
        frozenset({"TorchBlock", "RedstoneTorchBlock"}),
        4,
    ),
    (
        "VANILLA_WALL_TORCH_BLOCKS",
        frozenset({"WallTorchBlock", "RedstoneWallTorchBlock"}),
        4,
    ),
    (
        "VANILLA_IRON_BARS_CLASS_BLOCKS",
        frozenset({"IronBarsBlock", "StainedGlassPaneBlock"}),
        18,
    ),
    (
        "VANILLA_COPPER_GRATE_BLOCKS",
        frozenset({"WeatheringCopperGrateBlock", "WaterloggedTransparentBlock"}),
        8,
    ),
    (
        "VANILLA_FIXED_SOURCE_WATER_BLOCKS",
        frozenset(
            {
                "BubbleColumnBlock",
                "SeagrassBlock",
                "TallSeagrassBlock",
                "KelpBlock",
                "KelpPlantBlock",
            }
        ),
        5,
    ),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def generate(source_root: Path) -> str:
    source = parse_source(
        BLOCKS_SOURCE.as_posix(), (source_root / BLOCKS_SOURCE).read_bytes()
    )
    block_ids_source = parse_source(
        BLOCK_IDS_SOURCE.as_posix(), (source_root / BLOCK_IDS_SOURCE).read_bytes()
    )
    block_id_constants = extract_block_id_constants(block_ids_source)
    block_entity_types_source = parse_source(
        BLOCK_ENTITY_TYPES_SOURCE.as_posix(),
        (source_root / BLOCK_ENTITY_TYPES_SOURCE).read_bytes(),
    )
    block_registry = extract_block_registry_catalog(
        source, block_ids_source, block_entity_types_source
    )
    block_resource_paths = {
        declaration["field"]: declaration["resource_id"].removeprefix("minecraft:")
        for declaration in block_registry.declarations
    }
    fire_source = parse_source(
        FIRE_BLOCK_SOURCE.as_posix(), (source_root / FIRE_BLOCK_SOURCE).read_bytes()
    )
    block_class_sources = [
        parse_source(path.relative_to(source_root).as_posix(), path.read_bytes())
        for path in sorted((source_root / BLOCK_CLASS_SOURCE_DIR).rglob("*.java"))
    ]
    invisible_render_shape_types = invisible_render_shape_constructor_types(
        block_class_sources
    )
    half_transparent_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.HalfTransparentBlock",
        10,
    )
    leaves_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.LeavesBlock",
        4,
    )
    sapling_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.SaplingBlock",
        2,
    )
    shelf_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.ShelfBlock",
        1,
    )
    chest_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.ChestBlock",
        4,
    )
    amethyst_cluster_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.AmethystClusterBlock",
        1,
    )
    floor_skull_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.SkullBlock",
        3,
    )
    wall_skull_types = constructor_types_extending(
        block_class_sources,
        "net.minecraft.world.level.block.WallSkullBlock",
        4,
    )
    entries = no_terrain_particle_blocks(source)
    no_collision_entries = no_collision_blocks(source, block_id_constants)
    air_entries = air_blocks(source)
    friction_entries = block_friction_overrides(source, block_id_constants)
    speed_factor_entries = block_speed_factor_overrides(source, block_id_constants)
    flammable_entries = fire_flammable_blocks(fire_source, block_resource_paths)
    constructor_catalogs = [
        (
            name,
            constructor_family_blocks(
                source, constructor_types, expected_count, block_id_constants
            ),
        )
        for name, constructor_types, expected_count in (
            *CONSTRUCTOR_FAMILY_CATALOGS,
            (
                "VANILLA_HALF_TRANSPARENT_SKIP_RENDERING_BLOCKS",
                half_transparent_types,
                31,
            ),
            ("VANILLA_LEAVES_BLOCKS", leaves_types, 11),
            ("VANILLA_SAPLING_BLOCKS", sapling_types, 9),
            ("VANILLA_SHELF_BLOCKS", shelf_types, 12),
            ("VANILLA_CHEST_BLOCKS", chest_types, 10),
            ("VANILLA_AMETHYST_CLUSTER_BLOCKS", amethyst_cluster_types, 4),
            ("VANILLA_FLOOR_SKULL_BLOCKS", floor_skull_types, 7),
            ("VANILLA_WALL_SKULL_BLOCKS", wall_skull_types, 7),
            (
                "VANILLA_INVISIBLE_RENDER_SHAPE_BLOCKS",
                invisible_render_shape_types,
                12,
            ),
        )
    ]
    tag_resources = load_tag_resources(source_root, BLOCK_TAG_SOURCE_DIR)
    tag_catalogs = [
        (name, block_tag_membership(tag_resources, tag_id, expected_count))
        for name, tag_id, expected_count in BLOCK_TAG_CATALOGS
    ]
    for name, sources, expected_count in BLOCK_TAG_UNION_CATALOGS:
        values = tuple(
            dict.fromkeys(
                resource_id
                for tag_id, source_count in sources
                for resource_id in block_tag_membership(
                    tag_resources, tag_id, source_count
                )
            )
        )
        if len(values) != expected_count:
            raise ValueError(
                f"{name}: expected {expected_count} unique tag-union values, "
                f"extracted {len(values)}"
            )
        tag_catalogs.append((name, values))
    catalog_names = (
        "VANILLA_NO_TERRAIN_PARTICLE_BLOCKS",
        "VANILLA_NO_COLLISION_BLOCKS",
        "VANILLA_AIR_BLOCKS",
        "VANILLA_FIRE_FLAMMABLE_BLOCKS",
        *(name for name, _ in constructor_catalogs),
        *(name for name, _ in tag_catalogs),
    )
    lines = [
        "// @generated by scripts/generate-block-membership-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 block Java declarations/behavior and block tag JSON.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-block-membership-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-block-membership-catalog.py --check",
        "",
        "pub(super) const VANILLA_NO_TERRAIN_PARTICLE_BLOCKS: &[&str] = &[",
        *(f'    "{entry.resource_id}",' for entry in entries),
        "];",
        "",
        "pub(super) const VANILLA_NO_COLLISION_BLOCKS: &[&str] = &[",
        *(f'    "{entry.resource_id}",' for entry in no_collision_entries),
        "];",
        "",
        "pub(super) const VANILLA_AIR_BLOCKS: &[&str] = &[",
        *(f'    "{entry.resource_id}",' for entry in air_entries),
        "];",
        "",
        "pub(super) const VANILLA_FIRE_FLAMMABLE_BLOCKS: &[&str] = &[",
        *(f'    "{entry.resource_id}",' for entry in flammable_entries),
        "];",
        "",
        "pub(super) const VANILLA_BLOCK_FRICTION_OVERRIDES: &[(&str, u32)] = &[",
        *(
            f'    ("{entry.resource_id}", {entry.value!r}_f32.to_bits()),'
            for entry in friction_entries
        ),
        "];",
        "",
        "pub(super) const VANILLA_BLOCK_SPEED_FACTOR_OVERRIDES: &[(&str, u32)] = &[",
        *(
            f'    ("{entry.resource_id}", {entry.value!r}_f32.to_bits()),'
            for entry in speed_factor_entries
        ),
        "];",
        "",
    ]
    for name, values in constructor_catalogs:
        lines.extend(
            [
                f"pub(super) const {name}: &[&str] = &[",
                *(f'    "{entry.resource_id}",' for entry in values),
                "];",
                "",
            ]
        )
    for name, values in tag_catalogs:
        lines.extend(
            [
                f"pub(super) const {name}: &[&str] = &[",
                *(f'    "{resource_id}",' for resource_id in values),
                "];",
                "",
            ]
        )
    lines.extend(
        [
            "#[cfg(test)]",
            "mod tests {",
            "    use std::{fs, path::Path};",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn block_memberships_match_vanilla_order() {",
            '        assert_eq!(VANILLA_NO_TERRAIN_PARTICLE_BLOCKS, &["minecraft:barrier", "minecraft:structure_void"]);',
            "        assert_eq!(VANILLA_NO_COLLISION_BLOCKS.len(), 255);",
            '        assert_eq!(VANILLA_AIR_BLOCKS, &["minecraft:air", "minecraft:void_air", "minecraft:cave_air"]);',
            "        assert_eq!(VANILLA_FIRE_FLAMMABLE_BLOCKS.len(), 207);",
            "        assert_eq!(VANILLA_BLOCK_FRICTION_OVERRIDES.len(), 5);",
            "        assert_eq!(VANILLA_BLOCK_SPEED_FACTOR_OVERRIDES.len(), 2);",
            '        assert_eq!(VANILLA_SAME_BLOCK_ALL_FACE_SKIP_RENDERING_BLOCKS, &["minecraft:powder_snow"]);',
            '        assert_eq!(VANILLA_SAME_BLOCK_VERTICAL_FACE_SKIP_RENDERING_BLOCKS, &["minecraft:mangrove_roots"]);',
            "        assert_eq!(VANILLA_FLOOR_TORCH_BLOCKS.len(), 4);",
            "        assert_eq!(VANILLA_WALL_TORCH_BLOCKS.len(), 4);",
            "        assert_eq!(VANILLA_IRON_BARS_CLASS_BLOCKS.len(), 18);",
            "        assert_eq!(VANILLA_COPPER_GRATE_BLOCKS.len(), 8);",
            "        assert_eq!(VANILLA_FIXED_SOURCE_WATER_BLOCKS.len(), 5);",
            "        assert_eq!(VANILLA_HALF_TRANSPARENT_SKIP_RENDERING_BLOCKS.len(), 31);",
            "        assert_eq!(VANILLA_LEAVES_BLOCKS.len(), 11);",
            "        assert_eq!(VANILLA_SAPLING_BLOCKS.len(), 9);",
            "        assert_eq!(VANILLA_SHELF_BLOCKS.len(), 12);",
            "        assert_eq!(VANILLA_CHEST_BLOCKS.len(), 10);",
            "        assert_eq!(VANILLA_AMETHYST_CLUSTER_BLOCKS.len(), 4);",
            "        assert_eq!(VANILLA_FLOOR_SKULL_BLOCKS.len(), 7);",
            "        assert_eq!(VANILLA_WALL_SKULL_BLOCKS.len(), 7);",
            "        assert_eq!(VANILLA_INVISIBLE_RENDER_SHAPE_BLOCKS.len(), 12);",
            "        assert_eq!(VANILLA_BARS_BLOCKS.len(), 9);",
            "        assert_eq!(VANILLA_SLAB_BLOCKS.len(), 62);",
            "        assert_eq!(VANILLA_STAIR_BLOCKS.len(), 58);",
            "        assert_eq!(VANILLA_WALL_BLOCKS.len(), 26);",
            "        assert_eq!(VANILLA_FENCE_BLOCKS.len(), 13);",
            "        assert_eq!(VANILLA_FENCE_GATE_BLOCKS.len(), 12);",
            "        assert_eq!(VANILLA_WOOL_CARPET_BLOCKS.len(), 16);",
            "        assert_eq!(VANILLA_FLOWER_POT_BLOCKS.len(), 39);",
            "        assert_eq!(VANILLA_BED_BLOCKS.len(), 16);",
            "        assert_eq!(VANILLA_CANDLE_BLOCKS.len(), 17);",
            "        assert_eq!(VANILLA_CANDLE_CAKE_BLOCKS.len(), 17);",
            "        assert_eq!(VANILLA_CHAIN_BLOCKS.len(), 9);",
            "        assert_eq!(VANILLA_COPPER_GOLEM_STATUE_BLOCKS.len(), 8);",
            "        assert_eq!(VANILLA_LANTERN_BLOCKS.len(), 10);",
            "        assert_eq!(VANILLA_LIGHTNING_ROD_BLOCKS.len(), 8);",
            "        assert_eq!(VANILLA_HANGING_SIGN_BLOCKS.len(), 24);",
            "        assert_eq!(VANILLA_SHULKER_BOX_BLOCKS.len(), 17);",
            "        assert_eq!(VANILLA_ANVIL_BLOCKS.len(), 3);",
            "        assert_eq!(VANILLA_CAMPFIRE_BLOCKS.len(), 2);",
            '        assert_eq!(VANILLA_SOUL_FIRE_BASE_BLOCKS, &["minecraft:soul_sand", "minecraft:soul_soil"]);',
            "        assert_eq!(VANILLA_CAULDRON_BLOCKS.len(), 4);",
            "        assert_eq!(VANILLA_LOG_BLOCKS.len(), 44);",
            "        assert_eq!(VANILLA_PLANK_BLOCKS.len(), 12);",
            "        assert_eq!(VANILLA_ORE_BLOCKS.len(), 17);",
            "        assert_eq!(VANILLA_DOOR_BLOCKS.len(), 21);",
            "        assert_eq!(VANILLA_TRAPDOOR_BLOCKS.len(), 21);",
            "        assert_eq!(VANILLA_RAIL_BLOCKS.len(), 4);",
            "        assert_eq!(VANILLA_STANDING_SIGN_BLOCKS.len(), 12);",
            "        assert_eq!(VANILLA_WALL_SIGN_BLOCKS.len(), 12);",
            "        assert_eq!(VANILLA_BUTTON_BLOCKS.len(), 14);",
            "        assert_eq!(VANILLA_PRESSURE_PLATE_BLOCKS.len(), 16);",
            "        assert_eq!(VANILLA_CROP_BLOCKS.len(), 8);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_block_memberships() {",
            "        fn has_exact_resource_literals(source: &str, resources: &[&str]) -> bool {",
            "            source.matches(\"\\\"minecraft:\").count() == resources.len() && resources.iter().all(|resource| source.contains(&format!(\"\\\"{resource}\\\"\")))",
            "        }",
            "",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/terrain/generated_26_1.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.trim_start().starts_with(\"// @generated\") { continue; }",
            "                if production.contains(\"fn is_vanilla_rail_block_name(\") || production.contains(\"fn is_custom_head_skull_item_id(\") { violations.push(format!(\"{}: restored removed block membership lookup\", display)); }",
            "                for line in production.lines().filter(|line| line.contains(\"const \" ) || line.contains(\"static \")) {",
            "                    for name in [",
            '                        "VANILLA_BLOCK_FRICTION_OVERRIDES",',
            *(f'                        "{name}",' for name in catalog_names),
            "                    ] {",
            "                        if line.contains(name) { violations.push(format!(\"{}: parallel block membership declaration `{}`\", display, line.trim())); }",
            "                    }",
            "                }",
            "                for expression in production.split(\"matches!(\").skip(1).filter_map(|tail| tail.split_once(')').map(|(expression, _)| expression)) {",
            "                    for (name, resources) in [",
            *(f'                    ("{name}", {name}),' for name in catalog_names),
            "                    ] {",
            "                        if resources.len() >= 4 && has_exact_resource_literals(expression, resources) { violations.push(format!(\"{}: parallel {name} resource membership\", display)); }",
            "                    }",
            "                }",
            "            }",
            "        }",
            "",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel block memberships:\\n{}\", violations.join(\"\\n\"));",
            "    }",
            "}",
            "",
        ]
    )
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input="\n".join(lines),
        text=True,
        capture_output=True,
        check=True,
    )
    return formatted.stdout


def check_generated(path: Path, generated: str) -> bool:
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
    generated = generate(args.source_root)
    if args.check:
        return 0 if check_generated(args.output, generated) else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
