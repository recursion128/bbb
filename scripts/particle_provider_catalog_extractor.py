"""Tree-sitter-backed extraction of vanilla 26.1 particle provider facts.

Every `ParticleProvider` in `net/minecraft/client/particle/` is resolved to the
particle class it constructs, and five client-observable facts are derived from
that class's hierarchy. The handful of particles vanilla spawns straight from
client code — the tracking emitter, the firework starter, the item-pickup
animation — get the same five facts; they are found as the concrete particles
no provider constructs and nothing else extends, which is what tells them apart
from the family bases vanilla simply never marked `abstract`.

The five facts:

- rendering: whether the particle draws at all. `NoRenderParticle` subclasses
  are emitters whose visible output belongs to the particles they spawn.
- collision: whether the particle's tick can reach a colliding `move`. Vanilla
  opts out in four mechanical ways: `hasPhysics=false` (`Particle.move` then
  only translates), a `move` override that never delegates to `Particle.move`,
  a `tick` that never calls `move`, and a velocity pinned to zero — vanilla's
  own guard skips collision when all three deltas are zero (Particle.java:150).
- player coupling: local-player / camera reads inside the particle's own tick.
- sounds: `playLocalSound` raised by the particle's own tick. Spawn-site sounds
  belong to the packet/level-event handlers, not here.
- conditional removal: a `remove()` the tick can reach under a condition that is
  neither the lifetime counter nor statically dead for this provider.

The facts are computed over the tick call graph rather than over `tick`'s body
alone, because vanilla routinely puts them in a hook the base tick calls —
`DripParticle` gates removal in `postMoveUpdate`, not in `tick`.

And the rows are per provider, not per particle class, because the provider's
construction arguments decide behaviour: `DripParticle.FallingParticle` carries
a live fluid-surface removal gate for `WaterFallProvider` and a statically dead
one for `NectarFallProvider`, which passes `Fluids.EMPTY` into the same class.
So construction-site literals are threaded up the constructor chain and folded
into the gate conditions.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re

from vanilla_java_source import (
    JavaMethod,
    JavaSource,
    JavaType,
    parse_source,
    resolve_class_hierarchy,
)


PARTICLE_PACKAGE = "net.minecraft.client.particle"
PARTICLE_DIRECTORY = "net/minecraft/client/particle"
PROVIDER_INTERFACE = "ParticleProvider"
BASE_PARTICLE = f"{PARTICLE_PACKAGE}.Particle"

# `Particle.move` is the only collision path; an override that never delegates
# to it only translates the bounding box.
MOVE_METHOD = "move"
TICK_METHOD = "tick"

# Reads that couple a particle to the local player or the camera. Matched as
# call names so a `Camera` parameter in a render signature is not mistaken for
# camera coupling.
PLAYER_CALL_NAMES = frozenset(
    {
        "getNearestPlayer",
        "getCameraEntity",
        "getCameraPosition",
        "getMainCamera",
        "getPlayerVehicleOrPlayer",
    }
)
# Naming the client player's type is itself the coupling: `SpellParticle`
# reaches it as `LocalPlayer player = Minecraft.getInstance().player`, so no
# single fused expression would catch it.
PLAYER_TYPE_RE = re.compile(r"\bLocalPlayer\b|\bMinecraft\.getInstance\(\)\.player\b")

NO_RENDER_PARTICLE = f"{PARTICLE_PACKAGE}.NoRenderParticle"
RENDER_GROUP_METHOD = "getGroup"
NO_RENDER_GROUP = "ParticleRenderType.NO_RENDER"

SOUND_CALL_NAME = "playLocalSound"
REMOVE_CALL_NAME = "remove"

# The fields vanilla's own expiry tests name. A `remove()` guarded by one of
# them is the ordinary end of life, not a gate.
LIFETIME_FIELDS = ("lifetime", "age", "life")

# Fields that decide whether a particle moves at all.
VELOCITY_FIELDS = ("xd", "yd", "zd", "gravity")
ZERO_LITERALS = frozenset({"0", "0.0", "0F", "0.0F", "0f", "0.0f"})
# `Particle(level, x, y, z)` — the constructor that injects no velocity.
MOTIONLESS_PARTICLE_ARITY = 4

# Collision helpers `Particle.move` itself uses; an override that calls one is
# doing its own collision rather than translating.
COLLISION_HELPER_NAMES = frozenset({"collideBoundingBox", "getCollisions", "clip"})


@dataclass(frozen=True)
class Fact:
    """Whether vanilla exhibits a behaviour, and the reason it was decided."""

    present: bool
    reason: str


PROVIDER_SPAWN = "provider"
CODE_SPAWN = "code"


@dataclass(frozen=True)
class ParticleFacts:
    """One row of the particle coverage ledger."""

    name: str
    particle: str
    spawn: str
    renders: Fact
    collision: Fact
    player_coupled: Fact
    sounds: Fact
    conditional_removal: Fact

    def facts(self) -> dict[str, Fact]:
        return {
            "renderer binding": self.renders,
            "collision": self.collision,
            "player-coupled": self.player_coupled,
            "sounds": self.sounds,
            "conditional-removal": self.conditional_removal,
        }


@dataclass
class ParticleSources:
    """Every parsed type in the vanilla particle package, hierarchy resolved."""

    types: dict[str, JavaType]
    source_of: dict[str, JavaSource]

    @classmethod
    def load(cls, source_root: Path) -> "ParticleSources":
        directory = source_root / PARTICLE_DIRECTORY
        if not directory.is_dir():
            raise FileNotFoundError(f"vanilla particle sources not found: {directory}")
        parsed: list[JavaType] = []
        source_of: dict[str, JavaSource] = {}
        for path in sorted(directory.glob("*.java")):
            source = parse_source(str(path.relative_to(source_root)), path.read_bytes())
            parsed.extend(source.types)
            for declared in source.types:
                source_of[declared.fqcn] = source
        resolve_class_hierarchy(parsed)
        return cls({declared.fqcn: declared for declared in parsed}, source_of)

    def text_of(self, declared: JavaType) -> str:
        source = self.source_of[declared.fqcn]
        node = declared.body_node or declared.node
        return source.source[node.start_byte : node.end_byte].decode("utf-8")

    def is_abstract(self, declared: JavaType) -> bool:
        source = self.source_of[declared.fqcn]
        end = (declared.body_node or declared.node).start_byte
        return b"abstract" in source.source[declared.node.start_byte : end]

    def ancestry(self, declared: JavaType) -> list[JavaType]:
        """`declared` and its superclasses, nearest first, within the package."""
        chain: list[JavaType] = []
        seen: set[str] = set()
        current: JavaType | None = declared
        while current is not None and current.fqcn not in seen:
            chain.append(current)
            seen.add(current.fqcn)
            current = self.types.get(current.superclass) if current.superclass else None
        return chain

    def methods(self, declared: JavaType) -> list[JavaMethod]:
        return self.source_of[declared.fqcn].methods_for(declared)

    def method_text(self, declared: JavaType, method: JavaMethod) -> str:
        source = self.source_of[declared.fqcn]
        return source.source[method.node.start_byte : method.node.end_byte].decode("utf-8")

    def find_method(
        self, chain: list[JavaType], name: str, *, after: int = 0
    ) -> tuple[JavaType, JavaMethod] | None:
        """The nearest declaration of `name` at or above `chain[after]`."""
        for declared in chain[after:]:
            for method in self.methods(declared):
                if method.name == name and not method.is_constructor:
                    return declared, method
        return None

    def find_constructor(self, declared: JavaType, arity: int | None) -> JavaMethod | None:
        """The constructor a call of `arity` arguments selects.

        `SingleQuadParticle` overloads its constructor with and without a
        velocity triple, and which one runs decides whether the particle can
        ever move, so the arity has to pick rather than "the first declared".
        An unknown arity falls back to the first declaration.
        """
        constructors = [method for method in self.methods(declared) if method.is_constructor]
        for method in constructors:
            if arity is not None and len(method.parameters) == arity:
                return method
        return constructors[0] if constructors else None

    def node_text(self, declared: JavaType, node) -> str:
        source = self.source_of[declared.fqcn]
        return source.source[node.start_byte : node.end_byte].decode("utf-8")


# Any `name(` occurrence: the facts below only ask whether a named call appears
# anywhere in the reachable bodies, and vanilla reaches them through varied
# qualifiers (`this.level.playLocalSound`, `super.tick`, `particle.move`).
CALL_NAME_RE = re.compile(r"\b([a-zA-Z_]\w*)\s*\(")
JAVA_KEYWORDS = frozenset(
    {"if", "for", "while", "switch", "catch", "return", "new", "super", "this", "synchronized"}
)
# Nested particle classes are constructed by their qualified name — every
# `DripParticle` provider says `new DripParticle.DripHangParticle(...)`.
NEW_TYPE_RE = re.compile(r"\bnew\s+([A-Z][A-Za-z0-9_.]*)\s*[(<]")


def called_names(body: str) -> set[str]:
    return {name for name in CALL_NAME_RE.findall(body) if name not in JAVA_KEYWORDS}


def find_providers(sources: ParticleSources) -> list[JavaType]:
    """Concrete `ParticleProvider` implementations, in declaration order.

    A provider may implement the interface through an abstract base — the four
    `BreakingItemParticle` providers extend `ItemParticleProvider` — so the
    whole superclass chain is consulted, and abstract bases are not rows.
    """
    providers = []
    for declared in sources.types.values():
        if sources.is_abstract(declared):
            continue
        chain = sources.ancestry(declared)
        implements = any(
            PROVIDER_INTERFACE in (interface or "")
            for ancestor in chain
            for interface in (ancestor.interfaces or ())
        )
        if implements:
            providers.append(declared)
    return sorted(providers, key=provider_name)


def provider_name(declared: JavaType) -> str:
    """`AshParticle.Provider` — the name the runtime descriptors use."""
    return declared.fqcn[len(PARTICLE_PACKAGE) + 1 :]


@dataclass(frozen=True)
class Construction:
    """The particle class a row builds, and the arguments it passes.

    `arguments` is None for a particle no provider constructs: the call site is
    outside the package, so nothing can be folded into its conditions.
    """

    particle: JavaType
    arguments: tuple[str, ...] | None


def resolve_construction(
    sources: ParticleSources, provider: JavaType
) -> Construction | None:
    """The particle a provider's `createParticle` produces, with its arguments.

    Some providers return a static factory of the enclosing class rather than a
    constructor — every `DripParticle` provider does — so factories are
    followed one level, and the arguments then come from the factory body,
    which is where vanilla writes the per-provider constants.
    """
    outer_name = provider.fqcn.rsplit(".", 1)[0]
    for ancestor in sources.ancestry(provider):
        found = sources.find_method([ancestor], "createParticle")
        if found is None:
            continue
        body = sources.method_text(*found)
        construction = _construction_in(sources, body, outer_name)
        if construction is not None:
            return construction
        # Follow a static factory declared on the enclosing particle class.
        outer = sources.types.get(outer_name)
        if outer is None:
            continue
        for name in called_names(body):
            factory = sources.find_method([outer], name)
            if factory is None:
                continue
            construction = _construction_in(
                sources, sources.method_text(*factory), outer_name
            )
            if construction is not None:
                return construction
    return None


def _construction_in(
    sources: ParticleSources, body: str, outer_name: str
) -> Construction | None:
    for match in NEW_TYPE_RE.finditer(body):
        name = match.group(1)
        simple = name.rsplit(".", 1)[-1]
        candidates = (
            f"{PARTICLE_PACKAGE}.{name}",
            f"{outer_name}.{simple}",
            f"{PARTICLE_PACKAGE}.{simple}",
        )
        for candidate in candidates:
            declared = sources.types.get(candidate)
            if declared is None or not _is_particle(sources, declared):
                continue
            arguments = (
                _split_arguments(body[match.end() :])
                if match.group(0).endswith("(")
                else []
            )
            return Construction(declared, tuple(arguments))
    return None


def _is_particle(sources: ParticleSources, declared: JavaType) -> bool:
    return any(ancestor.fqcn == BASE_PARTICLE for ancestor in sources.ancestry(declared))


SUPER_CALL_NAME_RE = re.compile(r"\bsuper\.(\w+)\s*\(")


@dataclass(frozen=True)
class TickReach:
    """What the particle's effective tick can reach, and what it calls."""

    methods: tuple[tuple[JavaType, JavaMethod], ...]
    bodies: tuple[str, ...]
    called: frozenset[str]


def tick_reach(sources: ParticleSources, particle: JavaType) -> TickReach:
    """Walk the tick call graph across the particle's class hierarchy.

    An override that calls `super.tick()` reaches the base tick's `move`, so a
    super call has to resolve one level up the chain rather than back onto the
    override that made it.
    """
    chain = sources.ancestry(particle)
    visited: set[tuple[str, int]] = set()
    methods: list[tuple[JavaType, JavaMethod]] = []
    bodies: list[str] = []
    called: set[str] = set()
    pending: list[tuple[str, int]] = [(TICK_METHOD, 0)]
    while pending:
        key = pending.pop()
        if key in visited:
            continue
        visited.add(key)
        name, start = key
        found = sources.find_method(chain, name, after=start)
        if found is None:
            continue
        owner, method = found
        body = sources.method_text(owner, method)
        methods.append(found)
        bodies.append(body)
        names = called_names(body)
        called |= names
        super_names = set(SUPER_CALL_NAME_RE.findall(body))
        owner_index = chain.index(owner)
        for callee in names - super_names:
            pending.append((callee, 0))
        for callee in super_names:
            pending.append((callee, owner_index + 1))
    return TickReach(tuple(methods), tuple(bodies), frozenset(called))


def collision_status(
    sources: ParticleSources, construction: Construction, reach: TickReach
) -> Fact:
    """Whether a colliding `move` is reachable, and the vanilla reason if not.

    The four vanilla ways to opt out are exactly the four checked here.
    """
    chain = sources.ancestry(construction.particle)
    if has_physics_disabled(sources, chain):
        return Fact(False, "hasPhysics=false")

    override = sources.find_method(chain, MOVE_METHOD)
    if override is not None and override[0].fqcn != BASE_PARTICLE:
        owner, method = override
        body = sources.method_text(owner, method)
        names = called_names(body)
        delegates = "super.move(" in body
        collides = bool(names & COLLISION_HELPER_NAMES)
        if not delegates and not collides:
            return Fact(False, f"collision-free move override in {owner.simple_name}")

    if MOVE_METHOD not in reach.called:
        return Fact(False, "tick never calls move")

    if velocity_pinned_to_zero(sources, construction):
        return Fact(False, "velocity pinned to zero")

    return Fact(True, "colliding move reachable from tick")


def velocity_pinned_to_zero(sources: ParticleSources, construction: Construction) -> bool:
    """Whether the particle's deltas can never leave zero.

    `Particle.move` skips collision outright when all three deltas are zero
    (Particle.java:150), so a particle constructed through the motionless base
    constructor that never assigns a non-zero delta or gravity cannot collide
    however often its tick calls `move`.
    """
    for declared, arity in constructor_chain(sources, construction):
        if declared.fqcn == BASE_PARTICLE:
            return arity == MOTIONLESS_PARTICLE_ARITY
        for match in FIELD_ASSIGNMENT_RE.finditer(sources.text_of(declared)):
            field, operator, value = match.groups()
            if field not in VELOCITY_FIELDS:
                continue
            if operator != "=" or value.strip() not in ZERO_LITERALS:
                return False
    return False


def constructor_chain(
    sources: ParticleSources, construction: Construction
) -> list[tuple[JavaType, int]]:
    """Each class the construction runs through, with the arity it receives."""
    walk: list[tuple[JavaType, int | None]] = []
    declared: JavaType | None = construction.particle
    arity = len(construction.arguments) if construction.arguments is not None else None
    while declared is not None:
        constructor = sources.find_constructor(declared, arity)
        if arity is None and constructor is not None:
            arity = len(constructor.parameters)
        walk.append((declared, arity))
        if declared.fqcn == BASE_PARTICLE or constructor is None:
            break
        arity = len(super_call_arguments(sources, declared, constructor))
        declared = sources.types.get(declared.superclass) if declared.superclass else None
    return walk


def constructor_constants(
    sources: ParticleSources, construction: Construction
) -> dict[str, str]:
    """Fields whose value is a construction-site literal.

    Vanilla discriminates otherwise identical particle classes this way — the
    `Fluid` a drip compares its position against arrives as a constructor
    argument — so the literal is threaded up through each `super(...)` call.
    """
    constants: dict[str, str] = {}
    declared: JavaType | None = construction.particle
    arguments = list(construction.arguments or ())
    while declared is not None and declared.fqcn != BASE_PARTICLE:
        constructor = sources.find_constructor(declared, len(arguments))
        if constructor is None:
            break
        names = [parameter.canonical.split()[-1] for parameter in constructor.parameters]
        bound = dict(zip(names, arguments))
        body = sources.method_text(declared, constructor)
        for match in FIELD_ASSIGNMENT_RE.finditer(body):
            field, operator, value = match.groups()
            if operator != "=":
                continue
            resolved = bound.get(value.strip(), value.strip())
            if resolved not in bound:
                constants.setdefault(field, resolved)
        arguments = [
            bound.get(argument, argument)
            for argument in super_call_arguments(sources, declared, constructor)
        ]
        declared = sources.types.get(declared.superclass) if declared.superclass else None
    return constants


def conditional_removal(
    sources: ParticleSources, reach: TickReach, constants: dict[str, str]
) -> Fact:
    """Whether the tick can remove the particle other than by running out of life."""
    for owner, method in reach.methods:
        for node in _descendants(method.node):
            if node.type != "method_invocation":
                continue
            name = node.child_by_field_name("name")
            if name is None or sources.node_text(owner, name) != REMOVE_CALL_NAME:
                continue
            guards = _enclosing_conditions(sources, owner, node)
            if any(taken and _is_pure_lifetime(condition) for condition, taken in guards):
                continue
            if any(_is_statically_dead(condition, taken, constants) for condition, taken in guards):
                continue
            if not any(not _is_pure_lifetime(condition) for condition, _taken in guards):
                continue
            return Fact(True, f"conditional remove() in {owner.simple_name}.{method.name}")
    return Fact(False, "removal only on lifetime expiry")


def _descendants(node):
    for child in node.children:
        yield child
        yield from _descendants(child)


def _enclosing_conditions(
    sources: ParticleSources, owner: JavaType, node
) -> list[tuple[str, bool]]:
    """Every enclosing `if` condition, with whether the node is in its then-branch."""
    guards: list[tuple[str, bool]] = []
    parent = node.parent
    while parent is not None:
        if parent.type == "if_statement":
            condition = parent.child_by_field_name("condition")
            consequence = parent.child_by_field_name("consequence")
            taken = (
                consequence is not None
                and consequence.start_byte <= node.start_byte < consequence.end_byte
            )
            if condition is not None:
                guards.append((sources.node_text(owner, condition), taken))
        parent = parent.parent
    return guards


def _is_pure_lifetime(condition: str) -> bool:
    """Whether the condition tests nothing but the particle's remaining life.

    Vanilla writes expiry both ways round — `Particle.tick` removes in the
    then-branch of `age++ >= lifetime`, `CampfireSmokeParticle` in the
    else-branch of `age++ < lifetime && !(alpha <= 0)`. Only a lone lifetime
    test means the particle simply ran out; the moment another term joins it,
    reaching `remove()` can mean something else happened first.
    """
    if not any(f"this.{field}" in condition for field in LIFETIME_FIELDS):
        return False
    return "&&" not in condition and "||" not in condition


COMPARISON_RE = re.compile(r"^\(?\s*this\.(\w+)\s*(!=|==)\s*([\w.]+)\s*\)?$")


def _is_statically_dead(condition: str, taken: bool, constants: dict[str, str]) -> bool:
    """Whether this branch is unreachable given the provider's own arguments.

    Only comparisons against a construction-site constant are folded; anything
    else stays live, so an undecidable condition never silently drops a gate.
    """
    match = COMPARISON_RE.match(condition.strip())
    if match is None:
        return False
    field, operator, other = match.groups()
    value = constants.get(field)
    if value is None:
        return False
    holds = value != other if operator == "!=" else value == other
    return not (holds if taken else not holds)


# `this.<field> <op>= <value>;` — the shape every particle constructor uses to
# pin motion, physics and injected constants.
FIELD_ASSIGNMENT_RE = re.compile(r"this\.(\w+)\s*([-+*/]?=)\s*([^;]+);")
SUPER_CALL_RE = re.compile(r"\bsuper\s*\(")


def has_physics_disabled(sources: ParticleSources, chain: list[JavaType]) -> bool:
    """Whether `hasPhysics` is false for this particle.

    It is either assigned a literal, or threaded through a base constructor
    parameter — `BaseAshSmokeParticle` takes it as its last argument.
    """
    for declared in chain:
        for match in FIELD_ASSIGNMENT_RE.finditer(sources.text_of(declared)):
            field, operator, value = match.groups()
            if field == "hasPhysics" and operator == "=" and value.strip() == "false":
                return True
    for index in range(len(chain) - 1):
        parent_parameters = constructor_parameter_names(sources, chain[index + 1])
        if "hasPhysics" not in parent_parameters:
            continue
        arguments = declared_super_call_arguments(sources, chain[index])
        position = parent_parameters.index("hasPhysics")
        if position < len(arguments) and arguments[position] == "false":
            return True
    return False


def constructor_parameter_names(sources: ParticleSources, declared: JavaType) -> list[str]:
    for method in sources.methods(declared):
        if method.is_constructor:
            return [parameter.canonical.split()[-1] for parameter in method.parameters]
    return []


def super_call_arguments(
    sources: ParticleSources, declared: JavaType, constructor: JavaMethod
) -> list[str]:
    """Top-level argument texts of a constructor's `super(...)` call."""
    body = sources.method_text(declared, constructor)
    match = SUPER_CALL_RE.search(body)
    if match is None:
        return []
    return _split_arguments(body[match.end() :])


def declared_super_call_arguments(sources: ParticleSources, declared: JavaType) -> list[str]:
    """As above, for the class's first declared constructor."""
    for method in sources.methods(declared):
        if method.is_constructor:
            return super_call_arguments(sources, declared, method)
    return []


def _split_arguments(text: str) -> list[str]:
    depth = 0
    arguments: list[str] = []
    current = ""
    for character in text:
        if character in "([{":
            depth += 1
        elif character in ")]}":
            if depth == 0:
                arguments.append(current.strip())
                return arguments
            depth -= 1
        if character == "," and depth == 0:
            arguments.append(current.strip())
            current = ""
            continue
        current += character
    return arguments


def renders_status(sources: ParticleSources, particle: JavaType) -> Fact:
    """Whether the particle draws anything.

    `NoRenderParticle` exists solely to answer `NO_RENDER`; its subclasses are
    emitters whose visible output belongs to the particles they spawn, so bbb
    needs no renderer descriptor for them.
    """
    chain = sources.ancestry(particle)
    for ancestor in chain:
        if ancestor.fqcn == NO_RENDER_PARTICLE:
            return Fact(False, f"{ancestor.simple_name} subclass draws nothing")
    group = sources.find_method(chain, RENDER_GROUP_METHOD)
    if group is not None and NO_RENDER_GROUP in sources.method_text(*group):
        return Fact(False, f"{group[0].simple_name}.{RENDER_GROUP_METHOD} is NO_RENDER")
    return Fact(True, "draws through a particle render group")


def find_code_spawned(sources: ParticleSources, built: set[str]) -> list[JavaType]:
    """Concrete particles no provider builds and nothing else extends.

    Vanilla spawns a few particles straight from client code — the tracking
    emitter, the firework starter, the item-pickup animation — and they carry
    the same client-observable behaviour as a provider's particle. Excluding
    the classes that other particles extend is what separates them from the
    family bases vanilla simply never marked `abstract`.
    """
    extended = {
        ancestor.fqcn
        for declared in sources.types.values()
        for ancestor in sources.ancestry(declared)[1:]
    }
    spawned = []
    for fqcn, declared in sources.types.items():
        if fqcn == BASE_PARTICLE or fqcn in built or fqcn in extended:
            continue
        if sources.is_abstract(declared) or not _is_particle(sources, declared):
            continue
        spawned.append(declared)
    return sorted(spawned, key=lambda declared: declared.fqcn)


def particle_facts(
    sources: ParticleSources, name: str, spawn: str, construction: Construction
) -> ParticleFacts:
    particle = construction.particle
    reach = tick_reach(sources, particle)
    constants = constructor_constants(sources, construction)
    player = bool(reach.called & PLAYER_CALL_NAMES) or any(
        PLAYER_TYPE_RE.search(body) for body in reach.bodies
    )
    sounds = SOUND_CALL_NAME in reach.called
    return ParticleFacts(
        name=name,
        particle=particle.fqcn[len(PARTICLE_PACKAGE) + 1 :],
        spawn=spawn,
        renders=renders_status(sources, particle),
        collision=collision_status(sources, construction, reach),
        player_coupled=Fact(
            player,
            "tick reads the local player or camera"
            if player
            else "tick reads no local player or camera",
        ),
        sounds=Fact(
            sounds,
            f"tick raises {SOUND_CALL_NAME}"
            if sounds
            else f"tick raises no {SOUND_CALL_NAME}",
        ),
        conditional_removal=conditional_removal(sources, reach, constants),
    )


def extract_particle_facts(sources: ParticleSources) -> list[ParticleFacts]:
    facts = []
    built: set[str] = set()
    for provider in find_providers(sources):
        construction = resolve_construction(sources, provider)
        if construction is None:
            raise ValueError(
                f"{provider_name(provider)}: no particle class resolved from createParticle"
            )
        built.add(construction.particle.fqcn)
        facts.append(
            particle_facts(sources, provider_name(provider), PROVIDER_SPAWN, construction)
        )
    for particle in find_code_spawned(sources, built):
        name = particle.fqcn[len(PARTICLE_PACKAGE) + 1 :]
        facts.append(
            particle_facts(sources, name, CODE_SPAWN, Construction(particle, None))
        )
    return facts
