#!/usr/bin/env python3
"""Compare what two dedicated servers do to the same save while nobody plays.

The differential replay this repository accepts alignment on drives both
servers with the same client and compares what the client saw. That oracle is
structurally blind to everything a server does on its own: a scheduled block
tick, a fluid spreading, an entity falling and a periodic save all happen with
no inbound packet asking for them, so a client that never asks never sees them
and a client-shaped oracle can never report them missing.

This harness uses the other oracle the scope decision names: the official
server's own observable output. One save is prepared by the official server
itself -- it is the authority, and it is also the only one of the two that can
be typed at -- and then opened twice, once by each server, with no client
connected at all. Each side is frozen, stepped exactly the same number of
ticks, flushed and stopped. What is compared is the block and entity truth both
wrote back to their region files.

The step is what makes the comparison a measurement rather than a race. A
frozen server runs no game elements at all: `TickRateManager.tick`
(TickRateManager.java:56-61) sets `runGameElements` from `!isFrozen ||
frozenTicksToRun > 0` and decrements the step counter afterwards, so `tick step
Nt` runs exactly N ticks and then stops. `ServerLevel.tick` reads that flag
back through `runsNormally` (TickRateManager.java:32-34) into the `runs` it
gates on (ServerLevel.java:343-344), and `runs` is what `tickTime()` hangs off
(ServerLevel.java:366-368). `tickTime` advances the game clock by one
(ServerLevel.java:461-464) into the `Time` field `level.dat` carries
(PrimaryLevelData.java:133) -- the overworld being the level built with
`tickTime` true (MinecraftServer.java:430-431). So "did both sides really run N
ticks" stops being a guess about how long to sleep and becomes a number this
harness reads back off both saves and asserts on.

Three vanilla facts make an unattended run possible at all. The first two are
the two gates a tick has to pass, and they are not the same gate. A scheduled
block or fluid tick runs only where the predicate the two `LevelTicks` were
built with holds -- `ServerLevel.isPositionTickingWithEntitiesLoaded`
(ServerLevel.java:207-208 and :1789-1791) -- while a random tick runs only in
the chunks `ChunkMap.forEachBlockTickingChunk` (ChunkMap.java:951-961) hands
`ServerLevel.tickChunk`, which are the entity-ticking ones. Neither is reached
without a player near it, so the scenario chunks are `/forceload`ed:
`TicketType.FORCED` (TicketType.java:22) is registered with flags 15, which is
`FLAG_PERSIST | FLAG_LOADING | FLAG_SIMULATION | FLAG_KEEP_DIMENSION_ACTIVE`
(TicketType.java:12-16), and `ChunkMap.FORCED_TICKET_LEVEL` (ChunkMap.java:126)
is `ChunkLevel.byStatus(ENTITY_TICKING)` -- so a forceload raises the column to
the level both gates need and writes the ticket into the save, which the
resumed servers inherit without being told.

The third fact is what `FLAG_KEEP_DIMENSION_ACTIVE` buys. `ServerLevel.tick`
runs no entity at all once `emptyTime` reaches 300 (ServerLevel.java:404-412),
and `emptyTime` is only reset while `ServerChunkCache.hasActiveTickets`
(ServerChunkCache.java:482-484) holds, which asks
`TicketStorage.shouldKeepDimensionActive` (TicketStorage.java:118-128) for a
ticket carrying that flag. The forced tickets are what keeps the entity
scenarios ticking with nobody connected.

And the scenario has to be placed without any of it happening yet, so the
preparing server is `/tick freeze`n first: the frozen flag lives only on
`ServerTickRateManager` and is never serialised, so the save carries the
pending scheduled ticks and none of their effects, and each measurement run
freezes again on its own.

Determinism is bought per scenario rather than globally. Vanilla's random tick
draws `random_tick_speed` positions per randomly-ticking section per tick
(ServerLevel.java:495-520), each from `Level.getBlockRandomPos`
(Level.java:1059-1063), and no implementation can match another
implementation's sample sequence position for position. So every scenario here
is either built out of scheduled ticks and made immune to random ticks, or
built so that its random tick has exactly one terminal outcome and given a
`random_tick_speed` high enough that the terminal state is reached with
overwhelming probability inside the step. What is compared is always a state
nothing can leave.

One kind of server-side behaviour deliberately has no scenario here: a block
event leaves no trace in a save at all. `ServerLevel.runBlockEvents`
(ServerLevel.java:1241-1245) drains its queue into `triggerEvent`
(ServerLevel.java:1268-1271) inside the tick it was queued in, and nothing about
it is serialised, so no reading of a region file can say whether it happened.
The live differential replay owns that one.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import time
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import IO, Any

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.anvil import (  # noqa: E402
    AnvilError,
    BlockState,
    WorldSaveReader,
    read_compressed_nbt,
)
from server_release_gate import (  # noqa: E402
    OFFICIAL_REPLAY_PROPERTIES,
    OfficialServer,
    Paths,
    ReleaseGateError,
    ServerProcess,
    free_port,
    require,
)

SCHEMA_VERSION = 2
VANILLA_VERSION = "26.1"

#: How many ticks each side is stepped. Every scenario below settles well inside
#: it: the fluid needs seven spread steps at `WaterFluid.getTickDelay`'s 5 ticks
#: each, the leaf cascade quiesces within seven whatever order the reloaded
#: ticks come out in, `FALLING_SAND_COLUMN` needs
#: `FallingBlock.getDelayAfterPlace`'s 2 ticks (FallingBlock.java:59-61) and
#: then some twenty-five for a ten-block fall. The random scenarios' probability
#: arguments are stated on `LEAF_DECAY_PATCH` and `CHORUS_RANDOM_GROWTH`.
#:
#: This number is bounded from above as well as below, which is why it is a
#: constant with an argument rather than a taste. `ENTITY_FALL_AND_LAND`'s high
#: sand has to still be airborne when the step ends, and it lands at about tick
#: 230; that scenario states the arithmetic. The binding lower bound is
#: `CHORUS_RANDOM_GROWTH`, which needs two section hits and so becomes negligible
#: only in the hundreds. Roughly 100 to roughly 220 satisfies every scenario at
#: once, and 200 sits at the end of that window where the random-tick arguments
#: are strongest.
DEFAULT_STEP_TICKS = 200

#: Vanilla's tick rate. A stepped tick still costs its own wall clock, so this
#: is what turns a tick budget into the soonest a step can possibly be done.
TICKS_PER_SECOND = 20

BOOT_TIMEOUT = 180.0
SAVE_TIMEOUT = 120.0
COMMAND_TIMEOUT = 60.0
#: What a step is allowed on top of the wall clock its own ticks cost. A server
#: whose ticks run long is still stepping exactly N of them, so this only has to
#: cover slowness, never a different amount of work.
STEP_GRACE_SECONDS = 300.0
STEP_POLL_SECONDS = 1.0

#: What the official server prints for a console save and for a command it
#: could not parse. `<--[HERE]` is Brigadier's own context marker, so it catches
#: every syntax error whatever the argument was.
SAVED_MARKER = "Saved the game"

#: `/tick freeze` and `/tick step <n>t`, the two console lines the measurement
#: run is built on.
#:
#: The shape comes from `TickCommand.register` (TickCommand.java:33-42): `step`
#: takes a `TimeArgument.time(1)`, and `TimeArgument`'s unit table
#: (TimeArgument.java:78-83) is `d` 24000, `s` 20, `t` 1 and the empty unit 1 --
#: so `200t` is two hundred ticks and nothing has to be inferred about a
#: default unit. `freeze` is `setFreeze(source, true)` (TickCommand.java:53).
FREEZE_COMMAND = "tick freeze"

#: What each of the three answers is, by translation key and by the English the
#: language file gives that key. Both are checked, because the official server
#: logs the rendered string (`MinecraftServer.sendSystemMessage`,
#: MinecraftServer.java:1250-1252) while bbb's management answer carries the
#: component, and a harness that only knew one of the two forms would read a
#: refusal as silence.
#:
#: * `commands.tick.status.frozen` -> "The game is frozen" (en_us.json:3039)
#: * `commands.tick.step.fail` -> "Unable to step the game - the game must be
#:   frozen first" (en_us.json:3043)
#: * `commands.tick.step.success` -> "Stepping %s tick(s)" (en_us.json:3046),
#:   whose one argument is the `int` advance (TickCommand.java:135) rendered by
#:   `TranslatableContents.getArgument`'s `arg.toString()`
#:   (TranslatableContents.java:151-158), so plain decimal digits.
FROZEN_KEY = "commands.tick.status.frozen"
FROZEN_TEXT = "The game is frozen"
STEP_SUCCESS_KEY = "commands.tick.step.success"
STEP_FAILURE_KEY = "commands.tick.step.fail"
STEP_FAILURE_TEXT = "Unable to step the game - the game must be frozen first"

#: Every line that means a console command this harness typed did not do what
#: it was typed for.
#:
#: The first four are the parser's. `Commands.performCommand` appends the
#: `<--[HERE]` context only when the exception carries the input it failed on
#: (Commands.java:366-374), which is true of a parse error and false of the
#: contextless `SimpleCommandExceptionType.create()` a command's own body
#: throws -- so a refusal after a successful parse is a plain line with nothing
#: to mark it, and each one this harness can provoke has to be named.
#: Those are the rest, taken from the language file by key:
#:
#: * `commands.setblock.failed` (en_us.json:2917) and `commands.fill.failed`
#:   (en_us.json:2724) -- a scenario that was never built.
#: * `commands.fill.toobig` (en_us.json:2726) -- a `/fill` this harness sized
#:   wrong; `ground_commands` is the only one that computes its own bounds.
#: * `commands.forceload.added.failure` (en_us.json:2730) -- no ticket, so
#:   nothing ticks and every box comes back untouched.
#: * `argument.pos.unloaded` (en_us.json:381) and `argument.pos.outofworld`
#:   (en_us.json:380) -- `/setblock` resolves its position through
#:   `BlockPosArgument.getLoadedBlockPos`, which is where a y=300 typo or a
#:   forgotten forceload would surface.
COMMAND_FAILURE_MARKERS = (
    "Unknown or incomplete command",
    "Incorrect argument for command",
    "<--[HERE]",
    "An unexpected error occurred",
    # `TickCommand.step` returns 1 whether or not the step was taken
    # (TickCommand.java:140), so the refusal is only ever visible as this line.
    STEP_FAILURE_TEXT,
    "Could not set the block",
    "No blocks were filled",
    "Too many blocks in the specified area",
    "No chunks were marked for force loading",
    "That position is not loaded",
    "That position is out of this world!",
)

#: What the credential-less loopback caller on bbb's management endpoint is
#: granted. `ServerProcess` defaults this to `manage`, and `manage` is refused
#: for the one request this harness cannot do without: `AdminRequest`'s
#: `required_scope` puts `ExecuteCommand` alone behind `AdminScope::Command`
#: and `AdminState` answers anything lower with "scope manage does not satisfy
#: command" (bbb-server-core's `admin/protocol.rs` and `admin/state.rs`). The
#: freeze and the step are both `command.execute`, so the scope has to be named
#: here rather than inherited.
#:
#: `Command` is the top of an ordered ladder -- it satisfies `Manage`, which
#: satisfies `Read` -- so this single grant also covers every other management
#: call this run makes: `server.save` and `server.stop` both ask only for
#: `Manage`, and asking for the higher one cannot refuse them.
BBB_LOOPBACK_SCOPE = "command"

#: The dimension whose region files are compared. Every scenario is placed in
#: the overworld, which is the only dimension the flat capture generates around
#: the origin.
DIMENSION = "overworld"


def step_command(ticks: int) -> str:
    """The console line that runs exactly `ticks` ticks on a frozen server."""
    return f"tick step {ticks}t"


def step_success_text(ticks: int) -> str:
    """What `commands.tick.step.success` renders to for that many ticks."""
    return f"Stepping {ticks} tick(s)"


class OracleError(RuntimeError):
    """The harness could not produce a comparison."""


def world_template(paths: Paths) -> Path:
    """The official save the scenario is built inside a copy of.

    `Paths` is the release gate's shared record of where the binaries and the
    runtime live, and it already carries this field; there it is optional
    because only some families need it, while here it is the input.
    """
    if paths.world_template is None:
        raise OracleError(
            "the world oracle needs an official 26.1 save to build its scenario in; "
            "run it through scripts/server-world-oracle-26.1.sh"
        )
    return paths.world_template


@dataclass(frozen=True)
class Box:
    """An inclusive block-coordinate bounding box."""

    min_x: int
    min_y: int
    min_z: int
    max_x: int
    max_y: int
    max_z: int

    def positions(self) -> Iterator[tuple[int, int, int]]:
        for x in range(self.min_x, self.max_x + 1):
            for y in range(self.min_y, self.max_y + 1):
                for z in range(self.min_z, self.max_z + 1):
                    yield (x, y, z)

    @property
    def volume(self) -> int:
        return (
            (self.max_x - self.min_x + 1)
            * (self.max_y - self.min_y + 1)
            * (self.max_z - self.min_z + 1)
        )

    def chunk_columns(self) -> list[tuple[int, int]]:
        """Every chunk column the box touches, not only the corner ones.

        A box three chunks wide has a middle column that no corner names, and a
        column left unforceloaded is a column where nothing ticks.
        """
        return [
            (x, z)
            for x in range(self.min_x >> 4, (self.max_x >> 4) + 1)
            for z in range(self.min_z >> 4, (self.max_z >> 4) + 1)
        ]

    def as_json(self) -> dict[str, int]:
        return {
            "min_x": self.min_x,
            "min_y": self.min_y,
            "min_z": self.min_z,
            "max_x": self.max_x,
            "max_y": self.max_y,
            "max_z": self.max_z,
        }


#: The entity tags a record is compared on.
#:
#: `Entity.saveWithoutId` writes `Pos`, `Motion` and `OnGround`
#: (Entity.java:1976-1987) beside the `id` `saveAsPassenger` puts on the record
#: (Entity.java:1965), and `FallingBlockEntity.addAdditionalSaveData` adds
#: `BlockState` and `Time` (FallingBlockEntity.java:282-283).
#:
#: `UUID` (Entity.java:1990) is deliberately absent: a fresh entity takes
#: `Mth.createInsecureUUID(this.random)` (Entity.java:278), so the two servers
#: cannot agree on it and a comparison that included it would report a
#: difference on every run. The whole record is still reported, so a reader can
#: see the fields that were not compared.
ENTITY_FIELDS = ("id", "Pos", "Motion", "OnGround", "BlockState", "Time")


@dataclass(frozen=True)
class EntityProbe:
    """One entity-storage column a scenario compares.

    `entities/` is the sibling region directory of `region/` and is read by the
    same Anvil reader, so a scenario that wants the entity half names the chunk
    column rather than a box: `EntityStorage` keys its records by column, not by
    block.
    """

    chunk_x: int
    chunk_z: int
    fields: tuple[str, ...] = ENTITY_FIELDS

    def as_json(self) -> dict[str, Any]:
        return {
            "chunk_x": self.chunk_x,
            "chunk_z": self.chunk_z,
            "compared_fields": list(self.fields),
        }


@dataclass(frozen=True)
class Scenario:
    """One deterministic thing a server is supposed to do on its own."""

    name: str
    #: The vanilla owner whose behaviour the scenario is a probe for.
    owner: str
    #: The ledger row this scenario produces acceptance evidence for.
    row: str
    #: Console commands that place the scenario, run on a frozen server.
    setup: tuple[str, ...]
    #: The volume compared afterwards.
    box: Box
    #: The entity column compared afterwards, for a scenario that is about one.
    entities: EntityProbe | None = None

    def as_json(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "owner": self.owner,
            "row": self.row,
            "setup": list(self.setup),
            "box": self.box.as_json(),
            # The declaration, beside the verdict's own `entity` block: what
            # this scenario says it will read, rather than what it read.
            "entity_probe": None if self.entities is None else self.entities.as_json(),
        }


#: A water source at one end of a walled stone trench. `FlowingFluid.spread`
#: runs off `ServerLevel.tick`'s `fluidTicks`, one block per
#: `WaterFluid.getTickDelay`, and stops after seven levels, so the settled
#: trench is a source, seven falling levels and then air -- a shape that says
#: how far a server got, not just whether it moved.
#:
#: Nothing in the compared box is randomly ticked, whatever `random_tick_speed`
#: is: stone declares no random tick, and `LiquidBlock.isRandomlyTicking`
#: (LiquidBlock.java:105-107) defers to the fluid, whose base answer is false
#: (Fluid.java:79-81) and is only overridden by lava (LavaFluid.java:221-223).
FLUID_TRENCH = Scenario(
    name="fluid_trench",
    owner="ServerLevel.tick fluidTicks -> FlowingFluid.tick/spread",
    row="server-block-tick-scheduling",
    setup=(
        "fill 0 -60 3 15 -60 5 minecraft:stone",
        "fill 0 -59 3 15 -59 5 minecraft:stone",
        "fill 1 -59 4 14 -59 4 minecraft:air",
        "setblock 1 -59 4 minecraft:water",
    ),
    box=Box(0, -60, 3, 15, -58, 5),
)

#: A four-block sand column hung ten blocks above the flat world's surface.
#: `FallingBlock.onPlace` schedules a block tick, `FallingBlock.tick` turns the
#: block into a `FallingBlockEntity`, and the entity's own tick lands it -- so
#: this one scenario needs the scheduled block tick and the entity runtime
#: behind it, and the settled column sits on the ground rather than in the air.
FALLING_SAND_COLUMN = Scenario(
    name="falling_sand_column",
    owner="ServerLevel.tick blockTicks -> FallingBlock.tick -> FallingBlockEntity",
    row="server-entity-runtime",
    setup=("fill 4 -50 10 4 -47 10 minecraft:sand",),
    box=Box(4, -61, 10, 4, -46, 10),
)

#: Six leaves in a row hung off one oak log, and then the log taken away.
#:
#: This is the scheduled block tick that is not a falling block. `LeavesBlock`
#: keeps a `DISTANCE` property -- an `IntegerProperty` over 1..7
#: (BlockStateProperties.java:103) -- and recomputes it in its scheduled tick:
#: `LeavesBlock.tick` is `setBlock(pos, updateDistance(state, level, pos), 3)`
#: (LeavesBlock.java:79-81), and `updateDistance` (LeavesBlock.java:111-124)
#: starts at the `DECAY_DISTANCE` of 7 (LeavesBlock.java:30) and takes the
#: minimum with each of the six neighbours' distance plus one, where a
#: neighbour that has no distance at all counts as 7 (LeavesBlock.java:126-136).
#: What schedules that tick is `LeavesBlock.updateShape`
#: (LeavesBlock.java:103-106): it asks for one a tick later unless the
#: neighbour that changed is a supporting log *and* this leaf already records
#: the distance 1 that log implies -- so any change to a neighbouring leaf
#: schedules one, and removing the log starts a cascade that walks the row.
#:
#: What the cascade converges to is provable rather than counted. At a fixed
#: point every leaf satisfies `d = min(7, neighbours + 1)`; if the smallest
#: distance in the row were some `m < 7`, the leaf holding it would compute at
#: least `m + 1`, so it is not a fixed point. With the log gone the only fixed
#: point is therefore `distance=7` on all six -- a definite shape, not
#: "something changed". Quiescence is the same argument: a leaf is rescheduled
#: whenever a neighbour changes, so an empty tick queue means every leaf last
#: computed its own value against neighbours that have not moved since.
#:
#: How long it takes depends on the order the ticks come out of the queue,
#: because `LevelChunkTicks.pack` writes the heap in heap order and
#: `unpack` numbers the reloaded ticks in that order
#: (LevelChunkTicks.java:91-114). Simulating all 120 orders the first tick's
#: five pending leaves can come out in: every one of them reaches all-7, the
#: two extremes -- left to right and right to left -- do it in five and six
#: ticks, and the slowest of the 120 quiesces after seven.
#: `DEFAULT_STEP_TICKS` is two hundred, so no ordering is close to running out
#: of step.
#:
#: The leaves are `persistent=true` on purpose. Decay is not this tick at all --
#: `LeavesBlock.randomTick` (LeavesBlock.java:67-72) is what removes a leaf, and
#: both it and `isRandomlyTicking` (LeavesBlock.java:62-64) require
#: `!persistent`, so a persistent leaf is neither offered a random tick nor
#: removed by one. Since `LEAF_DECAY_PATCH` needs a high `random_tick_speed` in
#: the same world, `persistent=true` is what keeps this scenario's terminal
#: shape a shape rather than a race with the other scenario's gamerule.
#: `LeavesBlock.tick` and `updateDistance` never read `persistent`, so the
#: cascade under test is untouched and the leaves stay persistent through it.
LEAF_DISTANCE_CASCADE = Scenario(
    name="leaf_distance_cascade",
    owner="ServerLevel.tick blockTicks -> LeavesBlock.tick/updateDistance",
    row="server-block-tick-scheduling",
    setup=(
        "setblock 18 -55 2 minecraft:oak_log",
        "setblock 19 -55 2 minecraft:oak_leaves[distance=1,persistent=true]",
        "setblock 20 -55 2 minecraft:oak_leaves[distance=2,persistent=true]",
        "setblock 21 -55 2 minecraft:oak_leaves[distance=3,persistent=true]",
        "setblock 22 -55 2 minecraft:oak_leaves[distance=4,persistent=true]",
        "setblock 23 -55 2 minecraft:oak_leaves[distance=5,persistent=true]",
        "setblock 24 -55 2 minecraft:oak_leaves[distance=6,persistent=true]",
        # The support goes away. `Level.setBlock` runs `updateNeighbourShapes`
        # unless `UPDATE_KNOWN_SHAPE` is set (Level.java:252-256), and a
        # `/setblock` without the `strict` literal places with `2 | 256`
        # (SetBlockCommand.java:125, reached from the plain `executes` at
        # SetBlockCommand.java:32-41 with `strict` false), so the neighbouring
        # leaf really is asked to update its shape.
        "setblock 18 -55 2 minecraft:air",
    ),
    box=Box(18, -55, 2, 24, -55, 2),
)

#: A nine-leaf slab that is already unsupported, left to decay.
#:
#: This is the random tick. Its outcome is terminal and singular:
#: `LeavesBlock.randomTick` (LeavesBlock.java:67-72) drops the block's resources
#: and calls `removeBlock`, so a leaf that is randomly ticked becomes air, and
#: air is randomly ticked by nothing. There is no intermediate state to pin --
#: which is why a crop or frosted ice would not do, both of which count up
#: through states this harness could not name a moment for.
#:
#: `decaying` is `!persistent && distance == 7` (LeavesBlock.java:74-76), and a
#: `/fill` of `minecraft:oak_leaves` names no properties, so every one of the
#: nine takes the block's own default state, `distance=7, persistent=false`
#: (LeavesBlock.java:44) -- so they are decaying from the first tick, with no
#: log and no other leaf-distance source anywhere near them.
#:
#: The probability argument. `ServerLevel.tickChunk` draws `random_tick_speed`
#: positions per randomly-ticking section per tick (ServerLevel.java:505-510),
#: each covering the section's whole 16x16x16 (`Level.getBlockRandomPos`,
#: Level.java:1059-1063), so one particular block is missed by one draw with
#: probability 1 - 1/4096. Over `RANDOM_TICK_SPEED` draws for
#: `DEFAULT_STEP_TICKS` ticks that is (1 - 1/4096)^(1024*200) = e^-50, about
#: 2e-22 per leaf and under 2e-21 for all nine. The draws are an LCG rather than
#: independent samples, so treat that as an order of magnitude rather than a
#: measure -- it is twenty orders of margin either way, and a side that still
#: holds a leaf is a side that does not decay leaves, not a side that got
#: unlucky. The section keeps being randomly ticked until the last leaf goes,
#: because `LevelChunkSection.isRandomlyTicking` (LevelChunkSection.java:113-115)
#: answers off counts (LevelChunkSection.java:117-118) that
#: `LevelChunkSection.setBlockState` maintains per *state* rather than per block
#: (LevelChunkSection.java:72-91), so the draws neither stop early nor keep
#: running once the section is empty of them -- and the persistent leaves
#: `LEAF_DISTANCE_CASCADE` puts in the same section never enter the count.
LEAF_DECAY_PATCH = Scenario(
    name="leaf_decay_patch",
    owner="ServerLevel.tickChunk randomTick -> LeavesBlock.randomTick",
    row="server-block-tick-scheduling",
    setup=("fill 19 -50 9 21 -50 11 minecraft:oak_leaves",),
    box=Box(19, -51, 9, 21, -49, 11),
)

#: An unsupported chorus flower with the survival check still pending in the
#: prepared save. Removing its end-stone support makes `updateShape` schedule a
#: one-tick check (ChorusFlowerBlock.java:149-160), and that check destroys the
#: flower when it cannot survive (ChorusFlowerBlock.java:47-55). Age five
#: disables random ticks, so only the prepared scheduled tick can move it.
CHORUS_SURVIVAL_TICK = Scenario(
    name="chorus_survival_tick",
    owner="ServerLevel.tick blockTicks -> ChorusFlowerBlock.tick",
    row="server-block-tick-scheduling",
    setup=(
        "setblock 80 -60 4 minecraft:end_stone",
        "setblock 80 -59 4 minecraft:chorus_flower[age=5]",
        "setblock 80 -60 4 minecraft:air",
    ),
    box=Box(80, -60, 4, 80, -58, 4),
)

#: One random tick grows the age-four flower straight up from end stone; that
#: path does not draw a branch choice (ChorusFlowerBlock.java:64-126). Placing
#: the new flower uses update flag 2 (ChorusFlowerBlock.java:128-130), which
#: still updates neighbour shapes (Level.java:252-256); the plant therefore
#: connects up to that flower (ChorusPlantBlock.java:72-80). The stone two cells
#: above the new flower blocks another upward growth, and the age-four fallback
#: deterministically writes age five.
#:
#: Both flowers share one 4096-cell section. Across `RANDOM_TICK_SPEED` draws
#: for `DEFAULT_STEP_TICKS`, the expected hits are 50; fewer than the two hits
#: needed to reach the terminal state is about e^-50*(1+50), below 1e-20. The
#: LCG caveat stated for `LEAF_DECAY_PATCH` applies here too.
CHORUS_RANDOM_GROWTH = Scenario(
    name="chorus_random_growth",
    owner="ServerLevel.tickChunk randomTick -> ChorusFlowerBlock.randomTick",
    row="server-block-tick-scheduling",
    setup=(
        "setblock 84 -60 4 minecraft:end_stone",
        "setblock 84 -59 4 minecraft:chorus_flower[age=4]",
        "setblock 84 -56 4 minecraft:stone",
    ),
    box=Box(84, -60, 4, 84, -56, 4),
)

#: Two sand blocks dropped from two heights: one lands inside the step and one
#: is still falling when it ends.
#:
#: The point is the entity storage, not the block. `FallingBlockEntity` is
#: spawned by `FallingBlock.tick` and removed by its own tick when it lands
#: (`this.discard()` after `setBlock`, FallingBlockEntity.java:194-198), so a
#: save taken mid-flight has to hold exactly one record: the low block's entity
#: is gone and its sand is on the ground, while the high block's entity is still
#: in the air. Comparing two empty entity lists would prove nothing at all --
#: every server that never spawned an entity passes that -- so the scenario is
#: built so that there is something in `entities/` on both sides at the compared
#: instant, and the compared box says the same thing from the block side: the
#: low column has landed and the high one has not.
#:
#: The arithmetic, redone from the source rather than quoted. Each tick
#: `FallingBlockEntity.tick` adds the gravity to the velocity, moves by it, and
#: only then scales the velocity by 0.98 (FallingBlockEntity.java:152-154 and
#: :240), the gravity being `getDefaultGravity`'s 0.04
#: (FallingBlockEntity.java:142-144) added by `Entity.applyGravity`
#: (Entity.java:1440-1445). Writing the downward speed after tick n as u(n),
#: that is u(n) = 0.98*(u(n-1) + 0.04) from u(0) = 0, so u(n) = 1.96*(1-0.98^n),
#: the drop on tick n is u(n-1) + 0.04 = 2 - 1.96*0.98^(n-1), and the distance
#: fallen after t ticks is
#:
#:     S(t) = 2t - 98*(1 - 0.98^t)
#:
#: which rises to a terminal 1.96 blocks a tick.
#:
#: How many ticks each block gets. `/setblock` runs `FallingBlock.onPlace`,
#: which schedules the block's own tick `getDelayAfterPlace` = 2 ticks out
#: (FallingBlock.java:29-31 and :59-61), and the freeze means that delay is
#: still pending in the prepared save -- so `FallingBlock.tick` fires on the
#: second stepped tick and spawns the entity (FallingBlock.java:49-54).
#: `addFreshEntity` reaches `entityTickList` synchronously
#: (PersistentEntitySectionManager.java:74-97), and the entity loop is later in
#: the same `ServerLevel.tick` than the block ticks are
#: (ServerLevel.java:371-379 against :404-412), so the entity also ticks on the
#: tick it was spawned in. A `DEFAULT_STEP_TICKS` of 200 therefore gives it 199
#: ticks of flight.
#:
#: The two blocks. The low one is placed at y=-55 with `GROUND_Y` at -61, so it
#: has five blocks to fall and lands on its seventeenth tick -- S(16) = 4.93,
#: S(17) would be 5.51 -- which is stepped tick 18, and its sand comes to rest
#: at y=-60. The high one is placed at y=300 and falls S(199) = 301.8 blocks, to
#: y = -1.8: still 58 blocks and, at the terminal speed, some thirty further
#: ticks above the y=-60 it would land at. That is the margin the scenario needs
#: at both ends, and the step count cannot be raised much past 220 without
#: eating it. `FallingBlockEntity.tick` only discards a long flight past
#: `time > 600`, or past `time > 100` outside the world's height
#: (FallingBlockEntity.java:172), and 199 ticks between y=300 and y=-1.8 reaches
#: neither.
ENTITY_FALL_AND_LAND = Scenario(
    name="entity_fall_and_land",
    owner="FallingBlock.tick -> FallingBlockEntity spawn/discard -> EntityStorage",
    row="server-entity-runtime",
    setup=(
        "setblock 38 -55 8 minecraft:sand",
        "setblock 34 300 8 minecraft:sand",
    ),
    box=Box(34, -61, 8, 38, -55, 8),
    entities=EntityProbe(chunk_x=2, chunk_z=0),
)

#: A redstone lamp that was lit and then had its power taken away.
#:
#: `RedstoneLampBlock.neighborChanged` (RedstoneLampBlock.java:36-49) is
#: asymmetric on purpose: a lamp that is not lit and finds a signal writes
#: itself lit on the spot (:45), while a lit lamp that finds none does not
#: switch off, it schedules a tick four ticks out (:43). So the setup below
#: leaves the save holding a *lit* lamp with a pending block tick, and what a
#: server has to do on its own is run that tick. `RedstoneLampBlock.tick`
#: (RedstoneLampBlock.java:52-56) is what switches it off.
#:
#: The power is a `minecraft:redstone_block`, whose `PoweredBlock.getSignal`
#: (PoweredBlock.java:28-30) answers fifteen on every side. It is placed and
#: then removed inside the setup, which runs on a frozen server: `/setblock`
#: does not go through `UPDATE_NEIGHBORS`, it calls
#: `ServerLevel.updateNeighboursOnBlockSet` (ServerLevel.java:842-854)
#: afterwards, and that runs whether or not the game is frozen -- freezing only
#: stops `ServerLevel.tick`. The immediate half of the scenario is therefore
#: already baked into the capture, and the pending tick is the half that is not.
#:
#: Nothing in the compared box is randomly ticked: neither
#: `minecraft:redstone_lamp` (Blocks.java:2639-2648) nor
#: `minecraft:redstone_block` (Blocks.java:2881-2890) declares `randomTicks()`,
#: and neither overrides `isRandomlyTicking`.
REDSTONE_LAMP_RELEASE = Scenario(
    name="redstone_lamp_release",
    owner="ServerLevel.tick blockTicks -> RedstoneLampBlock.tick",
    row="server-redstone",
    setup=(
        "setblock 48 -60 4 minecraft:redstone_lamp",
        "setblock 49 -60 4 minecraft:redstone_block",
        "setblock 49 -60 4 minecraft:air",
    ),
    box=Box(48, -60, 4, 49, -60, 4),
)

#: The same pending tick, with the power put back before it fires.
#:
#: This is the branch a lamp that only remembered "a tick is due, switch off"
#: would get wrong. `RedstoneLampBlock.tick` (RedstoneLampBlock.java:52-56)
#: asks `hasNeighborSignal` a second time and does nothing when the answer came
#: back, and the fourth `/setblock` below is why the answer comes back. The
#: fourth one also writes nothing itself: `neighborChanged` (:41) compares the
#: lamp's own `LIT` against the signal, finds them equal again, and neither
#: writes nor schedules -- so the save carries exactly one pending tick, the one
#: the third `/setblock` armed.
#:
#: The re-powered lamp alone would settle into the shape the capture already
#: had, and this harness refuses to call that a measurement: a box the official
#: server did not change says nothing about the candidate. So a second lamp
#: stands in the same box two cells away, released and never re-powered, and it
#: is what makes the box move. A server that runs both ticks correctly puts out
#: the far lamp and leaves the near one alight; one that runs them naively puts
#: out both.
REDSTONE_LAMP_REPOWERED = Scenario(
    name="redstone_lamp_repowered",
    owner="ServerLevel.tick blockTicks -> RedstoneLampBlock.tick re-reading hasNeighborSignal",
    row="server-redstone",
    setup=(
        "setblock 48 -60 6 minecraft:redstone_lamp",
        "setblock 49 -60 6 minecraft:redstone_block",
        "setblock 49 -60 6 minecraft:air",
        "setblock 49 -60 6 minecraft:redstone_block",
        "setblock 51 -60 6 minecraft:redstone_lamp",
        "setblock 52 -60 6 minecraft:redstone_block",
        "setblock 52 -60 6 minecraft:air",
    ),
    box=Box(48, -60, 6, 52, -60, 6),
)


#: A torch tower whose upper torch is armed to go out.
#:
#: A standing redstone torch reads one cell: `hasNeighborSignal`
#: (RedstoneTorchBlock.java:68-69) asks whether the block it stands on carries a
#: signal. A block only carries one if something powers it *directly*, and the
#: only directly-powering block this server owns is another torch, which powers
#: the cell above itself (RedstoneTorchBlock.java:104-105 answers `DOWN`, and
#: `getDirectSignalTo` SignalGetter.java:17-46 asks the cell below with exactly
#: that direction). So the scenario is the classic tower: a torch, the block it
#: charges, and a second torch standing on that block.
#:
#: The order of the three writes is what leaves work behind. The upper torch is
#: placed while its support is still cold, so it is correctly lit; the lower
#: torch is placed last, and its `notifyNeighbors`
#: (RedstoneTorchBlock.java:48-53) updates the neighbours of the cell above it,
#: which is how the upper torch hears about a support it cannot see change.
#: `neighborChanged` (RedstoneTorchBlock.java:97-99) then finds `LIT` and the
#: signal agreeing -- both true -- and schedules the two-tick toggle
#: (RedstoneTorchBlock.java:31) rather than writing anything. The prepared save
#: therefore holds a lit torch over a charged block with one tick pending, and
#: the step is what turns it off (RedstoneTorchBlock.java:81-83).
#:
#: The lower torch is the control: its own support is the ground, which nothing
#: charges, so it must still be lit afterwards.
REDSTONE_TORCH_TOWER = Scenario(
    name="redstone_torch_tower",
    owner="ServerLevel.tick blockTicks -> RedstoneTorchBlock.tick",
    row="server-redstone",
    setup=(
        "setblock 54 -59 4 minecraft:stone",
        "setblock 54 -58 4 minecraft:redstone_torch",
        "setblock 54 -60 4 minecraft:redstone_torch",
    ),
    box=Box(54, -61, 4, 54, -58, 4),
)

#: A repeater whose delay has not run out yet.
#:
#: This is the first redstone scenario whose *pending* half is the block's own
#: rather than something else's. `DiodeBlock.neighborChanged`
#: (DiodeBlock.java:84-98) never writes: it reaches `checkTickOnNeighbor`
#: (:100-115), which arms a tick `getDelay` ticks out and returns.
#: `RepeaterBlock.getDelay` is `DELAY * 2` (RepeaterBlock.java:52-54), so the
#: `delay=4` below leaves an eight-tick tick in the save, and
#: `DiodeBlock.tick` (:54-67) is the only thing that can turn the repeater on.
#:
#: The order of the three writes is what leaves that work behind. The repeater
#: goes down first with nothing feeding it, then the lamp it points at, and the
#: `minecraft:redstone_block` last -- so the only `/setblock` that reaches the
#: repeater's `neighborChanged` with a signal to find is the last one, and it
#: arms rather than writes. Freezing only stops `ServerLevel.tick`;
#: `SetBlockCommand` runs `ServerLevel.updateNeighboursOnBlockSet` whatever the
#: tick loop is doing, which is why the arming half is already baked into the
#: capture and the firing half is not.
#:
#: `FACING` on a diode names the cell it reads its **input** from
#: (DiodeBlock.java:126-127), and the cell it feeds is the opposite one
#: (:178-179), so `facing=west` reads x-1 and drives x+1. The lamp is what makes
#: the scenario say more than "a property flipped": `DiodeBlock.tick` writes with
#: the bare `Block.UPDATE_CLIENTS` and runs no neighbour fan-out of its own
#: (:59, :61), so the lamp can only light if the write went through
#: `LevelChunk.setBlockState`'s `onPlace` (LevelChunk.java:326-328) into
#: `DiodeBlock.updateNeighborsInFront` (:177-183). A server that flipped
#: `POWERED` and stopped there leaves the lamp dark.
#:
#: Nothing in the compared box is randomly ticked: `minecraft:repeater`
#: (Blocks.java:2186-2188), `minecraft:redstone_lamp` (Blocks.java:2639-2648)
#: and `minecraft:redstone_block` (Blocks.java:2881-2890) all declare no
#: `randomTicks()` and none overrides `isRandomlyTicking`.
REPEATER_DELAY_RELEASE = Scenario(
    name="repeater_delay_release",
    owner="ServerLevel.tick blockTicks -> DiodeBlock.tick -> updateNeighborsInFront",
    row="server-redstone",
    setup=(
        "setblock 56 -60 10 minecraft:repeater[delay=4,facing=west]",
        "setblock 57 -60 10 minecraft:redstone_lamp",
        "setblock 55 -60 10 minecraft:redstone_block",
    ),
    box=Box(55, -60, 10, 57, -59, 10),
)

#: Three repeaters in one box: one that has to switch itself back off, one that
#: has to stay off because it is locked, and one that simply switches on.
#:
#: The third is the witness, and it is here for the reason
#: `REDSTONE_LAMP_REPOWERED`'s second lamp is: the two interesting repeaters both
#: *end* in the state the save already had, and this harness refuses to call a
#: box the official server did not move a measurement. A wrong server does not
#: end there, which is the whole point -- but the witness is what makes the run
#: a measurement at all.
#:
#: **The pulse** (x=59..61). The power is placed and taken away again before the
#: eight-tick tick fires, and nothing cancels a scheduled diode tick: the second
#: `/setblock` reaches `checkTickOnNeighbor` with `POWERED` and `shouldTurnOn`
#: both false, and `:104` only arms when they differ. So the tick still lands,
#: on an input that is gone. `DiodeBlock.tick`'s `else if (!on)` branch (:60-65)
#: writes `POWERED=true` anyway and then, because `shouldTurnOn` is false,
#: schedules *another* tick one delay later at `TickPriority.VERY_HIGH` -- and
#: that second tick is what switches the repeater off again. Those four lines
#: are why a repeater passes a pulse shorter than its own delay instead of
#: latching on, and a server that reads them as redundant ends this scenario
#: with a lit lamp and a powered repeater.
#:
#: **The lock** (x=67..68, z=12..14). `RepeaterBlock.isLocked`
#: (RepeaterBlock.java:83-85) is `getAlternateSignal(...) > 0`, and
#: `sideInputDiodesOnly` (:88-90) makes `SignalGetter.getControlInputSignal`
#: answer zero for anything that is not a `DiodeBlock` (SignalGetter.java:50-51)
#: -- so the side input has to be another repeater. The target is armed first
#: and locked afterwards, which leaves a save holding a pending tick that must
#: not do anything: `DiodeBlock.tick` re-asks `isLocked` (:55) rather than
#: trusting the schedule. The locking repeater has a redstone block behind it so
#: it is genuinely powered rather than merely written that way, which is what
#: keeps it from arming a tick of its own the first time anything nudges it.
#:
#: **The witness** (x=63..65) is `REPEATER_DELAY_RELEASE` at `delay=1`: two
#: ticks, then a powered repeater and a lit lamp.
#:
#: The three groups are separated by an air cell each and share nothing. A
#: redstone block powers no block it stands on -- `PoweredBlock` declares
#: `getSignal` and no `getDirectSignal` (PoweredBlock.java:10-31), so the dirt
#: under one carries nothing -- and a repeater feeds exactly one cell, so the
#: only paths between the groups are the ones the setup draws. Nothing in the
#: box is randomly ticked, for the reasons `REPEATER_DELAY_RELEASE` states.
REPEATER_PULSE_AND_LOCK = Scenario(
    name="repeater_pulse_and_lock",
    owner="ServerLevel.tick blockTicks -> DiodeBlock.tick re-arm and RepeaterBlock.isLocked",
    row="server-redstone",
    setup=(
        # The pulse.
        "setblock 60 -60 12 minecraft:repeater[delay=4,facing=west]",
        "setblock 61 -60 12 minecraft:redstone_lamp",
        "setblock 59 -60 12 minecraft:redstone_block",
        "setblock 59 -60 12 minecraft:air",
        # The witness.
        "setblock 64 -60 12 minecraft:repeater[delay=1,facing=west]",
        "setblock 65 -60 12 minecraft:redstone_lamp",
        "setblock 63 -60 12 minecraft:redstone_block",
        # The lock: arm the target, then stand a powered repeater on its
        # counter-clockwise side. `counterClockWise(west)` is south
        # (Direction.java:245-253), so the side cell is z+1 and the repeater
        # standing there has to read its own input from z+2 to feed this way.
        "setblock 68 -60 12 minecraft:repeater[delay=4,facing=west]",
        "setblock 67 -60 12 minecraft:redstone_block",
        "setblock 68 -60 14 minecraft:redstone_block",
        "setblock 68 -60 13 minecraft:repeater[delay=1,facing=south,powered=true]",
    ),
    box=Box(59, -60, 12, 68, -59, 14),
)

SCENARIOS: tuple[Scenario, ...] = (
    FLUID_TRENCH,
    FALLING_SAND_COLUMN,
    LEAF_DISTANCE_CASCADE,
    LEAF_DECAY_PATCH,
    CHORUS_SURVIVAL_TICK,
    CHORUS_RANDOM_GROWTH,
    ENTITY_FALL_AND_LAND,
    REDSTONE_LAMP_RELEASE,
    REDSTONE_LAMP_REPOWERED,
    REDSTONE_TORCH_TOWER,
    REPEATER_DELAY_RELEASE,
    REPEATER_PULSE_AND_LOCK,
)

#: What `LEAF_DECAY_PATCH` needs the gamerule to be. See its probability
#: argument; `GameRules.RANDOM_TICK_SPEED` (GameRules.java:74) is registered
#: with a minimum of 0 and no maximum -- the four-argument `registerInteger`
#: hands `Integer.MAX_VALUE` on (GameRules.java:181-183) -- and the cost is
#: bounded by the two randomly-ticking sections, one holding the decaying leaves
#: and one holding the chorus growth flower.
#:
#: Raising it globally is only safe because nothing else any scenario compares
#: is randomly ticked. Every block that can appear inside a compared box was
#: checked against its `BlockBehaviour.Properties` and any `isRandomlyTicking`
#: override:
#:
#: * `minecraft:air`, `minecraft:stone`, `minecraft:dirt`, `minecraft:bedrock`
#:   and `minecraft:sand` are declared without `randomTicks()`
#:   (Blocks.java:53, :54-57, :85, :275-283, :312-316) and override nothing.
#: * `minecraft:water` declares no `randomTicks()` either (Blocks.java:284-297),
#:   and both halves of the fluid answer false: `LiquidBlock.isRandomlyTicking`
#:   (LiquidBlock.java:105-107) defers to the fluid, whose base answer is false
#:   (Fluid.java:79-81) and is overridden only by lava (LavaFluid.java:221-223).
#: * `minecraft:oak_leaves` does declare `randomTicks()`
#:   (Blocks.java:6958-6971) but overrides `isRandomlyTicking` to
#:   `distance == 7 && !persistent` (LeavesBlock.java:62-64), which is exactly
#:   `LEAF_DECAY_PATCH`'s nine and none of `LEAF_DISTANCE_CASCADE`'s six.
#: * `minecraft:chorus_flower` declares random ticks (Blocks.java:4281-4294)
#:   but accepts them only below age five (ChorusFlowerBlock.java:54-55).
#:   `CHORUS_RANDOM_GROWTH` is built for those ticks, while its age-five flower
#:   is terminal; end stone and chorus plant do not declare random ticks
#:   (Blocks.java:2625-2628 and Blocks.java:4270-4280).
#: * `minecraft:grass_block` is the one block on the capture's surface that does
#:   (Blocks.java:82-84), which is what `GROUND_BLOCK` exists to remove.
#:
#: The one thing that still runs at this rate outside a section is
#: `ServerLevel.tickChunk`'s precipitation loop (ServerLevel.java:487-493). It
#: only ices and snows, both of which need `biome.shouldFreeze` or a raining
#: level (ServerLevel.java:564-593); the capture is plains and `advance_weather`
#: is off, so it writes nothing.
RANDOM_TICK_SPEED = 1024

#: The flat world's top layer, and what this harness replaces it with.
#:
#: `FlatLevelGeneratorSettings.getDefault` (FlatLevelGeneratorSettings.java:217-219)
#: lays one bedrock, two dirt and one grass block, so the capture's surface is
#: `minecraft:grass_block` at y=-61 and the first air is y=-60. Grass is
#: randomly ticked, and `SpreadingSnowyBlock.randomTick` turns a grass block
#: that cannot stay alive into its base block (SpreadingSnowyBlock.java:50-55) --
#: which is every grass block this harness builds on top of. With
#: `RANDOM_TICK_SPEED` raised that would land inside boxes that are about
#: something else entirely, so the surface is converted up front instead:
#: `minecraft:dirt` declares no random tick, and because the conversion covers
#: every forceloaded column there is no grass left anywhere that ticks to spread
#: back into it (SpreadingSnowyBlock.java:57-65 only reaches one block out).
GROUND_Y = -61
GROUND_BLOCK = "minecraft:dirt"

#: Run before anything is placed. The freeze has to come first so that no
#: scenario makes progress while the rest of the script is still being typed.
#:
#: `spawn_mobs`, `advance_weather` and `advance_time` switch off the three
#: sources of world change no scenario here is about. `block_drops` is off
#: because `LEAF_DECAY_PATCH` drops its leaves through `Block.popResource`,
#: which is gated on that rule (Block.java:442-446) and whose item entities
#: would otherwise land in the same `entities/` files `ENTITY_FALL_AND_LAND`
#: reads, at offsets drawn from the level's own `RandomSource`.
#: `random_tick_speed` is raised rather than switched off; see
#: `LEAF_DECAY_PATCH` for why that is safe for every other scenario.
PREAMBLE = (
    FREEZE_COMMAND,
    f"gamerule random_tick_speed {RANDOM_TICK_SPEED}",
    "gamerule block_drops false",
    "gamerule spawn_mobs false",
    "gamerule advance_weather false",
    "gamerule advance_time false",
)


def forceloaded_columns(scenarios: Sequence[Scenario]) -> list[tuple[int, int]]:
    """Every chunk column a run has to keep ticking, in a stable order.

    A box three chunks wide has a middle column no corner names. A scenario that
    reads `entities/` names its column directly, and that column has to tick
    too -- an entity in an unloaded column is an entity nothing moves.
    """
    columns = {column for scenario in scenarios for column in scenario.box.chunk_columns()}
    columns.update(
        (scenario.entities.chunk_x, scenario.entities.chunk_z)
        for scenario in scenarios
        if scenario.entities is not None
    )
    return sorted(columns)


def forceload_commands(scenarios: Sequence[Scenario]) -> list[str]:
    """One `/forceload add` per column the run keeps ticking."""
    return [f"forceload add {x * 16} {z * 16}" for x, z in forceloaded_columns(scenarios)]


def ground_commands(scenarios: Sequence[Scenario]) -> list[str]:
    """One `/fill` per forceloaded column, replacing the grass surface.

    See `GROUND_BLOCK`. The whole column is converted, not only the footprint of
    a scenario, because a single surviving grass block spreads back into the
    dirt beside it.
    """
    return [
        f"fill {x * 16} {GROUND_Y} {z * 16} {x * 16 + 15} {GROUND_Y} {z * 16 + 15} {GROUND_BLOCK}"
        for x, z in forceloaded_columns(scenarios)
    ]


def setup_script(scenarios: Sequence[Scenario]) -> list[str]:
    """Every console line the preparing server is given, in order."""
    lines = list(PREAMBLE)
    lines.extend(forceload_commands(scenarios))
    lines.extend(ground_commands(scenarios))
    for scenario in scenarios:
        lines.extend(scenario.setup)
    return lines


def describe(state: BlockState | None) -> str:
    """How a read block is named in the result, including a missing chunk."""
    return "<unwritten>" if state is None else state.describe()


def numeric_delta(one: Any, other: Any) -> float | None:
    """How far apart two entity field values are, when that is a number.

    `Pos` and `Motion` are lists of doubles, and two implementations running the
    same arithmetic on the same doubles should land on the same bits. Reporting
    the distance as well as the two values is what lets a reader tell a last-bit
    difference from a server that is a tick behind.
    """
    if isinstance(one, bool) or isinstance(other, bool):
        return None
    if isinstance(one, int | float) and isinstance(other, int | float):
        return float(abs(one - other))
    if isinstance(one, list) and isinstance(other, list) and len(one) == len(other):
        deltas = [numeric_delta(left, right) for left, right in zip(one, other, strict=True)]
        if any(delta is None for delta in deltas):
            return None
        return max(deltas) if deltas else 0.0
    return None


def project_entity(record: Mapping[str, Any], fields: Sequence[str]) -> dict[str, Any]:
    """One entity record cut down to the tags a comparison looks at."""
    return {field: record.get(field) for field in fields}


def order_entities(records: Sequence[Mapping[str, Any]]) -> list[Mapping[str, Any]]:
    """The records in a comparable order.

    Sorting is by `id` only, and Python's sort is stable, so records of one type
    keep the order the region file lists them in. That is unambiguous for the
    single-entity probe this harness ships; a probe holding several entities of
    one type would need a stronger key than a type name.
    """
    return sorted(records, key=lambda record: str(record.get("id", "")))


def compare_entities(
    probe: EntityProbe,
    official: Sequence[Mapping[str, Any]],
    candidate: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    """One entity column's verdict, record by record and field by field."""
    left = order_entities(official)
    right = order_entities(candidate)
    records: list[dict[str, Any]] = []
    differences = 0
    for index in range(max(len(left), len(right))):
        one = project_entity(left[index], probe.fields) if index < len(left) else None
        other = project_entity(right[index], probe.fields) if index < len(right) else None
        missing = None
        if one is None:
            missing = "official"
        elif other is None:
            missing = "bbb"
        fields: list[dict[str, Any]] = []
        if one is not None and other is not None:
            fields = [
                {
                    "field": field,
                    "official": one[field],
                    "bbb": other[field],
                    "delta": numeric_delta(one[field], other[field]),
                }
                for field in probe.fields
                if one[field] != other[field]
            ]
        if missing is not None or fields:
            differences += 1
        records.append(
            {
                "index": index,
                "missing": missing,
                "official": one,
                "bbb": other,
                "differences": fields,
            }
        )
    return {
        **probe.as_json(),
        "official_count": len(left),
        "bbb_count": len(right),
        "records": records,
        "difference_count": differences,
        # Two empty lists agree on every record they both have, which is none:
        # a server that never spawned an entity, one that spawned it in the
        # wrong column and one whose entity storage it never reached all look
        # identical here. `WorldSaveReader.entities` cannot tell an unwritten
        # column from an empty one either, so the only honest reading of this
        # is "nothing was measured" -- `compare_scenario` turns it into a named
        # failure rather than letting it total up as agreement.
        "empty_on_both_sides": not left and not right,
    }


def scenario_not_measured(
    scenario: Scenario,
    prepared: Mapping[tuple[int, int, int], BlockState | None],
    official_changed: int,
    entity: Mapping[str, Any] | None,
) -> list[str]:
    """Why this scenario is not a measurement, if it is not one.

    Two servers can agree perfectly on a comparison that asked them nothing,
    and a run that reports that as a pass is worse than no run: it is a green
    receipt for behaviour nobody exercised. The three ways that happens here
    are a setup that never landed, a step in which the authority itself did
    nothing, and an entity probe neither side wrote a record into -- and none
    of them shows up as a difference, because there is nothing to differ.
    """
    reasons: list[str] = []
    inputs = [prepared.get(position) for position in scenario.box.positions()]
    if all(state is None or state.name == "minecraft:air" for state in inputs):
        reasons.append(
            "the prepared save holds nothing but air inside the compared box, so the "
            "scenario's setup never landed"
        )
    if official_changed == 0:
        reasons.append(
            "the official server changed no block in the compared box during the step, so "
            "there is no behaviour here for the candidate to match"
        )
    if entity is not None and entity["empty_on_both_sides"]:
        reasons.append(
            "neither side wrote an entity into the compared column, so the entity halves "
            "agree only because both are empty"
        )
    return reasons


def compare_scenario(
    scenario: Scenario,
    prepared: Mapping[tuple[int, int, int], BlockState | None],
    official: Mapping[tuple[int, int, int], BlockState | None],
    candidate: Mapping[tuple[int, int, int], BlockState | None],
    *,
    official_entities: Sequence[Mapping[str, Any]] | None = None,
    candidate_entities: Sequence[Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    """One scenario's verdict, with every disagreeing coordinate named.

    Each difference carries the prepared input as well as the two outputs, so a
    reader can tell the two failures apart without opening a save: a candidate
    that still holds the input did nothing, while one that holds a third value
    did something else.

    `compared` and `matched` stay block counts, so the totals a reader already
    knows keep meaning what they meant; the entity half carries its own counts
    and both feed `difference_count`.
    """
    differences: list[dict[str, Any]] = []
    still_input = 0
    for position in scenario.box.positions():
        expected = official.get(position)
        actual = candidate.get(position)
        if expected == actual:
            continue
        was = prepared.get(position)
        if actual == was:
            still_input += 1
        differences.append(
            {
                "x": position[0],
                "y": position[1],
                "z": position[2],
                "prepared": describe(was),
                "official": describe(expected),
                "bbb": describe(actual),
                "bbb_unchanged_from_input": actual == was,
            }
        )
    entity = None
    if scenario.entities is not None:
        entity = compare_entities(
            scenario.entities, official_entities or (), candidate_entities or ()
        )
    entity_differences = 0 if entity is None else entity["difference_count"]
    compared = scenario.box.volume
    official_changed = sum(
        1
        for position in scenario.box.positions()
        if official.get(position) != prepared.get(position)
    )
    unmeasured = scenario_not_measured(scenario, prepared, official_changed, entity)
    return {
        **scenario.as_json(),
        "compared": compared,
        "matched": compared - len(differences),
        "differences": differences,
        "block_difference_count": len(differences),
        "entity": entity,
        "entity_difference_count": entity_differences,
        "difference_count": len(differences) + entity_differences,
        # How much of the box the authority itself moved. Zero is not a passing
        # comparison of two agreeing servers, it is a scenario that measured
        # nothing; `not_measured` says which way it failed.
        "official_changed_from_input": official_changed,
        "not_measured": unmeasured,
        "measured": not unmeasured,
        # A scenario whose every block difference is still the prepared input
        # says the candidate never ran the behaviour at all, which is a
        # different report from one that ran it and landed somewhere else.
        "bbb_made_no_change": len(differences) > 0 and still_input == len(differences),
        "pass": not differences and entity_differences == 0 and not unmeasured,
    }


def build_result(
    scenarios: Sequence[dict[str, Any]],
    *,
    step_ticks: int,
    world_template: Path,
    game_times: Mapping[str, int | None],
    stepped: Mapping[str, int] | None = None,
    generated_at: str | None = None,
) -> dict[str, Any]:
    """The machine-readable comparison, whether it passed or not."""
    if generated_at is None:
        generated_at = (
            datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")
        )
    compared = sum(entry["compared"] for entry in scenarios)
    differences = sum(entry["difference_count"] for entry in scenarios)
    # A scenario that measured nothing reports no difference, because there was
    # nothing to differ. It must still fail the run, or the receipt would say
    # the two servers agreed about behaviour neither of them was asked for.
    unmeasured = sorted(entry["name"] for entry in scenarios if entry.get("not_measured"))
    prepared = game_times.get("prepared")
    expected = None if prepared is None else prepared + step_ticks
    return {
        "schema_version": SCHEMA_VERSION,
        "vanilla_version": VANILLA_VERSION,
        "generated_at": generated_at,
        "world_template": str(world_template),
        "step": {
            "ticks": step_ticks,
            "seconds": step_ticks / TICKS_PER_SECOND,
            "freeze_command": FREEZE_COMMAND,
            # How many of the run's ticks each side was actually told to take.
            # The rest are the ones it ran between opening the save and its
            # freeze landing, which is a race neither side can win twice; both
            # halves are the same tick, and the total is what `step_ticks`
            # names.
            "stepped": dict(stepped or {}),
            "expected_game_time_delta": step_ticks,
            "expected_game_time": expected,
        },
        "dimension": DIMENSION,
        "preamble": list(PREAMBLE),
        "game_time": {
            **dict(game_times),
            "expected": expected,
            "expected_delta": step_ticks,
        },
        "compared": compared,
        "difference_count": differences,
        "unmeasured": unmeasured,
        "pass": differences == 0 and not unmeasured,
        "scenarios": list(scenarios),
    }


def step_progress(base_time: int, observed: int | None, ticks: int) -> str:
    """Where a side is against the step it was told to take.

    `"reached"` is the only acceptable answer at the end. `"short"` covers a
    save that has not caught up yet -- and a save with no `Data.Time` at all,
    which is a save that has not been written. `"past"` is a side that ran ticks
    nobody asked it for, which is the failure a wall-clock harness could never
    see: it looks exactly like success from the outside.
    """
    target = base_time + ticks
    if observed is None:
        return "short"
    if observed == target:
        return "reached"
    return "short" if observed < target else "past"


def require_stepped(name: str, base_time: int, observed: int | None, ticks: int) -> int:
    """The side's game time, or a named failure saying how far off it is.

    This is the assertion the whole tick-precise design exists for: both sides
    are told to run exactly `ticks` ticks, and `level.dat`'s `Data.Time` is the
    receipt that says they did.
    """
    progress = step_progress(base_time, observed, ticks)
    if progress == "reached" and observed is not None:
        return observed
    target = base_time + ticks
    if observed is None:
        raise OracleError(
            f"the {name} server's save carries no Data.Time, so nothing says it ran the "
            f"{ticks}-tick step from {base_time}"
        )
    raise OracleError(
        f"the {name} server ran {observed - base_time} ticks, not the {ticks} it was "
        f"stepped: level.dat Data.Time is {observed}, {abs(observed - target)} "
        f"{'past' if progress == 'past' else 'short of'} the {target} the step should reach"
    )


class ConsoleServer(OfficialServer):
    """An official server this harness can type at, on its own copy of a save.

    `OfficialServer` already owns everything about booting Mojang's 26.1 jar for
    this repository: the properties every replay shares, the copied world, the
    dropped session lock, the `Done (` readiness marker and the shutdown. It
    launches with the console closed, because the release families only ever
    drive the official server over the network. This harness has no network
    driver at all -- the scenario is typed at the console, which is the only
    surface that can build a world without a client -- so the one thing this
    subclass changes is that `stdin` is a pipe. Everything else is inherited.
    """

    def __init__(self, paths: Paths, world_dir: Path, port: int, name: str) -> None:
        super().__init__(paths, world_dir, port)
        self.name = name
        # `OfficialServer.__init__` puts its run under the release gate's
        # creation family by name. This harness runs the official server more
        # than once and keeps each under `official/<name>`, so the empty
        # directory that naming just made is dropped and the artifacts show
        # only what this harness wrote.
        stray = paths.artifact_dir / "creation"
        if stray.is_dir() and not any(stray.iterdir()):
            stray.rmdir()
        self.root = paths.child("official", name)
        self.log_path = paths.child("official", f"{name}.log")
        self.console: IO[bytes] | None = None

    @property
    def world(self) -> Path:
        """The save this run worked on, which is the copy, never the template."""
        return self.root / "world"

    def start(self) -> None:
        require(
            self.paths.java_bin is not None and self.paths.server_jar is not None,
            "the world oracle boots the official 26.1 server; run it through "
            "scripts/server-world-oracle-26.1.sh",
        )
        self.root.mkdir(parents=True, exist_ok=True)
        if self.world.exists():
            shutil.rmtree(self.world)
        shutil.copytree(self.world_dir, self.world)
        # The session lock belongs to the process that held it; copying one
        # forward would have this server refuse a save nobody is using.
        (self.world / "session.lock").unlink(missing_ok=True)
        (self.root / "eula.txt").write_text("eula=true\n", encoding="utf-8")
        shared = (REPO_ROOT / OFFICIAL_REPLAY_PROPERTIES).read_text(encoding="utf-8")
        (self.root / "server.properties").write_text(
            shared
            + "\n".join(
                [
                    f"server-port={self.port}",
                    "level-name=world",
                    f"motd=bbb world oracle {self.name} 26.1",
                    # Nobody ever connects to this server, and
                    # `DedicatedServerProperties` defaults
                    # `pause-when-empty-seconds` to 60
                    # (DedicatedServerProperties.java:126). At that point
                    # `MinecraftServer.tickServer` stops calling
                    # `tickRateManager.tick()` and `tickChildren` altogether
                    # (MinecraftServer.java:956-971), so a step longer than the
                    # boot plus a minute would silently stall with the harness
                    # waiting for ticks the server had stopped running. Zero
                    # switches the whole branch off, since it is guarded on the
                    # threshold being positive. This is written here rather than
                    # into the shared properties file because that file is
                    # pinned by scripts/tests/test_capture_replay_expectations.py
                    # for a harness that does have clients.
                    "pause-when-empty-seconds=0",
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        # Each name is booted once per run and the log is read back for markers
        # and for refused commands, so it is truncated rather than appended to:
        # a leftover log from an earlier run in the same artifact directory
        # would otherwise be read as this run's evidence.
        handle = self.log_path.open("wb")
        self.process = subprocess.Popen(
            [
                str(self.paths.java_bin),
                "-Xms512M",
                "-Xmx1G",
                "-jar",
                str(self.paths.server_jar),
                "nogui",
            ],
            cwd=self.root,
            stdin=subprocess.PIPE,
            stdout=handle,
            stderr=subprocess.STDOUT,
        )
        handle.close()
        self.console = self.process.stdin

    def send(self, commands: Sequence[str]) -> None:
        console = self.console
        if console is None:
            raise OracleError(f"the {self.name} server has no console")
        for command in commands:
            console.write(f"{command}\n".encode())
        console.flush()

    def log_text(self) -> str:
        if not self.log_path.is_file():
            return ""
        return self.log_path.read_text(encoding="utf-8", errors="replace")

    def marker_count(self, marker: str) -> int:
        return self.log_text().count(marker)

    def wait_for_marker(self, marker: str, minimum: int, timeout: float) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.marker_count(marker) >= minimum:
                return
            if self.process is not None and self.process.poll() is not None:
                raise OracleError(
                    f"the {self.name} server exited before {marker!r}; log: {self.log_path}"
                )
            time.sleep(0.25)
        raise OracleError(f"the {self.name} server never logged {marker!r}; log: {self.log_path}")

    def require_commands_accepted(self) -> None:
        """Fail on a console line the server could not parse or could not run.

        A refused command is silent otherwise: the server keeps running and the
        scenario is simply not there, which would show up as a passing
        comparison of two empty boxes.
        """
        text = self.log_text()
        refused = [
            line
            for line in text.splitlines()
            if any(marker in line for marker in COMMAND_FAILURE_MARKERS)
        ]
        if refused:
            raise OracleError(
                f"the {self.name} server refused a console command: {refused[0].strip()}"
                f" (log: {self.log_path})"
            )

    def freeze(self) -> None:
        """Stop the tick loop, and wait until the server says it has."""
        frozen = self.marker_count(FROZEN_TEXT)
        self.send([FREEZE_COMMAND])
        self.wait_for_marker(FROZEN_TEXT, frozen + 1, COMMAND_TIMEOUT)

    def step(self, ticks: int) -> None:
        """Ask for exactly `ticks` ticks, and wait until the ask was taken.

        The answer only says the command was accepted -- `stepGameIfPaused`
        sets `frozenTicksToRun` and returns (ServerTickRateManager.java:41-49),
        and the ticks run afterwards -- so the caller still has to wait for the
        game clock; what this refuses to do is start that wait against a server
        that never took the command.

        The refusal is watched for beside the success. `TickCommand.step`
        returns 1 either way (TickCommand.java:140), so `sendFailure`'s line is
        the only thing that tells them apart, and a run that only waited for
        the success would spend the whole command timeout before saying so.
        """
        command = step_command(ticks)
        wanted = step_success_text(ticks)
        stepped = self.marker_count(wanted)
        self.send([command])
        deadline = time.monotonic() + COMMAND_TIMEOUT
        while self.marker_count(wanted) <= stepped:
            # Both the parse errors and the "must be frozen first" refusal.
            self.require_commands_accepted()
            if self.process is not None and self.process.poll() is not None:
                raise OracleError(
                    f"the {self.name} server exited before it answered `/{command}`; "
                    f"log: {self.log_path}"
                )
            if time.monotonic() >= deadline:
                raise OracleError(
                    f"the {self.name} server never answered `/{command}` with {wanted!r}; "
                    f"log: {self.log_path}"
                )
            time.sleep(0.25)
        self.require_commands_accepted()

    def save(self) -> None:
        saved = self.marker_count(SAVED_MARKER)
        self.send(["save-all flush"])
        self.wait_for_marker(SAVED_MARKER, saved + 1, SAVE_TIMEOUT)

    def save_and_stop(self) -> None:
        self.save()
        self.send(["stop"])
        console = self.console
        if console is not None:
            console.close()
            self.console = None
        if self.process is not None:
            try:
                self.process.wait(timeout=BOOT_TIMEOUT)
            except subprocess.TimeoutExpired as error:
                self.process.kill()
                raise OracleError(
                    f"the {self.name} server did not stop; log: {self.log_path}"
                ) from error


def step_to_game_time(
    name: str,
    world: Path,
    freeze: Callable[[], None],
    step: Callable[[int], None],
    save: Callable[[], None],
    target_time: int,
) -> int:
    """Freeze a side, then step it until its clock reads exactly `target_time`.

    The step count is not a constant, because the number of ticks a side runs
    before its freeze lands is not one. Both servers open the same prepared
    save and then tick freely between the moment they are ready and the moment
    the freeze command reaches them, and no amount of care about how fast the
    command is sent makes that number the same twice -- it is a race between a
    console pipe and a 50 ms clock.

    Counting from a fixed origin instead removes the race from the measurement
    without removing any of the assertion. The prepared save's `Data.Time` is
    the origin both sides share; each side is frozen, asked what its own clock
    says, and then stepped by the difference. Both therefore run the same total
    number of world ticks over the same input, which is what the scenario
    arithmetic counts, and a side that could not be brought to the target is a
    named failure rather than a comparison at the wrong moment.

    A frozen tick and a stepped one are the same tick: `TickRateManager.tick`
    (TickRateManager.java:56-61) sets `runGameElements` from
    `!isFrozen || frozenTicksToRun > 0`, so the flag the world reads is the
    same either way and nothing distinguishes the ticks a side took before its
    freeze from the ones it was told to take.
    """
    freeze()
    # The save is what makes the frozen clock readable; nothing else on either
    # side publishes it.
    save()
    frozen_at = game_time(world)
    if frozen_at is None:
        raise OracleError(
            f"the {name} server's save carries no Data.Time once frozen, so nothing says "
            "where its step should start"
        )
    steps = target_time - frozen_at
    if steps < 1:
        raise OracleError(
            f"the {name} server was already at Data.Time {frozen_at} when it froze, at or "
            f"past the {target_time} both sides are stepped to: it ran "
            f"{frozen_at - target_time} ticks more than the run has to give it"
        )
    step(steps)
    await_step(name, world, save, frozen_at, steps)
    return steps


def await_step(
    name: str, world: Path, save: Callable[[], None], base_time: int, ticks: int
) -> int:
    """Wait until a side's own save says it ran exactly `ticks` ticks.

    There is no console line and no management event that means "the step is
    finished": the step's own answer is logged the moment the counter is set.
    What both sides do publish is the save, and `Data.Time` inside it is the
    number under test -- so the wait is the assertion, polled. A side that runs
    past the target is reported the moment it is seen rather than waited out.
    """
    target = base_time + ticks
    # A stepped tick still costs a tick of wall clock, so the step cannot
    # possibly be finished before this and polling earlier only writes saves.
    time.sleep(ticks / TICKS_PER_SECOND)
    deadline = time.monotonic() + STEP_GRACE_SECONDS
    observed: int | None = None
    while True:
        save()
        observed = game_time(world)
        progress = step_progress(base_time, observed, ticks)
        if progress != "short":
            return require_stepped(name, base_time, observed, ticks)
        if time.monotonic() >= deadline:
            raise OracleError(
                f"the {name} server did not finish its {ticks}-tick step within "
                f"{STEP_GRACE_SECONDS:.0f}s of grace: level.dat Data.Time is {observed}, "
                f"not the {target} the step should reach"
            )
        time.sleep(STEP_POLL_SECONDS)


def prepare_world(paths: Paths, scenarios: Sequence[Scenario]) -> Path:
    """Build the shared input save, by the official server, on a frozen tick."""
    server = ConsoleServer(paths, world_template(paths), free_port(), "prepare")
    try:
        server.start()
        server.wait_ready(BOOT_TIMEOUT)
        server.send(setup_script(scenarios))
        # The console is a pipe, so a command is only known to have run once the
        # server has answered something after it. The save is that something.
        server.save_and_stop()
        server.require_commands_accepted()
    finally:
        server.terminate()
    return server.world


def run_official(paths: Paths, prepared: Path, target_time: int) -> tuple[Path, int]:
    """Open the prepared save on the official server and step it to the target."""
    server = ConsoleServer(paths, prepared, free_port(), "replay")
    try:
        server.start()
        server.wait_ready(BOOT_TIMEOUT)
        steps = step_to_game_time(
            "official", server.world, server.freeze, server.step, server.save, target_time
        )
        server.save_and_stop()
        server.require_commands_accepted()
    finally:
        server.terminate()
    return server.world, steps


def bbb_command(
    server: ServerProcess, command: str, *, expect_result: bool = True
) -> dict[str, Any]:
    """Run one command on bbb-server through the management endpoint.

    `command.execute` is the surface this server publishes for the one
    dispatcher, and the release gate already drives `/reload` through it. The
    string travels without its leading slash, the way
    `ServerboundChatCommandPacket` carries one.

    `expect_result=False` is for the commands vanilla itself answers without an
    int. A custom executor reaches its caller only through the frame, and a
    frame fires its consumer on `/return` alone (Frame.java:6-12), so a
    `/function` whose body simply ends produces no result on either server.
    """
    answer = server.management(
        "call", "command.execute", "--arguments-json", json.dumps({"command": command})
    )
    report = answer.get("command") if isinstance(answer, dict) else None
    if not isinstance(report, dict):
        raise OracleError(
            f"bbb-server did not answer `/{command}` with a command report: "
            f"{json.dumps(answer, ensure_ascii=False)[:400]}"
        )
    if expect_result and report.get("result") is None:
        raise OracleError(
            f"bbb-server refused `/{command}`; a command that ends in a syntax error has no "
            f"result, so this is the answer of a server that does not have the command: "
            f"{json.dumps(report, ensure_ascii=False)[:400]}"
        )
    return report


def require_answer(command: str, report: Mapping[str, Any], *, wanted: str, text: str) -> None:
    """Fail unless bbb's answer to `command` names the feedback it should.

    Both the translation key and the English are accepted, because the answer
    carries the component while a rendered log line carries the string, and this
    harness must not depend on which of the two a server chose to hand back.
    """
    blob = json.dumps(report, ensure_ascii=False)
    if wanted in blob or text in blob:
        return
    raise OracleError(
        f"bbb-server answered `/{command}` without {wanted} ({text!r}): {blob[:400]}"
    )


def refuse_answer(command: str, report: Mapping[str, Any], *, refused: str, text: str) -> None:
    """Fail when bbb's answer to `command` names a refusal."""
    blob = json.dumps(report, ensure_ascii=False)
    if refused in blob or text in blob:
        raise OracleError(f"bbb-server refused `/{command}`: {blob[:400]}")


def freeze_bbb(server: ServerProcess) -> None:
    """The `ConsoleServer.freeze` of the side that has no console."""
    frozen = bbb_command(server, FREEZE_COMMAND)
    require_answer(FREEZE_COMMAND, frozen, wanted=FROZEN_KEY, text=FROZEN_TEXT)


def step_bbb(server: ServerProcess, ticks: int) -> None:
    """The `ConsoleServer.step` of the side that has no console."""
    command = step_command(ticks)
    stepped = bbb_command(server, command)
    # `TickCommand.step` answers 1 whether or not the step was taken
    # (TickCommand.java:131-141), so the result code cannot tell them apart and
    # the feedback has to be read.
    refuse_answer(command, stepped, refused=STEP_FAILURE_KEY, text=STEP_FAILURE_TEXT)
    require_answer(command, stepped, wanted=STEP_SUCCESS_KEY, text=step_success_text(ticks))


def run_bbb(paths: Paths, prepared: Path, target_time: int) -> tuple[Path, int]:
    """Open the same save on `bbb-server`, with nobody connected to it.

    The save is flushed and the process stopped through the management endpoint
    rather than a signal, because that is the surface this server publishes for
    it and the release gate already proves it is the one that writes a save.
    """
    world = paths.child("bbb", "world")
    if world.exists():
        shutil.rmtree(world)
    shutil.copytree(prepared, world)
    (world / "session.lock").unlink(missing_ok=True)
    server = ServerProcess(
        paths=paths,
        name="bbb-client-world-oracle",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("bbb", "admin"),
        world_dir=world,
        loopback_scope=BBB_LOOPBACK_SCOPE,
    )

    def save() -> None:
        saved = server.count_events("world.saved")
        server.management("call", "server.save")
        deadline = time.monotonic() + SAVE_TIMEOUT
        while server.count_events("world.saved") <= saved:
            if time.monotonic() >= deadline:
                raise OracleError(f"bbb-server never logged world.saved; log: {server.log_path}")
            time.sleep(0.1)

    try:
        server.start()
        server.wait_ready()
        server.wait_for_event("world.loaded")
        steps = step_to_game_time(
            "bbb",
            world,
            lambda: freeze_bbb(server),
            lambda count: step_bbb(server, count),
            save,
            target_time,
        )
        save()
        server.stop_through_management()
    finally:
        server.terminate()
    return world, steps


def read_box(world: Path, box: Box) -> dict[tuple[int, int, int], BlockState | None]:
    reader = WorldSaveReader(world, DIMENSION)
    try:
        return {position: reader.block_state(*position) for position in box.positions()}
    except AnvilError as error:
        raise OracleError(f"cannot read {world}: {error}") from error


def read_entities(world: Path, probe: EntityProbe) -> list[dict[str, Any]]:
    reader = WorldSaveReader(world, DIMENSION)
    try:
        return reader.entities(probe.chunk_x, probe.chunk_z)
    except AnvilError as error:
        raise OracleError(f"cannot read the entity storage of {world}: {error}") from error


def game_time(world: Path) -> int | None:
    """`level.dat`'s `Data.Time` (PrimaryLevelData.java:133), the tick receipt."""
    level = world / "level.dat"
    if not level.is_file():
        return None
    try:
        data = read_compressed_nbt(level).get("Data")
    except (AnvilError, OSError):
        return None
    if not isinstance(data, dict):
        return None
    value = data.get("Time")
    return value if isinstance(value, int) else None


def run(paths: Paths, scenarios: Sequence[Scenario], ticks: int) -> dict[str, Any]:
    prepared = prepare_world(paths, scenarios)
    base_time = game_time(prepared)
    if base_time is None:
        raise OracleError(
            f"the prepared save at {prepared} carries no Data.Time, so there is nothing to "
            "measure a step against"
        )
    target_time = base_time + ticks
    official, official_steps = run_official(paths, prepared, target_time)
    candidate, bbb_steps = run_bbb(paths, prepared, target_time)
    # The saves both sides wrote on their way out, rather than the ones the step
    # was waited on: a side that kept ticking after its step would show up here
    # and nowhere else. Each is measured against the shared origin, so the
    # assertion is the total the run asked for rather than the part of it that
    # happened to be stepped.
    official_time = require_stepped("official", base_time, game_time(official), ticks)
    bbb_time = require_stepped("bbb", base_time, game_time(candidate), ticks)
    results = []
    for scenario in scenarios:
        entities: dict[str, list[dict[str, Any]]] = {}
        if scenario.entities is not None:
            entities = {
                "official_entities": read_entities(official, scenario.entities),
                "candidate_entities": read_entities(candidate, scenario.entities),
            }
        results.append(
            compare_scenario(
                scenario,
                read_box(prepared, scenario.box),
                read_box(official, scenario.box),
                read_box(candidate, scenario.box),
                **entities,
            )
        )
    return build_result(
        results,
        step_ticks=ticks,
        world_template=world_template(paths),
        game_times={"prepared": base_time, "official": official_time, "bbb": bbb_time},
        stepped={"official": official_steps, "bbb": bbb_steps},
    )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--server-bin", type=Path, required=True)
    parser.add_argument("--admin-bin", type=Path, required=True)
    parser.add_argument("--vanilla-root", type=Path, required=True)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--server-jar", type=Path, required=True)
    parser.add_argument(
        "--world-template",
        type=Path,
        required=True,
        help="an official Mojang 26.1 save the scenario is built inside a copy of",
    )
    parser.add_argument(
        "--step-ticks",
        type=int,
        default=DEFAULT_STEP_TICKS,
        help=f"how many ticks each side is stepped; default {DEFAULT_STEP_TICKS}",
    )
    parser.add_argument(
        "--scenario",
        action="append",
        choices=sorted(scenario.name for scenario in SCENARIOS),
        help="scenario to compare; repeatable, default is every scenario",
    )
    parser.add_argument("--output", type=Path, help="where to write the result JSON")
    return parser.parse_args(argv)


def scenario_line(scenario: Mapping[str, Any]) -> str:
    """One human line per scenario, saying which half of it disagreed."""
    verdict = "pass" if scenario["pass"] else "FAIL"
    detail = f" ({scenario['matched']}/{scenario['compared']} blocks agree"
    entity = scenario["entity"]
    if entity is not None:
        detail += (
            f"; {entity['official_count']} official / {entity['bbb_count']} bbb entities, "
            f"{entity['difference_count']} differing"
        )
    detail += ")"
    if scenario["bbb_made_no_change"]:
        detail += "; bbb left every differing block at its input value"
    for reason in scenario["not_measured"]:
        detail += f"; NOT A MEASUREMENT: {reason}"
    return f"server world oracle: {scenario['name']} {verdict}{detail}"


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    selected = [
        scenario
        for scenario in SCENARIOS
        if args.scenario is None or scenario.name in args.scenario
    ]
    paths = Paths(
        artifact_dir=args.artifact_dir,
        server_bin=args.server_bin,
        # `Paths.client_bin` is only read by the release gate's `ClientRun`,
        # which this harness never builds: not needing a client is the entire
        # reason this oracle exists. It is filled with the server binary so the
        # shared record stays satisfied rather than gaining an optional field.
        client_bin=args.server_bin,
        admin_bin=args.admin_bin,
        vanilla_root=args.vanilla_root,
        java_bin=args.java_bin,
        server_jar=args.server_jar,
        world_template=args.world_template,
    )
    paths.artifact_dir.mkdir(parents=True, exist_ok=True)
    try:
        result = run(paths, selected, args.step_ticks)
    except (OracleError, ReleaseGateError) as error:
        print(f"server world oracle: {error}", file=sys.stderr)
        return 2
    destination = args.output or (paths.artifact_dir / "server-world-oracle.json")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", "utf-8")
    print(f"server world oracle: result at {destination}")
    print(
        f"server world oracle: stepped {result['step']['ticks']} ticks; game time "
        f"{result['game_time']['prepared']} -> official {result['game_time']['official']}, "
        f"bbb {result['game_time']['bbb']}"
    )
    for scenario in result["scenarios"]:
        print(scenario_line(scenario))
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
