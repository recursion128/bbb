#!/usr/bin/env python3
"""Drive the dedicated server through its release acceptance families.

The release ledger row names things a unit test cannot reach: twenty bbb
clients on one server at once, a restart that keeps the world, a refusal that
stays attributable under malformed input, and a process whose footprint does
not drift over a long run. Every one of those is a property of the running
process, so this gate runs the real `bbb-server`, drives it with the real
`bbb-client` client and administers it with the real `bbb-admin` client. There
is no bespoke client here: a harness that spoke its own protocol would prove
only that the harness and the server agree.

Each family produces one machine result. `cargo dev e2e server-release`
(`scripts/server_release_gate.py`) is the entry point; a run writes its
per-family results to `<artifact-dir>/release-summary.json`.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from statistics import median
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.execution.process import free_port  # noqa: E402
from bbb_tools.python_runner import default_target_dir as shared_default_target_dir  # noqa: E402
from bbb_tools.vanilla.errors import VanillaError  # noqa: E402
from bbb_tools.vanilla.workspace import (  # noqa: E402
    default_vanilla_root as bbb_default_vanilla_root,
)

#: The release target the ledger names. It is also `--max-players`' default, so
#: the gate exercises the shipped configuration rather than a special one.
RELEASE_PLAYERS = 20


def local_bot_connection_args(clients: int = RELEASE_PLAYERS) -> list[str]:
    return ["--max-connections-per-ip", str(max(16, clients + 4))]

#: How long a load session stays connected. Long enough for every session to
#: overlap and be observed through the management endpoint, short enough that
#: the gate is not dominated by waiting.
HOLD_SECONDS = 12

#: How long to wait for a server, a client or a management answer.
READY_TIMEOUT = 120.0
CLIENT_TIMEOUT = 180.0
MANAGEMENT_TIMEOUT = 60.0

#: The reason the client receives when login or world admission finds a full server.
CAPACITY_REASON = "multiplayer.disconnect.server_full"

#: The management endpoint's default HTTP path, which the hostile family has to
#: write by hand because it speaks raw HTTP rather than going through bbb-admin.
MANAGEMENT_PATH = "/mcp"

#: Vanilla's login name limit, which every generated username has to fit in.
USERNAME_LIMIT = 16

#: How long the hostile family's bystanders stay on the server. It has to
#: outlast every refusal below, because the property is that they play through
#: them rather than that they happened to leave first.
HOSTILE_HOLD_SECONDS = 90
#: How long the creation family's resident client stays on the server. It only
#: has to outlive one management save, so it is far shorter than the hostile
#: bystanders' hold.
CREATION_RESIDENT_HOLD_SECONDS = 20

#: `MinecraftServer.AUTOSAVE_INTERVAL` (MinecraftServer.java:209), which is also
#: what `computeNextAutosaveInterval` returns at the default 20 tps (:1028-1038).
#: The server has no knob for it — vanilla's only lever is the tick rate — so an
#: unattended save costs its full 300 seconds of wall clock and the family that
#: waits for one is priced accordingly.
AUTOSAVE_INTERVAL_TICKS = 6000
AUTOSAVE_RECEIPT_TIMEOUT = 480.0

#: The client's own refusal sessions that send a body no decoder can read
#: (`RefusalCase` in crates/bbb-client-net/src/probe.rs). The interaction replay drives
#: them, so this family gets the play-phase half of its contract from the real
#: client rather than from a harness speaking the protocol.
MALFORMED_PLAY_CASES = ("malformed_packet_body", "malformed_component_nbt")

#: How many rounds the soak family runs. Four settled rounds after the warm-up
#: is what makes a growth reading a trend rather than a pair of numbers.
#: What the restart family drops into the world's datapack directory to check
#: that discovery reports a pack it cannot open instead of failing the reload.
BROKEN_DATAPACK = "bbb-unreadable.zip"

# Used by the optional official differential tools.
OFFICIAL_REPLAY_PROPERTIES = "scripts/data/official-replay-server.properties"

#: What a created save has to hold before anything else can be said about it.
CREATED_SAVE_FILES = (
    "level.dat",
    "data/minecraft/world_gen_settings.dat",
    "dimensions/minecraft/overworld/region/r.0.0.mca",
)

SOAK_ROUNDS = 5

#: One tick's own budget, in microseconds. A window whose ticks average more
#: than this is a loop that cannot keep up with itself.
TICK_PERIOD_MICROS = 50_000

#: `MinecraftServer.OVERLOADED_THRESHOLD_NANOS` (:199): one second behind
#: schedule is where vanilla starts saying the server is overloaded.
VANILLA_OVERLOADED_MICROS = 1_000_000

#: What each footprint reading is allowed to move across the settled rounds.
#: An idle server samples in the tens of megabytes and a handful of descriptors,
#: so these sit far above the noise band and only a real leak reaches them.
SOAK_LIMITS = {"rss_kib": 256 * 1024, "open_files": 32, "threads": 16}


class ReleaseGateError(RuntimeError):
    """The server did not meet a release property."""


@dataclass
class Paths:
    artifact_dir: Path
    server_bin: Path
    client_bin: Path
    admin_bin: Path
    vanilla_root: Path | None = None
    #: Optional runtime for differential debugging, unused by release families.
    java_bin: Path | None = None
    server_jar: Path | None = None
    #: An official Mojang 26.1 save to copy for families that need persistence.
    #: `--world` can only open a save that already exists, so the world a
    #: restart family restarts on has to come from somewhere; the capture script
    #: is the repository's one owner of making one.
    world_template: Path | None = None
    disperse_players: bool = False

    def child(self, *parts: str) -> Path:
        path = self.artifact_dir.joinpath(*parts)
        path.parent.mkdir(parents=True, exist_ok=True)
        return path


@dataclass
class ServerProcess:
    """One `bbb-server` under test, with the two addresses it serves."""

    paths: Paths
    name: str
    game_port: int
    management_port: int
    admin_dir: Path
    #: The save this server opens. Absent keeps the ephemeral flat fixture,
    #: which is what a family about process behaviour under load wants: it is
    #: not testing persistence, and `--world` can only open a save that already
    #: exists (see `server-world-creation` in the server ledger).
    world_dir: Path | None = None
    process: subprocess.Popen[bytes] | None = None
    log_path: Path = field(init=False)

    def __post_init__(self) -> None:
        self.log_path = self.paths.child(f"{self.name}.log")

    @property
    def address(self) -> str:
        return f"127.0.0.1:{self.game_port}"

    @property
    def endpoint(self) -> str:
        return f"http://127.0.0.1:{self.management_port}/mcp"

    #: What a credential-less loopback caller is granted. `command.execute` is
    #: the explicitly highest-privilege surface, so a family that runs a command
    #: has to say so rather than inherit it.
    loopback_scope: str = "manage"

    def start(self, extra: list[str] | None = None, *, tick_samples: bool = False) -> None:
        arguments = [
            str(self.paths.server_bin),
            "--bind",
            self.address,
            "--mcp-bind",
            f"127.0.0.1:{self.management_port}",
            "--max-players",
            str(RELEASE_PLAYERS),
            # The interaction replay breaks blocks and writes creative slots.
            "--gamemode",
            "creative",
            "--force-gamemode",
            "--difficulty",
            "peaceful",
            "--admin-dir",
            str(self.admin_dir),
            "--mcp-loopback-scope",
            self.loopback_scope,
            # The shared file's distances; and an empty server driven over
            # management keeps ticking instead of pausing after a minute.
            "--view-distance",
            "2",
            "--simulation-distance",
            "2",
            "--pause-when-empty-seconds",
            "0",
            *(["--world", str(self.world_dir)] if self.world_dir is not None else []),
            *(extra or []),
        ]
        handle = self.log_path.open("ab")
        self.process = subprocess.Popen(
            arguments,
            stdin=subprocess.DEVNULL,
            stdout=handle,
            stderr=subprocess.STDOUT,
            env={
                **os.environ,
                "RUST_LOG": "info",
                "BBB_SERVER_TICK_SAMPLES": "1" if tick_samples else "0",
            },
        )
        handle.close()

    def wait_ready(self) -> None:
        self.wait_for_event("server.ready")

    def wait_for_event(self, event: str, timeout: float = READY_TIMEOUT) -> dict[str, Any]:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for record in self.events():
                if record.get("event") == event:
                    return record
            if self.process is not None and self.process.poll() is not None:
                raise ReleaseGateError(
                    f"{self.name} exited with {self.process.returncode} before {event};"
                    f" log: {self.log_path}"
                )
            time.sleep(0.1)
        raise ReleaseGateError(f"{self.name} never logged {event}; log: {self.log_path}")

    def events(self) -> list[dict[str, Any]]:
        """Every structured log record the server has written so far.

        The server logs JSON to stderr by contract, so the log is a readable
        record of what it decided rather than prose the gate has to guess at.
        """
        if not self.log_path.is_file():
            return []
        records = []
        for line in self.log_path.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                document = json.loads(line)
            except json.JSONDecodeError:
                continue
            fields = document.get("fields")
            if isinstance(fields, dict):
                records.append(fields)
        return records

    def count_events(self, event: str) -> int:
        return sum(1 for record in self.events() if record.get("event") == event)

    def management(self, *arguments: str, expect: tuple[int, ...] = (0,)) -> Any:
        """One `bbb-admin` call against this server's management endpoint."""
        completed = subprocess.run(
            [str(self.paths.admin_bin), "--endpoint", self.endpoint, *arguments],
            capture_output=True,
            timeout=MANAGEMENT_TIMEOUT,
            check=False,
        )
        if completed.returncode not in expect or (completed.returncode and not completed.stdout):
            raise ReleaseGateError(
                f"bbb-admin {' '.join(arguments)} exited {completed.returncode}: "
                f"{completed.stderr.decode('utf-8', 'replace').strip()}"
            )
        text = completed.stdout.decode("utf-8", "replace").strip()
        if not text:
            return None
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return text

    def stop_through_management(self) -> None:
        self.management("call", "server.stop")
        self.wait_exit()

    def wait_exit(self, timeout: float = READY_TIMEOUT) -> int:
        if self.process is None:
            raise ReleaseGateError(f"{self.name} was never started")
        try:
            return self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired as error:
            self.process.kill()
            raise ReleaseGateError(f"{self.name} did not exit; log: {self.log_path}") from error

    def signal(self, number: int) -> None:
        if self.process is None:
            raise ReleaseGateError(f"{self.name} was never started")
        self.process.send_signal(number)

    def kill(self) -> None:
        if self.process is None:
            raise ReleaseGateError(f"{self.name} was never started")
        self.process.kill()

    def terminate(self) -> None:
        if self.process is None or self.process.poll() is not None:
            return
        self.process.terminate()
        try:
            self.process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.process.kill()

    def stopped_counters(self) -> dict[str, Any]:
        for record in reversed(self.events()):
            if record.get("event") == "server.stopped":
                return record
        raise ReleaseGateError(f"{self.name} never logged server.stopped")


@dataclass
class ClientRun:
    """One `bbb-client` probe session."""

    paths: Paths
    username: str
    address: str
    hold_seconds: int | None = None
    interactions: bool = False
    process: subprocess.Popen[bytes] | None = None
    report_path: Path = field(init=False)
    error_path: Path = field(init=False)

    def __post_init__(self) -> None:
        self.report_path = self.paths.child("clients", f"{self.username}.json")
        self.error_path = self.paths.child("clients", f"{self.username}.err")

    def spawn(self) -> None:
        # A name the login refuses would surface as a client crash report ten
        # steps later, blamed on whatever the harness was testing at the time.
        require(
            len(self.username) <= USERNAME_LIMIT,
            f"{self.username!r} is longer than the {USERNAME_LIMIT}-unit login limit",
        )
        game_dir = self.paths.child("clients", f"{self.username}-game")
        game_dir.mkdir(parents=True, exist_ok=True)
        arguments = [
            str(self.paths.client_bin),
            "--game-dir",
            str(game_dir),
            *(
                ["--vanilla-root", str(self.paths.vanilla_root)]
                if self.paths.vanilla_root is not None
                else []
            ),
            "--server",
            self.address,
            "--username",
            self.username,
            "--probe-server",
        ]
        if self.interactions:
            arguments.append("--probe-server-interactions")
        if self.hold_seconds is not None:
            arguments += ["--probe-hold-seconds", str(self.hold_seconds)]
        self.process = subprocess.Popen(
            arguments,
            stdin=subprocess.DEVNULL,
            stdout=self.report_path.open("wb"),
            stderr=self.error_path.open("wb"),
        )

    def wait(self, timeout: float = CLIENT_TIMEOUT) -> int:
        if self.process is None:
            raise ReleaseGateError(f"{self.username} was never spawned")
        try:
            return self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired as error:
            self.process.kill()
            raise ReleaseGateError(f"{self.username} never finished") from error

    def report(self) -> dict[str, Any]:
        text = self.report_path.read_text(encoding="utf-8", errors="replace")
        try:
            return json.loads(text)
        except json.JSONDecodeError as error:
            detail = self.error_path.read_text(encoding="utf-8", errors="replace")
            raise ReleaseGateError(
                f"{self.username} produced no report: {detail.strip()[:400]}"
            ) from error

    def failure_detail(self) -> str:
        return self.error_path.read_text(encoding="utf-8", errors="replace")


def require(condition: bool, detail: str) -> None:
    if not condition:
        raise ReleaseGateError(detail)


def assert_played(run: ClientRun, *, held: bool) -> dict[str, Any]:
    report = run.report()
    require(
        report.get("reached_state") == "Play",
        f"{run.username} reached {report.get('reached_state')!r}, not Play",
    )
    require(
        report.get("first_chunk") is not None,
        f"{run.username} never received a first chunk",
    )
    summary = report.get("first_chunk_summary") or {}
    require(
        summary.get("state") == "Decoded",
        f"{run.username} could not decode its first chunk: {summary.get('state')!r}",
    )
    require(
        report.get("unsupported_packets") == 0,
        f"{run.username} saw {report.get('unsupported_packets')} unsupported packets",
    )
    if held:
        hold = report.get("hold") or {}
        require(
            hold.get("completed") is True,
            f"{run.username} was dropped before its hold finished: {hold}",
        )
    return report


def connected_players(server: ServerProcess) -> list[str]:
    answer = server.management("call", "players")
    players = answer.get("players") if isinstance(answer, dict) else None
    if not isinstance(players, list):
        raise ReleaseGateError(f"the players readout is not a list: {answer!r}")
    return sorted(str(entry.get("name")) for entry in players)


def wait_for_player_count(server: ServerProcess, count: int, timeout: float) -> list[str]:
    """Poll the management endpoint until the server itself says who is on it.

    Counting spawned processes would only say how many clients the gate
    started. The server's own player list is the fact the release row is about.
    """
    deadline = time.monotonic() + timeout
    seen: list[str] = []
    while time.monotonic() < deadline:
        seen = connected_players(server)
        if len(seen) == count:
            return seen
        time.sleep(0.25)
    raise ReleaseGateError(f"the server listed {len(seen)} players, expected {count}: {seen}")


def run_round(
    paths: Paths,
    server: ServerProcess,
    names: list[str],
    *,
    hold_seconds: int | None,
) -> list[ClientRun]:
    runs = [
        ClientRun(paths, username=name, address=server.address, hold_seconds=hold_seconds)
        for name in names
    ]
    for run in runs:
        run.spawn()
    return runs


def mode_concurrency(paths: Paths) -> dict[str, Any]:
    """Twenty bbb clients on one server at once, then again after they leave."""
    server = ServerProcess(
        paths=paths,
        name="concurrency-server",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("concurrency", "admin"),
    )
    names = [f"load-{index:02d}" for index in range(1, RELEASE_PLAYERS + 1)]
    try:
        server.start(extra=local_bot_connection_args())
        server.wait_ready()

        # Round one holds every session open, so the twenty are on the server
        # simultaneously rather than one after another.
        first = run_round(paths, server, names, hold_seconds=HOLD_SECONDS)
        listed = wait_for_player_count(server, RELEASE_PLAYERS, timeout=HOLD_SECONDS + 30)
        require(
            listed == sorted(names),
            f"the server listed {listed}, expected {sorted(names)}",
        )
        # `server.status` carries the same roster vanilla's ServerState does, so
        # the two readouts have to agree about who is on the server.
        status = server.management("call", "server.status")
        state = status.get("status", {}) if isinstance(status, dict) else {}
        require(state.get("started") is True, f"the server does not report itself started: {state}")
        online = sorted(str(entry.get("name")) for entry in state.get("players", []))
        require(
            online == sorted(names),
            f"status lists {online}, expected {sorted(names)}",
        )
        diagnostics = server.management("call", "server.diagnostics").get("diagnostics", {})
        require(
            diagnostics.get("onlinePlayers") == RELEASE_PLAYERS
            and diagnostics.get("maxPlayers") == RELEASE_PLAYERS,
            f"diagnostics report {diagnostics.get('onlinePlayers')}/"
            f"{diagnostics.get('maxPlayers')}, expected {RELEASE_PLAYERS}/{RELEASE_PLAYERS}",
        )

        # The twenty-first is refused while the original twenty remain admitted.
        overflow = ClientRun(paths, username="load-21", address=server.address)
        overflow.spawn()
        overflow_status = overflow.wait()
        require(overflow_status != 0, "a full server accepted a twenty-first player")
        require(
            CAPACITY_REASON in overflow.failure_detail(),
            f"the refused client was not told why: {overflow.failure_detail()[:400]}",
        )
        require(
            connected_players(server) == sorted(names),
            "capacity refusal changed the admitted player roster",
        )

        for run in first:
            require(run.wait() == 0, f"{run.username} failed: {run.failure_detail()[:400]}")
            assert_played(run, held=True)

        # Every session left; the same twenty names come back on new
        # connections, which is what reconnect means for a stateless join.
        wait_for_player_count(server, 0, timeout=READY_TIMEOUT)
        second = run_round(paths, server, names, hold_seconds=None)
        for run in second:
            require(
                run.wait() == 0,
                f"{run.username} failed to reconnect: {run.failure_detail()[:400]}",
            )
            assert_played(run, held=False)

        server.stop_through_management()
        counters = server.stopped_counters()
        expected_accepted = 2 * RELEASE_PLAYERS + 1
        require(
            counters.get("accepted_connections") == expected_accepted,
            f"the server accepted {counters.get('accepted_connections')} connections, "
            f"expected {expected_accepted}",
        )
        require(
            counters.get("rejected_connections") == 0,
            f"the server rejected {counters.get('rejected_connections')} connections at accept",
        )
        require(
            counters.get("completed_connections") == expected_accepted,
            f"the server completed {counters.get('completed_connections')} of "
            f"{expected_accepted} connections",
        )
    finally:
        server.terminate()

    return {
        "players": RELEASE_PLAYERS,
        "hold_seconds": HOLD_SECONDS,
        "rounds": 2,
        "observed_simultaneously": RELEASE_PLAYERS,
        "capacity_refusals": 1,
        "capacity_reason": CAPACITY_REASON,
        "accepted_connections": 2 * RELEASE_PLAYERS + 1,
    }


def mode_restart(paths: Paths) -> dict[str, Any]:
    """A server that stops and comes back on the same directories.

    World *content* fidelity across a save is the round-trip gate's subject
    (official writes, BBB reads, official reads back). What release acceptance
    adds is the process property: the same command line, pointed at the same
    save and the same stored user lists, comes back serving with the
    administrative state it was stopped with, and takes players again.
    """
    require(
        paths.world_template is not None and paths.world_template.is_dir(),
        "the restart family needs an official 26.1 save to restart on; capture one with "
        "scripts/capture-official-world-26.1.sh or name one in "
        "BBB_SERVER_RELEASE_WORLD_TEMPLATE",
    )
    world_dir = paths.child("restart", "world")
    if world_dir.exists():
        shutil.rmtree(world_dir)
    shutil.copytree(paths.world_template, world_dir)
    admin_dir = paths.child("restart", "admin")
    admin_dir.mkdir(parents=True, exist_ok=True)
    game_port = free_port()
    management_port = free_port()
    names = [f"restart-{index:02d}" for index in range(1, 6)]

    def boot(name: str) -> ServerProcess:
        server = ServerProcess(
            paths=paths,
            name=name,
            game_port=game_port,
            management_port=management_port,
            admin_dir=admin_dir,
            world_dir=world_dir,
            # The restart family runs `/reload`, which the ledger puts behind
            # the command scope.
            loopback_scope="command",
        )
        server.start()
        server.wait_ready()
        server.wait_for_event("world.loaded")
        return server

    first = boot("restart-server-first")
    try:
        for run in run_round(paths, first, names, hold_seconds=None):
            require(run.wait() == 0, f"{run.username} failed: {run.failure_detail()[:400]}")
            assert_played(run, held=False)

        # Administrative state the restart has to carry across: one allowlist
        # entry and one operator, both of which live in files the runtime owns.
        first.management(
            "call", "allowlist.add", "--arguments-json", '{"add":[{"name":"restart-01"}]}'
        )
        first.management(
            "call",
            "operators.add",
            "--arguments-json",
            '{"add":[{"player":{"name":"restart-01"},"permissionLevel":4}]}',
        )
        before_allowlist = first.management("call", "allowlist")["allowlist"]
        require(
            [entry.get("name") for entry in before_allowlist] == ["restart-01"],
            f"the allowlist did not take the entry: {before_allowlist}",
        )

        # `/reload` runs through the one command dispatcher, the way vanilla's
        # does, and the rebuild happens off the tick.
        answer = first.management(
            "call", "command.execute", "--arguments-json", '{"command":"reload"}'
        )
        require(
            answer["command"]["result"] == 0,
            f"reload did not answer the way ReloadCommand does: {answer}",
        )
        first.wait_for_event("world.datapacks_reloaded", timeout=60.0)
        require(
            first.count_events("world.datapack_reload_failed") == 0,
            "the datapack reload failed",
        )
        # A reloaded server is still a serving server.
        after_reload = ClientRun(paths, username="restart-reload", address=first.address)
        after_reload.spawn()
        require(after_reload.wait() == 0, f"a client could not join after a reload: "
                f"{after_reload.failure_detail()[:400]}")
        assert_played(after_reload, held=False)

        # A pack that is not a pack. Vanilla's `PackDetector` takes directories
        # and `.zip` files from `<world>/datapacks` and skips what it cannot
        # open, so the reload has to survive this, report the pack by name, and
        # keep serving with the packs it could read.
        broken = world_dir / "datapacks" / BROKEN_DATAPACK
        broken.parent.mkdir(parents=True, exist_ok=True)
        broken.write_bytes(b"this file is not an archive")
        first.management("call", "command.execute", "--arguments-json", '{"command":"reload"}')
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if first.count_events("world.datapacks_reloaded") >= 2:
                break
            time.sleep(0.2)
        require(
            first.count_events("world.datapacks_reloaded") >= 2,
            "the reload carrying an unreadable pack never finished",
        )
        require(
            first.count_events("world.datapack_reload_failed") == 0,
            "one unreadable pack failed the whole reload",
        )
        notices = [
            str(record.get("message"))
            for record in first.events()
            if record.get("event") == "world.datapack_warning"
            and BROKEN_DATAPACK in str(record.get("message"))
        ]
        require(notices, "the unreadable pack was loaded or skipped without saying so")
        with_broken = ClientRun(paths, username="restart-broken", address=first.address)
        with_broken.spawn()
        require(
            with_broken.wait() == 0,
            f"a client could not join after an unreadable pack: "
            f"{with_broken.failure_detail()[:400]}",
        )
        assert_played(with_broken, held=False)

        first.management("call", "server.save")
        first.stop_through_management()
    finally:
        first.terminate()

    second = boot("restart-server-second")
    try:
        allowlist = second.management("call", "allowlist")["allowlist"]
        require(
            [entry.get("name") for entry in allowlist] == ["restart-01"],
            f"the allowlist did not survive the restart: {allowlist}",
        )
        operators = second.management("call", "operators")["operators"]
        require(
            [entry.get("player", {}).get("name") for entry in operators] == ["restart-01"],
            f"the operator list did not survive the restart: {operators}",
        )
        for run in run_round(paths, second, names, hold_seconds=None):
            require(
                run.wait() == 0,
                f"{run.username} could not rejoin after the restart: "
                f"{run.failure_detail()[:400]}",
            )
            assert_played(run, held=False)
        second.stop_through_management()
    finally:
        second.terminate()

    return {
        "players": len(names),
        "restarts": 1,
        # Named by shape rather than by path: the capture lands in a fresh
        # temporary directory on every run, and a receipt that records where
        # this machine happened to put it would differ from every other run for
        # a reason that says nothing about the server.
        "world_template": "official-26.1-capture",
        "reloaded_datapacks": True,
        "unreadable_datapack_reported": True,
        "allowlist_entries_carried": 1,
        "operator_entries_carried": 1,
    }


def raw_exchange(port: int, payload: bytes, *, read: bool = True) -> bytes:
    """Sends bytes at a socket and reads whatever comes back.

    Deliberately invalid input has to be written as bytes; nothing here decodes
    a bbb protocol, and nothing here is a second implementation of one.
    """
    answer = b""
    with socket.create_connection(("127.0.0.1", port), timeout=10) as stream:
        stream.sendall(payload)
        if read:
            stream.settimeout(5)
            try:
                while True:
                    chunk = stream.recv(4096)
                    if not chunk:
                        break
                    answer += chunk
            except (TimeoutError, OSError):
                pass
    return answer


def http_status(answer: bytes) -> int:
    head = answer.split(b"\r\n", 1)[0].split(b" ")
    return int(head[1]) if len(head) > 1 and head[1].isdigit() else 0


#: Malformed bytes at the game port, and what the server must say about each.
#: The first two are refusals the decoder makes on its own budget; the last two
#: are peers that leave mid-frame, which must cost the connection and nothing
#: else. The expected reason is a substring of the logged `reason`, so a server
#: that starts closing these for a different cause fails here rather than
#: quietly passing.
HOSTILE_WIRE_CASES: tuple[tuple[str, bytes, str], ...] = (
    # A first byte that is a plausible frame length followed by bytes that are
    # not a packet: the VarInt packet id runs off the end of its own encoding.
    ("garbage_handshake", bytes([0x20]) + b"\xff" * 0x20, "varint is too long"),
    # A frame claiming 2 GiB. The outer length prefix admits only 21 bits
    # (Varint21FrameDecoder.java:25-38 throws "length wider than 21-bit"), so
    # the refusal lands on the fourth prefix byte before any allocation.
    ("oversized_frame", b"\xff\xff\xff\xff\x07", "varint is too long"),
    # A frame that promises more than it delivers, then goes away.
    ("truncated_frame", bytes([0x40]) + b"\x00" * 4, "RemoteClosed"),
    # A peer that connects and says nothing at all.
    ("silent_peer", b"", "RemoteClosed"),
)

#: Deliberately invalid management requests, written as raw HTTP because
#: bbb-admin cannot express them, and the status each one has to be answered
#: with. Every one is answered rather than dropped: an operator has to be able
#: to tell a refusal from an endpoint that died.
HOSTILE_MANAGEMENT_CASES: tuple[tuple[str, dict[str, str], str, int, bytes], ...] = (
    (
        "oversized_body",
        {"content-type": "application/json", "content-length": str(2 * 1024 * 1024)},
        "",
        413,
        b"",
    ),
    (
        "malformed_json",
        {"content-type": "application/json"},
        "{[,]}",
        400,
        b"-32700",
    ),
    ("wrong_media_type", {"content-type": "text/plain"}, "{}", 415, b""),
    (
        "foreign_origin",
        {"content-type": "application/json", "origin": "https://elsewhere.example"},
        "{}",
        403,
        b"",
    ),
)


def management_request(headers: dict[str, str], body: str) -> bytes:
    """One raw HTTP request at the management endpoint."""
    lines = [f"POST {MANAGEMENT_PATH} HTTP/1.1", "host: 127.0.0.1", "accept: application/json"]
    lines += [f"{name}: {value}" for name, value in headers.items()]
    if "content-length" not in headers:
        lines.append(f"content-length: {len(body.encode())}")
    lines.append("connection: close")
    return ("\r\n".join(lines) + "\r\n\r\n" + body).encode()


def closed_connections(server: ServerProcess) -> list[dict[str, Any]]:
    return [
        record
        for record in server.events()
        if record.get("event") in {"connection.closed", "connection.failed"}
    ]


def mode_hostile(paths: Paths) -> dict[str, Any]:
    """Input the server must refuse without losing its footing.

    Every case here is malformed on purpose. What is under test is not whether
    the server has an opinion about each one — the decoders own that, and their
    own tests pin it packet by packet — but the release property: the refusal is
    attributable, it costs the one connection that earned it, players already on
    the server play through it, and the server is still taking joins afterwards.
    """
    connection_budget = 8
    bystanders = ["hostile-idle-a", "hostile-idle-b"]
    server = ServerProcess(
        paths=paths,
        name="hostile-server",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("hostile", "admin"),
    )
    observed: dict[str, Any] = {}
    playing: list[ClientRun] = []
    try:
        server.start(["--max-connections", str(connection_budget)])
        server.wait_ready()

        # Legitimate sessions, live for the whole run. Everything below happens
        # to a server that is in the middle of serving somebody.
        playing = run_round(paths, server, bystanders, hold_seconds=HOSTILE_HOLD_SECONDS)
        require(
            wait_for_player_count(server, len(bystanders), READY_TIMEOUT) == sorted(bystanders),
            "the bystanders never made it onto the server",
        )

        wire: dict[str, str] = {}
        for name, payload, expected in HOSTILE_WIRE_CASES:
            before = len(closed_connections(server))
            raw_exchange(server.game_port, payload)
            deadline = time.monotonic() + 15
            record: dict[str, Any] | None = None
            while time.monotonic() < deadline:
                closed = closed_connections(server)
                if len(closed) > before:
                    record = closed[-1]
                    break
                time.sleep(0.1)
            require(record is not None, f"{name} was neither served nor refused")
            reason = str(record.get("reason") or record.get("error"))
            require(
                expected in reason,
                f"{name} was closed for {reason!r}, expected {expected!r}",
            )
            wire[name] = reason
        observed["wire_refusals"] = wire

        # More connections than the budget, held at once: the excess has to be
        # refused at accept rather than queued, or a full server would look like
        # a slow one.
        flood = []
        try:
            for _ in range(connection_budget + 6):
                flood.append(socket.create_connection(("127.0.0.1", server.game_port), timeout=10))
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if server.count_events("connection.rejected") > 0:
                    break
                time.sleep(0.2)
        finally:
            for stream in flood:
                stream.close()
        rejected = [
            record for record in server.events() if record.get("event") == "connection.rejected"
        ]
        require(rejected, "a server past its connection budget accepted everything")
        require(
            all(record.get("reason") == "connection_capacity" for record in rejected),
            f"a connection was refused for an unattributed reason: {rejected}",
        )
        observed["connections_rejected"] = len(rejected)

        # The management endpoint, abused the same way.
        management: dict[str, int] = {}
        for name, headers, body, status, contains in HOSTILE_MANAGEMENT_CASES:
            answer = raw_exchange(server.management_port, management_request(headers, body))
            require(
                http_status(answer) == status,
                f"{name} was answered {http_status(answer)}, expected {status}: "
                f"{answer[:200]!r}",
            )
            require(
                contains in answer,
                f"{name} was refused without naming {contains!r}: {answer[:200]!r}",
            )
            management[name] = status
        observed["management_refusals"] = management

        # Still serving: the point of every refusal above is that it cost one
        # connection rather than the process or somebody else's session.
        require(
            server.process is not None and server.process.poll() is None,
            "the server did not survive its refusals",
        )
        require(
            connected_players(server) == sorted(bystanders),
            "a bystander was dropped by somebody else's refusal",
        )
        # The survivor runs the interaction replay, which ends by driving the
        # client's own malformed-packet and malformed-NBT refusal sessions. That
        # is the play-phase half of this family, and it is driven by the real
        # client rather than by a harness that would have to speak the protocol
        # to send a packet this wrong.
        survivor = ClientRun(
            paths, username="hostile-join", address=server.address, interactions=True
        )
        survivor.spawn()
        require(
            survivor.wait() == 0,
            f"a legitimate client could not join afterwards: {survivor.failure_detail()[:400]}",
        )
        assert_played(survivor, held=False)
        decode_failures = server.count_events("connection.play_decode_failed")
        require(
            decode_failures >= len(MALFORMED_PLAY_CASES),
            f"the server logged {decode_failures} play decode refusals, expected at least "
            f"{len(MALFORMED_PLAY_CASES)}",
        )
        observed["play_decode_refusals"] = decode_failures

        for run in playing:
            require(
                run.wait() == 0,
                f"{run.username} did not survive the run: {run.failure_detail()[:400]}",
            )
            assert_played(run, held=True)
        players = connected_players(server)
        require(players == [], f"a refused peer was left on the player list: {players}")

        server.stop_through_management()
        counters = server.stopped_counters()
        require(
            counters.get("rejected_connections") == len(rejected),
            f"the stop report counts {counters.get('rejected_connections')} refusals, "
            f"expected {len(rejected)}",
        )
    finally:
        server.terminate()
        for run in playing:
            if run.process is not None and run.process.poll() is None:
                run.process.kill()

    return {
        "connection_budget": connection_budget,
        "bystanders": len(bystanders),
        **observed,
        "survived": True,
    }


def process_footprint(server: ServerProcess) -> dict[str, int]:
    """What the server process costs the machine right now.

    Read from `/proc`, which is where a Linux release host publishes it. A
    platform without `/proc` is refused by name rather than sampled with
    something that means something else.
    """
    require(server.process is not None, f"{server.name} was never started")
    pid = server.process.pid  # type: ignore[union-attr]
    root = Path("/proc") / str(pid)
    status = root / "status"
    if not status.is_file():
        raise ReleaseGateError(
            f"the soak family samples {status}, which this platform does not have; "
            "release acceptance runs on a host whose footprint can be read"
        )
    fields: dict[str, int] = {}
    for line in status.read_text(encoding="utf-8", errors="replace").splitlines():
        name, _, value = line.partition(":")
        if name in {"VmRSS", "Threads"}:
            fields[name] = int(value.split()[0])
    return {
        "rss_kib": fields.get("VmRSS", 0),
        "threads": fields.get("Threads", 0),
        "open_files": len(list((root / "fd").iterdir())),
    }


def tick_health(server: ServerProcess) -> list[dict[str, Any]]:
    return [record for record in server.events() if record.get("event") == "world.tick_health"]


def mode_soak(paths: Paths) -> dict[str, Any]:
    """The same twenty clients, round after round, watching what does not settle.

    A leak is not visible in one round; it is visible in the difference between
    rounds once the process has warmed up. So the first round is thrown away and
    the rest are compared against each other, against thresholds far enough
    above the noise band that a passing run means the footprint held rather than
    that the machine was quiet.
    """
    server = ServerProcess(
        paths=paths,
        name="soak-server",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("soak", "admin"),
    )
    rounds: list[dict[str, Any]] = []
    try:
        server.start(extra=local_bot_connection_args())
        server.wait_ready()
        for index in range(1, SOAK_ROUNDS + 1):
            names = [f"soak-{index}-{seat:02d}" for seat in range(1, RELEASE_PLAYERS + 1)]
            runs = run_round(paths, server, names, hold_seconds=HOLD_SECONDS)
            listed = wait_for_player_count(server, RELEASE_PLAYERS, timeout=HOLD_SECONDS + 30)
            require(
                listed == sorted(names),
                f"round {index} listed {listed}, expected {sorted(names)}",
            )
            for run in runs:
                require(
                    run.wait() == 0,
                    f"round {index}: {run.username} did not finish: "
                    f"{run.failure_detail()[:400]}",
                )
                assert_played(run, held=True)
            # The footprint is sampled with the server empty again, so what is
            # compared across rounds is what the round did not give back.
            wait_for_player_count(server, 0, timeout=60)
            require(
                connected_players(server) == [],
                f"round {index} left players on the server",
            )
            rounds.append({"round": index, **process_footprint(server)})

        # Vanilla's own two bounds on a tick loop: a tick that costs more than
        # its own period cannot keep up (`MinecraftServer` runs at 50ms), and
        # `OVERLOADED_THRESHOLD_NANOS` (:199) is how far behind schedule vanilla
        # tolerates before it says so. Both are orders of magnitude above the
        # microseconds an idle window samples, so neither fires on noise.
        samples = tick_health(server)
        require(
            len(samples) >= SOAK_ROUNDS,
            f"the server published {len(samples)} tick windows over {SOAK_ROUNDS} rounds",
        )
        worst_tick = max(int(sample.get("mean_tick_micros", 0)) for sample in samples)
        worst_behind = max(int(sample.get("max_behind_micros", 0)) for sample in samples)
        worst_queue = max(int(sample.get("max_queue_depth", 0)) for sample in samples)
        require(
            worst_tick < TICK_PERIOD_MICROS,
            f"a tick window averaged {worst_tick}us, which is past the {TICK_PERIOD_MICROS}us "
            "period it has to run in",
        )
        require(
            worst_behind < VANILLA_OVERLOADED_MICROS,
            f"the loop fell {worst_behind}us behind schedule, past vanilla's "
            f"{VANILLA_OVERLOADED_MICROS}us overload threshold",
        )

        # The first round is the warm-up: it is the one that grows the
        # allocator, the thread pool and the chunk cache. What has to hold is
        # every round after it.
        settled = rounds[1:]
        growth = {
            metric: max(sample[metric] for sample in settled)
            - min(sample[metric] for sample in settled)
            for metric in SOAK_LIMITS
        }
        for metric, limit in SOAK_LIMITS.items():
            require(
                growth[metric] <= limit,
                f"{metric} moved by {growth[metric]} across {len(settled)} settled rounds, "
                f"past the {limit} this gate allows: {settled}",
            )

        server.stop_through_management()
        counters = server.stopped_counters()
        require(
            counters.get("accepted_connections", 0) >= SOAK_ROUNDS * RELEASE_PLAYERS,
            f"the stop report counts {counters.get('accepted_connections')} connections over "
            f"{SOAK_ROUNDS} rounds of {RELEASE_PLAYERS}",
        )
    finally:
        server.terminate()

    return {
        "rounds": rounds,
        "players_per_round": RELEASE_PLAYERS,
        "tick_windows": len(samples),
        "worst_mean_tick_micros": worst_tick,
        "worst_behind_micros": worst_behind,
        "worst_queue_depth": worst_queue,
        "settled_growth": growth,
    }


#: The chunk-load family's forced area: `ForceLoadCommand`'s 256-chunk cap
#: (ForceLoadCommand.java:141), far enough from spawn that none of it is already
#: loaded for the players.
FORCED_CHUNK_MIN = 100
FORCED_CHUNK_SIDE = 16
CHUNK_LOAD_HOLD_SECONDS = 30
CHUNK_LOAD_TIMEOUT = 600

#: The chunk-load family's two tick bounds: no tick reaches vanilla's overload
#: threshold, and 99% of them fit the 50 ms period.
CHUNK_LOAD_MAX_TICK_MICROS = VANILLA_OVERLOADED_MICROS
CHUNK_LOAD_P99_MICROS = TICK_PERIOD_MICROS

#: Lower bucket edges of the server's `tick_histogram_millis` field
#: (`world_thread.rs` `TICK_HISTOGRAM_MILLIS`); the first bucket starts at 0.
TICK_HISTOGRAM_MILLIS = (5, 10, 20, 30, 40, 50, 100, 250, 500, 1000)


def histogram_percentile(counts: list[int], fraction: float) -> int:
    """The upper edge, in ms, of the bucket holding the given percentile."""
    total = sum(counts)
    rank = total * fraction
    seen = 0
    for bucket, count in enumerate(counts):
        seen += count
        if seen >= rank:
            return TICK_HISTOGRAM_MILLIS[bucket] if bucket < len(TICK_HISTOGRAM_MILLIS) else -1
    return -1


def command_at_tick(server: ServerProcess, command: str) -> tuple[dict[str, Any], int]:
    answer = server.management(
        "--raw", "call", "command.execute", "--arguments-json", json.dumps({"command": command}),
        expect=(0, 1),
    )
    if not answer.get("isError"):
        return answer["structuredContent"]["command"], int(answer["_meta"]["bbb/tick"])
    deferred = answer.get("_meta", {}).get("bbb/deferred")
    require(deferred is not None, f"{command}: {answer!r}")
    # A slow world tick finishes the accepted operation after the inline MCP deadline.
    deadline = time.monotonic() + CHUNK_LOAD_TIMEOUT
    while True:
        record = server.management("operation", str(deferred["operationId"]))
        if record["state"] == "applied":
            outcome = record["outcome"]
            require("result" in outcome, f"{command}: {outcome!r}")
            return outcome["result"]["command"], int(outcome["tick"])
        require(time.monotonic() < deadline, f"{command} is still pending: {record!r}")
        time.sleep(0.1)


def command_result(server: ServerProcess, command: str) -> dict[str, Any]:
    return command_at_tick(server, command)[0]


def burst_tick_health(server: ServerProcess, start_tick: int, end_tick: int) -> dict[str, Any]:
    # The closing management answer is sent before its tick's sample is logged.
    deadline = time.monotonic() + READY_TIMEOUT
    while True:
        samples = [
            record
            for record in server.events()
            if record.get("event") == "world.tick_sample"
            and start_tick < int(record["tick"]) <= end_tick
        ]
        if samples and int(samples[-1]["tick"]) == end_tick:
            break
        require(time.monotonic() < deadline, "the burst's closing tick sample was never logged")
        time.sleep(0.05)
    require(
        [int(sample["tick"]) for sample in samples] == list(range(start_tick + 1, end_tick + 1)),
        "the burst tick samples are incomplete or duplicated",
    )
    return summarize_burst(samples, start_tick)


def summarize_burst(samples: list[dict[str, Any]], start_tick: int) -> dict[str, Any]:
    durations = sorted(int(sample["tick_micros"]) for sample in samples)
    counts = [0] * (len(TICK_HISTOGRAM_MILLIS) + 1)
    for duration in durations:
        bucket = sum(duration >= edge * 1000 for edge in TICK_HISTOGRAM_MILLIS)
        counts[bucket] += 1
    ticks = len(durations)
    over_period = sum(duration >= CHUNK_LOAD_P99_MICROS for duration in durations)
    return {
        "start_tick_exclusive": start_tick,
        "end_tick_inclusive": samples[-1]["tick"],
        "peak_connections": max(sample["connections"] for sample in samples),
        "ticks": ticks,
        "ticks_over_period": over_period,
        "max_tick_micros": durations[-1],
        "p99_tick_micros": durations[(ticks * 99 + 99) // 100 - 1],
        "p99_bucket_millis": histogram_percentile(counts, 0.99),
        "tick_histogram_millis": counts,
    }


def check_burst_health(health: dict[str, Any]) -> None:
    peak, p99 = health["max_tick_micros"], health["p99_tick_micros"]
    require(peak <= CHUNK_LOAD_MAX_TICK_MICROS, f"a burst tick took {peak}us")
    require(p99 < CHUNK_LOAD_P99_MICROS, f"burst p99 took {p99}us")


def install_chunk_check(
    paths: Paths, world_dir: Path, chunks: list[tuple[int, int]], players: tuple[str, ...] = (),
) -> None:
    version = json.loads((paths.vanilla_root / "sources/26.1/version.json").read_text())
    pack_version = version["pack_version"]
    pack_format = [pack_version["data_major"], pack_version["data_minor"]]
    pack = world_dir / "datapacks/bbb-chunk-check"
    function = pack / "data/bbb/function/chunks_loaded.mcfunction"
    function.parent.mkdir(parents=True, exist_ok=True)
    (pack / "pack.mcmeta").write_text(json.dumps({"pack": {
        "description": "BBB release chunk readiness check",
        "min_format": pack_format,
        "max_format": pack_format,
    }}))
    # The management command budget is 256 characters; functions have their own
    # larger budget and evaluate every condition in one tick. Each line stays
    # shallow so the check does not depend on parser recursion depth.
    function.write_text(
        "scoreboard players set #ready bbb_chunk_check 0\n"
        + "".join(f"execute unless loaded {x * 16} 0 {z * 16} run return 0\n" for x, z in chunks)
        + "".join(f"execute unless entity {selector} run return 0\n" for selector in players)
        + "scoreboard players set #ready bbb_chunk_check 1\n"
    )


def loaded_area_tick(server: ServerProcess) -> int | None:
    _, tick = command_at_tick(server, "function bbb:chunks_loaded")
    score = command_result(server, "scoreboard players get #ready bbb_chunk_check")
    return tick if score["result"] == 1 else None


def mode_chunk_load(paths: Paths) -> dict[str, Any]:
    """Twenty players joining while 256 forced chunks generate.

    Chunk generation is the one load a server cannot pace by refusing it: the
    players and the ticket ask for terrain that does not exist yet. The tick
    loop has to keep running while it is produced, so the property is read from
    the server's own per-tick samples over the whole burst.
    """
    world_dir = paths.child("chunk_load", "world")
    if world_dir.exists():
        shutil.rmtree(world_dir)
    world_dir.mkdir(parents=True)
    server = ServerProcess(
        paths=paths,
        name="chunk-load-server",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("chunk_load", "admin"),
        world_dir=world_dir,
        loopback_scope="command",
    )
    names = [f"chunks-{index:02d}" for index in range(1, RELEASE_PLAYERS + 1)]
    low = FORCED_CHUNK_MIN * 16
    high = (FORCED_CHUNK_MIN + FORCED_CHUNK_SIDE) * 16 - 1
    forced = [
        (x, z)
        for x in range(FORCED_CHUNK_MIN, FORCED_CHUNK_MIN + FORCED_CHUNK_SIDE)
        for z in range(FORCED_CHUNK_MIN, FORCED_CHUNK_MIN + FORCED_CHUNK_SIDE)
    ]
    # Place five by four nonoverlapping view areas away from both spawn and the forced square.
    destinations = [
        (-120 + seat % 5 * 12, -120 + seat // 5 * 12) for seat in range(len(names))
    ] if paths.disperse_players else []
    positioned = tuple(
        f"@a[name={names[seat]},x={x * 16},y=-64,z={z * 16},dx=15,dy=383,dz=15]"
        for seat, (x, z) in enumerate(destinations)
    )
    install_chunk_check(paths, world_dir, forced + destinations, positioned)
    spread = world_dir / "datapacks/bbb-chunk-check/data/bbb/function/spread.mcfunction"
    if destinations:
        spread.write_text(
            "scoreboard players set #arrived bbb_chunk_check 0\n" + "".join(
                f"tp {name} {x * 16 + 8} 100 {z * 16 + 8}\n"
                f"execute if entity {selector} run "
                "scoreboard players add #arrived bbb_chunk_check 1\n"
                for name, (x, z), selector in zip(names, destinations, positioned, strict=True)
            )
        )
    progress = spread.with_name("chunk_progress.mcfunction")
    progress.write_text(
        "".join(
            f"scoreboard players set {holder} bbb_chunk_check 0\n"
            + "".join(
                f"execute if loaded {x * 16} 0 {z * 16} run "
                f"scoreboard players add {holder} bbb_chunk_check 1\n"
                for x, z in chunks
            )
            for holder, chunks in [("#forced", forced), ("#centers", destinations)]
        )
    )
    result: dict[str, Any] = {
        "forced_chunks": len(forced),
        "dispersed_centers": len(destinations),
        "seed": "bbb-chunk-load",
        "area_ready": False,
    }
    receipt = paths.child("chunk_load", "burst.json")
    start_tick = None
    try:
        server.start(
            extra=["--level-seed", result["seed"], *local_bot_connection_args()],
            tick_samples=True,
        )
        server.wait_ready()
        server.wait_for_event("world.loaded")
        command_result(server, "scoreboard objectives add bbb_chunk_check dummy")
        start_tick = int(server.management("call", "server.diagnostics")["diagnostics"]["tick"])
        burst_started = time.monotonic()

        hold_seconds = CHUNK_LOAD_TIMEOUT if destinations else CHUNK_LOAD_HOLD_SECONDS
        runs = run_round(paths, server, names, hold_seconds=hold_seconds)
        added = command_result(server, f"forceload add {low} {low} {high} {high}")
        require(
            added.get("result") == len(forced),
            f"forceload answered {added}, expected {len(forced)} changed chunks",
        )
        if destinations:
            listed = wait_for_player_count(server, RELEASE_PLAYERS, timeout=hold_seconds)
            require(listed == sorted(names), f"the dispersed cohort is incomplete: {listed}")
            result["players_at_spread"] = len(listed)
            command_result(server, "function bbb:spread")
            arrived = command_result(server, "scoreboard players get #arrived bbb_chunk_check")
            result["players_at_destinations"] = arrived["result"]
            require(
                arrived["result"] == len(names), f"dispersed players at destinations: {arrived}"
            )
        # Each checked column must be FULL, have an entity-ticking loading ticket,
        # and have finished loading its entities (ExecuteCommand.isChunkLoaded).
        deadline = burst_started + CHUNK_LOAD_TIMEOUT
        while True:
            end_tick = loaded_area_tick(server)
            if end_tick is not None:
                break
            if destinations:
                departed = [run.username for run in runs if run.process.poll() is not None]
                require(not departed, f"players left before all chunks were ready: {departed}")
            require(time.monotonic() < deadline, "the forced area was never fully loaded")
        settled_seconds = round(time.monotonic() - burst_started, 1)
        # Close on the function's tick, before the score read and remaining client holds.
        health = burst_tick_health(server, start_tick, end_tick)
        result.update(
            health,
            settled_seconds=settled_seconds,
            area_ready=True,
            forced_chunks_ready=len(forced),
            dispersed_centers_ready=len(destinations),
            forced_chunks_per_second=round(len(forced) / settled_seconds, 2),
            players_at_completion=len(connected_players(server)),
        )
        if destinations:
            require(
                result["players_at_completion"] == RELEASE_PLAYERS,
                f"players remaining when all chunks were ready: {result['players_at_completion']}",
            )
        receipt.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

        client_timeout = hold_seconds + CLIENT_TIMEOUT if destinations else CLIENT_TIMEOUT
        first_terrain_micros = []
        received = []
        for run in runs:
            require(
                run.wait(timeout=client_timeout) == 0,
                f"{run.username} failed: {run.failure_detail()[:400]}",
            )
            report = assert_played(run, held=True)
            latency = report["hold"].get("join_to_first_terrain_micros")
            require(latency is not None, f"{run.username} did not record first terrain latency")
            first_terrain_micros.append(latency)
            received.append(report["world_counters"]["chunks_received"])
        chunks_received = min(received)
        require(chunks_received > 1, f"a player received only {chunks_received} chunks")

        result.update(
            players=RELEASE_PLAYERS,
            min_chunks_received=chunks_received,
            join_to_first_terrain_micros={
                "min": min(first_terrain_micros),
                "median": median(first_terrain_micros),
                "max": max(first_terrain_micros),
            },
        )
        check_burst_health(health)
        server.stop_through_management()
    except (ReleaseGateError, subprocess.TimeoutExpired) as failure:
        result["failure"] = str(failure)
        if start_tick is not None and "ticks" not in result:
            elapsed = time.monotonic() - burst_started
            result["elapsed_seconds"] = round(elapsed, 2)
            if server.process.poll() is None:
                try:
                    command_result(server, "function bbb:chunk_progress")
                    checkpoint_seconds = time.monotonic() - burst_started
                    for holder, key in [
                        ("#forced", "forced_chunks_ready"), ("#centers", "dispersed_centers_ready")
                    ]:
                        score = command_result(
                            server, f"scoreboard players get {holder} bbb_chunk_check"
                        )
                        result[key] = score["result"]
                    result["progress_seconds"] = round(checkpoint_seconds, 2)
                    result["forced_chunks_per_second"] = round(
                        result["forced_chunks_ready"] / checkpoint_seconds, 2
                    )
                    result["players_at_failure"] = len(connected_players(server))
                except (ReleaseGateError, subprocess.TimeoutExpired) as error:
                    result["progress_read_error"] = str(error)
            samples = [
                row for row in server.events()
                if row.get("event") == "world.tick_sample" and int(row["tick"]) > start_tick
            ]
            if samples:
                result.update(summarize_burst(samples, start_tick))
        raise
    finally:
        server.terminate()
        receipt.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

    return result


def mode_signal(paths: Paths) -> dict[str, Any]:
    """The two stop signals, and what the process leaves behind on each.

    A supervisor's whole contract with this process is: send `SIGTERM`, get a
    saved world and a record of what happened. Both signals are exercised on a
    real save, because a stop that does not write is only visible in the save
    it did not update.
    """
    require(
        os.name != "nt",
        "the signal family requires POSIX SIGTERM/SIGINT delivery and is unsupported on Windows",
    )
    require(
        paths.world_template is not None and paths.world_template.is_dir(),
        "the signal family stops a server on a real save; capture one with "
        "scripts/capture-official-world-26.1.sh or name one in "
        "BBB_SERVER_RELEASE_WORLD_TEMPLATE",
    )
    stops: dict[str, Any] = {}
    for name, number in (("SIGTERM", signal.SIGTERM), ("SIGINT", signal.SIGINT)):
        world_dir = paths.child("signal", name.lower(), "world")
        if world_dir.exists():
            shutil.rmtree(world_dir)
        shutil.copytree(paths.world_template, world_dir)
        level = world_dir / "level.dat"
        before = level.stat().st_mtime_ns
        server = ServerProcess(
            paths=paths,
            name=f"signal-server-{name.lower()}",
            game_port=free_port(),
            management_port=free_port(),
            admin_dir=paths.child("signal", name.lower(), "admin"),
            world_dir=world_dir,
        )
        try:
            server.start()
            server.wait_ready()
            server.wait_for_event("world.loaded")
            # A player joins first, so the save the stop writes is a save with
            # something in it to write.
            run = ClientRun(paths, username=f"signal-{name[3:].lower()}", address=server.address)
            run.spawn()
            require(run.wait() == 0, f"{run.username} failed: {run.failure_detail()[:400]}")
            assert_played(run, held=False)

            server.signal(number)
            status = server.wait_exit()
            require(
                status == 0,
                f"a {name} stop exited {status}; a supervisor reads that as a failure",
            )
            signalled = [
                record for record in server.events() if record.get("event") == "server.signalled"
            ]
            require(
                [record.get("signal") for record in signalled] == [name],
                f"the log records {signalled} for a {name} stop",
            )
            saved = [record for record in server.events() if record.get("event") == "world.saved"]
            require(saved, f"a {name} stop wrote no save")
            require(
                level.stat().st_mtime_ns > before,
                f"a {name} stop left {level.name} untouched",
            )
            counters = server.stopped_counters()
            require(
                counters.get("accepted_connections", 0) >= 1
                and counters.get("completed_connections", 0) >= 1,
                f"the {name} stop report counts nothing it served: {counters}",
            )
            stops[name] = {
                "exit_status": status,
                "saves": len(saved),
                "accepted_connections": counters.get("accepted_connections"),
            }
        finally:
            server.terminate()

    # A failure has to be as attributable as a stop. A bind that cannot be
    # taken is the one failure a supervisor can produce on purpose, and it has
    # to arrive as a record and a non-zero status rather than as a bare line.
    holder = ServerProcess(
        paths=paths,
        name="signal-server-holder",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("signal", "holder", "admin"),
    )
    try:
        holder.start()
        holder.wait_ready()
        clash = ServerProcess(
            paths=paths,
            name="signal-server-clash",
            game_port=holder.game_port,
            management_port=free_port(),
            admin_dir=paths.child("signal", "clash", "admin"),
        )
        clash.start()
        status = clash.wait_exit()
        require(status != 0, "a server that could not bind its port exited cleanly")
        failures = [record for record in clash.events() if record.get("event") == "server.failed"]
        require(
            len(failures) == 1 and failures[0].get("error"),
            f"the failed start left no attributable record: {clash.events()}",
        )
        stops["failed_start"] = {
            "exit_status": status,
            "error": str(failures[0].get("error"))[:120],
        }
    finally:
        holder.terminate()

    return stops


class OfficialServer:
    """Optional official-server process for differential debugging tools."""

    def __init__(self, paths: Paths, world_dir: Path, port: int) -> None:
        self.paths = paths
        self.root = paths.child("creation", "official")
        self.port = port
        self.world_dir = world_dir
        self.log_path = paths.child("creation", "official.log")
        self.process: subprocess.Popen[bytes] | None = None

    def start(self) -> None:
        require(
            self.paths.java_bin is not None and self.paths.server_jar is not None,
            "official differential debugging needs the Java 26.1 runtime and server jar",
        )
        self.root.mkdir(parents=True, exist_ok=True)
        world = self.root / "world"
        if world.exists():
            shutil.rmtree(world)
        shutil.copytree(self.world_dir, world)
        # The session lock belongs to the process that held it; copying one
        # forward would have the official server refuse a save nobody is using.
        (world / "session.lock").unlink(missing_ok=True)
        (self.root / "eula.txt").write_text("eula=true\n", encoding="utf-8")
        # The same settings every other script gives the official server for
        # this replay, plus the two only this run knows.
        shared = (REPO_ROOT / OFFICIAL_REPLAY_PROPERTIES).read_text(encoding="utf-8")
        (self.root / "server.properties").write_text(
            shared
            + "\n".join(
                [
                    f"server-port={self.port}",
                    "level-name=world",
                    "motd=bbb created world 26.1",
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        handle = self.log_path.open("ab")
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
            stdin=subprocess.DEVNULL,
            stdout=handle,
            stderr=subprocess.STDOUT,
        )
        handle.close()

    def wait_ready(self, timeout: float = READY_TIMEOUT) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.log_path.is_file():
                text = self.log_path.read_text(encoding="utf-8", errors="replace")
                if "Done (" in text:
                    return
            if self.process is not None and self.process.poll() is not None:
                raise ReleaseGateError(
                    f"the official server exited before it was ready; log: {self.log_path}"
                )
            time.sleep(0.25)
        raise ReleaseGateError(f"the official server never started; log: {self.log_path}")

    def complaints(self) -> list[str]:
        text = self.log_path.read_text(encoding="utf-8", errors="replace")
        return [
            line
            for line in text.splitlines()
            if any(word in line for word in ("ERROR", "Exception", "Failed to load"))
        ]

    def terminate(self) -> None:
        if self.process is None or self.process.poll() is not None:
            return
        self.process.terminate()
        try:
            self.process.wait(timeout=60)
        except subprocess.TimeoutExpired:
            self.process.kill()


def list_world_files(world_dir: Path) -> list[str]:
    return sorted(
        path.relative_to(world_dir).as_posix() for path in world_dir.rglob("*") if path.is_file()
    )


def hard_kill_server(server: ServerProcess) -> int:
    # Terminate without running any shutdown handler so no save can be written on exit.
    sigkill = getattr(signal, "SIGKILL", None)
    if sigkill is not None:
        server.signal(sigkill)
        status = server.wait_exit()
        require(
            status == -sigkill,
            f"the killed server exited {status} rather than on the signal",
        )
        return status
    server.kill()
    status = server.wait_exit()
    require(
        status != 0,
        f"the killed server exited {status} rather than failing on hard kill",
    )
    return status


def mode_creation(paths: Paths) -> dict[str, Any]:
    """A directory with nothing in it, and the save this server makes of it.

    An empty directory starts and serves, its saved block state survives a BBB
    restart, and an unreadable existing level is refused rather than replaced.
    """
    world_dir = paths.child("creation", "world")
    if world_dir.exists():
        shutil.rmtree(world_dir)
    world_dir.mkdir(parents=True)
    server = ServerProcess(
        paths=paths,
        name="creation-server",
        loopback_scope="command",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("creation", "admin"),
        world_dir=world_dir,
    )
    observed: dict[str, Any] = {}
    try:
        # Superflat: the interaction replay needs a floor to break and place against.
        server.start(["--level-type", "flat"])
        server.wait_ready()
        server.wait_for_event("world.created")
        server.wait_for_event("world.loaded")

        # The interaction replay is what makes this more than a handshake: it
        # looks for a solid block to break and a face to place against, so a
        # created world whose floor is not where the save says would fail here
        # rather than pass quietly.
        played = ClientRun(
            paths, username="creation-bbb", address=server.address, interactions=True
        )
        played.spawn()
        require(
            played.wait() == 0,
            f"a client could not join the created world: {played.failure_detail()[:400]}",
        )
        assert_played(played, held=False)

        # Save while somebody is still on the server. Every other save in this
        # gate happens after the last client has gone, which leaves the player
        # patches empty and hides anything that only goes wrong for a player the
        # loaded save has never seen -- a first-time player is exactly that.
        resident = ClientRun(
            paths,
            username="creation-online",
            address=server.address,
            hold_seconds=CREATION_RESIDENT_HOLD_SECONDS,
        )
        resident.spawn()
        require(
            wait_for_player_count(server, 1, READY_TIMEOUT) == ["creation-online"],
            "the resident client never made it onto the server",
        )
        require(
            command_result(server, "setblock 8 100 8 minecraft:gold_block").get("result") == 1,
            "the created world did not accept its persistent block marker",
        )
        saved_before = server.count_events("world.saved")
        # `ServerStateService.save` submits to the server thread and answers
        # before it runs, the way vanilla's does; the save itself lands at the
        # next tick barrier. Counting straight after the call reads the answer,
        # not the save.
        server.management("call", "server.save")
        deadline = time.monotonic() + READY_TIMEOUT
        while time.monotonic() < deadline:
            if server.count_events("world.saved") > saved_before:
                break
            time.sleep(0.1)
        require(
            server.count_events("world.saved") > saved_before,
            "an occupied server did not save on request",
        )
        require(
            resident.wait() == 0,
            f"the resident client failed: {resident.failure_detail()[:400]}",
        )
        assert_played(resident, held=True)
        observed["saved_while_occupied"] = True

        server.stop_through_management()
        require(
            server.count_events("world.saved") >= 1,
            "the created world was never saved",
        )
        # A save that fails is still a save that was attempted, so counting
        # successes alone cannot tell the two apart.
        require(
            server.count_events("world.save_failed") == 0,
            f"the server reported {server.count_events('world.save_failed')} failed saves",
        )
    finally:
        server.terminate()

    written = list_world_files(world_dir)
    for required in CREATED_SAVE_FILES:
        require(required in written, f"the created save has no {required}: {written}")
    observed["created_files"] = written

    revived = ServerProcess(
        paths=paths,
        name="creation-server-revived",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=server.admin_dir,
        world_dir=world_dir,
        loopback_scope="command",
    )
    try:
        revived.start()
        revived.wait_ready()
        revived.wait_for_event("world.loaded")
        run = ClientRun(
            paths, username="creation-revived", address=revived.address, interactions=True
        )
        run.spawn()
        require(
            run.wait() == 0,
            f"a client could not play the created save after restart: {run.failure_detail()[:400]}",
        )
        assert_played(run, held=False)
        require(
            command_result(revived, "execute if block 8 100 8 minecraft:gold_block").get("result")
            == 1,
            "the created world's block marker did not survive save and restart",
        )
        observed["restored_block"] = "minecraft:gold_block"
        observed["restarted_playable"] = True
        revived.stop_through_management()
    finally:
        revived.terminate()

    # A level that is present and unreadable is not a directory to create in.
    unusable = paths.child("creation", "unusable")
    if unusable.exists():
        shutil.rmtree(unusable)
    unusable.mkdir(parents=True)
    (unusable / "level.dat").write_bytes(b"this is not a level")
    refused = ServerProcess(
        paths=paths,
        name="creation-server-unusable",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("creation", "unusable-admin"),
        world_dir=unusable,
    )
    try:
        refused.start()
        status = refused.wait_exit()
        require(status != 0, "a server opened a save whose level cannot be read")
        failures = [
            record for record in refused.events() if record.get("event") == "world.load_failed"
        ]
        require(
            len(failures) == 1 and failures[0].get("error"),
            f"the refused save left no attributable record: {refused.events()}",
        )
        require(
            not (unusable / "level.dat_old").exists()
            and (unusable / "level.dat").read_bytes() == b"this is not a level",
            "a save that could not be read was replaced instead of refused",
        )
        # The message names the file it could not read, and that path is a
        # temporary directory this run invented. The receipt keeps what the
        # server decided, not where this machine happened to put it.
        observed["refused_unusable_level"] = re.sub(
            r"/\S+", "<path>", str(failures[0].get("error"))
        )[:160]
    finally:
        refused.terminate()

    return observed


def mode_autosave(paths: Paths) -> dict[str, Any]:
    """The save nobody asked for, and the restart that proves it happened.

    Every other save in this gate is something an operator or a shutdown did.
    An unattended server is the case where nobody is there to ask: vanilla
    `MinecraftServer.tickServer` counts `ticksUntilAutosave` down and calls
    `autoSave` at zero (MinecraftServer.java:983-986), and that is the only
    writer that runs with no client connected and no signal sent.

    `SIGKILL` is what makes the receipt more than a log line. It cannot be
    caught, so nothing on the shutdown path runs; a world that comes back on a
    later game time than it started on can only have got there through the
    unattended save.
    """
    world_dir = paths.child("autosave", "world")
    if world_dir.exists():
        shutil.rmtree(world_dir)
    world_dir.mkdir(parents=True)
    admin_dir = paths.child("autosave", "admin")
    game_port = free_port()
    management_port = free_port()
    observed: dict[str, Any] = {}

    server = ServerProcess(
        paths=paths,
        name="autosave-server",
        game_port=game_port,
        management_port=management_port,
        admin_dir=admin_dir,
        world_dir=world_dir,
    )
    try:
        server.start(["--level-type", "flat"])
        server.wait_ready()
        loaded = server.wait_for_event("world.loaded")
        started_at = int(loaded.get("game_time", -1))
        require(
            started_at >= 0,
            f"the load receipt does not name the game time it resumed from: {loaded}",
        )

        # Nothing joins and nothing is asked for. The only thing that happens
        # between here and the receipt is the server ticking.
        saved = wait_for_autosave_receipt(server)
        observed["trigger"] = saved.get("trigger")
        require(
            server.count_events("world.save_failed") == 0,
            f"the unattended save failed: {server.events()[-3:]}",
        )
        require(
            server.count_events("admin.save_skipped") == 0,
            "the unattended save reached no storage",
        )
        # The only two other triggers are an operator's and the shutdown flush,
        # and this run performed neither, so no receipt may name them.
        others = sorted(
            {
                str(record.get("trigger"))
                for record in server.events()
                if record.get("event") == "world.saved"
            }
        )
        require(
            others == ["autosave"],
            f"an unattended run wrote saves triggered by {others}",
        )

        # Hard kill: no handler runs, so no save can be written on the way out.
        hard_kill_server(server)
        require(
            server.count_events("server.stopped") == 0,
            "a killed server ran its shutdown path",
        )
        observed["killed_without_shutdown"] = True
    finally:
        server.terminate()

    revived = ServerProcess(
        paths=paths,
        name="autosave-server-revived",
        game_port=game_port,
        management_port=free_port(),
        admin_dir=admin_dir,
        world_dir=world_dir,
    )
    try:
        revived.start()
        revived.wait_ready()
        reloaded = revived.wait_for_event("world.loaded")
        resumed_at = int(reloaded.get("game_time", -1))
        require(
            resumed_at >= started_at + AUTOSAVE_INTERVAL_TICKS,
            f"the world came back at game time {resumed_at} after starting at "
            f"{started_at}; the unattended save wrote nothing a kill could not undo",
        )
        observed["started_at"] = started_at
        observed["resumed_at"] = resumed_at
        # And it is still a server: recovering a save is worth nothing if the
        # world it recovered cannot be played.
        run = ClientRun(paths, username="autosave-revived", address=revived.address)
        run.spawn()
        require(
            run.wait() == 0,
            f"a client could not join the recovered world: {run.failure_detail()[:400]}",
        )
        assert_played(run, held=False)
        revived.stop_through_management()
    finally:
        revived.terminate()

    return observed


def wait_for_autosave_receipt(server: ServerProcess) -> dict[str, Any]:
    """The first `world.saved` record whose trigger is the unattended interval."""
    deadline = time.monotonic() + AUTOSAVE_RECEIPT_TIMEOUT
    while time.monotonic() < deadline:
        for record in server.events():
            if record.get("event") == "world.saved" and record.get("trigger") == "autosave":
                return record
        if server.process is not None and server.process.poll() is not None:
            raise ReleaseGateError(
                f"{server.name} exited with {server.process.returncode} before it autosaved;"
                f" log: {server.log_path}"
            )
        time.sleep(0.5)
    raise ReleaseGateError(
        f"{server.name} never wrote an unattended save in "
        f"{AUTOSAVE_RECEIPT_TIMEOUT:.0f}s; log: {server.log_path}"
    )


MODES = {
    "concurrency": mode_concurrency,
    "chunk_load": mode_chunk_load,
    "restart": mode_restart,
    "hostile": mode_hostile,
    "soak": mode_soak,
    "autosave": mode_autosave,
    "signal": mode_signal,
    "creation": mode_creation,
}


def _resolve_release_bin(explicit: Path | None, env_key: str, stem: str) -> Path:
    if explicit is not None:
        binary = explicit
    elif os.environ.get(env_key):
        binary = Path(os.environ[env_key])
    else:
        target = shared_default_target_dir()
        profile = "release" if stem == "bbb-server" else "debug"
        binary = target / profile / (f"{stem}.exe" if os.name == "nt" else stem)
    if not binary.is_file():
        raise SystemExit(f"server release gate: {binary} is unavailable (build it first)")
    return binary.resolve()


def _resolve_optional_vanilla(explicit: Path | None) -> Path | None:
    candidates: list[Path] = []
    if explicit is not None:
        candidates.append(explicit)
    if os.environ.get("BBB_VANILLA_ROOT"):
        candidates.append(Path(os.environ["BBB_VANILLA_ROOT"]))
    try:
        candidates.append(bbb_default_vanilla_root())
    except VanillaError:
        pass
    for cand in candidates:
        if (cand / "sources" / "26.1").is_dir() or (cand / "state" / "26.1.json").is_file():
            return cand.resolve()
    return None


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "families",
        nargs="*",
        choices=sorted(MODES),
        help="optional release family names; default is every family",
    )
    parser.add_argument(
        "--mode",
        action="append",
        choices=sorted(MODES),
        help="release family to run; repeatable, default is every family",
    )
    parser.add_argument("--artifact-dir", type=Path, default=None)
    parser.add_argument(
        "--disperse-players", action="store_true",
        help="in chunk_load, move twenty players to separate, newly generated view areas",
    )
    parser.add_argument("--server-bin", type=Path, default=None)
    parser.add_argument("--client-bin", type=Path, default=None)
    parser.add_argument("--admin-bin", type=Path, default=None)
    parser.add_argument("--vanilla-root", type=Path, default=None)
    parser.add_argument(
        "--world-template",
        type=Path,
        default=None,
        help="an official Mojang 26.1 save the restart family copies and restarts on",
    )
    return parser.parse_args(argv)


def _official_capture_host_error(
    *, platform_os: str | None = None, sh_bin: str | None = None
) -> str | None:
    effective_os = os.name if platform_os is None else platform_os
    if effective_os == "nt":
        return (
            "server release gate: automatic world capture via scripts/capture-official-world-26.1.sh\n"
            "  requires POSIX sh and mkfifo and is unsupported on Windows; pass\n"
            "  --world-template <official 26.1 save> or set BBB_SERVER_RELEASE_WORLD_TEMPLATE"
        )
    resolved_sh = shutil.which("sh") if sh_bin is None else sh_bin
    if not resolved_sh:
        return (
            "server release gate: POSIX 'sh' is unavailable for scripts/capture-official-world-26.1.sh;\n"
            "  pass --world-template <official 26.1 save> or set BBB_SERVER_RELEASE_WORLD_TEMPLATE"
        )
    return None


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    artifact_env = os.environ.get("BBB_SERVER_RELEASE_ARTIFACT_DIR")
    if args.artifact_dir is not None:
        artifact_dir = args.artifact_dir.resolve()
    elif artifact_env:
        artifact_dir = Path(artifact_env).resolve()
    else:
        artifact_dir = Path(tempfile.mkdtemp(prefix="bbb-server-release-"))
    artifact_dir.mkdir(parents=True, exist_ok=True)

    selected = list(dict.fromkeys([*(args.mode or []), *(args.families or [])])) or sorted(MODES)
    world_template = args.world_template or (
        Path(os.environ["BBB_SERVER_RELEASE_WORLD_TEMPLATE"])
        if os.environ.get("BBB_SERVER_RELEASE_WORLD_TEMPLATE")
        else None
    )
    if any(m in ("restart", "signal") for m in selected) and world_template is None:
        if os.environ.get("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA") != "1":
            print(
                "server release gate: the restart and signal families need a world; set\n"
                "  BBB_SERVER_RELEASE_WORLD_TEMPLATE=<official 26.1 save>, or\n"
                "  BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1 to capture one",
                file=sys.stderr,
            )
            return 2
        host_error = _official_capture_host_error()
        if host_error is not None:
            print(host_error, file=sys.stderr)
            return 2
        capture_log = artifact_dir / "capture.log"
        capture_env = os.environ.copy()
        capture_env["BBB_OFFICIAL_CAPTURE_ARTIFACT_DIR"] = str(artifact_dir / "capture")
        capture_env["BBB_OFFICIAL_CAPTURE_RECEIPT"] = str(
            artifact_dir / "capture" / "official-capture-receipt.json"
        )
        with capture_log.open("wb") as cap_out:
            cap_res = subprocess.run(
                ["sh", str(REPO_ROOT / "scripts/capture-official-world-26.1.sh")],
                cwd=REPO_ROOT,
                env=capture_env,
                stdout=cap_out,
                stderr=subprocess.STDOUT,
                check=False,
            )
        if cap_res.returncode != 0:
            print(
                f"server release gate: official world capture failed; log: {capture_log}",
                file=sys.stderr,
            )
            return 1
        world_template = artifact_dir / "capture" / "server" / "world"

    paths = Paths(
        artifact_dir=artifact_dir,
        server_bin=_resolve_release_bin(
            args.server_bin, "BBB_SERVER_RELEASE_SERVER_BIN", "bbb-server"
        ),
        client_bin=_resolve_release_bin(
            args.client_bin, "BBB_SERVER_RELEASE_CLIENT_BIN", "bbb-client"
        ),
        admin_bin=_resolve_release_bin(args.admin_bin, "BBB_SERVER_RELEASE_ADMIN_BIN", "bbb-admin"),
        vanilla_root=_resolve_optional_vanilla(args.vanilla_root),
        world_template=world_template,
        disperse_players=args.disperse_players,
    )
    families: dict[str, Any] = {}
    for mode in selected:
        started = time.monotonic()
        try:
            families[mode] = MODES[mode](paths)
        except ReleaseGateError as error:
            print(f"server release gate: {mode} failed: {error}", file=sys.stderr)
            return 1
        families[mode]["seconds"] = round(time.monotonic() - started, 1)
        print(f"server release gate: {mode} passed in {families[mode]['seconds']}s")

    summary = {"families": families}
    (paths.artifact_dir / "release-summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    print(f"server release gate: artifacts: {paths.artifact_dir}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
