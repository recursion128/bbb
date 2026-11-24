"""The captured world's expectations must track the replay that produces it.

`scripts/capture-official-world-26.1.sh` boots the official server and drives
the bbb interaction replay at it; the world it saves is then read back by the
ignored `BBB_TEST_JAVA_26_1_WORLD` persistence tests. Those tests state what the
replay left behind, but they live in `bbb-server-core`, which cannot reach the
client's replay constants — so a replay change can silently invalidate them, and
once did: the held stack grew from 32 to 48 and the expectation stayed at 32
until a release run happened to boot the capture again.

This is the mechanical alignment that would have caught it.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
REPLAY_CONSTANTS = REPO_ROOT / "crates/bbb-client-probe/src/types/interaction_replay.rs"
PLAYER_SCHEMA = (
    REPO_ROOT / "crates/bbb-server-persistence/src/player_schema/tests.rs"
)
# Every script that boots an official server for the interaction replay to join.
OFFICIAL_SERVER_SCRIPTS = (
    REPO_ROOT / "scripts/capture-official-world-26.1.sh",
    REPO_ROOT / "scripts/server-round-trip-gate-26.1.sh",
    REPO_ROOT / "scripts/server_playable_gate.py",
    REPO_ROOT / "scripts/server-dimension-runtime-gate-26.1.sh",
)
# The release gate drives the same BBB interaction replay.
RELEASE_GATE = REPO_ROOT / "scripts/server_release_gate.py"
#: The one declaration of what that server has to be.
SHARED_PROPERTIES = "scripts/data/official-replay-server.properties"
# The differential alone compares packet counters that vary with the first
# player's randomly selected spawn chunk, so it has one pre-login console fixture.
DIFFERENTIAL_COMMANDS = "scripts/data/official-differential-server.commands"
#: Every script that boots `bbb-server` for the same replay to join, or for a
#: save an official server wrote or will read. bbb-server takes its settings on
#: the command line instead of from a properties file, so nothing but a check
#: like this keeps its flags saying what the shared file says.
BBB_SERVER_SCRIPTS = (
    REPO_ROOT / "scripts/server_playable_gate.py",
    REPO_ROOT / "scripts/server-round-trip-gate-26.1.sh",
    REPO_ROOT / "scripts/server-dimension-runtime-gate-26.1.sh",
    RELEASE_GATE,
)


def replay_constant(name: str) -> int:
    body = REPLAY_CONSTANTS.read_text(encoding="utf-8")
    match = re.search(rf"pub const {name}: i32 =\s*(?P<value>[^;]+);", body)
    if match is None:
        raise AssertionError(f"{REPLAY_CONSTANTS} declares no {name}")
    expression = " ".join(match.group("value").split())
    if expression.isdigit():
        return int(expression)
    # The composed form, `A + B`, resolved through the same declarations.
    return sum(
        replay_constant(term.strip()) if not term.strip().isdigit() else int(term.strip())
        for term in expression.split("+")
    )


class CaptureReplayExpectationTests(unittest.TestCase):
    def test_the_captured_held_stack_is_the_one_the_replay_leaves(self) -> None:
        body = PLAYER_SCHEMA.read_text(encoding="utf-8")
        match = re.search(r"const CAPTURED_HELD_STONE: i32 = (?P<value>\d+);", body)
        self.assertIsNotNone(match, f"{PLAYER_SCHEMA} declares no CAPTURED_HELD_STONE")
        assert match is not None
        self.assertEqual(
            int(match.group("value")),
            replay_constant("INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE"),
        )

    def test_the_capture_recipe_baseline_includes_the_redstone_replay_leg(self) -> None:
        player_schema = PLAYER_SCHEMA.read_text(encoding="utf-8")
        count = re.search(
            r"const CAPTURED_RECIPE_COUNT: usize = (?P<value>\d+);", player_schema
        )
        self.assertIsNotNone(count, f"{PLAYER_SCHEMA} declares no recipe baseline")
        assert count is not None
        replay = REPLAY_CONSTANTS.read_text(encoding="utf-8")
        self.assertIn(
            'INTERACTION_REDSTONE_WIRE_ITEM_NAME: &str = "minecraft:redstone"', replay
        )
        self.assertEqual(27, int(count.group("value")))

    def test_every_official_server_reads_one_replay_configuration(self) -> None:
        # The replay observes a second client, so a one-seat server turns the
        # peer away and the run never reaches its edits. Both the capture and
        # the round-trip gate declared one seat until a release run tripped
        # over it. The fix was one file every script reads; this is what keeps
        # a script from quietly going back to writing its own.
        shared = (REPO_ROOT / SHARED_PROPERTIES).read_text(encoding="utf-8")
        seats = re.search(r"^max-players=(?P<seats>\d+)$", shared, re.MULTILINE)
        self.assertIsNotNone(seats, f"{SHARED_PROPERTIES} declares no seats")
        assert seats is not None
        self.assertGreaterEqual(int(seats.group("seats")), 2)
        self.assertIn("gamemode=creative", shared)

        for script in OFFICIAL_SERVER_SCRIPTS:
            with self.subTest(script=script.name):
                body = script.read_text(encoding="utf-8")
                self.assertIn(
                    SHARED_PROPERTIES,
                    body,
                    f"{script.name} boots an official server without the shared settings",
                )
                # The property assignment, not the flag: `--max-players` is
                # bbb-server's own option and has nothing to do with the
                # official server's properties file.
                for own in ("max-players=", "gamemode=", "difficulty=", "online-mode="):
                    self.assertNotIn(
                        own,
                        body,
                        f"{script.name} sets {own} itself instead of reading it",
                    )

    def test_every_bbb_server_is_started_on_the_shared_world_settings(self) -> None:
        # Each of these is applied over what the save itself holds, at every
        # startup, on both servers: `gamemode` and `force-gamemode` by
        # `DedicatedServer.java:237` and `:792-794`, `difficulty` by
        # `forceDifficulty` (`DedicatedServer.java:395-397`). The official side
        # reads them out of the shared properties file; bbb-server has no
        # properties file and takes them as flags, and its own defaults are
        # vanilla's (`survival`, unset, `easy`) rather than this replay's. So a
        # script that leaves one off runs the replay against a server that
        # differs from the one it is compared with -- and, since bbb writes the
        # live game type and difficulty back into `level.dat`, republishes a
        # save that answers differently too.
        shared = (REPO_ROOT / SHARED_PROPERTIES).read_text(encoding="utf-8")
        for key, flag in (("gamemode", "--gamemode"), ("difficulty", "--difficulty")):
            declared = re.search(
                rf"^{key}=(?P<value>[a-z]+)$", shared, re.MULTILINE
            )
            self.assertIsNotNone(declared, f"{SHARED_PROPERTIES} declares no {key}")
            assert declared is not None
            value = declared.group("value")
            for script in BBB_SERVER_SCRIPTS:
                with self.subTest(script=script.name, key=key):
                    body = script.read_text(encoding="utf-8")
                    self.assertRegex(
                        body,
                        rf"{flag}\W+{value}\b",
                        f"{script.name} boots bbb-server without {flag} {value}",
                    )
        # The one boolean: vanilla writes `force-gamemode=true`, bbb-server
        # takes the bare `--force-gamemode` flag, and the property's own default
        # is false (`DedicatedServerProperties.java:63`) — so a `false` here has
        # to mean the flag is absent, not that this check is skipped.
        declared = re.search(r"^force-gamemode=(?P<value>\w+)$", shared, re.MULTILINE)
        self.assertIsNotNone(declared, f"{SHARED_PROPERTIES} declares no force-gamemode")
        assert declared is not None
        forced = declared.group("value") == "true"
        for script in BBB_SERVER_SCRIPTS:
            with self.subTest(script=script.name, key="force-gamemode"):
                body = script.read_text(encoding="utf-8")
                self.assertEqual(
                    "--force-gamemode" in body,
                    forced,
                    f"{script.name} does not boot bbb-server with "
                    f"force-gamemode={str(forced).lower()}",
                )

        protection = re.search(r"^spawn-protection=(\d+)$", shared, re.MULTILINE)
        self.assertIsNotNone(protection)
        assert protection is not None
        playable = (REPO_ROOT / "scripts/server_playable_gate.py").read_text(
            encoding="utf-8"
        )
        self.assertRegex(playable, rf"--spawn-protection\W+{protection.group(1)}\b")

    def test_the_differential_fixes_the_spawn_before_the_official_probe(self) -> None:
        commands = (REPO_ROOT / DIFFERENTIAL_COMMANDS).read_text(encoding="utf-8")
        active = [
            line
            for line in commands.splitlines()
            if line and not line.startswith("#")
        ]
        self.assertEqual(active, ["gamerule minecraft:respawn_radius 0"])

        gate = (REPO_ROOT / "scripts/server_playable_gate.py").read_text(
            encoding="utf-8"
        )
        fixture_at = gate.index(f'REPO_ROOT / "{DIFFERENTIAL_COMMANDS}"')
        success_at = gate.index("Gamerule respawn_radius is now set to: 0")
        probe_at = gate.index("f\"127.0.0.1:{official_port}\"")
        self.assertLess(fixture_at, success_at)
        self.assertLess(success_at, probe_at)

    def test_every_official_server_gives_the_replay_the_same_budget(self) -> None:
        # The replay grew refusal sessions, which run one after another after
        # the rest of it: the capture script kept the old thirty second budget
        # and started timing out while the two gates beside it did not. One
        # budget, read from all three, is what keeps them from drifting apart.
        budgets = {}
        for script in OFFICIAL_SERVER_SCRIPTS:
            body = script.read_text(encoding="utf-8")
            match = re.search(
                r"timeout_seconds=\$\{[A-Z_]+:-(?P<sh>\d+)\}"
                r'|os\.environ\.get\("[A-Z_]+",\s*"(?P<py>\d+)"\)',
                body,
            )
            self.assertIsNotNone(match, f"{script} declares no probe timeout")
            assert match is not None
            budgets[script.name] = int(match.group("sh") or match.group("py"))
            self.assertNotIn(
                "--timeout 30 --",
                body,
                f"{script} still hard-codes a probe timeout beside the one it declares",
            )
        self.assertEqual(
            len(set(budgets.values())),
            1,
            f"the replay gets a different budget per script: {budgets}",
        )

    def test_every_release_gate_login_name_fits_the_login_limit(self) -> None:
        # `ClientRun` refuses a name longer than the limit, but only once the
        # family holding it runs -- and the families run in an order that put a
        # seventeen-character name behind five minutes of another family. The
        # names are literals in one file, so the check belongs where they are
        # written rather than where they are used.
        body = RELEASE_GATE.read_text(encoding="utf-8")
        limit_match = re.search(r"USERNAME_LIMIT = (?P<value>\d+)", body)
        self.assertIsNotNone(limit_match, f"{RELEASE_GATE} declares no USERNAME_LIMIT")
        assert limit_match is not None
        limit = int(limit_match.group("value"))
        names = re.findall(r"username=\"(?P<name>[^\"]+)\"", body)
        self.assertTrue(names, f"{RELEASE_GATE} hands no literal username to a client")
        too_long = sorted({name for name in names if len(name) > limit})
        self.assertEqual(
            too_long,
            [],
            f"{RELEASE_GATE} logs in with names past the {limit}-unit limit: {too_long}",
        )


if __name__ == "__main__":
    unittest.main()
