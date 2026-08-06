"""macOS development input through the system CoreGraphics API."""

from __future__ import annotations

import ctypes
import subprocess
import sys
from ctypes import c_bool, c_double, c_uint16, c_uint32, c_void_p
from ctypes.util import find_library

from .contract import InputBackend, InputError

KEY_CODES = {
    "a": 0,
    "s": 1,
    "d": 2,
    "f": 3,
    "h": 4,
    "g": 5,
    "z": 6,
    "x": 7,
    "c": 8,
    "v": 9,
    "b": 11,
    "q": 12,
    "w": 13,
    "e": 14,
    "r": 15,
    "y": 16,
    "t": 17,
    "1": 18,
    "2": 19,
    "3": 20,
    "4": 21,
    "5": 23,
    "6": 22,
    "7": 26,
    "8": 28,
    "9": 25,
    "space": 49,
    "enter": 36,
    "escape": 53,
    "shift": 56,
    "ctrl": 59,
    "f11": 103,
}
BUTTONS = {"left": 0, "right": 1, "middle": 2}


class CGPoint(ctypes.Structure):
    _fields_ = [("x", c_double), ("y", c_double)]


class MacOsQuartzBackend(InputBackend):
    def __init__(self) -> None:
        if sys.platform != "darwin":
            raise InputError("Quartz input is available only on macOS")
        super().__init__()
        library = find_library("ApplicationServices")
        if library is None:
            raise InputError("ApplicationServices.framework is unavailable")
        self._api = ctypes.CDLL(library)
        self._api.CGEventCreateKeyboardEvent.restype = c_void_p
        self._api.CGEventCreateKeyboardEvent.argtypes = [c_void_p, c_uint16, c_bool]
        self._api.CGEventCreateMouseEvent.restype = c_void_p
        self._api.CGEventCreateMouseEvent.argtypes = [c_void_p, c_uint32, CGPoint, c_uint32]
        self._api.CGEventCreate.restype = c_void_p
        self._api.CGEventCreate.argtypes = [c_void_p]
        self._api.CGEventGetLocation.restype = CGPoint
        self._api.CGEventGetLocation.argtypes = [c_void_p]
        self._api.CGEventPost.argtypes = [c_uint32, c_void_p]
        self._api.CFRelease.argtypes = [c_void_p]

    def _post(self, event: int) -> None:
        if not event:
            raise InputError("CoreGraphics could not create an input event")
        try:
            self._api.CGEventPost(0, event)
        finally:
            self._api.CFRelease(event)

    def key_event(self, key: str, pressed: bool) -> None:
        try:
            code = KEY_CODES[key.lower()]
        except KeyError as error:
            raise InputError(f"no macOS key code for {key!r}") from error
        self._post(self._api.CGEventCreateKeyboardEvent(None, code, pressed))

    def _pointer_location(self) -> CGPoint:
        event = self._api.CGEventCreate(None)
        if not event:
            raise InputError("CoreGraphics could not read the pointer location")
        try:
            return self._api.CGEventGetLocation(event)
        finally:
            self._api.CFRelease(event)

    def button_event(self, button: str, pressed: bool) -> None:
        try:
            code = BUTTONS[button]
        except KeyError as error:
            raise InputError(f"unknown macOS mouse button {button!r}") from error
        event_types = {
            0: (1, 2),
            1: (3, 4),
            2: (25, 26),
        }
        event_type = event_types[code][0 if pressed else 1]
        self._post(
            self._api.CGEventCreateMouseEvent(None, event_type, self._pointer_location(), code)
        )

    def move(self, dx: float, dy: float) -> None:
        current = self._pointer_location()
        destination = CGPoint(current.x + dx, current.y + dy)
        self._post(self._api.CGEventCreateMouseEvent(None, 5, destination, 0))

    @staticmethod
    def focus_pid(pid: int) -> None:
        script = (
            'tell application "System Events" to set frontmost of '
            f"first process whose unix id is {pid} to true"
        )
        completed = subprocess.run(
            ["/usr/bin/osascript", "-e", script], capture_output=True, text=True
        )
        if completed.returncode != 0:
            raise InputError(completed.stderr.strip() or "cannot focus macOS process")

    @staticmethod
    def clipboard(text: str) -> None:
        completed = subprocess.run(["/usr/bin/pbcopy"], input=text, text=True, capture_output=True)
        if completed.returncode != 0:
            raise InputError(completed.stderr.strip() or "pbcopy failed")
