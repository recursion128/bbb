#!/usr/bin/env python3
"""Make bbb-server write an `entities/` chunk, and hand it to Mojang's server.

The round-trip gate already proves that the official 26.1 server reopens the
*blocks* bbb-server saved. It proves nothing about entities, because nothing in
that run ever makes bbb own one: the capture is a freshly generated flat world
with no entity in it, so the `entities/` directory bbb writes back is the empty
one it read. `server-entity-runtime` asks for the other half -- "an
official-server reload of a save whose entity storage the runtime wrote" -- and
this module is the step that produces it.

The one entity bbb has a runtime for is `minecraft:falling_block`, and the one
way it comes into existence is the scheduled block tick of a `FallingBlock`
whose support was taken away: `FallingBlock.onPlace` schedules a tick
`getDelayAfterPlace()` -- two -- ticks out (FallingBlock.java:29-31, :59-61),
and that tick turns the block into a `FallingBlockEntity` when the block below
it is free (FallingBlock.java:49-54, :63-65). `FallingBlockEntity.fall`
(FallingBlockEntity.java:91-102) is what replaces the block with air and adds
the entity to the level, so the sand block on disk becomes an entity record on
disk with no packet, no player and no client asking for it.

That shape decides which process does what here, because bbb's command surface
cannot place a block: `IMPLEMENTED_COMMANDS`
(crates/bbb-server-core/src/commands/catalog.rs) has no `/setblock`, no
`/fill` and no `/forceload`. So the official server is the scenario author --
it hangs one sand block in the air over a forceloaded column, on a frozen tick
so that nothing happens yet -- and bbb-server is the process that runs the
block tick, spawns the entity, moves it and writes it into `entities/`. The
entity in the save the gate hands back is therefore bbb's own work, which is
the whole point of the step.

The entity has to still be in the air at every hand-over, because a landed one
is not a saved one: `FallingBlockEntity.tick` sets the block back and discards
the entity the moment it is supported (FallingBlockEntity.java:194-198), and
discards it outright after 600 ticks of flight (:172-177). So both sides are
driven with `/tick freeze` and `/tick step <n>t` rather than left to run, and
the sand is hung high enough that the remaining flight outlasts every unfrozen
moment the rest of the gate spends. `STEP_TICKS` states that arithmetic.

Nothing here re-implements the two-server driving that
`scripts/server_world_oracle.py` already owns: the official console, the frozen
step, the freeze/step management calls and the bbb-server run are imported from
it. What this module adds is the one scenario the round trip needs and the
assertions that name every way it can come back empty -- including the one an
"is the entity still there" reading cannot see on its own, which is a save the
official server never opened at all; `require_republished` is that check.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.anvil import (  # noqa: E402
    AnvilError,
    WorldSaveReader,
    chunk_entities,
    chunk_timestamp,
)
from server_release_gate import Paths, ReleaseGateError, free_port  # noqa: E402
from server_world_oracle import (  # noqa: E402
    BOOT_TIMEOUT,
    DIMENSION,
    ENTITY_FIELDS,
    Box,
    ConsoleServer,
    EntityProbe,
    OracleError,
    Scenario,
    forceload_commands,
    game_time,
    run_bbb,
)

#: Where the sand hangs, and what it is.
#:
#: The x/z put it in its own column, well away from the spawn the gate's probe
#: client plays in, and y=300 is inside the overworld's build range -- the
#: capture is the flat world whose floor is at y=-61, and the same height is
#: what `server_world_oracle.ENTITY_FALL_AND_LAND` drops its high sand from.
#: The height is what buys the flight time `STEP_TICKS` spends.
SAND_X = 34
SAND_Y = 300
SAND_Z = 8
SAND_BLOCK = "minecraft:sand"

#: What the record has to call itself. `Entity.saveAsPassenger` writes the
#: registry id under `id` (Entity.java:1965-1966).
FALLING_BLOCK_ID = "minecraft:falling_block"

#: The tags a saved falling block has to carry, which is
#: `server_world_oracle.ENTITY_FIELDS` -- `Pos`, `Motion` and `OnGround` from
#: `Entity.saveWithoutId` (Entity.java:1979-1987), `id` from `saveAsPassenger`
#: (Entity.java:1965-1966), and `BlockState` and `Time` from
#: `FallingBlockEntity.addAdditionalSaveData` (FallingBlockEntity.java:281-283).
#: A record missing any of them is not a record Mojang's own reader would make.
REQUIRED_FIELDS = ENTITY_FIELDS

#: How many ticks bbb is stepped after it freezes.
#:
#: Bounded from both ends. From below: bbb ticks freely between the moment it
#: is ready and the moment the freeze command reaches it, and the step is
#: measured from the prepared save's clock, so the count has to comfortably
#: outrun that drift -- it is tens of milliseconds, which is single-digit ticks.
#: Two ticks of it are spent before the entity exists at all
#: (`getDelayAfterPlace`, FallingBlock.java:59-61).
#:
#: From above: what is left of the flight is the budget every later unfrozen
#: moment of the gate spends, and the entity has to still be airborne at the
#: end of all of them. A falling block accelerates by 0.04 a tick
#: (FallingBlockEntity.java:142-144) and keeps 0.98 of its speed
#: (FallingBlockEntity.java:240), so it settles at 0.04 * 0.98 / 0.02 = 1.96
#: blocks a tick and the sum of the first N steps is 1.96*N - 98*(1 - 0.98^N).
#: Falling the 360 blocks from y=300 to the flat world's surface therefore
#: takes 229 ticks. Eighty of them drops it 81.5 blocks and leaves 149 -- seven
#: and a half seconds -- for the official server's own boot-to-freeze window,
#: which is one poll of the gate's marker loop.
#: `scripts/tests/test_server_round_trip_entity.py` runs that arithmetic.
STEP_TICKS = 80


class RoundTripEntityError(RuntimeError):
    """The entity half of the round trip did not hold."""


#: The scenario, in the oracle's own vocabulary so that the forceload it needs
#: is computed by the same code that computes the oracle's.
#:
#: A `Scenario` is what `forceload_commands` reads: without a `TicketType.FORCED`
#: ticket on the column, nothing schedules, nothing ticks and nothing falls,
#: with nobody connected. The box is the sand block itself, and the entity probe
#: names the column the record lands in -- the entity falls straight down, so it
#: is the sand's own column throughout.
FALLING_SAND = Scenario(
    name="round_trip_falling_block",
    owner="FallingBlock.tick -> FallingBlockEntity -> EntityStorage",
    row="server-entity-runtime",
    setup=(f"setblock {SAND_X} {SAND_Y} {SAND_Z} {SAND_BLOCK}",),
    box=Box(SAND_X, SAND_Y, SAND_Z, SAND_X, SAND_Y, SAND_Z),
    entities=EntityProbe(chunk_x=SAND_X >> 4, chunk_z=SAND_Z >> 4),
)


def setup_lines() -> list[str]:
    """The console lines the official server places the scenario with.

    The freeze is not here: it is sent, and waited for, before any of this, so
    that no tick runs between the forceload and the `setblock`.
    """
    return [*forceload_commands([FALLING_SAND]), *FALLING_SAND.setup]


def probe() -> EntityProbe:
    """The entity column every side of this step reads."""
    assert FALLING_SAND.entities is not None
    return FALLING_SAND.entities


def numbers(value: Any, count: int) -> list[float] | None:
    """`value` as a list of exactly `count` numbers, or None when it is not one."""
    if not isinstance(value, list) or len(value) != count:
        return None
    if any(isinstance(item, bool) or not isinstance(item, (int, float)) for item in value):
        return None
    return [float(item) for item in value]


def require_falling_block(
    records: Sequence[Mapping[str, Any]], *, actor: str, chunk: tuple[int, int]
) -> Mapping[str, Any]:
    """The one falling-block record in `records`, or a named failure.

    Every way this can come back wrong is its own message, because the failure
    that matters is never "they differ" -- it is that the step measured nothing
    and would otherwise pass on an empty comparison.
    """
    where = f"{actor}'s entity chunk ({chunk[0]}, {chunk[1]})"
    if not records:
        raise RoundTripEntityError(f"{where} is present but holds no entity records at all")
    falling = [record for record in records if record.get("id") == FALLING_BLOCK_ID]
    if not falling:
        held = ", ".join(sorted({str(record.get("id")) for record in records}))
        raise RoundTripEntityError(
            f"{where} holds {len(records)} record(s) but none of them is "
            f"{FALLING_BLOCK_ID}: {held}"
        )
    if len(falling) > 1:
        raise RoundTripEntityError(
            f"{where} holds {len(falling)} {FALLING_BLOCK_ID} records; the scenario places "
            "exactly one sand block"
        )
    record = falling[0]
    missing = [field for field in REQUIRED_FIELDS if field not in record]
    if missing:
        raise RoundTripEntityError(
            f"the {FALLING_BLOCK_ID} in {where} is missing {', '.join(missing)}, so it is not "
            f"the record a saved entity carries: {json.dumps(record, ensure_ascii=False)[:400]}"
        )
    position = numbers(record.get("Pos"), 3)
    if position is None:
        raise RoundTripEntityError(
            f"the {FALLING_BLOCK_ID} in {where} has a Pos that is not three doubles: "
            f"{record.get('Pos')!r}"
        )
    motion = numbers(record.get("Motion"), 3)
    if motion is None:
        raise RoundTripEntityError(
            f"the {FALLING_BLOCK_ID} in {where} has a Motion that is not three doubles: "
            f"{record.get('Motion')!r}"
        )
    if motion[1] >= 0.0:
        raise RoundTripEntityError(
            f"the {FALLING_BLOCK_ID} in {where} is not falling: Motion is {motion}, and the "
            "0.04 gravity (FallingBlockEntity.java:142-144) that `Entity.applyGravity` "
            "subtracts (Entity.java:1440-1445) makes every airborne tick of one negative"
        )
    state = record.get("BlockState")
    if not isinstance(state, dict) or state.get("Name") != SAND_BLOCK:
        raise RoundTripEntityError(
            f"the {FALLING_BLOCK_ID} in {where} carries {state!r} rather than the "
            f"{SAND_BLOCK} BlockState the scenario dropped"
        )
    time = record.get("Time")
    if isinstance(time, bool) or not isinstance(time, int) or time < 1:
        raise RoundTripEntityError(
            f"the {FALLING_BLOCK_ID} in {where} has Time {time!r}; a record written by a server "
            "that ticked it counts up from one (FallingBlockEntity.java:152)"
        )
    return record


def require_entity_storage(world: Path, *, actor: str) -> Mapping[str, Any]:
    """Read the scenario's entity column out of a save, and assert it holds the fall.

    The three degenerate shapes are separated here rather than folded together,
    because `WorldSaveReader.entities` answers `[]` for all of them: a save with
    no `entities/` directory at all, a directory with no chunk in it and a chunk
    written with an empty list are three different failures of three different
    processes.
    """
    column = probe()
    reader = WorldSaveReader(world, DIMENSION)
    if not reader.entities_dir.is_dir():
        raise RoundTripEntityError(
            f"{actor} wrote no entity storage at all: {reader.entities_dir} does not exist"
        )
    try:
        chunk = reader.entity_chunk(column.chunk_x, column.chunk_z)
    except AnvilError as error:
        raise RoundTripEntityError(
            f"cannot read {actor}'s entity storage under {reader.entities_dir}: {error}"
        ) from error
    if chunk is None:
        raise RoundTripEntityError(
            f"{actor} wrote no entity chunk at ({column.chunk_x}, {column.chunk_z}) under "
            f"{reader.entities_dir}, so nothing it saved holds the falling block"
        )
    try:
        records = chunk_entities(chunk)
    except AnvilError as error:
        raise RoundTripEntityError(
            f"{actor}'s entity chunk ({column.chunk_x}, {column.chunk_z}) does not decode: {error}"
        ) from error
    return require_falling_block(
        records, actor=actor, chunk=(column.chunk_x, column.chunk_z)
    )


def require_republished(published: Path, reloaded: Path) -> dict[str, int]:
    """The official server wrote the entity column itself, over bbb's copy.

    This is what separates "Mojang's server read what bbb wrote" from "the file
    was still on disk afterwards". A column it never loaded is a column it never
    writes, so it would come back exactly as bbb published it -- and a reader
    that only asked whether the entity was still there could not tell the two
    apart, which would let this whole step pass on a save nothing ever opened.

    What answers it is the region header rather than the chunk: `RegionFile`
    stamps a slot with the epoch second on every write of it, and only a write
    moves that stamp -- `bbb_tools.anvil.chunk_timestamp` carries the layout and
    the anchors it is read with. So a newer stamp on the reloaded
    save means this server rewrote the column, and the falling block found
    inside it is one Mojang's own reader accepted and Mojang's own writer
    published -- a record it had failed to read would have been written back
    without it.

    The two writes cannot collide inside one epoch second: between them the gate
    copies the whole save and boots a JVM.

    The tick the record ran is deliberately *not* asserted on. The reload is
    frozen as soon as the gate sees `Done (`, and whether the entity storage's
    own asynchronous load has landed by that tick is a race this step has been
    observed to lose; the advance is reported instead.
    """
    column = probe()
    stamps: dict[str, int] = {}
    for name, world in (("published", published), ("reloaded", reloaded)):
        directory = WorldSaveReader(world, DIMENSION).entities_dir
        try:
            stamp = chunk_timestamp(directory, column.chunk_x, column.chunk_z)
        except AnvilError as error:
            raise RoundTripEntityError(f"cannot read {directory}: {error}") from error
        if stamp is None:
            raise RoundTripEntityError(
                f"the {name} save holds no write stamp for entity chunk "
                f"({column.chunk_x}, {column.chunk_z}) under {directory}"
            )
        stamps[name] = stamp
    if stamps["reloaded"] <= stamps["published"]:
        raise RoundTripEntityError(
            f"the reloaded save's entity chunk still carries the stamp bbb-server wrote "
            f"({stamps['published']}): `RegionFile.write` stamps every column it writes, so an "
            "unmoved stamp means the official server never republished this one and the record "
            "read back here is bbb's own file rather than something Mojang's code accepted"
        )
    return {"published_at": stamps["published"], "republished_at": stamps["reloaded"]}


def require_not_behind(reloaded: Mapping[str, Any], written: Mapping[str, Any]) -> dict[str, Any]:
    """The reloaded record is the written one, carried forward rather than back.

    The official server ticks the entity on for as long as it runs before the
    freeze reaches it, so `Time` may be anything from what bbb wrote upwards --
    but never below it, and the block never rises.
    """
    was = written.get("Time")
    now = reloaded.get("Time")
    if not isinstance(was, int) or not isinstance(now, int):
        raise RoundTripEntityError(
            f"cannot compare the reloaded falling block against the written one: Time is "
            f"{was!r} before and {now!r} after"
        )
    if now < was:
        raise RoundTripEntityError(
            f"the reloaded falling block reads Time {now}, behind the {was} bbb-server wrote: "
            "the record came back as an older entity than the one that was handed over"
        )
    before = numbers(written.get("Pos"), 3)
    after = numbers(reloaded.get("Pos"), 3)
    if before is None or after is None:
        raise RoundTripEntityError(
            f"cannot compare the reloaded falling block's Pos against the written one: "
            f"{written.get('Pos')!r} before and {reloaded.get('Pos')!r} after"
        )
    if after[1] > before[1]:
        raise RoundTripEntityError(
            f"the reloaded falling block rose: y is {after[1]} after and was {before[1]} before, "
            "and nothing in a falling block's tick moves it upwards"
        )
    return {"ticks": now - was, "fell": before[1] - after[1]}


def require_staged_scenario(world: Path) -> None:
    """The staged save holds the sand as a *block*, and holds no entity yet.

    Both halves are what makes the entity in the published save attributable.
    The first says the scenario was really built -- a `/setblock` the official
    server refused would otherwise leave an empty world that ticks into an empty
    entity storage. The second says the official server did not spawn the thing
    under test: it places the block with its tick loop frozen, so the scheduled
    tick `FallingBlock.onPlace` filed (FallingBlock.java:29-31) is still pending
    and bbb-server is the process that runs it.
    """
    column = probe()
    reader = WorldSaveReader(world, DIMENSION)
    try:
        state = reader.block_state(SAND_X, SAND_Y, SAND_Z)
        records = reader.entities(column.chunk_x, column.chunk_z)
    except AnvilError as error:
        raise RoundTripEntityError(f"cannot read the staged save at {world}: {error}") from error
    if state is None or state.name != SAND_BLOCK:
        raise RoundTripEntityError(
            f"the staged save holds {describe_state(state)} at ({SAND_X}, {SAND_Y}, {SAND_Z}) "
            f"rather than the {SAND_BLOCK} the official server was told to hang there"
        )
    falling = [record for record in records if record.get("id") == FALLING_BLOCK_ID]
    if falling:
        raise RoundTripEntityError(
            f"the staged save already holds {len(falling)} {FALLING_BLOCK_ID}(s) in "
            f"({column.chunk_x}, {column.chunk_z}); the official server places the sand on a "
            "frozen tick so that bbb-server is the process that spawns the entity"
        )


def describe_state(state: Any) -> str:
    """How a read block is named in a failure, including an unwritten one."""
    return "<unwritten>" if state is None else state.describe()


def stage(paths: Paths, world: Path) -> Path:
    """Hang the sand block, by the official server, with its tick loop frozen.

    The freeze lands before the block is placed, so the save carries the
    scheduled tick `FallingBlock.onPlace` filed (FallingBlock.java:29-31) and
    none of its effect. The next server to open this save is the one that runs
    it.
    """
    server = ConsoleServer(paths, world, free_port(), "entity-setup")
    try:
        server.start()
        server.wait_ready(BOOT_TIMEOUT)
        server.freeze()
        server.send(setup_lines())
        # A console command is only known to have run once the server answers
        # something after it, and the save is that something.
        server.save_and_stop()
        server.require_commands_accepted()
    finally:
        server.terminate()
    return server.world


def write(paths: Paths, world: Path, output: Path) -> dict[str, Any]:
    """Stage the scenario, let bbb tick it into an entity, and publish the save."""
    staged = stage(paths, world)
    require_staged_scenario(staged)
    base_time = game_time(staged)
    if base_time is None:
        raise RoundTripEntityError(
            f"the staged save at {staged} carries no Data.Time, so nothing says where bbb's "
            "step should end"
        )
    # `run_bbb` is the oracle's own unattended bbb-server run: it copies the
    # save, opens it with nobody connected, freezes, steps to the target clock,
    # flushes and stops through the management endpoint. Reusing it is what
    # keeps one implementation of "drive bbb-server through a measured step".
    candidate, steps = run_bbb(paths, staged, base_time + STEP_TICKS)
    record = require_entity_storage(candidate, actor="bbb-server")
    if output.exists():
        shutil.rmtree(output)
    shutil.copytree(candidate, output)
    return {
        "staged_world": str(staged),
        "bbb_client_world": str(candidate),
        "published_world": str(output),
        "base_time": base_time,
        "step_ticks": STEP_TICKS,
        "stepped": steps,
        "entity": record,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)

    writer = commands.add_parser(
        "write", help="stage the sand block and have bbb-server write the entity"
    )
    writer.add_argument("--artifact-dir", type=Path, required=True)
    writer.add_argument("--server-bin", type=Path, required=True)
    writer.add_argument("--client-bin", type=Path, required=True)
    writer.add_argument("--admin-bin", type=Path, required=True)
    writer.add_argument("--vanilla-root", type=Path, required=True)
    writer.add_argument("--java-bin", type=Path, required=True)
    writer.add_argument("--server-jar", type=Path, required=True)
    writer.add_argument("--input-world", type=Path, required=True)
    writer.add_argument("--output-world", type=Path, required=True)

    verifier = commands.add_parser(
        "verify", help="assert a save still holds the falling block bbb wrote"
    )
    verifier.add_argument("--world", type=Path, required=True)
    verifier.add_argument(
        "--actor",
        default="the official 26.1 server",
        help="who wrote the save, for the failure messages",
    )
    verifier.add_argument(
        "--published",
        type=Path,
        help="the `write` report this reload came from; with it the reload is held against the "
        "save bbb published rather than only read on its own",
    )
    return parser.parse_args(argv)


def write_report(report_path: Path) -> Mapping[str, Any]:
    """A `write` report read back: the record bbb published and where it published it."""
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RoundTripEntityError(
            f"cannot read the write report at {report_path}: {error}"
        ) from error
    if not isinstance(report, dict) or not isinstance(report.get("entity"), dict):
        raise RoundTripEntityError(
            f"the write report at {report_path} carries no entity to compare the reload against"
        )
    if not isinstance(report.get("published_world"), str):
        raise RoundTripEntityError(
            f"the write report at {report_path} does not say which save bbb published"
        )
    return report


def main(argv: list[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        if arguments.command == "write":
            paths = Paths(
                artifact_dir=arguments.artifact_dir,
                server_bin=arguments.server_bin,
                client_bin=arguments.client_bin,
                admin_bin=arguments.admin_bin,
                vanilla_root=arguments.vanilla_root,
                java_bin=arguments.java_bin,
                server_jar=arguments.server_jar,
            )
            paths.artifact_dir.mkdir(parents=True, exist_ok=True)
            report = write(paths, arguments.input_world, arguments.output_world)
        else:
            reloaded = require_entity_storage(arguments.world, actor=arguments.actor)
            report = {"world": str(arguments.world), "entity": reloaded}
            if arguments.published is not None:
                published = write_report(arguments.published)
                report["republished"] = require_republished(
                    Path(str(published["published_world"])), arguments.world
                )
                report["advance"] = require_not_behind(reloaded, published["entity"])
    except (RoundTripEntityError, OracleError, ReleaseGateError) as error:
        print(f"round-trip entity: {error}", file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
