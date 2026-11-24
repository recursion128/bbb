"""Cross-platform process memory/CPU sampling and host CPU identification."""

from __future__ import annotations

import ctypes
import os
import platform
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class ProcessMetrics:
    rss_kib: int = 0
    peak_rss_kib: int = 0
    cpu_seconds: float = 0.0
    threads: int | None = None
    open_files: int | None = None


def _parse_ps_cpu_time(token: str) -> float:
    days = 0
    if "-" in token:
        day_part, token = token.split("-", 1)
        days = int(day_part)
    parts = token.split(":")
    if len(parts) == 3:
        hours, minutes, seconds = int(parts[0]), int(parts[1]), float(parts[2])
    elif len(parts) == 2:
        hours, minutes, seconds = 0, int(parts[0]), float(parts[1])
    else:
        return 0.0
    return days * 86400.0 + hours * 3600.0 + minutes * 60.0 + seconds


def _sample_linux_proc(pid: int) -> ProcessMetrics | None:
    proc_root = Path("/proc") / str(pid)
    status_path = proc_root / "status"
    if not status_path.is_file():
        return None
    rss_kib = 0
    peak_rss_kib = 0
    threads: int | None = None
    try:
        for line in status_path.read_text(encoding="utf-8", errors="replace").splitlines():
            name, _, value = line.partition(":")
            if name == "VmRSS":
                rss_kib = int(value.split()[0])
            elif name == "VmHWM":
                peak_rss_kib = int(value.split()[0])
            elif name == "Threads":
                threads = int(value.split()[0])
    except (OSError, ValueError, IndexError):
        return ProcessMetrics()

    cpu_secs = 0.0
    stat_path = proc_root / "stat"
    if stat_path.is_file() and hasattr(os, "sysconf"):
        try:
            fields = stat_path.read_text(encoding="utf-8", errors="replace").rsplit(")", 1)[1].split()
            cpu_secs = (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK")
        except (OSError, ValueError, IndexError):
            cpu_secs = 0.0

    open_files: int | None = None
    fd_dir = proc_root / "fd"
    if fd_dir.is_dir():
        try:
            open_files = sum(1 for _ in fd_dir.iterdir())
        except OSError:
            open_files = None

    return ProcessMetrics(
        rss_kib=rss_kib,
        peak_rss_kib=peak_rss_kib or rss_kib,
        cpu_seconds=cpu_secs,
        threads=threads,
        open_files=open_files,
    )


def _sample_windows(pid: int) -> ProcessMetrics:
    class _FileTime(ctypes.Structure):
        _fields_ = [("low", ctypes.c_uint32), ("high", ctypes.c_uint32)]

    class _ProcessMemoryCounters(ctypes.Structure):
        _fields_ = [
            ("cb", ctypes.c_uint32),
            ("PageFaultCount", ctypes.c_uint32),
            ("PeakWorkingSetSize", ctypes.c_size_t),
            ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t),
            ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.OpenProcess.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
    kernel32.OpenProcess.restype = ctypes.c_void_p
    kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
    kernel32.CloseHandle.restype = ctypes.c_int

    process_query_limited_information = 0x1000
    handle = kernel32.OpenProcess(process_query_limited_information, 0, pid)
    if not handle:
        return ProcessMetrics()
    try:
        rss_kib = 0
        peak_rss_kib = 0
        counters = _ProcessMemoryCounters()
        counters.cb = ctypes.sizeof(_ProcessMemoryCounters)
        get_mem = getattr(kernel32, "K32GetProcessMemoryInfo", None)
        if get_mem is None:
            psapi = ctypes.WinDLL("psapi", use_last_error=True)
            get_mem = psapi.GetProcessMemoryInfo
        get_mem.argtypes = [
            ctypes.c_void_p,
            ctypes.POINTER(_ProcessMemoryCounters),
            ctypes.c_uint32,
        ]
        get_mem.restype = ctypes.c_int
        if get_mem(handle, ctypes.byref(counters), counters.cb):
            rss_kib = int(counters.WorkingSetSize // 1024)
            peak_rss_kib = int(counters.PeakWorkingSetSize // 1024)

        cpu_secs = 0.0
        creation, exit_t, kernel_t, user_t = (
            _FileTime(),
            _FileTime(),
            _FileTime(),
            _FileTime(),
        )
        kernel32.GetProcessTimes.argtypes = [
            ctypes.c_void_p,
            ctypes.POINTER(_FileTime),
            ctypes.POINTER(_FileTime),
            ctypes.POINTER(_FileTime),
            ctypes.POINTER(_FileTime),
        ]
        kernel32.GetProcessTimes.restype = ctypes.c_int
        if kernel32.GetProcessTimes(
            handle,
            ctypes.byref(creation),
            ctypes.byref(exit_t),
            ctypes.byref(kernel_t),
            ctypes.byref(user_t),
        ):
            kernel_ticks = (kernel_t.high << 32) | kernel_t.low
            user_ticks = (user_t.high << 32) | user_t.low
            cpu_secs = (kernel_ticks + user_ticks) * 1e-7

        return ProcessMetrics(
            rss_kib=rss_kib,
            peak_rss_kib=peak_rss_kib or rss_kib,
            cpu_seconds=cpu_secs,
        )
    finally:
        kernel32.CloseHandle(handle)


def _sample_posix_ps(pid: int) -> ProcessMetrics:
    try:
        result = subprocess.run(
            ["ps", "-o", "rss=,time=", "-p", str(pid)],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return ProcessMetrics()
    if result.returncode != 0:
        return ProcessMetrics()
    fields = result.stdout.strip().split()
    if len(fields) < 2:
        return ProcessMetrics()
    try:
        rss_kib = int(fields[0])
        cpu_secs = _parse_ps_cpu_time(fields[1])
    except ValueError:
        return ProcessMetrics()
    return ProcessMetrics(rss_kib=rss_kib, peak_rss_kib=rss_kib, cpu_seconds=cpu_secs)


def sample_process_metrics(pid: int) -> ProcessMetrics:
    if pid <= 0:
        return ProcessMetrics()
    linux = _sample_linux_proc(pid)
    if linux is not None:
        return linux
    if os.name == "nt":
        return _sample_windows(pid)
    return _sample_posix_ps(pid)


def host_cpu_model() -> str:
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.is_file():
        try:
            for line in cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines():
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()
        except OSError:
            pass
    if sys.platform == "darwin":
        try:
            brand = subprocess.run(
                ["sysctl", "-n", "machdep.cpu.brand_string"],
                capture_output=True,
                text=True,
                check=False,
            )
            if brand.returncode == 0 and brand.stdout.strip():
                return brand.stdout.strip()
        except OSError:
            pass
    return platform.processor() or platform.machine()
