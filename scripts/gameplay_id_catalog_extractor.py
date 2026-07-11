"""Tree-sitter-backed domain extraction for vanilla gameplay numeric ids."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaField, JavaSource, descendants, type_body_members


class GameplayIdExtractionError(ValueError):
    pass


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


@dataclass(frozen=True)
class MenuTypeEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class EquipmentSlotEntry:
    constant_name: str
    slot_type: str
    index: int
    count_limit: int
    slot_id: int
    serialized_name: str


@dataclass(frozen=True)
class EquipmentSlotGroupEntry:
    constant_name: str
    group_id: int
    serialized_name: str


@dataclass(frozen=True)
class EntityPoseEntry:
    constant_name: str
    pose_id: int
    serialized_name: str


@dataclass(frozen=True)
class EntityDataSerializerEntry:
    constant_name: str


@dataclass(frozen=True)
class ClientGameEventTypeEntry:
    constant_name: str
    event_id: int


@dataclass(frozen=True)
class SoundSourceEntry:
    constant_name: str
    serialized_name: str


@dataclass(frozen=True)
class HeightmapTypeEntry:
    constant_name: str
    type_id: int
    serialization_key: str
    usage: str


@dataclass(frozen=True)
class GameTypeEntry:
    constant_name: str
    game_type_id: int
    serialized_name: str


@dataclass(frozen=True)
class PlayerInfoActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class DifficultyEntry:
    constant_name: str
    difficulty_id: int
    serialized_name: str


@dataclass(frozen=True)
class ResourcePackResponseActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class CustomChatCompletionsActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class ClientCommandActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class PlayerActionKindEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class PlayerCommandActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class EntityAnchorEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class RemoteDebugSampleTypeEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class ObjectiveRenderTypeEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class AdvancementFrameTypeEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class BossBarStyleEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class ScoreboardDisplaySlotEntry:
    constant_name: str
    slot_id: int
    serialized_name: str


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise GameplayIdExtractionError(
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
    if not isinstance(value, str):
        _fail(source, expression, "expected string literal")
    return value


def _integer(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type not in {"decimal_integer_literal", "hex_integer_literal"}:
        _fail(source, expression, "expected integer literal")
    try:
        return int(expression.canonical.replace("_", ""), 0)
    except ValueError:
        _fail(source, expression, "invalid integer literal")


def _fields(source: JavaSource, prefix: str) -> list[JavaField]:
    return [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith(prefix)
    ]


def _register_arguments(source: JavaSource, field: JavaField) -> tuple[JavaExpression, ...]:
    initializer = field.initializer
    if initializer is None or initializer.node.type != "method_invocation":
        expression = initializer or JavaExpression("", field.span, field.node)
        _fail(source, expression, f"{field.name} lacks a register call")
    name = initializer.node.child_by_field_name("name")
    arguments = initializer.node.child_by_field_name("arguments")
    if name is None or arguments is None or source.text(name) != "register":
        _fail(source, initializer, f"{field.name} is not initialized by register")
    return tuple(source.expression(node) for node in arguments.named_children)


def _calls(source: JavaSource, expression: JavaExpression, name: str) -> list:
    return [
        call
        for call in source.calls
        if call.name == name
        and expression.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= expression.span.end_byte
    ]


def _object_arguments(source: JavaSource, expression: JavaExpression) -> tuple[JavaExpression, ...]:
    candidates = [
        node
        for node in [expression.node, *descendants(expression.node)]
        if node.type == "object_creation_expression"
    ]
    if len(candidates) != 1:
        _fail(source, expression, f"expected one object creation, found {len(candidates)}")
    arguments = candidates[0].child_by_field_name("arguments")
    if arguments is None:
        _fail(source, expression, "object creation lacks arguments")
    return tuple(source.expression(node) for node in arguments.named_children)


def _validate(entries: list, description: str, resource_attr: str | None = None) -> None:
    constants = [entry.constant_name for entry in entries]
    if len(constants) != len(set(constants)):
        raise GameplayIdExtractionError(f"duplicate {description} constant")
    if resource_attr is not None:
        values = [getattr(entry, resource_attr) for entry in entries]
        if len(values) != len(set(values)):
            raise GameplayIdExtractionError(f"duplicate {description} {resource_attr}")


def attribute_entries(source: JavaSource, expected_count: int = 35) -> list[AttributeEntry]:
    entries: list[AttributeEntry] = []
    for field in _fields(source, "Holder<Attribute>"):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, "attribute register must have two arguments")
        constructor = _object_arguments(source, arguments[1])
        if len(constructor) < 1:
            _fail(source, arguments[1], "attribute constructor lacks description key")
        sentiment_calls = _calls(source, field.initializer, "setSentiment")
        if len(sentiment_calls) > 1 or (sentiment_calls and len(sentiment_calls[0].arguments) != 1):
            _fail(source, field.initializer, "ambiguous attribute sentiment")
        sentiment = "POSITIVE"
        if sentiment_calls:
            sentiment = sentiment_calls[0].arguments[0].canonical.rsplit(".", 1)[-1]
            if sentiment not in {"POSITIVE", "NEUTRAL", "NEGATIVE"}:
                _fail(source, sentiment_calls[0].arguments[0], "unknown attribute sentiment")
        sync_calls = _calls(source, field.initializer, "setSyncable")
        if len(sync_calls) > 1 or any(
            len(call.arguments) != 1 or call.arguments[0].canonical != "true"
            for call in sync_calls
        ):
            _fail(source, field.initializer, "unsupported attribute syncable expression")
        entries.append(
            AttributeEntry(
                field.name,
                "minecraft:" + _string(source, arguments[0]),
                _string(source, constructor[0]),
                sentiment,
                bool(sync_calls),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} attributes, extracted {len(entries)}"
        )
    _validate(entries, "attribute", "resource_id")
    return entries


def mob_effect_entries(source: JavaSource, expected_count: int = 40) -> list[MobEffectEntry]:
    entries: list[MobEffectEntry] = []
    for field in _fields(source, "Holder<MobEffect>"):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, "mob effect register must have two arguments")
        categories = [
            source.expression(node).canonical.rsplit(".", 1)[-1]
            for node in descendants(arguments[1].node)
            if node.type in {"field_access", "scoped_identifier"}
            and source.expression(node).canonical.startswith("MobEffectCategory.")
        ]
        if len(categories) != 1 or categories[0] not in {"BENEFICIAL", "HARMFUL", "NEUTRAL"}:
            _fail(source, arguments[1], f"expected one mob effect category, found {categories}")
        entries.append(
            MobEffectEntry(
                field.name, "minecraft:" + _string(source, arguments[0]), categories[0]
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} mob effects, extracted {len(entries)}"
        )
    _validate(entries, "mob effect", "resource_id")
    return entries


def menu_type_entries(source: JavaSource, expected_count: int = 25) -> list[MenuTypeEntry]:
    entries = []
    for field in _fields(source, "MenuType<"):
        arguments = _register_arguments(source, field)
        if len(arguments) < 2:
            _fail(source, field.initializer, "menu register lacks arguments")
        entries.append(MenuTypeEntry(field.name, "minecraft:" + _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} menu types, extracted {len(entries)}"
        )
    _validate(entries, "menu type", "resource_id")
    return entries


def _enum_constants(source: JavaSource) -> list[tuple[str, tuple[JavaExpression, ...]]]:
    enums = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.node.parent.type == "program"
    ]
    if len(enums) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one enum, found {len(enums)}"
        )
    result = []
    for node in type_body_members(enums[0].body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None:
            _fail(source, source.expression(node), "enum constant is incomplete")
        result.append(
            (
                source.text(name),
                tuple(source.expression(child) for child in arguments.named_children),
            )
        )
    return result


def heightmap_type_entries(
    source: JavaSource, expected_count: int = 6
) -> list[HeightmapTypeEntry]:
    enums = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == "Types"
    ]
    if len(enums) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one Heightmap.Types enum, found {len(enums)}"
        )
    entries = []
    for node in type_body_members(enums[0].body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None or len(arguments.named_children) != 4:
            _fail(source, source.expression(node), "heightmap type must have four arguments")
        values = tuple(source.expression(child) for child in arguments.named_children)
        usage = values[2].canonical.rsplit(".", 1)[-1]
        if usage not in {"WORLDGEN", "LIVE_WORLD", "CLIENT"}:
            _fail(source, values[2], f"unknown heightmap usage {usage}")
        entries.append(
            HeightmapTypeEntry(
                source.text(name),
                _integer(source, values[0]),
                _string(source, values[1]),
                usage,
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} heightmap types, extracted {len(entries)}"
        )
    _validate(entries, "heightmap type", "type_id")
    _validate(entries, "heightmap type", "serialization_key")
    if [entry.type_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("heightmap type ids are not contiguous declaration order")
    return entries


def game_type_entries(
    source: JavaSource, expected_count: int = 4
) -> list[GameTypeEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            _fail(source, arguments[0], f"game type {name} must have two arguments")
        entries.append(
            GameTypeEntry(
                name,
                _integer(source, arguments[0]),
                _string(source, arguments[1]),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} game types, extracted {len(entries)}"
        )
    _validate(entries, "game type", "game_type_id")
    _validate(entries, "game type", "serialized_name")
    if [entry.game_type_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("game type ids are not contiguous declaration order")
    return entries


def game_type_not_set_id(source: JavaSource) -> int:
    fields = [field for field in source.fields if field.name == "NOT_SET"]
    if len(fields) != 1 or fields[0].initializer is None:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one initialized NOT_SET field"
        )
    initializer = fields[0].initializer
    if initializer.canonical.replace(" ", "") != "-1":
        _fail(source, initializer, "game type NOT_SET must be -1")
    return int(initializer.canonical.replace(" ", ""), 0)


def _nested_enum_constants(
    source: JavaSource, enum_name: str, description: str
) -> list[tuple[str, tuple[JavaExpression, ...]]]:
    enums = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == enum_name
    ]
    if len(enums) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one nested {enum_name} enum, found {len(enums)}"
        )
    entries: list[tuple[str, tuple[JavaExpression, ...]]] = []
    for node in type_body_members(enums[0].body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        if name is None:
            _fail(source, source.expression(node), f"{description} lacks a name")
        arguments = node.child_by_field_name("arguments")
        entries.append(
            (
                source.text(name),
                tuple(
                    source.expression(child)
                    for child in (() if arguments is None else arguments.named_children)
                ),
            )
        )
    return entries


def _nested_enum_ordinals(
    source: JavaSource, enum_name: str, description: str, expected_count: int
) -> list[tuple[str, int]]:
    constants = _nested_enum_constants(source, enum_name, description)
    entries = [(name, ordinal) for ordinal, (name, _) in enumerate(constants)]
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} {description}s, extracted {len(entries)}"
        )
    return entries


def player_info_action_entries(
    source: JavaSource, expected_count: int = 8
) -> list[PlayerInfoActionEntry]:
    entries = [
        PlayerInfoActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "player-info action", expected_count
        )
    ]
    _validate(entries, "player-info action", "ordinal")
    return entries


def resource_pack_response_action_entries(
    source: JavaSource, expected_count: int = 8
) -> list[ResourcePackResponseActionEntry]:
    entries = [
        ResourcePackResponseActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "resource-pack response action", expected_count
        )
    ]
    _validate(entries, "resource-pack response action", "ordinal")
    return entries


def custom_chat_completions_action_entries(
    source: JavaSource, expected_count: int = 3
) -> list[CustomChatCompletionsActionEntry]:
    entries = [
        CustomChatCompletionsActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "custom-chat completions action", expected_count
        )
    ]
    _validate(entries, "custom-chat completions action", "ordinal")
    return entries


def client_command_action_entries(
    source: JavaSource, expected_count: int = 3
) -> list[ClientCommandActionEntry]:
    entries = [
        ClientCommandActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "client command action", expected_count
        )
    ]
    _validate(entries, "client command action", "ordinal")
    return entries


def player_action_kind_entries(
    source: JavaSource, expected_count: int = 8
) -> list[PlayerActionKindEntry]:
    entries = [
        PlayerActionKindEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "player action kind", expected_count
        )
    ]
    _validate(entries, "player action kind", "ordinal")
    return entries


def player_command_action_entries(
    source: JavaSource, expected_count: int = 7
) -> list[PlayerCommandActionEntry]:
    entries = [
        PlayerCommandActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "player command action", expected_count
        )
    ]
    _validate(entries, "player command action", "ordinal")
    return entries


def entity_anchor_entries(
    source: JavaSource, expected_count: int = 2
) -> list[EntityAnchorEntry]:
    entries = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(source, "Anchor", "entity anchor")
    ):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"entity anchor {name} must have two arguments")
        entries.append(EntityAnchorEntry(name, ordinal, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} entity anchors, extracted {len(entries)}"
        )
    _validate(entries, "entity anchor", "ordinal")
    _validate(entries, "entity anchor", "serialized_name")
    return entries


def remote_debug_sample_type_entries(
    source: JavaSource, expected_count: int = 1
) -> list[RemoteDebugSampleTypeEntry]:
    entries = [
        RemoteDebugSampleTypeEntry(name, ordinal)
        for ordinal, (name, _) in enumerate(_enum_constants(source))
    ]
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} remote debug sample types, extracted {len(entries)}"
        )
    _validate(entries, "remote debug sample type", "ordinal")
    return entries


def objective_render_type_entries(
    source: JavaSource, expected_count: int = 2
) -> list[ObjectiveRenderTypeEntry]:
    entries = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(source, "RenderType", "objective render type")
    ):
        if len(arguments) != 1:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"objective render type {name} must have one argument")
        entries.append(ObjectiveRenderTypeEntry(name, ordinal, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} objective render types, extracted {len(entries)}"
        )
    _validate(entries, "objective render type", "ordinal")
    _validate(entries, "objective render type", "serialized_name")
    return entries


def advancement_frame_type_entries(
    source: JavaSource, expected_count: int = 3
) -> list[AdvancementFrameTypeEntry]:
    entries = []
    for ordinal, (name, arguments) in enumerate(_enum_constants(source)):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"advancement frame type {name} must have two arguments")
        entries.append(AdvancementFrameTypeEntry(name, ordinal, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} advancement frame types, extracted {len(entries)}"
        )
    _validate(entries, "advancement frame type", "ordinal")
    _validate(entries, "advancement frame type", "serialized_name")
    return entries


def boss_bar_style_entries(
    source: JavaSource,
) -> tuple[list[BossBarStyleEntry], list[BossBarStyleEntry]]:
    def extract(enum_name: str, expected_count: int) -> list[BossBarStyleEntry]:
        entries = []
        for ordinal, (name, arguments) in enumerate(
            _nested_enum_constants(source, enum_name, enum_name)
        ):
            if not arguments:
                _fail(source, source.expression(source.types[0].node), f"{enum_name} {name} lacks name")
            entries.append(BossBarStyleEntry(name, ordinal, _string(source, arguments[0])))
        if len(entries) != expected_count:
            raise GameplayIdExtractionError(
                f"expected {expected_count} {enum_name} values, extracted {len(entries)}"
            )
        _validate(entries, enum_name, "ordinal")
        _validate(entries, enum_name, "serialized_name")
        return entries

    return extract("BossBarColor", 7), extract("BossBarOverlay", 5)


def difficulty_entries(
    source: JavaSource, expected_count: int = 4
) -> list[DifficultyEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            _fail(source, arguments[0], f"difficulty {name} must have two arguments")
        entries.append(
            DifficultyEntry(
                name,
                _integer(source, arguments[0]),
                _string(source, arguments[1]),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} difficulties, extracted {len(entries)}"
        )
    _validate(entries, "difficulty", "difficulty_id")
    _validate(entries, "difficulty", "serialized_name")
    if [entry.difficulty_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("difficulty ids are not contiguous declaration order")
    return entries


def _continuous_out_of_bounds_strategy(source: JavaSource, description: str) -> str:
    fields = [field for field in source.fields if field.name == "BY_ID"]
    if len(fields) != 1 or fields[0].initializer is None:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one initialized {description} BY_ID field"
        )
    initializer = fields[0].initializer
    if initializer.node.type != "method_invocation":
        _fail(source, initializer, f"{description} BY_ID must be a method invocation")
    name = initializer.node.child_by_field_name("name")
    arguments = initializer.node.child_by_field_name("arguments")
    if (
        name is None
        or source.text(name) != "continuous"
        or arguments is None
        or len(arguments.named_children) != 3
    ):
        _fail(source, initializer, f"{description} BY_ID must call ByIdMap.continuous")
    strategy = source.expression(arguments.named_children[-1]).canonical.rsplit(".", 1)[-1]
    return strategy


def difficulty_out_of_bounds_strategy(source: JavaSource) -> str:
    strategy = _continuous_out_of_bounds_strategy(source, "difficulty")
    if strategy != "WRAP":
        raise GameplayIdExtractionError(f"unsupported difficulty id strategy {strategy}")
    return strategy


def scoreboard_display_slot_entries(
    source: JavaSource, expected_count: int = 19
) -> tuple[list[ScoreboardDisplaySlotEntry], str]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"display slot {name} must have two arguments")
        entries.append(
            ScoreboardDisplaySlotEntry(
                name, _integer(source, arguments[0]), _string(source, arguments[1])
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} display slots, extracted {len(entries)}"
        )
    _validate(entries, "display slot", "slot_id")
    _validate(entries, "display slot", "serialized_name")
    if [entry.slot_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("display slot ids are not contiguous declaration order")
    strategy = _continuous_out_of_bounds_strategy(source, "display slot")
    if strategy != "ZERO":
        raise GameplayIdExtractionError(f"unsupported display slot id strategy {strategy}")
    return entries, strategy


def equipment_slot_entries(
    source: JavaSource, expected_count: int = 8
) -> list[EquipmentSlotEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) not in {4, 5}:
            _fail(source, arguments[0], f"equipment slot {name} has {len(arguments)} arguments")
        slot_type = arguments[0].canonical.rsplit(".", 1)[-1]
        numeric = [_integer(source, argument) for argument in arguments[1:-1]]
        entries.append(
            EquipmentSlotEntry(
                name,
                slot_type,
                numeric[0],
                numeric[1] if len(numeric) == 3 else 0,
                numeric[2] if len(numeric) == 3 else numeric[1],
                _string(source, arguments[-1]),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} equipment slots, extracted {len(entries)}"
        )
    _validate(entries, "equipment slot", "slot_id")
    _validate(entries, "equipment slot", "serialized_name")
    return entries


def equipment_slot_group_entries(
    source: JavaSource, expected_count: int = 11
) -> list[EquipmentSlotGroupEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) < 3:
            _fail(source, arguments[0], f"equipment slot group {name} lacks arguments")
        entries.append(
            EquipmentSlotGroupEntry(
                name, _integer(source, arguments[0]), _string(source, arguments[1])
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} equipment slot groups, extracted {len(entries)}"
        )
    _validate(entries, "equipment slot group", "group_id")
    _validate(entries, "equipment slot group", "serialized_name")
    return entries


def entity_pose_entries(source: JavaSource, expected_count: int = 18) -> list[EntityPoseEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            _fail(source, arguments[0], f"entity pose {name} must have two arguments")
        entries.append(
            EntityPoseEntry(name, _integer(source, arguments[0]), _string(source, arguments[1]))
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} entity poses, extracted {len(entries)}"
        )
    _validate(entries, "entity pose", "pose_id")
    _validate(entries, "entity pose", "serialized_name")
    return entries


def entity_data_serializer_entries(
    source: JavaSource, expected_count: int = 43
) -> list[EntityDataSerializerEntry]:
    declarations = {field.name for field in _fields(source, "EntityDataSerializer")}
    calls = [call for call in source.calls if call.name == "registerSerializer"]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries = []
    for call in calls:
        if len(call.arguments) != 1 or call.arguments[0].node.type != "identifier":
            _fail(source, source.expression(call.node), "unsupported serializer registration")
        entries.append(EntityDataSerializerEntry(call.arguments[0].canonical))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} entity-data serializers, extracted {len(entries)}"
        )
    _validate(entries, "entity-data serializer")
    missing = [entry.constant_name for entry in entries if entry.constant_name not in declarations]
    if missing:
        raise GameplayIdExtractionError(f"undeclared entity-data serializers: {missing}")
    return entries


def client_game_event_type_entries(
    source: JavaSource, expected_count: int = 14
) -> list[ClientGameEventTypeEntry]:
    entries = []
    for field in _fields(source, "ClientboundGameEventPacket.Type"):
        if not field.name.isupper():
            continue
        if field.initializer is None:
            _fail(source, JavaExpression("", field.span, field.node), "game event type lacks initializer")
        arguments = _object_arguments(source, field.initializer)
        if len(arguments) != 1:
            _fail(source, field.initializer, "game event type constructor must have one argument")
        entries.append(ClientGameEventTypeEntry(field.name, _integer(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} game-event types, extracted {len(entries)}"
        )
    _validate(entries, "client game-event type", "event_id")
    if [entry.event_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("client game-event ids are not contiguous")
    return entries


def sound_source_entries(source: JavaSource, expected_count: int = 11) -> list[SoundSourceEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 1:
            _fail(source, arguments[0], f"sound source {name} must have one argument")
        entries.append(SoundSourceEntry(name, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} sound sources, extracted {len(entries)}"
        )
    _validate(entries, "sound source", "serialized_name")
    return entries
