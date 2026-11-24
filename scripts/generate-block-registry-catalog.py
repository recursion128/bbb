#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 block registry declaration facts."""

from __future__ import annotations

import argparse
import difflib
import json
import struct
import subprocess
import sys
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
from block_explosion_resistance import extract_block_explosion_resistances
from block_registry_catalog_extractor import (
    block_factory_class,
    extract_beacon_beam_colors,
    extract_block_affect_neighbors_after_removal_override_owners,
    extract_block_analog_output_owners,
    extract_block_bonemeal_target_override_owners,
    extract_block_bucket_pickup_owners,
    extract_block_can_survive_override_owners,
    extract_block_clone_item_stack_override_owners,
    extract_block_constructor_block_ids,
    extract_block_constructor_block_tags,
    extract_block_get_state_for_placement_override_owners,
    extract_block_interface_membership,
    extract_block_liquid_container_owners,
    extract_block_loot_table_membership,
    extract_block_may_place_on_override_owners,
    extract_block_may_place_on_support_tags,
    extract_block_menu_provider_owners,
    extract_block_neighbor_changed_override_owners,
    extract_block_on_place_override_owners,
    extract_block_propagates_skylight_down_override_owners,
    extract_block_random_tick_override_owners,
    extract_block_redstone_signal_owners,
    extract_block_registry_catalog,
    extract_block_replaceable_membership,
    extract_block_subclass_membership,
    extract_block_tick_override_owners,
    extract_block_use_item_on_override_owners,
    extract_block_use_without_item_override_owners,
    extract_block_visual_shape_override_owners,
    extract_change_over_time_ages,
    extract_concrete_powder_targets,
    extract_fire_block_odds,
    extract_weathering_next_blocks,
)
from vanilla_java_source import parse_source

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-client-assets/data/block_registry_declarations_26_1.json"
DEFAULT_PROTOCOL_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/block_types.rs"
DEFAULT_PROTOCOL_BIN_OUTPUT = REPO_ROOT / "crates/bbb-facts/data/block_types_26_1.bin"
BLOCKS_PATH = Path("net/minecraft/world/level/block/Blocks.java")
BLOCK_IDS_PATH = Path("net/minecraft/references/BlockIds.java")
BLOCK_ENTITY_TYPES_PATH = Path(
    "net/minecraft/world/level/block/entity/BlockEntityType.java"
)
BLOCK_TAGS_PATH = Path("net/minecraft/tags/BlockTags.java")
PROPERTIES_PATH = Path("net/minecraft/world/level/block/state/BlockBehaviour.java")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--protocol-output", type=Path, default=DEFAULT_PROTOCOL_OUTPUT
    )
    parser.add_argument(
        "--protocol-bin-output", type=Path, default=DEFAULT_PROTOCOL_BIN_OUTPUT
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


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
    random_tick_owner_by_factory: dict[str, str | None],
    tick_owner_by_factory: dict[str, str | None],
    neighbor_changed_owner_by_factory: dict[str, str | None],
    on_place_owner_by_factory: dict[str, str | None],
    affect_neighbors_after_removal_owner_by_factory: dict[str, str | None],
    may_place_on_owner_by_factory: dict[str, str | None],
    may_place_on_support_tag_by_field: dict[str, str | None],
    constructor_block_id_by_field: dict[str, tuple[tuple[str, str], ...]],
    constructor_block_tag_by_field: dict[str, tuple[tuple[str, str], ...]],
    clone_item_stack_owner_by_factory: dict[str, str | None],
    get_state_for_placement_owner_by_factory: dict[str, str | None],
    bucket_pickup_owner_by_factory: dict[str, str | None],
    liquid_container_owner_by_factory: dict[str, str | None],
    growing_plant_head_by_factory: dict[str, bool],
    game_master_block_by_factory: dict[str, bool],
    bonemeal_target_owner_by_factory: dict[str, str | None],
    skylight_down_owner_by_factory: dict[str, str | None],
    visual_shape_owner_by_factory: dict[str, str | None],
    menu_provider_owner_by_factory: dict[str, str | None],
    replaceable_by_field: dict[str, bool],
    concrete_powder_targets: dict[str, str],
    beacon_beam_colors: dict[str, str],
    change_over_time_ages: dict[str, int],
    weathering_next_blocks: dict[str, str],
    fire_block_odds: dict[str, tuple[int, int]],
    explosion_resistances: dict[str, int],
    has_loot_table_by_field: dict[str, bool],
) -> tuple[str, bytes]:
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
    random_tick_owners = [
        random_tick_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    tick_owners = [
        tick_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    neighbor_changed_owners = [
        neighbor_changed_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    on_place_owners = [
        on_place_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    affect_neighbors_after_removal_owners = [
        affect_neighbors_after_removal_owner_by_factory[
            block_factory_class(declaration)
        ]
        for declaration in catalog.declarations
    ]
    may_place_on_owners = [
        may_place_on_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    may_place_on_support_tags = [
        may_place_on_support_tag_by_field[declaration["field"]]
        for declaration in catalog.declarations
    ]
    constructor_block_ids = [
        constructor_block_id_by_field[declaration["field"]]
        for declaration in catalog.declarations
    ]
    constructor_block_tags = [
        constructor_block_tag_by_field[declaration["field"]]
        for declaration in catalog.declarations
    ]
    clone_item_stack_owners = [
        clone_item_stack_owner_by_factory[block_factory_class(declaration)]
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
    visual_shape_owners = [
        visual_shape_owner_by_factory[block_factory_class(declaration)]
        for declaration in catalog.declarations
    ]
    menu_provider_owners = [
        menu_provider_owner_by_factory[block_factory_class(declaration)]
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
    random_tick_variants = sorted(
        {owner for owner in random_tick_owners if owner is not None}
    )
    tick_variants = sorted({owner for owner in tick_owners if owner is not None})
    neighbor_changed_variants = sorted(
        {owner for owner in neighbor_changed_owners if owner is not None}
    )
    on_place_variants = sorted(
        {owner for owner in on_place_owners if owner is not None}
    )
    affect_neighbors_after_removal_variants = sorted(
        {
            owner
            for owner in affect_neighbors_after_removal_owners
            if owner is not None
        }
    )
    may_place_on_variants = sorted(
        {owner for owner in may_place_on_owners if owner is not None}
    )
    clone_item_stack_variants = sorted(
        {owner for owner in clone_item_stack_owners if owner is not None}
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
    visual_shape_variants = sorted(
        {owner for owner in visual_shape_owners if owner is not None}
    )
    menu_provider_variants = sorted(
        {owner for owner in menu_provider_owners if owner is not None}
    )
    lines = [
        "// @generated by scripts/generate-block-registry-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Blocks.java registration order, replaceable properties, and ConcretePowderBlock targets; FireBlock.bootStrap ignite/burn odds; BlockBehaviour interaction/analog-output/canSurvive/getCloneItemStack/getStateForPlacement/getVisualShape overrides; BlockBehaviour block-update overrides (tick, neighborChanged, onPlace, affectNeighborsAfterRemoval); VegetationBlock.mayPlaceOn owners and BlockTags support-tag constructor arguments; BucketPickup/LiquidBlockContainer/GameMasterBlock interface owners; BonemealableBlock target overrides; and block class hierarchy.",
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
            "/// The class whose `BlockBehaviour.randomTick` body a block reaches.",
            "///",
            "/// `BlockBehaviour.randomTick` is empty, so `None` is \"nothing happens\"",
            "/// rather than \"unknown\"; a name is the class that declares the override,",
            "/// which for a method no subclass touches is the whole class.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaRandomTickOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in random_tick_variants)
    lines.extend(
        [
            "}",
            "",
            "/// The class whose `BlockBehaviour.tick` body a block reaches.",
            "///",
            "/// This is the *scheduled* tick -- what `ServerLevel` runs when a block",
            "/// ticket comes due -- and is a different member from `randomTick` even",
            "/// though the two share a signature. `BlockBehaviour.tick` is empty",
            "/// (`BlockBehaviour.java:352-353`), so `None` is \"a scheduled tick does",
            "/// nothing here\" rather than \"unknown\".",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaTickOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in tick_variants)
    lines.extend(
        [
            "}",
            "",
            "/// The class whose `BlockBehaviour.neighborChanged` body a block reaches.",
            "///",
            "/// This is the block-update entry point: the body that runs when a",
            "/// neighbouring block changes. `BlockBehaviour.neighborChanged` is empty",
            "/// (`BlockBehaviour.java:166-169`), so `None` is \"this block ignores its",
            "/// neighbours\".",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaNeighborChangedOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in neighbor_changed_variants)
    lines.extend(
        [
            "}",
            "",
            "/// The class whose `BlockBehaviour.onPlace` body a block reaches.",
            "///",
            "/// `onPlace` runs once the new state is already in the level, which is",
            "/// where a redstone component seeds its first scheduled tick.",
            "/// `BlockBehaviour.onPlace` is empty (`BlockBehaviour.java:171-172`), so",
            "/// `None` is \"placing this block starts nothing\".",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaOnPlaceOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in on_place_variants)
    lines.extend(
        [
            "}",
            "",
            "/// The class whose `BlockBehaviour.affectNeighborsAfterRemoval` body a",
            "/// block reaches.",
            "///",
            "/// The removal half of the update pair: the body that runs once the block",
            "/// is gone, where a power source drops the signal it was feeding and a",
            "/// container spills its contents. The base declaration is empty",
            "/// (`BlockBehaviour.java:174-175`), so `None` is \"removing this block",
            "/// disturbs nothing\".",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaAffectNeighborsAfterRemovalOverride {",
        ]
    )
    lines.extend(
        f"    {owner}," for owner in affect_neighbors_after_removal_variants
    )
    lines.extend(
        [
            "}",
            "",
            "/// The class whose `VegetationBlock.mayPlaceOn` body a block reaches.",
            "///",
            "/// `mayPlaceOn` is declared by `VegetationBlock`, not by",
            "/// `BlockBehaviour`, so `None` means the block is outside that",
            "/// hierarchy rather than that it accepts the default.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaMayPlaceOnOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in may_place_on_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaCloneItemStackOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in clone_item_stack_variants)
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
            "/// The class owning a block's `BlockBehaviour.getVisualShape` override,",
            "/// which the `ClipContext.Block.VISUAL` ray and the client's",
            "/// `BlockState.getVisualShape` reader both reach. Only these classes",
            "/// answer something other than the collision shape.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaVisualShapeOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in visual_shape_variants)
    lines.extend(
        [
            "}",
            "",
            "/// The class owning a block's `BlockBehaviour.getMenuProvider` override.",
            "/// `BaseEntityBlock` defers to whether the block entity at the position is",
            "/// itself a `MenuProvider`; `ChestBlock` and `LecternBlock` add the double-chest",
            "/// blocked check and the `HAS_BOOK` gate; the rest always provide a menu.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub enum VanillaBlockMenuProviderOverride {",
        ]
    )
    lines.extend(f"    {owner}," for owner in menu_provider_variants)
    lines.extend(
        [
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct BlockTypeCatalogEntry {",
            "    pub id: i32,",
            "    pub constant_name: &'static str,",
            "    pub resource_id: &'static str,",
            "    pub has_loot_table: bool,",
            "    pub block_entity_type_id: Option<i32>,",
            "    pub analog_output_override: Option<VanillaBlockAnalogOutputOverride>,",
            "    pub is_signal_source_override: Option<VanillaBlockRedstoneSignalOverride>,",
            "    pub signal_override: Option<VanillaBlockRedstoneSignalOverride>,",
            "    pub direct_signal_override: Option<VanillaBlockRedstoneSignalOverride>,",
            "    pub use_item_on_override: Option<VanillaBlockUseItemOnOverride>,",
            "    pub use_without_item_override: Option<VanillaBlockUseWithoutItemOverride>,",
            "    pub can_survive_override: Option<VanillaCanSurviveOverride>,",
            "    /// `BlockBehaviour.randomTick`: the body the random-tick",
            "    /// scheduler reaches for this block, or `None` when the block",
            "    /// inherits the empty default.",
            "    pub random_tick_override: Option<VanillaRandomTickOverride>,",
            "    /// `BlockBehaviour.tick`: the body a scheduled tick reaches for",
            "    /// this block, or `None` when the block inherits the empty",
            "    /// default. Distinct from `random_tick_override`.",
            "    pub tick_override: Option<VanillaTickOverride>,",
            "    /// `BlockBehaviour.neighborChanged`: the body a neighbour update",
            "    /// reaches for this block, or `None` when it inherits the empty",
            "    /// default.",
            "    pub neighbor_changed_override: Option<VanillaNeighborChangedOverride>,",
            "    /// `BlockBehaviour.onPlace`: the body that runs after this block's",
            "    /// state lands in the level, or `None` when it inherits the empty",
            "    /// default.",
            "    pub on_place_override: Option<VanillaOnPlaceOverride>,",
            "    /// `BlockBehaviour.affectNeighborsAfterRemoval`: the body that runs",
            "    /// once this block is gone, or `None` when it inherits the empty",
            "    /// default.",
            "    pub affect_neighbors_after_removal_override:",
            "        Option<VanillaAffectNeighborsAfterRemovalOverride>,",
            "    /// `VegetationBlock.mayPlaceOn`: the body",
            "    /// `VegetationBlock.canSurvive` reaches for this block, or `None`",
            "    /// when the block is not a `VegetationBlock` at all.",
            "    pub may_place_on_override: Option<VanillaMayPlaceOnOverride>,",
            "    /// The `TagKey<Block>` this block's registration hands the",
            "    /// constructor `mayPlaceOn` reads, for the owners whose body is",
            "    /// `state.is(this.<field>)` instead of a tag the class names.",
            "    pub may_place_on_support_tag: Option<&'static str>,",
            "    /// Every `ResourceKey<Block>` this block's registration hands its",
            "    /// own constructor, as the instance field the constructor stores it",
            "    /// in paired with the block resource id it names, sorted by field",
            "    /// name. Empty when the class takes no such argument.",
            "    pub constructor_block_ids: &'static [(&'static str, &'static str)],",
            "    /// The same for every `TagKey<Block>` constructor argument, paired",
            "    /// with the tag resource id it names.",
            "    pub constructor_block_tags: &'static [(&'static str, &'static str)],",
            "    /// `BlockBehaviour.getCloneItemStack`: the block whose middle-click",
            "    /// pick answers something other than `new ItemStack(asItem())`.",
            "    pub clone_item_stack_override: Option<VanillaCloneItemStackOverride>,",
            "    pub get_state_for_placement_override: Option<VanillaGetStateForPlacementOverride>,",
            "    pub bucket_pickup_override: Option<VanillaBucketPickupOverride>,",
            "    pub liquid_block_container_override: Option<VanillaLiquidBlockContainerOverride>,",
            "    pub growing_plant_head: bool,",
            "    pub bonemeal_target_override: Option<VanillaBonemealTargetOverride>,",
            "    pub propagates_skylight_down_override: Option<VanillaPropagatesSkylightDownOverride>,",
            "    pub visual_shape_override: Option<VanillaVisualShapeOverride>,",
            "    pub menu_provider_override: Option<VanillaBlockMenuProviderOverride>,",
            "    pub replaceable: bool,",
            "    /// `FireBlock.bootStrap` odds. Both are zero for blocks omitted",
            "    /// from the source table; waterlogged states suppress them at runtime.",
            "    pub fire_ignite_odds: i32,",
            "    pub fire_burn_odds: i32,",
            "    /// `BeaconBeamBlock.getColor()` for the blocks `BeaconBlockEntity.tick`",
            "    /// reads while walking the beam upward.",
            "    pub beacon_beam_dye_color_id: Option<i32>,",
            "    /// `ChangeOverTimeBlock.getAge().ordinal()`, i.e. the oxidation age the",
            "    /// registration hands the constructor, or `None` when the block is not a",
            "    /// `ChangeOverTimeBlock` at all -- which is the `instanceof` test",
            "    /// `ChangeOverTimeBlock.getNextState` runs on every neighbour",
            "    /// (`ChangeOverTimeBlock.java:36`).",
            "    ///",
            "    /// In 26.1 every `ChangeOverTimeBlock` is a `WeatheringCopper`",
            "    /// (`WeatheringCopper.java:17`), so the enum behind the ordinal is always",
            "    /// `WeatheringCopper.WeatherState` (`WeatheringCopper.java:99-103`) and the",
            "    /// `getAge().getClass()` comparison on `:38` is one every member passes.",
            "    /// The generator refuses to emit this column if that stops being true.",
            "    pub change_over_time_age: Option<i32>,",
            "    /// `WeatheringCopper.NEXT_BY_BLOCK` (`WeatheringCopper.java:18-60`): the",
            "    /// block one oxidation step further along, or `None` for a block the map",
            "    /// has no entry for -- which is exactly what",
            "    /// `WeatheringCopperFullBlock.isRandomlyTicking` asks",
            "    /// (`WeatheringCopperFullBlock.java:34-36`).",
            "    pub change_over_time_next_block_type_id: Option<i32>,",
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

    block_entity_variants = sorted(
        {
            declaration["block_entity_type"]
            for declaration in catalog.declarations
            if declaration["block_entity_type"] is not None
        }
    )
    dye_color_variants = sorted(set(beacon_beam_colors.values()))
    block_id_by_field = {
        declaration["field"]: declaration["declaration_order"]
        for declaration in catalog.declarations
    }

    def tag_map(variants: list[str]) -> dict[str, int]:
        if len(variants) >= 255:
            raise ValueError(f"too many enum variants for u8 tag: {len(variants)}")
        return {variant: idx + 1 for idx, variant in enumerate(variants)}

    block_entity_tags = tag_map(block_entity_variants)
    dye_color_tags = tag_map(dye_color_variants)
    analog_output_tags = tag_map(analog_output_variants)
    redstone_signal_tags = tag_map(redstone_signal_variants)
    use_item_on_tags = tag_map(use_item_on_variants)
    use_without_item_tags = tag_map(use_without_item_variants)
    can_survive_tags = tag_map(can_survive_variants)
    random_tick_tags = tag_map(random_tick_variants)
    tick_tags = tag_map(tick_variants)
    neighbor_changed_tags = tag_map(neighbor_changed_variants)
    on_place_tags = tag_map(on_place_variants)
    affect_removal_tags = tag_map(affect_neighbors_after_removal_variants)
    may_place_on_tags = tag_map(may_place_on_variants)
    clone_item_stack_tags = tag_map(clone_item_stack_variants)
    placement_tags = tag_map(get_state_for_placement_variants)
    bucket_pickup_tags = tag_map(bucket_pickup_variants)
    liquid_container_tags = tag_map(liquid_container_variants)
    bonemeal_target_tags = tag_map(bonemeal_target_variants)
    skylight_down_tags = tag_map(skylight_down_variants)
    visual_shape_tags = tag_map(visual_shape_variants)
    menu_provider_tags = tag_map(menu_provider_variants)

    string_pool = bytearray()
    string_offsets: dict[str, tuple[int, int]] = {}

    def intern_str(text: str) -> tuple[int, int]:
        cached = string_offsets.get(text)
        if cached is not None:
            return cached
        raw = text.encode("utf-8")
        if not raw.isascii():
            raise ValueError(f"non-ASCII block catalog string: {text!r}")
        if len(raw) >= 0xFFFF:
            raise ValueError(f"string exceeds u16 length: {len(raw)}")
        entry = (len(string_pool), len(raw))
        string_pool.extend(raw)
        string_offsets[text] = entry
        return entry

    pair_records: list[tuple[int, int, int, int]] = []
    pair_slice_cache: dict[tuple[tuple[str, str], ...], tuple[int, int]] = {}

    def intern_pairs(pairs: tuple[tuple[str, str], ...]) -> tuple[int, int]:
        if not pairs:
            return (0, 0)
        cached = pair_slice_cache.get(pairs)
        if cached is not None:
            return cached
        start = len(pair_records)
        if start > 0xFFFF or len(pairs) > 0xFF:
            raise ValueError("constructor pair slice exceeds packed bounds")
        for field_name, target_value in pairs:
            f_off, f_len = intern_str(field_name)
            v_off, v_len = intern_str(target_value)
            pair_records.append((f_off, f_len, v_off, v_len))
        result = (start, len(pairs))
        pair_slice_cache[pairs] = result
        return result

    record_struct = struct.Struct("<IHIHIHHBHB25BHHHI")
    packed_records = bytearray()
    for idx, (
        declaration,
        analog_output_owner,
        is_signal_source_owner,
        signal_owner,
        direct_signal_owner,
        use_item_on_owner,
        use_without_item_owner,
        can_survive_owner,
        random_tick_owner,
        tick_owner,
        neighbor_changed_owner,
        on_place_owner,
        affect_neighbors_after_removal_owner,
        may_place_on_owner,
        may_place_on_support_tag,
        constructor_block_id,
        constructor_block_tag,
        clone_item_stack_owner,
        get_state_for_placement_owner,
        bucket_pickup_owner,
        liquid_container_owner,
        growing_plant_head,
        bonemeal_target_owner,
        skylight_down_owner,
        visual_shape_owner,
        menu_provider_owner,
    ) in enumerate(
        zip(
            catalog.declarations,
            analog_output_owners,
            is_signal_source_owners,
            signal_owners,
            direct_signal_owners,
            use_item_on_owners,
            use_without_item_owners,
            can_survive_owners,
            random_tick_owners,
            tick_owners,
            neighbor_changed_owners,
            on_place_owners,
            affect_neighbors_after_removal_owners,
            may_place_on_owners,
            may_place_on_support_tags,
            constructor_block_ids,
            constructor_block_tags,
            clone_item_stack_owners,
            get_state_for_placement_owners,
            bucket_pickup_owners,
            liquid_container_owners,
            growing_plant_heads,
            bonemeal_target_owners,
            skylight_down_owners,
            visual_shape_owners,
            menu_provider_owners,
            strict=True,
        )
    ):
        field = declaration["field"]
        if declaration["declaration_order"] != idx:
            raise ValueError(
                f"declaration order mismatch at {idx}: {declaration['declaration_order']}"
            )
        const_off, const_len = intern_str(field)
        res_off, res_len = intern_str(declaration["resource_id"])
        if may_place_on_support_tag is not None:
            sup_off, sup_len = intern_str(may_place_on_support_tag)
        else:
            sup_off, sup_len = (0, 0xFFFF)
        ctor_ids_start, ctor_ids_len = intern_pairs(constructor_block_id)
        ctor_tags_start, ctor_tags_len = intern_pairs(constructor_block_tag)
        flags = (
            (1 if has_loot_table_by_field[field] else 0)
            | (2 if growing_plant_head else 0)
            | (4 if replaceable_by_field[field] else 0)
        )
        be_type = declaration["block_entity_type"]
        ignite_odds, burn_odds = fire_block_odds.get(field, (0, 0))
        if not (0 <= ignite_odds <= 0xFFFF and 0 <= burn_odds <= 0xFFFF):
            raise ValueError(f"fire odds out of u16 range for {field}")
        age_opt = change_over_time_ages.get(field)
        if age_opt is not None and not (0 <= age_opt < 0xFF):
            raise ValueError(f"change_over_time_age out of u8 range for {field}")
        age_byte = 0xFF if age_opt is None else age_opt
        next_field = weathering_next_blocks.get(field)
        next_id = 0xFFFF if next_field is None else block_id_by_field[next_field]
        packed_records.extend(
            record_struct.pack(
                const_off,
                const_len,
                res_off,
                res_len,
                sup_off,
                sup_len,
                ctor_ids_start,
                ctor_ids_len,
                ctor_tags_start,
                ctor_tags_len,
                flags,
                0 if be_type is None else block_entity_tags[be_type],
                0 if analog_output_owner is None else analog_output_tags[analog_output_owner],
                0
                if is_signal_source_owner is None
                else redstone_signal_tags[is_signal_source_owner],
                0 if signal_owner is None else redstone_signal_tags[signal_owner],
                0
                if direct_signal_owner is None
                else redstone_signal_tags[direct_signal_owner],
                0 if use_item_on_owner is None else use_item_on_tags[use_item_on_owner],
                0
                if use_without_item_owner is None
                else use_without_item_tags[use_without_item_owner],
                0 if can_survive_owner is None else can_survive_tags[can_survive_owner],
                0 if random_tick_owner is None else random_tick_tags[random_tick_owner],
                0 if tick_owner is None else tick_tags[tick_owner],
                0
                if neighbor_changed_owner is None
                else neighbor_changed_tags[neighbor_changed_owner],
                0 if on_place_owner is None else on_place_tags[on_place_owner],
                0
                if affect_neighbors_after_removal_owner is None
                else affect_removal_tags[affect_neighbors_after_removal_owner],
                0 if may_place_on_owner is None else may_place_on_tags[may_place_on_owner],
                0
                if clone_item_stack_owner is None
                else clone_item_stack_tags[clone_item_stack_owner],
                0
                if get_state_for_placement_owner is None
                else placement_tags[get_state_for_placement_owner],
                0 if bucket_pickup_owner is None else bucket_pickup_tags[bucket_pickup_owner],
                0
                if liquid_container_owner is None
                else liquid_container_tags[liquid_container_owner],
                0
                if bonemeal_target_owner is None
                else bonemeal_target_tags[bonemeal_target_owner],
                0 if skylight_down_owner is None else skylight_down_tags[skylight_down_owner],
                0 if visual_shape_owner is None else visual_shape_tags[visual_shape_owner],
                0 if menu_provider_owner is None else menu_provider_tags[menu_provider_owner],
                0
                if field not in beacon_beam_colors
                else dye_color_tags[beacon_beam_colors[field]],
                age_byte,
                next_id,
                ignite_odds,
                burn_odds,
                explosion_resistances[field],
            )
        )

    pairs_blob = bytearray()
    pair_struct = struct.Struct("<IHIH")
    for f_off, f_len, v_off, v_len in pair_records:
        pairs_blob.extend(pair_struct.pack(f_off, f_len, v_off, v_len))

    pairs_offset = len(string_pool)
    records_offset = pairs_offset + len(pairs_blob)
    blob = bytes(string_pool + pairs_blob + packed_records)

    def emit_enum_table(table_name: str, enum_name: str, variants: list[str]) -> None:
        lines.append(
            f"const {table_name}: [Option<{enum_name}>; {len(variants) + 1}] = ["
        )
        lines.append("    None,")
        for variant in variants:
            lines.append(f"    Some({enum_name}::{variant}),")
        lines.append("];")
        lines.append("")

    lines.extend(
        (
            'const BYTES: &[u8] = include_bytes!("../data/block_types_26_1.bin");',
            f"const BLOCK_COUNT: usize = {len(catalog.declarations)};",
            f"const PAIR_COUNT: usize = {len(pair_records)};",
            f"const PAIRS_OFFSET: usize = {pairs_offset};",
            f"const RECORDS_OFFSET: usize = {records_offset};",
            f"const RECORD_STRIDE: usize = {record_struct.size};",
            "const STRINGS_END: usize = PAIRS_OFFSET;",
            "",
            "const _: () = {",
            "    let mut i = 0;",
            "    while i + 8 <= STRINGS_END {",
            "        let word = u64::from_ne_bytes([",
            "            BYTES[i],",
            "            BYTES[i + 1],",
            "            BYTES[i + 2],",
            "            BYTES[i + 3],",
            "            BYTES[i + 4],",
            "            BYTES[i + 5],",
            "            BYTES[i + 6],",
            "            BYTES[i + 7],",
            "        ]);",
            "        assert!(word & 0x8080_8080_8080_8080 == 0);",
            "        i += 8;",
            "    }",
            "    while i < STRINGS_END {",
            "        assert!(BYTES[i] < 0x80);",
            "        i += 1;",
            "    }",
            "};",
            "",
            "macro_rules! u16_at {",
            "    ($offset:expr) => {{",
            "        let off = $offset;",
            "        u16::from_le_bytes([BYTES[off], BYTES[off + 1]])",
            "    }};",
            "}",
            "",
            "macro_rules! u32_at {",
            "    ($offset:expr) => {{",
            "        let off = $offset;",
            "        u32::from_le_bytes([",
            "            BYTES[off],",
            "            BYTES[off + 1],",
            "            BYTES[off + 2],",
            "            BYTES[off + 3],",
            "        ])",
            "    }};",
            "}",
            "",
            "macro_rules! str_at {",
            "    ($offset:expr, $len:expr) => {{",
            "        let off: usize = $offset;",
            "        let len: usize = $len;",
            "        assert!(off + len <= STRINGS_END);",
            "        // SAFETY: `BYTES[..STRINGS_END]` is verified ASCII above and `off + len`",
            "        // is bounded within `BYTES`.",
            "        unsafe {",
            "            core::str::from_utf8_unchecked(core::slice::from_raw_parts(",
            "                BYTES.as_ptr().add(off),",
            "                len,",
            "            ))",
            "        }",
            "    }};",
            "}",
            "",
            "macro_rules! opt_str_at {",
            "    ($offset:expr, $len:expr) => {{",
            "        let len_u16: u16 = $len;",
            "        if len_u16 == u16::MAX {",
            "            None",
            "        } else {",
            "            Some(str_at!($offset, len_u16 as usize))",
            "        }",
            "    }};",
            "}",
            "",
            f"const BLOCK_ENTITY_TYPE_TABLE: [Option<i32>; {len(block_entity_variants) + 1}] = [",
            "    None,",
        )
    )
    for be_type in block_entity_variants:
        lines.append(
            f"    Some(crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_{be_type}_ID),"
        )
    lines.extend(
        (
            "];",
            "",
            f"const DYE_COLOR_TABLE: [Option<i32>; {len(dye_color_variants) + 1}] = [",
            "    None,",
        )
    )
    for color in dye_color_variants:
        lines.append(
            f"    Some(crate::builtin_enum_component_values::VANILLA_DYE_COLOR_{color}_ID),"
        )
    lines.extend(
        (
            "];",
            "",
        )
    )
    emit_enum_table(
        "ANALOG_OUTPUT_TABLE", "VanillaBlockAnalogOutputOverride", analog_output_variants
    )
    emit_enum_table(
        "REDSTONE_SIGNAL_TABLE", "VanillaBlockRedstoneSignalOverride", redstone_signal_variants
    )
    emit_enum_table(
        "USE_ITEM_ON_TABLE", "VanillaBlockUseItemOnOverride", use_item_on_variants
    )
    emit_enum_table(
        "USE_WITHOUT_ITEM_TABLE", "VanillaBlockUseWithoutItemOverride", use_without_item_variants
    )
    emit_enum_table("CAN_SURVIVE_TABLE", "VanillaCanSurviveOverride", can_survive_variants)
    emit_enum_table("RANDOM_TICK_TABLE", "VanillaRandomTickOverride", random_tick_variants)
    emit_enum_table("TICK_TABLE", "VanillaTickOverride", tick_variants)
    emit_enum_table(
        "NEIGHBOR_CHANGED_TABLE", "VanillaNeighborChangedOverride", neighbor_changed_variants
    )
    emit_enum_table("ON_PLACE_TABLE", "VanillaOnPlaceOverride", on_place_variants)
    emit_enum_table(
        "AFFECT_REMOVAL_TABLE",
        "VanillaAffectNeighborsAfterRemovalOverride",
        affect_neighbors_after_removal_variants,
    )
    emit_enum_table("MAY_PLACE_ON_TABLE", "VanillaMayPlaceOnOverride", may_place_on_variants)
    emit_enum_table(
        "CLONE_ITEM_STACK_TABLE", "VanillaCloneItemStackOverride", clone_item_stack_variants
    )
    emit_enum_table(
        "PLACEMENT_TABLE",
        "VanillaGetStateForPlacementOverride",
        get_state_for_placement_variants,
    )
    emit_enum_table("BUCKET_PICKUP_TABLE", "VanillaBucketPickupOverride", bucket_pickup_variants)
    emit_enum_table(
        "LIQUID_CONTAINER_TABLE",
        "VanillaLiquidBlockContainerOverride",
        liquid_container_variants,
    )
    emit_enum_table(
        "BONEMEAL_TARGET_TABLE", "VanillaBonemealTargetOverride", bonemeal_target_variants
    )
    emit_enum_table(
        "SKYLIGHT_DOWN_TABLE",
        "VanillaPropagatesSkylightDownOverride",
        skylight_down_variants,
    )
    emit_enum_table("VISUAL_SHAPE_TABLE", "VanillaVisualShapeOverride", visual_shape_variants)
    emit_enum_table(
        "MENU_PROVIDER_TABLE", "VanillaBlockMenuProviderOverride", menu_provider_variants
    )

    lines.extend(
        (
            "const fn decode_constructor_pairs() -> [(&'static str, &'static str); PAIR_COUNT] {",
            '    let mut out = [("", ""); PAIR_COUNT];',
            "    let mut idx = 0;",
            "    while idx < PAIR_COUNT {",
            "        let base = PAIRS_OFFSET + idx * 12;",
            "        let field = str_at!(u32_at!(base) as usize, u16_at!(base + 4) as usize);",
            "        let value = str_at!(u32_at!(base + 6) as usize, u16_at!(base + 10) as usize);",
            "        out[idx] = (field, value);",
            "        idx += 1;",
            "    }",
            "    out",
            "}",
            "",
            "static DECODED_CONSTRUCTOR_PAIRS: [(&str, &str); PAIR_COUNT] =",
            "    decode_constructor_pairs();",
            "",
            "macro_rules! slice_pairs {",
            "    ($start:expr, $len:expr) => {{",
            "        let start: usize = $start;",
            "        let len: usize = $len;",
            "        if len == 0 {",
            "            &[]",
            "        } else {",
            "            assert!(start + len <= PAIR_COUNT);",
            "            // SAFETY: `start + len <= PAIR_COUNT` bounds the slice within",
            "            // `DECODED_CONSTRUCTOR_PAIRS`.",
            "            unsafe {",
            "                core::slice::from_raw_parts(",
            "                    DECODED_CONSTRUCTOR_PAIRS.as_ptr().add(start),",
            "                    len,",
            "                )",
            "            }",
            "        }",
            "    }};",
            "}",
            "",
            "const EMPTY_BLOCK_ENTRY: BlockTypeCatalogEntry = BlockTypeCatalogEntry {",
            "    id: 0,",
            '    constant_name: "",',
            '    resource_id: "",',
            "    has_loot_table: false,",
            "    block_entity_type_id: None,",
            "    analog_output_override: None,",
            "    is_signal_source_override: None,",
            "    signal_override: None,",
            "    direct_signal_override: None,",
            "    use_item_on_override: None,",
            "    use_without_item_override: None,",
            "    can_survive_override: None,",
            "    random_tick_override: None,",
            "    tick_override: None,",
            "    neighbor_changed_override: None,",
            "    on_place_override: None,",
            "    affect_neighbors_after_removal_override: None,",
            "    may_place_on_override: None,",
            "    may_place_on_support_tag: None,",
            "    constructor_block_ids: &[],",
            "    constructor_block_tags: &[],",
            "    clone_item_stack_override: None,",
            "    get_state_for_placement_override: None,",
            "    bucket_pickup_override: None,",
            "    liquid_block_container_override: None,",
            "    growing_plant_head: false,",
            "    bonemeal_target_override: None,",
            "    propagates_skylight_down_override: None,",
            "    visual_shape_override: None,",
            "    menu_provider_override: None,",
            "    replaceable: false,",
            "    fire_ignite_odds: 0,",
            "    fire_burn_odds: 0,",
            "    beacon_beam_dye_color_id: None,",
            "    change_over_time_age: None,",
            "    change_over_time_next_block_type_id: None,",
            "};",
            "",
            "const fn decode_block_types() -> [BlockTypeCatalogEntry; BLOCK_COUNT] {",
            "    let mut out = [EMPTY_BLOCK_ENTRY; BLOCK_COUNT];",
            "    let mut idx = 0;",
            "    while idx < BLOCK_COUNT {",
            "        let base = RECORDS_OFFSET + idx * RECORD_STRIDE;",
            "        let flags = BYTES[base + 24];",
            "        let age_raw = BYTES[base + 48];",
            "        let next_raw = u16_at!(base + 49);",
            "        out[idx] = BlockTypeCatalogEntry {",
            "            id: idx as i32,",
            "            constant_name: str_at!(u32_at!(base) as usize, u16_at!(base + 4) as usize),",
            "            resource_id: str_at!(u32_at!(base + 6) as usize, u16_at!(base + 10) as usize),",
            "            has_loot_table: (flags & 1) != 0,",
            "            block_entity_type_id: BLOCK_ENTITY_TYPE_TABLE[BYTES[base + 25] as usize],",
            "            analog_output_override: ANALOG_OUTPUT_TABLE[BYTES[base + 26] as usize],",
            "            is_signal_source_override: REDSTONE_SIGNAL_TABLE[BYTES[base + 27] as usize],",
            "            signal_override: REDSTONE_SIGNAL_TABLE[BYTES[base + 28] as usize],",
            "            direct_signal_override: REDSTONE_SIGNAL_TABLE[BYTES[base + 29] as usize],",
            "            use_item_on_override: USE_ITEM_ON_TABLE[BYTES[base + 30] as usize],",
            "            use_without_item_override: USE_WITHOUT_ITEM_TABLE[BYTES[base + 31] as usize],",
            "            can_survive_override: CAN_SURVIVE_TABLE[BYTES[base + 32] as usize],",
            "            random_tick_override: RANDOM_TICK_TABLE[BYTES[base + 33] as usize],",
            "            tick_override: TICK_TABLE[BYTES[base + 34] as usize],",
            "            neighbor_changed_override: NEIGHBOR_CHANGED_TABLE[BYTES[base + 35] as usize],",
            "            on_place_override: ON_PLACE_TABLE[BYTES[base + 36] as usize],",
            "            affect_neighbors_after_removal_override: AFFECT_REMOVAL_TABLE[BYTES[base + 37] as usize],",
            "            may_place_on_override: MAY_PLACE_ON_TABLE[BYTES[base + 38] as usize],",
            "            may_place_on_support_tag: opt_str_at!(u32_at!(base + 12) as usize, u16_at!(base + 16)),",
            "            constructor_block_ids: slice_pairs!(u16_at!(base + 18) as usize, BYTES[base + 20] as usize),",
            "            constructor_block_tags: slice_pairs!(u16_at!(base + 21) as usize, BYTES[base + 23] as usize),",
            "            clone_item_stack_override: CLONE_ITEM_STACK_TABLE[BYTES[base + 39] as usize],",
            "            get_state_for_placement_override: PLACEMENT_TABLE[BYTES[base + 40] as usize],",
            "            bucket_pickup_override: BUCKET_PICKUP_TABLE[BYTES[base + 41] as usize],",
            "            liquid_block_container_override: LIQUID_CONTAINER_TABLE[BYTES[base + 42] as usize],",
            "            growing_plant_head: (flags & 2) != 0,",
            "            bonemeal_target_override: BONEMEAL_TARGET_TABLE[BYTES[base + 43] as usize],",
            "            propagates_skylight_down_override: SKYLIGHT_DOWN_TABLE[BYTES[base + 44] as usize],",
            "            visual_shape_override: VISUAL_SHAPE_TABLE[BYTES[base + 45] as usize],",
            "            menu_provider_override: MENU_PROVIDER_TABLE[BYTES[base + 46] as usize],",
            "            replaceable: (flags & 4) != 0,",
            "            fire_ignite_odds: u16_at!(base + 51) as i32,",
            "            fire_burn_odds: u16_at!(base + 53) as i32,",
            "            beacon_beam_dye_color_id: DYE_COLOR_TABLE[BYTES[base + 47] as usize],",
            "            change_over_time_age: if age_raw == u8::MAX { None } else { Some(age_raw as i32) },",
            "            change_over_time_next_block_type_id: if next_raw == u16::MAX {",
            "                None",
            "            } else {",
            "                Some(next_raw as i32)",
            "            },",
            "        };",
            "        idx += 1;",
            "    }",
            "    out",
            "}",
            "",
            "static DECODED_BLOCK_TYPES: [BlockTypeCatalogEntry; BLOCK_COUNT] = decode_block_types();",
            "pub const VANILLA_BLOCK_TYPES: &[BlockTypeCatalogEntry] = &DECODED_BLOCK_TYPES;",
            "",
            "/// `Block.getExplosionResistance()`; StairBlock reads its base block.",
            "pub const fn vanilla_block_explosion_resistance(block_type_id: i32) -> Option<f32> {",
            "    if block_type_id < 0 || block_type_id as usize >= BLOCK_COUNT {",
            "        return None;",
            "    }",
            "    let base = RECORDS_OFFSET + (block_type_id as usize) * RECORD_STRIDE;",
            "    Some(f32::from_bits(u32_at!(base + 55)))",
            "}",
            "",
            "pub fn vanilla_block_type(id: i32) -> Option<&'static BlockTypeCatalogEntry> {",
            "    VANILLA_BLOCK_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "#[inline]",
            "pub fn vanilla_block_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static BlockTypeCatalogEntry> {",
            "    crate::lookup_tables::block_type_for_resource_id(resource_id)",
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
            "/// `BeaconBeamBlock.getColor()` for one block resource id, i.e. the beam",
            "/// colour `BeaconBlockEntity.tick` folds into its beam sections.",
            "#[inline]",
            "pub fn vanilla_beacon_beam_dye_color_id_for_block_name(",
            "    resource_id: &str,",
            ") -> Option<i32> {",
            "    crate::lookup_tables::beacon_beam_dye_color_id_for_block_name(resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
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
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.beacon_beam_dye_color_id.is_some()).count(), {len(beacon_beam_colors)});",
            "        assert_eq!(vanilla_beacon_beam_dye_color_id_for_block_name(\"minecraft:stone\"), None);",
            "        assert_eq!(vanilla_beacon_beam_dye_color_id_for_block_name(\"minecraft:beacon\"), Some(crate::builtin_enum_component_values::VANILLA_DYE_COLOR_WHITE_ID));",
            "        assert_eq!(vanilla_beacon_beam_dye_color_id_for_block_name(\"minecraft:lime_stained_glass_pane\"), Some(crate::builtin_enum_component_values::VANILLA_DYE_COLOR_LIME_ID));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.change_over_time_age.is_some()).count(), {len(change_over_time_ages)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.change_over_time_next_block_type_id.is_some()).count(), {len(weathering_next_blocks)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().change_over_time_age, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().change_over_time_next_block_type_id, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_COPPER_BLOCK_ID).unwrap().change_over_time_age, Some(0));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_COPPER_BLOCK_ID).unwrap().change_over_time_next_block_type_id, Some(VANILLA_BLOCK_EXPOSED_COPPER_ID));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OXIDIZED_COPPER_ID).unwrap().change_over_time_age, Some(3));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OXIDIZED_COPPER_ID).unwrap().change_over_time_next_block_type_id, None);",
            "        // A waxed copy is a plain block, not a `WeatheringCopper`, so it has",
            "        // no age at all and is not a `ChangeOverTimeBlock` neighbour either.",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_WAXED_COPPER_BLOCK_ID).unwrap().change_over_time_age, None);",
            "        // The three families registered as a `WeatheringCopperBlocks` group",
            "        // reach the map through `weatheringMapping()` rather than a `put`.",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_COPPER_BARS_ID).unwrap().change_over_time_next_block_type_id, Some(VANILLA_BLOCK_EXPOSED_COPPER_BARS_ID));",
            "        // Every oxidation chain is four blocks long and its ages count up",
            "        // one at a time, which is what lets `getNextState` compare ordinals.",
            "        for entry in VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.change_over_time_age.is_some()) {",
            "            let start = entry.change_over_time_age.unwrap();",
            "            let mut steps = 0;",
            "            let mut walked = entry;",
            "            while let Some(next) = walked.change_over_time_next_block_type_id {",
            "                walked = vanilla_block_type(next).unwrap();",
            "                steps += 1;",
            "                assert_eq!(walked.change_over_time_age, Some(start + steps));",
            "            }",
            "            assert_eq!(start + steps, 3);",
            "        }",
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
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.random_tick_override.is_some()).count(), {sum(owner is not None for owner in random_tick_owners)});",
            "        let mut random_tick_classes: Vec<VanillaRandomTickOverride> = Vec::new();",
            "        for owner in VANILLA_BLOCK_TYPES.iter().filter_map(|entry| entry.random_tick_override) {",
            "            if !random_tick_classes.contains(&owner) { random_tick_classes.push(owner); }",
            "        }",
            f"        assert_eq!(random_tick_classes.len(), {len(random_tick_variants)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().random_tick_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_SUGAR_CANE_ID).unwrap().random_tick_override, Some(VanillaRandomTickOverride::SugarCaneBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CACTUS_ID).unwrap().random_tick_override, Some(VanillaRandomTickOverride::CactusBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OAK_SAPLING_ID).unwrap().random_tick_override, Some(VanillaRandomTickOverride::SaplingBlock));",
            "        macro_rules! distinct_owners {",
            "            ($field:ident) => {{",
            "                let mut owners = Vec::new();",
            "                for owner in VANILLA_BLOCK_TYPES.iter().filter_map(|entry| entry.$field) {",
            "                    if !owners.contains(&owner) { owners.push(owner); }",
            "                }",
            "                owners.len()",
            "            }};",
            "        }",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.tick_override.is_some()).count(), {sum(owner is not None for owner in tick_owners)});",
            f"        assert_eq!(distinct_owners!(tick_override), {len(tick_variants)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().tick_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REPEATER_ID).unwrap().tick_override, Some(VanillaTickOverride::DiodeBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_COMPARATOR_ID).unwrap().tick_override, Some(VanillaTickOverride::ComparatorBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OBSERVER_ID).unwrap().tick_override, Some(VanillaTickOverride::ObserverBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REDSTONE_TORCH_ID).unwrap().tick_override, Some(VanillaTickOverride::RedstoneTorchBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_BUTTON_ID).unwrap().tick_override, Some(VanillaTickOverride::ButtonBlock));",
            "        // `randomTick` and `tick` are separate members: a sapling grows only",
            "        // on a random tick and schedules nothing, while sugar cane owns both.",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OAK_SAPLING_ID).unwrap().tick_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_SUGAR_CANE_ID).unwrap().tick_override, Some(VanillaTickOverride::SugarCaneBlock));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.neighbor_changed_override.is_some()).count(), {sum(owner is not None for owner in neighbor_changed_owners)});",
            f"        assert_eq!(distinct_owners!(neighbor_changed_override), {len(neighbor_changed_variants)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().neighbor_changed_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REPEATER_ID).unwrap().neighbor_changed_override, Some(VanillaNeighborChangedOverride::DiodeBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REDSTONE_WIRE_ID).unwrap().neighbor_changed_override, Some(VanillaNeighborChangedOverride::RedStoneWireBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REDSTONE_LAMP_ID).unwrap().neighbor_changed_override, Some(VanillaNeighborChangedOverride::RedstoneLampBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_HOPPER_ID).unwrap().neighbor_changed_override, Some(VanillaNeighborChangedOverride::HopperBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OAK_DOOR_ID).unwrap().neighbor_changed_override, Some(VanillaNeighborChangedOverride::DoorBlock));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.on_place_override.is_some()).count(), {sum(owner is not None for owner in on_place_owners)});",
            f"        assert_eq!(distinct_owners!(on_place_override), {len(on_place_variants)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().on_place_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REDSTONE_TORCH_ID).unwrap().on_place_override, Some(VanillaOnPlaceOverride::RedstoneTorchBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OBSERVER_ID).unwrap().on_place_override, Some(VanillaOnPlaceOverride::ObserverBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_HOPPER_ID).unwrap().on_place_override, Some(VanillaOnPlaceOverride::HopperBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_TNT_ID).unwrap().on_place_override, Some(VanillaOnPlaceOverride::TntBlock));",
            "        // 26.1 spells three of these overrides `public` instead of",
            "        // `protected`; the extractor matches the signature, so they stay in.",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_FROSTED_ICE_ID).unwrap().on_place_override, Some(VanillaOnPlaceOverride::FrostedIceBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_SNIFFER_EGG_ID).unwrap().on_place_override, Some(VanillaOnPlaceOverride::SnifferEggBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_SUSPICIOUS_SAND_ID).unwrap().tick_override, Some(VanillaTickOverride::BrushableBlock));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.affect_neighbors_after_removal_override.is_some()).count(), {sum(owner is not None for owner in affect_neighbors_after_removal_owners)});",
            f"        assert_eq!(distinct_owners!(affect_neighbors_after_removal_override), {len(affect_neighbors_after_removal_variants)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().affect_neighbors_after_removal_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_LEVER_ID).unwrap().affect_neighbors_after_removal_override, Some(VanillaAffectNeighborsAfterRemovalOverride::LeverBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_BUTTON_ID).unwrap().affect_neighbors_after_removal_override, Some(VanillaAffectNeighborsAfterRemovalOverride::ButtonBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_REDSTONE_WIRE_ID).unwrap().affect_neighbors_after_removal_override, Some(VanillaAffectNeighborsAfterRemovalOverride::RedStoneWireBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_OBSERVER_ID).unwrap().affect_neighbors_after_removal_override, Some(VanillaAffectNeighborsAfterRemovalOverride::ObserverBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_DISPENSER_ID).unwrap().affect_neighbors_after_removal_override, Some(VanillaAffectNeighborsAfterRemovalOverride::DispenserBlock));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.may_place_on_override.is_some()).count(), {sum(owner is not None for owner in may_place_on_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().may_place_on_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_POPPY_ID).unwrap().may_place_on_override, Some(VanillaMayPlaceOnOverride::VegetationBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_LILY_PAD_ID).unwrap().may_place_on_override, Some(VanillaMayPlaceOnOverride::LilyPadBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_WHEAT_ID).unwrap().may_place_on_override, Some(VanillaMayPlaceOnOverride::CropBlock));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.may_place_on_support_tag.is_some()).count(), {sum(tag is not None for tag in may_place_on_support_tags)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_POPPY_ID).unwrap().may_place_on_support_tag, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_PUMPKIN_STEM_ID).unwrap().may_place_on_support_tag, Some(\"minecraft:supports_pumpkin_stem\"));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CRIMSON_ROOTS_ID).unwrap().may_place_on_support_tag, Some(\"minecraft:supports_crimson_roots\"));",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| !entry.constructor_block_ids.is_empty()).count(), {sum(1 for pairs in constructor_block_ids if pairs)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| !entry.constructor_block_tags.is_empty()).count(), {sum(1 for pairs in constructor_block_tags if pairs)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().constructor_block_ids, &[]);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_PUMPKIN_STEM_ID).unwrap().constructor_block_ids, &[(\"attachedStem\", \"minecraft:attached_pumpkin_stem\"), (\"fruit\", \"minecraft:pumpkin\")]);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_PUMPKIN_STEM_ID).unwrap().constructor_block_tags, &[(\"fruitSupportBlocks\", \"minecraft:supports_pumpkin_stem_fruit\"), (\"stemSupportBlocks\", \"minecraft:supports_pumpkin_stem\")]);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_ATTACHED_MELON_STEM_ID).unwrap().constructor_block_ids, &[(\"fruit\", \"minecraft:melon\"), (\"stem\", \"minecraft:melon_stem\")]);",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.clone_item_stack_override.is_some()).count(), {sum(owner is not None for owner in clone_item_stack_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.get_state_for_placement_override.is_some()).count(), {sum(owner is not None for owner in get_state_for_placement_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.bucket_pickup_override.is_some()).count(), {sum(owner is not None for owner in bucket_pickup_owners)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.liquid_block_container_override.is_some()).count(), {sum(owner is not None for owner in liquid_container_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().use_item_on_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().use_without_item_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().can_survive_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().clone_item_stack_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_WHEAT_ID).unwrap().clone_item_stack_override, Some(VanillaCloneItemStackOverride::CropBlock));",
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
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.visual_shape_override.is_some()).count(), {sum(owner is not None for owner in visual_shape_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().visual_shape_override, None);",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.menu_provider_override.is_some()).count(), {sum(owner is not None for owner in menu_provider_owners)});",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().menu_provider_override, None);",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CHEST_ID).unwrap().menu_provider_override, Some(VanillaBlockMenuProviderOverride::ChestBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_LECTERN_ID).unwrap().menu_provider_override, Some(VanillaBlockMenuProviderOverride::LecternBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_FURNACE_ID).unwrap().menu_provider_override, Some(VanillaBlockMenuProviderOverride::BaseEntityBlock));",
            "        assert_eq!(vanilla_block_type(VANILLA_BLOCK_CRAFTING_TABLE_ID).unwrap().menu_provider_override, Some(VanillaBlockMenuProviderOverride::CraftingTableBlock));",
            "        assert!(vanilla_block_type(VANILLA_BLOCK_AIR_ID).unwrap().replaceable);",
            "        assert!(!vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().replaceable);",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.replaceable).count(), {sum(replaceable_by_field.values())});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.fire_ignite_odds > 0).count(), {len(fire_block_odds)});",
            f"        assert_eq!(VANILLA_BLOCK_TYPES.iter().filter(|entry| entry.fire_burn_odds > 0).count(), {len(fire_block_odds)});",
            "        assert!(vanilla_block_explosion_resistance((VANILLA_BLOCK_TYPES.len() - 1) as i32).is_some());",
            "        assert_eq!(vanilla_block_explosion_resistance(VANILLA_BLOCK_STONE_ID), Some(6.0));",
            "        assert_eq!(vanilla_block_explosion_resistance(VANILLA_BLOCK_HEAVY_CORE_ID), Some(1200.0));",
            "        assert_eq!(vanilla_block_explosion_resistance(-1), None);",
            "        assert_eq!(vanilla_block_explosion_resistance(VANILLA_BLOCK_TYPES.len() as i32), None);",
            "        assert_eq!((vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().fire_ignite_odds, vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap().fire_burn_odds), (0, 0));",
            "        assert_eq!((vanilla_block_type(VANILLA_BLOCK_OAK_PLANKS_ID).unwrap().fire_ignite_odds, vanilla_block_type(VANILLA_BLOCK_OAK_PLANKS_ID).unwrap().fire_burn_odds), (5, 20));",
            "        assert_eq!((vanilla_block_type(VANILLA_BLOCK_OAK_LEAVES_ID).unwrap().fire_ignite_odds, vanilla_block_type(VANILLA_BLOCK_OAK_LEAVES_ID).unwrap().fire_burn_odds), (30, 60));",
            "        assert_eq!((vanilla_block_type(VANILLA_BLOCK_TNT_ID).unwrap().fire_ignite_odds, vanilla_block_type(VANILLA_BLOCK_TNT_ID).unwrap().fire_burn_odds), (15, 100));",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines), blob


def assert_block_entity_column_is_entity_block_membership(
    declarations, entity_block_by_factory: dict[str, bool]
) -> None:
    """Pin the `block_entity_type` column to `BlockState.hasBlockEntity()`.

    `BlockBehaviour.BlockStateBase.hasBlockEntity` is
    `getBlock() instanceof EntityBlock`, while the column this catalog emits is
    built from the `BlockEntityType.register(name, factory, blocks...)` calls.
    Consumers read the column as the answer to both questions -- "does this
    state carry a block entity" and "which type" -- which is only sound while
    the two sets are the same one. In 26.1 they are; this fails the generator
    the moment a release makes a block an `EntityBlock` without registering it,
    or registers a block whose class is not one.
    """

    entity_blocks = {
        declaration["field"]
        for declaration in declarations
        if entity_block_by_factory[block_factory_class(declaration)]
    }
    registered = {
        declaration["field"]
        for declaration in declarations
        if declaration["block_entity_type"] is not None
    }
    if entity_blocks != registered:
        raise SystemExit(
            "block_entity_type no longer equals EntityBlock membership; "
            f"EntityBlock only: {sorted(entity_blocks - registered)}; "
            f"registered only: {sorted(registered - entity_blocks)}"
        )


def check_output(path: Path, generated: str) -> bool:
    current = path.read_text(encoding="utf-8") if path.exists() else ""
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


def check_binary_output(path: Path, generated: bytes) -> bool:
    current = path.read_bytes() if path.exists() else b""
    if current == generated:
        return True
    sys.stderr.write(
        f"binary output differs: {path} (expected {len(generated)} bytes, got {len(current)})\n"
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
    random_tick_owner_by_factory = extract_block_random_tick_override_owners(
        catalog.declarations, block_class_sources
    )
    tick_owner_by_factory = extract_block_tick_override_owners(
        catalog.declarations, block_class_sources
    )
    neighbor_changed_owner_by_factory = (
        extract_block_neighbor_changed_override_owners(
            catalog.declarations, block_class_sources
        )
    )
    on_place_owner_by_factory = extract_block_on_place_override_owners(
        catalog.declarations, block_class_sources
    )
    affect_neighbors_after_removal_owner_by_factory = (
        extract_block_affect_neighbors_after_removal_override_owners(
            catalog.declarations, block_class_sources
        )
    )
    may_place_on_owner_by_factory = extract_block_may_place_on_override_owners(
        catalog.declarations, block_class_sources
    )
    constructor_block_id_by_field = extract_block_constructor_block_ids(
        catalog.declarations,
        block_class_sources,
        parse_source(
            BLOCK_IDS_PATH.as_posix(),
            (args.source_root / BLOCK_IDS_PATH).read_bytes(),
        ),
    )
    constructor_block_tag_by_field = extract_block_constructor_block_tags(
        catalog.declarations,
        block_class_sources,
        parse_source(
            BLOCK_TAGS_PATH.as_posix(),
            (args.source_root / BLOCK_TAGS_PATH).read_bytes(),
        ),
    )
    may_place_on_support_tag_by_field = extract_block_may_place_on_support_tags(
        catalog.declarations,
        block_class_sources,
        parse_source(
            BLOCK_TAGS_PATH.as_posix(),
            (args.source_root / BLOCK_TAGS_PATH).read_bytes(),
        ),
    )
    clone_item_stack_owner_by_factory = extract_block_clone_item_stack_override_owners(
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
    assert_block_entity_column_is_entity_block_membership(
        catalog.declarations,
        extract_block_interface_membership(
            catalog.declarations,
            block_class_sources,
            "net.minecraft.world.level.block.EntityBlock",
        ),
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
    visual_shape_owner_by_factory = extract_block_visual_shape_override_owners(
        catalog.declarations,
        block_class_sources,
    )
    menu_provider_owner_by_factory = extract_block_menu_provider_owners(
        catalog.declarations,
        block_class_sources,
    )
    replaceable_by_field = extract_block_replaceable_membership(
        catalog.declarations
    )
    concrete_powder_targets = extract_concrete_powder_targets(
        catalog.declarations
    )
    beacon_beam_colors = extract_beacon_beam_colors(
        catalog.declarations,
        block_class_sources,
    )
    change_over_time_ages = extract_change_over_time_ages(
        catalog.declarations,
        block_class_sources,
    )
    weathering_next_blocks = extract_weathering_next_blocks(
        catalog.declarations,
        block_class_sources,
    )
    fire_block_odds = extract_fire_block_odds(
        catalog.declarations,
        block_class_sources,
    )
    explosion_resistances = extract_block_explosion_resistances(
        catalog.declarations,
        parse_source(BLOCKS_PATH.as_posix(), (args.source_root / BLOCKS_PATH).read_bytes()),
        parse_source(PROPERTIES_PATH.as_posix(), (args.source_root / PROPERTIES_PATH).read_bytes()),
        block_class_sources,
    )
    has_loot_table_by_field = extract_block_loot_table_membership(
        catalog.declarations,
        parse_source(BLOCKS_PATH.as_posix(), (args.source_root / BLOCKS_PATH).read_bytes()),
    )
    protocol_rust, protocol_blob = render_protocol(
        catalog,
        analog_output_owner_by_factory,
        is_signal_source_owner_by_factory,
        signal_owner_by_factory,
        direct_signal_owner_by_factory,
        use_item_on_owner_by_factory,
        use_without_item_owner_by_factory,
        can_survive_owner_by_factory,
        random_tick_owner_by_factory,
        tick_owner_by_factory,
        neighbor_changed_owner_by_factory,
        on_place_owner_by_factory,
        affect_neighbors_after_removal_owner_by_factory,
        may_place_on_owner_by_factory,
        may_place_on_support_tag_by_field,
        constructor_block_id_by_field,
        constructor_block_tag_by_field,
        clone_item_stack_owner_by_factory,
        get_state_for_placement_owner_by_factory,
        bucket_pickup_owner_by_factory,
        liquid_container_owner_by_factory,
        growing_plant_head_by_factory,
        game_master_block_by_factory,
        bonemeal_target_owner_by_factory,
        skylight_down_owner_by_factory,
        visual_shape_owner_by_factory,
        menu_provider_owner_by_factory,
        replaceable_by_field,
        concrete_powder_targets,
        beacon_beam_colors,
        change_over_time_ages,
        weathering_next_blocks,
        fire_block_odds,
        explosion_resistances,
        has_loot_table_by_field,
    )
    outputs = (
        (args.output, render_json(catalog)),
        (args.protocol_output, format_rust(protocol_rust)),
    )
    if args.check:
        checks = [check_output(path, generated) for path, generated in outputs]
        checks.append(check_binary_output(args.protocol_bin_output, protocol_blob))
        return 0 if all(checks) else 1
    for path, generated in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(generated, encoding="utf-8")
    args.protocol_bin_output.parent.mkdir(parents=True, exist_ok=True)
    args.protocol_bin_output.write_bytes(protocol_blob)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
