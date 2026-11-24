#!/usr/bin/env python3
"""Generate/check server-owned Vanilla 26.1 entity runtime dispatch facts."""

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
from entity_metadata_extractor import (
    ExtractionResult,
    class_nearest_method_owner,
    extract,
    source_for_owner,
)
from item_registry_catalog_extractor import (
    extract_item_declarations,
    extract_item_id_constants,
    item_resource_ids_for_declaration,
)
from sound_event_catalog_extractor import (
    SPECIAL_VARIANT_FIELDS,
    animal_sound_variant_sound_entries,
    direct_sound_event_entries,
)
from vanilla_java_source import canonical_expression, descendants, parse_source

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-server-core/src/entity_runtime_facts_26_1.rs"
DEFAULT_AMBIENT_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/entity_ambient_sounds.rs"
DEFAULT_PICK_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/entity_pick_results.rs"
ITEM_PROTOTYPES = REPO_ROOT / "crates/bbb-facts/data/item_prototype_components_26_1.json"
LIVING_ENTITY = "net.minecraft.world.entity.LivingEntity"
GENERAL_LIVING_EXCLUSIONS = {"minecraft:player", "minecraft:armor_stand"}


@dataclass(frozen=True)
class RuntimeFact:
    entity_type_id: int
    resource_id: str
    runtime_class: str
    is_living: bool


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--pick-output", type=Path, default=DEFAULT_PICK_OUTPUT)
    parser.add_argument("--ambient-output", type=Path, default=DEFAULT_AMBIENT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    args.source_root = resolve_source_root(args)
    return args


def runtime_facts(result: ExtractionResult) -> list[RuntimeFact]:
    rows = []
    for entity_type in result.entity_types:
        runtime_class = entity_type.runtime_class
        if runtime_class is None:
            raise ValueError(f"{entity_type.resource_id}: missing runtime class")
        is_living = entity_type.attribute_factory_owner is not None
        rows.append(
            RuntimeFact(
                entity_type.entity_type_id,
                entity_type.resource_id,
                runtime_class,
                is_living,
            )
        )
    validate_rows(rows)
    return rows


def validate_rows(rows: Sequence[RuntimeFact]) -> None:
    if len(rows) != 157:
        raise ValueError(f"expected 157 Vanilla entity rows, found {len(rows)}")
    if [row.entity_type_id for row in rows] != list(range(157)):
        raise ValueError("Vanilla entity type ids are not contiguous")
    if len({row.resource_id for row in rows}) != len(rows):
        raise ValueError("Vanilla entity resource ids are not unique")
    living = [row for row in rows if row.is_living]
    if len(living) != 92:
        raise ValueError(f"expected 92 DefaultAttributes living rows, found {len(living)}")
    general = [row for row in living if row.resource_id not in GENERAL_LIVING_EXCLUSIONS]
    if len(general) != 90:
        raise ValueError(f"expected 90 GeneralLiving rows, found {len(general)}")
    if {row.resource_id for row in living} & GENERAL_LIVING_EXCLUSIONS != GENERAL_LIVING_EXCLUSIONS:
        raise ValueError("GeneralLiving exclusions are not both living entity types")


def parrot_imitation_facts(
    result: ExtractionResult, source_root: Path
) -> list[tuple[int, str]]:
    owner = "net.minecraft.world.entity.animal.parrot.Parrot"
    source = source_for_owner(result, owner)
    initializer = next(
        field.initializer for field in source.fields_for(owner) if field.name == "MOB_SOUND_MAP"
    )
    if initializer is None:
        raise ValueError("Parrot.MOB_SOUND_MAP has no initializer")
    entity_ids = {entry.constant_name: entry.entity_type_id for entry in result.entity_types}
    sounds = parse_source(
        "net/minecraft/sounds/SoundEvents.java",
        (source_root / "net/minecraft/sounds/SoundEvents.java").read_bytes(),
    )
    sound_ids = {
        entry.constant_name: entry.resource_id for entry in direct_sound_event_entries(sounds)
    }

    def constant(expression, expected_owner: str) -> str:
        node = expression.node
        qualifier = node.child_by_field_name("object")
        field = node.child_by_field_name("field")
        if (
            node.type != "field_access"
            or qualifier is None
            or field is None
            or source.text(qualifier) != expected_owner
        ):
            raise ValueError(f"Parrot.MOB_SOUND_MAP requires {expected_owner} constants")
        return source.text(field)

    rows = []
    for call in source.calls_for(owner):
        if not (
            initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= initializer.span.end_byte
        ):
            continue
        if call.qualifier is None or call.qualifier.canonical != "map":
            continue
        if call.name != "put" or len(call.arguments) != 2:
            raise ValueError("Parrot.MOB_SOUND_MAP put call changed")
        rows.append(
            (
                entity_ids[constant(call.arguments[0], "EntityType")],
                sound_ids[constant(call.arguments[1], "SoundEvents")],
            )
        )
    if not rows or len({entity for entity, _ in rows}) != len(rows):
        raise ValueError("Parrot.MOB_SOUND_MAP is empty or repeats an entity type")
    return sorted(rows)


def render_rows(rows: Sequence[RuntimeFact], parrot_sounds: Sequence[tuple[int, str]]) -> str:
    validate_rows(rows)
    lines = [
        "// @generated by scripts/generate-server-entity-runtime-facts.py; do not edit manually.",
        "// Sources: Vanilla 26.1 EntityType, DefaultAttributes, and entity class hierarchies.",
        "// Parrot.MOB_SOUND_MAP supplies imitation entity/sound pairs.",
        "// Update: scripts/vanilla-source-tools.sh "
        "scripts/generate-server-entity-runtime-facts.py",
        "// Check: scripts/vanilla-source-tools.sh "
        "scripts/generate-server-entity-runtime-facts.py --check",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub struct VanillaEntityRuntimeFacts26_1 {",
        "    pub entity_type_id: i32,",
        "    pub resource_id: &'static str,",
        "    pub runtime_class: &'static str,",
        "    pub is_living: bool,",
        "}",
        "",
        "#[rustfmt::skip]",
        "const VANILLA_ENTITY_RUNTIME_FACTS_26_1: &[VanillaEntityRuntimeFacts26_1] = &[",
    ]
    for row in rows:
        lines.extend(
            [
                "    VanillaEntityRuntimeFacts26_1 {",
                f"        entity_type_id: {row.entity_type_id},",
                f'        resource_id: "{row.resource_id}",',
                f'        runtime_class: "{row.runtime_class}",',
                f"        is_living: {str(row.is_living).lower()},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "#[allow(dead_code)]",
            "pub fn vanilla_entity_runtime_facts_by_id_26_1(",
            "    entity_type_id: i32,",
            ") -> Option<&'static VanillaEntityRuntimeFacts26_1> {",
            "    usize::try_from(entity_type_id)",
            "        .ok()",
            "        .and_then(|id| VANILLA_ENTITY_RUNTIME_FACTS_26_1.get(id))",
            "}",
            "",
            "pub fn vanilla_entity_runtime_facts_26_1(",
            "    resource_id: &str,",
            ") -> Option<&'static VanillaEntityRuntimeFacts26_1> {",
            "    VANILLA_ENTITY_RUNTIME_FACTS_26_1",
            "        .iter()",
            "        .find(|facts| facts.resource_id == resource_id)",
            "}",
            "",
            "#[rustfmt::skip]",
            "pub const VANILLA_PARROT_IMITATION_SOUNDS: &[(i32, &str)] = &[",
            *[f'    ({entity_id}, "{sound}"),' for entity_id, sound in parrot_sounds],
            "];",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn runtime_fact_rows_are_total_unique_and_lookupable() {",
            "        assert_eq!(VANILLA_ENTITY_RUNTIME_FACTS_26_1.len(), 157);",
            "        let mut resources = HashSet::new();",
            "        for (expected_id, facts) in "
            "VANILLA_ENTITY_RUNTIME_FACTS_26_1.iter().enumerate() {",
            "            assert_eq!(facts.entity_type_id, expected_id as i32);",
            "            assert!(resources.insert(facts.resource_id));",
            "            assert_eq!(vanilla_entity_runtime_facts_by_id_26_1(expected_id as i32), "
            "Some(facts));",
            "            assert_eq!(vanilla_entity_runtime_facts_26_1(facts.resource_id), "
            "Some(facts));",
            "        }",
            "        assert!(vanilla_entity_runtime_facts_by_id_26_1(-1).is_none());",
            "        assert!(vanilla_entity_runtime_facts_26_1(\"minecraft:not_real\").is_none());",
            "    }",
            "",
            "    #[test]",
            "    fn default_attributes_membership_is_the_general_living_catalog_owner() {",
            "        let living = VANILLA_ENTITY_RUNTIME_FACTS_26_1.iter()"
            ".filter(|facts| facts.is_living).count();",
            "        let general = VANILLA_ENTITY_RUNTIME_FACTS_26_1",
            "            .iter()",
            "            .filter(|facts| facts.is_living)",
            "            .filter(|facts| !matches!(facts.resource_id, "
            "\"minecraft:player\" | \"minecraft:armor_stand\"))",
            "            .count();",
            "        assert_eq!(living, 92);",
            "        assert_eq!(general, 90);",
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
        raise ValueError("rustfmt rejected generated source:\n" + formatted.stderr)
    return formatted.stdout


def render(source_root: Path) -> str:
    result = extract(source_root)
    return render_rows(runtime_facts(result), parrot_imitation_facts(result, source_root))


@dataclass(frozen=True)
class PickFact:
    entity_type_id: int
    resource_id: str
    owner: str
    kind: str
    item_id: int | None


def method_body(result: ExtractionResult, owner: str, name: str, parameters: int = 0) -> str:
    source = source_for_owner(result, owner)
    methods = [
        m for m in source.methods_for(owner) if m.name == name and len(m.parameters) == parameters
    ]
    if len(methods) != 1:
        raise ValueError(f"{owner}.{name}/{parameters}: expected one declaration")
    body = methods[0].node.child_by_field_name("body")
    if body is None:
        raise ValueError(f"{owner}.{name}: unexpected abstract method")
    return canonical_expression(source.source, body)


def require_body(
    result: ExtractionResult, owner: str, name: str, expected: str, parameters: int = 0
) -> None:
    if method_body(result, owner, name, parameters) != expected:
        raise ValueError(f"{owner}.{name}: unsupported pick behavior")


def pick_facts(result: ExtractionResult, source_root: Path) -> list[PickFact]:
    def parse(relative: str):
        return parse_source(relative, (source_root / relative).read_bytes())

    prototypes = json.loads(ITEM_PROTOTYPES.read_text(encoding="utf-8"))
    if prototypes["vanilla_version"] != "26.1":
        raise ValueError("pick item prototypes are not Vanilla 26.1")
    item_ids = {name: i for i, name in enumerate(prototypes["item_resource_ids"])}
    items = parse("net/minecraft/world/item/Items.java")
    constants = extract_item_id_constants(parse("net/minecraft/references/ItemIds.java"))
    item_constants = {}
    for declaration in extract_item_declarations(items):
        resources = item_resource_ids_for_declaration(declaration, constants)
        if len(resources) == 1:
            item_constants[declaration["field"]] = item_ids["minecraft:" + resources[0]]

    entity_data = [
        c for c in prototypes["component_values"] if c["resource_id"] == "minecraft:entity_data"
    ]
    if len(entity_data) != 1:
        raise ValueError("expected one default ENTITY_DATA component catalog")
    spawn_items = {}
    for entry in entity_data[0]["values"]:
        entity = entry["value"]["id"]
        for start, end in entry["item_id_ranges"]:
            for item_id in range(start, end + 1):
                if entity in spawn_items or not 0 <= item_id < len(item_ids):
                    raise ValueError(f"ambiguous or invalid default ENTITY_DATA item for {entity}")
                spawn_items[entity] = item_id

    class_map = {c.fqcn: c for c in result.classes}
    cache = {}
    frame_cache = {}
    entity_owner = "net.minecraft.world.entity.Entity"
    mob_owner = "net.minecraft.world.entity.Mob"
    boat_owner = "net.minecraft.world.entity.vehicle.boat.AbstractBoat"
    frame_owner = "net.minecraft.world.entity.decoration.ItemFrame"
    require_body(result, entity_owner, "getPickResult", "{return null;}")
    require_body(
        result,
        mob_owner,
        "getPickResult",
        "{return SpawnEggItem.byId (this.getType ()).map (ItemStack::new).orElse (null);}",
    )
    require_body(
        result, boat_owner, "getPickResult", "{return new ItemStack (this.dropItem.get ());}"
    )
    require_body(
        result,
        frame_owner,
        "getPickResult",
        "{ItemStack framedStack = this.getItem (); return framedStack.isEmpty () ? "
        "this.getFrameItemStack () : framedStack.copy ();}",
    )
    require_body(
        result,
        "net.minecraft.world.entity.boss.enderdragon.EnderDragonPart",
        "getPickResult",
        "{return this.parentMob.getPickResult ();}",
    )
    for factory, runtime, parameter in [
        ("boatFactory", "Boat", "boatItem"),
        ("chestBoatFactory", "ChestBoat", "dropItem"),
        ("raftFactory", "Raft", "dropItem"),
        ("chestRaftFactory", "ChestRaft", "dropItem"),
    ]:
        require_body(
            result,
            "net.minecraft.world.entity.EntityType",
            factory,
            "{return (entityType, level) -> new "
            + runtime
            + " (entityType, level, "
            + parameter
            + ");}",
            1,
        )
    for runtime in ["Boat", "ChestBoat", "Raft", "ChestRaft", "AbstractChestBoat"]:
        require_body(
            result,
            "net.minecraft.world.entity.vehicle.boat." + runtime,
            runtime,
            "{super (type, level, dropItem);}",
            3,
        )
    require_body(
        result,
        boat_owner,
        "AbstractBoat",
        "{super (type, level); this.dropItem = dropItem; this.blocksBuilding = true;}",
        3,
    )
    spawn_source = parse("net/minecraft/world/item/SpawnEggItem.java")
    by_id = next(m for m in spawn_source.methods if m.name == "byId")
    if (
        canonical_expression(spawn_source.source, by_id.node.child_by_field_name("body"))
        != "{return BuiltInRegistries.ITEM.componentLookup ().findMatching "
        "(DataComponents.ENTITY_DATA, c -> c.type () == type).findAny ();}"
    ):
        raise ValueError("SpawnEggItem.byId no longer selects default ENTITY_DATA")

    def constant_item(owner: str, method: str) -> int:
        body = method_body(result, owner, method)
        source = source_for_owner(result, owner)
        declaration = next(
            m for m in source.methods_for(owner) if m.name == method and not m.parameters
        )
        references = [
            source.text(n)
            for n in descendants(declaration.node)
            if n.type == "field_access" and source.text(n).startswith("Items.")
        ]
        if len(references) != 1 or body != "{return new ItemStack (" + references[0] + ");}":
            raise ValueError(f"{owner}.{method}: unsupported constant item result")
        return item_constants[references[0].removeprefix("Items.")]

    registrations = source_for_owner(result, "net.minecraft.world.entity.EntityType")
    fields = {f.name: f for f in registrations.fields_for("net.minecraft.world.entity.EntityType")}
    rows = []
    for entity in result.entity_types:
        owner = class_nearest_method_owner(
            result, class_map, entity.runtime_class, "getPickResult", 0, cache
        )
        item_id = None
        if owner == entity_owner:
            kind = "None"
        elif owner == mob_owner:
            kind, item_id = "Mob", spawn_items.get(entity.resource_id)
        elif owner == frame_owner:
            fallback = class_nearest_method_owner(
                result, class_map, entity.runtime_class, "getFrameItemStack", 0, frame_cache
            )
            kind, item_id = "ItemFrame", constant_item(fallback, "getFrameItemStack")
        elif owner == boat_owner:
            initializer = fields[entity.constant_name].initializer
            factories = [
                n
                for n in descendants(initializer.node)
                if n.type == "method_invocation"
                and registrations.text(n.child_by_field_name("name"))
                in {"boatFactory", "chestBoatFactory", "raftFactory", "chestRaftFactory"}
            ]
            if len(factories) != 1:
                raise ValueError(f"{entity.resource_id}: unsupported boat factory")
            arguments = factories[0].child_by_field_name("arguments")
            suppliers = arguments.named_children
            if len(suppliers) != 1 or suppliers[0].type != "lambda_expression":
                raise ValueError(f"{entity.resource_id}: unsupported boat item supplier")
            supplier = registrations.text(suppliers[0].child_by_field_name("body"))
            if not supplier.startswith("Items."):
                raise ValueError(f"{entity.resource_id}: unsupported boat drop item")
            kind, item_id = "Item", item_constants[supplier.removeprefix("Items.")]
        else:
            kind, item_id = "Item", constant_item(owner, "getPickResult")
        rows.append(PickFact(entity.entity_type_id, entity.resource_id, owner, kind, item_id))
    if [r.entity_type_id for r in rows] != list(range(157)):
        raise ValueError("pick dispatch must cover all 157 registered entity types")
    return rows


def render_pick_rows(rows: Sequence[PickFact]) -> str:
    lines = [
        "// @generated by scripts/generate-server-entity-runtime-facts.py; do not edit manually.",
        "// Sources: getPickResult/getFrameItemStack owners, EntityType boat suppliers, "
        "and Vanilla item prototype ENTITY_DATA.",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub enum VanillaEntityPickResult {",
        "    None,",
        "    Mob { item_id: Option<i32> },",
        "    Item { item_id: i32 },",
        "    ItemFrame { empty_item_id: i32 },",
        "}",
        "",
        "#[rustfmt::skip]",
        "const ENTITY_PICK_RESULTS: &[VanillaEntityPickResult] = &[",
    ]
    for row in rows:
        value = "VanillaEntityPickResult::" + row.kind
        if row.kind == "Mob":
            value += (
                " { item_id: " + ("None" if row.item_id is None else f"Some({row.item_id})") + " }"
            )
        elif row.kind in {"Item", "ItemFrame"}:
            field = "empty_item_id" if row.kind == "ItemFrame" else "item_id"
            value += f" {{ {field}: {row.item_id} }}"
        lines.append(f"    {value},")
    lines.extend(
        [
            "];",
            "",
            "pub fn vanilla_entity_pick_result(entity_type_id: i32) "
            "-> Option<VanillaEntityPickResult> {",
            "    ENTITY_PICK_RESULTS.get(usize::try_from(entity_type_id).ok()?).copied()",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use super::*;",
            "    #[test]",
            "    fn entity_pick_catalog_covers_registered_entities_and_items() {",
            "        assert_eq!(ENTITY_PICK_RESULTS.len(), "
            "crate::entity_types::VANILLA_ENTITY_TYPES.len());",
            "        assert_eq!(vanilla_entity_pick_result(-1), None);",
            "        assert_eq!(vanilla_entity_pick_result(ENTITY_PICK_RESULTS.len() as i32), "
            "None);",
            "        for entity in crate::entity_types::VANILLA_ENTITY_TYPES {",
            "            let pick = vanilla_entity_pick_result(entity.id).unwrap();",
            "            let item = match pick {",
            "                VanillaEntityPickResult::None => None,",
            "                VanillaEntityPickResult::Mob { item_id } => item_id,",
            "                VanillaEntityPickResult::Item { item_id } => Some(item_id),",
            "                VanillaEntityPickResult::ItemFrame { empty_item_id } "
            "=> Some(empty_item_id),",
            "            };",
            "            if let Some(item) = item { "
            "assert!(crate::item_types::vanilla_item_type(item).is_some()); }",
            "        }",
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


@dataclass(frozen=True)
class AmbientFact:
    resource_id: str
    kind: str
    sound: str | None = None
    volume: str | None = None


def spawn_ambient_facts(result: ExtractionResult, source_root: Path) -> list[AmbientFact]:
    """Resolve only constant returns; stateful selectors remain explicitly dynamic."""
    classes = {item.fqcn: item for item in result.classes}
    sounds_source = parse_source(
        "net/minecraft/sounds/SoundEvents.java",
        (source_root / "net/minecraft/sounds/SoundEvents.java").read_bytes(),
    )
    sounds = {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(sounds_source)
    }
    variant_sounds = animal_sound_variant_sound_entries(
        sounds_source,
        {
            family: source
            for _, (family, _) in SPECIAL_VARIANT_FIELDS.items()
            for source in result.sources.values()
            if any(t.simple_name == family.title() + "SoundVariants" for t in source.types)
        },
    )
    fixed_cow_sets = {
        f"SoundEvents.COW_SOUNDS.get (CowSoundVariants.SoundSet.{entry.constant_name})":
        entry.ambient_sound_event_id
        for entry in variant_sounds["cow"]
    }
    mob = "net.minecraft.world.entity.Mob"
    brute = "net.minecraft.world.entity.monster.piglin.PiglinBrute"
    abstract_piglin = "net.minecraft.world.entity.monster.piglin.AbstractPiglin"
    require_body(
        result,
        abstract_piglin,
        "playAmbientSound",
        "{if (PiglinAi.isIdle (this)) {super.playAmbientSound ();}}",
    )
    require_body(
        result,
        "net.minecraft.world.entity.monster.piglin.PiglinAi",
        "isIdle",
        "{return body.getBrain ().isActive (Activity.IDLE);}",
        1,
    )
    require_body(
        result, brute, "makeBrain", "{return BRAIN_PROVIDER.makeBrain (this, packedBrain);}", 1
    )
    brain_source = parse_source(
        "net/minecraft/world/entity/ai/Brain.java",
        (source_root / "net/minecraft/world/entity/ai/Brain.java").read_bytes(),
    )
    brain = "net.minecraft.world.entity.ai.Brain"
    default = next(f for f in brain_source.fields_for(brain) if f.name == "defaultActivity")
    if default.initializer.canonical != "Activity.IDLE":
        raise ValueError("fresh PiglinBrute brain no longer defaults to IDLE")
    (constructor,) = [
        m for m in brain_source.methods_for(brain) if m.name == "Brain" and len(m.parameters) == 5
    ]
    body = constructor.node.child_by_field_name("body")
    if (
        canonical_expression(brain_source.source, body.named_children[-1])
        != "this.useDefaultActivity ();"
    ):
        raise ValueError("fresh PiglinBrute brain no longer activates its default activity")
    caches: dict[str, dict[str, str]] = {}

    def nearest(owner: str, method: str) -> str:
        return class_nearest_method_owner(
            result, classes, owner, method, 0, caches.setdefault(method, {})
        )

    def returned(owner: str, name: str):
        source = source_for_owner(result, owner)
        (method,) = [m for m in source.methods_for(owner) if m.name == name and not m.parameters]
        body = method.node.child_by_field_name("body")
        if body is None or len(body.named_children) != 1:
            return source, None
        statement = body.named_children[0]
        if statement.type != "return_statement" or len(statement.named_children) != 1:
            return source, None
        return source, statement.named_children[0]

    rows = []
    for entity in result.entity_types:
        owner = entity.runtime_class
        ancestor = owner
        while ancestor is not None and ancestor != mob:
            ancestor = classes[ancestor].superclass
        if ancestor != mob:
            continue
        ambient_owner = nearest(owner, "getAmbientSound")
        source, expression = returned(ambient_owner, "getAmbientSound")
        kind, sound, volume = "Dynamic", None, None
        if expression is not None and expression.type == "null_literal":
            kind = "Silent"
        elif expression is not None:
            if expression.type == "field_access":
                qualifier = expression.child_by_field_name("object")
                field = expression.child_by_field_name("field")
                if qualifier is not None and source.text(qualifier) == "SoundEvents":
                    sound = sounds[source.text(field)]
            elif (
                canonical_expression(source.source, expression)
                == "this.getSoundSet ().ambientSound ().value ()"
            ):
                selector_source, selector = returned(nearest(owner, "getSoundSet"), "getSoundSet")
                if selector is not None:
                    sound = fixed_cow_sets.get(
                        canonical_expression(selector_source.source, selector)
                    )
            if sound is not None:
                volume_source, volume_expression = returned(
                    nearest(owner, "getSoundVolume"), "getSoundVolume"
                )
                if (
                    volume_expression is not None
                    and volume_expression.type == "decimal_floating_point_literal"
                ):
                    literal = volume_source.text(volume_expression)
                    if literal.endswith("F"):
                        volume = str(float(literal[:-1]))
                if (
                    volume is not None
                    and (
                        nearest(owner, "playAmbientSound") == mob
                        or (
                            owner == brute and nearest(owner, "playAmbientSound") == abstract_piglin
                        )
                    )
                    and nearest(owner, "getVoicePitch") == LIVING_ENTITY
                ):
                    kind = "Fixed"
        rows.append(AmbientFact(entity.resource_id, kind, sound, volume))
    return rows


def render_spawn_ambient_rows(rows: Sequence[AmbientFact]) -> str:
    lines = [
        "// @generated by scripts/generate-server-entity-runtime-facts.py; do not edit manually.",
        "// Source: Vanilla 26.1 Mob ambient selectors and inherited "
        "volume/voice/playback methods.",
        "// Dynamic includes stateful sound, volume, voice, and playback overrides.",
        "// Fresh PiglinBrute starts in IDLE, satisfying its ambient playback gate.",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq)]",
        "pub enum VanillaMobSpawnAmbientSound {",
        "    Silent,",
        "    Fixed { sound: &'static str, volume: f32 },",
        "    Dynamic,",
        "}",
        "",
        "pub fn vanilla_mob_spawn_ambient_sound(resource_id: &str) "
        "-> Option<VanillaMobSpawnAmbientSound> {",
        "    Some(match resource_id {",
    ]
    for row in rows:
        value = "VanillaMobSpawnAmbientSound::" + row.kind
        if row.kind == "Fixed":
            value += f' {{ sound: "{row.sound}", volume: {row.volume} }}'
        lines.append(f'        "{row.resource_id}" => {value},')
    lines.extend(["        _ => return None,", "    })", "}", ""])
    return subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input="\n".join(lines),
        text=True,
        capture_output=True,
        check=True,
    ).stdout


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    result = extract(args.source_root)
    outputs = [
        (
            args.output,
            render_rows(runtime_facts(result), parrot_imitation_facts(result, args.source_root)),
        ),
        (args.pick_output, render_pick_rows(pick_facts(result, args.source_root))),
        (
            args.ambient_output,
            render_spawn_ambient_rows(spawn_ambient_facts(result, args.source_root)),
        ),
    ]
    failed = False
    for output, generated in outputs:
        current = output.read_text(encoding="utf-8") if output.exists() else ""
        if args.check:
            if current == generated:
                continue
            failed = True
            sys.stderr.writelines(
                difflib.unified_diff(
                    current.splitlines(keepends=True),
                    generated.splitlines(keepends=True),
                    fromfile=str(output),
                    tofile=str(output) + " (generated)",
                )
            )
        else:
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(generated, encoding="utf-8")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
