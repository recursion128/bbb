"""Stable input-backend command shared by development and live gates."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

from ..host.capabilities import probe_host
from .contract import InputBackend, InputError
from .linux_external import YdotoolBackend
from .macos_quartz import MacOsQuartzBackend

SCRIPTS = Path(__file__).resolve().parents[2]
LOGICAL_KEYS = {
    "forward": "w",
    "back": "s",
    "left": "a",
    "right": "d",
    "jump": "space",
    "sneak": "shift",
    "inventory": "e",
    "chat": "t",
    "paste": "v",
    "fullscreen": "f11",
    **{f"hotbar{index}": str(index) for index in range(1, 10)},
}


def _backend(name: str, pid: int | None = None) -> InputBackend:
    if name == "macos-quartz":
        return MacOsQuartzBackend(pid=pid)
    if name == "linux-ydotool":
        return YdotoolBackend()
    raise InputError(f"unknown direct input backend {name!r}")


def _logical_key(name: str) -> str:
    return LOGICAL_KEYS.get(name, name)


def _run_direct(args: argparse.Namespace) -> int:
    with _backend(args.backend, args.pid) as backend:
        if args.action == "probe":
            receipt = {"backend": args.backend, "status": "ready"}
            if isinstance(backend, MacOsQuartzBackend):
                capabilities = probe_host()
                if not capabilities.macos_accessibility_trusted:
                    raise InputError(
                        "macOS Accessibility permission is required for Quartz input"
                    )
                source = Path(__file__).with_name("macos_quartz.py")
                receipt.update(
                    {
                        "accessibility_trusted": True,
                        "driver": "coregraphics",
                        "input_backend_source_sha256": hashlib.sha256(
                            source.read_bytes()
                        ).hexdigest(),
                    }
                )
            print(json.dumps(receipt, sort_keys=True))
        elif args.action == "focus":
            if not isinstance(backend, MacOsQuartzBackend) or args.pid is None:
                raise InputError("focus requires macos-quartz and --pid")
            backend.focus_pid(args.pid)
        elif args.action == "key":
            backend.key(_logical_key(args.name), hold_seconds=args.hold_seconds)
        elif args.action == "chord":
            backend.chord(
                _logical_key(args.modifier),
                _logical_key(args.name),
                hold_seconds=args.hold_seconds,
            )
        elif args.action == "click":
            backend.click(args.button, hold_seconds=args.hold_seconds)
        elif args.action == "move":
            backend.move(args.dx, args.dy)
        elif args.action == "clipboard":
            if not isinstance(backend, MacOsQuartzBackend):
                raise InputError("clipboard is supported by macos-quartz only")
            backend.clipboard(args.text)
        else:
            raise InputError(f"{args.action!r} is not supported by {args.backend}")
    return 0


def _run_linux_release(arguments: list[str]) -> int:
    command = [
        sys.executable,
        str(SCRIPTS / "seat_inject.py"),
        "--driver",
        "uinput",
        *arguments,
    ]
    return subprocess.run(command, check=False).returncode


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="scripts/input")
    parser.add_argument(
        "--backend",
        choices=("macos-quartz", "linux-ydotool", "linux-release"),
        required=True,
    )
    parser.add_argument("--hold-seconds", type=float, default=0.4)
    parser.add_argument("--pid", type=int)
    subparsers = parser.add_subparsers(dest="action", required=True)
    subparsers.add_parser("probe")
    subparsers.add_parser("focus")
    key = subparsers.add_parser("key")
    key.add_argument("name")
    chord = subparsers.add_parser("chord")
    chord.add_argument("modifier")
    chord.add_argument("name")
    move = subparsers.add_parser("move")
    move.add_argument("dx", type=float)
    move.add_argument("dy", type=float)
    click = subparsers.add_parser("click")
    click.add_argument("button", choices=("left", "middle", "right"))
    clipboard = subparsers.add_parser("clipboard")
    clipboard.add_argument("text")
    scale = subparsers.add_parser("output-scale")
    scale.add_argument("scale")
    subparsers.add_parser("window")
    geometry = subparsers.add_parser("window-geometry")
    geometry.add_argument("width")
    geometry.add_argument("height")
    text = subparsers.add_parser("text")
    text.add_argument("characters")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    raw = list(sys.argv[1:] if argv is None else argv)
    args = _parser().parse_args(raw)
    try:
        if args.hold_seconds <= 0:
            raise InputError("--hold-seconds must be positive")
        if args.backend == "linux-release":
            backend_index = raw.index("--backend")
            forwarded = raw[:backend_index] + raw[backend_index + 2 :]
            return _run_linux_release(forwarded)
        return _run_direct(args)
    except InputError as error:
        print(f"input backend failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
