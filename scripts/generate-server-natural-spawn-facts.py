#!/usr/bin/env python3
"""Generate/check server-owned Vanilla 26.1 natural-spawn source facts."""

from __future__ import annotations

import argparse
import difflib
import json
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
from core_registry_catalog_extractor import entity_type_entries, vec3_constant
from entity_metadata_extractor import (
    ExtractionResult,
    class_nearest_method_owner,
    extract,
)
from vanilla_java_source import JavaExpression, JavaSource, type_body_members

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-server-core/src/entity/natural_spawn_facts_26_1.rs"
ENTITY_TYPE_SOURCE = "net/minecraft/world/entity/EntityType.java"
AVATAR_SOURCE = "net/minecraft/world/entity/Avatar.java"
MOB_CATEGORY_SOURCE = "net/minecraft/world/entity/MobCategory.java"
SPAWN_PLACEMENTS_SOURCE = "net/minecraft/world/entity/SpawnPlacements.java"
MOB = "net.minecraft.world.entity.Mob"
PATHFINDER_MOB = "net.minecraft.world.entity.PathfinderMob"
EXPECTED_CATEGORIES = 8
EXPECTED_ENTITY_TYPES = 157
EXPECTED_MOBS = 89
EXPECTED_PLACEMENTS = 82

CATEGORY_VARIANTS = {
    "MONSTER": "Monster",
    "CREATURE": "Creature",
    "AMBIENT": "Ambient",
    "AXOLOTLS": "Axolotls",
    "UNDERGROUND_WATER_CREATURE": "UndergroundWaterCreature",
    "WATER_CREATURE": "WaterCreature",
    "WATER_AMBIENT": "WaterAmbient",
    "MISC": "Misc",
}
PLACEMENT_VARIANTS = {
    "SpawnPlacementTypes.NO_RESTRICTIONS": "NoRestrictions",
    "SpawnPlacementTypes.IN_WATER": "InWater",
    "SpawnPlacementTypes.IN_LAVA": "InLava",
    "SpawnPlacementTypes.ON_GROUND": "OnGround",
}
HEIGHTMAP_VARIANTS = {
    "Heightmap.Types.MOTION_BLOCKING": "MotionBlocking",
    "Heightmap.Types.MOTION_BLOCKING_NO_LEAVES": "MotionBlockingNoLeaves",
}
PREDICATE_VARIANTS = {
    (
        "net.minecraft.world.entity.animal.nautilus.AbstractNautilus",
        "checkNautilusSpawnRules",
    ): "Nautilus",
    (
        "net.minecraft.world.entity.animal.AgeableWaterCreature",
        "checkSurfaceAgeableWaterCreatureSpawnRules",
    ): "SurfaceAgeableWaterCreature",
    ("net.minecraft.world.entity.animal.Animal", "checkAnimalSpawnRules"): "Animal",
    (
        "net.minecraft.world.entity.animal.armadillo.Armadillo",
        "checkArmadilloSpawnRules",
    ): "Armadillo",
    ("net.minecraft.world.entity.animal.axolotl.Axolotl", "checkAxolotlSpawnRules"): "Axolotl",
    ("net.minecraft.world.entity.ambient.Bat", "checkBatSpawnRules"): "Bat",
    ("net.minecraft.world.entity.animal.camel.Camel", "checkCamelSpawnRules"): "Camel",
    ("net.minecraft.world.entity.monster.zombie.Drowned", "checkDrownedSpawnRules"): "Drowned",
    ("net.minecraft.world.entity.monster.Endermite", "checkEndermiteSpawnRules"): "Endermite",
    ("net.minecraft.world.entity.animal.fox.Fox", "checkFoxSpawnRules"): "Fox",
    ("net.minecraft.world.entity.animal.frog.Frog", "checkFrogSpawnRules"): "Frog",
    ("net.minecraft.world.entity.monster.Ghast", "checkGhastSpawnRules"): "Ghast",
    ("net.minecraft.world.entity.animal.squid.GlowSquid", "checkGlowSquidSpawnRules"): "GlowSquid",
    ("net.minecraft.world.entity.animal.goat.Goat", "checkGoatSpawnRules"): "Goat",
    ("net.minecraft.world.entity.monster.Guardian", "checkGuardianSpawnRules"): "Guardian",
    ("net.minecraft.world.entity.monster.hoglin.Hoglin", "checkHoglinSpawnRules"): "Hoglin",
    ("net.minecraft.world.entity.monster.MagmaCube", "checkMagmaCubeSpawnRules"): "MagmaCube",
    ("net.minecraft.world.entity.Mob", "checkMobSpawnRules"): "Mob",
    (
        "net.minecraft.world.entity.monster.Monster",
        "checkAnyLightMonsterSpawnRules",
    ): "AnyLightMonster",
    ("net.minecraft.world.entity.monster.Monster", "checkMonsterSpawnRules"): "Monster",
    (
        "net.minecraft.world.entity.monster.Monster",
        "checkSurfaceMonstersSpawnRules",
    ): "SurfaceMonster",
    ("net.minecraft.world.entity.animal.cow.MushroomCow", "checkMushroomSpawnRules"): "MushroomCow",
    ("net.minecraft.world.entity.animal.feline.Ocelot", "checkOcelotSpawnRules"): "Ocelot",
    ("net.minecraft.world.entity.animal.parrot.Parrot", "checkParrotSpawnRules"): "Parrot",
    (
        "net.minecraft.world.entity.monster.PatrollingMonster",
        "checkPatrollingMonsterSpawnRules",
    ): "PatrollingMonster",
    ("net.minecraft.world.entity.monster.piglin.Piglin", "checkPiglinSpawnRules"): "Piglin",
    (
        "net.minecraft.world.entity.animal.polarbear.PolarBear",
        "checkPolarBearSpawnRules",
    ): "PolarBear",
    ("net.minecraft.world.entity.animal.rabbit.Rabbit", "checkRabbitSpawnRules"): "Rabbit",
    ("net.minecraft.world.entity.monster.Silverfish", "checkSilverfishSpawnRules"): "Silverfish",
    (
        "net.minecraft.world.entity.animal.equine.SkeletonHorse",
        "checkSkeletonHorseSpawnRules",
    ): "SkeletonHorse",
    ("net.minecraft.world.entity.monster.Slime", "checkSlimeSpawnRules"): "Slime",
    ("net.minecraft.world.entity.monster.skeleton.Stray", "checkStraySpawnRules"): "Stray",
    ("net.minecraft.world.entity.monster.Strider", "checkStriderSpawnRules"): "Strider",
    (
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "checkTropicalFishSpawnRules",
    ): "TropicalFish",
    ("net.minecraft.world.entity.animal.turtle.Turtle", "checkTurtleSpawnRules"): "Turtle",
    (
        "net.minecraft.world.entity.animal.fish.WaterAnimal",
        "checkSurfaceWaterAnimalSpawnRules",
    ): "SurfaceWaterAnimal",
    ("net.minecraft.world.entity.animal.wolf.Wolf", "checkWolfSpawnRules"): "Wolf",
    (
        "net.minecraft.world.entity.monster.zombie.ZombifiedPiglin",
        "checkZombifiedPiglinSpawnRules",
    ): "ZombifiedPiglin",
}


class NaturalSpawnExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class MobCategoryFact:
    constant_name: str
    variant: str
    serialized_name: str
    max_instances_per_chunk: int
    friendly: bool
    persistent: bool
    no_despawn_distance: int
    despawn_distance: int


@dataclass(frozen=True)
class PlacementFact:
    entity_constant: str
    placement: str
    heightmap: str
    predicate_kind: str
    predicate_owner: str
    predicate_method: str


@dataclass(frozen=True)
class NaturalSpawnTypeFact:
    entity_type_id: int
    constant_name: str
    resource_id: str
    runtime_class: str | None
    is_mob: bool
    category_variant: str
    placement: str
    heightmap: str
    predicate_kind: str
    predicate_owner: str | None
    predicate_method: str | None
    check_spawn_obstruction_owner: str | None
    walk_target_value_owner: str | None


@dataclass(frozen=True)
class NaturalSpawnCatalog:
    categories: tuple[MobCategoryFact, ...]
    entity_types: tuple[NaturalSpawnTypeFact, ...]


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    args.source_root = resolve_source_root(args)
    return args


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    raise NaturalSpawnExtractionError(
        f"{source.relative_path}:{expression.span.start_point[0] + 1}: {message}"
    )


def _integer(source: JavaSource, expression: JavaExpression, description: str) -> int:
    if expression.node.type == "decimal_integer_literal":
        spelling = expression.canonical.replace("_", "").rstrip("lL")
        try:
            return int(spelling, 10)
        except ValueError:
            pass
    if expression.node.type == "unary_expression":
        children = expression.node.children
        if len(children) == 2 and children[0].type == "-" and children[1].is_named:
            child = source.expression(children[1])
            return -_integer(source, child, description)
    _fail(source, expression, f"{description} is not a decimal integer literal")


def _boolean(source: JavaSource, expression: JavaExpression, description: str) -> bool:
    if expression.node.type == "true":
        return True
    if expression.node.type == "false":
        return False
    _fail(source, expression, f"{description} is not a boolean literal")


def _string(source: JavaSource, expression: JavaExpression, description: str) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, f"{description} is not a string literal")
    try:
        value = json.loads(expression.canonical)
    except json.JSONDecodeError as error:
        _fail(source, expression, f"{description} is not a plain string literal: {error}")
    if not isinstance(value, str):
        _fail(source, expression, f"{description} is not a string")
    return value


def mob_category_facts(
    source: JavaSource, expected_count: int = EXPECTED_CATEGORIES
) -> tuple[MobCategoryFact, ...]:
    categories = [item for item in source.types if item.simple_name == "MobCategory"]
    if len(categories) != 1 or categories[0].kind != "enum":
        raise NaturalSpawnExtractionError(f"{source.relative_path}: expected one MobCategory enum")
    distance_fields = [
        field for field in source.fields_for(categories[0]) if field.name == "noDespawnDistance"
    ]
    if len(distance_fields) != 1 or distance_fields[0].initializer is None:
        raise NaturalSpawnExtractionError(
            f"{source.relative_path}: expected one initialized noDespawnDistance field"
        )
    no_despawn_distance = _integer(source, distance_fields[0].initializer, "noDespawnDistance")
    rows: list[MobCategoryFact] = []
    for node in type_body_members(categories[0].body_node):
        if node.type != "enum_constant":
            continue
        name_node = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name_node is None or arguments is None or len(arguments.named_children) != 5:
            _fail(source, source.expression(node), "mob category constant needs five arguments")
        constant_name = source.text(name_node)
        variant = CATEGORY_VARIANTS.get(constant_name)
        if variant is None:
            _fail(source, source.expression(name_node), f"unknown mob category {constant_name}")
        values = [source.expression(child) for child in arguments.named_children]
        rows.append(
            MobCategoryFact(
                constant_name,
                variant,
                _string(source, values[0], "serialized name"),
                _integer(source, values[1], "maximum instances per chunk"),
                _boolean(source, values[2], "friendly flag"),
                _boolean(source, values[3], "persistent flag"),
                no_despawn_distance,
                _integer(source, values[4], "despawn distance"),
            )
        )
    if len(rows) != expected_count:
        raise NaturalSpawnExtractionError(
            f"{source.relative_path}: expected {expected_count} mob categories, found {len(rows)}"
        )
    if {row.constant_name for row in rows} != set(CATEGORY_VARIANTS):
        raise NaturalSpawnExtractionError(
            f"{source.relative_path}: mob category constant set drifted"
        )
    if len({row.serialized_name for row in rows}) != len(rows):
        raise NaturalSpawnExtractionError(f"{source.relative_path}: duplicate category name")
    return tuple(rows)


def _constant_reference(
    source: JavaSource,
    expression: JavaExpression,
    qualifier: str,
    description: str,
) -> str:
    if expression.node.type != "field_access":
        _fail(source, expression, f"{description} is not a qualified constant")
    object_node = expression.node.child_by_field_name("object")
    field_node = expression.node.child_by_field_name("field")
    if object_node is None or field_node is None:
        _fail(source, expression, f"{description} is incomplete")
    if source.expression(object_node).canonical != qualifier:
        _fail(source, expression, f"{description} has unknown qualifier")
    return source.text(field_node)


def _resolve_method_reference(source: JavaSource, expression: JavaExpression) -> tuple[str, str]:
    if expression.node.type != "method_reference":
        _fail(source, expression, "spawn predicate is not a method reference")
    children = expression.node.children
    named = expression.node.named_children
    if (
        len(children) != 3
        or len(named) != 2
        or children[1].type != "::"
        or named[0].type not in {"identifier", "scoped_identifier"}
        or named[1].type != "identifier"
    ):
        _fail(source, expression, "spawn predicate method reference is incomplete")
    owner_spelling = source.text(named[0])
    method = source.text(named[1])
    if named[0].type == "scoped_identifier":
        owner = owner_spelling
    elif owner_spelling in source.imports:
        owner = source.imports[owner_spelling]
    else:
        owner = f"{source.package}.{owner_spelling}"
    return owner, method


def spawn_placement_facts(
    source: JavaSource,
    entity_constants: set[str],
    expected_count: int = EXPECTED_PLACEMENTS,
) -> dict[str, PlacementFact]:
    rows: dict[str, PlacementFact] = {}
    calls = [
        call
        for call in source.calls
        if call.owner == "net.minecraft.world.entity.SpawnPlacements" and call.name == "register"
    ]
    for call in calls:
        expression = source.expression(call.node)
        if len(call.arguments) != 4:
            _fail(source, expression, "spawn placement register needs four arguments")
        entity_constant = _constant_reference(
            source, call.arguments[0], "EntityType", "entity type"
        )
        if entity_constant not in entity_constants:
            _fail(source, call.arguments[0], f"unknown entity type {entity_constant}")
        if entity_constant in rows:
            _fail(source, call.arguments[0], f"duplicate placement for {entity_constant}")
        placement = PLACEMENT_VARIANTS.get(call.arguments[1].canonical)
        if placement is None:
            _fail(source, call.arguments[1], "unknown spawn placement constant")
        heightmap = HEIGHTMAP_VARIANTS.get(call.arguments[2].canonical)
        if heightmap is None:
            _fail(source, call.arguments[2], "unknown spawn heightmap constant")
        predicate_owner, predicate_method = _resolve_method_reference(source, call.arguments[3])
        predicate_kind = PREDICATE_VARIANTS.get((predicate_owner, predicate_method))
        if predicate_kind is None:
            _fail(
                source,
                call.arguments[3],
                f"unknown spawn predicate {predicate_owner}::{predicate_method}",
            )
        rows[entity_constant] = PlacementFact(
            entity_constant,
            placement,
            heightmap,
            predicate_kind,
            predicate_owner,
            predicate_method,
        )
    if len(rows) != expected_count:
        raise NaturalSpawnExtractionError(
            f"{source.relative_path}: expected {expected_count} placements, found {len(rows)}"
        )
    return rows


def _extends(class_map: dict, owner: str, base: str) -> bool:
    seen: set[str] = set()
    current: str | None = owner
    while current is not None:
        if current == base:
            return True
        if current in seen:
            raise NaturalSpawnExtractionError(f"class hierarchy cycle at {current}")
        seen.add(current)
        java_type = class_map.get(current)
        if java_type is None:
            raise NaturalSpawnExtractionError(f"unknown runtime class {current}")
        current = java_type.superclass
    return False


def _validate_predicate_declarations(
    result: ExtractionResult, placements: dict[str, PlacementFact]
) -> None:
    for placement in placements.values():
        source = result.sources.get(
            next(
                (
                    item.source_path
                    for item in result.classes
                    if item.fqcn == placement.predicate_owner
                ),
                "",
            )
        )
        if source is None:
            raise NaturalSpawnExtractionError(
                f"unknown spawn predicate owner {placement.predicate_owner}"
            )
        declarations = [
            method
            for method in source.methods_for(placement.predicate_owner)
            if method.name == placement.predicate_method and len(method.parameters) == 5
        ]
        if len(declarations) != 1:
            raise NaturalSpawnExtractionError(
                f"{placement.predicate_owner}.{placement.predicate_method}/5: "
                f"expected one declaration, found {len(declarations)}"
            )


def build_catalog(result: ExtractionResult) -> NaturalSpawnCatalog:
    categories = mob_category_facts(result.sources[MOB_CATEGORY_SOURCE])
    registry_rows = entity_type_entries(
        result.sources[ENTITY_TYPE_SOURCE],
        expected_count=EXPECTED_ENTITY_TYPES,
        named_vec3_constants={
            "Avatar.DEFAULT_VEHICLE_ATTACHMENT": vec3_constant(
                result.sources[AVATAR_SOURCE], "DEFAULT_VEHICLE_ATTACHMENT"
            )
        },
    )
    metadata_by_constant = {row.constant_name: row for row in result.entity_types}
    if len(metadata_by_constant) != EXPECTED_ENTITY_TYPES:
        raise NaturalSpawnExtractionError("entity metadata constants are not unique and total")
    if set(metadata_by_constant) != {row.constant_name for row in registry_rows}:
        raise NaturalSpawnExtractionError("entity registry/metadata constant join is not exact")
    placements = spawn_placement_facts(
        result.sources[SPAWN_PLACEMENTS_SOURCE], set(metadata_by_constant)
    )
    _validate_predicate_declarations(result, placements)
    class_map = {item.fqcn: item for item in result.classes}
    obstruction_cache: dict[str, str] = {}
    walk_target_value_cache: dict[str, str] = {}
    rows: list[NaturalSpawnTypeFact] = []
    for registry in registry_rows:
        metadata = metadata_by_constant[registry.constant_name]
        if metadata.resource_id != registry.resource_id:
            raise NaturalSpawnExtractionError(
                f"{registry.constant_name}: registry/metadata resource id mismatch"
            )
        if metadata.entity_type_id != len(rows):
            raise NaturalSpawnExtractionError(
                f"{registry.constant_name}: expected id {len(rows)}, "
                f"found {metadata.entity_type_id}"
            )
        category_variant = CATEGORY_VARIANTS.get(registry.mob_category)
        if category_variant is None:
            raise NaturalSpawnExtractionError(
                f"{registry.constant_name}: unknown category {registry.mob_category}"
            )
        placement = placements.get(registry.constant_name)
        runtime_class = metadata.runtime_class
        is_mob = runtime_class is not None and _extends(class_map, runtime_class, MOB)
        obstruction_owner = None
        walk_target_value_owner = None
        if is_mob:
            obstruction_owner = class_nearest_method_owner(
                result, class_map, runtime_class, "checkSpawnObstruction", 1, obstruction_cache
            )
            if _extends(class_map, runtime_class, PATHFINDER_MOB):
                walk_target_value_owner = class_nearest_method_owner(
                    result,
                    class_map,
                    runtime_class,
                    "getWalkTargetValue",
                    2,
                    walk_target_value_cache,
                )
        rows.append(
            NaturalSpawnTypeFact(
                metadata.entity_type_id,
                registry.constant_name,
                registry.resource_id,
                runtime_class,
                is_mob,
                category_variant,
                placement.placement if placement else "NoRestrictions",
                placement.heightmap if placement else "MotionBlockingNoLeaves",
                placement.predicate_kind if placement else "Unrestricted",
                placement.predicate_owner if placement else None,
                placement.predicate_method if placement else None,
                check_spawn_obstruction_owner=obstruction_owner,
                walk_target_value_owner=walk_target_value_owner,
            )
        )
    validate_catalog(categories, rows)
    return NaturalSpawnCatalog(categories, tuple(rows))


def validate_catalog(
    categories: Sequence[MobCategoryFact],
    rows: Sequence[NaturalSpawnTypeFact],
) -> None:
    if len(categories) != EXPECTED_CATEGORIES:
        raise NaturalSpawnExtractionError(
            f"expected {EXPECTED_CATEGORIES} category rows, found {len(categories)}"
        )
    if len(rows) != EXPECTED_ENTITY_TYPES:
        raise NaturalSpawnExtractionError(
            f"expected {EXPECTED_ENTITY_TYPES} entity rows, found {len(rows)}"
        )
    if [row.entity_type_id for row in rows] != list(range(EXPECTED_ENTITY_TYPES)):
        raise NaturalSpawnExtractionError("entity ids are not contiguous in registry order")
    if len({row.constant_name for row in rows}) != len(rows):
        raise NaturalSpawnExtractionError("entity constants are not unique")
    if len({row.resource_id for row in rows}) != len(rows):
        raise NaturalSpawnExtractionError("entity resource ids are not unique")
    predicate_rows = [row for row in rows if row.predicate_owner is not None]
    if len(predicate_rows) != EXPECTED_PLACEMENTS:
        raise NaturalSpawnExtractionError(
            f"expected {EXPECTED_PLACEMENTS} predicate owners, found {len(predicate_rows)}"
        )
    for row in rows:
        if (row.predicate_owner is None) != (row.predicate_method is None):
            raise NaturalSpawnExtractionError(
                f"{row.resource_id}: predicate owner/method are partially populated"
            )
    actual_mobs = sum(row.is_mob for row in rows)
    for row in rows:
        if (row.check_spawn_obstruction_owner is not None) != row.is_mob:
            raise NaturalSpawnExtractionError(
                f"{row.resource_id}: Mob virtual owners disagree with Mob classification"
            )
        if row.runtime_class is None and row.is_mob:
            raise NaturalSpawnExtractionError(
                f"{row.resource_id}: createNothing row has Mob virtual owners"
            )
    if actual_mobs != EXPECTED_MOBS:
        raise NaturalSpawnExtractionError(f"expected {EXPECTED_MOBS} Mob rows, found {actual_mobs}")


def _option(value: str | None) -> str:
    return "None" if value is None else f'Some("{value}")'


def render_catalog(catalog: NaturalSpawnCatalog) -> str:
    validate_catalog(catalog.categories, catalog.entity_types)
    lines = [
        "// @generated by scripts/generate-server-natural-spawn-facts.py; do not edit manually.",
        "// Vanilla 26.1: MobCategory.java:6-59; SpawnPlacements.java:48-180;",
        "// EntityType.java registrations and entity class hierarchies.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-server-natural-spawn-facts.py",
        "// Check: scripts/vanilla-source-tools.sh "
        "scripts/generate-server-natural-spawn-facts.py --check",
        "",
        "use bbb_protocol::entity_types::VanillaEntityMobCategory;",
        "",
        "use crate::entity::minecart_spawner::spawn_placement::{",
        "    SpawnHeightmap, SpawnPlacementEntry, SpawnPlacementType, SpawnPredicateKind,",
        "};",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub struct VanillaMobCategorySpawnFacts26_1 {",
        "    pub category: VanillaEntityMobCategory,",
        "    pub max_instances_per_chunk: i32,",
        "    pub friendly: bool,",
        "    pub persistent: bool,",
        "    pub no_despawn_distance: i32,",
        "    pub despawn_distance: i32,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub struct VanillaNaturalSpawnTypeFacts26_1 {",
        "    pub entity_type_id: i32,",
        "    pub resource_id: &'static str,",
        "    pub is_mob: bool,",
        "    pub category: VanillaEntityMobCategory,",
        "    pub placement: SpawnPlacementEntry,",
        "    pub check_spawn_obstruction_owner: Option<&'static str>,",
    ]
    lines.extend(
        [
            "    pub walk_target_value_owner: Option<&'static str>,",
            "}",
            "",
            "#[rustfmt::skip]",
            "pub const VANILLA_MOB_CATEGORY_SPAWN_FACTS_26_1: "
            "&[VanillaMobCategorySpawnFacts26_1] = &[",
        ]
    )
    for row in catalog.categories:
        lines.append(
            "    VanillaMobCategorySpawnFacts26_1 { "
            f"category: VanillaEntityMobCategory::{row.variant}, "
            f"max_instances_per_chunk: {row.max_instances_per_chunk}, "
            f"friendly: {str(row.friendly).lower()}, persistent: {str(row.persistent).lower()}, "
            f"no_despawn_distance: {row.no_despawn_distance}, "
            f"despawn_distance: {row.despawn_distance} "
            "},"
        )
    lines.extend(
        [
            "];",
            "",
            "#[rustfmt::skip]",
            "pub const VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1: "
            "&[VanillaNaturalSpawnTypeFacts26_1] = &[",
        ]
    )
    for row in catalog.entity_types:
        lines.extend(
            [
                "    VanillaNaturalSpawnTypeFacts26_1 {",
                f"        entity_type_id: {row.entity_type_id}, "
                f'resource_id: "{row.resource_id}", is_mob: {str(row.is_mob).lower()},',
                f"        category: VanillaEntityMobCategory::{row.category_variant},",
                "        placement: SpawnPlacementEntry { "
                f"placement: SpawnPlacementType::{row.placement}, "
                f"heightmap: SpawnHeightmap::{row.heightmap}, "
                f"predicate: SpawnPredicateKind::{row.predicate_kind} "
                "},",
                f"        check_spawn_obstruction_owner: {_option(row.check_spawn_obstruction_owner)},",
            ]
        )
        lines.append(
            f"        walk_target_value_owner: {_option(row.walk_target_value_owner)},"
        )
        lines.append("    },")
    lines.extend(
        [
            "];",
            "",
            "pub fn vanilla_mob_category_spawn_facts_26_1(",
            "    category: VanillaEntityMobCategory,",
            ") -> Option<&'static VanillaMobCategorySpawnFacts26_1> {",
            "    VANILLA_MOB_CATEGORY_SPAWN_FACTS_26_1",
            "        .iter()",
            "        .find(|facts| facts.category == category)",
            "}",
            "",
            "pub fn vanilla_natural_spawn_type_facts_by_id_26_1(",
            "    entity_type_id: i32,",
            ") -> Option<&'static VanillaNaturalSpawnTypeFacts26_1> {",
            "    usize::try_from(entity_type_id)",
            "        .ok()",
            "        .and_then(|id| VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1.get(id))",
            "        .filter(|facts| facts.entity_type_id == entity_type_id)",
            "}",
            "",
            "pub fn vanilla_natural_spawn_type_facts_26_1(",
            "    resource_id: &str,",
            ") -> Option<&'static VanillaNaturalSpawnTypeFacts26_1> {",
            "    VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1",
            "        .iter()",
            "        .find(|facts| facts.resource_id == resource_id)",
            "}",
            "",
            "pub fn vanilla_natural_spawn_types_for_category_26_1(",
            "    category: VanillaEntityMobCategory,",
            ") -> impl Iterator<Item = &'static VanillaNaturalSpawnTypeFacts26_1> {",
            "    VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1",
            "        .iter()",
            "        .filter(move |facts| facts.category == category)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn natural_spawn_facts_are_total_unique_and_lookupable() {",
            "        assert_eq!(VANILLA_MOB_CATEGORY_SPAWN_FACTS_26_1.len(), 8);",
            "        assert_eq!(VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1.len(), 157);",
            "        let mut resources = HashSet::new();",
            "        for (id, facts) in VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1.iter().enumerate() {",
            "            assert_eq!(facts.entity_type_id, id as i32);",
            "            assert!(resources.insert(facts.resource_id));",
            "            assert_eq!(vanilla_natural_spawn_type_facts_by_id_26_1("
            "id as i32), Some(facts));",
            "            assert_eq!(vanilla_natural_spawn_type_facts_26_1("
            "facts.resource_id), Some(facts));",
            "        }",
            "        assert!(vanilla_natural_spawn_type_facts_26_1("
            '"minecraft:not_real").is_none());',
            "    }",
            "",
            "    #[test]",
            "    fn registered_and_default_placements_are_preserved() {",
            '        let cod = vanilla_natural_spawn_type_facts_26_1("minecraft:cod").unwrap();',
            "        assert_eq!(cod.placement.predicate, SpawnPredicateKind::SurfaceWaterAnimal);",
            "        let allay = vanilla_natural_spawn_type_facts_26_1("
            '"minecraft:allay").unwrap();',
            "        assert_eq!(allay.placement, SpawnPlacementEntry {",
            "            placement: SpawnPlacementType::NoRestrictions,",
            "            heightmap: SpawnHeightmap::MotionBlockingNoLeaves,",
            "            predicate: SpawnPredicateKind::Unrestricted,",
            "        });",
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
        check=False,
    )
    if formatted.returncode != 0:
        raise NaturalSpawnExtractionError("rustfmt rejected generated source:\n" + formatted.stderr)
    return formatted.stdout


def render(source_root: Path) -> str:
    return render_catalog(build_catalog(extract(source_root)))


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    generated = render(args.source_root)
    current = args.output.read_text(encoding="utf-8") if args.output.exists() else ""
    if args.check:
        if current == generated:
            return 0
        sys.stderr.writelines(
            difflib.unified_diff(
                current.splitlines(keepends=True),
                generated.splitlines(keepends=True),
                fromfile=str(args.output),
                tofile=str(args.output) + " (generated)",
            )
        )
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
