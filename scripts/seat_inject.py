#!/usr/bin/env python3
"""Inject input into the playable gate's client through a real compositor seat.

The live playable gate normally synthesises input through the ``input.*``
control-plane requests, which replay the canonical handlers but never exercise
the platform's own event path.  This injector closes that gap: it drives a
display server, which drives a remote-desktop client, which is a Wayland client
of the compositor under test, so the compositor delivers ``wl_seat`` focus,
keyboard and pointer events to the gate's client exactly as a physical seat
would.

    XTEST (this script) -> RDP client window -> compositor's RDP backend
        -> wl_seat -> bbb-native

The two hosts are configured out of band; this script only needs to know where
the remote-desktop client window lives:

``BBB_SEAT_INJECT_DISPLAY``
    X display hosting the remote-desktop client window (default ``:79``).
``BBB_SEAT_INJECT_WINDOW``
    Window class to search for on that display (default ``freerdp``).

Keyboard names are the gate's own key names (``hotbar4``, ``forward``, ...) so
a seat-driven scenario reads the same as the control-driven ones.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time

DEFAULT_DISPLAY = ":79"
DEFAULT_WINDOW_CLASS = "freerdp"

# Gate key name -> X keysym.  bbb reads physical keys, and the keysym a
# display server sends is translated back into an evdev scancode by the
# compositor, so the physical layout the client observes matches this table
# for any US-layout seat.
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
    **{f"hotbar{index}": str(index) for index in range(1, 10)},
}

BUTTONS = {"left": "1", "middle": "2", "right": "3"}


class InjectError(RuntimeError):
    """The injection path is unusable, so the caller must not claim receipts."""


def _xdotool(display: str, *args: str) -> str:
    binary = shutil.which("xdotool")
    if binary is None:
        raise InjectError("xdotool is not installed on the injection host")
    completed = subprocess.run(
        [binary, *args],
        env={**os.environ, "DISPLAY": display},
        capture_output=True,
        text=True,
        timeout=30,
    )
    if completed.returncode != 0:
        raise InjectError(
            f"xdotool {' '.join(args)} failed ({completed.returncode}): "
            f"{completed.stderr.strip()}"
        )
    return completed.stdout.strip()


def find_window(display: str, window_class: str) -> str:
    """Return the remote-desktop client window that fronts the compositor."""

    found = _xdotool(display, "search", "--class", "--", window_class)
    windows = [line for line in found.splitlines() if line.strip()]
    if not windows:
        raise InjectError(
            f"no {window_class!r} window on {display}: the compositor's "
            "remote-desktop client is not attached"
        )
    return windows[-1]


def _window_geometry(display: str, window: str) -> tuple[int, int, int, int]:
    output = _xdotool(display, "getwindowgeometry", "--shell", window)
    fields: dict[str, int] = {}
    for line in output.splitlines():
        key, _, value = line.partition("=")
        if value.strip().lstrip("-").isdigit():
            fields[key.strip()] = int(value)
    for key in ("X", "Y", "WIDTH", "HEIGHT"):
        if key not in fields:
            raise InjectError(f"could not read {key} of window {window}")
    return fields["X"], fields["Y"], fields["WIDTH"], fields["HEIGHT"]


def activate(display: str, window: str) -> None:
    """Give the remote-desktop client window the display server's focus."""

    try:
        _xdotool(display, "windowactivate", "--sync", window)
    except InjectError:
        # A bare display server without a window manager cannot activate, but
        # it can still focus, and XTEST delivers to the focused window.
        pass
    _xdotool(display, "windowfocus", window)


def focus(display: str, window: str, settle_seconds: float) -> dict[str, int]:
    """Hand the compositor's keyboard focus to the client under test.

    The compositor gives keyboard focus to the surface under the pointer when
    it is clicked, so a click inside the remote-desktop viewport is what makes
    the seat's keyboard events reach the client.  It has to be the left button:
    weston binds click-to-activate to the left and right buttons only, so a
    middle click moves no focus at all.

    The click lands near the bottom-left corner rather than the centre.  A
    client that has been sitting unfocused is showing its pause screen, whose
    buttons are all centred, so a corner click activates the surface without
    also pressing "Disconnect".
    """

    activate(display, window)
    x, y, width, height = _window_geometry(display, window)
    spot = (x + max(8, width // 12), y + height - max(8, height // 12))
    _xdotool(display, "mousemove", str(spot[0]), str(spot[1]))
    time.sleep(settle_seconds)
    _xdotool(display, "click", BUTTONS["left"])
    time.sleep(settle_seconds)
    return {"x": spot[0], "y": spot[1], "width": width, "height": height}


def key(display: str, window: str, name: str, settle_seconds: float, hold_seconds: float) -> None:
    """Press and release a key, holding it the way a person would.

    `xdotool key` releases about twelve milliseconds after it presses.  A
    client rendering at half a frame per second -- which this one does while it
    uploads terrain -- can advance a whole frame between the two events and
    never sample the key as down, so the hold has to outlast a frame rather
    than a human reaction.
    """

    keysym = KEYSYMS.get(name)
    if keysym is None:
        raise InjectError(f"no keysym is mapped for gate key {name!r}")
    activate(display, window)
    _xdotool(display, "keydown", keysym)
    time.sleep(hold_seconds)
    _xdotool(display, "keyup", keysym)
    time.sleep(settle_seconds)


def move(display: str, window: str, dx: int, dy: int, settle_seconds: float) -> None:
    """Move the pointer by a relative delta inside the client's viewport."""

    activate(display, window)
    _xdotool(display, "mousemove_relative", "--", str(dx), str(dy))
    time.sleep(settle_seconds)


def click(display: str, window: str, button: str, settle_seconds: float, hold_seconds: float) -> None:
    """Press and release a pointer button, held for the same reason keys are."""

    code = BUTTONS.get(button)
    if code is None:
        raise InjectError(f"unknown pointer button {button!r}")
    activate(display, window)
    _xdotool(display, "mousedown", code)
    time.sleep(hold_seconds)
    _xdotool(display, "mouseup", code)
    time.sleep(settle_seconds)


def probe(display: str, window_class: str) -> dict[str, object]:
    """Check the whole injection path before a gate run depends on it."""

    window = find_window(display, window_class)
    x, y, width, height = _window_geometry(display, window)
    if width <= 0 or height <= 0:
        raise InjectError(f"window {window} has no viewport ({width}x{height})")
    return {"display": display, "window": window, "width": width, "height": height}


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--display", default=os.environ.get("BBB_SEAT_INJECT_DISPLAY", DEFAULT_DISPLAY))
    parser.add_argument(
        "--window-class",
        default=os.environ.get("BBB_SEAT_INJECT_WINDOW", DEFAULT_WINDOW_CLASS),
    )
    parser.add_argument("--settle-seconds", type=float, default=0.25)
    parser.add_argument(
        "--hold-seconds",
        type=float,
        default=float(os.environ.get("BBB_SEAT_INJECT_HOLD_SECONDS", "1.5")),
    )
    sub = parser.add_subparsers(dest="action", required=True)
    sub.add_parser("probe")
    sub.add_parser("focus")
    key_parser = sub.add_parser("key")
    key_parser.add_argument("name")
    move_parser = sub.add_parser("move")
    move_parser.add_argument("dx", type=int)
    move_parser.add_argument("dy", type=int)
    click_parser = sub.add_parser("click")
    click_parser.add_argument("button", choices=sorted(BUTTONS))
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        if args.action == "probe":
            print(probe(args.display, args.window_class))
            return 0
        window = find_window(args.display, args.window_class)
        if args.action == "focus":
            print(focus(args.display, window, args.settle_seconds))
        elif args.action == "key":
            key(args.display, window, args.name, args.settle_seconds, args.hold_seconds)
        elif args.action == "move":
            move(args.display, window, args.dx, args.dy, args.settle_seconds)
        elif args.action == "click":
            click(
                args.display, window, args.button, args.settle_seconds, args.hold_seconds
            )
        return 0
    except InjectError as error:
        print(f"seat injection failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
