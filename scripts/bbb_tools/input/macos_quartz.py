"""macOS development input through the system CoreGraphics API."""

from __future__ import annotations

import ctypes
import subprocess
import sys
import time
from ctypes import c_bool, c_double, c_uint16, c_uint32, c_void_p
from ctypes.util import find_library
from dataclasses import dataclass

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
    "command": 55,
    "ctrl": 59,
    "f11": 103,
}
BUTTONS = {"left": 0, "right": 1, "middle": 2}
AX_ERROR_ATTRIBUTE_UNSUPPORTED = -25205


@dataclass(frozen=True)
class MacOsWindowPresentation:
    """Accessibility receipt for the exact native window targeted by a run."""

    pid: int
    frontmost: bool
    position: tuple[float, float]
    size: tuple[float, float]


def _require_accessibility_status(
    operation: str, status: int, *, optional_attribute: bool = False
) -> bool:
    """Validate an AX operation while tolerating a genuinely absent attribute.

    Cocoa windows are not required to expose AXMain or AXFocused as writable
    attributes.  They can still be raised and receive input once their process
    is frontmost.  All other failures remain fatal so platform acceptance never
    substitutes a synthetic focus receipt for a failed native activation.
    """

    if status == 0:
        return True
    if optional_attribute and status == AX_ERROR_ATTRIBUTE_UNSUPPORTED:
        return False
    raise InputError(f"macOS window was not {operation} ({status})")


class CGPoint(ctypes.Structure):
    _fields_ = [("x", c_double), ("y", c_double)]


class CGSize(ctypes.Structure):
    _fields_ = [("width", c_double), ("height", c_double)]


class ProcessSerialNumber(ctypes.Structure):
    _fields_ = [("high", c_uint32), ("low", c_uint32)]


class MacOsQuartzBackend(InputBackend):
    def __init__(self, pid: int | None = None) -> None:
        if sys.platform != "darwin":
            raise InputError("Quartz input is available only on macOS")
        super().__init__()
        self._pid = pid
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
        self._api.CGEventSetIntegerValueField.argtypes = [
            c_void_p,
            c_uint32,
            ctypes.c_int64,
        ]
        self._api.CGEventSetFlags.argtypes = [c_void_p, ctypes.c_uint64]
        self._api.CGEventPost.argtypes = [c_uint32, c_void_p]
        self._api.CGEventPostToPid.argtypes = [ctypes.c_int, c_void_p]
        self._api.CFRelease.argtypes = [c_void_p]
        self._api.GetProcessForPID.argtypes = [
            ctypes.c_int,
            ctypes.POINTER(ProcessSerialNumber),
        ]
        self._api.GetProcessForPID.restype = ctypes.c_int32
        self._api.SetFrontProcessWithOptions.argtypes = [
            ctypes.POINTER(ProcessSerialNumber),
            c_uint32,
        ]
        self._api.SetFrontProcessWithOptions.restype = ctypes.c_int32
        core_foundation = find_library("CoreFoundation")
        if core_foundation is None:
            raise InputError("CoreFoundation.framework is unavailable")
        self._cf = ctypes.CDLL(core_foundation)
        self._cf.CFStringCreateWithCString.argtypes = [
            c_void_p,
            ctypes.c_char_p,
            c_uint32,
        ]
        self._cf.CFStringCreateWithCString.restype = c_void_p
        self._cf.CFArrayGetCount.argtypes = [c_void_p]
        self._cf.CFArrayGetCount.restype = ctypes.c_long
        self._cf.CFArrayGetValueAtIndex.argtypes = [c_void_p, ctypes.c_long]
        self._cf.CFArrayGetValueAtIndex.restype = c_void_p
        self._cf.CFBooleanGetValue.argtypes = [c_void_p]
        self._cf.CFBooleanGetValue.restype = c_bool
        self._cf.CFRelease.argtypes = [c_void_p]
        self._api.AXUIElementCreateApplication.argtypes = [ctypes.c_int]
        self._api.AXUIElementCreateApplication.restype = c_void_p
        self._api.AXUIElementSetAttributeValue.argtypes = [c_void_p, c_void_p, c_void_p]
        self._api.AXUIElementSetAttributeValue.restype = ctypes.c_int32
        self._api.AXUIElementCopyAttributeValue.argtypes = [
            c_void_p,
            c_void_p,
            ctypes.POINTER(c_void_p),
        ]
        self._api.AXUIElementCopyAttributeValue.restype = ctypes.c_int32
        self._api.AXUIElementPerformAction.argtypes = [c_void_p, c_void_p]
        self._api.AXUIElementPerformAction.restype = ctypes.c_int32
        self._api.AXValueGetValue.argtypes = [c_void_p, c_uint32, c_void_p]
        self._api.AXValueGetValue.restype = c_bool

    def _post(self, event: int, *, application_targeted: bool = False) -> None:
        if not event:
            raise InputError("CoreGraphics could not create an input event")
        try:
            if application_targeted and self._pid is not None and self._pid > 0:
                self._api.CGEventPostToPid(self._pid, event)
            else:
                self._api.CGEventPost(0, event)
        finally:
            self._api.CFRelease(event)

    def key_event(self, key: str, pressed: bool) -> None:
        try:
            code = KEY_CODES[key.lower()]
        except KeyError as error:
            raise InputError(f"no macOS key code for {key!r}") from error
        event = self._api.CGEventCreateKeyboardEvent(None, code, pressed)
        if event and (
            (key.lower() == "command" and pressed)
            or (key.lower() != "command" and "command" in self._pressed_keys)
        ):
            self._api.CGEventSetFlags(event, 1 << 20)
        self._post(event, application_targeted=True)

    def _pointer_location(self) -> CGPoint:
        event = self._api.CGEventCreate(None)
        if not event:
            raise InputError("CoreGraphics could not read the pointer location")
        try:
            return self._api.CGEventGetLocation(event)
        finally:
            self._api.CFRelease(event)

    def _window_center(self) -> CGPoint:
        if self._pid is None or self._pid <= 0:
            raise InputError("reading a macOS window requires a positive pid")
        attributes = {
            name: self._cf.CFStringCreateWithCString(
                None, name.encode("ascii"), 0x08000100
            )
            for name in ("AXWindows", "AXPosition", "AXSize")
        }
        application = self._api.AXUIElementCreateApplication(self._pid)
        windows = c_void_p()
        position_value = c_void_p()
        size_value = c_void_p()
        try:
            if not all(attributes.values()) or not application:
                raise InputError("macOS could not open the window Accessibility bridge")
            status = self._api.AXUIElementCopyAttributeValue(
                application, attributes["AXWindows"], ctypes.byref(windows)
            )
            if status != 0 or not windows.value or self._cf.CFArrayGetCount(windows) < 1:
                raise InputError(f"macOS could not read process {self._pid}'s window")
            window = self._cf.CFArrayGetValueAtIndex(windows, 0)
            for name, output in (("AXPosition", position_value), ("AXSize", size_value)):
                status = self._api.AXUIElementCopyAttributeValue(
                    window, attributes[name], ctypes.byref(output)
                )
                if status != 0 or not output.value:
                    raise InputError(
                        f"macOS could not read process {self._pid}'s {name} ({status})"
                    )
            position = CGPoint()
            size = CGSize()
            if not self._api.AXValueGetValue(position_value, 1, ctypes.byref(position)):
                raise InputError("macOS window position had the wrong Accessibility type")
            if not self._api.AXValueGetValue(size_value, 2, ctypes.byref(size)):
                raise InputError("macOS window size had the wrong Accessibility type")
            return CGPoint(
                position.x + size.width / 2.0,
                position.y + size.height / 2.0,
            )
        finally:
            for value in (size_value, position_value, windows):
                if value.value:
                    self._cf.CFRelease(value)
            if application:
                self._cf.CFRelease(application)
            for value in attributes.values():
                if value:
                    self._cf.CFRelease(value)

    def window_presentation(self) -> MacOsWindowPresentation:
        """Read the frontmost state and geometry of this PID's first window.

        The formal Retina runner uses this after ``focus_pid``. A drawable-sized
        F2 image proves render-target pixels, while this separate receipt proves
        that those pixels belong to the frontmost native window rather than an
        occluded, half-sized, or otherwise non-presented surface.
        """

        if self._pid is None or self._pid <= 0:
            raise InputError("reading a macOS window requires a positive pid")
        attributes = {
            name: self._cf.CFStringCreateWithCString(
                None, name.encode("ascii"), 0x08000100
            )
            for name in ("AXFrontmost", "AXWindows", "AXPosition", "AXSize")
        }
        application = self._api.AXUIElementCreateApplication(self._pid)
        windows = c_void_p()
        frontmost_value = c_void_p()
        position_value = c_void_p()
        size_value = c_void_p()
        try:
            if not all(attributes.values()) or not application:
                raise InputError("macOS could not open the window presentation bridge")
            status = self._api.AXUIElementCopyAttributeValue(
                application, attributes["AXFrontmost"], ctypes.byref(frontmost_value)
            )
            if status != 0 or not frontmost_value.value:
                raise InputError(
                    f"macOS could not read process {self._pid}'s frontmost state ({status})"
                )
            frontmost = bool(self._cf.CFBooleanGetValue(frontmost_value))
            status = self._api.AXUIElementCopyAttributeValue(
                application, attributes["AXWindows"], ctypes.byref(windows)
            )
            if status != 0 or not windows.value or self._cf.CFArrayGetCount(windows) < 1:
                raise InputError(f"macOS could not read process {self._pid}'s window")
            window = self._cf.CFArrayGetValueAtIndex(windows, 0)
            for name, output in (("AXPosition", position_value), ("AXSize", size_value)):
                status = self._api.AXUIElementCopyAttributeValue(
                    window, attributes[name], ctypes.byref(output)
                )
                if status != 0 or not output.value:
                    raise InputError(
                        f"macOS could not read process {self._pid}'s {name} ({status})"
                    )
            position = CGPoint()
            size = CGSize()
            if not self._api.AXValueGetValue(position_value, 1, ctypes.byref(position)):
                raise InputError("macOS window position had the wrong Accessibility type")
            if not self._api.AXValueGetValue(size_value, 2, ctypes.byref(size)):
                raise InputError("macOS window size had the wrong Accessibility type")
            return MacOsWindowPresentation(
                pid=self._pid,
                frontmost=frontmost,
                position=(position.x, position.y),
                size=(size.width, size.height),
            )
        finally:
            for value in (size_value, position_value, frontmost_value, windows):
                if value.value:
                    self._cf.CFRelease(value)
            if application:
                self._cf.CFRelease(application)
            for value in attributes.values():
                if value:
                    self._cf.CFRelease(value)

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
        event = self._api.CGEventCreateMouseEvent(
            None, event_type, self._window_center(), code
        )
        if event:
            self._api.CGEventSetIntegerValueField(event, 1, 1)
        self._post(event)

    def move(self, dx: float, dy: float) -> None:
        current = self._pointer_location()
        destination = CGPoint(current.x + dx, current.y + dy)
        event = self._api.CGEventCreateMouseEvent(None, 5, destination, 0)
        if not event:
            raise InputError("CoreGraphics could not create a pointer-motion event")
        # A locked Cocoa cursor consumes the event's relative fields. Merely
        # changing the absolute location moves a desktop pointer but produces
        # no winit DeviceEvent::MouseMotion while the game owns the cursor.
        self._api.CGEventSetIntegerValueField(event, 4, round(dx))
        self._api.CGEventSetIntegerValueField(event, 5, round(dy))
        self._post(event)

    def focus_pid(self, pid: int) -> None:
        if pid <= 0:
            raise InputError("macOS focus pid must be positive")
        # An unbundled command-line Cocoa executable is not necessarily exposed
        # as an NSRunningApplication until it has first been activated. The
        # public Process Manager bridge can still resolve its exact Unix PID;
        # the two option bits mean front window only and caused by user input.
        serial = ProcessSerialNumber()
        status = self._api.GetProcessForPID(pid, ctypes.byref(serial))
        if status != 0:
            raise InputError(f"process {pid} has no macOS application identity ({status})")
        status = self._api.SetFrontProcessWithOptions(ctypes.byref(serial), 3)
        if status != 0:
            raise InputError(f"macOS could not focus process {pid} ({status})")

        # Process activation alone does not make an unbundled Cocoa window the
        # key window. Accessibility is already a declared mac-release
        # capability, so use that exact PID's first window to establish the
        # same focused state a physical click would create.
        attributes = {
            name: self._cf.CFStringCreateWithCString(
                None, name.encode("ascii"), 0x08000100
            )
            for name in ("AXFrontmost", "AXWindows", "AXMain", "AXFocused", "AXRaise")
        }
        if not all(attributes.values()):
            for value in attributes.values():
                if value:
                    self._cf.CFRelease(value)
            raise InputError("macOS could not create Accessibility attribute names")
        application = self._api.AXUIElementCreateApplication(pid)
        windows = c_void_p()
        true_value = c_void_p.in_dll(self._cf, "kCFBooleanTrue").value
        try:
            if not application or not true_value:
                raise InputError("macOS could not open the focus Accessibility bridge")
            status = self._api.AXUIElementSetAttributeValue(
                application, attributes["AXFrontmost"], true_value
            )
            if status != 0:
                raise InputError(f"macOS could not make process {pid} frontmost ({status})")
            status = self._api.AXUIElementCopyAttributeValue(
                application, attributes["AXWindows"], ctypes.byref(windows)
            )
            if status != 0 or not windows.value:
                raise InputError(f"macOS could not find process {pid}'s window ({status})")
            if self._cf.CFArrayGetCount(windows) < 1:
                raise InputError(f"macOS process {pid} has no Accessibility window")
            window = self._cf.CFArrayGetValueAtIndex(windows, 0)
            _require_accessibility_status(
                "main",
                self._api.AXUIElementSetAttributeValue(
                    window, attributes["AXMain"], true_value
                ),
                optional_attribute=True,
            )
            _require_accessibility_status(
                "focused",
                self._api.AXUIElementSetAttributeValue(
                    window, attributes["AXFocused"], true_value
                ),
                optional_attribute=True,
            )
            _require_accessibility_status(
                "raised",
                self._api.AXUIElementPerformAction(window, attributes["AXRaise"]),
            )
            time.sleep(0.2)

            frontmost = c_void_p()
            try:
                status = self._api.AXUIElementCopyAttributeValue(
                    application, attributes["AXFrontmost"], ctypes.byref(frontmost)
                )
                if (
                    status != 0
                    or not frontmost.value
                    or not self._cf.CFBooleanGetValue(frontmost)
                ):
                    raise InputError(
                        f"macOS process {pid} did not remain frontmost ({status})"
                    )
            finally:
                if frontmost.value:
                    self._cf.CFRelease(frontmost)
        finally:
            if windows.value:
                self._cf.CFRelease(windows)
            if application:
                self._cf.CFRelease(application)
            for value in attributes.values():
                self._cf.CFRelease(value)

    @staticmethod
    def clipboard(text: str) -> None:
        completed = subprocess.run(["/usr/bin/pbcopy"], input=text, text=True, capture_output=True)
        if completed.returncode != 0:
            raise InputError(completed.stderr.strip() or "pbcopy failed")
