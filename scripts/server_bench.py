#!/usr/bin/env python3
"""Equal-workload performance baseline for the dedicated server.

One noise Overworld save, made once by the official 26.1 server from a fixed
seed, is copied for every run, so the numbers taken before and after a refactor
step describe the same work: an idle loaded world, twenty bbb clients at spawn,
generating a forceloaded 16x16 chunk area, and 512 cows ticking on it.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

from bbb_tools.anvil import WorldSaveReader, read_chunk  # noqa: E402
from bbb_tools.host.process_metrics import (  # noqa: E402
    host_cpu_model,
    sample_process_metrics,
)
from server_release_gate import (  # noqa: E402
    RELEASE_PLAYERS,
    Paths,
    ReleaseGateError,
    ServerProcess,
    assert_played,
    free_port,
    install_chunk_check,
    loaded_area_tick,
    local_bot_connection_args,
    require,
    run_round,
    tick_health,
    wait_for_player_count,
)
from server_release_gate import (
    process_footprint as _linux_process_footprint,
)

SEED = "bbb-server-bench"
FORCELOAD = (2000, 2000, 2255, 2255)
COW_COLUMNS = 32
COW_ROWS = 16
COW_SPACING = 6
PLAYER_HOLD_SECONDS = 30
PLAYER_TICK_WINDOWS = 3
FAST_FLY_HOLD_SECONDS = 30
FAST_FLY_HOP_INTERVAL_SECONDS = 0.5
FAST_FLY_STEP_BLOCKS = 64
FAST_FLY_Y = 160
CHUNKGEN_TIMEOUT = 900.0
ALL_SCENARIOS = ("idle", "players", "chunkgen", "mobs", "fast_flying_autosave")


def parse_scenarios(raw: str) -> tuple[str, ...]:
    tokens = [token.strip() for token in raw.split(",") if token.strip()]
    if not tokens or tokens == ["all"]:
        return ALL_SCENARIOS
    if "all" in tokens:
        raise SystemExit("--scenarios cannot combine 'all' with individual names")
    unknown = [token for token in tokens if token not in ALL_SCENARIOS]
    if unknown:
        raise SystemExit(
            f"unknown --scenarios {', '.join(unknown)}; expected subset of {', '.join(ALL_SCENARIOS)}"
        )
    selected = tuple(name for name in ALL_SCENARIOS if name in tokens)
    if "mobs" in selected and "chunkgen" not in selected:
        raise SystemExit("--scenarios mobs requires chunkgen to prepare the forceloaded area")
    return selected


def make_template(java: Path, jar: Path, template: Path) -> None:
    if os.environ.get("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA") != "1":
        raise SystemExit("making the template needs BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1")
    run_dir = template.parent / "official-run"
    shutil.rmtree(run_dir, ignore_errors=True)
    run_dir.mkdir(parents=True)
    (run_dir / "eula.txt").write_text("eula=true\n")
    (run_dir / "server.properties").write_text(
        "\n".join(
            [
                f"level-seed={SEED}",
                "level-type=minecraft\\:normal",
                "online-mode=false",
                f"server-port={free_port()}",
                "difficulty=peaceful",
                "",
            ]
        )
    )
    process = subprocess.Popen(
        [str(java), "-Xmx4G", "-jar", str(jar), "nogui"],
        cwd=run_dir,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    assert process.stdin is not None and process.stdout is not None
    for line in process.stdout:
        if "Done (" in line:
            process.stdin.write("stop\n")
            process.stdin.flush()
            break
    process.wait(timeout=300)
    shutil.rmtree(template, ignore_errors=True)
    shutil.copytree(run_dir / "world", template)


def mean_tick(samples: list[dict[str, Any]]) -> dict[str, int]:
    means = [int(sample.get("mean_tick_micros", 0)) for sample in samples]
    peaks = [int(sample.get("max_tick_micros", 0)) for sample in samples]
    return {
        "windows": len(samples),
        "mean_tick_micros": sum(means) // max(len(means), 1),
        "max_tick_micros": max(peaks, default=0),
    }


def wait_windows(server: ServerProcess, after: int, count: int) -> list[dict[str, Any]]:
    deadline = time.monotonic() + 60 + count * 10
    while time.monotonic() < deadline:
        samples = tick_health(server)
        if len(samples) >= after + count:
            return samples[after : after + count]
        time.sleep(0.5)
    raise SystemExit(f"the server published no {count} tick windows after window {after}")


def settle(server: ServerProcess) -> None:
    # Diagnostics queue behind whatever tick is still running, so the first
    # answer means every earlier operation has applied. Only this idempotent
    # call is retried; a command that outlasts the admin client's wait is
    # never sent twice.
    deadline = time.monotonic() + CHUNKGEN_TIMEOUT
    while time.monotonic() < deadline:
        try:
            server.management("call", "server.diagnostics")
            return
        except ReleaseGateError as error:
            if "did not answer in time" not in str(error):
                raise
    raise ReleaseGateError("the server did not finish its pending tick within 900 seconds")


def command(server: ServerProcess, text: str) -> Any:
    try:
        return server.management(
            "call", "command.execute", "--arguments-json", json.dumps({"command": text})
        )
    except ReleaseGateError as error:
        if "did not answer in time" not in str(error):
            raise
        settle(server)
        return None


def terrain_snapshot(world: Path, output: Path) -> str:
    reader = WorldSaveReader(world, "overworld")
    x1, z1, x2, z2 = FORCELOAD
    chunks = []
    for z in range(z1 >> 4, (z2 >> 4) + 1):
        for x in range(x1 >> 4, (x2 >> 4) + 1):
            chunk = read_chunk(reader.region_dir, x, z)
            if chunk is None:
                raise ReleaseGateError(f"generated chunk {x},{z} was not saved")
            chunks.append(
                {
                    key: chunk[key]
                    for key in ("xPos", "zPos", "sections", "Heightmaps", "structures")
                }
            )
    encoded = json.dumps(chunks, sort_keys=True, separators=(",", ":"), default=list).encode()
    output.write_bytes(encoded)
    return hashlib.sha256(encoded).hexdigest()


def process_footprint(server: ServerProcess) -> dict[str, int]:
    require(server.process is not None, f"{server.name} was never started")
    pid = server.process.pid  # type: ignore[union-attr]
    if (Path("/proc") / str(pid) / "status").is_file():
        return _linux_process_footprint(server)
    metrics = sample_process_metrics(pid)
    footprint: dict[str, int] = {"rss_kib": metrics.rss_kib}
    if metrics.threads is not None:
        footprint["threads"] = metrics.threads
    if metrics.open_files is not None:
        footprint["open_files"] = metrics.open_files
    return footprint


def allocation_profile(server: ServerProcess, paths: Paths) -> dict[str, Any]:
    if server.process is None:
        raise ReleaseGateError("cannot profile a server that was not started")
    pid = server.process.pid
    maps_path = Path(f"/proc/{pid}/maps")
    if not maps_path.is_file():
        raise ReleaseGateError(
            "--profile-allocations requires Linux /proc/<pid>/maps and bpftrace"
        )
    libc = next(
        line.split()[-1]
        for line in maps_path.read_text(encoding="utf-8", errors="replace").splitlines()
        if line.split()[-1].endswith("/libc.so.6")
    )
    program = "\n".join(
        f'uprobe:"{libc}":{name} /pid == {pid}/ '
        f"{{ @calls[tid] = @calls[tid] + 1; @bytes[tid] = @bytes[tid] + ({size}); }}"
        for name, size in (
            ("malloc", "arg0"),
            ("calloc", "arg0 * arg1"),
            ("realloc", "arg1"),
            ("posix_memalign", "arg2"),
        )
    )
    program += "\ninterval:s:5 { exit(); }"
    before = server.management("call", "server.diagnostics")["diagnostics"]["tick"]
    started = time.monotonic()
    completed = subprocess.run(
        ["sudo", "-n", "bpftrace", "-q", "-e", program],
        capture_output=True,
        text=True,
        timeout=30,
        check=True,
    )
    paths.child("allocations.log").write_text(completed.stdout + completed.stderr)
    after = server.management("call", "server.diagnostics")["diagnostics"]["tick"]
    counts = {"calls": 0, "bytes": 0}
    for line in completed.stdout.splitlines():
        for key in counts:
            if line.startswith(f"@{key}["):
                counts[key] += int(line.rsplit(":", 1)[1])
    ticks = after - before
    if ticks <= 0 or counts["calls"] <= 0:
        raise ReleaseGateError("the server did not tick during allocation profiling")
    return {
        "method": "libc-uprobes; separate from timing windows; includes all server threads",
        "seconds": round(time.monotonic() - started, 2),
        "ticks": ticks,
        "allocation_calls": counts["calls"],
        "requested_bytes": counts["bytes"],
        "allocation_calls_per_tick": counts["calls"] // ticks,
        "requested_bytes_per_tick": counts["bytes"] // ticks,
    }


def benchmark_players(paths: Paths, server: ServerProcess) -> dict[str, Any]:
    names = [f"bench-{seat:02d}" for seat in range(1, RELEASE_PLAYERS + 1)]
    runs = run_round(paths, server, names, hold_seconds=PLAYER_HOLD_SECONDS)
    listed = wait_for_player_count(server, RELEASE_PLAYERS, timeout=PLAYER_HOLD_SECONDS)
    require(listed == sorted(names), f"the player workload never joined together: {listed}")
    # Skip the window already in progress when the final player joined.
    samples = wait_windows(server, len(tick_health(server)) + 1, PLAYER_TICK_WINDOWS)
    footprint = process_footprint(server)
    for run in runs:
        require(run.wait() == 0, f"{run.username} failed: {run.failure_detail()[:400]}")
        assert_played(run, held=True)
    require(
        all(int(sample.get("connections", 0)) == RELEASE_PLAYERS for sample in samples),
        "the twenty-player workload did not last through its timing windows",
    )
    result = mean_tick(samples)
    result.update(footprint)
    result.update(
        attempted=len(runs),
        completed=len(runs),
        peak_connections=RELEASE_PLAYERS,
    )
    return result


def _autosave_count(server: ServerProcess) -> int:
    return sum(
        1
        for record in server.events()
        if record.get("event") == "world.saved" and record.get("trigger") == "autosave"
    )


def benchmark_fast_flying_autosave(paths: Paths, server: ServerProcess) -> dict[str, Any]:
    flyer = "bench-flyer"
    prior_autosaves = _autosave_count(server)
    runs = run_round(paths, server, [flyer], hold_seconds=FAST_FLY_HOLD_SECONDS)
    listed = wait_for_player_count(server, 1, timeout=FAST_FLY_HOLD_SECONDS)
    require(listed == [flyer], f"the fast-flying player never joined cleanly: {listed}")

    command(server, f"gamemode creative {flyer}")
    # `tick rate 1` clamps ticks_until_autosave down to 300; restoring 20 TPS
    # keeps that 300-tick (15s) countdown so a real autosave lands mid-flight.
    command(server, "tick rate 1")
    command(server, "tick rate 20")

    before_window = len(tick_health(server))
    hops = 0
    autosaves_after = prior_autosaves
    deadline = time.monotonic() + FAST_FLY_HOLD_SECONDS
    while time.monotonic() < deadline:
        x = 1024 + hops * FAST_FLY_STEP_BLOCKS
        z = 1024 + (hops % 4) * 32
        command(server, f"tp {flyer} {x} {FAST_FLY_Y} {z}")
        hops += 1
        autosaves_after = _autosave_count(server)
        if autosaves_after > prior_autosaves and len(tick_health(server)) > before_window:
            break
        time.sleep(FAST_FLY_HOP_INTERVAL_SECONDS)

    samples = tick_health(server)[before_window:]
    if not samples:
        samples = wait_windows(server, before_window, 1)
    footprint = process_footprint(server)
    require(runs[0].wait() == 0, f"{flyer} failed: {runs[0].failure_detail()[:400]}")
    assert_played(runs[0], held=True)
    autosaves_after = max(autosaves_after, _autosave_count(server))
    require(
        autosaves_after > prior_autosaves,
        "fast-flying workload never triggered a world.saved autosave event",
    )

    result = mean_tick(samples)
    result["mean_mspt"] = round(result["mean_tick_micros"] / 1000.0, 3)
    result["max_mspt"] = round(result["max_tick_micros"] / 1000.0, 3)
    result.update(footprint)
    result.update(hops=hops, autosaves=autosaves_after - prior_autosaves)
    return result


def bench(
    paths: Paths,
    template: Path,
    *,
    profile_allocations: bool = False,
    scenarios: tuple[str, ...] = ALL_SCENARIOS,
) -> dict[str, Any]:
    selected = set(scenarios)
    world = paths.child("world")
    shutil.rmtree(world, ignore_errors=True)
    shutil.copytree(template, world)
    x1, z1, x2, z2 = FORCELOAD
    forced = [(x, z) for x in range(x1 // 16, x2 // 16 + 1) for z in range(z1 // 16, z2 // 16 + 1)]
    if "chunkgen" in selected:
        install_chunk_check(paths, world, forced)
    server = ServerProcess(
        paths=paths,
        name="bench-server",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("admin"),
        world_dir=world,
        loopback_scope="command",
    )
    result: dict[str, Any] = {}
    progress = paths.child("bench.partial.json")
    try:
        started = time.monotonic()
        server.start(extra=local_bot_connection_args())
        server.wait_ready()
        result["startup_seconds"] = round(time.monotonic() - started, 2)

        if "idle" in selected:
            result["idle"] = mean_tick(wait_windows(server, len(tick_health(server)), 3))
            result["idle"].update(process_footprint(server))
            progress.write_text(json.dumps(result, indent=2))

        if "players" in selected:
            result["players"] = benchmark_players(paths, server)
            progress.write_text(json.dumps(result, indent=2))

        if "chunkgen" in selected:
            chunk_count = len(forced)
            command(server, "scoreboard objectives add bbb_chunk_check dummy")
            before = len(tick_health(server))
            command(server, "tick freeze")
            started = time.monotonic()
            command(server, f"forceload add {x1} {z1} {x2} {z2}")
            while loaded_area_tick(server) is None:
                if time.monotonic() - started > CHUNKGEN_TIMEOUT:
                    raise SystemExit("the forceloaded area never finished loading")
                time.sleep(0.2)
            seconds = time.monotonic() - started
            result["chunkgen"] = {
                "chunks": chunk_count,
                "seconds": round(seconds, 2),
                "chunks_per_second": round(chunk_count / seconds, 1),
                **mean_tick(wait_windows(server, before, 1)),
            }

            server.management("call", "server.save")
            settle(server)
            result["chunkgen"]["terrain_sha256"] = terrain_snapshot(world, paths.child("terrain.json"))
            progress.write_text(json.dumps(result, indent=2))

        if "mobs" in selected:
            centre_x, centre_z = (x1 + x2) // 2, (z1 + z2) // 2
            command(server, "kill @e[type=minecraft:cow]")
            command(server, f"fill {x1} 300 {z1} {centre_x} 300 {z2} minecraft:grass_block")
            command(server, f"fill {centre_x + 1} 300 {z1} {x2} 300 {z2} minecraft:grass_block")
            for row in range(COW_ROWS):
                for column in range(COW_COLUMNS):
                    x = centre_x + (column - COW_COLUMNS // 2) * COW_SPACING + 0.5
                    z = centre_z + (row - COW_ROWS // 2) * COW_SPACING + 0.5
                    command(
                        server,
                        f"summon minecraft:cow {x} 301 {z}",
                    )
            command(server, "tick unfreeze")
            # Killed generated cows must finish their death ticks before counting the workload.
            wait_windows(server, len(tick_health(server)), 1)
            count = command(server, "execute if entity @e[type=minecraft:cow]") or {}
            result["mobs"] = {"cows": count.get("command", {}).get("result")}
            if result["mobs"]["cows"] != COW_COLUMNS * COW_ROWS:
                raise ReleaseGateError(f"expected 512 cows, got {result['mobs']['cows']}")
            result["mobs"].update(mean_tick(wait_windows(server, len(tick_health(server)) + 1, 4)))
            result["mobs"].update(process_footprint(server))
            final_count = command(server, "execute if entity @e[type=minecraft:cow]") or {}
            result["mobs"]["cows_after"] = final_count.get("command", {}).get("result")
            if result["mobs"]["cows_after"] != COW_COLUMNS * COW_ROWS:
                raise ReleaseGateError("the cow workload changed during its measurement")
            if profile_allocations:
                result["mobs"]["allocation_profile"] = allocation_profile(server, paths)
            progress.write_text(json.dumps(result, indent=2))

        if "fast_flying_autosave" in selected:
            result["fast_flying_autosave"] = benchmark_fast_flying_autosave(paths, server)
            progress.write_text(json.dumps(result, indent=2))

        server.stop_through_management()
    finally:
        server.terminate()
    counts: dict[str, int] = {}
    for record in server.events():
        name = str(record.get("event"))
        if name not in ("world.tick_health", "admin.operation"):
            counts[name] = counts.get(name, 0) + 1
    result["events"] = counts
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--server-bin", type=Path, required=True)
    parser.add_argument("--client-bin", type=Path, required=True)
    parser.add_argument("--admin-bin", type=Path, required=True)
    parser.add_argument("--vanilla-root", type=Path, required=True)
    parser.add_argument("--java-bin", type=Path)
    parser.add_argument("--server-jar", type=Path)
    parser.add_argument(
        "--profile-allocations",
        action="store_true",
        help="sample libc allocations with sudo -n bpftrace after timing",
    )
    parser.add_argument(
        "--scenarios",
        default="all",
        help=f"comma-separated subset of {', '.join(ALL_SCENARIOS)} (default: all)",
    )
    args = parser.parse_args()
    scenarios = parse_scenarios(args.scenarios)

    if not (args.template / "level.dat").exists():
        if args.java_bin is None or args.server_jar is None:
            raise SystemExit("no template; pass --java-bin and --server-jar to make one")
        make_template(args.java_bin, args.server_jar, args.template)
    args.artifact_dir.mkdir(parents=True, exist_ok=True)
    paths = Paths(
        artifact_dir=args.artifact_dir,
        server_bin=args.server_bin,
        client_bin=args.client_bin,
        admin_bin=args.admin_bin,
        vanilla_root=args.vanilla_root,
    )
    result = bench(
        paths,
        args.template,
        profile_allocations=args.profile_allocations,
        scenarios=scenarios,
    )
    result["provenance"] = {
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "machine": platform.platform(),
        "cpu": host_cpu_model(),
        "command": sys.argv,
        "binaries": {
            name: hashlib.sha256(path.read_bytes()).hexdigest()
            for name, path in (
                ("server", args.server_bin),
                ("client", args.client_bin),
                ("admin", args.admin_bin),
            )
        },
    }
    text = json.dumps(result, indent=2)
    (args.artifact_dir / "bench.json").write_text(text + "\n")
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
