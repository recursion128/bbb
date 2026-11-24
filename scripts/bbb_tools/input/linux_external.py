"""Mature Linux injection adapters; release receipts still require uinput."""

from __future__ import annotations

import shutil
import subprocess
import sys

from .contract import InputBackend, InputError

KEY_CODES = {
    "escape": 1,
    "enter": 28,
    "ctrl": 29,
    "shift": 42,
    "space": 57,
    "w": 17,
    "s": 31,
    "a": 30,
    "d": 32,
    "e": 18,
    "t": 20,
    "v": 47,
    "f11": 87,
    **{str(index): index + 1 for index in range(1, 10)},
}


class YdotoolBackend(InputBackend):
    """Drive the persistent ydotoold uinput device without owning its ABI."""

    def __init__(self) -> None:
        if not sys.platform.startswith("linux"):
            raise InputError("ydotool is available only on Linux")
        super().__init__()
        self.binary = shutil.which("ydotool")
        if self.binary is None:
            raise InputError("ydotool is not installed")

    def _run(self, *arguments: str) -> None:
        completed = subprocess.run(
            [self.binary, *arguments], capture_output=True, text=True, timeout=30
        )
        if completed.returncode != 0:
            raise InputError(completed.stderr.strip() or "ydotool failed")

    def key_event(self, key: str, pressed: bool) -> None:
        try:
            code = KEY_CODES[key.lower()]
        except KeyError as error:
            raise InputError(f"no Linux evdev code for {key!r}") from error
        self._run("key", f"{code}:{1 if pressed else 0}")

    def button_event(self, button: str, pressed: bool) -> None:
        buttons = {"left": 0, "right": 1, "middle": 2}
        try:
            value = buttons[button]
        except KeyError as error:
            raise InputError(f"unknown ydotool button {button!r}") from error
        flag = (0x40 if pressed else 0x80) | value
        self._run("click", hex(flag))

    def move(self, dx: float, dy: float) -> None:
        self._run("mousemove", "--", str(round(dx)), str(round(dy)))
