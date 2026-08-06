#!/usr/bin/env python3
"""Hold a virtual seat input device open and inject through it on request.

A compositor only advertises keyboard and pointer capabilities while input
devices exist, and a client only binds ``wl_keyboard``/``wl_pointer`` -- and so
only ever receives focus -- while those capabilities are advertised.  Tools
that create a virtual device per invocation therefore race the client: the
device is gone again before the client has bound anything.  This daemon creates
one ``uinput`` device and keeps it open for the whole run, so the capabilities
are stable and the client behaves exactly as it would with a physical seat.

Going through ``uinput`` also keeps the *physical* key identity honest.  The
virtual-keyboard Wayland protocol carries keysyms on a caller-supplied keymap,
so a client that reads physical key codes (as this one does, and as vanilla
does) sees whichever scancode the tool happened to pick.  A ``uinput`` device
emits real evdev scancodes, which libinput and the compositor pass through
unchanged.

    uinput device -> libinput -> compositor -> wl_seat -> client

Run it alongside the compositor::

    scripts/seat_device.py serve --socket /tmp/bbb-seat.sock

It needs write access to ``/dev/uinput``; on a runner that means one
provisioning step, e.g. ``sudo modprobe uinput`` plus a group grant such as
``sudo chgrp <group> /dev/uinput && sudo chmod 660 /dev/uinput``.
"""

from __future__ import annotations

import argparse
import fcntl
import os
import socket
import socketserver
import struct
import sys
import time

# Linux input-event-codes.h.  These are physical key positions, not symbols.
KEY_CODES = {
    "escape": 1,
    "enter": 28,
    "ctrl": 29,  # chord modifier; the injector has no gate key of its own
    "sneak": 42,  # left shift
    "jump": 57,  # space
    "forward": 17,  # W
    "back": 31,  # S
    "left": 30,  # A
    "right": 32,  # D
    "inventory": 18,  # E
    "chat": 20,  # T
    "paste": 47,  # V
    "fullscreen": 87,  # F11
    **{f"hotbar{index}": index + 1 for index in range(1, 10)},  # KEY_1..KEY_9
}
# QWERTY letter positions, for typing a literal string one physical key at a
# time.  An input method turns these into something else entirely, which is the
# point of typing through the seat rather than writing text into the client.
LETTER_CODES = {
    "q": 16, "w": 17, "e": 18, "r": 19, "t": 20,
    "y": 21, "u": 22, "i": 23, "o": 24, "p": 25,
    "a": 30, "s": 31, "d": 32, "f": 33, "g": 34,
    "h": 35, "j": 36, "k": 37, "l": 38,
    "z": 44, "x": 45, "c": 46, "v": 47, "b": 48, "n": 49, "m": 50,
    " ": 57,
}
BUTTON_CODES = {"left": 0x110, "middle": 0x112, "right": 0x111}
# One key of a typed string: long enough for the compositor to route it, short
# enough that a whole word does not outlast the scenario budget.
TYPE_HOLD_SECONDS = 0.08

UI_SET_EVBIT = 0x40045564
UI_SET_KEYBIT = 0x40045565
UI_SET_RELBIT = 0x40045566
UI_DEV_CREATE = 0x5501
UI_DEV_DESTROY = 0x5502

EV_SYN, EV_KEY, EV_REL = 0x00, 0x01, 0x02
SYN_REPORT = 0x00
REL_X, REL_Y = 0x00, 0x01

DEVICE_NAME = b"bbb-seat-virtual"
# The settle keeps a press visible across one frame of a client that is
# uploading terrain; a twelve-millisecond tap can land entirely between two.
DEFAULT_HOLD_SECONDS = 1.5


class SeatDeviceError(RuntimeError):
    """The virtual seat device is unusable, so no receipt may be claimed."""


class VirtualSeatDevice:
    """One uinput keyboard+pointer, held open for the life of the process."""

    def __init__(self) -> None:
        try:
            self._fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        except OSError as error:
            raise SeatDeviceError(
                f"cannot open /dev/uinput ({error}); load the uinput module and "
                "grant this user write access to it"
            ) from error
        for event_type in (EV_KEY, EV_REL, EV_SYN):
            fcntl.ioctl(self._fd, UI_SET_EVBIT, event_type)
        for code in {*KEY_CODES.values(), *BUTTON_CODES.values()}:
            fcntl.ioctl(self._fd, UI_SET_KEYBIT, code)
        for code in (REL_X, REL_Y):
            fcntl.ioctl(self._fd, UI_SET_RELBIT, code)
        # struct uinput_user_dev: name[80], input_id{bustype,vendor,product,
        # version}, ff_effects_max, absmax/absmin/absfuzz/absflat[64].
        payload = (
            DEVICE_NAME.ljust(80, b"\0")
            + struct.pack("HHHH", 0x03, 0x1234, 0x5678, 1)
            + struct.pack("i", 0)
            + b"\0" * (4 * 64 * 4)
        )
        os.write(self._fd, payload)
        fcntl.ioctl(self._fd, UI_DEV_CREATE)
        # libinput discovers the device asynchronously; injecting before the
        # compositor has it produces events nothing routes.
        time.sleep(1.0)

    def close(self) -> None:
        try:
            fcntl.ioctl(self._fd, UI_DEV_DESTROY)
        finally:
            os.close(self._fd)

    def _emit(self, event_type: int, code: int, value: int) -> None:
        # struct input_event: timeval{sec,usec}, type, code, value.  A zero
        # timestamp asks the kernel to stamp it.
        os.write(self._fd, struct.pack("llHHi", 0, 0, event_type, code, value))

    def _sync(self) -> None:
        self._emit(EV_SYN, SYN_REPORT, 0)

    def key(self, name: str, hold_seconds: float) -> None:
        code = KEY_CODES.get(name)
        if code is None:
            raise SeatDeviceError(f"no physical key is mapped for {name!r}")
        self._emit(EV_KEY, code, 1)
        self._sync()
        time.sleep(hold_seconds)
        self._emit(EV_KEY, code, 0)
        self._sync()

    def chord(self, modifier: str, name: str, hold_seconds: float) -> None:
        modifier_code = KEY_CODES.get(modifier)
        code = KEY_CODES.get(name)
        if modifier_code is None or code is None:
            raise SeatDeviceError(f"no physical key for chord {modifier}+{name}")
        self._emit(EV_KEY, modifier_code, 1)
        self._sync()
        self._emit(EV_KEY, code, 1)
        self._sync()
        time.sleep(hold_seconds)
        self._emit(EV_KEY, code, 0)
        self._sync()
        self._emit(EV_KEY, modifier_code, 0)
        self._sync()

    def click(self, button: str, hold_seconds: float) -> None:
        code = BUTTON_CODES.get(button)
        if code is None:
            raise SeatDeviceError(f"unknown pointer button {button!r}")
        self._emit(EV_KEY, code, 1)
        self._sync()
        time.sleep(hold_seconds)
        self._emit(EV_KEY, code, 0)
        self._sync()

    def text(self, characters: str) -> None:
        """Type a literal ASCII string through the physical keys.

        With an input method on the seat, what the client receives is a commit
        of something else entirely -- which is exactly what makes the receipt
        worth having.
        """

        for character in characters:
            code = LETTER_CODES.get(character.lower())
            if code is None:
                raise SeatDeviceError(f"no physical key types {character!r}")
            self._emit(EV_KEY, code, 1)
            self._sync()
            time.sleep(TYPE_HOLD_SECONDS)
            self._emit(EV_KEY, code, 0)
            self._sync()
            time.sleep(TYPE_HOLD_SECONDS)

    def move(self, dx: int, dy: int, steps: int = 10) -> None:
        """Relative pointer motion, delivered as a real device would: in
        several small deltas rather than one jump, so a client sampling per
        frame sees motion rather than a single teleport."""

        for step in range(steps):
            step_x = dx * (step + 1) // steps - dx * step // steps
            step_y = dy * (step + 1) // steps - dy * step // steps
            if step_x:
                self._emit(EV_REL, REL_X, step_x)
            if step_y:
                self._emit(EV_REL, REL_Y, step_y)
            self._sync()
            time.sleep(0.02)


def _handle(device: VirtualSeatDevice, line: str) -> str:
    parts = line.split()
    if not parts:
        return "error empty command"
    action, arguments = parts[0], parts[1:]
    try:
        if action == "ping":
            return "ok"
        if action == "keys":
            # The daemon outlives edits to this file: it holds the key table it
            # was started with.  Answering with that table lets a caller refuse
            # a stale daemon up front rather than discovering a missing key
            # deep inside a run.
            # Space is reported as `jump`, since a bare space cannot survive
            # this whitespace-separated answer.
            typeable = {name for name in LETTER_CODES if not name.isspace()}
            return "ok " + " ".join(sorted({*KEY_CODES, *typeable}))
        if action == "text":
            # The rest of the line verbatim, so a typed space survives.
            _, _, characters = line.partition(" ")
            if not characters:
                return "error text needs something to type"
            device.text(characters)
            return "ok"
        if action == "key":
            device.key(arguments[0], float(arguments[1]) if len(arguments) > 1 else DEFAULT_HOLD_SECONDS)
            return "ok"
        if action == "chord":
            device.chord(
                arguments[0],
                arguments[1],
                float(arguments[2]) if len(arguments) > 2 else DEFAULT_HOLD_SECONDS,
            )
            return "ok"
        if action == "click":
            device.click(arguments[0], float(arguments[1]) if len(arguments) > 1 else DEFAULT_HOLD_SECONDS)
            return "ok"
        if action == "move":
            device.move(int(arguments[0]), int(arguments[1]))
            return "ok"
        return f"error unknown command {action!r}"
    except (IndexError, ValueError) as error:
        return f"error bad arguments for {action}: {error}"
    except SeatDeviceError as error:
        return f"error {error}"


def serve(socket_path: str) -> int:
    device = VirtualSeatDevice()

    class Handler(socketserver.StreamRequestHandler):
        def handle(self) -> None:
            for raw in self.rfile:
                reply = _handle(device, raw.decode("utf-8", "replace").strip())
                self.wfile.write(reply.encode("utf-8") + b"\n")
                self.wfile.flush()

    class Server(socketserver.ThreadingUnixStreamServer):
        allow_reuse_address = True

    if os.path.exists(socket_path):
        os.unlink(socket_path)
    server = Server(socket_path, Handler)
    os.chmod(socket_path, 0o600)
    print(f"seat device ready on {socket_path}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        device.close()
        if os.path.exists(socket_path):
            os.unlink(socket_path)
    return 0


def request(socket_path: str, command: str, timeout: float = 120.0) -> str:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(timeout)
        try:
            client.connect(socket_path)
        except OSError as error:
            raise SeatDeviceError(
                f"no seat device is listening on {socket_path} ({error})"
            ) from error
        client.sendall(command.encode("utf-8") + b"\n")
        reply = b""
        while not reply.endswith(b"\n"):
            chunk = client.recv(4096)
            if not chunk:
                break
            reply += chunk
    answer = reply.decode("utf-8", "replace").strip()
    # Acting commands answer bare "ok"; reporting ones answer "ok <payload>".
    if answer != "ok" and not answer.startswith("ok "):
        raise SeatDeviceError(answer or "the seat device gave no answer")
    return answer


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--socket", default=os.environ.get("BBB_SEAT_DEVICE_SOCKET", "/tmp/bbb-seat.sock")
    )
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("serve")
    send = sub.add_parser("send")
    send.add_argument("words", nargs="+")
    return parser


def require_linux_host() -> None:
    """Refuse attributably off Linux instead of failing on a missing device.

    A macOS developer running the full dev chain will meet this script; the
    kernel interface it drives exists nowhere else, so say that rather than
    surfacing a `FileNotFoundError` on `/dev/uinput`.
    """
    if sys.platform != "linux":
        raise SeatDeviceError(
            f"the seat device needs Linux uinput; this host is {sys.platform}. "
            "Seat-driven gates run on the declared Linux release host only"
        )


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        require_linux_host()
        if args.command == "serve":
            return serve(args.socket)
        request(args.socket, " ".join(args.words))
        return 0
    except SeatDeviceError as error:
        print(f"seat device failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
