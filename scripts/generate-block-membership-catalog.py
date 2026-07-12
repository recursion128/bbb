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
    block_tag_membership,
    constructor_family_blocks,
    constructor_types_extending,
    invisible_render_shape_constructor_types,
    no_terrain_particle_blocks,
)
from block_registry_catalog_extractor import extract_block_id_constants
from vanilla_java_source import parse_source
from vanilla_tag_source import load_tag_resources


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-world/src/terrain/generated_26_1.rs"
BLOCKS_SOURCE = Path("net/minecraft/world/level/block/Blocks.java")
BLOCK_IDS_SOURCE = Path("net/minecraft/references/BlockIds.java")
BLOCK_TAG_SOURCE_DIR = Path("data/minecraft/tags/block")
BLOCK_CLASS_SOURCE_DIR = Path("net/minecraft/world/level/block")

BLOCK_TAG_CATALOGS = (
    ("VANILLA_BARS_BLOCKS", "minecraft:bars", 9),
    ("VANILLA_DOOR_BLOCKS", "minecraft:doors", 21),
    ("VANILLA_TRAPDOOR_BLOCKS", "minecraft:trapdoors", 21),
    ("VANILLA_NO_COLLISION_RAIL_BLOCKS", "minecraft:rails", 4),
    ("VANILLA_NO_COLLISION_SIGN_BLOCKS", "minecraft:all_signs", 48),
    ("VANILLA_NO_COLLISION_BUTTON_BLOCKS", "minecraft:buttons", 14),
    (
        "VANILLA_NO_COLLISION_PRESSURE_PLATE_BLOCKS",
        "minecraft:pressure_plates",
        16,
    ),
    ("VANILLA_NO_COLLISION_BANNER_BLOCKS", "minecraft:banners", 32),
    ("VANILLA_NO_COLLISION_SMALL_FLOWER_BLOCKS", "minecraft:small_flowers", 17),
    ("VANILLA_NO_COLLISION_CROP_BLOCKS", "minecraft:crops", 8),
    ("VANILLA_NO_COLLISION_FIRE_BLOCKS", "minecraft:fire", 2),
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
        "VANILLA_NO_COLLISION_TORCH_BLOCKS",
        frozenset(
            {
                "TorchBlock",
                "WallTorchBlock",
                "RedstoneTorchBlock",
                "RedstoneWallTorchBlock",
            }
        ),
        8,
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
    (
        "VANILLA_NO_COLLISION_CROP_SUPPLEMENT_BLOCKS",
        frozenset({"NetherWartBlock", "AttachedStemBlock", "SugarCaneBlock"}),
        4,
    ),
    (
        "VANILLA_NO_COLLISION_WIRE_BLOCKS",
        frozenset({"RedStoneWireBlock", "TripWireBlock", "TripWireHookBlock"}),
        3,
    ),
    (
        "VANILLA_NO_COLLISION_THIN_OVERLAY_BLOCKS",
        frozenset(
            {
                "FlowerBedBlock",
                "LeafLitterBlock",
                "SculkVeinBlock",
                "GlowLichenBlock",
                "MultifaceBlock",
                "FrogspawnBlock",
            }
        ),
        7,
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
    entries = no_terrain_particle_blocks(source)
    air_entries = air_blocks(source)
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
        "pub(super) const VANILLA_AIR_BLOCKS: &[&str] = &[",
        *(f'    "{entry.resource_id}",' for entry in air_entries),
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
            "    use super::*;",
            "",
            "    #[test]",
            "    fn block_memberships_match_vanilla_order() {",
            '        assert_eq!(VANILLA_NO_TERRAIN_PARTICLE_BLOCKS, &["minecraft:barrier", "minecraft:structure_void"]);',
            '        assert_eq!(VANILLA_AIR_BLOCKS, &["minecraft:air", "minecraft:void_air", "minecraft:cave_air"]);',
            '        assert_eq!(VANILLA_SAME_BLOCK_ALL_FACE_SKIP_RENDERING_BLOCKS, &["minecraft:powder_snow"]);',
            '        assert_eq!(VANILLA_SAME_BLOCK_VERTICAL_FACE_SKIP_RENDERING_BLOCKS, &["minecraft:mangrove_roots"]);',
            "        assert_eq!(VANILLA_NO_COLLISION_TORCH_BLOCKS.len(), 8);",
            "        assert_eq!(VANILLA_IRON_BARS_CLASS_BLOCKS.len(), 18);",
            "        assert_eq!(VANILLA_COPPER_GRATE_BLOCKS.len(), 8);",
            "        assert_eq!(VANILLA_FIXED_SOURCE_WATER_BLOCKS.len(), 5);",
            "        assert_eq!(VANILLA_NO_COLLISION_CROP_SUPPLEMENT_BLOCKS.len(), 4);",
            "        assert_eq!(VANILLA_NO_COLLISION_WIRE_BLOCKS.len(), 3);",
            "        assert_eq!(VANILLA_NO_COLLISION_THIN_OVERLAY_BLOCKS.len(), 7);",
            "        assert_eq!(VANILLA_HALF_TRANSPARENT_SKIP_RENDERING_BLOCKS.len(), 31);",
            "        assert_eq!(VANILLA_LEAVES_BLOCKS.len(), 11);",
            "        assert_eq!(VANILLA_INVISIBLE_RENDER_SHAPE_BLOCKS.len(), 12);",
            "        assert_eq!(VANILLA_BARS_BLOCKS.len(), 9);",
            "        assert_eq!(VANILLA_DOOR_BLOCKS.len(), 21);",
            "        assert_eq!(VANILLA_TRAPDOOR_BLOCKS.len(), 21);",
            "        assert_eq!(VANILLA_NO_COLLISION_RAIL_BLOCKS.len(), 4);",
            "        assert_eq!(VANILLA_NO_COLLISION_SIGN_BLOCKS.len(), 48);",
            "        assert_eq!(VANILLA_NO_COLLISION_BUTTON_BLOCKS.len(), 14);",
            "        assert_eq!(VANILLA_NO_COLLISION_PRESSURE_PLATE_BLOCKS.len(), 16);",
            "        assert_eq!(VANILLA_NO_COLLISION_BANNER_BLOCKS.len(), 32);",
            "        assert_eq!(VANILLA_NO_COLLISION_SMALL_FLOWER_BLOCKS.len(), 17);",
            "        assert_eq!(VANILLA_NO_COLLISION_CROP_BLOCKS.len(), 8);",
            '        assert_eq!(VANILLA_NO_COLLISION_FIRE_BLOCKS, &["minecraft:fire", "minecraft:soul_fire"]);',
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
