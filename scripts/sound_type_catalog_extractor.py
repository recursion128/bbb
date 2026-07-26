"""Tree-sitter-backed vanilla ``SoundType`` declaration extraction."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from sound_event_catalog_extractor import direct_sound_event_entries
from vanilla_java_source import JavaExpression, JavaSource


class SoundTypeCatalogExtractionError(ValueError):
    """An explicit unsupported or inconsistent SoundType source."""


SOUND_TYPE_OWNER = "net.minecraft.world.level.block.SoundType"


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise SoundTypeCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _field_constant(
    source: JavaSource, expression: JavaExpression, owner: str
) -> str:
    node = expression.node
    if node.type not in {"field_access", "scoped_identifier"}:
        _fail(source, expression, f"expected {owner} field constant")
    object_node = node.child_by_field_name("object") or node.child_by_field_name("scope")
    field_node = node.child_by_field_name("field") or node.child_by_field_name("name")
    if object_node is None or field_node is None:
        _fail(source, expression, f"incomplete {owner} field constant")
    if source.expression(object_node).canonical.replace(" ", "") != owner:
        _fail(source, expression, f"expected {owner} field constant")
    return source.text(field_node)


@dataclass(frozen=True)
class ExtractedSoundTypeCatalog:
    declarations: tuple[dict[str, Any], ...]

    def as_json_value(self) -> dict[str, Any]:
        return {
            "vanilla_version": "26.1",
            "generated_by": "scripts/generate-sound-type-catalog.py",
            "update_command": (
                "scripts/vanilla-source-tools.sh "
                "scripts/generate-sound-type-catalog.py"
            ),
            "check_command": (
                "scripts/vanilla-source-tools.sh "
                "scripts/generate-sound-type-catalog.py --check"
            ),
            "sources": [
                "net/minecraft/sounds/SoundEvents.java",
                "net/minecraft/world/level/block/SoundType.java",
            ],
            "declarations": list(self.declarations),
        }


def extract_sound_type_catalog(
    sound_type_source: JavaSource,
    sound_events_source: JavaSource,
    expected_sound_event_count: int = 1808,
) -> ExtractedSoundTypeCatalog:
    sound_events = {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(
            sound_events_source, expected_sound_event_count
        )
    }
    fields = [
        field
        for field in sound_type_source.fields_for(SOUND_TYPE_OWNER)
        if field.type_spelling == "SoundType"
    ]
    declarations: list[dict[str, Any]] = []
    for declaration_order, field in enumerate(fields):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "object_creation_expression":
            expression = initializer or sound_type_source.expression(field.node)
            _fail(sound_type_source, expression, f"SoundType.{field.name} lacks constructor")
        type_node = initializer.node.child_by_field_name("type")
        arguments = initializer.node.child_by_field_name("arguments")
        if (
            type_node is None
            or sound_type_source.expression(type_node).canonical.replace(" ", "")
            != "SoundType"
            or arguments is None
            or len(arguments.named_children) != 7
        ):
            _fail(sound_type_source, initializer, f"unsupported SoundType.{field.name} constructor")
        args = [
            sound_type_source.expression(argument)
            for argument in arguments.named_children
        ]
        break_constant = _field_constant(sound_type_source, args[2], "SoundEvents")
        place_constant = _field_constant(sound_type_source, args[4], "SoundEvents")
        hit_constant = _field_constant(sound_type_source, args[5], "SoundEvents")
        break_sound = sound_events.get(break_constant)
        place_sound = sound_events.get(place_constant)
        hit_sound = sound_events.get(hit_constant)
        if break_sound is None:
            _fail(sound_type_source, args[2], f"unresolved SoundEvents.{break_constant}")
        if place_sound is None:
            _fail(sound_type_source, args[4], f"unresolved SoundEvents.{place_constant}")
        if hit_sound is None:
            _fail(sound_type_source, args[5], f"unresolved SoundEvents.{hit_constant}")
        declarations.append(
            {
                "constant": field.name,
                "declaration_order": declaration_order,
                "volume_expression": args[0].canonical,
                "pitch_expression": args[1].canonical,
                "break_sound": break_sound,
                "place_sound": place_sound,
                "hit_sound": hit_sound,
                "source_path": sound_type_source.relative_path,
                "start_byte": initializer.span.start_byte,
                "end_byte": initializer.span.end_byte,
            }
        )
    if not declarations or len(declarations) != len(fields):
        raise SoundTypeCatalogExtractionError(
            f"{sound_type_source.relative_path}: failed to close all SoundType declarations"
        )
    return ExtractedSoundTypeCatalog(tuple(declarations))
