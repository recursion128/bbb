#!/usr/bin/env python3
"""Inject input into the playable gate's client through a real compositor seat.

The live playable gate normally synthesises input through the ``input.*``
control-plane requests, which replay the canonical handlers but never exercise
the platform's own event path.  This injector closes that gap by driving the
compositor's own ``wl_seat``, so the client receives focus, keyboard, pointer
and selection events exactly as it would from a physical seat.

Two drivers exist because two compositors reach different parts of the seat:

``wlroots`` (default)
    A wlroots compositor (sway with its headless backend) offers virtual
    keyboard, virtual pointer and data-control protocols, so this driver can
    deliver **relative** pointer motion and put text on the seat's clipboard.
    It also hands the client hardware Vulkan and focuses it on map.
``xtest``
    A remote-desktop seat: XTEST drives a remote-desktop client window that is
    itself a Wayland client of the compositor under test.  It reaches focus,
    keyboard and pointer buttons, but a remote-desktop pointer is an absolute
    device, so a locked cursor receives no motion from it.

Selected with ``BBB_SEAT_INJECT_DRIVER``.  Both read the same gate key names
(``hotbar4``, ``forward``, ...) so a seat-driven scenario reads the same as the
control-driven ones.

``xtest`` also reads ``BBB_SEAT_INJECT_DISPLAY`` (X display hosting the
remote-desktop client window, default ``:79``) and ``BBB_SEAT_INJECT_WINDOW``
(its window class, default ``freerdp``).  Both read
``BBB_SEAT_INJECT_HOLD_SECONDS``: keys and buttons are held rather than tapped,
because a twelve-millisecond tap lands entirely between two frames of a client
that is uploading terrain.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import seat_device  # noqa: E402  (sibling module, resolved from this file's directory)

DEFAULT_DISPLAY = ":79"
DEFAULT_WINDOW_CLASS = "freerdp"
# Long enough that a client sampling once per frame cannot miss a press, short
# enough to stay under any compositor's key-repeat delay.  A compositor that
# renders far slower than this needs `BBB_SEAT_INJECT_HOLD_SECONDS` raised --
# and its key repeat switched off, which the injector checks.
DEFAULT_HOLD_SECONDS = 0.4

# Gate key name -> keysym.  bbb reads physical keys, and both drivers hand the
# compositor a keysym that it maps back to an evdev scancode, so the physical
# layout the client observes matches this table for any US-layout seat.
KEYSYMS = {
    "forward": "w",
    "back": "s",
    "left": "a",
    "right": "d",
    "jump": "space",
    "sneak": "shift",
    "inventory": "e",
    "escape": "Escape",
    "chat": "t",
    "enter": "Return",
    "paste": "v",
    "fullscreen": "F11",
    **{f"hotbar{index}": str(index) for index in range(1, 10)},
}

# Letters the injector may type through the seat.  They are not gate keys --
# nothing binds them -- but the device must be able to reach them, so the
# stale-daemon check covers them too.
TYPEABLE_LETTERS = "abcdefghijklmnopqrstuvwxyz"

BUTTONS = {"left": "1", "middle": "2", "right": "3"}
WLROOTS_BUTTONS = {"left": "left", "middle": "middle", "right": "right"}


class InjectError(RuntimeError):
    """The injection path is unusable, so the caller must not claim receipts."""


def _run(command: list[str], env: dict[str, str] | None = None) -> str:
    binary = shutil.which(command[0])
    if binary is None:
        raise InjectError(f"{command[0]} is not installed on the injection host")
    completed = subprocess.run(
        [binary, *command[1:]],
        env={**os.environ, **(env or {})},
        capture_output=True,
        text=True,
        timeout=60,
    )
    if completed.returncode != 0:
        raise InjectError(
            f"{' '.join(command)} failed ({completed.returncode}): "
            f"{completed.stderr.strip() or completed.stdout.strip()}"
        )
    return completed.stdout.strip()


def _process_arguments(pid: object) -> list[str]:
    """The argv of a mapped window's process, or nothing if it is already gone."""

    if not isinstance(pid, int):
        return []
    try:
        raw = pathlib.Path(f"/proc/{pid}/cmdline").read_bytes()
    except OSError:
        return []
    return [part for part in raw.decode("utf-8", "replace").split("\0") if part]


def _keysym(name: str) -> str:
    keysym = KEYSYMS.get(name)
    if keysym is None:
        raise InjectError(f"no keysym is mapped for gate key {name!r}")
    return keysym


class WlrootsDriver:
    """Drive a wlroots compositor's seat through its virtual-input protocols."""

    name = "wlroots"

    def __init__(self, args: argparse.Namespace) -> None:
        self.hold_seconds = args.hold_seconds
        self.settle_seconds = args.settle_seconds
        self.output = args.output

    def _swaymsg(self, *command: str) -> str:
        return _run(["swaymsg", "-t", "command" if command[0] != "-t" else command[0], *command])

    def _tree(self) -> dict:
        raw = _run(["swaymsg", "-t", "get_tree", "-r"])
        try:
            return json.loads(raw)
        except json.JSONDecodeError as error:
            raise InjectError(f"could not read the compositor tree: {error}") from error

    def _toplevels(self) -> list[dict]:
        found: list[dict] = []

        def walk(node: dict) -> None:
            for key in ("nodes", "floating_nodes"):
                for child in node.get(key) or []:
                    walk(child)
            if node.get("type") in {"con", "floating_con"} and node.get("pid"):
                found.append(node)

        walk(self._tree())
        return found

    def _target(self) -> dict:
        """The window under test, not merely the newest one.

        A gate run maps more than one client -- the acceptance client that
        checks system-visible chat also gets a window -- and a wlroots
        compositor focuses whichever mapped last.  Driving the wrong one types
        into a client nobody is reading, so the target is identified by the
        control endpoint the gate drives it through.
        """

        toplevels = self._toplevels()
        if not toplevels:
            raise InjectError("the compositor has no toplevel window to drive")
        controlled = [
            window
            for window in toplevels
            if "--control" in _process_arguments(window.get("pid"))
        ]
        if len(controlled) == 1:
            return controlled[0]
        if not controlled:
            raise InjectError(
                "no mapped window belongs to a client with a control endpoint: "
                f"{[window.get('name') for window in toplevels]}"
            )
        raise InjectError(
            f"{len(controlled)} mapped windows carry a control endpoint, so the "
            "one under test is ambiguous"
        )

    def probe(self) -> dict[str, object]:
        # The virtual protocols are what make this driver worth choosing; a
        # compositor without them would silently reach nothing.
        for tool in ("swaymsg", "wtype", "wlrctl", "wl-copy"):
            if shutil.which(tool) is None:
                raise InjectError(f"{tool} is not installed on the injection host")
        window = self._target()
        return {
            "driver": self.name,
            "window": window.get("name"),
            "focused": window.get("focused"),
            "pid": window.get("pid"),
        }

    def focus(self) -> dict[str, object]:
        """A wlroots compositor focuses the toplevel it maps, so this only has
        to put focus back if something else took it."""

        window = self._target()
        if not window.get("focused"):
            _run(["swaymsg", f'[con_id={window["id"]}]', "focus"])
            time.sleep(self.settle_seconds)
        return {"driver": self.name, "window": window.get("name"), "focused": True}

    def key(self, name: str) -> None:
        self.focus()
        keysym = _keysym(name)
        _run(
            [
                "wtype",
                "-P", keysym,
                "-s", str(int(self.hold_seconds * 1000)),
                "-p", keysym,
            ]
        )
        time.sleep(self.settle_seconds)

    def chord(self, modifier: str, name: str) -> None:
        """Press a modified key -- the clipboard paste the gate needs."""

        self.focus()
        keysym = _keysym(name)
        _run(
            [
                "wtype",
                "-M", modifier,
                "-P", keysym,
                "-s", str(int(self.hold_seconds * 1000)),
                "-p", keysym,
                "-m", modifier,
            ]
        )
        time.sleep(self.settle_seconds)

    def move(self, dx: int, dy: int) -> None:
        self.focus()
        _run(["wlrctl", "pointer", "move", str(dx), str(dy)])
        time.sleep(self.settle_seconds)

    def click(self, button: str) -> None:
        self.focus()
        code = WLROOTS_BUTTONS.get(button)
        if code is None:
            raise InjectError(f"unknown pointer button {button!r}")
        _run(["wlrctl", "pointer", "click", code])
        time.sleep(self.settle_seconds)

    def text(self, characters: str) -> None:
        raise InjectError(
            "typing through the compositor's virtual keyboard would carry "
            "keysyms rather than physical keys; use the uinput driver"
        )

    def clipboard(self, text: str) -> None:
        """Put text on the seat's clipboard.

        `wl-copy` forks a daemon that stays alive to serve the selection, so
        its pipes must be discarded rather than read: waiting for them to close
        would wait for the selection to be replaced.
        """

        binary = shutil.which("wl-copy")
        if binary is None:
            raise InjectError("wl-copy is not installed on the injection host")
        completed = subprocess.run(
            [binary, "--", text],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=30,
        )
        if completed.returncode != 0:
            raise InjectError(f"wl-copy failed ({completed.returncode})")
        time.sleep(self.settle_seconds)
        if _run(["wl-paste", "--no-newline"]) != text:
            raise InjectError("the seat clipboard did not take the text")

    def output_scale(self, scale: str) -> None:
        _run(["swaymsg", "output", self.output, "scale", scale])
        time.sleep(self.settle_seconds)

    def window(self) -> dict[str, object]:
        """What the compositor believes about the window under test.

        The client's own surface size is the other half of a display-transition
        receipt; this is the half that says the compositor really made the
        change, rather than the client having resized itself.
        """

        target = self._target()
        rect = target.get("rect") or {}
        return {
            "driver": self.name,
            "window": target.get("name"),
            "logical_x": rect.get("x"),
            "logical_y": rect.get("y"),
            "logical_width": rect.get("width"),
            "logical_height": rect.get("height"),
            "fullscreen": bool(target.get("fullscreen_mode")),
            "floating": target.get("type") == "floating_con",
            "output_scale": self._output_scale(),
        }

    def _output_scale(self) -> float | None:
        raw = _run(["swaymsg", "-t", "get_outputs", "-r"])
        try:
            outputs = json.loads(raw)
        except json.JSONDecodeError as error:
            raise InjectError(f"could not read the compositor outputs: {error}") from error
        for output in outputs:
            if output.get("name") == self.output:
                return output.get("scale")
        raise InjectError(f"the compositor has no output named {self.output!r}")

    def window_geometry(self, width: int, height: int) -> None:
        """Pin the window to a fixed *logical* size before a scale change.

        A tiled window is sized in logical pixels to fill its workspace, and a
        scale change shrinks the workspace by exactly the factor it magnifies
        the buffer by -- so the physical surface never moves and the transition
        is unobservable.  A window with a fixed logical size is the desktop
        case the transition is about: same window, denser buffer.
        """

        target = self._target()
        selector = f"[con_id={target['id']}]"
        _run(["swaymsg", selector, "floating", "enable"])
        _run(["swaymsg", selector, "resize", "set", str(width), str(height)])
        time.sleep(self.settle_seconds)


class XtestDriver:
    """Drive a remote-desktop seat: XTEST -> remote-desktop client -> wl_seat."""

    name = "xtest"

    def __init__(self, args: argparse.Namespace) -> None:
        self.display = args.display
        self.window_class = args.window_class
        self.hold_seconds = args.hold_seconds
        self.settle_seconds = args.settle_seconds

    def _xdotool(self, *command: str) -> str:
        return _run(["xdotool", *command], env={"DISPLAY": self.display})

    def _window(self) -> str:
        found = self._xdotool("search", "--class", "--", self.window_class)
        windows = [line for line in found.splitlines() if line.strip()]
        if not windows:
            raise InjectError(
                f"no {self.window_class!r} window on {self.display}: the "
                "compositor's remote-desktop client is not attached"
            )
        return windows[-1]

    def _geometry(self, window: str) -> tuple[int, int, int, int]:
        output = self._xdotool("getwindowgeometry", "--shell", window)
        fields: dict[str, int] = {}
        for line in output.splitlines():
            key, _, value = line.partition("=")
            if value.strip().lstrip("-").isdigit():
                fields[key.strip()] = int(value)
        for key in ("X", "Y", "WIDTH", "HEIGHT"):
            if key not in fields:
                raise InjectError(f"could not read {key} of window {window}")
        return fields["X"], fields["Y"], fields["WIDTH"], fields["HEIGHT"]

    def _activate(self, window: str) -> None:
        try:
            self._xdotool("windowactivate", "--sync", window)
        except InjectError:
            # A bare display server without a window manager cannot activate,
            # but it can focus, and XTEST delivers to the focused window.
            pass
        self._xdotool("windowfocus", window)

    def probe(self) -> dict[str, object]:
        window = self._window()
        _, _, width, height = self._geometry(window)
        if width <= 0 or height <= 0:
            raise InjectError(f"window {window} has no viewport ({width}x{height})")
        return {
            "driver": self.name,
            "display": self.display,
            "window": window,
            "width": width,
            "height": height,
        }

    def focus(self) -> dict[str, object]:
        """The compositor gives keyboard focus to the surface that was clicked.

        The click lands near the bottom-left corner rather than the centre: a
        client that has been sitting unfocused shows its pause screen, whose
        buttons are all centred, so a corner click activates the surface
        without also pressing "Disconnect".  It must be the left button --
        weston binds click-to-activate to the left and right buttons only.
        """

        window = self._window()
        self._activate(window)
        x, y, width, height = self._geometry(window)
        spot = (x + max(8, width // 12), y + height - max(8, height // 12))
        self._xdotool("mousemove", str(spot[0]), str(spot[1]))
        time.sleep(self.settle_seconds)
        self._xdotool("click", BUTTONS["left"])
        time.sleep(self.settle_seconds)
        return {"driver": self.name, "x": spot[0], "y": spot[1]}

    def key(self, name: str) -> None:
        keysym = _keysym(name)
        window = self._window()
        self._activate(window)
        self._xdotool("keydown", keysym)
        time.sleep(self.hold_seconds)
        self._xdotool("keyup", keysym)
        time.sleep(self.settle_seconds)

    def chord(self, modifier: str, name: str) -> None:
        keysym = _keysym(name)
        window = self._window()
        self._activate(window)
        self._xdotool("keydown", modifier)
        self._xdotool("keydown", keysym)
        time.sleep(self.hold_seconds)
        self._xdotool("keyup", keysym)
        self._xdotool("keyup", modifier)
        time.sleep(self.settle_seconds)

    def move(self, dx: int, dy: int) -> None:
        window = self._window()
        self._activate(window)
        self._xdotool("mousemove_relative", "--", str(dx), str(dy))
        time.sleep(self.settle_seconds)

    def click(self, button: str) -> None:
        code = BUTTONS.get(button)
        if code is None:
            raise InjectError(f"unknown pointer button {button!r}")
        window = self._window()
        self._activate(window)
        self._xdotool("mousedown", code)
        time.sleep(self.hold_seconds)
        self._xdotool("mouseup", code)
        time.sleep(self.settle_seconds)

    def text(self, characters: str) -> None:
        raise InjectError(
            "the remote-desktop driver cannot type through the seat; use the "
            "uinput driver"
        )

    def clipboard(self, text: str) -> None:
        raise InjectError(
            "the remote-desktop driver has no seat clipboard; use the wlroots "
            "driver for clipboard receipts"
        )

    def output_scale(self, scale: str) -> None:
        raise InjectError(
            "the remote-desktop driver cannot change output scale; use the "
            "wlroots driver"
        )

    def window(self) -> dict[str, object]:
        raise InjectError(
            "the remote-desktop driver cannot report compositor window state; "
            "use the wlroots driver"
        )

    def window_geometry(self, width: int, height: int) -> None:
        raise InjectError(
            "the remote-desktop driver cannot pin window geometry; use the "
            "wlroots driver"
        )


class UinputDriver:
    """Drive a compositor seat through a held-open virtual input device.

    This is the faithful path: `scripts/seat_device.py` keeps one uinput
    device open for the whole run, so the seat's keyboard and pointer
    capabilities never blink and the client binds and holds focus exactly as it
    would with a physical seat.  The events carry real evdev scancodes, so the
    physical key the client reads is the physical key the gate asked for, and
    pointer motion is genuinely relative -- which is what an in-game locked
    cursor needs.

    Window focus stays a compositor matter: it focuses what it maps, and this
    driver only puts focus back when a later client has taken it.
    """

    name = "uinput"

    def __init__(self, args: argparse.Namespace) -> None:
        self.hold_seconds = args.hold_seconds
        self.settle_seconds = args.settle_seconds
        self.socket_path = args.device_socket
        self._wlroots = WlrootsDriver(args)

    def _send(self, *words: str) -> None:
        seat_device.request(self.socket_path, " ".join(words))
        time.sleep(self.settle_seconds)

    def probe(self) -> dict[str, object]:
        seat_device.request(self.socket_path, "ping")
        self._require_current_key_table()
        window = self._wlroots.probe()
        keyboard = self._zero_key_repeat_receipt(configure=True)
        pointer = self._flat_pointer_profile_receipt(configure=True)
        return {
            **window,
            **keyboard,
            **pointer,
            "driver": self.name,
            "device_socket": self.socket_path,
        }

    def _require_current_key_table(self) -> None:
        """Refuse a daemon whose key table predates this injector.

        The device is deliberately long-lived, which means it keeps running the
        code it was started with.  A key added since then is missing from it,
        and without this check the run discovers that only when it presses the
        key -- tens of minutes in, and reported as a seat failure rather than
        as the stale daemon it is.
        """

        answer = seat_device.request(self.socket_path, "keys")
        served = set(answer.split()[1:])
        missing = sorted((set(KEYSYMS) | set(TYPEABLE_LETTERS)) - served)
        if missing:
            raise InjectError(
                f"the running seat device cannot press {missing}; restart "
                f"{seat_device.__file__} so it picks up the current key table"
            )

    def _zero_key_repeat_receipt(self, configure: bool) -> dict[str, object]:
        """Keep held gate presses from turning into repeated toggles.

        The gate holds keys deliberately, long enough for a client rendering
        slowly to sample them.  Under the usual 600ms/25Hz repeat that one hold
        becomes twenty-odd presses, which leaves a toggle such as the pause
        screen back where it started.  The gate owns its virtual keyboard, so
        apply the no-repeat setting to that exact compositor identifier and
        read it back instead of relying on when an external sway config ran.
        """

        device_name = seat_device.DEVICE_NAME.decode("ascii")

        def read_devices() -> list[dict]:
            raw = _run(["swaymsg", "-t", "get_inputs", "-r"])
            try:
                devices = json.loads(raw)
            except json.JSONDecodeError as error:
                raise InjectError(f"could not read the compositor inputs: {error}") from error
            if not isinstance(devices, list):
                raise InjectError("the compositor input inventory was not a list")
            return [
                device
                for device in devices
                if device.get("type") == "keyboard" and device.get("name") == device_name
            ]

        devices = read_devices()
        if not devices:
            raise InjectError(
                f"the compositor has no {device_name!r} keyboard from the held-open seat"
            )
        identifiers = sorted(
            {
                identifier
                for device in devices
                if isinstance((identifier := device.get("identifier")), str)
                and identifier
            }
        )
        if not identifiers:
            raise InjectError(f"the compositor did not identify its {device_name!r} keyboard")

        if configure and any(device.get("repeat_rate") != 0 for device in devices):
            for identifier in identifiers:
                _run(["swaymsg", "input", identifier, "repeat_rate", "0"])
            devices = read_devices()

        repeat_rates = {device.get("repeat_rate") for device in devices}
        if repeat_rates != {0}:
            raise InjectError(
                f"the {device_name!r} keyboard needs repeat_rate 0 for held gate "
                f"presses, got {sorted(str(rate) for rate in repeat_rates)}"
            )
        return {
            "keyboard_repeat_rate": 0,
            "keyboard_identifiers": identifiers,
        }

    def _flat_pointer_profile_receipt(self, configure: bool) -> dict[str, object]:
        """Make relative uinput deltas map to reproducible logical pixels.

        Libinput's default adaptive acceleration scales the same evdev delta by
        its delivery speed.  That is correct for ordinary mouse use but makes
        an exact GUI hit target non-reproducible: a requested 666px move was
        observed as roughly twice that distance on the release host.  The
        platform gate owns this virtual device, so configure its compositor
        profile to flat and then independently read it back before claiming an
        exact pointer receipt.
        """

        device_name = seat_device.DEVICE_NAME.decode("ascii")

        def read_devices() -> list[dict]:
            raw = _run(["swaymsg", "-t", "get_inputs", "-r"])
            try:
                devices = json.loads(raw)
            except json.JSONDecodeError as error:
                raise InjectError(f"could not read the compositor inputs: {error}") from error
            if not isinstance(devices, list):
                raise InjectError("the compositor input inventory was not a list")
            return [
                device
                for device in devices
                if device.get("type") == "pointer" and device.get("name") == device_name
            ]

        devices = read_devices()
        if not devices:
            raise InjectError(
                f"the compositor has no {device_name!r} pointer from the held-open seat"
            )
        identifiers = sorted(
            {
                identifier
                for device in devices
                if isinstance((identifier := device.get("identifier")), str)
                and identifier
            }
        )
        if not identifiers:
            raise InjectError(f"the compositor did not identify its {device_name!r} pointer")

        if configure and any(
            (device.get("libinput") or {}).get("accel_profile") != "flat"
            for device in devices
        ):
            for identifier in identifiers:
                _run(["swaymsg", "input", identifier, "accel_profile", "flat"])
            devices = read_devices()

        profiles = {
            (device.get("libinput") or {}).get("accel_profile") for device in devices
        }
        if profiles != {"flat"}:
            raise InjectError(
                f"the {device_name!r} pointer needs flat acceleration for exact GUI "
                f"coordinates, got {sorted(str(profile) for profile in profiles)}"
            )
        return {
            "pointer_accel_profile": "flat",
            "pointer_identifiers": identifiers,
        }

    def focus(self) -> dict[str, object]:
        return {**self._wlroots.focus(), "driver": self.name}

    def key(self, name: str) -> None:
        self.focus()
        self._send("key", name, str(self.hold_seconds))

    def chord(self, modifier: str, name: str) -> None:
        self.focus()
        self._send("chord", modifier, name, str(self.hold_seconds))

    def move(self, dx: int, dy: int) -> None:
        self.focus()
        self._send("move", str(dx), str(dy))

    def click(self, button: str) -> None:
        self.focus()
        self._send("click", button, str(self.hold_seconds))

    def text(self, characters: str) -> None:
        self.focus()
        self._send("text", characters)

    def clipboard(self, text: str) -> None:
        self._wlroots.clipboard(text)

    def output_scale(self, scale: str) -> None:
        self._wlroots.output_scale(scale)

    def window(self) -> dict[str, object]:
        return {
            **self._wlroots.window(),
            **self._zero_key_repeat_receipt(configure=False),
            **self._flat_pointer_profile_receipt(configure=False),
            "driver": self.name,
        }

    def window_geometry(self, width: int, height: int) -> None:
        self._wlroots.window_geometry(width, height)


DRIVERS = {"uinput": UinputDriver, "wlroots": WlrootsDriver, "xtest": XtestDriver}


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--driver",
        choices=sorted(DRIVERS),
        default=os.environ.get("BBB_SEAT_INJECT_DRIVER", "uinput"),
    )
    parser.add_argument(
        "--display", default=os.environ.get("BBB_SEAT_INJECT_DISPLAY", DEFAULT_DISPLAY)
    )
    parser.add_argument(
        "--window-class",
        default=os.environ.get("BBB_SEAT_INJECT_WINDOW", DEFAULT_WINDOW_CLASS),
    )
    parser.add_argument(
        "--output", default=os.environ.get("BBB_SEAT_INJECT_OUTPUT", "HEADLESS-1")
    )
    parser.add_argument(
        "--device-socket",
        default=os.environ.get("BBB_SEAT_DEVICE_SOCKET", "/tmp/bbb-seat.sock"),
    )
    parser.add_argument("--settle-seconds", type=float, default=0.25)
    parser.add_argument(
        "--hold-seconds",
        type=float,
        default=float(
            os.environ.get("BBB_SEAT_INJECT_HOLD_SECONDS", str(DEFAULT_HOLD_SECONDS))
        ),
    )
    sub = parser.add_subparsers(dest="action", required=True)
    sub.add_parser("probe")
    sub.add_parser("focus")
    key_parser = sub.add_parser("key")
    key_parser.add_argument("name")
    chord_parser = sub.add_parser("chord")
    chord_parser.add_argument("modifier")
    chord_parser.add_argument("name")
    move_parser = sub.add_parser("move")
    move_parser.add_argument("dx", type=int)
    move_parser.add_argument("dy", type=int)
    click_parser = sub.add_parser("click")
    click_parser.add_argument("button", choices=sorted(BUTTONS))
    text_parser = sub.add_parser("text")
    text_parser.add_argument("characters")
    clipboard_parser = sub.add_parser("clipboard")
    clipboard_parser.add_argument("text")
    scale_parser = sub.add_parser("output-scale")
    scale_parser.add_argument("scale")
    sub.add_parser("window")
    geometry_parser = sub.add_parser("window-geometry")
    geometry_parser.add_argument("width", type=int)
    geometry_parser.add_argument("height", type=int)
    return parser


def require_linux_host() -> None:
    """Refuse attributably off Linux instead of failing on a missing device.

    A macOS developer running the full dev chain will meet this script; the
    uinput device and the wlroots compositor it drives exist nowhere else.
    """
    if sys.platform != "linux":
        raise InjectError(
            f"seat injection needs a Linux uinput seat; this host is {sys.platform}. "
            "Seat-driven gates run on the declared Linux release host only"
        )


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        require_linux_host()
    except InjectError as error:
        print(f"seat injection failed: {error}", file=sys.stderr)
        return 1
    driver = DRIVERS[args.driver](args)
    try:
        if args.action == "probe":
            print(json.dumps(driver.probe(), sort_keys=True))
        elif args.action == "focus":
            print(json.dumps(driver.focus(), sort_keys=True))
        elif args.action == "key":
            driver.key(args.name)
        elif args.action == "chord":
            driver.chord(args.modifier, args.name)
        elif args.action == "move":
            driver.move(args.dx, args.dy)
        elif args.action == "click":
            driver.click(args.button)
        elif args.action == "text":
            driver.text(args.characters)
        elif args.action == "clipboard":
            driver.clipboard(args.text)
        elif args.action == "output-scale":
            driver.output_scale(args.scale)
        elif args.action == "window":
            print(json.dumps(driver.window(), sort_keys=True))
        elif args.action == "window-geometry":
            driver.window_geometry(args.width, args.height)
        return 0
    except InjectError as error:
        print(f"seat injection failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
