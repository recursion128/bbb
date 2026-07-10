#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 core registry catalogs.

The current first-stage catalog covers damage types. Resource ids come from the
DamageTypes key declarations, while numeric ids follow the bootstrap registration
order. Normal builds consume only the generated Rust file; this maintenance tool
is the only path that needs the local decompiled vanilla sources.
"""

from __future__ import annotations

import argparse
import difflib
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_DAMAGE_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/damage_types.rs"
DEFAULT_ENTITY_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/entity_types.rs"
DEFAULT_BLOCK_ENTITY_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/block_entity_types.rs"
)
DEFAULT_DATA_COMPONENT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/data_component_types.rs"
)
DAMAGE_TYPES_SOURCE = Path("net/minecraft/world/damagesource/DamageTypes.java")
ENTITY_TYPES_SOURCE = Path("net/minecraft/world/entity/EntityType.java")
BLOCK_ENTITY_TYPES_SOURCE = Path(
    "net/minecraft/world/level/block/entity/BlockEntityType.java"
)
DATA_COMPONENT_PREDICATE_TYPES_SOURCE = Path(
    "net/minecraft/core/component/predicates/DataComponentPredicates.java"
)
DATA_COMPONENT_TYPES_SOURCE = Path("net/minecraft/core/component/DataComponents.java")


@dataclass(frozen=True)
class RegistryEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class EntityTypeEntry:
    constant_name: str
    resource_id: str
    allowed_in_peaceful: bool


@dataclass(frozen=True)
class DataComponentTypeEntry:
    constant_name: str
    resource_id: str
    persistent: bool


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument(
        "--damage-types-output", type=Path, default=DEFAULT_DAMAGE_TYPES_OUTPUT
    )
    parser.add_argument(
        "--entity-types-output", type=Path, default=DEFAULT_ENTITY_TYPES_OUTPUT
    )
    parser.add_argument(
        "--block-entity-types-output",
        type=Path,
        default=DEFAULT_BLOCK_ENTITY_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--data-component-types-output",
        type=Path,
        default=DEFAULT_DATA_COMPONENT_TYPES_OUTPUT,
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def method_body(source: str, signature: str) -> str:
    signature_start = source.find(signature)
    if signature_start < 0:
        raise ValueError(f"missing method signature {signature!r}")
    body_start = source.find("{", signature_start + len(signature))
    if body_start < 0:
        raise ValueError(f"missing method body for {signature!r}")

    depth = 0
    for index in range(body_start, len(source)):
        character = source[index]
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
            if depth == 0:
                return source[body_start + 1 : index]
    raise ValueError(f"unterminated method body for {signature!r}")


def damage_type_entries(source: str) -> list[RegistryEntry]:
    declaration = re.compile(
        r"ResourceKey<DamageType>\s+(\w+)\s*=\s*ResourceKey\.create\("
        r"Registries\.DAMAGE_TYPE,\s*Identifier\.withDefaultNamespace\("
        r'"([^"]+)"\)\s*\);'
    )
    resources: dict[str, str] = {}
    for constant_name, path in declaration.findall(source):
        resource_id = f"minecraft:{path}"
        previous = resources.setdefault(constant_name, resource_id)
        if previous != resource_id:
            raise ValueError(
                f"conflicting damage type ids for {constant_name}: "
                f"{previous}, {resource_id}"
            )
    if not resources:
        raise ValueError("no damage type key declarations parsed")

    bootstrap = method_body(source, "static void bootstrap")
    registration = re.compile(r"context\.register\(\s*(\w+)\s*,")
    registered_names = registration.findall(bootstrap)
    if not registered_names:
        raise ValueError("no damage type bootstrap registrations parsed")
    if len(set(registered_names)) != len(registered_names):
        raise ValueError("duplicate damage type bootstrap registration")

    unknown = set(registered_names) - resources.keys()
    missing = resources.keys() - set(registered_names)
    if unknown or missing:
        raise ValueError(
            "damage type declaration/bootstrap mismatch: "
            f"unknown={sorted(unknown)}, missing={sorted(missing)}"
        )
    return [RegistryEntry(name, resources[name]) for name in registered_names]


def parenthesized_body(source: str, opening_parenthesis: int) -> str:
    depth = 0
    in_string = False
    escaped = False
    for index in range(opening_parenthesis, len(source)):
        character = source[index]
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            continue
        if character == '"':
            in_string = True
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                return source[opening_parenthesis + 1 : index]
    raise ValueError("unterminated register call")


def entity_type_entries(source: str) -> list[EntityTypeEntry]:
    declaration = re.compile(
        r"public static final EntityType<[^>]+>\s+(\w+)\s*=\s*register\("
    )
    entries: list[EntityTypeEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        resource_match = re.match(r'\s*"([^"]+)"\s*,', arguments)
        if resource_match is None:
            raise ValueError(f"missing vanilla resource id for {constant_name}")
        entries.append(
            EntityTypeEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{resource_match.group(1)}",
                allowed_in_peaceful=".notInPeaceful()" not in arguments,
            )
        )
    if len(entries) != 157:
        raise ValueError(f"expected 157 entity types, parsed {len(entries)}")
    constant_names = [entry.constant_name for entry in entries]
    resource_ids = [entry.resource_id for entry in entries]
    if len(set(constant_names)) != len(constant_names):
        raise ValueError("duplicate entity type constant")
    if len(set(resource_ids)) != len(resource_ids):
        raise ValueError("duplicate entity type resource id")
    return entries


def block_entity_type_entries(source: str) -> list[RegistryEntry]:
    declaration = re.compile(
        r"public static final BlockEntityType<[^>]+>\s+(\w+)\s*=\s*register\("
    )
    entries: list[RegistryEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        resource_match = re.match(r'\s*"([^"]+)"\s*,', arguments)
        if resource_match is None:
            raise ValueError(f"missing vanilla resource id for {constant_name}")
        entries.append(
            RegistryEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{resource_match.group(1)}",
            )
        )
    if len(entries) != 49:
        raise ValueError(f"expected 49 block entity types, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate block entity type constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate block entity type resource id")
    return entries


def data_component_predicate_type_entries(source: str) -> list[RegistryEntry]:
    declaration = re.compile(
        r"public static final DataComponentPredicate\.Type<[^>]+>\s+"
        r"(\w+)\s*=\s*register\("
    )
    entries: list[RegistryEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        resource_match = re.match(r'\s*"([^"]+)"\s*,', arguments)
        if resource_match is None:
            raise ValueError(f"missing predicate resource id for {constant_name}")
        entries.append(
            RegistryEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{resource_match.group(1)}",
            )
        )
    if len(entries) != 15:
        raise ValueError(
            f"expected 15 data component predicate types, parsed {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate data component predicate type constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate data component predicate type resource id")
    return entries


def data_component_type_entries(source: str) -> list[DataComponentTypeEntry]:
    declaration = re.compile(
        r"public static final DataComponentType<[^;\n=]+>\s+"
        r"(\w+)\s*=\s*register\("
    )
    entries: list[DataComponentTypeEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        resource_match = re.match(r'\s*"([^"]+)"\s*,', arguments)
        if resource_match is None:
            raise ValueError(f"missing component resource id for {constant_name}")
        entries.append(
            DataComponentTypeEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{resource_match.group(1)}",
                persistent=".persistent(" in arguments,
            )
        )
    if len(entries) != 110:
        raise ValueError(f"expected 110 data component types, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate data component type constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate data component type resource id")
    return entries


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render_damage_types(entries: list[RegistryEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 DamageTypes.java key declarations and bootstrap order.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DamageTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
        "pub const VANILLA_DAMAGE_TYPES: &[DamageTypeCatalogEntry] = &[",
    ]
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    DamageTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_damage_type(id: i32) -> Option<&'static DamageTypeCatalogEntry> {",
            "    VANILLA_DAMAGE_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_damage_type_resource_id(id: i32) -> Option<&'static str> {",
            "    Some(vanilla_damage_type(id)?.resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn damage_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_DAMAGE_TYPES.len(), 50);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_DAMAGE_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn damage_type_catalog_follows_bootstrap_not_declaration_order() {",
            "        assert_eq!(vanilla_damage_type(0).unwrap().constant_name, \"IN_FIRE\");",
            "        assert_eq!(vanilla_damage_type(35).unwrap().constant_name, \"FIREWORKS\");",
            "        assert_eq!(vanilla_damage_type(48).unwrap().constant_name, \"WIND_CHARGE\");",
            "        assert_eq!(vanilla_damage_type_resource_id(49), Some(\"minecraft:mace_smash\"));",
            "        assert_eq!(vanilla_damage_type(-1), None);",
            "        assert_eq!(vanilla_damage_type(50), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else {",
            "            return;",
            "        };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root",
            "            .join(\"net/minecraft/world/damagesource/DamageTypes.java\")",
            "            .is_file()",
            "        {",
            "            return;",
            "        }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-core-registry-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--damage-types-output\")",
            "            .arg(manifest_dir.join(\"src/damage_types.rs\"))",
            "            .arg(\"--entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/entity_types.rs\"))",
            "            .arg(\"--block-entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/block_entity_types.rs\"))",
            "            .arg(\"--data-component-types-output\")",
            "            .arg(manifest_dir.join(\"src/data_component_types.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_entity_types(entries: list[EntityTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 EntityType.java static register calls.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct EntityTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub allowed_in_peaceful: bool,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_ENTITY_TYPE_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(("", "pub const VANILLA_ENTITY_TYPES: &[EntityTypeCatalogEntry] = &["))
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    EntityTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        allowed_in_peaceful: {str(entry.allowed_in_peaceful).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_entity_type(id: i32) -> Option<&'static EntityTypeCatalogEntry> {",
            "    VANILLA_ENTITY_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_entity_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static EntityTypeCatalogEntry> {",
            "    let path = resource_id.strip_prefix(\"minecraft:\").unwrap_or(resource_id);",
            "    if path.is_empty() || path.contains(':') {",
            "        return None;",
            "    }",
            "    VANILLA_ENTITY_TYPES",
            "        .iter()",
            "        .find(|entry| entry.resource_id.strip_prefix(\"minecraft:\") == Some(path))",
            "}",
            "",
            "pub fn vanilla_entity_type_id_for_resource_id(resource_id: &str) -> Option<i32> {",
            "    Some(vanilla_entity_type_for_resource_id(resource_id)?.id)",
            "}",
            "",
            "pub fn vanilla_entity_resource_id_for_type_id(entity_type_id: i32) -> Option<String> {",
            "    Some(vanilla_entity_type(entity_type_id)?.resource_id.to_string())",
            "}",
            "",
            "pub fn vanilla_entity_type_allowed_in_peaceful(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.allowed_in_peaceful)",
            "        .unwrap_or(true)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn entity_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_ENTITY_TYPES.len(), 157);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_ENTITY_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| !entry.allowed_in_peaceful).count(), 37);",
            "    }",
            "",
            "    #[test]",
            "    fn entity_type_catalog_preserves_vanilla_order_and_compatibility_api() {",
            "        assert_eq!(VANILLA_ENTITY_TYPES.first().unwrap().resource_id, \"minecraft:acacia_boat\");",
            "        assert_eq!(VANILLA_ENTITY_TYPE_PLAYER_ID, 155);",
            "        assert_eq!(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, 156);",
            "        assert_eq!(vanilla_entity_type_id_for_resource_id(\"minecraft:zombie\"), Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID));",
            "        assert_eq!(vanilla_entity_type_id_for_resource_id(\"creeper\"), Some(VANILLA_ENTITY_TYPE_CREEPER_ID));",
            "        assert_eq!(vanilla_entity_resource_id_for_type_id(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID).as_deref(), Some(\"minecraft:fishing_bobber\"));",
            "        assert_eq!(vanilla_entity_type_id_for_resource_id(\"example:zombie\"), None);",
            "        assert_eq!(vanilla_entity_resource_id_for_type_id(-1), None);",
            "    }",
            "",
            "    #[test]",
            "    fn peaceful_allowance_comes_from_generated_builder_metadata() {",
            "        assert!(!vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_ZOMBIE_ID));",
            "        assert!(!vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_WITHER_ID));",
            "        assert!(vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_PIG_ID));",
            "        assert!(vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID));",
            "        assert!(vanilla_entity_type_allowed_in_peaceful(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else {",
            "            return;",
            "        };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/entity/EntityType.java\").is_file() {",
            "            return;",
            "        }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-core-registry-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--damage-types-output\")",
            "            .arg(manifest_dir.join(\"src/damage_types.rs\"))",
            "            .arg(\"--entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/entity_types.rs\"))",
            "            .arg(\"--block-entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/block_entity_types.rs\"))",
            "            .arg(\"--data-component-types-output\")",
            "            .arg(manifest_dir.join(\"src/data_component_types.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_block_entity_types(entries: list[RegistryEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 BlockEntityType.java static register calls.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct BlockEntityTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
        "pub const VANILLA_BLOCK_ENTITY_TYPES: &[BlockEntityTypeCatalogEntry] = &[",
    ]
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    BlockEntityTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_block_entity_type(",
            "    type_id: i32,",
            ") -> Option<&'static BlockEntityTypeCatalogEntry> {",
            "    VANILLA_BLOCK_ENTITY_TYPES.get(usize::try_from(type_id).ok()?)",
            "}",
            "",
            "pub fn vanilla_block_entity_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {",
            "    Some(vanilla_block_entity_type(type_id)?.resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn block_entity_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_BLOCK_ENTITY_TYPES.len(), 49);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_BLOCK_ENTITY_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn block_entity_type_catalog_preserves_vanilla_order_and_compatibility_api() {",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(0), Some(\"minecraft:furnace\"));",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(9), Some(\"minecraft:mob_spawner\"));",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(45), Some(\"minecraft:vault\"));",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(48), Some(\"minecraft:copper_golem_statue\"));",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(-1), None);",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(49), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else {",
            "            return;",
            "        };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/level/block/entity/BlockEntityType.java\").is_file() {",
            "            return;",
            "        }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-core-registry-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--damage-types-output\")",
            "            .arg(manifest_dir.join(\"src/damage_types.rs\"))",
            "            .arg(\"--entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/entity_types.rs\"))",
            "            .arg(\"--block-entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/block_entity_types.rs\"))",
            "            .arg(\"--data-component-types-output\")",
            "            .arg(manifest_dir.join(\"src/data_component_types.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_data_component_types(
    component_entries: list[DataComponentTypeEntry],
    predicate_entries: list[RegistryEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 DataComponents.java and DataComponentPredicates.java.",
        "// Wire codecs remain hand-bound in packets/data_components/catalog.rs.",
        "",
        "pub use crate::packets::data_components::catalog::{",
        "    vanilla_data_component_descriptor, vanilla_data_component_descriptors, DataComponentDescriptor,",
        "};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DataComponentTypeCatalogEntry {",
        "    pub type_id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub persistent: bool,",
        "}",
        "",
        "pub const VANILLA_DATA_COMPONENT_TYPES: &[DataComponentTypeCatalogEntry] = &[",
    ]
    for type_id, entry in enumerate(component_entries):
        lines.extend(
            (
                "    DataComponentTypeCatalogEntry {",
                f"        type_id: {type_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        persistent: {str(entry.persistent).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
        "];",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DataComponentPredicateTypeCatalogEntry {",
        "    pub type_id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
        "pub const VANILLA_DATA_COMPONENT_PREDICATE_TYPES: &[DataComponentPredicateTypeCatalogEntry] = &[",
        )
    )
    for type_id, entry in enumerate(predicate_entries):
        lines.extend(
            (
                "    DataComponentPredicateTypeCatalogEntry {",
                f"        type_id: {type_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_data_component_type(",
            "    type_id: i32,",
            ") -> Option<&'static DataComponentTypeCatalogEntry> {",
            "    VANILLA_DATA_COMPONENT_TYPES.get(usize::try_from(type_id).ok()?)",
            "}",
            "",
            "pub fn vanilla_data_component_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {",
            "    Some(vanilla_data_component_type(type_id)?.resource_id)",
            "}",
            "",
            "pub fn vanilla_data_component_type_is_persistent(type_id: i32) -> bool {",
            "    vanilla_data_component_type(type_id).is_some_and(|entry| entry.persistent)",
            "}",
            "",
            "pub fn vanilla_data_component_predicate_type(",
            "    type_id: i32,",
            ") -> Option<&'static DataComponentPredicateTypeCatalogEntry> {",
            "    VANILLA_DATA_COMPONENT_PREDICATE_TYPES.get(usize::try_from(type_id).ok()?)",
            "}",
            "",
            "pub fn vanilla_data_component_predicate_resource_id(type_id: i32) -> Option<&'static str> {",
            "    Some(vanilla_data_component_predicate_type(type_id)?.resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn component_type_catalog_is_complete_contiguous_unique_and_persistent() {",
            "        assert_eq!(VANILLA_DATA_COMPONENT_TYPES.len(), 110);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (type_id, entry) in VANILLA_DATA_COMPONENT_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.type_id, type_id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(VANILLA_DATA_COMPONENT_TYPES.iter().filter(|entry| !entry.persistent).map(|entry| entry.type_id).collect::<Vec<_>>(), vec![20, 41, 48]);",
            "    }",
            "",
            "    #[test]",
            "    fn compatibility_queries_delegate_to_component_descriptor_catalog() {",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(0), Some(\"minecraft:custom_data\"));",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(109), Some(\"minecraft:shulker/color\"));",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(-1), None);",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(110), None);",
            "        assert!(vanilla_data_component_type_is_persistent(0));",
            "        assert!(!vanilla_data_component_type_is_persistent(20));",
            "        assert!(!vanilla_data_component_type_is_persistent(41));",
            "        assert!(!vanilla_data_component_type_is_persistent(48));",
            "    }",
            "",
            "    #[test]",
            "    fn predicate_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_DATA_COMPONENT_PREDICATE_TYPES.len(), 15);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (type_id, entry) in VANILLA_DATA_COMPONENT_PREDICATE_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.type_id, type_id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(0), Some(\"minecraft:damage\"));",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(14), Some(\"minecraft:villager/variant\"));",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(-1), None);",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(15), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else {",
            "            return;",
            "        };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/core/component/DataComponents.java\").is_file()",
            "            || !source_root.join(\"net/minecraft/core/component/predicates/DataComponentPredicates.java\").is_file()",
            "        {",
            "            return;",
            "        }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-core-registry-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--damage-types-output\")",
            "            .arg(manifest_dir.join(\"src/damage_types.rs\"))",
            "            .arg(\"--entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/entity_types.rs\"))",
            "            .arg(\"--block-entity-types-output\")",
            "            .arg(manifest_dir.join(\"src/block_entity_types.rs\"))",
            "            .arg(\"--data-component-types-output\")",
            "            .arg(manifest_dir.join(\"src/data_component_types.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def format_rust(source: str) -> str:
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input=source,
        text=True,
        capture_output=True,
        check=True,
    )
    return formatted.stdout


def generated_files(
    source_root: Path,
    damage_types_output: Path,
    entity_types_output: Path,
    block_entity_types_output: Path,
    data_component_types_output: Path,
) -> dict[Path, str]:
    damage_types_source = (source_root / DAMAGE_TYPES_SOURCE).read_text()
    entity_types_source = (source_root / ENTITY_TYPES_SOURCE).read_text()
    block_entity_types_source = (source_root / BLOCK_ENTITY_TYPES_SOURCE).read_text()
    predicate_types_source = (
        source_root / DATA_COMPONENT_PREDICATE_TYPES_SOURCE
    ).read_text()
    component_types_source = (source_root / DATA_COMPONENT_TYPES_SOURCE).read_text()
    return {
        damage_types_output: format_rust(
            render_damage_types(damage_type_entries(damage_types_source))
        ),
        entity_types_output: format_rust(
            render_entity_types(entity_type_entries(entity_types_source))
        ),
        block_entity_types_output: format_rust(
            render_block_entity_types(
                block_entity_type_entries(block_entity_types_source)
            )
        ),
        data_component_types_output: format_rust(
            render_data_component_types(
                data_component_type_entries(component_types_source),
                data_component_predicate_type_entries(predicate_types_source)
            )
        ),
    }


def check_file(path: Path, generated: str) -> bool:
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
    outputs = generated_files(
        args.source_root,
        args.damage_types_output,
        args.entity_types_output,
        args.block_entity_types_output,
        args.data_component_types_output,
    )
    if args.check:
        return 0 if all(check_file(path, generated) for path, generated in outputs.items()) else 1
    for path, generated in outputs.items():
        path.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
