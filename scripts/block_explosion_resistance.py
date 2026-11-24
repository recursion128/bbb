"""Extract 26.1 block explosion resistance from registration expressions."""

from __future__ import annotations

import re
import struct
from typing import Any

from block_registry_catalog_extractor import block_factory_class
from vanilla_java_source import JavaSource, expression_fact, resolve_class_hierarchy


class ExplosionResistanceExtractionError(ValueError):
    pass


BLOCKS = "net.minecraft.world.level.block.Blocks"
PROPERTIES = "net.minecraft.world.level.block.state.BlockBehaviour.Properties"
def _fail(message: str) -> None:
    raise ExplosionResistanceExtractionError(message)


def _compact(text: str) -> str:
    return re.sub(r"\s+", "", text)


def _return_fact(source: JavaSource, method) -> dict[str, Any]:
    body = method.node.child_by_field_name("body")
    if body is None or len(body.named_children) != 1:
        _fail(f"{source.relative_path}: {method.name} is not a single return")
    returned = body.named_children[0]
    if returned.type != "return_statement" or len(returned.named_children) != 1:
        _fail(f"{source.relative_path}: {method.name} has no single return value")
    return expression_fact(source, source.expression(returned.named_children[0]))


def _f32_bits(value: float) -> int:
    try:
        return struct.unpack(">I", struct.pack(">f", value))[0]
    except OverflowError as error:
        _fail(f"explosion resistance is not finite f32: {value}: {error}")


def _number(fact: dict[str, Any]) -> float:
    value = fact["canonical"].replace("_", "")
    if fact["kind"] == "unary_expression" and value.startswith("-"):
        return -_number(fact["children"][-1])
    if fact["kind"] not in {
        "decimal_floating_point_literal",
        "decimal_integer_literal",
    }:
        _fail(f"unsupported explosion resistance number {value}")
    if value.endswith(("F", "f", "D", "d")):
        value = value[:-1]
    try:
        return struct.unpack(">f", struct.pack(">f", float(value)))[0]
    except (ValueError, OverflowError) as error:
        _fail(f"invalid explosion resistance number {value}: {error}")


def _field(fact: dict[str, Any], env: dict[str, str]) -> str:
    name = fact["canonical"].replace(" ", "")
    if name.startswith("Blocks."):
        name = name.removeprefix("Blocks.")
    name = env.get(name, name)
    if not re.fullmatch(r"[A-Z][A-Z0-9_]*", name):
        _fail(f"unsupported block reference {fact['canonical']}")
    return name


def _verify_sources(properties: JavaSource, stairs: JavaSource) -> set[str]:
    source = properties.source.decode()
    required = (
        "private float explosionResistance;",
        "this.explosionResistance = properties.explosionResistance;",
        "BlockBehaviour.Properties copyTo = ofLegacyCopy(block);",
        "copyTo.explosionResistance = copyFrom.explosionResistance;",
        "return this.destroyTime(destroyTime).explosionResistance(explosionResistance);",
        "this.strength(destroyTime, destroyTime);",
        "return this.strength(0.0F);",
        "this.explosionResistance = Math.max(0.0F, explosionResistance);",
    )
    compact = _compact(source)
    for fragment in required:
        if _compact(fragment) not in compact:
            _fail(f"{properties.relative_path}: explosion resistance semantics changed: {fragment}")
    if compact.count("copyTo.explosionResistance=copyFrom.explosionResistance;") != 1:
        _fail(f"{properties.relative_path}: copy semantics changed")
    stair_source = _compact(stairs.source.decode())
    for fragment in (
        "this.base = baseState.getBlock();",
        "return this.base.getExplosionResistance();",
    ):
        if _compact(fragment) not in stair_source:
            _fail(f"{stairs.relative_path}: StairBlock resistance override changed")
    safe_by_name: dict[str, bool] = {}
    for method in properties.methods_for(PROPERTIES):
        if method.body is None or method.name in {
            "strength",
            "instabreak",
            "explosionResistance",
            "of",
            "ofLegacyCopy",
            "ofFullCopy",
        }:
            continue
        body = properties.source[method.body.start_byte : method.body.end_byte].decode()
        unchanged = not re.search(
            r"\b(?:explosionResistance|strength|instabreak|ofLegacyCopy|ofFullCopy)\s*\(",
            body,
        ) and "explosionResistance" not in body
        safe_by_name[method.name] = safe_by_name.get(method.name, True) and unchanged
    return {name for name, unchanged in safe_by_name.items() if unchanged}


def extract_block_explosion_resistances(
    declarations: list[dict[str, Any]],
    blocks: JavaSource,
    properties: JavaSource,
    block_sources: list[JavaSource],
) -> dict[str, int]:
    by_stem = {
        source.relative_path.rsplit("/", 1)[-1].removesuffix(".java"): source
        for source in block_sources
    }
    for required in (
        "StairBlock",
        "WeatheringCopperStairBlock",
        "WeatheringCopperBlocks",
        "InfestedBlock",
        "InfestedRotatedPillarBlock",
    ):
        if required not in by_stem:
            _fail(f"missing source for {required}")
    stairs = by_stem["StairBlock"]
    weathering = by_stem["WeatheringCopperBlocks"]
    infested = by_stem["InfestedBlock"]
    safe_methods = _verify_sources(properties, stairs)
    types = [java_type for source in block_sources for java_type in source.types]
    resolve_class_hierarchy(types)
    by_type = {java_type.simple_name: java_type for java_type in types}

    def inherits(name: str, ancestor: str) -> bool:
        seen = set()
        while name != ancestor:
            if name in seen or name not in by_type:
                return False
            seen.add(name)
            parent = by_type[name].superclass
            if parent is None:
                return False
            name = parent.rsplit(".", 1)[-1]
        return True

    constructor = [
        method
        for method in infested.methods_for("net.minecraft.world.level.block.InfestedBlock")
        if method.is_constructor
    ]
    if (
        len(constructor) != 1
        or (
            "super(properties.destroyTime(hostBlock.defaultDestroyTime()/2.0F)"
            ".explosionResistance(0.75F));"
        )
        not in _compact(infested.text(constructor[0].node))
    ):
        _fail(f"{infested.relative_path}: infested constructor resistance changed")
    if not inherits("InfestedRotatedPillarBlock", "InfestedBlock"):
        _fail("InfestedRotatedPillarBlock no longer inherits InfestedBlock")
    if "super(hostBlock,properties);" not in _compact(
        by_stem["InfestedRotatedPillarBlock"].source.decode()
    ):
        _fail("InfestedRotatedPillarBlock constructor no longer delegates to InfestedBlock")
    if "super(baseState,properties);" not in _compact(
        by_stem["WeatheringCopperStairBlock"].source.decode()
    ):
        _fail("WeatheringCopperStairBlock constructor no longer delegates its baseState")
    for source in block_sources:
        if source.relative_path in (
            blocks.relative_path,
            properties.relative_path,
            infested.relative_path,
        ):
            continue
        compact = _compact(source.source.decode())
        if re.search(
            r"\.(?:explosionResistance|strength|instabreak|ofLegacyCopy|ofFullCopy)\(", compact
        ):
            _fail(f"{source.relative_path}: unhandled constructor Properties mutator")
    for source in block_sources:
        for method in source.methods:
            if method.name == "getExplosionResistance" and method.owner.rsplit(".", 1)[-1] not in (
                "Block",
                "StairBlock",
            ):
                _fail(f"{source.relative_path}: unhandled resistance override")
    helper_methods = {method.name: method for method in blocks.methods_for(BLOCKS)}
    properties_values: dict[str, int] = {}
    values: dict[str, int] = {}

    def resolve(fact: dict[str, Any], env: dict[str, str], recorded: dict[str, int]) -> int:
        name = _field(fact, env)
        if name not in recorded:
            _fail(f"{blocks.relative_path}: unresolved or forward block resistance {name}")
        return recorded[name]

    def evaluate(fact: dict[str, Any], env: dict[str, str]) -> int:
        if fact["kind"] != "method_invocation":
            _fail(f"unsupported Properties expression {fact['canonical']}")
        name = fact["name"]
        args = fact["arguments"]
        parent = fact.get("object")
        if (
            name == "of"
            and parent
            and parent["canonical"].replace(" ", "") == "BlockBehaviour.Properties"
            and not args
        ):
            return _f32_bits(0.0)
        if (
            name in ("ofLegacyCopy", "ofFullCopy")
            and parent
            and parent["canonical"].replace(" ", "") == "BlockBehaviour.Properties"
            and len(args) == 1
        ):
            return resolve(args[0], env, properties_values)
        if name in helper_methods and parent is None:
            if sum(method.name == name for method in blocks.methods_for(BLOCKS)) != 1:
                _fail(f"{blocks.relative_path}: ambiguous Properties helper {name}")
            method = helper_methods[name]
            if name == "wallVariant":
                if len(args) != 2:
                    _fail(f"{blocks.relative_path}: wallVariant argument count changed")
                body = blocks.source[method.body.start_byte : method.body.end_byte].decode()
                compact = _compact(body)
                expected = (
                    "{BlockBehaviour.PropertieswallProperties="
                    "BlockBehaviour.Properties.of().overrideLootTable(standingBlock.getLootTable());"
                    "if(copyName){wallProperties=wallProperties.overrideDescription(standingBlock.getDescriptionId());}"
                    "returnwallProperties;}"
                )
                if compact != expected:
                    _fail(f"{blocks.relative_path}: wallVariant resistance changed")
                return _f32_bits(0.0)
            if len(args) != len(method.parameters):
                _fail(f"{blocks.relative_path}: {name} argument count changed")
            local = {
                parameter.canonical.split()[-1]: _field(argument, env)
                for parameter, argument in zip(method.parameters, args)
                if parameter.canonical.split()[-1] == "base"
            }
            returned = _return_fact(blocks, method)
            if returned["kind"] == "method_invocation" and returned["name"] == "register":
                returned = returned["arguments"][-1]
            return evaluate(returned, local)
        if parent is None:
            _fail(f"unsupported Properties call {fact['canonical']}")
        resistance = evaluate(parent, env)
        if name == "strength" and len(args) in (1, 2):
            return _f32_bits(max(0.0, _number(args[-1])))
        if name == "instabreak" and not args:
            return _f32_bits(0.0)
        if name == "explosionResistance" and len(args) == 1:
            return _f32_bits(max(0.0, _number(args[0])))
        if name in safe_methods:
            return resistance
        _fail(f"unverified Properties method {name}")

    weathering_text = _compact(weathering.source.decode())
    for state in ("UNAFFECTED", "EXPOSED", "WEATHERED", "OXIDIZED"):
        if (
            weathering_text.count(
                f"propertiesSupplier.apply(WeatheringCopper.WeatherState.{state})"
            )
            != 2
        ):
            _fail(f"{weathering.relative_path}: supplier mapping for {state} changed")

    for declaration in declarations:
        init = declaration["initializer"]
        helper = init["name"]
        args = init["arguments"]
        if declaration.get("group_variant"):
            if helper != "create" or len(args) != 5 or args[4]["kind"] != "lambda_expression":
                _fail(f"unsupported WeatheringCopperBlocks registration {declaration['field']}")
            supplier = args[4]["children"][-1]
            if supplier["kind"] != "method_invocation":
                _fail(f"unsupported copper properties supplier {declaration['field']}")
            property_resistance = evaluate(supplier, {})
        elif helper == "register":
            if len(args) not in (2, 3):
                _fail(f"unsupported register arity for {declaration['field']}")
            property_resistance = evaluate(args[-1], {})
        elif helper in (
            "registerLegacyStair",
            "registerStair",
            "registerBed",
            "registerStainedGlass",
        ):
            property_resistance = evaluate(init, {})
        else:
            _fail(f"unsupported block register helper {helper}")

        factory_class = block_factory_class(declaration)
        if helper in ("registerLegacyStair", "registerStair"):
            base = args[1]
        elif helper == "register" and len(args) == 3 and inherits(factory_class, "StairBlock"):
            factory = args[1]
            if (
                factory["kind"] != "lambda_expression"
                or factory["children"][-1]["kind"] != "object_creation_expression"
            ):
                _fail(f"unsupported StairBlock factory {declaration['field']}")
            base = factory["children"][-1]["arguments"][-2]
            if base.get("name") != "defaultBlockState" or base.get("object") is None:
                _fail(f"unsupported StairBlock base {declaration['field']}")
            base = base["object"]
        else:
            base = None
        if inherits(factory_class, "InfestedBlock"):
            property_resistance = _f32_bits(0.75)
        properties_values[declaration["field"]] = property_resistance
        if base is not None:
            values[declaration["field"]] = resolve(base, {}, values)
        else:
            values[declaration["field"]] = property_resistance
    if len(values) != len(declarations):
        _fail("not every registered block has an explosion resistance")
    return values
