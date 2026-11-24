#!/usr/bin/env python3
"""Generate/check shared Vanilla 26.1 block sound-group facts.

Every fact here comes out of the Vanilla sources:

* ``SoundType.java`` holds the 123 ``SoundType`` constants -- the five events,
  the volume and the pitch.
* ``SoundEvents.java`` registers every ``SoundEvent`` under its resource id.
* ``Blocks.java`` assigns a ``SoundType`` per block, either directly
  (``.sound(SoundType.X)``), by copying another block
  (``BlockBehaviour.Properties.ofFullCopy``/``ofLegacyCopy``), through one of
  its own property helpers (``logProperties``, ``leavesProperties``, ...), or
  by handing a ``BlockSetType``/``WoodType`` to a block class whose constructor
  calls ``properties.sound(type.soundType())``. Blocks that do none of these
  keep ``BlockBehaviour.Properties``' ``SoundType.STONE`` default
  (``BlockBehaviour.java:973``).
* ``BlockSetType.java``/``WoodType.java`` hold the ``SoundType`` those wood
  families answer.
* ``DecoratedPotBlock.getSoundType`` (``DecoratedPotBlock.java:212-214``) is the
  only state-dependent group; the states it splits come from the embedded
  built-in block-state catalog.
"""

from __future__ import annotations

import argparse
import difflib
import json
import re
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
from block_registry_catalog_extractor import (
    BLOCKS_OWNER,
    WEATHERING_COPPER_VARIANTS,
    block_factory_class,
    extract_block_registry_catalog,
)
from vanilla_java_source import (
    JavaExpression,
    JavaSource,
    descendants,
    parse_source,
)

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/block_sound_facts_26_1.rs"
BLOCK_STATES = REPO_ROOT / "crates/bbb-facts/data/block_states_26_1.json"

SOUND_TYPE_PATH = "net/minecraft/world/level/block/SoundType.java"
SOUND_EVENTS_PATH = "net/minecraft/sounds/SoundEvents.java"
SOUND_TYPE_OWNER = "net.minecraft.world.level.block.SoundType"
SOUND_EVENTS_OWNER = "net.minecraft.sounds.SoundEvents"
BLOCKS_PATH = "net/minecraft/world/level/block/Blocks.java"
BLOCK_IDS_PATH = "net/minecraft/references/BlockIds.java"
BLOCK_ENTITY_TYPES_PATH = "net/minecraft/world/level/block/entity/BlockEntityType.java"
BLOCK_SET_TYPE_PATH = (
    "net/minecraft/world/level/block/state/properties/BlockSetType.java"
)
WOOD_TYPE_PATH = "net/minecraft/world/level/block/state/properties/WoodType.java"
BLOCK_SET_TYPE_OWNER = "net.minecraft.world.level.block.state.properties.BlockSetType"
WOOD_TYPE_OWNER = "net.minecraft.world.level.block.state.properties.WoodType"
BLOCK_CLASS_ROOT = "net/minecraft/world/level/block"
BLOCK_BEHAVIOUR_PATH = "net/minecraft/world/level/block/state/BlockBehaviour.java"

# `BlockBehaviour.Properties`' own default (`BlockBehaviour.java:973`).
DEFAULT_SOUND_TYPE = "STONE"
# `BlockSetType`'s compact constructor default (`BlockSetType.java`), read off
# the record's own `new BlockSetType("oak")` form.
BLOCK_SET_TYPE_DEFAULT = "WOOD"
# Where `soundType` sits in each wood-family record (`BlockSetType.java:11-26`,
# `WoodType.java:11-13`).
BLOCK_SET_TYPE_SOUND_INDEX = 5
WOOD_TYPE_SOUND_INDEX = 2
WOOD_TYPE_HANGING_SIGN_SOUND_INDEX = 3
# `WoodType`'s compact constructor (`WoodType.java:65-67`).
WOOD_TYPE_DEFAULT = ("WOOD", "HANGING_SIGN")
# The `SoundType` a hanging-sign class takes from its `WoodType`
# (`WoodType.java:12`).
HANGING_SIGN_ACCESSOR = "hangingSignSoundType"
SOUND_ACCESSOR = "soundType"
# `BlockBehaviour.getSoundType(BlockState)` (`BlockBehaviour.java:403-405`) and
# the state-dependent override `DecoratedPotBlock` makes
# (`DecoratedPotBlock.java:212-214`).
GET_SOUND_TYPE_METHOD = "getSoundType"
BLOCK_STATE_PARAMETER = "BlockState"
# The property helpers `Blocks.java` builds a registration's properties with;
# their own bodies are where a `.sound(...)` or a copy lives.
PROPERTY_HELPERS = {
    "logProperties",
    "netherStemProperties",
    "leavesProperties",
    "shulkerBoxProperties",
    "pistonProperties",
    "buttonProperties",
    "flowerPotProperties",
    "candleProperties",
    "wallVariant",
    "registerBed",
    "registerStair",
    "registerLegacyStair",
    "registerStainedGlass",
}
FLOAT_LITERAL = re.compile(r"^-?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][-+]?\d+)?[FfDd]?$")
GROUP_CONSTANT = re.compile(r"^(BlockSetType|WoodType)\.([A-Z][A-Z0-9_]*)$")
ACCESSOR_CALL = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\.(soundType|hangingSignSoundType)\(\)$")
SOUND_TYPE_CONSTANT = re.compile(r"^SoundType\.([A-Z][A-Z0-9_]*)$")
BLOCK_STATE_PROPERTY = re.compile(r"^state\.getValue\(([A-Z][A-Z0-9_]*)\)$")

EXPECTED_SOUND_TYPES = 123
EXPECTED_BLOCKS = 1168
EXPECTED_STATES = 29873


class BlockSoundExtractionError(ValueError):
    """An explicit unsupported or inconsistent block sound source."""


class NoSoundAssignment(BlockSoundExtractionError):
    """This properties chain names no group, so the class or default decides."""


def _fail(source: JavaSource, node, message: str) -> None:
    span = source.span(node)
    raise BlockSoundExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--block-states", type=Path, default=BLOCK_STATES)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    args.source_root = resolve_source_root(args)
    return args


@dataclass(frozen=True)
class SoundTypeRow:
    name: str
    volume: str
    pitch: str
    events: tuple[str, str, str, str, str]


@dataclass(frozen=True)
class BlockSoundRow:
    block_type_id: int
    resource_id: str
    sound_type: str


@dataclass(frozen=True)
class StateSoundRow:
    state_id: int
    sound_type: str


@dataclass(frozen=True)
class BlockSoundCatalog:
    sound_types: tuple[SoundTypeRow, ...]
    blocks: tuple[BlockSoundRow, ...]
    overrides: tuple[StateSoundRow, ...]


def _arguments(source: JavaSource, invocation) -> list:
    arguments = invocation.child_by_field_name("arguments")
    if arguments is None:
        return []
    return [
        child
        for child in arguments.named_children
        if child.type != "line_comment" and child.type != "block_comment"
    ]


def _invocations(source: JavaSource, node, names: set[str]):
    """Every `name(...)` invocation at or under `node` whose name is in `names`.

    The node itself counts: a registration whose properties come from a
    `Blocks.java` helper is the helper call (`Blocks.OAK_STAIRS`), and its body
    is the only place the group is named.
    """

    for candidate in (node, *descendants(node)):
        if candidate.type != "method_invocation":
            continue
        name = candidate.child_by_field_name("name")
        if name is None or source.text(name) not in names:
            continue
        yield candidate, name


def _number_literal(source: JavaSource, node) -> str:
    text = source.text(node).strip()
    if not FLOAT_LITERAL.match(text):
        _fail(source, node, f"{text!r} is not a float literal")
    return text.rstrip("FfDd")


def _sound_event_ids(source: JavaSource) -> dict[str, str]:
    """`SoundEvents.java`: every `register("...")` constant's resource id."""

    ids: dict[str, str] = {}
    for field in source.fields_for(SOUND_EVENTS_OWNER):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            continue
        name = initializer.node.child_by_field_name("name")
        if name is None or source.text(name) not in {"register", "registerForHolder"}:
            continue
        arguments = _arguments(source, initializer.node)
        if not arguments or arguments[0].type != "string_literal":
            _fail(source, initializer.node, f"SoundEvents.{field.name} is not registered by id")
        if field.name in ids:
            _fail(source, initializer.node, f"duplicate SoundEvents.{field.name}")
        ids[field.name] = f"minecraft:{source.text(arguments[0]).strip('\"')}"
    if not ids:
        raise BlockSoundExtractionError(f"{source.relative_path}: no sound events found")
    return ids


def extract_sound_types(
    source: JavaSource, sound_events: JavaSource
) -> tuple[SoundTypeRow, ...]:
    """Every `SoundType` constant, in the order `SoundType.java` declares them."""

    event_ids = _sound_event_ids(sound_events)
    rows: list[SoundTypeRow] = []
    for field in source.fields_for(SOUND_TYPE_OWNER):
        if field.type_spelling != "SoundType":
            continue
        initializer = field.initializer
        if initializer is None or initializer.node.type != "object_creation_expression":
            _fail(source, field.node, f"SoundType.{field.name} is not a constructor call")
        arguments = _arguments(source, initializer.node)
        if len(arguments) != 7:
            _fail(
                source,
                initializer.node,
                f"SoundType.{field.name} has {len(arguments)} arguments, not 7",
            )
        constants = []
        for argument in arguments[2:]:
            text = source.text(argument).strip()
            if not text.startswith("SoundEvents."):
                _fail(source, argument, f"SoundType.{field.name} sound is not SoundEvents.*")
            constants.append(text.split(".", 1)[1])
        events = []
        for constant_name in constants:
            if constant_name not in event_ids:
                _fail(
                    source,
                    initializer.node,
                    f"SoundType.{field.name} names unknown SoundEvents.{constant_name}",
                )
            events.append(event_ids[constant_name])
        rows.append(
            SoundTypeRow(
                name=field.name,
                volume=_number_literal(source, arguments[0]),
                pitch=_number_literal(source, arguments[1]),
                events=tuple(events),
            )
        )
    if not rows:
        raise BlockSoundExtractionError(f"{source.relative_path}: no sound types found")
    return tuple(rows)


def _record_constant_sound_types(
    source: JavaSource, owner: str, indices: dict[int, str]
) -> dict[str, dict[int, str]]:
    """The `SoundType` arguments of one `BlockSetType`/`WoodType` record."""

    simple_name = owner.rsplit(".", 1)[-1]
    rows: dict[str, dict[int, str]] = {}
    for field in source.fields_for(owner):
        if field.type_spelling != simple_name:
            continue
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            _fail(source, field.node, f"{simple_name}.{field.name} is not register(new ...)")
        arguments = _arguments(source, initializer.node)
        if len(arguments) != 1 or arguments[0].type != "object_creation_expression":
            _fail(source, initializer.node, f"{simple_name}.{field.name} is not new {simple_name}")
        creation = _arguments(source, arguments[0])
        resolved: dict[int, str] = {}
        for index, label in indices.items():
            if index >= len(creation):
                continue
            text = source.text(creation[index]).strip()
            match = SOUND_TYPE_CONSTANT.match(text)
            if match is None:
                _fail(source, creation[index], f"{simple_name}.{field.name} {label} is not SoundType.*")
            resolved[index] = match.group(1)
        if field.name in rows:
            _fail(source, initializer.node, f"duplicate {simple_name}.{field.name}")
        rows[field.name] = resolved
    if not rows:
        raise BlockSoundExtractionError(f"{source.relative_path}: no {simple_name} constants")
    return rows


class BlockSoundResolver:
    """`BlockState.getSoundType()` for every block `Blocks.java` registers."""

    def __init__(
        self,
        source_root: Path,
        blocks: JavaSource,
        sound_types: tuple[SoundTypeRow, ...],
        block_sets: dict[str, dict[int, str]],
        wood_types: dict[str, dict[int, str]],
        fields_by_name: dict[str, object],
        factory_by_field: dict[str, str],
        aliases: dict[str, str],
    ) -> None:
        self._root = source_root
        self._blocks = blocks
        self._sound_type_names = {row.name for row in sound_types}
        self._block_sets = block_sets
        self._wood_types = wood_types
        self._fields = fields_by_name
        self._factory = factory_by_field
        self._aliases = aliases
        self._helpers: dict[str, list] = {}
        for method in blocks.methods_for(BLOCKS_OWNER):
            if method.body is None or method.name not in PROPERTY_HELPERS:
                continue
            self._helpers.setdefault(method.name, []).append(method)
        self._class_paths = {
            path.stem: path for path in (source_root / BLOCK_CLASS_ROOT).rglob("*.java")
        }
        self._class_cache: dict[str, JavaSource | None] = {}
        self._ctor_sound: dict[str, tuple[str, str] | None] = {}
        self._resolved: dict[str, str] = {}
        self._resolving: set[str] = set()

    def sound_type_for_field(self, field: str) -> str:
        if field in self._resolved:
            return self._resolved[field]
        if field in self._resolving:
            raise BlockSoundExtractionError(f"Blocks.{field} copies itself")
        self._resolving.add(field)
        try:
            resolved = self._resolve_field(field)
        finally:
            self._resolving.discard(field)
        self._resolved[field] = resolved
        return resolved

    def _resolve_field(self, field: str) -> str:
        field = self._aliases.get(field, field)
        declaration = self._fields.get(field)
        if declaration is None:
            raise BlockSoundExtractionError(f"Blocks.{field} is not a registration")
        node = declaration.initializer.node
        try:
            return self._from_node(node, self._blocks, {})
        except NoSoundAssignment:
            pass
        return self._from_constructor(field, node)

    def _from_node(
        self, node, source: JavaSource, bindings: dict[str, tuple]
    ) -> str:
        for invocation, name in _invocations(source, node, {"sound"}):
            arguments = _arguments(source, invocation)
            if len(arguments) != 1:
                _fail(source, invocation, "sound(...) does not take one argument")
            return self._sound_argument(arguments[0], source, bindings)
        for invocation, name in _invocations(source, node, {"ofFullCopy", "ofLegacyCopy"}):
            arguments = _arguments(source, invocation)
            if len(arguments) != 1:
                _fail(source, invocation, f"{source.text(name)}(...) does not take one argument")
            return self._copy_source(arguments[0], source, bindings)
        for invocation, name in _invocations(source, node, set(self._helpers)):
            helper_name = source.text(name)
            arguments = _arguments(source, invocation)
            for method in self._helpers[helper_name]:
                if len(method.parameters) != len(arguments):
                    continue
                inner = {
                    self._parameter_name(parameter): (argument, source, bindings)
                    for parameter, argument in zip(method.parameters, arguments)
                }
                body = method.node.child_by_field_name("body")
                if body is None:
                    continue
                try:
                    return self._from_node(body, source, inner)
                except NoSoundAssignment:
                    continue
        raise NoSoundAssignment(f"{source.relative_path}: no sound assignment found")

    def _copy_source(self, node, source: JavaSource, bindings: dict[str, tuple]) -> str:
        text = source.text(node).strip()
        bound = bindings.get(text)
        if bound is not None:
            argument, bound_source, bound_bindings = bound
            argument_text = bound_source.text(argument).strip()
            if argument_text in self._fields:
                return self.sound_type_for_field(argument_text)
            return self._from_node(argument, bound_source, bound_bindings)
        field = text.rsplit(".", 1)[-1]
        if field in self._fields:
            return self.sound_type_for_field(field)
        raise BlockSoundExtractionError(f"unsupported properties copy source {text!r}")

    def _sound_argument(self, node, source: JavaSource, bindings: dict[str, tuple]) -> str:
        text = source.text(node).strip()
        constant = SOUND_TYPE_CONSTANT.match(text)
        if constant is not None:
            if constant.group(1) not in self._sound_type_names:
                raise BlockSoundExtractionError(f"unknown SoundType.{constant.group(1)}")
            return constant.group(1)
        accessor = ACCESSOR_CALL.match(text)
        if accessor is not None:
            bound = bindings.get(accessor.group(1))
            if bound is None:
                raise BlockSoundExtractionError(f"unbound constructor parameter in {text!r}")
            argument, bound_source, bound_bindings = bound
            return self._group_argument(argument, bound_source, bound_bindings, accessor.group(2))
        bound = bindings.get(text)
        if bound is not None:
            argument, bound_source, bound_bindings = bound
            return self._sound_argument(argument, bound_source, bound_bindings)
        raise BlockSoundExtractionError(f"unsupported sound argument {text!r}")

    def _group_argument(
        self, node, source: JavaSource, bindings: dict[str, tuple], accessor: str
    ) -> str:
        text = source.text(node).strip()
        constant = SOUND_TYPE_CONSTANT.match(text)
        if constant is not None:
            return self._sound_argument(node, source, bindings)
        group = GROUP_CONSTANT.match(text)
        if group is None:
            raise BlockSoundExtractionError(f"unsupported wood family argument {text!r}")
        owner, name = group.group(1), group.group(2)
        if owner == "BlockSetType":
            row = self._block_sets.get(name)
            if row is None:
                raise BlockSoundExtractionError(f"unknown BlockSetType.{name}")
            return row.get(BLOCK_SET_TYPE_SOUND_INDEX) or BLOCK_SET_TYPE_DEFAULT
        row = self._wood_types.get(name)
        if row is None:
            raise BlockSoundExtractionError(f"unknown WoodType.{name}")
        if accessor == HANGING_SIGN_ACCESSOR:
            return row.get(WOOD_TYPE_HANGING_SIGN_SOUND_INDEX) or WOOD_TYPE_DEFAULT[1]
        return row.get(WOOD_TYPE_SOUND_INDEX) or WOOD_TYPE_DEFAULT[0]

    def _from_constructor(self, field: str, node) -> str:
        factory = self._factory[field]
        assignment = self.constructor_sound(factory)
        if assignment is None:
            return DEFAULT_SOUND_TYPE
        kind, value = assignment
        if kind == "literal":
            return value
        creation = None
        for candidate in descendants(node):
            if candidate.type != "object_creation_expression":
                continue
            name = candidate.child_by_field_name("type")
            if name is not None and self._blocks.text(name).rsplit(".", 1)[-1] == factory:
                creation = candidate
        if creation is None:
            raise BlockSoundExtractionError(f"Blocks.{field} has no new {factory}(...) call")
        arguments = _arguments(self._blocks, creation)
        candidates = [
            argument
            for argument in arguments
            if GROUP_CONSTANT.match(self._blocks.text(argument).strip())
        ]
        if len(candidates) != 1:
            raise BlockSoundExtractionError(
                f"Blocks.{field}: {factory} takes its sound from a wood family, "
                f"but the registration hands it {len(candidates)}"
            )
        return self._group_argument(candidates[0], self._blocks, {}, value)

    def constructor_sound(self, class_name: str) -> tuple[str, str] | None:
        """The `sound(...)` a block class constructor applies to its properties.

        `("literal", SOUND_TYPE)` when the body names the group itself, and
        `("accessor", "soundType"|"hangingSignSoundType")` when it takes the one
        its `BlockSetType`/`WoodType` constructor argument answers -- which is
        how every wood-family block gets its group
        (`DoorBlock.java:61`, `CeilingHangingSignBlock.java:62`, ...).
        """

        if class_name in self._ctor_sound:
            return self._ctor_sound[class_name]
        found: tuple[str, str] | None = None
        for source, owner in self._class_chain(class_name):
            for method in source.methods_for(owner):
                if not method.is_constructor:
                    continue
                for invocation, name in _invocations(source, method.node, {"sound"}):
                    arguments = _arguments(source, invocation)
                    if len(arguments) != 1:
                        continue
                    text = source.text(arguments[0]).strip()
                    constant = SOUND_TYPE_CONSTANT.match(text)
                    if constant is not None:
                        found = ("literal", constant.group(1))
                        break
                    accessor = ACCESSOR_CALL.match(text)
                    if accessor is None:
                        continue
                    parameter = self._parameter_type(method, source, accessor.group(1))
                    if parameter in {"BlockSetType", "WoodType"}:
                        found = ("accessor", accessor.group(2))
                        break
                if found is not None:
                    break
            if found is not None:
                break
        self._ctor_sound[class_name] = found
        return found

    def has_state_dependent_sound(self, class_name: str) -> bool:
        """Whether the class chain overrides `getSoundType(BlockState)`."""

        for source, owner in self._class_chain(class_name):
            for method in source.methods_for(owner):
                if method.name != GET_SOUND_TYPE_METHOD or method.is_constructor:
                    continue
                parameters = [
                    source.text(parameter.node.child_by_field_name("type"))
                    for parameter in method.parameters
                ]
                if parameters == [BLOCK_STATE_PARAMETER]:
                    return True
        return False

    @staticmethod
    def _parameter_parts(parameter: JavaExpression) -> tuple[str, str]:
        """A parameter's type and name, as `final SoundType soundType` spells them."""

        tokens = [
            token
            for token in parameter.canonical.split(" ")
            if token and token not in {"final", "@Nullable", "@Override"}
        ]
        if len(tokens) < 2:
            raise BlockSoundExtractionError(f"parameter {parameter.canonical!r} is incomplete")
        return tokens[-2], tokens[-1]

    @classmethod
    def _parameter_name(cls, parameter: JavaExpression) -> str:
        return cls._parameter_parts(parameter)[1]

    def _parameter_type(self, method, source: JavaSource, name: str) -> str | None:
        for parameter in method.parameters:
            parameter_type, parameter_name = self._parameter_parts(parameter)
            if parameter_name != name:
                continue
            return parameter_type.rsplit(".", 1)[-1].split("<", 1)[0]
        return None

    def _class_chain(self, class_name: str):
        current = class_name
        while current not in {"BlockBehaviour", "Block", "Object"}:
            source = self._class_source(current)
            if source is None:
                return
            owner = None
            for java_type in source.types:
                if java_type.simple_name == current:
                    owner = java_type
            if owner is None:
                return
            yield source, owner.fqcn
            spelling = (owner.superclass_spelling or "").replace(" ", "")
            current = spelling.rsplit(".", 1)[-1].split("<", 1)[0]

    def _class_source(self, class_name: str) -> JavaSource | None:
        if class_name in self._class_cache:
            return self._class_cache[class_name]
        path = self._class_paths.get(class_name)
        source = (
            parse_source(f"{BLOCK_CLASS_ROOT}/{class_name}.java", path.read_bytes())
            if path is not None
            else None
        )
        self._class_cache[class_name] = source
        return source


def _sound_rows(
    resolver: BlockSoundResolver,
    declarations: Sequence[dict],
    states: Sequence[dict],
) -> tuple[tuple[BlockSoundRow, ...], tuple[StateSoundRow, ...]]:
    """Every block's group, plus the states that answer a group of their own.

    A class that overrides `getSoundType(BlockState)` is authoritative for its
    block: the override's else branch is what its false-value states answer, so
    that is the block row, and its true-value states become override rows.
    """

    rows: list[BlockSoundRow] = []
    overrides: dict[int, str] = {}
    for index, declaration in enumerate(declarations):
        field = declaration["field"]
        resource_id = declaration["resource_id"]
        factory = block_factory_class(declaration)
        if resolver.has_state_dependent_sound(factory):
            property_name, then_sound, else_sound = _state_split(resolver, factory)
            sound_type = else_sound
            matched = 0
            for state in states:
                if state["name"] != resource_id:
                    continue
                value = state["properties"].get(property_name)
                if value is None:
                    raise BlockSoundExtractionError(
                        f"{resource_id} has no {property_name!r} property but its "
                        f"getSoundType override reads one"
                    )
                if value not in {"true", "false"}:
                    raise BlockSoundExtractionError(
                        f"{resource_id}.{property_name} is {value!r}, not a boolean"
                    )
                if value == "true":
                    overrides[state["id"]] = then_sound
                matched += 1
            if matched == 0:
                raise BlockSoundExtractionError(f"{resource_id} has no built-in states")
        else:
            sound_type = resolver.sound_type_for_field(field)
        rows.append(
            BlockSoundRow(
                block_type_id=index,
                resource_id=resource_id,
                sound_type=sound_type,
            )
        )
    overrides_rows = tuple(
        StateSoundRow(state_id=state_id, sound_type=sound)
        for state_id, sound in sorted(overrides.items())
    )
    return tuple(rows), overrides_rows


def _state_split(resolver: BlockSoundResolver, class_name: str) -> tuple[str, str, str]:
    """The property name and the two branches of a `getSoundType` override."""

    for source, owner in resolver._class_chain(class_name):
        for method in source.methods_for(owner):
            if method.name != GET_SOUND_TYPE_METHOD or method.is_constructor:
                continue
            parameters = [
                source.text(parameter.node.child_by_field_name("type"))
                for parameter in method.parameters
            ]
            if parameters != [BLOCK_STATE_PARAMETER]:
                continue
            body = method.node.child_by_field_name("body")
            if body is None:
                _fail(source, method.node, f"{class_name}.{GET_SOUND_TYPE_METHOD} has no body")
            returns = [
                candidate
                for candidate in descendants(body)
                if candidate.type == "return_statement"
            ]
            if len(returns) != 1:
                _fail(source, returns[0] if returns else body, "override has no single return")
            value = returns[0].named_children[0]
            if value.type != "ternary_expression":
                _fail(source, value, "override is not a conditional on one property")
            condition = value.child_by_field_name("condition")
            consequence = value.child_by_field_name("consequence")
            alternative = value.child_by_field_name("alternative")
            match = BLOCK_STATE_PROPERTY.match(source.text(condition).strip())
            then_sound = SOUND_TYPE_CONSTANT.match(source.text(consequence).strip())
            else_sound = SOUND_TYPE_CONSTANT.match(source.text(alternative).strip())
            if match is None or then_sound is None or else_sound is None:
                _fail(source, value, "override is not `state.getValue(P) ? A : B`")
            return (
                match.group(1).lower(),
                then_sound.group(1),
                else_sound.group(1),
            )
    raise BlockSoundExtractionError(f"{class_name} has no state-dependent {GET_SOUND_TYPE_METHOD}")


def _weathering_aliases(declarations: Sequence[dict]) -> dict[str, str]:
    """`WeatheringCopperBlocks.create`'s seven derived fields.

    All eight variants share the one `propertiesSupplier` the registration
    hands `create`, so a variant's group is its unaffected sibling's.
    """

    unaffected = {
        declaration["resource_id"]: declaration["field"]
        for declaration in declarations
        if declaration.get("group_variant") == "unaffected"
    }
    prefixes = {
        variant: prefix for variant, prefix, _ in WEATHERING_COPPER_VARIANTS
    }
    aliases: dict[str, str] = {}
    for declaration in declarations:
        variant = declaration.get("group_variant")
        if variant is None or variant == "unaffected":
            continue
        prefix = prefixes[variant]
        resource_id = declaration["resource_id"]
        base = f"minecraft:{resource_id[len('minecraft:') + len(prefix):]}"
        if not resource_id.startswith(f"minecraft:{prefix}") or base not in unaffected:
            raise BlockSoundExtractionError(
                f"{resource_id} has no unaffected WeatheringCopperBlocks sibling"
            )
        aliases[declaration["field"]] = unaffected[base]
    return aliases


def extract(source_root: Path, block_states: Path) -> BlockSoundCatalog:
    blocks = parse_source(BLOCKS_PATH, (source_root / BLOCKS_PATH).read_bytes())
    sound_type_source = parse_source(
        SOUND_TYPE_PATH, (source_root / SOUND_TYPE_PATH).read_bytes()
    )
    sound_events = parse_source(
        SOUND_EVENTS_PATH, (source_root / SOUND_EVENTS_PATH).read_bytes()
    )
    block_sets = _record_constant_sound_types(
        parse_source(BLOCK_SET_TYPE_PATH, (source_root / BLOCK_SET_TYPE_PATH).read_bytes()),
        BLOCK_SET_TYPE_OWNER,
        {BLOCK_SET_TYPE_SOUND_INDEX: "soundType"},
    )
    wood_types = _record_constant_sound_types(
        parse_source(WOOD_TYPE_PATH, (source_root / WOOD_TYPE_PATH).read_bytes()),
        WOOD_TYPE_OWNER,
        {
            WOOD_TYPE_SOUND_INDEX: "soundType",
            WOOD_TYPE_HANGING_SIGN_SOUND_INDEX: "hangingSignSoundType",
        },
    )
    block_ids = parse_source(BLOCK_IDS_PATH, (source_root / BLOCK_IDS_PATH).read_bytes())
    block_entity_types = parse_source(
        BLOCK_ENTITY_TYPES_PATH, (source_root / BLOCK_ENTITY_TYPES_PATH).read_bytes()
    )
    sound_types = extract_sound_types(sound_type_source, sound_events)
    catalog = extract_block_registry_catalog(blocks, block_ids, block_entity_types)
    declarations = catalog.declarations

    fields_by_name = {
        field.name: field
        for field in blocks.fields_for(BLOCKS_OWNER)
        if field.initializer is not None
    }
    factory_by_field = {
        declaration["field"]: block_factory_class(declaration)
        for declaration in declarations
    }
    aliases = _weathering_aliases(declarations)
    resolver = BlockSoundResolver(
        source_root,
        blocks,
        sound_types,
        block_sets,
        wood_types,
        fields_by_name,
        {
            field: factory_by_field[aliases.get(field, field)]
            for field, factory in factory_by_field.items()
        },
        aliases,
    )
    report = json.loads(block_states.read_text(encoding="utf-8"))
    states = report["states"]
    rows, overrides = _sound_rows(resolver, declarations, states)
    return BlockSoundCatalog(
        sound_types=sound_types, blocks=rows, overrides=overrides
    )


def assert_catalog_matches_runtime_order(catalog: BlockSoundCatalog, block_states: Path) -> None:
    """The emitted ids are the runtime registry's own order.

    `BuiltInRegistries.BLOCK` hands ids out in `Blocks.java` declaration order,
    and the embedded built-in state catalog lists block states in that same
    order, so the order the distinct block names first appear in is the block
    registry order. A generator that stopped agreeing with it would be writing
    ids that name a different block.
    """

    report = json.loads(block_states.read_text(encoding="utf-8"))
    states = report["states"]
    if len(states) != EXPECTED_STATES:
        raise BlockSoundExtractionError(
            f"embedded block-state report holds {len(states)} states, not {EXPECTED_STATES}"
        )
    order: list[str] = []
    seen: set[str] = set()
    for state in states:
        if state["name"] not in seen:
            seen.add(state["name"])
            order.append(state["name"])
    declared = [row.resource_id for row in catalog.blocks]
    if order != declared:
        mismatch = next(
            (
                (index, expected, actual)
                for index, (expected, actual) in enumerate(zip(order, declared))
                if expected != actual
            ),
            (min(len(order), len(declared)), "", ""),
        )
        raise BlockSoundExtractionError(
            "block declaration order does not match the state catalog: "
            f"index {mismatch[0]} is {mismatch[1]!r} in the catalog and "
            f"{mismatch[2]!r} in Blocks.java"
        )


def render_catalog(catalog: BlockSoundCatalog) -> str:
    if len(catalog.sound_types) != EXPECTED_SOUND_TYPES:
        raise BlockSoundExtractionError(
            f"expected {EXPECTED_SOUND_TYPES} sound types, got {len(catalog.sound_types)}"
        )
    if len(catalog.blocks) != EXPECTED_BLOCKS:
        raise BlockSoundExtractionError(
            f"expected {EXPECTED_BLOCKS} blocks, got {len(catalog.blocks)}"
        )
    index_by_name = {row.name: index for index, row in enumerate(catalog.sound_types)}
    for row in catalog.blocks:
        if row.sound_type not in index_by_name:
            raise BlockSoundExtractionError(f"{row.resource_id} names {row.sound_type}")
    for row in catalog.overrides:
        if row.sound_type not in index_by_name:
            raise BlockSoundExtractionError(f"state {row.state_id} names {row.sound_type}")

    lines = [
        "// @generated by scripts/generate-server-block-sound-facts.py; do not edit manually.",
        "// Vanilla 26.1: SoundType.java:7-818 (the SoundType constants),",
        "// SoundEvents.java registrations, Blocks.java sound assignments and",
        "// BlockBehaviour.java:973 (the SoundType.STONE default).",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-server-block-sound-facts.py",
        "// Check: scripts/vanilla-source-tools.sh "
        "scripts/generate-server-block-sound-facts.py --check",
        "",
        "use crate::block_state_types::{",
        "    vanilla_block_state_block_type_id, vanilla_block_state_id,",
        "};",
        "",
        "/// `SoundType` (`SoundType.java:6-844`): the two shared factors and the",
        "/// five events `BlockState.getSoundType()` hands back.",
        "#[derive(Clone, Copy, Debug, PartialEq)]",
        "pub struct VanillaBlockSoundType26_1 {",
        "    /// `SoundType.getVolume()` (`SoundType.java:846-848`).",
        "    pub volume: f32,",
        "    /// `SoundType.getPitch()` (`SoundType.java:850-852`).",
        "    pub pitch: f32,",
        "    /// `SoundType.getBreakSound()` (`SoundType.java:854-856`).",
        "    pub break_sound: &'static str,",
        "    /// `SoundType.getStepSound()` (`SoundType.java:858-860`).",
        "    pub step_sound: &'static str,",
        "    /// `SoundType.getPlaceSound()` (`SoundType.java:862-864`).",
        "    pub place_sound: &'static str,",
        "    /// `SoundType.getHitSound()` (`SoundType.java:866-868`).",
        "    pub hit_sound: &'static str,",
        "    /// `SoundType.getFallSound()` (`SoundType.java:870-872`).",
        "    pub fall_sound: &'static str,",
        "}",
        "",
        "/// Every `SoundType` constant `SoundType.java` declares, in declaration",
        "/// order.",
        "#[rustfmt::skip]",
        "pub const VANILLA_BLOCK_SOUND_TYPES_26_1: &[VanillaBlockSoundType26_1] = &[",
    ]
    for row in catalog.sound_types:
        lines.append(
            "    VanillaBlockSoundType26_1 { "
            f"volume: {row.volume}, pitch: {row.pitch}, "
            f'break_sound: "{row.events[0]}", step_sound: "{row.events[1]}", '
            f'place_sound: "{row.events[2]}", hit_sound: "{row.events[3]}", '
            f'fall_sound: "{row.events[4]}" }},  // {row.name}'
        )
    lines.extend(
        [
            "];",
            "",
            "/// One built-in block's `BlockBehaviour.getSoundType`",
            "/// (`BlockBehaviour.java:403-405`, `:871-873`) as an index into",
            "/// [`VANILLA_BLOCK_SOUND_TYPES_26_1`].",
            "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
            "pub struct VanillaBlockSoundGroupFacts26_1 {",
            "    /// The block registry id (`BuiltInRegistries.BLOCK`).",
            "    pub block_type_id: i32,",
            "    pub resource_id: &'static str,",
            "    /// The `SoundType` index this block answers for every state.",
            "    pub sound_type: u16,",
            "}",
            "",
            "/// Every built-in block's sound group, by block registry id.",
            "///",
            f"/// All {len(catalog.blocks)} blocks `Blocks.java` registers resolve; none falls",
            "/// back on a default this table does not name.",
            "#[rustfmt::skip]",
            "pub const VANILLA_BLOCK_SOUND_GROUP_FACTS_26_1: &[VanillaBlockSoundGroupFacts26_1] = &[",
        ]
    )
    for row in catalog.blocks:
        lines.append(
            "    VanillaBlockSoundGroupFacts26_1 { "
            f"block_type_id: {row.block_type_id}, "
            f'resource_id: "{row.resource_id}", '
            f"sound_type: {index_by_name[row.sound_type]} }},"
        )
    lines.extend(
        [
            "];",
            "",
            "/// The states whose `getSoundType` answers a group of their own.",
            "///",
            "/// `DecoratedPotBlock.getSoundType` (`DecoratedPotBlock.java:212-214`)",
            "/// reads `CRACKED`, so a `minecraft:decorated_pot` state is not",
            "/// answered by its block row alone. Sorted by state id.",
            "#[rustfmt::skip]",
            "pub const VANILLA_BLOCK_STATE_SOUND_GROUP_OVERRIDES_26_1: &[(i32, u16)] = &[",
        ]
    )
    for row in catalog.overrides:
        lines.append(
            f"    ({row.state_id}, {index_by_name[row.sound_type]}),  // {row.sound_type}"
        )
    lines.extend(
        [
            "];",
            "",
            "/// The number of built-in blocks [`VANILLA_BLOCK_SOUND_GROUP_FACTS_26_1`] covers.",
            f"pub const VANILLA_BLOCK_SOUND_GROUP_BLOCK_COUNT_26_1: usize = {len(catalog.blocks)};",
            "",
            "/// `SoundType.STONE`, the `BlockBehaviour.Properties` default.",
            f"const VANILLA_BLOCK_SOUND_TYPE_STONE_26_1: u16 = {index_by_name[DEFAULT_SOUND_TYPE]};",
            "",
            "/// A block sound group by its index in",
            "/// [`VANILLA_BLOCK_SOUND_TYPES_26_1`].",
            "#[must_use]",
            "pub fn vanilla_block_sound_type_26_1(index: u16) -> Option<&'static VanillaBlockSoundType26_1> {",
            "    VANILLA_BLOCK_SOUND_TYPES_26_1.get(usize::from(index))",
            "}",
            "",
            "/// A built-in block's group facts by its block registry id.",
            "#[must_use]",
            "pub fn vanilla_block_sound_group_26_1(",
            "    block_type_id: i32,",
            ") -> Option<&'static VanillaBlockSoundGroupFacts26_1> {",
            "    let index = usize::try_from(block_type_id).ok()?;",
            "    VANILLA_BLOCK_SOUND_GROUP_FACTS_26_1",
            "        .get(index)",
            "        .filter(|facts| facts.block_type_id == block_type_id)",
            "}",
            "",
            "/// A built-in block's group facts by block resource id.",
            "#[inline]",
            "#[must_use]",
            "pub fn vanilla_block_sound_group_by_resource_id_26_1(",
            "    resource_id: &str,",
            ") -> Option<&'static VanillaBlockSoundGroupFacts26_1> {",
            "    crate::lookup_tables::block_sound_group_by_resource_id_26_1(resource_id)",
            "}",
            "",
            "/// `BlockState.getSoundType()` (`BlockBehaviour.java:403-405`); an unknown",
            "/// state answers the `SoundType.STONE` default.",
            "#[must_use]",
            "pub fn vanilla_block_state_sound_type_26_1(block_state_id: i32) -> &'static VanillaBlockSoundType26_1 {",
            "    let index = vanilla_block_state_id(block_state_id).and_then(|state| {",
            "        match VANILLA_BLOCK_STATE_SOUND_GROUP_OVERRIDES_26_1.binary_search_by_key(&state, |row| row.0) {",
            "            Ok(index) => Some(VANILLA_BLOCK_STATE_SOUND_GROUP_OVERRIDES_26_1[index].1),",
            "            Err(_) => vanilla_block_state_block_type_id(state)",
            "                .and_then(vanilla_block_sound_group_26_1)",
            "                .map(|facts| facts.sound_type),",
            "        }",
            "    });",
            "    &VANILLA_BLOCK_SOUND_TYPES_26_1[usize::from(index.unwrap_or(VANILLA_BLOCK_SOUND_TYPE_STONE_26_1))]",
            "}",
            "",
            "/// `BlockItem.place`'s own sound (`BlockItem.java:86-87`): the placed",
            "/// state's place event, `(soundType.getVolume() + 1.0F) / 2.0F` and",
            "/// `soundType.getPitch() * 0.8F`.",
            "#[derive(Clone, Copy, Debug, PartialEq)]",
            "pub struct VanillaBlockPlaceSound26_1 {",
            "    pub sound: &'static str,",
            "    pub volume: f32,",
            "    pub pitch: f32,",
            "}",
            "",
            "/// The place sound `BlockItem.place` plays for one built-in 26.1 state.",
            "#[must_use]",
            "pub fn vanilla_block_place_sound_26_1(block_state_id: i32) -> VanillaBlockPlaceSound26_1 {",
            "    let sound_type = vanilla_block_state_sound_type_26_1(block_state_id);",
            "    VanillaBlockPlaceSound26_1 {",
            "        sound: sound_type.place_sound,",
            "        volume: (sound_type.volume + 1.0) / 2.0,",
            "        pitch: sound_type.pitch * 0.8,",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use super::*;",
            "",
            "    /// A group whose volume and pitch are not both 1.0, and the arithmetic",
            "    /// `BlockItem.place` does on them.",
            "    #[test]",
            "    fn the_place_arithmetic_reads_the_groups_own_factors() {",
            '        let anvil = vanilla_block_sound_group_by_resource_id_26_1("minecraft:anvil")',
            "            .expect(\"the anvil has a group\");",
            "        let group = vanilla_block_sound_type_26_1(anvil.sound_type).unwrap();",
            '        assert_eq!(group.place_sound, "minecraft:block.anvil.place");',
            "        assert_eq!(group.volume, 0.3);",
            "        assert_eq!(group.pitch, 1.0);",
            "        let default_state =",
            "            crate::block_state_types::vanilla_block_default_state_id(anvil.block_type_id)",
            "                .expect(\"anvil has a default state\");",
            "        let placed = vanilla_block_place_sound_26_1(default_state);",
            '        assert_eq!(placed.sound, "minecraft:block.anvil.place");',
            "        assert_eq!(placed.volume, 0.65);",
            "        assert_eq!(placed.pitch, 0.8);",
            "    }",
            "",
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
        raise BlockSoundExtractionError(
            "rustfmt rejected generated source:\n" + formatted.stderr
        )
    return formatted.stdout


def render(source_root: Path, block_states: Path) -> str:
    catalog = extract(source_root, block_states)
    assert_catalog_matches_runtime_order(catalog, block_states)
    return render_catalog(catalog)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    generated = render(args.source_root, args.block_states)
    current = args.output.read_text(encoding="utf-8") if args.output.exists() else ""
    if args.check:
        if current == generated:
            return 0
        sys.stderr.writelines(
            difflib.unified_diff(
                current.splitlines(keepends=True),
                generated.splitlines(keepends=True),
                fromfile=str(args.output),
                tofile=str(args.output) + " (generated)",
            )
        )
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
