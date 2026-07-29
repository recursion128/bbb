"""Tree-sitter-backed extraction for pure vanilla block memberships."""

from __future__ import annotations

from dataclasses import dataclass

from block_registry_catalog_extractor import field_constant, resolve_block_resource_path
from vanilla_java_source import (
    JavaExpression,
    JavaSource,
    descendants,
    resolve_class_hierarchy,
)
from vanilla_tag_source import VanillaTagSourceError, resolve_tag_values


class BlockMembershipCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class BlockMembershipEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class BlockFloatPropertyEntry:
    constant_name: str
    resource_id: str
    value: float


@dataclass(frozen=True)
class LeafAnimateParticleProfileEntry:
    constant_name: str
    resource_id: str
    chance: float
    particle_resource_id: str
    foliage_tinted: bool
    color_argb: int | None


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise BlockMembershipCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _builder_property_blocks(
    source: JavaSource,
    property_name: str,
    expected_count: int,
    block_id_constants: dict[str, str] | None = None,
    include_property_helpers: bool = False,
) -> list[BlockMembershipEntry]:
    property_helpers: set[str] = set()
    if include_property_helpers:
        for method in source.methods_for("net.minecraft.world.level.block.Blocks"):
            if method.body is None:
                continue
            property_calls = [
                call
                for call in source.calls
                if method.span.start_byte <= call.span.start_byte
                and call.span.end_byte <= method.span.end_byte
                and call.name == property_name
            ]
            if not property_calls:
                continue
            return_type = method.node.child_by_field_name("type")
            if return_type is None or source.expression(return_type).canonical.replace(
                " ", ""
            ) not in {"Properties", "BlockBehaviour.Properties"}:
                _fail(
                    source,
                    source.expression(method.node),
                    f"unsupported {property_name} registration helper",
                )
            return_statements = [
                node
                for node in descendants(method.node)
                if node.type == "return_statement"
            ]
            if (
                len(return_statements) != 1
                or len(return_statements[0].named_children) != 1
                or return_statements[0].named_children[0].type != "method_invocation"
                or not all(
                    return_statements[0].start_byte <= call.span.start_byte
                    and call.span.end_byte <= return_statements[0].end_byte
                    for call in property_calls
                )
            ):
                _fail(
                    source,
                    source.expression(method.node),
                    f"{property_name} registration helper must return one "
                    "unconditional call chain",
                )
            property_helpers.add(method.name)

    entries: list[BlockMembershipEntry] = []
    for field in source.fields_for("net.minecraft.world.level.block.Blocks"):
        initializer = field.initializer
        if (
            field.type_spelling != "Block"
            or initializer is None
            or initializer.node.type != "method_invocation"
        ):
            continue
        calls = [
            call
            for call in source.calls
            if initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= initializer.span.end_byte
        ]
        if not any(
            call.name == property_name
            or (
                call.name in property_helpers
                and (
                    call.qualifier is None
                    or call.qualifier.canonical.replace(" ", "")
                    in {"Blocks", "net.minecraft.world.level.block.Blocks"}
                )
            )
            for call in calls
        ):
            continue
        register_calls = [call for call in calls if call.name == "register"]
        if len(register_calls) != 1 or not register_calls[0].arguments:
            _fail(source, initializer, f"unsupported {property_name} block registration")
        entries.append(
            BlockMembershipEntry(
                field.name,
                "minecraft:"
                + resolve_block_resource_path(
                    source,
                    register_calls[0].arguments[0],
                    block_id_constants or {},
                ),
            )
        )

    selected_constants = {entry.constant_name for entry in entries}
    for field in source.fields_for("net.minecraft.world.level.block.Blocks"):
        if field.initializer is None or field.name in selected_constants:
            continue
        for call in source.calls:
            if not (
                field.initializer.span.start_byte <= call.span.start_byte
                and call.span.end_byte <= field.initializer.span.end_byte
                and call.name in {"ofFullCopy", "ofLegacyCopy"}
                and call.arguments
            ):
                continue
            copied_constant = (
                call.arguments[0].canonical.replace(" ", "").rsplit(".", 1)[-1]
            )
            if copied_constant in selected_constants:
                _fail(
                    source,
                    call.arguments[0],
                    f"{field.name} copies {property_name} from {copied_constant}; "
                    "copy propagation requires an explicit resolver",
                )

    if len(entries) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} {property_name} blocks, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate {property_name} block constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate {property_name} block resource id"
        )
    return entries


def no_terrain_particle_blocks(
    source: JavaSource, expected_count: int = 2
) -> list[BlockMembershipEntry]:
    return _builder_property_blocks(source, "noTerrainParticles", expected_count)


def no_collision_blocks(
    source: JavaSource,
    block_id_constants: dict[str, str],
    expected_count: int = 255,
) -> list[BlockMembershipEntry]:
    return _builder_property_blocks(
        source,
        "noCollision",
        expected_count,
        block_id_constants,
        include_property_helpers=True,
    )


def air_blocks(
    source: JavaSource, expected_count: int = 3
) -> list[BlockMembershipEntry]:
    return _builder_property_blocks(source, "air", expected_count)


def block_float_property_overrides(
    source: JavaSource,
    block_id_constants: dict[str, str],
    property_name: str,
    expected_count: int,
) -> list[BlockFloatPropertyEntry]:
    """Extract one literal float-valued ``BlockBehaviour.Properties`` override."""

    entries = _builder_property_blocks(
        source, property_name, expected_count, block_id_constants
    )
    by_constant = {entry.constant_name: entry for entry in entries}
    values: list[BlockFloatPropertyEntry] = []
    for field in source.fields_for("net.minecraft.world.level.block.Blocks"):
        entry = by_constant.get(field.name)
        if entry is None or field.initializer is None:
            continue
        calls = [
            call
            for call in source.calls
            if field.initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= field.initializer.span.end_byte
            and call.name == property_name
        ]
        if len(calls) != 1 or len(calls[0].arguments) != 1:
            _fail(
                source,
                field.initializer,
                f"{property_name} block lacks one literal value",
            )
        literal = calls[0].arguments[0].canonical
        if not literal.endswith(("F", "f")):
            _fail(
                source,
                calls[0].arguments[0],
                f"{property_name} value is not a float literal",
            )
        try:
            value = float(literal[:-1])
        except ValueError:
            _fail(
                source,
                calls[0].arguments[0],
                f"{property_name} value is not a float literal",
            )
        values.append(BlockFloatPropertyEntry(field.name, entry.resource_id, value))

    if len(values) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} {property_name} values, "
            f"extracted {len(values)}"
        )
    return values


def block_friction_overrides(
    source: JavaSource,
    block_id_constants: dict[str, str],
    expected_count: int = 5,
) -> list[BlockFloatPropertyEntry]:
    """Extract the non-default ``BlockBehaviour.Properties.friction`` values."""

    return block_float_property_overrides(
        source, block_id_constants, "friction", expected_count
    )


def block_speed_factor_overrides(
    source: JavaSource,
    block_id_constants: dict[str, str],
    expected_count: int = 2,
) -> list[BlockFloatPropertyEntry]:
    """Extract the non-default ``BlockBehaviour.Properties.speedFactor`` values."""

    return block_float_property_overrides(
        source, block_id_constants, "speedFactor", expected_count
    )


def fire_flammable_blocks(
    source: JavaSource,
    block_resource_paths: dict[str, str],
    expected_count: int = 207,
) -> list[BlockMembershipEntry]:
    """Extract the exact ``FireBlock.bootStrap`` flammability membership."""

    methods = [
        method
        for method in source.methods_for(
            "net.minecraft.world.level.block.FireBlock"
        )
        if method.name == "bootStrap"
    ]
    if len(methods) != 1 or methods[0].body is None:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected one FireBlock.bootStrap body"
        )
    method = methods[0]
    calls = [
        call
        for call in source.calls
        if method.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.span.end_byte
        and call.name == "setFlammable"
    ]
    entries: list[BlockMembershipEntry] = []
    for call in calls:
        if call.qualifier is None or call.qualifier.canonical.replace(" ", "") != "fire":
            _fail(
                source,
                source.expression(call.node),
                "setFlammable must target the bootstrap fire",
            )
        if len(call.arguments) != 3:
            _fail(
                source,
                source.expression(call.node),
                "setFlammable must have block and two odds",
            )
        constant_name = field_constant(source, call.arguments[0], "Blocks")
        if constant_name is None:
            _fail(
                source,
                call.arguments[0],
                "setFlammable block must be a Blocks constant",
            )
        resource_path = block_resource_paths.get(constant_name)
        if resource_path is None:
            _fail(
                source,
                call.arguments[0],
                f"unresolved Blocks.{constant_name}",
            )
        for odds in call.arguments[1:]:
            try:
                value = int(odds.canonical)
            except ValueError:
                _fail(source, odds, "setFlammable odds must be integer literals")
            if value <= 0:
                _fail(source, odds, "setFlammable odds must be positive")
        entries.append(
            BlockMembershipEntry(constant_name, f"minecraft:{resource_path}")
        )

    if len(entries) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} flammable blocks, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate flammable block constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate flammable block resource id"
        )
    return entries


def _constructed_type_name(source: JavaSource, node) -> str | None:
    if node.type == "object_creation_expression":
        type_node = node.child_by_field_name("type")
        if type_node is None:
            _fail(source, source.expression(node), "object creation lacks type")
        return source.expression(type_node).canonical.replace(" ", "")
    if node.type == "method_reference" and any(
        child.type == "new" for child in node.children
    ):
        type_nodes = [
            child for child in node.named_children if child.type != "type_arguments"
        ]
        if len(type_nodes) != 1:
            _fail(source, source.expression(node), "constructor reference lacks unique type")
        return source.expression(type_nodes[0]).canonical.replace(" ", "")
    return None


def constructor_family_blocks(
    source: JavaSource,
    constructor_types: frozenset[str],
    expected_count: int,
    block_id_constants: dict[str, str] | None = None,
) -> list[BlockMembershipEntry]:
    if not constructor_types:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: constructor family cannot be empty"
        )

    helper_types: dict[str, str] = {}
    for method in source.methods_for("net.minecraft.world.level.block.Blocks"):
        if method.body is None:
            continue
        constructed_types = {
            type_name
            for node in descendants(method.node)
            if (type_name := _constructed_type_name(source, node)) in constructor_types
        }
        if not constructed_types:
            continue
        if len(constructed_types) != 1:
            _fail(
                source,
                source.expression(method.node),
                "constructor helper matches multiple family types",
            )
        register_calls = [
            call
            for call in source.calls
            if method.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= method.span.end_byte
            and call.name == "register"
        ]
        if len(register_calls) != 1 or not register_calls[0].arguments:
            _fail(
                source,
                source.expression(method.node),
                "constructor helper lacks unique register call",
            )
        if not method.parameters:
            _fail(
                source,
                source.expression(method.node),
                "constructor helper lacks resource parameter",
            )
        parameter_name = method.parameters[0].node.child_by_field_name("name")
        if parameter_name is None or (
            register_calls[0].arguments[0].canonical
            != source.expression(parameter_name).canonical
        ):
            _fail(
                source,
                source.expression(method.node),
                "constructor helper does not forward its first parameter to register",
            )
        if method.name in helper_types:
            _fail(
                source,
                source.expression(method.node),
                "duplicate constructor helper name",
            )
        helper_types[method.name] = next(iter(constructed_types))

    entries: list[BlockMembershipEntry] = []
    for field in source.fields_for("net.minecraft.world.level.block.Blocks"):
        if field.type_spelling != "Block" or field.initializer is None:
            continue
        constructed_types = {
            type_name
            for node in descendants(field.initializer.node)
            if (type_name := _constructed_type_name(source, node)) in constructor_types
        }
        helper_calls = [
            call
            for call in source.calls
            if field.initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= field.initializer.span.end_byte
            and call.name in helper_types
        ]
        matched_types = constructed_types | {
            helper_types[call.name] for call in helper_calls
        }
        if not matched_types:
            continue
        if len(matched_types) != 1 or len(helper_calls) > 1:
            _fail(
                source,
                field.initializer,
                "block registration matches multiple constructor family types",
            )
        if helper_calls:
            if constructed_types or not helper_calls[0].arguments:
                _fail(
                    source,
                    field.initializer,
                    "constructor family has ambiguous direct/helper registration",
                )
            resource_expression = helper_calls[0].arguments[0]
        else:
            register_calls = [
                call
                for call in source.calls
                if field.initializer.span.start_byte <= call.span.start_byte
                and call.span.end_byte <= field.initializer.span.end_byte
                and call.name == "register"
            ]
            if len(register_calls) != 1 or not register_calls[0].arguments:
                _fail(
                    source,
                    field.initializer,
                    "constructor family lacks unique register call",
                )
            resource_expression = register_calls[0].arguments[0]
        entries.append(
            BlockMembershipEntry(
                field.name,
                "minecraft:"
                + resolve_block_resource_path(
                    source,
                    resource_expression,
                    block_id_constants or {},
                ),
            )
        )

    family = ", ".join(sorted(constructor_types))
    if len(entries) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} blocks constructed by "
            f"{{{family}}}, extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate constructor-family block constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate constructor-family block resource id"
        )
    return entries


def tinted_leaves_particle_constant(
    sources: list[JavaSource],
    owner_fqcn: str = "net.minecraft.world.level.block.TintedParticleLeavesBlock",
) -> str:
    matches: list[tuple[JavaSource, JavaExpression]] = []
    for source in sources:
        for method in source.methods_for(owner_fqcn):
            if method.name != "spawnFallingLeavesParticle" or method.body is None:
                continue
            for call in source.calls:
                if not (
                    method.span.start_byte <= call.span.start_byte
                    and call.span.end_byte <= method.span.end_byte
                    and call.name == "create"
                    and call.qualifier is not None
                    and call.qualifier.canonical.replace(" ", "") == "ColorParticleOption"
                    and len(call.arguments) == 2
                ):
                    continue
                constant_name = field_constant(
                    source, call.arguments[0], "ParticleTypes"
                )
                if constant_name is None:
                    _fail(
                        source,
                        call.arguments[0],
                        "tinted leaf particle must be a ParticleTypes constant",
                    )
                tint_calls = [
                    candidate
                    for candidate in source.calls
                    if call.arguments[1].span.start_byte
                    <= candidate.span.start_byte
                    and candidate.span.end_byte <= call.arguments[1].span.end_byte
                    and candidate.name == "getClientLeafTintColor"
                ]
                if (
                    len(tint_calls) != 1
                    or tint_calls[0].span != call.arguments[1].span
                    or len(tint_calls[0].arguments) != 1
                ):
                    _fail(
                        source,
                        call.arguments[1],
                        "tinted leaf color must come from getClientLeafTintColor",
                    )
                matches.append((source, call.arguments[0]))

    if len(matches) != 1:
        raise BlockMembershipCatalogExtractionError(
            f"expected one {owner_fqcn}.spawnFallingLeavesParticle "
            f"ColorParticleOption.create call, extracted {len(matches)}"
        )
    source, expression = matches[0]
    constant_name = field_constant(source, expression, "ParticleTypes")
    assert constant_name is not None
    return constant_name


def leaf_animate_particle_profiles(
    source: JavaSource,
    leaf_entries: list[BlockMembershipEntry],
    leaf_constructor_types: frozenset[str],
    foliage_tinted_constructor_types: frozenset[str],
    particle_resource_ids: dict[str, str],
    foliage_particle_constant: str,
    expected_count: int = 11,
) -> list[LeafAnimateParticleProfileEntry]:
    """Extract ``LeavesBlock.animateTick`` constructor configuration."""

    if not foliage_tinted_constructor_types <= leaf_constructor_types:
        raise BlockMembershipCatalogExtractionError(
            "foliage-tinted leaf constructor family is not a leaf-family subset"
        )
    leaf_by_constant = {entry.constant_name: entry for entry in leaf_entries}
    profiles: list[LeafAnimateParticleProfileEntry] = []
    for field in source.fields_for("net.minecraft.world.level.block.Blocks"):
        leaf_entry = leaf_by_constant.get(field.name)
        if leaf_entry is None:
            continue
        if field.initializer is None:
            raise BlockMembershipCatalogExtractionError(
                f"{source.relative_path}: leaf field {field.name} lacks initializer"
            )
        creations = [
            node
            for node in descendants(field.initializer.node)
            if _constructed_type_name(source, node) in leaf_constructor_types
        ]
        if len(creations) != 1:
            _fail(
                source,
                field.initializer,
                f"leaf field {field.name} must contain one leaf constructor",
            )
        creation = creations[0]
        constructor_type = _constructed_type_name(source, creation)
        assert constructor_type is not None
        arguments = creation.child_by_field_name("arguments")
        if arguments is None:
            _fail(source, source.expression(creation), "leaf constructor lacks arguments")
        args = [source.expression(child) for child in arguments.named_children]
        expected_arity = 2 if constructor_type in foliage_tinted_constructor_types else 3
        if len(args) != expected_arity:
            _fail(
                source,
                source.expression(creation),
                f"{constructor_type} constructor expected {expected_arity} arguments",
            )
        chance_literal = args[0].canonical.replace(" ", "")
        if not chance_literal.endswith(("F", "f")):
            _fail(source, args[0], "leaf particle chance is not a float literal")
        try:
            chance = float(chance_literal[:-1])
        except ValueError:
            _fail(source, args[0], "leaf particle chance is not a float literal")
        if not 0.0 <= chance <= 1.0:
            _fail(source, args[0], "leaf particle chance is outside [0, 1]")

        foliage_tinted = constructor_type in foliage_tinted_constructor_types
        color_argb = None
        if foliage_tinted:
            particle_constant = foliage_particle_constant
        else:
            particle_constant = field_constant(source, args[1], "ParticleTypes")
            if particle_constant is None:
                matching_calls = [
                    call
                    for call in source.calls
                    if call.span == args[1].span
                    and call.name == "create"
                    and call.qualifier is not None
                    and call.qualifier.canonical.replace(" ", "")
                    == "ColorParticleOption"
                ]
                if len(matching_calls) != 1 or len(matching_calls[0].arguments) != 2:
                    _fail(
                        source,
                        args[1],
                        "untinted leaf particle must be a ParticleTypes constant "
                        "or ColorParticleOption.create",
                    )
                particle_constant = field_constant(
                    source, matching_calls[0].arguments[0], "ParticleTypes"
                )
                if particle_constant is None:
                    _fail(
                        source,
                        matching_calls[0].arguments[0],
                        "colored leaf particle must be a ParticleTypes constant",
                    )
                color_literal = matching_calls[0].arguments[1].canonical.replace(" ", "")
                try:
                    color_argb = int(color_literal, 10) & 0xFFFF_FFFF
                except ValueError:
                    _fail(
                        source,
                        matching_calls[0].arguments[1],
                        "colored leaf ARGB value is not an integer literal",
                    )

        particle_resource_id = particle_resource_ids.get(particle_constant)
        if particle_resource_id is None:
            _fail(
                source,
                args[0],
                f"unresolved ParticleTypes.{particle_constant}",
            )
        profiles.append(
            LeafAnimateParticleProfileEntry(
                constant_name=field.name,
                resource_id=leaf_entry.resource_id,
                chance=chance,
                particle_resource_id=particle_resource_id,
                foliage_tinted=foliage_tinted,
                color_argb=color_argb,
            )
        )

    if len(profiles) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} leaf animate profiles, "
            f"extracted {len(profiles)}"
        )
    if len({profile.resource_id for profile in profiles}) != len(profiles):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate leaf animate profile resource id"
        )
    return profiles


def constructor_types_extending(
    sources: list[JavaSource], base_fqcn: str, expected_count: int
) -> frozenset[str]:
    types = [item for source in sources for item in source.types]
    resolve_class_hierarchy(types)
    known = {item.fqcn: item for item in types}
    if base_fqcn not in known:
        raise BlockMembershipCatalogExtractionError(
            f"missing constructor-family base class {base_fqcn}"
        )

    members: list[str] = []
    for item in types:
        current = item
        visited: set[str] = set()
        while current.fqcn not in visited:
            visited.add(current.fqcn)
            if current.fqcn == base_fqcn:
                members.append(item.simple_name)
                break
            if current.superclass is None or current.superclass not in known:
                break
            current = known[current.superclass]

    if len(members) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"expected {expected_count} classes extending {base_fqcn}, "
            f"extracted {len(members)}"
        )
    if len(set(members)) != len(members):
        raise BlockMembershipCatalogExtractionError(
            f"duplicate simple class name extending {base_fqcn}"
        )
    return frozenset(members)


def invisible_render_shape_constructor_types(
    sources: list[JavaSource], expected_count: int = 9
) -> frozenset[str]:
    classes: list[str] = []
    for source in sources:
        types = {item.fqcn: item for item in source.types}
        for method in source.methods:
            if method.name != "getRenderShape" or method.body is None:
                continue
            returns = [
                node
                for node in descendants(method.node)
                if node.type == "return_statement"
                and _nearest_method(node) == method.node
            ]
            values: list[str | None] = []
            for return_node in returns:
                expressions = return_node.named_children
                if len(expressions) != 1:
                    _fail(
                        source,
                        source.expression(return_node),
                        "getRenderShape return lacks unique expression",
                    )
                values.append(
                    field_constant(
                        source, source.expression(expressions[0]), "RenderShape"
                    )
                )
            if "INVISIBLE" not in values:
                continue
            if values != ["INVISIBLE"]:
                _fail(
                    source,
                    source.expression(method.node),
                    "getRenderShape mixes INVISIBLE with other returns",
                )
            owner = types.get(method.owner)
            if owner is None:
                _fail(source, source.expression(method.node), "getRenderShape owner is missing")
            classes.append(owner.simple_name)

    if len(classes) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"expected {expected_count} getRenderShape(INVISIBLE) block classes, "
            f"extracted {len(classes)}"
        )
    if len(set(classes)) != len(classes):
        raise BlockMembershipCatalogExtractionError(
            "duplicate getRenderShape(INVISIBLE) block class"
        )
    return frozenset(classes)


def _nearest_method(node):
    current = node.parent
    while current is not None:
        if current.type in {"method_declaration", "constructor_declaration"}:
            return current
        current = current.parent
    return None


def block_tag_membership(
    tag_resources: dict[str, tuple[str, bytes]],
    tag_id: str,
    expected_count: int,
) -> tuple[str, ...]:
    try:
        return resolve_tag_values(
            tag_resources, tag_id, expected_count, registry_name="block"
        )
    except VanillaTagSourceError as error:
        raise BlockMembershipCatalogExtractionError(str(error)) from error
