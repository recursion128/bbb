#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 gameplay numeric-id catalogs.

The current catalog covers attributes and mob effects. Normal builds consume
only the generated Rust files; this maintenance tool reads local vanilla
sources.
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
DEFAULT_ATTRIBUTE_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/attribute_types.rs"
DEFAULT_MOB_EFFECT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/mob_effect_types.rs"
)
ATTRIBUTE_TYPES_SOURCE = Path(
    "net/minecraft/world/entity/ai/attributes/Attributes.java"
)
MOB_EFFECT_TYPES_SOURCE = Path("net/minecraft/world/effect/MobEffects.java")


@dataclass(frozen=True)
class AttributeEntry:
    constant_name: str
    resource_id: str
    description_key: str
    sentiment: str
    syncable: bool


@dataclass(frozen=True)
class MobEffectEntry:
    constant_name: str
    resource_id: str
    category: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument(
        "--attribute-types-output", type=Path, default=DEFAULT_ATTRIBUTE_TYPES_OUTPUT
    )
    parser.add_argument(
        "--mob-effect-types-output", type=Path, default=DEFAULT_MOB_EFFECT_TYPES_OUTPUT
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


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


def attribute_entries(source: str) -> list[AttributeEntry]:
    declaration = re.compile(
        r"public static final Holder<Attribute>\s+(\w+)\s*=\s*register\("
    )
    entries: list[AttributeEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        metadata = re.match(
            r'\s*"([^"]+)"\s*,\s*new RangedAttribute\(\s*"([^"]+)"',
            arguments,
        )
        if metadata is None:
            raise ValueError(f"missing attribute metadata for {constant_name}")
        path, description_key = metadata.groups()
        sentiment_match = re.search(
            r"\.setSentiment\(Attribute\.Sentiment\.(POSITIVE|NEUTRAL|NEGATIVE)\)",
            arguments,
        )
        entries.append(
            AttributeEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{path}",
                description_key=description_key,
                sentiment=(sentiment_match.group(1) if sentiment_match else "POSITIVE"),
                syncable=".setSyncable(true)" in arguments,
            )
        )
    if len(entries) != 35:
        raise ValueError(f"expected 35 attributes, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate attribute constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate attribute resource id")
    return entries


def mob_effect_entries(source: str) -> list[MobEffectEntry]:
    declaration = re.compile(
        r"public static final Holder<MobEffect>\s+(\w+)\s*=\s*register\("
    )
    entries: list[MobEffectEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        resource = re.match(r'\s*"([^"]+)"\s*,', arguments)
        category = re.search(
            r"MobEffectCategory\.(BENEFICIAL|HARMFUL|NEUTRAL)", arguments
        )
        if resource is None or category is None:
            raise ValueError(f"missing mob effect metadata for {constant_name}")
        entries.append(
            MobEffectEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{resource.group(1)}",
                category=category.group(1),
            )
        )
    if len(entries) != 40:
        raise ValueError(f"expected 40 mob effects, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate mob effect constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate mob effect resource id")
    return entries


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render_attributes(entries: list[AttributeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Attributes.java static register calls.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum AttributeSentiment {",
        "    Positive,",
        "    Neutral,",
        "    Negative,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct AttributeTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub description_key: &'static str,",
        "    pub sentiment: AttributeSentiment,",
        "    pub syncable: bool,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_ATTRIBUTE_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(("", "pub const VANILLA_ATTRIBUTE_TYPES: &[AttributeTypeCatalogEntry] = &["))
    for entry_id, entry in enumerate(entries):
        sentiment = entry.sentiment.capitalize()
        lines.extend(
            (
                "    AttributeTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        description_key: {rust_string(entry.description_key)},",
                f"        sentiment: AttributeSentiment::{sentiment},",
                f"        syncable: {str(entry.syncable).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_attribute_type(id: i32) -> Option<&'static AttributeTypeCatalogEntry> {",
            "    VANILLA_ATTRIBUTE_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_attribute_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static AttributeTypeCatalogEntry> {",
            "    VANILLA_ATTRIBUTE_TYPES.iter().find(|entry| entry.resource_id == resource_id)",
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
            "    fn attribute_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.len(), 35);",
            "        let mut constants = HashSet::new();",
            "        let mut resources = HashSet::new();",
            "        for (id, entry) in VANILLA_ATTRIBUTE_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.iter().filter(|entry| entry.sentiment == AttributeSentiment::Negative).count(), 2);",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.iter().filter(|entry| entry.sentiment == AttributeSentiment::Neutral).count(), 4);",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.iter().filter(|entry| entry.syncable).count(), 27);",
            "    }",
            "",
            "    #[test]",
            "    fn attribute_catalog_preserves_vanilla_order_and_metadata() {",
            "        assert_eq!(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, 2);",
            "        assert_eq!(VANILLA_ATTRIBUTE_SCALE_ID, 25);",
            "        assert_eq!(VANILLA_ATTRIBUTE_WAYPOINT_RECEIVE_RANGE_ID, 34);",
            "        assert_eq!(vanilla_attribute_type(7).unwrap().sentiment, AttributeSentiment::Negative);",
            "        assert_eq!(vanilla_attribute_type(25).unwrap().sentiment, AttributeSentiment::Neutral);",
            "        assert_eq!(vanilla_attribute_type_for_resource_id(\"minecraft:attack_speed\").unwrap().id, 4);",
            "        assert_eq!(vanilla_attribute_type(-1), None);",
            "        assert_eq!(vanilla_attribute_type(35), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else { return; };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/entity/ai/attributes/Attributes.java\").is_file() { return; }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-gameplay-id-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--attribute-types-output\")",
            "            .arg(manifest_dir.join(\"src/attribute_types.rs\"))",
            "            .arg(\"--mob-effect-types-output\")",
            "            .arg(manifest_dir.join(\"src/mob_effect_types.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_mob_effects(entries: list[MobEffectEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MobEffects.java static register calls.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum MobEffectCategory {",
        "    Beneficial,",
        "    Harmful,",
        "    Neutral,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct MobEffectTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub category: MobEffectCategory,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_MOB_EFFECT_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(
        ("", "pub const VANILLA_MOB_EFFECT_TYPES: &[MobEffectTypeCatalogEntry] = &[")
    )
    for entry_id, entry in enumerate(entries):
        category = entry.category.capitalize()
        lines.extend(
            (
                "    MobEffectTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        category: MobEffectCategory::{category},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_mob_effect_type(id: i32) -> Option<&'static MobEffectTypeCatalogEntry> {",
            "    VANILLA_MOB_EFFECT_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_mob_effect_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static MobEffectTypeCatalogEntry> {",
            "    VANILLA_MOB_EFFECT_TYPES.iter().find(|entry| entry.resource_id == resource_id)",
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
            "    fn mob_effect_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.len(), 40);",
            "        let mut constants = HashSet::new();",
            "        let mut resources = HashSet::new();",
            "        for (id, entry) in VANILLA_MOB_EFFECT_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.iter().filter(|entry| entry.category == MobEffectCategory::Beneficial).count(), 20);",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.iter().filter(|entry| entry.category == MobEffectCategory::Harmful).count(), 16);",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.iter().filter(|entry| entry.category == MobEffectCategory::Neutral).count(), 4);",
            "    }",
            "",
            "    #[test]",
            "    fn mob_effect_catalog_preserves_vanilla_order_and_metadata() {",
            "        assert_eq!(VANILLA_MOB_EFFECT_SPEED_ID, 0);",
            "        assert_eq!(VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 15);",
            "        assert_eq!(VANILLA_MOB_EFFECT_BREATH_OF_THE_NAUTILUS_ID, 39);",
            "        assert_eq!(vanilla_mob_effect_type(18).unwrap().category, MobEffectCategory::Harmful);",
            "        assert_eq!(vanilla_mob_effect_type(30).unwrap().category, MobEffectCategory::Neutral);",
            "        assert_eq!(vanilla_mob_effect_type_for_resource_id(\"minecraft:conduit_power\").unwrap().id, 28);",
            "        assert_eq!(vanilla_mob_effect_type(-1), None);",
            "        assert_eq!(vanilla_mob_effect_type(40), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else { return; };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/effect/MobEffects.java\").is_file() { return; }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-gameplay-id-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--attribute-types-output\")",
            "            .arg(manifest_dir.join(\"src/attribute_types.rs\"))",
            "            .arg(\"--mob-effect-types-output\")",
            "            .arg(manifest_dir.join(\"src/mob_effect_types.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def format_rust(rendered: str) -> str:
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input=rendered,
        text=True,
        capture_output=True,
        check=True,
    )
    return formatted.stdout


def generate_attributes(source_root: Path) -> str:
    source = (source_root / ATTRIBUTE_TYPES_SOURCE).read_text()
    return format_rust(render_attributes(attribute_entries(source)))


def generate_mob_effects(source_root: Path) -> str:
    source = (source_root / MOB_EFFECT_TYPES_SOURCE).read_text()
    return format_rust(render_mob_effects(mob_effect_entries(source)))


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
    generated_attributes = generate_attributes(args.source_root)
    generated_mob_effects = generate_mob_effects(args.source_root)
    if args.check:
        matches = check_generated(
            args.attribute_types_output, generated_attributes
        )
        matches &= check_generated(
            args.mob_effect_types_output, generated_mob_effects
        )
        return 0 if matches else 1
    args.attribute_types_output.write_text(generated_attributes)
    args.mob_effect_types_output.write_text(generated_mob_effects)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
