"""Cross-platform host capability inventory."""

from __future__ import annotations

import ctypes
import os
import platform
import shutil
import sys
from ctypes.util import find_library
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True, slots=True)
class HostCapabilities:
    os: str
    architecture: str
    python: str
    python_312: bool
    cargo: bool
    rustc: bool
    git: bool
    java: bool
    graphical_session: bool
    wayland: bool
    vulkan_loader: bool
    linux_uinput: bool
    linux_seat_tools: bool
    libei: bool
    macos_quartz: bool
    macos_accessibility_trusted: bool

    def as_json(self) -> dict[str, Any]:
        return asdict(self)


PROFILE_REQUIREMENTS = {
    "portable": ("python_312", "cargo", "rustc", "git"),
    "mac-dev": ("python_312", "cargo", "rustc", "git", "macos_quartz"),
    "mac-input": (
        "python_312",
        "macos_quartz",
        "macos_accessibility_trusted",
    ),
    "linux-release": (
        "cargo",
        "rustc",
        "git",
        "java",
        "graphical_session",
        "wayland",
        "vulkan_loader",
        "linux_uinput",
        "linux_seat_tools",
    ),
}


def _macos_accessibility_trusted() -> bool:
    if sys.platform != "darwin":
        return False
    library = find_library("ApplicationServices")
    if library is None:
        return False
    try:
        api = ctypes.CDLL(library)
        api.AXIsProcessTrusted.restype = ctypes.c_bool
        api.AXIsProcessTrusted.argtypes = []
        return bool(api.AXIsProcessTrusted())
    except (AttributeError, OSError):
        return False


def _vulkan_loader_available() -> bool:
    return (
        find_library("vulkan") is not None
        or shutil.which("vulkaninfo") is not None
        or Path("/usr/lib/libvulkan.so.1").exists()
        or Path("/usr/lib64/libvulkan.so.1").exists()
    )


def probe_host() -> HostCapabilities:
    linux = sys.platform.startswith("linux")
    macos = sys.platform == "darwin"
    return HostCapabilities(
        os=sys.platform,
        architecture=platform.machine(),
        python=platform.python_version(),
        python_312=sys.version_info[:2] == (3, 12),
        cargo=shutil.which("cargo") is not None,
        rustc=shutil.which("rustc") is not None,
        git=shutil.which("git") is not None,
        java=(
            shutil.which("java") is not None
            or (
                bool(java_override := os.environ.get("BBB_PLAYABLE_JAVA"))
                and os.access(java_override, os.X_OK)
            )
        ),
        graphical_session=bool(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY")),
        wayland=linux and bool(os.environ.get("WAYLAND_DISPLAY")),
        vulkan_loader=_vulkan_loader_available(),
        linux_uinput=linux and os.access("/dev/uinput", os.W_OK),
        linux_seat_tools=linux
        and all(
            shutil.which(tool) is not None
            for tool in ("swaymsg", "wtype", "wlrctl", "wl-copy", "wl-paste")
        ),
        libei=linux
        and (
            shutil.which("ei-debug-events") is not None
            or shutil.which("libei-test-suite") is not None
            or find_library("ei") is not None
        ),
        macos_quartz=macos
        and Path("/System/Library/Frameworks/ApplicationServices.framework").exists(),
        macos_accessibility_trusted=_macos_accessibility_trusted(),
    )


def missing_for_profile(capabilities: HostCapabilities, profile: str) -> list[str]:
    try:
        required = PROFILE_REQUIREMENTS[profile]
    except KeyError as error:
        raise ValueError(f"unknown host profile {profile!r}") from error
    return [name for name in required if not getattr(capabilities, name)]
