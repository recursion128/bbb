"""Tree-sitter-backed extraction for vanilla entity renderer registrations."""

from __future__ import annotations

import struct
from dataclasses import dataclass

from vanilla_java_source import (
    JavaExpression,
    JavaMethod,
    JavaSource,
    JavaType,
    descendants,
    resolve_class_hierarchy,
    resolve_type_reference,
)


RENDERER_PACKAGE = "net.minecraft.client.renderer.entity"
ENTITY_RENDERERS_CLASS = RENDERER_PACKAGE + ".EntityRenderers"
THROWN_ITEM_RENDERER_CLASS = RENDERER_PACKAGE + ".ThrownItemRenderer"
ABSTRACT_HORSE_RENDERER_CLASS = RENDERER_PACKAGE + ".AbstractHorseRenderer"
MOB_RENDERER_CLASS = RENDERER_PACKAGE + ".MobRenderer"
ITEM_IN_HAND_LAYER_CLASS = RENDERER_PACKAGE + ".layers.ItemInHandLayer"
ABSTRACT_FELINE_MODEL_CLASS = (
    "net.minecraft.client.model.animal.feline.AbstractFelineModel"
)
SPECIAL_AVATAR_ENTITY_TYPES = frozenset({"MANNEQUIN", "PLAYER"})


class EntityRendererCatalogExtractionError(ValueError):
    """Explicit unsupported or inconsistent renderer source."""


@dataclass(frozen=True)
class EntityRendererRegistration:
    entity_type_id: int
    constant_name: str
    renderer_class: str
    static_full_block_light: bool
    thrown_item_scale_bits: int | None
    uses_abstract_feline_model: bool
    uses_abstract_horse_renderer: bool
    uses_mob_item_in_hand_layer: bool
    extracts_mob_aggressive: bool
    extracts_unhappy_counter: bool
    extracts_casting_spell: bool


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise EntityRendererCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _entity_type_constant(source: JavaSource, expression: JavaExpression) -> str:
    node = expression.node
    if node.type != "field_access":
        _fail(source, expression, "renderer registration type is not EntityType.CONSTANT")
    object_node = node.child_by_field_name("object")
    field_node = node.child_by_field_name("field")
    if (
        object_node is None
        or field_node is None
        or source.expression(object_node).canonical != "EntityType"
    ):
        _fail(source, expression, "renderer registration type is not EntityType.CONSTANT")
    return source.text(field_node)


def _provider_constructor(
    source: JavaSource, expression: JavaExpression
) -> tuple[str, tuple[JavaExpression, ...]]:
    node = expression.node
    if node.type == "method_reference":
        targets = node.named_children
        if len(targets) != 1:
            _fail(source, expression, "renderer constructor method reference is unsupported")
        return source.expression(targets[0]).canonical, ()
    if node.type != "lambda_expression":
        _fail(source, expression, "renderer provider is not a constructor reference or lambda")
    body = node.child_by_field_name("body")
    if body is None:
        _fail(source, expression, "renderer provider lambda lacks a body")
    if body.type != "object_creation_expression":
        _fail(source, expression, "renderer provider lambda is not one constructor call")
    creation = body
    type_node = creation.child_by_field_name("type")
    arguments_node = creation.child_by_field_name("arguments")
    if type_node is None or arguments_node is None:
        _fail(source, expression, "renderer provider constructor is incomplete")
    spelling = source.expression(type_node).canonical.split("<", 1)[0].strip()
    arguments = tuple(
        source.expression(argument) for argument in arguments_node.named_children
    )
    return spelling, arguments


def _method_return_expression(
    source: JavaSource, method: JavaMethod
) -> JavaExpression | None:
    body = method.node.child_by_field_name("body")
    if body is None or len(body.named_children) != 1:
        return None
    statement = body.named_children[0]
    if statement.type != "return_statement" or len(statement.named_children) != 1:
        return None
    return source.expression(statement.named_children[0])


def _returns_constant_full_block_light(source: JavaSource, method: JavaMethod) -> bool:
    expression = _method_return_expression(source, method)
    return (
        expression is not None
        and expression.node.type == "decimal_integer_literal"
        and expression.canonical == "15"
    )


def _float_literal_bits(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type != "decimal_floating_point_literal":
        _fail(source, expression, "ThrownItemRenderer scale is not a float literal")
    spelling = expression.canonical.replace("_", "").removesuffix("F").removesuffix("f")
    try:
        value = float(spelling)
        return struct.unpack("!I", struct.pack("!f", value))[0]
    except (OverflowError, ValueError, struct.error):
        _fail(source, expression, "invalid ThrownItemRenderer scale literal")


def _validate_thrown_item_constructor_contract(
    source_by_type: dict[str, JavaSource],
    type_by_name: dict[str, JavaType],
) -> None:
    renderer_type = type_by_name.get(THROWN_ITEM_RENDERER_CLASS)
    if renderer_type is None:
        raise EntityRendererCatalogExtractionError("missing ThrownItemRenderer class")
    source = source_by_type[renderer_type.fqcn]
    overrides = [
        method
        for method in source.methods_for(renderer_type)
        if method.name == "getBlockLightLevel" and len(method.parameters) == 2
    ]
    if len(overrides) != 1:
        raise EntityRendererCatalogExtractionError(
            "ThrownItemRenderer must define one getBlockLightLevel override"
        )
    expression = _method_return_expression(source, overrides[0])
    if expression is None or expression.node.type != "ternary_expression":
        _fail(
            source,
            source.expression(overrides[0].node),
            "ThrownItemRenderer block-light override is not one conditional return",
        )
    condition = expression.node.child_by_field_name("condition")
    consequence = expression.node.child_by_field_name("consequence")
    alternative = expression.node.child_by_field_name("alternative")
    if (
        condition is None
        or consequence is None
        or alternative is None
        or source.expression(condition).canonical != "this.fullBright"
        or source.expression(consequence).canonical != "15"
        or source.expression(alternative).canonical
        != "super.getBlockLightLevel (entity, blockPos)"
    ):
        _fail(
            source,
            expression,
            "unsupported ThrownItemRenderer full-bright expression",
        )
    constructors = [
        method
        for method in source.methods_for(renderer_type)
        if method.is_constructor and len(method.parameters) == 3
    ]
    if len(constructors) != 1 or [
        parameter.canonical for parameter in constructors[0].parameters
    ] != [
        "final EntityRendererProvider.Context context",
        "final float scale",
        "final boolean fullBright",
    ]:
        raise EntityRendererCatalogExtractionError(
            "unsupported ThrownItemRenderer full-bright constructor signature"
        )
    assignments = [
        source.expression(node).canonical
        for node in descendants(constructors[0].node)
        if node.type == "assignment_expression"
    ]
    if "this.scale = scale" not in assignments:
        raise EntityRendererCatalogExtractionError(
            "ThrownItemRenderer constructor does not retain scale"
        )
    if "this.fullBright = fullBright" not in assignments:
        raise EntityRendererCatalogExtractionError(
            "ThrownItemRenderer constructor does not retain fullBright"
        )
    default_constructors = [
        method
        for method in source.methods_for(renderer_type)
        if method.is_constructor and len(method.parameters) == 1
    ]
    if len(default_constructors) != 1:
        raise EntityRendererCatalogExtractionError(
            "ThrownItemRenderer must define one default constructor"
        )
    invocations = [
        source.expression(node).canonical
        for node in descendants(default_constructors[0].node)
        if node.type == "explicit_constructor_invocation"
    ]
    if invocations != ["this (context, 1.0F, false);"]:
        raise EntityRendererCatalogExtractionError(
            "unsupported ThrownItemRenderer default constructor"
        )


def _renderer_hierarchy_references_superclass_type(
    renderer_class: str,
    target_class: str,
    source_by_type: dict[str, JavaSource],
    type_by_name: dict[str, JavaType],
) -> bool:
    target_simple_name = target_class.rsplit(".", 1)[-1]
    current = type_by_name[renderer_class]
    while True:
        source = source_by_type[current.fqcn]
        superclass = current.node.child_by_field_name("superclass")
        if (
            superclass is not None
            and source.imports.get(target_simple_name) == target_class
            and any(
                source.text(node) == target_simple_name
                for node in (superclass, *descendants(superclass))
            )
        ):
            return True
        if current.superclass is None or current.superclass not in type_by_name:
            return False
        current = type_by_name[current.superclass]


def _renderer_inherits(
    renderer_class: str,
    target_class: str,
    type_by_name: dict[str, JavaType],
) -> bool:
    current = type_by_name[renderer_class]
    while True:
        if current.fqcn == target_class:
            return True
        if current.superclass is None or current.superclass not in type_by_name:
            return False
        current = type_by_name[current.superclass]


def _renderer_hierarchy_constructs_type(
    renderer_class: str,
    target_class: str,
    source_by_type: dict[str, JavaSource],
    type_by_name: dict[str, JavaType],
) -> bool:
    current = type_by_name[renderer_class]
    while True:
        source = source_by_type[current.fqcn]
        for method in source.methods_for(current):
            if not method.is_constructor:
                continue
            for node in descendants(method.node):
                if node.type != "object_creation_expression":
                    continue
                type_node = node.child_by_field_name("type")
                if type_node is None:
                    continue
                spelling = source.expression(type_node).canonical.split("<", 1)[0].strip()
                if resolve_type_reference(spelling, current, type_by_name) == target_class:
                    return True
        if current.superclass is None or current.superclass not in type_by_name:
            return False
        current = type_by_name[current.superclass]


def _renderer_hierarchy_contains_extract_assignment(
    renderer_class: str,
    expected: str,
    source_by_type: dict[str, JavaSource],
    type_by_name: dict[str, JavaType],
) -> bool:
    current = type_by_name[renderer_class]
    while True:
        source = source_by_type[current.fqcn]
        for method in source.methods_for(current):
            if method.name != "extractRenderState" or len(method.parameters) != 3:
                continue
            if any(
                node.type == "assignment_expression"
                and source.expression(node).canonical == expected
                for node in descendants(method.node)
            ):
                return True
        if current.superclass is None or current.superclass not in type_by_name:
            return False
        current = type_by_name[current.superclass]


def extract_entity_renderer_registrations(
    entity_renderers_source: JavaSource,
    renderer_sources: list[JavaSource],
    registry_constants: list[str],
) -> list[EntityRendererRegistration]:
    """Extract explicit provider registrations and static full-block-light behavior."""

    all_sources = [entity_renderers_source, *renderer_sources]
    all_types = [java_type for source in all_sources for java_type in source.types]
    resolve_class_hierarchy(all_types)
    type_by_name = {java_type.fqcn: java_type for java_type in all_types}
    source_by_type = {
        java_type.fqcn: source
        for source in all_sources
        for java_type in source.types
    }
    owners = [
        java_type
        for java_type in entity_renderers_source.types
        if java_type.fqcn == ENTITY_RENDERERS_CLASS
    ]
    if len(owners) != 1:
        raise EntityRendererCatalogExtractionError(
            f"{entity_renderers_source.relative_path}: expected one EntityRenderers class"
        )
    owner = owners[0]
    known = type_by_name
    registrations = []
    seen_constants: set[str] = set()
    registry_ids = {name: type_id for type_id, name in enumerate(registry_constants)}
    if len(registry_ids) != len(registry_constants):
        raise EntityRendererCatalogExtractionError(
            "entity type registry contains duplicate constants"
        )

    _validate_thrown_item_constructor_contract(source_by_type, type_by_name)

    calls = [
        call
        for call in entity_renderers_source.calls_for(owner)
        if call.name == "register"
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    for call in calls:
        if len(call.arguments) != 2:
            _fail(
                entity_renderers_source,
                entity_renderers_source.expression(call.node),
                "renderer register call must have two arguments",
            )
        constant_name = _entity_type_constant(
            entity_renderers_source, call.arguments[0]
        )
        if constant_name not in registry_ids:
            _fail(
                entity_renderers_source,
                call.arguments[0],
                f"renderer registration references unknown entity type {constant_name}",
            )
        if constant_name in seen_constants:
            _fail(
                entity_renderers_source,
                call.arguments[0],
                f"duplicate renderer registration for {constant_name}",
            )
        seen_constants.add(constant_name)
        entity_type_id = registry_ids[constant_name]

        renderer_spelling, constructor_arguments = _provider_constructor(
            entity_renderers_source, call.arguments[1]
        )
        renderer_class = resolve_type_reference(renderer_spelling, owner, known)
        if renderer_class is None:
            _fail(
                entity_renderers_source,
                call.arguments[1],
                f"unresolved renderer class {renderer_spelling}",
            )
        current = type_by_name[renderer_class]
        static_full_block_light = False
        while True:
            source = source_by_type[current.fqcn]
            overrides = [
                method
                for method in source.methods_for(current)
                if method.name == "getBlockLightLevel" and len(method.parameters) == 2
            ]
            if len(overrides) > 1:
                raise EntityRendererCatalogExtractionError(
                    f"{current.source_path}: multiple getBlockLightLevel overrides"
                )
            if overrides:
                static_full_block_light = _returns_constant_full_block_light(
                    source, overrides[0]
                )
                break
            if current.superclass is None or current.superclass not in type_by_name:
                break
            current = type_by_name[current.superclass]

        thrown_item_scale_bits = None
        if renderer_class == THROWN_ITEM_RENDERER_CLASS:
            if not constructor_arguments:
                static_full_block_light = False
                thrown_item_scale_bits = 0x3F800000
            elif (
                len(constructor_arguments) == 3
                and constructor_arguments[2].node.type in {"true", "false"}
            ):
                thrown_item_scale_bits = _float_literal_bits(
                    entity_renderers_source, constructor_arguments[1]
                )
                static_full_block_light = constructor_arguments[2].node.type == "true"
            else:
                _fail(
                    entity_renderers_source,
                    call.arguments[1],
                    "unsupported ThrownItemRenderer constructor arguments",
                )

        registrations.append(
            EntityRendererRegistration(
                entity_type_id,
                constant_name,
                renderer_class.removeprefix(RENDERER_PACKAGE + "."),
                static_full_block_light,
                thrown_item_scale_bits,
                _renderer_hierarchy_references_superclass_type(
                    renderer_class,
                    ABSTRACT_FELINE_MODEL_CLASS,
                    source_by_type,
                    type_by_name,
                ),
                _renderer_inherits(
                    renderer_class,
                    ABSTRACT_HORSE_RENDERER_CLASS,
                    type_by_name,
                ),
                _renderer_inherits(
                    renderer_class,
                    MOB_RENDERER_CLASS,
                    type_by_name,
                )
                and _renderer_hierarchy_constructs_type(
                    renderer_class,
                    ITEM_IN_HAND_LAYER_CLASS,
                    source_by_type,
                    type_by_name,
                ),
                _renderer_hierarchy_contains_extract_assignment(
                    renderer_class,
                    "state.isAggressive = entity.isAggressive ()",
                    source_by_type,
                    type_by_name,
                ),
                _renderer_hierarchy_contains_extract_assignment(
                    renderer_class,
                    "state.isUnhappy = entity.getUnhappyCounter ()> 0",
                    source_by_type,
                    type_by_name,
                ),
                _renderer_hierarchy_contains_extract_assignment(
                    renderer_class,
                    "state.isCastingSpell = entity.isCastingSpell ()",
                    source_by_type,
                    type_by_name,
                ),
            )
        )

    unregistered = set(registry_constants) - seen_constants
    if unregistered != SPECIAL_AVATAR_ENTITY_TYPES:
        raise EntityRendererCatalogExtractionError(
            "entity renderer registration coverage mismatch: "
            f"unregistered={sorted(unregistered)}, "
            f"expected_avatar_path={sorted(SPECIAL_AVATAR_ENTITY_TYPES)}"
        )
    registrations.sort(key=lambda entry: entry.entity_type_id)
    return registrations
