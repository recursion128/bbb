"""Unit coverage for the server world oracle's pure halves.

Two things in this harness can be wrong without any server running: the Anvil
decoding that turns a saved region into block names, and the scenario/compare
arithmetic that turns two readings into a verdict. Both are exercised here
against values derived from the official 26.1 owners the modules cite, so a
drift in either shows up before a two-server run is spent on it.
"""

from __future__ import annotations

import gzip
import inspect
import itertools
import shutil
import struct
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import pytest  # noqa: E402
from bbb_tools.anvil import (  # noqa: E402
    SECTOR_BYTES,
    AnvilError,
    BlockState,
    WorldSaveReader,
    cell_index,
    chunk_block_state,
    chunk_entities,
    decode_nbt,
    decompress_chunk,
    palette_entry,
    read_chunk,
    read_compressed_nbt,
    region_name,
    section_block_state,
    slot_index,
    storage_bits,
    unpack_cell,
)
from server_release_gate import ServerProcess  # noqa: E402
from server_world_oracle import (  # noqa: E402
    BBB_LOOPBACK_SCOPE,
    CHORUS_RANDOM_GROWTH,
    CHORUS_SURVIVAL_TICK,
    COMMAND_FAILURE_MARKERS,
    DEFAULT_STEP_TICKS,
    ENTITY_FALL_AND_LAND,
    ENTITY_FIELDS,
    FREEZE_COMMAND,
    GROUND_BLOCK,
    GROUND_Y,
    LEAF_DISTANCE_CASCADE,
    PREAMBLE,
    RANDOM_TICK_SPEED,
    SCENARIOS,
    STEP_FAILURE_TEXT,
    Box,
    EntityProbe,
    OracleError,
    Scenario,
    bbb_command,
    build_result,
    compare_entities,
    compare_scenario,
    describe,
    forceload_commands,
    forceloaded_columns,
    game_time,
    ground_commands,
    numeric_delta,
    require_stepped,
    run_bbb,
    setup_script,
    step_command,
    step_progress,
    step_success_text,
    step_to_game_time,
)

pytestmark = pytest.mark.unit


def _string(value: str) -> bytes:
    raw = value.encode("utf-8")
    return struct.pack(">H", len(raw)) + raw


def _compound(entries: list[tuple[int, str, bytes]]) -> bytes:
    body = b""
    for tag, name, payload in entries:
        body += struct.pack(">b", tag) + _string(name) + payload
    return body + b"\x00"


def _named_root(entries: list[tuple[int, str, bytes]]) -> bytes:
    return b"\x0a" + _string("") + _compound(entries)


def _long_array(values: list[int]) -> bytes:
    return struct.pack(">i", len(values)) + b"".join(
        struct.pack(">q", value - (1 << 64) if value >= 1 << 63 else value) for value in values
    )


def _palette_list(names: list[str]) -> bytes:
    body = struct.pack(">b", 10) + struct.pack(">i", len(names))
    for name in names:
        body += _compound([(8, "Name", _string(name))])
    return body


def _section(y: int, names: list[str], data: list[int] | None) -> bytes:
    block_states: list[tuple[int, str, bytes]] = [(9, "palette", _palette_list(names))]
    if data is not None:
        block_states.append((12, "data", _long_array(data)))
    return _compound(
        [
            (1, "Y", struct.pack(">b", y)),
            (10, "block_states", _compound(block_states)),
        ]
    )


def _entity_chunk_tag(entities: list[bytes]) -> bytes:
    body = struct.pack(">b", 10) + struct.pack(">i", len(entities))
    for entity in entities:
        body += entity
    return _named_root(
        [
            (11, "Position", struct.pack(">i", 2) + struct.pack(">ii", 1, 0)),
            (9, "Entities", body),
        ]
    )


def _falling_block(y: float) -> bytes:
    return _compound(
        [
            (8, "id", _string("minecraft:falling_block")),
            (
                9,
                "Pos",
                struct.pack(">b", 6)
                + struct.pack(">i", 3)
                + struct.pack(">ddd", 20.5, y, 10.5),
            ),
            (4, "Time", struct.pack(">q", 0)),
        ]
    )


def _chunk_tag(sections: list[bytes]) -> bytes:
    body = struct.pack(">b", 10) + struct.pack(">i", len(sections))
    for section in sections:
        body += section
    return _named_root([(9, "sections", body)])


class NbtDecodingTest(unittest.TestCase):
    def test_named_root_keeps_every_key_with_its_own_payload(self) -> None:
        # A compound's name has to be read before its payload. Reading them the
        # other way round still decodes, but shifts every key by one field, so
        # this fixture puts three differently sized payloads in a row.
        document = _named_root(
            [
                (8, "Status", _string("minecraft:full")),
                (3, "xPos", struct.pack(">i", -7)),
                (3, "zPos", struct.pack(">i", 4)),
                (1, "Y", struct.pack(">b", -3)),
            ]
        )
        self.assertEqual(
            decode_nbt(document),
            {"Status": "minecraft:full", "xPos": -7, "zPos": 4, "Y": -3},
        )

    def test_empty_list_consumes_only_its_header(self) -> None:
        empty = struct.pack(">b", 0) + struct.pack(">i", 0)
        document = _named_root([(9, "block_ticks", empty), (3, "after", struct.pack(">i", 9))])
        self.assertEqual(decode_nbt(document), {"block_ticks": [], "after": 9})

    def test_a_non_compound_root_is_refused(self) -> None:
        with self.assertRaises(AnvilError):
            decode_nbt(b"\x08" + _string("") + _string("nope"))

    def test_read_compressed_nbt_reads_a_level_dat_shaped_file(self) -> None:
        document = _named_root([(10, "Data", _compound([(4, "Time", struct.pack(">q", 424))]))])
        path = Path(self.enterContext(__import__("tempfile").TemporaryDirectory())) / "level.dat"
        path.write_bytes(gzip.compress(document))
        self.assertEqual(read_compressed_nbt(path), {"Data": {"Time": 424}})


class RegionAddressingTest(unittest.TestCase):
    def test_region_name_holds_thirty_two_chunks_per_axis(self) -> None:
        self.assertEqual(region_name(0, 0), "r.0.0.mca")
        self.assertEqual(region_name(31, 31), "r.0.0.mca")
        self.assertEqual(region_name(32, -1), "r.1.-1.mca")

    def test_slot_index_is_the_chunk_position_inside_its_region(self) -> None:
        self.assertEqual(slot_index(0, 0), 0)
        self.assertEqual(slot_index(1, 0), 1)
        self.assertEqual(slot_index(0, 1), 32)
        self.assertEqual(slot_index(-1, -1), 31 + 31 * 32)

    def test_every_registered_compression_id_is_named(self) -> None:
        self.assertEqual(decompress_chunk(1, gzip.compress(b"payload")), b"payload")
        self.assertEqual(decompress_chunk(2, zlib.compress(b"payload")), b"payload")
        self.assertEqual(decompress_chunk(3, b"payload"), b"payload")
        for unsupported in (4, 127, 9):
            with self.assertRaises(AnvilError):
                decompress_chunk(unsupported, b"payload")


class PaletteTest(unittest.TestCase):
    def test_storage_bits_follows_the_block_state_strategy_table(self) -> None:
        # `Strategy.createForBlockStates`: one entry stores nothing, two through
        # sixteen all store four, and above that it is the ceiling log itself.
        self.assertEqual(storage_bits(1), 0)
        for size in (2, 3, 5, 9, 16):
            self.assertEqual(storage_bits(size), 4)
        self.assertEqual(storage_bits(17), 5)
        self.assertEqual(storage_bits(32), 5)
        self.assertEqual(storage_bits(33), 6)
        self.assertEqual(storage_bits(257), 9)
        with self.assertRaises(AnvilError):
            storage_bits(0)

    def test_cell_index_is_y_then_z_then_x(self) -> None:
        # `Strategy.getIndex` with bitsPerAxis 4.
        self.assertEqual(cell_index(0, 0, 0), 0)
        self.assertEqual(cell_index(1, 0, 0), 1)
        self.assertEqual(cell_index(0, 0, 1), 16)
        self.assertEqual(cell_index(0, 1, 0), 256)
        self.assertEqual(cell_index(15, 15, 15), 4095)

    def test_unpack_cell_never_straddles_a_long(self) -> None:
        # Sixteen four-bit values per long, low entry first, and the sixteenth
        # entry of a long is the one a sign-extended shift would corrupt.
        packed = sum(value << (4 * value) for value in range(16))
        self.assertEqual([unpack_cell([packed], 4, i) for i in range(16)], list(range(16)))
        top = 0xF000000000000000
        self.assertEqual(unpack_cell([top], 4, 15), 15)
        # Five bits gives twelve values per long and four wasted bits, which is
        # exactly the no-straddle rule.
        self.assertEqual(unpack_cell([0b11111 << 55], 5, 11), 31)
        self.assertEqual(unpack_cell([0], 0, 4095), 0)
        with self.assertRaises(AnvilError):
            unpack_cell([0], 4, 16)

    def test_palette_entry_sorts_its_properties(self) -> None:
        state = palette_entry({"Name": "minecraft:water", "Properties": {"level": "3"}})
        self.assertEqual(state, BlockState("minecraft:water", (("level", "3"),)))
        self.assertEqual(state.describe(), "minecraft:water[level=3]")
        self.assertEqual(palette_entry({"Name": "minecraft:air"}).describe(), "minecraft:air")
        with self.assertRaises(AnvilError):
            palette_entry({"Properties": {}})

    def test_a_single_entry_palette_needs_no_data_array(self) -> None:
        section = decode_nbt(_named_root([(10, "s", _section(-4, ["minecraft:air"], None))]))["s"]
        self.assertEqual(section_block_state(section, 7, 7, 7), BlockState("minecraft:air"))

    def test_a_multi_entry_palette_reads_through_its_data_array(self) -> None:
        # Two entries, so four bits each; cell 0 is air and cell 1 is stone.
        section = decode_nbt(
            _named_root([(10, "s", _section(-4, ["minecraft:air", "minecraft:stone"], [1 << 4]))])
        )["s"]
        self.assertEqual(section_block_state(section, 0, 0, 0), BlockState("minecraft:air"))
        self.assertEqual(section_block_state(section, 1, 0, 0), BlockState("minecraft:stone"))


class WorldReadingTest(unittest.TestCase):
    def _write_region(self, region_dir: Path, chunk_x: int, chunk_z: int, tag: bytes) -> None:
        region_dir.mkdir(parents=True, exist_ok=True)
        payload = zlib.compress(tag)
        body = struct.pack(">i", len(payload) + 1) + b"\x02" + payload
        sectors = -(-len(body) // SECTOR_BYTES)
        table = bytearray(SECTOR_BYTES * 2)
        index = slot_index(chunk_x, chunk_z) * 4
        table[index : index + 3] = (2).to_bytes(3, "big")
        table[index + 3] = sectors
        path = region_dir / region_name(chunk_x, chunk_z)
        path.write_bytes(bytes(table) + body + b"\x00" * (sectors * SECTOR_BYTES - len(body)))

    def test_reader_resolves_a_world_coordinate_through_its_section(self) -> None:
        root = Path(self.enterContext(__import__("tempfile").TemporaryDirectory()))
        world = root / "world"
        region_dir = world / "dimensions" / "minecraft" / "overworld" / "region"
        # Section Y -4 covers y -64..-49, so y -60 is section-local y 4.
        names = ["minecraft:air", "minecraft:stone"]
        data = [0] * 256
        index = cell_index(3, 4, 5)
        data[index // 16] = 1 << (4 * (index % 16))
        self._write_region(region_dir, 0, 0, _chunk_tag([_section(-4, names, data)]))
        reader = WorldSaveReader(world)
        self.assertEqual(reader.block_state(3, -60, 5), BlockState("minecraft:stone"))
        self.assertEqual(reader.block_state(4, -60, 5), BlockState("minecraft:air"))
        # A chunk with no region file at all reads as unwritten rather than
        # failing, which is what a comparison wants to report.
        self.assertIsNone(reader.block_state(600, -60, 600))

    def test_the_reader_reads_the_entity_region_beside_the_block_region(self) -> None:
        # `entities/` is the same Anvil container as `region/`
        # (ServerLevel.java:247 next to ChunkMap.java:172), so one reader serves
        # both; only the tags inside a chunk differ.
        root = Path(self.enterContext(__import__("tempfile").TemporaryDirectory()))
        world = root / "world"
        dimension = world / "dimensions" / "minecraft" / "overworld"
        self._write_region(
            dimension / "entities",
            1,
            0,
            _entity_chunk_tag([_falling_block(-50.0), _falling_block(-49.0)]),
        )
        # `RegionFile` creates the file when the directory opens and only writes
        # the header once a chunk lands, so an empty region file is the normal
        # shape for a region that holds nothing (RegionFile.java:72-77 reads it
        # with a zero-filled header). Region 1,0 covers chunks 32..63.
        (dimension / "entities" / region_name(32, 0)).write_bytes(b"")
        reader = WorldSaveReader(world)

        records = reader.entities(1, 0)
        self.assertEqual([record["id"] for record in records], ["minecraft:falling_block"] * 2)
        self.assertEqual([record["Pos"][1] for record in records], [-50.0, -49.0])
        # An empty region file, a region file with no slot for the column, and a
        # directory with no region file at all all answer "no entities here",
        # which is what `EntityStorage.loadEntities` answers for each
        # (EntityStorage.java:46-57).
        self.assertEqual(reader.entities(32, 0), [])
        self.assertEqual(reader.entities(2, 0), [])
        self.assertEqual(reader.entities(600, 600), [])
        # The block half of the same save is untouched by any of it.
        self.assertIsNone(reader.block_state(20, -50, 10))

    def test_an_entity_chunk_without_the_list_is_empty_and_a_broken_one_is_refused(
        self,
    ) -> None:
        # `childrenListOrEmpty("Entities")` (EntityStorage.java:74) treats a
        # missing list as empty rather than as a failure.
        self.assertEqual(chunk_entities({"Position": [1, 0]}), [])
        with self.assertRaises(AnvilError):
            chunk_entities({"Entities": 7})
        with self.assertRaises(AnvilError):
            chunk_entities({"Entities": [7]})

    def test_a_truncated_region_header_is_refused_but_an_empty_file_is_not(self) -> None:
        root = Path(self.enterContext(__import__("tempfile").TemporaryDirectory()))
        region_dir = root / "region"
        region_dir.mkdir(parents=True)
        (region_dir / region_name(0, 0)).write_bytes(b"")
        self.assertIsNone(read_chunk(region_dir, 0, 0))
        # Half a sector table is corruption, not an empty region, and this is
        # evidence tooling: it fails closed rather than reporting no chunks.
        (region_dir / region_name(0, 0)).write_bytes(b"\x00" * (SECTOR_BYTES // 2))
        with self.assertRaises(AnvilError):
            read_chunk(region_dir, 0, 0)

    def test_a_missing_section_reads_as_unwritten(self) -> None:
        chunk = decode_nbt(_chunk_tag([_section(-4, ["minecraft:air"], None)]))
        self.assertIsNone(chunk_block_state(chunk, 0, 100, 0))
        with self.assertRaises(AnvilError):
            chunk_block_state({}, 0, 0, 0)


class ScenarioScriptTest(unittest.TestCase):
    def test_the_freeze_comes_before_anything_is_placed(self) -> None:
        # Nothing may make progress while the rest of the script is still being
        # typed, so the freeze has to be the very first console line.
        self.assertEqual(PREAMBLE[0], FREEZE_COMMAND)
        script = setup_script(SCENARIOS)
        self.assertEqual(script[0], FREEZE_COMMAND)
        for scenario in SCENARIOS:
            for command in scenario.setup:
                self.assertGreater(script.index(command), script.index(FREEZE_COMMAND))

    def test_the_whole_world_is_settled_before_anything_is_placed(self) -> None:
        # Everything that decides what a scenario is allowed to see -- the
        # gamerules, the tickets and the surface conversion the raised
        # `random_tick_speed` needs -- has to be in place before the first
        # scenario block exists, or a scenario is placed into a world that is
        # still changing under it.
        script = setup_script(SCENARIOS)
        first_placement = min(
            script.index(command) for scenario in SCENARIOS for command in scenario.setup
        )
        preparation = script[:first_placement]
        for command in PREAMBLE:
            self.assertIn(command, preparation)
        for command in forceload_commands(SCENARIOS) + ground_commands(SCENARIOS):
            self.assertIn(command, preparation)

    def test_the_random_tick_speed_is_the_one_the_decay_argument_assumes(self) -> None:
        # `leaf_decay_patch` states its probability argument against this
        # number, so a preamble that raised a different one would leave the
        # argument written on the scenario untrue.
        self.assertIn(f"gamerule random_tick_speed {RANDOM_TICK_SPEED}", PREAMBLE)
        self.assertGreater(RANDOM_TICK_SPEED, 0)

    def test_block_drops_are_off_so_the_entity_probe_reads_only_its_own_entities(self) -> None:
        # The decaying leaves drop through `Block.popResource`, which is gated
        # on this rule; item entities would otherwise land in the same
        # `entities/` files the entity scenario compares, at offsets drawn from
        # the level's own RandomSource.
        self.assertIn("gamerule block_drops false", PREAMBLE)

    def test_every_scenario_chunk_is_forceloaded(self) -> None:
        # Scheduled and random ticks only run in an entity-ticking chunk, and no
        # player is ever connected, so every compared column needs its own
        # ticket.
        wanted = {(x, z) for scenario in SCENARIOS for x, z in scenario.box.chunk_columns()}
        wanted.update(
            (scenario.entities.chunk_x, scenario.entities.chunk_z)
            for scenario in SCENARIOS
            if scenario.entities is not None
        )
        self.assertEqual(
            sorted(forceload_commands(SCENARIOS)),
            sorted(f"forceload add {x * 16} {z * 16}" for x, z in wanted),
        )
        wide = Scenario(
            name="wide",
            owner="test",
            row="test",
            setup=(),
            box=Box(-1, 0, 20, 16, 0, 20),
        )
        self.assertEqual(
            forceload_commands([wide]),
            ["forceload add -16 16", "forceload add 0 16", "forceload add 16 16"],
        )

    def test_an_entity_probes_column_is_forceloaded_even_when_no_box_names_it(self) -> None:
        # An entity in an unloaded column is an entity nothing moves, so the
        # probe's column is a ticket of its own rather than something the box
        # happens to cover.
        far = Scenario(
            name="far",
            owner="test",
            row="test",
            setup=(),
            box=Box(0, 0, 0, 0, 0, 0),
            entities=EntityProbe(chunk_x=5, chunk_z=-2),
        )
        self.assertEqual(forceloaded_columns([far]), [(0, 0), (5, -2)])
        self.assertIn("forceload add 80 -32", forceload_commands([far]))

    def test_the_surface_conversion_covers_whole_columns(self) -> None:
        # A single surviving grass block spreads back into the dirt beside it,
        # so the conversion is per column rather than per scenario footprint.
        one = Scenario(
            name="one",
            owner="test",
            row="test",
            setup=(),
            box=Box(3, 0, 4, 3, 0, 4),
        )
        self.assertEqual(
            ground_commands([one]),
            [f"fill 0 {GROUND_Y} 0 15 {GROUND_Y} 15 {GROUND_BLOCK}"],
        )
        self.assertEqual(len(ground_commands(SCENARIOS)), len(forceloaded_columns(SCENARIOS)))

    def test_shipped_scenarios_are_unique_and_place_something(self) -> None:
        self.assertEqual(len({scenario.name for scenario in SCENARIOS}), len(SCENARIOS))
        for scenario in SCENARIOS:
            with self.subTest(scenario=scenario.name):
                self.assertTrue(scenario.setup, "a scenario that places nothing probes nothing")
                self.assertTrue(scenario.owner)

    def test_no_two_scenarios_compare_the_same_block(self) -> None:
        # Two scenarios sharing a cell would make a reported difference
        # ambiguous: neither verdict could say which behaviour moved it.
        seen: dict[tuple[int, int, int], str] = {}
        for scenario in SCENARIOS:
            for position in scenario.box.positions():
                owner = seen.get(position)
                self.assertIsNone(
                    owner,
                    f"{scenario.name} and {owner} both compare {position}",
                )
                seen[position] = scenario.name

    def test_an_entity_probe_reads_a_column_its_own_box_stands_in(self) -> None:
        # The probe and the box have to be about the same place, or the block
        # half of the scenario says nothing about the entity half.
        for scenario in SCENARIOS:
            if scenario.entities is None:
                continue
            with self.subTest(scenario=scenario.name):
                column = (scenario.entities.chunk_x, scenario.entities.chunk_z)
                self.assertIn(column, scenario.box.chunk_columns())

    def test_the_shell_wrapper_documents_every_scenario(self) -> None:
        # The wrapper is where a reader picks a scenario name, and it is also
        # the only place the scenario list is written twice.
        wrapper = (
            Path(__file__).resolve().parents[2] / "scripts/server-world-oracle-26.1.sh"
        ).read_text(encoding="utf-8")
        for scenario in SCENARIOS:
            with self.subTest(scenario=scenario.name):
                self.assertIn(scenario.name, wrapper)
                self.assertIn(scenario.row, wrapper)


class ChorusScenarioTest(unittest.TestCase):
    def setUp(self) -> None:
        self.air = BlockState("minecraft:air")
        self.end_stone = BlockState("minecraft:end_stone")
        self.stone = BlockState("minecraft:stone")

    def test_the_two_chorus_scenarios_have_unique_names_and_the_tick_row(self) -> None:
        chorus = (CHORUS_SURVIVAL_TICK, CHORUS_RANDOM_GROWTH)
        self.assertEqual(
            [scenario.name for scenario in chorus],
            ["chorus_survival_tick", "chorus_random_growth"],
        )
        self.assertTrue(all(scenario.row == "server-block-tick-scheduling" for scenario in chorus))
        for scenario in chorus:
            self.assertEqual(
                sum(candidate.name == scenario.name for candidate in SCENARIOS),
                1,
            )

    def test_survival_setup_leaves_the_unsupported_flower_for_the_step(self) -> None:
        # The last write removes support after placing the flower. Vanilla's
        # updateShape schedules, rather than performs, the one-tick survival
        # check (ChorusFlowerBlock.java:149-160).
        self.assertEqual(
            CHORUS_SURVIVAL_TICK.setup,
            (
                "setblock 80 -60 4 minecraft:end_stone",
                "setblock 80 -59 4 minecraft:chorus_flower[age=5]",
                "setblock 80 -60 4 minecraft:air",
            ),
        )
        self.assertEqual(CHORUS_SURVIVAL_TICK.box, Box(80, -60, 4, 80, -58, 4))

    def test_random_growth_setup_names_the_support_flower_and_blocker(self) -> None:
        self.assertEqual(
            CHORUS_RANDOM_GROWTH.setup,
            (
                "setblock 84 -60 4 minecraft:end_stone",
                "setblock 84 -59 4 minecraft:chorus_flower[age=4]",
                "setblock 84 -56 4 minecraft:stone",
            ),
        )
        self.assertEqual(CHORUS_RANDOM_GROWTH.box, Box(84, -60, 4, 84, -56, 4))

    def test_each_chorus_setup_gets_its_forceload_and_safe_ground_first(self) -> None:
        preparation = [
            *PREAMBLE,
            "forceload add 80 0",
            f"fill 80 {GROUND_Y} 0 95 {GROUND_Y} 15 {GROUND_BLOCK}",
        ]
        for scenario in (CHORUS_SURVIVAL_TICK, CHORUS_RANDOM_GROWTH):
            with self.subTest(scenario=scenario.name):
                self.assertEqual(setup_script([scenario]), preparation + list(scenario.setup))

    def test_survival_terminal_state_is_a_measurement(self) -> None:
        prepared = {
            (80, -60, 4): self.air,
            (80, -59, 4): BlockState("minecraft:chorus_flower", (("age", "5"),)),
            (80, -58, 4): self.air,
        }
        terminal = {position: self.air for position in CHORUS_SURVIVAL_TICK.box.positions()}
        result = compare_scenario(CHORUS_SURVIVAL_TICK, prepared, terminal, terminal)
        self.assertTrue(result["pass"])
        self.assertTrue(result["measured"])
        self.assertEqual(result["official_changed_from_input"], 1)

    def test_random_growth_terminal_state_is_a_measurement(self) -> None:
        prepared = {
            (84, -60, 4): self.end_stone,
            (84, -59, 4): BlockState("minecraft:chorus_flower", (("age", "4"),)),
            (84, -58, 4): self.air,
            (84, -57, 4): self.air,
            (84, -56, 4): self.stone,
        }
        plant = BlockState(
            "minecraft:chorus_plant",
            (
                ("down", "true"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "true"),
                ("west", "false"),
            ),
        )
        terminal = {
            (84, -60, 4): self.end_stone,
            (84, -59, 4): plant,
            (84, -58, 4): BlockState("minecraft:chorus_flower", (("age", "5"),)),
            (84, -57, 4): self.air,
            (84, -56, 4): self.stone,
        }
        result = compare_scenario(CHORUS_RANDOM_GROWTH, prepared, terminal, terminal)
        self.assertTrue(result["pass"])
        self.assertTrue(result["measured"])
        self.assertEqual(result["official_changed_from_input"], 2)

    def test_chorus_agreement_on_setup_or_authority_no_op_is_rejected(self) -> None:
        for scenario in (CHORUS_SURVIVAL_TICK, CHORUS_RANDOM_GROWTH):
            with self.subTest(scenario=scenario.name, case="setup"):
                empty = {position: self.air for position in scenario.box.positions()}
                result = compare_scenario(scenario, empty, empty, empty)
                self.assertFalse(result["measured"])
                self.assertIn("setup never landed", " ".join(result["not_measured"]))

            with self.subTest(scenario=scenario.name, case="authority"):
                prepared = {position: self.air for position in scenario.box.positions()}
                first = next(iter(scenario.box.positions()))
                prepared[first] = self.stone
                result = compare_scenario(scenario, prepared, prepared, prepared)
                self.assertFalse(result["measured"])
                self.assertIn(
                    "official server changed no block",
                    " ".join(result["not_measured"]),
                )


class LeafCascadeArithmeticTest(unittest.TestCase):
    """The convergence `LEAF_DISTANCE_CASCADE` claims, re-derived here.

    `LeavesBlock.updateDistance` (LeavesBlock.java:111-124) is small enough to
    model exactly: start at `DECAY_DISTANCE` 7 and take the minimum with each
    neighbour's distance plus one, a neighbour without the property counting as
    7 (LeavesBlock.java:126-136). `LeavesBlock.updateShape`
    (LeavesBlock.java:103-106) reschedules a leaf whenever a neighbour changes.
    The reloaded order the ticks come out in is not fixed
    (LevelChunkTicks.java:91-114), so every order the first tick can take is
    tried and all of them have to land on the same terminal shape -- which is
    the shape the scenario's box is compared against.
    """

    def _row(self) -> dict[int, int]:
        row: dict[int, int] = {}
        for command in LEAF_DISTANCE_CASCADE.setup:
            parts = command.split()
            if parts[0] != "setblock" or "oak_leaves" not in parts[4]:
                continue
            distance = parts[4].split("distance=")[1].split(",")[0]
            row[int(parts[1])] = int(distance)
        return row

    def _settle(self, row: dict[int, int], order: tuple[int, ...]) -> tuple[dict[int, int], int]:
        distances = dict(row)
        pending = list(order)
        ticks = 0
        while pending and ticks <= DEFAULT_STEP_TICKS:
            ticks += 1
            following: list[int] = []
            for x in pending:
                # `updateDistance`: 7, then the minimum with each neighbour + 1.
                # Only the two along the row can be leaves; the other four
                # directions are air, which counts as 7 and never lowers the cap.
                settled = min(
                    [7] + [distances.get(x + step, 7) + 1 for step in (-1, 1)]
                )
                if settled != distances[x]:
                    distances[x] = settled
                    for step in (-1, 1):
                        if x + step in distances and x + step not in following:
                            following.append(x + step)
            pending = following
        return distances, ticks

    def test_the_setup_is_the_row_the_scenario_describes(self) -> None:
        self.assertEqual(self._row(), {19: 1, 20: 2, 21: 3, 22: 4, 23: 5, 24: 6})
        # The log is placed and then taken away again, so the settled row has
        # no support at either end.
        self.assertIn("setblock 18 -55 2 minecraft:oak_log", LEAF_DISTANCE_CASCADE.setup)
        self.assertIn("setblock 18 -55 2 minecraft:air", LEAF_DISTANCE_CASCADE.setup)
        self.assertEqual(LEAF_DISTANCE_CASCADE.setup[-1], "setblock 18 -55 2 minecraft:air")

    def test_every_first_tick_order_settles_on_distance_seven(self) -> None:
        row = self._row()
        # The leaves that carry a pending tick out of the setup are the five
        # each later `/setblock` updated the shape of; the sixth is only
        # reached once its neighbour moves.
        scheduled = sorted(row)[:-1]
        worst = 0
        for order in itertools.permutations(scheduled):
            with self.subTest(order=order):
                settled, ticks = self._settle(row, order)
                self.assertEqual(set(settled.values()), {7})
                self.assertLess(ticks, DEFAULT_STEP_TICKS)
                worst = max(worst, ticks)
        # The docstring's number, so a setup change that slows the cascade down
        # has to move it.
        self.assertEqual(worst, 7)

    def test_the_compared_box_is_exactly_the_row_plus_the_vacated_support(self) -> None:
        box = LEAF_DISTANCE_CASCADE.box
        row = self._row()
        self.assertEqual(box.max_x, max(row))
        self.assertEqual(box.min_x, min(row) - 1)
        self.assertEqual(box.volume, len(row) + 1)


class FallingBlockArithmeticTest(unittest.TestCase):
    """`ENTITY_FALL_AND_LAND`'s two flights, re-derived here.

    `FallingBlockEntity.tick` adds `getDefaultGravity`'s 0.04 to the velocity
    (FallingBlockEntity.java:142-144 via Entity.java:1440-1445), moves by it,
    and only then scales the velocity by 0.98
    (FallingBlockEntity.java:152-154 and :240). The scenario's whole point is
    that one block has landed and the other has not at the moment the step
    ends, so both ends of that need a margin rather than a coincidence.
    """

    GRAVITY = 0.04
    DRAG = 0.98
    #: `FallingBlock.getDelayAfterPlace` (FallingBlock.java:59-61) plus the fact
    #: that the entity is added to `entityTickList` before the same tick's
    #: entity loop reaches it, so a step of N ticks gives the entity N - 1.
    ENTITY_TICKS = DEFAULT_STEP_TICKS - 1

    def _fall(self, start_y: float, ticks: int, ground_y: float | None) -> tuple[float, int | None]:
        """Where the entity is after `ticks`, and which tick it landed on."""
        y = start_y
        velocity = 0.0
        for tick in range(1, ticks + 1):
            velocity -= self.GRAVITY
            if ground_y is not None and y + velocity <= ground_y:
                return ground_y, tick
            y += velocity
            velocity *= self.DRAG
        return y, None

    def _placements(self) -> dict[int, int]:
        placed: dict[int, int] = {}
        for command in ENTITY_FALL_AND_LAND.setup:
            parts = command.split()
            self.assertEqual(parts[0], "setblock")
            placed[int(parts[1])] = int(parts[2])
        return placed

    def test_the_low_block_lands_well_inside_the_step(self) -> None:
        placed = self._placements()
        low = min(placed.values())
        landing = GROUND_Y + 1
        rest, tick = self._fall(float(low), self.ENTITY_TICKS, float(landing))
        self.assertEqual(rest, float(landing))
        self.assertIsNotNone(tick)
        assert tick is not None
        self.assertEqual(tick, 17)
        self.assertLess(tick * 4, self.ENTITY_TICKS, "the landing has no margin")

    def test_the_high_block_is_still_airborne_when_the_step_ends(self) -> None:
        placed = self._placements()
        high = max(placed.values())
        landing = GROUND_Y + 1
        y, tick = self._fall(float(high), self.ENTITY_TICKS, float(landing))
        self.assertIsNone(tick, "the high block landed inside the step")
        # Terminal speed is 1.96 blocks a tick, so this is the number of extra
        # ticks the step would have to gain before the scenario stops working.
        self.assertGreater(y - landing, 50.0)
        self.assertGreater((y - landing) / 1.96, 25.0)

    def test_the_compared_box_spans_both_columns_and_the_landing(self) -> None:
        placed = self._placements()
        box = ENTITY_FALL_AND_LAND.box
        self.assertEqual(box.min_x, min(placed))
        self.assertEqual(box.max_x, max(placed))
        self.assertEqual(box.min_y, GROUND_Y)
        # The low block's own y is inside the box, so "it left" is visible as
        # well as "it arrived".
        self.assertEqual(box.max_y, min(placed.values()))
        # The high block is placed far above the box on purpose: nothing about
        # it should be readable from the blocks, only from `entities/`.
        self.assertGreater(max(placed.values()), box.max_y)
        self.assertIsNotNone(ENTITY_FALL_AND_LAND.entities)


class CommandRefusalTest(unittest.TestCase):
    def test_every_command_the_setup_types_has_a_refusal_marker(self) -> None:
        # A command that parses and then refuses prints a plain line: vanilla
        # only appends the `<--[HERE]` context when the exception carries the
        # input it failed on. So the scenario setup's own refusals have to be
        # named one by one, or a scenario that was never built would be
        # compared as two agreeing boxes of air.
        for marker in (
            "Could not set the block",
            "No blocks were filled",
            "No chunks were marked for force loading",
            "That position is not loaded",
        ):
            with self.subTest(marker=marker):
                self.assertIn(marker, COMMAND_FAILURE_MARKERS)

    def test_the_step_refusal_is_a_marker_because_the_return_code_is_not(self) -> None:
        # `TickCommand.step` returns 1 whether or not it stepped, so this line
        # is the only thing that tells a refused step from a taken one.
        self.assertIn(STEP_FAILURE_TEXT, COMMAND_FAILURE_MARKERS)
        self.assertNotIn(step_success_text(1), COMMAND_FAILURE_MARKERS)

    def test_no_marker_is_a_prefix_of_a_success_line(self) -> None:
        # A marker that also matched a success line would fail every run.
        for marker in COMMAND_FAILURE_MARKERS:
            with self.subTest(marker=marker):
                self.assertNotIn(marker, "Stepping 200 tick(s)")
                self.assertNotIn(marker, "The game is frozen")
                self.assertNotIn(marker, "Saved the game")


class ManagementScopeTest(unittest.TestCase):
    def test_the_bbb_side_asks_for_the_scope_that_reaches_the_dispatcher(self) -> None:
        # `ServerProcess` defaults to `manage`, which the server refuses for
        # `command.execute` -- and the freeze and the step are both
        # `command.execute`. The ladder is ordered, so the same grant still
        # reaches `server.save` and `server.stop`.
        self.assertEqual(BBB_LOOPBACK_SCOPE, "command")
        self.assertEqual(
            inspect.signature(ServerProcess).parameters["loopback_scope"].default,
            "manage",
            "the default this constant exists to override has moved",
        )
        source = inspect.getsource(run_bbb)
        self.assertIn("loopback_scope=BBB_LOOPBACK_SCOPE", source)


class StepTest(unittest.TestCase):
    def test_the_step_is_expressed_in_the_tick_unit(self) -> None:
        # `TimeArgument`'s unit table maps `t` to one tick, so the number in the
        # command is the number of ticks; any other suffix would multiply it.
        self.assertEqual(step_command(DEFAULT_STEP_TICKS), f"tick step {DEFAULT_STEP_TICKS}t")
        self.assertEqual(step_success_text(7), "Stepping 7 tick(s)")

    def test_a_side_that_ran_exactly_the_step_is_accepted(self) -> None:
        self.assertEqual(step_progress(1000, 1200, 200), "reached")
        self.assertEqual(require_stepped("official", 1000, 1200, 200), 1200)

    def test_a_side_that_stopped_short_is_named_with_both_numbers(self) -> None:
        self.assertEqual(step_progress(1000, 1150, 200), "short")
        with self.assertRaises(OracleError) as caught:
            require_stepped("bbb", 1000, 1150, 200)
        message = str(caught.exception)
        self.assertIn("bbb", message)
        self.assertIn("150 ticks", message)
        self.assertIn("1200", message)
        self.assertIn("short of", message)

    def test_a_side_that_ran_past_the_step_is_the_failure_a_sleep_cannot_see(self) -> None:
        # A wall-clock harness reads this as success: the state it wanted has
        # been reached and then left behind.
        self.assertEqual(step_progress(1000, 1201, 200), "past")
        with self.assertRaises(OracleError) as caught:
            require_stepped("bbb", 1000, 1201, 200)
        self.assertIn("past", str(caught.exception))

    def test_a_save_with_no_game_time_is_not_a_finished_step(self) -> None:
        self.assertEqual(step_progress(1000, None, 200), "short")
        with self.assertRaises(OracleError) as caught:
            require_stepped("official", 1000, None, 200)
        self.assertIn("no Data.Time", str(caught.exception))


class BoxTest(unittest.TestCase):
    def test_volume_matches_the_positions_enumerated(self) -> None:
        box = Box(0, -60, 3, 15, -58, 5)
        self.assertEqual(box.volume, 16 * 3 * 3)
        self.assertEqual(len(list(box.positions())), box.volume)
        self.assertIn((0, -60, 3), list(box.positions()))
        self.assertIn((15, -58, 5), list(box.positions()))

    def test_chunk_columns_use_the_floor_division_a_negative_coordinate_needs(self) -> None:
        self.assertEqual(
            Box(-1, 0, -1, 0, 0, 0).chunk_columns(),
            [(-1, -1), (-1, 0), (0, -1), (0, 0)],
        )


class CompareTest(unittest.TestCase):
    def setUp(self) -> None:
        self.scenario = Scenario(
            name="probe",
            owner="test",
            row="test",
            setup=(),
            box=Box(0, 0, 0, 1, 0, 0),
        )
        self.air = BlockState("minecraft:air")
        self.stone = BlockState("minecraft:stone")
        self.water = BlockState("minecraft:water", (("level", "1"),))

    def test_agreement_passes_and_names_no_coordinate(self) -> None:
        prepared = {(0, 0, 0): self.stone, (1, 0, 0): self.air}
        reading = {(0, 0, 0): self.stone, (1, 0, 0): self.water}
        result = compare_scenario(self.scenario, prepared, reading, reading)
        self.assertTrue(result["pass"])
        self.assertEqual(result["compared"], 2)
        self.assertEqual(result["matched"], 2)
        self.assertEqual(result["differences"], [])
        self.assertFalse(result["bbb_made_no_change"])
        self.assertTrue(result["measured"])
        self.assertEqual(result["not_measured"], [])
        self.assertEqual(result["official_changed_from_input"], 1)

    def test_a_candidate_that_did_nothing_is_reported_as_such(self) -> None:
        prepared = {(0, 0, 0): self.stone, (1, 0, 0): self.air}
        official = {(0, 0, 0): self.stone, (1, 0, 0): self.water}
        result = compare_scenario(self.scenario, prepared, official, prepared)
        self.assertFalse(result["pass"])
        self.assertEqual(result["matched"], 1)
        self.assertTrue(result["bbb_made_no_change"])
        self.assertEqual(result["not_measured"], [])
        self.assertEqual(
            result["differences"],
            [
                {
                    "x": 1,
                    "y": 0,
                    "z": 0,
                    "prepared": "minecraft:air",
                    "official": "minecraft:water[level=1]",
                    "bbb": "minecraft:air",
                    "bbb_unchanged_from_input": True,
                }
            ],
        )

    def test_a_candidate_that_did_something_else_is_told_apart(self) -> None:
        prepared = {(0, 0, 0): self.stone, (1, 0, 0): self.air}
        official = {(0, 0, 0): self.stone, (1, 0, 0): self.water}
        candidate = {(0, 0, 0): self.stone, (1, 0, 0): BlockState("minecraft:lava")}
        result = compare_scenario(self.scenario, prepared, official, candidate)
        self.assertFalse(result["bbb_made_no_change"])
        self.assertFalse(result["differences"][0]["bbb_unchanged_from_input"])

    def test_an_unwritten_block_is_named_rather_than_dropped(self) -> None:
        prepared = {(0, 0, 0): self.stone, (1, 0, 0): self.air}
        official = {(0, 0, 0): self.stone, (1, 0, 0): self.water}
        result = compare_scenario(self.scenario, prepared, official, {})
        self.assertEqual(describe(None), "<unwritten>")
        self.assertEqual(len(result["differences"]), 2)
        self.assertEqual(result["differences"][0]["bbb"], "<unwritten>")


class NotAMeasurementTest(unittest.TestCase):
    """The three ways two servers can agree about nothing at all."""

    def setUp(self) -> None:
        self.scenario = Scenario(
            name="probe",
            owner="test",
            row="test",
            setup=("setblock 0 0 0 minecraft:sand",),
            box=Box(0, 0, 0, 1, 0, 0),
        )
        self.air = BlockState("minecraft:air")
        self.stone = BlockState("minecraft:stone")

    def test_a_setup_that_never_landed_is_not_a_passing_comparison(self) -> None:
        # Every command could have been refused and both servers would still
        # write the same box of air back.
        empty = {(0, 0, 0): self.air, (1, 0, 0): self.air}
        result = compare_scenario(self.scenario, empty, empty, empty)
        self.assertFalse(result["pass"])
        self.assertFalse(result["measured"])
        self.assertEqual(result["difference_count"], 0)
        self.assertIn("setup never landed", " ".join(result["not_measured"]))

    def test_an_authority_that_did_nothing_is_not_a_passing_comparison(self) -> None:
        reading = {(0, 0, 0): self.stone, (1, 0, 0): self.air}
        result = compare_scenario(self.scenario, reading, reading, reading)
        self.assertFalse(result["pass"])
        self.assertEqual(result["official_changed_from_input"], 0)
        self.assertIn("official server changed no block", " ".join(result["not_measured"]))

    def test_two_empty_entity_lists_are_not_agreement(self) -> None:
        # `WorldSaveReader.entities` answers `[]` for a column the save never
        # wrote as well as for one written empty, so this is the reading a
        # server that never spawned the entity produces.
        scenario = Scenario(
            name="probe",
            owner="test",
            row="test",
            setup=("setblock 0 0 0 minecraft:sand",),
            box=Box(0, 0, 0, 1, 0, 0),
            entities=EntityProbe(chunk_x=0, chunk_z=0),
        )
        prepared = {(0, 0, 0): self.stone, (1, 0, 0): self.air}
        official = {(0, 0, 0): self.stone, (1, 0, 0): self.stone}
        result = compare_scenario(
            scenario, prepared, official, official, official_entities=[], candidate_entities=[]
        )
        self.assertEqual(result["difference_count"], 0)
        self.assertTrue(result["entity"]["empty_on_both_sides"])
        self.assertFalse(result["pass"])
        self.assertIn("neither side wrote an entity", " ".join(result["not_measured"]))

    def test_one_record_on_each_side_is_a_measurement(self) -> None:
        scenario = Scenario(
            name="probe",
            owner="test",
            row="test",
            setup=("setblock 0 0 0 minecraft:sand",),
            box=Box(0, 0, 0, 1, 0, 0),
            entities=EntityProbe(chunk_x=0, chunk_z=0),
        )
        prepared = {(0, 0, 0): self.stone, (1, 0, 0): self.air}
        official = {(0, 0, 0): self.stone, (1, 0, 0): self.stone}
        record = {"id": "minecraft:falling_block", "Time": 3}
        result = compare_scenario(
            scenario,
            prepared,
            official,
            official,
            official_entities=[record],
            candidate_entities=[record],
        )
        self.assertTrue(result["pass"])
        self.assertTrue(result["measured"])
        self.assertFalse(result["entity"]["empty_on_both_sides"])


class EntityCompareTest(unittest.TestCase):
    def setUp(self) -> None:
        self.probe = EntityProbe(chunk_x=2, chunk_z=0)

    def _falling(self, y: float, **overrides: object) -> dict[str, object]:
        record = {
            "id": "minecraft:falling_block",
            "Pos": [34.5, y, 8.5],
            "Motion": [0.0, -1.92, 0.0],
            "OnGround": False,
            "BlockState": {"Name": "minecraft:sand"},
            "Time": 197,
            "UUID": [1, 2, 3, 4],
        }
        record.update(overrides)
        return record

    def test_two_identical_records_agree(self) -> None:
        result = compare_entities(self.probe, [self._falling(2.0)], [self._falling(2.0)])
        self.assertEqual(result["difference_count"], 0)
        self.assertEqual(result["official_count"], 1)
        self.assertEqual(result["bbb_count"], 1)
        self.assertEqual(result["records"][0]["differences"], [])
        self.assertEqual(result["compared_fields"], list(ENTITY_FIELDS))

    def test_a_random_uuid_is_not_a_difference(self) -> None:
        # `Mth.createInsecureUUID` gives every fresh entity its own id, so the
        # two servers cannot agree on it; comparing it would fail every run.
        result = compare_entities(
            self.probe,
            [self._falling(2.0, UUID=[1, 2, 3, 4])],
            [self._falling(2.0, UUID=[9, 9, 9, 9])],
        )
        self.assertEqual(result["difference_count"], 0)
        # The record is still reported whole, so a reader can see what was not
        # compared.
        self.assertNotIn("UUID", result["records"][0]["official"])

    def test_a_moved_entity_is_reported_with_both_sides_and_the_distance(self) -> None:
        result = compare_entities(self.probe, [self._falling(2.0)], [self._falling(3.92)])
        self.assertEqual(result["difference_count"], 1)
        differences = result["records"][0]["differences"]
        self.assertEqual([entry["field"] for entry in differences], ["Pos"])
        self.assertEqual(differences[0]["official"], [34.5, 2.0, 8.5])
        self.assertEqual(differences[0]["bbb"], [34.5, 3.92, 8.5])
        self.assertAlmostEqual(differences[0]["delta"], 1.92)

    def test_an_entity_one_side_never_wrote_is_named_as_missing(self) -> None:
        result = compare_entities(self.probe, [self._falling(2.0)], [])
        self.assertEqual(result["difference_count"], 1)
        self.assertEqual(result["bbb_count"], 0)
        self.assertEqual(result["records"][0]["missing"], "bbb")
        self.assertIsNone(result["records"][0]["bbb"])
        mirrored = compare_entities(self.probe, [], [self._falling(2.0)])
        self.assertEqual(mirrored["records"][0]["missing"], "official")

    def test_records_pair_by_type_rather_than_by_file_order(self) -> None:
        item = {"id": "minecraft:item", "Pos": [1.0, 2.0, 3.0]}
        sand = self._falling(2.0)
        result = compare_entities(self.probe, [item, sand], [sand, item])
        self.assertEqual(result["difference_count"], 0)

    def test_a_non_numeric_field_has_no_distance(self) -> None:
        self.assertIsNone(numeric_delta("minecraft:sand", "minecraft:gravel"))
        self.assertIsNone(numeric_delta(True, False))
        self.assertIsNone(numeric_delta([1.0, 2.0], [1.0]))
        self.assertEqual(numeric_delta([1.0, 2.0], [1.0, 2.5]), 0.5)


class ScenarioEntityVerdictTest(unittest.TestCase):
    def test_an_entity_difference_fails_a_scenario_whose_blocks_all_agree(self) -> None:
        scenario = Scenario(
            name="probe",
            owner="test",
            row="test",
            setup=("setblock 0 0 0 minecraft:sand",),
            box=Box(0, 0, 0, 1, 0, 0),
            entities=EntityProbe(chunk_x=0, chunk_z=0),
        )
        sand = BlockState("minecraft:sand")
        prepared = {(0, 0, 0): sand, (1, 0, 0): BlockState("minecraft:air")}
        reading = {(0, 0, 0): sand, (1, 0, 0): sand}
        result = compare_scenario(
            scenario,
            prepared,
            reading,
            reading,
            official_entities=[{"id": "minecraft:falling_block", "Time": 3}],
            candidate_entities=[],
        )
        self.assertFalse(result["pass"])
        # The block totals keep meaning what they meant; the entity half carries
        # its own count and both feed `difference_count`.
        self.assertEqual(result["compared"], 2)
        self.assertEqual(result["matched"], 2)
        self.assertEqual(result["block_difference_count"], 0)
        self.assertEqual(result["entity_difference_count"], 1)
        self.assertEqual(result["difference_count"], 1)
        self.assertEqual(result["entity"]["records"][0]["missing"], "bbb")
        # The blocks did move, so this is a real measurement that the entity
        # half failed -- not one of the non-measurements.
        self.assertEqual(result["not_measured"], [])

    def test_a_scenario_with_no_probe_reports_no_entity_half(self) -> None:
        scenario = Scenario(
            name="blocks-only",
            owner="test",
            row="test",
            setup=("setblock 0 0 0 minecraft:sand",),
            box=Box(0, 0, 0, 0, 0, 0),
        )
        prepared = {(0, 0, 0): BlockState("minecraft:sand")}
        reading = {(0, 0, 0): BlockState("minecraft:air")}
        result = compare_scenario(scenario, prepared, reading, reading)
        self.assertIsNone(result["entity"])
        self.assertEqual(result["entity_difference_count"], 0)
        self.assertTrue(result["pass"])


class ResultTest(unittest.TestCase):
    def test_a_result_totals_its_scenarios_and_fails_when_any_does(self) -> None:
        passing = {"compared": 4, "difference_count": 0}
        failing = {"compared": 6, "difference_count": 2}
        result = build_result(
            [passing, failing],
            step_ticks=200,
            world_template=Path("/tmp/world"),
            game_times={"prepared": 10, "official": 210, "bbb": 205},
            generated_at="2026-08-18T00:00:00Z",
        )
        self.assertEqual(result["compared"], 10)
        self.assertEqual(result["difference_count"], 2)
        self.assertFalse(result["pass"])
        self.assertEqual(result["generated_at"], "2026-08-18T00:00:00Z")
        self.assertEqual(result["preamble"], list(PREAMBLE))
        self.assertEqual(result["game_time"]["official"], 210)

    def test_a_result_states_the_game_time_the_step_should_have_reached(self) -> None:
        # The three times and the delta are the receipt for the whole
        # tick-precise design, so they are in the JSON whether or not the
        # comparison passed.
        result = build_result(
            [{"compared": 4, "difference_count": 0}],
            step_ticks=200,
            world_template=Path("/tmp/world"),
            game_times={"prepared": 1000, "official": 1200, "bbb": 1200},
            stepped={"official": 197, "bbb": 194},
        )
        self.assertTrue(result["pass"])
        self.assertEqual(result["step"]["ticks"], 200)
        # The two sides are told to take different numbers of ticks because
        # they ran different numbers before their freeze landed; what the run
        # asserts is the total, which is the same on both.
        self.assertEqual(result["step"]["stepped"], {"official": 197, "bbb": 194})
        self.assertEqual(result["step"]["expected_game_time"], 1200)
        self.assertEqual(result["step"]["expected_game_time_delta"], 200)
        self.assertEqual(result["game_time"]["expected"], 1200)
        self.assertEqual(result["game_time"]["expected_delta"], 200)
        self.assertEqual(
            result["game_time"]["official"] - result["game_time"]["prepared"], 200
        )

    def test_a_scenario_that_measured_nothing_fails_the_whole_run(self) -> None:
        # It reports no difference, because there was nothing to differ. A run
        # that totalled that as a pass would be a green receipt for behaviour
        # nobody exercised.
        result = build_result(
            [
                {"name": "real", "compared": 4, "difference_count": 0},
                {
                    "name": "hollow",
                    "compared": 6,
                    "difference_count": 0,
                    "not_measured": ["the official server changed no block"],
                },
            ],
            step_ticks=200,
            world_template=Path("/tmp/world"),
            game_times={"prepared": 10, "official": 210, "bbb": 210},
        )
        self.assertEqual(result["difference_count"], 0)
        self.assertEqual(result["unmeasured"], ["hollow"])
        self.assertFalse(result["pass"])

    def test_a_result_with_no_prepared_time_names_no_expectation(self) -> None:
        result = build_result(
            [{"compared": 4, "difference_count": 0}],
            step_ticks=100,
            world_template=Path("/tmp/world"),
            game_times={},
        )
        self.assertIsNone(result["step"]["expected_game_time"])
        self.assertEqual(result["step"]["expected_game_time_delta"], 100)


if __name__ == "__main__":
    unittest.main()


class StepToGameTimeTest(unittest.TestCase):
    """The driver that removes the boot race from the measurement."""

    def _side(self, frozen_at: int, per_step: int = 1):
        """A fake side whose clock advances `per_step` ticks per stepped tick."""
        directory = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, directory, ignore_errors=True)
        clock = {"time": frozen_at}
        taken: list[int] = []

        def freeze() -> None:
            return None

        def step(count: int) -> None:
            taken.append(count)
            clock["time"] += count * per_step

        def save() -> None:
            document = _named_root(
                [(10, "Data", _compound([(4, "Time", struct.pack(">q", clock["time"]))]))]
            )
            (directory / "level.dat").write_bytes(gzip.compress(document))

        return directory, taken, freeze, step, save

    def test_the_step_count_is_the_distance_to_the_shared_target(self) -> None:
        # A side that ran seven ticks before its freeze landed is stepped seven
        # fewer, so both sides run the same total over the same input.
        directory, taken, freeze, step, save = self._side(1007)
        steps = step_to_game_time("fake", directory, freeze, step, save, 1200)
        self.assertEqual(steps, 193)
        self.assertEqual(taken, [193])
        self.assertEqual(game_time(directory), 1200)

    def test_a_side_already_past_the_target_is_a_named_failure(self) -> None:
        directory, _, freeze, step, save = self._side(1200)
        with self.assertRaises(OracleError) as raised:
            step_to_game_time("fake", directory, freeze, step, save, 1200)
        self.assertIn("already at Data.Time 1200", str(raised.exception))

    def test_a_side_that_overruns_its_step_is_reported_rather_than_compared(self) -> None:
        # Two ticks per stepped tick: the failure a wall-clock harness could
        # never see, because from outside it looks exactly like success.
        directory, _, freeze, step, save = self._side(1000, per_step=2)
        with self.assertRaises(OracleError) as raised:
            step_to_game_time("fake", directory, freeze, step, save, 1100)
        self.assertIn("past", str(raised.exception))


class BbbCommandResultTest(unittest.TestCase):
    """`/function` is a custom executor, so a resultless answer is not a refusal."""

    class Server:
        def __init__(self, answer: dict[str, object]) -> None:
            self.answer = answer

        def management(self, *_arguments: str) -> dict[str, object]:
            return {"command": self.answer}

    def test_a_missing_result_is_a_refusal_only_while_one_is_expected(self) -> None:
        answer = {"result": None, "messages": [], "broadcast": None}
        server = self.Server(answer)
        with self.assertRaises(OracleError):
            bbb_command(server, "function bbb:overflow")
        self.assertEqual(
            bbb_command(server, "function bbb:overflow", expect_result=False), answer
        )
