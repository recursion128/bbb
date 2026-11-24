#!/usr/bin/env python3
"""Build a local BBB save through the dedicated server's MCP endpoint."""

from __future__ import annotations

import argparse
import json
import os
import secrets
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

from bbb_tools.execution.deadline import Deadline
from bbb_tools.execution.process import ManagedProcess, ProcessFailure

SERVER_TIMEOUT = 120
MCP_TIMEOUT = 60
EXE_SUFFIX = ".exe" if os.name == "nt" else ""


class BuildError(RuntimeError):
    pass


def read_commands(path: Path) -> list[tuple[int, str]]:
    with path.open(encoding="utf-8-sig") as source:
        lines = ((number, line.rstrip("\r\n").lstrip()) for number, line in enumerate(source, 1))
        return [
            (number, line.removeprefix("/"))
            for number, line in lines
            if line and not line.startswith("#")
        ]


def wait_ready(server: ManagedProcess, log_path: Path) -> str:
    deadline = Deadline.after(SERVER_TIMEOUT)
    endpoint = None
    with log_path.open(encoding="utf-8", errors="replace") as log:
        while not deadline.expired():
            position = log.tell()
            line = log.readline()
            if not line.endswith("\n"):
                if server.receipt().returncode is not None:
                    raise BuildError("server exited before it was ready")
                log.seek(position)
                time.sleep(0.1)
                continue
            try:
                record = json.loads(line)["fields"]
            except json.JSONDecodeError:
                continue
            match record.get("event"):
                case "management.listening":
                    address = record["address"].removeprefix("Some(").removesuffix(")")
                    endpoint = f"http://{address}{record['path']}"
                case "server.ready" if endpoint is not None:
                    return endpoint
    raise BuildError(f"server did not become ready within {SERVER_TIMEOUT} seconds")


def admin_call(admin: list[str], tool: str, **arguments: object) -> dict[str, Any]:
    run = subprocess.run(
        [*admin, "call", tool, "--arguments-json", json.dumps(arguments)],
        capture_output=True,
        encoding="utf-8",
        timeout=MCP_TIMEOUT + 5,
        check=False,
    )
    if run.returncode:
        raise BuildError(f"{tool} failed: {run.stderr.strip()}")
    return json.loads(run.stdout)


def send_commands(admin: list[str], path: Path, commands: list[tuple[int, str]]) -> None:
    for number, command in commands:
        try:
            response = admin_call(admin, "command.execute", command=command)["command"]
            if response["result"] is None:
                raise BuildError(json.dumps(response["messages"], ensure_ascii=False))
        except (BuildError, subprocess.TimeoutExpired) as error:
            raise BuildError(f"{path}:{number}: {command}\n{error}") from error


def preload_chunks(admin: list[str], chunks: list[tuple[str, int, int]]) -> None:
    for dimension, x, z in chunks:
        area = {"dimension": dimension, "minX": x, "minZ": z, "maxX": x, "maxZ": z}
        surface = admin_call(admin, "world.surface", **area)
        prefix = f"execute in {surface['dimension']} run forceload"
        query = admin_call(admin, "command.execute", command=f"{prefix} query {x} {z}")["command"]
        if query["result"] != 1:
            added = admin_call(admin, "command.execute", command=f"{prefix} add {x} {z}")["command"]
            if added["result"] is None:
                raise BuildError(
                    f"could not force-load {dimension} at {x}, {z}: "
                    + json.dumps(added["messages"], ensure_ascii=False)
                )

        deadline = Deadline.after(SERVER_TIMEOUT)
        while not admin_call(admin, "world.surface", **area)["columns"][0]["loaded"]:
            if deadline.expired():
                raise BuildError(f"{dimension} at {x}, {z} did not load within {SERVER_TIMEOUT}s")
            time.sleep(0.1)


def run_server(
    args: argparse.Namespace,
    scratch: Path,
    phase: str,
    path: Path,
    commands: list[tuple[int, str]],
) -> None:
    log_path = scratch / f"{phase}.log"
    launch = [
        str(args.bin_dir / f"bbb-server{EXE_SUFFIX}"),
        "--bind",
        "127.0.0.1:0",
        "--mcp-bind",
        "127.0.0.1:0",
        "--mcp-loopback-scope",
        "none",
        "--mcp-token-file",
        str(scratch / "server.token"),
        "--mcp-request-timeout-seconds",
        str(MCP_TIMEOUT),
        "--pause-when-empty-seconds",
        "0",
        "--world",
        str(args.world),
    ]
    if phase == "build":
        if args.seed is not None:
            launch.append(f"--level-seed={args.seed}")
        if args.terrain is not None:
            launch.append(f"--level-type={args.terrain}")

    try:
        with (
            log_path.open("wb") as log,
            ManagedProcess(
                launch,
                stdout=subprocess.DEVNULL,
                stderr=log,
                env={**os.environ, "RUST_LOG": "bbb_server=info"},
            ) as server,
        ):
            endpoint = wait_ready(server, log_path)
            admin = [
                str(args.bin_dir / f"bbb-admin{EXE_SUFFIX}"),
                "--endpoint",
                endpoint,
                "--token-file",
                str(scratch / "admin.token"),
                "--timeout-seconds",
                str(MCP_TIMEOUT),
            ]
            try:
                preload_chunks(admin, args.load_chunk)
                send_commands(admin, path, commands)
                if phase == "build":
                    admin_call(admin, "server.save", flush=True)
            finally:
                failure = sys.exception()
                try:
                    admin_call(admin, "server.stop")
                    if (status := server.wait(SERVER_TIMEOUT)) != 0:
                        raise BuildError(f"server exited with status {status}")
                except (BuildError, ProcessFailure, subprocess.TimeoutExpired) as error:
                    if isinstance(failure, BuildError):
                        raise BuildError(f"{failure}\nShutdown failed: {error}") from error
                    raise
    except (BuildError, ProcessFailure, subprocess.TimeoutExpired):
        print(
            f"{phase} server log:\n{log_path.read_text(encoding='utf-8', errors='replace')}",
            file=sys.stderr,
        )
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world", required=True, type=Path, help="save directory")
    parser.add_argument("--seed", help="seed for a new save")
    parser.add_argument("--terrain", help="new save preset: normal, flat, large_biomes, amplified")
    parser.add_argument("--commands", required=True, type=Path, help="UTF-8 build command file")
    parser.add_argument("--verify", type=Path, help="UTF-8 command file to run after reopening")
    parser.add_argument(
        "--load-chunk",
        nargs=3,
        action="append",
        default=[],
        metavar=("DIMENSION", "BLOCK_X", "BLOCK_Z"),
        help="force-load a chunk and wait for it before running commands (repeatable)",
    )
    parser.add_argument(
        "--bin-dir",
        type=Path,
        default=os.environ.get("BBB_WORLD_BUILD_BIN_DIR"),
        help="directory containing bbb-server and bbb-admin (default: cargo dev build output)",
    )
    args = parser.parse_args()
    try:
        args.load_chunk = [(dimension, int(x), int(z)) for dimension, x, z in args.load_chunk]
    except ValueError:
        parser.error("--load-chunk requires integer block coordinates")
    if args.bin_dir is None:
        parser.error("supply --bin-dir or run cargo dev world-build")
    args.bin_dir = args.bin_dir.resolve()
    for binary in ("bbb-server", "bbb-admin"):
        if not (args.bin_dir / f"{binary}{EXE_SUFFIX}").is_file():
            parser.error(f"{binary} is missing in {args.bin_dir}. Run cargo dev build -p {binary}.")

    try:
        build = read_commands(args.commands)
        verify = read_commands(args.verify) if args.verify else None
        with tempfile.TemporaryDirectory(prefix="bbb-world-build-") as directory:
            scratch = Path(directory)
            token = secrets.token_urlsafe(32)
            (scratch / "server.token").write_text(
                f"world-build:command:{token}\n", encoding="utf-8"
            )
            (scratch / "admin.token").write_text(f"{token}\n", encoding="utf-8")
            run_server(args, scratch, "build", args.commands, build)
            print(f"Saved {args.world} after {len(build)} commands", flush=True)
            if verify is not None:
                run_server(args, scratch, "verify", args.verify, verify)
                print(f"Reopened {args.world} and ran {len(verify)} verification commands")
    except (BuildError, ProcessFailure, OSError, UnicodeError, subprocess.TimeoutExpired) as error:
        print(f"World build failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
