"""Narrow, semantic-neutral Tree-sitter Java frontend for vanilla extractors.

This module exposes only the CST facts currently needed by source-derived
catalogs.  It does not understand Minecraft concepts.  Tree-sitter owns Java
lexing, delimiter recovery, and declaration parsing; consumers must not add a
parallel text parser or silently recover from malformed CSTs.

Suitable derived facts include declaration/registration order, identifiers,
class inheritance, enum ordinals, constant expressions, raw defaults, and pure
builder properties.  Wire codecs, state transitions, ticks, physics, animation,
rendering, persistent Codec/SNBT semantics, and runtime fallback remain in the
corresponding Rust semantic owner.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Iterator, Sequence

from tree_sitter import Language, Node, Parser, Tree
import tree_sitter_java


JAVA_LANGUAGE = Language(tree_sitter_java.language())
TYPE_NODE_KINDS = {
    "class_declaration": "class",
    "interface_declaration": "interface",
    "enum_declaration": "enum",
    "record_declaration": "record",
    "annotation_type_declaration": "annotation",
}
COMMENT_NODE_KINDS = {"line_comment", "block_comment"}


class JavaSourceError(ValueError):
    """Explicit malformed/unsupported Java source diagnostic."""


@dataclass(frozen=True)
class SourceSpan:
    relative_path: str
    start_byte: int
    end_byte: int
    start_point: tuple[int, int]
    end_point: tuple[int, int]

    def bytes(self, source: bytes) -> bytes:
        return source[self.start_byte : self.end_byte]


@dataclass(frozen=True)
class JavaExpression:
    canonical: str
    span: SourceSpan
    node: Node = field(compare=False, repr=False)


@dataclass
class JavaType:
    fqcn: str
    simple_name: str
    source_path: str
    package: str
    kind: str
    superclass_spelling: str | None
    superclass: str | None
    interface_spellings: tuple[str, ...]
    interfaces: tuple[str, ...]
    declaration_order: int
    span: SourceSpan
    node: Node = field(repr=False)
    body_node: Node = field(repr=False)
    imports: dict[str, str] = field(repr=False)


@dataclass(frozen=True)
class JavaField:
    owner: str
    name: str
    type_spelling: str
    modifiers: tuple[str, ...]
    initializer: JavaExpression | None
    declaration_order: int
    span: SourceSpan
    node: Node = field(compare=False, repr=False)


@dataclass(frozen=True)
class JavaMethod:
    owner: str
    name: str
    is_constructor: bool
    parameters: tuple[JavaExpression, ...]
    body: SourceSpan | None
    declaration_order: int
    span: SourceSpan
    node: Node = field(compare=False, repr=False)


@dataclass(frozen=True)
class JavaCall:
    owner: str | None
    qualifier: JavaExpression | None
    name: str
    arguments: tuple[JavaExpression, ...]
    span: SourceSpan
    node: Node = field(compare=False, repr=False)


@dataclass
class JavaSource:
    relative_path: str
    source: bytes
    tree: Tree = field(repr=False)
    package: str
    imports: dict[str, str]
    types: list[JavaType]
    fields: list[JavaField]
    methods: list[JavaMethod]
    calls: list[JavaCall]

    @property
    def root_node(self) -> Node:
        return self.tree.root_node

    def text(self, node: Node) -> str:
        return self.source[node.start_byte : node.end_byte].decode("utf-8")

    def span(self, node: Node) -> SourceSpan:
        return _span(self.relative_path, node)

    def expression(self, node: Node) -> JavaExpression:
        return JavaExpression(canonical_expression(self.source, node), self.span(node), node)

    def fields_for(self, owner: JavaType | str) -> list[JavaField]:
        fqcn = owner.fqcn if isinstance(owner, JavaType) else owner
        return [item for item in self.fields if item.owner == fqcn]

    def methods_for(self, owner: JavaType | str) -> list[JavaMethod]:
        fqcn = owner.fqcn if isinstance(owner, JavaType) else owner
        return [item for item in self.methods if item.owner == fqcn]

    def calls_for(self, owner: JavaType | str) -> list[JavaCall]:
        fqcn = owner.fqcn if isinstance(owner, JavaType) else owner
        return [item for item in self.calls if item.owner == fqcn]


def _span(relative_path: str, node: Node) -> SourceSpan:
    return SourceSpan(
        relative_path,
        node.start_byte,
        node.end_byte,
        (node.start_point.row, node.start_point.column),
        (node.end_point.row, node.end_point.column),
    )


def descendants(node: Node, *, named_only: bool = True) -> Iterator[Node]:
    """Yield descendants in stable CST preorder."""

    stack = list(reversed(node.named_children if named_only else node.children))
    while stack:
        current = stack.pop()
        yield current
        children = current.named_children if named_only else current.children
        stack.extend(reversed(children))


def direct_children_of_type(node: Node, kinds: set[str]) -> Iterator[Node]:
    yield from (child for child in node.named_children if child.type in kinds)


def type_body_members(node: Node) -> Iterator[Node]:
    """Yield declarations owned directly by a type, including Java enum wrappers."""

    for child in node.named_children:
        if child.type == "enum_body_declarations":
            yield from child.named_children
        else:
            yield child


def _first_invalid_node(root: Node) -> Node | None:
    if root.type == "ERROR" or root.is_missing:
        return root
    for node in descendants(root, named_only=False):
        if node.type == "ERROR" or node.is_missing:
            return node
    return None


def _diagnose_tree(relative_path: str, root: Node) -> None:
    invalid = _first_invalid_node(root)
    if invalid is None:
        return
    kind = "MISSING" if invalid.is_missing else invalid.type
    start = invalid.start_point
    end = invalid.end_point
    raise JavaSourceError(
        f"{relative_path}:{start.row + 1}:{start.column + 1}-"
        f"{end.row + 1}:{end.column + 1}: Tree-sitter Java {kind} node"
    )


def _leaf_nodes(node: Node) -> Iterator[Node]:
    if node.type in {"string_literal", "character_literal", "text_block"}:
        yield node
        return
    if not node.children:
        yield node
        return
    for child in node.children:
        if child.type not in COMMENT_NODE_KINDS:
            yield from _leaf_nodes(child)


def canonical_expression(source: bytes, node: Node) -> str:
    """Render a deterministic expression from CST leaves and raw byte slices."""

    leaves = list(_leaf_nodes(node))
    if not leaves:
        return ""
    values = [source[item.start_byte : item.end_byte].decode("utf-8") for item in leaves]
    result = values[0]
    no_space_before = {".", ",", ";", ")", "]", "}", "::", ">"}
    no_space_after = {".", "(", "[", "{", "::", "@", "<"}
    for previous, value in zip(values, values[1:]):
        result += value if value in no_space_before or previous in no_space_after else " " + value
    return result


def expression_fact(
    source: JavaSource, expression: JavaExpression
) -> dict[str, Any]:
    """Project a CST expression into a deterministic, serializable narrow IR."""

    node = expression.node
    fact: dict[str, Any] = {
        "kind": node.type,
        "canonical": expression.canonical,
    }
    if node.type == "method_invocation":
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None:
            raise JavaSourceError(
                f"{source.relative_path}: method invocation is incomplete"
            )
        object_node = node.child_by_field_name("object")
        fact["name"] = source.text(name)
        fact["object"] = (
            expression_fact(source, source.expression(object_node))
            if object_node is not None
            else None
        )
        fact["arguments"] = [
            expression_fact(source, source.expression(child))
            for child in arguments.named_children
            if child.type not in COMMENT_NODE_KINDS
        ]
    elif node.type == "object_creation_expression":
        type_node = node.child_by_field_name("type")
        arguments = node.child_by_field_name("arguments")
        if type_node is None or arguments is None:
            raise JavaSourceError(
                f"{source.relative_path}: object creation is incomplete"
            )
        fact["name"] = source.expression(type_node).canonical.replace(" ", "")
        fact["arguments"] = [
            expression_fact(source, source.expression(child))
            for child in arguments.named_children
            if child.type not in COMMENT_NODE_KINDS
        ]
    elif node.type == "field_access":
        object_node = node.child_by_field_name("object")
        field_node = node.child_by_field_name("field")
        if object_node is None or field_node is None:
            raise JavaSourceError(f"{source.relative_path}: field access is incomplete")
        fact["name"] = source.text(field_node)
        fact["object"] = expression_fact(source, source.expression(object_node))
    elif node.named_children:
        fact["children"] = [
            expression_fact(source, source.expression(child))
            for child in node.named_children
            if child.type not in COMMENT_NODE_KINDS
        ]
    return fact


def _node_text(source: bytes, node: Node | None) -> str | None:
    if node is None:
        return None
    return source[node.start_byte : node.end_byte].decode("utf-8")


def _qualified_declaration_name(source: bytes, node: Node) -> str:
    name_node = next((child for child in node.named_children if child.type in {"identifier", "scoped_identifier"}), None)
    if name_node is None:
        raise JavaSourceError("package/import declaration lacks a name")
    return canonical_expression(source, name_node).replace(" ", "")


def _parse_package_imports(relative_path: str, source: bytes, root: Node) -> tuple[str, dict[str, str]]:
    package = ""
    imports: dict[str, str] = {}
    for node in root.named_children:
        if node.type == "package_declaration":
            package = _qualified_declaration_name(source, node)
        elif node.type == "import_declaration":
            imported = _qualified_declaration_name(source, node)
            if not imported.endswith(".*"):
                imports[imported.rsplit(".", 1)[-1]] = imported
    if not package:
        raise JavaSourceError(f"{relative_path}: missing package declaration")
    return package, imports


def _enclosing_type_node(node: Node) -> Node | None:
    current = node.parent
    while current is not None:
        if current.type in TYPE_NODE_KINDS:
            return current
        current = current.parent
    return None


def _parse_types(relative_path: str, source: bytes, root: Node, package: str, imports: dict[str, str]) -> list[JavaType]:
    nodes = [node for node in descendants(root) if node.type in TYPE_NODE_KINDS]
    result: list[JavaType] = []
    for order, node in enumerate(nodes):
        name_node = node.child_by_field_name("name")
        body = node.child_by_field_name("body")
        if name_node is None or body is None:
            raise JavaSourceError(f"{relative_path}: incomplete {node.type}")
        names = [_node_text(source, name_node)]
        parent = _enclosing_type_node(node)
        while parent is not None:
            parent_name = parent.child_by_field_name("name")
            if parent_name is None:
                raise JavaSourceError(f"{relative_path}: nested type parent lacks a name")
            names.append(_node_text(source, parent_name))
            parent = _enclosing_type_node(parent)
        names.reverse()
        superclass_node = node.child_by_field_name("superclass")
        superclass_spelling: str | None = None
        if superclass_node is not None:
            named = superclass_node.named_children
            if named:
                superclass_spelling = canonical_expression(source, named[0]).replace(" ", "")
        interface_spellings: tuple[str, ...] = ()
        interfaces_node = next(
            (
                child
                for child in node.named_children
                if child.type in {"super_interfaces", "extends_interfaces"}
            ),
            None,
        )
        if interfaces_node is not None:
            type_list = next(
                (child for child in interfaces_node.named_children if child.type == "type_list"),
                interfaces_node,
            )
            interface_spellings = tuple(
                canonical_expression(source, child).replace(" ", "")
                for child in type_list.named_children
            )
        result.append(
            JavaType(
                package + "." + ".".join(names),
                names[-1],
                relative_path,
                package,
                TYPE_NODE_KINDS[node.type],
                superclass_spelling,
                None,
                interface_spellings,
                (),
                order,
                _span(relative_path, node),
                node,
                body,
                imports,
            )
        )
    return result


def enclosing_type(types: Sequence[JavaType], node: Node) -> JavaType | None:
    type_node = _enclosing_type_node(node)
    if type_node is None:
        return None
    return next((item for item in types if item.node == type_node), None)


def _modifiers(source: bytes, declaration: Node) -> tuple[str, ...]:
    node = next((child for child in declaration.named_children if child.type == "modifiers"), None)
    if node is None:
        return ()
    return tuple(
        source[child.start_byte : child.end_byte].decode("utf-8")
        for child in node.children
        if child.type not in COMMENT_NODE_KINDS and child.is_named is False
    )


def _parse_fields(relative_path: str, source: bytes, types: Sequence[JavaType]) -> list[JavaField]:
    fields: list[JavaField] = []
    owner_orders: dict[str, int] = {}
    for owner in types:
        for declaration in (
            member
            for member in type_body_members(owner.body_node)
            if member.type in {"field_declaration", "constant_declaration"}
        ):
            type_node = declaration.child_by_field_name("type")
            if type_node is None:
                raise JavaSourceError(f"{relative_path}: field lacks a type")
            for declarator in direct_children_of_type(declaration, {"variable_declarator"}):
                name_node = declarator.child_by_field_name("name")
                if name_node is None:
                    raise JavaSourceError(f"{relative_path}: field declarator lacks a name")
                initializer = declarator.child_by_field_name("value")
                order = owner_orders.get(owner.fqcn, 0)
                owner_orders[owner.fqcn] = order + 1
                fields.append(
                    JavaField(
                        owner.fqcn,
                        _node_text(source, name_node),
                        canonical_expression(source, type_node),
                        _modifiers(source, declaration),
                        JavaExpression(canonical_expression(source, initializer), _span(relative_path, initializer), initializer)
                        if initializer is not None
                        else None,
                        order,
                        _span(relative_path, declarator),
                        declarator,
                    )
                )
    return fields


def _parse_methods(relative_path: str, source: bytes, types: Sequence[JavaType]) -> list[JavaMethod]:
    methods: list[JavaMethod] = []
    owner_orders: dict[str, int] = {}
    for owner in types:
        for node in (
            member
            for member in type_body_members(owner.body_node)
            if member.type in {"method_declaration", "constructor_declaration"}
        ):
            name_node = node.child_by_field_name("name")
            parameters = node.child_by_field_name("parameters")
            if name_node is None or parameters is None:
                raise JavaSourceError(f"{relative_path}: method/constructor is incomplete")
            body = node.child_by_field_name("body")
            parameter_values = tuple(
                JavaExpression(canonical_expression(source, child), _span(relative_path, child), child)
                for child in parameters.named_children
            )
            order = owner_orders.get(owner.fqcn, 0)
            owner_orders[owner.fqcn] = order + 1
            methods.append(
                JavaMethod(
                    owner.fqcn,
                    _node_text(source, name_node),
                    node.type == "constructor_declaration",
                    parameter_values,
                    _span(relative_path, body) if body is not None else None,
                    order,
                    _span(relative_path, node),
                    node,
                )
            )
    return methods


def _parse_calls(relative_path: str, source: bytes, types: Sequence[JavaType], root: Node) -> list[JavaCall]:
    calls: list[JavaCall] = []
    for node in descendants(root):
        if node.type != "method_invocation":
            continue
        name_node = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name_node is None or arguments is None:
            raise JavaSourceError(f"{relative_path}: method invocation is incomplete")
        object_node = node.child_by_field_name("object")
        owner = enclosing_type(types, node)
        calls.append(
            JavaCall(
                owner.fqcn if owner else None,
                JavaExpression(canonical_expression(source, object_node), _span(relative_path, object_node), object_node)
                if object_node is not None
                else None,
                _node_text(source, name_node),
                tuple(
                    JavaExpression(canonical_expression(source, child), _span(relative_path, child), child)
                    for child in arguments.named_children
                    if child.type not in COMMENT_NODE_KINDS
                ),
                _span(relative_path, node),
                node,
            )
        )
    return calls


def parse_source(relative_path: str, source: bytes) -> JavaSource:
    """Parse one Java compilation unit and reject all ERROR/MISSING CST nodes."""

    parser = Parser(JAVA_LANGUAGE)
    tree = parser.parse(source)
    _diagnose_tree(relative_path, tree.root_node)
    package, imports = _parse_package_imports(relative_path, source, tree.root_node)
    types = _parse_types(relative_path, source, tree.root_node, package, imports)
    fields = _parse_fields(relative_path, source, types)
    methods = _parse_methods(relative_path, source, types)
    calls = _parse_calls(relative_path, source, types, tree.root_node)
    return JavaSource(relative_path, source, tree, package, imports, types, fields, methods, calls)


def resolve_type_reference(spelling: str | None, owner: JavaType, known: dict[str, JavaType]) -> str | None:
    """Resolve a CST type spelling through exact, import, package, or suffix lookup."""

    if spelling is None or spelling in {"Object", "Record", "Enum"}:
        return None
    erased = spelling.split("<", 1)[0]
    if erased in known:
        return erased
    first, *rest = erased.split(".")
    candidates: list[str] = []
    if first in owner.imports:
        candidates.append(".".join([owner.imports[first], *rest]))
    candidates.append(".".join([owner.package, erased]))
    candidates.extend(fqcn for fqcn in known if fqcn.endswith("." + erased))
    unique = [candidate for candidate in dict.fromkeys(candidates) if candidate in known]
    if len(unique) == 1:
        return unique[0]
    if len(unique) > 1:
        raise JavaSourceError(f"{owner.source_path}: ambiguous type {spelling}: {unique}")
    return None


def resolve_class_hierarchy(types: list[JavaType]) -> None:
    known = {item.fqcn: item for item in types}
    for item in types:
        item.superclass = resolve_type_reference(item.superclass_spelling, item, known)
        item.interfaces = tuple(
            resolved
            for spelling in item.interface_spellings
            if (resolved := resolve_type_reference(spelling, item, known)) is not None
        )
