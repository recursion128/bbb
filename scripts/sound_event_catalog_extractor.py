"""Tree-sitter-backed extraction for the vanilla sound-event registry order."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import (
    JavaExpression,
    JavaMethod,
    JavaSource,
    descendants,
    parse_source,
    type_body_members,
)


class SoundEventCatalogExtractionError(ValueError):
    pass


def validate_variable_sound_event_values(
    source: JavaSource, sound_event_source: JavaSource
) -> None:
    contract = parse_source(
        "contract/SoundEvents.java",
        b"""
package net.minecraft.sounds;
class SoundEvents {
  private static Holder<SoundEvent> register(final Identifier id, final Identifier soundId,
      final float range) {
    return Registry.registerForHolder(BuiltInRegistries.SOUND_EVENT, id,
        SoundEvent.createFixedRangeEvent(soundId, range));
  }
  private static SoundEvent register(final String id) {
    return register(Identifier.withDefaultNamespace(id));
  }
  private static SoundEvent register(final Identifier id) { return register(id, id); }
  private static Holder.Reference<SoundEvent> registerForHolder(final String id) {
    return registerForHolder(Identifier.withDefaultNamespace(id));
  }
  private static Holder.Reference<SoundEvent> registerForHolder(final Identifier id) {
    return registerForHolder(id, id);
  }
  private static SoundEvent register(final Identifier id, final Identifier soundId) {
    return Registry.register(BuiltInRegistries.SOUND_EVENT, id,
        SoundEvent.createVariableRangeEvent(soundId));
  }
  private static Holder.Reference<SoundEvent> registerForHolder(
      final Identifier id, final Identifier soundId) {
    return Registry.registerForHolder(BuiltInRegistries.SOUND_EVENT, id,
        SoundEvent.createVariableRangeEvent(soundId));
  }
}
""",
    )

    def helper_facts(java: JavaSource) -> dict:
        facts = {}
        for method in java.methods_for("net.minecraft.sounds.SoundEvents"):
            if method.name not in {"register", "registerForHolder"}:
                continue
            key = (method.name, tuple(value.canonical for value in method.parameters))
            body = method.node.child_by_field_name("body")
            if body is None or key in facts:
                _fail(java, java.expression(method.node), "missing or duplicate sound value helper")
            facts[key] = java.expression(body).canonical
        return facts

    if helper_facts(source) != helper_facts(contract):
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: sound value registration helper contract changed"
        )
    helpers = [
        method
        for method in source.methods_for("net.minecraft.sounds.SoundEvents")
        if method.name in {"register", "registerForHolder"}
    ]
    for call in source.calls_for("net.minecraft.sounds.SoundEvents"):
        if call.name not in {"register", "registerForHolder"}:
            continue
        if any(
            method.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= method.span.end_byte
            for method in helpers
        ):
            continue
        if call.qualifier is not None or len(call.arguments) != 1:
            _fail(
                source,
                source.expression(call.node),
                "sound value registration must use the single-ID variable-range helper",
            )

    record = next(
        (
            value
            for value in sound_event_source.types
            if value.fqcn == "net.minecraft.sounds.SoundEvent"
        ),
        None,
    )
    parameters = None if record is None else record.node.child_by_field_name("parameters")
    expected = parse_source(
        "contract/SoundEvent.java",
        b"""
package net.minecraft.sounds;
public record SoundEvent(Identifier location, Optional<Float> fixedRange) {
  public static SoundEvent createVariableRangeEvent(final Identifier location) {
    return new SoundEvent(location, Optional.empty());
  }
}
""",
    )
    expected_parameters = expected.types[0].node.child_by_field_name("parameters")
    factory = _method(sound_event_source, "createVariableRangeEvent")
    expected_factory = _method(expected, "createVariableRangeEvent")
    body = factory.node.child_by_field_name("body")
    expected_body = expected_factory.node.child_by_field_name("body")
    if (
        record is None
        or record.kind != "record"
        or parameters is None
        or sound_event_source.expression(parameters).canonical
        != expected.expression(expected_parameters).canonical
        or tuple(value.canonical for value in factory.parameters)
        != tuple(value.canonical for value in expected_factory.parameters)
        or sound_event_source.expression(body).canonical
        != expected.expression(expected_body).canonical
        or any(
            member.type in {"constructor_declaration", "compact_constructor_declaration"}
            for member in type_body_members(record.body_node)
        )
        or any(
            method.name in {"location", "fixedRange"}
            for method in sound_event_source.methods_for("net.minecraft.sounds.SoundEvent")
        )
    ):
        raise SoundEventCatalogExtractionError(
            f"{sound_event_source.relative_path}: variable sound value constructor contract changed"
        )


@dataclass(frozen=True)
class DirectSoundEventEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class SoundSetEntry:
    constant_name: str
    resource_id: str
    sound_event_identifier: str


@dataclass(frozen=True)
class SoundVariantSoundEntry:
    constant_name: str
    resource_id: str
    ambient_sound_event_id: str | None
    adult_hurt_sound_event_id: str
    baby_hurt_sound_event_id: str
    adult_death_sound_event_id: str
    baby_death_sound_event_id: str


SPECIAL_VARIANT_FIELDS = {
    "CAT_SOUNDS": ("cat", "registerCatSoundVariants"),
    "CHICKEN_SOUNDS": ("chicken", "registerChickenSoundVariants"),
    "COW_SOUNDS": ("cow", "registerCowSoundVariants"),
    "PIG_SOUNDS": ("pig", "registerPigSoundVariants"),
    "WOLF_SOUNDS": ("wolf", "registerWolfSoundVariants"),
}

BABY_DEATH_SOUND_FIELDS = {
    "cat": "CAT_DEATH_BABY",
    "chicken": "CHICKEN_DEATH_BABY",
    "cow": None,
    "pig": "PIG_DEATH_BABY",
    "wolf": "WOLF_DEATH_BABY",
}

BABY_HURT_SOUND_FIELDS = {
    "cat": "CAT_HURT_BABY",
    "chicken": "CHICKEN_HURT_BABY",
    "cow": None,
    "pig": "PIG_HURT_BABY",
    "wolf": "WOLF_HURT_BABY",
}


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise SoundEventCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected string literal")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "expected non-empty string literal")
    return value


def _method(source: JavaSource, name: str) -> JavaMethod:
    methods = [method for method in source.methods if method.name == name]
    if len(methods) != 1 or methods[0].body is None:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: expected exactly one {name} method with a body"
        )
    return methods[0]


def _calls_in_method(source: JavaSource, method: JavaMethod, name: str) -> list:
    return [
        call
        for call in source.calls
        if call.name == name
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]


def _direct_registration_resource_id(
    source: JavaSource, expression: JavaExpression
) -> str | None:
    calls = [
        call
        for call in source.calls
        if expression.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= expression.span.end_byte
    ]
    registrations = [
        call for call in calls if call.name in {"register", "registerForHolder"}
    ]
    if not registrations:
        return None
    if len(registrations) != 1 or len(registrations[0].arguments) < 1:
        _fail(source, expression, "unsupported direct sound registration")
    return "minecraft:" + _string(source, registrations[0].arguments[0])


def direct_sound_event_entries(
    source: JavaSource, expected_count: int = 1808
) -> list[DirectSoundEventEntry]:
    entries = [
        DirectSoundEventEntry(field.name, resource_id)
        for field in source.fields_for("net.minecraft.sounds.SoundEvents")
        if field.initializer is not None
        and (resource_id := _direct_registration_resource_id(source, field.initializer))
        is not None
    ]
    if len(entries) != expected_count:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} direct sound events, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: duplicate direct sound constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: duplicate direct sound resource id"
        )
    return entries


def sound_set_entries(source: JavaSource) -> list[SoundSetEntry]:
    sound_sets = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == "SoundSet"
    ]
    if len(sound_sets) != 1:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: expected exactly one SoundSet enum"
        )
    entries: list[SoundSetEntry] = []
    for node in type_body_members(sound_sets[0].body_node):
        if node.type != "enum_constant":
            continue
        arguments = node.child_by_field_name("arguments")
        if arguments is None or len(arguments.named_children) != 2:
            _fail(source, source.expression(node), "SoundSet constant lacks two arguments")
        name = node.child_by_field_name("name")
        if name is None:
            _fail(source, source.expression(node), "SoundSet constant lacks a name")
        entries.append(
            SoundSetEntry(
                source.text(name),
                "minecraft:"
                + _string(source, source.expression(arguments.named_children[0])),
                _string(source, source.expression(arguments.named_children[1])),
            )
        )
    if not entries:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: empty SoundSet enum"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: duplicate SoundSet constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: duplicate SoundSet resource id"
        )
    if len({entry.sound_event_identifier for entry in entries}) != len(entries):
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: duplicate SoundSet event identifier"
        )
    return entries


def sound_set_event_identifiers(source: JavaSource) -> list[str]:
    return [entry.sound_event_identifier for entry in sound_set_entries(source)]


def _variant_helper_suffixes(source: JavaSource, method_name: str) -> list[str]:
    method = _method(source, method_name)
    calls = _calls_in_method(source, method, "registerForHolder")
    suffixes: list[str] = []
    for call in calls:
        if len(call.arguments) != 1:
            _fail(source, source.expression(call.node), "variant sound registration arity changed")
        argument = call.arguments[0]
        literals = [
            source.expression(node)
            for node in [argument.node, *descendants(argument.node)]
            if node.type == "string_literal"
        ]
        if len(literals) != 2 or _string(source, literals[0]) != "entity.":
            _fail(source, argument, "unsupported variant sound id expression")
        suffix = _string(source, literals[1])
        if not suffix.startswith(".") or len(suffix) == 1:
            _fail(source, literals[1], "variant sound suffix must start with a dot")
        suffixes.append(suffix[1:])
    if not suffixes or len(suffixes) != len(set(suffixes)):
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: empty or duplicate suffixes in {method_name}"
        )
    return suffixes


def _goat_horn_ids(source: JavaSource) -> list[str]:
    method = _method(source, "registerGoatHornSoundVariants")
    ranges = _calls_in_method(source, method, "range")
    registrations = _calls_in_method(source, method, "registerForHolder")
    if len(ranges) != 1 or len(ranges[0].arguments) != 2:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: unsupported goat-horn IntStream range"
        )
    bounds = [argument.canonical for argument in ranges[0].arguments]
    if bounds[0] != "0" or not bounds[1].isdigit():
        _fail(source, source.expression(ranges[0].node), "goat-horn range is not 0..integer")
    if len(registrations) != 1 or len(registrations[0].arguments) != 1:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: unsupported goat-horn registration template"
        )
    template = registrations[0].arguments[0]
    literals = [
        source.expression(node)
        for node in [template.node, *descendants(template.node)]
        if node.type == "string_literal"
    ]
    if len(literals) != 1 or _string(source, literals[0]) != "item.goat_horn.sound.":
        _fail(source, template, "unsupported goat-horn sound id expression")
    return [f"minecraft:item.goat_horn.sound.{index}" for index in range(int(bounds[1]))]


def sound_event_resource_ids(
    source: JavaSource,
    variant_sources: dict[str, JavaSource],
    expected_count: int = 1902,
) -> list[str]:
    owner = "net.minecraft.sounds.SoundEvents"
    fields = source.fields_for(owner)
    if not fields:
        raise SoundEventCatalogExtractionError(f"{source.relative_path}: SoundEvents has no fields")

    variants: dict[str, list[str]] = {}
    suffixes: dict[str, list[str]] = {}
    present_fields = {field.name for field in fields}
    for field_name, (family, helper_name) in SPECIAL_VARIANT_FIELDS.items():
        if field_name not in present_fields:
            continue
        if family not in variant_sources:
            raise SoundEventCatalogExtractionError(f"missing {family} SoundSet source")
        variants[family] = sound_set_event_identifiers(variant_sources[family])
        suffixes[family] = _variant_helper_suffixes(source, helper_name)

    ids: list[str] = []
    explained_fields = 0
    for field in fields:
        initializer = field.initializer
        if initializer is None:
            continue
        calls = [
            call
            for call in source.calls
            if initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= initializer.span.end_byte
        ]
        direct_resource_id = _direct_registration_resource_id(source, initializer)
        if direct_resource_id is not None:
            ids.append(direct_resource_id)
            explained_fields += 1
            continue
        if field.name == "GOAT_HORN_SOUND_VARIANTS":
            ids.extend(_goat_horn_ids(source))
            explained_fields += 1
            continue
        special = SPECIAL_VARIANT_FIELDS.get(field.name)
        if special is not None:
            family, expected_helper = special
            if len(calls) != 1 or calls[0].name != expected_helper:
                _fail(source, initializer, f"unsupported {field.name} initializer")
            ids.extend(
                f"minecraft:entity.{sound_id}.{suffix}"
                for sound_id in variants[family]
                for suffix in suffixes[family]
            )
            explained_fields += 1
            continue
        if field.name != "GOAT_HORN_VARIANT_COUNT":
            _fail(source, initializer, f"unexplained SoundEvents field {field.name}")

    if explained_fields != len(fields) - 1:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: explained {explained_fields}/{len(fields) - 1} "
            "registry fields"
        )
    if len(ids) != expected_count:
        raise SoundEventCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} sound events, extracted {len(ids)}"
        )
    if len(ids) != len(set(ids)):
        raise SoundEventCatalogExtractionError(f"{source.relative_path}: duplicate sound event id")
    return ids


def animal_sound_variant_sound_entries(
    source: JavaSource,
    variant_sources: dict[str, JavaSource],
    expected_direct_count: int = 1808,
) -> dict[str, list[SoundVariantSoundEntry]]:
    direct_events = {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(source, expected_direct_count)
    }
    present_fields = {
        field.name for field in source.fields_for("net.minecraft.sounds.SoundEvents")
    }
    catalogs = {}
    for field_name, (family, helper_name) in SPECIAL_VARIANT_FIELDS.items():
        if field_name not in present_fields:
            continue
        variant_source = variant_sources.get(family)
        if variant_source is None:
            raise SoundEventCatalogExtractionError(
                f"missing {family} SoundSet source"
            )
        suffixes = _variant_helper_suffixes(source, helper_name)
        if not {"hurt", "death"}.issubset(suffixes):
            raise SoundEventCatalogExtractionError(
                f"{source.relative_path}: {helper_name} no longer registers hurt/death sounds"
            )
        baby_hurt_field = BABY_HURT_SOUND_FIELDS[family]
        baby_hurt_sound = (
            direct_events.get(baby_hurt_field)
            if baby_hurt_field is not None
            else None
        )
        if baby_hurt_field is not None and baby_hurt_sound is None:
            raise SoundEventCatalogExtractionError(
                f"{source.relative_path}: missing direct baby hurt sound {baby_hurt_field}"
            )
        baby_death_field = BABY_DEATH_SOUND_FIELDS[family]
        baby_death_sound = (
            direct_events.get(baby_death_field)
            if baby_death_field is not None
            else None
        )
        if baby_death_field is not None and baby_death_sound is None:
            raise SoundEventCatalogExtractionError(
                f"{source.relative_path}: missing direct baby death sound {baby_death_field}"
            )
        catalogs[family] = [
            SoundVariantSoundEntry(
                entry.constant_name,
                entry.resource_id,
                f"minecraft:entity.{entry.sound_event_identifier}.ambient"
                if "ambient" in suffixes else None,
                f"minecraft:entity.{entry.sound_event_identifier}.hurt",
                baby_hurt_sound
                or f"minecraft:entity.{entry.sound_event_identifier}.hurt",
                f"minecraft:entity.{entry.sound_event_identifier}.death",
                baby_death_sound
                or f"minecraft:entity.{entry.sound_event_identifier}.death",
            )
            for entry in sound_set_entries(variant_source)
        ]
    return catalogs
