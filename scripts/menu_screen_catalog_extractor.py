"""Tree-sitter-backed extraction of Vanilla ``MenuScreens`` registrations."""

from __future__ import annotations

from dataclasses import dataclass

from gameplay_id_catalog_extractor import MenuTypeEntry
from vanilla_java_source import JavaExpression, JavaSource


class MenuScreenCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class MenuScreenRegistration:
    menu_id: int
    menu_constant: str
    resource_id: str
    screen_class: str


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise MenuScreenCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _menu_constant(source: JavaSource, expression: JavaExpression) -> str:
    node = expression.node
    if node.type != "field_access":
        _fail(source, expression, "menu registration must reference MenuType.CONSTANT")
    object_node = node.child_by_field_name("object")
    field_node = node.child_by_field_name("field")
    if (
        object_node is None
        or field_node is None
        or source.expression(object_node).canonical != "MenuType"
    ):
        _fail(source, expression, "menu registration must reference MenuType.CONSTANT")
    return source.text(field_node)


def _screen_class(source: JavaSource, expression: JavaExpression) -> str:
    node = expression.node
    if node.type != "method_reference" or not expression.canonical.endswith("::new"):
        _fail(source, expression, "screen factory must be a constructor method reference")
    targets = node.named_children
    if len(targets) != 1:
        _fail(source, expression, "screen constructor method reference is ambiguous")
    screen_class = source.expression(targets[0]).canonical
    imported = source.imports.get(screen_class)
    if imported is None or not imported.startswith(
        "net.minecraft.client.gui.screens.inventory."
    ):
        _fail(source, expression, f"screen class {screen_class!r} is not an inventory screen")
    return screen_class


def menu_screen_registrations(
    source: JavaSource,
    menu_types: list[MenuTypeEntry],
    expected_count: int = 25,
) -> list[MenuScreenRegistration]:
    owners = [
        java_type
        for java_type in source.types
        if java_type.simple_name == "MenuScreens"
        and java_type.node.parent.type == "program"
    ]
    if len(owners) != 1:
        raise MenuScreenCatalogExtractionError(
            f"{source.relative_path}: expected one top-level MenuScreens class, "
            f"found {len(owners)}"
        )

    raw: list[tuple[str, str, JavaExpression]] = []
    for call in source.calls_for(owners[0]):
        if call.name != "register":
            continue
        if call.qualifier is not None or len(call.arguments) != 2:
            anchor = call.arguments[0] if call.arguments else source.expression(call.node)
            _fail(source, anchor, "MenuScreens.register must have two unqualified arguments")
        raw.append(
            (
                _menu_constant(source, call.arguments[0]),
                _screen_class(source, call.arguments[1]),
                call.arguments[0],
            )
        )

    if len(raw) != expected_count:
        raise MenuScreenCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} MenuScreens registrations, "
            f"extracted {len(raw)}"
        )
    constants = [constant for constant, _, _ in raw]
    if len(constants) != len(set(constants)):
        duplicate = next(
            constant for constant in constants if constants.count(constant) > 1
        )
        expression = next(item[2] for item in raw if item[0] == duplicate)
        _fail(source, expression, f"duplicate menu screen registration {duplicate}")

    registry_constants = [entry.constant_name for entry in menu_types]
    if constants != registry_constants:
        missing = sorted(set(registry_constants) - set(constants))
        extra = sorted(set(constants) - set(registry_constants))
        raise MenuScreenCatalogExtractionError(
            f"{source.relative_path}: MenuScreens order/coverage differs from MenuType registry; "
            f"missing={missing}, extra={extra}, registrations={constants}, "
            f"registry={registry_constants}"
        )

    return [
        MenuScreenRegistration(index, constant, menu_types[index].resource_id, screen)
        for index, (constant, screen, _) in enumerate(raw)
    ]
