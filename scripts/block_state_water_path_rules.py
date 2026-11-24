"""Classify the WATER branch of Vanilla's effective block path predicate."""

from __future__ import annotations

from pathlib import Path

from vanilla_java_source import JavaSourceError, parse_source

STATIC = 0
STATE_FLUID_WATER = 1
LIQUID_SOURCE_NOT_LAVA = 2


def _canonical(source, node):
    return source.expression(node).canonical.replace(" ", "")


def _single_expression(source, statement):
    if statement.type != "return_statement" or len(statement.named_children) != 1:
        raise JavaSourceError(f"{source.relative_path}: unsupported path return")
    return statement.named_children[0]


def _classify_expression(source, expression, superclass):
    value = _canonical(source, expression)
    kind = expression.type
    if kind in {"true", "false"}:
        return STATIC
    if kind == "method_invocation" and value == "state.getFluidState().is(FluidTags.WATER)":
        return STATE_FLUID_WATER
    if kind == "method_invocation" and value == "state.getValue(WATERLOGGED)":
        return STATIC
    if kind == "unary_expression" and value == "!this.fluid.is(FluidTags.LAVA)":
        return LIQUID_SOURCE_NOT_LAVA
    if kind == "binary_expression" and len(expression.named_children) == 2:
        left, right = expression.named_children
        if _canonical(source, left) == "type==PathComputationType.WATER" and any(
            child.type == "&&" for child in expression.children
        ):
            return _classify_expression(source, right, superclass)
    if kind == "ternary_expression" and len(expression.named_children) == 3:
        condition, yes, no = expression.named_children
        condition_value = _canonical(source, condition)
        if condition_value in {
            "type==PathComputationType.AIR&&!this.hasCollision",
            "type==PathComputationType.LAND",
        }:
            if (
                _canonical(source, no) == "super.isPathfindable(state,type)"
                and no.type == "method_invocation"
            ):
                if superclass is None:
                    raise JavaSourceError(f"{source.relative_path}: missing path superclass")
                return superclass()
            return _classify_expression(source, no, superclass)
    if kind == "switch_expression":
        selector, block = expression.named_children
        if _canonical(source, selector) != "(type)" or block.type != "switch_block":
            raise JavaSourceError(f"{source.relative_path}: unsupported path switch selector")
        water = []
        for arm in block.named_children:
            if arm.type not in {"switch_rule", "switch_block_statement_group"}:
                raise JavaSourceError(f"{source.relative_path}: unsupported path switch arm")
            label, *statements = arm.named_children
            if label.type != "switch_label":
                raise JavaSourceError(f"{source.relative_path}: unsupported path switch label")
            labels = [_canonical(source, item) for item in label.named_children]
            if labels != ["WATER"]:
                if "WATER" in labels:
                    raise JavaSourceError(f"{source.relative_path}: mixed WATER path branch")
                continue
            if len(statements) != 1:
                raise JavaSourceError(f"{source.relative_path}: unsupported WATER path arm")
            candidate = statements[0]
            if arm.type == "switch_block_statement_group":
                candidate = _single_expression(source, candidate)
            elif candidate.type == "expression_statement" and len(candidate.named_children) == 1:
                candidate = candidate.named_children[0]
            else:
                raise JavaSourceError(f"{source.relative_path}: unsupported WATER path rule")
            water.append(_classify_expression(source, candidate, superclass))
        if len(water) != 1:
            raise JavaSourceError(f"{source.relative_path}: missing or repeated WATER path arm")
        return water[0]
    raise JavaSourceError(f"{source.relative_path}: unsupported WATER path expression {value}")


def classify_owner(source_root: Path, fqcn: str) -> int:
    cache = {}
    visiting = set()

    def classify(owner):
        if owner in cache:
            return cache[owner]
        if owner in visiting:
            raise JavaSourceError(f"cyclic path owner hierarchy at {owner}")
        visiting.add(owner)
        path = owner.replace(".", "/") + ".java"
        file = source_root / path
        if not file.is_file():
            raise JavaSourceError(f"missing path owner source {path}")
        source = parse_source(path, file.read_bytes())
        types = [item for item in source.types if item.fqcn == owner]
        if len(types) != 1:
            raise JavaSourceError(f"{path}: owner declaration is missing or ambiguous")
        declaration = types[0]
        parent_name = (
            declaration.superclass_spelling.split("<", 1)[0]
            if declaration.superclass_spelling is not None
            else None
        )

        def parent():
            if parent_name is None:
                raise JavaSourceError(f"{path}: missing path superclass")
            if parent_name in declaration.imports:
                parent_fqcn = declaration.imports[parent_name]
            elif "." in parent_name:
                parent_fqcn = parent_name
            else:
                parent_fqcn = declaration.package + "." + parent_name
            return classify(parent_fqcn)

        methods = [
            item for item in source.methods if item.owner == owner and item.name == "isPathfindable"
        ]
        matches = []
        for method in methods:
            parameters = method.node.child_by_field_name("parameters").named_children
            signature = [
                (
                    source.text(p.child_by_field_name("type")),
                    source.text(p.child_by_field_name("name")),
                )
                for p in parameters
            ]
            if signature == [("BlockState", "state"), ("PathComputationType", "type")]:
                matches.append(method)
        if not matches and not methods and declaration.superclass_spelling is not None:
            result = parent()
            cache[owner] = result
            return result
        if len(matches) != 1 or len(methods) != 1:
            raise JavaSourceError(
                f"{path}: expected one exact "
                "isPathfindable(BlockState, PathComputationType) declaration"
            )
        if parent_name is not None:
            parent()
        body = matches[0].node.child_by_field_name("body")
        if body is None or len(body.named_children) != 1:
            raise JavaSourceError(f"{path}: unsupported path body")
        statement = body.named_children[0]
        expression = (
            _single_expression(source, statement)
            if statement.type == "return_statement"
            else statement
        )
        result = _classify_expression(source, expression, parent)
        cache[owner] = result
        return result

    return classify(fqcn)
